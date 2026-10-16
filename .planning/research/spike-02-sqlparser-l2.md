# Spike 2：用 sqlparser-rs 校验 L2 Raw Trino SQL

**日期：** 2026-10-16
**来源 gate：** `docs/designs/trino-engine-sql-tiers.md`「Gates before /plan-eng-review」第 2 项（L2 解析）
**结论：** 可用。采用 `sqlparser` 0.63.0 + `GenericDialect` + 白名单 visitor，**执行时发送规范化后的重序列化 SQL（Display），不发送原文**；同时加输入长度上限与 round-trip 自检。
**置信度：** MEDIUM-HIGH（全部行为由实验 + Trino 435 官方解析器对照得出；未在运行中的 Trino 集群上执行，且对照解析器比当前 Trino 旧约 50 个版本）

标记约定：`[verified: 来源]` = 本次会话用工具读过原始来源或跑过实验；`[unverified]` = 未在本次会话核实。

## 1. 问题

L2 允许受信主体提交原始 Trino SQL。ontoforge 需要：只接受单条只读语句（`SELECT` / `WITH ... SELECT`）；拒绝表函数（含 JDBC connector `system.query` 透传）、`CALL`、`EXECUTE`、`PREPARE`、会话 / 事务语句、DML / DDL、多语句、无法解析的输入；提取所有被引用的 catalog.schema.table 做 allow-list 校验；注入或收紧 LIMIT。问题：`sqlparser` 能否可靠承担这件事。

## 2. sqlparser 现状

- 最新版 `0.63.0`，发布于 2026-09-13；上一版 `0.62.0`（2026-05-07）[verified: crates.io API]。约 4 个月一个 minor，且 0.x 每个 minor 都可能破坏 AST。
- 许可 `Apache-2.0`，仓库 `apache/datafusion-sqlparser-rs`，`edition = "2021"`，未声明 `rust-version` [verified: sqlparser-0.63.0/Cargo.toml:13,38,39]。在 Rust 1.98 / Windows 11 下编译通过 [verified: experiment]。总下载 83.7M，近 90 天 14.5M [verified: crates.io API]；`package-legitimacy` 结论 `OK` [verified: gsd-tools]。
- **没有 Trino / Presto 方言**。`dialect_from_str` 只认 `generic`、`mysql`、`postgresql`、`hive`、`sqlite`、`snowflake`、`redshift`、`mssql`、`clickhouse`、`bigquery`、`ansi`、`duckdb`、`databricks`、`spark`、`oracle`、`teradata` [verified: src/dialect/mod.rs:1911-1931]。上游 issue #2611「Add a Presto dialect」2026-10-04 开启，仍为 OPEN [verified: gh issue view]。
- 关键 AST 事实 [verified: src/ast/query.rs]：
  - `TableFactor::Table { name, args: Option<TableFunctionArgs>, .. }`：`args` 为 `Some` 时表示表值函数调用，不是普通表（1470-1477 行）。visitor 的 `pre_visit_relation` 对它照样触发，所以只收集 relation 名而不看 `args` 会漏掉 `FROM pg.system.query(...)`。
  - `TableFactor::TableFunction { expr, alias }` 对应 `TABLE(<expr>)`（1509-1515 行）；`TableFactor::Function` 对应 `LATERAL f(...)`（1516-1528 行）。
  - `SetExpr` 含 `Insert` / `Update` / `Delete` / `Merge(Statement)` 与 `Table(Box<Table>)`（150-180 行）；`Table` 只有 `table_name: Option<String>` 与 `schema_name: Option<String>`（301-306 行），**不是 `ObjectName`，不会触发 `pre_visit_relation`**，必须单独拒绝。
  - `LimitClause::LimitOffset` 的 Display 固定输出 `LIMIT x OFFSET y`（3129-3150 行）。Trino 语法要求 `OFFSET` 在 `LIMIT` 之前（见第 6 节）。
- 解析器默认递归深度 `DEFAULT_REMAINING_DEPTH: usize = 50` [verified: src/parser/mod.rs:195]；默认 feature `recursive-protection` 打开。

Trino 侧语法事实 [verified: trinodb/trino master `SqlBase.g4`]：
- 表函数只能通过 `TABLE '(' tableFunctionCall ')'` 调用（517 行）。
- 块注释 `'/*' .*? '*/'` **不可嵌套**（1577-1579 行）。sqlparser 的 `GenericDialect`、`AnsiDialect`、`PostgreSqlDialect`、`DatabricksDialect` 等把块注释视为可嵌套（`supports_nested_comments` 返回 true）[verified: src/dialect/generic.rs:192-193 等]。
- `queryNoWith: queryTerm orderBy? (OFFSET ...)? (LIMIT ... | FETCH ...)?`（283-290 行）。
- 新语法 `WITH SESSION ...` 前缀（`rootQueryWithSession`，236-237 行）与 `WITH FUNCTION ...`（`rootQuery`，231-233 行）。
- 字符串不使用反斜杠转义（`UNICODE_STRING` 等规则只认 `''`，1497-1499 行）。
- 标识符大小写：「Identifiers are not treated as case sensitive.」[verified: trino.io/docs/current/language/reserved.html]

## 3. 实验方法

- 临时 cargo 工程（`%TEMP%`，已删除）依赖 `sqlparser = { version = "0.63.0", features = ["visitor"] }`。语料 104 条：48 条应放行（G01-G48）+ 56 条应拒绝（B01-B56，其中 B51 后改判为应放行，见表注）。
- 9 个方言各跑一遍解析 + 白名单校验：generic、ansi、postgres、duckdb、hive、databricks、bigquery、snowflake、mysql。
- **真值对照**：下载 `io.trino:trino-parser:435` 及依赖（Maven Central，SHA1 已核对），用 `SqlParser.createStatement` + `SqlFormatter.formatSql` 解析三种文本：原文、sqlparser Display、注入 LIMIT 后的 Display。选 435 是因为它是最后一个以 Java 17 为目标的版本（本机只有 JDK 17）[verified: trino-root-435.pom `project.build.targetJdk=17`]。
- 白名单 visitor 规则见第 7 节；allow-list 为 `hive.{s,sales,dw}`、`pg.public`、`iceberg.s`，`edit` 代表 edit store catalog。

## 4. 方言对比

| 方言 | 应放行且解析成功（/49） | 误放行 | 误拒 | 备注 |
|---|---|---|---|---|
| **generic** | **45** | 1（B53，仅当发送原文时） | 5 | 唯一能解析 `MATCH_RECOGNIZE` 且 `UNNEST` 正确成为 `TableFactor::UNNEST` 的组合之一 |
| ansi | 42 | 1（B53） | 10 | `UNNEST(...)` 被解析为 `Table{args: Some}`，无 GROUPING SETS |
| postgres | 44 | 1（B53） | 6 | 无 `MATCH_RECOGNIZE`；不支持反引号 |
| duckdb | 44 | 0 | 8 | `UNNEST` 解析为 `Table{args: Some}` |
| hive | 43 | 0 | 9 | 同上 |
| databricks | 42 | 2（B53、B54） | 10 | 反斜杠转义字符串 |
| bigquery | 41 | 1（B54） | 9 | 反斜杠转义字符串 |
| snowflake | 41 | 1（B54） | 11 | 反斜杠转义字符串 |
| mysql | 40 | 1（B54） | 12 | 反斜杠转义字符串 |

[verified: experiment]。没有方言与 Trino 词法完全一致：generic / ansi / postgres / databricks 的嵌套注释与 Trino 不同（B53），databricks / bigquery / snowflake / mysql 的反斜杠转义与 Trino 不同（B54）。这两类差异只有在**把原文发给 Trino** 时才是越权漏洞，发送 Display 后均消失（第 6 节）。`GenericDialect` 覆盖度最高，选它。

## 5. 语料结果（GenericDialect + 白名单 visitor）

「Trino 435」列为官方解析器对原文的结果（`Query` 等为语句类名，`ERR` 为语法错误）。「判定」：ok = 符合预期；FR = 误拒（可用性）；FA = 误放行（危险）。

| ID | 内容 | Trino 435 | sqlparser 结果 | 判定 |
|---|---|---|---|---|
| G01 | 三段名简单 SELECT | Query | Query，rels=hive.sales.orders | ok |
| G02 | 多表 JOIN / LEFT JOIN | Query | 3 个 relation 全部提取 | ok |
| G03 | CTE | Query | CTE 名不当作物理表 | ok |
| G04 | WITH RECURSIVE + VALUES | Query | ok | ok |
| G05 | 窗口函数 + frame | Query | ok | ok |
| G06 | CROSS JOIN UNNEST AS t(tag) | Query | `TableFactor::UNNEST` | ok |
| G07 | UNNEST WITH ORDINALITY | Query | `TableFactor::UNNEST` | ok |
| G08 | lambda `transform(arr, x -> x + 1)`、`reduce` | Query | ok | ok |
| G09 | TRY_CAST | Query | ok（Display 把类型名大写，等价） | ok |
| G10 | AT TIME ZONE | Query | ok | ok |
| G11 | ROW / ARRAY / MAP 字面量，`CAST(.. AS ROW(x BIGINT, y DOUBLE))` | Query | 解析成功，但 **Display 输出 `ROW(x, BIGINT, y, DOUBLE)`** | ok（往返缺陷，见 6.2） |
| G12 | DATE / TIMESTAMP / INTERVAL 字面量 | Query | ok | ok |
| G13 | 引号标识符 `"pg"."public"."Orders"` | Query | 小写化后匹配 allow-list | ok |
| G14 | IN 子查询 + EXISTS | Query | 子查询内 relation 全部提取 | ok |
| G15 | LATERAL 子查询 | Query | `Derived{lateral: true}`，内部 relation 提取 | ok |
| G16 | `ORDER BY id OFFSET 10 LIMIT 5` | Query | 解析成功，**Display 变成 `LIMIT 5 OFFSET 10`，Trino 报语法错** | ok（往返缺陷，见 6.2） |
| G17 | OFFSET .. FETCH FIRST .. ROWS ONLY | Query | ok | ok |
| G18 | UNION ALL / EXCEPT | Query | ok | ok |
| G19 | GROUPING SETS + HAVING | Query | ok | ok |
| G20 | `FILTER (WHERE ..)`、`array_agg(x ORDER BY y)` | Query | ok | ok |
| G21 | 字段访问 / 下标 `arr[1]`、`m['k']` | Query | ok | ok |
| G22 | IS DISTINCT FROM、LIKE .. ESCAPE '\\'、BETWEEN | Query | ok（generic 不做反斜杠转义） | ok |
| G23 | TABLESAMPLE BERNOULLI | Query | ok | ok |
| G24 | `(VALUES ..) AS v(id, name)` | Query | ok | ok |
| G25 | `json_query(j, 'lax $.a' WITH ARRAY WRAPPER)` | Query | 解析失败 | FR |
| G26 | 末尾分号 `SELECT .. ;` | **ERR** | 1 条语句，Display 去掉分号 | ok（Display 反而修复） |
| G27 | MATCH_RECOGNIZE | Query | ok | ok |
| G28 | `U&'\0041'`、`'it''s'` | Query | ok，Display 为 `U&'A'`，等价 | ok |
| G29 | `FOR TIMESTAMP AS OF ..`（Iceberg time travel） | Query | 解析失败 | FR |
| G30 | `FOR VERSION AS OF ..` | Query | 解析失败 | FR |
| G31 | 块注释内藏 `; DROP ..` + 行尾注释 | Query | 1 条语句，注释丢弃 | ok |
| G32 | `LIMIT 100000` | Query | 注入后收紧为 `LIMIT 1000` | ok |
| G33 | `a.b.c.d` 列引用 | Query | ok | ok |
| G34 | `'a\'`（Trino 中是合法字符串） | Query | ok | ok |
| G35 | `U&'\0027'` 及含 `) UNION SELECT * FROM edit..` 的 U& 字符串 | Query | Display 正确转义为 `''`，Trino 解析为同一字符串 | ok |
| G36 | IF / TRY / extract / position / substring FROM FOR / trim BOTH | Query | ok | ok |
| G37 | `CAST(x AS ARRAY(VARCHAR))`、`MAP(VARCHAR, BIGINT)`、`TIMESTAMP(3) WITH TIME ZONE` | Query | 解析失败（第 89 列 `ARRAY(` 处期待 `<`） | FR |
| G38 | `listagg .. WITHIN GROUP`、count(DISTINCT) | Query | ok | ok |
| G39 | WINDOW 子句 + NULLS LAST | Query | ok | ok |
| G40 | `DOUBLE '1.0'`、`DECIMAL '1.2'`、`X'0A'`、`current_user` 等 | Query | ok | ok |
| G41 | `"$path"`、`hive.s."t$partitions"` 元数据表 | Query | relation=hive.s.t$partitions | ok |
| G42 | FULL OUTER JOIN USING | Query | ok | ok |
| G43 | `> ALL (子查询)` | Query | ok | ok |
| G44 | JSON_TABLE | Query | `TableFactor::JsonTable`，被白名单保守拒绝 | FR（可放开） |
| G45 | ROLLUP / CUBE | Query | ok | ok |
| G46 | 顶层 VALUES | Query | `SetExpr::Values` | ok |
| G47 | `LIMIT ALL` | Query | Display 省略 `LIMIT ALL`，等价 | ok |
| G48 | FETCH FIRST 5 ROWS WITH TIES | Query | ok（注入时去掉 WITH TIES） | ok |
| B01 | `TABLE(pg.system.query(query => 'DELETE ..'))` | Query | `TableFactor::TableFunction` → 拒 | ok |
| B02 | `TABLE(system.sequence(..))` | Query | TableFunction → 拒 | ok |
| B03 | `FROM pg.system.query(query => ..)`（无 TABLE） | ERR | `Table{args: Some}` → 拒 | ok |
| B04 | CTE 内藏 TABLE(system.query) | Query | 拒（visitor 进入 CTE） | ok |
| B05 | EXISTS 子查询内藏 TABLE(..) | Query | 拒 | ok |
| B06 | 逗号 join `, TABLE(..) q` | Query | 拒 | ok |
| B07 | `LATERAL pg.system.query(..)` | ERR | `TableFactor::Function` → 拒 | ok |
| B08 | CALL | Call | `Statement::Call` → 拒 | ok |
| B09 | EXECUTE .. USING | Execute | 拒 | ok |
| B10 | EXECUTE IMMEDIATE | ExecuteImmediate | `Statement::Execute` → 拒 | ok |
| B11 | PREPARE .. FROM | Prepare | 解析失败 → 拒 | ok |
| B12 | SET SESSION | SetSession | `Statement::Set` → 拒 | ok |
| B13 | RESET SESSION | ResetSession | 解析失败 → 拒 | ok |
| B14 | USE | ERR（435 未实现） | `Statement::Use` → 拒 | ok |
| B15-B16 | START TRANSACTION / COMMIT | 对应语句 | 拒 | ok |
| B17-B22 | INSERT / DELETE / MERGE / CTAS / DROP / ALTER | 对应语句 | 拒 | ok |
| B23 | `SELECT ..; DROP ..` | ERR | 2 条语句 → 拒 | ok |
| B24 | 行注释后换行再 `; DROP` | ERR | 2 条语句 → 拒 | ok |
| B25 | `/* /* */ ; DROP ..; /* */` | ERR | 嵌套注释未闭合 → 解析失败 | ok |
| B26 | EXPLAIN ANALYZE INSERT | ExplainAnalyze | `Statement::Explain` → 拒 | ok |
| B27 | SHOW TABLES | ShowTables | 拒 | ok |
| B28 | 全角分号 `；` | ERR | 解析失败 | ok |
| B29 | 关键字中插零宽空格 | ERR | 解析失败 | ok |
| B30 | `WITH FUNCTION f() .. SELECT f()` | Query | 解析失败 → 拒 | ok |
| B31 | `WITH SESSION k = v SELECT ..` | ERR（435 无此语法） | 解析失败 → 拒 | ok |
| B32 | `WITH d AS (DELETE .. RETURNING *)` | ERR | `SetExpr::Delete` → 拒 | ok |
| B33 | `TABLE edit.s.secret` | **Query** | 解析失败 → 拒 | ok（见风险 R3） |
| B34 | `FROM (TABLE edit.s.secret)` | **Query** | 解析失败 → 拒 | ok（见风险 R3） |
| B35 | `FROM edit.s.t` | Query | allow-list 外 → 拒 | ok |
| B36 | `"EDIT"."S"."T"` | Query | 小写化后 allow-list 外 → 拒 | ok |
| B37 | 未限定表名 `FROM t` | Query | 非三段名 → 拒 | ok |
| B38 | 引用不存在的 CTE 名 | Query | 非三段名 → 拒 | ok |
| B39 | SELECT INTO | ERR | `select.into` → 拒 | ok |
| B40 | FOR UPDATE | ERR | `query.locks` → 拒 | ok |
| B41-B42 | DEALLOCATE / GRANT | 对应语句 | 拒 | ok |
| B43 | 标量调用 `pg.system.query('x')` | Query | 限定名函数 → 拒 | ok |
| B44 | SET TIME ZONE | SetTimeZone | `Statement::Set` → 拒 | ok |
| B45 | ANALYZE | Analyze | 拒 | ok |
| B46 | REFRESH MATERIALIZED VIEW | RefreshMaterializedView | 解析失败 → 拒 | ok |
| B47-B50 | edit 表藏在 IN 子查询 / 标量子查询 / UNNEST 参数子查询 / JOIN USING | Query | 全部提取到 edit.* → 拒 | ok |
| B51 | 允许 schema 内的同形字（西里尔 е）表名 | Query | 放行 | ok（allow-list 为 schema 粒度，属预期） |
| B52 | 反引号 `` `edit`.s.t `` | ERR | generic 接受反引号，提取到 edit → 拒 | ok |
| B53 | `FROM hive.s.t /* /* */ , edit.s.secret -- */` | **Query：`FROM hive.s.t , edit.s.secret`** | 嵌套注释吞掉 `, edit.s.secret`，只看到 hive.s.t | **FA（发送原文时）**；Display 为 `SELECT 1 FROM hive.s.t`，安全 |
| B54 | `'x\', 1 FROM edit.s.secret -- ' FROM hive.s.t` | Query：`FROM edit.s.secret` | generic 正确识别 edit → 拒；反斜杠转义方言 FA | ok |
| B55 | catalog 名用同形字 `"hіve"` | Query | allow-list 外 → 拒 | ok |
| B56 | `TABLE(exclude_columns(input => TABLE(edit.s.t), ..))` | Query | TableFunction → 拒 | ok |

汇总 [verified: experiment + trino-parser 435]：generic 下 49 条应放行中 44 条通过、5 条误拒（G25 / G29 / G30 / G37 / G44）；55 条应拒绝中 54 条拒绝，唯一误放行 B53 仅在发送原文时成立。

## 6. 往返风险：发 Display 还是发原文

### 6.1 结论：发送规范化后的 Display，不发原文

把 45 条被放行语句的原文与 Display 分别交给 Trino 435 解析器：39 条 Trino AST 完全一致，6 条不同 [verified: experiment]：

- **B53（安全）**：原文在 Trino 中读到 `edit.s.secret`，Display 只有 `hive.s.t`。发原文 = 校验器看到的语句和 Trino 执行的语句不是同一条，allow-list 可被绕过。
- G26：原文末尾 `;` 被 Trino 拒绝，Display 去掉分号后可执行。
- G09 / G47：类型名大写、`LIMIT ALL` 省略，语义等价。
- G16、G11：Display 缺陷，见 6.2。

发 Display 的本质收益：Trino 执行的文本由「已校验的 AST」生成，注释被丢弃、字符串与标识符按 sqlparser 自己的规则重新转义（G35 已证明 U& 字符串里的 `'` 被正确转义为 `''`），sqlparser 与 Trino 之间的**词法差异**（注释嵌套、反斜杠转义、未来新增的词法规则）不再能造成「校验的是 A，执行的是 B」。剩下的风险是 Display 本身有 bug，它的失败模式通常是 Trino 报语法 / 类型错误（fail-closed），而不是越权，因为 Display 不会凭空生成 AST 中不存在的 relation。

发原文的唯一优点是 100% 保留用户语义；但它要求 sqlparser 与 Trino 词法逐字一致，这一点 B53 已经证伪。

### 6.2 已知 Display 缺陷与对策

| 缺陷 | 证据 | 对策 |
|---|---|---|
| `LIMIT m OFFSET n` 顺序与 Trino 相反 | G16：Trino 报 `mismatched input 'OFFSET'` [verified] | 用 `VisitorMut::post_visit_query` 把 `LimitOffset{limit: Some, offset: Some}` 改写为 `offset` + `Fetch{quantity: limit}`；输出 `OFFSET 10 FETCH FIRST 5 ROWS ONLY`，嵌套子查询与外层均被 Trino 435 接受 [verified: experiment F1/F2] |
| `ROW(x BIGINT, y DOUBLE)` 类型被解析为 `DataType::Custom(ROW, ["x","BIGINT","y","DOUBLE"])`，结构在解析时已丢失 | G11 [verified] | 白名单拒绝 `DataType::Custom` 中含非数字 modifier 的类型，返回明确错误；等上游支持后再放开 |
| 未知的其他有损 Display | 未穷举 | 校验后做 round-trip 自检：`parse(display(ast)) == ast`，不等即拒绝。本语料 45 条全部相等 [verified]，但 G11 说明「相等」不代表与 Trino 语义一致，因此自检只是兜底 |

## 7. 推荐设计

管线（全部在 ontoforge 内完成，Trino 只收到最终文本）：

1. 字节上限（建议 32 KB，可配置），超限拒绝。理由见风险 R1。
2. `Parser::parse_sql(&GenericDialect {}, sql)`；失败即拒绝，审计记录原文与解析错误。
3. 语句数必须为 1，且为 `Statement::Query`。
4. 白名单 visitor（`sqlparser::ast::Visitor`，`features = ["visitor"]`）：
   - `pre_visit_statement`：非 `Statement::Query` 一律拒绝（也覆盖 `SetExpr::Insert(Statement)` 等嵌套语句）。
   - `pre_visit_query`：拒绝 `locks`、`for_clause`、`settings`、`format_clause`、`pipe_operators`、`LIMIT BY`、`LIMIT a, b`、CTE 的 `materialized` / `from`；记录 CTE 名（小写）；递归检查 `body`：只允许 `Select`、`Query`、`SetOperation`、`Values`，拒绝 `SetExpr::Table`（绕过 relation 回调）与 DML。
   - `Select`：拒绝 `into`、`lateral_views`、`top`、`prewhere`、`connect_by`、`cluster_by` / `distribute_by` / `sort_by`、`qualify`、`value_table_mode`、`optimizer_hints`、`exclude`、`select_modifiers`、非 `SelectFlavor::Standard`（这些都是其他方言语法，Trino 不需要）。
   - `pre_visit_table_factor`：只允许 `Table{args: None, with_hints 空, partitions 空, json_path None, index_hints 空, with_ordinality false}`、`Derived`、`UNNEST`、`NestedJoin`、`MatchRecognize`；其余（`TableFunction`、`Function`、`JsonTable`、`OpenJsonTable`、`Pivot`、`Unpivot`、`XmlTable` 等）拒绝。默认拒绝，新增变体自动落入拒绝分支。
   - `pre_visit_expr`：`Expr::Function` 名称多于 1 段即拒绝（B43，同时挡住 catalog 内存储的函数）；`Expr::Cast` 的 `DataType::Custom` 含非数字 modifier 时拒绝（G11）。
   - `pre_visit_relation`：收集 `ObjectName`，各段取 `Ident.value` 后 `to_lowercase()`。
5. relation 判定：单段且命中 CTE 名 → 跳过；否则必须恰好三段，且 `(catalog, schema)` 在 allow-list 内，edit store catalog 永不入 allow-list。L2 执行时**不设置**会话默认 catalog / schema，使任何漏判的非限定名在 Trino 侧报错（CTE 作用域未精确建模，靠这一点兜底）。
6. 规范化：`VisitorMut` 把 `LIMIT + OFFSET` 改写为 `OFFSET + FETCH`。
7. 行数上限：只改最外层 `Query`。已有字面量 `LIMIT n` / `FETCH n` 且 `n <= cap` 保持；否则置为 `cap + 1`（多取 1 行用于判断 `truncated`）；去掉 `FETCH .. WITH TIES` / `PERCENT`。不要用 `SELECT * FROM (<q>) LIMIT n` 包裹，以免内层 `ORDER BY` 语义丢失 [unverified: Trino 对子查询 ORDER BY 的处理]。client 侧再按行数 + 字节上限截断并取消查询，作为第二道防线（与 spike-01 的 client 设计一致）。
8. round-trip 自检：`parse(display) == ast`，否则拒绝。
9. 执行 Display 文本；审计同时记录原文、执行文本、提取的 relation 列表、拒绝原因。

建议封装为独立 crate 内的单一入口（如 `fn check_l2(sql, &AllowList, cap) -> Result<L2Plan, L2Rejected>`），把 sqlparser 类型挡在模块内部，方便升级 0.x 版本时只改一处；`Cargo.toml` 锁定 `sqlparser = "=0.63.0"`，升级必须跑完本语料（作为 conformance 测试固化）。

## 8. 风险

- **R1 DoS：AST Drop 栈溢出（不可捕获，进程直接退出）**。`SELECT 1 + 1 + ... FROM hive.s.t`：2 000 / 5 000 项正常；10 000 项（约 40 KB）时解析、visit、Display 均成功，但 `drop(ast)` 在 Windows release 主线程（1 MB 栈）上 `has overflowed its stack`；括号嵌套 60 层则被递归上限挡住，返回 `recursion limit exceeded` [verified: experiment]。对策：字节上限 + 在专用大栈线程（如 8 MB）里完成解析 / 校验 / drop；把等价用例加入测试。Linux 生产环境的阈值未测 [unverified]。
- **R2 sqlparser 不是 Trino 语法的严格子集**。它接受很多 Trino 不支持的语法（反引号、其他方言子句），白名单 visitor 已把这些逐项挡在外面；未列举到的字段若在新版本出现，需要升级时复查。Trino 侧真正的权限边界仍是 L2 身份的 Trino ACL（设计文档 OV2），sqlparser 是第一道防线而非唯一防线。
- **R3 未来版本放行 `TABLE t` 查询形式**。Trino 支持 `TABLE edit.s.secret`（B33 / B34），当前 generic 解析失败所以安全；若上游新增支持，它会以 `SetExpr::Table` 出现，第 7 节规则已显式拒绝。
- **R4 Display 缺陷未穷举**：只发现 G16 与 G11 两处，均为 fail-closed（Trino 报错，不越权）。G11 在 Trino 侧解析成功、预计分析阶段报未知类型 [unverified]。
- **R5 对照解析器版本**：Trino 435 比当前版本旧（spike-01 记录当前文档为 Trino 484）。`WITH SESSION` 等新语法在 435 不存在，sqlparser 对它们解析失败即拒绝，方向安全；但新版 Trino 新增的、sqlparser 恰好能解析的读语法可能改变语义，需在 CI 用目标 Trino 版本复核。
- **R6 可用性**：Trino 专有读语法的误拒（JSON 函数子句、time travel、`ARRAY(T)` / `MAP(K,V)` 类型写法）。对 L2 受信用户可接受，但需要在错误信息中说明「L2 暂不支持该语法」。

## 9. Open items

1. 在真实 Trino（与生产同版本）上执行本语料的 Display 文本，确认 G11 的失败模式与 OFFSET/FETCH 改写的执行结果（依赖 spike-01 / spike-04 的 Trino 环境）。
2. 决定 32 KB 上限与大栈线程的具体数值，并在 Linux 上重测 R1 阈值。
3. JSON_TABLE（G44）是否放开：它是只读的，放开需确认其 `PATH` 表达式内不能引用表。
4. 是否向上游提 issue：`ROW(name type, ..)` 类型解析、`LIMIT/OFFSET` Display 顺序、`FOR TIMESTAMP|VERSION AS OF`，以及在 #2611 之外提议 Trino 方言（可选，不阻塞）。
5. 标量函数是否需要黑名单：本次只拒绝了限定名函数，未调研 Trino 内置标量函数是否存在副作用 [unverified]。

## 10. 来源

- crates.io API：`/api/v1/crates/sqlparser`、`/versions`（版本、日期、下载量）
- `sqlparser-0.63.0` 源码：`Cargo.toml`、`src/dialect/mod.rs`、`src/dialect/generic.rs`、`src/ast/query.rs`、`src/ast/visitor.rs`、`src/parser/mod.rs`
- GitHub：`apache/datafusion-sqlparser-rs` issue #2611（`gh issue view`）、#369、#384（`gh search issues`）
- Trino 语法：`trinodb/trino` master `core/trino-grammar/src/main/antlr4/io/trino/grammar/sql/SqlBase.g4`（`gh api`）
- Trino 解析器真值：Maven Central `io.trino:trino-parser:435`、`trino-grammar:435`、`antlr4-runtime:4.13.1`、`guava:32.1.3-jre`（SHA1 已核对）
- trino.io/docs/current/language/reserved.html（标识符大小写）
- 实验代码与输出位于 `%TEMP%\sqlspike`、`%TEMP%\trino_harness`，已在完成后删除
