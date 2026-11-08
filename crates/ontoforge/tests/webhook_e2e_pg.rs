//! End-to-end webhook delivery over PostgreSQL against an independent receiver process.
//!
//! The test is `#[ignore]`d; the Linux CI job `phase8-webhook-e2e` runs it with `--ignored`.
//! Without `ONTOFORGE_PG_URL` or `ONTOFORGE_WEBHOOK_RECEIVER_BIN` it panics instead of
//! skipping silently, so a green run always means the case really ran.
//!
//! The receiver runs as its own operating system process and records every request it
//! receives. The verdict is taken here, from the receiver's raw ledger and from the
//! terminal state of `of_webhook_outbox` in PostgreSQL; the receiver itself asserts
//! nothing and reports no verdict of its own.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus};
use std::str::FromStr;
use std::time::{Duration, Instant};

use ontoforge_action::{ActionItem, ActionLimits, ActionOutcome, ActionRequest, ActionService};
use ontoforge_auth::{
    Principal, PrincipalId,
    scope::{PrincipalScope, TypeScope},
};
use ontoforge_engine::config::{StoreConfig, StoreKind};
use ontoforge_expr::{Attrs, Pred};
use ontoforge_query::TokenCodec;
use ontoforge_schema::load_from_memory;
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::store::{EditStore, StoreSettings};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

const DEFS: &str = "kind: ObjectType
apiName: Gate
backing: editOnly
primaryKey: code
properties:
  code: { type: string, required: true }
  name: { type: string, required: true }
---
kind: ActionType
apiName: createGate
parameters:
  code: { type: string, required: true }
  name: { type: string, required: true }
rules:
  - kind: createObject
    objectType: Gate
    properties:
      code: { param: code }
      name: { param: name }
";

/// The lower bound of the retry gap: the backoff of the first attempt is one second.
const MIN_RETRY_GAP_MS: i64 = 900;
/// The upper bound of the retry gap: a liveness guard, not a delivery requirement.
const MAX_RETRY_GAP_MS: i64 = 30_000;

/// One received request, as the receiver recorded it.
#[derive(Debug, serde::Deserialize)]
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

/// The terminal state of one outbox row.
#[derive(Debug, sqlx::FromRow)]
struct OutboxRow {
    event_id: String,
    attempt_count: i32,
    delivered_at: Option<chrono::DateTime<chrono::Utc>>,
    last_error: Option<String>,
}

/// A child process that is killed and reaped when it goes out of scope.
struct Process(Child);

impl Process {
    fn id(&self) -> u32 {
        self.0.id()
    }

    fn try_wait(&mut self) -> Option<ExitStatus> {
        self.0.try_wait().expect("child process status")
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// The directory that receives the ledger and the process logs.
fn evidence_dir() -> PathBuf {
    let dir = std::env::var("ONTOFORGE_WEBHOOK_EVIDENCE_DIR").unwrap_or_else(|_| {
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/phase8-webhook").to_owned()
    });
    std::fs::create_dir_all(&dir).expect("evidence directory");
    PathBuf::from(dir)
}

/// The store settings of the CLI worker: the same `[store]` resolution the server uses.
fn pg_store_settings(schema_edit: &str, schema_sys: &str) -> StoreSettings {
    StoreSettings::from_config(
        &StoreConfig {
            kind: StoreKind::Postgres,
            path: None,
            url_env: Some("ONTOFORGE_PG_URL".to_owned()),
            url_file: None,
            schema_edit: schema_edit.to_owned(),
            schema_sys: schema_sys.to_owned(),
            lock_timeout_ms: 5_000,
            probe_timeout_ms: 60_000,
            trino: None,
        },
        camino::Utf8Path::new("."),
        &|name| std::env::var(name).ok(),
    )
    .expect("store settings")
}

/// Submits one Action per pair of command identifier and primary key and returns the event
/// identifiers of the committed outbox rows.
async fn committed_event_ids(store: &EditStore, submissions: &[(&str, &str)]) -> Vec<String> {
    let loaded = load_from_memory(&[("ontology.yaml", "apiName: demo\n"), ("defs.yaml", DEFS)]);
    assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
    let version = reload(
        store,
        &loaded.ontology.expect("ontology"),
        &ReloadOptions::default(),
    )
    .await
    .expect("reload")
    .def_version
    .expect("definition version");
    let principal = Principal {
        id: PrincipalId::new("test", "alice"),
        issuer: "test".to_owned(),
        subject: "alice".to_owned(),
        groups: BTreeSet::new(),
        attrs: Attrs::default(),
    };
    let mut scope = PrincipalScope::default();
    scope
        .actions
        .insert("createGate".to_owned().try_into().expect("action"));
    scope.types.insert(
        "Gate".to_owned().try_into().expect("type"),
        TypeScope {
            row: Pred::Const(true),
            columns: ["code", "name"]
                .into_iter()
                .map(|name| {
                    (
                        name.to_owned().try_into().expect("column"),
                        Pred::Const(true),
                    )
                })
                .collect(),
        },
    );
    let tokens = TokenCodec::new(b"phase8-webhook-test-key".to_vec()).expect("tokens");
    let service = ActionService::new(
        store,
        ActionLimits::new(8, 8, 65536).expect("limits"),
        &tokens,
    );
    let mut event_ids = Vec::new();
    for (command_id, code) in submissions {
        let outcome = service
            .execute(
                &principal,
                &scope,
                version,
                ActionRequest {
                    action: "createGate".to_owned(),
                    command_id: Some((*command_id).to_owned()),
                    items: vec![ActionItem {
                        params: serde_json::from_value(serde_json::json!({
                            "code": code,
                            "name": "Private Name",
                        }))
                        .expect("params"),
                        expected_versions: BTreeMap::new(),
                    }],
                    validate_only: false,
                },
            )
            .await
            .expect("Action submit");
        let ActionOutcome::Committed(receipt) = outcome else {
            panic!("expected committed Action")
        };
        event_ids.push(receipt.event_id);
    }
    event_ids
}

/// Writes the trusted server configuration of the worker.
fn write_server_config(dir: &Path, target: &str, schema_edit: &str, schema_sys: &str) -> PathBuf {
    let path = dir.join("ontoforge-server.toml");
    std::fs::write(
        &path,
        format!(
            "[store]\nkind = \"postgres\"\nurl_env = \"ONTOFORGE_PG_URL\"\nschema_edit = \"{schema_edit}\"\nschema_sys = \"{schema_sys}\"\n\n[webhook]\ntarget = \"{target}\"\npoll_interval_ms = 200\nrequest_timeout_ms = 1000\nlease_duration_ms = 5000\nbatch_limit = 10\n"
        ),
    )
    .expect("server configuration");
    path
}

/// Starts the receiver as an independent process on a loopback port of the operating
/// system's choosing.
fn spawn_receiver(binary: &str, dir: &Path) -> Process {
    Process(
        Command::new(binary)
            .arg("--port")
            .arg("0")
            .arg("--ledger")
            .arg(dir.join("ledger.jsonl"))
            .arg("--ready-file")
            .arg(dir.join("receiver.ready"))
            .arg("--fail-first-per-event")
            .stdout(File::create(dir.join("receiver.log")).expect("receiver log"))
            .stderr(File::create(dir.join("receiver.err.log")).expect("receiver error log"))
            .spawn()
            .expect("spawn the receiver"),
    )
}

/// Starts the real `ontoforge webhook-worker` binary against the generated configuration.
fn spawn_worker(config: &Path, url: &str, dir: &Path) -> Process {
    Process(
        Command::new(env!("CARGO_BIN_EXE_ontoforge"))
            .args([
                "webhook-worker",
                "--server-config",
                config.to_str().expect("utf8 configuration path"),
            ])
            .env("ONTOFORGE_PG_URL", url)
            .stdout(File::create(dir.join("worker.log")).expect("worker log"))
            .stderr(File::create(dir.join("worker.err.log")).expect("worker error log"))
            .spawn()
            .expect("spawn the webhook worker"),
    )
}

/// Waits until the receiver publishes the URL it listens on.
async fn wait_for_ready_file(path: &Path, dir: &Path) -> String {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if let Ok(text) = std::fs::read_to_string(path)
            && !text.trim().is_empty()
        {
            return text.trim().to_owned();
        }
        if Instant::now() >= deadline {
            let stderr = std::fs::read_to_string(dir.join("receiver.err.log")).unwrap_or_default();
            panic!("the receiver did not publish its target URL: {stderr}");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// Reads the raw ledger. Only complete lines are evidence; a line that is still in flight
/// is left out.
fn read_ledger(path: &Path) -> Vec<LedgerRecord> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut lines: Vec<&str> = text.lines().collect();
    if !text.ends_with('\n') {
        lines.pop();
    }
    lines
        .into_iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("ledger record"))
        .collect()
}

/// The records of one event, in the order the receiver wrote them.
fn ledger_records_for<'a>(records: &'a [LedgerRecord], event_id: &str) -> Vec<&'a LedgerRecord> {
    records
        .iter()
        .filter(|record| record.event_id.as_deref() == Some(event_id))
        .collect()
}

/// Reads the terminal state of the outbox from PostgreSQL.
async fn outbox_rows(pool: &sqlx::PgPool, schema_sys: &str) -> Vec<OutboxRow> {
    let sql = format!(
        "SELECT event_id, attempt_count, delivered_at, last_error FROM \"{schema_sys}\".of_webhook_outbox"
    );
    sqlx::query_as::<_, OutboxRow>(sqlx::AssertSqlSafe(sql))
        .fetch_all(pool)
        .await
        .expect("outbox rows")
}

/// Waits until the ledger holds a successful delivery for every submitted event.
async fn wait_for_delivery(
    worker: &mut Process,
    ledger: &Path,
    dir: &Path,
    submitted: &[String],
    deadline: Duration,
) -> Vec<LedgerRecord> {
    let started = Instant::now();
    loop {
        let records = read_ledger(ledger);
        let delivered = submitted.iter().all(|event_id| {
            ledger_records_for(&records, event_id)
                .iter()
                .any(|record| (200..300).contains(&record.status_code))
        });
        if delivered {
            return records;
        }
        if let Some(status) = worker.try_wait() {
            let stderr = std::fs::read_to_string(dir.join("worker.err.log")).unwrap_or_default();
            panic!("the webhook worker exited early with {status}: {stderr}");
        }
        if started.elapsed() >= deadline {
            let stderr = std::fs::read_to_string(dir.join("worker.err.log")).unwrap_or_default();
            let message = if records.is_empty() {
                format!("the receiver recorded no requests at all: {stderr}")
            } else {
                format!(
                    "the receiver recorded no successful delivery for every submitted event: {stderr}"
                )
            };
            panic!("{message}");
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Waits until every outbox row of the submitted events reached its terminal state.
async fn wait_for_outbox(
    pool: &sqlx::PgPool,
    schema_sys: &str,
    submitted: &[String],
    deadline: Duration,
) -> Vec<OutboxRow> {
    let started = Instant::now();
    loop {
        let rows = outbox_rows(pool, schema_sys).await;
        let settled = submitted.iter().all(|event_id| {
            rows.iter()
                .any(|row| &row.event_id == event_id && row.delivered_at.is_some())
        });
        if settled {
            return rows;
        }
        if started.elapsed() >= deadline {
            let rows = outbox_rows(pool, schema_sys).await;
            panic!("the outbox rows did not reach a terminal state: {rows:?}");
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// The milliseconds between two consecutive deliveries of the same event.
fn gaps_between(records: &[&LedgerRecord]) -> Vec<i64> {
    records
        .windows(2)
        .map(|pair| {
            let first = chrono::DateTime::parse_from_rfc3339(&pair[0].received_at)
                .expect("first timestamp");
            let second = chrono::DateTime::parse_from_rfc3339(&pair[1].received_at)
                .expect("second timestamp");
            (second - first).num_milliseconds()
        })
        .collect()
}

/// Asserts what the receiver recorded for one event.
fn assert_ledger_evidence(records: &[LedgerRecord], event_id: &str) {
    for pair in records.windows(2) {
        assert!(
            pair[1].seq > pair[0].seq,
            "the receiver numbers its records in order: {pair:?}"
        );
    }
    let delivered = ledger_records_for(records, event_id);
    assert!(
        delivered.len() >= 2,
        "the event must be attempted at least twice: {delivered:?}"
    );
    assert_eq!(
        delivered[0].status_code, 503,
        "the first attempt of the event must fail with 503: {delivered:?}"
    );
    let last = delivered.last().expect("the last attempt of the event");
    assert!(
        (200..300).contains(&last.status_code),
        "the last attempt of the event must succeed: {delivered:?}"
    );
    for gap in gaps_between(&delivered) {
        assert!(
            (MIN_RETRY_GAP_MS..=MAX_RETRY_GAP_MS).contains(&gap),
            "the retry must follow the backoff of one second: {gap} ms in {delivered:?}"
        );
    }
    for (index, record) in delivered.iter().enumerate() {
        assert_eq!(record.method, "POST", "the worker posts: {record:?}");
        assert_eq!(
            record.path, "/events",
            "the worker posts to /events: {record:?}"
        );
        assert!(
            record.body_len > 0,
            "the request carries a payload: {record:?}"
        );
        assert_eq!(
            record.attempt_seq,
            u64::try_from(index + 1).expect("attempt number"),
            "the attempt counter grows per event: {record:?}"
        );
        assert_eq!(
            record.header_event_id.as_deref(),
            Some(event_id),
            "the request header must carry the event id: {record:?}"
        );
        assert_eq!(
            record.payload_event_id.as_deref(),
            Some(event_id),
            "the payload must carry the event id: {record:?}"
        );
        assert_eq!(
            record.event_id.as_deref(),
            Some(event_id),
            "the recorded event id must be the submitted one: {record:?}"
        );
        assert!(
            record
                .remote_addr
                .as_deref()
                .is_some_and(|address| address.starts_with("127.0.0.1:")),
            "the receiver must see a loopback peer: {record:?}"
        );
    }
}

/// Asserts what PostgreSQL recorded for one event.
fn assert_outbox_evidence(rows: &[OutboxRow], event_id: &str) {
    let row = rows
        .iter()
        .find(|row| row.event_id == event_id)
        .expect("the outbox row of the event");
    assert!(
        row.delivered_at.is_some(),
        "the outbox row must be delivered: {row:?}"
    );
    assert_eq!(
        row.attempt_count, 1,
        "the outbox row must record one failed attempt: {row:?}"
    );
    assert_eq!(
        row.last_error.as_deref(),
        Some("http_status_503"),
        "the outbox row must record the safe failure category: {row:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs ONTOFORGE_PG_URL and ONTOFORGE_WEBHOOK_RECEIVER_BIN"]
async fn webhook_worker_delivers_to_an_independent_receiver_over_postgres() {
    let url = std::env::var("ONTOFORGE_PG_URL").expect("ONTOFORGE_PG_URL must be set");
    let receiver_bin = std::env::var("ONTOFORGE_WEBHOOK_RECEIVER_BIN")
        .expect("ONTOFORGE_WEBHOOK_RECEIVER_BIN must point at the built example");
    let dir = evidence_dir();
    let unique = ulid::Ulid::generate().to_string().to_lowercase();
    let schema_edit = format!("of_t_{unique}_edit");
    let schema_sys = format!("of_t_{unique}_sys");
    let settings = pg_store_settings(&schema_edit, &schema_sys);
    let store = EditStore::open(settings)
        .await
        .expect("open the PostgreSQL store");
    let submitted = committed_event_ids(&store, &[("phase8-webhook-command-1", "G1")]).await;
    let event_id = submitted[0].clone();

    let receiver = spawn_receiver(&receiver_bin, &dir);
    let target = wait_for_ready_file(&dir.join("receiver.ready"), &dir).await;
    assert!(
        target.starts_with("http://127.0.0.1:"),
        "the receiver must listen on the loopback address: {target}"
    );
    let config = write_server_config(&dir, &target, &schema_edit, &schema_sys);
    let mut worker = spawn_worker(&config, &url, &dir);

    let records = wait_for_delivery(
        &mut worker,
        &dir.join("ledger.jsonl"),
        &dir,
        &submitted,
        Duration::from_secs(60),
    )
    .await;
    assert_ledger_evidence(&records, &event_id);
    assert_ne!(
        receiver.id(),
        std::process::id(),
        "the receiver must run as an independent process"
    );
    assert!(
        worker.try_wait().is_none(),
        "the worker must keep polling after the delivery"
    );

    let Ok(options) = PgConnectOptions::from_str(&url) else {
        panic!("the PostgreSQL connection string is invalid");
    };
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect_with(options)
        .await
        .expect("connect to PostgreSQL");
    let rows = wait_for_outbox(&pool, &schema_sys, &submitted, Duration::from_secs(30)).await;
    assert_eq!(
        rows.len(),
        1,
        "the outbox must hold exactly one row: {rows:?}"
    );
    assert_outbox_evidence(&rows, &event_id);
}
