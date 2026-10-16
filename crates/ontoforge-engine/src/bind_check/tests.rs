use ontoforge_schema::diag::Severity;
use ontoforge_schema::ir::{BindingTarget, Ontology, ProfileName, SourceBinding};
use ontoforge_schema::load_from_memory;
use ontoforge_schema::loader::sourcemap::Seg;

use super::{MismatchKind, compare_binding, expected_columns};
use crate::{PhysicalColumn, PhysicalType};

const OBJECTS: &str = "\
kind: ObjectType
apiName: Flight
primaryKey: flightId
properties:
  flightId: { type: string, required: true }
  flightNumber: { type: string }
  seats: { type: integer }
  cost: { type: decimal, precision: 12, scale: 2 }
backing: source
";

const BINDING: &str = "\
kind: SourceBinding
target: { objectType: Flight }
source: { catalog: demo, schema: aviation, table: flights }
columns:
  flightNumber: { column: flight_no }
";

fn flight() -> (Ontology, SourceBinding) {
    let out = load_from_memory(&[
        ("ontology.yaml", "apiName: demo\ndefaultProfile: dev\n"),
        ("objects.yaml", OBJECTS),
        ("bindings/dev/flight.yaml", BINDING),
    ]);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    let ontology = out.ontology.expect("ontology");
    let dev = ProfileName::try_from("dev".to_owned()).expect("profile");
    let binding = ontology
        .profile_bindings(&dev)
        .expect("dev bindings")
        .objects
        .values()
        .next()
        .expect("flight binding")
        .clone();
    (ontology, binding)
}

fn column(name: &str, physical: PhysicalType) -> PhysicalColumn {
    PhysicalColumn {
        name: name.to_owned(),
        physical,
        nullable: true,
    }
}

fn varchar() -> PhysicalType {
    PhysicalType::Varchar { length: None }
}

fn decimal() -> PhysicalType {
    PhysicalType::Decimal {
        precision: 12,
        scale: 2,
    }
}

/// A physical table that matches the binding.
fn matching() -> Vec<PhysicalColumn> {
    vec![
        column("flightId", varchar()),
        column("flight_no", varchar()),
        column("seats", PhysicalType::Integer),
        column("cost", decimal()),
    ]
}

#[test]
fn expected_columns_come_from_bound_columns_and_effective_decimal() {
    let (ontology, binding) = flight();
    let expected = expected_columns(&ontology, &binding);
    let names: Vec<(&str, &str, bool)> = expected
        .iter()
        .map(|c| (c.key.as_str(), c.column.as_str(), c.explicit))
        .collect();
    assert_eq!(
        names,
        [
            ("flightId", "flightId", false),
            ("flightNumber", "flight_no", true),
            ("seats", "seats", false),
            ("cost", "cost", false),
        ]
    );
    let cost = &expected[3];
    assert_eq!((cost.precision, cost.scale), (12, 2));
    assert!(matches!(binding.target, BindingTarget::ObjectType(_)));
}

#[test]
fn matching_table_has_no_mismatch() {
    let (ontology, binding) = flight();
    assert_eq!(
        compare_binding(&ontology, &binding, Some(&matching())),
        Vec::new()
    );
}

#[test]
fn online_missing_table() {
    let (ontology, binding) = flight();
    let found = compare_binding(&ontology, &binding, None);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].kind, MismatchKind::TableMissing);
    assert_eq!(found[0].severity, Severity::Error);
    assert_eq!(
        found[0].path,
        [Seg::Key("source".into()), Seg::Key("table".into())]
    );
    assert!(found[0].message.contains("demo.aviation.flights"));
}

#[test]
fn missing_explicit_column_points_at_the_column_value() {
    let (ontology, binding) = flight();
    let mut physical = matching();
    physical.retain(|c| c.name != "flight_no");
    let found = compare_binding(&ontology, &binding, Some(&physical));
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].kind, MismatchKind::ColumnMissing);
    assert_eq!(
        found[0].path,
        [
            Seg::Key("columns".into()),
            Seg::Key("flightNumber".into()),
            Seg::Key("column".into())
        ]
    );
    assert!(found[0].message.contains("flight_no"));
}

#[test]
fn missing_implicit_column_points_at_the_table() {
    let (ontology, binding) = flight();
    let mut physical = matching();
    physical.retain(|c| c.name != "seats");
    let found = compare_binding(&ontology, &binding, Some(&physical));
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].kind, MismatchKind::ColumnMissing);
    assert_eq!(
        found[0].path,
        [Seg::Key("source".into()), Seg::Key("table".into())]
    );
    assert!(found[0].message.contains("seats"));
}

#[test]
fn column_names_are_compared_in_lower_case() {
    let (ontology, binding) = flight();
    let physical = vec![
        column("flightid", varchar()),
        column("FLIGHT_NO", varchar()),
        column("Seats", PhysicalType::Integer),
        column("COST", decimal()),
    ];
    assert_eq!(
        compare_binding(&ontology, &binding, Some(&physical)),
        Vec::new()
    );
}

#[test]
fn online_type_family_mismatch() {
    let (ontology, binding) = flight();
    let mut physical = matching();
    physical[2] = column("seats", varchar());
    let found = compare_binding(&ontology, &binding, Some(&physical));
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].kind, MismatchKind::TypeIncompatible);
    assert_eq!(found[0].severity, Severity::Error);
    assert!(found[0].message.contains("seats"));

    // Within one family, 02-08 decides; this plan reports nothing.
    physical[2] = column("seats", PhysicalType::Bigint);
    assert_eq!(
        compare_binding(&ontology, &binding, Some(&physical)),
        Vec::new()
    );
}

fn mismatch(kind: MismatchKind, severity: Severity) -> super::Mismatch {
    super::Mismatch {
        kind,
        severity,
        path: Vec::new(),
        message: format!("{kind:?}"),
        help: None,
    }
}

#[test]
fn warnings_do_not_degrade() {
    use super::{DegradeReason, TargetHealth, health_of};

    let warning = mismatch(MismatchKind::CharPadding, Severity::Warning);
    assert_eq!(
        health_of(vec![warning.clone()]),
        TargetHealth::Healthy {
            warnings: vec![warning.clone()]
        }
    );
    assert_eq!(
        health_of(Vec::new()),
        TargetHealth::Healthy { warnings: vec![] }
    );

    let error = mismatch(MismatchKind::ColumnMissing, Severity::Error);
    assert_eq!(
        health_of(vec![warning.clone(), error.clone()]),
        TargetHealth::Degraded {
            reason: DegradeReason::SourceSchemaDrift,
            mismatches: vec![warning, error],
        }
    );
}
