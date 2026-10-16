//! Source binding documents (`kind: SourceBinding`): loading, placement under
//! `bindings/<profile>/`, references and the shared helper functions on the IR.

use ontoforge_schema::diag::{Diagnostic, codes};
use ontoforge_schema::ir::ProfileName;
use ontoforge_schema::{LoadOutput, load_from_memory};

const MANIFEST: &str = "apiName: demo\ndefaultProfile: dev\n";

const FLIGHT: &str = "\
kind: ObjectType
apiName: Flight
primaryKey: flightId
properties:
  flightId: { type: string, required: true }
  flightNumber: { type: string, required: true }
  status: { type: string }
  departedAt: { type: timestamp }
backing: source
";

const PILOT: &str = "\
kind: ObjectType
apiName: Pilot
primaryKey: employeeId
properties:
  employeeId: { type: string, required: true }
  fullName: { type: string }
backing: source
";

const CREW: &str = "\
kind: LinkType
apiName: flightCrew
cardinality: manyToMany
backing: source
from: { objectType: Flight, apiName: crew }
to: { objectType: Pilot, apiName: assignedFlights }
";

const FLIGHT_BINDING: &str = "\
kind: SourceBinding
target: { objectType: Flight }
source: { catalog: demo, schema: aviation, table: flights }
columns:
  flightNumber: { column: flight_no }
  status: { collation: caseInsensitive }
  departedAt: { column: departed_at, timezone: Europe/Berlin }
fixture: { file: fixtures/flights.csv }
";

const CREW_BINDING: &str = "\
kind: SourceBinding
target: { linkType: flightCrew }
source: { catalog: demo, schema: aviation, table: flight_crew }
columns:
  from: { column: flight_id }
  to: { column: pilot_id }
";

const FLIGHTS_CSV: &str =
    "flightId,flight_no,status,departed_at\nF1,LH100,ok,2026-03-29T01:30:00\n";

/// The shared definitions plus `extra` files.
fn load_with(extra: &[(&str, &str)]) -> LoadOutput {
    let mut files = vec![
        ("ontology.yaml", MANIFEST),
        ("objects.yaml", FLIGHT),
        ("pilots.yaml", PILOT),
        ("links.yaml", CREW),
    ];
    files.extend_from_slice(extra);
    load_from_memory(&files)
}

fn only(out: &LoadOutput) -> &Diagnostic {
    assert_eq!(
        out.diagnostics.len(),
        1,
        "diagnostics: {:#?}",
        out.diagnostics
    );
    &out.diagnostics[0]
}

#[test]
fn flight_binding_loads_under_dev_profile() {
    let out = load_with(&[
        ("bindings/dev/flight.yaml", FLIGHT_BINDING),
        ("bindings/dev/fixtures/flights.csv", FLIGHTS_CSV),
    ]);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    let ontology = out.ontology.expect("ontology");
    assert_eq!(
        ontology.default_profile.as_ref().map(ProfileName::as_str),
        Some("dev")
    );
    let binding = &ontology.bindings["dev"].objects["Flight"];
    assert_eq!(binding.source.catalog, "demo");
    assert_eq!(binding.source.table, "flights");
    assert_eq!(binding.target_name(), "Flight");
    assert_eq!(binding.column_for("flightNumber"), "flight_no");
    assert_eq!(binding.column_for("status"), "status");
    assert_eq!(binding.timezone_for("departedAt"), Some("Europe/Berlin"));
    assert_eq!(binding.timezone_for("status"), None);
    assert_eq!(
        binding.fixture.as_ref().map(|f| f.file.as_str()),
        Some("fixtures/flights.csv")
    );
}

#[test]
fn binding_outside_bindings_dir_is_v16() {
    let out = load_with(&[("defs/flight.yaml", FLIGHT_BINDING)]);
    let d = only(&out);
    assert_eq!(d.code, codes::V16);
    assert_eq!(d.file, "defs/flight.yaml");
    assert_eq!((d.line, d.column), (1, 7), "points at the kind value");
    assert!(
        d.help
            .as_deref()
            .unwrap_or_default()
            .contains("bindings/<profile>/"),
        "{d:?}"
    );
    assert!(out.ontology.is_none());
}

#[test]
fn binding_directly_under_bindings_is_v16() {
    let out = load_with(&[("bindings/flight.yaml", FLIGHT_BINDING)]);
    let d = only(&out);
    assert_eq!(d.code, codes::V16);
    assert_eq!(d.file, "bindings/flight.yaml");
}

#[test]
fn definition_inside_bindings_dir_is_v16() {
    let extra = "kind: ObjectType\napiName: Gate\nprimaryKey: id\nproperties:\n  id: { type: string, required: true }\nbacking: editOnly\n";
    let out = load_with(&[("bindings/dev/x.yaml", extra)]);
    let d = only(&out);
    assert_eq!(d.code, codes::V16);
    assert_eq!(d.file, "bindings/dev/x.yaml");
    assert_eq!((d.line, d.column), (1, 7));
}

#[test]
fn bad_profile_dir_is_v16() {
    let out = load_with(&[("bindings/Prod/flight.yaml", FLIGHT_BINDING)]);
    let d = only(&out);
    assert_eq!(d.code, codes::V16);
    assert_eq!(d.file, "bindings/Prod/flight.yaml");
    assert!(d.message.contains("Prod"), "{}", d.message);
    assert!(d.message.contains("[a-z][a-z0-9_-]{0,31}"), "{}", d.message);
}

#[test]
fn profile_name_length_boundaries() {
    let ok = format!("bindings/{}/flight.yaml", "a".repeat(32));
    let out = load_with(&[(ok.as_str(), FLIGHT_BINDING)]);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    let too_long = format!("bindings/{}/flight.yaml", "a".repeat(33));
    let out = load_with(&[(too_long.as_str(), FLIGHT_BINDING)]);
    assert_eq!(only(&out).code, codes::V16);
}

#[test]
fn duplicate_binding_is_v08() {
    let out = load_with(&[
        ("bindings/dev/a.yaml", FLIGHT_BINDING),
        ("bindings/dev/b.yaml", FLIGHT_BINDING),
    ]);
    let d = only(&out);
    assert_eq!(d.code, codes::V08);
    assert_eq!(d.file, "bindings/dev/b.yaml");
    assert!(
        d.message.contains("bindings/dev/a.yaml:2:23"),
        "message names the first definition: {}",
        d.message
    );
}

#[test]
fn same_target_in_two_profiles_is_not_a_duplicate() {
    let out = load_with(&[
        ("bindings/dev/a.yaml", FLIGHT_BINDING),
        ("bindings/prod/a.yaml", FLIGHT_BINDING),
    ]);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    let ontology = out.ontology.expect("ontology");
    assert_eq!(ontology.bindings.len(), 2);
}

#[test]
fn unknown_target_is_v09() {
    let text = FLIGHT_BINDING.replace("Flight", "Flihgt");
    let out = load_with(&[("bindings/dev/flight.yaml", text.as_str())]);
    let d = only(&out);
    assert_eq!(d.code, codes::V09);
    assert_eq!(d.file, "bindings/dev/flight.yaml");
    assert_eq!((d.line, d.column), (2, 23));
    assert_eq!(d.help.as_deref(), Some("did you mean `Flight`?"));
}

#[test]
fn unknown_link_target_is_v09() {
    let text = CREW_BINDING.replace("flightCrew", "flightCrw");
    let out = load_with(&[("bindings/dev/crew.yaml", text.as_str())]);
    let d = only(&out);
    assert_eq!(d.code, codes::V09);
    assert_eq!(d.help.as_deref(), Some("did you mean `flightCrew`?"));
}

#[test]
fn unknown_binding_field_is_v04() {
    let text = format!("{FLIGHT_BINDING}profile: dev\n");
    let out = load_with(&[("bindings/dev/flight.yaml", text.as_str())]);
    let d = only(&out);
    assert_eq!(d.code, codes::V04);
    assert!(d.message.contains("profile"), "{}", d.message);
}

#[test]
fn fixtures_dir_is_not_scanned() {
    let out = load_with(&[
        ("bindings/dev/flight.yaml", FLIGHT_BINDING),
        ("bindings/dev/fixtures/flights.csv", FLIGHTS_CSV),
        ("bindings/dev/fixtures/rows.json", r#"[{"flightId":"F1"}]"#),
    ]);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
}

#[test]
fn top_level_fixtures_dir_still_loaded() {
    let gate = "kind: ObjectType\napiName: Gate\nprimaryKey: id\nproperties:\n  id: { type: string, required: true }\nbacking: editOnly\n";
    let hangar = gate.replace("Gate", "Hangar");
    let out = load_with(&[
        ("fixtures/objects.yaml", gate),
        ("defs/fixtures/x.yaml", hangar.as_str()),
    ]);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    let ontology = out.ontology.expect("ontology");
    assert!(ontology.object_types.contains_key("Gate"));
    assert!(ontology.object_types.contains_key("Hangar"));
}

#[test]
fn bound_columns_object_and_link() {
    let out = load_with(&[
        ("bindings/dev/flight.yaml", FLIGHT_BINDING),
        ("bindings/dev/crew.yaml", CREW_BINDING),
        ("bindings/dev/fixtures/flights.csv", FLIGHTS_CSV),
    ]);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    let ontology = out.ontology.expect("ontology");
    let dev = ontology.profile_bindings(&"dev".to_owned().try_into().expect("profile"));
    let dev = dev.expect("dev bindings");

    let flight = &dev.objects["Flight"];
    let columns = ontology.bound_columns(flight);
    let shown: Vec<(&str, &str, bool)> = columns
        .iter()
        .map(|c| (c.key, c.column, c.explicit))
        .collect();
    assert_eq!(
        shown,
        [
            ("flightId", "flightId", false),
            ("flightNumber", "flight_no", true),
            ("status", "status", false),
            ("departedAt", "departed_at", true),
        ]
    );
    assert_eq!(
        columns[1].property,
        &ontology.object_types["Flight"].properties["flightNumber"]
    );

    let crew = &dev.links["flightCrew"];
    let columns = ontology.bound_columns(crew);
    let shown: Vec<(&str, &str, bool)> = columns
        .iter()
        .map(|c| (c.key, c.column, c.explicit))
        .collect();
    assert_eq!(
        shown,
        [("from", "flight_id", true), ("to", "pilot_id", true)]
    );
    assert_eq!(
        columns[0].property,
        &ontology.object_types["Flight"].properties["flightId"]
    );
    assert_eq!(
        columns[1].property,
        &ontology.object_types["Pilot"].properties["employeeId"]
    );
}
