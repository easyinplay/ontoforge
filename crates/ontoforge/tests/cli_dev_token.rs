//! End-to-end tests of `ontoforge dev-token` (AUTH-02, D-02, D-04, D-05). Keys are generated
//! into a temporary directory; no fixed secret is involved.

use std::path::Path;
use std::process::{Command, Output};

use camino::Utf8Path;
use ontoforge_auth::Authenticator;
use ontoforge_auth::authn::dev::{DevKey, DevMode, DevVerifier};
use ontoforge_engine::config::parse_server_config;
use ontoforge_expr::rowpred::{AttrValue, Scalar};

const CONFIG: &str = r#"
[auth.principal_attrs.homeAirports]
claim = "airports"
type = "string_list"

[auth.principal_attrs.level]
claim = "lvl"
type = "integer"

[auth.principal_attrs.admin]
claim = "adm"
type = "boolean"

[auth.dev]
key_file = "dev.key"
"#;

fn ontoforge(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ontoforge"))
        .args(args)
        .env_remove("ONTOFORGE_PROFILE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn ontoforge")
}

fn server_dir(text: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("ontoforge-server.toml"), text).expect("write config");
    dir
}

fn config_of(dir: &tempfile::TempDir) -> String {
    dir.path()
        .join("ontoforge-server.toml")
        .to_str()
        .expect("utf-8 path")
        .to_owned()
}

fn dev_token(config: &str, extra: &[&str]) -> Output {
    let mut args = vec!["dev-token", "--server-config", config];
    args.extend_from_slice(extra);
    ontoforge(&args)
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// An authenticator that accepts dev tokens of the key at `key_file`, for the `[auth]` of `text`.
fn dev_authenticator(text: &str, key_file: &Path) -> Authenticator {
    let auth = parse_server_config(text)
        .expect("config")
        .auth
        .expect("auth section");
    let key = DevKey::load_or_create(Utf8Path::from_path(key_file).expect("utf-8 path"))
        .expect("existing key");
    let verifier = DevVerifier::new(DevMode::enable_insecure(), key, &auth.principal_attrs);
    Authenticator::from_config(&auth, Utf8Path::new("."))
        .expect("authenticator")
        .with_dev(verifier)
}

fn one_line(out: &Output) -> String {
    let text = stdout(out);
    assert_eq!(
        text.trim_end_matches('\n').lines().count(),
        1,
        "stdout must be exactly one line: {text:?}"
    );
    text.trim_end().to_owned()
}

#[tokio::test]
async fn issues_token_verifiable_by_dev_verifier() {
    let dir = server_dir(CONFIG);
    let out = dev_token(
        &config_of(&dir),
        &[
            "--sub",
            "alice",
            "--groups",
            "dispatchers",
            "--attr",
            "homeAirports=FRA,MUC",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let token = one_line(&out);
    // The key file named by `[auth.dev]` is relative to the configuration file.
    assert!(dir.path().join("dev.key").is_file());
    assert!(
        stderr(&out).contains("dev token for ontoforge-dev|alice, expires "),
        "{}",
        stderr(&out)
    );
    assert!(
        !stderr(&out).contains(&token),
        "the token must not be echoed on stderr"
    );

    let principal = dev_authenticator(CONFIG, &dir.path().join("dev.key"))
        .authenticate(&token)
        .await
        .expect("authenticated");

    assert_eq!(principal.id.as_str(), "ontoforge-dev|alice");
    assert_eq!(
        principal
            .groups
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["dispatchers"]
    );
    assert_eq!(
        principal.attrs.get("homeAirports"),
        &AttrValue::List(vec![
            Scalar::String("FRA".to_owned()),
            Scalar::String("MUC".to_owned()),
        ])
    );
}

#[tokio::test]
async fn scalar_attrs_are_converted_by_declared_type() {
    let dir = server_dir(CONFIG);
    let out = dev_token(
        &config_of(&dir),
        &[
            "--sub",
            "bob",
            "--attr",
            "level=3",
            "--attr",
            "admin=true",
            "--attr",
            "level=4",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));

    let principal = dev_authenticator(CONFIG, &dir.path().join("dev.key"))
        .authenticate(&one_line(&out))
        .await
        .expect("authenticated");

    // The later `--attr level=...` wins.
    assert_eq!(
        principal.attrs.get("level"),
        &AttrValue::Scalar(Scalar::Int(4))
    );
    assert_eq!(
        principal.attrs.get("admin"),
        &AttrValue::Scalar(Scalar::Bool(true))
    );
    assert!(principal.groups.is_empty());
}

#[test]
fn undeclared_attr_is_rejected() {
    let dir = server_dir(CONFIG);
    let out = dev_token(
        &config_of(&dir),
        &["--sub", "alice", "--attr", "tenantId=acme"],
    );
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr(&out));
    assert!(stderr(&out).contains("tenantId"), "{}", stderr(&out));
    assert!(stdout(&out).is_empty(), "no token on failure");
}
