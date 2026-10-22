# Phase 4 task plan

## Goal

按 `.planning/ROADMAP.md` 的 Phase 4 交付 QUERY-01..10。当前先完成研究、GSD plan-phase、独立计划检查，再进入 execute。详细实现任务以本目录 `04-*-PLAN.md` 为真相源；本文件只跟踪计划与执行顺序。

## Tasks

- [x] P0：隔离 `gsd/phase-04-scoped-l0-read` 分支，核对 Phase 3 UAT 2/2、VERIFICATION 8/8 与新 HEAD CI 三 job；前置依赖：Phase 3 完成。
- [x] P1：从 `.planning/adr/0020-query-execution-rust-orchestrated-overlay.md` ingest 决策到 `04-CONTEXT.md`，关联 scope、edit store 与设计文档；验收：不重开已有 ADR 的决定。
- [x] P2：研究 `crates/ontoforge-query/`、`crates/ontoforge-engine/`、`crates/ontoforge-storage/`、`crates/ontoforge-auth/` 的复用点与风险，产出 `04-RESEARCH.md`、`04-PATTERNS.md` 与 `04-VALIDATION.md` 草案；依赖 P1。
- [ ] P3：运行 GSD plan-phase 4，由独立 planner 生成 `04-*-PLAN.md`，plan-checker 核对 QUERY-01..10、Trino 查询形态、scope 安全与验证命令；依赖 P2。
- [ ] P4：把最终 PLAN 的文件所有权、wave 顺序和验收命令蒸馏到本文件，更新 `progress.md` 与 `.planning/STATE.md`；依赖 P3。

## Acceptance

- 每条 QUERY-01..10 都有计划与可执行验证；核心叠加和鉴权逻辑标记 TDD。
- 计划含 production tracer、SQLite 与真实 Trino 的测试边界、越权 conformance、延迟预算与失败处理。
- 独立 plan-checker 通过；ROADMAP 只保留 phase 概览和 plan 指针，STATE 保持 digest。

## Next Step

由 GSD planner 按 `04-CONTEXT.md`、`04-RESEARCH.md`、`04-PATTERNS.md` 和 `04-VALIDATION.md` 生成 Phase 4 的 tracer-first PLAN。
