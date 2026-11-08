---
phase: 07-rest-mcp-shell
date: 2026-11-03
finding: WR-01
status: implemented-and-verified
independent_review: pending
phase_acceptance: pending
source_blobs:
  crates/ontoforge-api/src/openapi.rs: 15e8672366c14ee64f310111cabbd527509e469f
  crates/ontoforge-api/tests/openapi_snapshot.rs: a6d8746c96cc536b2ffa560a147e659697e3598d
  docs/openapi.json: b399269cd134f3b95389624474a2610c94e37e1a
  clients/typescript/src/generated.ts: 29c567f86231cb7e0df1fb3bed380927a64e7fa3
---

# WR-01 history OpenAPI 响应补齐

只补充 `GET /api/objects/{object_type}/{pk}/history` 已有运行时错误状态的文档声明：400、408、409、504 均引用 `ErrorEnvelope`。保留 200/401/403/404/503 原声明，重新导出 OpenAPI 并生成 TypeScript；未修改 server 运行时行为或响应 DTO。

新增 `history_error_responses_match_runtime_contract` 同时检查 `openapi::render()` 与 checked-in JSON：responses keys 精确为 200/400/401/403/404/408/409/503/504；200 的 schema ref 为 `HistoryPage`，所有非 200 的 ref 为 `ErrorEnvelope`。这检查已确认的响应合同，不依赖生成文件和源文件彼此相等来判定正确性。

## RED

`target/phase7-history-openapi-red.log`：新增测试 1 failed，exit101。失败直接发生于 rendered response statuses，实际仅有 `[200,401,403,404,503]`，缺少四个状态；不是 checked-in snapshot 漂移导致的失败。

## GREEN

| 日志 | 结果 |
| --- | --- |
| `target/phase7-history-openapi-export.log` | `cargo run -p ontoforge --locked -- serve --export-openapi docs/openapi.json`，exit0 |
| `target/phase7-history-openapi-green.log` | API openapi_snapshot 8 passed、0 failed，exit0；包含精确状态/ref 与源文档一致性 |
| `target/phase7-history-openapi-clippy.log` | API all-targets Clippy `-D warnings`，exit0 |
| `target/phase7-history-openapi-ts.log` | 两次生成 hash 相同，TypeScript typecheck/build 均 exit0 |
| `target/phase7-history-openapi-format.log` | 定向 rustfmt check 与 diff check，exit0 |

最终人工核读 diff：源声明只增加 history 的四个响应；OpenAPI 与 TypeScript diff 也仅增加相应四个 `ErrorEnvelope` 响应。实现/测试/生成文件共 4 个，另有本报告，独立于 RF-05 文件集合。

## 限制

本修复依据已确认的 history 运行时状态合同；本轮执行 schema 合同与生成验证，没有重跑真实服务故障注入，也没有重跑未改动的 SDK runtime。未运行 RF-01 的待实施红例或完整 workspace。

Cargo 统一使用 `CARGO_BUILD_JOBS=2`，通过 PowerShell 实际执行。Warp/ralph-loop 不可用，未声称调用。共享 planning 由主会话维护；本 worker 未 commit/push，独立复签及提交归主会话。此报告不代表 Phase 7 完成。
