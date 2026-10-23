# Phase 5: Online Action - Context

**Gathered:** 2026-10-23
**Status:** Ready for architecture review and planning
**Source:** GSD discuss-phase; existing ADRs, code investigation, and Grok second opinion

<domain>
## Phase Boundary

交付 ACTION-01..12 的 online Action：参数与规则运行时校验、scope 与 submission criteria、validate-only、同一 Action 的原子批量调用、乐观并发、按主体作用域的 `command_id` 幂等、事务性 webhook outbox。Nessie/Iceberg `batch` action、REST 与 MCP shell 属后续范围。

</domain>

<decisions>
## Implementation Decisions

### 批量与幂等
- **D-01:** 同一 Action 的一次批量调用是一个原子命令：一个主体作用域的 `command_id`、一条含所有 edit entry 的 edit log、一份含逐项结果的批级回执、一条带稳定 `event_id` 的 outbox 事件。任一项失败时全部回滚，幂等记录和事件也不留存。此粒度复用 ADR-0018 的 `(principal_id, command_id)` 与 ADR-0026 的一次 `Commit` 一条日志；不采纳 Grok 建议的全局唯一键或逐 edit 日志。— **Reversibility:** one-way — 发布后改幂等键和日志粒度会改变客户端重试契约及已存档日志的读取方式。
- **D-02:** 同键同参重放只返回已存回执，不追加 edit、日志或 outbox；同键异参返回 `CommandIdReused`。返回回执或 `get_command_result` 前按当前主体、当前定义与 scope 重新检查 action 执行权及回执字段的披露权限；失权时拒绝披露，不撤销原提交。成功删除后的对象不再可见，不能以“当前目标行可见”作为查询原回执的必要条件。请求指纹覆盖 action、整批参数与影响执行语义的选项，以规范化结构计算，不依赖 JSON 键顺序。

### 授权与并发
- **D-03:** 保持 ADR-0018 的 ①-⑨ 管线：前置目标不可见与不存在同错；写字段越界、criteria 读取遮蔽列、写后不可见（含 link 两端与 release）返回不泄露状态的 `ScopeDenied`。validate-only 执行同样的检查且不写入。
- **D-04:** `_version` 是可选的客户端条件；有令牌时比较事务内锁定读集重算的 HMAC。无令牌也必须以本次前置读取观察到的 edit 版本做并发条件，使同 pk 并发提交最多一个成功。source 在前置读到提交间的变化按 ADR-0018 不二次读取。

### 参数与 source
- **D-05:** ACTION-01 的“约束”限当前 Action IR 可表达的 base type、objectReference、items、required、default 与 rule 静态合同；逐参数返回运行时错误。enum/range/regex/UUID/uniqueness 等新 value types 属 SCHEMAX-04，不引入 Phase 5。
- **D-06:** ACTION-12 的 `AmbiguousObject` 针对本次 Action 确实从 source 读取并观测到的重复 pk；只涉及已编辑对象且无需 source 内容的路径可在 Trino 不可用时执行。没有本地 PK registry，无法同时保证此离线路径对未读取的 source 重复 pk 做绝对检测；ADR-0018 本来也允许 source 采样后变化不检测。对实际读取到的重复 pk 必须拒写；此解释应在架构审查和验收中显式验证。

### Planner Discretion

事务组合 API、批内同 pk 多次规则的顺序与冲突处理、outbox worker 重试调度及 webhook payload 形状由 planner 定义，但不得改变上述原子性、错误语义和下游 `event_id` 去重契约。优先使用现有 crate 边界和双后端 conformance 测试。

</decisions>

<canonical_refs>
## Canonical References

- `.planning/ROADMAP.md` Phase 5、`.planning/REQUIREMENTS.md` ACTION-01..12：范围与验收。
- `.planning/adr/0016-storage-system-data-and-typed-edit-store.md`：PG/SQLite 系统表、单事务写入和 outbox。
- `.planning/adr/0018-authz-scope-and-sql-tiers.md`：Action 鉴权顺序、HMAC 令牌、主体作用域幂等和 source 采样边界。
- `.planning/adr/0024-edit-store-physical-format.md`：edit 物理表、锁与两后端类型。
- `.planning/adr/0025-row-predicate-semantics.md`：scope 行谓词语义。
- `.planning/adr/0026-edit-log-format.md`：一次 Commit 一条日志、append-only 和 storage 不去重。
- `docs/designs/trino-engine-sql-tiers.md`：online Action 与后续 Nessie/Iceberg `batch` action 的边界。

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/ontoforge-schema/src/ir/action.rs` 与 `validate/actions.rs`：Action IR 和配置期 rule/参数校验。
- `crates/ontoforge-auth/src/scope/`：action 授权、行列 scope 与 criteria 静态列覆盖。
- `crates/ontoforge-query/src/lib.rs`：HMAC `_version`、source `LIMIT 2` 歧义检测与全 patch 跳过 Trino 的 pk 读取。
- `crates/ontoforge-storage/src/commit.rs`：Edit、Commit、Receipt、定义栅栏、稳定锁序和单事务 edit/log；双后端 conformance harness 可扩展。

### Integration Points
- `crates/ontoforge-action/src/lib.rs` 仍是占位。`EditStore::commit` 当前自行开始与结束事务，`WriteTx` 仅 crate 内可用；需要在 storage 内建立受控组合入口，支持锁内鉴权/版本条件及同事务的命令与 outbox 写入。
- `ontoforge-query` 的 `version_for` 目前是私有方法，签名包含 scope 的 Debug 文本；规划须审查共享令牌计算与跨版本稳定性，避免 Action 复制算法后漂移。

</code_context>

<specifics>
## Specific Ideas

用户要求遇到未决事项先征询 Grok 第二意见，再结合项目 ADR 和代码作出判断。Grok 的建议仅作参考，不覆盖已接受的 ADR。

</specifics>

<deferred>
## Deferred Ideas

SCHEMAX-04 的新 value types、Nessie/Iceberg `batch` action、持续同步的 source PK registry 均不属于 Phase 5。

</deferred>
