//! MCP transport and handler entry points.

use serde_json::Map;
use std::sync::Arc;

use axum::http::request::Parts;
use ontoforge_action::{ActionItem, ActionLimits, ActionOutcome, ActionRequest, ActionService};
use ontoforge_api::dto as wire;
use ontoforge_engine::Engine;
use ontoforge_query::aggregate::{AggregateCapabilities, AggregateLimits};
use ontoforge_query::ast;
use ontoforge_query::compile::{CompileLimits, ValidatedQuery};
use ontoforge_query::{ObjectSet, QueryRequest, QueryService};
use ontoforge_schema::ir::{ObjectTypeName, PropertyName};
use ontoforge_storage::history::{CallerKind, HistoryRequest};
use ontoforge_storage::value::PropertyValue;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ErrorData, ListToolsResult,
    PaginatedRequestParams, ServerCapabilities, ServerConfig,
};
use rmcp::service::RequestContext;
use rmcp::{RoleServer, ServerHandler};
use serde_json::{Value, json};

use crate::context;
use crate::mcp_actions::ActionToolSchema;
use crate::mcp_catalog::{self, ToolCatalog};
use crate::{ServerHost, ServerLimits};

#[path = "mcp_l2.rs"]
pub mod mcp_l2;
#[path = "mcp_sessions.rs"]
pub mod mcp_sessions;

async fn context_failure(
    response: axum::response::Response,
    api_name: &str,
    fallback_id: &str,
) -> ErrorData {
    let request_id = response
        .headers()
        .get("x-request-id")
        .and_then(|id| id.to_str().ok())
        .filter(|id| ulid::Ulid::from_string(id).is_ok())
        .unwrap_or(fallback_id)
        .to_owned();
    let value = axum::body::to_bytes(response.into_body(), 4096)
        .await
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok());
    let code = match value
        .as_ref()
        .and_then(|value| value.get("code"))
        .and_then(Value::as_str)
    {
        Some("UNAUTHENTICATED") => "UNAUTHENTICATED",
        Some("INVALID_REQUEST") => "INVALID_REQUEST",
        Some("STORE_UNAVAILABLE") => "STORE_UNAVAILABLE",
        Some("SCOPE_UNAVAILABLE") => "SCOPE_UNAVAILABLE",
        _ => "INTERNAL_ERROR",
    };
    context_error(code, api_name, &request_id)
}

fn context_error(code: &'static str, api_name: &str, request_id: &str) -> ErrorData {
    ErrorData::invalid_request(
        code,
        Some(json!({"code":code,"apiName":api_name,"request_id":request_id})),
    )
}

#[derive(Clone)]
pub struct OntologyMcpHandler<E> {
    pub host: Arc<ServerHost<E>>,
    pub limits: ServerLimits,
    pub(crate) state: Arc<crate::state::ServerState>,
}

impl<E> OntologyMcpHandler<E> {
    #[must_use]
    pub fn new(host: Arc<ServerHost<E>>) -> Self {
        let state = crate::state::for_host(&host);
        Self {
            host,
            limits: ServerLimits::default(),
            state,
        }
    }
}

impl<E: Engine + Send + Sync + 'static> OntologyMcpHandler<E> {
    async fn request_context(
        &self,
        request: &RequestContext<RoleServer>,
        api_name: &str,
    ) -> Result<(context::RequestContext, String), ErrorData> {
        let fallback_id = request
            .extensions
            .get::<Parts>()
            .and_then(|parts| parts.extensions.get::<context::RequestTrace>())
            .map_or_else(
                || ulid::Ulid::generate().to_string(),
                context::RequestTrace::id,
            );
        let parts = request
            .extensions
            .get::<Parts>()
            .ok_or_else(|| context_error("UNAUTHENTICATED", api_name, &fallback_id))?;
        let (ctx, request_id) = match context::context_from_headers(
            &self.host,
            parts.headers.clone(),
            api_name,
        )
        .await
        {
            Ok(context) => context,
            Err(response) => return Err(context_failure(response, api_name, &fallback_id).await),
        };
        if let Some(session) = parts
            .headers
            .get("mcp-session-id")
            .and_then(|value| value.to_str().ok())
        {
            self.state
                .mcp_sessions
                .verify(session, ctx.principal.id.as_str())
                .map_err(|_| context_error("UNAUTHENTICATED", api_name, &request_id))?;
        }
        Ok((ctx, request_id))
    }

    fn catalog(ctx: &context::RequestContext) -> ToolCatalog {
        ToolCatalog::for_scope(&ctx.scope).with_actions(&ctx.scope, &ctx.active.ontology)
    }

    async fn call_l2(
        &self,
        name: &str,
        args: &Map<String, Value>,
        ctx: &context::RequestContext,
        trace: &context::RequestTrace,
    ) -> Result<Value, ErrorData> {
        let failure = |error: mcp_l2::McpL2Error| {
            crate::error::mcp(
                error.code(),
                name,
                error.request_id().unwrap_or(&trace.id()),
            )
        };
        let engine = crate::state::for_host(&self.host)
            .l2_engine()
            .filter(|engine| {
                engine.l2_status()
                    == Some(ontoforge_engine::trino::l2_health::L2HealthStatus::Enabled)
            })
            .ok_or_else(|| failure(mcp_l2::McpL2Error::Unavailable))?;
        let on_reserved = |id: &str| trace.reserved(id);
        let tools = mcp_l2::McpL2Tools {
            principal: &ctx.principal,
            scope: &ctx.scope,
            active: Arc::clone(&ctx.active),
            engine: &*engine,
            store: &self.host.store,
            limits: self.limits,
            on_reserved: Some(&on_reserved),
        };
        match name {
            mcp_catalog::EXECUTE_QUERY_TOOL => tools
                .execute_query(
                    args.get("query")
                        .and_then(Value::as_str)
                        .ok_or_else(|| crate::error::mcp("INVALID_REQUEST", name, &trace.id()))?,
                )
                .await
                .map_err(failure),
            mcp_catalog::EXPLAIN_QUERY_TOOL => tools
                .explain_query(
                    args.get("query")
                        .and_then(Value::as_str)
                        .ok_or_else(|| crate::error::mcp("INVALID_REQUEST", name, &trace.id()))?,
                    args.get("format").and_then(Value::as_str),
                )
                .await
                .map_err(failure),
            mcp_catalog::LIST_CATALOGS_TOOL => tools.list_catalogs().await.map_err(failure),
            mcp_catalog::LIST_SCHEMAS_TOOL => tools
                .list_schemas(args.get("catalog").and_then(Value::as_str))
                .await
                .map_err(failure),
            mcp_catalog::LIST_TABLES_TOOL => tools
                .list_tables(
                    args.get("catalog").and_then(Value::as_str),
                    args.get("schema").and_then(Value::as_str),
                )
                .await
                .map_err(failure),
            mcp_catalog::GET_TABLE_SCHEMA_TOOL => tools
                .get_table_schema(
                    args.get("table")
                        .and_then(Value::as_str)
                        .ok_or_else(|| crate::error::mcp("INVALID_REQUEST", name, &trace.id()))?,
                    args.get("catalog").and_then(Value::as_str),
                    args.get("schema").and_then(Value::as_str),
                )
                .await
                .map_err(failure),
            _ => Err(crate::error::mcp("NOT_FOUND", name, &trace.id())),
        }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "keeps MCP read tool dispatch on shared service paths"
    )]
    async fn call_read(
        &self,
        name: &str,
        args: &Map<String, Value>,
        ctx: &context::RequestContext,
        trace: &context::RequestTrace,
    ) -> Result<Value, ErrorData> {
        let failure = |code| crate::error::mcp(code, name, &trace.id());
        let engine = context::ScopedEngine {
            inner: &*self.host.engine,
            trace,
            limits: self.limits,
            drift: &ctx.drift,
            purpose: context::EnginePurpose::Query,
        };
        match name {
            mcp_catalog::DESCRIBE_TOOL => {
                let object_types = ctx
                    .active
                    .ontology
                    .object_types
                    .iter()
                    .filter(|(name, ty)| !ty.hidden && ctx.scope.types.contains_key(*name))
                    .map(|(name, ty)| {
                        let scope = &ctx.scope.types[name];
                        json!({
                            "apiName": name,
                            "displayName": ty.display_name,
                            "description": ty.description,
                            "primaryKey": ty.primary_key,
                            "properties": ty.properties.iter().filter(|(property, _)| scope.columns.contains_key(*property)).map(|(property, definition)| json!({"apiName": property, "type": format!("{:?}", definition.base_type), "required": definition.required, "displayName": definition.display_name, "description": definition.description})).collect::<Vec<_>>()
                        })
                    })
                    .collect::<Vec<_>>();
                let actions = ctx
                    .active
                    .ontology
                    .action_types
                    .values()
                    .filter(|action| ctx.scope.actions.contains(&action.api_name))
                    .map(|action| json!({"apiName": action.api_name, "displayName": action.display_name, "description": action.description}))
                    .collect::<Vec<_>>();
                let links = ctx.active.ontology.link_types.values()
                    .filter(|link| ctx.scope.types.contains_key(&link.from.object_type)
                        && ctx.scope.types.contains_key(&link.to.object_type))
                    .map(|link| json!({"apiName": link.api_name, "displayName": link.display_name,
                        "description": link.description, "from": link.from.object_type, "to": link.to.object_type}))
                    .collect::<Vec<_>>();
                let engine_status = match self.host.engine.status().await {
                    ontoforge_engine::EngineStatus::Available => "available",
                    ontoforge_engine::EngineStatus::Unavailable { .. } => "unavailable",
                };
                Ok(
                    json!({"apiName": ctx.active.ontology.api_name, "description": ctx.active.ontology.description, "defVersion": ctx.active.def_version, "objectTypes": object_types, "actions": actions, "links": links, "engineStatus": engine_status}),
                )
            }
            mcp_catalog::GET_OBJECT_TOOL => {
                let object_type = args
                    .get("objectType")
                    .and_then(Value::as_str)
                    .ok_or_else(|| failure("INVALID_REQUEST"))?;
                let object_type_name = ObjectTypeName::try_from(object_type.to_owned())
                    .map_err(|_| failure("INVALID_REQUEST"))?;
                let ty = ctx
                    .active
                    .ontology
                    .object_types
                    .get(&object_type_name)
                    .ok_or_else(|| failure("NOT_FOUND"))?;
                let pk = args
                    .get("pk")
                    .cloned()
                    .ok_or_else(|| failure("INVALID_REQUEST"))?;
                let select = ty.properties.keys().cloned().collect();
                let result = QueryService {
                    principal: &ctx.principal,
                    scope: &ctx.scope,
                    active: Arc::clone(&ctx.active),
                    engine: &engine,
                    store: &self.host.store,
                    limits: self.limits.execution,
                    tokens: &self.host.tokens,
                }
                .execute_audited_observed(
                    QueryRequest {
                        set: ObjectSet::Pk {
                            object_type: object_type_name,
                            pk,
                        },
                        select,
                    },
                    |id| trace.reserved(id),
                )
                .await
                .result
                .map_err(|error| failure(error.code()))?;
                if result.truncated {
                    return Err(failure("COST_EXCEEDED"));
                }
                Ok(json!(result.rows.into_iter().next()))
            }
            mcp_catalog::QUERY_TOOL
            | mcp_catalog::OBJECT_SET_TOOL
            | mcp_catalog::SEARCH_AROUND_TOOL => {
                let input: wire::QueryRequest = serde_json::from_value(
                    args.get("request")
                        .cloned()
                        .ok_or_else(|| failure("INVALID_REQUEST"))?,
                )
                .map_err(|_| failure("INVALID_REQUEST"))?;
                let set =
                    crate::rest::convert_set(input.set).map_err(|()| failure("INVALID_QUERY"))?;
                let select = input
                    .select
                    .into_iter()
                    .map(PropertyName::try_from)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| failure("INVALID_QUERY"))?;
                let order_by = input
                    .order_by
                    .into_iter()
                    .map(crate::rest::convert_order)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|()| failure("INVALID_QUERY"))?;
                let object_type = set.object_type().clone();
                let scope = ctx
                    .scope
                    .types
                    .get(&object_type)
                    .ok_or_else(|| failure("NOT_FOUND"))?;
                let validated = ValidatedQuery::new(
                    QueryRequest { set, select },
                    order_by,
                    ast::PageSpec {
                        first: input.page.first,
                        after: input.page.after,
                    },
                    &ctx.active.ontology,
                    scope,
                    CompileLimits::default(),
                )
                .map_err(|error| failure(error.code()))?;
                let result = QueryService {
                    principal: &ctx.principal,
                    scope: &ctx.scope,
                    active: Arc::clone(&ctx.active),
                    engine: &engine,
                    store: &self.host.store,
                    limits: self.limits.execution,
                    tokens: &self.host.tokens,
                }
                .execute_list_audited_observed(&validated, |id| trace.reserved(id))
                .await
                .result
                .map_err(|error| failure(error.code()))?;
                Ok(json!(result))
            }
            mcp_catalog::AGGREGATE_TOOL => {
                let input: wire::AggregateRequest = serde_json::from_value(
                    args.get("request")
                        .cloned()
                        .ok_or_else(|| failure("INVALID_REQUEST"))?,
                )
                .map_err(|_| failure("INVALID_REQUEST"))?;
                let set =
                    crate::rest::convert_set(input.set).map_err(|()| failure("INVALID_QUERY"))?;
                let group_by = input
                    .group_by
                    .into_iter()
                    .map(PropertyName::try_from)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| failure("INVALID_QUERY"))?;
                let measures = input
                    .measures
                    .into_iter()
                    .map(crate::rest::convert_measure)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|()| failure("INVALID_QUERY"))?;
                let result = QueryService {
                    principal: &ctx.principal,
                    scope: &ctx.scope,
                    active: Arc::clone(&ctx.active),
                    engine: &engine,
                    store: &self.host.store,
                    limits: self.limits.execution,
                    tokens: &self.host.tokens,
                }
                .execute_aggregate_audited_observed(
                    ast::AggregateRequest {
                        set,
                        group_by,
                        measures,
                    },
                    AggregateCapabilities::all_supported(),
                    AggregateLimits::phase4_default(),
                    |id| trace.reserved(id),
                )
                .await
                .result
                .map_err(|error| failure(crate::error::aggregate_code(&error)))?;
                Ok(json!(result))
            }
            mcp_catalog::HISTORY_TOOL => {
                let object_type = args
                    .get("objectType")
                    .and_then(Value::as_str)
                    .ok_or_else(|| failure("INVALID_REQUEST"))?;
                let object_type_name = ObjectTypeName::try_from(object_type.to_owned())
                    .map_err(|_| failure("INVALID_REQUEST"))?;
                let ty = ctx
                    .active
                    .ontology
                    .object_types
                    .get(&object_type_name)
                    .ok_or_else(|| failure("NOT_FOUND"))?;
                let pk_json = args
                    .get("pk")
                    .cloned()
                    .ok_or_else(|| failure("INVALID_REQUEST"))?;
                let pk_def = ty
                    .properties
                    .get(&ty.primary_key)
                    .ok_or_else(|| failure("INVALID_REQUEST"))?;
                let pk =
                    PropertyValue::from_json(pk_def, &pk_json).map_err(|_| failure("NOT_FOUND"))?;
                let type_scope = ctx
                    .scope
                    .types
                    .get(&object_type_name)
                    .ok_or_else(|| failure("NOT_FOUND"))?;
                let page = QueryService {
                    principal: &ctx.principal,
                    scope: &ctx.scope,
                    active: Arc::clone(&ctx.active),
                    engine: &engine,
                    store: &self.host.store,
                    limits: self.limits.execution,
                    tokens: &self.host.tokens,
                }
                .execute_audited_observed(
                    QueryRequest {
                        set: ObjectSet::Pk {
                            object_type: object_type_name.clone(),
                            pk: pk_json,
                        },
                        select: ty.properties.keys().cloned().collect(),
                    },
                    |id| trace.reserved(id),
                )
                .await
                .result
                .map_err(|error| failure(error.code()))?;
                if page.truncated {
                    return Err(failure("COST_EXCEEDED"));
                }
                let row = page
                    .rows
                    .into_iter()
                    .next()
                    .ok_or_else(|| failure("NOT_FOUND"))?;
                let current = row
                    .properties
                    .iter()
                    .map(|(name, value)| (name.clone(), ontoforge_query::scope_scalar(value)))
                    .collect::<std::collections::BTreeMap<_, _>>();
                let history = crate::history_page::read_history_page(
                    &self.host.store,
                    HistoryRequest {
                        type_name: &object_type_name,
                        pk: &pk,
                        scope: Some(type_scope),
                        attrs: &ctx.principal.attrs,
                        current_row: Some(&current),
                        caller: CallerKind::Ordinary,
                        limit: u32::try_from(
                            args.get("limit").and_then(Value::as_u64).unwrap_or(100),
                        )
                        .unwrap_or(u32::MAX),
                        before_seq: args.get("beforeSeq").and_then(Value::as_i64),
                    },
                    &ctx.active,
                    self.limits.execution.max_decoded_bytes,
                )
                .await
                .map_err(|error| failure(error.code()))?;
                Ok(json!(history))
            }
            _ => Err(failure("NOT_FOUND")),
        }
    }
}

impl<E: Engine + Send + Sync + 'static> ServerHandler for OntologyMcpHandler<E> {
    fn get_info(&self) -> ServerConfig {
        let mut info = ServerConfig::default();
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info
    }

    async fn list_tools(
        &self,
        request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let (ctx, _) = self.request_context(&context, "tools/list").await?;
        let catalog = Self::catalog(&ctx);
        let page = catalog.page(
            request.as_ref().and_then(|value| value.cursor.as_deref()),
            100,
        );
        let mut result = ListToolsResult::with_all_items(page.tools);
        result.next_cursor = page.next;
        Ok(result)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "keeps authenticated dispatch and owned Action admission at one boundary"
    )]
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let server_state = Arc::clone(&self.state);
        let _request = server_state.read_tasks.token();
        let tool_name = request.name.to_string();
        let (ctx, request_id) = self.request_context(&context, &tool_name).await?;
        let trace = context
            .extensions
            .get::<Parts>()
            .and_then(|parts| parts.extensions.get::<context::RequestTrace>())
            .cloned()
            .unwrap_or_else(context::RequestTrace::new);
        trace.reserved(&request_id);
        if !server_state.accepting() {
            return Err(crate::error::mcp(
                "SERVER_SHUTTING_DOWN",
                &tool_name,
                &trace.id(),
            ));
        }
        let catalog = Self::catalog(&ctx);
        let Some(_tool) = catalog.all().iter().find(|tool| tool.name == tool_name) else {
            return Err(crate::error::mcp("NOT_FOUND", &tool_name, &trace.id()));
        };
        let args = request.arguments.unwrap_or_default();
        if matches!(
            tool_name.as_str(),
            mcp_catalog::EXECUTE_QUERY_TOOL
                | mcp_catalog::EXPLAIN_QUERY_TOOL
                | mcp_catalog::LIST_CATALOGS_TOOL
                | mcp_catalog::LIST_SCHEMAS_TOOL
                | mcp_catalog::LIST_TABLES_TOOL
                | mcp_catalog::GET_TABLE_SCHEMA_TOOL
        ) {
            let result = tokio::select! {
                biased;
                () = context.ct.cancelled() => return Err(crate::error::mcp("CANCELLED", &tool_name, &trace.id())),
                () = server_state.read_cancellation.cancelled() => return Err(crate::error::mcp("SERVER_SHUTTING_DOWN", &tool_name, &trace.id())),
                result = self.call_l2(&tool_name, &args, &ctx, &trace) => result,
            }?;
            return Ok(CallToolResult::structured(result).into());
        }
        if tool_name.starts_with("action_") {
            let action_name = tool_name.trim_start_matches("action_");
            let action = ctx
                .active
                .ontology
                .action_types
                .values()
                .find(|action| action.api_name.as_str() == action_name)
                .ok_or_else(|| crate::error::mcp("NOT_FOUND", &tool_name, &trace.id()))?;
            let schema = ActionToolSchema::from_action(action, &ctx.active.ontology);
            schema
                .validate_arguments(&args)
                .map_err(|_| crate::error::mcp("INVALID_PARAMETERS", &tool_name, &trace.id()))?;
            let validate_only = args
                .get("validateOnly")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let command_id = args
                .get("commandId")
                .and_then(Value::as_str)
                .map(str::to_owned);
            let params = args
                .get("params")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            let action_name = action_name.to_owned();
            let host = Arc::clone(&self.host);
            let limits = self.limits;
            let owned_permit = context
                .extensions
                .get::<Parts>()
                .and_then(|parts| parts.extensions.get::<context::AdmissionPermit>())
                .and_then(|permit| permit.0.lock().unwrap().take());
            let (sender, receiver) = tokio::sync::oneshot::channel();
            let action_tool = tool_name.clone();
            let action_trace = trace.clone();
            context::spawn_action(&self.host, async move {
                let _permit = owned_permit;
                let engine = context::ScopedEngine {
                    inner: &*host.engine,
                    trace: &action_trace,
                    limits,
                    drift: &ctx.drift,
                    purpose: context::EnginePurpose::Action,
                };
                let result = async {
                    ActionService::new(
                        &host.store,
                        ActionLimits::new(256, 256, limits.max_body_bytes).map_err(|_| {
                            crate::error::mcp(
                                "ACTION_UNAVAILABLE",
                                &action_tool,
                                &action_trace.id(),
                            )
                        })?,
                        &host.tokens,
                    )
                    .with_engine(&engine)
                    .execute_with_snapshot(
                        &ctx.principal,
                        &ctx.scope,
                        &ctx.active,
                        ActionRequest {
                            action: action_name,
                            command_id,
                            items: vec![ActionItem {
                                params,
                                expected_versions: std::collections::BTreeMap::default(),
                            }],
                            validate_only,
                        },
                    )
                    .await
                    .map_err(|error| {
                        ErrorData::invalid_params(
                            crate::error::action_code(&error),
                            Some(json!(crate::error::action_envelope(
                                &error,
                                &action_tool,
                                &action_trace.id()
                            ))),
                        )
                    })
                }
                .await;
                let _ = sender.send(result);
            })
            .map_err(|()| crate::error::mcp("SERVER_SHUTTING_DOWN", &tool_name, &trace.id()))?;
            let result = receiver
                .await
                .map_err(|_| crate::error::mcp("ACTION_FAILED", &tool_name, &trace.id()))??;
            let value = match result {
                ActionOutcome::Committed(receipt) => {
                    json!({"receipt": crate::rest::receipt_json(&receipt)})
                }
                ActionOutcome::Validated => json!({"validated": true}),
            };
            return Ok(CallToolResult::structured(value).into());
        }
        let result = tokio::select! {
            biased;
            () = context.ct.cancelled() => return Err(crate::error::mcp("CANCELLED", &tool_name, &trace.id())),
            () = server_state.read_cancellation.cancelled() => return Err(crate::error::mcp("SERVER_SHUTTING_DOWN", &tool_name, &trace.id())),
            result = self.call_read(&tool_name, &args, &ctx, &trace) => result,
        }?;
        Ok(CallToolResult::structured(result).into())
    }
}

#[cfg(test)]
mod error_tests {
    use super::*;

    #[tokio::test]
    async fn context_error_preserves_safe_code_and_existing_id_only() {
        for code in [
            "UNAUTHENTICATED",
            "INVALID_REQUEST",
            "STORE_UNAVAILABLE",
            "SCOPE_UNAVAILABLE",
        ] {
            let id = ulid::Ulid::generate().to_string();
            let response = crate::error::response(
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                &id,
                json!({"code":code,"request_id":"untrusted-body-id","details":"physical_relation secret_token"}),
            );
            let error = context_failure(response, "history", "fallback").await;
            assert_eq!(error.code, rmcp::model::ErrorCode::INVALID_REQUEST);
            assert_eq!(
                error.data.unwrap(),
                json!({"code":code,"apiName":"history","request_id":id})
            );
        }
    }

    #[tokio::test]
    async fn context_error_bounds_and_filters_body_without_losing_header_id() {
        for body in [
            "not json".to_owned(),
            "x".repeat(4097),
            "{\"code\":\"physical_relation secret_token\"}".to_owned(),
        ] {
            let id = ulid::Ulid::generate().to_string();
            let response = axum::response::Response::builder()
                .header("x-request-id", &id)
                .body(axum::body::Body::from(body))
                .unwrap();
            let error = context_failure(response, "history", "fallback").await;
            assert_eq!(
                error.data.unwrap(),
                json!({"code":"INTERNAL_ERROR","apiName":"history","request_id":id})
            );
        }
    }

    #[tokio::test]
    async fn context_error_without_valid_header_id_uses_transport_id() {
        let id = ulid::Ulid::generate().to_string();
        for header in [None, Some("invalid-id")] {
            let mut response = axum::response::Response::builder();
            if let Some(header) = header {
                response = response.header("x-request-id", header);
            }
            let response = response
                .body(axum::body::Body::from("{}".to_owned()))
                .unwrap();
            let error = context_failure(response, "describe", &id).await;
            assert_eq!(error.data.unwrap()["request_id"], id);
        }
    }
}
