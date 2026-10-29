#![allow(dead_code, reason = "shared real HTTP fixture helpers")]

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
use ontoforge_server::{HostError, ServerHost, ServerLimits};
use ontoforge_storage::commit::{Actor, Commit, Edit, ObjectOp};
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::store::{BackendSettings, EditStore, StoreSettings, sqlite_connect_options};
use ontoforge_storage::value::PropertyValue;
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
    dir: tempfile::TempDir,
    path: Utf8PathBuf,
    store: Arc<EditStore>,
    pub engine: Arc<ControlledEngine>,
    token: String,
    host: ServerHost<ControlledEngine>,
}

pub async fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let path = Utf8PathBuf::from_path_buf(dir.path().join("edit.db")).unwrap();
    let key_path = Utf8PathBuf::from_path_buf(dir.path().join("token.key")).unwrap();
    let settings = StoreSettings {
        backend: BackendSettings::Sqlite { path: path.clone() },
        lock_timeout: Duration::from_secs(3),
        probe_timeout: Duration::from_secs(3),
        trino_catalog: None,
    };
    let store = Arc::new(EditStore::open(settings).await.unwrap());
    let loaded = load_dir(Utf8Path::new(AVIATION));
    let ontology = loaded.ontology.unwrap();
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
        .scope_apply(&scope, &props, &plan.hash, "client-contract-setup")
        .await
        .unwrap();
    let table_plan = plan_fixture_tables(
        &ontology,
        &loaded.sources,
        ontology.default_profile.as_ref().unwrap(),
    )
    .unwrap();
    let inner = SqliteEngine::from_fixtures(Utf8Path::new(AVIATION), &table_plan.tables)
        .await
        .unwrap();
    let engine = Arc::new(ControlledEngine {
        inner,
        block: AtomicBool::new(false),
        failure: Mutex::new(None),
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
        b"client-contract-signing-key".to_vec(),
    )
    .unwrap();
    put_airports(&store).await;
    Fixture {
        dir,
        path,
        store,
        engine,
        token,
        host,
    }
}

async fn put_airports(store: &EditStore) {
    let active = store.active_definition().await.unwrap();
    let edits = [
        ("BER", "Berlin", Some("DE")),
        ("FRA", "Frankfurt", Some("DE")),
        ("MUC", "Munich", None),
    ]
    .into_iter()
    .map(|(pk, name, country)| {
        let mut properties = BTreeMap::from([(
            "name".to_owned().try_into().unwrap(),
            PropertyValue::String(name.to_owned()),
        )]);
        if let Some(country) = country {
            properties.insert(
                "country".to_owned().try_into().unwrap(),
                PropertyValue::String(country.to_owned()),
            );
        }
        Edit::Object {
            type_name: "Airport".to_owned().try_into().unwrap(),
            pk: PropertyValue::String(pk.to_owned()),
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
    pub engine: Arc<ControlledEngine>,
    path: Utf8PathBuf,
    stop: oneshot::Sender<()>,
    task: tokio::task::JoinHandle<Result<(), HostError>>,
    _dir: tempfile::TempDir,
    store: Arc<EditStore>,
}

impl Fixture {
    pub async fn start(self) -> Running {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let (stop, rx) = oneshot::channel();
        let engine = Arc::clone(&self.engine);
        let task = tokio::spawn(Arc::new(self.host).serve_with_limits(
            listener,
            async move {
                let _ = rx.await;
            },
            ServerLimits::default(),
        ));
        Running {
            url,
            token: self.token,
            engine,
            path: self.path,
            stop,
            task,
            _dir: self.dir,
            store: self.store,
        }
    }
}

impl Running {
    pub async fn finish(self) {
        self.stop.send(()).unwrap();
        self.task.await.unwrap().unwrap();
        let pool = sqlx::SqlitePool::connect_with(sqlite_connect_options(&self.path))
            .await
            .unwrap();
        let _ = sqlx::query("SELECT request_id FROM of_read_audit")
            .fetch_all(&pool)
            .await
            .unwrap();
        pool.close().await;
    }

    pub async fn finish_actions(self) {
        self.stop.send(()).unwrap();
        self.task.await.unwrap().unwrap();
        let pool = sqlx::SqlitePool::connect_with(sqlite_connect_options(&self.path))
            .await
            .unwrap();
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM of_edit_log")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(count >= 2, "fixture setup plus action commit expected");
        pool.close().await;
    }
}

pub struct ControlledEngine {
    inner: SqliteEngine,
    pub block: AtomicBool,
    pub failure: Mutex<Option<EngineError>>,
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
        let result = if let Some(error) = self.failure.lock().unwrap().clone() {
            Err(error)
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
        self.inner.status().await
    }
}

pub fn _row_count(path: &Utf8Path) -> i64 {
    let _ = path;
    0
}
