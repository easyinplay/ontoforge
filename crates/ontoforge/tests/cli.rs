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
    ontoforge_env(args, &[])
}

/// Run with extra environment variables; `ONTOFORGE_PROFILE` is removed first so a
/// variable of the developer's shell cannot change a result.
fn ontoforge_env(args: &[&str], env: &[(&str, &str)]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ontoforge"))
        .args(args)
        .env_remove("ONTOFORGE_PROFILE")
        .env("NO_COLOR", "1")
        .envs(env.iter().copied())
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

const PROFILE_OBJECTS: &str = "\
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
  employeeId: { type: string, required: true }
backing: source
";

fn binding(object: &str, table: &str) -> String {
    format!(
        "kind: SourceBinding\ntarget: {{ objectType: {object} }}\nsource: {{ catalog: demo, schema: aviation, table: {table} }}\n"
    )
}

/// `dev` binds both source types, `prod` only Flight; `defaultProfile` is `dev`.
fn profile_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let write = |name: &str, text: &str| {
        let path = dir.path().join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    };
    write("ontology.yaml", "apiName: demo\ndefaultProfile: dev\n");
    write("objects.yaml", PROFILE_OBJECTS);
    write("bindings/dev/flight.yaml", &binding("Flight", "flights"));
    write("bindings/dev/pilot.yaml", &binding("Pilot", "pilots"));
    write("bindings/prod/flight.yaml", &binding("Flight", "flights"));
    dir
}

fn codes_of(out: &Output) -> Vec<String> {
    let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).expect("stdout is JSON");
    parsed
        .as_array()
        .expect("JSON array")
        .iter()
        .map(|d| d["code"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn profile_precedence() {
    let dir = profile_dir();
    let path = dir.path().to_str().unwrap();
    let run = |extra: &[&str], env: &[(&str, &str)]| {
        let mut args = vec!["validate", path, "--format", "json"];
        args.extend_from_slice(extra);
        ontoforge_env(&args, env)
    };

    // The manifest default applies when nothing else is given.
    let out = run(&[], &[]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );

    // The flag beats the default.
    let out = run(&["--profile", "prod"], &[]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(codes_of(&out), ["ONT-V16"]);

    // The environment beats the default.
    let out = run(&[], &[("ONTOFORGE_PROFILE", "prod")]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(codes_of(&out), ["ONT-V16"]);

    // The flag beats the environment.
    let out = run(&["--profile", "dev"], &[("ONTOFORGE_PROFILE", "prod")]);
    assert_eq!(out.status.code(), Some(0));

    // A name that is no profile name at all.
    let out = run(&["--profile", "Bad Name"], &[]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(codes_of(&out), ["ONT-V16"]);

    // An unknown but well-formed name.
    let out = run(&["--profile", "stage"], &[]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(codes_of(&out), ["ONT-V16"]);
}

#[test]
fn validate_summary_names_the_profile() {
    let dir = profile_dir();
    let path = dir.path().to_str().unwrap();
    let out = ontoforge(&["validate", path, "--color", "never"]);
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(stderr.contains("profile `dev`"), "{stderr}");
    let out = ontoforge(&["validate", path, "--color", "never", "--profile", "dev"]);
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(stderr.contains("profile `dev`"), "{stderr}");
}

#[test]
fn aviation_is_clean_under_dev() {
    for name in ["aviation", "aviation-json"] {
        let dir = format!(
            "{}/../../examples/ontologies/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        for command in ["validate", "lint"] {
            let out = ontoforge(&[command, &dir, "--format", "json"]);
            assert_eq!(out.status.code(), Some(0), "{command} {name}");
            assert!(codes_of(&out).is_empty(), "{command} {name}");
        }
    }
}
