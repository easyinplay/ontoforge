---
issue: GT-01
base: 972c7be9cc3e91460d300b6f45fd814df023a539
status: verified-awaiting-review
scope: crates/ontoforge-server/tests/mcp_l2_defaults.rs
---

## 问题与修改

原 false 分支发送 `notifications/cancelled` 后立即启动 `running.finish()`，后续 `cancelled` 审计可能来自 shutdown，不能独立证明取消通知生效。主会话与 Claude Code 第二意见确认该测试存在假阳性，第二意见保存在 `target/phase7-claude-cancel-review.json`。

本次仅修改测试及本报告，生产实现、共享 planning 和其他文件未修改。

- source entered 后确认 stub 只收到一个 tool trace，并保存精确 `control.traces[0]`。
- false 分支发送取消通知后，通过独立 SQLite 连接每 20 ms 轮询，最多 3 秒。必须在服务运行时看到全表恰好一条审计，且 `request_id` 与该 trace 精确相等、`status=cancelled`。
- 进入正常 shutdown 前，在 2 秒上限内检查 `/health` 成功，确认服务仍在线。
- 若运行中轮询失败，先保存失败结果，再仅为清理执行 shutdown 并 panic。清理产生的审计不会重新用于判定，因此无法将失败转为通过。
- true 分支专门验证 shutdown；保留最终唯一审计及 ID 对账。原请求未返回 JSON 仍被允许。

## TDD 与验证证据

已读取并遵循 `superpowers:test-driven-development`。主会话已先更新 `task_plan.md`、`progress.md`，本 lane 不重复写共享状态。所有 Cargo 命令均设置 `CARGO_BUILD_JOBS=2`。

1. **受控 mutation RED**：临时省略 false 分支的取消通知调用，保留运行中 SQLite 和 health 检查。定向测试实际编译成功后，在 3.16 秒失败，exit 101。日志 `target/phase7-cancel-causality-red.log` 包含：

   ```text
   no unique cancelled audit for 01M6GEP0Z7MPVDP4Z0JDYQN7F5 while server was running; shutdown cannot supply cancellation evidence
   ```

   该失败来自运行中因果断言，不是编译失败。mutation 已移除，未保留环境变量开关或测试旁路。

2. **GREEN**：恢复取消通知后，整个 `mcp_l2_defaults` 套件 6/6 通过，exit 0。日志 `target/phase7-cancel-causality-green.log` 保留两条 shutdown 前证据：

   ```text
   execute_query: unique cancelled audit 01M6GEQK80QBP1M6FTS64K3Z8Y observed before shutdown; health remains online
   get_table_schema: unique cancelled audit 01M6GEQKHCX2449KK48NQXEP2Y observed before shutdown; health remains online
   ```

3. **静态检查**：`cargo clippy -p ontoforge-server --test mcp_l2_defaults -- -D warnings` exit 0，日志 `target/phase7-cancel-causality-clippy.log`；目标文件 `rustfmt --edition 2024 --check` exit 0，日志 `target/phase7-cancel-causality-fmt.log`；目标 diff whitespace 检查通过。

## 证据边界

未发现需要修改生产实现的取消故障。本测试证明运行中的 MCP 取消通知使相应请求产生持久 `cancelled` 审计，且 ID 与真实源请求 trace 一致；true 分支独立覆盖 shutdown。

stub 在首个 POST 上阻塞，尚未返回 Trino query id，因此本次不宣称验证了 DELETE cleanup。也不宣称被取消请求必然收到 JSON 错误响应。

Warp 与 ralph-loop 不可用，未声称调用。未运行无关 workspace 测试，未 commit 或 push；当前修改冻结供独立复签。
