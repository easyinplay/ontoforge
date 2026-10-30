---
phase: 07-rest-mcp-shell
plan: "13"
status: blocked
subsystem: integration
tags: [postgres, trino, ci, real-services]
requires: [07-09, 07-10, 07-11, 07-15, 07-16]
provides: [server-services-ci-framework, real-service-test-entrypoints]
affects: [API-01, API-03, MCP-01, MCP-06, MCP-07, MCP-08, L2-04, SCOPE-07]
actuals:
  tokens: 11000
  tasks: 0
  commits: 0
plan_head_before: b6c36d5272003ecee4f0ba80fd82b716b7f2b600
plan_head_after: b6c36d5272003ecee4f0ba80fd82b716b7f2b600
---

# Phase 07 Plan 13 Summary

07-13 未完成：恢复后补充了真实 HTTP metadata/query/object、Action replay/result/history 与 Trino 版本/ACL 请求断言，并补充 CI 的 PG seed 和 L2 setup；PG snapshot/reload/revoke、MCP 六工具、60 秒 expiry/recovery 与 shutdown audit 尚未实现或验收。

## RED Evidence

- `cargo test -p ontoforge-server --test postgres_http_it --locked -- --ignored --nocapture --test-threads=1` 编译成功，2 个指定测试实际执行，因缺少 `ONTOFORGE_PG_URL` 明确失败。
- `cargo test -p ontoforge-server --test postgres_http_it --locked -- --ignored --list` 发现 2 个指定测试。
- `cargo test -p ontoforge-server --test trino_http_it --locked -- --ignored --list` 发现 2 个指定测试。
- `cargo test -p ontoforge-server --test health_trino_it --locked -- --ignored --list` 发现 2 个指定测试。
- `bash -n ci/server-services.sh` 通过；`git diff --check` 通过。
- 恢复执行后：三个测试 target 的 `cargo test --no-run` 通过；`cargo test ... -- --ignored --list` 发现每 target 2 个指定用例；`cargo fmt --all -- --check` 与 `bash -n ci/server-services.sh` 通过。

## Implemented Framework

- 新增 `ci/server-services.sh`：`start`、`verify`、`cleanup` 生命周期、named-suite discovery、Rust/TypeScript gate、服务日志路径。
- `.github/workflows/ci.yml` 新增 `phase7-real-services` Linux job，调用 `ci/server-services.sh verify`。
- 新增三个 ignored test target 和独立 `tests/support/real.rs` 环境/health helper。
- 恢复执行后 `postgres_http_it.rs` 增加 metadata/query/object/no-grant 请求，以及 Action 首次调用、同 command replay、异参 conflict、command result、history 请求断言。
- `trino_http_it.rs` 增加 base/L2 Trino 483 `/v1/info` 与 L2 allow/deny、loose ACL control 请求；当前仍未 drain `nextUri`，不能当作完整 query completion 证据。
- `ci/server-services.sh` 已补充 demo/editstore 数据库创建、aviation/corpus seed、`trino-l2` setup、两个 L2 URL 和 dev token/no-grant token 环境变量。

## Blockers

1. 当前新增的 HTTP 断言仍未覆盖计划全部名义，尤其缺真实 PG 双 store/barrier 的 snapshot/reload/revoke，以及 MCP 六工具、四 EXPLAIN format、60 秒 expiry/recovery、shutdown/audit；因此未提交为完成。
2. 本地 PostgreSQL 可启动并成功 seed aviation fixtures，基础 Trino 483 可启动。
3. 本地 Docker Desktop + WSL 的 L2 setup 在 bind mount 阶段失败：`accessing specified distro mount service ... ubuntu-26-04.sock: no such file or directory`。CI Linux runner 仍需执行真实 L2 suite。
4. 本地 Docker setup 通过 WSL wrapper 运行 `ci/query-services.sh` 时存在 Windows/WSL path conversion 差异，尚未固化到生产 CI 脚本。
5. 恢复时 WSL `docker info` 仍返回 Docker Desktop WSL integration 不可用，`psql` 也未发现；Windows daemon 可用不代表 WSL/Linux setup 可运行。
6. CI configured server 还缺 ontology reload/scope apply 初始化与 Trino readiness；当前脚本不能被认为已形成可运行的完整 GREEN 路径。
7. 纯 WSL 复验命令 `cd /mnt/d/DataSpellProjects/ontoforge && RUNNER_TEMP=/tmp/ontoforge-phase7 bash ci/server-services.sh verify` 立即失败：`The command 'docker' could not be found in this WSL 2 distro.`

## Known Stubs

- `crates/ontoforge-server/tests/postgres_http_it.rs`：已有常规 HTTP reads/Action replay/result/history，缺真实 PG 双 store/barrier 的 snapshot/reload/revoke、CAS/guard/跨定义与跨主体重放证据。
- `crates/ontoforge-server/tests/trino_http_it.rs`：已有 Trino info/ACL 请求，缺 long-read lock release、MCP 六工具和四 format，以及 Trino nextUri completion 断言。
- `crates/ontoforge-server/tests/health_trino_it.rs`：仍只有健康状态和 info 请求，缺 60 秒 expiry/recovery、shutdown/audit 行为。

## Verification Status

- `typecheck/build`: not applicable to this Rust-only plan.
- Real PG/Trino GREEN suites: not run to completion.
- Full workspace regression: not rerun in this blocked lane.
- No task commit was created; parent should not mark 07-13 complete.

## 恢复后的交接

- 本次未创建 task commit，全部改动保留在工作树供主会话审查。
- `Flight/F-1001` 与 `delayFlight` 已在 aviation fixture CSV/actions.yaml 中确认存在；但 configured PG server 必须执行 ontology reload/scope apply 后才能使用它们。
- 不推进 STATE/ROADMAP，不关闭 L2-04/SCOPE-07。
