---
phase: 06-l2-raw-dql-toolsets-audit
verified: 2026-10-28T12:08:10Z
status: passed
score: 22/22 must-haves verified
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 21/22
  gaps_closed:
    - "H1: PostgreSQL parity 与故障路径原始证据已核验（truth #8/#9）。"
    - "H2: Trino 483 ACL/活跃性/清理/Display 原始证据已核验（truth #10）。"
    - "H3: HEAD 20fd0005 的远端 CI run 40675450974 已通过（truth #17/#18）。"
    - "R1/CR-01: 06-11 保留共享 worker handle 并缓存终态；4 类真实行为 RED/GREEN 和新 HEAD CI 已独立核验（truth #22）。"
  gaps_remaining: []
  regressions: []
delivery_head: "2c9eee6461a412ead8f998c7fa038acaa458b2f9"
implementation_head: "7813e7fb591df1bd2eb607b1a31511492a62c9e3"
ci_run: 40682870456
ci_head_sha: "2c9eee6461a412ead8f998c7fa038acaa458b2f9"
covered_files: [".github/workflows/ci.yml",".planning/phases/06-l2-raw-dql-toolsets-audit/.evidence/.gitattributes",".planning/phases/06-l2-raw-dql-toolsets-audit/.evidence/clippy-workspace-20261028.log",".planning/phases/06-l2-raw-dql-toolsets-audit/.evidence/external-cleanup-20261028.log",".planning/phases/06-l2-raw-dql-toolsets-audit/.evidence/external-pg-audit-20261028.log",".planning/phases/06-l2-raw-dql-toolsets-audit/.evidence/external-trino-acl-20261028.log",".planning/phases/06-l2-raw-dql-toolsets-audit/.evidence/trino-fixture-start-20261028.log",".planning/phases/06-l2-raw-dql-toolsets-audit/06-01-PLAN.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-01-SUMMARY.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-02-PLAN.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-02-SUMMARY.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-03-PLAN.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-03-SUMMARY.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-04-PLAN.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-04-SUMMARY.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-05-PLAN.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-05-SUMMARY.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-06-PLAN.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-06-SUMMARY.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-07-PLAN.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-07-SUMMARY.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-08-PLAN.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-08-SUMMARY.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-09-PLAN.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-09-SUMMARY.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-10-PLAN.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-10-SUMMARY.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-11-CLIPPY.log",".planning/phases/06-l2-raw-dql-toolsets-audit/06-11-DIFF-CHECK.log",".planning/phases/06-l2-raw-dql-toolsets-audit/06-11-FIXTURE-CLEANUP.log",".planning/phases/06-l2-raw-dql-toolsets-audit/06-11-FMT.log",".planning/phases/06-l2-raw-dql-toolsets-audit/06-11-GREEN-join-error.log",".planning/phases/06-l2-raw-dql-toolsets-audit/06-11-GREEN-pg.log",".planning/phases/06-l2-raw-dql-toolsets-audit/06-11-GREEN-sqlite.log",".planning/phases/06-l2-raw-dql-toolsets-audit/06-11-PLAN.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-11-RED-cancelled-junit.json",".planning/phases/06-l2-raw-dql-toolsets-audit/06-11-RED-concurrent-junit.json",".planning/phases/06-l2-raw-dql-toolsets-audit/06-11-RED-failure-junit.json",".planning/phases/06-l2-raw-dql-toolsets-audit/06-11-RED-join-error-junit.json",".planning/phases/06-l2-raw-dql-toolsets-audit/06-11-RED-join-error.log",".planning/phases/06-l2-raw-dql-toolsets-audit/06-11-RED-join-error.xml",".planning/phases/06-l2-raw-dql-toolsets-audit/06-11-RED-pg.log",".planning/phases/06-l2-raw-dql-toolsets-audit/06-11-RED-sqlite.log",".planning/phases/06-l2-raw-dql-toolsets-audit/06-11-RED-sqlite.xml",".planning/phases/06-l2-raw-dql-toolsets-audit/06-11-SUMMARY.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-EXTERNAL-EVIDENCE.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-PR-REVIEW.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-VALIDATION.md","Cargo.lock","ci/query-services.sh","ci/trino/l2/access-control.properties","ci/trino/l2/config.properties","ci/trino/l2/rules.json","ci/trino/l2/seed.sql","crates/ontoforge-action/tests/entrypoints.rs","crates/ontoforge-engine/Cargo.toml","crates/ontoforge-engine/src/config.rs","crates/ontoforge-engine/src/printer/l2.rs","crates/ontoforge-engine/src/printer/mod.rs","crates/ontoforge-engine/src/trino/decode.rs","crates/ontoforge-engine/src/trino/headers.rs","crates/ontoforge-engine/src/trino/l2_health.rs","crates/ontoforge-engine/src/trino/l2_monitor.rs","crates/ontoforge-engine/src/trino/mod.rs","crates/ontoforge-engine/tests/data/l2_corpus.json","crates/ontoforge-engine/tests/l2_gate.rs","crates/ontoforge-engine/tests/l2_health.rs","crates/ontoforge-engine/tests/l2_monitor.rs","crates/ontoforge-engine/tests/trino_l2_acl_it.rs","crates/ontoforge-query/src/aggregate.rs","crates/ontoforge-query/src/execute.rs","crates/ontoforge-query/src/l2.rs","crates/ontoforge-query/src/lib.rs","crates/ontoforge-query/tests/audit_l0.rs","crates/ontoforge-query/tests/audit_pk.rs","crates/ontoforge-query/tests/definition_snapshot.rs","crates/ontoforge-query/tests/l2_discovery.rs","crates/ontoforge-query/tests/l2_execute.rs","crates/ontoforge-query/tests/security.rs","crates/ontoforge-storage/migrations/postgres/0005_read_audit.sql","crates/ontoforge-storage/migrations/postgres/0006_audit_truncated.sql","crates/ontoforge-storage/migrations/sqlite/0005_read_audit.sql","crates/ontoforge-storage/migrations/sqlite/0006_audit_truncated.sql","crates/ontoforge-storage/src/lib.rs","crates/ontoforge-storage/src/read_audit.rs","crates/ontoforge-storage/src/reload.rs","crates/ontoforge-storage/src/scope_store.rs","crates/ontoforge-storage/src/sql_exit.rs","crates/ontoforge-storage/src/store.rs","crates/ontoforge-storage/tests/conformance/mod.rs","crates/ontoforge-storage/tests/conformance_pg.rs","crates/ontoforge-storage/tests/conformance_sqlite.rs","crates/ontoforge-storage/tests/reload_pg.rs","crates/ontoforge-storage/tests/scope_store.rs","crates/ontoforge/src/lint_cmd.rs","crates/ontoforge/src/reload.rs","crates/ontoforge/src/scope.rs","crates/ontoforge/tests/cli_reload.rs","crates/ontoforge/tests/cli_scope.rs"]
covered_digest: "v3:sha256:0d99b15cfcb570bfbf1a832323aa7f3aee81bb4060873ffd4bb34875df990fe3"

---

# Phase 6: L2 Raw DQL, Toolsets & Audit Verification Report

**Phase Goal:** As a 受信主体, I want to 在 allow-list 内执行仅在 Trino 侧权限可证明安全时才开放的只读 Trino SQL, so that 所有级别的读取都留下元数据审计且定义与授权的变更只经部署方与管理通道.

**Verified:** 2026-10-28T12:08:10Z

**Status:** passed

**Re-verification:** Yes。上一轮 `21/22`、`gaps_found` 的唯一缺口 R1/CR-01 已由 06-11 关闭。本轮亲读两个增量源码文件、RED/GREEN 原始日志与四份 classifier，独立执行四次 RED classifier 和 GSD TDD gate，并等待新 CI 完成后读取三个必需 job 的关键原始日志；原 21 条历史证据保留，新增 #22 现有行为证据，最终 `22/22`、`passed`。未运行 Cargo、未修改源码、未重做前十个 PLAN 的全量核验。无 overrides。

**交付快照:** 本轮核验 HEAD 与 PR #7 的 `headRefOid` 均为 `2c9eee6461a412ead8f998c7fa038acaa458b2f9`，新 CI run `40682870456` 的 `headSha` 精确相同。RED 为 `0bb29a3652d217d4e638ff572fefffea651c43bf`，实现 GREEN 为 `7813e7fb591df1bd2eb607b1a31511492a62c9e3`；已执行 `git diff 7813e7f HEAD -- crates ci .github Cargo.toml Cargo.lock`，无输出，之后提交只改 SUMMARY。相对旧 `20fd0005`，增量仅两个 storage 源码/测试文件（142 additions、6 deletions），未更改 CI、fixture、依赖或其他生产模块。新 PR CI 实际 checkout 的合成 merge SHA 为 `5ff30dafc2bb39995d9b639a45bcf75c6b631bea`，不与分支 HEAD 混淆。

**历史证据边界:** 原始 #1-21 的源码行号、W 日志及 H1/H2/H3 章节描述各自核验快照；本轮没有重跑其中的本地测试。旧 run `40675450974` 仅证明 `20fd0005`，不能证明 06-11 修复。当前交付的远端证据以新 run `40682870456` 为准，详最终增量章节。后续仅收尾文档的提交不改变本报告记录的实现与 CI 快照。

## User Flow Coverage

中央 `user-story.validate` 对当前 ROADMAP Goal 返回 `true`。本阶段入口是库接口与已有管理 CLI；REST/MCP 外壳由 Phase 7 交付。

| Step | Expected | Evidence | Status |
| --- | --- | --- | --- |
| 部署方配置隔离身份和有界只读探针并启动库级 monitor | 未证明安全保持禁用，完整证明并清理后才开放 | `engine/src/trino/l2_monitor.rs:91,249,290`；本轮 startup/恢复/跨代清理相关命名证据见下文 | VERIFIED（库级与真实Trino，H2已闭合） |
| 受信主体列举被授予的 catalog/schema/table | 仅返回 allow-list 内名称和类型，排除 system/edit | `query/src/l2.rs:99,133,231,245,269,316`；`l2_discovery` 原始 GREEN | VERIFIED |
| 主体提交只读 SQL 或解释请求 | `sql:raw`、grants、ACL、AST 均通过才发送 Display SQL | `query/src/l2.rs:374,414,433,446`；104-ID 生产 gate 本轮 1/1 GREEN | VERIFIED |
| 提交越界、变更或不支持的 SQL | 拒绝且零 Trino POST | `printer/l2.rs:87,283`；`l2_execute_rejections_are_audited_without_post` 原始 GREEN | VERIFIED |
| 各读取入口返回数据或错误 | 先占用有界审计容量，记录元数据；积压和 sink 故障不释放结果 | `query/lib.rs:242`、`execute.rs:508`、`aggregate.rs:878`、`l2.rs:99,374`；SQLite/查询原始 GREEN | VERIFIED（SQLite与真实PG，H1已闭合） |
| 管理者更新 scope 或 reload 定义 | lint/validate、hash、定义锁和事务审计生效；在途请求保持旧快照 | `ontoforge/src/reload.rs:37`、`scope_store.rs:453,501`；本轮 L2 竞争快照 1/1 GREEN | VERIFIED |
| Outcome：安全读取有审计，定义和授权只经既定管理通道 | 已接受审计在 shutdown 完成前持久化，所有调用者一致观察失败 | 原 21 条证据保留；06-11 的共享 worker 终态与四类行为测试证明 #22，新 HEAD 三个必需 CI job 均成功 | VERIFIED |

真实 PostgreSQL、Trino 483 与本轮交付 HEAD 对应的远端 CI 证据均已补齐，R1 的库级 `ReadAudit::shutdown` 缺口已由 06-11 行为测试及修复关闭。Phase 7 服务持有 monitor 生命周期、HTTP `/health` 和 MCP 工具过滤仍按 ADR-0028 单独验收；L2-04/SCOPE-07 整体 Pending，Phase 7 未启动。

## Goal Achievement

### Observable Truths

表中相对路径 `engine/`、`query/`、`storage/` 分别指 `crates/ontoforge-engine/`、`crates/ontoforge-query/`、`crates/ontoforge-storage/`。`W` 指下方已亲读的最终 workspace 原始日志；不能把 W 中的 ignored 视为通过。

| # | Truth | Status | Evidence |
| --- | --- | --- | --- |
| 1 | ROADMAP SC1：L2 执行器只接受 SELECT / WITH，拒绝 `TABLE()` 表函数、`CALL`、`EXECUTE`、多语句与会话 / 事务语句，执行时发送重序列化后的 SQL；spike 2 语料全部按预期放行或拒绝 | VERIFIED | `engine/src/printer/l2.rs:87,157`、私有 `PrintedSql` 构造；本轮 `l2_corpus_each_case_runs_production_gate` GREEN。104 个 ID 均为 constructed-equivalent，恢复的历史原始 SQL 为 0；不宣称复原原文。合并 06-03 两条细化 truth。 |
| 2 | ROADMAP SC2：allow-list 外 catalog、edit store catalog 与 `system` 元数据表全部被拒绝 | VERIFIED | `printer/l2.rs:283` relation visitor；`query/l2.rs:133` metadata grant gate；W 的 unauthorized/system/hidden-relation 测试 GREEN。 |
| 3 | ROADMAP SC3：库级启动及周期自检无法证明 L2 身份受限（含只能看到自己的 `system.runtime.queries`）时 L2 被禁用，暴露可供服务读取的禁用原因；HTTP `/health` 告警由 Phase 7 验收（ADR-0028） | VERIFIED | `l2_health.rs:73,95,243` 默认禁用、TTL、结构化原因；`l2_monitor.rs:91,249` 启动 await 与周期；本轮失败/恢复命名测试 GREEN，W 含缺失配置和超时 GREEN。合并 06-04 默认 fail-closed 与 06-10 startup truth。真实 Trino 协议另列 #10。 |
| 4 | ROADMAP SC4：所有级别的读取写入元数据审计（主体、工具、对象类型、apiName 级条件、生效 role、SQL 哈希、行数、耗时、request_id），不含结果数据；审计积压时读请求返回 `AuditUnavailable` | VERIFIED | 四个公开边界 reserve/finish；`read_audit.rs:197,255,267,309`；W 中 pk、list、aggregate、execute/explain、四 discovery 的成功/拒绝/错误/取消/容量故障 GREEN。真实 PG backend parity 另列 #8/#9。 |
| 5 | ROADMAP SC5：role / scope 只能经管理通道或 L3 修改并审计；定义 reload 先 validate / lint，失败不切换；每个请求固定使用开始时的定义版本 | VERIFIED | `ontoforge/src/reload.rs:37` 在打开 store 前 lint；`scope_store.rs:453,501,544` hash/锁/审计同事务；`definition_snapshot.rs:107,328` 固定竞争点与新旧请求断言，L2 本轮 GREEN。合并 06-09 快照 truth。MCP 工具限制保留 Phase 7 边界。 |
| 6 | 06-01：公开 pk 读的成功、授权拒绝、源错误及运行中取消均有明确审计状态；返回前占用有界审计容量并提交无结果数据的元数据记录，无容量或 sink 失败返回 `AuditUnavailable` | VERIFIED | `query/src/lib.rs:242` 先 reserve、执行后 finish；W `audit_pk_success_records_one_metadata_row`、`audit_pk_denied_before_source_and_source_failure_have_distinct_statuses`、`audit_pk_cancelled_in_flight_records_without_result`、`audit_pk_full_queue_rejects_before_releasing_another_result` GREEN。 |
| 7 | 06-01：记录使用请求开始的 definition/scope 快照；内部 read_pk_source 不产生第二条读审计 | VERIFIED | 公共边界与 source 入口分离，metadata 取同一 `active/scope`；`definition_snapshot` 两条 W GREEN；本轮 L2 snapshot GREEN，pk 成功测试精确一行。 |
| 8 | 06-02：PostgreSQL system schema 与 SQLite 的审计字段、batch、保留期清理行为相同 | VERIFIED | `read_audit.rs:379,410` 与共用conformance已在初验核查；本次亲读 `.evidence/external-pg-audit-20261028.log`，PostgreSQL17.11真实运行 `audit_batch_retention_parity`、`audit_shutdown_drains_inflight_guard` 均GREEN，target共6/6、1.62s、exit0。H1闭合。 |
| 9 | 06-02：两个后端的 sink 失败都关闭新读且没有原始 SQL 或结果数据入表 | VERIFIED | SQLite故障初验GREEN；新增PG原始日志中的inflight finish、batch拒绝、cleanup回滚、连接中断恢复四条均GREEN；表字段与参数化写入源码不变。6/6、exit0，故障注入stderr未被删去。H1闭合。 |
| 10 | 06-04：另一主体已知活跃查询对 L2 不可见，越界/edit 对象均实际存在且错误是权限拒绝 | VERIFIED | 本次亲读 `.evidence/external-trino-acl-20261028.log`：受限/宽松实例均483且ready，六条集成测试默认并行6/6、9.91s、exit0，32条allow Display均FINISHED；与初验已核查的真实对象/精确拒绝/RUNNING前后观测及cleanup断言对应。H2闭合。 |
| 11 | 06-05：execute_list 的 filter/list/set/searchAround 和 execute_aggregate 都走同一审计门禁，内部调用不重复记录 | VERIFIED | `execute.rs:508`、`aggregate.rs:878`，聚合调用 `execute_list_internal`；W `audit_list_type_filter_set_and_search_around_record_once`、`audit_aggregate_partial_and_fallback_each_record_once` GREEN。 |
| 12 | 06-05：`AuditUnavailable` 在 `AggregateError` 转换后保持相同错误码；已计算但未受理审计的结果不释放 | VERIFIED | `aggregate.rs:137,878` 错误保留和 finish 顺序；W `audit_aggregate_unavailable_keeps_error_identity_and_no_result`、`audit_aggregate_sink_failure_closes_gate_without_result` GREEN。 |
| 13 | 06-06：execute_query 仅在 sql:raw、grants、ACL 自检均有效时调用受控 Display 入口；越界零 Trino POST | VERIFIED | `query/l2.rs:414,433,446` gate 及 `TrinoEngine::execute` 再检查；W `l2_execute_only_submits_display_sql_after_attestation_and_audits_result`、`l2_execute_rejections_are_audited_without_post` GREEN。 |
| 14 | 06-06：execute_query/explain_query 的成功、授权拒绝、源错误、截断与运行中取消均有明确审计状态；返回行数/字节限制而不伪造任意 SQL 的 keyset cursor | VERIFIED | `query/l2.rs:374` reserve/状态/finish，`:446` 有界 engine 输出，`L2Result` 没有 cursor；W 中 `l2_execute_*`/`l2_explain_*` 12 项 GREEN。 |
| 15 | 06-07：list_catalogs/list_schemas/list_tables/get_table_schema 只返回授权允许且非 system/edit 的名称与类型 | VERIFIED | `query/l2.rs:99,133,231,245,269,316` 分层授权、过滤、字段预算；W `l2_catalog_filters_and_audits_catalogs_and_schemas`、`l2_table_lists_only_granted_tables_and_describes_columns`、拒绝零 POST GREEN。 |
| 16 | 06-07：四项操作逐项执行前预留审计容量，成功、拒绝、Trino 错误、截断、取消均留下相应元数据；故障无结果释放 | VERIFIED | `query/l2.rs:99` 统一 reserve/finish；W `l2_discovery` 9 项 GREEN，含两类取消、schema字段截断、源错误及 audit failure。 |
| 17 | 06-08：CI 实际启动独立受限 Trino 483，并以测试发现数大于零的门禁跑 ACL、104-ID corpus 的 Trino 行为 | VERIFIED | 新 run `40682870456` 对应 `2c9eee6...`；integration job `122797742148` 发现 6 项 ACL 测试、6/6 通过（8.05s），精确 32 条 `[GB]ID: FINISHED`；Ubuntu/Windows corpus 各发现 6 项、6/6 通过（0.03s/0.07s）。旧 H3 证据保留但不替代此新 run。 |
| 18 | 06-08：CI 实际运行 PostgreSQL 审计 conformance，发现数大于零；Windows workspace 回归通过 | VERIFIED | 新 run integration：`postgres-audit: 9 ignored tests discovered`，9/9 通过（1.70s），含三条新增共享 shutdown 回归。Windows `122797742336` 与 Ubuntu `122797741785` 均 success；workspace 原始结果分别 1255/1258 passed、0 failed、106 ignored，fmt/Clippy/embedded-pg Clippy/corpus 均 success。 |
| 19 | 06-09：reload lint/validate 失败不切换，scope hash 错误不写，成功管理变更事务性写 of_admin_audit；远程 L2 工具无管理入口 | VERIFIED | `reload.rs:37` 与 `lint_cmd.rs:18` 前置链；`scope_store.rs:501,544` hash 先于 DML，审计随事务；W `invalid_definitions_never_open_store`、`stale_plan_hash_rejected`、`audit_insert_failure_rolls_back_scope_and_indexes` GREEN。L2 public surface 仅六个只读方法，实际 MCP 列表归 Phase 7。 |
| 20 | 06-10：Every cycle owns a fresh bounded ontology query, proves RUNNING across visibility observations, and cleans up before enabling. | VERIFIED | `l2_monitor.rs:249,290,319` 每轮自有 query、新ID、25s server limits、同origin；`:203` DELETE 成功才 complete。本轮跨代清理 GREEN；W `monitor_startup_owns_fresh_probe_limits_and_cleanup`、`monitor_cannot_enable_before_delete_response_succeeds` GREEN。 |
| 21 | 06-10：Periodic failure, expiry, shutdown and cancellation fail closed; recovery requires a fresh complete attestation. | VERIFIED | `l2_health.rs:95,114` TTL/epoch/generation；`l2_monitor.rs:40,203,249` shutdown/cleanup；本轮 periodic recovery GREEN；W 含 expiry unit、取消、Drop、shutdown竞争与deadline GREEN。 |
| 22 | Regression R1 / 06-11：`ReadAudit::shutdown` 的每个调用者，包括并发调用及取消后重试，都等待已接受审计持久化完成，并一致观察最终成功、sink 失败及 worker JoinError | VERIFIED | `storage/src/read_audit.rs:161,232-242` 的 `Running/Finished` 在 async mutex 内借用 handle 等待，取消保留共享所有权，结果无后续 await 地缓存。`conformance/mod.rs:2685,2713,2818` 三条共享回归显式 poll Pending、完成后查询持久行并重复调用；`:2909-2911` 接入双后端，`read_audit.rs:455` 实际 abort worker 验证 JoinError。四类行为均有真实 RED 后 GREEN；本地 SQLite 8/8、PG 9/9、unit 1/1，新 CI 三条共享测试与 unit 实际执行通过。 |

**Score:** 22/22 truths verified，0 条 behavior-unverified，0 条 FAILED truth，0 个未解决 BLOCKER；无 override 或待人工验收项。

合并规则：保留全部五条 ROADMAP SC；06-03 两条细化并入 #1，06-04 默认 fail-closed 和 06-10 startup 并入 #3，06-09 请求快照并入 #5；其余 16 条原 PLAN truth 保留。06-11 的三条 truth（共同等待、取消保留 worker、缓存成功/失败含 JoinError）均归并到既定 #22，不缩减合同。没有发现 `prohibitions`、`verification: backstop` 或 `<human-check>`。

### Required Artifacts

原十个 PLAN 的 artifact query 共检查 20 项（含重复），历史结果均 exit 0、`all_passed:true`。本轮仅对 06-11 独立 query，2/2、exit 0，再亲读实质实现与接线。下表保留原 artifact 索引并更新受影响两项；未因文件存在直接判定行为成立。

| Artifact | Expected | Status | Details |
| --- | --- | --- | --- |
| `storage/src/read_audit.rs` | 有界异步元数据审计及可靠 shutdown | VERIFIED | 原审计链保留；`:161,232-242` 的共享 Running/Finished 状态修复取消与错误传播，四类行为有 RED/GREEN 和新 CI 证据。 |
| `storage/migrations/sqlite/0005_read_audit.sql` | SQLite 审计表 | VERIFIED | store migration 接入；0006 增加 truncated 状态，升级测试 W GREEN。 |
| `storage/migrations/postgres/0005_read_audit.sql` | PG system schema 审计表 | VERIFIED | store PG migration 接入；0006 同步状态；真实PG执行已由H1原始证据验证。 |
| `storage/tests/conformance/mod.rs` | 双后端共用合同 | VERIFIED | 原字段/retention/事务故障断言保留；新增三条 shutdown 回归由 `all_cases!` 接入 SQLite/PG，局部 GREEN 8/8、9/9，新 CI 全 target SQLite 43/43、PG 45/45。 |
| `engine/src/printer/l2.rs` | 单受控 AST/Display 入口 | VERIFIED | visitor、关系/函数检查、LIMIT、重解析相等、私有构造。 |
| `engine/tests/data/l2_corpus.json` | 104-ID 机器 corpus | VERIFIED | include_str 接入生产 gate 测试与真实 Trino 测试；明确 constructed-equivalent。 |
| `engine/src/trino/l2_health.rs` | attestation 状态和检查 | VERIFIED | 配置、身份、coordinator、正反探针、TTL、epoch/generation。 |
| `ci/trino/l2/rules.json` | 独立受限权限配置 | VERIFIED | ontology/L2分离、catalog/table/查询owner规则，挂载到独立容器；真实协议已由H2验证。 |
| `ci/query-services.sh` | fixture 启停 | VERIFIED | pin Trino483、独立受限/宽松实例、ready/seed检查、cleanup；CI已调用。 |
| `engine/tests/trino_l2_acl_it.rs` | 真实ACL和Display验收 | VERIFIED | 六条明确 ignored 外部测试；H2本地真实运行已验证；远端CI run40675450974已验证。 |
| `query/tests/audit_l0.rs` | L0公开读取审计回归 | VERIFIED | list/set/searchAround/aggregate、取消和故障断言，W GREEN；PG-only单独ignored。 |
| `query/src/l2.rs` | 六个只读库操作 | VERIFIED | public methods 接到统一审计、授权和 engine 路径。 |
| `query/tests/l2_execute.rs` | execute/explain合同 | VERIFIED | 12条 W GREEN，验证请求体、拒绝零POST、审计与边界。 |
| `query/tests/l2_discovery.rs` | 四discovery合同 | VERIFIED | 9条 W GREEN，覆盖名称过滤、字段预算、取消和故障。 |
| `.github/workflows/ci.yml` | 真实服务CI门禁 | VERIFIED | workflow接通非零发现/实际运行命令；远端运行40675450974已验证。 |
| `query/tests/definition_snapshot.rs` | 在途版本/权限一致 | VERIFIED | 控制阻断点后执行真实reload/scope apply，核对新旧请求及审计，非串行替代。 |
| `engine/tests/l2_monitor.rs` | 生命周期/清理回归 | VERIFIED | registry按ID/owner/state，DELETE改变状态；16条W GREEN，2条初验复跑GREEN。 |
| `engine/src/trino/l2_monitor.rs` | startup/periodic/shutdown/Drop | VERIFIED | monitor API被真实/模拟测试调用；服务持有handle属于Phase7。 |

### Key Link Verification

CLI 的 11 条 key-link query 均 exit 1，detail 为 `Target not referenced in source`。该路径字符串启发式不能解析 Rust module/type 引用或 workflow 的 Cargo target，以下逐条检查实际调用后纠正机械结论；没有以文件存在代替接线。

| From | To | Via | Status | Details |
| --- | --- | --- | --- | --- |
| `query/src/lib.rs` | `storage/src/read_audit.rs` | 公共pk reserve/finish | WIRED | `:242` 经 store.read_audit() reserve，结果返回前finish。 |
| `storage/src/read_audit.rs` | PG `0005_read_audit.sql` | migration及参数化INSERT | WIRED | `store.rs:327` 迁移，`:226` 启动worker；`read_audit.rs:410` PG事务。 |
| `engine/src/printer/l2.rs` | `printer/mod.rs` | 私有PrintedSql构造 | WIRED | `l2.rs:157` 在所有检查后调用，`mod.rs:21,24` 构造边界。 |
| `engine/src/trino/l2_health.rs` | `trino/mod.rs` | 隔离身份探针和admission | WIRED | health实际调用engine；`mod.rs:444` 持读锁并重新检查enabled。 |
| `query/src/aggregate.rs` | `query/src/execute.rs` | 内部list不重复审计 | WIRED | 公共aggregate自己reserve，调用内部list实现；精确一条审计断言W GREEN。 |
| `query/src/l2.rs` | `engine/src/printer/l2.rs` | validate_l2 | WIRED | `:433` 必经受控入口，`:446` 只发PrintedSql。 |
| `query/src/l2.rs` | `storage/src/read_audit.rs` | reserve/finish | WIRED | `:99,374` 两入口族统一元数据guard。 |
| `query/src/l2.rs` | `engine/src/trino/mod.rs` | 元数据操作 | WIRED | discovery调用`list_l2_*`和类型化描述SQL，engine `:63,68,78` 均再走execute admission。 |
| `.github/workflows/ci.yml` | `engine/tests/trino_l2_acl_it.rs` | Cargo --test target | WIRED | `:140` 启动fixture，`:142`之后先list计数再--ignored执行；实际run40675450974已验证。 |
| `query/tests/definition_snapshot.rs` | `query/src/lib.rs` | ActiveDefinition与query边界 | WIRED | 两个竞争测试调用生产service，在源阻断后做scope/reload并核对审计。 |
| `engine/src/trino/l2_monitor.rs` | `engine/src/trino/l2_health.rs` | owner/generation/epoch及complete | WIRED | `run_cycle:249` 串行检查/清理后complete，stop/expiry阻止旧响应重启用。 |
| `storage/src/read_audit.rs::shutdown` | worker 最终持久化结果 | mutex 内借用 Running handle，完成后保存 Finished(Result) | WIRED | 首个 waiter 持锁等待，其他 caller 等待同一状态；取消时 handle 仍归 Running，后续重试可等待；Finished 返回缓存终态。新行为测试与实际持久行证明接线。 |
| `storage/tests/conformance/mod.rs` | `conformance_sqlite.rs` / `conformance_pg.rs` | `all_cases!` 中三条新增回归 | WIRED | `mod.rs:2909-2911`、SQLite `:8`、PG `:12` 实际展开。06-11 key-link query 的 Rust 路径字符串启发式返回 exit 1，`Target not referenced in source`；由此源码接线与两后端实际执行纠正，不把该机械输出误记为缺失。 |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
| --- | --- | --- | --- | --- |
| `query/src/l2.rs` execute/explain | `output.rows` | 受控Display -> Trino POST -> 解码/限额 -> L2Result | 是；`trino/mod.rs:312` body取PrintedSql，未丢弃响应 | FLOWING；真实483复验H2已通过 |
| `query/src/l2.rs` discovery | catalog/schema/table/columns | engine元数据SQL -> Trino返回 -> scope过滤 -> 有界L2Result | 是；没有硬编码空props或静态fallback | FLOWING |
| L0公共query边界 | QueryPage / aggregate | 现有源查询及overlay -> 授权 -> 结果；审计finish在返回之前 | 是；测试结果与审计精确匹配 | FLOWING |
| `storage/src/read_audit.rs` | ReadAuditRecord / shutdown 终态 | 真实 metadata -> 有界队列 -> 参数化 INSERT -> worker 完成 -> 缓存 Result | 是；SQL/result 原文不入表；新增测试在 guard finish 前确认空表，shutdown 完成后查询真实 principal | FLOWING；06-11 双后端与新 CI 已通过 |
| `scope_store.rs` | roles/bindings/admin audit | 管理scope输入 -> hash重算 -> 同事务DML和audit | 是；失败rollback测试核对表与索引 | FLOWING |
| `l2_monitor.rs` | health status | ontology自建query ID -> RUNNING前后观测/隔离 -> DELETE成功 -> complete | 是；没有固定成功返回，generation/epoch可拒绝旧结果 | FLOWING；真实483复验H2已通过 |

### Behavioral Spot-Checks

初验先尝试 `cargo test -p ontoforge-engine --test l2_gate --locked -j 2 l2_corpus_each_case_runs_production_gate -- --exact --nocapture`，触发重新编译。为遵守快速spot-check边界，已发送 Ctrl-C，session `90157` exit 1；没有把它记为测试失败或通过。改用 W 原始日志明确列出的同一 test executable，逐个运行一个命名测试。初验未重复完整workspace，未启动外部服务，未更改生产状态；本次窄复签没有执行这些测试或Cargo。

| Behavior | Command（路径相对仓库根） | Result | Status |
| --- | --- | --- | --- |
| 104-ID真实生产gate分类 | `target/debug/deps/l2_gate-2523ba014cbb8e9d.exe l2_corpus_each_case_runs_production_gate --exact --nocapture` | 1 passed，0 ignored，0.04s，exit0 | PASS（初验亲跑） |
| 周期失败及新证明恢复 | `target/debug/deps/l2_monitor-e94769286aaf1f46.exe monitor_periodic_failure_and_fresh_recovery --exact --nocapture` | 1 passed，0 ignored，0.20s，exit0 | PASS（初验亲跑） |
| 跨代pending清理后才能enabled | 同一monitor executable，`monitor_restart_must_clean_previous_generation_before_enabling --exact --nocapture` | 1 passed，0 ignored，0.08s，exit0 | PASS（初验亲跑） |
| shutdown停止准入仍drain在途guard | `target/debug/deps/conformance_sqlite-9e296005f191c484.exe audit_shutdown_drains_inflight_guard --exact --nocapture` | 1 passed，0 ignored，0.05s，exit0 | PASS（初验亲跑） |
| 在途L2请求与审计固定快照 | `target/debug/deps/definition_snapshot-9198927162269918.exe l2_inflight_reload_and_scope_apply_keep_request_snapshot --exact --nocapture` | 1 passed，0 ignored，0.34s，exit0 | PASS（初验亲跑） |
| 最终Windows workspace | W原始日志；命令由主会话记录为 `cargo test --workspace --locked -j 2 -- --test-threads=1` | 本轮用regex重新统计117个结果组：1251 passed、0 failed、103 ignored；日志包含上述executable路径 | PASS（亲读既有原始输出，非本轮重跑） |
| PostgreSQL audit | `conformance_pg-39e883f11e6a6c0e.exe audit_ --ignored --nocapture` | 新增原始日志：发现6项，PostgreSQL17.11，6 passed、0 failed/ignored、1.62s、exit0 | PASS（本次亲读原始输出及核对来源，非verifier重跑） |
| 真实Trino483 ACL/corpus | `trino_l2_acl_it-0af57445ba4215b1.exe --ignored --nocapture` | 新增原始日志：默认并行6/6、9.91s、exit0；32条FINISHED；独立受限/宽松实例483 | PASS（本次亲读原始输出及核对来源，非verifier重跑） |
| H3 历史 SHA 远端 CI（20fd0005） | `gh run view 40675450974 --json headSha,status,conclusion,jobs`，以及三个相关job的`--log` | headSha=20fd0005...，run success；integration/ubuntu/windows均success，PG与ACL各6/6，两个平台corpus各6/6 | PASS（H3 复签亲读；不证明 06-11） |
| shutdown 并发、取消后重试与 sink 失败传播 | 亲读 `06-11-RED-{sqlite,pg}.log` 与 `06-11-GREEN-{sqlite,pg}.log`，再读新 run integration/workspace 原始日志 | 三条共享回归双后端均先 RED 后 GREEN；本地 audit SQLite 8/8、PG 9/9，新 CI SQLite 全 target 43/43、PG 全 target 45/45，专门 PG audit 9/9 | PASS（原始行为证据，verifier 未运行 Cargo） |
| worker JoinError 重复调用保留 | 亲读 `06-11-RED-join-error.log` / `06-11-GREEN-join-error.log` 及两平台 workspace 原始日志 | 原实现第二次 shutdown 实际 Ok 导致 RED；修复后 1/1 GREEN，两平台 storage unit 49/49，命名 JoinError 测试均 ok | PASS |

W = `C:/Users/easyi/AppData/Local/Temp/ontoforge-phase06-10-workspace-accepted.log`。该日志持有最终实现快照的实际libtest输出，非SUMMARY转述；其编译/链接行显示29m55s。本轮读取其命名结果和总计，并确认当前实现/CI未变。初验W中的外部服务ignored不计分；本次新增真实运行原始输出后，#8/#9/#10才升级为VERIFIED。

### Probe Execution

| Probe | Command | Result | Status |
| --- | --- | --- | --- |
| 独立`probe-*.sh` | 扫描全部PLAN/SUMMARY及常规scripts入口 | 仓库无scripts目录，也无声明的probe脚本；部署方`active_query_sql`是运行时ACL探针，不是shell probe | N/A |
| ACL运行时探针 | 生产monitor测试及真实集成target | 库级命名行为PASS；真实服务H2已通过 | 见上表 |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
| --- | --- | --- | --- | --- |
| L2-01 | 06-06/07/08/09 | raw DQL与allow-list discovery | SATISFIED（库级与远端CI） | #13-18，六操作有源数据/授权/审计接线。 |
| L2-02 | 06-03/06/08 | AST白名单与Display | SATISFIED（gate与真实483）；远端CI已验证 | #1，104-ID生产gate亲跑；明确等价新语料而非原文。 |
| L2-03 | 06-03/04/06/07/08/10 | catalog/schema allow-list，硬排除system/edit | SATISFIED（库级与真实ACL）；远端CI已验证 | #2/#10/#15。 |
| L2-04 | 06-04/06/07/08/10 | 启动/周期fail-closed、健康原因 | 库级SATISFIED；整体Pending | #3/#20/#21；H2/H3均已验证。服务持有handle、优雅关闭及HTTP `/health` 按ADR-0028由Phase7验收。 |
| AUDIT-01 | 06-01/02/05/06/07/08/09/11 | 全读取元数据审计、背压、保留期及可靠排空 | SATISFIED | #4/#6-9/#11-18 的原证据保留；#22 的并发/取消/成功及失败终态合同已由真实 RED/GREEN 与新 HEAD CI 证明。 |
| SCOPE-07 | 06-09 | 管理变更审计与请求固定版本 | 当前阶段SATISFIED；整体Pending | #5/#19及竞争测试；MCP无变更工具限制属Phase7。 |

11 个 PLAN 的 requirement 并集与 Phase 6 原映射一致，无 ORPHANED requirement。本轮仅写本报告；主 session 负责根据本次验收更新 AUDIT-01 的中央状态。L2-01/02/03 保留已验收，L2-04/SCOPE-07 整体仍为 Pending，Phase 7 未启动。

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
| --- | --- | --- | --- | --- |
| 阶段修改源码/测试及fixture | 扫描范围见fingerprint及PLAN files_modified | 未发现无正式引用的TBD/FIXME/XXX债务注释；无待实现handler或静态结果替代 | 无BLOCKER | `mktemp`模板中的`XXXXXXXX`不是债务注释；match空分支是已校验成功路径，不是空实现。 |
| `query/tests/audit_l0.rs` | 484 | PG-only显式ignore | INFO | 初验workspace中仍为ignored，未把该单项算为通过；H1所需六条审计conformance现有真实PG运行证据。 |
| `engine/tests/trino_l2_acl_it.rs`、`storage/tests/conformance_pg.rs` | 外部targets | 普通workspace显式ignore | INFO | 本次额外原始日志证明--ignored真实执行，H1/H2闭合；H3远端运行现已验证。 |
| `engine/tests/l2_monitor.rs` | registry及DELETE分支 | 已核查历史“假RUNNING”风险修复 | INFO | query registry按ID/owner/state判断、成功DELETE移除，未发现用任意ID回显假活跃证明。 |
| `storage/src/read_audit.rs` | 旧快照 227-233；修复后 232-242 | 历史 R1/CR-01 提前 take / None 成功 | INFO（已关闭） | `7813e7f` 保留共享 handle 并缓存结果；四类行为先 RED 后 GREEN，新交付 CI 再确认，见 #22。 |
| `ontoforge-phase06-10-clippy-final-fixed.log`（Temp） | 文件末尾 | `items_after_test_module`历史失败 | INFO（历史记录） | 旧log仍为失败，未被覆盖或改写。本次另亲读`.evidence/clippy-workspace-20261028.log`：workspace/all-targets/locked/-j2/-D warnings，Finished dev profile、exit0、9.84s；当前实现与最终快照无diff，最终Clippy GREEN现有直接证据。 |

原始核心 L2/query 测试的 ignore 与明显自比较断言扫描结论保留。本轮只扫描两个增量文件，无无引用 TODO/FIXME/TBD/XXX/HACK/PLACEHOLDER，`git diff --check 20fd0005 HEAD -- <两文件>` exit 0。新增测试实际 poll Pending、实际 finish guard 并读取持久行，非 `assert!(true)`、自比较或 fixture-only 的预先成功状态。没有为只读验证新增测试或修改实现。

### Decisions And Quality Gates

`check.decision-coverage-verify` 返回 `total:6, honored:6, blocking:false`；本轮同时从实际源码核对D-01至D-06的授权边界、独立身份、PrintedSql、审计、管理通道及constructed-equivalent corpus。该机械结果不单独证明运行行为；H1-H3已分别以本地原始日志和远端CI原始日志补证。

原始审查索引保留。本轮亲读更新后的 `06-PR-REVIEW.md`：45 文件、六个适用 specialist 与最终 Red Team，本地 gate pass、0 open finding、CR-01 closed。该结论未代替本 verifier 对修复与行为证据的独立核验。外部 Codex lane 的 `401 Unauthorized` 按正式 review 的 Error handling 记录为非阻塞 `unavailable`，不能宣称外部或跨模型通过；本轮未重试。PR #7 经 `gh pr view 7 --json headRefOid,headRefName,isDraft,state` 为 OPEN、draft，head 精确匹配交付快照；不合并 PR，也不授权发布。

`ralph-loop` 本机不可用，未声称执行。GSD verifier 的 agent_skills 配置为空；已读取 `gsd-verify-work` 及必须引用的验证规则。没有增加 subagent。本轮仅写本报告，未修改共享状态、进度、代码或用户未跟踪文件，未运行 Cargo、启动服务或提交。

### Incremental Evidence Review（上一轮 H1/H2 快照）

本次仅亲读新增证据并核对来源，没有运行Cargo、重新构建fixture或重跑完整验证。读取 `06-EXTERNAL-EVIDENCE.md` 作为证据索引，再逐份读取下表原始日志；所有SHA256均由本次重新计算并与索引一致。还逐份验证 `git hash-object <log>` 等于 `git rev-parse :<log>`，五份日志index与disk字节一致；目录`.gitattributes`保留CRLF，`git ls-files --eol`显示i/crlf、w/crlf、attr/-text。

| Raw Log（相对phase目录） | SHA256 | 本次直接核验 |
| --- | --- | --- |
| `.evidence/external-pg-audit-20261028.log` | `7B5988A761068E98F60A35A66152DF85BF6A51BA7DE0ED59B2F649BFE4189279` | PostgreSQL17.11、6项发现、六个命名audit测试均ok、1.62s、test_exit_code=0。 |
| `.evidence/external-trino-acl-20261028.log` | `36A81A389A1262D2933DEF476E068369D9679C5FE535371314F3F99DEB2D9259` | 两实例483/ready、默认并行6/6、32条FINISHED、9.91s、test_exit_code=0。 |
| `.evidence/clippy-workspace-20261028.log` | `89C635C6C657D6E38E9C81C45F348B4CB1C5B9E56DCF1A9FF7ED048FB012E0C5` | 精确workspace/all-targets/locked/-j2/-D warnings命令、Cargo9.30s、计时9.8378895s、clippy_exit_code=0。 |
| `.evidence/trino-fixture-start-20261028.log` | `9CAD2F3D229B944D22225B22ACA86E87A1A6B9BAE7278B2014CAACDAEF608AAA` | 两个独立容器ID、seed输出、fixture_exit_code=0；ID与/v1/info及cleanup记录一致。 |
| `.evidence/external-cleanup-20261028.log` | `8181213775AE4B4AB4C62A773747B279B50D22FB9418A1FBC1E06AC081883636` | 本轮拥有的两容器已删除，原Trino/PG/Casdoor服务保留。 |

二进制来源经过交叉核验，不以mtime单独证明：H1/H2 补证复签当时重新计算 `conformance_pg-39e883f11e6a6c0e.exe` 的SHA256为 `453A0ACC8A4C423F7ED526D532758664BDA9A0B9C721F33E4FB93A773DF7D421`，`trino_l2_acl_it-0af57445ba4215b1.exe` 为 `C4DEC5B77F8188264AB76734436B0B90650F2B1881283587622119E06EC14DAF`，与外部原始日志一致。亲读最终accepted workspace日志对应的Running行，列出相同两个executable；该workspace日志当时重新计算SHA256为 `B53F76BD2FC9E1C2ED40B489CBBCACB35E0000E9218176F0DD15F4F869F0E4D8`，与证据索引一致。上一轮 H1/H2 补证时执行的 `git diff e81127f HEAD -- crates ci .github Cargo.toml Cargo.lock` 无输出，仅描述当时证据提交的快照，不适用于当前 HEAD。当前 HEAD 已包含 `4cc6d0c` / `5ea14ec` 的两份测试增量（13 行），其变更范围见本报告快照，运行证据由下方 H3 CI run `40675450974` 提供。

PG故障注入中两次 `read audit sink did not recover before the pending age limit` 与对应通过的故障测试一致，不视为成功路径的异常。真实Trino结果与初验已核查的六个测试断言对应，因此补足#10的外部协议证据。Clippy采用新增成功原始日志，旧失败日志继续保留为历史。

Temp目录删除被自动审批拒绝的事实由证据索引记录；本次没有触碰或重试该目录。该清理限制不改变上述外部运行结果。所有补证据运行session已结束；本verifier本次没有创建运行中的编译/服务session。

### H3 Remote CI Evidence（20fd0005 历史快照）

H3 复签当时直接执行 `gh run view 40675450974 --json databaseId,headSha,status,conclusion,event,workflowName,jobs,url`，run为`completed/success`、event为`pull_request`、headSha精确等于当时交付HEAD `20fd0005bc93707d2363231f36b4d5ec43da06fb`。当时再读取integration、Ubuntu和Windows的job日志，确认实际发现数和执行结果，而非只采用绿色徽标。本节保留历史，不作为 06-11 修复证据。

| Job | ID | Result | 直接证据 |
| --- | --- | --- | --- |
| integration | 122772568752 | success | PG17.11启动、PG audit发现6项并6/6通过(1.63s)；独立Trino483受限/宽松实例启动、ACL发现6项并6/6通过(9.56s)，32条allow Display FINISHED；非零发现检查与cleanup均success。 |
| test (ubuntu-latest) | 122772569236 | success | workspace、fmt、Clippy、embedded-pg Clippy均success；L2 corpus step发现6项、6/6通过(0.04s)。 |
| test (windows-latest) | 122772569253 | success | workspace、fmt、Clippy、embedded-pg Clippy均success；L2 corpus step发现6项、6/6通过(0.07s)。 |
| isolated-query-bench | 122772571428 | skipped | 本次是pull_request，workflow条件限定workflow_dispatch isolated模式；该跳过不是Phase6必需CI任务未执行。 |

Run：`https://github.com/easyinplay/ontoforge/actions/runs/40675450974`。最后必需job于`2026-10-28T11:13:34Z`完成。PR日志中的`GITHUB_SHA=964fc0f3057d376496acc07240053730f6a836ee`是GitHub合成merge提交；run metadata的headSha确认它对应当前分支交付HEAD。本次未运行Cargo、未触发新workflow、未修改代码。

### 06-11 Final Incremental Verification

**核验范围:** 前轮唯一 gap #22。已亲读 `06-11-PLAN.md`、`06-11-SUMMARY.md`、更新的 `06-VALIDATION.md`、两个增量源码文件和全部下列原始行为证据。SUMMARY 的完成声明未计分；下面的源代码、真实断言、原始日志和本 verifier 的独立命令结果才是证据。

**实现与接线:** `Worker::Running(JoinHandle)` / `Worker::Finished(Result)` 在 `read_audit.rs:161` 定义、`:194` 初始化。`shutdown:232-242` 先停止 admission、取走根 sender，再持有 async mutex 借用 handle 等待；取消 waiter 释放锁但不移走 handle，其他 waiter 经 mutex 等同一结果。`handle.await` 完成后缓存 Result 的路径没有下一次 await，JoinError 映射为 `Err(AuditUnavailable)` 并保留。三条新 conformance 在 `mod.rs:2909-2911` 注册到实际两个 backend target，单元测试在 `read_audit.rs:455` 实际 abort worker。没有新依赖、API、schema 或外部状态来源。

| 行为 | 亲读的真实 RED | 亲读的 GREEN 与断言 |
| --- | --- | --- |
| 两个 shutdown 都等待持久化 | `06-11-RED-sqlite.log`、`06-11-RED-pg.log` 的并发测试实际触发 `shutdown must wait for the accepted in-flight guard` | `audit_concurrent_shutdown_waits_for_durability` 先将两个 future 显式 poll 到 Pending，确认表空，finish guard 后等待两者并查 principal；两后端 GREEN、新 CI 均 ok，成功后再调用仍 Ok。 |
| 取消首个 waiter 后重试 | 相同两份 RED 日志中的取消测试命中相同计划断言，不是编译或 fixture 失败 | `audit_cancelled_shutdown_retry_waits_for_durability` 将首个 future poll 到 Pending 后离开作用域，重试仍 Pending；finish 后查询实际持久行，再次 shutdown 成功。 |
| sink 失败在重复调用后保留 | 两后端 RED 均为首次 Err 后第二次实际 Ok，触发 `repeat shutdown must preserve the worker failure` | `audit_shutdown_failure_is_sticky` 注入真实 DB 拒绝触发器，首次和重复调用均 Err，表仍空；两后端 GREEN、新 CI 均 ok。 |
| worker JoinError 在重复调用后保留 | `06-11-RED-join-error.log`：0 passed、1 failed，实际 worker abort 后第二次 Ok 触发 `repeat shutdown must preserve the worker join failure` | `06-11-GREEN-join-error.log`：1 passed；两平台新 CI storage unit 49/49，命名 JoinError 测试均实际 ok。 |

本地 GREEN 原始日志（本轮仅亲读，未重跑）：`06-11-GREEN-sqlite.log` 为 8 passed、0 failed/ignored、1.13s；`06-11-GREEN-pg.log` 为 9 passed、0 failed/ignored、1.48s；`06-11-GREEN-join-error.log` 为 1 passed、0 failed/ignored。对应 SHA256 分别为 `F234F07C5CD4F2B0CBBB20E2A74E85E12D6BA79606AB3AED3F0851DCCB9A4782`、`50ACCF1A5A929F989F6E70E65C7BCD0199B06435BD9D5A02AAE11FE079C596F2`、`FB95D56D2BF87AF06224640E24912646E860B3411E52712BE06818AF5FFADC9D`，均由本轮 `Get-FileHash -Algorithm SHA256` 重算。三份 RED 普通日志证明 SQLite 5 passed/3 failed、PG 6 passed/3 failed、JoinError 0 passed/1 failed；RED 提交 `0bb29a3` 先于实现 `7813e7f`，亲读提交差异确认 RED 时尚无生产状态修复。

本 verifier 逐项独立执行 `node C:/Users/easyi/.codex/gsd-core/bin/gsd-tools.cjs check tdd-red-evidence <record> --raw`，四个 `06-11-RED-{concurrent,cancelled,failure,join-error}-junit.json` 全部 exit 0、`RED_EVIDENCE_OK`、`reason: target_test_failed`，`report_errors: []`。还亲读两个未修改的 XML，与 record 的失败测试和具体断言对应。独立执行 `check tdd-review-checkpoint 06` 得 `block:false`、`passed:true`、9 TDD plans、0 violations，06-11 被识别为 RED/GREEN Pass。`06-11-CLIPPY.log` 为 storage/all-targets/locked/-j2/-D warnings、Finished dev、38.72s；`06-11-FMT.log`、`06-11-DIFF-CHECK.log` 均明确 exitCode=0。

### Final Remote CI Evidence（2c9eee6）

本 verifier 以每次不超过 40 秒的等待持续查询 `gh run view 40682870456 --json headSha,status,conclusion,jobs`。等待期间 Windows workspace 正常由编译转到测试，再进入后续 corpus/cache；workspace 约 10m57s，未超旧同 job 的 11m16s。run 于 `2026-10-28T12:04:49Z` 成为 `completed/success`，`event: pull_request`，`headSha: 2c9eee6461a412ead8f998c7fa038acaa458b2f9`。结束后分别直接执行 `gh run view 40682870456 --job <id> --log` 并读取关键原始输出；没有以 badge、旧 run 或 SUMMARY 代替执行证据。

| Job | ID / 完成时间 | 原始证据与结果 |
| --- | --- | --- |
| integration | `122797742148` / `2026-10-28T11:54:16Z` | PostgreSQL 17.11 启动成功；full conformance 45/45（6.94s），专门 audit 发现 9 项并 9/9（1.70s），三条新增共享用例均 ok；独立受限/宽松 Trino 483 fixture 成功，ACL 发现 6 项并 6/6（8.05s），32 条 allow Display SQL 全部 FINISHED；非零发现、凭据扫描与 cleanup success。 |
| test (ubuntu-latest) | `122797741785` / `2026-10-28T11:54:22Z` | workspace 独立按该 step 汇总 117 个 libtest 结果组：1258 passed、0 failed、106 ignored；storage unit 49/49，SQLite conformance 43/43，三条共享回归与 JoinError unit 均实际 ok；L2 corpus 发现 6 项、6/6（0.03s）；fmt、Clippy、embedded-pg Clippy、其余必需步骤全部 success。 |
| test (windows-latest) | `122797742336` / `2026-10-28T12:04:48Z` | workspace 同口径 117 组：1255 passed、0 failed、106 ignored；storage unit 49/49，SQLite conformance 43/43，新增四项实际 ok；L2 corpus 发现 6 项、6/6（0.07s）；fmt、Clippy、embedded-pg Clippy、其余必需步骤全部 success。 |
| isolated-query-bench | `122797743646` | skipped，workflow 限定 workflow_dispatch isolated 模式，本次 PR 不触发，不是必需 job 漏跑。 |

计数边界：新增是 3 条共享 conformance 加 1 条 unit，共 4 类行为；共享用例在两种 backend 各运行，因此本地新增行为合同共有 7 个 target/test 组合。普通 workspace 的 PG ignored 不计为通过，PG 的实际执行由 integration 的 45/45 及专门 audit 9/9 证明。Ubuntu/Windows 的 workspace 总数分别记录，不把平台差异或重复 target 执行相加冒充唯一测试数。

Trino FINISHED 按 case-sensitive `\b[GB][0-9]{2}: FINISHED, [0-9]+ row\(s\)` 精确计 32 条，ID 为 `G01,G02,G03,G04,G05,G09,G12,G13,G14,G15,G16,G17,G18,G20,G21,G22,G24,G26,G28,G31,G32,G33,G34,G35,G36,G38,G39,G40,G42,G47,B51,B53`；不把 Cargo 的 `Finished` 行混入。checkout 原始日志的 `git log -1 --format=%H` 为 `5ff30dafc2bb39995d9b639a45bcf75c6b631bea`，这是 `pull/7/merge` 的合成 merge SHA；与 run metadata 的分支 `headSha` 明确区分。

新 run URL：`https://github.com/easyinplay/ontoforge/actions/runs/40682870456`。所有新代码的 CI 结论均来自该 run；旧 `20fd0005` run 只作历史保留。

### Advisory (New Scope, Unevidenced)

None。R1 为既定 carried-forward gap，本轮按完整实现、接线与行为证据复核后关闭；没有新增未获证据支持的 scope finding。Phase 1–5 的历史 fingerprint staleness 留给 milestone audit，不在本轮修改旧报告。

### Human Verification Required

None。R1 的四类行为已有真实测试证据；本轮交付 HEAD 的新 run 三个必需 job 均通过。外部 Codex unavailable 是已记录的非阻塞工具限制，不形成未验证 truth。Phase 7 的服务体验不在本阶段验收范围。

### Deferred Boundary

已通过`roadmap.analyze`及ROADMAP Phase7 SC2/SC6核对：服务持有monitor生命周期、服务优雅关闭、HTTP `/health`告警和MCP工具过滤均属Phase7，ADR-0028已接受。本轮未将它们新增为Phase6失败truth。库级 `ReadAudit::shutdown` 的可靠排空与错误传播由 Phase 6 交付，不受该延期覆盖。H3 保持关闭，L2-04/SCOPE-07 整体保持 Pending。

### Gaps Summary

原 21 条 truth 的证据保留，唯一 carried-forward gap R1/CR-01 经 06-11 实现、四类真实 RED/GREEN、独立 classifier/TDD gate 与新交付 HEAD 的三 job CI 关闭。当前 `22/22`、`behavior_unverified=0`、`passed`，无剩余 gap 或人工验收项，AUDIT-01 的本阶段合同已满足。此结论不关闭 L2-04/SCOPE-07 的跨阶段合同，不表示 Phase 7 启动、PR 合并或发布授权。

---

_Verified: 2026-10-28T12:08:10Z_

_Verifier: the agent (gsd-verifier)_
