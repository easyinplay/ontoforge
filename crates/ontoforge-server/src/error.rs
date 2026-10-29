use axum::Json;
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use ontoforge_query::{QueryError, aggregate::AggregateError};
use serde_json::{Value, json};

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
        QueryError::Source | QueryError::Store | QueryError::AuditUnavailable => {
            StatusCode::SERVICE_UNAVAILABLE
        }
        QueryError::AmbiguousObject => StatusCode::CONFLICT,
        _ => StatusCode::BAD_REQUEST,
    }
}
pub fn aggregate_status(error: &AggregateError) -> StatusCode {
    match error {
        AggregateError::Source | AggregateError::Store | AggregateError::AuditUnavailable => {
            StatusCode::SERVICE_UNAVAILABLE
        }
        _ => StatusCode::BAD_REQUEST,
    }
}

pub fn aggregate_code(error: &AggregateError) -> &'static str {
    match error {
        AggregateError::InvalidQuery => "INVALID_QUERY",
        AggregateError::Unsupported => "UNSUPPORTED",
        AggregateError::AggregateCostExceeded => "COST_EXCEEDED",
        AggregateError::Source => "SOURCE_UNAVAILABLE",
        AggregateError::Store => "STORE_UNAVAILABLE",
        AggregateError::AuditUnavailable => "AUDIT_UNAVAILABLE",
        AggregateError::UnsupportedNumericValue => "UNSUPPORTED_NUMERIC_VALUE",
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
