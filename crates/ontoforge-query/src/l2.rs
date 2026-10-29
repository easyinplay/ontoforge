//! Library-level raw DQL operations for an attested Trino identity.

use std::sync::Arc;

use ontoforge_auth::Principal;
use ontoforge_auth::scope::PrincipalScope;
use ontoforge_engine::config::TrinoConfig;
use ontoforge_engine::printer::l2::{
    self, ExplainType, L2Error as SqlError, L2Policy, validate_l2,
};
use ontoforge_engine::trino::TrinoEngine;
use ontoforge_engine::trino::l2_health::L2HealthStatus;
use ontoforge_engine::{Column, Engine, EngineError, PhysicalType, QueryOutput, TableRef, Value};
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

#[derive(Clone, Copy)]
enum Discovery<'a> {
    Catalogs,
    Schemas(&'a str),
    Tables(&'a str, &'a str),
    TableSchema(&'a str, &'a str, &'a str),
}

#[derive(Clone, Copy)]
enum RunKind {
    Execute,
    Explain(ExplainType),
}

impl Discovery<'_> {
    fn tool(self) -> &'static str {
        match self {
            Self::Catalogs => "list_catalogs",
            Self::Schemas(_) => "list_schemas",
            Self::Tables(_, _) => "list_tables",
            Self::TableSchema(_, _, _) => "get_table_schema",
        }
    }
}

impl L2Service<'_> {
    pub async fn list_catalogs(&self) -> Result<L2Result, L2Error> {
        self.discover(Discovery::Catalogs).await
    }

    pub async fn list_schemas(&self, catalog: &str) -> Result<L2Result, L2Error> {
        self.discover(Discovery::Schemas(catalog)).await
    }

    pub async fn list_tables(&self, catalog: &str, schema: &str) -> Result<L2Result, L2Error> {
        self.discover(Discovery::Tables(catalog, schema)).await
    }

    pub async fn get_table_schema(
        &self,
        catalog: &str,
        schema: &str,
        table: &str,
    ) -> Result<L2Result, L2Error> {
        self.discover(Discovery::TableSchema(catalog, schema, table))
            .await
    }

    async fn discover(&self, request: Discovery<'_>) -> Result<L2Result, L2Error> {
        let mut audit = self
            .store
            .read_audit()
            .reserve(ReadAuditMetadata {
                principal: self.principal.id.to_string(),
                tool: request.tool().to_owned(),
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
        let result = self.discover_checked(request, &mut audit).await;
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
        request: Discovery<'_>,
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
        let allowed_catalog = |name: &str| self.allowed_catalog(name, edit);
        let allowed_schema = |name: &str, schema: &str| self.allowed_schema(name, schema, edit);
        let mut ctx = self
            .config
            .query_context(audit.request_id().to_owned(), vec![]);
        ctx.max_rows = ctx.max_rows.max(4096);
        ctx.max_bytes = ctx.max_bytes.max(1024 * 1024);
        match request {
            Discovery::Catalogs => {
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
                let output = self
                    .engine
                    .list_l2_catalogs(&ctx)
                    .await
                    .map_err(|e| self.discovery_error(e))?;
                self.filter_discovery(output, allowed_catalog)
            }
            Discovery::Schemas(name) => {
                if !allowed_catalog(name) {
                    return Err(L2Error::Denied);
                }
                let sql = l2::list_l2_schemas(name).map_err(|_| L2Error::Denied)?;
                audit.observe_sql(sql.as_str());
                let output = self
                    .engine
                    .list_l2_schemas(name, &ctx)
                    .await
                    .map_err(|e| self.discovery_error(e))?;
                self.filter_discovery(output, |schema| allowed_schema(name, schema))
            }
            Discovery::Tables(name, schema) => {
                if !allowed_schema(name, schema) {
                    return Err(L2Error::Denied);
                }
                let sql = l2::list_l2_tables(name, schema).map_err(|_| L2Error::Denied)?;
                audit.observe_sql(sql.as_str());
                let output = self
                    .engine
                    .list_l2_tables(name, schema, &ctx)
                    .await
                    .map_err(|e| self.discovery_error(e))?;
                self.filter_discovery(output, |table| !table.contains('$'))
            }
            Discovery::TableSchema(name, schema, table) => {
                if !allowed_schema(name, schema)
                    || table.is_empty()
                    || table.contains('$')
                    || table.contains('\0')
                {
                    return Err(L2Error::Denied);
                }
                let target = TableRef {
                    catalog: name.to_owned(),
                    schema: schema.to_owned(),
                    table: table.to_owned(),
                };
                let sql =
                    TrinoEngine::l2_table_description_sql(&target).map_err(L2Error::Source)?;
                audit.observe_sql(sql.as_str());
                let columns = self
                    .engine
                    .describe_tables(std::slice::from_ref(&target), &ctx)
                    .await
                    .map_err(|e| self.discovery_error(e))?;
                self.describe_result(columns.get(&target).map_or(&[], Vec::as_slice))
            }
        }
    }

    fn allowed_catalog(&self, name: &str, edit: &str) -> bool {
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
    }

    fn allowed_schema(&self, name: &str, schema: &str, edit: &str) -> bool {
        self.allowed_catalog(name, edit)
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
    }

    fn discovery_error(&self, error: EngineError) -> L2Error {
        if self.engine.l2_status() == Some(L2HealthStatus::Enabled) {
            L2Error::Source(error)
        } else {
            L2Error::Disabled
        }
    }

    fn filter_discovery(
        &self,
        output: QueryOutput,
        allowed: impl Fn(&str) -> bool,
    ) -> Result<L2Result, L2Error> {
        let mut names = Vec::new();
        for row in output.rows {
            let [Value::String(name)] = row.as_slice() else {
                return Err(L2Error::Source(EngineError::Protocol(
                    "metadata row has unexpected shape".to_owned(),
                )));
            };
            if allowed(name) {
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

    fn describe_result(
        &self,
        columns: &[ontoforge_engine::PhysicalColumn],
    ) -> Result<L2Result, L2Error> {
        let mut rows = Vec::new();
        let mut bytes = 0usize;
        let mut truncated = false;
        for column in columns {
            let kind = column.physical.render();
            let next = serde_json::to_string(&[&column.name, &kind])
                .map_err(|e| L2Error::Source(EngineError::Protocol(e.to_string())))?
                .len()
                + 6;
            if rows.len() >= self.config.max_rows
                || bytes.saturating_add(next) > self.config.max_bytes
            {
                truncated = true;
                break;
            }
            bytes += next;
            rows.push(vec![
                Value::String(column.name.clone()),
                Value::String(kind),
                Value::Boolean(column.nullable),
            ]);
        }
        Ok(L2Result {
            output: QueryOutput {
                columns: vec![
                    Column {
                        name: "name".to_owned(),
                        physical: PhysicalType::Varchar { length: None },
                    },
                    Column {
                        name: "type".to_owned(),
                        physical: PhysicalType::Varchar { length: None },
                    },
                    Column {
                        name: "nullable".to_owned(),
                        physical: PhysicalType::Boolean,
                    },
                ],
                rows,
                truncated,
            },
            narrowing_hint: truncated
                .then_some("Refine the metadata request to narrow the result."),
        })
    }

    pub async fn execute_query(&self, sql: &str) -> Result<L2Result, L2Error> {
        self.run(sql, RunKind::Execute).await
    }

    pub async fn explain_query(&self, sql: &str) -> Result<L2Result, L2Error> {
        self.explain_query_with_type(sql, ExplainType::Distributed)
            .await
    }

    pub async fn explain_query_with_type(
        &self,
        sql: &str,
        explain_type: ExplainType,
    ) -> Result<L2Result, L2Error> {
        self.run(sql, RunKind::Explain(explain_type)).await
    }

    async fn run(&self, sql: &str, kind: RunKind) -> Result<L2Result, L2Error> {
        let mut audit = self
            .store
            .read_audit()
            .reserve(ReadAuditMetadata {
                principal: self.principal.id.to_string(),
                tool: if matches!(kind, RunKind::Explain(_)) {
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
        let result = self.run_checked(sql, kind, &mut audit).await;
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
        kind: RunKind,
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
        let printed = match kind {
            RunKind::Execute => printed,
            RunKind::Explain(explain_type) => {
                ontoforge_engine::printer::l2::explain_l2_with_type(&printed, explain_type)
            }
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
