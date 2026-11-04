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
use ontoforge_engine::sqlite::fixture::{
    FixtureColumn, FixtureTable, LogicalType, plan_fixture_tables,
};
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

#[tokio::test]
async fn mapped_to_binding_is_the_only_source() {
    let csv = "record_id,aircraft_id\nM1,A1\nM2,A1\n";
    let production_binding = BINDING.replace("table: records", "table: production_records");
    let out = load_from_memory(&[
        ("ontology.yaml", "apiName: demo\ndefaultProfile: dev\n"),
        ("objects.yaml", OBJECTS),
        ("bindings/dev/record.yaml", BINDING),
        (FIXTURE, csv),
        ("bindings/prod/record.yaml", &production_binding),
        (
            "bindings/prod/fixtures/records.csv",
            "record_id,aircraft_id\nP1,P\n",
        ),
    ]);
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    let ontology = out.ontology.expect("ontology");
    let dev = ProfileName::try_from("dev".to_owned()).expect("profile");
    let prod = ProfileName::try_from("prod".to_owned()).expect("profile");
    let object = ObjectTypeName::try_from("MaintenanceRecord".to_owned()).expect("name");
    let key = BindingKey {
        profile: dev.clone(),
        target: BindingTarget::ObjectType(object.clone()),
    };
    let targets = cardinality_targets(&ontology, &dev, std::slice::from_ref(&key));
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].foreign_key, "aircraft_id");
    assert_eq!(targets[0].primary_key, "record_id");
    assert_eq!(targets[0].table.table, "records");
    assert!(
        cardinality_targets(&ontology, &prod, &[key]).is_empty(),
        "usable keys are profile-specific"
    );
    let production_targets = cardinality_targets(
        &ontology,
        &prod,
        &[BindingKey {
            profile: prod.clone(),
            target: BindingTarget::ObjectType(object),
        }],
    );
    assert_eq!(production_targets[0].table.table, "production_records");
    let (_dir, engine, _) = fixture(csv).await;
    let result = check_cardinality(&engine, &targets[0], &ctx())
        .await
        .expect("only dev table exists");
    assert_eq!(result.samples, vec![(string("A1"), 2)]);
    assert_eq!(engine.scans.load(Ordering::SeqCst), 1);
}

fn grouped_csv(groups: usize) -> String {
    let mut text = String::from("record_id,aircraft_id\n");
    for i in (0..groups).rev() {
        text.push_str(&format!("M{i}a,A{i:03}\nM{i}b,A{i:03}\n"));
    }
    text
}

#[tokio::test]
async fn samples_have_twenty_plus_sentinel_bound() {
    for groups in [20, 21, 22] {
        let finding = check(&grouped_csv(groups)).await;
        assert_eq!(finding.samples.len(), 20);
        assert_eq!(finding.more, groups > 20);
        let expected = (0..20)
            .map(|i| (string(&format!("A{i:03}")), 2))
            .collect::<Vec<_>>();
        assert_eq!(finding.samples, expected);
    }
}

#[tokio::test]
async fn tighter_budgets_are_not_widened() {
    let (_dir, engine, target) = fixture(&grouped_csv(22)).await;
    for max_rows in [0, 1, 20] {
        let context = QueryContext { max_rows, ..ctx() };
        let error = check_cardinality(&engine, &target, &context)
            .await
            .expect_err("incomplete rows");
        assert!(error.to_string().contains("truncated"), "{error}");
    }
    for max_bytes in [0, 1, 20] {
        let context = QueryContext { max_bytes, ..ctx() };
        let error = check_cardinality(&engine, &target, &context)
            .await
            .expect_err("incomplete bytes");
        assert!(error.to_string().contains("truncated"), "{error}");
    }
    let allowed = QueryContext {
        max_rows: 21,
        ..ctx()
    };
    assert!(
        check_cardinality(&engine, &target, &allowed)
            .await
            .expect("complete sentinel")
            .more
    );
    assert_eq!(engine.scans.load(Ordering::SeqCst), 7);
}

fn fixture_table(target: &CardinalityTarget, file: &str) -> FixtureTable {
    FixtureTable {
        def_names: vec!["dev/MaintenanceRecord".to_owned()],
        table: target.table.clone(),
        file: file.to_owned(),
        columns: [target.primary_key.clone(), target.foreign_key.clone()]
            .into_iter()
            .map(|name| FixtureColumn {
                name,
                logical: LogicalType::String,
            })
            .collect(),
    }
}

#[tokio::test]
async fn count_distinct_quotes_identifiers_on_sqlite() {
    let (dir, _engine, mut target) = fixture("record_id,aircraft_id\nM1,A1\n").await;
    target.table.table = "records\"; DROP TABLE victim; --".to_owned();
    target.primary_key = "pk\"; --".to_owned();
    target.foreign_key = "fk\" /* */".to_owned();
    let special_file = "special.csv";
    let mut csv = csv::Writer::from_path(dir.path().join(special_file)).expect("CSV");
    csv.write_record([&target.primary_key, &target.foreign_key])
        .expect("header");
    for row in [["M1", "A1"], ["M2", "A1"], ["M1", "A1"]] {
        csv.write_record(row).expect("row");
    }
    csv.flush().expect("flush");
    let mut victim = target.clone();
    victim.table.table = "victim".to_owned();
    let tables = [
        fixture_table(&target, special_file),
        fixture_table(&victim, special_file),
    ];
    let engine =
        SqliteEngine::from_fixtures(Utf8Path::from_path(dir.path()).expect("utf8"), &tables)
            .await
            .expect("quoted fixtures");
    let before = rows(&engine, &victim).await;
    let finding = check_cardinality(&engine, &target, &ctx())
        .await
        .expect("safe query");
    assert_eq!(finding.samples, vec![(string("A1"), 2)]);
    assert_eq!(before.rows.len(), 3);
    assert_eq!(rows(&engine, &victim).await, before);
    assert_eq!(rows(&engine, &target).await.rows, before.rows);
}

#[tokio::test]
async fn independent_runs_observe_current_source() {
    let (dir, engine, target) = fixture("record_id,aircraft_id\nM1,A1\nM2,A1\n").await;
    let first = check_cardinality(&engine, &target, &ctx())
        .await
        .expect("first scan");
    assert_eq!(first.samples, vec![(string("A1"), 2)]);
    std::fs::write(
        dir.path().join(FIXTURE),
        "record_id,aircraft_id\nM1,A1\nM2,A2\n",
    )
    .expect("controlled source update");
    let changed = SqliteEngine::from_fixtures(
        Utf8Path::from_path(dir.path()).expect("utf8"),
        &[fixture_table(&target, FIXTURE)],
    )
    .await
    .expect("updated fixture");
    let second = check_cardinality(&changed, &target, &ctx())
        .await
        .expect("new observation");
    assert!(second.samples.is_empty());
    assert_eq!(rows(&changed, &target).await.rows.len(), 2);
}
