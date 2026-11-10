---
phase: 06-l2-raw-dql-toolsets-audit
plan: 05
subsystem: query
tags: [audit, l0, sqlite, postgres, tdd]
requires:
  - phase: 06-l2-raw-dql-toolsets-audit
    provides: "06-01 的有界读审计 guard 与双后端 sink"
provides:
  - "公开 list/filter/set/searchAround 和 aggregate 的单请求单审计门禁"
  - "聚合审计不可用时保留 AuditUnavailable 错误身份"
affects: [06-06, 06-08, 06-09]
actuals:
  tokens: 11191
  tasks: 2
  commits: 5
commits: 5
plan_head_before: c779fbb960c22db009117722b6c72983745c6d8f
plan_head_after: 10b533318f1a655b51400ce94548303f17159188
tech-stack:
  added: []
  patterns: ["公开读入口 reserve/finish；内部流水线共用 WorkBudget 审计 guard"]
key-files:
  created:
    - crates/ontoforge-query/tests/audit_l0.rs
    - .planning/phases/06-l2-raw-dql-toolsets-audit/06-05-RED-task1-junit.json
    - .planning/phases/06-l2-raw-dql-toolsets-audit/06-05-RED-task2-junit.json
  modified:
    - crates/ontoforge-query/src/execute.rs
    - crates/ontoforge-query/src/aggregate.rs
key-decisions:
  - "审计 guard 随 WorkBudget 穿过 list、set、link 和 aggregate 内部调用，公开入口负责一次性结束审计。"
  - "AggregateError 独立保留 AuditUnavailable，禁止将审计故障映射为 InvalidQuery。"
requirements-completed: []
coverage:
  - id: D1
    description: "list、过滤、set 与 searchAround 单次审计及拒绝、源错、截断、取消、sink 故障关门"
    requirement: AUDIT-01
    verification:
      - kind: integration
        ref: "crates/ontoforge-query/tests/audit_l0.rs#audit_list_"
        status: pass
    human_judgment: false
  - id: D2
    description: "数据库部分聚合与 Rust 分解聚合单条审计、失败隔离和错误身份"
    requirement: AUDIT-01
    verification:
      - kind: integration
        ref: "crates/ontoforge-query/tests/audit_l0.rs#audit_aggregate_"
        status: pass
    human_judgment: false
  - id: D3
    description: "真实 PostgreSQL edit-only list 在系统审计表写入单条记录"
    requirement: AUDIT-01
    verification:
      - kind: integration
        ref: "crates/ontoforge-query/tests/audit_l0.rs#audit_list_pg_only_edit_store_records_once"
        status: pass
    human_judgment: false
duration: 38min
completed: 2026-10-26
status: complete
---

# Phase 6 Plan 05: L0 公开读取审计 Summary

公开 list/filter/set/searchAround 和 aggregate 在审计不可受理时拒绝结果，内部查询复用同一 guard，不重复记账。

## Performance

- **Duration:** 38 min（18:02 至 18:40，Asia/Shanghai）
- **Tasks:** 2
- **Files modified:** 5（含两份 RED JUnit 证据）

## Accomplishments

- `execute_list` 在公开边界预留有界审计，递归提取条件 apiName；正常结果、拒绝、源错、截断及取消分别记录对应状态。队满或 sink 健康失败时不释放新结果。
- `execute_aggregate` 对数据库部分聚合及 Rust fallback 各只记录一条审计；`AggregateError::from` 保留 `AuditUnavailable`，避免错误码被折叠。
- 真实 PostgreSQL edit-only 读经独立临时 schema 验证审计记录，测试结束清理 schema。

## Task Commits

- Task 1 RED：`5c8db56`、`7e16a82`；GREEN：`1fcc2b4`。
- Task 2 RED：`bf0b903`；GREEN：`10b5333`。

## TDD Gate Compliance

- Task 1：`audit_list_type_filter_set_and_search_around_record_once` 已实际执行，在第一条 list 结果后因 `expected 1 audit rows, found 0` 失败。`06-05-RED-task1-junit.json` 为未修改的 stdout JUnit 报告，分类器 `RED_EVIDENCE_OK`；语义检查确认失败在预定审计行数断言，不是构建或 fixture 错误。
- Task 2：`audit_aggregate_partial_and_fallback_each_record_once` 已实际执行，在首个部分聚合后因同一预定审计行数断言失败。`06-05-RED-task2-junit.json` 分类器 `RED_EVIDENCE_OK`；取消测试也在缺少审计行的预定断言失败。
- GREEN 后 `audit_l0` 非 ignored 9/9，真实 PG ignored 用例单独运行 1/1。无独立 REFACTOR 提交。

## Verification

- `cargo test -p ontoforge-query --locked -j 2`：退出 0，既有 query/aggregate/pk/set/searchAround 回归通过；现有五个 Trino+PG 外部集成测试仍按环境要求 ignored。
- `cargo test -p ontoforge-query --test audit_l0 --locked -- --nocapture`：9 passed、1 ignored；`audit_list_pg_only_edit_store_records_once -- --ignored --nocapture` 在本机 PostgreSQL 容器通过 1/1。
- `cargo clippy -p ontoforge-query --test audit_l0 --locked -- -D warnings`、`cargo fmt -p ontoforge-query -- --check`：通过。
- 两份 `tdd-red-evidence` 记录复核：`RED_EVIDENCE_OK`。

## Deviations from Plan

None. 测试额外使用本机 PostgreSQL 容器完成计划要求的 PG-only 路径验证，未引入新依赖或服务代码。

## Known Stubs

None. PostgreSQL 集成用例默认标记 ignored，因为普通本地运行不保证有服务；本次已通过真实 PostgreSQL 单独执行。

## Next Phase Readiness

L0 公开读入口已覆盖。`AUDIT-01` 全范围仍需后续计划完成 L2 入口审计与 Phase 6 总验收。

## Self-Check: PASSED

三份源码/测试文件、两份 RED 证据及 SUMMARY 均存在；五个任务提交均为当前 HEAD 的祖先。持久化 ledger 的 `c779fbb..10b5333` 实测提交数为 5，与 frontmatter 一致。
