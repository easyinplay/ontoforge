---
phase: 06-l2-raw-dql-toolsets-audit
plan: 09
subsystem: query
tags: [reload, scope, l0, l2, audit, sqlite, tdd]
requires:
  - phase: 06-l2-raw-dql-toolsets-audit
    provides: "06-01 读审计、06-05 L0 审计、06-06 L2 执行"
provides:
  - "管理审计写入故障的 role、binding、索引事务回滚回归"
  - "L0/L2 请求在途 reload 与 scope apply 的数据、授权和审计快照回归"
affects: [06-08, phase-06-verification]
actuals:
  tokens: 6019
  tasks: 2
  commits: 5
commits: 5
plan_head_before: 7de476787b4f41c87d010d1899642cc92212d2be
plan_head_after: 6ced9a61b50ca70d9b930b8bdbe66afb52821fef
tech-stack:
  added: []
  patterns: ["Notify 与受控 HTTP 响应阻断请求，再验证管理变更前后的两个快照"]
key-files:
  created:
    - crates/ontoforge-query/tests/definition_snapshot.rs
    - .planning/phases/06-l2-raw-dql-toolsets-audit/06-09-RED-task1-junit.json
    - .planning/phases/06-l2-raw-dql-toolsets-audit/06-09-RED-task2-junit.json
  modified:
    - crates/ontoforge/tests/cli_reload.rs
    - crates/ontoforge-storage/tests/scope_store.rs
key-decisions:
  - "已有生产路径满足快照合同，本计划只提交回归测试，不制造无意义生产改动。"
patterns-established:
  - "并发测试先确认源请求确实阻断，管理变更完成后才释放旧请求。"
requirements-completed: []
coverage:
  - id: D1
    description: "reload 失败和 scope 审计失败不产生部分管理状态，成功管理变更写审计"
    requirement: AUDIT-01
    verification:
      - kind: integration
        ref: "crates/ontoforge/tests/cli_reload.rs#reload_reports_role_breakage"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/scope_store.rs#audit_insert_failure_rolls_back_scope_and_indexes"
        status: pass
    human_judgment: false
  - id: D2
    description: "L0 和 L2 在途读结果及审计固定旧定义和角色，后续请求使用新快照"
    requirement: SCOPE-07
    verification:
      - kind: integration
        ref: "crates/ontoforge-query/tests/definition_snapshot.rs#l0_inflight_reload_and_scope_apply_keep_request_snapshot"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-query/tests/definition_snapshot.rs#l2_inflight_reload_and_scope_apply_keep_request_snapshot"
        status: pass
    human_judgment: false
duration: 28min
completed: 2026-10-28
status: complete
---

# Phase 6 Plan 09: 管理边界与并发快照回归 Summary

scope 管理事务失败整体回滚；L0/L2 在途读取跨越 scope apply 和 reload 时仍以旧授权返回并审计，后续请求使用新快照。

## Accomplishments

- 审计插入故障时 role、binding、索引均回滚；reload 失败保留原定义及角色。既有 CLI scope 测试覆盖错误 hash、成功 apply 和管理审计。
- L0 的 SQLite 源查询与 L2 的 Trino mock 响应在受控点阻断。管理变更完成后释放请求，旧请求返回并记旧 `def_version` 和 `effective_roles`，新请求被拒并记新版本与空角色。
- L2 public service 仍仅有四个 discovery 和两个只读 SQL 操作，没有管理写入口。

## Verification

- `cargo test -p ontoforge --test cli_reload --locked -- --nocapture`：8 passed。
- `cargo test -p ontoforge --test cli_scope --locked -- --nocapture`：8 passed。
- `cargo test -p ontoforge-storage --test scope_store --locked -- --nocapture`：12 passed、3 ignored。
- `cargo test -p ontoforge-query --test definition_snapshot --locked`：2 passed。
- `cargo clippy -p ontoforge-query -p ontoforge-storage -p ontoforge --tests --locked -- -D warnings`：passed。
- `cargo fmt --all --check`：passed。

## Task Commits

- Task 1：`31fecb2`、`4cc88d3`；格式修正 `f1c80c6`。
- Task 2：`8b21089`；Clippy 和 RED 语义记录修正 `6ced9a6`。

## TDD Gate Compliance

- Task 1：临时将 `scope_apply` 审计失败路径改为 commit，目标断言观察 role 数为 3、期望 0；`06-09-RED-task1-junit.json` 经 GSD 判为 `RED_EVIDENCE_OK`，随后恢复生产源码。测试通过后提交回归测试。
- Task 2：临时使 L2 审计丢弃 `effective_roles`；目标断言实际 `[]`、期望 `["dispatcher"]`，`06-09-RED-task2-junit.json` 经 GSD 判为 `RED_EVIDENCE_OK`。恢复源码后两项并发测试通过。
- 本计划是已有行为的回归探针，没有生产修复，也没有 `feat(06-09)` 提交。按 GSD 的 TDD commit-pattern 检查会标记缺少 GREEN 类型提交；实际 GREEN 测试已通过，不应通过空提交伪造生产变更。
- 2026-10-28 门禁对账：`06-09-PLAN.md` 的文件清单仅含 `tests/`，执行期规则将 test-only task 排除在生产 GREEN 提交要求之外。将 PLAN frontmatter 从 `type: tdd` 显式更正为 `type: execute`，保留两个任务的 `tdd="true"`、RED mutation 证据和实际 GREEN 测试，不修改生产源码或伪造 `feat` 提交。

## Deviations from Plan

- [Rule 3 - Blocking] 两项集成测试的 fixture 与受控并发场景超过 Clippy 的 100 行限制，按仓库已有模式为两个测试函数分别标注局部 `too_many_lines` 理由；`-D warnings` 复查通过。
- Task 1 的新回滚测试需要格式调整，单独提交 `f1c80c6` 后格式检查通过。

## Skipped Integration Tests

`scope_store` 的既有 `pg_audit_is_append_only_and_login_is_recorded`、`pg_concurrent_scope_apply_serializes`、`pg_scope_apply` 在没有 `ONTOFORGE_PG_URL` 或 `embedded-pg` feature 时按测试定义 ignored。本计划的 SQLite 管理回归和两种读快照已实际运行；这三项 PostgreSQL 条件测试尚无本轮通过证据，交由 `06-08` 的 PostgreSQL 验证处理。GSD 已在 `.planning/WINDOWS.md` 记录 skipped-test 条目 12、13、14。

## Known Stubs

None.

## Self-Check: PASSED

新增测试、两份 RED 证据、SUMMARY 和 WINDOWS ledger 均存在；五个任务提交均为当前 HEAD 的祖先，`git diff --check` 通过。
