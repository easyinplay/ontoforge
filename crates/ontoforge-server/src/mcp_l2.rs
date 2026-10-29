//! Generic MCP adapters for the six `sql:raw` L2 tools.

use std::sync::Arc;

use ontoforge_auth::Principal;
use ontoforge_auth::scope::PrincipalScope;
use ontoforge_engine::printer::l2::{self, ExplainType, L2Policy, L2TableNameError, McpDefaults};
use ontoforge_engine::{Engine, EngineError, QueryContext, QueryOutput, TableRef, Value};
use ontoforge_schema::ir::SqlLevel;
use ontoforge_storage::read_audit::{ReadAuditMetadata, ReadAuditStatus};
use ontoforge_storage::store::{ActiveDefinition, EditStore};
use serde_json::{Value as JsonValue, json};

use crate::ServerLimits;
use crate::state;

#[derive(Debug, thiserror::Error)]
pub enum McpL2Error {
    #[error("L2 access denied")]
    Denied,
    #[error("L2 unavailable")]
    Unavailable,
    #[error("invalid L2 request: {0}")]
    Invalid(String),
    #[error("L2 source failed: {0}")]
    Source(String),
    #[error("read audit unavailable")]
    AuditUnavailable,
}

pub struct McpL2Tools<'a, E> {
    pub principal: &'a Principal,
    pub scope: &'a PrincipalScope,
    pub active: Arc<ActiveDefinition>,
    pub engine: &'a E,
    pub store: &'a EditStore,
    pub limits: ServerLimits,
}

impl<'a, E: Engine + Sync> McpL2Tools<'a, E> {
    async fn gate(&self) -> Result<L2Policy, McpL2Error> {
        if !self
            .scope
            .toolsets
            .contains(&ontoforge_schema::ir::Toolset::SqlRaw)
            || !self.scope.sql_levels.contains(&SqlLevel::Raw)
        {
            return Err(McpL2Error::Denied);
        }
        if matches!(
            self.engine.status().await,
            ontoforge_engine::EngineStatus::Unavailable { .. }
        ) {
            return Err(McpL2Error::Unavailable);
        }
        Ok(L2Policy {
            raw_allowed: true,
            catalogs: self.scope.l2_catalogs.clone(),
            schemas: self.scope.l2_schemas.clone(),
            edit_catalog: "editstore".to_owned(),
            max_rows: self.limits.execution.max_raw_candidates as u64,
        })
    }

    async fn run_sql(
        &self,
        tool: &str,
        sql: ontoforge_engine::PrintedSql,
    ) -> Result<JsonValue, McpL2Error> {
        let mut audit = self
            .store
            .read_audit()
            .reserve(ReadAuditMetadata {
                principal: self.principal.id.to_string(),
                tool: tool.to_owned(),
                object_type: None,
                condition_api_names: vec![],
                effective_roles: self.scope.roles.iter().map(ToString::to_string).collect(),
                def_version: self.active.def_version,
            })
            .map_err(|_| McpL2Error::AuditUnavailable)?;
        audit.observe_sql(sql.as_str());
        let ctx = QueryContext {
            trace_token: audit.request_id().to_owned(),
            client_tags: vec!["mcp".to_owned(), tool.to_owned()],
            timeout: self.limits.deadline,
            max_rows: self.limits.execution.max_raw_candidates,
            max_bytes: self.limits.execution.max_decoded_bytes,
        };
        let result = self.engine.execute(&sql, &ctx).await;
        let (status, rows) = match &result {
            Ok(output) if output.truncated => (ReadAuditStatus::Truncated, output.rows.len()),
            Ok(output) => (ReadAuditStatus::Success, output.rows.len()),
            Err(EngineError::Cancelled) => (ReadAuditStatus::Cancelled, 0),
            Err(_) => (ReadAuditStatus::SourceError, 0),
        };
        audit
            .finish(status, rows)
            .map_err(|_| McpL2Error::AuditUnavailable)?;
        let output = result.map_err(|error| McpL2Error::Source(error.to_string()))?;
        Ok(output_json(output))
    }

    pub async fn execute_query(&self, query: &str) -> Result<JsonValue, McpL2Error> {
        let policy = self.gate().await?;
        let sql = l2::validate_l2(query, &policy)
            .map_err(|error| McpL2Error::Invalid(error.to_string()))?;
        self.run_sql("execute_query", sql).await
    }

    pub async fn explain_query(
        &self,
        query: &str,
        format: Option<&str>,
    ) -> Result<JsonValue, McpL2Error> {
        let policy = self.gate().await?;
        let explain_type = parse_explain_format(format)?;
        let sql = l2::validate_l2(query, &policy)
            .map_err(|error| McpL2Error::Invalid(error.to_string()))?;
        self.run_sql(
            "explain_query",
            l2::explain_l2_with_type(&sql, explain_type),
        )
        .await
    }

    pub async fn list_catalogs(&self) -> Result<JsonValue, McpL2Error> {
        let _ = self.gate().await?;
        self.run_sql("list_catalogs", l2::list_l2_catalogs())
            .await
            .map(|value| filter_names(value, |name| self.allowed_catalog(name)))
    }

    pub async fn list_schemas(&self, catalog: Option<&str>) -> Result<JsonValue, McpL2Error> {
        let _ = self.gate().await?;
        let catalog = self.resolve_catalog(catalog)?;
        let sql = l2::list_l2_schemas(&catalog)
            .map_err(|error| McpL2Error::Invalid(error.to_string()))?;
        self.run_sql("list_schemas", sql)
            .await
            .map(|value| filter_names(value, |name| self.allowed_schema(&catalog, name)))
    }

    pub async fn list_tables(
        &self,
        catalog: Option<&str>,
        schema: Option<&str>,
    ) -> Result<JsonValue, McpL2Error> {
        let _ = self.gate().await?;
        let (catalog, schema) = self.resolve_schema(catalog, schema)?;
        let sql = l2::list_l2_tables(&catalog, &schema)
            .map_err(|error| McpL2Error::Invalid(error.to_string()))?;
        self.run_sql("list_tables", sql)
            .await
            .map(|value| filter_names(value, |name| !name.contains('$')))
    }

    pub async fn get_table_schema(
        &self,
        table: &str,
        catalog: Option<&str>,
        schema: Option<&str>,
    ) -> Result<JsonValue, McpL2Error> {
        let _ = self.gate().await?;
        let target = self.resolve_table(table, catalog, schema)?;
        let audit = self
            .store
            .read_audit()
            .reserve(ReadAuditMetadata {
                principal: self.principal.id.to_string(),
                tool: "get_table_schema".to_owned(),
                object_type: None,
                condition_api_names: vec![],
                effective_roles: self.scope.roles.iter().map(ToString::to_string).collect(),
                def_version: self.active.def_version,
            })
            .map_err(|_| McpL2Error::AuditUnavailable)?;
        let ctx = QueryContext {
            trace_token: audit.request_id().to_owned(),
            client_tags: vec!["mcp".to_owned(), "get_table_schema".to_owned()],
            timeout: self.limits.deadline,
            max_rows: self.limits.execution.max_raw_candidates,
            max_bytes: self.limits.execution.max_decoded_bytes,
        };
        let result = self
            .engine
            .describe_tables(std::slice::from_ref(&target), &ctx)
            .await;
        let (status, rows) = match &result {
            Ok(columns) => (
                ReadAuditStatus::Success,
                columns.get(&target).map_or(0, Vec::len),
            ),
            Err(_) => (ReadAuditStatus::SourceError, 0),
        };
        audit
            .finish(status, rows)
            .map_err(|_| McpL2Error::AuditUnavailable)?;
        let columns = result
            .map_err(|error| McpL2Error::Source(error.to_string()))?
            .remove(&target)
            .unwrap_or_default();
        Ok(
            json!({"columns": columns.into_iter().map(|column| json!({"name": column.name, "type": column.physical.render(), "nullable": column.nullable})).collect::<Vec<_>>(), "truncated": false}),
        )
    }

    fn allowed_catalog(&self, name: &str) -> bool {
        !name.eq_ignore_ascii_case("system")
            && !name.eq_ignore_ascii_case("editstore")
            && (self
                .scope
                .l2_catalogs
                .iter()
                .any(|value| value.eq_ignore_ascii_case(name))
                || self.scope.l2_schemas.iter().any(|value| {
                    value
                        .split_once('.')
                        .is_some_and(|(catalog, _)| catalog.eq_ignore_ascii_case(name))
                }))
    }

    fn allowed_schema(&self, catalog: &str, schema: &str) -> bool {
        self.allowed_catalog(catalog)
            && !schema.eq_ignore_ascii_case("system")
            && !schema.eq_ignore_ascii_case("information_schema")
            && (self
                .scope
                .l2_catalogs
                .iter()
                .any(|value| value.eq_ignore_ascii_case(catalog))
                || self
                    .scope
                    .l2_schemas
                    .iter()
                    .any(|value| value.eq_ignore_ascii_case(&format!("{catalog}.{schema}"))))
    }

    fn resolve_catalog(&self, catalog: Option<&str>) -> Result<String, McpL2Error> {
        let defaults = self.defaults();
        let value = catalog
            .map(str::to_owned)
            .or(defaults.catalog)
            .ok_or_else(|| missing_context("catalog"))?;
        let value = resolve_component(&value)?;
        self.allowed_catalog(&value)
            .then_some(value)
            .ok_or(McpL2Error::Denied)
    }

    fn resolve_schema(
        &self,
        catalog: Option<&str>,
        schema: Option<&str>,
    ) -> Result<(String, String), McpL2Error> {
        let catalog = self.resolve_catalog(catalog)?;
        let value = schema
            .map(str::to_owned)
            .or_else(|| self.defaults().schema)
            .ok_or_else(|| missing_context("schema"))?;
        let value = resolve_component(&value)?;
        self.allowed_schema(&catalog, &value)
            .then_some((catalog, value))
            .ok_or(McpL2Error::Denied)
    }

    fn defaults(&self) -> McpDefaults {
        state::for_store(self.store).mcp_defaults()
    }

    fn resolve_table(
        &self,
        table: &str,
        catalog: Option<&str>,
        schema: Option<&str>,
    ) -> Result<TableRef, McpL2Error> {
        let trusted = self.defaults();
        let defaults = McpDefaults {
            catalog: catalog.map(str::to_owned).or(trusted.catalog),
            schema: schema.map(str::to_owned).or(trusted.schema),
        };
        let target = l2::resolve_l2_table_name(table, &defaults).map_err(table_name_error)?;
        if target.table.is_empty() || target.table.contains('$') {
            return Err(McpL2Error::Invalid("invalid table identifier".to_owned()));
        }
        self.allowed_schema(&target.catalog, &target.schema)
            .then_some(target)
            .ok_or(McpL2Error::Denied)
    }
}

fn missing_context(component: &str) -> McpL2Error {
    McpL2Error::Invalid(format!(
        "trusted L2 qualification context is missing: {component}"
    ))
}

fn table_name_error(error: L2TableNameError) -> McpL2Error {
    match error {
        L2TableNameError::MissingContext => missing_context("catalog or schema"),
        L2TableNameError::Unsupported => {
            McpL2Error::Invalid("invalid L2 table identifier".to_owned())
        }
    }
}

fn resolve_component(value: &str) -> Result<String, McpL2Error> {
    let defaults = McpDefaults {
        catalog: Some("__catalog__".to_owned()),
        schema: Some("__schema__".to_owned()),
    };
    let target = l2::resolve_l2_table_name(
        &format!("\"__catalog__\".\"__schema__\".{value}"),
        &defaults,
    )
    .map_err(table_name_error)?;
    Ok(target.table)
}

pub fn parse_explain_format(format: Option<&str>) -> Result<ExplainType, McpL2Error> {
    match format.unwrap_or("DISTRIBUTED") {
        "LOGICAL" => Ok(ExplainType::Logical),
        "DISTRIBUTED" => Ok(ExplainType::Distributed),
        "VALIDATE" => Ok(ExplainType::Validate),
        "IO" => Ok(ExplainType::Io),
        other => Err(McpL2Error::Invalid(format!(
            "unsupported explain format: {other}"
        ))),
    }
}

fn filter_names(value: JsonValue, allowed: impl Fn(&str) -> bool) -> JsonValue {
    let mut names = value["rows"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|row| row.as_array()?.first()?.as_str())
        .filter(|name| allowed(name))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    json!({"names": names, "truncated": value["truncated"].as_bool().unwrap_or(false)})
}

fn output_json(output: QueryOutput) -> JsonValue {
    json!({"columns": output.columns.into_iter().map(|column| json!({"name": column.name, "type": column.physical.render()})).collect::<Vec<_>>(), "rows": output.rows.into_iter().map(|row| row.into_iter().map(value_json).collect::<Vec<_>>()).collect::<Vec<_>>(), "truncated": output.truncated})
}

fn value_json(value: Value) -> JsonValue {
    match value {
        Value::Null => JsonValue::Null,
        Value::Boolean(value) => json!(value),
        Value::Long(value) => json!(value),
        Value::Double(value) => json!(value),
        Value::Decimal(value)
        | Value::Date(value)
        | Value::Timestamp(value)
        | Value::TimestampTz(value)
        | Value::String(value) => json!(value),
        Value::Array(values) => JsonValue::Array(values.into_iter().map(value_json).collect()),
        Value::Unsupported(value) => value,
    }
}
