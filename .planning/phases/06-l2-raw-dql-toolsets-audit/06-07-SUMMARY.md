---
phase: 06-l2-raw-dql-toolsets-audit
plan: 07
subsystem: query
tags: [l2, discovery, trino, acl, read-audit, tdd]
requires:
  - phase: 06-l2-raw-dql-toolsets-audit
    provides: "06-06 的固定主体 L2Service、Trino ACL health 与 ReadAudit guard"
provides:
  - "list_catalogs/list_schemas/list_tables/get_table_schema 四个授权元数据发现 API"
  - "受控 Trino 元数据 SQL、grant 过滤、结果上限与逐操作审计"
affects: [06-08, 06-09, phase-07]
actuals:
  tokens: 10588
  tasks: 2
  commits: 4
commits: 4
plan_head_before: c1ba9bb05d1a951257bb7c356a05dc1dc0888cbe
plan_head_after: a86c07cedceaab2d22e7d1ddf82ef752c0780168
tech-stack:
  added: ["sha2 0.10.9 dev-dependency，复用已有锁文件版本"]
  patterns: ["发现操作在 ACL/grant/审计预约后才发送受控元数据 SQL", "类型化 TableRef 描述语句与审计哈希一致"]
key-files:
  created:
    - crates/ontoforge-query/tests/l2_discovery.rs
    - .planning/phases/06-l2-raw-dql-toolsets-audit/06-07-RED-task1-junit.json
    - .planning/phases/06-l2-raw-dql-toolsets-audit/06-07-RED-task2-junit.json
  modified:
    - crates/ontoforge-engine/src/printer/l2.rs
    - crates/ontoforge-engine/src/trino/mod.rs
    - crates/ontoforge-query/src/l2.rs
    - crates/ontoforge-query/Cargo.toml
    - Cargo.lock
key-decisions:
  - "发现入口保留具体 TrinoEngine 的窄 API；只发送可信 SHOW 或 TableRef 类型化描述语句。"
  - "schema 授权同时接受 catalog 级与 catalog.schema 级 grant，system、information_schema 和 edit catalog 硬排除。"
requirements-completed: []
coverage:
  - id: D1
    description: "catalog/schema 发现仅返回有效 grant 的名称，拒绝与源错均逐项审计"
    requirement: L2-01
    verification:
      - kind: integration
        ref: "crates/ontoforge-query/tests/l2_discovery.rs#l2_catalog_"
        status: pass
    human_judgment: false
  - id: D2
    description: "table/字段发现采用受控 SQL、授权过滤、结果上限与实际 SQL 哈希审计"
    requirement: L2-03
    verification:
      - kind: integration
        ref: "crates/ontoforge-query/tests/l2_discovery.rs#l2_table_"
        status: pass
    human_judgment: false
duration: 40min
completed: 2026-10-28
status: complete
---

# Phase 6 Plan 7: 授权元数据发现 Summary

**四个 L2 元数据发现操作复用 ACL、grant 与审计门禁，受控 SQL 查询只释放授权名称和有界字段结果。**

## Accomplishments

- `list_catalogs`、`list_schemas`、`list_tables`、`get_table_schema` 在 ACL 未验证、无 `sql:raw`、越界 catalog/schema 时拒绝且零元数据 POST。
- `SHOW` 命令只由 printer 构造并转义标识符；字段查询复用 `describe_tables(TableRef)`，审计哈希取实际类型化 SQL。结果过滤、排序、去重、行/字节上限和截断提示由库级入口完成。
- 四入口分别记录成功、拒绝、源错、截断和取消；审计不可用时不释放结果。

## Task Commits

- Task 1 RED `647c250`，GREEN `96dad72`：catalog/schema 授权发现。
- Task 2 RED `900d109`，GREEN `a86c07c`：table/字段发现与审计边界。

## TDD Gate Compliance

- Task 1 的目标 `l2_catalog_filters_and_audits_catalogs_and_schemas` 在 ACL 未验证的 `Disabled` 断言处失败，脚手架实际返回 `InvalidSql`；`06-07-RED-task1-junit.json` 经 RED 分类为 `RED_EVIDENCE_OK`。
- Task 2 的目标 `l2_table_lists_only_granted_tables_and_describes_columns` 在同一预期门禁处失败，脚手架实际返回 `InvalidSql`；`06-07-RED-task2-junit.json` 经 RED 分类为 `RED_EVIDENCE_OK`。两次均是目标行为断言失败，非编译或 fixture 错误。GREEN 后发现测试 9/9 通过；无独立 REFACTOR 提交。

## Verification

- `cargo test -p ontoforge-query --test l2_discovery --locked -- --nocapture`：9/9，通过 catalog/schema/table/字段、拒绝零 POST、源错、截断、取消与审计故障矩阵。
- `cargo test -p ontoforge-query -p ontoforge-engine --locked -j 2 -- --test-threads=1`：退出 0，所有非 ignored 测试与文档测试通过。需要外部 Trino/PG 环境的原有测试按配置 ignored。
- `cargo clippy -p ontoforge-query -p ontoforge-engine --all-targets --locked -- -D warnings`、`cargo fmt --all -- --check`、`git diff --check`：通过。

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] 测试 SQL digest 依赖与 Clippy 合规**
- Task 2 为核对审计哈希与实际字段查询语句一致，增加 query crate 的 `sha2 0.10.9` dev-dependency。仓库锁文件已含该版本，未新增运行时依赖或执行包安装。
- `discover_checked` 超过现有 Clippy 行数限制，授权判断提成两个小方法；测试的局部 `sha2` 导入上移。Clippy 与发现测试复验通过，均在 `a86c07c` 内。

## Known Stubs

None. Phase 7 的 REST/MCP 外壳与 Phase 6 的真实受限 Trino/PG 环境门禁不属于本 PLAN；需求的阶段级完成状态等待后续验证。

## Next Phase Readiness

`06-08` 可以针对六个 L2 操作在受限 Trino 483 与 PostgreSQL 环境补 CI 门禁。原有未跟踪 Phase 2 diagnostics 和 `docs/` 未碰。

## Self-Check: PASSED

四个源码/测试产物、两份 RED 证据和本 SUMMARY 均存在；四个任务提交均是当前 HEAD 的祖先。ledger 基点 `c1ba9bb..a86c07c` 实测四个任务提交，与 frontmatter 一致。
