# ADR-0016 存储：PG 存系统数据与按类型强类型的 edit store

- status: accepted
- date: 2026-10-16
- supersedes: ADR-0004
- 依据：`docs/designs/trino-engine-sql-tiers.md`、`.planning/research/spike-03-trino-overlay-latency.md`、Eng Review D3 / D15 / D17 / D19 / D20 / D22

## Context
业务数据改由 Trino 读取企业已有数据源（ADR-0017）。PG 只保存 ontology 系统自身的数据和 Action 写入的 edit。spike 3 实测：经 Trino PG connector 读取时，JSONB 谓词不下推（json_extract 拉回 + Trino 侧解析），同一过滤强类型表中位 112ms、JSONB 6,465ms。

## Decision
- **PG（生产）/ SQLite（dev、test、`serve --dev`）** 存：定义版本、用户 / 组 / 角色与 scope、edit store、edit log、`action_commands`、webhook outbox、读审计。`sqlx` + `sea-query`。
- **edit store 形态**：每个 object type 一张强类型表、每个 M:N link type 一张 link edit 表 `(a_pk, b_pk, deleted, version)`，DDL 由定义生成；另有 append-only 历史表（JSONB 快照，只用于审计与按版本取历史，不参与过滤）。text 列一律 `COLLATE "C"`。索引：pk + 行谓词引用的属性 + 声明 `indexed: true` 的属性。
- **叠加粒度**：按属性 patch。edit 行只存 Action 设置过的列，`set_mask` 标记有效列；未设置的属性始终读 source。tombstone 压制 source 行且不会「复活」；release 是显式状态，版本号单调递增、永不重置。
- **定义变更**：新增类变更（新 type、可空或带 default 的属性、索引）在 reload 事务内自动执行；破坏性变更（删属性、改类型、改 pk、无 default 的 required 新属性）被 lint 拦下，须 `ontoforge migrate --plan/--apply`，旧列先归档。reload 加固：DDL `lock_timeout`；提交后调用 Trino `flush_metadata_cache` 并探测新列可见，确认后才切换定义；edit 行记录 `def_version`，旧定义的写入在栅栏后被拒绝。
- **写入**：只经 Action。单个 PG 事务内写 edit 行、edit log、`action_commands`、webhook outbox（至少一次投递，`event_id` 幂等）。
- **审计承诺**：写入审计与事务同生同灭；读审计经有界队列异步批量落库，进程崩溃时至多丢约 1 秒，积压或持续写失败时读请求 fail-closed（`AuditUnavailable`）。
- edit store catalog 在 Trino 侧不得开启数据 / 结果缓存（spike 3 已验证无缓存时提交后下一条查询可见）。

## Rejected
- 通用 JSONB 表（ADR-0004 原方案）：经 Trino 读取时过滤不下推，慢约 58 倍。
- 强类型列加 JSONB `extra` 兜底：同一属性分两处存，`extra` 内属性仍不下推。
- 整行快照永久覆盖 source：被编辑过一次的对象从此脱离 ETL 更新。
- 读审计同步落库：每次读多一次 PG 写入，挤占 PG 直读 p95 < 50ms 的预算。

## Consequences
REQUIREMENTS 的 STORE-02 改为「新增类变更无需手写 DDL；破坏性变更经 migrate」。业务数据导入（原 STORE-06）由 Trino ETL（L3）承担，`ontoforge import` 只导 edit-only 对象与替身引擎数据。
