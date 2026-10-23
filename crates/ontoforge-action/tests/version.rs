#![allow(
    clippy::unused_async_trait_impl,
    reason = "editOnly test engine has no source calls"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::Duration;

use camino::Utf8PathBuf;
use ontoforge_action::{ActionItem, ActionLimits, ActionRequest, ActionService};
use ontoforge_auth::{
    Principal, PrincipalId,
    scope::{PrincipalScope, TypeScope},
};
use ontoforge_engine::printer::{Dialect, PrintedSql};
use ontoforge_engine::{Engine, EngineError, EngineStatus, QueryContext, QueryOutput, TableRef};
use ontoforge_expr::{Attrs, Pred};
use ontoforge_query::{ExecutionLimits, ObjectSet, QueryRequest, QueryService, TokenCodec};
use ontoforge_schema::load_from_memory;
use ontoforge_storage::commit::{Actor, Commit, Edit, ObjectOp};
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::store::sqlite_connect_options;
use ontoforge_storage::store::{BackendSettings, EditStore, StoreSettings};
use ontoforge_storage::value::PropertyValue;
use sqlx::sqlite::SqlitePoolOptions;

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

async fn fixture() -> (tempfile::TempDir, EditStore, i64, Principal, PrincipalScope) {
    let dir = tempfile::tempdir().unwrap();
    let path = Utf8PathBuf::from_path_buf(dir.path().join("edit.db")).unwrap();
    let store = EditStore::open(StoreSettings {
        backend: BackendSettings::Sqlite { path },
        lock_timeout: Duration::from_secs(5),
        probe_timeout: Duration::from_secs(5),
        trino_catalog: None,
    })
    .await
    .unwrap();
    let loaded = load_from_memory(&[("ontology.yaml", "apiName: demo\n"), ("defs.yaml", DEFS)]);
    assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
    let def_version = reload(&store, &loaded.ontology.unwrap(), &ReloadOptions::default())
        .await
        .unwrap()
        .def_version
        .unwrap();
    store
        .commit(Commit {
            actor: Actor::Principal("https://idp.example.com|alice".to_owned()),
            action: "seed".to_owned(),
            def_version,
            params: serde_json::Value::Null,
            edits: vec![Edit::Object {
                type_name: "Gate".to_owned().try_into().unwrap(),
                pk: PropertyValue::String("A1".to_owned()),
                op: ObjectOp::Set(BTreeMap::from([(
                    "name".to_owned().try_into().unwrap(),
                    PropertyValue::String("Allowed".to_owned()),
                )])),
            }],
        })
        .await
        .unwrap();
    let principal = Principal {
        id: PrincipalId::new("https://idp.example.com", "alice"),
        issuer: "https://idp.example.com".to_owned(),
        subject: "alice".to_owned(),
        groups: BTreeSet::new(),
        attrs: Attrs::new(),
    };
    let mut scope = PrincipalScope::default();
    scope
        .actions
        .insert("renameGate".to_owned().try_into().unwrap());
    scope.types.insert(
        "Gate".to_owned().try_into().unwrap(),
        TypeScope {
            row: Pred::Const(true),
            columns: ["code", "name"]
                .into_iter()
                .map(|name| (name.to_owned().try_into().unwrap(), Pred::Const(true)))
                .collect(),
        },
    );
    (dir, store, def_version, principal, scope)
}

fn request(token: &str) -> ActionRequest {
    ActionRequest {
        action: "renameGate".to_owned(),
        command_id: Some("bad-token".to_owned()),
        items: vec![ActionItem {
            params: serde_json::from_value(serde_json::json!({ "gate": "A1", "name": "Updated" }))
                .unwrap(),
            expected_versions: BTreeMap::from([("gate".to_owned(), token.to_owned())]),
        }],
        validate_only: false,
    }
}

struct NeverEngine;

impl Engine for NeverEngine {
    fn dialect(&self) -> Dialect {
        Dialect::Sqlite
    }

    async fn execute(&self, _: &PrintedSql, _: &QueryContext) -> Result<QueryOutput, EngineError> {
        panic!("editOnly token must not access source")
    }

    async fn describe_tables(
        &self,
        _: &[TableRef],
        _: &QueryContext,
    ) -> Result<BTreeMap<TableRef, Vec<ontoforge_engine::PhysicalColumn>>, EngineError> {
        panic!("editOnly token must not describe source")
    }

    async fn status(&self) -> EngineStatus {
        EngineStatus::Available
    }
}

async fn query_token(
    store: &EditStore,
    principal: &Principal,
    scope: &PrincipalScope,
    tokens: &TokenCodec,
) -> String {
    let engine = NeverEngine;
    QueryService {
        principal,
        scope,
        active: Arc::clone(&store.active_definition().await.unwrap()),
        engine: &engine,
        store,
        limits: ExecutionLimits::phase4_default(),
        tokens,
    }
    .execute(QueryRequest {
        set: ObjectSet::Pk {
            object_type: "Gate".to_owned().try_into().unwrap(),
            pk: serde_json::json!("A1"),
        },
        select: vec!["name".to_owned().try_into().unwrap()],
    })
    .await
    .unwrap()
    .rows
    .remove(0)
    .version
}

async fn assert_unchanged(store: &EditStore) {
    let active = store.active_definition().await.unwrap();
    let row = store
        .read_object_edit(
            &active,
            &"Gate".to_owned().try_into().unwrap(),
            &PropertyValue::String("A1".to_owned()),
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.row.version, 1);
    assert_eq!(store.read_log(0, 10).await.unwrap().len(), 1);
}

async fn assert_no_action_rows(dir: &tempfile::TempDir) {
    let path = Utf8PathBuf::from_path_buf(dir.path().join("edit.db")).unwrap();
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(sqlite_connect_options(&path))
        .await
        .unwrap();
    for table in ["of_action_command", "of_webhook_outbox"] {
        let query = format!("SELECT count(*) FROM {table}");
        let count: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(query))
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0, "{table} changed after conflict");
    }
}

#[tokio::test]
async fn token_query_issued_is_accepted_by_unchanged_action_read_set() {
    let (_dir, store, version, principal, scope) = fixture().await;
    let tokens = TokenCodec::new(b"shared-key".to_vec()).unwrap();
    let token = query_token(&store, &principal, &scope, &tokens).await;
    assert!(token.starts_with("v1."));
    let service = ActionService::new(&store, ActionLimits::new(8, 8, 65536).unwrap(), &tokens);
    service
        .execute(&principal, &scope, version, request(&token))
        .await
        .unwrap();
    assert_eq!(store.read_log(0, 10).await.unwrap().len(), 2);
}

#[tokio::test]
async fn observed_no_token_commits_with_current_read_set() {
    let (_dir, store, version, principal, scope) = fixture().await;
    let tokens = TokenCodec::new(b"shared-key".to_vec()).unwrap();
    let mut command = request("");
    command.items[0].expected_versions.clear();
    let service = ActionService::new(&store, ActionLimits::new(8, 8, 65536).unwrap(), &tokens);
    service
        .execute(&principal, &scope, version, command)
        .await
        .unwrap();
    assert_eq!(store.read_log(0, 10).await.unwrap().len(), 2);
}

#[tokio::test]
async fn token_definition_change_is_rejected_without_action_writes() {
    let (dir, store, version, principal, scope) = fixture().await;
    let tokens = TokenCodec::new(b"shared-key".to_vec()).unwrap();
    let token = query_token(&store, &principal, &scope, &tokens).await;
    let changed_defs = DEFS.replace(
        "  name: { type: string, required: true }",
        "  name: { type: string, required: true }\n  note: { type: string }",
    );
    let loaded = load_from_memory(&[
        ("ontology.yaml", "apiName: demo\n"),
        ("defs.yaml", changed_defs.as_str()),
    ]);
    assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
    let next_version = reload(&store, &loaded.ontology.unwrap(), &ReloadOptions::default())
        .await
        .unwrap()
        .def_version
        .unwrap();
    assert_ne!(version, next_version);
    let service = ActionService::new(&store, ActionLimits::new(8, 8, 65536).unwrap(), &tokens);
    let error = service
        .execute(&principal, &scope, next_version, request(&token))
        .await
        .expect_err("prior definition token must not commit");
    assert!(
        matches!(error, ontoforge_action::ActionError::VersionConflict),
        "{error:?}"
    );
    assert_unchanged(&store).await;
    assert_no_action_rows(&dir).await;
}

#[tokio::test]
async fn token_conflicts_after_edit_scope_or_key_change() {
    for changed in ["edit", "scope", "key", "principal", "legacy"] {
        let (dir, store, version, principal, scope) = fixture().await;
        let tokens = TokenCodec::new(b"shared-key".to_vec()).unwrap();
        let token = query_token(&store, &principal, &scope, &tokens).await;
        let mut changed_scope = scope.clone();
        let mut changed_principal = principal.clone();
        let rotated = TokenCodec::new(b"rotated-key".to_vec()).unwrap();
        let (token, verifier) = match changed {
            "edit" => {
                store
                    .commit(Commit {
                        actor: Actor::Principal(principal.id.to_string()),
                        action: "otherEdit".to_owned(),
                        def_version: version,
                        params: serde_json::Value::Null,
                        edits: vec![Edit::Object {
                            type_name: "Gate".to_owned().try_into().unwrap(),
                            pk: PropertyValue::String("A1".to_owned()),
                            op: ObjectOp::Set(BTreeMap::from([(
                                "name".to_owned().try_into().unwrap(),
                                PropertyValue::String("Changed".to_owned()),
                            )])),
                        }],
                    })
                    .await
                    .unwrap();
                (token, &tokens)
            }
            "scope" => {
                changed_scope
                    .types
                    .get_mut("Gate")
                    .unwrap()
                    .columns
                    .remove("name");
                (token, &tokens)
            }
            "principal" => {
                changed_principal.id = PrincipalId::new("https://idp.example.com", "bob");
                (token, &tokens)
            }
            "legacy" => (token.trim_start_matches("v1.").to_owned(), &tokens),
            _ => (token, &rotated),
        };
        let service = ActionService::new(&store, ActionLimits::new(8, 8, 65536).unwrap(), verifier);
        let error = service
            .execute(&changed_principal, &changed_scope, version, request(&token))
            .await
            .expect_err(changed);
        assert!(
            matches!(error, ontoforge_action::ActionError::VersionConflict),
            "{changed}: {error:?}"
        );
        if changed == "edit" {
            assert_eq!(store.read_log(0, 10).await.unwrap().len(), 2);
        } else {
            assert_unchanged(&store).await;
        }
        assert_no_action_rows(&dir).await;
    }
}

#[tokio::test]
async fn token_malformed_is_rejected_without_writes() {
    let (dir, store, version, principal, scope) = fixture().await;
    let tokens = TokenCodec::new(b"version-test-key".to_vec()).unwrap();
    let service = ActionService::new(&store, ActionLimits::new(8, 8, 65536).unwrap(), &tokens);
    let error = service
        .execute(&principal, &scope, version, request("v1.bogus"))
        .await
        .expect_err("a forged token must not commit");
    assert_eq!(error.to_string(), "the observed edit version changed");
    let active = store.active_definition().await.unwrap();
    let row = store
        .read_object_edit(
            &active,
            &"Gate".to_owned().try_into().unwrap(),
            &PropertyValue::String("A1".to_owned()),
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.row.version, 1);
    assert_eq!(store.read_log(0, 10).await.unwrap().len(), 1);
    assert_no_action_rows(&dir).await;
}
