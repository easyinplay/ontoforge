---
phase: 07-rest-mcp-shell
review_date: 2026-11-03
review_base: 4d203bf
review_target: recorded-working-tree-blobs
status: ds01-resolved
findings: 0
resolved_findings: [DS01]
other_review_findings: pending-spec-owner
tests_rerun: false
real_services_verified: false
phase_acceptance: pending
source_blobs:
  crates/ontoforge-engine/src/lib.rs: f3e144e92dfa347a54a3d8a8ffdf2d478d931999
  crates/ontoforge-engine/src/bind_check.rs: 355e023f16c89a06f38382f8354c79ac18f796fd
  crates/ontoforge-query/src/lib.rs: 94c8aa38ac79db1833ff5e9ce3f2d7ac0c79879a
  crates/ontoforge-query/src/execute.rs: 9bf78c07f5e50cc11c77e8e07fe2c6e13ac32fb4
  crates/ontoforge-query/src/aggregate.rs: 174483df7809faeff921c09626b396a0ac489c96
  crates/ontoforge-query/src/link.rs: bec626bb411c9d0bb467a82d382a9b01ab8e325f
  crates/ontoforge-action/src/lib.rs: 68218b54e954014e998c17081a2bf427502c5de2
  crates/ontoforge-action/src/source.rs: a41954b05581938016668acd2ed9a6d82321c1b3
  crates/ontoforge-server/src/drift.rs: e7eacbb94dadf727e3d0bd1ad9e8a9c60c74b568
  crates/ontoforge-server/src/context.rs: bfc178c59d5e8de31a0d7f36197bcc5b31be82ec
  crates/ontoforge-server/src/state.rs: 50ff35009b09f204fcabb2b51d61f11b71c4310e
  crates/ontoforge-server/src/lib.rs: 38005f3943f73da00026c8fa9ff3c4b08f12c7d8
  crates/ontoforge-server/src/lifecycle.rs: 17b7f4e8f794459661e73b19f8d5833797734dc1
  crates/ontoforge-server/src/health.rs: da21a2c4b3c10fdfb0c2f811c891354ad0f9e18f
  crates/ontoforge-server/src/rest.rs: 4ba27e85a1e512c7adb69aa1d190682e36fde939
  crates/ontoforge-server/src/mcp.rs: aed54c866f87162a95e76c9553da9b63d1d859e2
  crates/ontoforge-server/src/error.rs: dd1572eed41f6e96f76f796e990323debbff6e62
  crates/ontoforge/src/serve.rs: 439ce1ce1eafe6cbe638f92fb2725925c51d7616
---

# RF-01 安全、并发与生命周期独立复签

初审发现的 DS01 已修复并独立复核关闭，本报告自身无剩余 finding。RF-01 和 Phase 7 验收仍 pending，Action 缺 type grant 时提前暴露 drift 的独立发现由 spec reviewer 负责复签，本报告不替它放行。初审 18 个生产文件哈希一致；DS01 复核时仅 server/lib.rs 变更为上列新 blob，其他 17 个文件未变。后续 worker 修改不自动纳入本报告。

## DS01：非法 limits 的重复启动会关闭已有 owner（resolved）

- **原分类**：FIXABLE，P2；已由真实 RED 证实并完成 GREEN。
- **原位置**：旧 blob `8e717a810f573896a365c15ad0c59fa484adaa66` 的 `crates/ontoforge-server/src/lib.rs:166-179`，尤其 limits 校验失败分支中的 `ServerLifecycle::new(self).shutdown().await?`。
- **触发**：先用一个 `Arc<ServerHost>` 正常运行 listener A；再用同一个 host、listener B 调用 `serve_with_limits`，其中 `max_concurrent_requests=0`。第二次调用在检查 `start_drift` 是否已有 owner 前，直接关闭共享宿主生命周期。
- **结果**：`lifecycle.rs:47-76` 停止 A 的 admission、取消 reads、关闭共享 read audit 并取走 drift/L2 owner。第二次调用返回配置错误，但第一个有效实例已经被破坏。基线只是 `limits.validate()?`，没有此共享 cleanup 副作用。
- **已实施修复**：`initialize` 首行恢复纯 `let limits = limits.validate()?;`，在查询 ServerState 或启动 drift 之前拒绝非法配置。主会话取得 Claude Code 同方向意见后明确所有权边界：尚未准入的 fresh invalid 调用也不能关闭调用者传入的 store/audit；因此初审建议中“首次配置失败清理已安装资源”的宽泛表述已收窄。首次 probe 真正启动后的 shutdown/ready 失败清理仍保留。
- **新测试**：`source_drift.rs` blob `a14ff0301467da322affe11e9ac3cb5a067fe7cf` 的 `invalid_duplicate_startup_cannot_close_the_running_owner_or_its_audit` 断言第二次返回 Limits、原实例业务读取 200、audit 仍健康且 shutdown 后实际存在一条读取审计；`invalid_fresh_startup_does_not_probe_or_take_the_store` 断言返回 Limits、零 metadata、audit 仍由调用者持有。
- **验证性质**：亲读 `phase7-drift-ds01-red.log` 的 0 passed/2 failed，失败分别为原实例 SHUTTING_DOWN 和 fresh audit 被关闭；亲读 `phase7-drift-ds01-green.log` 的 17 passed/0 failed/0 ignored，包含两个新测试。reviewer 未运行 Cargo。

## 其他已核查路径

| 边界 | 静态证据与结论 |
| --- | --- |
| 首次 probe 取消与初始化失败 | `lib.rs:170-189` 先注册 monitor 后等待 ready，shutdown 信号或 ready 失败都进入 lifecycle；`drift.rs:158-166` 初始检查可被 stop 取消。已有测试显式阻塞 metadata 后发送 shutdown，断言活跃 metadata 回到 0、audit 关闭。此结论针对正常 shutdown 协议，不宣称任意外部 abort 都有自动清理保证。 |
| task panic、正常异常结束 | `drift.rs:128-136,155` 的 TaskExit 在退出时标记 stopped 并发布 CheckFailed；`:91-94` 对包括未来版本在内的新 capture 返回 CheckFailed，避免永久 healthy/Pending。`:197-203` 保留 JoinError 使 lifecycle 报失败。 |
| exact key 与旧报告隔离 | key 同时保存 def_version/default_profile；`drift.rs:91-117` capture 只取完全相等 key，publish 只接纳已观察 key。round 用 `request_snapshot` 取得真实已提交定义，不读缓存；锁仅覆盖本地状态修改，storage/network await 均在锁外。 |
| captured view 与 Action fence | `context.rs:289-295` 从同一 snapshot 捕获 immutable Arc view/active；REST/MCP Action 的 owned async 内构造 ScopedEngine，继续传固定 principal/scope/active。Action `execute_with_snapshot` 保留 active.def_version，原 `commit_action_guarded` 提交 fence 未被改写。 |
| source 出口与授权 | Query PK `lib.rs:520`、list 首批/后续/backfill `execute.rs:747,900`、aggregate `aggregate.rs:787`、M:N `link.rs:304` 均在相应业务 execute 前检查逻辑 target。完整 patch 不经 source 便沿原路径。SourceTarget 显式传递到 Query/Aggregate/Action，REST 为 503，MCP 沿 ADR-0031 JSON-RPC 安全错误，未输出 operator detail。 |
| 调度与共同预算 | `drift.rs:156-157` 在首次 round 前建立 30s interval/Skip，单 task 顺序执行 round；Notify 合并唤醒，不 spawn 并行 probe。`:208-210` 20s timeout 覆盖 snapshot 和整轮；bind_check 私有 helper 为后续 catalog 下调剩余 timeout，公共 describe_bindings 保留原单调用语义。 |
| 正常关闭 | `lifecycle.rs:45-125` 延续共同 deadline，Action/read 在前，audit 在后，drift/L2 同处截止第 8s 的 monitor 阶段，raw core/L2/monitor cleanup 在总第 10s 前 drain；所有阶段结果合并，取消清理失败不会返回成功。 |
| CLI 启动顺序 | `serve.rs:144` 在构造 L2 monitor 和 source owner 前 bind listener；host signing/install 失败走有界 cleanup helper。初始 runtime probe failure 是可观察状态而非直接启动退出；结构/snapshot 初始化错误仍退出并走清理。 |

## 日志与测试边界

读取了 `07-DRIFT-IMPLEMENTATION.md` 及实际日志：

- `target/phase7-drift-red.log`：编译成功后 4 项真实行为断言失败，不能被误称为编译 RED。
- `target/phase7-drift-exits.log`：12 passed、0 failed；包含首次 probe 取消、snapshot 失败、30s 起点、共同预算、source 出口和完整 patch 路径。
- `target/phase7-drift-owner-green.log`：2 passed、0 failed，覆盖 exact-key/inflight view 与 TaskExit 后未来版本。
- 初审时 `target/phase7-drift-final-focused.log` 尚只有编译输出。DS01 复核时重新读取，已包含 cli_shutdown 9/9、drift_budget 2/2、source_drift 15/15；这是 DS01 修复前的相邻 tranche，不替代之后 source_drift 17/17 的修复验证。
- `target/phase7-drift-ds01-green.log` 的 17/17 已完成；受控 metadata panic 是测试主动触发，后续对应测试明确为 ok，不能将该 panic 输出误计为测试失败。
- `target/phase7-drift-clippy.log` 有 Checking 与 Finished 输出，没有完整命令或 EXIT marker；不将其扩展为所有当前 targets 已通过。
- 新真实 Trino/PG 用例未运行，真实服务、旧 probe/reload barrier 与远端 cancellation drain 的最终验收仍 pending。本 reviewer 没有运行 Cargo、服务或压力测试。

已读 ADR-0030/0031、07-DRIFT-FIX-DESIGN、07-DRIFT-ENG-REVIEW 与原 RF-01 合同。仅写本报告，按 planning-with-files worker 所有权规则由主会话更新共享状态。一次 PowerShell 查询误用 brace 路径语法，未执行成功；已改为逐个显式路径重查，不影响证据。

Recommendation: Close DS01 because validation now returns before touching shared ownership and both regression cases pass in the 17-test tranche; keep RF-01 acceptance pending the separate Action authorization review and real-service verification.
