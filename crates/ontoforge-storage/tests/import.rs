//! Bulk import of editOnly data (plan 03-19: STORE-06, D-11).
//!
//! Every batch of an import is one system commit; these tests read the effect from the edit log
//! and from the typed edit tables.

mod support;

use camino::{Utf8Path, Utf8PathBuf};
use ontoforge_schema::diag::{Diagnostic, codes};
use ontoforge_schema::ir::{LinkTypeName, ObjectTypeName, Ontology};
use ontoforge_schema::load_dir;
use ontoforge_storage::commit::{Actor, Commit, Edit, EditError, ObjectOp};
use ontoforge_storage::import::{
    ImportError, ImportOptions, ImportReport, ImportTarget, OnExisting, import,
};
use ontoforge_storage::log::{EditOp, LogRecord};
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::store::EditStore;
use ontoforge_storage::value::PropertyValue;
use support::{TestStore, ontology_from, sqlite_store};

const AVIATION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/ontologies/aviation"
);
const AIRPORTS_CSV: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/data/aviation/airports.csv"
);
const AIRPORTS_JSON: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/data/aviation/airports.json"
);

fn aviation() -> Ontology {
    let out = load_dir(Utf8Path::new(AVIATION));
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    out.ontology.expect("aviation ontology")
}

fn object(name: &str) -> ImportTarget {
    ImportTarget::Object(ObjectTypeName::try_from(name.to_owned()).expect("type name"))
}

fn link(name: &str) -> ImportTarget {
    ImportTarget::Link(LinkTypeName::try_from(name.to_owned()).expect("link name"))
}

fn on_existing(mode: OnExisting) -> ImportOptions {
    ImportOptions {
        on_existing: mode,
        ..ImportOptions::default()
    }
}

/// The problems of a refused import.
fn problems(result: Result<ImportReport, ImportError>) -> Vec<Diagnostic> {
    match result {
        Err(ImportError::Data(found)) => found,
        other => panic!("expected a data problem, got {other:?}"),
    }
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

/// Imports the airports of `content` as a file called `name`.
async fn import_airports(
    t: &TestStore,
    name: &str,
    content: &str,
    opts: &ImportOptions,
) -> Result<ImportReport, ImportError> {
    let (_dir, path) = file_with(name, content);
    import(&t.store, object("Airport"), &path, opts).await
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

/// Two editOnly types with an editOnly many-to-many link, a source backed link, and a one to one
/// link whose foreign key is `Stand.gate`.
const GATES_AND_STANDS: &str = "kind: ObjectType
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
apiName: gateSlots
cardinality: manyToMany
from: { objectType: Gate, apiName: slots }
to: { objectType: Stand, apiName: slotGates }
backing: source
---
kind: LinkType
apiName: gateStand
cardinality: oneToOne
from: { objectType: Gate, apiName: stand }
to: { objectType: Stand, apiName: assignedGate, foreignKey: gate }
";

fn gates_and_stands() -> Ontology {
    ontology_from(&[("defs.yaml", GATES_AND_STANDS)])
}

async fn count(t: &TestStore, table: &str) -> i64 {
    let pool = t.side_pool().await;
    sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
        .fetch_one(&pool)
        .await
        .expect("count")
}

/// Every airport row as the edit table holds it.
type AirportRow = (
    String,
    Option<String>,
    bool,
    Option<String>,
    bool,
    Option<String>,
    bool,
    Option<String>,
    bool,
    i64,
);

async fn airport_rows(t: &TestStore) -> Vec<AirportRow> {
    let pool = t.side_pool().await;
    sqlx::query_as(
        "SELECT code, name, name__set, city, city__set, country, country__set, location, \
         location__set, of_version FROM of_obj_airport ORDER BY code",
    )
    .fetch_all(&pool)
    .await
    .expect("airport rows")
}

// ---------------------------------------------------------------------------------------------
// The tracer: a CSV becomes system edits
// ---------------------------------------------------------------------------------------------

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
    let rows: Vec<String> = (0..7).map(|i| format!("P{i},{i}.5")).collect();
    let content = format!("code,price\n{}\n", rows.join("\n"));
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

/// A lot with a wide decimal, to see that a decimal is never rounded on its way in.
fn lots() -> Ontology {
    ontology_from(&[(
        "defs.yaml",
        "kind: ObjectType\napiName: Lot\nbacking: editOnly\nprimaryKey: code\nproperties:\n  \
         code: { type: string, required: true }\n  \
         amount: { type: decimal, precision: 38, scale: 10 }\n",
    )])
}

#[tokio::test(flavor = "multi_thread")]
async fn json_decimals_are_never_rounded() {
    let t = store_with(&lots()).await;
    // A JSON number with a fraction passes through an f64 and would lose digits.
    let (_dir, path) = file_with(
        "lots.json",
        r#"[{"code": "L1", "amount": 12345678.1234567891}]"#,
    );
    let found = problems(import(&t.store, object("Lot"), &path, &ImportOptions::default()).await);
    assert!(found.iter().all(|d| d.code == codes::V27), "{found:#?}");
    assert!(
        found
            .iter()
            .any(|d| d.message.contains("amount") && d.message.contains("as a string")),
        "{found:#?}"
    );
    assert!(whole_log(&t.store).await.is_empty(), "nothing was written");

    let (_dir, path) = file_with(
        "lots2.json",
        r#"[{"code": "L2", "amount": "12345678.1234567891"}, {"code": "L3", "amount": 12345678901234567890}]"#,
    );
    let report = import(&t.store, object("Lot"), &path, &ImportOptions::default())
        .await
        .expect("import");
    assert_eq!(report.created, 2);
    let log = whole_log(&t.store).await;
    let amounts: Vec<(&str, &str)> = log[0]
        .entries
        .iter()
        .map(|e| {
            let pk = e.pk.as_str().expect("pk");
            let payload = e.payload.as_ref().expect("payload");
            (pk, payload["amount"].as_str().expect("amount"))
        })
        .collect();
    assert_eq!(
        amounts,
        [
            ("L2", "12345678.1234567891"),
            ("L3", "12345678901234567890.0000000000")
        ]
    );
}

// ---------------------------------------------------------------------------------------------
// JSON and links
// ---------------------------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn json_import_matches_csv() {
    let from_csv = store_with(&aviation()).await;
    import(
        &from_csv.store,
        object("Airport"),
        Utf8Path::new(AIRPORTS_CSV),
        &ImportOptions::default(),
    )
    .await
    .expect("csv import");
    let from_json = store_with(&aviation()).await;
    let report = import(
        &from_json.store,
        object("Airport"),
        Utf8Path::new(AIRPORTS_JSON),
        &ImportOptions::default(),
    )
    .await
    .expect("json import");
    assert_eq!(report.created, 5);

    let csv_rows = airport_rows(&from_csv).await;
    assert_eq!(csv_rows.len(), 5);
    assert_eq!(airport_rows(&from_json).await, csv_rows);
}

#[tokio::test(flavor = "multi_thread")]
async fn editonly_link_import() {
    let t = store_with(&gates_and_stands()).await;
    let (_g, gates) = file_with("gates.csv", "code,name\nA1,Alpha\nA2,Beta\n");
    let (_s, stands) = file_with("stands.csv", "id,label\n1,one\n2,two\n3,three\n");
    import(&t.store, object("Gate"), &gates, &ImportOptions::default())
        .await
        .expect("gates");
    import(
        &t.store,
        object("Stand"),
        &stands,
        &ImportOptions::default(),
    )
    .await
    .expect("stands");
    let (_l, edges) = file_with("edges.csv", "from,to\nA1,1\nA1,2\nA2,3\n");
    let report = import(
        &t.store,
        link("gateStands"),
        &edges,
        &ImportOptions::default(),
    )
    .await
    .expect("link import");
    assert_eq!((report.rows, report.created), (3, 3));
    assert_eq!(count(&t, "of_lnk_gate_stands").await, 3);

    let log = whole_log(&t.store).await;
    let last = log.last().expect("a record");
    assert_eq!(last.actor, "system:import");
    assert_eq!(last.params["file"], "edges.csv");
    assert_eq!(last.entries.len(), 3);
    assert!(last.entries.iter().all(|e| e.op == EditOp::LinkAdd));
    assert!(last.entries.iter().all(|e| e.type_name == "gateStands"));

    // The same edges again: refused by default, skipped on request, never written twice.
    let found = problems(
        import(
            &t.store,
            link("gateStands"),
            &edges,
            &ImportOptions::default(),
        )
        .await,
    );
    assert_eq!(found.len(), 3, "{found:#?}");
    let log_len = whole_log(&t.store).await.len();
    let report = import(
        &t.store,
        link("gateStands"),
        &edges,
        &on_existing(OnExisting::Skip),
    )
    .await
    .expect("skip");
    assert_eq!(
        (report.created, report.skipped, report.commits.len()),
        (0, 3, 0)
    );
    assert_eq!(whole_log(&t.store).await.len(), log_len);

    // A key that is not an integer is a problem of the file, with its line.
    let (_b, bad) = file_with("bad.csv", "from,to\nA1,x\n");
    let found = problems(
        import(
            &t.store,
            link("gateStands"),
            &bad,
            &ImportOptions::default(),
        )
        .await,
    );
    assert!(
        found.iter().any(|d| d.line == 2 && d.code == codes::V27),
        "{found:#?}"
    );
    let (_c, columns) = file_with("columns.csv", "from,other\nA1,1\n");
    let found = problems(
        import(
            &t.store,
            link("gateStands"),
            &columns,
            &ImportOptions::default(),
        )
        .await,
    );
    assert!(
        found.iter().any(|d| d.message.contains("`to`")),
        "{found:#?}"
    );
}

// ---------------------------------------------------------------------------------------------
// What may be written
// ---------------------------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn source_backed_type_is_rejected() {
    let t = store_with(&aviation()).await;
    let (_dir, path) = file_with("flights.csv", "flightId,flightNumber\nF1,LH1\n");
    let result = import(&t.store, object("Flight"), &path, &ImportOptions::default()).await;
    assert!(
        matches!(result, Err(ImportError::NotEditOnly(_))),
        "{result:?}"
    );

    let (_dir, edges) = file_with("crew.csv", "from,to\nF1,P1\n");
    let result = import(
        &t.store,
        link("flightCrew"),
        &edges,
        &ImportOptions::default(),
    )
    .await;
    assert!(
        matches!(result, Err(ImportError::NotEditOnly(_))),
        "{result:?}"
    );

    // A link that is not many to many has no rows of its own either.
    let result = import(
        &t.store,
        link("flightDepartureAirport"),
        &edges,
        &ImportOptions::default(),
    )
    .await;
    assert!(
        matches!(result, Err(ImportError::NotEditOnly(_))),
        "{result:?}"
    );

    let result = import(
        &t.store,
        object("Nothing"),
        &path,
        &ImportOptions::default(),
    )
    .await;
    assert!(
        matches!(result, Err(ImportError::Edit(EditError::UnknownType(_)))),
        "{result:?}"
    );
    assert!(whole_log(&t.store).await.is_empty(), "nothing was written");
}

#[tokio::test(flavor = "multi_thread")]
async fn existing_objects_block_create_only() {
    let t = store_with(&aviation()).await;
    import(
        &t.store,
        object("Airport"),
        Utf8Path::new(AIRPORTS_CSV),
        &ImportOptions::default(),
    )
    .await
    .expect("first import");
    let before = airport_rows(&t).await;
    let more = "code,name\nNRD,Northfield Renamed\nXYZ,Brand New\nLKV,Lakeview Renamed\n";

    let found =
        problems(import_airports(&t, "more.csv", more, &on_existing(OnExisting::Error)).await);
    let named: Vec<&str> = found
        .iter()
        .filter(|d| d.code == codes::V27 && d.message.contains("already exists"))
        .map(|d| d.message.as_str())
        .collect();
    assert_eq!(named.len(), 2, "{found:#?}");
    assert!(named.iter().any(|m| m.contains("NRD")) && named.iter().any(|m| m.contains("LKV")));
    assert_eq!(airport_rows(&t).await, before, "nothing was written");
    assert_eq!(whole_log(&t.store).await.len(), 1);

    let report = import_airports(&t, "more.csv", more, &on_existing(OnExisting::Skip))
        .await
        .expect("skip");
    assert_eq!((report.created, report.updated, report.skipped), (1, 0, 2));
    assert_eq!(report.commits.len(), 1);
    let rows = airport_rows(&t).await;
    assert_eq!(rows.len(), 6);
    assert!(rows.iter().all(|r| r.9 == 1), "{rows:?}");

    let report = import_airports(&t, "more.csv", more, &on_existing(OnExisting::Update))
        .await
        .expect("update");
    assert_eq!((report.created, report.updated, report.skipped), (0, 3, 0));
    let rows = airport_rows(&t).await;
    let nrd = rows.iter().find(|r| r.0 == "NRD").expect("NRD");
    assert_eq!(nrd.1.as_deref(), Some("Northfield Renamed"));
    assert_eq!(nrd.9, 2, "an update is one more version");
    assert_eq!(
        nrd.3.as_deref(),
        Some("Northfield"),
        "a column the file does not have keeps its value"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn tombstoned_object_is_never_revived() {
    let t = store_with(&aviation()).await;
    import(
        &t.store,
        object("Airport"),
        Utf8Path::new(AIRPORTS_CSV),
        &ImportOptions::default(),
    )
    .await
    .expect("import");
    let version = t
        .store
        .active_definition()
        .await
        .expect("active")
        .def_version;
    t.store
        .commit(Commit {
            actor: Actor::Principal("test|alice".to_owned()),
            action: "closeAirport".to_owned(),
            def_version: version,
            params: serde_json::json!({}),
            edits: vec![Edit::Object {
                type_name: ObjectTypeName::try_from("Airport".to_owned()).expect("type"),
                pk: PropertyValue::String("NRD".to_owned()),
                op: ObjectOp::Tombstone,
            }],
        })
        .await
        .expect("tombstone");
    let before = airport_rows(&t).await;
    let log_before = whole_log(&t.store).await.len();
    let again = "code,name\nNRD,Back Again\nXYZ,Brand New\n";

    for mode in [OnExisting::Update, OnExisting::Error] {
        let found = problems(import_airports(&t, "again.csv", again, &on_existing(mode)).await);
        assert!(
            found
                .iter()
                .any(|d| d.code == codes::V27 && d.message.contains("NRD")),
            "{mode:?}: {found:#?}"
        );
    }
    assert_eq!(
        airport_rows(&t).await,
        before,
        "not even the new row was written"
    );
    assert_eq!(whole_log(&t.store).await.len(), log_before);

    let report = import_airports(&t, "again.csv", again, &on_existing(OnExisting::Skip))
        .await
        .expect("skip");
    assert_eq!((report.created, report.skipped), (1, 1));
    let rows = airport_rows(&t).await;
    let nrd = rows.iter().find(|r| r.0 == "NRD").expect("NRD");
    assert_eq!(
        nrd.1.as_deref(),
        Some("Northfield Regional"),
        "the old name is still there"
    );
}

// ---------------------------------------------------------------------------------------------
// The file is checked as a whole
// ---------------------------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn file_level_validation() {
    let t = store_with(&aviation()).await;
    let plain = ImportOptions::default();

    let found =
        problems(import_airports(&t, "unknown.csv", "code,name,bogus\nA,B,C\n", &plain).await);
    assert!(
        found
            .iter()
            .any(|d| d.line == 1 && d.message.contains("bogus")),
        "{found:#?}"
    );
    let found = problems(import_airports(&t, "nopk.csv", "name\nB\n", &plain).await);
    assert!(
        found
            .iter()
            .any(|d| d.line == 1 && d.message.contains("`code`")),
        "{found:#?}"
    );
    let found = problems(import_airports(&t, "noreq.csv", "code,city\nA,B\nC,D\n", &plain).await);
    assert_eq!(
        found
            .iter()
            .filter(|d| d.message.contains("`name`"))
            .count(),
        1,
        "a missing required column is reported once, not for every row: {found:#?}"
    );

    // Several kinds of problem in one file: all of them are reported, with their lines.
    let many = "code,name,location\nA1,Alpha,\"1,2\"\nA2,,\"1,2\"\nA3,Gamma,\"95,2\"\nA1,Again,\"1,2\"\n,Empty,\n";
    let found = problems(import_airports(&t, "many.csv", many, &plain).await);
    let at = |line: u32| found.iter().any(|d| d.line == line && d.code == codes::V27);
    assert!(at(3) && at(4) && at(5) && at(6), "{found:#?}");
    assert!(!at(2), "the good row is not a problem: {found:#?}");
    assert!(found.iter().all(|d| d.file.ends_with("many.csv")));
    assert!(whole_log(&t.store).await.is_empty(), "nothing was written");

    // A file that is not UTF-8.
    let (_dir, path) = file_with("latin.csv", "");
    std::fs::write(&path, b"code,name\nA1,Caf\xe9\n").expect("write");
    let found = problems(import(&t.store, object("Airport"), &path, &plain).await);
    assert!(found.iter().any(|d| d.code == codes::V27), "{found:#?}");

    // Neither .csv nor .json.
    let found = problems(import_airports(&t, "airports.txt", "code,name\nA1,B\n", &plain).await);
    assert!(
        found.iter().any(|d| d.message.contains(".csv")),
        "{found:#?}"
    );
    assert!(whole_log(&t.store).await.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn limits_are_enforced() {
    let t = store_with(&aviation()).await;
    let rows: Vec<String> = (0..5).map(|i| format!("A{i},Name {i}")).collect();
    let (_dir, path) = file_with("five.csv", &format!("code,name\n{}\n", rows.join("\n")));

    let few = ImportOptions {
        max_rows: 3,
        ..ImportOptions::default()
    };
    let found = problems(import(&t.store, object("Airport"), &path, &few).await);
    assert!(
        found.iter().any(|d| d.message.contains("more than 3 rows")),
        "{found:#?}"
    );

    let small = ImportOptions {
        max_bytes: 10,
        ..ImportOptions::default()
    };
    let found = problems(import(&t.store, object("Airport"), &path, &small).await);
    assert!(
        found.iter().any(|d| d.message.contains("larger")),
        "{found:#?}"
    );

    let batch = ImportOptions {
        batch_size: 10_001,
        ..ImportOptions::default()
    };
    let found = problems(import(&t.store, object("Airport"), &path, &batch).await);
    assert!(found.iter().any(|d| d.code == codes::V27), "{found:#?}");
    assert!(whole_log(&t.store).await.is_empty());

    // Exactly max_rows rows are accepted.
    let exact = ImportOptions {
        max_rows: 5,
        ..ImportOptions::default()
    };
    let report = import(&t.store, object("Airport"), &path, &exact)
        .await
        .expect("exactly max_rows");
    assert_eq!(report.rows, 5);
}

#[tokio::test(flavor = "multi_thread")]
async fn at_most_one_hundred_problems_are_listed() {
    let t = store_with(&aviation()).await;
    let rows: Vec<String> = (0..150).map(|i| format!("A{i},")).collect();
    let content = format!("code,name\n{}\n", rows.join("\n"));
    let found = problems(import_airports(&t, "bad.csv", &content, &ImportOptions::default()).await);
    let errors = found
        .iter()
        .filter(|d| d.code == codes::V27 && d.line > 0)
        .count();
    assert_eq!(errors, 100, "{} diagnostics", found.len());
    assert!(
        found
            .iter()
            .any(|d| d.message.contains("only the first 100"))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn empty_and_repeated_imports_are_stable() {
    let t = store_with(&aviation()).await;
    let plain = ImportOptions::default();
    let report = import_airports(&t, "none.csv", "code,name\n", &plain)
        .await
        .expect("header only");
    assert_eq!((report.rows, report.commits.len()), (0, 0));

    // A byte order mark in front of the header is ignored.
    let bom = "\u{feff}code,name\nB1,With BOM\n";
    let report = import_airports(&t, "bom.csv", bom, &plain)
        .await
        .expect("BOM file");
    assert_eq!(report.created, 1);

    // The same file again: refused as a whole by default, a no-op when skipping.
    let found = problems(import_airports(&t, "bom.csv", bom, &plain).await);
    assert!(
        found.iter().any(|d| d.message.contains("already exists")),
        "{found:#?}"
    );
    let log_before = whole_log(&t.store).await.len();
    let report = import_airports(&t, "bom.csv", bom, &on_existing(OnExisting::Skip))
        .await
        .expect("skip");
    assert_eq!(
        (report.created, report.skipped, report.commits.len()),
        (0, 1, 0)
    );
    assert_eq!(whole_log(&t.store).await.len(), log_before);

    // JSON: the top level must be an array of objects, and every item is checked.
    let found = problems(import_airports(&t, "shape.json", "{\"code\": \"A\"}", &plain).await);
    assert!(
        found.iter().any(|d| d.message.contains("array")),
        "{found:#?}"
    );
    let items = "[{\"code\": \"J1\", \"name\": \"Fine\"}, 7, {\"code\": \"J2\"}, \
                 {\"code\": \"J3\", \"name\": \"N\", \"nope\": 1}]";
    let found = problems(import_airports(&t, "items.json", items, &plain).await);
    for item in ["item 2", "item 3", "item 4"] {
        assert!(
            found.iter().any(|d| d.message.contains(item)),
            "{item}: {found:#?}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// A batch fails after others were written
// ---------------------------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn partial_failure_is_reported() {
    let t = store_with(&gates_and_stands()).await;
    let two = ImportOptions {
        batch_size: 2,
        ..ImportOptions::default()
    };
    // The one to one link gives each gate at most one stand: S4 claims A1 again in batch 1.
    let (_dir, first) = file_with(
        "stands.csv",
        "id,label,gate\n1,S1,A1\n2,S2,A2\n3,S3,A3\n4,S4,A1\n",
    );
    let err = import(&t.store, object("Stand"), &first, &two)
        .await
        .expect_err("the second batch violates the link");
    let ImportError::Partial {
        committed,
        failed_batch,
        error,
    } = err
    else {
        panic!("expected a partial failure, got {err:?}");
    };
    assert_eq!(failed_batch, 1);
    assert_eq!(committed.len(), 1);
    assert!(
        matches!(*error, EditError::CardinalityViolation { .. }),
        "{error:?}"
    );
    assert_eq!(count(&t, "of_obj_stand").await, 2, "the first batch stays");
    assert_eq!(whole_log(&t.store).await.len(), 1);

    // The file is corrected; running it again with skip writes the rest.
    let (_dir2, fixed) = file_with(
        "stands.csv",
        "id,label,gate\n1,S1,A1\n2,S2,A2\n3,S3,A3\n4,S4,A4\n",
    );
    let resume = ImportOptions {
        batch_size: 2,
        on_existing: OnExisting::Skip,
        ..ImportOptions::default()
    };
    let report = import(&t.store, object("Stand"), &fixed, &resume)
        .await
        .expect("resume");
    assert_eq!((report.created, report.skipped), (2, 2));
    assert_eq!(count(&t, "of_obj_stand").await, 4);
}

#[tokio::test(flavor = "multi_thread")]
async fn import_goes_through_the_definition_fence() {
    let t = store_with(&gates_and_stands()).await;
    let other = t.reopen().await;
    let changed = ontology_from(&[(
        "defs.yaml",
        &GATES_AND_STANDS.replacen(
            "  name: { type: string, required: true }
",
            "  name: { type: string, required: true }
  note: { type: string }
",
            1,
        ),
    )]);
    reload(&other, &changed, &ReloadOptions::default())
        .await
        .expect("another process switches the definition");

    // This store still holds version 1 in memory; the database fence refuses its writes.
    let (_dir, path) = file_with("gates.csv", "code,name\nA1,Alpha\n");
    let err = import(&t.store, object("Gate"), &path, &ImportOptions::default())
        .await
        .expect_err("stale definition");
    let ImportError::Partial {
        committed,
        failed_batch,
        error,
    } = err
    else {
        panic!("expected a partial failure, got {err:?}");
    };
    assert_eq!((committed.len(), failed_batch), (0, 0));
    assert!(
        matches!(*error, EditError::DefinitionStale { .. }),
        "{error:?}"
    );
    assert!(whole_log(&t.store).await.is_empty());

    // The refusal made the store follow the database, so the next import uses version 2.
    let (_dir2, with_note) = file_with("gates2.csv", "code,name,note\nA1,Alpha,first\n");
    let report = import(
        &t.store,
        object("Gate"),
        &with_note,
        &ImportOptions::default(),
    )
    .await
    .expect("import against the new definition");
    assert_eq!(report.created, 1);
    let log = whole_log(&t.store).await;
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].def_version, 2);
}

// ---------------------------------------------------------------------------------------------
// PostgreSQL
// ---------------------------------------------------------------------------------------------

/// A type whose key is a timestamp and whose other properties cover the cell conversions that
/// differ between the two databases.
fn readings() -> Ontology {
    ontology_from(&[(
        "defs.yaml",
        "kind: ObjectType
apiName: Reading
backing: editOnly
primaryKey: at
properties:
  at: { type: timestamp, required: true }
  amount: { type: decimal, precision: 8, scale: 2 }
  day: { type: date }
  tags: { type: array, items: integer }
  spot: { type: geopoint }
  ratio: { type: double }
",
    )])
}

/// The import modes, the existing-row lookup (string, integer and timestamp keys, edges) and the
/// cell conversions on PostgreSQL. Run with `--ignored` against `ONTOFORGE_PG_URL` or with
/// `--features embedded-pg`.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs a PostgreSQL server"]
async fn pg_import_modes_and_links() {
    let pg = support::pg_store().await;
    let store = &pg.store;
    reload(store, &gates_and_stands(), &ReloadOptions::default())
        .await
        .expect("reload");
    let plain = ImportOptions::default();
    let (_g, gates) = file_with("gates.csv", "code,name\nA1,Alpha\nA2,Beta\n");
    let (_s, stands) = file_with("stands.csv", "id,label\n1,one\n2,two\n3,three\n");
    import(store, object("Gate"), &gates, &plain)
        .await
        .expect("gates");
    import(store, object("Stand"), &stands, &plain)
        .await
        .expect("stands");
    let (_e, edges) = file_with("edges.csv", "from,to\nA1,1\nA1,2\nA2,3\n");
    let report = import(store, link("gateStands"), &edges, &plain)
        .await
        .expect("edges");
    assert_eq!(report.created, 3);

    // The same files again.
    let found = problems(import(store, object("Gate"), &gates, &plain).await);
    assert_eq!(found.len(), 2, "{found:#?}");
    let found = problems(import(store, object("Stand"), &stands, &plain).await);
    assert_eq!(found.len(), 3, "{found:#?}");
    let found = problems(import(store, link("gateStands"), &edges, &plain).await);
    assert_eq!(found.len(), 3, "{found:#?}");
    let skip = on_existing(OnExisting::Skip);
    for (target, file) in [
        (object("Gate"), &gates),
        (object("Stand"), &stands),
        (link("gateStands"), &edges),
    ] {
        let report = import(store, target, file, &skip).await.expect("skip");
        assert_eq!((report.created, report.commits.len()), (0, 0));
    }
    let update = on_existing(OnExisting::Update);
    let report = import(store, object("Gate"), &gates, &update)
        .await
        .expect("update");
    assert_eq!((report.created, report.updated), (0, 2));
    let version: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT of_version FROM \"{}\".of_obj_gate WHERE code = 'A1'",
        pg.schema_edit
    )))
    .fetch_one(pg.side())
    .await
    .expect("version");
    assert_eq!(version, 2);
    assert!(
        whole_log(store)
            .await
            .iter()
            .all(|record| record.actor == "system:import")
    );
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs a PostgreSQL server"]
async fn pg_cells_and_keys_of_every_kind() {
    let pg = support::pg_store().await;
    let store = &pg.store;
    reload(store, &readings(), &ReloadOptions::default())
        .await
        .expect("reload");
    let csv = "at,amount,day,tags,spot,ratio\n\
               2026-03-29T02:30:00+02:00,12.5,2026-03-29,\"[1, 2, 3]\",\"50.03,8.57\",0.25\n\
               2026-03-30T00:00:00Z,0.5,,,,\n";
    let (_dir, path) = file_with("readings.csv", csv);
    let target =
        ImportTarget::Object(ObjectTypeName::try_from("Reading".to_owned()).expect("type"));
    let report = import(store, target.clone(), &path, &ImportOptions::default())
        .await
        .expect("import");
    assert_eq!(report.created, 2);

    let log = whole_log(store).await;
    let first = log[0].entries[0].payload.as_ref().expect("payload");
    assert_eq!(first["amount"], "12.50");
    assert_eq!(first["tags"], serde_json::json!([1, 2, 3]));
    assert_eq!(
        log[0].entries[0].pk,
        serde_json::json!("2026-03-29T00:30:00.000Z"),
        "the timestamp key is stored in UTC"
    );

    // The lookup of existing rows finds a timestamp key written in another form.
    let again = "at,amount\n2026-03-29T00:30:00Z,1\n";
    let (_d2, again_path) = file_with("again.csv", again);
    let found = problems(
        import(
            store,
            target.clone(),
            &again_path,
            &ImportOptions::default(),
        )
        .await,
    );
    assert!(
        found.iter().any(|d| d.message.contains("already exists")),
        "{found:#?}"
    );
    let report = import(store, target, &again_path, &on_existing(OnExisting::Update))
        .await
        .expect("update");
    assert_eq!(report.updated, 1);
}
