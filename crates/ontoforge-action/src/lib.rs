//! Action execution through the storage-owned edit/log/command/outbox transaction.

use std::collections::BTreeMap;

use chrono::{Duration, SecondsFormat, Utc};
use ontoforge_auth::{Principal, scope::PrincipalScope};
use ontoforge_schema::ir::{ActionType, ActionTypeName, Backing, ParamType, Rule, ValueSource};
use ontoforge_storage::commit::{
    ActionCommitError, ActionCommitOutcome, ActionEditPosition, ActionTransactionInput, Actor,
    Commit, Edit, ObjectOp,
};
use ontoforge_storage::error::StoreError;
use ontoforge_storage::store::{ActiveDefinition, EditStore};
use ontoforge_storage::value::PropertyValue;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

pub use ontoforge_storage::commit::{ActionReceipt, EditSummary, ItemReceipt};

/// Cargo package name of this crate.
pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");

/// One client command. Host limits and principal identity are not wire fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ActionRequest {
    pub action: String,
    pub command_id: Option<String>,
    pub items: Vec<ActionItem>,
    #[serde(default)]
    pub validate_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ActionItem {
    pub params: Map<String, Value>,
}

/// Cost caps come from the trusted host, never from an Action request.
#[derive(Debug, Clone, Copy)]
pub struct ActionLimits {
    items: usize,
    edits: usize,
    request_bytes: usize,
}

impl ActionLimits {
    pub fn new(
        max_items: usize,
        max_edits: usize,
        max_request_bytes: usize,
    ) -> Result<Self, ActionError> {
        if max_items == 0 || max_edits == 0 || max_request_bytes == 0 {
            return Err(ActionError::InvalidLimits);
        }
        Ok(Self {
            items: max_items,
            edits: max_edits,
            request_bytes: max_request_bytes,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ActionError {
    #[error("a normal Action submission needs a non-empty command_id")]
    MissingCommandId,
    #[error("the Action is not available to this principal")]
    ScopeDenied,
    #[error("unknown Action `{0}`")]
    UnknownAction(String),
    #[error("invalid Action request: {0}")]
    InvalidRequest(String),
    #[error("this Action rule is not supported by the edit-only tracer")]
    UnsupportedRule,
    #[error("Action limits must be positive")]
    InvalidLimits,
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Commit(#[from] ActionCommitError),
}

pub struct ActionService<'a> {
    store: &'a EditStore,
    limits: ActionLimits,
}

impl<'a> ActionService<'a> {
    pub fn new(store: &'a EditStore, limits: ActionLimits) -> Self {
        Self { store, limits }
    }

    /// Executes the edit-only `ModifyObject` slice using an authenticated principal and
    /// the host's resolved scope. Other rule kinds are refused until their full path exists.
    pub async fn execute(
        &self,
        principal: &Principal,
        scope: &PrincipalScope,
        def_version: i64,
        request: ActionRequest,
    ) -> Result<ActionReceipt, ActionError> {
        validate_request(&request, self.limits)?;
        let active = self
            .store
            .active_definition()
            .await
            .ok_or_else(|| ActionError::InvalidRequest("no active definition".to_owned()))?;
        let action_name: ActionTypeName = request
            .action
            .clone()
            .try_into()
            .map_err(|_| ActionError::UnknownAction(request.action.clone()))?;
        if !scope.actions.contains(&action_name) {
            return Err(ActionError::ScopeDenied);
        }
        let action = active
            .ontology
            .action_types
            .get(&action_name)
            .ok_or_else(|| ActionError::UnknownAction(request.action.clone()))?;
        let chosen_time = Utc::now();
        let chosen_current_time = chosen_time.to_rfc3339_opts(SecondsFormat::Millis, true);
        let expires_at =
            (chosen_time + Duration::days(7)).to_rfc3339_opts(SecondsFormat::Millis, true);
        let prepared = prepare_edits(
            self.store,
            &active,
            action,
            principal,
            &request.items,
            &chosen_current_time,
            self.limits.edits,
        )
        .await?;

        let fingerprint = json!({
            "action": request.action,
            "items": request.items,
            "validate_only": request.validate_only,
        });
        let input = ActionTransactionInput {
            commit: Commit {
                actor: Actor::Principal(principal.id.as_str().to_owned()),
                action: request.action,
                def_version,
                params: json!({ "items": request.items }),
                edits: prepared.edits,
            },
            command_id: request.command_id,
            request_hash: digest(&fingerprint),
            action_def_hash: digest(
                &serde_json::to_value(action)
                    .map_err(|e| ActionError::InvalidRequest(e.to_string()))?,
            ),
            chosen_current_time,
            expires_at,
            item_count: prepared
                .positions
                .iter()
                .map(|p| p.item_ordinal + 1)
                .max()
                .unwrap_or(0),
            positions: prepared.positions,
            observed_versions: prepared.observed_versions,
            validate_only: false,
        };
        match self.store.commit_action(input).await? {
            ActionCommitOutcome::Committed { receipt, .. }
            | ActionCommitOutcome::Replay(receipt) => Ok(receipt),
            ActionCommitOutcome::Validated => Err(ActionError::InvalidRequest(
                "unexpected validation result".to_owned(),
            )),
        }
    }
}

fn validate_request(request: &ActionRequest, limits: ActionLimits) -> Result<(), ActionError> {
    if !request.validate_only && request.command_id.as_deref().is_none_or(str::is_empty) {
        return Err(ActionError::MissingCommandId);
    }
    if request.validate_only {
        return Err(ActionError::InvalidRequest(
            "validate-only is not available in the edit-only tracer".to_owned(),
        ));
    }
    if request.items.is_empty() || request.items.len() > limits.items {
        return Err(ActionError::InvalidRequest(
            "item count exceeds host limits".to_owned(),
        ));
    }
    let request_value =
        serde_json::to_value(request).map_err(|e| ActionError::InvalidRequest(e.to_string()))?;
    if request_value.to_string().len() > limits.request_bytes {
        return Err(ActionError::InvalidRequest(
            "request exceeds host byte limit".to_owned(),
        ));
    }
    Ok(())
}

struct PreparedEdits {
    edits: Vec<Edit>,
    positions: Vec<ActionEditPosition>,
    observed_versions: Vec<i64>,
}

async fn prepare_edits(
    store: &EditStore,
    active: &ActiveDefinition,
    action: &ActionType,
    principal: &Principal,
    items: &[ActionItem],
    chosen_current_time: &str,
    max_edits: usize,
) -> Result<PreparedEdits, ActionError> {
    let mut prepared = PreparedEdits {
        edits: Vec::new(),
        positions: Vec::new(),
        observed_versions: Vec::new(),
    };
    for (item_ordinal, item) in items.iter().enumerate() {
        validate_item_params(action, item)?;
        for (rule_ordinal, rule) in action.rules.iter().enumerate() {
            let Rule::ModifyObject(rule) = rule else {
                return Err(ActionError::UnsupportedRule);
            };
            let reference = action.parameters.get(&rule.object).ok_or_else(|| {
                ActionError::InvalidRequest("unknown object parameter".to_owned())
            })?;
            let ParamType::ObjectReference(type_name) = &reference.param_type else {
                return Err(ActionError::InvalidRequest(
                    "object parameter is not a reference".to_owned(),
                ));
            };
            let ty = active
                .ontology
                .object_types
                .get(type_name)
                .ok_or_else(|| ActionError::InvalidRequest("unknown object type".to_owned()))?;
            if ty.backing != Backing::EditOnly {
                return Err(ActionError::UnsupportedRule);
            }
            let pk_def = ty.properties.get(&ty.primary_key).ok_or_else(|| {
                ActionError::InvalidRequest("missing primary key definition".to_owned())
            })?;
            let pk_json = parameter_value(item, rule.object.as_str(), reference.default.as_ref())?;
            let pk = PropertyValue::from_json(pk_def, pk_json).map_err(|e| {
                ActionError::InvalidRequest(format!("invalid object reference: {e}"))
            })?;
            let observed = store
                .read_object_edit(active, type_name, &pk)
                .await?
                .ok_or(ActionError::ScopeDenied)?;
            if observed.row.deleted || observed.row.released {
                return Err(ActionError::ScopeDenied);
            }
            let mut set = BTreeMap::new();
            for (property, source) in &rule.properties {
                let property_def = ty.properties.get(property).ok_or_else(|| {
                    ActionError::InvalidRequest("unknown rule property".to_owned())
                })?;
                let raw = match source {
                    ValueSource::Param(name) => {
                        let param = action.parameters.get(name).ok_or_else(|| {
                            ActionError::InvalidRequest("unknown value parameter".to_owned())
                        })?;
                        parameter_value(item, name.as_str(), param.default.as_ref())?.clone()
                    }
                    ValueSource::Static(value) => value.clone(),
                    ValueSource::CurrentUser(_) => Value::String(principal.id.as_str().to_owned()),
                    ValueSource::CurrentTime(_) => Value::String(chosen_current_time.to_owned()),
                };
                let value = PropertyValue::from_json(property_def, &raw).map_err(|e| {
                    ActionError::InvalidRequest(format!("invalid `{property}` value: {e}"))
                })?;
                set.insert(property.clone(), value);
            }
            prepared.edits.push(Edit::Object {
                type_name: type_name.clone(),
                pk,
                op: ObjectOp::Set(set),
            });
            prepared.positions.push(ActionEditPosition {
                item_ordinal,
                rule_ordinal,
            });
            prepared.observed_versions.push(observed.row.version);
            if prepared.edits.len() > max_edits {
                return Err(ActionError::InvalidRequest(
                    "edit count exceeds host limits".to_owned(),
                ));
            }
        }
    }
    Ok(prepared)
}

fn validate_item_params(action: &ActionType, item: &ActionItem) -> Result<(), ActionError> {
    for name in item.params.keys() {
        let known = action.parameters.keys().any(|key| key.as_str() == name);
        if !known {
            return Err(ActionError::InvalidRequest(format!(
                "unknown parameter `{name}`"
            )));
        }
    }
    for (name, def) in &action.parameters {
        if def.required && !item.params.contains_key(name.as_str()) && def.default.is_none() {
            return Err(ActionError::InvalidRequest(format!(
                "missing parameter `{name}`"
            )));
        }
    }
    Ok(())
}

fn parameter_value<'a>(
    item: &'a ActionItem,
    name: &str,
    default: Option<&'a Value>,
) -> Result<&'a Value, ActionError> {
    item.params
        .get(name)
        .or(default)
        .ok_or_else(|| ActionError::InvalidRequest(format!("missing parameter `{name}`")))
}

fn canonical_json(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort_unstable();
            let mut ordered = Map::new();
            for key in keys {
                ordered.insert(key.clone(), canonical_json(&map[key]));
            }
            Value::Object(ordered)
        }
        Value::Array(items) => Value::Array(items.iter().map(canonical_json).collect()),
        other => other.clone(),
    }
}

fn digest(value: &Value) -> String {
    let bytes = serde_json::to_vec(&canonical_json(value)).expect("JSON values serialize");
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::{
        ActionError, ActionItem, ActionLimits, ActionRequest, ActionService, CRATE_NAME, digest,
    };
    use camino::Utf8PathBuf;
    use ontoforge_auth::{Principal, PrincipalId, scope::PrincipalScope};
    use ontoforge_expr::rowpred::Attrs;
    use ontoforge_schema::load_from_memory;
    use ontoforge_storage::commit::{
        ActionCommitError, ActionCommitOutcome, ActionEditPosition, ActionStage,
        ActionTransactionInput, Actor, Commit, Edit, EditError, ObjectOp,
    };
    use ontoforge_storage::reload::{ReloadOptions, reload};
    use ontoforge_storage::store::{
        BackendSettings, EditStore, StoreSettings, sqlite_connect_options,
    };
    use ontoforge_storage::value::PropertyValue;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::collections::{BTreeMap, BTreeSet};
    use std::time::Duration;

    const DEFS: &str = "kind: ObjectType
apiName: Gate
backing: editOnly
primaryKey: code
properties:
  code: { type: string, required: true }
  name: { type: string, required: true }
---
kind: ActionType
apiName: renameGate
parameters:
  gate: { type: { objectReference: Gate }, required: true }
  name: { type: string, required: true }
rules:
  - kind: modifyObject
    object: gate
    properties:
      name: { param: name }
";

    #[test]
    fn crate_name_has_ontoforge_prefix() {
        assert!(CRATE_NAME.starts_with("ontoforge-"));
    }

    #[tokio::test]
    #[expect(
        clippy::too_many_lines,
        reason = "one database tracer checks commit, rollback and ordering"
    )]
    async fn tracer_commits_one_action_batch() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = Utf8PathBuf::from_path_buf(dir.path().join("edit.db")).expect("utf8 path");
        let store = EditStore::open(StoreSettings {
            backend: BackendSettings::Sqlite { path: path.clone() },
            lock_timeout: Duration::from_secs(5),
            probe_timeout: Duration::from_secs(5),
            trino_catalog: None,
        })
        .await
        .expect("open store");
        let loaded = load_from_memory(&[("ontology.yaml", "apiName: demo\n"), ("defs.yaml", DEFS)]);
        assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
        let ontology = loaded.ontology.expect("ontology");
        let version = reload(&store, &ontology, &ReloadOptions::default())
            .await
            .expect("reload")
            .def_version
            .expect("definition version");
        let property = |name: &str| name.to_owned().try_into().expect("property name");
        let edit = |name: &str| Edit::Object {
            type_name: "Gate".to_owned().try_into().expect("type name"),
            pk: PropertyValue::String("A1".to_owned()),
            op: ObjectOp::Set(BTreeMap::from([(
                property("name"),
                PropertyValue::String(name.to_owned()),
            )])),
        };
        store
            .commit(Commit {
                actor: Actor::Principal("https://idp.example.com|alice".to_owned()),
                action: "seed".to_owned(),
                def_version: version,
                params: serde_json::Value::Null,
                edits: vec![edit("Old")],
            })
            .await
            .expect("seed");

        let principal = Principal {
            id: PrincipalId::new("https://idp.example.com", "alice"),
            issuer: "https://idp.example.com".to_owned(),
            subject: "alice".to_owned(),
            groups: BTreeSet::default(),
            attrs: Attrs::new(),
        };
        let mut scope = PrincipalScope::default();
        scope
            .actions
            .insert("renameGate".to_owned().try_into().expect("action name"));
        let service = ActionService::new(&store, ActionLimits::new(8, 8, 65536).expect("limits"));
        let request = ActionRequest {
            action: "renameGate".to_owned(),
            command_id: Some("command-1".to_owned()),
            items: vec![ActionItem {
                params: serde_json::from_value(serde_json::json!({"gate": "A1", "name": "New"}))
                    .expect("params"),
            }],
            validate_only: false,
        };
        let receipt = service
            .execute(&principal, &scope, version, request.clone())
            .await
            .expect("Action execute");
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(sqlite_connect_options(&path))
            .await
            .expect("inspect store");
        let command_tables: i64 = sqlx::query_scalar("SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'of_action_command'")
            .fetch_one(&pool).await.expect("command table query");
        assert_eq!(
            command_tables, 1,
            "Action batch must persist a command claim"
        );
        let command_count: i64 = sqlx::query_scalar("SELECT count(*) FROM of_action_command")
            .fetch_one(&pool)
            .await
            .expect("command count");
        let outbox_count: i64 = sqlx::query_scalar("SELECT count(*) FROM of_webhook_outbox")
            .fetch_one(&pool)
            .await
            .expect("outbox count");
        assert_eq!(command_count, 1);
        assert_eq!(outbox_count, 1);
        assert_eq!(receipt.items.len(), 1);
        assert_eq!(receipt.items[0].edits[0].version_after, 2);
        let log = store.read_log(0, 10).await.expect("read log");
        assert_eq!(log.len(), 2, "seed plus exactly one Action log");
        assert_eq!(log[1].event_id, receipt.event_id);
        assert_eq!(log[1].seq, receipt.log_seq);
        let command: (String, String) = sqlx::query_as(
            "SELECT event_id, receipt FROM of_action_command WHERE principal_id = 'https://idp.example.com|alice' AND command_id = 'command-1'",
        )
        .fetch_one(&pool)
        .await
        .expect("stored command");
        assert_eq!(command.0, receipt.event_id);
        let stored: serde_json::Value = serde_json::from_str(&command.1).expect("receipt JSON");
        assert_eq!(
            stored,
            serde_json::to_value(&receipt).expect("outward receipt JSON")
        );
        let outbox: (String, String) =
            sqlx::query_as("SELECT event_id, payload FROM of_webhook_outbox")
                .fetch_one(&pool)
                .await
                .expect("outbox event");
        assert_eq!(outbox.0, receipt.event_id);
        let payload: serde_json::Value = serde_json::from_str(&outbox.1).expect("outbox JSON");
        assert_eq!(payload["event_id"], receipt.event_id);
        assert_eq!(payload["receipt"], stored);
        let receipt_text = command.1;
        assert!(!receipt_text.contains("Old") && !receipt_text.contains("New"));

        let mut missing_id = request.clone();
        missing_id.command_id = None;
        assert!(matches!(
            service
                .execute(&principal, &scope, version, missing_id)
                .await,
            Err(ActionError::MissingCommandId)
        ));
        let mut stale = request;
        stale.command_id = Some("stale-command".to_owned());
        assert!(matches!(
            service
                .execute(&principal, &scope, version + 1, stale)
                .await,
            Err(ActionError::Commit(ActionCommitError::Edit(
                EditError::DefinitionStale { .. }
            )))
        ));
        let commands: i64 = sqlx::query_scalar("SELECT count(*) FROM of_action_command")
            .fetch_one(&pool)
            .await
            .expect("command count after refusals");
        let outbox: i64 = sqlx::query_scalar("SELECT count(*) FROM of_webhook_outbox")
            .fetch_one(&pool)
            .await
            .expect("outbox count after refusals");
        assert_eq!((commands, outbox), (1, 1));
        assert_eq!(
            store
                .read_log(0, 10)
                .await
                .expect("log after refusals")
                .len(),
            2
        );

        let traced = store
            .commit_action(ActionTransactionInput {
                commit: Commit {
                    actor: Actor::Principal(principal.id.as_str().to_owned()),
                    action: "renameGate".to_owned(),
                    def_version: version,
                    params: serde_json::Value::Null,
                    edits: vec![edit("Trace")],
                },
                command_id: Some("trace-command".to_owned()),
                request_hash: "trace-request-hash".to_owned(),
                action_def_hash: "trace-action-hash".to_owned(),
                chosen_current_time: "2026-10-23T00:00:00.000Z".to_owned(),
                expires_at: "2026-10-30T00:00:00.000Z".to_owned(),
                item_count: 1,
                positions: vec![ActionEditPosition {
                    item_ordinal: 0,
                    rule_ordinal: 0,
                }],
                observed_versions: vec![2],
                validate_only: false,
            })
            .await
            .expect("traced Action commit");
        let ActionCommitOutcome::Committed { trace, .. } = traced else {
            panic!("the trace command must commit");
        };
        assert_eq!(
            trace,
            [
                ActionStage::Begin,
                ActionStage::Fence,
                ActionStage::Claim,
                ActionStage::Locks,
                ActionStage::Cas,
                ActionStage::Log,
                ActionStage::EditRows,
                ActionStage::Entries,
                ActionStage::Command,
                ActionStage::Outbox,
                ActionStage::Commit,
            ]
        );
    }

    #[test]
    fn canonical_request_hash_ignores_object_key_insertion_order() {
        let first: serde_json::Value =
            serde_json::from_str(r#"{"nested":{"b":2,"a":1},"action":"renameGate"}"#)
                .expect("first");
        let reordered: serde_json::Value =
            serde_json::from_str(r#"{"action":"renameGate","nested":{"a":1,"b":2}}"#)
                .expect("reordered");
        let changed: serde_json::Value =
            serde_json::from_str(r#"{"action":"renameGate","nested":{"a":1,"b":3}}"#)
                .expect("changed");
        assert_eq!(digest(&first), digest(&reordered));
        assert_ne!(digest(&first), digest(&changed));
    }
}
