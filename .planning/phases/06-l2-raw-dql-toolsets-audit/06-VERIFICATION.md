---
phase: 06-l2-raw-dql-toolsets-audit
verified: 2026-10-28T03:42:11Z
status: human_needed
score: 19/21 must-haves verified
behavior_unverified: 2
overrides_applied: 0
re_verification:
  previous_status: human_needed
  previous_score: 16/21
  gaps_closed:
    - "H1: PostgreSQL parity 与故障路径原始证据已核验（truth #8/#9）。"
    - "H2: Trino 483 ACL/活跃性/清理/Display 原始证据已核验（truth #10）。"
  gaps_remaining:
    - "H3: 当前交付 SHA 的远端 CI 实际运行（truth #17/#18）。"
  regressions: []
covered_files: [".github/workflows/ci.yml",".planning/phases/06-l2-raw-dql-toolsets-audit/.evidence/.gitattributes",".planning/phases/06-l2-raw-dql-toolsets-audit/.evidence/clippy-workspace-20261028.log",".planning/phases/06-l2-raw-dql-toolsets-audit/.evidence/external-cleanup-20261028.log",".planning/phases/06-l2-raw-dql-toolsets-audit/.evidence/external-pg-audit-20261028.log",".planning/phases/06-l2-raw-dql-toolsets-audit/.evidence/external-trino-acl-20261028.log",".planning/phases/06-l2-raw-dql-toolsets-audit/.evidence/trino-fixture-start-20261028.log",".planning/phases/06-l2-raw-dql-toolsets-audit/06-01-PLAN.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-01-SUMMARY.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-02-PLAN.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-02-SUMMARY.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-03-PLAN.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-03-SUMMARY.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-04-PLAN.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-04-SUMMARY.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-05-PLAN.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-05-SUMMARY.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-06-PLAN.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-06-SUMMARY.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-07-PLAN.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-07-SUMMARY.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-08-PLAN.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-08-SUMMARY.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-09-PLAN.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-09-SUMMARY.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-10-PLAN.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-10-SUMMARY.md",".planning/phases/06-l2-raw-dql-toolsets-audit/06-EXTERNAL-EVIDENCE.md","Cargo.lock","ci/query-services.sh","ci/trino/l2/access-control.properties","ci/trino/l2/config.properties","ci/trino/l2/rules.json","ci/trino/l2/seed.sql","crates/ontoforge-action/tests/entrypoints.rs","crates/ontoforge-engine/Cargo.toml","crates/ontoforge-engine/src/config.rs","crates/ontoforge-engine/src/printer/l2.rs","crates/ontoforge-engine/src/printer/mod.rs","crates/ontoforge-engine/src/trino/decode.rs","crates/ontoforge-engine/src/trino/headers.rs","crates/ontoforge-engine/src/trino/l2_health.rs","crates/ontoforge-engine/src/trino/l2_monitor.rs","crates/ontoforge-engine/src/trino/mod.rs","crates/ontoforge-engine/tests/data/l2_corpus.json","crates/ontoforge-engine/tests/l2_gate.rs","crates/ontoforge-engine/tests/l2_health.rs","crates/ontoforge-engine/tests/l2_monitor.rs","crates/ontoforge-engine/tests/trino_l2_acl_it.rs","crates/ontoforge-query/src/aggregate.rs","crates/ontoforge-query/src/execute.rs","crates/ontoforge-query/src/l2.rs","crates/ontoforge-query/src/lib.rs","crates/ontoforge-query/tests/audit_l0.rs","crates/ontoforge-query/tests/audit_pk.rs","crates/ontoforge-query/tests/definition_snapshot.rs","crates/ontoforge-query/tests/l2_discovery.rs","crates/ontoforge-query/tests/l2_execute.rs","crates/ontoforge-storage/migrations/postgres/0005_read_audit.sql","crates/ontoforge-storage/migrations/postgres/0006_audit_truncated.sql","crates/ontoforge-storage/migrations/sqlite/0005_read_audit.sql","crates/ontoforge-storage/migrations/sqlite/0006_audit_truncated.sql","crates/ontoforge-storage/src/lib.rs","crates/ontoforge-storage/src/read_audit.rs","crates/ontoforge-storage/src/reload.rs","crates/ontoforge-storage/src/scope_store.rs","crates/ontoforge-storage/src/sql_exit.rs","crates/ontoforge-storage/src/store.rs","crates/ontoforge-storage/tests/conformance/mod.rs","crates/ontoforge-storage/tests/conformance_pg.rs","crates/ontoforge-storage/tests/conformance_sqlite.rs","crates/ontoforge-storage/tests/scope_store.rs","crates/ontoforge/src/lint_cmd.rs","crates/ontoforge/src/reload.rs","crates/ontoforge/src/scope.rs","crates/ontoforge/tests/cli_reload.rs","crates/ontoforge/tests/cli_scope.rs"]
covered_digest: "v3:sha256:199f5642faf1829bd58b920f30760a9bd85d83905ce8ff5179e4921152ed9238"
behavior_unverified_items:
  - truth: "06-08: CI 实际启动独立受限 Trino 483，并以测试发现数大于零的门禁跑 ACL、104-ID corpus 的 Trino 行为。"
    test: "取得当前交付 SHA 的 GitHub Actions integration/test jobs 运行及日志。"
    expected: "独立 Trino 483 ready，发现数大于零，ACL、Display corpus 实际执行且 jobs 成功。"
    why_human: "CI 配置已接通；gh 对 HEAD 与最终实现 SHA 的精确查询均返回 []。"
  - truth: "06-08: CI 实际运行 PostgreSQL 审计 conformance，发现数大于零；Windows workspace 回归通过。"
    test: "取得同一交付 SHA 的 PostgreSQL audit CI step 与 Windows test job 日志。"
    expected: "PG audit 发现数大于零且真实运行，Windows workspace job 成功。"
    why_human: "本地最终 workspace 原始日志已核验，但本地结果不能证明 CI 已运行。"
human_verification:
  - test: "验收当前交付 SHA 的远端 CI。"
    expected: "Windows/Linux test 与 integration jobs 成功，PG/Trino 目标发现数大于零并实际执行。"
    why_human: "两条 PLAN truth 明确要求 CI 实际运行；当前两次精确 SHA 查询均无 run。需完成远端执行，或由开发者显式接受有记录的验收变更。"
---

# Phase 6: L2 Raw DQL, Toolsets & Audit Verification Report

**Phase Goal:** As a 受信主体, I want to 在 allow-list 内执行仅在 Trino 侧权限可证明安全时才开放的只读 Trino SQL, so that 所有级别的读取都留下元数据审计且定义与授权的变更只经部署方与管理通道.

**Verified:** 2026-10-28T03:42:11Z

**Status:** human_needed

**Re-verification:** Yes，基于 `c674e32` 的初验报告做窄范围证据复签；只复核原5条未验证项中的H1/H2及Clippy证据，保留原16条结论。无 overrides。

**快照:** 证据提交 HEAD `ecb9883`；最终实现及全量本地测试快照 `e81127fa41099365c44c2c42750024efad6653cb`。已执行 `git diff e81127f HEAD -- crates ci .github Cargo.toml Cargo.lock`，无差异。

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
| Outcome：安全读取有审计，定义和授权只经既定管理通道 | 库级路径完整，真实服务与当前 SHA 的 CI 证据齐全 | 以下 19 条已验证；2 条远端CI执行证据未闭合 | UNCERTAIN（WARNING；human_needed） |

真实PostgreSQL和Trino验收证据现已补齐，当前SHA的远端CI仍未签署。以下章节保留初验源码及自动化证据，并记录本次复签的原始日志核对；本地外部测试不代替远端CI。

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
| 17 | 06-08：CI 实际启动独立受限 Trino 483，并以测试发现数大于零的门禁跑 ACL、104-ID corpus 的 Trino 行为 | PRESENT_BEHAVIOR_UNVERIFIED（WARNING） | `.github/workflows/ci.yml:54,140,142` 接线正确；`gh run list --commit d6903af...` 及 `--commit e81127f...` 均 `[]`。配置不能证明实际执行。H3。 |
| 18 | 06-08：CI 实际运行 PostgreSQL 审计 conformance，发现数大于零；Windows workspace 回归通过 | PRESENT_BEHAVIOR_UNVERIFIED（WARNING） | `.github/workflows/ci.yml:97` PG service、URL 和非零发现门禁；本地 W 通过，但当前 SHA 无远端 run。H3。 |
| 19 | 06-09：reload lint/validate 失败不切换，scope hash 错误不写，成功管理变更事务性写 of_admin_audit；远程 L2 工具无管理入口 | VERIFIED | `reload.rs:37` 与 `lint_cmd.rs:18` 前置链；`scope_store.rs:501,544` hash 先于 DML，审计随事务；W `invalid_definitions_never_open_store`、`stale_plan_hash_rejected`、`audit_insert_failure_rolls_back_scope_and_indexes` GREEN。L2 public surface 仅六个只读方法，实际 MCP 列表归 Phase 7。 |
| 20 | 06-10：Every cycle owns a fresh bounded ontology query, proves RUNNING across visibility observations, and cleans up before enabling. | VERIFIED | `l2_monitor.rs:249,290,319` 每轮自有 query、新ID、25s server limits、同origin；`:203` DELETE 成功才 complete。本轮跨代清理 GREEN；W `monitor_startup_owns_fresh_probe_limits_and_cleanup`、`monitor_cannot_enable_before_delete_response_succeeds` GREEN。 |
| 21 | 06-10：Periodic failure, expiry, shutdown and cancellation fail closed; recovery requires a fresh complete attestation. | VERIFIED | `l2_health.rs:95,114` TTL/epoch/generation；`l2_monitor.rs:40,203,249` shutdown/cleanup；本轮 periodic recovery GREEN；W 含 expiry unit、取消、Drop、shutdown竞争与deadline GREEN。 |

**Score:** 19/21 truths verified，2 条 present、behavior-unverified。无 FAILED truth、实现 BLOCKER 或 override；这不是 `passed`。

合并规则：保留全部五条 ROADMAP SC；06-03 两条细化并入 #1，06-04 默认 fail-closed 和 06-10 startup 并入 #3，06-09 请求快照并入 #5；其余 16 条 PLAN truth 保留。10 个 PLAN 的 21 条原始 truth 均有对应。没有发现 `prohibitions`、`verification: backstop` 或 `<human-check>`。

### Required Artifacts

十个 PLAN 的 artifact query 共检查 20 项（含重复），均 exit 0、`all_passed:true`。下表为去重后 17 项及额外关键生命周期实现；VERIFIED 表示源码存在、实质实现且接通，外部运行证据的限制仍按上表处理。

| Artifact | Expected | Status | Details |
| --- | --- | --- | --- |
| `storage/src/read_audit.rs` | 有界异步元数据审计 | VERIFIED | reserve、guard Drop、sink健康、batch、双后端事务写入均非 stub。 |
| `storage/migrations/sqlite/0005_read_audit.sql` | SQLite 审计表 | VERIFIED | store migration 接入；0006 增加 truncated 状态，升级测试 W GREEN。 |
| `storage/migrations/postgres/0005_read_audit.sql` | PG system schema 审计表 | VERIFIED | store PG migration 接入；0006 同步状态；真实PG执行已由H1原始证据验证。 |
| `storage/tests/conformance/mod.rs` | 双后端共用合同 | VERIFIED | 真实字段/retention/事务故障断言，SQLite/PG targets 均调用；PG runtime已由H1验证。 |
| `engine/src/printer/l2.rs` | 单受控 AST/Display 入口 | VERIFIED | visitor、关系/函数检查、LIMIT、重解析相等、私有构造。 |
| `engine/tests/data/l2_corpus.json` | 104-ID 机器 corpus | VERIFIED | include_str 接入生产 gate 测试与真实 Trino 测试；明确 constructed-equivalent。 |
| `engine/src/trino/l2_health.rs` | attestation 状态和检查 | VERIFIED | 配置、身份、coordinator、正反探针、TTL、epoch/generation。 |
| `ci/trino/l2/rules.json` | 独立受限权限配置 | VERIFIED | ontology/L2分离、catalog/table/查询owner规则，挂载到独立容器；真实协议已由H2验证。 |
| `ci/query-services.sh` | fixture 启停 | VERIFIED | pin Trino483、独立受限/宽松实例、ready/seed检查、cleanup；CI已调用。 |
| `engine/tests/trino_l2_acl_it.rs` | 真实ACL和Display验收 | VERIFIED | 六条明确 ignored 外部测试；H2本地真实运行已验证；远端CI仍待H3。 |
| `query/tests/audit_l0.rs` | L0公开读取审计回归 | VERIFIED | list/set/searchAround/aggregate、取消和故障断言，W GREEN；PG-only单独ignored。 |
| `query/src/l2.rs` | 六个只读库操作 | VERIFIED | public methods 接到统一审计、授权和 engine 路径。 |
| `query/tests/l2_execute.rs` | execute/explain合同 | VERIFIED | 12条 W GREEN，验证请求体、拒绝零POST、审计与边界。 |
| `query/tests/l2_discovery.rs` | 四discovery合同 | VERIFIED | 9条 W GREEN，覆盖名称过滤、字段预算、取消和故障。 |
| `.github/workflows/ci.yml` | 真实服务CI门禁 | VERIFIED | workflow接通非零发现/实际运行命令；远端运行仍待 H3。 |
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
| `.github/workflows/ci.yml` | `engine/tests/trino_l2_acl_it.rs` | Cargo --test target | WIRED | `:140` 启动fixture，`:142`之后先list计数再--ignored执行；实际run待H3。 |
| `query/tests/definition_snapshot.rs` | `query/src/lib.rs` | ActiveDefinition与query边界 | WIRED | 两个竞争测试调用生产service，在源阻断后做scope/reload并核对审计。 |
| `engine/src/trino/l2_monitor.rs` | `engine/src/trino/l2_health.rs` | owner/generation/epoch及complete | WIRED | `run_cycle:249` 串行检查/清理后complete，stop/expiry阻止旧响应重启用。 |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
| --- | --- | --- | --- | --- |
| `query/src/l2.rs` execute/explain | `output.rows` | 受控Display -> Trino POST -> 解码/限额 -> L2Result | 是；`trino/mod.rs:312` body取PrintedSql，未丢弃响应 | FLOWING；真实483复验H2已通过 |
| `query/src/l2.rs` discovery | catalog/schema/table/columns | engine元数据SQL -> Trino返回 -> scope过滤 -> 有界L2Result | 是；没有硬编码空props或静态fallback | FLOWING |
| L0公共query边界 | QueryPage / aggregate | 现有源查询及overlay -> 授权 -> 结果；审计finish在返回之前 | 是；测试结果与审计精确匹配 | FLOWING |
| `storage/src/read_audit.rs` | ReadAuditRecord | 真实请求metadata、SQL SHA256、状态/行数/耗时 -> 有界队列 -> 参数化INSERT | 是；SQL/result原文不入record/table | FLOWING；真实PG复验H1已通过 |
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
| 当前SHA远端CI | `gh run list --commit <SHA> --json databaseId,headSha,status,conclusion` | 初验d6903af/e81127f均`[]`；本次查询证据提交ecb9883仍`[]`，exit0 | SKIP（WARNING H3） |

W = `C:/Users/easyi/AppData/Local/Temp/ontoforge-phase06-10-workspace-accepted.log`。该日志持有最终实现快照的实际libtest输出，非SUMMARY转述；其编译/链接行显示29m55s。本轮读取其命名结果和总计，并确认当前实现/CI未变。初验W中的外部服务ignored不计分；本次新增真实运行原始输出后，#8/#9/#10才升级为VERIFIED。

### Probe Execution

| Probe | Command | Result | Status |
| --- | --- | --- | --- |
| 独立`probe-*.sh` | 扫描全部PLAN/SUMMARY及常规scripts入口 | 仓库无scripts目录，也无声明的probe脚本；部署方`active_query_sql`是运行时ACL探针，不是shell probe | N/A |
| ACL运行时探针 | 生产monitor测试及真实集成target | 库级命名行为PASS；真实服务H2已通过 | 见上表 |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
| --- | --- | --- | --- | --- |
| L2-01 | 06-06/07/08/09 | raw DQL与allow-list discovery | 库级SATISFIED；CI NEEDS HUMAN | #13-18，六操作有源数据/授权/审计接线。 |
| L2-02 | 06-03/06/08 | AST白名单与Display | SATISFIED（gate与真实483）；远端CI待H3 | #1，104-ID生产gate亲跑；明确等价新语料而非原文。 |
| L2-03 | 06-03/04/06/07/08/10 | catalog/schema allow-list，硬排除system/edit | SATISFIED（库级与真实ACL）；远端CI待H3 | #2/#10/#15。 |
| L2-04 | 06-04/06/07/08/10 | 启动/周期fail-closed、健康原因 | 库级SATISFIED；整体Pending | #3/#20/#21；H2已验证，H3待验。服务持有handle、优雅关闭及HTTP `/health` 按ADR-0028由Phase7验收。 |
| AUDIT-01 | 06-01/02/05/06/07/08/09 | 全读取元数据审计、背压、保留期 | SQLite/PG及查询边界SATISFIED；远端CI NEEDS HUMAN | #4/#6-9/#11-18。 |
| SCOPE-07 | 06-09 | 管理变更审计与请求固定版本 | 当前阶段SATISFIED；整体Pending | #5/#19及竞争测试；MCP无变更工具限制属Phase7。 |

PLAN requirement并集与REQUIREMENTS中Phase6映射一致，无ORPHANED requirement。没有更新需求checkbox，L2-04/SCOPE-07整体仍为Pending。

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
| --- | --- | --- | --- | --- |
| 阶段修改源码/测试及fixture | 扫描范围见fingerprint及PLAN files_modified | 未发现无正式引用的TBD/FIXME/XXX债务注释；无待实现handler或静态结果替代 | 无BLOCKER | `mktemp`模板中的`XXXXXXXX`不是债务注释；match空分支是已校验成功路径，不是空实现。 |
| `query/tests/audit_l0.rs` | 484 | PG-only显式ignore | INFO | 初验workspace中仍为ignored，未把该单项算为通过；H1所需六条审计conformance现有真实PG运行证据。 |
| `engine/tests/trino_l2_acl_it.rs`、`storage/tests/conformance_pg.rs` | 外部targets | 普通workspace显式ignore | INFO | 本次额外原始日志证明--ignored真实执行，H1/H2闭合；H3远端运行继续WARNING。 |
| `engine/tests/l2_monitor.rs` | registry及DELETE分支 | 已核查历史“假RUNNING”风险修复 | INFO | query registry按ID/owner/state判断、成功DELETE移除，未发现用任意ID回显假活跃证明。 |
| `ontoforge-phase06-10-clippy-final-fixed.log`（Temp） | 文件末尾 | `items_after_test_module`历史失败 | INFO（历史记录） | 旧log仍为失败，未被覆盖或改写。本次另亲读`.evidence/clippy-workspace-20261028.log`：workspace/all-targets/locked/-j2/-D warnings，Finished dev profile、exit0、9.84s；当前实现与最终快照无diff，最终Clippy GREEN现有直接证据。 |

已扫描核心L2/query测试的ignore与明显自比较断言；没有发现用`assert!(true)`或相同变量自比较代替行为的情况。初始空状态、合法match空分支和测试fixture均未误判为stub。没有为只读验证新增测试或修改实现。

### Decisions And Quality Gates

`check.decision-coverage-verify` 返回 `total:6, honored:6, blocking:false`；本轮同时从实际源码核对D-01至D-06的授权边界、独立身份、PrintedSql、审计、管理通道及constructed-equivalent corpus。该机械结果不覆盖H1-H3的运行证据缺口。

已读原始 `06-REVIEW.md`、`06-REVIEW-DISPOSITION.md`、`06-10-REVIEW.md` 和 `06-SECURITY.md`，并追踪历史CR-01/WR-01/WR-02及增量清理/代际隔离修复。审查文档的“closed/verified”只作为检查索引，未代替代码和行为证据。正式PR、远端CI、外部Codex gate、发布审批未执行。本轮不提交代码或报告。

`ralph-loop` 本机不可用，未声称执行。GSD verifier的agent_skills配置为空；已读取`gsd-verify-work`及必须引用的验证规则。没有增加subagent，也没有修改共享规划文件。

### Incremental Evidence Review

本次仅亲读新增证据并核对来源，没有运行Cargo、重新构建fixture或重跑完整验证。读取 `06-EXTERNAL-EVIDENCE.md` 作为证据索引，再逐份读取下表原始日志；所有SHA256均由本次重新计算并与索引一致。还逐份验证 `git hash-object <log>` 等于 `git rev-parse :<log>`，五份日志index与disk字节一致；目录`.gitattributes`保留CRLF，`git ls-files --eol`显示i/crlf、w/crlf、attr/-text。

| Raw Log（相对phase目录） | SHA256 | 本次直接核验 |
| --- | --- | --- |
| `.evidence/external-pg-audit-20261028.log` | `7B5988A761068E98F60A35A66152DF85BF6A51BA7DE0ED59B2F649BFE4189279` | PostgreSQL17.11、6项发现、六个命名audit测试均ok、1.62s、test_exit_code=0。 |
| `.evidence/external-trino-acl-20261028.log` | `36A81A389A1262D2933DEF476E068369D9679C5FE535371314F3F99DEB2D9259` | 两实例483/ready、默认并行6/6、32条FINISHED、9.91s、test_exit_code=0。 |
| `.evidence/clippy-workspace-20261028.log` | `89C635C6C657D6E38E9C81C45F348B4CB1C5B9E56DCF1A9FF7ED048FB012E0C5` | 精确workspace/all-targets/locked/-j2/-D warnings命令、Cargo9.30s、计时9.8378895s、clippy_exit_code=0。 |
| `.evidence/trino-fixture-start-20261028.log` | `9CAD2F3D229B944D22225B22ACA86E87A1A6B9BAE7278B2014CAACDAEF608AAA` | 两个独立容器ID、seed输出、fixture_exit_code=0；ID与/v1/info及cleanup记录一致。 |
| `.evidence/external-cleanup-20261028.log` | `8181213775AE4B4AB4C62A773747B279B50D22FB9418A1FBC1E06AC081883636` | 本轮拥有的两容器已删除，原Trino/PG/Casdoor服务保留。 |

二进制来源经过交叉核验，不以mtime单独证明：本次重新计算 `conformance_pg-39e883f11e6a6c0e.exe` 的SHA256为 `453A0ACC8A4C423F7ED526D532758664BDA9A0B9C721F33E4FB93A773DF7D421`，`trino_l2_acl_it-0af57445ba4215b1.exe` 为 `C4DEC5B77F8188264AB76734436B0B90650F2B1881283587622119E06EC14DAF`，与外部原始日志一致。亲读最终accepted workspace日志对应的Running行，列出相同两个executable；该workspace日志本次重新计算SHA256为 `B53F76BD2FC9E1C2ED40B489CBBCACB35E0000E9218176F0DD15F4F869F0E4D8`，与证据索引一致。本次再次执行 `git diff e81127f HEAD -- crates ci .github Cargo.toml Cargo.lock`，无输出，证据提交没有改变实现、测试或fixture。

PG故障注入中两次 `read audit sink did not recover before the pending age limit` 与对应通过的故障测试一致，不视为成功路径的异常。真实Trino结果与初验已核查的六个测试断言对应，因此补足#10的外部协议证据。Clippy采用新增成功原始日志，旧失败日志继续保留为历史。

Temp目录删除被自动审批拒绝的事实由证据索引记录；本次没有触碰或重试该目录。该清理限制不改变上述外部运行结果。所有补证据运行session已结束；本verifier本次没有创建运行中的编译/服务session。

### Advisory (New Scope, Unevidenced)

None。本次仅复签既有H1/H2与Clippy证据，没有新增scope或更改验收合同。

### Human Verification Required

#### H3. 当前交付SHA远端CI

**Test:** 完成当前交付SHA的GitHub Actions test/integration运行并检查每个step输出。初验只读核查d6903af/e81127f均无run，本次对ecb9883查询仍无run。

**Expected:** Windows/Linux workspace成功；独立Trino483和真实PG启动，ACL/corpus与audit目标发现数均大于零且实际执行。

**Why human:** 06-08两条truth明确要求“CI实际运行”。本地Windows1251 passed与workflow存在不满足该运行断言。需要可审查的远端证据，或开发者显式接受有记录的验收变更；本报告未自行添加override。对应#17/#18。

### Deferred Boundary

已通过`roadmap.analyze`及ROADMAP Phase7 SC2/SC6核对：服务持有monitor生命周期、服务优雅关闭、HTTP `/health`告警和MCP工具过滤均属Phase7，ADR-0028已接受。本轮未将它们新增为Phase6失败truth，也没有把H3延期到Phase7。剩余两条远端CI证据缺口没有被后续阶段的泛化目标吞掉。

### Gaps Summary

没有发现可直接证明的实现缺失、stub、断链或未引用债务BLOCKER。H1/H2的原始PG/Trino证据经本次独立复核后，#8/#9/#10升级为VERIFIED；仅#17/#18仍为PRESENT_BEHAVIOR_UNVERIFIED，合并为H3一个远端CI验收组，因此状态必须是`human_needed`，不能把全部任务已完成、已有review签署或本地测试绿转写为Phase目标`passed`。19/21分数保留初验16条并新增3条具有可核验原始行为证据的truth；剩余2条远端CI truth不计分。

---

_Verified: 2026-10-28T03:42:11Z_

_Verifier: the agent (gsd-verifier)_
