---
phase: 07-rest-mcp-shell
plan: "13"
status: blocked
subsystem: integration
tags: [postgres, trino, ci, real-services]
requires: [07-09, 07-10, 07-11, 07-15, 07-16]
provides: [server-services-ci-framework, real-service-test-entrypoints, local-real-service-evidence]
affects: [API-01, API-03, MCP-01, MCP-06, MCP-07, MCP-08, L2-04, SCOPE-07]
---

# Phase 07 Plan 13 Summary

07-13 已完成本地真实 PG/Trino 行为验证和 CI 生命周期实现，但未关闭：Linux `phase7-real-services` 尚未在远程 runner 执行，独立读取消的专门断言仍未新增。

## Implemented

- `a733759`：真实 PostgreSQL 独立 schema、双 store、reload/revoke barrier、原子写入计数、持久审计与 HTTP Action/history/result 合同。
- `2057c33`：真实 Trino source barrier、snapshot/reload、L2 MCP 六工具、四种 EXPLAIN、ACL deny、61 秒 proof expiry/recovery、core 503/recovery、shutdown Action dependency/audit。
- `6325a8b`、`b2e7cc9`、`aba451b`：PG/Trino/L2 服务生命周期、seed、named-suite discovery、Rust/TypeScript gates、credential scan、always cleanup 与 CRLF 兼容。

## Local Verification

- `cargo test -p ontoforge-server --test postgres_http_it --locked -- --ignored --nocapture --test-threads=1`: **2 passed, 0 failed**。
- `cargo test -p ontoforge-storage --test request_snapshot --locked -- --ignored --nocapture --test-threads=1`: **1 passed, 0 failed**。
- `cargo test -p ontoforge-server --test trino_http_it --locked -- --ignored --nocapture --test-threads=1`: **2 passed, 0 failed**。
- `cargo test -p ontoforge-server --test health_trino_it --locked -- --ignored --nocapture --test-threads=1`: **2 passed, 0 failed**，包含真实 61 秒 TTL。
- 三个 test target `--no-run`、named discovery、`cargo fmt --all -- --check`、`bash -n ci/server-services.sh`、`git diff --check`: 通过。
- 本地 Git Bash/native Docker 启动 PostgreSQL、Trino 483、受限 L2 和 loose ACL 对照实例；L2 端口 `58081/58082` readiness 与 seed 通过。

## Remaining Blockers

1. Linux `phase7-real-services` workflow 尚未实际运行，不能把 CI job 标记为 GREEN。
2. long-read 已证明 source barrier 期间 snapshot/reload 可完成，但尚未新增独立“取消读请求后 durable audit 状态”的专门断言。
3. 全 workspace regression、full Clippy 和远程 artifact credential scan 尚未在本 lane 重跑；既有 `ontoforge-api/openapi.rs:202,208` `clippy::doc_markdown` 仍是范围外阻塞。

## Not Complete

- 不更新 ROADMAP 的 07-13 checkbox。
- 不关闭 `L2-04` / `SCOPE-07`。
- 不进入 07-14 release 计划，直到 Linux CI 真实服务 job 通过。

## Next Step

在 Linux runner 执行 `bash ci/server-services.sh verify`，确认真实服务 job、named suites、TypeScript gate、artifact scan 和 always cleanup 全部通过；随后补独立读取消 audit 断言，再由主会话执行 Phase 7 verify。
