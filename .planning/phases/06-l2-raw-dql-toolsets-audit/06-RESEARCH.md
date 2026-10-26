# Phase 6: L2 Raw DQL, Toolsets & Audit - Research

**Researched:** 2026-10-26
**Domain:** Rust SQL AST gate, Trino 483 ACL, PostgreSQL/SQLite read audit
**Confidence:** MEDIUM (existing interfaces and negative probes HIGH; production ACL behavior remains to be exercised)

<user_constraints>
## User Constraints (from CONTEXT.md)

The following is planner input copied from `06-CONTEXT.md`; that file and accepted ADRs remain authoritative.

### Locked Decisions

- **D-01:** 保持 ADR-0018 的 `sql:raw` + catalog/schema allow-list，L2 不施加 L0 行列 scope；`system` 元数据表与 edit store catalog 硬排除。只接受单条 SELECT / WITH，拒绝 TABLE()、其他表函数、CALL、EXECUTE、多语句、会话/事务语句和无法解析的输入；执行仅发送校验后的 sqlparser Display 文本，不发送原文。解析器、allow-list 与 LIMIT 必须在单一受控入口收敛；任何无法证实的 AST 变体 fail-closed。
- **D-02:** Trino ontology/L2 身份隔离。L2 默认不启用，只有部署方显式声明 Trino ACL 且启动自检证明 allow-list 外、edit store 被拒及 `system.runtime.queries` 不泄露其他主体查询时才可启用；未知、超时或成功越界均禁用 L2 并报告健康状态。按主体 `X-Trino-User` 透传是已接受设计中的可选部署模式，不是 Phase 6 强制新合同。
- **D-03:** `sqlparser` 仅校验 L2 输入，不替换现有类型化 SQL printer。现有 `PrintedSql` 私有构造应保留为信任边界；research/planner 设计受控已验证 SQL 到 Engine 的窄入口，不能把可由调用者构造的任意字符串提升为 `PrintedSql`。
- **D-04:** `AUDIT-01` 记录主体、工具、对象类型、apiName 级条件、生效 role、SQL 哈希、行数、耗时和 request_id，不存结果数据或完整原始 SQL。双后端系统表、有界异步批量队列、可配置保留期；读请求在无法入队、积压或持续落库失败时返回 `AuditUnavailable`。按 ADR-0016 的承诺，崩溃时至多丢约 1 秒已受理读审计；验收须覆盖队列与 sink 故障。— **Reversibility:** costly — 审计持久字段与已有查询入口的接线会成为运营合同，变更需要迁移和跨模块回归。
- **D-05:** Phase 6 复用已有 `reload` 与 `scope plan|apply` CLI 管理边界。定义先 validate/lint，失败不切换；role/scope 仍受计划哈希、定义锁及事务性 `of_admin_audit` 保护；请求开始固定 `Arc<ActiveDefinition>` 快照。新增远程管理 API 和 MCP 工具不在本 phase，Phase 7 外壳复用库接口。
- **D-06:** spike 2 的历史 104 条仅以 Markdown 描述保留，原始 SQL 恢复 0 条。`06-SPIKE-483.md` 已对 29 条新构造等价样例复验，尚有 75 个历史 ID 未覆盖；`G11` 的 ROW 类型和 `G16` 的 OFFSET/LIMIT 虽 Display AST 往返相等，却在 Trino 483 执行失败。必须固定机器可执行 corpus，区分原文与新写样例，对实际生产 AST 白名单、Display、拒绝零发送及 Trino 483 行为复验，不能将此 spike 当成最终安全验收。现有 483 容器没有 ACL，ACL 验收需独立受限配置。

### Planner Discretion

审计队列容量/批量参数、内部 trait/API、SQL 受控入口的模块布局和 ACL 探针实现由 research 与 planner 依据现有 crate 决定。必须对所有公开读取入口逐一列矩阵，证明不会绕过审计；队列已满时无法写入同一审计 sink 的拒绝事件，不承诺不存在的审计记录，可用健康/日志观察。核心逻辑与安全边界按 TDD 先 RED 后 GREEN。

### Deferred Ideas (OUT OF SCOPE)

REST/MCP 暴露与真实生产发布属于 Phase 7；L1 Ontology SQL 与 `sql:ontology` 属下一 milestone。当前没有额外产品范围。
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | REQUIREMENTS.md 原文 | Research Support |
| --- | --- | --- |
| L2-01 | 被授予 `sql:raw` 的主体可以执行只读 Trino DQL 并列举 allow-list 内的 catalog / schema / 表 | `PrincipalScope` 与六个库级入口、授权过滤；见 Architecture Patterns。 |
| L2-02 | L2 SQL 经 AST 白名单校验（只允许 SELECT / WITH，拒绝表函数、CALL、EXECUTE、多语句、会话与事务语句），执行时发送重序列化后的 SQL | `PrintedSql` 边界、固定 104-ID corpus、Trino 483 Display 证据。 |
| L2-03 | L2 只能访问 catalog / schema allow-list，硬排除 edit store catalog 与 `system` 元数据表 | AST relation/CTE/表函数遍历加独立 ACL；拒绝路径零 POST。 |
| L2-04 | 启动自检无法证明 L2 身份在 Trino 侧受限时禁用 L2 并告警（fail-closed） | 独立受限 Trino 配置、已知另一主体活跃 query 和健康状态。 |
| AUDIT-01 | 所有级别的读取记录元数据审计（不含结果数据），异步落库，积压或持续失败时读请求 fail-closed；保留期可配 | `QueryService` 三个公开读入口与六个 L2 入口、双后端 conformance、队列故障注入。 |
</phase_requirements>

## Summary

推荐按四个依赖边界规划：先固定 104-ID 的机器可执行 constructed-equivalent corpus 和 Trino 483 差分结果，再在 `ontoforge-engine` 建唯一的 L2 校验至发送入口；用独立受限 Trino fixture 证明 ACL；最后把双后端审计 sink 和有界队列接入每个公开读取操作。[VERIFIED: `06-CONTEXT.md` D-01..D-06; `06-ENG-REVIEW.md` P0; `06-SPIKE-483.md` 结论]

`G11` 应显式拒绝有损 ROW 类型形式，`G16` 只在受控 AST 改写成 Trino 483 已执行成功的 `OFFSET ... FETCH FIRST ... ROWS ONLY` 后放行。历史 G/B 标签不是当前策略真相，fixture 须逐 ID 记录合同预期、受支持性和例外理由；不能把 `parse(Display) == ast` 或 `CATALOG_NOT_FOUND` 算成语义/权限证明。[VERIFIED: `06-SPIKE-483.md` 结论、Trino 483 实测; `.planning/research/spike-02-sqlparser-l2.md:158-180`]

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
| --- | --- | --- | --- |
| AST gate、LIMIT/Display 与唯一发送 | `ontoforge-engine` | `ontoforge-auth` | `Engine::execute` 信任 `PrintedSql`；主体 grants 来自已编译 scope。[VERIFIED: `crates/ontoforge-engine/src/lib.rs:8-11,86-103`; `crates/ontoforge-auth/src/scope/compile.rs:50-66`] |
| ACL 自检和健康状态 | `ontoforge-engine` 库层 | 部署方 Trino | 现有 `TrinoEngine` 按身份持有 HTTP client；真实授权由 Trino ACL 执行。[VERIFIED: `crates/ontoforge-engine/src/trino/mod.rs:35-68`; ADR-0018] |
| 审计系统表、批量写、保留期 | `ontoforge-storage` | PG/SQLite | `EditStore::open` 已分别运行迁移，PG system schema 与 edit schema 分离。[VERIFIED: `crates/ontoforge-storage/src/store.rs:1-7,174-210`] |
| 审计门禁和结果释放 | `ontoforge-query`、L2 库接口 | audit sink | 公开读入口在 QueryService，内部复用不可重复算作对外读。[VERIFIED: `crates/ontoforge-query/src/lib.rs:226-239`; `execute.rs:392-421`; `aggregate.rs:872-895`] |
| REST/MCP 与 `/health` 路由 | Phase 7 shell | Phase 6 库健康状态 | 当前 Phase 6 只交付可复用调用与状态。[VERIFIED: `06-CONTEXT.md` Phase Boundary、D-05; ADR-0019] |

## Standard Stack

| Existing dependency | Verified version / location | Use |
| --- | --- | --- |
| `sqlparser` | `0.63.0`, workspace dependency，当前仅 `ontoforge-engine` dev-dependency。[VERIFIED: `Cargo.toml:43`; `crates/ontoforge-engine/Cargo.toml:29-38`; `cargo info sqlparser@0.63.0`] | 提升为 engine 正常 dependency，启用 `visitor`，固定 lockfile；只用于 L2 输入校验与 Display。[VERIFIED: ADR-0017; `.planning/research/spike-02-sqlparser-l2.md:2-5`] |
| `reqwest` / `tokio` | workspace 已有 `0.13.5` / `1.53.3`。[VERIFIED: `Cargo.toml:34-35`] | 复用现有 Trino HTTP client 与 `tokio::sync`，无新 client。[VERIFIED: `crates/ontoforge-engine/src/trino/mod.rs:1-5,50-68`] |
| `sqlx` / `sea-query` | workspace 已有 `0.9.0` / `1.0.2`。[VERIFIED: `Cargo.toml:36,45`] | 复用迁移、PG/SQLite store 与现有 conformance 测试；不要引入第二套 ORM。[VERIFIED: `crates/ontoforge-storage/src/store.rs:174-210`; ADR-0016] |

**Package Legitimacy Audit:** `sqlparser` 已在 lockfile/测试依赖中；本次 `cargo search sqlparser --limit 1` 返回 `0.63.0`，`gsd-tools query package-legitimacy check --ecosystem crates sqlparser` 返回 `OK`（2018-09-03 首发、仓库为 `apache/datafusion-sqlparser-rs`）。无需新包。[VERIFIED: registry commands 2026-10-26; `Cargo.lock` sqlparser entry]

## Architecture Patterns

### Request path

```text
Principal + fixed definition/scope -> audit capacity/health reservation
  -> L0 public read OR L2 authorize -> AST whitelist -> Display -> Trino ACL -> bounded result
  -> metadata audit enqueue -> result release
  -> AuditUnavailable on failed reservation/sink; L2 disabled on failed ACL proof
```

此顺序是基于 D-01..D-05 的设计建议，必须由测试证明取消、失败和 reload 中途也走相同门禁。[VERIFIED: `06-CONTEXT.md` D-01..D-05; `06-ENG-REVIEW.md` P0]

### SQL trust boundary

`PrintedSql` 定义为 `pub struct PrintedSql(String)`，但 `fn new(sql: String)` 为私有；Trino POST body 是 `sql.as_str().to_owned()`，源级守卫测试扫描 `src/printer/` 之外的 SQL 字面量/构造。[VERIFIED: `crates/ontoforge-engine/src/printer/mod.rs:19-37`; `crates/ontoforge-engine/src/trino/mod.rs:205-223`; `crates/ontoforge-engine/tests/sql_text_only_from_printer.rs:1-45`；原文引用]

把 L2 gate 放在 `printer` 模块树的窄入口，只有全链通过才能生成供 `Engine::execute` 使用的值；公开函数只接收原始请求与不可伪造的 grants/config，不接受调用者制造的“已校验 SQL”。任何 SQL 字面量探针也留在该模块树，并保留源级守卫测试。[VERIFIED: 上述私有构造和守卫；设计推论]

`TableFactor::Table { args: Option<TableFunctionArgs>, .. }` 的 `args: Some` 是表值函数，`TableFunction` 与 `Function` 为其他变体；只依靠 relation visitor 会漏判。`LimitClause::LimitOffset` 的 Display 先写 `LIMIT` 再写 `OFFSET`。逐层白名单枚举所有 `Statement`、`SetExpr`、`TableFactor`、函数与查询子句，未知变体默认拒绝。[VERIFIED: `C:/Users/easyi/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sqlparser-0.63.0/src/ast/query.rs:1460-1530,3110-3150`；对应源码原文 `args: Option<TableFunctionArgs>`、`TableFunction`、`Function`、`write!(f, " LIMIT {limit}")`]

输入字节上限、专用栈的压力用例和完整 AST drop 路径必须在 TDD 内覆盖；32 KiB/8 MiB 是 spike 建议值，不是已证明的 Linux 阈值。`G11` 保守拒绝有损类型；`G16` 的受控改写与外层 cap 要在真实 Trino 483 上验证结果，拒绝的任何输入 HTTP POST 计数为零。[VERIFIED: `.planning/research/spike-02-sqlparser-l2.md:179-181,183-207`; `06-SPIKE-483.md` 实测]

### ACL proof

另建受限 483 fixture，显式 `access-control.name=file` 与 `security.config-file`、catalog/schema/table、query 和 system information 规则；官方文档说明无 query rules 时所有用户可看/杀任意主体查询，自己的 query 始终可见。`system.runtime.queries` 包含其他用户的原始 SQL 和用户名，因此用已知另一主体的活跃查询进行负向证明；未知、对象不存在、语法错误或空结果不能算权限拒绝。[CITED: https://trino.io/docs/483/security/file-system-access-control.html; https://trino.io/docs/483/connector/system.html; VERIFIED: `06-ENG-REVIEW.md` P0]

正探针须证明 allow-list 内实际对象可读；负探针须针对确实存在的越界对象/edit catalog 得到权限拒绝，并证实另一主体的 query 不可见。身份相同、配置未声明、探针不确定、超时及复检失败均保留禁用和不泄露秘密的原因。[VERIFIED: `06-CONTEXT.md` D-02; `06-ENG-REVIEW.md` P0]

### Audit and read paths

`QueryService` 当前公开 `execute`、`execute_list`、`execute_aggregate`，后者调用 `execute_list_internal`；`read_pk_source` 是 Action 复用的内部 source lookup。把审计门禁放在三个公开方法入口，内部方法不要重复记录。库级 L2 六个操作逐个经过同一门禁。`QueryService` 的字段当前公开，新增审计依赖时需改所有构造调用并防止公开方法绕开；`AggregateError` 的转换也需保留 `AuditUnavailable` 而非折叠为 `InvalidQuery`。[VERIFIED: `crates/ontoforge-query/src/lib.rs:65-108,226-239,370-377`; `execute.rs:392-421`; `aggregate.rs:113-143,701-712,872-895`；枚举原文 `InvalidQuery`, `Source`, `Store`，尚无 `AuditUnavailable`]

推荐先在存储层实现同语义 `of_read_audit` PG/SQLite 系统表及批量写/按保留期清理，再用有界 permit 在实际读取前预留容量；提交元数据记录后才能释放结果。队满或持续写失败返回 `AuditUnavailable`，sink 状态/最旧待写时间必须可观测，重试耗尽与 writer 退出禁用新读；取消路径需要明确是否入队及进程关闭 drain。约 1 秒是故障丢失上界目标，应使用受控时钟/阻断 writer 测试，不称为每次响应已持久化。[VERIFIED: ADR-0016; `06-CONTEXT.md` D-04; `06-ENG-REVIEW.md` P0]

读审计允许保留期清理，勿套用 `of_admin_audit` 的 append-only 删除触发器；PG 新表由 `schema_sys` 的 `search_path` 创建，不暴露给 edit store catalog。保留期使用明确单位、正数校验和索引，清理失败进入 sink 健康策略并有 PG/SQLite conformance。[VERIFIED: `crates/ontoforge-storage/src/store.rs:1-7,174-210`; `migrations/postgres/0003_scope.sql:30-45`; `migrations/sqlite/0003_scope.sql:30-49`; ADR-0016；设计推论]

## Don't Hand-Roll

| Problem | Use |
| --- | --- |
| SQL 词法/AST | 已锁定 `sqlparser` 0.63.0 `GenericDialect` + AST 白名单；不以正则判断只读。[VERIFIED: ADR-0018/0019; spike 2] |
| Trino 协议 | 复用现有 `TrinoEngine` POST/GET/DELETE、timeout 与 cancellation。[VERIFIED: `crates/ontoforge-engine/src/trino/mod.rs:205-249,345-380`] |
| 数据库迁移与测试 | 复用 `sqlx::migrate!`、`EditStore` 和双后端 conformance 宏。[VERIFIED: `crates/ontoforge-storage/src/store.rs:174-210`; `tests/conformance_pg.rs:1-18`; `tests/conformance_sqlite.rs:1-8`] |

## Common Pitfalls

1. Display AST 往返相等仍有 Trino 语义损坏 (`G11`, `G16`)；测试必须比较 Trino 483 响应，已知有损形态拒绝或受控改写。[VERIFIED: `06-SPIKE-483.md`]
2. `CATALOG_NOT_FOUND`、空结果和默认查询权限不证明 ACL 拒绝；自检必须区分拒绝错误与其他终态，造出另一主体活跃 query。[VERIFIED: `06-SPIKE-483.md` B53; CITED: https://trino.io/docs/483/security/file-system-access-control.html]
3. 104 个历史 ID 的原 SQL 均不在仓库；固定新 fixture 必须标 `constructed-equivalent`，逐 ID 记录策略预期与原因，75 个未覆盖 ID 不能靠现有 29 条声称通过。[VERIFIED: `06-SPIKE-483.md` 本地覆盖与缺口]
4. 只在 L2 审计或只审计内部 source lookup 会漏掉纯 PG L0、集合和聚合；每个公开读入口与六个 L2 操作测成功、拒绝、源错、截断、取消。[VERIFIED: `06-ENG-REVIEW.md` P0; `crates/ontoforge-query/src/aggregate.rs:701-712`]
5. 队列已接受与数据库已持久化并非同一事件；writer 失败后释放先前计算出的未审计结果是 fail-open。约 1 秒失效承诺须实测。[VERIFIED: ADR-0016; `06-ENG-REVIEW.md` P0]
6. `StoreConfig`、`QueryService` 新字段会改变现有测试构造；PLAN 须列全部调用点和兼容调整，不能靠测试模式无审计默认值旁路。[VERIFIED: `crates/ontoforge-engine/src/config.rs:30-47,195-220`; `crates/ontoforge-query/src/lib.rs:226-235`；设计推论]

## Validation Architecture

| Requirement | RED then GREEN evidence | Command family |
| --- | --- | --- |
| L2-01..03 | 固定 104-ID corpus、属性/CTE/嵌套/函数/未限定名/`system` 拒绝、wiremock 证明拒绝零 POST、Display body 精确匹配、限制行数与字节 | `cargo test -p ontoforge-engine --locked l2_`; `cargo test -p ontoforge-engine --test trino_it --locked -- --ignored` [VERIFIED: tests and CI; commands are planned additions] |
| L2-04 | 独立受限 Trino 483 的实际正/反探针、另一主体活跃 query、ACL 撤销复检、未知/timeout 禁用健康状态 | 新 `trino_l2_acl_it` ignored target 并加入 Linux CI；不能复用无 ACL `ci/query-services.sh trino` [VERIFIED: `ci/query-services.sh:27-60`; `06-SPIKE-483.md`] |
| AUDIT-01 | PG/SQLite migration 与批量/清理 parity，队满、持续 sink 故障、恢复、取消；各公开入口无未审计结果、库表无完整 SQL/结果 | `cargo test -p ontoforge-storage --test conformance_sqlite --locked audit_`; PG 同名 ignored target，`cargo test -p ontoforge-query --locked audit_` [VERIFIED: `tests/conformance_pg.rs:1-18`; `.github/workflows/ci.yml:75-88`; commands are planned additions] |
| SC-5 | reload lint 拒绝不切换，scope plan hash 错误不写，成功写管理审计；请求中途 reload 的版本/scope 快照一致 | 现有 reload/scope 测试加 focused case [VERIFIED: `06-ENG-REVIEW.md` P1; `crates/ontoforge-storage/tests/scope_store.rs` path verified by file listing] |

Windows `cargo test --workspace --locked` 仍为基线，真实 PG/Trino ignored 测试需在 CI 保证发现数大于 0 并实际执行；`ci.yml` 已有相同门禁模式。[VERIFIED: `.github/workflows/ci.yml:40-50,94-103,122-148`]

## Security Domain

| ASVS category | Applicability | Contract test |
| --- | --- | --- |
| V2 Authentication | 已有主体与双服务身份，Phase 6 不引入新认证方式。[VERIFIED: `crates/ontoforge-engine/src/config.rs:136-167`; ADR-0018] | L2 与 ontology 身份不能相同；身份/ACL 不可证实时禁用。 |
| V4 Access Control | 核心。[VERIFIED: ADR-0018] | `sql:raw` + grant 交叉、hard exclusions、真实 Trino ACL 与查询所有者隔离。 |
| V5 Input Validation | 核心。[VERIFIED: ADR-0018; sqlparser source above] | 单条 AST 白名单、长度/深度/栈压力、Display-only 发送与零 POST。 |
| V6 Cryptography | 审计 SQL 哈希复用项目现有 digest 实现或标准 crate，不自写散列。[VERIFIED: `crates/ontoforge-storage/Cargo.toml:39`; ADR-0016] | 不记录完整 SQL 或结果，确认字段与日志脱敏。 |

## Environment Availability

| Dependency | Local observation | Phase gate |
| --- | --- | --- |
| Rust | `cargo 1.98.0`, `rustc 1.98.0`，符合 workspace `rust-version = "1.98"`。[VERIFIED: CLI 2026-10-26; `Cargo.toml:7`] | Windows unit/conformance。 |
| Docker / Trino | Docker 29.7.2；`ontoforge-trino-05-06` healthy `trinodb/trino:483`，但无 ACL。[VERIFIED: CLI 2026-10-26; `06-SPIKE-483.md`] | 语法/执行复验可用，ACL 需新隔离配置。 |
| PostgreSQL | CI 已启动 PostgreSQL 17.11；本地是否启动未核实。[VERIFIED: `.github/workflows/ci.yml:69-88`; `ci/query-services.sh:3-26`] | CI 双后端测试或 `embedded-pg`。 |
| Context7 | CLI `ctx7` 不在本机 PATH，MCP 未提供。[VERIFIED: local command discovery 2026-10-26] | 使用版本固定的 Trino 483 官方文档和已下载 sqlparser 源码。 |

## Project Constraints (from AGENTS.md)

本仓库没有磁盘上的 `AGENTS.md`；本会话由用户提供的 AGENTS.md 约束有效：GSD `.planning/` 维持阶段记忆，`planning-with-files` 的 `task_plan.md`/`progress.md`/`findings.md` 为持久计划；核心安全/数据逻辑 TDD RED-GREEN-REFACTOR；子任务验收需证据；Phase 6 先 research/PLAN/checker 后 execute；状态与 ROADMAP 只写摘要指针；默认中文；不得把 gstack 独立复审冒充正式 gate。[VERIFIED: user-supplied AGENTS.md; `06-ENG-REVIEW.md` header]

## Open Questions / Planning Decisions

- 配置字段、队列容量、批量周期与保留期单位属 `06-CONTEXT.md` Planner Discretion。PLAN 应选明确的正值范围并用失败注入证明约 1 秒承诺；这是工程参数，不需新增产品决定。[VERIFIED: `06-CONTEXT.md` Planner Discretion; ADR-0016]
- 历史 `G11` 被标 G，但已锁定拒绝有损 ROW 类型；PLAN 的 corpus 预期应记录 `known-unsupported` 而非伪称原始 G 成功。`G25/G29/G30/G37` 的可用性误拒也要说明；不能用未覆盖或改判隐藏 ROADMAP SC-1 的文字差异。[VERIFIED: `.planning/research/spike-02-sqlparser-l2.md:158-180`; `06-CONTEXT.md` D-06]
- 自检用到的受限 Trino 配置必须在计划中明确文件所有权和启动命令。现有 483 容器仅挂载 catalog 配置、无 ACL。[VERIFIED: `ci/query-services.sh:27-46`; `06-SPIKE-483.md`]

## Sources

- Accepted ADR-0016/0017/0018/0019、`06-CONTEXT.md`、`06-ENG-REVIEW.md`、`06-SPIKE-483.md`，2026-10-26 本地阅读。[VERIFIED]
- `sqlparser` 0.63.0 本机 registry 源码 `src/ast/query.rs` 与项目 `Cargo.toml`/`Cargo.lock`，`cargo info`/`cargo search`/package-legitimacy，2026-10-26。[VERIFIED]
- Trino 483 官方文档 `https://trino.io/docs/483/security/file-system-access-control.html` 和 `https://trino.io/docs/483/connector/system.html`，2026-10-26 抓取。[CITED]
- 当前 crate 与 `.github/workflows/ci.yml` 逐文件读取，2026-10-26。[VERIFIED]

**Confidence breakdown:** 已有 Rust 接口 HIGH；Trino 483 文档与 29 条探针 MEDIUM；真实受限 ACL 与 75 条未覆盖语料仍待执行。研究有效至实施时再次核对锁定版本与 CI 目标。
