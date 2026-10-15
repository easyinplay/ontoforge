# Project Research Summary

**Project:** ontoforge
**Date:** 2026-10-15
**输入:** `rust-stack.md`、`foundry-docs-spec.md`、`ref-open-foundry.md`、`ref-bkn-foundry.md`、`ref-open-ontology.md`、`.planning/adr/0001–0011`（ADR 与研究冲突时以 ADR 为准）

## Executive Summary

ontoforge 是 Rust 实现、可自托管的开源 Ontology 操作层，复刻 Foundry Ontology 核心，并强化两个差异化方向：

1. **Lint 引擎**：把 Foundry 的 best practices / anti-patterns 变成静态规则（E / W / I 三级，line:col 诊断）。
2. **写路径强制完整性**：对象写入只经 Action 校验与授权，落不可变 edit log，为后续 scenario / branch 打基础。

同一份 ontology 通过 REST 与 MCP 两个同等地位的出口，分别服务业务应用与 AI agent。

## Key Findings

### Recommended Stack（rust-stack.md + ADR）
- **Schema**：YAML 为主、JSON 等价；`serde-saphyr` + `miette` 诊断；`schemars` 导出 JSON Schema
- **存储**：`StorageProvider` trait；SQLite（dev / test）+ PostgreSQL（生产），`sqlx`；共享 conformance 测试；通用 JSON 属性表
- **查询**：自有 ObjectSet AST（同时是 wire 格式）→ `sea-query`；全文用后端原生 FTS（`tantivy` 留 v2）
- **表达式**：CEL（`cel` crate）
- **API / MCP**：`axum` + `utoipa`（OpenAPI 3.1）；官方 `rmcp`（stdio + streamable HTTP）
- **客户端**：Rust client 手写共享 DTO；TS 平台层由 `openapi-typescript` 生成；ontology 级强类型 codegen 留到 MVP 之后（ADR-0009）
- **Workspace**：10 个 crate 起步（ADR-0011）

### Expected Features（foundry-docs-spec.md §5）
- **MVP**：schema 定义 → lint → 存储与 edit log → action → object set 查询 → REST + client → MCP → RBAC
- **v2**：interface、shared property、struct、value type、派生属性、scenario、functions、行列级安全、vector / KNN、订阅、branching

### Architecture Patterns（参考项目）
- **open-foundry**（TS）：Action 是唯一写路径；存储 SPI + conformance 测试；link 一等实体 + 数据库层基数约束；history 表
- **bkn-foundry**（Go）：分支内建在定义主键中；Cypher → 计划 → SQL 的编译管线；fail-closed 授权与「先过滤后分页」
- **open-ontology**（TS）：IR 是唯一契约；声明式 change；`model:check` 带位置的诊断

## Implications for Roadmap

垂直切片，每个 phase 端到端可用：

| Phase | 目标 | 进入前 spike | 主要陷阱 |
|-------|------|-------------|---------|
| 1. Schema + Lint | YAML/JSON → IR，E/W 规则，line:col 诊断 | `serde-saphyr` 按路径取 span（1d） | IR 是唯一契约，避免出现第二份 schema 表示 |
| 2. Storage + edit log | StorageProvider，SQLite + PG，append-only edit log，批量导入 | `postgresql_embedded` on Windows；`sea-query` JSON 表达式与索引 | edit log 格式是 scenario / branch 的基础，必须在此定死 |
| 3. Action 执行 | 参数校验、rules、CEL submission criteria、batch、validate-only | `cel` 静态类型检查（0.5d） | 事务边界；先授权后求值 |
| 4. Object set 查询 | filter / search / sort / 分页 / searchAround / 集合运算 / 聚合 | — | SQLite 与 PG 结果等价；递归遍历深度上限 |
| 5. Auth | OIDC JWT、dev token、RBAC、fail-closed | — | dev 模式不得 allow-all；列表先过滤后分页 |
| 6. REST + 客户端 | OpenAPI 3.1、Rust client、TS 平台层 client | — | OpenAPI 与实现零漂移（快照测试） |
| 7. MCP server | stdio + HTTP，describe / query / 每个 action 一个工具 | `rmcp` 动态 tool 列表（0.5d） | 工具列表随 ontology 动态生成；与 REST 共用授权与 edit log |

phase 顺序可由 roadmapper 调整，但依赖关系不变：schema → storage → action / query → auth → REST → MCP。

### Research Flags
- 需要 phase 级 research：Phase 2（存储表设计与 edit log 格式）、Phase 4（查询编译器）、Phase 7（MCP 工具面）
- 模式已明确、可跳过 research：Phase 5（RBAC 范围小）、Phase 6（axum + utoipa 常规用法）

## Confidence Assessment

| 领域 | 等级 | 依据 |
|------|------|------|
| 技术栈 | 高 | crates.io / GitHub 现状核查 + ADR 锁定 |
| 能力规格 | 高 | 镜像 Foundry 文档（含补抓 118 页）+ 三个参考实现 |
| 存储与查询 | 中-高 | SPI 模式成熟；SQLite / PG 等价需要充分测试 |
| 权限模型 | 中 | 自研 RBAC 范围可控，fail-closed 细节需审慎 |
| Functions（v2） | 低 | 只预留 trait |

### Gaps to Address
- spike 清单见上表，结论写回对应 ADR
- 通用 JSON 表 vs 热点属性物化列：Phase 2 research 决定
- edit log 如何承载未来的 branch 视图：Phase 2 设计时预留字段
- 文档镜像剩余小缺口（geo / fuzzy 运算符名、函数限制数值、object edits 机制、restricted view 语法）不阻塞 v1

## Sources

- `.planning/PROJECT.md`、`.planning/research/*.md`、`.planning/adr/0001–0011`（2026-10-15）

---
*Research completed: 2026-10-15*
*Ready for roadmap: yes*
