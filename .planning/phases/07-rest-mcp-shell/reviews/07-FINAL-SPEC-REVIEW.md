# 最终 Spec / Testing 审查

2026-11-03。首轮 NO FINDINGS（confidence ≥80）；后续增量状态见下表。仅静态审查，未运行 Cargo、服务或提交。已调用 planning-with-files，复用 Phase 7 计划。依据指定 design/gaps 文档及相关 PLAN。

真实 HTTP/CLI/SDK 路径、barrier、请求计数、完整持久行及原 receipt 均已核对。E5 限 WireJson；E6 测量窗口无后台 L2 monitor。撤权仅影响后续 snapshot；慢 initialize 不承诺任意 handler join。

T24 23/23、Rust SDK 9/9、TS 17/17 为首轮既有 lane 证据。未执行不列为代码缺陷；后续日志仅在实际读到后确认。

## 增量复签

| 范围 | 结论与证据 |
|---|---|
| 8913f92 CLI 实际 Initialize、receipt 提取与 N2 | 未发现问题。真实 host session、DELETE 后 404、业务不重发；在线坏 token 为 host 401、业务零请求；302 origin 收到 bearer 后 trap 零连接，stdout 空且退出非零，stderr 无凭据。已读 `target/phase7-bridge-security-cli-r2.log`（11/11）及 `target/phase7-bridge-security-clippy-r4.log`（完成无报错）。 |
| N1 两主体全链边界 | CLOSED。已按 CC 接受合同复核：Alice 成功链保留；`cli_stdio.rs:107` Mia 经双 transport submit/dryrun/history 明确 NOT_FOUND；MCP 同 command Action replay、HTTP GET result 均不得取得 Alice receipt。每次拒绝后完整持久行不变且 Alice receipt 仍可见。已读 `target/phase7-bridge-security-mia.log`（定向 1/1、8 次拒绝）及 `target/phase7-bridge-security-mia-clippy.log`（通过）。catalog 无 result tool，未新增 API/权限或第二 Action 故事。 |
| N3 query typed EXPLAIN | query 部分通过。四 ExplainType 实际触发 timeout、GET nextUri barrier 后取消及独立并发，精确核对 DELETE、每请求一次 audit/status/request_id。已读 `target/phase7-explain-query-tests.log`（15/15）。此证据不证明 HTTP 并发数量上限。 |
| N3 HTTP typed EXPLAIN | OPEN，confidence 100。`07-15-PLAN.md:131` 要求“四format共享timeout/cancel/concurrency上限”，并发层级由 `07-EXPLAIN-EVIDENCE-PLAN.md` 明确在真实 HTTP 验证。`mcp_l2_defaults.rs:414` 在首请求上游 barrier 持续阻塞、max=1 时第二请求实际 200，预期 429；日志 `target/phase7-explain-server-tests-r2.log` 为 0 passed / 1 failed。`context.rs:180` 在 middleware 获得 SSE response 后 take permit，实际 handler 尚在运行，不能据 query 15/15 关闭 N3。主会话已接管 CC 修复决策；本轮不改生产。 |

| 增量文件 | blob |
|---|---|
| crates/ontoforge/tests/cli_stdio.rs | 4c3bf392652e43032f04d4828c51c6f597e780a2 |
| crates/ontoforge/tests/stdio_security_support/mod.rs | 7b91517336080e3c339f0656c93a2eff95ca827b |
| .planning/phases/07-rest-mcp-shell/reviews/07-E3-BRIDGE-DESIGN.md | a984c13de13fd15316813d9ba320113179f0a198 |
| crates/ontoforge-query/tests/l2_execute.rs | 4813a568f6f54aa9dc4d16c676499217b99a0c21 |
| crates/ontoforge-server/tests/mcp_l2_defaults.rs | 8d951961ce704772a9135937e6d380e26dd4ecd3 |

## 审查内容指纹

HEAD：`552556e7bf9324c996f347b0960a52a096cfc90f`。以下为工作树 Git blob；后续修改需复核。

| 文件 | blob |
|---|---|
| .github/workflows/ci.yml | 840df5c8a41238e4d1cb6a3c4c13c258ae7a9936 |
| Cargo.lock | fc35fdbe18663909ecdf254cbc1bca3bd1a26847 |
| ci/server-services.sh | ef44d35076303a05484dc2a0544ee58946aa4b46 |
| clients/typescript/tests/contract.test.mjs | 85bee8af7ff04b4ce1851256387e4443dc8f84d1 |
| clients/typescript/tests/numeric-security.test.mjs | fb07c5c164fac76e1ce5564e9f7426605ed3bc11 |
| clients/typescript/tests/support-security.mjs | 2f97775d6742f522c7b5d86c3d0a8c8b5136bf7d |
| crates/ontoforge-client/tests/contract.rs | 1ae2c798d3bfc3e3f20c7433a6c2ce7d1204aa16 |
| crates/ontoforge-client/tests/support/mod.rs | ea811b476f0ca88df5da9298db0a14c599ed9713 |
| crates/ontoforge-client/tests/support/security.rs | b7a72d7a854d9828187b5b57c2fa4f9852256bab |
| crates/ontoforge-server/Cargo.toml | 78f302c7868b7f5d18a7625ad517b9197e7b107b |
| crates/ontoforge-server/src/context.rs | f4d09de9c9e42e824eda8fb34a62f1d8be6bbff8 |
| crates/ontoforge-server/src/lib.rs | ca4cb7a8c51f60181ff223000cfe1fd2577af0ba |
| crates/ontoforge-server/src/lifecycle.rs | 00b95093ead7d8eda33028f168658fdf9add9dc1 |
| crates/ontoforge-server/src/mcp.rs | 9f067c1eb7a82cb9b69136c2900a2d9e51ba31b4 |
| crates/ontoforge-server/src/mcp_sessions.rs | 892360952af2e3f1459ec8bd366811f525456c38 |
| crates/ontoforge-server/src/state.rs | 3fa5c9a2d1a5924db6d466ca7ae9473ea1c7b476 |
| crates/ontoforge-server/tests/http_action_safety.rs | 1ae179feaa75ea17dc8b50dbbe4a42b4f8907d47 |
| crates/ontoforge-server/tests/mcp_actions.rs | 0665e2726c3d2967d7c22aecce5d30b57b1684b6 |
| crates/ontoforge-server/tests/mcp_contract.rs | 66e940b44846ced6f8b8d0eb5a5b98490df2def0 |
| crates/ontoforge-server/tests/mcp_l2_defaults.rs | d13ff87e03ee2508f536d9a8249ed9471be26112 |
| crates/ontoforge-server/tests/mcp_transport.rs | 57959eca24403cf94e9329520cfe04b8fa884942 |
| crates/ontoforge-server/tests/postgres_http_it.rs | c48fe9416fc92dbe0956e0b52ab759daf0fdb188 |
| crates/ontoforge-server/tests/unit/mcp_sessions.test.rs | b24ced57d7589f5ebba7c60888fdf41e51d1ce8b |
| crates/ontoforge-server/tests/unit/rest.test.rs | 95bf377f5aa176d4e6b75021d33c98b7b9eb4207 |
| crates/ontoforge-server/tests/support/security_evidence.rs | 6d1d7a5d944cf8e8f2f1d3241a242bd574e45728 |
| crates/ontoforge/tests/cli_stdio.rs | 673f5be98e62af7371745f8dd34b7bfca279bd79 |
| crates/ontoforge/tests/stdio_security_support/mod.rs | ac9c29d01bd494f990afc3311a36f723b960e1c7 |
