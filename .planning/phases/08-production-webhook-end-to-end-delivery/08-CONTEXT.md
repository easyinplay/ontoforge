# Phase 8: Production Webhook End-to-End Delivery - Context

**Gathered:** 2026-11-08
**Status:** Ready for planning

<domain>
## Phase Boundary

让 `ontoforge webhook-worker` 二进制在 Linux CI 上对一个**独立进程运行的真实 HTTP 接收端**完成端到端投递，并从**接收端侧**证明三条语义：503 重试、按 `event_id` 去重、至少一次送达（无事件被静默丢弃）。

本 phase 只交付**机器可判定**的部分（CI 中可复现的端到端证据）。systemd unit 与崩溃后自动拉起属 Phase 9（REL-03），其中 systemd 的实际拉起行为是人工签字项，结论位固定 `VERDICT: PENDING`。

不做：不新增产品能力、不做接收端产品化、不引入 TLS/自定义 CA 钩子、不改 `ontoforge-action` 的投递语义。

</domain>

<decisions>
## Implementation Decisions

### 接收端 harness 的形态与归属

- **D-01:** 接收端实现为 Rust 独立 binary，放在 `crates/ontoforge/examples/` 下（建议文件名 `webhook_receiver.rs`），用 `cargo build --example` 构建。**不进发行包、不新增 workspace member、不作为 `ontoforge` 的 subcommand 对外暴露。**
  - 理由：它是测试 harness 而非产品能力；Phase 12 是 ontoforge 的首次对外发布，不应在第一次发布就多出一个消费者能调用的面。与项目同构、无额外运行时依赖，且 examples 天然被 release 打包流程排除。
  - **Reversibility:** reversible — 挪到独立 crate 或改为 subcommand 都是加法，只动 `Cargo.toml` 与 CI 引用。

### 证据通道

- **D-02:** 接收端把**每一次收到的请求**追加写入 ledger 文件（JSONL，每行一条请求记录）；CI 从该文件断言。接收端**不自己断言、不以退出码表达成功失败、不暴露 `/counts` 聚合端点**。
  - 理由：证据必须是原始记录而非自报的聚合值，才能人工复核；与项目「机器可判定结果只作证据、结论位固定 `VERDICT: PENDING`、执行者不代签」的习惯一致。
  - **Reversibility:** reversible — ledger 格式与断言位置都只影响 CI job 内部。

### 生产保真度

- **D-03:** worker 的 store 用 **PostgreSQL**（`[store] kind = "postgres"`，`url_env = "ONTOFORGE_PG_URL"`）；接收端走 **明文 HTTP**（`http://127.0.0.1:<port>`）。**不覆盖 HTTPS。**
  - 理由：生产即 PG，PG 路径必须被真实二进制（而非仅库级测试）覆盖；deferred item 原文要求的「持久数据库」在 PG 下才完全成立。HTTPS 需要给 `reqwest` 加自定义受信 CA 的配置钩子，那是**新增能力**，超出本 phase 边界。
  - **Reversibility:** costly — 改回 SQLite 会让「持久数据库」这条 deferred item 只满足一半并需要重写断言基线；加 TLS 则要新增配置面。

### 验证规模与 CI 编排

- **D-04:** 提交**多条（≥3 条）不同事件**以验证「无事件被静默丢弃」；**只验证第 1 次重试**（退避约 1s）。
  - 理由：覆盖 ROADMAP Phase 8 的 success criteria 1 / 3 / 4（端到端送达、503 后转 2xx 成功、每条事件至少一次、无静默丢弃）。退避算法是 `1 << (attempts-1)` 秒、上限 300s，验证第 k 次重试需等 2^(k-1) 秒；1 次重试在 CI 时长预算内足够证明退避路径被走通。
  - **Reversibility:** reversible — 事件条数与重试次数是 CI job 内的参数。

- **D-05:** 在 `.github/workflows/ci.yml` **新增一个独立 job**（建议名 `phase8-webhook-e2e`），职责只有这一条端到端用例；**在 PR 与 main push 上都跑**（沿用现有 `test` job 的触发条件），不并入现有 job、不只跑 main。
  - 理由：与现有 job 解耦，不拖慢其他 job 的职责边界；PR 上跑才有回归保护价值——这条用例正是 v1.0 从未跑过的路径。
  - **Reversibility:** reversible — job 拆分与合并只动 `.github/workflows/ci.yml`。

### Carried-Forward Constraints（继承既有决策，不再重议）

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

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 需求与验收
- `.planning/REQUIREMENTS.md` — REL-01 / REL-02 的需求正文与 Out of Scope（Phase 8 只覆盖这两条）
- `.planning/ROADMAP.md` — Phase 8 的 Goal 与 5 条 Success Criteria（本 phase 的验收基准）
- `.planning/MILESTONES.md` — Known Gaps 第 1 项（本 phase 缺口的原文）
- `.planning/STATE.md` — Deferred Items 里 v1.1 的承接映射（`05 → Phase 08 / 09`）

### worker 与投递语义
- `crates/ontoforge/WEBHOOK-WORKER.md` — worker 的运维契约：`[webhook]` 五个配置键、至少一次语义、接收端负责去重、`target` 只从可信配置读取、生产必须用服务管理器重启
- `crates/ontoforge-action/src/outbox.rs` — 退避算法 `1 << (attempts-1).min(16)` 秒 / 上限 300s、`X-Ontoforge-Event-Id` 请求头、payload 含 `event_id`、claim → send → mark_delivered / record_retry 全链路
- `crates/ontoforge/src/webhook_worker.rs` — CLI 入口 `ontoforge webhook-worker --server-config <toml>`、轮询循环、Ctrl+C 退出、失败只 `eprintln` 后继续
- `crates/ontoforge-engine/src/config.rs` — `[webhook]` 与 `[store]` 的校验规则：`lease_duration_ms > request_timeout_ms`、`batch_limit ∈ 1..=100`、`target` 不得带凭据或 fragment

### 现有测试基线（Phase 8 的对照起点）
- `crates/ontoforge/tests/cli_webhook_worker.rs` — 现有基线：跑真实 CLI 二进制 + 真实 HTTP + 503 重试 + 重启恢复，但接收端是**进程内**的 `wiremock::MockServer`、store 是 SQLite、不做 `event_id` 去重
- `crates/ontoforge-action/tests/outbox_pg.rs` — PG 库级 outbox 测试与 `#[ignore = "needs ONTOFORGE_PG_URL or --features embedded-pg"]` 约定
- `.github/workflows/ci.yml` — CI job 结构、PG / Trino service container 模式、第三方 action 固定到 commit SHA、`cargo test --workspace --locked -- --test-threads=1`
- `ci/query-services.sh` — CI 起停 PG service container 与 artifact 检查的既有脚本

### 来源与前置决策
- `.planning/milestones/v1.0-phases/05-online-action/deferred-items.md` — 本 phase 的来源：deferred item 原文「以真实部署的服务管理器、持久数据库和实际下游接收端联调 `ontoforge webhook-worker`：确认自动重启、503 重试、下游以 `event_id` 去重和至少一次送达」
- `.planning/milestones/v1.0-phases/05-online-action/05-CONTEXT.md` — Phase 05 关于 webhook 投递语义与去重责任的既定决策
- `.planning/adr/0016-storage-system-data-and-typed-edit-store.md` — PG（生产）/ SQLite（dev、test）分工；webhook outbox 存 PG；至少一次投递与 `event_id` 幂等
- `.planning/adr/0026-edit-log-format.md` — `event_id` 是唯一 ULID；store 层不去重，幂等键是 `command_id`

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `WebhookWorker::poll_at` / `poll_at_with_after_send`（`crates/ontoforge-action/src/outbox.rs`）：库级轮询入口，已实现 claim → send → mark_delivered / record_retry 全链路。Phase 8 **不改它**，只从 CLI 侧驱动。
- `WebhookConfig::new`（同上）：target / lease / timeout / batch 的统一校验入口。CI 侧生成的配置必须满足 `lease_duration_ms > request_timeout_ms`、`batch_limit ∈ 1..=100`、`target` 不带凭据与 fragment。
- `EditStore::claim_due_outbox` / `mark_delivered` / `record_retry`（`crates/ontoforge-storage`）：PG 后端路径已由 `outbox_pg.rs` 覆盖。
- `crates/ontoforge/tests/cli_webhook_worker.rs` 的 `committed()` helper：提交 Action 产生 outbox 事件的完整路径。PG 变体需替换 store 后端与 scope/token 构造。
- `ci/query-services.sh postgres`：CI 起 PG service container 的既有脚本，Phase 8 的新 job 应复用而非另写。
- `wiremock` 已在 `crates/ontoforge/Cargo.toml` 的 dev-dependencies 中：现有测试的 mock 手段。Phase 8 的接收端**不是** wiremock。

### Established Patterns
- **退避算法**：`1 << (attempts-1).min(16)` 秒，单次上限 300s（`outbox.rs:157`）→ CI 内验证第 1 次重试约等 1s，第 2 次再等 2s。
- **事件标识**：同一事件同时出现在 `X-Ontoforge-Event-Id` 请求头与 JSON payload 的 `event_id`（ULID，唯一）；接收端任一即可去重。
- **去重责任分层**：store 层不去重（幂等由 Action 层 `command_id` 负责），webhook 侧去重在接收端，worker 只保证至少一次。
- **配置来源**：`[webhook]` 只从可信 `ontoforge-server.toml` 读取；Action 请求不能改 target。
- **证据与结论分离**：机器可判定结果只作证据，结论位固定 `VERDICT: PENDING`，执行者不代签（Phase 02 已建立该惯例）。
- **CI 约束**：第三方 action 固定到 commit SHA；当前 `cargo test --workspace --locked -- --test-threads=1`（Phase 10 会拆掉 `--test-threads=1`，本 phase 不依赖它也不受影响）。

### Integration Points
- 新增 `crates/ontoforge/examples/webhook_receiver.rs` → CI job 用 `cargo build --example` 构建，作为**独立进程**运行并监听端口。
- CI job → PG service container（复用 `ci/query-services.sh postgres`）→ 生成 `[store] kind = "postgres"` + `url_env = "ONTOFORGE_PG_URL"` 的 server config。
- CI job → `ontoforge webhook-worker --server-config <toml>` → 接收端 `http://127.0.0.1:<port>`。
- 接收端 → ledger 文件 → CI 读取文件并断言（`jq` 或脚本）。
- CI job → 证据落盘（可选归档进 `08-evidence/`）。

</code_context>

<specifics>
## Specific Ideas

- 接收端必须实现**按 `event_id` 去重的计数**，而不是数原始请求数 —— 这是 REL-02 的核心，也是现有 `wiremock` 基线（`server.received_requests()` 数原始请求）缺的那一环。
- 「独立进程」是硬要求：接收端**不得**与测试进程同进程运行。这正是与现有 wiremock 基线的关键差异，也是 success criterion 5 的落点。
- ledger 需保留**原始请求记录**以便人工复核，不能只留聚合计数。
- 至少一次送达的证据口径：接收端按 `event_id` 去重后的唯一集合与提交的事件集合**完全相等**（不多不少），且每个唯一 id 的原始请求数 ≥ 1。
- 现有 `cli_webhook_worker.rs` 的三条用例（端到端送达 / 503 后重试 / 重启恢复）是有效的库级回归护栏，**不应删除**；Phase 8 是在其之上叠加「独立进程接收端 + PG store + event_id 去重」的更强证据。

</specifics>

<deferred>
## Deferred Ideas

- **接收端的 HTTPS / TLS 支持**：`reqwest` 当前没有自定义受信 CA 的配置钩子，加上它属于新增产品能力，不在 Phase 8 边界内。若将来 CI 需要 https 接收端，需另立需求（涉及 `WebhookConfig` 与 `[webhook]` 配置面变更）。
- **接收端产品化**：把接收端做成 `ontoforge webhook-receiver` 子命令或独立 crate，供用户自测自己的 webhook 配置。Phase 12 首次发布刻意不包含它；若将来要做，需另立 phase 并补发行包与文档。

</deferred>

---

*Phase: 8-Production Webhook End-to-End Delivery*
*Context gathered: 2026-11-08*
