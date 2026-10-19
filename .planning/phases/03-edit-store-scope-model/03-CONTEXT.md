# Phase 3: Edit Store & Scope Model - Context

**Gathered:** 2026-10-19
**Status:** Ready for planning

<domain>
## Phase Boundary

交付两块系统数据能力，供 Phase 4（scoped L0 读）与 Phase 5（online Action）消费：

1. **edit store**：按 object type / M:N link type 生成强类型 edit 表，PG 与 SQLite 双后端同一套 conformance；按属性 patch 的叠加语义（`set_mask`、tombstone、release、版本单调）；不可变 edit log 与历史表；变更历史查询（库级 API）；定义 reload（新增类变更自动 DDL）与 `ontoforge migrate --plan/--apply`（破坏性变更）；`ontoforge import`；link 基数在 edit store 写入时强制。
2. **scope 模型**：user / group / role 与授权项（roles-as-code）、CEL 行谓词的 SQL 编译与 Rust 单行求值（差分等价）、多 role 合并、criteria 列覆盖 lint、OIDC JWT 与 dev token 认证。

不在本 phase：L0 读路径编排与叠加查询（Phase 4）、Action 鉴权管线执行与 `command_id` / outbox（Phase 5）、L2（Phase 6）、REST / MCP 与常驻服务（Phase 7）。需求：STORE-01..07、SCOPE-01..07、AUTH-01..02。

**已锁定、不再讨论**（ADR-0016、ADR-0018、设计文档 Eng Review D3 / D5 / D12 / D13 / D17 / D19 / D20 / D23）：强类型分表、按属性 patch 叠加、M:N link edit 表 `(a_pk, b_pk, deleted, version)`、text 列 `COLLATE "C"`、索引规则、reload 加固（`lock_timeout`、`flush_metadata_cache` + 可见探测、`def_version` 栅栏）、多 role 合并公式、行谓词只引用同行属性与 `principal.attrs`、fail-closed、OIDC JWT + 自写 JWKS 缓存。

</domain>

<decisions>
## Implementation Decisions

### 角色 / scope 的管理通道
- **D-01:** roles-as-code：role 及其授权项（可见 type、属性 allow-list、行谓词、action、SQL 级别、L2 catalog / schema allow-list、toolset）写在独立目录的 YAML 中（新 kind，暂名 `Role` / `RoleBinding`，不与 ontology 定义或 `SourceBinding` 混放），经 CLI plan / apply 写入系统库，每次 apply 记管理审计。行谓词校验与 criteria 列覆盖 lint 在 apply 前离线执行，不通过则整体不生效。— **Reversibility:** costly — 新 kind 进入 loader、JSON Schema 与示例；改成命令式需重做变更通道与审计。
- **D-02:** role → user / group 的绑定写在 YAML 中，apply 为**全量同步**（plan 列出将删除的绑定）。user ↔ group 成员关系不落库，每次请求取自 JWT `groups` claim；dev token 由 CLI 指定 groups。不支持本地自建组。
- **D-03:** apply 是系统库中 role / 绑定的唯一写入口；Phase 7 的 REST 管理端对 role 只读（满足 SCOPE-07「只经管理通道或 L3 修改并审计」）。
- **D-04:** `principal_id` = `iss` + `sub`；首次认证自动建 user 记录，只存标识与最近登录时间，不存任何权限。
- **D-05:** `principal.attrs` 的 claim → 属性映射写在 `ontoforge-server.toml`，每个属性声明类型（String / Integer / list<String> 等），供行谓词静态类型检查；缺失按 null（再经 D-06 得不可见）。

### 行谓词语义
- **D-06:** null 语义以 **SQL 三值逻辑**为准：Rust 单行求值器实现 Kleene 三值逻辑，最终 unknown → 不可见。因此 `object.x != 'a'` 在 x 为 null 时不可见；想让 null 可见须显式写 `object.x == null || …`。此语义与标准 CEL 不同，必须写入用户文档与谓词规范。— **Reversibility:** one-way — 一旦有用户按此语义写了 role，改语义会静默改变可见范围（安全回归）；差分测试语料与反例集也随之作废。
- **D-07:** 行谓词允许的 CEL 子集：比较运算（== != < <= > >=）、`&&` `||` `!`、`== null` / `!= null`、`in`（右侧为列表字面量或 list 型 `principal.attrs`）、字符串 `startsWith` / `endsWith` / `contains`（编译为带转义的 LIKE）。拒绝：宏（all / exists / map / filter）、`matches`、算术、`size`、其他函数。超出子集 → validate / apply 报 E。
- **D-08:** 行谓词引用了在绑定中声明为大小写不敏感 collation 的列 → E（列表路径必须把谓词下推到 Trino，D23 禁止此类列下推字符串比较）。
- **D-09:** 差分测试按 D23：完整 proptest 只跑 Rust vs SQLite；Trino 端跑固定种子语料 + 历史反例（Linux CI）。

### edit log
- **D-10:** edit log 每条 edit 只存**新值 + 操作类型**：`(type, pk, op ∈ {set, unset, tombstone, release, link_add, link_remove}, 设置的属性 → 新值, 变更后 version)`。不存旧值、不复制 source 值。重放 edit log 必须能重建 edit store（作为 conformance 用例），这也是后续 scenario / branch overlay 的输入。— **Reversibility:** one-way — 格式 MVP 定死（PROJECT.md），已写入的日志无法补出旧值，scenario / branch 将依赖此格式。
- **D-11:** 一条 log 记录 = 一次成功提交（可含多条 edit）：全局单调 `seq` + ULID `event_id`、actor（principal_id）、action、时间、`def_version`、Action 参数（原样存储，脱敏留给 Markings 立项）。`ontoforge import` 以保留的 system 主体记入。— **Reversibility:** one-way — 同 D-10。
- **D-12:** edit log 与历史表永不删除：PG 与 SQLite 都以触发器拒绝 UPDATE / DELETE，conformance 测试覆盖。
- **D-13:** 变更历史查询（STORE-04）按**当前 scope 遮蔽**：调用者须对该对象当前可见（不可见与不存在返回同一错误）；每条历史只返回当前可见列的值，其余列只标「有变更」不给值；Action 原始参数不对普通调用者返回，只经管理通道。本 phase 交付库级 API，REST / MCP 外壳在 Phase 7。

### reload / migrate
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

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 已锁决策
- `.planning/adr/0016-storage-system-data-and-typed-edit-store.md` — edit store 形态、叠加粒度、定义变更与 reload 加固、写入与审计承诺
- `.planning/adr/0018-authz-scope-and-sql-tiers.md` — 主体、role 授权项、多 role 合并、同源两处执行、Action 管线（Phase 5 消费，本 phase 需为其提供数据结构）、配置期 lint
- `.planning/adr/0008-auth.md` — AuthN 部分仍有效（OIDC JWT + JWKS 缓存、dev 本地 token）
- `.planning/adr/0006-expression-language.md`、`.planning/adr/0014-cel-reserved-words-not-apinames.md` — CEL 约束
- `.planning/adr/0002-naming-convention.md`、`.planning/adr/0012-definition-document-shape.md` — 新 kind 与字段命名、文档形状
- `.planning/adr/0021-source-binding-files-and-backing.md` — `backing: editOnly` 与绑定文件，决定哪些 type 有 source
- `docs/designs/trino-engine-sql-tiers.md` §3 Scope 模型、「Eng Review 决策」D3 / D5 / D12 / D13 / D17 / D19 / D20 / D23、Crate 布局与依赖方向

### 前置调研
- `.planning/research/spike-03-trino-overlay-latency.md` — 强类型表 vs JSONB 实测，edit store catalog 不开缓存
- `.planning/research/spike-03b-scoped-overlay.md` — scoped 编译「role 谓词 AND 原列过滤、遮蔽只在投影层」形态，行谓词 SQL 编译须产出此形态
- `.planning/research/rust-stack.md` — sqlx / sea-query / cel 选型

### 需求与上一 phase
- `.planning/REQUIREMENTS.md` — STORE-01..07、SCOPE-01..07、AUTH-01..02
- `.planning/phases/02-source-binding-engine/02-CONTEXT.md` — 服务配置文件 `ontoforge-server.toml`、密钥只以 `*_env` / `*_file` 引用（D-06 / D-07），本 phase 的 claim 映射与 JWKS 配置沿用
- `.planning/phases/02-source-binding-engine/02-SECURITY.md` — 待跟进的非阻塞安全 observations

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/ontoforge-expr/`（analyze / check / infer / walk / limits）：CEL 预检、类型推断与 AST 遍历；行谓词子集校验、SQL 编译与单行求值基于它，`analyze` 是入口。
- `crates/ontoforge-engine/src/printer.rs`：自有 Trino / SQLite printer，值只经两个转义函数输出；行谓词 SQL 编译输出到该 printer，不另写 renderer。
- `crates/ontoforge-engine/src/sqlite.rs`：SQLite 替身（`ATTACH` 的 `catalog__schema`），差分测试的 SQL 端。
- `crates/ontoforge-schema/src/loader/`、`diag.rs`、`lint/`、`schema_export.rs`：`Role` / `RoleBinding` 作为新 kind 接入 dispatch，诊断码与 JSON Schema 快照沿用现有模式。
- `crates/ontoforge-engine/src/config.rs`：`ontoforge-server.toml` 解析，claim 映射与 OIDC 配置在此扩展。

### Established Patterns
- IR 是唯一契约，`deny_unknown_fields`，新字段须进 JSON Schema 与 insta 快照。
- 新依赖需 legitimacy 审核（Phase 1 SECURITY.md）；当前 workspace 尚无 sea-query、sqlx `postgres` feature、JWT 库，引入时按此审核。
- Windows 本机 `cargo test` 只依赖 SQLite；PG 测试本机走 `postgresql_embedded`，CI 走 service container。
- proptest 已在 workspace 依赖中。

### Integration Points
- `crates/ontoforge-storage`、`crates/ontoforge-auth`、`crates/ontoforge-action` 目前是 14 行空壳。`ontoforge-storage/Cargo.toml` 依赖 `ontoforge-query`，与设计文档依赖方向 `schema/expr → auth → query → engine/storage → action` 需在 planning 时核对。
- Lane 冲突提示（设计文档「并行化」）：storage 与 auth 都可能改 `ontoforge-expr`，由 auth 独占该 crate 的修改。
- CLI `crates/ontoforge/src/`：新增 `reload`、`migrate`、`import`、role apply、dev token 子命令。
- `examples/ontologies/aviation/`：补 role 示例（含多 role 列 × 行交叉用例）与 import 数据。

</code_context>

<specifics>
## Specific Ideas

- 行谓词典型用例：`object.tenantId == principal.attrs.tenantId`、`object.region in principal.attrs.regions`、`object.region == null || object.region == principal.attrs.region`（「未分配区域对象可见」须显式写 null 分支）。
- conformance 必含：edit log 重放重建 edit store；edit log / 历史表 UPDATE / DELETE 被拒绝；历史查询不泄露被遮蔽列值；多 role「A 的列 × B 的行」不泄露。

</specifics>

<deferred>
## Deferred Ideas

- Action 参数脱敏：随 Markings 子系统立项。
- 本地自建组（不依赖 IdP）：当前不支持，有真实需求再议。
- 定义中的稳定 id（零成本改名）：未采纳，如改名频繁再评估。

</deferred>

---

*Phase: 03-edit-store-scope-model*
