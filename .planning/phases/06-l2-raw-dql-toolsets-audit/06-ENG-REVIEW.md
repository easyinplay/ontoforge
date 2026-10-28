# Phase 6 工程审查

**日期：** 2026-10-26  
**性质：** 基于当前代码与 accepted ADR 的独立复审，供 GSD plan-phase 拆分任务。`docs/designs/trino-engine-sql-tiers.md` 中既有的 gstack `/plan-eng-review` 为跨层设计的 `CLEAR (PLAN)`，本文件不冒称新的正式 gate。  
**结论：** 合同可进入详细规划。以下 P0 项须在 PLAN 中指派文件、测试和可观察完成证据；本轮仅静态审查，未运行语料或 ACL 集成测试。

## P0 阻断项

1. **L2 SQL 的受控入口。** `crates/ontoforge-engine/src/printer/mod.rs` 的 `PrintedSql` 构造私有，`Engine::execute` 只接受它，Trino POST 发送 `sql.as_str()`；当前没有 L2 校验器。规划一个窄入口，将字节上限、单条 `SELECT`/`WITH` AST 白名单、relation/CTE 判定、catalog/schema 授权、edit store 与 `system` 硬排除、外层 LIMIT、Display round-trip 和发送受控 Display 绑定为不可拆开的调用链。不得暴露 `PrintedSql::from(String)` 让任意字符串冒充 printer 产物。未知 AST 变体默认拒绝。spike 2 记录长表达式 AST drop 栈溢出，需字节上限及专用执行栈的压力用例。

2. **Trino ACL 的证明。** `crates/ontoforge-engine/src/config.rs` 已解析 `identities.l2`，`TrinoEngine` 可隔离 user/auth；尚无 L2 enable、自检和健康状态。默认禁用；部署方显式声明 ACL 且受限 Trino 配置通过正反探针后才启用。越界 catalog/edit store 必须收到权限拒绝，不能把对象不存在、空结果或语法错误算作权限证明。检查 `system.runtime.queries` 时应有另一主体的已知活跃查询，单凭当前看不到别人无法证明“只见自己”。身份相同、探针对象不可用、超时/未知结果、ACL 撤销后复检失败均禁用并给出不含秘密的健康原因。现有无 ACL 的 Trino 483 仅支持语法和执行复验。按主体 `X-Trino-User` 透传仍是可选部署模式。

3. **读审计门禁完整覆盖。** `QueryService` 公开入口是 `execute`（pk，`crates/ontoforge-query/src/lib.rs`）、`execute_list`（含 set/searchAround，`execute.rs`）、`execute_aggregate`（`aggregate.rs`）；`read_pk_source` 也供 Action 使用，是内部辅助，不能重复记为公开读。六个 L2 库级工具入口亦须覆盖。PLAN 应列每个入口的成功、拒绝、源错误、截断和取消行为。开始执行前检查 sink 健康并预留有界队列容量，完成后提交元数据记录；队满/持续写失败返回 `AuditUnavailable`，即使结果已经计算也不能释放未审计结果。批量任务退出、重试耗尽须使共享健康状态不可用，恢复前不受理新读。崩溃至多丢约 1 秒是设计承诺，须明确 flush 周期、关闭 drain 和故障注入证据，不称为持久化确认。队满时无法保证拒绝事件再写入同一个失效 sink，可用健康/日志观察。记录主体、工具、对象类型、apiName 级条件、生效 role、SQL 哈希、行数、耗时和 request_id；不得存结果或完整原始 SQL。

4. **双后端持久化与保留期。** `crates/ontoforge-storage/migrations/{postgres,sqlite}` 有 `of_admin_audit`，没有读审计表。分别新增迁移及同语义批量写入/清理接口，确定保留期单位、配置校验、索引、清理失败行为；PG 表在 system schema，不可由 edit store catalog 读取。故障测试覆盖 PG 和 SQLite，不以纯内存队列宣称 AUDIT-01 通过。

## P1 接口与范围

- **授权和快照：** `CompiledScope::for_principal` 已合并 `sql_levels`、`l2_catalogs`、`l2_schemas`、`roles`。L2 检查 `sql:raw` 及请求时的 scope 快照，catalog/schema grant 包含语义写成测试。`compile_scope` 当前只校验 schema 形态和 raw 级别，运行时仍须硬排除 `system`/edit catalog。`QueryService` 持有 `Arc<ActiveDefinition>`；请求开始固定它及 scope，整个查询与审计使用同一 `def_version`。
- **工具库：** ADR-0019 的 `execute_query`、`list_catalogs`、`list_schemas`、`list_tables`、`get_table_schema`、`explain_query` 在 Phase 6 提供库级操作、参数校验、授权过滤、行数/字节限制、审计。`list_*` 仅返回授权名称，`explain_query` 亦先校验 SQL。REST/MCP 注册、动态 `list_tools`、HTTP `/health` 路由属于 Phase 7；Phase 6 提供可复用的健康状态与调用接口。
- **管理复用：** `crates/ontoforge/src/reload.rs` 已先 validate/lint 再打开 store，`crates/ontoforge-storage/src/reload.rs` 锁内复核后切定义；`scope apply` 锁内重算 plan hash 并事务性写 `of_admin_audit`。沿用 CLI/库契约。CLI 本地 `actor()` 不是远程认证主体，管理路径不可暴露为 agent MCP 工具。
- **错误和指标：** `QueryError` 尚无 `AuditUnavailable`。L0/L2 分别保留 ADR-0019 的错误可见性；request_id、队列深度/最旧待写时间、sink 故障和 L2 禁用原因由库层可观察。

## 可执行验收

| 合同 | 必需证据 |
| --- | --- |
| L2-01/02 | 机器可执行 corpus 标记 spike 2 可还原原文与新写等价用例；TDD RED 后验证 AST 放行/拒绝、Display-only POST、截断、长表达式；Trino 483 执行允许语句，拒绝路径 POST 为零。 |
| L2-03 | catalog/schema grant 交叉、未限定名称、CTE、quoted identifier、`TABLE`、表函数、edit catalog、`system.runtime.queries` 的拒绝语料和发送计数。 |
| L2-04 | 单独受限 ACL 配置；越界 catalog、edit store、另一主体已知活跃 query 的探针；未知、超时、错误、ACL 撤销复检均禁用且有健康原因。无 ACL 容器不可替代。 |
| AUDIT-01 | pk/list/set/searchAround/aggregate 与六个 L2 操作逐入口覆盖；PG/SQLite 迁移、批量 flush、保留期、队满、sink 失败、恢复、取消；证实无未审计结果、结果数据或完整 SQL 入库。 |
| SC-5 | CLI reload lint 失败不切换；scope apply 错 hash 不写，成功有事务审计；请求中途 reload 保持单一 `def_version` 及 scope 快照。 |

先固定 corpus 并在真实 Trino 483 复验 Display，再计划 L2 SQL、ACL 自检、审计迁移/队列/入口和管理回归。核心安全逻辑按 TDD RED/GREEN/REFACTOR；各任务在 PLAN 中写具体文件、故障注入方法及测试命令。Phase 7 复用已验收的库接口。
