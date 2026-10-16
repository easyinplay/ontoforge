# Spike 03b：多 role 遮蔽下推、Rust 两段归并 vs Trino 内叠加、可分解聚合、resource group

- 日期：2026-10-16
- 对应：`docs/designs/trino-engine-sql-tiers.md`「Eng Review 决策」D16（spike 3b 门禁）与 D17（按属性 patch）
- 环境：同 spike 3 的生产 Trino 483 集群（3 节点），经 MCP 只读通道；edit store 为测试 PG 16 `ontoforge` 库，Trino 经 `pg_cat`（postgresql connector，只读角色）读取。Trino 侧只跑 SELECT / SHOW / EXPLAIN / EXPLAIN ANALYZE
- 通用性约定：catalog / schema / 表 / 列名一律为占位符，不含真实名称与数据值
- **负载条件（重要）**：本轮测试时段集群**安静**。测试前 1 小时 2402 条已完成查询，每分钟 17–119 条，任一时刻并发查询数峰值 ≤ 4；本轮每条被测查询启动时的并发数为 1–3。spike 3 附录 2 的「繁忙」时段（同时 RUNNING 约 50 条）未能复现，繁忙结论引用 spike 3 数据并明确标注

## 结论先行

| D16 问题 | 结论 |
|---|---|
| ① 多 role 遮蔽 scoped SQL 能否下推 | **能，但必须用改写形态**。用户过滤写在 CASE 遮蔽表达式上（朴素形态）时，只有 role 行谓词的 OR 下推，用户过滤与 TopN 全部留在 Trino，等于拉回约 290 万行，中位 2.5s、最大 17s；把 `x op v` 改写为 `(授予 x 的 role 谓词之 OR) AND x op v` 作用在原列、遮蔽只留投影层后，`constraint on [...]` + `sortOrder ... limit=50` 整体下推，中位 312ms，结果集与朴素形态逐行一致 |
| ② Rust 两段归并 vs Trino 内叠加 | **安静集群两者同量级**（Trino 内叠加 D17 形态中位 581ms；Rust 两段归并服务端耗时串行和约 0.94s、批次并行约 0.62s、按依赖并行约 0.36s）。**但 Trino 内叠加是 7 个 fragment**，spike 3 已证明多 stage 查询在繁忙集群上退化到 2–4s，而 Rust 路径只由 ≤ 2 fragment 的单表查询 + PG 直连组成。另有两个结构性陷阱只存在于 Trino 内叠加：edit 分支写成 `edit LEFT JOIN source` 时 source 被选为广播 build 侧（全表扫描，中位 2.4s）；动态过滤 / IN 超过 256 个值时对 JDBC source 不再生效 |
| ③ 可分解聚合改写 | `count` / `sum` / `avg` 可分解且结果与朴素叠加聚合相等（两组过滤均验证）。但**分解只有在「source ∩ edit 集合」那一项按 pk 分批（≤ 256）查询时才有收益**：写成单条 Trino join 时它退化为广播 source 侧，非选择性过滤下分解形态 7.0–9.8s，并不比朴素 7.2–7.8s 快；改为分批 pk 查询后单批 336ms、source 聚合下推 193ms。另实测 SQL Server connector **`sum(integer)` 不下推**（`count` / `min` / `max` / `sum(double)` / `avg(double)` 下推） |
| ④ 专用 resource group | **解决不了 stage 调度瓶颈**。本轮所有查询 `queued_time_ms` ≤ 1ms，前 1 小时 2402 条查询最大排队 1ms，集群只有一个 `global` 组；spike 3 繁忙时段排队同样为 0。时间全部在 analysis + planning（36–45%）与执行期（Blocked 等待）。resource group 只管准入排队，对已运行查询的 split 调度没有优先级；它唯一的间接作用是由管理员给**其他**工作负载设并发上限以降低争用（未验证，需要管理员权限） |

对 D2 / D10 的建议：

1. **D2 列表路径改为「Rust 两段归并为主」**：Trino 只承担单表查询（source 谓词 + TopN over-fetch；按 pk 分批 ≤ 256 的回查），edit store 由 PG 直连读取，叠加与分页在 Rust 内完成（Rust 实现本来就是 D2 规定的参考实现，改为主路径后少维护一套叠加 SQL 编译）。Trino 内叠加 SQL 降级为备选（例如 L2 / 只读分析场景），不作为延迟承诺路径。
2. **聚合由 Rust 编排分解**：`count` / `sum` / `avg` 走「source 聚合下推 − pk 分批修正 + PG 聚合」；`min` / `max` 走 source TopN 补取；`count(distinct)` 走分组计数分解或回退朴素叠加（见 ③）。
3. **D10 预算**：安静集群实测所有推荐形态都在 0.72s 内（n=7，p95 无统计意义，只能看最大值），建议列表 / 过滤预算定为「安静或隔离集群 p95 < 1s（Rust 路径）」，聚合 p95 < 1.5s；**删除「专用 Trino resource group」这一前提**，替换为「Trino 查询形态约束（单表、≤ 2 fragment）+ 部署文档建议管理员限制其他工作负载并发」；繁忙共享集群仍只监测不承诺。

## 1. 测试对象与数据

| 占位名 | 说明 |
|---|---|
| `ss_cat.schema_a.table_a` | SQL Server connector，约 291 万行，pk `id`（integer） |
| `col_status` | 低基数整数列（8 个取值），`col_status = 2` 约 261 万行 |
| `col_cat` | 整数列（约 2000 个取值）；spike 3 的历史查询已过期无法反查，按「`col_cat > 400` 非选择性、`BETWEEN 400 AND 600` 选择性」选定。`col_status = 2 AND col_cat BETWEEN 400 AND 600` 命中 131 行（单一 `col_cat` 取值） |
| `pg_cat.edit_store.spike3b_patch` | 本 spike 新建的 D17 形态 edit 表（见下） |

```sql
CREATE TABLE edit_store.spike3b_patch(
  pk integer PRIMARY KEY,          -- 与 source id 类型对齐（source 为 integer，故未用 bigint）
  version bigint NOT NULL, deleted boolean NOT NULL DEFAULT false,
  released boolean NOT NULL DEFAULT false, set_mask int NOT NULL,   -- bit0 = col_status, bit1 = col_cat
  col_status int, col_cat int);
-- + 索引 (col_status)、(col_cat)；ANALYZE；GRANT SELECT TO 只读角色
```

数据（共 2050 行）：

- 60 行取自过滤命中集合中 id 最大的 100 个，按 5 类轮换：tombstone、只改 `col_status` 使其出过滤、两列都改使其出过滤（`col_cat` 越界）、只设 `col_status = 2`（仍在过滤内，需要 source 的 `col_cat`）、两列都改且仍在过滤内。
- 1940 行取自 source 的伪随机 id（`id % 1460 = 7`）：约 10% tombstone、约 30% 只设 `col_status`、其余两列都设；约 2% 标 `released`（应视为纯 source）。
- 50 行 edit-only（id 大于 source 最大值），两列都设，一半满足过滤。
- 汇总：tombstone 215、只设 `col_status` 641、两列都设 1194、released 32、edit-only 50。

两种叠加结果的正确性做了三方对照：Trino 朴素叠加（无 over-fetch 截断的 ground truth）、Trino 改写叠加、Python 模拟的 Rust 归并，top 50 完全一致；全量计数三方都是 300。

## 2. ① 多 role 遮蔽的 scoped SQL

Role：R1 行谓词 `col_status = 2`，可见列 {id, col_status, col_cat}；R2 行谓词 `col_cat > 400`，可见列 {id, col_cat}。用户过滤 `col_status = 2 AND col_cat BETWEEN 400 AND 600 ORDER BY id DESC LIMIT 50`。

(a) 朴素：

```sql
SELECT * FROM (
  SELECT CASE WHEN (col_status = 2 OR col_cat > 400) THEN id END AS id,
         CASE WHEN col_status = 2 THEN col_status END AS col_status,
         CASE WHEN (col_status = 2 OR col_cat > 400) THEN col_cat END AS col_cat
  FROM ss_cat.schema_a.table_a WHERE (col_status = 2 OR col_cat > 400)) v
WHERE col_status = 2 AND col_cat BETWEEN 400 AND 600 ORDER BY id DESC LIMIT 50
```

(b) 改写（遮蔽只在投影；`id` 被 scope 内所有 role 授予，直接输出原列）：

```sql
SELECT id,
       CASE WHEN col_status = 2 THEN col_status END AS col_status,
       CASE WHEN (col_status = 2 OR col_cat > 400) THEN col_cat END AS col_cat
FROM ss_cat.schema_a.table_a
WHERE (col_status = 2 OR col_cat > 400)                              -- scoped 基关系
  AND (col_status = 2) AND col_status = 2                            -- 授予 col_status 的 role：R1
  AND (col_status = 2 OR col_cat > 400) AND col_cat BETWEEN 400 AND 600  -- 授予 col_cat 的 role：R1、R2
ORDER BY id DESC LIMIT 50
```

EXPLAIN（DISTRIBUTED）关键片段：

```
(a) ScanFilterProject[table = ss_cat:schema_a.table_a constraints=[ParameterizedExpression[("col_status") = (?)) OR ((?) < ("col_cat"))]],
      filterPredicate = ((CASE WHEN (col_status = 2) THEN (col_status = 2) ELSE null END) AND (CASE WHEN ... THEN (400 <= col_cat) ...) ...)]
      Estimates: {rows: 1470245 ...}
    └ 上层 TopNPartial ×3（Trino 侧）→ Fragment 0 TopN
(b) TableScan[table = ss_cat:schema_a.table_a constraint on [col_status, col_cat]
      constraints=[ParameterizedExpression[("col_status") = (?)) OR ((?) < ("col_cat"))]]
      sortOrder=[id DESC NULLS LAST] limit=50]
    Fragment 0 只剩 Project（CASE 遮蔽）+ TopN 合并
```

- 两者都是 2 个 fragment（SOURCE + SINGLE），与 spike 3 的单表基线相同；差别在 (a) 的用户过滤与 TopN 全部留在 Trino。注意 role 谓词的 `OR` 本身可以作为 `ParameterizedExpression` 下推，这说明改写后的布尔表达式（含 OR）都能下推。
- 语义等价：去掉 LIMIT 后 (a)、(b) 都是 131 行，`a EXCEPT b`、`b EXCEPT a` 均为 0 行；带 LIMIT 50 时两者 `count / sum(id)` 一致（7 轮每轮都一致）。

延迟（服务端，`system.runtime.queries`，7 轮交替执行；pre = analysis + planning；外层套 `count(*), sum(id)` 压缩输出，内层计划不变）：

| 查询 | pre min / med / max (ms) | total min / med / max (ms) | 启动时并发 |
|---|---|---|---|
| (a) 朴素 | 126 / 137 / 234 | 2,051 / 2,455 / 17,312 | 1–3 |
| (b) 改写 | 98 / 112 / 247 | 217 / 312 / 438 | 1–3 |

(a) 的 7 次里有 3 次在 16.5–17.3s，其余约 2.1–2.5s（源端全表扫描受 SQL Server 缓存状态影响），(b) 稳定。

改写规则（给编译器）：

1. 对用户谓词 `P(mask(x))`，其中 `mask(x) = CASE WHEN g THEN x END`、`g` 为授予 x 的 role 谓词之 OR，恒等于 `(g AND P(x)) OR (NOT g AND P(NULL))`。`P(NULL)` 是常量，可在编译期折叠：若它为 NULL / FALSE（`=`、`<`、`BETWEEN`、`IN`、`LIKE` 等 null-rejecting 谓词），得到 `g AND P(x)`；若为 TRUE（`IS NULL`、`coalesce(x, v) = v` 一类），保留 OR 形态，仍可下推。
2. 多列谓词 `P(x, y)` 且 x、y 授予集合不同时，按授予组合展开（组合数 2^k，k 为谓词涉及的「非全授予」列数）；k 过大时回退朴素形态并告警。
3. 列被 scope 内所有 role 授予时直接输出原列（Trino 不会利用基关系谓词把 CASE 化简，(a) 中 `id` 的 CASE 保留到了 TopN 键上）。
4. 未覆盖：`ORDER BY` 一个部分授予的列。遮蔽后的排序等价于 `ORDER BY g DESC, x`（NULLS LAST 时），这是表达式排序，JDBC TopN 不下推。建议 lint 提示或在 Rust 侧排序。

## 3. ② 列表读：Trino 内叠加 vs Rust 两段归并

### 3.1 Trino 内叠加（D17 按属性 patch）

先试了 D17 的直译形态 P1：edit 分支 = `e LEFT JOIN source ON source.id = e.pk`，按 `set_mask` 位取 edit 或 source 值，合并后再过滤。EXPLAIN：

```
LeftJoin[criteria = (pk = id), distribution = REPLICATED]
├─ TableScan[pg_cat:edit_store.spike3b_patch constraint on [deleted, released]]
└─ RemoteSource ← Fragment 5: TableScan[ss_cat:schema_a.table_a columns=[col_cat, col_status, id]]   -- 无约束，BROADCAST
     Estimates: {rows: 2909617 (41.60MB)}
```

LEFT JOIN 的右侧固定为 build 侧，动态过滤只能作用于 probe 侧，于是 source 被**全表**广播。实测中位 2.4s（n=3，安静集群）。

改写形态 P2（实测采用）：edit 分支按 `set_mask` 分组编译，每组只对**未设置**的列在 source 侧施加用户过滤的对应合取项，且让 edit 作为 build 侧：

```sql
WITH e AS (SELECT pk, deleted, set_mask, col_status, col_cat
           FROM pg_cat.edit_store.spike3b_patch WHERE NOT released)
SELECT id, col_status, col_cat FROM (
  -- source 分支：谓词 + TopN over-fetch，anti-join 全部 edit（含 tombstone）
  SELECT s.id, s.col_status, s.col_cat
  FROM (SELECT id, col_status, col_cat FROM ss_cat.schema_a.table_a
        WHERE col_status = 2 AND col_cat BETWEEN 400 AND 600 ORDER BY id DESC LIMIT 100) s
  LEFT JOIN e ON e.pk = s.id WHERE e.pk IS NULL
  UNION ALL
  -- 全设置组（set_mask = 3）：只读 edit
  SELECT pk, col_status, col_cat FROM e
  WHERE NOT deleted AND set_mask = 3 AND col_status = 2 AND col_cat BETWEEN 400 AND 600
  UNION ALL
  -- 部分设置组（set_mask = 1）：已设列在 edit 侧过滤，未设列的合取项下推到 source
  SELECT p.pk, p.col_status, s2.col_cat
  FROM ss_cat.schema_a.table_a s2
  JOIN (SELECT pk, col_status FROM e WHERE NOT deleted AND set_mask = 1 AND col_status = 2) p ON s2.id = p.pk
  WHERE s2.col_cat BETWEEN 400 AND 600
) u ORDER BY id DESC LIMIT 50
```

计划要点（EXPLAIN + EXPLAIN ANALYZE）：

- 共 **7 个 fragment**（spike 3 的 anti-join + UNION ALL 为 5 个）。
- source 分支：`constraint on [col_status, col_cat] sortOrder=[id DESC] limit=100` 整体下推；anti-join 的 PG 侧带动态过滤，`SortedRangeSet ranges=100` 下推到 PG，实读 60 行。
- 全设置组：`TableScan[pg_cat:... constraint on [deleted, set_mask, col_cat, released, col_status] sortOrder=[pk DESC] limit=50]`，**过滤 + TopN 整体下推到 PG**。
- 部分设置组：PG 侧 335 行作为广播 build；source 侧 `constraint on [col_cat]` 下推，动态过滤 `ranges=335` 在 Trino 侧收集到了，但 source 实读 218 行，正好是 `col_cat BETWEEN 400 AND 600` 的全部行，即**动态过滤没有作用到 JDBC 源端**（推测为 `domain_compaction_threshold = 256` 把 335 个值压成 min/max 范围）。本例靠「未设列合取项」兜住了选择性；若未设列的合取项不选择性（例如只设 `col_cat` 而过滤主要靠 `col_status`），这一支会拉回大量 source 行。
- PG catalog 统计已生效（估算 `rows: 241` / `29`，不再是 `?`），spike 3 遗留的「Trino 估算为 ?」问题在 ANALYZE 后消失。
- EXPLAIN ANALYZE 一次：Queued 0.2ms、Analysis 113ms、Planning 149ms、Execution 568ms；source 扫描物理读 87ms、PG 各扫描 2–5ms，各 fragment CPU 合计约 0.28s，其余为 exchange / 调度等待。

### 3.2 Rust 两段归并（模拟）

| 段 | 内容 | 实现 |
|---|---|---|
| Q1 | source 单表：谓词 + `ORDER BY id DESC LIMIT 100`（over-fetch） | Trino，2 fragment，全部下推 |
| Q2 | edit 表 `pk = ANY(Q1 的 100 个 id)`：压制 / 合并 Q1 行 | PG 直连 |
| Q3a | edit 候选：`NOT deleted AND NOT released AND (bit0 未设 OR col_status = 2) AND (bit1 未设 OR col_cat BETWEEN 400 AND 600)`，552 行（全设置 217、部分设置 335） | PG 直连 |
| Q3b | 部分设置候选需要 source 值：`WHERE id IN (...)` 按 ≤ 256 分两批（168 + 167） | Trino，单 fragment，`constraint on [id]` 下推 |
| 合并 | Rust 内存合并、排序、截取 50 | 可忽略（几百行） |

Q3b 有一个 Rust 侧才方便做的剪枝：先用 Q1 存活行 + 全设置候选得到暂定第 50 名的 id，只有 pk 大于它的部分设置候选才需要回查。本数据中 335 个部分设置候选只剩 33 个（一批即可）。

延迟（Trino 段为服务端耗时，7 轮交替；PG 段为 Python 客户端往返，复用连接，9 次）：

| 查询 | pre min / med / max (ms) | total min / med / max (ms) |
|---|---|---|
| Trino 内叠加 P2（7 fragment） | 252 / 259 / 311 | 516 / **581** / 664 |
| Trino 内叠加 P1（LEFT JOIN 直译，n=3） | 225 / 231 / 345 | 2,141 / 2,392 / 2,755 |
| Rust Q1（Trino） | 104 / 110 / 236 | 212 / 261 / 498 |
| Rust Q2（PG） | — | 5.9 / 6.3 / 10.9 |
| Rust Q3a（PG） | — | 6.3 / 6.8 / 13.0 |
| Rust Q3b 批 1（Trino，168 个 pk） | 103 / 112 / 133 | 285 / 316 / 365 |
| Rust Q3b 批 2（Trino，167 个 pk） | 106 / 113 / 149 | 239 / 350 / 435 |

Rust 路径合计（中位）：

| 编排方式 | 估算 |
|---|---|
| 全串行（Q1 → Q2 → Q3a → Q3b 两批） | 261 + 6 + 7 + 316 + 350 ≈ **940ms** |
| Q3b 两批并行 | 261 + 6 + 7 + 350 ≈ 624ms |
| 按依赖并行：`(Q1 → Q2) ∥ (Q3a → Q3b)` | max(267, 357) ≈ **360ms** |
| 加 TopN 剪枝（Q3b 只剩 33 个 pk、一批） | 约 300–360ms（Q3b 单批与 Q1 同量级） |

判断：

- 安静集群上，Trino 内叠加（581ms）与 Rust 两段归并（360–940ms，取决于编排）同量级，**没有压倒性差距**。
- 差异在负载敏感性与陷阱：Trino 内叠加是 7 fragment，spike 3 附录 2 在繁忙集群上实测多 stage 叠加查询中位 2.6–3.9s、单 fragment 基线在 1s 内（瓶颈为 Blocked 而非扫描）；Rust 路径的每条 Trino 查询都是 ≤ 2 fragment 的单表形态，且 PG 段为毫秒级。另外 P1 陷阱（LEFT JOIN 方向导致 source 全表广播）和 256 值动态过滤失效，在 Rust 路径中都不存在（pk 回查由 Rust 控制批大小）。
- 上表 Trino 段是服务端耗时，不含客户端 HTTP 轮询（`nextUri`）与结果传输。Rust 路径的 Trino 查询数多（3 条 vs 1 条），这部分客户端开销对 Rust 路径不利，**未测**（MCP 通道无法测客户端视角）。
- 正确性：over-fetch 100 在本数据上足够（top 50 全部来自 edit 行）；按 D22，生产实现必须在 anti-join 后不足 N 行时循环补取。

## 4. ③ 聚合

### 4.1 选择性过滤（同 ①）：`count(*)`、`sum(col_cat)`

- 朴素 G：在 P2 叠加关系（去掉 TopN）上直接 `count(*) / sum(col_cat)`。
- 分解 H：`agg(source 下推，带过滤) − agg(source 中 pk ∈ edit 集合且满足过滤) + agg(合并后 edit 行满足过滤)`。其中 edit 集合含 tombstone、不含 released。

```sql
WITH e AS (... WHERE NOT released),
s AS (SELECT count(*) c, sum(col_cat) sm FROM ss_cat.schema_a.table_a
      WHERE col_status = 2 AND col_cat BETWEEN 400 AND 600),
x AS (SELECT count(*) c, coalesce(sum(t.col_cat), 0) sm FROM ss_cat.schema_a.table_a t
      JOIN (SELECT pk FROM e) k ON t.id = k.pk
      WHERE t.col_status = 2 AND t.col_cat BETWEEN 400 AND 600),
m AS (/* 全设置组 + 部分设置组，同 P2 的两个 edit 分支 */)
SELECT s.c - x.c + m.c, s.sm - x.sm + m.sm FROM s, x, m
```

结果：G 与 H 均为 `count = 300`、`sum` 相同（7 轮每轮一致）；分项 s = 131、x = 60、m = 229。

| 查询 | pre min / med / max (ms) | total min / med / max (ms) |
|---|---|---|
| G 朴素叠加聚合 | 229 / 254 / 286 | 545 / 600 / 655 |
| H 分解（单条 Trino 查询） | 300 / 309 / 358 | 599 / 663 / 715 |
| 参照：source `count / min / max` 下推（单 fragment） | 97 / 104 / 137 | 195 / 209 / 298 |

选择性过滤下 source 侧本来就只有 131 行，分解没有收益。下推观察：

```
count(*), min(col_cat), max(col_cat)：
  TableScan[ss_cat:Query[SELECT count_big(*), min("col_cat"), max("col_cat") FROM ... WHERE "col_status" = ? AND ...]]   -- 单 fragment
count(*), sum(col_cat)（col_cat 为 integer）：
  Aggregate[PARTIAL] ← ScanProject[... expr := CAST(col_cat AS bigint)]   -- sum 不下推，整段聚合留在 Trino
count(*), sum(double 列), avg(double 列)：
  TableScan[ss_cat:Query[SELECT count_big(*), sum(...), avg(...) ...]]   -- 下推
```

`sum(integer)` 在 Trino 中结果类型为 bigint，SQL Server connector 不下推（推测是溢出语义不同）。编译器对 integer 列的 sum 不能指望下推。

### 4.2 非选择性过滤（`col_status = 2`，source 约 261 万行）：`count(*)`

| 查询 | pre min / med / max (ms) | total min / med / max (ms) |
|---|---|---|
| G2 朴素叠加聚合（n=3） | 191 / 192 / 377 | 7,164 / 7,384 / 7,843 |
| H2 分解，单条 Trino 查询（n=3） | 229 / 242 / 247 | 7,028 / 8,639 / 9,781 |
| H2 的 s 项单独：`count(*)` 下推（n=1） | 112 | 193 |
| H2 的 x 项按 pk 分批：`count(*) WHERE col_status = 2 AND id IN (168 个 pk)`（n=3） | 97 / 104 / 108 | 257 / 336 / 340 |

两种结果相等（2,612,575）。H2 单条查询没有变快，原因在 x 项：

```
InnerJoin[criteria = (pk = id), distribution = REPLICATED]
├─ ScanFilter[pg_cat:edit_store.spike3b_patch, dynamicFilters = {pk = #df}]
└─ RemoteSource ← TableScan[ss_cat:schema_a.table_a constraint on [col_status] columns=[id]]   -- BROADCAST，source 成了 build 侧
```

source 估算行数被低估（SQL Server 统计给出 36 万，实际 261 万），优化器把 source 当成小表广播，等于把 261 万个 id 拉进 Trino。即使翻转方向，2018 个 edit pk 的动态过滤也超过 256、对 JDBC 失效。

结论：**分解改写的收益只在 Rust 编排下成立**：s 项单 fragment 下推（193ms）+ x 项按 ≤ 256 分批 pk 查询（单批 336ms，2018 个 edit pk 需 8 批，并行时预计约 0.4–0.6s，串行约 2.7s，**未全量实测**）+ m 项 PG 直连聚合（毫秒级）。相比朴素 7.4s 是一个数量级的改善。另外：当过滤只引用某 mask 组已设置的列、且聚合只需要 count 时，m 项完全不需要 source（本例即如此）。

### 4.3 不可分解的聚合与替代

| 聚合 | 为何不能「减法」分解 | 可行替代 |
|---|---|---|
| `min` / `max` | 没有逆运算：source 中被 edit 覆盖的那行恰好是极值时，无法从「全体 min」中扣除 | `min = min(source 排除 edit pk 后的 min, 合并后 edit 行的 min)`。前一项需要 anti-join；在 JDBC 上可用 TopN 代替：`ORDER BY x LIMIT k`（TopN 下推）取前 k 行，Rust 丢弃 pk 在 edit 集合中的行，取第一条存活行；不够就倍增 k。edit 少时一两次即可 |
| `count(distinct x)` | 集合语义，没有可减的计数 | 分组计数分解：source `GROUP BY x` 的 count 下推（spike 3 已证 JDBC 分组聚合下推）− pk 分批修正的分组计数 + edit 合并后的分组计数，净计数 > 0 的组数即结果。只适合 x 基数可控的情形，否则回退朴素叠加 |
| `avg` | 可分解 | `sum / count` 两者分别分解 |
| 近似聚合（`approx_distinct` 等） | HLL 等草图不支持删除 | 回退朴素叠加，或文档声明「近似值不反映未发布的 edit」 |

## 5. ④ resource group（只观测）

| 观测 | 数据 |
|---|---|
| 资源组 | 所有查询 `resource_group_id = ['global']`，集群只有一个组 |
| 本轮被测查询排队 | 全部 `queued_time_ms` ≤ 1ms |
| 前 1 小时全集群 | 2402 条已完成查询，`max(queued_time_ms) = 1`，p99 = 1ms；查询耗时中位 1.7s、p95 9.4s |
| spike 3（繁忙时段） | 第 3 节与附录 2 的 queued 同样为 0 |

耗时构成（中位）：

| 查询 | 排队 | analysis + planning | 执行（total − pre − queued） | pre 占比 |
|---|---|---|---|---|
| (b) scoped 改写 | 0 | 112 | 190 | 36% |
| Rust Q1 | 0 | 110 | 145 | 42% |
| P2 Trino 内叠加 | 0 | 259 | 328 | 45% |
| H 分解聚合 | 0 | 309 | 345 | 47% |

判断：

- 排队占比为 0，专用 resource group（`hardConcurrencyLimit`、`maxQueued`、`schedulingWeight` 等只决定查询何时被准入、谁先出队）**没有可改善的对象**。
- 繁忙时的退化来自执行期：已运行查询共享 worker 的 task executor 时间片，split 调度对资源组无感，多 stage 查询的 exchange 等待被放大（spike 3 附录 2 EXPLAIN ANALYZE：数据访问几十毫秒，Execution 3.56s 几乎全是 Blocked）。**resource group 解决不了 stage 调度瓶颈。**
- 唯一可能的间接作用：管理员给其他工作负载的资源组设低并发上限，降低 ontoforge 查询执行时面对的争用。这是集群级策略，**未验证**（需要管理员权限修改资源组配置，且需要繁忙时段对照实验）。
- 能由 ontoforge 自己控制的降耗手段是查询形态：单表 ≤ 2 fragment、全部谓词与 TopN 下推（spike 3 繁忙时段这类查询仍在 1s 内）；以及 JDBC catalog 开 `metadata.cache-ttl` 压低 analysis 固定税（spike 3 已建议）。

## 6. 未验证项

1. **繁忙时段对照**：本轮集群安静（并发 ≤ 4），未能在繁忙时段重测 P2 与 Rust 路径；「繁忙时 Rust 路径更稳」是基于 spike 3 单 fragment vs 多 stage 数据的推断。建议在实现后的 benchmark 中加繁忙时段（或人为并发压测）对照。
2. **客户端视角耗时**：全部 Trino 数字为服务端 `created → end`，不含 HTTP 轮询、结果下载；Rust 路径 Trino 调用次数多，客户端开销未计入。
3. **x 项 8 批并行的实际耗时**：只测了单批（168 个 pk），8 批并行为估算。
4. **专用 resource group 的效果**：需要管理员权限，未配置、未测。
5. **iceberg source（`ice_cat.schema_b.table_b`）对照**：时间不够跳过。iceberg 无 pk 索引，Q3b 型 pk 回查预期每批都要扫文件（spike 3：单个 pk 查找中位 831ms），Rust 路径在 iceberg source 上的回查成本需单独评估。
6. 部分授予列上的 `ORDER BY`、多列谓词的授予组合展开、`P(NULL) = TRUE` 类谓词：只做了推导，未实测。
7. `domain_compaction_threshold` 调大（catalog 配置）后 JDBC 动态过滤能否承载更多值：需要可写会话或管理员配置。
8. 样本量 n=7（部分 n=3），p95 无统计意义，表中只能看 min / 中位 / max。

## 7. 遗留测试对象

- PG：`ontoforge` 库 `edit_store.spike3b_patch`（2050 行，索引 `spike3b_patch_status`、`spike3b_patch_cat`，已授予只读角色 SELECT）。清理：`DROP TABLE edit_store.spike3b_patch;`
- 本机临时脚本（仓库外）：`%TEMP%/spike3b/`（连接参数从本机 env 文件读取，脚本与输出不含凭据）。
