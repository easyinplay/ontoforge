---
phase: 07-rest-mcp-shell
reviewed: 2026-11-03T10:13:43Z
specialist: testing
base: fd24beb36091bc4d9fc56e2984a4103ee923fe3f
head: 972c7be9cc3e91460d300b6f45fd814df023a539
status: issues_found
recheck_status: clean
rechecked: 2026-11-03T10:22:52Z
findings:
  critical: 0
  warning: 1
  info: 0
  total: 1
---

# Phase 07: gstack Testing 专项增量审查

## 范围与限制

- 固定审查范围为 `git diff fd24beb..972c7be`。源码证据均通过 `git show 972c7be:<path>` 或此固定 diff 读取，未将工作树并行修改纳入结论。
- 应用 `C:/Users/easyi/.agents/skills/gstack/review/specialists/testing.md`，检查修复行为的覆盖、负向与边界断言、fixture 隔离、取消和关闭测试的可辨识性。
- 逐项检查该范围内 15 个修改或新增测试文件，以及 `mcp.rs` 内新增的 context failure 单元测试；对照相应 dev、history、MCP、DTO、SDK 实现，并追踪既有 `tests/support/mod.rs`、`mcp_transport.rs` 和服务关闭路径。
- 未运行 Cargo、测试或 mutation test。本报告的结论来自固定提交的静态控制流证据，不宣称复跑通过，也不宣称产品取消功能已被证实失效。
- RF-01、RF-05、07-14 尚未实施，未作为新增 finding。已接受的取消场景可能无 JSON envelope、可控 Trino stub 与真实集群的区别、history 审计语义等限制均保持原边界。

## Narrative Findings (AI reviewer)

### WR-01: WARNING：客户端取消测试被紧随其后的服务关闭补救

```json
{"severity":"INFORMATIONAL","classification":"WARNING","confidence":97,"path":"crates/ontoforge-server/tests/mcp_l2_defaults.rs","line":357,"category":"testing","summary":"shutdown=false 分支发送 notifications/cancelled 后立即启动 running.finish()，随后才检查 durable audit。finish 本身触发 read cancellation，因此即使客户端取消通知未取消 L2 请求，关闭路径也可以生成同一 request_id 的 cancelled audit；当响应无 JSON 时，唯一能区分 CANCELLED 与 SERVER_SHUTTING_DOWN 的断言也不会执行。该测试无法独立守住客户端取消行为。","fix":"仅在 shutdown=false 分支，在调用 finish 前有界等待运行中 SQLite 出现 source trace token 对应的 status=cancelled audit，并发起一个无关请求确认服务仍可用；满足后才执行 finish。shutdown=true 分支继续直接关闭。保留允许原请求无 JSON 的合同，不以 transport body 作为唯一完成条件。","fingerprint":"crates/ontoforge-server/tests/mcp_l2_defaults.rs:357:testing","specialist":"testing","evidence":"固定 head 的 tests/mcp_l2_defaults.rs:349-357 在通知返回成功后无条件 finish；:374-383 的 audit 断言晚于 shutdown，响应断言为条件执行。tests/support/mod.rs:271-273 的 finish_expect 发送 stop，src/lib.rs:227 随后 server_state.cancel_reads()。"}
```

**证据链：** `D:/DataSpellProjects/ontoforge/crates/ontoforge-server/tests/mcp_l2_defaults.rs:349-357` 在两个分支都调用关闭；`tests/support/mod.rs:271-273` 发出 stop；`src/lib.rs:227` 取消活动 read。`src/mcp.rs:570-572` 的客户端取消和服务关闭是不同分支，但这两条路径都会丢弃正在等待的 L2 future，最终断言只要求同一 audit ID 的 `cancelled`。此处问题是测试无法区分触发原因，与已接受的“无 JSON envelope”限制不同。

**最小修复验收：** 对 `execute_query` 与 `get_table_schema` 的客户端取消分支，必须在关闭服务前观察到对应 durable audit 完成，且无关请求仍可成功；关闭分支单独保持覆盖。若将取消通知改为未知 request ID，前置等待应超时失败，不能被之后的关闭补救。

**active query cleanup 的范围：** 当前 `L2Control.block` 在首个 `/v1/statement` POST 返回之前挂起（同文件第 51-53 行），fixture 尚未返回 Trino query ID 或 `nextUri`。因此不能直接据此断言远端 query DELETE cleanup。若另需覆盖该行为，应扩展 stub：先返回带 `nextUri` 的响应，再挂起后续 GET，记录针对该查询的 DELETE，并在关闭前断言。仅修复本 finding 不需要修改生产实现或将该扩展混作已验证证据。

## 已检查测试文件

- `clients/typescript/tests/contract.test.mjs`
- `clients/typescript/tests/error-types.ts`
- `clients/typescript/tests/health-types.ts`
- `crates/ontoforge-api/tests/openapi_snapshot.rs`
- `crates/ontoforge-client/tests/contract.rs`
- `crates/ontoforge-client/tests/error_diagnostics.rs`
- `crates/ontoforge-client/tests/support/mod.rs`
- `crates/ontoforge-server/tests/history_errors.rs`
- `crates/ontoforge-server/tests/history_pagination.rs`
- `crates/ontoforge-server/tests/http_action_diagnostics.rs`
- `crates/ontoforge-server/tests/mcp_actions.rs`
- `crates/ontoforge-server/tests/mcp_contract.rs`
- `crates/ontoforge-server/tests/mcp_l2_defaults.rs`
- `crates/ontoforge/tests/dev_lifecycle.rs`
- `crates/ontoforge/tests/unit/dev.test.rs`

仅新增本报告；未修改实现、其他报告或共享 planning，未提交。

## GT-01 修复独立复签

**结论：** 当前测试修改关闭 WR-01 / GT-01，未发现新的阻塞或警告。上方 `status: issues_found` 保留原固定提交范围的历史审查事实，`recheck_status: clean` 表示本节所界定修复的复签结果。

**复签对象：** HEAD `972c7be9cc3e91460d300b6f45fd814df023a539` 上的当前工作树 diff，仅 `crates/ontoforge-server/tests/mcp_l2_defaults.rs`；读取时文件 Git blob 为 `88f8a08ce74196409119766b4df8af344acd6206`。另读取 `07-CANCEL-CAUSALITY-FIX.md` 及 `target/phase7-cancel-causality-{red,green,clippy,fmt}.log`。本轮未运行 Cargo 或其他测试命令。

**因果隔离已恢复：** 第 384-387 行从 source trace 取得唯一 audit ID；第 390-395 行在服务运行期间发送取消通知，使用独立 SQLite 连接等待唯一且 ID 精确匹配的 `cancelled` 记录，并验证 `/health` 成功。正常关闭只能在这些条件之后于第 406 行发生。若等待失败，第 396-400 行仅清理后无条件 panic，既不重读 shutdown 产生的记录，也不将失败值改为成功。因此 shutdown 无法再补救此断言。

**已读取的动态证据：**

- RED 日志显示省略通知的受控 mutation 已成功编译，测试在 3.16 秒失败，明确出现 `shutdown cannot supply cancellation evidence`；不是编译失败冒充 RED。
- GREEN 日志显示全部 6 项 `mcp_l2_defaults` 测试通过；`execute_query` 与 `get_table_schema` 均分别输出关闭前的唯一 audit ID 和在线 health 证据。
- Clippy 日志显示定向目标完成，无诊断。fmt 日志为空，与成功的 `rustfmt --check` 静默行为相符，但该空日志本身不能独立证明 exit code；exit 0 来自执行 owner 报告，本 reviewer 未复跑。

**保留的证据边界：** 当前 stub 在初始 POST 响应前阻塞，仍未证明 Trino query DELETE cleanup；原请求无 JSON 的情况仍被允许。上述限制不影响本次“取消通知生效必须先于 shutdown”的复签结论。未修改测试或生产源码，未提交。
