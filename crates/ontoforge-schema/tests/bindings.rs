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
    let ok_csv = format!("bindings/{}/fixtures/flights.csv", "a".repeat(32));
    let out = load_with(&[
        (ok.as_str(), FLIGHT_BINDING),
        (ok_csv.as_str(), FLIGHTS_CSV),
    ]);
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
        ("bindings/dev/fixtures/flights.csv", FLIGHTS_CSV),
        ("bindings/prod/a.yaml", FLIGHT_BINDING),
        ("bindings/prod/fixtures/flights.csv", FLIGHTS_CSV),
    ]);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    let ontology = out.ontology.expect("ontology");
    assert_eq!(ontology.bindings.len(), 2);
}

#[test]
fn unknown_target_is_v09() {
    let text = FLIGHT_BINDING.replace("Flight", "Flihgt");
    let out = load_with(&[
        ("bindings/dev/flight.yaml", text.as_str()),
        ("bindings/dev/fixtures/flights.csv", FLIGHTS_CSV),
    ]);
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
        "apiName: demo\ndefaultProfile: devv\n",
    ));
    files.extend([
        ("objects.yaml".to_owned(), FLIGHT),
        ("pilots.yaml".to_owned(), PILOT),
        ("links.yaml".to_owned(), CREW),
    ]);
    let borrowed: Vec<(&str, &str)> = files.iter().map(|(p, t)| (p.as_str(), *t)).collect();
    let out = load_from_memory(&borrowed);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    let found = selected(&out, Some(("devv", ProfileOrigin::Manifest)));
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

// ---------------------------------------------------------------------------
// Structural checks of one binding: columns, identifiers, fixtures, time zones
// ---------------------------------------------------------------------------

/// A Flight binding with `extra` appended to the document.
fn flight_binding_with(extra: &str) -> String {
    format!(
        "kind: SourceBinding\ntarget: {{ objectType: Flight }}\nsource: {{ catalog: demo, schema: aviation, table: flights }}\n{extra}"
    )
}

/// Load the shared definitions with one extra binding file and no fixture file.
fn load_binding(text: &str) -> LoadOutput {
    load_with(&[("bindings/dev/flight.yaml", text)])
}

#[test]
fn column_key_unknown_property_is_v09() {
    let text = flight_binding_with("columns:\n  flightNumbr: { column: flight_no }\n");
    let out = load_binding(&text);
    let d = only(&out);
    assert_eq!(d.code, codes::V09);
    assert_eq!(d.file, "bindings/dev/flight.yaml");
    assert_eq!((d.line, d.column), (5, 3), "points at the key");
    assert_eq!(d.help.as_deref(), Some("did you mean `flightNumber`?"));
}

#[test]
fn link_columns_need_from_and_to() {
    let only_from = "kind: SourceBinding\ntarget: { linkType: flightCrew }\nsource: { catalog: demo, schema: aviation, table: flight_crew }\ncolumns:\n  from: { column: flight_id }\n";
    let out = load_with(&[("bindings/dev/crew.yaml", only_from)]);
    let d = only(&out);
    assert_eq!(d.code, codes::V16);
    assert!(d.message.contains("`to`"), "{}", d.message);

    let none = "kind: SourceBinding\ntarget: { linkType: flightCrew }\nsource: { catalog: demo, schema: aviation, table: flight_crew }\n";
    let out = load_with(&[("bindings/dev/crew.yaml", none)]);
    assert_eq!(out.diagnostics.len(), 2, "{:#?}", out.diagnostics);
    assert!(out.diagnostics.iter().all(|d| d.code == codes::V16));

    let extra = format!("{CREW_BINDING}  via: {{ column: x }}\n");
    let out = load_with(&[("bindings/dev/crew.yaml", extra.as_str())]);
    let d = only(&out);
    assert_eq!(d.code, codes::V09);
    assert!(d.message.contains("via"), "{}", d.message);
}

fn source_binding(catalog: &str, schema: &str, table: &str) -> String {
    format!(
        "kind: SourceBinding\ntarget: {{ objectType: Flight }}\nsource:\n  catalog: \"{catalog}\"\n  schema: \"{schema}\"\n  table: \"{table}\"\n"
    )
}

#[test]
fn physical_identifier_rules() {
    // `\x07` is a bell character once YAML has unescaped it.
    let cases = [
        ("", "aviation", "flights", "catalog", 4),
        ("demo", "a\\x07", "flights", "schema", 5),
        ("demo", "aviation", "", "table", 6),
        ("demo", "aviation", "x\\x07y", "table", 6),
    ];
    for (catalog, schema, table, key, line) in cases {
        let out = load_binding(&source_binding(catalog, schema, table));
        let d = only(&out);
        assert_eq!(d.code, codes::V16, "{key}");
        assert_eq!(d.line, line, "{key}: points at the value");
        assert!(
            d.message.contains(&format!("`{key}`")),
            "{key}: {}",
            d.message
        );
    }
    let out = load_binding(&source_binding("demo", "aviation", "flights"));
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);

    let text = flight_binding_with("columns:\n  status: { column: \"\" }\n");
    let out = load_binding(&text);
    let d = only(&out);
    assert_eq!(d.code, codes::V16);
    assert_eq!(d.line, 5, "points at the empty column value");
    assert!(d.message.contains("column"), "{}", d.message);
}

#[test]
fn fixture_reference_rules() {
    for (file, needle) in [
        ("../../../x.csv", "leaves the ontology directory"),
        ("fixtures/x.txt", "`.csv` or `.json`"),
        ("/abs.csv", "relative"),
        ("fixtures/missing.csv", "not found"),
    ] {
        let text = flight_binding_with(&format!("fixture: {{ file: {file} }}\n"));
        let out = load_binding(&text);
        let d = only(&out);
        assert_eq!(d.code, codes::V16, "{file}");
        assert_eq!(d.file, "bindings/dev/flight.yaml");
        assert_eq!(
            (d.line, d.column),
            (4, 18),
            "{file}: points at fixture.file"
        );
        assert!(d.message.contains(needle), "{file}: {}", d.message);
    }
    // A fixture that exists is accepted, and its content is not read.
    let text = flight_binding_with("fixture: { file: fixtures/rows.csv }\n");
    let out = load_with(&[
        ("bindings/dev/flight.yaml", text.as_str()),
        (
            "bindings/dev/fixtures/rows.csv",
            "\u{0}not,even\u{1},a header",
        ),
    ]);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
}

#[test]
fn oversized_fixture_is_v16() {
    use camino::Utf8Path;
    use ontoforge_schema::load_dir;
    use ontoforge_schema::loader::MAX_FILE_BYTES;

    let dir = tempfile::tempdir().expect("tempdir");
    let write = |name: &str, text: &str| {
        let path = dir.path().join(name);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        std::fs::write(path, text).expect("write");
    };
    write("ontology.yaml", MANIFEST);
    write("objects.yaml", FLIGHT);
    write("pilots.yaml", PILOT);
    write("links.yaml", CREW);
    let text = flight_binding_with("fixture: { file: fixtures/big.csv }\n");
    write("bindings/dev/flight.yaml", &text);
    let big = dir.path().join("bindings/dev/fixtures/big.csv");
    std::fs::create_dir_all(big.parent().expect("parent")).expect("mkdir");
    std::fs::write(
        &big,
        vec![b'a'; usize::try_from(MAX_FILE_BYTES + 1).expect("fits")],
    )
    .expect("write big fixture");

    let root = Utf8Path::from_path(dir.path()).expect("utf-8 temp dir");
    let out = load_dir(root);
    let d = only(&out);
    assert_eq!(d.code, codes::V16);
    assert!(d.message.contains("over the"), "{}", d.message);
    assert_eq!(d.file, "bindings/dev/flight.yaml");

    // At the limit the file is accepted.
    std::fs::write(
        &big,
        vec![b'a'; usize::try_from(MAX_FILE_BYTES).expect("fits")],
    )
    .expect("write limit fixture");
    let out = load_dir(root);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);

    // A missing file is reported as such.
    std::fs::remove_file(&big).expect("remove");
    let out = load_dir(root);
    let d = only(&out);
    assert!(d.message.contains("not found"), "{}", d.message);

    // A directory is not a fixture.
    std::fs::create_dir_all(&big).expect("mkdir big.csv");
    let out = load_dir(root);
    let d = only(&out);
    assert_eq!(d.code, codes::V16);
    assert!(d.message.contains("not a regular file"), "{}", d.message);
}

#[cfg(unix)]
#[test]
fn symlinked_fixture_is_v16() {
    use camino::Utf8Path;
    use ontoforge_schema::load_dir;

    let dir = tempfile::tempdir().expect("tempdir");
    let write = |name: &str, text: &str| {
        let path = dir.path().join(name);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        std::fs::write(path, text).expect("write");
    };
    write("ontology.yaml", MANIFEST);
    write("objects.yaml", FLIGHT);
    write("pilots.yaml", PILOT);
    write("links.yaml", CREW);
    write(
        "bindings/dev/flight.yaml",
        &flight_binding_with("fixture: { file: fixtures/link.csv }\n"),
    );
    write("real.csv", "a,b\n");
    std::fs::create_dir_all(dir.path().join("bindings/dev/fixtures")).expect("mkdir");
    std::os::unix::fs::symlink(
        dir.path().join("real.csv"),
        dir.path().join("bindings/dev/fixtures/link.csv"),
    )
    .expect("symlink");
    let out = load_dir(Utf8Path::from_path(dir.path()).expect("utf-8"));
    let d = only(&out);
    assert_eq!(d.code, codes::V16);
    assert!(d.message.contains("not a regular file"), "{}", d.message);
}

#[test]
fn timezone_value_rules() {
    for ok in ["Europe/Berlin", "aware", "UTC", "Asia/Shanghai"] {
        let text = flight_binding_with(&format!("columns:\n  departedAt: {{ timezone: {ok} }}\n"));
        let out = load_binding(&text);
        assert!(out.diagnostics.is_empty(), "{ok}: {:#?}", out.diagnostics);
    }
    for bad in ["Mars/Base", "berlin", "+02:00", "Aware"] {
        let text = flight_binding_with(&format!(
            "columns:\n  departedAt: {{ timezone: \"{bad}\" }}\n"
        ));
        let out = load_binding(&text);
        let d = only(&out);
        assert_eq!(d.code, codes::V16, "{bad}");
        assert!(
            d.message.contains("IANA time zone name or `aware`"),
            "{bad}: {}",
            d.message
        );
        assert!(d.message.contains(bad), "{bad}: {}", d.message);
        assert_eq!(d.line, 5, "{bad}: points at the value");
    }
}

#[test]
fn precision_on_non_decimal_is_v06() {
    let object = |property: &str| {
        format!(
            "kind: ObjectType\napiName: Invoice\nprimaryKey: id\nproperties:\n  id: {{ type: string, required: true }}\n  {property}\nbacking: editOnly\n"
        )
    };
    let bad = object("ref: { type: string, precision: 10 }");
    let out = load_from_memory(&[("ontology.yaml", "apiName: demo\n"), ("invoice.yaml", &bad)]);
    let d = only(&out);
    assert_eq!(d.code, codes::V06);
    assert_eq!(d.file, "invoice.yaml");
    assert_eq!((d.line, d.column), (6, 24), "points at the precision key");

    let bad = object("ref: { type: array, items: string, scale: 2 }");
    let out = load_from_memory(&[("ontology.yaml", "apiName: demo\n"), ("invoice.yaml", &bad)]);
    assert_eq!(only(&out).code, codes::V06);

    for ok in [
        "amount: { type: decimal, precision: 12, scale: 2 }",
        "amounts: { type: array, items: decimal, precision: 12, scale: 2 }",
        "plain: { type: decimal }",
    ] {
        let good = object(ok);
        let out = load_from_memory(&[
            ("ontology.yaml", "apiName: demo\n"),
            ("invoice.yaml", &good),
        ]);
        assert!(out.diagnostics.is_empty(), "{ok}: {:#?}", out.diagnostics);
    }
}
