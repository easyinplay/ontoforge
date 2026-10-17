//! `ontoforge-server.toml`: Trino connection, service identities and engine profiles
//! (D-06..D-10).
//!
//! Secrets are never written into the file: an identity names an environment variable
//! (`*_env`) or a file (`*_file`) that holds the value.

use std::collections::BTreeMap;
use std::ops::Range;
use std::time::Duration;

use camino::Utf8Path;
use ontoforge_schema::diag::parse_custom_de_error;
use ontoforge_schema::ir::ProfileName;
use ontoforge_schema::loader::{MAX_FILE_BYTES, ReadFailure, read_limited};
use reqwest::Url;
use secrecy::SecretString;
use serde::Deserialize;
use toml::de::{DeTable, DeValue};

use crate::trino::{Auth, TrinoConnection, TrinoEngine, is_valid_client_tag};
use crate::{
    DEFAULT_MAX_BYTES, DEFAULT_MAX_ROWS, DEFAULT_QUERY_TIMEOUT_MS, EngineError, QueryContext,
};

/// Largest accepted secret file.
const MAX_SECRET_FILE_BYTES: u64 = 64 * 1024;

/// The parsed server configuration.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerConfig {
    #[serde(default)]
    pub trino: Option<TrinoConfig>,
    #[serde(default)]
    pub profiles: BTreeMap<ProfileName, ProfileEngine>,
}

/// The `[trino]` section.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrinoConfig {
    pub url: String,
    /// PEM file with extra root certificates, added to the platform roots.
    #[serde(default)]
    pub ca_file: Option<String>,
    #[serde(default = "default_connect_timeout_ms")]
    pub connect_timeout_ms: u64,
    #[serde(default = "default_query_timeout_ms")]
    pub query_timeout_ms: u64,
    #[serde(default = "default_max_rows")]
    pub max_rows: usize,
    #[serde(default = "default_max_bytes")]
    pub max_bytes: usize,
    #[serde(default = "default_max_page_bytes")]
    pub max_page_bytes: usize,
    /// Origins besides `url` a `nextUri` may point to (a gateway in front of Trino).
    #[serde(default)]
    pub next_uri_origins: Vec<String>,
    #[serde(default = "default_client_tags")]
    pub client_tags: Vec<String>,
    pub identities: Identities,
}

fn default_connect_timeout_ms() -> u64 {
    5_000
}

fn default_query_timeout_ms() -> u64 {
    DEFAULT_QUERY_TIMEOUT_MS
}

fn default_max_rows() -> usize {
    DEFAULT_MAX_ROWS
}

fn default_max_bytes() -> usize {
    DEFAULT_MAX_BYTES
}

fn default_max_page_bytes() -> usize {
    64 * 1024 * 1024
}

fn default_client_tags() -> Vec<String> {
    vec!["ontoforge".to_owned()]
}

impl TrinoConfig {
    /// A [`QueryContext`] carrying this configuration's limits. The one place the limits are
    /// turned into a context.
    #[must_use]
    pub fn query_context(&self, trace_token: String, client_tags: Vec<String>) -> QueryContext {
        QueryContext {
            trace_token,
            client_tags,
            timeout: Duration::from_millis(self.query_timeout_ms),
            max_rows: self.max_rows,
            max_bytes: self.max_bytes,
        }
    }
}

/// The two service identities. `l2` is parsed and built but not used in this phase.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Identities {
    pub ontology: IdentityConfig,
    #[serde(default)]
    pub l2: Option<IdentityConfig>,
}

/// One service identity.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityConfig {
    pub user: String,
    #[serde(default)]
    pub auth: AuthKind,
    #[serde(default)]
    pub password_env: Option<String>,
    #[serde(default)]
    pub password_file: Option<String>,
    #[serde(default)]
    pub token_env: Option<String>,
    #[serde(default)]
    pub token_file: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthKind {
    #[default]
    None,
    Basic,
    Bearer,
}

/// A `[profiles.<name>]` entry.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileEngine {
    pub engine: EngineKind,
}

/// Which engine a profile queries. `Sqlite` is the in-memory stand-in driven by fixtures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EngineKind {
    Trino,
    Sqlite,
}

/// Something wrong with the configuration. Texts name keys, variables and paths, never
/// secret values.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct ConfigError {
    pub message: String,
    pub help: Option<String>,
    pub span: Option<Range<usize>>,
}

impl ConfigError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            help: None,
            span: None,
        }
    }

    fn help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    fn at(mut self, span: Option<Range<usize>>) -> Self {
        self.span = span;
        self
    }
}

/// Parses and checks `text`.
pub fn parse_server_config(text: &str) -> Result<ServerConfig, Vec<ConfigError>> {
    let cfg: ServerConfig = toml::from_str(text).map_err(|e| vec![from_toml_error(&e)])?;
    let errors = Checker::new(text).check(&cfg);
    if errors.is_empty() {
        Ok(cfg)
    } else {
        Err(errors)
    }
}

/// Reads and parses the file at `path`; also returns its text so the caller can turn spans
/// into line and column. On failure the text comes back with the errors when the file could be
/// read, so the spans refer to exactly the text that was parsed (no second read). The text may
/// hold a rejected plaintext secret: use it for line and column only, never print it.
pub fn load_server_config(
    path: &Utf8Path,
) -> Result<(ServerConfig, String), (Vec<ConfigError>, Option<String>)> {
    let bytes = read_limited(path.as_std_path(), true, MAX_FILE_BYTES).map_err(|f| {
        (
            vec![read_error("configuration file", path, &f, MAX_FILE_BYTES)],
            None,
        )
    })?;
    let text = String::from_utf8(bytes).map_err(|_| {
        (
            vec![ConfigError::new(format!(
                "configuration file `{path}` is not valid UTF-8"
            ))],
            None,
        )
    })?;
    match parse_server_config(&text) {
        Ok(cfg) => Ok((cfg, text)),
        Err(errors) => Err((errors, Some(text))),
    }
}

fn read_error(what: &str, path: &Utf8Path, failure: &ReadFailure, limit: u64) -> ConfigError {
    let reason = match failure {
        ReadFailure::Io(kind) => kind.to_string(),
        ReadFailure::NotRegular => "not a regular file".to_owned(),
        ReadFailure::TooLarge(_) => format!("larger than {limit} bytes"),
    };
    ConfigError::new(format!("cannot read {what} `{path}`: {reason}"))
}

fn from_toml_error(e: &toml::de::Error) -> ConfigError {
    let raw = e.message();
    let span = e.span();
    if let Some(custom) = parse_custom_de_error(raw) {
        return ConfigError {
            message: custom.message,
            help: custom.help,
            span,
        };
    }
    let unknown = raw
        .strip_prefix("unknown field `")
        .and_then(|rest| rest.split_once('`'))
        .map(|(name, _)| name);
    if let Some(name @ ("password" | "token" | "secret")) = unknown {
        let help = match name {
            "password" => {
                "reference the password with `password_env` (an environment variable name) or `password_file` (a file path)"
            }
            "token" => {
                "reference the token with `token_env` (an environment variable name) or `token_file` (a file path)"
            }
            _ => {
                "reference secrets with `password_env`, `password_file`, `token_env` or `token_file`"
            }
        };
        return ConfigError::new(format!(
            "plaintext secrets are not accepted in ontoforge-server.toml (found `{name}`)"
        ))
        .help(help)
        .at(span);
    }
    ConfigError::new(raw).at(span)
}

/// Semantic checks on a configuration that deserialized.
struct Checker<'a> {
    root: Option<toml::Spanned<DeTable<'a>>>,
}

impl<'a> Checker<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            root: DeTable::parse(text).ok(),
        }
    }

    /// Span of the key at `path`, for error reports.
    fn key_span(&self, path: &[&str]) -> Option<Range<usize>> {
        let mut table = self.root.as_ref()?.get_ref();
        let (last, parents) = path.split_last()?;
        for name in parents {
            let (_, value) = table.iter().find(|(k, _)| {
                let key: &str = k.get_ref().as_ref();
                key == *name
            })?;
            let DeValue::Table(inner) = value.get_ref() else {
                return None;
            };
            table = inner;
        }
        let (key, _) = table.iter().find(|(k, _)| {
            let key: &str = k.get_ref().as_ref();
            key == *last
        })?;
        Some(key.span())
    }

    fn check(&self, cfg: &ServerConfig) -> Vec<ConfigError> {
        let mut errors = Vec::new();
        if let Some(trino) = &cfg.trino {
            self.check_trino(trino, &mut errors);
        }
        for (name, profile) in &cfg.profiles {
            if profile.engine == EngineKind::Trino && cfg.trino.is_none() {
                errors.push(
                    ConfigError::new(format!(
                        "profile `{name}` uses engine \"trino\" but there is no [trino] section"
                    ))
                    .help("add a [trino] section, or use engine = \"sqlite\"")
                    .at(self.key_span(&["profiles", name.as_str()])),
                );
            }
        }
        errors
    }

    fn check_trino(&self, trino: &TrinoConfig, errors: &mut Vec<ConfigError>) {
        let url_span = || self.key_span(&["trino", "url"]);
        let url = Url::parse(&trino.url).ok();
        if url.is_none() {
            errors.push(ConfigError::new("`url` is not a valid URL").at(url_span()));
        }
        if let Some(url) = &url {
            if !matches!(url.scheme(), "http" | "https") {
                errors.push(ConfigError::new("`url` must use http or https").at(url_span()));
            }
            if url.host_str().is_none() {
                errors.push(ConfigError::new("`url` has no host").at(url_span()));
            }
            if !url.username().is_empty() || url.password().is_some() {
                errors.push(
                    ConfigError::new("`url` must not carry user information")
                        .help("reference secrets with `*_env` or `*_file` in an identity")
                        .at(url_span()),
                );
            }
        }
        for (key, value) in [
            ("connect_timeout_ms", trino.connect_timeout_ms),
            ("query_timeout_ms", trino.query_timeout_ms),
            ("max_rows", trino.max_rows as u64),
            ("max_bytes", trino.max_bytes as u64),
            ("max_page_bytes", trino.max_page_bytes as u64),
        ] {
            if value == 0 {
                errors.push(
                    ConfigError::new(format!("`{key}` must be greater than zero"))
                        .at(self.key_span(&["trino", key])),
                );
            }
        }
        for (index, origin) in trino.next_uri_origins.iter().enumerate() {
            let span = self.key_span(&["trino", "next_uri_origins"]);
            let n = index + 1;
            let Ok(origin) = Url::parse(origin) else {
                errors.push(
                    ConfigError::new(format!("next_uri_origins entry #{n} is not a valid URL"))
                        .at(span),
                );
                continue;
            };
            let bare = origin.host_str().is_some()
                && matches!(origin.path(), "" | "/")
                && origin.query().is_none()
                && origin.fragment().is_none()
                && origin.username().is_empty()
                && origin.password().is_none();
            if !bare {
                errors.push(
                    ConfigError::new(format!(
                        "next_uri_origins entry #{n} must be of the form scheme://host[:port]"
                    ))
                    .at(span),
                );
            } else if let Some(url) = &url
                && origin.scheme() != url.scheme()
            {
                errors.push(
                    ConfigError::new(format!(
                        "next_uri_origins entry #{n} uses scheme `{}` but `url` uses `{}`",
                        origin.scheme(),
                        url.scheme()
                    ))
                    .help("an origin must keep the scheme of `url`, otherwise credentials could be sent in the clear")
                    .at(span),
                );
            }
        }
        if let Some(tag) = trino.client_tags.iter().find(|t| !is_valid_client_tag(t)) {
            errors.push(
                ConfigError::new(format!(
                    "client tag of {} characters is invalid",
                    tag.chars().count()
                ))
                .help("client tags are 1 to 64 characters from [A-Za-z0-9._:-]")
                .at(self.key_span(&["trino", "client_tags"])),
            );
        }

        let https = url.as_ref().is_none_or(|u| u.scheme() == "https");
        self.check_identity("ontology", &trino.identities.ontology, https, errors);
        if let Some(l2) = &trino.identities.l2 {
            self.check_identity("l2", l2, https, errors);
        }
    }

    fn check_identity(
        &self,
        label: &str,
        identity: &IdentityConfig,
        https: bool,
        errors: &mut Vec<ConfigError>,
    ) {
        let span = self
            .key_span(&["trino", "identities", label, "auth"])
            .or_else(|| self.key_span(&["trino", "identities", label]));
        let mut problem = |message: String| {
            errors.push(ConfigError::new(message).at(span.clone()));
        };
        if identity.user.is_empty()
            || reqwest::header::HeaderValue::from_str(&identity.user).is_err()
        {
            problem(format!("identity `{label}` needs a valid `user`"));
        }
        let kind = match identity.auth {
            AuthKind::None => "none",
            AuthKind::Basic => "basic",
            AuthKind::Bearer => "bearer",
        };
        if identity.auth != AuthKind::None && !https {
            problem(format!(
                "identity `{label}` uses {kind} authentication, which needs an https `url`"
            ));
        }
        let count = |env: &Option<String>, file: &Option<String>| {
            usize::from(env.is_some()) + usize::from(file.is_some())
        };
        let password_refs = count(&identity.password_env, &identity.password_file);
        let token_refs = count(&identity.token_env, &identity.token_file);
        let (wanted, unwanted, secret_kind) = match identity.auth {
            AuthKind::None => (0, password_refs + token_refs, ""),
            AuthKind::Basic => (password_refs, token_refs, "password"),
            AuthKind::Bearer => (token_refs, password_refs, "token"),
        };
        if identity.auth != AuthKind::None && wanted != 1 {
            problem(format!(
                "identity `{label}` with auth = \"{kind}\" needs exactly one of `{secret_kind}_env` and `{secret_kind}_file`"
            ));
        }
        if unwanted != 0 {
            problem(format!(
                "identity `{label}` with auth = \"{kind}\" has secret references that do not apply"
            ));
        }
    }
}

/// Looks up an environment variable. The library never reads the process environment
/// itself; the CLI passes the real lookup and tests pass a fixed map.
pub type EnvLookup<'a> = &'a dyn Fn(&str) -> Option<String>;

/// Reads a secret named by `env_name` or stored in `file` (relative to `config_dir`).
///
/// Errors name the variable or the path, never a value.
pub fn resolve_secret(
    env_name: Option<&str>,
    file: Option<&str>,
    config_dir: &Utf8Path,
    env: EnvLookup,
) -> Result<SecretString, ConfigError> {
    match (env_name, file) {
        (Some(name), None) => match env(name) {
            Some(value) if !value.is_empty() => Ok(SecretString::from(value)),
            Some(_) => Err(ConfigError::new(format!(
                "environment variable `{name}` is empty"
            ))),
            None => Err(ConfigError::new(format!(
                "environment variable `{name}` is not set"
            ))),
        },
        (None, Some(file)) => {
            let path = config_dir.join(file);
            let bytes = read_limited(path.as_std_path(), true, MAX_SECRET_FILE_BYTES)
                .map_err(|f| read_error("secret file", &path, &f, MAX_SECRET_FILE_BYTES))?;
            let text = String::from_utf8(bytes).map_err(|_| {
                ConfigError::new(format!("secret file `{path}` is not valid UTF-8"))
            })?;
            let value = text
                .strip_suffix("\r\n")
                .or_else(|| text.strip_suffix('\n'))
                .unwrap_or(&text);
            if value.is_empty() {
                return Err(ConfigError::new(format!("secret file `{path}` is empty")));
            }
            Ok(SecretString::from(value.to_owned()))
        }
        _ => Err(ConfigError::new(
            "exactly one of an environment variable name and a file path is required",
        )),
    }
}

/// The engines for the configured service identities.
#[derive(Debug)]
pub struct TrinoEngines {
    pub ontology: TrinoEngine,
    pub l2: Option<TrinoEngine>,
}

/// What every identity of one `[trino]` section shares.
struct Shared<'a> {
    cfg: &'a TrinoConfig,
    url: Url,
    extra_origins: Vec<Url>,
    ca_pem: Option<Vec<u8>>,
    config_dir: &'a Utf8Path,
    env: EnvLookup<'a>,
}

/// Builds one [`TrinoEngine`] per configured identity, each with its own HTTP client.
pub fn build_trino_engines(
    cfg: &TrinoConfig,
    config_dir: &Utf8Path,
    env: EnvLookup,
) -> Result<TrinoEngines, Vec<ConfigError>> {
    let mut errors = Vec::new();
    let url =
        Url::parse(&cfg.url).map_err(|_| vec![ConfigError::new("`url` is not a valid URL")])?;
    let mut extra_origins = Vec::new();
    for (index, origin) in cfg.next_uri_origins.iter().enumerate() {
        match Url::parse(origin) {
            Ok(origin) => extra_origins.push(origin),
            Err(_) => errors.push(ConfigError::new(format!(
                "next_uri_origins entry #{} is not a valid URL",
                index + 1
            ))),
        }
    }
    let ca_pem = match &cfg.ca_file {
        Some(file) => match read_ca(config_dir, file) {
            Ok(pem) => Some(pem),
            Err(e) => {
                errors.push(e);
                None
            }
        },
        None => None,
    };
    let shared = Shared {
        cfg,
        url,
        extra_origins,
        ca_pem,
        config_dir,
        env,
    };
    let ontology = build_engine(&shared, "ontology", &cfg.identities.ontology);
    let l2 = cfg
        .identities
        .l2
        .as_ref()
        .map(|identity| build_engine(&shared, "l2", identity));

    let mut built_l2 = None;
    match l2 {
        Some(Ok(engine)) => built_l2 = Some(engine),
        Some(Err(e)) => errors.push(e),
        None => {}
    }
    match ontology {
        Ok(ontology) if errors.is_empty() => Ok(TrinoEngines {
            ontology,
            l2: built_l2,
        }),
        Ok(_) => Err(errors),
        Err(e) => {
            errors.push(e);
            Err(errors)
        }
    }
}

fn read_ca(config_dir: &Utf8Path, file: &str) -> Result<Vec<u8>, ConfigError> {
    let path = config_dir.join(file);
    let pem = read_limited(path.as_std_path(), true, MAX_FILE_BYTES)
        .map_err(|f| read_error("ca_file", &path, &f, MAX_FILE_BYTES))?;
    match reqwest::Certificate::from_pem_bundle(&pem) {
        Ok(certs) if !certs.is_empty() => Ok(pem),
        _ => Err(ConfigError::new(format!(
            "ca_file `{path}` holds no PEM certificate"
        ))),
    }
}

fn build_engine(
    shared: &Shared<'_>,
    label: &str,
    identity: &IdentityConfig,
) -> Result<TrinoEngine, ConfigError> {
    let in_label = |e: ConfigError| ConfigError {
        message: format!("identity `{label}`: {}", e.message),
        ..e
    };
    let secret = |env_name: &Option<String>, file: &Option<String>| {
        resolve_secret(
            env_name.as_deref(),
            file.as_deref(),
            shared.config_dir,
            shared.env,
        )
        .map_err(in_label)
    };
    let auth = match identity.auth {
        AuthKind::None => Auth::None,
        AuthKind::Basic => Auth::Basic {
            password: secret(&identity.password_env, &identity.password_file)?,
        },
        AuthKind::Bearer => Auth::Bearer {
            token: secret(&identity.token_env, &identity.token_file)?,
        },
    };
    TrinoEngine::new(TrinoConnection {
        url: shared.url.clone(),
        user: identity.user.clone(),
        auth,
        ca_pem: shared.ca_pem.clone(),
        connect_timeout: Duration::from_millis(shared.cfg.connect_timeout_ms),
        max_page_bytes: shared.cfg.max_page_bytes,
        extra_origins: shared.extra_origins.clone(),
    })
    .map_err(|e| match e {
        EngineError::Config(message) => ConfigError::new(format!("identity `{label}`: {message}")),
        other => ConfigError::new(format!("identity `{label}`: {other}")),
    })
}
