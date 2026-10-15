# FoundryOntology Rust 技术栈选型调研

> 日期：2026-10-15 ｜ 工具链约束：rustc 1.98，Windows 11 开发与测试，Linux 部署
> 上游输入：`ref-open-foundry.md` §12-14、`ref-open-ontology.md` §9-11、`ref-bkn-foundry.md` §9-11、`foundry-docs-spec.md` §5（MVP 清单）
> 数据来源：crates.io API（版本 / 更新日期 / 下载量，2026-10-15 抓取）、`gh api`（star / 最近 push / release）、crate README、Tavily 搜索（仅 WASI 0.3 状态）。
> 标注约定：**[已验证]** = 本次查过一手数据；**[未验证]** = 推断或需 spike 确认；`NEEDS_USER_DECISION` = 需产品负责人拍板（业务 / 不可逆 / 偏好）。

## 0. 结论速览

| # | 决策 | 推荐 | 是否需用户决策 |
|---|------|------|----------------|
| 1 | Ontology 定义格式 | YAML（主）+ JSON（等价），`serde-saphyr` 解析带位置，`schemars` 导出 JSON Schema，`miette` 报错 | `NEEDS_USER_DECISION`（用户可见格式，后改代价高） |
| 2 | 存储 | `StorageProvider` trait + SQLite（dev 单二进制）+ PostgreSQL（生产），均用 `sqlx` 0.9；通用 JSON 属性表 + 系统列；conformance suite 两后端同跑 | `NEEDS_USER_DECISION`（SQLite 的支持等级） |
| 3 | 查询层 / 搜索 | 自有 ObjectSet AST → 自写 compiler → `sea-query` 1.0 出 SQL；全文 MVP 用后端原生（FTS5 / tsvector），`tantivy` 留 v2 | 否 |
| 4 | 表达式语言 | CEL：`cel` 0.15（原 `cel-interpreter` 已更名） | 否 |
| 5 | HTTP API | `axum` 0.8 + `utoipa` 6 + `utoipa-axum`；MVP 不做 GraphQL | 否 |
| 6 | MCP server | `rmcp` 3.5（官方 SDK），stdio + streamable HTTP（挂在同一 axum 进程） | 否 |
| 7 | Functions 运行时（v2） | 现在只定 `FunctionRuntime` trait + WIT 接口；v2 首选 `wasmtime` component model，HTTP 远程函数作为适配器 | `NEEDS_USER_DECISION`（函数作者语言，v2 前定即可） |
| 8 | AuthZ / AuthN | 自研 RBAC（trait 隔离），`cedar-policy` 作为 v2 行/属性级候选；AuthN 用 `jsonwebtoken` 11 + 自写 JWKS 缓存 | 否 |
| 9 | TS / Rust SDK | TS：OpenAPI → `openapi-typescript` + `openapi-fetch` 做通用层，再按 ontology 自研 typed codegen；Rust：手写薄 client 复用共享 DTO crate（不用 `progenitor`） | 否 |
| 10 | Workspace 布局 | 见 §10，统一前缀 `fo-`（占位） | `NEEDS_USER_DECISION`（crate 前缀 / 商标） |

---

## 1. Ontology 定义格式

| 选项 | 优点 | 缺点 | 成熟度（2026-10） |
|------|------|------|------------------|
| YAML + JSON（`serde` + `schemars` JSON Schema） | LLM 生成最稳；JSON Schema 让编辑器（VS Code YAML 插件）零成本补全/校验；一定义一文件，git diff 友好；与 IR 同构 | YAML 本身陷阱（Norway 问题、缩进）；`serde_yaml` 已废弃需选新解析器 | `schemars` 1.2.3（2026-10-12，2 亿 recent dl）[已验证]；`serde-saphyr` 2.0.0（2026-10-13，带 `miette` feature）[已验证]；`jsonschema` 0.58.6 [已验证] |
| GraphQL SDL 方言（directive 扩展） | 类型语法紧凑，open-foundry 已验证可行；`apollo-parser` 带精确 span | 表达 action 规则/参数/条件时 directive 膨胀；需自写语义层；与 JSON Schema 生态不直通；用户会误以为有 GraphQL API | `apollo-parser` 0.8.6（2026-08）[已验证] |
| KDL | 人类可读性好，`kdl` 6.x 自带 span + miette 诊断 | 生态小，LLM 训练语料少，编辑器支持弱；无 JSON Schema 等价物 | `kdl` 6.7.1（2026-05，160 万 recent dl）[已验证] |
| Rust macro DSL | 编译期类型安全 | 非 Rust 用户（含 agent、UI）无法编辑；运行时不可加载/热更新；与 "ontology-as-data" 冲突 | N/A |

**推荐**：YAML 为主作者格式，JSON 为等价机器格式（同一 serde 模型），两者都只反序列化到唯一的 IR（`fo-schema`）。
**理由**：
- 参考仓库共识是「作者面 → IR → 后端」三层，IR 必须 JSON-safe（`ref-open-ontology.md` §9.1）。YAML/JSON 天然就是 IR 的序列化，不需要第二套 AST（避免 open-foundry「AST 与运行时 schema 两套」的问题）。
- line:col 错误：`serde-saphyr` 提供带位置的反序列化错误并有 `miette` 集成 [已验证 feature 列表]；语义 lint（ONT-E/W 规则）需要把 IR 节点映射回 span，做法是解析阶段保留 `Spanned<T>` 侧表（path → span）。[未验证：`serde-saphyr` 是否支持任意节点 span 查询，需 1 天 spike；fallback 是 `saphyr` 0.2.2 低层 API 自建 path→marker 表]
- 不选 `serde_yaml`（作者标 deprecated）、`serde_yml`（0.0.x，曾有维护争议）。
- 后续若要 UI/TS DSL 作者面（Foundry 的 ontology-as-code TS DSL），也只需输出同一 JSON IR。

> `NEEDS_USER_DECISION` #1 定义格式：选项 (a) YAML+JSON【推荐】/ (b) GraphQL SDL 方言 / (c) KDL。这是对外 "ontology-as-code" 契约，发布后迁移成本高。

## 2. 存储

| 选项 | 优点 | 缺点 | 成熟度 |
|------|------|------|--------|
| PostgreSQL via `sqlx`，每 object type 一张表 | 列类型强约束、索引/统计最好、SQL 可读 | 每次 schema 变更 = DDL 迁移（含 rename/类型变更），branch 时要多套表；动态 SQL 拼表名需白名单 | `sqlx` 0.9.0（2026-05，仓库迁至 `transact-rs/sqlx`，17.5k star）[已验证] |
| PostgreSQL 通用 JSONB 属性表 | schema 变更免 DDL；branch / edit-log overlay 简单；表达式索引 + GIN 补性能 | 类型约束上移到应用层；大规模聚合比列存慢 | 同上 |
| 嵌入式 SQLite（`sqlx` sqlite / `rusqlite`） | 单二进制 dev 模式、Windows CI 零依赖；有 JSON1、FTS5、partial index、递归 CTE | 单写者；与 PG 方言差异需 conformance 兜底 | `rusqlite` 0.40.2（2026-08）、`sqlx` `sqlite-bundled` feature [已验证] |
| KV 嵌入式（`redb` / `fjall`） | 纯 Rust、快 | 需自建索引、查询、事务语义、遍历，等于自写数据库 | `redb` 4.4.0、`fjall` 3.1.12，均活跃 [已验证] |
| DataFusion over Parquet | 分析型聚合强，可下推 | 不是 OLTP：事务写入、唯一约束、基数强制都要自建；编译体积大 | `datafusion` 55.2.0（2026-10）[已验证] |

**推荐 MVP 方案**：`StorageProvider` trait + `capabilities()` + 两个后端 `fo-storage-sqlite`（dev / 嵌入 / 测试）与 `fo-storage-postgres`（生产），两者都用 `sqlx` 0.9（统一 async API，同一迁移工具），一套 `fo-storage-conformance` 测试同时跑两者。

数据模型（两后端同构）：
- `objects(object_type, pk, properties JSON/JSONB, _version, _created_at, _updated_at, _deleted_at, PRIMARY KEY(object_type, pk))`；被声明 `indexed` / `unique` 的属性由后端生成表达式索引（PG `((properties->>'x'))`，SQLite 同理）。
- `links(link_type, source_pk, target_pk, properties, _deleted_at)`；1:1 / N:1 基数用 **partial unique index**（`WHERE _deleted_at IS NULL`，PG 与 SQLite 均支持 [已验证：两者都支持 partial index 语法]），M:N 用普通唯一键。
- `edits`（append-only edit log：action_rid、actor、command_id 幂等键、schema_version、diff）+ `object_history`（每次写入一行快照）支撑 `asOf`；edit log 格式 MVP 定死，供 v2 scenario / branch 做 overlay（`foundry-docs-spec.md` MVP #4）。
- 遍历（search around / 多跳）：`WITH RECURSIVE` 两后端都有；MVP 限定跳数上限。

**理由**：
- 通用 JSON 表 vs 每类型一张表：ontology 是运行时动态 schema，MVP 阶段 schema 频繁变动，通用表让「schema 变更 = 元数据变更」，branch/overlay 也简单；性能瓶颈出现时可在 PG 后端加「热点类型物化为列表」作为后端内部优化，不影响 trait。
- Windows 测试：本机 `docker version` 报 `failed to connect to the docker API at npipe:////./pipe/dockerDesktopLinuxEngine`（Docker Desktop 未运行）[已验证]。`testcontainers` 0.28 依赖 Docker（Windows 上需 Docker Desktop + WSL2 Linux 容器），不适合作为 Windows 本地默认路径。建议：
  - Windows 本地 PG 测试用 `postgresql_embedded` 0.21.0（下载/打包真 PG 二进制，README 写明支持 Windows）[已验证]，`--features pg-tests` 开关；
  - Linux CI 用 GitHub Actions `services: postgres`（或 testcontainers）；
  - SQLite conformance 永远默认开启，保证 `cargo test` 在 Windows 无外部依赖全绿。
- 不选 redb/fjall/DataFusion 作主存储：要自建事务约束与查询引擎，违背 simplicity；DataFusion 可作为 v2 "外部 Parquet 数据源只读 object type" 的后端。

> `NEEDS_USER_DECISION` #2 SQLite 支持等级：(a) 仅 dev/test/单机演示，不承诺生产【推荐】/ (b) 正式支持的嵌入式生产后端（需承诺迁移与兼容）/ (c) 不做 SQLite，Postgres-only（Windows 测试走 `postgresql_embedded`）。影响对外承诺与测试矩阵。

## 3. 查询层与全文搜索

| 选项 | 优点 | 缺点 | 成熟度 |
|------|------|------|--------|
| 自写 ObjectSet AST → 手拼 SQL | 完全掌控 | 两种方言的引号/参数/JSON 运算符自己处理，注入面大 | — |
| 自有 AST → compiler → `sea-query` | 方言抽象（PG/SQLite）、参数化绑定、支持 CTE / 窗口；有 `sea-query-sqlx` 0.9.1（依赖 `sqlx ^0.9`、`sea-query ^1.0`）[已验证] | JSON 路径运算需部分 `Expr::cust`；多一层依赖 | `sea-query` 1.0.2（2026-08，1.7k star）[已验证] |
| DataFusion LogicalPlan | 统一下推、优化器、跨源 | 不能直接写 OLTP 事务；SQL 方言 unparser 对 SQLite/PG JSON 支持不完整 [未验证]；编译慢 | `datafusion` 55.2.0 [已验证] |
| 全文：后端原生（SQLite FTS5 / PG tsvector） | 与写入同事务，action 后立即可搜（read-your-writes） | 两后端分词/打分语义不同 | 成熟 |
| 全文：`tantivy` | 纯 Rust、Windows 可用、语义跨后端一致、支持 fuzzy/phrase | 与主库是两份数据，需异步同步，失去事务一致性 | `tantivy` 0.26.2（2026-09，16k star）[已验证] |

**推荐**：ObjectSet AST 是对外 wire 契约（对齐 Foundry v2 filter/aggregate 形状），必须自有；compiler 产出 `sea-query` 语句，由 `fo-storage-*` 选择方言执行。全文 MVP 用后端原生，conformance 只断言共同子集（`containsAllTerms`、`containsAnyTerm`、前缀）；`tantivy` 作为 v2 独立 search index provider（与 vector/KNN 一起）。
**理由**：AST 自有 + sea-query 只负责「SQL 文本与绑定」，符合 `ref-bkn-foundry.md`「SQL 生成是唯一拼文本处」的不变量；DataFusion 留给 v2 外部数据源。

## 4. 表达式语言（submission criteria / validations）

| 选项 | 优点 | 缺点 | 成熟度 |
|------|------|------|--------|
| CEL（`cel` crate） | 非图灵完备、无副作用、可预估成本；K8s/Envoy/Firebase 同款，LLM 熟悉；跨语言（TS/Go 侧可复用同一表达式） | Rust 实现静态类型检查能力有限 [未验证：仓库有 `common/decls.rs`、`env.rs`，是否可做声明式类型检查需 spike] | `cel` 0.15.0（2026-10-06，370 万 recent dl，693 star）；旧名 `cel-interpreter` 停在 0.10.0（2025-07）[已验证] |
| `rhai` | 功能全、文档好、可设操作数上限 | 图灵完备脚本，安全与可分析性差；不跨语言 | `rhai` 1.26.1（2026-09，5.7k star）[已验证] |
| 自研最小条件树 | 可完全静态分析、可生成 UI | 表达力增长后会长成一门语言 | — |

**推荐**：CEL（`cel` 0.15），注意依赖名用 `cel` 而非 `cel-interpreter`。schema 里 submission criteria 以结构化条件（Foundry 风格：参数 / 当前用户 / 对象属性比较）为主，编译成 CEL 求值；高级场景允许直接写 CEL 字符串。
**理由**：`ref-open-foundry.md` §12.7 已指出进程内 CEL 是相对 TS 实现的结构优势；条件 → CEL 单一求值路径，避免两套语义。lint 阶段对 CEL 做「引用的参数/属性存在」检查（AST 遍历即可，不依赖完整类型检查器）。

## 5. HTTP API

| 选项 | 优点 | 缺点 | 成熟度 |
|------|------|------|--------|
| `axum` + `utoipa` (+ `utoipa-axum`) | 生态最大；tower 中间件与 `rmcp` streamable HTTP 共享；utoipa 输出 OpenAPI 3.1 | 宏注解与 handler 有重复声明 | `axum` 0.8.9（1.3 亿 recent dl）、`utoipa` 6.0.0 / `utoipa-axum` 0.3.0（2026-09-22）[已验证] |
| `axum` + `aide` | 从类型推导，重复少 | 用户量小一个量级 | `aide` 0.15.1（2026-04）[已验证] |
| `poem-openapi` | spec-first 体验好 | 最近 release 2025-07，生态小，与 tower 生态割裂 | 5.1.16 [已验证] |
| GraphQL `async-graphql` | dynamic schema 可从 ontology 生成 | MVP 已有 REST + SDK + MCP 三个表面，再加一个增加授权面与测试面 | 7.2.1（2026-04）[已验证] |

**推荐**：`axum` 0.8 + `utoipa` 6 + `utoipa-axum`；REST 路径与 JSON 形状对齐 Foundry v2（`foundry-docs-spec.md` 1.12）；MVP 不做 GraphQL，v2 视需求用 `async-graphql` dynamic schema。
**理由**：同一 tower 栈承载 REST、MCP（streamable HTTP）、auth 中间件、tracing；OpenAPI 是 TS SDK 的输入。

## 6. MCP server

| 选项 | 优点 | 缺点 | 成熟度 |
|------|------|------|--------|
| `rmcp`（官方 `modelcontextprotocol/rust-sdk`） | 官方；实现 2026-07-28 规范并兼容 2025-11-25；`#[tool]` 宏 + `schemars`；transports：stdio、streamable HTTP server（含 session）、child-process；带 OAuth 相关 feature | 迭代快（3.x），API 可能继续变 | `rmcp` 3.5.1（2026-10-05，1980 万 recent dl，4k star，rust_version 1.88）[已验证] |
| 社区 SDK / 自写 JSON-RPC | — | 跟规范成本高 | 不推荐 |

**推荐**：`rmcp` 3.5，features `server`、`macros`、`schemars`、`transport-io`（stdio）、`transport-streamable-http-server`。`fo-mcp` crate 只做薄封装：describe ontology、object set 查询、每个 action type 动态生成一个 tool（JSON Schema 来自 action 参数定义）。stdio 给本地 agent，streamable HTTP 挂在 `fo-server` 的 `/mcp` 路由上共享鉴权。
**理由**：MVP #6 要求 MCP 是一级 API；工具是动态的（随 ontology 变化），需用 `rmcp` 的手写 `ServerHandler`（`list_tools` / `call_tool`）而非仅静态宏 [未验证：动态 tool 列表的推荐写法，spike 确认]。锁定 minor 版本。

## 7. Functions 运行时（v2，现在只定形状）

| 选项 | 优点 | 缺点 | 成熟度 |
|------|------|------|--------|
| `wasmtime` component model（WASI 0.2，0.3 async） | 强沙箱、fuel/epoch 限时、WIT 定义类型化签名、多语言（Rust/TS via jco/componentize-js/Python） | 作者工具链门槛较高；编译依赖大 | `wasmtime` 49.0.3（2026-10-14，rust_version 1.96，18.7k star）[已验证]；WASI 0.3 据第三方博客于 2026-06 批准 [未验证一手来源] |
| 嵌入 JS（`rquickjs` / `boa`） | 贴近 Foundry TS Functions 体验 | 需 TS 转译；沙箱/资源限制要自建；`rquickjs` 依赖 C 编译（Windows 需 MSVC，可用）| `rquickjs` 0.14.0、`boa_engine` 0.22.0 [已验证] |
| 进程外 HTTP functions | 任意语言、最简单、与 Foundry「compute module」类似 | 网络延迟、需鉴权回调、部署多一组件 | — |
| `extism`（基于 wasmtime 的插件框架） | 上手快 | 非 component model，类型签名弱 | 1.41.0 [已验证] |

**推荐**：MVP 只建 `fo-function` crate 定义 `FunctionRuntime` trait（输入/输出用 ontology 类型、显式声明读取的 object set 与可产生的 edits），并在 schema IR 预留 `function` 声明。v2 实现顺序：(1) HTTP remote 适配器（最小可用）；(2) `wasmtime` component 运行时（WIT 包 `fo:function`）。
**理由**：先固定契约，运行时可替换；component model 是长期最稳的沙箱方案。

> `NEEDS_USER_DECISION` #3（v2 前决定即可）函数作者语言优先级：(a) 语言无关（WASM component + HTTP）【推荐】/ (b) TypeScript 优先（嵌入 QuickJS，贴近 Foundry 开发者体验）/ (c) 仅 Rust。

## 8. AuthZ / AuthN

| 选项 | 优点 | 缺点 | 成熟度 |
|------|------|------|--------|
| 自研 RBAC + 关系派生检查 | 精确对齐 MVP #7（ontology 级/资源级 role、action 执行权限）；可做「先过滤后分页」不变量；无外部进程 | 行/列级与复杂 ABAC 要逐步扩 | — |
| OpenFGA（Zanzibar） | ReBAC 成熟、可扩展 | 需独立 Go 服务，破坏单二进制；Rust 客户端非官方（`openfga-client` 0.6.1，社区维护）[已验证] | 服务端 6k star 活跃 |
| `cedar-policy` | Rust 原生、进程内、策略可形式化分析、支持 ABAC | 学习成本；策略与 ontology schema 需映射 | 4.13.0（2026-09，1.8k star，活跃）[已验证] |
| `casbin` | 模型灵活，BKN 参考实现用过 | 字符串模型 DSL 难做静态检查与解释 | 2.20.0（2026-02，仓库已迁 `apache/casbin-rs`）[已验证] |
| `oso`（开源库） | 曾经体验好 | 开源库最近 release 2024-01、repo 最近 push 2025-02，已转向 Oso Cloud [已验证日期] | 不推荐 |

**推荐**：`fo-authz` 定义 `Authorizer` trait（`check(principal, action, resource)` + `filter_object_set(principal, set)` 返回谓词下推），MVP 实现自研 RBAC（role 绑定存储在同库，role 可从 link 关系派生），dev 模式也开启授权（反 open-foundry 的 allow-all 默认）。v2 行级/属性级安全优先评估 `cedar-policy` 作为第二实现。
**AuthN**：`jsonwebtoken` 11.1.0（2026-09，5800 万 recent dl）[已验证] 校验 OIDC access token，自写 JWKS 拉取 + 缓存 + `kid` 轮换（约 100 行，`reqwest`）；`openidconnect` 4.0.1 面向 RP 登录流，服务端只校验 token 时过重；`jwt-authorizer` 最近 release 2024-08 [已验证]，不选。dev 模式支持静态 API token。

## 9. 客户端 SDK 生成

| 选项 | 优点 | 缺点 | 成熟度 |
|------|------|------|--------|
| OpenAPI → TS（`openapi-typescript` + `openapi-fetch`） | 仅生成类型，运行时 ~6KB；跟 utoipa 3.1 输出直通 | 只有平台级类型，`properties` 是泛型 map，无 ontology 级强类型 | `openapi-typescript` 7.13.0、`openapi-fetch` 0.17.0（2026-06）[已验证] |
| `@hey-api/openapi-ts` / `orval` | 生成完整 SDK 函数、React Query 等 | 生成代码量大；`@hey-api` 仍 0.x | 0.99.0 / 8.43.0（2026-10）[已验证] |
| 按 ontology schema 自研 typed codegen（OSDK 式） | 对象/属性/link/action 参数全强类型，核心差异化 | 自研成本（模板 + 测试） | — |
| Rust client via `progenitor` | Oxide 维护 | README 声明面向 OpenAPI 3.0.x，而 `utoipa` 输出 3.1.0 [已验证]，有不兼容风险 | 0.15.1 [已验证] |

**推荐 MVP**：
- 平台层：`fo-api`（共享 DTO crate，`serde` + `utoipa::ToSchema`）同时被 server 与 Rust client 使用；Rust client `fo-client` 手写薄封装（`reqwest`），类型与服务端零漂移，不引入 progenitor。
- TS 平台层：`openapi-typescript` + `openapi-fetch` 生成 `@<scope>/client`。
- Ontology 层：`fo-codegen`（Rust，读 IR）生成 TS（MVP）与 Rust（后续）类型化包装：`ObjectType` 接口、属性枚举、link 访问器、action 参数类型，薄包装平台层 client。CLI：`fo codegen ts --ontology ./ontology`。
**理由**：MVP #5 要求生成的 Rust 与 TS 客户端；平台层用成熟工具，强类型层自研贴合我们自己的 IR，也是相对 open-foundry（TS SDK 占位）的差异化。

## 10. Workspace 布局

前缀暂用 `fo-`（占位，见 `NEEDS_USER_DECISION` #4）。只使用 Foundry 官方词汇（ontology / object type / link type / action type / object set / function / interface / branch）。

```
Cargo.toml                    # [workspace] resolver = "3", edition 2024, rust-version = "1.98"
crates/
  fo-core/                    # 基础类型：Rid、ApiName、PrimaryKey、PropertyValue、错误码、时间
  fo-schema/                  # Ontology IR：ObjectType / PropertyType / LinkType / ActionType / Interface / FunctionDecl / ValueType(预留)；serde + schemars
  fo-schema-loader/           # YAML/JSON 加载、span 表、JSON Schema 校验、多文件合并、checksum
  fo-lint/                    # 语义校验 + lint 规则（E/W/I），miette 诊断，--json 输出
  fo-expr/                    # CEL 封装：编译、引用检查、求值上下文（params / currentUser / object）
  fo-object-set/              # ObjectSet AST（filter/search/pivot/set-ops/aggregate/sort/page），wire 格式
  fo-query/                   # ObjectSet → sea-query 编译器，方言无关
  fo-storage/                 # StorageProvider trait + Capabilities + 事务抽象
  fo-storage-sqlite/          # sqlx sqlite 后端
  fo-storage-postgres/        # sqlx postgres 后端
  fo-storage-conformance/     # 共享 conformance 测试套件（dev-dependency 方式被两后端调用）
  fo-action/                  # Action 执行：参数校验、submission criteria、rules → edits、edit log、validate-only、batch
  fo-authz/                   # Authorizer trait + RBAC 实现
  fo-authn/                   # OIDC JWT 校验、JWKS 缓存、dev token
  fo-function/                # FunctionRuntime trait（v2 实现：remote / wasm）
  fo-api/                     # REST DTO（与 Rust client 共享）+ utoipa schema
  fo-server/                  # axum 路由、OpenAPI 导出、中间件、/mcp 挂载
  fo-mcp/                     # rmcp ServerHandler：describe / query / action tools；stdio + HTTP
  fo-client/                  # Rust 客户端（reqwest + fo-api）
  fo-codegen/                 # 按 ontology 生成 TS（及 Rust）typed SDK
  fo-cli/                     # bin `fo`：validate / lint / serve / migrate / codegen / mcp-stdio
  fo-test-support/            # fixture ontology、测试数据构造、insta 设置
packages/
  client-ts/                  # openapi-typescript 生成的平台层 TS client
examples/ontologies/          # 示例 ontology（同时作为 lint/conformance 固定输入）
```

依赖方向：`core ← schema ← (loader, lint, expr, object-set) ← query ← storage-* ← action ← server/mcp ← cli`；`fo-api` 只依赖 `core`/`object-set`，`fo-client` 不依赖 server 侧任何 crate。

通用 crate（版本为 2026-10-15 crates.io 最新稳定版）[已验证]：

| 用途 | 选择 | 说明 |
|------|------|------|
| 库错误 | `thiserror` 2.0.21 | 每个库 crate 一个 `Error` enum；对外 API 错误映射到 Foundry 风格 `errorCode/errorName/parameters` |
| 应用错误 | `anyhow` 1.0.104 | 仅 `fo-cli` / 测试 |
| 诊断 | `miette` 7.6.0 | schema/lint 的 line:col 报告（最近 release 2025-04，仍是事实标准） |
| 日志追踪 | `tracing` 0.1.44 + `tracing-subscriber` | OpenTelemetry 导出留 v2 |
| 配置 | `figment` 0.10.19 或 `config` 0.15.27 | 推荐 `config`（2026-09 仍在发版）；`figment` 最近 release 2024-05 |
| 迁移 | `sqlx` migrate | 系统表迁移；ontology 变更不走 DDL（通用表设计） |
| 快照测试 | `insta` 1.49.0 | lint 输出、生成 SQL、OpenAPI、codegen 产物 |
| 性质测试 | `proptest` 1.12.0 | ObjectSet 编译器 / 集合运算代数律 / SQLite 与 PG 结果等价 |
| CLI | `clap` 4.6.7（derive） | — |
| 异步运行时 | `tokio` | sqlx / axum / rmcp 共同要求 |

> `NEEDS_USER_DECISION` #4 项目与 crate 命名：crates.io 名称发布后不可回收；"Foundry" 是 Palantir 产品名，开源项目对外名称与 crate 前缀含 "foundry" 有商标风险 [未做法律核查]。选项：(a) 中性前缀（如 `fo-` 或新品牌名）【推荐，先占位 `fo-`】/ (b) `foundry-ontology-*` 全名 / (c) 另起品牌名后统一替换。

## 11. 需 spike 验证的未决技术点（不需用户决策）

1. `serde-saphyr` 能否按 IR 路径取 span（决定 lint 精确定位实现方式），1 天。
2. `cel` 0.15 是否支持声明式变量类型检查；否则只做引用存在性检查，0.5 天。
3. `rmcp` 3.5 动态 tool 列表 + streamable HTTP 挂入 axum 的写法，0.5 天。
4. `sea-query` 对 PG `jsonb ->>` 与 SQLite `json_extract` 的表达式与表达式索引生成，0.5 天。
5. `postgresql_embedded` 在本机 Windows 首次下载/启动耗时与 CI 缓存方案，0.5 天。
