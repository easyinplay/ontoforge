# Phase 7 task plan

## Goal

交付 ROADMAP Phase 7 的 REST、OpenAPI、Rust/TS client、MCP、serve --dev、release 与跨阶段 L2-04/SCOPE-07 验收。范围与需求真相源为 `.planning/ROADMAP.md`、`.planning/REQUIREMENTS.md`；本文件跟踪顺序与阶段验收。

## Tasks

- [x] P0：确认 Phase 6 的 owner 状态为 passed，保留 PR #7 不合并约束。
- [x] P1：完成 GSD discuss。用户决策见 `07-CONTEXT.md`，备选项审计见 `07-DISCUSSION-LOG.md`；已消费临时检查点。
- [x] P2：工程审查、研究与 rmcp spike 已完成；16 份 PLAN 经第二轮独立 checker 通过，详 `reviews/07-PLAN-CHECK-R2.md`。
- [ ] P3：按 PLAN 实施，核心授权、状态与数据逻辑强制 TDD，实施交给 subagent。
- [ ] P4：验证 REST/MCP 授权一致性、dev 演示、真实 L2 生命周期与健康、三平台 release 构建；通过正式审查和 GSD 验收后关闭阶段。

## Ownership

主会话拥有共享计划和阶段讨论文档；每份 PLAN 的 files_modified 由单一执行 lane 负责，同 wave 无文件交集。业务实施尚未启动。

## Acceptance

以 ROADMAP Phase 7 六条 Success Criteria 和 REQUIREMENTS 为准。L2-04/SCOPE-07 在 Phase 7 证据齐备前仍 Pending。Phase 6 CI 只证明 `2c9eee6`。

## Next Step

下一步为 `$gsd-execute-phase 7`，从 `07-01-PLAN.md` 的真实认证读取 tracer 开始。当前 auto_advance=false，尚未启动执行。32 个 task 的验收映射见 `07-VALIDATION.md`，需求与边界映射见 `07-EDGE-COVERAGE.md`，最终计划审查见 `reviews/07-PLAN-CHECK-R2.md`。

## Execution Order

以下为 PLAN frontmatter 派生的 12 个 wave，详细依赖与跨切合同留在各 PLAN 中，不复制到 ROADMAP。

| Wave | Plans | 能力 |
|------|-------|------|
| 1 | 01 | 认证 HTTP 读取 tracer 与 serve |
| 2 | 02 | 原子 snapshot、Action/history 固定定义 |
| 3 | 03 | REST 读取与统一错误 |
| 4 | 04 | REST Action 与历史 |
| 5 | 05 | OpenAPI 与共享 wire 合同 |
| 6 | 06,15 | health/shutdown；typed L2 EXPLAIN |
| 7 | 07,10,11 | MCP 工具；Rust/TS clients |
| 8 | 08 | MCP L2 与 HTTP transport 安全 |
| 9 | 16 | MCP 可信默认值与 qualified 参数 |
| 10 | 09 | 真实 stdio 桥接 |
| 11 | 12,13 | 持久 dev 演示；真实 PG/Trino 验收 |
| 12 | 14 | 三平台发行产物与 smoke |

## Errors Encountered

- Grok session 91385 在 55 秒上限返回 ETIMEDOUT / SIGTERM；部分正文仅作第二意见，未当作完整审查。
- `C:/Users/easyi/.codex/rules/web-search.md` 不存在；已找到并读取 `C:/Users/easyi/.claude/rules/web-search.md`。
- GSD query commit 跳过已删除的检查点路径；通过显式 git add -u 单独记录删除，避免旧检查点留在仓库。
- `07-TEST-PLAN.md` 被 GSD 按 `*-PLAN.md` 误计为执行计划，已改名 `07-TEST-MATRIX.md` 并同步指针；没有把测试规划文件当作可执行 PLAN。
