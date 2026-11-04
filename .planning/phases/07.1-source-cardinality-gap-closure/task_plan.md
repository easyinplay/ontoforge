# Phase 07.1 执行主计划

## Goal

补齐 STORE-05 的 source OneToOne 报告，入口、计数和不阻塞读取合同以 `07.1-CONTEXT.md` 为准。T1 指针为 `.planning/ROADMAP.md` 与 `.planning/REQUIREMENTS.md`；本文件只记录本 phase 的执行依赖。

## Tasks

- [x] 消费 CONTEXT、RESEARCH、PATTERNS、VALIDATION、EDGE-COVERAGE 与前序 source/edit 证据。
- [x] 按原锁定合同写入 3 个 PLAN 草稿，建立 GOAL / REQ / RESEARCH / CONTEXT 来源覆盖。
- [x] 3 份 GSD frontmatter / structure 检查通过；7 个task必要字段、顺序依赖、威胁唯一性及文尾完整性通过。
- [x] 用户2026-11-04选A；加入整轮总deadline，定稿3个PLAN并同步最终VALIDATION任务映射。
- [ ] 主会话独立 plan-checker；修正有证据的缺口。
- [ ] Wave 1：07.1-01，CLI → metadata → typed printer → engine → warning tracer 与逻辑对象计数。
- [ ] Wave 2：07.1-02，候选过滤、opt-in、样本边界与未完成/取消路径。
- [ ] Wave 3：07.1-03，真实 Trino、用户文档和受影响回归。
- [ ] 独立 VERIFICATION + TDD/security review 后，由主会话更新 STORE-05 的 T1 状态。

## Current Phase

**Status:** ready_for_plan_checker
规划 owner 只写本目录 PLAN、task_plan/progress/findings 和 VALIDATION；不构建、不改生产源码、不提交。执行阶段 Cargo 单 owner，`CARGO_BUILD_JOBS=2`。用户 untracked 与其他 worker 修改保留。

## Next Step

由主会话派独立plan-checker审查3个PLAN。通过后主会话按branching_strategy=phase准备07.1分支、use_worktrees=false，分配单Cargo owner再按wave执行；planner不提交或构建。

## Errors Encountered

| Error | Resolution |
|---|---|
| 仓库根无磁盘 AGENTS.md / RTK.md | 使用本会话用户提供的 AGENTS.md；不生成替代文件。 |
| target/phase071-planner-skills.json 不是 JSON | 实际内容是 planner persona；按文本消费，额外 agent-skills 查询为空。 |
| 大批文本读取超过工具输出上限 | 后续限定具体章节和路径，使用已读取研究事实，停止扩大探索。 |
| findings的scoped patch匹配失败 | 旧文本含误插的不可见字符；修正匹配文本后成功，不改其他文件。 |
