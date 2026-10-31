---
phase: 07-rest-mcp-shell
reviewed: 2026-10-31
target_commit: 3f5b5dbf73716c6320e6def6537d1750bbf2da72
finding: CR-03
status: resolved
review_mode: focused_read_only
new_findings: 0
---

# MCP CR-03 独立复签

结论：`07-13-GATE-REVIEW.md` 的 CR-03 已由 `3f5b5db` 修复。当前复签未发现该修复引入的阻塞问题。原审查报告保留，不修改其历史结论。

## 源码核对

| 入口 | schema 与实现核对 | 结论 |
| --- | --- | --- |
| `describe` | `mcp_catalog.rs:151` 的空参数 schema 对应 `mcp.rs:160`，不再提前读取 `objectType` | 一致 |
| `query`、`object_set`、`searchAround` | `mcp_catalog.rs:117` 仅要求 `request`；`mcp.rs:229` 从 `wire::QueryRequest` 转换 set，由 set 确定类型，再查当前 `ctx.scope.types` 并构造 `ValidatedQuery` | 一致，未跳过类型授权 |
| `aggregate` | 同一 request-only schema 对应 `mcp.rs:285`；沿用 `QueryService::execute_aggregate`，后者在 `aggregate.rs:939` 的执行路径中查 active ontology 和 principal scope，并执行 `validate_request` | 一致，未绕过聚合授权与审计 |
| `get_object` | `mcp.rs:190` 分支内保留 `objectType`、类型名与 ontology 查找，并交由 `QueryService` 执行 | 必需参数保持 |
| `history` | `mcp.rs:328` 分支内保留 `objectType`；继续检查 type scope、当前对象行，并用同一 active snapshot 读取历史 | 必需参数与授权保持 |

所有调用仍先进入 `call_tool` (`mcp.rs:436`)：重新构造 authenticated request context，再用当前主体 catalog 检查 tool 是否可用。`request_context` 继续校验 session owner；`context::build` 继续验证 bearer、获取 request snapshot、编译当前 scope。此次改动仅把仅两种工具需要的类型解析移入对应分支。

## 回归证据

本复签独立读取了提交差异、catalog、dispatcher、context、聚合服务与新增真实 HTTP 测试。以下运行结果由 `phase07_13_trino` 执行 lane 提供，本复签未重复执行测试。

- RED：`cargo test -p ontoforge-server --test mcp_contract --locked mcp_read_tools_use_declared_argument_shapes -- --nocapture` 在旧实现上 `1 failed`；`describe({})` 返回 JSON-RPC `-32602`、`objectType is required`，属于行为失败。
- GREEN：相同命令在修复后 `1 passed`；fixture 重构与测试 Clippy 修复后的最终复跑仍 `1 passed`。
- 扩展回归：`cargo test -p ontoforge-server --test mcp_contract --test mcp_transport --test mcp_l2_defaults --test mcp_l2 --locked -- --nocapture`：`mcp_contract` 5/5、`mcp_transport` 4/4、`mcp_l2_defaults` 3/3、`mcp_l2` 2/2，合计 14 passed、0 ignored。
- `cargo clippy -p ontoforge-server --lib --locked -- -D warnings` 通过。
- `cargo clippy -p ontoforge-server --test mcp_contract --locked -- -D warnings` 通过。
- 上述 Cargo 命令均设置 `CARGO_BUILD_JOBS=2`。

新增 `mcp_read_tools_use_declared_argument_shapes` (`tests/mcp_contract.rs:53`) 经实际 listener 初始化 MCP session、发送 `notifications/initialized`，随后以 schema 所示形状调用五个工具；检查 describe 的类型可见性、三种 list 的真实行数、aggregate 计数、未授权 Aircraft 拒绝，以及缺 `objectType` 的 get_object 拒绝。最终确认三次 list 与一次 aggregate 产生四条成功审计。

## 验证边界

- 这是 CR-03 的定向复签，不代表 Phase 7 的完整验收或 Linux CI 已通过。
- 测试使用真实 HTTP/MCP transport 与本地 fixture，不替代真实 PostgreSQL/Trino 服务门禁。
- 未重新审查本次提交未改变的全部 MCP 安全边界；既有门禁仍按 07-13 计划执行。
