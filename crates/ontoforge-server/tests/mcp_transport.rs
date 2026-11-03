use serde_json::{Value, json};
use std::time::Duration;

use ontoforge_server::mcp::mcp_sessions::{SessionAdmission, SessionOwners};

mod support;

#[tokio::test]
async fn transport_authenticates_initialize_and_control_requests() {
    let fixture = support::fixture().await;
    let running = fixture.start().await;
    let response = running.client.post(format!("{}/mcp", running.url))
        .header("accept", "application/json, text/event-stream")
        .json(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"0"}}}))
        .send().await.unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
    let session = initialize(&running, "2025-03-26").await;
    for method in [
        reqwest::Method::GET,
        reqwest::Method::DELETE,
        reqwest::Method::POST,
    ] {
        let response = running.client.request(method, format!("{}/mcp", running.url))
            .header("accept", "application/json, text/event-stream")
            .header("mcp-protocol-version", "2025-03-26")
            .header("mcp-session-id", &session)
            .json(&json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":2}}))
            .send().await.unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
    }
    for method in [
        reqwest::Method::GET,
        reqwest::Method::DELETE,
        reqwest::Method::POST,
    ] {
        let response = running.client.request(method, format!("{}/mcp", running.url))
            .bearer_auth(&running.token)
            .header("x-principal", "forged")
            .header("accept", "application/json, text/event-stream")
            .header("mcp-protocol-version", "2025-03-26")
            .header("mcp-session-id", &session)
            .json(&json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":2}}))
            .send().await.unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);
    }
    let _ = running.finish().await;
}

#[tokio::test]
async fn transport_binds_initialize_owner_before_first_tool() {
    use ontoforge_auth::authn::dev::{DevClaims, DevKey, issue_dev_token};
    let fixture = support::fixture().await;
    let path = camino::Utf8PathBuf::from_path_buf(fixture.dir.path().join("token.key")).unwrap();
    let key = DevKey::load_or_create(&path).unwrap();
    let bob = issue_dev_token(
        &key,
        &DevClaims {
            sub: "bob".to_owned(),
            groups: vec!["dispatchers".to_owned()],
            attrs: serde_json::Map::new(),
            ttl: Duration::from_secs(600),
        },
    )
    .unwrap();
    let running = fixture.start().await;
    let session = initialize(&running, "2025-03-26").await;
    for method in [
        reqwest::Method::POST,
        reqwest::Method::GET,
        reqwest::Method::DELETE,
    ] {
        let response = running
            .client
            .request(method, format!("{}/mcp", running.url))
            .bearer_auth(&bob)
            .header("accept", "application/json, text/event-stream")
            .header("mcp-protocol-version", "2025-03-26")
            .header("mcp-session-id", &session)
            .json(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
    }
    let owner = session_post(
        &running,
        &session,
        &running.token,
        "2025-03-26",
        json!({"jsonrpc":"2.0","id":3,"method":"tools/list"}),
    )
    .await;
    assert!(owner.status().is_success());
    let _ = running.finish().await;
}

#[tokio::test]
async fn shutdown_closes_live_sse_connection() {
    let running = support::fixture().await.start().await;
    let session = initialize(&running, "2025-03-26").await;
    let response = running
        .client
        .get(format!("{}/mcp", running.url))
        .bearer_auth(&running.token)
        .header("accept", "text/event-stream")
        .header("mcp-protocol-version", "2025-03-26")
        .header("mcp-session-id", &session)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert!(
        tokio::time::timeout(Duration::from_secs(7), running.finish())
            .await
            .is_ok()
    );
    assert!(
        tokio::time::timeout(Duration::from_secs(1), response.bytes())
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn cancellation_stops_only_requested_read_and_persists_audit() {
    use std::sync::atomic::Ordering;
    let running = support::fixture().await.start().await;
    let session = initialize(&running, "2025-03-26").await;
    running.engine.block.store(true, Ordering::SeqCst);
    let client = running.client.clone();
    let url = format!("{}/mcp", running.url);
    let token = running.token.clone();
    let read_session = session.clone();
    let pending = tokio::spawn(async move {
        client.post(url).bearer_auth(token)
            .header("accept", "application/json, text/event-stream")
            .header("mcp-protocol-version", "2025-03-26")
            .header("mcp-session-id", read_session)
            .json(&json!({"jsonrpc":"2.0","id":"blocked-read","method":"tools/call","params":{"name":"query","arguments":{"request":{"set":{"kind":"type","object_type":"Flight"},"select":["flightNumber"],"page":{"first":3}}}}}))
            .send().await.unwrap().text().await
    });
    tokio::time::timeout(Duration::from_secs(3), running.engine.entered.notified())
        .await
        .unwrap();
    let response = session_post(&running, &session, &running.token, "2025-03-26", json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":"blocked-read"}})).await;
    assert!(response.status().is_success());
    tokio::time::timeout(Duration::from_secs(2), running.engine.cancelled.notified())
        .await
        .expect("target engine future must be cancelled");
    running.engine.block.store(false, Ordering::SeqCst);
    let unrelated = session_post(
        &running,
        &session,
        &running.token,
        "2025-03-26",
        json!({"jsonrpc":"2.0","id":999,"method":"tools/list"}),
    )
    .await;
    assert!(unrelated.status().is_success());
    pending.abort();
    let audit = running.finish().await;
    assert_eq!(audit.len(), 1);
    assert_eq!(audit[0]["status"], "cancelled");
}

#[tokio::test]
async fn shutdown_surfaces_owned_action_task_failure() {
    let fixture = support::fixture().await;
    ontoforge_server::context::register_action(
        &fixture.host,
        tokio::spawn(async { panic!("accepted action task failed") }),
    );
    assert!(
        ontoforge_server::lifecycle::ServerLifecycle::new(&fixture.host)
            .shutdown()
            .await
            .is_err()
    );
}

#[tokio::test]
async fn shutdown_deadline_still_closes_audit_after_stuck_action() {
    let fixture = support::fixture().await;
    let task = tokio::spawn(std::future::pending::<()>());
    let abort = task.abort_handle();
    ontoforge_server::context::register_action(&fixture.host, task);
    let started = std::time::Instant::now();
    let result = ontoforge_server::lifecycle::ServerLifecycle::new(&fixture.host)
        .shutdown()
        .await;
    abort.abort();
    assert!(result.is_err());
    assert!(started.elapsed() < Duration::from_secs(7));
    assert!(
        !fixture.store.read_audit().health().available,
        "timeout must still close the audit sink"
    );
}

#[tokio::test]
async fn cancelled_action_survives_transport_shutdown_without_replay() {
    use std::sync::atomic::Ordering;
    let running = support::fixture().await.start().await;
    let session = initialize(&running, "2025-03-26").await;
    running.engine.block.store(true, Ordering::SeqCst);
    let client = running.client.clone();
    let url = format!("{}/mcp", running.url);
    let token = running.token.clone();
    let action_session = session.clone();
    let pending = tokio::spawn(async move {
        client.post(url).bearer_auth(token)
            .header("accept", "application/json, text/event-stream")
            .header("mcp-protocol-version", "2025-03-26")
            .header("mcp-session-id", action_session)
            .json(&json!({"jsonrpc":"2.0","id":"accepted-action","method":"tools/call","params":{"name":"action_delayFlight","arguments":{"flight":"F-1001","delayMinutes":2,"commandId":"owned-mcp-action"}}}))
            .send().await
    });
    tokio::time::timeout(Duration::from_secs(3), running.engine.entered.notified())
        .await
        .unwrap();
    let cancelled = session_post(&running, &session, &running.token, "2025-03-26", json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":"accepted-action"}})).await;
    assert!(cancelled.status().is_success());
    let client = running.client.clone();
    let url = format!("{}/mcp", running.url);
    let token = running.token.clone();
    let read_session = session.clone();
    let read = tokio::spawn(async move {
        client.post(url).bearer_auth(token)
            .header("accept", "application/json, text/event-stream")
            .header("mcp-protocol-version", "2025-03-26")
            .header("mcp-session-id", read_session)
            .json(&json!({"jsonrpc":"2.0","id":"shutdown-read","method":"tools/call","params":{"name":"query","arguments":{"request":{"set":{"kind":"type","object_type":"Flight"},"select":["flightNumber"],"page":{"first":3}}}}}))
            .send().await.unwrap().text().await
    });
    tokio::time::timeout(Duration::from_secs(3), running.engine.entered.notified())
        .await
        .unwrap();
    let sse = running
        .client
        .get(format!("{}/mcp", running.url))
        .bearer_auth(&running.token)
        .header("accept", "text/event-stream")
        .header("mcp-protocol-version", "2025-03-26")
        .header("mcp-session-id", session)
        .send()
        .await
        .unwrap();
    assert_eq!(sse.status(), reqwest::StatusCode::OK);
    let engine = std::sync::Arc::clone(&running.engine);
    let release = async move {
        assert!(
            tokio::time::timeout(Duration::from_millis(100), engine.cancelled.notified())
                .await
                .is_err(),
            "independent reads must remain active until the accepted Action finishes"
        );
        engine.block.store(false, Ordering::SeqCst);
        engine.release.notify_one();
    };
    let (audit, ()) = tokio::join!(running.finish_actions(), release);
    assert_eq!(audit.len(), 1);
    assert_eq!(audit[0]["status"], "cancelled");
    assert!(
        tokio::time::timeout(Duration::from_secs(1), sse.bytes())
            .await
            .is_ok()
    );
    read.abort();
    pending.abort();
}

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
    assert!(owners.verify("one", "alice").is_err());
    assert!(owners.verify("one", "bob").is_err());
    assert_eq!(owners.admit("two", "bob"), Ok(SessionAdmission::Bound));
    assert!(owners.verify("one", "alice").is_err());
    owners.remove("two");
    assert_eq!(owners.len(), 0);
    let owners = SessionOwners::new(1, Duration::from_secs(30));
    owners.admit("one", "alice").unwrap();
    owners.admit("two", "bob").unwrap();
    assert!(owners.verify("one", "alice").is_err());
    assert!(owners.verify("one", "bob").is_err());
    assert!(owners.verify("two", "bob").is_ok());
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
async fn legacy_session_lists_tools() {
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
