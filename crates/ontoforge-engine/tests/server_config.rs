//! `ontoforge-server.toml`, secret references and the construction time transport rules.

use std::collections::HashMap;
use std::time::Duration;

use camino::Utf8Path;
use ontoforge_engine::config::{
    AuthKind, ConfigError, EngineKind, TrinoConfig, build_trino_engines, load_server_config,
    parse_server_config, resolve_secret,
};
use ontoforge_engine::trino::{Auth, TrinoConnection, TrinoEngine};
use ontoforge_engine::{
    DEFAULT_MAX_BYTES, DEFAULT_MAX_ROWS, DEFAULT_QUERY_TIMEOUT_MS, EngineError,
};
use secrecy::{ExposeSecret, SecretString};

const SECRET: &str = "s3cr3t-value";

const FULL: &str = r#"
[trino]
url = "https://trino.example:8443"
ca_file = "ca.pem"

[trino.identities.ontology]
user = "ontoforge-ontology"
auth = "basic"
password_env = "TRINO_PASSWORD"

[trino.identities.l2]
user = "ontoforge-l2"
auth = "bearer"
token_file = "l2.token"

[profiles.dev]
engine = "sqlite"

[profiles.prod]
engine = "trino"
"#;

fn with_trino(url: &str, identity: &str) -> String {
    format!("[trino]\nurl = \"{url}\"\n\n[trino.identities.ontology]\nuser = \"svc\"\n{identity}\n")
}

fn errors(text: &str) -> Vec<ConfigError> {
    parse_server_config(text).expect_err("the configuration must be rejected")
}

fn trino(text: &str) -> TrinoConfig {
    parse_server_config(text)
        .expect("valid configuration")
        .trino
        .expect("trino section")
}

fn no_env(_: &str) -> Option<String> {
    None
}

fn utf8(path: &std::path::Path) -> &Utf8Path {
    Utf8Path::from_path(path).expect("utf-8 path")
}

fn data_dir() -> &'static Utf8Path {
    Utf8Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data"))
}

#[test]
fn parses_full_example() {
    let cfg = parse_server_config(FULL).expect("valid");
    let trino = cfg.trino.expect("trino section");
    assert_eq!(trino.url, "https://trino.example:8443");
    assert_eq!(trino.ca_file.as_deref(), Some("ca.pem"));
    assert_eq!(trino.connect_timeout_ms, 5_000);
    assert_eq!(trino.query_timeout_ms, DEFAULT_QUERY_TIMEOUT_MS);
    assert_eq!(trino.query_timeout_ms, 30_000);
    assert_eq!(trino.max_rows, 10_000);
    assert_eq!(trino.client_tags, vec!["ontoforge".to_owned()]);
    assert_eq!(trino.identities.ontology.auth, AuthKind::Basic);
    assert_eq!(
        trino.identities.ontology.password_env.as_deref(),
        Some("TRINO_PASSWORD")
    );
    let l2 = trino.identities.l2.expect("l2 identity");
    assert_eq!(l2.auth, AuthKind::Bearer);
    assert_eq!(l2.token_file.as_deref(), Some("l2.token"));
    assert_eq!(cfg.profiles.len(), 2);
    assert_eq!(cfg.profiles["dev"].engine, EngineKind::Sqlite);
    assert_eq!(cfg.profiles["prod"].engine, EngineKind::Trino);
}

#[test]
fn plaintext_password_is_rejected() {
    let text = with_trino(
        "https://trino.example",
        "auth = \"basic\"\npassword = \"x\"",
    );
    let errs = errors(&text);
    let e = &errs[0];
    assert!(e.message.contains("plaintext"), "{e:?}");
    let help = e.help.as_deref().expect("help");
    assert!(
        help.contains("password_env") && help.contains("password_file"),
        "{help}"
    );
    let span = e.span.clone().expect("span");
    assert_eq!(&text[span], "password");

    let text = with_trino("https://trino.example", "auth = \"bearer\"\ntoken = \"x\"");
    let e = &errors(&text)[0];
    let help = e.help.as_deref().expect("help");
    assert!(
        help.contains("token_env") && help.contains("token_file"),
        "{help}"
    );
    assert_eq!(&text[e.span.clone().expect("span")], "token");
}

#[test]
fn unknown_key_is_rejected() {
    let text = format!("{}\nskip_tls = true\n", with_trino("https://t.example", ""));
    // `skip_tls` lands in the identity table, which is the last table of the text.
    let errs = errors(&text);
    let e = &errs[0];
    assert!(e.message.contains("unknown field"), "{e:?}");
    assert_eq!(&text[e.span.clone().expect("span")], "skip_tls");

    let text = "[trino]\nskip_tls = true\nurl = \"https://t.example\"\n\n[trino.identities.ontology]\nuser = \"svc\"\n";
    let e = &errors(text)[0];
    assert_eq!(&text[e.span.clone().expect("span")], "skip_tls");
}

#[test]
fn basic_over_http_is_rejected() {
    for identity in [
        "auth = \"basic\"\npassword_env = \"P\"",
        "auth = \"bearer\"\ntoken_env = \"T\"",
    ] {
        let errs = errors(&with_trino("http://trino.example:8080", identity));
        assert!(
            errs.iter().any(|e| e.message.contains("https")),
            "{identity}: {errs:?}"
        );
    }
    let ok = with_trino("http://trino.example:8080", "auth = \"none\"");
    assert!(parse_server_config(&ok).is_ok());

    let text = "[trino]
url = \"https://trino.example\"
next_uri_origins = [\"http://trino.example\"]

[trino.identities.ontology]
user = \"svc\"
";
    let errs = errors(text);
    assert!(
        errs.iter().any(|e| e.message.contains("scheme")),
        "{errs:?}"
    );
}

#[test]
fn library_rejects_credentials_over_http() {
    for auth in [
        Auth::Basic {
            password: SecretString::from(SECRET.to_owned()),
        },
        Auth::Bearer {
            token: SecretString::from(SECRET.to_owned()),
        },
    ] {
        let err = TrinoEngine::new(connection("http://127.0.0.1:1", auth, Vec::new()))
            .expect_err("credentials over http");
        assert!(matches!(err, EngineError::Config(_)), "{err:?}");
        assert!(!err.to_string().contains(SECRET));
    }
    TrinoEngine::new(connection("http://127.0.0.1:1", Auth::None, Vec::new()))
        .expect("no credentials, no problem");
}

#[test]
fn library_rejects_cross_scheme_origin() {
    let err = TrinoEngine::new(connection(
        "https://trino.example:8443",
        Auth::None,
        vec![reqwest::Url::parse("http://trino.example:8443").expect("url")],
    ))
    .expect_err("downgrading origin");
    assert!(matches!(err, EngineError::Config(_)), "{err:?}");
    TrinoEngine::new(connection(
        "https://trino.example:8443",
        Auth::None,
        vec![reqwest::Url::parse("https://gateway.example").expect("url")],
    ))
    .expect("same scheme is fine");
}

fn connection(url: &str, auth: Auth, extra_origins: Vec<reqwest::Url>) -> TrinoConnection {
    TrinoConnection {
        url: reqwest::Url::parse(url).expect("url"),
        user: "ontoforge".to_owned(),
        auth,
        ca_pem: None,
        connect_timeout: Duration::from_secs(5),
        max_page_bytes: 1024 * 1024,
        extra_origins,
    }
}

#[test]
fn profile_key_must_be_profile_name() {
    for key in ["Prod", "\"dev/x\""] {
        let text = format!("[profiles.{key}]\nengine = \"sqlite\"\n");
        let errs = errors(&text);
        let e = &errs[0];
        assert!(e.message.contains("[a-z][a-z0-9_-]{0,31}"), "{e:?}");
        let span = e.span.clone().expect("span");
        assert!(text[span].contains(key.trim_matches('"')), "{e:?}");
    }
}

#[test]
fn profile_engine_has_no_library_path() {
    let text = "[profiles.dev]\nengine = \"sqlite\"\nsqlite_dir = \"/data\"\n";
    let e = &errors(text)[0];
    assert!(e.message.contains("unknown field"), "{e:?}");
}

#[test]
fn trino_profile_needs_trino_section() {
    let errs = errors("[profiles.prod]\nengine = \"trino\"\n");
    assert!(
        errs.iter().any(|e| e.message.contains("[trino]")),
        "{errs:?}"
    );
}

#[test]
fn query_context_from_config() {
    let cfg = trino(&with_trino("https://t.example", ""));
    let ctx = cfg.query_context("trace-1".to_owned(), vec!["a".to_owned()]);
    assert_eq!(ctx.timeout, Duration::from_millis(DEFAULT_QUERY_TIMEOUT_MS));
    assert_eq!(ctx.max_rows, DEFAULT_MAX_ROWS);
    assert_eq!(ctx.max_bytes, DEFAULT_MAX_BYTES);
    assert_eq!(ctx.trace_token, "trace-1");
    assert_eq!(ctx.client_tags, vec!["a".to_owned()]);

    let text = format!(
        "{}query_timeout_ms = 5000\n",
        "[trino]\nurl = \"https://t.example\"\n"
    ) + "\n[trino.identities.ontology]\nuser = \"svc\"\n";
    let ctx = trino(&text).query_context("t".to_owned(), Vec::new());
    assert_eq!(ctx.timeout, Duration::from_secs(5));
}

#[test]
fn basic_requires_exactly_one_secret_ref() {
    for identity in [
        "auth = \"basic\"",
        "auth = \"basic\"\npassword_env = \"P\"\npassword_file = \"p.txt\"",
        "auth = \"bearer\"",
        "auth = \"none\"\npassword_env = \"P\"",
    ] {
        let errs = errors(&with_trino("https://t.example", identity));
        assert!(!errs.is_empty(), "{identity}");
    }
    assert!(
        parse_server_config(&with_trino(
            "https://t.example",
            "auth = \"basic\"\npassword_file = \"p.txt\""
        ))
        .is_ok()
    );
}

#[test]
fn secret_from_env() {
    let env = |name: &str| (name == "TRINO_PASSWORD").then(|| SECRET.to_owned());
    let secret =
        resolve_secret(Some("TRINO_PASSWORD"), None, Utf8Path::new("."), &env).expect("set");
    assert_eq!(secret.expose_secret(), SECRET);

    let err = resolve_secret(Some("MISSING_VAR"), None, Utf8Path::new("."), &no_env)
        .expect_err("missing variable");
    assert!(err.message.contains("MISSING_VAR"), "{err:?}");
    assert!(!err.message.contains(SECRET));
}

#[test]
fn secret_from_file_follows_symlink_and_trims() {
    let dir = tempfile::tempdir().expect("tempdir");
    let base = utf8(dir.path());
    std::fs::write(base.join("pw.txt"), format!("{SECRET}\n")).expect("write");
    std::fs::write(base.join("crlf.txt"), format!("{SECRET}\r\n")).expect("write");

    let secret = resolve_secret(None, Some("pw.txt"), base, &no_env).expect("file");
    assert_eq!(secret.expose_secret(), SECRET);
    let secret = resolve_secret(None, Some("crlf.txt"), base, &no_env).expect("file");
    assert_eq!(secret.expose_secret(), SECRET);

    // A link is how Kubernetes mounts secrets. Creating one may need privileges on Windows.
    #[cfg(unix)]
    let linked = std::os::unix::fs::symlink(base.join("pw.txt"), base.join("link.txt")).is_ok();
    #[cfg(windows)]
    let linked =
        std::os::windows::fs::symlink_file(base.join("pw.txt"), base.join("link.txt")).is_ok();
    if linked {
        let secret = resolve_secret(None, Some("link.txt"), base, &no_env).expect("link");
        assert_eq!(secret.expose_secret(), SECRET);
    }

    let mut big = SECRET.as_bytes().to_vec();
    big.resize(64 * 1024 + 1, b'x');
    std::fs::write(base.join("big.txt"), big).expect("write");
    let err = resolve_secret(None, Some("big.txt"), base, &no_env).expect_err("too large");
    assert!(err.message.contains("big.txt"), "{err:?}");
    assert!(!err.message.contains(SECRET));

    // WR-E01: a reference that names no file is not echoed (it may be the secret itself).
    let err = resolve_secret(None, Some("absent.txt"), base, &no_env).expect_err("absent");
    assert!(err.message.contains("does not exist"), "{err:?}");
    assert!(!err.message.contains("absent.txt"), "{err:?}");
}

#[test]
fn secret_written_into_a_reference_key_is_never_echoed() {
    // WR-E01: the secret value pasted into `*_env` / `*_file` instead of a name or a path.
    const PASTED: &str = "Pa55w0rd!xyz";
    const JWT: &str = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ4In0.c2lnbmF0dXJl";
    let mut seen: Vec<String> = Vec::new();
    for identity in [
        format!("auth = \"basic\"\npassword_env = \"{PASTED}\""),
        format!("auth = \"bearer\"\ntoken_env = \"{PASTED}\""),
    ] {
        let errs = errors(&with_trino("https://t.example", &identity));
        assert!(
            errs.iter()
                .any(|e| e.message.contains("must be an environment variable name")),
            "{errs:?}"
        );
        seen.extend(errs.iter().map(|e| format!("{e} {e:?}")));
    }
    let e = resolve_secret(Some(PASTED), None, Utf8Path::new("."), &no_env).expect_err("env");
    assert!(e.message.contains("not a valid variable name"), "{e:?}");
    seen.push(format!("{e} {e:?}"));
    let dir = tempfile::tempdir().expect("tempdir");
    let e = resolve_secret(None, Some(JWT), utf8(dir.path()), &no_env).expect_err("file");
    seen.push(format!("{e} {e:?}"));
    for text in &seen {
        assert!(!text.contains(PASTED) && !text.contains(JWT), "{text}");
    }
    // A well-formed variable name is still named, so the operator knows what to set.
    let e =
        resolve_secret(Some("TRINO_PW_2"), None, Utf8Path::new("."), &no_env).expect_err("unset");
    assert!(e.message.contains("TRINO_PW_2"), "{e:?}");
}

#[test]
fn secret_never_in_debug_or_errors() {
    let dir = tempfile::tempdir().expect("tempdir");
    let base = utf8(dir.path());
    std::fs::write(base.join("l2.token"), format!("{SECRET}-l2\n")).expect("write");
    let env = |name: &str| (name == "TRINO_PASSWORD").then(|| SECRET.to_owned());
    let cfg = trino(&FULL.replace("ca_file = \"ca.pem\"\n", ""));
    let engines = build_trino_engines(&cfg, base, &env).expect("engines");
    assert!(engines.l2.is_some());
    let debug = format!("{engines:?}");
    assert!(!debug.contains(SECRET), "{debug}");

    let mut seen: Vec<String> = Vec::new();
    // Rejected plaintext value.
    let plaintext = with_trino(
        "https://t.example",
        &format!("auth = \"basic\"\npassword = \"{SECRET}\""),
    );
    for e in errors(&plaintext) {
        seen.push(format!("{e} {e:?}"));
    }
    // Secret file that cannot be used.
    std::fs::write(base.join("empty.txt"), "\n").expect("write");
    let mut big = SECRET.as_bytes().to_vec();
    big.resize(64 * 1024 + 1, b'x');
    std::fs::write(base.join("big.txt"), big).expect("write");
    for file in ["empty.txt", "big.txt", "absent.txt"] {
        let e = resolve_secret(None, Some(file), base, &no_env).expect_err(file);
        seen.push(format!("{e} {e:?}"));
    }
    // Credentials over http at the engine level.
    let e = TrinoEngine::new(connection(
        "http://127.0.0.1:1",
        Auth::Basic {
            password: SecretString::from(SECRET.to_owned()),
        },
        Vec::new(),
    ))
    .expect_err("http");
    seen.push(format!("{e} {e:?}"));
    for text in &seen {
        assert!(!text.contains(SECRET), "{text}");
    }
    assert!(seen.iter().all(|t| !t.is_empty()));
}

#[test]
fn ca_file_is_loaded() {
    let text = FULL
        .replace(
            "auth = \"basic\"\npassword_env = \"TRINO_PASSWORD\"",
            "auth = \"none\"",
        )
        .replace(
            "auth = \"bearer\"\ntoken_file = \"l2.token\"",
            "auth = \"none\"",
        )
        .replace("ca.pem", "test-ca.pem");
    let cfg = trino(&text);
    build_trino_engines(&cfg, data_dir(), &no_env).expect("valid CA");

    let dir = tempfile::tempdir().expect("tempdir");
    let base = utf8(dir.path());
    std::fs::write(base.join("test-ca.pem"), "this is not a certificate").expect("write");
    let errs = build_trino_engines(&cfg, base, &no_env).expect_err("not PEM");
    assert!(
        errs.iter().any(|e| e.message.contains("test-ca.pem")),
        "{errs:?}"
    );
    let errs = build_trino_engines(&cfg, &base.join("missing"), &no_env).expect_err("absent");
    assert!(!errs.is_empty());
}

#[test]
fn load_server_config_reads_the_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let base = utf8(dir.path());
    std::fs::write(base.join("ontoforge-server.toml"), FULL).expect("write");
    let (cfg, text) = load_server_config(&base.join("ontoforge-server.toml")).expect("loads");
    assert_eq!(text, FULL);
    assert_eq!(cfg.profiles.len(), 2);

    let (errs, text) = load_server_config(&base.join("missing.toml")).expect_err("absent");
    assert!(errs[0].message.contains("missing.toml"), "{errs:?}");
    assert!(text.is_none());

    // CR-C01: a parse failure hands back the text that was parsed, so the caller never reads
    // the file a second time.
    let bad = "[profiles.Prod]\nengine = \"sqlite\"\n";
    std::fs::write(base.join("bad.toml"), bad).expect("write");
    let (errs, text) = load_server_config(&base.join("bad.toml")).expect_err("invalid");
    assert!(!errs.is_empty());
    assert_eq!(text.as_deref(), Some(bad));
}

#[test]
fn engine_debug_names_the_identity_user() {
    // The Debug output of an engine names its user, which tells the two identities apart.
    let env = |name: &str| match name {
        "TRINO_PASSWORD" => Some("pw-one".to_owned()),
        _ => None,
    };
    let dir = tempfile::tempdir().expect("tempdir");
    let base = utf8(dir.path());
    std::fs::write(base.join("l2.token"), "tok").expect("write");
    let cfg = trino(&FULL.replace("ca_file = \"ca.pem\"\n", ""));
    let engines = build_trino_engines(&cfg, base, &env).expect("engines");
    let by_user: HashMap<_, _> = [
        ("ontology", format!("{:?}", engines.ontology)),
        ("l2", format!("{:?}", engines.l2.expect("l2"))),
    ]
    .into_iter()
    .collect();
    assert!(by_user["ontology"].contains("ontoforge-ontology"));
    assert!(by_user["l2"].contains("ontoforge-l2"));
}
