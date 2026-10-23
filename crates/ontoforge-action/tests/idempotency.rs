use std::collections::{BTreeMap, BTreeSet};

use ontoforge_action::{ActionItem, ActionLimits, ActionOutcome, ActionRequest, ActionService};
use ontoforge_auth::{Principal, PrincipalId, scope::{PrincipalScope, TypeScope}};
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
    let version = reload(&db.store, &loaded.ontology.unwrap(), &ReloadOptions::default())
        .await.unwrap().def_version.unwrap();
    let principal = Principal {
        id: PrincipalId::new("https://idp.example.com", "alice"),
        issuer: "https://idp.example.com".to_owned(),
        subject: "alice".to_owned(),
        groups: BTreeSet::new(),
        attrs: Attrs::new(),
    };
    db.store.commit(Commit {
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
    }).await.unwrap();
    let mut scope = PrincipalScope::default();
    scope.actions.insert("renameGate".to_owned().try_into().unwrap());
    scope.types.insert("Gate".to_owned().try_into().unwrap(), TypeScope {
        row: Pred::Const(true),
        columns: ["code", "name"].into_iter().map(|name| (
            name.to_owned().try_into().unwrap(), Pred::Const(true),
        )).collect(),
    });
    Fixture { db, principal, scope, version, tokens: TokenCodec::new(b"idempotency-test-key".to_vec()).unwrap() }
}

fn rename(command: &str, name: Option<&str>) -> ActionRequest {
    let mut params = serde_json::Map::new();
    params.insert("gate".to_owned(), serde_json::json!("A1"));
    if let Some(name) = name { params.insert("name".to_owned(), serde_json::json!(name)); }
    ActionRequest {
        action: "renameGate".to_owned(), command_id: Some(command.to_owned()),
        items: vec![ActionItem { params, expected_versions: BTreeMap::new() }],
        validate_only: false,
    }
}

fn receipt(outcome: ActionOutcome) -> ontoforge_action::ActionReceipt {
    let ActionOutcome::Committed(receipt) = outcome else { panic!("expected a receipt") };
    receipt
}

#[tokio::test]
async fn expired_command_can_be_claimed_as_new_without_deleting_audit() {
    let f = fixture().await;
    let service = ActionService::new(&f.db.store, ActionLimits::new(8, 8, 65536).unwrap(), &f.tokens);
    let request = rename("expiring", Some("New"));
    let first = receipt(service.execute(&f.principal, &f.scope, f.version, request.clone()).await.unwrap());
    let pool = f.db.side_pool().await;
    sqlx::query("UPDATE of_action_command SET expires_at = '2000-01-01T00:00:00.000Z' WHERE principal_id = ? AND command_id = ?")
        .bind(f.principal.id.as_str()).bind("expiring").execute(&pool).await.unwrap();
    let second = receipt(service.execute(&f.principal, &f.scope, f.version, request).await.unwrap());
    assert_ne!(first.event_id, second.event_id, "an expired key must execute again");
    let logs: i64 = sqlx::query_scalar("SELECT count(*) FROM of_edit_log").fetch_one(&pool).await.unwrap();
    let outbox: i64 = sqlx::query_scalar("SELECT count(*) FROM of_webhook_outbox").fetch_one(&pool).await.unwrap();
    let commands: i64 = sqlx::query_scalar("SELECT count(*) FROM of_action_command").fetch_one(&pool).await.unwrap();
    assert_eq!((logs, outbox, commands), (3, 2, 1));
}
