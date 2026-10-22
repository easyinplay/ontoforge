# Phase 4: Scoped L0 Read - Context

**Gathered:** 2026-10-22
**Status:** Ready for planning
**Source:** ADR Ingest Express Path (`.planning/adr/0020-query-execution-rust-orchestrated-overlay.md`)

<domain>
## Phase Boundary

交付 QUERY-01..10：在调用者 scope 内按 pk、过滤、搜索、排序分页、遍历、集合运算与聚合读取对象。Rust 合并 Trino source 与 PG edit store，Trino 只执行单表查询。REST、MCP、Action 提交与 L2 不在本 phase。

</domain>

<decisions>
## Implementation Decisions

- **D-01:** ObjectSet AST 是查询入口与 wire 格式；生产叠加只在 Rust 实现。Trino 查询单表且不超过 2 fragment，编译器强制；不采用 Trino 内叠加主路径。
- **D-02:** pk 读取先读 PG edit，按需通过 Trino `LIMIT 2` 查 source；列表并行取得 source 与 edit 候选，按 pk 至多 256 一批回查、合并、去重并补取到页满或 source 耗尽。
- **D-03:** scope 行谓词对原列过滤，列遮蔽在投影边界；合并后的行用同一 CEL 语义在 Rust 求值。多 role 的行列授权遵守 ADR-0018，不能通过过滤、排序、聚合或错误反推隐藏列。
- **D-04:** count / sum / avg 分解聚合，min / max 通过 TopN 补取，distinct 成本过高时明确报错；connector 能力影响下推选择。
- **D-05:** FK link 由合并后的对象决定；M:N 的 source join 表与 PG link edit 合并。String 搜索用 token 匹配，不提供相关度排序。
- **D-06:** keyset 游标按排序键与 pk 定位，`has_more` 判定结束；跨批次非快照一致。读结果带不透明 HMAC `_version`，行数与字节双上限超出时返回 `truncated`。
- **D-07:** Rust 叠加对照内存参考模型做 proptest；SQLite 替身与 Trino 对照固定单表语义语料。安静或隔离集群的延迟预算按 ADR-0020 验收。

### Planner Discretion

模块拆分、内部 AST 类型、批量大小以下的补取调度和测试文件布局由 planner 依据现有代码确定，不改变上述外部语义。

</decisions>

<canonical_refs>
## Canonical References

- `.planning/ROADMAP.md` Phase 4、`.planning/REQUIREMENTS.md` QUERY-01..10：范围与验收标准。
- `.planning/adr/0020-query-execution-rust-orchestrated-overlay.md`：本上下文的 ADR ingest 来源与叠加主路径。
- `.planning/adr/0018-authz-scope-and-sql-tiers.md`、`.planning/adr/0025-row-predicate-semantics.md`：行列授权与 CEL 的 Rust/SQL 一致性。
- `.planning/adr/0015-source-binding-and-type-contract.md`、`.planning/adr/0024-edit-store-physical-format.md`：source pk 歧义与 edit patch 物理语义。
- `.planning/adr/0019-mcp-toolsets-and-l2-tools.md`：结果上限、游标和 L0 错误边界。
- `.planning/adr/0023-phase3-crate-dependency-direction.md`：`ontoforge-query` 依赖 `ontoforge-storage` 的方向。
- `docs/designs/trino-engine-sql-tiers.md` §2、§3、§6：读路径、scope 与已完成 Eng Review 决策。
- `.planning/research/spike-03-trino-overlay-latency.md`、`.planning/research/spike-03b-scoped-overlay.md`：单表与 scoped 查询实测依据。

</canonical_refs>

<code_context>
## Existing Code Insights

- `crates/ontoforge-query/src/lib.rs` 仍是占位 crate；其中 `sea-query` 编译注释已被 ADR-0020 取代，实施时须以 ADR 为准。
- `crates/ontoforge-engine/src/printer/` 提供 Trino/SQLite 类型化 SQL printer；`crates/ontoforge-engine/src/sqlite/` 提供 dev/test 替身。
- `crates/ontoforge-storage/src/` 持有 edit 表、历史、scope store 与版本状态；`crates/ontoforge-auth/src/scope/` 持有行谓词编译和授权合并。

</code_context>

<specifics>
## Specific Ideas

已有 ADR 与设计文档锁定外部语义。本次未新增用户偏好。

</specifics>

<deferred>
## Deferred Ideas

快照一致分页与相关度排序留给后续 milestone；REST、MCP、Action、L2 按 ROADMAP 的后续 phase 处理。

</deferred>
