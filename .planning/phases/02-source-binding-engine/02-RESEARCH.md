# Phase 2: Source Binding & Engine - Research

**Researched:** 2026-10-16
**Domain:** Rust 自写 Trino HTTP client、类型化 SQL printer、SQLite 替身引擎、ontology 绑定文件（新 kind）与在线校验、Linux CI（Trino + PG + Iceberg）
**Confidence:** MEDIUM-HIGH（依赖决策、Phase 1 代码事实、crates 解析与 Windows 编译均本会话实测；Trino 线格式细节与 CI 编排依赖未能在本机实跑的 Trino，列入 Assumptions）

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** 绑定写在**独立的绑定文件**里（新 kind，暂名 `SourceBinding`），按 profile 分目录（如 `bindings/dev/`、`bindings/prod/`）。ontology 定义文件保持纯语义、不出现任何物理 catalog / schema / 表 / 列名，开源示例因此不含任何公司命名。— **Reversibility:** costly — 改变定义文件格式、JSON Schema 与 loader；**修订 ADR-0015**（其原文「object type 增加可选 `source`」改为独立绑定文件），需新写一份 ADR supersede ADR-0015 的该条，其余类型契约条款不变。
- **D-02:** ObjectType（以及 M:N link）在 ontology 中**显式声明 backing**（暂名 `backing: source | editOnly`，纯语义字段）。声明 `source` 但选定 profile 中没有绑定 → E；声明 `editOnly` 却有绑定 → E。漏配不能静默变成「没有数据的 edit-only 对象」。— **Reversibility:** costly — 定义格式字段，示例与 schema 都要带上。
- **D-03:** 列映射：未列出的属性按同名列匹配，只写不同名的列；配一条 W 级 lint 提示「属性在绑定表中找不到对应列」（离线可判定的部分；在线不存在的列由 `validate --online` 报 E）。
- **D-04:** 一个 object type 只绑定单表或 view，不允许 SQL 片段作为源。多表拼接由部署方在 Trino 建 view。与 ADR-0020「Trino 只跑单表查询」一致。
- **D-05:** profile 选定：`--profile <name>` 或环境变量 `ONTOFORGE_PROFILE`，都未指定时用 ontology manifest（`ontology.yaml`）中声明的缺省 profile。示例 ontology 的缺省 profile 为 `dev`，指向 SQLite 替身。
- **D-06:** Trino 连接配置放在**独立的服务配置文件**（暂名 `ontoforge-server.toml`），与 ontology 定义和 lint 配置 `ontoforge.toml` 分开：地址、TLS、超时、两个服务身份、profile → 引擎映射。定义可公开共享，部署配置不共享。
- **D-07:** 密钥只以 `*_env`（环境变量名）或 `*_file`（文件路径）引用，配置文件不接受明文；密钥不进日志、诊断、错误信息，也不进发给 Trino 的 query text。
- **D-08:** 本 phase 的 Trino client 支持三种认证：用户名密码（Basic，仅 HTTPS）、无密码（仅 `X-Trino-User`，供本地 / CI 容器）、JWT / OAuth2 bearer。Kerberos 不做。
- **D-09:** TLS：可配置自定义 CA（`ca_file`），**不提供**跳过证书校验的开关。
- **D-10:** 两个服务身份（ontology 身份、L2 身份）在本 phase 的配置模型与 client 实例化中都支持（各自独立连接池 / 凭据）；L2 是否启用与启动自检留到 Phase 6。
- **D-11:** 替身的「source 表」数据来自示例自带的 CSV / JSON fixture：dev profile 的绑定指向 fixture 文件，加载进 `ATTACH` 的 `catalog__schema` 库，列类型由绑定 + IR 推出。测试与后续 `serve --dev` 共用同一机制。
- **D-12:** aviation 示例的 fixture 手写、小规模（每个 type 几十行），刻意覆盖 null、decimal 边界、时区、重复 pk 等用例所需的行；数据完全虚构，与任何公司无关。
- **D-13:** 类型兼容只接受**无损映射**：数值只可加宽、decimal 的精度与小数位不超过声明值、varchar 只映射到 String 等。有损映射报 E；可保留但精度变化报 W。映射表写入绑定规范文档。
- **D-14:** `validate --online` 复用 Phase 1 诊断体系：新增在线诊断码（编号由 planner 在 ONT-V16 之后分配），定位到绑定文件 line:col，支持 human / JSON 两种格式；有 E 时退出码非 0；Trino 不可达使用单独的退出码，与「绑定错误」区分。
- **D-15:** drift 校验在本 phase 交付库函数与按 object type 的降级状态模型（healthy / degraded + 原因），CLI 与测试共用；启动 / 定时执行与 `/health` 暴露在 Phase 7 有服务进程时接入。

### Claude's Discretion

- Trino 客户端协议细节（`nextUri` 轮询退避参数、连接池大小、各类 header 的取值）。
- 自有 printer 的 API 形态与模块划分，只要满足「值只经两个转义函数输出」。
- `Engine` trait 的接口形态（流式返回还是整批返回），依据 Phase 4 的分批回查需求自行判断。
- 绑定文件与服务配置文件的最终命名、字段命名（遵守 ADR-0002 命名规范）。
- CI 中 Trino / PG 容器版本与测试数据灌入方式。

### Deferred Ideas (OUT OF SCOPE)

- Kerberos 认证 — 有需求时再做。
- drift 校验的定时执行、`/health` 暴露 — Phase 7（服务进程）。
- L2 身份的启用与启动自检 — Phase 6。
- 按主体透传 Trino 身份（impersonation）— 依赖 JWT 支持，后续评估。
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| BIND-01 | 开发者可为 object type 与 M:N link 声明 source 绑定（catalog / schema / table / columns），无绑定的 object type 是 edit-only | 新 kind `SourceBinding` + `backing` 字段的 IR / loader / JSON Schema 方案；见「Binding 文件形状」「Phase 1 接入点」 |
| BIND-02 | 绑定遵守类型契约（decimal 精度、无时区 timestamp 的 `timezone`、可选 collation、geopoint 不支持），违反时 lint 报 E | decimal 精度进 `PropertyDef`；新增 lint E 规则（ID 须避开 spec 表已占用的 E01-E12）；离线可判定性分析见 Open Question 1 |
| BIND-03 | `validate --online` 对 Trino 校验绑定，`--check-pk` 查重；drift 不匹配时 object type 降级（`SourceSchemaDrift`），其余照常 | 纯函数 `compare_binding` + `describe_table` + 类型兼容表 + `DriftReport` 模型；CLI 退出码方案 |
| ENGINE-01 | 经自写 Trino client 连接已有 Trino，支持超时、断连取消、trace token 与两个服务身份 | Trino 协议要点（文档核实）、client 设计、wiremock 测试方案、`nextUri` 同源校验 |
| ENGINE-02 | Trino 不可用时 fail-closed（`EngineUnavailable`），`/health` 与 describe 暴露引擎状态 | 本 phase 交付错误分类与 `EngineStatus` 探测；`/health` / describe 暴露属 Phase 7，需在 traceability 注明 ENGINE-02 本 phase 部分完成 |
| ENGINE-03 | SQL 由自有 printer 生成，值只经转义函数输出，proptest 与真实执行验证 | 类型化 AST + `PrintedSql` 新类型 + 两个转义函数 + sqlparser 往返与 SQLite 执行的 proptest 设计 |
| ENGINE-04 | SQLite 替身引擎支撑 Windows 本机 `cargo test` 与 `serve --dev` | `sqlx` sqlite（不能用 rusqlite，见 Pitfall 1）、ATTACH 每连接、fixture 加载 |
| ENGINE-05 | Linux CI 跑 Trino + PG 集成测试，nightly 加跑 Iceberg；SQL Server 走私有 fixture | CI 编排（services 不能挂载仓库文件）、`#[ignore]` + `--ignored` 门控、nightly workflow 与失败开 issue |
</phase_requirements>

## Summary

本 phase 在 Phase 1 的 schema / loader / lint / CLI 之上加两块：(1) 定义层：新 kind `SourceBinding`、ObjectType 与 M:N link 的 `backing` 字段、decimal `precision` / `scale`、manifest 的 `defaultProfile`，全部进 IR、JSON Schema 与 insta 快照；(2) 新 crate `ontoforge-engine`：`Engine` trait、自写 Trino client、类型化 SQL printer、SQLite 替身。`validate --online` 把二者接起来：对选定 profile 的绑定调用 `Engine::describe_table`，用一个纯函数 `compare_binding` 产出不匹配项，CLI 把它映射成带 line:col 的诊断，Phase 7 的 drift 复用同一函数。

最重要的三个事实：(a) **`rusqlite` 与 `sqlx` 的 sqlite 后端不能同时存在**（`libsqlite3-sys` 的 `links = "sqlite3"` 冲突，已实测），而 Phase 3 的 storage 已定用 `sqlx`，所以 engine 的 SQLite 替身必须用 `sqlx`；(b) **`backing` 设为必填会改动 24 个 invalid 样例目录、aviation 与 aviation-json 两个示例、13 个 Rust 源 / 测试文件**，需要一个专门的、可脚本化的迁移 plan 放在最前；(c) **ONT-E10..E12 / W12..W16 已被 foundry spec 规则表占用**，新增绑定类 lint 规则必须从 ONT-E13、ONT-W17 起编号。

Docker daemon 在本机未运行（`docker info` 退出码 1），Java 只有 17，因此 Trino 相关行为在 Windows 本机只能靠 wiremock 协议 fixture 验证，真实 Trino 只在 Linux CI 暴露；这是 ADR-0017 已接受的风险，plan 要把「哪些断言只在 CI 跑」列清楚，并保证 CI 里 `--ignored` 测试在环境缺失时失败而不是静默跳过。

**Primary recommendation:** 先做 plan 0「`backing` 必填迁移 + IR / loader / 诊断码骨架」，再并行推进 engine crate（printer + SQLite 替身）与 Trino client（wiremock），最后收口 lint 规则、`validate --online` / `--check-pk`、CI。Trino client、`Engine` trait 用原生 `async fn` + 枚举分发，整批返回并在 client 内强制行数 / 字节上限，不引入 `async-trait`、`bigdecimal`、`testcontainers`。

## Project Constraints (from CLAUDE.md)

来源：`D:/DataSpellProjects/ontoforge/.claude/CLAUDE.md`、`C:/Users/easyi/.claude/CLAUDE.md`、auto-memory。

- Rust 1.98，edition 2024，cargo workspace；依赖 axum / sqlx / sea-query / cel / rmcp 已在 ADR-0003..0011 锁定（engine 不得引入与之冲突的栈）。
- **必须在 Windows 11 上 `cargo build` / `cargo test` 通过，本机不依赖 Docker**；Postgres / Trino 测试放 Linux CI 或门控之后。
- 命名：crate 前缀 `ontoforge`；遵循 ADR-0002；**禁止沿用参考仓库标识符**（`kn_id`、KN、BKN、ODL 等）；不复制参考仓库代码（mcp-trino 只借设计）。
- 错误码 `UPPER_SNAKE_CASE`，wire 字段 camelCase，Rust 惯例 snake_case，数据库 `of_` 前缀（本 phase 无 DB 表）。
- 优先复用现成 crate，自研须写明理由（memory）。本研究对自写 Trino client 沿用 spike 1 已写明的理由。
- 通用系统原则：公开仓库不得出现公司 catalog / 表名；公司环境验证只经 `ONTOFORGE_PRIVATE_FIXTURES`（memory + CONTEXT specifics）。
- 决策自主：能自调研自决的不问用户，必须拍板的才问（memory）。本研究把必须拍板的项集中在 Open Questions，并给推荐。
- GSD：任何文件变更经 GSD 命令；本 phase 产物在 `.planning/phases/02-source-binding-engine/`。
- 安全基线（Phase 1 SECURITY）：32 MiB 栈线程、4 MiB 文件上限、控制字符净化、`Cargo.lock` 入库、CI `--locked`、新依赖需 legitimacy 审核。
- 工作流约束：`unsafe_code = "forbid"`；测试里不得调用 `std::env::set_var`（`CONTRIBUTING.md:27`），环境变量只能经 `Command::env` 传给子进程，库函数应以参数形式接收环境变量值。
- 输出语言简体中文，技术名词保留英文。

## Architectural Responsibility Map

本项目不是 Web 多层应用，下表把「层」替换为 workspace 内的责任边界。

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| 绑定文件解析、`backing` 校验、profile 选定 | `ontoforge-schema`（loader / validate） | CLI（读 flag / env） | IR 是唯一契约（ADR-0003）；profile 选定值由 CLI 传入库，库本身不读环境变量（`set_var` 禁令 + 可测性） |
| 类型契约 lint（decimal / timezone / collation / geopoint） | `ontoforge-schema`（lint） | — | 离线可判定，沿用 lint 注册表与 `ontoforge.toml` 配置 |
| 绑定与物理表比对（`compare_binding`）、类型兼容表 | `ontoforge-engine`（或 schema 下的纯模块） | CLI 映射为 Diagnostic | 纯函数，无 IO，Windows 可全覆盖；drift（Phase 7）复用 |
| Trino HTTP 协议、认证、TLS、取消、重试 | `ontoforge-engine::trino` | — | 自写薄 client（ADR-0017） |
| SQL 生成与转义 | `ontoforge-engine::printer` | — | 值只经两个转义函数；`PrintedSql` 新类型是执行入口的唯一入参 |
| SQLite 替身 + fixture 加载 | `ontoforge-engine::sqlite` | `ontoforge-schema` 提供 fixture 声明 IR | 与 Phase 3 storage 共用 `sqlx` sqlite，避免 `links` 冲突 |
| 服务配置（`ontoforge-server.toml`）与密钥引用 | `ontoforge-engine::config` | Phase 7 `ontoforge-server` 扩展 | 本 phase 只需 Trino 连接 + 身份 + profile→引擎映射；整体 `ServerConfig` 留给 Phase 7 |
| `validate --online` / `--check-pk` / 退出码 | `ontoforge`（CLI） | engine | 编排 + 渲染；异步运行时只在 CLI 的 online 路径内创建 |
| Trino / PG 集成测试与 nightly Iceberg | `.github/workflows/` | 测试用 `#[ignore]` 门控 | service container 不能挂载仓库文件（见 Pitfall 7） |

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `reqwest` | 0.13.5 | Trino HTTP client（`default-features = false, features = ["rustls"]`） | spike 1 选定；本会话实测 `rustls` feature 解析出 `rustls-platform-verifier` + `aws-lc-rs`，Windows `cargo check` 通过（26 s）[VERIFIED: 本机 probe `cargo tree` / `cargo check`] |
| `tokio` | 1.53.3 | 异步运行时（`rt`、`time`、`sync`、测试用 `macros`） | reqwest / sqlx 的运行时；CLI 仅在 online 路径创建 current_thread runtime [VERIFIED: cargo search] |
| `sqlx` | 0.9.0 | SQLite 替身（`default-features = false, features = ["sqlite", "runtime-tokio"]`） | 与 ADR-0016 / Phase 3 storage 同栈；`sqlite` feature 带 bundled SQLite 3.51.3 [VERIFIED: probe `cargo tree -e features`；`libsqlite3-sys-0.37.0/sqlite3/sqlite3.h:149` `#define SQLITE_VERSION        "3.51.3"`] |
| `secrecy` | 0.10.3 | 密钥类型（Debug 脱敏、不意外 Serialize） | 避免自研；legitimacy `OK` [VERIFIED: gsd-tools package-legitimacy] |
| `csv` | 1.4.0 | CSV fixture 解析 | legitimacy `OK`；JSON fixture 用已有 `serde_json` |
| `chrono-tz` | 0.10.4 | 校验绑定里的 IANA `timezone` | 与现有 `chrono` 同族；legitimacy `OK`。features 需 planner 以 `default-features = false` 试编译确认 [ASSUMED] |
| `toml` | 1.1.8（已在 workspace） | `ontoforge-server.toml` 解析 | 已有依赖 |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `wiremock` | 0.6.5（dev） | 模拟 Trino 协议（POST / GET nextUri / DELETE / 429 / 503 / 头断言） | 所有 Windows 本机可跑的 client 测试 |
| `proptest` | 1.12.0（dev） | printer 注入 / 往返性质测试 | ENGINE-03 |
| `sqlparser` | 0.63.0（dev，仅测试） | proptest 中把 printer 输出解析回 AST 比较结构 | 只在 engine 的 `[dev-dependencies]`；L2 用途在 Phase 6 |
| `ulid` | 3.0.0 | CLI 生成 `X-Trino-Trace-Token`（ADR-0002 的 `rid` 本就用 ULID） | 仅 CLI 需要；调用方也可自带字符串。legitimacy `OK` |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `sqlx` sqlite | `rusqlite` 0.40.2 | **不可行**：`rusqlite 0.40.2` 要 `libsqlite3-sys ^0.38.2`，`sqlx-sqlite 0.9.0` 要 `>=0.30.1, <0.38.0`，cargo 报 `links = "sqlite3"` 冲突（实测，见 Pitfall 1） |
| 原生 `async fn` in trait + 枚举分发 | `async-trait` | 本 phase 只有 `TrinoEngine` / `SqliteEngine` 两个实现，枚举足够，省一个依赖；Phase 4 若需要 mock 可用泛型 [ASSUMED: 设计取舍] |
| 整批返回 + client 内强制上限 | 流式 `Stream<Row>` | ADR-0020 的所有 Trino 查询都是 TopN / ≤256 pk 批 / 聚合，结果有界；上限命中时 client `DELETE nextUri` 并标 `truncated`。流式增加生命周期复杂度，YAGNI |
| `secrecy` | 自写 `Secret(String)` | memory 要求优先复用；`secrecy` 已 `OK` |
| `bigdecimal` | — | **本 phase 不加**：decimal 值在 `Value` 里保持规范化字符串，数值比较是 Phase 4 的事（ADR-0015 把 Rust 侧 38 位十进制比较放在叠加读） |
| `testcontainers` 0.28.0 | `docker run` / GHA `services:` | 需要 Docker，与「Windows 本机不依赖 Docker」冲突；Linux CI 用 workflow 步骤编排 |

**Installation（workspace `[workspace.dependencies]` 新增）：**

```bash
# 新增到根 Cargo.toml 的 [workspace.dependencies]，各 crate 用 .workspace = true 引用
# reqwest = { version = "0.13.5", default-features = false, features = ["rustls"] }
# tokio   = { version = "1.53.3", features = ["rt", "time", "sync"] }
# sqlx    = { version = "0.9.0", default-features = false, features = ["sqlite", "runtime-tokio"] }
# secrecy = "0.10.3"
# csv     = "1.4.0"
# chrono-tz = "0.10.4"
# ulid    = "3.0.0"
# dev: wiremock = "0.6.5", proptest = "1.12.0", sqlparser = "0.63.0"
cargo generate-lockfile && cargo build --workspace --locked
```

**Version verification:** 版本均于 2026-10-16 经 `cargo search` 取得；crates 组合在 `/tmp` 隔离 probe 工程中 `cargo generate-lockfile` 成功（259 个包）并 `cargo check` 通过 [VERIFIED: 本机]。

## Package Legitimacy Audit

`gsd-tools query package-legitimacy check --ecosystem crates ...` 对下列包全部返回 `OK`（含 reqwest、tokio、sqlx、rusqlite、proptest、wiremock、bigdecimal、csv、sqlparser、testcontainers、httpmock、tokio-util、futures-util、url、base64、zeroize、secrecy、chrono-tz、jiff、async-trait、toml_edit、ulid、uuid）[VERIFIED: gsd-tools，2026-10-16]。

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| reqwest | crates.io | 多年 | 极高 | github.com/seanmonstar/reqwest | OK | Approved |
| tokio | crates.io | 多年 | 极高 | github.com/tokio-rs/tokio | OK | Approved |
| sqlx | crates.io | 多年 | 极高 | github.com/launchbadge/sqlx | OK | Approved（Phase 3 已定） |
| secrecy | crates.io | 多年 | 高 | github.com/iqlusioninc/crates | OK | Approved |
| csv | crates.io | 多年 | 极高 | github.com/BurntSushi/rust-csv | OK | Approved |
| chrono-tz | crates.io | 多年 | 高 | github.com/chronotope/chrono-tz | OK | Approved |
| ulid | crates.io | 多年 | 高 | github.com/dylanhart/ulid-rs | OK | Approved |
| wiremock | crates.io | 多年 | 高 | github.com/LukeMathWalker/wiremock-rs | OK | Approved（dev） |
| proptest | crates.io | 多年 | 高 | github.com/proptest-rs/proptest | OK | Approved（dev） |
| sqlparser | crates.io | 多年 | 83.7M 总量（spike 2） | github.com/apache/datafusion-sqlparser-rs | OK | Approved（dev，本 phase 仅测试） |

注：上表 Age / Downloads / Source Repo 为定性记述，未逐项抓取；`gsd-tools` 的 verdict 是权威字段。包名均来自 spike 报告与官方文档中已出现的名称，不是 WebSearch 新发现。

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

补充（Node.js 的 postinstall 检查不适用）：Rust 依赖的等价风险是 `build.rs`。本 phase 新增的带 build script 的链：`libsqlite3-sys`（编译 bundled SQLite，需要 C 编译器）、`aws-lc-sys`（经 reqwest/rustls）。两者本机均已编译通过 [VERIFIED: probe]。plan 应保持 `Cargo.lock` 入库与 CI `--locked`（Phase 1 SECURITY 基线）。

## Architecture Patterns

### System Architecture Diagram

```
                          ┌──────────────────────── 定义层（ontoforge-schema）──────────────────────┐
 ontology dir             │ ontology.yaml(manifest + defaultProfile)                               │
  objects/links/actions ─►│ bindings/<profile>/*.yaml  (kind: SourceBinding, 路径推出 profile)      │
  bindings/dev/fixtures ─►│ loader: discover → parse → dispatch(新 kind) → merge → validate(+V16..) │
                          │ lint: E13.. (decimal/timezone/collation/geopoint) + W17 (fixture 列缺失)│
                          └───────────────┬────────────────────────────────────────────────────────┘
                                          │ Ontology{ .., bindings } + SourceMap(binding 文件 line:col)
   CLI: validate [--profile] [--online [--check-pk]]                                   
        flag > ONTOFORGE_PROFILE > manifest.defaultProfile   ← 解析为 ProfileName(只作 map key,不拼路径)
                                          │
              ┌── 离线:就此输出诊断, exit 0/1 ──┘
              │
              └── --online ─► 构造 Engine (server config: profile→engine;无配置且绑定带 fixture ⇒ SQLite 替身)
                                │
              ┌─────────────────┴───────────────────────────────┐
              ▼                                                 ▼
   TrinoEngine (reqwest, rustls)                     SqliteEngine (sqlx sqlite, bundled)
   POST /v1/statement ─► GET nextUri… ─► done        ATTACH ':memory:' AS "cat__sch" (每连接)
   DELETE nextUri (超时/断连/上限)                    fixture(csv/json) ─► CREATE TABLE + 绑定 INSERT
   头: User/Source/Trace-Token/Client-Tags/TZ=UTC     PRAGMA query_only=ON 之后才接受查询
   错误分类: Unavailable / Unauthorized / Timeout /   
            Cancelled / Query{errorName}             
              │  只接受 PrintedSql                              │
              └──────────────┬──────────────────────────────────┘
                             ▼
              printer(Dialect::Trino | Sqlite)   ← 唯一生成 SQL 的地方
              AST: Select{proj, from, where, group_by, having, order_by(NULLS 显式), limit}
              出口: quote_ident / quote_string(两个转义函数) → PrintedSql(私有构造)
                             ▼
   describe_table(cat,sch,tbl) ─► Vec<PhysicalColumn{name, PhysicalType, nullable}>
                             ▼
   compare_binding(binding, ontology, physical) ─► Vec<Mismatch>   (纯函数)
        ├─ validate --online: Mismatch ─► Diagnostic(E/W, 绑定文件 line:col)
        └─ drift(Phase 7): Mismatch ─► ObjectTypeHealth::Degraded{SourceSchemaDrift, reasons}
   --check-pk: SELECT pk, count(*) … GROUP BY pk HAVING count(*) > 1 LIMIT n  +  pk IS NULL 计数
        退出码: 0 ok / 1 有 E(含绑定不匹配、重复 pk) / 2 clap 用法错误 / 3 EngineUnavailable 或认证失败 / 101 内部错误
```

### Recommended Project Structure

```
crates/
├── ontoforge-schema/src/
│   ├── ir/binding.rs          # SourceBinding, BindingTarget, SourceRef, ColumnBinding, Fixture, Collation, Backing
│   ├── ir/{object,link,manifest,basetype}.rs   # +backing, +precision/scale, +defaultProfile
│   ├── loader/{dispatch,discover,sourcemap}.rs # +SourceBinding kind, bindings/<profile> 路径规则, DefKind::SourceBinding
│   ├── validate/bindings.rs   # V16.. 引用 / 完备性 / fixture 路径
│   └── lint/bind_*.rs         # E13.. / W17
├── ontoforge-engine/          # 新 crate(workspace 成员 crates/* 自动纳入)
│   └── src/
│       ├── lib.rs             # Engine trait, AnyEngine 枚举, EngineError, EngineStatus
│       ├── value.rs           # Value, PhysicalType(+解析 information_schema data_type 字符串)
│       ├── printer/{mod,ast,escape,dialect}.rs
│       ├── trino/{client,config,error,decode,headers}.rs
│       ├── sqlite/{mod,fixture}.rs
│       ├── config.rs          # ontoforge-server.toml 中与引擎相关的部分
│       └── bind_check.rs      # compare_binding, Mismatch, DriftReport, ObjectTypeHealth, 类型兼容表
│   └── tests/{trino_client.rs(wiremock), printer_prop.rs, sqlite_engine.rs, trino_it.rs(#[ignore])}
├── ontoforge/src/{cli,lib,online}.rs  # +--profile --online --check-pk --server-config, tokio current_thread
examples/ontologies/aviation/
├── objects.yaml links.yaml        # +backing(+cost 的 decimal 属性等)
├── ontology.yaml                  # +defaultProfile: dev
└── bindings/dev/{*.yaml, fixtures/*.csv}
ci/trino/catalog/*.properties, ci/pg/seed.sql, .github/workflows/{ci.yml,nightly.yml}
docs/source-binding.md             # 绑定规范 + D-13 类型映射表
.planning/adr/0021-*.md            # supersede ADR-0015 的 inline source 条款(并注明 ADR-0011 增加 engine crate、ADR-0012 增加 backing/defaultProfile)
```

### Pattern 1: Binding 文件形状（推荐）

**What:** 一个 `SourceBinding` 文档绑定一个 target（object type 或 M:N link）；profile 由目录推出，不在文档内重复声明。`columns` 总是对象形式（避免 untagged enum 丢失错误路径，ADR-0003 spike 结论）。

```yaml
# bindings/dev/flight.yaml   (profile = "dev"，由路径 bindings/dev/ 推出)
kind: SourceBinding
target: { objectType: Flight }
source: { catalog: demo, schema: aviation, table: flights }
columns:
  flightId:           { column: flight_no }
  scheduledDeparture: { column: dep_ts, timezone: Asia/Shanghai }   # 无时区 timestamp 列必须声明
  status:             { collation: caseInsensitive }                # 不写 column 即同名
fixture: { file: fixtures/flights.csv }                            # 仅 dev profile 使用
---
kind: SourceBinding
target: { linkType: flightCrew }                                    # M:N join 表
source: { catalog: demo, schema: aviation, table: flight_crew }
columns:
  from: { column: flight_id }                                       # from / to 对应 link 两端
  to:   { column: pilot_id }
```

**When to use:** 所有 source-backed 的 object type 与 M:N link。**目录是 profile 的唯一来源**：`bindings/<profile>/…` 之外出现 `kind: SourceBinding` 报错；profile 名限制为 `[a-z][a-z0-9_-]{0,31}`，且只作为已加载 map 的 key 去查，**绝不拼进文件系统路径**（`--profile` / `ONTOFORGE_PROFILE` 是外部输入）。

**ontology 侧：**

```yaml
# objects.yaml
kind: ObjectType
apiName: Flight
backing: source          # source | editOnly，必填
...
# ontology.yaml
defaultProfile: dev
```

### Pattern 2: 纯函数 `compare_binding` 作为 CLI 与 drift 的唯一实现

**What:** `fn compare_binding(binding: &SourceBinding, object: &ObjectType, physical: Option<&[PhysicalColumn]>) -> Vec<Mismatch>`。`Mismatch` 带枚举原因（`TableMissing`、`ColumnMissing{property, column}`、`TypeIncompatible{property, source, declared}`、`PrecisionChanged`、`TimezoneMismatch`…）和指向绑定文档的 `Seg` 路径。CLI 把它映射为 `Diagnostic`（用 `SourceMap::locate`），drift 把它折叠为 `ObjectTypeHealth::{Healthy, Degraded{reasons}}`。无 IO，Windows 可 100% 单测。

### Pattern 3: `PrintedSql` 新类型把「SQL 只来自 printer」变成编译期约束

**What:** `pub struct PrintedSql(String)`，构造函数 `pub(crate)`，`Engine::execute` 只接受 `PrintedSql`。information_schema 查询、`--check-pk` 查询也走 printer。Phase 6 的 L2 在此加一个 `RawValidatedSql` 变体，不改本 phase 的不变量。

**两个转义函数是 `printer::escape` 的唯一公开出口：**

```rust
// Trino 与 SQLite 的字符串字面量都只需把 ' 写成 ''（Trino 无反斜杠转义，见 spike 2 对 Trino 语法的核对）
pub(super) fn quote_string(s: &str) -> Result<String, PrintError> { /* 拒绝 NUL；'→'' */ }
pub(super) fn quote_ident(s: &str) -> Result<String, PrintError>  { /* 拒绝空串与 NUL；"→"" */ }
```

AST 节点携带**原始值**，转义只在 `Display` 阶段发生；AST 里没有「已转义字符串」类型，因此无法绕过。

### Pattern 4: Trino client 生命周期（协议要点，文档核实）

```
POST {base}/v1/statement   body=SQL 文本           ─► QueryResults{ nextUri?, columns?, data?, error?, stats }
GET  {nextUri}  …循环直到响应没有 nextUri
DELETE {nextUri}           取消（超时 / 断连 / 行数字节上限）
状态码: 200 正常; 429 按 Retry-After 重试; 502/503/504 间歇故障 50-100ms 后重试; 其余视为失败
```

[CITED: trino.io/docs/current/develop/client-protocol.html，当前文档版本 484] `X-Trino-Session`（逗号分隔 name=value）只在初始 POST 设置 [CITED: 同上与 query management 文档]；超时兜底用 session 属性 `query_max_execution_time`（duration，默认 100d）[CITED: trino.io/docs/current/admin/properties-query-management.html]。

**client 设计要点：**

1. 每个服务身份一个 `TrinoEngine` 实例，各自一个 `reqwest::Client`（独立连接池 / 凭据）；每请求无状态组装 header：`X-Trino-User`、`X-Trino-Source: ontoforge`、`X-Trino-Trace-Token`（调用方传入的 request_id）、`X-Trino-Client-Tags`、`X-Trino-Time-Zone: UTC`。响应里的 `X-Trino-Set-*` / `X-Trino-Started-Transaction-Id` 不回写，记 warn。
2. `reqwest::redirect::Policy::none()`；**`nextUri` 必须与配置的 `url` 同 scheme / host / port，否则拒绝并返回协议错误**（否则被劫持的 `nextUri` 会拿到 `Authorization` 头；网关场景用显式配置放开，不默认放开）。
3. 取消：把「当前 `nextUri`」放进一个 guard，`Drop` 时若查询未完成则 `tokio::spawn` 一个 `DELETE`（`Handle::try_current()` 失败则放弃）；超时由 `tokio::time::timeout` 包住整个 `execute`，到期同样 `DELETE` 后返回 `Timeout`。
4. 单页响应体上限：按块读取并累计，超过上限（建议 64 MiB，可配）即取消并报错，不依赖 `Content-Length`。
5. 值解码按 `columns[].typeSignature` 输出 `Value` 枚举；不支持的类型降级为 `Value::Unsupported(raw_json)`，**不让整条查询失败**（spike 1 对现成 crate 的主要批评之一）。decimal / date / timestamp 保持规范化字符串 + 类型标签，数值解析留 Phase 4。

### Pattern 5: 类型化 printer 的方言差异（必须在 Phase 2 就体现）

| 点 | Trino | SQLite（bundled 3.51.3） | printer 要求 |
|----|-------|------------------------|--------------|
| 表引用 | `"cat"."sch"."tbl"` | `"cat__sch"."tbl"`（ATTACH 库名） | `TableRef::print(dialect)` |
| 字符串字面量 | `'…'`，`'` 写 `''`，无反斜杠转义 | 同 | `quote_string` |
| 类型化字面量 | `DECIMAL '1.2'`、`DOUBLE '1.5'`、`DATE '…'`、`TIMESTAMP '…'`（spike 2 语料 G12、G40 解析通过） | 数值裸写，日期 / 时间戳为 ISO 文本 | `Literal::print(dialect)` |
| 小数字面量 | 裸 `1.5` 是 DECIMAL（见 Assumptions A9） | 浮点 | double 一律 `DOUBLE '…'` |
| NULL 排序 | ASC 默认 NULLS LAST | ASC 默认 NULLS FIRST | **一律显式 `NULLS FIRST/LAST`**（SQLite ≥ 3.30 支持，bundled 3.51.3 满足） |
| `LIKE` | 区分大小写 | 对 ASCII 默认不区分大小写 | 本 phase **不放进 printer**（QUERY-02/03 属 Phase 4）；AST 留扩展位，并把「LIKE 模式的通配符转义也必须落在两个转义函数的同一出口」写进 Phase 4 备忘 |
| 非有限浮点 | `nan()` / `infinity()` | 无字面量 | 拒绝（`PrintError`） |
| NUL 字符 | 未核实 | C API 以 NUL 截断 SQL 文本（Assumptions A6） | 字符串与标识符含 NUL 一律 `PrintError` |

Phase 2 的 AST 子集：`SELECT` 列（列、`count(*)`、`count/sum/min/max/avg`）、单表 `FROM`、`WHERE`（比较、`IN` 列表、`IS [NOT] NULL`、`AND/OR/NOT`）、`GROUP BY`、`HAVING`、`ORDER BY`（显式 NULLS）、`LIMIT`。足以覆盖 information_schema 查询与 `--check-pk`，并给 Phase 4 留出扩展面。

### Pattern 6: 在线校验与 `--check-pk` 的查询形态

- 列信息：Trino 用 `SELECT column_name, data_type, is_nullable FROM "cat".information_schema.columns WHERE table_schema = '…' AND table_name = '…' ORDER BY ordinal_position`（`information_schema.columns` 含 `table_catalog, table_schema, table_name, column_name, ordinal_position, column_default, is_nullable, data_type` [CITED: 第三方 Trino 元数据指南，WebSearch 结果，见 A3]）。SQLite 没有 information_schema：`describe_table` 的 SQLite 实现走 `pragma_table_info`，因此 `Engine::describe_table` 要作为 trait 方法而不是让调用方拼 information_schema SQL。
- **名字大小写**：Trino 标识符不区分大小写 [CITED: spike 2，引 trino.io reserved 文档]，PG / SQL Server connector 暴露为小写。比对时两侧一律 `to_lowercase()`；两个只差大小写的列无法区分，应在 `compare_binding` 报 W。
- `--check-pk`（显式开关，ADR-0015 禁止启动 / 定时跑）：`SELECT "pk", count(*) AS n FROM … GROUP BY "pk" HAVING count(*) > 1 ORDER BY n DESC, "pk" LIMIT 21`（取 21 行以判断是否还有更多）加一条 `pk IS NULL` 计数。超时按 `--timeout` 或配置，结果上限命中就取消。
- PG 无限定 `NUMERIC` 在 Trino 484 文档里映射为 `NUMBER`，PG 数组默认 `DISABLED`（由 `postgresql.array-mapping` 控制）[CITED: trino.io/docs/current/connector/postgresql.html]。`PhysicalType` 必须有 `Number` 与 `Other(String)` 变体，未知类型按「不兼容」报 E 并带 help，而不是 panic 或静默放过。

### Pattern 7: D-13 类型兼容表（推荐写入绑定规范文档）

| 属性类型（IR） | 源列类型（Trino `data_type`） | 判定 |
|---|---|---|
| byte | tinyint | OK |
| short | tinyint, smallint | OK |
| integer | tinyint, smallint, integer | OK；bigint 为 E（有损） |
| long | tinyint, smallint, integer, bigint | OK |
| float | real | OK；double 为 E |
| double | real, double | OK（real→double 属加宽，保留 W 不必要） |
| decimal(P,S) | decimal(p,s) 满足 `s ≤ S` 且 `p - s ≤ P - S` | OK；`p < P` 或 `s < S` 时 W（精度变化可保留）；否则 E |
| decimal | NUMBER、无界 numeric、double、real | E（有损），help 提示在 Trino 建 view 并 cast |
| string | varchar(n)、char(n) | OK；`char(n)` 带 W（补空格语义） |
| boolean | boolean | OK |
| date | date | OK；timestamp 为 E |
| timestamp | `timestamp(p) with time zone` | OK（不需要 `timezone`）；`timestamp(p)` 无时区必须有 `timezone`，否则 E |
| array(items) | `array(T)`，T 按元素类型同表判定 | OK |
| geopoint | 任意 | E（本阶段不支持 source 绑定） |

D-13 字面写「decimal 的精度与小数位不超过声明值」，直读为 `p ≤ P 且 s ≤ S`，但 `decimal(10,2)` 映射到声明 `decimal(10,5)` 满足该读法却会溢出整数位（8 位整数位放进 5 位）。上表用整数位规则 `p - s ≤ P - S` 取代，见 Open Question 6。

### Anti-Patterns to Avoid

- **在 printer 之外拼 SQL**（`format!("... '{}'", v)`）：违反 ENGINE-03。保留 `PrintedSql` 私有构造，并加一个源码级测试：engine crate 里除 `printer/` 外不得出现 `format!` 拼 `SELECT`。
- **让 profile / 环境变量值进文件系统路径**：只作 map key。
- **用 `Mutex<SqliteConnection>` 以外的方式在池里复用 ATTACH**：ATTACH 是每连接的状态，池里新开的连接没有 attach 过的库（见 Pitfall 2）。
- **在库函数里读 `std::env::var("ONTOFORGE_PROFILE")`**：库 API 接收 `Option<&str>`，只有 `main` / CLI 层读环境变量（`set_var` 禁令使库内读 env 无法在测试中隔离）。
- **把 `/health` / describe 的暴露塞进本 phase**：ENGINE-02 的暴露面属 Phase 7；本 phase 交 `EngineError::Unavailable`、`Engine::status()` 与测试。

### 建议的 plan 切分（供 planner 参考，文件归属互不重叠以便并行）

| Wave | Plan | 内容 | 主要文件归属 | 覆盖需求 |
|------|------|------|--------------|----------|
| 1 | A | `backing` 必填迁移（脚本批量改既有 fixture）+ ADR-0021 起草 | `examples/`、`crates/*/tests`、`.planning/adr/` | BIND-01 前置 |
| 1 | B | `ontoforge-engine` 骨架：`Value` / `PhysicalType` / `EngineError` / `Engine` trait / `PrintedSql` / printer + 转义 + proptest | `crates/ontoforge-engine/src/{lib,value,printer}` | ENGINE-03 |
| 2 | C | schema：`SourceBinding` kind、IR、`defaultProfile`、decimal `precision` / `scale`、`DefKind::SourceBinding`、诊断码 V16.. 与 validate、JSON Schema 与快照 | `crates/ontoforge-schema/src/{ir,loader,validate,schema_export}` | BIND-01 |
| 2 | D | Trino client + 配置 + 密钥引用（wiremock 全套） | `crates/ontoforge-engine/src/{trino,config}` | ENGINE-01、ENGINE-02 |
| 2 | E | SQLite 替身 + fixture 加载 | `crates/ontoforge-engine/src/sqlite` | ENGINE-04 |
| 3 | F | lint：ONT-E13.. / ONT-W17、`KNOWN_RULE_IDS`、invalid 样例 | `crates/ontoforge-schema/src/lint` | BIND-02 |
| 3 | G | `compare_binding` / drift 模型 / 类型兼容表 + 绑定规范文档 | `crates/ontoforge-engine/src/bind_check.rs`、`docs/source-binding.md` | BIND-03 |
| 4 | H | CLI：`--profile`、`--online`、`--check-pk`、`--server-config`、退出码 3、aviation 绑定与 fixture | `crates/ontoforge/src`、`examples/ontologies/aviation` | BIND-03、DX 示例 |
| 4 | I | CI：Trino + PG job、nightly Iceberg、`trino_it` 测试 | `.github/workflows`、`ci/`、`crates/ontoforge-engine/tests/trino_it.rs` | ENGINE-05 |

C 与 B / D / E 对 `ontoforge-schema` 与 `ontoforge-engine` 的写入范围不同，可并行；F 依赖 C；G 依赖 B、C；H 依赖 D、E、F、G。

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| HTTP / TLS / 连接池 | 自写 socket、TLS | `reqwest` + rustls | spike 1：只自写 Trino 协议层 |
| 自定义 CA | 自解析 PEM | `reqwest::Certificate::from_pem` + builder 的 `tls_certs_merge` | [CITED: docs.rs/reqwest tls 模块]；方法名随 0.13 变化，planner 以编译确认（A1） |
| 密钥脱敏 | 自写 Debug 脱敏 | `secrecy::SecretString` | 避免遗漏 `Debug` / `Serialize` 路径 |
| IANA 时区校验 | 自带时区表 / 正则 | `chrono-tz` | 时区名集合会变 |
| CSV 解析 | `split(',')` | `csv` | 引号、换行、BOM |
| 注入检测 | 自写 SQL 解析器 | `sqlparser`（仅 dev，往返比对） | 已在 spike 2 评估；生成路径不用它（G11 / G16 / G37 缺陷） |
| 随机测试 | 手写随机字符串 | `proptest` | 带收缩，可固化历史反例 |
| 协议 mock | 自写 TCP mock | `wiremock` | 支持头断言、`DELETE` 计数、按序响应 |

**Key insight:** 本 phase 唯一值得自研的是「Trino 协议循环 + 类型化 printer + `compare_binding`」，三者都是项目特有语义；其余全部走生态。

## Common Pitfalls

### Pitfall 1: `rusqlite` 与 `sqlx` sqlite 的 `libsqlite3-sys` 冲突

**What goes wrong:** engine 选 `rusqlite`，Phase 3 storage 用 `sqlx` sqlite，workspace 解析失败。
**Why it happens:** `libsqlite3-sys` 声明 `links = "sqlite3"`，同一依赖图只允许一个版本；`rusqlite 0.40.2` 需要 `^0.38.2`，`sqlx-sqlite 0.9.0` 需要 `>=0.30.1, <0.38.0`。
**How to avoid:** engine 用 `sqlx`（`sqlite` feature，bundled）。
**Warning signs:** 实测报错原文 `package \`libsqlite3-sys\` links to the native library \`sqlite3\`, but it conflicts with a previous package which links to \`sqlite3\` as well` [VERIFIED: 本机 `cargo generate-lockfile`，probe 已删除]。

### Pitfall 2: SQLite 的 `ATTACH` 与内存库是「每连接」的

**What goes wrong:** 连接池第二个连接看不到 `demo__aviation`，或各连接的 `:memory:` 互相独立，测试偶发失败。
**Why it happens:** `ATTACH` 与 `:memory:` 数据库的生命周期绑在单个连接上（`after_connect` 对每个新连接各执行一次 [CITED: docs.rs sqlx PoolOptions::after_connect]）。
**How to avoid:** 替身固定 `max_connections(1)`，或用 `file:…?mode=memory&cache=shared` 共享缓存；fixture 加载用同一连接完成，加载后执行 `PRAGMA query_only = ON`（A7）再对外服务。SQLite 默认最多 attach 10 个库（上限 125）[CITED: sqlite.org/limits.html]，`catalog__schema` 数量超过 10 时在配置期报错而不是运行期。
**Warning signs:** 同一测试单跑通过、全量跑失败。

### Pitfall 3: `backing` 必填会改动大量 Phase 1 fixture

**What goes wrong:** 加了必填字段后 24 个 invalid 样例目录、aviation / aviation-json 示例、13 个 Rust 文件里的内联 YAML 全部多出 `ONT-V05`，快照大面积漂移。
**Why it happens:** Phase 1 的对象定义都没有该字段。`rg -l "kind: ObjectType"` 在仓库中命中 39 个文件（含 1 个设计文档），其中 invalid 样例 24 个文件（24 个目录）、Rust 源 / 测试 13 个，另有 JSON 写法的 aviation-json 示例 [VERIFIED: 本机 `rg -l` 计数，2026-10-16]。
**How to avoid:** 单独一个 plan：脚本批量给既有 fixture 加 `backing: editOnly`（M:N link 同理），一次提交内完成并确认 `cargo test --workspace` 快照零漂移，再引入新规则。aviation 例外，见 Open Question 4。
**Warning signs:** `invalid_examples` 快照出现与预期无关的 V05。

### Pitfall 4: 默认 `include` 会把绑定 YAML 当定义文件、把 JSON fixture 当定义文档

**What goes wrong:** `DEFAULT_INCLUDE = ["**/*.yaml", "**/*.yml", "**/*.json"]`（`discover.rs:14`）会选中 `bindings/` 下的 YAML（需要新 kind 才能解析，且 profile 要从路径推出）和 `fixtures/*.json`（被当作定义文档，报 `ONT-V02` 缺 `kind`）。
**How to avoid:** (1) dispatch 新增 `SourceBinding` 并带路径信息；(2) loader 把名为 `fixtures` 的目录加入跳过集合（现有 `SKIPPED_DIRS = ["target", "node_modules"]`，`discover.rs:17`，以及 `.` 开头目录）；(3) fixture 默认推荐 CSV。编辑器配置 glob `examples/ontologies/**/*.yaml` 已覆盖 `bindings/` 下的 YAML（见 `.zed/settings.json` / `.vscode/settings.json`），只要 `SourceBinding` 进了 `Definition` 的 JSON Schema `oneOf`，补全即生效；仍需一个回归测试（Phase 1 plan 14 的模式）。

### Pitfall 5: lint 的 `KNOWN_RULE_IDS` 是定长数组，且 E10..E12 / W12..W16 已被占用

**What goes wrong:** 直接新增 `ONT-E10` 与 foundry spec 规则表冲突；忘记更新 `KNOWN_RULE_IDS` 导致 `ontoforge.toml` 里的新规则被判未知（`ONT-V15`）。
**Evidence:** `KNOWN_RULE_IDS: [&str; 13]`（`lint/mod.rs:150-153`，值 `"ONT-E08", "ONT-E09", "ONT-W01" … "ONT-W11"`）；spec 表占用 `ONT-E10`（value type）、`ONT-E11`（interface）、`ONT-E12`（security policy）、`ONT-W12..W16`（`foundry-docs-spec.md` 规则表）。`CONTRIBUTING.md` 规定诊断码不得复用或改义。
**How to avoid:** 绑定类 lint 从 `ONT-E13`、`ONT-W17` 起；同步改数组长度与 `registry_ids_are_known_and_unique` 测试。

### Pitfall 6: `Finding` 以 `DefKey { kind, api_name }` 定位，绑定文档没有对应 `DefKind`

**What goes wrong:** lint / validate 想把诊断指到绑定文件 line:col，但 `DefKind` 只有 `ObjectType | LinkType | ActionType`（`sourcemap.rs:16-21`），`SourceMap::register_def` 以它为 key。
**How to avoid:** 加 `DefKind::SourceBinding`，`api_name` 取 `"{profile}/{target}"`（如 `dev/Flight`）；`dispatch` 的 `KINDS` / `IMPLEMENTED_KINDS`（`dispatch.rs:16-18`）同步加入；重复绑定（同 profile 同 target）报 `ONT-V08`。

### Pitfall 7: GitHub Actions `services:` 无法挂载仓库里的 Trino catalog 文件

**What goes wrong:** Trino 需要 `etc/catalog/*.properties`，service container 在 `checkout` 之前启动，拿不到仓库文件。
**How to avoid:** PG 用 `services: postgres`，Trino 用 checkout 之后的 `docker run -d --network host -v "$PWD/ci/trino/catalog:/etc/trino/catalog" trinodb/trino:483`，再轮询 `GET :8080/v1/info` 直到 `starting` 为 false（A5）；种子数据用 `psql -f ci/pg/seed.sql`。镜像标签存在性已核实：`484`、`483`、`482` …均在 Docker Hub [VERIFIED: hub.docker.com API，2026-10-16]。建议 PR job 钉 `483`（CONTEXT 记录的唯一真实生产集群版本），nightly 同时跑 `484`。
**Warning signs:** Trino 启动即退出，日志里 catalog 目录为空。

### Pitfall 8: `#[ignore]` 集成测试在 CI 里被静默跳过

**What goes wrong:** `cargo test --workspace` 默认不跑 `#[ignore]`；若 Linux 集成 job 忘了 `-- --ignored`，或环境变量缺失时测试直接 return，CI 绿但什么都没测。
**How to avoid:** 集成测试文件整体 `#[ignore = "needs ONTOFORGE_TRINO_URL"]`，读不到环境变量时 `panic!`（而不是 return）；job 命令固定为 `cargo test -p ontoforge-engine --test trino_it --locked -- --ignored`；同一 job 增加一步断言测试数量大于 0（`--list` 计数）。

### Pitfall 9: `ONTOFORGE_PROFILE` 与 `--profile` 的测试隔离

`set_var` 在 edition 2024 是 `unsafe`，被 `unsafe_code = "forbid"` 拦截（`CONTRIBUTING.md:27`）。profile 解析写成纯函数 `resolve_profile(flag: Option<&str>, env: Option<&str>, manifest: Option<&str>)`；CLI 测试经 `Command::env("ONTOFORGE_PROFILE", …)` 验证优先级。

### Pitfall 10: Windows 上连接被拒绝的耗时与端口占用

测试「Trino 不可达 → `EngineUnavailable`」时用「绑定后立即释放的本机端口」模拟，并把 `connect_timeout` 设为 1 s；Windows 对关闭端口的连接失败可能比 Linux 慢 [ASSUMED: 经验]，不要依赖默认超时，否则 `cargo test` 在 Windows 上偶发慢测。

### Pitfall 11: 密钥文件常是符号链接

Phase 1 的 `read_limited(path, follow_links=false)`（`loader/mod.rs:166`）会拒绝符号链接，这对定义文件是对的。`*_file` 密钥引用（Kubernetes secret 卷里是符号链接，A8）必须允许跟随符号链接，但仍限制大小（建议 64 KiB）、去掉末尾换行、读取失败的错误信息只含「文件路径」不含内容。`ca_file` 同理。

### Pitfall 12: 远端返回的字符串进入诊断

`information_schema` 的列名、Trino `error.message` 是不可信输入，可能含终端控制序列。诊断经 `render::sanitize`（`render.rs`，替换 C0 / C1 / bidi 控制字符）输出，JSON 由 `serde_json` 转义；新诊断必须走现有 `Diagnostic` 渲染，不得 `println!` 直出。

## Code Examples

### Engine trait 与错误分类（形态建议）

```rust
// Source: 本研究设计（原生 async fn in trait，不引入 async-trait）
pub trait Engine {
    async fn execute(&self, sql: &PrintedSql, ctx: &QueryContext) -> Result<QueryOutput, EngineError>;
    async fn describe_table(&self, t: &TableRef, ctx: &QueryContext)
        -> Result<Option<Vec<PhysicalColumn>>, EngineError>;   // None = 表不存在
    async fn status(&self) -> EngineStatus;                    // 探针：SELECT 1
    fn dialect(&self) -> Dialect;
}
pub struct QueryContext { pub trace_token: String, pub client_tags: Vec<String>,
                          pub timeout: Duration, pub max_rows: usize, pub max_bytes: usize }
pub struct QueryOutput { pub columns: Vec<Column>, pub rows: Vec<Vec<Value>>, pub truncated: bool }
pub enum EngineError { Unavailable{ retryable: bool, .. }, Unauthorized, Timeout, Cancelled,
                       Query{ error_name: String, .. }, Protocol(String), Print(PrintError) }
pub enum AnyEngine { Trino(TrinoEngine), Sqlite(SqliteEngine) }   // 运行时按 profile 选
```

`Unavailable` 覆盖：连接失败 / DNS / TLS 握手失败 / 重试耗尽后的 502 / 503 / 504 / Trino `SERVER_STARTING_UP`、`SERVER_SHUTTING_DOWN`（这两个 errorName 在 `StandardErrorCode` 中存在 [VERIFIED: raw.githubusercontent.com trinodb/trino master StandardErrorCode.java:175,178]）。401 / 403 单独归 `Unauthorized`，CLI 同样退出码 3 但消息不同。用户类错误（`TABLE_NOT_FOUND`、`COLUMN_NOT_FOUND`、`SCHEMA_NOT_FOUND`、`CATALOG_NOT_FOUND`，同文件 :70, :71, :69, :68）在 `describe_table` 里转为 `Ok(None)` / 绑定不匹配，而不是 `Unavailable`。

### 服务配置（`ontoforge-server.toml`）形态建议

```toml
[trino]
url      = "https://trino.example.com:8443"
ca_file  = "certs/ca.pem"          # 可选；不存在「跳过校验」开关
timeout  = "30s"

[trino.identities.ontology]
user         = "ontoforge"
auth         = "basic"             # basic | none | bearer
password_env = "ONTOFORGE_TRINO_PASSWORD"   # *_env 或 *_file，二选一；出现 password = "…" 即 deny_unknown_fields 报错

[trino.identities.l2]               # 本 phase 只解析并能实例化独立 client；是否启用留 Phase 6
user = "ontoforge_l2"
auth = "bearer"
token_file = "/run/secrets/l2-token"

[profiles.dev]
engine = "sqlite"
[profiles.prod]
engine = "trino"
```

`deny_unknown_fields` + 自定义错误提示「配置文件不接受明文密钥，请用 `password_env` / `password_file`」；`bearer` / `basic` 在 `http://` URL 下直接配置期报错（Trino 服务端默认也不接受明文密码，A4）。

### wiremock 验证「断连取消」（形态）

```rust
// 流程：POST 返回含 nextUri 的 QUEUED；GET 返回仍含 nextUri 的 RUNNING；
// 在第一次 GET 后 drop 查询 future；断言收到一次 DELETE 到最新 nextUri，且 Authorization 头未发往别的 host。
let cancelled = Mock::given(method("DELETE")).and(path("/v1/statement/q1/2"))
    .respond_with(ResponseTemplate::new(204)).expect(1);
```

### proptest：任意字符串往返不改变结构（形态）

```rust
// 1) sqlparser 往返: 生成 `SELECT a FROM t WHERE b = <lit(s)>`，GenericDialect 解析为恰 1 条语句，
//    抽出右值字面量 == s；并把 s 换成固定字符串 "x" 后的 AST 与原 AST 逐节点相等(结构不变)。
// 2) SQLite 执行: `SELECT <lit(s)>` 回读 == s（s 含 NUL 时 printer 必须 Err）。
// 3) 标识符: `SELECT 1 AS <ident(s)>` 回读列名 == s。
// Trino 端只跑固定语料: 引号、反斜杠、`--`、`/*`、`;`、U&、换行、emoji、超长串(D23)。
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| 绑定内联在 object type（ADR-0015 原文） | 独立 `SourceBinding` 文件 + `backing` 语义字段（D-01 / D-02） | 2026-10-16 讨论 | 需新 ADR（建议 ADR-0021）supersede 该条 |
| 现成 `trino-rust-client` | 自写薄 client | spike 1 | 现成库三次破坏性变更且共享 session，见 spike 1 §4 |
| Trino 483 | Trino 484 为 `current` 文档与 `latest` 镜像（2026-10-16） | 本月 | CI 同时覆盖 483 与 484 |
| PG `NUMERIC` 无界映射为不可用 | Trino 484 起映射为 `NUMBER` 类型 | 近期版本 | `PhysicalType` 需 `Number` 变体 [CITED: trino.io PostgreSQL connector 文档] |

**Deprecated/outdated:**
- `rusqlite` 作为 workspace 的 SQLite 驱动：与 `sqlx` 不能并存（Pitfall 1）。
- 启动期全量查重 pk（ADR-0015 已 Rejected）：`--check-pk` 仅显式触发。

## Runtime State Inventory

跳过：本 phase 是新增功能，不是 rename / refactor / migration。唯一接近的是 `backing` 必填迁移，其影响面是仓库内文件（已在 Pitfall 3 枚举），无运行时存储、OS 注册、外部服务配置或构建产物携带旧名字。

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | 全部 | ✓ | rustc / cargo 1.98.0 | — |
| C 编译器（MSVC） | bundled SQLite、aws-lc-sys | ✓ | probe `cargo check` 通过 | — |
| Docker daemon | 本地跑 Trino / PG | ✗ | client 29.7.2，daemon 未运行（`docker info` 退出码 1） | Windows 本机用 wiremock + SQLite 替身；真实 Trino 只在 Linux CI |
| Java | 本地跑 Trino | 部分 | JDK 17（Trino 483/484 的要求未核实） | 不在本机跑 Trino |
| `psql` | CI 灌 PG 数据 | ✗（本机） | — | CI runner 自带或用 `docker exec`；本机不需要 |
| `gh` CLI | nightly 失败自动开 issue（在 Actions 里用） | ✓（本机 2.96.0） | — | Actions runner 自带 `gh` |
| 外部 Trino（私有 483 集群） | 私有验证 | 不使用 | — | 只读使用，凭据待轮换（STATE.md Blockers）；研究与公开 CI 均不连接 |

**Missing dependencies with no fallback:** 无阻塞项。
**Missing dependencies with fallback:** Docker / 真实 Trino：用 wiremock 与 SQLite 覆盖 Windows 本机；Linux CI 兜底。

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` + `insta` 1.49.0（快照）+ `proptest` 1.12.0 + `wiremock` 0.6.5 + `sqlx` sqlite（测试内造表） |
| Config file | 无独立配置；`#[ignore]` 门控真实 Trino 测试；`PROPTEST_CASES` 环境变量调例数 |
| Quick run command | `cargo test -p ontoforge-engine` 或 `cargo test -p ontoforge-schema` |
| Full suite command | `cargo test --workspace --locked`（Windows 与 Ubuntu 均跑，不需要 Docker） |
| Integration（仅 Linux CI） | `cargo test -p ontoforge-engine --test trino_it --locked -- --ignored`，环境变量 `ONTOFORGE_TRINO_URL` 等 |
| Gate extras | `cargo fmt --all --check`；`cargo clippy --workspace --all-targets --locked -- -D warnings` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| BIND-01 | `SourceBinding` 解析（object / M:N link）、`backing` 声明、profile 由路径推出、`source` 无绑定 / `editOnly` 有绑定报 E | unit + integration + insta | `cargo test -p ontoforge-schema --test bindings` | ❌ Wave 0 |
| BIND-01 | aviation 示例带绑定后 `validate` 与 `lint` 仍 clean，JSON Schema 快照含新 kind | CLI e2e + snapshot | `cargo test -p ontoforge --test cli --test cli_lint --test cli_schema` | ✅ 需更新 |
| BIND-02 | decimal 范围、`timezone` 缺失 / 非法、collation 非法、geopoint 绑定各一个 invalid 样例与 lint 单测 | invalid 样例 + insta | `cargo test -p ontoforge --test invalid_examples`；`cargo test -p ontoforge-schema lint::` | ✅ 框架在；样例 ❌ Wave 0 |
| BIND-03 | `compare_binding` 全部不匹配分支（表缺失、列缺失、有损类型、精度变化、timezone 与列类型不一致） | unit（纯函数） | `cargo test -p ontoforge-engine bind_check` | ❌ Wave 0 |
| BIND-03 | `validate --online` 对 SQLite 替身：绑定正确时 0，列缺失时 1 且诊断指向绑定文件 line:col；`--check-pk` 对重复 pk 报 E；Trino 不可达退出码 3 | CLI e2e（测试内用 sqlx 造表） | `cargo test -p ontoforge --test cli_online` | ❌ Wave 0 |
| BIND-03 | drift 降级状态：`DriftReport` 把 mismatch 折叠为 `Degraded{SourceSchemaDrift}`，其余 type 仍 Healthy | unit | `cargo test -p ontoforge-engine drift` | ❌ Wave 0 |
| ENGINE-01 | 头集合（User / Source / Trace-Token / Client-Tags / TZ）、Basic / Bearer / none、429 Retry-After、502-504 重试、`DELETE nextUri`（超时、future 被 drop、行数上限）、`nextUri` 异源拒绝、两个身份各自独立 client | integration（wiremock） | `cargo test -p ontoforge-engine --test trino_client` | ❌ Wave 0 |
| ENGINE-01 | 真实 Trino 上的一次查询、超时取消、trace token 出现在 `system.runtime.queries` | integration（Linux CI） | `cargo test -p ontoforge-engine --test trino_it -- --ignored` | ❌ Wave 0 |
| ENGINE-02 | 连接被拒 / DNS 失败 / 503 耗尽重试 / `SERVER_STARTING_UP` 全部归为 `EngineError::Unavailable`；`status()` 探针 | integration（wiremock + 本机关闭端口） | `cargo test -p ontoforge-engine --test trino_client unavailable` | ❌ Wave 0 |
| ENGINE-03 | 两个转义函数 + `PrintedSql` 私有构造；任意字符串 / 标识符往返不改变结构（sqlparser）；SQLite 执行回读一致；含 NUL 被拒；源码级检查 printer 外不拼 SQL | proptest + unit | `cargo test -p ontoforge-engine --test printer_prop` | ❌ Wave 0 |
| ENGINE-03 | Trino 端固定语料与历史反例 | integration（Linux CI） | 同 trino_it | ❌ Wave 0 |
| ENGINE-04 | fixture（CSV / JSON）加载进 `catalog__schema`；null、decimal 边界、时区、重复 pk 行；`query_only` 生效；超过 attach 上限的配置期报错 | integration（SQLite） | `cargo test -p ontoforge-engine --test sqlite_engine` | ❌ Wave 0 |
| ENGINE-05 | CI workflow 结构：PR job 含 Trino + PG，nightly 含 Iceberg 且失败开 issue；actions 钉 commit SHA、`permissions` 最小化 | file check（rg / 结构测试） | `rg -n "trinodb/trino" .github/workflows/ci.yml` 与工作流实际运行 | ❌ Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test -p <被改动的 crate>`
- **Per wave merge:** `cargo test --workspace --locked` + clippy + fmt
- **Phase gate:** 全套件绿（Windows 与 Ubuntu），Linux 集成 job 绿（`--ignored` 实际执行了 ≥ 1 个测试），`/gsd-verify-work` 前完成
- **最大反馈延迟:** 120 s（含首次编译 aws-lc-sys / SQLite bundled 的冷构建除外，probe 增量检查 26 s）

### Wave 0 Gaps

- [ ] `crates/ontoforge-engine/`（Cargo.toml、lib.rs、测试目录）与 workspace 依赖项
- [ ] `crates/ontoforge-schema/tests/bindings.rs` 与 `examples/ontologies/invalid/` 下的绑定 / 类型契约样例目录（目录名前缀决定期望诊断码，见 `CONTRIBUTING.md`）
- [ ] `crates/ontoforge/tests/cli_online.rs`（测试内用 `sqlx` 在 tempdir 造 SQLite 表，不依赖 Docker）
- [ ] `ci/trino/catalog/*.properties`、`ci/pg/seed.sql`、`.github/workflows/nightly.yml`
- [ ] 既有 fixture 的 `backing` 迁移（Pitfall 3）作为首个 plan

## Security Domain

`security_enforcement` 未显式关闭，`security_asvs_level: 1`，`security_block_on: high`。

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes（对 Trino 的服务端凭据） | Basic 仅 HTTPS、bearer、none 仅本地 / CI；凭据只经 `*_env` / `*_file`，`secrecy` 包裹 |
| V3 Session Management | no（无会话；Trino 响应的 `X-Trino-Set-*` 不回写） | 无状态 client |
| V4 Access Control | 部分（两个服务身份各自独立凭据 / 连接池） | L2 身份启用留 Phase 6 |
| V5 Input Validation | yes | printer 两个转义函数 + 类型化 AST；定义 / 绑定 / 配置 `deny_unknown_fields`；profile 名白名单且不进路径 |
| V6 Cryptography | yes | rustls（reqwest）+ 平台验证器；**不提供** `danger_accept_invalid_certs`，建议用 clippy `disallowed-methods` 禁用该方法（A10）；不手写加密 |
| V7 Error Handling / Logging | yes | 密钥不进日志 / 诊断 / 错误 / query text；远端字符串经 `sanitize` |
| V9 Communications | yes | HTTPS 下才允许带凭据；`redirect::Policy::none()`；`nextUri` 同源校验 |
| V12 Files | yes | fixture / `ca_file` / `*_file` 路径不得逃逸（fixture 复用 `escapes_root` 逻辑，密钥文件允许符号链接但限大小）；4 MiB 文件上限沿用 |
| V14 Configuration | yes | 配置文件不接受明文密钥；`Cargo.lock` 入库，CI `--locked` |

### Known Threat Patterns for Rust + 外部 SQL 引擎 client

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| SQL 注入（Trino 无带外参数绑定） | Tampering | 类型化 AST + `quote_string` / `quote_ident` 唯一出口；拒绝 NUL；proptest + 真实执行验证；`PrintedSql` 私有构造 |
| 凭据经被篡改的 `nextUri` 外泄 | Information Disclosure | 同源校验 + 不跟随重定向 |
| 密钥进日志 / 错误信息 / query text | Information Disclosure | `secrecy`；错误类型不含 header 值；测试断言诊断与 Debug 输出不含密钥 |
| 失控查询占用共享集群 | DoS | 客户端超时 + `DELETE nextUri` + session `query_max_execution_time` 兜底；单页响应体上限；`--check-pk` 显式开关 |
| fixture / 配置路径遍历与超大文件 | Tampering / DoS | 相对路径限制、4 MiB 上限、`.csv` / `.json` 白名单；行数上限 |
| 远端元数据含终端控制序列 | Spoofing | 诊断统一经 `sanitize` |
| SQLite 替身被注入后写盘 | Tampering | 加载后 `PRAGMA query_only = ON`；库为内存库 |
| 供应链（reqwest / sqlx / aws-lc-sys build.rs） | Tampering | 仅 legitimacy `OK` 的包；`Cargo.lock` + `--locked`；CI action 钉 SHA（沿用 Phase 1 做法） |
| nightly 开 issue 的权限提升 | EoP | `nightly.yml` 独立 `permissions: { contents: read, issues: write }`，不用 `pull_request_target` |

## Open Questions

1. **无时区 timestamp 的 `timezone` 在离线 lint 里如何判定？**
   - What we know: ADR-0015 与 Roadmap 都要求「源列是无时区 timestamp 且未声明 `timezone` → lint E」；但离线时 lint 看不到源列物理类型。
   - What's unclear: 谁来判定「源列是否带时区」。
   - Recommendation: 离线规则改为「source-backed 对象的每个 `timestamp` 属性，其 `columns` 条目必须显式声明 `timezone`（IANA 名，或 `aware` 表示源列自带时区）」，缺失为 E；在线再核对：声明 IANA 而源列带时区 → W，声明 `aware` 而源列无时区 → E。这样「E」在离线可判定，在线补验。需在新 ADR 里写明该解释（对 ADR-0015 措辞的细化，不改变意图）。

2. **没有 `ontoforge-server.toml` 时 `--online` 选哪个引擎？**
   - What we know: D-06 把 profile→引擎映射放在服务配置；D-05 说示例缺省 profile `dev` 指向 SQLite 替身；示例应当零配置跑通。
   - Recommendation: 无服务配置文件时，若选定 profile 的绑定全部带 `fixture`，则使用内存 SQLite 替身；否则报错并提示 `--server-config`。有配置时以配置为准，配置映射到 Trino 而绑定带 `fixture` 时给 W。

3. **`Engine` 的返回形态。**
   - Recommendation: 整批返回，client 内强制 `max_rows` / `max_bytes` 并在命中时 `DELETE`、置 `truncated`。理由：ADR-0020 的 Trino 查询都有界（TopN over-fetch、≤256 pk 批、聚合）。Phase 4 若出现无界扫描需求再加流式方法，不破坏现有 trait。

4. **aviation 示例的改动范围。**
   - What we know: `Airport.location` 是 geopoint，source 绑定不支持（E）；示例没有 decimal 属性，而 D-12 要求 fixture 覆盖 decimal 边界；示例有 `timestamp` 属性（`scheduledDeparture`、`performedAt`），需要 `timezone`；`flightCrew` 是 M:N。
   - Recommendation: `Airport` 设为 `backing: editOnly`（参考数据由 Action 维护），其余 type 为 `source`；给 `MaintenanceRecord` 增加 `cost: { type: decimal, precision: 12, scale: 2 }`；重复 pk、schema drift 的用例**不放进示例 fixture**，改为测试内在 tempdir 用 `sqlx` 现造 SQLite 表，保持示例是干净的演示数据。示例使用中性命名（catalog `demo`、schema `aviation`），不含任何公司名。

5. **`backing` 必填还是可选。**
   - Recommendation: 按 D-02 必填（M:N link 同样）。理由：可选默认 `editOnly` 无法拦住「同时漏写 backing 与绑定」。代价是 Pitfall 3 的迁移，作为首个 plan 一次完成。新 ADR 需要写明这是对 ADR-0012「后续字段为向后兼容扩展」惯例的有意例外。

6. **D-13 的 decimal 规则字面读法有缺口。**
   - Recommendation: 采用整数位规则（`s ≤ S` 且 `p - s ≤ P - S`），并在新 ADR / 绑定规范里注明替换了「精度与小数位不超过声明值」的直读。

7. **ENGINE-02 的完成口径。**
   - 本 phase 交付：`EngineError::Unavailable` 分类、`Engine::status()`、测试。`/health` 与 describe 暴露属 Phase 7。建议 REQUIREMENTS traceability 在 Phase 2 完成后将 ENGINE-02 标「部分」或把暴露面拆到 Phase 7 的 API-01，避免验收时歧义。

8. **fixture 空单元格语义。**
   - Recommendation: CSV 空单元格 = NULL（所有类型含 String），JSON fixture 用真 `null`；写进绑定规范。

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | reqwest 0.13 异步 `ClientBuilder` 上自定义 CA 的方法是 `tls_certs_merge`（docs.rs 在 blocking builder 文档中出现；async 同名未逐一核对） | Don't Hand-Roll | 编译期立刻暴露，成本低 |
| A2 | `chrono-tz` 可用 `default-features = false` 编译并满足时区名校验 | Standard Stack | 退回默认 feature，仅影响编译体积 |
| A3 | Trino `information_schema.columns` 的列集与 `data_type` 字符串形态（`decimal(10,2)`、`timestamp(3) with time zone`、`array(varchar)`、`varchar(255)`，小写）；来源为 WebSearch 摘要，未在真实 Trino 上取证 | Pattern 6 | 类型解析器误判；Linux CI golden 表会暴露 |
| A4 | Trino 默认拒绝明文 HTTP 上的密码认证 | 服务配置 | 配置期校验过严或过松 |
| A5 | 启动完成可通过 `GET /v1/info` 的 `starting` 字段判断；`DELETE nextUri` 返回 204 | Pitfall 7、Pattern 4 | CI 等待逻辑需调整；取消成功判定需放宽 |
| A6 | SQLite 在 SQL 文本遇到 NUL 时会截断 | Pattern 5 | 若不截断，拒绝 NUL 仍是安全保守选择，无负面影响 |
| A7 | `PRAGMA query_only = ON` 对该连接上所有 attach 库生效 | Pitfall 2 | 防御层失效；仍有「printer 是唯一入口」主防线 |
| A8 | Kubernetes secret 卷中的文件是符号链接 | Pitfall 11 | 若不是，允许跟随符号链接是多余但无害 |
| A9 | Trino 裸小数字面量（如 `1.5`）默认是 DECIMAL 而非 DOUBLE | Pattern 5 | printer 输出的 double 字面量语义偏差；一律输出带类型前缀可规避 |
| A10 | clippy `disallowed-methods` 可以按路径禁用 `reqwest::ClientBuilder::danger_accept_invalid_certs` | Security | 退回代码评审 + grep 测试 |
| A11 | Trino 各类型 JSON 线格式（尤其 `timestamp(p) with time zone` 的 zone id / 偏移量两种形态、NaN / Infinity 字符串）与 spike 1 一致 | Pattern 4 | 值解码错误；Linux CI golden 表暴露 |
| A12 | MinIO 作 Iceberg JDBC catalog 的 S3 属性（`fs.native-s3.enabled`、`s3.endpoint`、`s3.path-style-access`、`s3.region`、`s3.aws-access-key`、`s3.aws-secret-key`）；仅 JDBC catalog 自身的必填项 `iceberg.jdbc-catalog.{driver-class,connection-url,default-warehouse-dir,catalog-name}` 已由文档核实 | CI | nightly 首次需要调通 |
| A13 | 客户端重试策略：GET `nextUri` 遇 502 / 503 / 504 无条件重试；POST 仅在连接失败或明确未处理（例如 429、503 `SERVER_STARTING_UP`）时重试 | Pattern 4 | 重复提交非幂等语句；本 phase 只发 SELECT，影响有限 |
| A14 | sqlx 的 sqlite `query()` 对含多条语句的文本会逐条执行 | Security | 若不是，则更安全；printer 单语句保证 + `query_only` 已覆盖 |
| A15 | Windows 上对关闭端口的连接失败耗时较长 | Pitfall 10 | 只影响测试耗时 |
| A16 | 单页响应体上限建议 64 MiB、`nextUri` 轮询退避从 50 ms 起指数增至 1 s | Pattern 4 | 调参问题（Claude's Discretion 范围内） |

## Sources

### Primary (HIGH confidence)

- 仓库文件（本会话 Read）：`02-CONTEXT.md`、`REQUIREMENTS.md`、`STATE.md`、`ROADMAP.md` Phase 2、ADR-0002 / 0003 / 0004 / 0011 / 0012 / 0015 / 0016 / 0017 / 0020、`docs/designs/trino-engine-sql-tiers.md`（D1-D26）、`spike-01..03b`、Phase 1 代码：`diag.rs`、`loader/{mod,dispatch,discover,sourcemap}.rs`、`ir/*.rs`、`lint/mod.rs`、`schema_export.rs`、`ontoforge/src/*`、`ci.yml`、`CONTRIBUTING.md`
- 本机实验：`cargo generate-lockfile`（rusqlite + sqlx 冲突实测）；候选依赖组合 probe 的 `cargo tree` / `cargo check`（Windows，26 s）；`libsqlite3-sys-0.37.0/sqlite3/sqlite3.h:149`；`docker info`；`gsd-tools package-legitimacy`
- crates.io API：`rusqlite/0.40.2` 与 `sqlx-sqlite/0.9.0` 的 `libsqlite3-sys` 依赖区间
- Docker Hub API：`trinodb/trino` 标签 `484` / `483` / `482`…
- trinodb/trino `StandardErrorCode.java`（raw.githubusercontent.com）：errorName 值与编号

### Secondary (MEDIUM confidence)

- [Trino client REST API](https://trino.io/docs/current/develop/client-protocol.html)（生命周期、重试状态码、头）
- [Trino query management properties](https://trino.io/docs/current/admin/properties-query-management.html)（`query_max_execution_time`）
- [Trino PostgreSQL connector](https://trino.io/docs/current/connector/postgresql.html)（类型映射、`NUMBER`、`array-mapping`、catalog 属性）
- [Trino data types](https://trino.io/docs/current/language/types.html)（类型集合、`NUMBER`、decimal ≤ 38）
- [Trino metastores / Iceberg JDBC catalog](https://trino.io/docs/current/object-storage/metastores.html)（JDBC catalog 属性名）
- [SQLite limits](https://www.sqlite.org/limits.html)（`SQLITE_MAX_ATTACHED` 默认 10、上限 125）
- [docs.rs reqwest tls](https://docs.rs/reqwest/latest/reqwest/tls/index.html)、[sqlx PoolOptions::after_connect](https://docs.rs/sqlx/latest/sqlx/any/type.AnyPoolOptions.html)（经 Context7）

### Tertiary (LOW confidence)

- WebSearch 摘要：`information_schema.columns` 列集（A3）、`X-Trino-Session` 值 URL 编码（来自 trino-go-client README 的转述，未取原文）

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH，依赖冲突与组合编译均本机实测，legitimacy 全部 OK
- Architecture: MEDIUM-HIGH，Phase 1 接入点逐文件核对；`Engine` 形态与 Open Questions 属设计推荐
- Pitfalls: MEDIUM-HIGH，1 / 3 / 4 / 5 / 6 有代码或实验证据；Trino 线格式与 CI 编排（A3 / A5 / A11 / A12）需 Linux CI 首跑确认

**Research date:** 2026-10-16
**Valid until:** 2026-10-30（Trino 约每 6 周发版；reqwest / sqlx 0.x 变动快，Wave 0 前重新 `cargo search` 一次）
