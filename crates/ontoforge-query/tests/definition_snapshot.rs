use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::Duration;

use camino::{Utf8Path, Utf8PathBuf};
use ontoforge_auth::Principal;
use ontoforge_auth::authn::PrincipalId;
use ontoforge_auth::scope::{PrincipalScope, ScopeInputs, TypeScope, compile_scope};
use ontoforge_engine::config::{TrinoConfig, build_trino_engines, parse_server_config};
use ontoforge_engine::printer::Dialect;
use ontoforge_engine::sqlite::SqliteEngine;
use ontoforge_engine::sqlite::fixture::plan_fixture_tables;
use ontoforge_engine::trino::TrinoEngine;
use ontoforge_engine::trino::l2_health::L2HealthStatus;
use ontoforge_engine::{
    Engine, EngineError, EngineStatus, PhysicalColumn, PrintedSql, QueryContext, QueryOutput,
    TableRef,
};
use ontoforge_expr::{Attrs, Pred};
use ontoforge_query::ast::PageSpec;
use ontoforge_query::compile::{CompileLimits, ValidatedQuery};
use ontoforge_query::l2::{L2Error, L2Service};
use ontoforge_query::{ExecutionLimits, ObjectSet, QueryRequest, QueryService, TokenCodec};
use ontoforge_schema::ir::{
    AttrType, ObjectTypeName, ProfileName, PropertyName, RoleName, ScopeSet, SqlLevel,
};
use ontoforge_schema::{load_dir, load_scope_dir};
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::store::{BackendSettings, EditStore, StoreSettings};
use serde_json::json;
use sqlx::Row;
use tokio::sync::Notify;
use wiremock::matchers::{body_string_contains, header, method};
use wiremock::{Mock, MockServer, ResponseTemplate};

const AVIATION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/ontologies/aviation"
);
const AVIATION_SCOPE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/scopes/aviation"
);

struct PausedEngine {
    inner: SqliteEngine,
    started: Notify,
    release: Notify,
}

impl Engine for PausedEngine {
    fn dialect(&self) -> Dialect {
        self.inner.dialect()
    }

    async fn execute(
        &self,
        sql: &PrintedSql,
        ctx: &QueryContext,
    ) -> Result<QueryOutput, EngineError> {
        self.started.notify_one();
        self.release.notified().await;
        self.inner.execute(sql, ctx).await
    }

    async fn describe_tables(
        &self,
        tables: &[TableRef],
        ctx: &QueryContext,
    ) -> Result<BTreeMap<TableRef, Vec<PhysicalColumn>>, EngineError> {
        self.inner.describe_tables(tables, ctx).await
    }

    async fn status(&self) -> EngineStatus {
        self.inner.status().await
    }
}

async fn audit_rows(path: &Utf8PathBuf, expected: usize) -> Vec<sqlx::sqlite::SqliteRow> {
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
#[allow(
    clippy::too_many_lines,
    reason = "single controlled L0 concurrency scenario"
)]
async fn l0_inflight_reload_and_scope_apply_keep_request_snapshot() {
    let loaded = load_dir(Utf8Path::new(AVIATION));
    assert!(loaded.diagnostics.is_empty(), "{:#?}", loaded.diagnostics);
    let ontology = loaded.ontology.unwrap();
    let profile = ProfileName::try_from("dev".to_owned()).unwrap();
    let tables = plan_fixture_tables(&ontology, &loaded.sources, &profile).unwrap();
    let engine = Arc::new(PausedEngine {
        inner: SqliteEngine::from_fixtures(Utf8Path::new(AVIATION), &tables.tables)
            .await
            .unwrap(),
        started: Notify::new(),
        release: Notify::new(),
    });
    let dir = tempfile::tempdir().unwrap();
    let path = Utf8PathBuf::from_path_buf(dir.path().join("edit.db")).unwrap();
    let store = Arc::new(
        EditStore::open(StoreSettings {
            backend: BackendSettings::Sqlite { path: path.clone() },
            lock_timeout: Duration::from_secs(5),
            probe_timeout: Duration::from_secs(5),
            trino_catalog: None,
        })
        .await
        .unwrap(),
    );
    reload(&store, &ontology, &ReloadOptions::default())
        .await
        .unwrap();
    let first = store.active_definition().await.unwrap();
    let scope_file = load_scope_dir(Utf8Path::new(AVIATION_SCOPE));
    assert!(
        scope_file.diagnostics.is_empty(),
        "{:#?}",
        scope_file.diagnostics
    );
    let granted = scope_file.scope.unwrap();
    let props = compile_scope(&ScopeInputs {
        ontology: &ontology,
        profile: Some(&profile),
        attrs: &BTreeMap::from([("homeAirports".to_owned(), AttrType::StringList)]),
        scope: &granted,
        sources: None,
    })
    .unwrap()
    .row_filter_properties();
    let plan = store.scope_plan(&granted, &props).await.unwrap();
    store
        .scope_apply(&granted, &props, &plan.hash, "test@host")
        .await
        .unwrap();
    let flight = ObjectTypeName::try_from("Flight".to_owned()).unwrap();
    let scope = PrincipalScope {
        roles: BTreeSet::from([RoleName::try_from("dispatcher".to_owned()).unwrap()]),
        types: BTreeMap::from([(
            flight.clone(),
            TypeScope {
                row: Pred::Const(true),
                columns: ontology.object_types[&flight]
                    .properties
                    .keys()
                    .cloned()
                    .map(|p| (p, Pred::Const(true)))
                    .collect(),
            },
        )]),
        ..PrincipalScope::default()
    };
    let principal = Principal {
        id: PrincipalId::new("test", "snapshot"),
        issuer: "test".into(),
        subject: "snapshot".into(),
        groups: BTreeSet::new(),
        attrs: Attrs::default(),
    };
    let query = ValidatedQuery::new(
        QueryRequest {
            set: ObjectSet::Type {
                object_type: flight.clone(),
            },
            select: vec![PropertyName::try_from("status".to_owned()).unwrap()],
        },
        vec![],
        PageSpec {
            first: 10,
            after: None,
        },
        &ontology,
        &scope.types[&flight],
        CompileLimits::default(),
    )
    .unwrap();
    let tokens = TokenCodec::new(b"snapshot-key".to_vec()).unwrap();
    let worker = {
        let engine = Arc::clone(&engine);
        let store = Arc::clone(&store);
        let principal = principal.clone();
        let scope = scope.clone();
        let first = Arc::clone(&first);
        tokio::spawn(async move {
            let tokens = TokenCodec::new(b"snapshot-key".to_vec()).unwrap();
            QueryService {
                principal: &principal,
                scope: &scope,
                active: first,
                engine: &*engine,
                store: &store,
                limits: ExecutionLimits::phase4_default(),
                tokens: &tokens,
            }
            .execute_list(&query)
            .await
        })
    };
    tokio::time::timeout(Duration::from_secs(3), engine.started.notified())
        .await
        .unwrap();
    let empty = ScopeSet::default();
    let empty_plan = store.scope_plan(&empty, &BTreeMap::new()).await.unwrap();
    store
        .scope_apply(&empty, &BTreeMap::new(), &empty_plan.hash, "test@host")
        .await
        .unwrap();
    let mut changed = ontology.clone();
    changed.description = Some("snapshot version two".to_owned());
    reload(&store, &changed, &ReloadOptions::default())
        .await
        .unwrap();
    let second = store.active_definition().await.unwrap();
    assert_eq!(second.def_version, first.def_version + 1);
    engine.release.notify_one();
    let page = worker
        .await
        .unwrap()
        .expect("in-flight read keeps first grant");
    assert!(!page.rows.is_empty());
    let query = ValidatedQuery::new(
        QueryRequest {
            set: ObjectSet::Type {
                object_type: flight.clone(),
            },
            select: vec![PropertyName::try_from("status".to_owned()).unwrap()],
        },
        vec![],
        PageSpec {
            first: 10,
            after: None,
        },
        &ontology,
        &scope.types[&flight],
        CompileLimits::default(),
    )
    .unwrap();
    let denied = QueryService {
        principal: &principal,
        scope: &PrincipalScope::default(),
        active: second.clone(),
        engine: &engine.inner,
        store: &store,
        limits: ExecutionLimits::phase4_default(),
        tokens: &tokens,
    }
    .execute_list(&query)
    .await;
    assert!(denied.is_err(), "new request loses the Flight grant");
    let rows = audit_rows(&path, 2).await;
    assert_eq!(rows[0].get::<i64, _>("def_version"), first.def_version);
    assert_eq!(
        rows[0].get::<String, _>("effective_roles"),
        "[\"dispatcher\"]"
    );
    assert_eq!(rows[0].get::<String, _>("status"), "success");
    assert_eq!(rows[1].get::<i64, _>("def_version"), second.def_version);
    assert_eq!(rows[1].get::<String, _>("effective_roles"), "[]");
    assert_eq!(rows[1].get::<String, _>("status"), "denied");
}

fn l2_config(url: &str) -> TrinoConfig {
    parse_server_config(&format!(
        "[trino]\nurl = \"{url}\"\nmax_rows = 8\nmax_bytes = 4096\n[trino.identities.ontology]\nuser = \"ontology\"\n[trino.identities.l2]\nuser = \"l2\"\n[trino.l2_acl.allowed]\ncatalog = \"demo\"\nschema = \"tiny\"\ntable = \"nation\"\n[trino.l2_acl.outside]\ncatalog = \"outside\"\nschema = \"tiny\"\ntable = \"nation\"\n[trino.l2_acl.edit]\ncatalog = \"editstore\"\nschema = \"tiny\"\ntable = \"nation\"\n"
    )).unwrap().trino.unwrap()
}

async fn l2_reply(server: &MockServer, user: &str, needle: &str, body: serde_json::Value) {
    Mock::given(method("POST"))
        .and(wiremock::matchers::path("/v1/statement"))
        .and(header("x-trino-user", user))
        .and(body_string_contains(needle))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(server)
        .await;
}

async fn attest_l2(server: &MockServer, ontology: &TrinoEngine, l2: &TrinoEngine) {
    let success = json!({"id":"probe", "columns":[{"name":"x","type":"integer"}], "data":[[1]], "stats":{"state":"FINISHED"}});
    for catalog in ["demo", "outside", "editstore"] {
        l2_reply(
            server,
            "ontology",
            &format!("\"{catalog}\"."),
            success.clone(),
        )
        .await;
    }
    l2_reply(server, "l2", "\"demo\".", success).await;
    let denied = json!({"id":"probe", "error":{"errorName":"PERMISSION_DENIED", "message":"denied"}, "stats":{"state":"FAILED"}});
    for catalog in ["outside", "editstore"] {
        l2_reply(server, "l2", &format!("\"{catalog}\"."), denied.clone()).await;
    }
    l2_reply(server, "ontology", "other_1", json!({"id":"probe", "columns":[{"name":"query_id","type":"varchar"},{"name":"state","type":"varchar"}], "data":[["other_1","RUNNING"]], "stats":{"state":"FINISHED"}})).await;
    l2_reply(server, "l2", "other_1", json!({"id":"probe", "columns":[{"name":"query_id","type":"varchar"}], "data":[], "stats":{"state":"FINISHED"}})).await;
    assert_eq!(
        l2.recheck_l2_acl(ontology, "other_1").await,
        L2HealthStatus::Enabled
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[allow(
    clippy::too_many_lines,
    reason = "single controlled L2 concurrency scenario"
)]
async fn l2_inflight_reload_and_scope_apply_keep_request_snapshot() {
    let server = MockServer::start().await;
    let config = l2_config(&server.uri());
    let env = |_: &str| None;
    let engines = build_trino_engines(&config, Utf8Path::new("."), &env).unwrap();
    let ontology_engine = engines.ontology;
    let engine = engines.l2.unwrap();
    attest_l2(&server, &ontology_engine, &engine).await;

    let dir = tempfile::tempdir().unwrap();
    let path = Utf8PathBuf::from_path_buf(dir.path().join("edit.db")).unwrap();
    let store = EditStore::open(StoreSettings {
        backend: BackendSettings::Sqlite { path: path.clone() },
        lock_timeout: Duration::from_secs(5),
        probe_timeout: Duration::from_secs(5),
        trino_catalog: Some("editstore".to_owned()),
    })
    .await
    .unwrap();
    let ontology = load_dir(Utf8Path::new(AVIATION)).ontology.unwrap();
    reload(&store, &ontology, &ReloadOptions::default())
        .await
        .unwrap();
    let first = store.active_definition().await.unwrap();
    let scope_file = load_scope_dir(Utf8Path::new(AVIATION_SCOPE));
    assert!(
        scope_file.diagnostics.is_empty(),
        "{:#?}",
        scope_file.diagnostics
    );
    let granted = scope_file.scope.unwrap();
    let plan = store.scope_plan(&granted, &BTreeMap::new()).await.unwrap();
    store
        .scope_apply(&granted, &BTreeMap::new(), &plan.hash, "test@host")
        .await
        .unwrap();
    let scope = PrincipalScope {
        roles: BTreeSet::from([RoleName::try_from("dispatcher".to_owned()).unwrap()]),
        sql_levels: BTreeSet::from([SqlLevel::Raw]),
        l2_catalogs: BTreeSet::from(["demo".to_owned()]),
        l2_schemas: BTreeSet::from(["demo.tiny".to_owned()]),
        ..PrincipalScope::default()
    };
    let principal = Principal {
        id: PrincipalId::new("test", "raw-snapshot"),
        issuer: "test".into(),
        subject: "raw-snapshot".into(),
        groups: BTreeSet::new(),
        attrs: Attrs::default(),
    };
    let started = Arc::new(Notify::new());
    let (release, waiting) = std::sync::mpsc::channel::<()>();
    let waiting = std::sync::Mutex::new(waiting);
    let signal = Arc::clone(&started);
    Mock::given(method("POST"))
        .and(wiremock::matchers::path("/v1/statement"))
        .and(header("x-trino-user", "l2"))
        .and(body_string_contains("SELECT nationkey FROM demo.tiny.nation"))
        .respond_with(move |_: &wiremock::Request| {
            signal.notify_one();
            waiting.lock().unwrap().recv_timeout(Duration::from_secs(10)).expect("release in-flight L2 response");
            ResponseTemplate::new(200).set_body_json(json!({"id":"read", "columns":[{"name":"nationkey","type":"integer"}], "data":[[1]], "stats":{"state":"FINISHED"}}))
        })
        .with_priority(1)
        .mount(&server).await;

    let store = Arc::new(store);
    let engine = Arc::new(engine);
    let config = Arc::new(config);
    let worker = {
        let store = Arc::clone(&store);
        let engine = Arc::clone(&engine);
        let config = Arc::clone(&config);
        let principal = principal.clone();
        let scope = scope.clone();
        let first = Arc::clone(&first);
        tokio::spawn(async move {
            L2Service {
                principal: &principal,
                scope: &scope,
                active: first,
                engine: &engine,
                store: &store,
                config: &config,
            }
            .execute_query("SELECT nationkey FROM demo.tiny.nation")
            .await
        })
    };
    tokio::time::timeout(Duration::from_secs(3), started.notified())
        .await
        .unwrap();
    assert!(!worker.is_finished(), "source response is still blocked");
    let empty = ScopeSet::default();
    let empty_plan = store.scope_plan(&empty, &BTreeMap::new()).await.unwrap();
    store
        .scope_apply(&empty, &BTreeMap::new(), &empty_plan.hash, "test@host")
        .await
        .unwrap();
    let mut changed = ontology;
    changed.description = Some("L2 snapshot version two".to_owned());
    reload(&store, &changed, &ReloadOptions::default())
        .await
        .unwrap();
    let second = store.active_definition().await.unwrap();
    assert_eq!(second.def_version, first.def_version + 1);
    assert!(
        !worker.is_finished(),
        "source response must remain blocked until after management changes"
    );
    release.send(()).unwrap();
    let old = worker
        .await
        .unwrap()
        .expect("in-flight L2 read keeps first grant");
    assert_eq!(
        old.output.rows,
        vec![vec![ontoforge_engine::Value::Long(1)]]
    );
    let denied = L2Service {
        principal: &principal,
        scope: &PrincipalScope::default(),
        active: second.clone(),
        engine: &engine,
        store: &store,
        config: &config,
    }
    .execute_query("SELECT nationkey FROM demo.tiny.nation")
    .await;
    assert!(matches!(denied, Err(L2Error::Denied)));
    let rows = audit_rows(&path, 2).await;
    assert_eq!(rows[0].get::<i64, _>("def_version"), first.def_version);
    assert_eq!(
        rows[0].get::<String, _>("effective_roles"),
        "[\"dispatcher\"]"
    );
    assert_eq!(rows[0].get::<String, _>("status"), "success");
    assert_eq!(rows[1].get::<i64, _>("def_version"), second.def_version);
    assert_eq!(rows[1].get::<String, _>("effective_roles"), "[]");
    assert_eq!(rows[1].get::<String, _>("status"), "denied");
}
