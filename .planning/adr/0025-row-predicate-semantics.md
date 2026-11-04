# ADR-0025 行谓词语义：只用 cel 解析、自有 `Pred` IR、SQL 三值逻辑、unknown 不可见

- status: accepted
- date: 2026-10-20
- decider: Claude（orchestrator 按用户「决策自主」偏好在 03-05 Task 1 checkpoint 选定 option-a；用户可事后否决。否决等于改变 role 的可见范围，须先停用已写入的 role，且 03-11 的差分语料与反例集作废）
- supersedes: ADR-0018 Decision「主体」一条中「`principal.attrs` 缺失按 null，相关谓词求值为 false（fail-closed）」一句（其余内容不变；ADR-0018 的状态行已加注）
- 细化：ADR-0006（表达式语言）、ADR-0018（行谓词只能引用同行属性与 `principal.attrs`）
- 依据：`.planning/phases/03-edit-store-scope-model/03-CONTEXT.md` D-05 / D-06 / D-07，`03-RESEARCH.md` Pattern 6 与 Pitfall 3 / 4，`03-05-PLAN.md` Task 1

## Context
ADR-0018 要求同一份 CEL 行谓词既编译成 SQL（Trino / SQLite），也在 Rust 内对单行求值（PG 直读快速路径），并由差分测试强制两者等价。这要求「谓词的含义」只有一个定义，而且 SQL 一侧的行为是不可改的事实：`WHERE` 对 NULL 的处理是 SQL 三值逻辑。

`cel` 0.15 的运行时与此相反。本机实测（`Program::compile` 后 `execute`，变量 `x` 为 `null`）：`x == 'a'` 得 `false`，`x != 'a'` 得 `true`。如果用 `cel` 求值，null 列在 `object.x != 'a'` 下可见，而同一谓词编译成 SQL 后不可见，两条执行路径对同一行给出相反的授权结果。

「缺失主体属性求值为 false」一句在 ADR-0018 里没有给出机制。在三值逻辑下 `NOT (x == 'a')` 对缺失属性是 unknown 而不是 true，「相关谓词为 false」不能同时对 `p` 与 `NOT p` 成立，因此该句不可实现，由本 ADR 取代。

## Decision

### 1. 管线
- 行谓词文本只用 `cel` 0.15 **解析**（经 `ontoforge_expr::analyze` 的大栈线程，长度与嵌套预检、panic 转 `ExprError::Internal`），在闭包内立即降为自有的 `Pred` IR（`ontoforge_expr::rowpred::ir`），`cel::Program` 不离开闭包。
- 求值与 SQL 编译都从 `Pred` 出发，不使用 `cel` 运行时：Rust 一侧是 `rowpred::eval`，SQL 一侧是 `ontoforge-auth::scope::lower`（03-11）。
- `Pred` 的 Display 输出可被 `compile_row_filter` 再解析的规范 CEL 文本，`compile(display(p)) == p` 由 proptest 覆盖。

### 2. 允许的子集（D-07）
`==` `!=` `<` `<=` `>` `>=`；`&&` `||` `!`；`== null` / `!= null`（null 在任一侧，规范化为 `IsNull`）；`in`，右侧为非空或空的列表字面量，或 list 型 `principal.attrs.<name>`；`startsWith` / `endsWith` / `contains`，目标为 string 属性，参数为 string 字面量或 string 型 `principal.attrs.<name>`；`true` / `false` 常量。

拒绝并各给出带表达式内 line:col 的 `PredIssue`：宏（`all` `exists` `exists_one` `map` `filter`）、`has()`、`matches`、算术、`size`、三元、索引、其他函数、bytes 与 uint 字面量、裸的属性（`object.flag` 须写成 `object.flag == true`）。同一表达式的所有问题一并返回。

### 3. 同行限制
自由变量根只允许 `object` 与 `principal.attrs`。`object.<name>` 必须是该 object type 的现存属性；`principal.attrs.<name>` 必须在 `ontoforge-server.toml` 的 `[auth.principal_attrs]` 声明（D-05）；`principal.id` 与 `principal.groups` 本 phase 不开放；不允许引用 link 或其他对象。

### 4. 类型规则（比 `infer.rs` 严）
- 比较两侧必须同族：bool、整数族（byte / short / integer / long）、double 族（float / double）、string。**int 与 double 不得混比**（`infer.rs` 允许混比，行谓词不允许，因为 Rust 与 SQL 对隐式转换的结果不同）。
- `<` `<=` `>` `>=` 只用于 string 与数值，bool 只能 `==` / `!=`。
- date、timestamp、decimal、array、geopoint 属性不得出现在比较、`in` 与字符串匹配中（CEL 没有这些类型的字面量，Rust 与 SQL 的比较语义需另立规则）。`== null` / `!= null` 对除 geopoint 外的任意属性类型可用。
- 列表字面量元素同族，且**不得含 `null`**。list 型主体属性同样不得含 null 元素（`Attrs::insert` 拒绝，JWT 映射在 03-07 同口径）。
- list 型主体属性只能出现在 `in` 右侧。

### 5. 三值规则（D-06，Task 1 确认）
Rust 单行求值器实现 SQL 三值（Kleene）逻辑，值域 `True | False | Unknown`，**最终 `Unknown` 视为不可见**。

| 构造 | 规则 |
|---|---|
| `a op b`（六种比较） | 任一侧为 Null → Unknown；否则二值 |
| `x == null` / `x != null` | 恒为二值 |
| `&&` | 任一侧 False → False；否则任一侧 Unknown → Unknown；否则 True |
| `\|\|` | 任一侧 True → True；否则任一侧 Unknown → Unknown；否则 False |
| `!` | Unknown 保持 Unknown |
| `x in [e1 .. en]`，n ≥ 1 | 有相等元素 → True；x 为 Null 或某元素为 Null 且无匹配 → Unknown；否则 False |
| `x in []` | **恒为 False，x 为 Null 时也是**；SQL 侧编译为常量 `FALSE` |
| `x in principal.attrs.<list>` | 该属性缺失（Null）→ Unknown；存在则按上两行 |
| 字符串匹配 | 任一侧为 Null → Unknown；否则按 UTF-8 字节 `starts_with` / `ends_with` / `contains`，**区分大小写，通配符按字面**，空 needle 对任意非 Null 字符串为 True |

四条具体规则（Task 1 option-a）：
1. `x in []` 恒为 False，即使 x 为 Null；SQL 侧编译为常量 FALSE。
2. 列表字面量与 list 型主体属性不得含 null 元素（JWT list claim 含 null 元素在映射阶段被拒绝，见 03-07）。Rust 求值器仍实现完整的 SQL `IN`（含 Null 元素），以便未来放开。
3. 字符串匹配区分大小写、通配符按字面。SQLite 方言用 `GLOB` 而不是 `LIKE`（03-11），needle 转义 `[` `*` `?`；Trino 方言用带 `ESCAPE` 的 `LIKE`，转义 `\` `%` `_`。
4. fail-closed 只来自「Unknown 不可见」，**不对缺失属性特判**。因此 `principal.attrs.x == null || …` 会让没有 `x` 的主体看到那些行（Pitfall 4），这是已接受的后果，写入 `docs/scope.md` 并配示例。

缺失的主体属性在求值时为 Null（D-05）。**本条取代 ADR-0018 中「缺失按 null，相关谓词求值为 false」**：缺失属性使引用它的比较为 Unknown，而不是 False；`!` 之下仍为 Unknown，故任何不显式测试 null 的谓词对缺失属性的主体都是不可见的，fail-closed 的目的不变，机制改为三值逻辑。

### 6. 诊断级别（D-07 的细化）
超出子集的行谓词报**不可关闭的 V 级诊断 ONT-V22**，而不是 D-07 字面的 E 级。E 级是可在 `ontoforge.toml` 中关闭或降级的 lint 规则，而 SCOPE-06 与 D-07 要求超出子集的谓词「报错不生效」、不可被配置绕过。语义（拒绝集合）不变，只提高诊断级别。`PredIssue` 到 ONT-V22 的映射在 03-14。

## 证据
- SQLite 3.51.0：`NULL IN ()` = 0（规则 1）；`NULL IN (1,2)` = NULL；`1 IN (2,NULL)` = NULL；`1 IN (1,NULL)` = 1；`'a' LIKE 'A'` = 1 而 `'a' GLOB 'A'` = 0；`'a*c' GLOB 'a[*]c'` = 1（规则 3）。
- Trino 与 PG 不接受空 `IN ()`，故规则 1 在 SQL 侧用常量 `FALSE`，两端一致。
- `cel` 0.15 运行时：`null == 'a'` 为 false，`null != 'a'` 为 true，与 D-06 相反，因此只解析、不求值。
- `rowpred_eval` 的 `kleene_laws` proptest 锁定：De Morgan 两个方向、双重否定、交换律、`!` 与 Unknown、`IsNull` 恒二值且与取反互补、`x in [e1..en]` ≡ `x == e1 || … || x == en`、`x in []` ≡ False。

## Rejected
- **用 `cel` 运行时求值**：null 语义与 SQL 相反，同一谓词两条执行路径授权结果不同（上文证据）。
- **对缺失属性特判为 false**（ADR-0018 原句）：在三值逻辑下无法同时对 `p` 与 `!p` 成立，且需要在 Rust 与每个 SQL 方言里各加一个特判分支，差分测试多一类反例。
- **标准 CEL 的二值 null 语义**：与 SQL `WHERE` 的 NULL 处理不可调和；要保持等价只能在 SQL 里给每个比较加 `COALESCE`，改变索引使用并让下推到 Trino 失去意义。
- **字符串匹配编译为 SQLite `LIKE`**：`LIKE` 对 ASCII 不区分大小写，与 Rust 的字节匹配不等价。
- **允许列表含 null 元素**：`x IN (…, NULL)` 的三值陷阱（无匹配时是 NULL 而不是 FALSE）让 `!` 之下的可见性反转；本 phase 直接拒绝，求值器保持完整语义以备放开。

## Consequences
- 这是 one-way 决定：一旦有用户按此语义写了 role，改语义会静默改变可见范围（安全回归），03-11 的差分语料与反例集也随之作废。
- 03-11 的差分测试可以要求 Rust 与 SQLite 逐位相等（`True` / `False` / `Unknown` 与 SQL 的 TRUE / FALSE / NULL 一一对应，`WHERE` 只保留 TRUE）；Trino 一侧用固定种子语料与历史反例（D-09）。
- `object.x != 'a'` 在 x 为 null 时不可见；想让 null 可见须显式写 `object.x == null || object.x != 'a'`。**显式 null 分支也会放宽可见范围**：`object.region == null || object.region == principal.attrs.region` 让没有 `region` 属性的主体也看到未分配区域的行（Pitfall 4）。这些写入 `docs/scope.md`，作为已接受后果。
- double 列的 NaN 与 ±Infinity 不在 Rust ≡ SQL 的保证范围内（SQLite 把 NaN 存为 NULL，Trino 与 Rust 对 NaN 的比较语义不同）。若 Phase 4 读到 NaN，需重新评估。
- 行谓词引用大小写不敏感 collation 的列（D-08）在 03-11 / 03-14 处理，不在本 ADR。
