---
phase: "08"
slug: "production-webhook-end-to-end-delivery"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: true
wave_0_complete: true
created: "2026-11-08"
---

# Phase 08 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
>
> Seeded from `08-RESEARCH.md` § Validation Architecture. Task IDs are the plan slots the planner is expected to fill; they are re-mapped by `/gsd-validate-phase` once PLAN files exist.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust 内置 `#[test]` / `#[tokio::test]`（`tokio` 1.53.3，`macros` + `rt-multi-thread` 由 `ontoforge` 的 dev-dependencies 提供） |
| **Config file** | 无独立配置；`#[ignore]` 约定沿用 `crates/ontoforge-action/tests/outbox_pg.rs` / `crates/ontoforge/tests/cli_trino_it.rs` 的写法 |
| **Quick run command** | `cargo fmt --all --check && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo build -p ontoforge --example webhook_receiver --locked` |
| **Full suite command** | `cargo test --workspace --locked --offline` |
| **CI 命中命令** | `cargo test -p ontoforge --test webhook_e2e_pg --locked -- --ignored --nocapture`（需 `ONTOFORGE_PG_URL`） |
| **Estimated runtime** | 本机 quick 约 60–120 s（编译级）；CI e2e 约 30–90 s（PG 容器启动 + 1 次退避等待） |

---

## Sampling Rate

- **After every task commit:** Run `{quick run command}` —— **本机（Windows）编译级验证，约 60–120 s**
- **After every plan wave:** Run `cargo test --workspace --locked --offline` + `cargo test -p ontoforge --test cli_webhook_worker --locked`（既有基线不得被破坏）
- **Before `/gsd-verify-work`:** Linux CI 的 `phase8-webhook-e2e` job 绿，且 ledger artifact 已上传并归档进 `08-evidence/`
- **Max feedback latency:** **CI job 级别（分钟级）**，不是 120 秒。例外来源：端到端裁决门是 `gh run list`（等 CI 跑完），且 08-01-01 要求先 RED 后 GREEN 两次 CI run。本机可跑的只有编译级验证 —— 端到端用例是 `#[ignore]`d 且需要 PostgreSQL，而 D-03 与项目硬约束（Windows 本机不依赖 Docker）排除了本地执行。

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 08-01-01 | 01 | 1 | REL-01, REL-02a | T-08-01 / — | 接收端只写 ledger、不做断言、不信任何入参；worker 走 PG store，target 只来自可信配置 | integration (cross-process) | `cargo test -p ontoforge --test webhook_e2e_pg --locked -- --ignored --nocapture`（只在 Linux CI 的 `phase8-webhook-e2e` job 内可跑） | ✅ `crates/ontoforge/examples/webhook_receiver.rs` + `crates/ontoforge/tests/webhook_e2e_pg.rs`（`#[ignore]`，`--ignored --list` 列出 1 条）+ `.github/workflows/ci.yml` 的 `phase8-webhook-e2e` job | ✅ green — CI run `42515808955` 的 `phase8-webhook-e2e` job 结论 success，测试行 `webhook_worker_delivers_to_an_independent_receiver_over_postgres ... ok`；RED 态由 run `42505885675` 承担（行为断言失败，非编译错误） |
| 08-01-02 | 01 | 1 | REL-02b, REL-02c | T-08-02 / — | 判定只从接收端侧原始记录与 PG `of_webhook_outbox` 终态两处独立取证 | integration | 同上（同一测试内断言唯一集合相等、无丢弃、退避间隔、PG 终态交叉取证） | ✅ 同上（断言关键字 `of_webhook_outbox` / `delivered_at` / `attempt_count` / `last_error` / `remote_addr` / `try_wait` 均在测试文件内） | ✅ green — CI run `42510584474` 与 `42515808955` 均 success；ledger 侧 3 个唯一 `event_id` == 提交集合、每条首 503 末 204、退避间隔 1.0366 / 1.0276 / 1.0205 s，PG 侧 `of_webhook_outbox` 恰好三行且 `delivered_at` 非空、`attempt_count = 1`、`last_error = 'http_status_503'` |
| 08-02-01 | 02 | 2 | REL-01 | T-08-03 / — | 归档不含 PG 凭据；索引逐成员 SHA256 可独立复算 | CI (artifact) + 本机脚本 | `bash ci/query-services.sh check-artifacts target/phase8-webhook` + 归档索引校验命令（见 plan 08-02-01 的 `<automated>` 集合） | ✅ `08-evidence/` 三件套（README / `ci-evidence-index.json` / `ci-evidence.tar.gz`）+ `ci/verify-evidence-archive.sh` | ✅ green — `bash ci/verify-evidence-archive.sh <tar.gz> <index.json>` → `OK: 6 members verified against …`（退出码 0）；`--selftest` → `SELFTEST OK: 4 tamper scenarios rejected`（退出码 0）；归档 6 个成员无绝对路径、无 `..`、grep 不到 PostgreSQL 连接串 |
| 08-02-02 | 02 | 2 | REL-01 | — | 运维文档指向 CI job 名、环境变量与 harness 位置 | static (source) | `grep -c 'phase8-webhook-e2e' crates/ontoforge/WEBHOOK-WORKER.md` | ✅ `crates/ontoforge/WEBHOOK-WORKER.md` 文末新增 `## 端到端联调（CI）` | ✅ green — `phase8-webhook-e2e` / `phase8-webhook-ledger` / `ONTOFORGE_WEBHOOK_RECEIVER_BIN` / `ONTOFORGE_WEBHOOK_EVIDENCE_DIR` 四个字面量均在文内；该文件 diff 为 25 行新增、零删除行（既有段落逐字未改） |
| 回归 | — | — | 既有 3 条 CLI worker 基线不被破坏 | — | N/A | integration | `cargo test -p ontoforge --test cli_webhook_worker --locked` | ✅ 已存在 | ✅ green — 本机实跑 `3 passed; 0 failed` |
| 回归 | — | — | 接收端自身编译且不破坏 clippy/fmt | — | N/A | static | `cargo fmt --all --check` + `cargo clippy --workspace --all-targets --locked -- -D warnings` + `cargo build -p ontoforge --example webhook_receiver --locked` | ✅ 既有 CI 步骤 | ✅ green — CI run `42515808955` 的 `test (ubuntu-latest)` 与 `test (windows-latest)` job 均 success（含 fmt / clippy 门）；本机 post-merge build gate 亦为 `BUILD_EXIT=0` |

> **Task ID 对齐说明（2026-11-08 修订）：** 本表已与 `08-01-PLAN.md` / `08-02-PLAN.md` 的实际 task 一一对应（4 个 task：`08-01-01` tracer、`08-01-02`、`08-02-01`、`08-02-02`）。先前版本按 REL-01/REL-02 的子语义拆成 7 行（`08-01-01`..`08-01-05`），与计划的 task 粒度不一致，会让 Nyquist 审计定位不到 `08-01-03/04/05` —— 已按 checker 的 `nyquist_compliance` warning 修正。

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] `crates/ontoforge/examples/webhook_receiver.rs` — 独立进程接收端（**只写 ledger，不做断言、不暴露 `/counts`、不以退出码表达成败**）
- [x] `crates/ontoforge/tests/webhook_e2e_pg.rs` — `#[ignore]` 端到端编排 + 全部断言
- [x] `crates/ontoforge/Cargo.toml` — `[dev-dependencies]` 增加 `axum.workspace = true`
- [x] `.github/workflows/ci.yml` — 新增 `phase8-webhook-e2e` job
- [x] `08-evidence/` 归档产物 — ledger + CI 日志 + index（沿用 v1.0 `07.1-evidence/` 的 README + index JSON + tar.gz 三件套）
- Framework install: 无（沿用 workspace 已有 tokio / sqlx / tempfile / ulid / clap / serde_json）

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Phase 8 的整体结论位 | REL-01 / REL-02 | 项目惯例：机器可判定结果只作证据，结论位固定 `VERDICT: PENDING`，执行者不代签 | 人工复核 ledger 与 CI 日志后签字，写入 `08-VERIFICATION.md` 的人工签字节 |
| 「接收端只计一次」的证据口径映射 | REL-02b | D-02 禁止接收端自证、禁止暴露 `/counts` 聚合端点，因此**去重判定由测试侧对 ledger 原始记录做集合运算完成**（唯一集合 == 提交集合），接收端自身的 per-event 计数没有独立断言面（该点已裁定不补单测） | 人工复核时须按此口径理解：机器证据表现为「测试侧去重后为 3」，**不是**「接收端自报计数为 3」。SUMMARY 的证据叙述里必须显式写明这一映射，避免被误读 |
| systemd 实际自动拉起 | REL-03 | **不属 Phase 8** —— 属 Phase 9 | 不适用 |

---

## 已定的 Claude's Discretion（research 的三个 open question 已裁决）

- **证据归档：纳入。** phase gate 要求 ledger artifact 上传并归档进 `08-evidence/`，沿用 v1.0 `07.1-evidence/` 的三件套结构；CI 侧上传失败必须让 job 红，不能被掩盖。
- **接收端 HTTP 实现：用 axum**（workspace 已 pin 0.8.9），作 **dev-dependency** —— 从依赖图层面保证进不了发行包，且不必手写 HTTP 解析。
- **接收端去重逻辑不单独写单元测试** —— e2e 测试直接覆盖（ledger 唯一集合 == 提交集合即为去重正确的充分证据）。

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency 符合本 phase 的口径：**CI job 级别（分钟级）**，本机只有编译级验证（例外来源见 § Sampling Rate）
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** validated — Nyquist 审计未发现 gap（REL-01 / REL-02 各有对应自动化验证，且已在 Linux CI 上跑绿）；未派发 `gsd-nyquist-auditor`，未新增测试文件。人工可判定项（REL-01 / REL-02 的结论位签字、REL-02b 的去重口径映射）仍按上表留在 Manual-Only。

## Validation Audit 2026-11-08

| Metric | Count |
|---|---|
| Gaps found | 0 |
| Resolved | 0 |
| Escalated | 0 |
