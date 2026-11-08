# Phase 8: Production Webhook End-to-End Delivery - Research

**Researched:** 2026-11-08
**Domain:** CI end-to-end verification of `ontoforge webhook-worker` against an independent HTTP receiver, over PostgreSQL
**Confidence:** HIGH (all in-repo facts verified by reading the source this session; one external API confirmed via official docs)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### 接收端 harness 的形态与归属

- **D-01:** 接收端实现为 Rust 独立 binary，放在 `crates/ontoforge/examples/` 下（建议文件名 `webhook_receiver.rs`），用 `cargo build --example` 构建。**不进发行包、不新增 workspace member、不作为 `ontoforge` 的 subcommand 对外暴露。**
  - 理由：它是测试 harness 而非产品能力；Phase 12 是 ontoforge 的首次对外发布，不应在第一次发布就多出一个消费者能调用的面。与项目同构、无额外运行时依赖，且 examples 天然被 release 打包流程排除。
  - **Reversibility:** reversible — 挪到独立 crate 或改为 subcommand 都是加法，只动 `Cargo.toml` 与 CI 引用。

#### 证据通道

- **D-02:** 接收端把**每一次收到的请求**追加写入 ledger 文件（JSONL，每行一条请求记录）；CI 从该文件断言。接收端**不自己断言、不以退出码表达成功失败、不暴露 `/counts` 聚合端点**。
  - 理由：证据必须是原始记录而非自报的聚合值，才能人工复核；与项目「机器可判定结果只作证据、结论位固定 `VERDICT: PENDING`、执行者不代签」的习惯一致。
  - **Reversibility:** reversible — ledger 格式与断言位置都只影响 CI job 内部。

#### 生产保真度

- **D-03:** worker 的 store 用 **PostgreSQL**（`[store] kind = "postgres"`，`url_env = "ONTOFORGE_PG_URL"`）；接收端走 **明文 HTTP**（`http://127.0.0.1:<port>`）。**不覆盖 HTTPS。**
  - 理由：生产即 PG，PG 路径必须被真实二进制（而非仅库级测试）覆盖；deferred item 原文要求的「持久数据库」在 PG 下才完全成立。HTTPS 需要给 `reqwest` 加自定义受信 CA 的配置钩子，那是**新增能力**，超出本 phase 边界。
  - **Reversibility:** costly — 改回 SQLite 会让「持久数据库」这条 deferred item 只满足一半并需要重写断言基线；加 TLS 则要新增配置面。

#### 验证规模与 CI 编排

- **D-04:** 提交**多条（≥3 条）不同事件**以验证「无事件被静默丢弃」；**只验证第 1 次重试**（退避约 1s）。
  - 理由：覆盖 ROADMAP Phase 8 的 success criteria 1 / 3 / 4（端到端送达、503 后转 2xx 成功、每条事件至少一次、无静默丢弃）。退避算法是 `1 << (attempts-1)` 秒、上限 300s，验证第 k 次重试需等 2^(k-1) 秒；1 次重试在 CI 时长预算内足够证明退避路径被走通。
  - **Reversibility:** reversible — 事件条数与重试次数是 CI job 内的参数。

- **D-05:** 在 `.github/workflows/ci.yml` **新增一个独立 job**（建议名 `phase8-webhook-e2e`），职责只有这一条端到端用例；**在 PR 与 main push 上都跑**（沿用现有 `test` job 的触发条件），不并入现有 job、不只跑 main。
  - 理由：与现有 job 解耦，不拖慢其他 job 的职责边界；PR 上跑才有回归保护价值——这条用例正是 v1.0 从未跑过的路径。
  - **Reversibility:** reversible — job 拆分与合并只动 `.github/workflows/ci.yml`。

#### Carried-Forward Constraints（继承既有决策，不再重议）

- **CF-01:** 投递是**至少一次**语义；**去重是接收端的责任**，worker 自身不去重（Phase 05 决策）。接收端须按 `X-Ontoforge-Event-Id` 头或 JSON payload 的 `event_id` 去重，同一事件重试时该 ID 不变。
- **CF-02:** `target` 仅从可信的 `ontoforge-server.toml` 读取，Action 请求不能修改它；接收端无认证（`[webhook]` 配置没有认证字段）。
- **CF-03:** store 层不去重，幂等键是 Action 层的 `command_id`；`event_id` 是唯一 ULID。
- **CF-04:** 生产部署必须由服务管理器在进程退出后重启 worker —— 该责任落在 Phase 9，本 phase 不交付 systemd unit。

### Claude's Discretion

以下留给 planner / executor 自主决定，Claude 有自由度（用户已明示可自行决定）：

- ledger 的具体字段结构与落盘路径（建议 JSONL；字段建议 `event_id`、`attempt_seq`、`received_at`、`path`、`header_event_id`、`payload_event_id`、`status_code`）
- 接收端进程在 CI job 里的启停方式与清理保证（避免 job 挂起）
- 复用 `ci/query-services.sh postgres` 起 PG service container 的具体接法
- 事件条数的具体取值（≥3，取 3–5）与 `batch_limit` 的取值（上限 100，`batch_limit` 必须 ≥ 事件条数才能一轮排空）
- 是否为接收端的 `event_id` 去重逻辑补 `cargo test` 单元测试
- CI job 的超时预算与 job 名
- 证据文件是否按 v1.0 惯例（见 `.planning/milestones/v1.0-phases/07.1-source-cardinality-gap-closure/07.1-evidence/`）归档进 `.planning/phases/08-production-webhook-end-to-end-delivery/08-evidence/`

### Deferred Ideas (OUT OF SCOPE)

- **接收端的 HTTPS / TLS 支持**：`reqwest` 当前没有自定义受信 CA 的配置钩子，加上它属于新增产品能力，不在 Phase 8 边界内。若将来 CI 需要 https 接收端，需另立需求（涉及 `WebhookConfig` 与 `[webhook]` 配置面变更）。
- **接收端产品化**：把接收端做成 `ontoforge webhook-receiver` 子命令或独立 crate，供用户自测自己的 webhook 配置。Phase 12 首次发布刻意不包含它；若将来要做，需另立 phase 并补发行包与文档。
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| REL-01 | 运维者能在 Linux CI 上运行 `ontoforge webhook-worker`，把 outbox 中的事件投递到**真实 HTTP 接收端**（独立进程监听端口），CI 日志中留下接收端收到事件的证据 | 见「CI job 结构」「接收端设计」「事件提交路径」三节；`crates/ontoforge/src/webhook_worker.rs` 的 CLI 入口与 `crates/ontoforge-action/src/outbox.rs` 的投递链路已存在且可被独立进程驱动 |
| REL-02 | 接收端返回 503 时，webhook-worker 按退避策略重试直至成功或触达上限；同一 `event_id` 在重试与重复投递场景下被接收端只计一次，整体保证至少一次送达 | 见「退避与重试时间线」「去重口径」「outbox 终态断言」三节；退避公式与 503 错误分类已从源码逐字确认 |

**Out of Scope（本 phase 不覆盖）**：REL-03（Phase 9，systemd 人工签字）、REL-04（Phase 10）、REL-05/06（Phase 11）、REL-07（Phase 12）。
</phase_requirements>

## Summary

Phase 8 **不需要写任何生产 Rust 代码**。`ontoforge webhook-worker` 的 CLI 入口、outbox 的 claim → send → mark_delivered / record_retry 全链路、PG store 的建表与迁移**都已经存在且正确**（`crates/ontoforge/src/webhook_worker.rs`、`crates/ontoforge-action/src/outbox.rs`、`crates/ontoforge-storage/src/store.rs`）。缺的是**证据**：一条在 Linux CI 上、把真实二进制、真实 PG、真实独立 HTTP 接收端串起来的端到端链路，以及从接收端侧读出来的原始记录。

本 phase 的实际交付物是三样东西：

1. **一个接收端 harness**（`crates/ontoforge/examples/webhook_receiver.rs`，Rust example target，非 bin、非 subcommand）。它监听真实 TCP 端口，把**每一次**收到的请求追加成一行 JSONL 写进 ledger，并按「同一 `event_id` 的第 1 次返回 503、其后返回 204」的确定性策略制造重试场景。它**不自己断言、不以退出码表达成败、不暴露聚合端点**（D-02）。
2. **一个 `#[ignore]` 的端到端集成测试**（`crates/ontoforge/tests/webhook_e2e_pg.rs`）。它在 PG 里提交 ≥3 个 Action 产生 ≥3 个 outbox 事件，然后把接收端与 worker 作为**两个独立子进程**拉起，轮询 ledger 直到证据齐备，最后做机器可判定的断言。
3. **一个独立 CI job**（`phase8-webhook-e2e`），复用 `ci/query-services.sh postgres` 起 PG service container，跑上面那个被 `--ignored` 命中的测试，把 ledger 与进程日志作为 artifact 上传。

**关键设计判断：进程编排放在 Rust 测试里，不放在 CI shell 里。** 现有基线 `crates/ontoforge/tests/cli_webhook_worker.rs` 已经演示了「`Command::spawn` + `impl Drop { kill; wait }`」的模式，把接收端和 worker 的生命周期收进测试进程，CI shell 里就**没有任何后台进程**，从根上消掉「job 挂起」「清理不掉」「日志截断」这一整类失败模式。

**Primary recommendation:** 新建 `crates/ontoforge/examples/webhook_receiver.rs`（axum 实现，只写 ledger、不做断言）+ `crates/ontoforge/tests/webhook_e2e_pg.rs`（`#[ignore]`，Rust 内完成提交 / 起进程 / 断言）+ `.github/workflows/ci.yml` 的 `phase8-webhook-e2e` job（起 PG → 建 example → 跑被 ignored 的测试 → 上传 ledger → 清理容器）。

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| outbox 事件写入（Action 提交产生 event_id） | Database / Storage | API / Backend | `enqueue_action_event` 在 Action 事务内写 `of_webhook_outbox`（`crates/ontoforge-storage/src/commit.rs:1065-1092`）；只有 Action 路径会产生 outbox 行 |
| 事件认领与租约 | Database / Storage | — | `claim_due_outbox` 在一个短事务内 `SELECT ... FOR UPDATE` + `UPDATE lease_owner/lease_until`，提交后才做网络 I/O（`commit.rs:168-268`） |
| HTTP 投递与退避 | API / Backend | — | `WebhookTransport::send` 发 POST；退避在 `poll_at_with_after_send` 里算并交给 `record_retry`（`outbox.rs:74-173`） |
| 轮询循环与进程生命周期 | API / Backend | — | `crates/ontoforge/src/webhook_worker.rs` 是唯一的常驻进程入口；systemd 拉起属 Phase 9 |
| 接收端 HTTP 服务 | 独立进程（harness） | — | D-01/D-03：example target，监听 `127.0.0.1:<port>`，明文 HTTP |
| 证据落盘（ledger JSONL） | 独立进程（harness）→ 文件系统 | — | D-02：接收端只写原始记录，判定在测试/CI 侧 |
| 端到端判定（去重 / 至少一次 / 无丢弃 / 退避） | CI（Rust 测试） | — | 断言读 ledger + 读 PG `of_webhook_outbox` 终态，不依赖接收端自报 |
| PG service container 生命周期 | CI 基础设施 | — | 复用 `ci/query-services.sh postgres` / `cleanup` |

## Verified Facts（本 phase 依赖的逐字事实）

以下每条都来自本次会话用 Read 打开的源文件，值逐字引用。

### 1. 请求头与载荷字段

`crates/ontoforge-action/src/outbox.rs:74-90`：

```rust
    async fn send(&self, config: &WebhookConfig, event: &ClaimedOutboxEvent) -> Result<(), String> {
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
    }
```

- 请求头名逐字为 `X-Ontoforge-Event-Id`（`outbox.rs:79`）。**HTTP/1.1 头名大小写不敏感，hyper 在链路上发小写**，接收端必须大小写不敏感地取头。
- 非 2xx 的安全错误分类逐字为 `http_status_503`（`outbox.rs:88` 的 `format!("http_status_{}", status)`）。
- 传输层错误的分类逐字为 `transport_error`（`outbox.rs:84`）。

载荷里携带 event id 的字段名逐字为 `event_id`，位于 JSON 顶层。`crates/ontoforge-storage/src/commit.rs:1071-1079`：

```rust
    let payload = json!({
        "schema_version": 1,
        "event_id": receipt.event_id,
        "log_seq": receipt.log_seq,
        "committed_at": receipt.committed_at,
        "principal_id": principal_id,
        "action": action,
        "receipt": receipt,
    });
```

### 2. 退避公式与上限

`crates/ontoforge-action/src/outbox.rs:153-168`：

```rust
                Err(category) => {
                    let attempts = event.attempt_count.checked_add(1).ok_or_else(|| {
                        StoreError::Corrupt("outbox attempt count overflow".to_owned())
                    })?;
                    let backoff = 1_i64 << (attempts - 1).min(16);
                    let retry_at = completed_at + chrono::Duration::seconds(backoff.min(300));
                    self.store
                        .record_retry(
                            &event.event_id,
                            &event.lease_owner,
                            attempts,
                            retry_at,
                            &category,
                        )
                        .await?;
                }
```

- 退避逐字为 `1_i64 << (attempts - 1).min(16)` 秒，单次上限 `backoff.min(300)` 秒。
- 第 1 次失败（`attempts = 1`）→ 退避 `1 << 0` = **1 秒**；第 2 次失败 → `1 << 1` = 2 秒；第 k 次 → `2^(k-1)` 秒，第 17 次起被 `.min(16)` 夹住，再被 `.min(300)` 夹在 300 秒。
- D-04「只验证第 1 次重试」= 只等 1 秒退避，CI 预算内绰绰有余。

### 3. `[webhook]` 配置键（恰好 5 个）

`crates/ontoforge-engine/src/config.rs:54-63`：

```rust
/// The `[webhook]` section used by `ontoforge webhook-worker`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebhookWorkerConfig {
    pub target: String,
    pub poll_interval_ms: u64,
    pub request_timeout_ms: u64,
    pub lease_duration_ms: u64,
    pub batch_limit: u32,
}
```

校验规则逐字（`config.rs:611-651`）：

- `webhook.target` must be an http or https URL without credentials or fragment
- `webhook.poll_interval_ms` / `request_timeout_ms` / `lease_duration_ms` must be greater than zero
- `webhook.lease_duration_ms` must exceed request_timeout_ms
- `webhook.batch_limit` must be between 1 and 100

**没有认证字段**（CF-02）。`deny_unknown_fields` 意味着生成的配置里不能出现第 6 个键。

### 4. `[store]` 配置键与 env 间接层

`crates/ontoforge-engine/src/config.rs:223-255`：

```rust
pub struct StoreConfig {
    pub kind: StoreKind,
    /// SQLite database file, relative to the configuration file.
    #[serde(default)]
    pub path: Option<String>,
    /// Environment variable that holds the PostgreSQL connection string.
    #[serde(default)]
    pub url_env: Option<String>,
    /// File that holds the PostgreSQL connection string, relative to the configuration file.
    #[serde(default)]
    pub url_file: Option<String>,
    /// PostgreSQL schema of the typed edit tables.
    #[serde(default = "default_schema_edit")]
    pub schema_edit: String,
    /// PostgreSQL schema of system tables (edit log, definition snapshots).
    #[serde(default = "default_schema_sys")]
    pub schema_sys: String,
    ...
}
```

`StoreKind` 的取值逐字为 `Sqlite` / `Postgres`，serde 名逐字为 `"sqlite"` / `"postgres"`（`config.rs:216-221`，`#[serde(rename_all = "snake_case")]`）。

校验规则逐字（`config.rs:771-799`）：

- a postgres store needs exactly one of `url_env` and `url_file`
- a postgres store must not set `path`; it belongs to a sqlite store
- `url_env` must be an environment variable name ([A-Za-z_][A-Za-z0-9_]*)

默认 schema 名逐字为 `ontoforge_edit`（edit 表）与 `ontoforge_sys`（系统表：edit log / definition snapshots / **outbox**），见 `config.rs:265-271`。两者必须不同（`config.rs:812-816`）。

**关键结论（回答研究问题 3）**：CLI worker 路径与 server 走**完全相同**的 store 解析代码。`crates/ontoforge/src/webhook_worker.rs:38` 用 `EditStore::open(server.settings)`，`server.settings` 来自 `crates/ontoforge/src/reload.rs:206`：

```rust
    let settings = StoreSettings::from_config(store, config_dir, &|name| std::env::var(name).ok())
        .map_err(|e| vec![store_error_diagnostic(&e, file)])?;
```

`StoreSettings::from_config`（`crates/ontoforge-storage/src/store.rs:99-133`）对 Postgres 分支逐字：

```rust
            StoreKind::Postgres => BackendSettings::Postgres {
                url: resolve_secret(
                    cfg.url_env.as_deref(),
                    cfg.url_file.as_deref(),
                    config_dir,
                    env,
                )
                .map_err(|e| StoreError::Config(e.message))?,
                schema_edit: cfg.schema_edit.clone(),
                schema_sys: cfg.schema_sys.clone(),
            },
```

即 `[store] kind = "postgres"` + `url_env = "ONTOFORGE_PG_URL"` 与 server 走的机制**完全相同**，worker 无需任何额外配置面。

### 5. PG store 打开即建表建 schema（无需独立迁移步骤）

`crates/ontoforge-storage/src/store.rs:310-335`：

```rust
async fn open_postgres(
    url: &SecretString,
    schema_edit: &str,
    schema_sys: &str,
) -> Result<Db, StoreError> {
    let options = pg_connect_options(url, schema_sys)?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .map_err(|e| connect_error(&e))?;
    for schema in [schema_sys, schema_edit] {
        sql_exit::create_schema(&pool, schema).await?;
    }
    sqlx::migrate!("./migrations/postgres")
        .run(&pool)
        .await
        .map_err(|e| StoreError::Migrate(e.to_string()))?;
```

**回答研究问题 4 的后半段：PG 不需要任何独立的 schema / migration 步骤。** `EditStore::open` 自己 `CREATE SCHEMA` 并跑 `migrations/postgres`。outbox 表来自 `crates/ontoforge-storage/migrations/postgres/0004_action.sql:20-29`：

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

该迁移在 system schema 下运行（文件头注释：*Runs with search_path set to the system schema*），因此 `of_webhook_outbox` 落在 `schema_sys` 里。

### 6. PG 与 SQLite 在 worker 路径上的差异

`crates/ontoforge-storage/src/commit.rs:201-213`，claim 的 SELECT：

```rust
            if backend == crate::store::Backend::Postgres {
                select.lock(LockType::Update);
            }
```

即 **PG 用 `SELECT ... FOR UPDATE`（阻塞式行锁），SQLite 不用**（SQLite 走 `BEGIN IMMEDIATE` 写锁）。差异对本 phase 的影响：

- 事务在 claim 完成后**立即 commit**（`commit.rs:259-261`），网络 I/O 发生在事务之外 → 锁只持有毫秒级，不会因为接收端慢而卡住别的 worker。
- PG 的 `FOR UPDATE` 不 skip locked，两个 worker 并发认领同一行时第二个会**阻塞**而不是跳过。`crates/ontoforge-action/tests/outbox_pg.rs:126`（`postgres_outbox_parallel_workers_claim_one_generation`）已覆盖该语义。**本 phase 只有 1 个 worker，无并发争抢**，不引入额外风险。
- `mark_delivered` / `record_retry` 都以 `lease_owner` 令牌做 fence（`commit.rs:271-303` / `306-343`），PG 与 SQLite 行为一致。

### 7. PG 环境变量与测试库约定

`crates/ontoforge-storage/tests/support/mod.rs:271-276`：

```rust
async fn pg_url() -> (String, ServerGuard) {
    match std::env::var("ONTOFORGE_PG_URL") {
        Ok(url) if !url.trim().is_empty() => (url, None),
        _ => embedded_url().await,
    }
}
```

- 环境变量名逐字为 `ONTOFORGE_PG_URL`。
- `support::pg_store()` 为每个测试生成独立 schema 对，逐字为 `of_t_{unique}_edit` / `of_t_{unique}_sys`，其中 `unique` 是小写 ULID（`support/mod.rs:244-246`）。
- **重要陷阱**：该 helper 文件里有 `#[cfg(feature = "embedded-pg")]`。`ontoforge` 包**没有**这个 feature，把该文件 `#[path]` 引入 `ontoforge` 的测试会触发 `unexpected_cfgs` 警告，在 `cargo clippy --workspace --all-targets --locked -- -D warnings` 下**直接失败**。→ **不要**在 `crates/ontoforge/tests/` 里复用 `ontoforge-storage/tests/support/mod.rs`；改为在测试内直接构造 `BackendSettings::Postgres` / `StoreSettings::from_config`。（`crates/ontoforge-action/tests/outbox_pg.rs:18-19` 能安全复用是因为 `ontoforge-action` 声明了 `embedded-pg` feature。）

### 8. 现有基线测试（Phase 8 的对照起点）

`crates/ontoforge/tests/cli_webhook_worker.rs` 已有 3 条用例，**都不带 `#[ignore]`**，因此已在 `ubuntu-latest` 的 `test` job 里跑：

- `committed_action_is_delivered_by_running_cli_worker`
- `running_cli_worker_retries_after_http_failure`
- `restarted_cli_worker_recovers_failed_delivery`

它已具备：真实 CLI 二进制（`env!("CARGO_BIN_EXE_ontoforge")`）、真实 HTTP（`wiremock::MockServer`，虽然同进程但监听真实 TCP 端口）、503 重试、重启恢复、`X-Ontoforge-Event-Id` 与 payload `event_id` 断言。

**它的三处差距正是 Phase 8 要补的**：接收端是**进程内**的 wiremock、store 是 **SQLite**、**不做 `event_id` 去重**（它数的是 `server.received_requests()` 的原始请求数）。这三条用例**不应删除**（CONTEXT Specific Ideas 明确要求）。

### 9. CI 现状

`.github/workflows/ci.yml`：

- workflow 级 `env: CARGO_BUILD_JOBS: "2"`。
- 触发条件逐字为 `push: branches: [main]`、`pull_request:`、`workflow_dispatch:`。**D-05 的「PR 与 main push 上都跑」由 workflow 级触发条件自动满足**，新 job 不需要额外 `if:`。
- job 列表：`test`（matrix windows-latest + ubuntu-latest）、`integration`（PG + Trino）、`isolated-query-bench`、`phase7-real-services`。
- 第三方 action 全部固定到 commit SHA：`actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1`（v7.0.1）、`dtolnay/rust-toolchain@e2a55d2ffb04f378e9626c28d38d36d230d1e12f`（v1）、`Swatinem/rust-cache@6323deb102c322ba6fcbdcafc7e3dddab59af2b6`（v2.9.2）、`actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02`（v4）。
- `integration` job 的 PG 启动逐字为 `run: bash ci/query-services.sh postgres`，随后单独建库：`psql -v ON_ERROR_STOP=1 -d postgres -c 'CREATE DATABASE demo' -c 'CREATE DATABASE editstore'`，job 级 env 为 `PGHOST: localhost` / `PGUSER: postgres`。
- `ci/ontoforge-server.ci.toml` **只有 `[trino]` 与 `[profiles.dev]`，没有 `[store]`、没有 `[webhook]`** → 不能直接给 worker 用；且它是**签入仓库**的固定文件，不能内嵌 job 专属的 PG 端口/密码/端口号。**新 job 必须生成自己的 config**（生成到 `$RUNNER_TEMP`，不签入）。
- `cargo test --workspace --locked -- --test-threads=1`（`test` job）。

`ci/query-services.sh postgres` 分支（`ci/query-services.sh:15-48`）逐字：

- `printf 'PGPASSWORD=%s\n' "$password" >> "$GITHUB_ENV"`
- `printf 'ONTOFORGE_PG_URL=postgres://postgres:%s@127.0.0.1:5432/editstore\n' "$password" >> "$GITHUB_ENV"`
- 容器名 `postgres`，镜像 `postgres:17.11@sha256:2d2b8998d31037bf721cfdf764d76ba74171b4fab3431b7f72c27c56ddbdf9e3`，`-p 127.0.0.1:5432:5432`，health-cmd `pg_isready -U postgres`。
- 60 次 × 2s 的就绪等待（120s 上限），超时 `::error::PostgreSQL did not accept connections within 120 s`。
- **它不建 `editstore` 库**（只起容器）；`editstore` 库由 `integration` job 的 psql 语句或 `query-services.sh seed` 建立。
- `cleanup` 分支逐字为 `docker rm -f trino postgres` + 删 catalog 目录，容器不存在时 `|| true` 兜底 → 新 job 可安全在 `if: always()` 里调用。

### 10. Action 路径不需要 Trino

`crates/ontoforge-action/tests/outbox_pg.rs:42-54` 已在**只有 PG、没有 Trino** 的条件下完成 `reload` + Action 提交并产出 outbox 事件。`backing: editOnly` 的对象类型不触发源绑定探测；`reload` 不带 probe（`ReloadOptions::default()`）时不做任何 Trino 往返。

### 11. 发行包不会包含 example

`ci/release-package.ps1` 只 stage 三样东西：编译出的 `ontoforge` 二进制、`manifest.json`、`*.sha256`，并逐项断言归档里**只有 2 个成员**（binary + manifest）。它**不运行 `cargo package`**。→ D-01「不进发行包」由现有打包流程自动满足，**不需要改打包脚本**。

### 12. TLS 依赖守卫不拦 axum

`ci/check-tls-deps.sh` 跑 `cargo tree --workspace --locked --features ontoforge-storage/embedded-pg -e normal,build,dev`，只禁止 `openssl` 与 `native-tls` 两个包。`axum` 不引入 TLS 栈，且 `ontoforge` → `ontoforge-server` → axum 已在现有依赖图里。→ 把 axum 加成 `ontoforge` 的 dev-dependency **不改变**依赖图，守卫不会红。

## Standard Stack

本 phase **不引入任何新的外部 crate**。全部依赖都已在 workspace 中并已 pin 版本。

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| axum | `=0.8.9`（workspace pin） | 接收端 HTTP 服务（独立进程） | 项目自身的 HTTP 框架（`ontoforge-server` 在用）；避免手写 HTTP/1.1 解析 |
| tokio | `1.53.3`（workspace） | axum 的运行时；`rt-multi-thread` + `macros` 已由 `ontoforge` 的 dev-dep 提供 | axum 依赖 tokio，`net` feature 由 axum 的 `tokio` feature 带入 |
| serde_json | `1.0.154`（workspace） | ledger 每行 JSON、payload 里的 `event_id` 读取 | 全项目统一使用 |
| clap | `4.6.7`（workspace，`derive` + `env`） | 接收端的参数（`--port` / `--ledger` / `--fail-first-per-event`） | 与 `crates/ontoforge/src/cli.rs` 的 CLI 风格一致 |
| ontoforge-storage / ontoforge-action / ontoforge-auth / ontoforge-expr / ontoforge-query / ontoforge-schema / ontoforge-engine | workspace path crates | PG store、outbox worker、Principal/Scope、Pred、TokenCodec、ontology 加载、server config 解析 | 全部是 `ontoforge` 包已有的依赖，测试可直接用 |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| sqlx | `0.9.0`（workspace，`postgres` + `runtime-tokio`） | 测试侧直连 PG 读 `of_webhook_outbox` 终态 | 需要断言 `delivered_at` / `attempt_count` / `last_error` 时 |
| ulid | `3.0.0`（workspace） | 生成测试专属 schema 名（`of_t_<ulid>_edit` / `_sys`） | 避免与其它 job / 测试互相污染 |
| tempfile | `3.27.0`（workspace dev-dep） | 临时目录放 server config | 生成 job 内配置 |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| axum 接收端 | 纯 `std::net::TcpListener` 手写 HTTP/1.1 | 零 Cargo.toml 改动；但要自己处理 `\r\n\r\n` 分隔、Content-Length、keep-alive，约 40 行易错代码。**不推荐**（违反 Don't Hand-Roll） |
| Rust 测试内编排进程 | CI shell 里 `cmd & echo $! > pid` + `if: always()` 清理 | 贴近 `ci/server-services.sh` 的既有风格；但引入后台进程挂起、清理遗漏、日志被缓冲三大失败模式。**不推荐** |
| Rust 测试内编排进程 | GitHub Actions 的 `background: true` / `wait` / `cancel` 步骤关键字（2026-06-25 起可用） | 新特性，仓库其余 job 未使用；本方案本就不需要后台步骤。**不推荐** |
| 直接用 `psql` 往 `of_webhook_outbox` 插行造事件 | 走 Action 路径提交 | 直插会绕开 Action 原子提交与 edit log 外键，证据不真实（deferred item 要求「持久数据库 + 实际下游接收端」联调）。**不推荐** |
| 用 `ontoforge serve` + REST Action 端点 + dev-token 造事件 | 直接调 `ActionService` | serve 路径要额外准备 `[auth]`、`scope apply`、dev token、HTTP 客户端，失败面大 5 倍且与 webhook 无关。**不推荐** |
| 用 `ontoforge import` 造事件 | 走 Action 路径 | `import` **不写 outbox**（outbox 行只由 `enqueue_action_event` 写入，`commit.rs:1065`）。**不可行** |

### 版本与合法性核对

`gsd_run query package-legitimacy check --ecosystem crates axum tokio serde_json clap` → 4 个包全部 `OK`（axum 12.2M/周、tokio 23.1M/周、serde_json 32.4M/周、clap 21.9M/周；均有官方 repo；无 postinstall；未 deprecated）。

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| axum | crates | 2021-07-22 首发 | 12,204,705/周 | github.com/tokio-rs/axum | OK | Approved（新增为 `ontoforge` 的 **dev-dependency**，不进发行包） |
| tokio | crates | 2016-07-01 首发 | 23,142,567/周 | github.com/tokio-rs/tokio | OK | Approved（已在 workspace） |
| serde_json | crates | 2015-08-07 首发 | 32,416,901/周 | github.com/serde-rs/json | OK | Approved（已在 workspace） |
| clap | crates | 2015-03-01 首发 | 21,949,845/周 | github.com/clap-rs/clap | OK | Approved（已在 workspace） |

**Packages removed due to [SLOP] verdict:** 无（未引入新包）。
**Packages flagged as suspicious [SUS]:** 无。

**注意**：`axum` 若被加入 `crates/ontoforge/Cargo.toml`，必须加在 **`[dev-dependencies]`**（examples 可用 dev-dependencies），不能加进 `[dependencies]`——加进 `[dependencies]` 会让它进 `ontoforge` 的发布产物依赖，违反 D-01「不进发行包」。

## Architecture Patterns

### System Architecture Diagram

```
                    ┌──────────────────────── CI job: phase8-webhook-e2e (ubuntu-latest) ────────────────────────┐
                    │                                                                                          │
  docker run ──────┼─► PostgreSQL 17.11 container "postgres"  (127.0.0.1:5432)                                 │
  (ci/query-       │     └─ db "editstore"  ├─ schema of_t_<ulid>_sys  → of_webhook_outbox                    │
   services.sh)    │                        └─ schema of_t_<ulid>_edit → of_obj_gate                         │
                    │                                                                                          │
  cargo test ──────┼─► [TEST PROCESS] crates/ontoforge/tests/webhook_e2e_pg.rs  (cargo test --ignored)           │
                    │      │                                                                                   │
                    │      │ 1. EditStore::open(Postgres{schema_edit, schema_sys})  → 建 schema + 跑 migration   │
                    │      │ 2. reload(ontology)  → def_version                                            │
                    │      │ 3. ActionService::execute ×3  (3 个不同 command_id / 3 个不同 pk)                  │
                    │      │       └─► of_webhook_outbox 插入 3 行 (event_id = ULID, next_attempt_at = now)      │
                    │      │                                                                                   │
                    │      │ 4. spawn ──► [RECEIVER PROCESS] target/debug/examples/webhook_receiver               │
                    │      │       bind 127.0.0.1:0 → 实际端口写 ready-file                                   │
                    │      │       每次 POST /events → 追加一行 JSONL 到 ledger.jsonl                          │
                    │      │       同 event_id 第 1 次 → 503；其后 → 204                                      │
                    │      │                                                                                   │
                    │      │ 5. 写 server-config.toml: [store] postgres/url_env + [webhook] target=…/events      │
                    │      │                                                                                   │
                    │      │ 6. spawn ──► [WORKER PROCESS] target/debug/ontoforge webhook-worker               │
                    │      │       claim → POST → 503 → record_retry(next_attempt_at=+1s) → 下一轮 claim          │
                    │      │       → POST → 204 → mark_delivered                                            │
                    │      │                                                                                   │
                    │      │ 7. 轮询 ledger 直到 3 个 event_id 各有 ≥2 次尝试且至少 1 次 2xx（带 deadline）           │
                    │      │ 8. 断言: 唯一 event_id 集合 == 提交集合；首次 503 / 末次 2xx；                        │
                    │      │        退避间隔 ≥ 0.9s；PG of_webhook_outbox.delivered_at 非空、attempt_count=1、    │
                    │      │        last_error='http_status_503'                                              │
                    │      │ 9. kill + wait 两个子进程（Drop impl）                                            │
                    │                                                                                          │
  upload-artifact ──┼─► artifact "phase8-webhook-ledger": ledger.jsonl + receiver.log + worker.log              │
                    │                                                                                          │
  query-services.sh ┼─► cleanup（always()）: docker rm -f postgres                                          │
                    └──────────────────────────────────────────────────────────────────────────────────────────┘
```

数据流追踪：Action 提交 → `of_webhook_outbox` 行 → worker `claim_due_outbox` → `WebhookTransport::send`（HTTP POST，带 `X-Ontoforge-Event-Id`）→ 接收端写 ledger 并回 503/204 → worker `record_retry` / `mark_delivered` → PG 终态。断言从**接收端侧**（ledger）与**PG 终态侧**两处独立取证。

### Recommended Project Structure

```
crates/ontoforge/
├── examples/
│   └── webhook_receiver.rs        # 新增：独立进程接收端（axum），只写 ledger，不做断言
├── tests/
│   ├── cli_webhook_worker.rs      # 既有基线，不动
│   └── webhook_e2e_pg.rs          # 新增：#[ignore] 端到端编排 + 断言
└── Cargo.toml                     # 新增一行：[dev-dependencies] axum.workspace = true

.github/workflows/ci.yml           # 新增 job: phase8-webhook-e2e

.planning/phases/08-production-webhook-end-to-end-delivery/
├── 08-evidence/                   # 可选（Claude's discretion）：ci-evidence.tar.gz + ci-evidence-index.json + README.md
└── 08-RESEARCH.md
```

### Pattern 1: 独立进程 harness（example target）

**What:** 接收端作为 `crates/ontoforge/examples/` 下的 example target，用 `cargo build -p ontoforge --example webhook_receiver` 构建。它**不是** `[[bin]]`（不产生 `ontoforge` 的子命令），**不是** workspace member。

**When to use:** 任何需要「真实独立进程 + 真实 TCP 端口」但不想污染产品 API 面的测试 harness。

**Example:**

```toml
# crates/ontoforge/Cargo.toml
[dev-dependencies]
axum.workspace = true   # examples 可用 dev-dependencies；不进发布产物
```

```rust
// crates/ontoforge/examples/webhook_receiver.rs
use std::{collections::BTreeMap, net::SocketAddr, path::PathBuf, sync::Mutex};

use axum::{
    Router,
    body::Bytes,
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode},
    routing::post,
};

/// Everything the receiver records for one request.
#[derive(serde::Serialize)]
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

#[derive(Clone)]
struct Shared {
    ledger: PathBuf,
    state: Mutex<ReceiverState>,
}

#[derive(Default)]
struct ReceiverState {
    seq: u64,
    attempts: BTreeMap<String, u64>,
}

async fn receive(
    State(shared): State<Shared>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    path: axum::http::Uri,
    body: Bytes,
) -> StatusCode {
    // The header is case-insensitive on the wire; HeaderMap::get is too.
    let header_event_id = headers
        .get("x-ontoforge-event-id")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let payload_event_id = serde_json::from_slice::<serde_json::Value>(&body)
        .ok()
        .and_then(|value| value["event_id"].as_str().map(str::to_owned));
    let event_id = header_event_id.clone().or_else(|| payload_event_id.clone());

    let (attempt_seq, status_code) = {
        let mut state = shared.state.lock().expect("receiver state");
        let seq = state.seq;
        state.seq += 1;
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
        (attempt, status)
    };

    let record = LedgerRecord {
        seq: shared.state.lock().expect("receiver state").seq,
        received_at: chrono::Utc::now().to_rfc3339(),
        method: "POST".to_owned(),
        path: path.path().to_owned(),
        header_event_id,
        payload_event_id,
        event_id,
        attempt_seq,
        status_code,
        body_len: body.len(),
        remote_addr: Some(peer.to_string()),
    };
    // One unbuffered write per record so a polling reader sees it immediately.
    let mut line = serde_json::to_vec(&record).expect("ledger record");
    line.push(b'\n');
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&shared.ledger)
        .expect("ledger file");
    std::io::Write::write_all(&mut file, &line).expect("ledger write");
    std::io::Write::flush(&mut file).expect("ledger flush");

    StatusCode::from_u16(status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
}

#[tokio::main]
async fn main() {
    // --port 0 → let the OS choose; the actual address is written to --ready-file.
    // --ledger <path> → JSONL, one record per received request.
    // --fail-first-per-event → answer 503 to the first attempt of each event_id.
}
```

**Source:** axum 0.8 官方文档（`Bytes` extractor、`HeaderMap`、`StatusCode` 返回、`axum::serve(listener, app)`）— [CITED: https://github.com/tokio-rs/axum/blob/axum-v0.8.4/axum/src/docs/extract.md]、[CITED: https://github.com/tokio-rs/axum/blob/axum-v0.8.4/axum/src/docs/response.md]。

### Pattern 2: 真实 CLI 二进制 + 独立接收端进程 + PG store 的端到端测试

**What:** `crates/ontoforge/tests/webhook_e2e_pg.rs`，`#[ignore]`，在 Linux CI 用 `--ignored` 命中。它把「提交事件 / 起进程 / 断言」全放在 Rust 里。

**When to use:** 需要跨进程边界验证、且不想在 CI shell 里维护后台进程生命周期时。

**Example（骨架）：**

```rust
//! End-to-end webhook delivery over PostgreSQL against an independent receiver process.
//! The test is `#[ignore]`d; the Linux CI job `phase8-webhook-e2e` runs it with `--ignored`.

use std::process::{Child, Command, Stdio};
use std::time::Duration;

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

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs ONTOFORGE_PG_URL and ONTOFORGE_WEBHOOK_RECEIVER_BIN"]
async fn webhook_worker_delivers_to_an_independent_receiver_over_postgres() {
    // 1. The two environment inputs; fail loudly when they are missing.
    let url = std::env::var("ONTOFORGE_PG_URL").expect("ONTOFORGE_PG_URL must be set");
    let receiver_bin = std::env::var("ONTOFORGE_WEBHOOK_RECEIVER_BIN")
        .expect("ONTOFORGE_WEBHOOK_RECEIVER_BIN must point at the built example");

    // 2. A schema pair of its own, so the job never touches another test's tables.
    let unique = ulid::Ulid::generate().to_string().to_lowercase();
    let schema_edit = format!("of_t_{unique}_edit");
    let schema_sys = format!("of_t_{unique}_sys");

    // 3. The very same store resolution the CLI worker uses.
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

    // 4. Three distinct Action submissions -> three distinct outbox events.
    //    Distinct command_id AND distinct primary key, otherwise the second
    //    submission is idempotent (command_id) or conflicts on the pk.
    // 5. Spawn the receiver (independent process, real TCP port).
    // 6. Write the server config with [webhook] target = http://127.0.0.1:<port>/events.
    // 7. Spawn the worker: env!("CARGO_BIN_EXE_ontoforge") webhook-worker --server-config <toml>
    // 8. Poll the ledger until every event has a 503 then a 2xx.
    // 9. Assert the unique event_id set equals the submitted set, and read the
    //    terminal state of of_webhook_outbox straight from PostgreSQL.
}
```

**Source:** `crates/ontoforge/tests/cli_webhook_worker.rs`（`committed()` helper、`Worker` 结构与 `impl Drop { kill; wait }` 模式）、`crates/ontoforge-action/tests/outbox_pg.rs`（PG 版提交路径）。

### Anti-Patterns to Avoid

- **把接收端做成 `#[ignore]` 测试里的 in-process mock**：违反 D-01 与 SC5（接收端必须是独立进程监听真实端口）。`wiremock::MockServer` 虽然监听真实 TCP 端口，但与测试同进程，不满足「独立进程」。
- **在 CI shell 里用 `cmd &` 起接收端/worker**：后台进程会让 job 挂起或在取消时泄漏；GitHub 取消时对不处理信号的脚本清理代码可能根本不执行。
- **用 `psql` 直接 INSERT `of_webhook_outbox` 造事件**：绕过 Action 原子提交与 `of_edit_log` 外键，证据不真实。
- **让接收端自己统计并暴露 `/counts` 或用退出码表达成功**：违反 D-02（证据必须是原始记录，判定在 CI 侧）。
- **在 `crates/ontoforge/tests/` 里 `#[path]` 引入 `ontoforge-storage/tests/support/mod.rs`**：其中的 `#[cfg(feature = "embedded-pg")]` 会触发 `unexpected_cfgs`，在 `cargo clippy --workspace --all-targets --locked -- -D warnings` 下失败。
- **在 `crates/ontoforge/Cargo.toml` 的 `[dependencies]`（而非 `[dev-dependencies]`）加 axum**：会让它进发布产物依赖，违反 D-01。
- **给 `[webhook]` 加第 6 个键**：`WebhookWorkerConfig` 是 `deny_unknown_fields`，多一个键整个 config 解析失败。
- **复用 `ci/ontoforge-server.ci.toml` 给 worker 用**：该文件没有 `[store]` 也没有 `[webhook]`，且是签入仓库的静态文件，装不下 job 专属的 PG 端口 / 接收端端口。

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| HTTP/1.1 服务端（解析请求行、头、Content-Length、keep-alive） | 手写 socket 解析 | axum 0.8（workspace 已 pin） | 分块编码、连接复用、错误响应都要自己处理；axum 已在依赖图里，增量成本为零 |
| PG 连接池 / 事务 / 迁移 | 手写 PG 协议 | `ontoforge_storage::EditStore` | store 层已封装 schema 创建、migration、`FOR UPDATE` claim、lease fencing |
| outbox 投递与退避 | 自己写 worker | `ontoforge_action::outbox::WebhookWorker` | 退避、lease、fencing token、失败分类都已实现且有测试 |
| ULID / 事件 id 生成 | 自己生成 id | `ulid` crate（已在依赖里）+ Action 层产出 | `event_id` 必须是唯一 ULID 且与 `of_edit_log` 外键一致 |
| 进程清理 | 自己写 signal 处理 | `Child::kill()` + `Child::wait()`（`impl Drop`） | 现有 `cli_webhook_worker.rs` 已用此模式；SIGKILL 也不丢已落盘的 ledger 行 |
| CI 里起 PG 容器 | 自己写 docker run | `ci/query-services.sh postgres` | 已含密码生成、`::add-mask::`、就绪等待、`GITHUB_ENV` 导出 |
| Action 提交 / 幂等 / scope | 自己拼 SQL | `ActionService::execute` | 原子提交、edit log、outbox 三者同事务 |

**Key insight:** 本 phase 的全部难点不在「怎么投递」——投递逻辑早已存在且被 `cli_webhook_worker.rs` 覆盖。难点在**如何构造一条既真实又可判定的证据链**：真实二进制 + 真实 PG + 真实独立进程接收端 + 原始记录 + 从记录反推结论。任何一环退化成 mock 或自报聚合值，deferred item 要求的「真实部署联调」就只剩一半。

## Common Pitfalls

### Pitfall 1: `unexpected_cfgs` 让 clippy 失败
**What goes wrong:** 在 `crates/ontoforge/tests/` 里 `#[path = "../../ontoforge-storage/tests/support/mod.rs"] mod support;`，`cargo clippy --workspace --all-targets --locked -- -D warnings` 报 `unexpected cfg condition value: embedded-pg` 并因 `-D warnings` 失败。
**Why it happens:** `ontoforge` 包没有声明 `embedded-pg` feature，而 support 文件里有 `#[cfg(feature = "embedded-pg")]`。
**How to avoid:** 在 `crates/ontoforge/tests/` 里自建 PG store（用 `StoreSettings::from_config` + `StoreConfig`），不复用 storage 的 support 模块。
**Warning signs:** clippy 报 `unexpected cfg condition value`。

### Pitfall 2: `editstore` 库不存在
**What goes wrong:** `ci/query-services.sh postgres` 只起容器，**不建库**；`ONTOFORGE_PG_URL` 指向 `.../editstore`，worker 打开 store 时报连接失败。
**Why it happens:** 容器默认只建 `postgres` 库；`integration` job 是靠自己的 `psql ... -c 'CREATE DATABASE editstore'` 建的。
**How to avoid:** 新 job 在起 PG 之后加一步 `psql -v ON_ERROR_STOP=1 -d postgres -c 'CREATE DATABASE editstore'`（job 级 env `PGHOST=localhost` / `PGUSER=postgres`，`PGPASSWORD` 由 `query-services.sh` 写进 `GITHUB_ENV`）。
**Warning signs:** worker stderr 出现 `cannot open webhook worker edit store`。

### Pitfall 3: 请求头大小写
**What goes wrong:** 接收端用 `headers.get("X-Ontoforge-Event-Id")` 取不到值，去重退化成「按 payload 取」，或直接判失败。
**Why it happens:** HTTP/1.1 头名大小写不敏感，hyper 在链路上发小写；`HeaderMap::get` 传入非小写字符串时行为依赖具体实现，裸 `std` 实现一定会漏。
**How to avoid:** 用 axum 的 `HeaderMap`（`HeaderName` 解析大小写不敏感），并**同时**保留 payload `event_id` 作为回退（CF-01 允许二者任一）。
**Warning signs:** ledger 里 `header_event_id` 恒为 `null`。

### Pitfall 4: `batch_limit` 小于事件条数
**What goes wrong:** 3 个事件、`batch_limit = 2` 时，第一轮只处理 2 个，第三个要等下一轮；不影响正确性，但会让「一轮排空」的断言不成立。
**Why it happens:** `poll_at_with_after_send` 的循环条件是 `while count < self.config.batch_limit as usize`（`outbox.rs:126`）。
**How to avoid:** `batch_limit` 取 ≥ 事件条数（上限 100）。CONTEXT 已在 Claude's Discretion 里点出这一点。
**Warning signs:** ledger 的首个成功时间戳比预期晚一个 poll 周期。

### Pitfall 5: 退避被当成「立即重试」
**What goes wrong:** 断言只检查「收到过 503 且后来收到 2xx」，但 worker 其实是毫秒级忙重试，退避路径根本没被走通。
**Why it happens:** 只看状态码序列看不出时间间隔。
**How to avoid:** 从 ledger 的 `received_at` 计算同一 event_id 相邻两次的间隔，断言 **≥ 0.9s**（退避公式给出 1s，见 Verified Facts §2），上界给到 30s 抗抖动。同时断言 PG 里 `attempt_count = 1` 且 `last_error = 'http_status_503'`。
**Warning signs:** 间隔远小于 1s。

### Pitfall 6: 断言早于 ledger 落盘
**What goes wrong:** CI 读到空的或不完整的 ledger 就断言，job 假绿或假红。
**Why it happens:** 接收端若用 `BufWriter` 而不逐条 flush，读侧看不到最新行。
**How to avoid:** 每条记录用**无缓冲**写入（`File::write_all` + `flush`），并由测试**轮询** ledger 直到满足条件或超时，绝不「睡固定时长然后读一次」。
**Warning signs:** ledger 行数偶尔比请求数少。

### Pitfall 7: worker 静默早退
**What goes wrong:** worker 因为 config 错误、store 打不开等原因立刻退出，测试一直等不到投递，最后超时失败，但错误信息淹没在超时里。
**Why it happens:** `crates/ontoforge/src/webhook_worker.rs:13-29` 对配置错误只 `eprintln!` 后 `return 1`。
**How to avoid:** 测试在轮询循环里检查子进程是否还活着（`child.try_wait()`），一旦退出就把 worker 的 stderr 全文打进失败信息；worker 的 stdout/stderr 落盘到证据文件。
**Warning signs:** 轮询直到 deadline 才发现 worker 已死。

### Pitfall 8: 接收端没起来 / 端口冲突
**What goes wrong:** 目标端口已被占用，接收端 bind 失败退出，worker 拿到 `transport_error` 而不是 `http_status_503`。
**Why it happens:** 固定端口在 CI 上可能撞车。
**How to avoid:** 接收端 `--port 0` 让 OS 分配，把实际地址写进 `--ready-file`；测试**先等 ready-file 出现**再写 server config、再起 worker。
**Warning signs:** `last_error` 是 `transport_error` 而非 `http_status_503`。

### Pitfall 9: `lease_duration_ms` 必须大于 `request_timeout_ms`
**What goes wrong:** config 校验失败，worker 拒绝启动（`config.rs:639-644`）。
**Why it happens:** 生成的 config 随手写成了相等或更小。
**How to avoid:** 固定取值 `request_timeout_ms = 1000`、`lease_duration_ms = 5000`、`poll_interval_ms = 200`、`batch_limit = 10`。
**Warning signs:** worker stderr `invalid [webhook] settings`。

### Pitfall 10: `cargo test` 会编译 examples
**What goes wrong:** 现有 `test` job（windows-latest + ubuntu-latest）的 `cargo test --workspace --locked` 会**顺带编译 examples**，接收端因此进入每个 CI run 的编译路径，增加构建时间；且接收端必须在 **Windows 上也能编译**（项目硬约束）。
**Why it happens:** `cargo test` 默认构建 examples 以验证其可编译（[CITED: https://doc.rust-lang.org/cargo/commands/cargo-test.html]）。
**How to avoid:** 保持接收端极小（axum + serde_json + chrono）；不要引入需要平台特定链接的依赖；接收端只监听 `127.0.0.1`、只写普通文件，跨平台无差异。
**Warning signs:** `cargo test --workspace` 在 Windows 上报编译错误。

### Pitfall 11: PG `FOR UPDATE` 阻塞语义被误解
**What goes wrong:** 误以为 PG 的 claim 会 skip locked，于是在测试里并发拉起两个 worker 期待「各拿一个」。
**Why it happens:** PG 的 `SELECT ... FOR UPDATE` 是阻塞锁，不是 skip locked（`commit.rs:211-213` 只加 `LockType::Update`）。
**How to avoid:** 本 phase 只拉起 **1 个** worker；并发语义已由 `crates/ontoforge-action/tests/outbox_pg.rs:126` 覆盖，不要在本 phase 重复造。
**Warning signs:** 测试 hang 在 claim 上。

### Pitfall 12: 证据上传步骤掩盖真实失败
**What goes wrong:** `upload-artifact` 用 `if-no-files-found: error`，测试失败导致 ledger 没生成时，artifact 步骤再报一次错，真正的失败原因被淹没。
**Why it happens:** 现有 job 一律用 `if-no-files-found: error`（因为它们在成功路径上）。
**How to avoid:** 新 job 的上传步骤用 `if: ${{ !cancelled() }}` + `if-no-files-found: warn`，让测试步骤的退出码成为唯一的失败信号。
**Warning signs:** 日志里最后一条错误是 artifact 相关。

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust 1.98.0 toolchain | 全部构建 | ✓ | 1.98.0（CI 用 `dtolnay/rust-toolchain` pin） | — |
| Docker | PG service container | ✓（CI ubuntu-latest） | — | 无（本机 Windows 无 Docker 时无法本地跑本 case，属预期） |
| `openssl` | `ci/query-services.sh postgres` 生成密码 | ✓（ubuntu-latest 自带） | — | — |
| `psql` 客户端 | 建 `editstore` 库 | ✓（ubuntu-latest 自带；现有 `integration` job 已在用） | — | — |
| `jq` | 可选（用 Rust 断言则不需要） | ✓ | — | 用 Rust 断言即可不需要 jq |
| `curl` | 可选（就绪探测） | ✓ | — | 测试用 ready-file 探测，不需 curl |
| PostgreSQL 17.11 容器镜像 | PG store | ✓（`ci/query-services.sh` 已 pin digest `sha256:2d2b…d9e3`） | 17.11 | — |
| 本机 Windows 11 + Rust | 本地跑 `cargo build --example` / `cargo test` | ✓ | — | 本 phase 的端到端 case **只能在 Linux CI 跑**（需 PG + Docker），本机只保证编译通过 |

**Missing dependencies with no fallback:** 无（CI 侧全部具备）。
**Missing dependencies with fallback:**
- 本机（Windows）没有 Docker → 无法本地跑完整端到端 case。**这是设计使然**：D-03 要求生产保真度用 PG，Windows 本机按项目约束不依赖 Docker。本机验证口径 = `cargo build -p ontoforge --example webhook_receiver --locked` + `cargo test --workspace --locked --offline`（新测试被 `#[ignore]`，只编译不运行）。

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust 内置 `#[test]` / `#[tokio::test]`（`tokio` 1.53.3，`macros` + `rt-multi-thread` 由 `ontoforge` 的 dev-dependencies 提供） |
| Config file | 无独立配置；`#[ignore]` 约定沿用 `crates/ontoforge-action/tests/outbox_pg.rs` / `crates/ontoforge/tests/cli_trino_it.rs` 的写法 |
| Quick run command | `cargo test -p ontoforge --test webhook_e2e_pg --locked -- --ignored --nocapture`（需 `ONTOFORGE_PG_URL` + `ONTOFORGE_WEBHOOK_RECEIVER_BIN`） |
| Full suite command | `cargo test --workspace --locked --offline`（新测试被 ignore，只编译不执行） |
| CI 命中命令 | `cargo test -p ontoforge --test webhook_e2e_pg --locked -- --ignored --nocapture` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| REL-01 | worker 把 outbox 事件投递给**独立进程**的 HTTP 接收端，接收端留下原始记录 | integration (cross-process) | `cargo test -p ontoforge --test webhook_e2e_pg --locked -- --ignored --nocapture` | ❌ Wave 0 新建 `crates/ontoforge/tests/webhook_e2e_pg.rs` + `crates/ontoforge/examples/webhook_receiver.rs` |
| REL-02a | 接收端返回 503 后 worker 按退避重试 | integration | 同上（同一测试内断言） | ❌ 同上 |
| REL-02b | 同一 `event_id` 被接收端只计一次 | integration | 同上（ledger 唯一集合 == 提交集合） | ❌ 同上 |
| REL-02c | 至少一次送达、无事件静默丢弃 | integration | 同上（每个提交事件 ≥1 次成功；PG 侧 `delivered_at` 非空） | ❌ 同上 |
| SC5 | 接收端是独立进程监听真实端口 | integration | 同上（子进程 pid ≠ 测试 pid；ledger 记录 remote_addr 为 127.0.0.1:<port>） | ❌ 同上 |
| 回归 | 既有 3 条 CLI worker 基线用例不被破坏 | unit/integration | `cargo test -p ontoforge --test cli_webhook_worker --locked` | ✅ 已存在 |
| 回归 | 接收端自身编译且不破坏 clippy/fmt | static | `cargo fmt --all --check` + `cargo clippy --workspace --all-targets --locked -- -D warnings` | ✅ 既有 CI 步骤 |

### 机器可判定 vs 人工

**机器可判定（全部由 CI job 的退出码裁决）**

1. 接收端 ledger 里出现的 **唯一 `event_id` 集合 == 测试提交的 3 个 event_id 集合**（不多不少）→ SC3 / SC4。
2. 每个 event_id 在 ledger 里 **≥2 条记录**，第 1 条 `status_code == 503`、最后一条 `status_code ∈ 2xx` → SC2。
3. 同一 event_id 相邻两次的 `received_at` 间隔 **≥ 0.9s**（退避 1s 的下界）与 ≤ 30s（活性上界）→ 证明退避路径被走通，而非忙重试。
4. PG `of_webhook_outbox` 里 3 行全部 `delivered_at IS NOT NULL`、`attempt_count = 1`、`last_error = 'http_status_503'` → 证明重试后确实转成功并落终态。
5. ledger 里 `header_event_id` 与 `payload_event_id` 均等于该行的 `event_id`，且与 PG `of_webhook_outbox.event_id` 一致 → CF-01 / CF-03。
6. 接收端是 `Child` 子进程（pid ≠ 测试进程 pid），worker 通过 TCP 连到 `127.0.0.1:<port>`（ledger 的 `remote_addr`）→ SC5。
7. CI job 退出码为 0。

**人工（Phase 8 不代签）**

- 结论位固定 `VERDICT: PENDING`，等人工复核 ledger 与 CI 日志后签字；执行者不代签（沿用 Phase 02 建立的惯例）。
- REL-03（systemd 实际自动拉起）属 Phase 9，本 phase 不涉及。

### Sampling Rate

- **Per task commit:** `cargo fmt --all --check` + `cargo clippy --workspace --all-targets --locked -- -D warnings` + `cargo build -p ontoforge --example webhook_receiver --locked`（本机 Windows，编译级验证）。
- **Per wave merge:** `cargo test --workspace --locked --offline`（本机；新测试被 ignore）+ `cargo test -p ontoforge --test cli_webhook_worker --locked`（既有基线）。
- **Phase gate:** Linux CI 的 `phase8-webhook-e2e` job 绿 + ledger artifact 已上传并归档进 `08-evidence/`。

### Wave 0 Gaps

- [ ] `crates/ontoforge/examples/webhook_receiver.rs` — 独立进程接收端（只写 ledger，不做断言）
- [ ] `crates/ontoforge/tests/webhook_e2e_pg.rs` — `#[ignore]` 端到端编排 + 全部断言
- [ ] `crates/ontoforge/Cargo.toml` — `[dev-dependencies]` 增加 `axum.workspace = true`（若 planner 选 std-only 实现则不需要）
- [ ] `.github/workflows/ci.yml` — 新增 `phase8-webhook-e2e` job
- Framework install: 无（沿用 workspace 已有 tokio / sqlx / tempfile / ulid / clap / serde_json）

## Security Domain

`security_enforcement` 默认为启用（`.planning/config.json` 的 `workflow.security_enforcement` 为 `true`，`security_asvs_level: 1`）。本 phase 不新增对外攻击面，但引入了一个**监听真实端口的进程**，需要显式评估。

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | 接收端无认证（CF-02：`[webhook]` 没有认证字段，接收端必须接受匿名 POST）。这是**已知且被锁定的设计**，不是本 phase 引入的缺陷。接收端只绑 `127.0.0.1`，不对外暴露 |
| V3 Session Management | no | 无会话 |
| V4 Access Control | yes | 接收端只监听回环地址；ledger 路径由 CI 指定在 `target/` 下，不入 git；`ONTOFORGE_PG_URL` 里的密码由 `query-services.sh` 用 `::add-mask::` 掩码，且**不得**出现在 ledger / 日志 / artifact 里 |
| V5 Input Validation | yes | 接收端必须容忍任意 body（不假设 payload 形状），事件 id 优先取请求头、回退到 payload 的 `event_id`；不因 body 畸形而 panic（解析失败即 `None`） |
| V6 Cryptography | yes | **不引入**。D-03 明确不做 HTTPS；`reqwest` 不加自定义 CA 钩子。ledger 里不写任何凭据 |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| 接收端被当作无认证的公网端点 | Spoofing / Elevation | 只绑 `127.0.0.1`；不发布、不进发行包（D-01）；job 结束即杀进程 |
| PG 密码泄漏进 artifact / ledger | Information Disclosure | `query-services.sh` 已 `::add-mask::`；**新 job 必须像 `integration` job 那样在上传 artifact 前跑 `bash ci/query-services.sh check-artifacts <evidence-dir>`**（该子命令逐字扫描目录里是否出现 `$PGPASSWORD` 并在命中时报 `::error::Job credential found in artifact directory:`） |
| ledger 内容被当作可信输入 | Tampering | 判定只读 ledger 与 PG 两处独立来源并交叉比对；ledger 由接收端写但判定在测试侧，接收端不自证（D-02） |
| 事件 id 被伪造导致重复计数 | Spoofing | 断言 `header_event_id == payload_event_id == PG.of_webhook_outbox.event_id`；worker 的 target 只来自可信 config（CF-02） |
| 依赖引入 native-tls / OpenSSL | Tampering（供应链） | `ci/check-tls-deps.sh` 已在 `test` job 里跑；axum 不引入 TLS 栈，不改变依赖图 |
| 端口占用 / 端口扫描 | Denial of Service | `--port 0` 由 OS 分配；job 结束 `if: always()` 清理容器 |

**必须新增的一条 CI 步骤**（对齐 `integration` job 的既有做法）：上传 artifact 前先 `bash ci/query-services.sh check-artifacts <证据目录>`，命中密码即 `::error::` 并失败。

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `wiremock::MockServer` 作为接收端（进程内） | 独立 OS 进程 + 真实 TCP 端口 + 原始记录 ledger | 本 phase | 满足 SC5「不使用内存内 mock 替代」 |
| 只数原始请求数（`received_requests().len()`） | 按 `event_id` 去重后的唯一集合 | 本 phase | 满足 SC3 / REL-02「只计一次」 |
| SQLite store 的 CLI worker 验证 | PG store 的 CLI worker 验证 | 本 phase | 满足 D-03 生产保真度 |
| CI shell 后台进程（`cmd &` + pid 文件） | 进程生命周期收进 Rust 测试（`Child` + `Drop`） | 本 phase | 消除 job 挂起与清理遗漏 |

**Deprecated / 不可用**：
- `ontoforge webhook-receiver` 子命令形态：Phase 12 首次发布刻意不包含（D-01、Deferred Ideas）。
- 接收端 HTTPS / TLS：需要给 `reqwest` 加自定义受信 CA 的配置钩子，属新增能力，超出本 phase 边界（Deferred Ideas）。

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `cargo test` 默认编译 examples（因此接收端会进入 `test` job 的编译路径，且必须在 Windows 上可编译） | Pitfall 10 | 若实际不编译，则少一条 Windows 编译约束，但不影响任何断言 |
| A2 | `axum` 0.8.9 的默认 feature 集合包含 `tokio`（带入 `tokio/net`、`rt`、`macros`、`sync`、`time`） | Standard Stack / Pattern 1 | 若不含，`tokio::net::TcpListener` 不可用 → 需显式加 `tokio` 的 `net` feature 到 dev-dependencies（`tokio = { workspace = true, features = ["macros", "rt-multi-thread", "test-util", "net"] }`）。**planner 执行前建议先跑一次 `cargo build -p ontoforge --example webhook_receiver --locked` 验证** |
| A3 | `ONTOFORGE_WEBHOOK_RECEIVER_BIN` 由 CI job 通过 env 注入，测试读不到时**快速失败**（而非跳过） | Pattern 2 | 若改成「读不到就 skip」，CI 会假绿（正是 Pitfall 8 要防的） |
| A4 | GitHub Actions 的 `background: true` 步骤关键字已可用（2026-06-25 发布），但本方案不依赖它 | Alternatives Considered | 不影响结论 |
| A5 | `ubuntu-latest` 自带 `psql` 客户端与 `openssl` | Environment Availability | 现有 `integration` job 已在同一 runner 上用 `psql`，风险极低 |
| A6 | 单个 event_id 在「503 一次后 204」策略下恰好产生 2 次请求 | Pitfall 5 / Validation | 若 worker 在 204 与 `mark_delivered` 之间被杀会产生第 3 次请求；断言已按「≥2 次 + 末次 2xx」的宽容形式设计，不依赖恰好 2 次 |
| A7 | 断言「退避间隔 ≥ 0.9s」足以区分退避与忙重试 | Pitfall 5 | 若时钟分辨率或调度抖动导致观测值偏小，断言可能偶发失败；建议下界取 0.9s 并在失败信息里打印实测间隔 |
| A8 | 归档 CI 证据进 `08-evidence/` 沿用 v1.0 的 `ci-evidence.tar.gz` + `ci-evidence-index.json` + `README.md` 三件套格式 | 证据归档 | 该归档由执行者在 CI run 结束后用 `gh` CLI 事后抓取，不是 CI job 自己写进仓库；若本 phase 不做归档，需在 SUMMARY 里显式声明 |

## Open Questions

1. **证据归档是否纳入本 phase 的 Definition of Done？**
   - What we know: v1.0 的 07.2 phase 有 `evidence/ci-evidence.tar.gz` + `ci-evidence-index.json` + `README.md` 三件套，内容是 CI run JSON + 成功 job 日志 + 核验脚本，共 15 个文件，逐项记 SHA256；`gh` CLI 本机可用（2.103.3）。
   - What's unclear: CONTEXT 把「证据文件是否按 v1.0 惯例归档」列在 **Claude's Discretion**，不是硬要求；且归档必须等 CI run 真实跑完才有材料。
   - Recommendation: **纳入但降级为 best-effort**：CI job 必须上传 ledger + 进程日志为 artifact（这是硬要求）；`08-evidence/` 的 tar.gz 归档作为 phase 收尾时的可选动作，若 CI run 尚未产生则在 SUMMARY 里写明「归档待 CI run 完成后补」。

2. **接收端用 axum 还是纯 std？**
   - What we know: axum 已在 workspace（`=0.8.9`）且已在 `ontoforge` 的传递依赖图里，加进 `[dev-dependencies]` 不改变依赖图、不触发 TLS 守卫；std-only 方案零 Cargo.toml 改动但要手写 HTTP/1.1。
   - What's unclear: 纯偏好。
   - Recommendation: **axum**。理由写在 Don't Hand-Roll：HTTP 解析的手写缺陷会以「重试时机不对 / 503 没读到 / 连接被提前关闭」的形式出现，排查成本远高于一行 Cargo.toml。

3. **`batch_limit` 与事件条数的最终取值？**
   - What we know: 上限 100；必须 ≥ 事件条数才能一轮排空；CONTEXT 给的区间是 3–5 条事件。
   - What's unclear: 无实质分歧。
   - Recommendation: **事件 3 条、`batch_limit = 10`**。3 条已满足 D-04 的 ≥3；`batch_limit = 10` 留出余量且远小于 100。

4. **是否给接收端的去重逻辑补单元测试？**
   - What we know: CONTEXT 把这列为 discretion；接收端的去重状态机很小（`BTreeMap<String, u64>` 计数）。
   - What's unclear: 是否值得为 harness 单独开 `#[cfg(test)]` 模块。
   - Recommendation: **不补**。接收端的正确性由端到端测试直接检验（同一 event_id 计数正确、503→204 序列正确），再补一层单测是重复覆盖；且 example target 的 `#[cfg(test)]` 会被 `cargo test` 编译但不构成独立护栏。

5. **本地（Windows）要不要也跑一遍端到端 case？**
   - What we know: 项目硬约束是 Windows 11 上 `cargo build` / `cargo test` 通过且不依赖 Docker；D-03 要求 PG；本机无 Docker。
   - What's unclear: 是否值得为本地复现引入 `postgresql_embedded`。
   - Recommendation: **不引入**。`postgresql_embedded` 会拉 native-tls 除非关默认 feature（`ci/check-tls-deps.sh` 与 CI 注释已记录这个坑），且新增 feature 会扩大本 phase 的改动面。本 phase 的 case 就是 CI-only 的，Windows 侧只保证编译通过。

## Sources

### Primary (HIGH confidence) — 仓库内源文件，本次会话用 Read 逐字打开
- `crates/ontoforge-action/src/outbox.rs` — 退避公式与上限、`X-Ontoforge-Event-Id` 头、`http_status_503` 分类、payload 原样发送、`batch_limit` 循环边界
- `crates/ontoforge/src/webhook_worker.rs` — CLI 入口、轮询循环、Ctrl+C 退出 0、失败只 `eprintln!`
- `crates/ontoforge-engine/src/config.rs` — `[webhook]` 五个键与校验、`[store]` 键与校验、schema 默认名
- `crates/ontoforge-storage/src/commit.rs` — `claim_due_outbox`（PG `FOR UPDATE`）、`mark_delivered`、`record_retry`、`enqueue_action_event`（payload 形状）
- `crates/ontoforge-storage/src/store.rs` — `StoreSettings::from_config`、`open_postgres`（建 schema + 跑 migration）
- `crates/ontoforge-storage/migrations/postgres/0004_action.sql` — `of_webhook_outbox` 表定义
- `crates/ontoforge-storage/tests/support/mod.rs` — `ONTOFORGE_PG_URL` 约定、`pg_store()` 的 schema 命名、`embedded-pg` cfg 陷阱
- `crates/ontoforge-action/tests/outbox_pg.rs` — PG 版提交路径（无 Trino）、`#[ignore]` 文案约定
- `crates/ontoforge/tests/cli_webhook_worker.rs` — 既有基线三条用例、`committed()` helper、`Child` + `Drop` 清理模式
- `crates/ontoforge/Cargo.toml`、`Cargo.toml`（workspace） — 依赖与 pin、tokio features、axum pin
- `.github/workflows/ci.yml` — job 结构、触发条件、SHA pin、PG 起法
- `ci/query-services.sh` — `postgres` / `cleanup` / `check-artifacts` 子命令的逐字行为
- `ci/ontoforge-server.ci.toml` — 确认无 `[store]` / `[webhook]`
- `ci/release-package.ps1` — 发行包只含 binary + manifest + checksum（example 天然不入包）
- `ci/check-tls-deps.sh` — TLS 依赖守卫范围
- `.planning/phases/08-production-webhook-end-to-end-delivery/08-CONTEXT.md`、`.planning/REQUIREMENTS.md`、`.planning/ROADMAP.md`、`.planning/STATE.md`、`.planning/config.json`
- `.planning/milestones/v1.0-phases/05-online-action/deferred-items.md` — deferred item 原文
- `.planning/milestones/v1.0-phases/07.2-minio-contract-gap-closure/evidence/`（README.md + ci-evidence-index.json）— 证据归档格式

### Secondary (MEDIUM confidence) — 官方文档
- axum 官方文档（`/tokio-rs/axum`，v0.8.4 标签）：`Bytes` / `HeaderMap` / `StatusCode` 提取与返回、`axum::serve(TcpListener, Router)` — [CITED: https://github.com/tokio-rs/axum/blob/axum-v0.8.4/axum/src/docs/extract.md]、[CITED: https://github.com/tokio-rs/axum/blob/axum-v0.8.4/axum/src/docs/response.md]、[CITED: https://github.com/tokio-rs/axum/blob/axum-v0.8.4/README.md]
- cargo test 会构建 examples — [CITED: https://doc.rust-lang.org/cargo/commands/cargo-test.html]
- PostgreSQL `SELECT ... FOR UPDATE` 阻塞语义 — [CITED: https://www.postgresql.org/docs/current/sql-select.html]

### Tertiary (LOW confidence) — 仅作背景，未作为决策依据
- GitHub Actions 后台步骤关键字（`background: true` / `wait` / `cancel`）与取消时的信号序列 — [CITED: https://github.blog/changelog/2026-06-25-actions-steps-can-now-be-run-in-parallel]（本方案不依赖该特性）

### 工具核对记录
- `gsd_run query package-legitimacy check --ecosystem crates axum tokio serde_json clap` → 4 个包全部 `OK`
- `gsd_run query classify-confidence --provider context7` → MEDIUM；`--provider websearch` → LOW

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — 全部依赖已在 workspace 内并已 pin 版本，`package-legitimacy` 全 OK，无新外部包
- Architecture: HIGH — 每一环的源码（worker 入口 / outbox 语义 / store 打开 / PG 迁移 / CI 脚本接口）本次会话逐字读过
- Pitfalls: HIGH — 全部来自源码可推导的机制（`FOR UPDATE` 阻塞、`deny_unknown_fields`、`unexpected_cfgs`、`BatchLimit` 循环边界、`::add-mask::`），非泛化建议
- CI 编排细节: MEDIUM — `ci/query-services.sh` 与 `ci.yml` 是逐字读过的，但「runner 上 psql / openssl / docker 的具体版本」未逐一探测（沿用现有 job 的既有事实）

**Research date:** 2026-11-08
**Valid until:** 30 天（2026-12-08）—— 仓库内部结构变更（outbox 表形状、config 键、CI job 结构）会立即使本文失效；外部生态部分（axum / tokio）按小版本变更复核。
