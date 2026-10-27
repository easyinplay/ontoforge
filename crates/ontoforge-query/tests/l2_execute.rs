use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use camino::{Utf8Path, Utf8PathBuf};
use ontoforge_auth::Principal;
use ontoforge_auth::authn::PrincipalId;
use ontoforge_auth::scope::PrincipalScope;
use ontoforge_engine::config::{TrinoConfig, build_trino_engines, parse_server_config};
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

fn data(rows: Value) -> Value {
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
            data(json!([[1]])),
        )
        .await;
    }
    answer(server, "l2", "\"demo\".", data(json!([[1]]))).await;
    for catalog in ["outside", "editstore"] {
        answer(server, "l2", &format!("\"{catalog}\"."), json!({"id":"q1", "error":{"errorName":"PERMISSION_DENIED","message":"denied"}, "stats":{"state":"FAILED"}})).await;
    }
    answer(server, "ontology", "other_1", json!({"id":"q1", "columns":[{"name":"query_id","type":"varchar"}], "data":[["other_1"]], "stats":{"state":"FINISHED"}})).await;
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
        data(json!([[1]])),
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
        data(json!([[1], [2], [3]])),
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
                .set_body_json(data(json!([[1]])))
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
