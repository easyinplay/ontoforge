//! Bulk import of editOnly data (plan 03-19: STORE-06, D-11).
//!
//! Every batch of an import is one system commit; these tests read the effect from the edit log
//! and from the typed edit tables.

mod support;

use camino::{Utf8Path, Utf8PathBuf};
use ontoforge_schema::diag::codes;
use ontoforge_schema::ir::{ObjectTypeName, Ontology};
use ontoforge_schema::load_dir;
use ontoforge_storage::import::{ImportError, ImportOptions, ImportTarget, import};
use ontoforge_storage::log::{EditOp, LogRecord};
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::store::EditStore;
use support::{TestStore, ontology_from, sqlite_store};

const AVIATION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/ontologies/aviation"
);
const AIRPORTS_CSV: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/data/aviation/airports.csv"
);

fn aviation() -> Ontology {
    let out = load_dir(Utf8Path::new(AVIATION));
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    out.ontology.expect("aviation ontology")
}

fn object(name: &str) -> ImportTarget {
    ImportTarget::Object(ObjectTypeName::try_from(name.to_owned()).expect("type name"))
}

/// An SQLite store with `ontology` reloaded.
async fn store_with(ontology: &Ontology) -> TestStore {
    let t = sqlite_store().await;
    reload(&t.store, ontology, &ReloadOptions::default())
        .await
        .expect("reload");
    t
}

/// A file named `name` with `content` in a directory that lives as long as the returned guard.
fn file_with(name: &str, content: &str) -> (tempfile::TempDir, Utf8PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = Utf8PathBuf::from_path_buf(dir.path().join(name)).expect("utf-8 path");
    std::fs::write(&path, content).expect("write import file");
    (dir, path)
}

async fn whole_log(store: &EditStore) -> Vec<LogRecord> {
    store.read_log(0, 500).await.expect("read_log")
}

/// A place with a geopoint and a decimal, to see the cell conversions.
fn places() -> Ontology {
    ontology_from(&[(
        "defs.yaml",
        "kind: ObjectType\napiName: Place\nbacking: editOnly\nprimaryKey: code\nproperties:\n  \
         code: { type: string, required: true }\n  location: { type: geopoint }\n  \
         price: { type: decimal, precision: 8, scale: 2 }\n",
    )])
}

#[tokio::test(flavor = "multi_thread")]
async fn airports_csv_is_imported_as_system_edits() {
    let t = store_with(&aviation()).await;
    let report = import(
        &t.store,
        object("Airport"),
        Utf8Path::new(AIRPORTS_CSV),
        &ImportOptions::default(),
    )
    .await
    .expect("import");
    assert_eq!(report.rows, 5);
    assert_eq!(report.created, 5);
    assert_eq!((report.updated, report.skipped), (0, 0));
    assert_eq!(report.commits.len(), 1);

    let pool = t.side_pool().await;
    let rows: Vec<(String, Option<String>, bool)> =
        sqlx::query_as("SELECT code, city, city__set FROM of_obj_airport ORDER BY code")
            .fetch_all(&pool)
            .await
            .expect("airport rows");
    assert_eq!(rows.len(), 5, "{rows:?}");
    let helmsgate = rows.iter().find(|row| row.0 == "HLM").expect("HLM");
    assert!(!helmsgate.2, "an empty cell is not set: {helmsgate:?}");
    assert_eq!(helmsgate.1, None);

    let log = whole_log(&t.store).await;
    assert_eq!(log.len(), 1);
    let record = &log[0];
    assert_eq!(record.actor, "system:import");
    assert_eq!(record.action, "system:import");
    assert_eq!(record.params["file"], "airports.csv");
    assert_eq!(record.params["rows"], 5);
    assert_eq!(record.entries.len(), 5);
    assert!(record.entries.iter().all(|e| e.op == EditOp::Set));
}

#[tokio::test(flavor = "multi_thread")]
async fn batches_follow_batch_size() {
    let t = store_with(&places()).await;
    let mut content = String::from("code\n");
    // A type with only a key cannot be created by a set; the fixture adds a cell below.
    content.clear();
    content.push_str("code,price\n");
    for i in 0..7 {
        content.push_str(&format!("P{i},{i}.5\n"));
    }
    let (_dir, path) = file_with("places.csv", &content);
    let opts = ImportOptions {
        batch_size: 3,
        ..ImportOptions::default()
    };
    let report = import(&t.store, object("Place"), &path, &opts)
        .await
        .expect("import");
    assert_eq!(report.rows, 7);
    assert_eq!(report.commits.len(), 3);

    let log = whole_log(&t.store).await;
    let sizes: Vec<usize> = log.iter().map(|r| r.entries.len()).collect();
    assert_eq!(sizes, [3, 3, 1]);
    let batches: Vec<i64> = log
        .iter()
        .map(|r| r.params["batch"].as_i64().expect("batch"))
        .collect();
    assert_eq!(batches, [0, 1, 2]);
    assert_eq!(
        log.iter().map(|r| r.seq).collect::<Vec<_>>(),
        report.commits
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn geopoint_and_decimal_cells() {
    let t = store_with(&places()).await;
    let (_dir, path) = file_with("p.csv", "code,location,price\nP1,\"50.03,8.57\",1.5\n");
    import(&t.store, object("Place"), &path, &ImportOptions::default())
        .await
        .expect("import");
    let log = whole_log(&t.store).await;
    let payload = log[0].entries[0].payload.as_ref().expect("payload");
    assert_eq!(payload["location"]["lat"], 50.03);
    assert_eq!(payload["location"]["lon"], 8.57);
    assert_eq!(payload["price"], "1.50", "a decimal is padded to its scale");

    for (name, content) in [
        ("lat.csv", "code,location\nP2,\"91,0\"\n"),
        ("round.csv", "code,price\nP3,1.555\n"),
    ] {
        let (_dir, path) = file_with(name, content);
        let err = import(&t.store, object("Place"), &path, &ImportOptions::default())
            .await
            .expect_err(name);
        let ImportError::Data(found) = err else {
            panic!("{name}: expected a data problem, got {err:?}");
        };
        assert!(
            found.iter().all(|d| d.code == codes::V27),
            "{name}: {found:#?}"
        );
        assert!(
            found.iter().any(|d| d.line == 2 && d.file.ends_with(name)),
            "{name}: the problem names file and line: {found:#?}"
        );
    }
    assert_eq!(
        whole_log(&t.store).await.len(),
        1,
        "nothing else was written"
    );
}
