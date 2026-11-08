# ontoforge

## What This Is

ontoforge 是一个用 Rust 实现、可自托管的开源 Ontology 操作层，复刻 Palantir Foundry Ontology 的核心能力：用 YAML/JSON 定义 object type / link type / action type（ontology-as-code），对象数据只能通过带校验的 Action 写入并留下不可变 edit log，再用 object set 查询（过滤、遍历、集合运算、聚合）读取。同一份 ontology 同时通过 REST（配 Rust / TypeScript client）服务业务应用，通过 MCP 服务 AI agent。

v1.0 已交付的完整链路：本体定义与 lint 引擎、数据源绑定与 Trino 引擎接线、edit store 与 scope 授权模型、scoped L0 读、online Action、L2 只读 DQL 与读审计、REST + MCP 外壳与三平台发行包。

## Core Value

定义一次 ontology，应用和 agent 就能通过 REST 与 MCP 两个同等地位的出口，对同一份对象图做受控的读写：写入一律经 Action 校验与授权，每次写入都可审计。

## Requirements

### Validated

- ✓ ontology-as-code：YAML/JSON → 单一 IR，line:col 诊断，导出 JSON Schema 供编辑器补全 — v1.0（Phase 1，14/14 plans，VERIFICATION `passed` 102/102 accounted for）
- ✓ lint 引擎：E 级规则 + W01–W11，human / JSON 双格式，`ontoforge.toml` 配置 — v1.0（Phase 1）
- ✓ 数据源绑定与 Trino 连接（BIND-01..03、ENGINE-01..05）：`SourceBinding` 文件与必填 `backing`、类型契约 lint、`validate --online` / `--check-pk`、自写 Trino client、自有 SQL printer、SQLite 替身、Linux CI Trino + PG 与 nightly Iceberg；ontoforge 只带连接能力，不打包 Trino（ADR-0015、ADR-0017、ADR-0021） — v1.0（Phase 2 + 07.2；2026-11-08 验收闭环：verifier `passed` 109/109、UAT 9/9、G-02-B/C/D 人工签字）
- ✓ edit store、edit log 与 scope 模型（STORE-01..07、SCOPE-01..06、AUTH-01..02） — v1.0（Phase 3，VERIFICATION `passed` 139/139，含 1 项用户明确接受的 override）
- ✓ scoped L0 读取（QUERY-01..10）：Rust 编排 source/edit 叠加、行列授权与单表 Trino 查询 — v1.0（Phase 4，VERIFICATION `passed` 36/36）
- ✓ online Action（ACTION-01..12）：鉴权、并发令牌、主体作用域幂等与 webhook outbox — v1.0（Phase 5，VERIFICATION `passed` 36/36）
- ✓ L2 Raw DQL、toolsets 与读审计（L2-01..04、AUDIT-01） — v1.0（Phase 6，VERIFICATION `passed` 22/22）
- ✓ REST、OpenAPI 3.1、Rust / TypeScript SDK、MCP toolsets 与持久 dev 演示（API-01..05、MCP-01..08、DX-01..03） — v1.0（Phase 7，VERIFICATION `passed` 80/80，UAT 14/14）
- ✓ source 基数报告与 MinIO / Iceberg 精确合同补证（STORE-05 source 子范围、ENGINE-05） — v1.0（Phase 07.1 `passed` 18/18、Phase 07.2 `passed` 9/9）
- ✓ release workflow 三平台发行包（DX-04） — v1.0（Phase 7，历史发行验收；本轮未重跑 native release）

### Active

v1.0 关闭后进入下一 milestone 的候选需求，尚未排期（编号见 `.planning/milestones/v1.0-REQUIREMENTS.md` 的 v2 Requirements 节）：

- L1 Ontology SQL（`sql:ontology`，类 SQL 按 object 名书写，服务端改写并注入 scope）
- 湖仓与引擎：Nessie 分支直写 Iceberg 后 merge、edit store materialize 与 edit log 归档、Spark / Doris / Kyuubi 作为可选引擎
- Schema 扩展：interfaces、shared properties、struct 属性、value types、runtime 派生属性
- 治理：AI 提议 ontology 变更 → 人审批 → 生效的流程与管理界面
- 检索：vector 属性与 KNN、`tantivy` 全文索引
- 运行时：query / edit functions 运行时、WebSocket 订阅、scenarios、schema branching + proposals、ontology 级强类型 SDK codegen、GraphQL
- 安全：评估 `cedar-policy`、行谓词跨对象引用（SEC-02、SEC-03）

### Out of Scope

- 应用层（Workshop、Object Explorer、Quiver、Slate 等前端产品）：不是 ontology 核心
- 数据集成平台（Pipeline Builder、数据集事务、构建调度）：只提供批量导入与后续 connector 扩展点
- AIP Logic、模型部署、文档解析管线：作为外部插件对接
- Markings、cipher、media sets、time series、geotemporal series：子系统规模大，按需求单独立项
- Release management、Marketplace、Developer Console UI
- 外部系统原生 writeback 连接器：MVP 只留 webhook side effect 扩展点
- GraphQL：v1 只做 REST + MCP（ADR-0007）
- 通用 CRUD 写接口：违背「写入只经 Action」的核心设计
- 打包、部署或托管 Trino：ontoforge 是服务，只带连接能力；Trino 由部署方提供

v2 候选（不在当前 milestone）：interfaces 与 shared properties、struct、value types、runtime 派生属性、vector + KNN、scenarios、functions 运行时、WebSocket 订阅、schema branching + proposals、ontology 级强类型 SDK codegen。

## Current State

- **milestone**：v1.0 已于 2026-11-08 关闭并归档（`.planning/milestones/`）；9 个 phase、94 个 plan、115 个 task 全部完成。
- **需求**：79 项 v1 需求全部 Complete（`.planning/milestones/v1.0-REQUIREMENTS.md`）；验收报告与 UAT 证据在 `.planning/milestones/v1.0-phases/`。
- **代码规模**：cargo workspace 11 个 crate，`crates/**.rs` 126,982 行（tracked，2026-11-08）。
- **技术栈**：Rust 1.98 / edition 2024，axum + utoipa（REST）、rmcp（MCP）、sqlx + sea-query（存储）、cel（表达式）、serde-saphyr（YAML span）。
- **本机门槛**：Windows 11 上 `cargo test --workspace --locked --offline` 全绿，不依赖 Docker；Postgres / Trino 走 CI service container，私有 fixture 经仓库外入口。
- **出口**：REST（OpenAPI 3.1 + Rust / TypeScript client）与 MCP（core / full / sql:raw 按主体过滤的 toolset）共享同一授权与 edit log。
- **残余缺口（已记录，不宣称审计通过）**：
  - Phase 05：生产环境 `webhook-worker` 与真实下游接收端的联调尚未执行（WSL 探针只验证隔离环境机制）。
  - Phase 06：`security::cap_counts_final_masked_row_and_version` 的纯计算断言与审计时间敏感集成验证仍共用墙钟，CI 用 `--test-threads=1` 规避，不能声称慢机器上完全消除波动。
  - Phase 07.2 / Phase 02：ADR-0032 的通用化只落到 planning 层，产品层 `private_fixtures.rs` / `CONTRIBUTING.md` / `docs/designs/trino-engine-sql-tiers.md` 仍点名 SQL Server（代码无按类型分叉，不构成功能缺陷）。
  - Phase 02：`02-10-SUMMARY.md` 的陈旧断言（本地 warehouse 描述、测试计数）未追改，历史 SUMMARY 为快照。
  - 审计结论本身仍是 `gaps_found`（2026-11-05 dated 快照）；Phase 02 于 2026-11-08 单独闭环后，报告门驱动的 8 项 unsatisfied 已失效，但审计文件未被改写。
- **未发布**：Release / npm 未发布；`v1.0` tag 为本地 annotated tag，未推送。

## Next Milestone Goals

下一 milestone（候选 v2.0）尚未排期，方向优先级待定：

1. **湖仓写入与联邦计算**（LAKE-01..03、SQL-01）：`batch` 写入模式、Nessie 分支直写 Iceberg 后 merge、edit store materialize 与快照一致分页，验证「Trino 只读企业数据 + ontoforge 写 edit store」在真实湖仓上的边界。
2. **ontology 表达力**（SCHEMAX-01..05）：interfaces、shared properties、struct、value types、runtime 派生属性。
3. **治理闭环**（GOV-01）：AI 提议 ontology 变更 → 人审批 → 生效，落在已有的 scope 审计与 fail-closed 门禁上。
4. **检索与运行时**（SEARCH-01..02、FUNC-01..02、SUB-01、SCEN-01、BRANCH-01、SDK-01、GQL-01）：按 v1.0 遗留的 overlay 基础与 edit log 格式推进。

下一 milestone 的范围需经 `/gsd-new-milestone` 重新讨论与排期；v1.0 的残余缺口（Phase 05 生产 webhook 联调、Phase 06 审计时序拆分）应先作为 v1.x 的技术债进入 backlog。

## Context

- 设计参考：Foundry 官方文档镜像（`docs/foundry/`，含 2026-10-15 补抓的 118 页参考），以及三个开源项目的调研（`.planning/research/ref-*.md`）。只借设计不借命名（ADR-0002）。
- 能力规格与优先级：`.planning/research/foundry-docs-spec.md`；技术选型：`.planning/research/rust-stack.md`。
- 关键差异化：lint 引擎（把 Foundry best practices / anti-patterns 变成静态检查）、单二进制 dev 体验、MCP 一等公民、写路径强制完整性与审计。
- edit log 格式在 MVP 定死，后续 scenario / branch 以它为 overlay 基础，后改成本高。
- 待 spike 的技术点（进入对应 phase 前验证）：`serde-saphyr` 按路径取 span、`cel` 静态类型检查、`rmcp` 动态 tool 列表、`sea-query` JSON 表达式与表达式索引、`postgresql_embedded` 在 Windows 上的表现。

## Constraints

- **Tech stack**：Rust 1.98，edition 2024，cargo workspace（ADR-0011）；axum + utoipa、sqlx、sea-query、cel、rmcp — 已在 ADR-0003～0011 锁定
- **Platform**：必须在 Windows 11 上 `cargo build` / `cargo test` 通过（本机不依赖 Docker），生产部署目标 Linux
- **Testing**：Windows 本机默认测试只依赖 SQLite；Postgres 测试本机用 `postgresql_embedded`，CI 用 service container
- **Naming**：项目 / crate 前缀 `ontoforge`；标识符遵循 ADR-0002，禁止沿用参考仓库的标识符
- **License**：不复制参考仓库代码（bkn-foundry 含 OpenBKN License 文件）

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| 品牌名 ontoforge（ADR-0001） | 规避 Palantir "Foundry" 商标风险 | ✓ Good — 仓库、crate 与 tag 前缀统一为 ontoforge |
| 应用 + Agent 双出口定位 | 同一核心同时服务业务系统与 AI agent，MCP 是 REST 之上的薄封装 | ✓ Good — 已由 Phase 7 REST/HTTP/stdio 同权限与持久证据验证；详 Phase 7 UAT |
| YAML/JSON → 单一 IR（ADR-0003） | LLM 友好、diff 友好、line:col 诊断 | ✓ Good — Phase 1 交付，多文档 YAML 需项目级 yaml.schemas（G-1-2，Phase 1 gap closure 已闭合） |
| Postgres 生产 + SQLite dev/test（ADR-0004） | 单二进制开发体验 + 生产可靠性，conformance 测试保证一致 | ⚠️ Revisited — 被 ADR-0016 取代：业务数据经 Trino，PG 只存系统数据与 edit store |
| 写入只经 Action + 不可变 edit log（ADR-0004） | 审计与后续 scenario / branch 的基础 | ✓ Good — Phase 3 edit log 不可变 + Phase 5 Action 唯一写路径已交付并验收 |
| CEL 做表达式（ADR-0006） | 非图灵完备、可静态检查、进程内求值 | ✓ Good — Phase 1 静态检查 + Phase 3 `Pred` IR 与差分 proptest 已交付 |
| 自研 RBAC，fail-closed（ADR-0008） | 单二进制，不依赖外部授权服务 | ⚠️ Revisited — AuthZ 被 ADR-0018（scope 模型）取代 |
| Functions 延后到 v2（ADR-0010） | MVP 只定 trait，控制范围 | ✓ Good — 按计划延后，已进入下一 milestone 候选 |
| Trino 主引擎 + 分级 SQL（L0 / L2），ontoforge 只带连接能力 | 复用企业湖仓与联邦查询；可验证的 agent 权限边界 | ✓ Good — ADR-0015..0020 已落地（Phase 2 引擎、Phase 4 L0 读、Phase 6 L2 与审计） |
| 绑定独立成 `SourceBinding` 文件、`backing` 必填无默认（ADR-0021） | 可选默认值拦不住同时漏写 backing 与绑定 | ✓ Good — Phase 2 交付 |
| 被忽略的 fixture 诊断降为 Info（ADR-0022） | 同一份绑定 dev 用替身、prod 对 Trino 复核是常态；W 级会让 `--deny-warnings` 在任何 Trino 配置下失败 | ✓ Good — Phase 2 UAT 用户选定 |
| 私有数据源不按数据库类型单列（ADR-0032） | `sqlserver` 只是 Trino 暴露的一类 catalog，与 `mysql` / `hive` / `iceberg` 并列，代码里没有按数据库类型分叉的路径；ENGINE-05 与 Phase 02 措辞统一为「私有 Trino 数据源」 | ✓ Good — Phase 2 交付，用户 2026-11-08 拍板；产品层措辞仍待追改（见 Current State 残余缺口） |
| 机器可判定部分与人工验收部分显式切分，结论位固定 `VERDICT: PENDING`（02-11 gap closure） | 机器结果只作证据，judgment 结论只能由人签字，执行者不得代签，也不得因门禁通过而吸收为 closed | ✓ Good — Phase 2 验收，G-02-B/C/D 于 2026-11-08 签字关闭 |
| milestone v1.0 以 override 方式关闭，残余缺口作为技术债承接 | 9 个 phase 全部 `phase_complete && verification_status=passed`，79/79 需求 Complete；审计的 dated `gaps_found` 结论不因关闭而改写 | ✓ Good — `closeout_type=override_closeout`，3 项审计开放项经 acknowledge 记录为延后事项，见 `.planning/MILESTONES.md` 与 STATE.md Deferred Items |

决策全文见 `.planning/adr/`。

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-11-08 after v1.0 milestone*
