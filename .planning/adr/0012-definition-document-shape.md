# ADR-0012 ontology-as-code 定义文档形状

- status: accepted
- date: 2026-10-15
- decider: user（Phase 1 plan 01-01 的 checkpoint:decision，回复 `recommended`）
- 依据：`.planning/phases/01-workspace-schema-lint/01-CONTEXT.md` D-02、D-03、D-07、D-08、D-09；`01-RESEARCH.md` 的 Definition Format Proposal 与 Assumption A1
- 相关：ADR-0002（命名）、ADR-0003（定义格式与解析方案）、ADR-0011（workspace 布局）

## Context
ADR-0003 决定了「YAML / JSON 解析到单一 IR」，但没有定死文档的字段级形状。D-02、D-08 与 RESEARCH A1（LinkType 两端字段与端 apiName 的语义）都是对外契约，发布后修改意味着所有用户的定义文件迁移（one-way）。按用户「决策自主」偏好，one-way 决策在落地前经用户确认一次。

## Decision
用户选择选项 `recommended`：整体接受 RESEARCH 推荐形状，包括 LinkType 端 apiName 的语义 "an end's apiName is the accessor on THAT end's objectType"。用户明确在该语义与 Foundry 原文措辞方向（选项 `foundry-link-direction`）之间选择了前者。未采用的选项：`foundry-link-direction`、`grouped-documents`、`tagged-value-source`。

### 文档与 manifest（D-01、D-02）
- 一个 ontology 一个目录，根目录固定文件名 `ontology.yaml` 为 manifest，无 `kind`，字段 `apiName`、`displayName`、`description`、`include`，未知字段报错。
- 其余文档顶层带 `kind: ObjectType | LinkType | ActionType`。YAML 用 `---` 写多文档；JSON 文件可为单个对象或对象数组。
- `Interface` / `ValueType` / `SharedProperty` 预留，loader 报 `ONT-V03`（v1 不支持）。
- 所有 struct 带 `deny_unknown_fields`，字段名 camelCase（D-05）。
- loader 内部把 `{kind: X, ...}` 重写为 externally tagged 的 `{X: {...}}` 再反序列化，以保留 `serde_path_to_error` 路径与节点级 span（Spike S1）；`#[serde(tag = "kind")]` 只用于 JSON Schema 导出。

### ObjectType（D-03）
`apiName`（PascalCase）、`displayName`、`description`、`aliases`、`hidden`、`primaryKey`、`titleKey`、`properties`。`properties` 是以属性 apiName（camelCase）为 key 的 map，每项 `type`、`items`、`required`、`displayName`、`description`。`titleKey` 缺省等于 `primaryKey`；主键属性隐含 `required: true`。Walking Skeleton 阶段（plan 01-01）只实现 `apiName`、`displayName`、`description`、`primaryKey`、`properties`（`type`、`required`），其余字段由 plan 01-04 补齐。

### LinkType（RESEARCH A1）
`apiName`（camelCase，link type 之间唯一）、`cardinality: oneToOne | oneToMany | manyToMany`、`from` / `to` 两端 `{ objectType, apiName, displayName, description, foreignKey }`。

端 apiName 语义：**一个端的 apiName 是「该端 `objectType` 上到达另一端的访问器名」**。例如 `from: { objectType: Airport, apiName: departingFlights }` 表示 `Airport` 上的 `departingFlights` 返回 `Flight`。这与 Foundry 文档措辞方向相反（Foundry 中写在某端的名字是对端类型上的访问器），是有意差异，必须在 JSON Schema description 与示例注释中写明。好处：D-04「端 apiName 在所关联 object type 内唯一且不与属性同名」可直接在 `objectType` 命名空间内检查。

`foreignKey` 只出现在 `oneToOne` / `oneToMany` 的 `to` 端，类型必须等于 `from` 端主键类型；`manyToMany` 不得出现。

### ActionType（D-07、D-08、D-09）
- `parameters`：以参数 apiName 为 key 的 map，`type` 为 base type 或 `{ objectReference: <ObjectType> }`，另有 `items`、`required`、`default`、`description`。
- `rules`：有序列表，每项 `kind: createObject | modifyObject | deleteObject | addLink | removeLink`。
- 值来源（D-08）：单键结构化 map `{ param: x }` / `{ static: v }` / `{ currentUser: {} }` / `{ currentTime: {} }`，不用字符串模板，不用 `{ source: ..., name: ... }` 式显式判别写法。
- `submissionCriteria`：列表，每项 `{ condition: <CEL 字符串>, failureMessage: <文本> }`；CEL 上下文变量固定为 `params`、`user`、`objects`。

## Consequences
- plan 01-03 至 01-12 全部按此形状实现；示例、JSON Schema 快照、invalid 样例与诊断快照固化该形状。
- 后续新增字段（`pluralDisplayName`、`editsOnlyViaActions`、`agentDescription`、`formatVersion` 等）属于 D-05 允许的向后兼容扩展，不改变既有字段。
- 若以后要改端 apiName 方向或值来源写法，需要新 ADR（superseding）并提供迁移工具；本 ADR 不可变。
