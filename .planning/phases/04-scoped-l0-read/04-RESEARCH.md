# Phase 4: Scoped L0 Read - Research

**Researched:** 2026-10-22
**Domain:** Rust 编排的 scoped ObjectSet 读取、Trino 单表查询、edit store patch 叠加
**Confidence:** HIGH（仓库 API 与锁定语义），MEDIUM（查询性能与 connector 下推）

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** ObjectSet AST 是查询入口与 wire 格式；生产叠加只在 Rust 实现。Trino 查询单表且不超过 2 fragment，编译器强制；不采用 Trino 内叠加主路径。
- **D-02:** pk 读取先读 PG edit，按需通过 Trino `LIMIT 2` 查 source；列表并行取得 source 与 edit 候选，按 pk 至多 256 一批回查、合并、去重并补取到页满或 source 耗尽。
- **D-03:** scope 行谓词对原列过滤，列遮蔽在投影边界；合并后的行用同一 CEL 语义在 Rust 求值。多 role 的行列授权遵守 ADR-0018，不能通过过滤、排序、聚合或错误反推隐藏列。
- **D-04:** count / sum / avg 分解聚合，min / max 通过 TopN 补取，distinct 成本过高时明确报错；connector 能力影响下推选择。
- **D-05:** FK link 由合并后的对象决定；M:N 的 source join 表与 PG link edit 合并。String 搜索用 token 匹配，不提供相关度排序。
- **D-06:** keyset 游标按排序键与 pk 定位，`has_more` 判定结束；跨批次非快照一致。读结果带不透明 HMAC `_version`，行数与字节双上限超出时返回 `truncated`。
- **D-07:** Rust 叠加对照内存参考模型做 proptest；SQLite 替身与 Trino 对照固定单表语义语料。安静或隔离集群的延迟预算按 ADR-0020 验收。

### the agent's Discretion

模块拆分、内部 AST 类型、批量大小以下的补取调度和测试文件布局由 planner 依据现有代码确定，不改变上述外部语义。

### Deferred Ideas (OUT OF SCOPE)

快照一致分页与相关度排序留给后续 milestone；REST、MCP、Action、L2 按 ROADMAP 的后续 phase 处理。
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|---|---|---|
| QUERY-01 | 按 pk 读取、属性子集 | edit 只读 API、source `LIMIT 2`、统一叠加与遮蔽 |
| QUERY-02 | WhereClause 全部运算符 | 类型化 ObjectSet AST、scoped 谓词编译、Rust 复核 |
| QUERY-03 | String token 搜索 | 单表匹配与合并后复核，无相关度排序 |
| QUERY-04 | 排序、keyset 游标 | 复合键、补取循环、`has_more` |
| QUERY-05 | searchAround 与关联对象 | FK 取合并值，M:N source/edit link 合并 |
| QUERY-06 | union / intersect / subtract | scoped set 的 pk 去重与同型约束 |
| QUERY-07 | 聚合与 groupBy | 分解修正、TopN、distinct 成本边界 |
| QUERY-08 | 参考模型 proptest、SQLite/Trino 单表语义 | 本地属性测试 + CI 固定语料 |
| QUERY-09 | HMAC `_version`、物理名隔离、双上限 | 统一结果封装与错误映射 |
| QUERY-10 | 查询形态与 p95 预算 | AST 守卫、EXPLAIN、隔离集群测量 |
</phase_requirements>

## Summary

Phase 4 应以 `ontoforge-query` 为 ObjectSet AST、查询编排与唯一生产叠加的所有者；`ontoforge-storage` 增加只读 edit/link 批量访问，`ontoforge-engine` 扩展现有类型化单表 printer。依赖方向是 `query → storage, auth, engine`，绝不能恢复 `storage → query`。`ontoforge-query/src/lib.rs` 的 “compilation to sea-query” 是 Phase 1 占位注释；ADR-0020 已明确取代该方案。[VERIFIED: .planning/adr/0020-query-execution-rust-orchestrated-overlay.md:1-6] [VERIFIED: .planning/adr/0023-phase3-crate-dependency-direction.md:16-29] [VERIFIED: crates/ontoforge-query/src/lib.rs:1-4]

第一批计划应先固定 typed contract、纯 Rust overlay/reference model、只读 storage API，再让 pk/list 经过 scope，最后扩展集合、link 和聚合。难点是 edited row 能改变过滤、排序、scope 与 link 的结果；对 source 预过滤的候选，必须与 edit 候选补取、按 pk 回查和合并后重算共同闭环。短页不表示结束；engine 的 `truncated` 也不能当作 source 耗尽。[VERIFIED: .planning/adr/0020-query-execution-rust-orchestrated-overlay.md:11-22] [VERIFIED: crates/ontoforge-engine/src/lib.rs:77-83]

**Primary recommendation:** 按“纯语义核心 → storage 只读出口 → 单表 SQL 编译 → scoped list/pk → link/set/aggregate → 差分、越权、性能验证”拆任务，每个查询路径共用同一 Rust 合并与授权投影。[VERIFIED: .planning/adr/0020-query-execution-rust-orchestrated-overlay.md:11-32]

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|---|---|---|---|
| ObjectSet AST、overlay、set、游标、聚合修正 | Rust query crate | engine/storage | 一份生产合并语义，编排两个读取边界 [VERIFIED: .planning/adr/0020-query-execution-rust-orchestrated-overlay.md:11-32] |
| scope 合并与 Rust 行求值 | auth crate | query crate | `TypeScope::evaluate/lower` 已提供两侧入口 [VERIFIED: crates/ontoforge-auth/src/scope/merge.rs:101-130] |
| source 单表 SQL 与执行 | engine crate | Trino/SQLite | `Engine::execute` 只接受 `PrintedSql` [VERIFIED: crates/ontoforge-engine/src/lib.rs:92-111] |
| edit/link 读取 | storage crate | PostgreSQL/SQLite | `EditStore` 拥有 DB、物理 schema 和解码 [VERIFIED: crates/ontoforge-storage/src/store.rs:173-244] |
| HMAC key、调用者与定义快照 | server integration（后续） | query crate | 本 phase 需提供不依赖 REST/MCP 的可注入上下文 [ASSUMED] |

## Standard Stack

| Component | Repository version / API | Prescriptive use |
|---|---|---|
| Rust | `rust-version = "1.98"` | 沿用 workspace edition 2024；`cargo ... --locked` [VERIFIED: Cargo.toml:4-10] |
| `ontoforge-query` | 当前仅 `ontoforge-schema` 依赖 | 在此实现 AST 与 orchestration，按 ADR-0023 补 storage/auth/engine 依赖 [VERIFIED: crates/ontoforge-query/Cargo.toml:1-15] |
| `ontoforge-engine` | `Engine`, `QueryContext`, `QueryOutput`, `printer::ast::Select` | 扩展现有 typed AST，不另造 SQL 字符串出口 [VERIFIED: crates/ontoforge-engine/src/lib.rs:53-111] [VERIFIED: crates/ontoforge-engine/src/printer/ast.rs:14-115] |
| `ontoforge-storage` | `EditStore`, `ActiveDefinition`, `EditRow`, `LinkRow`, `PropertyValue` | 增加只读批量方法并复用内部 `read_expr`、`decode_object` [VERIFIED: crates/ontoforge-storage/src/store.rs:161-244] [VERIFIED: crates/ontoforge-storage/src/state.rs:35-57] |
| `ontoforge-auth` / `ontoforge-expr` | `PrincipalScope`, `TypeScope`, `RowValues` | Rust 合并后 scope 求值；SQL 下推由 `TypeScope::lower` 起步 [VERIFIED: crates/ontoforge-auth/src/scope/compile.rs:59-70] [VERIFIED: crates/ontoforge-auth/src/scope/merge.rs:101-130] |
| `proptest` | workspace `1.12.0` | overlay/reference model、游标及授权不变量 [VERIFIED: Cargo.toml:39-39] |

本研究不建议新增第三方 package。HMAC 实现需在计划时核对现有 crypto 栈与 key 管理接口，使用已审计的实现，禁止自写 MAC 算法。[ASSUMED]

### Source-of-truth literals

以下逐字摘录本会话已打开的定义，供 planner 使用离散值时核对：

```rust
// crates/ontoforge-schema/src/ir/binding.rs:21-27
pub enum Backing {
    Source,
    EditOnly,
}

// crates/ontoforge-storage/src/state.rs:39-52
pub struct EditRow {
    pub set: BTreeMap<PropertyName, PropertyValue>,
    pub deleted: bool,
    pub released: bool,
    pub version: i64,
}
pub struct LinkRow {
    pub deleted: bool,
    pub version: i64,
}

// crates/ontoforge-engine/src/lib.rs:77-83
pub struct QueryOutput {
    pub columns: Vec<Column>,
    pub rows: Vec<Vec<Value>>,
    pub truncated: bool,
}

// crates/ontoforge-engine/src/printer/ast.rs:14-23
pub struct Select {
    pub projection: Vec<SelectItem>,
    pub from: Option<TableRef>,
    pub filter: Option<Expr>,
    pub group_by: Vec<Expr>,
    pub having: Option<Expr>,
    pub order_by: Vec<OrderItem>,
    pub limit: Option<u64>,
}
```

`Expr` 当前变体逐字为 `Column`, `Literal`, `CountStar`, `Aggregate`, `Compare`, `InList`, `IsNull`, `StringMatch`, `And`, `Or`, `Not`；这组封闭枚举没有 CASE 变体。[VERIFIED: crates/ontoforge-engine/src/printer/ast.rs:32-66]

## Architecture Patterns

### Data Flow

```text
ObjectSet + principal + definition/profile
  → validate names/types/scope and compile bounded single-table plan
  → source Engine::execute(PrintedSql) ─┐
  → storage read edit candidates ────────┤ (parallel for list)
                                       ↓
                pk backfill (≤256) → Rust overlay → TypeScope::evaluate
                                       ↓
                      scoped filter/order/set/link/aggregate
                                       ↓
                keyset page + has_more + HMAC _version + limits
```

### Component Responsibilities

| Component | Plan action |
|---|---|
| `crates/ontoforge-query` | 定义 public AST/result/error、纯 overlay/reference model、编排、游标、聚合修正 |
| `crates/ontoforge-storage` | 无副作用的 edit/link get 与 batch scan，沿用 backend 解码和 SQL exit |
| `crates/ontoforge-engine/src/printer` | 增加遮蔽投影所需 typed expression；保留 `Select` 单 `from` 形态 |
| `crates/ontoforge-auth` | 复用 `TypeScope`，仅在有确切语义缺口时扩展公开 API |
| `crates/ontoforge-query/tests` | 属性测试、SQLite 单表 corpus、受环境变量门控的 Trino/PG integration |

### Pattern 1: 以合并后的行作为最终判定对象

`EditRow` 的 `set` 是属性 patch；显式设为 NULL 与未设置不同。`deleted` 压制 source，`released` 清空 patch 并回到 source，`version` 单调递增。主键列不带 `__set`。只在 Rust 合并后做 scope、WhereClause、token、排序与关联判定的最终复核，才可覆盖 edit 将一行移入或移出结果的情况。[VERIFIED: crates/ontoforge-storage/src/state.rs:35-123] [VERIFIED: .planning/adr/0024-edit-store-physical-format.md:11-21]

### Pattern 2: scope 下推与最终遮蔽成对实现

`TypeScope` 的 `row` 是 role 行谓词的 OR，`columns[x]` 是授予 x 的 role 行谓词的 OR；`evaluate` 用合并后的行计算可见性。用户过滤 `x op v` 在 source 原列上按 `(grant(x) AND predicate(x))` 下推，仅在适用的 null 拒绝谓词上直接化简；`isNull`/`not`/复合表达式须按三值语义处理。输出列只在投影边界遮蔽。部分可见列的排序与 groupBy 不可用未遮蔽原值作最终序，需用 scope 后可见值或明确拒绝不可安全执行的形态。[VERIFIED: crates/ontoforge-auth/src/scope/merge.rs:1-19] [VERIFIED: .planning/research/spike-03b-scoped-overlay.md:116-123] [VERIFIED: .planning/adr/0025-row-predicate-semantics.md:40-79]

### Pattern 3: 限界查询与补取

按 pk 先读 edit；只有 patch 缺少需要的 source 列或 release 等情形才查 Trino，source pk 查询用 `LIMIT 2` 检测歧义。列表 source 与 edit 候选并行，≤256 个 pk 回查，按 stable sort key + pk 合并去重，继续补取直至页满且已确认下一条、或 source 耗尽。每轮保持进度且设置请求级工作预算，避免高 edit 覆盖率下无限补取；预算耗尽时返回显式成本/截断信号而非伪称完整结果。[VERIFIED: .planning/adr/0020-query-execution-rust-orchestrated-overlay.md:11-22] [ASSUMED: 工作预算的具体错误合同未锁定]

### Pattern 4: 单表 SQL AST 守卫

现有 `Select` 只有一个可选 `from: Option<TableRef>`，`PrintedSql` 只能由 printer 构造；可用 AST 结构禁止 JOIN/CTE/UNION 进入 L0 Trino 路径。现有 `Expr` 没有 CASE，需小范围扩展 typed projection 才能表达列遮蔽，不能拼接原始 SQL。`≤2 fragment` 取决于 Trino 实际执行计划；编译器可强制 AST 形态，另在 Trino 固定语料中 `EXPLAIN (TYPE DISTRIBUTED)` 验证 fragment 上限。静态 AST 检查不能单独证明运行时 fragment 数。[VERIFIED: crates/ontoforge-engine/src/printer/ast.rs:14-115] [VERIFIED: crates/ontoforge-engine/src/printer/mod.rs:15-51] [VERIFIED: .planning/research/spike-03b-scoped-overlay.md:105-108]

## Don't Hand-Roll

| Problem | Use existing boundary | Why |
|---|---|---|
| SQL escaping/dialect | `printer::ast` + `print_select` | `PrintedSql` 构造受限；Trino/SQLite 已有文字值与字符串匹配转义 [VERIFIED: crates/ontoforge-engine/src/printer/mod.rs:15-51] |
| role 合并、CEL/SQL 三值逻辑 | `TypeScope` + `Pred` evaluator/lowerer | 自写 CEL 二值求值会在 NULL 时与 SQL 分歧 [VERIFIED: .planning/adr/0025-row-predicate-semantics.md:16-25] |
| edit patch 状态转换 | `EditRow`/`LinkRow` 与 storage 解码 | `__set` 区分“设 NULL”与“未设” [VERIFIED: crates/ontoforge-storage/src/physical.rs:45-65] |
| 十进制、timestamp 规范化 | 现有 `PropertyValue` 与 engine 比较组件 | 文本字典序与数值/时间序不等价 [VERIFIED: crates/ontoforge-storage/src/value.rs:25-45] [VERIFIED: crates/ontoforge-engine/src/sqlite/mod.rs:211-250] |
| HMAC 密码学原语 | 经校验的现有/正式 crypto crate | 自写 MAC 无必要且易出错 [ASSUMED] |

## Common Pitfalls

1. **旧占位文案误导。** `ontoforge-query` 的 description 和注释仍说 sea-query；ADR-0020 明确否决 sea-query + 递归 CTE 作为 L0 编译目标。计划需先修正 crate 合同。[VERIFIED: crates/ontoforge-query/Cargo.toml:1-3] [VERIFIED: .planning/adr/0020-query-execution-rust-orchestrated-overlay.md:25-29]
2. **写路径读函数冒充查询 API。** `commit::load_object/load_link` 会先插入 version 0 行并加更新锁；不能用于纯读。`EditStore::db` 与 `sql_exit` 是 crate-private，Phase 4 要由 storage 暴露新只读 API。[VERIFIED: crates/ontoforge-storage/src/commit.rs:771-819] [VERIFIED: crates/ontoforge-storage/src/store.rs:173-179]
3. **遮蔽列泄露。** 只遮蔽返回字段却在 WHERE、ORDER BY、groupBy、count/distinct 或错误中使用未授权原值，会造成跨 role 列×行泄露。空/缺失主体属性按 ADR-0025 的 Unknown 处理，不能沿用 ADR-0018 已被取代的“相关谓词为 false”文字。[VERIFIED: .planning/adr/0018-authz-scope-and-sql-tiers.md:13-21] [VERIFIED: .planning/adr/0025-row-predicate-semantics.md:50-79]
4. **短页、engine 截断与重复 pk。** 短页不足以证明已结束；`QueryOutput.truncated` 表示 engine 自身行/字节上限命中，必须继续或显式截断。pk `LIMIT 2` 多行报 `AmbiguousObject`，列表去重并告警。[VERIFIED: .planning/adr/0019-mcp-toolsets-and-l2-tools.md:11-13] [VERIFIED: crates/ontoforge-engine/src/lib.rs:77-83] [VERIFIED: .planning/adr/0015-source-binding-and-type-contract.md:22-22]
5. **旧 spike 的物理 patch 形状。** spike-03b 示例用 `set_mask int`；正式 edit store 已采用每属性 `<col>__set`，按当前 `PhysicalSchema` 读取，不能复制 spike SQL。[VERIFIED: .planning/research/spike-03b-scoped-overlay.md:52-56] [VERIFIED: .planning/adr/0024-edit-store-physical-format.md:11-21]
6. **connector 能力与基准环境。** SQL Server connector 上 `sum(integer)` 未下推；Iceberg 未分区 pk 扫描与 JDBC 索引路径成本不同。按能力表选聚合策略，基准记录 connector、数据规模、负载与 p95 样本，繁忙共享集群只监测。[VERIFIED: .planning/research/spike-03b-scoped-overlay.md:20-25] [VERIFIED: .planning/adr/0020-query-execution-rust-orchestrated-overlay.md:20-22]
7. **浮点特殊值。** ADR-0025 明说 NaN/Infinity 不在 Rust/SQL 等价保证内；进入 L0 数值排序、过滤与聚合前须决定拒绝或规范化并给出测试。[VERIFIED: .planning/adr/0025-row-predicate-semantics.md:101-103]

## Code Examples

下面是已存在 API 的调用形状，不表示完整查询实现：

```rust
// Source: crates/ontoforge-auth/src/scope/merge.rs:101-130
let visibility = type_scope.evaluate(&merged_row, attrs);
if !visibility.visible {
    return None;
}
// Only project fields in visibility.columns after all query predicates are resolved.
```

```rust
// Source: crates/ontoforge-engine/src/lib.rs:92-111 and printer/mod.rs:48-51
let sql = ontoforge_engine::printer::print_select(&select, engine.dialect())?;
let output = engine.execute(&sql, &context).await?;
```

## Validation Architecture

### Test Framework

| Property | Value |
|---|---|
| Framework | Rust `cargo test`；`proptest` workspace `1.12.0` [VERIFIED: Cargo.toml:39-39] |
| Config | Cargo workspace；Phase 4 测试文件尚不存在，作为 Wave 0 [VERIFIED: crates/ontoforge-query/src/lib.rs:1-15] |
| Quick | `cargo test -p ontoforge-query --locked` |
| Full | `cargo test --workspace --locked` |

以下文件与 test target 名称是建议的 Wave 0 命名，尚非现有路径。[ASSUMED]

### Phase Requirements to Tests

| ID | Behavior / test type | Local automated command | Real service boundary |
|---|---|---|---|
| QUERY-01 | pk、patch/tombstone/release、`LIMIT 2` 重复 pk；integration | `cargo test -p ontoforge-query --test sqlite_query --locked pk -- --nocapture` | `cargo test -p ontoforge-query --test trino_query_it --locked -- --ignored pk`，同时需 PG edit store |
| QUERY-02 | 全部 WhereClause、null/role 改写；unit + SQLite differential | `cargo test -p ontoforge-query --test sqlite_query --locked where_clause` | Trino 固定单表语义 corpus |
| QUERY-03 | token 边界、大小写、通配符、edit 移入/移出；integration | `cargo test -p ontoforge-query --test sqlite_query --locked token_search` | Trino token SQL 固定 corpus |
| QUERY-04 | 稳定复合键、null sort、短页、补取、`has_more`；property + integration | `cargo test -p ontoforge-query --test overlay_prop --locked pagination` | Trino/PG 同时运行的 edit 高覆盖率 corpus |
| QUERY-05 | FK 合并值、M:N source join 表与 link edit；integration | `cargo test -p ontoforge-query --test sqlite_query --locked search_around` | Trino 单表 join-binding 读取 + PG link edit |
| QUERY-06 | union/intersect/subtract 在 scope 内按 pk 去重；property | `cargo test -p ontoforge-query --test overlay_prop --locked set_ops` | 固定 Trino/PG 端到端反例 |
| QUERY-07 | count/sum/avg 分解，min/max TopN，distinct 成本，groupBy 遮蔽；property + integration | `cargo test -p ontoforge-query --test overlay_prop --locked aggregate` | Trino/PG 聚合固定语料、SQL Server connector 能力私有 fixture |
| QUERY-08 | Rust reference model 与生产 overlay 差分、SQLite/Trino 单表等价；property + integration | `cargo test -p ontoforge-query --test overlay_prop --locked` | `cargo test -p ontoforge-query --test trino_query_it --locked -- --ignored` |
| QUERY-09 | `_version` 篡改/轮换、物理名屏蔽、双上限；unit + integration | `cargo test -p ontoforge-query --test security --locked` | PG/Trino 原始失败转 L0 错误的 corpus |
| QUERY-10 | 单表 AST 与 2 fragment、p95；unit + environment gate | `cargo test -p ontoforge-query --test sql_shape --locked` | `trino_query_it` 中 EXPLAIN 断言；安静/隔离集群单独采样并保留原始数据 |

### Sampling Rate and Wave 0

- 每个 task：`cargo test -p ontoforge-query --locked`；涉及 storage/engine 的 task 还跑对应 crate 测试。
- 每个 wave：`cargo fmt --all --check`、`cargo clippy --workspace --all-targets --locked -- -D warnings`、`cargo test --workspace --locked`。
- Phase gate：上述全量绿色、Linux Trino/PG ignored tests 实际运行、隔离性能报告满足 QUERY-10。
- Wave 0：建立 `overlay_prop.rs`、`sqlite_query.rs`、`trino_query_it.rs`、`security.rs`、`sql_shape.rs`；Trino test 的缺失 `ONTOFORGE_TRINO_URL`/PG URL 应失败，不能静默跳过；将 query ignored test 加入 CI integration 及计数守卫。[VERIFIED: crates/ontoforge-engine/tests/trino_it.rs:1-9] [VERIFIED: .github/workflows/ci.yml:110-149]

Windows 本地 SQLite 覆盖纯 Rust 合并语义与单表替身语义，不能证明 Trino connector 下推、fragment 数或 PG 读取语义。当前 CI 已在 Windows/Linux 跑全量测试；Linux integration 启动 PostgreSQL 17.11、Trino 483，测试以 `#[ignore]` 和 `ONTOFORGE_TRINO_URL` 门控。[VERIFIED: .github/workflows/ci.yml:8-43] [VERIFIED: .github/workflows/ci.yml:45-130] [VERIFIED: crates/ontoforge-engine/tests/trino_it.rs:1-9]

性能验收只在安静/隔离集群统计 p95，分别记录 pk <500ms、列表 <1s、聚合 <1.5s、已编辑对象 PG 直读 <50ms；按请求端到端耗时采样，并附 Trino plan/fragment、数据分布、edit 覆盖率及负载条件。spike-03b 的 n=7 不能充当 p95 证明。[VERIFIED: .planning/adr/0020-query-execution-rust-orchestrated-overlay.md:20-22] [VERIFIED: .planning/research/spike-03b-scoped-overlay.md:26-27]

## Security Domain

| ASVS category | Applicability | Planning control |
|---|---|---|
| V2 Authentication | 上游主体输入 | 查询接口使用已认证 `Principal`，不信任 wire 里的 principal 属性 [VERIFIED: crates/ontoforge-auth/src/lib.rs:13-16] |
| V3 Session Management | 无新会话 | 请求固定 definition/principal scope 快照 [VERIFIED: .planning/adr/0018-authz-scope-and-sql-tiers.md:26-26] |
| V4 Access Control | 核心 | 每条查询路径用 `TypeScope`，行判定、列授权及越权错误统一 [VERIFIED: crates/ontoforge-auth/src/scope/merge.rs:1-19] |
| V5 Input Validation | 核心 | ObjectSet 名称/类型/深度/limit/cursor 先校验；SQL 仅 typed printer [VERIFIED: crates/ontoforge-engine/src/printer/mod.rs:1-51] [ASSUMED: 具体限值未锁定] |
| V6 Cryptography | `_version` | 按 ADR-0018 的 HMAC 输入定义实现，密钥轮换使旧 token 冲突 [VERIFIED: .planning/adr/0018-authz-scope-and-sql-tiers.md:19-20] |

威胁验证至少覆盖换 pk 探测、role 行列交叉、filter/sort/aggregate 反推、异常信息泄露、游标篡改与 token 枚举。ADR-0018 已要求其中的越权 conformance；游标完整性须由计划明确。[VERIFIED: .planning/adr/0018-authz-scope-and-sql-tiers.md:35-35] [ASSUMED: 游标保护方式未锁定]

## Environment Availability

| Dependency | Available here | Evidence / fallback |
|---|---|---|
| Rust/Cargo 1.98 | yes | 本机会话 `rustc --version`、`cargo --version` 均为 1.98.0 [VERIFIED: local CLI probe 2026-10-22] |
| SQLite stand-in | repo implementation | Windows 本地 gate 可运行，无 Docker [VERIFIED: crates/ontoforge-engine/src/sqlite/mod.rs:37-70] |
| PostgreSQL + Trino | CI configured；本机未探测服务 | Linux integration job 启动服务；私有延迟 fixture 另行配置 [VERIFIED: .github/workflows/ci.yml:45-103] |
| GSD graphify / CodeGraph | 不可用 | `graphify status` 显示 disabled；仓库无 `.codegraph/` [VERIFIED: local CLI/filesystem probe 2026-10-22] |

## Decision Closure and Execution Gates

### Q1: distinct 与 work budget

**已锁定语义。** ADR-0020 要求高成本 `distinct` 明确报错，列表补取须到页满或 source 耗尽；ADR-0019 的 `truncated=true` 指最终行数/字节输出上限与继续方式，不能把未完成的计算伪装成完整结果。[VERIFIED: .planning/adr/0020-query-execution-rust-orchestrated-overlay.md:15-16] [VERIFIED: .planning/adr/0019-mcp-toolsets-and-l2-tools.md:11-11]

**PLAN 中的可执行合同。** 04-04 将列表预算错误写成 `QueryError::WorkBudgetExceeded`：backend call、raw candidate、backfill key、decoded byte 计入同一个 `WorkBudget`；耗尽时无 partial rows，且不置 `truncated`。`has_more` 只由第 `page_size + 1` 条经合并、scope 与复核后合格的行确定。04-06 将 aggregate 成本错误写成 `AggregateCostExceeded`：完整 exact buckets 或无 data typed error；group/bucket/value 数量、decoded bytes 与 distinct set memory 均计入预算，成功聚合没有 `truncated`。[VERIFIED: .planning/phases/04-scoped-l0-read/04-04-PLAN.md:133-153] [VERIFIED: .planning/phases/04-scoped-l0-read/04-06-PLAN.md:169-197]

**必须在 04-04 Task 2 与 04-06 Task 3 开始前锁定的参数。** `ExecutionLimits` 至少含正整数 `max_backend_calls`、`max_raw_candidates`、`max_backfill_keys`、`max_decoded_bytes`；`AggregateLimits` 至少含正整数 `max_groups`、`max_buckets`、`max_distinct_values`、`max_distinct_memory_bytes`，并消耗同一个请求级 `WorkBudget`。这些值由可信构造参数在请求开始时固定，不由 ObjectSet wire 调高；逐维在 B、B+1 和两个维度同时临界时测边界。测试 fixture 可用小值 `B=2`，它只验证边界机制，不是生产默认值。[ASSUMED: 参数名、测试 fixture 与生产默认值尚未获 ADR 锁定]

现有 `Engine::QueryContext` 的 `DEFAULT_MAX_ROWS = 10_000`、`DEFAULT_MAX_BYTES = 16 * 1024 * 1024`、`DEFAULT_QUERY_TIMEOUT_MS = 30_000` 是**单条 engine 查询**限额，不是整个列表/aggregate 请求的总预算；不能据此推断 production `ExecutionLimits`。原文：`pub const DEFAULT_MAX_ROWS: usize = 10_000;`、`pub const DEFAULT_MAX_BYTES: usize = 16 * 1024 * 1024;`、`pub const DEFAULT_QUERY_TIMEOUT_MS: u64 = 30_000;`。[VERIFIED: crates/ontoforge-engine/src/lib.rs:23-30] 生产默认值需在执行前由 planner 写入 `04-04-PLAN.md` 和 `04-06-PLAN.md`，并在第一批 RED tests 中固定；缺值、零值或溢出必须拒绝构造，不能等运行时无限扫描。[ASSUMED: 默认值决策属于 planner 的执行准备]

### Q2: 部分授权列

**已锁定语义。** `TypeScope.row` 是各 role 行谓词的 OR；`TypeScope.columns[x]` 是授予属性 x 的 role 谓词的 OR。合并后的行决定可见性，最终 filter/search/sort/group/measure 只消费该行 scope 后的值；source 下推使用原列时，必须附授予该列的 role guard，并在 Rust 重新求值。`isNull`、`not` 与多列复合式不能套用只对 null-rejecting 比较成立的简单改写。[VERIFIED: crates/ontoforge-auth/src/scope/merge.rs:1-19] [VERIFIED: .planning/research/spike-03b-scoped-overlay.md:116-123] [VERIFIED: .planning/phases/04-scoped-l0-read/04-04-PLAN.md:95-118]

**PLAN 中的外部结果合同。** 部分授权的排序键用合并后已遮蔽值排序，显式 null policy，最后按 canonical pk 打破平局；groupBy 的隐藏值与逻辑 NULL 在 outward response 中是同一个 masked-null key，measure/distinct 只读取可见值。改变任何未授权原值不得改变 membership、顺序、group、measure 或错误外形。04-04 与 04-06 已安排独立 reference model、多 role mutation pairs 和属性测试；无需另开用户决策。[VERIFIED: .planning/phases/04-scoped-l0-read/04-04-PLAN.md:95-118] [VERIFIED: .planning/phases/04-scoped-l0-read/04-06-PLAN.md:169-197]

### Q3: HMAC key 与 cursor 边界

**已锁定语义。** `_version` 采用 `HMAC(server_key, principal_id ‖ type ‖ pk ‖ 规范化(edit 行 + source 行))`；轮换 key 后旧 token 冲突。keyset 是非快照游标，`has_more` 决定结束，游标预留 `snapshot_id`。[VERIFIED: .planning/adr/0018-authz-scope-and-sql-tiers.md:16-16] [VERIFIED: .planning/adr/0019-mcp-toolsets-and-l2-tools.md:11-11] [VERIFIED: .planning/adr/0020-query-execution-rust-orchestrated-overlay.md:19-19]

**PLAN 中的 Phase 4 合同。** 04-01 从可信 `TokenCodec` 构造参数注入 key，使用现有 `jsonwebtoken::crypto::{sign,verify}` 与 HS256；`_version` 只输出 base64url MAC，签名输入用长度前缀规范编码，并绑定 def_version、完整 source/edit 状态。04-04 的 cursor 使用同一 key 原语，versioned payload 绑定 canonical principal id、effective scope fingerprint、def_version、logical type、完整排序描述符与最后的 typed sort values + pk；任何篡改、跨上下文重放或 key 轮换都返回同形 `InvalidCursor`。payload 不含物理名，executor 只接收通过验证的 `CursorPosition`。本 phase 的 key 生命周期边界是“宿主注入、query 进程内使用、轮换使旧 token/cursor 失效”；key 的部署存储/分发属于后续 server 集成，不在 L0 wire 中新增字段。[VERIFIED: .planning/phases/04-scoped-l0-read/04-01-PLAN.md:108-113] [VERIFIED: .planning/phases/04-scoped-l0-read/04-04-PLAN.md:168-193]

### Q4: Trino fragment 与 p95 证据

**已锁定语义。** 编译器限制 source 查询为单表 typed `Select`，每条真实 Trino 查询须 `≤2 fragment`；安静或隔离集群四条 p95 阈值按 ADR-0020/QUERY-10 验收。AST 单表条件只能证明没有 JOIN/CTE/UNION，不能静态证明 Trino optimizer 的实际 fragment 数。[VERIFIED: crates/ontoforge-engine/src/printer/ast.rs:14-23] [VERIFIED: .planning/adr/0020-query-execution-rust-orchestrated-overlay.md:13-22]

**PLAN 中的验收门禁。** 04-02 增加只接受 typed `Select` 的 distributed EXPLAIN printer；04-07 对每种 source-producing shape 在真实 Trino 执行 `EXPLAIN (TYPE DISTRIBUTED)`，严格解析完整 fragment header 并按 id 去重，0、重复、格式不符或 >2 即失败。Linux CI 必须实际运行 `trino_query_it` 与 PG edit fixture，不能只编译/列出 ignored tests。隔离 benchmark 四条路径各先 5 次 warmup，再至少 40 次计入失败/超时的 measured samples，nearest-rank p95 与 raw timings/plan artifact/commit/run URL 写入 `04-PERFORMANCE.md`；shared CI timing 仅 smoke。只有这些证据齐全才能关闭 QUERY-10，不需用户再裁决。[VERIFIED: .planning/phases/04-scoped-l0-read/04-02-PLAN.md:125-150] [VERIFIED: .planning/phases/04-scoped-l0-read/04-07-PLAN.md:95-127] [VERIFIED: .planning/phases/04-scoped-l0-read/04-07-PLAN.md:149-179]

## Assumptions Log

| # | Claim | Risk if wrong |
|---|---|---|
| A1 | `ExecutionLimits` 与 `AggregateLimits` 的字段名和生产默认数值由 planner 在执行前锁定；本文件的 `B=2` 仅为边界测试 fixture | 若不锁定，成本超限无法形成稳定的可测合同 |
| A2 | Phase 4 由 query 宿主向 `TokenCodec` 注入 key，正式 server 的密钥部署与分发留待集成 | 若宿主不能安全注入，需调整构造 API |
| A3 | 可复用现有 `jsonwebtoken` crypto 后端，无新增 package | 如现有 API 不适用，新增 package 须先做官方来源与 legitimacy 审计 |

## Sources

- HIGH：`.planning/phases/04-scoped-l0-read/04-CONTEXT.md`；`.planning/adr/0015`、`0018`、`0020`、`0023`、`0024`、`0025`；本研究所引述的 crate 源文件与 CI 配置（本会话逐份打开）。
- MEDIUM：`.planning/research/spike-03-trino-overlay-latency.md`、`spike-03b-scoped-overlay.md`（真实 Trino 测试，但负载、connector 和 n=7 限制外推）。
- LOW：Assumptions Log A1-A3，等待 planner 在执行前核定。

**Valid until:** 2026-11-21；任何上游 ADR、crate API 或 Trino/connector 版本变化后重新检查。
