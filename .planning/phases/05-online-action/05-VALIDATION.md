---
phase: "05"
slug: "online-action"
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-10-23"
---

# Phase 05 - Validation Strategy

> 执行期反馈与收尾验收合同；任务细节与失败信号见 `05-01..07-PLAN.md`。

## Test Infrastructure

| Property | Value |
|----------|-------|
| Framework | Rust `cargo test`、`tokio::test`；HTTP 假端点使用已有 `wiremock` |
| Config file | `Cargo.toml`、`.github/workflows/ci.yml` |
| Quick run command | `cargo test -p ontoforge-action --locked` |
| Storage parity | `cargo test -p ontoforge-storage --test conformance_sqlite --locked`；真实 PG 使用 `conformance_pg -- --ignored` |
| Full suite command | `cargo test --workspace --locked` 加真实 PG、Trino、webhook 集成目标及 `cargo clippy --workspace --all-targets --locked -- -D warnings`、`cargo fmt --all --check` |
| Estimated runtime | 执行期在本机与 CI 实测后记录；当前不猜测 |

## Sampling Rate

- 每个 TDD 子任务：先运行新测试得到 red，再运行相关 crate 的窄目标得到 green。
- 每个 wave：运行 `ontoforge-action` 与 storage SQLite conformance，并运行当前可用的真实 PG/Trino 目标。
- `$gsd-verify-work 05` 前：workspace、真实 PG/Trino、webhook 故障恢复与 CI 同 SHA 全绿；ignored 测试必须明确执行且统计运行数。
- 最大反馈延迟：窄目标和全量目标的基线待 Wave 0 实测；超出时拆分测试入口，不跳过验收。

## Per-Task Verification Map

| Plans | Requirement | Secure behavior / failure | Test type | Representative automated command | Status |
|-------|-------------|---------------------------|-----------|----------------------------------|--------|
| 05-01, 05-02 | ACTION-01..06,08 | 参数矩阵、五类 rule、逐项摘要、masked criteria、validate-only 零写入 | unit + SQLite + real PG | `cargo test -p ontoforge-action --test action --locked batch_ -- --nocapture` | planned; tests pending |
| 05-03, 05-04 | ACTION-08,09 | 共享 token、旧格式失效、稳定读集锁序、双后端故障回滚与同 pk 并发 | SQLite + real PG | `cargo test -p ontoforge-action --features embedded-pg --test action_pg --locked concurrent_same_pk_ -- --ignored --nocapture` | planned; tests pending |
| 05-05 | ACTION-10 | 同键竞态一日志一 outbox、异参拒绝、失权重放拒绝、删除后最小回执 | SQLite + real PG | `cargo test -p ontoforge-action --features embedded-pg --test idempotency_pg --locked pg_same_key_ -- --ignored --nocapture` | planned; tests pending |
| 05-06 | ACTION-12 | 实际观测重复 pk 拒写、按需 Trino、全 patch 离线零调用 | fake Engine + real Trino | `cargo test -p ontoforge-action --test trino_action_it --locked trino_action_ -- --ignored --nocapture` | planned; tests pending |
| 05-07 | ACTION-07,11 | 非 Action principal 写被拒；提交后崩溃、HTTP 失败、发送后标记前崩溃重试同 event_id | source/API audit + `wiremock` + DB reopen | `cargo test -p ontoforge-action --test outbox --locked crash_ -- --nocapture` | planned; tests pending |

## Wave 0 Requirements

- [ ] Action 测试 harness 和各当前 `ParamType` 的参数错误矩阵。
- [ ] SQLite/真实 PG 共用的命令、日志、outbox 原子性断言及确定性故障注入。
- [ ] 真实 PG 双连接 barrier，覆盖同 pk 和同 `command_id` 两种竞态。
- [ ] worker 重启与 `wiremock` 投递故障 fixture。
- [ ] CI 纳入真实 Trino/PG Action 目标并拒绝目标零测试通过。

## Manual-Only Verifications

None planned. 真实服务与故障注入以自动化证据验收。

## Validation Sign-Off

- [ ] 每个 PLAN task 有 `<automated>` verify 或明确的 Wave 0 依赖。
- [ ] 连续 3 个 task 之间不存在测试空窗。
- [ ] Wave 0 建好本文件标出的全部新目标。
- [ ] 真实 PG 并发和 Trino/worker 目标在匹配代码 SHA 的 CI 中运行，未被 skipped target 冒充。
- [ ] `nyquist_compliant: true` 仅在验证证据齐全后设置。

**Approval:** pending
