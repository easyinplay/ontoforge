# Phase 07.1 执行主计划

## Goal

补齐 STORE-05 的 source OneToOne 报告，入口、计数和不阻塞读取合同以 `07.1-CONTEXT.md` 为准。T1 指针为 `.planning/ROADMAP.md` 与 `.planning/REQUIREMENTS.md`；本文件只记录本 phase 的执行依赖。

## Tasks

- [x] 消费 CONTEXT、RESEARCH、PATTERNS、VALIDATION、EDGE-ACCEPTANCE 与前序 source/edit 证据。
- [x] 按原锁定合同写入 3 个 PLAN 草稿，建立 GOAL / REQ / RESEARCH / CONTEXT 来源覆盖。
- [x] 3 份 GSD frontmatter / structure 检查通过；7 个task必要字段、顺序依赖、威胁唯一性及文尾完整性通过。
- [x] 用户2026-11-04选A；加入整轮总deadline，定稿3个PLAN并同步最终VALIDATION任务映射。
- [x] 主会话独立 plan-checker；11个fails_when绑定修正，18/18通过。
- [x] Wave 1：07.1-01，CLI → metadata → typed printer → engine → warning tracer 与逻辑对象计数；SUMMARY 4714e3d，21个独立GREEN。
- [x] Wave 2：07.1-02，候选过滤、opt-in、样本边界与未完成/取消路径；SUMMARY b942b55，77个独立测试通过。
- [x] 审查修正：SQL排序别名冲突，490adcb修复，SQLite/Trino真实RED/GREEN及两路独立review复签；详07.1-ALIAS-DECISION.md。
- [x] Wave 3：07.1-03，真实 Trino、用户文档和受影响回归；最终01aa2df workspace1525/0/119，SUMMARY及分类证据已提交。
- [x] 最终回归修正：6be5c08，deadline虚拟时间、注释与helper lint；定向/CLI lib/engine、Clippy/fmt通过，两路review复签。完整workspace仍归Wave3门。
- [x] 独立VERIFICATION passed18/18、UAT10/10与TDD/security通过，主会话已依报告关闭STORE-05。

## Current Phase

**Status:** complete
主会话维护共享计划与T1，executor只提交分配的源码/测试/每plan ledger和SUMMARY。Cargo单owner，`CARGO_BUILD_JOBS=2`。用户untracked与其他worker修改保留。

### 完整回归新增任务：monitor启动取消测试

- [x] 只在crates/ontoforge-engine/tests/l2_monitor.rs复现固定80ms取消不能保证pending已存在的时序；原workspace507/1/14失败保留。受控probe响应延迟只限该fixture模式，不改生产。
- [x] 取消同步到system.runtime.queries观测到达（生产先登记pending再run_checks），明确abort并等待任务结束；验证Stopped、真实DELETE、目标query移除、随后能重新Enabled并shutdown，保留已有清理时限。
- [x] 单目标RED/GREEN、完整l2_monitor与静态门、独立审查及最后workspace均通过；WR-01已关闭，无生产修改。

## Next Step

三份SUMMARY与产物/同分支commits/原始日志spot-check通过。Nyquist矩阵24项有证据，UAT原生10/10 complete，TDD gate violations0、代码open0、安全6/6。独立verifier已passed18/18且原生verification.status通过，native phase.complete已执行；当前准备07.1 PR/CI与合并，之后转MinIO合同补证。旧Phase1–7指纹未修改。

## Errors Encountered

| Error | Resolution |
|---|---|
| 仓库根无磁盘 AGENTS.md / RTK.md | 使用本会话用户提供的 AGENTS.md；不生成替代文件。 |
| target/phase071-planner-skills.json 不是 JSON | 实际内容是 planner persona；按文本消费，额外 agent-skills 查询为空。 |
| 大批文本读取超过工具输出上限 | 后续限定具体章节和路径，使用已读取研究事实，停止扩大探索。 |
| findings的scoped patch匹配失败 | 旧文本含误插的不可见字符；修正匹配文本后成功，不改其他文件。 |

2026-11-04：边界文档迁移为07.1-EDGE-ACCEPTANCE.md，避免GSD API coverage命名误识别；正文哈希相同，原路径及原因见findings。


### 独立收尾单元：engine测试辅助代码lint

前序Clippy已修复注释与CLI测试问题，工具推进后新发现engine cardinality测试6项lint。当前executor已达到其单task三次修正上限，停止该修正循环；主会话按明确新诊断安排独立单文件单元，owner为phase071_lint_cleanup，仅crates/ontoforge-engine/tests/cardinality_check.rs。修正1处format_push_string及5个无await trait helper，保留测试数据/断言与生产。该单元不Cargo/Docker/commit；原executor继续独占Cargo做deadline/CLI lib检查，完成单文件修改后统一Clippy、fmt及workspace验证，再验收该单元。机械测试helper调整TDD不适用，不disable lint。
