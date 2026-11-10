//! Crash-recovery supervision of `ontoforge webhook-worker` over PostgreSQL.
//!
//! Both tests are `#[ignore]`d; the Linux CI job `phase9-webhook-supervision` runs them
//! with `--ignored`. Missing prerequisites panic instead of skipping silently, so a
//! green run always means the case really ran.
//!
//! `killed_worker_recovers_undelivered_event_after_respawn` kills a worker with SIGKILL
//! while the receiver holds the first delivery behind its `--hold-first-request`
//! barrier, then proves that a respawned worker recovers the still-leased outbox row
//! and delivers the event. The test process itself plays the supervisor: the observed
//! signal, the respawn and the redelivery are recorded in `supervisor.jsonl`.
//!
//! `systemd_restarts_killed_worker_and_redelivers` (Linux only) installs the shipped
//! unit on a real systemd PID 1, SIGKILLs the service `MainPID` and observes systemd's
//! own restart (incremented `NRestarts`, changed `InvocationID`) before the event is
//! redelivered. The test never asks systemd for a start or restart after the kill; the
//! restart can only come from the unit's `Restart=on-failure`.

#![cfg(unix)]

use std::collections::BTreeMap;
use std::fs::File;
use std::os::unix::process::ExitStatusExt;
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

/// The supervision state of one outbox row.
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
struct OutboxRow {
    event_id: String,
    attempt_count: i32,
    next_attempt_at: chrono::DateTime<chrono::Utc>,
    lease_owner: Option<String>,
    lease_until: Option<chrono::DateTime<chrono::Utc>>,
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

    /// Sends SIGKILL and reaps the child; the returned status carries the signal.
    fn kill_and_wait(&mut self) -> ExitStatus {
        self.0.kill().expect("kill the child process");
        self.0.wait().expect("reap the child process")
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// The root evidence directory: `harness/` and `systemd/` subdirectories keep the two
/// cases isolated from each other and from the phase8 `target/phase8-webhook` output.
fn evidence_dir() -> PathBuf {
    let dir = std::env::var("ONTOFORGE_WEBHOOK_EVIDENCE_DIR").unwrap_or_else(|_| {
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/phase9-webhook").to_owned()
    });
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

/// Submits one Action per pair of command identifier and primary key and returns the
/// event identifiers of the committed outbox rows.
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
        groups: std::collections::BTreeSet::new(),
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
    let tokens = TokenCodec::new(b"phase9-webhook-test-key".to_vec()).expect("tokens");
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

/// Writes the trusted server configuration of the worker. The request timeout sits well
/// below the lease so a killed worker always leaves a live lease behind, and the batch
/// holds one event so the in-flight delivery is unambiguous.
fn write_server_config(dir: &Path, target: &str, schema_edit: &str, schema_sys: &str) -> PathBuf {
    let path = dir.join("ontoforge-server.toml");
    std::fs::write(
        &path,
        format!(
            "[store]\nkind = \"postgres\"\nurl_env = \"ONTOFORGE_PG_URL\"\nschema_edit = \"{schema_edit}\"\nschema_sys = \"{schema_sys}\"\n\n[webhook]\ntarget = \"{target}\"\npoll_interval_ms = 100\nrequest_timeout_ms = 4000\nlease_duration_ms = 5000\nbatch_limit = 1\n"
        ),
    )
    .expect("server configuration");
    path
}

/// Starts the receiver as an independent process on a loopback port of the operating
/// system's choosing, with the first-request barrier held on `hold_dir`.
fn spawn_receiver(binary: &str, dir: &Path, hold_dir: &Path) -> Process {
    Process(
        Command::new(binary)
            .arg("--port")
            .arg("0")
            .arg("--ledger")
            .arg(dir.join("ledger.jsonl"))
            .arg("--ready-file")
            .arg(dir.join("receiver.ready"))
            .arg("--hold-first-request")
            .arg(hold_dir)
            .stdout(File::create(dir.join("receiver.log")).expect("receiver log"))
            .stderr(File::create(dir.join("receiver.err.log")).expect("receiver error log"))
            .spawn()
            .expect("spawn the receiver"),
    )
}

/// Starts the real `ontoforge webhook-worker` binary against the generated configuration.
fn spawn_worker(config: &Path, url: &str, dir: &Path, name: &str) -> Process {
    Process(
        Command::new(env!("CARGO_BIN_EXE_ontoforge"))
            .args([
                "webhook-worker",
                "--server-config",
                config.to_str().expect("utf8 configuration path"),
            ])
            .env("ONTOFORGE_PG_URL", url)
            .stdout(File::create(dir.join(format!("{name}.log"))).expect("worker log"))
            .stderr(File::create(dir.join(format!("{name}.err.log"))).expect("worker error log"))
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

/// The raw lines of the ledger, as the receiver wrote them.
fn ledger_lines(path: &Path) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(str::to_owned)
        .collect()
}

/// The password of a PostgreSQL connection string of the form
/// `postgres://<user>:<password>@<host>:<port>/<database>`.
fn password_of(url: &str) -> Option<&str> {
    let after_scheme = url.split_once("://").map(|(_, rest)| rest)?;
    let (userinfo, _authority) = after_scheme.rsplit_once('@')?;
    let (_user, password) = userinfo.split_once(':')?;
    Some(password)
}

/// Asserts what the receiver recorded for one event: every record is a POST to
/// `/events` carrying the event id in both the header and the payload, numbered in
/// order, from a loopback peer.
fn assert_event_evidence(records: &[LedgerRecord], event_id: &str) {
    let delivered = ledger_records_for(records, event_id);
    for pair in delivered.windows(2) {
        assert!(
            pair[1].seq > pair[0].seq,
            "the receiver numbers its records in order: {pair:?}"
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
        chrono::DateTime::parse_from_rfc3339(&record.received_at).expect("received_at is RFC 3339");
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
        assert!(
            record
                .remote_addr
                .as_deref()
                .is_some_and(|address| address.starts_with("127.0.0.1:")),
            "the receiver must see a loopback peer: {record:?}"
        );
    }
}

/// Asserts that the ledger holds no credential of the job.
fn assert_ledger_has_no_credential(lines: &[String], url: &str) {
    let password = password_of(url);
    for line in lines {
        assert!(
            !line.contains(url),
            "the ledger must not hold the connection string: {line}"
        );
        if let Some(password) = password {
            assert!(
                !line.contains(password),
                "the ledger must not hold the database password: {line}"
            );
        }
    }
}

/// Reads the supervision state of the outbox from PostgreSQL.
async fn outbox_rows(pool: &sqlx::PgPool, schema_sys: &str) -> Vec<OutboxRow> {
    let sql = format!(
        "SELECT event_id, attempt_count, next_attempt_at, lease_owner, lease_until, delivered_at, last_error FROM \"{schema_sys}\".of_webhook_outbox"
    );
    sqlx::query_as::<_, OutboxRow>(sqlx::AssertSqlSafe(sql))
        .fetch_all(pool)
        .await
        .expect("outbox rows")
}

/// Snapshots the outbox rows into the evidence directory.
fn write_outbox_snapshot(dir: &Path, name: &str, rows: &[OutboxRow]) {
    let text = serde_json::to_string_pretty(rows).expect("outbox snapshot");
    std::fs::write(dir.join(name), format!("{text}\n")).expect("write outbox snapshot");
}

/// The failure message for a worker process that exited before its delivery was done.
fn worker_exit_reason(dir: &Path, name: &str, status: ExitStatus) -> String {
    let stderr = std::fs::read_to_string(dir.join(format!("{name}.err.log"))).unwrap_or_default();
    format!("the webhook worker {name} exited early with {status}: {stderr}")
}

/// Waits until the ledger satisfies `done`. `liveness` returns `Some(reason)` as soon as
/// the process under test is known to be gone; a `None` liveness means the deadline is
/// the only failure signal (used where systemd itself reaps and restarts the worker).
async fn wait_for_ledger(
    ledger: &Path,
    deadline: Duration,
    mut liveness: impl FnMut() -> Option<String>,
    done: impl Fn(&[LedgerRecord]) -> bool,
) -> Vec<LedgerRecord> {
    let started = Instant::now();
    loop {
        let records = read_ledger(ledger);
        if done(&records) {
            return records;
        }
        if let Some(reason) = liveness() {
            panic!("{reason}");
        }
        assert!(
            started.elapsed() < deadline,
            "the receiver ledger {} did not reach the expected state within {deadline:?}: {records:?}",
            ledger.display()
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Waits until every outbox row of the submitted events reached its delivered state.
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
            panic!("the outbox rows did not reach a delivered state: {rows:?}");
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Appends one supervision event to `supervisor.jsonl` in the evidence directory.
fn record_supervision(dir: &Path, record: &serde_json::Value) {
    use std::io::Write;
    let mut line = serde_json::to_vec(record).expect("supervision record");
    line.push(b'\n');
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("supervisor.jsonl"))
        .expect("supervisor ledger");
    file.write_all(&line).expect("supervision write");
    file.flush().expect("supervision flush");
}

/// Connects to PostgreSQL for direct outbox inspection.
async fn pg_pool(url: &str) -> sqlx::PgPool {
    let Ok(options) = PgConnectOptions::from_str(url) else {
        panic!("the PostgreSQL connection string is invalid");
    };
    PgPoolOptions::new()
        .max_connections(2)
        .connect_with(options)
        .await
        .expect("connect to PostgreSQL")
}

/// Provisions one isolated schema pair, commits `commands` and returns their event
/// identifiers together with a live pool for outbox inspection.
async fn provision(
    url: &str,
    commands: &[(&str, &str)],
) -> (String, String, Vec<String>, sqlx::PgPool) {
    let unique = ulid::Ulid::generate().to_string().to_lowercase();
    let schema_edit = format!("of_t_{unique}_edit");
    let schema_sys = format!("of_t_{unique}_sys");
    let settings = pg_store_settings(&schema_edit, &schema_sys);
    let store = EditStore::open(settings)
        .await
        .expect("open the PostgreSQL store");
    let submitted = committed_event_ids(&store, commands).await;
    let pool = pg_pool(url).await;
    (schema_edit, schema_sys, submitted, pool)
}

const COMMAND_IDS: &[(&str, &str)] = &[("phase9-webhook-command-1", "G1")];

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs ONTOFORGE_PG_URL and ONTOFORGE_WEBHOOK_RECEIVER_BIN"]
#[allow(
    clippy::too_many_lines,
    reason = "the kill-recovery orchestration reads as one linear script"
)]
async fn killed_worker_recovers_undelivered_event_after_respawn() {
    let url = std::env::var("ONTOFORGE_PG_URL").expect("ONTOFORGE_PG_URL must be set");
    let receiver_bin = std::env::var("ONTOFORGE_WEBHOOK_RECEIVER_BIN")
        .expect("ONTOFORGE_WEBHOOK_RECEIVER_BIN must point at the built example");
    let dir = evidence_dir().join("harness");
    std::fs::create_dir_all(&dir).expect("harness evidence directory");
    let (schema_edit, schema_sys, submitted, pool) = provision(&url, COMMAND_IDS).await;
    let [event_id] = submitted.as_slice() else {
        panic!("expected exactly one submitted event")
    };
    let event_id = event_id.clone();

    let barrier = dir.join("barrier");
    std::fs::create_dir_all(&barrier).expect("barrier directory");
    let receiver = spawn_receiver(&receiver_bin, &dir, &barrier);
    let target = wait_for_ready_file(&dir.join("receiver.ready"), &dir).await;
    assert!(
        target.starts_with("http://127.0.0.1:"),
        "the receiver must listen on the loopback address: {target}"
    );
    let config = write_server_config(&dir, &target, &schema_edit, &schema_sys);
    let mut first_worker = spawn_worker(&config, &url, &dir, "worker1");

    // The worker claims the event and posts it; the receiver records the request and
    // then holds the response behind the barrier, so the worker is killed mid-dispatch.
    let ledger_path = dir.join("ledger.jsonl");
    wait_for_ledger(
        &ledger_path,
        Duration::from_secs(60),
        || {
            first_worker
                .try_wait()
                .map(|status| worker_exit_reason(&dir, "worker1", status))
        },
        |records| !ledger_records_for(records, &event_id).is_empty(),
    )
    .await;

    let old_pid = first_worker.id();
    let status = first_worker.kill_and_wait();
    assert_eq!(
        status.signal(),
        Some(9),
        "the worker must have been terminated by SIGKILL: {status}"
    );

    // The kill left the delivery unacknowledged: no delivered_at and the dead worker's
    // lease still held. If the row looks otherwise the barrier never held and the test
    // did not exercise the target scenario.
    let rows = outbox_rows(&pool, &schema_sys).await;
    write_outbox_snapshot(&dir, "outbox-after-kill.json", &rows);
    let row = rows
        .iter()
        .find(|row| row.event_id == event_id)
        .expect("the outbox row of the event");
    assert!(
        row.delivered_at.is_none(),
        "the fixture missed mid-dispatch kill: the event was already delivered {row:?}"
    );
    assert!(
        row.lease_owner.is_some()
            && row
                .lease_until
                .is_some_and(|until| until > chrono::Utc::now()),
        "the fixture missed mid-dispatch kill: no live lease remains {row:?}"
    );

    // Release the barrier (the response now drains into the dead connection) and let a
    // fresh worker process recover the event once the dead lease expires.
    std::fs::write(barrier.join("release"), b"").expect("release the receiver barrier");
    let mut respawned_worker = spawn_worker(&config, &url, &dir, "worker2");
    record_supervision(
        &dir,
        &serde_json::json!({
            "case": "sigkill_mid_dispatch",
            "supervisor": "test-process",
            "event_id": event_id,
            "old_pid": old_pid,
            "observed_signal": status.signal(),
            "new_pid": respawned_worker.id(),
        }),
    );

    let records = wait_for_ledger(
        &ledger_path,
        Duration::from_secs(60),
        || {
            respawned_worker
                .try_wait()
                .map(|status| worker_exit_reason(&dir, "worker2", status))
        },
        |records| {
            ledger_records_for(records, &event_id)
                .iter()
                .any(|record| record.attempt_seq >= 2 && (200..300).contains(&record.status_code))
        },
    )
    .await;
    let delivered = ledger_records_for(&records, &event_id);
    assert!(
        delivered.len() >= 2,
        "the recovered event must be attempted more than once: {delivered:?}"
    );
    assert_event_evidence(&records, &event_id);
    assert_ne!(
        receiver.id(),
        std::process::id(),
        "the receiver must run as an independent process"
    );
    assert!(
        respawned_worker.try_wait().is_none(),
        "the respawned worker must keep polling after the redelivery"
    );
    assert_ledger_has_no_credential(&ledger_lines(&ledger_path), &url);

    let rows = wait_for_outbox(&pool, &schema_sys, &submitted, Duration::from_secs(30)).await;
    write_outbox_snapshot(&dir, "outbox-final.json", &rows);
    let row = rows
        .iter()
        .find(|row| row.event_id == event_id)
        .expect("the outbox row of the event");
    assert!(
        row.delivered_at.is_some(),
        "the recovered outbox row must be delivered: {row:?}"
    );
}

/// Runs `sudo -n <args>` and panics when it fails; `stderr` is attached to the panic.
#[cfg(target_os = "linux")]
fn sudo_run(args: &[&str]) {
    let output = Command::new("sudo")
        .arg("-n")
        .args(args)
        .output()
        .expect("spawn sudo");
    assert!(
        output.status.success(),
        "sudo -n {} failed with {}: {}",
        args.join(" "),
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Runs `sudo -n <args>` and returns stdout; panics on failure.
#[cfg(target_os = "linux")]
fn sudo_output(args: &[&str]) -> String {
    let output = Command::new("sudo")
        .arg("-n")
        .args(args)
        .output()
        .expect("spawn sudo");
    assert!(
        output.status.success(),
        "sudo -n {} failed with {}: {}",
        args.join(" "),
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf8 command output")
}

/// Runs a command and returns stdout; panics on failure.
#[cfg(target_os = "linux")]
fn command_output(program: &str, args: &[&str]) -> String {
    let output = Command::new(program)
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("spawn {program}: {error}"));
    assert!(
        output.status.success(),
        "{program} {} failed with {}: {}",
        args.join(" "),
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf8 command output")
}

/// The selected `systemctl show` properties of a unit, as a key/value map.
#[cfg(target_os = "linux")]
fn systemctl_show(unit: &str, keys: &[&str]) -> BTreeMap<String, String> {
    let mut args = vec!["show", unit];
    for key in keys {
        args.push("-p");
        args.push(key);
    }
    let text = command_output("systemctl", &args);
    text.lines()
        .filter_map(|line| {
            let (key, value) = line.split_once('=')?;
            Some((key.to_owned(), value.to_owned()))
        })
        .collect()
}

/// Removes the installed unit, configuration, binary and service account, whatever the
/// test outcome was.
#[cfg(target_os = "linux")]
struct UnitCleanup;

#[cfg(target_os = "linux")]
impl Drop for UnitCleanup {
    fn drop(&mut self) {
        let cleanup: &[&[&str]] = &[
            &["systemctl", "stop", "ontoforge-webhook-worker.service"],
            &["systemctl", "disable", "ontoforge-webhook-worker.service"],
            &["systemctl", "unset-environment", "ONTOFORGE_PG_URL"],
            &[
                "rm",
                "-f",
                "/etc/systemd/system/ontoforge-webhook-worker.service",
            ],
            &["rm", "-rf", "/etc/ontoforge"],
            &["rm", "-f", "/usr/local/bin/ontoforge"],
            &["systemctl", "daemon-reload"],
            &["userdel", "ontoforge"],
        ];
        for args in cleanup {
            let _ = Command::new("sudo").arg("-n").args(*args).status();
        }
    }
}

/// Polls `systemctl show` until systemd restarts the unit after the kill: the restart
/// counter must grow and the invocation id must change. No start or restart command is
/// ever issued here, so the observed transition can only come from `Restart=on-failure`.
#[cfg(target_os = "linux")]
async fn wait_for_systemd_restart(
    unit: &str,
    previous: &BTreeMap<String, String>,
    deadline: Duration,
) -> BTreeMap<String, String> {
    let previous_restarts: u64 = previous
        .get("NRestarts")
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    let previous_invocation = previous.get("InvocationID").cloned().unwrap_or_default();
    let wait_begin = Instant::now();
    loop {
        let current = systemctl_show(
            unit,
            &[
                "NRestarts",
                "InvocationID",
                "MainPID",
                "ActiveState",
                "SubState",
                "ExecMainStatus",
                "Result",
            ],
        );
        let restarts: u64 = current
            .get("NRestarts")
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        let invocation = current.get("InvocationID").cloned().unwrap_or_default();
        if restarts > previous_restarts
            && !invocation.is_empty()
            && invocation != previous_invocation
        {
            return current;
        }
        assert!(
            wait_begin.elapsed() < deadline,
            "systemd did not restart {unit} within {deadline:?}: {current:?}"
        );
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

#[cfg(target_os = "linux")]
#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs systemd as PID 1, passwordless sudo, ONTOFORGE_PG_URL and ONTOFORGE_WEBHOOK_RECEIVER_BIN"]
#[allow(
    clippy::too_many_lines,
    reason = "the install-kill-observe-cleanup orchestration reads as one linear script"
)]
async fn systemd_restarts_killed_worker_and_redelivers() {
    let url = std::env::var("ONTOFORGE_PG_URL").expect("ONTOFORGE_PG_URL must be set");
    let receiver_bin = std::env::var("ONTOFORGE_WEBHOOK_RECEIVER_BIN")
        .expect("ONTOFORGE_WEBHOOK_RECEIVER_BIN must point at the built example");

    // Infrastructure preconditions: a failure here means the host cannot run this case,
    // never that the case passed.
    let pid1 = std::fs::read_to_string("/proc/1/comm").unwrap_or_default();
    assert_eq!(
        pid1.trim(),
        "systemd",
        "PID 1 must be systemd for the systemd supervision case"
    );
    assert!(
        Command::new("systemctl")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success()),
        "systemctl must be available"
    );
    assert!(
        Command::new("sudo")
            .args(["-n", "true"])
            .status()
            .is_ok_and(|status| status.success()),
        "passwordless sudo is required (sudo -n true)"
    );

    let dir = evidence_dir().join("systemd");
    std::fs::create_dir_all(&dir).expect("systemd evidence directory");
    let test_begin = chrono::Utc::now();
    let (schema_edit, schema_sys, submitted, pool) = provision(&url, COMMAND_IDS).await;
    let [event_id] = submitted.as_slice() else {
        panic!("expected exactly one submitted event")
    };
    let event_id = event_id.clone();

    let barrier = dir.join("barrier");
    std::fs::create_dir_all(&barrier).expect("barrier directory");
    let _receiver = spawn_receiver(&receiver_bin, &dir, &barrier);
    let target = wait_for_ready_file(&dir.join("receiver.ready"), &dir).await;
    assert!(
        target.starts_with("http://127.0.0.1:"),
        "the receiver must listen on the loopback address: {target}"
    );

    // The cleanup guard is armed before the first host mutation.
    let _cleanup = UnitCleanup;

    // Dedicated system account.
    let user_exists = Command::new("id")
        .args(["-u", "ontoforge"])
        .status()
        .is_ok_and(|status| status.success());
    if !user_exists {
        sudo_run(&[
            "useradd",
            "--system",
            "--user-group",
            "--shell",
            "/usr/sbin/nologin",
            "--no-create-home",
            "ontoforge",
        ]);
        assert!(
            Command::new("id")
                .args(["-u", "ontoforge"])
                .status()
                .is_ok_and(|status| status.success()),
            "the ontoforge system account was not created"
        );
    }

    // The unit hardening makes workspace paths unreachable for the service, so the unit,
    // the binary and the configuration are installed at their real locations.
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root");
    let unit_source = repo_root.join("deploy/systemd/ontoforge-webhook-worker.service");
    assert!(
        unit_source.is_file(),
        "the shipped unit file must exist: {unit_source:?}"
    );
    sudo_run(&[
        "install",
        "-m",
        "0644",
        "-o",
        "root",
        "-g",
        "root",
        unit_source.to_str().expect("utf8 unit path"),
        "/etc/systemd/system/ontoforge-webhook-worker.service",
    ]);
    sudo_run(&[
        "install",
        "-m",
        "0755",
        "-o",
        "root",
        "-g",
        "root",
        env!("CARGO_BIN_EXE_ontoforge"),
        "/usr/local/bin/ontoforge",
    ]);
    sudo_run(&[
        "install",
        "-d",
        "-m",
        "0750",
        "-o",
        "root",
        "-g",
        "ontoforge",
        "/etc/ontoforge",
    ]);
    let config = write_server_config(&dir, &target, &schema_edit, &schema_sys);
    sudo_run(&[
        "install",
        "-m",
        "0640",
        "-o",
        "root",
        "-g",
        "ontoforge",
        config.to_str().expect("utf8 configuration path"),
        "/etc/ontoforge/ontoforge-server.toml",
    ]);

    let verify_out = Command::new("sudo")
        .args([
            "-n",
            "systemd-analyze",
            "verify",
            "/etc/systemd/system/ontoforge-webhook-worker.service",
        ])
        .output()
        .expect("spawn systemd-analyze");
    let verify_dump = format!(
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&verify_out.stdout),
        String::from_utf8_lossy(&verify_out.stderr)
    );
    std::fs::write(dir.join("systemd-analyze-verify.txt"), &verify_dump)
        .expect("write systemd-analyze evidence");
    assert!(
        verify_out.status.success(),
        "systemd-analyze verify rejected the unit: {verify_dump}"
    );

    // The service resolves the store through ONTOFORGE_PG_URL; injecting it into the
    // manager environment keeps the connection string out of every installed file.
    let pg_env = format!("ONTOFORGE_PG_URL={url}");
    sudo_run(&["systemctl", "set-environment", pg_env.as_str()]);
    sudo_run(&["systemctl", "daemon-reload"]);
    sudo_run(&[
        "systemctl",
        "enable",
        "--now",
        "ontoforge-webhook-worker.service",
    ]);
    std::fs::write(
        dir.join("systemctl-cat.txt"),
        sudo_output(&["systemctl", "cat", "ontoforge-webhook-worker.service"]),
    )
    .expect("write systemctl cat evidence");

    // Wait for the service to report an active state with a live MainPID.
    let wait_begin = Instant::now();
    let initial_props = loop {
        let current = systemctl_show(
            "ontoforge-webhook-worker.service",
            &[
                "NRestarts",
                "InvocationID",
                "MainPID",
                "ActiveState",
                "SubState",
            ],
        );
        let main_pid: u64 = current
            .get("MainPID")
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        if current.get("ActiveState").is_some_and(|s| s == "active") && main_pid > 0 {
            break current;
        }
        if wait_begin.elapsed() >= Duration::from_secs(60) {
            let journal = sudo_output(&[
                "journalctl",
                "-u",
                "ontoforge-webhook-worker.service",
                "-b",
                "--no-pager",
            ]);
            panic!("the service did not become active within 60s: {current:?}\n{journal}");
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    };
    let initial_dump = initial_props
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(
        dir.join("systemctl-show-initial.txt"),
        format!("{initial_dump}\n"),
    )
    .expect("write initial systemctl show evidence");
    let main_pid = initial_props
        .get("MainPID")
        .cloned()
        .expect("service MainPID");

    // The service posts the event; the receiver records it and holds the response.
    let ledger_path = dir.join("ledger.jsonl");
    wait_for_ledger(
        &ledger_path,
        Duration::from_secs(60),
        || None,
        |records| !ledger_records_for(records, &event_id).is_empty(),
    )
    .await;

    // SIGKILL the service MainPID, then only observe: the restart must come from
    // systemd's Restart=on-failure, never from a test-issued start or restart.
    sudo_run(&["kill", "-9", main_pid.as_str()]);
    let restart_props = wait_for_systemd_restart(
        "ontoforge-webhook-worker.service",
        &initial_props,
        Duration::from_secs(60),
    )
    .await;
    let restart_dump = restart_props
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(
        dir.join("systemctl-show-after-restart.txt"),
        format!("{restart_dump}\n"),
    )
    .expect("write post-restart systemctl show evidence");
    record_supervision(
        &dir,
        &serde_json::json!({
            "case": "sigkill_mid_dispatch",
            "supervisor": "systemd",
            "unit": "ontoforge-webhook-worker.service",
            "event_id": event_id,
            "old_pid": main_pid.parse::<u64>().expect("initial MainPID"),
            "observed_signal": 9,
            "initial_invocation_id": initial_props.get("InvocationID"),
            "restarted_invocation_id": restart_props.get("InvocationID"),
            "nrestarts": restart_props.get("NRestarts"),
            "new_pid": restart_props
                .get("MainPID")
                .and_then(|value| value.parse::<u64>().ok()),
        }),
    );

    // The restarted worker's post is the receiver's second request, so it is answered
    // without the barrier; the release still happens for completeness and ordering.
    std::fs::write(barrier.join("release"), b"").expect("release the receiver barrier");
    let records = wait_for_ledger(
        &ledger_path,
        Duration::from_secs(60),
        || None,
        |records| {
            ledger_records_for(records, &event_id)
                .iter()
                .any(|record| record.attempt_seq >= 2 && (200..300).contains(&record.status_code))
        },
    )
    .await;
    let delivered = ledger_records_for(&records, &event_id);
    assert!(
        delivered.len() >= 2,
        "the recovered event must be attempted more than once: {delivered:?}"
    );
    assert_event_evidence(&records, &event_id);
    assert_ledger_has_no_credential(&ledger_lines(&ledger_path), &url);

    let rows = wait_for_outbox(&pool, &schema_sys, &submitted, Duration::from_secs(30)).await;
    write_outbox_snapshot(&dir, "outbox-final.json", &rows);
    let row = rows
        .iter()
        .find(|row| row.event_id == event_id)
        .expect("the outbox row of the event");
    assert!(
        row.delivered_at.is_some(),
        "the recovered outbox row must be delivered: {row:?}"
    );

    let final_props = systemctl_show(
        "ontoforge-webhook-worker.service",
        &[
            "NRestarts",
            "InvocationID",
            "MainPID",
            "ActiveState",
            "SubState",
            "ExecMainStatus",
            "Result",
        ],
    );
    let final_dump = final_props
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(
        dir.join("systemctl-show-final.txt"),
        format!("{final_dump}\n"),
    )
    .expect("write final systemctl show evidence");
    let since = test_begin.format("%Y-%m-%d %H:%M:%S").to_string();
    std::fs::write(
        dir.join("journal.txt"),
        sudo_output(&[
            "journalctl",
            "-u",
            "ontoforge-webhook-worker.service",
            "-b",
            "--since",
            since.as_str(),
            "--no-pager",
        ]),
    )
    .expect("write journal evidence");
}
