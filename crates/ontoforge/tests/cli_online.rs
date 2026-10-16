//! `ontoforge validate --online` end to end, with the SQLite stand-in loaded from fixtures.

use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

const AVIATION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/ontologies/aviation"
);
const AVIATION_JSON: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/ontologies/aviation-json"
);

const MANIFEST: &str = "apiName: demo\ndefaultProfile: dev\n";

const FLIGHT: &str = "\
kind: ObjectType
apiName: Flight
primaryKey: flightId
properties:
  flightId: { type: string, required: true }
  flightNumber: { type: string }
  status: { type: string }
backing: source
";

const BINDING: &str = "\
kind: SourceBinding
target: { objectType: Flight }
source: { catalog: demo, schema: aviation, table: flights }
columns:
  flightNumber: { column: flight_no }
fixture: { file: fixtures/flights.csv }
";

const BINDING_WITHOUT_FIXTURE: &str = "\
kind: SourceBinding
target: { objectType: Flight }
source: { catalog: demo, schema: aviation, table: flights }
columns:
  flightNumber: { column: flight_no }
";

fn ontoforge(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ontoforge"))
        .args(args)
        .env_remove("ONTOFORGE_PROFILE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn ontoforge")
}

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("create directory");
    std::fs::write(path, text).expect("write file");
}

/// An ontology with one source object type `Flight`.
fn flight_ontology(binding: &str, csv: Option<&str>) -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    write(dir.path(), "ontology.yaml", MANIFEST);
    write(dir.path(), "objects.yaml", FLIGHT);
    write(dir.path(), "bindings/dev/flight.yaml", binding);
    if let Some(csv) = csv {
        write(dir.path(), "bindings/dev/fixtures/flights.csv", csv);
    }
    dir
}

fn online(dir: &Path, extra: &[&str]) -> Output {
    let dir = dir.to_str().expect("utf-8 path");
    let mut args = vec!["validate", dir, "--online", "--color", "never"];
    args.extend_from_slice(extra);
    ontoforge(&args)
}

fn diagnostics(out: &Output) -> Vec<serde_json::Value> {
    let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).expect("stdout is JSON");
    parsed.as_array().expect("JSON array").clone()
}

/// 1-based line and column of byte offset `offset` in `text`.
fn line_col(text: &str, offset: usize) -> (u64, u64) {
    let before = &text[..offset];
    let line = before.matches('\n').count() + 1;
    let column = before.rsplit('\n').next().map_or(0, |l| l.chars().count()) + 1;
    (line as u64, column as u64)
}

#[test]
fn online_aviation_is_clean() {
    for dir in [AVIATION, AVIATION_JSON] {
        let out = ontoforge(&[
            "validate", dir, "--online", "--format", "json", "--color", "never",
        ]);
        assert_eq!(
            out.status.code(),
            Some(0),
            "{dir}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "[]", "{dir}");
    }
    let out = ontoforge(&["validate", AVIATION, "--online", "--color", "never"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("online"), "{stderr}");
    assert!(stderr.contains("profile `dev`"), "{stderr}");
}

#[test]
fn online_missing_explicit_column() {
    let dir = flight_ontology(BINDING, Some("flightId,status\nF1,ok\n"));
    let out = online(dir.path(), &["--format", "json"]);
    assert_eq!(out.status.code(), Some(1));
    let diags = diagnostics(&out);
    assert_eq!(diags.len(), 1, "{diags:#?}");
    let d = &diags[0];
    assert_eq!(d["code"], "ONT-V17");
    assert_eq!(d["severity"], "error");
    assert_eq!(d["file"], "bindings/dev/flight.yaml");
    let offset = BINDING.find("flight_no").expect("column value");
    let (line, column) = line_col(BINDING, offset);
    assert_eq!(d["line"], line);
    assert_eq!(d["column"], column);
    assert_eq!(d["span"]["offset"], offset);
    assert!(d["message"].as_str().unwrap().contains("flight_no"));
}

#[test]
fn online_missing_implicit_column() {
    let dir = flight_ontology(BINDING, Some("flightId,flight_no\nF1,LH1\n"));
    let out = online(dir.path(), &["--format", "json"]);
    assert_eq!(out.status.code(), Some(1));
    let diags = diagnostics(&out);
    assert_eq!(diags.len(), 1, "{diags:#?}");
    let d = &diags[0];
    assert_eq!(d["code"], "ONT-V17");
    assert_eq!(d["file"], "bindings/dev/flight.yaml");
    let offset = BINDING.find("table: flights").expect("table value") + "table: ".len();
    assert_eq!(d["span"]["offset"], offset);
    assert!(d["message"].as_str().unwrap().contains("status"));
}

#[test]
fn online_binding_without_fixture_is_v20() {
    let dir = flight_ontology(BINDING_WITHOUT_FIXTURE, None);
    let out = online(dir.path(), &["--format", "json"]);
    assert_eq!(out.status.code(), Some(1));
    let diags = diagnostics(&out);
    assert_eq!(diags.len(), 1, "{diags:#?}");
    let d = &diags[0];
    assert_eq!(d["code"], "ONT-V20");
    assert_eq!(d["file"], "bindings/dev/flight.yaml");
    let offset = BINDING_WITHOUT_FIXTURE
        .find("{ catalog")
        .expect("source value");
    assert_eq!(d["span"]["offset"], offset);
    assert!(d["help"].as_str().unwrap().contains("--server-config"));
}

#[test]
fn online_missing_column_human() {
    let dir = flight_ontology(BINDING, Some("flightId,status\nF1,ok\n"));
    let out = online(dir.path(), &[]);
    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(stdout.contains("ONT-V17"), "{stdout}");
    assert!(stdout.contains("bindings/dev/flight.yaml:"), "{stdout}");
    insta::assert_snapshot!("online_missing_column_human", stdout);
}

#[test]
fn offline_validate_unchanged() {
    let out = ontoforge(&["validate", AVIATION, "--color", "never"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(out.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("online"), "{stderr}");
}

#[test]
fn timeout_requires_online() {
    let out = ontoforge(&["validate", AVIATION, "--timeout", "1500"]);
    assert_eq!(out.status.code(), Some(2));
    let out = ontoforge(&["validate", AVIATION, "--online", "--timeout", "1500"]);
    assert_eq!(out.status.code(), Some(0));
}

fn fixture_files(root: &str) -> Vec<(String, Vec<u8>)> {
    let dir = Path::new(root).join("bindings/dev/fixtures");
    let mut files: Vec<(String, Vec<u8>)> = std::fs::read_dir(&dir)
        .expect("fixtures directory")
        .map(|entry| {
            let path = entry.expect("entry").path();
            let name = path
                .file_name()
                .expect("name")
                .to_string_lossy()
                .into_owned();
            (name, std::fs::read(&path).expect("read fixture"))
        })
        .collect();
    files.sort();
    files
}

#[test]
fn aviation_fixture_data() {
    let files = fixture_files(AVIATION);
    assert_eq!(files.len(), 5);
    let mut all = String::new();
    for (name, bytes) in &files {
        let text = String::from_utf8(bytes.clone()).expect("utf-8 fixture");
        let records = text.lines().count().saturating_sub(1);
        assert!(records >= 20, "{name} has only {records} records");
        all.push_str(&text);
    }
    // The boundaries ADR-0021 asks the demo data to exercise.
    for needle in [
        "9999999999.99",
        "-9999999999.99",
        "0.01",
        "2026-03-29",
        "2026-10-25",
        "+01:00",
        "+02:00",
        "Z",
        "[]",
    ] {
        assert!(all.contains(needle), "no fixture row contains `{needle}`");
    }
    let out = ontoforge(&[
        "validate", AVIATION, "--online", "--format", "json", "--color", "never",
    ]);
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn aviation_json_fixtures_match_yaml_fixtures() {
    assert_eq!(fixture_files(AVIATION), fixture_files(AVIATION_JSON));
}

/// `float` reads only `real` and `double` reads `real` or `double` (D-13), so the stand-in must
/// report each property type with the physical type its own rule accepts.
#[test]
fn float_and_double_properties_are_clean() {
    const READING: &str = "\
kind: ObjectType
apiName: Reading
primaryKey: readingId
properties:
  readingId: { type: string, required: true }
  ratio: { type: float }
  total: { type: double }
backing: source
";
    const READING_BINDING: &str = "\
kind: SourceBinding
target: { objectType: Reading }
source: { catalog: demo, schema: lab, table: readings }
fixture: { file: fixtures/readings.csv }
";
    let dir = TempDir::new().expect("tempdir");
    write(dir.path(), "ontology.yaml", MANIFEST);
    write(dir.path(), "objects.yaml", READING);
    write(dir.path(), "bindings/dev/reading.yaml", READING_BINDING);
    write(
        dir.path(),
        "bindings/dev/fixtures/readings.csv",
        "readingId,ratio,total\nR1,1.5,2.25\nR2,,\n",
    );
    let out = online(dir.path(), &["--format", "json"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "[]");
}
