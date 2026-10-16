# ontoforge

## What This Is

ontoforge 是一个用 Rust 实现、可自托管的开源 Ontology 操作层，复刻 Palantir Foundry Ontology 的核心能力：用 YAML/JSON 定义 object type / link type / action type（ontology-as-code），对象数据只能通过带校验的 Action 写入并留下不可变 edit log，再用 object set 查询（过滤、遍历、集合运算、聚合）读取。同一份 ontology 同时通过 REST（配 Rust / TypeScript client）服务业务应用，通过 MCP 服务 AI agent。

## Core Value

定义一次 ontology，应用和 agent 就能通过 REST 与 MCP 两个同等地位的出口，对同一份对象图做受控的读写：写入一律经 Action 校验与授权，每次写入都可审计。

## Requirements

### Validated

- ✓ ontology-as-code：YAML/JSON → 单一 IR，line:col 诊断，导出 JSON Schema 供编辑器补全 — Phase 1
- ✓ lint 引擎：E 级规则 + W01–W11，human / JSON 双格式，`ontoforge.toml` 配置 — Phase 1

### Active

- [ ] 数据源绑定与 Trino 连接（ontoforge 只带连接能力，不打包 Trino）— ADR-0015、ADR-0017
- [ ] edit store 按 object type 强类型分表、按属性 patch，不可变 edit log — ADR-0016
- [ ] 用户 / 组 / 角色与 scope 模型（行列级安全提前进 v1）、Action 鉴权管线 — ADR-0018
- [ ] scoped L0 读：Rust 编排叠加，Trino 只跑单表查询 — ADR-0020
- [ ] online Action：并发令牌、`command_id`、webhook outbox
- [ ] L2 Raw DQL（AST 白名单 + Trino 侧权限自检）与全级别读审计
- [ ] REST（OpenAPI 3.1 + Rust / TS client）与 MCP（toolset 分层）两个出口；`serve --dev` 单二进制演示 — ADR-0019

需求编号与 phase 映射见 `.planning/REQUIREMENTS.md`；方向依据 `docs/designs/trino-engine-sql-tiers.md`。

### Out of Scope

- 应用层（Workshop、Object Explorer、Quiver、Slate 等前端产品）：不是 ontology 核心
- 数据集成平台（Pipeline Builder、数据集事务、构建调度）：只提供批量导入与后续 connector 扩展点
- AIP Logic、模型部署、文档解析管线：作为外部插件对接
- Markings、cipher、media sets、time series、geotemporal series：子系统规模大，按需求单独立项
- Release management、Marketplace、Developer Console UI
- 外部系统原生 writeback 连接器：MVP 只留 webhook side effect 扩展点
- GraphQL：MVP 只做 REST + MCP（ADR-0007）

v2 候选（不在当前 milestone）：interfaces 与 shared properties、struct、value types、runtime 派生属性、行 / 列级安全、vector + KNN、scenarios、functions 运行时、WebSocket 订阅、schema branching + proposals、ontology 级强类型 SDK codegen。

## Context

- 设计参考：Foundry 官方文档镜像（`docs/foundry/`，含 2026-10-15 补抓的 118 页参考），以及三个开源项目的调研（`.planning/research/ref-*.md`）。只借设计不借命名（ADR-0002）。
- 能力规格与优先级：`.planning/research/foundry-docs-spec.md`；技术选型：`.planning/research/rust-stack.md`。
- 关键差异化：lint 引擎（把 Foundry best practices / anti-patterns 变成静态检查）、单二进制 dev 体验、MCP 一等公民、写路径强制完整性与审计。
- edit log 格式在 MVP 定死，后续 scenario / branch 以它为 overlay 基础，后改成本高。
- 待 spike 的技术点（进入对应 phase 前验证）：`serde-saphyr` 按路径取 span、`cel` 静态类型检查、`rmcp` 动态 tool 列表、`sea-query` JSON 表达式与表达式索引、`postgresql_embedded` 在 Windows 上的表现。

## Constraints

- **Tech stack**：Rust 1.98，edition 2024，cargo workspace（ADR-0011）；axum + utoipa、sqlx、sea-query、cel、rmcp — 已在 ADR-0003～0011 锁定
- **Platform**：必须在 Windows 11 上 `cargo build` / `cargo test` 通过（本机不依赖 Docker），生产部署目标 Linux
- **Testing**：Windows 本机默认测试只依赖 SQLite；Postgres 测试本机用 `postgresql_embedded`，CI 用 service container
- **Naming**：项目 / crate 前缀 `ontoforge`；标识符遵循 ADR-0002，禁止沿用参考仓库的标识符
- **License**：不复制参考仓库代码（bkn-foundry 含 OpenBKN License 文件）

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| 品牌名 ontoforge（ADR-0001） | 规避 Palantir "Foundry" 商标风险 | — Pending |
| 应用 + Agent 双出口定位 | 同一核心同时服务业务系统与 AI agent，MCP 是 REST 之上的薄封装 | — Pending |
| YAML/JSON → 单一 IR（ADR-0003） | LLM 友好、diff 友好、line:col 诊断 | ✓ Good — Phase 1 交付，多文档 YAML 需项目级 yaml.schemas（G-1-2） |
| Postgres 生产 + SQLite dev/test（ADR-0004） | 单二进制开发体验 + 生产可靠性，conformance 测试保证一致 | ⚠️ Revisited — 被 ADR-0016 取代：业务数据经 Trino，PG 只存系统数据与 edit store |
| 写入只经 Action + 不可变 edit log（ADR-0004） | 审计与后续 scenario / branch 的基础 | — Pending |
| CEL 做表达式（ADR-0006） | 非图灵完备、可静态检查、进程内求值 | — Pending |
| 自研 RBAC，fail-closed（ADR-0008） | 单二进制，不依赖外部授权服务 | ⚠️ Revisited — AuthZ 被 ADR-0018（scope 模型）取代 |
| Functions 延后到 v2（ADR-0010） | MVP 只定 trait，控制范围 | — Pending |
| Trino 主引擎 + 分级 SQL（L0 / L2），ontoforge 只带连接能力 | 复用企业湖仓与联邦查询；可验证的 agent 权限边界 | — Pending（ADR-0015..0020，ROADMAP Phase 2–7 已重排） |

决策全文见 `.planning/adr/`。

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-10-16 after Phase 1*
