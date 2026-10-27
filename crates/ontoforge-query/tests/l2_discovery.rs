use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use camino::{Utf8Path, Utf8PathBuf};
use ontoforge_auth::Principal;
use ontoforge_auth::authn::PrincipalId;
use ontoforge_auth::scope::PrincipalScope;
use ontoforge_engine::Value;
use ontoforge_engine::config::{TrinoConfig, build_trino_engines, parse_server_config};
use ontoforge_engine::trino::TrinoEngine;
use ontoforge_engine::trino::l2_health::L2HealthStatus;
use ontoforge_expr::Attrs;
use ontoforge_query::l2::{L2Error, L2Service};
use ontoforge_schema::ir::SqlLevel;
use ontoforge_schema::load_dir;
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::store::{ActiveDefinition, BackendSettings, EditStore, StoreSettings};
use serde_json::{Value as Json, json};
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
        "[trino]\nurl = \"{url}\"\nmax_rows = 8\nmax_bytes = 4096\n[trino.identities.ontology]\nuser = \"ontology\"\n[trino.identities.l2]\nuser = \"l2\"\n[trino.l2_acl.allowed]\ncatalog = \"demo\"\nschema = \"tiny\"\ntable = \"nation\"\n[trino.l2_acl.outside]\ncatalog = \"outside\"\nschema = \"tiny\"\ntable = \"nation\"\n[trino.l2_acl.edit]\ncatalog = \"editstore\"\nschema = \"tiny\"\ntable = \"nation\"\n"
    ))
    .unwrap().trino.unwrap()
}

fn data(rows: &Json) -> Json {
    json!({"id":"q1", "columns":[{"name":"name", "type":"varchar"}], "data":rows, "stats":{"state":"FINISHED"}})
}

async fn answer(server: &MockServer, user: &str, needle: &str, response: Json) {
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
        answer(server, "l2", &format!("\"{catalog}\"."), json!({"id":"q1", "error":{"errorName":"PERMISSION_DENIED", "message":"denied"}, "stats":{"state":"FAILED"}})).await;
    }
    answer(server, "ontology", "other_1", json!({"id":"q1", "columns":[{"name":"query_id", "type":"varchar"}], "data":[["other_1"]], "stats":{"state":"FINISHED"}})).await;
    answer(server, "l2", "other_1", json!({"id":"q1", "columns":[{"name":"query_id", "type":"varchar"}], "data":[], "stats":{"state":"FINISHED"}})).await;
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
            id: PrincipalId::new("test", "reader"),
            issuer: "test".into(),
            subject: "reader".into(),
            groups: BTreeSet::new(),
            attrs: Attrs::default(),
        },
        scope: PrincipalScope {
            sql_levels: BTreeSet::from([SqlLevel::Raw]),
            l2_catalogs: BTreeSet::new(),
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
async fn l2_catalog_filters_and_audits_catalogs_and_schemas() {
    let server = MockServer::start().await;
    let f = fixture(&server).await;
    assert!(matches!(
        f.service().list_catalogs().await,
        Err(L2Error::Disabled)
    ));
    assert!(server.received_requests().await.unwrap().is_empty());
    attest(&server, &f.ontology, &f.l2).await;
    answer(
        &server,
        "l2",
        "SHOW CATALOGS",
        data(&json!([["system"], ["editstore"], ["demo"], ["outside"]])),
    )
    .await;
    answer(
        &server,
        "l2",
        "SHOW SCHEMAS FROM \"demo\"",
        data(&json!([["information_schema"], ["other"], ["tiny"]])),
    )
    .await;
    let before = server.received_requests().await.unwrap().len();
    let catalogs = f.service().list_catalogs().await.unwrap();
    assert_eq!(
        catalogs.output.rows,
        vec![vec![Value::String("demo".to_owned())]]
    );
    let schemas = f.service().list_schemas("demo").await.unwrap();
    assert_eq!(
        schemas.output.rows,
        vec![vec![Value::String("tiny".to_owned())]]
    );
    assert_eq!(server.received_requests().await.unwrap().len(), before + 2);
    let audits = audit_rows(&f.path, 3).await;
    assert_eq!(
        audits
            .iter()
            .map(|r| r.get::<String, _>("tool"))
            .collect::<Vec<_>>(),
        ["list_catalogs", "list_catalogs", "list_schemas"]
    );
    assert_eq!(audits[0].get::<String, _>("status"), "denied");
    assert!(
        audits
            .iter()
            .skip(1)
            .all(|r| r.get::<String, _>("status") == "success")
    );
}

#[tokio::test]
async fn l2_catalog_rejects_ungranted_input_without_post() {
    let server = MockServer::start().await;
    let mut f = fixture(&server).await;
    attest(&server, &f.ontology, &f.l2).await;
    let before = server.received_requests().await.unwrap().len();
    for catalog in ["outside", "system", "editstore", "demo\"; DROP TABLE x; --"] {
        assert!(
            matches!(
                f.service().list_schemas(catalog).await,
                Err(L2Error::Denied)
            ),
            "{catalog}"
        );
    }
    f.scope.sql_levels.clear();
    assert!(matches!(
        f.service().list_catalogs().await,
        Err(L2Error::Denied)
    ));
    assert_eq!(server.received_requests().await.unwrap().len(), before);
    let audits = audit_rows(&f.path, 5).await;
    assert!(
        audits
            .iter()
            .all(|r| r.get::<String, _>("status") == "denied")
    );
}

#[tokio::test]
async fn l2_catalog_truncation_source_error_and_audit_failure() {
    let server = MockServer::start().await;
    let mut f = fixture(&server).await;
    attest(&server, &f.ontology, &f.l2).await;
    f.config.max_rows = 1;
    f.scope.l2_catalogs.insert("other".to_owned());
    answer(
        &server,
        "l2",
        "SHOW CATALOGS",
        data(&json!([["demo"], ["other"], ["outside"]])),
    )
    .await;
    let page = f.service().list_catalogs().await.unwrap();
    assert_eq!(
        page.output.rows,
        vec![vec![Value::String("demo".to_owned())]]
    );
    assert!(page.output.truncated);
    assert!(page.narrowing_hint.is_some());
    assert_eq!(
        audit_rows(&f.path, 1).await[0].get::<String, _>("status"),
        "truncated"
    );

    answer(&server, "l2", "SHOW SCHEMAS FROM \"demo\"", json!({"id":"q1", "error":{"errorName":"INTERNAL_ERROR", "message":"source failed"}, "stats":{"state":"FAILED"}})).await;
    assert!(matches!(
        f.service().list_schemas("demo").await,
        Err(L2Error::Source(_))
    ));
    assert_eq!(
        audit_rows(&f.path, 2).await[1].get::<String, _>("status"),
        "source_error"
    );
    let before = server.received_requests().await.unwrap().len();
    f.store.read_audit().shutdown().await.unwrap();
    assert!(matches!(
        f.service().list_catalogs().await,
        Err(L2Error::AuditUnavailable)
    ));
    assert_eq!(server.received_requests().await.unwrap().len(), before);
}

#[tokio::test]
async fn l2_catalog_cancelled_inflight_audits_once() {
    let server = MockServer::start().await;
    let f = Arc::new(fixture(&server).await);
    attest(&server, &f.ontology, &f.l2).await;
    let started = Arc::new(Notify::new());
    let signal = Arc::clone(&started);
    Mock::given(method("POST"))
        .and(path("/v1/statement"))
        .and(header("x-trino-user", "l2"))
        .and(body_string_contains("SHOW CATALOGS"))
        .respond_with(move |_: &wiremock::Request| {
            signal.notify_one();
            ResponseTemplate::new(200)
                .set_delay(Duration::from_secs(10))
                .set_body_json(data(&json!([["demo"]])))
        })
        .mount(&server)
        .await;
    let worker = {
        let f = Arc::clone(&f);
        tokio::spawn(async move { f.service().list_catalogs().await })
    };
    tokio::time::timeout(Duration::from_secs(3), started.notified())
        .await
        .unwrap();
    worker.abort();
    assert!(worker.await.unwrap_err().is_cancelled());
    let rows = audit_rows(&f.path, 1).await;
    assert_eq!(rows[0].get::<String, _>("status"), "cancelled");
    assert_eq!(rows[0].get::<String, _>("tool"), "list_catalogs");
}
