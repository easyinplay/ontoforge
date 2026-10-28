use axum::{
    Router,
    extract::{Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::Response,
};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use rmcp::{RoleServer, ServerHandler, model::*, service::RequestContext};
use serde_json::json;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct Principal(pub String);

#[derive(Clone, Default)]
pub struct Gate(Arc<Mutex<HashMap<String, String>>>);

async fn auth(
    State(gate): State<Gate>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let principal = match req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
    {
        Some("Bearer alice") => "alice",
        Some("Bearer bob") => "bob",
        _ => return Err(StatusCode::UNAUTHORIZED),
    }
    .to_owned();
    if let Some(sid) = req
        .headers()
        .get("mcp-session-id")
        .and_then(|v| v.to_str().ok())
    {
        if gate.0.lock().unwrap().get(sid) != Some(&principal) {
            return Err(StatusCode::FORBIDDEN);
        }
    }
    req.extensions_mut().insert(Principal(principal.clone()));
    let response = next.run(req).await;
    if let Some(sid) = response
        .headers()
        .get("mcp-session-id")
        .and_then(|v| v.to_str().ok())
    {
        gate.0.lock().unwrap().insert(sid.to_owned(), principal);
    }
    Ok(response)
}

#[derive(Clone, Default)]
pub struct Dynamic {
    started: CancellationToken,
    finished: CancellationToken,
}

fn principal(context: &RequestContext<RoleServer>) -> Result<&Principal, ErrorData> {
    context
        .extensions
        .get::<axum::http::request::Parts>()
        .and_then(|parts| parts.extensions.get::<Principal>())
        .ok_or_else(|| ErrorData::invalid_request("authentication required", None))
}

fn tools_for(who: &str) -> Vec<Tool> {
    let mut tools = Vec::new();
    if who == "alice" {
        // Runtime metadata fixture, rather than a tool macro or Rust input type.
        let action = ("launch", "flightId", "string");
        let mut properties = serde_json::Map::new();
        properties.insert(action.1.to_owned(), json!({"type":action.2}));
        properties.insert(
            "validateOnly".to_owned(),
            json!({"type":"boolean","default":false}),
        );
        let schema = json!({"type":"object","properties":properties,"required":[action.1],"additionalProperties":false});
        tools.push(Tool::new(
            format!("action_{}", action.0),
            "Runtime Action",
            schema.as_object().unwrap().clone(),
        ));
        tools.push(Tool::new(
            "slow",
            "Cancellation fixture",
            json!({"type":"object"}).as_object().unwrap().clone(),
        ));
    }
    tools.push(Tool::new(
        "describe",
        "Scoped metadata",
        json!({"type":"object"}).as_object().unwrap().clone(),
    ));
    tools
}

impl ServerHandler for Dynamic {
    fn get_info(&self) -> ServerConfig {
        let mut info = ServerConfig::default();
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info
    }

    async fn list_tools(
        &self,
        _: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let mut result = ListToolsResult::default();
        result.tools = tools_for(&principal(&context)?.0);
        Ok(result)
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let who = principal(&context)?.0.clone();
        if !tools_for(&who).iter().any(|tool| tool.name == request.name) {
            return Err(ErrorData::invalid_request("tool unavailable", None));
        }
        if request.name == "slow" {
            let _finished = self.finished.clone().drop_guard();
            self.started.cancel();
            tokio::select! {
                _ = context.ct.cancelled() => return Ok(CallToolResult::structured_error(json!({"code":"CANCELLED"})).into()),
                _ = tokio::time::sleep(Duration::from_secs(60)) => {}
            }
        }
        Ok(CallToolResult::structured(json!({"principal":who})).into())
    }
}

pub async fn bridge<R, W>(
    read: R,
    write: W,
    url: &str,
    token: String,
    stop: CancellationToken,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
    W: tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    use rmcp::transport::{
        Transport,
        async_rw::AsyncRwTransport,
        streamable_http_client::{
            StreamableHttpClientTransport, StreamableHttpClientTransportConfig,
        },
    };
    let config = StreamableHttpClientTransportConfig::with_uri(url.to_owned())
        .auth_header(token)
        .reinit_on_expired_session(false)
        .max_concurrent_requests(16)
        .control_request_timeout(Duration::from_secs(2));
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let mut remote = StreamableHttpClientTransport::with_client(client, config);
    let mut local = AsyncRwTransport::new_server(read, write).with_max_line_length(4096);
    let mut sends = tokio::task::JoinSet::new();
    let outcome: Result<(), Box<dyn std::error::Error + Send + Sync>> = async {
        loop {
            tokio::select! {
                _ = stop.cancelled() => break,
                incoming = local.receive() => match incoming {
                    Some(message) => {
                        if sends.len() >= 64 { return Err("bridge send limit".into()); }
                        let send = remote.send(message);
                        sends.spawn(send);
                    }
                    None => break,
                },
                incoming = remote.receive() => match incoming {
                    Some(message) => local.send(message).await?,
                    None => break,
                },
                done = sends.join_next(), if !sends.is_empty() => { done.unwrap()??; }
            }
        }
        Ok(())
    }
    .await;
    let closed = tokio::time::timeout(Duration::from_secs(3), remote.close()).await;
    sends.abort_all();
    while sends.join_next().await.is_some() {}
    local.close().await?;
    closed??;
    outcome
}

pub async fn start() -> (String, CancellationToken, tokio::task::JoinHandle<()>) {
    let (url, stop, task, _, _) = start_with_signals().await;
    (url, stop, task)
}

pub async fn start_with_signals() -> (
    String,
    CancellationToken,
    tokio::task::JoinHandle<()>,
    CancellationToken,
    CancellationToken,
) {
    let stop = CancellationToken::new();
    let dynamic = Dynamic::default();
    let started = dynamic.started.clone();
    let finished = dynamic.finished.clone();
    let config = StreamableHttpServerConfig::default()
        .with_cancellation_token(stop.child_token())
        .with_max_request_body_bytes(4096);
    let svc = StreamableHttpService::new(
        move || Ok(dynamic.clone()),
        Arc::new(LocalSessionManager::default()),
        config,
    );
    let app = Router::new()
        .nest_service("/mcp", svc)
        .layer(middleware::from_fn_with_state(Gate::default(), auth));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/mcp", listener.local_addr().unwrap());
    let stopping = stop.clone();
    let task = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(stopping.cancelled_owned())
            .await
            .unwrap();
    });
    (url, stop, task, started, finished)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    async fn rpc(
        url: &str,
        who: &str,
        sid: Option<&str>,
        request: Value,
    ) -> (StatusCode, Option<String>, Value) {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        let version = request
            .pointer("/params/_meta/io.modelcontextprotocol~1protocolVersion")
            .and_then(Value::as_str)
            .unwrap_or("2025-03-26");
        let mut req = client
            .post(url)
            .bearer_auth(who)
            .header("accept", "application/json, text/event-stream")
            .header("mcp-protocol-version", version)
            .json(&request);
        if version >= "2026-07-28" {
            req = req.header("mcp-method", request["method"].as_str().unwrap());
        }
        if let Some(sid) = sid {
            req = req.header("mcp-session-id", sid);
        }
        let response = req.send().await.unwrap();
        let status = response.status();
        let sid = response
            .headers()
            .get("mcp-session-id")
            .map(|v| v.to_str().unwrap().to_owned());
        let body = response.text().await.unwrap();
        let body = body
            .lines()
            .find_map(|line| {
                line.strip_prefix("data: ")
                    .and_then(|v| serde_json::from_str(v).ok())
            })
            .or_else(|| serde_json::from_str(&body).ok())
            .unwrap_or(Value::Null);
        (status, sid, body)
    }

    async fn initialize(url: &str, who: &str) -> String {
        let (status, sid, body) = rpc(url, who, None, json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"spike","version":"0"}}})).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let sid = sid.expect("legacy session");
        let (status, _, _) = rpc(
            url,
            who,
            Some(&sid),
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        )
        .await;
        assert!(status.is_success());
        sid
    }

    #[tokio::test]
    async fn dynamic_tools_are_principal_scoped_and_schema_is_runtime_data() {
        let (url, stop, task) = start().await;
        let alice = initialize(&url, "alice").await;
        let bob = initialize(&url, "bob").await;
        let (_, _, a) = rpc(
            &url,
            "alice",
            Some(&alice),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
        )
        .await;
        let (_, _, b) = rpc(
            &url,
            "bob",
            Some(&bob),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
        )
        .await;
        assert_eq!(a["result"]["tools"][0]["name"], "action_launch");
        assert_eq!(
            a["result"]["tools"][0]["inputSchema"]["properties"]["flightId"]["type"],
            "string"
        );
        assert_eq!(b["result"]["tools"][0]["name"], "describe");
        assert_eq!(b["result"]["tools"].as_array().unwrap().len(), 1);
        stop.cancel();
        tokio::time::timeout(Duration::from_secs(3), task)
            .await
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn direct_unlisted_tool_and_cross_principal_session_are_denied() {
        let (url, stop, task) = start().await;
        let alice = initialize(&url, "alice").await;
        let bob = initialize(&url, "bob").await;
        let (_, _, denied) = rpc(&url, "bob", Some(&bob), json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"action_launch","arguments":{"flightId":"F1"}}})).await;
        assert!(denied.get("error").is_some(), "{denied}");
        let (status, _, _) = rpc(
            &url,
            "bob",
            Some(&alice),
            json!({"jsonrpc":"2.0","id":4,"method":"tools/list"}),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        let (status, _, _) = rpc(
            &url,
            "expired",
            Some(&alice),
            json!({"jsonrpc":"2.0","id":4,"method":"tools/list"}),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        stop.cancel();
        tokio::time::timeout(Duration::from_secs(3), task)
            .await
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn modern_requests_use_per_request_principal_without_session() {
        let (url, stop, task) = start().await;
        let request = json!({"jsonrpc":"2.0","id":8,"method":"tools/list","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{},"io.modelcontextprotocol/clientInfo":{"name":"modern-spike","version":"0"}}}});
        let (status, sid, a) = rpc(&url, "alice", None, request.clone()).await;
        assert_eq!(status, StatusCode::OK, "{a}");
        assert!(sid.is_none());
        assert_eq!(a["result"]["tools"][0]["name"], "action_launch");
        let (status, sid, b) = rpc(&url, "bob", None, request).await;
        assert_eq!(status, StatusCode::OK, "{b}");
        assert!(sid.is_none());
        assert_eq!(b["result"]["tools"].as_array().unwrap().len(), 1);
        stop.cancel();
        tokio::time::timeout(Duration::from_secs(3), task)
            .await
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn http_cancel_notification_stops_running_handler_and_shutdown_is_bounded() {
        let (url, stop, task, started, finished) = start_with_signals().await;
        let sid = initialize(&url, "alice").await;
        let call_url = url.clone();
        let call_sid = sid.clone();
        let slow = tokio::spawn(async move {
            rpc(&call_url, "alice", Some(&call_sid), json!({"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"slow","arguments":{}}})).await
        });
        tokio::time::timeout(Duration::from_secs(3), started.cancelled())
            .await
            .unwrap();
        let (status, _, _) = rpc(&url, "alice", Some(&sid), json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":9,"reason":"test"}})).await;
        assert!(status.is_success());
        tokio::time::timeout(Duration::from_secs(3), finished.cancelled())
            .await
            .unwrap();
        let _ = tokio::time::timeout(Duration::from_secs(3), slow)
            .await
            .unwrap()
            .unwrap();
        stop.cancel();
        tokio::time::timeout(Duration::from_secs(3), task)
            .await
            .unwrap()
            .unwrap();
    }
}
