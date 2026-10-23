# Phase 5 task plan

## Goal

交付 `.planning/ROADMAP.md` Phase 5 的 ACTION-01..12。ROADMAP 与 `.planning/REQUIREMENTS.md` 是目标和验收的真相源；本文件只记录执行顺序、所有权与当前节点。

## Tasks

- [x] P0：从已完成的 Phase 4 HEAD 建立 `gsd/phase-05-online-action` 分支，检查工作区和现有 phase 产物；验收：原有未跟踪文件未改动。
- [x] P1：审阅 ADR-0016/0018/0024/0026、Phase 3/4 API 与 Action 设计，按 GSD discuss-phase 锁定仍开放的 Phase 5 决策；产出 `05-CONTEXT.md` 与 `findings.md`；依赖 P0。
- [x] P2a：对跨 query、auth、storage 的事务与宿主边界做独立工程审查，结果写入 `05-ENG-REVIEW.md`；依赖 P1。
- [ ] P2b：正式 gstack `plan-eng-review` gate；当前因 `AskUserQuestion` 不可用而阻塞，不能记为通过；关键模块 PR 前回补，并同时运行 `/review`。Phase 5 的一次性替代治理为 `05-ENG-REVIEW.md` E1-E7 + GSD plan-checker PASSED；不推广到后续 Phase。
- [x] P3：GSD plan-phase 研究、生成 7 份 `05-*-PLAN.md`，由独立 checker 核对 ACTION-01..12 与 RED/GREEN、真实 PG/Trino 和失败回滚验收；更新 `task_plan.md`、`progress.md`、`05-VALIDATION.md`；依赖 P2a。验收：checker `VERIFICATION PASSED`、7/7 决策覆盖、36 条 verify 命令无探针告警。
- [ ] P4：按 GSD execute-phase 的 7 个 wave 执行 `crates/ontoforge-action/`、`crates/ontoforge-query/`、`crates/ontoforge-storage/` 及宿主/CI 接线；逐子任务 TDD 与测试证据写入各 SUMMARY；依赖 P3。具体文件所有权见 `05-01..07-PLAN.md`。
- [ ] P5：GSD verify-work、phase-level verification、安全复审及匹配代码 SHA 的 CI 验收；更新 ROADMAP/STATE 的完成指针；依赖 P4。

## Acceptance

- ACTION-01..12 各有可执行的行为验收；事务回滚、并发冲突、幂等重放、scope 与 outbox 为高可靠性路径，按 TDD 验证。
- Phase 5 的测试区分 SQLite 本地、真实 PostgreSQL、需要 source 时的 Trino 与隔离故障注入；不以 skipped test 代替后端证据。
- 文档遵守 T1 摘要、T2 细节、T3 任务的单一真相源结构；STATE 保持少于 100 行。

## Next Step

执行 `05-07-PLAN.md` 的 outbox worker、写入口边界与 CI；05-06 source、release、token 和真实 Trino 证据见 `05-06-SUMMARY.md`。ACTION-08/09/12 的计划级验收已完成，Phase 5 总体验证仍待执行。正式 gstack gate 仍为 BLOCKED，关键 PR 前回补。
