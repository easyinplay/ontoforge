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
apiName: deleteGate
parameters:
  gate: { type: { objectReference: Gate }, required: true }
rules:
  - kind: deleteObject
    object: gate
";

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs ONTOFORGE_PG_URL or --features embedded-pg"]
async fn pg_response_lost_retry_reads_completed_receipt() {
    let pg = support::pg_store().await;
    let loaded = load_from_memory(&[("ontology.yaml", "apiName: demo\n"), ("defs.yaml", DEFS)]);
    assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
    let version = reload(&pg.store, &loaded.ontology.unwrap(), &ReloadOptions::default())
        .await.unwrap().def_version.unwrap();
    let principal = Principal {
        id: PrincipalId::new("https://idp.example.com", "alice"),
        issuer: "https://idp.example.com".to_owned(),
        subject: "alice".to_owned(),
        groups: BTreeSet::new(),
        attrs: Attrs::new(),
    };
    pg.store.commit(Commit {
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
    scope.actions.insert("deleteGate".to_owned().try_into().unwrap());
    scope.types.insert("Gate".to_owned().try_into().unwrap(), TypeScope {
        row: Pred::Const(true),
        columns: ["code", "name"].into_iter().map(|name| (
            name.to_owned().try_into().unwrap(), Pred::Const(true),
        )).collect(),
    });
    let request = ActionRequest {
        action: "deleteGate".to_owned(),
        command_id: Some("lost-response".to_owned()),
        items: vec![ActionItem {
            params: serde_json::from_value(serde_json::json!({"gate": "A1"})).unwrap(),
            expected_versions: BTreeMap::new(),
        }],
        validate_only: false,
    };
    let tokens = TokenCodec::new(b"idempotency-test-key".to_vec()).unwrap();
    let service = ActionService::new(&pg.store, ActionLimits::new(8, 8, 65536).unwrap(), &tokens);
    let ActionOutcome::Committed(first) = service.execute(&principal, &scope, version, request.clone())
        .await.unwrap() else { panic!("first execution must commit") };

    let retry = service.execute(&principal, &scope, version, request).await;
    let ActionOutcome::Committed(second) = retry.expect("committed response must survive a lost response and deleted target") else {
        panic!("retry must replay the stored receipt")
    };
    assert_eq!(first, second);
    assert_eq!(pg.store.read_log(0, 10).await.unwrap().len(), 2);
    for table in ["of_action_command", "of_webhook_outbox"] {
        let sql = format!("SELECT count(*) FROM \"{}\".{table}", pg.schema_sys);
        let rows: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
            .fetch_one(pg.side()).await.unwrap();
        assert_eq!(rows, 1, "{table} must not grow on replay");
    }
}
