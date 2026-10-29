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

主会话拥有共享计划、STATE、ROADMAP、REQUIREMENTS；每份 PLAN 的 files_modified 由单一 executor 负责。workflow.use_worktrees=false，所有 PLAN 顺序执行。

## Acceptance

以 ROADMAP Phase 7 六条 Success Criteria 和 REQUIREMENTS 为准。L2-04/SCOPE-07 在 Phase 7 证据齐备前仍 Pending。Phase 6 CI 只证明 `2c9eee6`。

## Next Step

P3 执行中：有效完成07-01至07-04，共4/16；07-04 SUMMARY `a4b9576` Self-Check PASSED，真实HTTP Action/history/result 2/2、安全重放并发3/3、server回归/Clippy/fmt通过。方案A已实现：宿主跟踪、owned permit、HTTP超时504后任务继续、同command_id显式查询/replay。下一步唯一ready为07-05 OpenAPI/wire合同；auto_advance=false，用户授权范围仍为Phase 7全部剩余12份PLAN。

## 07-05 Execution

- [x] 05-1 RED: add OpenAPI/serde contract tests and capture semantic failure (`e73623a`)
- [x] 05-1 GREEN: add shared DTO schemas and OpenAPI document generation (`44cac2d`)
- [x] 05-1 REFACTOR: rerun focused API contract tests and clean schema registration (`487073b`)
- [x] 05-2 RED: add deterministic export tests for `serve --export-openapi` (`f5e547e`)
- [x] 05-2 GREEN: add static export path and checked-in `docs/openapi.json` (`3fafb7f`)
- [x] 05-2 REFACTOR: rerun export and API snapshot tests (`487073b`)

Verification:

- `cargo test -p ontoforge --lib --locked openapi_ -- --nocapture`: exit 0, 1 passed, 0 failed.
- `cargo test -p ontoforge-api --test openapi_snapshot --locked -- --nocapture`: exit 0, 4 passed, 0 failed.
- `cargo fmt --all -- --check`: exit 0 after formatting.

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
