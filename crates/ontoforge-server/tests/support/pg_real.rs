#![allow(dead_code)]

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use camino::{Utf8Path, Utf8PathBuf};
use ontoforge_auth::authn::Authenticator;
use ontoforge_auth::authn::dev::{DevClaims, DevKey, DevMode, DevVerifier, issue_dev_token};
use ontoforge_auth::scope::{ScopeInputs, attr_types, compile_scope};
use ontoforge_engine::config::AuthConfig;
use ontoforge_engine::trino::{Auth, TrinoConnection, TrinoEngine};
use ontoforge_engine::{
    Engine, EngineError, EngineStatus, PhysicalColumn, PrintedSql, QueryContext, QueryOutput,
    TableRef,
};
use ontoforge_schema::{load_dir, load_scope_dir};
use ontoforge_server::{HostError, ServerHost, ServerLimits};
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::store::{BackendSettings, EditStore, StoreSettings};
use reqwest::Client;
use secrecy::SecretString;
use serde_json::json;
use tokio::net::TcpListener;
use tokio::sync::{Notify, oneshot};

const AVIATION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/ontologies/aviation"
);
const SCOPE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/scopes/aviation"
);

pub struct Fixture {
    pub store: Arc<EditStore>,
    pub other: Arc<EditStore>,
    pub side: sqlx::PgPool,
    pub schema_sys: String,
    pub engine: Arc<BarrierEngine>,
    pub host: ServerHost<BarrierEngine>,
    pub token: String,
    pub no_grant_token: String,
    pub key_dir: tempfile::TempDir,
}

async fn fixture(
    pg_url: String,
    trino_url: String,
    schema_edit: String,
    schema_sys: String,
) -> Fixture {
    let settings = || StoreSettings {
        backend: BackendSettings::Postgres {
            url: SecretString::from(pg_url.clone()),
            schema_edit: schema_edit.clone(),
            schema_sys: schema_sys.clone(),
        },
        lock_timeout: Duration::from_secs(5),
        probe_timeout: Duration::from_secs(5),
        trino_catalog: Some("editstore".to_owned()),
    };
    let store = Arc::new(
        EditStore::open(settings())
            .await
            .expect("open real PostgreSQL store"),
    );

    let loaded = load_dir(Utf8Path::new(AVIATION));
    let ontology = loaded.ontology.expect("aviation ontology");
    reload(&store, &ontology, &ReloadOptions::default())
        .await
        .expect("load aviation ontology into PostgreSQL");
    let scope = load_scope_dir(Utf8Path::new(SCOPE))
        .scope
        .expect("aviation scope");
    let config: AuthConfig = serde_json::from_value(json!({
        "principal_attrs": {"homeAirports": {"claim": "airports", "type": "string_list"}}
    }))
    .expect("auth config");
    let compiled = compile_scope(&ScopeInputs {
        ontology: &ontology,
        profile: None,
        attrs: &attr_types(Some(&config)),
        scope: &scope,
        sources: None,
    })
    .expect("compile scope");
    let props = compiled.row_filter_properties();
    let plan = store.scope_plan(&scope, &props).await.expect("scope plan");
    store
        .scope_apply(&scope, &props, &plan.hash, "pg-http-setup")
        .await
        .expect("apply scope");
    let other = Arc::new(
        EditStore::open(settings())
            .await
            .expect("second PostgreSQL store"),
    );
    let side = sqlx::PgPool::connect(&pg_url)
        .await
        .expect("PostgreSQL side pool");

    let key_dir = tempfile::tempdir().expect("key dir");
    let key_path = Utf8PathBuf::from_path_buf(key_dir.path().join("dev.key")).expect("utf8 key");
    let key = DevKey::load_or_create(&key_path).expect("dev key");
    let token = issue_dev_token(
        &key,
        &DevClaims {
            sub: "alice".to_owned(),
            groups: vec!["dispatchers".to_owned()],
            attrs: serde_json::Map::new(),
            ttl: Duration::from_secs(600),
        },
    )
    .expect("alice token");
    let no_grant_token = issue_dev_token(
        &key,
        &DevClaims {
            sub: "bob".to_owned(),
            groups: Vec::new(),
            attrs: serde_json::Map::new(),
            ttl: Duration::from_secs(600),
        },
    )
    .expect("bob token");
    let key_dir_path =
        Utf8PathBuf::from_path_buf(key_dir.path().to_path_buf()).expect("utf8 key dir");
    let auth = Authenticator::from_config(&config, Utf8Path::new(&key_dir_path))
        .expect("authenticator")
        .with_dev(DevVerifier::new(
            DevMode::enable_insecure(),
            key,
            &config.principal_attrs,
        ));
    let trino = TrinoEngine::new(TrinoConnection {
        url: trino_url.parse().expect("Trino URL"),
        user: "ontoforge".to_owned(),
        auth: Auth::None,
        ca_pem: None,
        connect_timeout: Duration::from_secs(5),
        max_page_bytes: 64 * 1024 * 1024,
        extra_origins: Vec::new(),
    })
    .expect("Trino engine");
    let engine = Arc::new(BarrierEngine {
        inner: trino,
        pause: AtomicBool::new(false),
        entered: Notify::new(),
        release: Notify::new(),
    });
    let host = ServerHost::new(
        Arc::clone(&store),
        Arc::clone(&engine),
        auth,
        config,
        b"pg-http-signing-key".to_vec(),
    )
    .expect("server host");
    Fixture {
        store,
        other,
        side,
        schema_sys,
        engine,
        host,
        token,
        no_grant_token,
        key_dir,
    }
}

pub struct Running {
    pub url: String,
    pub token: String,
    pub no_grant_token: String,
    pub client: Client,
    pub store: Arc<EditStore>,
    pub other: Arc<EditStore>,
    pub side: sqlx::PgPool,
    pub schema_sys: String,
    pub engine: Arc<BarrierEngine>,
    stop: oneshot::Sender<()>,
    task: Arc<Mutex<Option<tokio::task::JoinHandle<Result<(), HostError>>>>>,
    _key_dir: tempfile::TempDir,
}

impl Fixture {
    pub async fn start(self) -> Running {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let url = format!("http://{}", listener.local_addr().expect("address"));
        let (stop, rx) = oneshot::channel();
        let task = tokio::spawn(Arc::new(self.host).serve_with_limits(
            listener,
            async {
                let _ = rx.await;
            },
            ServerLimits::default(),
        ));
        Running {
            url,
            token: self.token,
            no_grant_token: self.no_grant_token,
            client: Client::new(),
            store: self.store,
            other: self.other,
            side: self.side,
            schema_sys: self.schema_sys,
            engine: self.engine,
            stop,
            task: Arc::new(Mutex::new(Some(task))),
            _key_dir: self.key_dir,
        }
    }
}

impl Running {
    pub async fn apply_scope(&self, scope: &ontoforge_schema::ir::ScopeSet) {
        let active = self
            .other
            .active_definition()
            .await
            .expect("active definition");
        let config: AuthConfig = serde_json::from_value(json!({
            "principal_attrs": {"homeAirports": {"claim": "airports", "type": "string_list"}}
        }))
        .expect("auth config");
        let compiled = compile_scope(&ScopeInputs {
            ontology: &active.ontology,
            profile: None,
            attrs: &attr_types(Some(&config)),
            scope,
            sources: None,
        })
        .expect("compile changed scope");
        let props = compiled.row_filter_properties();
        let plan = self
            .other
            .scope_plan(scope, &props)
            .await
            .expect("scope plan");
        self.other
            .scope_apply(scope, &props, &plan.hash, "pg-http-external-revoke")
            .await
            .expect("external scope apply");
    }

    pub async fn get(&self, path: &str, token: &str) -> reqwest::Response {
        self.client
            .get(format!("{}{path}", self.url))
            .bearer_auth(token)
            .send()
            .await
            .expect("GET")
    }

    pub async fn post(
        &self,
        path: &str,
        token: &str,
        body: &serde_json::Value,
    ) -> reqwest::Response {
        self.client
            .post(format!("{}{path}", self.url))
            .bearer_auth(token)
            .json(body)
            .send()
            .await
            .expect("POST")
    }

    pub async fn get_json(
        &self,
        path: &str,
        token: &str,
    ) -> (reqwest::StatusCode, serde_json::Value) {
        let response = self.get(path, token).await;
        let status = response.status();
        (status, response.json().await.expect("GET JSON"))
    }

    pub async fn post_json(
        &self,
        path: &str,
        token: &str,
        body: &serde_json::Value,
    ) -> (reqwest::StatusCode, serde_json::Value) {
        let response = self.post(path, token, body).await;
        let status = response.status();
        (status, response.json().await.expect("POST JSON"))
    }

    pub async fn finish(self) {
        self.stop.send(()).expect("stop");
        let task = self
            .task
            .lock()
            .expect("server task slot")
            .take()
            .expect("server task");
        task.await.expect("server task").expect("server shutdown");
        self.other
            .read_audit()
            .shutdown()
            .await
            .expect("second store audit drain");
    }

    pub async fn counts(&self) -> [i64; 5] {
        let mut counts = [0_i64; 5];
        for (index, table) in [
            "of_edit_log",
            "of_edit_log_entry",
            "of_edit_history",
            "of_action_command",
            "of_webhook_outbox",
        ]
        .iter()
        .enumerate()
        {
            let query = format!("SELECT COUNT(*) FROM \"{}\".\"{table}\"", self.schema_sys);
            counts[index] = sqlx::query_scalar(sqlx::AssertSqlSafe(query))
                .fetch_one(&self.side)
                .await
                .expect("PostgreSQL count");
        }
        counts
    }
}

pub async fn run_case<F, Fut>(test: F)
where
    F: FnOnce(Running) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    let pg_url = required("ONTOFORGE_PG_URL");
    let trino_url = required("ONTOFORGE_TRINO_URL");
    let unique = ulid::Ulid::generate().to_string().to_lowercase();
    let schema_edit = format!("of_it_{unique}_edit");
    let schema_sys = format!("of_it_{unique}_sys");
    let cleanup_pool = sqlx::PgPool::connect(&pg_url)
        .await
        .expect("cleanup PostgreSQL pool");
    let initialized = tokio::spawn(fixture(
        pg_url,
        trino_url,
        schema_edit.clone(),
        schema_sys.clone(),
    ))
    .await;
    let result = match initialized {
        Ok(fixture) => {
            let running = fixture.start().await;
            let task = Arc::clone(&running.task);
            let engine = Arc::clone(&running.engine);
            let store = Arc::clone(&running.store);
            let other = Arc::clone(&running.other);
            let result = tokio::spawn(test(running)).await;
            engine.release.notify_waiters();
            let pending = task.lock().expect("server task slot").take();
            if let Some(mut pending) = pending {
                if tokio::time::timeout(Duration::from_secs(10), &mut pending)
                    .await
                    .is_err()
                {
                    pending.abort();
                    let _ = pending.await;
                }
            }
            store
                .read_audit()
                .shutdown()
                .await
                .expect("primary store audit cleanup");
            other
                .read_audit()
                .shutdown()
                .await
                .expect("secondary store audit cleanup");
            result
        }
        Err(error) => Err(error),
    };
    for schema in [&schema_edit, &schema_sys] {
        let drop_sql = format!("DROP SCHEMA IF EXISTS \"{schema}\" CASCADE");
        sqlx::query(sqlx::AssertSqlSafe(drop_sql))
            .execute(&cleanup_pool)
            .await
            .expect("drop test PostgreSQL schema even after assertion failure");
    }
    cleanup_pool.close().await;
    if let Err(error) = result {
        if error.is_panic() {
            std::panic::resume_unwind(error.into_panic());
        }
        panic!("real PostgreSQL test task cancelled: {error}");
    }
}

pub struct BarrierEngine {
    inner: TrinoEngine,
    pub pause: AtomicBool,
    pub entered: Notify,
    pub release: Notify,
}

impl Engine for BarrierEngine {
    fn dialect(&self) -> ontoforge_engine::Dialect {
        self.inner.dialect()
    }

    async fn execute(
        &self,
        sql: &PrintedSql,
        ctx: &QueryContext,
    ) -> Result<QueryOutput, EngineError> {
        if self.pause.swap(false, Ordering::SeqCst) {
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

pub fn required(name: &str) -> String {
    match std::env::var(name) {
        Ok(value) if !value.trim().is_empty() => value,
        _ => panic!("{name} is required for real PostgreSQL HTTP tests"),
    }
}
