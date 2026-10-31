use ontoforge_auth::scope::PrincipalScope;
use ontoforge_schema::ir::Toolset;
use ontoforge_server::mcp_catalog::{AGGREGATE_TOOL, DESCRIBE_TOOL, HISTORY_TOOL, ToolCatalog};
use serde_json::{Value, json};

mod support;

async fn call_read_tool(
    running: &support::Running,
    session: &str,
    id: u64,
    name: &str,
    arguments: Value,
) -> Value {
    let body = running.client
        .post(format!("{}/mcp", running.url))
        .bearer_auth(&running.token)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-03-26")
        .header("mcp-session-id", session)
        .json(&json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":name,"arguments":arguments}}))
        .send().await.expect("MCP read request")
        .text().await.expect("MCP read response");
    body.lines()
        .find_map(|line| {
            line.strip_prefix("data: ")
                .and_then(|data| serde_json::from_str(data).ok())
        })
        .or_else(|| serde_json::from_str(&body).ok())
        .expect("MCP JSON response")
}

async fn full_read_fixture() -> support::Fixture {
    use camino::Utf8PathBuf;
    use ontoforge_auth::authn::dev::{DevClaims, DevKey, issue_dev_token};
    let mut fixture = support::fixture().await;
    let key_path = Utf8PathBuf::from_path_buf(fixture.dir.path().join("token.key")).unwrap();
    let key = DevKey::load_or_create(&key_path).unwrap();
    fixture.token = issue_dev_token(
        &key,
        &DevClaims {
            sub: "alice".to_owned(),
            groups: vec!["dispatchers".to_owned(), "crew-schedulers".to_owned()],
            attrs: serde_json::Map::from_iter([("airports".to_owned(), json!(["BER"]))]),
            ttl: std::time::Duration::from_secs(600),
        },
    )
    .unwrap();
    fixture
}

#[tokio::test]
async fn mcp_read_tools_use_declared_argument_shapes() {
    let fixture = full_read_fixture().await;
    let running = fixture.start().await;
    let initialized = running.client.post(format!("{}/mcp", running.url))
        .bearer_auth(&running.token)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-03-26")
        .json(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"read-contract","version":"0"}}}))
        .send().await.unwrap();
    let session = initialized.headers()["mcp-session-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let notification = running
        .client
        .post(format!("{}/mcp", running.url))
        .bearer_auth(&running.token)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-03-26")
        .header("mcp-session-id", &session)
        .json(&json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
        .send()
        .await
        .unwrap();
    assert!(notification.status().is_success());
    let described = call_read_tool(&running, &session, 2, "describe", json!({})).await;
    assert!(
        described["error"].is_null(),
        "describe accepts its empty argument schema: {described}"
    );
    let types = described["result"]["structuredContent"]["objectTypes"]
        .as_array()
        .expect("describe object types");
    assert!(types.iter().any(|ty| ty["apiName"] == "Airport"));
    assert!(!types.iter().any(|ty| ty["apiName"] == "Aircraft"));
    for (id, name) in [(3, "query"), (4, "object_set"), (5, "searchAround")] {
        let payload = call_read_tool(
            &running,
            &session,
            id,
            name,
            json!({"request":{
                "set":{"kind":"type","object_type":"Airport"},"select":["name"],"page":{"first":3}
            }}),
        )
        .await;
        assert!(
            payload["error"].is_null(),
            "{name} accepts request-only arguments: {payload}"
        );
        assert_eq!(
            payload["result"]["structuredContent"]["rows"]
                .as_array()
                .unwrap()
                .len(),
            3
        );
    }
    let aggregated = call_read_tool(
        &running,
        &session,
        6,
        "aggregate",
        json!({"request":{
            "set":{"kind":"type","object_type":"Airport"},"measures":[{"op":"count"}]
        }}),
    )
    .await;
    assert!(
        aggregated["error"].is_null(),
        "aggregate accepts request-only arguments: {aggregated}"
    );
    assert_eq!(
        aggregated["result"]["structuredContent"]["buckets"][0]["values"][0],
        3
    );
    let denied = call_read_tool(&running, &session, 7, "query", json!({"request":{
        "set":{"kind":"type","object_type":"Aircraft"},"select":["tailNumber"],"page":{"first":1}
    }})).await;
    assert!(
        denied["error"].is_object(),
        "ungranted types remain hidden: {denied}"
    );
    let missing = call_read_tool(&running, &session, 8, "get_object", json!({"pk":"BER"})).await;
    assert!(
        missing["error"].is_object(),
        "get_object still requires objectType: {missing}"
    );
    let audit = running.finish().await;
    assert_eq!(
        audit.len(),
        4,
        "three list calls and one aggregate must be audited once each"
    );
    assert!(audit.iter().all(|row| row["status"] == "success"));
}

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
