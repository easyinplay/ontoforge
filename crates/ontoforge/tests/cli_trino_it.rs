//! `ontoforge validate --online --check-pk` end to end against the CI Trino (ENGINE-05).
//!
//! The test is `#[ignore]`d; the Linux CI job `integration` runs it with `--ignored` after
//! seeding the aviation example, using `ci/ontoforge-server.ci.toml`. Without
//! `ONTOFORGE_TRINO_URL` it panics instead of passing silently (RESEARCH Pitfall 8).

use std::process::Command;

use serde_json::Value;

const URL_VAR: &str = "ONTOFORGE_TRINO_URL";

const REPO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

/// The Trino URL taken from `value`; panics when it is absent.
fn trino_url(value: Option<String>) -> String {
    value.unwrap_or_default()
}

#[test]
fn missing_env_panics() {
    let outcome = std::panic::catch_unwind(|| trino_url(None));
    let payload = outcome.expect_err("a missing ONTOFORGE_TRINO_URL must panic");
    let message = payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_owned()))
        .unwrap_or_default();
    assert!(message.contains(URL_VAR), "{message}");
}

/// Every binding of the aviation example names a fixture that the Trino check does not use,
/// so the only diagnostics are those ONT-V17 warnings; the exit code is 0.
#[test]
#[ignore = "needs ONTOFORGE_TRINO_URL (Linux CI)"]
fn validate_online_against_ci_trino() {
    let url = trino_url(std::env::var(URL_VAR).ok());
    let config = format!("{REPO}/ci/ontoforge-server.ci.toml");
    let config_text = std::fs::read_to_string(&config).expect("read the CI server config");
    assert!(
        config_text.contains(&format!("url = \"{url}\"")),
        "{URL_VAR}={url} is not the URL in {config}"
    );

    let out = Command::new(env!("CARGO_BIN_EXE_ontoforge"))
        .args([
            "validate",
            "--online",
            "--check-pk",
            "--server-config",
            &config,
            &format!("{REPO}/examples/ontologies/aviation"),
            "--format",
            "json",
            "--color",
            "never",
        ])
        .env_remove("ONTOFORGE_PROFILE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn ontoforge");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stdout: {stdout}\nstderr: {stderr}"
    );

    let parsed: Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {stdout}"));
    let diagnostics = parsed.as_array().expect("JSON array");
    assert_eq!(diagnostics.len(), 5, "one per binding: {diagnostics:#?}");
    for d in diagnostics {
        assert_eq!(d["code"], "ONT-V17", "{d:#?}");
        assert_eq!(d["severity"], "warning", "{d:#?}");
        let message = d["message"].as_str().unwrap_or_default();
        assert!(message.contains("fixture"), "{d:#?}");
        assert!(message.contains("is ignored"), "{d:#?}");
    }
}
