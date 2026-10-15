# FoundryOntology 能力规格（从 Palantir Foundry 本地文档镜像蒸馏）

- 来源：`docs/foundry/ontology/*.md`（23 篇全读）+ `docs/foundry/dev-toolchain/{overview.md, ontology-sdk/, ontology-mcp/, global-branching/}`（略读），附带 `superrepo/tutorial-develop-with-a-superrepo.md`（Ontology-as-code 示例）；2026-10-15 补抓参考页 118 篇（`docs/foundry/ontology/{object-link-types,action-types,functions,interfaces,object-permissioning}/`）。
- 文档抓取日期 2026-10-08（参考页 2026-10-15）；本文生成 2026-10-15，同日按参考页修订。
- 引用约定：`[文件名]` 指 `docs/foundry/ontology/` 下文件；`[osdk/x]` = `dev-toolchain/ontology-sdk/x`，`[mcp/x]` = `dev-toolchain/ontology-mcp/x`，`[gb/x]` = `dev-toolchain/global-branching/x`；补抓参考页：`[olt/x]` = `ontology/object-link-types/x`，`[at/x]` = `ontology/action-types/x`，`[fn/x]` = `ontology/functions/x`，`[if/x]` = `ontology/interfaces/x`，`[op/x]` = `ontology/object-permissioning/x`。
- **缺口状态**：首版缺的参考页已补抓（站点无独立 `value-types/` 节，value type 页位于 `object-link-types/`）。原 `[推断]` / `[GAP]` 已按原文改写，剩余未决项见文末「下一步阻塞项」。

---

## 1. 核心概念与精确语义

### 1.1 总体映射 [core-concepts.md]
| Dataset | Ontology |
|---|---|
| Dataset | Object type |
| Row | Object |
| Column | Property |
| Field | Property value |
| Join | Link type |

- Ontology = semantic 元素（objects / properties / links）+ kinetic 元素（actions / functions / dynamic security）[overview.md]。
- 决策四要素：Data / Logic / Action / Security；Ontology 记录 "decision lineage"（何时、基于哪版数据、经由哪个应用做出决策）[why-ontology.md]。对实现的含义：**每次 action 应用都要落审计事件**。
- Roles 是中央权限模型，可授予在 Ontology 级或单个资源级 [core-concepts.md]。

### 1.2 Object type / Object / Object set [core-concepts.md]
- Object type：真实世界实体或事件的 schema；Object：单个实例；Object set：多个实例的集合。
- 每个 object 有 `$primaryKey`、`$apiName`/`$objectType`、`$title`、`$rid`（可选加载）[osdk/typescript-osdk.md, osdk/typescript-osdk-testing.md]。
- **Primary key**：唯一标识，派生属性不能做 PK [derived-properties.md]；PK 值不可变（`$clone` 改 PK 会报错）[osdk/typescript-osdk-testing.md]。PK 可挂 value type（例：Scenario Reference）[persisted-scenario.md]。
- **Title key**：指定一个属性作为 `$title` 展示值（示例 `restaurantName (title)`）[osdk/typescript-osdk.md]。
- 可配置 "Only allow edits via actions"（禁止绕过 action 直接改）[persisted-scenario.md]。
- 可标记 hidden（非语义的技术类型隐藏于默认视图）[ontology-best-practices.md]。
- 需支持 description、aliases（找不到起点类型的首因之一是缺 alias）[ontology-design-validation.md]。
- 存储后端差异：OSv1 有 10,000 结果上限（`ObjectsExceededLimit`），OSv2 无上限 [osdk/typescript-osdk.md]。

### 1.3 Property 与 base types
- Property = 对象特征的 schema；property value = 某对象上的值 [core-concepts.md]。
- **Base types 完整清单**（原文定义）[olt/properties-overview.md, olt/base-types.md]：除 `Map`、`Binary` 外所有 dataset field type 均可作 base type。

| Base type | 可作 title key | 可作 PK | 备注 |
|---|---|---|---|
| `String` `Integer` `Short` | 是 | 是 | 推荐 PK 用 `String` |
| `Date` `Timestamp` | 是 | 不推荐 | 存储格式与展示格式不同，易碰撞 |
| `Boolean` `Byte` `Long` | 是 | 不推荐 | `Byte` 只能经 `Integer` 参数赋值；`Long` >1e15 在 JS 失精度 |
| `Float` `Double` `Decimal` | 是 | 否 | |
| `Vector` | 否 | 否 | 不可放进 array |
| `Array` | 内层类型可作 title 时才可 | 否 | 不得含 null 元素；OSv2 不支持嵌套数组 |
| `Struct` | 否 | 否 | 不可嵌套、字段不可为 array，至少 1 个字段；可有 struct array [olt/structs-overview.md] |
| `Media Reference` `Time Series` `Geotemporal Series` `Attachment` | 否 | 否 | `Time series` 不可放进 array |
| `Geopoint` | 是 | 否 | 存为 `"lat,lon"` 字符串 |
| `Geoshape` `Marking` | 否 | 否 | |
| `Cipher`（cipher text） | 是 | 否 | |

- 函数侧类型映射（TSv1 / TSv2 / Python）见 [fn/types-reference.md]：标量、`List/Set/Map/Optional/Struct`、`Range`、二/三维聚合、`Object/ObjectSet/Interface/InterfaceObjectSet/OntologyEdit`、`Attachment`、`Notification`、`Media`、`User/Group/Principal`、`GeoPoint/GeoShape`（GeoJSON，经度在前）。
- **Required**：仅 OSv2；索引时校验（数据源含 null 时 schema 修改仍成功，但索引失败），action 写 null / 空数组时 apply 失败；array required = 至少 1 个元素 [olt/required-properties.md]。**Edit-only property**：不映射 backing 列，只经编辑写入，权限挂靠某个 backing dataset [olt/edit-only-properties.md]。
- 属性级配置：required（派生属性不可 required）、value formatting / conditional formatting、inline actions、visibility（隐藏边缘属性）、"Enable regex queries" 索引开关 [derived-properties.md, search-syntax.md, ontology-anti-patterns.md]。
- **Property reducer**：从数组/struct 数组里选一个代表值用于展示和 interface 实现；定义了 reducer 时 collect 聚合按它排序；reducer 不改变查询所用的完整数组 [derived-properties-reference.md, ontology-structural-guidance.md]。
- 时间序列：高频变化值用 time series property，不要建多版本对象 [ontology-anti-patterns.md]。

### 1.4 Shared property [core-concepts.md]
- 同一个属性定义被多个 object type 复用，元数据（名称、描述、类型）集中管理；**只共享元数据，不共享数据** [olt/shared-property-overview.md]。可直接新建或由已有属性转换；可挂 value type [olt/use-value-type.md]。
- 与 interface 的关系：interface property 可**本地定义（推荐）**或引用 shared property [if/interface-overview.md]；Pipeline Builder 实现 interface 时同名 shared property 自动映射 [if/implement-interface.md]。

### 1.5 Value types
- 定义：field type 之上的语义包装，带元数据 + 约束；归属 **space**（一个 space 一个 ontology），只能在定义它的 space 内使用，Default ontology 不可用 [olt/value-types-overview.md]。可用于 PK（Scenario Reference）[persisted-scenario.md]；派生属性不能直接挂，但继承源属性的 value type [derived-properties.md]。
- 使用点：object type 属性、shared property、Pipeline Builder logical type cast、action 参数约束（`String/Integer/Long/Double/Decimal/Date/Timestamp` 及其 list；list 可选 `Array` value type 整体约束或元素级约束；struct 参数不可用）；属性值不满足约束 → **该 object type 索引失败**；action 校验失败显示 value type 的 failure validation message [olt/use-value-type.md]。
- **约束种类** [olt/value-type-constraints.md]：

| 约束 | 适用 base type | 语义 |
|---|---|---|
| Enum (one of) | String、Boolean、Decimal、Double、Float、Integer、Short | 静态允许值集合；String 可选大小写敏感 |
| Range | Decimal、Double、Float、Integer、Short、Date、Timestamp、String、Array | min/max；String 约束长度，Array 约束元素个数 |
| Regex | String | 可选"子串匹配即通过" |
| RID / UUID | String | 合法 RID / UUID |
| Uniqueness | Array | 元素唯一 |
| Nested | Array | 对元素施加 value type 约束 |
| Element constraints | Struct | struct 字段 → value type 引用 |

- **版本**：name / description / apiName 可随时改；base type 与约束不可变，改约束 = 新版本；非破坏性新版本自动传播到 ontology 与 action；有消费者时破坏性变更建议弃用旧类型另建 [olt/value-types-versions.md]。value type 单独权限控制 [olt/value-types-permissions.md]。

### 1.6 Struct [ontology-structural-guidance.md]
- 把语义相关的多字段合成一个属性（地址、坐标、LLM 输出 + confidence + reasoning + source）。
- **Main field**：一个或多个主字段，使 struct 在多数上下文（含 interfaces 与 queries）表现得像简单属性或其子集。
- 支持 struct array + reducer（例：按 `lastOccupied` 取最新地址）。
- OSDK 支持度不一：Python 不支持（仅函数 beta），OpenAPI 渲染为 `any` [osdk/unsupported-types.md]。

### 1.7 Vector / embedding property
- 配置两项：**Dimension**（数组长度）与 **Similarity Function**（距离函数，如 cosine）[using-custom-models-to-create-a-semantic-search-workflow.md]。
- 属性可绑定 embedding model，使 `nearestNeighbors` 能接受文本，查询时自动 embed [osdk/typescript-osdk.md]。
- 约束：vector 不能被聚合、不能被派生属性选择、派生属性不能产出 vector；`collectList/Set` 不接受 vector [derived-properties-reference.md]。
- Vector 可被 Actions 与 Functions 编辑 [using-custom-models...md]。
- KNN 上限：TS OSDK 2.x / TSv2 `1 <= numNeighbors <= 500`；TSv1 `0 < K <= 100`；Workshop KNN object set `1-100` 且不能按 relevance 排序 [osdk/typescript-osdk.md, using-palantir-provided-models...md]。
- `$orderBy: "relevance"` 返回 `$score`；Python 返回 `_score` [同上]。
- 语义检索流水线：chunking（chunk size / overlap / 优先级分隔符）→ explode → `chunk_id = object_id + "_" + position` → embed → chunk 对象 link 回源对象 [document-processing.md]。混合检索：keyword（`matchAnyToken` + `orderByRelevance`）与 KNN 结果做 RRF，`score = Σ 1/(k + rank)`，默认 `k=60` [ontology-augmented-generation.md]。

### 1.8 Link type
- 两个 object type 间关系的 schema；link = 单个实例 [core-concepts.md]。
- **Cardinality / multiplicity**：single 侧生成 `SingleLinkAccessor`（`fetchOne()` / `fetchOneWithErrors()`），many 侧生成目标类型的 `ObjectSet` [osdk/typescript-osdk.md]。类型：one-to-one、many-to-one、one-to-many、many-to-many [derived-properties-reference.md, osdk/python-osdk.md]。
- **Foreign key 型**（one-to-many）：many 侧对象持有 FK 属性指向 one 侧 PK。Ontology-as-code 示例：`defineLink({ apiName, one: {object, metadata:{apiName}}, toMany: {object, metadata:{apiName}}, manyForeignKeyProperty: "assigneeId", editsEnabled: true })` [superrepo/tutorial-develop-with-a-superrepo.md]。FK 可以是数组，此时两个方向都按"可能多个"处理 [derived-properties-reference.md]。
- **Many-to-many**：由 **join table dataset** 支撑：数据集需含两侧 PK 对应列，每列只能映射一个 PK；可 **Generate join table** 按两侧 PK 自动生成；M:N 必须有 backing datasource 才能编辑/writeback；同一 datasource 不能同时 back 两个 link type（`Phonograph2:DatasetAndBranchAlreadyRegistered`）[olt/create-link-type.md, olt/link-types-overview.md]。scenario 中只有 M:N 链接作为 `(source, target, linkType)` 三元组返回，1:N 的编辑表现为持有 FK 一侧对象的编辑 [osdk-scenario.md]。
- **方向性**：一个 link type 天然双向，两个 side 各有 display name 与 API name；不要为反向再建 link type；同一对 object type 间可有多个 link type（各代表不同关系），side API name 须唯一；支持自关联；不支持跨 ontology link [olt/link-types-overview.md]。
- **Object-backed link**：关系本身带元数据（role / startDate / allocation / status）时，用中间对象类型 `Employee → VentureStaffing → Venture` [ontology-structural-guidance.md]。原生支持为一等 link 存储方式：先建 backing object type 及其到两侧的两条 many-to-one link，再建 object-backed link type；已有 link 可转换；支持 restricted views [olt/create-link-type.md]。
- 双向命名：每个方向一个 API name（`department` / `employees`）[ontology-structural-guidance.md]。
- 派生属性和 FK：派生属性不能作 link FK [derived-properties-reference.md]。

### 1.9 Action type
- 定义：用户一次性执行的一组对 objects、property values、links 的编辑，**包含提交时的 side effect 行为** [core-concepts.md]。
- **Parameters**：有类型（含 `ObjectSet`、`InterfaceObjectSet`、`MarkingList`、`scenarioReference`、attachment 等）；可设 required、默认值（含"对象属性当前值"默认，OSDK 省略该参数时会回写该值 [at/consistency-guarantees.md]）、**Constraints**（含 value type 约束）、**Visibility**、override [persisted-scenario.md, at/parameter-overview.md, at/parameters-default-value.md, at/parameters-override.md]。属性类型 → 参数类型映射（如 `Byte` → `Integer`、`Float` → `Double`）见 [at/scale-property-limits.md]。
- **Rules 完整清单** [at/rules.md]：
  - Ontology rules：① Create object（PK 必填）② Modify object(s)（对象来自 object reference 参数，不能引用本 action 新建的对象）③ Create or modify object(s)（未选对象则新建，PK 自动生成或用户提交）④ Delete object(s)（同 ② 限制）⑤ Create link(s) ⑥ Delete link（⑤⑥ 仅 M:N；1:1 / 1:N 必须用 Modify object 改 FK）⑦ **Function rule**（引用 Ontology edit function；存在时**不得配置其他任何 rule**）⑧-⑫ interface 版 Create / Modify / Delete object(s) of interface、Create / Delete link(s) on object(s) of interface [at/actions-on-interfaces.md]。另有 **Apply Scenario** [merge-scenario.md]。
  - Side effect rules：Notification（edits 全部应用后发送，内容基于应用前状态）、Webhook（可配置在 edits 前或后执行）；高级 Schedule rule（触发 schedule build，build 开始后才应用 edits）[at/rules.md, at/side-effects-overview.md]。
  - 属性值来源：From parameter（新增属性默认自动建同名参数）/ Object parameter property / Static value / Current User / Current Time（取执行开始时间，可不同于 action log 提交时间）；link rule 只接受 object reference 参数。
  - 多 rule 编译为**每对象一个 edit**，后写覆盖前写、rule 顺序有意义；非法组合：先删后增/改、先改后增、同一提交内重复创建。可在 Create object rule 里同时建 M:N link。
- **Submission criteria**（旧称 validations）[at/submission-criteria.md]：条件模板 `Current user`（user ID、group IDs 含继承成员、multipass 属性按 string list 比较）/ `Parameter`（可取对象参数的属性、list 长度）/ `Execution context`（是否在 Scenario 内）；不支持 attachment 与 object set 参数。
  - 单值运算符：`is` `is not` `matches`（正则）`is less than` `is greater than or equals` 等；多值：`includes` `includes any` `is included in` `each is` `each is not`；右值 = 参数 / 静态值 / 无值（判空）。
  - 逻辑运算符 all / any / none，可嵌套；根级每个条件/运算符各有 failure message。禁止对 group / marking / organization 成员资格用 `NOT`（scoped token 可能缺属性导致误放行）。
- **Side effects**：notifications、webhooks（REST / ERP writeback；OAuth 2.0 webhook 的 action 不能经 OSDK 调用）[at/side-effects-overview.md, at/webhooks.md, osdk/unsupported-types.md]。side effect **不在**原子性范围内，revert 也不撤销 side effect [at/consistency-guarantees.md, at/action-reverts.md]。
- **执行语义**：
  - 模式：`VALIDATE_ONLY` / `VALIDATE_AND_EXECUTE`；TS 中 `$validateOnly` 与 `$returnEdits` 互斥 [osdk/typescript-osdk-migration.md]。
  - 校验响应：`validation.result` ∈ `VALID|INVALID`；`submissionCriteria[]`；`parameters.{name}.{result, evaluatedConstraints[], required}` [osdk/typescript-osdk.md]。
  - 编辑响应：`edits[]` 元素类型 `addObject|modifyObject|deleteObject`（+ link 增删），并给 `addedObjectCount / modifiedObjectsCount / deletedObjectsCount / addedLinksCount / deletedLinksCount` [osdk/typescript-osdk.md, osdk/python-osdk.md]。
  - **Batch apply**：同一 action 多组参数，all-or-nothing；响应只含 edits 不含 validation [osdk/typescript-osdk.md]。
  - 每个 action 有面向 agent 的 **Agent tool description** 字段 [mcp/mcp-tools-and-agent-configuration.md]。
  - **限制** [at/scale-property-limits.md]：primitive list 参数 ≤10,000 元素、object reference list ≤1,000、submission criteria 中 list ≤1,000；单次提交编辑 object type ≤50、对象 ≤10,000、单对象 edit ≤32KB（OSv1）/ 3MB（OSv2）；batch 调用 ≤10,000 次（function-backed 且未开 batched execution 时 ≤20），batch 内 edits 合并计限。scenario merge 同受此限 [overview-ontology-scenario.md]。
  - **事务语义** [at/consistency-guarantees.md]：ACID；edits 单批原子提交；Consistency = 满足 value type 约束与 required。写模式：batched writes（默认，本 action 读不到自己的编辑）/ staged writes（读己之写，仅 TSv2 / Python / AIP Logic function-backed，且只支持 legacy 隔离）。隔离级别：**snapshot isolation**（新 action 默认，读取单一快照，写-写冲突按**对象级**检测，不同属性也冲突，不防 write skew）、legacy、read committed（即将推出）；backing dataset 重建可引发瞬时冲突；可配置瞬时错误自动重试（从头重跑）。
  - **审计与撤销**：action log 把**成功**提交建模为对象（失败看 action metrics），只记录经 action type 的编辑，可设"该 object type 只允许带 action log 的 action" [at/action-log.md]；逐对象完整编辑历史另用 edit history。Revert 仅 OSv2、仅提交者本人、仅当该 action 是对象最新编辑时可撤销，新 action 默认开启 [at/action-reverts.md]。
  - **Apply 权限** [at/permissions.md]：须能查看被编辑的 object/link type 及其数据源并通过 submission criteria；"仅经 action 编辑"（新 object type 默认）时只需对被编辑对象 `Read`（可能创建自己看不到的对象）；否则需 writeback dataset 的 `Edit`；另有 marking 维度的 read/write authorizations [at/read-write-authorizations.md]。

### 1.10 Function
- 接收参数、返回输出的代码逻辑；可接收 objects / object sets，读属性，被 action 和应用复用 [core-concepts.md]。
- 服务端隔离环境执行；语言：TypeScript（v1 / v2）、Python；能力差异见 [fn/language-feature-support.md]。用途含 Workshop function-backed columns / 聚合图层、function-backed action、外部系统查询、streaming [fn/overview.md]。
- **Query function**（只读、无 side effect，可选经 API gateway 暴露；TSv1 `@Query({apiName})` / TSv2 导出 `config = {apiName}` / Python `@function(api_name=...)`；API name 有校验与版本规则）[fn/query-functions.md]：返回 Object、ObjectSet、Attachment、二维聚合 `{groups:[{key,value}]}`、三维聚合（嵌套 groups）、Map<Object, V>、标量 [osdk/python-osdk.md]。OSDK 调用：`client(fn).executeFunction({...})` [osdk/typescript-osdk.md]。
- **Edit function**：返回 `OntologyEdit[]`；TSv1 `@OntologyEditFunction` + `@Edits(...)`；TSv2 `createEditBatch(client)` → `batch.update(obj, {...})` → `batch.getEdits()`，用 `Edits.Object<T> | Edits.Interface<I> | Edits.Link<T, "link">` 声明可编辑实体（provenance，供 action 做权限检查）；Python 用 `FoundryClient` edits 容器 + `@function(edits=[...])` [fn/edits-overview.md, fn/typescript-v2-ontology-edits.md, fn/python-ontology-edits.md]。业务错误抛 `UserFacingError` [superrepo/tutorial-develop-with-a-superrepo.md]。
  - **只有经 function-backed action（function rule）才真正落库**；在 Authoring 里运行只预览 edits [fn/edits-overview.md, at/function-actions-overview.md]。受 action 限制与函数执行限制双重约束。
  - link 编辑：M:N 直接增删 link；1:1 / 1:N 编辑持 FK 的对象。
  - 读己之写：edit batch 模式下 `Objects.search()` 的过滤 / search around / 聚合看不到本函数编辑（按 PK 取已编辑对象则会应用待提交编辑）；staged writes（beta）提供读己之写 [fn/edits-overview.md]。新对象 ID 生成见 [fn/edits-generate-id.md]。
- **Function-backed column vs 运行时派生列**：Workshop runtime 派生属性只支持单跳 link，排序时对象集上限 200 行；多跳或大集合排序改用 ontology-defined 派生属性（≤3 跳）或 function-backed column [fn/workshop-derived-properties.md]。
- 所有 function（AIP Logic 需 staged writes）可在 scenario 上运行：读 scenario 状态，写入 scenario [overview-ontology-scenario.md]。
- Model 通过 wrapper function 进入 Ontology（live inference）[models.md]。

### 1.11 Interface
- 描述 object type 形状与能力的 Ontology 类型，提供多态 [overview.md, core-concepts.md]。**抽象**：无 backing dataset、不可直接实例化，对象只能作为某具体 object type 的实例出现；权限经 Ontology roles [if/interface-overview.md]。
- 组成：interface properties（required / optional）+ **link type constraints** + **action type constraints**（beta）+ metadata [if/interface-overview.md, if/interface-metadata.md]。
- **Extends**：可继承任意多个 interface（多层），继承 properties、link 约束、action 约束；移除 extension 同时移除全部继承项 [if/extend-interface.md]。
- **Implement**：object type 必须 ① 把本地属性映射到全部 required interface property ② 为每个 required link 约束指定满足它的具体 link type ③ 为每个 required action 约束指定具体 action type 并映射 required 参数；保存时校验 [if/implement-interface.md]。实现后 OSS 搜索 interface 会返回实现类型的对象；同一对象既可用本地 API name 也可用 interface API name 访问。
- **Link type constraint**：target = interface 或 object type；cardinality `ONE` / `MANY`；可 required；由实现类型的具体 link type 满足 [if/interface-link-types-overview.md]。
- **Action type constraint**：display name、description、API name、参数约束（类型、单值/list、required）；只描述能力形状，不定义执行逻辑 / submission criteria / side effect；同一具体参数不能满足两个参数约束 [if/interface-action-type-constraints.md]。
- 支持度：OSS 支持按 interface 搜索/排序，聚合与 interface link **开发中**；函数仅 TSv2；OSDK 仅 TS [if/interface-overview.md]。
- 实现时 interface property 映射到 object type 的具体属性（例：`Scenario Reference` → PK，`Expiration Time` → 某 timestamp 属性）[persisted-scenario.md]。
- 两类用法：能力型（`Inspectable`、`Schedulable`、`Billable`）与分类型（`MilitaryAsset`）[ontology-best-practices.md]。
- Interface object set 的订阅更新以底层 object type 返回，需用 object type full metadata 做属性重映射 [osdk/websocket-subscriptions.md]。

### 1.12 Object set 操作（汇总）
| 类别 | 操作 | 来源 |
|---|---|---|
| 构造 | base(objectType)、static（对象列表 / 对象 RID 列表 / object set RID，如 `Objects.search().flights([flight])`）、interface base | osdk/typescript-osdk.md, fn/api-object-sets.md |
| 过滤 | `where(WhereClause)`，布尔组合 `$and/$or/$not` | 同上 |
| 遍历 | `pivotTo(linkApiName)`（Search Around），可链式 | 同上 |
| 集合运算 | `union` / `intersect` / `subtract`（同类型） | 同上 |
| 向量 | `nearestNeighbors(query, k, property)` | 同上 |
| 派生 | `withProperties({...})` | derived-properties.md |
| 读取 | `fetchOne(pk)`、`fetchPage({$pageSize,$orderBy,$select?})`、`asyncIter()`，分页 `nextPageToken` | osdk/typescript-osdk.md |
| 排序 | `$orderBy: {prop: asc|desc}` 或 `"relevance"`；字符串排序大小写敏感（数字 < 大写 < 小写） | 同上 |
| 聚合 | `aggregate({$select, $groupBy})` | 同上 |
| 订阅 | `subscribe({onChange,onSuccessfulSubscription,onOutOfDate,onError},{properties})` | osdk/typescript-subscriptions.md |

Wire 格式（REST/WebSocket 通用）：`{"type":"base","objectType":"Country"}`、`{"type":"filter","objectSet":{...},"where":{"type":"gte","field":"population","value":1000000}}` [osdk/websocket-subscriptions.md]。

### 1.13 Derived properties [derived-properties.md, derived-properties-reference.md]
- 查询时计算，不存储、不索引、**只读**（不能被 action/function 编辑）。
- 三种计算方式：跨 link 聚合、从单个 linked 对象选取属性、算术/日期表达式。
- 两种形态：

| | Ontology-defined | Runtime-defined |
|---|---|---|
| 配置位置 | Ontology 定义 | 请求内 object set 定义（`withProperties`） |
| 可见性 | 所有消费者 | 声明它的请求/模块，及保存的 object set |
| 最大 link 遍历 | 3 | OSDK 无固定上限（Workshop/Insight 为 1） |
| 表达式 | 不支持 | 支持 |
| 状态 | Beta | GA |

- 选取 vs 聚合判定：每跳最多到达 1 个对象 → 可直接选取；任一跳可能多个 / 含 M:N / FK 为数组 → 必须聚合。
- 表达式只能引用**更早**的 `withProperties` 中声明的派生属性；同一操作内不能互相引用；link/聚合型派生属性不能引用其他派生属性；不支持静态字面量。
- 能力：可 return（需显式 select）、filter（exact / range / prefix / phrase / full-text / geo / relative date）、sort、aggregate、group by；不可编辑、不可作 PK、不可作 FK、不可 required、不可挂 value type / formatting / inline action。
- **安全**：在计算链上每个对象（含中间跳）做访问检查，不同用户可看到同一对象同一派生属性的不同值；collect 结果保留每个值自身的 marking。
- collect 截断语义：limit（默认 10，最大 100）只限制返回，filter/sort/aggregate 针对完整值集合；reducer 等同 limit=1 但仍对全部值过滤。

### 1.14 Scenarios [overview-ontology-scenario.md, temporary-scenario.md, persisted-scenario.md, merge-scenario.md, osdk-scenario.md]
- 定义：在 Ontology 数据之上应用 action 产生的编辑沙箱（what-if），**不是**历史快照/数据版本工具。
- **Temporary**：仅当前会话，元数据不持久化。
- **Persisted**：以对象形式保存元数据；对象类型实现 `Ontology Scenario` interface，PK 为 `Scenario Reference` value type，可选 `Expiration Time` 映射（≤ 当前时间 + 31 天）；可挂 Created By / Status / Reviewed By 等，用 object security policy + submission criteria 做 Draft → In review → Approved/Rejected 审批流。
- TTL 默认 31 天，到期删除全部编辑；每 10 分钟自动 rebase 到其 base（main 或 global branch），不可手动 rebase、不可调间隔。
- **Merge**：通过带 **Apply Scenario** rule 的 action 执行（自动暴露 `scenarioReference` 参数），把全部 staged 编辑作为**单事务**提交；merge action 定义作用范围（对象类型 + link 类型）与执行权限。
- 约束：scenario 内不支持 batch action；不可嵌套；不可与 transaction 组合；基于 branch 的 client 创建的 scenario 以该 branch 为 base。
- 编辑枚举 API：`getEditedEntityTypes` → `{objectTypes, linkTypes}`；`getEditedEntities(type,{pageSize})` / `editedEntitiesAsyncIter`（按 PK 去重，稀疏结果只含 `$apiName`/`$primaryKey`）；`getEditedLinkTypes` / `getEditedLinks` 返回 M:N `(source,target,linkType)`。

### 1.15 Global Branching / Proposals [gb/core-concepts.md, gb/branch-security.md, gb/resource-protection-and-approval-policies.md]
- 面向 builder：在 branch 上改 schema（object type 定义）+ 数据管线 + 应用，端到端测试后合并；scenario 面向使用者改数据 [overview-ontology-scenario.md]。
- Ontology 资源在 branch 上增删改不影响 main；branch 创建时绑定一个 ontology，之后不可变。
- **Rebase**：非冲突变更自动合并；"同一资源同一属性两边都改"为真冲突，需手动选版本。
- **Proposal**：带 markdown 描述；状态 Open / Merged / Closed；合并前对每个资源跑 **checks**（rebase 冲突、所需审批），全部通过才能合并；部分合并失败不可回滚，需修复后重试。
- **Branch 状态**：Active / Inactive（默认 35 天无活动，之后 7 天删数据，ontology 去索引）/ Archived（手动，可恢复）/ Merged（终态）。
- **安全**：branch 角色 Owner（space Administrator 等价）；角色只管 branch 管理动作，不授予资源编辑权；访问还需属于 branch 的 organization（须为 space organizations 子集）；任何能看到 proposal 的人在审批与 checks 满足、未设 "Do not merge" 时可合并。
- **审批策略**：resource protection（受保护资源只能经 branch 改）；project 级 policy：eligible reviewers、所需批准数、是否允许 contributor 自批；默认策略可"自动满足"（贡献者自身权限覆盖）或"经评审满足"；任一 rejection 阻止整个 proposal。
- OSDK 本身不可 branch [gb/integrations.md]。

### 1.16 Object permissioning [op/overview.md, op/ontology-permissions.md, op/managing-object-security.md, op/object-security-policies.md, op/multi-datasource-objects.md]
- **两层**：ontology 资源（object / link / action type = schema）与 objects / links（数据）分开授权；能看 object type ≠ 能看对象。
- **资源权限（当前模型）**：资源存于 Compass project，按 project 角色（Viewer / Editor / Owner）授权，可按 folder 批量、可加 marking 隐藏；新 ontology 默认开启。legacy：Ontology roles（`viewer/editor/owner`）与 datasource-derived（对 object type 的权限 = 对 backing datasource 的权限）[op/ontology-permissions-legacy.md]。编辑 link 需 link type + 两侧 object type 的编辑权；编辑 action 需 action type + 其编辑的全部资源的编辑权。
- **看对象** = object type `Viewer` + 数据访问，数据访问二选一：
  - **Object / property security policy**（推荐）：配置在 object type 上，与 backing datasource 权限解耦（无数据源的 object type 必须用）。object policy = 行级，property policy = 列级，二者叠加 = cell 级；通过 object policy 但不通过 property policy → 该属性值显示为 `null`。policy 内容 = granular policy（条件，可引用 mandatory control property）+ markings / organizations / classification。约束：须先有 object policy；PK 不能进 property policy；每个非 PK 属性最多属于一个 property policy；创建/编辑 policy 需 object type `Owner`。变更近实时生效，支持 streaming 与 Global Branching。
  - **Data source policy**：数据集 / stream `Viewer` 即可看其产生的全部对象；行级用 restricted view（policy 如 `user.userAttribute('care_center') == care_center`，仅数据集，改策略需重建），列级用 **column-wise MDO**（多数据源按 PK 拼列，每个属性只能来自一个数据源，PK 必须在每个数据源中；无权数据源的属性显示 `null`；仅 OSv2，≤70 数据源；不支持 row-wise MDO 与 streaming）。
- **读时强制**：granular 行/列控制只过滤读取，不随数据向下游传播；需要传播用 marking / CBAC。媒体等外部资源的访问不受属性级控制约束 [op/managing-object-security.md]。

---

## 2. 查询/检索语法

### 2.1 WhereClause 运算符 [osdk/typescript-osdk.md, osdk/python-osdk.md, osdk-scenario.md]
| 运算符 | 适用类型 | 语义 |
|---|---|---|
| `$startsWith` | string | 前缀匹配，大小写不敏感 |
| `$containsAnyTerm` | string | 空白分词，任一 term 命中，大小写不敏感 |
| `$containsAllTerms` | string | 全部 term 命中，顺序无关 |
| `$containsAllTermsInOrder` | string | 全部 term 按序且相邻（phrase） |
| `$lt` `$gt` `$lte` `$gte` | numeric / string / datetime | 范围 |
| `$eq` | boolean / datetime / numeric / string | 相等 |
| `$in` | 含 `$primaryKey` | 集合成员 |
| `$isNull` | array / boolean / datetime / numeric / string | 空值；非空用 `$not` 包裹 |
| `$and` `$or` `$not` | 任意 | 布尔组合 |
- **Geo 运算符**（TSv1 名称；TSv2 对应见 typescript-v2-migration 的 filter operator mapping，未镜像）[fn/api-object-sets.md]：geopoint `withinDistanceOf` `withinPolygon` `withinBoundingBox`；geoshape `withinBoundingBox` `intersectsBoundingBox` `doesNotIntersectBoundingBox` `withinPolygon` `intersectsPolygon` `doesNotIntersectPolygon`。派生属性同样支持 geographic 与 relative date 过滤 [derived-properties-reference.md]。
- TSv1 其他过滤：`exactMatch`（空数组则忽略该过滤）、`hasProperty()`（判空）、`phrase` `phrasePrefix` `prefixOnLastToken` `matchAnyToken` `fuzzyMatchAnyToken` `matchAllTokens` `fuzzyMatchAllTokens`（`Fuzziness` 同 Elasticsearch）、`range().lt/lte/gt/gte`、`isTrue/isFalse`、link `isPresent()`、array `contains`（任一命中）[fn/api-object-sets.md]。
- Struct 查询按 Elasticsearch object 字段语义逐字段独立求值：struct array 上 `firstName=Ada AND lastName=Moreau` 可跨元素命中 [olt/structs-overview.md]。
- TSv1 函数 API 有 `exactMatch`、`matchAnyToken`、`orderByRelevance()`（OSv2 索引自带相关度打分）[ontology-augmented-generation.md, using-custom-models...md]。

### 2.2 Regex 检索 [search-syntax.md]
- 属性需开启 "Enable regex queries" 索引。
- **整值匹配**（索引存未分析完整字符串）：`cat` 只匹配 `cat`；子串需 `.*cat.*`；不支持 `^` `$`。
- 支持：`.` `?` `+` `*` `{n}` `{n,}` `{n,m}` `|` `()` `[]`（含 `-` 区间与 `^` 取反）、`"..."` 字面量串、`\` 转义与 `\d \D \s \S \w \W`。
- 实现提示：语义接近 Lucene/Elasticsearch regexp（全匹配、无锚点）。Rust 侧可用 `regex` crate 加隐式 `^(?:...)$`，`"..."` 需自行转义为字面量。

### 2.3 聚合 [osdk/typescript-osdk.md, derived-properties-reference.md]
- `$select` key 为 `$count` 或 `"<prop>:<metric>"`；value 为排序 `unordered|asc|desc`；仅当 `$groupBy` ≤ 1 个键时可排序。
- 指标：`$count`、`sum`、`avg`、`min`、`max`、`approximateDistinct`、`exactDistinct`、（派生属性中）`collectList`、`collectSet`、`approximatePercentile`。
- 类型规则：sum/avg 仅数值；min/max 支持数值、string、boolean、date、timestamp；数组输入先 flatten。
- Sum 拓宽：`short/integer → long`、`float → double`、`decimal` 增精度。
- `$groupBy`：`"exact"`、`{$fixedWidth: n}`（报告桶起点）、`{$ranges: [[start, end)]}`（报告 `{startValue,endValue}`，两端必填）、`{$duration: [n, unit]}`（`seconds/minutes/hours` 仅 timestamp；`weeks/months/quarters/years` 只能 1）。
- 结果形状：无 groupBy 返回单对象；有 groupBy 返回行数组 `{$group:{...}, ...metrics}`；聚合结果上限 10,000（`ObjectsExceededLimit`）。
- Python 支持指标别名 `name`，并可先 `with_properties` 选取 linked 属性再 group by [osdk/python-osdk.md]。

### 2.4 派生属性表达式 [derived-properties-reference.md]
- 数值：`add` `subtract` `multiply` `divide` `max` `min` `abs` `negate`。
- 日期/时间戳：`min` `max` `extractPart(DAYS|MONTHS|QUARTERS|YEARS)`。
- 成本模型：load 低；search（filter/sort）多数高；collect 可下推到源属性索引，search 成本低；推荐 approximateDistinct 而非 exactDistinct；先过滤再聚合。

---

## 3. OSDK 与 Ontology MCP 接口面

### 3.1 OSDK 客户端操作清单 [osdk/typescript-osdk.md, osdk/python-osdk.md, osdk/java-osdk.md]
- **Objects**：`fetchOne(pk)`、`fetchOneWithErrors`、`fetchPage`、`asyncIter`、`where`、`pivotTo`、`union/intersect/subtract`、`nearestNeighbors`、`withProperties`、`aggregate`、`subscribe`、可选加载 RID 与指定属性子集。
- **Links**：`obj.$link.<name>`：single → `fetchOne`；many → 完整 ObjectSet API。
- **Actions**：`applyAction(params, {$returnEdits | $validateOnly})`、`batchApplyAction([params...])`。
- **Queries（functions）**：`executeFunction(params)`。
- **Attachments**：`upload(file, name)` → `Attachment{rid}`，`get_metadata()`、`read()`；未在 1 小时内被 action 写入属性即删除，写入前仅上传者可读 [osdk/python-osdk.md]。
- **Media**：读 content / metadata / media reference，上传，作为 action 参数 [osdk/java-osdk.md, osdk/python-osdk.md]。
- **Scenarios**（实验性）：`createScenario`、`withScenario(rid)`、`getScenarioReference`、编辑枚举 API（见 1.14）。
- **Subscriptions**：WebSocket `wss://{host}/api/v2/ontologySubscriptions/ontologies/{ontology}/streamSubscriptions`，子协议 `Bearer-{token}`；请求 `{id, requests:[{objectSet, propertySet?, referenceSet?, objectLoadingResponseOptions?}]}`；新消息整体替换活跃订阅集合，空数组 = 全退订；状态 `ADDED_OR_UPDATED | DELETED`；`onOutOfDate` 要求客户端全量重载；不支持 `pivotTo` 构造的集合、不跨服务端重启持久、有单订阅内存上限 [osdk/typescript-subscriptions.md, osdk/websocket-subscriptions.md]。
- **代码生成**：按 application 选定的 ontology 子集生成强类型 SDK（TS / Python / Java / OpenAPI）；token 限定于选中实体 + 用户自身权限；行/列级安全只在**读取时**强制 [osdk/index.md]。
- **测试工具**：`createMockOsdkObject`（含 `$primaryKey/$title/$rid/$link/$clone`）、mock object set、stub client 调用 [osdk/typescript-osdk-testing.md]。
- **Ontology-as-code**：TS DSL（`defineLink` 等）定义 object types / interfaces / actions，本地 seed 数据 + 本地 SDK 再生成 [osdk/index.md, superrepo/tutorial-develop-with-a-superrepo.md]。这一模式与 Rust 项目的 "schema 即代码" 路线一致。

### 3.2 Ontology MCP 工具面 [mcp/index.md, mcp/sample-architecture.md, mcp/debugging-with-mcp-inspector.md, mcp/authentication-and-authorization.md]
- 定位：面向 ontology **消费者**（读数据 + 受控写数据）；Palantir MCP 面向 builder（70+ 工具改 schema，不写数据）。
- 工具来源 = Developer Console application 的资源范围（application restrictions）：
  - **Object types** → 一个 **SQL 工具**，agent 用 SQL 读已暴露的对象类型（示例中另有 `search-<objectType>` 形态的搜索工具）。
  - **Action types** → 每个 action 一个工具（如 `create-osdk-todo-task`），描述来自 Agent tool description。
  - **Query functions** → 每个函数一个工具；AIP Logic / chatbot 保存为函数后也可作为工具（agents as tools）。
- 不支持 MCP prompts / resources。
- 认证：复用应用的 OAuth 2.0；authorization code grant（代表用户）或 client credentials grant（服务用户）；token 受应用 scope 与用户权限双重限制。
- 价值点：agent 的每次写入都记录在 ontology 中，提供无人值守场景的审计 [mcp/example-mcp-workflows.md]。

---

## 4. 可自动执行的设计校验（lint 规则候选）

来源：[ontology-best-practices.md]、[ontology-anti-patterns.md]、[ontology-structural-guidance.md]、[derived-properties*.md]、[persisted-scenario.md]。严重级：E = 硬错误（schema 无效），W = 警告，I = 建议。

| ID | 级 | 规则 | 依据 |
|---|---|---|---|
| ONT-E01 | E | PK 不得是派生属性；派生属性不得 required、不得挂 value type / formatting / inline action、不得作 link FK | derived-properties.md |
| ONT-E02 | E | Ontology-defined 派生属性 link 遍历 ≤ 3 且不含表达式 | derived-properties.md |
| ONT-E03 | E | 链上任一跳可能多个对象（含 M:N、FK 数组）时必须用聚合，不得直接 select | derived-properties-reference.md |
| ONT-E04 | E | 表达式只能引用之前 `withProperties` 中声明的派生属性；link/聚合型不得引用派生属性 | 同上 |
| ONT-E05 | E | vector 属性不得参与聚合/派生；vector 必须声明 dimension 与 similarity function | derived-properties-reference.md, using-custom-models...md |
| ONT-E06 | E | 聚合指标与属性类型匹配（sum/avg 仅数值等） | osdk/typescript-osdk.md |
| ONT-E07 | E | scenario 对象类型必须实现 `Ontology Scenario` interface，PK 为 Scenario Reference，过期时间 ≤ 31 天 | persisted-scenario.md |
| ONT-E08 | E | 含 Function rule 的 action 不得有其他 rule；Create/Delete link rule 只用于 M:N，1:N/1:1 改 FK；rule 组合不得出现先删后增/改、先改后增、重复创建 | at/rules.md |
| ONT-E09 | E | Base type 与用途匹配：PK 只能是 `String`/`Integer`/`Short`（`Date`/`Timestamp`/`Boolean`/`Byte`/`Long` 警告）；title key 不得为 `Vector`/`Struct`/`Media Reference`/`Time Series`/`Geotemporal Series`/`Attachment`/`Geoshape`/`Marking`；struct 不嵌套、字段非 array；array 不含 `Vector`/`Time series`、OSv2 不嵌套 | olt/properties-overview.md, olt/structs-overview.md |
| ONT-E10 | E | Value type 约束与 base type 兼容（见 1.5 表） | olt/value-type-constraints.md |
| ONT-E11 | E | 实现 interface 时 required property / link 约束 / action 约束全部映射，类型兼容，同一 action 参数不得满足两个参数约束 | if/implement-interface.md, if/interface-action-type-constraints.md |
| ONT-E12 | E | Property security policy：须已有 object policy；PK 不得入 policy；非 PK 属性至多属于一个 policy；无数据源 object type 必须有 object policy | op/object-security-policies.md, op/managing-object-security.md |
| ONT-W16 | W | Submission criteria 或 granular policy 对 group / marking / organization 成员资格使用 `NOT` | at/submission-criteria.md, op/object-security-policies.md |
| ONT-W01 | W | Object type 名为泛化词（`Data` `Item` `Record` `Info` `Object` `Entity`）或复数 | ontology-structural-guidance.md |
| ONT-W02 | W | 属性名为未限定泛化词（`value` `type` `status` `date` `name` `quantity` `score`）且对象中存在歧义 | ontology-anti-patterns.md (Misnomer) |
| ONT-W03 | W | 属性名含编码/缩写模式（`^dt[A-Z]`、`^n[A-Z]{2,}`、`\d{2,}$`、`field[A-Z0-9]`、snake_case 源列名） | ontology-best-practices.md |
| ONT-W04 | W | 日期命名不一致（同 ontology 混用 `createdDate` 与 `dateOfCreation`） | ontology-structural-guidance.md |
| ONT-W05 | W | Kitchen Sink：属性名以 `_` 开头或含 `etl` `extracted_at` `received_at` `batched_at` `sequence` `table_version` `internal_record_id` | ontology-anti-patterns.md |
| ONT-W06 | W | God Object：属性数超阈值（如 >60）且多数属性空值率 > 80%；存在 `*Type`/`*Subtype` 判别列 | ontology-anti-patterns.md |
| ONT-W07 | W | System/Department Silo：类型名带系统或部门前缀（`HR System`、`Sales`、`Billing`、`CRM`），或两类型 PK 语义相同且属性集 Jaccard ≥ 0.6 | ontology-anti-patterns.md |
| ONT-W08 | W | Rule of three：≥3 个类型共享同一属性子集（≥3 个同名同类型属性）→ 建议提取 interface / shared property | ontology-best-practices.md |
| ONT-W09 | W | Action Sprawl：单对象类型 action > 10，或 action 名匹配 `^(Set|Update) \w+` 且只改 1 个属性 | ontology-anti-patterns.md |
| ONT-W10 | W | Time Machine：属性 `version` `revision` `isCurrent`，或类型名带年份/`v\d` 后缀 | ontology-anti-patterns.md |
| ONT-W11 | W | Link 命名泛化（`related*`、`link\d`）或缺任一方向 API name | ontology-structural-guidance.md |
| ONT-W12 | W | 反范式拷贝：属性名 = `<linkedType><Prop>`（如 `managerName`）且存在对应 link；手工维护的 `*Count` 且存在可计数的 link → 建议派生属性 | ontology-structural-guidance.md |
| ONT-W13 | W | 关系元数据挂在端点上（`ventureRole`、`ventureStartDate` 前缀 = 关联类型名）→ 建议 object-backed link | ontology-structural-guidance.md |
| ONT-W14 | W | 为安全拆类型：成对出现 `Public*`/`Restricted*` 且 schema 高度重叠 → 用行/列级策略 | ontology-structural-guidance.md |
| ONT-W15 | W | 安全策略按名称引用 group（应用稳定 group ID） | persisted-scenario.md |
| ONT-I01 | I | ≥3 个属性共享前缀（`addressStreet` `addressCity` …）→ 建议 struct + main field | ontology-structural-guidance.md |
| ONT-I02 | I | 缺 description / aliases 的类型、属性、link、action | ontology-best-practices.md, ontology-design-validation.md |
| ONT-I03 | I | Golden Hammer：function-backed 列只依赖同一对象的源列（如拼接）→ 建议上游 pipeline 预计算 | ontology-anti-patterns.md |
| ONT-I04 | I | 非语义技术类型未标 hidden | ontology-best-practices.md |
| ONT-I05 | I | 派生属性用 exactDistinct 时提示 approximateDistinct；>10k 对象规模下派生属性做 filter/sort 时提示成本 | derived-properties-reference.md, ontology-structural-guidance.md |
| ONT-I06 | I | 新增 link / 属性跨越受限数据边界时提示访问控制一致性复核 | ontology-structural-guidance.md |

另：任务式验证（drill）不可自动化，但可提供 "question suite" 回归机制：把业务问题存为带期望答案的 object set 查询，每次 schema 变更后重跑，记录耗时与路径 [ontology-design-validation.md]。

---

## 5. 优先级建议

判断依据：(a) 没有它核心不可用；(b) 后加时代价高（数据模型/存储格式层面）；(c) 文档自身标为 Beta/Experimental 的降级处理。

### MVP（可用核心）
1. **Schema 层（ontology-as-code）**：object type（PK、title、description、aliases、hidden）、属性 base types 子集（数值 7 种、string、boolean、date、timestamp、array、geopoint）、link type（1:1、1:N FK、M:N join 表，双向 API name）、action type 定义。理由：一切的地基；Foundry 自身也在走 ontology-as-code（superrepo）。
2. **Lint 引擎**：第 4 节全部 E 级 + W01-W11。理由：纯静态规则、成本低、是本项目相对 Foundry 的差异化卖点。
3. **Object set 查询**：base / filter（2.1 全部运算符，full-text 用 tantivy 类倒排索引）/ sort / 游标分页 / `pivotTo` / 集合运算 / 聚合（`$count` sum avg min max approximateDistinct exactDistinct，groupBy exact / fixedWidth / ranges / duration）。理由：读路径是全部消费者（UI、SDK、MCP）的公共依赖。
4. **Action 执行**：参数（类型、required、默认值、简单约束）、规则（create / modify / delete / link add/remove，值来源 = 参数 / 静态值 / 当前用户）、submission criteria（用户、group、对象当前属性）、validate-only、batch all-or-nothing、标准化 edits 响应、**不可变 edit log（审计）**、"仅允许经 action 编辑"开关。理由：kinetic 层是 Ontology 区别于 schema registry 的核心；edit log 若以事件溯源实现，后续 scenario 与 branch 可直接复用 overlay 机制，后加成本高，所以 MVP 就定格式。
5. **API + 客户端**：REST（对齐 Foundry v2 wire 格式，见 1.12）+ 生成的 Rust 与 TypeScript 客户端。
6. **Ontology MCP server**：describe schema、search/SQL 读工具、每个 action 一个工具（含 agent tool description）、每个 query function 一个工具。理由：agent 消费是当前主战场，且只是第 3、4、5 项的薄封装。
7. **基础权限**：ontology 级与资源级 roles；action 执行权限。理由：写路径没有权限就无法安全暴露给 agent。

### v2
- **Interfaces**（属性映射、extends、interface object set 查询、面向 interface 的 action）与 **shared properties**：best practices 的核心手段，但依赖 MVP schema 稳定后再加；schema 格式在 MVP 预留 `implements` 字段。
- **Struct**（main field、struct array）与 **property reducers**。
- **Runtime 派生属性**（`withProperties`：link select、聚合含 collectList/Set、表达式）；ontology-defined 派生属性放后（Foundry 自身为 Beta）。
- **Value types**（约束集见 1.5：enum / range / regex / RID / UUID / uniqueness / nested / struct element；版本化 + 索引时校验 + action 参数校验）。
- **行级 / 列级（cell 级）安全**，含派生属性逐跳权限检查与读时强制。
- **Vector 属性 + KNN**（dimension、similarity、`nearestNeighbors` 1-500、relevance score）+ regex 索引 + RRF 混合检索辅助。
- **Scenarios**：temporary + persisted（基于 edit-log overlay）、Apply Scenario rule 单事务合并、编辑枚举 API、TTL；按执行上下文区分 submission criteria。
- **Edit functions / query functions 运行时**（WASM 或外部进程插件），function-backed action rule。
- **订阅**（WebSocket，对齐 1.12 / 3.1 协议）。
- **Schema branching + proposals**：仅限 ontology 资源（branch、rebase 含字段级冲突检测、checks、审批策略、Do not merge）。

### Out-of-scope（至少 v2 之前不做）
- Workshop / Quiver / Object Explorer / Slate / Carbon / Map 等应用层 [applications.md]：前端产品，非 ontology 核心。
- Pipeline Builder、数据集事务、构建调度、Global Branching 的跨应用部分 [gb/integrations.md]：属于数据集成平台。
- AIP Logic、模型部署与 live inference [models.md]、文档解析/分块管线 [document-processing.md]：可作为外部插件对接。
- Markings、cipher、classification-based access control、media sets、time series 与 geotemporal series：安全/存储子系统规模大，按需求单独立项。
- Release management、Marketplace 安装、Developer Console UI。
- 自动 writeback 到 ERP 等外部系统的原生连接器：MVP 只提供 webhook side effect 扩展点。

### 下一步阻塞项
- 参考页已补抓，原 `[GAP]` / `[推断]` 均已按原文改写。剩余未决（不阻塞 phase 规划）：
  - TSv2 / OSDK 的 geo 与 fuzzy 过滤运算符名（`typescript-v2-migration` 的 filter operator mapping 未镜像）。
  - 函数执行限制具体数值（`functions/manage-functions` 的 enforced limits 未镜像）。
  - Object edits 应用机制、edit history、materialization（`object-edits/` 节未镜像，与 action log / MDO 编辑相关）。
  - Restricted view 策略语法全集（`security/restricted-views` 未镜像）。
