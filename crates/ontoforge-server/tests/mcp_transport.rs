use serde_json::{Value, json};
use std::time::Duration;

use ontoforge_server::mcp::mcp_sessions::{SessionAdmission, SessionOwners};

mod support;

async fn initialize(running: &support::Running, version: &str) -> String {
    let response = running
        .client
        .post(format!("{}/mcp", running.url))
        .bearer_auth(&running.token)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", version)
        .json(&json!({
            "jsonrpc":"2.0",
            "id":1,
            "method":"initialize",
            "params":{"protocolVersion":version,"capabilities":{},"clientInfo":{"name":"transport-contract","version":"0"}}
        }))
        .send()
        .await
        .unwrap();
    response
        .headers()
        .get("mcp-session-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned()
}

async fn session_post(
    running: &support::Running,
    session: &str,
    token: &str,
    version: &str,
    body: Value,
) -> reqwest::Response {
    running
        .client
        .post(format!("{}/mcp", running.url))
        .bearer_auth(token)
        .header("accept", "application/json, text/event-stream")
        .header("content-type", "application/json")
        .header("mcp-protocol-version", version)
        .header("mcp-session-id", session)
        .json(&body)
        .send()
        .await
        .unwrap()
}

#[tokio::test]
async fn mcp_old_session_reauth_owner_ttl_capacity_delete() {
    let fixture = support::fixture().await;
    let running = fixture.start().await;
    let session = initialize(&running, "2025-03-26").await;
    assert!(!session.is_empty());
    let denied = session_post(
        &running,
        &session,
        "invalid-owner-token",
        "2025-03-26",
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
    )
    .await;
    let denied_status = denied.status();
    let denied_body = denied.text().await.unwrap();
    assert!(
        denied_status == reqwest::StatusCode::UNAUTHORIZED || denied_body.contains("error"),
        "status={denied_status} body={denied_body}"
    );
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
    let replay = session_post(
        &running,
        &session,
        &running.token,
        "2025-03-26",
        json!({"jsonrpc":"2.0","id":3,"method":"tools/list"}),
    )
    .await;
    assert_eq!(replay.status(), reqwest::StatusCode::NOT_FOUND);
    let _ = running.finish().await;
}

#[test]
fn session_owners_expire_evict_and_delete() {
    let owners = SessionOwners::new(1, Duration::from_millis(1));
    assert_eq!(owners.admit("one", "alice"), Ok(SessionAdmission::Bound));
    assert_eq!(owners.admit("one", "alice"), Ok(SessionAdmission::Existing));
    assert!(owners.admit("one", "bob").is_err());
    std::thread::sleep(Duration::from_millis(3));
    assert_eq!(owners.admit("two", "bob"), Ok(SessionAdmission::Bound));
    owners.remove("two");
    assert_eq!(owners.len(), 0);
}

#[tokio::test]
async fn mcp_new_protocol_without_session_requires_metadata() {
    let fixture = support::fixture().await;
    let running = fixture.start().await;
    let response = running
        .client
        .post(format!("{}/mcp", running.url))
        .bearer_auth(&running.token)
        .header("accept", "application/json, text/event-stream")
        .header("content-type", "application/json")
        .header("mcp-protocol-version", "2026-07-28")
        .json(&json!({"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}))
        .send()
        .await
        .unwrap();
    assert!(response.status().is_client_error(), "{}", response.status());
    let _ = running.finish().await;
}

#[tokio::test]
async fn mcp_cancel_disconnect_sse_do_not_replay_action() {
    let fixture = support::fixture().await;
    let running = fixture.start().await;
    let session = initialize(&running, "2025-03-26").await;
    let response = session_post(
        &running,
        &session,
        &running.token,
        "2025-03-26",
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
    )
    .await;
    assert!(response.status().is_success());
    let _ = running.finish().await;
}
