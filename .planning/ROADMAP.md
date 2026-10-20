# Roadmap: ontoforge

## Overview

Phase 1 定死 IR 契约。2026-10-16 起 Phase 2–7 按 `docs/designs/trino-engine-sql-tiers.md`（office-hours / CEO / eng review，决策 D1–D26）与 ADR-0015..0020 的纵切片重排：数据源绑定与引擎 → edit store 与 scope 模型 → scoped L0 读 → online Action → L2 / toolset / 审计 → REST 与 MCP 外壳。业务数据经 Trino 读取企业已有数据源，PG 只存系统数据与 edit store；读写都受同一套 scope 约束。需求全文见 `.planning/REQUIREMENTS.md`，决策见 `.planning/adr/`。

## Phases

**Phase Numbering:**

- Integer phases (1, 2, 3): 本 milestone 计划内工作
- Decimal phases (2.1, 2.2): 紧急插入（标注 INSERTED）

- [x] **Phase 1: Workspace, Schema & Lint** - cargo workspace 骨架，YAML/JSON 到单一 IR，E/W lint 规则与 line:col 诊断 (completed 2026-10-16)
- [x] **Phase 2: Source Binding & Engine** - IR 数据源绑定与类型契约，`ontoforge-engine`（自写 Trino client、自有 SQL printer、SQLite 替身），`validate --online` 与 drift 校验 (completed 2026-10-19)
- [ ] **Phase 3: Edit Store & Scope Model** - 按 type 强类型 edit store、DDL diff / migrate / reload 加固、edit log；用户 / 组 / 角色、CEL 行谓词（SQL 编译 + Rust 单行求值）
- [ ] **Phase 4: Scoped L0 Read** - Rust 编排叠加读：pk 读、列表、聚合分解、searchAround、keyset 分页、HMAC 版本令牌
- [ ] **Phase 5: Online Action** - Action 鉴权管线 ①-⑨、并发令牌、`command_id`、webhook outbox
- [ ] **Phase 6: L2 Raw DQL, Toolsets & Audit** - L2 AST 白名单与自检、读审计、定义 reload 门禁与 role 管理通道
- [ ] **Phase 7: REST & MCP Shell** - REST + OpenAPI + clients、MCP toolset（core / full / sql:raw）、`serve --dev`、release

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

**Plans**: 14/14 plans executed（6 waves + gap closure）

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

- [x] 01-14-PLAN.md — 多文档 YAML 的编辑器 schema 关联（项目级 yaml.schemas、每个 document 的 modeline、回归测试）

**Details**: `.planning/phases/01-workspace-schema-lint/`

### Phase 2: Source Binding & Engine

**Goal**: 开发者可以把 object type / M:N link 绑定到 Trino 物理表，ontoforge 经自写 client 连接已有 Trino 并校验绑定；Windows 本机用 SQLite 替身零外部依赖跑通
**Mode:** mvp
**Depends on**: Phase 1
**Requirements**: BIND-01, BIND-02, BIND-03, ENGINE-01, ENGINE-02, ENGINE-03, ENGINE-04, ENGINE-05
**Spike**: 已完成（spike 1 Trino client、spike 2 sqlparser）；决策见 ADR-0015、ADR-0017
**Success Criteria** (what must be TRUE):

  1. 定义以 `backing: source | editOnly` 显式声明数据来源，`SourceBinding` 绑定文件（按 profile 分目录）声明 source 绑定；decimal 精度、无时区 timestamp 的 `timezone`、collation、geopoint 不支持等契约违反时 lint 报 E，示例 ontology 仍 lint clean
  2. `ontoforge validate --online` 对真实 Trino 校验绑定（列存在、类型兼容）并报告不匹配；`--check-pk` 报告重复主键
  3. Trino client 能执行查询、按超时与客户端断连取消、带 trace token 与 client tags；Trino 不可达时返回 `EngineUnavailable`
  4. 自有 printer 生成的 SQL 经 proptest（任意字符串值往返不改变结构）与真实 Trino / SQLite 执行验证；SQLite 替身在 Windows `cargo test` 全绿
  5. Linux CI 以 Trino + PG service container 跑集成测试，nightly 加跑 Iceberg

**Plans**: 10/10 plans executed（5 waves）

Plans:

**Wave 1**

- [x] 02-01-PLAN.md — backing 必填与 Phase 1 fixture 全量迁移、ADR-0021、wave 2 共用前置（read_limited / ProfileName / V16-V20）
- [x] 02-02-PLAN.md — ontoforge-engine 骨架：Engine trait、类型化 printer、SQLite 替身执行、proptest

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 02-03-PLAN.md — SourceBinding 定义层、profile 选定与完备性、aviation 绑定
- [x] 02-04-PLAN.md — 自写 Trino client 主路径、ontoforge-server.toml、认证 / CA / 两个服务身份

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 02-05-PLAN.md — phase tracer：`validate --online` 经 SQLite 替身比对绑定、fixture 语义
- [x] 02-06-PLAN.md — Trino client 不可用分类、重试、超时 / 断连 / 上限取消
- [x] 02-07-PLAN.md — 类型契约 lint ONT-E13..E16 与 ONT-W17

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 02-08-PLAN.md — D-13 完整兼容表、drift 状态模型、绑定规范文档
- [x] 02-09-PLAN.md — `--server-config` / Trino 路径、退出码 3、`--check-pk`（查重排除 NULL）

**Wave 5** *(blocked on Wave 4 completion)*

- [x] 02-10-PLAN.md — Linux CI：Trino + PG 集成作业、nightly Iceberg、私有 fixture 入口

**Details**: `.planning/phases/02-source-binding-engine/`

### Phase 3: Edit Store & Scope Model

**Goal**: edit store 按 object type 生成强类型表并安全演进，所有写入有不可变 edit log；用户 / 组 / 角色与 scope 规则可配置，同一份 CEL 行谓词既能编译成 SQL 也能在 Rust 中对单行求值且结果一致
**Mode:** mvp
**Depends on**: Phase 2
**Requirements**: STORE-01, STORE-02, STORE-03, STORE-04, STORE-05, STORE-06, STORE-07, SCOPE-01, SCOPE-02, SCOPE-03, SCOPE-04, SCOPE-05, SCOPE-06, SCOPE-07, AUTH-01, AUTH-02
**Spike**: 无（spike 3 已定 edit store 形态）；决策见 ADR-0016、ADR-0018
**Success Criteria** (what must be TRUE):

  1. 加载示例 ontology 后生成每个 object type 与 M:N link type 的 edit 表；新增类定义变更在 reload 时自动执行，破坏性变更被 lint 拦下且 `ontoforge migrate --plan/--apply` 可完成并归档旧列；reload 失败时保留旧定义
  2. 同一套 conformance 测试在 SQLite（Windows 默认）与 PostgreSQL 上通过；edit log 与历史表只能追加
  3. 按属性 patch 的叠加语义（set_mask、tombstone、release、版本单调）由内存参考模型的 proptest 覆盖
  4. 角色可授予 object type、属性 allow-list、CEL 行谓词、action、SQL 级别与 toolset；行谓词只能引用同行属性与 `principal.attrs`，缺失属性按 false；多 role 合并不产生「A 的列 × B 的行」泄露
  5. 同一 CEL 行谓词的 SQL 编译结果与 Rust 单行求值经差分测试一致；criteria 列覆盖 lint 在 reload 与 role 变更时生效；OIDC JWT 与 dev token 均可认证

**Plans**: 8/20 plans executed（8 waves）

Plans:

**Wave 1**

- [x] 03-01-PLAN.md — 依赖 legitimacy 核验、全部新依赖与 crate 依赖方向（ADR-0023）、供应链守卫
- [x] 03-02-PLAN.md — `indexed`、ONT-V21..V27、主体属性类型、server config 的 `[store]` / `[auth]`

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 03-03-PLAN.md — tracer：物理格式 checkpoint + `reload` 库函数在 SQLite 建 edit 表、SQL 出口守卫（ADR-0024）
- [x] 03-05-PLAN.md — 行谓词：D-06 checkpoint、CEL 子集 → `Pred` → Kleene 求值（ADR-0025）
- [x] 03-06-PLAN.md — `Role` / `RoleBinding` 文档、scope 加载器与 JSON Schema
- [x] 03-07-PLAN.md — OIDC JWT 认证与 JWKS 缓存

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 03-04-PLAN.md — `ontoforge reload` 子命令、PostgreSQL 后端与两种方言 DDL 快照
- [x] 03-11-PLAN.md — 行谓词降为 SQL、`StringMatch`、差分 proptest、多 role 合并

**Wave 4** *(blocked on Wave 3 completion)*

- [ ] 03-08-PLAN.md — edit log 格式 checkpoint、commit 写路径与双后端 conformance 框架
- [ ] 03-10-PLAN.md — 索引（含 1:1 部分唯一索引）、物理名上限与 ONT-V24
- [ ] 03-12-PLAN.md — dev token 与 `ontoforge dev-token`
- [ ] 03-14-PLAN.md — scope 编译：引用 / 谓词 / criteria 列覆盖 / collation

**Wave 5** *(blocked on Wave 4 completion)*

- [ ] 03-09-PLAN.md — append-only 触发器、重放重建 edit store、重复提交（ADR-0026）
- [ ] 03-15-PLAN.md — `ontoforge migrate --plan/--apply`、改名与归档

**Wave 6** *(blocked on Wave 5 completion)*

- [ ] 03-13-PLAN.md — 完整叠加状态机、参考模型 proptest、基数与并发
- [ ] 03-16-PLAN.md — `ontoforge scope plan|apply`、管理审计、user 记录

**Wave 7** *(blocked on Wave 6 completion)*

- [ ] 03-17-PLAN.md — 变更历史 API 与当前 scope 遮蔽
- [ ] 03-18-PLAN.md — reload 加固：栅栏、lock_timeout、D-18 复核、Trino 可见探测
- [ ] 03-19-PLAN.md — `ontoforge import`（editOnly）

**Wave 8** *(blocked on Wave 7 completion)*

- [ ] 03-20-PLAN.md — Linux CI（PG conformance、Trino 语料与探测）与 `docs/edit-store.md`

**Details**: `.planning/phases/03-edit-store-scope-model/`

### Phase 4: Scoped L0 Read

**Goal**: 调用者只能在自己的 scope 内读取对象：按 pk、过滤、排序分页、searchAround、集合运算与聚合都由 Rust 编排叠加完成，Trino 只承担单表查询
**Mode:** mvp
**Depends on**: Phase 3
**Requirements**: QUERY-01, QUERY-02, QUERY-03, QUERY-04, QUERY-05, QUERY-06, QUERY-07, QUERY-08, QUERY-09, QUERY-10
**Spike**: 已完成（spike 3、spike 3b）；决策见 ADR-0020
**Success Criteria** (what must be TRUE):

  1. 已编辑对象经 PG 直读返回，未编辑对象经 Trino 单表 pk 查询返回，tombstone 不可见、release 后回到 source；重复 source pk 返回 `AmbiguousObject`
  2. WhereClause 全部运算符、排序与 keyset 分页（`has_more` 判定结束，补取循环凑满页）在示例数据上正确，结果按 pk 去重；String 搜索按 token 匹配
  3. searchAround、集合运算可用；count / sum / avg 分解聚合与朴素叠加结果相等，min / max / distinct 按 ADR-0020 处理
  4. 越权 conformance（换 ID、伪造主体属性、多 role 列 × 行交叉、谓词 / 聚合反推、令牌反推）全部被拒绝；L0 输入输出与错误不含物理名；结果超限带 `truncated`
  5. 编译器保证每条 Trino 查询单表、≤ 2 fragment；安静或隔离集群下 pk 读 p95 < 500ms、列表 p95 < 1s、聚合 p95 < 1.5s（私有 fixture 实测记录）

**Plans**: TBD
**Details**: `.planning/phases/04-scoped-l0-read/`

### Phase 5: Online Action

**Goal**: 对象图只能经 Action 修改：参数校验、鉴权管线 ①-⑨、submission criteria、乐观并发与 `command_id` 幂等全部可用，每次提交都原子写入 edit 与 edit log 并通知下游
**Mode:** mvp
**Depends on**: Phase 4
**Requirements**: ACTION-01, ACTION-02, ACTION-03, ACTION-04, ACTION-05, ACTION-06, ACTION-07, ACTION-08, ACTION-09, ACTION-10, ACTION-11, ACTION-12
**Spike**: 无；决策见 ADR-0016、ADR-0018
**Success Criteria** (what must be TRUE):

  1. 参数错误返回逐参数错误且不写入；合法 Action 在单个 PG 事务内写 edit、edit log、`action_commands` 与 webhook outbox，任一失败整体回滚；validate-only 与批量提交可用
  2. 写字段超出调用者可见列、criteria 引用当前行被遮蔽的列、写后对象对调用者不可见（含 link 两端与 release）时一律 `ScopeDenied`，错误不泄露对象状态
  3. 同 pk 并发提交恰好一个成功、其余 `VersionConflict`；过期 `_version` 令牌被拒绝；`command_id` 重放返回缓存结果、同键异参 `CommandIdReused`、回放前失权则拒绝
  4. 目标或读集需要 Trino 时 Trino 不可用返回 `EngineUnavailable`；只涉及已编辑对象的 Action 照常执行
  5. 提交后下一次读取可见；下游 webhook 至少收到一次带 `event_id` 的事件

**Plans**: TBD
**Details**: `.planning/phases/05-online-action/`

### Phase 6: L2 Raw DQL, Toolsets & Audit

**Goal**: 受信主体可以在 allow-list 内执行只读 Trino SQL，且只有在 Trino 侧权限可证明安全时才开放；所有级别的读取都留下元数据审计；定义与授权的变更只经部署方与管理通道
**Mode:** mvp
**Depends on**: Phase 5
**Requirements**: L2-01, L2-02, L2-03, L2-04, AUDIT-01
**Spike**: 进入前在 Trino 483 上复验 spike 2 语料；决策见 ADR-0018、ADR-0019
**Success Criteria** (what must be TRUE):

  1. L2 执行器只接受 SELECT / WITH，拒绝 `TABLE()` 表函数、`CALL`、`EXECUTE`、多语句与会话 / 事务语句，执行时发送重序列化后的 SQL；spike 2 语料全部按预期放行或拒绝
  2. allow-list 外 catalog、edit store catalog 与 `system` 元数据表全部被拒绝
  3. 启动自检无法证明 L2 身份受限（含只能看到自己的 `system.runtime.queries`）时 L2 被禁用并在 `/health` 告警
  4. 所有级别的读取写入元数据审计（主体、工具、对象类型、apiName 级条件、生效 role、SQL 哈希、行数、耗时、request_id），不含结果数据；审计积压时读请求返回 `AuditUnavailable`
  5. role / scope 只能经管理通道或 L3 修改并审计；定义 reload 先 validate / lint，失败不切换；每个请求固定使用开始时的定义版本

**Plans**: TBD
**Details**: `.planning/phases/06-l2-toolsets-audit/`

### Phase 7: REST & MCP Shell

**Goal**: 应用经 REST（OpenAPI 3.1、Rust / TS client），agent 经 MCP（按主体过滤的 toolset）使用全部能力；`ontoforge serve --dev` 单二进制在本机跑通端到端演示，并发布三平台二进制
**Mode:** mvp
**Depends on**: Phase 6
**Requirements**: API-01, API-02, API-03, API-04, API-05, MCP-01, MCP-02, MCP-03, MCP-04, MCP-05, MCP-06, MCP-07, MCP-08, DX-01, DX-02, DX-04
**Spike**: 进入前验证 `rmcp` 动态 tool 列表
**Success Criteria** (what must be TRUE):

  1. REST 覆盖元数据、对象读取、查询、Action 提交与历史；错误统一为结构化错误码；OpenAPI 3.1 纳入快照测试；Rust 与 TS client 覆盖每个端点组
  2. agent 经 stdio 或 `/mcp` 连接；`list_tools` 只返回主体被授予的 toolset（core / full / sql:raw），L3 与任何变更工具不出现
  3. 每个 action type 自动暴露为 MCP 工具（含 validate-only）；6 个 L2 工具与 mcp-trino 名称参数一致，结果按 allow-list 过滤
  4. 同一身份经 MCP 与 REST 得到相同授权结果，写入同样记入 edit log
  5. `ontoforge serve --dev`（SQLite 替身 + dev token）在 Windows 本机零外部依赖跑通示例 ontology 的查询 → Action → 历史演示；release workflow 发布 Linux / macOS / Windows 二进制

**Plans**: TBD
**Details**: `.planning/phases/07-rest-mcp-shell/`

## Progress

**Execution Order:**
Phases 按数字顺序执行：1 → 2 → 3 → 4 → 5 → 6 → 7（纵切片，严格顺序依赖）

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Workspace, Schema & Lint | 14/14 | Complete    | 2026-10-16 |
| 2. Source Binding & Engine | 10/10 | Complete    | 2026-10-19 |
| 3. Edit Store & Scope Model | 8/20 | In Progress|  |
| 4. Scoped L0 Read | 0/TBD | Not started | - |
| 5. Online Action | 0/TBD | Not started | - |
| 6. L2 Raw DQL, Toolsets & Audit | 0/TBD | Not started | - |
| 7. REST & MCP Shell | 0/TBD | Not started | - |
