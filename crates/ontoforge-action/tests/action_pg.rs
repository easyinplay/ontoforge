#![cfg(feature = "test-support")]
#![allow(
    clippy::unused_async_trait_impl,
    reason = "editOnly token query has no source calls"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::Duration;

use ontoforge_action::{
    ActionError, ActionItem, ActionLimits, ActionOutcome, ActionRequest, ActionService,
};
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
use ontoforge_storage::value::PropertyValue;
use tokio::sync::Barrier;
use tokio::time::timeout;

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
  name: { type: string, required: true }
rules:
  - kind: modifyObject
    object: gate
    properties:
      name: { param: name }
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

struct Fixture {
    pg: support::PgTestStore,
    version: i64,
    principal: Principal,
    scope: PrincipalScope,
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
    pg.store
        .commit(Commit {
            actor: Actor::Principal("https://idp.example.com|alice".to_owned()),
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
    let principal = Principal {
        id: PrincipalId::new("https://idp.example.com", "alice"),
        issuer: "https://idp.example.com".to_owned(),
        subject: "alice".to_owned(),
        groups: BTreeSet::new(),
        attrs: Attrs::new(),
    };
    let mut scope = PrincipalScope::default();
    for action in ["renameGate", "createGate"] {
        scope.actions.insert(action.to_owned().try_into().unwrap());
    }
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
        version,
        principal,
        scope,
        tokens: TokenCodec::new(b"pg-race-key".to_vec()).unwrap(),
    }
}

fn rename(command: &str, codes: &[&str], token: Option<&str>) -> ActionRequest {
    ActionRequest {
        action: "renameGate".to_owned(),
        command_id: Some(command.to_owned()),
        items: codes
            .iter()
            .map(|code| ActionItem {
                params: serde_json::from_value(serde_json::json!({"gate": code, "name": command}))
                    .unwrap(),
                expected_versions: token
                    .map(|value| BTreeMap::from([("gate".to_owned(), value.to_owned())]))
                    .unwrap_or_default(),
            })
            .collect(),
        validate_only: false,
    }
}

fn create(command: &str, code: &str) -> ActionRequest {
    ActionRequest {
        action: "createGate".to_owned(),
        command_id: Some(command.to_owned()),
        items: vec![ActionItem {
            params: serde_json::from_value(serde_json::json!({"code": code, "name": command}))
                .unwrap(),
            expected_versions: BTreeMap::new(),
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

async fn query_token(f: &Fixture) -> String {
    QueryService {
        principal: &f.principal,
        scope: &f.scope,
        active: f.pg.store.active_definition().await.unwrap(),
        engine: &NeverEngine,
        store: &f.pg.store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &f.tokens,
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

async fn race(
    f: &Fixture,
    first: ActionRequest,
    second: ActionRequest,
) -> (
    Result<ActionOutcome, ActionError>,
    Result<ActionOutcome, ActionError>,
) {
    let other = f.pg.open_another(Duration::from_secs(10)).await;
    let gate = Arc::new(Barrier::new(2));
    let left_gate = Arc::clone(&gate);
    let left = async {
        ActionService::new(
            &f.pg.store,
            ActionLimits::new(8, 8, 65536).unwrap(),
            &f.tokens,
        )
        .execute_with_test_gate(&f.principal, &f.scope, f.version, first, async move {
            left_gate.wait().await;
        })
        .await
    };
    let right = async {
        ActionService::new(&other, ActionLimits::new(8, 8, 65536).unwrap(), &f.tokens)
            .execute_with_test_gate(&f.principal, &f.scope, f.version, second, async move {
                gate.wait().await;
            })
            .await
    };
    timeout(Duration::from_secs(30), async {
        tokio::join!(Box::pin(left), Box::pin(right))
    })
    .await
    .expect("concurrent Action requests must not deadlock")
}

async fn count(f: &Fixture, table: &str) -> i64 {
    let sql = format!("SELECT count(*) FROM \"{}\".{table}", f.pg.schema_sys);
    sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
        .fetch_one(f.pg.side())
        .await
        .unwrap()
}

async fn assert_single_winner(
    f: &Fixture,
    results: (
        Result<ActionOutcome, ActionError>,
        Result<ActionOutcome, ActionError>,
    ),
    rows: &[&str],
) {
    let (left, right) = results;
    let wins = [&left, &right]
        .into_iter()
        .filter(|outcome| matches!(outcome, Ok(ActionOutcome::Committed(_))))
        .count();
    let conflicts = [&left, &right]
        .into_iter()
        .filter(|outcome| matches!(outcome, Err(ActionError::VersionConflict)))
        .count();
    assert_eq!((wins, conflicts), (1, 1), "left={left:?}, right={right:?}");
    for code in rows {
        let active = f.pg.store.active_definition().await.unwrap();
        let row =
            f.pg.store
                .read_object_edit(
                    &active,
                    &"Gate".to_owned().try_into().unwrap(),
                    &PropertyValue::String((*code).to_owned()),
                )
                .await
                .unwrap()
                .unwrap();
        assert_eq!(row.row.version, if *code == "C1" { 1 } else { 2 }, "{code}");
    }
    assert_eq!(f.pg.store.read_log(0, 10).await.unwrap().len(), 2);
    assert_eq!(count(f, "of_action_command").await, 1);
    assert_eq!(count(f, "of_webhook_outbox").await, 1);
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs ONTOFORGE_PG_URL or --features embedded-pg"]
async fn concurrent_same_pk_with_token_has_one_winner() {
    let f = fixture().await;
    let token = query_token(&f).await;
    let results = Box::pin(race(
        &f,
        rename("token-left", &["A1"], Some(&token)),
        rename("token-right", &["A1"], Some(&token)),
    ))
    .await;
    assert_single_winner(&f, results, &["A1"]).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs ONTOFORGE_PG_URL or --features embedded-pg"]
async fn concurrent_same_pk_without_token_has_one_winner() {
    let f = fixture().await;
    let results = Box::pin(race(
        &f,
        rename("plain-left", &["A1"], None),
        rename("plain-right", &["A1"], None),
    ))
    .await;
    assert_single_winner(&f, results, &["A1"]).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs ONTOFORGE_PG_URL or --features embedded-pg"]
async fn concurrent_multirow_reverse_request_order_does_not_deadlock() {
    let f = fixture().await;
    let results = Box::pin(race(
        &f,
        rename("multi-left", &["A1", "B1"], None),
        rename("multi-right", &["B1", "A1"], None),
    ))
    .await;
    assert_single_winner(&f, results, &["A1", "B1"]).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs ONTOFORGE_PG_URL or --features embedded-pg"]
async fn concurrent_absent_edit_create_is_serialized() {
    let f = fixture().await;
    let active = f.pg.store.active_definition().await.unwrap();
    let absent =
        f.pg.store
            .read_object_edit(
                &active,
                &"Gate".to_owned().try_into().unwrap(),
                &PropertyValue::String("C1".to_owned()),
            )
            .await
            .unwrap();
    assert!(
        absent.is_none(),
        "both requests must prepare from an absent target"
    );
    let results = Box::pin(race(
        &f,
        create("insert-left", "C1"),
        create("insert-right", "C1"),
    ))
    .await;
    assert_single_winner(&f, results, &["C1"]).await;
}
