# Phase 6: L2 Raw DQL, Toolsets & Audit - Context

**Gathered:** 2026-10-26
**Status:** Ready for research and engineering review
**Source:** GSD phase context from accepted ADRs, existing code, Trino 483 probe, and Grok second opinion; no new user product decision

<domain>
## Phase Boundary

交付 L2-01..04、AUDIT-01：受信主体在显式授权的 catalog/schema 上执行受控 Trino 只读 SQL，Trino 侧权限无法证明安全时禁用 L2；所有现有公开读取路径的元数据审计在队列积压或持续写入失败时 fail-closed。巩固已有 reload、role/scope CLI 管理通道及请求定义版本固定。REST/MCP 外壳与真实发布属于 Phase 7。

</domain>

<decisions>
## Implementation Decisions

### L2 SQL 与授权
- **D-01:** 保持 ADR-0018 的 `sql:raw` + catalog/schema allow-list，L2 不施加 L0 行列 scope；`system` 元数据表与 edit store catalog 硬排除。只接受单条 SELECT / WITH，拒绝 TABLE()、其他表函数、CALL、EXECUTE、多语句、会话/事务语句和无法解析的输入；执行仅发送校验后的 sqlparser Display 文本，不发送原文。解析器、allow-list 与 LIMIT 必须在单一受控入口收敛；任何无法证实的 AST 变体 fail-closed。
- **D-02:** Trino ontology/L2 身份隔离。L2 默认不启用，只有部署方显式声明 Trino ACL 且启动自检证明 allow-list 外、edit store 被拒及 `system.runtime.queries` 不泄露其他主体查询时才可启用；未知、超时或成功越界均禁用 L2 并报告健康状态。按主体 `X-Trino-User` 透传是已接受设计中的可选部署模式，不是 Phase 6 强制新合同。
- **D-03:** `sqlparser` 仅校验 L2 输入，不替换现有类型化 SQL printer。现有 `PrintedSql` 私有构造应保留为信任边界；research/planner 设计受控已验证 SQL 到 Engine 的窄入口，不能把可由调用者构造的任意字符串提升为 `PrintedSql`。

### 审计与管理
- **D-04:** `AUDIT-01` 记录主体、工具、对象类型、apiName 级条件、生效 role、SQL 哈希、行数、耗时和 request_id，不存结果数据或完整原始 SQL。双后端系统表、有界异步批量队列、可配置保留期；读请求在无法入队、积压或持续落库失败时返回 `AuditUnavailable`。按 ADR-0016 的承诺，崩溃时至多丢约 1 秒已受理读审计；验收须覆盖队列与 sink 故障。— **Reversibility:** costly — 审计持久字段与已有查询入口的接线会成为运营合同，变更需要迁移和跨模块回归。
- **D-05:** Phase 6 复用已有 `reload` 与 `scope plan|apply` CLI 管理边界。定义先 validate/lint，失败不切换；role/scope 仍受计划哈希、定义锁及事务性 `of_admin_audit` 保护；请求开始固定 `Arc<ActiveDefinition>` 快照。新增远程管理 API 和 MCP 工具不在本 phase，Phase 7 外壳复用库接口。

### 验收范围
- **D-06:** spike 2 的历史 104 条仅以 Markdown 描述保留，临时 harness 已删除。必须将可还原原文和需重新编写的等价用例分别标记，固定机器可执行 corpus，在真实 Trino 483 对 Display 与拒绝路径复验；不得把新造样例声称为旧原文。现有 483 容器没有 ACL，能验证语法/执行，不能作为 L2 权限自检通过的证据；ACL 验收需独立受限配置。

### Planner Discretion

审计队列容量/批量参数、内部 trait/API、SQL 受控入口的模块布局和 ACL 探针实现由 research 与 planner 依据现有 crate 决定。必须对所有公开读取入口逐一列矩阵，证明不会绕过审计；队列已满时无法写入同一审计 sink 的拒绝事件，不承诺不存在的审计记录，可用健康/日志观察。核心逻辑与安全边界按 TDD 先 RED 后 GREEN。

</decisions>

<canonical_refs>
## Canonical References

- `.planning/ROADMAP.md` Phase 6、`.planning/REQUIREMENTS.md` L2-01..04 / AUDIT-01：范围与五条行为验收。
- `.planning/adr/0016-storage-system-data-and-typed-edit-store.md`：系统库、读审计有界队列与失效承诺。
- `.planning/adr/0018-authz-scope-and-sql-tiers.md`：L2 授权、双身份、自检 fail-closed 和管理变更权限。
- `.planning/adr/0019-mcp-toolsets-and-l2-tools.md`：L2 工具名、结果上限、REST/MCP Phase 7 边界。
- `.planning/adr/0017-engine-and-trino-connection.md`：sqlparser 只校验 L2 输入，已有 printer 的信任边界。
- `.planning/research/spike-02-sqlparser-l2.md`：104 条历史语料、Display 差异、DoS 风险和复验待办。
- `docs/designs/trino-engine-sql-tiers.md` §4-6、Eng Review D11/D14/D15/D21/OV2/OV3：已完成 `/plan-eng-review` 的跨层架构合同；具体代码接线仍需 Phase 6 独立审查。
- `.planning/phases/03-edit-store-scope-model/03-CONTEXT.md`：现有 reload、scope apply 与 admin audit 的 Phase 3 合同。

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/ontoforge-auth/src/scope/compile.rs`、`crates/ontoforge-schema/src/ir/scope.rs`：`sql:raw` grant、L2 catalog/schema 授权与生效 role。
- `crates/ontoforge-engine/src/config.rs`、`src/lib.rs`：独立 L2 `TrinoEngine`、`QueryContext` 和有界执行；`printer/mod.rs` 保留 `PrintedSql` 私有构造。
- `crates/ontoforge-query/src/{lib,execute,aggregate}.rs`：pk/list/aggregate 公开读入口与定义快照；searchAround/set 经这些路径。
- `crates/ontoforge-storage/src/{reload,scope_store}.rs`：定义锁、DDL 可见性、管理审计；PG system schema 与 SQLite 后端。

### Integration Points
- `ontoforge-server` / `ontoforge-api` 尚为占位；Phase 6 库级 L2 与现有 CLI 可被 Phase 7 REST/MCP 复用。L2 无校验/执行入口，读审计表/API 与 `AuditUnavailable` 尚不存在。
- Trino 483 容器 `ontoforge-trino-05-06` 在 `127.0.0.1:58080` healthy，`SELECT 1` 可执行；其当前配置没有 ACL，必须另建受限验收环境。

</code_context>

<specifics>
## Specific Ideas

用户要求不决时先问 Grok 并综合第二意见。本次 Grok 建议审计完整 SQL、双身份透传必选等项与 REQUIREMENTS/ADR 不符；采用它对语料固定、AST 零越界发送、ACL 真拒绝、审计 sink 故障注入的测试建议，取舍见 `findings.md`。

</specifics>

<deferred>
## Deferred Ideas

REST/MCP 暴露与真实生产发布属于 Phase 7；L1 Ontology SQL 与 `sql:ontology` 属下一 milestone。当前没有额外产品范围。

</deferred>

---

*Phase: 06-l2-raw-dql-toolsets-audit*
