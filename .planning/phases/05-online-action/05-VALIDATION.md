---
phase: "05"
slug: "online-action"
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-10-23"
---

# Phase 05 - Validation Strategy

> 执行期反馈与收尾验收合同；实际任务映射由 `05-*-PLAN.md` 产出后补全。

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

| Slice | Requirement | Secure behavior / failure | Test type | Automated command | File exists | Status |
|-------|-------------|---------------------------|-----------|-------------------|-------------|--------|
| 参数与 rule | ACTION-01..03,08 | 逐参数错误、masked criteria、前置隐藏与不存在同错、写后不可见 | unit + SQLite | `cargo test -p ontoforge-action --locked` | W0 pending | pending |
| 原子批量 | ACTION-02,04..06 | 逐项摘要、后项失败回滚、validate-only 零持久化 | SQLite + real PG | `cargo test -p ontoforge-storage --test conformance_sqlite --locked`；PG 目标 | W0 pending | pending |
| 写入口 | ACTION-07 | 非 Action principal 写被拒，审计导入保留 | source/API audit + integration | Action 与 storage 定向目标，命令由 PLAN 固定 | W0 pending | pending |
| scope 与版本 | ACTION-08,09 | 读集锁序、同 pk 并发仅一成功、query 令牌通用 | real PG + security integration | 新 Action PG 双连接目标，命令由 PLAN 固定 | W0 pending | pending |
| 幂等 | ACTION-10 | 同键竞态一日志一 outbox、异参拒绝、失权重放拒绝、删除后回执 | SQLite + real PG | 新 command race 目标，命令由 PLAN 固定 | W0 pending | pending |
| outbox | ACTION-11 | 提交后崩溃、HTTP 失败、发送后标记前崩溃均可重试同 event_id | `wiremock` + DB reopen | 新 worker 集成目标，命令由 PLAN 固定 | W0 pending | pending |
| source | ACTION-12 | 实际观测重复 pk 拒写、按需 Trino、全 patch 离线零调用 | fake Engine + real Trino | 新 `trino_action_it` 目标，命令由 PLAN 固定 | W0 pending | pending |

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
