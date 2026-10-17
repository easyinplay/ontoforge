# 数据源绑定规范

本文档说明 ontoforge 如何把 object type 与多对多（manyToMany）link type 绑定到 Trino 已有的物理表或 view，以及 `ontoforge validate --online` 如何校验这些绑定。类型映射表（D-13）以本文档为唯一来源，并由测试 `crates/ontoforge-engine/tests/doc_type_table.rs` 逐行对照代码，文档与实现不能矛盾。

## 概览

ontology 定义（object type、link type、action type）与物理表的绑定分开写：

- 定义文件只描述业务语义，不出现 catalog、schema、表名。同一份定义因此可以公开、可以在多个部署之间复用。
- 绑定文件（`kind: SourceBinding`）写在 `bindings/<profile>/` 下，说明某个 object type 或 link type 在这个 profile 里读哪张物理表、哪一列。profile 是部署环境的名字，例如 `dev` 用 SQLite 替身，`prod` 用 Trino。
- 对象数据的读取走绑定；写入仍然只能经 Action，并留下不可变的 edit log。

理由：定义与部署配置耦合，会让开源示例带上具体的表名，也无法在 dev 与 prod 之间切换绑定。

一个绑定只指向单张表或 view，不接受 SQL 片段。需要联表、改名或做类型转换时，在 Trino 里建 view，再绑定这个 view。

## 绑定文件

位置：`bindings/<profile>/` 下的任意 `.yaml` / `.yml` / `.json` 文件。profile 名由目录推出，格式为 `[a-z][a-z0-9_-]{0,31}`，只作为已加载数据的 key，不会拼进文件系统路径。一个文件可以含多个文档（YAML 用 `---` 分隔，JSON 用数组）。

字段：

| 字段 | 必填 | 说明 |
|---|---|---|
| `kind` | 是 | 固定为 `SourceBinding` |
| `target` | 是 | `{ objectType: Flight }` 或 `{ linkType: flightCrew }`，link 只允许 `manyToMany` |
| `source` | 是 | `{ catalog, schema, table }`，指向一张表或 view |
| `columns` | 否 | 与同名默认不同的列映射，key 是属性 apiName（link 是 `from` / `to`），值是 `{ column?, timezone?, collation? }` |
| `fixture` | 否 | `{ file }`，SQLite 替身使用的样例数据，见「fixture」 |

YAML 示例（object type）：

```yaml
kind: SourceBinding
target: { objectType: Flight }
source: { catalog: demo, schema: aviation, table: flights }
columns:
  flightId: { column: flight_id }
  flightNumber: { column: flight_no }
  # 该列没有时区，墙上时间按此时区读取。
  scheduledDeparture: { column: scheduled_departure, timezone: Europe/Berlin }
  # 列名相同，但比较时忽略大小写。
  status: { collation: caseInsensitive }
fixture: { file: fixtures/flights.csv }
```

多对多 link 绑定到连接表，`from` 与 `to` 是两端：

```yaml
kind: SourceBinding
target: { linkType: flightCrew }
source: { catalog: demo, schema: aviation, table: flight_crew }
columns:
  from: { column: flight_id }
  to: { column: pilot_id }
fixture: { file: fixtures/flight_crew.csv }
```

JSON 写法等价：

```json
[
  {
    "kind": "SourceBinding",
    "target": { "objectType": "Aircraft" },
    "source": { "catalog": "demo", "schema": "aviation", "table": "aircraft" },
    "columns": { "tailNumber": { "column": "tail_number" } },
    "fixture": { "file": "fixtures/aircraft.csv" }
  }
]
```

完整示例见 `examples/ontologies/aviation`（YAML）与 `examples/ontologies/aviation-json`（JSON）。

## backing

object type 与 manyToMany link type 必须写 `backing`，没有缺省值：

- `backing: source`：对象来自选定 profile 绑定的物理表，编辑叠加在其上。选定 profile 没有对应绑定时报 `ONT-V16`。
- `backing: editOnly`：对象只经 Action 或导入创建，不允许有绑定；有绑定时报 `ONT-V16`。

缺失 `backing` 报 `ONT-V05`（link 上的非法出现是 `ONT-V11`）。不设缺省值是有意的：漏写 `backing` 与绑定时，部署方会以为在读企业数据，实际读到的是空集。

## profile 选定

选定顺序：命令行 `--profile` > 环境变量 `ONTOFORGE_PROFILE` > manifest（`ontology.yaml`）的 `defaultProfile`。示例的缺省 profile 是 `dev`，对应 SQLite 替身。

## 列映射与同名默认

未在 `columns` 中列出的属性读同名列。`columns.<key>.column` 只在列名与 apiName 不同时才需要写。

列名按小写比较，因为 Trino 把标识符展示为小写。绑定列在物理表中有多个只差大小写的候选（例如 `Status` 与 `status`）时，列选择不确定，报错而不是任选一列：见「大小写歧义」。与绑定无关的列即使大小写冲突也不报告。

多对多 link 的 `from` 与 `to` 列按两端 object type 的主键类型比对。

## timezone 与 collation

- `timezone`：timestamp 属性必须声明。写 IANA 名（如 `Europe/Berlin`）表示源列没有时区、墙上时间按该时区读取；写 `aware` 表示源列自带时区。缺失报 `ONT-E14`。在线复核见「timezone 在线复核」。
- `collation`：`caseSensitive` 或 `caseInsensitive`，只用于 string 属性（及 string 数组、对应的 link 端列），用在其他类型上报 `ONT-E15`。

## fixture

fixture 是 SQLite 替身使用的样例数据，替身让 Windows 本机不需要 Trino 就能跑通全部测试。

- 格式：`.csv` 或 `.json`（JSON 是对象数组）。路径相对绑定文件所在目录，必须在 ontology 目录之内。
- 位置：必须放在 `bindings/<profile>/fixtures/` 下（可以有子目录，也可以引用其他 profile 的 fixtures 目录）。只有这个目录不参与定义文件扫描，所以解析到别处的 fixture 路径报 `ONT-V16`。
- 列：CSV 的表头、JSON 的键并集对应物理列名。绑定没有映射的多余列按 varchar 保留。
- NULL：CSV 的空单元格对所有类型（包括 string）都是 NULL；JSON 用真正的 `null`，缺失的键也是 NULL。
- 类型转换严格，错误只带文件、记录行号、列名与类型说明，不复述单元格内容：
  - 整数按属性范围检查（`byte` 为 8 位，依此类推），不接受前导 `+`。
  - 布尔只接受小写 `true` / `false`。
  - decimal 以字符串保存并规范化到 `scale` 位小数，从不舍入：多余的小数位或超出 `precision - scale` 的整数位直接拒绝。JSON 中 decimal 必须写成字符串。
  - date 为 `YYYY-MM-DD`。
  - timestamp 规范化为毫秒精度；小数秒超过 3 位拒绝而不是截断。声明了 IANA 时区的列写本地墙上时间（拒绝偏移）；`aware` 的列要求 `Z` 或 `±HH:MM`。
  - array 在 CSV 中写 JSON 数组文本，逐元素按元素类型转换。
- 上限：单个文件 4 MiB；全部 fixture 与定义文件合计 64 MiB；单个 fixture 至多 100 000 条记录；替身最多附加 10 个 `catalog__schema` 库。
- 共享表：同一物理表被多个绑定引用时，必须引用同一个 fixture 文件，同名列的类型必须一致。

## fixture 模式的验证边界

未给 `--server-config` 时，`validate --online` 用 fixture 加载内存 SQLite 替身。替身对外报告的物理列类型是由绑定与 IR 推导出来的：整数按属性范围报告 `tinyint` 到 `bigint`，`float` 报告 `real`、`double` 报告 `double`，decimal 用属性的 `precision` 与 `scale`，timestamp 为 `timestamp(3)`（`aware` 时带时区），string 为 `varchar`。

因此 fixture 模式下 `--online` 只验证两件事：

1. 绑定要求的表与列在 fixture 中存在。
2. fixture 的值可以按声明的类型转换。

它不能证明真实源列的类型兼容：类型表的判定在 fixture 模式下对着推导出来的类型比较，几乎总是通过。部署前必须用 `--server-config` 对真实 Trino 再跑一次。

## 类型兼容表

在线比对只接受无损映射（D-13）。规则：

- 整数只可加宽：属性的位宽不得小于源列。`integer` 属性读 `bigint` 列是 error。
- `float` 只接受 `real`；`double` 接受 `real` 与 `double`。
- `string` 接受 `varchar`；`char` 可读但有警告（值带尾部空格填充，`CharPadding`）。
- `date` 只接受 `date`，`boolean` 只接受 `boolean`。
- `array` 按元素递归比较，源列必须也是数组。
- `geopoint`、`number`（无界 numeric）与替身或 Trino 无法建模的类型一律 error，提示在 Trino 建 view 并做 cast。
- 跨类型族一律 error。

判定列：`OK` 表示没有任何不匹配；`W` 表示一条 warning（不降级，`ONT-V17` 严重级为 warning）；`E` 表示一条 error。属性声明列中 `decimal` 不写精度表示缺省的 `decimal(38,10)`；`(timezone: X)` 表示绑定为该列声明了 `timezone: X`；没有这个后缀表示没有声明。

<!-- type-table:start -->
| 属性声明 | 源列类型 | 判定 |
|---|---|---|
| `byte` | `tinyint` | OK |
| `byte` | `smallint` | E |
| `short` | `smallint` | OK |
| `short` | `integer` | E |
| `integer` | `smallint` | OK |
| `integer` | `integer` | OK |
| `integer` | `bigint` | E |
| `long` | `bigint` | OK |
| `long` | `integer` | OK |
| `integer` | `varchar` | E |
| `integer` | `double` | E |
| `float` | `real` | OK |
| `float` | `double` | E |
| `double` | `real` | OK |
| `double` | `double` | OK |
| `decimal(12,2)` | `decimal(12,2)` | OK |
| `decimal(12,2)` | `decimal(11,2)` | W |
| `decimal(12,2)` | `decimal(11,1)` | W |
| `decimal(12,2)` | `decimal(13,2)` | E |
| `decimal(12,2)` | `decimal(12,3)` | E |
| `decimal(12,2)` | `decimal(12,1)` | E |
| `decimal(12,2)` | `number` | E |
| `decimal` | `decimal(38,10)` | OK |
| `decimal` | `decimal(20,2)` | W |
| `decimal` | `decimal(38,9)` | E |
| `string` | `varchar` | OK |
| `string` | `varchar(255)` | OK |
| `string` | `char(3)` | W |
| `string` | `integer` | E |
| `boolean` | `boolean` | OK |
| `boolean` | `varchar` | E |
| `date` | `date` | OK |
| `date` | `timestamp(3)` | E |
| `timestamp (timezone: Europe/Berlin)` | `timestamp(3)` | OK |
| `timestamp (timezone: Europe/Berlin)` | `timestamp(3) with time zone` | W |
| `timestamp (timezone: aware)` | `timestamp(6) with time zone` | OK |
| `timestamp (timezone: aware)` | `timestamp(3)` | E |
| `timestamp` | `timestamp(3)` | E |
| `timestamp` | `timestamp(3) with time zone` | W |
| `array<integer>` | `array(integer)` | OK |
| `array<integer>` | `array(bigint)` | E |
| `array<decimal(10,2)>` | `array(decimal(10,2))` | OK |
| `array<decimal(10,2)>` | `array(decimal(9,2))` | W |
| `array<string>` | `array(char(2))` | W |
| `array<integer>` | `integer` | E |
| `geopoint` | `varchar` | E |
| `string` | `map(varchar,integer)` | E |
| `integer` | `row(a integer)` | E |
<!-- type-table:end -->

### decimal 的整数位规则

属性 `decimal(P,S)` 读源列 `decimal(p,s)`，兼容当且仅当：

```
s ≤ S  且  p - s ≤ P - S
```

`p - s` 是源列的整数位数，`P - S` 是属性能容纳的整数位数。两个条件分别保证小数部分和整数部分都不会丢失。兼容且 `p < P` 或 `s < S` 时报 warning（`PrecisionChanged`，精度变化但不丢值），否则 error。

只比较精度与小数位各自不超过声明值是不够的：`decimal(10,2)` 放进 `decimal(10,5)` 满足「精度不超过、小数位不超过」，但整数位从 8 位变成 5 位，会溢出。例如属性 `decimal(12,2)` 能容纳 10 位整数：源列 `decimal(12,1)` 有 11 位整数，所以是 error；源列 `decimal(10,0)` 有 10 位整数，是 warning。

`P` 与 `S` 取自属性的有效值：属性没写 `precision` / `scale` 时各自取缺省 38 与 10（ADR-0021），比对不另行处理缺省。`number`（无界 numeric）没有固定精度，一律 error，提示在 Trino 建 view 并 cast 成 `decimal(p, s)`。

### timezone 在线复核

| 绑定声明 | 源列 | 判定 |
|---|---|---|
| IANA 名 | 无时区 | OK |
| IANA 名 | 带时区 | warning：声明的时区不会被应用，应写 `aware` |
| `aware` | 带时区 | OK |
| `aware` | 无时区 | error |
| 未声明 | 无时区 | error |
| 未声明 | 带时区 | warning |

### 大小写歧义

绑定列小写化后匹配到多个物理列（例如 `Status` 与 `status`）时，`compare_binding` 报一条 `CaseCollision` error，消息列出全部候选的原始名，说明列选择不确定，并不再做类型比较。定位到 `source.table`（同名默认）或 `columns.<key>.column`（显式映射）。处理办法：在 Trino 建 view，以唯一的名字暴露其中一列，再绑定这个 view。

## 诊断码

诊断码一经发布不会复用或改义。

| 码 | 来源 | 严重级 | 含义 |
|---|---|---|---|
| `ONT-V05` | validate | error | 缺少必填字段，包括 object type 与 manyToMany link 缺 `backing` |
| `ONT-V11` | validate | error | link type 一致性，包括 `backing` 出现在非 manyToMany link 上 |
| `ONT-V16` | validate | error | 绑定定义与完备性：profile 名、绑定目标、fixture 引用与 fixture 内容；`source` 但选定 profile 无绑定、`editOnly` 却有绑定 |
| `ONT-V17` | validate --online | error 或 warning | 在线绑定不匹配：表不存在、列缺失、类型不兼容、精度或时区变化、char 填充、大小写歧义、不支持的类型 |
| `ONT-V18` | validate --check-pk | error | 主键列存在重复值或 NULL |
| `ONT-V19` | validate --online | error | 因引擎侧原因无法完成在线校验：不可用、认证失败、超时、协议或查询错误 |
| `ONT-V20` | validate / 服务配置 | error | 服务配置（`ontoforge-server.toml`）错误；也用于选定 profile 的绑定没有 fixture 又没有 `--server-config` 时 |
| `ONT-E13` | lint | error | decimal 的 `precision` 须在 1 到 38，`scale` 须在 0 到 `precision` |
| `ONT-E14` | lint | error | source 绑定的 timestamp 属性必须声明 `timezone`；在非 timestamp 属性上声明 `timezone` 也报 |
| `ONT-E15` | lint | error | `collation` 只能用于 string 属性、string 数组及对应的 link 端列 |
| `ONT-E16` | lint | error | `backing: source` 的 object type 不得含 geopoint 属性 |
| `ONT-W17` | lint | warning | fixture 的表头缺少绑定要求的列；表头无法读取或解析时，同一规则给出定位到 `fixture.file` 的「无法离线检查」提示，而不是静默跳过 |

lint 规则 `ONT-E13` 至 `ONT-E16` 与 `ONT-W17` 可在 `ontoforge.toml` 的 `[lint]` 中按 ID 调整级别或关闭。`ONT-V*` 是结构性错误，不可配置。

## 命令行与退出码

```
ontoforge validate <DIR> --online [--profile <NAME>] [--server-config <FILE>] [--check-pk] [--timeout <MS>]
```

- `--online`：对选定 profile 的每个绑定描述物理表并比对。未给 `--server-config` 时，选定 profile 的绑定必须全部带 `fixture`，用内存 SQLite 替身；否则报 `ONT-V20` 并提示 `--server-config`。
- `--profile <NAME>`：选定 profile，优先于环境变量 `ONTOFORGE_PROFILE` 与 `defaultProfile`。
- `--server-config <FILE>`：读取 `ontoforge-server.toml`，对映射到 Trino 的 profile 连接真实 Trino。配置映射到 Trino 而绑定仍带 `fixture` 时给出 warning。本 phase 不做服务配置的隐式查找。
- `--check-pk`：在显式要求时，对每个绑定的主键列查重复值。重复值查询排除 NULL 主键，NULL 主键单独计数，二者都报 `ONT-V18`。drift 检查不做这件事，因为它需要扫描全表。
- `--timeout <MS>`：覆盖每个元数据请求的超时（毫秒），只能与 `--online` 同用。

每个 catalog 只发一次元数据请求。

退出码：

| 码 | 含义 |
|---|---|
| 0 | 没有 error（warning 不影响，除非加 `--deny-warnings`） |
| 1 | 有 error：绑定不匹配、重复主键、配置错误等 |
| 2 | 命令行用法错误（clap） |
| 3 | 引擎侧原因导致在线校验无法完成（`ONT-V19`），与绑定错误区分 |
| 101 | 内部错误 |

不可用与认证失败这类整轮错误中止校验。单个绑定或单个 catalog 的查询类错误（例如对某个 catalog 没有权限）只让该 catalog 的绑定得到 `ONT-V19`，其余 catalog 的绑定照常比对；只要出现任一 `ONT-V19`，退出码就是 3。

## 服务配置

`ontoforge-server.toml` 只经 `--server-config` 指定。字段：

| 位置 | 字段 | 说明 |
|---|---|---|
| `[trino]` | `url` | Trino 地址，`http` 或 `https`，不得带用户信息 |
| `[trino]` | `ca_file` | PEM 文件，附加的根证书，与平台根证书合并 |
| `[trino]` | `connect_timeout_ms` | 连接超时，缺省 5000 |
| `[trino]` | `query_timeout_ms` | 单次查询超时，缺省 30000 |
| `[trino]` | `max_rows` / `max_bytes` / `max_page_bytes` | 结果行数、字节数与单页字节数上限，须大于 0 |
| `[trino]` | `next_uri_origins` | 除 `url` 外 `nextUri` 可以指向的 origin（Trino 前面有网关时用），形如 `scheme://host[:port]`，scheme 必须与 `url` 相同 |
| `[trino]` | `client_tags` | 客户端标签，每个 1 到 64 个字符，取自 `[A-Za-z0-9._:-]` |
| `[trino.identities.ontology]` | `user`、`auth`、`password_env` / `password_file`、`token_env` / `token_file` | ontology 服务身份；`auth` 为 `none`、`basic` 或 `bearer` |
| `[trino.identities.l2]` | 同上 | 可选的第二个身份，本 phase 只解析不使用 |
| `[profiles.<name>]` | `engine` | `"trino"` 或 `"sqlite"` |

`engine = "sqlite"` 即 fixture 驱动的内存替身；不提供指向现成 SQLite 库文件的目录配置项。`engine = "trino"` 要求有 `[trino]` 段。

密钥规则：

- 不接受明文密钥：出现 `password`、`token`、`secret` 字段会被拒绝。密钥只以环境变量名（`*_env`）或文件路径（`*_file`，相对配置文件所在目录，不超过 64 KiB）引用，错误信息只提变量名或路径，不含值。
- 不提供跳过证书校验的开关；自签名证书用 `ca_file` 加入信任。
- `basic` 与 `bearer` 认证只走 `https`，`url` 是 `http` 时报错。

示例：

```toml
[trino]
url = "https://trino.example.internal:8443"
ca_file = "certs/ca.pem"

[trino.identities.ontology]
user = "ontoforge"
auth = "basic"
password_env = "ONTOFORGE_TRINO_PASSWORD"

[profiles.prod]
engine = "trino"

[profiles.dev]
engine = "sqlite"
```

## drift 状态模型

drift 检查在运行时发现物理表与绑定不再一致，并只让受影响的目标降级，其余照常。本 phase 交付库函数与状态模型，启动时与定时执行、以及通过 `/health` 暴露由后续的服务进程接入。

`ontoforge_engine::bind_check::check_drift(engine, ontology, profile, ctx)` 描述选定 profile 的全部绑定并比对，返回 `DriftReport`，其中每个 object type 与 link type 是 `TargetHealth`：

- `Healthy { warnings }`：没有 error 级不匹配。精度、时区、char 填充这类 warning 保留在 `warnings` 中，目标不降级。
- `Degraded { reason, mismatches }`：
  - `SOURCE_SCHEMA_DRIFT`：有 error 级不匹配（表或列消失、类型变得不兼容等），`mismatches` 是全部不匹配。
  - `SOURCE_UNREADABLE`：这个目标的元数据查询失败（例如权限被收回），`error` 是引擎错误的文本（不含凭据）；其余目标照常判定。

`check_drift` 只在整轮引擎错误（不可用、认证失败等）时返回 `Err`，不会因此产生「全部降级」或「全部健康」的报告：引擎健康与 schema drift 是两件事。drift 与 `validate --online` 共用同一个 `compare_binding`。

## 私有 fixture

公司环境或其他不便公开的数据，只经环境变量 `ONTOFORGE_PRIVATE_FIXTURES` 指向仓库外的目录来验证；仓库中不放任何私有绑定或数据，CI 不运行这些测试。目录约定见 `CONTRIBUTING.md`。
