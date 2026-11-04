use std::sync::atomic::Ordering;
use std::time::Duration;

use reqwest::StatusCode;
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::rest_tests::support;

#[tokio::test]
async fn slow_body_has_one_bounded_control_reservation_until_deadline() {
    let fixture = support::fixture().await;
    let state = crate::state::for_host(&fixture.host);
    let running = fixture
        .start_with_limits(crate::ServerLimits {
            max_concurrent_requests: 1,
            max_body_bytes: 1024,
            deadline: Duration::from_secs(1),
            ..crate::ServerLimits::default()
        })
        .await;
    let endpoint = format!("{}/mcp", running.url);
    let initialize = running.client.post(&endpoint).bearer_auth(&running.token)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-03-26")
        .json(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"reservation","version":"0"}}}))
        .send().await.unwrap();
    assert_eq!(initialize.status(), StatusCode::OK);
    let session = initialize
        .headers()
        .get("mcp-session-id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    initialize.bytes().await.unwrap();
    running.engine.block.store(true, Ordering::SeqCst);
    let request = running.client.post(&endpoint).bearer_auth(&running.token)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-03-26").header("mcp-session-id", &session)
        .json(&json!({"jsonrpc":"2.0","id":"blocked","method":"tools/call","params":{"name":"query","arguments":{"request":{"set":{"kind":"type","object_type":"Flight"},"select":["flightNumber"],"page":{"first":3}}}}}));
    let pending = tokio::spawn(async move { request.send().await.unwrap().bytes().await });
    tokio::time::timeout(Duration::from_secs(3), running.engine.entered.notified())
        .await
        .unwrap();
    let calls = running.engine.calls.load(Ordering::SeqCst);
    let authority = running.url.strip_prefix("http://").unwrap();
    let mut slow = tokio::net::TcpStream::connect(authority).await.unwrap();
    let headers = format!(
        "POST /mcp HTTP/1.1\r\nHost: {authority}\r\nAuthorization: Bearer {}\r\nMcp-Session-Id: {session}\r\nMCP-Protocol-Version: 2025-03-26\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\nContent-Length: 100\r\nConnection: close\r\n\r\n{{",
        running.token
    );
    slow.write_all(headers.as_bytes()).await.unwrap();
    tokio::time::timeout(Duration::from_secs(1), async {
        while state.mcp_controls.available_permits() != 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("incomplete body must hold the host's only control reservation");
    let cancel = || {
        running.client.post(&endpoint).bearer_auth(&running.token)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-03-26").header("mcp-session-id", &session)
        .json(&json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":"blocked"}}))
    };
    let saturated = tokio::time::timeout(Duration::from_millis(500), cancel().send())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(saturated.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(running.engine.calls.load(Ordering::SeqCst), calls);
    assert!(!pending.is_finished());
    let mut response = vec![0; 4096];
    let length = tokio::time::timeout(Duration::from_secs(2), slow.read(&mut response))
        .await
        .unwrap()
        .unwrap();
    assert!(
        std::str::from_utf8(&response[..length])
            .unwrap()
            .starts_with("HTTP/1.1 504")
    );
    assert_eq!(state.mcp_controls.available_permits(), 1);
    drop(slow);
    let accepted = cancel().send().await.unwrap();
    assert!(accepted.status().is_success());
    tokio::time::timeout(Duration::from_secs(2), running.engine.cancelled.notified())
        .await
        .unwrap();
    let _ = pending.await.unwrap();
    let audit = running.finish().await;
    assert_eq!(audit.len(), 1);
    assert_eq!(audit[0]["status"], "cancelled");
}
