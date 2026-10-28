# Phase 7 findings

- 2026-10-29：研究已完成，rmcp 3.8.0 / axum 0.8.9 独立 spike 5 passed、0 failed，主会话已读取最终日志。现代无 session 协议、旧 session 主体绑定、取消与 EOF 均有 spike 证据；正式业务验收边界见 `07-RESEARCH.md`，不把 spike 代替阶段完成。
- 2026-10-29：plan:pre UI gate 返回 frontend=false、block=false；本阶段是服务外壳与平台 clients，不含产品 UI 或 LLM 推理，跳过 UI/AI 设计生成。仅规划文档同步，本次跳过新增业务 TDD；执行 PLAN 继续要求适用任务 Red-Green-Refactor。ralph-loop 未安装，不能声称运行；实际终端为 PowerShell。

- 2026-10-28：两项用户决策已蒸馏到 `07-CONTEXT.md`，完整选项见 `07-DISCUSSION-LOG.md`，不重复询问。ADR-0007/0009/0019/0028 已固定框架、DTO、toolset 和 ACL 生命周期边界。
- Grok 上轮有界咨询超时，但返回了建议正文，仅作为部分第二意见：建议 stdio 认证桥接同一服务、dev 数据默认持久化、部分 L2 降级在健康正文表达。来源为 Phase 6 findings 的 Phase 7 预查条目。health HTTP 状态现已由用户接受，见 CONTEXT D-03；不采纳“所有非 200 都会触发重启”的泛化推断。
- 本轮仅记录讨论和状态，不修改业务实现，按项目判据跳过新 TDD；以 JSON 解析、git diff --check 和 Phase 6 verification.status 验证。ralph-loop 本机此前确认不可用，本轮未运行或声称运行该工具。
- 跳过 office-hours/plan-ceo-review：本轮延续已获批 milestone 的既定 Phase 7 范围，不涉及新产品方向。Phase 层 discuss 正在进行，复杂架构在 P2 进入 plan-eng-review；实现前按任务判据判断 brainstorming。

## Open Decisions

- 用户已确认两项运行方式，产品讨论无剩余问题。剩余项均为工程审查事项。
- 工程审查 D1 已接受。继续核对服务 shutdown、桥接认证与断连、定义快照；均须服从既有 fail-closed、审计与权限合同。

## 只读勘查与第二意见

- startup scout 已完成，只读、未跑 Cargo。API/server/client 尚为占位；复用入口与合同指针见 CONTEXT。
- `ActionService::execute_inner` 在 `crates/ontoforge-action/src/lib.rs:258` 重新读数据库定义；`EditStore::history` 在 `crates/ontoforge-storage/src/history.rs:112` 重新读缓存。跨进程 reload 及单请求一致快照是本阶段接线风险，尚未复现为具体漏洞。
- Grok session 91385 返回 `ETIMEDOUT` / `SIGTERM`，不是成功完成。部分建议：进程存活与能力健康分开，L2 禁用时维持核心服务；先停止接入、排空请求和审计，再关闭 monitor/连接；Action 重试需幂等键。其建议用 MCP request id 作为幂等键不能直接接受，协议 ID 与持久 command_id 的生命周期不同；需沿用现有 Action 幂等合同。seed/reset 部分输出被截断，无完整结论。
- gstack preamble 已执行：显式 checkpoint、无 push、telemetry off。检测到工具更新，本阶段不引入无关全局工具升级。项目规则优先：已有范围已批准，继续完整范围，不重复要求批准 SDK/release。
- 已通过 planning-with-files resolver 显式选择 `.planning/phases/07-rest-mcp-shell`，不会恢复其他 phase 的同名计划。
- Context7 primary-source 查询确认 axum graceful shutdown / extensions 与 rmcp 动态列表入口；详情见 `07-ENG-REVIEW.md`。尚未执行 rmcp 编译 spike，也未选定依赖版本。
- 用户于 2026-10-28 选择 D1 的 `1`；四节工程设计审查现已完成，详 ENG-REVIEW。研究 spike、PLAN 和实现验收仍未完成。

## Grok 第二轮有界咨询

- 2026-10-28 session 41414：exit 0，无 signal/error，单轮、禁用工具/subagent/web。与上轮超时区分，本轮收到完整正文。
- 有效提醒：两个独立读语句不能保证 definition/scope 一致；保留 Action 的提交 fence，不自动重试写；在等待 Action 完成前不得取消其依赖的 source read。
- 不采纳的建议：把 replay 绑定原定义会改变 ADR-0018 的当前授权合同；新增 history 版本列前需注意现有 edit log header 已写入 def_version（`commit.rs` 的 `NewRecord`）。history 的问题是请求内读取一致性，并非已证实缺少持久版本。
- 不采纳的简化：仅增加宿主 joined SELECT，仍不能消除 Action/history 内部后续读取定义的入口；是否需要新增最小 snapshot-aware 入口须由源码核查确定。
- 需要核验而非直接接受：Grok 担心 L2 monitor 在 audit flush 后继续产生日志，现有 monitor 位于 engine crate，未查到它依赖 storage read_audit。不能据此改变关闭顺序。

## 本轮检查边界

- 仅更新决策/审查和计划文档，尚不新增业务测试；核心实现继续要求 TDD。
- 探查时尝试的 `query/src/budget.rs`、`query/src/limits.rs`、`action/src/types.rs`、`action/tests/versioned_submission.rs` 不存在；已改用 `rg --files` 获取实际布局，未据此断言功能缺失。
- scout 追加核查已返回：`begin_definition_tx` 与 scope_apply 共享 advisory lock，PG 普通 begin 不保证多次 SELECT 快照；最小方案是短事务复用该锁读取定义和完整 ScopeSet。Action/history 需最小 snapshot-aware 入口，旧 wrapper 保留。
- 原 `0002_edit_log.sql` / LogRecord 已存 def_version，idempotency 测试明确允许定义变化后回放原 receipt；monitor 没有 storage audit handle。Grok 对这三点的扩展建议已排除，未修改既有合同。
