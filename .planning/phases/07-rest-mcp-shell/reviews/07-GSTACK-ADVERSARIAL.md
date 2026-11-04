---
phase: 07-rest-mcp-shell
review_date: 2026-11-03
review_base: fd24beb
review_head: 972c7be
reviewer: fresh-adversarial-subagent
review_modes: [adversarial, red-team]
independent_reviewers: 1
status: no-new-findings
new_findings: 0
dynamic_tests_run: false
phase_acceptance: pending
---

# Phase 7 固定增量 adversarial 审查

Pre-Landing Review: No issues found.

本报告表示固定增量未发现新的高置信度可触发问题，不表示 Phase 7 验收通过。adversarial 与 red-team 是同一个 fresh reviewer 的交叉视角，不是两个独立 reviewer。按主会话记录使用可用 runtime fallback；未运行外部模型或另派 reviewer。

## 范围与方法

只读 `git diff fd24beb..972c7be` 和 `git show 972c7be:<path>` 的实现、相关测试与必要 caller/callee。覆盖 dev OS 锁与 Unix 权限、MCP L0/L2/Action、REST history 与 Action diagnostics、history helper、共享 DTO、Rust/TypeScript SDK。读取 gstack `review/checklist.md`、`review/specialists/red-team.md` 和 review skill，先检查并发、数据安全与边界，再检查错误传播和极端输入。未使用后续工作树实现作为证据。

## 检查证据

| 攻击或故障路径 | 固定 head 的静态证据 |
| --- | --- |
| 并发启动、活跃实例 reset、崩溃恢复 | `crates/ontoforge/src/dev.rs:84-91` 先获取 OS 锁再复核目录与 key，随后才允许 reset；`:322-327` 持有文件锁，未引入删除活跃 lock inode 的路径。 |
| L2 reserve 后取消、shutdown 与审计 ID | `crates/ontoforge-server/src/mcp_l2.rs:134-136,234-236` 在 source await 前同步 observer；`mcp.rs:570-571,671-672` 使用当前 trace ID。`crates/ontoforge-storage/src/read_audit.rs:318-323` 的 guard Drop 排入取消审计。此链路静态可追踪，不替代通知到达及 durable audit 的真实测试。 |
| history 组边界、末页和预算 | `crates/ontoforge-server/src/history_page.rs:18-24` 以严格更旧 seq lookahead；`:42-66` 只在完整 seq 组边界计算 UTF-8 JSON 页预算，首组超限失败；`:69-76` 从实际保留尾组派生 cursor。没有发现因截断同组而永久跳过条目的新路径。 |
| Action/存储错误经 MCP、REST 和 SDK 外发 | `crates/ontoforge-server/src/error.rs:78-108` 仅投影 ParameterErrors/CriteriaFailed；MCP 保留静态 code 和已有 request ID；Rust `lib.rs:283` 与 TypeScript `src/index.ts:errorEnvelope` 校验可选诊断字段。非法诊断拒绝解码属于已记录合同，不据此报成 HTTP 错误处理回归。 |

## 排除与限制

- 已知 GT01 仍由 Testing owner 处理：固定 head 的 `mcp_l2_cancellation_and_shutdown_keep_reserved_audit_ids` 在取消通知后立即 shutdown，不能单独证明通知已触发取消。本 reviewer 没有将它重新计为新发现，也未验证后续补丁。
- history lookahead 读取并解码完整旧 commit 组的开销与 P01 重合，且复用既有 storage API 是已批准合同；未做 benchmark，不升级为新性能结论。
- 检查到 L0 `QueryError::Source` 会抹平 engine Timeout/Cancelled 的分类。该行为在基线已存在，`07-03-PLAN.md:65` 将细分状态合同限定 REST；主会话确认本轮排除，不扩大固定增量。
- RF-01 drift owner、RF-05 嵌套 params 和 07-14 release 明确未实施，不重复作为新增 finding。
- 未运行 Cargo、服务、压力测试或跨平台测试。未修改实现、共享 planning 或提交 git。仅将本次只读审查证据按 planning-with-files worker 所有权规则写入本报告。

Recommendation: Accept this fixed diff's adversarial review because no new reproducible defect was identified beyond the already tracked GT01 cancellation-test gap; keep overall acceptance pending its independent verification.
