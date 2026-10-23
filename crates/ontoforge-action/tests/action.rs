use camino::Utf8PathBuf;
use ontoforge_action::{ActionLimits, ActionRequest, ActionService};
use ontoforge_auth::{Principal, PrincipalId, scope::PrincipalScope};
use ontoforge_expr::rowpred::Attrs;
use ontoforge_schema::load_from_memory;
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::store::{BackendSettings, EditStore, StoreSettings, sqlite_connect_options};
use sqlx::sqlite::SqlitePoolOptions;
use std::collections::BTreeSet;
use std::time::Duration;

#[test]
fn action_request_wire_cannot_override_host_limits() {
    let request = serde_json::json!({
        "action": "renameGate",
        "commandId": "command-1",
        "items": [{"params": {"gate": "A1", "name": "New"}}],
        "validateOnly": false
    });
    let parsed: ActionRequest = serde_json::from_value(request.clone()).expect("public request");
    assert_eq!(parsed.command_id.as_deref(), Some("command-1"));
    assert_eq!(parsed.items.len(), 1);
    let mut untrusted = request;
    untrusted["maxItems"] = serde_json::json!(1000);
    assert!(serde_json::from_value::<ActionRequest>(untrusted).is_err());
    assert!(ActionLimits::new(0, 8, 1024).is_err());
}

#[tokio::test]
async fn action_batch_schema_exists() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = Utf8PathBuf::from_path_buf(dir.path().join("edit.db")).expect("utf8 path");
    let _store = EditStore::open(StoreSettings {
        backend: BackendSettings::Sqlite { path: path.clone() },
        lock_timeout: Duration::from_secs(5),
        probe_timeout: Duration::from_secs(5),
        trino_catalog: None,
    })
    .await
    .expect("open store");
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(sqlite_connect_options(&path))
        .await
        .expect("inspect store");
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name IN ('of_action_command', 'of_webhook_outbox') ORDER BY name",
    )
    .fetch_all(&pool)
    .await
    .expect("action table query");
    assert_eq!(tables, ["of_action_command", "of_webhook_outbox"]);
}

#[tokio::test]
async fn params_report_all_errors() {
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
  count: { type: integer }
  tags: { type: array, items: string }
rules:
  - kind: modifyObject
    object: gate
    properties:
      name: { param: name }
";
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
    let version = reload(
        &store,
        &loaded.ontology.expect("ontology"),
        &ReloadOptions::default(),
    )
    .await
    .expect("reload")
    .def_version
    .expect("version");
    let principal = Principal {
        id: PrincipalId::new("https://idp.example.com", "alice"),
        issuer: "https://idp.example.com".to_owned(),
        subject: "alice".to_owned(),
        groups: BTreeSet::new(),
        attrs: Attrs::new(),
    };
    let mut scope = PrincipalScope::default();
    scope.actions.insert("renameGate".to_owned().try_into().expect("action"));
    let error = ActionService::new(&store, ActionLimits::new(8, 8, 65536).expect("limits"))
        .execute(
            &principal,
            &scope,
            version,
            ActionRequest {
                action: "renameGate".to_owned(),
                command_id: Some("bad-params".to_owned()),
                items: vec![ontoforge_action::ActionItem {
                    params: serde_json::from_value(serde_json::json!({
                        "gate": 42,
                        "count": "wrong",
                        "tags": ["valid", 2],
                        "extra": true
                    }))
                    .expect("params"),
                }],
                validate_only: false,
            },
        )
        .await
        .expect_err("all invalid parameters must be rejected");
    let message = error.to_string();
    for name in ["gate", "name", "count", "tags", "extra"] {
        assert!(message.contains(name), "missing {name} in {message}");
    }
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(sqlite_connect_options(&path))
        .await
        .expect("inspect store");
    let commands: i64 = sqlx::query_scalar("SELECT count(*) FROM of_action_command")
        .fetch_one(&pool).await.expect("command count");
    let outbox: i64 = sqlx::query_scalar("SELECT count(*) FROM of_webhook_outbox")
        .fetch_one(&pool).await.expect("outbox count");
    let logs: i64 = sqlx::query_scalar("SELECT count(*) FROM of_edit_log")
        .fetch_one(&pool).await.expect("log count");
    assert_eq!((commands, outbox, logs), (0, 0, 0));
}
