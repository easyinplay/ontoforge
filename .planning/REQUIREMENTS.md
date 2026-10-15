# Requirements: ontoforge

**Defined:** 2026-10-15
**Core Value:** 定义一次 ontology，应用和 agent 就能通过 REST 与 MCP 两个同等地位的出口，对同一份对象图做受控的读写：写入一律经 Action 校验与授权，每次写入都可审计。

"开发者"指编写 ontology 的人，"客户端"指调用 REST / Rust client / TS client 的应用，"agent"指通过 MCP 接入的 AI agent。

## v1 Requirements

### Schema（ontology-as-code）

- [x] **SCHEMA-01**: 开发者可以用 YAML（或等价 JSON）在多个文件中定义一个 ontology，`ontoforge validate` 把它们合并加载为单一 IR
- [x] **SCHEMA-02**: 开发者可以定义 object type：`apiName`、primary key、title key、displayName、description、aliases、hidden
- [x] **SCHEMA-03**: 开发者可以为属性选择 base type：Byte / Short / Integer / Long / Float / Double / Decimal、String、Boolean、Date、Timestamp、Array、Geopoint，并声明 required
- [x] **SCHEMA-04**: 开发者可以定义 link type：1:1、1:N（外键属性）、M:N（join 定义），两个方向各有 apiName
- [ ] **SCHEMA-05**: 开发者可以定义 action type：参数（类型、required、默认值）、rules（create / modify / delete object，add / remove link）、submission criteria（CEL）
- [x] **SCHEMA-06**: 定义有错时，开发者看到带文件名与 line:col 的诊断信息
- [ ] **SCHEMA-07**: 开发者可以导出 ontology 定义的 JSON Schema，用于编辑器补全与 LLM 生成

### Lint

- [ ] **LINT-01**: `ontoforge lint` 对 ontology 运行 E 级规则（适用于 v1 schema 范围的 ONT-E 规则），有 E 级问题时退出码非 0
- [ ] **LINT-02**: `ontoforge lint` 运行 W01–W11 命名与结构警告
- [ ] **LINT-03**: lint 结果可输出为人类可读格式（带源码定位）和 JSON 格式
- [ ] **LINT-04**: 开发者可以在配置中按规则 ID 关闭或调整严重级

### Storage 与 edit log

- [ ] **STORE-01**: 服务可以在 SQLite（dev / test）或 PostgreSQL（生产）上运行，两个后端通过同一套 conformance 测试
- [ ] **STORE-02**: 加载新版 ontology 不需要手写 DDL 迁移（通用表设计）
- [ ] **STORE-03**: 每次成功提交的写入都追加到不可变 edit log，记录 actor、action、时间、参数与 edits
- [ ] **STORE-04**: 客户端可以查询某个对象的变更历史
- [ ] **STORE-05**: link 基数（1:1、1:N）在存储层强制，违反时写入被拒绝
- [ ] **STORE-06**: 开发者可以用 `ontoforge import` 从 JSON / CSV 批量导入对象与 link，导入作为系统 edit 记入 edit log

### Action

- [ ] **ACTION-01**: 客户端提交 action 时，参数按类型、required 与约束校验，失败时返回逐参数的错误
- [ ] **ACTION-02**: action 的 rules 在单个事务内执行，生成 create / modify / delete object 与 add / remove link 的 edits
- [ ] **ACTION-03**: submission criteria（可引用参数、当前用户、对象当前属性）不满足时，action 被拒绝并返回配置的失败信息
- [ ] **ACTION-04**: 客户端可以用 validate-only 模式提交 action，只得到校验结果，不产生写入
- [ ] **ACTION-05**: 客户端可以批量提交同一 action，全部成功或全部回滚
- [ ] **ACTION-06**: action 成功后返回标准化的 edits 摘要（新增 / 修改 / 删除的对象与 link）
- [ ] **ACTION-07**: object type 被配置为仅允许经 action 编辑时，没有其他写入途径（导入除外，且导入可审计）

### Object set 查询

- [ ] **QUERY-01**: 客户端可以按 primary key 获取单个对象，并可指定返回的属性子集
- [ ] **QUERY-02**: 客户端可以用 WhereClause（eq / gt / gte / lt / lte / isNull / in / startsWith / containsAnyTerm / containsAllTerms / and / or / not）过滤对象
- [ ] **QUERY-03**: 客户端可以对 String 属性做全文搜索
- [ ] **QUERY-04**: 客户端可以对结果排序，并用游标分页遍历
- [ ] **QUERY-05**: 客户端可以沿 link 遍历（searchAround），也可以获取单个对象的关联对象
- [ ] **QUERY-06**: 客户端可以对 object set 做 union / intersect / subtract
- [ ] **QUERY-07**: 客户端可以对 object set 做聚合（count / sum / avg / min / max / approximate & exact distinct），支持按 exact 值、固定宽度与范围 groupBy
- [ ] **QUERY-08**: 同一查询在 SQLite 与 PostgreSQL 上返回相同结果（等价测试覆盖）

### API 与客户端

- [ ] **API-01**: 服务暴露 REST API，覆盖 ontology 元数据、对象读取、object set 查询、action 提交与 edit 历史
- [ ] **API-02**: 服务导出 OpenAPI 3.1 文档，并纳入快照测试
- [ ] **API-03**: 错误响应统一为 `{ errorCode, errorName, parameters }`
- [ ] **API-04**: Rust 开发者可以用 `ontoforge-client` 调用全部 REST 能力
- [ ] **API-05**: TypeScript 开发者可以使用由 OpenAPI 生成的平台层 client 调用全部 REST 能力

### MCP

- [ ] **MCP-01**: agent 可以通过 stdio 或 streamable HTTP 连接 MCP server
- [ ] **MCP-02**: agent 可以调用 describe 工具获取 ontology 结构（object types、属性、links、actions 及其描述）
- [ ] **MCP-03**: agent 可以通过查询工具执行 object set 查询（filter / search / searchAround / aggregate）
- [ ] **MCP-04**: 每个 action type 自动暴露为一个 MCP 工具，输入 schema 由参数定义派生
- [ ] **MCP-05**: agent 可以以 validate-only 方式调用 action 工具（dry run）
- [ ] **MCP-06**: agent 通过 MCP 的读写与 REST 走同一套授权与 edit log

### Auth

- [ ] **AUTH-01**: 客户端可以用 OIDC 签发的 JWT 认证（JWKS 自动拉取与缓存）
- [ ] **AUTH-02**: dev 模式下开发者可以用 CLI 生成本地 token，无需外部 IdP
- [ ] **AUTH-03**: 管理员可以在 ontology、object type、action type 级别授予角色（viewer / editor / owner 等）
- [ ] **AUTH-04**: 未授权的读写一律被拒绝（fail-closed）；列表与查询结果先按权限过滤再分页
- [ ] **AUTH-05**: action 执行先检查执行权限，再求值 submission criteria，错误信息不泄露无权访问的对象状态

### 开发者体验

- [ ] **DX-01**: 开发者用一个二进制 `ontoforge serve --dev` 即可在本机（含 Windows）启动完整服务（SQLite + dev token）
- [ ] **DX-02**: 仓库附带至少一个示例 ontology，可完成导入、查询、执行 action 的端到端演示
- [x] **DX-03**: Windows 本机 `cargo test` 不依赖 Docker 即可全部通过

## v2 Requirements

### Schema 扩展

- **SCHEMAX-01**: interfaces（属性映射、extends、interface object set 查询、面向 interface 的 action）
- **SCHEMAX-02**: shared properties
- **SCHEMAX-03**: struct 属性（main field、struct array）与 property reducers
- **SCHEMAX-04**: value types（enum / range / regex / UUID / uniqueness 等约束，版本化）
- **SCHEMAX-05**: runtime 派生属性（`withProperties`：link select、聚合、表达式）

### 安全

- **SEC-01**: 行级与列级安全策略（读时强制）
- **SEC-02**: 评估 `cedar-policy` 作为策略引擎

### 检索

- **SEARCH-01**: vector 属性与 KNN（nearestNeighbors）
- **SEARCH-02**: `tantivy` 全文索引替换后端原生 FTS

### 运行时

- **FUNC-01**: query / edit functions 运行时（HTTP 远程函数，之后 `wasmtime` component）
- **FUNC-02**: function-backed action rule
- **SUB-01**: WebSocket 对象订阅
- **SCEN-01**: scenarios（基于 edit log overlay）
- **BRANCH-01**: schema branching + proposals
- **SDK-01**: ontology 级强类型 SDK codegen（TS 优先）
- **GQL-01**: GraphQL API

## Out of Scope

| Feature | Reason |
|---------|--------|
| 应用层（Workshop、Object Explorer、Quiver、Slate 等） | 前端产品，不是 ontology 核心 |
| 数据集成平台（Pipeline Builder、数据集事务、构建调度） | 属于数据平台；v1 只提供批量导入 |
| AIP Logic、模型部署、文档解析管线 | 作为外部插件对接 |
| Markings、cipher、media sets、time series、geotemporal series | 子系统规模大，按需求单独立项 |
| Release management、Marketplace、Developer Console UI | 平台运营功能，非核心 |
| 外部系统原生 writeback 连接器 | v1 只留 webhook side effect 扩展点 |
| 通用 CRUD 写接口 | 违背「写入只经 Action」的核心设计 |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| SCHEMA-01 | Phase 1 | Complete |
| SCHEMA-02 | Phase 1 | Complete |
| SCHEMA-03 | Phase 1 | Complete |
| SCHEMA-04 | Phase 1 | Complete |
| SCHEMA-05 | Phase 1 | Pending |
| SCHEMA-06 | Phase 1 | Complete |
| SCHEMA-07 | Phase 1 | Pending |
| LINT-01 | Phase 1 | Pending |
| LINT-02 | Phase 1 | Pending |
| LINT-03 | Phase 1 | Pending |
| LINT-04 | Phase 1 | Pending |
| DX-03 | Phase 1 | Complete |
| STORE-01 | Phase 2 | Pending |
| STORE-02 | Phase 2 | Pending |
| STORE-03 | Phase 2 | Pending |
| STORE-04 | Phase 2 | Pending |
| STORE-05 | Phase 2 | Pending |
| STORE-06 | Phase 2 | Pending |
| ACTION-01 | Phase 3 | Pending |
| ACTION-02 | Phase 3 | Pending |
| ACTION-03 | Phase 3 | Pending |
| ACTION-04 | Phase 3 | Pending |
| ACTION-05 | Phase 3 | Pending |
| ACTION-06 | Phase 3 | Pending |
| ACTION-07 | Phase 3 | Pending |
| QUERY-01 | Phase 4 | Pending |
| QUERY-02 | Phase 4 | Pending |
| QUERY-03 | Phase 4 | Pending |
| QUERY-04 | Phase 4 | Pending |
| QUERY-05 | Phase 4 | Pending |
| QUERY-06 | Phase 4 | Pending |
| QUERY-07 | Phase 4 | Pending |
| QUERY-08 | Phase 4 | Pending |
| AUTH-01 | Phase 5 | Pending |
| AUTH-02 | Phase 5 | Pending |
| AUTH-03 | Phase 5 | Pending |
| AUTH-04 | Phase 5 | Pending |
| AUTH-05 | Phase 5 | Pending |
| API-01 | Phase 6 | Pending |
| API-02 | Phase 6 | Pending |
| API-03 | Phase 6 | Pending |
| API-04 | Phase 6 | Pending |
| API-05 | Phase 6 | Pending |
| DX-01 | Phase 6 | Pending |
| DX-02 | Phase 6 | Pending |
| MCP-01 | Phase 7 | Pending |
| MCP-02 | Phase 7 | Pending |
| MCP-03 | Phase 7 | Pending |
| MCP-04 | Phase 7 | Pending |
| MCP-05 | Phase 7 | Pending |
| MCP-06 | Phase 7 | Pending |

**Coverage:**

- v1 requirements: 51 total
- Mapped to phases: 51
- Unmapped: 0

---
*Requirements defined: 2026-10-15*
*Last updated: 2026-10-15 after roadmap creation*
