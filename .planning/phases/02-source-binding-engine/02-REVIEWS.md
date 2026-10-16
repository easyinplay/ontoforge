---
phase: 2
reviewers: [gemini, codex]
reviewed_at: 2026-10-16T17:33:59Z
plans_reviewed: [02-01-PLAN.md, 02-02-PLAN.md, 02-03-PLAN.md, 02-04-PLAN.md, 02-05-PLAN.md, 02-06-PLAN.md, 02-07-PLAN.md, 02-08-PLAN.md, 02-09-PLAN.md, 02-10-PLAN.md]
models:
  gemini: "unknown"
  codex: "gpt-6-sol (reasoning=high)"
model_sources:
  gemini: "unknown"
  codex: "banner"
---

# Cross-AI Plan Review — Phase 2

<!-- gsd:plan-revision-conflicts:begin -->
## Plan-Revision Conflicts
<!-- gsd:plan-revision-conflicts:end -->

## Gemini Review

gemini review failed or returned empty output. stderr:
Error authenticating: IneligibleTierError: This client is no longer supported for Gemini Code Assist for individuals. To continue using Gemini, please migrate to the Antigravity suite of products: https://antigravity.google
    at throwIneligibleOrProjectIdError (file:///C:/Users/easyi/AppData/Roaming/npm/node_modules/@google/gemini-cli/bundle/chunk-MLY4WQFO.js:311202:11)

---

## Codex Review

> [reviewed-without-source-citations] This reviewer declared source-grounded evidence but cited no file:line source evidence, so it reviewed the pasted plan text only — down-weight its verdict in the Consensus Summary.

本次尝试使用终端、编辑器界面和 subagent 读取仓库，但运行环境均报错。因此以下是**覆盖全部 10 个 plan 的计划文本审查**，不是源码核验；无法提供可信的源码 `path:line` 引用。风险项均按待核实问题表述。

## 02-01

- **Summary：** 先迁移必填 `backing`，再开展绑定工作，顺序合理。依据：[02-01-PLAN.md](D:/DataSpellProjects/ontoforge/.planning/phases/02-source-binding-engine/02-01-PLAN.md) Task 1。
- **Strengths：** 要求保留既有快照的 code、severity、message、help，能约束迁移造成的行为漂移。
- **Concerns：** **MEDIUM，待核实：**按文本位置改写 YAML、JSON 和 Rust 内联字符串，能否覆盖故意损坏的样例及不同字符串写法？现有快照检查命令也只识别特定 JSON 行格式。
- **Suggestions：** 先列出并审阅实际命中文件；迁移后按旧快照逐字段比较，单独核对跳过的文件。
- **Risk Assessment：MEDIUM。** 改动面广，迁移方法是主要风险。

## 02-02

- **Summary：** printer 到 SQLite 的 tracer 和性质测试覆盖了关键 SQL 入口。依据：[02-02-PLAN.md](D:/DataSpellProjects/ontoforge/.planning/phases/02-source-binding-engine/02-02-PLAN.md) Tasks 1–3。
- **Strengths：** NUL、引号、方言差异和 SQLite 实际回读都有明确验收。
- **Concerns：** **HIGH，计划内部矛盾：**`PrintedSql::new` 设为 `pub(crate)`，同 crate 的非 printer 模块仍能构造它，不能形成宣称的编译期约束。**MEDIUM，待核实：**SQLite 先 `fetch_all` 再按 `max_rows` 截断，无法限制取数时的内存。
- **Suggestions：** 将构造权限限制在 printer 模块；为 SQLite 执行实现取数过程中的行数和字节上限。
- **Risk Assessment：HIGH。** 两个核心不变量尚未被所写机制完整保证。

## 02-03

- **Summary：** 独立绑定文件、profile 和完备性校验有连贯的实现路径。依据：[02-03-PLAN.md](D:/DataSpellProjects/ontoforge/.planning/phases/02-source-binding-engine/02-03-PLAN.md) Tasks 1–3。
- **Strengths：** profile 值只用于已加载映射查找，并为路径、引用和缺绑定安排了诊断。
- **Concerns：** **MEDIUM，待核实：**`SourceBinding` 的 `DefKey.api_name` 仅为 `profile/target`；若 object type 与 link type 可同名，定位和重复检查会碰撞。JSON 示例使用绑定对象数组，需核实当前 loader 是否接受该文档形状。
- **Suggestions：** 增加同名跨类型目标和 JSON 数组的加载测试；必要时把 target kind 纳入绑定键。
- **Risk Assessment：MEDIUM。** 对外文件格式定型成本高，上述两处需要先证实。

## 02-04

- **Summary：** Trino 协议主路径、配置和认证拆分清楚。依据：[02-04-PLAN.md](D:/DataSpellProjects/ontoforge/.planning/phases/02-source-binding-engine/02-04-PLAN.md) Tasks 1–3。
- **Strengths：** 禁止重定向、校验 `nextUri` origin、隔离两套身份，针对凭据泄露路径提供了测试。
- **Concerns：** **HIGH，待核实：**Basic/Bearer 的 HTTPS 约束写在配置解析中，但公开的 `TrinoConnection` 可直接构造，测试还明确绕过该约束。显式 `next_uri_origins` 若允许从 HTTPS 降到 HTTP，也可能发送 Authorization。
- **Suggestions：** 在 `TrinoEngine::new` 和每次发送认证头前验证目标 scheme；测试直接构造和跨 scheme `nextUri`。
- **Risk Assessment：HIGH。** 安全约束需要覆盖库 API，而不只是配置入口。

## 02-05

- **Summary：** SQLite fixture 让在线校验获得可在 Windows 运行的端到端路径。依据：[02-05-PLAN.md](D:/DataSpellProjects/ontoforge/.planning/phases/02-source-binding-engine/02-05-PLAN.md) Tasks 1–2。
- **Strengths：** 缺列诊断、decimal 不舍入和 fixture 上限都有具体测试。
- **Concerns：** **MEDIUM，待核实：**替身的物理类型由绑定和 IR 推导，再与同一 IR 比对；该路径不能独立证明真实源列类型兼容。02-03 又允许两个类型绑定同一物理表，逐绑定 `CREATE TABLE` 是否会重复建表？
- **Suggestions：** 明确 fixture 模式可验证的边界；为共享物理表加入去重或明确拒绝，并测试冲突定义。
- **Risk Assessment：MEDIUM。** 本地 tracer 有价值，但不能替代真实元数据验证。

## 02-06

- **Summary：** 不可用分类、重试和取消覆盖了 Trino client 的主要故障路径。依据：[02-06-PLAN.md](D:/DataSpellProjects/ontoforge/.planning/phases/02-source-binding-engine/02-06-PLAN.md) Tasks 1–3。
- **Strengths：** 超时、future 被丢弃和响应上限分别设有 DELETE 验证。
- **Concerns：** **HIGH，待核实：**计划把 POST 收到 503 解释为“明确未处理”并重试；网关返回 503 未必证明查询没有提交，可能产生重复的昂贵查询。
- **Suggestions：** 核对 Trino 协议对 POST 重试的保证；没有明确保证时，仅重试可证明请求未发出的连接失败，并记录首次 POST 后结果未知的情形。
- **Risk Assessment：HIGH。** 重试策略直接影响共享集群负载与查询结果确定性。

## 02-07

- **Summary：** 五条规则覆盖了计划中的离线类型契约。依据：[02-07-PLAN.md](D:/DataSpellProjects/ontoforge/.planning/phases/02-source-binding-engine/02-07-PLAN.md) Tasks 1–3。
- **Strengths：** 每条规则要求触发与不触发快照，并覆盖 `ontoforge.toml` 配置。
- **Concerns：** **MEDIUM，待核实：**fixture 表头解析失败时计划静默跳过 W17，`lint` 仍可能显示 clean；这与“缺列给出离线提示”的验收表述需要区分。
- **Suggestions：** 对损坏的 fixture 给出可定位诊断，或在文档与验收中明确 W17 只适用于成功解析的 fixture。
- **Risk Assessment：MEDIUM。** 主要风险是静默漏报。

## 02-08

- **Summary：** 把 CLI 与 drift 统一到 `compare_binding`，有利于避免两套兼容规则分叉。依据：[02-08-PLAN.md](D:/DataSpellProjects/ontoforge/.planning/phases/02-source-binding-engine/02-08-PLAN.md) Tasks 1–3。
- **Strengths：** decimal 整数位规则、未知类型 fail-closed，以及文档表格对照测试都很具体。
- **Concerns：** **MEDIUM，待核实：**大小写冲突仅报 warning，但后续按小写匹配列时可能选到不确定的一列。`Healthy` 状态似乎不保留 warning，调用方可能看不到精度或时区提醒。
- **Suggestions：** 对含糊的列匹配报 error；在 `DriftReport` 中保留非降级 warning。
- **Risk Assessment：MEDIUM。** 兼容表覆盖较全，剩余风险集中在状态表达。

## 02-09

- **Summary：** 该 plan 补齐真实 Trino 选择、退出码和显式 PK 检查。依据：[02-09-PLAN.md](D:/DataSpellProjects/ontoforge/.planning/phases/02-source-binding-engine/02-09-PLAN.md) Tasks 1–2。
- **Strengths：** Trino 不可用时退出码 3、无成功摘要，且禁止自动回退到 SQLite。
- **Concerns：** **HIGH，待核实：**`sqlite_dir` 通过 ATTACH 后设置 `query_only` 被称为“只读打开”，需要确认文件实际上以只读模式打开。重复 PK 查询未排除 NULL；两个以上 NULL 可能同时进入重复值结果和单独的 NULL 计数。
- **Suggestions：** 以 SQLite 只读打开模式实现 `sqlite_dir`；重复查询加入 `pk IS NOT NULL`，保留单独的 NULL 计数查询。
- **Risk Assessment：HIGH。** 读写边界与 PK 诊断正确性都影响部署判断。

## 02-10

- **Summary：** CI 负责验证本机无法运行的真实 Trino 路径，方向符合阶段目标。依据：[02-10-PLAN.md](D:/DataSpellProjects/ontoforge/.planning/phases/02-source-binding-engine/02-10-PLAN.md) Tasks 1–3。
- **Strengths：** `--ignored` 执行数量门禁、真实 printer 语料和私有 fixture 隔离均有明确安排。
- **Concerns：** **HIGH，待核实：**Trino 使用 host network 连接 `localhost:5432`，PostgreSQL service 必须把端口发布到 runner；计划未明确写出。GitHub Actions 的 `permissions` 只能设在 workflow 或 job 层，不能只给开 issue 的 step 授权。真实 Trino golden 的预期值计划凭推导填写，首次 CI 未运行前不能算验收通过。
- **Suggestions：** 明写 PostgreSQL 端口映射；将 issue 操作放入独立 job 并授予 `issues: write`；把 PR integration 和 nightly 首次实跑通过列为 Phase 2 完成门禁。
- **Risk Assessment：HIGH。** CI 编排尚有可预见的阻断点，真实执行证据也尚未取得。

**跨 plan 判断：** Phase 2 计划把 ENGINE-02 的 `/health`、BIND-03 的启动及定时 drift、ENGINE-04 的 `serve --dev` 留到 Phase 7。验收时应在 `.planning/REQUIREMENTS.md` 对这三项分别标明 Phase 2 交付范围，避免将整个 requirement 标为完成。源码行号、现有 loader 行为及 CI 配置均因本次工具故障未核实；上述审查不能替代要求中的源码核验。

---

## Consensus Summary

只有 codex 一个 lane 产出了评审，且带 `[reviewed-without-source-citations]` 标记（运行环境读不到仓库，只审了 plan 文本），因此下列结论按降权处理，每条在修订前须对照源码核实。Gemini lane 失败：`IneligibleTierError`（Gemini CLI 个人版不再受支持，需迁移 Antigravity）。

### Agreed Strengths
（单一 reviewer，无交叉一致项）codex 认可：tracer 顺序、printer 性质测试、`nextUri` 同源与禁止重定向、`compare_binding` 统一 CLI 与 drift、`--ignored` 执行数量门禁。

### Agreed Concerns
（单一 reviewer，按严重度排序，均需源码核实）
1. HIGH 02-02：`PrintedSql::new` 为 `pub(crate)`，同 crate 非 printer 模块仍可构造，编译期约束不成立；SQLite `fetch_all` 后截断不限内存。
2. HIGH 02-04：Basic/Bearer 的 HTTPS 约束只在配置解析层，`TrinoConnection` 可直接构造绕过；跨 scheme `nextUri` 可能带 Authorization 降级到 HTTP。
3. HIGH 02-06：POST 收 503 即重试，网关 503 不能证明查询未提交，可能重复执行。
4. HIGH 02-09：`sqlite_dir` 需以只读模式打开而非仅 `query_only`；重复 PK 查询未排除 NULL。
5. HIGH 02-10：PG service 端口映射未写明；`permissions` 只能 job 级授予，开 issue 需独立 job；Trino golden 首跑前不能算验收通过。
6. MEDIUM 02-03：`SourceBinding` 键 `profile/target` 未含 target kind，object 与 link 同名会碰撞；JSON 数组文档形状需核实 loader。
7. MEDIUM 02-05：fixture 物理类型由 IR 推导再与 IR 比对，不能独立证明类型兼容；共享物理表重复建表。
8. MEDIUM 02-07 / 02-08：fixture 表头解析失败静默跳过 W17；大小写冲突仅 warning 但匹配不确定；`Healthy` 状态丢失 warning。
9. MEDIUM 02-01：文本位置改写迁移对故意损坏样例的覆盖需核实。
10. 跨 plan：ENGINE-02 `/health`、BIND-03 启动 / 定时 drift、ENGINE-04 `serve --dev` 留到后续 phase，REQUIREMENTS traceability 需分别标 Phase 2 交付范围。

### Divergent Views
无（仅一个有效 lane）。
