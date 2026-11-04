---
phase: 07-rest-mcp-shell
reviewed: 2026-10-31
scope: SDK gate fixes only
reviewed_commits: [4b0b18d, 9e6d858, 7fa2401]
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

## 额外发现与关闭

初次复签发现 Rust `OntoforgeClient::history` 构造 `limit=${limit}` 和 `beforeSeq=${before_seq}`，其中 `$` 会进入实际 query 参数。服务端 `rest::history` 的数字解析因此失败并落回默认值/None，原有 contract 未验证非空参数的分页结果。

SDK owner 已通过 TDD 在 `7fa2401` 关闭：两处格式串移除 `$`，新增真实 HTTP contract，连续提交两次 Action 后分别验证 `limit=1` 和 `beforeSeq`。owner 的 RED 实际返回 `([3, 2], [3, 2])`，期望 `([3], [2])`；修复后 GREEN。该 RED 由 owner 执行，本复签 lane 未重复恢复缺陷代码。

本复签 lane 重新检查修复源码与新增断言，并直接运行包含该新增用例的已构建测试程序，避免等待完整 workspace 测试持有的 Cargo build lock：

```text
target/debug/deps/contract-c64ca9177a3c3aa8.exe --list
6 tests, 0 benchmarks，包含新增 history contract

target/debug/deps/contract-c64ca9177a3c3aa8.exe rust_client_history_paginates_with_limit_and_before_seq --exact --nocapture
1 passed, 0 failed, 0 ignored, 5 filtered out，0.15s
```

这是实际测试程序的独立重跑，不是本 lane 新执行的 `cargo test`，也不替代完整 workspace 验证。history 新发现状态为 resolved；CR-01/CR-02 的既有结论不用于推导该结果。本复签 lane 未修改 SDK 实现。

## 边界

- 未复跑完整 workspace、全部 TypeScript contracts 或远程 CI；这些由各执行 lane 和主会话负责。
- 本报告不复签 MCP CR-03；由 SDK owner 独立检查对应 `3f5b5db` 及其真实 session RED/GREEN 证据。
- 本报告不把 SQLite client fixture 当作真实 PG snapshot 证据。
