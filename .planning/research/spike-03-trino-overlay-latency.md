# Spike 03（部分）：Trino 叠加读取的下推与延迟实测

- 日期：2026-10-16
- 对应：`docs/designs/trino-engine-sql-tiers.md`「Gates before /plan-eng-review」第 3 项
- 环境：生产 Trino 483 集群（3 节点，`system.runtime.nodes`），经 MCP 只读访问；全程只跑 SELECT / SHOW / EXPLAIN，所有查询都带 pk 等值或 LIMIT
- 状态：**部分完成**。集群没有 `postgresql` catalog，PG connector 相关项（JSONB 谓词下推、读后写可见性）无法在此验证，见「未验证项」
- 通用性约定：下文 catalog / schema / 表 / 列名一律为占位符，不含任何真实名称与数据值

## 1. 测试对象

| 占位名 | connector | 规模 | 说明 |
|---|---|---|---|
| `ss_cat.schema_a.table_a` | sqlserver（base-jdbc） | 约 2.9M 行（`SHOW STATS`） | `id` 为 identity 整型，近似唯一 |
| `ss_cat.schema_a.table_dim` | sqlserver | 14 行 | 状态维表，`code` 与 `table_a.col_status` 关联 |
| `ice_cat.schema_b.table_b` | iceberg（Nessie catalog） | 约 4.5M 行，**1 个 parquet 文件约 141MB，未分区** | format v2，copy-on-write，`id` 整型 |

选 sqlserver 的理由：sqlserver / mysql / postgresql 共用 base-jdbc 框架（谓词 / TopN / 聚合 / join 下推的规则引擎相同，差异在各 connector 的表达式改写能力），可作为 PG connector 的间接证据。

## 2. 下推结论（EXPLAIN TYPE DISTRIBUTED / IO）

| 谓词 / 操作 | sqlserver（JDBC） | iceberg |
|---|---|---|
| (a) pk 等值 `id = ?` | **下推**：`TableScan[... constraint on [id]]`，无独立 Filter 节点 | 约束传入 connector（IO plan 有 `columnConstraints id = EXACTLY`），用于文件 / row group min-max 裁剪；`ScanFilter` 仍保留残余过滤，估算按全表 4.5M 行 |
| (b) 范围 `id BETWEEN` | **下推**：`constraint on [id]` | 同 (a)，仅统计裁剪 |
| (c) `LIKE '%x%'` / `LIKE 'x%'` | **不下推**：`ScanFilterProject ... filterPredicate = $like(...)`，Trino 侧过滤（该列在源端为 `nvarchar`） | 不下推到存储；前缀 LIKE 被改写为范围 `'x' <= col < 'y'`，可参与统计裁剪 |
| (d) LIMIT / TopN | `ORDER BY id DESC LIMIT 50`：**整体下推**（`sortOrder=[id DESC] limit=50`）；但 `LIKE + LIMIT` 时 LIMIT 只在 Trino 侧 `LimitPartial` | 不下推，`TopNPartial` 在 Trino 侧 |
| (e) 聚合 `GROUP BY + count + max` | **下推**：`TableScan[Query[SELECT ..., count_big(*), max(id) ... WHERE ... GROUP BY ...]]` | 不下推（`count(*)` 无谓词时 TableScan 无列，靠元数据加速） |
| 同 catalog JOIN（fact pk 过滤 × dim） | **下推**：整个 join 变成单条远端 SQL `... INNER JOIN ... WHERE id = ?` | — |
| 跨 catalog JOIN（ice pk 过滤 × ss fact，等值键） | 不能 join 下推；iceberg 侧 BROADCAST 到 ss 侧，`InnerJoin distribution = REPLICATED`，ss 侧带 `dynamicFilters` | build 侧 |
| 跨 catalog JOIN（ice pk 过滤 × ss dim） | dim 侧带 `dynamicFilters = {code = #df}` | build 侧 |

关键发现：

1. **join 键类型必须严格一致**。iceberg 侧键为 `varchar`、ss 侧为 `varchar(2000)` 时，计划中动态过滤挂在表达式上：
   ```
   ScanFilterProject[table = ss_cat:schema_a.table_a, dynamicFilters = {CAST(col_key AS varchar) = #df_336}]
   ```
   把 cast 移到 build 侧（`CAST(b.col_key AS varchar(2000)) = a.col_key`）后，动态过滤直接落在列上：
   ```
   ScanFilter[table = ss_cat:schema_a.table_a, dynamicFilters = {col_key = #df_337}]
   ```
   前者是否还能转成 JDBC 下推的 `WHERE col_key IN (...)` 未实测（实测需要可能全表拉取 2.9M 行，按只读约束跳过）。ontoforge 编译叠加 SQL 时应**按绑定规范显式对齐键类型，cast 放在小表侧**。
2. JDBC 动态过滤等待：`ss_cat.dynamic_filtering_enabled = true`，`dynamic_filtering_wait_timeout = 20.00s`（JDBC 扫描会等 build 侧完成再发远端 SQL）；iceberg 为 `1.00s`。叠加 JOIN 时 edit store 若在 probe 侧，会等 source 侧 build 完成。
3. iceberg pk 查找没有 pk 索引：未分区 + 单文件 + 无排序时，每次都要读整个文件的相关列（见第 3 节延迟）。

## 3. 延迟实测

方法：每类查询 5 次，查询文本带唯一 tag，从 `system.runtime.queries` 取 `queued_time_ms`、`analysis_time_ms`、`planning_time_ms`、`end - created`（服务端视角，不含客户端 HTTP 轮询与结果传输）。pre = queued + analysis + planning。filter 查询外层套 `count(*)` 以压缩输出（内层 TopN / LIMIT 计划不变）。

| 查询 | pre min / med / max (ms) | total min / med / max (ms) |
|---|---|---|
| A. ss pk 查找 `WHERE id = ?` | 94 / 97 / 160 | 171 / 318 / 1027 |
| B. iceberg pk 查找 `WHERE id = ?` | 17 / 20 / 29 | 716 / 831 / 1390 |
| C. ss 选择性过滤 `col_status = ? AND col_cat > ? ORDER BY id DESC LIMIT 50` | 97 / 104 / 119 | 234 / 416 / 1474 |
| D. iceberg 选择性过滤 `col_sku = ? AND col_flag = ? LIMIT 50` | 29 / 31 / 60 | 130 / 247 / 845 |
| E. 跨 catalog pk join（ice pk 过滤 × ss fact，cast 对齐） | 128 / 136 / 154 | 603 / 667 / 1016 |

观察：

- queued 全部为 0（测试时集群空闲，**生产繁忙时段的排队时间未覆盖**）。
- JDBC catalog 的 analysis 稳定在 65–85ms，明显高于 iceberg 的 10–18ms，推测是每次分析都向 SQL Server 取元数据（表 / 列 / 统计），可通过 connector 的 `metadata.cache-ttl` 降低（只读无法确认当前配置）。
- 每组都有 1 次约 1s 的离群值，多为首次执行（冷连接 / 冷缓存）。
- iceberg pk 查找由执行阶段主导（700ms+），原因是全文件扫描；iceberg 选择性过滤 + LIMIT 反而更快，因为 LIMIT 命中即提前结束。

## 4. 对照预算

| 预算 | 判断 |
|---|---|
| 按 pk 取对象 p95 < 500ms | JDBC source：中位 318ms 达标，但 5 次里 1 次 1027ms，p95 **有风险**；iceberg source：**不达标**（中位 831ms），且随表规模线性变差 |
| 常见过滤查询 p95 < 2s | 两类 source 均**达标**（最大 1474ms），前提是谓词可下推或有 LIMIT 提前结束 |
| Action 端到端 p95 < 1s | 一次 Action 至少「叠加读当前状态（1 次 Trino 查询）+ PG 写事务」。叠加读本身就是 source × edit store 的跨 catalog join（形态同 E，中位 667ms），加上客户端轮询与 PG 写，**很紧，大概率超标**，不能每次 Action 都走 Trino 叠加读 |

主导因素：planning（analysis + planning）在 JDBC 上约 100–150ms，是固定税；剩余主要是执行（远端查询 / 文件扫描）与冷启动离群。纯规划不是瓶颈，**执行形态（能否下推、源端是否有索引 / 布局）决定能否达标**。

## 5. 未验证项（需要测试用 PG catalog）

1. **JSONB 属性谓词下推**：PG connector 对 `json` / `jsonb` 列默认按 `json` 类型映射，`json_extract_scalar` 等函数不在 base-jdbc 的下推表达式集合内，预期**不下推**，需实测 `EXPLAIN` 确认。
2. **读后写可见性**：PG 提交后下一条 Trino 查询能否立即读到；需确认 edit store catalog 未开 `metadata.cache-ttl` 之外的数据缓存，且无 `query.max-...` 类结果缓存。
3. PG connector 上 `LIKE` 下推：PG connector 对 varchar 的 LIKE 改写能力与 sqlserver 不同，需单独确认（sqlserver 上不下推）。
4. 叠加 SQL 形态（`FULL OUTER JOIN` vs `UNION ALL` + 窗口去重）在 PG catalog 上的计划：edit store 侧是否带 pk 约束或动态过滤，是否会全表扫描。
5. 跨 catalog join 中键类型不一致时，动态过滤是否仍能下推到 JDBC（见 2.1）。
6. 繁忙时段排队时间与 p95（本次样本 n=5，只能看趋势）。

## 6. 对 edit store 物理形态的建议

1. **按 object type 分表（或至少按 type 分区 + 关键属性为真实列），不要用「通用 JSONB 宽表」承载叠加读取**。理由：base-jdbc 的下推只覆盖列上的简单比较、IN、范围、TopN、聚合；JSONB 表达式预期无法下推，会导致 Trino 把 edit store 拉全量再过滤。真实列（或 PG 生成列 + 索引）可以让 `id = ?`、范围、`IN (dynamic filter)` 全部下推到 PG 并走索引。
2. **edit store 每张表以 pk 建唯一索引**，叠加 SQL 中 pk 谓词同时作用于 source 与 edit store 两侧（不要只写在 join 之后）；编译器显式对齐 join 键类型，cast 放小表侧。
3. **Action 读当前状态不走 Trino**：先在 PG 内读 edit store（同事务，`SELECT ... FOR UPDATE` 配 `_version`），只有 edit store 无记录时才经 Trino 读 source 行；这样 Action 的 Trino 调用退化为 source 侧 pk 查找（JDBC 中位 ~300ms），并天然解决读后写一致。
4. **iceberg source 的 pk 查找需要布局支持**：要求（或在 lint 中提示）source 表按 pk 排序写入 / bucket 分区 / 开 parquet bloom filter，否则 pk 查找预算不可达；也可对 iceberg source 的 get-by-pk 接受「中位 < 1s」的降级目标，由用户在公司环境实测后调整（设计文档已允许）。
5. JDBC catalog 建议部署文档要求开 `metadata.cache-ttl`（降 analysis 固定税），但 edit store catalog 的元数据缓存不影响数据可见性，数据缓存必须关闭。

## 7. 观测到的 JDBC catalog 会话属性（只读，`SHOW SESSION`）

```
<ss_cat>.aggregation_pushdown_enabled      = true
<ss_cat>.join_pushdown_enabled             = true
<ss_cat>.join_pushdown_strategy            = AUTOMATIC
<ss_cat>.complex_join_pushdown_enabled     = true
<ss_cat>.topn_pushdown_enabled             = true
<ss_cat>.dynamic_filtering_enabled         = true
<ss_cat>.dynamic_filtering_wait_timeout    = 20.00s
<ss_cat>.domain_compaction_threshold       = 256
<ss_cat>.unsupported_type_handling         = IGNORE
<ice_cat>.dynamic_filtering_wait_timeout   = 1.00s
enable_dynamic_filtering                   = true
```

全部为默认值。`domain_compaction_threshold = 256` 意味着 `IN` 列表或动态过滤超过 256 个值时会被压缩成范围，批量按 pk 读取（如 link 遍历）时需注意分批大小。

## 附录：PG 侧 edit store 实测（2026-10-16，测试 PG 16.14，`ontoforge` 库）

数据：通用快照表 `edit_store.object_edits`（20 万行，2 个 object type，JSONB 属性，`(object_type, pk, version)` 主键，`properties->>'status'` 表达式部分索引）；最新版本视图 `object_current`（`DISTINCT ON`）；对照的按类型分表 `edit_store.flight_current`（5 万行，强类型列 + 索引）。均 `ANALYZE` 后 `EXPLAIN (ANALYZE, BUFFERS)`。

| 查询 | 计划 | 执行时间 |
|---|---|---|
| A：JSONB 表达式等值 + LIMIT 50 | 表达式索引扫描 | 0.22 ms |
| A：JSONB 数值比较 `(props->>'x')::int > N` | 借道部分索引顺扫后过滤（无专用索引） | 0.56 ms |
| A：当前版本视图按 pk | 主键索引反向扫描 | 0.13 ms |
| A：当前版本视图 + 属性过滤 | 增量排序 + Unique 后过滤（先取最新版再过滤，语义正确但随数据量线性变差） | 0.74 ms |
| B：分表按 pk | 主键索引扫描 | 0.07 ms |
| B：分表按强类型列过滤 | 普通索引扫描 | 0.11 ms |

结论：
- PG 内部两种形态都在亚毫秒级，**edit store 本身不是延迟瓶颈**；瓶颈在 Trino 规划与 source 侧执行（正文延迟表）。
- 通用 JSONB 表的「当前版本 + 属性过滤」必须先去重再过滤，无法用属性索引直接定位，数据量增大后最先劣化；且经 Trino JDBC 读取时 JSONB 谓词预期无法下推（仍待 PG catalog 实测）。
- 支持正文建议：**edit store 按 object type 分表，属性为强类型列（由 ontology 定义生成 DDL），保留 `version` / `deleted` 与 append-only 历史表**；Action 前置读在 PG 事务内先读 edit store，未命中再经 Trino 读 source。
- 代价：违背原 STORE-02「加载新 ontology 无需 DDL」，ontology 变更需要生成迁移 —— 交给 `/plan-eng-review` 与 ADR-0004 修订决定。

仍待 Trino 侧实测（需在 Trino 上挂测试 PG catalog）：JDBC 读 edit store 的谓词下推（强类型列 vs JSONB）、`LIKE` 下推、读后写可见性、叠加 SQL 的整体计划与延迟。

## 附录 2：Trino 经 PG connector 实测（2026-10-16）

环境：同一 Trino 483 集群新挂 `pg_cat`（postgresql connector，只读角色）指向测试 PG 16 的 `ontoforge` 库 `edit_store` schema（数据同附录 1）。写入一律经 psycopg（autocommit）直连 PG；Trino 侧只跑 SELECT / EXPLAIN / EXPLAIN ANALYZE。**本轮测试时集群繁忙**（10 分钟窗口内约 1100 条查询、同时 RUNNING 约 50 条），与正文第 3 节的空闲时段不同，因此本附录同时重测了非叠加基线作对照。

### A2.1 类型映射与下推

PG `jsonb` 在 Trino 中映射为 `json`（`properties:json:jsonb`），`uuid` / `timestamptz` 均有原生映射。`pg_cat.enable_string_pushdown_with_collate = false`（默认，未改；MCP 只读通道无法 `SET SESSION`），其余会话属性与 sqlserver catalog 相同（TopN / 聚合 / join 下推开，动态过滤等待 20s）。

| 谓词 / 操作 | 计划结论 |
|---|---|
| `flight_current` pk 等值 | **下推**：单 fragment `TableScan[... constraint on [pk]]` |
| 状态等值 + 整型范围 + `ORDER BY delay_minutes DESC LIMIT 50` | **整体下推**：`constraint on [delay_minutes, status] sortOrder=[delay_minutes DESC] limit=50` |
| `gate LIKE '%1%' LIMIT 50` | **下推**（与 sqlserver 不同）：`constraints=[ParameterizedExpression[("gate") LIKE (?)]] limit=50` |
| `gate LIKE 'G1%' LIMIT 50` | LIKE 下推，但 Trino 同时把它改写成 `'G1' <= gate < 'G2'` 的残余 `filterPredicate`（varchar 范围在 collate 关闭时不能下推），LIMIT 退回 `LimitPartial` |
| `GROUP BY status`（varchar 键）+ `count(*)` | 聚合**不下推**（varchar 分组键同样受 collate 开关限制），只有 `delay_minutes > 100` 下推 |
| `object_edits` 上 `json_extract_scalar(properties,'$.status') = 'delayed'` | **不下推**：`ScanFilterProject ... filterPredicate = (json_extract_scalar(...))`，只有 `object_type` 约束下推，估算拉回该 type 全部 9 万行 |
| `CAST(json_extract(properties,'$.delayMinutes') AS integer) > 100` | **不下推**，同上 |
| `object_edits` pk 取最新版 `ORDER BY version DESC LIMIT 1` | **整体下推**（`constraint on [object_type, pk] sortOrder=[version DESC] limit=1`） |
| 视图 `object_current`（DISTINCT ON）按 pk | 下推进视图（`constraint on [object_type, pk]`），PG 端走主键 |
| 视图 `object_current` + JSON 谓词 + LIMIT | 不下推；EXPLAIN ANALYZE：拉回 10,655 行（1MB），JSON 解析 CPU 2.61s，Execution 6.66s |

结论：PG connector 对强类型列的下推能力强于 sqlserver（LIKE 也能下推），但**任何 JSON 表达式都不下推**，且 Trino 的 `json` 类型要在 Trino 侧重新解析文本，代价是 CPU 秒级。附录 1 的预期得到证实。

### A2.2 读后写可见性

每轮：psycopg autocommit 更新 `flight_current` 一行（`gate` 写唯一 marker、`version + 1`）并向 `object_edits` 插入新版本，提交后立即经 Trino 读同一 pk（同一查询同时读 `flight_current` 与 `object_current` 视图）。

| 轮次 | 提交 → Trino 查询创建 | Trino 读到 marker | 该次读服务端耗时 |
|---|---|---|---|
| 1 | 4.85s | 是（两张表都是新值） | 14,193ms |
| 2 | 2.27s | 是 | 518ms |
| 3 | 2.42s | 是 | 1,486ms |
| 4 | 2.15s | 是 | 10,371ms |
| 5 | 2.20s | 是 | 881ms |

- 5/5 首次读取即可见，`version` 与 marker 一致；Trino 对 PG connector 无数据缓存，提交即可见，**设计文档第 2 节「读后写一致」前提成立**。
- 间隔 2s 左右是 MCP 调用往返（本环境无法更紧地串联），不是可见性延迟；PG 事务语义保证提交后的新连接一定可见。
- 该查询是标量子查询 + cross join（多 fragment），读本身的耗时离群（10–14s）与 A2.3 的多 fragment 现象一致。

### A2.3 叠加查询形态与延迟

测试数据：在 `flight_current` 中插入 5 行，pk 取 source 表真实 id 的字符串：1 行编辑（status/gate 改写）、2 行 tombstone、1 行编辑后不再满足过滤、1 行 edit-only（source 无此 id）。这些行保留在测试库中。

pk 查找形态（LEFT JOIN + edit-wins；两侧都显式写 pk 谓词，cast 放 source 侧的单行上）：

```sql
SELECT s.id,
       CASE WHEN e.pk IS NOT NULL THEN e.status ELSE CAST(s.col_status AS varchar) END AS status,
       e.gate, COALESCE(e.version, 0) AS _version
FROM (SELECT id, col_status FROM ss_cat.schema_a.table_a WHERE id = ?) s
LEFT JOIN (SELECT pk, version, deleted, status, gate
           FROM pg_cat.edit_store.flight_current WHERE pk = ?) e
  ON e.pk = CAST(s.id AS varchar)
WHERE e.deleted IS NOT TRUE
```

计划：两侧 pk 约束都下推（source 1 行、PG 0–1 行），但 join 为 `LeftJoin distribution = PARTITIONED`（PG 侧无统计，估算 `rows: ?`），产生 3 个 fragment / 多 task。语义验证：编辑行返回 edit 值且 `_version = 1`；tombstone 返回空；无 edit 返回 source 值且 `_version = 0`。

过滤形态（LEFT JOIN 只能表达「source 存在」的对象，过滤读必须补 edit 分支，故用 anti-join + UNION ALL）：

```sql
SELECT id, status, gate, _version FROM (
  SELECT s.id, CAST(s.col_status AS varchar) AS status, CAST(NULL AS varchar) AS gate, BIGINT '0' AS _version
  FROM (SELECT id, col_status FROM ss_cat.schema_a.table_a
        WHERE col_status = 2 AND col_cat > 400 ORDER BY id DESC LIMIT 100) s   -- over-fetch: 50 + edit 上界
  LEFT JOIN pg_cat.edit_store.flight_current e ON e.pk = CAST(s.id AS varchar)
  WHERE e.pk IS NULL                                                            -- 未被编辑的 source 行
  UNION ALL
  SELECT CAST(e.pk AS integer), e.status, e.gate, e.version
  FROM pg_cat.edit_store.flight_current e WHERE e.status = '2' AND NOT e.deleted -- edit 胜出且满足过滤
) ORDER BY id DESC LIMIT 50
```

计划：source 侧 `constraint on [col_status, col_cat] sortOrder=[id DESC] limit=100` 整体下推；anti-join 的 PG 侧带 `dynamicFilters = {pk = #df}`（source 的 100 个 id 作为 IN 列表下推到 PG）；edit 分支 `constraint on [deleted, status]` 下推走索引。语义验证：tombstone 被压制、被编辑出过滤范围的行消失、edit-only 行出现。**注意**：source 内层 LIMIT 下推必须 over-fetch（N + 可能被压制的 edit 数），否则 anti-join 后不足 N 行；over-fetch 上界由编译器从 edit store 计数得出。

延迟（服务端，方法同正文第 3 节；同一繁忙时段内对照）：

| 查询 | pre min / med / max (ms) | total min / med / max (ms) |
|---|---|---|
| 基线 A'：source pk 查找（单 fragment） | 85 / 93 / 131 | 171 / 735 / 774 |
| 基线 PG'：edit store pk 查找（单 fragment） | 50 / 57 / 103 | 62 / 102 / 750 |
| 叠加 pk 查找（LEFT JOIN，5 次） | 178 / 197 / 392 | 787 / 2,624 / 20,471 |
| 基线 C'：source 过滤 + TopN 50 | 99 / 107 / 127 | 228 / 519 / 1,179 |
| 叠加过滤（anti-join + UNION ALL，LIMIT 50，5 次） | 172 / 194 / 219 | 2,085 / 3,873 / 4,439 |

EXPLAIN ANALYZE（叠加 pk 查找一次）：source 扫描 66ms、PG 扫描 1.6ms，但 Execution 3.56s，时间几乎全部是 join stage 的 `Blocked`（等待上游 task 调度与 exchange）。**数据访问不是瓶颈，多 stage 调度在繁忙集群上才是**；单 fragment 查询（基线 A' / PG'）在同一时段仍在 1s 内。

| 预算 | 判断 |
|---|---|
| pk p95 < 500ms | 叠加 pk 查找**不达标**（中位 2.6s，最大 20s）；单 fragment 基线也只有中位达标 |
| 过滤 p95 < 2s | 叠加过滤**不达标**（最小 2.1s）；基线 C' 达标 |

### A2.4 强类型表 vs JSONB 表（同一逻辑过滤）

`status = 'delayed' AND delay > 100 AND NOT deleted LIMIT 50`，外层 `count(*)`，各 5 次交替执行：

| 形态 | pre min / med / max (ms) | total min / med / max (ms) |
|---|---|---|
| `flight_current` 强类型列（全部下推，单 fragment） | 56 / 60 / 70 | 68 / 112 / 844 |
| `object_current` 视图 + JSON 谓词（不下推，两 fragment） | 71 / 74 / 162 | 525 / 6,465 / 16,308 |

中位相差约 58 倍；JSONB 形态的成本随 type 内对象数线性增长（拉回 + Trino 侧 JSON 解析）。

### spike 3 结论

1. **edit store 物理形态定为「按 object type 分表、属性为强类型列」**（由 ontology 定义生成 DDL / 迁移），外加 append-only 历史表（可保留 JSONB 快照，只用于审计与按 pk 取版本，不参与过滤）。通用 JSONB 表经 Trino 读取的过滤不可接受（A2.1、A2.4）。代价（违背原 STORE-02「加载 ontology 无需 DDL」）交 `/plan-eng-review` 与 ADR-0004 修订。
2. **读后写一致成立**：PG 提交后下一条 Trino 查询必然可见（A2.2），无需额外缓存配置；部署文档只需写明 edit store catalog 不得开启任何数据 / 结果缓存。
3. **按 pk 取对象与 Action 前置读不走 Trino 叠加 join**：改为「PG 直读 edit store（同事务 `FOR UPDATE`，亚毫秒）→ 命中且非 tombstone 直接返回；tombstone 返回不存在；未命中才经 Trino 做 source 单表 pk 查找（单 fragment）」，叠加在 Rust 侧完成。Action 端到端预算因此只含一次单 fragment Trino 查询。
4. **列表 / 过滤读仍在 Trino 内叠加**，采用 anti-join + UNION ALL 形态（A2.3）：source 侧谓词与 TopN 下推（over-fetch N + edit 上界），edit 侧依赖动态过滤 IN 下推与强类型索引。编译器需保证 join 键类型对齐、cast 放小表侧、两侧都写可下推谓词；`domain_compaction_threshold = 256` 限制 over-fetch 上限，超过需分批。
5. **预算修订建议**：pk 查找 p95 < 500ms 仅对「PG 直读 + source 单 fragment」路径成立，且需空闲 / 有资源组隔离的集群；Trino 内叠加过滤在繁忙集群上 p95 约 4–5s，建议把过滤预算改为「空闲集群 p95 < 2s，繁忙集群由资源组保障」，并要求部署为 ontoforge 配置专用 Trino resource group（排队与 stage 调度是主要波动源）。
6. 仍未覆盖：开启 `enable_string_pushdown_with_collate` 后前缀 LIKE / varchar 聚合的下推（需要可写会话）；PG 侧补统计（`ANALYZE` 已做，但 Trino 估算仍为 `?`，需确认 connector `statistics.enabled`）以让叠加 join 选 REPLICATED；样本 n=5，p95 只能看趋势。
