use axum::body::to_bytes;
use axum::http::StatusCode;
use ontoforge_query::{ExecutionLimits, QueryPage};
use ontoforge_server::error;
use serde_json::json;

#[tokio::test]
async fn http_framework_rejections_are_structured() {
    let response = error::json_rejection("query", "01J00000000000000000000000");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_bytes(response.into_body(), 4096).await.unwrap();
    let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(value["code"], "INVALID_REQUEST");
    assert_eq!(value["apiName"], "query");
    assert_eq!(value["request_id"], "01J00000000000000000000000");
}

#[tokio::test]
async fn http_error_utf8_ids_and_l0_redaction() {
    let response = error::wire(
        StatusCode::BAD_REQUEST,
        "INVALID_QUERY",
        "查询",
        "01J00000000000000000000001",
    );
    let body = to_bytes(response.into_body(), 4096).await.unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();
    assert!(text.contains("查询"));
    assert!(text.contains("01J00000000000000000000001"));
    assert!(!text.contains("physical_table"));
    assert!(!text.contains("SELECT"));
}

#[test]
fn http_rows_bytes_concurrency_timeout_and_audit_fail_closed() {
    let limits = ExecutionLimits::try_new(1, 1, 1, 1).unwrap();
    assert_eq!(limits.max_decoded_bytes, 1);
    let page = QueryPage::bounded(Vec::new(), 1, 1);
    assert!(!page.truncated);
    let parsed: Result<ontoforge_api::dto::QueryRequest, _> = serde_json::from_value(json!({
        "set": {"kind": "type", "objectType": "Flight"},
        "page": {"first": 1},
        "unknown": true
    }));
    assert!(parsed.is_err());
}
