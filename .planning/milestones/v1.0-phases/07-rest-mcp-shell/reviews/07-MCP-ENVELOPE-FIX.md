---
phase: 07-rest-mcp-shell
date: 2026-11-03
contract: RF-05
decision: .planning/adr/0029-mcp-action-parameter-envelope.md
status: implemented-and-verified
independent_review: pending
phase_acceptance: pending
source_blobs:
  crates/ontoforge-server/src/mcp_actions.rs: 3d7d770a16e58a6bfc56ecd02812cfa2f65a485d
  crates/ontoforge-server/src/mcp.rs: eefc5584a05b84baa6b611bcc0da94d8f78d4fe8
  crates/ontoforge-server/tests/mcp_actions.rs: dbec960fc4d395e2d373aa9fa92e8fa74d709b0c
  crates/ontoforge-server/tests/mcp_contract.rs: 7f4b14c6cde4963e38c99dd100e755a193681f63
  crates/ontoforge-server/tests/mcp_transport.rs: a94725fcd3de4f0a1f9bca47a99c424c8297ab04
  docs/api.md: 72951978a5a1988b4707b1a1578751fec647da5e
---

# RF-05 MCP Action 参数信封修复

实现按 ADR-0029：顶层只允许 `commandId`、`validateOnly`、`params`，只从嵌套 `params` 提取业务值，无平铺双读。业务 schema、required、default、原类型校验移至内层；只有存在必填业务字段时要求外层 `params`。省略可选 `params` 等价于空 object，显式 null/数组/标量拒绝。业务字段名 `commandId`、`validateOnly`、`params` 不再被控制字段覆盖或过滤。

`mcp.rs` 仅改 Action 分支参数提取，控制字段与 ActionService 的 missing command、validateOnly、默认值和 replay 语义保留。已有 MCP Action caller 已同步；额外 caller 仅 `mcp_transport.rs` 的 accepted Action fixture 一行，已获主会话授权。`docs/api.md` 记录新形状和不兼容平铺的边界，REST 合同不变。

## RED

- `target/phase7-mcp-envelope-red.log`：schema/validator 新增 3 项均失败，exit101，分别缺少嵌套默认值/unknown 禁止声明，以及同名 `params` 仍为 string 而非外层 object。
- `target/phase7-mcp-envelope-http-red.log`：真实 MCP listener 新增 3 项均失败，exit101。同名业务值的嵌套调用被拒，optional Action 的显式空 params 被拒，旧平铺 delayFlight 仍实际返回 `validated:true`。

## GREEN 与验收证据

- `target/phase7-mcp-envelope-green.log`：`mcp_actions` 9、`mcp_contract` 16、`mcp_transport` 11，共 36 passed，0 failed，exit0。
- 真实 MCP 调用将三个同名业务值写入新逻辑对象并通过 REST 查询逐一核对；外层 validateOnly 确认无对象写入，缺外层 commandId 仍为 MISSING_COMMAND_ID；正式提交回执使用外层 ID，replay 返回同一回执且最终业务 commit 数不增加。
- 另一个真实 Action 证明省略 params/空 object 均可 validate，业务默认值由 ActionService 应用并实际写入；null/数组/标量、内外层未知键、缺参和错误类型拒绝；合法旧平铺调用拒绝。
- `target/phase7-mcp-envelope-clippy.log`：server lib 与 actions/contract/transport targets 的 Clippy `-D warnings` exit0。
- `target/phase7-mcp-envelope-format.log`：定向 rustfmt check 与 diff check exit0。

首轮 GREEN 尝试为 35/36：测试误要求 JSON Schema 的 required 数组按字母排序；改为字段集合比较，保留定义顺序。原日志留在 `target/phase7-mcp-envelope-first-green.log`。其后的 raw string lint 修正仅去掉多余 delimiter hashes，fixture 字节未变。

## 范围与工作流

- 已按主会话锁定的合同执行 TDD；不再触发战略或子任务 brainstorming，因为 ADR 已解决命名空间和兼容性决策。
- 使用 PowerShell 真正执行上述 Cargo 命令，统一 `CARGO_BUILD_JOBS=2`。Warp/ralph-loop 未暴露，未声称调用或输出其 completion-promise。
- 未运行待实施 RF-01 的 `source_drift.rs`，未运行完整 workspace，未修改共享 planning，未 commit/push。
- 本报告属于实现证据，不替代独立复签，不代表 Phase 7 整体完成。
