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

07-13 未关闭。2026-11-03 Staff修复已复签，本地workspace/Clippy以及PR #8全部4个必需CI job通过；本地完整服务的所有测试通过，但Windows脚本SIGINT未触发graceful shutdown，收尾exit1，正在修复。此前本地真实PG/Trino结果仅作为历史证据。

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
- `02a9c4e`：Windows私有隐藏console owner/stop helper，验证PID与创建时间、精确console成员、Job异常回收、真实退出码和Bash非零归一；9项真实进程回归通过，已接入Windows CI。完整服务复验与独立review待收齐。

## Earlier Local Verification (2026-10-31)

- `cargo test -p ontoforge-server --test postgres_http_it --locked -- --ignored --nocapture --test-threads=1`: **2 passed, 0 failed**。
- `cargo test -p ontoforge-storage --test request_snapshot --locked -- --ignored --nocapture --test-threads=1`: **1 passed, 0 failed**。
- `cargo test -p ontoforge-server --test trino_http_it --locked -- --ignored --nocapture --test-threads=1`: **2 passed, 0 failed**。
- `cargo test -p ontoforge-server --test health_trino_it --locked -- --ignored --nocapture --test-threads=1`: **2 passed, 0 failed**，包含真实 61 秒 TTL。
- 三个 test target `--no-run`、named discovery、`cargo fmt --all -- --check`、`bash -n ci/server-services.sh`、`git diff --check`: 通过。
- 本地 Git Bash/native Docker 启动 PostgreSQL、Trino 483、受限 L2 和 loose ACL 对照实例；L2 端口 `58081/58082` readiness 与 seed 通过。

## Remaining Blockers

2026-11-03 Staff五项修复已冻结并独立复签，0剩余BLOCKER，详`reviews/07-13-STAFF-RECHECK.md`。提交链为`513a2f6`、`100976a`、`cf2a589`、`9aeb3ae`、`0dd3995`、`f835752`。transport11、stdio5、CLI最终L2/cleanup10、engine lib55/monitor19/resilience31通过；最终完整门禁仍待结果。

1. CI run `41577377433`（head `d649260`，PR merge测试commit `55309c0`）全部4个必需job通过。专项artifact位于`target/phase7-linux-ci-41577377433/`，Rust named suites共19项、TS9项通过；credential scan、上传和cleanup全部success。
2. 本地完整脚本Rust19与TS9全部通过且无skip，credential scan独立exit0，但cleanup等待30秒后强杀返回137，使总exit1。根因经独立真实进程复现2/2，见`findings.md`；必须修复实际graceful shutdown信号路径并重跑，不能接受强杀退出为成功。
3. 本地全workspace regression已exit0（1373 passed、0 failed、113 ignored，含doc-tests），full Clippy与embedded-pg Clippy通过；远程artifact credential scan已success。最终交叉复签已落盘。

## Not Complete

- 不更新 ROADMAP 的 07-13 checkbox。
- 不关闭 `L2-04` / `SCOPE-07`。
- 不进入 07-14 release 计划，直到本轮剩余完整门禁通过。

## Next Step

完成本地workspace与完整服务门禁，并收齐Draft PR #8的Windows/Linux CI结果；已通过的Linux专项服务证据见上文。全部通过后解除07-14依赖，Phase 7 verify在全部PLAN验收后执行。
