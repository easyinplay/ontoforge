# Spike 1：Rust Trino client 选型

**日期：** 2026-10-16
**来源 gate：** `docs/designs/trino-engine-sql-tiers.md`「Gates before /plan-eng-review」第 1 项
**结论：** 自写薄 client（基于 `reqwest` 0.13 + rustls，约 700 行实现 + 约 400 行测试），挂在引擎 trait 之后；不引入 `trino-rust-client` / `prusto`。
**置信度：** MEDIUM（crate 侧事实均已核对源码；Trino 实机行为因本机 Docker daemon 未运行未能实测，列入 open items）

标记约定：`[verified: 来源]` = 本次会话用工具读过原始来源；`[unverified]` = 未在本次会话核实。

## 1. 问题

ontoforge 作为 client 连接 Trino（强制外部计算引擎）。需求来自设计文档：

- 两个固定服务身份（ontology 身份、L2 身份），各自有独立的 `X-Trino-User`、凭据与 resource group 归属；L2 可选按主体透传 `X-Trino-User`（impersonation）。
- OV4：执行时限、resource group（按服务身份）、客户端断连即取消查询。
- 8A：读取审计需要 request_id 与 Trino query id 关联。
- 2A：L0 只返回结构化错误码，原始 Trino 错误只进日志，因此需要结构化解析 `QueryError`。
- 4B：结果按行数 + 字节双上限截断，查询结果规模受控。
- 读取结果要映射到 ontology property 类型（decimal、timestamp with time zone、array、map、row）。

问题：用现成 crate，还是自写 HTTP 协议薄封装？

## 2. Trino client 协议要点（选型依据）

当前文档版本为 Trino 484 [verified: trino.io/docs/current/develop/client-protocol.html]。

- 生命周期：`POST /v1/statement` 提交 SQL，循环 `GET nextUri` 直到响应中没有 `nextUri`；取消查询是对 `nextUri` 发 `DELETE` [verified: 同上]。
- 状态码：502 / 503 / 504 视为间歇故障，50-100 ms 后重试；429 按 `Retry-After` 重试；200、429、502、503、504 以外的状态一律视为查询失败 [verified: 同上]。
- 请求头（文档原文列出的）：`X-Trino-User`、`X-Trino-Original-User`、`X-Trino-Source`、`X-Trino-Catalog`、`X-Trino-Schema`、`X-Trino-Time-Zone`、`X-Trino-Language`、`X-Trino-Trace-Token`、`X-Trino-Session`、`X-Trino-Role`、`X-Trino-Prepared-Statement`、`X-Trino-Transaction-Id`、`X-Trino-Client-Info`、`X-Trino-Client-Tags`（用于识别 resource group）、`X-Trino-Client-Capabilities`、`X-Trino-Resource-Estimate`、`X-Trino-Extra-Credential`、`X-Trino-Query-Data-Encoding` [verified: 同上]。
- 响应头：`X-Trino-Set-Catalog`、`X-Trino-Set-Schema`、`X-Trino-Set-Session`、`X-Trino-Clear-Session`、`X-Trino-Started-Transaction-Id`、`X-Trino-Query-Data-Encoding` 等 [verified: 同上]。
- spooling：client 用 `X-Trino-Query-Data-Encoding` 声明编码（如 `json+zstd,json+lz4,json`），server 不支持则回退 direct 协议；`data` 变为含 segment 的对象，segment 分 `inline`（base64）与 `spooled`（`uri` + `ackUri`），取完须带同样的 headers `GET ackUri` [verified: 同上]。
- `timestamp with time zone` 的 JSON 线格式尾部可以是偏移量（`+08:00`）或 zone id（如 `UTC`、`America/Los_Angeles`）：官方 Python client 按最后一个空格切分，偏移量走 `timezone()`，其余走 `ZoneInfo()` [verified: trinodb/trino-python-client `trino/mapper.py`:153-174]。

## 3. 候选清单

crates.io 搜索 `trino` / `presto`，以及按名直查 `prusto`、`trino-rust-client`、`presto-rs`、`prestodb`、`trino-client`、`trino-rs`、`presto-client` [verified: crates.io API，2026-10-16]。后 5 个名字在 crates.io 不存在 [verified: crates.io API 返回 `crate ... does not exist`]。

| crate | 最新版本 / 日期 | 下载（总 / 近 90 天） | 仓库活跃度 | License | 判定 |
|---|---|---|---|---|---|
| `trino-rust-client` | 0.12.0 / 2026-08-19 | 553,482 / 408,485 | 最后 push 2026-09-21，14 star，7 open issue；fork 自 prusto（2024-12） | MIT | 唯一认真候选，详见 §4 |
| `prusto` | 0.5.2 / 2024-12-06 | 418,270 / 90,130 | 最后 commit 2026-05-13（bearer 支持已合并未发版），48 star，19 open issue | MIT | 淘汰：依赖 `reqwest` 0.11 + `http` 0.2 + `tokio` `full`，与 axum 的 `http` 1.x 栈不兼容会引入两套 HTTP 栈 |
| `dbcrossbar_trino` | 0.2.4 / 2024-12-06 | 5,179 / 35 | 所属仓库 dbcrossbar 最后 push 2026-09-02 | Apache-2.0 OR MIT | 淘汰：docs.rs 自述 "A basic Trino REST API client, useful for testing"；`Client` 只有 `run_statement` / `get_all` / `get_one_row` 等，无认证、无取消、无 builder |
| `trino` | 0.1.0 / 2021-09-10 | 3,207 / 1 | 最后 push 2022-05-17 | Apache-2.0 | 淘汰：废弃 |
| `hamelin_trino` | 0.23.7 / 2026-10-15 | 17,232 / 2,502 | 活跃 | `non-standard` | 淘汰：是 Hamelin 查询语言的 Trino 后端，内部依赖 `prusto`，不是通用 client |
| `presto` | 0.1.2 / 2026-09-27 | 54 / 18 | — | MIT | 无关：描述为 "A simple TUI music player with Vim bindings" |

以上数字与日期 [verified: crates.io API + `gh api repos/<owner>/<repo>`]。`trino-rust-client` 在 crates.io 上公开反向依赖数为 0 [verified: crates.io reverse_dependencies API，`meta.total = 0`]，下载量不代表生态采用度。

## 4. `trino-rust-client` 0.12.0 逐项评估

源码依据：GitHub `nudibranches-tech/trino-rust-client` main @ `bfbc21d934`，以及 crates.io 发布的 0.12.0。

| 维度 | 结论 | 来源 |
|---|---|---|
| async / tokio | 是；依赖 `tokio` 仅 `rt`、`sync` 特性 | [verified: Cargo.toml `tokio = {version = "1.53", features = ["rt", "sync"]}`] |
| MSRV / edition | `rust-version = "1.88"`、`edition = "2021"`，兼容本项目 Rust 1.98 | [verified: Cargo.toml workspace.package] |
| HTTP / TLS | `reqwest = {version = "0.13.4", default-features = false, features = ["rustls", "json"]}`；TLS provider 为 aws-lc-rs（`aws-lc-sys` 0.46.0 进入依赖树） | [verified: Cargo.toml；probe 工程 `cargo tree -i aws-lc-sys`] |
| Windows 构建 | 在本机（Windows 11，rustc 1.98.0）新建 probe 工程依赖 `trino-rust-client = "=0.12.0"`，`cargo build` 成功，耗时 46.20s，含 `aws-lc-sys` | [verified: 本机 probe，输出 `Finished \`dev\` profile ... in 46.20s`] |
| 协议 | `POST /v1/statement` + `nextUri` 轮询；`get` / `get_next` 低层 API、`get_all` 全量、`stream` 流式 | [verified: src/client.rs:655,760,1175,1201] |
| 认证 | `Auth::Basic(String, Option<String>)`、`Auth::Jwt(String)`、`Auth::OAuth2(Arc<OAuth2State>)`（交互式浏览器流程）；HTTP 明文下带认证默认报 `Error::BasicAuthWithHttp` | [verified: src/auth/mod.rs:16-21；src/client.rs:302-304] |
| 身份头 | `X-Trino-User` 取自 `session.user`，在 `ClientBuilder::new(user, host)` 时固定；没有按请求覆盖 user 的 API；未发现 `X-Trino-Original-User` 常量 | [verified: src/client.rs:344-346；src/header.rs 全文] |
| session / tags | `client_tags`、`client_info`、`source`、`trace_token`、`properties`（`X-Trino-Session`）、`roles`、`extra_credentials` 都是 builder 级配置，存在 `RwLock<Session>` 中，一个 Client 共享 | [verified: src/session.rs；src/client.rs:49,133-216] |
| 共享可变 session | 每个响应的 `X-Trino-Set-Catalog` / `Set-Schema` / `Set-Session` / `Clear-Session` / `Set-Role` 写回同一个 `RwLock<Session>`，并发请求共享这份状态 | [verified: src/client.rs:1345-1356] |
| 连接池 | 每个 `Client::build()` 内部新建 `reqwest::ClientBuilder::new()`，不能注入外部 `reqwest::Client` | [verified: src/client.rs:306-319] |
| 取消 | `Client::cancel(query_id)` 发 `DELETE {url}v1/query/{id}`；`RowStream` 未读完被 drop 时在 tokio 上 fire-and-forget 同一个 DELETE。用的是 `/v1/query/{id}`，不是文档规定的 `DELETE nextUri` | [verified: src/client.rs:588-600,684-690,1223-1225] |
| 重试 | `RetryPolicy`（默认 3 次，1s→2s）；GET nextUri 遇 502/503/504 及连接错误重试，POST 只在确定未处理时重试；client 源码中未发现 429 / `Retry-After` 处理 | [verified: CHANGELOG 0.12.0；`rg "429\|Retry-After"` 对 src/client.rs、src/retry.rs 无匹配] |
| 类型解码 | 静态 `#[derive(Trino)]` 或动态 `Row`（`Vec<serde_json::Value>`）；decimal 用 `bigdecimal`；`TimestampWithTimeZone` 解析格式 `"%Y-%m-%d %H:%M:%S%.3f %:z"`，测试 fixture 只有 `+00:00` 偏移量；`TimeWithTimeZone`、HyperLogLog、P4HyperLogLog、QDigest、Geometry 产生 `Error::UnsupportedType` | [verified: src/types/date_time.rs:91-100；tests/data/types/date_time；src/types/mod.rs:180-185] |
| 动态 Row 的类型失败 | `get_all::<Row>` 构造 DataSet 时对每列调 `TrinoTy::from_column`，任一列类型不支持则整个查询返回 `Error::Decode` | [verified: src/types/data_set.rs:72-90] |
| spooling | 可选 feature `spooling`（引入 `zstd`、`lz4`、`flate2`）；默认请求编码 `json+zstd` | [verified: Cargo.toml features；src/session.rs `spooling_encoding: Some("json+zstd")`] |
| 额外依赖 | 无条件依赖 `open` 5.x（OAuth2 打开浏览器用） | [verified: probe `cargo tree --depth 1`] |
| API 稳定性 | 0.10.0（2026-07-18）、0.11.0（2026-07-19）、0.12.0（2026-08-19）三个版本都有 **Breaking** 条目 | [verified: CHANGELOG.md] |
| 近期正确性 / 安全缺陷 | 0.12.0 修复「Transactions were unusable」；main 上 Unreleased 修复「`Client::execute` bypassed authentication on its final result fetch」（PR #72，2026-09-03），尚未发版 | [verified: CHANGELOG.md；`gh api .../commits`] |
| TLS provider 可选性 | open PR #70「Allow opting out of reqwest's aws-lc-rs TLS provider」、#71「selectable rustls crypto provider」未合并 | [verified: `gh api .../issues`] |

`timestamp with time zone` 解码的风险推断：Trino 线格式可带 zone id（§2），而该 crate 只按 `%:z` 偏移量、3 位小数解析，遇到 `UTC` 或 `timestamp(6) with time zone` 大概率解析失败 [unverified：未连真实 Trino 实测]。

## 5. 方案对比

### 方案 A：采用 `trino-rust-client`

优点：活跃维护、MIT、已在本机 Windows 构建通过；已实现 nextUri 轮询、幂等感知重试、drop 即取消、spooling、OAuth2、tracing span 带 query_id。

与 ontoforge 需求的错配：

1. **身份模型是「一个 Client = 一个 session 用户」**。两个服务身份可以各建一个 Client；但 L2 按主体透传 `X-Trino-User` 时要每主体建一个 Client，每个 Client 自带独立连接池与 TLS 握手，只能再套一层 LRU 缓存。
2. **按请求的 session 属性 / client tags 无法覆盖**。若要按工具设置 `query_max_execution_time` 或不同 client tags，只能多建 Client 或改共享 session（并发竞态）。
3. **共享可变 session**：任何一条 SQL 触发的 `X-Trino-Set-Session` / `Set-Catalog` 会改写同一 Client 后续所有请求的状态。L2 白名单拒绝会话语句后风险下降，但 L3（`sql:admin`，任意 SQL）若复用同一 Client 会污染其他请求。
4. **类型解码基本用不上**：ontoforge 要把 Trino 值映射到自己的 property 类型，crate 的静态 derive 不适用于运行期 ontology；动态 `Row` 路径又会因单列不支持类型让整条查询失败（L2 原始 SELECT 尤其容易碰到）。
5. **429 未处理、取消走 `/v1/query/{id}` 而非 `DELETE nextUri`**，与协议文档有偏差。
6. **API 抖动与缺陷密度**：两个月三次破坏性变更，近期有「事务完全不可用」「execute 不带认证」这类基础缺陷，说明测试覆盖面薄。
7. 实际能复用的部分只有提交 / 轮询 / 重试 / 模型反序列化，约 400-600 行量级。

### 方案 B：自写薄 client（推荐）

基于 `reqwest`（与 `trino-rust-client` 同为 0.13 + rustls，构建成本已在本机验证）实现。必须处理的协议细节：

1. `POST /v1/statement`，body 为 SQL 文本；之后循环 `GET nextUri`，`nextUri` 原样使用（绝对 URI，经网关 / 反代时不得改写 host）。
2. 每请求无状态地组装 header：`X-Trino-User`（服务身份或透传主体）、`X-Trino-Source`（固定 `ontoforge`）、`X-Trino-Client-Tags`（resource group 选择）、`X-Trino-Client-Info`、`X-Trino-Trace-Token`（填 ontoforge request_id，满足 8A 关联）、`X-Trino-Session`（name=value，值需 URL 编码 [unverified：编码规则需对照 Java client]）、`X-Trino-Catalog` / `X-Trino-Schema`、`X-Trino-Time-Zone`（固定 `UTC` 保证结果确定性）、`X-Trino-Client-Capabilities`。
3. 认证：Basic（仅 HTTPS）与 Bearer JWT；rustls + 可配置根证书；凭据只在服务端配置。
4. 状态码：429 读 `Retry-After`；502/503/504 短退避重试（仅 GET nextUri 无条件重试，POST 仅在确定未被处理时重试）；其他状态视为失败。
5. 解析 `QueryResults`：`id`、`infoUri`、`nextUri`、`columns`（可能在若干页之后才出现）、`data`（可跨多页、可为空页）、`error`（`errorCode` / `errorName` / `errorType` / `message` / `errorLocation`）、`stats.state`、`updateType`、`warnings`。错误解析直接产出 2A 需要的结构化错误码，原文进日志。
6. 取消：`DELETE nextUri`；挂在三个触发点：客户端断连（axum handler future 被 drop）、ontoforge 执行时限到期、行数 / 字节上限达到后提前结束（4B）。另以 `query_max_execution_time` session 属性作为 Trino 端兜底 [unverified：属性名需对照当前 Trino 文档]。
7. 值解码：按 `columns[].typeSignature` 把 JSON 值转成 ontoforge 自己的值类型：bigint / integer、decimal（字符串）、double / real（含 `NaN` / `Infinity` 字符串表示 [unverified]）、varchar / char、boolean、date、time、timestamp(p)、timestamp(p) with time zone（偏移量与 zone id 两种）、varbinary（base64）、array、map、row（位置数组 + typeSignature 字段名）、json、uuid。不支持的类型降级为字符串或返回该列级错误，**不让整条查询失败**。
8. 响应中的 `X-Trino-Set-*` / `X-Trino-Started-Transaction-Id`：ontoforge 是无状态 client，不回写状态；出现时记 warn 日志（L0 / L2 本不应产生）。
9. 不实现：事务（Action 写入直接走 PG，不经 Trino）、prepared statement、OAuth2 交互流程、spooling（4B 已限制结果规模，direct 协议足够）。

规模估算 [unverified：类比同类协议实现的经验估计]：

| 模块 | 行数（不含测试） |
|---|---|
| 配置、身份、认证、TLS | ~100 |
| header 组装 | ~60 |
| 提交 / 轮询 / 重试 / 429 / 取消 | ~170 |
| `QueryResults` / `Column` / `TypeSignature` / `QueryError` serde 模型 | ~120 |
| 值解码到 ontoforge 值类型 | ~220 |
| 合计 | **~670** |
| 测试（wiremock 录制的协议 fixture + 类型解码表驱动 + Linux CI 上的 Trino service container 集成测试） | ~400 |

## 6. 推荐与理由

**采用方案 B：在引擎 trait 之后自写 Trino 薄 client**（位置建议：新的引擎 crate 或 `ontoforge-query` 下的 `engine::trino` 模块，由 `/plan-eng-review` 定）。

按「优先复用现成 crate」规则写明的自研理由：

1. 唯一可用的现成 crate（`trino-rust-client`）是面向「单用户、有状态 session」的交互式 client，ontoforge 需要的是服务端「每请求无状态身份与 header + 共享连接池」，错配位于核心抽象（§5 A.1-A.3），不是补一个参数能解决的。
2. 它的类型层对 ontoforge 无用且有害（§5 A.4），而值解码是 ontoforge 无论如何都要自写的部分（ontology 类型映射、2A 错误映射、4B 截断）。
3. 剩下可复用的只是约 400-600 行协议循环；换来的是一个两个月内三次破坏性变更、近期出现过认证与事务基础缺陷的依赖。
4. 仍然复用生态的部分：`reqwest`（HTTP + rustls）、`serde` / `serde_json`、`bigdecimal` 或 `rust_decimal`、`chrono` / `chrono-tz`、测试用 `wiremock`。只自写 Trino 协议本身这一层。

复评条件：下一 milestone 若需要 spooling（大结果集导出、materialize），先评估 `trino-rust-client` 的 `spooling` 模块能否单独借用思路，或届时它是否已支持按请求 header 与外部 `reqwest::Client` 注入；满足则可切换。

## 7. 风险

| 风险 | 影响 | 缓解 |
|---|---|---|
| 协议边界情况遗漏（空页、columns 延迟出现、长时间 QUEUED、网关重定向） | 查询挂起或误报失败 | wiremock fixture 覆盖各状态；Linux CI Trino 集成测试；整体超时 + 取消兜底 |
| 类型线格式理解错误（timestamptz zone id、精度、NaN、map key） | 值错误，属于 conformance 失败 | 以 Trino CI 实测为准建立类型 golden 表；替身 vs Trino 等价套件覆盖 |
| Windows 本机无法跑真实 Trino（Docker daemon 未运行，项目约束本机不依赖 Docker） | Trino 侧行为只在 CI 暴露 | 协议层用 wiremock 录制 fixture 在 Windows 跑；真实行为只在 Linux CI 验证（设计文档 §5 已接受该风险） |
| 取消不可达（进程崩溃、网络断开） | 孤儿查询占资源 | Trino 端 `query_max_execution_time` 与 resource group 兜底 |
| 自写 client 维护成本 | 随 Trino 协议演进需要跟进 | 协议面窄（只用 direct 协议 + 少量 header）；client capability 不声明未实现的能力 |
| TLS provider 选择（aws-lc-rs vs ring）是 workspace 级决定 | 构建时间、C 依赖 | 本机已验证 aws-lc-sys 可构建；与 rmcp / 其他 HTTP 依赖统一后在 ADR 中锁定 |

## 8. Open items

1. **实机验证（Linux CI 或本机启动 Docker 后）**：`timestamp(6) with time zone` 的 JSON 线格式（zone id / 偏移量 / 精度）；`DELETE nextUri` 后的状态与资源释放；`X-Trino-User` 透传在 Trino impersonation 规则下的拒绝 / 放行行为；`X-Trino-Client-Tags` 命中 resource group selector。
2. **impersonation 语义**：L2 透传时用「服务身份认证 + `X-Trino-User` = 主体」，需确认 Trino 端 impersonation 规则配置方式，以及是否需要 `X-Trino-Original-User`。归入 spike 3 或 L2 设计。
3. **服务身份凭据形态**：Basic 密码还是 JWT（若 JWT，谁签发、如何轮换）。由 eng review 定。
4. **网关场景**：部署方若用 Trino Gateway / 反向代理，`nextUri` 的 host 与粘性路由需验证。
5. **decimal 库选择**：`bigdecimal` 与 `rust_decimal`（后者最大 28 位精度，Trino decimal 最大 38 位 [unverified]）需与 ontology property 类型定义对齐。
6. **ADR**：结论写入新 ADR「引擎 trait 与 Trino 连接」（设计文档 §6 已列）。
