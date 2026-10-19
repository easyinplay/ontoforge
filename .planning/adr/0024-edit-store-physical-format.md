# ADR-0024 edit store 物理格式：`__set` 布尔列、命名、取值编码与 schema 划分

- status: accepted
- date: 2026-10-20
- decider: Claude（orchestrator 按用户「决策自主」偏好在 03-03 Task 1 checkpoint 选定 option-a；用户可事后否决，否决需按本 ADR 的「Consequences」逐表迁移）
- 细化：ADR-0016（edit store 形态）
- 依据：`.planning/phases/03-edit-store-scope-model/03-CONTEXT.md` D-15、D-17，`03-RESEARCH.md` Pattern 1、Open Question 1 / 2 / 5 与 Assumption A9，`03-03-PLAN.md` Task 1

## Context
ADR-0016 规定每个 object type 一张强类型 edit 表、按属性 patch、用 `set_mask` 标记有效列，但没有定死物理编码。这一层是 one-way 决定：PG 里的 edit 表对 Trino 可见，Phase 4 的叠加查询（形如 `col__set = false OR col = v` 的谓词）、03-08 的 commit 写路径、03-15 的归档与改名、`of_def_version.physical_json` 都读它；上线后改编码要对每张 edit 表做数据迁移并重写存档的物理 schema。

## Decision
1. **set_mask 编码：每个非主键属性一个 `<col>__set` 布尔列。** 值列为 NULL 可空，`__set` 为 `NOT NULL DEFAULT false`（SQLite 为 `INTEGER NOT NULL DEFAULT 0`）。主键列不带 `__set`：一行存在即说明主键已设置。不使用 `set_mask bigint` 位图，也不建「属性到位槽」的 `of_property_slot` 注册表。
   理由：没有属性个数上限，也没有槽位注册表与对应 lint；改名与归档对 `<col>__set` 就是和 `<col>` 一起做的普通列操作；Phase 4 的谓词是普通布尔比较，Trino 的 PG connector 可以下推；RESEARCH Open Question 1 与 Assumption A9 推荐此方案，下游 03-04 / 08 / 10 / 13 / 15 / 19 已按它假设。
2. **表名。** object type：`of_obj_<snake(apiName)>`；M:N link type：`of_lnk_<snake(apiName)>`；snake 化用 `heck::ToSnakeCase`。前缀 `of_` 遵守 ADR-0002 第 5 条。oneToOne / oneToMany 的外键是 `to` 端 type 的普通属性列，不另建表。
3. **列名。** 属性列 = `snake(apiName)`；标志列 = `<col>__set`（snake 化结果不含连续下划线，故不与属性列冲突）；归档列 = `<col>__arch_v<N>`，其 `__set` 列同步归档（N 为触发归档的 `def_version`，03-15 实现）。
4. **系统列**（一律 `of_` 前缀，因为 `version`、`deleted` 是常见的用户属性名）：`of_version`（bigint，未编辑为 0，首次 edit 为 1，单调递增永不重置）、`of_deleted`、`of_released`、`of_def_version`、`of_log_seq`（可空）。link 表为 `(a_pk, b_pk, of_deleted, of_version, of_def_version, of_log_seq)`，主键 `(a_pk, b_pk)`，没有 `of_released`；`a_pk` 对应 `from` 端主键，`b_pk` 对应 `to` 端主键，类型与各端主键相同。
5. **物理名长度上限**（PG 标识符 63 字节，03-10 实现）：表名 snake 部分至多 29 字节、列名至多 30 字节、索引名至多 63 字节。超出时取前缀 + `_` + 完整名的 FNV-1a 32 位哈希（8 位十六进制）。截断后仍碰撞、两个逻辑名映射到同一物理名、或与系统列同名，一律报 ONT-V24（沿用 03-02 的编号，不用 RESEARCH 里暂拟的 E18 / E19）。
6. **索引命名**：普通索引 `of_ix_<table>_<col>`，oneToOne 外键的部分唯一索引 `of_ux_<table>_<col>`。
7. **取值编码与列类型。**

   | BaseType | PostgreSQL | SQLite |
   |---|---|---|
   | byte, short | `smallint` | `INTEGER` |
   | integer | `integer` | `INTEGER` |
   | long | `bigint` | `INTEGER` |
   | float | `real` | `REAL` |
   | double | `double precision` | `REAL` |
   | boolean | `boolean` | `INTEGER`（0 / 1） |
   | string | `text COLLATE "C"` | `TEXT`（BINARY） |
   | decimal(p, s) | `numeric(p, s)` | `TEXT COLLATE ontoforge_decimal` |
   | date | `date` | `TEXT`（`YYYY-MM-DD`） |
   | timestamp | `timestamptz` | `TEXT COLLATE ontoforge_timestamp`（UTC 规范文本） |
   | array | 元素类型加 `[]`（原生数组） | `TEXT`（JSON 数组文本） |
   | geopoint | `jsonb` | `TEXT`（JSON 文本 `{"lat":..,"lon":..}`） |
   | `<col>__set` | `boolean NOT NULL DEFAULT false` | `INTEGER NOT NULL DEFAULT 0` |

   SQLite 的 decimal 不用 sea-query 的 `decimal_len`：它会给列 NUMERIC 亲和性，文本被转成浮点。两个 collation 由 `ontoforge-engine` 导出，SQLite 连接选项与替身引擎共用同一实现（03-01）。SQLite 的 `of_version`、`of_deleted`、`of_released`、`of_def_version` 都带 `DEFAULT 0`：SQLite 的 `ADD COLUMN` 不接受没有默认值的 `NOT NULL` 列，带默认值才能让 reload 补齐中断后缺列的表；写入路径总是显式设置 `of_def_version`，默认值不会被依赖。
8. **PG 两个 schema。** `ontoforge_edit` 放 `of_obj_*` / `of_lnk_*`，Trino 连接账号只授 `USAGE` 与 `SELECT`；`ontoforge_sys` 放定义、`_sqlx_migrations`、日志、历史、user、role、审计，Trino 账号无权限。schema 名可配置（`[store] schema_edit` / `schema_sys`）。测试用唯一 schema 名隔离。
9. **SQLite 为独立文件。** edit store 不 `ATTACH` 到引擎替身，也不与替身共用连接；每个连接 `recursive_triggers = ON`，源码中禁止 `OR REPLACE` / `REPLACE INTO`（`tests/sql_exit_guard.rs` 扫描）。
10. **per-type 表不走 sqlx 迁移。** `sqlx::migrate!("./migrations/<backend>")` 只管系统表（`of_def_state`、`of_def_version` 以及后续的日志与 scope 表）；edit 表由 `reload` 按定义生成，其物理 schema 存入 `of_def_version.physical_json`，与 `ontology_json`、`migration_json` 同表保存。
11. **reload 的分类。** 新 object type、新 M:N link、可空属性是加法，自动执行；删属性、删 type、改属性类型（含 decimal 精度、byte 与 short 之分）、改主键、新增 required 属性、link 端点变化一律是破坏性，报 ONT-V25 并指向 `ontoforge migrate --plan`，不执行任何 DDL。IR 没有 `default`，所以新增 required 属性一律破坏性。改名在 diff 上表现为删加一对（D-14）。
12. **SQL 文本出口。** 发往数据库的 SQL 只经 `ontoforge-storage/src/sql_exit.rs`，那里是唯一的 `AssertSqlSafe` 调用点；DDL 与 DML 文本来自 sea-query，标识符由它加引号（T-03-07）。

## Rejected
- **单个 `set_mask bigint` 位图加 `of_property_slot` 注册表**：每个 type 至多 63 个属性（含归档槽，槽永不复用），需要额外的注册表与 lint；位运算谓词在 Trino 的 PG connector 上不下推，Phase 4 列表路径更慢。与 spike 3b 的实验形态一致不足以抵消这些成本。
- **edit store `ATTACH` 到引擎替身**：两个子系统共用连接与事务边界，替身的只读保证与 edit store 的写入互相牵制。
- **系统列用裸名 `version` / `deleted`**：与常见用户属性名冲突。
- **通用 JSONB 行**：ADR-0016 已否决（Trino 经 PG connector 读取时谓词不下推）。

## Consequences
- 03-04 的 PG 方言按本表的类型列实现，并为两种方言加 DDL 快照；03-08 的 commit 写路径以 `<col>__set` 记录按属性 patch；03-10 实现长度上限、碰撞检查（ONT-V24）与索引；03-15 的 migrate 用 `RENAME COLUMN` 同步移动 `<col>` 与 `<col>__set`。
- 每个属性多一列，edit 表比位图方案宽；换来的是没有属性数上限、没有注册表。
- 改动上面任何一项（编码、命名、系统列、取值编码、schema 划分），都需要对每张 edit 表做数据迁移，并重写所有已存档的 `physical_json`。
