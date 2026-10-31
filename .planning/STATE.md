---
gsd_state_version: "1.0"
current_phase: 7
current_phase_name: REST & MCP Shell
current_plan: 13
status: executing
stopped_at: Phase 7 07-13 real-service tests and CI verification in progress
last_updated: "2026-10-31"
last_activity: "2026-10-31: 07-13 SDK/MCP blocker fixes passed focused tests; full service, workspace and Linux CI gates in progress"
state_head: 150a6d2
progress:
  total_phases: 7
  completed_phases: 6
  total_plans: 88
  completed_plans: 86
  percent: 98
last_gate_trip: null
last_activity_desc: Phase 7 有效完成14/16，07-13使用Git Bash/native Docker继续真实验收；详phase SUMMARY
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-10-28)

**Core value:** 定义一次 ontology，应用和 agent 就能通过 REST 与 MCP 对同一份对象图做受控读写：写入一律经 Action 校验与授权，每次写入都可审计。
**Current focus:** Phase 7 — REST & MCP Shell

## Current Position

Phase: 7 (REST & MCP Shell) — EXECUTING
Current Plan: 13
Total Plans in Phase: 16
Status: Real-service verification in progress
Last activity: 2026-10-31: SDK 8/8、MCP 14/14通过；完整服务、workspace和Linux CI待验收

Progress: [██████████] 98%

## Performance Metrics

逐 plan 的耗时与文件数见各 `phases/*/NN-*-SUMMARY.md`。
**Per-Plan Metrics:**

| Plan | Duration | Tasks | Files |
|------|----------|-------|-------|
| Phase 07 P05 | 33m12s | 2 tasks | 7 files |
| Phase 07 P10 | 65m | 2 tasks | 5 files |
| Phase 07 P08 | 38m | 2 tasks | 6 files |
| Phase 07-rest-mcp-shell P16 | 43m | 2 tasks | 5 files |
| Phase 07 P09 | 70 | 2 tasks | 7 files |
| Phase 7 P12 | 28m | 2 tasks | 9 files |

## Accumulated Context

### Decisions

决策真相源为 `.planning/adr/`；Phase 1-6 的实施证据见各自 `SUMMARY.md`、`VERIFICATION.md` 和 git 历史。Phase 7 运行方式见 `phases/07-rest-mcp-shell/07-CONTEXT.md`，工作顺序见同目录的 `task_plan.md`。
- [Phase 07]: OpenAPI 3.1.0 由静态平台 DTO 生成并以 checked-in JSON 做漂移门
- [Phase 07]: serve --export-openapi 在无配置、store、listener 时直接导出确定性文档
- [Phase 07]: Rust client手写轻量REST调用，生产依赖仅ontoforge-api/HTTP/serde/error，写入不自动重试
- [Phase 07]: ClientError保留HTTP状态、code、apiName、request_id和完整ErrorEnvelope，command_id回放必须显式调用
- [Phase 07]: MCP raw adapters reuse validate_l2 and typed ExplainType with audit and limits
- [Phase 07]: Legacy MCP session owners are bounded TTL entries and every tool request reauthenticates the principal
- [Phase 07-rest-mcp-shell]: Serve MCP defaults通过手写clap适配器解析，保持既有ServeArgs字面量兼容
- [Phase 07-rest-mcp-shell]: MCP qualified table解析使用07-15 typed resolver，缺上下文以MissingContext在发送前拒绝
- [Phase 07]: stdio 通过 rmcp serve_client 与 serve_server 桥接既有 /mcp，共用 host 状态，不创建第二 store/engine/DB
- [Phase 07]: stdio transport 禁止 redirect 与 expired-session 自动 reinit，token 仅来自 env/token-file 并脱敏错误
- [Phase 7]: serve --dev 使用 marker bounded persistent data dir and explicit reset
- [Phase 7]: dev auth key and query signing key are independent persistent files

### Pending Todos

继续07-13真实测试和Linux CI门禁；07-14等待07-13。07-11已完成，见同phase SUMMARY。

### Blockers/Concerns

- [Ops]: 测试凭据待轮换（`ontoforge_trino_ro`、postgres），测试 catalog `ontoforge_pg` 待删除
- [Phase 2]: 非阻塞安全 observations 待后续 phase 跟进（详: `phases/02-source-binding-engine/02-SECURITY.md` Observations）
- [Phase 5]: 生产 webhook 联调与正式 gstack gate 待发布/PR 前完成；GSD 关闭警告详见 `phases/05-online-action/deferred-items.md`
- [Phase 6]: 本地 PR review、CI 与增量复签通过；外部 Codex 审查不可用。Phase 1–5 历史 fingerprint staleness 待 milestone audit 复核，详 Phase 6 VERIFICATION。
- [Phase 7]: 有效完成14/16；本地PG/Trino/L2已可用，07-13完整测试与Linux CI尚待验收；L2-04/SCOPE-07 Pending；PR #7 不合并。

## Deferred Items

| Category | Item | Status | Deferred At | Milestone |
|----------|------|--------|-------------|-----------|
| *(none)* | | | | |

## Session Continuity

Last session: 2026-10-31
Stopped at: Phase 7 07-13 real-service tests and CI verification in progress
Resume file: None
