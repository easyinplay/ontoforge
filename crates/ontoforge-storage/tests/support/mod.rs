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
    pub store: EditStore,
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
    TestStore { dir, path, store }
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
