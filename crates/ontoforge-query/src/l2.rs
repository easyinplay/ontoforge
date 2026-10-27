//! Library-level raw DQL operations for an attested Trino identity.

use std::sync::Arc;

use ontoforge_auth::Principal;
use ontoforge_auth::scope::PrincipalScope;
use ontoforge_engine::config::TrinoConfig;
use ontoforge_engine::printer::l2::{L2Error as SqlError, L2Policy, validate_l2};
use ontoforge_engine::trino::TrinoEngine;
use ontoforge_engine::trino::l2_health::L2HealthStatus;
use ontoforge_engine::{Engine, EngineError, QueryOutput};
use ontoforge_schema::ir::SqlLevel;
use ontoforge_storage::read_audit::{ReadAuditGuard, ReadAuditMetadata, ReadAuditStatus};
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
    pub async fn list_catalogs(&self) -> Result<L2Result, L2Error> {
        Err(L2Error::InvalidSql)
    }

    pub async fn list_schemas(&self, _catalog: &str) -> Result<L2Result, L2Error> {
        Err(L2Error::InvalidSql)
    }

    pub async fn execute_query(&self, sql: &str) -> Result<L2Result, L2Error> {
        self.run(sql, false).await
    }

    pub async fn explain_query(&self, sql: &str) -> Result<L2Result, L2Error> {
        self.run(sql, true).await
    }

    async fn run(&self, sql: &str, explain: bool) -> Result<L2Result, L2Error> {
        let mut audit = self
            .store
            .read_audit()
            .reserve(ReadAuditMetadata {
                principal: self.principal.id.to_string(),
                tool: if explain {
                    "explain_query"
                } else {
                    "execute_query"
                }
                .to_owned(),
                object_type: None,
                condition_api_names: vec![],
                effective_roles: self
                    .scope
                    .roles
                    .iter()
                    .map(|r| r.as_str().to_owned())
                    .collect(),
                def_version: self.active.def_version,
            })
            .map_err(|_| L2Error::AuditUnavailable)?;
        audit.observe_sql(sql);
        let result = self.run_checked(sql, explain, &mut audit).await;
        let (status, rows) = match &result {
            Ok(page) if page.output.truncated => {
                (ReadAuditStatus::Truncated, page.output.rows.len())
            }
            Ok(page) => (ReadAuditStatus::Success, page.output.rows.len()),
            Err(L2Error::Source(EngineError::Cancelled)) => (ReadAuditStatus::Cancelled, 0),
            Err(L2Error::Source(_)) => (ReadAuditStatus::SourceError, 0),
            Err(_) => (ReadAuditStatus::Denied, 0),
        };
        audit
            .finish(status, rows)
            .map_err(|_| L2Error::AuditUnavailable)?;
        result
    }

    async fn run_checked(
        &self,
        sql: &str,
        explain: bool,
        audit: &mut ReadAuditGuard,
    ) -> Result<L2Result, L2Error> {
        if self.engine.l2_status() != Some(L2HealthStatus::Enabled) {
            return Err(L2Error::Disabled);
        }
        let Some(acl) = &self.config.l2_acl else {
            return Err(L2Error::Disabled);
        };
        let policy = L2Policy {
            raw_allowed: self.scope.sql_levels.contains(&SqlLevel::Raw),
            catalogs: self.scope.l2_catalogs.clone(),
            schemas: self.scope.l2_schemas.clone(),
            edit_catalog: acl.edit.catalog.clone(),
            max_rows: u64::try_from(self.config.max_rows).map_err(|_| L2Error::InvalidSql)?,
        };
        let printed = validate_l2(sql, &policy).map_err(|err| match err {
            SqlError::Denied => L2Error::Denied,
            SqlError::Unsupported => L2Error::InvalidSql,
        })?;
        let printed = if explain {
            ontoforge_engine::printer::l2::explain_l2(&printed)
        } else {
            printed
        };
        audit.observe_sql(printed.as_str());
        let ctx = self
            .config
            .query_context(audit.request_id().to_owned(), vec![]);
        let output = self.engine.execute(&printed, &ctx).await.map_err(|error| {
            if self.engine.l2_status() == Some(L2HealthStatus::Enabled) {
                L2Error::Source(error)
            } else {
                L2Error::Disabled
            }
        })?;
        let narrowing_hint = output
            .truncated
            .then_some("Refine the SQL to narrow the result.");
        Ok(L2Result {
            output,
            narrowing_hint,
        })
    }
}
