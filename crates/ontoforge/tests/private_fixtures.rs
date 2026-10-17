//! Validation against private environments (ENGINE-05): SQL Server behind Trino, company
//! clusters and other data that must not enter the public repository.
//!
//! `ONTOFORGE_PRIVATE_FIXTURES` names a directory outside the repository. Each subdirectory
//! `<name>/` holding both `ontology/` and `ontoforge-server.toml` is one fixture set, checked
//! with `ontoforge validate --online --server-config <name>/ontoforge-server.toml
//! <name>/ontology`. The test is `#[ignore]`d and neither CI workflow runs this target; see
//! CONTRIBUTING.md.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

const FIXTURES_VAR: &str = "ONTOFORGE_PRIVATE_FIXTURES";

/// The private fixture directory taken from `value`; panics when it is absent.
fn fixtures_root(value: Option<OsString>) -> PathBuf {
    match value {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => panic!(
            "{FIXTURES_VAR} is not set: point it at a directory outside the repository \
             (see CONTRIBUTING.md)"
        ),
    }
}

/// The fixture sets under `root`: subdirectories with both `ontology/` and
/// `ontoforge-server.toml`, sorted by name.
fn fixture_sets(root: &Path) -> Vec<PathBuf> {
    let entries =
        std::fs::read_dir(root).unwrap_or_else(|e| panic!("cannot read {}: {e}", root.display()));
    let mut sets: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|dir| dir.join("ontology").is_dir() && dir.join("ontoforge-server.toml").is_file())
        .collect();
    sets.sort();
    sets
}

#[test]
fn missing_env_panics() {
    let outcome = std::panic::catch_unwind(|| fixtures_root(None));
    let payload = outcome.expect_err("a missing ONTOFORGE_PRIVATE_FIXTURES must panic");
    let message = payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_owned()))
        .unwrap_or_default();
    assert!(message.contains(FIXTURES_VAR), "{message}");
}

#[test]
fn a_set_needs_ontology_and_server_config() {
    let root = tempfile::tempdir().expect("tempdir");
    let complete = root.path().join("complete");
    std::fs::create_dir_all(complete.join("ontology")).expect("create");
    std::fs::write(complete.join("ontoforge-server.toml"), "").expect("write");
    let no_config = root.path().join("no-config");
    std::fs::create_dir_all(no_config.join("ontology")).expect("create");
    let no_ontology = root.path().join("no-ontology");
    std::fs::create_dir_all(&no_ontology).expect("create");
    std::fs::write(no_ontology.join("ontoforge-server.toml"), "").expect("write");

    assert_eq!(fixture_sets(root.path()), vec![complete]);
}

#[test]
#[ignore = "needs ONTOFORGE_PRIVATE_FIXTURES"]
fn private_fixtures_validate_online() {
    let root = fixtures_root(std::env::var_os(FIXTURES_VAR));
    let sets = fixture_sets(&root);
    assert!(
        !sets.is_empty(),
        "no <name>/ontology with <name>/ontoforge-server.toml under {}",
        root.display()
    );
    let mut failures = Vec::new();
    for set in &sets {
        let out = Command::new(env!("CARGO_BIN_EXE_ontoforge"))
            .arg("validate")
            .arg("--online")
            .arg("--server-config")
            .arg(set.join("ontoforge-server.toml"))
            .arg(set.join("ontology"))
            .args(["--color", "never"])
            .env_remove("ONTOFORGE_PROFILE")
            .env("NO_COLOR", "1")
            .output()
            .expect("spawn ontoforge");
        if out.status.code() != Some(0) {
            failures.push(format!(
                "{}: exit {:?}\nstdout: {}\nstderr: {}",
                set.display(),
                out.status.code(),
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
