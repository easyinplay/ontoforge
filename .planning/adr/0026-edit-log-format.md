# ADR-0026 edit log 格式：三张日志表、取值编码、seq 分配、重放与 append-only

- status: accepted
- date: 2026-10-20
- decider: Claude（orchestrator 按用户「决策自主」偏好在 03-08 Task 1 checkpoint 选定 option-a 并追加补充 S1-S4；用户可事后否决，否决需按「Consequences」对已写入的日志逐行迁移）
- 细化：ADR-0016（edit store 形态）、ADR-0024（edit store 物理格式）
- 依据：`.planning/phases/03-edit-store-scope-model/03-CONTEXT.md` D-10、D-11、D-12，`03-RESEARCH.md` Pattern 3、Pattern 4、Pitfall 1、Pitfall 12、Assumption A4，`03-08-SUMMARY.md`（checkpoint 结论与研究事实），`03-09-PLAN.md`

## Context
D-10、D-11 由用户锁定并标为 one-way：log 每条 edit 只存新值和操作类型，一条记录对应一次成功提交。已写入的日志无法补出旧值，scenario / branch overlay、历史读取（03-17）、迁移归档（03-15）和 Phase 4 的叠加查询都读这份格式。03-08 在写入第一行之前经 checkpoint 定下具体列、取值编码与历史快照结构，03-09 加上 append-only 和重放并在此记录。

## Decision

### 1. 三张表
均在系统 schema（PG 的 `ontoforge_sys`）中，由 `migrations/<backend>/0002_edit_log.sql` 建立。括号内是 PG 类型。

- `of_edit_log(seq, event_id, actor, action, committed_at, def_version, params)`：`seq` 为 `INTEGER PRIMARY KEY AUTOINCREMENT`（`bigint GENERATED ALWAYS AS IDENTITY`）；`event_id` 文本唯一（ULID）；`committed_at` 文本（`timestamptz`）；`params` JSON 文本（`jsonb`），Action 参数原样存储。
- `of_edit_log_entry(log_seq, ordinal, kind, type_name, pk, op, payload, version_after)`：主键 `(log_seq, ordinal)`，`log_seq` 外键指向 `of_edit_log`；`kind ∈ {object, link}`；`op ∈ {set, unset, tombstone, release, link_add, link_remove}`（CHECK）；`pk` 对 object 是主键标量的 JSON，对 link 是 `[a, b]`；`payload` 可空：`set` 为「属性名 → 新值」，`unset` 为属性名数组，其余为空；`version_after` 为该 edit 之后行的 `of_version`。
- `of_edit_history(log_seq, ordinal, kind, type_name, pk, version, snapshot)`：主键 `(log_seq, ordinal)`，索引 `of_ix_edit_history_object (type_name, pk, version)`。没有指向 entry 的外键。

### 2. 取值的 JSON 编码
日志、历史与主键共用 `PropertyValue::to_json` 这一种编码：bool、整数、有限 double 原样；decimal 为规范十进制字符串（按属性 scale 补齐小数位，不做舍入）；date 为 `YYYY-MM-DD`；timestamp 为 UTC `YYYY-MM-DDTHH:MM:SS.fffZ`（毫秒；输入必须带偏移并换算为 UTC，小数秒超过 3 位拒绝而不截断）；array 为 JSON 数组；geopoint 为 `{"lat": .., "lon": ..}`；float 在 `PropertyValue::canonical` 中取一次最近的 f32，使日志、历史和两个数据库里是同一个数。

### 3. 补充 S1-S4（03-08 checkpoint 追加）
- **S1** timestamp 在日志、历史和主键里是 `…T…Z` 形式；SQLite 的 edit 列存 `YYYY-MM-DD HH:MM:SS.fffZ`（空格而不是 `T`），因为 `ontoforge-engine` 的 `timestamp_key` 只解析 `%Y-%m-%d %H:%M:%S`，遇到 `T` 会静默退回文本序。PG 用 `timestamptz`。三处都由同一个规范 `PropertyValue::Timestamp` 渲染。
- **S2** JSON 列（`params`、`payload`、`snapshot`）在 conformance 与重放中一律按解析后的 `serde_json::Value` 比较，不比文本：PG `jsonb` 会重排键，而工作区的 `serde_json` 开了 `preserve_order`。
- **S3** `committed_at` 由 Rust 生成（`Utc::now()`，毫秒截断，`SecondsFormat::Millis`），不用数据库时钟；`read_log` 在两个后端返回同一文本（PG 以 `to_char(.. AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"')` 读出）。
- **S4** link entry 的历史快照为 `{"deleted": bool}`；object entry 的快照为 `{"deleted", "released", "set": {属性: 值}}`，只含 edit 行状态，不含 source 值。

### 4. 实现事实（来自 03-RESEARCH 与 03-08 的核实）
- SQLite 的 sea-query binder 不支持 `Value::Array`（会 panic），所以 SQLite 的数组与 geopoint 以 JSON 文本绑定。
- PG 的数组一律以 `text[]` 绑定，再 `CAST` 为元素类型（驱动没有原生 decimal、date、timestamp 类型，一条规则覆盖所有元素类型）；decimal、date、timestamp、jsonb 标量以文本绑定并 `CAST`。
- `FOR UPDATE` 在 SQLite 的 sea-query builder 上是 no-op，所以无条件添加；SQLite 的写锁在 `BEGIN IMMEDIATE` 取得。

### 5. seq 在行锁之后分配
一次提交先按固定顺序（`RowKey`：kind、name、pk JSON）锁定或创建所有涉及的 edit 行，计算新状态，之后才插入 `of_edit_log` 取得 `seq`，再写行、entry 与快照。这样触及同一行的两次提交，其 `seq` 顺序就是它们对该行的生效顺序，按 `seq` 重放时每行的版本单调递增。
**排序告诫**：PG 的 identity 序列不按提交时间排序。涉及不同行的两次并发提交，实时 tail（`read_log(after_seq)`）可能先看到较大的 `seq`，之后才看到一个较小的 `seq`。离线重放按 `seq` 排序不受影响；实时 follower 不能假定 `seq` 的空洞已经最终确定。

### 6. 保留主体与幂等
- actor 为 `Principal("{iss}|{sub}")` 或保留主体 `system:import`（不含 `|`，不会与 principal 碰撞）。`ontoforge import` 的 actor 与 action 都写 `system:import`。
- store 层不去重：同一个 `Commit` 提交两次得到两条 log 记录，`event_id` 不同，行版本各加 1（conformance 用例 `same_commit_twice_creates_two_records`，两后端同一用例）。幂等键是 Phase 5 的 `command_id`，在 Action 层处理。

### 7. 重放语义
`ontoforge_storage::log::replay(records, target)`（D-10）：
- 目标 store 须已加载写入这些记录时的同一定义（`def_version` 相同，否则返回 `EditError::DefinitionStale`）且为空。记录按 `seq`、entry 按 `ordinal` 应用，用与提交相同的 `next_state` / `next_link_state` 计算新行，payload 经 `PropertyValue::from_json` 按当前定义解析。
- 每条 entry 之后，算出的版本必须等于记录的 `version_after`，否则返回 `ReplayError::Diverged { seq, ordinal }`。这使日志与 edit 表可以互相印证。
- 行上的 `of_log_seq` 写原 `seq`，`of_def_version` 写记录的 `def_version`；目标 store 的日志与历史表不写。
- 全部记录在一个事务中应用：失败的重放不改动目标。代价是大日志占用一个长事务，重放是离线重建，可以接受。
- 本版只在单一定义版本内重放，逐列重建。跨定义版本的重放依赖 `of_def_version.migration_json` 中由 migrate 记录的改名与归档步骤（03-15 写入）。
- `unset`、`tombstone`、`release` 与 link `remove` 的转换由 03-13 在 `next_state` 与 `update_row` 中实现，重放自动共用。

### 8. append-only 的两道防线（D-12）
1. **触发器。** 三张表都拒绝 UPDATE 与 DELETE：
   - PG：函数 `of_reject_mutation()`（plpgsql）以 SQLSTATE `23000` 抛出 `table % is append-only`；三张表各有 `BEFORE UPDATE OR DELETE ... FOR EACH ROW` 触发器和 `BEFORE TRUNCATE ... FOR EACH STATEMENT` 触发器。行触发器看不到 TRUNCATE，所以需要语句级触发器。03-16 的 `of_admin_audit` 复用该函数。
   - SQLite：三张表各有 `BEFORE UPDATE` 与 `BEFORE DELETE` 触发器，体为 `SELECT RAISE(ABORT, 'table <name> is append-only')`。`INSERT OR REPLACE` 与 `REPLACE INTO` 先删旧行，只有连接设了 `recursive_triggers = ON`（`sqlite_connect_options`，ADR-0024 第 9 点）该 DELETE 才触发；upsert（`ON CONFLICT DO UPDATE`）本身是 UPDATE。conformance 用例 `append_only_rejects_sqlite_replace_writes` 覆盖这三种写法（Pitfall 1 回归）。
   - PG 对被外键引用的表单独 TRUNCATE 时，会先报外键错误再轮到触发器，所以 TRUNCATE 用例对无引用的 entry、history 单表执行，并对三张表合并执行。
2. **部署时 REVOKE。** 触发器对表的所有者不是安全边界（`ALTER TABLE .. DISABLE TRIGGER`、`DROP TRIGGER`）。部署要求应用账号不是日志表的所有者，并对它 `REVOKE UPDATE, DELETE, TRUNCATE`（只留 INSERT 与 SELECT）；文档由 03-20 写入。

### 9. sqlx 迁移与触发器（Assumption A4）
sqlx 迁移原样执行了含 `$$` 的 plpgsql 函数和含 `BEGIN … END;` 的 SQLite 触发器脚本，两后端 conformance 实测通过，所以不需要 A4 的回退（把触发器语句放进 `sql_exit.rs`、由 `open` 在迁移之后逐条执行）。`store.rs` 与 `sql_exit.rs` 未改动。

## Rejected
- **在 entry 中存旧值（或整行前像）**：D-10 已锁定只存新值。历史读取靠快照而不是日志里的前像。
- **先分配 seq 再写行**：并发提交触及同一行时 `seq` 顺序可能与生效顺序相反，按 `seq` 重放会得到错误的版本序列，`Diverged` 检查也就失去意义。
- **只靠应用层、不加触发器**：任何持有 INSERT 权限的进程或一条手工 SQL 都能改写日志；D-12 要求两个后端都由数据库拒绝。
- **用数据库时钟写 `committed_at`**：SQLite 与 PG 的格式和精度不同，两后端文本不会逐字相等；S3 在 Rust 一处生成。
- **JSON 列按文本比较**：PG `jsonb` 重排键，会让跨后端比较和重放校验出现假差异（S2）。

## Consequences
- 03-13 实现完整转换表后，`replay` 自动覆盖 unset、tombstone、release 与 link remove，需要补对应的 replay conformance 用例。
- 03-15 的 migrate 把改名与归档步骤写入 `migration_json` 后，才能扩展到跨定义版本的重放。
- 03-17 的历史读取基于 `of_edit_history` 与索引 `of_ix_edit_history_object`。
- 03-16 的 `of_admin_audit` 复用 `of_reject_mutation()`。
- 改动本 ADR 的任何一项（列、取值编码、快照结构、`seq` 分配时机、保留主体）都需要迁移已写入的日志，而日志本身是不可改写的；实际做法只能是新增版本化的格式并让读取方兼容两者。
