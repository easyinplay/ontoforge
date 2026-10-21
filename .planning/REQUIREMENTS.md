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
- [x] **SCHEMA-05**: 开发者可以定义 action type：参数（类型、required、默认值）、rules（create / modify / delete object，add / remove link）、submission criteria（CEL）
- [x] **SCHEMA-06**: 定义有错时，开发者看到带文件名与 line:col 的诊断信息
- [x] **SCHEMA-07**: 开发者可以导出 ontology 定义的 JSON Schema，用于编辑器补全与 LLM 生成

### Lint

- [x] **LINT-01**: `ontoforge lint` 对 ontology 运行 E 级规则（适用于 v1 schema 范围的 ONT-E 规则），有 E 级问题时退出码非 0
- [x] **LINT-02**: `ontoforge lint` 运行 W01–W11 命名与结构警告
- [x] **LINT-03**: lint 结果可输出为人类可读格式（带源码定位）和 JSON 格式
- [x] **LINT-04**: 开发者可以在配置中按规则 ID 关闭或调整严重级

### 数据源绑定与引擎

- [x] **BIND-01**: 开发者在独立的 `SourceBinding` 绑定文件（按 profile 分目录）中为 object type 与 M:N link 声明 source 绑定（catalog / schema / table / columns），定义中以 `backing: source | editOnly` 显式声明数据来源（ADR-0021）
- [x] **BIND-02**: 绑定遵守类型契约（decimal 精度、无时区 timestamp 的 `timezone`、可选 collation、geopoint 不支持），违反时 lint 报 E
- [x] **BIND-03**: `ontoforge validate --online` 对 Trino 校验绑定，`--check-pk` 查重；启动与定时 drift 校验不匹配时该 object type 降级（`SourceSchemaDrift`），其余照常
- [x] **ENGINE-01**: 服务经自写 Trino client 连接部署方已有的 Trino（不打包、不托管），支持超时、断连取消、trace token 与两个服务身份
- [x] **ENGINE-02**: Trino 不可用时，需要 Trino 的读写 fail-closed（`EngineUnavailable`），`/health` 与 describe 暴露引擎状态
- [x] **ENGINE-03**: 发往 Trino / SQLite 的 SQL 由自有 printer 生成，值只经转义函数输出，注入防护由 proptest 与真实执行验证
- [x] **ENGINE-04**: SQLite 替身引擎支撑 Windows 本机 `cargo test` 与 `serve --dev`（不承诺生产）
- [x] **ENGINE-05**: Linux CI 跑 Trino + PG 集成测试，nightly 加跑 Iceberg；SQL Server 由私有 fixture 验证

### Edit store 与 edit log

- [ ] **STORE-01**: edit store 在 PostgreSQL（生产）或 SQLite（dev / test）上运行，两个后端通过同一套 conformance 测试
- [ ] **STORE-02**: 新增类定义变更在 reload 时自动生成 DDL；破坏性变更被 lint 拦下，经 `ontoforge migrate --plan/--apply` 完成并归档旧列；reload 原子生效（Trino 元数据可见后才切换、`lock_timeout`、`def_version` 栅栏）
- [ ] **STORE-03**: 每次成功提交的写入都追加到不可变 edit log，记录 actor、action、时间、参数与 edits
- [ ] **STORE-04**: 客户端可以查询某个对象的变更历史
- [ ] **STORE-05**: link 基数（1:1、1:N）在 edit store 写入时强制；source 数据的基数由 lint / 校验查询报告，不阻塞读取
- [x] **STORE-06**: 开发者可以用 `ontoforge import` 导入 edit-only 对象与替身引擎数据，导入作为系统 edit 记入 edit log；业务数据摄取由 Trino ETL（L3）负责
- [x] **STORE-07**: 叠加按属性 patch：edit 只存设置过的属性，未设置的属性跟随 source；tombstone 不复活，release 显式且版本单调递增

### Scope 与认证

- [ ] **SCOPE-01**: 管理员可以管理用户、组（可同步自 OIDC `groups` claim）与角色，角色绑定到用户或组
- [ ] **SCOPE-02**: 角色可授予可见 object type、属性 allow-list、CEL 行谓词（只引用同行属性与 `principal.attrs`）、action 执行权、SQL 级别与 MCP toolset
- [ ] **SCOPE-03**: 多角色合并为 scoped 基关系（行 = 各角色谓词之 OR，列按授予它的角色谓词遮蔽），所有表达式只能绑定到该关系，不产生跨角色列 × 行泄露
- [ ] **SCOPE-04**: 同一 CEL 行谓词编译成 SQL 与在 Rust 中对单行求值的结果一致（差分测试覆盖）
- [ ] **SCOPE-05**: 无授权 = 不可见（fail-closed）；缺失的主体属性使相关谓词为 false；无法编译的谓词在 validate 时报错
- [x] **SCOPE-06**: 定义 reload 与角色变更时，被授予某 action 的角色列集必须覆盖其 criteria 引用的列，否则报错不生效
- [ ] **SCOPE-07**: 角色 / scope 只能经管理通道或 L3 修改并审计；ontology 定义只经部署方 reload；agent MCP 无变更工具；每个请求固定使用开始时的定义版本
- [x] **AUTH-01**: 客户端可以用 OIDC 签发的 JWT 认证（JWKS 自动拉取与缓存）
- [x] **AUTH-02**: dev 模式下开发者可以用 CLI 生成本地 token，无需外部 IdP

### Object set 查询（L0）

- [ ] **QUERY-01**: 客户端可以按 primary key 获取单个对象（已编辑对象 PG 直读，未编辑对象 Trino 单表查询），并可指定返回的属性子集
- [ ] **QUERY-02**: 客户端可以用 WhereClause（eq / gt / gte / lt / lte / isNull / in / startsWith / containsAnyTerm / containsAllTerms / and / or / not）过滤对象
- [ ] **QUERY-03**: 客户端可以对 String 属性做 token 匹配搜索（Trino 上无相关度排序）
- [ ] **QUERY-04**: 客户端可以对结果排序，并用 keyset 游标分页遍历（`has_more` 判定结束）
- [ ] **QUERY-05**: 客户端可以沿 link 遍历（searchAround），也可以获取单个对象的关联对象
- [ ] **QUERY-06**: 客户端可以对 object set 做 union / intersect / subtract
- [ ] **QUERY-07**: 客户端可以对 object set 做聚合（count / sum / avg / min / max / distinct），支持 groupBy
- [ ] **QUERY-08**: Rust 叠加实现对照内存参考模型的 proptest 通过，SQLite 替身与 Trino 的单表查询语义一致
- [ ] **QUERY-09**: 读结果带不透明的 HMAC `_version` 令牌；L0 输入、输出与错误不含物理名；结果按行数与字节上限截断并标 `truncated`
- [ ] **QUERY-10**: 每条 Trino 查询单表、≤ 2 fragment；安静或隔离集群下 pk 读 p95 < 500ms、列表 p95 < 1s、聚合 p95 < 1.5s、已编辑对象 PG 直读 p95 < 50ms

### Action

- [ ] **ACTION-01**: 客户端提交 action 时，参数按类型、required 与约束校验，失败时返回逐参数的错误
- [ ] **ACTION-02**: action 的 rules 在单个事务内执行，生成 create / modify / delete object 与 add / remove link 的 edits
- [ ] **ACTION-03**: submission criteria 不满足时 action 被拒绝并返回配置的失败信息；criteria 只在调用者的遮蔽视图上求值
- [ ] **ACTION-04**: 客户端可以用 validate-only 模式提交 action，只得到校验结果，不产生写入
- [ ] **ACTION-05**: 客户端可以批量提交同一 action，全部成功或全部回滚
- [ ] **ACTION-06**: action 成功后返回标准化的 edits 摘要（新增 / 修改 / 删除的对象与 link）
- [ ] **ACTION-07**: 对象只能经 action（或可审计的导入）写入，没有其他写入途径
- [ ] **ACTION-08**: 写字段必须在调用者对该行的可见列内且非系统字段；写后状态（含 link 两端与 release）必须对调用者可见，否则 `ScopeDenied`，错误不泄露对象状态
- [ ] **ACTION-09**: 客户端可附带读取时的 `_version` 令牌，不一致返回 `VersionConflict`；edit 读集在事务内按稳定顺序加锁
- [ ] **ACTION-10**: `command_id` 以主体为作用域幂等：重放前重新鉴权，同键异参报 `CommandIdReused`，响应丢失后可凭同键查询结果
- [ ] **ACTION-11**: 提交成功后经事务性 outbox 向下游 webhook 至少投递一次带 `event_id` 的事件
- [ ] **ACTION-12**: Action 只在目标或读集需要 Trino 时依赖 Trino；source 重复 pk 的对象返回 `AmbiguousObject` 且拒绝写入

### L2 与审计

- [ ] **L2-01**: 被授予 `sql:raw` 的主体可以执行只读 Trino DQL 并列举 allow-list 内的 catalog / schema / 表
- [ ] **L2-02**: L2 SQL 经 AST 白名单校验（只允许 SELECT / WITH，拒绝表函数、CALL、EXECUTE、多语句、会话与事务语句），执行时发送重序列化后的 SQL
- [ ] **L2-03**: L2 只能访问 catalog / schema allow-list，硬排除 edit store catalog 与 `system` 元数据表
- [ ] **L2-04**: 启动自检无法证明 L2 身份在 Trino 侧受限时禁用 L2 并告警（fail-closed）
- [ ] **AUDIT-01**: 所有级别的读取记录元数据审计（不含结果数据），异步落库，积压或持续失败时读请求 fail-closed；保留期可配

### API 与客户端

- [ ] **API-01**: 服务暴露 REST API，覆盖 ontology 元数据、对象读取、object set 查询、action 提交与 edit 历史
- [ ] **API-02**: 服务导出 OpenAPI 3.1 文档，并纳入快照测试
- [ ] **API-03**: 错误响应统一为结构化错误码 + apiName 级描述 + request_id；L0 不含物理名
- [ ] **API-04**: Rust 开发者可以用 `ontoforge-client` 调用全部 REST 能力
- [ ] **API-05**: TypeScript 开发者可以使用由 OpenAPI 生成的平台层 client 调用全部 REST 能力

### MCP

- [ ] **MCP-01**: agent 可以通过 stdio 或 streamable HTTP 连接 MCP server
- [ ] **MCP-02**: agent 可以调用 describe 工具获取 ontology 结构（object types、属性、links、actions 及其描述）
- [ ] **MCP-03**: agent 可以通过查询工具执行 object set 查询（filter / search / searchAround / aggregate）
- [ ] **MCP-04**: 每个 action type 自动暴露为一个 MCP 工具，输入 schema 由参数定义派生
- [ ] **MCP-05**: agent 可以以 validate-only 方式调用 action 工具（dry run）
- [ ] **MCP-06**: agent 通过 MCP 的读写与 REST 走同一套授权与 edit log
- [ ] **MCP-07**: `list_tools` 按主体过滤 toolset（core / full / sql:raw）；L2 的 6 个工具名称与参数对齐 mcp-trino
- [ ] **MCP-08**: L3 与任何定义 / 授权变更工具不出现在 agent MCP 中

### 开发者体验

- [ ] **DX-01**: 开发者用一个二进制 `ontoforge serve --dev` 即可在本机（含 Windows）启动完整服务（SQLite 替身引擎 + dev token），无需 Trino
- [ ] **DX-02**: 仓库附带至少一个示例 ontology，可完成查询、执行 action、查看历史的端到端演示
- [x] **DX-03**: Windows 本机 `cargo test` 不依赖 Docker 即可全部通过
- [ ] **DX-04**: release workflow 发布 Linux / macOS / Windows 单二进制到 GitHub Releases

## v2 Requirements

### Schema 扩展

- **SCHEMAX-01**: interfaces（属性映射、extends、interface object set 查询、面向 interface 的 action）
- **SCHEMAX-02**: shared properties
- **SCHEMAX-03**: struct 属性（main field、struct array）与 property reducers
- **SCHEMAX-04**: value types（enum / range / regex / UUID / uniqueness 等约束，版本化）
- **SCHEMAX-05**: runtime 派生属性（`withProperties`：link select、聚合、表达式）

### 安全

- **SEC-02**: 评估 `cedar-policy` 作为策略引擎
- **SEC-03**: 行谓词引用 link / 其他对象（跨对象 scope）

（原 SEC-01 行级与列级安全已提前进 v1：SCOPE-01..07）

### 湖仓与引擎（下一 milestone）

- **SQL-01**: L1 Ontology SQL（`sql:ontology`，类 SQL 按 object 名书写，服务端改写并注入 scope）
- **LAKE-01**: `batch` 写入模式：Nessie 分支上直写 Iceberg 后 merge
- **LAKE-02**: edit store materialize 与 edit log 归档到 Iceberg；快照一致分页（游标 `snapshot_id`）
- **LAKE-03**: Spark / Doris / Kyuubi 作为可选引擎服务特定 action
- **GOV-01**: AI 提议 ontology 变更 → 人审批 → 生效的流程与管理界面

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
| 打包、部署或托管 Trino | ontoforge 是服务，只带连接能力；Trino 由部署方提供 |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| SCHEMA-01 | Phase 1 | Complete |
| SCHEMA-02 | Phase 1 | Complete |
| SCHEMA-03 | Phase 1 | Complete |
| SCHEMA-04 | Phase 1 | Complete |
| SCHEMA-05 | Phase 1 | Complete |
| SCHEMA-06 | Phase 1 | Complete |
| SCHEMA-07 | Phase 1 | Complete |
| LINT-01 | Phase 1 | Complete |
| LINT-02 | Phase 1 | Complete |
| LINT-03 | Phase 1 | Complete |
| LINT-04 | Phase 1 | Complete |
| DX-03 | Phase 1 | Complete |
| BIND-01 | Phase 2 | Complete |
| BIND-02 | Phase 2 | Complete |
| BIND-03 | Phase 2（validate --online、--check-pk、drift 库函数与降级状态模型）+ Phase 7（启动 / 定时 drift 执行） | Complete |
| ENGINE-01 | Phase 2 | Complete |
| ENGINE-02 | Phase 2（错误分类与 status()）+ Phase 7（/health 与 describe 暴露，API-01） | Complete |
| ENGINE-03 | Phase 2 | Complete |
| ENGINE-04 | Phase 2（SQLite 替身支撑 Windows cargo test）+ Phase 7（serve --dev，DX-01） | Complete |
| ENGINE-05 | Phase 2 | Complete |
| STORE-01 | Phase 3 | Pending |
| STORE-02 | Phase 3 | Pending |
| STORE-03 | Phase 3 | Pending |
| STORE-04 | Phase 3 | Pending |
| STORE-05 | Phase 3 | Pending |
| STORE-06 | Phase 3 | Complete |
| STORE-07 | Phase 3 | Complete |
| SCOPE-01 | Phase 3 | Pending |
| SCOPE-02 | Phase 3 | Pending |
| SCOPE-03 | Phase 3 | Pending |
| SCOPE-04 | Phase 3 | Pending |
| SCOPE-05 | Phase 3 | Pending |
| SCOPE-06 | Phase 3 | Complete |
| SCOPE-07 | Phase 3 | Pending |
| AUTH-01 | Phase 3 | Complete |
| AUTH-02 | Phase 3 | Complete |
| QUERY-01 | Phase 4 | Pending |
| QUERY-02 | Phase 4 | Pending |
| QUERY-03 | Phase 4 | Pending |
| QUERY-04 | Phase 4 | Pending |
| QUERY-05 | Phase 4 | Pending |
| QUERY-06 | Phase 4 | Pending |
| QUERY-07 | Phase 4 | Pending |
| QUERY-08 | Phase 4 | Pending |
| QUERY-09 | Phase 4 | Pending |
| QUERY-10 | Phase 4 | Pending |
| ACTION-01 | Phase 5 | Pending |
| ACTION-02 | Phase 5 | Pending |
| ACTION-03 | Phase 5 | Pending |
| ACTION-04 | Phase 5 | Pending |
| ACTION-05 | Phase 5 | Pending |
| ACTION-06 | Phase 5 | Pending |
| ACTION-07 | Phase 5 | Pending |
| ACTION-08 | Phase 5 | Pending |
| ACTION-09 | Phase 5 | Pending |
| ACTION-10 | Phase 5 | Pending |
| ACTION-11 | Phase 5 | Pending |
| ACTION-12 | Phase 5 | Pending |
| L2-01 | Phase 6 | Pending |
| L2-02 | Phase 6 | Pending |
| L2-03 | Phase 6 | Pending |
| L2-04 | Phase 6 | Pending |
| AUDIT-01 | Phase 6 | Pending |
| API-01 | Phase 7 | Pending |
| API-02 | Phase 7 | Pending |
| API-03 | Phase 7 | Pending |
| API-04 | Phase 7 | Pending |
| API-05 | Phase 7 | Pending |
| MCP-01 | Phase 7 | Pending |
| MCP-02 | Phase 7 | Pending |
| MCP-03 | Phase 7 | Pending |
| MCP-04 | Phase 7 | Pending |
| MCP-05 | Phase 7 | Pending |
| MCP-06 | Phase 7 | Pending |
| MCP-07 | Phase 7 | Pending |
| MCP-08 | Phase 7 | Pending |
| DX-01 | Phase 7 | Pending |
| DX-02 | Phase 7 | Pending |
| DX-04 | Phase 7 | Pending |

**Coverage:**

- v1 requirements: 79 total
- Mapped to phases: 79
- Unmapped: 0

---
*Requirements defined: 2026-10-15*
*Last updated: 2026-10-19 after Phase 2*
