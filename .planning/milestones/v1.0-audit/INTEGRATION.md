---
milestone: v1.0
audit_date: 2026-11-04
auditor: gsd-integration-checker
status: gaps_found
inspected_head: fa81ca0271fb1d620e8217824a0bf2619e3235ed
dynamic_evidence_source: adc02278b5cc55111def9a30a1c3b778485e8279
method: static-call-trace-and-existing-local-evidence
---

# Phase 1–7 集成审计

## 结论与范围

**INTEGRATION_STATUS: GAPS_FOUND**。本次明确枚举的 12 个集成边界中，11 个 WIRED，1 个 BROKEN；9 条用户流中，8 条在列明的合同与证据边界内 WIRED，1 条自定义 edit catalog 隔离流 BROKEN。计数是下表的审计单元数，不是 phase 数、export 总数或独立测试数。不能引用为「79/79 需求全部当前通过」。逐需求三源对账由主 milestone auditor 负责。

本轮未运行 Cargo、未启动服务、未改实现或旧阶段报告。动态证据为亲读的已有本地日志与 CI JSON，不代表本轮重新执行，也未在线重新查询 CI。源码只读审计后按 orchestrator 授权新建本报告。

已读取 PROJECT、ROADMAP、REQUIREMENTS、STATE；遍历各 SUMMARY 的 provides/requires/key-files/requirements 元数据；核对七个 VERIFICATION/VALIDATION 的状态与接线证据；读取 Phase 7 UAT、07-14 SUMMARY、Phase 5/6 deferred-items。项目无 `.codegraph/`、`.codex/skills/` 或 `.agents/skills/`，`agent_skills` 配置为空。适用记录规则来自 `gsd-audit-milestone`；未重新启动其顶层 orchestration。

## 当前源码与旧证据的差异

- `git diff --name-only adc0227 HEAD`：25 个文件，全在 `.planning/`。当前 tracked production code、CI、SDK 与最终动态证据源码一致。工作区存在其他 worker 的 Phase 1 VALIDATION 和新增测试等修改，本报告不把这些未提交项归入 `adc0227` 动态证据。
- Phase 1–6 指纹不新鲜说明旧 covered_files 已被下游改变，不能自动推导功能回归。后继实际变化包括 `request_snapshot`、`history_with_snapshot`、Action 固定定义入口、L2 typed EXPLAIN/监控，并已由 Phase 7 current-source suites 覆盖其相关路径。
- 例如相对 Phase 6 历史 `106a5aa`，`l2_health.rs` 为 +174/-49、`query/src/l2.rs` 为 +29/-11、`storage/src/history.rs` 为 +13/-2。变化属于真实实现演进，不能只刷新 fingerprint 就宣称旧测试重新通过。
- 本报告的 INT-01 在 `adc0227` 的当前生产路径已存在，属于 Phase 6→7 合同遗漏；不是 `adc0227` 之后的文档提交引入的回归。

## 集成边界清单

| ID | 提供方 → 消费方与实际调用 | 需求 | 状态 |
|---|---|---|---|
| B01 | P1 schema/expr → P2 CLI：`ontoforge/src/pipeline.rs:29` load_dir→check_profile→check_ontology；`online.rs:55,201,248` 将选定 IR/binding 交给 describe/兼容与 PK 检查 | SCHEMA-01..06、BIND-01..03 | WIRED |
| B02 | P1/P2 validation/lint → P3 reload：`ontoforge/src/reload.rs:46` lint 后才 open/reload，`storage/src/reload.rs:293,401,444` role 复核→DDL→原子切定义；`storage/src/migrate.rs:406,425,429` 同步复核与切换 | LINT-01、BIND-02、STORE-02、SCOPE-06 | WIRED |
| B03 | P3 auth/scope/store → P7 请求：`server/src/context.rs:401` build 中 authenticate→store.request_snapshot→compile_scope→for_principal；`storage/src/request_snapshot.rs:23` 同锁读取定义与 scope 后结束事务 | AUTH-01/02、SCOPE-01..07、API-01、MCP-06 | WIRED |
| B04 | P2 Engine + P3 edit/scope → P4 L0：`query/src/lib.rs:309` execute_pk 读取 edit、判 tombstone、按 patch 完备性决定 source、overlay→grant.evaluate→select masking→version；`:513,520` printer→check_source_target→engine | ENGINE-01..04、STORE-07、SCOPE-03/05、QUERY-01/08/09 | WIRED |
| B05 | P4 list/aggregate → P6 audit → P7 transport：`server/src/rest.rs:279,361` 调用 audited list/aggregate；`mcp.rs:357,393` 相同 QueryService；`query/src/execute.rs:527,545` 与 `aggregate.rs:912,933` reserve/finish，积压失败返回 AuditUnavailable | QUERY-02..07、AUDIT-01、API-01、MCP-03/06 | WIRED |
| B06 | P4 _version/P3 storage → P5 Action → P7：`server/src/rest.rs:457,481` 与 `mcp.rs:633,645` 调 execute_with_snapshot；`action/src/lib.rs:336,386` prepared read/edits→commit_action_guarded；`storage/src/commit.rs:617..678` definition fence、锁、CAS、guard、validate-only rollback，后续 receipt/log/outbox 同事务 | ACTION-01..10/12、STORE-03/05、QUERY-09、SCOPE-07 | WIRED |
| B07 | P5 receipt/log → P3 history → P7 display：`server/src/rest.rs:history`、`mcp.rs:480`→`history_page.rs:8,17,28`→`storage/src/history.rs:120,224` 固定定义/当前 scope 遮蔽；SDK decode 返回 history/receipt | STORE-04、ACTION-06/10、API-01/04/05、MCP-06 | WIRED |
| B08 | P5 outbox → CLI worker → HTTP receiver：`storage/src/commit.rs:1081` 写 outbox；`ontoforge/src/webhook_worker.rs:54` poll_at；`action/src/outbox.rs:123,149,154` lease/send/mark or retry；发送 event_id header 与持久 payload | ACTION-11、STORE-03 | WIRED；实际生产接收端联调仍 deferred |
| B09 | P2 drift 状态 → P7 启动/周期 → P4/P5 admission/health：`server/src/lib.rs:167` start_drift；`drift.rs:155` 首轮后 30 秒周期；`context.rs:349` check_source_target；`health.rs:16` 取相同定义的 drift_view | BIND-03、ENGINE-02、QUERY-01、ACTION-12、API-01 | WIRED |
| B10 | P6 L2 ACL monitor → P7 生命周期/执行/health：`ontoforge/src/serve.rs:177,211` start monitor→set_l2_engine/install；`server/src/mcp.rs:150` 仅取 Enabled dedicated engine；`engine/src/trino/mod.rs:456` 持读锁再检查 proof；`health.rs:16` 同实例状态 | L2-04、ENGINE-02、MCP-07 | WIRED |
| B11 | P6 deployment edit-catalog policy → P7 MCP L2 adapter：Phase 6 `query/src/l2.rs:447` 使用 acl.edit.catalog，但 MCP `server/src/mcp_l2.rs:104,274` 写死 editstore，详 INT-01 | L2-03；关联 MCP-07 | BROKEN / BLOCKER |
| B12 | P7 host/OpenAPI → Rust/TS clients、stdio 和实际安装包：`server/src/lib.rs:242` route registration；`client/src/lib.rs:129..224`、`clients/typescript/src/index.ts:177..211` 实际 HTTP 调用；`ontoforge/src/mcp.rs:40` stdio→HTTP /mcp；`ci/release-smoke.ps1:155..200` 开包后 query→Action→history→restart/reset→stdio | API-01..05、MCP-01/06、DX-01/02/04 | WIRED |

## API 消费与认证

8 个 REST 路由组（含 `/health`）均有 Rust 与 TS 实际客户端调用：ontology、object、query、aggregate、action、command-result、history、health。`validate` 复用 action endpoint；`/mcp` 有 stdio bridge 与协议测试消费者。没有发现本清单中的 route orphan。L2 属 MCP 六工具出口，未要求额外 REST SQL endpoint。

7 个业务 REST 路由组均进入 `context_from_headers`；MCP router 使用 `authenticate_mcp`，tool handler 每次重新构造当前 scope，不只信任 session 初始化授权。`/health` 是公开且脱敏的状态接口。此计数只覆盖已注册的业务路由组，不等于逐 handler 安全证明。

## E2E 用户流清单

| ID | 完整路径与错误/空状态 | 亲核证据 | 状态 |
|---|---|---|---|
| F01 | YAML/JSON→校验/lint→reload→scope apply→发 token→认证请求；无效定义不触及 store、空 scope 默认不可见 | B01/B02/B03；CLI `lib.rs:32..38` 确认命令实际 dispatch；dev `src/dev.rs:122,140,160` 初始化同一套 reload/scope_store | WIRED（本轮静态追踪） |
| F02 | SDK 请求→REST auth/snapshot→L0→edit/source→scope masking→version→decode；空行/越权 404、source drift/失败结构化错误、分页/aggregate 截断 | B03/B04/B05/B09；fresh 日志 Rust9、TS17、Trino2；`source_drift.rs:159,237,379` 检查拒绝与完整 patch 不依赖 source | WIRED |
| F03 | REST/MCP Action→参数/criteria/visible columns→CAS/fence→单事务 edits/log/command/outbox→receipt→下一读取与 history；validate-only 零写，stale/replay/race拒绝或返回原 receipt | B06/B07；`postgres_http_it.rs:37,75,130,257` 断言持久行；fresh PG4；native smoke query→Action→history | WIRED |
| F04 | stdio→authenticated HTTP MCP→当前 toolset/action schema→共享 Query/Action/history→协议结果；无权工具直调拒绝、token/session错误、EOF退出 | B03/B06/B07/B12；`mcp.rs:514,554,570,633`；Phase7 UAT9 与 bridge logs；native stdio smoke | WIRED |
| F05 | L2 identity→启动/周期 proof→六工具 allow-list/AST→重序列化 SQL→Trino→audit→MCP；proof失效拒绝、health降级、恢复重开 | B05/B10，默认 editstore 配置；fresh ACL6+health2；`mcp_l2_defaults.rs:597` expiry/recovery，`:817` cancellation/shutdown | WIRED，受 F06 限制 |
| F06 | 自定义 edit catalog→deployment exclusion policy→MCP execute/explain/discovery→本地拒绝（应不发送上游） | B11；INT-01 源码控制流证实 policy 丢失；现有 fixture只覆盖 editstore | BROKEN / BLOCKER |
| F07 | dev单二进制→嵌入 ontology/fixtures→SQLite store/engine/token→query/Action/history→restart保留→reset重建 | B12；`ci/release-smoke.ps1:155..200`；release JSON 3 native+aggregate success | WIRED；发布事件上传仅静态检查 |
| F08 | Action commit→outbox→独立 webhook-worker→HTTP event_id→2xx标记或503重试→过期lease恢复 | B08；现有 CI JSON integration 的 Action atomicity/races/idempotency/webhook step success；生产部署实接收端未验证 | WIRED（机制）；生产联调 WARNING |
| F09 | 服务关闭→停止 admission→Action有界完成→读取消→monitor与audit排空→进程退出 | `server/src/lib.rs:290` 后 serve/shutdown顺序、`lifecycle.rs`；fresh exit0，native cleanup；`source_drift.rs:308,329` 覆盖部分启动失败 | WIRED |

动态计数独立核对：`target/phase7-final-adc0227-verify.log` 中 PG4、drift2、snapshot1、Trino2、health2、ACL6、Rust9、TS17，总计43，fail0，末行 `fresh_verify_exit=0`；`evidence/commit.txt` 为 adc0227。CI JSON 为同 SHA 的四个必需 job success；release JSON 为三个 native+aggregate success、publish skipped。上述有重叠，不能加总成唯一测试总数，也不能覆盖未存在的 F06 用例。

## INT-01：部署 edit catalog 未接入 MCP L2 policy

- **分类：BLOCKER。** `L2-03` 的「硬排除 edit store catalog」跨 Phase 6→7 断链。`MCP-07` 的六工具 allow-list 合同受影响。`MCP-06` 要求的是 REST/MCP 公共核心授权与 log，已证明的缺口不直接扩大为该需求整体失败。
- **起点与断点：** `config.rs:572..580` 允许并核验 `trino.l2_acl.edit.catalog == store.trino.catalog`，名称不是固定 editstore。Phase6 `query/src/l2.rs:441..451` 取可信配置；生产 MCP `server/src/mcp.rs:154..166` 改构造另一 adapter。`mcp_l2.rs:104` policy和`:274..290` metadata filtering均只有字符串 editstore，没有部署 catalog 输入字段。
- **可达条件：** 配置 `store.trino.catalog = "private_edits"`，ACL edit probe同名；一个 role 有 SqlRaw toolset/Raw level，并误将 `private_edits` 或其某 schema 加入 allow-list。MCP `execute_query("SELECT * FROM private_edits.public.some_table")` / typed explain 在本地 AST gate获准；schema/table/columns discovery也不由本地排除此catalog。标准部署可只返回上游拒绝，但「硬排除」已不成立。
- **下层不能补该连接：** `engine/src/printer/l2.rs:454` 只比较传入 policy.edit_catalog；`engine/src/trino/mod.rs:456..471` 仅 proof Enabled 后发送SQL。`l2_health.rs:274,295` 对配置的一张 edit relation做 PERMISSION_DENIED负探针，既未把 catalog传回adapter，也未在每次 query重做catalog AST fence。若Trino ACL确实拒绝整个catalog，仍会阻止数据读取。不能据本次静态审计声称已发生数据泄露；若配置仅拒绝探针表、其他 edit tables可读，现有证明与错误allow-list组合才会进一步读到数据。
- **现有测试覆盖空白：** `server/tests/mcp_l2_defaults.rs:87` 的 attestation fixture固定editstore。`query/tests/l2_execute.rs` 验证L2Service本身不代表生产MCP消费它。`rg 'L2Service\s*\{' crates` 只在 query tests发现实例，生产 MCP不调用它。这是替代实现遗失合同，不因该库仅有测试consumer就判全部L2 orphan。
- **最小复现计划（未运行）：** 在受控engine/attested fake fixture改edit catalog为private_edits，probe表拒绝、另一edit表可读，给role该catalog的allow-list；调用execute/explain/get_table_schema/list_schemas/list_tables并断言业务statement/metadata calls为0、返回本地拒绝；list_catalogs仍需要上游枚举，但输出必须剔除private_edits。分别验证catalog级与schema级 grants、大小写、qualified table和可信defaults。先建立RED，再修复；不需要真实敏感数据。

## 最小修复设计建议（未实施）

1. 优先让 `TrinoEngine` 暴露只读 `l2_edit_catalog(&self) -> Option<&str>`，从私有 `l2_health.config.edit.catalog` 返回，和 proof 使用同一个配置来源。MCP `call_l2` 拿到已安装 engine 后读取该值；缺失时 fail-closed，不能退回editstore。
2. `McpL2Tools` 增加可信 `edit_catalog: &'a str`，`gate`构造 `L2Policy` 与 `allowed_catalog` 均消费此字段。这比在ServerState另存一份易漂移配置小；若不宜增加engine accessor，可在serve install时传不可分割的 engine+policy capability，不能由请求参数/role/MCP defaults提供该值。
3. SQLite/dev与无Trino路径不安装L2 engine，生产 `call_l2` 已拒绝，因此无需虚构默认edit catalog或让SQLite自动开启L2。直接构造adapter的unit fixture应显式传editstore或自定义值，作为测试拥有的可信配置。
4. 可直接复用现有 `L2Policy` 和 `validate_l2`。Phase6 `L2Service::run_checked` 的policy构造私有且与execution/audit绑定，不能直接作为helper调用；整套改用L2Service会同时改变新defaults/typed schema/资源与audit边界，非最小修改。将排catalog逻辑提到L2Policy方法可减少两处重复，但应限于本次合同修复。

## 非阻塞 WARNING 与 deferred

| 项目 | 需求/当前实现位置 | 判断 |
|---|---|---|
| Phase1/2/3/5 VALIDATION仍draft（Phase1当前正由其他worker处理） | DX-03、SCHEMA/LINT、BIND/ENGINE、STORE/SCOPE/AUTH、ACTION；实现入口如 pipeline.rs:29、storage/reload.rs:142、action/lib.rs:242 | 过程/覆盖矩阵债务，不能直接判行为失败或刷为通过 |
| 旧Phase1 VERIFICATION frontmatter passed/10-of-11，正文仍human_needed | SCHEMA-07；schema export/editor配置；当前CLI导出逻辑为 ontoforge/src/lib.rs 的 Schema分支 | 历史报告内部时点矛盾，不能拿frontmatter替代真实编辑器验收。不是已证明的跨Phase断链 |
| 旧SUMMARY requirements-completed为空/缺失 | 多个Phase3及Phase7 SUMMARY；实际连接证据见B02/B03/B06/B12 | 追踪元数据缺口，非requirement orphan。按主auditor三源对账补齐映射，不能反推功能未交付 |
| 生产webhook服务管理器、接收端去重/503/重启未联调 | ACTION-11；action/outbox.rs:123、ontoforge/webhook_worker.rs:54；Phase5 deferred-items | 当前机制连接完整；生产验收债务保留 |
| Windows慢机audit deadline波动 | AUDIT-01、QUERY-09；storage/read_audit.rs:202、query/tests/security.rs 的cap_counts_final_masked_row_and_version | 现有单线程CI成功不能保证墙钟波动消失；Phase6 deferred记录仍适用 |
| 隔离集群性能、SQL Server私有fixture、nightly Iceberg不由本轮重跑证明 | QUERY-10、ENGINE-05；query execute/engine路径及CI nightly | 历史证据边界保留，不把普通CI的benchmark smoke视为完整性能验收 |
| GitHub Release真实上传/npm发布未执行 | DX-04；.github/workflows/release.yml、ci/release-package.ps1 | 三native安装门通过，发布事件路径未实际触发；为授权边界，不是已观察实现bug |

## 未宣称覆盖的部分

- 没有穷举每个public export，故不提供「全部exports connected/orphaned」的虚假总数；12个边界覆盖本次可验证的主要跨阶段合同。`REST_ROUTE_PATHS` 常量只列4条但实际router注册8组，此常量不是API存在性的真相源，不据此判丢路由。
- 不将所有79项都判WIRED；SCHEMA-07、LINT-02/03/04、DX-03等有合法的单阶段或工具链内验收，不要求为了统计强造跨Phase连接。QUERY-10与ENGINE-05的环境/性能验收也不能单靠源码调用证明。
- requirements总体orphan/partial表归主auditor；本审计明确必须带回的partial为L2-03（关联MCP-07），当前未证明其他行为orphan。历史draft/stale是另一个维度。

## 建议

先对INT-01做最小RED与修复、复签自定义catalog矩阵及原六工具/ACL/取消/audit回归，再归并milestone结论。保留现有Phase7成功动态证据与历史指纹差异，新增证据应绑定修复后的真实源码；不覆写旧时点报告伪装当时已通过。
