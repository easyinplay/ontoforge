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

`07.1-EDGE-ACCEPTANCE.md` 两条 resolved/explicit 分别进入 01 的重复运行只读 truth 和 02 的中断/截断 truth。外部 source 允许并发变化，结果只描述各自查询观测，不建立跨表快照或持续唯一性保证。

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


## 执行状态更新：07.1-02验收后

01/02的实际结果以各自SUMMARY及07.1-evidence为准；上文规划时的pending不是当前执行状态。02的77个独立测试全部通过，真实deadline RED由daf3911记录、GREEN由7ecc32b实现，SUMMARY b942b55。03只增加既有行为的真实Trino验收与文档，不新增生产行为，TDD不适用；如发现生产bug，先保留真实assert RED再修复。共享计划归主会话；executor独占Cargo/Docker，代码/安全review只读并行。独立review模型resolver给出的gpt-5.6系列不可用，沿已采用的gpt-6-astra fallback，effort分别high/xhigh；不声称模型等价。


## GSD verify-pre 文档命名冲突

2026-11-04，api-coverage.verify-pre因唯一的*-COVERAGE.md被识别为API矩阵，误将两行边界用例表解释为empty capabilities。主会话先核对gate源码，只向Claude Code提供需求/事实，其独立推荐语义重命名及引用迁移，与主会话判断同向。将07.1-EDGE-COVERAGE.md改为07.1-EDGE-ACCEPTANCE.md，SHA256前后一致；活动PLAN/规划引用同步，既有边界记录与验收要求不变。保留原gate阻塞JSON，不改全局工具、不关闭gate、不虚构API能力。CC prompt/result位于target/phase071-coverage-name-*.{txt,json}，实际MiniMax-M3.1-Flash[1m]。使用未stage的文件移动避免与executor共享Git index冲突，最终Git仍可按相同内容识别rename。后续gate必须对非空完整PLAN重新检测，不能以改名本身宣称通过。


## 冷构建预算与独立意见

完整workspace前两次各600秒均在编译阶段exit124，没有test result；持续出现新目标和产物，不把它描述为测试失败或watch mode。主会话独立判断应区分构建与测试预算，向Claude Code仅提供事实后，其同向推荐必要时用同scope的--no-run先完成构建，再运行原完整测试命令；保留600秒有界管理。采纳该条件流程，不改全局配置或测试语义。第三次若直接成功，立即使用其真实计数收尾，不额外重跑。若第三次仍编译超时，最多4次600秒--no-run缓存续跑，每轮记录目标/产物进展并回收本次进程树；连续3轮同目标无产物进展、真实编译错误/测试失败，或该预算耗尽时停止盲目重试并调查，保持验收未完成。构建exit0不等于测试通过，最终仍需完整cargo test --workspace --locked实际运行成功。独立原始咨询归07.1-evidence/build-budget-independent-opinion.json。


## 完整回归中的deadline测试时序

第三次workspace构建完成8m17s后，首个CLI lib suite为24 passed/1 failed，exit101；失败cardinality_total_deadline_is_shared实际contexts.len=1，期望2。失败只证明第二target没进入，不能证明生产重置deadline或诊断丢失。生产使用tokio::time::Instant，测试用真实30ms/1s sleep和200ms预算，调度敏感是源码支持的解释；原始失败未记录足够现场来精确断言第一次sleep被延迟的机制。保留原日志，不以一次重跑成功覆盖失败。

主会话独立判断使用虚拟时钟隔离测试调度噪声，向Claude Code仅提供需求/事实/代码，其独立推荐相同方向：CLI自身dev Tokio增加既有test-util feature，测试用start_paused=true，生产和断言不变。授权executor调整两个使用BudgetEngine的deadline单测（共享预算与配置测试），都只验证逻辑时间；不增加依赖版本、不改变Cargo.lock、不放大全局timeout、不串行化workspace。原workspace失败是测试可靠性缺陷，不能冒称新的生产RED。需目标测试、完整CLI lib、最终并发workspace重新通过，若再失败记录实际contexts/timeout/diagnostics并调查。

Clippy新增doc_markdown失败为schema/diag.rs的OneToOne注释缺反引号，允许只修本phase新增注释的标记并保留原exit101；文档/lint修正TDD不适用，Clippy必须真实重新通过。所有后续审查需覆盖dev-only test-util及纯注释变更，不沿用旧manifest零diff声明。

## 构建预算耗尽后的观测修正

4轮600秒no-run于06:06:20Z全部结束124，已停止Cargo。只读metadata列11个package、141个test=true目标；当前05:15:02Z后保留138个exe（3,846,044,160 bytes），mtime桶27/27/11/23/50是当前分布，不能当历史精确新增数。已确认health_trino_it/http_action_safety重复生成，但原因及最后截断target未知；目标名称碰撞使exe前缀匹配不能证明完成。原始事实归plan03-build-readonly-diagnosis.json。

仅提供事实向Claude Code取得独立方案（实际MiniMax-M3.1-Flash[1m]），核心判断与主会话同向：缺少Cargo freshness与目标级观测，继续相同无观测重试无依据。采用一次600秒同scope --no-run --message-format=json -vv诊断，记录artifact fresh、前后完整快照及自有进程CPU/target观测；成功后才执行原完整workspace测试600秒。未授权再次自动追加4轮，也不改profile/features/jobs或缩减目标。此为诊断观测调整，无生产/测试语义变更，TDD不适用；验收仍须实际workspace通过。

对独立回答保留技术判断：进程退出不能排除此前锁争用，无磁盘错误不能排除IO瓶颈，metadata/mtime不是fresh证明；不采用不同build/test命令耗时相减推断链接成本，也不采用bench/example范围推测。完整prompt/result、实际模型、原始结果哈希及采纳边界见build-limit-independent-opinion.json。该诊断轮再次124或出现真实错误即停止，不能标构建或phase通过。

## 最终回归暴露的文档诊断码集合遗漏

可观测构建exit0后，原完整workspace于06:19:50Z至06:22:44Z实际exit101，54个harness汇总450 passed/1 failed/14 ignored。唯一失败doc_diagnostic_codes_exist原文the document mentions unknown code ONT-W18。主会话已读doc_type_table.rs:166起的完整校验：known只包含硬编码V01..V20和KNOWN_RULE_IDS，required也未列W18。schema diag.rs已有W18、online.rs实际发此warning、source-binding.md正确记录在线且非LintRule边界；故缺口定位于文档保护测试的已知码集合，不能通过把W18塞进离线规则表修复。原日志完整保留；独立咨询只提供事实和代码，不附修复方案。

独立意见同向：仅修文档测试known/required以显式纳入codes::W18，不改生产或离线规则表，不引入全局ALL_CODES抽象。executor新增该测试文件ownership，先复现实际RED再改并取得GREEN；原workspace失败是测试校验集合遗漏，不冒称生产行为RED。此次跳过brainstorming：根因明确、既有集合模式、无API或算法设计；生产TDD不适用，保留现有测试RED/GREEN证据。最终workspace必须完整运行并据日志重新计数，不能采纳独立回答预计451/0/14的提前停止数量；不无理由重跑生产/seed未变的真实Trino。完整prompt/result和采纳边界见doc-code-independent-opinion.json。

## 后继回归：monitor取消测试的生命周期前置条件

834abd0完整workspace真实exit101，60个harness507 passed/1 failed/14 ignored；W18已通过，失败旧l2_monitor的monitor_cancelled_startup_cleans_up_and_stays_disabled，1秒等待deletes报probe cancelled: Elapsed(())。该测试用固定80ms取消启动，未证明已登记pending；生产start_probe先登记pending再发system.runtime.queries。fixture观察该查询才有可靠前置条件。原失败现场没有submissions/observations，故准确时序unknown，不声称生产泄漏或已证实仅慢机。

仅事实独立咨询支持同方向：同步到观测查询后取消，等待task abort生效，保留Stopped、DELETE和重启验证，新增对应query移除断言。授权executor单测试文件范围，先用该case专用受控probe响应延迟复现错误前置条件，再修同步机制；不能修改生产、全局timeout或串行化suite。此为已有生命周期合同的测试可靠性修正，跳过新brainstorming，保留真实受控RED/GREEN但不冒称生产RED。原始咨询及采纳边界见monitor-startup-independent-opinion.json。

49eb4d5独立review新增WR-01：fixture先deletes++再锁queries删除，test仅等计数后立即断言删除存在合法交错，源码可证但未实际复现。再次纯事实CC意见同向，授权在原1秒窗口内同时等待DELETE计数增长与对应query消失，保留原后置断言、锁不跨await；不修改fixture顺序或生产。执行中的完整workspace不强杀，保留其结果；修正后的最后源码仍需完整workspace，不采纳无针对性100次循环。咨询见monitor-assertion-independent-opinion.json。
