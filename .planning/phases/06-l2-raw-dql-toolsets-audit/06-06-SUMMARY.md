---
phase: 06-l2-raw-dql-toolsets-audit
plan: 06
subsystem: query
tags: [l2, trino, acl, read-audit, tdd]
requires:
  - phase: 06-l2-raw-dql-toolsets-audit
    provides: "06-01 审计 guard、06-03 受控 SQL gate、06-04 Trino ACL health"
provides:
  - "固定主体、scope、定义版本的 execute_query 和 explain_query 库级 API"
  - "Trino ACL、受控 Display、审计入队与行/字节/时间上限的联合门禁"
  - "双后端 truncated 审计状态的旧库升级路径"
affects: [06-07, 06-08, 06-09]
actuals:
  tokens: 9554
  tasks: 2
  commits: 8
commits: 8
plan_head_before: 0ad7f5e9b64064c2e3fd399a46ac194d94c7bfaa
plan_head_after: 16d5873dff488399262e07a450bea2a5b93f9985
tech-stack:
  added: ["wiremock dev-dependency already present in workspace"]
  patterns: ["库级 L2 入口只接受受控 PrintedSql", "审计 guard 在结果释放前结束，Drop 记录取消"]
key-files:
  created:
    - crates/ontoforge-query/src/l2.rs
    - crates/ontoforge-query/tests/l2_execute.rs
    - crates/ontoforge-storage/migrations/sqlite/0006_audit_truncated.sql
    - crates/ontoforge-storage/migrations/postgres/0006_audit_truncated.sql
  modified:
    - crates/ontoforge-query/src/lib.rs
    - crates/ontoforge-query/Cargo.toml
    - Cargo.lock
    - crates/ontoforge-storage/src/read_audit.rs
    - crates/ontoforge-storage/src/sql_exit.rs
key-decisions:
  - "L2 API 持有独立 TrinoEngine；配置中的 edit catalog 与固定 scope 一同传给 validate_l2，ACL health 在入口与 Engine 发送侧均检查。"
  - "任意 SQL 超限只给收窄提示，不伪造 cursor；truncated 作为独立审计状态持久化。"
requirements-completed: []
coverage:
  - id: D1
    description: "execute_query 的授权、ACL、Display、上下限、故障和取消门禁"
    requirement: L2-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-query/tests/l2_execute.rs#l2_execute_"
        status: pass
    human_judgment: false
  - id: D2
    description: "explain_query 只包装已验证 SQL，复用审计和结果门禁"
    requirement: L2-03
    verification:
      - kind: integration
        ref: "crates/ontoforge-query/tests/l2_execute.rs#l2_explain_"
        status: pass
    human_judgment: false
  - id: D3
    description: "审计截断状态兼容旧 SQLite/PostgreSQL 表"
    requirement: AUDIT-01
    verification:
      - kind: integration
        ref: "crates/ontoforge-query/tests/l2_execute.rs#l2_execute_sqlite_v5_audit_upgrade_preserves_rows"
        status: pass
      - kind: integration
        ref: "PostgreSQL 17.11 临时 schema 0005 -> 0006 旧行保留和 truncated 插入"
        status: pass
    human_judgment: false
duration: 101min
completed: 2026-10-28
status: complete
---

# Phase 6 Plan 06: L2 execute/explain 库级门禁 Summary

库级 `execute_query` 和 `explain_query` 在固定主体、scope、定义版本下复用受控 SQL、独立 Trino L2 身份及有界读审计；拒绝路径不会向 Trino POST。

## Accomplishments

- `L2Service` 从主体 grants 生成 `L2Policy`，在已通过 ACL health 时调用 `validate_l2`，仅将 `PrintedSql` 或其 `explain_l2` 包装交给 `TrinoEngine`。Engine 发送前再次检查 health；权限、语法、审计与源错误保持不同错误码。
- 两个公开操作在审计预约失败时不放行结果；成功、拒绝、源错误、截断与请求取消分别记录状态。结果受配置的行数、字节与超时限制，超限只返回收窄提示，不假造 keyset cursor。
- `truncated` 需持久化为独立状态，原 SQLite/PostgreSQL 表的 CHECK 约束不接纳它。两份增量迁移扩展 CHECK，SQLite 复制旧行与 seq 后重建索引，PostgreSQL 原位替换约束；旧库升级均经实测。

## Task Commits

- Task 1 RED：`a116931`、`4c44c9d`；GREEN：`9cd8ff0`。
- Task 2 RED：`d85fc98`；GREEN：`9084c45`。
- 补充上限/超时测试：`b0237ff`；回归修正：`30942cf`、`16d5873`。

## TDD Gate Compliance

- Task 1：`l2_execute_only_submits_display_sql_after_attestation_and_audits_result` 在尚未验证 ACL 时期待 `Disabled`，脚手架返回 `InvalidSql`，真实目标断言失败；`06-06-RED-task1-junit.json` 返回 `RED_EVIDENCE_OK`。扩展授权矩阵的 `l2_execute_rejections_are_audited_without_post` 也在缺少 `sql:raw` 的预定 `Denied` 断言失败，`06-06-RED-task1b-junit.json` 返回 `RED_EVIDENCE_OK`。
- Task 2：`l2_explain_requires_attestation_and_submits_only_validated_display` 在同一 ACL 未验证断言处失败，`06-06-RED-task2-junit.json` 返回 `RED_EVIDENCE_OK`。RED 均为目标断言失败，非编译或 fixture 错误；之后 `l2_execute` 12/12 通过。
- 无独立 REFACTOR 提交。RED 脚手架用于让测试编译，并非可用实现；GREEN 后补的字节/超时边界测试也已通过。

## Verification

- `cargo test -p ontoforge-query --test l2_execute --locked -- --nocapture`：12/12 通过，覆盖零 POST、Display body、审计元数据与状态、源错、故障、取消、截断、字节和超时。
- `cargo test -p ontoforge-query -p ontoforge-storage --locked -j 2 -- --test-threads=1`：退出 0，全部非 ignored query/storage 与文档测试通过。外部 Trino/PG 环境测试仍按原配置 ignored；本 PLAN 的 ACL HTTP 对抗测试使用 `wiremock`。
- `cargo test -p ontoforge-storage --test sql_exit_guard --locked`：1/1；`cargo clippy -p ontoforge-query -p ontoforge-storage --all-targets --locked -- -D warnings`、`cargo fmt --all -- --check`、`git diff --check`：通过。
- PostgreSQL 17.11 容器的独立临时 schema 执行 `0005` 后插入 `old-id:success`、执行 `0006` 后插入 `new-id:truncated`，两行同时存在；临时 schema 已清理。SQLite 旧表同样由自动测试证明数据与 seq 保留。

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Critical] 审计状态 CHECK 不允许 truncated**
- **Found during:** Task 1 上限行为测试。
- **Issue:** 旧系统表的 CHECK 仅允许四种状态，无法持久化本 PLAN 约定的 `truncated`。
- **Fix:** 添加 SQLite/PG `0006` 迁移、状态枚举和旧库升级验证；不改原迁移。
- **Files modified:** `crates/ontoforge-storage/src/read_audit.rs`、两份 `0006_audit_truncated.sql`、`crates/ontoforge-query/tests/l2_execute.rs`。
- **Commits:** `9cd8ff0`、`b0237ff`。

**2. [Rule 1 - Bug] worker 的首次 tick 提前拆开审计批次**
- **Found during:** 完整 storage 回归的 `audit_batch_rejection_is_atomic`，单独复现。
- **Issue:** `tokio::interval` 首次 tick 即刻 ready，可能在两条连续记录入队之间提前 flush 第一条，使批量拒绝的原子断言失败。
- **Fix:** 在 worker 主循环前消费首次 tick，首轮等待实际 `flush_interval`；目标用例及最终完整回归通过。
- **File modified:** `crates/ontoforge-storage/src/read_audit.rs`。
- **Commit:** `30942cf`。

**3. [Rule 3 - Blocking] 审计固定 SQL 未集中到 sql_exit**
- **Found during:** 完整 storage 回归的 `sql_exit_guard`，报告 `read_audit.rs` 内 4 处固定 SQL 文本。
- **Fix:** 将 4 条固定 SQL 文本迁入既有 `sql_exit.rs` 常量，原绑定和执行逻辑未变；guard 1/1、完整回归均通过。
- **Files modified:** `crates/ontoforge-storage/src/read_audit.rs`、`crates/ontoforge-storage/src/sql_exit.rs`。
- **Commit:** `16d5873`。

## Known Stubs

None. 本 PLAN 不提供 Phase 7 的 REST/MCP 外壳；`L2-01..04`、`AUDIT-01` 的全需求状态保持 Pending，直到元数据工具、CI 真实环境与 Phase 6 总验证完成。

## Next Phase Readiness

`06-07` 可复用 `L2Service` 的固定授权/审计管线接入元数据操作；`06-08` 需在受限 Trino 483 与 PG 环境重跑 CI 门禁。原有未跟踪 Phase 2 诊断和 `docs/` 文件未碰。

## Self-Check: PASSED

`l2.rs`、`l2_execute.rs`、两份增量迁移和三份 RED 证据存在；八个计划提交均是 `HEAD` 的祖先。ledger 基点 `0ad7f5e..16d5873` 实测提交数 8，和 frontmatter 一致。
