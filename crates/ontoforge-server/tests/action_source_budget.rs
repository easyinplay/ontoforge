#[path = "support/mod.rs"]
mod support;

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ontoforge_engine::{
    Engine, EngineError, EngineStatus, PhysicalColumn, PrintedSql, QueryContext, QueryOutput,
    TableRef,
};
use ontoforge_server::{HostError, ServerHost, ServerLimits};
use reqwest::{Client, StatusCode};
use serde_json::{Value, json};
use sqlx::Row;
use tokio::io::AsyncWriteExt;
use tokio::sync::{Notify, Semaphore, oneshot};

struct BudgetEngine {
    inner: Arc<support::ControlledEngine>,
    block: AtomicBool,
    entered: Notify,
    release: Semaphore,
    contexts: Mutex<Vec<QueryContext>>,
}

impl Engine for BudgetEngine {
    fn dialect(&self) -> ontoforge_engine::Dialect {
        self.inner.dialect()
    }

    async fn execute(
        &self,
        sql: &PrintedSql,
        ctx: &QueryContext,
    ) -> Result<QueryOutput, EngineError> {
        self.contexts.lock().unwrap().push(ctx.clone());
        self.entered.notify_one();
        tokio::time::timeout(ctx.timeout, async {
            if self.block.load(Ordering::SeqCst) {
                self.release.acquire().await.unwrap().forget();
            }
            self.inner.execute(sql, ctx).await
        })
        .await
        .unwrap_or_else(|_| {
            Err(EngineError::Timeout {
                after_ms: u64::try_from(ctx.timeout.as_millis()).unwrap(),
            })
        })
    }

    async fn describe_tables(
        &self,
        tables: &[TableRef],
        ctx: &QueryContext,
    ) -> Result<BTreeMap<TableRef, Vec<PhysicalColumn>>, EngineError> {
        self.inner.describe_tables(tables, ctx).await
    }

    async fn status(&self) -> EngineStatus {
        self.inner.status().await
    }
}

struct Running {
    url: String,
    token: String,
    path: camino::Utf8PathBuf,
    engine: Arc<BudgetEngine>,
    host: Arc<ServerHost<BudgetEngine>>,
    client: Client,
    stop: oneshot::Sender<()>,
    task: tokio::task::JoinHandle<Result<(), HostError>>,
    _dir: tempfile::TempDir,
}

fn limits() -> ServerLimits {
    ServerLimits {
        deadline: Duration::from_millis(200),
        max_body_bytes: 4096,
        execution: ontoforge_query::ExecutionLimits {
            max_raw_candidates: 1,
            max_decoded_bytes: 32,
            ..ontoforge_query::ExecutionLimits::phase4_default()
        },
        ..ServerLimits::default()
    }
}

async fn start(block: bool) -> Running {
    let fixture = support::fixture().await;
    let engine = Arc::new(BudgetEngine {
        inner: fixture.engine,
        block: AtomicBool::new(block),
        entered: Notify::new(),
        release: Semaphore::new(0),
        contexts: Mutex::new(Vec::new()),
    });
    let original = fixture.host;
    let host = Arc::new(ServerHost {
        store: original.store,
        engine: Arc::clone(&engine),
        auth: original.auth,
        auth_config: original.auth_config,
        tokens: original.tokens,
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (stop, rx) = oneshot::channel();
    let task = tokio::spawn(Arc::clone(&host).serve_with_limits(
        listener,
        async {
            let _ = rx.await;
        },
        limits(),
    ));
    let client = Client::new();
    assert_eq!(
        client
            .get(format!("{url}/health"))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    Running {
        url,
        token: fixture.token,
        path: fixture.path,
        engine,
        host,
        client,
        stop,
        task,
        _dir: fixture.dir,
    }
}

async fn initialize(running: &Running) -> String {
    let response = running.client.post(format!("{}/mcp", running.url)).bearer_auth(&running.token)
        .header("accept", "application/json, text/event-stream")
        .json(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"action-budget","version":"0"}}})).send().await.unwrap();
    let session = response.headers()["mcp-session-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let response = mcp_request(running, &session)
        .json(&json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
        .send()
        .await
        .unwrap();
    assert!(response.status().is_success());
    session
}

fn mcp_request(running: &Running, session: &str) -> reqwest::RequestBuilder {
    running
        .client
        .post(format!("{}/mcp", running.url))
        .bearer_auth(&running.token)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-03-26")
        .header("mcp-session-id", session)
}

fn action_body(command: &str, validate_only: bool) -> Value {
    json!({"commandId":command,"validateOnly":validate_only,"items":[{"params":{"flight":"F-1001","delayMinutes":15}}]})
}

fn mcp_body(command: &str, validate_only: bool) -> Value {
    json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"action_delayFlight","arguments":{"commandId":command,"validateOnly":validate_only,"params":{"flight":"F-1001","delayMinutes":15}}}})
}

fn decode_mcp(body: &str) -> Value {
    body.lines()
        .find_map(|line| {
            line.strip_prefix("data: ")
                .and_then(|data| serde_json::from_str(data).ok())
        })
        .or_else(|| serde_json::from_str(body).ok())
        .expect("MCP JSON response")
}

fn assert_domain_context(ctx: &QueryContext) {
    assert_eq!(
        ctx.timeout,
        Duration::from_millis(ontoforge_engine::DEFAULT_QUERY_TIMEOUT_MS)
    );
    assert_eq!(
        ctx.max_rows, 3,
        "Action keeps duplicate-PK detection capacity"
    );
    assert_eq!(ctx.max_bytes, limits().max_body_bytes);
}

async fn assert_durable_once(path: &camino::Utf8Path, command: &str, receipt: &Value) {
    let pool =
        sqlx::SqlitePool::connect_with(ontoforge_storage::store::sqlite_connect_options(path))
            .await
            .unwrap();
    let rows = sqlx::query("SELECT event_id, receipt FROM of_action_command WHERE command_id = ?")
        .bind(command)
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get::<String, _>("event_id"), receipt["eventId"]);
    let stored: Value = serde_json::from_str(&rows[0].get::<String, _>("receipt")).unwrap();
    assert_eq!(stored["event_id"], receipt["eventId"]);
    assert_eq!(stored["committed_at"], receipt["committedAt"]);
    let logs: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM of_edit_log WHERE action = 'delayFlight'")
            .fetch_one(&pool)
            .await
            .unwrap();
    let edits: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM of_obj_flight WHERE of_version = 1")
        .fetch_one(&pool)
        .await
        .unwrap();
    let outbox: Vec<String> = sqlx::query_scalar("SELECT payload FROM of_webhook_outbox")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(logs, 1);
    assert_eq!(edits, 1);
    assert_eq!(outbox.len(), 1);
    let event: Value = serde_json::from_str(&outbox[0]).unwrap();
    assert_eq!(event["event_id"], receipt["eventId"]);
    assert_eq!(event["receipt"]["command_id"], command);
    pool.close().await;
}

#[derive(Clone, Copy)]
enum Transport {
    Rest,
    Mcp,
}

async fn open_socket_request(
    running: &Running,
    path: &str,
    session: Option<&str>,
    body: &Value,
) -> tokio::net::TcpStream {
    let mut stream = tokio::net::TcpStream::connect(running.url.strip_prefix("http://").unwrap())
        .await
        .unwrap();
    let metadata = session.map_or(String::new(), |session| format!("Mcp-Session-Id: {session}\r\nMcp-Protocol-Version: 2025-03-26\r\nAccept: application/json, text/event-stream\r\n"));
    let body = body.to_string();
    stream.write_all(format!("POST {path} HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {}\r\n{metadata}Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", running.token, body.len()).as_bytes()).await.unwrap();
    stream
}

async fn submitted_action_survives(transport: Transport, disconnect: bool) {
    let running = start(true).await;
    let session = if matches!(transport, Transport::Mcp) {
        Some(initialize(&running).await)
    } else {
        None
    };
    let command = "action-owned-budget";
    let (path, body) = match transport {
        Transport::Rest => ("/api/actions/delayFlight", action_body(command, false)),
        Transport::Mcp => ("/mcp", mcp_body(command, false)),
    };
    let mut socket = None;
    let mut request = None;
    if disconnect {
        socket = Some(open_socket_request(&running, path, session.as_deref(), &body).await);
    } else {
        let builder = match session.as_deref() {
            Some(session) => mcp_request(&running, session).timeout(limits().deadline),
            None => running
                .client
                .post(format!("{}{path}", running.url))
                .bearer_auth(&running.token),
        }
        .json(&body);
        request = Some(tokio::spawn(async move {
            let response = builder.send().await?;
            let status = response.status();
            Ok::<_, reqwest::Error>((status, response.text().await?))
        }));
    }
    tokio::time::timeout(Duration::from_secs(3), running.engine.entered.notified())
        .await
        .unwrap();
    if let Some(mut stream) = socket {
        stream.shutdown().await.unwrap();
        drop(stream);
    }
    let response = if let Some(request) = request {
        Some(request.await.unwrap())
    } else {
        None
    };
    tokio::time::sleep(limits().deadline * 2).await;
    running.engine.block.store(false, Ordering::SeqCst);
    running.engine.release.add_permits(1);
    tokio::time::timeout(
        Duration::from_secs(3),
        ontoforge_server::context::wait_actions(&running.host),
    )
    .await
    .unwrap()
    .unwrap();
    let receipt_response = running
        .client
        .get(format!("{}/api/commands/{command}", running.url))
        .bearer_auth(&running.token)
        .send()
        .await
        .unwrap();
    let receipt_status = receipt_response.status();
    let receipt: Value = receipt_response.json().await.unwrap();
    let replay = running
        .client
        .post(format!("{}/api/actions/delayFlight", running.url))
        .bearer_auth(&running.token)
        .json(&action_body(command, false))
        .send()
        .await
        .unwrap();
    let replay_status = replay.status();
    let replay_body: Value = replay.json().await.unwrap();
    running.stop.send(()).unwrap();
    running.task.await.unwrap().unwrap();
    assert_eq!(receipt_status, StatusCode::OK, "{receipt}");
    assert_eq!(replay_status, StatusCode::OK, "{replay_body}");
    assert_eq!(receipt, replay_body);
    if let Some(response) = response {
        match transport {
            Transport::Rest => assert_eq!(response.unwrap().0, StatusCode::GATEWAY_TIMEOUT),
            Transport::Mcp => assert!(
                matches!(&response, Err(error) if error.is_timeout())
                    || matches!(&response, Ok((StatusCode::GATEWAY_TIMEOUT, _)))
            ),
        }
    }
    let contexts = running.engine.contexts.lock().unwrap().clone();
    assert_eq!(
        contexts.len(),
        1,
        "explicit replay must use the original receipt"
    );
    assert_domain_context(&contexts[0]);
    assert_durable_once(&running.path, command, &receipt).await;
}

#[tokio::test]
async fn rest_action_http_deadline_keeps_domain_source_budget_and_one_receipt() {
    submitted_action_survives(Transport::Rest, false).await;
}
#[tokio::test]
async fn rest_action_disconnect_keeps_domain_source_budget_and_one_receipt() {
    submitted_action_survives(Transport::Rest, true).await;
}
#[tokio::test]
async fn mcp_action_http_deadline_keeps_domain_source_budget_and_one_receipt() {
    submitted_action_survives(Transport::Mcp, false).await;
}
#[tokio::test]
async fn mcp_action_disconnect_keeps_domain_source_budget_and_one_receipt() {
    submitted_action_survives(Transport::Mcp, true).await;
}

#[tokio::test]
async fn validate_only_preserves_domain_rows_bytes_while_query_stays_limited() {
    let running = start(false).await;
    let rest = running
        .client
        .post(format!("{}/api/actions/delayFlight", running.url))
        .bearer_auth(&running.token)
        .json(&action_body("validate-budget", true))
        .send()
        .await
        .unwrap();
    let rest_status = rest.status();
    let rest_body: Value = rest.json().await.unwrap();
    let session = initialize(&running).await;
    let mcp = mcp_request(&running, &session)
        .json(&mcp_body("validate-budget", true))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let mcp_body = decode_mcp(&mcp);
    let query = running
        .client
        .get(format!("{}/api/objects/Flight/F-1001", running.url))
        .bearer_auth(&running.token)
        .send()
        .await
        .unwrap();
    let query_status = query.status();
    let query_body: Value = query.json().await.unwrap();
    running.stop.send(()).unwrap();
    running.task.await.unwrap().unwrap();
    assert_eq!(rest_status, StatusCode::OK, "{rest_body}");
    assert_eq!(rest_body["validated"], true);
    assert_eq!(
        mcp_body["result"]["structuredContent"]["validated"], true,
        "{mcp_body}"
    );
    assert_eq!(query_status, StatusCode::BAD_REQUEST);
    assert_eq!(query_body["code"], "COST_EXCEEDED");
    let contexts = running.engine.contexts.lock().unwrap().clone();
    assert_eq!(contexts.len(), 3);
    assert_domain_context(&contexts[0]);
    assert_domain_context(&contexts[1]);
    assert_eq!(contexts[2].timeout, limits().deadline);
    assert_eq!(contexts[2].max_rows, 1);
    assert_eq!(contexts[2].max_bytes, 32);
    let pool = sqlx::SqlitePool::connect_with(ontoforge_storage::store::sqlite_connect_options(
        &running.path,
    ))
    .await
    .unwrap();
    let commands: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM of_action_command")
        .fetch_one(&pool)
        .await
        .unwrap();
    let outbox: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM of_webhook_outbox")
        .fetch_one(&pool)
        .await
        .unwrap();
    let action_logs: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM of_edit_log WHERE action = 'delayFlight'")
            .fetch_one(&pool)
            .await
            .unwrap();
    let edits: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM of_obj_flight")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(commands, 0);
    assert_eq!(outbox, 0);
    assert_eq!(action_logs, 0);
    assert_eq!(edits, 0);
    pool.close().await;
}
