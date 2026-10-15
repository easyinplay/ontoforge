# ADR-0007 对外接口：REST + MCP，MVP 不做 GraphQL

- status: accepted
- date: 2026-10-15

## Decision
- REST：`axum` + `utoipa` / `utoipa-axum`，OpenAPI 3.1 从代码导出并入库做 snapshot 测试。
- MCP：官方 `rmcp`，stdio + streamable HTTP（挂在同一 axum 进程 `/mcp`）。工具面参照 Foundry Ontology MCP：describe / object-set 查询 / 每个 action 一个工具 / 每个 query function 一个工具。
- GraphQL 不进 MVP。
