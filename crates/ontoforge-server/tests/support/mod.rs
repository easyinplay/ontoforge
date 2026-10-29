#![allow(
    dead_code,
    reason = "shared HTTP fixtures are used by several contract suites"
)]

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use camino::{Utf8Path, Utf8PathBuf};
use ontoforge_auth::authn::Authenticator;
use ontoforge_auth::authn::dev::{DevClaims, DevKey, DevMode, DevVerifier, issue_dev_token};
use ontoforge_auth::scope::{ScopeInputs, attr_types, compile_scope};
use ontoforge_engine::config::AuthConfig;
use ontoforge_engine::sqlite::{SqliteEngine, fixture::plan_fixture_tables};
use ontoforge_engine::{
    Engine, EngineError, EngineStatus, PhysicalColumn, PrintedSql, QueryContext, QueryOutput,
    TableRef,
};
use ontoforge_schema::{load_dir, load_scope_dir};
use ontoforge_storage::commit::{Actor, Commit, Edit, ObjectOp};
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::store::{BackendSettings, EditStore, StoreSettings, sqlite_connect_options};
use ontoforge_storage::value::PropertyValue;
use reqwest::{Client, StatusCode};
use serde_json::{Value, json};
use sqlx::Row;
use tokio::net::TcpListener;
use tokio::sync::{Notify, oneshot};

use ontoforge_server::{HostError, ServerHost, ServerLimits};

pub const AVIATION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/ontologies/aviation"
);
const SCOPE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/scopes/aviation"
);

pub struct Fixture {
    pub dir: tempfile::TempDir,
    pub path: Utf8PathBuf,
    pub store: Arc<EditStore>,
    pub engine: Arc<ControlledEngine>,
    pub token: String,
    pub host: ServerHost<ControlledEngine>,
}

pub fn settings(path: &Utf8Path) -> StoreSettings {
    StoreSettings {
        backend: BackendSettings::Sqlite {
            path: path.to_owned(),
        },
        lock_timeout: Duration::from_secs(3),
        probe_timeout: Duration::from_secs(3),
        trino_catalog: None,
    }
}

pub async fn fixture() -> Fixture {
    fixture_airports(&[
        ("BER", "Berlin", Some("DE")),
        ("FRA", "Frankfurt", Some("DE")),
        ("MUC", "Munich", None),
    ])
    .await
}

pub async fn fixture_airports(airports: &[(&str, &str, Option<&str>)]) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let path = Utf8PathBuf::from_path_buf(dir.path().join("edit.db")).unwrap();
    let key_path = Utf8PathBuf::from_path_buf(dir.path().join("token.key")).unwrap();
    let store = Arc::new(EditStore::open(settings(&path)).await.unwrap());
    let loaded = load_dir(Utf8Path::new(AVIATION));
    let mut ontology = loaded.ontology.unwrap();
    ontology.description = Some("航空数据 UTF-8".to_owned());
    reload(&store, &ontology, &ReloadOptions::default())
        .await
        .unwrap();
    let scope = load_scope_dir(Utf8Path::new(SCOPE)).scope.unwrap();
    let config: AuthConfig = serde_json::from_value(json!({
        "principal_attrs": {"homeAirports": {"claim": "airports", "type": "string_list"}}
    }))
    .unwrap();
    let compiled = compile_scope(&ScopeInputs {
        ontology: &ontology,
        profile: None,
        attrs: &attr_types(Some(&config)),
        scope: &scope,
        sources: None,
    })
    .unwrap();
    let props = compiled.row_filter_properties();
    let plan = store.scope_plan(&scope, &props).await.unwrap();
    store
        .scope_apply(&scope, &props, &plan.hash, "http-contract-setup")
        .await
        .unwrap();
    let plan = plan_fixture_tables(
        &ontology,
        &loaded.sources,
        ontology.default_profile.as_ref().unwrap(),
    )
    .unwrap();
    let inner = SqliteEngine::from_fixtures(Utf8Path::new(AVIATION), &plan.tables)
        .await
        .unwrap();
    let engine = Arc::new(ControlledEngine {
        inner,
        block: AtomicBool::new(false),
        fail: AtomicBool::new(false),
        failure: Mutex::new(None),
        unavailable: AtomicBool::new(false),
        calls: AtomicUsize::new(0),
        entered: Notify::new(),
        release: Notify::new(),
        cancelled: Notify::new(),
    });
    let key = DevKey::load_or_create(&key_path).unwrap();
    let token = issue_dev_token(
        &key,
        &DevClaims {
            sub: "alice".to_owned(),
            groups: vec!["dispatchers".to_owned()],
            attrs: serde_json::Map::new(),
            ttl: Duration::from_secs(600),
        },
    )
    .unwrap();
    let auth = Authenticator::from_config(&config, key_path.parent().unwrap())
        .unwrap()
        .with_dev(DevVerifier::new(
            DevMode::enable_insecure(),
            key,
            &config.principal_attrs,
        ));
    let host = ServerHost::new(
        Arc::clone(&store),
        Arc::clone(&engine),
        auth,
        config,
        b"http-contract-signing-key".to_vec(),
    )
    .unwrap();
    put_airports(&store, airports).await;
    Fixture {
        dir,
        path,
        store,
        engine,
        token,
        host,
    }
}

pub async fn put_airports(store: &EditStore, rows: &[(&str, &str, Option<&str>)]) {
    let active = store.active_definition().await.unwrap();
    let edits = rows
        .iter()
        .map(|(pk, name, country)| {
            let mut properties = BTreeMap::from([(
                "name".to_owned().try_into().unwrap(),
                PropertyValue::String((*name).to_owned()),
            )]);
            if let Some(country) = country {
                properties.insert(
                    "country".to_owned().try_into().unwrap(),
                    PropertyValue::String((*country).to_owned()),
                );
            }
            Edit::Object {
                type_name: "Airport".to_owned().try_into().unwrap(),
                pk: PropertyValue::String((*pk).to_owned()),
                op: ObjectOp::Set(properties),
            }
        })
        .collect();
    store
        .commit(Commit {
            actor: Actor::Principal("ontoforge-dev|alice".to_owned()),
            action: "createAirport".to_owned(),
            def_version: active.def_version,
            params: json!({}),
            edits,
        })
        .await
        .unwrap();
}

pub struct Running {
    pub url: String,
    pub token: String,
    pub store: Arc<EditStore>,
    pub engine: Arc<ControlledEngine>,
    pub path: Utf8PathBuf,
    pub client: Client,
    stop: oneshot::Sender<()>,
    task: tokio::task::JoinHandle<Result<(), HostError>>,
    _dir: tempfile::TempDir,
}

impl Fixture {
    pub async fn start(self) -> Running {
        self.start_with_limits(ServerLimits::default()).await
    }

    pub async fn start_with_limits(self, limits: ServerLimits) -> Running {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let (stop, rx) = oneshot::channel();
        let task = tokio::spawn(Arc::new(self.host).serve_with_limits(
            listener,
            async {
                let _ = rx.await;
            },
            limits,
        ));
        Running {
            url,
            token: self.token,
            store: self.store,
            engine: self.engine,
            path: self.path,
            client: Client::new(),
            stop,
            task,
            _dir: self.dir,
        }
    }
}

impl Running {
    pub async fn get(&self, path: &str) -> reqwest::Response {
        self.client
            .get(format!("{}{path}", self.url))
            .bearer_auth(&self.token)
            .send()
            .await
            .unwrap()
    }
    pub async fn post(&self, path: &str, value: &Value) -> reqwest::Response {
        self.client
            .post(format!("{}{path}", self.url))
            .bearer_auth(&self.token)
            .json(value)
            .send()
            .await
            .unwrap()
    }
    pub async fn query(&self, set: Value, select: Value) -> Value {
        success(
            self.post(
                "/api/query",
                &json!({"set":set,"select":select,"page":{"first":100}}),
            )
            .await,
        )
        .await
    }
    pub async fn finish(self) -> Vec<Value> {
        self.stop.send(()).unwrap();
        self.task.await.unwrap().unwrap();
        let pool = sqlx::SqlitePool::connect_with(sqlite_connect_options(&self.path))
            .await
            .unwrap();
        let rows = sqlx::query("SELECT request_id, principal, def_version, status, row_count, tool FROM of_read_audit ORDER BY request_id").fetch_all(&pool).await.unwrap();
        let rows = rows.iter().map(|r| json!({
            "request_id":r.get::<String,_>("request_id"), "principal":r.get::<String,_>("principal"),
            "def_version":r.get::<i64,_>("def_version"), "status":r.get::<String,_>("status"),
            "row_count":r.get::<i64,_>("row_count"), "tool":r.get::<String,_>("tool")
        })).collect();
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM of_edit_log")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(
            count, 1,
            "reads and rejections must not write business commits"
        );
        pool.close().await;
        rows
    }
}

pub async fn success(response: reqwest::Response) -> Value {
    let status = response.status();
    let value: Value = response.json().await.unwrap();
    assert_eq!(status, StatusCode::OK, "{value}");
    value
}

pub fn keys(page: &Value) -> Vec<String> {
    page["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["pk"].as_str().unwrap().to_owned())
        .collect()
}

pub struct ControlledEngine {
    pub inner: SqliteEngine,
    pub block: AtomicBool,
    pub fail: AtomicBool,
    pub failure: Mutex<Option<EngineError>>,
    pub unavailable: AtomicBool,
    pub calls: AtomicUsize,
    pub entered: Notify,
    pub release: Notify,
    pub cancelled: Notify,
}

struct CancelWatch<'a>(&'a Notify, bool);
impl Drop for CancelWatch<'_> {
    fn drop(&mut self) {
        if !self.1 {
            self.0.notify_one();
        }
    }
}

impl Engine for ControlledEngine {
    fn dialect(&self) -> ontoforge_engine::Dialect {
        self.inner.dialect()
    }
    async fn execute(
        &self,
        sql: &PrintedSql,
        ctx: &QueryContext,
    ) -> Result<QueryOutput, EngineError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let mut watch = CancelWatch(&self.cancelled, false);
        if self.block.load(Ordering::SeqCst) {
            self.entered.notify_one();
            self.release.notified().await;
        }
        let failure = self.failure.lock().unwrap().clone();
        let result = if let Some(error) = failure {
            Err(error)
        } else if self.fail.load(Ordering::SeqCst) {
            Err(EngineError::Config(
                "physical_table SELECT secret_token=sentinel".to_owned(),
            ))
        } else {
            self.inner.execute(sql, ctx).await
        };
        watch.1 = true;
        result
    }
    async fn describe_tables(
        &self,
        tables: &[TableRef],
        ctx: &QueryContext,
    ) -> Result<BTreeMap<TableRef, Vec<PhysicalColumn>>, EngineError> {
        self.inner.describe_tables(tables, ctx).await
    }
    async fn status(&self) -> EngineStatus {
        if self.unavailable.load(Ordering::SeqCst) {
            EngineStatus::Unavailable {
                reason: "physical_table SELECT secret_token=sentinel".to_owned(),
            }
        } else {
            self.inner.status().await
        }
    }
}
