# Phase 05: Online Action - Research

**Researched:** 2026-10-23
**Domain:** Rust Action execution, scoped authorization, SQL transactions, idempotency, webhook outbox
**Confidence:** HIGH for repository contracts; MEDIUM for the proposed composition API and delivery schedule

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
<!-- DATA_A7C9F2D1_START -->
- **D-01:** 同一 Action 的一次批量调用是一个原子命令：一个主体作用域的 `command_id`、一条含所有 edit entry 的 edit log、一份含逐项结果的批级回执、一条带稳定 `event_id` 的 outbox 事件。任一项失败时全部回滚，幂等记录和事件也不留存。此粒度复用 ADR-0018 的 `(principal_id, command_id)` 与 ADR-0026 的一次 `Commit` 一条日志；不采纳 Grok 建议的全局唯一键或逐 edit 日志。— **Reversibility:** one-way — 发布后改幂等键和日志粒度会改变客户端重试契约及已存档日志的读取方式。
- **D-02:** 同键同参重放只返回已存回执，不追加 edit、日志或 outbox；同键异参返回 `CommandIdReused`。回执只含发起主体原提交的对象/link 标识、operation 与版本等审计摘要，不含旧属性值或当前对象状态。重放或 `get_command_result` 以 `(principal_id, command_id)` 限定主体，按当前授权重新检查 action 执行权；失权时拒绝披露，不撤销原提交。成功删除后的目标行不可见不妨碍有权主体查询原回执；若未来返回当前对象数据，该数据另按当前行列 scope 过滤。请求指纹覆盖 action、整批客户端参数与影响执行语义的选项，以规范化结构计算，不依赖 JSON 键顺序；server 生成的 `currentTime` 不进客户端指纹，选定值随回执持久化。默认幂等保留 7 天，过期后的同键不承诺去重。
- **D-03:** 保持 ADR-0018 的 ①-⑨ 管线：前置目标不可见与不存在同错；写字段越界、criteria 读取遮蔽列、写后不可见（含 link 两端与 release）返回不泄露状态的 `ScopeDenied`。validate-only 执行同样的检查且不写入。
- **D-04:** `_version` 是可选的客户端条件；有令牌时比较事务内锁定读集重算的 HMAC。无令牌也必须以本次前置读取观察到的 edit 版本做并发条件，使同 pk 并发提交最多一个成功。source 在前置读到提交间的变化按 ADR-0018 不二次读取。
- **D-05:** ACTION-01 的“约束”限当前 Action IR 可表达的 base type、objectReference、items、required、default 与 rule 静态合同；逐参数返回运行时错误。enum/range/regex/UUID/uniqueness 等新 value types 属 SCHEMAX-04，不引入 Phase 5。
- **D-06:** ACTION-12 的 `AmbiguousObject` 针对本次 Action 确实从 source 读取并观测到的重复 pk；只涉及已编辑对象且无需 source 内容的路径可在 Trino 不可用时执行。没有本地 PK registry，无法同时保证此离线路径对未读取的 source 重复 pk 做绝对检测；ADR-0018 本来也允许 source 采样后变化不检测。对实际读取到的重复 pk 必须拒写；验收文字与此观测语义保持一致。
<!-- DATA_A7C9F2D1_END -->

### the agent's Discretion
<!-- DATA_B52E8A61_START -->
事务组合 API、批内同 pk 多次规则的顺序与冲突处理、outbox worker 重试调度及 webhook payload 形状由 planner 定义，但不得改变上述原子性、错误语义和下游 `event_id` 去重契约。优先使用现有 crate 边界和双后端 conformance 测试。
<!-- DATA_B52E8A61_END -->

### Deferred Ideas (OUT OF SCOPE)
<!-- DATA_C91D4E73_START -->
SCHEMAX-04 的新 value types、Nessie/Iceberg `batch` action、持续同步的 source PK registry 均不属于 Phase 5。
<!-- DATA_C91D4E73_END -->

以上逐字复制自 `05-CONTEXT.md:17-31,70-73`。[VERIFIED: .planning/phases/05-online-action/05-CONTEXT.md:17-31] [VERIFIED: .planning/phases/05-online-action/05-CONTEXT.md:70-73]
</user_constraints>

<phase_requirements>
## Phase Requirements

以下 Description 为需求原文；`ACTION-12` 已包含观测限定。[VERIFIED: .planning/REQUIREMENTS.md:75-86]

<!-- DATA_E3A6B294_START -->
| ID | Description | Research Support |
|---|---|---|
| ACTION-01 | 客户端提交 action 时，参数按类型、required 与约束校验，失败时返回逐参数的错误 | 使用 IR 与 `check_literal`，见参数合同。 |
| ACTION-02 | action 的 rules 在单个事务内执行，生成 create / modify / delete object 与 add / remove link 的 edits | 展平为一个 `Commit`，见 E1/E6。 |
| ACTION-03 | submission criteria 不满足时 action 被拒绝并返回配置的失败信息；criteria 只在调用者的遮蔽视图上求值 | 复用静态列覆盖，运行时再检查实际行遮蔽。 |
| ACTION-04 | 客户端可以用 validate-only 模式提交 action，只得到校验结果，不产生写入 | 完整检查后回滚，检查无残留。 |
| ACTION-05 | 客户端可以批量提交同一 action，全部成功或全部回滚 | 批级命令和一条事务。 |
| ACTION-06 | action 成功后返回标准化的 edits 摘要（新增 / 修改 / 删除的对象与 link） | 原始 ordinal 到 batch item 的映射。 |
| ACTION-07 | 对象只能经 action（或可审计的导入）写入，没有其他写入途径 | 收紧公开 `commit` 主体入口。 |
| ACTION-08 | 写字段必须在调用者对该行的可见列内且非系统字段；写后状态（含 link 两端与 release）必须对调用者可见，否则 `ScopeDenied`，错误不泄露对象状态 | 锁内写前与写后授权。 |
| ACTION-09 | 客户端可附带读取时的 `_version` 令牌，不一致返回 `VersionConflict`；edit 读集在事务内按稳定顺序加锁 | 共用 token codec，锁后 CAS。 |
| ACTION-10 | `command_id` 以主体为作用域幂等：重放前重新鉴权，同键异参报 `CommandIdReused`，响应丢失后可凭同键查询结果 | 数据库唯一约束与重读。 |
| ACTION-11 | 提交成功后经事务性 outbox 向下游 webhook 至少投递一次带 `event_id` 的事件 | 事务内写事件，worker 重试。 |
| ACTION-12 | Action 只在目标或读集需要 Trino 时依赖 Trino；本次 Action 从 source 读取并观测到重复 pk 时返回 `AmbiguousObject` 且拒绝写入；无需 source 内容的已编辑对象可在 Trino 不可用时执行 | 共用 source 采样决策与 `LIMIT 2` 歧义检查。 |
<!-- DATA_E3A6B294_END -->
</phase_requirements>

## Summary

Phase 5 是 `ontoforge-action` 中的新运行时，依赖现有 schema/auth/query/storage 能力；该 crate 目前只有占位常量。[VERIFIED: crates/ontoforge-action/src/lib.rs:1-12] 核心风险集中在 storage 事务组合、query 签名格式、并发命令争用和回放披露；E1-E7 逐项合同见下表。[VERIFIED: .planning/phases/05-online-action/05-ENG-REVIEW.md:10-21]

**Primary recommendation:** 按“纯参数预检与按需 source 采样 → storage 持有的短事务内定义栅栏、命令争用、稳定锁、CAS 与授权 → 一次 `Commit`、一条命令回执、一条 outbox → 事务外 worker 投递”分 wave 计划；每个 wave 配双后端或真实服务的可执行证据。[VERIFIED: .planning/adr/0018-authz-scope-and-sql-tiers.md:16-19] [VERIFIED: .planning/adr/0016-storage-system-data-and-typed-edit-store.md:12-16]

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|---|---|---|---|
| 参数、rule、criteria 与授权 | API/backend (`ontoforge-action`) | `schema` / `auth` / `expr` | Action 编排业务语义；已有 IR 与 scope 求值。[VERIFIED: crates/ontoforge-schema/src/ir/action.rs:16-193] [VERIFIED: crates/ontoforge-auth/src/scope/merge.rs:98-118] |
| source 采样与读合成 | API/backend (`ontoforge-query`) | Trino / SQLite Engine | 现有 pk read 已按需调用 engine 并做歧义检查。[VERIFIED: crates/ontoforge-query/src/lib.rs:236-320,368-424] |
| 原子写、锁、幂等与 outbox | Database/storage (`ontoforge-storage`) | `ontoforge-action` | `WriteTx` 当前只在 storage 可见，存储层持有提交边界。[VERIFIED: crates/ontoforge-storage/src/store.rs:142-158] [VERIFIED: crates/ontoforge-storage/src/commit.rs:136-163] |
| webhook 投递 | API/backend worker | Database outbox | 事务内仅持久化；网络投递在提交后。[VERIFIED: .planning/REQUIREMENTS.md:85-85] [ASSUMED] |

## Standard Stack

不新增外部 crate；沿用 workspace 锁定依赖与既有 `Cargo.lock`。[VERIFIED: Cargo.toml:10-54] 因本 phase 无安装步骤，Package Legitimacy Gate 不触发。现有版本引用的 source-of-truth 原文如下。[VERIFIED: Cargo.toml:19-50]

<!-- DATA_D45A7B90_START -->
```toml
cel = "0.15.0"
chrono = { version = "0.4.45", default-features = false, features = ["std", "clock"] }
tokio = { version = "1.53.3", features = ["rt", "time", "sync"] }
sqlx = { version = "0.9.0", default-features = false, features = ["sqlite", "runtime-tokio", "postgres", "migrate", "macros", "json", "chrono", "tls-rustls-aws-lc-rs"] }
ulid = "3.0.0"
wiremock = "0.6.5"
sea-query = { version = "1.0.2", default-features = false, features = ["backend-postgres", "backend-sqlite", "with-json", "with-chrono", "postgres-array"] }
sea-query-sqlx = { version = "0.9.1", default-features = false, features = ["sqlx-postgres", "sqlx-sqlite", "runtime-tokio", "with-json", "with-chrono", "postgres-array"] }
jsonwebtoken = { version = "11.1.0", default-features = false, features = ["aws_lc_rs"] }
```
<!-- DATA_D45A7B90_END -->
[VERIFIED: Cargo.toml:19-50]

| Library | Version | Purpose | Planning instruction |
|---|---:|---|---|
| `sqlx` + `sea-query` + `sea-query-sqlx` | 0.9.0 / 1.0.2 / 0.9.1 | PG/SQLite 事务与参数绑定 | 扩展 `sql_exit`，维持 SQL 出口唯一性。[VERIFIED: crates/ontoforge-storage/src/sql_exit.rs:1-28] |
| `cel` + `ontoforge-expr` | 0.15.0 / in-repo | criteria 解析和运行 | 保留 `analyze` 的长度、深度、栈保护；静态检查不能代替运行时遮蔽。[VERIFIED: crates/ontoforge-expr/src/analyze.rs:60-83] [VERIFIED: crates/ontoforge-auth/src/scope/coverage.rs:1-26] |
| `jsonwebtoken` + shared `TokenCodec` | 11.1.0 / in-repo | HS256 `_version` | 一份签名输入编码供 query 和 Action 共用。[VERIFIED: crates/ontoforge-query/src/lib.rs:153-225,334-365] |
| `chrono`, `ulid`, `tokio`, `wiremock` | 0.4.45 / 3.0.0 / 1.53.3 / 0.6.5 | 时间、事件 ID、worker、HTTP 假端点 | 复用已有依赖；把 worker 的 HTTP 实现接到已有 `reqwest`，不要装新库。[VERIFIED: Cargo.toml:29-50] |

## Architecture Patterns

**Recommended project structure:** Action 运行时和 worker 留在 `crates/ontoforge-action/src/`；把 query 现有 `TokenCodec` 的版本输入计算提取为 query 的公开窄 API，让 Action 添加本地 `ontoforge-query` 依赖；组合事务、命令/outbox migration 留在 `crates/ontoforge-storage/`；scope 检查仍调用 `ontoforge-auth`。[VERIFIED: crates/ontoforge-action/Cargo.toml:1-17] [VERIFIED: crates/ontoforge-query/Cargo.toml:1-23] [VERIFIED: crates/ontoforge-storage/Cargo.toml:1-39] [ASSUMED] 当前 query 没有 Action 依赖，因此这个 crate 方向无需新共享 crate；计划仍需编译检查。[VERIFIED: crates/ontoforge-query/Cargo.toml:1-23]

```text
trusted host + Principal + Action request
  -> current action grant + canonical client fingerprint
  -> parameter/default/rule expansion + source sampling if required
     -> source duplicate? AmbiguousObject / engine down? EngineUnavailable
  -> storage-owned write transaction
     -> definition fence + database unique command claim
        -> old command? current authorization -> stored receipt or CommandIdReused
     -> stable edit/read-set locks -> observed-version CAS / HMAC
     -> current-row scope + masked criteria -> in-memory post-state scope
        -> one Commit -> one log/event_id + edit rows/history
        -> one stored batch receipt + one outbox row -> COMMIT
  -> outbox worker -> webhook -> mark delivered / retry same event_id
```

该流程的 source 读取在写事务外；锁内必须重新检查 edit 版本以及受其影响的授权状态，不能重读 source。[VERIFIED: .planning/adr/0018-authz-scope-and-sql-tiers.md:16-19] 现有 `commit()` 的 `check_shape → begin_write → fence → validate → write → commit/rollback` 必须拆出 storage 私有步骤，Action 不能在外层事务里再调用公开 `commit()`。[VERIFIED: crates/ontoforge-storage/src/commit.rs:136-163]

### E1: storage 组合事务合同

建议 `ontoforge-storage` 暴露受限 `ActionTx`/`commit_action` 入口：由 storage 创建和结束 `WriteTx`，只给 Action 提供锁后逻辑视图、检查钩子和最终提交操作，不公开原始 SQL 或 `WriteTx`。[VERIFIED: crates/ontoforge-storage/src/store.rs:142-158] [ASSUMED] 新命令路径先取定义栅栏再争用命令唯一键，保持现有“首条事务语句读取 active definition”的锁顺序；重放可在写事务前按主体读取旧结果，争用后必须复核。E1 审查提出“claim 后 fence”，与既有栅栏顺序不同，计划应明确按 ADR 与当前锁顺序执行并以并发测试覆盖。[VERIFIED: crates/ontoforge-storage/src/commit.rs:165-200] [VERIFIED: .planning/phases/05-online-action/05-ENG-REVIEW.md:12-12]

事务内完整 read/edit set 按现有 `RowKey` 稳定顺序锁定；不存在的 edit row 用 version-0 skeleton 锁住。对每个唯一 row 先比较前置观察的 edit 版本，再逐 rule 推进内存状态，随后执行 ①-⑧ 授权及日志/行/回执/outbox 写入；任一错误回滚，validate-only 始终回滚且没有 version-0 row 残留。[VERIFIED: crates/ontoforge-storage/src/commit.rs:218-264,521-568,770-830] [VERIFIED: .planning/phases/05-online-action/05-CONTEXT.md:22-23]

### E2: 共享 `_version` 合同

当前 token 使用长度前缀字段和 HS256，但 `version_for` 是私有方法，scope 输入为 `format!("{:?}", self.scope)`。[VERIFIED: crates/ontoforge-query/src/lib.rs:153-225,334-365] 建议把“版本化输入、规范化 effective type scope、主体属性、principal、type、pk、definition、source 快照、锁定 edit 行”封装成单一 codec，query 签发与 Action 锁后验证调用相同函数。[ASSUMED] 首次切换时明确让旧无版本 token 返回 `VersionConflict`，不隐式用 Debug 格式兼容；Phase 7 API 尚未交付，但需在计划/发布说明写明令牌失效合同。[VERIFIED: .planning/ROADMAP.md:251-264] [ASSUMED] 对无 `_version` 请求仍比较前置 edit 版本；source 漂移不检查。[VERIFIED: .planning/phases/05-online-action/05-CONTEXT.md:23-23]

### E3: 回执披露合同

回执只存批内原始 item ordinal、对象/link 标识、operation、版本、`event_id`/log 序号和选定的 server time；不存旧属性值或可随 scope 改变的当前行数据。[VERIFIED: .planning/phases/05-online-action/05-CONTEXT.md:19-19] [ASSUMED] `get_command_result` 与同键重放先按主体限定查找，再用**当前** action 定义与 grant 授权；role 失权或 action 定义消失则统一拒绝披露。删除或 release 后不要求重新证明原目标当前行可见；因为 receipt 是历史审计摘要，而 `TypeScope::evaluate` 需要当前 row 值，日志也无 pre-image。[VERIFIED: .planning/phases/05-online-action/05-CONTEXT.md:19-19] [VERIFIED: crates/ontoforge-auth/src/scope/merge.rs:107-118] [VERIFIED: .planning/adr/0026-edit-log-format.md:17-22] 过期同键可按新命令处理，但不得把旧结果当成缓存命中；清理策略须保留已提交日志/outbox。[VERIFIED: .planning/phases/05-online-action/05-CONTEXT.md:19-19]

### E4/E7: 幂等争用与时间合同

新增系统表的唯一约束必须**精确**覆盖原文 `(principal_id, command_id)`；`params_hash`、action identity、definition version、receipt、chosen time、expiry 是命令记录的字段，而不是用全局 `command_id` 去重。[VERIFIED: .planning/adr/0018-authz-scope-and-sql-tiers.md:19-19] [ASSUMED] JSON 指纹将 action 与所有 client 参数/语义选项做递归键排序的规范序列化；先 hash 原始 client 请求，再按定义应用 default，server `currentTime` 只在 winner 中选一次并随回执提交。[VERIFIED: .planning/phases/05-online-action/05-CONTEXT.md:19-19] [ASSUMED] 若同键重试时 action 定义版本已变，当前 grant 存在且 fingerprint 相同则只回放原回执，不重新执行；action 消失则按 E3 拒绝。该推荐保持 D-02 的同键重放承诺。[ASSUMED]

PG 用 `INSERT ... ON CONFLICT DO NOTHING` 唯一争用：loser 等 winner 完成后用**新语句**重读；winner rollback 时 loser 可取得 claim，winner commit 时 loser 比 hash 后回放或报 `CommandIdReused`。失败的 PG 语句事务不能盲目继续，选择非抛错 conflict 语句或显式 rollback 后新事务重读。PostgreSQL 的 Read Committed 文档明确说明 `DO NOTHING` 可因当前语句快照不可见的并发事务而不插入。[CITED: https://www.postgresql.org/docs/current/transaction-iso.html] SQLite 继续用已有 `BEGIN IMMEDIATE` 序列化写者。[VERIFIED: crates/ontoforge-storage/src/sql_exit.rs:54-69] 回放路径不能因旧定义版本而重新执行；授权顺序和双方超时/取消必须写成状态机测试。[ASSUMED]

### E5/E6: source 和批内顺序合同

`ActionType.rules` 原文为“Rules run in list order”；`Commit::Receipt.versions` 原文为“in the order of `Commit::edits`”。[VERIFIED: crates/ontoforge-schema/src/ir/action.rs:98-115] [VERIFIED: crates/ontoforge-storage/src/commit.rs:95-102] 先按 batch item index、再按 rule index 展平并记 `(item_ordinal, rule_ordinal, edit_ordinal)`，同 pk 在单批中按原顺序连续应用、每 edit 增版；不同 row 的物理锁序仍用稳定 `RowKey`，log entry 恢复原 edit ordinal。[VERIFIED: crates/ontoforge-storage/src/commit.rs:259-264,521-544,646-664] [ASSUMED] 同批新建对象供后续 rule/item 引用不在当前静态合同内，需拒绝这种引用或仅支持已存在的 `objectReference`，不要建立隐式跨 item 对象命名机制。[VERIFIED: crates/ontoforge-schema/src/ir/action.rs:37-52,120-152]

source-backed 目标只有缺少所需 source 属性时调用 source；query 当前 all-patched 路径跳过 engine，source 路径 `LIMIT 2` 并在两行时拒绝。[VERIFIED: crates/ontoforge-query/src/lib.rs:267-284,368-424] Action 需共享这个采样决策，不能把 query 当前泛化的 `QueryError::Source` 直接当作 `EngineUnavailable`；`EngineError` 已区分 `Unavailable`、`Unauthorized`、`Timeout`、`Query` 等分支。[VERIFIED: crates/ontoforge-query/src/lib.rs:64-100,411-424] [VERIFIED: crates/ontoforge-engine/src/error.rs:49-92] 明确对“实际观察到重复 pk”“需要 source 而 engine 不可用”“全 patch 零 source 调用”各做一条验收，不重新打开 ACTION-12 的绝对唯一性解释。[VERIFIED: .planning/REQUIREMENTS.md:86-86]

### ACTION-11: outbox worker 合同

每次 batch 持久化一份不可变 payload 和同一 `event_id`；建议 payload 仅含事件 schema version、`event_id`、action、log seq、commit time 与批级 edits 审计摘要，不带任意 client 指定的目标 URL、旧属性值或当前对象状态。[VERIFIED: .planning/phases/05-online-action/05-CONTEXT.md:18-19] [ASSUMED] worker 由可信部署配置提供 webhook 目标，事务外发送；库表保留 pending、lease 到期时间、attempt count、next attempt、delivered time。PG worker 用 `FOR UPDATE SKIP LOCKED` 短事务 claim，SQLite 用 `BEGIN IMMEDIATE` 串行 claim；租约过期可重新领取，HTTP 确认成功后才标记 delivered。[VERIFIED: crates/ontoforge-storage/src/sql_exit.rs:54-69] [ASSUMED] 建议指数退避从 1 秒到 5 分钟并持续保留失败事件，避免达到重试次数上限后无声丢弃；重复投递必须逐次携带原 `event_id`，收件方按该 ID 去重。[ASSUMED] 计划要测试 worker 重启、并行 worker 不双 claim、发送成功而标记前崩溃，以及 backlog/attempt/lease 观测指标。[VERIFIED: .planning/phases/05-online-action/05-ENG-REVIEW.md:24-38]

**Cost gate:** 在读取 source 前检查 batch item、展平 edit、distinct read key 与 source call 的硬上限；在真实 PG 压测记录 batch 大小、锁持有时间、source 调用次数和 outbox backlog，再固定默认上限。事务持锁期间不得调用 Trino 或 webhook。[VERIFIED: .planning/phases/05-online-action/05-ENG-REVIEW.md:24-38] [ASSUMED]

## Don't Hand-Roll

| Problem | Use Instead | Reason |
|---|---|---|
| Action 参数值类型 | `BaseType::check_literal` 与 `PropertyValue::from_json` | 两处已有 JSON 检查和持久化规范；计划需验证它们对 decimal/float 等行为一致。[VERIFIED: crates/ontoforge-schema/src/ir/basetype.rs:99-165] [VERIFIED: crates/ontoforge-storage/src/value.rs:1-20] |
| row/column scope | `TypeScope::evaluate` 与编译期 criteria coverage | 既有三值逻辑和列授予规则。[VERIFIED: crates/ontoforge-auth/src/scope/merge.rs:98-118] [VERIFIED: crates/ontoforge-auth/src/scope/coverage.rs:20-50] |
| SQL 与事务驱动 | `sql_exit`、`sea-query`、`sqlx` | 维持唯一 SQL text 出口和双后端绑定。[VERIFIED: crates/ontoforge-storage/src/sql_exit.rs:1-37] |
| edit 状态机与日志 | `next_state`、`next_link_state`、`Commit` 内部 helper | 现有排序、版本、cardinality、append-only 语义不能复制一份。[VERIFIED: crates/ontoforge-storage/src/commit.rs:521-568,615-664] |
| webhook 去重假设 | 持久化同一个 `event_id` 并通知收件方去重 | 至少一次投递不能由发送方保证只收一次。[VERIFIED: .planning/REQUIREMENTS.md:85-85] [ASSUMED] |

## Common Pitfalls

1. **先读缓存再写：** 同键并发可双写；唯一索引必须在同一事务里争用，并在 PG loser 等待后另发 SELECT。[CITED: https://www.postgresql.org/docs/current/transaction-iso.html]
2. **只锁被写 row：** criteria、link endpoint 或 release 所依赖的 edit 状态可在检查后变化；锁完整 read set 后重算可见性和条件。[VERIFIED: .planning/phases/05-online-action/05-ENG-REVIEW.md:12-22]
3. **`validate-only` 创建 skeleton：** 现有 `load_row` 会插入 version-0 row；dry run 必须回滚整个事务，测试再次读取为 absent。[VERIFIED: crates/ontoforge-storage/src/commit.rs:770-831]
4. **仅做静态 criteria coverage：** role 有列权限不等于某行可看该列；运行时要用该行的 `RowVisibility.columns`，失败只给 `ScopeDenied`。[VERIFIED: crates/ontoforge-auth/src/scope/merge.rs:98-118] [VERIFIED: .planning/phases/05-online-action/05-CONTEXT.md:22-22]
5. **先发送再提交或发送后改 event ID：** crash 可丢事件或令重试变成不同事件；outbox 持久 payload/event ID，网络只在事务外。[VERIFIED: .planning/adr/0016-storage-system-data-and-typed-edit-store.md:16-16] [ASSUMED]
6. **公开原始 principal commit：** 目前 `Actor::Principal` 与 `EditStore::commit` 都公开，直接调用绕过 ACTION-07；`import` 使用 `Actor::System(SystemActor::Import)`。计划须审计所有非测试调用，并把业务 principal 写入口收进 Action。[VERIFIED: crates/ontoforge-storage/src/commit.rs:30-63,136-163] [VERIFIED: crates/ontoforge-storage/src/import.rs:1316-1324]

## Code Examples

以下是规划用伪代码，不是已存在 API；涉及的新类型名和阶段安排均为建议。[ASSUMED]

```rust
// Storage owns begin/commit/rollback and never exposes WriteTx.
let snapshot = action.prepare_and_sample(&request).await?;
let outcome = store.commit_action(snapshot, |locked| async move {
    locked.compare_observed_versions()?;
    locked.verify_query_issued_tokens()?;
    action.check_scope_and_criteria(locked)?;
    action.check_post_state(locked)?;
    Ok(action.flatten_one_commit(locked)?)
}).await?;
```

PostgreSQL 命令争用的关键 SQL 形状为 `INSERT ... ON CONFLICT DO NOTHING`，当未插入时必须新语句重读结果；表名与字段由迁移决定，SQL 经 `sql_exit`/`sea-query` 构造。[CITED: https://www.postgresql.org/docs/current/transaction-iso.html] [VERIFIED: crates/ontoforge-storage/src/sql_exit.rs:1-37]

## E1-E7 Planning Contracts and Evidence

| Finding | Required task contract | Executable evidence |
|---|---|---|
| E1 | 组合事务按 fence、claim、锁、CAS、log/edit/command/outbox 排序；任一失败回滚。 | SQLite 与真实 PG 分别在 edit row、log、command、outbox 后注入失败，检查所有表和版本无残留；validate-only 同检但零残留。[VERIFIED: .planning/phases/05-online-action/05-ENG-REVIEW.md:12-12,31-33] |
| E2 | query/Action 共用版本化 codec；旧 token 明确失效。 | query 发 token 后 Action 接受；edit、scope、attrs、key 变化拒绝；无 token 同 pk 双请求只成功一条。[VERIFIED: .planning/phases/05-online-action/05-ENG-REVIEW.md:13-13] [ASSUMED] |
| E3 | 主体限定 + 当前 action grant + 最小历史回执。 | `get_command_result` 和 same-key replay 分别覆盖 role 撤销、row predicate 变化、delete、release、action 删除、过期；无当前 grant 不披露。[VERIFIED: .planning/phases/05-online-action/05-ENG-REVIEW.md:14-14] |
| E4 | 唯一 claim 与 winner/loser 状态机。 | 真实 PG 两连接 same-key 同参/异参、winner abort、响应丢失重试；SQLite 同语义；仅一 log 和一 outbox。[VERIFIED: .planning/phases/05-online-action/05-ENG-REVIEW.md:15-15] |
| E5 | ACTION-12 已写为实际 source 观察。 | 真 Trino 重复 pk 拒写；不可用且需要 source 返回 `EngineUnavailable`；全 patch 不调用 engine 仍成功。[VERIFIED: .planning/REQUIREMENTS.md:86-86] |
| E6 | batch item/rule/edit ordinal 与同 pk 顺序。 | 多 rule、多 item、同 pk 逐 edit 版本、后项失败全回滚；一条 log 保原 ordinal，一 event。[VERIFIED: .planning/phases/05-online-action/05-ENG-REVIEW.md:17-17] |
| E7 | client 指纹与 default/time 扩展分开，winner 时间原子保存。 | JSON 键序等价、default 扩展、跨时间边界重试、定义变化同键回放。[VERIFIED: .planning/phases/05-online-action/05-ENG-REVIEW.md:18-18] |

## Environment Availability

| Dependency | Required By | Available here | Fallback / gate |
|---|---|---|---|
| Rust / Cargo | workspace tests | `rustc 1.98.0`, `cargo 1.98.0` [VERIFIED: local CLI, 2026-10-23] | No fallback needed. |
| Docker daemon | local PG/Trino integration | `docker info` reports `29.7.2` [VERIFIED: local CLI, 2026-10-23] | CI service scripts already exist.[VERIFIED: .github/workflows/ci.yml:80-127] |
| PostgreSQL | true concurrency/rollback | `ONTOFORGE_PG_URL` unset; `psql` not found in this shell [VERIFIED: local environment probe, 2026-10-23] | Use Docker/CI PostgreSQL or existing `embedded-pg`; do not count SQLite as concurrency proof.[VERIFIED: crates/ontoforge-storage/tests/conformance_pg.rs:1-16] |
| Trino | observed duplicate and required/offline path | `ONTOFORGE_TRINO_URL` unset [VERIFIED: local environment probe, 2026-10-23] | Existing CI starts Trino 483 and runs ignored integration tests.[VERIFIED: .github/workflows/ci.yml:84-127] |

## Validation Architecture

`workflow.nyquist_validation` 为 `true`。[VERIFIED: .planning/config.json:24-24] 现有 SQLite/PG conformance 宏一处加 case、两端执行，PG cases 默认 ignored；CI 另外运行真实 Trino/PG。[VERIFIED: crates/ontoforge-storage/tests/conformance/mod.rs:2006-2053] [VERIFIED: crates/ontoforge-storage/tests/conformance_pg.rs:1-16] [VERIFIED: .github/workflows/ci.yml:80-127]

| Property | Value |
|---|---|
| Framework | Rust `cargo test`、`tokio::test`; HTTP fake 用现有 `wiremock`。[VERIFIED: crates/ontoforge-storage/tests/conformance/mod.rs:2006-2053] [VERIFIED: Cargo.toml:31-41] |
| Quick run | `cargo test -p ontoforge-action --locked` 与 `cargo test -p ontoforge-storage --test conformance_sqlite --locked` |
| PG gate | `cargo test -p ontoforge-storage --test conformance_pg --locked -- --ignored`，另加 Action PG 集成测试；必须在真实 server 执行。[VERIFIED: crates/ontoforge-storage/tests/conformance_pg.rs:1-16] |
| Trino gate | `cargo test -p ontoforge-action --test trino_action_it --locked -- --ignored`（建议新增）；CI 服务环境参照现有 query target。[VERIFIED: .github/workflows/ci.yml:121-127] [ASSUMED] |

| Requirements | Test slice | Test type | Wave 0 / gate |
|---|---|---|---|
| ACTION-01..03,08 | parameter/default/rule/CEL/scope matrix; hidden vs missing | unit + SQLite integration | New `ontoforge-action` tests; table cases for each current `ParamType`.[VERIFIED: crates/ontoforge-schema/src/ir/action.rs:58-76] |
| ACTION-02,04..06,09 | batch rollback, validate-only, receipt ordinal, CAS/token | dual-backend integration | Extend conformance plus Action-specific PG race; no ignored-only green run. |
| ACTION-07 | public mutation caller audit and negative compile/API test | source audit + integration | Review `commit` references and fixture migration.[VERIFIED: crates/ontoforge-storage/src/commit.rs:136-163] |
| ACTION-10 | same-key concurrent winner/loser, replay auth/expiry/definition change | real PG + SQLite | Two independent connections; barrier plus failpoint. |
| ACTION-11 | crash after commit, failed HTTP, crash after HTTP success, retry same `event_id` | `wiremock` + DB restart | Worker must resume pending rows after reopen; delivery count >= 1. |
| ACTION-12 | observed duplicate, source required/offline and all-patched no-call | fake Engine + real Trino | Count Engine calls; CI Trino target cannot be skipped. |

**Sampling:** per task run the narrow affected Rust test; per wave run Action + storage SQLite; phase gate runs real PG races/failure injection, real Trino corpus, webhook retries, full `cargo test --workspace --locked`, and checks ignored tests discovered/executed.[VERIFIED: .github/workflows/ci.yml:129-156] [ASSUMED]

**Wave 0 gaps:** Action test harness, shared dual-backend command/outbox assertions, deterministic failpoints, worker restart fixture, true PG two-connection barrier, and CI Action Trino target do not yet exist in the Action placeholder.[VERIFIED: crates/ontoforge-action/src/lib.rs:1-12] [ASSUMED]

## Security Domain

`security_enforcement` 为 `true`。[VERIFIED: .planning/config.json:47-47] 适用 ASVS 类别如下；编号为安全审查分类，具体控制为本 phase 推荐。[ASSUMED]

| ASVS Category | Applies | Standard control |
|---|---|---|
| V2 Authentication | yes | 可信 host 传 `Principal`，Action 不接受 client 自报主体。[VERIFIED: crates/ontoforge-auth/src/authn/principal.rs:41-53] [ASSUMED] |
| V3 Session Management | no direct session state | 此 phase 只使用请求时当前 `PrincipalScope`；会话生命周期由宿主负责。[VERIFIED: crates/ontoforge-auth/src/scope/compile.rs:55-67] [ASSUMED] |
| V4 Access Control | yes | ①-⑨、实际行列 scope、回放时当前 action grant、无状态泄露的错误。[VERIFIED: .planning/adr/0018-authz-scope-and-sql-tiers.md:17-19] |
| V5 Input Validation | yes | IR 类型、`check_literal`、`PropertyValue`、criteria 解析限额、批大小/调用次数限额。[VERIFIED: crates/ontoforge-schema/src/ir/basetype.rs:99-165] [VERIFIED: crates/ontoforge-expr/src/analyze.rs:60-83] [ASSUMED] |
| V6 Cryptography | yes | 复用 HS256 token codec，规范输入，密钥轮换令旧 token 冲突。[VERIFIED: crates/ontoforge-query/src/lib.rs:153-225] [VERIFIED: .planning/adr/0018-authz-scope-and-sql-tiers.md:16-16] |

| Threat | STRIDE | Control / test |
|---|---|---|
| masked criteria dry-run oracle | Information Disclosure | 行级遮蔽检查先于 CEL 求值，validate-only 用同一规则；hidden vs missing 同错。[VERIFIED: .planning/adr/0018-authz-scope-and-sql-tiers.md:17-17] |
| replay after revoked role | Elevation of Privilege | 查回执前用当前 action grant 重鉴权。[VERIFIED: .planning/phases/05-online-action/05-CONTEXT.md:19-19] |
| same-key race double write | Tampering | `(principal_id, command_id)` 唯一约束和事务 winner/loser 测试。[VERIFIED: .planning/adr/0018-authz-scope-and-sql-tiers.md:19-19] |
| SQL or webhook payload injection | Tampering | `sea-query` 值绑定；webhook payload 由已存结构序列化，事件去重。[VERIFIED: crates/ontoforge-storage/src/sql_exit.rs:1-37] [ASSUMED] |

## Assumptions Log

| # | Claim | Risk if wrong |
|---|---|---|
| A1 | storage 受限事务 facade 是最小可维护的组合 API。 | async lifetime/API 复杂度需在首个 wave 试证；可改为 storage-owned closure。 |
| A2 | 旧无版本 `_version` token 在 Phase 5 部署时明确失效。 | 若已有外部客户端持久化 token，需兼容窗口。 |
| A3 | 相同 `command_id`、相同 client 请求在定义变更后回放旧 receipt，只要当前 action grant 仍存在。 | 若产品希望定义变化强制拒绝，需要在计划前锁定。 |
| A4 | batch 内同 pk 顺序按 item、rule、edit ordinal；跨 item 引用新建对象不支持。 | 若需跨 item 引用，IR/API 范围将扩大。 |
| A5 | outbox worker 用已有 `reqwest`/`wiremock`，可信目标配置、到期租约、1 秒至 5 分钟的持续退避。 | 需要测试 backoff、租约、并行 worker 与 crash recovery。 |

## Open Questions

无必须返还用户的合同空白。A2-A5 属 planner discretion 或可在执行前以测试固定的实施选择；正式 gstack `plan-eng-review` gate 仍记录为未通过，不能因本研究替代。[VERIFIED: .planning/phases/05-online-action/05-CONTEXT.md:29-31] [VERIFIED: .planning/phases/05-online-action/05-ENG-REVIEW.md:1-7,43-47]

## Project Constraints (from AGENTS.md)

仓库根目录没有实体 `AGENTS.md`；以下来自本轮用户直接提供的 `AGENTS.md instructions`，对 planner 仍有效。[VERIFIED: local `Test-Path AGENTS.md`, 2026-10-23]

- 默认简体中文；代码、路径、API、命令、错误原文保留英文；聊天禁 emoji 与 em dash。[VERIFIED: user-provided AGENTS.md instructions]
- GSD `.planning/` 是 phase 状态真相源，Plan 阶段同时用 `planning-with-files` 更新 `task_plan.md`、`progress.md`、`findings.md`；T1 只放摘要/指针，T2 放研究与计划细节。[VERIFIED: user-provided AGENTS.md instructions]
- 复杂架构先过 gstack `/plan-eng-review`；当前工程审查不是正式 gate，计划不可标记已通过。[VERIFIED: user-provided AGENTS.md instructions] [VERIFIED: .planning/phases/05-online-action/05-ENG-REVIEW.md:1-7]
- 核心业务逻辑、并发、数据处理、回归高风险任务执行 TDD；每个执行子任务使用 `ralph-loop` 并有真实测试日志，跳过 TDD 要在 `findings.md` 写原因。[VERIFIED: user-provided AGENTS.md instructions]
- 执行阶段主 session 不直接写非 trivial 实现；按通信需要路由 subagent/Agent Teams。代码修改小步、遵循现有 crate 边界，任务完成后 verify 和 review。[VERIFIED: user-provided AGENTS.md instructions]

## Sources

### Primary (HIGH)

- `.planning/phases/05-online-action/05-CONTEXT.md:17-31`, `.planning/REQUIREMENTS.md:75-86`, `.planning/adr/0016-storage-system-data-and-typed-edit-store.md:12-16`, `.planning/adr/0018-authz-scope-and-sql-tiers.md:16-19`, `.planning/adr/0026-edit-log-format.md:15-47`：锁定合同。
- `crates/ontoforge-storage/src/commit.rs:136-200,521-664,770-830`, `crates/ontoforge-query/src/lib.rs:153-225,236-424`, `crates/ontoforge-auth/src/scope/merge.rs:98-118`, `crates/ontoforge-schema/src/ir/action.rs:16-193`：当前实现。

### Official documentation (provider tier LOW; directly fetched 2026-10-23)

- https://www.postgresql.org/docs/current/transaction-iso.html ：Read Committed 的 `ON CONFLICT DO NOTHING` 并发可见性。
- https://www.postgresql.org/docs/current/explicit-locking.html ：`FOR UPDATE` 锁在事务结束释放。

## Metadata

**Confidence breakdown:** Stack HIGH (workspace declarations), architecture HIGH (ADR + source), PG conflict protocol MEDIUM (official docs + existing store pattern), worker schedule MEDIUM (planner discretion). `research-plan` 先选 Context7；本环境无 Context7 MCP/CLI，改为直读 PostgreSQL 官方文档。`classify-confidence --provider webfetch --verified` 给 LOW，因此官方网页结论只用 `[CITED]`，不升级成 `[VERIFIED]`。[VERIFIED: local tool probes, 2026-10-23]

**Research date:** 2026-10-23
**Valid until:** 2026-11-22 (repository contracts); external PostgreSQL documentation should be rechecked at execution.
