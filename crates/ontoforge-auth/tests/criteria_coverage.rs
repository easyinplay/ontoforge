//! Criteria column coverage (plan 03-14 Task 2, SCOPE-06, ONT-V23).
//!
//! A submission criterion that reads `objects.<param>.<prop>` lets a caller probe the value
//! with a dry run. The role granted the action must therefore be able to read that property.

use std::collections::BTreeMap;

use camino::Utf8Path;
use ontoforge_auth::scope::{CompiledScope, ScopeInputs, compile_scope};
use ontoforge_schema::diag::{Diagnostic, codes};
use ontoforge_schema::ir::{AttrType, Ontology};
use ontoforge_schema::{load_dir, load_from_memory, load_scope_from_memory};

const AVIATION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/ontologies/aviation"
);

fn aviation() -> Ontology {
    let out = load_dir(Utf8Path::new(AVIATION));
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    out.ontology.expect("aviation ontology")
}

/// Compiles the roles in `yaml` against `ontology`, with positions.
fn compile(ontology: &Ontology, yaml: &str) -> Result<CompiledScope, Vec<Diagnostic>> {
    let out = load_scope_from_memory(&[("roles.yaml", yaml)]);
    let scope = out.scope.expect("role documents load");
    let attrs: BTreeMap<String, AttrType> = BTreeMap::new();
    compile_scope(&ScopeInputs {
        ontology,
        profile: None,
        attrs: &attrs,
        scope: &scope,
        sources: Some(&out.sources),
    })
}

fn only(result: Result<CompiledScope, Vec<Diagnostic>>) -> Diagnostic {
    let diagnostics = result.expect_err("the scope must not compile");
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    diagnostics.into_iter().next().expect("one diagnostic")
}

const COVERED: &str = "\
kind: Role
apiName: dispatcher
objectTypes:
  Flight:
    properties: [flightId, status, delayMinutes]
actions: [delayFlight]
";

#[test]
fn covered_criteria_compile() {
    compile(&aviation(), COVERED).unwrap_or_else(|d| panic!("{d:#?}"));
}

#[test]
fn hidden_criteria_column_is_v23_at_the_action_entry() {
    let yaml = COVERED.replace(
        "[flightId, status, delayMinutes]",
        "[flightId, delayMinutes]",
    );
    let d = only(compile(&aviation(), &yaml));
    assert_eq!(d.code, codes::V23);
    assert_eq!(d.file, "roles.yaml");
    // `actions: [delayFlight]` is line 6; the entry starts after `actions: [`.
    assert_eq!((d.line, d.column), (6, 11));
    assert!(d.message.contains("objects.flight.status"), "{}", d.message);
    assert!(d.message.contains("delayFlight"), "{}", d.message);
    assert!(d.message.contains("dispatcher"), "{}", d.message);
}

#[test]
fn ungranted_object_type_is_v23() {
    let yaml = "\
kind: Role
apiName: dispatcher
actions: [delayFlight]
";
    let d = only(compile(&aviation(), yaml));
    assert_eq!(d.code, codes::V23);
    assert!(d.message.contains("Flight"), "{}", d.message);
    assert!(d.message.contains("delayFlight"), "{}", d.message);
}

#[test]
fn criteria_without_object_reads_need_no_properties() {
    // `assignPilot` only reads `user.groups`.
    let yaml = "\
kind: Role
apiName: crewScheduler
actions: [assignPilot]
";
    compile(&aviation(), yaml).unwrap_or_else(|d| panic!("{d:#?}"));
}

#[test]
fn dynamic_object_path_is_v23_fail_closed() {
    let ontology = load_from_memory(&[
        ("ontology.yaml", "apiName: demo\n"),
        (
            "objects.yaml",
            "\
kind: ObjectType
apiName: Flight
backing: editOnly
primaryKey: flightId
properties:
  flightId: { type: string, required: true }
  status: { type: string }
",
        ),
        (
            "actions.yaml",
            "\
kind: ActionType
apiName: touchFlight
parameters:
  flight: { type: { objectReference: Flight }, required: true }
  field: { type: string, required: true }
rules:
  - kind: modifyObject
    object: flight
    properties:
      status: { static: TOUCHED }
submissionCriteria:
  - condition: \"objects.flight[params.field] == 'x'\"
    failureMessage: \"no\"
",
        ),
    ]);
    assert!(
        ontology.diagnostics.is_empty(),
        "{:#?}",
        ontology.diagnostics
    );
    let ontology = ontology.ontology.expect("ontology");
    let yaml = "\
kind: Role
apiName: toucher
objectTypes:
  Flight:
    properties: [flightId, status]
actions: [touchFlight]
";
    let d = only(compile(&ontology, yaml));
    assert_eq!(d.code, codes::V23);
    assert!(d.message.contains("statically"), "{}", d.message);
    assert!(d.message.contains("touchFlight"), "{}", d.message);
}

/// `Flight` (editOnly, with `status` and `gate`) and `checkFlight`, whose only submission
/// criterion is `condition`.
fn flight_checked_by(condition: &str) -> Ontology {
    let actions = format!(
        "\
kind: ActionType
apiName: checkFlight
parameters:
  flight: {{ type: {{ objectReference: Flight }}, required: true }}
  x: {{ type: string, required: true }}
rules:
  - kind: modifyObject
    object: flight
    properties:
      status: {{ static: CHECKED }}
submissionCriteria:
  - condition: \"{condition}\"
    failureMessage: \"no\"
"
    );
    let out = load_from_memory(&[
        ("ontology.yaml", "apiName: demo\n"),
        (
            "objects.yaml",
            "\
kind: ObjectType
apiName: Flight
backing: editOnly
primaryKey: flightId
properties:
  flightId: { type: string, required: true }
  status: { type: string }
  gate: { type: string }
",
        ),
        ("actions.yaml", &actions),
    ]);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    out.ontology.expect("ontology")
}

#[test]
fn whole_object_read_needs_every_property() {
    // Comparing or null-testing the object reveals whether its properties exist and what they
    // hold, so the allow-list must cover all of them.
    for condition in ["objects.flight == params.x", "objects.flight != null"] {
        let ontology = flight_checked_by(condition);
        let narrow = "\
kind: Role
apiName: checker
objectTypes:
  Flight:
    properties: [flightId, status]
actions: [checkFlight]
";
        let d = only(compile(&ontology, narrow));
        assert_eq!(d.code, codes::V23, "{condition}");
        assert!(d.message.contains("objects.flight"), "{}", d.message);
        assert!(d.message.contains("gate"), "{}", d.message);
        assert!(d.message.contains("checkFlight"), "{}", d.message);

        let wide = narrow.replace("[flightId, status]", "[flightId, status, gate]");
        compile(&ontology, &wide).unwrap_or_else(|d| panic!("{condition}: {d:#?}"));
    }
}
