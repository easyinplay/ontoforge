---
phase: "08"
slug: "production-webhook-end-to-end-delivery"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
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

- **After every task commit:** Run `{quick run command}`
- **After every plan wave:** Run `cargo test --workspace --locked --offline` + `cargo test -p ontoforge --test cli_webhook_worker --locked`（既有基线不得被破坏）
- **Before `/gsd-verify-work`:** Linux CI 的 `phase8-webhook-e2e` job 绿，且 ledger artifact 已上传并归档进 `08-evidence/`
- **Max feedback latency:** 120 秒

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 08-01-01 | 01 | 1 | REL-01 | T-08-01 / — | 接收端只写 ledger、不做断言、不信任何入参 | integration (cross-process) | `cargo test -p ontoforge --test webhook_e2e_pg --locked -- --ignored --nocapture` | ❌ W0 新建 `crates/ontoforge/examples/webhook_receiver.rs` | ⬜ pending |
| 08-01-02 | 01 | 1 | REL-01 | T-08-02 / — | worker 走 PG store，target 只来自可信配置 | integration | 同上 | ❌ W0 新建 `crates/ontoforge/tests/webhook_e2e_pg.rs` | ⬜ pending |
| 08-01-03 | 01 | 1 | REL-02a | T-08-03 / — | 退避上界内重试，收到 2xx 后停止 | integration | 同上 | ❌ 同上 | ⬜ pending |
| 08-01-04 | 01 | 1 | REL-02b | T-08-04 / — | ledger 唯一 event_id 集合 == 提交集合 | integration | 同上 | ❌ 同上 | ⬜ pending |
| 08-01-05 | 01 | 1 | REL-02c | T-08-05 / — | PG `of_webhook_outbox` 终态 `delivered_at` 非空 | integration | 同上 | ❌ 同上 | ⬜ pending |
| 08-02-01 | 02 | 2 | REL-01 | — | CI job 拉起 PG + 接收端 + worker，退出码裁决 | CI (shell) | `cargo test -p ontoforge --test webhook_e2e_pg --locked -- --ignored --nocapture`（在 `.github/workflows/ci.yml` 的 `phase8-webhook-e2e` job 内） | ❌ W0 改 `.github/workflows/ci.yml` | ⬜ pending |
| 08-02-02 | 02 | 2 | REL-01 | — | ledger artifact 上传并归档 | CI (artifact) | artifact 上传步骤存在且 job 绿 | ❌ W0 归档脚本 | ⬜ pending |
| 回归 | — | — | 既有 3 条 CLI worker 基线不被破坏 | — | N/A | integration | `cargo test -p ontoforge --test cli_webhook_worker --locked` | ✅ 已存在 | ⬜ pending |
| 回归 | — | — | 接收端自身编译且不破坏 clippy/fmt | — | N/A | static | `cargo fmt --all --check` + `cargo clippy --workspace --all-targets --locked -- -D warnings` | ✅ 既有 CI 步骤 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/ontoforge/examples/webhook_receiver.rs` — 独立进程接收端（**只写 ledger，不做断言、不暴露 `/counts`、不以退出码表达成败**）
- [ ] `crates/ontoforge/tests/webhook_e2e_pg.rs` — `#[ignore]` 端到端编排 + 全部断言
- [ ] `crates/ontoforge/Cargo.toml` — `[dev-dependencies]` 增加 `axum.workspace = true`
- [ ] `.github/workflows/ci.yml` — 新增 `phase8-webhook-e2e` job
- [ ] `08-evidence/` 归档产物 — ledger + CI 日志 + index（沿用 v1.0 `07.1-evidence/` 的 README + index JSON + tar.gz 三件套）
- Framework install: 无（沿用 workspace 已有 tokio / sqlx / tempfile / ulid / clap / serde_json）

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Phase 8 的整体结论位 | REL-01 / REL-02 | 项目惯例：机器可判定结果只作证据，结论位固定 `VERDICT: PENDING`，执行者不代签 | 人工复核 ledger 与 CI 日志后签字，写入 `08-VERIFICATION.md` 的人工签字节 |
| systemd 实际自动拉起 | REL-03 | **不属 Phase 8** —— 属 Phase 9 | 不适用 |

---

## 已定的 Claude's Discretion（research 的三个 open question 已裁决）

- **证据归档：纳入。** phase gate 要求 ledger artifact 上传并归档进 `08-evidence/`，沿用 v1.0 `07.1-evidence/` 的三件套结构；CI 侧上传失败必须让 job 红，不能被掩盖。
- **接收端 HTTP 实现：用 axum**（workspace 已 pin 0.8.9），作 **dev-dependency** —— 从依赖图层面保证进不了发行包，且不必手写 HTTP 解析。
- **接收端去重逻辑不单独写单元测试** —— e2e 测试直接覆盖（ledger 唯一集合 == 提交集合即为去重正确的充分证据）。

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
