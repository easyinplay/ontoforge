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
    assert!(
        stdout(&out).trim().is_empty(),
        "no diagnostics, in particular no warning that the directory differs from the active definition: {}",
        stdout(&out)
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

// ---------------------------------------------------------------------------------------------
// JSON, links, refusals and a failed batch
// ---------------------------------------------------------------------------------------------

const AIRPORTS_JSON: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/data/aviation/airports.json"
);

/// An ontology directory with two editOnly types and an editOnly many-to-many link, and a
/// one-to-one link whose foreign key is `Stand.gate`.
fn gates_ontology() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("ontology.yaml"), "apiName: demo\n").expect("manifest");
    std::fs::write(
        dir.path().join("defs.yaml"),
        "kind: ObjectType
apiName: Gate
backing: editOnly
primaryKey: code
properties:
  code: { type: string, required: true }
  name: { type: string, required: true }
---
kind: ObjectType
apiName: Stand
backing: editOnly
primaryKey: id
properties:
  id: { type: integer, required: true }
  label: { type: string }
  gate: { type: string }
---
kind: LinkType
apiName: gateStands
cardinality: manyToMany
from: { objectType: Gate, apiName: stands }
to: { objectType: Stand, apiName: gates }
backing: editOnly
---
kind: LinkType
apiName: gateStand
cardinality: oneToOne
from: { objectType: Gate, apiName: stand }
to: { objectType: Stand, apiName: assignedGate, foreignKey: gate }
",
    )
    .expect("definitions");
    dir
}

fn write(dir: &tempfile::TempDir, name: &str, content: &str) -> PathBuf {
    let path = dir.path().join(name);
    std::fs::write(&path, content).expect("write file");
    path
}

#[tokio::test(flavor = "multi_thread")]
async fn imports_airports_json() {
    let dir = server_dir();
    let config = config_of(&dir);
    reload(AVIATION, &config);
    let out = import(
        AVIATION,
        &config,
        &["--type", "Airport"],
        AIRPORTS_JSON,
        &[],
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert!(
        stderr(&out).contains("imported 5 object(s)"),
        "{}",
        stderr(&out)
    );
    let store = open_store(&dir).await;
    let log = store.read_log(0, 100).await.expect("read_log");
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].params["file"], "airports.json");
}

#[tokio::test(flavor = "multi_thread")]
async fn source_backed_type_is_rejected() {
    let dir = server_dir();
    let config = config_of(&dir);
    reload(AVIATION, &config);
    let data = tempfile::tempdir().expect("tempdir");
    let flights = write(&data, "flights.csv", "flightId,flightNumber\nF1,LH1\n");

    let out = import(
        AVIATION,
        &config,
        &["--type", "Flight"],
        text(&flights),
        &[],
    );
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr(&out));
    assert!(stdout(&out).contains("ONT-V27"), "{}", stdout(&out));
    assert!(stdout(&out).contains("source backed"), "{}", stdout(&out));

    let crew = write(&data, "crew.csv", "from,to\nF1,P1\n");
    let out = import(
        AVIATION,
        &config,
        &["--link", "flightCrew"],
        text(&crew),
        &[],
    );
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr(&out));
    assert!(stdout(&out).contains("ONT-V27"), "{}", stdout(&out));

    let store = open_store(&dir).await;
    assert!(store.read_log(0, 10).await.expect("read_log").is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn existing_objects_block_create_only() {
    let dir = server_dir();
    let config = config_of(&dir);
    reload(AVIATION, &config);
    let first = import(AVIATION, &config, &["--type", "Airport"], AIRPORTS_CSV, &[]);
    assert_eq!(first.status.code(), Some(0), "stderr: {}", stderr(&first));

    let again = import(AVIATION, &config, &["--type", "Airport"], AIRPORTS_CSV, &[]);
    assert_eq!(again.status.code(), Some(1), "stderr: {}", stderr(&again));
    assert!(stdout(&again).contains("ONT-V27"), "{}", stdout(&again));
    assert!(
        stdout(&again).contains("already exists"),
        "{}",
        stdout(&again)
    );

    let skip = import(
        AVIATION,
        &config,
        &["--type", "Airport"],
        AIRPORTS_CSV,
        &["--on-existing", "skip"],
    );
    assert_eq!(skip.status.code(), Some(0), "stderr: {}", stderr(&skip));
    assert!(
        stderr(&skip)
            .contains("imported 0 object(s) (created 0, updated 0, skipped 5) in 0 commit(s)"),
        "{}",
        stderr(&skip)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn link_import_and_partial_failure() {
    let ontology = gates_ontology();
    let ontology_dir = text(ontology.path()).to_owned();
    let dir = server_dir();
    let config = config_of(&dir);
    reload(&ontology_dir, &config);
    let data = tempfile::tempdir().expect("tempdir");

    let gates = write(&data, "gates.csv", "code,name\nA1,Alpha\nA2,Beta\n");
    let out = import(
        &ontology_dir,
        &config,
        &["--type", "Gate"],
        text(&gates),
        &[],
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));

    // Stand 4 claims gate A1 a second time, in the second batch of two.
    let stands = write(
        &data,
        "stands.csv",
        "id,label,gate\n1,S1,A1\n2,S2,A2\n3,S3,A3\n4,S4,A1\n",
    );
    let failed = import(
        &ontology_dir,
        &config,
        &["--type", "Stand"],
        text(&stands),
        &["--batch-size", "2"],
    );
    assert_eq!(failed.status.code(), Some(1), "stdout: {}", stdout(&failed));
    let message = stderr(&failed);
    assert!(message.contains("committed 1 batch(es)"), "{message}");
    assert!(message.contains("batch 2 failed"), "{message}");
    assert!(message.contains("--on-existing skip"), "{message}");

    // Corrected, the file finishes with skip.
    let fixed = write(
        &data,
        "stands_fixed.csv",
        "id,label,gate\n1,S1,A1\n2,S2,A2\n3,S3,A3\n4,S4,A4\n",
    );
    let resumed = import(
        &ontology_dir,
        &config,
        &["--type", "Stand"],
        text(&fixed),
        &["--batch-size", "2", "--on-existing", "skip"],
    );
    assert_eq!(
        resumed.status.code(),
        Some(0),
        "stderr: {}",
        stderr(&resumed)
    );
    assert!(
        stderr(&resumed).contains("created 2, updated 0, skipped 2"),
        "{}",
        stderr(&resumed)
    );

    let edges = write(&data, "edges.csv", "from,to\nA1,1\nA2,2\n");
    let linked = import(
        &ontology_dir,
        &config,
        &["--link", "gateStands"],
        text(&edges),
        &[],
    );
    assert_eq!(linked.status.code(), Some(0), "stderr: {}", stderr(&linked));
    assert!(
        stderr(&linked).contains("imported 2 link(s)"),
        "{}",
        stderr(&linked)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn command_line_mistakes_are_reported() {
    let dir = server_dir();
    let config = config_of(&dir);

    // Nothing is loaded in the store yet.
    let out = import(AVIATION, &config, &["--type", "Airport"], AIRPORTS_CSV, &[]);
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr(&out));
    assert!(
        stdout(&out).contains("ontoforge reload"),
        "the output tells to reload first: {}",
        stdout(&out)
    );

    reload(AVIATION, &config);
    let out = import(AVIATION, &config, &["--type", "Nope"], AIRPORTS_CSV, &[]);
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr(&out));
    assert!(stdout(&out).contains("ONT-V09"), "{}", stdout(&out));

    let out = import(
        AVIATION,
        &config,
        &["--type", "Airport"],
        AIRPORTS_CSV,
        &["--batch-size", "0"],
    );
    assert_eq!(out.status.code(), Some(2), "clap refuses a batch size of 0");
    let out = import(
        AVIATION,
        &config,
        &["--type", "Airport", "--link", "flightCrew"],
        AIRPORTS_CSV,
        &[],
    );
    assert_eq!(
        out.status.code(),
        Some(2),
        "--type and --link exclude each other"
    );
    let out = ontoforge(&[
        "import",
        AVIATION,
        "--server-config",
        text(&config),
        "--file",
        AIRPORTS_CSV,
    ]);
    assert_eq!(
        out.status.code(),
        Some(2),
        "one of --type and --link is needed"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn json_output_reports_the_result() {
    let dir = server_dir();
    let config = config_of(&dir);
    reload(AVIATION, &config);
    let out = import(
        AVIATION,
        &config,
        &["--type", "Airport"],
        AIRPORTS_CSV,
        &["--format", "json"],
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).expect("stdout is JSON");
    assert_eq!(value["result"]["rows"], 5);
    assert_eq!(value["result"]["created"], 5);
    assert_eq!(value["result"]["commits"].as_array().map(Vec::len), Some(1));
    assert!(
        value["diagnostics"]
            .as_array()
            .expect("diagnostics")
            .is_empty()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_directory_that_was_not_reloaded_is_flagged() {
    let ontology = gates_ontology();
    let ontology_dir = text(ontology.path()).to_owned();
    let dir = server_dir();
    let config = config_of(&dir);
    reload(&ontology_dir, &config);

    // The directory gains a property that the edit store does not know yet.
    let defs = ontology.path().join("defs.yaml");
    let changed = std::fs::read_to_string(&defs)
        .expect("definitions")
        .replacen(
            "  name: { type: string, required: true }\n",
            "  name: { type: string, required: true }\n  note: { type: string }\n",
            1,
        );
    std::fs::write(&defs, changed).expect("change definitions");

    let data = tempfile::tempdir().expect("tempdir");
    let gates = write(&data, "gates.csv", "code,name\nA1,Alpha\n");
    let out = import(
        &ontology_dir,
        &config,
        &["--type", "Gate"],
        text(&gates),
        &[],
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert!(
        stdout(&out).contains("differ from the ones the edit store follows"),
        "{}",
        stdout(&out)
    );

    // The edit store's definition decides what a file may hold: `note` is not a column yet.
    let with_note = write(&data, "gates2.csv", "code,name,note\nA2,Beta,x\n");
    let out = import(
        &ontology_dir,
        &config,
        &["--type", "Gate"],
        text(&with_note),
        &[],
    );
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr(&out));
    assert!(stdout(&out).contains("unknown column"), "{}", stdout(&out));
}
