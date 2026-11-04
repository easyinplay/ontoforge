#[path = "support/real.rs"]
mod support;

use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use camino::Utf8PathBuf;
use ontoforge_auth::authn::Authenticator;
use ontoforge_auth::authn::dev::{DevClaims, DevKey, DevMode, DevVerifier, issue_dev_token};
use ontoforge_auth::scope::{ScopeInputs, attr_types, compile_scope};
use ontoforge_engine::config::AuthConfig;
use ontoforge_engine::trino::{Auth, TrinoConnection, TrinoEngine};
use ontoforge_engine::{
    Engine, EngineError, EngineStatus, PhysicalColumn, PrintedSql, QueryContext, QueryOutput,
    TableRef,
};
use ontoforge_schema::ir::Ontology;
use ontoforge_server::{HostError, ServerHost};
use ontoforge_storage::commit::{Actor, Commit, Edit, ObjectOp};
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::store::{BackendSettings, EditStore, StoreSettings};
use ontoforge_storage::value::PropertyValue;
use reqwest::StatusCode;
use secrecy::SecretString;
use serde_json::{Value, json};
use tokio::sync::{Notify, Semaphore, oneshot};

const OBSERVE_TIMEOUT: Duration = Duration::from_secs(75);

struct MetadataBarrier {
    inner: TrinoEngine,
    hold_next: AtomicUsize,
    entered: Notify,
    release: Semaphore,
    business_calls: AtomicUsize,
}

impl Engine for MetadataBarrier {
    fn dialect(&self) -> ontoforge_engine::Dialect {
        self.inner.dialect()
    }

    async fn execute(
        &self,
        sql: &PrintedSql,
        ctx: &QueryContext,
    ) -> Result<QueryOutput, EngineError> {
        self.business_calls.fetch_add(1, Ordering::SeqCst);
        self.inner.execute(sql, ctx).await
    }

    async fn describe_tables(
        &self,
        tables: &[TableRef],
        ctx: &QueryContext,
    ) -> Result<BTreeMap<TableRef, Vec<PhysicalColumn>>, EngineError> {
        let metadata = self.inner.describe_tables(tables, ctx).await?;
        if self
            .hold_next
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |count| {
                count.checked_sub(1)
            })
            .is_ok()
        {
            self.entered.notify_one();
            self.release
                .acquire()
                .await
                .expect("metadata release")
                .forget();
        }
        Ok(metadata)
    }

    async fn status(&self) -> EngineStatus {
        self.inner.status().await
    }
    async fn drain_cancellations(&self) -> Result<(), EngineError> {
        self.inner.drain_cancellations().await
    }
}

fn ontology(source_schema: &str, next_version: bool) -> Ontology {
    let extra_property = if next_version {
        "  newValue: { type: string }\n"
    } else {
        ""
    };
    let extra_binding = if next_version {
        "  newValue: { column: new_value }\n"
    } else {
        ""
    };
    let objects = format!(
        "kind: ObjectType\napiName: Wide\nbacking: source\nprimaryKey: id\nproperties:\n  id: {{ type: string, required: true }}\n  value: {{ type: string }}\n{extra_property}---\nkind: ObjectType\napiName: Narrow\nbacking: source\nprimaryKey: id\nproperties:\n  id: {{ type: string, required: true }}\n  value: {{ type: string }}\n---\nkind: ObjectType\napiName: Local\nbacking: editOnly\nprimaryKey: id\nproperties:\n  id: {{ type: string, required: true }}\n  value: {{ type: string }}\n"
    );
    let bindings = format!(
        "kind: SourceBinding\ntarget: {{ objectType: Wide }}\nsource: {{ catalog: demo, schema: {source_schema}, table: source_rows }}\ncolumns:\n  id: {{ column: physical_id }}\n  value: {{ column: drift_value }}\n{extra_binding}---\nkind: SourceBinding\ntarget: {{ objectType: Narrow }}\nsource: {{ catalog: demo, schema: {source_schema}, table: source_rows }}\ncolumns:\n  id: {{ column: physical_id }}\n  value: {{ column: stable_value }}\n"
    );
    let loaded = ontoforge_schema::load_from_memory(&[
        (
            "ontology.yaml",
            "apiName: driftFixture\ndefaultProfile: dev\n",
        ),
        ("objects.yaml", &objects),
        ("bindings/dev/source.yaml", &bindings),
    ]);
    assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
    loaded.ontology.expect("drift ontology")
}

fn settings(pg_url: &str, edit: &str, sys: &str) -> StoreSettings {
    StoreSettings {
        backend: BackendSettings::Postgres {
            url: SecretString::from(pg_url.to_owned()),
            schema_edit: edit.to_owned(),
            schema_sys: sys.to_owned(),
        },
        lock_timeout: Duration::from_secs(5),
        probe_timeout: Duration::from_secs(5),
        trino_catalog: Some("editstore".to_owned()),
    }
}

async fn apply_scope(store: &EditStore, definition: &Ontology, config: &AuthConfig) {
    let loaded = ontoforge_schema::load_scope_from_memory(&[(
        "scope.yaml",
        "kind: Role\napiName: reader\nobjectTypes:\n  Wide:\n    properties: [id, value]\n  Narrow:\n    properties: [id, value]\n  Local:\n    properties: [id, value]\ntoolsets: [core, full]\n---\nkind: RoleBinding\nrole: reader\nsubjects:\n  - group: readers\n",
    )]);
    let scope = loaded.scope.expect("drift scope");
    let compiled = compile_scope(&ScopeInputs {
        ontology: definition,
        profile: None,
        attrs: &attr_types(Some(config)),
        scope: &scope,
        sources: None,
    })
    .expect("compile drift scope");
    let props = compiled.row_filter_properties();
    let plan = store.scope_plan(&scope, &props).await.expect("scope plan");
    store
        .scope_apply(&scope, &props, &plan.hash, "drift-real-fixture")
        .await
        .expect("scope apply");
}

type ServerTask = Arc<Mutex<Option<tokio::task::JoinHandle<Result<(), HostError>>>>>;

struct Running {
    url: String,
    token: String,
    source: sqlx::PgPool,
    side: sqlx::PgPool,
    schema_source: String,
    schema_sys: String,
    store: Arc<EditStore>,
    other: Arc<EditStore>,
    engine: Arc<MetadataBarrier>,
    stop: Option<oneshot::Sender<()>>,
    task: ServerTask,
    _keys: tempfile::TempDir,
}

impl Drop for Running {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
    }
}

async fn source_sql(pool: &sqlx::PgPool, schema: &str, suffix: &str) {
    assert!(
        schema.starts_with("of_drift_")
            && schema
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    );
    let sql = format!("ALTER TABLE \"{schema}\".source_rows {suffix}");
    sqlx::query(sqlx::AssertSqlSafe(sql))
        .execute(pool)
        .await
        .expect("alter isolated source table");
}

#[expect(
    clippy::too_many_lines,
    reason = "keeps isolated real-service setup and ownership together"
)]
async fn setup(
    pg_url: String,
    trino_url: String,
    schema_edit: String,
    schema_sys: String,
    schema_source: String,
    source: sqlx::PgPool,
    side: sqlx::PgPool,
) -> Running {
    let create = format!("CREATE SCHEMA \"{schema_source}\"");
    sqlx::query(sqlx::AssertSqlSafe(create))
        .execute(&source)
        .await
        .expect("create isolated source schema");
    let table = format!(
        "CREATE TABLE \"{schema_source}\".source_rows (physical_id varchar PRIMARY KEY, stable_value varchar, drift_value varchar)"
    );
    sqlx::query(sqlx::AssertSqlSafe(table))
        .execute(&source)
        .await
        .expect("create isolated source table");
    let insert = format!(
        "INSERT INTO \"{schema_source}\".source_rows VALUES ('row-1', 'stable', 'original')"
    );
    sqlx::query(sqlx::AssertSqlSafe(insert))
        .execute(&source)
        .await
        .expect("seed isolated source row");
    let store = Arc::new(
        EditStore::open(settings(&pg_url, &schema_edit, &schema_sys))
            .await
            .unwrap_or_else(|_| panic!("open isolated PostgreSQL store failed")),
    );
    let definition = ontology(&schema_source, false);
    reload(&store, &definition, &ReloadOptions::default())
        .await
        .expect("reload drift fixture");
    let config: AuthConfig = serde_json::from_value(json!({})).expect("auth config");
    apply_scope(&store, &definition, &config).await;
    let active = store.active_definition().await.expect("active definition");
    store
        .commit(Commit {
            actor: Actor::Principal("ontoforge-dev|drift-reader".to_owned()),
            action: "seedLocal".to_owned(),
            def_version: active.def_version,
            params: json!({}),
            edits: vec![Edit::Object {
                type_name: "Local".to_owned().try_into().unwrap(),
                pk: PropertyValue::String("row-1".to_owned()),
                op: ObjectOp::Set(BTreeMap::from([(
                    "value".to_owned().try_into().unwrap(),
                    PropertyValue::String("local".to_owned()),
                )])),
            }],
        })
        .await
        .expect("seed edit-only row");
    let other = Arc::new(
        EditStore::open(settings(&pg_url, &schema_edit, &schema_sys))
            .await
            .unwrap_or_else(|_| panic!("open second PostgreSQL store failed")),
    );
    let keys = tempfile::tempdir().expect("key directory");
    let key_path = Utf8PathBuf::from_path_buf(keys.path().join("dev.key")).unwrap();
    let key = DevKey::load_or_create(&key_path).expect("dev key");
    let token = issue_dev_token(
        &key,
        &DevClaims {
            sub: "drift-reader".to_owned(),
            groups: vec!["readers".to_owned()],
            attrs: serde_json::Map::new(),
            ttl: Duration::from_secs(600),
        },
    )
    .expect("dev token");
    let auth = Authenticator::from_config(&config, key_path.parent().unwrap())
        .expect("authenticator")
        .with_dev(DevVerifier::new(
            DevMode::enable_insecure(),
            key,
            &config.principal_attrs,
        ));
    let inner = TrinoEngine::new(TrinoConnection {
        url: trino_url
            .parse()
            .unwrap_or_else(|_| panic!("invalid Trino URL")),
        user: "ontoforge".to_owned(),
        auth: Auth::None,
        ca_pem: None,
        connect_timeout: Duration::from_secs(5),
        max_page_bytes: 16 * 1024 * 1024,
        extra_origins: Vec::new(),
    })
    .unwrap_or_else(|_| panic!("Trino engine configuration failed"));
    let engine = Arc::new(MetadataBarrier {
        inner,
        hold_next: AtomicUsize::new(0),
        entered: Notify::new(),
        release: Semaphore::new(0),
        business_calls: AtomicUsize::new(0),
    });
    let host = ServerHost::new(
        Arc::clone(&store),
        Arc::clone(&engine),
        auth,
        config,
        b"drift-real-signing-key".to_vec(),
    )
    .expect("host");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (stop, rx) = oneshot::channel();
    let task = tokio::spawn(Arc::new(host).serve(listener, async {
        let _ = rx.await;
    }));
    Running {
        url,
        token,
        source,
        side,
        schema_source,
        schema_sys,
        store,
        other,
        engine,
        stop: Some(stop),
        task: Arc::new(Mutex::new(Some(task))),
        _keys: keys,
    }
}

async fn run_case<F, Fut>(test: F)
where
    F: FnOnce(Running) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let pg_url = support::required("ONTOFORGE_PG_URL");
    let trino_url = support::required("ONTOFORGE_TRINO_URL");
    let options = sqlx::postgres::PgConnectOptions::from_str(&pg_url)
        .unwrap_or_else(|_| panic!("invalid PostgreSQL URL"));
    let cleanup = sqlx::PgPool::connect_with(options.clone())
        .await
        .unwrap_or_else(|_| panic!("PostgreSQL connection failed"));
    let source = sqlx::PgPool::connect_with(options.database("demo"))
        .await
        .unwrap_or_else(|_| panic!("source PostgreSQL connection failed"));
    let unique = ulid::Ulid::generate().to_string().to_lowercase();
    let edit = format!("of_drift_{unique}_edit");
    let sys = format!("of_drift_{unique}_sys");
    let src = format!("of_drift_{unique}_source");
    let initialized = tokio::spawn(setup(
        pg_url,
        trino_url,
        edit.clone(),
        sys.clone(),
        src.clone(),
        source.clone(),
        cleanup.clone(),
    ))
    .await;
    let mut shutdown_ok = true;
    let result = match initialized {
        Ok(running) => {
            let task = Arc::clone(&running.task);
            let engine = Arc::clone(&running.engine);
            let store = Arc::clone(&running.store);
            let other = Arc::clone(&running.other);
            let outcome = tokio::spawn(test(running)).await;
            engine.release.add_permits(2);
            let pending = task.lock().unwrap().take();
            if let Some(mut pending) = pending {
                match tokio::time::timeout(Duration::from_secs(12), &mut pending).await {
                    Ok(Ok(Ok(()))) => {}
                    Ok(_) => shutdown_ok = false,
                    Err(_) => {
                        pending.abort();
                        let _ = pending.await;
                        shutdown_ok = false;
                    }
                }
            }
            shutdown_ok &= store.read_audit().shutdown().await.is_ok();
            shutdown_ok &= other.read_audit().shutdown().await.is_ok();
            shutdown_ok &= engine.drain_cancellations().await.is_ok();
            outcome
        }
        Err(error) => Err(error),
    };
    for (pool, schema) in [(&source, &src), (&cleanup, &edit), (&cleanup, &sys)] {
        let sql = format!("DROP SCHEMA IF EXISTS \"{schema}\" CASCADE");
        sqlx::query(sqlx::AssertSqlSafe(sql))
            .execute(pool)
            .await
            .expect("drop only suite-owned schema");
    }
    source.close().await;
    cleanup.close().await;
    if let Err(error) = result {
        if error.is_panic() {
            std::panic::resume_unwind(error.into_panic());
        }
        panic!("drift test task cancelled");
    }
    assert!(
        shutdown_ok,
        "host must stop before test schemas are dropped"
    );
}

struct Reply {
    status: StatusCode,
    id: String,
    body: Value,
}

async fn get(running: &Running, path: &str) -> Reply {
    let response = support::get(&running.url, path, &running.token).await;
    let status = response.status();
    let id = response
        .headers()
        .get("x-request-id")
        .and_then(|id| id.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    Reply {
        status,
        id,
        body: response.json().await.expect("JSON response"),
    }
}

async fn wait_state(
    running: &Running,
    path: &str,
    status: StatusCode,
    field: &str,
    expected: &str,
) -> Reply {
    let mut last = None;
    let outcome = tokio::time::timeout(OBSERVE_TIMEOUT, async {
        loop {
            let reply = get(running, path).await;
            if reply.status == status && reply.body[field] == expected {
                break reply;
            }
            last = Some((reply.status, reply.body[field].clone()));
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    })
    .await;
    outcome.unwrap_or_else(|_| {
        panic!("production monitor did not reach {expected} within observation budget; last HTTP/{field}: {last:?}")
    })
}

async fn assert_healthy(running: &Running) {
    let health = wait_state(running, "/health", StatusCode::OK, "status", "healthy").await;
    assert_eq!(
        health.body,
        json!({"status":"healthy","engineStatus":"available"})
    );
    for (target, value) in [
        ("Wide", "original"),
        ("Narrow", "stable"),
        ("Local", "local"),
    ] {
        assert_rest_object(running, target, value).await;
    }
}

fn assert_object(body: &Value, target: &str, value: &str) {
    assert!(body.is_object(), "{target}: expected an object, got {body}");
    assert_eq!(body["objectType"], target);
    assert_eq!(body["pk"], "row-1");
    assert_eq!(body["properties"]["id"], "row-1");
    assert_eq!(body["properties"]["value"], value);
}

async fn assert_rest_object(running: &Running, target: &str, value: &str) {
    let reply = get(running, &format!("/api/objects/{target}/row-1")).await;
    assert_eq!(reply.status, StatusCode::OK, "{target}: {}", reply.body);
    assert_object(&reply.body, target, value);
}

fn assert_mcp_object(response: &Value, target: &str, value: &str) {
    assert!(response.get("error").is_none(), "{response}");
    assert!(response["result"].is_object(), "{response}");
    assert_object(&response["result"]["structuredContent"], target, value);
}

async fn initialize_mcp(running: &Running) -> String {
    let response = support::client().post(format!("{}/mcp", running.url)).bearer_auth(&running.token)
        .header("accept", "application/json, text/event-stream")
        .json(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"drift-real","version":"0"}}}))
        .send().await.expect("MCP initialize");
    assert!(response.status().is_success());
    let session = response.headers()["mcp-session-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let initialized = support::client()
        .post(format!("{}/mcp", running.url))
        .bearer_auth(&running.token)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-03-26")
        .header("mcp-session-id", &session)
        .json(&json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
        .send()
        .await
        .expect("MCP initialized notification");
    assert!(initialized.status().is_success());
    session
}

async fn mcp_get(running: &Running, session: &str, id: u64, target: &str) -> Value {
    let response = support::client().post(format!("{}/mcp", running.url)).bearer_auth(&running.token)
        .header("accept", "application/json, text/event-stream").header("mcp-protocol-version", "2025-03-26").header("mcp-session-id", session)
        .json(&json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":"get_object","arguments":{"objectType":target,"pk":"row-1"}}}))
        .send().await.expect("MCP call");
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "MCP tool responses keep HTTP 200"
    );
    let body = response.text().await.expect("MCP body");
    let payload: Value = body
        .lines()
        .find_map(|line| {
            line.strip_prefix("data: ")
                .and_then(|data| serde_json::from_str(data).ok())
        })
        .or_else(|| serde_json::from_str(&body).ok())
        .expect("MCP JSON");
    assert_eq!(payload["jsonrpc"], "2.0", "{payload}");
    assert_eq!(payload["id"], id, "{payload}");
    assert_ne!(
        payload.get("result").is_some(),
        payload.get("error").is_some(),
        "exactly one result or error is required: {payload}"
    );
    if let Some(error) = payload.get("error") {
        assert!(
            error.is_object() && error["code"].as_i64().is_some(),
            "{payload}"
        );
    } else {
        assert!(payload["result"].is_object(), "{payload}");
    }
    payload
}

async fn assert_audit(running: &Running, id: &str, version: i64) {
    assert!(!id.is_empty());
    let sql = format!(
        "SELECT def_version, status FROM \"{}\".of_read_audit WHERE request_id = $1",
        running.schema_sys
    );
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let rows: Vec<(i64, String)> = sqlx::query_as(sqlx::AssertSqlSafe(sql.clone()))
                .bind(id)
                .fetch_all(&running.side)
                .await
                .expect("audit row");
            if !rows.is_empty() {
                assert_eq!(rows, vec![(version, "source_error".to_owned())]);
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("durable read audit");
}

#[tokio::test]
#[ignore = "requires ONTOFORGE_PG_URL and ONTOFORGE_TRINO_URL; real Trino demo catalog must address the PostgreSQL demo database"]
async fn real_source_drift_periodic_target_isolation_and_recovery() {
    run_case(|running| async move {
        assert_healthy(&running).await;
        let session = initialize_mcp(&running).await;
        let version = running
            .store
            .request_snapshot()
            .await
            .unwrap()
            .active
            .def_version;
        source_sql(
            &running.source,
            running.schema_source.as_str(),
            "DROP COLUMN drift_value",
        )
        .await;
        let denied = wait_state(
            &running,
            "/api/objects/Wide/row-1",
            StatusCode::SERVICE_UNAVAILABLE,
            "code",
            "SOURCE_SCHEMA_DRIFT",
        )
        .await;
        assert_eq!(denied.body["request_id"], denied.id);
        assert_audit(&running, &denied.id, version).await;
        let calls = running.engine.business_calls.load(Ordering::SeqCst);
        let rpc = mcp_get(&running, &session, 2, "Wide").await;
        assert_eq!(rpc["error"]["data"]["code"], "SOURCE_SCHEMA_DRIFT", "{rpc}");
        assert_eq!(rpc["error"]["data"]["apiName"], "get_object");
        assert_audit(
            &running,
            rpc["error"]["data"]["request_id"].as_str().unwrap(),
            version,
        )
        .await;
        assert_eq!(
            running.engine.business_calls.load(Ordering::SeqCst),
            calls,
            "known drift must reject before business SQL"
        );
        for (target, value) in [("Narrow", "stable"), ("Local", "local")] {
            assert_rest_object(&running, target, value).await;
            assert_mcp_object(&mcp_get(&running, &session, 3, target).await, target, value);
        }
        let health = get(&running, "/health").await;
        assert_eq!(health.status, StatusCode::OK);
        assert_eq!(
            health.body,
            json!({"status":"degraded","engineStatus":"available"})
        );
        for secret in [
            running.schema_source.as_str(),
            "source_rows",
            "drift_value",
            "Wide",
            "Narrow",
        ] {
            assert!(!health.body.to_string().contains(secret));
        }
        assert!(!denied.body.to_string().contains(&running.schema_source));
        source_sql(
            &running.source,
            &running.schema_source,
            "ADD COLUMN drift_value varchar",
        )
        .await;
        let restore = format!(
            "UPDATE \"{}\".source_rows SET drift_value = 'original'",
            running.schema_source
        );
        sqlx::query(sqlx::AssertSqlSafe(restore))
            .execute(&running.source)
            .await
            .expect("restore isolated source column");
        assert_healthy(&running).await;
        assert_mcp_object(
            &mcp_get(&running, &session, 4, "Wide").await,
            "Wide",
            "original",
        );
    })
    .await;
}

#[tokio::test]
#[ignore = "requires ONTOFORGE_PG_URL and ONTOFORGE_TRINO_URL; exercises two PostgreSQL stores and a real metadata response barrier"]
async fn real_source_drift_external_reload_does_not_publish_old_metadata() {
    run_case(|running| async move {
        assert_healthy(&running).await;
        let previous = running
            .store
            .request_snapshot()
            .await
            .unwrap()
            .active
            .def_version;
        running.engine.hold_next.store(2, Ordering::SeqCst);
        tokio::time::timeout(OBSERVE_TIMEOUT, running.engine.entered.notified())
            .await
            .expect("old production periodic metadata captured");
        let next = ontology(&running.schema_source, true);
        tokio::time::timeout(
            Duration::from_secs(5),
            reload(&running.other, &next, &ReloadOptions::default()),
        )
        .await
        .expect("metadata lock must not span source await")
        .expect("external reload");
        let current = running
            .other
            .request_snapshot()
            .await
            .unwrap()
            .active
            .def_version;
        assert!(current > previous);
        let pending = get(&running, "/api/objects/Wide/row-1").await;
        assert_eq!(pending.status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            pending.body["code"], "SOURCE_CHECK_PENDING",
            "{}",
            pending.body
        );
        running.engine.release.add_permits(1);
        tokio::time::timeout(Duration::from_secs(10), running.engine.entered.notified())
            .await
            .expect("new-version metadata captured after old response release");
        let pending = get(&running, "/api/objects/Wide/row-1").await;
        assert_eq!(pending.status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            pending.body["code"], "SOURCE_CHECK_PENDING",
            "old healthy report must not be reused: {}",
            pending.body
        );
        let health = get(&running, "/health").await;
        assert_eq!(health.status, StatusCode::SERVICE_UNAVAILABLE);
        running.engine.release.add_permits(1);
        let denied = wait_state(
            &running,
            "/api/objects/Wide/row-1",
            StatusCode::SERVICE_UNAVAILABLE,
            "code",
            "SOURCE_SCHEMA_DRIFT",
        )
        .await;
        assert_audit(&running, &denied.id, current).await;
        assert_rest_object(&running, "Narrow", "stable").await;
        assert_rest_object(&running, "Local", "local").await;
    })
    .await;
}
