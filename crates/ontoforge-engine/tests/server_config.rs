//! `ontoforge-server.toml`, secret references and the construction time transport rules.

use std::collections::HashMap;
use std::time::Duration;

use camino::Utf8Path;
use ontoforge_engine::config::{
    AuthConfig, AuthKind, ConfigError, EngineKind, StoreConfig, StoreKind, TrinoConfig,
    build_trino_engines, load_server_config, parse_server_config, resolve_secret,
};
use ontoforge_schema::ir::AttrType;
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
    // WR-E05: one page is of the order of the whole result limit, not four times it.
    assert_eq!(trino.max_page_bytes, 16 * 1024 * 1024);
    assert_eq!(trino.max_page_bytes, DEFAULT_MAX_BYTES);
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
    // WR-E02: the configured tags (default `ontoforge`) come first, then the caller's.
    assert_eq!(ctx.client_tags, ["ontoforge", "a"]);

    let cfg = trino(&with_trino("https://t.example", "").replace(
        "[trino.identities",
        "client_tags = [\"team-a\", \"x\"]\n\n[trino.identities",
    ));
    let ctx = cfg.query_context("t".to_owned(), vec!["x".to_owned(), "b".to_owned()]);
    assert_eq!(ctx.client_tags, ["team-a", "x", "b"]);

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

// ---------------------------------------------------------------------------------------------
// `[store]` and `[auth]` (Phase 3, plan 03-02)
// ---------------------------------------------------------------------------------------------

/// The text a configuration error points at.
fn span_text<'a>(text: &'a str, e: &ConfigError) -> &'a str {
    &text[e.span.clone().expect("span")]
}

fn store(text: &str) -> StoreConfig {
    parse_server_config(text)
        .expect("valid configuration")
        .store
        .expect("store section")
}

fn auth(text: &str) -> AuthConfig {
    parse_server_config(text)
        .expect("valid configuration")
        .auth
        .expect("auth section")
}

#[test]
fn store_sqlite_defaults() {
    let s = store("[store]\nkind = \"sqlite\"\npath = \"edit.db\"\n");
    assert_eq!(s.kind, StoreKind::Sqlite);
    assert_eq!(s.path.as_deref(), Some("edit.db"));
    assert_eq!(s.schema_edit, "ontoforge_edit");
    assert_eq!(s.schema_sys, "ontoforge_sys");
    assert_eq!(s.lock_timeout_ms, 5_000);
    assert_eq!(s.probe_timeout_ms, 60_000);
    assert!(s.trino.is_none());
}

#[test]
fn store_postgres_with_trino_catalog() {
    let s = store(
        "[store]\nkind = \"postgres\"\nurl_env = \"ONTOFORGE_PG_URL\"\n\n[store.trino]\ncatalog = \"edit_pg\"\n",
    );
    assert_eq!(s.kind, StoreKind::Postgres);
    assert_eq!(s.url_env.as_deref(), Some("ONTOFORGE_PG_URL"));
    assert_eq!(s.trino.expect("trino").catalog, "edit_pg");
    let s = store("[store]\nkind = \"postgres\"\nurl_file = \"pg.url\"\n");
    assert_eq!(s.url_file.as_deref(), Some("pg.url"));
}

#[test]
fn store_postgres_requires_exactly_one_url_source() {
    for body in [
        "kind = \"postgres\"\n",
        "kind = \"postgres\"\nurl_env = \"A\"\nurl_file = \"b\"\n",
    ] {
        let text = format!("[store]\n{body}");
        let errs = errors(&text);
        assert_eq!(errs.len(), 1, "{body}: {errs:?}");
        assert!(
            errs[0].message.contains("url_env") && errs[0].message.contains("url_file"),
            "{errs:?}"
        );
        assert!(errs[0].span.is_some(), "{errs:?}");
    }
}

#[test]
fn store_sqlite_rejects_url_and_postgres_rejects_path() {
    let text = "[store]\nkind = \"sqlite\"\npath = \"edit.db\"\nurl_env = \"A\"\n";
    let errs = errors(text);
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert!(errs[0].message.contains("sqlite"), "{errs:?}");
    let text = "[store]\nkind = \"postgres\"\nurl_env = \"A\"\npath = \"edit.db\"\n";
    let errs = errors(text);
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert!(errs[0].message.contains("postgres"), "{errs:?}");
    let errs = errors("[store]\nkind = \"sqlite\"\n");
    assert!(errs[0].message.contains("path"), "{errs:?}");
}

#[test]
fn store_plaintext_url_is_rejected() {
    let text = "[store]\nkind = \"postgres\"\nurl = \"postgres://u:p@h/db\"\n";
    let errs = errors(text);
    let e = &errs[0];
    assert!(e.message.contains("plaintext"), "{e:?}");
    let help = e.help.as_deref().expect("help");
    assert!(help.contains("url_env") && help.contains("url_file"), "{help}");
    assert_eq!(span_text(text, e), "url");
    assert!(!format!("{errs:?}").contains("u:p"));
}

#[test]
fn store_schema_names_are_validated() {
    let text = "[store]\nkind = \"sqlite\"\npath = \"e.db\"\nschema_edit = \"Edit\"\n";
    let errs = errors(text);
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert_eq!(span_text(text, &errs[0]), "\"Edit\"");

    let text = "[store]\nkind = \"sqlite\"\npath = \"e.db\"\nschema_edit = \"same\"\nschema_sys = \"same\"\n";
    let errs = errors(text);
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert!(errs[0].message.contains("differ"), "{errs:?}");

    let text = "[store]\nkind = \"sqlite\"\npath = \"e.db\"\nlock_timeout_ms = 0\n";
    let errs = errors(text);
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert_eq!(span_text(text, &errs[0]), "0");

    let text = "[store]\nkind = \"sqlite\"\npath = \"e.db\"\nprobe_timeout_ms = 3600001\n";
    assert_eq!(errors(text).len(), 1);

    let text =
        "[store]\nkind = \"postgres\"\nurl_env = \"A\"\n\n[store.trino]\ncatalog = \"Bad-Name\"\n";
    let errs = errors(text);
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert_eq!(span_text(text, &errs[0]), "\"Bad-Name\"");
}

#[test]
fn oidc_two_issuers_with_defaults() {
    let a = auth(
        "[[auth.oidc]]\nissuer = \"https://idp-a.example\"\naudience = \"ontoforge\"\njwks_uri = \"https://idp-a.example/jwks\"\n\n[[auth.oidc]]\nissuer = \"https://idp-b.example\"\naudience = \"ontoforge\"\njwks_uri = \"https://idp-b.example/jwks\"\ngroups_claim = \"roles\"\nleeway_secs = 5\n",
    );
    assert_eq!(a.oidc.len(), 2);
    assert_eq!(a.oidc[0].groups_claim, "groups");
    assert_eq!(a.oidc[0].algorithms, ["RS256", "ES256", "EdDSA"]);
    assert_eq!(a.oidc[0].leeway_secs, 30);
    assert_eq!(a.oidc[0].jwks_min_refresh_secs, 30);
    assert_eq!(a.oidc[0].jwks_max_stale_secs, 86_400);
    assert!(a.oidc[0].ca_file.is_none());
    assert_eq!(a.oidc[1].groups_claim, "roles");
    assert_eq!(a.oidc[1].leeway_secs, 5);
}

const OIDC: &str = "[[auth.oidc]]\nissuer = \"https://idp.example\"\naudience = \"ontoforge\"\njwks_uri = \"https://idp.example/jwks\"\n";

#[test]
fn oidc_rejects_symmetric_and_none_algorithms() {
    for list in ["[\"HS256\"]", "[\"none\"]", "[]", "[\"RS256\", \"HS512\"]"] {
        let text = format!("{OIDC}algorithms = {list}\n");
        let errs = errors(&text);
        assert_eq!(errs.len(), 1, "{list}: {errs:?}");
        assert!(errs[0].message.contains("algorithms"), "{list}: {errs:?}");
        assert!(errs[0].span.is_some());
    }
    let bad = format!("{OIDC}algorithms = [\"PS384\", \"ES512x\"]\n");
    assert!(!errors(&bad).is_empty());
    let ok = format!("{OIDC}algorithms = [\"PS384\", \"EdDSA\"]\n");
    assert_eq!(auth(&ok).oidc[0].algorithms, ["PS384", "EdDSA"]);
}

#[test]
fn oidc_issuer_rules() {
    let with = |issuer: &str, jwks: &str| {
        format!("[[auth.oidc]]\nissuer = \"{issuer}\"\naudience = \"a\"\njwks_uri = \"{jwks}\"\n")
    };
    let text = with("http://idp.example.com", "https://idp.example.com/jwks");
    let errs = errors(&text);
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert_eq!(span_text(&text, &errs[0]), "\"http://idp.example.com\"");

    let text = with("https://a|b", "https://a/jwks");
    let errs = errors(&text);
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert!(errs[0].message.contains('|'), "{errs:?}");

    let text = with("https://idp.example", "http://jwks.example/keys");
    assert_eq!(errors(&text).len(), 1);

    let text = format!(
        "{}\n{}",
        with("https://idp.example", "https://idp.example/jwks"),
        with("https://idp.example", "https://idp.example/jwks2")
    );
    let errs = errors(&text);
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert!(errs[0].message.contains("duplicate"), "{errs:?}");

    let text = with("http://127.0.0.1:9000", "http://localhost:9000/jwks");
    assert_eq!(auth(&text).oidc.len(), 1);
}

#[test]
fn oidc_numeric_bounds() {
    for extra in [
        "leeway_secs = 301",
        "jwks_min_refresh_secs = 0",
        "jwks_min_refresh_secs = 100\njwks_max_stale_secs = 50",
        "groups_claim = \"\"",
    ] {
        let text = format!("{OIDC}{extra}\n");
        assert_eq!(errors(&text).len(), 1, "{extra}");
    }
}

#[test]
fn principal_attrs_types() {
    let a = auth(
        "[auth.principal_attrs.tenantId]\nclaim = \"tenant\"\ntype = \"string\"\n[auth.principal_attrs.level]\nclaim = \"lvl\"\ntype = \"integer\"\n[auth.principal_attrs.admin]\nclaim = \"adm\"\ntype = \"boolean\"\n[auth.principal_attrs.regions]\nclaim = \"regs\"\ntype = \"string_list\"\n[auth.principal_attrs.sites]\nclaim = \"sites\"\ntype = \"integer_list\"\n",
    );
    assert_eq!(a.principal_attrs.len(), 5);
    assert_eq!(a.principal_attrs["regions"].ty, AttrType::StringList);
    assert_eq!(a.principal_attrs["level"].claim, "lvl");

    let text = "[auth.principal_attrs.regions]\nclaim = \"r\"\ntype = \"list<string>\"\n";
    let errs = errors(text);
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert!(errs[0].span.is_some());

    for name in ["TenantId", "in"] {
        let text = format!("[auth.principal_attrs.{name}]\nclaim = \"c\"\ntype = \"string\"\n");
        let errs = errors(&text);
        assert_eq!(errs.len(), 1, "{name}: {errs:?}");
        assert!(text[errs[0].span.clone().expect("span")].contains(name));
    }
    let text = "[auth.principal_attrs.tenantId]\nclaim = \"\"\ntype = \"string\"\n";
    assert_eq!(errors(text).len(), 1);
}

#[test]
fn dev_auth_key_file() {
    let a = auth("[auth.dev]\nkey_file = \"dev.key\"\n");
    assert_eq!(a.dev.expect("dev").key_file.as_deref(), Some("dev.key"));
    let a = auth("[auth.dev]\n");
    assert!(a.dev.expect("dev").key_file.is_none());
}

#[test]
fn unknown_key_in_store_or_auth_is_v20() {
    let text = "[store]\nkind = \"sqlite\"\npath = \"e.db\"\nfoo = 1\n";
    let errs = errors(text);
    assert!(errs[0].message.contains("unknown field"), "{errs:?}");
    assert_eq!(span_text(text, &errs[0]), "foo");
    let text = format!("{OIDC}x = 1\n");
    let errs = errors(&text);
    assert!(errs[0].message.contains("unknown field"), "{errs:?}");
    assert_eq!(span_text(&text, &errs[0]), "x");
}

#[test]
fn store_and_auth_are_optional() {
    let cfg = parse_server_config("").expect("empty config");
    assert!(cfg.store.is_none() && cfg.auth.is_none());
}
