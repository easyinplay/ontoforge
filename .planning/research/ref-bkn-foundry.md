# 参考项目调研：openbkn-ai/bkn-foundry

> 调研日期：2026-10-15。方式：`gh api` 只读拉取 metadata / tree / 文件内容，未 clone、未执行任何代码。
> 仓库：https://github.com/openbkn-ai/bkn-foundry （default branch `main`，tree 5781 项，未截断）

## 1. 概况与成熟度

| 项 | 值 |
|---|---|
| 定位 | "Ontology back-end foundation of OpenBKN"：把本体驱动的业务语义变成运行时服务（data / logic / action / 安全治理 / 可观测）。**只有后端**，无 Web UI（UI 在独立 repo `bkn-studio`，CLI/TS SDK 在 `bkn-sdk`） |
| 主题 | Agent 的 Context Engineering + Harness Engineering（给 LLM agent 供上下文、约束执行），而非传统 BI/应用开发平台 |
| Stars / Forks | 699 / 57；open issues 156；contributors 36 |
| Commits | 约 1709（2026-05-26 创建，2026-10-15 仍在日更，例如 `refactor(object-metrics): retire legacy APIs and standardize V1 (#2185)`） |
| Release | v0.1.3 (2026-08-10) → v0.1.4 (08-31) → v0.1.5 (09-28)，仍是 0.x |
| License | **混合许可**（`LICENSE`）：上游 `kweaver-ai/kweaver-core` 派生文件（BKN Engine、Context Loader、VEGA、Exec Factory）= Apache-2.0；OpenBKN 新增文件（BKN Safe、SDK、Data Migrator 等）= `LICENSE-OPENBKN.txt`（Apache-2.0 + Additional Conditions，限制 "Restricted Hosted Service"、商业 entitlement）。逐文件 header 判定 |
| 血统 | 源自 kweaver-ai/kweaver-core（文件头 `Copyright The kweaver.ai Authors.`），概念上沿袭国产"业务知识网络"路线 |

结论：活跃、工程量大（Go ~24.8MB 源码），但 0.x、许可复杂；**借鉴设计可以，直接抄 OpenBKN-License 文件有风险**（Apache 部分可参考）。

## 2. 技术栈

- 语言：Go 1.25（主体，各服务独立 `go.mod`）；Python（`data-migrator`、`infra/sandbox`、示例 toolbox）；少量 TS；ANTLR（Cypher 语法）。
- 元数据库：MariaDB/MySQL（亦支持 DM8/KDB9 国产库，经 `openbkn-rds` driver）。迁移脚本 `migrations/<service>/mariadb/<ver>/init.sql`。
- 搜索/向量：OpenSearch（概念搜索 + 对象索引 + kNN）。
- 消息：Kafka（`vega/kafka-connect`，outbox 模式表 `t_kn_proxy_sync_outbox`、`migrations/*-trace-outbox`）。
- 身份：ORY Hydra（OAuth2）+ 自研 `bkn-safe`（Casbin RBAC，GORM adapter）。
- 观测：OpenTelemetry（`comm-go/otel`、`bkn-trace/otelcol-contribute-chart`）。
- 部署：Kubernetes + Helm（每服务 `helm/`），一键脚本在独立 repo `openbkn-deploy`。
- 架构风格：六边形（`interfaces/` ports、`driveradapters/` HTTP handler、`drivenadapters/` 外部依赖、`logics/` 业务），`interfaces/mock/` 自动生成 mock。

## 3. Repo 布局（顶层）

```
adp/bkn/bkn-backend        本体定义（schema）CRUD、BKN Lang 导入导出、Cypher→SQL、概念搜索
adp/bkn/ontology-query     运行时：对象实例查询、子图、逻辑属性求值、Action 执行/调度/日志
adp/context-loader/agent-retrieval   面向 Agent 的 MCP server（召回→粗排→精排）
adp/execution-factory/operator-integration  工具箱 / MCP / Skill / Python 函数的注册与执行
vega/vega-backend          数据虚拟化：Catalog / Resource / Connector / Dataset / 发现与构建任务
bkn-safe                   认证（Hydra provider）+ 鉴权（Casbin）+ 用户目录 + 行过滤策略
bkn-trace                  证据链 / agent observability
comm-go                    共享库：audit、authz、propertyaccess、projectiongrant、mq、otel、rest
infra/                     bkn-agent、model-factory（LLM/embedding 管理）、sandbox、oss-gateway
docs/api/<service>/*.yaml  OpenAPI 3.1 契约；rules/ARCHITECTURE.md 架构铁律
```

架构铁律（`rules/ARCHITECTURE.md`）：Foundry 不含 UI / BFF；Public API 只能向后兼容（URL major `/api/v1`→`/api/v2`，同 major 只加可选字段 / 扩枚举）；API 变更需 ADR + OpenAPI diff + contract test；**Foundry 后端微服务预算 < 5**，CI 计数。

## 4. 核心领域模型（`adp/bkn/bkn-backend/server/interfaces/*.go`）

顶层容器是 **Knowledge Network（KN，≈ Foundry Ontology）**，所有子概念主键为 `(kn_id, branch, id)`。

### 4.1 KN（`knowledge_network.go`）
`KN{id, name, CommonInfo(tags/comment/icon/color/detail...), branch, skill_content, concept_groups[], object_types[], relation_types[], action_types[], risk_types[], creator/updater/时间, module_type, statistics, operations[], _vector, _score}`
- `skill_content`：KN 自带一份 Agent Skill 文本（示例目录下有 `SKILL.md`），即本体直接生成给 LLM 的使用说明。
- `KNReadAccessMode`: `full` / `navigation_only`（只能看结构不能看数据）。
- 遍历方向常量：`forward / backward / bidirectional`；`OverviewGraphQuery{NodeLimit, EdgeLimit, FocusObjectTypeID, ExpandDepth, ConceptGroupID}`。

### 4.2 ObjectType（`object_type.go`）
- 关键字段：`id, name, data_source(ResourceInfo), data_properties[], logic_properties[], primary_keys[], display_key, incremental_key, concept_groups[], status, index_status`。
- `DataProperty{name, display_name, type, comment, mapped_field{name,type,display_name}, mask_rule, condition_operations[], index_features[]}`；上限 `MAX_PROPERTY_NUM = 1000`。
- 类型系统（`data_type`）：integer / unsigned_integer / string / keyword / float / decimal / text / date / timestamp / time / datetime / boolean / binary / json / **vector / point / shape / ip**。主键只允许 integer/unsigned/string/text。
- **LogicProperty**（≈ Foundry derived property / function-backed property）：`type ∈ {metric, tool}`，`data_source`、`parameters[]`、`analysis_dimensions[]`、`metric_binding`。参数 `Parameter{name, type, source, operation, value_from(property|input|const), value, required, default, if_system_generate}`，即"把对象属性绑定为工具/指标参数"。
- `ObjectTypeStatus{incremental_key, incremental_value, index, doc_count, storage_size}`：索引物化状态；`index_status.state ∈ available/unavailable/unknown/resource_missing/not_applicable`。
- `mask_rule`（`common/maskrule/rule.go`）：`kind ∈ fixed / partial / email / round / date_granularity`，带 `keep_start/keep_end/preserve_domain/step/granularity`，属性级脱敏。

### 4.3 RelationType（≈ Link Type，`relation_type.go`）
`{id, name, source_object_type_id, target_object_type_id, type, mapping_rules}`，三种 `type`：
- `direct`：`[]Mapping{source_property, target_property}`（外键等值连接）。
- `indirect`：`InDirectMapping{backing_data_source, source_mapping_rules[], target_mapping_rules[]}`（中间表，≈ Foundry many-to-many link 的 join table）。
- `filtered_cross_join`：`{source_condition, target_condition}`（条件笛卡尔，非键关联）。
- 无显式基数字段（基数只写在 BKN 文档"业务语义"散文里）。

### 4.4 ActionType（`action_type.go`）
`{id, name, action_type ∈ add/modify/delete, action_intent, object_type_id, condition(ActionCondCfg 递归树), affect{object_type_id, expected_operation, affected_fields[]}, impact_contracts[]{object_type_id, expected_operation, description, affected_fields[]}, action_source{type ∈ tool/mcp, box_id, tool_id, mcp_id, tool_name}, parameters[], schedule{type, expression}}`
- 条件算子 30 个：and/or/==/!=/>/>=/</<=/in/not_in/empty/range/between/like/prefix/regex/contain/exist/null/current 等（`ActionCondOperationMap`）。
- 关键差异：**Action 不直接改本体数据**，而是绑定外部 tool / MCP 执行；`affect` / `impact_contracts` 只是声明式"影响契约"（用于审计、风险与 agent 理解），平台不做 Foundry 式 ontology edit + writeback。
- `schedule`（如 `FIX_RATE 5m`）+ `condition` 可让 action 定时对满足条件的对象自动触发（`action_schedule*.go`）。

### 4.5 其他概念
- **RiskType**（`risk_type.go` + `.bkn`）：结构体只有 id/name/CommonInfo，正文是 Markdown 段落（Control Scope / Control Policy / Pre-checks 表 / Risk Mitigation / Rollback Plan / Audit Requirements）；执行时由内置工具 `bkn_common_risk_assessment_tool` 做评估。即"风险=半结构化规则 + LLM/工具评估"。
- **ConceptGroup**（`concept_group.go`）：对 object/relation/action 分组（≈ Foundry Ontology 的 group / 子域），`t_concept_group_relation(concept_type, concept_id)`。
- **CapabilityBinding**（`capability_binding.go`）：KN 挂载能力 `capability_type ∈ skill / function / mcp_tool (/api)`，`sources.kind ∈ manual/box/object_type/action_type`，导入报告 `CapabilityImportReport{bound, skipped[]}`。
- **ObjectMetric V1**（`object_metric_v1.go`，表 `t_object_metric_definition`，主键含 `f_version`）：版本化指标 `{code, owner_object_type_id, metric_type, calculation_scope, result_semantics, specification, query_capabilities, lifecycle}`，可暴露为对象的 logic property；有 validate / preview（`output_schema{shape, dimensions, value, time_key}`）。
- **没有 Interface（多态/共享 schema）、没有 shared property、没有 struct/array 复合属性类型、没有 object-type 继承**。

## 5. 本体定义的表达方式：BKN Lang（Markdown DSL）

- 目录式网络：`network.bkn` + `object_types/*.bkn` + `relation_types/*.bkn` + `action_types/*.bkn` + `risk_types/*.bkn` + `concept_groups/*.bkn` + `object_metrics/<id>@<version>.json` + `CHECKSUM` + `SKILL.md`。示例：`adp/bkn/bkn-backend/server/bkn-specification/examples/{k8s-network,exchange-recovery,supplychain-hd,mock_system}`。
- 单文件 = YAML frontmatter（`type: object_type`, `id`, `name`, `tags`）+ 约定标题与 Markdown 表格，例如 `object_types/pod.bkn`：
  ```
  ### Data Source        | Type | ID | Name |  → resource d2mio43q6gt6p380dis0 pod_info_view
  ### Data Properties    | Name | Display Name | Type | Description | Mapped Field |
  ### Keys               Primary Keys: id / Display Key: pod_name / Incremental Key:
  ```
  Action 的触发条件嵌 ```yaml condition:``` 代码块；参数绑定表 `| 参数 | 来源(property/const/input) | 绑定 |`。
- Go SDK（`bkn-specification/bkn/`）：`parser.go`（逐类型 `Parse*File`）、`serialize.go`（模型→文本，双向往返）、`loader.go`（目录 / tar / 内存 FS）、`validator.go`、`checksum.go`、`differ.go`/`networkdiff.go`（create/update/delete 差异）、`cmd/validate-dir`。
- 后端落库时保存原文：`t_object_type.f_bkn_raw_content MEDIUMTEXT`，结构化字段（`f_data_properties` 等）存 JSON 列。
- REST 侧对应：`POST /api/bkn-backend/v1/bkns`（导入 tar）、`GET /bkns/{kn_id}`（导出）、`POST /bkns/diff`、`POST /bkns/diff/object-data-stats`（变更影响预估）。
- 设计取向：给业务专家 + LLM 写，Git diff 友好，一定义一文件；代价是语法靠"标题+表格"约定，格式脆弱、无类型检查器级 IDE 支持。

## 6. 存储与查询引擎

- **本体对象不落自有存储**：ObjectType 通过 `data_source` 绑定 VEGA 的 Resource；`ontology-query` 实时把对象查询下推到 `vega-backend`（`logics/object_type/object_type_service.go` `getObjectsFromResource` / `queryResourcePage`）。≈ Foundry 的 object storage 被换成"虚拟化视图 + 可选 OpenSearch 物化索引"。
- VEGA（`vega/vega-backend`，README "Virtual First / OpenSearch Native / Dataset API / Unified DSL"）：Catalog → Resource（table / index / fileset / dataset / logic_view）；connector 分 local（`logics/connectors/local/{table,index,fileset}`）与 remote HTTP connector；有 discover / build / semantic_understanding / catalog health-check 任务。分页：index-backed 读有 `vegaIndexResultWindow = 10000`，超出改 cursor session。
- **Cypher 查询**（`logics/cypher/`，`POST /knowledge-networks/{kn_id}/cypher-queries`）：openCypher M19 全语法 ANTLR 解析（`parser/Cypher.g4`，生成代码已提交），再在 AST 层收窄为只读子集（拒绝 CREATE/WITH/UNION/UNWIND/procedure/path variable，OPTIONAL MATCH 仅限单关系）。管线 parse → analyze → binder（绑定到本体 schema）→ plan（`Plan{Tables, Joins, Where, GroupBy, Select, OrderBy, Limit, Properties}`）→ `sqlgen.go` 输出 MySQL 方言 SQL（Postgres 已生成未启用），表名以 `{{.resource_id}}` 占位由 VEGA 替换，`DefaultLimit` 防无界扫描；`Plan.Properties` 用于执行前做属性级鉴权；`row_filter.go` 注入行过滤。
- 子图：`POST /ontology-query/v1/knowledge-networks/{kn_id}/subgraph`、`/subgraph/objects`；`filtered_cross_join_expand.go` 处理条件关联展开。
- 概念搜索：KN/OT/RT/AT 元数据写入 OpenSearch（`_vector`/`_score` 字段），字段集 `CONCPET_QUERY_FIELD_STR`，供 agent 语义找 schema。

## 7. API 面

- REST（OpenAPI 3.1，`docs/api/`）：
  - `bkn-backend`：`/api/bkn-backend/v1/knowledge-networks[/{kn_id}]`、`/object-types`、`/relation-types`、`/relation-type-paths`、`/action-types`、`/risk-types`、`/concept-groups`、`/capabilities`、`/action-schedules`、`/cypher-queries`、`/overview-graph`、`/validation`；对象级 `PUT .../object-types/{ob_id}/data_properties/{property_names}`；内部面 `/api/bkn-backend/in/v1/...`（Public / Internal 分层）。
  - `ontology-query`：`.../object-types/{ot_id}`（POST 查询实例）、`/schema`、`/sample-data`、`/properties`（逻辑属性求值）、`.../action-types/{at_id}/execute`、`/action-executions/{id}`、`/action-logs[/{id}/results|/cancel]`。
- **MCP**（`adp/context-loader/agent-retrieval/server/driveradapters/mcp/schemas/`）：`list_knowledge_networks`、`get_kn_detail`、`get_object_types`、`get_relation_types`、`explore_subgraph`（`path_length` 1-3、`direction`）、`get_logic_properties_values`、`get_action_info`、`execute_action`、`get_action_execution`、`list_action_executions`、`run_cypher`、`list_skills`/`get_skill_content`/`execute_skill`、`execute_tool`、`describe_resource`/`list_resources`；输出支持 `response_format: json | toon`（默认 TOON，省 token）；schema 描述做 zh-CN/en-US 双语本地化，描述里写满"先调 X 再调 Y、不可凭名称猜 ID"之类的 agent 指引。
- SDK/CLI：独立 repo `openbkn-ai/bkn-sdk`（`npm i -g @openbkn/bkn-sdk`，命令 `openbkn bkn list`）；Skill 契约（`rules/ARCHITECTURE.md` A.3）：`name/version/stability/auth.scopes/audit/io.input_schema/output_schema`，同样受向后兼容约束。
- 无 GraphQL；无类型生成式 OSDK（Foundry 的强类型 per-ontology SDK 不存在）。

## 8. 权限模型（`docs/api/knowledge-network-authorization.md`、`bkn-safe/`）

- 资源类型 7 种：`knowledge_network / concept_group / object_type / relation_type / action_type / metric / risk_type`；子资源 ID = `{kn_id}/{business_id}`；**鉴权永远评估已发布的 `main` 分支模型**。
- 操作：`view_detail / query_data / modify / delete / authorize / task_manage / execute`；子资源一跳继承父 KN（`delete`→父 `modify`），**`action_type.execute` 不继承**，必须对具体 action 直接授权，`action_type:*:execute` 非法。
- 执行 action 需同时满足：KN `execute` + action-type `execute` + 执行资源（tool/MCP）`execute` + 所有数据依赖 `query_data`。执行记录保存认证主体，worker 在外调前对同一主体复检；自动重试沿用原主体，手动重跑为新主体。
- 列表先构造可信候选集 → 批量过滤（`POST /api/safe/v1/authz/resource-filter`）→ 再算 total / 分页，防止计数泄露；chunk 失败整体失败，不返回部分结果；fail-closed。
- 行级：row-filter policy（`bkn-safe/server/internal/rowfilter`，管理 API `docs/api/bkn-safe-admin/row-filter-policies.yaml`），查询服务取调用者的固定条件谓词下推到数据查询（分页前）。
- 属性级：`property-grants.yaml`、`authz/property-levels`、`comm-go/propertyaccess` + 属性 `mask_rule` 脱敏。
- 引擎：Casbin（RBAC + 资源实例 `keyMatch`，显式 deny 优先，`super_admin` 不可 deny），policy 存库；Hydra 负责 token，bkn-safe 做 login/consent provider；另有 managed proxy account（KN 代理身份，供定时 action 等非人主体使用，`t_kn_proxy_account` + grant source ledger + outbox 同步）。
- 审计：`t_operation_audit`、`comm-go/audit`、`bkn-safe/internal/decisionlog`。

## 9. 值得借鉴的设计决策

1. **schema / runtime 拆两服务**：`bkn-backend`（定义、版本、导入导出、Cypher 编译）与 `ontology-query`（实例查询、action 执行）分离，读路径可独立扩缩。
2. **Branch 内建在主键**：所有定义表 `PRIMARY KEY (f_kn_id, f_branch, f_id)`，`main` 为发布态，配 `bkns/diff` 实现"改本体像提 PR"（≈ Foundry Ontology branching / proposals）。
3. **Ontology-as-code 文本格式 + checksum + diff + 往返序列化**，tar 打包分发；同一格式既给人看也给 LLM 读，并附 `SKILL.md`。
4. **Cypher 子集编译到 SQL 下推**：全语法解析、语义层拒绝并给出"不支持原因+位置"，planner 只产出已绑定 schema 的名字，SQL 生成是唯一拼文本处，表名占位符防注入；`Plan.Properties` 作为执行前属性鉴权清单。对 Rust 实现可映射为 `pest/lalrpop` 或 `open-cypher` crate + 自写 binder/planner + 下推到 DataFusion / 目标库。
5. **Logic property = 参数绑定的 metric / tool**：对象属性可声明为"调用某函数，参数来自 property / input / const"，统一了 derived property 与 function-backed property。
6. **Action 的声明式影响契约**（`affect`, `impact_contracts`, `expected_operation`, `affected_fields`）+ 风险类型 + 前置条件，执行前可做影响预估与风险拦截；执行主体复检、action 不继承执行权限，安全模型扎实。
7. **Agent-first API**：MCP 工具粒度贴合"先发现 schema → 探索子图 → 查实例 → 执行 action"的 agent 循环；TOON 输出省 token；工具描述本地化。
8. **列表鉴权"先过滤后分页"、fail-closed、批量一致性**等不变量写成独立契约文档，跨服务共享。
9. 工程纪律：API 向后兼容规则、服务数预算、OpenAPI breaking 检测、contract test。

## 10. 缺失 / 薄弱点（FoundryOntology 的差异化空间）

- **无原生对象写入 / edits / writeback**：action 只转发到外部 tool/MCP，平台不维护 ontology edit 日志、乐观锁、edit → 数据源回写，也就没有 Foundry 式"本体即可写的操作层"。
- **无 Interface / 共享属性 / 继承 / 复合（struct、array）属性**；关系无基数、无 link 属性；类型系统是扁平标量 + vector/geo。
- **无强类型 OSDK / 代码生成**、无 GraphQL；Function（TS/Python 函数绑定对象）只是 execution-factory 里的外部工具，没有与对象类型的类型化签名。
- **无对象级物化存储与一致性语义**：依赖 VEGA 实时下推 + OpenSearch 索引（10000 窗口限制、cursor 会话），跨源 join 依赖 VEGA SQL 翻译，性能与一致性取决于底层源。
- Cypher 只读且子集很窄（无 WITH / 聚合管线受限 / 无变长路径变量），子图最多 3 跳。
- BKN Lang 以 Markdown 表格承载结构，解析脆弱、缺 JSON Schema / LSP；RiskType 规则大部分是散文，可执行性依赖 LLM 工具。
- 运维重量级：Hydra + MariaDB + OpenSearch + Kafka + K8s/Helm，多服务（虽设预算 <5，但实际 adp / vega / safe / trace / infra 远超）；单机开发体验差（README 要求 Linux 全量安装）。
- 0.x 版本、API 仍在退役重构（如 object metrics legacy API 刚移除），许可证对托管服务有限制。

## 11. 对 FoundryOntology（Rust）的落地建议

- 直接借鉴：KN/branch 主键设计、ObjectType/RelationType 三种 mapping、ActionType 的 condition 树 + affect/impact contract、Logic property 参数绑定模型、7 类资源 + 操作继承表 + "execute 不继承"、先过滤后分页的鉴权不变量、MCP 工具集划分与 TOON 输出。
- 改进方向：用 YAML/TOML/JSON-Schema 化的定义格式（或 Rust DSL）替代 Markdown 表格，保留"一定义一文件 + checksum + diff"；补 Interface、struct/array 属性、link 基数与属性；做原生 edits + writeback 与强类型 SDK 生成；查询层用 DataFusion 统一下推，Cypher 子集编译到 DataFusion LogicalPlan 而不是拼 SQL 字符串；部署以单二进制 + 可选外部组件为目标。
