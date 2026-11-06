---
milestone: v1.0
audit_date: 2026-11-05
auditor: gsd-integration-checker
status: static_wiring_verified
inspected_head: 060019fe52005c6b85ae5c5f59da9868f464e579
source_equivalent_to: 8d9d8c6
method: independent-current-source-call-trace
dynamic_reverification: completed_by_separate_owner
dynamic_evidence: FINAL-REVERIFY.md
boundaries: { wired: 12, broken: 0 }
flows: { statically_wired: 9, broken: 0, dynamically_rerun_here: 0 }
---

# v1.0 跨阶段集成审计

## 结论与证据边界

**12/12 个列明边界 WIRED，9/9 条列明用户流静态 WIRED，新增 BLOCKER 为 0。INT-01 的生产接线已修复。此结论不是当前源码动态 E2E 通过，也不是 milestone passed 或 79/79 需求验收完成。**

本报告独立重做，以亲读的当前工作树源码、实际调用与返回路径为依据。已撤回的 fa81ca0 通过结论不作为证据；旧报告只用于保持 B01-B12、F01-F09 标识连续，不继承其判断。没有执行 Cargo、Docker、服务启动、CI 触发、commit 或 push。真实复验由另一个 owner 负责，本报告不替其签字。

必须保留的判断：

- Phase01：`human_needed`，不把源码或自动化配置检查当编辑器人工体验验收。
- Phase02：`gaps_found`，不改旧报告或 fingerprint。其历史 MinIO 缺口由 Phase07.2 后继证据关闭，不意味着原报告在原时点通过。
- ENGINE-05：**Partial，SQL Server 私有 fixture/来源 judgment 仍未闭环**。07.2 仅关闭 MinIO 合同缺口，不关闭整个需求。
- 07.1/07.2 已合入 main：读取合并记录，并以 `git merge-base --is-ancestor` 验证 `aac4fd8`、`73bbec2` 均为当前 HEAD 祖先；Phase03-07 的 `ef0ab9c` 同样成立。未在线重新查询 PR。

已读主会话 `task_plan.md`、`findings.md`、`.planning/STATE.md`、`.planning/REQUIREMENTS.md`，各阶段代表性 SUMMARY 的 provides/requires/key-files，以及相关 VERIFICATION、L2 修复、07.1 原始日志、07.2 运行与 CI 证据。纯审计跳过新功能讨论、brainstorming 和业务 TDD；主会话已持久化计划，不重复修改共享文档。ralph-loop/Warp 不可用，以逐项验收闭环替代，不声称调用。

### 源码身份

- 当前 HEAD：`060019fe52005c6b85ae5c5f59da9868f464e579`。
- `git diff --name-only 8d9d8c6 HEAD` 仅列 `.planning/HANDOFF.json`、`.planning/STATE.md`、`.planning/milestones/v1.0-audit/.continue-here.md`。两个 commit 的完整 tree 不同，源码相同，不能写成完整 tree 相同。
- 对 `crates clients ci .github Cargo.toml Cargo.lock schemas examples tools` 的定向 diff 为空；审计时工作区已有其他 worker 的状态/证据/文档变化，未覆盖或清理。
- 关键接线文件 SHA256 与查询结果另存 `evidence/2026-11-05/integration-final-provenance.json`。这是静态定位证据，不是运行日志。
- 项目无 `.codegraph/`、`.codex/skills/` 或 `.agents/skills/`。GSD `query agent-skills gsd-integration-checker --skills-only` 未返回配置技能；已读 bootstrap 与 planning-with-files skill，未启动另一个顶层工作流。

## Wiring Summary

计数单位为下表列出的集成边界，不是 public export 总数。**Connected：12；Missing：0；导致预期流断链的关键 orphan：0。** 不声称穷举全仓库 export。

### 提供/消费图

| Phase | 从 SUMMARY 与源码提取的提供项 | 已追到的消费者与调用 |
|---|---|---|
| 01 | load_dir、profile/expr、lint、schema IR | CLI pipeline/lint_cmd → reload/serve；IR → scope/query/action |
| 02 | Engine、Trino/SQLite、printer、binding/PK/drift | CLI online、query source lookup、Action source snapshot、server drift、serve |
| 03 | EditStore、reload/migrate、request_snapshot、auth/scope、history | context → QueryService/ActionService/history；下一请求重编译已生效 scope |
| 04 | QueryService、read_pk_source、overlay、cursor/version、aggregate | REST/MCP audited methods；Action source.rs 共用 bounded source lookup |
| 05 | execute_with_snapshot、guarded commit、receipt/outbox | REST/MCP action、command result、history、独立 webhook-worker |
| 06 | L2 AST policy/ACL proof、read audit | MCP adapter 消费 policy/dedicated engine；query/aggregate/L2 共用 store audit；lifecycle drain |
| 07 | router、MCP catalog/action schema、stdio、OpenAPI、SDK/dev/package | Rust/TS 实际 HTTP；stdio → /mcp；release smoke 调用解包后的二进制 |
| 07.1 | source OneToOne scanner、--check-cardinality、ONT-W18 | CLI online → binding 筛选 → scanner → printer/Engine → 定位诊断/退出码 |
| 07.2 | owner-isolated MinIO、S3/restart、nightly runner | workflow → iceberg-services.sh nightly → PG/MinIO/Trino → Rust targets → 计数/S3/cleanup 门 |

### 12 个集成边界

路径相对仓库根目录，行号指本次读取的源码。每项不仅检查 import，还检查调用、数据终点与失败分支。

| ID | 提供方 → 消费方及实际数据路径 | 错误/空态与终点 | 需求 | 判定 |
|---|---|---|---|---|
| B01 | P1/P2 → CLI/P07.1：`crates/ontoforge/src/pipeline.rs:26` load/profile/CEL；`crates/ontoforge/src/online.rs:275` describe/binding → PK/cardinality 独立分支；`:310` → `crates/ontoforge-engine/src/cardinality_check.rs:34` / `:127` | offline 错误不进引擎；不可用 binding 不扫描；online `:345` 定位诊断。source OneToOne 非 NULL FK 按 distinct PK 计数，非 referential integrity，不阻断读 | SCHEMA-01..06、BIND-01..03、STORE-05 | WIRED |
| B02 | P1/P2 → P3：`crates/ontoforge/src/reload.rs:35` lint 成功才 open/reload；`crates/ontoforge-storage/src/reload.rs:142` 锁/role/DDL/Trino 可见性/切换；`crates/ontoforge-storage/src/migrate.rs:365` 同类合同 | lint/role/锁/可见性失败不提前切 active；request_snapshot 只读实际生效版本 | LINT-01、BIND-02、STORE-02、SCOPE-06/07 | WIRED |
| B03 | P3 → P7：`crates/ontoforge/src/serve.rs:109` Authenticator；`crates/ontoforge-server/src/context.rs:401` authenticate → request_snapshot → compile_scope → for_principal；`crates/ontoforge-storage/src/request_snapshot.rs:23` 同锁读取定义/scope 后结束事务 | token/scope 失败拒绝，无 grant 不可见；MCP 每次 request_context 重建当前授权，不复用初始化旧权限 | AUTH-01/02、SCOPE-01..07、API-01、MCP-06 | WIRED |
| B04 | P2/P3 → P4：`crates/ontoforge-query/src/lib.rs:307` edit → tombstone → source 必要性 → overlay → evaluate → mask → HMAC version；`:513` printer、`:520` guard → engine | 不存在/越权均 NotFound；source 重复/故障安全错误；完整 patch/editOnly 不强制依赖 Trino；返回 ObjectRow/QueryPage | ENGINE-01..04、STORE-01/07、SCOPE-03..05、QUERY-01/08/09 | WIRED |
| B05 | P4 → P6 → P7：REST `crates/ontoforge-server/src/rest.rs:168` / `:281` / `:360` 与 MCP 调 QueryService audited 方法；`crates/ontoforge-query/src/execute.rs:527`、`crates/ontoforge-query/src/aggregate.rs:912` reserve → execute → finish | `crates/ontoforge-storage/src/read_audit.rs:202` 拒绝积压/不健康 sink；`:326` worker → SQLite/PG/retention；空/截断/失败也走终结/DTO | QUERY-02..07/09、AUDIT-01、API-01、MCP-03/06 | WIRED |
| B06 | P4/P3 → P5 → P7：REST `crates/ontoforge-server/src/rest.rs:457` / `:481`、MCP `crates/ontoforge-server/src/mcp.rs:649` → execute_with_snapshot；`crates/ontoforge-action/src/lib.rs:324` prepare → `:386` commit → `crates/ontoforge-storage/src/commit.rs:617` fence/claim/locks/CAS/criteria | validate-only 走 guard 后 rollback；成功同事务 edits/log/history/command/outbox → receipt；replay 前复核 scope.actions，异参拒绝 | ACTION-01..10/12、STORE-03/05、QUERY-09、SCOPE-07 | WIRED |
| B07 | P5 → P3 → P7/SDK：REST history 先按当前 scope 读对象；`crates/ontoforge-server/src/history_page.rs:8` → `crates/ontoforge-storage/src/history.rs:120` 固定定义 → `:224` 可见列 → DTO/SDK | 隐藏值不回传，空页/字节预算/错误有结果，不直接暴露 raw log | STORE-04、ACTION-06/10、API-01/04/05、MCP-06 | WIRED |
| B08 | P5 → CLI → HTTP：commit 写 outbox；`crates/ontoforge/src/webhook_worker.rs:54` poll_at → `crates/ontoforge-action/src/outbox.rs:123` claim/send/mark_delivered/record_retry | 持久 payload 与 X-Ontoforge-Event-Id，2xx 标记、错误退避、lease/fencing 恢复；生产接收端/自动重启未由源码证明 | ACTION-11、STORE-03 | WIRED |
| B09 | P2 → P7 → P4/P5：`crates/ontoforge-server/src/lib.rs:167` start_drift；`crates/ontoforge-server/src/drift.rs:155` 首轮/30 秒周期；`crates/ontoforge-server/src/context.rs:349` ScopedEngine → drift.check | Query source 执行前检查，Action 共用 source lookup；`crates/ontoforge-server/src/health.rs:16` 同定义 drift_view；不扩大到不需 source 的请求 | BIND-03、ENGINE-02、QUERY-01、ACTION-12 | WIRED |
| B10 | P6 → P7：`crates/ontoforge/src/serve.rs:177` start_l2_acl_monitor → `:211` set_l2_engine/install；MCP 只取 Enabled engine；`crates/ontoforge-engine/src/trino/mod.rs:465` admission 读锁重查 proof | 未配置/失败/过期 fail-closed，无 source 身份回退；health 取同一 state；lifecycle 关闭 monitor/audit | L2-04、ENGINE-01/02、MCP-07、AUDIT-01 | WIRED |
| B11 | P2/P6 exclusion → P7：`crates/ontoforge-engine/src/config.rs:572` store/ACL catalog 一致；`crates/ontoforge-engine/src/trino/mod.rs:144` accessor → `crates/ontoforge-server/src/mcp.rs:157` → edit_catalog → `crates/ontoforge-server/src/mcp_l2.rs:104` policy | execute/explain 在 run_sql 前 validate_l2；metadata `:275` allowed_catalog/resolve_* 大小写不敏感排 edit/system；list_catalogs 枚举后过滤，不要求零上游 | L2-01..03、MCP-07 | WIRED，INT-01 已关闭 |
| B12 | P7 host/OpenAPI → SDK/stdio/dev/install 与 P07.2 CI：`crates/ontoforge-server/src/lib.rs:219` 八类 HTTP；`crates/ontoforge-client/src/lib.rs:129`、`clients/typescript/src/index.ts:177` 实际调用；`crates/ontoforge/src/mcp.rs:40` stdio→HTTP；OpenAPI generator；`ci/release-smoke.ps1:155` 完整 smoke；`.github/workflows/nightly.yml:45` → MinIO runner | SDK 解码结构化错误/大整数；dev reset 拒绝旧 token；runner 对 discovery/执行数/S3/restart/cleanup 有硬门；发布上传未执行 | API-01..05、MCP-01/06、DX-01/02/04、ENGINE-05 | WIRED |

## API Coverage 与 Auth Protection

实际 router 是真相源，不以 REST_ROUTE_PATHS 常量或 OpenAPI 单独证明路由已安装。以下八类路径均在 `crates/ontoforge-server/src/lib.rs:219` 起注册。

| 路径 | Rust 消费者，crates/ontoforge-client/src/lib.rs | TS 消费者，clients/typescript/src/index.ts:177 起 | 授权 |
|---|---|---|---|
| GET /health | health，:157 | health | 有意公开非敏感健康摘要 |
| GET /api/ontology | metadata，:129 | metadata | context + scope 过滤 |
| GET /api/objects/{object_type}/{pk} | get_object，:132 | get | context + QueryService |
| POST /api/query | query，:144 | query | context + QueryService |
| POST /api/aggregate | aggregate，:147 | aggregate | context + QueryService |
| POST /api/actions/{action} | execute_action/validate_action，:160 | action/validate | context + ActionService |
| GET /api/commands/{command_id} | get_command_result，:220 | commandResult | context + 当前 action grant + 主体隔离 |
| GET /api/objects/{object_type}/{pk}/history | history，:195 | history | context + 当前行/列遮蔽 |

**Consumed：8/8；orphaned：0/8。** `/mcp` 是额外协议挂载，不混入八类 REST 路径计数。stdio bridge 带 token 访问此挂载，不另装无鉴权后门。

**敏感区：7 类 REST 操作 + 1 个 MCP 挂载均接入鉴权；未发现预期受保护却无 auth 的区域。** MCP 在 `crates/ontoforge-server/src/context.rs:484` 验证 token/session owner；list/call 再经 request_context；`crates/ontoforge-server/src/mcp.rs:573` 先查当前授权 catalog，不能绕过 list_tools 直调未授予工具。catalog 仅列 core/full/sql:raw 与授权 action，不列 reload、角色/定义修改或 L3 工具。OpenAPI 是生成文档，不臆造 `/openapi` HTTP 路由。

## E2E Flows

WIRED 表示入口、实际调用、状态/存储、输出与列明失败分支的静态路径完整。**本轮动态完成数为 0，不可将 9 WIRED 改写成 9 次真实 E2E 成功。**

| ID | 完整静态路径 | 错误/空态与已有测试入口 | 判定 |
|---|---|---|---|
| F01 | CLI YAML/JSON → validation/lint → reload → scope apply → token → snapshot → scoped read；附显式 source cardinality 分支 | 无效定义不触 store，无 scope 默认不可见，reload 复核 role；`crates/ontoforge/src/scope.rs:121` / `:198` → scope_apply；online `:296` scanner → warning/exit code | WIRED，B01/B02/B03 |
| F02 | Rust/TS → REST → auth/snapshot → query/source/edit overlay → mask/version/audit → DTO → decode | 空/无权/坏 cursor/source drift/截断有分支；`crates/ontoforge-query/src/cursor.rs:235` 绑定主体/定义；`crates/ontoforge-server/tests/source_drift.rs:237` / `:379` 拒绝与完整 patch 断言，未重跑 | WIRED，B03/B04/B05/B09/B12 |
| F03 | REST/MCP action → 参数/criteria/写后授权 → CAS/fence → edits/log/command/outbox → receipt → 下一读取/history | stale、跨主体重放、异参、validate-only、批量 rollback；`crates/ontoforge-server/tests/postgres_http_it.rs:257` durable rows、`:75` race 同 receipt，未重跑 | WIRED，B06/B07/B08 |
| F04 | stdio → authenticated /mcp → 当前 toolset/action schema → 共享 Query/Action/history → structuredContent | session 错误/未授权直调拒绝；`crates/ontoforge-server/src/mcp_actions.rs` schema/decoder；`ci/release-smoke.ps1:184` 后含桥接与 EOF 断言，未重跑 | WIRED，B03/B06/B07/B12 |
| F05 | L2 proof/周期复验 → 六工具 allow-list/AST → PrintedSql → dedicated Trino → audit → MCP | 未配置/失效/取消拒绝或终止；执行时重查 proof；audit 不健康不发业务请求；`crates/ontoforge-server/tests/mcp_l2_defaults.rs:860` expired proof 零发送断言，未重跑 | WIRED，B05/B10/B11 |
| F06 | 自定义 store/ACL edit catalog → trusted accessor → MCP policy → SQL/metadata 本地排除或结果过滤 | `crates/ontoforge-server/tests/mcp_l2_defaults.rs:382` / `:387` / `:458` 含自定义/default/正常 catalog；`:445` 上游/业务计数。未运行 fixture，不声称新 GREEN 或实际泄漏 | WIRED，B11；INT-01 已关闭 |
| F07 | 解包二进制 → serve --dev → embedded fixture → SQLite store/engine/token → query → Action → history → restart 保留 → reset 重建 → stdio | `crates/ontoforge/src/dev.rs:122` reload/scope 初始化；smoke `:155` 后比较值/receipt/history，reset 后旧 token/command 拒绝；不包含真实发布上传 | WIRED，B12 |
| F08 | Action commit → outbox → 独立 webhook-worker → HTTP event_id → mark 或 retry → lease 恢复 | 不重读当前对象、不跟随 redirect、错误退避/稳定 event_id；生产管理器/接收端 dedup 待联调，at-least-once 不等于 exactly-once | WIRED（机制），B08；W03 |
| F09 | shutdown → stop admission → Action 有界等待 → cancel reads → session/read-task 收束 → audit drain/monitor join → exit | `crates/ontoforge-server/src/lib.rs:249` / `crates/ontoforge-server/src/lifecycle.rs:44` 同 deadline，失败返回错误；`crates/ontoforge-server/tests/source_drift.rs:308` / `:329` 启动取消断言，未重跑 | WIRED，B05/B09/B10/B12 |

### 07.1/07.2 后继合同闭环

这两项纳入 B01/B12，不增加 12 边界或 9 用户流分母。

- **STORE-05/source：** online.rs:296 由 args.check_cardinality 实际调用 scanner；仅扫描当前 profile 下通过 binding/describe 的 source OneToOne to-object，使用映射 FK/PK 和 printer 的 COUNT(DISTINCT ...) > 1，返回 bounded samples/more；整轮 deadline 与清理接通。不以对象 PK 检查或 edit-side constraint 冒充 source 报告。已读 `07.1-evidence/plan03-real-trino-final.json` 与 `.log`，记录一个精确真实测试退出 0；这是 2026-11-04 旧运行，不是本轮复验。`07.1-VERIFICATION.md` 为 18/18 passed。
- **ENGINE-05/MinIO：** nightly.yml:45 调共享 runner；`ci/iceberg-services.sh:203` 精确发现/执行 Iceberg test，`:214` 要求 source/copied 均 29，`:216` 要求 s3://warehouse/，`:239` 起逐 target 强制非零且 failed/ignored 为 0，`:500` restart 后复查，finally cleanup 保留失败证据。不是将 local warehouse 改名为 MinIO。已读 07.2-RUNTIME-EVIDENCE、07.2-CI-VERIFICATION、归档索引及 `target/phase072-minio/ci/run-41833307824-verified.json`：既有记录为 483/484 各 49 corpus、前后 29/29、S3/restart/cleanup 成立，notification skipped；`07.2-VERIFICATION.md` 为 9/9 passed。未在线重查或重跑，不证明 SQL Server 私有 fixture。

## Detailed Findings

### BLOCKER / Missing Connections / Broken Flows

本次列明范围内为 **0**。12 边界与 9 流都有实际调用，不因动态未重跑而伪造 BROKEN，也不因静态 WIRED 而伪造动态 passed。

### INT-01 处置与替代实现

**已修复，当前 B11/F06 WIRED。** call_l2 不再丢弃 deployment edit catalog，缺失 trusted catalog 时 fail-closed；SQL AST 与 metadata 使用同一注入值。默认值、显式参数、qualified table 的最终目标均经 policy/resolve。list_catalogs 的合同是枚举后过滤，不是零上游。

`L2-CATALOG-FIX.md`、`PHASE-07-REVERIFY.md` 保存历史 RED/GREEN。本轮只以亲读代码/测试断言作静态证据，不把历史 GREEN 冒充当前运行。`rg 'L2Service\s*\{' crates` 只在 query tests 找到构造，生产 MCP 用 McpL2Tools。不是整个 L2 功能 orphan，也不能拿 L2Service 测试代替 adapter 覆盖。

### WARNING 与剩余验收

| ID | 需求/范围 | 连接存在但仍须保留的限制 | 责任/依据 |
|---|---|---|---|
| W01 | SCHEMA-07、Phase01 | 自动导出/编辑器配置不等于人工体验；human_needed 保留，不重开已接受体验或改 passed | Phase01 VERIFICATION，主 auditor |
| W02 | ENGINE-05、Phase02 | MinIO 后继补证已闭合；私有 SQL Server fixture/来源 judgment 未证；ENGINE-05 Partial、Phase02 gaps_found | Phase02/07.2 VERIFICATION |
| W03 | ACTION-11 | outbox/worker/HTTP 机制 WIRED；生产管理器重启、接收端 503/dedup/恢复未由本轮证明 | Phase05 deferred-items，部署/动态 owner |
| W04 | QUERY-10、AUDIT-01 | 静态预算不证明隔离集群 p95；Windows audit pending 墙钟波动已有记录，单线程 CI 不等于根除 | Phase04 性能证据、Phase06 deferred-items |
| W05 | DX-04 | 三平台安装 smoke 与真实发布不同；release upload/npm 发布不在本轮执行范围 | release.yml:91 事件门、07.2 CI upload skipped |
| W06 | 所有动态结果 | workspace、PG/Trino/MinIO、SDK、shutdown 未在本轮重跑；历史 ignored/skipped 不计通过 | 独立复验 owner，不靠刷新 fingerprint 关闭 |
| W07 | L2-01..04、MCP-07 | L2Service/McpL2Tools 双实现，未来只修前者或只测 query 层会漏生产 adapter 合同 | B10/B11，生产/测试消费者检索 |

没有将 env/feature 门控的 ignored 测试称 orphan，没有将缺 private fixture 当产品已实证失败。无新增灰区需自行裁决；若要改变历史 judgment 或发布范围，返回 `STATUS: NEEDS_CLARIFICATION`，不由本报告扩权。

## Requirements Integration Map

覆盖 REQUIREMENTS 中 79 个 v1 ID。**WIRED 仅表示涉及的集成路径追通，不代表每条验收断言或当前动态结果通过。** N/A 表示该项验收本身无强制跨阶段 touchpoint，不是孤儿代码，其工具可以被其他阶段复用。验收限制单列，不把 ENGINE-05 WIRED 解释为 Complete。

| Requirement | Integration Path | Status | 验收限制/Issue |
|---|---|---|---|
| SCHEMA-01 | P1 load → B01 CLI → B02 active IR | WIRED | 本轮静态 |
| SCHEMA-02 | P1 object IR → B04/B07 typed read/history | WIRED | 本轮静态 |
| SCHEMA-03 | P1 property types → P2 printer/decode → B04/B06 | WIRED | 本轮静态 |
| SCHEMA-04 | P1 link IR → B05 traversal/B06 write/B01 scanner | WIRED | 本轮静态 |
| SCHEMA-05 | P1 action IR/CEL → B06 → P7 dynamic schema | WIRED | 本轮静态 |
| SCHEMA-06 | P1 SourceMap/diagnostics → B01/B02 CLI result | WIRED | 本轮静态 |
| SCHEMA-07 | P1 schema export → repository editor association | N/A | W01，human_needed 保留 |
| LINT-01 | P1 lint → B02 reload admission | WIRED | 本轮静态 |
| LINT-02 | P1 naming/structure warning → lint diagnostics | N/A | 单阶段规则验收，未重跑 |
| LINT-03 | P1 diagnostics → CLI human/JSON renderer | N/A | 单阶段输出验收，未重跑 |
| LINT-04 | P1 config → lint severity/disable | N/A | 单阶段配置验收，未重跑 |
| BIND-01 | P1 SourceBinding/profile → B01 → B04 source | WIRED | 本轮静态 |
| BIND-02 | P1 types → B01 online/B02 reload | WIRED | 本轮静态 |
| BIND-03 | B01 describe/PK → B09 drift → source guard | WIRED | 本轮静态 |
| ENGINE-01 | P2 Trino → B04/B06 source 与 B10 dedicated L2 | WIRED | 身份/取消实跑归动态 owner |
| ENGINE-02 | P2 status/drift → B09/B10 → health/admission | WIRED | 本轮静态 |
| ENGINE-03 | P2 printer → B04/B05/B06/B11 engine call | WIRED | proptest/真实注入未重跑 |
| ENGINE-04 | P2 SQLite → B04/B06/B12 dev runtime | WIRED | 本轮静态 |
| ENGINE-05 | P2 targets → B12 P07.2 nightly/MinIO/S3 | WIRED | Partial；SQL Server 未闭环，W02 |
| STORE-01 | P3 EditStore → B04/B06 → SQLite/PG backend | WIRED | conformance 未重跑 |
| STORE-02 | P1/P2 checks → B02 reload/migrate → B03 snapshot | WIRED | 本轮静态 |
| STORE-03 | B06 transaction → log/history/outbox → B07/B08 | WIRED | 本轮静态 |
| STORE-04 | B06 log → B07 masked history → SDK/MCP | WIRED | 本轮静态 |
| STORE-05 | B06 edit constraints + B01 P07.1 source scanner | WIRED | source 报告 opt-in，不阻塞读 |
| STORE-06 | CLI import → storage import → system:import log | WIRED | 已读 import.rs；ETL 非 L0 |
| STORE-07 | P3 patch/tombstone/release → B04/B06 | WIRED | 本轮静态 |
| SCOPE-01 | CLI scope apply/OIDC groups → B03 snapshot | WIRED | 本轮静态 |
| SCOPE-02 | P3 compile_scope → B03/B04/B06/P7 tool catalog | WIRED | 本轮静态 |
| SCOPE-03 | P3 merged row/column grants → B04/B05/B06 | WIRED | 本轮静态 |
| SCOPE-04 | P3 evaluate/lower → P4 query compiler/row mask | WIRED | 差分测试未重跑 |
| SCOPE-05 | B03 scope → B04 invisible/false → safe result | WIRED | 本轮静态 |
| SCOPE-06 | B02 reload + scope apply → role/criteria coverage | WIRED | 本轮静态 |
| SCOPE-07 | B02/B03 fixed definition → B06 fence；MCP 无管理工具 | WIRED | 本轮静态 |
| AUTH-01 | Authenticator/OIDC/JWKS → B03 REST/MCP principal | WIRED | 未请求 IdP/JWKS |
| AUTH-02 | dev-token/DevVerifier → B03/B12 dev auth | WIRED | 本轮静态 |
| QUERY-01 | B04 pk → source/edit → REST/MCP/SDK | WIRED | 本轮静态 |
| QUERY-02 | P7 QueryRequest → P4 compile/filter → B05 result | WIRED | 本轮静态 |
| QUERY-03 | P7 search set → P4 compile/search → B05 | WIRED | 本轮静态 |
| QUERY-04 | P4 keyset cursor/context → B05 page → SDK | WIRED | 本轮静态 |
| QUERY-05 | P1 link IR → P4 searchAround → B05/P7 | WIRED | 本轮静态 |
| QUERY-06 | P7 set AST → P4 set execution → B05 | WIRED | 本轮静态 |
| QUERY-07 | P7 aggregate → P4 aggregate → B05 audit/result | WIRED | 本轮静态 |
| QUERY-08 | P3 overlay → B04/P4 execute → P2 dialects | WIRED | 内存模型/proptest 未重跑 |
| QUERY-09 | B04 mask/version + B05 budget → P7 errors/SDK | WIRED | 本轮静态 |
| QUERY-10 | B04/B05 single-table plans → P2 Trino/PG | WIRED | W04，不证明本轮 p95 |
| ACTION-01 | P7 action input → P5 params → B06 errors | WIRED | 本轮静态 |
| ACTION-02 | P5 rules → B06 guarded transaction | WIRED | 本轮静态 |
| ACTION-03 | P3 scoped view → P5 criteria → B06 guard | WIRED | 本轮静态 |
| ACTION-04 | REST/MCP validateOnly → B06 rollback → DTO | WIRED | 本轮静态 |
| ACTION-05 | P7 items → P5 prepare → B06 commit/rollback | WIRED | 本轮静态 |
| ACTION-06 | B06 edits summary/receipt → P7/SDK | WIRED | 本轮静态 |
| ACTION-07 | P7 ActionService + audited CLI import → P3 writes | WIRED | MCP 无直接写 SQL/管理工具 |
| ACTION-08 | P3 visibility → P5 guard/poststate → B06 | WIRED | 本轮静态 |
| ACTION-09 | B04 _version → P5 authorize → B06 locks/CAS | WIRED | 本轮静态 |
| ACTION-10 | B03 principal/grant → B06 command claim/replay → SDK | WIRED | 本轮静态 |
| ACTION-11 | B06 outbox → B08 worker/HTTP/retry | WIRED | W03，生产联调待验 |
| ACTION-12 | P5 source.rs → P4 read_pk_source → B09 guard | WIRED | 完整 patch 不强制访问 Trino |
| L2-01 | P3 raw grant → B10/B11 → Trino/MCP result | WIRED | 本轮静态 |
| L2-02 | B11 validate_l2 AST → PrintedSql → B10 engine | WIRED | 本轮静态 |
| L2-03 | B11 trusted edit catalog → SQL/metadata fence | WIRED | INT-01 已修；动态回归未重跑 |
| L2-04 | B10 ACL monitor/proof → admission/health | WIRED | 本轮静态 |
| AUDIT-01 | B04/B05/B11 reserve/finish → P3 sink → F09 drain | WIRED | W04/W06 |
| API-01 | B03-B07 services → B12 actual router/SDK | WIRED | 八类路径均有消费者 |
| API-02 | P7 OpenAPI render → docs/openapi.json → TS generator | WIRED | snapshot/generation 未重跑 |
| API-03 | B03-B07 errors → P7 envelope → B12 typed SDK error | WIRED | 本轮静态 |
| API-04 | B12 Rust client → 八类 HTTP → DTO decode | WIRED | 本轮静态 |
| API-05 | B12 OpenAPI TS → raw GET/POST → decode | WIRED | 本轮静态 |
| MCP-01 | CLI stdio transport → B12 HTTP /mcp | WIRED | 本轮静态 |
| MCP-02 | B03 filtered ontology → MCP describe | WIRED | 本轮静态 |
| MCP-03 | P7 tools → B05 QueryService | WIRED | 本轮静态 |
| MCP-04 | P1 action IR → P7 schema/catalog → B06 | WIRED | 本轮静态 |
| MCP-05 | P7 validateOnly → B06 validated outcome | WIRED | 本轮静态 |
| MCP-06 | REST/MCP → B03/B05/B06/B07 shared scope/log | WIRED | 本轮静态 |
| MCP-07 | P3 toolsets → P7 catalog → B10/B11 six tools | WIRED | INT-01 已修；W07 |
| MCP-08 | P7 positive catalog → 无 L3/definition/scope mutation | WIRED | 本轮静态 |
| DX-01 | B12 embedded dev → P2 SQLite/P3 store/auth → server | WIRED | 本轮静态 |
| DX-02 | B12 aviation → F07 query/Action/history | WIRED | 演示未重跑 |
| DX-03 | workspace/CI Windows test + CLI color contracts | N/A | 工具链验收，不由跨 phase trace 证明 |
| DX-04 | B12 package → installed binary smoke → event gate | WIRED | W05，未发布 |

**无独立跨阶段验收 touchpoint 的 5 项：** SCHEMA-07、LINT-02、LINT-03、LINT-04、DX-03。其余 74 项有集成路径，不代表 74 项动态验收全通过。

## 交付验收与 owner 交接

- [x] 亲读当前源码，建立提供/消费图，12 边界逐项 WIRED/BROKEN 结算。
- [x] 检查实际 router、Rust/TS 请求、MCP auth/tool admission，而非只看 exports/路由定义。
- [x] 追踪 9 流到存储/返回，核对列明错误、空态、取消与 rollback。
- [x] INT-01、07.1 scanner、07.2 nightly MinIO 均追到实际消费者。
- [x] 覆盖全部 79 个 requirement ID，分开 wiring/verification，保留 01/02、ENGINE-05 判断。
- [x] 仅写本文件与 evidence/2026-11-05/integration-final-* 新证据，不改共享计划或他人文件。
- [x] 当前源码真实复验已由独立动态 owner 完成，见 FINAL-REVERIFY.md；此处为主 auditor 后续补记，静态 worker 不签动态通过。总体审计另见 SUMMARY.md。
- [ ] SQL Server judgment、生产 webhook、隔离性能/发布边界按 W01-W06 保留，不在此清零。

本次完成的是独立静态集成审计交付，不是 milestone 完成、归档、合并或发布信号。
