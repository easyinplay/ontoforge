//! `--check-pk`: duplicate and NULL primary keys of a bound object type (BIND-03, ADR-0015).
//!
//! The duplicate query leaves NULL out; NULL keys are only counted by the second query (E17).

use std::fmt::Write as _;

use camino::Utf8Path;
use tempfile::TempDir;

use ontoforge_engine::bind_check::BindingKey;
use ontoforge_engine::pk_check::{PkFinding, check_primary_keys, display_value, pk_queries};
use ontoforge_engine::sqlite::SqliteEngine;
use ontoforge_engine::sqlite::fixture::plan_fixture_tables;
use ontoforge_engine::{AnyEngine, Dialect, QueryContext, TableRef, Value};
use ontoforge_schema::ir::{BindingTarget, ObjectTypeName, Ontology, ProfileName};
use ontoforge_schema::load_from_memory;

const MANIFEST: &str = "apiName: demo\ndefaultProfile: dev\n";
const OBJECTS: &str = "\
kind: ObjectType
apiName: Flight
primaryKey: flightId
properties:
  flightId: { type: string, required: true }
  status: { type: string }
backing: source
";
const BINDING: &str = "\
kind: SourceBinding
target: { objectType: Flight }
source: { catalog: demo, schema: aviation, table: flights }
columns:
  flightId: { column: flight_id }
fixture: { file: fixtures/flights.csv }
";
const FIXTURE: &str = "bindings/dev/fixtures/flights.csv";

fn dev() -> ProfileName {
    ProfileName::try_from("dev".to_owned()).expect("profile")
}

fn flight_key() -> BindingKey {
    BindingKey {
        profile: dev(),
        target: BindingTarget::ObjectType(
            ObjectTypeName::try_from("Flight".to_owned()).expect("name"),
        ),
    }
}

fn flights() -> TableRef {
    TableRef {
        catalog: "demo".to_owned(),
        schema: "aviation".to_owned(),
        table: "flights".to_owned(),
    }
}

fn ctx() -> QueryContext {
    QueryContext::new("test".to_owned(), Vec::new())
}

/// Checks the primary keys of `Flight` loaded from a fixture with `csv` as its text.
async fn check(csv: &str) -> Vec<PkFinding> {
    let files = [
        ("ontology.yaml", MANIFEST),
        ("objects.yaml", OBJECTS),
        ("bindings/dev/flight.yaml", BINDING),
        (FIXTURE, csv),
    ];
    let out = load_from_memory(&files);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    let ontology: Ontology = out.ontology.expect("ontology");
    let plan = plan_fixture_tables(&ontology, &out.sources, &dev()).expect("plan");

    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(FIXTURE);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("directories");
    std::fs::write(&path, csv).expect("write fixture");
    let root = Utf8Path::from_path(dir.path()).expect("utf-8 temp dir");
    let engine = SqliteEngine::from_fixtures(root, &plan.tables)
        .await
        .expect("fixture loads");

    check_primary_keys(
        &AnyEngine::Sqlite(engine),
        &ontology,
        &dev(),
        &[flight_key()],
        &ctx(),
    )
    .await
    .expect("the check runs")
}

/// A fixture with `groups` duplicated keys (two rows each), `nulls` NULL keys and one unique
/// key.
fn csv(groups: usize, nulls: usize) -> String {
    let mut text = String::from("flight_id,status\nU1,ok\n");
    for i in 0..groups {
        writeln!(text, "F{i:03},a\nF{i:03},b").expect("write to a String");
    }
    for _ in 0..nulls {
        text.push_str(",ok\n");
    }
    text
}

fn string(s: &str) -> Value {
    Value::String(s.to_owned())
}

#[test]
fn pk_queries_are_printed() {
    for (dialect, name) in [(Dialect::Sqlite, "sqlite"), (Dialect::Trino, "trino")] {
        let queries = pk_queries(&flights(), "flight_id", dialect).expect("printable");
        assert!(
            queries.duplicates.as_str().contains("IS NOT NULL"),
            "{}",
            queries.duplicates
        );
        insta::assert_snapshot!(format!("pk_duplicates_{name}"), queries.duplicates.as_str());
        insta::assert_snapshot!(format!("pk_nulls_{name}"), queries.nulls.as_str());
    }
}

#[tokio::test]
async fn duplicates_and_nulls_found() {
    let found = check(&csv(2, 1)).await;
    assert_eq!(found.len(), 1, "{found:#?}");
    let finding = &found[0];
    assert_eq!(finding.key, flight_key());
    assert_eq!(finding.table, flights());
    assert_eq!(
        finding.duplicates,
        vec![(string("F000"), 2), (string("F001"), 2)]
    );
    assert!(!finding.more_duplicates);
    assert_eq!(finding.null_count, 1);

    let found = check(&csv(25, 0)).await;
    assert_eq!(found[0].duplicates.len(), 20);
    assert!(found[0].more_duplicates);
    assert_eq!(found[0].null_count, 0);
}

#[tokio::test]
async fn twenty_groups_are_not_more() {
    let found = check(&csv(20, 0)).await;
    assert_eq!(found[0].duplicates.len(), 20);
    assert!(!found[0].more_duplicates);

    let found = check(&csv(21, 0)).await;
    assert_eq!(found[0].duplicates.len(), 20);
    assert!(found[0].more_duplicates);
}

#[tokio::test]
async fn nulls_are_not_duplicates() {
    let found = check(&csv(1, 3)).await;
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].duplicates, vec![(string("F000"), 2)]);
    assert!(found[0].duplicates.iter().all(|(v, _)| *v != Value::Null));
    assert_eq!(found[0].null_count, 3);

    let found = check(&csv(0, 2)).await;
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(found[0].duplicates.is_empty());
    assert_eq!(found[0].null_count, 2);
}

#[tokio::test]
async fn clean_and_empty_tables_have_no_finding() {
    assert!(check(&csv(0, 0)).await.is_empty());
    assert!(check("flight_id,status\n").await.is_empty());
}

#[test]
fn display_value_is_short_text() {
    assert_eq!(display_value(&string("F001")), "F001");
    assert_eq!(display_value(&Value::Long(42)), "42");
    assert_eq!(display_value(&Value::Null), "NULL");
    let long = "x".repeat(70);
    let shown = display_value(&string(&long));
    assert_eq!(shown, format!("{}…", "x".repeat(64)));
}
