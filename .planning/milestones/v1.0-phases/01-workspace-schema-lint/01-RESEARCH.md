# Phase 1: Workspace, Schema & Lint - Research

**Researched:** 2026-10-15
**Domain:** Rust cargo workspace、ontology-as-code（YAML/JSON → 单一 IR）、带 span 的诊断、静态 lint、JSON Schema 导出
**Confidence:** HIGH（spike 全部实跑并留有证据；lint 启发式阈值为 MEDIUM）

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

本 phase 的灰色地带都是技术实现问题。按用户的「决策自主」偏好（自行调研决定，必须由用户拍板的才问），全部由 Claude 决定，依据写在每条后面。

#### 定义文件布局
- **D-01：** 一个 ontology 对应一个目录，根目录放 manifest `ontology.yaml`。manifest 包含 `apiName`、`displayName`、`description`，以及 `include` glob 列表（默认 `**/*.yaml`、`**/*.yml`、`**/*.json`）。其余文件的位置和数量不做限制。理由：开发者可以按领域自行拆分文件，loader 只看 glob 结果，不依赖目录约定。
- **D-02：** 每个定义文档都是带 `kind` 判别字段的对象，`kind` 取值为 `ObjectType` / `LinkType` / `ActionType`（预留 `Interface`、`ValueType`、`SharedProperty`，loader 遇到时报「v1 不支持」）。一个 YAML 文件可以用 `---` 写多个文档；JSON 文件可以是单个对象，也可以是对象数组。理由：`kind` 判别让 serde internally-tagged enum 和 JSON Schema 的 `oneOf` 都能直接表达，LLM 生成时也不容易混淆。 — **Reversibility:** one-way — 这是对外的 ontology-as-code 契约，发布后改字段名或结构，所有用户的定义文件都要迁移。
- **D-03：** 属性内联在 object type 里（`properties:` 下以 apiName 为 key 的 map）；link type 单独成文档，两端都写在 link type 里。理由：与 Foundry 的模型一致（link type 是独立资源）；1:N 外键属性仍在 object type 里声明，link type 只引用它。

#### 命名与校验规则（遵循 Foundry 文档 + ADR-0002）
- **D-04：**
  - object type 的 apiName 用 PascalCase：`^[A-Z][A-Za-z0-9]{0,99}$`，在 ontology 内唯一。
  - property 的 apiName 用 camelCase：`^[a-z][A-Za-z0-9]{0,99}$`，在 object type 内唯一。
  - link 两端的 apiName 用 camelCase，在所关联的 object type 范围内唯一，并且不能与该类型的属性同名。
  - action type 的 apiName 用 camelCase，在 ontology 内唯一；MCP 工具名在 Phase 7 由它派生为 kebab-case。
  - 依据：`docs/foundry/ontology/object-link-types/create-object-type.md`、`create-link-type.md`。保留关键字表由 research 补齐。
- **D-05：** 未知字段一律报错（serde `deny_unknown_fields`，诊断里附「你是否想写 X」建议）。理由：拼写错误静默忽略是 ontology-as-code 最常见的坑；以后新增字段属于向后兼容的扩展。
- **D-06：** 结构性错误属于 validate 范畴（hard error，不能关闭），例如解析失败、未知 base type、重复 apiName、引用不存在的类型或属性、主键类型非法、外键类型与目标主键不匹配。lint 的 ONT-* 规则在 IR 构建成功之后运行，可以配置。理由：IR 构建失败时 lint 无法运行，两层必须分开。

#### Action type 定义形状
- **D-07：** 参数为 `parameters:`，以 apiName 为 key 的 map，字段有 `type`、`required`、`default`、`description`。type 取值为 base type，或 `objectReference: <ObjectType>`。
- **D-08：** rules 是有序列表，每条为 `{ kind: createObject | modifyObject | deleteObject | addLink | removeLink, ... }`。值来源用结构化写法，不用字符串模板：`{ param: x }` / `{ static: v }` / `{ currentUser: {} }` / `{ currentTime: {} }`。理由：结构化写法可以静态校验类型；字符串模板要再引入一种语法。 — **Reversibility:** one-way — 同属 ontology-as-code 契约。
- **D-09：** submission criteria 为列表，每条 `{ condition: <CEL 字符串>, failureMessage: <文本> }`。CEL 上下文变量固定为 `params`、`user`（`id`、`groups`）、`objects`（按参数名访问 objectReference 参数指向的对象当前属性）。本 phase 只做 CEL 编译、变量引用存在性检查、参数类型推断（取决于 spike 结论），不求值。

#### 诊断与输出
- **D-10：** 诊断模型统一为 `{ code, severity, message, file, line, column, span, help }`。人类可读输出用 `miette` 的 fancy 渲染；`--format json` 输出该模型的 JSON 数组。validate 和 lint 共用同一模型。退出码：有 error 时为 1，否则为 0；`--deny-warnings` 让 warning 也返回 1。
- **D-11：** span 精度至少到产生问题的那个 key 或 value 节点。如果 spike 证明 `serde-saphyr` 无法按 IR 路径取 span，就退化为先解析成带 span 的 YAML 节点树，再反序列化为 IR，同时维护「IR 路径 → span」表。具体方案由 research / spike 决定，但「到节点级」这个精度要求不变。JSON 输入同样要求 line:col。

#### Lint 规则范围与配置
- **D-12：** v1 schema 范围内适用的 E 级规则为 ONT-E08（action rule 组合约束）和 ONT-E09（base type 用途约束：主键 / title key / array 元素）。其余 E 级规则（派生属性、vector、scenario、value type、interface、security policy）因对应 schema 元素不在 v1，本 phase 不实现，规则 ID 保留。
- **D-13：** W01–W11 都实现，但只做静态部分。ONT-W06（God Object）只检查属性数量阈值和 `*Type` / `*Subtype` 判别列，空值率依赖数据，留到有存储之后。ONT-W07 的 Jaccard 比较只基于属性名和类型集合。
- **D-14：** lint 配置写在 ontology 根目录的 `ontoforge.toml`，`[lint]` 段中按规则 ID 设置 `"off" | "info" | "warn" | "error"`。CLI 的 `--config` 可以覆盖配置文件路径。理由：Rust 生态习惯用 TOML 做工具配置，也和 ontology 定义本身（YAML）区分开。
- **D-15：** 每条 lint 规则一个模块，每条规则配一组 fixture 测试（触发和不触发两种），用 `insta` 做快照。

#### 示例 ontology
- **D-16：** 示例 ontology `examples/ontologies/aviation/` 沿用 Foundry 文档中的航空领域，包含 Airport、Aircraft、Flight、Pilot：
  - 1:N：Flight 的 departureAirport 外键；
  - 1:1：Aircraft 与当前维护记录，或类似关系；
  - M:N：Flight 与 Pilot 的机组分配；
  - 2–3 个 action type：assignPilot、delayFlight 等。
  - 同时提供一份等价的 JSON 版本，用于「YAML 与 JSON 得到同一 IR」的测试。
  - 另建 `examples/ontologies/invalid/` 存放刻意出错的样例，作为诊断测试的 fixture。
  - 理由：领域与 Foundry 文档一致，可以直接对照文档语义；三种基数都能覆盖。

#### Workspace
- **D-17：** 10 个 crate 按 ADR-0011 一次性建好。共享依赖版本放在根 `Cargo.toml` 的 `[workspace.dependencies]`，lint 配置放在 `[workspace.lints]`（`unsafe_code = "forbid"`，clippy pedantic 选择性开启）。新增 `.gitattributes` 设置 `* text=auto eol=lf`，避免 Windows 上 CRLF 造成快照 diff。
- **D-18：** 本 phase 加 CI（GitHub Actions）：Windows 和 Ubuntu 两个平台跑 `cargo fmt --check`、`cargo clippy -D warnings`、`cargo test --workspace`。理由：DX-03 的回归门槛需要自动化守住，否则后续 phase 容易破坏 Windows 构建。

### Claude's Discretion
- crate 内部模块划分、IR 类型的具体 Rust 表示（Arc / interning 等）、错误枚举设计。
- `ontoforge` CLI 子命令的具体参数命名（遵循 clap 惯例）。
- JSON Schema 的导出方式：`ontoforge schema export` 子命令，并把生成产物入库做快照。

### Deferred Ideas (OUT OF SCOPE)
- ONT-W06 基于空值率的检测：需要存储层数据，Phase 2 之后再加。
- 编辑器集成（LSP / VS Code 插件）：JSON Schema 已覆盖基本补全，LSP 属于新能力，记入 backlog。
- `ontoforge init` 脚手架命令：交给 DX 相关 phase 评估。
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SCHEMA-01 | 多文件 YAML/JSON 合并加载为单一 IR | Spike S1：`Spanned` 节点树对 YAML 多文档、JSON 单对象/数组统一解析；loader 流水线、确定性排序、IR 与 span 分离（见 Architecture Patterns 1、2、4） |
| SCHEMA-02 | object type：apiName、primary key、title key、displayName、description、aliases、hidden | 推荐定义形状（ObjectType 示例）；apiName 正则与保留字（Domain Facts 2） |
| SCHEMA-03 | 属性 base type 与 required | v1 base type 表、主键/title key 适用性、参数类型映射（Domain Facts 1） |
| SCHEMA-04 | link type 1:1、1:N（外键）、M:N（join），两端各有 apiName | 推荐 LinkType 形状；Foundry link 语义；V11 校验目录（Domain Facts 3、Validate 目录） |
| SCHEMA-05 | action type：参数、rules、submission criteria（CEL） | rule 语义（Domain Facts 4）；Spike S2（CEL 仅能做语法与引用检查，类型检查需自建迷你推断） |
| SCHEMA-06 | 带文件名与 line:col 的诊断 | Spike S1：节点级 span、serde_path_to_error 路径回查、miette 渲染实测 |
| SCHEMA-07 | 导出 JSON Schema | Spike S3：schemars 1.2.3 输出 2020-12 `oneOf` + `const kind`；`#[schemars(tag)]` 覆盖技巧 |
| LINT-01 | E 级规则，有 E 级问题退出码非 0 | E08/E09 到 v1 IR 的静态检查映射（Lint 规则映射） |
| LINT-02 | W01–W11 | 每条规则的输入、算法、默认阈值与 fixture（Lint 规则映射） |
| LINT-03 | 人类可读与 JSON 输出 | 诊断模型、miette 手动 handler、JSON 无 ANSI（Pattern 5、Pitfalls） |
| LINT-04 | 按规则 ID 关闭或调整严重级 | `ontoforge.toml` 解析实测（toml 1.1.8 span 可用）；未知规则 ID 报错 |
| DX-03 | Windows 本机 `cargo test` 不依赖 Docker 全绿 | workspace 骨架实测通过；Windows 栈溢出两处陷阱与缓解（Pitfalls 1） |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

来源：`D:/DataSpellProjects/ontoforge/.claude/CLAUDE.md`（项目级）与 `PROJECT.md` Constraints。

- Rust 1.98、edition 2024、cargo workspace（ADR-0011）；技术栈已由 ADR-0003～0011 锁定。
- Windows 11 上 `cargo build` / `cargo test` 必须通过，不依赖 Docker；生产目标 Linux。
- 命名：项目与 crate 前缀 `ontoforge`；标识符遵循 ADR-0002，禁止沿用参考仓库的任何标识符。不复制参考仓库代码。
- 直接修改仓库文件前必须经由 GSD 命令入口（`/gsd-execute-phase` 等）；本研究只产出 RESEARCH.md。
- 文档与规划用中文，标识符与代码用英文，无 emoji。
- 用户全局规则：主 session 不直接写实现代码（执行阶段由 subagent 完成）；核心逻辑（loader、校验、lint 规则）适用 TDD；子任务用 ralph-loop 交付。

## Summary

本 phase 的核心风险是 D-11：能否做到节点级 span。三个强制 spike 全部实跑完成，结论如下。

(1) `serde-saphyr` 2.0.0 的 `Spanned<T>` 能给出带 `line/column/byte_offset/byte_len` 的 span，但只在普通 struct 字段与自定义 `Deserialize` 中有效；在 `#[serde(tag = "kind")]`、`untagged`、`flatten` 内位置丢失为 0:0（实测与源码注释一致）。`deny_unknown_fields` 的错误 `location()` 为 `None`，unknown variant 的位置只落在文档起点。所以「直接反序列化进 IR 并用 `Spanned` 字段」不可行。可行方案是：用 `serde-saphyr` 自身把每个文档解析成一棵自建的 `Node` 树（每个值与每个 key 都是 `Spanned`），转为 `serde_json::Value`，再用 `serde_path_to_error` 反序列化为类型化定义，失败时按路径回查 `Node` 树得到 key 或 value 的 span。YAML 多文档、JSON 单对象/数组、tab 缩进、CRLF、BOM、中文字符、重复 key 均实测通过，且 miette 渲染出正确的行列与下划线。该方案不需要额外的 YAML 解析 crate，保持 ADR-0003 的 `serde-saphyr` 选型；`saphyr` 0.2.2 的 `MarkedYaml` 实测也能给 key/value span，作为 Plan B。

(2) `cel` 0.15.0 只能编译（不求值），`references()` 会把 comprehension 迭代变量（如 `all(i, ...)` 里的 `i`）误当作自由变量，且没有任何静态类型检查或变量声明机制。所以需要自写约 150 行的 AST 遍历（AST 类型 `IdedExpr/Expr` 是 pub）来做作用域感知的引用收集，再叠加一个保守的迷你类型推断。一个重要陷阱：`cel` 的 antlr 解析器在 Windows 1 MiB 主线程栈的 debug 构建下，括号嵌套约 10 层即栈溢出（进程直接 abort，不可 catch）。必须在专用大栈线程里编译，并预先限制长度与嵌套深度。

(3) `schemars` 1.2.3 默认输出 JSON Schema 2020-12。`#[serde(tag = "kind")]` 产出 `oneOf`，每个分支含 `properties.kind.const` 与 `additionalProperties: false`。`#[schemars(tag = "kind")]` 可以只覆盖 schema 侧而让 serde 侧保持 externally tagged，这是本研究推荐的关键技巧：loader 把 `{kind: X, ...}` 重写为 `{X: {...}}` 后用 externally tagged 反序列化，`serde_path_to_error` 在 externally tagged 下能保留完整深层路径（实测 `rules[0].createObject.properties.a.param`），而 internally tagged 下路径丢失。`insta` 1.49.0 在 Windows 上对快照文件 CRLF 与实际值 CRLF 均自动归一化，`.gitattributes` 主要保护 fixture 的字节偏移稳定。

**Primary recommendation:** 采用「`serde-saphyr` Spanned 节点树 → `serde_json::Value`（kind 重写为 externally tagged）→ `serde_path_to_error` 类型化反序列化 → 路径回查 span」的单一流水线处理 YAML 与 JSON；IR 直接使用带 `serde + schemars` derive 的定义类型且不含 span，span 放在并行的 `SourceMap`；CEL 检查放在 `ontoforge-expr`（它依赖 schema，所以由 CLI 层编排），编译在 32 MiB 栈的线程里执行并先做长度与嵌套深度限制。

## Architectural Responsibility Map

本项目 phase 1 是纯库 + CLI，没有浏览器 / SSR / CDN 层。这里把「tier」解释为 crate 边界（ADR-0011 的依赖方向 `schema ← expr ← … ← cli`）。

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| YAML/JSON 解析、Node 树、SourceMap | `ontoforge-schema` | — | ADR-0011 明确 loader 在 schema crate；span 不进 IR |
| 定义类型 = IR（serde + schemars derive） | `ontoforge-schema` | — | 「IR 是唯一契约」；后续 phase 只读这套类型 |
| Validate（ONT-V 结构性校验、引用完整性） | `ontoforge-schema` | — | 与 IR 构建同层；hard error 不可关闭 |
| Lint 规则（E08、E09、W01–W11）与 `ontoforge.toml` 解析 | `ontoforge-schema` | — | ADR-0011：lint 在 schema crate；每规则一个模块 |
| CEL 编译、引用与类型检查 | `ontoforge-expr` | `ontoforge-schema`（提供 IR 与 Diagnostic 类型） | expr 依赖 schema，schema 不能反向依赖，故检查入口在 expr |
| 编排 validate → expr 检查 → lint、退出码、渲染 | `ontoforge`（CLI） | — | CLI 同时依赖 schema 与 expr，是唯一能串起两者的位置 |
| 人类可读渲染（miette fancy）与 JSON 输出 | `ontoforge`（CLI） | — | 渲染依赖不泄漏到库 crate；库只返回数据化 `Diagnostic` |
| JSON Schema 生成 | `ontoforge-schema` | `ontoforge`（`schema export` 子命令） | schemars derive 在定义类型上，CLI 只负责落盘 |
| 其余 7 个 crate（query、storage、action、auth、api、server、client） | 各自骨架 | — | 本 phase 只需能编译并有占位测试 |

## Spike Results (mandatory)

所有 spike 代码在 `C:\tmp\ontoforge-spikes\`（仓库之外），下载的 crate 源码按不可信数据处理，只读不执行其脚本。工具链：rustc 1.98.0（2026-08-18）、cargo 1.98.0、clippy 0.1.98、rustfmt 1.9.0 `[VERIFIED: rustc --version / cargo --version，本会话]`。

### S1：serde-saphyr 2.0.0 的 span 能力（结论：不能直接按 IR 路径取 span，但「Spanned 节点树」方案可行）

| 问题 | 结果 | 证据 |
|------|------|------|
| (a) 特定 key/value 节点的 `Spanned<T>` | 可行：`serde_saphyr::Spanned<T>{ value, referenced, defined }`，`Location` 含 `line()`、`column()`（均 1 起）、`span().byte_offset()/byte_len()`、字符 `offset()/len()` | 实测：`apiName: Flight` 值位于 2:10，byte_len 6；中文值 `航班` char len 2、byte len 6 |
| 在 `#[serde(tag="kind")]` 内的 `Spanned<T>` | 位置丢失：`apiName loc = 0:0` | 实测 + 源码 `src/spanned.rs` 文档（flatten / untagged / tag 均丢失） |
| (b) 多文档 `---` | 可行：`from_multiple::<Node>(text)` 返回 `Vec<Node>`；空文档被跳过（`---\n---\nkind: A` 得 1 个） | 实测 |
| (c) `deny_unknown_fields` 错误位置 | `e.location()` 为 `None`；消息 `unknown field \`primaryKy\`, expected one of ...` | 实测 |
| (c) 未知 enum variant（`kind`） | 有位置但落在文档起点 `1:1`，不是 `kind` 值 | 实测 |
| JSON 输入 | 同一 API 解析 JSON（YAML 1.2 超集）：对象、数组、tab 缩进、`\t` 均通过；key span 含引号（`"apiName"` len 9）；缺陷：尾逗号 `{...,}` 与注释被当作合法 YAML 接受 | 实测，需用 `serde_json` 对 `.json` 做严格语法预检 |
| BOM / CRLF / 非 ASCII | BOM 不计入列号但计入 byte offset（`bytes=Some(3)`）；CRLF 行列正确；`&text[byte_offset..byte_offset+byte_len]` 可精确切出值 | 实测 |
| 重复 key | `DuplicateKeyPolicy::Error` 报错带位置（2:1） | 实测 |
| 严格化选项 | `strict_booleans = true` 后 `yes/NO/on` 保持字符串；`merge_keys = MergeKeyPolicy::Error` 拒绝 `<<`；默认 `strict_booleans=false` 会把 YAML 1.1 布尔字面量当 bool | 实测；源码 `src/de/options.rs:296-300` |
| 预算 | 默认 `Budget.max_depth = 64`、`max_documents = 1024`、`max_nodes = 250_000`、`max_events = 1_000_000` | 源码 `src/de/budget.rs:275-300` |

**采用方案（单一方案，YAML 与 JSON 共用，单一 IR）：**

```rust
// Source: spike C:\tmp\ontoforge-spikes\s1-yaml\examples\pipeline.rs（实跑通过）
#[derive(Debug)] pub struct Node { pub loc: serde_saphyr::Location, pub kind: Kind }
#[derive(Debug)] pub enum Kind {
    Null, Bool(bool), Int(i64), Float(f64), Str(String),
    Seq(Vec<Node>), Map(Vec<(serde_saphyr::Spanned<String>, Node)>),
}
impl<'de> Deserialize<'de> for Node {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = Spanned::<Kind>::deserialize(d)?;   // 外层 Spanned 取节点自身 span
        Ok(Node { loc: s.referenced, kind: s.value })
    }
}
// Kind::deserialize 用 deserialize_any + Visitor：visit_seq 收集 next_element::<Node>()，
// visit_map 收集 (next_key::<Spanned<String>>(), next_value::<Node>())。
// 解析：serde_saphyr::from_multiple_with_options::<Node>(text, opts) 得到 Vec<Node>
// opts: strict_booleans = true; duplicate_keys = Error; merge_keys = Error; reject_unsupported_tags = true;
//       with_snippet = false; budget.max_depth = 32
```

后续步骤（均已在 pipeline 示例中实跑）：`Node::to_value()` → `serde_json::Value`；从 map 里取出 `kind` 字符串自行分派（缺失或未知 kind 直接产出带 `kind` 值 span 的诊断）；用 `serde_path_to_error::deserialize::<_, T>(value)` 反序列化；失败时按 `e.path()` 的 `Segment::{Map, Seq, Enum}` 逐段下探 `Node` 树：命中 map key 时记下 key 的 span，最终取最后一个 key 的 span（unknown field 指向写错的 key，类型错误指向该 key 或 seq 元素值）。miette 渲染示例（实测输出）：

```
  × unknown field `primaryKy`, expected `apiName` or `props`
   ╭─[a.yaml:3:1]
 2 │ apiName: Flight
 3 │ primaryKy: id
   · ────┬────
   ·     ╰── here
```

**Plan B（已验证可用，未采用）：** `saphyr` 0.2.2 `MarkedYaml::load_from_str` 返回带 `span.start.line()/col()` 的节点（列号 0 起，行号 1 起），key 与 value 各有 span；对应实测输出 `key "primaryKy" @3:0  val @3:11`。把 `Node` 构造隔离在 `loader::parse` 一个模块内，即可一文件替换解析器。

**风险说明：** `serde-saphyr` 2.0.0 发布于 2026-10-13（距研究日 2 天），major 版本刚升 `[VERIFIED: crates.io API]`；它基于 `granit-parser`（saphyr-parser 的 fork）`[VERIFIED: serde-saphyr-2.0.0/README.md、Cargo.toml features 含 dep:granit-parser]`。Cargo.toml 写 `serde-saphyr = "2.0"` 并提交 `Cargo.lock`，解析层封装在单模块内以便切到 Plan B。

**ADR-0003 回写文本（执行阶段由 plan 中的 task 落盘）：**

> Spike 结论（2026-10-15）：`serde-saphyr` 2.0.0 的 `Spanned<T>` 在 `#[serde(tag)]` / `untagged` / `flatten` 内丢失 span，`deny_unknown_fields` 错误无位置，故不能「直接反序列化进 IR 并按路径取 span」。决定：用 `Spanned` 构建节点树（YAML 与 JSON 同一路径）→ `serde_json::Value` → `serde_path_to_error` → 路径回查 span。IR 不含 span，span 存于并行 `SourceMap`。Plan B：`saphyr` 0.2.2 `MarkedYaml`（已验证 key/value span）。`.json` 文件额外用 `serde_json` 做严格语法预检（YAML 解析器会接受尾逗号与注释）。

### S2：`cel` 0.15.0（结论：可编译不求值、可列引用，无静态类型检查，需自建）

| 问题 | 结果 | 证据 |
|------|------|------|
| 只编译不求值 | 可：`cel::Env::stdlib().compile(src) -> Result<Program, ParseErrors>`；`Program::compile(src)` 等价 | 实测 `Program::compile ok: true` |
| 枚举引用变量 | `program.references().variables()` 返回自由变量名，但把 comprehension 迭代变量也算进去：`params.items.all(i, i.qty > 0 ...)` 得 `vars=["i","params"]` | 实测 |
| 枚举 `params.x` / `objects.p.prop` 成员路径 | `Program::expression()` 返回 `&IdedExpr`，`Expr::{Call, Select, Ident, Comprehension, List, Map, Struct, Literal}` 全部 pub；`SelectExpr{operand, field, test}`，`has(a.b)` 展开为 `test: true` 的 Select。自写遍历可得精确路径集合，并正确处理 `Comprehension` 作用域（`iter_var`、`iter_var2`、`accu_var`） | 实测：`paths=["params.items"]`（`i.qty` 被正确排除）、`has(params.note)` 产出 `params.note` |
| 静态类型检查 | 无。`Env` 只有函数/宏/struct 类型注册（`add_overload`、`add_type`、`StructDef`），没有变量类型声明，也没有 checker；未知变量/未知函数/不存在成员都能 compile 成功 | 实测 `unknownVar.foo > 1`、`nope(1)` 均 compile 成功；源码 `src/env.rs` pub fn 清单无 check/declare variable |
| 错误位置 | `ParseErrors.errors[i].pos = (line, col)`（1 起，针对表达式文本）；节点偏移：`program.source_info().offset_for(node.id) -> Option<(start, stop)>`，二元表达式指向运算符位置，ident 指向起点 | 实测：`params.x +` 得 `pos=(1, 11)` |
| 栈安全 | 严重陷阱：debug 构建 + Windows 1 MiB 主线程栈，`((((((((((1))))))))))` 括号嵌套 ≥ 10 层即 `thread 'main' has overflowed its stack`（进程 abort，不可 catch）；5 层通过。test 线程（2 MiB）10 层通过；32 MiB 线程 60 层、64 MiB 线程 90 层通过。release 下 ≥ 100 层返回 `Err(Syntax error ...)`（内置递归上限），50/90 层通过 | 实测（deep2 / deep3 示例） |

**API 片段（实测通过）：**

```rust
// Source: spike C:\tmp\ontoforge-spikes\s2-cel\src\main.rs
use cel::{Env, common::ast::{Expr, IdedExpr, EntryExpr}};
let env = Env::stdlib();
let program = env.compile("params.reason != '' && user.id == params.owner")?;   // 不求值
let root: &IdedExpr = program.expression();
// 自写 walk(root, &mut scope, &mut out)：Select 链还原 qualified name，
// Comprehension 先 walk iter_range，再把 iter_var/accu_var 压入 scope 后 walk loop_cond/loop_step/result
let pos = program.source_info().offset_for(root.id);   // Some((start, stop))
```

**结论落入 ADR-0006 回写文本：**

> Spike 结论（2026-10-15）：`cel` 0.15.0 无声明式变量类型检查。静态检查范围定为：语法、自由变量根必须 ∈ {`params`, `user`, `objects`}、`params.<name>` 必须是已声明参数、`objects.<name>.<prop>` 必须是 objectReference 参数且属性存在于目标 object type、`user.<field>` 必须 ∈ {`id`, `groups`}、顶层结果类型在可推断时必须为 bool、比较两侧在可推断时类型兼容；类型推断为 `ontoforge-expr` 自建的保守推断（未知即 Any，不报错）。`references()` 对 comprehension 变量有误报，改用自写 AST 遍历。解析必须在大栈线程内执行并预限长度与嵌套深度（debug + Windows 主线程括号嵌套 ≥ 10 层栈溢出）。

### S3：miette / schemars / insta

| 问题 | 结果 | 证据 |
|------|------|------|
| 版本 | miette 7.6.0（2025-04-27 发布，仍是 crates.io 最新稳定版）、schemars 1.2.3（2026-10-12）、insta 1.49.0（2026-10-03）、jsonschema 0.58.6 | `[VERIFIED: crates.io API，2026-10-15]` |
| `#[serde(tag="kind")]` 的 JSON Schema | `"Document": { "oneOf": [ { "type":"object", "properties": { "kind": {"type":"string","const":"ObjectType"}, ... }, "additionalProperties": false, "required": ["kind", ...] }, ... ] }`；默认 `$schema` 为 draft 2020-12；`rename_all`、`deny_unknown_fields`、`#[schemars(regex(pattern=...))]`、`default` 均被体现 | 实测快照 |
| 「discriminator」 | JSON Schema 没有 `discriminator` 关键字（那是 OpenAPI 扩展）；编辑器使用的是 `oneOf` + `const` 约定。yaml-language-server 支持 draft-04/07/2019-09/2020-12。分支收窄补全的效果因版本而异，需在实际编辑器验证；若不理想可在导出时后处理为 `allOf` + `if/then` | `[CITED: github.com/redhat-developer/yaml-language-server README]`；补全收窄为 `[ASSUMED]` |
| 只覆盖 schema 侧的 tag | `#[serde(rename_all="camelCase")] #[schemars(tag = "kind")] enum Rule {...}`：schema 呈 internally tagged（`kind` const），serde 仍是 externally tagged | 实测 |
| `serde_path_to_error` 路径 | externally tagged 下完整保留：`rules[0].createObject.properties.a.param`；internally tagged（经 Content 缓冲）下路径丢失（`.`） | 实测 |
| schema 校验实例 | `jsonschema::validator_for(&schema)`：合法实例、JSON 数组形式、未知 kind、多余字段、apiName 正则违规均按预期通过/失败 | 实测 4 个测试通过 |
| insta + CRLF | 快照文件整体转 CRLF 后测试仍通过；实际值含 `\r\n` 对比 LF 快照也通过（insta 双向归一化行尾） | 实测（`INSTA_UPDATE=no`） |
| `cargo-insta` | 本机未安装（`cargo insta` → no such command）；不影响测试，更新快照用 `INSTA_UPDATE=always cargo test` | 实测 |
| `jsonschema` 默认 feature | 默认拉 `reqwest/rustls/aws-lc-rs`（`resolve-http`、`tls-aws-lc-rs`），Windows 上增加 C 构建面；`default-features = false` 实测可用且依赖树无网络栈 | 实测；`Cargo.toml` `[features] default` 行 |
| `toml` 1.1.8 | `[lint]` 下带引号 key `"ONT-W01" = "off"` 解析为 `IndexMap<String, Level>` 正常；`toml::de::Error::span()` 返回 `Option<Range<usize>>`，如 `"silent"` 的 19..27；`deny_unknown_fields` 对错误 section 名 `[lnt]` 报 span 1..4 | 实测（s5-toml） |

### S4：workspace 骨架（补充 spike，验证 D-17）

用 `resolver = "3"`、`[workspace.package]`（edition 2024、rust-version 1.98）、`[workspace.dependencies]`、`[workspace.lints.rust] unsafe_code = "forbid"`、`[workspace.lints.clippy] pedantic = { level = "warn", priority = -1 }` 加 3 个 crate，`members = ["crates/*"]`：`cargo test --workspace` 通过（含 `env!("CARGO_BIN_EXE_ontoforge")` 的 CLI 集成测试，无需 `assert_cmd`），`cargo clippy --workspace --all-targets -- -D warnings` 通过，`cargo fmt --check` 只对未格式化的占位代码报 diff。`git config core.autocrlf` 在本机为 `true`，所以 `.gitattributes` 的 `eol=lf` 是必需的 `[VERIFIED: git config --get core.autocrlf，本会话]`。

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| serde | 1.0.229 | derive 序列化 | 事实标准 `[VERIFIED: crates.io API]` |
| serde_json | 1.0.154，启用 `preserve_order` | 中间 `Value`（保持 map 声明顺序）、JSON 严格语法预检 | 事实标准；`preserve_order` 保证 `properties` 顺序，否则按字母序 `[VERIFIED: crates.io API]`；preserve_order 行为为 `[ASSUMED]`（未单测，需 Wave 0 加测试） |
| serde-saphyr | 2.0.0（`"2.0"`） | YAML/JSON → `Spanned` 节点树 | ADR-0003 锁定；spike 证实 `Spanned` 可用 `[VERIFIED: crates.io + spike S1]` |
| serde_path_to_error | 0.1.20 | 反序列化错误路径 | dtolnay 维护；对 externally tagged 保留完整路径 `[VERIFIED: docs.rs + crates.io + spike S1/S3]` |
| schemars | 1.2.3（features `derive`、`indexmap2`） | JSON Schema 导出 | ADR-0003 锁定 `[VERIFIED: crates.io + spike S3]` |
| indexmap | 2.14.2（feature `serde`） | 保持声明顺序的 map | `[VERIFIED: crates.io API]` |
| thiserror | 2.0.21 | 库 crate 错误枚举 | rust-stack.md 推荐 `[VERIFIED: crates.io API]` |
| cel | 0.15.0 | CEL 编译与 AST（仅 `ontoforge-expr`） | ADR-0006 锁定；不是 `cel-interpreter` `[VERIFIED: crates.io + spike S2]` |
| clap | 4.6.7（feature `derive`） | CLI（仅 `ontoforge`） | rust-stack.md 推荐 `[VERIFIED: crates.io API]` |
| miette | 7.6.0（`ontoforge` CLI 用 `fancy`） | 人类可读诊断渲染 | ADR-0003 锁定 `[VERIFIED: crates.io + spike S1]` |
| toml | 1.1.8 | `ontoforge.toml` lint 配置，带 span 的配置错误 | D-14；`[VERIFIED: docs.rs + crates.io + spike s5]` |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| strsim | 0.11.1 | 「你是否想写 X」建议（`jaro_winkler` / `levenshtein`） | unknown field、unknown kind、未知规则 ID `[VERIFIED: docs.rs + crates.io]` |
| globset | 0.4.20 | `include` glob 匹配 | manifest 的 include 展开 `[VERIFIED: docs.rs + crates.io]` |
| walkdir | 2.5.0 | 目录遍历（`follow_links` 默认关闭） | 加载 ontology 目录 `[VERIFIED: docs.rs + crates.io]` |
| camino | 1.2.6 | UTF-8 路径，诊断里统一 `/` 分隔 | 文件字段、SourceMap `[VERIFIED: docs.rs + crates.io]` |
| regex | 1.13.1 | apiName 正则、W03/W05 模式（线性时间） | 校验与 lint `[VERIFIED: docs.rs + crates.io]` |
| chrono | 0.4.45（`default-features = false`、`std`） | 校验 `static`/`default` 的 date / timestamp 字面量 | 已经是 `cel` 的传递依赖；仅用于解析 `[VERIFIED: crates.io API + gsd package-legitimacy OK]` |
| insta | 1.49.0（features `json`） | 快照测试 | D-15 `[VERIFIED: crates.io + spike S3]` |
| jsonschema | 0.58.6，`default-features = false`（dev-dependency） | 测试里校验示例 ontology 满足导出的 schema | `[VERIFIED: crates.io + spike S3]` |
| tempfile | 3.27.0（dev） | CLI 测试里造临时目录 | `[VERIFIED: crates.io API + gsd package-legitimacy OK]` |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `serde-saphyr` Spanned 节点树 | `saphyr` 0.2.2 `MarkedYaml` | Plan B，实测可用，但新增依赖且需自写 scalar 类型解析（bool/int/float）；serde-saphyr 已含 YAML 1.2 scalar 规则 |
| `json-spanned-value` 0.2.2 | — | 最后发布 2020-10-10，停更，不采用 |
| 自写 AST 遍历 | `cel` 的 `references()` | 后者误报 comprehension 变量，且无法给出 `params.x` 成员路径 |
| 手写 `Node → T` 转换（无 serde） | `Value` + `serde_path_to_error` | 手写诊断更自由，但与 schemars 导出的类型会产生双份模型；推荐方案只维护一份类型 |
| `assert_cmd` + `predicates` | `std::process::Command` + `env!("CARGO_BIN_EXE_ontoforge")` | 后者零依赖，S4 实测可用；JSON 输出断言用 `serde_json` 即可 |

**Installation（根 `Cargo.toml` 的 `[workspace.dependencies]` 只列本 phase 实际用到的依赖，其余 phase 再加）：**

```toml
[workspace.dependencies]
serde = { version = "1.0.229", features = ["derive"] }
serde_json = { version = "1.0.154", features = ["preserve_order"] }
serde-saphyr = "2.0"
serde_path_to_error = "0.1.20"
schemars = { version = "1.2.3", features = ["derive", "indexmap2"] }
indexmap = { version = "2.14.2", features = ["serde"] }
thiserror = "2.0.21"
cel = "0.15.0"
clap = { version = "4.6.7", features = ["derive"] }
miette = { version = "7.6.0", features = ["fancy"] }
toml = "1.1.8"
strsim = "0.11.1"
globset = "0.4.20"
walkdir = "2.5.0"
camino = "1.2.6"
regex = "1.13.1"
chrono = { version = "0.4.45", default-features = false, features = ["std"] }
insta = { version = "1.49.0", features = ["json"] }
jsonschema = { version = "0.58.6", default-features = false }
tempfile = "3.27.0"
```

**Version verification：** 上表版本均于 2026-10-15 通过 crates.io API 查得（`max_stable_version`），`gsd-tools query package-legitimacy check --ecosystem crates` 对全部包返回 `OK`（含 weeklyDownloads 与 repoUrl）。

## Package Legitimacy Audit

| Package | Registry | Age | Downloads (weekly) | Source Repo | Verdict | Disposition |
|---------|----------|-----|--------------------|-------------|---------|-------------|
| serde-saphyr | crates.io | 首版 2025-09-27 | 604,706 | github.com/bourumir-wyngs/serde-saphyr | OK | Approved（ADR-0003 锁定；2.0.0 刚发布，见风险说明） |
| saphyr | crates.io | 2024-04 | 71,910 | github.com/saphyr-rs/saphyr | OK | 仅作 Plan B，不安装 |
| cel | crates.io | 2020-01 | 287,406 | github.com/cel-rust/cel-rust | OK | Approved（ADR-0006 锁定） |
| miette | crates.io | 2021-08 | 1,621,496 | github.com/zkat/miette | OK | Approved |
| schemars | crates.io | 2019-08 | 15,612,098 | github.com/GREsau/schemars | OK | Approved |
| insta | crates.io | 多年 | 极高 | github.com/mitsuhiko/insta | OK | Approved |
| serde_path_to_error | crates.io | 多年 | 7,560,559 | github.com/dtolnay/path-to-error | OK | Approved（已由 docs.rs 官方文档确认） |
| strsim | crates.io | 多年 | 19,069,546 | github.com/rapidfuzz/strsim-rs | OK | Approved（docs.rs 确认） |
| globset | crates.io | 多年 | 4,665,254 | github.com/BurntSushi/ripgrep（crates/globset） | OK | Approved（docs.rs 确认） |
| walkdir | crates.io | 多年 | 13,079,868 | github.com/BurntSushi/walkdir | OK | Approved（docs.rs 确认） |
| camino | crates.io | 多年 | 5,691,655 | github.com/camino-rs/camino | OK | Approved（docs.rs 确认） |
| toml | crates.io | 多年 | 20,149,026 | github.com/toml-rs/toml | OK | Approved（docs.rs 确认） |
| regex | crates.io | 多年 | 21,840,041 | github.com/rust-lang/regex | OK | Approved（docs.rs 确认） |
| chrono | crates.io | 多年 | 15,627,558 | github.com/chronotope/chrono | OK | Approved（已是 cel 传递依赖） |
| serde / serde_json / clap / thiserror / indexmap / tempfile / jsonschema | crates.io | 多年 | 极高 | 官方仓库 | OK | Approved |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none。无需插入 `checkpoint:human-verify`。

注：`cel` 带入 `antlr4rust`、`chrono-tz`、`regex`、`base64` 等传递依赖；本 phase 不审计传递依赖，仅在 Pitfalls 中提示 antlr4rust 的栈用量。

## Architecture Patterns

### System Architecture Diagram

```
ontoforge validate|lint <dir>                       ontoforge schema export --out-dir
        |                                                     |
        v                                                     v
 [CLI: clap]  --config ontoforge.toml ---------> [lint config parse: toml, spans]
        |                                                     
        v   (worker thread, 16-32 MiB stack)                  [schemars: schema_for!(DefinitionFile) / Manifest]
 [discover]  manifest ontology.yaml -> include globs -> walkdir (no symlinks, skip dotdirs)
        |        sort by normalized relative path (deterministic)
        v
 [read file] UTF-8 check -> .json: serde_json strict syntax precheck
        |
        v
 [parse]  serde-saphyr from_multiple -> Vec<Node>   (YAML multi-doc / JSON object / JSON array)
        |             \---- parse error -> ONT-V01 (file:line:col from serde_saphyr::Error)
        v
 [per document]  Node -> serde_json::Value ; peek `kind` ; rewrap {kind:X,...} -> {X:{...}}
        |             \---- bad/missing kind -> ONT-V02/V03 (span = kind value or doc root)
        v
 serde_path_to_error::deserialize::<Definition>  ----error----> path -> Node tree lookup -> Diagnostic(key/value span)
        |
        v  Vec<(DefRef, Definition)>
 [merge + validate]  duplicates, refs, PK/FK, rule/param typing   -> ONT-V* (hard)
        |   result: Loaded { ontology: Ontology (IR, no spans), sources: SourceMap }
        v
 [ontoforge-expr]  for each criterion: len/depth limit -> compile (big-stack thread) -> AST walk
        |           -> reference check + mini type inference      -> ONT-V13 (hard)
        v
 [lint]  rule registry (E08, E09, W01..W11) + effective level from ontoforge.toml -> findings
        v
 [diagnostics]  sort (file, line, col, code) -> human (miette) | --format json -> exit code 0/1
```

### Recommended Project Structure

```
Cargo.toml                       # [workspace] resolver="3", members=["crates/*"]
rust-toolchain.toml              # channel = "1.98.0", components = clippy, rustfmt (建议)
.cargo/config.toml               # [env] RUST_MIN_STACK = "16777216"（测试线程栈）
.gitattributes                   # * text=auto eol=lf ; *.snap text eol=lf
.github/workflows/ci.yml         # windows-latest + ubuntu-latest
crates/
  ontoforge-schema/
    src/lib.rs
    src/ir/            # 定义类型 = IR（serde + schemars derive）：ontology.rs object.rs link.rs action.rs names.rs basetype.rs
    src/loader/        # discover.rs parse.rs(Node) dispatch.rs(kind 重写) sourcemap.rs
    src/validate/      # mod.rs refs.rs keys.rs links.rs actions.rs （ONT-V*）
    src/diag.rs        # Diagnostic / Severity / Location / 排序
    src/lint/          # mod.rs(registry, config) e08.rs e09.rs w01.rs ... w11.rs
    src/schema_export.rs
    tests/fixtures/    # 内联小 ontology 的辅助函数；快照在 tests/snapshots
  ontoforge-expr/      # src/{check.rs, walk.rs, infer.rs, limits.rs}
  ontoforge/           # src/main.rs(薄) src/lib.rs(commands) src/render.rs ; tests/cli.rs
  ontoforge-query|storage|action|auth|api|server|client/   # 仅 Cargo.toml + lib.rs + 占位测试
examples/ontologies/
  aviation/ (ontology.yaml + *.yaml)   aviation-json/ (等价 JSON)   invalid/<case>/ontology.yaml...
schemas/                         # 导出产物入库：ontoforge-definition.schema.json, ontoforge-manifest.schema.json
```

### Pattern 1：Node 树 + Value + 路径回查（核心）

**What:** 见 Spike S1。**When:** 所有 YAML/JSON 加载。**要点：**
- `Node` 是 loader 私有类型，不进 IR。保留每个文档的 `Node` 树在 `SourceMap` 里，供后续 validate/lint 按「定义 + 指针」回查 span。
- 回查函数 `locate(node, path) -> Location` 对 `Segment::Map{key}`、`Segment::Seq{index}` 下探；对 `Segment::Enum{variant}`：若当前 map 含同名 key 则下探（真实的 `{ param: x }`），否则视为透明（被 kind 重写出来的包装）。
- `unknown field` 时 `serde_path_to_error` 路径末段就是写错的 key，直接得到 key span；`missing field` 时路径停在父节点，落父 map 的起点，建议改为落在 `kind` 值或 apiName 值上（对应 V05）。
- 自定义校验错误（newtype 的 `Deserialize` 里 `de::Error::custom`）同样带路径，用于 apiName 正则、保留字，附 help。

### Pattern 2：`kind` 重写成 externally tagged（解决 internally tagged 丢路径）

```rust
// Source: spike s3-schema/tests/rewrap.rs（schema 侧）+ s1 pipeline（路径侧）
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(tag = "kind")]            // 仅影响 schema：internally tagged + kind const
pub enum Rule { CreateObject(CreateObject), ModifyObject(ModifyObject), DeleteObject(DeleteObject),
                AddLink(LinkEdit), RemoveLink(LinkEdit) }
// loader：对 rules[*] 与文档根执行 rewrap：{kind: "createObject", a: 1} -> {"createObject": {a: 1}}
// 再用 serde（externally tagged）反序列化；每个变体 struct 自带 deny_unknown_fields
```

重写站点只有两类：文档根（`ObjectType` / `LinkType` / `ActionType`）和 `ActionType.rules[*]`。`DefinitionFile` 的 schema 版本另有一个仅用于导出的 `untagged` 包装（单对象或对象数组）。

### Pattern 3：IR 即定义类型，span 在 SourceMap

- IR（`Ontology`、`ObjectType`、`PropertyDef`、`LinkType`、`ActionType` 等）同一套类型既做 serde 反序列化目标、也做 schemars 导出源、也做下游 phase 的唯一契约。校验后的新类型 `ApiName` 系列（`ObjectTypeName`、`PropertyName` 等）在 `Deserialize` 内校验正则与保留字。
- `Ontology` 的聚合 map 用 `BTreeMap`（按 apiName 排序，输出确定性），`properties` / `parameters` 用 `IndexMap`（保留作者声明顺序）。`Ontology: PartialEq + Serialize`，支撑「YAML 与 JSON 得到同一 IR」测试：`assert_eq!(load(yaml_dir).ontology, load(json_dir).ontology)`；还要测文件顺序无关（打乱文件名排序后 IR 相等）。
- `SourceMap` 结构建议：`files: Vec<SourceFile{path: Utf8PathBuf, text: Arc<str>, docs: Vec<DocEntry{kind, api_name, node: Node}>}>` 与 `defs: HashMap<(DefKind, String), (FileId, DocIdx)>`；API `locate(def: DefKey, ptr: &[Seg], part: Key|Value) -> Location`。validate 与 lint 只说「ObjectType Flight 的 /properties/status/type 的 value」，不关心文件。

### Pattern 4：Diagnostic 数据模型与排序

```rust
// D-10：{ code, severity, message, file, line, column, span, help }
#[derive(Serialize)] #[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub code: String,                 // "ONT-V08" / "ONT-E09" / "ONT-W01"
    pub severity: Severity,           // 序列化为 "error" | "warning" | "info"
    pub message: String,
    pub file: String,                 // 相对 ontology 根、统一 '/'，不含 \\?\ 前缀
    pub line: u32, pub column: u32,   // 1 起；列按 Unicode 字符计
    pub span: Span,                   // { offset, length } 字节偏移，供 miette
    pub help: Option<String>,
}
```
排序键 `(file, line, column, code)` 保证快照确定。配置里的 `"warn"` 对应输出的 `"warning"`，文档里要写明。库 crate 不依赖 miette：CLI 的 `render.rs` 把 `Diagnostic` 加 `NamedSource`（来自 `SourceMap.files[..].text`）转成 `miette::Report`。

### Pattern 5：渲染与输出

- 人类可读：CLI 里显式构造 `miette::GraphicalReportHandler`（`--color auto|always|never`；`NO_COLOR` 或非 tty 用 `GraphicalTheme::unicode_nocolor()`）；快照测试固定用 `unicode_nocolor` 或 `NarratableReportHandler`，避免依赖终端探测。
- JSON：`--format json` 直接 `serde_json::to_string_pretty(&Vec<Diagnostic>)`，stdout 只有 JSON，无 ANSI；人类可读输出走 stderr 还是 stdout 需统一（建议诊断都走 stdout，进度/汇总走 stderr，JSON 模式汇总不输出）。
- 退出码：0 无 error（且 `--deny-warnings` 时无 warning）；1 有 error；2 用法错误（clap 默认）。IO / 找不到 manifest 也是 ONT-V14，退出码 1。

### Pattern 6：lint 注册表与配置

```rust
pub trait LintRule { fn id(&self) -> &'static str; fn default_level(&self) -> Level; fn check(&self, cx: &LintCx, out: &mut Vec<Finding>); }
// 静态数组 RULES: [&dyn LintRule; 13]（E08, E09, W01..W11）；Finding { rule, locator, message, help, severity_override }
// 生效级别 = ontoforge.toml [lint] 覆盖 或 default_level；"off" 跳过；规则可带阈值：[lint.options."ONT-W06"] max_properties = 60
```
- 配置里未知规则 ID 必须报错（带 strsim 建议），不能静默忽略（否则 `"ONT-W1" = "off"` 的笔误永远不生效）。
- 配置文件解析错误用 `toml::de::Error::span()` 渲染，文件字段为 `ontoforge.toml`。
- validate 的 `ONT-V*` 不受配置影响。

### Pattern 7：expr 与 schema 的 crate 依赖方向

`ontoforge-expr` 依赖 `ontoforge-schema`（ADR-0011 `schema ← expr`），所以 schema 的 validate 不能调用 CEL。CEL 检查入口在 expr：`pub fn check_ontology(loaded: &Loaded) -> Vec<Diagnostic>`；CLI（`ontoforge` crate）按顺序编排 `schema::load_and_validate` → `expr::check_ontology` → `schema::lint`。把编排函数放在 `ontoforge` crate 的 `lib.rs`（不是 `main.rs`），这样后续 server（Phase 6、7）和测试可复用，CLI 测试也不必全靠 spawn 进程。

### Anti-Patterns to Avoid

- **在 IR 里存 span：** 会破坏「YAML 与 JSON 得到同一 IR」的相等断言，也污染下游契约。span 只放 `SourceMap`。
- **直接 `#[serde(tag = "kind")]` 反序列化：** 经 Content 缓冲后 `serde_path_to_error` 路径丢失、`Spanned` 为 0:0。只用于 schema 侧（`#[schemars(tag)]`）。
- **把 `.json` 当 YAML 一股脑喂给 YAML 解析器：** 尾逗号、注释、单引号会被放过。
- **用 `HashMap` / 目录遍历原始顺序产生输出：** 诊断与 IR 顺序必须确定（排序 + `BTreeMap` / `IndexMap`）。
- **在测试里 `std::env::set_var`：** edition 2024 下它是 `unsafe`，被 `unsafe_code = "forbid"` 拦截；用 `Command::env()`。
- **依赖 `cel::Program::references()`：** 对 comprehension 变量误报。

## Definition Format Proposal（D-02/D-03/D-07/D-08/D-09 的具体形状）

下面是本研究推荐的具体 YAML 形状，planner 可直接锁定。它不改动任何 D-xx 决策，只补齐 CONTEXT 未明确的字段名。均为 `[ASSUMED: 设计推荐]`，但与 D-xx 一致。

```yaml
# ontology.yaml（manifest，无 kind；固定文件名；deny_unknown_fields）
apiName: aviation
displayName: Aviation
description: 航空运营示例
include: ["**/*.yaml", "**/*.yml", "**/*.json"]    # 可省略；manifest 自身与 ontoforge.toml 始终排除
```

```yaml
# objects.yaml
kind: ObjectType
apiName: Flight
displayName: Flight
description: 一次航班
aliases: [trip, sector]
hidden: false
primaryKey: flightId            # 必须是 properties 中存在的 key
titleKey: flightNumber          # 可省略，默认等于 primaryKey
properties:
  flightId:             { type: string, required: true }
  flightNumber:         { type: string, required: true, displayName: Flight number }
  departureAirportCode: { type: string, required: true }
  scheduledDeparture:   { type: timestamp }
  status:               { type: string }
  delayMinutes:         { type: integer }
  tags:                 { type: array, items: string }
---
kind: ObjectType
apiName: Airport
primaryKey: code
properties:
  code: { type: string, required: true }
  name: { type: string }
```

```yaml
# links.yaml
kind: LinkType
apiName: flightDepartureAirport     # camelCase，link type 之间唯一；addLink/removeLink 引用它
cardinality: oneToMany              # oneToOne | oneToMany | manyToMany；from 为「一」端
from: { objectType: Airport, apiName: departingFlights, displayName: Departing flights }
to:   { objectType: Flight,  apiName: departureAirport, displayName: Departure airport,
        foreignKey: departureAirportCode }      # 外键属性位于 to 端；类型必须等于 from 端主键类型
---
kind: LinkType
apiName: flightCrew
cardinality: manyToMany             # 无 foreignKey
from: { objectType: Flight, apiName: crew }
to:   { objectType: Pilot,  apiName: assignedFlights }
```

```yaml
# actions.yaml
kind: ActionType
apiName: delayFlight
displayName: Delay flight
parameters:
  flight:       { type: { objectReference: Flight }, required: true }
  delayMinutes: { type: integer, required: true }
  reason:       { type: string, default: "unspecified" }
rules:
  - kind: modifyObject
    object: flight                  # 必须是 objectReference 参数名
    properties:
      status:       { static: DELAYED }
      delayMinutes: { param: delayMinutes }
submissionCriteria:
  - condition: "params.delayMinutes > 0 && objects.flight.status != 'CANCELLED'"
    failureMessage: "延误分钟数必须为正数，且已取消航班不能再延误"
---
kind: ActionType
apiName: assignPilot
parameters:
  flight: { type: { objectReference: Flight }, required: true }
  pilot:  { type: { objectReference: Pilot },  required: true }
rules:
  - kind: addLink
    link: flightCrew
    from: flight                    # 参数名，须为 objectReference 且类型匹配 link 的 from 端
    to: pilot
submissionCriteria:
  - condition: "'dispatcher' in user.groups"
    failureMessage: "仅 dispatcher 组可以分配飞行员"
```

设计要点与理由：
- **link 端 apiName 的语义（相对 Foundry 的差异）：** Foundry 文档中「某一侧的 API name 是在另一端 object type 上取回这一侧对象的访问器」`[VERIFIED: create-link-type.md:109 原文 "if the API name on the \`Aircraft\` side of the link type is \`assignedAircraft\`, then calling \`Flight.assignedAircraft.get()\` will return the \`Aircraft\` objects linked to those \`Flight\` objects."]`。本推荐把 `apiName` 写在它所访问的那一端旁边，语义改为「`objectType` 上用于到达另一端的访问器名」（`from.apiName = departingFlights` 是 Airport 上的访问器，返回 Flight）。这与 D-04「在所关联的 object type 范围内唯一，且不能与该类型的属性同名」直接对应：每个访问器名都落在 `objectType` 的命名空间里。这是与 Foundry 措辞的有意差异，需在 JSON Schema description 与示例注释里写明。
- **`foreignKey` 只出现在 `oneToOne` / `oneToMany` 的 `to` 端**；`manyToMany` 禁止出现。Foundry 对 1:1 与 1:N 同样要求在外键端修改 FK `[VERIFIED: rules.md:16 原文 "To create or delete one-to-many or one-to-one links, object rules need to be used and the foreign key property on the object needs to be modified."]`。
- **REQ SCHEMA-04 的「M:N（join 定义）」：** v1 没有 backing dataset，M:N 的 join 由系统管理的 link 表承担（Phase 2 通用表设计）；本 phase 的 join 定义即两端 `(objectType, apiName)` 声明，不增加字段。后续如需 join 额外属性，按 D-05 属向后兼容扩展。
- **ParamType 的 serde 表示：** 把参数类型做成 externally tagged enum：单元变体（`string`、`integer`、...）接受标量字符串，`ObjectReference(ObjectTypeName)` 接受 `{ objectReference: Flight }`；serde 对「字符串或单键 map」的 externally tagged enum 原生支持，且 `serde_path_to_error` 保留路径。无需 `untagged`。数组型参数：`{ type: array, items: string }` 与属性一致。
- **ValueSource：** `enum ValueSource { Param(ParamName), Static(serde_json::Value), CurrentUser(Empty), CurrentTime(Empty) }`，`rename_all = "camelCase"`，externally tagged，`{ currentUser: {} }` 的 `Empty` 是 `deny_unknown_fields` 的空 struct。
- **`titleKey` 默认等于 `primaryKey`、PK 属性隐含 `required: true`（显式 `required: false` 报 V10）：** Foundry 把 title key 与 primary key 都列为保存对象类型的必填项 `[VERIFIED: create-object-type.md:292-293 "Title key" "Primary key" 在 "Additionally, the following property fields must not be empty" 列表中]`；SCHEMA-02 把 title key 列为可定义项，故取默认值而非强制。
- **清单外的字段暂不加：** `pluralDisplayName`、`editsOnlyViaActions`（Phase 3）、`agentDescription`（Phase 7）、`formatVersion` 都属于 D-05 允许的向后兼容扩展，本 phase 不加；是否在 manifest 增加 `formatVersion: 1` 见 Open Questions。

## Domain Facts（Foundry 文档实读）

### 1. v1 base type：主键 / title key / array 适用性

SCHEMA-03 的 v1 类型为 Byte / Short / Integer / Long / Float / Double / Decimal、String、Boolean、Date、Timestamp、Array、Geopoint。原表节选（`docs/foundry/ontology/object-link-types/properties-overview.md:22-35`）：

```
| Commonly used: `String`, `Integer`, `Short` | Yes | Yes |  |
| Time-based: `Date`, `Timestamp` | Yes | Discouraged | ...
| Number-like: `Boolean`, `Byte`, `Long` | Yes | Discouraged | `Boolean` limits your object type to two object instances. ...
| Float-like: `Float`, `Double`, `Decimal` | Yes | No |  |
| `Array` | Yes | No | Array properties cannot contain null elements. If the inner type of the `Array` is not a valid title property, the `Array` property also cannot be used as the title property. Nested arrays are not supported in Object Storage v2. |
| `Geopoint` | Yes | No | Geopoint values are stored as a comma-separated string in the format `latitude,longitude` (for example, `57.64911,10.40744`). ...
```
`[VERIFIED: properties-overview.md:22-35]`。另 `base-types.md:26` 原文 "All base types may be used in arrays to represent multiple values for a property, excluding the `Vector` and `Time series` types."（`Vector` / `Time series` 不在 v1）。

v1 适用性（派生自上表，供 `basetype.rs` 的纯函数 `pk_suitability` / `title_suitability` 使用）：

| v1 type | 可作 title key | 可作 PK | 备注 |
|---------|---------------|---------|------|
| string, integer, short | 是 | 是（Valid） | 推荐 PK 用 string |
| date, timestamp, boolean, byte, long | 是 | Discouraged | E09 给 warning |
| float, double, decimal | 是 | Invalid | V10 hard error |
| geopoint | 是 | Invalid | V10 hard error |
| array | 是（内层类型可作 title 时）；v1 全部内层均可 | Invalid | V10 hard error；嵌套 array 为 E09 error |

参数类型映射（`docs/foundry/ontology/action-types/scale-property-limits.md:44-66`，节选）：`Byte → Integer`、`Short → Integer`、`Float → Double`、`Long → Long`、`Decimal → Decimal`、`Geopoint → Geopoint`、`Date → Date`、`Timestamp → Timestamp` `[VERIFIED: scale-property-limits.md:48,53,58,62]`。因此「属性类型 X 接受参数类型 Y」按 class 折叠比较：{byte, short, integer} 同属 integer 类，{float, double} 同属 double 类。

主键不能被 action 修改：`[VERIFIED: scale-property-limits.md:96 "Currently, actions cannot be used to edit the **primary key** of an object."]`

### 2. apiName 规则与保留字

- object type：`[VERIFIED: create-object-type.md:258-261]` "Begin with an uppercase character and consist of only alphanumeric characters." / "Be written in PascalCase" / "Be unique across all object types." / "Be between 1 and 100 characters long."
- property：`[VERIFIED: create-object-type.md:267-270]` "Begin with a lowercase character and consist of only alphanumeric characters." / "Be written in camelCase" / "Be unique across all properties belonging to the same object type." / "Be between 1 and 100 characters long."
- link 端：`[VERIFIED: create-link-type.md:111-115]` "Begin with a lowercase character and consist of only alphanumeric characters." / "Be unique across all link types associated with the same object type." / "Be between 1 and 100 characters long." / "Be NFKC normalized." / "Not be a reserved keyword."（ASCII 正则天然满足 NFKC。）
- **保留字（补齐 D-04）：** `[VERIFIED: create-object-type.md:327 "A number of reserved keywords cannot be used for API names: \`ontology\`, \`object\`, \`property\`, \`link\`, \`relation\`, \`rid\`, \`primaryKey\`, \`typeId\`, and \`ontologyObject\`."]` 文档未写大小写规则；推荐按大小写不敏感匹配，覆盖 object type 的 `Object`、`Link`、`Property`（PascalCase 名字在 case-sensitive 下永远不会命中全小写保留字，因此不加这条会让该规则对 object type 失效）`[ASSUMED]`。
- 文档里另有一处较宽松的描述（"Consist of only alphanumeric characters and underscores"，create-object-type.md:317），与前述 "only alphanumeric" 冲突；D-04 已锁定无下划线的正则，本研究以 D-04 为准。含义：W03 的 snake_case 分支与 W05 的前导 `_` 分支在 apiName 上不可达（被 V07 先拦下）。
- action type 与 parameter 的保留字文档未写，沿用同一保留字表 `[ASSUMED]`。

### 3. link type 语义

`[VERIFIED: create-link-type.md:48]` "*One-to-one cardinality:* ... the one-to-one cardinality serves as an indicator of the intended relationship, but the one-to-one cardinality is not enforced."；`link-types-overview.md:22` link type 双向、两侧各有 display name 与 API name；M:N 由 join table 支撑；FK 型要求外键属性类型与对端主键类型一致 `[VERIFIED: create-link-type.md:69-70 "The foreign key matches the primary key of the linked object type." / "The property types of both objects match."]`。同一对 object type 间允许多个 link type（`link-types-overview.md:26`），支持自关联。

### 4. action rule 语义（E08 的依据）

rules 原文要点（`docs/foundry/ontology/action-types/rules.md`）：
- 第 5、6 条：Create/Delete link 只用于多对多 `[VERIFIED: rules.md:22-23 "For foreign key links, one has to use the **Modify object** rule to explicitly modify the foreign key property."]`
- 第 2、4 条：Modify/Delete 的对象来自 object reference 参数，不能引用本 action 内新建的对象 `[VERIFIED: rules.md:19,21 "This cannot reference an object created as a part of the current action."]`
- 第 7 条：Function rule 存在时不得有其他 rule（v1 无 function rule，`kind: function` 视为 ONT-V03 不支持）`[VERIFIED: rules.md:24]`
- 值来源：`[VERIFIED: rules.md:35-38]` From parameter / Object parameter property / Static value / Current User/Time；"String and timestamp properties can also take on contextual values in the form of the action's current user or the time"。
- 非法组合：`[VERIFIED: rules.md:50-52 "Objects cannot be deleted before they are added or modified." "Objects cannot be modified before they are added." "Objects cannot be created twice in one form submission."]`；多 rule 编译为每对象一个 edit，后写覆盖前写，rule 顺序有意义 `[VERIFIED: rules.md:48]`。

submission criteria 原文要点（`submission-criteria.md`）：条件模板 Current user（user ID、group IDs 含继承成员）/ Parameter / Execution context；单值运算符 is、is not、matches、is less than、is greater than or equals；多值 includes、includes any、is included in、each is、each is not；all/any/none 逻辑组合；"Avoid using `NOT` conditions with group, marking, or organization memberships"（lines 38）；不支持 attachment 与 object set 参数（line 59）。ontoforge 用 CEL 字符串表达这些（D-09），对应关系：`is` → `==`，`includes` → `x in list`，`matches` → `.matches()`，`each is` → `list.all(x, ...)`。ONT-W16（对 group 成员用 NOT）属 v2 范围表（spec 表中 W12–W16 不在本 phase），但静态可检测，作为可选加分项列入 Open Questions。

### 5. 对应「lint 规则表」原文

`[VERIFIED: .planning/research/foundry-docs-spec.md:297-298,303-313]`：

```
| ONT-E08 | E | 含 Function rule 的 action 不得有其他 rule；Create/Delete link rule 只用于 M:N，1:N/1:1 改 FK；rule 组合不得出现先删后增/改、先改后增、重复创建 | at/rules.md |
| ONT-E09 | E | Base type 与用途匹配：PK 只能是 `String`/`Integer`/`Short`（`Date`/`Timestamp`/`Boolean`/`Byte`/`Long` 警告）；title key 不得为 `Vector`/`Struct`/...；struct 不嵌套、字段非 array；array 不含 `Vector`/`Time series`、OSv2 不嵌套 | ... |
```
W01–W11 原文见该文件第 303–313 行，下一节逐条落地。

## Validate 诊断目录（ONT-V，hard error，不可配置）

D-06 要求结构性错误是 hard error。为与 lint 的 ONT-E/W/I 区分并保持稳定，推荐码段 `ONT-V01..V14`（V = validate）`[ASSUMED: 命名]`。planner 可在此基础上细分，但「码一经发布不可复用」。

| Code | 含义 | 定位 span | help 示例 |
|------|------|-----------|-----------|
| ONT-V01 | YAML/JSON 语法错误、重复 key、预算超限、非 UTF-8 | 解析器给的 line:col | |
| ONT-V02 | 文档不是 mapping、缺 `kind`、`kind` 不是字符串 | 文档根或 `kind` 值 | |
| ONT-V03 | 未知 `kind`，或预留 kind（`Interface`/`ValueType`/`SharedProperty`，以及 `function` rule）在 v1 不支持 | `kind` 值 | 「v1 不支持 Interface」或 strsim 建议 |
| ONT-V04 | 未知字段（`deny_unknown_fields`） | 写错的 key | `你是否想写 primaryKey` |
| ONT-V05 | 缺少必填字段 | 所在 mapping 起点或 `kind`/apiName 值 | |
| ONT-V06 | 类型或取值错误（serde invalid type/value、未知 base type、未知 cardinality） | 该 value（或 seq 元素） | 列出可选值 |
| ONT-V07 | apiName 格式违规（正则 D-04）或命中保留字 | apiName 值 | 给出规范化建议，如 `flight` → `Flight` |
| ONT-V08 | 重复：object/link/action apiName、同类型内属性、参数、link 访问器与属性同名 | 后出现者；消息带首次出现位置 `path:line:col` | |
| ONT-V09 | 引用不存在：link 端 objectType、foreignKey 属性、rule 的 objectType / property / link / 参数、objectReference 目标类型 | 引用处的值 | 近似名建议 |
| ONT-V10 | 主键/title key：未声明、指向不存在属性、PK 类型 Invalid（float/double/decimal/geopoint/array）、PK 显式 `required:false` | `primaryKey`/`titleKey`/属性 | |
| ONT-V11 | link 一致性：`foreignKey` 与 cardinality 不符（manyToMany 不得有；oneToOne/oneToMany 必须有）、FK 属性不在 `to` 端 object type 上、FK 类型 ≠ `from` 端主键类型 | `foreignKey` 值或 cardinality 值 | |
| ONT-V12 | action 结构：createObject 缺 PK 赋值或缺 required 属性赋值、modifyObject 赋值主键、objectReference 参数类型与 rule 目标不符、值来源类型与目标属性不兼容（见 Domain Facts 1 的 class 折叠）、`static`/`default` 字面量越界或格式错误、currentUser 赋给非 string、currentTime 赋给非 timestamp | 具体 value | |
| ONT-V13 | CEL：语法错误、长度/嵌套超限、自由变量根不在 {params,user,objects}、未知参数/成员/属性、`objects.x` 不是 objectReference 参数、顶层结果可推断为非 bool、比较类型不兼容 | `condition` 值（消息里附表达式内 line:col） | |
| ONT-V14 | manifest / 文件系统：找不到 `ontology.yaml`、include 无匹配、include 含 `..` 或绝对路径、文件过大、读取失败 | manifest 或路径 | |

边界决策（回应 D-06 与 D-12 的重叠，见 Assumptions A5）：「主键类型 Invalid」放在 V10（存储依赖此不变量，必须 hard）；E09 负责 Discouraged 主键（Date/Timestamp/Boolean/Byte/Long，默认 warning）、title key 适用性与 array 元素约束（嵌套 array，默认 error）。同一份 `basetype.rs` 表同时被两层使用，保证不重复定义。

## Lint 规则映射（E08、E09、W01–W11 → v1 IR 静态检查）

通用约定：每条规则一个模块 `lint/<id>.rs`，输入 `&LintCx { ontology: &Ontology, sources: &SourceMap, options }`，输出 `Finding`。名字分词用同一个 `split_camel(&str) -> Vec<String>`（`ReleaseDate` → `[release, date]`，数字单独成词；约 10 行，单测覆盖）。默认级别：E→error，W→warn。阈值可通过 `[lint.options."ONT-Wxx"]` 覆盖，默认值如下均为 `[ASSUMED]`，来源于 spec 表或合理启发式，planner 应保留在一处常量里。

| ID | 作用域 | 静态检查（输入 → 判定） | 默认阈值 / 控制误报 | fixture（触发 / 不触发） |
|----|--------|--------------------------|--------------------|---------------------------|
| ONT-E08 | action | (a) `addLink`/`removeLink` 的 link 的 cardinality ≠ manyToMany → error，help 指向「用 modifyObject 修改 `<foreignKey>`」；(b) 同一 objectReference 参数上 `deleteObject` 之后又出现 `modifyObject` 或再次 `deleteObject` → error；(c) 同一 action 内两条 `createObject` 目标同一 objectType 且主键值来源相同（相同 `param` 名或相等 `static`）→ error（重复创建）；(d) `modifyObject` 赋值主键属性 → error（`scale-property-limits.md:96`）。「先改后增」在 v1 不可表达（modify 的对象必为参数）；function rule 不可表达（V03），均记为 N/A | 无阈值 | 触发：addLink 指向 oneToMany link、delete 后 modify、重复 createObject、modify PK。不触发：addLink 指向 manyToMany，modify 后 delete（顺序合法） |
| ONT-E09 | 属性 / object type | (a) PK 类型 ∈ Discouraged → finding（severity warning）；(b) title key 类型适用性（v1 全部适用，保留函数）；(c) `items` 为 `array`（嵌套 array）→ error；(d) 提醒：`array` 属性 `required: true` 表示至少 1 个元素（文档语义，仅 info 不输出）。Invalid 主键类型已在 V10 | 同一规则 ID 下 error / warning 混合，`ontoforge.toml` 的覆盖对整条规则统一生效（见 A5） | 触发：`primaryKey` 指向 timestamp / long；`items: array`。不触发：PK 为 string |
| ONT-W01 | object type | apiName 的分词中最后一词（小写）∈ {data, item, record, info, object, entity} 且整名由泛化词构成或仅有 1~2 词 → 泛化；或最后一词看起来是复数：以 `s` 结尾且不以 `ss`、`us`、`is` 结尾，不在白名单 {series, species, news, status, address, analysis, business, access, class, process} 内，或以 `ies` 结尾 | 复数启发式易误报，默认 warn，建议在 help 里写出「若有意可 off」 | 触发：`Data`、`Flights`；不触发：`Status`、`Address`、`Flight` |
| ONT-W02 | 属性 | 属性名（整名小写）∈ {value, type, status, date, name, quantity, score}，且同一 object type 内存在另一属性，其分词最后一词等于该泛化词（如 `status` 与 `paymentStatus`）→ 歧义 | 无另一同后缀属性则不报，避免对仅有一个 `name` 的类型误报 | 触发：`status`+`shippingStatus`；不触发：只有 `name` |
| ONT-W03 | 属性 | apiName 匹配任一：`^dt[A-Z]`、`^n[A-Z]{2,}`、`\d{2,}$`、`field[A-Z0-9]`（snake_case 分支被 V07 排除，不实现） | 固定正则，regex crate 线性时间 | 触发：`dtCreated`、`nTotal`、`code01`、`field1`；不触发：`createdAt`、`level2`（单个数字结尾不报） |
| ONT-W04 | ontology（跨类型） | 收集所有 date / timestamp 类型属性名，判风格：A = 以 `Date`、`At`、`Time` 结尾；B = 以 `dateOf`、`timeOf` 开头。两种风格同时存在 → 对少数风格的属性报 warn；数量相同报后出现者 | 仅当 A、B 同时出现才报 | 触发：`createdDate` + `dateOfBirth`；不触发：全是 A |
| ONT-W05 | 属性 | 分词序列包含任一子序列：[etl]、[extracted, at]、[received, at]、[batched, at]、[sequence]、[table, version]、[internal, record, id]（前导 `_` 分支被 V07 排除） | `sequence` 可能是正当领域词（如航段序号），warn 级别 + help 提示可 off | 触发：`etlBatchId`、`rowSequence`；不触发：`createdAt` |
| ONT-W06 | object type | 属性数 > `max_properties`（默认 60）→ warn；或存在判别列（属性名分词最后一词 ∈ {type, subtype} 且类型为 string）且属性数 > `max_properties / 2`（默认 30）→ warn。空值率分支不实现（D-13） | 阈值可配 | 触发：61 个属性；31 个属性 + `accountType`。不触发：20 个属性 + `type` |
| ONT-W07 | ontology（跨类型） | (a) object type 名第一词 ∈ {hr, sales, billing, crm}（可配列表）→ silo 前缀；(b) 任两个 object type：主键同名同类型，且属性集合 `{(name,type)}` 的 Jaccard ≥ 0.6，且各自属性数 ≥ 4 → 疑似孤岛 | 仅基于属性名与类型集合（D-13） | 触发：`SalesOrder`；`CrmContact` 与 `Contact` 重合度高。不触发：Jaccard 0.3 |
| ONT-W08 | ontology（跨类型） | 以 `(name,type)` 为键统计，若存在 ≥ 3 个 object type 共享 ≥ 3 个相同键（交集大小 ≥ 3）→ 建议提取 interface / shared property；先用「出现在 ≥ 3 个类型的键」预过滤再枚举三元组，复杂度可控；按共享键集去重 | 常见 `createdAt`/`updatedAt` 恰好 3 个类型共享时易触发，所以要求交集 ≥ 3 个属性 | 触发：3 类型共享 `createdAt`/`updatedAt`/`ownerId`；不触发：共享 2 个 |
| ONT-W09 | action | (a) 归属于某 object type 的 action（其 rules 中 createObject/modifyObject/deleteObject 的目标类型，modify/delete 经参数类型解析）数 > `max_actions_per_type`（默认 10）→ warn；(b) apiName 匹配 `^(set|update)[A-Z]`（spec 的 `^(Set|Update) \w+` 针对带空格的显示名，apiName 是 camelCase）且 rules 仅 1 条 `modifyObject` 且只赋 1 个属性 → warn | | 触发：11 个 action；`setStatus` 只改 status。不触发：`delayFlight` 改 2 个属性 |
| ONT-W10 | object type / 属性 | 属性名整名 ∈ {version, revision, isCurrent}；object type 名匹配 `\d{4}$` 或 `[vV]\d+$` | | 触发：`isCurrent`、`Flight2024`、`FlightV2`；不触发：`Version`（类型名不查属性规则） |
| ONT-W11 | link | link 的 apiName 或任一端 apiName 匹配 `^related` 或 `^link\d`（大小写不敏感）→ 泛化。「缺任一方向 apiName」在 v1 由格式保证（V05），此规则不再重复 | | 触发：`relatedFlights`、`link1`；不触发：`departingFlights` |

规则的 locator 约定：Finding 用 `(DefKey, 指针, Part)` 表达，由 `SourceMap` 解析为 span。例如 W01 定位 `ObjectType/<name>` 的 `/apiName` value；W03 定位 `/properties/<p>` 的 key；W04 定位少数风格属性的 key；E08 定位 `rules[i]` 下相关 key（`link` / `object` value）。

可选扩展（CONTEXT `<specifics>` 的 `model:check` 风格）：「criteria 引用了可选且无默认值的参数却没有 `has()` 保护」是能编译但运行时会出错的写法。它不在 W01–W11 与 spec 表中，也不是 D-12/D-13 的范围；建议作为 `ontoforge-expr` 产出的可配置 lint 规则 `ONT-W17`（`ONT-W12`–`W16` 已被 spec 表占用）。是否在本 phase 加入见 Open Questions。

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| YAML 解析 / JSON 解析 | 自写 tokenizer | `serde-saphyr`（YAML+JSON 节点树）+ `serde_json`（JSON 严格预检） | YAML 1.2 边界极多（缩进、flow、多文档、BOM、alias 预算） |
| 反序列化错误路径 | 自己追踪 path | `serde_path_to_error` | externally tagged 下完整路径（实测） |
| JSON Schema 生成 | 手写 schema JSON | `schemars` derive（+ `#[schemars(tag="kind")]`） | 与类型同源，不会漂移 |
| 诊断渲染 | 自己画下划线与源码片段 | `miette` GraphicalReportHandler | 字节 span、多行、unicode / 无色主题 |
| 「你是否想写 X」 | 自己写编辑距离 | `strsim::jaro_winkler` / `levenshtein` | 现成、无依赖 |
| glob 匹配与目录遍历 | 自己展开 `**` | `globset` + `walkdir`（`follow_links(false)`） | 跨平台、路径分隔符处理 |
| UTF-8 路径与分隔符归一 | `PathBuf` 手工替换 `\` | `camino::Utf8Path` | 诊断里统一 `/`，避免 `\\?\` |
| TOML 配置解析与错误 span | 手工解析 `ontoforge.toml` | `toml` + serde，`Error::span()` | 实测可用 |
| CEL 语法与 AST | 自写表达式解析 | `cel` 的 `Env::compile` 与 pub AST | 自写只有「引用/类型检查」那一层 |
| 日期时间字面量校验 | 正则 | `chrono`（`NaiveDate::parse_from_str`、`DateTime::parse_from_rfc3339`） | 闰年等边界 |
| 快照测试 | 手写 golden 文件对比 | `insta` | 自动归一化 CRLF |

**Key insight:** 本 phase 真正需要自建的只有三块薄逻辑：Node 树与路径回查、CEL 的作用域感知引用遍历与迷你类型推断、lint 规则本身。其余都是组合成熟 crate。

## Runtime State Inventory

greenfield phase（仓库目前只有 `.planning/` 与 `docs/`），无 rename / migration，该节省略。

## Common Pitfalls

### Pitfall 1：Windows debug 构建的栈溢出（两处，进程直接 abort）
**What goes wrong:** (a) `cel` 的 antlr 解析器在 1 MiB 主线程栈 debug 构建下，括号嵌套约 10 层溢出；(b) `serde-saphyr` 的 `Spanned` 节点树在 debug 下，纯 mapping 嵌套 24 层溢出（20 层通过）；`Budget.max_depth` 默认 64 保护不了 1 MiB 栈。
**Why it happens:** debug 栈帧大 + Windows 主线程栈仅 1 MiB（实测）。
**How to avoid:** CLI `main` 把整个流水线放到 `std::thread::Builder::new().stack_size(32 << 20)` 的线程里；`.cargo/config.toml` 设 `[env] RUST_MIN_STACK = "16777216"` 让 `cargo test` 线程栈更大；`Options.budget.max_depth = 32`；CEL 表达式预限 `len <= 4096` 字符、括号/方括号/花括号最大嵌套 `<= 32`，超限直接 ONT-V13，不调用 `compile`。16 MiB 线程实测可解析 60 层；32 MiB 线程可编译 60 层括号。
**Warning signs:** `thread 'main' has overflowed its stack` 且无 Rust panic 信息。
**测试:** 必须有测试 `deeply_nested_expression_is_rejected_not_crashing` 与 `deeply_nested_yaml_is_rejected_not_crashing`。

### Pitfall 2：`Spanned` 与 `serde_path_to_error` 在 internally tagged 里失效
见 Pattern 2。症状：位置 0:0、路径 `.`。

### Pitfall 3：YAML 隐式类型
**What goes wrong:** 默认 `strict_booleans=false` 会把 `yes/no/on/off` 当布尔；`017` 被解析为浮点 17.0（实测）；`1_000` 被解析为 1000；`1.10` 变 1.1。
**How to avoid:** `strict_booleans = true`；类型为 string 的字段（apiName、enum 值）收到非字符串标量时，诊断 help 写「请加引号」；`static: no` 之类需要字符串时作者必须加引号。

### Pitfall 4：JSON 经 YAML 解析器过于宽松
`{"a":1,}` 与注释会被 serde-saphyr 接受（实测 `saphyr on bad json: true`，而 `serde_json` 报 `trailing comma at line 2 column 16`）。**How to avoid:** `.json` 文件先 `serde_json::from_str::<IgnoredAny>` 严格预检，失败产出 ONT-V01，行列取自 `serde_json::Error::line()/column()`。

### Pitfall 5：行列与字节偏移的单位
serde-saphyr 的 line/column 按字符计（1 起），BOM 不计列但计字节；miette 需要字节偏移。**How to avoid:** `SourceSpan` 用 `Location.span().byte_offset()/byte_len()`（仅 `&str` 输入可用，所以一律 `read_to_string` 后 `from_*_str`，不要用 reader API）；JSON 的 key span 包含引号，渲染时可接受。

### Pitfall 6：路径与行尾的平台差异
`canonicalize` 在 Windows 产生 `\\?\` 前缀（cargo fmt 的 diff 输出里能看到），诊断的 `file` 字段必须是相对 ontology 根、统一 `/` 的路径：用 `camino` + `strip_prefix`，不要 `canonicalize`。本机 `core.autocrlf=true`，缺 `.gitattributes` 时工作区文件会变 CRLF，导致 fixture 字节偏移与 CI（Linux）不一致；`* text=auto eol=lf` 解决。`.gitattributes` 还应对 `*.snap`、`*.json`、`*.yaml`、`*.toml` 显式 `text eol=lf`。

### Pitfall 7：输出非确定性
`HashMap` 迭代、`walkdir` 原始顺序、`serde_json::Map` 默认字母序。**How to avoid:** 发现的文件按规范化相对路径排序；`Ontology` 用 `BTreeMap`；`properties` / `parameters` 用 `IndexMap` 且启用 `serde_json` 的 `preserve_order`；诊断按 `(file, line, column, code)` 排序。

### Pitfall 8：edition 2024 与 `unsafe_code = "forbid"`
测试里不能 `std::env::set_var`（edition 2024 起为 `unsafe`）。CLI 测试用 `Command::env("NO_COLOR","1")`。`gen` 是保留关键字，不要用作标识符。

### Pitfall 9：`include` 默认 glob 的副作用
默认 `**/*.yaml`、`**/*.yml`、`**/*.json` 会匹配 manifest 自身、`.github/workflows/*.yml`（若 ontology 目录在仓库根）、`package.json` 等无关文件，而每个无 `kind` 的文档都会报 V02。**How to avoid:** 始终排除 manifest 与 `ontoforge.toml`；遍历时跳过以 `.` 开头的目录及 `target`、`node_modules`；`include` 含 `..` 或绝对路径时拒绝（V14）；不跟随符号链接。

### Pitfall 10：`ontoforge.toml` 的 key 与未知规则 ID
规则 ID 含连字符，TOML 里必须写成带引号的 key `"ONT-W01" = "off"`；未知规则 ID 必须报错（见 Pattern 6）。

### Pitfall 11：`cel` 的语义盲区
`references()` 误报 comprehension 变量；未知函数（如 `nope(1)`）也能 compile 成功，且 `Env` 没有公开的函数名枚举接口。**How to avoid:** 自写遍历；未知函数检测不在本 phase 范围（交给 Phase 3 的运行时求值错误），若要做则维护一份 CEL 标准库函数名白名单常量并加测试固定它 `[ASSUMED]`。`Date`/`Timestamp`/`Geopoint`/`Decimal` 属性在 CEL 里的运行时类型取决于 Phase 3 如何注入，所以迷你推断对它们一律返回 `Any`（不报类型错）。

### Pitfall 12：insta 在 CI 与本地的行为
`cargo-insta` 本机未安装，更新快照用 `INSTA_UPDATE=always cargo test -p <crate>`；CI 环境（`CI=true`）insta 默认不写入，缺失快照即测试失败，符合预期。快照里不要出现绝对路径、耗时、机器相关内容（诊断 `file` 已相对化）。

### Pitfall 13：serde-saphyr 的错误文案是库的措辞
`Error::to_string()` 在 `with_snippet=false` 时形如 `duplicate mapping key: a, set DuplicateKeyPolicy in Options if acceptable at line 2, column 1`（实测）。需要去掉尾部的 ` at line L, column C` 并按需改写开发者向的提示再放进 ONT-V01。

## Code Examples

### 严格解析选项
```rust
// Source: spike s1-yaml/examples/tricky.rs（实跑）
let mut o = serde_saphyr::Options::default();
o.strict_booleans = true;
o.duplicate_keys = serde_saphyr::DuplicateKeyPolicy::Error;
o.merge_keys = serde_saphyr::MergeKeyPolicy::Error;
o.reject_unsupported_tags = true;
o.with_snippet = false;
// o.budget.as_mut().unwrap().max_depth = 32;
let docs: Vec<Node> = serde_saphyr::from_multiple_with_options(text, o)?;
```

### 路径回查 span
```rust
// Source: spike s1-yaml/examples/pipeline.rs（实跑，简化）
fn lookup(root: &Node, path: &serde_path_to_error::Path) -> Location {
    use serde_path_to_error::Segment as S;
    let (mut cur, mut last_key) = (root, None);
    for seg in path.iter() {
        match (seg, &cur.kind) {
            (S::Map { key }, Kind::Map(m)) | (S::Enum { variant: key }, Kind::Map(m)) =>
                match m.iter().find(|(k, _)| &k.value == key) {
                    Some((k, v)) => { last_key = Some(k.referenced); cur = v; }
                    None => continue,                       // Enum 段是 kind 重写出的透明包装
                },
            (S::Seq { index }, Kind::Seq(s)) => match s.get(*index) { Some(v) => { cur = v; last_key = None } None => break },
            _ => break,
        }
    }
    last_key.unwrap_or(cur.loc)
}
```

### schema 导出（含 `$schema` 注入）
```rust
// Source: spike s3-schema（schema_for! 已实跑）；注入 $schema 为 [ASSUMED] 的后处理
let schema = schemars::schema_for!(DefinitionFile);             // draft 2020-12
let mut v = serde_json::to_value(&schema)?;
// 后处理：给 $defs.Document.oneOf[*].properties 注入 "$schema": {"type":"string"}，loader 读取时丢弃顶层 "$schema" key
std::fs::write("schemas/ontoforge-definition.schema.json", serde_json::to_string_pretty(&v)? + "\n")?;
```
编辑器接入：YAML 文件头 `# yaml-language-server: $schema=../../schemas/ontoforge-definition.schema.json`（示例 ontology 里附上），manifest 用另一份 schema。导出两份：`ontoforge-manifest.schema.json`、`ontoforge-definition.schema.json`（后者根为 `anyOf: [Document, Document[]]`，覆盖 JSON 数组文件）。

### CLI 集成测试（零额外依赖）
```rust
// Source: spike s4-ws/crates/ontoforge/tests/cli.rs（实跑）
let out = std::process::Command::new(env!("CARGO_BIN_EXE_ontoforge"))
    .args(["validate", "../../examples/ontologies/aviation"]).env("NO_COLOR", "1").output().unwrap();
assert!(out.status.success());
```

### CI 骨架（D-18）
```yaml
# .github/workflows/ci.yml  [ASSUMED: 未在本机运行；action 最新 tag 经 gh api 查得]
name: ci
on: { push: { branches: [main] }, pull_request: {} }
jobs:
  test:
    strategy: { fail-fast: false, matrix: { os: [windows-latest, ubuntu-latest] } }
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v7           # 最新 v7.0.1
      - uses: dtolnay/rust-toolchain@master
        with: { toolchain: "1.98.0", components: "clippy, rustfmt" }
      - uses: Swatinem/rust-cache@v2        # 最新 v2.9.2
      - run: cargo fmt --all --check
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - run: cargo test --workspace
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `serde_yaml`（已 deprecated） | `serde-saphyr`（YAML 1.2、预算、Spanned） | 2025-2026 | ADR-0003 已选；v2.0.0 于 2026-10-13 发布 |
| `cel-interpreter` | `cel`（0.15.0） | 改名，旧名停在 0.10.0（2025-07） | 依赖名写 `cel` |
| schemars 0.8（draft-07、`schema_for!` 返回 RootSchema） | schemars 1.x（默认 2020-12、`$defs`、`oneOf` + `const`） | 1.0 | 编辑器兼容性见 S3 |
| JSON Schema `discriminator` | `oneOf` + `const`（或 `if/then`） | JSON Schema 无此关键字 | 补全收窄需实机验证 |

**Deprecated/outdated:** `serde_yaml`、`serde_yml`、`json-spanned-value`（停更于 2020）。

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | 推荐的 YAML 形状（LinkType 的 `from`/`to`、访问器 `apiName` 语义、`foreignKey` 位置、`cardinality` 取值、`submissionCriteria` 字段名、manifest 无 `kind`、`ParamType` 用 externally tagged） | Definition Format Proposal | 属 one-way 契约（D-02/D-08），错了所有用户文件要迁移；但 CONTEXT 把字段名留给 Claude，落盘前应在 PLAN 里作为一个明确锁定点 |
| A2 | 保留字按大小写不敏感匹配，并同样适用于 action type 与 parameter | Domain Facts 2 | 过严会拒绝合法名字（如 `Object`）；过松则后续 codegen 冲突 |
| A3 | `titleKey` 缺省取 `primaryKey`；PK 隐含 required，显式 `required:false` 报 V10 | Definition Format Proposal | 影响示例与 V10 的细节 |
| A4 | Validate 码段命名 `ONT-V01..V14` 与各码语义 | Validate 诊断目录 | 码一旦发布不可复用，需早定 |
| A5 | PK Invalid 类型归 V10（hard），E09 只管 Discouraged PK / title key / 嵌套 array，且 E09 同一规则 ID 下 error 与 warning 混合、配置覆盖整条生效 | Lint 规则映射 | D-06 与 D-12 字面有重叠；若用户希望 E09 独占全部主键类型检查，则 V10 的 PK 类型分支改为 E09 error，但失去「hard」保证 |
| A6 | 各 W 规则的默认阈值与启发式（W01 复数白名单、W02 歧义判据、W06 `max_properties=60` 及判别列阈值 30、W07 前缀表与 Jaccard 条件、W08 交集 ≥3、W09 阈值 10） | Lint 规则映射 | 误报或漏报；均可配置，风险低 |
| A7 | CEL 迷你类型推断范围（Date/Timestamp/Geopoint/Decimal 返回 Any；仅 Int/Double/String/Bool/List 参与不兼容检查；整数与浮点不互斥报错） | Pitfall 11、S2 | 过严产生误报，过松漏检；Phase 3 注入类型后可收紧 |
| A8 | CEL 表达式长度上限 4096、嵌套上限 32、worker 线程栈 32 MiB、YAML `max_depth` 32 | Pitfall 1 | 过紧拒绝合法表达式；数值来自实测阈值的保守折中，可配置 |
| A9 | yaml-language-server 对 `oneOf + const` 的补全收窄效果；必要时后处理为 `if/then` | S3 | 仅影响编辑器体验，不影响校验 |
| A10 | CI 配置（action 版本、`dtolnay/rust-toolchain@master` 带 `toolchain: 1.98.0`） | Code Examples | CI 首次运行才能验证 |
| A11 | `serde_json` 的 `preserve_order` 能让 `Value` map 保持作者声明顺序 | Standard Stack | 若无效，`properties` 顺序变字母序；Wave 0 加一个顺序断言测试即可确认 |
| A12 | 无 `rust-toolchain.toml` 时依赖 `rust-version = "1.98"` 约束；是否加 `rust-toolchain.toml` 钉死 1.98.0 | Recommended Project Structure | 本机 stable 即 1.98.0，加不加都能跑；加了更稳 |

## Open Questions

1. **LinkType 的 `apiName` 访问器语义与字段命名（A1）**
   - 已知：Foundry 把端 apiName 视作「在另一端上取回本端对象的访问器」；D-04 要求端 apiName 在所关联 object type 内唯一且不与属性同名。
   - 不清楚：作者是否更习惯 Foundry 的方向。
   - 建议：采用本文「写在所属 objectType 旁、访问器落在该类型上」的语义，并在 schema description 与示例里写明；这是 planner 可直接锁定的点，无需再问用户（用户已授权自主决策）。

2. **E09 与 V10 的边界（A5）**
   - 建议：按本文划分实现，在 PLAN 里写明并在两处测试里各留一条用例。

3. **`ONT-W17`（可选参数无 `has()` 保护）是否本 phase 做**
   - 已知：CONTEXT specifics 提到但不属于 D-12/D-13。
   - 建议：不做，记录到 backlog；本 phase 先把 V13 的引用与类型检查做扎实。如要做，放 `ontoforge-expr`，ID 用 W17。

4. **manifest 是否加 `formatVersion: 1`**
   - 一次性契约版本号成本极低，但 CONTEXT 未列。按 D-05 属向后兼容扩展，可晚加。建议先不加。

5. **CEL 白名单函数名检测**
   - 已知：`cel` 不提供函数名枚举，未知函数 compile 成功。建议本 phase 不做；Phase 3 求值时报错。

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| rustc / cargo | 全部 | ✓ | 1.98.0 / 1.98.0 | — |
| clippy / rustfmt | D-18 CI、本地门禁 | ✓ | 0.1.98 / 1.9.0 | — |
| rustup | 工具链管理 | ✓ | 1.29.0 | — |
| git（autocrlf=true） | `.gitattributes` 必要性 | ✓ | — | — |
| gh CLI | 查 action 版本（可选） | ✓ | — | — |
| cargo-insta | 交互审阅快照（可选） | ✗ | — | `INSTA_UPDATE=always cargo test` |
| cargo-nextest | 非必须 | ✗ | — | `cargo test` |
| Docker | 本 phase 不需要 | 未使用 | — | — |
| 网络（crates.io） | 首次 `cargo fetch` | ✓ | — | — |

Missing dependencies with no fallback: 无。Missing dependencies with fallback: cargo-insta、cargo-nextest（都可选）。

## Validation Architecture

`workflow.nyquist_validation` 为 `true`（`.planning/config.json`），本节生效。

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust 内置 `cargo test` + `insta` 1.49.0（快照）+ `jsonschema` 0.58.6（dev，schema 校验） |
| Config file | 无（Wave 0 新建 `Cargo.toml`、`.cargo/config.toml`；insta 默认读 `tests/snapshots`） |
| Quick run command | `cargo test -p ontoforge-schema` |
| Full suite command | `cargo test --workspace` |
| 门禁附加 | `cargo clippy --workspace --all-targets -- -D warnings`；`cargo fmt --all --check` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SCHEMA-01 | 多文件合并为单一 IR；文件顺序无关；YAML 与 JSON 得到相同 IR | integration（fixture） | `cargo test -p ontoforge-schema --test load_examples` | ❌ Wave 0 |
| SCHEMA-02 | object type 全部字段解析、默认值（titleKey=PK、hidden=false）、apiName 规则 | unit | `cargo test -p ontoforge-schema ir::` | ❌ Wave 0 |
| SCHEMA-03 | 13 个 v1 base type 解析；required；array/items；未知 base type 报 V06 | unit + snapshot | `cargo test -p ontoforge-schema basetype` | ❌ Wave 0 |
| SCHEMA-04 | 三种 cardinality 解析；FK 一致性 V11；端 apiName 唯一与属性同名 V08 | unit + invalid fixture | `cargo test -p ontoforge-schema links` | ❌ Wave 0 |
| SCHEMA-05 | parameters、五种 rule、ValueSource、criteria 解析；类型兼容 V12；CEL V13 | unit + `ontoforge-expr` 测试 | `cargo test -p ontoforge-schema actions && cargo test -p ontoforge-expr` | ❌ Wave 0 |
| SCHEMA-06 | 每个 invalid 样例的 `(file, line, column, code)` 快照；CLI 非 0 退出码 | snapshot + CLI | `cargo test -p ontoforge-schema --test invalid_examples && cargo test -p ontoforge --test cli` | ❌ Wave 0 |
| SCHEMA-07 | 导出 schema 与入库快照一致；aviation（YAML 与 JSON）通过 schema；invalid 样例不通过 | snapshot + jsonschema | `cargo test -p ontoforge-schema --test schema_export` | ❌ Wave 0 |
| LINT-01 | E08、E09 每条 trigger / clean fixture；有 E 级问题时 CLI 退出码 1 | snapshot + CLI | `cargo test -p ontoforge-schema lint::e` | ❌ Wave 0 |
| LINT-02 | W01–W11 每条 trigger / clean fixture | snapshot | `cargo test -p ontoforge-schema lint::w` | ❌ Wave 0 |
| LINT-03 | human 输出（无色快照）与 `--format json` 输出（schema 固定字段）；JSON 无 ANSI | CLI + snapshot | `cargo test -p ontoforge --test cli lint_formats` | ❌ Wave 0 |
| LINT-04 | `ontoforge.toml` off / info / warn / error 覆盖；未知规则 ID 报错；`--config` 覆盖；`--deny-warnings` | unit + CLI | `cargo test -p ontoforge-schema lint::config && cargo test -p ontoforge --test cli config` | ❌ Wave 0 |
| DX-03 | Windows 本机无 Docker `cargo test --workspace` 全绿；10 个 crate 存在且占位测试通过 | workspace smoke + CI | `cargo test --workspace` | ❌ Wave 0 |

附加必测（来自 Pitfalls）：`deeply_nested_expression_is_rejected_not_crashing`、`deeply_nested_yaml_is_rejected_not_crashing`、`json_trailing_comma_is_rejected`、`yaml_yes_no_stay_strings`、`diagnostic_paths_use_forward_slash`、`properties_keep_declared_order`（验证 A11）、`unknown_lint_rule_id_is_error`。

### Sampling Rate
- **每个 task 提交：** `cargo test -p <被改动的 crate>`（schema 约数秒；首次含 `cel` 编译较慢）
- **每个 wave 合并：** `cargo test --workspace` 与 `cargo clippy --workspace --all-targets -- -D warnings`
- **Phase 门禁：** 全套件绿 + `cargo fmt --all --check`，再 `/gsd-verify-work`

### Wave 0 Gaps
- [ ] 根 `Cargo.toml`、`rust-toolchain.toml`、`.cargo/config.toml`、`.gitattributes`、`.github/workflows/ci.yml`
- [ ] 10 个 crate 骨架与占位测试
- [ ] `ontoforge-schema` 测试辅助：`load_from_memory(files: &[(&str, &str)])` 与 `lint_fixture(rule, name)`（内联 fixture 也能走完整流水线，不依赖磁盘）
- [ ] `examples/ontologies/aviation/`、`aviation-json/`、`invalid/<case>/`（每个 V 码至少一例）
- [ ] 快照目录与 `INSTA_UPDATE` 使用约定写入 CONTRIBUTING 或 README 一行
- [ ] 无需安装新的测试框架（`cargo test` 内置）

## Security Domain

`security_enforcement` 为 `true`，`security_asvs_level = 1`。本 phase 无网络面、无认证，攻击面是「不可信的定义文件与配置」。

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | no | — |
| V5 Input Validation | yes | serde `deny_unknown_fields` + 校验型 newtype（apiName 正则与保留字）+ `serde-saphyr` Budget + CEL 长度/嵌套限制 |
| V6 Cryptography | no | — |
| V12 Files and Resources | yes | glob 拒绝 `..` 与绝对路径；`walkdir` 不跟随符号链接；单文件大小上限；`camino` 相对路径 |
| V14 Configuration | yes | `ontoforge.toml` 严格解析（`deny_unknown_fields`、未知规则 ID 报错） |

### Known Threat Patterns for 此栈

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| YAML alias bomb / 超大文档 / 文档风暴 | DoS | `serde-saphyr` `Budget`（默认 `max_aliases`、`max_nodes=250_000`、`max_documents=1024`），并设 `max_depth=32`、`merge_keys=Error`；另加单文件字节上限（建议 1 MiB，可配）`[ASSUMED]` |
| 深层嵌套导致栈溢出（YAML / CEL） | DoS | 大栈 worker 线程 + 深度/长度预检（Pitfall 1） |
| YAML tag 注入 / 非预期类型构造 | Tampering | `reject_unsupported_tags = true`；serde-saphyr 本身不支持 tag 驱动对象构造 `[CITED: serde-saphyr README "Safer by construction"]` |
| include glob 路径逃逸（`../**`）与符号链接 | Information disclosure | 拒绝 `..` 与绝对路径；不跟随符号链接；规范化后校验仍在根目录内 |
| 正则 ReDoS | DoS | lint 内固定正则，使用 `regex` crate 的线性时间保证 `[CITED: docs.rs/regex]`；CEL `matches` 不在本 phase 求值 |
| 错误信息泄露本机绝对路径 | Information disclosure | 诊断 `file` 一律相对 ontology 根 |
| 供应链（`cel` 的 antlr4rust 传递依赖、serde-saphyr 的 granit-parser） | Tampering | `Cargo.lock` 入库；CI 后续可加 `cargo audit`（本 phase 不做） |

## Sources

### Primary (HIGH confidence)
- 本机实跑 spike：`C:\tmp\ontoforge-spikes\{s1-yaml, s1b-saphyr, s2-cel, s3-schema, s4-ws, s5-toml}`（rustc 1.98.0，Windows 11）
- crate 源码（只读，作为不可信数据）：`serde-saphyr-2.0.0`（README、`src/spanned.rs`、`src/de/options.rs`、`src/de/budget.rs`、`src/de/path_map.rs`）、`cel-0.15.0`（`src/lib.rs`、`src/env.rs`、`src/parser/references.rs`、`src/common/ast/mod.rs`）、`jsonschema-0.58.6` Cargo.toml
- 仓库文档：`.planning/adr/0002,0003,0006,0011`、`.planning/research/{rust-stack.md, foundry-docs-spec.md}`、`.planning/REQUIREMENTS.md`、`01-CONTEXT.md`
- Foundry 文档镜像：`docs/foundry/ontology/object-link-types/{properties-overview, base-types, create-object-type, create-link-type}.md`、`docs/foundry/ontology/action-types/{rules, submission-criteria, parameter-overview, scale-property-limits}.md`
- crates.io API（2026-10-15）与 `gsd-tools query package-legitimacy check`
- docs.rs：serde_path_to_error、strsim、globset、walkdir、camino、toml、regex

### Secondary (MEDIUM confidence)
- yaml-language-server README（draft 支持范围）：github.com/redhat-developer/yaml-language-server（经 WebSearch 摘要）

### Tertiary (LOW confidence)
- 编辑器对 `oneOf + const` 补全收窄的实际效果（A9，未实测）；CI action 配置（A10）

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH，版本与合法性均查证，核心行为已 spike
- Architecture: HIGH，Node 树 + 路径回查 + kind 重写均端到端实跑；crate 依赖方向受 ADR-0011 约束已推导
- Lint 规则映射: MEDIUM，规则依据来自 spec 表，阈值与启发式为设计推荐
- Pitfalls: HIGH，栈溢出、JSON 宽松、strict_booleans 等均实测

**Research date:** 2026-10-15
**Valid until:** 2026-11-14（serde-saphyr 2.0.0 刚发布，7 天内应复查是否有 2.0.x 修复；其余稳定 30 天）
