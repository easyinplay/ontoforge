#![allow(dead_code)]

use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

use crate::dto;

#[derive(Default)]
struct SecurityModifier;

impl Modify for SecurityModifier {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearerAuth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .description(Some("Bearer token accepted by the authenticated REST host"))
                        .build(),
                ),
            );
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/ontology",
    responses(
        (status = 200, description = "Scoped ontology metadata", body = dto::Metadata),
        (status = 400, body = dto::ErrorEnvelope),
        (status = 401, body = dto::ErrorEnvelope),
        (status = 403, body = dto::ErrorEnvelope),
        (status = 503, body = dto::ErrorEnvelope)
    ),
    security(("bearerAuth" = []))
)]
fn ontology() {}

#[utoipa::path(
    get,
    path = "/api/objects/{object_type}/{pk}",
    params(
        ("object_type" = String, Path, description = "Logical object type API name"),
        ("pk" = String, Path, description = "JSON or string primary key")
    ),
    responses(
        (status = 200, body = dto::ObjectRow),
        (status = 401, body = dto::ErrorEnvelope),
        (status = 403, body = dto::ErrorEnvelope),
        (status = 404, body = dto::ErrorEnvelope),
        (status = 409, body = dto::ErrorEnvelope),
        (status = 503, body = dto::ErrorEnvelope)
    ),
    security(("bearerAuth" = []))
)]
fn object() {}

#[utoipa::path(
    post,
    path = "/api/query",
    request_body = dto::QueryRequest,
    responses(
        (status = 200, body = dto::ListPage),
        (status = 400, body = dto::ErrorEnvelope),
        (status = 401, body = dto::ErrorEnvelope),
        (status = 403, body = dto::ErrorEnvelope),
        (status = 404, body = dto::ErrorEnvelope),
        (status = 409, body = dto::ErrorEnvelope),
        (status = 503, body = dto::ErrorEnvelope)
    ),
    security(("bearerAuth" = []))
)]
fn query() {}

#[utoipa::path(
    post,
    path = "/api/aggregate",
    request_body = dto::AggregateRequest,
    responses(
        (status = 200, body = dto::AggregateResult),
        (status = 400, body = dto::ErrorEnvelope),
        (status = 401, body = dto::ErrorEnvelope),
        (status = 403, body = dto::ErrorEnvelope),
        (status = 404, body = dto::ErrorEnvelope),
        (status = 409, body = dto::ErrorEnvelope),
        (status = 503, body = dto::ErrorEnvelope)
    ),
    security(("bearerAuth" = []))
)]
fn aggregate() {}

#[utoipa::path(
    post,
    path = "/api/actions/{action}",
    params(("action" = String, Path, description = "Action API name")),
    request_body = dto::ActionRequest,
    responses(
        (status = 200, description = "Committed receipt or validation acknowledgement", content_type = "application/json", body = inline(serde_json::Value)),
        (status = 400, body = dto::ErrorEnvelope),
        (status = 401, body = dto::ErrorEnvelope),
        (status = 403, body = dto::ErrorEnvelope),
        (status = 404, body = dto::ErrorEnvelope),
        (status = 409, body = dto::ErrorEnvelope),
        (status = 503, body = dto::ErrorEnvelope)
    ),
    security(("bearerAuth" = []))
)]
fn action() {}

#[utoipa::path(
    get,
    path = "/api/commands/{command_id}",
    params(("command_id" = String, Path, description = "Durable Action command id")),
    responses(
        (status = 200, body = dto::ActionReceipt),
        (status = 401, body = dto::ErrorEnvelope),
        (status = 403, body = dto::ErrorEnvelope),
        (status = 404, body = dto::ErrorEnvelope),
        (status = 503, body = dto::ErrorEnvelope)
    ),
    security(("bearerAuth" = []))
)]
fn command_result() {}

#[utoipa::path(
    get,
    path = "/api/objects/{object_type}/{pk}/history",
    params(
        ("object_type" = String, Path, description = "Logical object type API name"),
        ("pk" = String, Path, description = "JSON or string primary key"),
        ("limit" = Option<u32>, Query, description = "Maximum history entries"),
        ("beforeSeq" = Option<i64>, Query, description = "Return entries before sequence")
    ),
    responses(
        (status = 200, body = dto::HistoryPage),
        (status = 401, body = dto::ErrorEnvelope),
        (status = 403, body = dto::ErrorEnvelope),
        (status = 404, body = dto::ErrorEnvelope),
        (status = 503, body = dto::ErrorEnvelope)
    ),
    security(("bearerAuth" = []))
)]
fn history() {}

#[utoipa::path(
    get,
    path = "/health",
    responses((status = 200, body = dto::HealthResponse), (status = 503, body = dto::HealthResponse))
)]
fn health() {}

#[derive(OpenApi)]
#[openapi(
    version = "3.1.0",
    info(
        title = "Ontoforge REST API",
        version = "0.1.0",
        description = "Stable platform wire contract for authenticated ontology reads and writes. Unicode descriptions are preserved in UTF-8."
    ),
    paths(ontology, object, query, aggregate, action, command_result, history, health),
    components(
        schemas(
            dto::ObjectSet,
            dto::LinkDirection,
            dto::TermMode,
            dto::WhereClause,
            dto::OrderBy,
            dto::Direction,
            dto::Nulls,
            dto::PageSpec,
            dto::QueryRequest,
            dto::AggregateRequest,
            dto::AggregateMeasure,
            dto::Metadata,
            dto::EngineAvailability,
            dto::ObjectMetadata,
            dto::PropertyMetadata,
            dto::LinkMetadata,
            dto::ActionMetadata,
            dto::ErrorEnvelope,
            dto::ActionRequest,
            dto::ActionItem,
            dto::ObjectRow,
            dto::ListPage,
            dto::AggregateBucket,
            dto::AggregateResult,
            dto::HistoryEntry,
            dto::HistoryPage,
            dto::ItemReceipt,
            dto::ActionReceipt,
            dto::ValidationResponse,
            dto::HealthResponse
        )
    ),
    modifiers(&SecurityModifier)
)]
struct ApiDoc;

/// Build the static OpenAPI document without opening a store, listener, or tenant state.
#[must_use]
pub fn document() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}

/// Render the static OpenAPI document with stable pretty JSON and a trailing newline.
#[must_use]
pub fn render() -> String {
    let mut value = serde_json::to_value(document()).expect("OpenAPI document is serializable");
    if let Some(object) = value.as_object_mut() {
        object.insert("openapi".to_owned(), serde_json::Value::String("3.1.0".to_owned()));
    }
    let mut text = serde_json::to_string_pretty(&value).expect("OpenAPI document is JSON");
    text.push('\n');
    text
}
