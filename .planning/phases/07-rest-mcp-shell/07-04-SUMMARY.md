---
phase: 07-rest-mcp-shell
plan: "04"
subsystem: rest-action-history
tags: [rust, axum, rest, action, history, idempotency, tdd]
requires: ["07-03"]
provides:
  - REST Action validate/submit endpoint
  - command receipt lookup and object history endpoint
  - HTTP Action replay and owned permit lifecycle
affects: ["07-05", "07-06", "07-13", "07-14"]
tech-stack:
  added: []
  patterns: [snapshot-aware handlers, tracked background Action task, structured error mapping]
key-files:
  created:
    - crates/ontoforge-server/tests/http_actions.rs
    - crates/ontoforge-server/tests/http_action_safety.rs
    - .planning/phases/07-rest-mcp-shell/07-04-SUMMARY.md
  modified:
    - crates/ontoforge-api/src/dto.rs
    - crates/ontoforge-server/src/context.rs
    - crates/ontoforge-server/src/error.rs
    - crates/ontoforge-server/src/lib.rs
    - crates/ontoforge-server/src/rest.rs
    - crates/ontoforge-server/tests/support/mod.rs
    - crates/ontoforge-query/src/lib.rs
    - crates/ontoforge-storage/src/history.rs
decisions:
  - "正式 Action 按方案 A 进入宿主跟踪任务，HTTP deadline/断连只结束响应等待，任务继续并释放 owned permit。"
  - "GET command result 先按当前 principal 查询 action_command，再调用 snapshot-aware domain API；未知、跨主体、过期、撤权统一 NOT_FOUND。"
  - "History 先通过当前 scope 的 PK 查询建立 current row，再调用 history_with_snapshot，响应显式 entries/truncated/nextBeforeSeq。"
metrics:
  duration: "约 00:55"
  completed_date: "2026-10-29"
  status: complete
estimate:
  tokens: 25000
  raw_tokens: 25000
actuals:
  tokens: 7234
  tasks: 2
  commits: 4
plan_head_before: fb45f384932f944f3e8a818799a09a874df8e0d1
plan_head_after: fc05a617e3216ea9b22dccf95d59726f92f78f3b
commits: 4
---

# Phase 07 Plan 04: REST Action、历史与 receipt Summary

REST 已接入 Action validate/submit、command receipt 查询与对象历史读取。正式 Action 在认证、body 解析、请求 snapshot 和基本校验完成后进入宿主跟踪任务，HTTP deadline 返回 `504` 时任务继续执行；相同 `command_id` 的显式 replay 保留原 receipt/time，异参拒绝且不产生第二次业务提交。

## TDD

### RED

提交 `3f0d10c` 新增两个真实 listener 测试文件。命令 `CARGO_BUILD_JOBS=2 cargo test -p ontoforge-server --test http_actions --locked -- --nocapture` 退出码 `1`，发现 2 个目标测试并实际执行：Action 路由返回 `404` 而预期 `200`，空 Action 路由返回 `404` 而预期 `400`。失败来自行为断言，不是编译、fixture 或零测试。

### GREEN

提交 `e936057` 注册 3 条路由，扩展 wire DTO，实现 Action/receipt/history handler、错误映射与真实 HTTP fixture。`http_actions` 两个目标用例通过。

### REFACTOR

提交 `53203ac` 完成 replay、并发 command race、异参冲突和 validate-only 安全断言，修复 admission permit 在响应完成后的释放，并通过 server 回归。提交 `fc05a61` 仅为格式化测试文件。

## Verification

- `CARGO_BUILD_JOBS=2 cargo test -p ontoforge-server --test http_actions --locked -- --nocapture`：退出码 `0`，2 passed。
- `CARGO_BUILD_JOBS=2 cargo test -p ontoforge-server --test http_action_safety --locked -- --nocapture`：退出码 `0`，3 passed。
- `CARGO_BUILD_JOBS=2 cargo test -p ontoforge-server --locked -- --nocapture`：退出码 `0`，unit 7 passed、http_action_safety 3 passed、http_actions 2 passed、http_contract 3 passed。
- `CARGO_BUILD_JOBS=2 cargo clippy -p ontoforge-server --all-targets --locked -- -D warnings`：退出码 `0`。
- `cargo fmt --all`：退出码 `0`。

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking compile issue] 允许 HistoryRequest 跨 Axum handler future 保持 Send**
- **Found during:** Task 04-1 GREEN
- **Issue:** `HistoryRequest.current_row` 使用不带 `Sync` 的 trait object，history handler 无法满足 Axum `Handler` 的 Send future 约束。
- **Fix:** 将字段改为 `Option<&(dyn RowValues + Sync)>`；现有 `BTreeMap` row 实现满足该约束。
- **Files modified:** `crates/ontoforge-storage/src/history.rs`
- **Commit:** `e936057`

**2. [Rule 3 - Blocking API visibility] 暴露已有 scope scalar 转换函数**
- **Found during:** Task 04-1 GREEN
- **Issue:** REST history 需要把真实查询 row 转成 scope evaluator 的 `Scalar`，已有 `scope_scalar` 仅为 `pub(crate)`。
- **Fix:** 将已有函数提升为 `pub`，未新增逻辑或依赖。
- **Files modified:** `crates/ontoforge-query/src/lib.rs`
- **Commit:** `e936057`

### Deferred

- 真实 PostgreSQL、Trino、private source 联调未在本地环境执行，交由 07-13/07-14。
- Warp 与 `ralph-loop` 不可用，未声称运行其 completion promise。

## Known Stubs

None in files changed by this plan.

## Self-Check: PASSED

- SUMMARY file exists at `.planning/phases/07-rest-mcp-shell/07-04-SUMMARY.md`.
- Commits `3f0d10c`, `e936057`, `53203ac`, `fc05a61` are ancestors of `HEAD`.
- Untracked `.planning/phases/02-source-binding-engine/.review-diagnostics/`, `docs/README.md`, `docs/foundry/`, and `docs/images/` are pre-existing user files and were not modified or staged.
