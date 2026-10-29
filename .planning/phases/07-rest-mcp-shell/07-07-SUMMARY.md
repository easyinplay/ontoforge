---
phase: 07-rest-mcp-shell
plan: "07"
subsystem: mcp
tags: [rmcp, mcp, authorization, tool-catalog, action-schema, tdd]
requires: [07-06]
provides: [mcp-core-full-catalog, mcp-read-tools, mcp-dynamic-action-tools]
affects: [07-08, 07-09, 07-13]
tech-stack:
  added: []
  patterns: [per-request-authentication, shared-tool-catalog, runtime-action-schema]
key-files:
  created:
    - crates/ontoforge-server/src/mcp.rs
    - crates/ontoforge-server/src/mcp_catalog.rs
    - crates/ontoforge-server/src/mcp_actions.rs
    - crates/ontoforge-server/tests/mcp_contract.rs
    - crates/ontoforge-server/tests/mcp_actions.rs
  modified:
    - crates/ontoforge-server/src/lib.rs
    - crates/ontoforge-server/src/context.rs
    - crates/ontoforge-server/src/rest.rs
decisions:
  - "MCP list_tools 与 call_tool 共用每请求编译的 ToolCatalog，toolset 过滤先于分页，隐藏管理工具不注册也不接受猜测调用。"
  - "Action schema 由当前 definition 的真实 ParameterDef 生成，validateOnly 与 commandId 为显式控制字段，调用仍走 ActionService validator/fence/CAS。"
metrics:
  duration: 2h10m
  completed: 2026-10-30
  tasks: 2
  files: 8
  commits: 9
  plan_head_before: 31515e78d005cb385e3442f38bd90403bb77b9af
  plan_head_after: 743d2096c76097c2adbaec877f1e7947aac76b89
  status: complete
actuals:
  tokens: 10601
  tasks: 2
  commits: 9
---

# Phase 7 Plan 7: MCP core/full 与动态 Action 工具 Summary

`/mcp` 现在由同一 `ServerHost` 提供 rmcp HTTP 服务，按请求认证并基于主体 scope 生成 core/full 与动态 Action 工具。读工具复用既有 Query/History 服务，Action 调用复用 `ActionService` 的参数验证和写入保护。

## TDD Evidence

### Task 07-1: core/full 读工具与权限过滤

- RED command: `CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=target/task07 cargo test -p ontoforge-server --test mcp_contract --locked -- --nocapture`
- RED result: exit 1，目标断言分别暴露 core 缺少 `object_set`/`searchAround`，以及后续 Action 参数类型校验缺口；失败为行为断言，不是编译或 fixture 失败。
- RED commit: `5b01f16`
- GREEN implementation: 新增 `OntologyMcpHandler`、`ToolCatalog`、`/mcp` `StreamableHttpService` 路由、逐请求 `context_from_headers` 认证、core/full 工具分页和隐藏工具拒绝；读入口调用既有 `QueryService`、aggregate、history，`describe` 返回主体可见 definition 投影。
- GREEN commit: `9449985`
- Correctness fix: `describe` 初版只注册未执行，补齐 scoped metadata 返回。
- Fix commit: `743d209`
- Final command: `CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=target/task07 cargo test -p ontoforge-server --test mcp_contract --locked -- --nocapture --test-threads=1`
- Final result: 4 passed, 0 failed, 0 ignored；包括真实 rmcp HTTP client `mcp_real_http_client_lists_core_tools_and_rejects_hidden_call`。

### Task 07-2: 动态 Action schema、校验与 validate-only

- RED command: `CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=target/task07 cargo test -p ontoforge-server --test mcp_actions --locked -- --nocapture`
- RED result: exit 1，`mcp_dynamic_action_schema_matches_real_validation` 发现 schema 缺少 `validateOnly` 与 `commandId` 控制字段；此前参数类型非法输入也被固定为失败断言。
- RED commit: `14607ce`
- GREEN implementation: `ActionToolSchema` 从 `ActionType.parameters` 生成真实类型、required、description、default 与控制字段；`call_tool` 统一名称/授权检查后显式执行类型校验，再调用 `ActionService::execute_with_snapshot`，保留 validate-only、fence/CAS/guard 和 command_id 语义。
- GREEN commit: `160d955`
- Final command: `CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=target/task07 cargo test -p ontoforge-server --test mcp_actions --locked -- --nocapture --test-threads=1`
- Final result: 4 passed, 0 failed, 0 ignored；包括真实 HTTP `tools/list` 动态 schema 与 `tools/call` validate-only 零业务写测试。

## Verification

- `cargo test -p ontoforge-server --test mcp_contract --locked -- --nocapture --test-threads=1`: 4/4 passed, 0 ignored。
- `cargo test -p ontoforge-server --test mcp_actions --locked -- --nocapture --test-threads=1`: 4/4 passed, 0 ignored。
- `cargo clippy -p ontoforge-server --tests --no-deps --locked -- -D warnings`: passed。
- `rustfmt --edition 2024` on all plan-owned Rust files: passed；`git diff --check`: passed。
- Full `cargo clippy -p ontoforge-server --tests --locked -- -D warnings` 未通过，失败来自既有 `crates/ontoforge-api/src/openapi.rs` 两条 `clippy::doc_markdown`，不在本 plan 文件范围内。
- Workspace 全量回归未在本 executor 重跑，父会话正在执行同一 branch 的 wave gate；本 plan 的真实 rmcp focused tests 已独立执行。

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] 使用隔离 target 目录完成 Cargo 验证。**

- **Found during:** RED 编译与 focused verification
- **Issue:** 父会话并行 Cargo 任务持有共享 `target`/package cache 锁。
- **Fix:** 所有本 plan 命令统一使用 `CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=target/task07`，未清理或删除共享产物。
- **Files modified:** none

**2. [Rule 1 - Bug] 补齐已注册 `describe` 工具的实际调用路径。**

- **Found during:** final acceptance review
- **Issue:** 工具已出现在 `list_tools`，但 `call_tool` 未分支处理，会错误返回 `tool unavailable`。
- **Fix:** 返回当前 definition 下主体可见 object/property/action metadata，并保留隐藏项过滤。
- **Files modified:** `crates/ontoforge-server/src/mcp.rs`
- **Commit:** `743d209`

## Deferred Issues

- Full workspace clippy 的 `ontoforge-api` 文档 lint 属于 07-05/父会话范围，未在本 plan 修改。
- 真实 PG/Trino、stdio bridge、L2 工具和 session owner map 按 07-08/07-09/07-13 计划承接；本 plan 未宣称这些证据已通过。
- Warp 与 `ralph-loop` 在环境中不可用，未运行也未声称运行。

## Known Stubs

None in files created or modified by this plan.

## Self-Check: PASSED

- Created files exist: `mcp.rs`, `mcp_catalog.rs`, `mcp_actions.rs`, `mcp_contract.rs`, `mcp_actions.rs`。
- Task commits are ancestors of `HEAD`: `5b01f16`, `9449985`, `14607ce`, `160d955`, `4f45bc8`, `743d209`。
- Required named tests executed with no ignored cases: mcp_contract 4/4, mcp_actions 4/4。
- Measured ledger values: `plan_head_before=31515e78d005cb385e3442f38bd90403bb77b9af`, `plan_head_after=743d2096c76097c2adbaec877f1e7947aac76b89`, `commits=9`。
