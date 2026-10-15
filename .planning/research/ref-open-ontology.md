# 参考调研：bjacobso/open-ontology

> 调研日期：2026-10-15 ｜ 方式：`gh api`（只读，未 clone、未执行代码）｜ 仓库：https://github.com/bjacobso/open-ontology
> 结论一句话：一个 **v0.1 实验性 TypeScript 包**，核心价值在于「双作者面（TS DSL + Lisp DSL）→ 单一 JSON-safe IR → 可替换后端（Triplex 三元组 + Datalog）」这一分层，以及面向 agent 的建模校验器；持久化、权限、API、Function、Interface 全部缺失。

## 1. 成熟度

| 指标 | 值 |
| --- | --- |
| Stars / Forks | 16 / 0 |
| Commits（main） | 13 |
| 创建 / 最后 push | 2026-09-28 / 2026-10-13（`01c03ce` Upgrade Effect to 4.0.3） |
| License | MIT |
| 版本 | `@open-ontology/ontology` 0.1.0，未发布 npm |
| Open PR | #5 WorldVM 定位、#9 skills 原语、#11 prelude 目录页、#13 落地页、#14 **编译到 Foundry Ontology as Code**（`@osdk/maker`） |
| 前身 | README 称改编自 `bjacobso/open-ontology-legacy` commit `7c2ff8a` 的 `packages/ontology`（该仓库 API 返回 404，不可见/私有） |
| 定位 | WorldVM 实验家族成员；网站 open-ontology.com（Cloudflare Worker 静态站），主读者是「替人建模的 agent」 |

README 原文：`Maturity: **experimental (v0.1)**. One package, four executable examples, and a local in-memory runtime; no durable server or npm release yet.`

## 2. 技术栈

- 语言：TypeScript ESM（strict），Node >= 22，pnpm 10.11.0；构建 `tsdown`，测试 `vitest`，运行示例 `tsx`。
- 核心依赖（`package.json`）：
  - `effect` 4.0.3：Effect Schema 作为属性/动作输入类型系统，Effect 作为运行时效应模型。
  - `@triplex-build/triplex` 0.2.0：三元组（EAV）存储 + Datalog 查询引擎，带事务元数据；仓库自带 patch `patches/@triplex-build__triplex@0.2.0.patch`（cursor 编码迁移到 `Base64Url`）。
  - `@formalang/ts` 0.3.0：Forma（一种 Lisp 方言）reader + descriptor/elaboration API。
- 语言统计里 NewLisp/Common Lisp 约 127KB，实为 `.lisp` 的 Forma 源（preludes + libraries）。

## 3. 仓库布局

```
src/ir.ts            JSON-safe IR 类型（契约中心）
src/model.ts         TS 侧带类型的 Definition（Effect Schema + branded id）
src/dsl.ts           TS 作者 DSL：Ontology.ObjectType/LinkType/ActionType/QueryType, Property, Change, Input, Datalog
src/datalog.ts       类型化 Datalog 构造器（Datalog.variables/match/predicate/not/query）
src/forma.ts         Forma 源 → IR（elaborateFormaOntology），只接受 5 种 form
src/forma-libraries.ts  内置库注册表（/std/git, /github），import 展开
src/materialize.ts   IR → 可执行 OntologyDefinition（含 JSON Schema → Effect Schema 还原与可移植性校验）
src/runtime.ts       Triplex 适配：toTriplexConfig、makeOntologyRuntime（invoke / query）
preludes/*.lisp      Forma 语言自身定义（define-form、meta-fn、define-elaboration），运行时加载
libraries/std/git.lisp, libraries/github.lisp   可 import 的领域库
scripts/check-model.ts  模型检查器（file:line:col 诊断 + 语义 warning，--json）
examples/{field-service,support-desk,github,onboarded}
apps/site/public/llms.txt, llms-full.txt   给 agent 的建模指南（被测试校验）
test/*.test.ts       含 TS/Forma 等价性、agent 指南可执行块、Effect 兼容性
```

## 4. 核心领域模型（`src/ir.ts`）

只有 **4 种声明**：`OntologyDeclarationIR = ObjectTypeIR | LinkTypeIR | ActionTypeIR | QueryTypeIR`。

```ts
OntologyIR { kind:"ontology"; name; version; objectTypes[]; linkTypes[]; actionTypes[]; queryTypes[] }

PropertyIR { name; key: `:${ns}/${attr}`; valueType; required; cardinality:"one"|"many"; unique }
OntologyValueTypeIR = {kind: "string"|"number"|"boolean"|"datetime"|"json"} | {kind:"ref"; target}

ObjectTypeIR { kind:"object-type"; name; description?; properties[] }
LinkTypeIR   { kind:"link-type"; name; from; to; description?; properties[] }   // 链接可带属性

ActionTypeIR { kind:"action-type"; name; description?; input: ActionInputIR[];
               inputSchema?: JsonSchema.Document<"draft-2020-12">; changes: ActionChangeIR[] }
ActionInputIR { name; valueType; cardinality?; optional? }
ActionValueIR = {kind:"input"; name} | {kind:"now"} | string | number | boolean | null
ActionChangeIR = create-object{objectType,id,properties}
               | set-property{object,property,value}
               | clear-property{object,property}
               | create-link{linkType,from,to,properties}

QueryTypeIR { kind:"query-type"; name; description?; query: DatalogQueryIR }
DatalogQueryIR { find; where; aggregate?; having?; orderBy?[{variable,direction}]; limit?; offset?;
                 rules?[{name, body, maxDepth?}]; optionalProjection? }
```

与 Foundry 概念对照：

| Foundry | open-ontology | 备注 |
| --- | --- | --- |
| Object Type | `ObjectTypeIR` / `define-entity` | 无 primary key 声明，id 是外部给的字符串；无 title property |
| Property | `PropertyIR`，key 形如 `:ticket/status` | 命名空间化属性 key（Datomic 风格），类型 6 种 + `cardinality:"many"`，无 struct、geo、vector、attachment |
| 外键 Link | `ref` 类型属性（`ticket/customer Customer`） | 单向引用 |
| Link Type（多对多） | `LinkTypeIR` / `define-relation` | **链接被具象化为实体** `XxxLink`，端点属性 `:x/from`、`:x/to`，可挂属性 |
| Action Type | `ActionTypeIR` | 纯声明式 change 列表，无条件、无计算值、无删除/unlink、无 side effect |
| Function | 无 | 只有命名 Datalog 查询，且**无参数** |
| Interface / Shared Property | 无 | |
| Object Set | 无 | 查询结果是行，不是对象集合 |
| Value Type / Struct | 无 | `json` 兜底 |

## 5. 定义表达方式（双作者面 → 同一 IR）

**Forma（Lisp）** `examples/support-desk/model.lisp`：

```lisp
(define-entity Ticket
  (:field [ticket/title String {:required true}])
  (:field [ticket/customer Customer {:required true}]))   ; 非标量类型名 = ref
(define-relation escalated-to Ticket Engineer
  (:field [escalated-to/reason String {:required true}]))
(define-action escalate-ticket
  (:input [ticket String {:required true}]) (:returns String)
  (:do (changes (clear ticket :ticket/status) (set ticket :ticket/status "escalated")
                (link escalated-to ticket engineer {:escalated-to/reason reason :escalated-to/at now}))))
(define-datalog-query tickets-by-status
  (:query {:find ["?status" "?count"] :where [["?t" ":ticket/status" "?status"]]
           :aggregate [["count" "?t" "?count"]]}))
```

类型映射（`src/forma.ts` `scalarType`）：`String/Symbol/Ref→string`、`Int/Float/Number→number`、`Bool/Boolean`、`Instant/DateTime→datetime`、`Json/Any→json`、其他名字 → `ref`；`(List X)`/`(Set X)` → `cardinality:"many"`。

**TypeScript DSL** `examples/field-service/model.ts`：`Ontology.ObjectType("WorkOrder", {properties:{status: Property.make(":work-order/status", Schema.String).required()}})`、`Ontology.LinkType(...)`、`Change.clear/set/link`、`Input.value("x")`、`Input.now`、`Datalog.variables({...})` 提供类型化变量，查询行类型可推导。`defineOntology({name,version}, [...])`。

- `test/field-service.test.ts` 校验两种作者面产出 **IR 等价**；AGENTS.md 规定「Keep the portable IR JSON-safe and both authoring paths equivalent」。
- Forma 语言本身由 `preludes/ontology.lisp`（`define-form` + payload contract）和 `preludes/ontology-compiler.lisp`（`define-elaboration`）用 Lisp 自举定义。prelude 已声明约 30 种 form：`define-schema/define-api-group/endpoint`、`define-role/define-group/define-membership/define-contextual-role/define-permission`、`define-constraint`（`entity/severity/violation-query/message/assigns-task-to`）、`define-view/define-workspace`、`define-process/define-task`、`define-document/define-pdf-mapping` 等；但 `src/forma.ts` 的 `supportedForms` **只接受 5 种**：`define-entity`、`define-relation`、`define-action`、`define-mutation`、`define-datalog-query`，其余报 `... is not yet supported by the Triplex ontology adapter`。
- 库/复用：`(import "/std/git")`、`(import "/github")`，**显式内置注册表**，禁止文件系统/网络解析，每库每模型只加载一次，展开后 IR 里就是普通声明（`docs/architecture.md`、`docs/git-libraries.md`）。错误保留 `error.loc` 与 `error.source`。

## 6. 存储与查询引擎（`src/runtime.ts`）

- 后端 = Triplex：EAV 三元组 + Datalog。示例只用内存 `KvTriples.layer`，**无持久化**。
- `toTriplexConfig(ir)`：object type → Triplex `EntityType`，属性 → `ConfigAttribute.text/number/boolean/instant/ref`（`json` 暂不支持），两遍编译（先标量后 ref，解决前向引用）；link type → 名为 `PascalCase(name)+"Link"` 的实体，端点 + 属性都是 attribute；另写一个 `ConfigNode{kind:"open-ontology.definition", attrs:{formatVersion:1, source: JSON.stringify(ir)}}` 把 **IR 本身存进库**（自描述）。required/cardinality/unique 编译成 `GraphConstraint`。
- `makeOntologyRuntime(ir)` 暴露两个方法：
  - `invoke(actionRef, rawInput, {actor, commandId, correlationId, now, configSnapshot, enforce})`：先 `Schema.decodeUnknownEffect(action.input)` 校验输入，再把 changes 翻译成 `TransactOp[]`（`assert` / `retract-pattern`）一次 `transact`。事务元数据携带 actor/commandId/correlationId（审计与幂等的钩子，但 v0.1 只记录不校验）。
  - `query(nameOrQuery)` → `triples.queryAll`，结果行带类型。
- 链接 id 确定性：`link:${linkType}:${from}:${to}`，同一对端点重复 link 等于覆盖（天然幂等，但不支持同对多条边）。
- 单值属性「替换」需显式 `clear` 再 `set`，否则旧值仍可查（checker 会 warn）。即 store 是累积事实，不是行更新。
- Datalog 能力：模式匹配、`!=` 谓词、`not`、`aggregate`（count 等）、`having`、`orderBy/limit/offset`、递归 `rules`（带 `maxDepth`）、`optionalProjection`。

## 7. API 面

- 只有 **库级 TS API**：`exports` 为 `.`、`./forma`、`./ir`、`./runtime`、`./preludes/*`、`./libraries/*`。
- **无 REST / GraphQL / OpenAPI / MCP server / UI**。README「Next checkpoint」把 HTTP reflection 与 OpenAPI 排在 SQLite 持久化、权限、版本化配置发布之后。
- CLI：`pnpm model:check <file> [--json]`、`pnpm example*`。
- Agent 面：`llms.txt` + `llms-full.txt`（建模流程、语法、checker、运行语义、限制），`test/agent-guide.test.ts` 保证指南里每个 Forma 块无 warning 通过、引用的输出与真实命令一致。PR #9 探索从契约生成 SKILL.md / tool descriptor（未合并，无 MCP）。
- PR #14（open）：`compileOsdkMaker` 把模型编译成 `@osdk/maker`（Foundry Ontology as Code）：entity→object type，ref→外键 link，relation→多对多或 intermediary link，action→logic rules，查询不可表达则 warning。说明作者把 Foundry 当作目标后端之一。

## 8. 权限

- 运行时 **无任何鉴权**：`llms-full.txt` §11 原文 `Security: no policy or authorization. \`actor\` is recorded, not checked.`
- prelude 里有未接通的设计：`define-role`、`define-group`、`define-membership`、`define-contextual-role`、`define-permission`（slot：`principal / action / resource / effect / condition`），即 RBAC + 上下文角色 + 条件化 allow/deny 的雏形（`preludes/ontology.lisp` 第 586-670 行附近）。
- `examples/onboarded/account/evaluate.ts` 演示 **三值逻辑策略求值**：未记录的事实 = unknown，unknown 结果会列出它在等待的事实；`check.ts` 要求每个 policy/automation 判定被 scenario 覆盖到 true 和 false 两边（决策覆盖率）。示例代码，非包导出。

## 9. 值得借鉴的设计决策

1. **IR 是唯一契约**：作者面（DSL A / DSL B / 未来 UI）与后端（Triplex / Foundry OAC / 未来 SQL）都只对 IR 说话；IR 必须 JSON-safe、可往返。对 Rust：`serde` 可序列化的 `OntologyIr` crate 作为中心，前端解析器和存储适配器各自独立 crate。
2. **动作 = 声明式 change 列表**（create-object / set / clear / create-link），值来源仅 `input | now | literal`。易于静态分析、编译到事务、做审计。可作为 Rust 版 Action 的 MVP 形态，再扩展 condition / delete / unlink / 计算值。
3. **动作输入保留 JSON Schema draft-2020-12 文档**，并在 materialize 时拒绝无法无损往返的校验（`assertPortableActionSchema`）。启示：Rust 端用 JSON Schema 作为跨语言输入契约，拒绝「近似」约束。
4. **命名空间化属性 key** `:type/attr`：全局唯一、天然适配 EAV/三元组与 Datalog，也便于跨类型共享属性（可演化为 Shared Property）。
5. **链接具象化为实体**：link 有自己的 id 与属性，查询时与对象同构；确定性 link id 带来幂等。
6. **IR 自存入存储**（`open-ontology.definition` config node）：数据与 schema 同库、可版本化。
7. **模型 checker 作为一等公民**：located diagnostics（file:line:col）+ 「能编译但会出错」的语义 warning（未声明属性、未绑定 find 变量、optional input 被 change 使用、单值属性 set 前未 clear、固定 id 目标）。对 agent 生成 ontology 场景价值极高。
8. **库 import 只允许内置注册表**：可复现、无供应链风险；展开后 IR 不感知库。
9. **面向 agent 的文档且被测试锁定**：`llms-full.txt` 中示例必须真实可跑，防文档腐烂。
10. **事务元数据**：actor / commandId（幂等键）/ correlationId / configSnapshot（按哪个 schema 版本写入）/ enforce，为审计、幂等、schema 版本化预留了正确的位置。

## 10. 缺失 / 薄弱点（FoundryOntology 需自行补齐）

- 持久化：仅内存；无 SQLite/Postgres、无重启/历史查询测试、无迁移。
- 完整性：`required`/`cardinality`/`unique` 只记录、写入时不强制（`llms-full.txt` §11）。
- 权限：无 RBAC/ABAC、无行级/属性级安全、无 marking。
- Action：无条件/校验规则、无删除、无 unlink、无计算值、无 side effect/webhook、无提交前校验 criteria。
- 查询：命名查询无参数；无 Object Set 代数、无搜索/全文/向量、无分页游标 API（Triplex 内部有 cursor）。
- 无 Function、Interface、Value Type、Shared Property、Struct、Time series、Attachment。
- 无 primary key / title key 声明，对象 id 是调用方给的裸字符串。
- 无 schema 版本演进与 breaking change 检测（只有 `version` 字符串）。
- 无数据源绑定（dataset / pipeline / CDC 写回），无 Writeback 概念。
- API：无 REST/GraphQL/MCP/OSDK 生成；只有 TS 库。
- 规模：13 commits、单作者、强依赖小众库（Triplex 0.2、Forma 0.3、Effect 4 需 override + patch），可借鉴思想，不宜作为依赖。
- prelude 宣称的 form（constraint、permission、process、workspace、document）远多于实际可执行的 5 种，存在「设计先行、实现滞后」的落差。

## 11. 对 FoundryOntology（Rust）的具体建议

- 采用 `作者面 → IR → 后端适配` 三层；IR crate 零依赖、`serde` + JSON Schema 导出；IR 里声明种类在 MVP 就预留 `interface`、`function`、`value-type`，避免像 open-ontology 那样后补。
- Action MVP 抄其 change 列表模型，但第一版就加 `delete-object`、`delete-link`、`precondition`（基于查询的 submission criteria），并在写入路径强制 required/unique/cardinality。
- 属性 key 可借鉴 `:ns/attr` 命名空间化；存储层不必是三元组，但查询 IR 保持后端无关（Datalog 风格 IR 可编译到 SQL/DataFusion）。
- 复刻 `model:check` 式 checker（located errors + 语义 warning + `--json`），并让 agent 指南与测试绑定。
- 权限参考 prelude 中 `define-permission {principal, action, resource, effect, condition}` 的形状，以及三值逻辑求值「unknown 列出缺失事实」的可解释性设计。
- 事务元数据保留 `actor / command_id / correlation_id / schema_snapshot`，并真正落实幂等与审计。
- 观察 PR #14 的 Foundry 映射（ref→外键 link、relation→多对多/intermediary link、action→logic rules）可作为我们兼容 Foundry OAC 导出时的对照表。

## 来源文件

`README.md`、`AGENTS.md`、`docs/architecture.md`、`docs/git-libraries.md`、`package.json`、`src/ir.ts`、`src/model.ts`、`src/forma.ts`、`src/runtime.ts`、`src/materialize.ts`、`scripts/check-model.ts`、`preludes/ontology.lisp`、`preludes/ontology-compiler.lisp`、`examples/field-service/model.{lisp,ts}`、`examples/support-desk/model.lisp`、`libraries/github.lisp`、`examples/onboarded/README.md`、`apps/site/public/llms-full.txt`，以及 PR #5/#9/#14 描述。
