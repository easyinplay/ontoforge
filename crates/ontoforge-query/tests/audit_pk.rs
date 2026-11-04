#![allow(
    clippy::unused_async_trait_impl,
    reason = "test engines return immediately or wait for cancellation"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use camino::{Utf8Path, Utf8PathBuf};
use ontoforge_auth::Principal;
use ontoforge_auth::authn::PrincipalId;
use ontoforge_auth::scope::{ScopeInputs, attr_types, compile_scope};
use ontoforge_engine::printer::{Dialect, PrintedSql};
use ontoforge_engine::{Engine, EngineError, EngineStatus, QueryContext, QueryOutput, TableRef};
use ontoforge_expr::Attrs;
use ontoforge_query::aggregate::{AggregateCapabilities, AggregateLimits};
use ontoforge_query::ast::{AggregateMeasure, AggregateRequest, PageSpec};
use ontoforge_query::compile::{CompileLimits, ValidatedQuery};
use ontoforge_query::{
    ExecutionLimits, ObjectSet, QueryError, QueryRequest, QueryService, TokenCodec,
};
use ontoforge_schema::ir::{ObjectTypeName, ProfileName, PropertyName};
use ontoforge_schema::{load_dir, load_scope_dir};
use ontoforge_storage::commit::{Actor, Commit, Edit, ObjectOp};
use ontoforge_storage::read_audit::ReadAuditSettings;
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::store::{BackendSettings, EditStore, StoreSettings};
use ontoforge_storage::value::PropertyValue;
use serde_json::json;
use sqlx::Row;
use tokio::sync::Notify;

const AVIATION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/ontologies/aviation"
);
const SCOPE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/scopes/aviation"
);

fn name(value: &str) -> ObjectTypeName {
    ObjectTypeName::try_from(value.to_owned()).unwrap()
}

fn prop(value: &str) -> PropertyName {
    PropertyName::try_from(value.to_owned()).unwrap()
}

struct NeverEngine;

impl Engine for NeverEngine {
    fn dialect(&self) -> Dialect {
        Dialect::Sqlite
    }
    async fn execute(&self, _: &PrintedSql, _: &QueryContext) -> Result<QueryOutput, EngineError> {
        panic!("edit-only pk read must not call the source engine")
    }
    async fn describe_tables(
        &self,
        _: &[TableRef],
        _: &QueryContext,
    ) -> Result<BTreeMap<TableRef, Vec<ontoforge_engine::PhysicalColumn>>, EngineError> {
        panic!("pk read must not describe source tables")
    }
    async fn status(&self) -> EngineStatus {
        EngineStatus::Available
    }
}

struct ErrorEngine(AtomicUsize);

impl Engine for ErrorEngine {
    fn dialect(&self) -> Dialect {
        Dialect::Sqlite
    }
    async fn execute(&self, _: &PrintedSql, _: &QueryContext) -> Result<QueryOutput, EngineError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err(EngineError::Query {
            error_name: "hidden_sql_secret".to_owned(),
            message: "hidden_result_secret".to_owned(),
        })
    }
    async fn describe_tables(
        &self,
        _: &[TableRef],
        _: &QueryContext,
    ) -> Result<BTreeMap<TableRef, Vec<ontoforge_engine::PhysicalColumn>>, EngineError> {
        panic!("pk read must not describe source tables")
    }
    async fn status(&self) -> EngineStatus {
        EngineStatus::Available
    }
}

struct BlockingEngine {
    started: Notify,
}

impl Engine for BlockingEngine {
    fn dialect(&self) -> Dialect {
        Dialect::Sqlite
    }
    async fn execute(&self, _: &PrintedSql, _: &QueryContext) -> Result<QueryOutput, EngineError> {
        self.started.notify_one();
        std::future::pending().await
    }
    async fn describe_tables(
        &self,
        _: &[TableRef],
        _: &QueryContext,
    ) -> Result<BTreeMap<TableRef, Vec<ontoforge_engine::PhysicalColumn>>, EngineError> {
        panic!("pk read must not describe source tables")
    }
    async fn status(&self) -> EngineStatus {
        EngineStatus::Available
    }
}

struct Fixture {
    _dir: tempfile::TempDir,
    path: Utf8PathBuf,
    store: EditStore,
    active: Arc<ontoforge_storage::store::ActiveDefinition>,
    principal: Principal,
    scope: ontoforge_auth::scope::PrincipalScope,
}

async fn fixture(audit_settings: Option<ReadAuditSettings>) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let path = Utf8PathBuf::from_path_buf(dir.path().join("edit.db")).unwrap();
    let settings = StoreSettings {
        backend: BackendSettings::Sqlite { path: path.clone() },
        lock_timeout: Duration::from_secs(5),
        probe_timeout: Duration::from_secs(5),
        trino_catalog: None,
    };
    let store = EditStore::open_with_read_audit(settings, audit_settings.unwrap_or_default())
        .await
        .unwrap();
    let ontology = load_dir(Utf8Path::new(AVIATION)).ontology.unwrap();
    reload(&store, &ontology, &ReloadOptions::default())
        .await
        .unwrap();
    let scope_dir = load_scope_dir(Utf8Path::new(SCOPE));
    let attrs = serde_json::from_value(json!({
        "principal_attrs": {"homeAirports": {"claim": "airports", "type": "string_list"}}
    }))
    .unwrap();
    let compiled = compile_scope(&ScopeInputs {
        ontology: &ontology,
        profile: Some(&ProfileName::try_from("dev".to_owned()).unwrap()),
        attrs: &attr_types(Some(&attrs)),
        scope: &scope_dir.scope.unwrap(),
        sources: Some(&scope_dir.sources),
    })
    .unwrap();
    let principal = Principal {
        id: PrincipalId::new("ontoforge-dev", "alice"),
        issuer: "ontoforge-dev".to_owned(),
        subject: "alice".to_owned(),
        groups: BTreeSet::from(["dispatchers".to_owned()]),
        attrs: Attrs::default(),
    };
    let scope = compiled.for_principal(&principal);
    let active = store.active_definition().await.unwrap();
    store
        .commit(Commit {
            actor: Actor::Principal(principal.id.to_string()),
            action: "createAirport".to_owned(),
            def_version: active.def_version,
            params: json!({}),
            edits: vec![Edit::Object {
                type_name: name("Airport"),
                pk: PropertyValue::String("BER".to_owned()),
                op: ObjectOp::Set(BTreeMap::from([(
                    prop("name"),
                    PropertyValue::String("Berlin".to_owned()),
                )])),
            }],
        })
        .await
        .unwrap();
    Fixture {
        _dir: dir,
        path,
        store,
        active,
        principal,
        scope,
    }
}

fn request(object_type: &str, pk: &str, selected: &str) -> QueryRequest {
    QueryRequest {
        set: ObjectSet::Pk {
            object_type: name(object_type),
            pk: json!(pk),
        },
        select: vec![prop(selected)],
    }
}

#[tokio::test]
async fn audit_observed_wrappers_report_single_reservation_and_never_report_failed_reserve() {
    let f = fixture(None).await;
    let tokens = TokenCodec::new(b"audit-test-key".to_vec()).unwrap();
    let service = QueryService {
        principal: &f.principal,
        scope: &f.scope,
        active: Arc::clone(&f.active),
        engine: &NeverEngine,
        store: &f.store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &tokens,
    };
    let query = ValidatedQuery::new(
        QueryRequest {
            set: ObjectSet::Type {
                object_type: name("Airport"),
            },
            select: vec![prop("name")],
        },
        vec![],
        PageSpec {
            first: 10,
            after: None,
        },
        &f.active.ontology,
        &f.scope.types[&name("Airport")],
        CompileLimits::default(),
    )
    .unwrap();
    let aggregate = || AggregateRequest {
        set: ObjectSet::Type {
            object_type: name("Airport"),
        },
        group_by: vec![],
        measures: vec![AggregateMeasure::Count { property: None }],
    };
    let calls = AtomicUsize::new(0);
    let observed = std::sync::Mutex::new(Vec::new());
    let observe = |id: &str| {
        calls.fetch_add(1, Ordering::SeqCst);
        observed.lock().unwrap().push(id.to_owned());
    };
    let pk = service
        .execute_audited_observed(request("Airport", "BER", "name"), observe)
        .await;
    let list = service.execute_list_audited_observed(&query, observe).await;
    let agg = service
        .execute_aggregate_audited_observed(
            aggregate(),
            AggregateCapabilities::all_supported(),
            AggregateLimits::phase4_default(),
            observe,
        )
        .await;
    assert_eq!(pk.result.unwrap().rows.len(), 1);
    assert_eq!(list.result.unwrap().rows.len(), 1);
    assert_eq!(
        agg.result.unwrap().buckets[0].values,
        json!([1]).as_array().unwrap().clone()
    );
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    assert_eq!(
        *observed.lock().unwrap(),
        vec![
            pk.request_id.unwrap(),
            list.request_id.unwrap(),
            agg.request_id.unwrap()
        ]
    );
    f.store.read_audit().shutdown().await.unwrap();
    let rows = audit_rows(&f.path, 3).await;
    assert_eq!(rows.len(), 3, "each wrapper must reserve only once");
    assert!(
        rows.iter()
            .all(|r| r.get::<String, _>("status") == "success")
    );
    let pk = service
        .execute_audited_observed(request("Airport", "BER", "name"), observe)
        .await;
    let list = service.execute_list_audited_observed(&query, observe).await;
    let agg = service
        .execute_aggregate_audited_observed(
            aggregate(),
            AggregateCapabilities::all_supported(),
            AggregateLimits::phase4_default(),
            observe,
        )
        .await;
    assert!(matches!(pk.result, Err(QueryError::AuditUnavailable)));
    assert!(matches!(list.result, Err(QueryError::AuditUnavailable)));
    assert!(matches!(
        agg.result,
        Err(ontoforge_query::aggregate::AggregateError::AuditUnavailable)
    ));
    assert!(pk.request_id.is_none() && list.request_id.is_none() && agg.request_id.is_none());
    assert_eq!(
        calls.load(Ordering::SeqCst),
        3,
        "failed reservation cannot call observer"
    );
}

async fn audit_rows(path: &Utf8PathBuf, count: usize) -> Vec<sqlx::sqlite::SqliteRow> {
    let pool =
        sqlx::SqlitePool::connect_with(ontoforge_storage::store::sqlite_connect_options(path))
            .await
            .unwrap();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    loop {
        let rows = sqlx::query("SELECT * FROM of_read_audit ORDER BY seq")
            .fetch_all(&pool)
            .await
            .unwrap();
        if rows.len() >= count {
            return rows;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "audit rows not durable: expected {count}, got {}",
            rows.len()
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[tokio::test]
async fn audit_pk_success_records_one_metadata_row() {
    let f = fixture(None).await;
    let tokens = TokenCodec::new(b"audit-test-key".to_vec()).unwrap();
    let page = QueryService {
        principal: &f.principal,
        scope: &f.scope,
        active: Arc::clone(&f.active),
        engine: &NeverEngine,
        store: &f.store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &tokens,
    }
    .execute(request("Airport", "BER", "name"))
    .await
    .unwrap();
    assert_eq!(page.rows.len(), 1);

    let pool =
        sqlx::SqlitePool::connect_with(ontoforge_storage::store::sqlite_connect_options(&f.path))
            .await
            .unwrap();
    let table_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'of_read_audit'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(table_count, 1, "pk read must have a durable audit table");
    let rows = audit_rows(&f.path, 1).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].get::<String, _>("principal"),
        f.principal.id.as_str()
    );
    assert_eq!(rows[0].get::<String, _>("tool"), "query.pk");
    assert_eq!(rows[0].get::<String, _>("object_type"), "Airport");
    assert_eq!(rows[0].get::<i64, _>("row_count"), 1);
    assert_eq!(rows[0].get::<String, _>("status"), "success");
    assert_eq!(rows[0].get::<String, _>("sql_sha256"), "no-sql");
    assert_eq!(rows[0].get::<i64, _>("def_version"), f.active.def_version);
    assert!(!rows[0].get::<String, _>("request_id").is_empty());
    assert_eq!(
        rows[0].get::<String, _>("condition_api_names"),
        r#"["code"]"#
    );
    assert_ne!(rows[0].get::<String, _>("effective_roles"), "[]");
    assert!(!format!("{rows:?}").contains("Berlin"));
}

#[tokio::test]
async fn audit_pk_execution_returns_durable_id_for_success_and_denial() {
    let f = fixture(None).await;
    let tokens = TokenCodec::new(b"audit-test-key".to_vec()).unwrap();
    let service = QueryService {
        principal: &f.principal,
        scope: &f.scope,
        active: Arc::clone(&f.active),
        engine: &NeverEngine,
        store: &f.store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &tokens,
    };
    let success = service
        .execute_audited(request("Airport", "BER", "name"))
        .await;
    assert!(success.result.is_ok());
    assert!(
        success.request_id.is_some(),
        "successful query must return its audit guard ID"
    );
    let denied = service
        .execute_audited(request("Airport", "MISSING", "name"))
        .await;
    assert!(matches!(denied.result, Err(QueryError::NotFound)));
    assert!(
        denied.request_id.is_some(),
        "denied query must retain its reserved audit ID"
    );
    f.store.read_audit().shutdown().await.unwrap();
    let rows = audit_rows(&f.path, 2).await;
    for (execution, status) in [(success, "success"), (denied, "denied")] {
        let id = execution.request_id.unwrap();
        let row = rows
            .iter()
            .find(|row| row.get::<String, _>("request_id") == id)
            .unwrap();
        assert_eq!(row.get::<String, _>("status"), status);
    }
}

#[tokio::test]
async fn audit_pk_execution_reserve_failure_has_no_audit_id() {
    let f = fixture(None).await;
    f.store.read_audit().shutdown().await.unwrap();
    let tokens = TokenCodec::new(b"audit-test-key".to_vec()).unwrap();
    let execution = QueryService {
        principal: &f.principal,
        scope: &f.scope,
        active: Arc::clone(&f.active),
        engine: &NeverEngine,
        store: &f.store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &tokens,
    }
    .execute_audited(request("Airport", "BER", "name"))
    .await;
    assert!(matches!(
        execution.result,
        Err(QueryError::AuditUnavailable)
    ));
    assert!(execution.request_id.is_none());
}

struct ReleasedErrorEngine {
    entered: Notify,
    release: Notify,
}

impl Engine for ReleasedErrorEngine {
    fn dialect(&self) -> Dialect {
        Dialect::Sqlite
    }
    async fn execute(&self, _: &PrintedSql, _: &QueryContext) -> Result<QueryOutput, EngineError> {
        self.entered.notify_one();
        self.release.notified().await;
        Err(EngineError::Config("source failed".to_owned()))
    }
    async fn describe_tables(
        &self,
        _: &[TableRef],
        _: &QueryContext,
    ) -> Result<BTreeMap<TableRef, Vec<ontoforge_engine::PhysicalColumn>>, EngineError> {
        Ok(BTreeMap::new())
    }
    async fn status(&self) -> EngineStatus {
        EngineStatus::Available
    }
}

#[tokio::test]
async fn audit_pk_execution_finish_failure_retains_reserved_id() {
    let f = Arc::new(
        fixture(Some(ReadAuditSettings {
            capacity: 4,
            batch_size: 1,
            flush_interval: Duration::from_millis(10),
            ..ReadAuditSettings::default()
        }))
        .await,
    );
    let engine = Arc::new(ReleasedErrorEngine {
        entered: Notify::new(),
        release: Notify::new(),
    });
    let task = {
        let f = Arc::clone(&f);
        let engine = Arc::clone(&engine);
        tokio::spawn(async move {
            let tokens = TokenCodec::new(b"audit-test-key".to_vec()).unwrap();
            QueryService {
                principal: &f.principal,
                scope: &f.scope,
                active: Arc::clone(&f.active),
                engine: &*engine,
                store: &f.store,
                limits: ExecutionLimits::phase4_default(),
                tokens: &tokens,
            }
            .execute_audited(request("Flight", "F-1001", "status"))
            .await
        })
    };
    tokio::time::timeout(Duration::from_secs(3), engine.entered.notified())
        .await
        .unwrap();
    let pool =
        sqlx::SqlitePool::connect_with(ontoforge_storage::store::sqlite_connect_options(&f.path))
            .await
            .unwrap();
    sqlx::query("DROP TABLE of_read_audit")
        .execute(&pool)
        .await
        .unwrap();
    let tokens = TokenCodec::new(b"audit-test-key".to_vec()).unwrap();
    QueryService {
        principal: &f.principal,
        scope: &f.scope,
        active: Arc::clone(&f.active),
        engine: &NeverEngine,
        store: &f.store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &tokens,
    }
    .execute(request("Airport", "BER", "name"))
    .await
    .unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        while f.store.read_audit().health().available {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    engine.release.notify_one();
    let execution = task.await.unwrap();
    assert!(matches!(
        execution.result,
        Err(QueryError::AuditUnavailable)
    ));
    assert!(
        execution.request_id.is_some(),
        "finish failure must retain the audit guard ID"
    );
    assert!(f.store.read_audit().shutdown().await.is_err());
    pool.close().await;
}

#[tokio::test]
async fn audit_pk_denied_before_source_and_source_failure_have_distinct_statuses() {
    let f = fixture(None).await;
    let tokens = TokenCodec::new(b"audit-test-key".to_vec()).unwrap();
    let mut denied_scope = f.scope.clone();
    denied_scope.types.remove(&name("Flight"));
    let denied = QueryService {
        principal: &f.principal,
        scope: &denied_scope,
        active: Arc::clone(&f.active),
        engine: &NeverEngine,
        store: &f.store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &tokens,
    }
    .execute(request("Flight", "F-1001", "status"))
    .await
    .unwrap_err();
    assert!(matches!(denied, QueryError::NotFound));
    let engine = ErrorEngine(AtomicUsize::new(0));
    let source_error = QueryService {
        principal: &f.principal,
        scope: &f.scope,
        active: Arc::clone(&f.active),
        engine: &engine,
        store: &f.store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &tokens,
    }
    .execute(request("Flight", "F-1001", "status"))
    .await
    .unwrap_err();
    assert!(matches!(source_error, QueryError::Source));
    assert_eq!(engine.0.load(Ordering::SeqCst), 1);
    let rows = audit_rows(&f.path, 2).await;
    assert_eq!(
        rows.iter()
            .map(|r| r.get::<String, _>("status"))
            .collect::<Vec<_>>(),
        ["denied", "source_error"]
    );
    assert_eq!(rows[0].get::<String, _>("sql_sha256"), "no-sql");
    let hash = rows[1].get::<String, _>("sql_sha256");
    assert_eq!(hash.len(), 64, "source SQL must be hashed before execution");
    for row in &rows {
        assert_eq!(row.get::<i64, _>("row_count"), 0);
        assert_eq!(row.get::<i64, _>("def_version"), f.active.def_version);
    }
    let db_text = std::fs::read(&f.path).unwrap();
    let db_text = String::from_utf8_lossy(&db_text);
    for secret in [
        "hidden_sql_secret",
        "hidden_result_secret",
        "SELECT ",
        "F-1001",
    ] {
        assert!(!db_text.contains(secret), "audit store leaked {secret}");
    }
}

#[tokio::test]
async fn audit_pk_cancelled_in_flight_records_without_result() {
    let f = Arc::new(
        fixture(Some(ReadAuditSettings {
            capacity: 1,
            batch_size: 1,
            ..ReadAuditSettings::default()
        }))
        .await,
    );
    let engine = Arc::new(BlockingEngine {
        started: Notify::new(),
    });
    let task = {
        let f = Arc::clone(&f);
        let engine = Arc::clone(&engine);
        tokio::spawn(async move {
            let tokens = TokenCodec::new(b"audit-test-key".to_vec()).unwrap();
            QueryService {
                principal: &f.principal,
                scope: &f.scope,
                active: Arc::clone(&f.active),
                engine: &*engine,
                store: &f.store,
                limits: ExecutionLimits::phase4_default(),
                tokens: &tokens,
            }
            .execute(request("Flight", "F-1001", "status"))
            .await
        })
    };
    tokio::time::timeout(Duration::from_secs(3), engine.started.notified())
        .await
        .unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    let rows = audit_rows(&f.path, 1).await;
    assert_eq!(rows[0].get::<String, _>("status"), "cancelled");
    assert_eq!(rows[0].get::<i64, _>("row_count"), 0);
    assert_eq!(rows[0].get::<String, _>("sql_sha256").len(), 64);
    assert_eq!(f.store.read_audit().health().pending, 0);
    let tokens = TokenCodec::new(b"audit-test-key".to_vec()).unwrap();
    let next = QueryService {
        principal: &f.principal,
        scope: &f.scope,
        active: Arc::clone(&f.active),
        engine: &NeverEngine,
        store: &f.store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &tokens,
    }
    .execute(request("Airport", "BER", "name"))
    .await
    .unwrap();
    assert_eq!(
        next.rows.len(),
        1,
        "cancelled read must release its audit capacity"
    );
    assert_eq!(audit_rows(&f.path, 2).await.len(), 2);
}

#[tokio::test]
async fn audit_pk_full_queue_rejects_before_releasing_another_result() {
    let f = fixture(Some(ReadAuditSettings {
        capacity: 1,
        batch_size: 1,
        flush_interval: Duration::from_millis(10),
        max_pending_age: Duration::from_millis(100),
        ..ReadAuditSettings::default()
    }))
    .await;
    let pool =
        sqlx::SqlitePool::connect_with(ontoforge_storage::store::sqlite_connect_options(&f.path))
            .await
            .unwrap();
    let mut lock = pool.acquire().await.unwrap();
    sqlx::query("BEGIN IMMEDIATE")
        .execute(&mut *lock)
        .await
        .unwrap();
    let tokens = TokenCodec::new(b"audit-test-key".to_vec()).unwrap();
    let service = QueryService {
        principal: &f.principal,
        scope: &f.scope,
        active: Arc::clone(&f.active),
        engine: &NeverEngine,
        store: &f.store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &tokens,
    };
    let first = service
        .execute(request("Airport", "BER", "name"))
        .await
        .unwrap();
    assert_eq!(first.rows.len(), 1);
    let second = service
        .execute(request("Airport", "BER", "name"))
        .await
        .unwrap_err();
    assert!(matches!(second, QueryError::AuditUnavailable));
    assert_eq!(second.code(), "AUDIT_UNAVAILABLE");
    sqlx::query("ROLLBACK").execute(&mut *lock).await.unwrap();
    let rows = audit_rows(&f.path, 1).await;
    assert_eq!(
        rows.len(),
        1,
        "a refused read has no result and no second audit slot"
    );
}

#[tokio::test]
async fn audit_worker_shutdown_drains_and_reopen_recovers() {
    let f = Arc::new(
        fixture(Some(ReadAuditSettings {
            capacity: 2,
            batch_size: 1,
            flush_interval: Duration::from_millis(10),
            max_pending_age: Duration::from_millis(100),
            ..ReadAuditSettings::default()
        }))
        .await,
    );
    let pool =
        sqlx::SqlitePool::connect_with(ontoforge_storage::store::sqlite_connect_options(&f.path))
            .await
            .unwrap();
    let mut lock = pool.acquire().await.unwrap();
    sqlx::query("BEGIN IMMEDIATE")
        .execute(&mut *lock)
        .await
        .unwrap();
    let tokens = TokenCodec::new(b"audit-test-key".to_vec()).unwrap();
    let first = QueryService {
        principal: &f.principal,
        scope: &f.scope,
        active: Arc::clone(&f.active),
        engine: &NeverEngine,
        store: &f.store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &tokens,
    }
    .execute(request("Airport", "BER", "name"))
    .await
    .unwrap();
    assert_eq!(first.rows.len(), 1);
    assert_eq!(f.store.read_audit().health().pending, 1);

    let close_store = Arc::clone(&f);
    let shutdown = tokio::spawn(async move { close_store.store.read_audit().shutdown().await });
    tokio::time::sleep(Duration::from_millis(30)).await;
    sqlx::query("ROLLBACK").execute(&mut *lock).await.unwrap();
    tokio::time::timeout(Duration::from_secs(3), shutdown)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let rows = audit_rows(&f.path, 1).await;
    assert_eq!(
        rows.len(),
        1,
        "shutdown must drain accepted records to SQLite"
    );
    assert!(
        !f.store.read_audit().health().available,
        "shutdown must keep the read gate closed"
    );
    let denied = QueryService {
        principal: &f.principal,
        scope: &f.scope,
        active: Arc::clone(&f.active),
        engine: &NeverEngine,
        store: &f.store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &tokens,
    }
    .execute(request("Airport", "BER", "name"))
    .await
    .unwrap_err();
    assert!(matches!(denied, QueryError::AuditUnavailable));

    let reopened = EditStore::open(StoreSettings {
        backend: BackendSettings::Sqlite {
            path: f.path.clone(),
        },
        lock_timeout: Duration::from_secs(5),
        probe_timeout: Duration::from_secs(5),
        trino_catalog: None,
    })
    .await
    .unwrap();
    let after_restart = QueryService {
        principal: &f.principal,
        scope: &f.scope,
        active: Arc::clone(&f.active),
        engine: &NeverEngine,
        store: &reopened,
        limits: ExecutionLimits::phase4_default(),
        tokens: &tokens,
    }
    .execute(request("Airport", "BER", "name"))
    .await
    .unwrap();
    assert_eq!(after_restart.rows.len(), 1);
    assert_eq!(audit_rows(&f.path, 2).await.len(), 2);
}

#[tokio::test]
async fn audit_worker_persistent_sink_failure_stops_and_reports_failed_drain() {
    let f = fixture(Some(ReadAuditSettings {
        capacity: 2,
        batch_size: 1,
        flush_interval: Duration::from_millis(10),
        max_pending_age: Duration::from_millis(100),
        ..ReadAuditSettings::default()
    }))
    .await;
    let pool =
        sqlx::SqlitePool::connect_with(ontoforge_storage::store::sqlite_connect_options(&f.path))
            .await
            .unwrap();
    sqlx::query("DROP TABLE of_read_audit")
        .execute(&pool)
        .await
        .unwrap();
    let tokens = TokenCodec::new(b"audit-test-key".to_vec()).unwrap();
    let service = QueryService {
        principal: &f.principal,
        scope: &f.scope,
        active: Arc::clone(&f.active),
        engine: &NeverEngine,
        store: &f.store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &tokens,
    };
    assert_eq!(
        service
            .execute(request("Airport", "BER", "name"))
            .await
            .unwrap()
            .rows
            .len(),
        1
    );
    tokio::time::timeout(Duration::from_secs(2), async {
        while f.store.read_audit().health().available {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(matches!(
        service.execute(request("Airport", "BER", "name")).await,
        Err(QueryError::AuditUnavailable)
    ));
    let result = tokio::time::timeout(Duration::from_secs(3), f.store.read_audit().shutdown())
        .await
        .expect("shutdown must not wait forever on a permanently failing sink");
    assert!(
        result.is_err(),
        "failed drain must not claim that records were persisted"
    );
    assert!(!f.store.read_audit().health().available);
}

#[tokio::test]
async fn audit_worker_pending_age_closes_gate_until_blocked_writer_recovers() {
    let f = fixture(Some(ReadAuditSettings {
        capacity: 2,
        batch_size: 1,
        flush_interval: Duration::from_millis(250),
        max_pending_age: Duration::from_millis(1000),
        ..ReadAuditSettings::default()
    }))
    .await;
    let pool =
        sqlx::SqlitePool::connect_with(ontoforge_storage::store::sqlite_connect_options(&f.path))
            .await
            .unwrap();
    let mut lock = pool.acquire().await.unwrap();
    sqlx::query("BEGIN IMMEDIATE")
        .execute(&mut *lock)
        .await
        .unwrap();
    let tokens = TokenCodec::new(b"audit-test-key".to_vec()).unwrap();
    let service = QueryService {
        principal: &f.principal,
        scope: &f.scope,
        active: Arc::clone(&f.active),
        engine: &NeverEngine,
        store: &f.store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &tokens,
    };
    assert_eq!(
        service
            .execute(request("Airport", "BER", "name"))
            .await
            .unwrap()
            .rows
            .len(),
        1
    );
    tokio::time::timeout(Duration::from_millis(1500), async {
        while f.store.read_audit().health().available {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("pending age should close the read gate near one second");
    let health = f.store.read_audit().health();
    assert_eq!(health.pending, 1);
    assert!(health.oldest_pending.unwrap() >= Duration::from_millis(1000));
    assert!(matches!(
        service.execute(request("Airport", "BER", "name")).await,
        Err(QueryError::AuditUnavailable)
    ));
    sqlx::query("ROLLBACK").execute(&mut *lock).await.unwrap();
    assert_eq!(audit_rows(&f.path, 1).await.len(), 1);
    assert!(f.store.read_audit().health().available);
    assert_eq!(
        service
            .execute(request("Airport", "BER", "name"))
            .await
            .unwrap()
            .rows
            .len(),
        1
    );
    assert_eq!(audit_rows(&f.path, 2).await.len(), 2);
}

#[tokio::test]
async fn audit_worker_transient_sink_failure_recovers_before_retry_limit() {
    let f = fixture(Some(ReadAuditSettings {
        capacity: 2,
        batch_size: 1,
        flush_interval: Duration::from_millis(10),
        max_pending_age: Duration::from_millis(1000),
        ..ReadAuditSettings::default()
    }))
    .await;
    let pool =
        sqlx::SqlitePool::connect_with(ontoforge_storage::store::sqlite_connect_options(&f.path))
            .await
            .unwrap();
    sqlx::query("ALTER TABLE of_read_audit RENAME TO audit_hold")
        .execute(&pool)
        .await
        .unwrap();
    let tokens = TokenCodec::new(b"audit-test-key".to_vec()).unwrap();
    let service = QueryService {
        principal: &f.principal,
        scope: &f.scope,
        active: Arc::clone(&f.active),
        engine: &NeverEngine,
        store: &f.store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &tokens,
    };
    assert_eq!(
        service
            .execute(request("Airport", "BER", "name"))
            .await
            .unwrap()
            .rows
            .len(),
        1
    );
    tokio::time::timeout(Duration::from_millis(500), async {
        while f.store.read_audit().health().available {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(matches!(
        service.execute(request("Airport", "BER", "name")).await,
        Err(QueryError::AuditUnavailable)
    ));
    sqlx::query("ALTER TABLE audit_hold RENAME TO of_read_audit")
        .execute(&pool)
        .await
        .unwrap();
    let rows = audit_rows(&f.path, 1).await;
    assert_eq!(rows.len(), 1);
    assert!(f.store.read_audit().health().available);
    assert_eq!(
        service
            .execute(request("Airport", "BER", "name"))
            .await
            .unwrap()
            .rows
            .len(),
        1
    );
    assert_eq!(audit_rows(&f.path, 2).await.len(), 2);
}

#[test]
fn audit_worker_rejects_invalid_limits() {
    let defaults = ReadAuditSettings::default();
    for invalid in [
        ReadAuditSettings {
            capacity: 0,
            ..defaults
        },
        ReadAuditSettings {
            capacity: 65_537,
            ..defaults
        },
        ReadAuditSettings {
            batch_size: 0,
            ..defaults
        },
        ReadAuditSettings {
            batch_size: defaults.capacity + 1,
            ..defaults
        },
        ReadAuditSettings {
            flush_interval: Duration::ZERO,
            ..defaults
        },
        ReadAuditSettings {
            flush_interval: Duration::from_millis(1001),
            ..defaults
        },
        ReadAuditSettings {
            max_pending_age: Duration::from_millis(249),
            ..defaults
        },
        ReadAuditSettings {
            max_pending_age: Duration::from_millis(1001),
            ..defaults
        },
        ReadAuditSettings {
            retention: Duration::ZERO,
            ..defaults
        },
        ReadAuditSettings {
            retention: Duration::from_hours(3651 * 24),
            ..defaults
        },
    ] {
        assert!(
            invalid.validate().is_err(),
            "invalid read audit settings accepted: {invalid:?}"
        );
    }
    assert!(defaults.validate().is_ok());
}
