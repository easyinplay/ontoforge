# ADR-0017 引擎与 Trino 连接

- status: accepted
- date: 2026-10-16
- 依据：`docs/designs/trino-engine-sql-tiers.md` Premise 4、§4、§5；`.planning/research/spike-01-trino-client.md`、`spike-02-sqlparser-l2.md`、`ref-mcp-trino.md`；Eng Review D9 / D14

## Context
Trino 是必需的外部计算平台：读 source、联邦关联、ETL、L2 原始查询都依赖它。ontoforge 是服务，只带连接能力，不打包、不托管 Trino。

## Decision
- 新 crate `ontoforge-engine`：`Engine` trait；两个实现：Trino（生产）、SQLite 替身（测试与 `serve --dev`，不承诺生产；`catalog.schema` 映射为 `ATTACH` 的库名 `catalog__schema`）。
- **Trino client 自写**：`reqwest` + rustls 薄封装 Trino HTTP 协议（约 700 行）。keep-alive 连接池；`nextUri` 轮询带退避；客户端断连发 `DELETE` 取消查询；`X-Trino-Trace-Token` = request_id，`X-Trino-Client-Tags` 标注来源。
- **SQL 生成用自有小型类型化 printer**，只覆盖编译器产出的子集；字面量与标识符只经两个转义函数输出（Trino 没有带外参数绑定）。用 proptest 与真实 Trino / SQLite 执行结果验证。`sqlparser` 只用于 L2 输入校验与往返自检。
- **两个 Trino 服务身份**：ontology 身份（L0 叠加读、Action 前置读；可访问 source catalog 与 edit store catalog）；L2 身份（只读、只能访问 allow-list，见 ADR-0018）。
- **运维**：Trino 不可用时，需要 Trino 的读与 Action fail-closed（`EngineUnavailable`，可重试，不写入）；`/health` 与 MCP describe 暴露引擎状态。查询时限、按服务身份的 Trino resource group（部署文档要求为 ontoforge 配置专用组）、主体并发配额。
- **CI**：PR 跑 Trino + PG（source 与 edit store 两个 catalog）；nightly 加跑 Iceberg（JDBC catalog + MinIO），失败自动开 issue；SQL Server 只走 `ONTOFORGE_PRIVATE_FIXTURES` 私有 fixture。

## Rejected
- `trino-rust-client` 0.12.0 / `prusto`：成熟度与协议覆盖不足（spike 1）。
- mcp-trino 作为 sidecar：破坏单二进制、只能拿到 JSON 文本、无法按请求取消与透传身份。
- 用 `sqlparser` AST 生成 Trino SQL：无法解析 timestamptz / array cast，ROW 与 LIMIT/OFFSET 的 Display 对 Trino 有损（spike 2 G11 / G16 / G37）。
- 打包或托管 Trino：ontoforge 只带连接能力。

## Consequences
Windows 本机测试覆盖不到 Trino 方言语义，靠 Linux CI 集成测试与差分测试补足；sea-query 只用于 edit store（PG / SQLite），不用于 Trino。
