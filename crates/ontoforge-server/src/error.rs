use axum::Json;
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use ontoforge_action::ActionError;
use ontoforge_query::{QueryError, aggregate::AggregateError};
use serde_json::{Value, json};

pub fn mcp(code: &'static str, api_name: &str, request_id: &str) -> rmcp::model::ErrorData {
    rmcp::model::ErrorData::invalid_params(
        code,
        Some(json!({"code": code, "apiName": api_name, "request_id": request_id})),
    )
}

pub fn wire(status: StatusCode, code: &str, api_name: &str, request_id: &str) -> Response {
    response(
        status,
        request_id,
        json!({"code": code, "apiName": api_name, "request_id": request_id}),
    )
}
pub fn response(status: StatusCode, request_id: &str, value: Value) -> Response {
    let mut response = (status, Json(value)).into_response();
    response.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(request_id).expect("server-generated ULID"),
    );
    response
}
pub fn query_status(error: &QueryError) -> StatusCode {
    match error {
        QueryError::NotFound => StatusCode::NOT_FOUND,
        QueryError::Source
        | QueryError::SourceTarget(_)
        | QueryError::Store
        | QueryError::AuditUnavailable => StatusCode::SERVICE_UNAVAILABLE,
        QueryError::AmbiguousObject => StatusCode::CONFLICT,
        _ => StatusCode::BAD_REQUEST,
    }
}
pub fn aggregate_status(error: &AggregateError) -> StatusCode {
    match error {
        AggregateError::Source
        | AggregateError::SourceTarget(_)
        | AggregateError::Store
        | AggregateError::AuditUnavailable => StatusCode::SERVICE_UNAVAILABLE,
        _ => StatusCode::BAD_REQUEST,
    }
}

pub fn aggregate_code(error: &AggregateError) -> &'static str {
    match error {
        AggregateError::InvalidQuery => "INVALID_QUERY",
        AggregateError::Unsupported => "UNSUPPORTED",
        AggregateError::AggregateCostExceeded => "COST_EXCEEDED",
        AggregateError::Source => "SOURCE_UNAVAILABLE",
        AggregateError::SourceTarget(error) => error.code(),
        AggregateError::Store => "STORE_UNAVAILABLE",
        AggregateError::AuditUnavailable => "AUDIT_UNAVAILABLE",
        AggregateError::UnsupportedNumericValue => "UNSUPPORTED_NUMERIC_VALUE",
    }
}

pub fn action_status(error: &ActionError) -> StatusCode {
    match error {
        ActionError::ScopeDenied | ActionError::NotFound | ActionError::UnknownAction(_) => {
            StatusCode::NOT_FOUND
        }
        ActionError::VersionConflict
        | ActionError::Commit(ontoforge_storage::commit::ActionCommitError::Edit(
            ontoforge_storage::commit::EditError::DefinitionStale { .. },
        )) => StatusCode::CONFLICT,
        ActionError::EngineUnavailable | ActionError::SourceTarget(_) | ActionError::Store(_) => {
            StatusCode::SERVICE_UNAVAILABLE
        }
        ActionError::Commit(ontoforge_storage::commit::ActionCommitError::CommandIdReused) => {
            StatusCode::CONFLICT
        }
        _ => StatusCode::BAD_REQUEST,
    }
}

pub fn action_envelope(
    error: &ActionError,
    api_name: &str,
    request_id: &str,
) -> ontoforge_api::dto::ErrorEnvelope {
    let (parameter_errors, message) = match error {
        ActionError::ParameterErrors(problems) => (
            Some(
                problems
                    .iter()
                    .map(|problem| ontoforge_api::dto::ParameterError {
                        item_ordinal: u32::try_from(problem.item_ordinal)
                            .expect("bounded action item ordinal"),
                        name: problem.name.clone(),
                        reason: problem.reason.clone(),
                    })
                    .collect(),
            ),
            None,
        ),
        ActionError::CriteriaFailed(message) => (None, Some(message.clone())),
        _ => (None, None),
    };
    ontoforge_api::dto::ErrorEnvelope {
        code: action_code(error).to_owned(),
        api_name: api_name.to_owned(),
        request_id: request_id.to_owned(),
        parameter_errors,
        message,
    }
}

pub fn action_code(error: &ActionError) -> &'static str {
    match error {
        ActionError::VersionConflict => "VERSION_CONFLICT",
        ActionError::ScopeDenied | ActionError::NotFound | ActionError::UnknownAction(_) => {
            "NOT_FOUND"
        }
        ActionError::EngineUnavailable => "SOURCE_UNAVAILABLE",
        ActionError::SourceTarget(error) => error.code(),
        ActionError::Store(_) => "STORE_UNAVAILABLE",
        ActionError::Commit(ontoforge_storage::commit::ActionCommitError::CommandIdReused) => {
            "COMMAND_ID_REUSED"
        }
        ActionError::Commit(ontoforge_storage::commit::ActionCommitError::Edit(
            ontoforge_storage::commit::EditError::DefinitionStale { .. },
        )) => "DEFINITION_STALE",
        ActionError::MissingCommandId => "MISSING_COMMAND_ID",
        ActionError::ParameterErrors(_) => "INVALID_PARAMETERS",
        ActionError::InvalidRequest(_) => "INVALID_REQUEST",
        ActionError::AmbiguousObject => "AMBIGUOUS_OBJECT",
        ActionError::CriteriaFailed(_) => "CRITERIA_FAILED",
        ActionError::UnsupportedRule => "UNSUPPORTED",
        ActionError::InvalidLimits => "ACTION_UNAVAILABLE",
        ActionError::Commit(_) => "ACTION_FAILED",
    }
}
pub fn json_rejection(api_name: &str, request_id: &str) -> Response {
    wire(
        StatusCode::BAD_REQUEST,
        "INVALID_REQUEST",
        api_name,
        request_id,
    )
}

pub async fn not_found(request: axum::extract::Request) -> Response {
    let request_id = ulid::Ulid::generate().to_string();
    let api_name = request
        .uri()
        .path()
        .split('/')
        .next_back()
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown");
    wire(StatusCode::NOT_FOUND, "NOT_FOUND", api_name, &request_id)
}

pub async fn method_not_allowed(
    axum::extract::Extension(trace): axum::extract::Extension<crate::context::RequestTrace>,
    request: axum::extract::Request,
) -> Response {
    let api_name = request.uri().path().split('/').nth(2).unwrap_or("unknown");
    wire(
        StatusCode::METHOD_NOT_ALLOWED,
        "METHOD_NOT_ALLOWED",
        api_name,
        &trace.id(),
    )
}
