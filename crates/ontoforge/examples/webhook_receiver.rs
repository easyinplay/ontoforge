//! Independent HTTP receiver for the Phase 8 webhook end-to-end test.
//!
//! The receiver runs as its own operating system process, binds a loopback port and writes
//! one JSONL record per received request to a ledger file. It records raw facts only: it
//! asserts nothing, exposes no aggregate endpoint and never reports a delivery verdict
//! through its exit code. The verdict is taken by the test, from the ledger and from the
//! terminal state of the outbox in PostgreSQL.

use std::collections::BTreeMap;
use std::io::Write;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Bytes;
use axum::extract::{ConnectInfo, State};
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use axum::routing::post;
use clap::Parser;
use serde::Serialize;

#[derive(Debug, Parser)]
#[command(
    name = "webhook-receiver",
    about = "Independent webhook receiver for the end-to-end test"
)]
struct Args {
    /// TCP port on 127.0.0.1; 0 lets the operating system choose one.
    #[arg(long, default_value_t = 0)]
    port: u16,
    /// JSONL ledger that receives one line per request.
    #[arg(long, value_name = "FILE")]
    ledger: PathBuf,
    /// File that receives the full target URL once the listener is bound.
    #[arg(long, value_name = "FILE")]
    ready_file: PathBuf,
    /// Answer the first attempt of every event with 503 and later attempts with 204.
    #[arg(long)]
    fail_first_per_event: bool,
}

/// One received request, as the receiver saw it.
#[derive(Debug, Serialize)]
struct LedgerRecord {
    seq: u64,
    received_at: String,
    method: String,
    path: String,
    header_event_id: Option<String>,
    payload_event_id: Option<String>,
    event_id: Option<String>,
    attempt_seq: u64,
    status_code: u16,
    body_len: usize,
    remote_addr: Option<String>,
}

/// The counters the receiver keeps between requests.
#[derive(Default)]
struct ReceiverState {
    seq: u64,
    attempts: BTreeMap<String, u64>,
}

/// The state every request handler shares.
#[derive(Clone)]
struct Shared {
    ledger: PathBuf,
    fail_first_per_event: bool,
    state: Arc<Mutex<ReceiverState>>,
}

/// Answers one delivery attempt and records it.
async fn receive(
    State(shared): State<Shared>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    method: Method,
    uri: Uri,
    body: Bytes,
) -> StatusCode {
    // The header name is case insensitive on the wire and hyper sends it in lower case.
    let header_event_id = headers
        .get("x-ontoforge-event-id")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    // A body that is not JSON is recorded as a missing payload event id, never as a panic.
    let payload_event_id = serde_json::from_slice::<serde_json::Value>(&body)
        .ok()
        .and_then(|value| {
            value
                .get("event_id")
                .and_then(|id| id.as_str())
                .map(str::to_owned)
        });
    let event_id = header_event_id.clone().or_else(|| payload_event_id.clone());

    let (seq, attempt_seq, status_code) = {
        let mut state = shared.state.lock().expect("receiver state");
        state.seq += 1;
        let seq = state.seq;
        let attempt_seq = match &event_id {
            Some(id) => {
                let counter = state.attempts.entry(id.clone()).or_insert(0);
                *counter += 1;
                *counter
            }
            None => 0,
        };
        let status_code = if shared.fail_first_per_event && attempt_seq == 1 {
            503
        } else {
            204
        };
        (seq, attempt_seq, status_code)
    };

    let record = LedgerRecord {
        seq,
        received_at: chrono::Utc::now().to_rfc3339(),
        method: method.as_str().to_owned(),
        path: uri.path().to_owned(),
        header_event_id,
        payload_event_id,
        event_id,
        attempt_seq,
        status_code,
        body_len: body.len(),
        remote_addr: Some(peer.to_string()),
    };
    let mut line = serde_json::to_vec(&record).expect("ledger record");
    line.push(b'\n');
    // One unbuffered write per record, so a polling reader sees the line at once.
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&shared.ledger)
        .expect("ledger file");
    file.write_all(&line).expect("ledger write");
    file.flush().expect("ledger flush");

    StatusCode::from_u16(status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", args.port))
        .await
        .expect("bind the receiver on 127.0.0.1");
    let address = listener.local_addr().expect("receiver address");
    let target = format!("http://{address}/events");
    // The ready file is written through a temporary file so a reader never sees half a line.
    let temporary = args.ready_file.with_extension("tmp");
    std::fs::write(&temporary, target.as_bytes()).expect("write the ready file");
    std::fs::rename(&temporary, &args.ready_file).expect("publish the ready file");
    let shared = Shared {
        ledger: args.ledger,
        fail_first_per_event: args.fail_first_per_event,
        state: Arc::new(Mutex::new(ReceiverState::default())),
    };
    let app = Router::new()
        .route("/events", post(receive))
        .with_state(shared);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("serve the receiver");
}
