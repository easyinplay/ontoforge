//! End-to-end tests of `ontoforge reload`: edit tables from a definition, dry run, exit codes
//! and the rule that a broken definition never opens the store (STORE-02, D-15).

use std::path::Path;
use std::process::{Command, Output};

const AVIATION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/ontologies/aviation"
);

const SQLITE_CONFIG: &str = "[store]\nkind = \"sqlite\"\npath = \"edit.db\"\n";

fn ontoforge(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ontoforge"))
        .args(args)
        .env_remove("ONTOFORGE_PROFILE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn ontoforge")
}

fn reload(dir: &str, config: &Path, extra: &[&str]) -> Output {
    let config = config.to_str().expect("utf-8 path");
    let mut args = vec!["reload", dir, "--server-config", config];
    args.extend_from_slice(extra);
    ontoforge(&args)
}

/// A temporary directory holding `ontoforge-server.toml` with `text`.
fn server_dir(text: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("ontoforge-server.toml"), text).expect("write config");
    dir
}

fn config_of(dir: &tempfile::TempDir) -> std::path::PathBuf {
    dir.path().join("ontoforge-server.toml")
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn reload_creates_tables() {
    let dir = server_dir(SQLITE_CONFIG);
    let first = reload(AVIATION, &config_of(&dir), &[]);
    assert_eq!(first.status.code(), Some(0), "stderr: {}", stderr(&first));
    assert!(
        stderr(&first).contains("def_version 1"),
        "{}",
        stderr(&first)
    );
    assert!(dir.path().join("edit.db").is_file());

    let second = reload(AVIATION, &config_of(&dir), &[]);
    assert_eq!(second.status.code(), Some(0), "stderr: {}", stderr(&second));
    assert!(
        stderr(&second).contains("definitions unchanged"),
        "{}",
        stderr(&second)
    );
}

#[test]
fn dry_run_prints_ddl_only() {
    let dir = server_dir(SQLITE_CONFIG);
    let dry = reload(AVIATION, &config_of(&dir), &["--dry-run"]);
    assert_eq!(dry.status.code(), Some(0), "stderr: {}", stderr(&dry));
    assert!(stdout(&dry).contains("CREATE TABLE"), "{}", stdout(&dry));
    assert!(stderr(&dry).contains("nothing applied"), "{}", stderr(&dry));

    let real = reload(AVIATION, &config_of(&dir), &[]);
    assert_eq!(real.status.code(), Some(0), "stderr: {}", stderr(&real));
    assert!(stderr(&real).contains("def_version 1"), "{}", stderr(&real));
}

#[test]
fn missing_store_section() {
    let dir = server_dir("");
    let out = reload(AVIATION, &config_of(&dir), &[]);
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr(&out));
    assert!(stdout(&out).contains("ONT-V20"), "{}", stdout(&out));
}

#[test]
fn store_unavailable() {
    let dir = server_dir("[store]\nkind = \"sqlite\"\npath = \"no-such-dir/sub/edit.db\"\n");
    let out = reload(AVIATION, &config_of(&dir), &[]);
    assert_eq!(out.status.code(), Some(3), "stderr: {}", stderr(&out));
    assert!(stdout(&out).contains("ONT-V26"), "{}", stdout(&out));
}

#[test]
fn invalid_definitions_never_open_store() {
    let dir = server_dir(SQLITE_CONFIG);
    let defs = tempfile::tempdir().expect("tempdir");
    std::fs::write(defs.path().join("ontology.yaml"), "apiName: demo\n").expect("manifest");
    std::fs::write(
        defs.path().join("defs.yaml"),
        "kind: ObjectType\napiName: Gate\nbacking: editOnly\nprimaryKey: code\nproperties:\n  code: { type: string, required: true }\n---\nkind: LinkType\napiName: gateStands\ncardinality: manyToMany\nfrom: { objectType: Gate, apiName: stands }\nto: { objectType: Missing, apiName: gates }\nbacking: editOnly\n",
    )
    .expect("defs");
    let out = reload(
        defs.path().to_str().expect("utf-8 path"),
        &config_of(&dir),
        &[],
    );
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr(&out));
    assert!(
        !dir.path().join("edit.db").exists(),
        "the store file must not be created for an invalid definition"
    );
}

#[test]
fn json_output() {
    let dir = server_dir(SQLITE_CONFIG);
    let out = reload(AVIATION, &config_of(&dir), &["--format", "json"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).expect("stdout is JSON");
    assert!(
        value["diagnostics"]
            .as_array()
            .expect("diagnostics")
            .is_empty()
    );
    let result = &value["result"];
    assert_eq!(result["defVersion"], 1);
    assert_eq!(result["changed"], true);
    assert_eq!(result["dryRun"], false);
    assert!(
        result["ddl"]
            .as_array()
            .expect("ddl")
            .iter()
            .any(|s| s.as_str().is_some_and(|s| s.contains("CREATE TABLE"))),
        "{result}"
    );
}
