mod support;

use std::sync::atomic::Ordering;

use ontoforge_engine::EngineError;
use ontoforge_server::ServerLimits;
use ontoforge_storage::store::sqlite_connect_options;
use reqwest::StatusCode;
use serde_json::{Value, json};
use support::{fixture, success};

fn request_id(response: &reqwest::Response) -> String {
    response.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned()
}

async fn rejection(
    response: reqwest::Response,
    status: StatusCode,
    code: &str,
    api_name: &str,
) -> String {
    let actual_status = response.status();
    let id = request_id(&response);
    let body: Value = response.json().await.unwrap();
    assert_eq!(actual_status, status, "{body}");
    assert_eq!(
        body,
        json!({"code": code, "apiName": api_name, "request_id": id}),
        "the error envelope must retain its ID without leaking backend details"
    );
    id
}

fn audit_matches(audit: &[Value], id: &str, status: &str, rows: usize) {
    assert_eq!(audit.len(), 1);
    assert_eq!(audit[0]["request_id"], id);
    assert_eq!(audit[0]["status"], status);
    assert_eq!(audit[0]["row_count"], rows);
    assert_eq!(audit[0]["tool"], "query.pk");
}

#[tokio::test]
async fn history_success_uses_its_durable_audit_id() {
    let f = fixture().await.start().await;
    let response = f.get("/api/objects/Airport/BER/history").await;
    let id = request_id(&response);
    let audit = f.finish().await;
    let body = success(response).await;
    assert_eq!(body["entries"].as_array().unwrap().len(), 1);
    assert_eq!(body["entries"][0]["values"]["name"], "Berlin");
    audit_matches(&audit, &id, "success", 1);
}

#[tokio::test]
async fn history_missing_object_remains_not_found_with_its_audit_id() {
    let f = fixture().await.start().await;
    let response = f.get("/api/objects/Airport/ABSENT/history").await;
    let audit = f.finish().await;
    let id = rejection(response, StatusCode::NOT_FOUND, "NOT_FOUND", "Airport").await;
    audit_matches(&audit, &id, "denied", 0);
}

#[tokio::test]
async fn history_source_failures_preserve_status_and_audit_id() {
    for (failure, status, code) in [
        (
            EngineError::Config("physical_table SELECT secret_token=sentinel".to_owned()),
            StatusCode::SERVICE_UNAVAILABLE,
            "SOURCE_UNAVAILABLE",
        ),
        (
            EngineError::Timeout { after_ms: 1 },
            StatusCode::GATEWAY_TIMEOUT,
            "DEADLINE_EXCEEDED",
        ),
        (
            EngineError::Cancelled,
            StatusCode::REQUEST_TIMEOUT,
            "CANCELLED",
        ),
    ] {
        let f = fixture().await.start().await;
        *f.engine.failure.lock().unwrap() = Some(failure);
        let response = f.get("/api/objects/Flight/F-1001/history").await;
        let calls = f.engine.calls.load(Ordering::SeqCst);
        let audit = f.finish().await;
        let id = rejection(response, status, code, "Flight").await;
        assert_eq!(calls, 1);
        audit_matches(&audit, &id, "source_error", 0);
    }
}

#[tokio::test]
async fn history_query_store_failure_is_unavailable_with_its_audit_id() {
    let f = fixture().await.start().await;
    let pool = sqlx::SqlitePool::connect_with(sqlite_connect_options(&f.path))
        .await
        .unwrap();
    sqlx::query("DROP TABLE of_obj_airport")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
    let response = f.get("/api/objects/Airport/BER/history").await;
    let audit = f.finish().await;
    let id = rejection(
        response,
        StatusCode::SERVICE_UNAVAILABLE,
        "STORE_UNAVAILABLE",
        "Airport",
    )
    .await;
    audit_matches(&audit, &id, "source_error", 0);
}

#[tokio::test]
async fn history_log_store_failure_is_unavailable_with_its_audit_id() {
    let f = fixture().await.start().await;
    let pool = sqlx::SqlitePool::connect_with(sqlite_connect_options(&f.path))
        .await
        .unwrap();
    sqlx::query("DROP TABLE of_edit_history")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
    let response = f.get("/api/objects/Airport/BER/history").await;
    let audit = f.finish().await;
    let id = rejection(
        response,
        StatusCode::SERVICE_UNAVAILABLE,
        "STORE_UNAVAILABLE",
        "Airport",
    )
    .await;
    audit_matches(&audit, &id, "success", 1);
}

#[tokio::test]
async fn history_audit_unavailable_fails_closed_before_source_dispatch() {
    let f = fixture().await.start().await;
    f.store.read_audit().shutdown().await.unwrap();
    let response = f.get("/api/objects/Flight/F-1001/history").await;
    let calls = f.engine.calls.load(Ordering::SeqCst);
    let audit = f.finish().await;
    rejection(
        response,
        StatusCode::SERVICE_UNAVAILABLE,
        "AUDIT_UNAVAILABLE",
        "Flight",
    )
    .await;
    assert_eq!(calls, 0);
    assert!(audit.is_empty());
}

#[tokio::test]
async fn history_truncated_pk_is_cost_exceeded_with_its_audit_id() {
    let mut limits = ServerLimits::default();
    limits.execution.max_decoded_bytes = 1;
    let f = fixture().await.start_with_limits(limits).await;
    let response = f.get("/api/objects/Airport/BER/history").await;
    let audit = f.finish().await;
    let id = rejection(
        response,
        StatusCode::BAD_REQUEST,
        "COST_EXCEEDED",
        "Airport",
    )
    .await;
    audit_matches(&audit, &id, "success", 0);
}
