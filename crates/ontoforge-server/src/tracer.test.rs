#![allow(clippy::unused_async_trait_impl, reason = "test engine delegates after a synchronization barrier")]

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use camino::{Utf8Path, Utf8PathBuf};
use ontoforge_auth::authn::dev::{DevClaims, DevKey, DevMode, DevVerifier, issue_dev_token};
use ontoforge_auth::authn::Authenticator;
use ontoforge_auth::scope::{ScopeInputs, attr_types, compile_scope};
use ontoforge_engine::config::AuthConfig;
use ontoforge_engine::sqlite::{SqliteEngine, fixture::plan_fixture_tables};
use ontoforge_engine::{Engine, EngineError, EngineStatus, PhysicalColumn, PrintedSql, QueryContext, QueryOutput, TableRef};
use ontoforge_query::TokenCodec;
use ontoforge_schema::ir::{ObjectTypeName, PropertyName};
use ontoforge_schema::{load_dir, load_scope_dir};
use ontoforge_storage::commit::{Actor, Commit, Edit, ObjectOp};
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::store::{BackendSettings, EditStore, StoreSettings, sqlite_connect_options};
use ontoforge_storage::value::PropertyValue;
use reqwest::StatusCode;
use serde_json::{Value, json};
use sqlx::Row;
use tokio::net::TcpListener;
use tokio::sync::{Notify, oneshot};

use super::ServerHost;

const AVIATION: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/ontologies/aviation");
const SCOPE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/scopes/aviation");

struct Fixture {
    _dir: tempfile::TempDir,
    path: Utf8PathBuf,
    store: Arc<EditStore>,
    engine: SqliteEngine,
    auth: Authenticator,
    config: AuthConfig,
    token: String,
    expired: String,
}

fn settings(path: &Utf8Path) -> StoreSettings {
    StoreSettings {
        backend: BackendSettings::Sqlite { path: path.to_owned() },
        lock_timeout: Duration::from_secs(3),
        probe_timeout: Duration::from_secs(3),
        trino_catalog: None,
    }
}

async fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let path = Utf8PathBuf::from_path_buf(dir.path().join("edit.db")).unwrap();
    let key_path = Utf8PathBuf::from_path_buf(dir.path().join("token.key")).unwrap();
    let store = Arc::new(EditStore::open(settings(&path)).await.unwrap());
    let loaded = load_dir(Utf8Path::new(AVIATION));
    let ontology = loaded.ontology.unwrap();
    reload(&store, &ontology, &ReloadOptions::default()).await.unwrap();
    let scope = load_scope_dir(Utf8Path::new(SCOPE)).scope.unwrap();
    let config: AuthConfig = serde_json::from_value(json!({
        "principal_attrs": {"homeAirports": {"claim": "airports", "type": "string_list"}}
    })).unwrap();
    let compiled = compile_scope(&ScopeInputs {
        ontology: &ontology, profile: None, attrs: &attr_types(Some(&config)),
        scope: &scope, sources: None,
    }).unwrap();
    let props = compiled.row_filter_properties();
    let plan = store.scope_plan(&scope, &props).await.unwrap();
    store.scope_apply(&scope, &props, &plan.hash, "tracer-setup").await.unwrap();
    let active = store.active_definition().await.unwrap();
    store.commit(Commit {
        actor: Actor::Principal("ontoforge-dev|alice".to_owned()),
        action: "createAirport".to_owned(), def_version: active.def_version,
        params: json!({}),
        edits: vec![Edit::Object {
            type_name: ObjectTypeName::try_from("Airport".to_owned()).unwrap(),
            pk: PropertyValue::String("BER".to_owned()),
            op: ObjectOp::Set(BTreeMap::from([(
                PropertyName::try_from("name".to_owned()).unwrap(),
                PropertyValue::String("Berlin".to_owned()),
            )])),
        }],
    }).await.unwrap();
    let plan = plan_fixture_tables(&ontology, &loaded.sources, ontology.default_profile.as_ref().unwrap()).unwrap();
    let engine = SqliteEngine::from_fixtures(Utf8Path::new(AVIATION), &plan.tables).await.unwrap();
    let key = DevKey::load_or_create(&key_path).unwrap();
    let token = issue_dev_token(&key, &DevClaims {
        sub: "alice".to_owned(), groups: vec!["dispatchers".to_owned()],
        attrs: serde_json::Map::new(), ttl: Duration::from_secs(300),
    }).unwrap();
    let expired = jsonwebtoken::encode(
        &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256),
        &json!({"iss":"ontoforge-dev", "aud":"ontoforge-dev", "sub":"alice", "exp":1}),
        &jsonwebtoken::EncodingKey::from_secret(&std::fs::read(&key_path).unwrap()),
    ).unwrap();
    let auth = Authenticator::from_config(&config, key_path.parent().unwrap()).unwrap()
        .with_dev(DevVerifier::new(DevMode::enable_insecure(), key, &config.principal_attrs));
    Fixture { _dir: dir, path, store, engine, auth, config, token, expired }
}

fn host<E>(f: &Fixture, engine: E, auth: Authenticator) -> Arc<ServerHost<E>> {
    Arc::new(ServerHost {
        store: Arc::clone(&f.store), engine: Arc::new(engine), auth,
        auth_config: f.config.clone(), tokens: TokenCodec::new(b"tracer-token-key".to_vec()).unwrap(),
    })
}

async fn start<E: Engine + Send + Sync + 'static>(host: Arc<ServerHost<E>>) -> (String, oneshot::Sender<()>, tokio::task::JoinHandle<Result<(), super::HostError>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (tx, rx) = oneshot::channel();
    let task = tokio::spawn(host.serve(listener, async { let _ = rx.await; }));
    (url, tx, task)
}

#[tokio::test]
async fn tracer_authenticated_pk_is_durable_after_shutdown() {
    let mut f = fixture().await;
    let auth = std::mem::replace(&mut f.auth, Authenticator::from_config(&f.config, Utf8Path::new(".")).unwrap());
    let engine = std::mem::replace(&mut f.engine, SqliteEngine::in_memory().await.unwrap());
    let (url, stop, task) = start(host(&f, engine, auth)).await;
    let response = reqwest::Client::new().get(format!("{url}/api/objects/Airport/BER"))
        .bearer_auth(&f.token).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK, "authenticated pk read must reach QueryService");
    let request_id = response.headers().get("x-request-id").unwrap().to_str().unwrap().to_owned();
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["properties"]["name"], "Berlin");
    assert!(body.get("_version").and_then(Value::as_str).is_some());
    stop.send(()).unwrap();
    task.await.unwrap().unwrap();
    let reopened = EditStore::open(settings(&f.path)).await.unwrap();
    let pool = sqlx::SqlitePool::connect_with(sqlite_connect_options(&f.path)).await.unwrap();
    let rows = sqlx::query("SELECT request_id, principal, def_version, status FROM of_read_audit").fetch_all(&pool).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get::<String, _>("request_id"), request_id);
    assert_eq!(rows[0].get::<String, _>("principal"), "ontoforge-dev|alice");
    assert_eq!(rows[0].get::<i64, _>("def_version"), reopened.active_definition().await.unwrap().def_version);
    assert_eq!(rows[0].get::<String, _>("status"), "success");
    reopened.read_audit().shutdown().await.unwrap();
    pool.close().await;
}

#[tokio::test]
async fn tracer_unauthenticated_and_forged_context_are_denied() {
    let mut f = fixture().await;
    let auth = std::mem::replace(&mut f.auth, Authenticator::from_config(&f.config, Utf8Path::new(".")).unwrap());
    let engine = std::mem::replace(&mut f.engine, SqliteEngine::in_memory().await.unwrap());
    let (url, stop, task) = start(host(&f, engine, auth)).await;
    let client = reqwest::Client::new();
    for token in [None, Some("invalid"), Some(f.expired.as_str())] {
        let mut request = client.get(format!("{url}/api/objects/Airport/BER"));
        if let Some(token) = token { request = request.bearer_auth(token); }
        let response = request.send().await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "untrusted bearer must be rejected before read");
        let body: Value = response.json().await.unwrap();
        assert_eq!(body["code"], "UNAUTHENTICATED");
        assert_eq!(body["apiName"], "Airport");
        assert!(body["request_id"].as_str().is_some_and(|id| !id.is_empty()));
        assert!(!body.to_string().contains("Berlin"));
    }
    for field in ["principal", "roles", "scope", "limits"] {
        for request in [
            client.get(format!("{url}/api/objects/Airport/BER?{field}=forged")),
            client.get(format!("{url}/api/objects/Airport/BER")).json(&json!({field:"forged"})),
            client.get(format!("{url}/api/objects/Airport/BER")).header(format!("x-{field}"), "forged"),
        ] {
            let response = request.bearer_auth(&f.token).send().await.unwrap();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST, "trusted context injection must be rejected");
            let body: Value = response.json().await.unwrap();
            assert_eq!(body["code"], "INVALID_REQUEST");
            assert!(!body.to_string().contains("Berlin"));
        }
    }
    stop.send(()).unwrap();
    task.await.unwrap().unwrap();
    let pool = sqlx::SqlitePool::connect_with(sqlite_connect_options(&f.path)).await.unwrap();
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM of_read_audit").fetch_one(&pool).await.unwrap();
    assert_eq!(count, 0, "authentication and forged context rejection must not admit reads");
    pool.close().await;
}

struct BlockingEngine {
    inner: SqliteEngine,
    entered: Notify,
    release: Notify,
}

impl Engine for BlockingEngine {
    fn dialect(&self) -> ontoforge_engine::Dialect { self.inner.dialect() }
    async fn execute(&self, sql: &PrintedSql, ctx: &QueryContext) -> Result<QueryOutput, EngineError> {
        self.entered.notify_one();
        self.release.notified().await;
        self.inner.execute(sql, ctx).await
    }
    async fn describe_tables(&self, tables: &[TableRef], ctx: &QueryContext) -> Result<BTreeMap<TableRef, Vec<PhysicalColumn>>, EngineError> {
        self.inner.describe_tables(tables, ctx).await
    }
    async fn status(&self) -> EngineStatus { self.inner.status().await }
}

#[tokio::test]
async fn tracer_snapshot_releases_lock_before_source() {
    let mut f = fixture().await;
    let auth = std::mem::replace(&mut f.auth, Authenticator::from_config(&f.config, Utf8Path::new(".")).unwrap());
    let inner = std::mem::replace(&mut f.engine, SqliteEngine::in_memory().await.unwrap());
    let host = host(&f, BlockingEngine { inner, entered: Notify::new(), release: Notify::new() }, auth);
    let (url, stop, task) = start(Arc::clone(&host)).await;
    let token = f.token.clone();
    let request = tokio::spawn(async move {
        reqwest::Client::new().get(format!("{url}/api/objects/Flight/F-1001"))
            .bearer_auth(token).send().await.unwrap()
    });
    let entered = tokio::time::timeout(Duration::from_secs(2), host.engine.entered.notified()).await;
    assert!(entered.is_ok(), "authenticated source request must reach the source barrier");
    let other = EditStore::open(settings(&f.path)).await.unwrap();
    let snapshot = tokio::time::timeout(Duration::from_secs(2), other.request_snapshot()).await
        .expect("source network must not retain the metadata lock").unwrap();
    let mut ontology = snapshot.active.ontology.clone();
    ontology.description = Some("Reload while source waits".to_owned());
    let reloaded = tokio::time::timeout(Duration::from_secs(2), reload(&other, &ontology, &ReloadOptions::default())).await
        .expect("reload must complete while source is pending").unwrap();
    assert!(reloaded.changed);
    host.engine.release.notify_one();
    let response = request.await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let request_id = response.headers()["x-request-id"].to_str().unwrap().to_owned();
    stop.send(()).unwrap();
    task.await.unwrap().unwrap();
    let pool = sqlx::SqlitePool::connect_with(sqlite_connect_options(&f.path)).await.unwrap();
    let version: i64 = sqlx::query_scalar("SELECT def_version FROM of_read_audit WHERE request_id = ?")
        .bind(request_id).fetch_one(&pool).await.unwrap();
    assert_eq!(version, snapshot.active.def_version, "in-flight request must retain its original definition");
    other.read_audit().shutdown().await.unwrap();
    pool.close().await;
}
