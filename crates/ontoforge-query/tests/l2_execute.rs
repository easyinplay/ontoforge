use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use camino::{Utf8Path, Utf8PathBuf};
use ontoforge_auth::Principal;
use ontoforge_auth::authn::PrincipalId;
use ontoforge_auth::scope::PrincipalScope;
use ontoforge_engine::Engine;
use ontoforge_engine::config::{TrinoConfig, build_trino_engines, parse_server_config};
use ontoforge_engine::printer::l2::ExplainType;
use ontoforge_engine::trino::TrinoEngine;
use ontoforge_engine::trino::l2_health::L2HealthStatus;
use ontoforge_expr::Attrs;
use ontoforge_query::l2::{L2Error, L2Service};
use ontoforge_schema::ir::SqlLevel;
use ontoforge_schema::load_dir;
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::store::{ActiveDefinition, BackendSettings, EditStore, StoreSettings};
use serde_json::{Value, json};
use sqlx::Row;
use tokio::sync::Notify;
use wiremock::matchers::{body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const AVIATION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/ontologies/aviation"
);

fn config(url: &str) -> TrinoConfig {
    parse_server_config(&format!(
        "[trino]\nurl = \"{url}\"\nmax_rows = 2\nmax_bytes = 4096\n[trino.identities.ontology]\nuser = \"ontology\"\n[trino.identities.l2]\nuser = \"l2\"\n[trino.l2_acl.allowed]\ncatalog = \"demo\"\nschema = \"tiny\"\ntable = \"nation\"\n[trino.l2_acl.outside]\ncatalog = \"outside\"\nschema = \"tiny\"\ntable = \"nation\"\n[trino.l2_acl.edit]\ncatalog = \"editstore\"\nschema = \"tiny\"\ntable = \"nation\"\n"
    ))
    .unwrap().trino.unwrap()
}

fn data(rows: &Value) -> Value {
    json!({"id":"q1", "columns":[{"name":"nationkey","type":"integer"}], "data":rows, "stats":{"state":"FINISHED"}})
}

async fn answer(server: &MockServer, user: &str, needle: &str, response: Value) {
    Mock::given(method("POST"))
        .and(path("/v1/statement"))
        .and(header("x-trino-user", user))
        .and(body_string_contains(needle))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
        .mount(server)
        .await;
}

async fn attest(server: &MockServer, ontology: &TrinoEngine, l2: &TrinoEngine) {
    for catalog in ["demo", "outside", "editstore"] {
        answer(
            server,
            "ontology",
            &format!("\"{catalog}\"."),
            data(&json!([[1]])),
        )
        .await;
    }
    answer(server, "l2", "\"demo\".", data(&json!([[1]]))).await;
    for catalog in ["outside", "editstore"] {
        answer(server, "l2", &format!("\"{catalog}\"."), json!({"id":"q1", "error":{"errorName":"PERMISSION_DENIED","message":"denied"}, "stats":{"state":"FAILED"}})).await;
    }
    answer(server, "ontology", "other_1", json!({"id":"q1", "columns":[{"name":"query_id","type":"varchar"},{"name":"state","type":"varchar"}], "data":[["other_1","RUNNING"]], "stats":{"state":"FINISHED"}})).await;
    answer(server, "l2", "other_1", json!({"id":"q1", "columns":[{"name":"query_id","type":"varchar"}], "data":[], "stats":{"state":"FINISHED"}})).await;
    assert_eq!(
        l2.recheck_l2_acl(ontology, "other_1").await,
        L2HealthStatus::Enabled
    );
}

struct Fixture {
    _dir: tempfile::TempDir,
    path: Utf8PathBuf,
    config: TrinoConfig,
    ontology: TrinoEngine,
    l2: TrinoEngine,
    principal: Principal,
    scope: PrincipalScope,
    active: Arc<ActiveDefinition>,
    store: EditStore,
}

impl Fixture {
    fn service(&self) -> L2Service<'_> {
        L2Service {
            principal: &self.principal,
            scope: &self.scope,
            active: Arc::clone(&self.active),
            engine: &self.l2,
            store: &self.store,
            config: &self.config,
        }
    }
}

async fn fixture(server: &MockServer) -> Fixture {
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
    let config = config(&server.uri());
    let env = |_: &str| None;
    let engines = build_trino_engines(&config, Utf8Path::new("."), &env).unwrap();
    Fixture {
        _dir: dir,
        path,
        config,
        ontology: engines.ontology,
        l2: engines.l2.unwrap(),
        principal: Principal {
            id: PrincipalId::new("test", "raw-reader"),
            issuer: "test".into(),
            subject: "raw-reader".into(),
            groups: BTreeSet::new(),
            attrs: Attrs::default(),
        },
        scope: PrincipalScope {
            sql_levels: BTreeSet::from([SqlLevel::Raw]),
            l2_catalogs: BTreeSet::from(["demo".to_owned()]),
            l2_schemas: BTreeSet::from(["demo.tiny".to_owned()]),
            ..PrincipalScope::default()
        },
        active: store.active_definition().await.unwrap(),
        store,
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
async fn l2_execute_only_submits_display_sql_after_attestation_and_audits_result() {
    let server = MockServer::start().await;
    let f = fixture(&server).await;
    let sql = "SELECT /* untrusted comment */ nationkey FROM demo.tiny.nation";
    assert!(matches!(
        f.service().execute_query(sql).await,
        Err(L2Error::Disabled)
    ));
    assert!(server.received_requests().await.unwrap().is_empty());
    attest(&server, &f.ontology, &f.l2).await;
    let before = server.received_requests().await.unwrap().len();
    answer(
        &server,
        "l2",
        "SELECT nationkey FROM demo.tiny.nation",
        data(&json!([[1]])),
    )
    .await;
    let result = f
        .service()
        .execute_query(sql)
        .await
        .expect("attested L2 SELECT");
    assert_eq!(result.output.rows.len(), 1);
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), before + 1);
    let body = String::from_utf8(requests.last().unwrap().body.clone()).unwrap();
    assert_eq!(body, "SELECT nationkey FROM demo.tiny.nation LIMIT 3");
    assert!(!body.contains("untrusted comment"));
    let rows = audit_rows(&f.path, 2).await;
    assert_eq!(rows[0].get::<String, _>("status"), "denied");
    assert_eq!(rows[1].get::<String, _>("status"), "success");
    assert_eq!(rows[1].get::<String, _>("tool"), "execute_query");
    assert_eq!(rows[1].get::<i64, _>("row_count"), 1);
    assert_eq!(rows[1].get::<i64, _>("def_version"), f.active.def_version);
    assert_eq!(
        rows[1].get::<String, _>("principal"),
        f.principal.id.to_string()
    );
    assert_eq!(rows[1].get::<String, _>("effective_roles"), "[]");
    assert_eq!(rows[1].get::<String, _>("sql_sha256").len(), 64);
    assert!(!rows[1].get::<String, _>("request_id").is_empty());
}

#[tokio::test]
async fn l2_execute_rejections_are_audited_without_post() {
    let server = MockServer::start().await;
    let mut f = fixture(&server).await;
    attest(&server, &f.ontology, &f.l2).await;
    let before = server.received_requests().await.unwrap().len();
    f.scope.sql_levels.clear();
    assert!(matches!(
        f.service()
            .execute_query("SELECT 1 FROM demo.tiny.nation")
            .await,
        Err(L2Error::Denied)
    ));
    f.scope.sql_levels.insert(SqlLevel::Raw);
    for sql in [
        "SELECT * FROM outside.tiny.nation",
        "SELECT * FROM editstore.tiny.nation",
        "SELECT * FROM system.runtime.queries",
    ] {
        assert!(
            matches!(f.service().execute_query(sql).await, Err(L2Error::Denied)),
            "{sql}"
        );
    }
    assert!(matches!(
        f.service()
            .execute_query("INSERT INTO demo.tiny.nation VALUES (1)")
            .await,
        Err(L2Error::InvalidSql)
    ));
    assert_eq!(
        server.received_requests().await.unwrap().len(),
        before,
        "denials must not submit SQL"
    );
    let rows = audit_rows(&f.path, 5).await;
    assert_eq!(rows.len(), 5);
    assert!(
        rows.iter()
            .all(|row| row.get::<String, _>("status") == "denied")
    );
}

#[tokio::test]
async fn l2_execute_truncation_returns_narrowing_hint_and_audit_status() {
    let server = MockServer::start().await;
    let f = fixture(&server).await;
    attest(&server, &f.ontology, &f.l2).await;
    answer(
        &server,
        "l2",
        "SELECT nationkey FROM demo.tiny.nation",
        data(&json!([[1], [2], [3]])),
    )
    .await;
    let result = f
        .service()
        .execute_query("SELECT nationkey FROM demo.tiny.nation")
        .await
        .unwrap();
    assert_eq!(result.output.rows.len(), 2);
    assert!(result.output.truncated);
    assert!(
        result.narrowing_hint.is_some(),
        "arbitrary SQL has no safe keyset cursor"
    );
    let rows = audit_rows(&f.path, 1).await;
    assert_eq!(rows[0].get::<String, _>("status"), "truncated");
    assert_eq!(rows[0].get::<i64, _>("row_count"), 2);
}

#[tokio::test]
async fn l2_execute_source_error_and_audit_failure_never_release_rows() {
    let server = MockServer::start().await;
    let f = fixture(&server).await;
    attest(&server, &f.ontology, &f.l2).await;
    answer(&server, "l2", "SELECT nationkey FROM demo.tiny.nation", json!({"id":"q1", "error":{"errorName":"INTERNAL_ERROR","message":"sensitive source details"}, "stats":{"state":"FAILED"}})).await;
    assert!(matches!(
        f.service()
            .execute_query("SELECT nationkey FROM demo.tiny.nation")
            .await,
        Err(L2Error::Source(_))
    ));
    let rows = audit_rows(&f.path, 1).await;
    assert_eq!(rows[0].get::<String, _>("status"), "source_error");
    let before = server.received_requests().await.unwrap().len();
    f.store.read_audit().shutdown().await.unwrap();
    assert!(matches!(
        f.service()
            .execute_query("SELECT nationkey FROM demo.tiny.nation")
            .await,
        Err(L2Error::AuditUnavailable)
    ));
    assert_eq!(
        server.received_requests().await.unwrap().len(),
        before,
        "audit failure must prevent the POST"
    );
}

#[tokio::test]
async fn l2_execute_inflight_cancellation_records_event() {
    let server = MockServer::start().await;
    let f = Arc::new(fixture(&server).await);
    attest(&server, &f.ontology, &f.l2).await;
    let started = Arc::new(Notify::new());
    let signal = Arc::clone(&started);
    Mock::given(method("POST"))
        .and(path("/v1/statement"))
        .and(header("x-trino-user", "l2"))
        .and(body_string_contains(
            "SELECT nationkey FROM demo.tiny.nation",
        ))
        .respond_with(move |_: &wiremock::Request| {
            signal.notify_one();
            ResponseTemplate::new(200)
                .set_delay(Duration::from_secs(10))
                .set_body_json(data(&json!([[1]])))
        })
        .mount(&server)
        .await;
    let worker = {
        let f = Arc::clone(&f);
        tokio::spawn(async move {
            f.service()
                .execute_query("SELECT nationkey FROM demo.tiny.nation")
                .await
        })
    };
    tokio::time::timeout(Duration::from_secs(3), started.notified())
        .await
        .unwrap();
    worker.abort();
    assert!(worker.await.unwrap_err().is_cancelled());
    let rows = audit_rows(&f.path, 1).await;
    assert_eq!(rows[0].get::<String, _>("status"), "cancelled");
    assert_eq!(rows[0].get::<i64, _>("row_count"), 0);
}

#[tokio::test]
async fn l2_execute_sqlite_v5_audit_upgrade_preserves_rows() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::raw_sql(include_str!(
        "../../ontoforge-storage/migrations/sqlite/0005_read_audit.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO of_read_audit (at, request_id, principal, tool, condition_api_names, effective_roles, sql_sha256, row_count, elapsed_ms, def_version, status) VALUES ('old', 'old-id', 'p', 'query.pk', '[]', '[]', 'no-sql', 1, 0, 1, 'success')")
        .execute(&pool).await.unwrap();
    sqlx::raw_sql(include_str!(
        "../../ontoforge-storage/migrations/sqlite/0006_audit_truncated.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    let old: (i64, String) =
        sqlx::query_as("SELECT seq, status FROM of_read_audit WHERE request_id = 'old-id'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(old, (1, "success".to_owned()));
    sqlx::query("INSERT INTO of_read_audit (at, request_id, principal, tool, condition_api_names, effective_roles, sql_sha256, row_count, elapsed_ms, def_version, status) VALUES ('new', 'new-id', 'p', 'execute_query', '[]', '[]', 'hash', 2, 1, 2, 'truncated')")
        .execute(&pool).await.unwrap();
    let new: (i64, String) =
        sqlx::query_as("SELECT seq, status FROM of_read_audit WHERE request_id = 'new-id'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(new, (2, "truncated".to_owned()));
}

#[tokio::test]
async fn l2_execute_byte_cap_truncates_without_releasing_partial_row() {
    let server = MockServer::start().await;
    let mut f = fixture(&server).await;
    attest(&server, &f.ontology, &f.l2).await;
    f.config.max_bytes = 1;
    answer(
        &server,
        "l2",
        "SELECT nationkey FROM demo.tiny.nation",
        data(&json!([[1]])),
    )
    .await;
    let result = f
        .service()
        .execute_query("SELECT nationkey FROM demo.tiny.nation")
        .await
        .unwrap();
    assert!(result.output.truncated);
    assert!(result.output.rows.is_empty());
    assert!(result.narrowing_hint.is_some());
    let rows = audit_rows(&f.path, 1).await;
    assert_eq!(rows[0].get::<String, _>("status"), "truncated");
    assert_eq!(rows[0].get::<i64, _>("row_count"), 0);
}

#[tokio::test]
async fn l2_execute_timeout_is_audited_without_result() {
    let server = MockServer::start().await;
    let mut f = fixture(&server).await;
    attest(&server, &f.ontology, &f.l2).await;
    f.config.query_timeout_ms = 30;
    Mock::given(method("POST"))
        .and(path("/v1/statement"))
        .and(header("x-trino-user", "l2"))
        .and(body_string_contains(
            "SELECT nationkey FROM demo.tiny.nation",
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_millis(250))
                .set_body_json(data(&json!([[1]]))),
        )
        .mount(&server)
        .await;
    assert!(matches!(
        f.service()
            .execute_query("SELECT nationkey FROM demo.tiny.nation")
            .await,
        Err(L2Error::Source(
            ontoforge_engine::EngineError::Timeout { .. }
        ))
    ));
    let rows = audit_rows(&f.path, 1).await;
    assert_eq!(rows[0].get::<String, _>("status"), "source_error");
    assert_eq!(rows[0].get::<i64, _>("row_count"), 0);
}

#[tokio::test]
async fn l2_explain_requires_attestation_and_submits_only_validated_display() {
    let server = MockServer::start().await;
    let f = fixture(&server).await;
    let sql = "SELECT /* raw comment */ nationkey FROM demo.tiny.nation";
    assert!(matches!(
        f.service().explain_query(sql).await,
        Err(L2Error::Disabled)
    ));
    assert!(server.received_requests().await.unwrap().is_empty());
    attest(&server, &f.ontology, &f.l2).await;
    let before = server.received_requests().await.unwrap().len();
    answer(
        &server,
        "l2",
        "EXPLAIN (TYPE DISTRIBUTED) SELECT nationkey FROM demo.tiny.nation",
        data(&json!([[1]])),
    )
    .await;
    let result = f.service().explain_query(sql).await.unwrap();
    assert_eq!(result.output.rows.len(), 1);
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), before + 1);
    let body = String::from_utf8(requests.last().unwrap().body.clone()).unwrap();
    assert_eq!(
        body,
        "EXPLAIN (TYPE DISTRIBUTED) SELECT nationkey FROM demo.tiny.nation LIMIT 3"
    );
    let rows = audit_rows(&f.path, 2).await;
    assert_eq!(rows[0].get::<String, _>("status"), "denied");
    assert_eq!(rows[1].get::<String, _>("status"), "success");
    assert_eq!(rows[1].get::<String, _>("tool"), "explain_query");
    assert_eq!(rows[1].get::<i64, _>("row_count"), 1);
}

#[tokio::test]
async fn l2_explain_invalid_and_unauthorized_sql_never_posts() {
    let server = MockServer::start().await;
    let mut f = fixture(&server).await;
    attest(&server, &f.ontology, &f.l2).await;
    let before = server.received_requests().await.unwrap().len();
    f.scope.sql_levels.clear();
    assert!(matches!(
        f.service()
            .explain_query("SELECT 1 FROM demo.tiny.nation")
            .await,
        Err(L2Error::Denied)
    ));
    f.scope.sql_levels.insert(SqlLevel::Raw);
    assert!(matches!(
        f.service()
            .explain_query("SELECT * FROM outside.tiny.nation")
            .await,
        Err(L2Error::Denied)
    ));
    for sql in [
        "DELETE FROM demo.tiny.nation",
        "EXPLAIN ANALYZE SELECT * FROM demo.tiny.nation",
    ] {
        assert!(
            matches!(
                f.service().explain_query(sql).await,
                Err(L2Error::InvalidSql)
            ),
            "{sql}"
        );
    }
    assert_eq!(server.received_requests().await.unwrap().len(), before);
    let rows = audit_rows(&f.path, 4).await;
    assert!(
        rows.iter()
            .all(|row| row.get::<String, _>("status") == "denied")
    );
}

#[tokio::test]
async fn l2_explain_truncation_source_error_and_audit_fault() {
    let server = MockServer::start().await;
    let f = fixture(&server).await;
    attest(&server, &f.ontology, &f.l2).await;
    answer(
        &server,
        "l2",
        "EXPLAIN (TYPE DISTRIBUTED) SELECT nationkey FROM demo.tiny.nation",
        data(&json!([[1], [2], [3]])),
    )
    .await;
    let result = f
        .service()
        .explain_query("SELECT nationkey FROM demo.tiny.nation")
        .await
        .unwrap();
    assert_eq!(result.output.rows.len(), 2);
    assert!(result.output.truncated);
    assert!(result.narrowing_hint.is_some());
    let rows = audit_rows(&f.path, 1).await;
    assert_eq!(rows[0].get::<String, _>("status"), "truncated");
    f.store.read_audit().shutdown().await.unwrap();
    let before = server.received_requests().await.unwrap().len();
    assert!(matches!(
        f.service()
            .explain_query("SELECT nationkey FROM demo.tiny.nation")
            .await,
        Err(L2Error::AuditUnavailable)
    ));
    assert_eq!(server.received_requests().await.unwrap().len(), before);

    let other_server = MockServer::start().await;
    let other = fixture(&other_server).await;
    attest(&other_server, &other.ontology, &other.l2).await;
    answer(&other_server, "l2", "EXPLAIN (TYPE DISTRIBUTED) SELECT nationkey FROM demo.tiny.nation", json!({"id":"q1", "error":{"errorName":"INTERNAL_ERROR","message":"sensitive"}, "stats":{"state":"FAILED"}})).await;
    assert!(matches!(
        other
            .service()
            .explain_query("SELECT nationkey FROM demo.tiny.nation")
            .await,
        Err(L2Error::Source(_))
    ));
    let rows = audit_rows(&other.path, 1).await;
    assert_eq!(rows[0].get::<String, _>("status"), "source_error");
}

#[tokio::test]
async fn l2_explain_inflight_cancellation_records_event() {
    let server = MockServer::start().await;
    let f = Arc::new(fixture(&server).await);
    attest(&server, &f.ontology, &f.l2).await;
    let started = Arc::new(Notify::new());
    let signal = Arc::clone(&started);
    Mock::given(method("POST"))
        .and(path("/v1/statement"))
        .and(header("x-trino-user", "l2"))
        .and(body_string_contains(
            "EXPLAIN (TYPE DISTRIBUTED) SELECT nationkey FROM demo.tiny.nation",
        ))
        .respond_with(move |_: &wiremock::Request| {
            signal.notify_one();
            ResponseTemplate::new(200)
                .set_delay(Duration::from_secs(10))
                .set_body_json(data(&json!([[1]])))
        })
        .mount(&server)
        .await;
    let worker = {
        let f = Arc::clone(&f);
        tokio::spawn(async move {
            f.service()
                .explain_query("SELECT nationkey FROM demo.tiny.nation")
                .await
        })
    };
    tokio::time::timeout(Duration::from_secs(3), started.notified())
        .await
        .unwrap();
    worker.abort();
    assert!(worker.await.unwrap_err().is_cancelled());
    let rows = audit_rows(&f.path, 1).await;
    assert_eq!(rows[0].get::<String, _>("status"), "cancelled");
}

#[tokio::test]
async fn l2_explain_all_types_share_admission_and_audit() {
    let server = MockServer::start().await;
    let f = fixture(&server).await;
    attest(&server, &f.ontology, &f.l2).await;
    for (explain_type, keyword) in [
        (ExplainType::Logical, "LOGICAL"),
        (ExplainType::Distributed, "DISTRIBUTED"),
        (ExplainType::Validate, "VALIDATE"),
        (ExplainType::Io, "IO"),
    ] {
        answer(
            &server,
            "l2",
            &format!("EXPLAIN (TYPE {keyword}) SELECT nationkey FROM demo.tiny.nation"),
            data(&json!([[1]])),
        )
        .await;
        let result = f
            .service()
            .explain_query_with_type("SELECT nationkey FROM demo.tiny.nation", explain_type)
            .await
            .expect("typed explain result");
        assert_eq!(result.output.rows.len(), 1);
    }
    let rows = audit_rows(&f.path, 4).await;
    assert_eq!(rows.len(), 4);
    assert!(rows.iter().all(|row| {
        row.get::<String, _>("tool") == "explain_query"
            && row.get::<String, _>("status") == "success"
    }));
}

#[tokio::test]
async fn l2_explain_old_wrapper_and_rejection_contract() {
    let server = MockServer::start().await;
    let f = fixture(&server).await;
    attest(&server, &f.ontology, &f.l2).await;
    let before = server.received_requests().await.unwrap().len();
    for sql in [
        "DELETE FROM demo.tiny.nation",
        "EXPLAIN ANALYZE SELECT * FROM demo.tiny.nation",
        "SELECT * FROM demo.tiny.nation; SELECT * FROM demo.tiny.nation",
    ] {
        assert!(matches!(
            f.service().explain_query(sql).await,
            Err(L2Error::InvalidSql)
        ));
    }
    assert_eq!(server.received_requests().await.unwrap().len(), before);
    let rows = audit_rows(&f.path, 3).await;
    assert!(
        rows.iter()
            .all(|row| row.get::<String, _>("status") == "denied")
    );
}

#[tokio::test]
async fn l2_explain_all_types_timeout_cancel_concurrency() {
    for (explain_type, keyword) in [
        (ExplainType::Logical, "LOGICAL"),
        (ExplainType::Distributed, "DISTRIBUTED"),
        (ExplainType::Validate, "VALIDATE"),
        (ExplainType::Io, "IO"),
    ] {
        typed_explain_timeout(explain_type, keyword).await;
        typed_explain_cancel_isolated(explain_type, keyword).await;
    }
}

struct ExplainGate {
    entered: Arc<Notify>,
    release: Arc<AtomicBool>,
}

async fn explain_gate(
    server: &MockServer,
    keyword: &str,
    sql: &str,
    query_id: &str,
    delay: Duration,
) -> ExplainGate {
    let entered = Arc::new(Notify::new());
    let release = Arc::new(AtomicBool::new(false));
    let next_path = format!("/v1/statement/{query_id}/1");
    let running = json!({
        "id": query_id,
        "nextUri": format!("{}{next_path}", server.uri()),
        "stats": {"state": "RUNNING"}
    });
    Mock::given(method("POST"))
        .and(path("/v1/statement"))
        .and(header("x-trino-user", "l2"))
        .and(wiremock::matchers::body_string(format!(
            "EXPLAIN (TYPE {keyword}) {sql} LIMIT 3"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(&running))
        .expect(1)
        .mount(server)
        .await;
    let signal = Arc::clone(&entered);
    let released = Arc::clone(&release);
    let finished = json!({
        "id": query_id,
        "columns": [{"name": "nationkey", "type": "integer"}],
        "data": [[1]],
        "stats": {"state": "FINISHED"}
    });
    Mock::given(method("GET"))
        .and(path(&next_path))
        .respond_with(move |_: &wiremock::Request| {
            signal.notify_one();
            if released.load(Ordering::Acquire) {
                ResponseTemplate::new(200).set_body_json(&finished)
            } else {
                ResponseTemplate::new(200)
                    .set_delay(delay)
                    .set_body_json(&running)
            }
        })
        .mount(server)
        .await;
    Mock::given(method("DELETE"))
        .and(path(&next_path))
        .respond_with(ResponseTemplate::new(204))
        .mount(server)
        .await;
    ExplainGate { entered, release }
}

async fn typed_explain_timeout(explain_type: ExplainType, keyword: &str) {
    let server = MockServer::start().await;
    let mut f = fixture(&server).await;
    attest(&server, &f.ontology, &f.l2).await;
    server.reset().await;
    f.config.query_timeout_ms = 500;
    let f = Arc::new(f);
    let sql = "SELECT nationkey FROM demo.tiny.nation WHERE nationkey = 1";
    let gate = explain_gate(&server, keyword, sql, "timed_out", Duration::from_secs(2)).await;
    let worker = {
        let f = Arc::clone(&f);
        tokio::spawn(async move { f.service().explain_query_with_type(sql, explain_type).await })
    };
    tokio::time::timeout(Duration::from_secs(3), gate.entered.notified())
        .await
        .expect("timeout query reached upstream nextUri");
    assert!(
        matches!(
            tokio::time::timeout(Duration::from_secs(3), worker)
                .await
                .expect("bounded timeout result")
                .unwrap(),
            Err(L2Error::Source(ontoforge_engine::EngineError::Timeout {
                after_ms: 500
            }))
        ),
        "{keyword}"
    );
    f.l2.drain_cancellations().await.unwrap();
    f.store.read_audit().shutdown().await.unwrap();
    assert_explain_audits(&f, &server, &[(sql, "source_error", 0)]).await;
    assert_explain_deletes(&server, &["/v1/statement/timed_out/1"]).await;
}

async fn typed_explain_cancel_isolated(explain_type: ExplainType, keyword: &str) {
    let server = MockServer::start().await;
    let f = Arc::new(fixture(&server).await);
    attest(&server, &f.ontology, &f.l2).await;
    server.reset().await;
    let cancelled_sql = "SELECT nationkey FROM demo.tiny.nation WHERE nationkey = 2";
    let independent_sql = "SELECT nationkey FROM demo.tiny.nation WHERE nationkey = 3";
    let cancelled_gate = explain_gate(
        &server,
        keyword,
        cancelled_sql,
        "cancelled",
        Duration::from_secs(10),
    )
    .await;
    let independent_gate = explain_gate(
        &server,
        keyword,
        independent_sql,
        "independent",
        Duration::from_millis(10),
    )
    .await;
    let cancelled = {
        let f = Arc::clone(&f);
        tokio::spawn(async move {
            f.service()
                .explain_query_with_type(cancelled_sql, explain_type)
                .await
        })
    };
    tokio::time::timeout(Duration::from_secs(3), cancelled_gate.entered.notified())
        .await
        .expect("cancelled query reached upstream nextUri");
    let independent = {
        let f = Arc::clone(&f);
        tokio::spawn(async move {
            f.service()
                .explain_query_with_type(independent_sql, explain_type)
                .await
        })
    };
    tokio::time::timeout(Duration::from_secs(3), independent_gate.entered.notified())
        .await
        .expect("independent query reached upstream nextUri concurrently");
    cancelled.abort();
    assert!(cancelled.await.unwrap_err().is_cancelled());
    tokio::time::timeout(Duration::from_secs(3), f.l2.drain_cancellations())
        .await
        .unwrap()
        .unwrap();
    assert!(
        !independent.is_finished(),
        "independent query remains gated after cancellation"
    );
    assert_explain_deletes(&server, &["/v1/statement/cancelled/1"]).await;
    independent_gate.release.store(true, Ordering::Release);
    let result = tokio::time::timeout(Duration::from_secs(3), independent)
        .await
        .expect("independent query completes after release")
        .unwrap()
        .expect("independent query must survive another request cancellation");
    assert_eq!(
        result.output.rows,
        vec![vec![ontoforge_engine::Value::Long(1)]]
    );
    f.l2.drain_cancellations().await.unwrap();
    f.store.read_audit().shutdown().await.unwrap();
    assert_explain_audits(
        &f,
        &server,
        &[
            (cancelled_sql, "cancelled", 0),
            (independent_sql, "success", 1),
        ],
    )
    .await;
    assert_explain_deletes(&server, &["/v1/statement/cancelled/1"]).await;
}

async fn assert_explain_audits(f: &Fixture, server: &MockServer, expected: &[(&str, &str, i64)]) {
    let requests = server.received_requests().await.unwrap();
    let rows = audit_rows(&f.path, expected.len()).await;
    assert_eq!(
        rows.len(),
        expected.len(),
        "one audit per request after audit drain"
    );
    let mut request_ids = BTreeSet::new();
    for &(sql, status, row_count) in expected {
        let posts: Vec<_> = requests
            .iter()
            .filter(|request| {
                request.method.as_str() == "POST"
                    && String::from_utf8_lossy(&request.body).contains(sql)
            })
            .collect();
        assert_eq!(posts.len(), 1, "one POST for {sql}");
        let request_id = posts[0]
            .headers
            .get("x-trino-trace-token")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(request_ids.insert(request_id), "distinct request IDs");
        let audits: Vec<_> = rows
            .iter()
            .filter(|row| row.get::<String, _>("request_id") == request_id)
            .collect();
        assert_eq!(audits.len(), 1, "one audit for {sql}");
        assert_eq!(audits[0].get::<String, _>("status"), status);
        assert_eq!(audits[0].get::<String, _>("tool"), "explain_query");
        assert_eq!(audits[0].get::<i64, _>("row_count"), row_count);
    }
}

async fn assert_explain_deletes(server: &MockServer, expected: &[&str]) {
    let requests = server.received_requests().await.unwrap();
    let paths: Vec<_> = requests
        .iter()
        .filter(|request| request.method.as_str() == "DELETE")
        .map(|request| request.url.path())
        .collect();
    assert_eq!(paths, expected, "cancel only the abandoned upstream query");
}
