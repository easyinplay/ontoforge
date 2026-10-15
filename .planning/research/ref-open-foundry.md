# 参考项目调研：syzygyhack/open-foundry

> 调研日期 2026-10-15。数据源：`gh api`（repo 元数据、git tree、文件内容），未 clone、未执行代码。
> 仓库：https://github.com/syzygyhack/open-foundry（commit 快照 = 2026-10-10 HEAD）

## 1. 概要与成熟度

| 项 | 值 |
|---|---|
| 定位 | "open-source ontology platform for operational digital twins"，类 Palantir Foundry Ontology 的开源实现 |
| License | Apache-2.0 |
| Stars / Forks | 120 / 34 |
| 创建 / 最近 push | 2026-02-09 / 2026-10-11 |
| Release | v0.1.0-rc.1 (2026-05) → v0.2.0 (2026-08-23) → v0.3.0 (2026-09-07)，pre-1.0，minor 可含 breaking |
| 贡献者 | 实质单人（`syzygyhack` 285 commits，另 1 人 1 commit） |
| 代码量 | TypeScript ~2.49 MB，Go ~39 KB（CEL sidecar） |
| 来源 | `docs/project-history.md`：Phase 1 由 agent pipeline（"Cardinal" + Claude Opus 4.6）按 spec 拆 ~120 task 自动生成，Phase 2 人工 + cross-model review 加固（自称修 200+ 问题） |
| 自评 | README："pre-1.0 ... suitable for evaluation, prototyping and controlled pilots"，无独立安全审计 |

结论：spec 完整度远高于实现完整度；是一个**设计参考价值大于代码参考价值**的项目。核心规范文档 `docs/open-foundry-spec-v2.md`（149 KB，14 章 + 4 附录）是最值得借鉴的产物。

## 2. 技术栈

- 语言：TypeScript（Node 24，pnpm workspace + turbo monorepo，vitest），CEL 评估器用 Go（`packages/cel-evaluator/`，gRPC，`proto/cel_service.proto`）。
- API：Apollo GraphQL、Express 风格 REST、WebSocket subscriptions、FHIR R4 facade。
- 存储：PostgreSQL 17 + Apache AGE（生产）、in-memory（测试），共享 SPI conformance suite。
- 授权：OpenFGA（Zanzibar ReBAC）；认证：OIDC（Keycloak realm 示例 `deploy/keycloak/openfoundry-realm.json`）。
- 事件：in-memory bus 或 Redpanda/Kafka（`packages/api/src/events/redpanda-event-bus.ts`），CloudEvents 格式。
- 限流：in-memory / Redis（`packages/api/src/governance/redis-rate-limiter.ts`）。
- CDC：Debezium（`packages/sync/src/cdc/cdc-consumer.ts`）。
- 可观测：OpenTelemetry traces + Prometheus metrics（`packages/observability/`）。
- 部署：docker-compose（`deploy/docker-compose.yaml`）+ Helm chart（`deploy/helm/openfoundry/`，含 HPA / PDB / NetworkPolicy / ServiceMonitor）。服务拆分：api-gateway、ontology-engine、action-executor、security-service、sync-engine、cel-evaluator。

## 3. 仓库布局

```
packages/
  spi/              核心类型契约（OntologyObject/Link、StorageProvider、Transaction、Audit、Consent、Provenance）
  odl/              ODL parser / validator / diff+classify / codegen(GraphQL, OpenFGA, SDK) / registry / CLI `odl`
  engine/           object-manager、link-manager(UUIDv7)、validation、computed fields、lineage、events、object-sets
  actions/          YAML manifest parser、action-executor(8 步 pipeline)、CEL client、side-effects、tool-registry(AI)
  api/              GraphQL resolver-generator、REST route-generator、OpenAPI/AsyncAPI 生成、FHIR、CDM、consent、governance
  security/         OIDC authenticator、role-mapping、authorization-service(OpenFGA)、consent-service、audit
  storage-memory/   in-memory provider
  storage-postgres/ DDL 生成、object/link CRUD、filter-to-sql、aggregate、search(tsvector)、temporal、traversal、schema-registry
  sync/             connectors(JDBC/REST/identity)、mapping+transforms、overlay、conflict resolver、CDC
  cel-evaluator/    Go gRPC sidecar
  sdk-typescript/   占位（src/index.ts 仅 `export {}`）
domain-packs/       core / aml / supply-chain / nhs-acute（schema/*.odl + actions/*.yaml + permissions/*.fga + connectors/*.yaml）
examples/library-pack/  教程用最小 pack
tests/              spi-conformance(10 类) / integration(Docker stack) / pilot-scenarios
docs/               spec-v2、first-domain-pack、external-domain-packs、api-spec、nhs/
```

## 4. 本体定义方式：ODL = GraphQL SDL + 语义 directive

所有定义写成合法 `.graphql`（扩展名 `.odl`），按 Domain Pack 组织，行为另放 YAML。四类文件分工：

| 文件 | 内容 | 例 |
|---|---|---|
| `schema/*.odl` | ObjectType / LinkType / ActionType 签名 / enum / interface / scalar | `domain-packs/nhs-acute/schema/patient.odl` |
| `actions/*.yaml` | Action 行为：preconditions / effects / sideEffects / rollback / undo | `domain-packs/nhs-acute/actions/transfer-ward.yaml` |
| `permissions/*.fga` + `field-permissions.yaml` | OpenFGA 模型扩展 + 字段级可见性 | `domain-packs/nhs-acute/permissions/` |
| `connectors/*.yaml` | 外部数据源映射与 sync 模式 | `domain-packs/nhs-acute/connectors/pas-jdbc.yaml` |
| `pack.yaml` | name/version/namespace/dependencies/capabilities/文件清单 | `domain-packs/nhs-acute/pack.yaml` |

命名空间：`extend schema @namespace(name: "nhs.acute", version: "0.2.0")`；pack 依赖 `openfoundry.core: ">=1.0.0"`。

### 4.1 Directive 全集（`packages/odl/src/parser/types.ts`，spec §2.3 / 附录 C）

- Type 级：`@objectType`、`@linkType(from, to, cardinality)`、`@actionType(permission?)`、`@function(runtime, entry)`、`@constraint(expr)`（`this` 指对象）、`@deprecated(reason)`。
- Field 级：`@primary`、`@unique`、`@indexed`、`@readonly`（引擎管理）、`@immutable`（创建后冻结）、`@sensitive`、`@param`（Action/Function 入参）、`@link(type, direction: INBOUND|OUTBOUND, history?)`、`@computed(fn, args?, cache: LAZY|EAGER|TTL, ttl?)`、`@constraint(expr)`（`value` 指字段值）、`@default(value)`、`@terminology(system)`、`@searchable(weight?, analyzer?)`。
- Cardinality：`ONE_TO_ONE | ONE_TO_MANY | MANY_TO_ONE | MANY_TO_MANY`，只约束 **active** link（已终止 link 不计）。

示例（README）：

```graphql
type Book @objectType {
  id: ID! @primary
  title: String! @indexed @searchable(weight: 2.0)
  status: BookStatus!
  borrower: Member @link(type: "BorrowedBy", direction: OUTBOUND)
}
type BorrowedBy @linkType(from: "Book", to: "Member", cardinality: MANY_TO_ONE) {
  id: ID! @primary
  borrowedAt: DateTime!
}
type BorrowBook @actionType(permission: "can_borrow") {
  book: Book! @param
  member: Member! @param
}
```

### 4.2 AST（`ParsedSchema`）

`{ namespace?, objectTypes[], linkTypes[], actionTypes[], enums[], interfaces[], scalars[] }`。`FieldTypeRef = { name, nonNull, isList, listElementNonNull }`。注意：**没有 `functions[]`**，`@function` 只在 directive 层被识别（`parser/index.ts:354`），无对应 AST 集合与运行时。

### 4.3 Core pack（`domain-packs/core/schema/core.odl`）

Scalar：`Date DateTime Duration GeoPoint JSON URI`；Interface：`Identifiable{id}`、`Auditable{createdAt,createdBy,updatedAt,updatedBy @readonly}`、`Locatable{location,address}`、`Temporal{validFrom,validTo}`、`CodeableConcept{system,code,display}`。Interface 语义 = GraphQL `implements`，只是共享形状，**不是** Foundry 那种可被多态查询 / 绑定 Action 的 Interface。

## 5. 核心领域模型（运行时 / SPI，`packages/spi/src/ontology.ts`）

- `OntologyObject`：系统字段 `_tenantId _type _id _version _createdAt _updatedAt _deletedAt?` + 任意属性（`[key]: unknown`）。
- `OntologyLink`：`_tenantId _type _id _fromType _fromId _toType _toId _version _createdAt _updatedAt _deletedAt?` + link 属性。**Link 是带 id 和属性的一等实体**（同一对对象可多次 link，如重复入院），link id 由 Engine 用 UUIDv7 生成（`packages/engine/src/links/uuidv7.ts`），SPI 不得生成。
- 运行时 schema 精简表示：`OntologySchema{version, objectTypes: {name, properties: PropertyDefinition{name,type,required,defaultValue,description}[], indexes}, linkTypes: {name, fromType, toType, cardinality, properties}}`。即 ODL AST → 存储层只看到"降维后的"定义。
- `FilterExpression` = `FieldPredicate{field, operator: eq|neq|gt|gte|lt|lte|in|contains|startsWith|exists, value}` | `LogicalPredicate{and, or, not}`。
- `TraversalPath{steps: {linkType, direction, filter?, maxDepth?}[]}` → `TraversalResult{nodes, edges, totalCount}`。
- `QueryOptions{limit, offset, orderBy, includeDeleted, asOfVersion, asOfTime}`：时态查询直接进 query options。
- `AggregateQuery{fields: {field, fn: count|sum|avg|min|max, alias}[], groupBy, filter, orderBy}`；`SearchQuery{query, fields, filter}` → `SearchHit{object, score, highlights}`。
- `ObjectSetDefinition`（`spi/src/object-set.ts`）：持久化的命名 filter+sort+aggregation，`isPublic`，租户隔离。对应 Foundry Object Set，但只是"保存的查询"，无集合代数（union/intersect/searchAround）。
- `FieldProvenance`（`spi/src/provenance.ts`）：`{objectType, objectId, field, valueHash, producedAt, source}`，source 为 `ACTION | SYNC | FUNCTION` 判别联合。字段级血缘，值得借鉴。
- `StorageCapabilities`：`supportsTransactions / TemporalQueries / FullTextSearch / GeoQueries / GraphTraversal / BulkMutations, maxTraversalDepth, replicationSupport`。API 生成依 capability 裁剪（如无全文检索则不生成 `searchFoos`）。

### 5.1 StorageProvider SPI（`packages/spi/src/storage-provider.ts`）

所有方法首参 `ctx: RequestContext{tenantId, actorId?, traceId?}`，强制租户隔离。分组：Schema（`applySchema/getSchema`）、Objects（CRUD + `queryObjects/aggregateObjects/searchObjects/bulkMutate`，`updateObject` 带 `expectedVersion` 乐观锁）、Links（CRUD + `getLinks(objectId, linkType, direction)` + `traverse`）、`beginTransaction`、Versioning（`getObjectAtVersion/AtTime`）、Indices、`healthCheck/capabilities`。`Transaction` 只暴露写操作 + `commit/rollback`。

## 6. Action（唯一写路径）

**设计核心：没有通用 CRUD 写 API，所有 mutation 必须走 Action**，从而 preconditions / 授权 / audit 不可绕过（README "There is no generic create/update/delete path"）。例外：boot 时 seed 数据绕过 pipeline 且不审计。

Manifest 结构（`packages/actions/src/parser/types.ts`）：

```
ActionManifest { action, version, reversible,
  preconditions: {expr(CEL), error}[],
  effects: (updateObject{target,set,condition} | createObject{objectType,properties}
           | createLink{linkType,from,to,properties,condition}
           | deleteLink{linkType, filter{from,to,active}, expect: ONE|ALL}
           | recordConsent{subject,purpose,decision,evidence,condition})[],
  sideEffects: {name, type(event/webhook...), config, retries, retryDelay}[],
  rollback: {onSideEffectFailure: LOG_AND_CONTINUE|RETRY_INDEFINITELY|ROLLBACK_ALL},
  undo?: {overrides[{effect, undoEffect}], sideEffects, window} }
```

执行 pipeline（`packages/actions/src/executor/action-executor.ts`，spec §5.3）：Validate → Authorise(ReBAC) → Consent → Preconditions(CEL) → Execute(单 SPI 事务，按 manifest 顺序) → Side-effects(inline retry；spec 要求生产用 Temporal) → Audit → Emit CloudEvents。

关键语义（可直接借鉴）：
- **授权先于 preconditions**：防止通过 precondition 错误信息探测对象状态（spec §5.3 rationale）。
- **效果求值基于 action 开始时的不可变快照**：`patient.currentWard` 在后续 effect 中仍指原 ward，不随 effect 重绑定。
- `deleteLink` 用 filter 解析为具体 link id，`expect: ONE` 不为 1 则 `LINK_RESOLUTION_AMBIGUOUS/LINK_NOT_FOUND`，解析结果入 audit。
- 表达式统一 CEL：变量 `params / actor / now` + 每个 `@param` 作为已解析对象；函数 `has_link / count_links / actor.hasRole / actor.hasPermission / duration()`。ODL 类型 → CEL 类型映射见 spec §5.2.3。CEL 权威求值必须走 canonical 引擎（Go sidecar 或 WASM），纯 TS 实现只用于 lint（spec §5.2.4）。
- 权限名推导：`@actionType(permission:)` 显式，否则 "动词 + 类型名" 推导（`TransferWard` → `can_transfer`），注释承认推导有歧义风险。

## 7. AI / Agent 面（spec §5.7，`packages/actions/src/tools/`）

- GraphQL `availableTools(filter)` 返回 `ToolDescriptor{name, kind: ACTION|QUERY|FUNCTION, description, parameters(JSON Schema), returnType, requiredPermissions, dryRunSupported, reversible}`，声称与 OpenAI function-calling / Anthropic tool-use 兼容。
- `AgentContext{agentId, sessionId, dryRun, model}`；`PolicyGuard.evaluate(actionName, riskLevel: low|medium|high, ctx)` → 高风险 action 进 `PENDING_APPROVAL`（human-in-the-loop）。dry-run 返回预计 effects 不提交。
- **没有 MCP server**；只是 GraphQL 里的 tool 发现 + action API 加 agent header。

## 8. 存储与查询引擎（`packages/storage-postgres/`）

- **每 ObjectType 一张表 + 一张 `_history` 表**（`schema/ddl-objects.ts`）：系统列 `_tenant_id,_id,_type,_version,_created_at,_updated_at,_deleted_at`，PK `(_tenant_id,_id)`；history 表 `_history_id IDENTITY` + 全列副本，索引 `(_tenant_id,_id,_version)`，支撑 `asOfVersion/asOfTime`。
- **每 LinkType 一张表**（`schema/ddl-links.ts`），`_from_type/_from_id/_to_type/_to_id` 双向索引；**cardinality 用 partial unique index（`WHERE _deleted_at IS NULL`）在 DB 层强制**，如 MANY_TO_ONE → `UNIQUE(_tenant_id,_from_id)`。简洁有效，值得照抄。
- 类型映射（`schema/type-mapping.ts`）：String→TEXT、Int→INTEGER、Float→DOUBLE PRECISION、DateTime→TIMESTAMPTZ、Duration→INTERVAL、JSON→JSONB、enum/未知→TEXT。字段名 camelCase→snake_case。
- 全文检索：`@searchable` → GIN `to_tsvector('english', col)`（`objects/search.ts`）。
- Apache AGE：只创建 graph `openfoundry` 及每类型 vlabel/elabel（`schema/ddl-graph.ts`）；**实际 traversal 是逐 step 的 SQL `SELECT ... WHERE _from_id IN (...)`（`links/traversal.ts`），文件头声称用 Cypher 但代码未用**。限 depth 10 / nodes 10000。
- 血缘表 `lineage.field_provenance`（`schema/ddl-lineage.ts`），source JSONB。
- Schema registry：`_schema_registry` 表存版本化 `ParsedSchema` 快照，advisory lock 串行化版本分配（`schema-registry/postgres-schema-registry.ts`）。
- Schema 演进：`odl` CLI `validate / diff / apply / generate graphql|openfga / rollback`（`packages/odl/src/cli/index.ts`）；diff 分类 safe / compatible / breaking，breaking 需 YAML migration plan（prechecks / transforms / verification 全 CEL，spec §2.5.2）；rollback = 生成 reverse diff 作为新的更高版本（forward-only）。
- SPI conformance suite（`tests/spi-conformance/src/categories/`）10 类：crud / links / queries / aggregation / search / temporal / transactions / schema / lineage / multi-tenancy。是可移植的"存储后端验收标准"思路。

## 9. API 面

- **GraphQL**（`packages/api/src/graphql/resolver-generator.ts`，SDL 产物 `packages/api/spec/schema.graphql`）：每 ObjectType 生成 `foo(id)`、`foos(filter, orderBy, first/after/last/before)` Relay connection、`searchFoos`、`searchAll`、`typeahead`；每 ActionType 生成 mutation；subscriptions `fooChanged(id)` / `foosChanged(filter)`（服务端过滤 + ReBAC 过滤 + `resumeAfter` 重放）。Filter input 自动生成，含 `AND/OR/NOT`。
- **为字段级脱敏，生成的 GraphQL 中所有非 primary 字段一律 nullable**，非空约束只在 Engine 写入时校验；每个类型附 `_redactedFields: [String!]` 与 `_consentRestricted: Boolean`（spec §7.1.3）。避开 GraphQL null-bubbling 的好做法。
- **REST**（`packages/api/src/rest/route-generator.ts`）：`GET /api/v1/{plural}`、`/{plural}/:id`、`/:id/links/:linkType`、`/:id/history`、`/{plural}/aggregate`、`/{plural}/search`、`POST /api/v1/actions/{Action}`、`/api/v1/object-sets[/:id[/execute|/aggregate]]`、`/api/v1/relationships`（FGA tuple 授予）、`/api/v1/openapi.json`、`/health`（聚合子系统 + circuit breaker 状态）。乐观锁：REST `If-Match` = `_version`。
- 契约产物：OpenAPI（`packages/api/spec/openapi.yaml`，161 KB）、GraphQL SDL、AsyncAPI（事件），随 release 发布。
- Capability-gated facade：FHIR R4（`/fhir/*`，只读）和 CDM 投影（`/api/v1/cdm/*`），仅当 pack 声明 `capabilities: [fhir, cdm]` 时挂载。
- 事件：CloudEvents，type `openfoundry.{object|link}.{created|updated|deleted}`、`openfoundry.action.{submitted|completed|failed}`、`openfoundry.schema.updated`；分区键 `(tenantId, objectType, objectId)`，同一 action 的事件共享 `causedBy.actionId` 并原子发布（spec §4.2.1）。
- SDK：`packages/sdk-typescript` 仅占位；`odl/src/codegen/sdk.ts` 有生成器但未发布。

## 10. 权限与治理

- **ReBAC via OpenFGA**：模型由 ODL 自动生成（`packages/odl/src/codegen/openfga.ts`）：每 ObjectType → FGA type 带 `viewer/editor`；`@link` → relation（如 `admitted_to: [ward]`）并 `viewer from admitted_to` 沿图传递；每 ActionType → `can_<verb>` relation。pack 的 `.fga` 文件可覆盖/扩展。效果：病人换病区，可见性自动随图变化。
- 四层权限：schema 级、object 级（图派生）、action 级、field 级（`field-permissions.yaml`：`objectType / alwaysVisible / fieldsByRelation{relation: [fields]}`；一旦为某类型配置，未列字段默认隐藏，fail-closed）。
- `@sensitive` 只是意图标记，真正脱敏要靠 field-permissions 配置（README 明确提醒）。
- 列表查询用 OpenFGA `ListObjects` 预计算可见 id 集合再求交，避免 N×M Check（spec §7.1.5）。
- Consent（`packages/spi/src/consent.ts`）：purpose 为开放字符串（NHS 预设 `DIRECT_CARE` 等），`ConsentDecision{allowed, purpose, basis, restrictions[]}`，支持 batch check、revoke；pipeline 中独立于授权的一步，错误码 `CONSENT_DENIED`。
- Audit（`packages/spi/src/audit.ts`）：`AuditRecord{id, timestamp, traceId, actor{type,id,roles,ip}, operation{type,objectType,objectId,actionType,actionId}, detail{before,after,result,denialReason,consentDecision}}`，append-only（PG 表不授 UPDATE/DELETE）。
- 多租户：每请求必带 `tenantId`，所有表 PK 含 `_tenant_id`，FGA tuple 按租户隔离。
- 生产 fail-closed：production boot 校验配置与 FGA 模型否则退出；OpenFGA circuit open 时一律 deny。**但 dev 模式（默认 compose）把 OpenFGA 和 CEL 换成 allow-all stub，并给匿名请求合成 admin**。
- API governance：query complexity gate、rate limiter、execution guard（`packages/api/src/governance/`）。

## 11. Sync / 数据集成（`packages/sync/`）

Connector YAML：`datasource / connector: jdbc|rest / connection{url, table} / mapping{objectType, primaryKey{source,target,transform}, properties{prop: {source, transform}}} / sync{mode: OVERLAY|CDC, cacheStrategy, cacheTTL, writeback, conflictResolution}`。transform 为小 DSL（`prefix()`、`concat()`、`parseDate()`、`ifPresent()`，见 `mapping/transforms.ts`）。Overlay 模式 = 不落地、按 TTL 缓存读穿；冲突策略 `LAST_WRITE_WINS | SOURCE_PRIORITY | ACTION_PRIORITY`，可按字段配置。identity resolution 在 `connectors/identity.ts`。

## 12. 值得借鉴的设计决策（对 Rust FoundryOntology）

1. **Schema-first + 单一来源生成全部表面**：一份 schema → DDL、GraphQL、REST、OpenAPI/AsyncAPI、FGA 模型、tool descriptor。Rust 侧可用 build-time codegen 或运行时动态 schema（async-graphql dynamic schema）。
2. **Action 作为唯一写路径** + 声明式 effects（5 种 effect 原语）+ 不可变快照求值 + 单事务 + 补偿/undo 窗口。
3. **授权先于 precondition** 防信息泄露；生成 API 时非 primary 字段一律可空 + `_redactedFields`。
4. **Link 一等公民**：有 id、有属性、有 history、cardinality 只计 active，DB 层 partial unique index 强制。
5. **存储 SPI + capability 描述 + conformance suite**：Rust trait `StorageProvider` + `capabilities()`，一套 conformance 测试同时跑 memory 与 PG 后端。
6. **系统字段约定** `_tenant_id/_id/_type/_version/_created_at/_updated_at/_deleted_at` + per-type history 表实现 `asOfVersion/asOfTime`。
7. **CEL 统一表达式语言**（precondition / constraint / migration / quality rule）：Rust 有 `cel-interpreter` crate，可省掉 Go sidecar，直接进程内求值（这是 open-foundry 因 TS 而付出的架构代价）。
8. **字段级 provenance**（ACTION/SYNC/FUNCTION 三源）、CloudEvents + `causedBy.actionId` 关联。
9. **Domain Pack** 打包与 `capabilities` opt-in 挂载 facade；schema diff 三级分类 + forward-only rollback。
10. AI tool descriptor 直接从 ActionType 派生 JSON Schema + dryRun + PolicyGuard 人审。

## 13. 缺失 / 薄弱点（实现与 spec 的落差）

- **Functions 未实现**：`@function` 仅解析 directive，`ParsedSchema` 无 functions，所有 pack `functions: 0`；spec 要求的 WASM/V8 沙箱不存在。
- **Computed fields 仅 LAZY + 单一内建 `countLinks`**（`packages/engine/src/computed/computed-field-evaluator.ts`），EAGER/TTL、依赖追踪未做。
- **图查询名不副实**：AGE 只建 label，traversal 走多次 SQL hop；无图查询语言暴露给用户，无 "search around"。
- **Object Set 只是保存的查询**，无集合运算、无物化、无跨类型。
- **Interface 只是 GraphQL 形状复用**，无多态 Object Set 查询、无 interface 级 link/action。
- **无 Value Type / Struct / 数组属性语义 / 地理与时间序列属性 / 媒体引用**；类型映射未知一律 TEXT。
- **无 MCP server**，无 Python SDK，TS SDK 为占位。
- Side-effect 无 Temporal（inline retry）；Backup/Restore、Federation、Schema workspaces、Data quality rules 均 spec-only。
- FHIR 只读；Admin UI / 应用框架未做。
- dev 模式默认 allow-all，容易误判安全效果；单人维护 + agent 生成代码，bus factor 低。
- 运行时 schema（`OntologySchema`）与 ODL AST 两套表示，存储层丢失 directive 信息（`ddl-objects.ts` 注释承认 @indexed/@unique 需调用方补 IndexDefinition）。

## 14. 对本项目的建议取舍

- 照搬：系统字段约定、link 表 + partial unique cardinality、history 表、Action manifest 结构与 pipeline 顺序、FilterExpression/TraversalPath/AggregateQuery 形状、SPI + conformance、ReBAC 从 link 派生、`_redactedFields`、CloudEvents 格式、ToolDescriptor。
- 改进：CEL 进程内（Rust crate）；schema 单一内部表示（不要 AST 与运行时 schema 两套）；真正的 Function 运行时（WASM，如 wasmtime）；Object Set 代数；一等 Interface；MCP server 作为一级 API；dev 模式也保持授权逻辑可开启。
- 待决（交 discuss 阶段）：ODL 采用 GraphQL SDL 方言 vs YAML/JSON vs Rust DSL；授权自研 vs 接 OpenFGA；图存储走 PG 递归 CTE vs 嵌入式图引擎。
