# Phase 1: Workspace, Schema & Lint - Context

**Gathered:** 2026-10-15
**Status:** Ready for planning

<domain>
## Phase Boundary

交付三样东西：
- 按 ADR-0011 搭好的 cargo workspace；
- ontology-as-code 的定义格式（YAML / JSON），以及加载为单一 IR 的 loader；
- CLI 命令 `ontoforge validate`、`ontoforge lint` 和 JSON Schema 导出。

只做静态定义与静态检查。存储、action 执行、查询、服务进程都不在本 phase。本 phase 用到的 crate 是 `ontoforge-schema`、`ontoforge-expr`（只做 CEL 编译与引用检查）和 `ontoforge`（CLI）；其余 crate 只建骨架，能编译、有占位测试即可。

</domain>

<decisions>
## Implementation Decisions

本 phase 的灰色地带都是技术实现问题。按用户的「决策自主」偏好（自行调研决定，必须由用户拍板的才问），全部由 Claude 决定，依据写在每条后面。

### 定义文件布局
- **D-01:** 一个 ontology 对应一个目录，根目录放 manifest `ontology.yaml`。manifest 包含 `apiName`、`displayName`、`description`，以及 `include` glob 列表（默认 `**/*.yaml`、`**/*.yml`、`**/*.json`）。其余文件的位置和数量不做限制。理由：开发者可以按领域自行拆分文件，loader 只看 glob 结果，不依赖目录约定。
- **D-02:** 每个定义文档都是带 `kind` 判别字段的对象，`kind` 取值为 `ObjectType` / `LinkType` / `ActionType`（预留 `Interface`、`ValueType`、`SharedProperty`，loader 遇到时报「v1 不支持」）。一个 YAML 文件可以用 `---` 写多个文档；JSON 文件可以是单个对象，也可以是对象数组。理由：`kind` 判别让 serde internally-tagged enum 和 JSON Schema 的 `oneOf` 都能直接表达，LLM 生成时也不容易混淆。 — **Reversibility:** one-way — 这是对外的 ontology-as-code 契约，发布后改字段名或结构，所有用户的定义文件都要迁移。
- **D-03:** 属性内联在 object type 里（`properties:` 下以 apiName 为 key 的 map）；link type 单独成文档，两端都写在 link type 里。理由：与 Foundry 的模型一致（link type 是独立资源）；1:N 外键属性仍在 object type 里声明，link type 只引用它。

### 命名与校验规则（遵循 Foundry 文档 + ADR-0002）
- **D-04:**
  - object type 的 apiName 用 PascalCase：`^[A-Z][A-Za-z0-9]{0,99}$`，在 ontology 内唯一。
  - property 的 apiName 用 camelCase：`^[a-z][A-Za-z0-9]{0,99}$`，在 object type 内唯一。
  - link 两端的 apiName 用 camelCase，在所关联的 object type 范围内唯一，并且不能与该类型的属性同名。
  - action type 的 apiName 用 camelCase，在 ontology 内唯一；MCP 工具名在 Phase 7 由它派生为 kebab-case。
  - 依据：`docs/foundry/ontology/object-link-types/create-object-type.md`、`create-link-type.md`。保留关键字表由 research 补齐。
- **D-05:** 未知字段一律报错（serde `deny_unknown_fields`，诊断里附「你是否想写 X」建议）。理由：拼写错误静默忽略是 ontology-as-code 最常见的坑；以后新增字段属于向后兼容的扩展。
- **D-06:** 结构性错误属于 validate 范畴（hard error，不能关闭），例如解析失败、未知 base type、重复 apiName、引用不存在的类型或属性、主键类型非法、外键类型与目标主键不匹配。lint 的 ONT-* 规则在 IR 构建成功之后运行，可以配置。理由：IR 构建失败时 lint 无法运行，两层必须分开。

### Action type 定义形状
- **D-07:** 参数为 `parameters:`，以 apiName 为 key 的 map，字段有 `type`、`required`、`default`、`description`。type 取值为 base type，或 `objectReference: <ObjectType>`。
- **D-08:** rules 是有序列表，每条为 `{ kind: createObject | modifyObject | deleteObject | addLink | removeLink, ... }`。值来源用结构化写法，不用字符串模板：`{ param: x }` / `{ static: v }` / `{ currentUser: {} }` / `{ currentTime: {} }`。理由：结构化写法可以静态校验类型；字符串模板要再引入一种语法。 — **Reversibility:** one-way — 同属 ontology-as-code 契约。
- **D-09:** submission criteria 为列表，每条 `{ condition: <CEL 字符串>, failureMessage: <文本> }`。CEL 上下文变量固定为 `params`、`user`（`id`、`groups`）、`objects`（按参数名访问 objectReference 参数指向的对象当前属性）。本 phase 只做 CEL 编译、变量引用存在性检查、参数类型推断（取决于 spike 结论），不求值。

### 诊断与输出
- **D-10:** 诊断模型统一为 `{ code, severity, message, file, line, column, span, help }`。人类可读输出用 `miette` 的 fancy 渲染；`--format json` 输出该模型的 JSON 数组。validate 和 lint 共用同一模型。退出码：有 error 时为 1，否则为 0；`--deny-warnings` 让 warning 也返回 1。
- **D-11:** span 精度至少到产生问题的那个 key 或 value 节点。如果 spike 证明 `serde-saphyr` 无法按 IR 路径取 span，就退化为先解析成带 span 的 YAML 节点树，再反序列化为 IR，同时维护「IR 路径 → span」表。具体方案由 research / spike 决定，但「到节点级」这个精度要求不变。JSON 输入同样要求 line:col。

### Lint 规则范围与配置
- **D-12:** v1 schema 范围内适用的 E 级规则为 ONT-E08（action rule 组合约束）和 ONT-E09（base type 用途约束：主键 / title key / array 元素）。其余 E 级规则（派生属性、vector、scenario、value type、interface、security policy）因对应 schema 元素不在 v1，本 phase 不实现，规则 ID 保留。
- **D-13:** W01–W11 都实现，但只做静态部分。ONT-W06（God Object）只检查属性数量阈值和 `*Type` / `*Subtype` 判别列，空值率依赖数据，留到有存储之后。ONT-W07 的 Jaccard 比较只基于属性名和类型集合。
- **D-14:** lint 配置写在 ontology 根目录的 `ontoforge.toml`，`[lint]` 段中按规则 ID 设置 `"off" | "info" | "warn" | "error"`。CLI 的 `--config` 可以覆盖配置文件路径。理由：Rust 生态习惯用 TOML 做工具配置，也和 ontology 定义本身（YAML）区分开。
- **D-15:** 每条 lint 规则一个模块，每条规则配一组 fixture 测试（触发和不触发两种），用 `insta` 做快照。

### 示例 ontology
- **D-16:** 示例 ontology `examples/ontologies/aviation/` 沿用 Foundry 文档中的航空领域，包含 Airport、Aircraft、Flight、Pilot：
  - 1:N：Flight 的 departureAirport 外键；
  - 1:1：Aircraft 与当前维护记录，或类似关系；
  - M:N：Flight 与 Pilot 的机组分配；
  - 2–3 个 action type：assignPilot、delayFlight 等。
  - 同时提供一份等价的 JSON 版本，用于「YAML 与 JSON 得到同一 IR」的测试。
  - 另建 `examples/ontologies/invalid/` 存放刻意出错的样例，作为诊断测试的 fixture。
  - 理由：领域与 Foundry 文档一致，可以直接对照文档语义；三种基数都能覆盖。

### Workspace
- **D-17:** 10 个 crate 按 ADR-0011 一次性建好。共享依赖版本放在根 `Cargo.toml` 的 `[workspace.dependencies]`，lint 配置放在 `[workspace.lints]`（`unsafe_code = "forbid"`，clippy pedantic 选择性开启）。新增 `.gitattributes` 设置 `* text=auto eol=lf`，避免 Windows 上 CRLF 造成快照 diff。
- **D-18:** 本 phase 加 CI（GitHub Actions）：Windows 和 Ubuntu 两个平台跑 `cargo fmt --check`、`cargo clippy -D warnings`、`cargo test --workspace`。理由：DX-03 的回归门槛需要自动化守住，否则后续 phase 容易破坏 Windows 构建。

### Claude's Discretion
- crate 内部模块划分、IR 类型的具体 Rust 表示（Arc / interning 等）、错误枚举设计。
- `ontoforge` CLI 子命令的具体参数命名（遵循 clap 惯例）。
- JSON Schema 的导出方式：`ontoforge schema export` 子命令，并把生成产物入库做快照。

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### 已锁定决策
- `.planning/adr/0002-naming-convention.md`：标识符、apiName、错误码命名
- `.planning/adr/0003-ontology-definition-format.md`：YAML/JSON → 单一 IR，`serde-saphyr` + `miette` + `schemars`；本 phase 的 spike 结论写回这里
- `.planning/adr/0006-expression-language.md`：CEL（`cel` crate）
- `.planning/adr/0011-workspace-layout.md`：10 个 crate 与依赖方向

### 规格与规则
- `.planning/research/foundry-docs-spec.md` §1：object type / property / link type / action type 语义
- `.planning/research/foundry-docs-spec.md` §4：lint 规则表（ONT-E08、E09、W01–W11）
- `.planning/research/rust-stack.md` §1、§10、§11：定义格式选型、通用 crate 版本、spike 清单
- `docs/foundry/ontology/object-link-types/base-types.md`、`properties-overview.md`：base type 全表及可否作主键 / title key
- `docs/foundry/ontology/object-link-types/create-object-type.md`、`create-link-type.md`：apiName 规则
- `docs/foundry/ontology/action-types/rules.md`、`submission-criteria.md`、`parameter-overview.md`（若存在）：action rule 与 criteria 语义

### 设计参考（只借设计，不借命名）
- `.planning/research/ref-open-ontology.md`：IR 唯一契约、`model:check` 诊断设计
- `.planning/research/ref-open-foundry.md`：定义格式与校验流程

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- 无：greenfield 仓库，目前只有 `.planning/` 与 `docs/`（Foundry 文档镜像）。

### Established Patterns
- `.gitignore` 已忽略 `/target/`。
- 文档与规划都用中文，标识符用英文。

### Integration Points
- 本 phase 产出的 IR（`ontoforge-schema`）会被 Phase 2–7 的所有 crate 消费，它是唯一契约，后续 phase 不得另建 schema 表示。

</code_context>

<specifics>
## Specific Ideas

- `model:check` 风格（参考 open-ontology）：对「能编译但运行时会出错」的写法给 warning，例如 submission criteria 引用了可选参数却没做空值判断。
- 诊断文案要给出修复建议（`help` 字段），比如 apiName 大小写错误时直接给出正确写法。

</specifics>

<deferred>
## Deferred Ideas

- ONT-W06 基于空值率的检测：需要存储层数据，Phase 2 之后再加。
- 编辑器集成（LSP / VS Code 插件）：JSON Schema 已覆盖基本补全，LSP 属于新能力，记入 backlog。
- `ontoforge init` 脚手架命令：交给 DX 相关 phase 评估。

</deferred>

---

*Phase: 01-workspace-schema-lint*
*Context gathered: 2026-10-15*
