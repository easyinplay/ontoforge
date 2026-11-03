# Phase 7 task plan

## Goal

交付 ROADMAP Phase 7 的 REST、OpenAPI、Rust/TS client、MCP、serve --dev、release 与跨阶段 L2-04/SCOPE-07 验收。范围与需求真相源为 `.planning/ROADMAP.md`、`.planning/REQUIREMENTS.md`；本文件跟踪顺序与阶段验收。

## Tasks

- [x] P0：确认 Phase 6 的 owner 状态为 passed，保留 PR #7 不合并约束。
- [x] P1：完成 GSD discuss。用户决策见 `07-CONTEXT.md`，备选项审计见 `07-DISCUSSION-LOG.md`；已消费临时检查点。
- [x] P2：工程审查、研究与 rmcp spike 已完成；16 份 PLAN 经第二轮独立 checker 通过，详 `reviews/07-PLAN-CHECK-R2.md`。
- [ ] P3：按 PLAN 实施，核心授权、状态与数据逻辑强制 TDD，实施交给 subagent。07-11 已选方案 A：生成器隔离 workspace 使用兼容 TypeScript 版本，主 client 保留 TypeScript 7。
- [ ] P4：验证 REST/MCP 授权一致性、dev 演示、真实 L2 生命周期与健康、三平台 release 构建；通过正式审查和 GSD 验收后关闭阶段。

## Ownership

主会话拥有共享计划、STATE、ROADMAP、REQUIREMENTS；每份 PLAN 的 files_modified 由单一 executor 负责。workflow.use_worktrees=false，所有 PLAN 顺序执行。

## Acceptance

以 ROADMAP Phase 7 六条 Success Criteria 和 REQUIREMENTS 为准。L2-04/SCOPE-07 在 Phase 7 证据齐备前仍 Pending。Phase 6 CI 只证明 `2c9eee6`。

## Next Step

07-13已由10175f6四必需CI成功与本地fresh36/36验收关闭。下一步执行07-14的实际native三平台构建/archive/checksum与无checkout完整安装smoke，范围见07-14-PLAN.md、reviews/07-14-PREFLIGHT.md。主会话维护状态与提交，executor仅拥有四个计划文件和独立实施记录；不创建Release、不发布npm、不合并PR。

P3执行中：已验收15/16，07-13已关闭，07-14执行中。Windows方案A与后续修复提交/证据统一见`07-13-SUMMARY.md`，不重新执行已完成的环境诊断。

07-11 已完成：`70a9ed9`、`5c76afe`；独立 generator workspace、TypeScript 7 client、无损 codec 与 5 项真实 HTTP contract 均通过。主 package 已移除 `openapi-typescript`，避免 clean install peer 冲突。

## 07-13 当前验收清单

- [x] GT-01：客户端取消在shutdown前独立对账audit，mutation RED→6/6 GREEN、Clippy通过且独立复签clean；详reviews/07-CANCEL-CAUSALITY-FIX.md。
- [x] API WR-01：history OpenAPI9状态合同RED→API8项GREEN、生成/typecheck/build/lint通过，独立复签clean。
- [x] Staff CR-01：独立L2执行身份与proof准入接线，真实生产assembly回归。
- [x] Staff CR-02/03/04：MCP全transport认证、初始owner绑定、定向取消和有界SSE shutdown。
- [x] 关闭补强：原子Action准入/注册、按07-06既定顺序排空、等待Trino取消任务且清理失败可见。
- [x] Staff CR-05：完整多edit receipt与history显式wire转换，OpenAPI/Rust/TS真实合同一致。
- [x] 修复workspace历史测试RowValues + Sync类型回归并完成整体验证。
- [x] SDK无损JSON capability与公开入口修复，8项contract及生成/typecheck/build通过。
- [x] MCP参数schema修复，新增真实回归及相邻14项测试通过。
- [x] Rust client数值边界修复，4 unit与5 HTTP contract、client Clippy通过。
- [x] Rust history分页参数修复及真实HTTP分页回归（`7fa2401`，4 unit+6 HTTP contract）。
- [x] CLI与全workspace Clippy、embedded-pg Clippy、workspace回归（1373 passed / 0 failed / 113 ignored）。
- [x] 完整服务脚本（包括取消读audit、Rust/TS、credential scan与cleanup）真实通过（`fd24beb`，unset OS，exit0，Windows state exit0）。
- [x] SDK/MCP/client交叉复签无剩余blocker（最终冻结`f835752`；整体动态门禁另计）。
- [x] Phase 7 Draft PR #8的旧head0a3e359全部4必需CI job通过（run41600147824），包括dev Unix权限实际运行。
- [x] 双轴审查11/12修复已定向验证、独立复签并提交，RF05为0b4a661；详修复合同与SUMMARY。
- [x] 整合head972c7be的Windows/Linux CI与真实服务门禁通过（41608551893）；后续实现仍需新head门禁。
- [x] RF-05 MCP参数信封按ADR0029实施，36项通过且独立复签clean。
- [x] RF-01 drift按ADR0030与工程审查实施及验收，最终27项、本地fresh36项和10175f6四必需CI通过。

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
