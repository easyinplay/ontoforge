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

// ---------------------------------------------------------------------------
// Profile selection and completeness (D-02, D-05)
// ---------------------------------------------------------------------------

use ontoforge_schema::validate::{ProfileOrigin, check_profile};

const PILOT_BINDING: &str = "\
kind: SourceBinding
target: { objectType: Pilot }
source: { catalog: demo, schema: aviation, table: pilots }
columns:
  employeeId: { column: employee_id }
  fullName: { column: full_name }
";

const GATE: &str = "\
kind: ObjectType
apiName: Gate
primaryKey: id
properties:
  id: { type: string, required: true }
backing: editOnly
";

fn profile(name: &str) -> ProfileName {
    name.to_owned().try_into().expect("profile name")
}

/// Run `check_profile` on a loaded ontology.
fn selected(out: &LoadOutput, selection: Option<(&str, ProfileOrigin)>) -> Vec<Diagnostic> {
    let ontology = out.ontology.as_ref().expect("ontology");
    let chosen = selection.map(|(name, origin)| (profile(name), origin));
    check_profile(
        ontology,
        &out.sources,
        chosen.as_ref().map(|(p, o)| (p, *o)),
    )
}

fn complete(profile_dir: &str) -> Vec<(String, &'static str)> {
    vec![
        (
            format!("bindings/{profile_dir}/flight.yaml"),
            FLIGHT_BINDING,
        ),
        (format!("bindings/{profile_dir}/pilot.yaml"), PILOT_BINDING),
        (format!("bindings/{profile_dir}/crew.yaml"), CREW_BINDING),
        (
            format!("bindings/{profile_dir}/fixtures/flights.csv"),
            FLIGHTS_CSV,
        ),
    ]
}

fn load_complete(profiles: &[&str]) -> LoadOutput {
    let mut extra: Vec<(String, &'static str)> = Vec::new();
    for p in profiles {
        extra.extend(complete(p));
    }
    let borrowed: Vec<(&str, &str)> = extra.iter().map(|(p, t)| (p.as_str(), *t)).collect();
    load_with(&borrowed)
}

#[test]
fn complete_profile_has_no_diagnostics() {
    let out = load_complete(&["dev"]);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    assert!(selected(&out, Some(("dev", ProfileOrigin::Manifest))).is_empty());
}

#[test]
fn source_type_without_binding_is_v16() {
    let out = load_with(&[
        ("bindings/dev/flight.yaml", FLIGHT_BINDING),
        ("bindings/dev/crew.yaml", CREW_BINDING),
        ("bindings/dev/fixtures/flights.csv", FLIGHTS_CSV),
    ]);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    let found = selected(&out, Some(("dev", ProfileOrigin::Manifest)));
    assert_eq!(found.len(), 1, "{found:#?}");
    let d = &found[0];
    assert_eq!(d.code, codes::V16);
    assert_eq!(d.file, "pilots.yaml");
    assert_eq!((d.line, d.column), (7, 10), "points at the backing value");
    assert!(
        d.message.contains("Pilot") && d.message.contains("dev"),
        "{}",
        d.message
    );
    let help = d.help.as_deref().unwrap_or_default();
    assert!(help.contains("target: { objectType: Pilot }"), "{help}");
    assert!(help.contains("bindings/dev/"), "{help}");
}

#[test]
fn many_to_many_without_binding_is_v16() {
    let out = load_with(&[
        ("bindings/dev/flight.yaml", FLIGHT_BINDING),
        ("bindings/dev/pilot.yaml", PILOT_BINDING),
        ("bindings/dev/fixtures/flights.csv", FLIGHTS_CSV),
    ]);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    let found = selected(&out, Some(("dev", ProfileOrigin::Manifest)));
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].code, codes::V16);
    assert_eq!(found[0].file, "links.yaml");
    assert_eq!((found[0].line, found[0].column), (4, 10));
    let help = found[0].help.as_deref().unwrap_or_default();
    assert!(help.contains("target: { linkType: flightCrew }"), "{help}");
}

#[test]
fn edit_only_type_with_binding_is_v16() {
    let text = "kind: SourceBinding\ntarget: { objectType: Gate }\nsource: { catalog: demo, schema: aviation, table: gates }\n";
    let out = load_with(&[("gates.yaml", GATE), ("bindings/prod/gate.yaml", text)]);
    let d = only(&out);
    assert_eq!(d.code, codes::V16);
    assert_eq!(d.file, "bindings/prod/gate.yaml");
    assert_eq!((d.line, d.column), (2, 23));
    assert!(d.message.contains("editOnly"), "{}", d.message);
    assert!(
        d.help
            .as_deref()
            .unwrap_or_default()
            .contains("backing: source"),
        "{d:?}"
    );
}

#[test]
fn one_to_many_link_binding_is_v16() {
    let leg = "kind: ObjectType\napiName: Leg\nprimaryKey: legId\nproperties:\n  legId: { type: string, required: true }\n  gateId: { type: string }\nbacking: editOnly\n";
    let link = "kind: LinkType\napiName: gateLegs\ncardinality: oneToMany\nfrom: { objectType: Gate, apiName: legs }\nto: { objectType: Leg, apiName: gate, foreignKey: gateId }\n";
    let binding = "kind: SourceBinding\ntarget: { linkType: gateLegs }\nsource: { catalog: demo, schema: aviation, table: gate_legs }\ncolumns:\n  from: { column: a }\n  to: { column: b }\n";
    let out = load_with(&[
        ("gates.yaml", GATE),
        ("legs.yaml", leg),
        ("gate-legs.yaml", link),
        ("bindings/prod/gate-legs.yaml", binding),
    ]);
    let d = only(&out);
    assert_eq!(d.code, codes::V16);
    assert_eq!(d.file, "bindings/prod/gate-legs.yaml");
    assert!(d.message.contains("manyToMany"), "{}", d.message);
}

#[test]
fn unknown_profile_from_flag_is_v16_with_suggestion() {
    let out = load_complete(&["dev", "prod"]);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    let found = selected(&out, Some(("prdo", ProfileOrigin::Flag)));
    assert_eq!(found.len(), 1, "{found:#?}");
    let d = &found[0];
    assert_eq!(d.code, codes::V16);
    assert_eq!(d.file, "ontology.yaml");
    assert_eq!(d.line, 0, "a flag has no file position");
    assert!(d.message.contains("prdo"), "{}", d.message);
    assert_eq!(d.help.as_deref(), Some("did you mean `prod`?"));
}

#[test]
fn unknown_profile_from_manifest_points_at_default_profile() {
    let mut files: Vec<(String, &'static str)> = complete("dev");
    files.push((
        "ontology.yaml".to_owned(),
        "apiName: demo\ndefaultProfile: prdo\n",
    ));
    files.extend([
        ("objects.yaml".to_owned(), FLIGHT),
        ("pilots.yaml".to_owned(), PILOT),
        ("links.yaml".to_owned(), CREW),
    ]);
    let borrowed: Vec<(&str, &str)> = files.iter().map(|(p, t)| (p.as_str(), *t)).collect();
    let out = load_from_memory(&borrowed);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    let found = selected(&out, Some(("prdo", ProfileOrigin::Manifest)));
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].code, codes::V16);
    assert_eq!(found[0].file, "ontology.yaml");
    assert_eq!((found[0].line, found[0].column), (2, 17));
    assert_eq!(found[0].help.as_deref(), Some("did you mean `dev`?"));
}

#[test]
fn no_profile_selected_is_v16() {
    let out = load_complete(&["dev"]);
    let found = selected(&out, None);
    assert_eq!(found.len(), 1, "{found:#?}");
    let d = &found[0];
    assert_eq!(d.code, codes::V16);
    assert_eq!(d.file, "ontology.yaml");
    let help = d.help.as_deref().unwrap_or_default();
    for way in ["--profile", "ONTOFORGE_PROFILE", "defaultProfile"] {
        assert!(help.contains(way), "{way} missing from {help}");
    }
}

#[test]
fn all_edit_only_needs_no_profile() {
    let out = load_from_memory(&[("ontology.yaml", "apiName: demo\n"), ("gates.yaml", GATE)]);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    assert!(selected(&out, None).is_empty());
}
