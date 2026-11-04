//! `ontoforge dev-token`: issue a development token (AUTH-02).
//!
//! The command reads the server configuration for the principal attribute declarations and
//! the key file path, creates the signing key on first use, and prints the token as the only
//! line on stdout. Everything else goes to stderr, and stderr never carries the token. Exit
//! code 0 on success, 1 for any problem with the arguments, the configuration or the key.

use std::collections::BTreeMap;
use std::time::Duration;

use camino::{Utf8Path, Utf8PathBuf};
use chrono::{SecondsFormat, TimeDelta, Utc};
use ontoforge_auth::authn::dev::{DEV_ISSUER, DevClaims, DevKey, issue_dev_token};
use ontoforge_engine::config::{PrincipalAttrConfig, load_server_config};
use ontoforge_schema::diag::Diagnostic;
use ontoforge_schema::ir::AttrType;
use ontoforge_schema::loader::SourceMap;
use serde_json::{Map, Value};

use crate::cli::DevTokenArgs;
use crate::online;
use crate::render::{self, Format};

/// Longest token lifetime: 30 days.
const MAX_TTL_SECS: u64 = 30 * 86_400;

/// Why no token was issued.
enum Failure {
    /// Problems in the server configuration, reported as ONT-V20.
    Config(Vec<Diagnostic>),
    Message(String),
}

impl From<String> for Failure {
    fn from(message: String) -> Self {
        Self::Message(message)
    }
}

/// Runs `ontoforge dev-token` and returns the process exit code.
pub fn run(args: &DevTokenArgs) -> u8 {
    match issue(args) {
        Ok(()) => 0,
        Err(Failure::Config(diagnostics)) => {
            let text = render::render(&diagnostics, &SourceMap::default(), Format::Human, false);
            eprint!("{text}");
            1
        }
        Err(Failure::Message(message)) => {
            eprintln!("error: {message}");
            1
        }
    }
}

fn issue(args: &DevTokenArgs) -> Result<(), Failure> {
    let path = &args.server_config;
    let (config, _text) = load_server_config(path).map_err(|(errors, text)| {
        // The text only turns spans into line and column; it is never printed.
        Failure::Config(
            errors
                .iter()
                .map(|e| online::config_diagnostic(e, path.as_str(), text.as_deref()))
                .collect(),
        )
    })?;
    let no_attrs = BTreeMap::new();
    let (declared, configured_key) = match &config.auth {
        Some(auth) => (
            &auth.principal_attrs,
            auth.dev.as_ref().and_then(|dev| dev.key_file.as_deref()),
        ),
        None => (&no_attrs, None),
    };

    // Everything that can be wrong with the arguments is checked before the key file is
    // created, so a rejected call leaves no file behind.
    if args.sub.is_empty() {
        return Err("`--sub` must not be empty".to_owned().into());
    }
    let attrs = attr_claims(&args.attr, declared)?;
    let ttl = parse_ttl(&args.ttl)?;
    let groups = parse_groups(args.groups.as_deref());
    let expires = Utc::now()
        .checked_add_signed(TimeDelta::seconds(
            i64::try_from(ttl.as_secs()).map_err(|_| ttl_range())?,
        ))
        .ok_or_else(ttl_range)?;
    let key_path = key_path(args, path, configured_key)?;

    let key = DevKey::load_or_create(&key_path).map_err(|e| e.to_string())?;
    let claims = DevClaims {
        sub: args.sub.clone(),
        groups,
        attrs,
        ttl,
    };
    let token = issue_dev_token(&key, &claims).map_err(|e| e.to_string())?;
    println!("{token}");
    eprintln!(
        "dev token for {DEV_ISSUER}|{}, expires {}; accepted only by a server started with --dev",
        args.sub,
        expires.to_rfc3339_opts(SecondsFormat::Secs, true)
    );
    Ok(())
}

fn ttl_range() -> String {
    "`--ttl` is out of range".to_owned()
}

/// The key file: `--key-file` (relative to the working directory), else `[auth.dev] key_file`
/// (relative to the configuration file).
fn key_path(
    args: &DevTokenArgs,
    config_path: &Utf8Path,
    configured: Option<&str>,
) -> Result<Utf8PathBuf, String> {
    if let Some(path) = &args.key_file {
        return Ok(path.clone());
    }
    let Some(file) = configured else {
        return Err(
            "no signing key file: pass `--key-file` or set `key_file` in [auth.dev]".to_owned(),
        );
    };
    let dir = config_path
        .parent()
        .filter(|p| !p.as_str().is_empty())
        .unwrap_or(Utf8Path::new("."));
    Ok(dir.join(file))
}

/// Group names from a comma separated list: trimmed, empty names dropped, duplicates removed.
fn parse_groups(list: Option<&str>) -> Vec<String> {
    let mut groups: Vec<String> = Vec::new();
    for name in list.unwrap_or_default().split(',').map(str::trim) {
        if !name.is_empty() && !groups.iter().any(|g| g == name) {
            groups.push(name.to_owned());
        }
    }
    groups
}

/// `<n>m`, `<n>h` or `<n>d`.
fn parse_ttl(text: &str) -> Result<Duration, String> {
    let invalid = || "`--ttl` must be a number followed by m, h or d, for example 8h".to_owned();
    let (digits, unit_secs) = if let Some(n) = text.strip_suffix('m') {
        (n, 60_u64)
    } else if let Some(n) = text.strip_suffix('h') {
        (n, 3_600)
    } else if let Some(n) = text.strip_suffix('d') {
        (n, 86_400)
    } else {
        return Err(invalid());
    };
    let count: u64 = digits.parse().map_err(|_| invalid())?;
    let secs = count.checked_mul(unit_secs).ok_or_else(ttl_range)?;
    if secs == 0 || secs > MAX_TTL_SECS {
        return Err("`--ttl` must be between 1m and 30d".to_owned());
    }
    Ok(Duration::from_secs(secs))
}

/// The `--attr name=value` arguments as token claims, keyed by the declared claim name. A later
/// argument for the same attribute replaces an earlier one.
fn attr_claims(
    raw: &[String],
    declared: &BTreeMap<String, PrincipalAttrConfig>,
) -> Result<Map<String, Value>, String> {
    let mut claims = Map::new();
    for item in raw {
        let (name, value) = item
            .split_once('=')
            .ok_or("an `--attr` argument must look like name=value")?;
        let attr = declared.get(name).ok_or_else(|| {
            format!("attribute `{name}` is not declared in [auth.principal_attrs]")
        })?;
        let value = attr_json(value, attr.ty)
            .ok_or_else(|| format!("attribute `{name}` must be {}", expected(attr.ty)))?;
        claims.insert(attr.claim.clone(), value);
    }
    Ok(claims)
}

fn expected(ty: AttrType) -> &'static str {
    match ty {
        AttrType::String => "a string",
        AttrType::Integer => "an integer",
        AttrType::Boolean => "`true` or `false`",
        AttrType::StringList => "a comma separated list of strings",
        AttrType::IntegerList => "a comma separated list of integers",
    }
}

/// `value` as the JSON a token claim of type `ty` holds. An empty list value is the empty list.
fn attr_json(value: &str, ty: AttrType) -> Option<Value> {
    match ty {
        AttrType::String => Some(Value::from(value)),
        AttrType::Integer => value.parse::<i64>().ok().map(Value::from),
        AttrType::Boolean => match value {
            "true" => Some(Value::Bool(true)),
            "false" => Some(Value::Bool(false)),
            _ => None,
        },
        AttrType::StringList => Some(Value::Array(elements(value).map(Value::from).collect())),
        AttrType::IntegerList => elements(value)
            .map(|e| e.parse::<i64>().ok().map(Value::from))
            .collect::<Option<Vec<_>>>()
            .map(Value::Array),
    }
}

fn elements(value: &str) -> impl Iterator<Item = &str> {
    value.split(',').filter(|_| !value.is_empty())
}
