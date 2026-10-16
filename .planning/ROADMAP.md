# Roadmap: ontoforge

## Overview

按依赖链 schema → storage / edit log → action 与 query → auth → REST + clients → MCP 逐层交付，每个 phase 都能用命令或测试端到端验证。Phase 1、2 定死两份长期契约（IR 与 edit log 格式）；Phase 3、4 分别打通受控写路径与读路径；Phase 5 统一套上认证授权；Phase 6、7 通过 REST 与 MCP 两个同等地位的出口对外。需求全文见 `.planning/REQUIREMENTS.md`，决策见 `.planning/adr/`。

## Phases

**Phase Numbering:**

- Integer phases (1, 2, 3): 本 milestone 计划内工作
- Decimal phases (2.1, 2.2): 紧急插入（标注 INSERTED）

- [ ] **Phase 1: Workspace, Schema & Lint** - cargo workspace 骨架，YAML/JSON 到单一 IR，E/W lint 规则与 line:col 诊断
- [ ] **Phase 2: Storage & Edit Log** - SQLite + PostgreSQL 双后端，不可变 edit log 与对象历史，`ontoforge import`
- [ ] **Phase 3: Action Execution** - 参数校验、rules 单事务执行、CEL submission criteria、validate-only 与 batch
- [ ] **Phase 4: Object Set Query** - filter / 全文搜索 / 排序与游标分页 / searchAround / 集合运算 / 聚合，双后端等价
- [ ] **Phase 5: Auth & RBAC** - OIDC JWT 与 dev token 认证，三级 RBAC，fail-closed
- [ ] **Phase 6: REST API & Clients** - OpenAPI 3.1、Rust 与 TS client、`ontoforge serve --dev` 端到端演示
- [ ] **Phase 7: MCP Server** - stdio + streamable HTTP，describe / 查询 / 每个 action 一个工具

## Phase Details

### Phase 1: Workspace, Schema & Lint

**Goal**: 开发者可以用 YAML/JSON 编写 ontology，并通过 `ontoforge validate` / `ontoforge lint` 得到带源码定位的诊断；仓库在 Windows 本机 `cargo test` 全绿
**Mode:** mvp
**Depends on**: Nothing (first phase)
**Requirements**: SCHEMA-01, SCHEMA-02, SCHEMA-03, SCHEMA-04, SCHEMA-05, SCHEMA-06, SCHEMA-07, LINT-01, LINT-02, LINT-03, LINT-04, DX-03
**Spike**: 进入前验证 `serde-saphyr` 能否按路径取 span，结论写回 ADR-0003
**Success Criteria** (what must be TRUE):

  1. cargo workspace 按 ADR-0011 建好 10 个 `ontoforge-*` crate（edition 2024、resolver 3、rust-version 1.98），Windows 11 本机不依赖 Docker 执行 `cargo test --workspace` 全部通过；此条作为后续每个 phase 的回归门槛
  2. `ontoforge validate examples/ontologies/<示例>/` 把分散在多个文件中的 object type、属性 base type 与 required、link type（1:1 / 1:N / M:N）、action type（参数、rules、submission criteria）合并加载为单一 IR；同一 ontology 的 YAML 与 JSON 写法得到相同 IR
  3. 定义有错（重复 apiName、未知 base type、link 指向不存在的 object type 等）时，CLI 输出带文件名与 line:col 的诊断，退出码非 0
  4. `ontoforge lint` 运行适用于 v1 schema 范围的 ONT-E 规则与 ONT-W01–W11，输出人类可读（带源码定位）与 JSON 两种格式；有 E 级问题时退出码非 0；配置文件可按规则 ID 关闭规则或调整严重级
  5. 开发者可以导出 ontology 定义的 JSON Schema，仓库内示例 ontology 通过该 Schema 校验

**Plans**: 13/14 plans executed（6 waves + gap closure）

Plans:

**Wave 1**

- [x] 01-01-PLAN.md — 定义格式契约 checkpoint + Walking Skeleton tracer（ObjectType YAML → IR → `ontoforge validate` line:col）

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 01-02-PLAN.md — 其余 8 个 crate 骨架、CI、CONTRIBUTING、ADR-0003 回写
- [x] 01-03-PLAN.md — 加载层：include glob、路径安全、严格 JSON、栈安全、确定性
- [x] 01-04-PLAN.md — object type 与 13 个 base type、V10 校验、aviation 双格式示例

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 01-05-PLAN.md — link type（1:1 / 1:N / M:N）与 V08 / V09 / V11
- [x] 01-07-PLAN.md — CEL 预检、大栈线程隔离编译、作用域感知成员路径遍历

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 01-06-PLAN.md — action type（参数、rules、值来源、criteria 结构）与 V09 / V12

**Wave 5** *(blocked on Wave 4 completion)*

- [x] 01-08-PLAN.md — lint 框架、ontoforge.toml、ONT-E08 / E09、`ontoforge lint`
- [x] 01-12-PLAN.md — CEL 引用检查与类型推断（V13）接入 validate、ADR-0006 回写

**Wave 6** *(blocked on Wave 5 completion)*

- [x] 01-09-PLAN.md — JSON Schema 导出（`ontoforge schema export`）
- [x] 01-13-PLAN.md — 用现成 crate 替换手写实现（toml DeTable、heck、anstream、行列换算去重）
- [x] 01-10-PLAN.md — ONT-W01..W05 命名规则
- [x] 01-11-PLAN.md — ONT-W06..W11 结构规则与 phase 门禁

**Gap closure**（UAT G-1-2）

- [ ] 01-14-PLAN.md — 多文档 YAML 的编辑器 schema 关联（项目级 yaml.schemas、每个 document 的 modeline、回归测试）

**Details**: `.planning/phases/01-workspace-schema-lint/`

### Phase 2: Storage & Edit Log

**Goal**: 对象与 link 可以持久化到 SQLite 或 PostgreSQL，每次写入追加到不可变 edit log，示例数据可经 `ontoforge import` 批量导入并追溯历史
**Mode:** mvp
**Depends on**: Phase 1
**Requirements**: STORE-01, STORE-02, STORE-03, STORE-04, STORE-05, STORE-06
**Spike**: 进入前验证 `postgresql_embedded` 在 Windows 上的表现，以及 `sea-query` 的 JSON 表达式与表达式索引；phase research 决定通用 JSON 表与热点属性物化列的取舍、edit log 格式（为 scenario / branch overlay 预留字段），edit log 格式在本 phase 定死并写 ADR
**Success Criteria** (what must be TRUE):

  1. 同一套 conformance 测试在 SQLite（Windows 默认 `cargo test`）与 PostgreSQL（本机 `postgresql_embedded`、CI service container）上全部通过
  2. 加载新增 object type 或属性的新版 ontology 后，不写任何 DDL 迁移即可写入并读出新类型的对象
  3. `ontoforge import` 从 JSON / CSV 导入示例对象与 link，每次导入作为系统 edit 追加到 edit log（含 actor、来源、时间、edits）；已有 edit log 记录无法被修改或删除
  4. 违反 1:1 / 1:N 基数的 link 写入被存储层拒绝并返回明确错误
  5. 按 primary key 可以读出某个对象的完整变更历史，每条历史对应一条 edit log 记录

**Plans**: TBD
**Details**: `.planning/phases/02-storage-edit-log/`

### Phase 3: Action Execution

**Goal**: 对象图只能经 action 修改：参数校验、rules 单事务执行、CEL submission criteria、validate-only 与 batch 全部可用，每次成功提交都留下 edit log 记录
**Mode:** mvp
**Depends on**: Phase 2
**Requirements**: ACTION-01, ACTION-02, ACTION-03, ACTION-04, ACTION-05, ACTION-06, ACTION-07
**Spike**: 进入前验证 `cel` crate 的静态类型检查能力，结论写回 ADR-0006
**Success Criteria** (what must be TRUE):

  1. 提交类型错误、缺少 required 参数或违反约束的 action，得到逐参数的错误列表，且不产生任何写入
  2. 合法 action 在单个事务内执行 create / modify / delete object 与 add / remove link rules，返回标准化 edits 摘要（新增 / 修改 / 删除的对象与 link），并追加一条 edit log 记录（actor、action、时间、参数、edits）；任一 rule 失败则整体回滚
  3. submission criteria（引用参数、当前用户、对象当前属性）不满足时 action 被拒绝，返回 ontology 中配置的失败信息
  4. validate-only 提交返回与真实提交一致的校验结果但不写入；批量提交同一 action 时全部成功或全部回滚
  5. 被配置为仅允许经 action 编辑的 object type，除 action 与可审计的 `ontoforge import` 外不存在其他写入途径（测试覆盖存储层公开 API）

**Plans**: TBD
**Details**: `.planning/phases/03-action-execution/`

### Phase 4: Object Set Query

**Goal**: 客户端可以用 ObjectSet AST 读取对象图：按主键取对象、过滤、全文搜索、排序与游标分页、searchAround、集合运算与聚合，SQLite 与 PostgreSQL 结果一致
**Mode:** mvp
**Depends on**: Phase 2（与 Phase 3 无依赖，可调换顺序）
**Requirements**: QUERY-01, QUERY-02, QUERY-03, QUERY-04, QUERY-05, QUERY-06, QUERY-07, QUERY-08
**Spike**: 无；phase research 覆盖查询编译器（ObjectSet AST → `sea-query`、递归 CTE 遍历与深度上限）
**Success Criteria** (what must be TRUE):

  1. 按 primary key 获取单个对象时可指定属性子集；WhereClause 全部运算符（eq / gt / gte / lt / lte / isNull / in / startsWith / containsAnyTerm / containsAllTerms / and / or / not）与 String 属性全文搜索在示例数据上返回正确结果
  2. 结果可按属性排序，用游标分页遍历完整结果集时不重复、不遗漏
  3. searchAround 沿 1:1 / 1:N / M:N link 遍历得到关联对象集合，单个对象的关联对象查询可用
  4. union / intersect / subtract 可用；count / sum / avg / min / max / approximate 与 exact distinct 聚合可用，groupBy 支持 exact 值、固定宽度与范围分桶
  5. 同一组查询在 SQLite 与 PostgreSQL 上返回相同结果，由等价测试与集合代数律 property test 覆盖

**Plans**: TBD
**Details**: `.planning/phases/04-object-set-query/`

### Phase 5: Auth & RBAC

**Goal**: 每次读写都带已认证身份并经 RBAC 检查：OIDC JWT 与 dev token 都能认证，角色可在 ontology / object type / action type 级授予，未授权一律拒绝
**Mode:** mvp
**Depends on**: Phase 3, Phase 4
**Requirements**: AUTH-01, AUTH-02, AUTH-03, AUTH-04, AUTH-05
**Spike**: 无（RBAC 范围小，可跳过 phase research）
**Success Criteria** (what must be TRUE):

  1. OIDC 签发的 JWT 通过认证，JWKS 自动拉取并缓存（测试用本地 mock IdP 验证）；过期、签名错误或缺失的 token 被拒绝
  2. dev 模式下开发者用 CLI 生成本地 token 即可认证，无需外部 IdP；dev 模式同样执行真实授权检查，不存在 allow-all
  3. 管理员在 ontology、object type、action type 级授予角色（viewer / editor / owner 等）后，对应身份的读写权限随之生效；未获任何授权的身份读写一律被拒绝
  4. 列表与 object set 查询结果先按权限过滤再分页，无权对象不出现在结果中，也不计入分页与聚合
  5. 执行 action 时先检查执行权限再求值 submission criteria；无权执行时返回的错误不包含任何对象状态

**Plans**: TBD
**Details**: `.planning/phases/05-auth-rbac/`

### Phase 6: REST API & Clients

**Goal**: 应用可以通过 REST（OpenAPI 3.1）、`ontoforge-client`（Rust）与 TS 平台层 client 使用全部能力；单个二进制 `ontoforge serve --dev` 在本机跑通示例 ontology 的端到端演示
**Mode:** mvp
**Depends on**: Phase 5
**Requirements**: API-01, API-02, API-03, API-04, API-05, DX-01, DX-02
**Spike**: 无（axum + utoipa 常规用法，可跳过 phase research）
**Success Criteria** (what must be TRUE):

  1. `ontoforge serve --dev` 单个二进制在 Windows 本机启动完整服务（SQLite + dev token），无需任何外部依赖
  2. REST API 覆盖 ontology 元数据、对象读取、object set 查询、action 提交与 edit 历史；所有错误响应统一为 `{ errorCode, errorName, parameters }`
  3. 服务导出 OpenAPI 3.1 文档并纳入快照测试，接口与实现出现漂移时测试失败
  4. `ontoforge-client`（Rust）与由 OpenAPI 生成的 TS 平台层 client 都能调用全部 REST 能力，集成测试覆盖每个端点组
  5. 按仓库内说明，用示例 ontology 完成 导入 → 查询 → 执行 action → 查看 edit 历史 的端到端演示

**Plans**: TBD
**Details**: `.planning/phases/06-rest-api-clients/`

### Phase 7: MCP Server

**Goal**: AI agent 通过 MCP（stdio 与 streamable HTTP）读取 ontology 结构、查询对象、调用 action，与 REST 共用同一套授权与 edit log
**Mode:** mvp
**Depends on**: Phase 6
**Requirements**: MCP-01, MCP-02, MCP-03, MCP-04, MCP-05, MCP-06
**Spike**: 进入前验证 `rmcp` 动态 tool 列表；phase research 覆盖 MCP 工具面设计
**Success Criteria** (what must be TRUE):

  1. agent 可以通过 `ontoforge mcp`（stdio）或 `ontoforge serve` 进程挂载的 `/mcp`（streamable HTTP）连接 MCP server
  2. describe 工具返回 object types、属性、links、actions 及其描述；查询工具可执行 filter / search / searchAround / aggregate
  3. 每个 action type 自动暴露为一个 MCP 工具，输入 schema 由参数定义派生，工具列表由当前加载的 ontology 生成而非手写
  4. 以 validate-only 方式调用 action 工具只返回校验结果，不产生写入
  5. 同一身份经 MCP 与 REST 得到相同的授权结果；经 MCP 提交的 action 与 REST 提交的写入同样记入 edit log

**Plans**: TBD
**Details**: `.planning/phases/07-mcp-server/`

## Progress

**Execution Order:**
Phases 按数字顺序执行：1 → 2 → 3 → 4 → 5 → 6 → 7（Phase 3 与 4 互不依赖，可调换）

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Workspace, Schema & Lint | 13/13 | In Progress|  |
| 2. Storage & Edit Log | 0/TBD | Not started | - |
| 3. Action Execution | 0/TBD | Not started | - |
| 4. Object Set Query | 0/TBD | Not started | - |
| 5. Auth & RBAC | 0/TBD | Not started | - |
| 6. REST API & Clients | 0/TBD | Not started | - |
| 7. MCP Server | 0/TBD | Not started | - |
