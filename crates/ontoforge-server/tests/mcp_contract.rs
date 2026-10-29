use ontoforge_auth::scope::PrincipalScope;
use ontoforge_schema::ir::Toolset;
use ontoforge_server::mcp_catalog::{AGGREGATE_TOOL, DESCRIBE_TOOL, HISTORY_TOOL, ToolCatalog};
use serde_json::{Value, json};

mod support;

fn scope(toolsets: &[Toolset]) -> PrincipalScope {
    PrincipalScope {
        toolsets: toolsets.iter().copied().collect(),
        ..PrincipalScope::default()
    }
}

#[test]
fn mcp_core_full_describe_query_history_equal_rest() {
    let core = ToolCatalog::for_scope(&scope(&[Toolset::Core]));
    assert!(core.all().iter().any(|tool| tool.name == DESCRIBE_TOOL));
    assert!(core.all().iter().any(|tool| tool.name == "object_set"));
    assert!(core.all().iter().any(|tool| tool.name == "searchAround"));
    assert!(!core.all().iter().any(|tool| tool.name == AGGREGATE_TOOL));
    let full = ToolCatalog::for_scope(&scope(&[Toolset::Core, Toolset::Full]));
    assert!(full.all().iter().any(|tool| tool.name == AGGREGATE_TOOL));
    assert!(full.all().iter().any(|tool| tool.name == HISTORY_TOOL));
}

#[test]
fn mcp_list_filters_before_keyset_and_direct_call_denies() {
    let full = ToolCatalog::for_scope(&scope(&[Toolset::Full]));
    let first = full.page(None, 1);
    assert_eq!(first.tools.len(), 1);
    assert!(first.has_more);
    let second = full.page(first.next.as_deref(), 16);
    assert!(!second.tools.is_empty());
    assert!(!full.allows("admin_reload"));
    assert!(!full.allows("scope_apply"));
}

#[test]
fn mcp_inflight_snapshot_and_revoke_on_next_call() {
    let full = ToolCatalog::for_scope(&scope(&[Toolset::Full]));
    assert!(full.allows(DESCRIBE_TOOL));
    let revoked = ToolCatalog::for_scope(&scope(&[]));
    assert!(!revoked.allows(DESCRIBE_TOOL));
}

#[tokio::test]
async fn mcp_real_http_client_lists_core_tools_and_rejects_hidden_call() {
    let fixture = support::fixture().await;
    let running = fixture.start().await;
    let url = format!("{}/mcp", running.url);
    let client = reqwest::Client::new();
    let initialize = client
        .post(&url)
        .bearer_auth(&running.token)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-03-26")
        .json(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"contract","version":"0"}}}))
        .send()
        .await
        .unwrap();
    assert!(initialize.status().is_success());
    let session = initialize
        .headers()
        .get("mcp-session-id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let initialized = client
        .post(&url)
        .bearer_auth(&running.token)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-03-26")
        .header("mcp-session-id", &session)
        .json(&json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
        .send()
        .await
        .unwrap();
    assert!(initialized.status().is_success());
    let listed = client
        .post(&url)
        .bearer_auth(&running.token)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-03-26")
        .header("mcp-session-id", &session)
        .json(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}))
        .send()
        .await
        .unwrap();
    let body = listed.text().await.unwrap();
    let payload: Value = body
        .lines()
        .find_map(|line| {
            line.strip_prefix("data: ")
                .and_then(|value| serde_json::from_str(value).ok())
        })
        .or_else(|| serde_json::from_str(&body).ok())
        .unwrap_or(Value::Null);
    assert!(
        payload["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == DESCRIBE_TOOL)
    );
    let denied = client
        .post(&url)
        .bearer_auth(&running.token)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-03-26")
        .header("mcp-session-id", &session)
        .json(&json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"admin_reload","arguments":{}}}))
        .send()
        .await
        .unwrap();
    let denied_body = denied.text().await.unwrap();
    assert!(denied_body.contains("error") || denied_body.contains("tool unavailable"));
    let _ = running.finish().await;
}
