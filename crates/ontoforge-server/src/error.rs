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
pub fn json_rejection(api_name: &str, request_id: &str) -> Response {
    wire(
        StatusCode::BAD_REQUEST,
        "INVALID_REQUEST",
        api_name,
        request_id,
    )
}
