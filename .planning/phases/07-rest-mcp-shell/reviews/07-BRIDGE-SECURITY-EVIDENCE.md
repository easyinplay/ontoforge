# E3 真实 stdio 证据

状态：既有 11 项验证通过；独立 Spec 复签要求的 Mia 直调用拒绝扩展也已定向验证通过。最终 Cargo 已归还主会话。

## Mia 直调用拒绝补证据

- 依据 `07-E3-BRIDGE-DESIGN.md` 的 A+，仅扩展既有 `live_stdio_two_principals_validate_action_history_and_rest_receipt`，不增加 Mia 权限或第二套 Action 故事。
- Mia 有效 bearer 经真实 stdio 直调 `action_delayFlight` submit、validateOnly、Flight history，以及 Alice 原 command 的 action replay；HTTP 直调 delayFlight submit/validateOnly、Flight history、Alice command result。
- 每次拒绝后比较全部事务与 overlay 行快照，并以 Alice 身份重新查询原 receipt/time，要求仍完全相同。
- 当前 MCP catalog 没有独立 commandResult tool。stdio 采用同 command 的 action replay 检查不可取得 Alice receipt，HTTP 使用实际 `/api/commands/{id}`；已报告主会话该真实入口差异，不将未知工具响应当权限证据。
- 当前 bridge 将上游 ErrorData 包装为 `-32603` + message；测试严格提取 `Mcp error: -32602: NOT_FOUND(...)` 内原始 envelope，断言 code=NOT_FOUND 与目标 apiName；HTTP 断言 404 + NOT_FOUND。
- 未改共享 helper，其他 10 项测试行为未变；此前 11/11 为本扩展前基线，最后仅重跑该成功链及 CLI Clippy。
- 最终定向命令：`cargo test -p ontoforge --test cli_stdio --locked live_stdio_two_principals_validate_action_history_and_rest_receipt -- --exact --nocapture`，exit 0，1 passed / 0 failed / 0 ignored / 10 filtered out，编译 1m09s、测试 2.52s；日志 `target/phase7-bridge-security-mia.log`。
- Mia 的 stdio submit/validateOnly/history/原 command replay 与 HTTP submit/validateOnly/history/command result 共 8 次实际拒绝，精确 NOT_FOUND、逐次全持久行不变、Alice receipt/time 仍能查询的断言全部通过。
- 最终 CLI Clippy：`cargo clippy -p ontoforge --test cli_stdio --locked -- -D warnings` exit 0，7.18s；日志 `target/phase7-bridge-security-mia-clippy.log`。owned `rustfmt --check` 与 `git diff --check` exit 0。
- 本轮使用 `CARGO_BUILD_JOBS=2`。主会话另行处理 MCP SSE permit 问题；本 lane 未修改生产代码，未将该问题与 Mia 授权测试混淆。

## 最终执行证据

- `cargo test -p ontoforge --test cli_stdio --locked -- --nocapture`：exit 0，11 passed / 0 failed / 0 ignored / 0 filtered out，编译 44.71s、测试 2.08s。日志：`target/phase7-bridge-security-cli-r2.log`。
- `cargo clippy -p ontoforge --test cli_stdio --locked -- -D warnings`：exit 0，1.90s。日志：`target/phase7-bridge-security-clippy-r4.log`。
- `rustfmt --edition 2024 --check --config skip_children=true crates/ontoforge/tests/cli_stdio.rs crates/ontoforge/tests/stdio_security_support/mod.rs` 与 owned-path `git diff --check`：exit 0。
- 本 lane 新增 6 项测试，原有 5 项也实际通过。真实执行工具为 PowerShell；未声称 Warp 或 ralph-loop 执行。
- 未修改生产代码、共享 server fixture、依赖、CI 或其他 lane 文件；未创建 commit。全 workspace / fresh service / CI 由主会话统一执行。

| 新增用例 | 实际证据 |
|---|---|
| `live_stdio_closed_upstream_session_never_reinitializes_or_replays` | session 真实创建；透传 DELETE 成功；host 返回 404；initialize=1 / notification=1 / tool POST=1；全部持久行不变 |
| `live_stdio_disconnected_before_forward_never_retries_or_commits` | 完整接收 POST 后断连、不转发；tool POST=1；五表 delta=0，全部 overlay/事务行不变 |
| `live_stdio_lost_commit_response_requires_explicit_replay` | host 提交 receipt 后丢响应；tool POST=1，五表各 +1；Flight version=1、DELAYED、delay=5；新 CLI 显式 replay 返回原 receipt/time，全部持久行不变 |
| `live_stdio_two_principals_validate_action_history_and_rest_receipt` | alice/mia scoped list/describe/query 与 REST 对照；authorized validateOnly 零持久写；Action/history/REST result/replay 全 receipt 一致；audit 包含两个实际身份 |
| `live_stdio_online_bad_token_never_reaches_business_or_leaks_secret` | 真实 host 401；initialize=1 / tool POST=0；stdout 空、退出非零、stderr 无 token；全部持久行不变 |
| `live_stdio_real_redirect_never_connects_to_credential_trap` | 真实 origin 收到有 Authorization 的 initialize 后返回 302；origin hit=1 / trap connections=0；stdout 空、退出非零、stderr 无 token |

## 当前已接受边界（优先于历史记录）

- 真实 bridge 使用 `Initialize` lifecycle/session，proxy 只透传，不改写版本或虚设 legacy-only gateway。分别断言 initialize=1、initialized notification=1、业务 POST=1、server/discover=0。
- 原三项故障测试保留真实 DELETE→host404、完整 POST 接收后断连、真实 receipt 后丢响应、显式新 CLI replay，以及全持久行/版本断言。
- Nyquist 新增双有效主体成功链、在线坏 token、真实 302→loopback trap。设计与 CC 依据见 `07-E3-BRIDGE-DESIGN.md` 更正节。
- 成功链为 alice（dispatcher + crewScheduler）与 mia（maintenanceLead）分别验证 scoped list/describe/query 与 REST 相符；authorized validateOnly 成功且全部业务持久行不变；提交后 history 与 REST 相同，REST command result/显式 Action replay 返回 stdio 原 receipt/time，持久行不变。现有 MCP catalog 无独立 result tool，result 按实际 REST 入口验证。
- 所有已读取 stdout 帧检查 JSON-RPC 和 token 泄露，stderr 并发消费并检查；拒绝场景要求 stdout 空且子进程非零退出。
- `cargo clippy -p ontoforge --test cli_stdio --locked -- -D warnings` r4 exit 0，1.90s；日志 `target/phase7-bridge-security-clippy-r4.log`。r1/r2 为本 lane lint 报错，已整理 helper 修复；未改生产或其他 lane。

## 范围与依据

- 按 `07-E3-BRIDGE-DESIGN.md` 已接受设计补 `07-09-PLAN.md` 的既有合同测试。
- ownership：`crates/ontoforge/tests/cli_stdio.rs`、CLI 专用 support 与本文。生产代码、共享 fixture、共享 planning 与 CI 不改。
- 已显式读取 `planning-with-files`，复用 Phase 7 计划；本文作为 worker 独立 ledger，主会话拥有共享计划。
- 跳过新战略、Phase 与子任务 brainstorm：主会话及 Claude Code REFINE 已接受具体设计。
- TDD：此次补已有生产行为证据，新增测试先运行；正确实现可直接 GREEN。若出现行为失败，记录真实 RED 并报告，不修生产代码。

## 验收计划

1. legacy-only gateway 对 `discover` 返回 correlated method-not-found，限定旧版本 `initialize` 后透传真实 host session；真实 DELETE 关闭该 session，再次业务 POST 由 host 返回 404，不自动 initialize 或重放。
2. modern 默认协议下，完整接收业务 POST 后、不转发即断连：1 POST / 0 commit。
3. modern 默认协议下，收到真实提交 receipt 后丢弃响应：1 POST / 1 commit；新 CLI 连接显式同 command 回放原 receipt / committedAt，5 张持久表无新增。
4. timeout、fault barrier、请求计数、子进程 stdout/stderr 与代理后台失败均可观察；清理有界。

## 发现

- `cli_stdio.rs` 现有 legacy 测试只改变本地 `initialize.protocolVersion`，上游仍走 rmcp modern `discover`。
- server fixture 提供公开 `path` / `store` / `token` / `url`；可复用且无需改共享 fixture。
- 持久表：`of_edit_log`、`of_edit_log_entry`、`of_edit_history`、`of_action_command`、`of_webhook_outbox`。
- 当前未使用 Warp 或 `ralph-loop`，不得声称其执行成功。

## 已写入测试与静态检查

- `live_stdio_closed_upstream_session_never_reinitializes_or_replays`：真实 session 建立、proxy 透传真实 DELETE、业务 POST 收到 host 404；分别断言 discover=1、initialize=1、notification=1、业务 POST=1。
- `live_stdio_disconnected_before_forward_never_retries_or_commits`：完整 POST 接收后 fault barrier、断连、错误或 transport EOF、有界 settle；业务 POST=1，五表 delta=0。
- `live_stdio_lost_commit_response_requires_explicit_replay`：真实 receipt 到达后 barrier 与丢响应；业务 POST=1，五表 delta=1，新 CLI 显式 replay 返回原 receipt / committedAt。
- 追加全部 `of_obj_*` / `of_lnk_*` 与五张事务表完整行快照。执行前断连及 session 关闭保持所有行不变；提交后 Flight overlay 必须为 `F-1001`、version=1、status=DELAYED、delay_minutes=5；显式 replay 后所有行、属性、version 完全不变。
- CLI stdout 每行解析、stderr 并发消费、正常退出和清理 deadline；代理每连接有 timeout，JoinSet 汇集后台 panic，fault barrier 与 shutdown 亦有 deadline。
- 已运行 `rustfmt --edition 2024 --config skip_children=true crates/ontoforge/tests/cli_stdio.rs crates/ontoforge/tests/stdio_security_support/mod.rs`，exit 0。该命令不编译、不证明测试通过。
- 全仓 `git diff --check` 发现其他 lane 的 `crates/ontoforge-server/src/lib.rs:330: new blank line at EOF.`；本 lane 不修改该文件，随后仅检查本 lane diff。
- 待运行：`cargo test -p ontoforge --test cli_stdio --locked -- --nocapture --test-threads=1`。没有 RED/GREEN 运行结果，不能输出任务 COMPLETE。

## sqlx 0.9 编译适配

- 主会话跨 crate check r2 发现本 helper 动态 `query_scalar` / `query` 不满足 `SqlSafeStr`，日志：`target/phase7-t24-integrated-check-r2.log`。这是编译失败，不是行为 RED。
- 根据 `target/phase7-test-sql-claude-review.json` 已接受最小方案，参照 SDK helper 改用 `QueryBuilder`；table 限定 aviation fixture 的 11 张表，identifier 仅允许 ASCII 字母、数字、下划线并双引号引用。
- `pragma_table_info(?)` 的 table 参数与 `json_object` 键均使用 bind。未使用 `AssertSqlSafe` 包裹业务输入，未修改生产、依赖或 SQL guard。
- 本次仅运行 helper 的 `rustfmt` 与本 lane `git diff --check`，均 exit 0。Cargo 仍由 T24 lane 独占，编译与行为验证等待主会话。

## 第一次真实执行

- 2026-11-03：`CARGO_BUILD_JOBS=2 cargo test -p ontoforge --test cli_stdio --locked -- --nocapture`，exit 101，5 passed / 3 failed / 0 ignored；编译 3m01s，测试 10.21s，日志 `target/phase7-bridge-security-cli.log`。
- receipt fixture 错误：真实 action 返回 `structuredContent.receipt`，旧断言误读 `structuredContent.eventId` 导致代理 panic，JoinSet 已使该失败可观察。已机械修复读取路径。
- 协议假设错误：本机 rmcp 3.8.0 `service/client.rs` 的 `serve_client` 明确选 `ClientLifecycleMode::Initialize`，当前生产 bridge 正是该调用。首轮上游 discovery 次数为 0，与设计声称的默认 modern discovery 不符。wire 方法名实际为 `server/discover`，已修正测试 gateway 匹配名。
- `STATUS: NEEDS_CLARIFICATION` 已回主会话：按实际 Initialize 修订证据合同，或先审查生产生命周期切换。未自行修改生产或削弱不重试/完整持久断言；该结果不能归因生产缺陷，也不能标 GREEN。
