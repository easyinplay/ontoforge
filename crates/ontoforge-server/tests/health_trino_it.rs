#[path = "support/real.rs"]
mod support;

#[path = "support/mod.rs"]
mod fixtures;

use camino::Utf8Path;
use ontoforge_engine::config::{build_trino_engines, parse_server_config};
use ontoforge_engine::trino::TrinoEngine;
use ontoforge_engine::trino::l2_health::{L2DisableReason, L2HealthStatus};
use ontoforge_engine::{
    Engine, EngineError, EngineStatus, PhysicalColumn, PrintedSql, QueryContext, QueryOutput,
    TableRef,
};
use ontoforge_server::ServerHost;
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::Notify;
use tokio::sync::oneshot;

struct DelayedActionTrino {
    inner: TrinoEngine,
    blocked: AtomicBool,
    entered: Notify,
    release: Notify,
}

struct CoreProbe {
    inner: Arc<TrinoEngine>,
    disconnected: Arc<TrinoEngine>,
    fail: AtomicBool,
}

impl Engine for CoreProbe {
    fn dialect(&self) -> ontoforge_engine::Dialect {
        self.inner.dialect()
    }
    async fn execute(
        &self,
        sql: &PrintedSql,
        ctx: &QueryContext,
    ) -> Result<QueryOutput, EngineError> {
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
        if self.fail.load(Ordering::SeqCst) {
            self.disconnected.status().await
        } else {
            self.inner.status().await
        }
    }
}

impl Engine for DelayedActionTrino {
    fn dialect(&self) -> ontoforge_engine::Dialect {
        self.inner.dialect()
    }
    async fn execute(
        &self,
        sql: &PrintedSql,
        ctx: &QueryContext,
    ) -> Result<QueryOutput, EngineError> {
        if self.blocked.load(Ordering::SeqCst) {
            self.entered.notify_one();
            self.release.notified().await;
        }
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

fn acl_engines(url: &str) -> (Arc<TrinoEngine>, Arc<TrinoEngine>) {
    let text = format!(
        "[trino]\nurl = \"{url}\"\n[trino.identities.ontology]\nuser = \"ontology\"\n[trino.identities.l2]\nuser = \"l2\"\n[trino.l2_acl.allowed]\ncatalog = \"demo\"\nschema = \"tiny\"\ntable = \"nation\"\n[trino.l2_acl.outside]\ncatalog = \"outside\"\nschema = \"tiny\"\ntable = \"nation\"\n[trino.l2_acl.edit]\ncatalog = \"editstore\"\nschema = \"tiny\"\ntable = \"nation\"\n"
    );
    let config = parse_server_config(&text)
        .expect("ACL config")
        .trino
        .expect("Trino config");
    let engines = build_trino_engines(&config, Utf8Path::new("."), &|_: &str| None)
        .expect("real ACL engines");
    (
        Arc::new(engines.ontology),
        Arc::new(engines.l2.expect("L2 engine")),
    )
}

async fn active_query(url: &str) -> (String, String) {
    let client = support::client();
    let mut page: Value = client
        .post(format!("{url}/v1/statement"))
        .header("X-Trino-User", "ontology")
        .body("SELECT count(*) FROM demo.sf100000.orders")
        .send()
        .await
        .expect("active probe POST")
        .json()
        .await
        .expect("active probe JSON");
    let id = page["id"].as_str().expect("probe query id").to_owned();
    for _ in 0..30 {
        assert!(page.get("error").is_none(), "{page}");
        if page["stats"]["state"] == "RUNNING" {
            return (
                id,
                page["nextUri"]
                    .as_str()
                    .expect("active query URI")
                    .to_owned(),
            );
        }
        let next = page["nextUri"]
            .as_str()
            .expect("probe remains active")
            .to_owned();
        page = client
            .get(next)
            .send()
            .await
            .expect("probe next page")
            .json()
            .await
            .expect("probe page JSON");
    }
    panic!("probe did not become RUNNING: {page}");
}

async fn health_response(url: &str) -> (reqwest::StatusCode, Value) {
    support::json(
        support::client()
            .get(format!("{url}/health"))
            .send()
            .await
            .expect("health"),
    )
    .await
}

#[tokio::test]
#[ignore = "requires real Trino L2 ACL services and an authenticated ontoforge HTTP server"]
async fn real_health_l2_failure_expiry_recovery_keeps_core() {
    let url = support::required("ONTOFORGE_TRINO_L2_URL");
    let loose = support::required("ONTOFORGE_TRINO_L2_LOOSE_URL");
    let (ontology, l2) = acl_engines(&url);
    let (disconnected, _) = acl_engines("http://127.0.0.1:0");
    let core = Arc::new(CoreProbe {
        inner: Arc::clone(&ontology),
        disconnected,
        fail: AtomicBool::new(false),
    });
    let fixture = fixtures::fixture().await;
    let status_engine = Arc::clone(&l2);
    let host = ServerHost {
        store: Arc::clone(&fixture.host.store),
        engine: Arc::clone(&core),
        auth: fixture.host.auth,
        auth_config: fixture.host.auth_config,
        tokens: fixture.host.tokens,
    }
    .with_l2_status_provider(move || status_engine.l2_status());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let server = format!("http://{}", listener.local_addr().expect("address"));
    let (stop, rx) = oneshot::channel();
    let serving = tokio::spawn(Arc::new(host).serve(listener, async {
        let _ = rx.await;
    }));

    let (status, initial) = health_response(&server).await;
    assert_eq!(status, reqwest::StatusCode::OK);
    assert_eq!(initial["status"], "degraded");
    assert_eq!(initial["l2"]["reason"], "NOT_VERIFIED");

    let (id, cancel) = active_query(&url).await;
    assert_eq!(
        l2.recheck_l2_acl(&ontology, &id).await,
        L2HealthStatus::Enabled
    );
    support::client()
        .delete(cancel)
        .send()
        .await
        .expect("cancel active probe");
    let (status, enabled) = health_response(&server).await;
    assert_eq!(status, reqwest::StatusCode::OK);
    assert_eq!(enabled["status"], "healthy");
    assert_eq!(enabled["l2"]["status"], "enabled");

    core.fail.store(true, Ordering::SeqCst);
    let (status, disconnected) = health_response(&server).await;
    assert_eq!(status, reqwest::StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(disconnected["status"], "unhealthy");
    assert_eq!(disconnected["engineStatus"], "unavailable");
    core.fail.store(false, Ordering::SeqCst);
    assert_eq!(health_response(&server).await.1["status"], "healthy");

    tokio::time::sleep(Duration::from_secs(61)).await;
    let (status, expired) = health_response(&server).await;
    assert_eq!(status, reqwest::StatusCode::OK);
    assert_eq!(expired["status"], "degraded");
    assert_eq!(expired["l2"]["reason"], "EXPIRED");
    assert_eq!(expired["engineStatus"], "available");

    let (id, cancel) = active_query(&url).await;
    assert_eq!(
        l2.recheck_l2_acl(&ontology, &id).await,
        L2HealthStatus::Enabled
    );
    support::client()
        .delete(cancel)
        .send()
        .await
        .expect("cancel recovery probe");
    assert_eq!(health_response(&server).await.1["status"], "healthy");

    let (loose_ontology, loose_l2) = acl_engines(&loose);
    assert_eq!(
        loose_l2.recheck_l2_acl(&loose_ontology, "dummy_1").await,
        L2HealthStatus::Disabled {
            reason: L2DisableReason::NegativeProbeFailed
        }
    );
    stop.send(()).expect("stop health host");
    tokio::time::timeout(Duration::from_secs(10), serving)
        .await
        .expect("host shutdown")
        .expect("serve task")
        .expect("clean serve shutdown");
}

#[tokio::test]
#[ignore = "requires real Trino L2 ACL services and an authenticated ontoforge HTTP server"]
async fn real_shutdown_action_dependency_audit_and_cleanup() {
    let trino = support::required("ONTOFORGE_TRINO_URL");
    let fixture = fixtures::fixture().await;
    let config = parse_server_config(&format!(
        "[trino]\nurl = \"{trino}\"\n[trino.identities.ontology]\nuser = \"ontology\"\n"
    ))
    .expect("Trino config")
    .trino
    .expect("Trino");
    let engines = build_trino_engines(&config, Utf8Path::new("."), &|_: &str| None)
        .expect("real Trino engine");
    let engine = Arc::new(DelayedActionTrino {
        inner: engines.ontology,
        blocked: AtomicBool::new(true),
        entered: Notify::new(),
        release: Notify::new(),
    });
    let store = Arc::clone(&fixture.store);
    let host = ServerHost {
        store: Arc::clone(&store),
        engine: Arc::clone(&engine),
        auth: fixture.host.auth,
        auth_config: fixture.host.auth_config,
        tokens: fixture.host.tokens,
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let server = format!("http://{}", listener.local_addr().expect("address"));
    let (stop, rx) = oneshot::channel();
    let task = tokio::spawn(Arc::new(host).serve(listener, async {
        let _ = rx.await;
    }));
    let bearer = fixture.token.clone();
    let action_server = server.clone();
    let request = tokio::spawn(async move {
        support::post_json(
            &action_server,
            "/api/actions/delayFlight",
            &bearer,
            &serde_json::json!({
                "commandId": format!("shutdown-{}", ulid::Ulid::generate()),
                "items": [{"params": {"flight": "F-1001", "delayMinutes": 5}}]
            }),
        )
        .await
    });
    tokio::time::timeout(Duration::from_secs(5), engine.entered.notified())
        .await
        .expect("Action accepted and waiting on real source dependency");
    stop.send(()).expect("request graceful shutdown");
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        !task.is_finished(),
        "shutdown must wait for accepted Action source dependency"
    );
    engine.blocked.store(false, Ordering::SeqCst);
    engine.release.notify_one();
    let response = tokio::time::timeout(Duration::from_secs(30), request)
        .await
        .expect("accepted Action deadline")
        .expect("Action task");
    let (status, receipt) = support::json(response).await;
    assert_eq!(status, reqwest::StatusCode::OK, "{receipt}");
    tokio::time::timeout(Duration::from_secs(10), task)
        .await
        .expect("shutdown after Action")
        .expect("server task")
        .expect("audit cleanup");
    assert!(
        !store.read_audit().health().available,
        "read audit must be durably stopped"
    );
    let pool = sqlx::SqlitePool::connect_with(ontoforge_storage::store::sqlite_connect_options(
        &fixture.path,
    ))
    .await
    .expect("audit pool");
    let commits: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM of_edit_log WHERE action = 'delayFlight'")
            .fetch_one(&pool)
            .await
            .expect("durable Action commits");
    assert_eq!(
        commits, 1,
        "accepted Action must commit exactly once before shutdown"
    );
    pool.close().await;
}
