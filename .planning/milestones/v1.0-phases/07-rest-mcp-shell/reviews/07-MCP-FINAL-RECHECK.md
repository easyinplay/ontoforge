---
phase: 07-rest-mcp-shell
reviewed: 2026-11-03T08:56:50Z
depth: standard
review_base: f834d039749e39d0b2fdf3cac6153a3ae30aced1
review_target: frozen-working-tree-blobs
scope_contracts: [RF-06-L2-context-observer, RF-07-MCP, RF-03-MCP-service-error-projection]
source_blobs:
  crates/ontoforge-server/src/mcp.rs: a19cad60a3b9ba7b9614cc7054895cfedc6cc355
  crates/ontoforge-server/src/mcp_l2.rs: 8aeab63cd468d6955203455c280dfb701e712b0c
  crates/ontoforge-server/tests/mcp_contract.rs: ca6d37d24f5385945d14f58a3c90ea6ca6b43ae4
  crates/ontoforge-server/tests/mcp_l2_defaults.rs: fbc9221e9615fd4c7cb82d69efac16f08d319e42
dependency_blob:
  crates/ontoforge-server/src/error.rs: 250b188e8599ad5e44da2b3bd6265c84cf3b895b
files_reviewed: 4
files_reviewed_list:
  - crates/ontoforge-server/src/mcp.rs
  - crates/ontoforge-server/src/mcp_l2.rs
  - crates/ontoforge-server/tests/mcp_contract.rs
  - crates/ontoforge-server/tests/mcp_l2_defaults.rs
findings:
  critical: 0
  warning: 0
  info: 0
  total: 0
status: clean
remaining_scope: RF-05
phase_acceptance: pending
---

# Phase 7 Final MCP Recheck

冻结的最终 MCP tranche 复签为 clean，无新增未解决 BLOCKER 或 WARNING。本轮读取四文件增量、相关函数/断言，以及依赖的 RF-03 action_envelope；未将其他 SDK 修复或整个 Phase 纳入结论。

RF-05 参数控制信封仍待用户决定，原平铺字段冲突不由本报告关闭。MCP 当前调用未提交的 RF-03 helper，提交顺序必须先冻结 RF-03，再提交这四文件，不能产生缺 helper 的中间提交。仅新增报告，未修改实现/共享 planning、未运行 Cargo、未提交。

## Narrative Findings (AI reviewer)

无新增未解决发现。

## Static Evidence

| 边界 | 证据与结论 |
| --- | --- |
| L2 安全分类 | mcp_l2.rs:18-77 的 Source 保存 EngineError；code() 穷举 Timeout/Cancelled 和其余类别，只输出稳定码。Audited 保存 reserve 后的 source/finish 错误 ID。mcp.rs:135-211 直接构造 ErrorData，不再将 Invalid/Source 等 Display 送往 wire。 |
| 取消前可见的 ID | mcp_l2.rs:134-137、234-237 在 reserve 成功后、source await 前同步 observer。mcp.rs:540-546 复用 Parts 中共享 RequestTrace；L2 调用与外层 cancel/shutdown 分支读取同一 trace。没有 reserve 时保留 transport ID，不冒充持久 audit ID。 |
| 早期 context 错误 | mcp.rs:35-71 最多读取 4096 bytes；仅允许 UNAUTHENTICATED/INVALID_REQUEST/STORE_UNAVAILABLE/SCOPE_UNAVAILABLE，其余为 INTERNAL_ERROR。保留合法原 x-request-id ULID，否则用可信 fallback；不采用 body request_id、不透传原 body/details。request_context 的缺 Parts/owner 错误同样返回安全结构。 |
| RF-07 接线 | mcp.rs:477-502 调用已复签 read_history_page，复用同一 ctx.active/scope/current_row 与 max_decoded_bytes。结果直接规范化为 HistoryPage，未复制另一套分页算法；预算按页内容而非 MCP 外壳计。 |
| Action 诊断 | mcp.rs:642-651 对 ActionService 错误复用 action_envelope：只有安全参数问题与配置 criteria message 可出现；其他错误无原始 Display。Action 使用 transport ID，不能称为已 reserve 的读取审计 ID。早期 schema 拒绝不新增任意详情。 |

RF-06 L0 的已签部分沿用 07-MCP-L0-RECHECK.md。本轮补齐其 L2 与 context/observer 缺口；RF-03 两 SDK 的普通 JSON 错误兼容修复仍由独立报告负责。

## Dynamic Evidence

独立核读 owner 的以下原始日志及对应断言，执行归属 owner：

- phase7-mcp-next-red-corrected.log：contract 10 passed/3 failed，L2 defaults 4 passed/1 failed，暴露 criteria 消息、历史终页/预算及真实 source sentinel 泄漏。没有采用早期未校正 fixture 的 RED。
- phase7-mcp-early-red.log：context helper 0/2；phase7-mcp-observer-red.log：observer-before-await 0/1，均为已运行后的行为断言失败。
- next-green.log 与 next-adjacent.log：唯一用例为 **lib 10 + contract 13 + L2 defaults 6 + actions 6 + transport 11 = 46 passed**，0 failed。两个日志都运行了 L2 6，不重复计成 52。
- next-clippy.log 包含 Finished dev profile（2.88s）；全 server all-target Clippy exit0 由 owner 报告，日志本身没有 EXIT 标记。

六个 L2 工具通过真实 MCP HTTP 到可控 Trino HTTP stub，分别制造带物理关系、SQL、token sentinel 的故障，断言安全码/工具名及持久 audit ID；invalid format 和 audit 关闭不增加 source 调用。该 stub 不等于真实 Trino 集群。

MCP/REST 历史用例比较相同 scope 的完整页：同 seq 两 edits 不拆、真实下一页、终页 false/null；1024 与 10000 byte 场景比较预算拒绝/成功及审计关联。Action 新增真实 HTTP criteria 失败断言，不夸大为所有早期参数错误都有诊断详情。

## Cancellation Evidence Boundary

取消/关闭测试包含 execute_query、get_table_schema × 请求取消、服务关闭，共四个场景。owner 明确报告四场景没有 JSON 返回；实际测试只有在 response 可解析时才检查 wire envelope。因此本报告**不声称 CANCELLED/SERVER_SHUTTING_DOWN JSON 已发送到客户端**。

无条件通过的行为证据是 source 收到的 trace_token 与唯一 durable audit request_id 相等、最终 audit.status=cancelled。另一个测试对两条 reserve 路径阻塞 source，在 future 尚未完成时断言 observer 已同步记录 ID，再 drop future 后对账 cancelled audit。结合共享 trace 的源码路径，这证明取消前 ID 可见与审计保留；不能替代取消 wire 送达保证。

这些是定向 Windows/SQLite、真实 MCP listener 与可控 source 的证据，不代表全 Phase 或真实 PostgreSQL/Trino 验收。
