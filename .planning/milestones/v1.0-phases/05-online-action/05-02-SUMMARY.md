---
phase: 05-online-action
plan: 02
subsystem: action-authorization
tags: [rust, action, scope, cel, sqlite, postgresql, tdd]
requires:
  - phase: 05-01
    provides: Action transaction tracer, command receipt and outbox
provides:
  - parameter validation and five ordered Action rule kinds
  - transaction-locked masked criteria and pre/post scope guard
  - rollback-only validate-only outcome
affects: [05-03, 05-04, 05-05, 05-06, 05-07]
actuals:
  tokens: 25357
  tasks: 2
  commits: 10
commits: 10
plan_head_before: 8c65ab13e862cac00b7fe7d278541278dc4d0296
plan_head_after: 08014bd52efaf0d3393987d6ae8f18d4114a8259
tech-stack:
  added: [cel dependency edge in ontoforge-action]
  patterns: [stable RowKey lock order with per-edit guard state, discriminated ActionOutcome]
key-files:
  created:
    - crates/ontoforge-action/src/params.rs
    - crates/ontoforge-action/src/rules.rs
    - crates/ontoforge-action/src/criteria.rs
    - crates/ontoforge-action/tests/security.rs
  modified:
    - crates/ontoforge-action/src/lib.rs
    - crates/ontoforge-action/tests/action.rs
    - crates/ontoforge-storage/src/commit.rs
    - crates/ontoforge-storage/tests/conformance/mod.rs
key-decisions:
  - Use ActionOutcome::Committed(receipt) or Validated; validation never invents audit IDs.
  - Recheck masked criteria and scope with a synchronous read-only guard under storage locks after CAS.
  - Verify each intermediate edit state in ordinal order, while locking distinct rows in RowKey order.
  - Verify release source visibility in 05-06 when its Action rule and source sampler exist.
requirements-completed: [ACTION-01, ACTION-02, ACTION-03, ACTION-04, ACTION-05, ACTION-06]
coverage:
  - id: ACTION-PARAMS-RULES
    description: Parameter errors and five ordered rule kinds produce an atomic batch and ordinal receipt.
    requirement: ACTION-01
    verification:
      - kind: integration
        ref: cargo test -p ontoforge-action --locked -- --nocapture
        status: pass
    human_judgment: false
  - id: ACTION-MASKED-GUARD
    description: Hidden and missing targets share an outward error; masked criteria and denied writes fail under the lock.
    requirement: ACTION-03
    verification:
      - kind: integration
        ref: cargo test -p ontoforge-action --test security --locked scope_ -- --nocapture
        status: pass
    human_judgment: false
  - id: ACTION-VALIDATE
    description: Validate-only runs the current rule authorization path and leaves no database writes or sequence gaps.
    requirement: ACTION-04
    verification:
      - kind: integration
        ref: cargo test -p ontoforge-action --test action --locked validate_only_ -- --nocapture
        status: pass
    human_judgment: false
duration: 1h 26m
completed: 2026-10-23
status: complete
---

# Phase 5 Plan 02: Action Semantics and Authorization Summary

参数校验和当前五类 Action rule 已接入原子批次；scope、遮蔽视图 criteria、link 两端与逐 edit 写后状态在 storage 锁内审核，validate-only 使用同一 guard 后回滚。

## Accomplishments

- 参数错误跨 item 汇总；默认值、objectReference、显式请求指纹与 trusted limits 各守原有合同。五类规则按 item/rule/edit ordinal 展平，同 pk 连续修改逐步增版，receipt 保留原顺序。
- storage 在稳定 RowKey 锁序与 observed-version CAS 后提供逐 edit before/after 和 guard read set。Action 在此只读回调中校验目标、可见列、masked CEL criterion、每个中间态及 link 两端。
- `ActionOutcome::Validated` 不携带虚构的 receipt。成功与拒绝的 dry-run 均不保留 object、edit、log、command、outbox；成功 dry-run 后真实提交的 `log_seq` 连续。

## TDD Gate Compliance

- Task 1 RED：`cbbf8fa`、`99100bf`、`bf9b363`、`80f0324` 固定参数汇总、规则顺序、扩展上限及跨 item 错误。`target/gsd-05-02-task1-*-red.json` 与基础记录均由 `gsd-tools check tdd-red-evidence` 返回 `RED_EVIDENCE_OK`，每次 1 个目标断言失败。GREEN `04a58e5`。
- Task 2 RED `90343d3`：隐藏目标、遮蔽 criterion 与 validate-only 的三份原生 JUnit 记录均返回 `RED_EVIDENCE_OK`，每次 1 个目标断言失败。GREEN `08014bd`。`147083c` 明确 `ActionOutcome` 合同，`13b694f` 将不可达的 release/source 专项移至 05-06。
- 计划 ledger 从 `8c65ab1` 到 `08014bd` 测得 10 个提交，其中包含主会话的计划、决策和进度文档提交；不是 10 个实现提交。

## Verification

- `cargo test -p ontoforge-action --locked -- --nocapture`：lib 3/3、action 6/6、security 10/10，通过；仅 Doc-tests 目标为 0 个测试。
- `cargo test -p ontoforge-storage --test conformance_sqlite --locked action_batch_ -- --nocapture`：5/5。
- `cargo test -p ontoforge-storage --features embedded-pg --test conformance_pg --locked action_batch_ -- --ignored --nocapture`：真实 embedded PostgreSQL 5/5。
- `cargo clippy -p ontoforge-action --all-targets --locked -- -D warnings`、`cargo fmt --all --check`、`git diff --check`：通过。

## Scope and Deviations

- 05-02 只验收当前可达的 create/modify/delete object、add/remove link。release/source Action IR、source snapshot 可见性及 Engine 调用同路径由 05-06 的真实 RED/GREEN 验收；ACTION-08 因包含 release 暂不标完成。
- 相比原计划的独立 `execute.rs`，现有 Action 服务继续留在 `lib.rs`，避免搬移公共入口。锁内纯同步 guard 扩展了 storage 事务 API，以闭合 scope/criteria 的 TOCTOU 窗口。
- 当前未安装 `ralph-loop`；各子任务用 GSD RED 证据、GREEN 回归、原子提交与本 SUMMARY 的迭代验收代替，未声称运行该工具。

## Self-Check: PASSED

所列新文件存在；Task 1/2 RED、GREEN 提交均为当前 HEAD 祖先；ledger 提交数为 10，代码改动已提交。
