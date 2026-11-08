# Phase 8: Production Webhook End-to-End Delivery - Pattern Map

**Mapped:** 2026-11-08
**Files analyzed:** 5 (3 new, 2 modified, 1 evidence dir)
**Analogs found:** 5 / 5

All analog paths below are verified git-tracked source (`git ls-files`).

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/ontoforge/examples/webhook_receiver.rs` | example / harness (HTTP server) | file-I/O (JSONL append) + event-driven (HTTP) | `crates/ontoforge/tests/cli_webhook_worker.rs` (worker spawn + config shape) + `crates/ontoforge-action/src/outbox.rs` (wire contract) | role-match (no existing example target in repo) |
| `crates/ontoforge/tests/webhook_e2e_pg.rs` | integration test | event-driven / cross-process + CRUD (PG) | `crates/ontoforge/tests/cli_webhook_worker.rs` (same crate, same CLI-binary spawn pattern) + `crates/ontoforge-action/tests/outbox_pg.rs` (`#[ignore]` + PG convention) | exact (composite of two analogs) |
| `crates/ontoforge/Cargo.toml` | config (manifest) | n/a | same file, `[dev-dependencies]` block (lines 41-48) | exact |
| `.github/workflows/ci.yml` | config (CI workflow) | n/a | same file, `integration` job (lines 84-287) + `phase7-real-services` job (lines 350-377) | exact |
| `.planning/phases/08-production-webhook-end-to-end-delivery/08-evidence/` | evidence archive | file-I/O | `.planning/milestones/v1.0-phases/07.2-minio-contract-gap-closure/evidence/` (README.md + `ci-evidence-index.json` + `ci-evidence.tar.gz`) | exact |

---

## Pattern Assignments

### `crates/ontoforge/examples/webhook_receiver.rs` (new — example target, file-I/O)

**Analog:** `crates/ontoforge/tests/cli_webhook_worker.rs` (lines 121-148: config writer + `Child`/`Drop` spawn contract), wire contract from `crates/ontoforge-action/src/outbox.rs:74-90`.

**Key constraint (D-01):** an example target, not `[[bin]]`, not a subcommand, not a workspace member. `cargo test --workspace` compiles examples, so it must build on Windows too (axum + serde_json + chrono only).

**Imports pattern** (project conventions: workspace-pinned crates, `camino::Utf8PathBuf` for paths):
```rust
use std::{collections::BTreeMap, net::SocketAddr, path::PathBuf, sync::Mutex};

use axum::{
    Router,
    body::Bytes,
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode},
    routing::post,
};
```
`axum` is already workspace-pinned at `=0.8.9` (workspace `Cargo.toml:50`); `serde_json` (`1.0.154`), `clap` (`4.6.7`, features `derive`,`env`), `chrono` (`0.4.45`), `tokio` (`1.53.3`) are all workspace deps.

**Wire contract the receiver must satisfy** (from `crates/ontoforge-action/src/outbox.rs:74-90`):
```rust
        let response = self
            .client
            .post(config.target.clone())
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header("X-Ontoforge-Event-Id", &event.event_id)
            .timeout(config.request_timeout)
            .body(event.payload.clone())
            .send()
            .await
            .map_err(|_| "transport_error".to_owned())?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(format!("http_status_{}", response.status().as_u16()))
        }
```
Consequences for the receiver:
- Header lookup must be **case-insensitive** (hyper sends lowercase on the wire) → use axum `HeaderMap` and `headers.get("x-ontoforge-event-id")`.
- Payload `event_id` is a top-level JSON field (`crates/ontoforge-storage/src/commit.rs:1071-1079`):
```rust
    let payload = json!({
        "schema_version": 1,
        "event_id": receipt.event_id,
        "log_seq": receipt.log_seq,
        ...
```
  Use header first, payload as fallback (CF-01 allows either).
- Non-2xx must be a retryable status; the worker's error category is `http_status_503` / `transport_error`.

**JSONL ledger write pattern** (append + explicit flush, no `BufWriter`):
```rust
    let mut line = serde_json::to_vec(&record).expect("ledger record");
    line.push(b'\n');
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&shared.ledger)
        .expect("ledger file");
    std::io::Write::write_all(&mut file, &line).expect("ledger write");
    std::io::Write::flush(&mut file).expect("ledger flush");
```
`std::fs::OpenOptions` usage precedent in this repo: `crates/ontoforge/src/dev.rs:3,203,315`, `crates/ontoforge-auth/src/authn/dev.rs:17,154`.

**Deterministic 503-then-204 policy** (per-event attempt counter, `BTreeMap<String, u64>`):
```rust
    let attempt = match &event_id {
        Some(id) => {
            let counter = state.attempts.entry(id.clone()).or_insert(0);
            *counter += 1;
            *counter
        }
        None => 0,
    };
    // Fail the first attempt of every event, then accept the retry.
    let status = if attempt == 1 { 503 } else { 204 };
```

**Bind pattern** (port 0 → OS assigns, actual address written to a `--ready-file` so the test never guesses a port):
```rust
#[tokio::main]
async fn main() {
    // --port 0 → let the OS choose; the actual address is written to --ready-file.
    // --ledger <path> → JSONL, one record per received request.
    // --fail-first-per-event → answer 503 to the first attempt of each event_id.
}
```
CLI style precedent: `crates/ontoforge/src/cli.rs` (clap derive, `crates/ontoforge/Cargo.toml:30` `clap.workspace = true`).

**D-02 guardrails (do NOT do):** no assertion logic, no `/counts` endpoint, no exit-code success signal, no credentials written into the ledger. The receiver records raw facts only; the test decides.

---

### `crates/ontoforge/tests/webhook_e2e_pg.rs` (new — integration test, cross-process + PG CRUD)

**Analogs:** `crates/ontoforge/tests/cli_webhook_worker.rs` (primary) and `crates/ontoforge-action/tests/outbox_pg.rs` (PG convention).

**Imports pattern** (copy from `cli_webhook_worker.rs:1-19`):
```rust
use std::collections::{BTreeMap, BTreeSet};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use camino::Utf8PathBuf;
use ontoforge_action::{ActionItem, ActionLimits, ActionOutcome, ActionRequest, ActionService};
use ontoforge_auth::{
    Principal, PrincipalId,
    scope::{PrincipalScope, TypeScope},
};
use ontoforge_expr::{Attrs, Pred};
use ontoforge_query::TokenCodec;
use ontoforge_schema::load_from_memory;
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::store::{EditStore, StoreSettings};
```
`sqlx.workspace = true` is already a dev-dependency of `ontoforge` (`crates/ontoforge/Cargo.toml:47`); `ulid` is a regular dependency (`crates/ontoforge/Cargo.toml:37`).

**`#[ignore]` convention** (from `crates/ontoforge-action/tests/outbox_pg.rs:124-126`):
```rust
#[tokio::test]
#[ignore = "needs ONTOFORGE_PG_URL or --features embedded-pg"]
async fn postgres_outbox_parallel_workers_claim_one_generation() {
```
Doc-header convention for a CI-only ignored test (`crates/ontoforge/tests/cli_trino_it.rs:1-6`):
```rust
//! Explicit online data checks end to end against the CI Trino (ENGINE-05, STORE-05).
//!
//! The test is `#[ignore]`d; the Linux CI job `integration` runs it with `--ignored` after
//! seeding the aviation example, using a private copy of `ci/ontoforge-server.ci.toml`
//! with the actual Trino URL. Without
//! `ONTOFORGE_TRINO_URL` it panics instead of passing silently (RESEARCH Pitfall 8).
```

**Action submission that produces an outbox event** (from `cli_webhook_worker.rs:42-119`, `committed()`), adapted to PG. Three submissions must use **distinct `command_id` AND distinct primary key** (`code`), otherwise the second is idempotent or conflicts on the pk:
```rust
    let outcome = service
        .execute(
            &principal,
            &scope,
            version,
            ActionRequest {
                action: "createGate".to_owned(),
                command_id: Some("worker-cli-command".to_owned()),
                items: vec![ActionItem {
                    params: serde_json::from_value(
                        serde_json::json!({"code":"G1", "name":"Private Name"}),
                    )
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
    receipt.event_id
```
Ontology `DEFS` const (`cli_webhook_worker.rs:21-40`) uses `backing: editOnly` → no Trino round trip required (`08-RESEARCH.md` §10).

**PG store construction — DO NOT reuse `ontoforge-storage/tests/support/mod.rs`.** That file has `#[cfg(feature = "embedded-pg")]` (`crates/ontoforge-storage/tests/support/mod.rs:278`) and `ontoforge` declares no such feature (`crates/ontoforge/Cargo.toml`), so `#[path]`-importing it triggers `unexpected_cfgs` and fails `cargo clippy --workspace --all-targets --locked -- -D warnings` (`08-RESEARCH.md` Pitfall 1). Build the settings locally with `StoreSettings::from_config` (`crates/ontoforge-storage/src/store.rs:99-133`):
```rust
    let unique = ulid::Ulid::generate().to_string().to_lowercase();
    let schema_edit = format!("of_t_{unique}_edit");
    let schema_sys = format!("of_t_{unique}_sys");
    let settings = StoreSettings::from_config(
        &StoreConfig {
            kind: StoreKind::Postgres,
            path: None,
            url_env: Some("ONTOFORGE_PG_URL".to_owned()),
            url_file: None,
            schema_edit: schema_edit.clone(),
            schema_sys: schema_sys.clone(),
            lock_timeout_ms: 5_000,
            probe_timeout_ms: 60_000,
            trino: None,
        },
        camino::Utf8Path::new("."),
        &|name| std::env::var(name).ok(),
    )
    .expect("store settings");
    let store = EditStore::open(settings).await.expect("open PostgreSQL store");
```
Schema naming convention copied from `crates/ontoforge-storage/tests/support/mod.rs:244-246` (`of_t_{ulid}_edit` / `of_t_{ulid}_sys`). `StoreConfig` field list: `crates/ontoforge-engine/src/config.rs:229-254`. `EditStore::open` creates the schemas and runs the PG migration itself (`crates/ontoforge-storage/src/store.rs:310-335`) → no separate migration step.

**Spawning the real CLI binary as a child process** (from `cli_webhook_worker.rs:128-148`):
```rust
struct Worker(Child);

impl Worker {
    fn start(config: &Utf8PathBuf) -> Self {
        Self(
            Command::new(env!("CARGO_BIN_EXE_ontoforge"))
                .args(["webhook-worker", "--server-config", config.as_str()])
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn()
                .expect("spawn worker"),
        )
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
```
For the receiver subprocess use the same `Child` + `Drop { kill; wait }` shape but spawn the built example binary (`ONTOFORGE_WEBHOOK_RECEIVER_BIN` env var, or a path resolved from `CARGO_BIN_EXE`/`target/debug/examples`). **Never** use `cmd &` in CI shell (background process hangs the job).

**Server config generation** (from `cli_webhook_worker.rs:121-126`, adapted to PG). The `[webhook]` section is `deny_unknown_fields` with exactly five keys (`crates/ontoforge-engine/src/config.rs:54-63`):
```rust
pub struct WebhookWorkerConfig {
    pub target: String,
    pub poll_interval_ms: u64,
    pub request_timeout_ms: u64,
    pub lease_duration_ms: u64,
    pub batch_limit: u32,
}
```
Validation rules (`config.rs:611-651`): `lease_duration_ms > request_timeout_ms`; `batch_limit ∈ 1..=100`; `target` is http/https without credentials or fragment. Safe values: `request_timeout_ms = 1000`, `lease_duration_ms = 5000`, `poll_interval_ms = 200`, `batch_limit = 10` (≥ event count so one poll drains all events).

Config file body to generate (NOT `ci/ontoforge-server.ci.toml`, which has only `[trino]` and `[profiles.dev]` — no `[store]`, no `[webhook]`, and is a checked-in static file):
```toml
[store]
kind = "postgres"
url_env = "ONTOFORGE_PG_URL"
schema_edit = "of_t_<ulid>_edit"
schema_sys = "of_t_<ulid>_sys"

[webhook]
target = "http://127.0.0.1:<port>/events"
poll_interval_ms = 200
request_timeout_ms = 1000
lease_duration_ms = 5000
batch_limit = 10
```
`[store]` key semantics (`crates/ontoforge-engine/src/config.rs:223-255`, validation `771-799`): a postgres store needs exactly one of `url_env`/`url_file`, must not set `path`, and `url_env` must be an env var name. The worker resolves the store through the same code path as the server (`crates/ontoforge/src/webhook_worker.rs:38` → `crates/ontoforge/src/reload.rs:206` → `StoreSettings::from_config`), so no extra config surface is needed.

**Polling the ledger instead of sleeping** (from `cli_webhook_worker.rs:150-162`, adapted to a file):
```rust
async fn wait_requests(server: &MockServer, count: usize) -> Vec<wiremock::Request> {
    tokio::time::timeout(Duration::from_secs(6), async {
        loop {
            let requests = server.received_requests().await.expect("received requests");
            if requests.len() >= count {
                break requests;
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    })
    .await
    .expect("worker should deliver the committed event")
}
```
The ledger reader must poll until the evidence is complete or the deadline hits, and must check the child's liveness inside the loop (`child.try_wait()`), dumping worker stderr on early exit (`crates/ontoforge/src/webhook_worker.rs:13-29` only `eprintln`s and returns 1).

**Reading `of_webhook_outbox` terminal state from PG** (sqlx pattern; closest existing precedent `crates/ontoforge-action/src/lib.rs:713-745`):
```rust
        let outbox_count: i64 = sqlx::query_scalar("SELECT count(*) FROM of_webhook_outbox")
            .fetch_one(&pool)
            .await
            .expect("outbox count");
        ...
        let outbox: (String, String) =
            sqlx::query_as("SELECT event_id, payload FROM of_webhook_outbox")
                .fetch_one(&pool)
                .await
                .expect("outbox event");
```
Schema-qualified variant (as in `crates/ontoforge-action/tests/idempotency_pg.rs:153-159`):
```rust
async fn count(f: &Fixture, table: &str) -> i64 {
    let sql = format!("SELECT count(*) FROM \"{}\".{table}", f.pg.schema_sys);
    sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
        .fetch_one(f.pg.side())
        .await
        .unwrap()
}
```
Table shape (`crates/ontoforge-storage/migrations/postgres/0004_action.sql:20-29`):
```sql
CREATE TABLE of_webhook_outbox (
    event_id text PRIMARY KEY REFERENCES of_edit_log (event_id),
    payload jsonb NOT NULL,
    attempt_count integer NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    next_attempt_at timestamptz NOT NULL,
    lease_owner text,
    lease_until timestamptz,
    delivered_at timestamptz,
    last_error text
);
```
Assertions to read: `delivered_at IS NOT NULL`, `attempt_count = 1`, `last_error = 'http_status_503'`. The table lives in the **system** schema (`schema_sys`).

**Backoff formula to assert against** (`crates/ontoforge-action/src/outbox.rs:153-168`):
```rust
                    let backoff = 1_i64 << (attempts - 1).min(16);
                    let retry_at = completed_at + chrono::Duration::seconds(backoff.min(300));
```
→ first retry is 1 s later; assert the gap between consecutive ledger rows of the same `event_id` is ≥ 0.9 s and ≤ 30 s.

**Deadlock/assertion caveats:** PG `claim_due_outbox` uses `SELECT ... FOR UPDATE` (blocking, not skip-locked) — `crates/ontoforge-storage/src/commit.rs:201-213`. Run exactly **one** worker in this test.

---

### `crates/ontoforge/Cargo.toml` (modified)

**Analog:** the same file, `[dev-dependencies]` block (lines 41-48):
```toml
[dev-dependencies]
ontoforge-query = { path = "../ontoforge-query" }
insta.workspace = true
tempfile.workspace = true
serde_json.workspace = true
tokio = { workspace = true, features = ["macros", "rt-multi-thread", "test-util"] }
sqlx.workspace = true
wiremock.workspace = true
```
Add exactly one line (examples may use dev-dependencies, and they do not enter the release artifact):
```toml
axum.workspace = true
```
**Must be `[dev-dependencies]`, never `[dependencies]`** — putting it in `[dependencies]` would pull axum into the published `ontoforge` crate's dependency set (violates D-01). `axum = "=0.8.9"` is already pinned in the workspace root `Cargo.toml:50`, so `workspace = true` resolves to the pin and the dependency graph does not change (`ci/check-tls-deps.sh` guard stays green: axum pulls no TLS stack).

---

### `.github/workflows/ci.yml` (modified — new job `phase8-webhook-e2e`)

**Analog:** the `integration` job (lines 84-287) for the PG service + ignored-test convention, and `phase7-real-services` (lines 350-377) for a standalone job with its own artifact + cleanup shape.

**Trigger condition:** D-05 ("PR and main push") is already satisfied by the workflow-level trigger (lines 3-13) — no extra `if:` needed:
```yaml
on:
  push:
    branches: [main]
  pull_request:
  workflow_dispatch:
```

**Job skeleton** (third-party actions pinned to full commit SHAs, copied from lines 91-96):
```yaml
  phase8-webhook-e2e:
    runs-on: ubuntu-latest
    timeout-minutes: 30
    env:
      PGHOST: localhost
      PGUSER: postgres
    steps:
      - id: checkout
        uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
      - uses: dtolnay/rust-toolchain@e2a55d2ffb04f378e9626c28d38d36d230d1e12f # v1
        with:
          toolchain: "1.98.0"
      - uses: Swatinem/rust-cache@6323deb102c322ba6fcbdcafc7e3dddab59af2b6 # v2.9.2
      - name: Start PostgreSQL 17.11 with job-scoped credentials
        run: bash ci/query-services.sh postgres
      - name: Create the editstore database
        run: psql -v ON_ERROR_STOP=1 -d postgres -c 'CREATE DATABASE editstore'
```

**Critical PG sequence:** `ci/query-services.sh postgres` (`ci/query-services.sh:15-48`) only **starts the container** and exports `PGPASSWORD` / `ONTOFORGE_PG_URL` (pointing at `.../editstore`) into `$GITHUB_ENV`; it does **not** create the `editstore` database. The `integration` job creates it separately (`.github/workflows/ci.yml:100-102`):
```yaml
      - name: Seed PostgreSQL from the aviation fixtures
        run: |
          psql -v ON_ERROR_STOP=1 -d postgres -c 'CREATE DATABASE demo' -c 'CREATE DATABASE editstore'
```
The new job needs only `CREATE DATABASE editstore` (no Trino, no fixtures — `backing: editOnly` actions need no source binding).

`ci/query-services.sh postgres` verbatim behaviour relied upon:
```bash
    printf 'PGPASSWORD=%s\n' "$password" >> "$GITHUB_ENV"
    printf 'ONTOFORGE_PG_URL=postgres://postgres:%s@127.0.0.1:5432/editstore\n' "$password" >> "$GITHUB_ENV"

    docker run -d --name postgres \
      -e POSTGRES_PASSWORD="$password" \
      -p 127.0.0.1:5432:5432 \
      --health-cmd 'pg_isready -U postgres' \
      ...
```

**Build the example and run the ignored test** (the `--ignored` + `--list` discovery guard is the `integration` job's convention, lines 127-144):
```yaml
      - name: Build the independent webhook receiver
        run: cargo build -p ontoforge --example webhook_receiver --locked
      - name: Webhook end-to-end delivery (PostgreSQL + independent receiver process)
        env:
          ONTOFORGE_WEBHOOK_RECEIVER_BIN: ${{ github.workspace }}/target/debug/examples/webhook_receiver
        shell: bash
        run: |
          set -euo pipefail
          : "${ONTOFORGE_PG_URL:?PostgreSQL service must set ONTOFORGE_PG_URL}"
          output=$(cargo test -p ontoforge --test webhook_e2e_pg --locked -- --ignored --list)
          count=$(printf '%s\n' "$output" | awk '/: test$/ { n++ } END { print n+0 }')
          echo "webhook_e2e_pg: $count ignored tests discovered"
          if [ "$count" -eq 0 ]; then echo "::error::webhook_e2e_pg discovered zero tests"; exit 1; fi
          cargo test -p ontoforge --test webhook_e2e_pg --locked -- --ignored --nocapture
```

**Credential scan + artifact upload + cleanup** (copied from the `integration` job, lines 258-287):
```yaml
      - id: credential_scan
        name: Check webhook evidence for job credentials
        if: ${{ !cancelled() }}
        run: bash ci/query-services.sh check-artifacts target/phase8-webhook
      - name: Upload webhook ledger and process logs
        if: ${{ !cancelled() && steps.credential_scan.outcome == 'success' }}
        uses: actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4
        with:
          name: phase8-webhook-ledger
          path: target/phase8-webhook/*
          if-no-files-found: warn
      - name: PostgreSQL logs
        if: failure()
        run: docker logs postgres 2>&1 || true
      - name: Clean up the PostgreSQL container
        if: ${{ always() && steps.checkout.outcome == 'success' }}
        run: bash ci/query-services.sh cleanup
```
`check-artifacts` (`ci/query-services.sh:145-161`) scans the directory for `$PGPASSWORD` and fails with `::error::Job credential found in artifact directory:` — the ledger must never contain the PG password.

**Artifact upload must use `if-no-files-found: warn`** (not `error`, used by the existing jobs) so a failed test does not produce a second, masking error.

---

### `.planning/phases/08-production-webhook-end-to-end-delivery/08-evidence/` (new, best-effort)

**Analog:** `.planning/milestones/v1.0-phases/07.2-minio-contract-gap-closure/evidence/` — three tracked files: `README.md`, `ci-evidence-index.json`, `ci-evidence.tar.gz`. This is a post-run archive performed by the executor with `gh` CLI after the CI job finishes; it is **not** written by the CI job itself.

---

## Shared Patterns

### Process lifecycle: `Child` + `impl Drop { kill; wait }`
**Source:** `crates/ontoforge/tests/cli_webhook_worker.rs:128-148`
**Apply to:** receiver subprocess and worker subprocess in `webhook_e2e_pg.rs`.
```rust
impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
```
No background processes in CI shell — all process lifetimes live inside the Rust test.

### Server configuration shape
**Source:** `crates/ontoforge/tests/cli_webhook_worker.rs:121-126` + `crates/ontoforge-engine/src/config.rs:54-63, 229-254`
**Apply to:** the generated `[store]`/`[webhook]` TOML in the test; the worker CLI reads it via `--server-config`.

### Config validation invariants
**Source:** `crates/ontoforge-engine/src/config.rs:611-651, 771-799`
**Apply to:** every generated config: `lease_duration_ms > request_timeout_ms`, `batch_limit ∈ 1..=100`, `target` without credentials/fragment, postgres store with exactly one of `url_env`/`url_file` and no `path`, `schema_edit != schema_sys`. `WebhookWorkerConfig` is `deny_unknown_fields` — a sixth `[webhook]` key makes the whole config fail to parse.

### Error handling in worker / test
**Source:** `crates/ontoforge/src/webhook_worker.rs:12-29` — the CLI only `eprintln!`s and returns a non-zero code; it never panics.
**Apply to:** the test must detect early worker exit (`child.try_wait()`) and surface the captured stderr in the failure message instead of timing out silently.

### Credential hygiene
**Source:** `ci/query-services.sh:16-31, 145-161`
**Apply to:** the CI job — password is `::add-mask::`ed and only reaches the job via `$GITHUB_ENV`; run `check-artifacts` on the evidence directory before uploading; never write the PG URL/password into the ledger or logs.

### Formatting / lint gates (must stay green)
**Source:** `.github/workflows/ci.yml:43, 52, 59`
**Apply to:** every new file.
```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked -- --test-threads=1
```
`cargo test --workspace` also builds examples → the receiver must compile on Windows.

### Evidence discipline
**Source:** `08-CONTEXT.md` D-02, `08-RESEARCH.md` "机器可判定 vs 人工"
**Apply to:** the test asserts; the receiver only records; the `VERDICT:` line stays `PENDING` — the executor does not sign off.

---

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `crates/ontoforge/examples/webhook_receiver.rs` (process-launch + ready-file handshake) | example / harness | event-driven | The repo has no `crates/*/examples/` Rust targets and no existing "ready-file" port-handshake pattern; the receiver's HTTP server, ledger writer and port-0 binding follow axum/std conventions, with the wire contract taken from `crates/ontoforge-action/src/outbox.rs` |

---

## Metadata

**Analog search scope:** `crates/ontoforge/tests/`, `crates/ontoforge-action/tests/`, `crates/ontoforge-storage/tests/`, `crates/ontoforge/src/`, `crates/ontoforge-engine/src/`, `crates/ontoforge-storage/src/`, `crates/ontoforge-storage/migrations/`, `ci/`, `.github/workflows/`
**Files scanned:** 12 source files read verbatim (analogs + wire-contract sources)
**Tracked-source check:** every analog path verified with `git ls-files` (all listed paths are tracked; no gitignored install/runtime mirrors used)
**Pattern extraction date:** 2026-11-08
