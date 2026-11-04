use camino::Utf8PathBuf;
use ontoforge_action::{ActionError, ActionLimits, ActionOutcome, ActionRequest, ActionService};
use ontoforge_auth::{
    Principal, PrincipalId,
    scope::{PrincipalScope, TypeScope},
};
use ontoforge_expr::{Pred, rowpred::Attrs};
use ontoforge_query::TokenCodec;
use ontoforge_schema::load_from_memory;
use ontoforge_storage::commit::{ActionCommitError, Actor, Commit, Edit, EditError, ObjectOp};
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::store::{BackendSettings, EditStore, StoreSettings, sqlite_connect_options};
use ontoforge_storage::value::PropertyValue;
use sqlx::sqlite::SqlitePoolOptions;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;
use std::time::Duration;

fn tokens() -> &'static TokenCodec {
    static TOKENS: OnceLock<TokenCodec> = OnceLock::new();
    TOKENS.get_or_init(|| TokenCodec::new(b"action-test-key".to_vec()).expect("token codec"))
}

fn allow_types(scope: &mut PrincipalScope) {
    for type_name in ["Gate", "Pilot"] {
        scope.types.insert(
            type_name.to_owned().try_into().expect("type"),
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
    }
}

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
#[expect(
    clippy::too_many_lines,
    reason = "SQLite fixture checks parameter aggregation and no writes"
)]
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
    scope
        .actions
        .insert("renameGate".to_owned().try_into().expect("action"));
    let error = ActionService::new(
        &store,
        ActionLimits::new(8, 8, 65536).expect("limits"),
        tokens(),
    )
    .execute(
        &principal,
        &scope,
        version,
        ActionRequest {
            action: "renameGate".to_owned(),
            command_id: Some("bad-params".to_owned()),
            items: vec![
                ontoforge_action::ActionItem {
                    params: serde_json::from_value(serde_json::json!({
                        "gate": 42,
                        "count": "wrong",
                        "tags": ["valid", 2],
                        "extra": true
                    }))
                    .expect("params"),
                    expected_versions: BTreeMap::default(),
                },
                ontoforge_action::ActionItem {
                    params: serde_json::from_value(serde_json::json!({
                        "name": 5,
                        "tags": [false]
                    }))
                    .expect("params"),
                    expected_versions: BTreeMap::default(),
                },
            ],
            validate_only: false,
        },
    )
    .await
    .expect_err("all invalid parameters must be rejected");
    let ActionError::ParameterErrors(problems) = &error else {
        panic!("expected parameter problems, got {error}");
    };
    assert_eq!(
        problems.len(),
        8,
        "all items must report errors: {problems:?}"
    );
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
        .fetch_one(&pool)
        .await
        .expect("command count");
    let outbox: i64 = sqlx::query_scalar("SELECT count(*) FROM of_webhook_outbox")
        .fetch_one(&pool)
        .await
        .expect("outbox count");
    let logs: i64 = sqlx::query_scalar("SELECT count(*) FROM of_edit_log")
        .fetch_one(&pool)
        .await
        .expect("log count");
    assert_eq!((commands, outbox, logs), (0, 0, 0));
}

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "one SQLite fixture covers all rule kinds and ordinal receipts"
)]
async fn batch_all_rule_kinds_expand_in_declared_order() {
    const DEFS: &str = "kind: ObjectType
apiName: Gate
backing: editOnly
primaryKey: code
properties:
  code: { type: string, required: true }
  name: { type: string, required: true }
---
kind: ObjectType
apiName: Pilot
backing: editOnly
primaryKey: code
properties:
  code: { type: string, required: true }
  name: { type: string, required: true }
---
kind: LinkType
apiName: gateCrew
cardinality: manyToMany
from: { objectType: Gate, apiName: crew }
to: { objectType: Pilot, apiName: gates }
backing: editOnly
---
kind: ActionType
apiName: updateGates
parameters:
  gate: { type: { objectReference: Gate }, required: true }
  doomed: { type: { objectReference: Gate }, required: true }
  pilot: { type: { objectReference: Pilot }, required: true }
  newCode: { type: string, required: true }
  name: { type: string, required: true }
rules:
  - kind: createObject
    objectType: Gate
    properties:
      code: { param: newCode }
      name: { param: name }
  - kind: modifyObject
    object: gate
    properties:
      name: { param: name }
  - kind: addLink
    link: gateCrew
    from: gate
    to: pilot
  - kind: removeLink
    link: gateCrew
    from: gate
    to: pilot
  - kind: deleteObject
    object: doomed
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
    let object = |type_name: &str, pk: &str, name: Option<&str>| {
        let mut properties = BTreeMap::new();
        if let Some(name) = name {
            properties.insert(
                "name".to_owned().try_into().expect("property"),
                PropertyValue::String(name.to_owned()),
            );
        }
        Edit::Object {
            type_name: type_name.to_owned().try_into().expect("type"),
            pk: PropertyValue::String(pk.to_owned()),
            op: ObjectOp::Set(properties),
        }
    };
    store
        .commit(Commit {
            actor: Actor::Principal("https://idp.example.com|alice".to_owned()),
            action: "seed".to_owned(),
            def_version: version,
            params: serde_json::Value::Null,
            edits: vec![
                object("Gate", "A1", Some("Old")),
                object("Gate", "B1", Some("Old")),
                object("Pilot", "P1", Some("Pilot")),
            ],
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
    let mut scope = PrincipalScope::default();
    scope
        .actions
        .insert("updateGates".to_owned().try_into().expect("action"));
    allow_types(&mut scope);
    let limited = ActionService::new(
        &store,
        ActionLimits::new(8, 1, 65536).expect("limits"),
        tokens(),
    );
    let limited_result = limited
        .execute(
            &principal,
            &scope,
            version,
            ActionRequest {
                action: "updateGates".to_owned(),
                command_id: Some("too-many-edits".to_owned()),
                items: vec![ontoforge_action::ActionItem {
                    params: serde_json::from_value(serde_json::json!({
                        "gate": "hidden", "doomed": "B1", "pilot": "P1",
                        "newCode": "C1", "name": "New"
                    }))
                    .expect("params"),
                    expected_versions: BTreeMap::default(),
                }],
                validate_only: false,
            },
        )
        .await
        .expect_err("expanded edits must be bounded before target reads");
    assert!(
        limited_result
            .to_string()
            .contains("edit count exceeds host limits"),
        "unexpected limit result: {limited_result}"
    );
    let receipt = ActionService::new(
        &store,
        ActionLimits::new(8, 16, 65536).expect("limits"),
        tokens(),
    )
    .execute(
        &principal,
        &scope,
        version,
        ActionRequest {
            action: "updateGates".to_owned(),
            command_id: Some("all-rules".to_owned()),
            items: vec![ontoforge_action::ActionItem {
                params: serde_json::from_value(serde_json::json!({
                    "gate": "A1", "doomed": "B1", "pilot": "P1",
                    "newCode": "C1", "name": "New"
                }))
                .expect("params"),
                expected_versions: BTreeMap::default(),
            }],
            validate_only: false,
        },
    )
    .await;
    assert!(
        receipt.is_ok(),
        "five ordered rules must commit: {receipt:?}"
    );
    let ActionOutcome::Committed(receipt) = receipt.expect("receipt") else {
        panic!("normal submission must commit")
    };
    assert_eq!(receipt.items.len(), 1);
    let ordinals: Vec<_> = receipt.items[0]
        .edits
        .iter()
        .map(|edit| edit.rule_ordinal)
        .collect();
    assert_eq!(ordinals, [0, 1, 2, 3, 4]);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(sqlite_connect_options(&path))
        .await
        .expect("inspect store");
    let commands: i64 = sqlx::query_scalar("SELECT count(*) FROM of_action_command")
        .fetch_one(&pool)
        .await
        .expect("commands");
    let outbox: i64 = sqlx::query_scalar("SELECT count(*) FROM of_webhook_outbox")
        .fetch_one(&pool)
        .await
        .expect("outbox");
    assert_eq!((commands, outbox), (1, 1));
    assert_eq!(store.read_log(0, 10).await.expect("log").len(), 2);
}

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "SQLite fixture verifies repeated edit versions and batch rollback"
)]
async fn batch_ordinal_and_same_pk_order_are_stable() {
    const DEFS: &str = "kind: ObjectType
apiName: Gate
backing: editOnly
primaryKey: code
properties:
  code: { type: string, required: true }
  name: { type: string, required: true }
---
kind: ActionType
apiName: changeGate
parameters:
  gate: { type: { objectReference: Gate }, required: true }
  name: { type: string, required: true }
  second: { type: string, default: Default }
rules:
  - kind: modifyObject
    object: gate
    properties:
      name: { param: name }
  - kind: modifyObject
    object: gate
    properties:
      name: { param: second }
---
kind: ActionType
apiName: deleteGateTwice
parameters:
  gate: { type: { objectReference: Gate }, required: true }
  name: { type: string, required: true }
rules:
  - kind: modifyObject
    object: gate
    properties:
      name: { param: name }
  - kind: deleteObject
    object: gate
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
                    PropertyValue::String("Old".to_owned()),
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
    let mut scope = PrincipalScope::default();
    scope
        .actions
        .insert("changeGate".to_owned().try_into().expect("action"));
    scope
        .actions
        .insert("deleteGateTwice".to_owned().try_into().expect("action"));
    allow_types(&mut scope);
    let service = ActionService::new(
        &store,
        ActionLimits::new(8, 8, 65536).expect("limits"),
        tokens(),
    );
    let item = |name: &str| ontoforge_action::ActionItem {
        params: serde_json::from_value(serde_json::json!({"gate": "A1", "name": name}))
            .expect("params"),
        expected_versions: BTreeMap::default(),
    };
    let receipt = service
        .execute(
            &principal,
            &scope,
            version,
            ActionRequest {
                action: "changeGate".to_owned(),
                command_id: Some("ordered".to_owned()),
                items: vec![item("First"), item("Second")],
                validate_only: false,
            },
        )
        .await
        .expect("ordered batch");
    let ActionOutcome::Committed(receipt) = receipt else {
        panic!("normal submission must commit")
    };
    assert_eq!(receipt.items.len(), 2);
    let edits: Vec<_> = receipt
        .items
        .iter()
        .flat_map(|item| {
            item.edits.iter().map(move |edit| {
                (
                    item.item_ordinal,
                    edit.rule_ordinal,
                    edit.edit_ordinal,
                    edit.version_after,
                )
            })
        })
        .collect();
    assert_eq!(
        edits,
        [(0, 0, 0, 2), (0, 1, 1, 3), (1, 0, 2, 4), (1, 1, 3, 5)]
    );
    let active = store.active_definition().await.expect("active");
    let row = store
        .read_object_edit(
            &active,
            &"Gate".to_owned().try_into().expect("type"),
            &PropertyValue::String("A1".to_owned()),
        )
        .await
        .expect("read")
        .expect("row");
    assert_eq!(row.row.version, 5);
    assert_eq!(
        row.row.set["name"],
        PropertyValue::String("Default".to_owned())
    );

    let late_failure = service
        .execute(
            &principal,
            &scope,
            version,
            ActionRequest {
                action: "deleteGateTwice".to_owned(),
                command_id: Some("late-rollback".to_owned()),
                items: vec![item("First"), item("Second")],
                validate_only: false,
            },
        )
        .await;
    assert!(
        matches!(
            late_failure,
            Err(ActionError::Commit(ActionCommitError::Edit(
                EditError::Tombstoned
            )))
        ),
        "later edit must fail in the transaction: {late_failure:?}"
    );

    let mut invalid = item("Ignored");
    invalid
        .params
        .insert("extra".to_owned(), serde_json::json!(true));
    let failed = service
        .execute(
            &principal,
            &scope,
            version,
            ActionRequest {
                action: "changeGate".to_owned(),
                command_id: Some("rollback".to_owned()),
                items: vec![item("Valid"), invalid],
                validate_only: false,
            },
        )
        .await;
    assert!(
        failed.is_err(),
        "later invalid item must reject whole batch"
    );
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(sqlite_connect_options(&path))
        .await
        .expect("inspect");
    let commands: i64 = sqlx::query_scalar("SELECT count(*) FROM of_action_command")
        .fetch_one(&pool)
        .await
        .expect("commands");
    let outbox: i64 = sqlx::query_scalar("SELECT count(*) FROM of_webhook_outbox")
        .fetch_one(&pool)
        .await
        .expect("outbox");
    assert_eq!((commands, outbox), (1, 1));
    assert_eq!(store.read_log(0, 10).await.expect("log").len(), 2);
}

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "SQLite fixture checks validation and five persisted table counts"
)]
async fn validate_only_runs_full_pipeline_without_writes() {
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
    scope
        .actions
        .insert("createGate".to_owned().try_into().expect("action"));
    scope.types.insert(
        "Gate".to_owned().try_into().expect("type"),
        TypeScope {
            row: Pred::Const(true),
            columns: BTreeMap::from([
                (
                    "code".to_owned().try_into().expect("code"),
                    Pred::Const(true),
                ),
                (
                    "name".to_owned().try_into().expect("name"),
                    Pred::Const(true),
                ),
            ]),
        },
    );
    let request = ActionRequest {
        action: "createGate".to_owned(),
        command_id: None,
        items: vec![ontoforge_action::ActionItem {
            params: serde_json::from_value(serde_json::json!({
                "code": "A1", "name": "New"
            }))
            .expect("params"),
            expected_versions: BTreeMap::default(),
        }],
        validate_only: true,
    };
    let service = ActionService::new(
        &store,
        ActionLimits::new(8, 8, 65536).expect("limits"),
        tokens(),
    );
    let outcome = service
        .execute(&principal, &scope, version, request.clone())
        .await;
    assert!(
        matches!(outcome, Ok(ActionOutcome::Validated)),
        "valid dry-run must succeed: {outcome:?}"
    );
    let denied = service
        .execute(
            &principal,
            &PrincipalScope::default(),
            version,
            request.clone(),
        )
        .await;
    assert!(matches!(denied, Err(ActionError::ScopeDenied)));
    let mut hidden_post = scope.clone();
    hidden_post.types.get_mut("Gate").expect("type").row = Pred::Const(false);
    let failed_dry_run = service
        .execute(&principal, &hidden_post, version, request.clone())
        .await;
    assert!(matches!(failed_dry_run, Err(ActionError::ScopeDenied)));
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(sqlite_connect_options(&path))
        .await
        .expect("inspect");
    let skeletons: i64 = sqlx::query_scalar("SELECT count(*) FROM of_obj_gate")
        .fetch_one(&pool)
        .await
        .expect("objects");
    let edits: i64 = sqlx::query_scalar("SELECT count(*) FROM of_edit_log_entry")
        .fetch_one(&pool)
        .await
        .expect("edits");
    let logs: i64 = sqlx::query_scalar("SELECT count(*) FROM of_edit_log")
        .fetch_one(&pool)
        .await
        .expect("logs");
    let commands: i64 = sqlx::query_scalar("SELECT count(*) FROM of_action_command")
        .fetch_one(&pool)
        .await
        .expect("commands");
    let outbox: i64 = sqlx::query_scalar("SELECT count(*) FROM of_webhook_outbox")
        .fetch_one(&pool)
        .await
        .expect("outbox");
    assert_eq!((skeletons, edits, logs, commands, outbox), (0, 0, 0, 0, 0));
    let real = ActionRequest {
        command_id: Some("after-dry-run".to_owned()),
        validate_only: false,
        ..request
    };
    let outcome = service
        .execute(&principal, &scope, version, real)
        .await
        .expect("real commit");
    let ActionOutcome::Committed(receipt) = outcome else {
        panic!("committed")
    };
    assert_eq!(
        receipt.log_seq, 1,
        "dry-run must not consume a log sequence"
    );
}
