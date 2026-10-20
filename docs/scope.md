# 授权范围（scope）规范

本文档面向 ontology 作者与管理员，说明 role 里的行谓词（`rowFilter`）怎么写、怎么求值。Role / RoleBinding 与 `scope plan` / `scope apply` 见本文「Role 与 RoleBinding（roles-as-code）」一节。

规则的决定记录见 `.planning/adr/0025-row-predicate-semantics.md`；实现在 `crates/ontoforge-expr/src/rowpred/`，文档与实现不一致时以测试 `crates/ontoforge-expr/tests/rowpred_subset.rs` 与 `rowpred_eval.rs` 为准。

## 行谓词（rowFilter）

行谓词决定一个主体能看到某个 object type 的哪些行。它是一个 CEL 布尔表达式，写在 role 的 `objectTypes.<Type>.rowFilter`：

```yaml
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

## Role 与 RoleBinding（roles-as-code）

Role 说明持有者能读什么、能做什么，RoleBinding 把 role 绑定到用户和组。两者都写成 YAML（或 JSON）文档，放在一个**与 ontology 目录分开的 scope 目录**里，用 `ontoforge scope plan` / `scope apply` 同步进系统库。可运行的例子见 `examples/scopes/aviation/`（配套的 ontology 是 `examples/ontologies/aviation/`，示例服务配置是 `examples/server/ontoforge-server.dev.toml`）。

### Role

```yaml
kind: Role
apiName: crewScheduler
displayName: Crew scheduler
objectTypes:
  Flight:
    properties: [flightId, aircraftTailNumber, scheduledDeparture, departureAirportCode]
    rowFilter: "object.departureAirportCode in principal.attrs.homeAirports"
actions: [assignPilot]
toolsets: [core, full]
```

| 字段 | 含义 |
|---|---|
| `apiName` | role 名，camelCase，RoleBinding 用它引用 role |
| `objectTypes.<Type>.properties` | 可读属性的白名单，必须含主键；不在表里的属性对持有者不可见 |
| `objectTypes.<Type>.rowFilter` | 行谓词，写法见上文「行谓词」；省略表示所有行可见 |
| `actions` | 可提交的 action type |
| `sqlLevels` | 在隐含的 L0 之外可用的 SQL 层级（`raw`、`admin`） |
| `l2.catalogs`、`l2.schemas` | 可直接查询的 Trino catalog 与 `catalog.schema` |
| `toolsets` | 可用的 MCP 工具集（`core`、`full`、`sql:raw`） |

role 里没有列出的类型、属性、action 一律不可见、不可用（fail-closed）。

### RoleBinding

```yaml
kind: RoleBinding
role: crewScheduler
subjects:
  - group: crew-schedulers
  - user: "ontoforge-dev|mia"
```

- `group` 是 JWT 里 `groups` claim 的组名。**组成员关系不落库**，每次请求取 token 里的组；配置了多个 OIDC issuer 时组名带 issuer 前缀，写成 `<iss>|<group>`。
- `user` 是主体 id，`<iss>|<sub>`。绑定引用的 user 不必已经登录过。
- dev token 的组：本地开发用 `ontoforge dev-token --sub mia --groups dispatchers,crew-schedulers` 签发 token。dev token 的 issuer 固定是 `ontoforge-dev`，所以 `--sub mia` 对应 `user: "ontoforge-dev|mia"`；`--groups` 里的名字原样对应 `group:`，dev token 的组从不加 issuer 前缀。
- 一个 role 至多一个 RoleBinding 文档。

### 先预览，再写入：plan 与 apply

```text
ontoforge reload examples/ontologies/aviation --server-config examples/server/ontoforge-server.dev.toml
ontoforge scope plan  examples/scopes/aviation --ontology examples/ontologies/aviation --server-config examples/server/ontoforge-server.dev.toml
ontoforge scope apply examples/scopes/aviation --ontology examples/ontologies/aviation --server-config examples/server/ontoforge-server.dev.toml --plan-hash <HASH>
```

`scope plan` 只读，输出每一项变化，最后一行是 plan hash：

```text
add role dispatcher
add binding dispatcher group dispatchers
remove binding crewScheduler group crew-schedulers
create index of_ix_of_obj_flight_departure_airport_code
plan hash: 5a0c3e91d27b84f6
```

`scope apply --plan-hash <HASH>` 在系统库的一个事务里重新计算 plan，hash 与审阅过的不同就拒绝（退出码 1，什么也不写），所以审阅之后目录或系统库再变都不会被悄悄写入。没有变化的 plan 也能 apply，它仍然留一条审计。

**全量同步。** apply 之后系统库里的 role 与绑定等于 scope 目录：目录里没有的 role 与绑定会被删除，plan 里逐条列出（`remove role ...`、`remove binding ...`）。空目录会删除全部 role 与绑定，plan 同样列出。plan 的输出与 hash 与 YAML 里的书写顺序无关：role 按 apiName、绑定按 (role, 种类, 主体) 排序，`properties`、`actions`、`sqlLevels`、`toolsets`、`l2` 里的列表在存储与计算 hash 前排序去重。

**apply 前的离线校验。** `plan` 与 `apply` 在打开系统库之前，先对 ontology、所选 profile 的绑定和 `[auth.principal_attrs]` 做完整检查：引用的类型、属性、action 是否存在（ONT-V21），行谓词是否合法（ONT-V22，包括不能作用在 profile 声明为大小写不敏感的列上），role 持有的 action 的提交条件读到的属性是否都在白名单里（ONT-V23）。任何一条诊断都使命令以退出码 1 结束，系统库不发生任何写入，role 不会部分生效。`--ontology` 指向的定义还必须与系统库当前生效的定义一致，否则提示先运行 `ontoforge reload`。

退出码：0 成功，1 诊断（scope、ontology、配置，或 plan 已变化），3 系统库不可用（ONT-V26）。

**行谓词的索引。** apply 会为行谓词读到的属性建 `of_ix_<table>_<column>` 索引（已有的跳过）。索引只增不删：撤掉一个 role 不会删除索引。

### 审计

每次 apply 在 `of_admin_audit` 里追加一条记录：操作者、完整的 plan（JSON）、apply 前后 scope 的 hash。操作者是运行命令的 OS 用户加主机名（`<user>@<host>`）。这张表与 edit log 一样只能追加，UPDATE、DELETE（PostgreSQL 上还有 TRUNCATE）都被触发器拒绝；触发器不是针对表所有者的边界，部署时还要收回应用账号对它的改写权限。管理通道目前是部署方本机的命令行，操作者身份不经过认证。

### 谁能改 role

role 与绑定只能经 `scope apply` 修改：storage 里只有 `scope_store` 模块写这几张表，源码守卫测试（`scope_tables_written_only_by_scope_store`）强制这一点。后续 REST 管理端（Phase 7）对 role 只读，MCP 没有任何改动 role 的工具。

### 用户记录

主体首次通过认证时自动建一条 user 记录（`of_user`），只存主体 id、issuer、subject、创建时间和最近登录时间，**不存**组、claim 和任何权限。同一主体的最近登录时间在一个进程内至多每 60 秒写一次。

### 多个 role 合并后的可见性

主体可以同时持有多个 role（直接绑定的和经组得到的，同一个 role 经两条路径到达只算一次）。同一类型上：

- **行**：任一 role 的行谓词为真，这一行就可见（OR）。没有行谓词的 role 让所有行可见。
- **列**：一列只有在授予该列的 role 的行谓词对这一行为真时才显示值，否则该列对这一行遮蔽。所以「A 的列配 B 的行」不会发生。
- action、SQL 层级、L2 范围和工具集取各 role 的并集。
