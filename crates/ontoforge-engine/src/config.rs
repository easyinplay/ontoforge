//! `ontoforge-server.toml`: Trino connection, service identities, engine profiles
//! (D-06..D-10), the edit store (`[store]`) and authentication (`[auth]`).
//!
//! Secrets are never written into the file: an identity names an environment variable
//! (`*_env`) or a file (`*_file`) that holds the value. The same holds for the edit
//! store's PostgreSQL connection string (`url_env` / `url_file`).

use std::collections::BTreeMap;
use std::ops::Range;
use std::time::Duration;

use camino::Utf8Path;
use ontoforge_schema::diag::parse_custom_de_error;
use ontoforge_schema::ir::names::CEL_RESERVED_WORDS;
use ontoforge_schema::ir::{AttrType, ProfileName};
use ontoforge_schema::loader::{MAX_FILE_BYTES, ReadFailure, read_limited};
use reqwest::Url;
use secrecy::SecretString;
use serde::Deserialize;
use toml::de::{DeTable, DeValue};

use crate::trino::l2_health::L2AclConfig;
use crate::trino::{Auth, TrinoConnection, TrinoEngine, is_valid_client_tag};
use crate::{
    DEFAULT_MAX_BYTES, DEFAULT_MAX_PAGE_BYTES, DEFAULT_MAX_ROWS, DEFAULT_QUERY_TIMEOUT_MS,
    EngineError, QueryContext,
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
    /// Where the edit store lives.
    #[serde(default)]
    pub store: Option<StoreConfig>,
    /// How callers authenticate.
    #[serde(default)]
    pub auth: Option<AuthConfig>,
    /// Outbound Action event delivery by a trusted worker process.
    #[serde(default)]
    pub webhook: Option<WebhookWorkerConfig>,
}

/// The `[webhook]` section used by `ontoforge webhook-worker`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebhookWorkerConfig {
    pub target: String,
    pub poll_interval_ms: u64,
    pub request_timeout_ms: u64,
    pub lease_duration_ms: u64,
    pub batch_limit: u32,
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
    /// Deployment-owned Trino ACL declaration and probe fixtures. Absent means L2 disabled.
    #[serde(default)]
    pub l2_acl: Option<L2AclConfig>,
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
    DEFAULT_MAX_PAGE_BYTES
}

fn default_client_tags() -> Vec<String> {
    vec!["ontoforge".to_owned()]
}

impl TrinoConfig {
    /// A [`QueryContext`] carrying this configuration's limits and client tags. The one place
    /// the limits are turned into a context. `extra_tags` (the caller's own tags) follow the
    /// configured `client_tags`; a tag already present is not repeated.
    #[must_use]
    pub fn query_context(&self, trace_token: String, extra_tags: Vec<String>) -> QueryContext {
        let mut client_tags = self.client_tags.clone();
        for tag in extra_tags {
            if !client_tags.contains(&tag) {
                client_tags.push(tag);
            }
        }
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

/// Which database holds the edit store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoreKind {
    Sqlite,
    Postgres,
}

/// The `[store]` section (ADR-0016).
///
/// A `sqlite` store needs `path`. A `postgres` store needs exactly one of `url_env` and
/// `url_file`; the connection string itself is never written into the file.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoreConfig {
    pub kind: StoreKind,
    /// SQLite database file, relative to the configuration file.
    #[serde(default)]
    pub path: Option<String>,
    /// Environment variable that holds the PostgreSQL connection string.
    #[serde(default)]
    pub url_env: Option<String>,
    /// File that holds the PostgreSQL connection string, relative to the configuration file.
    #[serde(default)]
    pub url_file: Option<String>,
    /// PostgreSQL schema of the typed edit tables.
    #[serde(default = "default_schema_edit")]
    pub schema_edit: String,
    /// PostgreSQL schema of system tables (edit log, definition snapshots).
    #[serde(default = "default_schema_sys")]
    pub schema_sys: String,
    /// How long a write waits for a lock.
    #[serde(default = "default_lock_timeout_ms")]
    pub lock_timeout_ms: u64,
    /// How long the Trino visibility probe may take.
    #[serde(default = "default_probe_timeout_ms")]
    pub probe_timeout_ms: u64,
    /// Trino access to a PostgreSQL edit store.
    #[serde(default)]
    pub trino: Option<StoreTrinoConfig>,
}

/// The `[store.trino]` section.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoreTrinoConfig {
    /// Trino catalog that maps the edit store's PostgreSQL schemas.
    pub catalog: String,
}

fn default_schema_edit() -> String {
    "ontoforge_edit".to_owned()
}

fn default_schema_sys() -> String {
    "ontoforge_sys".to_owned()
}

fn default_lock_timeout_ms() -> u64 {
    5_000
}

fn default_probe_timeout_ms() -> u64 {
    60_000
}

/// The `[auth]` section.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthConfig {
    /// Trusted identity providers. Group names are namespaced by issuer, so several
    /// issuers can be trusted at once.
    #[serde(default)]
    pub oidc: Vec<OidcConfig>,
    /// Principal attributes by name: which token claim feeds them and their type (D-05).
    #[serde(default)]
    pub principal_attrs: BTreeMap<String, PrincipalAttrConfig>,
    /// Development tokens signed with a local key.
    #[serde(default)]
    pub dev: Option<DevAuthConfig>,
}

/// One `[[auth.oidc]]` entry.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OidcConfig {
    pub issuer: String,
    pub audience: String,
    pub jwks_uri: String,
    #[serde(default = "default_groups_claim")]
    pub groups_claim: String,
    /// Accepted signature algorithms: asymmetric ones only.
    #[serde(default = "default_algorithms")]
    pub algorithms: Vec<String>,
    #[serde(default = "default_leeway_secs")]
    pub leeway_secs: u64,
    #[serde(default = "default_jwks_min_refresh_secs")]
    pub jwks_min_refresh_secs: u64,
    #[serde(default = "default_jwks_max_stale_secs")]
    pub jwks_max_stale_secs: u64,
    /// PEM file with extra root certificates for the key set fetch, relative to the
    /// configuration file.
    #[serde(default)]
    pub ca_file: Option<String>,
}

fn default_groups_claim() -> String {
    "groups".to_owned()
}

fn default_algorithms() -> Vec<String> {
    ["RS256", "ES256", "EdDSA"].map(str::to_owned).to_vec()
}

fn default_leeway_secs() -> u64 {
    30
}

fn default_jwks_min_refresh_secs() -> u64 {
    30
}

fn default_jwks_max_stale_secs() -> u64 {
    86_400
}

/// One `[auth.principal_attrs.<name>]` entry.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrincipalAttrConfig {
    pub claim: String,
    #[serde(rename = "type")]
    pub ty: AttrType,
}

/// The `[auth.dev]` section.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DevAuthConfig {
    /// File with the signing key for development tokens, relative to the configuration
    /// file. Without it the server generates a key at start.
    #[serde(default)]
    pub key_file: Option<String>,
}

/// Signature algorithms an OIDC issuer may be configured with. Symmetric algorithms and
/// `none` are not on the list (algorithm confusion).
const OIDC_ALGORITHMS: [&str; 9] = [
    "RS256", "RS384", "RS512", "PS256", "PS384", "PS512", "ES256", "ES384", "EdDSA",
];

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
    if unknown == Some("url") && raw.contains("`url_env`") {
        return ConfigError::new(
            "plaintext secrets are not accepted in ontoforge-server.toml (found `url`)",
        )
        .help(
            "reference the connection string with `url_env` (an environment variable name) or `url_file` (a file path)",
        )
        .at(span);
    }
    ConfigError::new(raw).at(span)
}

/// One step of a path into the parsed document.
#[derive(Clone, Copy)]
enum Seg<'s> {
    Key(&'s str),
    Idx(usize),
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

    /// Span of the value at `path` (keys and array indexes), for error reports that
    /// should point at what is wrong rather than at the key.
    fn value_span(&self, path: &[Seg<'_>]) -> Option<Range<usize>> {
        enum Cur<'c, 'i> {
            Table(&'c DeTable<'i>),
            Array(&'c [toml::Spanned<DeValue<'i>>]),
            Leaf,
        }
        fn descend<'c, 'i>(value: &'c DeValue<'i>) -> Cur<'c, 'i> {
            match value {
                DeValue::Table(t) => Cur::Table(t),
                DeValue::Array(a) => Cur::Array(a),
                _ => Cur::Leaf,
            }
        }
        let mut cur = Cur::Table(self.root.as_ref()?.get_ref());
        let mut found = None;
        for seg in path {
            let value = match (&cur, seg) {
                (Cur::Table(table), Seg::Key(name)) => {
                    table
                        .iter()
                        .find(|(k, _)| {
                            let key: &str = k.get_ref().as_ref();
                            key == *name
                        })?
                        .1
                }
                (Cur::Array(items), Seg::Idx(i)) => items.get(*i)?,
                _ => return None,
            };
            found = Some(value.span());
            cur = descend(value.get_ref());
        }
        found
    }

    fn check(&self, cfg: &ServerConfig) -> Vec<ConfigError> {
        let mut errors = Vec::new();
        if let Some(trino) = &cfg.trino {
            self.check_trino(trino, &mut errors);
        }
        if let Some(store) = &cfg.store {
            self.check_store(store, &mut errors);
        }
        if let (Some(acl), Some(edit_catalog)) = (
            cfg.trino.as_ref().and_then(|trino| trino.l2_acl.as_ref()),
            cfg.store.as_ref().and_then(|store| store.trino.as_ref()),
        ) && acl.edit.catalog != edit_catalog.catalog
        {
            errors.push(ConfigError::new(
                "L2 ACL edit probe must use the configured edit-store catalog",
            ));
        }
        if let Some(auth) = &cfg.auth {
            self.check_auth(auth, &mut errors);
        }
        if let Some(webhook) = &cfg.webhook {
            self.check_webhook(webhook, &mut errors);
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

    fn check_webhook(&self, webhook: &WebhookWorkerConfig, errors: &mut Vec<ConfigError>) {
        let target = Url::parse(&webhook.target).ok();
        if target.as_ref().is_none_or(|url| {
            !matches!(url.scheme(), "http" | "https")
                || url.host().is_none()
                || !url.username().is_empty()
                || url.password().is_some()
                || url.fragment().is_some()
        }) {
            errors.push(
                ConfigError::new(
                    "`webhook.target` must be an http or https URL without credentials or fragment",
                )
                .at(self.key_span(&["webhook", "target"])),
            );
        }
        for (key, value) in [
            ("poll_interval_ms", webhook.poll_interval_ms),
            ("request_timeout_ms", webhook.request_timeout_ms),
            ("lease_duration_ms", webhook.lease_duration_ms),
        ] {
            if value == 0 {
                errors.push(
                    ConfigError::new(format!("`webhook.{key}` must be greater than zero"))
                        .at(self.key_span(&["webhook", key])),
                );
            }
        }
        if webhook.lease_duration_ms <= webhook.request_timeout_ms {
            errors.push(
                ConfigError::new("`webhook.lease_duration_ms` must exceed request_timeout_ms")
                    .at(self.key_span(&["webhook", "lease_duration_ms"])),
            );
        }
        if !(1..=100).contains(&webhook.batch_limit) {
            errors.push(
                ConfigError::new("`webhook.batch_limit` must be between 1 and 100")
                    .at(self.key_span(&["webhook", "batch_limit"])),
            );
        }
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
            if l2.user == trino.identities.ontology.user {
                errors.push(ConfigError::new(
                    "ontology and l2 identities must use different users",
                ));
            }
        }
    }

    fn check_store(&self, store: &StoreConfig, errors: &mut Vec<ConfigError>) {
        use Seg::Key;
        let at = |key: &str| self.value_span(&[Key("store"), Key(key)]);
        match store.kind {
            StoreKind::Sqlite => {
                match store.path.as_deref() {
                    None => {
                        errors.push(ConfigError::new("a sqlite store needs `path`").at(at("kind")));
                    }
                    Some("") => {
                        errors.push(ConfigError::new("`path` must not be empty").at(at("path")));
                    }
                    Some(_) => {}
                }
                if store.url_env.is_some() || store.url_file.is_some() {
                    errors.push(
                        ConfigError::new(
                            "a sqlite store must not set `url_env` or `url_file`; they belong to a postgres store",
                        )
                        .at(at("url_env").or_else(|| at("url_file"))),
                    );
                }
            }
            StoreKind::Postgres => {
                let sources =
                    usize::from(store.url_env.is_some()) + usize::from(store.url_file.is_some());
                if sources != 1 {
                    errors.push(
                        ConfigError::new(
                            "a postgres store needs exactly one of `url_env` and `url_file`",
                        )
                        .help("the connection string is read from that variable or file, never from this file")
                        .at(at("kind")),
                    );
                }
                if store.path.is_some() {
                    errors.push(
                        ConfigError::new(
                            "a postgres store must not set `path`; it belongs to a sqlite store",
                        )
                        .at(at("path")),
                    );
                }
                if store.url_env.as_deref().is_some_and(|n| !is_env_name(n)) {
                    errors.push(
                        ConfigError::new(
                            "`url_env` must be an environment variable name ([A-Za-z_][A-Za-z0-9_]*)",
                        )
                        .at(at("url_env")),
                    );
                }
            }
        }
        for (key, name) in [
            ("schema_edit", &store.schema_edit),
            ("schema_sys", &store.schema_sys),
        ] {
            if !is_schema_name(name) {
                errors.push(
                    ConfigError::new(format!("`{key}` must match ^[a-z_][a-z0-9_]{{0,62}}$"))
                        .at(at(key)),
                );
            }
        }
        if store.schema_edit == store.schema_sys {
            errors.push(
                ConfigError::new("`schema_edit` and `schema_sys` must differ").at(at("schema_sys")),
            );
        }
        for (key, value, max) in [
            ("lock_timeout_ms", store.lock_timeout_ms, 600_000),
            ("probe_timeout_ms", store.probe_timeout_ms, 3_600_000),
        ] {
            if !(1..=max).contains(&value) {
                errors.push(
                    ConfigError::new(format!("`{key}` must be between 1 and {max}")).at(at(key)),
                );
            }
        }
        if let Some(trino) = &store.trino
            && !is_catalog_name(&trino.catalog)
        {
            errors.push(
                ConfigError::new("`catalog` must match ^[a-z][a-z0-9_]{0,62}$")
                    .at(self.value_span(&[Key("store"), Key("trino"), Key("catalog")])),
            );
        }
    }

    fn check_auth(&self, auth: &AuthConfig, errors: &mut Vec<ConfigError>) {
        use Seg::{Idx, Key};
        let mut seen: Vec<&str> = Vec::new();
        for (index, oidc) in auth.oidc.iter().enumerate() {
            let at = |key: &str| self.value_span(&[Key("auth"), Key("oidc"), Idx(index), Key(key)]);
            if oidc.issuer.contains('|') {
                errors.push(
                    ConfigError::new("`issuer` must not contain `|`")
                        .help("`|` separates the issuer from a group name in namespaced group keys")
                        .at(at("issuer")),
                );
            } else {
                if let Some(problem) = oidc_url_problem(&oidc.issuer) {
                    errors.push(ConfigError::new(format!("`issuer` {problem}")).at(at("issuer")));
                }
                if seen.contains(&oidc.issuer.as_str()) {
                    errors.push(
                        ConfigError::new("duplicate `issuer` in [[auth.oidc]]").at(at("issuer")),
                    );
                }
                seen.push(&oidc.issuer);
            }
            if let Some(problem) = oidc_url_problem(&oidc.jwks_uri) {
                errors.push(ConfigError::new(format!("`jwks_uri` {problem}")).at(at("jwks_uri")));
            }
            for (key, value) in [
                ("audience", &oidc.audience),
                ("groups_claim", &oidc.groups_claim),
            ] {
                if value.is_empty() {
                    errors.push(ConfigError::new(format!("`{key}` must not be empty")).at(at(key)));
                }
            }
            if oidc.algorithms.is_empty() {
                errors.push(
                    ConfigError::new("`algorithms` must list at least one algorithm")
                        .at(at("algorithms")),
                );
            }
            for algorithm in &oidc.algorithms {
                if !OIDC_ALGORITHMS.contains(&algorithm.as_str()) {
                    errors.push(
                        ConfigError::new(format!("`algorithms` entry `{algorithm}` is not allowed"))
                            .help("only asymmetric algorithms are accepted: RS256, RS384, RS512, PS256, PS384, PS512, ES256, ES384, EdDSA")
                            .at(at("algorithms")),
                    );
                }
            }
            if oidc.leeway_secs > 300 {
                errors.push(
                    ConfigError::new("`leeway_secs` must be at most 300").at(at("leeway_secs")),
                );
            }
            if oidc.jwks_min_refresh_secs < 1 {
                errors.push(
                    ConfigError::new("`jwks_min_refresh_secs` must be at least 1")
                        .at(at("jwks_min_refresh_secs")),
                );
            }
            if oidc.jwks_max_stale_secs < oidc.jwks_min_refresh_secs {
                errors.push(
                    ConfigError::new(
                        "`jwks_max_stale_secs` must not be smaller than `jwks_min_refresh_secs`",
                    )
                    .at(at("jwks_max_stale_secs")),
                );
            }
        }
        for (name, attr) in &auth.principal_attrs {
            if !is_attr_name(name) || CEL_RESERVED_WORDS.contains(&name.as_str()) {
                errors.push(
                    ConfigError::new(format!(
                        "principal attribute name `{name}` must match ^[a-z][A-Za-z0-9]{{0,99}}$ and not be a CEL reserved word"
                    ))
                    .at(self.key_span(&["auth", "principal_attrs", name.as_str()])),
                );
            }
            if attr.claim.is_empty() {
                errors.push(
                    ConfigError::new("`claim` must not be empty").at(self.value_span(&[
                        Key("auth"),
                        Key("principal_attrs"),
                        Key(name.as_str()),
                        Key("claim"),
                    ])),
                );
            }
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
        // A value that is not a variable name is most likely the secret itself, pasted into
        // the wrong key: name the key, never the value.
        for (key, name) in [
            ("password_env", &identity.password_env),
            ("token_env", &identity.token_env),
        ] {
            if name.as_deref().is_some_and(|n| !is_env_name(n)) {
                problem(format!(
                    "identity `{label}`: `{key}` must be an environment variable name ([A-Za-z_][A-Za-z0-9_]*)"
                ));
            }
        }
    }
}

/// Whether `s` has the shape of an environment variable name.
fn is_env_name(s: &str) -> bool {
    let mut bytes = s.bytes();
    bytes
        .next()
        .is_some_and(|c| c == b'_' || c.is_ascii_alphabetic())
        && bytes.all(|c| c == b'_' || c.is_ascii_alphanumeric())
}

/// Why `value` cannot be an OIDC endpoint, or `None` when it can: https, or http to a
/// loopback host (a local test identity provider).
fn oidc_url_problem(value: &str) -> Option<&'static str> {
    let Ok(url) = Url::parse(value) else {
        return Some("is not a valid URL");
    };
    if !url.username().is_empty() || url.password().is_some() {
        return Some("must not carry user information");
    }
    let Some(host) = url.host_str() else {
        return Some("has no host");
    };
    let loopback = host == "localhost"
        || host
            .trim_start_matches('[')
            .trim_end_matches(']')
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback());
    match url.scheme() {
        "https" => None,
        "http" if loopback => None,
        _ => Some("must use https (http is accepted only for a loopback host)"),
    }
}

/// `^[a-z_][a-z0-9_]{0,62}$`.
fn is_schema_name(s: &str) -> bool {
    let mut bytes = s.bytes();
    s.len() <= 63
        && bytes
            .next()
            .is_some_and(|c| c == b'_' || c.is_ascii_lowercase())
        && bytes.all(|c| c == b'_' || c.is_ascii_lowercase() || c.is_ascii_digit())
}

/// `^[a-z][a-z0-9_]{0,62}$`.
fn is_catalog_name(s: &str) -> bool {
    let mut bytes = s.bytes();
    s.len() <= 63
        && bytes.next().is_some_and(|c| c.is_ascii_lowercase())
        && bytes.all(|c| c == b'_' || c.is_ascii_lowercase() || c.is_ascii_digit())
}

/// `^[a-z][A-Za-z0-9]{0,99}$`, the property apiName shape.
fn is_attr_name(s: &str) -> bool {
    let mut bytes = s.bytes();
    s.len() <= 100
        && bytes.next().is_some_and(|c| c.is_ascii_lowercase())
        && bytes.all(|c| c.is_ascii_alphanumeric())
}

/// Looks up an environment variable. The library never reads the process environment
/// itself; the CLI passes the real lookup and tests pass a fixed map.
pub type EnvLookup<'a> = &'a dyn Fn(&str) -> Option<String>;

/// Reads a secret named by `env_name` or stored in `file` (relative to `config_dir`).
///
/// Errors name the variable or the path, never a value. A variable name that is not one, and
/// a file that does not exist, are not echoed either: such a reference is most likely the
/// secret itself, written into the wrong key.
pub fn resolve_secret(
    env_name: Option<&str>,
    file: Option<&str>,
    config_dir: &Utf8Path,
    env: EnvLookup,
) -> Result<SecretString, ConfigError> {
    match (env_name, file) {
        (Some(name), None) if !is_env_name(name) => Err(ConfigError::new(
            "the environment variable reference is not a valid variable name ([A-Za-z_][A-Za-z0-9_]*); it is not shown in case it is the secret itself",
        )),
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
            let bytes =
                read_limited(path.as_std_path(), true, MAX_SECRET_FILE_BYTES).map_err(|f| {
                    if matches!(f, ReadFailure::Io(std::io::ErrorKind::NotFound)) {
                        ConfigError::new(format!(
                            "the secret file does not exist (looked under `{config_dir}`); the reference is not shown in case it is the secret itself"
                        ))
                    } else {
                        read_error("secret file", &path, &f, MAX_SECRET_FILE_BYTES)
                    }
                })?;
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
    if cfg
        .identities
        .l2
        .as_ref()
        .is_some_and(|l2| l2.user == cfg.identities.ontology.user)
    {
        errors.push(ConfigError::new(
            "ontology and l2 identities must use different users",
        ));
    }
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
    let l2 = cfg.identities.l2.as_ref().map(|identity| {
        build_engine(&shared, "l2", identity).map(|engine| engine.into_l2(cfg.l2_acl.clone()))
    });

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
