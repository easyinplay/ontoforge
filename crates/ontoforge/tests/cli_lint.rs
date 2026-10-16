//! End-to-end tests of `ontoforge lint`: exit codes, output formats, configuration.

use std::path::Path;
use std::process::{Command, Output};

const AVIATION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/ontologies/aviation"
);
const V04: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/ontologies/invalid/v04-unknown-field"
);

const MANIFEST: &str = "apiName: demo\n";

/// An addLink on a oneToMany link: ONT-E08.
const E08_OBJECTS: &str = "\
kind: ObjectType
apiName: Airport
primaryKey: code
properties:
  code: { type: string, required: true }
backing: editOnly
---
kind: ObjectType
apiName: Flight
primaryKey: flightId
properties:
  flightId: { type: string, required: true }
  originCode: { type: string }
backing: editOnly
---
kind: LinkType
apiName: departures
cardinality: oneToMany
from: { objectType: Airport, apiName: departingFlights }
to: { objectType: Flight, apiName: origin, foreignKey: originCode }
";

const E08_ACTIONS: &str = "\
kind: ActionType
apiName: addDeparture
parameters:
  airport: { type: { objectReference: Airport }, required: true }
  flight: { type: { objectReference: Flight }, required: true }
rules:
  - { kind: addLink, link: departures, from: airport, to: flight }
";

/// A timestamp primary key: ONT-E09 as a warning.
const WARN_OBJECTS: &str = "\
kind: ObjectType
apiName: Event
primaryKey: at
properties:
  at: { type: timestamp, required: true }
backing: editOnly
";

fn write(dir: &Path, name: &str, text: &str) {
    std::fs::write(dir.join(name), text).unwrap();
}

fn e08_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "ontology.yaml", MANIFEST);
    write(dir.path(), "objects.yaml", E08_OBJECTS);
    write(dir.path(), "actions.yaml", E08_ACTIONS);
    dir
}

fn warn_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "ontology.yaml", MANIFEST);
    write(dir.path(), "objects.yaml", WARN_OBJECTS);
    dir
}

fn ontoforge(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ontoforge"))
        .args(args)
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn ontoforge")
}

/// Run `ontoforge lint <dir> --format json <extra>` and parse stdout.
fn lint_json(dir: &str, extra: &[&str]) -> (Output, Vec<serde_json::Value>) {
    let mut args = vec!["lint", dir, "--format", "json"];
    args.extend_from_slice(extra);
    let out = ontoforge(&args);
    let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).expect("stdout is JSON");
    (out, parsed.as_array().expect("JSON array").clone())
}

fn path(dir: &tempfile::TempDir) -> &str {
    dir.path().to_str().unwrap()
}

fn codes(diags: &[serde_json::Value]) -> Vec<&str> {
    diags.iter().map(|d| d["code"].as_str().unwrap()).collect()
}

#[test]
fn lint_aviation_is_clean() {
    let (out, diags) = lint_json(AVIATION, &[]);
    assert_eq!(out.status.code(), Some(0));
    assert!(diags.is_empty(), "{diags:#?}");
    let (out, diags) = lint_json(AVIATION, &["--deny-warnings"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(diags.is_empty());
}

#[test]
fn lint_error_rule_exits_1() {
    let dir = e08_dir();
    let (out, diags) = lint_json(path(&dir), &[]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(codes(&diags), ["ONT-E08"]);
    assert_eq!(diags[0]["severity"], "error");
    assert_eq!(diags[0]["file"], "actions.yaml");
    assert_eq!(diags[0]["line"], 7);
}

#[test]
fn warning_only_exit_codes() {
    let dir = warn_dir();
    let (out, diags) = lint_json(path(&dir), &[]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(codes(&diags), ["ONT-E09"]);
    assert_eq!(diags[0]["severity"], "warning");
    let (out, _) = lint_json(path(&dir), &["--deny-warnings"]);
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn config_off_disables_rule() {
    let dir = e08_dir();
    write(
        dir.path(),
        "ontoforge.toml",
        "[lint]\n\"ONT-E08\" = \"off\"\n",
    );
    let (out, diags) = lint_json(path(&dir), &[]);
    assert_eq!(out.status.code(), Some(0));
    assert!(diags.is_empty(), "{diags:#?}");
}

#[test]
fn config_info_level_never_fails() {
    let dir = warn_dir();
    write(
        dir.path(),
        "ontoforge.toml",
        "[lint]\n\"ONT-E09\" = \"info\"\n",
    );
    let (out, diags) = lint_json(path(&dir), &["--deny-warnings"]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(codes(&diags), ["ONT-E09"]);
    assert_eq!(diags[0]["severity"], "info");
}

#[test]
fn config_error_level_fails() {
    let dir = warn_dir();
    write(
        dir.path(),
        "ontoforge.toml",
        "[lint]\n\"ONT-E09\" = \"error\"\n",
    );
    let (out, diags) = lint_json(path(&dir), &[]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(diags[0]["severity"], "error");
}

#[test]
fn config_flag_overrides_root_file() {
    let dir = e08_dir();
    write(
        dir.path(),
        "ontoforge.toml",
        "[lint]\n\"ONT-E08\" = \"off\"\n",
    );
    let other = tempfile::tempdir().unwrap();
    write(
        other.path(),
        "lint.toml",
        "[lint]\n\"ONT-E09\" = \"warn\"\n",
    );
    let config = other.path().join("lint.toml");
    let (out, diags) = lint_json(path(&dir), &["--config", config.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(codes(&diags), ["ONT-E08"], "the root file is ignored");
}

#[test]
fn missing_config_flag_path_fails() {
    let dir = e08_dir();
    let (out, diags) = lint_json(path(&dir), &["--config", "no-such-file.toml"]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(codes(&diags), ["ONT-V15"]);
}

#[test]
fn unknown_rule_id_in_config_fails() {
    let dir = e08_dir();
    write(
        dir.path(),
        "ontoforge.toml",
        "[lint]\n\"ONT-E8\" = \"off\"\n",
    );
    let (out, diags) = lint_json(path(&dir), &[]);
    assert_eq!(out.status.code(), Some(1));
    // Lint did not run: only the configuration problem is reported.
    assert_eq!(codes(&diags), ["ONT-V15"]);
    assert_eq!(diags[0]["file"], "ontoforge.toml");
    assert_eq!(
        (diags[0]["line"].as_u64(), diags[0]["column"].as_u64()),
        (Some(2), Some(1))
    );
}

#[test]
fn validate_codes_cannot_be_turned_off_by_config() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "ontology.yaml", MANIFEST);
    write(
        dir.path(),
        "objects.yaml",
        "kind: ObjectType\napiName: Broken\n",
    );
    write(
        dir.path(),
        "ontoforge.toml",
        "[lint]\n\"ONT-V05\" = \"off\"\n",
    );
    let (out, diags) = lint_json(path(&dir), &[]);
    assert_eq!(out.status.code(), Some(1));
    assert!(codes(&diags).contains(&"ONT-V05"), "{diags:#?}");
}

#[test]
fn lint_skips_rules_when_validation_fails() {
    let (out, diags) = lint_json(V04, &[]);
    assert_eq!(out.status.code(), Some(1));
    assert!(!diags.is_empty());
    assert!(
        codes(&diags).iter().all(|c| c.starts_with("ONT-V")),
        "{diags:#?}"
    );
    let stderr =
        String::from_utf8_lossy(&ontoforge(&["lint", V04, "--color", "never"]).stderr).into_owned();
    assert!(
        stderr.contains("validation failed, lint skipped"),
        "{stderr}"
    );
}

#[test]
fn lint_json_has_no_ansi() {
    let dir = e08_dir();
    let out = ontoforge(&["lint", path(&dir), "--format", "json", "--color", "always"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        !out.stdout.contains(&0x1b),
        "stdout contains an ANSI escape"
    );
    serde_json::from_slice::<serde_json::Value>(&out.stdout).expect("stdout is JSON");
}

#[test]
fn lint_summary_counts_by_severity() {
    let dir = e08_dir();
    let out = ontoforge(&["lint", path(&dir), "--color", "never"]);
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(stderr.contains("1 error, 0 warnings, 0 infos"), "{stderr}");
    let dir = warn_dir();
    let out = ontoforge(&["lint", path(&dir), "--color", "never"]);
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(stderr.contains("0 errors, 1 warning, 0 infos"), "{stderr}");
}

#[test]
fn e08_human() {
    let dir = e08_dir();
    let out = ontoforge(&["lint", path(&dir), "--color", "never"]);
    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("ONT-E08"), "{stdout}");
    insta::assert_snapshot!("e08_human", stdout);
}
