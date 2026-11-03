---
phase: 07-rest-mcp-shell
reviewed: 2026-11-03T12:13:10Z
depth: standard
review_target: RF01-working-tree
files_reviewed: 3
files_reviewed_list:
  - crates/ontoforge-server/tests/source_drift.rs
  - crates/ontoforge-server/tests/source_drift_trino_it.rs
  - crates/ontoforge-server/tests/action_source_budget.rs
findings:
  critical: 0
  warning: 0
  info: 0
  total: 0
status: clean
closed_findings:
  - FT01
tests_executed_by_reviewer: false
final_blob_runtime_recheck: passed
source_modified: false
---

# RF01 gstack Testing 专项复签

## Narrative Findings (AI reviewer)

NO FINDINGS

本轮亲读最终 AB01/FT01 测试、生产用途分支和最终合跑日志，没有确认新增测试可靠性问题。原 FT01 测试覆盖 WARNING 已由真实 listener 的第二次 source gate/SQL 回归关闭；AB01/FT01 最终测试 blob 已绑定通过记录。clean 是本专项结论，不代表整阶段已验收。

## FT01 关闭证据

原 WARNING（置信度 96/100）是：旧测试只证明 Shared Arc、metadata probe 版本隔离或一次 source gate 后 reload，缺少一个业务请求跨 report 更新后再次执行 gate 的回归。主会话与 Claude Code 已接受 test-only 补测；已亲读 `target/phase7-claude-fixed-view-test-review.json` 的正常结束与 AGREE，不再需要重新请求同一决定。该覆盖缺口从未被标为已证明的生产漏洞。

亲读 `crates/ontoforge-server/tests/source_drift.rs:1009-1133` 的 `inflight_list_keeps_v1_view_for_second_source_call_after_v2_drift`：

- 第 1015-1036 行启动 healthy monitor，旧分页请求进入第一条 source SQL barrier，明确断言 gate/call 都为 1。
- 第 1038-1066 行令后续 metadata 为空，真实 reload 产生 v2，通过生产 health 等到 degraded 报告，未等待 30 秒周期。
- 第 1067-1075 行在释放旧 SQL 前发新请求；第 1089-1094 行断言 503 / `SOURCE_SCHEMA_DRIFT`。
- 第 1077-1108 行放行旧请求，核对 HTTP 200、256 行首尾 PK、`hasMore=false`，明确要求 target gate 和 source SQL 的总次数都为 2。fixture 第 67 行保留真实空第二页，第二条 SQL 确实经过 inner Engine。
- 第 1115-1132 行按 request_id 查询 durable audit：旧请求 `def_version=v1`、`success`、256 行；新请求 `def_version=v2`、`source_error`。

该用例实现了已接受的分页 v1 跨 v2 合同。Action 跨 reload 成功不属于本项验收，仍应遵守 commit fence。FT01 CLOSED。

## AB01 最终静态复签

完整亲读 `crates/ontoforge-server/tests/action_source_budget.rs`。预算 Engine 第 38-51 行把 barrier 与真实 SQLite 调用共同纳入 `ctx.timeout`；第 257-350 行先等待 source 已进入，再触发 deadline 或 TCP 断连，放行后等待 owned Action，再读取 receipt 和显式 replay。第 199-233 行复开数据库核对 command、edit log、version-1 edit、outbox 恰好一次及 event 关联，且只允许一次 source 调用。

第 371-445 行同时检查 REST/MCP validate-only 保持领域 rows/bytes、Query 仍受独立 limits，最终加强了 command、outbox、action edit log 和 Flight edit 全部为零的断言。断连请求提取为 `open_socket_request` 后，认证/session/header/body 与 barrier 顺序仍完整。

生产 `context.rs:198-220` 保留共同固定 drift gate，Action 用途原样透传领域 QueryContext，Query 用途继续缩紧预算。`rest.rs:453,477` 的正式/validate-only 与 `mcp.rs:622` 的 Action 使用 Action 用途；读路径保持 Query。未发现为恢复预算而绕过 drift gate 的问题。

MCP deadline 用例证明 client HTTP/SSE timeout 后 owned Action 能提交，未将其误称为 server 在 SSE 上返回 504。

## 运行证据与边界

以下日志均由本 reviewer 亲读；本 reviewer 没有运行 Cargo、测试或服务。

| 日志 | 亲读结果 | 范围 |
|---|---|---|
| `target/phase7-action-budget-red.log` | `0 passed; 5 failed` | AB01 修复前行为 RED；正式 Action receipt 404 |
| `target/phase7-action-budget-green.log` | `5 passed; 0 failed; 0 ignored` | AB01 领域预算、四种 timeout/断连路径与 validate-only 对照 |
| `target/phase7-fixed-view-green.log` | `20 passed; 0 failed; 0 ignored` | source_drift 全组，明确包含新增 FT01 测试 |
| `target/phase7-drift-real-green.log` | `2 passed; 0 failed; 0 ignored` | 真实 Trino/PostgreSQL 周期、恢复与 metadata 版本隔离 |
| `target/phase7-drift-cleanup-budget-final.log` | `5 passed`、`20 passed`、`2 passed`；三个 target 均 `0 failed; 0 ignored` | 最终 AB01、source_drift/FT01、DC01 合跑，build `3m 08s` |

早先 AB01/FT01 GREEN 后发生的 lint/测试整理已由最终三 targets 共 27 例合跑覆盖。2026-11-03 20:13（Asia/Shanghai）本 reviewer 亲读完整 `target/phase7-drift-cleanup-budget-final.log`，再次计算工作树 Git blob：AB01 `84634688a5feca5d3b29477b7035c16f697cc18f`、FT01/source_drift `45083f2ee5caeb8bbeaac7bc0f2eddb6acc3521b`，与已复签最终源码及本次运行交接一致，完成最终 blob 绑定。日志明确包含新增 FT01 用例通过。受控 monitor panic 用例输出预期 panic 后自身为 `ok`，整组为 20/20。没有声称执行过 fixed-view mutation。

DC01 源码静态审查仍由独立 owner/reviewer 负责，本次没有扩展源码审查范围。仅记录本次同一日志中两个 cleanup 用例均通过，及交接 blob `0dcf954763cbec905d795714e7cc8aa716dbf30b` 与本次重新计算结果一致，不将其冒称为本 reviewer 的源码审查结论。

## 审查范围与 blob

三份主文件已亲读；最终补读新 FT01 用例、完整 AB01 文件和 EnginePurpose 分支。初审已追踪共享 fixture、context/task registry、REST/MCP Action ownership 与相近旧回归。遵循任务提供的项目指令，已读 gstack `review/specialists/testing.md` 并执行 GSD reviewer skills-only 自加载查询，无额外技能。没有 `.codegraph/` 或项目技能目录。

| 文件 | 最终静态复签 Git blob |
|---|---|
| `crates/ontoforge-server/tests/source_drift.rs` | `45083f2ee5caeb8bbeaac7bc0f2eddb6acc3521b` |
| `crates/ontoforge-server/tests/source_drift_trino_it.rs` | `50ac17c2adcadd54b828933b1327a6f8e88c5ae6` |
| `crates/ontoforge-server/tests/action_source_budget.rs` | `84634688a5feca5d3b29477b7035c16f697cc18f` |
| `crates/ontoforge-server/tests/unit/drift.test.rs` | `ae1e75f1a0690fcd3513d5e71c5674da8a295a53` |
| `crates/ontoforge-server/src/context.rs` | `5f55345d24301b8b43d8ff669d51e32ae556f3ec` |
| `crates/ontoforge-server/src/rest.rs` | `e39b93f6131187488233bd728adc2118b4b6420c` |
| `crates/ontoforge-server/src/mcp.rs` | `c0cd841d97aeb63f6805bfb66de15ff1e49e8465` |

RT01 的 strict object/MCP envelope assertions 初审已亲读，保持关闭。已有授权先行/零 business SQL/零副作用断言未出现新增问题。本 reviewer 只更新本报告，没有修改源码或 commit。
