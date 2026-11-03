---
phase: 07-rest-mcp-shell
plan: "13"
status: executing
subsystem: integration
tags: [postgres, trino, ci, real-services]
requires: [07-09, 07-10, 07-11, 07-15, 07-16]
provides: [server-services-ci-framework, real-service-test-entrypoints, local-real-service-evidence]
affects: [API-01, API-03, MCP-01, MCP-06, MCP-07, MCP-08, L2-04, SCOPE-07]
---

# Phase 07 Plan 13 Summary

07-13 未关闭。2026-11-03 正在完成Staff修复后的最终回归；本轮完整服务脚本、全workspace门禁与Linux `phase7-real-services` 尚待运行。此前本地真实PG/Trino结果仅作为历史证据。

## Implemented

- `a733759`：真实 PostgreSQL 独立 schema、双 store、reload/revoke barrier、原子写入计数、持久审计与 HTTP Action/history/result 合同。
- `2057c33`：真实 Trino source barrier、snapshot/reload、L2 MCP 六工具、四种 EXPLAIN、ACL deny、61 秒 proof expiry/recovery、core 503/recovery、shutdown Action dependency/audit。
- `0aa3e67`：独立 TCP 读断开后的 cancellation、durable audit `cancelled`/`row_count=0` 与无业务写入断言。
- `6325a8b`、`b2e7cc9`、`aba451b`：PG/Trino/L2 服务生命周期、seed、named-suite discovery、Rust/TypeScript gates、credential scan、always cleanup 与 CRLF 兼容。
- `4b0b18d`：SDK发送前无损JSON能力验证，移除公开raw绕过入口；8项contract通过。
- `3f5b5db`、`150a6d2`：MCP参数schema修复与server lint收尾，14项相邻测试通过。
- `9e6d858`、`7fa2401`：Rust receipt序号checked conversion、15项client lint和history分页参数修复，最终4 unit+6 HTTP contract通过。
- `c0abe96`：CLI纯lint最小修改已提交；本地完整Clippy/既有行为回归仍待结果。
- `ec15dd2`、`77382e7`：取消读任务清理及精确audit断言、容器内seed、Windows网络/运行binary隔离接线与Cargo jobs=2。

## Earlier Local Verification (2026-10-31)

- `cargo test -p ontoforge-server --test postgres_http_it --locked -- --ignored --nocapture --test-threads=1`: **2 passed, 0 failed**。
- `cargo test -p ontoforge-storage --test request_snapshot --locked -- --ignored --nocapture --test-threads=1`: **1 passed, 0 failed**。
- `cargo test -p ontoforge-server --test trino_http_it --locked -- --ignored --nocapture --test-threads=1`: **2 passed, 0 failed**。
- `cargo test -p ontoforge-server --test health_trino_it --locked -- --ignored --nocapture --test-threads=1`: **2 passed, 0 failed**，包含真实 61 秒 TTL。
- 三个 test target `--no-run`、named discovery、`cargo fmt --all -- --check`、`bash -n ci/server-services.sh`、`git diff --check`: 通过。
- 本地 Git Bash/native Docker 启动 PostgreSQL、Trino 483、受限 L2 和 loose ACL 对照实例；L2 端口 `58081/58082` readiness 与 seed 通过。

## Remaining Blockers

2026-11-03 Staff五项修复已冻结并独立复签，0剩余BLOCKER，详`reviews/07-13-STAFF-RECHECK.md`。提交链为`513a2f6`、`100976a`、`cf2a589`、`9aeb3ae`、`0dd3995`、`f835752`。transport11、stdio5、CLI最终L2/cleanup10、engine lib55/monitor19/resilience31通过；最终完整门禁仍待结果。

1. Linux `phase7-real-services` workflow 尚未实际运行，不能把 CI job 标记为 GREEN。
2. 修复冻结后重新执行完整服务脚本，要求本轮exit0、named suites、credential scan与cleanup完整证据；旧日志不作为本轮通过依据。
3. 全workspace regression、full Clippy和远程artifact credential scan尚待完成；最终交叉复签待落盘。

## Not Complete

- 不更新 ROADMAP 的 07-13 checkbox。
- 不关闭 `L2-04` / `SCOPE-07`。
- 不进入 07-14 release 计划，直到 Linux CI 真实服务 job 通过。

## Next Step

完成本地完整服务及workspace门禁，创建独立Phase 7 Draft PR并在Linux runner执行`bash ci/server-services.sh verify`；确认named suites、TypeScript gate、artifact scan和always cleanup全部通过后再解除07-14依赖。Phase 7 verify在全部PLAN验收后执行。
