# Phase 7 task plan

## Goal

交付 ROADMAP Phase 7 的 REST、OpenAPI、Rust/TS client、MCP、serve --dev、release 与跨阶段 L2-04/SCOPE-07 验收。范围与需求真相源为 `.planning/ROADMAP.md`、`.planning/REQUIREMENTS.md`；本文件跟踪顺序与阶段验收。

## Tasks

- [x] P0：确认 Phase 6 的 owner 状态为 passed，保留 PR #7 不合并约束。
- [ ] P1：完成 GSD discuss。用户已确认的决策保存于 `07-DISCUSS-CHECKPOINT.json`，尚待 dev 数据保留方式答复；完成后蒸馏为 `07-CONTEXT.md`。
- [ ] P2：工程审查服务生命周期、stdio 桥接与健康契约，完成 rmcp 动态工具列表 spike；形成研究与可执行 PLAN。
- [ ] P3：按 PLAN 实施，核心授权、状态与数据逻辑强制 TDD，实施交给 subagent。
- [ ] P4：验证 REST/MCP 授权一致性、dev 演示、真实 L2 生命周期与健康、三平台 release 构建；通过正式审查和 GSD 验收后关闭阶段。

## Ownership

主会话拥有共享计划和阶段讨论文档；实施文件在 PLAN 阶段明确分配。当前尚无实现任务。

## Acceptance

以 ROADMAP Phase 7 六条 Success Criteria 和 REQUIREMENTS 为准。L2-04/SCOPE-07 在 Phase 7 证据齐备前仍 Pending。Phase 6 CI 只证明 `2c9eee6`。

## Next Step

等待用户选择 serve --dev 重启后默认保留或重置数据，然后完成讨论并准备工程审查。
