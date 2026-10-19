//! The D-13 compatibility table, one test per type family (02-08).

use ontoforge_schema::diag::Severity;
use ontoforge_schema::ir::{Ontology, ProfileName, SourceBinding};
use ontoforge_schema::load_from_memory;
use ontoforge_schema::loader::sourcemap::Seg;

use super::{Mismatch, MismatchKind as K, compare_binding};
use crate::{PhysicalColumn, PhysicalType};

/// What the comparison says about one column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    Ok,
    Warn(K),
    Error(K),
}

use Verdict::{Error as E, Ok as Good, Warn as W};

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

fn dev() -> ProfileName {
    ProfileName::try_from("dev".to_owned()).expect("profile")
}

/// Loads an ontology with one source-backed type `Thing` whose property `v` is declared by
/// `property`, bound to table `t`; `columns` is the binding's `columns:` map (or empty).
fn thing(property: &str, columns: &str) -> (Ontology, SourceBinding) {
    let objects = format!(
        "kind: ObjectType\napiName: Thing\nprimaryKey: id\nproperties:\n  id: {{ type: string, required: true }}\n  v: {{ {property} }}\nbacking: source\n"
    );
    let binding = format!(
        "kind: SourceBinding\ntarget: {{ objectType: Thing }}\nsource: {{ catalog: demo, schema: s, table: t }}\n{columns}"
    );
    let out = load_from_memory(&[
        ("ontology.yaml", "apiName: demo\ndefaultProfile: dev\n"),
        ("objects.yaml", objects.as_str()),
        ("bindings/dev/thing.yaml", binding.as_str()),
    ]);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    let ontology = out.ontology.expect("ontology");
    let binding = ontology
        .profile_bindings(&dev())
        .expect("dev bindings")
        .objects
        .values()
        .next()
        .expect("thing binding")
        .clone();
    (ontology, binding)
}

fn verdict_of(found: &[Mismatch]) -> Verdict {
    match found {
        [] => Good,
        [one] if one.severity == Severity::Warning => W(one.kind),
        [one] => E(one.kind),
        many => panic!("expected at most one mismatch, got {many:#?}"),
    }
}

/// The verdict for property `v` (declared by `property`, with `timezone` if given) read from a
/// column of physical type `physical`.
fn judge(property: &str, timezone: Option<&str>, physical: &str) -> Verdict {
    let columns = timezone.map_or_else(String::new, |zone| {
        format!("columns:\n  v: {{ timezone: {zone} }}\n")
    });
    let (ontology, binding) = thing(property, &columns);
    let table = vec![
        column("id", varchar()),
        column("v", PhysicalType::parse(physical)),
    ];
    verdict_of(&compare_binding(&ontology, &binding, Some(&table)))
}

type Case<'a> = (&'a str, Option<&'a str>, &'a str, Verdict);

fn run_table(cases: &[Case<'_>]) {
    use std::fmt::Write as _;
    let mut failures = String::new();
    for (property, timezone, physical, expected) in cases {
        let actual = judge(property, *timezone, physical);
        if actual != *expected {
            let _ = writeln!(
                failures,
                "{{ {property} }} tz={timezone:?} <- {physical}: expected {expected:?}, got {actual:?}"
            );
        }
    }
    assert!(failures.is_empty(), "\n{failures}");
}

#[test]
fn integers_only_widen() {
    run_table(&[
        ("type: byte", None, "tinyint", Good),
        ("type: byte", None, "smallint", E(K::TypeIncompatible)),
        ("type: byte", None, "bigint", E(K::TypeIncompatible)),
        ("type: short", None, "tinyint", Good),
        ("type: short", None, "smallint", Good),
        ("type: short", None, "integer", E(K::TypeIncompatible)),
        ("type: integer", None, "tinyint", Good),
        ("type: integer", None, "smallint", Good),
        ("type: integer", None, "integer", Good),
        ("type: integer", None, "bigint", E(K::TypeIncompatible)),
        ("type: long", None, "tinyint", Good),
        ("type: long", None, "smallint", Good),
        ("type: long", None, "integer", Good),
        ("type: long", None, "bigint", Good),
        // Across families stays an error (02-09 relies on it).
        ("type: integer", None, "varchar", E(K::TypeIncompatible)),
        ("type: integer", None, "double", E(K::TypeIncompatible)),
        (
            "type: integer",
            None,
            "decimal(10,0)",
            E(K::TypeIncompatible),
        ),
    ]);
}

#[test]
fn floats_accept_real_and_double_only_for_double() {
    run_table(&[
        ("type: float", None, "real", Good),
        ("type: float", None, "double", E(K::TypeIncompatible)),
        ("type: double", None, "real", Good),
        ("type: double", None, "double", Good),
        (
            "type: double",
            None,
            "decimal(10,2)",
            E(K::TypeIncompatible),
        ),
        ("type: double", None, "integer", E(K::TypeIncompatible)),
    ]);
}

#[test]
fn decimal_integer_digits_and_scale() {
    let d = "type: decimal, precision: 12, scale: 2";
    run_table(&[
        (d, None, "decimal(12,2)", Good),
        (d, None, "decimal(11,2)", W(K::PrecisionChanged)),
        (d, None, "decimal(11,1)", W(K::PrecisionChanged)),
        (d, None, "decimal(10,0)", W(K::PrecisionChanged)),
        (d, None, "decimal(13,2)", E(K::TypeIncompatible)),
        (d, None, "decimal(12,3)", E(K::TypeIncompatible)),
        // 11 integer digits do not fit the 10 the property allows.
        (d, None, "decimal(12,1)", E(K::TypeIncompatible)),
        (d, None, "decimal(3,3)", E(K::TypeIncompatible)),
        (d, None, "double", E(K::TypeIncompatible)),
        (d, None, "integer", E(K::TypeIncompatible)),
    ]);
}

#[test]
fn decimal_without_precision_uses_the_effective_defaults() {
    let d = "type: decimal";
    run_table(&[
        (d, None, "decimal(38,10)", Good),
        (d, None, "decimal(20,2)", W(K::PrecisionChanged)),
        (d, None, "decimal(38,11)", E(K::TypeIncompatible)),
        (d, None, "decimal(38,9)", E(K::TypeIncompatible)),
    ]);
}

#[test]
fn unbounded_numbers_are_refused_with_a_cast_hint() {
    let (ontology, binding) = thing("type: decimal, precision: 12, scale: 2", "");
    let table = vec![column("id", varchar()), column("v", PhysicalType::Number)];
    let found = compare_binding(&ontology, &binding, Some(&table));
    assert_eq!(verdict_of(&found), E(K::UnsupportedType));
    let help = found[0].help.as_deref().expect("help");
    assert!(help.contains("cast"), "{help}");
}

#[test]
fn strings_booleans_and_dates() {
    run_table(&[
        ("type: string", None, "varchar", Good),
        ("type: string", None, "varchar(255)", Good),
        ("type: string", None, "char(3)", W(K::CharPadding)),
        ("type: string", None, "integer", E(K::TypeIncompatible)),
        ("type: boolean", None, "boolean", Good),
        ("type: boolean", None, "varchar", E(K::TypeIncompatible)),
        ("type: boolean", None, "tinyint", E(K::TypeIncompatible)),
        ("type: date", None, "date", Good),
        ("type: date", None, "timestamp(3)", E(K::TypeIncompatible)),
        ("type: date", None, "varchar", E(K::TypeIncompatible)),
    ]);
}

#[test]
fn timestamps_are_checked_against_the_declared_zone() {
    let t = "type: timestamp";
    run_table(&[
        (t, Some("Europe/Berlin"), "timestamp(3)", Good),
        (
            t,
            Some("Europe/Berlin"),
            "timestamp(3) with time zone",
            W(K::TimezoneMismatch),
        ),
        (t, Some("aware"), "timestamp(6) with time zone", Good),
        (t, Some("aware"), "timestamp(3)", E(K::TimezoneMismatch)),
        (t, None, "timestamp(3)", E(K::TimezoneMismatch)),
        (
            t,
            None,
            "timestamp(3) with time zone",
            W(K::TimezoneMismatch),
        ),
        (t, Some("Europe/Berlin"), "date", E(K::TypeIncompatible)),
        (t, Some("Europe/Berlin"), "varchar", E(K::TypeIncompatible)),
    ]);
}

#[test]
fn arrays_are_compared_by_element() {
    let items_decimal = "type: array, items: decimal, precision: 10, scale: 2";
    run_table(&[
        ("type: array, items: integer", None, "array(integer)", Good),
        ("type: array, items: integer", None, "array(smallint)", Good),
        (
            "type: array, items: integer",
            None,
            "array(bigint)",
            E(K::TypeIncompatible),
        ),
        ("type: array, items: string", None, "array(varchar)", Good),
        (
            "type: array, items: string",
            None,
            "array(char(2))",
            W(K::CharPadding),
        ),
        (items_decimal, None, "array(decimal(10,2))", Good),
        (
            items_decimal,
            None,
            "array(decimal(9,2))",
            W(K::PrecisionChanged),
        ),
        (
            items_decimal,
            None,
            "array(decimal(11,2))",
            E(K::TypeIncompatible),
        ),
        (
            "type: array, items: integer",
            None,
            "integer",
            E(K::TypeIncompatible),
        ),
        (
            "type: array, items: integer",
            None,
            "varchar",
            E(K::TypeIncompatible),
        ),
        (
            "type: integer",
            None,
            "array(integer)",
            E(K::TypeIncompatible),
        ),
        (
            "type: array, items: timestamp",
            Some("Europe/Berlin"),
            "array(timestamp(3))",
            Good,
        ),
        (
            "type: array, items: timestamp",
            Some("aware"),
            "array(timestamp(3))",
            E(K::TimezoneMismatch),
        ),
    ]);
}

#[test]
fn unsupported_types_are_errors_with_a_view_hint() {
    run_table(&[
        ("type: geopoint", None, "varchar", E(K::UnsupportedType)),
        (
            "type: string",
            None,
            "map(varchar,integer)",
            E(K::UnsupportedType),
        ),
        (
            "type: integer",
            None,
            "row(a integer)",
            E(K::UnsupportedType),
        ),
        (
            "type: array, items: geopoint",
            None,
            "array(varchar)",
            E(K::UnsupportedType),
        ),
    ]);
    let (ontology, binding) = thing("type: string", "");
    let table = vec![
        column("id", varchar()),
        column("v", PhysicalType::parse("map(varchar,integer)")),
    ];
    let found = compare_binding(&ontology, &binding, Some(&table));
    let help = found[0].help.as_deref().expect("help");
    assert!(help.contains("view") && help.contains("cast"), "{help}");
    assert!(found[0].message.contains("map(varchar,integer)"));
}

#[test]
fn warnings_point_at_the_binding_like_errors() {
    let (ontology, binding) = thing("type: string", "columns:\n  v: { column: vv }\n");
    let table = vec![
        column("id", varchar()),
        column("vv", PhysicalType::Char { length: 3 }),
    ];
    let found = compare_binding(&ontology, &binding, Some(&table));
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].kind, K::CharPadding);
    assert_eq!(found[0].severity, Severity::Warning);
    assert_eq!(
        found[0].path,
        [
            Seg::Key("columns".into()),
            Seg::Key("v".into()),
            Seg::Key("column".into())
        ]
    );
    assert!(found[0].message.contains("char(3)"));
}

#[test]
fn case_collision() {
    // The default (same name) column has two candidates: the choice would be arbitrary.
    let (ontology, binding) = thing("type: string", "");
    let table = vec![
        column("id", varchar()),
        column("V", varchar()),
        column("v", varchar()),
    ];
    let found = compare_binding(&ontology, &binding, Some(&table));
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].kind, K::CaseCollision);
    assert_eq!(found[0].severity, Severity::Error);
    assert_eq!(
        found[0].path,
        [Seg::Key("source".into()), Seg::Key("table".into())]
    );
    assert!(found[0].message.contains("`V`") && found[0].message.contains("`v`"));
    assert!(found[0].message.contains("ambiguous"));

    // No type comparison happens on an ambiguous column, even when the types disagree.
    let table = vec![
        column("id", varchar()),
        column("V", PhysicalType::Integer),
        column("v", varchar()),
    ];
    let found = compare_binding(&ontology, &binding, Some(&table));
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].kind, K::CaseCollision);

    // An explicit mapping points at the column value.
    let (ontology, binding) = thing("type: string", "columns:\n  v: { column: Val }\n");
    let table = vec![
        column("id", varchar()),
        column("Val", varchar()),
        column("val", varchar()),
    ];
    let found = compare_binding(&ontology, &binding, Some(&table));
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].kind, K::CaseCollision);
    assert_eq!(
        found[0].path,
        [
            Seg::Key("columns".into()),
            Seg::Key("v".into()),
            Seg::Key("column".into())
        ]
    );

    // A clash between columns the binding does not use is nobody's business.
    let (ontology, binding) = thing("type: string", "");
    let table = vec![
        column("id", varchar()),
        column("v", varchar()),
        column("Note", varchar()),
        column("note", varchar()),
    ];
    assert_eq!(
        compare_binding(&ontology, &binding, Some(&table)),
        Vec::new()
    );
}

const FLIGHT_CREW_OBJECTS: &str = "\
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
  employeeId: { type: integer, required: true }
backing: source
---
kind: LinkType
apiName: flightCrew
cardinality: manyToMany
from: { objectType: Flight, apiName: crew }
to: { objectType: Pilot, apiName: assignedFlights }
backing: source
";

const FLIGHT_CREW_BINDING: &str = "\
kind: SourceBinding
target: { linkType: flightCrew }
source: { catalog: demo, schema: s, table: flight_crew }
columns:
  from: { column: flight_id }
  to: { column: pilot_id }
";

#[test]
fn link_ends_are_compared_with_the_primary_keys() {
    let out = load_from_memory(&[
        ("ontology.yaml", "apiName: demo\ndefaultProfile: dev\n"),
        ("objects.yaml", FLIGHT_CREW_OBJECTS),
        ("bindings/dev/flight-crew.yaml", FLIGHT_CREW_BINDING),
    ]);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    let ontology = out.ontology.expect("ontology");
    let binding = ontology
        .profile_bindings(&dev())
        .expect("dev bindings")
        .links
        .values()
        .next()
        .expect("flightCrew binding")
        .clone();

    let table = |from: PhysicalType, to: PhysicalType| {
        vec![column("flight_id", from), column("pilot_id", to)]
    };
    // Flight's key is a string, Pilot's an integer.
    assert_eq!(
        compare_binding(
            &ontology,
            &binding,
            Some(&table(varchar(), PhysicalType::Integer))
        ),
        Vec::new()
    );
    let found = compare_binding(
        &ontology,
        &binding,
        Some(&table(PhysicalType::Integer, PhysicalType::Integer)),
    );
    assert_eq!(verdict_of(&found), E(K::TypeIncompatible));
    assert_eq!(
        found[0].path,
        [
            Seg::Key("columns".into()),
            Seg::Key("from".into()),
            Seg::Key("column".into())
        ]
    );
    assert!(found[0].message.contains("flight_id"));
    let found = compare_binding(
        &ontology,
        &binding,
        Some(&table(varchar(), PhysicalType::Bigint)),
    );
    assert_eq!(verdict_of(&found), E(K::TypeIncompatible));
    assert_eq!(found[0].path[1], Seg::Key("to".into()));
}
