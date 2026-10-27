//! Library-level raw DQL operations for an attested Trino identity.

use std::sync::Arc;

use ontoforge_auth::Principal;
use ontoforge_auth::scope::PrincipalScope;
use ontoforge_engine::QueryOutput;
use ontoforge_engine::config::TrinoConfig;
use ontoforge_engine::trino::TrinoEngine;
use ontoforge_storage::store::{ActiveDefinition, EditStore};

pub struct L2Service<'a> {
    pub principal: &'a Principal,
    pub scope: &'a PrincipalScope,
    pub active: Arc<ActiveDefinition>,
    pub engine: &'a TrinoEngine,
    pub store: &'a EditStore,
    pub config: &'a TrinoConfig,
}

#[derive(Debug, thiserror::Error)]
pub enum L2Error {
    #[error("L2 access denied")]
    Denied,
    #[error("invalid L2 SQL")]
    InvalidSql,
    #[error("L2 ACL is disabled")]
    Disabled,
    #[error("read audit unavailable")]
    AuditUnavailable,
    #[error("L2 source failed: {0}")]
    Source(ontoforge_engine::EngineError),
}

impl L2Error {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Denied => "L2_DENIED",
            Self::InvalidSql => "L2_INVALID_SQL",
            Self::Disabled => "L2_DISABLED",
            Self::AuditUnavailable => "AUDIT_UNAVAILABLE",
            Self::Source(_) => "SOURCE_ERROR",
        }
    }
}

#[derive(Debug)]
pub struct L2Result {
    pub output: QueryOutput,
    pub narrowing_hint: Option<&'static str>,
}

impl L2Service<'_> {
    pub async fn execute_query(&self, _sql: &str) -> Result<L2Result, L2Error> {
        Err(L2Error::InvalidSql)
    }
}
