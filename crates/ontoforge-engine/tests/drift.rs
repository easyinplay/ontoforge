//! Drift checks end to end (BIND-03, D-15): the aviation example loaded into the SQLite
//! stand-in, then judged by `check_drift`; the judging itself is a pure function and is
//! tested with hand-built descriptions.

use std::fs;
use std::net::TcpListener;
use std::path::Path;
use std::time::Duration;

use camino::Utf8Path;
use ontoforge_engine::bind_check::{
    BindingKey, DegradeReason, DescribedBinding, DriftReport, MAX_DETAIL_BYTES, MismatchKind,
    OperatorDetail, TargetHealth, check_drift, drift_report,
};
use ontoforge_engine::sqlite::SqliteEngine;
use ontoforge_engine::sqlite::fixture::plan_fixture_tables;
use ontoforge_engine::trino::{Auth, TrinoConnection, TrinoEngine};
use ontoforge_engine::{
    AnyEngine, EngineError, PhysicalColumn, PhysicalType, QueryContext, TableRef,
};
use ontoforge_schema::ir::{BindingTarget, LinkTypeName, ObjectTypeName, Ontology, ProfileName};
use ontoforge_schema::load_from_memory;

const AVIATION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/ontologies/aviation"
);

fn dev() -> ProfileName {
    ProfileName::try_from("dev".to_owned()).expect("profile")
}

fn ctx() -> QueryContext {
    QueryContext::new("trace-drift".to_owned(), vec!["ontoforge".to_owned()])
}

fn object(name: &str) -> ObjectTypeName {
    ObjectTypeName::try_from(name.to_owned()).expect("object type name")
}

fn link(name: &str) -> LinkTypeName {
    LinkTypeName::try_from(name.to_owned()).expect("link type name")
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("create dir");
    for entry in fs::read_dir(from).expect("read dir") {
        let entry = entry.expect("entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("file type").is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).expect("copy");
        }
    }
}

/// A scratch copy of the aviation example.
fn aviation_copy() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    copy_dir(Path::new(AVIATION), dir.path());
    dir
}

/// Removes column `name` from the header and every record of a CSV file.
fn drop_csv_column(path: &Path, name: &str) {
    let mut reader = csv::ReaderBuilder::new()
        .from_path(path)
        .expect("open fixture");
    let headers = reader.headers().expect("headers").clone();
    let index = headers
        .iter()
        .position(|h| h == name)
        .unwrap_or_else(|| panic!("fixture has no column {name}"));
    let keep = |record: &csv::StringRecord| -> Vec<String> {
        record
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != index)
            .map(|(_, cell)| cell.to_owned())
            .collect()
    };
    let mut rows = vec![keep(&headers)];
    for record in reader.records() {
        rows.push(keep(&record.expect("record")));
    }
    let mut writer = csv::Writer::from_path(path).expect("write fixture");
    for row in rows {
        writer.write_record(&row).expect("write record");
    }
    writer.flush().expect("flush");
}

async fn drift_of(dir: &Path) -> DriftReport {
    let root = Utf8Path::from_path(dir).expect("utf-8 path");
    let out = ontoforge_schema::load_dir(root);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    let ontology = out.ontology.expect("ontology");
    let profile = dev();
    let plan = plan_fixture_tables(&ontology, &out.sources, &profile).expect("fixture plan");
    assert!(plan.without_fixture.is_empty());
    let engine = SqliteEngine::from_fixtures(root, &plan.tables)
        .await
        .expect("stand-in loads");
    check_drift(&AnyEngine::Sqlite(engine), &ontology, &profile, &ctx())
        .await
        .expect("a stand-in is always available")
}

#[tokio::test]
async fn aviation_is_healthy() {
    let dir = aviation_copy();
    let report = drift_of(dir.path()).await;
    assert_eq!(report.profile, dev());
    assert!(report.is_healthy(), "{report:#?}");
    let objects: Vec<&str> = report.objects.keys().map(ObjectTypeName::as_str).collect();
    assert_eq!(
        objects,
        ["Aircraft", "Flight", "MaintenanceRecord", "Pilot"],
        "Airport is editOnly and has no binding"
    );
    for (name, health) in &report.objects {
        assert!(
            matches!(health, TargetHealth::Healthy { warnings } if warnings.is_empty()),
            "{name}: {health:?}"
        );
    }
    let links: Vec<&str> = report.links.keys().map(LinkTypeName::as_str).collect();
    assert_eq!(links, ["flightCrew"]);
    assert!(matches!(
        report.links[&link("flightCrew")],
        TargetHealth::Healthy { .. }
    ));
}

#[tokio::test]
async fn missing_column_degrades_only_flight() {
    let dir = aviation_copy();
    drop_csv_column(
        &dir.path().join("bindings/dev/fixtures/flights.csv"),
        "delay_minutes",
    );
    let report = drift_of(dir.path()).await;
    assert!(!report.is_healthy());
    match &report.objects[&object("Flight")] {
        TargetHealth::Degraded {
            reason: DegradeReason::SourceSchemaDrift,
            mismatches,
        } => {
            assert!(
                mismatches
                    .iter()
                    .any(|m| m.kind == MismatchKind::ColumnMissing),
                "{mismatches:#?}"
            );
        }
        other => panic!("expected Flight to drift, got {other:?}"),
    }
    for name in ["Aircraft", "MaintenanceRecord", "Pilot"] {
        assert!(
            matches!(report.objects[&object(name)], TargetHealth::Healthy { .. }),
            "{name} must stay healthy"
        );
    }
    assert!(matches!(
        report.links[&link("flightCrew")],
        TargetHealth::Healthy { .. }
    ));
}

const OBJECTS: &str = "\
kind: ObjectType
apiName: Flight
primaryKey: flightId
properties:
  flightId: { type: string, required: true }
backing: source
---
kind: ObjectType
apiName: Pilot
primaryKey: employeeId
properties:
  employeeId: { type: string, required: true }
backing: source
";

fn pure_ontology() -> Ontology {
    let out = load_from_memory(&[
        ("ontology.yaml", "apiName: demo\ndefaultProfile: dev\n"),
        ("objects.yaml", OBJECTS),
        (
            "bindings/dev/flight.yaml",
            "kind: SourceBinding\ntarget: { objectType: Flight }\nsource: { catalog: demo, schema: aviation, table: flights }\n",
        ),
        (
            "bindings/dev/pilot.yaml",
            "kind: SourceBinding\ntarget: { objectType: Pilot }\nsource: { catalog: demo, schema: aviation, table: pilots }\n",
        ),
    ]);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    out.ontology.expect("ontology")
}

fn described(
    name: &str,
    table: &str,
    physical: Result<Option<Vec<PhysicalColumn>>, EngineError>,
) -> DescribedBinding {
    DescribedBinding {
        key: BindingKey {
            profile: dev(),
            target: BindingTarget::ObjectType(object(name)),
        },
        table: TableRef {
            catalog: "demo".to_owned(),
            schema: "aviation".to_owned(),
            table: table.to_owned(),
        },
        physical,
    }
}

fn key_column(name: &str) -> PhysicalColumn {
    PhysicalColumn {
        name: name.to_owned(),
        physical: PhysicalType::Varchar { length: None },
        nullable: false,
    }
}

#[test]
fn query_error_degrades_only_that_target() {
    let ontology = pure_ontology();
    let all = [
        described(
            "Flight",
            "flights",
            Err(EngineError::Query {
                error_name: "PERMISSION_DENIED".to_owned(),
                message: "denied".to_owned(),
            }),
        ),
        described("Pilot", "pilots", Ok(Some(vec![key_column("employeeId")]))),
    ];
    let report = drift_report(&ontology, &dev(), &all);
    assert!(!report.is_healthy());
    match &report.objects[&object("Flight")] {
        TargetHealth::Degraded {
            reason:
                DegradeReason::SourceUnreadable {
                    code,
                    error_name,
                    detail,
                },
            mismatches,
        } => {
            assert_eq!(*code, "ENGINE_QUERY_FAILED");
            assert_eq!(error_name.as_deref(), Some("PERMISSION_DENIED"));
            assert!(detail.text().contains("denied"), "{}", detail.text());
            assert!(mismatches.is_empty());
        }
        other => panic!("expected Flight to be unreadable, got {other:?}"),
    }
    assert_eq!(
        report.objects[&object("Pilot")],
        TargetHealth::Healthy { warnings: vec![] }
    );
}

#[test]
fn missing_table_is_drift() {
    let ontology = pure_ontology();
    let all = [described("Pilot", "pilots", Ok(None))];
    let report = drift_report(&ontology, &dev(), &all);
    match &report.objects[&object("Pilot")] {
        TargetHealth::Degraded {
            reason: DegradeReason::SourceSchemaDrift,
            mismatches,
        } => assert_eq!(mismatches[0].kind, MismatchKind::TableMissing),
        other => panic!("expected drift, got {other:?}"),
    }
}

/// A local port nothing listens on: bind, note the port, release it.
fn closed_port_url() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    drop(listener);
    format!("http://127.0.0.1:{port}")
}

#[tokio::test]
async fn engine_error_is_not_a_report() {
    let engine = AnyEngine::Trino(
        TrinoEngine::new(TrinoConnection {
            url: reqwest::Url::parse(&closed_port_url()).expect("url"),
            user: "ontoforge".to_owned(),
            auth: Auth::None,
            ca_pem: None,
            connect_timeout: Duration::from_secs(2),
            max_page_bytes: 64 * 1024 * 1024,
            extra_origins: Vec::new(),
        })
        .expect("engine"),
    );
    let err = check_drift(&engine, &pure_ontology(), &dev(), &ctx())
        .await
        .expect_err("an unavailable engine is an error, not a report");
    assert!(matches!(err, EngineError::Unavailable { .. }), "{err:?}");
}

#[test]
fn degrade_reason_code() {
    assert_eq!(
        DegradeReason::SourceSchemaDrift.code(),
        "SOURCE_SCHEMA_DRIFT"
    );
    assert_eq!(
        DegradeReason::SourceUnreadable {
            code: "ENGINE_QUERY_FAILED",
            error_name: None,
            detail: OperatorDetail::new("x"),
        }
        .code(),
        "SOURCE_UNREADABLE"
    );
}

#[test]
fn unreadable_reason_keeps_engine_text_out_of_the_exposable_fields() {
    // WR-E07: Trino messages name internal hosts and quote data values; only the error code
    // and a well-formed error name may reach a caller, the text stays operator-only.
    let ontology = pure_ontology();
    let leaky = "Failed connecting to Hive metastore: [thrift://10.0.0.5:9083] Cannot cast 'alice@corp' to INTEGER";
    let all = [
        described(
            "Flight",
            "flights",
            Err(EngineError::Query {
                error_name: "HIVE_METASTORE_ERROR".to_owned(),
                message: leaky.to_owned(),
            }),
        ),
        described(
            "Pilot",
            "pilots",
            Err(EngineError::Query {
                error_name: "not a name: thrift://10.0.0.5".to_owned(),
                message: "x".repeat(10_000),
            }),
        ),
    ];
    let report = drift_report(&ontology, &dev(), &all);
    let debug = format!("{report:?}");
    for secret in ["10.0.0.5", "alice@corp", "thrift"] {
        assert!(!debug.contains(secret), "{debug}");
    }
    let unreadable = |name: &str| match &report.objects[&object(name)] {
        TargetHealth::Degraded {
            reason:
                DegradeReason::SourceUnreadable {
                    code,
                    error_name,
                    detail,
                },
            ..
        } => (*code, error_name.clone(), detail.clone()),
        other => panic!("expected {name} to be unreadable, got {other:?}"),
    };
    let (code, error_name, detail) = unreadable("Flight");
    assert_eq!(code, "ENGINE_QUERY_FAILED");
    assert_eq!(error_name.as_deref(), Some("HIVE_METASTORE_ERROR"));
    // Operators still get the whole text.
    assert!(detail.text().contains(leaky), "{}", detail.text());

    let (_, error_name, detail) = unreadable("Pilot");
    assert_eq!(error_name, None, "a malformed error name is dropped");
    assert!(
        detail.text().len() <= MAX_DETAIL_BYTES + 3,
        "{}",
        detail.text().len()
    );
}

#[test]
fn warnings_do_not_degrade() {
    let ontology = pure_ontology();
    let padded = PhysicalColumn {
        name: "flightId".to_owned(),
        physical: PhysicalType::Char { length: 8 },
        nullable: false,
    };
    let all = [
        described("Flight", "flights", Ok(Some(vec![padded]))),
        described("Pilot", "pilots", Ok(Some(vec![key_column("employeeId")]))),
    ];
    let report = drift_report(&ontology, &dev(), &all);
    assert!(report.is_healthy(), "{report:#?}");
    match &report.objects[&object("Flight")] {
        TargetHealth::Healthy { warnings } => {
            assert_eq!(warnings.len(), 1, "{warnings:#?}");
            assert_eq!(warnings[0].kind, MismatchKind::CharPadding);
        }
        other @ TargetHealth::Degraded { .. } => {
            panic!("expected Flight to stay healthy, got {other:?}")
        }
    }
}
