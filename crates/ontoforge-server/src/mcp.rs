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

#[derive(Clone)]
pub struct OntologyMcpHandler<E> {
    pub host: Arc<ServerHost<E>>,
    pub limits: ServerLimits,
}

impl<E> OntologyMcpHandler<E> {
    #[must_use]
    pub fn new(host: Arc<ServerHost<E>>) -> Self {
        Self {
            host,
            limits: ServerLimits::default(),
        }
    }
}

impl<E: Engine + Send + Sync + 'static> OntologyMcpHandler<E> {
    async fn request_context(
        &self,
        request: &RequestContext<RoleServer>,
        api_name: &str,
    ) -> Result<(context::RequestContext, String), ErrorData> {
        let parts = request
            .extensions
            .get::<Parts>()
            .ok_or_else(|| ErrorData::invalid_request("authentication required", None))?;
        context::context_from_headers(&self.host, parts.headers.clone(), api_name)
            .await
            .map_err(|_| ErrorData::invalid_request("authentication required", None))
    }

    fn catalog(ctx: &context::RequestContext) -> ToolCatalog {
        ToolCatalog::for_scope(&ctx.scope)
            .with_actions(&ctx.scope, ctx.active.ontology.action_types.values())
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
    ) -> Result<Value, String> {
        let trace = context::RequestTrace::new();
        let object_type = args
            .get("objectType")
            .and_then(Value::as_str)
            .ok_or_else(|| "objectType is required".to_owned())?;
        let object_type_name = ObjectTypeName::try_from(object_type.to_owned())
            .map_err(|_| "invalid objectType".to_owned())?;
        let ty = ctx
            .active
            .ontology
            .object_types
            .get(&object_type_name)
            .ok_or_else(|| "object not found".to_owned())?;
        let engine = context::ScopedEngine {
            inner: &*self.host.engine,
            trace: &trace,
            limits: self.limits,
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
                Ok(
                    json!({"apiName": ctx.active.ontology.api_name, "description": ctx.active.ontology.description, "defVersion": ctx.active.def_version, "objectTypes": object_types, "actions": actions}),
                )
            }
            mcp_catalog::GET_OBJECT_TOOL => {
                let pk = args
                    .get("pk")
                    .cloned()
                    .ok_or_else(|| "pk is required".to_owned())?;
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
                .execute_audited(QueryRequest {
                    set: ObjectSet::Pk {
                        object_type: object_type_name,
                        pk,
                    },
                    select,
                })
                .await
                .result
                .map_err(|error| error.to_string())?;
                Ok(json!(result.rows.into_iter().next()))
            }
            mcp_catalog::QUERY_TOOL
            | mcp_catalog::OBJECT_SET_TOOL
            | mcp_catalog::SEARCH_AROUND_TOOL => {
                let input: wire::QueryRequest = serde_json::from_value(
                    args.get("request")
                        .cloned()
                        .ok_or_else(|| "request is required".to_owned())?,
                )
                .map_err(|_| "invalid request".to_owned())?;
                let set =
                    crate::rest::convert_set(input.set).map_err(|()| "invalid query".to_owned())?;
                let select = input
                    .select
                    .into_iter()
                    .map(PropertyName::try_from)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| "invalid query".to_owned())?;
                let order_by = input
                    .order_by
                    .into_iter()
                    .map(crate::rest::convert_order)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|()| "invalid query".to_owned())?;
                let object_type = set.object_type().clone();
                let scope = ctx
                    .scope
                    .types
                    .get(&object_type)
                    .ok_or_else(|| "object not found".to_owned())?;
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
                .map_err(|error| error.to_string())?;
                let result = QueryService {
                    principal: &ctx.principal,
                    scope: &ctx.scope,
                    active: Arc::clone(&ctx.active),
                    engine: &engine,
                    store: &self.host.store,
                    limits: self.limits.execution,
                    tokens: &self.host.tokens,
                }
                .execute_list_audited(&validated)
                .await
                .result
                .map_err(|error| error.to_string())?;
                Ok(json!(result))
            }
            mcp_catalog::AGGREGATE_TOOL => {
                let input: wire::AggregateRequest = serde_json::from_value(
                    args.get("request")
                        .cloned()
                        .ok_or_else(|| "request is required".to_owned())?,
                )
                .map_err(|_| "invalid request".to_owned())?;
                let set =
                    crate::rest::convert_set(input.set).map_err(|()| "invalid query".to_owned())?;
                let group_by = input
                    .group_by
                    .into_iter()
                    .map(PropertyName::try_from)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| "invalid query".to_owned())?;
                let measures = input
                    .measures
                    .into_iter()
                    .map(crate::rest::convert_measure)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|()| "invalid query".to_owned())?;
                let result = QueryService {
                    principal: &ctx.principal,
                    scope: &ctx.scope,
                    active: Arc::clone(&ctx.active),
                    engine: &engine,
                    store: &self.host.store,
                    limits: self.limits.execution,
                    tokens: &self.host.tokens,
                }
                .execute_aggregate(
                    ast::AggregateRequest {
                        set,
                        group_by,
                        measures,
                    },
                    AggregateCapabilities::all_supported(),
                    AggregateLimits::phase4_default(),
                )
                .await
                .map_err(|error| error.to_string())?;
                Ok(json!(result))
            }
            mcp_catalog::HISTORY_TOOL => {
                let pk_json = args
                    .get("pk")
                    .cloned()
                    .ok_or_else(|| "pk is required".to_owned())?;
                let pk_def = ty
                    .properties
                    .get(&ty.primary_key)
                    .ok_or_else(|| "invalid primary key".to_owned())?;
                let pk = PropertyValue::from_json(pk_def, &pk_json)
                    .map_err(|_| "object not found".to_owned())?;
                let type_scope = ctx
                    .scope
                    .types
                    .get(&object_type_name)
                    .ok_or_else(|| "object not found".to_owned())?;
                let row = QueryService {
                    principal: &ctx.principal,
                    scope: &ctx.scope,
                    active: Arc::clone(&ctx.active),
                    engine: &engine,
                    store: &self.host.store,
                    limits: self.limits.execution,
                    tokens: &self.host.tokens,
                }
                .execute(QueryRequest {
                    set: ObjectSet::Pk {
                        object_type: object_type_name.clone(),
                        pk: pk_json,
                    },
                    select: ty.properties.keys().cloned().collect(),
                })
                .await
                .map_err(|error| error.to_string())?
                .rows
                .into_iter()
                .next()
                .ok_or_else(|| "object not found".to_owned())?;
                let current = row
                    .properties
                    .iter()
                    .map(|(name, value)| (name.clone(), ontoforge_query::scope_scalar(value)))
                    .collect::<std::collections::BTreeMap<_, _>>();
                let history = self
                    .host
                    .store
                    .history_with_snapshot(
                        &HistoryRequest {
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
                    )
                    .await
                    .map_err(|error| error.to_string())?;
                Ok(
                    json!({"entries": history, "truncated": false, "nextBeforeSeq": history.last().map(|entry| entry.seq)}),
                )
            }
            _ => Err("tool unavailable".to_owned()),
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

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let tool_name = request.name.to_string();
        let (ctx, _) = self.request_context(&context, &tool_name).await?;
        let catalog = Self::catalog(&ctx);
        let Some(_tool) = catalog.all().iter().find(|tool| tool.name == tool_name) else {
            return Err(ErrorData::invalid_request("tool unavailable", None));
        };
        let args = request.arguments.unwrap_or_default();
        if tool_name.starts_with("action_") {
            let action_name = tool_name.trim_start_matches("action_");
            let action = ctx
                .active
                .ontology
                .action_types
                .values()
                .find(|action| action.api_name.as_str() == action_name)
                .ok_or_else(|| ErrorData::invalid_request("tool unavailable", None))?;
            let schema = ActionToolSchema::from_action(action);
            schema
                .validate_arguments(&args)
                .map_err(|error| ErrorData::invalid_params(error, None))?;
            let validate_only = args
                .get("validateOnly")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let command_id = args
                .get("commandId")
                .and_then(Value::as_str)
                .map(str::to_owned);
            let params = args
                .iter()
                .filter(|(key, _)| *key != "validateOnly" && *key != "commandId")
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect();
            let result = ActionService::new(
                &self.host.store,
                ActionLimits::new(256, 256, self.limits.max_body_bytes)
                    .map_err(|_| ErrorData::internal_error("action unavailable", None))?,
                &self.host.tokens,
            )
            .with_engine(&*self.host.engine)
            .execute_with_snapshot(
                &ctx.principal,
                &ctx.scope,
                &ctx.active,
                ActionRequest {
                    action: action_name.to_owned(),
                    command_id,
                    items: vec![ActionItem {
                        params,
                        expected_versions: std::collections::BTreeMap::default(),
                    }],
                    validate_only,
                },
            )
            .await
            .map_err(|error| ErrorData::invalid_params(error.to_string(), None))?;
            let value = match result {
                ActionOutcome::Committed(receipt) => json!({"receipt": receipt}),
                ActionOutcome::Validated => json!({"validated": true}),
            };
            return Ok(CallToolResult::structured(value).into());
        }
        let result = self
            .call_read(&tool_name, &args, &ctx)
            .await
            .map_err(|error| ErrorData::invalid_params(error, None))?;
        Ok(CallToolResult::structured(result).into())
    }
}
