//! `ontoforge-server.toml`: Trino connection, service identities and engine profiles
//! (D-06..D-10).
//!
//! Secrets are never written into the file: an identity names an environment variable
//! (`*_env`) or a file (`*_file`) that holds the value.

use std::collections::BTreeMap;
use std::ops::Range;

use camino::Utf8Path;
use ontoforge_schema::ir::ProfileName;
use serde::Deserialize;

use crate::trino::TrinoEngine;
use crate::{DEFAULT_MAX_BYTES, DEFAULT_MAX_ROWS, DEFAULT_QUERY_TIMEOUT_MS, QueryContext};

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
    /// A [`QueryContext`] with this configuration's limits.
    #[must_use]
    pub fn query_context(&self, trace_token: String, client_tags: Vec<String>) -> QueryContext {
        let _ = (trace_token, client_tags);
        QueryContext::new(String::new(), Vec::new())
    }
}

/// The two service identities.
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

/// Parses and checks `text`.
pub fn parse_server_config(text: &str) -> Result<ServerConfig, Vec<ConfigError>> {
    let _ = text;
    Err(Vec::new())
}

/// Reads and parses the file at `path`; also returns its text so the caller can turn spans
/// into line and column.
pub fn load_server_config(path: &Utf8Path) -> Result<(ServerConfig, String), Vec<ConfigError>> {
    let _ = path;
    Err(Vec::new())
}

/// Looks up an environment variable. The library never reads the process environment
/// itself; the CLI passes the real lookup and tests pass a fixed map.
pub type EnvLookup<'a> = &'a dyn Fn(&str) -> Option<String>;

/// Reads a secret named by `env_name` or stored in `file`.
pub fn resolve_secret(
    env_name: Option<&str>,
    file: Option<&str>,
    config_dir: &Utf8Path,
    env: EnvLookup,
) -> Result<secrecy::SecretString, ConfigError> {
    let _ = (env_name, file, config_dir, env);
    Err(ConfigError {
        message: String::new(),
        help: None,
        span: None,
    })
}

/// The engines for the configured service identities.
#[derive(Debug)]
pub struct TrinoEngines {
    pub ontology: TrinoEngine,
    pub l2: Option<TrinoEngine>,
}

/// Builds one [`TrinoEngine`] per configured identity, each with its own HTTP client.
pub fn build_trino_engines(
    cfg: &TrinoConfig,
    config_dir: &Utf8Path,
    env: EnvLookup,
) -> Result<TrinoEngines, Vec<ConfigError>> {
    let _ = (cfg, config_dir, env);
    Err(Vec::new())
}
