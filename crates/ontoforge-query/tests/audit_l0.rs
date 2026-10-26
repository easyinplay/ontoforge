#![allow(
    clippy::unused_async_trait_impl,
    reason = "test engines return immediately or wait for cancellation"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::Duration;

use camino::{Utf8Path, Utf8PathBuf};
use ontoforge_auth::Principal;
use ontoforge_auth::authn::PrincipalId;
use ontoforge_auth::scope::{PrincipalScope, TypeScope};
use ontoforge_engine::sqlite::SqliteEngine;
use ontoforge_engine::sqlite::fixture::plan_fixture_tables;
use ontoforge_engine::{
    Engine, EngineError, EngineStatus, PhysicalColumn, PrintedSql, QueryContext, QueryOutput,
    TableRef,
};
use ontoforge_expr::{Attrs, Pred};
use ontoforge_query::QueryError;
use ontoforge_query::ast::PageSpec;
use ontoforge_query::compile::{CompileLimits, ValidatedQuery};
use ontoforge_query::{ExecutionLimits, ObjectSet, QueryRequest, QueryService, TokenCodec};
use ontoforge_schema::ir::{ObjectTypeName, ProfileName, PropertyName};
use ontoforge_schema::load_dir;
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

fn name(raw: &str) -> ObjectTypeName {
    ObjectTypeName::try_from(raw.to_owned()).unwrap()
}
fn prop(raw: &str) -> PropertyName {
    PropertyName::try_from(raw.to_owned()).unwrap()
}

struct Fixture {
    _dir: tempfile::TempDir,
    path: Utf8PathBuf,
    store: EditStore,
    active: Arc<ontoforge_storage::store::ActiveDefinition>,
    ontology: ontoforge_schema::ir::Ontology,
    scope: PrincipalScope,
    principal: Principal,
    engine: SqliteEngine,
}

async fn fixture(settings: Option<ReadAuditSettings>) -> Fixture {
    let root = Utf8Path::new(AVIATION);
    let loaded = load_dir(root);
    let ontology = loaded.ontology.unwrap();
    let profile = ProfileName::try_from("dev".to_owned()).unwrap();
    let tables = plan_fixture_tables(&ontology, &loaded.sources, &profile).unwrap();
    let engine = SqliteEngine::from_fixtures(root, &tables.tables)
        .await
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = Utf8PathBuf::from_path_buf(dir.path().join("edit.db")).unwrap();
    let store = EditStore::open_with_read_audit(
        StoreSettings {
            backend: BackendSettings::Sqlite { path: path.clone() },
            lock_timeout: Duration::from_secs(5),
            probe_timeout: Duration::from_secs(5),
            trino_catalog: None,
        },
        settings.unwrap_or_default(),
    )
    .await
    .unwrap();
    reload(&store, &ontology, &ReloadOptions::default())
        .await
        .unwrap();
    let active = store.active_definition().await.unwrap();
    store
        .commit(Commit {
            actor: Actor::Principal("test|l0-reader".to_owned()),
            action: "airportFixture".to_owned(),
            def_version: active.def_version,
            params: json!({}),
            edits: vec![Edit::Object {
                type_name: name("Airport"),
                pk: PropertyValue::String("FRA".to_owned()),
                op: ObjectOp::Set(
                    [(prop("name"), PropertyValue::String("Frankfurt".to_owned()))].into(),
                ),
            }],
        })
        .await
        .unwrap();
    let scope = PrincipalScope {
        types: ["Flight", "Airport"]
            .into_iter()
            .map(|raw| {
                let ty = name(raw);
                (
                    ty.clone(),
                    TypeScope {
                        row: Pred::Const(true),
                        columns: ontology.object_types[&ty]
                            .properties
                            .keys()
                            .cloned()
                            .map(|p| (p, Pred::Const(true)))
                            .collect(),
                    },
                )
            })
            .collect(),
        ..PrincipalScope::default()
    };
    let principal = Principal {
        id: PrincipalId::new("test", "l0-reader"),
        issuer: "test".into(),
        subject: "l0-reader".into(),
        groups: BTreeSet::new(),
        attrs: Attrs::default(),
    };
    Fixture {
        _dir: dir,
        path,
        store,
        active,
        ontology,
        scope,
        principal,
        engine,
    }
}

fn query(f: &Fixture, set: ObjectSet, selected: &str) -> ValidatedQuery {
    let ty = set.object_type().clone();
    ValidatedQuery::new(
        QueryRequest {
            set,
            select: vec![prop(selected)],
        },
        vec![],
        PageSpec {
            first: 10,
            after: None,
        },
        &f.ontology,
        &f.scope.types[&ty],
        CompileLimits::default(),
    )
    .unwrap()
}

async fn rows(path: &Utf8PathBuf, expected: usize) -> Vec<sqlx::sqlite::SqliteRow> {
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
        if rows.len() >= expected {
            return rows;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "expected {expected} audit rows, found {}",
            rows.len()
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[tokio::test]
async fn audit_list_type_filter_set_and_search_around_record_once() {
    let f = fixture(None).await;
    let tokens = TokenCodec::new(b"l0-audit-test-key".to_vec()).unwrap();
    let service = QueryService {
        principal: &f.principal,
        scope: &f.scope,
        active: Arc::clone(&f.active),
        engine: &f.engine,
        store: &f.store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &tokens,
    };
    let modes = [
        (
            json!({"kind":"type", "object_type":"Flight"}),
            "status",
            "[]",
        ),
        (
            json!({"kind":"where", "object_type":"Flight", "predicate":{"op":"eq", "property":"status", "value":"scheduled"}}),
            "status",
            "[\"status\"]",
        ),
        (
            json!({"kind":"union", "object_type":"Flight", "sets":[{"kind":"pk", "object_type":"Flight", "pk":"F-1001"},{"kind":"pk", "object_type":"Flight", "pk":"F-1002"}]}),
            "status",
            "[\"flightId\"]",
        ),
        (
            json!({"kind":"searchAround", "object_type":"Airport", "base":{"kind":"pk", "object_type":"Flight", "pk":"F-1001"}, "link":"flightDepartureAirport", "direction":"toFrom"}),
            "code",
            "[\"flightId\",\"flightDepartureAirport\"]",
        ),
    ];
    for (index, (set, selected, conditions)) in modes.into_iter().enumerate() {
        let page = service
            .execute_list(&query(&f, serde_json::from_value(set).unwrap(), selected))
            .await
            .unwrap();
        assert!(!page.rows.is_empty(), "mode {index} returned no rows");
        let audit = rows(&f.path, index + 1).await;
        assert_eq!(audit.len(), index + 1, "one row per public request");
        assert_eq!(audit[index].get::<String, _>("tool"), "query.list");
        assert_eq!(audit[index].get::<String, _>("status"), "success");
        assert_eq!(
            audit[index].get::<i64, _>("row_count"),
            i64::try_from(page.rows.len()).unwrap()
        );
        assert_eq!(
            audit[index].get::<String, _>("condition_api_names"),
            conditions
        );
        assert_eq!(audit[index].get::<String, _>("sql_sha256").len(), 64);
        assert_eq!(
            audit[index].get::<i64, _>("def_version"),
            f.active.def_version
        );
        assert_eq!(
            audit[index].get::<String, _>("principal"),
            f.principal.id.to_string()
        );
        assert!(!audit[index].get::<String, _>("request_id").is_empty());
        assert!(audit[index].get::<i64, _>("elapsed_ms") >= 0);
    }
}

struct FaultEngine {
    truncated: bool,
    started: Notify,
}
impl Engine for FaultEngine {
    fn dialect(&self) -> ontoforge_engine::printer::Dialect {
        ontoforge_engine::printer::Dialect::Sqlite
    }
    async fn execute(&self, _: &PrintedSql, _: &QueryContext) -> Result<QueryOutput, EngineError> {
        self.started.notify_one();
        if self.truncated {
            Ok(QueryOutput {
                columns: vec![],
                rows: vec![],
                truncated: true,
            })
        } else {
            Err(EngineError::Query {
                error_name: "hidden_sql_secret".into(),
                message: "hidden_result_secret".into(),
            })
        }
    }
    async fn describe_tables(
        &self,
        _: &[TableRef],
        _: &QueryContext,
    ) -> Result<BTreeMap<TableRef, Vec<PhysicalColumn>>, EngineError> {
        unreachable!()
    }
    async fn status(&self) -> EngineStatus {
        EngineStatus::Available
    }
}

#[tokio::test]
async fn audit_list_denied_source_error_and_truncation_record_status() {
    let f = fixture(None).await;
    let tokens = TokenCodec::new(b"l0-audit-test-key".to_vec()).unwrap();
    let list = query(
        &f,
        ObjectSet::Type {
            object_type: name("Flight"),
        },
        "status",
    );
    let mut denied_scope = f.scope.clone();
    denied_scope.types.remove(&name("Flight"));
    let denied = QueryService {
        principal: &f.principal,
        scope: &denied_scope,
        active: Arc::clone(&f.active),
        engine: &f.engine,
        store: &f.store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &tokens,
    };
    assert!(matches!(
        denied.execute_list(&list).await,
        Err(QueryError::InvalidQuery)
    ));
    for truncated in [false, true] {
        let engine = FaultEngine {
            truncated,
            started: Notify::new(),
        };
        let service = QueryService {
            principal: &f.principal,
            scope: &f.scope,
            active: Arc::clone(&f.active),
            engine: &engine,
            store: &f.store,
            limits: ExecutionLimits::phase4_default(),
            tokens: &tokens,
        };
        assert!(matches!(
            service.execute_list(&list).await,
            Err(QueryError::Source | QueryError::WorkBudgetExceeded)
        ));
    }
    let audit = rows(&f.path, 3).await;
    assert_eq!(audit.len(), 3);
    assert_eq!(
        audit
            .iter()
            .map(|r| r.get::<String, _>("status"))
            .collect::<Vec<_>>(),
        ["denied", "source_error", "denied"]
    );
    assert_eq!(audit[1].get::<String, _>("sql_sha256").len(), 64);
    assert!(
        !String::from_utf8_lossy(&std::fs::read(&f.path).unwrap()).contains("hidden_sql_secret")
    );
}

struct BlockingEngine {
    started: Notify,
}
impl Engine for BlockingEngine {
    fn dialect(&self) -> ontoforge_engine::printer::Dialect {
        ontoforge_engine::printer::Dialect::Sqlite
    }
    async fn execute(&self, _: &PrintedSql, _: &QueryContext) -> Result<QueryOutput, EngineError> {
        self.started.notify_one();
        std::future::pending().await
    }
    async fn describe_tables(
        &self,
        _: &[TableRef],
        _: &QueryContext,
    ) -> Result<BTreeMap<TableRef, Vec<PhysicalColumn>>, EngineError> {
        unreachable!()
    }
    async fn status(&self) -> EngineStatus {
        EngineStatus::Available
    }
}

#[tokio::test]
async fn audit_list_cancellation_records_event_and_releases_slot() {
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
            let tokens = TokenCodec::new(b"l0-audit-test-key".to_vec()).unwrap();
            let list = query(
                &f,
                ObjectSet::Type {
                    object_type: name("Flight"),
                },
                "status",
            );
            QueryService {
                principal: &f.principal,
                scope: &f.scope,
                active: Arc::clone(&f.active),
                engine: &*engine,
                store: &f.store,
                limits: ExecutionLimits::phase4_default(),
                tokens: &tokens,
            }
            .execute_list(&list)
            .await
        })
    };
    tokio::time::timeout(Duration::from_secs(3), engine.started.notified())
        .await
        .unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    let audit = rows(&f.path, 1).await;
    assert_eq!(audit[0].get::<String, _>("status"), "cancelled");
    assert_eq!(audit[0].get::<i64, _>("row_count"), 0);
    assert_eq!(f.store.read_audit().health().pending, 0);
}

#[tokio::test]
async fn audit_list_sink_failure_closes_gate_without_releasing_result() {
    let f = fixture(Some(ReadAuditSettings {
        capacity: 1,
        batch_size: 1,
        flush_interval: Duration::from_millis(10),
        max_pending_age: Duration::from_millis(250),
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
    let tokens = TokenCodec::new(b"l0-audit-test-key".to_vec()).unwrap();
    let list = query(
        &f,
        ObjectSet::Type {
            object_type: name("Flight"),
        },
        "status",
    );
    let reader = QueryService {
        principal: &f.principal,
        scope: &f.scope,
        active: Arc::clone(&f.active),
        engine: &f.engine,
        store: &f.store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &tokens,
    };
    let _ = reader.execute_list(&list).await;
    tokio::time::timeout(Duration::from_secs(2), async {
        while f.store.read_audit().health().available {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(matches!(
        reader.execute_list(&list).await,
        Err(QueryError::AuditUnavailable)
    ));
    sqlx::query("ALTER TABLE audit_hold RENAME TO of_read_audit")
        .execute(&pool)
        .await
        .unwrap();
}
