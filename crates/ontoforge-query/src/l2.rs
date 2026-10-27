//! Library-level raw DQL operations for an attested Trino identity.

use std::sync::Arc;

use ontoforge_auth::Principal;
use ontoforge_auth::scope::PrincipalScope;
use ontoforge_engine::config::TrinoConfig;
use ontoforge_engine::printer::l2::{self, L2Error as SqlError, L2Policy, validate_l2};
use ontoforge_engine::trino::TrinoEngine;
use ontoforge_engine::trino::l2_health::L2HealthStatus;
use ontoforge_engine::{Column, Engine, EngineError, PhysicalType, QueryOutput, Value};
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
        self.discover("list_catalogs", None).await
    }

    pub async fn list_schemas(&self, catalog: &str) -> Result<L2Result, L2Error> {
        self.discover("list_schemas", Some(catalog)).await
    }

    pub async fn list_tables(&self, _catalog: &str, _schema: &str) -> Result<L2Result, L2Error> {
        Err(L2Error::InvalidSql)
    }

    pub async fn get_table_schema(
        &self,
        _catalog: &str,
        _schema: &str,
        _table: &str,
    ) -> Result<L2Result, L2Error> {
        Err(L2Error::InvalidSql)
    }

    async fn discover(&self, tool: &str, catalog: Option<&str>) -> Result<L2Result, L2Error> {
        let mut audit = self
            .store
            .read_audit()
            .reserve(ReadAuditMetadata {
                principal: self.principal.id.to_string(),
                tool: tool.to_owned(),
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
        let result = self.discover_checked(catalog, &mut audit).await;
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

    async fn discover_checked(
        &self,
        catalog: Option<&str>,
        audit: &mut ReadAuditGuard,
    ) -> Result<L2Result, L2Error> {
        if self.engine.l2_status() != Some(L2HealthStatus::Enabled) || self.config.l2_acl.is_none()
        {
            return Err(L2Error::Disabled);
        }
        if !self.scope.sql_levels.contains(&SqlLevel::Raw) {
            return Err(L2Error::Denied);
        }
        let edit = &self
            .config
            .l2_acl
            .as_ref()
            .expect("checked ACL")
            .edit
            .catalog;
        let allowed_catalog = |name: &str| {
            !name.eq_ignore_ascii_case("system")
                && !name.eq_ignore_ascii_case(edit)
                && (self
                    .scope
                    .l2_catalogs
                    .iter()
                    .any(|c| c.eq_ignore_ascii_case(name))
                    || self.scope.l2_schemas.iter().any(|s| {
                        s.split_once('.')
                            .is_some_and(|(c, _)| c.eq_ignore_ascii_case(name))
                    }))
        };
        let allowed_schema = |name: &str, schema: &str| {
            allowed_catalog(name)
                && !schema.eq_ignore_ascii_case("system")
                && !schema.eq_ignore_ascii_case("information_schema")
                && (self
                    .scope
                    .l2_catalogs
                    .iter()
                    .any(|c| c.eq_ignore_ascii_case(name))
                    || self
                        .scope
                        .l2_schemas
                        .iter()
                        .any(|s| s.eq_ignore_ascii_case(&format!("{name}.{schema}"))))
        };
        let mut ctx = self
            .config
            .query_context(audit.request_id().to_owned(), vec![]);
        ctx.max_rows = ctx.max_rows.max(4096);
        ctx.max_bytes = ctx.max_bytes.max(1024 * 1024);
        let output = match catalog {
            None => {
                if !self.scope.l2_catalogs.iter().any(|c| allowed_catalog(c))
                    && !self
                        .scope
                        .l2_schemas
                        .iter()
                        .any(|s| s.split_once('.').is_some_and(|(c, _)| allowed_catalog(c)))
                {
                    return Err(L2Error::Denied);
                }
                let sql = l2::list_l2_catalogs();
                audit.observe_sql(sql.as_str());
                self.engine.list_l2_catalogs(&ctx).await
            }
            Some(name) => {
                if !allowed_catalog(name) {
                    return Err(L2Error::Denied);
                }
                let sql = l2::list_l2_schemas(name).map_err(|_| L2Error::Denied)?;
                audit.observe_sql(sql.as_str());
                self.engine.list_l2_schemas(name, &ctx).await
            }
        };
        let output = output.map_err(|e| {
            if self.engine.l2_status() == Some(L2HealthStatus::Enabled) {
                L2Error::Source(e)
            } else {
                L2Error::Disabled
            }
        })?;
        self.filter_discovery(output, catalog, allowed_catalog, allowed_schema)
    }

    fn filter_discovery(
        &self,
        output: QueryOutput,
        catalog: Option<&str>,
        allowed_catalog: impl Fn(&str) -> bool,
        allowed_schema: impl Fn(&str, &str) -> bool,
    ) -> Result<L2Result, L2Error> {
        let mut names = Vec::new();
        for row in output.rows {
            let [Value::String(name)] = row.as_slice() else {
                return Err(L2Error::Source(EngineError::Protocol(
                    "metadata row has unexpected shape".to_owned(),
                )));
            };
            if match catalog {
                Some(c) => allowed_schema(c, name),
                None => allowed_catalog(name),
            } {
                names.push(name.clone());
            }
        }
        names.sort_unstable();
        names.dedup();
        let mut rows = Vec::new();
        let mut bytes = 0;
        let mut truncated = output.truncated;
        for name in names {
            let next = serde_json::to_string(&name)
                .map_err(|e| L2Error::Source(EngineError::Protocol(e.to_string())))?
                .len()
                + 2;
            if rows.len() == self.config.max_rows || bytes + next > self.config.max_bytes {
                truncated = true;
                break;
            }
            bytes += next;
            rows.push(vec![Value::String(name)]);
        }
        Ok(L2Result {
            output: QueryOutput {
                columns: vec![Column {
                    name: "name".to_owned(),
                    physical: PhysicalType::Varchar { length: None },
                }],
                rows,
                truncated,
            },
            narrowing_hint: truncated
                .then_some("Refine the metadata request to narrow the result."),
        })
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
