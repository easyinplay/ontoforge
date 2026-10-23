# Phase 5 task plan

## Goal

交付 `.planning/ROADMAP.md` Phase 5 的 ACTION-01..12。ROADMAP 与 `.planning/REQUIREMENTS.md` 是目标和验收的真相源；本文件只记录执行顺序、所有权与当前节点。

## Tasks

- [x] P0：从已完成的 Phase 4 HEAD 建立 `gsd/phase-05-online-action` 分支，检查工作区和现有 phase 产物；验收：原有未跟踪文件未改动。
- [x] P1：审阅 ADR-0016/0018/0024/0026、Phase 3/4 API 与 Action 设计，按 GSD discuss-phase 锁定仍开放的 Phase 5 决策；产出 `05-CONTEXT.md` 与 `findings.md`；依赖 P0。
- [x] P2a：对跨 query、auth、storage 的事务与宿主边界做独立工程审查，结果写入 `05-ENG-REVIEW.md`；依赖 P1。
- [ ] P2b：正式 gstack `plan-eng-review` gate；当前因 `AskUserQuestion` 不可用而阻塞，不能记为通过；关键模块 PR 前回补，并同时运行 `/review`。Phase 5 的一次性替代治理为 `05-ENG-REVIEW.md` E1-E7 + GSD plan-checker PASSED；不推广到后续 Phase。
- [x] P3：GSD plan-phase 研究、生成 7 份 `05-*-PLAN.md`，由独立 checker 核对 ACTION-01..12 与 RED/GREEN、真实 PG/Trino 和失败回滚验收；更新 `task_plan.md`、`progress.md`、`05-VALIDATION.md`；依赖 P2a。验收：checker `VERIFICATION PASSED`、7/7 决策覆盖、36 条 verify 命令无探针告警。
- [x] P4：按 GSD execute-phase 的 7 个 wave 执行 `crates/ontoforge-action/`、`crates/ontoforge-query/`、`crates/ontoforge-storage/` 及宿主/CI 接线；逐子任务 TDD 与测试证据写入各 SUMMARY；依赖 P3。具体文件所有权见 `05-01..07-PLAN.md`。
- [ ] P5：GSD verify-work、phase-level verification、安全复审及匹配代码 SHA 的 CI 验收；更新 ROADMAP/STATE 的完成指针；依赖 P4。
- [x] P5a：关闭 `05-VERIFICATION.md` 的 source 多别名和 Query row budget 缺口。所有权：`crates/ontoforge-action/src/source.rs`、`tests/source.rs`；`crates/ontoforge-query/src/lib.rs`、`tests/pk_read.rs`，以及共享 helper 的最小调用点。针对性 RED/GREEN、Action source 11/11、Query crate 全量、workspace 组合回归与 Clippy 已通过。
- [x] P5b：关闭 webhook 生产驱动缺口。所有权：`crates/ontoforge/src/{cli,lib,webhook_worker}.rs`、`crates/ontoforge-engine/src/config.rs`、对应 CLI/配置集成测试及两个 Cargo.toml 的最小依赖。独立常驻 `ontoforge webhook-worker --server-config` 读取可信 `[webhook]` 设置并复用 EditStore；真实 CLI 测试覆盖投递、503 重试、进程重启恢复；Ctrl+C 退出尚未自动化测试。提交 `31f7c49`。
- [ ] P5c：05-02 PLAN 的 `src/execute.rs` 假指针已校正到实际 `src/lib.rs`；复跑独立 verification、GSD UAT 与关键审查，远端 CI 和正式 gstack gate 分开记录。
- [x] P5d：关闭新代码复审的 outbox 时间 WARNING。`d4517e0` 先取得两项有效 RED，再逐条 claim、完成时计时；SQLite outbox 11/11、真实 PG outbox 2/2、局部 Clippy/fmt/diff check 通过，独立代码复审 clean。
- [ ] P5e：修复远端 `ci.yml` run `39997950852` 的 PostgreSQL conformance 回归。`append_only_rejects_truncate_on_pg` 的三表 TRUNCATE 未包含 Phase 5 新增的两个 `of_edit_log` 外键引用表，实际先报 `0A000`。保持 `23000` trigger 断言，显式补齐依赖表；使用失败的远端 RED、真实 PostgreSQL GREEN、重新触发匹配新 SHA 的 CI 验收。所有权：`crates/ontoforge-storage/tests/conformance/mod.rs`；不得更改 migration。

## Acceptance

- ACTION-01..12 各有可执行的行为验收；事务回滚、并发冲突、幂等重放、scope 与 outbox 为高可靠性路径，按 TDD 验证。
- Phase 5 的测试区分 SQLite 本地、真实 PostgreSQL、需要 source 时的 Trino 与隔离故障注入；不以 skipped test 代替后端证据。
- 文档遵守 T1 摘要、T2 细节、T3 任务的单一真相源结构；STATE 保持少于 100 行。

## Next Step

P5e 先修复 PostgreSQL conformance 测试并重跑真实后端及远端 CI。`05-VERIFICATION.md` 为 human_needed；UAT 第 18 项、真实部署和正式 gstack gate 尚待确认，不更新 REQUIREMENTS 完成勾选。
