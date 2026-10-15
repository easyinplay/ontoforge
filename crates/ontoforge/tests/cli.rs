use std::process::{Command, Output};

const AVIATION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/ontologies/aviation"
);
const V04: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/ontologies/invalid/v04-unknown-field"
);

fn ontoforge(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ontoforge"))
        .args(args)
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn ontoforge")
}

fn json_diagnostics(dir: &str) -> (Output, Vec<serde_json::Value>) {
    let out = ontoforge(&["validate", dir, "--format", "json"]);
    let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).expect("stdout is JSON");
    (out, parsed.as_array().expect("JSON array").clone())
}

#[test]
fn validate_aviation_succeeds_silently() {
    let out = ontoforge(&["validate", AVIATION, "--color", "never"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.stdout.is_empty(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn validate_unknown_field_points_at_the_misspelled_key() {
    let (out, diags) = json_diagnostics(V04);
    assert_eq!(out.status.code(), Some(1));
    let first = &diags[0];
    assert_eq!(first["code"], "ONT-V04");
    assert_eq!(first["file"], "objects.yaml");
    assert_eq!(first["line"], 3);
    assert_eq!(first["column"], 1);
    assert!(first["help"].as_str().unwrap().contains("primaryKey"));
}

#[test]
fn json_output_has_exactly_the_documented_fields_and_no_ansi() {
    let (out, diags) = json_diagnostics(V04);
    assert!(
        !out.stdout.contains(&0x1b),
        "stdout contains an ANSI escape"
    );
    for d in &diags {
        let mut keys: Vec<&str> = d.as_object().unwrap().keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "code", "column", "file", "help", "line", "message", "severity", "span"
            ]
        );
        assert_eq!(d["severity"], "error");
        assert!(d["span"]["offset"].is_u64() && d["span"]["length"].is_u64());
    }
}

#[test]
fn validate_human_output_for_unknown_field() {
    let out = ontoforge(&["validate", V04, "--color", "never"]);
    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).unwrap();
    insta::assert_snapshot!("validate_unknown_field_human", stdout);
}

#[test]
fn missing_manifest_is_reported_as_v14() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_str().unwrap();
    let (out, diags) = json_diagnostics(path);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(diags[0]["code"], "ONT-V14");
    assert_eq!(diags[0]["file"], "ontology.yaml");
}
