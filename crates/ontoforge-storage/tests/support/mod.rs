//! Shared helpers of the storage integration tests.
#![allow(dead_code, reason = "each test target uses a subset of the helpers")]

use camino::Utf8PathBuf;
use ontoforge_schema::ir::Ontology;
use ontoforge_schema::load_from_memory;
use ontoforge_storage::store::{BackendSettings, EditStore, StoreSettings, sqlite_connect_options};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Row, SqlitePool};
use tempfile::TempDir;

/// An SQLite edit store in a temporary directory. The directory lives as long as the value.
pub struct TestStore {
    pub dir: TempDir,
    pub path: Utf8PathBuf,
    pub store: std::sync::Arc<EditStore>,
}

impl TestStore {
    /// A second connection to the same file, for inspecting or preparing the database from
    /// outside the store.
    pub async fn side_pool(&self) -> SqlitePool {
        SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(sqlite_connect_options(&self.path))
            .await
            .expect("side pool")
    }

    /// Opens a new `EditStore` on the same file.
    pub async fn reopen(&self) -> EditStore {
        EditStore::open(settings(&self.path)).await.expect("reopen")
    }

    /// Column names of `table`, empty when it does not exist.
    pub async fn columns(&self, table: &str) -> Vec<String> {
        let pool = self.side_pool().await;
        let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
            "PRAGMA table_info(\"{table}\")"
        )))
        .fetch_all(&pool)
        .await
        .expect("table_info");
        rows.iter()
            .map(|row| row.get::<String, _>("name"))
            .collect()
    }

    /// `of_def_state.active_def_version`.
    pub async fn active_version(&self) -> i64 {
        let pool = self.side_pool().await;
        sqlx::query_scalar("SELECT active_def_version FROM of_def_state")
            .fetch_one(&pool)
            .await
            .expect("active version")
    }

    /// Number of rows of `of_def_version`.
    pub async fn version_rows(&self) -> i64 {
        let pool = self.side_pool().await;
        sqlx::query_scalar("SELECT count(*) FROM of_def_version")
            .fetch_one(&pool)
            .await
            .expect("version rows")
    }
}

fn settings(path: &Utf8PathBuf) -> StoreSettings {
    StoreSettings {
        backend: BackendSettings::Sqlite { path: path.clone() },
        lock_timeout: std::time::Duration::from_secs(5),
        probe_timeout: std::time::Duration::from_secs(5),
        trino_catalog: None,
    }
}

pub async fn sqlite_store() -> TestStore {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = Utf8PathBuf::from_path_buf(dir.path().join("edit.db")).expect("utf8 path");
    let store = EditStore::open(settings(&path)).await.expect("open store");
    TestStore {
        dir,
        path,
        store: std::sync::Arc::new(store),
    }
}

/// Loads and checks an ontology held in memory. A manifest is added when `files` has none.
pub fn ontology_from(files: &[(&str, &str)]) -> Ontology {
    let mut all: Vec<(&str, &str)> = files.to_vec();
    if !all.iter().any(|(path, _)| *path == "ontology.yaml") {
        all.push(("ontology.yaml", "apiName: demo\n"));
    }
    let out = load_from_memory(&all);
    let Some(ontology) = out.ontology else {
        panic!("definition does not load: {:#?}", out.diagnostics);
    };
    let problems = ontoforge_expr::check_ontology(&ontology, &out.sources);
    assert!(
        problems.is_empty(),
        "definition does not check: {problems:#?}"
    );
    ontology
}

// ---------------------------------------------------------------------------------------------
// PostgreSQL
// ---------------------------------------------------------------------------------------------

/// What keeps an embedded server alive. The server stops when the last holder is dropped.
type ServerGuard = Option<std::sync::Arc<dyn std::any::Any + Send + Sync>>;

/// A PostgreSQL edit store on a schema pair of its own, so tests never see each other's tables.
pub struct PgTestStore {
    pub store: std::sync::Arc<EditStore>,
    pub schema_edit: String,
    pub schema_sys: String,
    url: String,
    side: sqlx::PgPool,
    _server: ServerGuard,
}

impl PgTestStore {
    /// The connection string of the test server (a secret; never print it).
    pub fn url(&self) -> &str {
        &self.url
    }

    /// A connection outside the store, for inspecting the database.
    pub fn side(&self) -> &sqlx::PgPool {
        &self.side
    }

    /// Opens a second `EditStore` on the same schemas, as another process would, with the given
    /// lock timeout. It loads the definition that is active in the database now and does not
    /// follow later reloads of `store`.
    pub async fn open_another(&self, lock_timeout: std::time::Duration) -> EditStore {
        EditStore::open(self.settings(lock_timeout))
            .await
            .expect("open a second PostgreSQL store")
    }

    fn settings(&self, lock_timeout: std::time::Duration) -> StoreSettings {
        StoreSettings {
            backend: BackendSettings::Postgres {
                url: secrecy::SecretString::from(self.url.clone()),
                schema_edit: self.schema_edit.clone(),
                schema_sys: self.schema_sys.clone(),
            },
            lock_timeout,
            probe_timeout: std::time::Duration::from_secs(5),
            trino_catalog: None,
        }
    }

    /// Table names of `schema`, sorted.
    pub async fn tables(&self, schema: &str) -> Vec<String> {
        sqlx::query_scalar::<_, String>(
            "SELECT CAST(table_name AS text) FROM information_schema.tables \
             WHERE CAST(table_schema AS text) = $1 ORDER BY 1",
        )
        .bind(schema)
        .fetch_all(&self.side)
        .await
        .expect("list tables")
    }

    /// Column names of `schema.table`, empty when it does not exist.
    pub async fn columns(&self, schema: &str, table: &str) -> Vec<String> {
        sqlx::query_scalar::<_, String>(
            "SELECT CAST(column_name AS text) FROM information_schema.columns \
             WHERE CAST(table_schema AS text) = $1 AND CAST(table_name AS text) = $2 \
             ORDER BY ordinal_position",
        )
        .bind(schema)
        .bind(table)
        .fetch_all(&self.side)
        .await
        .expect("list columns")
    }

    /// The data type PostgreSQL reports for `schema.table.column`.
    pub async fn data_type(&self, schema: &str, table: &str, column: &str) -> String {
        sqlx::query_scalar::<_, String>(
            "SELECT CAST(data_type AS text) FROM information_schema.columns \
             WHERE CAST(table_schema AS text) = $1 AND CAST(table_name AS text) = $2 \
             AND CAST(column_name AS text) = $3",
        )
        .bind(schema)
        .bind(table)
        .bind(column)
        .fetch_one(&self.side)
        .await
        .expect("column type")
    }

    /// The collation of `schema.table.column`, `None` for the column default.
    pub async fn collation(&self, schema: &str, table: &str, column: &str) -> Option<String> {
        sqlx::query_scalar::<_, Option<String>>(
            "SELECT CAST(collation_name AS text) FROM information_schema.columns \
             WHERE CAST(table_schema AS text) = $1 AND CAST(table_name AS text) = $2 \
             AND CAST(column_name AS text) = $3",
        )
        .bind(schema)
        .bind(table)
        .bind(column)
        .fetch_one(&self.side)
        .await
        .expect("column collation")
    }

    /// `of_def_state.active_def_version` of the system schema.
    pub async fn active_version(&self) -> i64 {
        let sql = format!(
            "SELECT active_def_version FROM \"{}\".of_def_state",
            self.schema_sys
        );
        sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
            .fetch_one(&self.side)
            .await
            .expect("active version")
    }

    /// Number of rows of `of_def_version` in the system schema.
    pub async fn version_rows(&self) -> i64 {
        let sql = format!(
            "SELECT count(*) FROM \"{}\".of_def_version",
            self.schema_sys
        );
        sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
            .fetch_one(&self.side)
            .await
            .expect("version rows")
    }
}

/// An edit store on PostgreSQL, from `ONTOFORGE_PG_URL` or, with `--features embedded-pg`, from
/// a `postgresql_embedded` server shared by the tests of one binary that run at the same time.
///
/// Panics when neither exists, so that an ignored test run with `--ignored` fails loudly instead
/// of passing without having touched PostgreSQL (RESEARCH Pitfall 5, as for Trino).
pub async fn pg_store() -> PgTestStore {
    let (url, server) = pg_url().await;
    let unique = ulid::Ulid::generate().to_string().to_lowercase();
    let schema_edit = format!("of_t_{unique}_edit");
    let schema_sys = format!("of_t_{unique}_sys");
    let settings = StoreSettings {
        backend: BackendSettings::Postgres {
            url: secrecy::SecretString::from(url.clone()),
            schema_edit: schema_edit.clone(),
            schema_sys: schema_sys.clone(),
        },
        lock_timeout: std::time::Duration::from_secs(5),
        probe_timeout: std::time::Duration::from_secs(5),
        trino_catalog: None,
    };
    let store = EditStore::open(settings)
        .await
        .expect("open PostgreSQL store");
    let side = sqlx::PgPool::connect(&url).await.expect("side pool");
    PgTestStore {
        store: std::sync::Arc::new(store),
        schema_edit,
        schema_sys,
        url,
        side,
        _server: server,
    }
}

async fn pg_url() -> (String, ServerGuard) {
    match std::env::var("ONTOFORGE_PG_URL") {
        Ok(url) if !url.trim().is_empty() => (url, None),
        _ => embedded_url().await,
    }
}

#[cfg(feature = "embedded-pg")]
mod embedded {
    use std::sync::{Arc, Weak};

    use postgresql_embedded::{PostgreSQL, Settings};

    /// A running embedded server. `PostgreSQL` stops the server and removes its data when
    /// dropped.
    pub struct Server {
        _pg: PostgreSQL,
        pub url: String,
    }

    static SERVER: tokio::sync::Mutex<Weak<Server>> = tokio::sync::Mutex::const_new(Weak::new());

    /// The running server, started when no test holds one.
    pub async fn acquire() -> Arc<Server> {
        let mut slot = SERVER.lock().await;
        if let Some(server) = slot.upgrade() {
            return server;
        }
        // initdb and startup can take longer than the default 5 s on a loaded machine.
        let settings = Settings {
            timeout: Some(std::time::Duration::from_secs(180)),
            ..Settings::default()
        };
        let mut pg = PostgreSQL::new(settings);
        pg.setup().await.expect("set up embedded PostgreSQL");
        pg.start().await.expect("start embedded PostgreSQL");
        let url = pg.settings().url("postgres");
        let server = Arc::new(Server { _pg: pg, url });
        *slot = Arc::downgrade(&server);
        server
    }
}

#[cfg(feature = "embedded-pg")]
async fn embedded_url() -> (String, ServerGuard) {
    let server = embedded::acquire().await;
    (server.url.clone(), Some(server))
}

#[cfg(not(feature = "embedded-pg"))]
#[allow(
    clippy::unused_async,
    reason = "same signature as the embedded-pg variant"
)]
async fn embedded_url() -> (String, ServerGuard) {
    panic!(
        "{}",
        concat!(
            "ONTOFORGE_PG_URL is not set and the `embedded-pg` feature is off: the PostgreSQL ",
            "tests need a PostgreSQL server. Set ONTOFORGE_PG_URL=postgres://user:password@host/db, ",
            "or run with `--features embedded-pg` to download and start a local one"
        )
    )
}

// ---------------------------------------------------------------------------------------------
// Writers that race a definition switch (plan 03-18)
// ---------------------------------------------------------------------------------------------

/// A commit that sets the string `name` of the gate `code`, validated against `def_version`.
pub fn gate_name_commit(
    def_version: i64,
    code: &str,
    name: &str,
) -> ontoforge_storage::commit::Commit {
    use ontoforge_storage::commit::{Actor, Commit, Edit, ObjectOp};
    use ontoforge_storage::value::PropertyValue;
    let values = std::collections::BTreeMap::from([(
        ontoforge_schema::ir::PropertyName::try_from("name".to_owned()).expect("property name"),
        PropertyValue::String(name.to_owned()),
    )]);
    Commit {
        actor: Actor::Principal("test|alice".to_owned()),
        action: "setGate".to_owned(),
        def_version,
        params: serde_json::json!({ "code": code }),
        edits: vec![Edit::Object {
            type_name: ontoforge_schema::ir::ObjectTypeName::try_from("Gate".to_owned())
                .expect("type name"),
            pk: PropertyValue::String(code.to_owned()),
            op: ObjectOp::Set(values),
        }],
    }
}

/// What a writer saw while a definition switch ran.
#[derive(Debug, Default)]
pub struct WriterStats {
    pub committed: usize,
    /// Commits refused as `DefinitionStale`.
    pub stale: usize,
}

/// Commits gate rows through `store`, each time against the definition version the store holds,
/// until `switched` is set and ten more commits have gone through. A commit may only succeed or
/// be refused as stale; anything else fails the test.
pub async fn write_gates_across_a_switch(
    store: std::sync::Arc<EditStore>,
    switched: std::sync::Arc<std::sync::atomic::AtomicBool>,
    tag: &str,
) -> WriterStats {
    use std::sync::atomic::Ordering;
    let mut stats = WriterStats::default();
    let mut after_switch = 0;
    let mut after_switch_attempts = 0;
    let mut i = 0_usize;
    loop {
        let version = store
            .active_definition()
            .await
            .expect("a definition is loaded")
            .def_version;
        let committed = match store
            .commit(gate_name_commit(version, &format!("{tag}{i}"), "North"))
            .await
        {
            Ok(_) => {
                stats.committed += 1;
                true
            }
            Err(ontoforge_storage::commit::EditError::DefinitionStale { .. }) => {
                stats.stale += 1;
                false
            }
            Err(other) => panic!("a commit may only succeed or be stale, got {other:?}"),
        };
        i += 1;
        if switched.load(Ordering::SeqCst) {
            after_switch_attempts += 1;
            assert!(
                after_switch_attempts <= 100,
                "writer never made ten commits after switch: {stats:?}"
            );
            if committed {
                after_switch += 1;
                if after_switch >= 10 {
                    return stats;
                }
            }
        }
        tokio::task::yield_now().await;
    }
}

/// The definition version of every log record in `seq` order never goes back: no write under the
/// old definition is ordered after a write under the new one.
pub async fn assert_log_versions_never_go_back(store: &EditStore) -> Vec<i64> {
    let mut versions = Vec::new();
    let mut after = 0;
    loop {
        let page = store.read_log(after, 500).await.expect("read_log");
        let Some(last) = page.last() else { break };
        after = last.seq;
        versions.extend(page.iter().map(|record| record.def_version));
    }
    assert!(
        versions.windows(2).all(|pair| pair[0] <= pair[1]),
        "log records under an old definition after one under the new: {versions:?}"
    );
    versions
}
