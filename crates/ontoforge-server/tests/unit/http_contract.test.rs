#[path = "../support/mod.rs"]
mod support;

use ontoforge_engine::EngineError;
use ontoforge_server::ServerLimits;
use reqwest::StatusCode;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::sync::atomic::Ordering;
use std::time::Duration;
use support::{fixture, success};
use tokio::io::AsyncWriteExt;

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
    for secret in ["physical_table", "SELECT", "secret_token", "sentinel"] {
        assert!(
            !body.to_string().contains(secret),
            "backend details must not appear in the actual HTTP body"
        );
    }
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
    unknown["limits"] = json!({"maxRows": 999_999});
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
    for (response, allowed) in [
        (f.get("/api/query").await, "POST"),
        (f.post("/api/ontology", &json!({})).await, "GET,HEAD"),
    ] {
        assert_eq!(response.headers()["allow"], allowed);
        rejection(
            response,
            StatusCode::METHOD_NOT_ALLOWED,
            "METHOD_NOT_ALLOWED",
        )
        .await;
    }
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
    for (failure, status, code) in [
        (
            EngineError::Config(String::new()),
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
        *f.engine.failure.lock().unwrap() = Some(failure);
        for operation in ["pk", "list", "aggregate"] {
            let response = match operation {
                "pk" => f.get("/api/objects/Flight/F-1001").await,
                "list" => f.post("/api/query",&json!({"set":{"kind":"type","object_type":"Flight"},"select":["flightId"],"page":{"first":1}})).await,
                _ => f.post("/api/aggregate",&json!({"set":{"kind":"type","object_type":"Flight"},"measures":[{"op":"count"}]})).await,
            };
            assert!(ids.insert(rejection(response, status, code).await));
        }
    }
    let audit = f.finish().await;
    assert_eq!(audit.len(), 25);
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
    work_budget_boundaries().await;
    pk_budget_boundaries().await;
    body_budget_boundaries().await;
    deadline_and_disconnect().await;
    concurrency_admission().await;
    audit_unavailable().await;
}

async fn work_budget_boundaries() {
    // Edit scanning charges JSON property bytes and the canonical JSON primary key.
    let bytes = b"{\"country\":\"DE\",\"name\":\"Berlin\"}".len()
        + b"\"BER\"".len()
        + b"{\"country\":\"DE\",\"name\":\"Frankfurt\"}".len()
        + b"\"FRA\"".len()
        + b"{\"name\":\"Munich\"}".len()
        + b"\"MUC\"".len();
    for (rows, max_bytes, succeeds) in [(3, bytes, true), (2, bytes, false), (3, bytes - 1, false)]
    {
        let mut limits = ServerLimits::default();
        limits.execution.max_raw_candidates = rows;
        limits.execution.max_decoded_bytes = max_bytes;
        let f = fixture().await.start_with_limits(limits).await;
        let response = f.post("/api/query", &json!({"set":{"kind":"type","object_type":"Airport"},"select":["name"],"page":{"first":100}})).await;
        let id = response.headers()["x-request-id"]
            .to_str()
            .unwrap()
            .to_owned();
        if succeeds {
            assert_eq!(
                support::keys(&success(response).await),
                ["BER", "FRA", "MUC"]
            );
        } else {
            rejection(response, StatusCode::BAD_REQUEST, "WORK_BUDGET_EXCEEDED").await;
        }
        let audit = f.finish().await;
        assert_eq!(audit.len(), 1);
        assert_eq!(audit[0]["request_id"], id);
        assert_eq!(audit[0]["row_count"], if succeeds { 3 } else { 0 });
        assert_eq!(
            audit[0]["status"],
            if succeeds { "success" } else { "denied" }
        );
    }

    let utf8_bytes = "{\"name\":\"北京\"}".len() + b"\"PEK\"".len();
    for (cap, succeeds) in [(utf8_bytes, true), (utf8_bytes - 1, false)] {
        let mut limits = ServerLimits::default();
        limits.execution.max_decoded_bytes = cap;
        let f = support::fixture_airports(&[("PEK", "北京", None)])
            .await
            .start_with_limits(limits)
            .await;
        let response = f.post("/api/query", &json!({"set":{"kind":"type","object_type":"Airport"},"select":["name"],"page":{"first":1}})).await;
        if succeeds {
            assert_eq!(
                success(response).await["rows"][0]["properties"]["name"],
                "北京"
            );
        } else {
            rejection(response, StatusCode::BAD_REQUEST, "WORK_BUDGET_EXCEEDED").await;
        }
        assert_eq!(f.finish().await.len(), 1);
    }
}

async fn body_budget_boundaries() {
    let body =
        json!({"set":{"kind":"type","object_type":"Airport"},"select":["name"],"page":{"first":1}})
            .to_string();
    for (cap, succeeds) in [(body.len(), true), (body.len() - 1, false)] {
        let limits = ServerLimits {
            max_body_bytes: cap,
            ..ServerLimits::default()
        };
        let f = fixture().await.start_with_limits(limits).await;
        let response = f
            .client
            .post(format!("{}/api/query", f.url))
            .bearer_auth(&f.token)
            .header("content-type", "application/json")
            .body(body.clone())
            .send()
            .await
            .unwrap();
        if succeeds {
            assert_eq!(support::keys(&success(response).await), ["BER"]);
        } else {
            rejection(response, StatusCode::PAYLOAD_TOO_LARGE, "BODY_TOO_LARGE").await;
        }
        assert_eq!(f.finish().await.len(), usize::from(succeeds));
    }
}

async fn pk_budget_boundaries() {
    let f = fixture().await.start().await;
    let row = success(f.get("/api/objects/Airport/BER").await).await;
    let bytes = serde_json::to_vec(&row).unwrap().len();
    assert_eq!(f.finish().await.len(), 1);
    for (cap, succeeds) in [(bytes, true), (bytes - 1, false)] {
        let mut limits = ServerLimits::default();
        limits.execution.max_decoded_bytes = cap;
        let f = fixture().await.start_with_limits(limits).await;
        let response = f.get("/api/objects/Airport/BER").await;
        let id = response.headers()["x-request-id"].to_str().unwrap().to_owned();
        if succeeds {
            assert_eq!(success(response).await["pk"], "BER");
        } else {
            rejection(response, StatusCode::BAD_REQUEST, "COST_EXCEEDED").await;
        }
        assert_eq!(f.engine.calls.load(Ordering::SeqCst), 0);
        let audit = f.finish().await;
        assert_eq!(audit.len(), 1);
        assert_eq!(audit[0]["request_id"], id);
        assert_eq!(audit[0]["row_count"], usize::from(succeeds));
    }
}

async fn deadline_and_disconnect() {
    for operation in ["pk", "list", "aggregate"] {
        let limits = ServerLimits {
            deadline: Duration::from_millis(200),
            ..ServerLimits::default()
        };
        let f = fixture().await.start_with_limits(limits).await;
        f.engine.block.store(true, Ordering::SeqCst);
        let client = f.client.clone();
        let url = f.url.clone();
        let token = f.token.clone();
        let pending = tokio::spawn(async move {
            let request = match operation {
                "pk" => client.get(format!("{url}/api/objects/Flight/F-1001")),
                "list" => client.post(format!("{url}/api/query")).json(&json!({"set":{"kind":"type","object_type":"Flight"},"select":["flightId"],"page":{"first":1}})),
                _ => client.post(format!("{url}/api/aggregate")).json(&json!({"set":{"kind":"type","object_type":"Flight"},"measures":[{"op":"count"}]})),
            };
            request.bearer_auth(token).send().await.unwrap()
        });
        tokio::time::timeout(Duration::from_secs(3), f.engine.entered.notified())
            .await
            .unwrap();
        let response = tokio::time::timeout(Duration::from_secs(3), pending)
            .await
            .unwrap()
            .unwrap();
        let id = rejection(response, StatusCode::GATEWAY_TIMEOUT, "DEADLINE_EXCEEDED").await;
        tokio::time::timeout(Duration::from_secs(3), f.engine.cancelled.notified())
            .await
            .expect("deadline must drop the pending engine future");
        f.engine.block.store(false, Ordering::SeqCst);
        assert_eq!(
            success(f.get("/api/objects/Airport/BER").await).await["pk"],
            "BER"
        );
        let audit = f.finish().await;
        assert_eq!(audit.len(), 2);
        assert!(
            audit.iter().any(|r| r["request_id"] == id
                && r["status"] == "cancelled"
                && r["row_count"] == 0),
            "deadline ID must match exactly the cancelled reservation"
        );
    }

    let limits = ServerLimits {
        deadline: Duration::from_secs(5),
        ..ServerLimits::default()
    };
    let f = fixture().await.start_with_limits(limits).await;
    f.engine.block.store(true, Ordering::SeqCst);
    let address = f.url.strip_prefix("http://").unwrap();
    let mut socket = tokio::net::TcpStream::connect(address).await.unwrap();
    socket.write_all(format!("GET /api/objects/Flight/F-1001 HTTP/1.1\r\nHost: {address}\r\nAuthorization: Bearer {}\r\n\r\n", f.token).as_bytes()).await.unwrap();
    tokio::time::timeout(Duration::from_secs(3), f.engine.entered.notified())
        .await
        .unwrap();
    drop(socket);
    let cancelled =
        tokio::time::timeout(Duration::from_secs(3), f.engine.cancelled.notified()).await;
    if cancelled.is_err() {
        f.engine.block.store(false, Ordering::SeqCst);
        f.engine.release.notify_waiters();
    }
    let audit = f.finish().await;
    assert!(
        cancelled.is_ok(),
        "client disconnect must drop the independent read future"
    );
    assert_eq!(audit.len(), 1);
    assert_eq!(audit[0]["status"], "cancelled");
    assert_eq!(audit[0]["row_count"], 0);
}

async fn concurrency_admission() {
    let f = fixture().await.start().await;
    f.engine.block.store(true, Ordering::SeqCst);
    let mut pending = Vec::new();
    for _ in 0..32 {
        let client = f.client.clone();
        let url = f.url.clone();
        let token = f.token.clone();
        pending.push(tokio::spawn(async move {
            client
                .get(format!("{url}/api/objects/Flight/F-1001"))
                .bearer_auth(token)
                .send()
                .await
                .unwrap()
        }));
        tokio::time::timeout(Duration::from_secs(3), f.engine.entered.notified())
            .await
            .unwrap();
    }
    let saturated = f
        .client
        .get(format!("{}/api/objects/Flight/F-1001", f.url))
        .bearer_auth(&f.token)
        .timeout(Duration::from_secs(1))
        .send()
        .await;
    f.engine.block.store(false, Ordering::SeqCst);
    f.engine.release.notify_waiters();
    for task in pending {
        success(task.await.unwrap()).await;
    }
    let audit = f.finish().await;
    assert!(
        saturated.is_ok(),
        "saturated HTTP admission must promptly return a structured rejection"
    );
    rejection(
        saturated.unwrap(),
        StatusCode::TOO_MANY_REQUESTS,
        "CONCURRENCY_LIMIT",
    )
    .await;
    assert_eq!(
        audit.len(),
        32,
        "rejected admission cannot reserve read audit"
    );
}

async fn audit_unavailable() {
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
