# Phase 2: Source Binding & Engine - Context

**Gathered:** 2026-10-16
**Status:** Ready for planning

<domain>
## Phase Boundary

开发者可以把 object type / M:N link 绑定到 Trino 物理表（按 profile 区分部署环境）；新 crate `ontoforge-engine` 提供自写 Trino client、自有 SQL printer 与 SQLite 替身引擎；`ontoforge validate --online` 对真实 Trino（或替身）校验绑定。Windows 本机零外部依赖 `cargo test` 全绿，Linux CI 跑 Trino + PG 集成测试。

需求：BIND-01..03、ENGINE-01..05（见 `.planning/REQUIREMENTS.md`）。

不在本 phase：edit store、scope / 授权、读路径叠加、Action、L2 执行与自检、服务进程（`serve`、`/health`、定时任务）。

</domain>

<decisions>
## Implementation Decisions

### 数据源绑定的位置与形状
- **D-01:** 绑定写在**独立的绑定文件**里（新 kind，暂名 `SourceBinding`），按 profile 分目录（如 `bindings/dev/`、`bindings/prod/`）。ontology 定义文件保持纯语义、不出现任何物理 catalog / schema / 表 / 列名，开源示例因此不含任何公司命名。— **Reversibility:** costly — 改变定义文件格式、JSON Schema 与 loader；**修订 ADR-0015**（其原文「object type 增加可选 `source`」改为独立绑定文件），需新写一份 ADR supersede ADR-0015 的该条，其余类型契约条款不变。
- **D-02:** ObjectType（以及 M:N link）在 ontology 中**显式声明 backing**（暂名 `backing: source | editOnly`，纯语义字段）。声明 `source` 但选定 profile 中没有绑定 → E；声明 `editOnly` 却有绑定 → E。漏配不能静默变成「没有数据的 edit-only 对象」。— **Reversibility:** costly — 定义格式字段，示例与 schema 都要带上。
- **D-03:** 列映射：未列出的属性按同名列匹配，只写不同名的列；配一条 W 级 lint 提示「属性在绑定表中找不到对应列」（离线可判定的部分；在线不存在的列由 `validate --online` 报 E）。
- **D-04:** 一个 object type 只绑定单表或 view，不允许 SQL 片段作为源。多表拼接由部署方在 Trino 建 view。与 ADR-0020「Trino 只跑单表查询」一致。
- **D-05:** profile 选定：`--profile <name>` 或环境变量 `ONTOFORGE_PROFILE`，都未指定时用 ontology manifest（`ontology.yaml`）中声明的缺省 profile。示例 ontology 的缺省 profile 为 `dev`，指向 SQLite 替身。

### Trino 连接与凭据
- **D-06:** Trino 连接配置放在**独立的服务配置文件**（暂名 `ontoforge-server.toml`），与 ontology 定义和 lint 配置 `ontoforge.toml` 分开：地址、TLS、超时、两个服务身份、profile → 引擎映射。定义可公开共享，部署配置不共享。
- **D-07:** 密钥只以 `*_env`（环境变量名）或 `*_file`（文件路径）引用，配置文件不接受明文；密钥不进日志、诊断、错误信息，也不进发给 Trino 的 query text。
- **D-08:** 本 phase 的 Trino client 支持三种认证：用户名密码（Basic，仅 HTTPS）、无密码（仅 `X-Trino-User`，供本地 / CI 容器）、JWT / OAuth2 bearer。Kerberos 不做。
- **D-09:** TLS：可配置自定义 CA（`ca_file`），**不提供**跳过证书校验的开关。
- **D-10:** 两个服务身份（ontology 身份、L2 身份）在本 phase 的配置模型与 client 实例化中都支持（各自独立连接池 / 凭据）；L2 是否启用与启动自检留到 Phase 6。

### SQLite 替身的数据来源
- **D-11:** 替身的「source 表」数据来自示例自带的 CSV / JSON fixture：dev profile 的绑定指向 fixture 文件，加载进 `ATTACH` 的 `catalog__schema` 库，列类型由绑定 + IR 推出。测试与后续 `serve --dev` 共用同一机制。
- **D-12:** aviation 示例的 fixture 手写、小规模（每个 type 几十行），刻意覆盖 null、decimal 边界、时区、重复 pk 等用例所需的行；数据完全虚构，与任何公司无关。

### validate --online 与 drift
- **D-13:** 类型兼容只接受**无损映射**：数值只可加宽、decimal 的精度与小数位不超过声明值、varchar 只映射到 String 等。有损映射报 E；可保留但精度变化报 W。映射表写入绑定规范文档。
- **D-14:** `validate --online` 复用 Phase 1 诊断体系：新增在线诊断码（编号由 planner 在 ONT-V16 之后分配），定位到绑定文件 line:col，支持 human / JSON 两种格式；有 E 时退出码非 0；Trino 不可达使用单独的退出码，与「绑定错误」区分。
- **D-15:** drift 校验在本 phase 交付库函数与按 object type 的降级状态模型（healthy / degraded + 原因），CLI 与测试共用；启动 / 定时执行与 `/health` 暴露在 Phase 7 有服务进程时接入。

### Claude's Discretion
- Trino 客户端协议细节（`nextUri` 轮询退避参数、连接池大小、各类 header 的取值）。
- 自有 printer 的 API 形态与模块划分，只要满足「值只经两个转义函数输出」。
- `Engine` trait 的接口形态（流式返回还是整批返回），依据 Phase 4 的分批回查需求自行判断。
- 绑定文件与服务配置文件的最终命名、字段命名（遵守 ADR-0002 命名规范）。
- CI 中 Trino / PG 容器版本与测试数据灌入方式。

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 方向与已锁决策
- `docs/designs/trino-engine-sql-tiers.md` — 设计文档；「Eng Review 决策」D1–D26 为准（尤其 D8、D14、D18、D23、D24、D25）
- `.planning/adr/0015-source-binding-and-type-contract.md` — 绑定与类型契约（**D-01 修订其「内联 source」一条**，其余有效）
- `.planning/adr/0017-engine-and-trino-connection.md` — 引擎 crate、自写 client、自有 printer、两个服务身份、CI 矩阵
- `.planning/adr/0020-query-execution-rust-orchestrated-overlay.md` — Trino 查询形态约束（单表、≤ 2 fragment），影响 Engine trait 与 printer 覆盖范围
- `.planning/adr/0002-naming-convention.md`、`.planning/adr/0012-definition-document-shape.md` — 新 kind 与字段的命名、文档形状
- `.planning/adr/0011-workspace-layout.md` — 新增 `ontoforge-engine` crate 的布局约定

### 前置调研
- `.planning/research/spike-01-trino-client.md` — Trino HTTP 协议要点、自写 client 设计与风险
- `.planning/research/spike-02-sqlparser-l2.md` — sqlparser 的 Display 缺陷（G11 / G16 / G37），说明为何生成路径不用它
- `.planning/research/spike-03-trino-overlay-latency.md`、`.planning/research/spike-03b-scoped-overlay.md` — connector 下推能力（含 `sum(integer)` 不下推、256 值动态过滤上限），影响 printer 生成形态
- `.planning/research/ref-mcp-trino.md` — 连接配置面参照（只借设计）

### 需求
- `.planning/REQUIREMENTS.md` — BIND-01..03、ENGINE-01..05

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/ontoforge-schema/src/loader/`（discover / dispatch / parse / sourcemap）：多文件、按 `kind` 分派、带 span 的加载管线；绑定文件作为新 kind 接入 `dispatch`，line:col 诊断直接复用 `SourceMap`。
- `crates/ontoforge-schema/src/diag.rs`：诊断码表（当前到 ONT-V15）与渲染；在线诊断沿用。
- `crates/ontoforge-schema/src/ir/basetype.rs`：BaseType 与字面量校验；D-13 的类型映射表以此为属性侧类型。
- `crates/ontoforge-schema/src/lint/`：规则注册、`ontoforge.toml` 配置；D-03 的 W 规则按现有 naming / structure 模式加入。
- `crates/ontoforge-schema/src/schema_export.rs` + `schemas/`：新 kind 与新字段须进入导出的 JSON Schema 与快照测试，编辑器配置（`.zed`、`.vscode`）的 glob 需覆盖绑定文件目录。
- `crates/ontoforge/src/`（cli / pipeline / render）：`validate --online` 与 `--profile` 在现有 `validate` 子命令上扩展。

### Established Patterns
- IR 是唯一契约，`deny_unknown_fields`；新字段必须进 IR 与 JSON Schema，并有 insta 快照。
- 安全基线：32 MiB 栈线程、4 MiB 文件上限、控制字符净化、`Cargo.lock` 入库、新依赖需 legitimacy 审核（Phase 1 SECURITY.md）。
- Windows 本机 `cargo test` 不依赖 Docker；需要 Trino / PG 的测试放在 Linux CI 或 feature / env 门控之后。

### Integration Points
- 新 crate `crates/ontoforge-engine/`（workspace 成员）；`ontoforge`（CLI）依赖它实现 `validate --online`。
- `crates/ontoforge-storage/Cargo.toml` 目前依赖 `ontoforge-query`；Phase 2 不改 storage，但 engine 不应依赖 query（ADR-0020 crate 方向：engine 只接收渲染好的 SQL / AST）。
- `.github/workflows/ci.yml`：新增 Linux Trino + PG service container 作业与 nightly Iceberg 作业。
- `examples/ontologies/aviation/`：加 `backing` 字段、`bindings/dev/` 绑定与 fixture 数据。

</code_context>

<specifics>
## Specific Ideas

- 通用系统原则：仓库公开部分不得出现公司 catalog / 表名；公司环境验证用 `ONTOFORGE_PRIVATE_FIXTURES` 指向仓库外的私有绑定与 fixture。
- 已有一台 Trino 483 生产集群可做私有验证（大量 sqlserver catalog、iceberg、iceberg_rest）；只读使用。
- 用户偏好：绑定与部署配置分离，「定义可以开源共享，部署配置不共享」。

</specifics>

<deferred>
## Deferred Ideas

- Kerberos 认证 — 有需求时再做。
- drift 校验的定时执行、`/health` 暴露 — Phase 7（服务进程）。
- L2 身份的启用与启动自检 — Phase 6。
- 按主体透传 Trino 身份（impersonation）— 依赖 JWT 支持，后续评估。

</deferred>

---

*Phase: 02-source-binding-engine*
*Context gathered: 2026-10-16*
