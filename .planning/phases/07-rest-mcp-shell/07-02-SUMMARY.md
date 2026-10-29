---
phase: 07-rest-mcp-shell
plan: 02
subsystem: storage-action-history
tags: [rust, sqlite, postgres, snapshots, actions, history]
requires: [07-01]
provides: [request-snapshot-module, snapshot-aware-action-api, snapshot-aware-history-api]
affects: [07-03, 07-13]
tech-stack:
  added: []
  patterns: [shared-definition-lock, frozen-active-definition, wrapper-delegation]
key-files:
  created:
    - crates/ontoforge-storage/src/request_snapshot.rs
    - crates/ontoforge-storage/tests/request_snapshot.rs
    - crates/ontoforge-action/tests/frozen_definition.rs
    - crates/ontoforge-storage/tests/frozen_history.rs
  modified:
    - crates/ontoforge-storage/src/lib.rs
    - crates/ontoforge-action/src/lib.rs
    - crates/ontoforge-storage/src/history.rs
decisions:
  - "Action snapshot-aware APIs receive the caller-owned ActiveDefinition; legacy wrappers read the current definition once and delegate."
  - "History wrapper preserves visibility short-circuit before active-definition lookup."
metrics:
  duration: "~51m"
  completed: 2026-10-29
  estimateTokens: 26000
  actuals:
    tokens: 6473
    tasks: 2
    commits: 3
  commits: 3
  plan_head_before: 46f0058c0ceafcb8ae7ab0e076b555e2e15f2474
  plan_head_after: b8806e0057cdbe648fd3fdc6f8c0a640e83307f2
status: complete
---

# Phase 07 Plan 02: 固定请求快照与 Action/history 入口 Summary

完成 storage metadata snapshot 模块化、双后端共同锁读取测试，以及 Action/result/history 的 snapshot-aware 入口。旧 wrapper 保留并复用同一内部路径，定义 fence、CAS、guard、当前授权和原 receipt/time 语义保持不变。

## Delivered

- 将 `RequestSnapshot` 与 `EditStore::request_snapshot` 从 `lib.rs` 移到 `request_snapshot.rs`，继续复用 `reload::begin_definition_tx`，在返回前提交或回滚事务。
- 增加 SQLite snapshot active definition + scope 一致性测试、短锁释放测试和真实 PG ignored 测试入口。
- 增加 `ActionService::execute_with_snapshot`、`get_command_result_with_snapshot`，旧 `execute`、`get_command_result` wrapper 读取当前定义后委托。
- 增加 `EditStore::history_with_snapshot`，历史读取仍按当前调用 scope 做可见性遮蔽并保留 keyset/整记录分页。
- 增加 stale submit/validate-only 零业务写、replay 当前授权保留原 receipt、跨定义 replay 时间稳定、history snapshot 可见性的真实测试。

## TDD Evidence

- Task 1 characterization/RED：首次运行 2 SQLite 用例中 1 个因测试 fixture 未声明 `homeAirports` attribute 失败，修正 fixture 后 GREEN；实现来自 07-01 已有行为，没有制造伪 RED。
- Task 1 GREEN：`request_snapshot` 2 passed，PG 1 ignored（无 `ONTOFORGE_PG_URL`）。
- Task 2 GREEN：`frozen_definition` 3 passed；`idempotency` 9 passed；`frozen_history` 1 passed；`history_masking` 12 passed，PG 1 ignored。
- Workspace：`cargo test --workspace --locked -- --test-threads=1` exit 0；执行测试全部通过，环境依赖的 PG/Trino/private 用例按既有合同 ignored。

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] 保留 history visibility short-circuit**

- **Found during:** Task 2 regression
- **Issue:** 新 wrapper 先读取 active definition，破坏 `invisible_check_precedes_store_access`。
- **Fix:** `history` 在 active lookup 前继续调用 `visible_columns(req)?`。
- **Files modified:** `crates/ontoforge-storage/src/history.rs`
- **Commit:** `a4311bc`

## Verification

- `cargo fmt --all -- --check` passed。
- `cargo clippy --workspace --all-targets --locked -- -D warnings` passed。
- `cargo clippy -p ontoforge-storage --features embedded-pg --all-targets --locked -- -D warnings` passed。
- TLS gate 未发现 `native-tls` package；`cargo tree ... -i native-tls` 以 exit 101 表示依赖图中无该 package。
- Warp/`ralph-loop` 在本机不可用，未声称运行其 completion promise。

## Commits

- `6efa315`: `feat(07-02): 固定 storage metadata request snapshot`
- `a4311bc`: `feat(07-02): 接通 frozen Action 与 history 入口`
- `b8806e0`: `fix(07-02): 格式化并收敛测试 gate`

## Self-Check: PASSED

- Summary 文件已创建。
- 三个 commit 均可在当前 HEAD 的 ancestry 中找到。
- 生产改动仅涉及计划允许的七个文件；共享状态文件未修改。
