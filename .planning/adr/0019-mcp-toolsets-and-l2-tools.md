# ADR-0019 MCP toolset 分层与 L2 工具

- status: accepted
- date: 2026-10-16
- 修订：ADR-0007 的 MCP 工具面（REST 部分不变）
- 依据：`docs/designs/trino-engine-sql-tiers.md` §4、§6（Phase 7）；CEO review 4B / 8A；`.planning/research/ref-mcp-trino.md`

## Decision
- **toolset 按主体过滤**：`core`（精简：describe、object set 查询、按 pk 取对象、action 工具）、`full`（加聚合、遍历、历史等）、`sql:raw`（L2 工具）。`list_tools` 只返回主体被授予的工具。
- **L2 工具**：名称与参数对齐 mcp-trino 的 6 个工具（`execute_query`、`list_catalogs`、`list_schemas`、`list_tables`、`get_table_schema`、`explain_query`），便于已使用 mcp-trino 的 agent 迁移；只借设计，不复制代码。安全机制全部重做（ADR-0018 的 AST 白名单、allow-list、自检、L2 身份），`list_*` 结果按 allow-list 过滤。
- **结果上限**：每类工具行数 + 字节双上限，超限返回 `truncated=true` 与继续方式；分页用 keyset 游标（`has_more` 判定结束，短页不等于结束），游标格式预留 `snapshot_id`；L2 注入 LIMIT。
- **错误分层**：L0 返回结构化错误码 + apiName 级描述 + request_id，原始 Trino 错误只进服务端日志；L2 / L3 可见原始错误。
- L3 与任何定义 / 授权变更工具都不进 agent MCP。L1 / `sql:ontology` 推迟到下一 milestone。

## Rejected
- 照搬 mcp-trino 的正则只读守卫：可被 `TABLE(system.query …)`、注释内引号、`EXPLAIN ANALYZE` 绕过。
- 静态 Trino 用户 + fail-open 的身份透传（mcp-trino 现状）。
