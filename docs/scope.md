# 授权范围（scope）规范

本文档面向 ontology 作者与管理员，说明 role 里的行谓词（`rowFilter`）怎么写、怎么求值。Role / RoleBinding 与 apply 的说明在后续章节补充。

规则的决定记录见 `.planning/adr/0025-row-predicate-semantics.md`；实现在 `crates/ontoforge-expr/src/rowpred/`，文档与实现不一致时以测试 `crates/ontoforge-expr/tests/rowpred_subset.rs` 与 `rowpred_eval.rs` 为准。

## 行谓词（rowFilter）

行谓词决定一个主体能看到某个 object type 的哪些行。它是一个 CEL 布尔表达式，写在 role 的 `grants.objectTypes.<Type>.rowFilter`：

```yaml
grants:
  objectTypes:
    Document:
      properties: [id, title, tenantId]
      rowFilter: "object.tenantId == principal.attrs.tenantId"
```

同一份谓词有两处执行：编译成 SQL 交给 Trino 或 SQLite，以及在 Rust 里对单行求值。两处的结果必须一致，所以行谓词只支持一个固定的 CEL 子集，并且 **null 的含义与标准 CEL 不同**（见下文「三值语义」）。

### 可以引用什么

只有两样东西：

| 写法 | 含义 |
|---|---|
| `object.<property>` | 这一行自己的属性。必须是该 object type 的现存属性 |
| `principal.attrs.<name>` | 当前主体的属性，来自 JWT claim。必须在 `ontoforge-server.toml` 的 `[auth.principal_attrs]` 里声明类型 |

不能引用其他行、其他对象或 link。`principal.id` 与 `principal.groups` 本版本不开放，需要按身份区分时先把它映射成一个 `principal.attrs`。

### 允许的写法

| 写法 | 说明 |
|---|---|
| `==` `!=` `<` `<=` `>` `>=` | 两侧必须是同一类值，见「类型规则」 |
| `&&` `\|\|` `!` | 照常嵌套，可用括号 |
| `x == null`、`x != null` | `null` 可以在任一侧 |
| `x in [..]` | 右侧是列表字面量（可以为空），或 list 型的 `principal.attrs.<name>` |
| `object.p.startsWith(s)`、`endsWith(s)`、`contains(s)` | `object.p` 必须是 string 属性；`s` 是字符串字面量或 string 型的 `principal.attrs.<name>` |
| `true`、`false` | 常量 |

示例：

```cel
object.tenantId == principal.attrs.tenantId
object.region in principal.attrs.regions
object.status != 'CANCELLED' && object.priority >= 3
object.code.startsWith(principal.attrs.prefix)
```

### 会被拒绝的写法

超出上表的写法在 `validate` / `apply` 时报 ONT-V22，错误带表达式内的行列号，同一表达式里的问题一并列出。ONT-V22 是不可关闭的 V 级诊断，`ontoforge.toml` 不能把它降级或关闭。

- 宏：`all`、`exists`、`exists_one`、`map`、`filter`
- `has()`、`matches`、`size`、其他函数、索引（`object['x']`）
- 算术（`+ - * / %`）和三元表达式 `a ? b : c`
- bytes 与 uint 字面量（`b'x'`、`1u`）
- 裸属性当条件：`object.flag` 要写成 `object.flag == true`
- 不存在的属性、未声明的 `principal.attrs.<name>`、`principal.id`、`principal.groups`、`request` 之类的其他变量

### 类型规则

- 比较两侧必须同一类：bool、整数（byte / short / integer / long）、浮点（float / double）、string。**整数与浮点不能混比**：`object.n == 1.5` 在 `n` 是 integer 时被拒绝，`object.d == 1` 在 `d` 是 double 时也被拒绝，写成 `1.0`。
- `<` `<=` `>` `>=` 只用于 string 与数值。bool 只能用 `==` 与 `!=`。
- date、timestamp、decimal、array、geopoint 类型的属性不能出现在比较、`in` 和字符串匹配里；除 geopoint 外，它们可以和 `null` 比较（`object.tags == null`）。
- 列表字面量里的元素类型必须一致，并且**不能含 `null`**：`object.region in ['a', null]` 被拒绝。想让 null 可见，用 `object.region == null || …`。
- list 型的 `principal.attrs.<name>` 只能放在 `in` 的右侧。JWT 里的 list claim 若含 null 元素，会在映射阶段被拒绝。

### 三值语义（与标准 CEL 不同）

行谓词按 SQL 的三值（Kleene）逻辑求值，结果是 **真、假、未知** 之一，**只有真才让这一行可见，未知等于不可见**。

比较里只要有一侧是 null，结果就是未知（不是假）。关键后果：

| 谓词 | `object.x` 为 null 时 |
|---|---|
| `object.x == 'a'` | 未知，不可见 |
| `object.x != 'a'` | **未知，不可见**（标准 CEL 里这是真） |
| `!(object.x == 'a')` | 未知，不可见 |
| `object.x == null` | 真，可见 |
| `object.x == null \|\| object.x != 'a'` | 真，可见 |

三值逻辑的运算表：

| 构造 | 规则 |
|---|---|
| `a && b` | 有一个为假 → 假；否则有一个未知 → 未知；否则真 |
| `a \|\| b` | 有一个为真 → 真；否则有一个未知 → 未知；否则假 |
| `!a` | 未知保持未知 |
| `x == null`、`x != null` | 永远是真或假，不会未知 |
| `x in [a, b]` | 有相等的元素 → 真；`x` 为 null → 未知；否则假 |
| `x in []` | **永远为假**，`x` 为 null 时也是 |
| `x in principal.attrs.regions` | 该属性缺失 → 未知；属性存在但是空列表 → 假 |

想让 null 的行可见，必须显式写出来：

```cel
object.region == null || object.region == principal.attrs.region
```

上面这条让 `region` 未分配的对象对所有主体可见（包括没有 `region` 属性的主体），已分配的对象只对 `region` 相等的主体可见。

### 主体属性缺失：已接受的后果

主体没有某个属性（JWT 里没有对应 claim，或映射结果为空）时，这个属性在求值时是 null。不显式测试 null 的谓词因此对该主体不可见，这是默认的 fail-closed。

**显式测试 null 会放宽可见范围。** 对没有 `region` 属性的主体，`object.region == null || object.region == principal.attrs.region` 在 `object.region` 为 null 的行上为真，即使主体根本没有区域。同样，直接写 `principal.attrs.region == null || …` 会让缺失该属性的主体可见那些行。规则里没有「缺失属性一律不可见」的特例；不可见只来自「未知等于不可见」。写 `== null` 分支之前，确认这是你要的范围。

这条规则取代了 ADR-0018 中「缺失属性求值为 false」的说法，决定见 ADR-0025。

### 字符串匹配

`startsWith`、`endsWith`、`contains` 区分大小写，匹配按 UTF-8 字节进行，通配符按字面：

- `object.code.startsWith('A')` 对 `"abc"` 为假。
- 参数 `'%'`、`'_'`、`'*'`、`'?'`、`'['` 只匹配含这个字符本身的值，不是通配符。
- 空参数 `''` 对任意非 null 字符串为真。
- 属性值为 null、或参数是缺失的 `principal.attrs.<name>`，结果为未知。

在 SQLite 上用 `GLOB`（不用 `LIKE`，后者对 ASCII 不区分大小写），在 Trino 上用带 `ESCAPE` 的 `LIKE`，转义由生成器完成。

### 浮点数

double 与 float 属性的比较只对有限值保证 Rust 与 SQL 一致。NaN 与无穷大不在保证范围内，不要在行谓词里依赖它们。

### `principal.attrs` 的来源

主体属性由 JWT claim 按 `ontoforge-server.toml` 的 `[auth.principal_attrs.<name>]` 映射得到，每个属性声明名字、claim 与类型（string、integer、boolean、string_list、integer_list）。行谓词在加载时按这些声明做静态类型检查，所以在请求时读到的值一定是声明的类型；不存在的值就是 null。
