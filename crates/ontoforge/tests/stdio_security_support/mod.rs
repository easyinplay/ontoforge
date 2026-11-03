use std::fmt::Write as _;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};
use sqlx::{QueryBuilder, Sqlite};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Notify, mpsc, oneshot};
use tokio::task::{JoinHandle, JoinSet};

pub const DEADLINE: Duration = Duration::from_secs(10);
pub const LEGACY_VERSION: &str = "2025-03-26";

pub fn principal_token(
    fixture: &super::support::Fixture,
    subject: &str,
    groups: &[&str],
) -> String {
    use ontoforge_auth::authn::dev::{DevClaims, DevKey, issue_dev_token};
    let key_path =
        camino::Utf8PathBuf::from_path_buf(fixture.dir.path().join("token.key")).unwrap();
    let key = DevKey::load_or_create(&key_path).unwrap();
    issue_dev_token(
        &key,
        &DevClaims {
            sub: subject.to_owned(),
            groups: groups.iter().map(|group| (*group).to_owned()).collect(),
            attrs: serde_json::Map::from_iter([("airports".to_owned(), json!(["BER"]))]),
            ttl: Duration::from_secs(600),
        },
    )
    .unwrap()
}

pub async fn http_json(request: reqwest::RequestBuilder) -> Value {
    tokio::time::timeout(DEADLINE, async {
        let response = request.send().await.unwrap();
        assert!(
            response.status().is_success(),
            "HTTP status {}",
            response.status()
        );
        response.json().await.unwrap()
    })
    .await
    .expect("HTTP JSON deadline")
}

pub struct Cli {
    child: super::BridgeChild,
    frames: mpsc::UnboundedReceiver<Result<Value, String>>,
    stdout: std::thread::JoinHandle<()>,
    stderr: std::thread::JoinHandle<String>,
    token: String,
}

impl Cli {
    pub fn spawn(url: &str, token: &str) -> Self {
        let mut child = super::BridgeChild(
            Command::new(env!("CARGO_BIN_EXE_ontoforge"))
                .args(["mcp", "--server-url", url])
                .env("ONTOFORGE_TOKEN", token)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap(),
        );
        let stdout = child.0.stdout.take().unwrap();
        let mut stderr = child.0.stderr.take().unwrap();
        let (tx, frames) = mpsc::unbounded_channel();
        let stdout = std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let frame = line
                    .map_err(|e| e.to_string())
                    .and_then(|line| serde_json::from_str(&line).map_err(|e| e.to_string()));
                if tx.send(frame).is_err() {
                    break;
                }
            }
        });
        let stderr = std::thread::spawn(move || {
            let mut output = String::new();
            stderr.read_to_string(&mut output).unwrap();
            output
        });
        Self {
            child,
            frames,
            stdout,
            stderr,
            token: token.to_owned(),
        }
    }

    pub fn send(&mut self, frame: &Value) {
        writeln!(self.child.0.stdin.as_mut().unwrap(), "{frame}").unwrap();
    }

    pub async fn receive(&mut self) -> Option<Value> {
        let frame = tokio::time::timeout(DEADLINE, self.frames.recv())
            .await
            .expect("stdio response deadline")
            .map(|frame| frame.expect("every stdout line must be JSON-RPC"));
        if let Some(frame) = &frame {
            assert_eq!(frame["jsonrpc"], "2.0");
            assert!(
                !frame.to_string().contains(&self.token),
                "stdout leaked token"
            );
        }
        frame
    }

    pub async fn initialize(&mut self) {
        self.send(
            &json!({"jsonrpc":"2.0","id":"init","method":"initialize","params":{
                "protocolVersion":LEGACY_VERSION,"capabilities":{},
                "clientInfo":{"name":"bridge-security","version":"0"}
            }}),
        );
        let response = self.receive().await.expect("initialize response");
        assert_eq!(response["id"], "init");
        assert!(response.get("error").is_none(), "{response}");
        self.send(&json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
    }

    pub async fn expect_failure(&mut self, id: &Value) {
        if let Some(frame) = self.receive().await {
            assert_eq!(&frame["id"], id);
            assert!(frame.get("error").is_some(), "fault must fail: {frame}");
        }
    }

    pub async fn request(&mut self, frame: &Value) -> Value {
        self.send(frame);
        let response = self.receive().await.expect("RPC response");
        assert_eq!(response["id"], frame["id"]);
        assert!(response.get("error").is_none(), "{response}");
        response["result"].clone()
    }

    pub async fn tool(&mut self, id: u64, name: &str, arguments: Value) -> Value {
        let result = self.request(&json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":name,"arguments":arguments}})).await;
        assert_ne!(result["isError"], true, "{result}");
        result["structuredContent"].clone()
    }

    pub async fn finish(mut self, token: &str) -> std::process::ExitStatus {
        drop(self.child.0.stdin.take());
        let status = tokio::time::timeout(DEADLINE, async {
            let status = loop {
                if let Some(status) = self.child.0.try_wait().unwrap() {
                    break status;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            };
            while !self.stdout.is_finished() || !self.stderr.is_finished() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            status
        })
        .await
        .expect("bridge and pipe readers must stop within deadline");
        self.stdout.join().expect("stdout reader panic");
        let stderr = self.stderr.join().expect("stderr reader panic");
        assert!(!stderr.contains(token), "stderr leaked token");
        while let Some(frame) = self.frames.recv().await {
            let frame = frame.expect("stdout must contain only protocol JSON");
            assert_eq!(frame["jsonrpc"], "2.0");
            assert!(!frame.to_string().contains(token), "stdout leaked token");
        }
        status
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    DropBeforeForward,
    DropAfterCommit,
    Pass,
}

#[derive(Clone, Debug)]
pub struct Observation {
    pub verb: String,
    pub method: String,
    pub session: Option<String>,
    pub status: Option<u16>,
}

#[derive(Default)]
pub struct Evidence {
    pub requests: Vec<Observation>,
    pub session: Option<String>,
    pub receipt: Option<Value>,
}

pub struct Proxy {
    pub url: String,
    pub evidence: Arc<Mutex<Evidence>>,
    injected: Arc<Notify>,
    stop: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<()>>,
}

pub struct RedirectTrap {
    pub url: String,
    pub origin_requests: Arc<std::sync::atomic::AtomicUsize>,
    pub trap_connections: Arc<std::sync::atomic::AtomicUsize>,
    stop: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<()>>,
}

impl RedirectTrap {
    pub async fn start(token: String) -> Self {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let origin = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let trap = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/mcp", origin.local_addr().unwrap());
        let destination = format!("http://{}/mcp", trap.local_addr().unwrap());
        let origin_requests = Arc::new(AtomicUsize::new(0));
        let trap_connections = Arc::new(AtomicUsize::new(0));
        let hits = Arc::clone(&origin_requests);
        let leaks = Arc::clone(&trap_connections);
        let (stop, mut stopping) = oneshot::channel();
        let task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut stopping => break,
                    accepted = origin.accept() => {
                        let (mut socket, _) = accepted.unwrap();
                        tokio::time::timeout(DEADLINE, async {
                            let request = read_request(&mut socket).await.unwrap();
                            assert_eq!(request.verb, "POST");
                            assert!(request.headers.iter().any(|(key, value)| key == "authorization" && value == &format!("Bearer {token}")), "origin must receive authorization");
                            let body: Value = serde_json::from_slice(&request.body).unwrap();
                            assert_eq!(body["method"], "initialize");
                            hits.fetch_add(1, Ordering::SeqCst);
                            socket.write_all(format!("HTTP/1.1 302 Found\r\nLocation: {destination}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").as_bytes()).await.unwrap();
                        }).await.expect("redirect origin deadline");
                    }
                    accepted = trap.accept() => {
                        let (socket, _) = accepted.unwrap();
                        leaks.fetch_add(1, Ordering::SeqCst);
                        drop(socket);
                    }
                }
            }
        });
        Self {
            url,
            origin_requests,
            trap_connections,
            stop: Some(stop),
            task: Some(task),
        }
    }

    pub async fn finish(mut self) {
        let _ = self.stop.take().unwrap().send(());
        tokio::time::timeout(DEADLINE, self.task.as_mut().unwrap())
            .await
            .expect("redirect trap shutdown deadline")
            .expect("redirect trap task panic");
        self.task.take();
    }
}

impl Drop for RedirectTrap {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}

impl Proxy {
    pub async fn start(upstream: &str, mode: Mode) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/mcp", listener.local_addr().unwrap());
        let evidence = Arc::new(Mutex::new(Evidence::default()));
        let injected = Arc::new(Notify::new());
        let (stop, mut stopping) = oneshot::channel();
        let state = Arc::clone(&evidence);
        let barrier = Arc::clone(&injected);
        let upstream = format!("{upstream}/mcp");
        let task = tokio::spawn(async move {
            let client = reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .unwrap();
            let mut jobs = JoinSet::new();
            loop {
                tokio::select! {
                    _ = &mut stopping => break,
                    accepted = listener.accept() => {
                        let (stream, _) = accepted.unwrap();
                        let (client, upstream, state, barrier) =
                            (client.clone(), upstream.clone(), Arc::clone(&state), Arc::clone(&barrier));
                        jobs.spawn(async move {
                            tokio::time::timeout(DEADLINE, forward(stream, client, upstream, mode, state, barrier))
                                .await.expect("proxy connection deadline");
                        });
                    }
                    result = jobs.join_next(), if !jobs.is_empty() => {
                        result.unwrap().expect("proxy worker panic");
                    }
                }
            }
            jobs.abort_all();
            while let Some(result) = jobs.join_next().await {
                if let Err(error) = result {
                    assert!(error.is_cancelled(), "proxy panic: {error}");
                }
            }
        });
        Self {
            url,
            evidence,
            injected,
            stop: Some(stop),
            task: Some(task),
        }
    }

    pub async fn injected(&self) {
        tokio::time::timeout(DEADLINE, self.injected.notified())
            .await
            .expect("fault injection barrier");
        self.assert_running();
    }

    pub fn assert_running(&self) {
        assert!(
            !self.task.as_ref().unwrap().is_finished(),
            "proxy terminated unexpectedly"
        );
    }

    pub async fn settle(&self) {
        tokio::time::sleep(Duration::from_millis(500)).await;
        self.assert_running();
    }

    pub fn count(&self, method: &str) -> usize {
        self.evidence
            .lock()
            .unwrap()
            .requests
            .iter()
            .filter(|r| r.method == method)
            .count()
    }

    pub async fn finish(mut self) {
        let _ = self.stop.take().unwrap().send(());
        tokio::time::timeout(DEADLINE, self.task.as_mut().unwrap())
            .await
            .expect("proxy shutdown deadline")
            .expect("proxy background panic");
        self.task.take();
    }
}

impl Drop for Proxy {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}

struct HttpRequest {
    verb: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

async fn read_request(stream: &mut TcpStream) -> Option<HttpRequest> {
    let mut bytes = Vec::new();
    let end = loop {
        let mut buffer = [0; 4096];
        let read = stream.read(&mut buffer).await.unwrap();
        if read == 0 {
            return None;
        }
        bytes.extend_from_slice(&buffer[..read]);
        if let Some(index) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            break index + 4;
        }
        assert!(bytes.len() < 65536, "request header bounded");
    };
    let header = std::str::from_utf8(&bytes[..end]).unwrap();
    let mut lines = header.split("\r\n");
    let verb = lines
        .next()
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .to_owned();
    let headers: Vec<(String, String)> = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(key, value)| (key.to_ascii_lowercase(), value.trim().to_owned()))
        .collect();
    assert!(
        !headers.iter().any(|(k, _)| k == "transfer-encoding"),
        "fixture expects sized JSON"
    );
    let length = headers
        .iter()
        .find(|(k, _)| k == "content-length")
        .map_or(0, |(_, value)| value.parse::<usize>().unwrap());
    assert!(length < 1024 * 1024);
    while bytes.len() < end + length {
        let mut buffer = [0; 4096];
        let read = stream.read(&mut buffer).await.unwrap();
        assert_ne!(read, 0, "entire POST must arrive before fault");
        bytes.extend_from_slice(&buffer[..read]);
    }
    Some(HttpRequest {
        verb,
        headers,
        body: bytes[end..end + length].to_vec(),
    })
}

async fn forward(
    mut stream: TcpStream,
    client: reqwest::Client,
    upstream: String,
    mode: Mode,
    evidence: Arc<Mutex<Evidence>>,
    injected: Arc<Notify>,
) {
    let Some(HttpRequest {
        verb,
        headers,
        body,
    }) = read_request(&mut stream).await
    else {
        return;
    };
    let rpc: Value = if body.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&body).unwrap()
    };
    let method = rpc["method"].as_str().unwrap_or("").to_owned();
    let session = headers
        .iter()
        .find(|(k, _)| k == "mcp-session-id")
        .map(|(_, v)| v.clone());
    let index = {
        let mut state = evidence.lock().unwrap();
        state.requests.push(Observation {
            verb: verb.clone(),
            method: method.clone(),
            session,
            status: None,
        });
        state.requests.len() - 1
    };
    if mode == Mode::DropBeforeForward && method == "tools/call" {
        injected.notify_one();
        return;
    }
    let mut request = client.request(verb.parse::<reqwest::Method>().unwrap(), &upstream);
    for (key, value) in &headers {
        if !matches!(
            key.as_str(),
            "host" | "content-length" | "connection" | "transfer-encoding"
        ) {
            request = request.header(key, value);
        }
    }
    if !body.is_empty() {
        request = request.body(body);
    }
    let response = request.send().await.unwrap();
    let status = response.status();
    {
        let mut state = evidence.lock().unwrap();
        state.requests[index].status = Some(status.as_u16());
        if let Some(session) = response.headers().get("mcp-session-id") {
            state.session = Some(session.to_str().unwrap().to_owned());
        }
    }
    if mode == Mode::DropAfterCommit && method == "tools/call" {
        assert!(status.is_success());
        let text = response.text().await.unwrap();
        let rpc = parse_rpc(&text);
        assert!(rpc.get("error").is_none(), "{rpc}");
        let receipt = rpc["result"]["structuredContent"]["receipt"].clone();
        assert!(receipt["eventId"].is_string(), "{rpc}");
        assert!(receipt["committedAt"].is_string(), "{rpc}");
        evidence.lock().unwrap().receipt = Some(receipt);
        injected.notify_one();
        return;
    }
    relay_response(&mut stream, response).await;
}

async fn relay_response(stream: &mut TcpStream, mut response: reqwest::Response) {
    let mut head = format!(
        "HTTP/1.1 {} OK\r\nConnection: close\r\nTransfer-Encoding: chunked\r\n",
        response.status().as_u16()
    );
    for (key, value) in response.headers() {
        if !matches!(
            key.as_str(),
            "content-length" | "connection" | "transfer-encoding"
        ) {
            write!(head, "{}: {}\r\n", key, value.to_str().unwrap()).unwrap();
        }
    }
    head.push_str("\r\n");
    if stream.write_all(head.as_bytes()).await.is_err() {
        return;
    }
    while let Some(chunk) = response.chunk().await.unwrap() {
        if stream
            .write_all(format!("{:x}\r\n", chunk.len()).as_bytes())
            .await
            .is_err()
            || stream.write_all(&chunk).await.is_err()
            || stream.write_all(b"\r\n").await.is_err()
        {
            return;
        }
    }
    let _ = stream.write_all(b"0\r\n\r\n").await;
}

fn parse_rpc(text: &str) -> Value {
    serde_json::from_str(text)
        .ok()
        .or_else(|| {
            text.lines().find_map(|line| {
                line.strip_prefix("data: ")
                    .and_then(|line| serde_json::from_str(line).ok())
            })
        })
        .expect("upstream MCP JSON response")
}

pub fn action(id: u64, command: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{
        "name":"action_delayFlight","arguments":{"commandId":command,
        "params":{"flight":"F-1001","delayMinutes":5}}
    }})
}

pub async fn counts(path: &camino::Utf8Path) -> [i64; 5] {
    tokio::time::timeout(DEADLINE, counts_inner(path))
        .await
        .expect("durable counts deadline")
}

fn quoted_identifier(name: &str) -> String {
    assert!(
        !name.is_empty()
            && name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'),
        "unexpected fixture schema identifier: {name}"
    );
    format!("\"{name}\"")
}

fn quoted_table(name: &str) -> String {
    assert!(
        matches!(
            name,
            "of_edit_log"
                | "of_edit_log_entry"
                | "of_edit_history"
                | "of_action_command"
                | "of_webhook_outbox"
                | "of_obj_aircraft"
                | "of_obj_airport"
                | "of_obj_flight"
                | "of_obj_maintenance_record"
                | "of_obj_pilot"
                | "of_lnk_flight_crew"
        ),
        "unexpected aviation fixture table: {name}"
    );
    quoted_identifier(name)
}

async fn counts_inner(path: &camino::Utf8Path) -> [i64; 5] {
    let pool =
        sqlx::SqlitePool::connect_with(ontoforge_storage::store::sqlite_connect_options(path))
            .await
            .unwrap();
    let mut counts = [0; 5];
    for (index, table) in [
        "of_edit_log",
        "of_edit_log_entry",
        "of_edit_history",
        "of_action_command",
        "of_webhook_outbox",
    ]
    .iter()
    .enumerate()
    {
        let mut query = QueryBuilder::<Sqlite>::new("SELECT COUNT(*) FROM ");
        query.push(quoted_table(table));
        counts[index] = query.build_query_scalar().fetch_one(&pool).await.unwrap();
    }
    pool.close().await;
    counts
}

pub async fn durable_rows(path: &camino::Utf8Path) -> std::collections::BTreeMap<String, Value> {
    tokio::time::timeout(DEADLINE, async {
        let pool = sqlx::SqlitePool::connect_with(ontoforge_storage::store::sqlite_connect_options(path)).await.unwrap();
        let tables: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type='table' AND (name GLOB 'of_obj_*' OR name GLOB 'of_lnk_*' OR name IN ('of_edit_log', 'of_edit_log_entry', 'of_edit_history', 'of_action_command', 'of_webhook_outbox')) ORDER BY name"
        ).fetch_all(&pool).await.unwrap();
        let mut state = std::collections::BTreeMap::new();
        for table in tables {
            let columns: Vec<String> = sqlx::query_scalar("SELECT name FROM pragma_table_info(?) ORDER BY cid")
                .bind(&table).fetch_all(&pool).await.unwrap();
            let mut query = QueryBuilder::<Sqlite>::new(
                "SELECT json_group_array(json(row_json)) FROM (SELECT json_object(",
            );
            for (index, name) in columns.into_iter().enumerate() {
                if index > 0 {
                    query.push(", ");
                }
                let identifier = quoted_identifier(&name);
                query.push_bind(name).push(", CAST(").push(identifier).push(" AS TEXT)");
            }
            query.push(") AS row_json FROM ").push(quoted_table(&table)).push(" ORDER BY rowid)");
            let encoded: String = query.build_query_scalar().fetch_one(&pool).await.unwrap();
            state.insert(table, serde_json::from_str(&encoded).unwrap());
        }
        pool.close().await;
        state
    }).await.expect("durable row snapshot deadline")
}
