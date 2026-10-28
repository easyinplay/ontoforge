# Phase 7 findings

- 2026-10-28：两项用户决策已蒸馏到 `07-CONTEXT.md`，完整选项见 `07-DISCUSSION-LOG.md`，不重复询问。ADR-0007/0009/0019/0028 已固定框架、DTO、toolset 和 ACL 生命周期边界。
- Grok 上轮有界咨询超时，但返回了建议正文，仅作为部分第二意见：建议 stdio 认证桥接同一服务、dev 数据默认持久化、部分 L2 降级在健康正文表达。来源为 Phase 6 findings 的 Phase 7 预查条目。health HTTP 状态尚未接受；不采纳“所有非 200 都会触发重启”的泛化推断。
- 本轮仅记录讨论和状态，不修改业务实现，按项目判据跳过新 TDD；以 JSON 解析、git diff --check 和 Phase 6 verification.status 验证。ralph-loop 本机此前确认不可用，本轮未运行或声称运行该工具。
- 跳过 office-hours/plan-ceo-review：本轮延续已获批 milestone 的既定 Phase 7 范围，不涉及新产品方向。Phase 层 discuss 正在进行，复杂架构在 P2 进入 plan-eng-review；实现前按任务判据判断 brainstorming。

## Open Decisions

- 用户已确认两项运行方式，产品讨论无剩余问题。剩余项均为工程审查事项。
- 工程审查：HTTP /health 的部分 L2 降级状态码、服务 shutdown 顺序、桥接认证与断连行为。均须服从既有 fail-closed、审计与权限合同。

## 只读勘查与第二意见

- startup scout 已完成，只读、未跑 Cargo。API/server/client 尚为占位；复用入口与合同指针见 CONTEXT。
- `ActionService::execute_inner` 在 `crates/ontoforge-action/src/lib.rs:258` 重新读数据库定义；`EditStore::history` 在 `crates/ontoforge-storage/src/history.rs:112` 重新读缓存。跨进程 reload 及单请求一致快照是本阶段接线风险，尚未复现为具体漏洞。
- Grok session 91385 返回 `ETIMEDOUT` / `SIGTERM`，不是成功完成。部分建议：进程存活与能力健康分开，L2 禁用时维持核心服务；先停止接入、排空请求和审计，再关闭 monitor/连接；Action 重试需幂等键。其建议用 MCP request id 作为幂等键不能直接接受，协议 ID 与持久 command_id 的生命周期不同；需沿用现有 Action 幂等合同。seed/reset 部分输出被截断，无完整结论。
- gstack preamble 已执行：显式 checkpoint、无 push、telemetry off。检测到工具更新，本阶段不引入无关全局工具升级。项目规则优先：已有范围已批准，继续完整范围，不重复要求批准 SDK/release。
- 已通过 planning-with-files resolver 显式选择 `.planning/phases/07-rest-mcp-shell`，不会恢复其他 phase 的同名计划。
- Context7 primary-source 查询确认 axum graceful shutdown / extensions 与 rmcp 动态列表入口；详情见 `07-ENG-REVIEW.md`。尚未执行 rmcp 编译 spike，也未选定依赖版本。
- 工程审查 Step 0 已完成。D1 健康状态选项通过 request_user_input_async 发出；Architecture 尚未完成，后续三节尚未进入，不宣称 gate passed。
