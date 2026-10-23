use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::Duration;

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
use tokio::sync::Barrier;
use tokio::time::{sleep, timeout};

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

struct Fixture {
    pg: support::PgTestStore,
    principal: Principal,
    scope: PrincipalScope,
    version: i64,
    tokens: TokenCodec,
}

async fn fixture() -> Fixture {
    let pg = support::pg_store().await;
    let loaded = load_from_memory(&[("ontology.yaml", "apiName: demo\n"), ("defs.yaml", DEFS)]);
    assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
    let version = reload(
        &pg.store,
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
    pg.store
        .commit(Commit {
            actor: Actor::Principal(principal.id.as_str().to_owned()),
            action: "seed".to_owned(),
            def_version: version,
            params: serde_json::Value::Null,
            edits: ["A1", "B1"]
                .into_iter()
                .map(|code| Edit::Object {
                    type_name: "Gate".to_owned().try_into().unwrap(),
                    pk: PropertyValue::String(code.to_owned()),
                    op: ObjectOp::Set(BTreeMap::from([(
                        "name".to_owned().try_into().unwrap(),
                        PropertyValue::String("Original".to_owned()),
                    )])),
                })
                .collect(),
        })
        .await
        .unwrap();
    let mut scope = PrincipalScope::default();
    scope
        .actions
        .insert("deleteGate".to_owned().try_into().unwrap());
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
    Fixture {
        pg,
        principal,
        scope,
        version,
        tokens: TokenCodec::new(b"idempotency-test-key".to_vec()).unwrap(),
    }
}

fn delete(command: &str, code: &str) -> ActionRequest {
    ActionRequest {
        action: "deleteGate".to_owned(),
        command_id: Some(command.to_owned()),
        items: vec![ActionItem {
            params: serde_json::from_value(serde_json::json!({"gate": code})).unwrap(),
            expected_versions: BTreeMap::new(),
        }],
        validate_only: false,
    }
}

async fn count(f: &Fixture, table: &str) -> i64 {
    let sql = format!("SELECT count(*) FROM \"{}\".{table}", f.pg.schema_sys);
    sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
        .fetch_one(f.pg.side())
        .await
        .unwrap()
}

async fn race(
    f: &Fixture,
    left: ActionRequest,
    right: ActionRequest,
) -> (
    Result<ActionOutcome, ontoforge_action::ActionError>,
    Result<ActionOutcome, ontoforge_action::ActionError>,
) {
    let other = f.pg.open_another(Duration::from_secs(10)).await;
    let gate = Arc::new(Barrier::new(2));
    let left_gate = Arc::clone(&gate);
    let first_service = ActionService::new(
        &f.pg.store,
        ActionLimits::new(8, 8, 65536).unwrap(),
        &f.tokens,
    );
    let second_service =
        ActionService::new(&other, ActionLimits::new(8, 8, 65536).unwrap(), &f.tokens);
    timeout(Duration::from_secs(30), async {
        tokio::join!(
            first_service.execute_with_test_gate(
                &f.principal,
                &f.scope,
                f.version,
                left,
                async move {
                    left_gate.wait().await;
                }
            ),
            second_service.execute_with_test_gate(
                &f.principal,
                &f.scope,
                f.version,
                right,
                async move {
                    gate.wait().await;
                }
            ),
        )
    })
    .await
    .expect("same-key contention must terminate")
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs ONTOFORGE_PG_URL or --features embedded-pg"]
async fn pg_response_lost_retry_reads_completed_receipt() {
    let f = fixture().await;
    let request = delete("lost-response", "A1");
    let service = ActionService::new(
        &f.pg.store,
        ActionLimits::new(8, 8, 65536).unwrap(),
        &f.tokens,
    );
    let ActionOutcome::Committed(first) = service
        .execute(&f.principal, &f.scope, f.version, request.clone())
        .await
        .unwrap()
    else {
        panic!("first execution must commit")
    };

    let retry = service
        .execute(&f.principal, &f.scope, f.version, request)
        .await;
    let ActionOutcome::Committed(second) =
        retry.expect("committed response must survive a lost response and deleted target")
    else {
        panic!("retry must replay the stored receipt")
    };
    assert_eq!(first, second);
    assert_eq!(f.pg.store.read_log(0, 10).await.unwrap().len(), 2);
    for table in ["of_action_command", "of_webhook_outbox"] {
        assert_eq!(count(&f, table).await, 1, "{table} must not grow on replay");
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs ONTOFORGE_PG_URL or --features embedded-pg"]
async fn pg_same_key_same_params_has_one_winner_and_one_replay() {
    let f = fixture().await;
    let (left, right) = race(&f, delete("same-key", "A1"), delete("same-key", "A1")).await;
    let (Ok(ActionOutcome::Committed(first)), Ok(ActionOutcome::Committed(second))) = (left, right)
    else {
        panic!("same request must replay for loser")
    };
    assert_eq!(first, second);
    assert_eq!(f.pg.store.read_log(0, 10).await.unwrap().len(), 2);
    assert_eq!(count(&f, "of_action_command").await, 1);
    assert_eq!(count(&f, "of_webhook_outbox").await, 1);
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs ONTOFORGE_PG_URL or --features embedded-pg"]
async fn pg_same_key_different_params_has_one_winner_and_one_reused() {
    let f = fixture().await;
    let (left, right) = race(&f, delete("reused-key", "A1"), delete("reused-key", "B1")).await;
    let results = [&left, &right];
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(result, Ok(ActionOutcome::Committed(_))))
            .count(),
        1,
        "{results:?}"
    );
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(
                result,
                Err(ontoforge_action::ActionError::Commit(
                    ontoforge_storage::commit::ActionCommitError::CommandIdReused
                ))
            ))
            .count(),
        1,
        "{results:?}"
    );
    assert_eq!(f.pg.store.read_log(0, 10).await.unwrap().len(), 2);
    assert_eq!(count(&f, "of_action_command").await, 1);
    assert_eq!(count(&f, "of_webhook_outbox").await, 1);
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs ONTOFORGE_PG_URL or --features embedded-pg"]
async fn pg_winner_abort_allows_loser_to_claim() {
    let f = fixture().await;
    let mut aborted = f.pg.side().begin().await.unwrap();
    let sql = format!(
        "INSERT INTO \"{}\".of_action_command \
        (principal_id, command_id, action, request_hash, action_def_hash, def_version, chosen_current_time, expires_at) \
        VALUES ($1, $2, 'deleteGate', 'pending', 'pending', $3, now(), now() + interval '7 days')",
        f.pg.schema_sys
    );
    sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(f.principal.id.as_str())
        .bind("aborted-key")
        .bind(f.version)
        .execute(&mut *aborted)
        .await
        .unwrap();
    let service = ActionService::new(
        &f.pg.store,
        ActionLimits::new(8, 8, 65536).unwrap(),
        &f.tokens,
    );
    let ((), result) = timeout(Duration::from_secs(30), async {
        tokio::join!(
            async {
                sleep(Duration::from_millis(150)).await;
                aborted.rollback().await.unwrap();
            },
            service.execute(
                &f.principal,
                &f.scope,
                f.version,
                delete("aborted-key", "A1")
            ),
        )
    })
    .await
    .expect("loser must progress after winner rollback");
    assert!(
        matches!(result, Ok(ActionOutcome::Committed(_))),
        "{result:?}"
    );
    assert_eq!(f.pg.store.read_log(0, 10).await.unwrap().len(), 2);
    assert_eq!(count(&f, "of_action_command").await, 1);
    assert_eq!(count(&f, "of_webhook_outbox").await, 1);
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs ONTOFORGE_PG_URL or --features embedded-pg"]
async fn pg_same_command_id_for_different_principals_is_independent() {
    let f = fixture().await;
    let service = ActionService::new(
        &f.pg.store,
        ActionLimits::new(8, 8, 65536).unwrap(),
        &f.tokens,
    );
    let alice = service
        .execute(
            &f.principal,
            &f.scope,
            f.version,
            delete("shared-key", "A1"),
        )
        .await
        .unwrap();
    let mut bob = f.principal.clone();
    bob.id = PrincipalId::new("https://idp.example.com", "bob");
    bob.subject = "bob".to_owned();
    let bob_result = service
        .execute(&bob, &f.scope, f.version, delete("shared-key", "B1"))
        .await
        .unwrap();
    assert!(matches!(
        (alice, bob_result),
        (ActionOutcome::Committed(_), ActionOutcome::Committed(_))
    ));
    assert_eq!(f.pg.store.read_log(0, 10).await.unwrap().len(), 3);
    assert_eq!(count(&f, "of_action_command").await, 2);
    assert_eq!(count(&f, "of_webhook_outbox").await, 2);
}
