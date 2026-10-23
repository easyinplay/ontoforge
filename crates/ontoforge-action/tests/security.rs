use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use camino::Utf8PathBuf;
use ontoforge_action::{ActionError, ActionItem, ActionLimits, ActionRequest, ActionService};
use ontoforge_auth::{
    Principal, PrincipalId,
    scope::{PrincipalScope, TypeScope},
};
use ontoforge_expr::{CmpOp, Operand, Pred, Scalar, rowpred::Attrs};
use ontoforge_schema::load_from_memory;
use ontoforge_storage::commit::{Actor, Commit, Edit, ObjectOp};
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::store::{BackendSettings, EditStore, StoreSettings};
use ontoforge_storage::value::PropertyValue;

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
submissionCriteria:
  - condition: 'objects.gate.name == \"Allowed\"'
    failureMessage: criterion was false
";

async fn setup() -> (tempfile::TempDir, EditStore, i64, Principal) {
    setup_with_defs(DEFS).await
}

async fn setup_with_defs(defs: &str) -> (tempfile::TempDir, EditStore, i64, Principal) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = Utf8PathBuf::from_path_buf(dir.path().join("edit.db")).expect("utf8 path");
    let store = EditStore::open(StoreSettings {
        backend: BackendSettings::Sqlite { path },
        lock_timeout: Duration::from_secs(5),
        probe_timeout: Duration::from_secs(5),
        trino_catalog: None,
    })
    .await
    .expect("open store");
    let loaded = load_from_memory(&[("ontology.yaml", "apiName: demo\n"), ("defs.yaml", defs)]);
    assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
    let version = reload(
        &store,
        &loaded.ontology.expect("ontology"),
        &ReloadOptions::default(),
    )
    .await
    .expect("reload")
    .def_version
    .expect("version");
    store
        .commit(Commit {
            actor: Actor::Principal("https://idp.example.com|alice".to_owned()),
            action: "seed".to_owned(),
            def_version: version,
            params: serde_json::Value::Null,
            edits: vec![Edit::Object {
                type_name: "Gate".to_owned().try_into().expect("type"),
                pk: PropertyValue::String("A1".to_owned()),
                op: ObjectOp::Set(BTreeMap::from([(
                    "name".to_owned().try_into().expect("property"),
                    PropertyValue::String("Allowed".to_owned()),
                )])),
            }],
        })
        .await
        .expect("seed");
    let principal = Principal {
        id: PrincipalId::new("https://idp.example.com", "alice"),
        issuer: "https://idp.example.com".to_owned(),
        subject: "alice".to_owned(),
        groups: BTreeSet::new(),
        attrs: Attrs::new(),
    };
    (dir, store, version, principal)
}

fn scope(visible: bool, name_visible: bool) -> PrincipalScope {
    let mut scope = PrincipalScope::default();
    scope
        .actions
        .insert("renameGate".to_owned().try_into().expect("action"));
    let mut columns = BTreeMap::new();
    columns.insert(
        "code".to_owned().try_into().expect("column"),
        Pred::Const(true),
    );
    if name_visible {
        columns.insert(
            "name".to_owned().try_into().expect("column"),
            Pred::Const(true),
        );
    }
    scope.types.insert(
        "Gate".to_owned().try_into().expect("type"),
        TypeScope {
            row: Pred::Const(visible),
            columns,
        },
    );
    scope
}

fn request(pk: &str, id: &str) -> ActionRequest {
    ActionRequest {
        action: "renameGate".to_owned(),
        command_id: Some(id.to_owned()),
        items: vec![ActionItem {
            params: serde_json::from_value(serde_json::json!({
                "gate": pk, "name": "Updated"
            }))
            .expect("params"),
            expected_versions: BTreeMap::default(),
        }],
        validate_only: false,
    }
}

#[tokio::test]
async fn scope_hidden_target_matches_missing_outward_shape() {
    let (_dir, store, version, principal) = setup().await;
    let service = ActionService::new(&store, ActionLimits::new(8, 8, 65536).expect("limits"));
    let hidden = service
        .execute(
            &principal,
            &scope(false, true),
            version,
            request("A1", "hidden"),
        )
        .await
        .expect_err("hidden target must be rejected");
    let missing = service
        .execute(
            &principal,
            &scope(false, true),
            version,
            request("Z9", "missing"),
        )
        .await
        .expect_err("missing target must be rejected");
    assert_eq!(hidden.to_string(), missing.to_string());
    assert!(!hidden.to_string().contains("A1"));
    assert!(!hidden.to_string().contains("Gate"));
}

#[tokio::test]
async fn scope_masked_criteria_is_denied_before_evaluation() {
    let (_dir, store, version, principal) = setup().await;
    let service = ActionService::new(&store, ActionLimits::new(8, 8, 65536).expect("limits"));
    let outcome = service
        .execute(
            &principal,
            &scope(true, false),
            version,
            request("A1", "masked"),
        )
        .await;
    assert!(
        matches!(outcome, Err(ActionError::ScopeDenied)),
        "masked criterion must be denied, not evaluated: {outcome:?}"
    );
}

#[tokio::test]
async fn scope_criterion_false_uses_configured_message() {
    let (_dir, store, version, principal) = setup().await;
    store
        .commit(Commit {
            actor: Actor::Principal(principal.id.as_str().to_owned()),
            action: "seed".to_owned(),
            def_version: version,
            params: serde_json::Value::Null,
            edits: vec![Edit::Object {
                type_name: "Gate".to_owned().try_into().expect("type"),
                pk: PropertyValue::String("A1".to_owned()),
                op: ObjectOp::Set(BTreeMap::from([(
                    "name".to_owned().try_into().expect("name"),
                    PropertyValue::String("Other".to_owned()),
                )])),
            }],
        })
        .await
        .expect("change seed");
    let service = ActionService::new(&store, ActionLimits::new(8, 8, 65536).expect("limits"));
    let result = service
        .execute(
            &principal,
            &scope(true, true),
            version,
            request("A1", "criterion-false"),
        )
        .await;
    assert!(
        matches!(result, Err(ActionError::CriteriaFailed(ref message)) if message == "criterion was false")
    );
}

#[tokio::test]
async fn scope_post_state_must_remain_visible() {
    let (_dir, store, version, principal) = setup().await;
    let mut granted = scope(true, true);
    granted.types.get_mut("Gate").expect("type scope").row = Pred::Cmp {
        left: Operand::Column("name".to_owned().try_into().expect("name")),
        op: CmpOp::Eq,
        right: Operand::Lit(Scalar::String("Allowed".to_owned())),
    };
    let service = ActionService::new(&store, ActionLimits::new(8, 8, 65536).expect("limits"));
    let result = service
        .execute(
            &principal,
            &granted,
            version,
            request("A1", "post-invisible"),
        )
        .await;
    assert!(matches!(result, Err(ActionError::ScopeDenied)));
    assert_eq!(store.read_log(0, 10).await.expect("log").len(), 1);
}

#[tokio::test]
async fn scope_each_item_criterion_sees_prior_edit_on_same_row() {
    let (_dir, store, version, principal) = setup().await;
    let service = ActionService::new(&store, ActionLimits::new(8, 8, 65536).expect("limits"));
    let mut batch = request("A1", "two-items");
    batch.items.push(batch.items[0].clone());
    let result = service
        .execute(&principal, &scope(true, true), version, batch)
        .await;
    assert!(
        matches!(result, Err(ActionError::CriteriaFailed(ref message)) if message == "criterion was false"),
        "second item must evaluate its current logical before state: {result:?}"
    );
    assert_eq!(store.read_log(0, 10).await.expect("log").len(), 1);
}

#[tokio::test]
async fn scope_write_column_must_be_visible() {
    let defs = DEFS.replace(
        "objects.gate.name == \"Allowed\"",
        "params.name == \"Updated\"",
    );
    let (_dir, store, version, principal) = setup_with_defs(&defs).await;
    let service = ActionService::new(&store, ActionLimits::new(8, 8, 65536).expect("limits"));
    let result = service
        .execute(
            &principal,
            &scope(true, false),
            version,
            request("A1", "write-masked"),
        )
        .await;
    assert!(
        matches!(result, Err(ActionError::ScopeDenied)),
        "masked write must fail: {result:?}"
    );
    assert_eq!(store.read_log(0, 10).await.expect("log").len(), 1);
}

#[tokio::test]
async fn scope_missing_type_matches_hidden_row_and_missing_target() {
    let (_dir, store, version, principal) = setup().await;
    let service = ActionService::new(&store, ActionLimits::new(8, 8, 65536).expect("limits"));
    let hidden = service
        .execute(
            &principal,
            &scope(false, true),
            version,
            request("A1", "hidden-row"),
        )
        .await
        .expect_err("hidden row");
    let missing = service
        .execute(
            &principal,
            &scope(true, true),
            version,
            request("Z9", "missing-row"),
        )
        .await
        .expect_err("missing row");
    let mut ungranted = scope(true, true);
    ungranted.types.clear();
    let hidden_type = service
        .execute(
            &principal,
            &ungranted,
            version,
            request("A1", "hidden-type"),
        )
        .await
        .expect_err("hidden type");
    assert_eq!(hidden.to_string(), missing.to_string());
    assert_eq!(hidden_type.to_string(), missing.to_string());
}

#[tokio::test]
async fn scope_validate_only_rejects_masked_criteria_without_writes() {
    let (_dir, store, version, principal) = setup().await;
    let service = ActionService::new(&store, ActionLimits::new(8, 8, 65536).expect("limits"));
    let mut dry = request("A1", "ignored-command");
    dry.validate_only = true;
    dry.command_id = None;
    let result = service
        .execute(&principal, &scope(true, false), version, dry)
        .await;
    assert!(matches!(result, Err(ActionError::ScopeDenied)));
    assert_eq!(store.read_log(0, 10).await.expect("log").len(), 1);
}

#[tokio::test]
async fn scope_action_grant_is_checked_before_missing_target() {
    let (_dir, store, version, principal) = setup().await;
    let service = ActionService::new(&store, ActionLimits::new(8, 8, 65536).expect("limits"));
    let result = service
        .execute(
            &principal,
            &PrincipalScope::default(),
            version,
            request("Z9", "no-grant"),
        )
        .await;
    assert!(matches!(result, Err(ActionError::ScopeDenied)));
    assert_eq!(store.read_log(0, 10).await.expect("log").len(), 1);
}

#[tokio::test]
async fn scope_link_requires_both_endpoints() {
    let defs = format!(
        "{DEFS}---\nkind: ObjectType\napiName: Pilot\nbacking: editOnly\nprimaryKey: code\nproperties:\n  code: {{ type: string, required: true }}\n  name: {{ type: string, required: true }}\n---\nkind: LinkType\napiName: gateCrew\ncardinality: manyToMany\nfrom: {{ objectType: Gate, apiName: crew }}\nto: {{ objectType: Pilot, apiName: gates }}\nbacking: editOnly\n---\nkind: ActionType\napiName: connectCrew\nparameters:\n  gate: {{ type: {{ objectReference: Gate }}, required: true }}\n  pilot: {{ type: {{ objectReference: Pilot }}, required: true }}\nrules:\n  - kind: addLink\n    link: gateCrew\n    from: gate\n    to: pilot\n"
    );
    let (_dir, store, version, principal) = setup_with_defs(&defs).await;
    store
        .commit(Commit {
            actor: Actor::Principal(principal.id.as_str().to_owned()),
            action: "seed".to_owned(),
            def_version: version,
            params: serde_json::Value::Null,
            edits: vec![Edit::Object {
                type_name: "Pilot".to_owned().try_into().expect("type"),
                pk: PropertyValue::String("P1".to_owned()),
                op: ObjectOp::Set(BTreeMap::from([(
                    "name".to_owned().try_into().expect("name"),
                    PropertyValue::String("Pilot".to_owned()),
                )])),
            }],
        })
        .await
        .expect("seed pilot");
    let mut granted = scope(true, true);
    granted
        .actions
        .insert("connectCrew".to_owned().try_into().expect("action"));
    granted.types.insert(
        "Pilot".to_owned().try_into().expect("type"),
        TypeScope {
            row: Pred::Const(true),
            columns: ["code", "name"]
                .into_iter()
                .map(|name| {
                    (
                        name.to_owned().try_into().expect("column"),
                        Pred::Const(true),
                    )
                })
                .collect(),
        },
    );
    let request = |pilot: &str, validate_only: bool| ActionRequest {
        action: "connectCrew".to_owned(),
        command_id: (!validate_only).then(|| "link".to_owned()),
        items: vec![ActionItem {
            params: serde_json::from_value(serde_json::json!({
                "gate": "A1", "pilot": pilot,
            }))
            .expect("params"),
            expected_versions: BTreeMap::new(),
        }],
        validate_only,
    };
    let service = ActionService::new(&store, ActionLimits::new(8, 8, 65536).expect("limits"));
    let mut hidden_from = granted.clone();
    hidden_from.types.get_mut("Gate").expect("Gate").row = Pred::Const(false);
    let mut hidden_to = granted.clone();
    hidden_to.types.get_mut("Pilot").expect("Pilot").row = Pred::Const(false);
    for (permission, request) in [
        (&hidden_from, request("P1", false)),
        (&hidden_to, request("P1", true)),
        (&granted, request("P2", true)),
    ] {
        let result = service
            .execute(&principal, permission, version, request)
            .await;
        assert!(
            matches!(result, Err(ActionError::ScopeDenied)),
            "hidden/missing link endpoint: {result:?}"
        );
    }
    assert_eq!(store.read_log(0, 10).await.expect("log").len(), 2);
}
