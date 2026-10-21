//! End-to-end tests of `ontoforge import`: a file becomes system edits of an editOnly type or
//! link, and the command refuses what it must not write (STORE-06, D-11).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use camino::Utf8PathBuf;
use ontoforge_storage::store::{BackendSettings, EditStore, StoreSettings};

const AVIATION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/ontologies/aviation"
);
const AIRPORTS_CSV: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/data/aviation/airports.csv"
);

const SQLITE_CONFIG: &str = "[store]\nkind = \"sqlite\"\npath = \"edit.db\"\n";

fn ontoforge(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ontoforge"))
        .args(args)
        .env_remove("ONTOFORGE_PROFILE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn ontoforge")
}

/// A temporary directory holding `ontoforge-server.toml` with an SQLite store.
fn server_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("ontoforge-server.toml"), SQLITE_CONFIG).expect("config");
    dir
}

fn config_of(dir: &tempfile::TempDir) -> PathBuf {
    dir.path().join("ontoforge-server.toml")
}

fn text(path: &Path) -> &str {
    path.to_str().expect("utf-8 path")
}

fn reload(dir: &str, config: &Path) {
    let out = ontoforge(&["reload", dir, "--server-config", text(config)]);
    assert_eq!(out.status.code(), Some(0), "reload: {}", stderr(&out));
}

fn import(dir: &str, config: &Path, target: &[&str], file: &str, extra: &[&str]) -> Output {
    let mut args = vec!["import", dir, "--server-config", text(config)];
    args.extend_from_slice(target);
    args.extend_from_slice(&["--file", file]);
    args.extend_from_slice(extra);
    ontoforge(&args)
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

async fn open_store(dir: &tempfile::TempDir) -> EditStore {
    let path = Utf8PathBuf::from_path_buf(dir.path().join("edit.db")).expect("utf-8 path");
    EditStore::open(StoreSettings {
        backend: BackendSettings::Sqlite { path },
        lock_timeout: std::time::Duration::from_secs(5),
        probe_timeout: std::time::Duration::from_secs(5),
        trino_catalog: None,
    })
    .await
    .expect("open the edit store")
}

#[tokio::test(flavor = "multi_thread")]
async fn imports_airports_csv() {
    let dir = server_dir();
    let config = config_of(&dir);
    reload(AVIATION, &config);

    let out = import(AVIATION, &config, &["--type", "Airport"], AIRPORTS_CSV, &[]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert!(
        stderr(&out).contains("imported 5 object(s)"),
        "{}",
        stderr(&out)
    );

    let url = format!("sqlite://{}", dir.path().join("edit.db").display());
    let pool = sqlx::SqlitePool::connect(&url).await.expect("side pool");
    let rows: Vec<(String, bool)> =
        sqlx::query_as("SELECT code, city__set FROM of_obj_airport ORDER BY code")
            .fetch_all(&pool)
            .await
            .expect("airport rows");
    assert_eq!(rows.len(), 5, "{rows:?}");
    assert_eq!(
        rows.iter().filter(|(_, set)| !set).count(),
        1,
        "the row with an empty city leaves it unset: {rows:?}"
    );

    let store = open_store(&dir).await;
    let log = store.read_log(0, 100).await.expect("read_log");
    assert!(!log.is_empty());
    for record in &log {
        assert_eq!(record.actor, "system:import");
        assert_eq!(record.action, "system:import");
        assert_eq!(record.params["file"], "airports.csv");
    }
}
