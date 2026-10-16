# 参考项目调研：tuannvm/mcp-trino

> 调研日期：2026-10-16。方式：`gh api` 只读拉取 metadata / tree / 文件内容（base64 解码到临时目录），未 clone、未执行其任何代码。只借设计，不复制代码（项目 License 约束）。
> 仓库：https://github.com/tuannvm/mcp-trino （default branch `main`）
> 上游决策：`docs/designs/trino-engine-sql-tiers.md`「SQL 级别」「CEO Review 决策」；Trino client 选型见 `.planning/research/spike-01-trino-client.md`。

标记约定：`[verified: 来源]` = 本次会话用工具读过原始来源（路径均相对 mcp-trino 仓库根，行号对应 main @ `3e0be007`）；`[unverified]` = 推断或未核实。

## 0. 概况

| 项 | 值 |
|---|---|
| 定位 | "A high-performance Model Context Protocol (MCP) server for Trino implemented in Go." [verified: GitHub repo description] |
| Stars / License | 122 / MIT [verified: `gh api repos/tuannvm/mcp-trino`] |
| 最近 | main 最后 commit 2026-04-16 `refactor(secret): remove in-process secret resolver ...`；最新 release v4.3.1（2026-04-01）[verified: commits / releases API] |
| 依赖 | Go 1.25.9；`mark3labs/mcp-go` v0.43.1、`trinodb/trino-go-client` v0.328.0、`tuannvm/oauth-mcp-proxy` v1.0.1 [verified: go.mod] |
| 规模 | 核心只有 4 个文件：`internal/config/config.go`（288 行）、`internal/trino/client.go`（701 行）、`internal/mcp/server.go`（289 行）、`internal/mcp/handlers.go`（345 行）；另有 CLI / REPL（`internal/cli/`）、Helm chart、Claude plugin 打包 [verified: git tree] |

结论：结构小而清楚，适合作为 L2 工具面与配置面的参照；安全实现以正则为主，不能照搬（见第 4 节）。

## 1. 架构

三层：配置层（env → `TrinoConfig`）→ Trino client 层（`database/sql` + trino-go-client + 自定义 `http.RoundTripper`）→ MCP 层（mcp-go server、6 个 tool handler、可选 OAuth middleware）。`cmd/main.go` 按 `--mcp` / `--cli` / `MCP_PROTOCOL_VERSION` / `MCP_TRANSPORT` / 是否 TTY 决定进 MCP server 还是 CLI [verified: cmd/main.go:21-104]。

```
 MCP client (stdio | HTTP POST /mcp, /sse)
        │  Authorization: Bearer <JWT>   (仅 HTTP + OAUTH_ENABLED)
        ▼
 server.go  createMCPHandler: CORS *, 无 Bearer → 401 + resource_metadata
        │  oauth.CreateHTTPContextFunc → ctx 带 OAuth user
        ▼
 mcp-go MCPServer ── WithToolHandlerMiddleware(oauth middleware) 校验 token
        ▼
 handlers.go  6 个 handler：解析 args → (可选) prepareImpersonationContext
        ▼
 client.go  ExecuteQueryWithContext
   ├─ TrimSuffix ';'  → isReadOnlyQuery (正则)  ✗→ "security restriction" 错误
   ├─ context.WithTimeout(TRINO_QUERY_TIMEOUT)
   ├─ NamedArgs: X-Trino-Client-Tags / Client-Info = OAuth 用户名，(可选) X-Trino-User
   ├─ sql.DB.QueryContext ──► trino-go-client ──► headerRoundTripper(X-Trino-Source, X-Trino-User)
   │                                               └─► Trino  POST /v1/statement, GET nextUri
   └─ rows → []map[col]value，超 TRINO_MAX_ROWS 截断并 rows.Close()
        ▼
 handler: json.MarshalIndent → TextContent（截断时附 structuredContent）
```

启动时 `NewClient` 做 `db.Ping()`，再 `ListCatalogs` 做连通自检，失败直接 `log.Fatalf` [verified: cmd/main.go:106-131；client.go:160-169]。连接池固定 `MaxOpenConns=10`、`MaxIdleConns=5`、`ConnMaxLifetime=5m` [verified: client.go:156-158]。

## 2. 六个工具

工具定义见 [verified: handlers.go:299-345]；行为见各 handler 与 client 方法。所有参数均为 string。

| 工具 | 参数（JSON schema 要点） | 行为 | 注解 |
|---|---|---|---|
| `execute_query` | `query` required | 只读检查后执行；结果截断到 `TRINO_MAX_ROWS` | `destructiveHint: true` |
| `list_catalogs` | 无 | `SHOW CATALOGS`，按 `TRINO_ALLOWED_CATALOGS` 过滤 | `readOnlyHint: true` |
| `list_schemas` | `catalog` optional（缺省 `TRINO_CATALOG`） | `SHOW SCHEMAS FROM <catalog>`，按 `TRINO_ALLOWED_SCHEMAS` 过滤 | `readOnlyHint: true` |
| `list_tables` | `catalog`、`schema` optional（缺省取配置） | `SHOW TABLES FROM <c>.<s>`，按 `TRINO_ALLOWED_TABLES` 过滤 | `readOnlyHint: true` |
| `get_table_schema` | `table` required；`catalog`、`schema` optional；`table` 可写成 `s.t` 或 `c.s.t` 覆盖前两者 | 若配了 `TRINO_ALLOWED_TABLES` 先查表白名单，再 `DESCRIBE c.s.t` | `readOnlyHint: true` |
| `explain_query` | `query` required；`format` optional，枚举 `LOGICAL` / `DISTRIBUTED` / `VALIDATE` / `IO` | 拼成 `EXPLAIN [(TYPE f)] <query>` 后走 `ExecuteQueryWithContext`；非法 format 报错 | `readOnlyHint: true` |

[verified: client.go:440-597；handlers.go:54-294]

**输出形态**（以代码为准）：
- `execute_query`：TextContent = 美化 JSON 数组，每行是 `map[列名]值`；截断时额外返回 structuredContent `{results, truncated: true, rowCount, message}`，未截断时没有 `truncated` 字段 [verified: handlers.go:82-102]。
- `list_*`：TextContent = JSON 字符串数组 [verified: handlers.go:118-125,154-161,193-200]。
- `get_table_schema` / `explain_query`：`DESCRIBE` / `EXPLAIN` 的原始行 map 数组 [verified: handlers.go:242-249,286-293]。
- `docs/tools.md` 写的 `{"columns", "data"}`、`{"catalogs": [...]}` 等形态与代码不一致，文档已过时 [verified: docs/tools.md 对照 handlers.go]。

**行 map 的副作用**：行用 Go map 承载，`SELECT a.id, b.id` 这类重名列会互相覆盖，且列顺序丢失（`encoding/json` 对 map key 排序）[verified: client.go:409-414；map key 排序为 Go 标准库行为，unverified 于本仓库测试]。

**错误处理**：所有失败都以 `mcp.NewToolResultErrorFromErr`（`isError` 的 tool result）返回，不走 JSON-RPC 错误；消息为 `query execution failed: <Trino 原始错误>` 等，原样暴露给 agent [verified: handlers.go:76-80 等]。`rows.Scan` 失败只 `log` 后 `continue`，该行被静默丢弃 [verified: client.go:404-407]。连接错误会把密码替换为 `[PASSWORD_REDACTED]` [verified: client.go:599-618]。

## 3. 配置面

全部来自环境变量，代码默认值为准 [verified: config.go:60-231；server.go:110-116,258-268；cmd/main.go:142-151]。

| Key | 代码默认 | 说明 |
|---|---|---|
| `TRINO_HOST` / `TRINO_PORT` | `localhost` / `8080` | |
| `TRINO_USER` / `TRINO_PASSWORD` | `trino` / 空 | 唯一的 Trino 认证凭据（DSN userinfo） |
| `TRINO_CATALOG` / `TRINO_SCHEMA` | `memory` / `default` | 工具参数缺省值 |
| `TRINO_SCHEME` / `TRINO_SSL` | `https` / `true` | scheme 为 https 时强制 SSL |
| `TRINO_SSL_INSECURE` | **`true`** | 默认跳过证书校验 |
| `TRINO_ALLOW_WRITE_QUERIES` | `false` | 打开即跳过只读检查 |
| `TRINO_QUERY_TIMEOUT` | `300`（秒） | `docs/deployment.md:240` 写 30，文档过时 |
| `TRINO_MAX_ROWS` | `10000` | `0` = 不限 |
| `TRINO_ALLOWED_CATALOGS` / `_SCHEMAS` / `_TABLES` | 空 | 逗号分隔；schema 必须 `c.s`，table 必须 `c.s.t`（按点数校验） |
| `TRINO_ENABLE_IMPERSONATION` | `false` | |
| `TRINO_IMPERSONATION_FIELD` | `username` | `username` / `email` / `subject` |
| `TRINO_SOURCE` | `mcp-trino/<version>` | `X-Trino-Source` |
| `MCP_TRANSPORT` / `MCP_PORT` / `MCP_HOST` / `MCP_URL` | `stdio` / `8080` / `localhost` / 由前三者拼 | |
| `HTTPS_CERT_FILE` / `HTTPS_KEY_FILE` | 空 | 都设才走 TLS |
| `OAUTH_ENABLED` / `OAUTH_MODE` / `OAUTH_PROVIDER` | `false` / `native` / `hmac` | provider：`hmac` / `okta` / `google` / `azure` |
| `JWT_SECRET` | 空 | HMAC 签名、proxy 模式 state 签名 |
| `OIDC_ISSUER` / `OIDC_AUDIENCE` / `OIDC_CLIENT_ID` / `OIDC_CLIENT_SECRET` | 空 | audience 无默认值 |
| `OAUTH_ALLOWED_REDIRECT_URIS`（旧名 `OAUTH_REDIRECT_URI`） | 空 | 单值 = fixed 模式，逗号列表 = allowlist 模式 |
| `MCP_PROTOCOL_VERSION` | — | 存在即进 MCP 模式 |

## 4. 安全机制与弱点

### 4.1 只读：正则，不是 parser

`isReadOnlyQuery` [verified: client.go:194-279]：小写化 → 把单引号字面量换成 `'LITERAL'`、双引号 / 反引号标识符换成占位、再删 `--` 与 `/* */` 注释 → 含 `;` 拒绝 → 开头须为 `select` / `show` / `describe` / `explain` / `with` → 全文不得出现 18 个写关键字（`insert` … `call`、`execute`、`refresh`、`set`、`reset`，`\b` 词边界）；`SHOW CREATE ...` 特判放行。

本次用独立重述的同一正则流水线（Python `re`，脚本不含其代码）做了探测 [verified: 本机 `python -I probe.py` 输出]：

| 输入 | 判定 | 含义 |
|---|---|---|
| `SELECT * FROM TABLE(pg.system.query(query => 'DELETE FROM x RETURNING *'))` | 放行 | 字面量被抹掉，**表函数透传完全不在检查范围** |
| `EXPLAIN ANALYZE -- '⏎INSERT INTO t VALUES (1) -- '` | 放行 | 字面量替换先于注释删除：注释里的 `'` 与下一行配对，吞掉 `INSERT`。**解析不同步 → 绕过** |
| `EXPLAIN ANALYZE INSERT INTO t VALUES (1)` | 拒绝 | 直写会命中关键字 |
| `CALL system.execute(query => 'DROP TABLE x')` | 拒绝 | 不以只读前缀开头 + `call` 关键字 |
| `SELECT * FROM secret_catalog.hr.salaries` | 放行 | 只读检查不看 catalog |
| `SELECT update FROM t` | 拒绝 | 误杀：列名撞关键字 |

- `EXPLAIN ANALYZE` 会真实执行语句："Execute the statement and show the distributed execution plan" [verified: trino.io/docs/current/sql/explain-analyze.html]。上表第 2 行若 Trino 接受 `EXPLAIN ANALYZE INSERT`，就是一次真实写入 [unverified：未连 Trino 实测 DML 是否被 EXPLAIN ANALYZE 接受]。
- `explain_query` 自称 "without running expensive queries" 且标 `readOnlyHint`，但 `query = "ANALYZE SELECT ..."` 会拼成 `EXPLAIN ANALYZE SELECT ...` 并通过检查，即执行全量查询 [verified: client.go:583-596 + 只读规则]。
- `system.query` 表函数：文档要求 "The native query passed to the underlying data source is required to return a table as a result set"，"Only use passthrough queries to read data" 是使用建议而非强制 [verified: trino.io/docs/current/connector/postgresql.html]。返回结果集但有副作用的原生 SQL（有副作用的函数、PG 侧可写 CTE）能否经此执行取决于 connector 包装方式 [unverified]。无论如何它绕过 Trino 侧的表级权限语义，只受 connector 数据库账号权限约束。
- `system.execute` 过程可执行 DDL / DML [verified: 同上 PostgreSQL connector 页]；mcp-trino 靠 `call` 关键字挡住，但开 `TRINO_ALLOW_WRITE_QUERIES` 后全部放开。

### 4.2 allow-list：只作用于浏览类工具

- `execute_query` / `explain_query` **完全不检查** allow-list [verified: client.go:325-438 无任何 allowed* 调用]；任何 `SELECT` 可读 allow-list 外的 catalog。项目自己的文档也承认："Not primary security: Don't rely solely on allowlists"、"Bypass possible" [verified: docs/allowlists.md:168-169]。
- `get_table_schema` 只查 `AllowedTables`，不查 catalog / schema 级 [verified: client.go:565-570]；`list_schemas` 不校验传入 catalog 是否在 `AllowedCatalogs` [verified: client.go:473-496]。
- 标识符直接 `fmt.Sprintf` 拼进 `SHOW SCHEMAS FROM %s` / `SHOW TABLES FROM %s.%s` / `DESCRIBE %s.%s.%s`，无引号转义 [verified: client.go:478,513,573]；后续只靠只读正则兜底。
- 因此 view、CTE、子查询都无需"绕过"：执行路径本来就没有 catalog 约束。即使补上文本级检查，view 展开后的真实引用也只有 Trino 能约束（与 ontoforge OV2 结论一致）。

### 4.3 超时与上限

- 超时：`context.WithTimeout(ctx, TRINO_QUERY_TIMEOUT)` [verified: client.go:336]；trino-go-client 的 `driverRows.Close()` 发 `DELETE /v1/query/{id}` 取消查询（不是协议文档的 `DELETE nextUri`）[verified: trinodb/trino-go-client v0.328.0 trino/trino.go:1393-1404]。
- 行上限截断后立即 `rows.Close()`，即服务端取消 [verified: client.go:419-425]。无字节上限、无注入 `LIMIT`、无并发配额、无 resource group 选择。
- 客户端断连是否传导为 ctx 取消，取决于 mcp-go 的请求 ctx 生命周期 [unverified]。

### 4.4 其他

- `/mcp` 响应头 `Access-Control-Allow-Origin: *` [verified: server.go:172]；`/status` 无认证 [verified: server.go:93,218-222]；stdio 模式无任何认证。
- 没有查询审计：handler 只在失败时 `log.Printf`，成功的查询不留记录 [verified: handlers.go 全文]。

## 5. 身份

- **Trino 认证只有一份静态凭据**：DSN 的 `TRINO_USER` / `TRINO_PASSWORD` [verified: client.go:119-133]。MCP 侧的 OAuth token 不会转发给 Trino（未使用 trino-go-client 的 `accessToken` / `ForwardAuthorizationHeader`）[verified: client.go 全文；trino.go:1029-1033 该能力存在但未被调用]。
- **归属标签（attribution，默认随 OAuth 开启）**：每次查询以 NamedArg 传 `X-Trino-Client-Tags` 与 `X-Trino-Client-Info`，值都是 OAuth 用户名（回退 email → subject → `mcp-trino-user`）；`TRINO_SOURCE` 为空时 `X-Trino-Source` 也用用户名 [verified: client.go:283-303,342-359]。trino-go-client 把以 `X-Trino-` 开头的 NamedArg 原样加为该次请求的 header [verified: trino.go:1063-1071]。弱点：client tags 被用户名占用，部署方无法再用 tags 做 resource group 选择；用户名含逗号会被 Trino 拆成多个 tag [unverified]。
- **impersonation（opt-in）**：`TRINO_ENABLE_IMPERSONATION=true` 时按 `TRINO_IMPERSONATION_FIELD` 从 OAuth user 取 principal，同时经 NamedArg 与 `headerRoundTripper` 设 `X-Trino-User` [verified: handlers.go:30-52；client.go:86-108,351-355]。Trino 以服务账号认证、按 `impersonation` 规则判定能否切换用户，失败报 "Access Denied: Cannot impersonate user" [verified: docs/impersonation.md:249-290,480-487]。字段为空时只打 warning，**回退为服务账号执行**（fail-open）[verified: handlers.go:45-49]。
- **OAuth 模式**（均由 `oauth-mcp-proxy` 库实现）[verified: docs/oauth.md:47-140]：
  - `native`：MCP server 作为 resource server，按 `OIDC_ISSUER` 的 JWKS（或 HMAC `JWT_SECRET`）校验 Bearer JWT 与 `OIDC_AUDIENCE`；client 自己向 IdP 取 token。
  - `proxy`：MCP server 代理整个授权码流程（`OIDC_CLIENT_ID/SECRET`、redirect URI fixed / allowlist 两种模式、HMAC 签名 state、PKCE），并暴露 `/.well-known/oauth-authorization-server`、`/.well-known/oauth-protected-resource`、`/oauth/callback`。

## 6. ontoforge 的取舍

### 6.1 原样采用

- 6 个工具名与参数名：`execute_query(query)`、`list_catalogs()`、`list_schemas(catalog?)`、`list_tables(catalog?, schema?)`、`get_table_schema(catalog?, schema?, table)`、`explain_query(query, format?)`，`format` 枚举 `LOGICAL|DISTRIBUTED|VALIDATE|IO`。保证 agent 迁移成本为零（设计文档第 4 节已定）。
- MCP 注解：浏览类 `readOnlyHint`；`execute_query` 在 ontoforge 只读，改标 `readOnlyHint: true`（不再 destructive）。
- 末尾 `;` 去除；错误以 `isError` tool result 返回而非协议错误；`X-Trino-Source` 固定标识来源。

### 6.2 改造后采用（对齐 L2 规则）

| mcp-trino 做法 | ontoforge 做法 |
|---|---|
| 正则只读检查 | `sqlparser-rs` 解析，单语句且只接受 `Query`（SELECT / WITH）；拒绝 `TABLE(...)` 表函数（含 `system.query`）、`CALL`、`EXECUTE`、`EXPLAIN ANALYZE`、会话 / 事务语句；解析失败一律拒绝，错误 `L2Rejected`（设计文档第 4 节、spike 2）。不做关键字黑名单，因而无列名误杀 |
| `explain_query` 字符串拼接 | 先用同一白名单解析 `query`，再由代码构造 `EXPLAIN (TYPE f) <ast>`；不接受 `ANALYZE` |
| allow-list 只过滤列表 | 执行路径同样强制：AST 中每个表引用按「显式 catalog.schema，否则用固定的默认」解析后对 allow-list 校验；edit store catalog 硬编码排除。**AST 只是第一道防线**：view 展开只有 Trino 能约束，L2 硬依赖 Trino 侧 ACL + 启动只读探测，失败禁用 L2（OV2） |
| 标识符 `Sprintf` 拼接 | 先对 allow-list 校验，再按 Trino 规则加双引号转义后拼接（或用 sqlparser AST 渲染） |
| 行上限 `TRINO_MAX_ROWS` | 行 + 字节双上限（4B）；AST 层注入 / 收紧 `LIMIT cap+1`；输出恒含 `truncated` 字段；达上限即 `DELETE nextUri` |
| 行 = map | `{columns: [{name, type}], rows: [[...]], truncated, row_count, request_id}`，保留列序与重名列；单列解码失败降级为字符串（spike 1 §5 B.7），不静默丢行 |
| 归属：Client-Tags = 用户名 | `X-Trino-Client-Tags` 留给 resource group（按服务身份，如 `ontoforge-l2`）；`X-Trino-Client-Info` = 主体 id + 工具名；`X-Trino-Trace-Token` = request_id；`X-Trino-Source` = `ontoforge` |
| 单一服务账号 | 两个服务身份：ontology 身份（L0 / Action 叠加读）与 L2 身份（只读、仅 allow-list）；凭据分开配置 |
| impersonation 取 OAuth 字段，空值回退服务账号 | 仅 L2 可选；principal 取 `ontoforge-auth` 解析后的主体 id，不直接取 JWT 字段；取不到即拒绝（fail-closed）；自检覆盖 impersonation 规则 |
| ctx 超时 | 按工具可配超时 + `query_max_execution_time` session 属性兜底 + 断连取消 + 主体并发配额（OV4） |
| 错误原样透出 | L2 可见原始 Trino 错误（2A），但经统一错误类型（`EngineQueryError` / `EngineTimeout` / `L2Rejected`）包装，凭据脱敏 |
| 无审计 | 每次调用写读取审计（8A）：主体、工具、编译 SQL 哈希、行数、耗时、request_id、Trino query id，不记结果 |
| OAuth native 模式 | 对应 `ontoforge-auth` 的 OIDC resource server（JWKS 校验 + audience），MCP 与 REST 共用 |
| `list_catalogs` 执行 `SHOW CATALOGS` 再过滤 | 返回「allow-list ∩ Trino 实际可见」；`list_tools` 只对持有 `sql:raw` 且 L2 自检通过的主体出现这 6 个工具 |

### 6.3 拒绝

- `TRINO_ALLOW_WRITE_QUERIES`：写 / DDL 属于 L3，不进 agent MCP。
- `TRINO_SSL_INSECURE=true` 默认：ontoforge 默认校验证书，跳过须显式配置并告警。
- `TRINO_CATALOG=memory` 这类隐式默认 catalog：L2 要求表引用可解析到 allow-list，否则拒绝。
- OAuth `proxy` 模式与 `oauth-mcp-proxy`：授权服务器职责不属于 ontoforge；本 milestone 只做 resource server。
- CLI / REPL 与按 TTY 猜模式：ontoforge 已有自己的 CLI 与 `serve` 子命令。
- `Access-Control-Allow-Origin: *`、无认证 `/status`：ontoforge 的 `/health` 按 server 的统一策略。
- `database/sql` + 全局 `RegisterCustomClient` + ctx 塞 header 的做法：ontoforge 自写 client 每请求无状态组装 header（spike 1 方案 B）。

## 7. 组件映射

| mcp-trino 组件 | ontoforge 去处 | 说明 |
|---|---|---|
| `internal/config/config.go` | `ontoforge-server` 的配置模块（`engine.trino.*`、`l2.*` 键名由 eng review 定） | 配置文件为主、env 覆盖；两套身份凭据分开 |
| `internal/trino/client.go` 连接 / 执行 / 截断 | **新 crate `ontoforge-engine`**：引擎 trait + `trino` 模块（reqwest + rustls 薄 client）+ SQLite 替身 | 理由：L0 叠加读（`ontoforge-query`）、Action 前置读（`ontoforge-action`）、L2 三方共用；放进 `ontoforge-query` 会让 AST / 编译 crate 依赖 HTTP 栈与 TLS。若 eng review 否决新 crate，退路是 `ontoforge-query::engine::trino`（spike 1 §6） |
| `isReadOnlyQuery` / allow-list 检查 | `ontoforge-query::l2`（`sqlparser-rs` 白名单 + 表引用解析） | 纯函数、可在 Windows 单测；下一 milestone 的 L1 解析同处 |
| `headerRoundTripper` / attribution / impersonation | `ontoforge-engine::trino`（header 组装）+ `ontoforge-auth`（主体解析、`sql:raw` 授权、L2 allow-list 存储） | 身份决策在 auth，header 只是传输 |
| `internal/mcp/handlers.go`、`server.go` | `ontoforge-server` 的 MCP 模块（`rmcp`），L2 toolset 一个子模块 | toolset 按主体过滤 |
| 结果 JSON 形态 | `ontoforge-api`（L2 结果 DTO、错误码） | REST 与 MCP 共用 |
| 启动连通检查 | `ontoforge-server` 启动流程 + `ontoforge-engine` 自检（L2 ACL 探测） | 失败禁用 L2，不 `Fatal` 整个进程 |
| 读取审计（mcp-trino 无） | 审计记录写入 `ontoforge-storage`（系统数据层） | 8A |

## 8. Open items

1. Trino 是否接受 `EXPLAIN ANALYZE INSERT/CTAS`（决定 4.1 第 2 行是否为真实写入），以及 PG connector `system.query` 对有副作用原生 SQL 的包装方式。对 ontoforge 结论无影响（两者都在白名单外），但影响 conformance 用例的描述。
2. `X-Trino-Client-Tags` 的分隔与转义规则、resource group selector 命中行为，随 spike 1 open item 1 一起在 Linux CI 实测。
3. rmcp 是否支持 tool annotations 与 structuredContent 的当前版本行为，归入 Phase 7 调研。
