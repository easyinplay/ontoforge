use std::sync::atomic::Ordering;
use std::time::Duration;

use ontoforge_server::ServerLimits;
use reqwest::StatusCode;
use serde_json::{Value, json};

mod support;

async fn running() -> support::Running {
    support::fixture()
        .await
        .start_with_limits(ServerLimits {
            max_concurrent_requests: 1,
            max_body_bytes: 1024,
            ..ServerLimits::default()
        })
        .await
}

async fn initialize(running: &support::Running, suffix: &str) -> String {
    let response = running.client.post(format!("{}/mcp{suffix}", running.url))
        .bearer_auth(&running.token).header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-03-26")
        .json(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"admission","version":"0"}}}))
        .send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let session = response
        .headers()
        .get("mcp-session-id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    response.bytes().await.unwrap();
    session
}

fn rpc(running: &support::Running, session: &str) -> reqwest::RequestBuilder {
    running
        .client
        .post(format!("{}/mcp", running.url))
        .bearer_auth(&running.token)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-03-26")
        .header("mcp-session-id", session)
}

fn query() -> Value {
    json!({"set":{"kind":"type","object_type":"Flight"},"select":["flightNumber"],"page":{"first":3}})
}

async fn entered(running: &support::Running) {
    tokio::time::timeout(Duration::from_secs(3), running.engine.entered.notified())
        .await
        .unwrap();
}

async fn available(running: &support::Running) {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let response = running.get("/api/ontology").await;
            if response.status() == StatusCode::OK {
                break;
            }
            assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("execution permit must be released");
}

#[tokio::test]
async fn rest_request_blocks_mcp_until_actual_execution_finishes() {
    let running = running().await;
    let session = initialize(&running, "").await;
    running.engine.block.store(true, Ordering::SeqCst);
    let request = running
        .client
        .post(format!("{}/api/query", running.url))
        .bearer_auth(&running.token)
        .json(&query());
    let pending = tokio::spawn(async move { request.send().await.unwrap() });
    entered(&running).await;
    let calls = running.engine.calls.load(Ordering::SeqCst);
    let rejected = rpc(&running, &session)
        .json(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}))
        .send()
        .await
        .unwrap();
    assert_eq!(rejected.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(running.engine.calls.load(Ordering::SeqCst), calls);
    running.engine.block.store(false, Ordering::SeqCst);
    running.engine.release.notify_one();
    assert_eq!(pending.await.unwrap().status(), StatusCode::OK);
    available(&running).await;
    let response = rpc(&running, &session)
        .json(&json!({"jsonrpc":"2.0","id":3,"method":"tools/list"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    response.bytes().await.unwrap();
    assert_eq!(running.finish().await.len(), 1);
}

#[tokio::test]
async fn mcp_read_holds_shared_slot_but_real_cancel_and_sse_still_work() {
    let running = running().await;
    let session = initialize(&running, "").await;
    running.engine.block.store(true, Ordering::SeqCst);
    let request = rpc(&running, &session).json(&json!({"jsonrpc":"2.0","id":"blocked","method":"tools/call","params":{"name":"query","arguments":{"request":query()}}}));
    let pending = tokio::spawn(async move { request.send().await.unwrap().bytes().await });
    entered(&running).await;
    let calls = running.engine.calls.load(Ordering::SeqCst);
    let rejected = running.get("/api/ontology").await;
    assert_eq!(rejected.status(), StatusCode::TOO_MANY_REQUESTS);
    for body in [
        json!({"jsonrpc":"2.0","id":null,"method":"notifications/cancelled","params":{"requestId":"blocked"}}),
        json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{}}),
        json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":true}}),
        json!({"jsonrpc":"2.0","id":9,"method":"tools/list"}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!([{"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":"blocked"}}]),
    ] {
        let response = rpc(&running, &session)
            .header("Mcp-Method", "notifications/cancelled")
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS, "{body}");
    }
    for body in [
        r#"{"jsonrpc":"2.0","method":"tools/list","method":"notifications/cancelled","params":{"requestId":"blocked"}}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/cancelled","params":{},"params":{"requestId":"blocked"}}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":null,"requestId":"blocked"}}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":"blocked","reason":false}}"#,
    ] {
        let response = rpc(&running, &session)
            .header("content-type", "application/json")
            .body(body)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS, "{body}");
    }
    let subpath = running.client.post(format!("{}/mcp/other", running.url)).bearer_auth(&running.token)
        .json(&json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":"blocked"}}))
        .send().await.unwrap();
    assert_eq!(subpath.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(running.engine.calls.load(Ordering::SeqCst), calls);
    assert!(!pending.is_finished());
    let sse = running
        .client
        .get(format!("{}/mcp", running.url))
        .bearer_auth(&running.token)
        .header("accept", "text/event-stream")
        .header("mcp-protocol-version", "2025-03-26")
        .header("mcp-session-id", &session)
        .send()
        .await
        .unwrap();
    assert_eq!(sse.status(), StatusCode::OK);
    let cancel = rpc(&running, &session).json(&json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":"blocked"}})).send().await.unwrap();
    assert!(cancel.status().is_success());
    tokio::time::timeout(Duration::from_secs(2), running.engine.cancelled.notified())
        .await
        .expect("cancel must reach the active engine future");
    running.engine.block.store(false, Ordering::SeqCst);
    available(&running).await;
    let _ = tokio::time::timeout(Duration::from_secs(2), pending)
        .await
        .unwrap()
        .unwrap();
    let deleted = running
        .client
        .delete(format!("{}/mcp", running.url))
        .bearer_auth(&running.token)
        .header("mcp-protocol-version", "2025-03-26")
        .header("mcp-session-id", &session)
        .send()
        .await
        .unwrap();
    assert!(deleted.status().is_success());
    assert!(
        tokio::time::timeout(Duration::from_secs(2), sse.bytes())
            .await
            .is_ok()
    );
    let audit = running.finish().await;
    assert_eq!(audit.len(), 1);
    assert_eq!(audit[0]["status"], "cancelled");
}

#[tokio::test]
async fn cancelled_disconnected_action_holds_permit_until_owned_task_finishes() {
    let running = running().await;
    let session = initialize(&running, "").await;
    running.engine.block.store(true, Ordering::SeqCst);
    let response = rpc(&running, &session).json(&json!({"jsonrpc":"2.0","id":"action","method":"tools/call","params":{"name":"action_delayFlight","arguments":{"params":{"flight":"F-1001","delayMinutes":2},"commandId":"admission-owned-action"}}})).send().await.unwrap();
    entered(&running).await;
    let cancel = rpc(&running, &session).json(&json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":"action"}})).send().await.unwrap();
    assert!(cancel.status().is_success());
    drop(response);
    assert_eq!(
        running.get("/api/ontology").await.status(),
        StatusCode::TOO_MANY_REQUESTS
    );
    let delete = running
        .client
        .delete(format!("{}/mcp", running.url))
        .bearer_auth(&running.token)
        .header("mcp-protocol-version", "2025-03-26")
        .header("mcp-session-id", &session)
        .send()
        .await
        .unwrap();
    assert!(
        delete.status().is_success(),
        "teardown must work while the Action owns the business slot"
    );
    assert_eq!(
        running.get("/api/ontology").await.status(),
        StatusCode::TOO_MANY_REQUESTS
    );
    assert!(
        tokio::time::timeout(
            Duration::from_millis(30),
            running.engine.cancelled.notified()
        )
        .await
        .is_err()
    );
    running.engine.block.store(false, Ordering::SeqCst);
    running.engine.release.notify_one();
    available(&running).await;
    let _ = running.finish_actions().await;
}

#[tokio::test]
async fn rejected_messages_and_large_body_release_reserved_slots() {
    let running = running().await;
    let session = initialize(&running, "/").await;
    for body in ["{".to_owned(), "x".repeat(1025)] {
        let response = rpc(&running, &session)
            .header("content-type", "application/json")
            .body(body)
            .send()
            .await
            .unwrap();
        assert!(response.status().is_client_error());
        available(&running).await;
    }
    let invalid_init = running.client.post(format!("{}/mcp", running.url)).bearer_auth(&running.token)
        .header("accept", "application/json, text/event-stream").header("mcp-protocol-version", "2025-03-26")
        .json(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2026-07-28","capabilities":{},"clientInfo":{"name":"invalid","version":"0"}}}))
        .send().await.unwrap();
    assert!(invalid_init.status().is_client_error());
    available(&running).await;
    let rejected = rpc(&running, &session)
        .bearer_auth("invalid")
        .json(&json!({"jsonrpc":"2.0","id":3,"method":"tools/list"}))
        .send()
        .await
        .unwrap();
    assert_eq!(rejected.status(), StatusCode::UNAUTHORIZED);
    available(&running).await;
    let tools = rpc(&running, &session)
        .json(&json!({"jsonrpc":"2.0","id":4,"method":"tools/list"}))
        .send()
        .await
        .unwrap();
    assert_eq!(tools.status(), StatusCode::OK);
    tools.bytes().await.unwrap();
    assert!(running.finish().await.is_empty());
}
