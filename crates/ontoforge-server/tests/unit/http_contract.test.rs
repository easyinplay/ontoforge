#[path = "../support/mod.rs"]
mod support;

use reqwest::StatusCode;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::sync::atomic::Ordering;
use support::{fixture, success};

async fn rejection(response: reqwest::Response, expected: StatusCode, code: &str) -> String {
    assert_eq!(response.status(), expected);
    assert!(
        response
            .headers()
            .get("content-type")
            .is_some_and(|v| v == "application/json"),
        "every rejection must use the structured JSON envelope"
    );
    let id = response.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["code"], code);
    assert_eq!(body["request_id"], id);
    assert!(body["apiName"].as_str().is_some_and(|v| !v.is_empty()));
    assert_eq!(
        body.as_object().unwrap().len(),
        3,
        "error text and backend details must stay private"
    );
    id
}

#[tokio::test]
async fn http_framework_rejections_are_structured() {
    let f = fixture().await.start().await;
    let query =
        json!({"set":{"kind":"type","object_type":"Airport"},"select":["name"],"page":{"first":1}});
    let invalid = f
        .client
        .post(format!("{}/api/query", f.url))
        .bearer_auth(&f.token)
        .header("content-type", "application/json")
        .body("{")
        .send()
        .await
        .unwrap();
    rejection(invalid, StatusCode::BAD_REQUEST, "INVALID_REQUEST").await;
    let mut unknown = query.clone();
    unknown["limits"] = json!({"maxRows": 999999});
    rejection(
        f.post("/api/query", &unknown).await,
        StatusCode::BAD_REQUEST,
        "INVALID_REQUEST",
    )
    .await;
    rejection(
        f.get("/api/objects/Airport/%FF").await,
        StatusCode::BAD_REQUEST,
        "INVALID_REQUEST",
    )
    .await;
    rejection(
        f.get("/api/ontology?first=garbage").await,
        StatusCode::BAD_REQUEST,
        "INVALID_REQUEST",
    )
    .await;
    rejection(
        f.post("/api/query?first=garbage", &query).await,
        StatusCode::BAD_REQUEST,
        "INVALID_REQUEST",
    )
    .await;
    rejection(
        f.get("/api/absent").await,
        StatusCode::NOT_FOUND,
        "NOT_FOUND",
    )
    .await;
    let large = f
        .client
        .post(format!("{}/api/query", f.url))
        .bearer_auth(&f.token)
        .body(" ".repeat(1024 * 1024 + 1))
        .send()
        .await
        .unwrap();
    rejection(large, StatusCode::PAYLOAD_TOO_LARGE, "BODY_TOO_LARGE").await;
    let unauth = f
        .client
        .post(format!("{}/api/query", f.url))
        .json(&query)
        .send()
        .await
        .unwrap();
    rejection(unauth, StatusCode::UNAUTHORIZED, "UNAUTHENTICATED").await;
    assert_eq!(f.engine.calls.load(Ordering::SeqCst), 0);
    assert!(
        f.finish().await.is_empty(),
        "framework rejections must not admit Query or business writes"
    );
}

#[tokio::test]
async fn http_error_utf8_ids_and_l0_redaction() {
    let f = fixture().await.start().await;
    let metadata = success(f.get("/api/ontology").await).await;
    assert_eq!(metadata["description"], "航空数据 UTF-8");
    f.engine.fail.store(true, Ordering::SeqCst);
    let mut tasks = Vec::new();
    for _ in 0..16 {
        let client = f.client.clone();
        let url = f.url.clone();
        let token = f.token.clone();
        tasks.push(tokio::spawn(async move {
            let response = client.post(format!("{url}/api/query")).bearer_auth(token).json(&json!({"set":{"kind":"type","object_type":"Flight"},"select":["flightId"],"page":{"first":1}})).send().await.unwrap();
            rejection(response, StatusCode::SERVICE_UNAVAILABLE, "SOURCE_UNAVAILABLE").await
        }));
    }
    let mut ids = BTreeSet::new();
    for task in tasks {
        assert!(
            ids.insert(task.await.unwrap()),
            "concurrent request IDs must be distinct"
        );
    }
    let audit = f.finish().await;
    assert_eq!(audit.len(), 16);
    assert!(
        audit
            .iter()
            .all(|r| ids.contains(r["request_id"].as_str().unwrap())
                && r["status"] == "source_error")
    );
    let text = serde_json::to_string(&audit).unwrap();
    for secret in ["physical_table", "SELECT", "secret_token", "sentinel"] {
        assert!(!text.contains(secret));
    }
}

#[tokio::test]
async fn http_rows_bytes_concurrency_timeout_and_audit_fail_closed() {
    let f = fixture().await.start().await;
    f.store.read_audit().shutdown().await.unwrap();
    rejection(
        f.get("/api/objects/Flight/F-1001").await,
        StatusCode::SERVICE_UNAVAILABLE,
        "AUDIT_UNAVAILABLE",
    )
    .await;
    rejection(f.post("/api/query", &json!({"set":{"kind":"type","object_type":"Flight"},"select":["flightId"],"page":{"first":1}})).await, StatusCode::SERVICE_UNAVAILABLE,"AUDIT_UNAVAILABLE").await;
    rejection(
        f.post(
            "/api/aggregate",
            &json!({"set":{"kind":"type","object_type":"Flight"},"measures":[{"op":"count"}]}),
        )
        .await,
        StatusCode::SERVICE_UNAVAILABLE,
        "AUDIT_UNAVAILABLE",
    )
    .await;
    assert_eq!(
        f.engine.calls.load(Ordering::SeqCst),
        0,
        "audit unavailable must fail closed before engine dispatch"
    );
    assert!(f.finish().await.is_empty());
}
