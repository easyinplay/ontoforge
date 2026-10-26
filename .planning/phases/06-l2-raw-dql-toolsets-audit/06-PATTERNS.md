# Phase 6: L2 Raw DQL, Toolsets & Audit - Pattern Map

**Mapped:** 2026-10-26
**Scope:** PLAN 候选路径，尚未写业务代码；planner 可在同一 crate 边界内调整新文件名。所有 analog 均由 `git ls-files -- <path>` 确认是已跟踪源码。

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
| --- | --- | --- | --- | --- |
| `crates/ontoforge-engine/src/printer/l2.rs` (new candidate) | utility, security gate | transform, request-response | `crates/ontoforge-engine/src/printer/mod.rs:17-32` | role-match |
| `crates/ontoforge-engine/src/printer/mod.rs` | utility, trusted constructor | transform | same file, `:17-80` | exact |
| `crates/ontoforge-engine/src/trino/mod.rs` | provider | request-response, streaming | same file, `:204-249`, `:345-384` | exact |
| `crates/ontoforge-engine/src/config.rs` | config | file-I/O, transform | same file, `:31-47`, `:133-157`, `:191-223` | exact |
| `crates/ontoforge-engine/src/lib.rs` | provider contract, model | request-response | same file, `:51-112` | exact |
| `crates/ontoforge-auth/src/scope/compile.rs` | authorization model | transform | same file, `:55-67`, `:445-484` | exact |
| `crates/ontoforge-query/src/l2.rs` (new candidate) | service, six library operations | request-response | `crates/ontoforge-query/src/lib.rs:226-285` and engine printer | role-match |
| `crates/ontoforge-query/src/lib.rs` | service, error model | request-response | same file, `:64-107`, `:226-285` | exact |
| `crates/ontoforge-query/src/execute.rs` | service, list/set/searchAround | request-response | same file, `:392-426` | exact |
| `crates/ontoforge-query/src/aggregate.rs` | service, error conversion | request-response | same file, `:112-140`, `:701-714`, `:872-895` | exact |
| `crates/ontoforge-storage/src/read_audit.rs` (new candidate) | store, batch sink | batch, CRUD | `crates/ontoforge-storage/src/scope_store.rs:637-668` | role-match, batching new |
| `crates/ontoforge-storage/src/lib.rs` and `src/store.rs` | store integration | CRUD | same files, `lib.rs:1-23`, `store.rs:175-220`, `:292-317` | exact |
| `crates/ontoforge-storage/migrations/postgres/0005_read_audit.sql` (new candidate) | migration | CRUD | `migrations/postgres/0003_scope.sql:29-45` | role-match |
| `crates/ontoforge-storage/migrations/sqlite/0005_read_audit.sql` (new candidate) | migration | CRUD | `migrations/sqlite/0003_scope.sql:29-49` | role-match |
| `crates/ontoforge-engine/tests/data/l2_corpus.*` (new candidate) | test fixture | file-I/O, transform | `crates/ontoforge-auth/tests/data/rowpred_corpus.json` | role-match; SQL 原文缺失 |
| `crates/ontoforge-engine/tests/l2_gate.rs` (new candidate) | test | request-response | `tests/sql_text_only_from_printer.rs:26-62`, `tests/trino_it.rs:26-79` | role-match |
| `crates/ontoforge-engine/tests/trino_l2_acl_it.rs` (new candidate) | integration test | request-response | `tests/trino_it.rs:1-79` | role-match; ACL probe new |
| `crates/ontoforge-engine/tests/server_config.rs` | test | transform | same file, `:20-55`, `:57-70` | exact |
| `crates/ontoforge-storage/tests/conformance/mod.rs` | shared test | CRUD, batch | same file, `:1-65`, `:2498-2558` | exact |
| `crates/ontoforge-storage/tests/conformance_{sqlite,pg}.rs` | backend test entry | CRUD | same files, SQLite `:1-8`, PG `:1-15` | exact |
| `crates/ontoforge-query/tests/security.rs` (modify or new audit-focused test) | test | request-response | `crates/ontoforge-query/src/lib.rs:226-285` | role-match |
| `ci/trino/l2-access-control.*` (new candidate) | test config | file-I/O | no tracked ACL configuration | no analog |
| `ci/query-services.sh` and `.github/workflows/ci.yml` | integration config | process orchestration | same files, script `:4-44`, CI `:69-103`, `:121-148` | exact |
| `crates/ontoforge/tests/cli_reload.rs`, `cli_scope.rs` | regression test | request-response | existing tests for the same CLI commands | exact |

`0005` 是现有最高迁移号 `0004_action.sql` 后的候选编号；planner 应核对并行修改后再定。当前 Phase 6 不应创建 server route 或 MCP handler。

## Pattern Assignments

### L2 SQL gate and wire path

**Analog:** `crates/ontoforge-engine/src/printer/mod.rs:17-32`, `:47-73`; `crates/ontoforge-engine/src/trino/mod.rs:204-249`。应用于新 `printer/l2.rs`、既有 printer、Trino 实现及 L2 查询库入口。

```rust
pub struct PrintedSql(String);

impl PrintedSql {
    fn new(sql: String) -> Self {
        Self(sql)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

```rust
let mut page = client::send(
    || {
        Ok(self
            .request(Method::POST, self.statement_url.clone(), ctx, true)?
            .body(sql.as_str().to_owned()))
    },
    true,
    max_page,
)
.await?;
```

`PrintedSql::new` 只能由 printer 模块树调用。先在同一窄入口完成字节上限、单条 AST、relation/CTE、allow-list、硬排除、外层 LIMIT、Display/Trino 兼容判定，再构造；不要开放 `From<String>`。`Engine::execute` 目前只收 `&PrintedSql` (`engine/src/lib.rs:91-99`)。`TrinoEngine::execute` 的 timeout、truncation 与取消路径在 `trino/mod.rs:350-384`，直接复用。现有源级守卫 `tests/sql_text_only_from_printer.rs:26-62` 会扫 printer 外 SQL 字面量，ACL 探针的固定 SQL 需在该边界内安排或显式审查守卫的覆盖方式。

### Authorization and fixed scope

**Analog:** `crates/ontoforge-auth/src/scope/compile.rs:55-67`, `:452-484`; `crates/ontoforge-query/src/lib.rs:226-251`。应用于 L2 库工具、授权过滤及 L0 审计 metadata。

```rust
pub struct PrincipalScope {
    pub roles: BTreeSet<RoleName>,
    pub types: BTreeMap<ObjectTypeName, TypeScope>,
    pub actions: BTreeSet<ActionTypeName>,
    pub sql_levels: BTreeSet<SqlLevel>,
    pub l2_catalogs: BTreeSet<String>,
    pub l2_schemas: BTreeSet<String>,
    pub toolsets: BTreeSet<Toolset>,
}
```

```rust
scope.sql_levels.extend(role.sql_levels.iter().copied());
scope.l2_catalogs.extend(role.l2.catalogs.iter().cloned());
scope.l2_schemas.extend(role.l2.schemas.iter().cloned());
```

`QueryService` 当前持有 `principal`、`scope`、`Arc<ActiveDefinition>`、`engine`、`store` (`query/src/lib.rs:226-234`)；请求固定同一份 definition/scope 供执行和审计。`auth/tests/scope_compile.rs:141-150` 已断言 `SqlLevel::Raw` 与 `Toolset::SqlRaw`，新用例应补 catalog/schema 授权与无角色拒绝。

### Public read gates and error propagation

**Analog:** `crates/ontoforge-query/src/lib.rs:64-107`, `:226-285`; `src/execute.rs:392-426`; `src/aggregate.rs:112-140`, `:701-714`, `:872-895`。应用于三个公开读取方法和六个 L2 方法。

```rust
pub async fn execute_list(&self, query: &ValidatedQuery) -> Result<ListPage, QueryError> {
    let mut budget = WorkBudget::new(self.limits);
    if matches!(query.request.set, crate::ast::ObjectSet::Union { .. }
        | crate::ast::ObjectSet::Intersect { .. }
        | crate::ast::ObjectSet::Subtract { .. }) {
        return self.execute_set_list(query, &mut budget).await;
    }
    // SearchAround and ordinary list continue through their respective internal paths.
    self.execute_list_internal(query, &mut budget, None).await
}
```

摘录基于 `execute.rs:398-415`，省略了 SearchAround 分支，不能直接粘贴替换生产代码。审计 permit 放在 `execute`、`execute_list`、`execute_aggregate` 的公开入口，内部 `execute_list_internal` 由 set、searchAround、aggregate 复用 (`aggregate.rs:701-714`)，不能重复审计。`read_pk_source` 是 Action 复用的内部辅助 (`lib.rs:369-377`)，不将它错误计成独立公开读。`QueryError::code` 统一错误码 (`lib.rs:90-106`)；新增 `AuditUnavailable` 时必须在 `AggregateError::from` 显式映射，当前通配分支会折叠为 `InvalidQuery` (`aggregate.rs:128-138`)。

### Dual-backend audit persistence

**Analog:** `crates/ontoforge-storage/src/store.rs:175-220`, `:292-317`; `src/scope_store.rs:637-668`; `migrations/{postgres,sqlite}/0003_scope.sql:29-49`。应用于新 `read_audit.rs`、`store.rs`、`lib.rs` 与两份迁移。

```rust
let insert = Query::insert()
    .into_table(Alias::new(AUDIT_TABLE))
    .columns([Alias::new("at"), Alias::new("actor"), Alias::new("action")])
    // Existing scope_store implementation adds other metadata and backend typed values.
    .to_owned();
let built = sql_exit::build(&insert, tx);
```

以上是 `scope_store.rs:643-665` 的缩短示意，不是可直接编译的完整调用。现有实现通过 `sql_exit::typed_text` 区分 PostgreSQL `timestamptz/jsonb` 与 SQLite TEXT。`EditStore::open` 的 SQLite 分支运行 `sqlx::migrate!("./migrations/sqlite")` (`store.rs:186-206`)，PG 在 system schema `search_path` 下运行 postgres 迁移 (`store.rs:292-317`)。新 `of_read_audit` 应归 system schema，不能套用 `of_admin_audit` 的 append-only UPDATE/DELETE 触发器，因为读审计须按保留期清理。SQL 哈希应复用已存在的 `sha2::{Digest, Sha256}` 使用方式 (`storage/src/import.rs:13-19`, `:549`)，不要拿用于 plan hash 的 `fnv1a64_hex` 代替。

### Shared conformance and external services

**Analog:** `crates/ontoforge-storage/tests/conformance/mod.rs:1-65`, `:2498-2558`; `tests/conformance_sqlite.rs:1-8`; `tests/conformance_pg.rs:1-15`; `crates/ontoforge-engine/tests/trino_it.rs:1-79`。

```rust
macro_rules! conformance_case {
    ($factory:path, $name:ident $(, $attr:meta)?) => {
        #[tokio::test(flavor = "multi_thread")]
        $(#[$attr])?
        async fn $name() {
            let harness = $factory().await;
            $crate::conformance::$name(&harness).await;
        }
    };
}
```

SQLite/PG 的同一审计 case 加进 `all_cases!`，PG target 用 `#[ignore]` 且缺 `ONTOFORGE_PG_URL` 时失败，不静默跳过。现有 Trino integration 使用 `ONTOFORGE_TRINO_URL` 缺失即 panic (`trino_it.rs:26-60`)，适合 ACL target 的服务前提模板。CI 已以 `--ignored --list` 统计真实发现数，0 条直接失败 (`.github/workflows/ci.yml:93-103`)；`ci/query-services.sh:39-44` 当前只挂载 catalog，必须另备独立 ACL 容器配置和真实正反探针，不能将已有无 ACL 集群当作通过证据。

## Shared Patterns

- **错误码和安全原因:** `QueryError::code` (`query/src/lib.rs:90-106`) 返回稳定码；`EngineError` 包装 Trino 错误，ACL 自检应区分权限拒绝与对象不存在/语法错，并把禁用原因脱敏。现有 `EngineStatus::{Available, Unavailable { reason }}` (`engine/src/lib.rs:84-89`) 可作为库健康状态的类比。
- **配置边界:** `ServerConfig` 和 `Identities` 使用 `#[serde(deny_unknown_fields)]`、Option/default (`engine/src/config.rs:31-47`, `:133-157`)；`server_config.rs:20-55` 用成对合法/非法 TOML 用例测试正值范围与身份不同。
- **数据库边界:** `EditStore` 只暴露同一 backend，`storage/src/lib.rs:1-23` 保持 storage -> auth/engine、query -> storage 的依赖方向；管理路径 `scope_store.rs:510-545` 锁内重算 plan hash、写入并审计，不应改成远程可调用管理工具。
- **测试边界:** 核心 SQL、ACL 和审计逻辑先 TDD RED，再 GREEN；`06-VALIDATION.md` 目前为 draft，测试文件/CI 发现数在实施后更新。历史 104-ID 原 SQL 恢复数为 0，新 fixture 必须逐 ID 标 `constructed-equivalent`，不能用原 G/B 标签掩盖 `G11` 已知语义损坏。

## No Analog Found

| File / Behavior | Role | Data Flow | Reason |
| --- | --- | --- | --- |
| `ci/trino/l2-access-control.*` | test config | file-I/O | 当前 `ci/query-services.sh:39-44` 只挂 catalog；仓库没有已跟踪的 Trino ACL 规则。按 Trino 483 官方 ACL 格式设计并实测。 |
| `read_audit.rs` 的有界 permit/批量 writer/失败健康状态 | service, store | batch, event-driven | `scope_store` 是同步于事务的管理审计，不提供异步有界队列；复用存储和错误风格，队列逻辑按 `06-RESEARCH.md` 另行 TDD。 |
| 104-ID 原始 SQL fixture | test fixture | file-I/O | 原试验仅剩 Markdown 描述，原始 SQL 0 条可恢复；29 条新构造样例和剩余 75 个 ID 均须显式标来源及支持性。 |

## Metadata

**Analog search scope:** `crates/ontoforge-{engine,auth,query,storage}`, `crates/ontoforge/tests`, `ci/`, `.github/workflows/`。
**Tracked analog families:** printer/Trino、scope、QueryService、storage/migrations、conformance/Trino CI。
**Pattern extraction date:** 2026-10-26。
