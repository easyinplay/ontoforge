use chrono::{TimeZone, Utc};
use std::collections::{BTreeMap, BTreeSet};

use ontoforge_action::{ActionItem, ActionLimits, ActionOutcome, ActionRequest, ActionService};
use ontoforge_auth::{
    Principal, PrincipalId,
    scope::{PrincipalScope, TypeScope},
};
use ontoforge_expr::{Attrs, Pred};
use ontoforge_query::TokenCodec;
use ontoforge_schema::load_from_memory;
use ontoforge_storage::{
    commit::{Actor, Commit, Edit, ObjectOp},
    reload::{ReloadOptions, reload},
    value::PropertyValue,
};

#[path = "../../ontoforge-storage/tests/support/mod.rs"]
mod support;

const DEFS: &str = "kind: ObjectType
apiName: Gate
backing: editOnly
primaryKey: code
properties:
  code: { type: string, required: true }
  name: { type: string, required: true }
  changedAt: { type: timestamp }
---
kind: ActionType
apiName: renameGate
parameters:
  gate: { type: { objectReference: Gate }, required: true }
  name: { type: string, default: Default }
rules:
  - kind: modifyObject
    object: gate
    properties:
      name: { param: name }
      changedAt: { currentTime: {} }
";

struct Fixture {
    db: support::TestStore,
    principal: Principal,
    scope: PrincipalScope,
    version: i64,
    tokens: TokenCodec,
}

async fn fixture() -> Fixture {
    let db = support::sqlite_store().await;
    let loaded = load_from_memory(&[("ontology.yaml", "apiName: demo\n"), ("defs.yaml", DEFS)]);
    assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
    let version = reload(
        &db.store,
        &loaded.ontology.unwrap(),
        &ReloadOptions::default(),
    )
    .await
    .unwrap()
    .def_version
    .unwrap();
    let principal = Principal {
        id: PrincipalId::new("https://idp.example.com", "alice"),
        issuer: "https://idp.example.com".to_owned(),
        subject: "alice".to_owned(),
        groups: BTreeSet::new(),
        attrs: Attrs::new(),
    };
    db.store
        .commit(Commit {
            actor: Actor::Principal(principal.id.as_str().to_owned()),
            action: "seed".to_owned(),
            def_version: version,
            params: serde_json::Value::Null,
            edits: vec![Edit::Object {
                type_name: "Gate".to_owned().try_into().unwrap(),
                pk: PropertyValue::String("A1".to_owned()),
                op: ObjectOp::Set(BTreeMap::from([(
                    "name".to_owned().try_into().unwrap(),
                    PropertyValue::String("Original".to_owned()),
                )])),
            }],
        })
        .await
        .unwrap();
    let mut scope = PrincipalScope::default();
    scope
        .actions
        .insert("renameGate".to_owned().try_into().unwrap());
    scope.types.insert(
        "Gate".to_owned().try_into().unwrap(),
        TypeScope {
            row: Pred::Const(true),
            columns: ["code", "name", "changedAt"]
                .into_iter()
                .map(|name| (name.to_owned().try_into().unwrap(), Pred::Const(true)))
                .collect(),
        },
    );
    Fixture {
        db,
        principal,
        scope,
        version,
        tokens: TokenCodec::new(b"idempotency-test-key".to_vec()).unwrap(),
    }
}

fn rename(command: &str, name: Option<&str>) -> ActionRequest {
    let mut params = serde_json::Map::new();
    params.insert("gate".to_owned(), serde_json::json!("A1"));
    if let Some(name) = name {
        params.insert("name".to_owned(), serde_json::json!(name));
    }
    ActionRequest {
        action: "renameGate".to_owned(),
        command_id: Some(command.to_owned()),
        items: vec![ActionItem {
            params,
            expected_versions: BTreeMap::new(),
        }],
        validate_only: false,
    }
}

fn receipt(outcome: ActionOutcome) -> ontoforge_action::ActionReceipt {
    let ActionOutcome::Committed(receipt) = outcome else {
        panic!("expected a receipt")
    };
    receipt
}

#[tokio::test]
async fn expired_command_can_be_claimed_as_new_without_deleting_audit() {
    let f = fixture().await;
    let service = ActionService::new(
        &f.db.store,
        ActionLimits::new(8, 8, 65536).unwrap(),
        &f.tokens,
    );
    let request = rename("expiring", Some("New"));
    let first = receipt(
        service
            .execute(&f.principal, &f.scope, f.version, request.clone())
            .await
            .unwrap(),
    );
    let pool = f.db.side_pool().await;
    sqlx::query("UPDATE of_action_command SET expires_at = '2000-01-01T00:00:00.000Z' WHERE principal_id = ? AND command_id = ?")
        .bind(f.principal.id.as_str()).bind("expiring").execute(&pool).await.unwrap();
    let second = receipt(
        service
            .execute(&f.principal, &f.scope, f.version, request)
            .await
            .unwrap(),
    );
    assert_ne!(
        first.event_id, second.event_id,
        "an expired key must execute again"
    );
    let logs: i64 = sqlx::query_scalar("SELECT count(*) FROM of_edit_log")
        .fetch_one(&pool)
        .await
        .unwrap();
    let outbox: i64 = sqlx::query_scalar("SELECT count(*) FROM of_webhook_outbox")
        .fetch_one(&pool)
        .await
        .unwrap();
    let commands: i64 = sqlx::query_scalar("SELECT count(*) FROM of_action_command")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!((logs, outbox, commands), (3, 2, 1));
}

#[tokio::test]
async fn fingerprint_ignores_key_order_but_preserves_explicit_default() {
    let f = fixture().await;
    let service = ActionService::new(
        &f.db.store,
        ActionLimits::new(8, 8, 65536).unwrap(),
        &f.tokens,
    );
    let mut first = rename("order-first", Some("New"));
    let mut second = rename("order-second", Some("New"));
    first.items[0].params = serde_json::from_str(r#"{"gate":"A1","name":"New"}"#).unwrap();
    second.items[0].params = serde_json::from_str(r#"{"name":"New","gate":"A1"}"#).unwrap();
    receipt(
        service
            .execute(&f.principal, &f.scope, f.version, first)
            .await
            .unwrap(),
    );
    receipt(
        service
            .execute(&f.principal, &f.scope, f.version, second)
            .await
            .unwrap(),
    );
    let pool = f.db.side_pool().await;
    let hashes: Vec<String> =
        sqlx::query_scalar("SELECT request_hash FROM of_action_command ORDER BY command_id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(hashes.len(), 2);
    assert_eq!(hashes[0], hashes[1]);

    receipt(
        service
            .execute(&f.principal, &f.scope, f.version, rename("presence", None))
            .await
            .unwrap(),
    );
    let explicit = service
        .execute(
            &f.principal,
            &f.scope,
            f.version,
            rename("presence", Some("Default")),
        )
        .await;
    assert!(
        matches!(
            explicit,
            Err(ontoforge_action::ActionError::Commit(
                ontoforge_storage::commit::ActionCommitError::CommandIdReused
            ))
        ),
        "{explicit:?}"
    );
}

#[tokio::test]
async fn replay_across_clock_and_definition_change_keeps_winner_time() {
    let f = fixture().await;
    let first_time = Utc.with_ymd_and_hms(2026, 10, 1, 0, 0, 0).unwrap();
    let later = Utc.with_ymd_and_hms(2026, 10, 2, 0, 0, 0).unwrap();
    let limits = ActionLimits::new(8, 8, 65536).unwrap();
    let first = receipt(
        ActionService::new(&f.db.store, limits, &f.tokens)
            .with_clock(move || first_time)
            .execute(
                &f.principal,
                &f.scope,
                f.version,
                rename("clock", Some("New")),
            )
            .await
            .unwrap(),
    );
    assert_eq!(first.chosen_current_time, "2026-10-01T00:00:00.000Z");

    let changed = DEFS.replace("default: Default", "default: Updated");
    let loaded = load_from_memory(&[
        ("ontology.yaml", "apiName: demo\n"),
        ("changed.yaml", &changed),
    ]);
    assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
    let version = reload(
        &f.db.store,
        &loaded.ontology.unwrap(),
        &ReloadOptions::default(),
    )
    .await
    .unwrap()
    .def_version
    .unwrap();
    assert_ne!(version, f.version);
    let second = receipt(
        ActionService::new(&f.db.store, limits, &f.tokens)
            .with_clock(move || later)
            .execute(
                &f.principal,
                &f.scope,
                version,
                rename("clock", Some("New")),
            )
            .await
            .unwrap(),
    );
    assert_eq!(first, second);
    assert_eq!(f.db.store.read_log(0, 10).await.unwrap().len(), 2);
}

#[tokio::test]
async fn replay_grant_removal_and_action_missing_deny_both_entries() {
    let f = fixture().await;
    let service = ActionService::new(
        &f.db.store,
        ActionLimits::new(8, 8, 65536).unwrap(),
        &f.tokens,
    );
    let original = receipt(
        service
            .execute(
                &f.principal,
                &f.scope,
                f.version,
                rename("grant", Some("New")),
            )
            .await
            .unwrap(),
    );
    let denied = PrincipalScope::default();
    assert!(matches!(
        service
            .execute(
                &f.principal,
                &denied,
                f.version,
                rename("grant", Some("New"))
            )
            .await,
        Err(ontoforge_action::ActionError::ScopeDenied)
    ));
    assert!(matches!(
        service
            .get_command_result(&f.principal, &denied, "renameGate", "grant")
            .await,
        Err(ontoforge_action::ActionError::ScopeDenied)
    ));

    let without_action = DEFS.split("---").next().unwrap();
    let loaded = load_from_memory(&[
        ("ontology.yaml", "apiName: demo\n"),
        ("object.yaml", without_action),
    ]);
    assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
    let version = reload(
        &f.db.store,
        &loaded.ontology.unwrap(),
        &ReloadOptions::default(),
    )
    .await
    .unwrap()
    .def_version
    .unwrap();
    assert!(matches!(
        service
            .execute(
                &f.principal,
                &f.scope,
                version,
                rename("grant", Some("New"))
            )
            .await,
        Err(ontoforge_action::ActionError::ScopeDenied)
    ));
    assert!(matches!(
        service
            .get_command_result(&f.principal, &f.scope, "renameGate", "grant")
            .await,
        Err(ontoforge_action::ActionError::ScopeDenied)
    ));
    assert!(!original.event_id.is_empty());
}

#[tokio::test]
async fn replay_external_definition_removal_does_not_disclose() {
    let f = fixture().await;
    let service = ActionService::new(&f.db.store, ActionLimits::new(8, 8, 65536).unwrap(), &f.tokens);
    receipt(service.execute(&f.principal, &f.scope, f.version, rename("external", Some("New"))).await.unwrap());

    let other = f.db.reopen().await;
    let without_action = DEFS.split("---").next().unwrap();
    let loaded = load_from_memory(&[("ontology.yaml", "apiName: demo\n"), ("object.yaml", without_action)]);
    assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
    reload(&other, &loaded.ontology.unwrap(), &ReloadOptions::default()).await.unwrap();

    let retry = service.execute(&f.principal, &f.scope, f.version, rename("external", Some("New"))).await;
    assert!(matches!(retry, Err(ontoforge_action::ActionError::ScopeDenied)), "{retry:?}");
    let query = service.get_command_result(&f.principal, &f.scope, "renameGate", "external").await;
    assert!(matches!(query, Err(ontoforge_action::ActionError::ScopeDenied)), "{query:?}");
}

#[tokio::test]
async fn replay_row_predicate_change_and_delete_keep_minimal_receipt() {
    let f = fixture().await;
    let service = ActionService::new(
        &f.db.store,
        ActionLimits::new(8, 8, 65536).unwrap(),
        &f.tokens,
    );
    let original = receipt(
        service
            .execute(
                &f.principal,
                &f.scope,
                f.version,
                rename("historical", Some("Sensitive")),
            )
            .await
            .unwrap(),
    );
    let mut hidden = f.scope.clone();
    hidden.types.get_mut("Gate").unwrap().row = Pred::Const(false);
    assert_eq!(
        service
            .get_command_result(&f.principal, &hidden, "renameGate", "historical")
            .await
            .unwrap(),
        Some(original.clone())
    );
    assert_eq!(
        receipt(
            service
                .execute(
                    &f.principal,
                    &hidden,
                    f.version,
                    rename("historical", Some("Sensitive"))
                )
                .await
                .unwrap()
        ),
        original
    );

    f.db.store
        .commit(Commit {
            actor: Actor::Principal(f.principal.id.as_str().to_owned()),
            action: "delete".to_owned(),
            def_version: f.version,
            params: serde_json::Value::Null,
            edits: vec![Edit::Object {
                type_name: "Gate".to_owned().try_into().unwrap(),
                pk: PropertyValue::String("A1".to_owned()),
                op: ObjectOp::Tombstone,
            }],
        })
        .await
        .unwrap();
    assert_eq!(
        service
            .get_command_result(&f.principal, &hidden, "renameGate", "historical")
            .await
            .unwrap(),
        Some(original.clone())
    );
    let summary = serde_json::to_string(&original).unwrap();
    assert!(!summary.contains("Sensitive") && !summary.contains("Original"));
}

#[tokio::test]
async fn replay_cross_principal_lookup_matches_missing() {
    let f = fixture().await;
    let service = ActionService::new(
        &f.db.store,
        ActionLimits::new(8, 8, 65536).unwrap(),
        &f.tokens,
    );
    receipt(
        service
            .execute(
                &f.principal,
                &f.scope,
                f.version,
                rename("private", Some("New")),
            )
            .await
            .unwrap(),
    );
    let mut bob = f.principal.clone();
    bob.id = PrincipalId::new("https://idp.example.com", "bob");
    bob.subject = "bob".to_owned();
    let other = service
        .get_command_result(&bob, &f.scope, "renameGate", "private")
        .await
        .unwrap();
    let missing = service
        .get_command_result(&bob, &f.scope, "renameGate", "missing")
        .await
        .unwrap();
    assert_eq!(other, missing);
    assert_eq!(other, None);
    assert!(
        service
            .get_command_result(&f.principal, &f.scope, "renameGate", "private")
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn replay_release_keeps_minimal_receipt_available() {
    let f = fixture().await;
    let service = ActionService::new(
        &f.db.store,
        ActionLimits::new(8, 8, 65536).unwrap(),
        &f.tokens,
    );
    let original = receipt(
        service
            .execute(
                &f.principal,
                &f.scope,
                f.version,
                rename("released", Some("New")),
            )
            .await
            .unwrap(),
    );
    // The source-backed release workflow is exercised in 05-06; emulate its row flag here.
    let active = f.db.store.active_definition().await.unwrap();
    let table = active.physical.objects.get("Gate").unwrap();
    let sql = format!(
        "UPDATE \"{}\" SET of_released = 1 WHERE \"{}\" = ?",
        table.table, table.pk.column
    );
    sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind("A1")
        .execute(&f.db.side_pool().await)
        .await
        .unwrap();
    let row =
        f.db.store
            .read_object_edit(
                &active,
                &"Gate".to_owned().try_into().unwrap(),
                &PropertyValue::String("A1".to_owned()),
            )
            .await
            .unwrap()
            .unwrap();
    assert!(row.row.released);
    assert_eq!(
        service
            .get_command_result(&f.principal, &f.scope, "renameGate", "released")
            .await
            .unwrap(),
        Some(original.clone())
    );
    assert_eq!(
        receipt(
            service
                .execute(
                    &f.principal,
                    &f.scope,
                    f.version,
                    rename("released", Some("New"))
                )
                .await
                .unwrap()
        ),
        original
    );
}

#[tokio::test]
async fn expired_clock_boundary_is_seven_days() {
    let f = fixture().await;
    let start = Utc.with_ymd_and_hms(2026, 10, 1, 0, 0, 0).unwrap();
    let before = Utc.with_ymd_and_hms(2026, 10, 7, 23, 59, 59).unwrap();
    let expired = Utc.with_ymd_and_hms(2026, 10, 8, 0, 0, 0).unwrap();
    let limits = ActionLimits::new(8, 8, 65536).unwrap();
    let request = rename("boundary", Some("New"));
    let first = receipt(
        ActionService::new(&f.db.store, limits, &f.tokens)
            .with_clock(move || start)
            .execute(&f.principal, &f.scope, f.version, request.clone())
            .await
            .unwrap(),
    );
    let replay = receipt(
        ActionService::new(&f.db.store, limits, &f.tokens)
            .with_clock(move || before)
            .execute(&f.principal, &f.scope, f.version, request.clone())
            .await
            .unwrap(),
    );
    assert_eq!(first, replay);
    let second = receipt(
        ActionService::new(&f.db.store, limits, &f.tokens)
            .with_clock(move || expired)
            .execute(&f.principal, &f.scope, f.version, request)
            .await
            .unwrap(),
    );
    assert_ne!(first.event_id, second.event_id);
    assert_eq!(second.chosen_current_time, "2026-10-08T00:00:00.000Z");
}
