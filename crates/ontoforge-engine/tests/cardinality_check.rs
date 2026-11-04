//! Real SQLite checks of logical object cardinality, including physical duplicate rows.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use camino::Utf8Path;
use ontoforge_engine::bind_check::BindingKey;
use ontoforge_engine::cardinality_check::{
    CardinalityFinding, CardinalityTarget, cardinality_targets, check_cardinality,
};
use ontoforge_engine::printer::ast::{Expr, OrderItem, Select, SelectItem};
use ontoforge_engine::printer::print_select;
use ontoforge_engine::sqlite::SqliteEngine;
use ontoforge_engine::sqlite::fixture::plan_fixture_tables;
use ontoforge_engine::{
    Dialect, Engine, EngineError, EngineStatus, PhysicalColumn, PrintedSql, QueryContext,
    QueryOutput, TableRef, Value,
};
use ontoforge_schema::ir::{BindingTarget, ObjectTypeName, ProfileName};
use ontoforge_schema::load_from_memory;
use tempfile::TempDir;

const OBJECTS: &str = "\
kind: ObjectType
apiName: Aircraft
primaryKey: id
properties:
  id: { type: string, required: true }
backing: editOnly
---
kind: ObjectType
apiName: MaintenanceRecord
primaryKey: id
properties:
  id: { type: string, required: true }
  aircraftId: { type: string }
backing: source
---
kind: LinkType
apiName: aircraftMaintenance
cardinality: oneToOne
from: { objectType: Aircraft, apiName: maintenance }
to: { objectType: MaintenanceRecord, apiName: aircraft, foreignKey: aircraftId }
";
const BINDING: &str = "\
kind: SourceBinding
target: { objectType: MaintenanceRecord }
source: { catalog: demo, schema: aviation, table: records }
columns:
  id: { column: record_id }
  aircraftId: { column: aircraft_id }
fixture: { file: fixtures/records.csv }
";
const FIXTURE: &str = "bindings/dev/fixtures/records.csv";

struct TrackedEngine {
    inner: SqliteEngine,
    scans: AtomicUsize,
}

impl Engine for TrackedEngine {
    fn dialect(&self) -> Dialect {
        self.inner.dialect()
    }

    async fn execute(
        &self,
        sql: &PrintedSql,
        ctx: &QueryContext,
    ) -> Result<QueryOutput, EngineError> {
        self.scans.fetch_add(1, Ordering::SeqCst);
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

fn ctx() -> QueryContext {
    QueryContext::new("cardinality-test".to_owned(), Vec::new())
}

async fn fixture(csv: &str) -> (TempDir, TrackedEngine, CardinalityTarget) {
    let out = load_from_memory(&[
        ("ontology.yaml", "apiName: demo\ndefaultProfile: dev\n"),
        ("objects.yaml", OBJECTS),
        ("bindings/dev/record.yaml", BINDING),
        (FIXTURE, csv),
    ]);
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    let ontology = out.ontology.expect("ontology");
    let profile = ProfileName::try_from("dev".to_owned()).expect("profile");
    let key = BindingKey {
        profile: profile.clone(),
        target: BindingTarget::ObjectType(
            ObjectTypeName::try_from("MaintenanceRecord".to_owned()).expect("name"),
        ),
    };
    let mut targets = cardinality_targets(&ontology, &profile, &[key]);
    assert_eq!(
        targets.len(),
        1,
        "a clean result must not come from skipping the target"
    );
    let target = targets.remove(0);
    let plan = plan_fixture_tables(&ontology, &out.sources, &profile).expect("fixture plan");
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(FIXTURE);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    std::fs::write(&path, csv).expect("fixture");
    let inner =
        SqliteEngine::from_fixtures(Utf8Path::from_path(dir.path()).expect("utf8"), &plan.tables)
            .await
            .expect("SQLite");
    (
        dir,
        TrackedEngine {
            inner,
            scans: AtomicUsize::new(0),
        },
        target,
    )
}

async fn check(csv: &str) -> CardinalityFinding {
    let (_dir, engine, target) = fixture(csv).await;
    let finding = check_cardinality(&engine, &target, &ctx())
        .await
        .expect("scan");
    assert_eq!(
        engine.scans.load(Ordering::SeqCst),
        1,
        "exactly one actual SQLite query"
    );
    assert_eq!(finding.target, target);
    finding
}

fn string(value: &str) -> Value {
    Value::String(value.to_owned())
}

#[tokio::test]
async fn distinct_objects_sharing_fk_are_reported() {
    let finding = check("record_id,aircraft_id\nM1,A1\nM2,A1\n").await;
    assert_eq!(finding.samples, vec![(string("A1"), 2)]);
    assert!(!finding.more);
}

#[tokio::test]
async fn duplicate_physical_rows_are_one_object() {
    let repeated = "record_id,aircraft_id\nM1,A1\nM1,A1\nM1,A1\n";
    assert!(check(repeated).await.samples.is_empty());
    let finding = check(&format!("{repeated}M2,A1\n")).await;
    assert_eq!(finding.samples, vec![(string("A1"), 2)]);
}

#[tokio::test]
async fn null_keys_and_clean_links_have_no_finding() {
    for csv in [
        "record_id,aircraft_id\nM1,\nM2,\n",
        "record_id,aircraft_id\nM1,A1\n,A1\n,A1\n",
        "record_id,aircraft_id\n,A1\n,A1\n",
        "record_id,aircraft_id\nM1,A1\nM2,A2\n",
        "record_id,aircraft_id\n",
    ] {
        let finding = check(csv).await;
        assert!(finding.samples.is_empty(), "{csv}: {finding:?}");
        assert!(!finding.more);
    }
}

async fn rows(engine: &SqliteEngine, target: &CardinalityTarget) -> QueryOutput {
    let columns = [target.primary_key.clone(), target.foreign_key.clone()];
    let query = print_select(
        &Select {
            projection: columns
                .iter()
                .map(|column| SelectItem {
                    expr: Expr::Column(column.clone()),
                    alias: None,
                })
                .collect(),
            from: Some(target.table.clone()),
            filter: None,
            group_by: Vec::new(),
            having: None,
            order_by: columns
                .iter()
                .map(|column| OrderItem {
                    expr: Expr::Column(column.clone()),
                    descending: false,
                    nulls_first: true,
                })
                .collect(),
            limit: None,
        },
        Dialect::Sqlite,
    )
    .expect("typed source read");
    engine.execute(&query, &ctx()).await.expect("source rows")
}

#[tokio::test]
async fn repeated_check_is_read_only() {
    let csv = "record_id,aircraft_id\nM4,B\nM2,A\nM1,A\nM3,B\nM5,C\nM6,C\nM7,C\nM1,A\n";
    let (dir, engine, target) = fixture(csv).await;
    let before = rows(&engine.inner, &target).await;
    assert_eq!(before.rows.len(), 8);
    let first = check_cardinality(&engine, &target, &ctx())
        .await
        .expect("first");
    let second = check_cardinality(&engine, &target, &ctx())
        .await
        .expect("second");
    assert_eq!(first, second);
    assert_eq!(
        first.samples,
        vec![(string("C"), 3), (string("A"), 2), (string("B"), 2)]
    );
    assert_eq!(engine.scans.load(Ordering::SeqCst), 2);
    assert_eq!(before, rows(&engine.inner, &target).await);
    assert_eq!(
        std::fs::read_to_string(dir.path().join(FIXTURE)).expect("fixture"),
        csv
    );
    let files = walkdir::WalkDir::new(dir.path())
        .into_iter()
        .map(|entry| entry.expect("entry"))
        .filter(|entry| entry.file_type().is_file())
        .count();
    assert_eq!(
        files, 1,
        "the fixture is the only file; no edit store was created"
    );
}
