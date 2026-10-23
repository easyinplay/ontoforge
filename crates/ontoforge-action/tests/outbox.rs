use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use camino::Utf8PathBuf;
use ontoforge_action::{ActionItem, ActionLimits, ActionOutcome, ActionRequest, ActionService};
use ontoforge_auth::{Principal, PrincipalId, scope::{PrincipalScope, TypeScope}};
use ontoforge_expr::{Attrs, Pred};
use ontoforge_query::TokenCodec;
use ontoforge_schema::load_from_memory;
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::store::{BackendSettings, EditStore, StoreSettings, sqlite_connect_options};
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
apiName: createGate
parameters:
  code: { type: string, required: true }
  name: { type: string, required: true }
rules:
  - kind: createObject
    objectType: Gate
    properties:
      code: { param: code }
      name: { param: name }
";

#[tokio::test]
async fn payload_is_minimal_and_versioned() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = Utf8PathBuf::from_path_buf(dir.path().join("edit.db")).expect("utf8 path");
    let store = EditStore::open(StoreSettings {
        backend: BackendSettings::Sqlite { path: path.clone() },
        lock_timeout: Duration::from_secs(5),
        probe_timeout: Duration::from_secs(5),
        trino_catalog: None,
    }).await.expect("open store");
    let loaded = load_from_memory(&[("ontology.yaml", "apiName: demo\n"), ("defs.yaml", DEFS)]);
    assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
    let version = reload(&store, &loaded.ontology.expect("ontology"), &ReloadOptions::default())
        .await.expect("reload").def_version.expect("version");
    let principal = Principal {
        id: PrincipalId::new("test", "alice"),
        issuer: "test".to_owned(),
        subject: "alice".to_owned(),
        groups: BTreeSet::new(),
        attrs: Attrs::default(),
    };
    let mut scope = PrincipalScope::default();
    scope.actions.insert("createGate".to_owned().try_into().expect("action"));
    scope.types.insert("Gate".to_owned().try_into().expect("type"), TypeScope {
        row: Pred::Const(true),
        columns: ["code", "name"].into_iter().map(|name| {
            (name.to_owned().try_into().expect("column"), Pred::Const(true))
        }).collect(),
    });
    let tokens = TokenCodec::new(b"outbox-test-key".to_vec()).expect("tokens");
    let service = ActionService::new(&store, ActionLimits::new(8, 8, 65536).expect("limits"), &tokens);
    let outcome = service.execute(&principal, &scope, version, ActionRequest {
        action: "createGate".to_owned(),
        command_id: Some("cmd-one".to_owned()),
        items: vec![ActionItem {
            params: serde_json::from_value(serde_json::json!({"code":"G1", "name":"Private Name"})).expect("params"),
            expected_versions: BTreeMap::new(),
        }],
        validate_only: false,
    }).await.expect("Action commit");
    let ActionOutcome::Committed(receipt) = outcome else { panic!("expected committed Action") };
    let pool = SqlitePoolOptions::new().max_connections(1)
        .connect_with(sqlite_connect_options(&path)).await.expect("inspect store");
    let payload: String = sqlx::query_scalar("SELECT payload FROM of_webhook_outbox WHERE event_id = ?")
        .bind(&receipt.event_id).fetch_one(&pool).await.expect("outbox payload");
    let value: serde_json::Value = serde_json::from_str(&payload).expect("JSON payload");
    assert_eq!(value["schema_version"], 1, "webhook payload must declare its format");
    assert_eq!(value["event_id"], receipt.event_id);
    assert!(!payload.contains("Private Name"), "row values must not leave the outbox");
}
