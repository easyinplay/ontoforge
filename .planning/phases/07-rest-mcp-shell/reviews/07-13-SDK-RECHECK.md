---
phase: 07-rest-mcp-shell
reviewed: 2026-10-31
scope: SDK gate fixes only
reviewed_commits: [4b0b18d, 9e6d858]
status: selected_fixes_verified
---

# SDK 修复复签

本报告复签 `07-13-GATE-REVIEW.md` 的 CR-01、CR-02 和 Rust receipt ordinal 修复。原始审查报告保持不变；本报告不代表 Phase 7 总体验收或远程 CI 通过。

## 结论

| 项目 | 结论 | 源码与执行证据 |
| --- | --- | --- |
| CR-01：不支持无损 JSON 的运行时可能先提交 Action 再报错 | 已修复并独立验证 | `clients/typescript/src/index.ts:69` 在构造时检查，`:75-78` 在每次实际 fetch 前重查。分别移除 `JSON.rawJSON`、reviver `context.source` 时，构造和已有 client 的 Action/metadata 调用均拒绝，fetch 计数保持 0。 |
| CR-02：公共 raw client 绕过 bigint 解码 | 已修复并独立验证 | `OntoforgeClient` 和返回对象均不再公开 `raw`；全部九个 endpoint wrapper 使用 `parseAs: 'text'` 后由 `decodeWire` 解码。定向测试检查公开对象无 raw，并从每个 endpoint 返回精确的 `9007199254740993n`。 |
| Rust receipt ordinal 越界截断 | 已修复并独立验证 | `crates/ontoforge-client/src/lib.rs:487` 使用 `u32::try_from`。camelCase/snake_case 输入的 -1、4294967296 均返回 Decode；0 和 u32::MAX 原样保留。 |

`hasLosslessRuntime` 同时检查 native `rawJSON` 和实际 `JSON.parse` reviver 的 `context.source`，不是只检查函数名称。发送前检查覆盖仅含安全 number/string 的 Action，因此不会等到成功响应解码时才发现缺能力。

## 独立执行

```text
npm --prefix clients/typescript run build
通过

node --test --test-name-pattern='ts_client_(rejects_missing_lossless_runtime_before_construction|rechecks_lossless_runtime_before_every_request|public_endpoints_preserve_bigint_without_raw_escape)' clients/typescript/tests/contract.test.mjs
3 tests, 3 passed, 0 failed, 0 skipped

CARGO_BUILD_JOBS=2 cargo test -p ontoforge-client --lib receipt_ --locked -- --nocapture
3 passed, 0 failed, 0 ignored, 1 filtered out
```

TypeScript 测试使用独立临时 `serve --dev` fixture；未重启或清理 PG/Trino lane 的容器。Rust receipt 的三个定向测试在该复签 lane 重新执行，不只引用 SDK owner 的结果。

## 额外发现

Rust `OntoforgeClient::history` 仍构造 `limit=${limit}` 和 `beforeSeq=${before_seq}`，其中 `$` 会进入实际 query 参数。服务端 `rest::history` 的数字解析因此失败并落回默认值/None，现有传 None 的 contract 未覆盖该行为。

该发现已同步主会话及 SDK owner；本复签 lane 保持只读，不修改 SDK。关闭该新问题须有带非空 limit/beforeSeq 的真实 HTTP 回归及修复提交，不能由 CR-01/CR-02 的通过推导其已修复。

## 边界

- 未复跑完整 workspace、全部 TypeScript contracts 或远程 CI；这些由各执行 lane 和主会话负责。
- 本报告不复签 MCP CR-03；由 SDK owner 独立检查对应 `3f5b5db` 及其真实 session RED/GREEN 证据。
- 本报告不把 SQLite client fixture 当作真实 PG snapshot 证据。
