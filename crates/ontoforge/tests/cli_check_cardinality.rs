//! Explicit source OneToOne checks through the real CLI and SQLite fixture engine.

use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;
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

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    std::fs::write(path, text).expect("write");
}

fn ontology(csv: &str) -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    write(
        dir.path(),
        "ontology.yaml",
        "apiName: demo\ndefaultProfile: dev\n",
    );
    write(dir.path(), "objects.yaml", OBJECTS);
    write(dir.path(), "bindings/dev/record.yaml", BINDING);
    write(dir.path(), "bindings/dev/fixtures/records.csv", csv);
    dir
}

fn run(root: &Path, format: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ontoforge"))
        .arg("validate")
        .arg(root)
        .args([
            "--online",
            "--check-cardinality",
            "--format",
            format,
            "--color",
            "never",
        ])
        .env_remove("ONTOFORGE_PROFILE")
        .env("NO_COLOR", "1")
        .output()
        .expect("CLI")
}

#[test]
fn cardinality_tracer_reports_conflict() {
    let dir = ontology("record_id,aircraft_id\nM1,A1\nM2,A1\n");
    let out = run(dir.path(), "json");
    assert_eq!(
        out.status.code(),
        Some(0),
        "expected warning exit0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let diagnostics: Vec<Value> = serde_json::from_slice(&out.stdout).expect("diagnostics JSON");
    let warnings: Vec<_> = diagnostics
        .iter()
        .filter(|d| d["code"] == "ONT-W18")
        .collect();
    assert_eq!(warnings.len(), 1, "{diagnostics:#?}");
    let warning = warnings[0];
    assert_eq!(warning["severity"], "warning");
    let message = warning["message"].as_str().expect("message");
    for expected in ["aircraftMaintenance", "aircraft_id", "A1", "2"] {
        assert!(message.contains(expected), "{message}");
    }
    assert!(
        warning["file"]
            .as_str()
            .expect("file")
            .ends_with("bindings/dev/record.yaml"),
        "{warning}"
    );
    assert_eq!(warning["line"], 6, "FK mapping location: {warning}");
    assert!(
        warning["help"]
            .as_str()
            .expect("help")
            .contains("referential integrity")
    );

    let human = run(dir.path(), "human");
    assert_eq!(human.status.code(), Some(0));
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&human.stdout),
        String::from_utf8_lossy(&human.stderr)
    );
    assert!(text.contains("ONT-W18"), "{text}");
    assert!(text.contains("1 warning"), "{text}");
    assert!(
        !text.contains("ok: ontology"),
        "warning must not be summarized as clean: {text}"
    );
}

fn file_snapshot(root: &Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    fn collect(root: &Path, dir: &Path, files: &mut Vec<(std::path::PathBuf, Vec<u8>)>) {
        for entry in std::fs::read_dir(dir).expect("read directory") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                collect(root, &path, files);
            } else {
                files.push((
                    path.strip_prefix(root).expect("relative").to_owned(),
                    std::fs::read(path).expect("read file"),
                ));
            }
        }
    }
    let mut files = Vec::new();
    collect(root, root, &mut files);
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

#[test]
fn repeated_check_is_read_only() {
    let dir = ontology("record_id,aircraft_id\nM4,B\nM2,A\nM1,A\nM3,B\nM5,C\nM6,C\nM7,C\nM1,A\n");
    let before = file_snapshot(dir.path());
    let first = run(dir.path(), "json");
    let second = run(dir.path(), "json");
    assert_eq!(
        first.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert_eq!(second.status.code(), Some(0));
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(first.stderr, second.stderr);
    let diagnostics: Vec<Value> = serde_json::from_slice(&first.stdout).expect("JSON");
    let warning = diagnostics
        .iter()
        .find(|d| d["code"] == "ONT-W18")
        .expect("warning");
    let message = warning["message"].as_str().expect("message");
    assert!(
        message
            .contains("`C`: 3 distinct objects, `A`: 2 distinct objects, `B`: 2 distinct objects"),
        "{message}"
    );
    assert_eq!(
        before,
        file_snapshot(dir.path()),
        "no source changes or edit store files"
    );
}
