# 07-15 typed EXPLAIN 动态证据 ledger

日期：2026-11-03。范围来自 `07-EXPLAIN-EVIDENCE-PLAN.md` 和 `07-15-PLAN.md`。

## 当前状态

2026-11-04 收口：下述为本 lane 原始 RED 记录，已由后继 T08 修复闭合。query 15/15、最终 server 50/50 及独立安全复签通过；实现与最终日志见 `07-MCP-EXECUTION-ADMISSION-IMPLEMENTATION.md`、`07-T08-SECURITY-RECHECK.md`，总体门禁见 `../07-14-SUMMARY.md`。保留以下失败与回流过程，不将历史失败误作当前未解决缺口。

本 lane 原执行时点：query/server 测试源码已补齐并通过独立 rustfmt。query 全 suite 15 passed；server typed limiter 暴露真实行为 RED，已回主会话 CC 裁决。生产无修改，任务尚未 COMPLETE。
已应用 planning-with-files，以本 ledger 记录 worker 发现与进度；共享 task_plan/progress/findings 由主会话持有。
既有合同补测试，无需制造生产破坏作为 RED。若发现真实失败，先记录并返回主会话裁决。
Warp 与 ralph-loop 未暴露，未声称调用。

## 已核事实

- 原 `l2_explain_all_types_timeout_cancel_concurrency` 只有四格式成功与 audit 总数 4。
- `TrinoConfig` 只有 timeout/rows/bytes/page limits，无并发数量配置。
- `TrinoEngine::execute` 用 L2 health admission RwLock 保证 attestation，非数量 Semaphore。
- 真实并发数量限制在 `ontoforge-server/src/lib.rs` 的 `ServerLimits.max_concurrent_requests` 和 `context.rs` 的 `try_acquire_owned`，耗尽即时返回 429；query-only test 无法直接证明该边界。
- CC 已同意按实际层级补齐，主会话正式扩 ownership 到 `server/tests/mcp_l2_defaults.rs`。使用既有 `Fixture::start_with_limits`，无需修改共享 support；仅扩现有 `L2Control` 的 release Notify 并增加 typed 路由测试。
- `QueryGuard` 在获取 `nextUri` 后掌握 DELETE 目标。测试将在该 URI 的 GET 通知到达后 abort，避免仅取消尚未拿到 query handle 的 POST。

## 测试计划与预期破坏点

| Exact test | 行为与断言 | 当前证据 |
|---|---|---|
| `l2_explain_all_types_timeout_cancel_concurrency` | LOGICAL/DISTRIBUTED/VALIDATE/IO 各触发真实 query timeout，返回 Timeout，只有一条 source_error audit，零结果；timeout cleanup DELETE 指向该请求 | 全 suite 实际通过 |
| 同上 | 每格式请求已到 nextUri GET 后 abort，等待 cancellation drain，只有目标收到 DELETE，只有一条 cancelled audit；同时独立请求正常成功，audit request_id 独立且各一条 | 全 suite 实际通过 |
| `mcp_l2_typed_explain_concurrency_limit_releases_permit` | 四 format 使用可信 ServerLimits=1 的真实 admission 耗尽/释放，第二请求即时 429、上游零增量、拒绝无 audit；首请求释放后第三请求成功，各成功一条 audit | 真实 RED：首个 LOGICAL 请求上游阻塞时第二 HTTP 为 200，预期 429；其余格式尚未跑到 |

可捕获破坏：typed 路径绕过 timeout、丢失 QueryGuard cancellation、取消错误请求、遗漏/重复 audit、并发请求共享 request_id。外部 HTTP 用 wiremock；L2Service、TrinoEngine、SQLite audit 均为生产实现。

query 取消并发顺序由两个 nextUri GET 的 Notify 建立；独立查询只有 release AtomicBool 置位才结束。10ms HTTP polling delay 仅节流，不决定事件顺序。timeout 500ms 与 2s upstream 延迟构造真实截止触发。每 fixture 最后 shutdown audit，持久队列排空后核对准确条数。

## 验证记录

已完成：`cargo check -p ontoforge-query --test l2_execute --locked`，exit 0，37.82s。日志 `target/phase7-explain-query-check.log`。仅证明编译，不当作行为通过。

已完成：`rustfmt --edition 2024 --check crates/ontoforge-query/tests/l2_execute.rs crates/ontoforge-server/tests/mcp_l2_defaults.rs`，exit 0；owned diff `git diff --check`，exit 0。

已完成：query 全 `l2_execute` suite，exit 0，15 passed / 0 failed / 0 ignored / 0 filtered，测试 2.82s，编译 1m59s。日志 `target/phase7-explain-query-tests.log`。

server 首次单测日志 `target/phase7-explain-server-tests.log`；重跑日志 `target/phase7-explain-server-tests-r2.log`。

server 首次运行：exit 101，0 passed / 1 failed。失败在 `all_statements` 的首请求计数，actual 11 / expected 10。根因是测试遗漏 `McpL2Tools::run_sql` 的 `ensure_available` 与 `TrinoEngine::status` 发出的正常 status probe，每个接受的 typed 调用实际是 probe + EXPLAIN 两个 HTTP 请求。只修正测试预期为每成功请求 +2，429 区间仍要求全部身份/标签的上游计数零增量。此失败不是生产行为缺陷，不标为功能 RED；生产无修改。重跑另存 `target/phase7-explain-server-tests-r2.log`。

## 真实行为 RED 与回流

第二次运行：exit 101，0 passed / 1 failed / 9 filtered，0.14s。`mcp_l2_defaults.rs:414` 的 HTTP status 断言为 actual 200 / expected 429。第一个 LOGICAL 调用已由 upstream Notify 确认进入并保持 blocked，第二调用被接纳。

源码定位：`context::admit` 在 `next.run(request)` 返回 response 后执行 `permit_slot.lock().unwrap().take()`，释放 permit；MCP Streamable HTTP 会在工具尚运行时返回 SSE response。`mcp.rs` 的 L2 tool 分支直接 await `call_l2`，没有 Action 分支那样把 `AdmissionPermit` 移入执行生命周期。因此 HTTP response header 完成不等于 typed EXPLAIN 执行完成。

主会话已收到真实 RED 和源码证据。此 lane 保持验收预期，不改生产、不放宽为 200。尚不能声称四格式数量上限通过。失败清理日志 `read audit unavailable while recording cancellation` 出现在断言 panic 后，不能当作另一项独立生产缺陷证据。

实际 query 命令：`cargo test -p ontoforge-query --test l2_execute --locked -- --nocapture`；新 exact test 已包含其中，无需另跑重复单测。

实际 server 命令：`cargo test -p ontoforge-server --test mcp_l2_defaults --locked mcp_l2_typed_explain_concurrency_limit_releases_permit -- --exact --nocapture`；结果见上述真实 RED。

| 命令 | 结果 | 日志 |
|---|---|---|
| `cargo clippy -p ontoforge-query --test l2_execute --locked -- -D warnings` | exit 0，11.70s | `target/phase7-explain-query-clippy.log` |
| `cargo clippy -p ontoforge-server --test mcp_l2_defaults --locked -- -D warnings` | exit 0，9.31s | `target/phase7-explain-server-clippy.log` |
| `cargo test -p ontoforge-server --test mcp_l2_defaults --locked mcp_l2_cancellation_and_shutdown_keep_reserved_audit_ids -- --exact --nocapture` | exit 0，1 passed / 0 failed / 0 ignored / 9 filtered，0.93s；验证 release Notify 改动不破坏原取消/关闭 audit 行为 | `target/phase7-explain-server-cancel-regression.log` |
| 两个 owned Rust 文件 `rustfmt --edition 2024 --check` 与 owned `git diff --check` | exit 0 | 命令返回记录 |

Cargo 已归还主会话，worker 源码冻结，无 commit。未重跑 61s proof expiry 或整个 server suite。下一步为主会话 CC 审核真实 permit 生命周期缺口；修复获批并实施后重跑 server exact test，四格式均通过前不输出 COMPLETE。
