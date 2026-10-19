# Phase 3: Edit Store & Scope Model - Research

**Researched:** 2026-10-19
**Domain:** Rust 双后端（PostgreSQL / SQLite）强类型 edit store + DDL 演进 + 不可变 edit log；CEL 行谓词的 SQL 编译与 Rust 单行求值（Kleene 三值逻辑、差分等价）；roles-as-code；OIDC JWT / dev token 认证
**Confidence:** HIGH（外部事实均已对照文档 / crate 源码 / 本机实测；剩余风险集中在 Trino `CALL` 经现有 client 的行为与 postgresql_embedded 的 Windows 首次运行，已列入 Assumptions Log）

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**已锁定、不再讨论**（ADR-0016、ADR-0018、设计文档 Eng Review D3 / D5 / D12 / D13 / D17 / D19 / D20 / D23）：强类型分表、按属性 patch 叠加、M:N link edit 表 `(a_pk, b_pk, deleted, version)`、text 列 `COLLATE "C"`、索引规则、reload 加固（`lock_timeout`、`flush_metadata_cache` + 可见探测、`def_version` 栅栏）、多 role 合并公式、行谓词只引用同行属性与 `principal.attrs`、fail-closed、OIDC JWT + 自写 JWKS 缓存。

#### 角色 / scope 的管理通道
- **D-01:** roles-as-code：role 及其授权项（可见 type、属性 allow-list、行谓词、action、SQL 级别、L2 catalog / schema allow-list、toolset）写在独立目录的 YAML 中（新 kind，暂名 `Role` / `RoleBinding`，不与 ontology 定义或 `SourceBinding` 混放），经 CLI plan / apply 写入系统库，每次 apply 记管理审计。行谓词校验与 criteria 列覆盖 lint 在 apply 前离线执行，不通过则整体不生效。— **Reversibility:** costly — 新 kind 进入 loader、JSON Schema 与示例；改成命令式需重做变更通道与审计。
- **D-02:** role → user / group 的绑定写在 YAML 中，apply 为**全量同步**（plan 列出将删除的绑定）。user ↔ group 成员关系不落库，每次请求取自 JWT `groups` claim；dev token 由 CLI 指定 groups。不支持本地自建组。
- **D-03:** apply 是系统库中 role / 绑定的唯一写入口；Phase 7 的 REST 管理端对 role 只读（满足 SCOPE-07「只经管理通道或 L3 修改并审计」）。
- **D-04:** `principal_id` = `iss` + `sub`；首次认证自动建 user 记录，只存标识与最近登录时间，不存任何权限。
- **D-05:** `principal.attrs` 的 claim → 属性映射写在 `ontoforge-server.toml`，每个属性声明类型（String / Integer / list<String> 等），供行谓词静态类型检查；缺失按 null（再经 D-06 得不可见）。

#### 行谓词语义
- **D-06:** null 语义以 **SQL 三值逻辑**为准：Rust 单行求值器实现 Kleene 三值逻辑，最终 unknown → 不可见。因此 `object.x != 'a'` 在 x 为 null 时不可见；想让 null 可见须显式写 `object.x == null || …`。此语义与标准 CEL 不同，必须写入用户文档与谓词规范。— **Reversibility:** one-way — 一旦有用户按此语义写了 role，改语义会静默改变可见范围（安全回归）；差分测试语料与反例集也随之作废。
- **D-07:** 行谓词允许的 CEL 子集：比较运算（== != < <= > >=）、`&&` `||` `!`、`== null` / `!= null`、`in`（右侧为列表字面量或 list 型 `principal.attrs`）、字符串 `startsWith` / `endsWith` / `contains`（编译为带转义的 LIKE）。拒绝：宏（all / exists / map / filter）、`matches`、算术、`size`、其他函数。超出子集 → validate / apply 报 E。
- **D-08:** 行谓词引用了在绑定中声明为大小写不敏感 collation 的列 → E（列表路径必须把谓词下推到 Trino，D23 禁止此类列下推字符串比较）。
- **D-09:** 差分测试按 D23：完整 proptest 只跑 Rust vs SQLite；Trino 端跑固定种子语料 + 历史反例（Linux CI）。

#### edit log
- **D-10:** edit log 每条 edit 只存**新值 + 操作类型**：`(type, pk, op ∈ {set, unset, tombstone, release, link_add, link_remove}, 设置的属性 → 新值, 变更后 version)`。不存旧值、不复制 source 值。重放 edit log 必须能重建 edit store（作为 conformance 用例），这也是后续 scenario / branch overlay 的输入。— **Reversibility:** one-way — 格式 MVP 定死（PROJECT.md），已写入的日志无法补出旧值，scenario / branch 将依赖此格式。
- **D-11:** 一条 log 记录 = 一次成功提交（可含多条 edit）：全局单调 `seq` + ULID `event_id`、actor（principal_id）、action、时间、`def_version`、Action 参数（原样存储，脱敏留给 Markings 立项）。`ontoforge import` 以保留的 system 主体记入。— **Reversibility:** one-way — 同 D-10。
- **D-12:** edit log 与历史表永不删除：PG 与 SQLite 都以触发器拒绝 UPDATE / DELETE，conformance 测试覆盖。
- **D-13:** 变更历史查询（STORE-04）按**当前 scope 遮蔽**：调用者须对该对象当前可见（不可见与不存在返回同一错误）；每条历史只返回当前可见列的值，其余列只标「有变更」不给值；Action 原始参数不对普通调用者返回，只经管理通道。本 phase 交付库级 API，REST / MCP 外壳在 Phase 7。

#### reload / migrate
- **D-14:** 改名（属性或 object type）在 diff 上表现为「删 + 增」，按破坏性变更被 lint 拦下；`ontoforge migrate --plan` 接受显式 rename 声明（形如 `--rename <Type>.<old>=<new>`，type 同理）把 edit 数据搬到新列 / 新表；未声明则旧列归档、新列为空。定义文件中不加稳定 id。— **Reversibility:** costly — 若日后改为稳定 id 方案，需要改定义格式与 JSON Schema。
- **D-15:** reload 交付为 `ontoforge-storage` 的库函数（加载 → lint → 新增类 DDL → Trino 可见探测 → 切换 `def_version`）+ 一次性 CLI `ontoforge reload`（带 `--dry-run` 输出 DDL）。Phase 7 服务进程的触发方式复用同一函数。
- **D-16:** 旧列归档 = 同表改名为带 `def_version` 后缀的归档列（保留类型、不搬数据、读路径忽略）；清理归档列走 `migrate` 的显式子命令。
- **D-17:** 物理表名由 apiName 派生并加固定前缀（具体前缀与 link 表命名遵守 ADR-0002）。
- **D-18:** reload 时对已 apply 的 role 重跑 criteria 列覆盖 lint（SCOPE-06）与行谓词校验；任一失败则整体不切换，保留旧定义。

### Claude's Discretion
- 系统库表结构（定义版本、user、role、binding、管理审计）的具体列与索引。
- `Role` / `RoleBinding` 的字段命名与目录约定（遵守 ADR-0002 / ADR-0012），以及 apply 的 CLI 子命令名。
- dev token 的签名方式与本地密钥存放位置（只要 dev 模式外不可用）。
- JWKS 缓存刷新策略、`kid` 未命中时的重拉节流。
- 历史表 JSONB 快照的写入时机与结构（只用于审计 / 按版本取历史，不参与过滤）。
- edit store 在 SQLite 上与替身引擎 `catalog__schema` 库的关系（同库还是独立文件）。
- `postgresql_embedded` 在 Windows 本机的门控方式（feature / env）。

### Deferred Ideas (OUT OF SCOPE)
- Action 参数脱敏：随 Markings 子系统立项。
- 本地自建组（不依赖 IdP）：当前不支持，有真实需求再议。
- 定义中的稳定 id（零成本改名）：未采纳，如改名频繁再评估。
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| STORE-01 | edit store 在 PG 或 SQLite 上运行，两后端通过同一套 conformance | 单一 `EditStore` 实现 + 后端枚举，conformance 写成泛型函数在 SQLite（默认）与 PG（`#[ignore]` + `ONTOFORGE_PG_URL` / `embedded-pg` feature）两处调用；sea-query 1.0.2 + sea-query-sqlx 0.9.1 + sqlx 0.9.0 |
| STORE-02 | reload 自动 DDL；破坏性变更 lint 拦下，`migrate --plan/--apply`；reload 原子生效 | 物理 schema 模型 + diff（纯函数）、`SET LOCAL lock_timeout`、PG 事务内 DDL、Trino `CALL <catalog>.system.flush_metadata_cache()` + `describe_tables` 探测、`of_def_state` 栅栏行 |
| STORE-03 | 每次成功提交追加不可变 edit log | `of_edit_log` + `of_edit_log_entry`；seq 在写完 edit 行之后分配；PG / SQLite 触发器拒绝 UPDATE / DELETE（SQLite 须开 `recursive_triggers`，已实测） |
| STORE-04 | 变更历史查询 | 库级 API 接收调用者已解析的当前行 + 合并 scope，用同一 Rust 求值器判可见并遮蔽列 |
| STORE-05 | link 基数在 edit store 写入时强制 | oneToOne 的 FK 列上 partial unique index（PG 与 SQLite 均支持，SQLite 已实测）；oneToMany 的 FK 单值天然满足；source 侧基数不在本 phase |
| STORE-06 | `ontoforge import` | CSV / JSON 复用 fixture 解析与类型规则，经 `commit()` 以保留主体 `system:import` 写入，默认 create-only |
| STORE-07 | 按属性 patch 叠加：tombstone 不复活、release 显式、版本单调 | 纯函数状态机 `next_state` + 独立朴素内存参考模型 + proptest |
| SCOPE-01 | 用户 / 组 / 角色管理 | `Role` / `RoleBinding` kind + `ontoforge scope plan/apply`（全量同步、管理审计）；user 表只存标识 |
| SCOPE-02 | 角色授权项 | `Role` IR：types → 属性 allow-list + `rowFilter`；actions；sqlLevels；L2 allow-list；toolsets |
| SCOPE-03 | 多角色合并无列 × 行泄露 | `TypeScope { row = OR(R_i), column(x) = OR(R_i where x in C_i) }`，同一结构既求值又编译；「A 的列 × B 的行」专项用例 |
| SCOPE-04 | CEL 行谓词 SQL 编译 ≡ Rust 单行求值 | `RowPred` IR（`ontoforge-expr`）→ Kleene 求值器 + 降到 printer AST；proptest 差分 Rust vs SQLite，Trino 固定语料 |
| SCOPE-05 | fail-closed | 无授权 = 无 TypeScope；缺失 attr = null；非子集谓词在 apply / validate 报错 |
| SCOPE-06 | criteria 列覆盖 lint | 复用 `member_paths` 取 `objects.<param>.<prop>`，逐 role 检查 allow-list；reload 与 apply 两处触发（D-18） |
| SCOPE-07 | 变更权 | apply 为唯一写入口 + `of_admin_audit`（append-only）；REST 只读留给 Phase 7 |
| AUTH-01 | OIDC JWT + JWKS | jsonwebtoken 11.1.0（`aws_lc_rs`）+ 自写 JWKS 缓存（TTL、kid-miss 单飞与节流、stale-if-error） |
| AUTH-02 | dev token | HS256 本地密钥，仅 `--dev` 构造的独立验证路径；`ontoforge dev-token` 子命令 |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

来自 `./.claude/CLAUDE.md` 与全局 / memory 的、对本 phase 有约束力的指令：

- Rust 1.98，edition 2024，cargo workspace；axum + utoipa、sqlx、sea-query、cel、rmcp 已在 ADR-0003～0011 锁定。
- **Windows 11 上 `cargo build` / `cargo test` 必须通过，本机不依赖 Docker**；Windows 默认测试只依赖 SQLite；Postgres 测试本机用 `postgresql_embedded`，CI 用 service container。
- 命名：项目 / crate 前缀 `ontoforge`；标识符遵循 ADR-0002，**禁止沿用参考仓库标识符**（`kn_id` 等）；不复制参考仓库代码。
- 通用系统：示例、默认配置、命名不得绑定公司（role 示例用 aviation 虚构数据）。
- 加功能前先查现成 crate，自研须写明理由（memory：prefer-existing-crates）。
- 抉择先自调研自决，必须用户拍板才问（本文 Open Questions 按此给出推荐）。
- GSD 强制：文件改动须经 GSD 命令；核心业务逻辑 / 算法 / 高可靠性模块（本 phase 的状态机、谓词求值、DDL diff、JWT 全部属于）**强制 TDD**；karpathy 心法（小步、最小有效代码）；STATE.md 保持 digest。
- `workflow.security_enforcement: true`，`security_asvs_level: 1`，`security_block_on: high`（`.planning/config.json`）；`nyquist_validation: true`。
- Phase 1/2 SECURITY 要求：新依赖做 legitimacy 审核并随 `Cargo.lock`（CI 全部 `--locked`）；SQL 文本只经单一出口（Phase 2 的 printer 守卫测试模式）。

## Summary

Phase 3 在已有 `schema / expr / engine` 三个 crate 上落地两块能力：`ontoforge-storage`（edit store、DDL 演进、edit log、reload / migrate / import）与 `ontoforge-auth`（role / scope、行谓词、认证）。最大的技术风险不在外部依赖，而在四个「静默语义分叉」点，本研究都做了实测：(1) SQLite `INSERT OR REPLACE` 在 `recursive_triggers` 关闭（默认）时**不触发 DELETE 触发器，append-only 日志可被静默覆盖**，必须在每个连接上 `PRAGMA recursive_triggers=ON` 并配回归测试；(2) SQLite `LIKE` 对 ASCII 大小写不敏感，而 Rust / Trino 区分大小写，`case_sensitive_like` 已被官方标为 deprecated，SQLite 方言应改用 `GLOB`（已实测转义与空串行为）；(3) Kleene `IN`（含 NULL、空列表）在 SQLite 与 Trino 的语义需由 Rust 求值器逐字复刻，空列表必须编译成 `FALSE`（Trino 不接受空 `IN ()`，printer 现已返回 `EmptyInList`）；(4) `cel` 0.15 的 AST 形态已用探针验证（二元左结合、`has()` 是 `Select.test`、`exists` 是 Comprehension、负数字面量直接是 `Lit(Int(-3))`），子集校验器可直接按这些形态匹配。

架构上的关键裁剪：行谓词只在 `ontoforge-expr` 中用 `cel` 做**解析**，随后立刻降成自有 `RowPred` IR；Rust 求值与 SQL 编译都从这个 IR 出发，不使用 `cel` 运行时求值（其 `null == 'a'` 为 false 的语义与 D-06 相反）。降到 SQL 的一步放在 `ontoforge-auth`（依赖 `ontoforge-engine` 的 printer AST），因此 crate 依赖图需要修正：`storage → {schema, expr, auth, engine}`，删除占位的 `storage → query`（Phase 4 的 query 反过来依赖 storage），`auth → engine` 是新增边但不成环。edit 表的 `set_mask` 推荐物理编码为「每属性一个 `<col>__set` 布尔列」而不是单个位图（无属性数上限、无需持久化槽位注册表、reload / rename / 归档都是普通列操作），这是 Claude's Discretion 范围内的物理选择，列入 Open Questions 供确认。

**Primary recommendation:** 先做 Wave 0（workspace 依赖与 feature、crate 依赖图修正、IR 增 `indexed`、测试脚手架与守卫测试），再并行三条 lane：storage（命名 / DDL / 状态机 / 日志 / conformance）、expr+auth 谓词管线（`RowPred`、Kleene 求值、printer 的 `StringMatch` 节点、差分测试）、auth 认证（JWT / JWKS / dev token）；最后收口 reload / migrate / import / scope apply 四个 CLI 与 CI 的 PG conformance 步骤。

## Architectural Responsibility Map

本项目无浏览器 / CDN 层，tier 取「crate / 运行层」。

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Role / RoleBinding 文档解析与诊断 | `ontoforge-schema`（loader / IR / JSON Schema） | `ontoforge`（CLI） | 沿用 `kind` dispatch、source map、insta 快照；IR 是唯一契约 |
| 行谓词子集校验、`RowPred` IR、Kleene 求值 | `ontoforge-expr` | `ontoforge-auth` | 与 `analyze` / `member_paths` 同处；auth 独占该 crate 的修改（设计文档 Lane 冲突提示） |
| 行谓词降到 SQL（printer AST） | `ontoforge-auth` | `ontoforge-engine`（printer） | 只有 printer 能产出 `PrintedSql`；auth 产 AST，engine 打印 |
| 多 role 合并（`TypeScope`） | `ontoforge-auth` | — | 求值与编译共用同一结构，防分叉 |
| criteria 列覆盖 lint、D-08 collation 检查 | `ontoforge-auth` | `ontoforge-expr`（`member_paths`） | 需要 ontology + 选定 profile 的绑定 |
| OIDC JWT 校验、JWKS 缓存、dev token | `ontoforge-auth` | `ontoforge-engine::config`（配置类型） | `ServerConfig` 的 `deny_unknown_fields` 解析基础设施在 engine，新增 `[auth]` / `[store]` 段的数据类型放同处 |
| 物理命名、DDL 生成 / diff / 内省 | `ontoforge-storage` | — | 纯函数，两个后端执行器共用 |
| edit 状态机、commit 写路径、edit log、history | `ontoforge-storage` | — | 单事务内：edit 行 → 日志 → 历史 |
| reload / migrate | `ontoforge-storage`（库函数） | `ontoforge`（CLI）、`ontoforge-engine`（Trino 探测） | D-15 |
| role plan / apply 的持久化与审计 | `ontoforge-storage` | `ontoforge`（编排：load → auth 校验 → storage 写） | storage 不依赖 CLI；校验在 auth |
| Trino 元数据刷新与可见探测 | `ontoforge-engine`（`Engine` trait + printer 增 `CALL`） | `ontoforge-storage` | 既有 `describe_tables` 可直接用于探测 |

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| sqlx | 0.9.0（workspace 已有；**增加** features `postgres`、`migrate`、`macros`、`json`、`chrono`、`tls-rustls-aws-lc-rs`） | 双后端驱动与迁移 | 已锁定；`sqlite` feature 在 0.9.0 中即 `sqlite-bundled` [VERIFIED: crates.io API sqlx 0.9.0 features `"sqlite":["sqlite-bundled",...]`]；本机锁定的 libsqlite3-sys 0.37.0 自带 SQLite **3.51.3** [VERIFIED: `libsqlite3-sys-0.37.0/sqlite3/sqlite3.h:149` `#define SQLITE_VERSION        "3.51.3"`]，满足 RENAME COLUMN / DROP COLUMN 需求 |
| sea-query | 1.0.2（2026-08-12） | DML / DDL 构造（Table::create / alter、Index::create、Query::insert/select/update） | ADR-0003/0016 已锁定；features 取 `backend-postgres`、`backend-sqlite`、`with-json`、`with-chrono` [VERIFIED: crates.io API 1.0.2 features 列表含这些项] |
| sea-query-sqlx | 0.9.1（2026-05-30） | `SqlxBinder::build_sqlx` 把 sea-query 值转成 sqlx 参数 | 依赖 `sea-query ^1.0.0`、`sqlx ^0.9` [VERIFIED: crates.io API dependencies]；源码确认 trait 形态 `fn build_sqlx<T>(&self, query_builder: T) -> (String, SqlxValues) where T: QueryBuilder`，只覆盖 Select / Update / Insert / Delete / With（DDL 用 `build()` 得 `String`）[VERIFIED: sea-query-sqlx-0.9.1/src/sqlx.rs:4-10,20-24 已下载读取] |
| jsonwebtoken | 11.1.0（2026-09-16），`default-features = false, features = ["aws_lc_rs"]` | JWT 解码与校验 | ADR-0008 锁定；**必须选一个 crypto 后端 feature，否则首次 encode / decode 时 panic** [CITED: github.com/keats/jsonwebtoken `_autodocs/api-reference/index.md`，经 Context7 /keats/jsonwebtoken]；workspace 的 reqwest 0.13 `rustls` feature 走 aws-lc-rs [VERIFIED: crates.io API reqwest 0.13.5 `"rustls":["__rustls-aws-lc-rs",...]`；`Cargo.lock` 含 `aws-lc-rs`，`rustls` 依赖它]，因此选 `aws_lc_rs` 不引入第二个 crypto 栈 |
| cel | 0.15.0（workspace 已有） | **只做解析** | 见下文 AST 探针；不用其运行时求值 |
| proptest | 1.12.0（workspace 已有） | 状态机与差分 proptest | 已锁定 |
| ulid | 3.0.0（workspace 已有） | `event_id` | 已锁定 |
| chrono | 0.4.45（workspace 已有，**需增 `clock` feature** 供 `Utc::now()`） | 提交时间 | 时间由 Rust 侧生成，保证两后端一致 |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| postgresql_embedded | 0.21.0（2026-07-19），**可选**依赖，`default-features = false`，features `["tokio", "theseus", "tls-rustls-aws-lc-rs"]` | Windows 本机 PG conformance | 仅在 `ontoforge-storage` 的 `embedded-pg` feature 下编译；默认 feature 含 `tls-native-tls`，会经 sqlx 的 feature 统一把 native-tls / openssl 引入图中 [VERIFIED: crates.io API postgresql_embedded 0.21.0 `"default":["tls-native-tls","theseus"]`，依赖 `sqlx ^0.9.0`；`Cargo.lock` 当前 `openssl` 条目数为 0]。运行时从 GitHub `theseus-rs/postgresql-binaries` 下载 PG 二进制，Windows msvc 包存在且带 `provenance.json` [VERIFIED: GitHub releases API，tag 18.6.1 与 17.11.1 均存在] |
| getrandom | 0.4.3 | 生成 dev token 的 32 字节密钥 | `ontoforge-auth`；见 Package Legitimacy Audit（名称来源标 `[ASSUMED]`） |
| reqwest | 0.13.5（workspace 已有，`rustls`） | 拉 JWKS | 已有；JWKS 响应体用 `bytes()` + `serde_json` 自行解析（workspace 未开 `json` feature） |
| wiremock | 0.6.5（workspace dev） | JWKS 端点桩 | 已有，Windows 可跑 |
| insta / tempfile / walkdir / regex | workspace 已有 | 快照、临时 SQLite 文件、SQL 出口守卫测试 | 沿用 Phase 2 模式 |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| sea-query 1.0.2 + sea-query-sqlx | 手写 SQL 字符串 + `AssertSqlSafe` | 已被 ADR 锁定；手写需自己保证两方言引号与占位符；sea-query 对 SQLite 的 ALTER 每条语句只能一个操作 |
| 每属性 `<col>__set` 布尔列 | 单个 `set_mask bigint` 位图（spike 3b 所用） | 位图要持久化「属性 → 位槽」注册表且槽永不复用，≤ 63 个属性；布尔列无上限、rename / 归档只是列操作、SQL 谓词 `col__set = false OR col = v` 更直白 |
| HS256 dev token | EdDSA（ed25519-dalek 生成密钥） | 需额外依赖与 PKCS#8 处理；dev 令牌只在本机自签自验，对称密钥足够 |
| `jwks` crate 0.5.3 | 自写 JWKS 缓存 | ADR-0008 锁定自写；该 crate 近期下载仅约 2 万、且要自己管 kid-miss 节流与 stale-if-error |
| `postgresql_embedded` | `testcontainers` / 本机 Docker | 项目约束禁止本机依赖 Docker |
| SQLite `LIKE` + `case_sensitive_like` | `GLOB` | 官方标注该 pragma deprecated，且改变 `LIKE` 语义可能令使用 LIKE 的索引 / CHECK 看起来损坏 [CITED: sqlite.org/pragma.html]；`GLOB` 无连接状态依赖 |

**Installation（Wave 0 修改 workspace `Cargo.toml`）：**

```toml
# [workspace.dependencies] 变更示意
sqlx = { version = "0.9.0", default-features = false, features = ["sqlite", "runtime-tokio", "postgres", "migrate", "macros", "json", "chrono", "tls-rustls-aws-lc-rs"] }
sea-query = { version = "1.0.2", default-features = false, features = ["backend-postgres", "backend-sqlite", "with-json", "with-chrono"] }
sea-query-sqlx = { version = "0.9.1", default-features = false, features = ["sqlx-postgres", "sqlx-sqlite", "runtime-tokio", "with-json", "with-chrono"] }
jsonwebtoken = { version = "11.1.0", default-features = false, features = ["aws_lc_rs"] }
getrandom = "0.4.3"
chrono = { version = "0.4.45", default-features = false, features = ["std", "clock"] }
postgresql_embedded = { version = "0.21.0", default-features = false, features = ["tokio", "theseus", "tls-rustls-aws-lc-rs"] }
```

具体 feature 清单在第一个 plan 里用 `cargo tree -e features -i openssl`、`cargo tree -d` 复核：`openssl` 必须不在图中（把它写成 Wave 0 的守卫检查）。[ASSUMED：feature 精确名称组合，需 `cargo check` 确认]

**Version verification:** 以上版本均于 2026-10-19 用 crates.io API 查得（sea-query 1.0.2、sea-query-sqlx 0.9.1、jsonwebtoken 11.1.0、postgresql_embedded 0.21.0、getrandom 0.4.3、sqlx 0.9.0、cel 0.15.0）。

## Package Legitimacy Audit

`gsd-tools query package-legitimacy check --ecosystem crates ...` 于 2026-10-19 运行的原始输出（`publishedAt` 为该 crate 首次发布日期）。

| Package | Registry | Age（首次发布） | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| sea-query | crates.io | 2020-12（约 6 年） | 453,930 / 周 | github.com/SeaQL/sea-query | OK | Approved（名称来自 ADR-0003 + Context7）[VERIFIED: crates.io + gsd-tools] |
| sea-query-sqlx | crates.io | 2025-08（约 14 个月） | 116,990 / 周 | github.com/SeaQL/sea-query | OK | Approved with note：名称来自本项目 `rust-stack.md` 与 crates.io，Context7 文档仍写旧名 `sea-query-binder`（0.7.0，2025-08-01 末版）。已下载 0.9.1 源码核对 API。名称来源标 `[ASSUMED]`，planner 在引入处加 `checkpoint:human-verify`（核对 docs.rs 页面与 `cargo tree`） |
| jsonwebtoken | crates.io | 2015-11（约 11 年） | 4,652,330 / 周 | github.com/Keats/jsonwebtoken | OK | Approved（ADR-0008 + Context7）[VERIFIED] |
| postgresql_embedded | crates.io | 2024-02（约 2.7 年） | 142,471 / 周 | github.com/theseus-rs/postgresql-embedded | OK | Approved（PROJECT.md 约束点名 + Context7 未收录，官方 README 确认 Windows）[VERIFIED: gsd-tools + README]；仅 test-only feature；传递依赖 `postgresql_archive` 同为 OK（146,364 / 周） |
| getrandom | crates.io | 2019-01（约 7.7 年） | 52,206,198 / 周 | github.com/rust-random/getrandom | OK | Approved with note：名称来自训练记忆，标 `[ASSUMED]`，planner 加 `checkpoint:human-verify` |
| bigdecimal | crates.io | 2017-04 | 2,405,257 / 周 | github.com/akubera/bigdecimal-rs | OK | **本 phase 不引入**：decimal 值保持规范化字符串（Phase 2 RESEARCH 已定），PG 侧绑定用 `CAST($n AS numeric(p,s))`，行谓词类型子集不含 decimal |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none（上表全部 OK）。`npm view ... scripts.postinstall` 不适用（crates 生态）；`postgresql_embedded` 与 `aws-lc-sys` 类 crate 带 build script / 运行时下载，见 Security Domain 的供应链条目。

## Architecture Patterns

### System Architecture Diagram

```
 ontoforge CLI ── reload / migrate / import / scope plan|apply / dev-token
        │                    │                     │
        ▼                    ▼                     ▼
 [schema loader] ──► Ontology IR ──┐        Role/RoleBinding IR ──┐ (scope loader,
   (ontology dir)                  │          (scope dir)         │  复用 parse/dispatch)
                                   ▼                              ▼
                      ┌──── ontoforge-storage ────┐     ┌──── ontoforge-auth ─────────────┐
 reload(def) ───────► │ naming ─► PhysicalSchema   │     │ validate_scope(ontology,profile,│
                      │ diff(old,new) ─► Plan      │     │   server_cfg, roles)            │
                      │   additive │ destructive   │ ◄───│   ├ RowPred (expr crate)        │
                      │ exec: PG txn / SQLite txn  │     │   ├ criteria coverage (SCOPE-06)│
                      │  SET LOCAL lock_timeout    │     │   └ collation check (D-08)      │
                      │  DDL ► of_def_version row  │     │ TypeScope merge (OR rows,       │
                      │ commit ► [Trino flush+probe]     │   per-column OR grants)         │
                      │  ► fence: of_def_state     │     │   ├ eval(row) Kleene ──► bool   │
                      └─────────┬──────────────────┘     │   └ lower() ► printer AST ──┐   │
                                │                        │ Authenticator               │   │
 commit(edits) ───────► apply edits (pure next_state)    │   OIDC JWT + JWKS cache     │   │
   one txn:             ► of_obj_* / of_lnk_* rows       │   dev token (HS256, --dev)  │   │
   (1) fence check      ► of_edit_log + entries          └─────────────────────────────┼───┘
   (2) lock+upsert rows ► of_edit_history snapshot                                     ▼
   (3) log LAST (seq)   append-only triggers        ontoforge-engine: printer (Trino|SQLite)
                                                    + Engine::describe_tables (probe)
 history(obj, scope, current_row) ► mask via TypeScope.eval ► entries
```

### Recommended Project Structure

```
crates/
├── ontoforge-schema/src/
│   ├── ir/scope.rs            # Role, RoleBinding, ScopeDefinition (+ JSON Schema)
│   ├── ir/object.rs           # PropertyDef.indexed (ADR-0016), 其余不变
│   └── loader/scope.rs        # load_scope_dir(): 复用 parse/sourcemap/dispatch, kind 白名单 {Role, RoleBinding}
├── ontoforge-expr/src/
│   └── rowpred/{mod,ir,parse,typecheck,eval}.rs   # CEL 解析 → RowPred IR；Kleene 求值
├── ontoforge-engine/src/
│   ├── printer/ast.rs         # 新增 Expr::StringMatch{expr, needle, kind}（见 Pattern 6）
│   ├── printer/call.rs        # CALL <catalog>.system.flush_metadata_cache()
│   └── config.rs              # 增 [auth] / [store] 段的数据类型
├── ontoforge-auth/src/
│   ├── scope/{merge,lower,lint}.rs
│   ├── authn/{oidc,jwks,dev,principal}.rs
│   └── lib.rs
├── ontoforge-storage/
│   ├── migrations/{postgres,sqlite}/*.sql   # 仅系统表与触发器
│   └── src/{naming,physical,diff,ddl,state,commit,log,history,reload,migrate,import,scope_store,sql_exit}.rs
│       └── tests/{conformance.rs(泛型), conformance_sqlite.rs, conformance_pg.rs(#[ignore]), model_prop.rs, replay.rs, append_only.rs, sql_exit_guard.rs}
└── ontoforge/src/             # 新增 reload.rs migrate.rs import.rs scope.rs dev_token.rs
```

依赖方向（修正设计文档 ASCII 图，并在代码注释里同步）：

```
schema ◄─ expr ◄─ engine(仅依赖 schema)
                   ▲
          auth ────┘（auth → schema, expr, engine）
          ▲
 storage ─┘（storage → schema, expr, auth, engine）   ← 删除现有 storage → query
 query（Phase 4）→ storage, auth, engine；action → storage, auth, expr
```

当前 `crates/ontoforge-storage/Cargo.toml` 的 `[dependencies]` 为 `ontoforge-schema` 与 `ontoforge-query`；`ontoforge-auth` 只依赖 `ontoforge-schema` [VERIFIED: 两个 Cargo.toml 全文已读]。Phase 4 的叠加读需要读 edit store，所以 query 依赖 storage 而不是反过来；`auth → engine` 无环（engine 只依赖 schema [VERIFIED: ontoforge-engine/Cargo.toml]）。

### Pattern 1: 物理命名与 schema 模型（纯函数）

**What:** `PhysicalSchema::from(&Ontology)` 产出每个 object type 一张 `of_obj_<snake>` 表、每个 M:N link 一张 `of_lnk_<snake>` 表（命名遵守 ADR-0002「系统表前缀 `of_`、snake_case、不使用前导下划线列名」[VERIFIED: .planning/adr/0002-naming-convention.md 第 5 条]）。diff、DDL 生成、内省比对全部基于这个模型。

**规则（prescriptive）：**
- 属性列 = snake_case(apiName)；set 标志列 = `<col>__set`（snake 化结果不含连续下划线，故不会与属性列冲突）；系统列一律 `of_` 前缀：`of_version`（bigint，未编辑对象内部版本 0，首次 edit 为 1）、`of_deleted`、`of_released`、`of_def_version`、`of_log_seq`。不用 ADR/spike 里的裸名 `deleted` / `version`，因为 `version`、`deleted` 是极可能出现的用户属性名；link 表保留 `a_pk` / `b_pk`（a = `from` 端，b = `to` 端）。
- 归档列 = `<col>__arch_v<N>`（N = 触发归档的 `def_version`）；同时归档其 `__set` 列。
- apiName 最长 100 字符且仅 `[A-Za-z0-9]` [VERIFIED: ontoforge-schema/src/ir/names.rs:55 `const MAX_LEN: usize = 100;` 与 `all(|c| c.is_ascii_alphanumeric())`]，PG 标识符上限 63 字节：物理名超 63 字节时截断并追加 8 位十六进制 hash（对完整未截断名取 hash），规则写进 `naming.rs` 并测试。
- snake 化存在碰撞（如 `ABTest` 与 `AbTest`）：`PhysicalSchema` 构造时检测任意两个逻辑名映射到同一物理名，报错（建议新诊断 ONT-E19 或 reload 期错误，见下「诊断码」）。
- 列类型映射（PG / SQLite）：byte,short→`smallint`/`INTEGER`；integer→`integer`/`INTEGER`；long→`bigint`/`INTEGER`；float→`real`/`REAL`；double→`double precision`/`REAL`；boolean→`boolean`/`INTEGER`；string→`text COLLATE "C"`/`TEXT`；decimal(p,s)→`numeric(p,s)`/**`TEXT` + `ontoforge_decimal` 排序规则**（不要用 sea-query 的 `decimal_len`：SQLite 会给 `decimal(p,s)` NUMERIC 亲和性，文本会被转成浮点）；date→`date`/`TEXT`；timestamp→`timestamptz`/`TEXT` + `ontoforge_timestamp` 规则；array→PG 原生数组 / SQLite JSON 文本；geopoint→`jsonb`/JSON 文本。SQLite 的两个自定义 collation 目前是 `pub(crate)`（`DECIMAL_COLLATION` / `TIMESTAMP_COLLATION` 在 `printer/sqlite_ddl.rs`，比较函数 `compare_decimal` / `compare_timestamp` 在 `sqlite/mod.rs:209,245`，注册在 `sqlite/mod.rs:48-49`），Wave 0 需导出并让 storage 的 SQLite 连接选项复用同一实现 [VERIFIED: 文件已读]。
- pk 列类型 = pk 属性类型（validate 允许 String / Integer / Short 为 Valid，Date / Timestamp / Boolean / Byte / Long 为 Discouraged 但可通过 [VERIFIED: ontoforge-schema/src/ir/basetype.rs `pk_suitability`]），pk 列为表主键；日志里 pk 存规范 JSON 文本。
- **Postgres 两个 schema**（推荐，可配置）：`ontoforge_edit` 放 `of_obj_*` / `of_lnk_*`（Trino 连接账号只授 `USAGE` + `SELECT`），`ontoforge_sys` 放定义、日志、历史、user、role、审计（Trino 账号无权限）。SQLite 单命名空间，表名相同。测试用唯一 schema 名隔离。（见 Open Question 5）

### Pattern 2: 按属性 patch 状态机（纯函数 + 独立参考模型）

**What:** 生产实现是一个纯函数 `next_state(prev: Option<EditRow>, op: Op, def_version) -> Result<EditRow, EditError>`；`EditStore::commit` 在事务内读当前行、调用它、写回。测试侧另写一个**朴素**参考模型（`BTreeMap<(type,pk), ModelState>` 加逐条规则），proptest 生成 `Vec<Op>`，同时喂给模型与真实 store（SQLite 内存库，PG 另跑），逐步比较行内容与版本。SC3 的「内存参考模型 proptest」由此满足。

**状态与转移（建议规格，proptest 的不变量来源）：**

| 当前 \ 操作 | set(props) | unset(props) | tombstone | release |
|---|---|---|---|---|
| 不存在 | 创建：置位对应 `__set`，version = 1 | 错误 `NotFound` | 允许（压制 source 行），version = 1 | 错误 |
| live | 覆盖已置位属性，version + 1 | 清 `__set` 并置 NULL，version + 1 | deleted = true，version + 1 | 清所有 `__set`、`released = true`，version + 1（仅 source-backed） |
| tombstoned | **错误 `Tombstoned`（不复活）** | 错误 | 幂等错误或 no-op（建议错误，调用方不应重复删） | `deleted = false, released = true`，version + 1（回到纯 source） |
| released | 重新进入 live：`released = false`，置位属性，version + 1 | 错误 | deleted = true，version + 1 | no-op 错误 |

不变量：version 每次成功写严格 +1、永不重置、永不减小；tombstoned 只能经 release 离开；release 后所有 `__set` 为 false；editOnly 类型的 release 报错（无 source 可回）；一个 commit 内同 pk 多条 edit 逐条各 +1，日志里每条 entry 记 `version_after`。

link 表 `(a_pk, b_pk, of_deleted, of_version)`：`link_add` → `of_deleted = false`、version + 1；`link_remove` → `of_deleted = true`、version + 1（保留行以压制 source join 行）；两者都允许反复切换。

### Pattern 3: commit 写路径与 edit log

**What:** `EditStore::commit(Commit) -> Receipt` 一个事务内完成：

1. **栅栏**：PG `SELECT active_def_version FROM of_def_state FOR SHARE`（SQLite 同事务内普通读），与 `Commit.def_version` 不等 → `DefinitionStale`，整体回滚。reload 的切换 `UPDATE of_def_state` 需要排他行锁，会等待在途写事务结束，之后的新写事务看到新版本，实现「栅栏后旧定义写入被拒绝」（ADR-0016/D20）。
2. 对 edits 按 `(type, pk)` 稳定排序；每条先 `INSERT ... ON CONFLICT (pk) DO NOTHING` 一个版本 0 的骨架行，再 `SELECT ... FOR UPDATE`（SQLite 无此语法，靠写事务独占），调用 `next_state`，`UPDATE` 回写，同时在 `of_edit_history` 追加快照行。
3. **最后**插入 `of_edit_log` 头行取得 `seq`，再插入 `of_edit_log_entry`。顺序理由：此时所有行锁已持有，同一 pk 的两个事务的 seq 顺序与锁顺序（即提交顺序）一致；若先取 seq，持有较小 seq 的事务可能晚于较大 seq 提交，重放顺序与实际状态序不符。[ASSUMED：PG identity 序列在事务内取值、提交顺序不保证与 seq 顺序一致，属 PG 通用行为]
4. SQLite 用 `pool.begin_with("BEGIN IMMEDIATE")`（sqlx 0.9 的 `begin_with(statement: impl SqlSafeStr)`，字符串字面量满足 `SqlSafeStr`）[VERIFIED: sqlx-core-0.9.0/src/pool/mod.rs:391-396 与 sqlx-sqlite-0.9.0/src/connection/mod.rs:253-262]，避免默认 DEFERRED 事务在写升级时 `SQLITE_BUSY`。

日志表（系统库）：`of_edit_log(seq pk 自增, event_id text unique, actor, action, committed_at, def_version, params json)`；`of_edit_log_entry(log_seq, ordinal, kind('object'|'link'), type_name, pk json, op, payload json, version_after, primary key(log_seq, ordinal))`。`payload` 只含「设置的属性 → 新值」（D-10，不存旧值、不复制 source 值）。重放 conformance：对同一定义，按 seq 顺序把 entries 回放到一个全新 store，比较两个 store 的全部 edit 表（排除 `of_log_seq` 以外的元数据即可保持相等，若重放时保留原 seq 则可完全相等）。

### Pattern 4: append-only 强制（触发器 + 权限）

- **PG**（系统库迁移文件，`$$` 函数体）：一个 `of_reject_mutation()` plpgsql 函数 `RAISE EXCEPTION 'table % is append-only', TG_TABLE_NAME USING ERRCODE = 'integrity_constraint_violation'`；在 `of_edit_log`、`of_edit_log_entry`、`of_edit_history`、`of_admin_audit` 上建 `BEFORE UPDATE OR DELETE ... FOR EACH ROW` 触发器，**再单独建 `BEFORE TRUNCATE ... FOR EACH STATEMENT`**（行触发器不拦 TRUNCATE）。触发器对表所有者不构成安全边界（`ALTER TABLE ... DISABLE TRIGGER`），部署文档须要求应用账号只授 `INSERT, SELECT`（`REVOKE UPDATE, DELETE, TRUNCATE`）。错误码：用户可见断言用 SQLSTATE 类 23（integrity_constraint_violation `23000`）；`P0001` 为通用 `RAISE EXCEPTION` 默认码 [CITED: postgresql.org/docs/17 errcodes-appendix]。
- **SQLite**：`BEFORE UPDATE` / `BEFORE DELETE` 触发器 `SELECT RAISE(ABORT, 'append-only')`。**实测陷阱**（SQLite 3.51.0，Python 3.13 自带，与 bundled 3.51.3 同一大版本）：`recursive_triggers` 默认 OFF 时，`INSERT OR REPLACE` 与 `REPLACE INTO` **直接覆盖行且不触发 DELETE 触发器**（结果 `[('b',)]` / `[('z',)]`）；`ON CONFLICT DO UPDATE`、`UPDATE`、`DELETE` 被拒。把 `PRAGMA recursive_triggers=ON` 后 5 种写法全部被拒 [VERIFIED: 本机实测 t1.py，输出见 Common Pitfalls]。因此：storage 的 `SqliteConnectOptions` 对每个连接 `.pragma("recursive_triggers", "ON")`（`pragma` 方法存在 [VERIFIED: sqlx-sqlite-0.9.0/src/options/mod.rs:358]），并增加回归测试「`INSERT OR REPLACE` 覆盖 `of_edit_log` 被拒」；代码里禁止出现 `OR REPLACE` / `REPLACE INTO`（sql_exit 守卫测试扫描）。
- 与 sqlx 迁移的关系：触发器放在系统表的迁移文件里（`migrations/postgres/*.sql`、`migrations/sqlite/*.sql`，两套目录，因为函数 / 语法不同），由 `sqlx::migrate!("./migrations/<backend>")` 嵌入二进制。**per-type edit 表不走 sqlx 迁移**，由 DDL 引擎生成并记录于 `of_def_version`。迁移文件含 `$$` 或 `BEGIN ... END;` 多语句，sqlx 对 PG 走简单查询协议、对 SQLite 逐语句执行，二者都能吃多语句脚本 [ASSUMED：需第一个 plan 里用真实迁移验证一次]。

### Pattern 5: reload / migrate 流程

`reload(ctx, ontology, profile, opts) -> ReloadOutcome`（`ontoforge-storage` 库函数，D-15）：

1. 加载 + lint（E 级阻断）。
2. 载入当前生效定义快照（`of_def_version.ontology_json`，规范化 JSON）→ `diff(old, new)` 分为 **additive**（新 type、新 M:N link、可空属性、索引）与 **destructive**（删属性 / 删 type、改类型、改 pk、新增 required 属性——IR 当前**没有** `default` 字段，故任何新增 required 属性一律视为破坏性，见 Open Question 2、改名在 diff 上就是删 + 增）。destructive 非空 → 报新诊断码并指向 `migrate`，**不执行任何 DDL**。
3. 对已 apply 的 role 重跑 criteria 覆盖 lint 与行谓词校验（D-18，roles 以 IR JSON 存于 `of_role`，无需 YAML）；任一失败 → 整体中止。
4. 取单飞锁：PG `pg_advisory_xact_lock(<固定 bigint>)`；SQLite 靠 `BEGIN IMMEDIATE`。
5. 一个事务内：PG `SET LOCAL lock_timeout = '<n>ms'`（数值格式化进语句，`SET` 不接受绑定参数，需 `AssertSqlSafe`，且 n 来自配置的整数）→ 幂等 DDL（`CREATE TABLE IF NOT EXISTS`；PG `ADD COLUMN IF NOT EXISTS`；SQLite 无 `ADD COLUMN IF NOT EXISTS`，先 `PRAGMA table_info` 内省再加）→ 索引对账（`CREATE INDEX IF NOT EXISTS`，名称确定性 `of_ix_<table>_<col>`）→ 插入 `of_def_version` 新行（状态 `pending`）。超时错误码 PG `55P03 lock_not_available` [CITED: postgresql.org/docs/17 errcodes-appendix]，映射成「保留旧定义」的错误。提交。
6. Trino 可见探测（仅当 profile 的引擎是 Trino 且配置了 edit store 的 Trino catalog）：先 `CALL <catalog>.system.flush_metadata_cache()`，再用现有 `Engine::describe_tables` 轮询新表 / 新列直到可见或超时；超时 → reload 失败，**不切换**（DDL 已提交但为加法、对旧定义无害，下次重试因幂等而安全）。
7. 切换：`UPDATE of_def_state SET active_def_version = N`（排他行锁，等在途写事务）并把 `of_def_version` 行置 `active`。

`migrate --plan` 输出确定的步骤清单（含 `--rename Type.old=new` 声明）与 plan hash；`--apply` 重新计算 plan 并要求 hash 一致（防 TOCTOU），在同一栅栏协议里执行：归档 = `ALTER TABLE ... RENAME COLUMN`（PG 与 SQLite ≥ 3.25 均支持，本机 3.51 实测可用，且重命名会同步更新索引 [VERIFIED: 实测 + sqlite.org/lang_altertable.html 原文 "The column name is changed both within the table definition itself and also within all indexes, triggers, and views that reference the column"]）；rename 声明 = 同表 `RENAME COLUMN` 或 `RENAME TO`（表）；类型变更 = 归档旧列 + 新建同名新列；删 type = 表改名为 `of_obj_<x>__arch_v<N>`。清理归档列的显式子命令 `migrate --drop-archived`（SQLite `DROP COLUMN` 不能删有索引 / 在触发器或视图中被引用的列 [CITED: sqlite.org/lang_altertable.html]，先删索引）。

### Pattern 6: 行谓词管线（一份 IR，两处消费）

**CEL AST 探针**（本 session 在独立 scratch crate 中以 `cel = "=0.15.0"` 运行，`Program::compile` 后遍历 `expression()`）[VERIFIED: 探针输出]：

| 源码 | AST 形态 |
|---|---|
| `a && b` / `a \|\| b` / `!x` | `Call("_&&_")` / `Call("_\|\|_")` / `Call("!_")`，**二元且左结合**（三个 `\|\|` 嵌成两层） |
| `object.x` | `Select(Ident(object), field "x", test=false)` |
| `principal.attrs.regions` | `Select(Select(Ident(principal),"attrs"),"regions")` |
| `x == 'a'`、`!=`、`>=`、`<` | `Call("_==_" / "_!=_" / "_>=_" / "_<_")`，字面量 `Lit(String(..))`、`Lit(Int(..))`、`Lit(Double(..))`、`Lit(Boolean(..))` |
| `x == null` / `null == x` | `Call("_==_", [.., Lit(Null)])`，null 可在任一侧，需规范化 |
| `x in [1, 2, -3]` / `x in principal.attrs.regions` | `Call("@in", [x, List[...]])`；`-3` 直接是 `Lit(Int(-3))` |
| `x.startsWith('ab')` 等 | `Call("startsWith", target=Some(x), args=[Lit(String)])`，`endsWith`、`contains` 同形 |
| `has(object.x)` | `Select(.., test=true)`，应拒绝 |
| `list.exists(t, ..)` | `Comprehension`，应拒绝（宏） |
| `c ? a : b`、`1 + 2`、`b'abc'`、`1u` | `Call("_?_:_")`、`Call("_+_")`、`Lit(Bytes)`、`Lit(UInt)`，均拒绝 |

与现有代码一致：`infer.rs` 用 `op::LOGICAL_AND | op::LOGICAL_OR | op::LOGICAL_NOT | op::IN | op::NOT_STRICTLY_FALSE`（277-281 行）和 `op::EQUALS | op::NOT_EQUALS | op::LESS | op::LESS_EQUALS | op::GREATER | op::GREATER_EQUALS`（282-287 行）匹配，`"startsWith" | "endsWith" | "contains"` 在 320 行 [VERIFIED: infer.rs 已读]。注意 `NOT_STRICTLY_FALSE` 是 comprehension 展开用，行谓词必须拒绝。

**RowPred IR（`ontoforge-expr::rowpred`）：**

```rust
// 设计示意，值均来自上表探针与 D-07
pub enum Pred { Const(bool), Cmp{ l: Operand, op: CmpOp, r: Operand }, IsNull{ x: Operand, negated: bool },
                InList{ x: Operand, list: Vec<Scalar> }, StrMatch{ col: ColumnRef, needle: Operand, kind: MatchKind },
                And(Box<Pred>, Box<Pred>), Or(Box<Pred>, Box<Pred>), Not(Box<Pred>) }
pub enum Operand { Column(PropertyName), Lit(Scalar), Attr(String) /* 求值/编译时代入 */ }
```

- 类型检查（比 `infer.rs` 更严格，因为 `compatible` 允许 int 与 double 混比 [VERIFIED: infer.rs:53-58]）：比较两侧必须同族（bool / 整数族 / double 族 / string），**不允许 int 与 double 混比**；`<`,`<=`,`>`,`>=` 仅 string 与数值；属性类型允许 boolean、byte..long、float、double、string；`== null` / `!= null` 对除 geopoint 外任意属性类型可用；date / timestamp / decimal / array / geopoint 属性出现在比较或 `in` 中 → E（CEL 无这些类型的字面量，且 Rust 与 SQL 的比较语义需另立规则，后续按需放开）。list 字面量元素同类型且**不得含 null**（避免 `x IN (…, NULL)` 的三值陷阱；Rust 求值器仍按 SQL 语义实现完整 `IN`，以便未来放开）。`principal.attrs.<name>` 的类型来自 D-05 配置，缺失属性在静态检查阶段报 E，运行时缺失 = `Scalar::Null`。
- 同行限制（ADR-0018 / D12）：自由变量根只允许 `object` 与 `principal.attrs`；`object.<name>` 的 name 必须是该 type 的现存属性；`principal.id` / `principal.groups` 本 phase 不开放（设计文档只列 `principal.attrs`）。
- **Kleene 求值器**（`eval(&Pred, &Row, &Attrs) -> Tri`，`Tri = {True, False, Unknown}`）：`Cmp` 任一侧 Null → Unknown；`IsNull` 恒二值；`And`/`Or`/`Not` 用 Kleene 真值表；`InList`：`x` 为 Null → 列表空则 False 否则 Unknown；有相等元素 → True；列表含 Null 且无匹配 → Unknown；否则 False；`StrMatch`：col 为 Null → Unknown，needle 为 Null → Unknown，否则按字节串 `starts_with` / `ends_with` / `contains`（空 needle 对任意非 Null 字符串为 True）。最终 `Unknown` → 不可见。已用 SQLite 3.51 实测 `NULL IN (1,2)`→NULL、`1 IN (2,NULL)`→NULL、`1 IN (1,NULL)`→1、`NULL IN ()`→0、`NOT NULL`→NULL、`NULL OR 1`→1、`NULL AND 0`→0、`NULL AND 1`→NULL、`NULL OR 0`→NULL、`NULL = NULL`→NULL、`NULL IS NULL`→1 [VERIFIED: 本机实测 t3.py]。Trino 的 `IN` / `NOT` / `AND` / `OR` 遵循 SQL 标准三值逻辑 [ASSUMED：Trino 文档未描述 IN 与 NULL，属标准 SQL 行为，由固定语料在 Linux CI 验证]。
- **降到 SQL**（`ontoforge-auth::scope::lower`，输出 `ontoforge_engine::printer::ast::Expr`）：`Cmp` → `Compare`；`== null` → `IsNull{negated:false}`；`Attr` 在编译时代入 `Literal`（null attr → `Literal::Null`，打印为 `NULL` [VERIFIED: dialect.rs:209 `Literal::Null => Ok("NULL".to_owned())`]）；`InList` 空列表 → 常量 `FALSE`（printer 对空 `InList` 返回 `PrintError::EmptyInList` [VERIFIED: dialect.rs:129]，而 Trino 不接受空 `IN ()`，SQLite 虽接受但两端须一致）；`And` / `Or` 的二元链由 printer 的 `And(Vec)` / `Or(Vec)` 打平（printer 现有 `bool_list` 对每个子表达式加括号 [VERIFIED: dialect.rs `bool_list`]）。
- **字符串匹配新增 AST 节点**（ast.rs 顶部注释已预留：「Phase 4 note: a `LIKE` node must route its pattern wildcard escaping through the same `escape` exit as every other literal.」[VERIFIED: printer/ast.rs:6-7]）：`Expr::StringMatch { expr, needle: Literal::String, kind: StartsWith|EndsWith|Contains }`。**Trino 方言**：`expr LIKE '<escaped>' ESCAPE '\'`，转义规则 = 对 `\`、`%`、`_` 前缀 `\`，再按 `startsWith` → 尾加 `%`、`endsWith` → 头加 `%`、`contains` → 两端加 `%`；Trino 的 `LIKE` 区分大小写，转义字符本身需转义 [CITED: trino.io/docs/current/functions/comparison.html]，反斜杠在 Trino / SQLite 字面量里都不特殊（`escape.rs` 头注释）[VERIFIED: printer/escape.rs:1-5]。**SQLite 方言**：`expr GLOB '<escaped>'`，转义 `[`→`[[]`、`*`→`[*]`、`?`→`[?]`，再加 `*`；已实测 `'a' like 'A'`=1 而 `'a' glob 'A'`=0、`'a*c' glob 'a[*]c'`=1、`'abc' glob 'a[*]c'`=0、`'abc' glob '**'`=1、`'' glob '**'`=1、`NULL glob '**'`=NULL、`'É' like 'é'`=0（非 ASCII 本就区分）[VERIFIED: 本机实测 t2.py]。needle 含 NUL → 沿用 `PrintError::NulInString`。

### Pattern 7: scope 合并（SCOPE-03）

`TypeScope { row: Pred, columns: BTreeMap<PropertyName, Pred> }` 由某主体对某 type 的 role 集合 `{(R_i, C_i)}` 构造：`row = OR(R_i)`（role 未写 `rowFilter` 则 `Const(true)`）；`columns[x] = OR(R_i where x ∈ C_i)`；没有任何 role 授予该 type → 不构造（= 不可见，fail-closed）。Rust 侧：`visible = eval(row).is_true()`，列 x 可见 = `visible && eval(columns[x]).is_true()`。SQL 侧（Phase 4 编译用，本 phase 提供 lowering 并差分测试）：投影 `CASE WHEN <columns[x]> THEN x END`，且遵循 spike 3b 的形态——用户过滤改写为「`columns[x]` AND 原列过滤」、遮蔽只留在最外层投影 [CITED: .planning/research/spike-03b-scoped-overlay.md 第 2 节规则 1，ADR-0020]；本 phase 不做用户过滤改写，但 `lower` 必须能分别输出 `row` 与每列的授予谓词，供 Phase 4 直接拼装。必测用例：role A（行谓词 p_A，列集 C_A）与 role B（p_B，C_B）合并，构造只满足 p_A 的行，断言只属于 C_B 的列被遮蔽。

### Pattern 8: 认证

- **验证器路由按 JWT header 的 `alg`，不看未验证 payload**：非对称算法（RS*/PS*/ES*/EdDSA）→ OIDC 路径；`HS256` → dev 路径（仅当 `--dev` 构造了 dev 验证器，否则拒绝）；`none` 与其他一律拒绝。OIDC `Validation` 的 `algorithms` 取显式白名单且**不含 HS***；`set_issuer`、`set_audience` 必填，`required_spec_claims` 至少 `exp`、`iss`、`aud`、`sub`，`leeway` 小值（建议 30 秒，可配）。Context7 文档里的示例 `Validation::new(header.alg)` 直接信任令牌头的算法，**不要照抄** [CITED: Context7 /keats/jsonwebtoken jwk.md]。永不跟随令牌里的 `jku` / `x5u` / `jwks_uri`，JWKS 地址只来自 `ontoforge-server.toml`。
- **JWKS 缓存**：`RwLock<Cached{ keys: HashMap<kid, DecodingKey>, fetched_at, expires_at }>`；TTL 取响应 `Cache-Control: max-age` 夹在 [60 s, 24 h]，缺省 1 h；`kid` 未命中 → 进入单飞重拉（`tokio::sync::Mutex` 内再次检查缓存），两次重拉最小间隔（建议 30 s，可配）；对未知 `kid` 做负缓存，使攻击者用随机 kid 最多每个间隔触发一次拉取；拉取失败时继续用旧集（stale-if-error，上限建议 24 h 后 fail-closed）；响应体大小上限（建议 1 MiB）与超时；仅允许 `https`，`http` 只允许回环地址（与 Phase 2 的 T-02-14 同口径）。`DecodingKey::from_jwk` 对 RSA / EC / OKP 分别走 `from_rsa_components` / `from_ec_components` / `from_ed_components`，`Other` 报 `UnsupportedAlgorithm` [CITED: Context7 jwk.md]。
- **principal**：`principal_id` 规范串 `"{iss}|{sub}"`（`|` 不可能出现在 URL 形式的 `iss` 中；配置校验 `iss` 不含 `|`），同时在 `of_user` 存 `(iss, sub)` 两列；system 主体用 `system:import` 这类不含 `|` 的保留串，避免与用户冲突。`last_login_at` 更新在进程内节流（建议同一主体每 60 秒至多一次写）。
- **dev token**：HS256，密钥 32 字节随机，存 `<数据目录>/dev-token.key`（Unix 0600；Windows 靠用户目录 ACL，文档注明）；`ontoforge dev-token --sub .. --groups a,b --attr tenantId=acme --ttl 8h`；iss 固定 `ontoforge-dev`。dev 验证器**只能由调用方显式传入 `DevMode` 凭证构造**（binary 层 `--dev`），配置文件里有 `[auth.dev]` 段不足以启用。
- `ontoforge-server.toml` 扩展（沿用 `*_env` / `*_file` 规则，Phase 2 D-06/D-07）：`[auth.oidc] issuer, audience, jwks_uri, groups_claim`、`[auth.principal_attrs.<name>] claim, type`、`[auth.dev] key_file`、`[store] kind, path | url_env, schema_edit, schema_sys, lock_timeout_ms`、`[store.trino] catalog, schema`。`ServerConfig` 当前 `deny_unknown_fields` 且位于 `ontoforge-engine/src/config.rs`（`pub struct ServerConfig` 在 32 行）[VERIFIED]，新段的数据类型放同文件，行为在 auth / storage。

### Pattern 9: roles-as-code

**Kinds**（Claude's Discretion，遵守 ADR-0012 的 camelCase + `deny_unknown_fields` + `kind` 重写）：

```yaml
kind: Role
apiName: dispatcher            # camelCase，scope 目录内唯一
description: ...
grants:
  objectTypes:
    Flight:
      properties: [flightId, flightNumber, status]   # 属性 allow-list，须含 pk（见 OQ4）
      rowFilter: "object.status != 'CANCELLED' || object.status == null"
  actions: [assignPilot, delayFlight]
  sqlLevels: []                # 缺省仅 L0；可含 raw / admin
  l2: { catalogs: [], schemas: [] }
  toolsets: [core]             # core | full | sql:raw （ADR-0019）
---
kind: RoleBinding
role: dispatcher
subjects:
  - user: "https://idp.example.com|alice"
  - group: "dispatchers"
```

- 加载：`ontoforge-schema` 新增 `ScopeDefinition { Role, RoleBinding }` 枚举与独立 JSON Schema 文件 `ontoforge-scope.schema.json`（若把两个 kind 塞进现有 `Definition`，会让 ontology 目录的编辑器补全出现不属于该目录的 kind；`schema_export::rendered_schemas` 当前导出两个文件 [VERIFIED: schema_export.rs]）。`dispatch()` 的 kind 白名单与 `Definition` 反序列化目前写死（`KINDS` / `IMPLEMENTED_KINDS` 常量，`Definition` 枚举）[VERIFIED: loader/dispatch.rs:14-17、ir/mod.rs]，需泛化为按「文档集合」参数化；ontology 目录里出现 `Role` / `RoleBinding` 报 `ONT-V03` 并提示应放 scope 目录。
- apply = 全量同步：plan 输出 role 增 / 改 / 删与 binding 增 / 删；apply 在单事务内重算 plan 并校验 hash，写 `of_role`（IR JSON + 内容 hash）、`of_role_binding`、一条 `of_admin_audit`（actor = 操作系统用户 @ 主机、plan JSON、前后 hash）。校验阶段（apply 之前，离线）：`rowFilter` 子集 / 类型 / 同行引用、`properties` 引用存在、`actions` 引用存在、criteria 覆盖（SCOPE-06）、D-08 collation（需要选定 profile 的绑定：`ColumnBinding.collation`，枚举值 `CaseSensitive` / `CaseInsensitive`，在 `ir/binding.rs:94-98`）、`principal.attrs` 引用须在 `ontoforge-server.toml` 声明。CLI：`ontoforge scope plan|apply <scope-dir> --ontology <dir> --server-config <file> [--profile]`。
- 索引对账：ADR-0016「索引 = pk + 行谓词引用的属性 + `indexed: true` 属性」要求 role 变更也可能触发 `CREATE INDEX`。推荐：对账函数同时被 reload 与 scope apply 调用，只增不删，`IF NOT EXISTS`（OQ9）。
- **IR 现状缺口**：`PropertyDef` 目前字段为 `base_type`、`required`、`display_name`、`description`、`items`、`precision`、`scale`，**没有 `indexed`** [VERIFIED: ir/object.rs:44-72 已读]，也没有 `default`；Wave 0 需新增 `indexed: bool`（`#[serde(default)]`，进 JSON Schema 与 insta 快照，`ontoforge schema export` 重生成）。

### Pattern 10: 变更历史 API（STORE-04 / D-13）

`history(scope: &TypeScope, attrs, current_row: &ResolvedRow, object_ref) -> Result<Vec<HistoryEntry>, NotFound>`。调用方（Phase 4 / 7）负责给出已叠加 source 的当前完整行（edit-only 类型可直接取 edit 行）；函数用同一 `TypeScope.eval` 判 `current_row` 可见，不可见与不存在返回同一错误；可见列集由 `current_row` 求出（不是历史行）；每条历史把不在可见列集内的列只标 `changed: true` 不给值；Action `params` 字段对普通调用者整体省略。conformance：构造被遮蔽列，断言响应序列化后不含其值（扫描整个 JSON）。

### Anti-Patterns to Avoid

- **用 `cel` 运行时求值行谓词**：其 `null == 'a'` 为 false、`null != 'a'` 为 true，与 D-06 相反；只用它解析。
- **把 role 谓词 SQL 直接拼字符串**：所有文本经 printer（SQL 出口唯一）；storage 内同样要有 `sql_exit.rs` 作为唯一 `AssertSqlSafe` 调用点，并复用 Phase 2 `sql_text_only_from_printer.rs` 的扫描器模式写守卫（该测试扫描 `"SELECT|WITH|INSERT|UPDATE|DELETE|CREATE|DROP|ATTACH|PRAGMA"` 字面量 [VERIFIED: tests/sql_text_only_from_printer.rs:15]）。
- **先分配 `seq` 再写 edit 行**（见 Pattern 3）。
- **SQLite 默认 DEFERRED 事务写入**（`SQLITE_BUSY`）。
- **在同一条 SQLite ALTER 里放多个操作**（sea-query 文档：SQLite 不支持单语句多操作 [CITED: Context7 DeepWiki sea-query 7.2]），一条语句一个操作。
- **让 edit store 与 SQLite 替身共用连接**：替身是单连接内存库且加载后 `PRAGMA query_only=ON`（`sqlite_ddl.rs` 的 `query_only`）[VERIFIED]；edit store 用独立 `SqlitePool`（文件或共享缓存内存库），Phase 3 无需 ATTACH 到替身。

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| JWT 签名校验、算法族匹配、claims 校验 | 自己拆 JWS / 比对签名 | `jsonwebtoken` 11.1.0 | 只自写 JWKS 拉取缓存（ADR-0008）；算法与 key 族匹配、exp / nbf / aud / iss 校验交给库，我们只设白名单 |
| PG / SQLite 方言的 DDL / DML 文本 | 手拼 `ALTER TABLE` / `INSERT` | sea-query 1.0.2（`Table::alter().rename_column`、`Index::create().if_not_exists()`、`Query::insert`） | 标识符引号与方言差异；每条 SQLite ALTER 单操作 |
| 迁移执行与校验和 | 自写迁移表 | `sqlx::migrate!` | 系统表与触发器的版本管理 |
| ULID | 自己编码 | `ulid` 3.0.0（已在 workspace） | — |
| 随机字节 | 用时间戳或 `RandomState` 当密钥 | `getrandom` | dev 密钥的唯一密码学随机源 |
| 字符串匹配转义 | 临时拼 `%` | printer 的 `StringMatch` 节点（两方言各一个转义函数，位于 `escape` 出口） | 注入与大小写语义分叉的唯一防线，proptest 覆盖 |
| criteria 引用提取 | 正则扫表达式 | `ontoforge_expr::member_paths` | 已处理 comprehension 变量误报与 `['field']` 索引写法 |
| Rust 与 SQL 的结果比对框架 | 手写穷举 | proptest（已在 workspace，Phase 2 `printer_prop.rs` 有字符串策略与 `FileFailurePersistence` 配置可复用） | 反例自动落盘为回归 |
| 嵌入式 PG 的下载 / 解包 / initdb | 自己下 PG 二进制 | `postgresql_embedded`（test-only feature） | 项目约束指定 |

**Key insight:** 本 phase 的难点是「两处实现必须逐位一致」（Rust vs SQL、SQLite vs PG、model vs store、log vs tables）。凡是能让两处共用同一份数据结构 / 同一出口的地方（`RowPred`、`TypeScope`、`PhysicalSchema`、转义函数、`next_state`）都不要复制，剩下无法共用的地方用差分 / 重放 / 参考模型三类测试夹住。

## Common Pitfalls

### Pitfall 1: SQLite `INSERT OR REPLACE` 绕过 append-only 触发器
**What goes wrong:** 日志行被静默覆盖，conformance 的「UPDATE / DELETE 被拒」测试全绿。
**Why it happens:** `recursive_triggers` 默认 OFF 时 REPLACE 冲突解决的隐式 DELETE 不触发 DELETE 触发器 [CITED: sqlite.org/pragma.html 说明默认 OFF；行为本机实测]。
**How to avoid:** 每个连接 `PRAGMA recursive_triggers=ON`；代码中禁止 `OR REPLACE`；加回归测试。实测输出（SQLite 3.51.0）：关闭时 `INSERT OR REPLACE INTO log VALUES (1,'b') -> ALLOWED [('b',)]`、`REPLACE INTO log VALUES (1,'z') -> ALLOWED [('z',)]`；开启后 `... -> rejected: append-only`。
**Warning signs:** 触发器测试只覆盖 `UPDATE` 与 `DELETE`。

### Pitfall 2: SQLite `LIKE` 大小写不敏感，与 Rust / Trino 分叉
**What goes wrong:** `object.name.startsWith('A')` 在 SQLite 替身上也命中 `'abc'`，差分测试在 ASCII 大小写混合的语料上失败，更糟的是 `serve --dev` 下可见范围比生产宽（安全回归）。
**Why it happens:** 默认 `'a' LIKE 'A'` 为真；`PRAGMA case_sensitive_like` 已 deprecated，可能被 `SQLITE_OMIT_DEPRECATED` 编译掉，且改变 LIKE 定义可能让 schema 中使用 LIKE 的对象看起来损坏 [CITED: sqlite.org/pragma.html]。
**How to avoid:** SQLite 方言一律 `GLOB`（见 Pattern 6）；差分语料必须含大小写混合、`%`、`_`、`\`、`[`、`*`、`?`、空串、非 ASCII。

### Pitfall 3: 三值逻辑里空 `IN` 与 NULL 元素
**What goes wrong:** 空 `principal.attrs.regions` 生成 `IN ()`，Trino 语法错误，或 Rust 返回 Unknown 而 SQL 返回 FALSE。
**How to avoid:** 空列表编译为常量 FALSE 且 Rust 对 `x IN []` 返回 False（即使 x 为 Null）；JWT 里 list claim 的 null 元素在 attr 映射阶段丢弃或拒绝；list 字面量禁 null。
**Warning signs:** 差分生成器没有「空列表」「含 null 的 attr」。

### Pitfall 4: 显式 null 分支会让「缺失 attr」变成可见
**What goes wrong:** ADR-0018 写「缺失属性相关谓词求值为 false」，但 D-06 之后 `principal.attrs.tenant == null || …` 在 attr 缺失时为 TRUE。
**How to avoid:** 文档与谓词规范明说「fail-closed 来自 Unknown 不可见，而不是对 attr 缺失特判」；可选：lint 警告（W 级）出现 `principal.attrs.x == null` 这类对主体属性的 null 测试。属 D-06 的已接受后果，不要在实现里特判。

### Pitfall 5: postgresql_embedded 默认 feature 把 native-tls / openssl 拉进图
**What goes wrong:** Linux CI 需要 openssl 头文件，或 Windows 引入 schannel；`Cargo.lock` 里出现第二个 TLS 栈。
**Why it happens:** 其 `default = ["tls-native-tls","theseus"]` 且它依赖 `sqlx ^0.9.0`，features 会与 workspace 的 sqlx 统一 [VERIFIED: crates.io API]。
**How to avoid:** `default-features = false`，显式选 `tls-rustls-aws-lc-rs`；Wave 0 守卫：`cargo tree -i openssl` 必须无输出（当前 `Cargo.lock` 的 `openssl` 条目为 0）。

### Pitfall 6: jsonwebtoken 11 不选 crypto 后端会 panic
**What goes wrong:** 第一次 decode 时 panic。
**How to avoid:** `features = ["aws_lc_rs"]`；二选一不可同时开 `rust_crypto` [CITED: Context7]；Wave 0 加一个真正签发并校验一次 RS256 / HS256 的单测当冒烟。

### Pitfall 7: Trino PG connector 默认跳过数组列
**What goes wrong:** reload 后的「新列可见」探测对数组列永远失败，reload 永远超时。
**Why it happens:** `postgresql.array-mapping` 默认 `DISABLED`，数组列被跳过；不支持的类型默认被 `IGNORE` [CITED: trino.io/docs/current/connector/postgresql.html]。仓库的 `ci/trino/catalog/demo.properties` 设了 `postgresql.array-mapping=AS_ARRAY`，而 `ci/trino/catalog/editstore.properties` 没有 [VERIFIED: 两文件已读]。
**How to avoid:** 探测只检查 Trino 能映射的列（数组、geopoint 的 jsonb→`JSON` 需映射规则）——探测列集 = 物理 schema 里类型在 Trino 可见集合内的列，其余跳过并在 `describe` 输出标注；同时在 `editstore.properties` 增加 `postgresql.array-mapping=AS_ARRAY` 并写进部署文档。

### Pitfall 8: 标识符长度与碰撞
见 Pattern 1：apiName 100 字符对 PG 63 字节；snake 化可能碰撞。构造期检测并报错，不要静默截断。

### Pitfall 9: sqlx 0.9 动态 SQL 必须 `AssertSqlSafe`
`sqlx::query_with(&sql, values)` 对 `&String` 不再编译 [VERIFIED: sqlx-core-0.9.0/src/sql_str.rs 文档「To bypass this error, manually audit for potential injection vulnerabilities and wrap with `AssertSqlSafe()`」]。把唯一包装点收敛到 `storage/src/sql_exit.rs`，其余模块拿到的是 sea-query 产物，由出口函数包装；守卫测试禁止其他文件出现 `AssertSqlSafe`。

### Pitfall 10: 角色 / 组跨 issuer 混淆
多个 issuer 同时受信时，`groups: ["admins"]` 在两个 IdP 里语义不同。v1 推荐配置只允许一个 OIDC issuer 断言 groups（dev 令牌除外），多 issuer 时 group 键必须带 issuer（OQ10）。

### Pitfall 11: PG `ALTER TABLE` 的锁
`RENAME COLUMN` / `ADD COLUMN` 取 ACCESS EXCLUSIVE 锁（短暂），非 `CONCURRENTLY` 的 `CREATE INDEX` 阻塞写 [ASSUMED：PG 通用行为]；所以 DDL 事务前置 `SET LOCAL lock_timeout`，且 `CREATE INDEX CONCURRENTLY` 不能放在事务里，edit 表通常很小，v1 用普通 `CREATE INDEX` + 超时，不做并发建索引。

### Pitfall 12: 重放 / 历史对时间与 seq 的依赖
重放 conformance 若比较 `committed_at` 或自增 seq 会假失败：比较 edit 表内容与 `of_version`，日志重放保持原 seq 以便整体等价，时间戳列排除在比较之外。

## Code Examples

### SQLite 方言 GLOB 转义（Pattern 6）
```rust
// 来源：GLOB 语义本机实测（t2.py）：特殊字符 * ? [ 用方括号转义；'' glob '**' = 1
fn glob_escape(needle: &str) -> String {
    let mut out = String::with_capacity(needle.len() + 8);
    for c in needle.chars() {
        match c {
            '[' => out.push_str("[[]"),
            '*' => out.push_str("[*]"),
            '?' => out.push_str("[?]"),
            other => out.push(other),
        }
    }
    out
}
// StartsWith => format!("{}*", glob_escape(n)); EndsWith => format!("*{}", ..); Contains => format!("*{}*", ..)
// 最终字面量仍须经 printer 的 quote_string 出口。
```

### Trino 方言 LIKE 转义
```rust
// 来源：trino.io/docs/current/functions/comparison.html（ESCAPE 单字符；转义字符自身需转义）
fn like_escape(needle: &str) -> String {
    needle.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}
// 打印：<expr> LIKE '<pattern>' ESCAPE '\'
```

### Kleene 三值核心
```rust
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tri { True, False, Unknown }
fn and(a: Tri, b: Tri) -> Tri { match (a, b) { (Tri::False, _) | (_, Tri::False) => Tri::False,
    (Tri::True, Tri::True) => Tri::True, _ => Tri::Unknown } }
fn or(a: Tri, b: Tri) -> Tri { match (a, b) { (Tri::True, _) | (_, Tri::True) => Tri::True,
    (Tri::False, Tri::False) => Tri::False, _ => Tri::Unknown } }
fn not(a: Tri) -> Tri { match a { Tri::True => Tri::False, Tri::False => Tri::True, Tri::Unknown => Tri::Unknown } }
```

### SQLite 写事务与连接选项
```rust
// 来源：sqlx-sqlite 0.9.0 options：pragma / collation / busy_timeout 存在；Pool::begin_with(impl SqlSafeStr)
let opts = SqliteConnectOptions::new()
    .pragma("recursive_triggers", "ON")          // Pitfall 1
    .foreign_keys(true)
    .busy_timeout(std::time::Duration::from_secs(5))
    .collation(DECIMAL_COLLATION, compare_decimal)       // 需 Wave 0 导出
    .collation(TIMESTAMP_COLLATION, compare_timestamp);
let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
```

### PG DDL 事务骨架
```rust
// SET 不接受绑定参数：整数来自配置，格式化后经唯一出口包装
let mut tx = pool.begin().await?;
sql_exit::execute(&mut *tx, &format!("SET LOCAL lock_timeout = '{lock_timeout_ms}ms'")).await?;
// ... 幂等 DDL ... 失败时 SQLSTATE 55P03 -> ReloadError::LockTimeout（保留旧定义）
tx.commit().await?;
```

### JWT 校验骨架（白名单算法，不信任 header.alg）
```rust
// 来源：Context7 /keats/jsonwebtoken validation.md（set_audience / set_issuer / set_required_spec_claims / leeway）
let mut v = Validation::new(Algorithm::RS256);
v.algorithms = vec![Algorithm::RS256, Algorithm::ES256, Algorithm::EdDSA]; // 不含 HS*
v.leeway = 30;
v.set_audience(&[cfg.audience.as_str()]);
v.set_issuer(&[cfg.issuer.as_str()]);
v.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `sea-query-binder`（sqlx 0.8） | `sea-query-sqlx` 0.9.1（sqlx ^0.9） | 2025-08 起新名 | Context7 / 旧文档示例仍用 `sea_query_binder::SqlxBinder`，导入路径以 crate 实际为准 |
| sqlx 0.8 `query(&String)` | sqlx 0.9 要求 `SqlSafeStr`，动态串用 `AssertSqlSafe` | sqlx 0.9 | 项目已采用（`sqlite/mod.rs`） |
| jsonwebtoken 9：`rust_crypto` 内置 | 11：`rust_crypto` / `aws_lc_rs` 二选一，无则 panic | 10+ | Pitfall 6 |
| SQLite `case_sensitive_like` | `GLOB` / 显式大小写函数 | pragma 标 deprecated | Pitfall 2 |

**Deprecated/outdated:** `PRAGMA case_sensitive_like`（deprecated）；`cel-interpreter`（改名 `cel`，workspace 已用新名）；`rusqlite`（与 sqlx 的 `libsqlite3-sys` links 冲突，ADR-0021 已否）。

## Runtime State Inventory

不适用：本 phase 是新增能力，不是 rename / refactor / migration。注：`ontoforge migrate` 的 rename 属于用户 ontology 的数据迁移，由 Pattern 5 覆盖。

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `sea-query-sqlx` 为 sea-query 当前官方 sqlx 绑定 crate（名称来源于项目 rust-stack.md 与 crates.io 元数据；Context7 文档仍写旧名） | Package Legitimacy Audit | 低：源码已读、legitimacy OK；错则换 `sea-query-binder` 并随之降 sqlx，需复核 |
| A2 | `getrandom` 0.4.3 的 API 满足「填充 32 字节」且无需额外 feature | Standard Stack | 低：换成 `aws-lc-rs::rand` 或 `rand` |
| A3 | workspace 的精确 feature 组合（sea-query、sea-query-sqlx、postgresql_embedded 的 tls feature）能一次 `cargo check` 通过且不引入 openssl | Standard Stack、Pitfall 5 | 中：Wave 0 立即暴露，调整 feature 名即可 |
| A4 | sqlx 的 `migrate!` 能执行含 `$$` 函数体（PG）与 `BEGIN…END;` 触发器（SQLite）的多语句迁移 | Pattern 4 | 中：第一个 plan 用真实迁移验证；退路是把触发器 DDL 作为 Rust 常量经 `sql_exit` 逐条执行 |
| A5 | 经现有 Trino client 发送 `CALL <catalog>.system.flush_metadata_cache()`（无结果列的语句）能正常完成 | Pattern 5 | 中：需 Linux CI 集成测试确认；失败则在 client 里补「无列结果」处理。（Trino 文档确认该过程存在且无参数） |
| A6 | Trino 对 `col = NULL`、`NULL IS NULL`、`x IN (…)` 的三值语义与 SQLite 一致（标准 SQL 行为） | Pattern 6 | 低：固定种子语料在 Linux CI 验证 |
| A7 | `postgresql_embedded` 可经 Settings 把版本固定到 17.x（GitHub 上有 17.11.1 等 release）且在 Windows 11 非管理员账号下可 initdb | Environment Availability | 中：首次运行需下载；失败时本机只跑 SQLite，PG 由 CI 覆盖 |
| A8 | PG identity 序列取值顺序与提交顺序不保证一致；PG `ALTER TABLE RENAME/ADD COLUMN` 取 ACCESS EXCLUSIVE，非并发 `CREATE INDEX` 阻塞写 | Pattern 3、Pitfall 11 | 低：设计保守，错则只是多余的谨慎 |
| A9 | 物理编码采用每属性 `<col>__set` 布尔列而非单个 `set_mask` 位图，用户可接受（ADR 只写「`set_mask` 标记有效列」） | Pattern 1/2，OQ1 | 中：若坚持位图，需增「属性→位槽」注册表且限制 ≤ 63 属性；影响 DDL 生成与 Phase 4 候选查询形态 |
| A10 | 新增 required 属性一律视为破坏性（IR 无 `default`） | Pattern 5，OQ2 | 低：只会更保守；放开需加 IR 字段 |
| A11 | dev 密钥文件在 Windows 上依赖用户目录 ACL 即可 | Pattern 8 | 低：dev-only |
| A12 | PG 的 Trino 账号只读 `ontoforge_edit` schema，两个 schema 的拆分被接受 | Pattern 1，OQ5 | 低：不接受则合并为一个 schema |

## Open Questions

1. **`set_mask` 的物理编码**
   - What we know: ADR-0016 与 D17 写「`set_mask` 标记有效列」；spike 3b 用 `int` 位图；ADR 未规定物理形式。
   - What's unclear: 用户是否把「位图」当作锁定项。
   - Recommendation: 采用每属性 `<col>__set` 布尔列（无上限、无注册表）。若用户要位图，则 `bigint set_mask` + `of_property_slot` 注册表、槽永不复用、每 type ≤ 63 个属性（含归档槽）并加 lint。

2. **IR 缺 `default`，新增 required 属性如何处理**
   - What we know: D20 写「新增 required 属性须带 default，否则按破坏性」；`PropertyDef` 无 `default`。
   - Recommendation: 本 phase 不加 `default`，新增 required 属性一律 destructive（走 `migrate`，source 行不受影响，edit 表列可空）。如需放开另开 IR 变更。

3. **STORE-06「替身引擎数据」的含义**
   - What we know: 替身是内存库，数据来自绑定 fixture，不落盘（Phase 2 拒绝 `sqlite_dir`）。
   - Recommendation: `ontoforge import` 只导 editOnly object type 与 editOnly M:N link（CSV / JSON，复用 fixture 的类型规则），经 `commit()` 以 `system:import` 写入，默认 create-only（已存在 pk 报错，`--upsert` 显式放开），按 1000 行一个 commit；「替身引擎数据」由既有 fixture 满足。

4. **allow-list 是否必须含主键**
   - Recommendation: 必须含（validate 报错），保持显式；Phase 4 的身份与去重依赖 pk。

5. **PG 是否拆 `ontoforge_edit` / `ontoforge_sys` 两个 schema**
   - Recommendation: 拆，Trino 账号最小权限；schema 名可配，测试用唯一后缀。

6. **`[auth]` / `[store]` 配置类型放哪**
   - Recommendation: 数据类型放 `ontoforge-engine/src/config.rs`（`ServerConfig` 与 `Checker` 已在此），行为在 auth / storage。

7. **Role / RoleBinding 字段形状**
   - Recommendation: 采用 Pattern 9 的草案（属 Claude's Discretion），落地时同步 JSON Schema 与 insta 快照；新诊断码建议：`ONT-V21` scope 文档结构与引用、`ONT-V22` 行谓词子集 / 类型 / 同行引用（D-07、D-08 的 E 级归此）、`ONT-E17` criteria 列覆盖（SCOPE-06）、`ONT-E18` 物理名碰撞 / 超长、`ONT-E19` reload 检出破坏性变更。码永不复用；E10-E12、W12-W16 已被占用 [VERIFIED: diag.rs 注释与 ADR-0021 第 11 条]。

8. **Trino `CALL` 经现有 client 的行为**（A5）：Linux CI 集成测试确认。

9. **scope apply 是否自动建索引**：推荐是（对账函数与 reload 共用，只增不删）。

10. **多 issuer 的 group 命名空间**：v1 配置只允许单个 OIDC issuer，多 issuer 时 group 绑定带 issuer。

11. **字符串范围比较在 source 列上的下推**：Trino PG connector 默认不下推 `CHAR` / `VARCHAR` 的范围谓词，等值与 `IN` 下推 [CITED: trino.io PG connector docs]。这影响 Phase 4 性能而不影响本 phase 正确性；D-08 已处理大小写不敏感列。

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | 全部 | ✓ | cargo 1.98.0 / rustc 1.98.0 | — |
| SQLite（bundled） | SQLite 后端 conformance、差分 | ✓ | 3.51.3（libsqlite3-sys 0.37.0 头文件）；本机 `sqlite3` CLI 3.51.0 | — |
| PostgreSQL 本机 | PG conformance（本机） | ✗（`psql` / `pg_ctl` 未找到） | — | `postgresql_embedded`（首次运行从 GitHub 下载，网络可达：`github.com/theseus-rs/postgresql-binaries/releases/latest` 返回 200，Windows msvc 包存在）；或 CI service container |
| Docker | 不依赖（项目约束） | 二进制存在、daemon 状态不作为依赖 | 29.7.2 client | 不使用 |
| Trino | 可见探测、固定语料差分 | ✗（本机无） | CI 为 483 / 484 | Windows 本机跳过，Linux CI `--ignored` 跑 |
| GitHub Actions `integration` job | PG conformance、Trino 语料 | ✓（已有 PG 17.11 service + `editstore` 数据库已在 job 里创建；`ci/trino/catalog/editstore.properties` 已存在） | — | 需新增 `cargo test -p ontoforge-storage --test conformance_pg -- --ignored` 步骤与「ignored 测试数量非零」守卫 |
| Python 3.13（仅研究用） | 本研究的 SQLite 行为实测 | ✓ | 3.13.9 | 不进入产品代码 |

**Missing dependencies with no fallback:** 无。
**Missing dependencies with fallback:** 本机 PG（用 embedded 或 CI）、Trino（Linux CI）。注意：Phase 2 记录的 Windows broken-windows ledger 中 CI integration job 尚未真正在 GitHub 上跑过（`WINDOWS.md` 第 5、6 行 open），Phase 3 新增的 CI 步骤同样首次落地前需手动 `workflow_dispatch` 验证。

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust 内置 `cargo test` + proptest 1.12.0 + insta 1.49.0 + wiremock 0.6.5 |
| Config file | 无统一配置；proptest 失败持久化沿用 `printer_prop.rs` 的 `FileFailurePersistence::WithSource("proptest-regressions")` 模式 |
| Quick run command | `cargo test -p ontoforge-expr -p ontoforge-auth -p ontoforge-storage --locked` |
| Full suite command | `cargo test --workspace --locked`（Windows 与 Linux 均跑，仅依赖 SQLite） |

默认 proptest 用例数：纯函数（Kleene、状态机）256；带 SQLite 的 store / 差分 64（`PROPTEST_CASES` 环境变量可在 nightly 调高），保证 Windows 本机默认套件耗时可控。

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| STORE-01 | 同一 conformance 在 SQLite / PG 通过 | integration | SQLite：`cargo test -p ontoforge-storage --test conformance_sqlite --locked`；PG：`ONTOFORGE_PG_URL=… cargo test -p ontoforge-storage --test conformance_pg --locked -- --ignored`；本机 embedded：`cargo test -p ontoforge-storage --features embedded-pg --test conformance_pg -- --ignored` | ❌ Wave 0 |
| STORE-02 | additive reload 自动 DDL；destructive 被拦；`migrate --plan/--apply` 归档；失败保留旧定义；`lock_timeout`；栅栏 | integration | `cargo test -p ontoforge-storage --test reload --locked`；`cargo test -p ontoforge --test cli_reload --locked`；`cargo test -p ontoforge --test cli_migrate --locked` | ❌ Wave 0 |
| STORE-02 | Trino 刷新与可见探测 | integration（Linux CI） | `ONTOFORGE_TRINO_URL=… cargo test -p ontoforge-storage --test reload_trino_it --locked -- --ignored` | ❌ Wave 0 |
| STORE-03 | 提交追加日志；重放重建 edit store；UPDATE / DELETE / TRUNCATE / `INSERT OR REPLACE` 被拒 | integration | `cargo test -p ontoforge-storage --test replay --locked`；`cargo test -p ontoforge-storage --test append_only --locked` | ❌ Wave 0 |
| STORE-04 | 历史按当前 scope 遮蔽，响应不含被遮蔽值 | unit/integration | `cargo test -p ontoforge-storage --test history_masking --locked` | ❌ Wave 0 |
| STORE-05 | oneToOne FK 唯一（partial unique index）、M:N 重复添加 | integration | 并入 `conformance`（`cardinality_*` 用例） | ❌ Wave 0 |
| STORE-06 | import：类型解析、create-only、system 主体、日志 | integration | `cargo test -p ontoforge --test cli_import --locked` | ❌ Wave 0 |
| STORE-07 | 状态机不变量 + 模型对照 | property | `cargo test -p ontoforge-storage --test model_prop --locked` | ❌ Wave 0 |
| SCOPE-01 | scope plan / apply：全量同步、删除绑定列入 plan、审计 append-only | integration | `cargo test -p ontoforge --test cli_scope --locked` | ❌ Wave 0 |
| SCOPE-02 | Role / RoleBinding 解析、JSON Schema 快照、诊断快照 | unit/snapshot | `cargo test -p ontoforge-schema --test scope_documents --locked` | ❌ Wave 0 |
| SCOPE-03 | 多 role 合并；A 的列 × B 的行不泄露 | property + example | `cargo test -p ontoforge-auth --test scope_merge --locked` | ❌ Wave 0 |
| SCOPE-04 | Rust 求值 ≡ SQL（SQLite）proptest；Trino 固定语料 | property / integration | `cargo test -p ontoforge-auth --test rowpred_diff --locked`；`ONTOFORGE_TRINO_URL=… cargo test -p ontoforge-auth --test rowpred_trino_it --locked -- --ignored` | ❌ Wave 0 |
| SCOPE-05 | 无授权不可见；缺失 attr = null；非子集谓词报错 | unit | `cargo test -p ontoforge-expr --test rowpred_subset --locked` | ❌ Wave 0 |
| SCOPE-06 | criteria 覆盖 lint 在 apply 与 reload 触发 | unit/integration | `cargo test -p ontoforge-auth --test criteria_coverage --locked`；reload 侧并入 `reload` 测试 | ❌ Wave 0 |
| SCOPE-07 | role 只经 apply 写；每次 apply 一条审计 | integration | 并入 `cli_scope` | ❌ Wave 0 |
| AUTH-01 | RS256 / ES256 校验、过期 / 错 iss / 错 aud / 错 alg / `none` / HS256 冒充被拒；JWKS 缓存、kid-miss 节流、stale-if-error | unit（wiremock） | `cargo test -p ontoforge-auth --test oidc --locked` | ❌ Wave 0 |
| AUTH-02 | dev token 往返；非 `--dev` 不接受；密钥文件权限 | unit | `cargo test -p ontoforge-auth --test dev_token --locked`；`cargo test -p ontoforge --test cli_dev_token --locked` | ❌ Wave 0 |
| 守卫 | SQL 文本只经 `sql_exit`；`OR REPLACE` 不出现；依赖图无 openssl | static | `cargo test -p ontoforge-storage --test sql_exit_guard --locked`；`cargo tree -i openssl`（应为空） | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p <被改 crate> --locked`（对应单文件 `--test <name>`）
- **Per wave merge:** `cargo test --workspace --locked` + `cargo clippy --workspace --all-targets --locked -- -D warnings` + `cargo fmt --all --check`
- **Phase gate:** 完整套件绿，Linux CI `integration` job 的 PG conformance 与 Trino 固定语料步骤绿，再 `/gsd-verify-work`

### Wave 0 Gaps
- [ ] workspace 依赖与 feature 调整；`cargo tree -i openssl` 守卫
- [ ] `ontoforge-storage/tests/conformance.rs`（泛型 harness）+ `conformance_sqlite.rs` + `conformance_pg.rs`（`#[ignore]`，读 `ONTOFORGE_PG_URL` 或 `embedded-pg`）
- [ ] 参考模型与 `model_prop.rs`、`replay.rs`、`append_only.rs`、`sql_exit_guard.rs`
- [ ] `ontoforge-expr/tests/rowpred_subset.rs`；`ontoforge-auth/tests/{rowpred_diff,scope_merge,criteria_coverage,oidc,dev_token}.rs`
- [ ] 差分测试的 SQLite 行集构造 helper（在测试内用 sqlx 直连 `ATTACH ':memory:' AS "<catalog>__<schema>"`，经 printer 打印后执行；不改 `SqliteEngine` 的只读语义）
- [ ] CI：`integration` job 增加 PG conformance 与 Trino 语料步骤；新增 ignored 测试数量守卫（沿用 ci.yml 现有模式）
- [ ] 导出 `DECIMAL_COLLATION` / `TIMESTAMP_COLLATION` 与比较函数供 storage 复用
- [ ] aviation 示例：新增 `examples/ontologies/aviation-scope/`（或同级目录）的 Role / RoleBinding 示例（含多 role 列 × 行交叉）与 editOnly 的 `Airport` import 数据
- [ ] `ci/trino/catalog/editstore.properties` 增 `postgresql.array-mapping=AS_ARRAY`

## Security Domain

`security_enforcement: true`，ASVS level 1，`security_block_on: high`。

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes | OIDC JWT（jsonwebtoken 白名单算法 + iss / aud / exp 校验）；dev 令牌独立验证路径且仅 `--dev` |
| V3 Session Management | no | 无服务端会话，令牌无状态；`last_login_at` 仅记录 |
| V4 Access Control | yes | roles-as-code；fail-closed；多 role 合并公式；apply 唯一写入口 + 审计 |
| V5 Input Validation | yes | CEL 子集校验 + 类型检查；printer 唯一 SQL 出口与两个转义函数；DDL 标识符来自已校验的 apiName 并经 sea-query 引号；import 文件大小 / 行数上限 |
| V6 Cryptography | yes | 不手写：JWT 交给 jsonwebtoken（aws-lc-rs）；dev 密钥用 OS 随机数；`_version` HMAC 属 Phase 5 |
| V7 Error Handling & Logging | yes | edit log / 历史 / 管理审计 append-only；错误不含物理名或被遮蔽值 |
| V8 Data Protection | yes | Action `params` 原样存日志（D-11，脱敏延后），历史 API 不对普通调用者返回；JWKS / 令牌不写日志 |
| V14 Configuration | yes | 密钥只经 `*_env` / `*_file`；`deny_unknown_fields`；PG 应用账号最小权限（REVOKE UPDATE/DELETE/TRUNCATE） |

### Known Threat Patterns for {Rust + PG/SQLite + OIDC}

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| JWT 算法混淆（`none`、HS256 用公钥当密钥） | Spoofing | 按 `alg` 路由 + 显式白名单；OIDC 路径不含 HS*；不信任 header 中的 `jku` / `x5u` |
| `kid` 洪泛导致 JWKS 重拉放大 | DoS | 单飞 + 最小重拉间隔 + 负缓存 + 响应大小 / 超时上限 |
| 过期或错受众令牌被接受 | Spoofing | `exp`、`iss`、`aud`、`sub` 必填并校验 |
| 行谓词注入 / 越权（字符串拼接、LIKE 通配符） | Tampering / Info Disclosure | `RowPred` IR → printer；转义函数唯一出口；proptest 注入语料（含 `%`、`_`、`\`、`'`、`[`、`*`、`?`） |
| 多 role 列 × 行交叉泄露 | Info Disclosure | `columns[x] = OR(R_i where x∈C_i)`；专项用例 |
| 谓词 / 聚合反推被遮蔽列 | Info Disclosure | 遮蔽只放投影层、用户过滤改写（Phase 4 落地；本 phase 提供分列授予谓词） |
| SQLite 替身比生产更宽（LIKE 大小写） | Elevation of Privilege | `GLOB` + 差分语料含大小写混合 |
| 审计 / 日志被改写（含 `INSERT OR REPLACE`） | Repudiation / Tampering | 触发器（含 TRUNCATE）+ `recursive_triggers=ON` + 账号权限 + 回归测试 |
| 并发 reload / 写入穿越定义切换 | Tampering | `of_def_state` 栅栏 + advisory 锁 + `lock_timeout` |
| 恶意 / 超大 Role YAML 或 import 文件 | DoS | 沿用 loader 的 `MAX_FILE_BYTES` 4 MiB / `MAX_FILES` / `MAX_TOTAL_BYTES` 预算与 `deny_unknown_fields` [VERIFIED: loader/mod.rs:43,46,50]；import 流式读取并设行数上限 |
| dev 令牌在生产被接受 | Elevation of Privilege | dev 验证器不由配置构造，仅 `--dev` 路径持有 `DevMode` 凭证；生产配置含 `[auth.dev]` 时启动告警并忽略 |
| 供应链：新依赖与 build script、`postgresql_embedded` 运行时下载二进制 | Tampering | `Cargo.lock` 入库 + `--locked`；legitimacy 审核已做；embedded-pg 仅 test feature、默认关闭，下载包带 `provenance.json`（是否校验由 crate 实现决定，未验证，见 A7） |
| 管理操作者身份弱（CLI 无 authN） | Repudiation | 审计记录 OS 用户 @ 主机与 plan hash；Phase 7 REST 管理端接入后补强 |

## Sources

### Primary (HIGH confidence)
- 仓库文件（本 session 已读）：`03-CONTEXT.md`、`REQUIREMENTS.md`、`STATE.md`、`ROADMAP.md`（Phase 3 段）、`ADR-0002/0006/0008/0011/0012/0014/0015/0016/0018/0019/0020/0021`、`docs/designs/trino-engine-sql-tiers.md`、`spike-03b-scoped-overlay.md`、`rust-stack.md`、`crates/ontoforge-{expr,engine,schema,storage,auth,action,query}` 关键源文件、`.github/workflows/ci.yml`、`ci/trino/catalog/*`、`02-SECURITY.md`、`WINDOWS.md`
- 本机实测：SQLite 3.51.0（Python 3.13 绑定）触发器 / REPLACE / GLOB / LIKE / ALTER / 部分唯一索引 / 三值逻辑；`cel = "=0.15.0"` AST 探针；sqlx 0.9.0 与 sea-query-sqlx 0.9.1、sea-query 1.0.2 源码阅读；crates.io API 版本与 features
- Context7 `/keats/jsonwebtoken`（features、JWK 转换、Validation）、`/websites/deepwiki_seaql_sea-query`（alter / SQLite 限制 / sqlx 集成）
- trino.io：PostgreSQL connector 文档（`system.flush_metadata_cache()`、`metadata.cache-ttl` 默认 `0s`、`postgresql.array-mapping` 默认 `DISABLED`、字符串范围谓词不下推）、functions/comparison（LIKE ESCAPE 区分大小写）
- sqlite.org：lang_altertable、pragma（`case_sensitive_like` deprecated、`recursive_triggers` 默认 OFF）
- postgresql.org/docs/17：errcodes-appendix（`55P03`、`23000`、`P0001`、`40001`、`40P01`、`23505`）

### Secondary (MEDIUM confidence)
- GitHub `theseus-rs/postgresql-embedded` README（Windows 支持、二进制来源、`bundled` feature）与 `theseus-rs/postgresql-binaries` releases API
- `gsd-tools query package-legitimacy check`（crates 生态）

### Tertiary (LOW confidence)
- 标注 `[ASSUMED]` 的 PG 锁 / 序列行为与 migrate! 多语句行为（见 Assumptions Log）

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — 版本、features、依赖关系均来自 crates.io API 与源码；唯一不确定的是精确 feature 组合的一次通过（A3）
- Architecture: HIGH — crate 依赖修正、状态机、写路径顺序都与已锁 ADR 一致并由实测支撑；`__set` 布尔列属物理选择（A9，待确认）
- Pitfalls: HIGH — 触发器绕过、LIKE 大小写、三值 IN、cel AST 形态均已本机实测复现
- Trino 集成（`CALL`、空结果、固定语料）: MEDIUM — 依赖 Linux CI 首次验证（A5、A6）

**Research date:** 2026-10-19
**Valid until:** 2026-11-18（sqlx / sea-query / jsonwebtoken 迭代较快，Wave 0 开始前复核版本）
