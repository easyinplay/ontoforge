#![allow(
    clippy::unused_async_trait_impl,
    reason = "test engines return immediately"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use camino::{Utf8Path, Utf8PathBuf};
use ontoforge_auth::Principal;
use ontoforge_auth::authn::PrincipalId;
use ontoforge_auth::scope::{ScopeInputs, attr_types, compile_scope};
use ontoforge_engine::printer::{Dialect, PrintedSql};
use ontoforge_engine::sqlite::SqliteEngine;
use ontoforge_engine::sqlite::fixture::plan_fixture_tables;
use ontoforge_engine::{
    Engine, EngineError, EngineStatus, QueryContext, QueryOutput, TableRef, Value,
};
use ontoforge_query::{ExecutionLimits, ObjectSet, QueryRequest, QueryService, TokenCodec};
use ontoforge_schema::ir::{ObjectTypeName, ProfileName, PropertyName};
use ontoforge_schema::{load_dir, load_scope_dir};
use ontoforge_storage::commit::{Actor, Commit, Edit, ObjectOp};
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::store::{BackendSettings, EditStore, StoreSettings};
use ontoforge_storage::value::PropertyValue;
use serde_json::json;

const AVIATION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/ontologies/aviation"
);
const SCOPE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/scopes/aviation"
);

struct Fixture {
    dir: tempfile::TempDir,
    store: EditStore,
    principal: Principal,
    scope: ontoforge_auth::scope::PrincipalScope,
}

struct StaticEngine {
    rows: Vec<Vec<Value>>,
    calls: AtomicUsize,
    max_rows: AtomicUsize,
    sql: Mutex<String>,
}

impl StaticEngine {
    fn new(rows: Vec<Vec<Value>>) -> Self {
        Self {
            rows,
            calls: AtomicUsize::new(0),
            max_rows: AtomicUsize::new(0),
            sql: Mutex::new(String::new()),
        }
    }
}

impl Engine for StaticEngine {
    fn dialect(&self) -> Dialect {
        Dialect::Sqlite
    }
    async fn execute(
        &self,
        sql: &PrintedSql,
        ctx: &QueryContext,
    ) -> Result<QueryOutput, EngineError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.max_rows.store(ctx.max_rows, Ordering::SeqCst);
        let mut recorded = self.sql.lock().unwrap();
        recorded.clear();
        recorded.push_str(sql.as_str());
        Ok(QueryOutput {
            columns: Vec::new(),
            rows: self.rows.clone(),
            truncated: false,
        })
    }
    async fn describe_tables(
        &self,
        _: &[TableRef],
        _: &QueryContext,
    ) -> Result<BTreeMap<TableRef, Vec<ontoforge_engine::PhysicalColumn>>, EngineError> {
        panic!("pk read must not describe tables")
    }
    async fn status(&self) -> EngineStatus {
        EngineStatus::Available
    }
}

fn flight(status: &str) -> Vec<Value> {
    vec![
        Value::String("F-1001".to_owned()),
        Value::String("LH100".to_owned()),
        Value::String("FRA".to_owned()),
        Value::String("JFK".to_owned()),
        Value::String("D-AAAA".to_owned()),
        Value::Timestamp("2026-03-28T09:15:00".to_owned()),
        Value::String(status.to_owned()),
        Value::Long(0),
        Value::Array(Vec::new()),
    ]
}

async fn commit_edit(f: &Fixture, object_type: &str, pk: &str, op: ObjectOp) {
    let active = f.store.active_definition().await.unwrap();
    f.store
        .commit(Commit {
            actor: Actor::Principal(f.principal.id.to_string()),
            action: "testEdit".to_owned(),
            def_version: active.def_version,
            params: json!({}),
            edits: vec![Edit::Object {
                type_name: name(object_type),
                pk: PropertyValue::String(pk.to_owned()),
                op,
            }],
        })
        .await
        .unwrap();
}

fn service<'a, E: Engine>(
    f: &'a Fixture,
    active: Arc<ontoforge_storage::store::ActiveDefinition>,
    engine: &'a E,
    tokens: &'a TokenCodec,
) -> QueryService<'a, E> {
    QueryService {
        principal: &f.principal,
        scope: &f.scope,
        active,
        engine,
        store: &f.store,
        limits: ExecutionLimits::phase4_default(),
        tokens,
    }
}

fn name(raw: &str) -> ObjectTypeName {
    ObjectTypeName::try_from(raw.to_owned()).unwrap()
}
fn prop(raw: &str) -> PropertyName {
    PropertyName::try_from(raw.to_owned()).unwrap()
}
fn request(object_type: &str, pk: &str, select: &[&str]) -> QueryRequest {
    QueryRequest {
        set: ObjectSet::Pk {
            object_type: name(object_type),
            pk: json!(pk),
        },
        select: select.iter().map(|p| prop(p)).collect(),
    }
}

async fn fixture() -> Fixture {
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
    let loaded = load_dir(Utf8Path::new(AVIATION));
    assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
    let ontology = loaded.ontology.unwrap();
    reload(&store, &ontology, &ReloadOptions::default())
        .await
        .unwrap();
    let loaded_scope = load_scope_dir(Utf8Path::new(SCOPE));
    let cfg = serde_json::from_value(json!({
        "principal_attrs": { "homeAirports": { "claim": "airports", "type": "string_list" } }
    }))
    .unwrap();
    let compiled = compile_scope(&ScopeInputs {
        ontology: &ontology,
        profile: Some(&ProfileName::try_from("dev".to_owned()).unwrap()),
        attrs: &attr_types(Some(&cfg)),
        scope: &loaded_scope.scope.unwrap(),
        sources: Some(&loaded_scope.sources),
    })
    .unwrap();
    let principal = Principal {
        id: PrincipalId::new("ontoforge-dev", "alice"),
        issuer: "ontoforge-dev".to_owned(),
        subject: "alice".to_owned(),
        groups: BTreeSet::from(["dispatchers".to_owned()]),
        attrs: ontoforge_expr::Attrs::default(),
    };
    let scope = compiled.for_principal(&principal);
    Fixture {
        dir,
        store,
        principal,
        scope,
    }
}

struct NeverEngine;

impl Engine for NeverEngine {
    fn dialect(&self) -> Dialect {
        Dialect::Sqlite
    }
    async fn execute(&self, _: &PrintedSql, _: &QueryContext) -> Result<QueryOutput, EngineError> {
        panic!("edit-only read called source engine")
    }
    async fn describe_tables(
        &self,
        _: &[TableRef],
        _: &QueryContext,
    ) -> Result<BTreeMap<TableRef, Vec<ontoforge_engine::PhysicalColumn>>, EngineError> {
        panic!("edit-only read described source tables")
    }
    async fn status(&self) -> EngineStatus {
        EngineStatus::Available
    }
}

#[test]
fn execution_limits_phase4_defaults_are_exact() {
    let got = ExecutionLimits::phase4_default();
    assert_eq!(
        (
            got.max_backend_calls,
            got.max_raw_candidates,
            got.max_backfill_keys,
            got.max_decoded_bytes
        ),
        (32, 10_000, 2_048, 64 * 1024 * 1024)
    );
}

#[tokio::test]
async fn edited_pk_uses_store_only() {
    let f = fixture().await;
    let active = f.store.active_definition().await.unwrap();
    let mut values = BTreeMap::new();
    values.insert(
        prop("name"),
        PropertyValue::String("Berlin Airport".to_owned()),
    );
    f.store
        .commit(Commit {
            actor: Actor::Principal(f.principal.id.to_string()),
            action: "createAirport".to_owned(),
            def_version: active.def_version,
            params: json!({}),
            edits: vec![Edit::Object {
                type_name: name("Airport"),
                pk: PropertyValue::String("BER".to_owned()),
                op: ObjectOp::Set(values),
            }],
        })
        .await
        .unwrap();
    let engine = NeverEngine;
    let tokens = TokenCodec::new(b"phase4-test-key".to_vec()).unwrap();
    let service = QueryService {
        principal: &f.principal,
        scope: &f.scope,
        active,
        engine: &engine,
        store: &f.store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &tokens,
    };
    let page = service
        .execute(request("Airport", "BER", &["name"]))
        .await
        .unwrap();
    assert_eq!(page.rows.len(), 1);
    assert_eq!(page.rows[0].properties["name"], json!("Berlin Airport"));
    assert!(!page.rows[0].version.is_empty());
}

#[tokio::test]
async fn missing_read_has_no_side_effect() {
    let f = fixture().await;
    let active = f.store.active_definition().await.unwrap();
    let path = Utf8PathBuf::from_path_buf(f.dir.path().join("edit.db")).unwrap();
    let pool =
        sqlx::SqlitePool::connect_with(ontoforge_storage::store::sqlite_connect_options(&path))
            .await
            .unwrap();
    let table = &active.physical.objects["Airport"].table;
    let count = |table: &str| format!("SELECT count(*) FROM \"{table}\"");
    let before_rows: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(count(table)))
        .fetch_one(&pool)
        .await
        .unwrap();
    let before_log: i64 = sqlx::query_scalar("SELECT count(*) FROM of_edit_log")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(
        f.store
            .read_object_edit(
                &active,
                &name("Airport"),
                &PropertyValue::String("NONE".to_owned())
            )
            .await
            .unwrap()
            .is_none()
    );
    let after_rows: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(count(table)))
        .fetch_one(&pool)
        .await
        .unwrap();
    let after_log: i64 = sqlx::query_scalar("SELECT count(*) FROM of_edit_log")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        (
            after_rows,
            after_log,
            f.store.active_definition().await.unwrap().def_version
        ),
        (before_rows, before_log, active.def_version)
    );
}

#[tokio::test]
async fn unedited_pk_uses_single_source_query() {
    let f = fixture().await;
    let loaded = load_dir(Utf8Path::new(AVIATION));
    let ontology = loaded.ontology.unwrap();
    let plan = plan_fixture_tables(
        &ontology,
        &loaded.sources,
        &ProfileName::try_from("dev".to_owned()).unwrap(),
    )
    .unwrap();
    let engine = SqliteEngine::from_fixtures(Utf8Path::new(AVIATION), &plan.tables)
        .await
        .unwrap();
    let tokens = TokenCodec::new(b"phase4-test-key".to_vec()).unwrap();
    let service = QueryService {
        principal: &f.principal,
        scope: &f.scope,
        active: f.store.active_definition().await.unwrap(),
        engine: &engine,
        store: &f.store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &tokens,
    };
    let page = service
        .execute(request("Flight", "F-1001", &["flightNumber", "status"]))
        .await
        .unwrap();
    assert_eq!(page.rows[0].properties["flightNumber"], json!("LH100"));
}

#[tokio::test]
async fn source_pk_uses_limit_two_and_rejects_ambiguous_rows() {
    let f = fixture().await;
    let engine = StaticEngine::new(vec![flight("scheduled"), flight("scheduled")]);
    let tokens = TokenCodec::new(b"phase4-test-key".to_vec()).unwrap();
    let result = service(
        &f,
        f.store.active_definition().await.unwrap(),
        &engine,
        &tokens,
    )
    .execute(request("Flight", "F-1001", &["status"]))
    .await;
    assert!(matches!(
        result,
        Err(ontoforge_query::QueryError::AmbiguousObject)
    ));
    assert_eq!(engine.calls.load(Ordering::SeqCst), 1);
    let sql = engine.sql.lock().unwrap().clone();
    assert!(sql.contains("LIMIT 2"), "{sql}");
    assert!(sql.contains("FROM \"demo__aviation\".\"flights\""), "{sql}");
}

#[tokio::test]
async fn patch_null_and_unset_are_distinct() {
    let f = fixture().await;
    let mut patch = BTreeMap::new();
    patch.insert(prop("delayMinutes"), PropertyValue::Null);
    commit_edit(&f, "Flight", "F-1001", ObjectOp::Set(patch)).await;
    let engine = StaticEngine::new(vec![flight("scheduled")]);
    let tokens = TokenCodec::new(b"phase4-test-key".to_vec()).unwrap();
    let page = service(
        &f,
        f.store.active_definition().await.unwrap(),
        &engine,
        &tokens,
    )
    .execute(request("Flight", "F-1001", &["delayMinutes", "status"]))
    .await
    .unwrap();
    assert_eq!(page.rows[0].properties["delayMinutes"], json!(null));
    assert_eq!(page.rows[0].properties["status"], json!("scheduled"));
    commit_edit(
        &f,
        "Flight",
        "F-1001",
        ObjectOp::Unset(BTreeSet::from([prop("delayMinutes")])),
    )
    .await;
    let page = service(
        &f,
        f.store.active_definition().await.unwrap(),
        &engine,
        &tokens,
    )
    .execute(request("Flight", "F-1001", &["delayMinutes"]))
    .await
    .unwrap();
    assert_eq!(page.rows[0].properties["delayMinutes"], json!(0));
}

#[tokio::test]
async fn tombstone_is_not_found_and_release_reads_source() {
    let f = fixture().await;
    commit_edit(&f, "Flight", "F-1001", ObjectOp::Tombstone).await;
    let engine = StaticEngine::new(vec![flight("scheduled")]);
    let tokens = TokenCodec::new(b"phase4-test-key".to_vec()).unwrap();
    let hidden = service(
        &f,
        f.store.active_definition().await.unwrap(),
        &engine,
        &tokens,
    )
    .execute(request("Flight", "F-1001", &["status"]))
    .await;
    assert!(matches!(hidden, Err(ontoforge_query::QueryError::NotFound)));
    assert_eq!(engine.calls.load(Ordering::SeqCst), 0);
    commit_edit(&f, "Flight", "F-1001", ObjectOp::Release).await;
    let page = service(
        &f,
        f.store.active_definition().await.unwrap(),
        &engine,
        &tokens,
    )
    .execute(request("Flight", "F-1001", &["status"]))
    .await
    .unwrap();
    assert_eq!(page.rows[0].properties["status"], json!("scheduled"));
    assert_eq!(engine.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn scope_runs_after_overlay() {
    let f = fixture().await;
    let engine = StaticEngine::new(vec![flight("CANCELLED")]);
    let tokens = TokenCodec::new(b"phase4-test-key".to_vec()).unwrap();
    let hidden = service(
        &f,
        f.store.active_definition().await.unwrap(),
        &engine,
        &tokens,
    )
    .execute(request("Flight", "F-1001", &["status"]))
    .await;
    assert!(matches!(hidden, Err(ontoforge_query::QueryError::NotFound)));
    commit_edit(
        &f,
        "Flight",
        "F-1001",
        ObjectOp::Set(BTreeMap::from([(
            prop("status"),
            PropertyValue::String("scheduled".to_owned()),
        )])),
    )
    .await;
    let page = service(
        &f,
        f.store.active_definition().await.unwrap(),
        &engine,
        &tokens,
    )
    .execute(request(
        "Flight",
        "F-1001",
        &["status", "aircraftTailNumber"],
    ))
    .await
    .unwrap();
    assert_eq!(page.rows[0].properties["status"], json!("scheduled"));
    assert!(!page.rows[0].properties.contains_key("aircraftTailNumber"));
}

#[tokio::test]
async fn partial_edit_without_source_is_not_found() {
    let f = fixture().await;
    commit_edit(
        &f,
        "Flight",
        "F-1001",
        ObjectOp::Set(BTreeMap::from([(
            prop("status"),
            PropertyValue::String("scheduled".to_owned()),
        )])),
    )
    .await;
    let engine = StaticEngine::new(Vec::new());
    let tokens = TokenCodec::new(b"phase4-test-key".to_vec()).unwrap();
    let result = service(
        &f,
        f.store.active_definition().await.unwrap(),
        &engine,
        &tokens,
    )
    .execute(request("Flight", "F-1001", &["status"]))
    .await;
    assert!(matches!(result, Err(ontoforge_query::QueryError::NotFound)));
}

#[tokio::test]
async fn pk_read_obeys_host_raw_candidate_budget() {
    let f = fixture().await;
    commit_edit(
        &f,
        "Flight",
        "F-1001",
        ObjectOp::Set(BTreeMap::from([(
            prop("status"),
            PropertyValue::String("scheduled".to_owned()),
        )])),
    )
    .await;
    let engine = StaticEngine::new(vec![flight("scheduled")]);
    let tokens = TokenCodec::new(b"phase4-test-key".to_vec()).unwrap();
    let mut query = service(
        &f,
        f.store.active_definition().await.unwrap(),
        &engine,
        &tokens,
    );
    query.limits = ExecutionLimits::try_new(32, 1, 2_048, 64 * 1024 * 1024).unwrap();

    let page = query
        .execute(request("Flight", "F-1001", &["flightNumber"]))
        .await
        .unwrap();
    assert_eq!(page.rows.len(), 1);
    assert_eq!(engine.max_rows.load(Ordering::SeqCst), 1);
}

#[test]
fn execution_limits_reject_zero_and_overflow() {
    for i in 0..4 {
        let mut fields = [32, 10_000, 2_048, 64 * 1024 * 1024];
        fields[i] = 0;
        assert!(ExecutionLimits::try_new(fields[0], fields[1], fields[2], fields[3]).is_err());
    }
    assert!(ExecutionLimits::try_new(usize::MAX, 10_000, 2_048, 64 * 1024 * 1024).is_err());
    let bad = json!({"set": {"kind": "pk", "object_type": "Flight", "pk": "F-1001"},
        "select": ["status"], "maxBackendCalls": 999});
    assert!(serde_json::from_value::<QueryRequest>(bad).is_err());
}
