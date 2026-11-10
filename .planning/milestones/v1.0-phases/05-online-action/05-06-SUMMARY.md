---
phase: 05-online-action
plan: 06
subsystem: action-query
tags: [rust, trino, postgresql, source, authorization, tdd]
requires:
  - phase: 05-05
    provides: Action 单事务命令、edit、log、outbox 与幂等回执
provides:
  - 按实际字段需求读取 source，重复 logical pk 拒写，完整 edit overlay 零 Engine 调用
  - 公共 releaseObject 规则、锁内 source 可见性校验与 Query 到 Action 的 source-backed token 往返
  - 真实 Trino/PostgreSQL 四分支 Action 集成测试及 PostgreSQL 插入竞态测试
affects: [05-07, ACTION-08, ACTION-09, ACTION-12]
actuals:
  tokens: 21190
  tasks: 2
  commits: 7
commits: 7
plan_head_before: 6ac95b4838fefcdbfd6df7c50b1ec0879a54f2fa
plan_head_after: c360dd6ec129915a960f367b33249d91f2e9a0a8
tech-stack:
  added: []
  patterns: [按需 source read-set, Query 共享 pk overlay, 事务前 bounded LIMIT 2, 锁内 edit CAS]
key-files:
  created:
    - crates/ontoforge-action/src/source.rs
    - crates/ontoforge-action/tests/trino_action_it.rs
  modified:
    - crates/ontoforge-action/src/lib.rs
    - crates/ontoforge-action/src/rules.rs
    - crates/ontoforge-action/src/criteria.rs
    - crates/ontoforge-action/src/version.rs
    - crates/ontoforge-action/tests/source.rs
    - crates/ontoforge-query/src/lib.rs
    - crates/ontoforge-schema/src/ir/action.rs
    - crates/ontoforge-schema/src/validate/actions.rs
key-decisions:
  - source 只在所需属性缺失、source-backed token 需要样本或 releaseObject 时采样；同一 RowKey 在批次内去重。
  - source 样本在 write transaction 前取得；锁内复核 edit version 和基于该样本的 scope，不二次读取 Engine。
  - Task 2 的集成断言首次有效运行即通过，如实记录缺少独立 RED，不制造失败证据。
requirements-completed: [ACTION-08, ACTION-09, ACTION-12]
coverage:
  - id: ACTION-RELEASE-SOURCE
    description: releaseObject 锁内核验 source-backed row 的写后可见性，拒绝无持久化副作用。
    requirement: ACTION-08
    verification:
      - kind: integration
        ref: crates/ontoforge-action/tests/source.rs#release_source_visibility_guard
        status: pass
    human_judgment: false
  - id: ACTION-SOURCE-TOKEN
    description: Query 签发的 source-backed token 在 Action 同源样本下通过，source 漂移或 edit 插入竞态拒写。
    requirement: ACTION-09
    verification:
      - kind: integration
        ref: crates/ontoforge-action/tests/source.rs#source_backed_query_token_roundtrip_and_source_drift
        status: pass
      - kind: integration
        ref: crates/ontoforge-action/tests/source.rs#source_guard_absent_target_insert_race
        status: pass
    human_judgment: false
  - id: ACTION-ON-DEMAND-TRINO
    description: 真实 Trino 四分支区分唯一、断连、观测重复 pk 与全 patch 零调用；失败前不写 PostgreSQL。
    requirement: ACTION-12
    verification:
      - kind: integration
        ref: crates/ontoforge-action/tests/trino_action_it.rs#trino_action_
        status: pass
    human_judgment: false
duration: 63min
completed: 2026-10-23
status: complete
---

# Phase 5 Plan 06: 按需 source Action Summary

Action 按字段需求复用 Query 的 bounded pk source 读取和 overlay；真实 Trino 与 PostgreSQL 证实断连、重复 pk 和离线全 patch 的事务边界。

## Accomplishments

- source-backed 对象先读取 edit overlay，只在所需字段缺失、token 需要 source 样本或 release 时查询 Engine；相同 RowKey 去重，调用数超限前拒绝继续访问。
- Query 与 Action 共用 `read_pk_source` 和 `overlay_pk`。实际采样用 `LIMIT 2`，观测两行返回 `AmbiguousObject`；Engine 错误对外稳定为 `EngineUnavailable`。
- 公共 `releaseObject` 写 storage release；锁内 guard 使用事务前 source 样本校验写后可见性。Query 的 source-backed `_version` token 在 Action 验证，source 漂移与独立 PostgreSQL 连接的同 key 插入竞态均拒写。
- 真实 Trino/PostgreSQL 四个 ignored tests 经 `--ignored` 实际执行，隔离的 source/edit schema 在通过时清理。

## TDD Gate Compliance

- Task 1 的 RED 提交 `01af076`、`c345f55`、`7c5083b` 先于 GREEN `b184215`。原生 JUnit 记录 `target/gsd-05-06-task1-red.json` 与 `target/gsd-05-06-release-red.json` 已获 `RED_EVIDENCE_OK`；目标断言的语义由执行时检查。
- Task 2 的四个真实集成测试在修正测试编译和 fixture DDL 后，首次执行到行为断言即 4/4 GREEN。原因是 Task 1 的实现已包含这些行为；编译失败和 fixture 错误均不是有效 RED。集成测试提交为 `c360dd6`，Task 2 未取得独立 RED，按 GSD fail-fast 规则如实标记此偏差。Grok 只读第二意见支持保留真实 GREEN 证据并记录偏差，不人工制造 RED。
- 计划 ledger 从 `6ac95b4` 到 `c360dd6` 实测 7 个提交，其中 2 个为主会话同步的文档提交；`actuals.tokens` 按代码实际 diff 的 84,757 字符除以 4 计算。

## Verification

- `cargo test -p ontoforge-action --test trino_action_it --locked -j 2 trino_action_ -- --ignored --nocapture`：真实 Trino/PostgreSQL 4 passed、0 filtered。
- `cargo test -p ontoforge-action --features embedded-pg --test source --locked -j 2 -- --nocapture`：9 passed，包括真实 PostgreSQL 的 absent-target 插入竞态。默认 source 套件另有 8 passed。
- `cargo test -p ontoforge-query --test pk_read --locked -j 2 -- --nocapture`：10 passed。release schema 定向测试 2 passed。
- `cargo clippy -p ontoforge-query -p ontoforge-action --all-targets --features embedded-pg --locked -j 2 -- -D warnings`、`cargo fmt --all --check` 与 `git diff --check` 通过。
- 不带 `embedded-pg` 的计划 clippy 命令及独立 `cargo check -p ontoforge-action --locked` 因现有 feature 组合失败，详见 `deferred-items.md`；未将失败记为通过。

## Deviations from Plan

- Task 2 计划中的独立 RED 未出现，因为实现已在 Task 1 GREEN 中完成；真实 backend 测试作为实现后集成验证提交。未以测试编译或 fixture 初始化错误替代行为 RED。
- 现有 Cargo feature 组合问题不属于本计划引入的代码，记录于 `deferred-items.md`，未扩展修改范围。

## Known Stubs

None. 本计划文件的 placeholder/TODO 扫描未发现阻止目标达成的 stub。

## Next Phase Readiness

ACTION-08/09/12 的源读取、release、token 和真实后端证据已就位；05-07 可继续宿主/CI 接线。正式 phase verification 仍需独立执行。

## Self-Check: PASSED

关键 source、Trino test、SUMMARY、deferred 文件均存在；`01af076`、`c345f55`、`7c5083b`、`b184215`、`c360dd6` 均为当前 HEAD 的祖先。
