#[path = "support/mod.rs"]
mod support;

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ontoforge_engine::{
    Engine, EngineError, EngineStatus, PhysicalColumn, PrintedSql, QueryContext, QueryOutput,
    TableRef,
};
use ontoforge_server::{HostError, ServerHost};
use reqwest::StatusCode;
use serde_json::{Value, json};
use sqlx::Row;
use tokio::sync::{Notify, Semaphore, oneshot};

struct DriftEngine {
    inner: Arc<support::ControlledEngine>,
    metadata_calls: AtomicUsize,
    metadata_error: Option<EngineError>,
    block_metadata: AtomicBool,
    metadata_entered: Notify,
    metadata_release: Semaphore,
    active_metadata: AtomicUsize,
    use_real_metadata: AtomicBool,
    metadata_budgets: Mutex<Vec<Duration>>,
    target_checks: AtomicUsize,
    reject_after: AtomicUsize,
    deny_links: AtomicBool,
    expand_first_page: AtomicBool,
    panic_metadata: AtomicBool,
}

struct ActiveProbe<'a>(&'a AtomicUsize);
impl Drop for ActiveProbe<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

impl Engine for DriftEngine {
    fn dialect(&self) -> ontoforge_engine::Dialect {
        self.inner.dialect()
    }
    fn check_source_target(
        &self,
        target: &ontoforge_schema::ir::BindingTarget,
    ) -> Result<(), ontoforge_engine::SourceTargetError> {
        let count = self.target_checks.fetch_add(1, Ordering::SeqCst);
        if count >= self.reject_after.load(Ordering::SeqCst)
            || (self.deny_links.load(Ordering::SeqCst)
                && matches!(target, ontoforge_schema::ir::BindingTarget::LinkType(_)))
        {
            Err(ontoforge_engine::SourceTargetError::SchemaDrift)
        } else {
            Ok(())
        }
    }
    async fn execute(
        &self,
        sql: &PrintedSql,
        ctx: &QueryContext,
    ) -> Result<QueryOutput, EngineError> {
        let mut output = self.inner.execute(sql, ctx).await?;
        if self.expand_first_page.load(Ordering::SeqCst) && !output.rows.is_empty() {
            let pk = output
                .columns
                .iter()
                .position(|column| column.name == "flightId")
                .unwrap();
            let first = output.rows[0].clone();
            output.rows = (0..256)
                .map(|index| {
                    let mut row = first.clone();
                    row[pk] = ontoforge_engine::Value::String(format!("X-{index:04}"));
                    row
                })
                .collect();
        }
        Ok(output)
    }
    async fn describe_tables(
        &self,
        tables: &[TableRef],
        ctx: &QueryContext,
    ) -> Result<BTreeMap<TableRef, Vec<PhysicalColumn>>, EngineError> {
        self.metadata_calls.fetch_add(1, Ordering::SeqCst);
        assert!(
            !self.panic_metadata.load(Ordering::SeqCst),
            "controlled metadata panic"
        );
        self.metadata_budgets.lock().unwrap().push(ctx.timeout);
        self.active_metadata.fetch_add(1, Ordering::SeqCst);
        let _active = ActiveProbe(&self.active_metadata);
        self.metadata_entered.notify_one();
        if self.block_metadata.load(Ordering::SeqCst) {
            self.metadata_release.acquire().await.unwrap().forget();
        }
        if let Some(error) = &self.metadata_error {
            return Err(error.clone());
        }
        if self.use_real_metadata.load(Ordering::SeqCst) {
            return self.inner.describe_tables(tables, ctx).await;
        }
        Ok(BTreeMap::new())
    }
    fn status(&self) -> impl Future<Output = EngineStatus> + Send {
        std::future::ready(EngineStatus::Available)
    }
}

async fn start(
    host: ServerHost<DriftEngine>,
) -> (
    String,
    oneshot::Sender<()>,
    tokio::task::JoinHandle<Result<(), HostError>>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (stop, rx) = oneshot::channel();
    let task = tokio::spawn(Arc::new(host).serve(listener, async {
        let _ = rx.await;
    }));
    (url, stop, task)
}

fn host(
    original: ServerHost<support::ControlledEngine>,
    metadata_error: Option<EngineError>,
) -> ServerHost<DriftEngine> {
    ServerHost {
        store: original.store,
        engine: Arc::new(DriftEngine {
            inner: original.engine,
            metadata_calls: AtomicUsize::new(0),
            metadata_error,
            block_metadata: AtomicBool::new(false),
            metadata_entered: Notify::new(),
            metadata_release: Semaphore::new(0),
            active_metadata: AtomicUsize::new(0),
            use_real_metadata: AtomicBool::new(false),
            metadata_budgets: Mutex::new(Vec::new()),
            target_checks: AtomicUsize::new(0),
            reject_after: AtomicUsize::new(usize::MAX),
            deny_links: AtomicBool::new(false),
            expand_first_page: AtomicBool::new(false),
            panic_metadata: AtomicBool::new(false),
        }),
        auth: original.auth,
        auth_config: original.auth_config,
        tokens: original.tokens,
    }
}

#[tokio::test]
async fn startup_drift_rejects_source_before_sql_and_preserves_audit_id() {
    let fixture = support::fixture().await;
    let drift_host = host(fixture.host, None);
    let engine = Arc::clone(&drift_host.engine);
    let (url, stop, task) = start(drift_host).await;
    let response = reqwest::Client::new()
        .get(format!("{url}/api/objects/Flight/F-1001"))
        .bearer_auth(&fixture.token)
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .unwrap();
    let status = response.status();
    let request_id = response.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let body: Value = response.json().await.unwrap();
    stop.send(()).unwrap();
    task.await.unwrap().unwrap();
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
    assert_eq!(body["code"], "SOURCE_SCHEMA_DRIFT");
    assert_eq!(body["request_id"], request_id);
    assert!(engine.metadata_calls.load(Ordering::SeqCst) > 0);
    assert_eq!(engine.inner.calls.load(Ordering::SeqCst), 0);
    let pool = sqlx::SqlitePool::connect_with(ontoforge_storage::store::sqlite_connect_options(
        &fixture.path,
    ))
    .await
    .unwrap();
    let rows = sqlx::query("SELECT request_id, status FROM of_read_audit")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get::<String, _>("request_id"), request_id);
    assert_eq!(rows[0].get::<String, _>("status"), "source_error");
    pool.close().await;
}

#[tokio::test]
async fn all_targets_degraded_keeps_health_200_without_details() {
    let fixture = support::fixture().await;
    let (url, stop, task) = start(host(fixture.host, None)).await;
    let response = reqwest::get(format!("{url}/health")).await.unwrap();
    let status = response.status();
    let body: Value = response.json().await.unwrap();
    stop.send(()).unwrap();
    task.await.unwrap().unwrap();
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        json!({"status":"degraded", "engineStatus":"available"})
    );
}

#[tokio::test]
async fn metadata_engine_failure_remains_503_despite_live_status_success() {
    let fixture = support::fixture().await;
    let (url, stop, task) = start(host(
        fixture.host,
        Some(EngineError::Unauthorized {
            reason: "secret catalog schema table".to_owned(),
        }),
    ))
    .await;
    let response = reqwest::get(format!("{url}/health")).await.unwrap();
    let status = response.status();
    let body: Value = response.json().await.unwrap();
    stop.send(()).unwrap();
    task.await.unwrap().unwrap();
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["status"], "unhealthy");
    assert_eq!(body["engineStatus"], "available");
    assert!(!body.to_string().contains("secret"));
}

#[tokio::test]
async fn source_drift_blocks_list_aggregate_and_both_action_modes_without_writes() {
    let fixture = support::fixture().await;
    let drift_host = host(fixture.host, None);
    let engine = Arc::clone(&drift_host.engine);
    let (url, stop, task) = start(drift_host).await;
    let client = reqwest::Client::new();
    let requests = [
        (
            "/api/query",
            json!({"set":{"kind":"type","object_type":"Flight"},"select":["flightId"],"page":{"first":10}}),
        ),
        (
            "/api/aggregate",
            json!({"set":{"kind":"type","object_type":"Flight"},"measures":[{"op":"count"}]}),
        ),
        (
            "/api/actions/delayFlight",
            json!({"items":[{"params":{"flight":"F-1001","delayMinutes":15}}],"validateOnly":true}),
        ),
        (
            "/api/actions/delayFlight",
            json!({"commandId":"drift-blocked","items":[{"params":{"flight":"F-1001","delayMinutes":15}}]}),
        ),
    ];
    let mut results = Vec::new();
    for (path, body) in requests {
        let response = client
            .post(format!("{url}{path}"))
            .bearer_auth(&fixture.token)
            .json(&body)
            .send()
            .await
            .unwrap();
        results.push((
            path,
            response.status(),
            response.json::<Value>().await.unwrap(),
        ));
    }
    let edit_only = client
        .get(format!("{url}/api/objects/Airport/BER"))
        .bearer_auth(&fixture.token)
        .send()
        .await
        .unwrap()
        .status();
    stop.send(()).unwrap();
    task.await.unwrap().unwrap();
    for (path, status, body) in results {
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{path}: {body}");
        assert_eq!(body["code"], "SOURCE_SCHEMA_DRIFT", "{path}: {body}");
    }
    assert_eq!(edit_only, StatusCode::OK);
    assert_eq!(engine.inner.calls.load(Ordering::SeqCst), 0);
    let pool = sqlx::SqlitePool::connect_with(ontoforge_storage::store::sqlite_connect_options(
        &fixture.path,
    ))
    .await
    .unwrap();
    let commits: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM of_edit_log")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        commits, 1,
        "only the initial Airport fixture commit may exist"
    );
    pool.close().await;
}

#[tokio::test]
async fn shutdown_during_initial_probe_cancels_and_joins_owner() {
    let fixture = support::fixture().await;
    let drift_host = host(fixture.host, None);
    let engine = Arc::clone(&drift_host.engine);
    engine.block_metadata.store(true, Ordering::SeqCst);
    let (_url, stop, task) = start(drift_host).await;
    tokio::time::timeout(Duration::from_secs(3), engine.metadata_entered.notified())
        .await
        .unwrap();
    assert_eq!(engine.active_metadata.load(Ordering::SeqCst), 1);
    stop.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(10), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(engine.active_metadata.load(Ordering::SeqCst), 0);
    assert!(!fixture.store.read_audit().health().available);
}

#[tokio::test]
async fn initial_snapshot_failure_stops_host_and_drains_audit() {
    let fixture = support::fixture().await;
    let pool = sqlx::SqlitePool::connect_with(ontoforge_storage::store::sqlite_connect_options(
        &fixture.path,
    ))
    .await
    .unwrap();
    sqlx::query("DROP TABLE of_def_state")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
    let (_url, _stop, task) = start(host(fixture.host, None)).await;
    let result = tokio::time::timeout(Duration::from_secs(10), task)
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(result, Err(HostError::SourceMonitor)));
    assert!(!fixture.store.read_audit().health().available);
}

#[tokio::test]
async fn source_unreadable_preserves_safe_code_and_zero_source_execution() {
    let fixture = support::fixture().await;
    let drift_host = host(
        fixture.host,
        Some(EngineError::Query {
            error_name: "PERMISSION_DENIED".to_owned(),
            message: "secret catalog table".to_owned(),
        }),
    );
    let engine = Arc::clone(&drift_host.engine);
    let (url, stop, task) = start(drift_host).await;
    let response = reqwest::Client::new()
        .get(format!("{url}/api/objects/Flight/F-1001"))
        .bearer_auth(&fixture.token)
        .send()
        .await
        .unwrap();
    let status = response.status();
    let body: Value = response.json().await.unwrap();
    stop.send(()).unwrap();
    task.await.unwrap().unwrap();
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["code"], "SOURCE_UNREADABLE");
    assert!(!body.to_string().contains("secret"));
    assert_eq!(engine.inner.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn complete_source_patch_remains_readable_and_actionable_during_drift() {
    use ontoforge_storage::commit::{Actor, Commit, Edit, ObjectOp};
    use ontoforge_storage::value::PropertyValue;
    let fixture = support::fixture().await;
    let active = fixture.store.active_definition().await.unwrap();
    let type_name = "Flight".to_owned().try_into().unwrap();
    let pk = PropertyValue::String("F-1001".to_owned());
    let row = ontoforge_query::read_pk_source(
        &active,
        &fixture.engine.inner,
        &type_name,
        &pk,
        3,
        1024 * 1024,
    )
    .await
    .unwrap()
    .unwrap();
    let ty = &active.ontology.object_types[&type_name];
    let properties = row
        .into_iter()
        .filter(|(name, _)| name != &ty.primary_key)
        .map(|(name, mut value)| {
            if ty.properties[&name].base_type == ontoforge_schema::ir::BaseType::Timestamp {
                value = json!("2026-03-29T02:30:00Z");
            }
            if name.as_str() == "tags" {
                value = json!([]);
            }
            let property = PropertyValue::from_json(&ty.properties[&name], &value).unwrap();
            (name, property)
        })
        .collect();
    fixture
        .store
        .commit(Commit {
            actor: Actor::Principal("ontoforge-dev|alice".to_owned()),
            action: "full-patch".to_owned(),
            def_version: active.def_version,
            params: json!({}),
            edits: vec![Edit::Object {
                type_name,
                pk,
                op: ObjectOp::Set(properties),
            }],
        })
        .await
        .unwrap();
    let drift_host = host(fixture.host, None);
    let engine = Arc::clone(&drift_host.engine);
    let (url, stop, task) = start(drift_host).await;
    let client = reqwest::Client::new();
    let read = client
        .get(format!("{url}/api/objects/Flight/F-1001"))
        .bearer_auth(&fixture.token)
        .send()
        .await
        .unwrap();
    let read_status = read.status();
    let read_body: Value = read.json().await.unwrap();
    let action = client.post(format!("{url}/api/actions/delayFlight")).bearer_auth(&fixture.token)
        .json(&json!({"commandId":"full-patch-drift","items":[{"params":{"flight":"F-1001","delayMinutes":15}}]})).send().await.unwrap();
    let action_status = action.status();
    let action_body: Value = action.json().await.unwrap();
    stop.send(()).unwrap();
    task.await.unwrap().unwrap();
    assert_eq!(read_status, StatusCode::OK, "{read_body}");
    assert_eq!(action_status, StatusCode::OK, "{action_body}");
    assert_eq!(engine.inner.calls.load(Ordering::SeqCst), 0);
}

async fn settle_until(mut ready: impl FnMut() -> bool) {
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while !ready() {
        assert!(
            std::time::Instant::now() < deadline,
            "controlled task did not reach its barrier"
        );
        tokio::task::yield_now().await;
        // SQLite runs on a worker thread; keep the paused clock from auto-advancing while it answers.
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[tokio::test]
async fn first_probe_duration_does_not_extend_the_30_second_period() {
    let fixture = support::fixture().await;
    let drift_host = host(fixture.host, None);
    let engine = Arc::clone(&drift_host.engine);
    engine.block_metadata.store(true, Ordering::SeqCst);
    let (_url, stop, task) = start(drift_host).await;
    tokio::time::timeout(Duration::from_secs(3), engine.metadata_entered.notified())
        .await
        .unwrap();
    tokio::time::pause();
    tokio::time::advance(Duration::from_secs(15)).await;
    engine.block_metadata.store(false, Ordering::SeqCst);
    engine.metadata_release.add_permits(1);
    settle_until(|| engine.active_metadata.load(Ordering::SeqCst) == 0).await;
    tokio::time::advance(Duration::from_secs(14)).await;
    assert_eq!(engine.metadata_calls.load(Ordering::SeqCst), 1);
    tokio::time::advance(Duration::from_millis(1100)).await;
    settle_until(|| engine.metadata_calls.load(Ordering::SeqCst) == 2).await;
    tokio::time::resume();
    stop.send(()).unwrap();
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn catalog_probes_share_the_round_budget_and_cancel_inflight_metadata() {
    let fixture = support::fixture().await;
    let mut ontology = fixture
        .store
        .active_definition()
        .await
        .unwrap()
        .ontology
        .clone();
    let profile = ontology.default_profile.clone().unwrap();
    ontology
        .bindings
        .get_mut(&profile)
        .unwrap()
        .objects
        .get_mut("Pilot")
        .unwrap()
        .source
        .catalog = "second".to_owned();
    ontoforge_storage::reload::reload(
        &fixture.store,
        &ontology,
        &ontoforge_storage::reload::ReloadOptions::default(),
    )
    .await
    .unwrap();
    let drift_host = host(fixture.host, None);
    let engine = Arc::clone(&drift_host.engine);
    engine.block_metadata.store(true, Ordering::SeqCst);
    let (url, stop, task) = start(drift_host).await;
    tokio::time::timeout(Duration::from_secs(3), engine.metadata_entered.notified())
        .await
        .unwrap();
    tokio::time::pause();
    tokio::time::advance(Duration::from_secs(12)).await;
    engine.metadata_release.add_permits(1);
    settle_until(|| engine.metadata_calls.load(Ordering::SeqCst) == 2).await;
    assert!(engine.metadata_budgets.lock().unwrap()[1] <= Duration::from_secs(8));
    assert_eq!(engine.active_metadata.load(Ordering::SeqCst), 1);
    tokio::time::advance(Duration::from_millis(8100)).await;
    settle_until(|| engine.active_metadata.load(Ordering::SeqCst) == 0).await;
    tokio::time::resume();
    let health = reqwest::get(format!("{url}/health")).await.unwrap();
    let health_status = health.status();
    let health_body: Value = health.json().await.unwrap();
    let rejected = reqwest::Client::new()
        .get(format!("{url}/api/objects/Flight/F-1001"))
        .bearer_auth(&fixture.token)
        .send()
        .await
        .unwrap();
    let rejected_status = rejected.status();
    let rejected_body: Value = rejected.json().await.unwrap();
    stop.send(()).unwrap();
    task.await.unwrap().unwrap();
    assert_eq!(health_status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        health_body,
        json!({"status":"unhealthy", "engineStatus":"available"})
    );
    assert_eq!(rejected_status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(rejected_body["code"], "SOURCE_CHECK_UNAVAILABLE");
}

#[tokio::test]
async fn later_list_page_and_backfill_recheck_the_logical_target() {
    use ontoforge_storage::commit::{Actor, Commit, Edit, ObjectOp};
    use ontoforge_storage::value::PropertyValue;
    for backfill in [false, true] {
        let fixture = support::fixture().await;
        if backfill {
            let active = fixture.store.active_definition().await.unwrap();
            fixture
                .store
                .commit(Commit {
                    actor: Actor::Principal("ontoforge-dev|alice".to_owned()),
                    action: "partial-patch".to_owned(),
                    def_version: active.def_version,
                    params: json!({}),
                    edits: vec![Edit::Object {
                        type_name: "Flight".to_owned().try_into().unwrap(),
                        pk: PropertyValue::String("F-1001".to_owned()),
                        op: ObjectOp::Set(BTreeMap::from([(
                            "delayMinutes".to_owned().try_into().unwrap(),
                            PropertyValue::Int(4),
                        )])),
                    }],
                })
                .await
                .unwrap();
        }
        let drift_host = host(fixture.host, None);
        let engine = Arc::clone(&drift_host.engine);
        engine.use_real_metadata.store(true, Ordering::SeqCst);
        engine.expand_first_page.store(!backfill, Ordering::SeqCst);
        engine.reject_after.store(1, Ordering::SeqCst);
        let (url, stop, task) = start(drift_host).await;
        let response = reqwest::Client::new().post(format!("{url}/api/query")).bearer_auth(&fixture.token)
            .json(&json!({"set":{"kind":"type","object_type":"Flight"},"select":["flightId"],"page":{"first":500}})).send().await.unwrap();
        let status = response.status();
        let body: Value = response.json().await.unwrap();
        stop.send(()).unwrap();
        task.await.unwrap().unwrap();
        assert_eq!(
            status,
            StatusCode::SERVICE_UNAVAILABLE,
            "backfill={backfill}: {body}"
        );
        assert_eq!(body["code"], "SOURCE_SCHEMA_DRIFT");
        assert_eq!(
            engine.target_checks.load(Ordering::SeqCst),
            2,
            "backfill={backfill}: {body}"
        );
        assert_eq!(engine.inner.calls.load(Ordering::SeqCst), 1);
    }
}

#[tokio::test]
async fn search_around_checks_the_link_target_before_reading_join_rows() {
    use ontoforge_auth::authn::dev::{DevClaims, DevKey, issue_dev_token};
    let fixture = support::fixture().await;
    let key_path =
        camino::Utf8PathBuf::from_path_buf(fixture.dir.path().join("token.key")).unwrap();
    let key = DevKey::load_or_create(&key_path).unwrap();
    let token = issue_dev_token(
        &key,
        &DevClaims {
            sub: "alice".to_owned(),
            groups: vec!["dispatchers".to_owned(), "crew-schedulers".to_owned()],
            attrs: serde_json::Map::from_iter([("airports".to_owned(), json!(["FRA"]))]),
            ttl: Duration::from_secs(600),
        },
    )
    .unwrap();
    let drift_host = host(fixture.host, None);
    let engine = Arc::clone(&drift_host.engine);
    engine.use_real_metadata.store(true, Ordering::SeqCst);
    engine.deny_links.store(true, Ordering::SeqCst);
    let (url, stop, task) = start(drift_host).await;
    let response = reqwest::Client::new().post(format!("{url}/api/query")).bearer_auth(token)
        .json(&json!({"set":{"kind":"searchAround","object_type":"Pilot","base":{"kind":"pk","object_type":"Flight","pk":"F-1001"},"link":"flightCrew","direction":"fromTo"},"select":["employeeId"],"page":{"first":10}})).send().await.unwrap();
    let status = response.status();
    let body: Value = response.json().await.unwrap();
    stop.send(()).unwrap();
    task.await.unwrap().unwrap();
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
    assert_eq!(body["code"], "SOURCE_SCHEMA_DRIFT");
    assert_eq!(engine.target_checks.load(Ordering::SeqCst), 2);
    assert_eq!(
        engine.inner.calls.load(Ordering::SeqCst),
        1,
        "only the authorized base PK may reach source"
    );
}

#[tokio::test]
async fn no_source_definition_is_healthy_without_metadata_calls() {
    let fixture = support::fixture().await;
    let mut ontology = fixture
        .store
        .active_definition()
        .await
        .unwrap()
        .ontology
        .clone();
    for ty in ontology.object_types.values_mut() {
        ty.backing = ontoforge_schema::ir::Backing::EditOnly;
    }
    for link in ontology.link_types.values_mut() {
        if link.backing.is_some() {
            link.backing = Some(ontoforge_schema::ir::Backing::EditOnly);
        }
    }
    ontology.bindings.clear();
    ontology.default_profile = None;
    let path = camino::Utf8PathBuf::from_path_buf(fixture.dir.path().join("edit-only.db")).unwrap();
    let store = Arc::new(
        ontoforge_storage::store::EditStore::open(support::settings(&path))
            .await
            .unwrap(),
    );
    ontoforge_storage::reload::reload(
        &store,
        &ontology,
        &ontoforge_storage::reload::ReloadOptions::default(),
    )
    .await
    .unwrap();
    let mut drift_host = host(fixture.host, None);
    drift_host.store = store;
    let engine = Arc::clone(&drift_host.engine);
    let (url, stop, task) = start(drift_host).await;
    let response = reqwest::get(format!("{url}/health")).await.unwrap();
    let status = response.status();
    let body: Value = response.json().await.unwrap();
    stop.send(()).unwrap();
    task.await.unwrap().unwrap();
    fixture.store.read_audit().shutdown().await.unwrap();
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "healthy");
    assert_eq!(engine.metadata_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn actual_monitor_panic_becomes_check_failed_and_shutdown_reports_failure() {
    let fixture = support::fixture().await;
    let drift_host = host(fixture.host, None);
    let engine = Arc::clone(&drift_host.engine);
    let (url, stop, task) = start(drift_host).await;
    let initial: Value = reqwest::get(format!("{url}/health"))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(initial["status"], "degraded");
    engine.panic_metadata.store(true, Ordering::SeqCst);
    let mut ontology = fixture
        .store
        .active_definition()
        .await
        .unwrap()
        .ontology
        .clone();
    ontology.description = Some("new version after owner panic".to_owned());
    ontoforge_storage::reload::reload(
        &fixture.store,
        &ontology,
        &ontoforge_storage::reload::ReloadOptions::default(),
    )
    .await
    .unwrap();
    let _ = reqwest::get(format!("{url}/health")).await.unwrap();
    settle_until(|| engine.metadata_calls.load(Ordering::SeqCst) >= 2).await;
    let response = reqwest::Client::new()
        .get(format!("{url}/api/objects/Flight/F-1001"))
        .bearer_auth(&fixture.token)
        .send()
        .await
        .unwrap();
    let status = response.status();
    let body: Value = response.json().await.unwrap();
    stop.send(()).unwrap();
    assert!(matches!(task.await.unwrap(), Err(HostError::Shutdown)));
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["code"], "SOURCE_CHECK_UNAVAILABLE");
    assert_eq!(engine.inner.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn duplicate_owner_installation_is_rejected_without_stopping_the_first() {
    let fixture = support::fixture().await;
    let drift_host = Arc::new(host(fixture.host, None));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (stop, rx) = oneshot::channel();
    let task = tokio::spawn(Arc::clone(&drift_host).serve(listener, async {
        let _ = rx.await;
    }));
    let _ = reqwest::get(format!("{url}/health")).await.unwrap();
    let second = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let result = Arc::clone(&drift_host)
        .serve(second, std::future::pending())
        .await;
    let health = reqwest::get(format!("{url}/health"))
        .await
        .unwrap()
        .status();
    stop.send(()).unwrap();
    task.await.unwrap().unwrap();
    assert!(matches!(result, Err(HostError::SourceMonitor)));
    assert_eq!(health, StatusCode::OK);
    assert_eq!(drift_host.engine.metadata_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn invalid_duplicate_startup_cannot_close_the_running_owner_or_its_audit() {
    let fixture = support::fixture().await;
    let drift_host = Arc::new(host(fixture.host, None));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (stop, rx) = oneshot::channel();
    let task = tokio::spawn(Arc::clone(&drift_host).serve(listener, async {
        let _ = rx.await;
    }));
    let _ = reqwest::get(format!("{url}/health")).await.unwrap();
    let second = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let limits = ontoforge_server::ServerLimits {
        max_concurrent_requests: 0,
        ..Default::default()
    };
    let result = Arc::clone(&drift_host)
        .serve_with_limits(second, std::future::pending(), limits)
        .await;
    let response = reqwest::Client::new()
        .get(format!("{url}/api/objects/Airport/BER"))
        .bearer_auth(&fixture.token)
        .send()
        .await
        .unwrap();
    let status = response.status();
    let body: Value = response.json().await.unwrap();
    let audit_available = fixture.store.read_audit().health().available;
    stop.send(()).unwrap();
    let _ = task.await.unwrap();
    assert!(matches!(result, Err(HostError::Limits)));
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(audit_available);
    let pool = sqlx::SqlitePool::connect_with(ontoforge_storage::store::sqlite_connect_options(
        &fixture.path,
    ))
    .await
    .unwrap();
    let ids: Vec<String> = sqlx::query_scalar("SELECT request_id FROM of_read_audit")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(ids.len(), 1);
    pool.close().await;
}

#[tokio::test]
async fn invalid_fresh_startup_does_not_probe_or_take_the_store() {
    let fixture = support::fixture().await;
    let drift_host = Arc::new(host(fixture.host, None));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let limits = ontoforge_server::ServerLimits {
        max_concurrent_requests: 0,
        ..Default::default()
    };
    let result = Arc::clone(&drift_host)
        .serve_with_limits(listener, std::future::pending(), limits)
        .await;
    let available = fixture.store.read_audit().health().available;
    fixture.store.read_audit().shutdown().await.unwrap();
    assert!(matches!(result, Err(HostError::Limits)));
    assert!(available);
    assert_eq!(drift_host.engine.metadata_calls.load(Ordering::SeqCst), 0);
}

async fn action_only_fixture() -> support::Fixture {
    use ontoforge_auth::scope::{ScopeInputs, attr_types, compile_scope};
    let fixture = support::fixture().await;
    let snapshot = fixture.store.request_snapshot().await.unwrap();
    let mut ontology = snapshot.active.ontology.clone();
    ontology
        .action_types
        .get_mut("delayFlight")
        .unwrap()
        .submission_criteria
        .clear();
    ontoforge_storage::reload::reload(
        &fixture.store,
        &ontology,
        &ontoforge_storage::reload::ReloadOptions::default(),
    )
    .await
    .unwrap();
    let mut scope = snapshot.scope;
    scope
        .roles
        .get_mut("dispatcher")
        .unwrap()
        .object_types
        .clear();
    let compiled = compile_scope(&ScopeInputs {
        ontology: &ontology,
        profile: None,
        attrs: &attr_types(Some(&fixture.host.auth_config)),
        scope: &scope,
        sources: None,
    })
    .unwrap();
    let props = compiled.row_filter_properties();
    let plan = fixture.store.scope_plan(&scope, &props).await.unwrap();
    fixture
        .store
        .scope_apply(&scope, &props, &plan.hash, "action-without-type")
        .await
        .unwrap();
    fixture
}

async fn mcp_session(client: &reqwest::Client, url: &str, token: &str) -> String {
    let response = client.post(format!("{url}/mcp")).bearer_auth(token).header("accept", "application/json, text/event-stream")
        .json(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"drift-contract","version":"0"}}})).send().await.unwrap();
    let session = response.headers()["mcp-session-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let response = client
        .post(format!("{url}/mcp"))
        .bearer_auth(token)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-03-26")
        .header("mcp-session-id", &session)
        .json(&json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
        .send()
        .await
        .unwrap();
    assert!(response.status().is_success());
    session
}

async fn call_mcp_action(
    client: &reqwest::Client,
    url: &str,
    token: &str,
    session: &str,
    validate_only: bool,
) -> (StatusCode, Value) {
    let response = client.post(format!("{url}/mcp")).bearer_auth(token).header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-03-26").header("mcp-session-id", session)
        .json(&json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"action_delayFlight","arguments":{"params":{"flight":"F-1001","delayMinutes":15},"validateOnly":validate_only,"commandId":"mcp-drift-action"}}})).send().await.unwrap();
    let status = response.status();
    let body = response.text().await.unwrap();
    let value = body
        .lines()
        .find_map(|line| {
            line.strip_prefix("data: ")
                .and_then(|data| serde_json::from_str(data).ok())
        })
        .or_else(|| serde_json::from_str(&body).ok())
        .expect("MCP response JSON");
    (status, value)
}

async fn assert_action_rejection_has_no_writes_or_read_audit(path: &camino::Utf8Path) {
    let pool =
        sqlx::SqlitePool::connect_with(ontoforge_storage::store::sqlite_connect_options(path))
            .await
            .unwrap();
    let commits: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM of_edit_log")
        .fetch_one(&pool)
        .await
        .unwrap();
    let audits: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM of_read_audit")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(commits, 1, "only the initial fixture commit may exist");
    assert_eq!(
        audits, 0,
        "Action rejection must not invent a read audit subsystem"
    );
    pool.close().await;
}

#[tokio::test]
async fn action_type_authorization_precedes_source_drift_for_rest_and_mcp() {
    for healthy in [false, true] {
        let fixture = action_only_fixture().await;
        let drift_host = host(fixture.host, None);
        let engine = Arc::clone(&drift_host.engine);
        engine.use_real_metadata.store(healthy, Ordering::SeqCst);
        let (url, stop, task) = start(drift_host).await;
        let client = reqwest::Client::new();
        let session = mcp_session(&client, &url, &fixture.token).await;
        let mut rest = Vec::new();
        let mut mcp = Vec::new();
        for validate_only in [false, true] {
            let response = client.post(format!("{url}/api/actions/delayFlight")).bearer_auth(&fixture.token)
                .json(&json!({"commandId":"rest-no-type", "validateOnly":validate_only,"items":[{"params":{"flight":"F-1001","delayMinutes":15}}]})).send().await.unwrap();
            rest.push((response.status(), response.json::<Value>().await.unwrap()));
            mcp.push(call_mcp_action(&client, &url, &fixture.token, &session, validate_only).await);
        }
        stop.send(()).unwrap();
        task.await.unwrap().unwrap();
        for (status, body) in rest {
            assert_eq!(status, StatusCode::NOT_FOUND, "healthy={healthy}: {body}");
            assert_eq!(body["code"], "NOT_FOUND");
        }
        for (status, body) in mcp {
            assert_eq!(status, StatusCode::OK);
            assert_eq!(
                body["error"]["data"]["code"], "NOT_FOUND",
                "healthy={healthy}: {body}"
            );
        }
        assert_eq!(engine.inner.calls.load(Ordering::SeqCst), 0);
        assert_action_rejection_has_no_writes_or_read_audit(&fixture.path).await;
    }
}

#[tokio::test]
async fn mcp_authorized_action_preserves_drift_code_in_both_modes_without_side_effects() {
    let fixture = support::fixture().await;
    let drift_host = host(fixture.host, None);
    let engine = Arc::clone(&drift_host.engine);
    let (url, stop, task) = start(drift_host).await;
    let client = reqwest::Client::new();
    let session = mcp_session(&client, &url, &fixture.token).await;
    let mut results = Vec::new();
    for validate_only in [false, true] {
        results.push(call_mcp_action(&client, &url, &fixture.token, &session, validate_only).await);
    }
    stop.send(()).unwrap();
    task.await.unwrap().unwrap();
    for (status, body) in results {
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            body["error"]["data"]["code"], "SOURCE_SCHEMA_DRIFT",
            "{body}"
        );
        assert_eq!(body["error"]["data"]["apiName"], "action_delayFlight");
        assert!(
            body["error"]["data"]["request_id"]
                .as_str()
                .is_some_and(|id| !id.is_empty())
        );
    }
    assert_eq!(engine.inner.calls.load(Ordering::SeqCst), 0);
    assert_action_rejection_has_no_writes_or_read_audit(&fixture.path).await;
}

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "keep the fixed-view barrier scenario and durable assertions in causal order"
)]
async fn inflight_list_keeps_v1_view_for_second_source_call_after_v2_drift() {
    let fixture = support::fixture().await;
    let original = fixture.store.active_definition().await.unwrap();
    let original_version = original.def_version;
    let drift_host = host(fixture.host, None);
    let engine = Arc::clone(&drift_host.engine);
    engine.use_real_metadata.store(true, Ordering::SeqCst);
    engine.expand_first_page.store(true, Ordering::SeqCst);
    engine.inner.block.store(true, Ordering::SeqCst);
    let (url, stop, task) = start(drift_host).await;
    let client = reqwest::Client::new();
    let healthy: Value = client
        .get(format!("{url}/health"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(healthy["status"], "healthy");
    let old_request = client.post(format!("{url}/api/query")).bearer_auth(&fixture.token)
        .json(&json!({"set":{"kind":"type","object_type":"Flight"},"select":["flightId"],"page":{"first":500}}));
    let old = tokio::spawn(async move { old_request.send().await.unwrap() });
    tokio::time::timeout(Duration::from_secs(3), engine.inner.entered.notified())
        .await
        .unwrap();
    assert_eq!(engine.inner.calls.load(Ordering::SeqCst), 1);
    assert_eq!(engine.target_checks.load(Ordering::SeqCst), 1);

    engine.use_real_metadata.store(false, Ordering::SeqCst);
    let mut ontology = original.ontology.clone();
    ontology.description = Some("new generation with a degraded metadata report".to_owned());
    tokio::time::timeout(
        Duration::from_secs(3),
        ontoforge_storage::reload::reload(
            &fixture.store,
            &ontology,
            &ontoforge_storage::reload::ReloadOptions::default(),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    let new_version = fixture.store.active_definition().await.unwrap().def_version;
    assert!(new_version > original_version);
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let response = client.get(format!("{url}/health")).send().await.unwrap();
            let status = response.status();
            let body: Value = response.json().await.unwrap();
            if status == StatusCode::OK && body["status"] == "degraded" {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    let fresh = client
        .get(format!("{url}/api/objects/Flight/F-1001"))
        .bearer_auth(&fixture.token)
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .unwrap();
    let fresh_status = fresh.status();
    let fresh_body: Value = fresh.json().await.unwrap();

    engine.inner.block.store(false, Ordering::SeqCst);
    engine.inner.release.notify_one();
    let old = tokio::time::timeout(Duration::from_secs(3), old)
        .await
        .unwrap()
        .unwrap();
    let old_status = old.status();
    let old_id = old.headers()["x-request-id"].to_str().unwrap().to_owned();
    let old_body: Value = old.json().await.unwrap();
    stop.send(()).unwrap();
    task.await.unwrap().unwrap();

    assert_eq!(
        fresh_status,
        StatusCode::SERVICE_UNAVAILABLE,
        "{fresh_body}"
    );
    assert_eq!(fresh_body["code"], "SOURCE_SCHEMA_DRIFT");
    assert_eq!(old_status, StatusCode::OK, "{old_body}");
    assert_eq!(old_body["rows"].as_array().unwrap().len(), 256);
    assert_eq!(old_body["rows"][0]["pk"], "X-0000");
    assert_eq!(old_body["rows"][255]["pk"], "X-0255");
    assert_eq!(old_body["hasMore"], false);
    assert_eq!(
        engine.target_checks.load(Ordering::SeqCst),
        2,
        "the old request must pass its second gate"
    );
    assert_eq!(
        engine.inner.calls.load(Ordering::SeqCst),
        2,
        "the old request must actually read its second source window"
    );
    let pool = sqlx::SqlitePool::connect_with(ontoforge_storage::store::sqlite_connect_options(
        &fixture.path,
    ))
    .await
    .unwrap();
    let audit = sqlx::query(
        "SELECT def_version, status, row_count FROM of_read_audit WHERE request_id = ?",
    )
    .bind(&old_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audit.get::<i64, _>("def_version"), original_version);
    assert_eq!(audit.get::<String, _>("status"), "success");
    assert_eq!(audit.get::<i64, _>("row_count"), 256);
    let fresh_audit =
        sqlx::query("SELECT def_version, status FROM of_read_audit WHERE request_id = ?")
            .bind(fresh_body["request_id"].as_str().unwrap())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(fresh_audit.get::<i64, _>("def_version"), new_version);
    assert_eq!(fresh_audit.get::<String, _>("status"), "source_error");
    pool.close().await;
}
