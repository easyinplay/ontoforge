---
phase: 08-production-webhook-end-to-end-delivery
verified: 2026-11-08T20:43:44Z
status: human_needed
score: 16/16 must-haves verified
covered_files:
  - .github/workflows/ci.yml
  - .planning/phases/08-production-webhook-end-to-end-delivery/08-01-PLAN.md
  - .planning/phases/08-production-webhook-end-to-end-delivery/08-01-SUMMARY.md
  - .planning/phases/08-production-webhook-end-to-end-delivery/08-02-PLAN.md
  - .planning/phases/08-production-webhook-end-to-end-delivery/08-02-SUMMARY.md
  - .planning/phases/08-production-webhook-end-to-end-delivery/08-evidence/README.md
  - .planning/phases/08-production-webhook-end-to-end-delivery/08-evidence/ci-evidence-index.json
  - .planning/phases/08-production-webhook-end-to-end-delivery/08-evidence/ci-evidence.tar.gz
  - Cargo.lock
  - ci/verify-evidence-archive.sh
  - crates/ontoforge-action/src/outbox.rs
  - crates/ontoforge-action/tests/outbox.rs
  - crates/ontoforge-storage/migrations/postgres/0004_action.sql
  - crates/ontoforge-storage/migrations/sqlite/0004_action.sql
  - crates/ontoforge-storage/src/commit.rs
  - crates/ontoforge/Cargo.toml
  - crates/ontoforge/WEBHOOK-WORKER.md
  - crates/ontoforge/examples/webhook_receiver.rs
  - crates/ontoforge/tests/webhook_e2e_pg.rs
covered_digest: "v4:sha256:bb93e20da0edc7ff710131f88cad03fb5b22108c8618296e8fa64927b16128ef"
behavior_unverified: 0
overrides_applied: 0
behavior_unverified_items: []
human_verification:
  - test: "对照 CI run 42515808955 的日志与 `phase8-webhook-ledger` artifact 的 `ledger.jsonl`，人工复核 6 条原始记录：3 条事件各 2 次尝试（首条 503、末条 204），相邻间隔 1.157s / 1.183s / 1.026s 落在 [0.9s, 30s]，唯一 event_id 集合与 PostgreSQL of_webhook_outbox 的 3 行完全相等；然后签署 Phase 8 的结论位"
    expected: "ledger 原始记录、去重集合、PostgreSQL 终态三者互相印证，结论位由 PENDING 改为人工签署（本项目约定：机器可判定结果只作证据，执行者与 verifier 均不代签）"
    why_human: "项目约定把 Phase 8 的结论位固定为 `VERDICT: PENDING`，由人工复核 ledger 与 CI 日志后签字"
---

# Phase 08: Production Webhook End-to-End Delivery 验证报告

**Phase Goal:** 运维者能在 Linux CI 上把 outbox 中的事件投递给真实 HTTP 接收端，并观察到重试、去重与至少一次送达语义
**Verified:** 2026-11-08T20:43:44Z
**Status:** human_needed
**Re-verification:** No — initial verification（目录内此前无 `*-VERIFICATION.md`）
**结论位:** `VERDICT: PENDING`（本报告不代签）

## Goal Achievement

### Observable Truths（16 条，来自 08-01 / 08-02 的 must_haves.truths）

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | 运维者能在 Linux CI 上用真实 `ontoforge webhook-worker` 二进制把 outbox 事件投递给独立进程监听的真实 HTTP 接收端，接收端侧留下每次请求的原始记录（REL-01） | ✓ VERIFIED | CI run 42515808955 / job `phase8-webhook-e2e`（job id 128838450761，结论 success，`gh run view` 独立确认）；`crates/ontoforge/tests/webhook_e2e_pg.rs:244` 以 `Command::new(env!("CARGO_BIN_EXE_ontoforge"))` + `webhook-worker` 起真实 CLI；归档 ledger 6 行原始记录 |
| 2 | 首个尝试 503 后 worker 按退避重试、转 2xx 后投递成功；相邻两次间隔落在 [0.9s, 30s]（REL-02a） | ✓ VERIFIED | 归档 `ledger.jsonl` 实测间隔 1.157s / 1.183s / 1.026s（seq1→4、2→5、3→6 的 `received_at` 之差）；`webhook_e2e_pg.rs` 断言该区间，CI job `test result: ok. 1 passed` |
| 3 | 接收端按 event_id 去重后的唯一集合与提交集合完全相等；每个 event_id 至少观察到一次 2xx（REL-02b/c） | ✓ VERIFIED | `webhook_e2e_pg.rs:437` 对 `BTreeSet` 做 `assert_eq!`（集合相等，不是包含）；归档 ledger 唯一 event_id = 3 = 提交 3；每条事件 attempt_seq 2 的记录为 204 |
| 4 | 判定只从接收端原始记录与 PostgreSQL `of_webhook_outbox` 终态两处独立取证；接收端不做断言、不暴露聚合端点、不以退出码表达成败（D-02） | ✓ VERIFIED | `crates/ontoforge/examples/webhook_receiver.rs` 全文无 `assert!` / `assert_eq!`，路由只有 `Router::new().route("/events", post(receive))`（无 `/counts`），无 `process::exit`；断言全部在 `webhook_e2e_pg.rs` |
| 5 | 接收端是独立子进程（pid 不同）、只绑 127.0.0.1 真实 TCP 端口、记录里的 `remote_addr` 指向回环地址（SC5） | ✓ VERIFIED | `webhook_receiver.rs:149` `TcpListener::bind(("127.0.0.1", args.port))`；`webhook_e2e_pg.rs:583` 断言子进程 pid ≠ 测试进程 pid、`:429` 断言 `remote_addr` 以 `127.0.0.1:` 开头；归档 ledger 每行 `remote_addr` = `127.0.0.1:56908` |
| 6 | `[webhook] target` 只来自受信的 `ontoforge-server.toml`，Action 请求无法修改；`[store] kind = "postgres"` + `url_env` 走 server 同一条 store 解析路径（D-03 / CF-02） | ✓ VERIFIED | `crates/ontoforge/src/webhook_worker.rs:13` `load_server(&args.server_config)` → `:38` `EditStore::open(server.settings)`（与 server 同一解析路径）；`crates/ontoforge-engine/src/config.rs:56-63` `WebhookWorkerConfig` 五键 + `deny_unknown_fields`；`outbox.rs:79` 请求头取自 `config.target` |
| 7 | ledger 与上传的 artifact 不含 PostgreSQL 密码或连接串 | ✓ VERIFIED | `tar -xzOf …tar.gz \| grep -cE 'postgres://[^[:space:]]*:[^[:space:]]*@'` = 0；CI job 的 `credential_scan`（`check-artifacts target/phase8-webhook`）先于上传执行，且上传 step 的 `if` 依赖 `steps.credential_scan.outcome == 'success'` |
| 8 | FA-01（backstop）：接收端对非 JSON 请求体不 panic，`payload_event_id` 记为 null 且请求仍入 ledger | ✓ VERIFIED | 源码路径无 panic 分支：`serde_json::from_slice::<Value>(&body).ok().and_then(...)` → `None`；`event_id` 回退取 header；`event_id` 为 `None` 时 `attempt_seq = 0` 且状态码走 204 分支；ledger 写入是无条件路径。执行者另在本机实际观测过一次非 JSON POST（`header_event_id`/`payload_event_id`/`event_id` 三者 null、进程不 panic、ledger 照常多一行）——该观测在执行者记录里，**未**固化为仓库内测试 |
| 9 | FA-02：计算退避从第 10 次已持久化失败起达到 300 秒上限；高次计数补充验证覆盖 attempts 1..20 | ✓ VERIFIED | `crates/ontoforge-action/tests/outbox.rs::retry_backoff_caps_at_five_minutes_without_leaking_response` 循环上限扩至 20（已固化进本分支）后实跑 `cargo test -p ontoforge-action --locked --test outbox -- --nocapture` 通过（11 passed; 0 failed; 0.97s），逐次断言 `next_attempt_at - poll_input ∈ [min(2^(k-1),300), +1s)`，k=10..20 全部观测到 300 秒上限。注：原描述「第 17 次起」有误，上限自第 10 次已记录失败起生效；`.min(16)` 仅防 i64 移位溢出，在 `.min(300)` 之后不可观测。既有 CI job（`ci.yml:73`）自 Phase 05 起已每次运行该用例至 attempt 10，本提交后覆盖至 attempt 20。证据边界：SQLite + 可控时钟 + 单 worker，公式路径与后端无关 |
| 10 | CI 的 `phase8-webhook-e2e` job 在 PR 与 main push 上都跑（workflow 级触发条件覆盖、无额外 `if:`），是本 phase 唯一的端到端裁决点（D-05） | ✓ VERIFIED | `.github/workflows/ci.yml:3-8` 触发条件 `push: branches: [main]` / `pull_request:` / `workflow_dispatch:`；`ci.yml:383` 的 job 块内无任何 `if:` |
| 11 | job 的退出码是唯一的失败信号；artifact 以 `if-no-files-found: warn` 上传，不掩盖真实失败原因 | ✓ VERIFIED | `ci.yml:427` `if-no-files-found: warn`（该 job 内；其余 4 处 `error` 属于别的 job）；上传 step 的 `if` 为 `${{ !cancelled() && steps.credential_scan.outcome == 'success' }}` |
| 12 | 上传前用 `bash ci/query-services.sh check-artifacts target/phase8-webhook` 扫描证据目录 | ✓ VERIFIED | `ci.yml:419`（`credential_scan` step）位于上传 step 之前 |
| 13 | 无论成败，PostgreSQL 容器与临时目录都在 `if: always()` 分支里被 `cleanup` 清理 | ✓ VERIFIED | `ci.yml` 最后一个 step：`if: ${{ always() && steps.checkout.outcome == 'success' }}` → `run: bash ci/query-services.sh cleanup` |
| 14 | CI 证据归档成 README / index JSON / tar.gz 三件套，逐成员 path+size+sha256，且一致性由 `ci/verify-evidence-archive.sh` 机器复算判定过 | ✓ VERIFIED | `bash ci/verify-evidence-archive.sh …` → `OK: 6 members verified against …`（exit 0）；`--selftest` → `SELFTEST OK: 4 tamper scenarios rejected`（exit 0）；`tar -tzf` 列出 6 个普通文件成员，无绝对路径、无 `..`；归档内容连接串匹配数 0 |
| 15 | 运维者能在 `crates/ontoforge/WEBHOOK-WORKER.md` 找到这条端到端联调怎么跑：job 名、环境变量、harness 位置与边界 | ✓ VERIFIED | `WEBHOOK-WORKER.md:24` 新增 `## 端到端联调（CI）`；`phase8-webhook-e2e` / `phase8-webhook-ledger` / `ONTOFORGE_WEBHOOK_RECEIVER_BIN` / `ONTOFORGE_WEBHOOK_EVIDENCE_DIR` 四个字面量齐备；既有 `X-Ontoforge-Event-Id`（:22）与 `至少一次`（:22/:47）措辞保留；`git diff $(git merge-base main HEAD) --shortstat` = 25 insertions、0 deletions（纯追加） |
| 16 | FA-03（backstop）：归档内容确实来自一次成功 run | ✓ VERIFIED | 索引 `source_boundary.runs["42515808955"].successful_job_ids = [128838450761]`，`gh run view 42515808955 --json jobs` 独立确认该 job 结论 success；归档 ledger 的 `received_at` 落在 `2026-11-08T19:36:57.741Z – 19:36:59.107Z`，正好在 run 42515808955 的 phase8 job 日志窗口内（该 run 日志：`19:36:53.9` 起跑、`19:36:59.237` `test result: ok. 1 passed`）；较早的多事件矩阵 run 42510584474 的 ledger 时间戳是 `18:34`，两者可区分。**该时间戳关联是本次验证新增的确定性证据**，超出 08-02 执行时只能靠人工对照的判定力 |

**Score:** 16/16 truths verified

### ROADMAP Success Criteria 对照（goal-backward 口径）

| # | ROADMAP SC | Status | Evidence |
|---|-----------|--------|----------|
| SC1 | worker 在 GitHub Actions Linux runner 上把 outbox 事件投递给独立进程监听的真实 HTTP 接收端，CI 留存接收端侧证据 | ✓ VERIFIED | run 42515808955 / job `phase8-webhook-e2e` success；artifact `phase8-webhook-ledger` 已上传并冻结进 `08-evidence/`（索引逐成员 sha256 可复算） |
| SC2 | 接收端前若干次 503 → worker 退避重试 → 转 2xx 后投递成功，CI 留存每次尝试次数与退避间隔 | ✓ VERIFIED（含字面口径偏差，见 W-A） | 归档 ledger 记录 `attempt_seq` 1→2 与 `received_at`，间隔 1.157s/1.183s/1.026s |
| SC3 | 同一 `event_id` 被投递多次时，接收端按 `event_id` 只计一次 | ✓ VERIFIED | 唯一 event_id 集合 = 3 = 提交集合（`assert_eq!` 集合相等）；每条事件 attempt_seq 1（503）与 2（204）各一行 |
| SC4 | N 条事件排空后每条至少被观察一次，无事件被静默丢弃 | ✓ VERIFIED | 3 条提交 → 3 个唯一 event_id，无多无少；PostgreSQL `of_webhook_outbox` 恰好 3 行且 `delivered_at` 非空 |
| SC5 | 接收端是独立进程监听真实端口，不用内存内 mock 替代 | ✓ VERIFIED | `Command::spawn` 独立子进程 + `TcpListener::bind(("127.0.0.1", 0))` + ledger `remote_addr` 全部 `127.0.0.1:56908` |

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/ontoforge/examples/webhook_receiver.rs` | 独立进程接收端 harness | ✓ VERIFIED | 172 行；绑 127.0.0.1、ready-file、逐条无缓冲写 JSONL、同 event_id 首试 503 其后 204；无断言、无聚合端点 |
| `crates/ontoforge/tests/webhook_e2e_pg.rs` | 被 ignore 的端到端编排与断言 | ✓ VERIFIED | 610 行；`#[ignore = "needs ONTOFORGE_PG_URL and ONTOFORGE_WEBHOOK_RECEIVER_BIN"]`；缺环境变量即 `expect` panic（不静默跳过）；本机 `--ignored --list` 列出 1 条用例 |
| `crates/ontoforge/Cargo.toml` | dev-dependencies 段的 `axum.workspace = true` | ✓ VERIFIED | `:45 axum.workspace = true`；`[dependencies]` 段无 axum |
| `Cargo.lock` | `[[package]] name = "ontoforge"` 依赖列表多一条 `"axum",` 且零删除行 | ✓ VERIFIED | `awk … \| grep -qx ' "axum",'` 命中；本分支 diff 无删除行 |
| `.github/workflows/ci.yml` | 独立 job `phase8-webhook-e2e` | ✓ VERIFIED | 见 SC1/SC5 与 Key Link 表 |
| `ci/verify-evidence-archive.sh` | 归档完整性校验器 + 四场景自检 | ✓ VERIFIED | 325 行；实跑 exit 0 / `OK: 6 members verified`；`--selftest` exit 0 / `SELFTEST OK: 4 tamper scenarios rejected` |
| `08-evidence/README.md` | 归档说明 | ✓ VERIFIED | 72 行；含来源 job 名与核对/恢复步骤 |
| `08-evidence/ci-evidence-index.json` | 逐成员 path/size/sha256 + 来源边界 | ✓ VERIFIED | `file_count = 6` = `files` 长度；`source_boundary.acceptance` 写明「不代替独立 phase 验收」 |
| `08-evidence/ci-evidence.tar.gz` | 冻结的 CI 证据 | ✓ VERIFIED | 6 个普通文件成员；连接串匹配 0 |
| `crates/ontoforge/WEBHOOK-WORKER.md` | 运维者入口 | ✓ VERIFIED | 新增 `## 端到端联调（CI）`，25 行纯新增 |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `crates/ontoforge-action/src/outbox.rs` | `crates/ontoforge/examples/webhook_receiver.rs` | `X-Ontoforge-Event-Id` 请求头 + payload `event_id` | ✓ WIRED | `outbox.rs:79` `.header("X-Ontoforge-Event-Id", &event.event_id)`；接收端读小写 `x-ontoforge-event-id`；归档 ledger 里 `header_event_id == payload_event_id == event_id` |
| `crates/ontoforge/tests/webhook_e2e_pg.rs` | `crates/ontoforge/examples/webhook_receiver.rs` | `ONTOFORGE_WEBHOOK_RECEIVER_BIN` | ✓ WIRED | `:543` 读取并 `expect`；CI step env 指向 `target/debug/examples/webhook_receiver` |
| `crates/ontoforge/tests/webhook_e2e_pg.rs` | `crates/ontoforge/src/webhook_worker.rs` | `CARGO_BIN_EXE_ontoforge webhook-worker --server-config` | ✓ WIRED | `:244-246`；`webhook_worker.rs:13` 读 `--server-config` |
| `crates/ontoforge/tests/webhook_e2e_pg.rs` | `of_webhook_outbox` 终态列 | `SELECT event_id, attempt_count, delivered_at, last_error FROM "{schema_sys}".of_webhook_outbox` | ✓ WIRED | `:331` 独立连接取第二份证据；断言 `delivered_at.is_some()`、`attempt_count == 1`、`last_error == "http_status_503"` |
| `.github/workflows/ci.yml` | `08-evidence/ci-evidence.tar.gz` | artifact `phase8-webhook-ledger` | ✓ WIRED | `ci.yml:425` artifact 名与索引 `source_boundary.runs` 的 run id 对齐 |
| `crates/ontoforge/WEBHOOK-WORKER.md` | `.github/workflows/ci.yml` | job 名逐字一致 | ✓ WIRED | 文档与 `ci.yml:383` 同为 `phase8-webhook-e2e` |
| `08-evidence/ci-evidence-index.json` | `08-evidence/ci-evidence.tar.gz` | 逐成员 sha256/size 复算 | ✓ WIRED | `ci/verify-evidence-archive.sh` 实跑 exit 0 |
| `ci/verify-evidence-archive.sh` | `08-evidence/ci-evidence-index.json` | 按 `files` 数组逐条复算 + `file_count` 对齐 | ✓ WIRED | 实跑 `OK: 6 members verified`；四篡改场景自检通过 |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `examples/webhook_receiver.rs` | `event_id` / `attempt_seq` / `status_code` | 真实 HTTP 请求（header + body） | ✓ | ✓ FLOWING — 6 条记录来自 worker 的真实投递，非静态返回 |
| `tests/webhook_e2e_pg.rs` | `submitted: Vec<String>` | `ActionService` 提交的 `receipt.event_id`（真实 outbox 行） | ✓ | ✓ FLOWING — 非 psql 直接插表 |
| `tests/webhook_e2e_pg.rs` | `rows: Vec<OutboxRow>` | 独立 `PgPoolOptions` 连接查 `of_webhook_outbox` | ✓ | ✓ FLOWING — 第二来源交叉取证 |
| `08-evidence/ci-evidence.tar.gz` | `ledger.jsonl` | CI run 42515808955 的 artifact | ✓ | ✓ FLOWING — 时间戳与该 run 的 job 日志窗口吻合 |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| 端到端用例在本机可编译且被 discover（本机无 Docker，只能编译级验证） | `cargo test -p ontoforge --test webhook_e2e_pg --locked -- --ignored --list` | `1 test, 0 benchmarks`（`webhook_worker_delivers_to_an_independent_receiver_over_postgres: test`） | ✓ PASS |
| 既有基线用例未被破坏 | `cargo test -p ontoforge --test cli_webhook_worker --locked` | `test result: ok. 3 passed; 0 failed` | ✓ PASS |
| 归档完整性逐成员可复算 | `bash ci/verify-evidence-archive.sh …/ci-evidence.tar.gz …/ci-evidence-index.json` | `OK: 6 members verified against …`（exit 0） | ✓ PASS |
| 校验器不是空壳 | `bash ci/verify-evidence-archive.sh --selftest` | `SELFTEST OK: 4 tamper scenarios rejected`（exit 0） | ✓ PASS |
| RED 态是行为断言失败（不是编译错误、不是零用例发现） | `gh run view 42505885675 --log --job 128808308765` | `webhook_e2e_pg: 1 ignored tests discovered` → `the receiver recorded no requests at all` → `test result: FAILED. 0 passed; 1 failed` | ✓ PASS |
| GREEN 态在 Linux CI 上通过 | `gh run view 42515808955 --log --job 128838450761` | `test webhook_worker_delivers_… ... ok` / `test result: ok. 1 passed; 0 failed` | ✓ PASS |
| 四个 phase8 run 的 job 结论 | `gh run view <id> --json jobs` | 42505885675 failure / 42508002310 success / 42510584474 success / 42515808955 success | ✓ PASS |

### Probe Execution

不适用（SKIPPED）：本 phase 没有声明 `scripts/*/tests/probe-*.sh` 式 probe，也不在 probe/migration 类的 phase 范围内；机器可判定证据由上表的 CI run 与归档校验承担。

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| REL-01 | 08-01, 08-02 | 运维者能在 Linux CI 上运行 `ontoforge webhook-worker`，把 outbox 事件投递到真实 HTTP 接收端（独立进程监听端口），CI 日志中留下接收端收到事件的证据 | ✓ SATISFIED | run 42515808955 job success；artifact 已上传；归档三件套可复算；`WEBHOOK-WORKER.md` 给出运维入口 |
| REL-02 | 08-01, 08-02 | 接收端返回 503 时按退避重试直至成功；同一 `event_id` 被接收端只计一次，整体至少一次送达 | ✓ SATISFIED（机器可判定部分） | 归档 ledger：每条事件 503→204、间隔 1.157/1.183/1.026s；唯一集合相等；PG 侧 3 行 `delivered_at` 非空 / `attempt_count=1` / `last_error='http_status_503'`。**注**：退避上限（≥17 次失败）不在本条覆盖内，属 FA-02 |

**ID 核对（PLAN frontmatter ↔ REQUIREMENTS.md）**

- 08-01 frontmatter `requirements: [REL-01, REL-02]`；08-02 frontmatter `requirements: [REL-01, REL-02]` → 两份计划声明的 ID 均已在上表逐条结清，无遗漏。
- REQUIREMENTS.md 中映射到 Phase 8 的 ID 仅 REL-01、REL-02（`:15`、`:16`，状态栏 `:58`、`:59` 为 Complete）；REL-03→Phase 9、REL-04→Phase 10、REL-05/06→Phase 11、REL-07→Phase 12，均非本 phase 声明范围，**无 ORPHANED requirement**。
- REL-01 / REL-02 在 REQUIREMENTS.md 的状态勾选为 `[x]`，与本报告结论一致（REL-02 的退避上限部分由 REL-03 之外的 D-04 约定排除在本 phase 之外）。

### Prohibitions

| Prohibition | Tier | Status | Evidence |
|-------------|------|--------|----------|
| 接收端不得是进程内 mock；事件不得由 psql 直接插入 outbox | test | ✓ HELD | 接收端经 `Command::spawn` 起独立进程；事件经 `ActionService` 提交产生真实 outbox 行 |
| 接收端不得自证成功（不断言、无聚合端点、不以退出码表达成败） | test | ✓ HELD | 接收端源码无 `assert!`、无 `/counts`、无 `process::exit`；`grep -c 'webhook_receiver' crates/ontoforge/src/cli.rs` = 0、`main.rs` = 0 |
| ledger 与 artifact 不得含凭据 | test | ✓ HELD | 归档连接串匹配 0；CI `check-artifacts` 先于上传 |
| 接收端只能绑 127.0.0.1 | test | ✓ HELD | `bind(("127.0.0.1", args.port))`；ledger `remote_addr` 全为 `127.0.0.1:56908` |
| 接收端不得被产品化 | judgment | ✓ HELD | 非 `[[bin]]`、非 workspace member、非子命令；`ci/release-package.ps1:63/66` 只打包 binary + manifest（+ checksum），example 不入发行包 |
| 不得把「收到请求」等同于「事件送达」 | test | ✓ HELD | 判定用「唯一 event_id 集合 == 提交集合」+ `delivered_at` 非空，非原始条数 |
| 不得为让用例变绿而改投递语义 | test | ✓ HELD | `git diff --name-only main...HEAD -- crates/ontoforge-action/src/outbox.rs crates/ontoforge-engine/src/config.rs crates/ontoforge-storage/` 输出为空（0 文件） |
| 不得在 worker / store 层新增 event_id 去重 | test | ✓ HELD | 上述三个路径本分支零改动；去重只在接收端计数 |
| 归档与 artifact 不得含凭据 | test | ✓ HELD | 同上 |
| 归档不得被表述为独立验收结论；REL-03 保持 PENDING | judgment | ✓ HELD | 索引 `source_boundary.acceptance` 与 README 边界声明；REQUIREMENTS.md 的 REL-03 仍为 `[ ]` / Pending |
| 文档里接收端只能被描述为 CI 内部 harness | judgment | ✓ HELD | `WEBHOOK-WORKER.md` 新节明确「不是 ontoforge 的子命令、不进发行包、不作为对外能力」 |

### Anti-Patterns Found

无。对本 phase 改动的 6 个实现文件（`webhook_receiver.rs`、`webhook_e2e_pg.rs`、`verify-evidence-archive.sh`、`WEBHOOK-WORKER.md`、`ci.yml`、`Cargo.toml`）扫描 `TBD|FIXME|XXX` / `TODO|HACK|PLACEHOLDER` / `not yet implemented|coming soon` / `return null|return {}|=> {}|todo!()` 均无命中。

### Warnings（非阻塞，不改变 status）

- **W-A（字面口径偏差）**：ROADMAP SC2 写的是「CI 日志留存每次尝试的次数与退避间隔」。实测：`webhook_e2e_pg.rs` 无任何 `println!`/`eprintln!`，接收端与 worker 的 stdout/stderr 日志均为 0 字节 —— 逐次尝试的次数与间隔是通过 **CI artifact（`phase8-webhook-ledger`）** 留存、再由 `08-evidence/` 冻结进仓库的，**不在日志正文里**。只翻 run 日志文本而不下载 artifact 的复核者看不到这些数字。证据链完整（artifact → 归档 → 逐成员 sha256 → 复算脚本），故按 VERIFIED 记录，但字面与载体存在偏差，登记为待人工确认。
- **W-B（08-REVIEW.md WR-01）**：`receiver.ready` 不清理，第二次本地重跑可能指向已死端口。**不影响** CI 证据（每次 CI job 都是新容器、新 ledger、新 ready 文件）。仅影响本地可重跑性。
- **W-C（WR-02）**：ledger 追加不截断，同一证据目录里第二次本地运行会混入上一轮 event_id。仅影响本地可重跑性。
- **W-D（WR-03）**：接收端在锁内分配 `seq`、在锁外写文件，并发下文件顺序可能与 `seq` 顺序解耦。当前负载下（6 条记录、顺序一致）不影响结论；属于接收端 harness 的精度问题。
- **W-E（WR-04）**：`assert_ne!(receiver.id(), std::process::id())` 恒真（`Command::spawn` 的子进程 PID 必不同于父进程），该断言本身不构成独立性证据；真正的独立性证据是 spawn 调用点 + ledger 的 `remote_addr`。本报告采信后者（`remote_addr` 全部 `127.0.0.1:56908`，且端口由 OS 分配）。
- **W-F（SUMMARY 叙述不精确）**：`08-02-SUMMARY.md:159` 写「归档成员的 ledger 是 08-01 那次多事件矩阵 run 的原始记录」。实际来源是 run **42515808955**（ledger 时间戳 `19:36`，而多事件矩阵 run 42510584474 是 `18:34`）。同一份 SUMMARY 的 D3 行与索引 `source_boundary.runs` 记录的 run id 是正确的，仅该行叙述有误。
- **W-G（WINDOWS.md 残留）**：第 20 条（FA-03）仍为 open，但本次验证已给出时间戳级别的确定性关联（见 truth 16），建议人工复核后关闭。第 19 条（FA-02）已于 2026-11-09 随运行时证据补齐判 fixed（原「应保持 open 直到人工执行」的口径已被可控时钟验证取代——见 truth 9）。

### Human Verification Required

#### 1. Phase 8 结论位的人工签署（本项目固定流程）

**Test:** 对照 CI run 42515808955 的日志与 `phase8-webhook-ledger` artifact 的 `ledger.jsonl` 人工复核：6 条原始记录、3 条事件各 2 次尝试（503→204）、间隔 1.157s / 1.183s / 1.026s、唯一 event_id 集合与 PostgreSQL `of_webhook_outbox` 的 3 行完全相等；据此签署结论位
**Expected:** ledger 原始记录、去重集合、PostgreSQL 终态三者互相印证，结论位由 `VERDICT: PENDING` 改为人工签署
**Why human:** 项目约定把结论位固定为 `VERDICT: PENDING`——机器可判定结果只作证据，执行者与 verifier 均不代签

### Gaps Summary

无 blocker gap。16/16 条 must-have 由仓库源码 + Linux CI run + 冻结归档 + 可控时钟运行时验证判定通过。FA-02 原判 PRESENT_BEHAVIOR_UNVERIFIED（本 phase 端到端用例按 D-04 只执行第 1 次重试，「presence != behavior」）；2026-11-09 两轮独立复核查明 300 秒封顶自 attempt 10 起生效且该用例自 Phase 05 起每次 CI 都在跑（`ci.yml:73`），随后把 `retry_backoff_caps_at_five_minutes_without_leaking_response` 循环上限扩至 20 并固化进本分支，本机实测 11 passed / 0 failed / 0.97s——改判 VERIFIED。证据边界：SQLite + 可控时钟 + 单 worker，非 PostgreSQL 生产端到端的高次重试；公式与 claim/retry 代码路径与后端无关。

REL-01 / REL-02 的机器可判定部分已达成：Linux CI 上的真实 `ontoforge webhook-worker` 二进制 → 真实 PostgreSQL outbox → 独立进程、真实回环 TCP 端口上的 HTTP 接收端，重试、去重与至少一次送达三件事都可从接收端原始 ledger 与 PostgreSQL 终态两处独立复算。

---

_Verified: 2026-11-08T20:43:44Z_
_Verifier: Claude (gsd-verifier)_
