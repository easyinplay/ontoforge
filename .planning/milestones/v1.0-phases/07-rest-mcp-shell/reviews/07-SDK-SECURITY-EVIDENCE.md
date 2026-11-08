# SDK E4 / E5 / E7 动态证据

日期：2026-11-03。范围：07-10 / 07-11 已批准合同。Rust 合同 9/9 与 all-target Clippy 已通过；TS 合同 17/17 已通过，最终集成仍须对当前源码新构建的 binary 复验 TS。

## 计划与边界

- E4：两个有效不同权限主体并发；真实数据库 scope 撤权；已进入执行的请求取消与另一请求成功相互隔离。
- E5：TS u64 max 经 codec 与公开端点无损，unsafe number 拒绝，decimal 字符串不变。
- E7：真实 host 经故障转发代理，执行前断连及提交后丢响应均只发送一次；显式 replay 原 receipt/time，持久事务表与业务状态仅改变一次。
- 主会话拥有共享 task_plan/progress/findings，worker 仅维护本文和测试。已读取 planning-with-files 及 phase 计划；不另建竞争计划。
- 这是正确实现的证据补测，按已批准设计允许直接 GREEN。若发现生产缺陷另报真实行为 RED，不将编译失败算 RED。
- 未提供 Warp / ralph-loop 工具，不声称已调用。主会话本轮授予独占 Cargo 令牌，命令均设置 `CARGO_BUILD_JOBS=2`；测试与 Clippy 完成后已归还。

## 已核实

- 根目录 AGENTS.md 不存在，使用会话注入规则。
- Rust fixture 提供真实 ServerHost、ControlledEngine 阻塞通知及 SQLite store。
- TS fixture 启动真实 serve --dev；Node 内置 http/sqlite 可提供故障代理与持久计数，无需新依赖。

## 验证记录

- 跨crate `cargo check --tests --locked` 首轮因新增fixture四处动态String不满足sqlx0.9 SqlSafeStr而exit101，日志target/phase7-t24-integrated-check.log。这是编译适配失败，不是行为RED。Claude Code AGREE最小fixture修复，见target/phase7-test-sql-claude-review.json。worker已改QueryBuilder、静态pragma_table_info(?)绑定、JSON键push_bind及严格fixture identifier白名单；不放宽生产guard，不改依赖。后续真实合同测试已成功编译并执行。

- `npm --prefix clients/typescript run build`：exit 0。
- `npm --prefix clients/typescript run typecheck`：exit 0，strict/noUncheckedIndexedAccess。
- `node --check clients/typescript/tests/contract.test.mjs`、`node --check clients/typescript/tests/support-security.mjs`：exit 0。
- `node --test clients/typescript/tests/contract.test.mjs`：最终一次 exit 0，17 passed / 0 failed / 0 skipped，1873.9087 ms。日志：`target/phase7-sdk-runtime/ts-contract.log`。
- TS 运行显式设置 `ONTOFORGE_BIN` 为 `target/phase7-sdk-runtime/ontoforge.exe`，从已有 debug binary 复制，先检查存在，未触发 Cargo fallback。SHA256：`859DA06A4DA61CCD9EA9B1C81B45060CDAF484DA04C622ABB33A4BD8B68478D7`。这是先前生产 binary，最终集成须在当前源码重新构建后复验。
- `rustfmt --edition 2024 crates/ontoforge-client/tests/contract.rs`：exit 0；这只证明格式处理，不证明编译或动态行为。
- `cargo test -p ontoforge-client --test contract --locked -- --nocapture` 首轮 exit 0，9 passed / 0 failed / 0 ignored，编译 1m23s、测试 0.42s；日志 `target/phase7-sdk-runtime/rust-contract.log`。
- `cargo clippy -p ontoforge-client --all-targets --locked -- -D warnings` 首轮 exit 101，仅 `support/security.rs` 使用 `_dir` 触发 `used_underscore_binding`；日志 `target/phase7-sdk-runtime/rust-clippy.log`。将 key 路径改为既有数据库路径的父目录，不改生产实现。
- 同一 Clippy 命令复跑 exit 0，1.40s；日志 `target/phase7-sdk-runtime/rust-clippy-r2.log`。
- 同一 Rust contract 命令最终复跑 exit 0，9 passed / 0 failed / 0 ignored，编译 22.37s、测试 0.46s；日志 `target/phase7-sdk-runtime/rust-contract-r2.log`。这是 fixture 路径修正后的最终证据。

## 合同与证据覆盖

| 项 | 新增测试 | 证据 |
|---|---|---|
| E4 TS | `ts_valid_principals_real_revoke_and_inflight_abort_are_isolated` | alice dispatcher 与 mia maintenanceLead 同时成功获取各自不同 metadata；跨权限对象均隐藏且 requestId 不同；真实 HTTP forward 后两个响应同时挂起，取消第一请求、释放第二请求并成功；删除 SQLite dispatcher binding 后原 bearer 下一请求拒绝，mia 不受影响，finally 恢复 binding。 |
| E4 Rust | `rust_client_valid_principals_revoke_and_inflight_cancel_are_isolated` | 已通过。相同双主体隔离；两个请求均在 ControlledEngine barrier 进入执行，取消一个并释放另一个；通过 scope_apply 持久撤权，下一请求拒绝。 |
| E5 TS | `ts_u64_max_codec_and_public_query_preserve_integer_tokens_and_decimal` | `18446744073709551615n` 经 codec 与公开 query 的 WireJson 请求/响应精确保留；wire 保持整数 token；decimal 字符串不变；unsafe number 在发送前拒绝。独立数字协议 HTTP fixture，已 import 进既有 contract 入口，CI 会执行。 |
| E7 TS | `ts_transport_failures_never_retry_and_replay_has_one_durable_commit` | 真实 serve + Node HTTP 故障代理；执行前断连：1 请求/0 forward/全部持久内容不变；提交后丢响应：1 请求/1 forward；log +1、entry +2、history +2、command +1、outbox +1；Flight version +2，其余业务表完整内容不变；显式 replay/result 返回捕获的原完整 receipt，全部持久业务/事务表逐行不变。 |
| E7 Rust | `rust_client_transport_failures_never_retry_and_replay_has_one_durable_commit` | 已通过。同类真实 host + TCP forwarding proxy、完整持久表快照与计数。代理请求有 4 秒上限、upstream 3 秒上限，显式 finish 检查后台 JoinHandle panic。 |

## 范围限制与后续

- TS 的取消 barrier 位于 upstream 已完成、响应仍在客户端在途的位置，证明 SDK AbortSignal/并发隔离，不宣称证明服务端执行取消。Rust 的 ControlledEngine 覆盖服务端已进入执行时客户端取消与另一请求成功，已通过。
- typed receipt/history 的 logSeq/seq/version/versionAfter 现合同为 i64。E5 用公开 query 的 WireJson 边界验证 u64，不扩宽 typed DTO，也不要求通用 codec 拒绝更大 bigint。
- 撤权只验证下一授权 snapshot 拒绝，不要求自动取消既有在途请求。
- 未改生产代码、依赖、CI 或共享 planning；未提交。
- 下一步由主会话在当前生产源码构建后统一复验 TS 集成；本轮不重复执行基于旧 binary 的 TS 合同。
