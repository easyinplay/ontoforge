# Phase 07.1 规划发现与来源覆盖

## 已验证边界

- `07.1-RESEARCH.md` 与 `07.1-PATTERNS.md` 提供当前接口和最近 analog；Level 0 discovery，沿已有 Rust / typed printer / Engine / CLI 模式，无新增依赖，无外部 API 接入。
- STORE-05 的 edit enforcement 已由 Phase 03-10/03-13 交付，本次只关闭 source 报告缺口；不能用 PK 检查代替 FK 基数。
- 当前 CLI online 摘要在 `crates/ontoforge/src/lib.rs::validate_online`，warning exit0 时仍有成功前缀，需要本 phase 精确修正并测试。
- schema contribution 列出的 ORM 路径均不适用；`diag.rs` 常量与专用测试 seed 不属于数据库产品 schema push。AI/UI contribution 不适用。assumption-delta 实际返回 detected=false。
- 规划只读已有来源；未运行测试，不把 analog 通过记录当本 phase 证据。

## tracer 尺寸裁决

完整 CLI → engine → CountDistinct 的首条路径跨越现有 AST、printer、模块导出、CLI 参数/online/摘要和诊断注册。按单 task 五文件硬拆会留下不可运行的水平层或迫使 SQL 职责移入 CLI。主会话及 CC 已同意首个完整 tracer 的必要尺寸例外，原始证据 `target/phase071-tracer-plan-review.json`；精确 ownership、单 owner 和明确 RED/GREEN 是条件。后续 task 均控制在五文件内。此裁决不改变用户合同，不减少验收。

## Decision traceability

CONTEXT 的 `Locked decisions` 未带 D-NN。D-01至D-10保留最初十条决策的稳定规划追踪别名，新增总deadline使用D-11，不因CONTEXT插入条目而重编号。D-01 opt-in及组合；D-02 原flag/default成本；D-03 source to-object映射；D-04 单列distinct与NULL；D-05 非referential integrity；D-06 typed printer；D-07 warning及预检；D-08 未完成、保留诊断及drain；D-09 20+1及适用范围；D-10 report/read隔离；D-11 cardinality整轮默认30秒总deadline，后续link只取剩余时限，metadata/check-pk沿原语义。这里只引用用户决策，不创设新产品合同。

## Specless probe

`07.1-EDGE-COVERAGE.md` 两条 resolved/explicit 分别进入 01 的重复运行只读 truth 和 02 的中断/截断 truth。外部 source 允许并发变化，结果只描述各自查询观测，不建立跨表快照或持续唯一性保证。

Prohibition recall 针对 STORE-05 先提出十个候选：默认全扫、重复物理行误报、NULL误报、错误target、样本无界、错误后假通过、取消泄漏、改写source、以报告阻断读取、误称引用完整性。precision 后均是已有明确合同或可测试的工程正确性，由任务/edge/安全测试接收；新增 values/safety/ethics 项为0。SQL注入属于通用安全，交 threat_model、typed quoting测试及独立安全复核，不创建 descriptor-less 人工禁止项。`must_haves.prohibitions` 为空列表是明确结果，不是省略 probe。

## TDD 与工具适配

核心 CLI/scanner 数据行为必须真实 Cargo RED → GREEN；编译错误、缺符号、fixture错误、零用例不算 RED。已满足的边界测试如首跑 GREEN，应如实写 regression evidence，不制造假 RED。纯文档/seed/对已有行为做跨backend验收没有新生产逻辑，记录 TDD 不适用并运行真实验证。

历史原生 nextest/JUnit 模式见 `07-01-evidence/nextest.toml` 与 `07-02-evidence/run.ps1`（均位于 `../07-rest-mcp-shell/`）。执行前核验工具仍可用；优先保留原始 Cargo日志及原生JUnit机器门。若本机工具不可用，遵循主会话已确认适配：记录 GSD classifier unsupported 与实际 Cargo assertion/计数复核，不伪称机器门通过、不安装新依赖或修改全局GSD。

## 来源审计

以下是用户2026-11-04选A后锁定合同的计划覆盖审计；COVERED只表示已排入任务，不表示实施或验证通过。旧定向AGREE仅作为历史，新决策依据为用户选择及 `07.1-INDEPENDENT-OPINION.md` 的独立比较。

| Source | ID | 项目 | Plan / Task | Status |
|---|---|---|---|---|
| GOAL | Goal | 显式source OneToOne报告且保持默认和读取行为 | 01-1、02-1 | COVERED |
| GOAL | SC1 | distinct对象、NULL、物理重复 | 01-1/01-2、03-1 | COVERED |
| GOAL | SC2 | opt-in、warning与退出码 | 02-1/02-3 | COVERED |
| GOAL | SC3 | typed SQL、限额、取消、SQLite/Trino | 01-1、02-2/02-3、03-1 | COVERED |
| REQ | STORE-05 | source报告；edit enforcement沿用Phase03证据 | 01-03；不重做edit | COVERED |
| CONTEXT | D-01 | 独立flag、online前提、可组合 | 01-1、02-1 | COVERED |
| CONTEXT | D-02 | 默认online/check-pk无新扫描 | 02-1 | COVERED |
| CONTEXT | D-03 | source to-object及profile映射 | 01-1、02-1 | COVERED |
| CONTEXT | D-04 | 单列distinct、NULL及物理重复 | 01-2、03-1 | COVERED |
| CONTEXT | D-05 | 非referential integrity边界 | 01-1、02-1、03-2 | COVERED |
| CONTEXT | D-06 | typed CountDistinct与私有SQL出口 | 01-1、02-2 | COVERED |
| CONTEXT | D-07 | warning注册、退出码、预检、摘要 | 01-1、02-1/02-3 | COVERED |
| CONTEXT | D-08 | 不完整V19、保留诊断、取消drain | 01-1、02-3 | COVERED |
| CONTEXT | D-09 | 20+1样本与候选范围 | 01-1、02-1/02-2 | COVERED |
| CONTEXT | D-10 | 不改变L0/Action/admission、非阻塞读取 | 01-1、02-3、03-2 | COVERED |
| CONTEXT | D-11 | 整轮默认30秒总deadline、递减剩余timeout | 01-1、02-3、03-2 | COVERED |
| RESEARCH | R1 | 复用现有stack、配置、profile、renderer | 01-1、02-1 | COVERED |
| RESEARCH | R2 | metadata可用BindingKey过滤及逐target保留结果 | 01-1、02-3 | COVERED |
| RESEARCH | R3 | CountDistinct projection/HAVING/order、稳定排序 | 01-1、02-2 | COVERED |
| RESEARCH | R4 | timeout/bytes/rows与truncated先行 | 01-1、02-3 | COVERED |
| RESEARCH | R5 | 全出口drain、drain失败V19 | 02-3 | COVERED |
| RESEARCH | R6 | W18候选冲突核查、非离线LintRule、binding定位 | 01-1、03-2 | COVERED |
| RESEARCH | R7 | display_value上限、20+1及full-scan说明 | 02-2、03-2 | COVERED |
| RESEARCH | R8 | quoted标识符、真实SQLite、SQL出口守卫 | 02-2 | COVERED |
| RESEARCH | R9 | 真实Trino专用seed、缺环境panic、非零计数 | 03-1 | COVERED |
| RESEARCH | R10 | 旧PK/printer/CLI回归、文档/证据闭合 | 01/02 verification、03-2 | COVERED |
| EDGE | E1 | 冻结数据重复运行只读且确定 | 01-2 | COVERED |
| EDGE | E2 | 当次观测、中断不假通过、取消 | 02-2/02-3 | COVERED |

## 用户选择与定稿

2026-11-04主会话按用户要求只给需求/事实取得独立B方案，再比较；用户明确选A。最终3个plan/7个task保留数据库COUNT DISTINCT、有界20+1输出、独立warning，并吸收D-11总deadline。没有纳入B的客户端分页归约、NULL PK/同PK多FK身份检查或OneToMany统计；NULL PK仍只遵循不计入distinct及独立check-pk合同。LIMIT与总deadline不承诺数据库物理扫描行数上界。

VALIDATION已映射到实际task：01-1 tracer、01-2对象语义/只读重复、02-1scope/flags、02-2采样/quoting、02-3失败/cleanup/总deadline、03-1真实Trino、03-2文档与受影响回归。新增C5a/C5b/C6分别验证剩余预算、真实CLI到期输出、默认/覆盖来源；C7验证cleanup失败。所有执行项仍pending。

总deadline留在CLI内部cardinality轮次，无新公共API或参数；默认值和覆盖来自现有query_context。现有engine dev-dependency具有tokio test-util，CLI没有显式启用；计划使用短预算和recording Engine传入timeout断言，不新增依赖或要求30秒慢测。
