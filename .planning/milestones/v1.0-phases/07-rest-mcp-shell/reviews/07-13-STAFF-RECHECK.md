---
phase: 07-rest-mcp-shell
reviewed: 2026-11-03T04:14:40Z
depth: deep
review_head: f835752feff2d011be1843195fce208c87da78c8
review_protocol: gstack-review-spawned-session
files_reviewed: 28
files_reviewed_list:
  - crates/ontoforge-api/Cargo.toml
  - crates/ontoforge-api/src/dto.rs
  - crates/ontoforge-client/src/lib.rs
  - crates/ontoforge-server/src/lib.rs
  - crates/ontoforge-server/src/context.rs
  - crates/ontoforge-server/src/state.rs
  - crates/ontoforge-server/src/mcp.rs
  - crates/ontoforge-server/src/mcp_l2.rs
  - crates/ontoforge-server/src/mcp_sessions.rs
  - crates/ontoforge-server/src/lifecycle.rs
  - crates/ontoforge-server/src/rest.rs
  - crates/ontoforge-engine/src/lib.rs
  - crates/ontoforge-engine/src/trino/cancel.rs
  - crates/ontoforge-engine/src/trino/l2_monitor.rs
  - crates/ontoforge-engine/src/trino/mod.rs
  - crates/ontoforge/src/serve.rs
  - crates/ontoforge/src/mcp.rs
  - clients/typescript/src/index.ts
  - crates/ontoforge/tests/cli_l2_identity.rs
  - crates/ontoforge/tests/cli_stdio.rs
  - crates/ontoforge-server/tests/mcp_transport.rs
  - crates/ontoforge-server/tests/mcp_l2_defaults.rs
  - crates/ontoforge-server/tests/http_actions.rs
  - crates/ontoforge-server/tests/support/mod.rs
  - crates/ontoforge-engine/tests/l2_monitor.rs
  - crates/ontoforge-engine/tests/trino_resilience.rs
  - crates/ontoforge-client/tests/contract.rs
  - clients/typescript/tests/contract.test.mjs
findings:
  critical: 0
  warning: 0
  info: 0
  total: 0
status: clean
original_findings:
  CR-01: resolved
  CR-02: resolved
  CR-03: resolved
  CR-04: resolved
  CR-05: resolved
phase_acceptance: pending
---

# Phase 7 Staff Review Recheck

原报告五项 BLOCKER 的根因在冻结的 `f835752` 已关闭，本次定向源码复签没有发现剩余 BLOCKER 或 WARNING。此结论是原五项及修复 diff 的代码复签，不代表 07-13 全部动态门禁通过，不批准合并 PR #7、Release 或 npm 发布。

## Scope And Evidence

- 读取原 `07-13-STAFF-REVIEW.md`、phase 的 `task_plan.md` / `progress.md` / `findings.md` 和 07-06 关闭合同，按 caller/callee 追踪五项修复；未扩大为整个分支的全量审查。上列文件包含针对相关实现、diff 与测试断言的检查，不宣称每个未改动函数均经深审。
- 使用 gstack `/review` 的核心、Security、API Contract、Testing、Maintainability checklist，遵守 spawned 模式。未启动新 subagent、外部 Codex/Grok、Greptile 或浏览器。
- 工作目录无 `AGENTS.md`、`.codegraph/`、`.codex/skills/`、`.agents/skills/`；适用会话中的 AGENTS 规则。GSD `query agent-skills gsd-code-reviewer --skills-only` 没有额外 skill。遵守 `.gitignore`，未将生成产物当作独立源码审查证据。
- 稳定修复为 `513a2f6`（L2 能力）、`100976a`（原子 Action 准入）、`cf2a589`（wire）、`9aeb3ae`（engine cleanup）、`0dd3995`（transport/shutdown）、`f835752`（stdio ID/握手）。最终检查确认生产相关路径与 `f835752` 没有未提交差异。
- 本 lane 未运行 Cargo、测试 binary、Docker 或远程 CI。以下动态结果均标为 owner 报告；本 lane 独立读取了相应测试的实际断言。rmcp 的本地 registry 源码也核对了默认 `ServerHandler::initialize` 设置 peer info 并执行协议协商的行为。

## Narrative Findings (AI reviewer)

无新增未解决发现。原问题的关闭证据如下。

### CR-01: [BLOCKER, resolved] L2 独立执行身份与 proof 准入

**源码证据：** `crates/ontoforge/src/serve.rs:200` 附近安装独立 L2 engine；`crates/ontoforge-server/src/state.rs:144` 的 setter 拒绝不具备 L2 health/proof 的普通 engine；`crates/ontoforge-server/src/mcp.rs:85` 六工具统一取该能力，没有 ontology fallback。`crates/ontoforge-engine/src/trino/mod.rs:450` 的发送路径在 admission lock 下再次检查 proof；`describe_tables` 通过相同 `execute` 路径。

**断言证据：** `cli_l2_identity.rs` 通过生产 `run_with_shutdown` 装配，检查全部六工具的 `x-trino-user == l2`；缺失能力与失败 proof 不发送 SQL。另在同一 host 验证健康 provider 不能代替执行能力、NotVerified/AllowedProbeFailed/真实 61 秒 Expired 均拒绝六工具，并保留 L0 可用性。没有把直接用 L2 engine 构造测试 host 代替生产装配证据。

### CR-02: [BLOCKER, resolved] transport 当前认证与初始化 owner

**源码证据：** `crates/ontoforge-server/src/lib.rs:168` 在整个 `/mcp` service 外层安装认证；`crates/ontoforge-server/src/context.rs:313` 每次验证当前 token、forged headers 及已有 owner，在 initialize 响应暴露 session ID 前绑定初始主体。`mcp_sessions.rs:78` 的 `verify` 不会重新认领失效 entry；Missing 返回 404，Mismatched 返回 403。工具执行仍重建当前请求 snapshot；DELETE 成功回收 binding。

**断言证据：** `mcp_transport.rs:9` 覆盖无 token initialize、GET/DELETE/取消通知与 forged headers；`:50` 覆盖首次工具调用前另一主体 POST/GET/DELETE 被拒绝、原 owner 仍可用；TTL/容量测试验证失效 session 对两个主体均不可 verify；删除后的重用返回 404。TTL/容量是 owner 数据结构测试，不夸大为真实等待 30 分钟的 transport 测试。

### CR-03: [BLOCKER, resolved] 定向取消与 stdio 握手

**源码证据：** `crates/ontoforge/src/mcp.rs:164` 先本地处理 initialize，使用 rmcp 默认协商并以上游协议为支持上界，不重复转发初始化；普通请求取 `handle.id`，直接 pin `await_response`，没有 detached waiter。`crates/ontoforge-server/src/mcp.rs:448` 后的独立读/L2 分支监听请求与服务取消 token；Action 使用 `spawn_action` 独立登记，取消响应等待不 abort 已接受写入，也没有自动重发。

**断言证据：** `cli_stdio.rs:18` / `:23` 对 `2025-11-25` 与 `2025-03-26` 分别启动真实 CLI bridge，断言协商版本与 tools capability；同时提交本地字符串 ID 和数值 777，取消字符串请求后数值请求成功，durable audit 恰好一条 cancelled、一条 success，EOF 正常退出。`mcp_transport.rs:201` 在 Action 已进入 source 后取消请求并触发 shutdown，确认 Action 提交且没有重放，独立读最终 cancelled。旧的 absent-server 测试不再承担活跃取消证明。

### CR-04: [BLOCKER, resolved] 有界关闭与完整 cleanup 所有权

**源码证据：** `crates/ontoforge-server/src/context.rs:40` / `:55` 以同一 mutex 原子检查/登记 Action 与停止准入。`lib.rs:205` 在等待 HTTP/SSE 完成前启动关闭流程；先等已接受 Action 及其依赖，再触发共享 read/SSE token。`lifecycle.rs:9` 的总预算为 10 秒，Action/read/HTTP 截止 5 秒、audit 6 秒、monitor 8 秒、cleanup 10 秒；前段失败仍执行后续有界清理，最后保留失败结果。

`crates/ontoforge-engine/src/trino/cancel.rs` 在 spawn 前同步登记任务计数，Drop 与 `cancel_now` 的 DELETE 都可等待，失败保留且 404 视作已清除。`lifecycle.rs:71` 在有界 monitor shutdown 前保留 `L2MonitorCleanup`，最后并行等待 core、L2 与独立 monitor ontology 的 cleanup。`l2_monitor.rs:47` / `:103` 覆盖 monitor shutdown future 被取消后的 Drop 重试：重试受跟踪，drain 检查所属 generation 的未清理 probe，较新 generation 不归旧 handle。recoverable probe obligation 与普通 QueryGuard 的 sticky failure 分开，恢复不会被旧 probe 错误永久污染。

**断言证据：** 真正保持打开的 SSE 会在 shutdown 后结束；stuck Action 超时仍关闭 audit；Action panic 返回失败；被接受 Action 先完成后才取消独立读。engine 测试检查延迟 DELETE、并发 drain waiter、取消 `cancel_now` waiter、500/超时/404、每实例隔离及原 query Timeout 语义。monitor 测试主动用 50ms timeout 取消 shutdown，确认 retained handle 等待 Drop 重试，成功与失败均可见；另验证 generation 归属和重复 drain 不重发 DELETE。生产 serve 测试还检查延迟/失败 DELETE 对退出时机与退出码的影响。

本 lane 中途指出的 monitor timeout 遗漏已由 `9aeb3ae` 的 retained cleanup handle 及 `0dd3995` 的最后阶段接线关闭，不作为剩余问题重复列出。

### CR-05: [BLOCKER, resolved] 完整 receipt/history wire

**源码证据：** `crates/ontoforge-server/src/rest.rs:659` 明确转换 history 的全部字段；`:712` 映射每个 item 的所有 edits，保留 ordinal/kind/typeName/pk/operation/versionAfter。共享 DTO 与 Rust serde 解码同形，`crates/ontoforge-client/src/lib.rs:443` 不再以空值掩盖缺失字段；`clients/typescript/src/index.ts:74` / `:97` 验证 receipt/history 必填字段和整数边界。API crate 依赖仍仅 serde、serde_json、utoipa。

**断言证据：** Rust 与 TS 真实 HTTP 合同检查一 item 两 edits 的全部字段、commandResult 与幂等重放一致、history event/time 实值。server 的遮蔽历史测试检查 `maskedChanges` 保留字段名、`values` 不泄漏隐藏值、普通 caller 的 params 为 null。坏 shape、ordinal 边界和合法 zero-edit receipt 有专门断言；history 沿用当前 scope 的遮蔽结果，没有强行复制 receipt 内容。

## Verification Limits And Remaining Gates

| 证据 | 归属与状态 |
| --- | --- |
| API/client 18、server HTTP 3、TS contract 9；相关 Clippy/typecheck/生成漂移 | owner 报告通过；本 lane 读源码与断言，未复跑 |
| L2 初期定向 17（含真实 61 秒 TTL） | owner 报告通过；不当作后续 cleanup 增补的最终回归 |
| `9aeb3ae` engine lib 55 + monitor 19 + resilience 31，共 105 | owner 报告通过；本 lane 独立检查最终实现与相关断言 |
| transport 11、stdio 5（含旧新协议） | 主会话报告通过；未由本 lane 复跑 |
| 最终 CLI L2/cleanup 10 | owner 运行通过，主会话复核 `target/phase07-l2-evidence/cli-final.log`；61.67s，0失败 |
| 完整 workspace、真实 PG/Trino 服务脚本、Linux CI、07-14 | 本次未证明通过，仍由主会话统一验收 |

本报告的 `clean` 只表示上述源码复签未留下发现。Phase 7 验收状态仍为 pending，不能由本报告推导整阶段完成或发布许可。只新增此报告，未修改生产、测试或共享 planning 状态；提交由 orchestrator 负责。
