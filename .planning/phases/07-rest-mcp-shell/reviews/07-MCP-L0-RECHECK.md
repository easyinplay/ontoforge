---
phase: 07-rest-mcp-shell
reviewed: 2026-11-03T08:17:48Z
depth: standard
review_base: 4f9d159060a6a2ffd825e76dabcdda9c6e9c0c85
review_target: frozen-working-tree-blobs
scope_contracts: [RF-02, RF-04, RF-09, RF-06-L0-Action]
source_blobs:
  crates/ontoforge-server/src/mcp.rs: 83924f6135511d434202a415cc1ec5df2772c2a9
  crates/ontoforge-server/src/mcp_actions.rs: e456febc5da1a9214382c1aaa7c9436a1dc6446a
  crates/ontoforge-server/src/mcp_catalog.rs: 80c643618826d59d03f821b8cbc60b6f63f110e5
  crates/ontoforge-server/src/error.rs: ae051be7713d2456da0c2f76ec60ce4642643b6c
  crates/ontoforge-server/tests/mcp_contract.rs: 6b48b1fcd940d1b0a23ae67c48bf4e05dbcb4610
  crates/ontoforge-server/tests/mcp_actions.rs: e8cc27e5b6444f46051a55dab24a635a197f2a4f
files_reviewed: 6
files_reviewed_list:
  - crates/ontoforge-server/src/mcp.rs
  - crates/ontoforge-server/src/mcp_actions.rs
  - crates/ontoforge-server/src/mcp_catalog.rs
  - crates/ontoforge-server/src/error.rs
  - crates/ontoforge-server/tests/mcp_contract.rs
  - crates/ontoforge-server/tests/mcp_actions.rs
findings:
  critical: 0
  warning: 0
  info: 0
  total: 0
status: clean
remaining_contracts: [RF-05, RF-07, RF-06-L2]
phase_acceptance: pending
---

# Phase 7 MCP L0 Recheck

冻结的 MCP L0 tranche 定向复签为 clean，无新增未解决 BLOCKER 或 WARNING。结论覆盖 RF-02/04/09，以及 RF-06 已认证工具调用中的 L0/Action 领域错误路径，不能推导 RF-06 整体或 Phase 7 完成。

检查六文件变更 hunk、新测试断言与必要 caller/callee：固定 RequestContext、Action 参数校验、Query audited wrappers、静态错误码、当前 REST metadata 和历史读取合同。没有逐读整个仓库。未运行 Cargo、未修改实现或共享 planning，只新增本报告。

## Narrative Findings (AI reviewer)

无新增未解决发现。

## Static Evidence

| 合同 | 核实内容 |
| --- | --- |
| RF-02 | mcp.rs:196-208 的 links 要求两端对象类型均在当前 scope；字段与 REST links 相同。engineStatus 只输出 available/unavailable，丢弃底层 EngineStatus.reason。真实 MCP/REST 对照同时覆盖引擎可用/不可用，并排除 Aircraft 和 secret_token。 |
| RF-04 | catalog 和 call_tool 均将同一请求固定 ontology 传给 ActionToolSchema。mcp_actions.rs:32-53 从目标真实 PK PropertyDef 生成 reference schema，validator 保存该定义；:100-132 使用既有 PropertyValue::from_json/check_literal。:136-161 让 Float/Double 数组使用 number items，Decimal 延续领域校验支持 string/number。没有再次读取实时 ontology。 |
| RF-09 | mcp.rs:237-253 保留 PK QueryPage.truncated 并返回 COST_EXCEEDED；真正不存在仍是 NOT_FOUND。history 前置 PK 同样在 :407 拒绝截断，未把超预算伪装为空对象。 |
| RF-06 L0 | Query/list/aggregate/history 使用 audited_observed 同步更新 request-local trace；错误仅取穷举静态领域码或安全常量，经 error.rs:8-13 输出 code/apiName/request_id。HistoryError::Store 丢弃底层消息；不存在直接 Display 入正文的已审 L0 领域路径。 |
| RF-06 Action | 参数校验详情不直接输出；ActionService 错误经 action_code 静态映射，spawn/receiver 失败亦用安全常量。独立 Action 生命周期、接收后不重放及取消所有权路径没有被该错误映射变更破坏。 |

## IDs And Explicit Remaining Scope

已 reserve 的 Query/list/aggregate/history 失败与 L0 请求取消使用该 guard 的真实审计 ID。未 reserve 的早期错误，以及 Action 错误，使用服务端生成的 transport ID；不宣称它们有同 ID 的持久读取审计。历史读取仍关联前置 query.pk 的审计状态，沿用 RF-12 已记录边界。

RF-06 的 L2 分支仍保留旧字符串错误路径，必须继续修复。认证 RequestContext 建立前的常量错误仍沿原路径；本报告不宣称全部 MCP transport 错误已统一成新 envelope。RF-05 平铺控制字段冲突、RF-07 历史分页/预算仍待其各自 tranche；本次没有把未实施工作记为完成，也不把它们当成该冻结增量的新发现。mcp_l2_defaults.rs 的准备中 RED 测试不在冻结范围，RF-03 也不在范围。

## Dynamic Evidence

独立读取 owner 的 target/phase7-mcp-review-* 日志并核读相应断言，未复跑：

- red.log：mcp_actions 4 passed/1 failed，mcp_contract 5 passed/5 failed，共 6 个实际行为失败；分别暴露浮点数组类型、非字符串 PK、describe 字段缺失、PK 截断、NOT_FOUND envelope 与 store 错误问题。
- final-tests.log：mcp_actions 6/6、mcp_contract 10/10、mcp_transport 11/11，总计 27 passed、0 failed、0 ignored。新增 typed Action 的真实 HTTP validateOnly 接受 integer/boolean PK 与 Float 数组，拒绝坏数组；不是提交写入测试。
- store 故障用例实际 DROP SQLite 对象表/历史表并检查无物理名、SQL 泄漏；失败响应 ID 与 shutdown 后真实 of_read_audit 对账。Action 目标表故障只断言安全 envelope 和非空 transport ID，不夸大为 durable audit 对账。
- clippy.log 包含 Finished dev profile，1.12s；退出码 0 来自 owner 报告，日志本身未保存 EXIT 标记。

上述 source/store 测试使用真实 HTTP MCP 会话、SQLite 与可控 Engine；不是 PostgreSQL/Trino 全链路或整个 Phase 的验收证明。
