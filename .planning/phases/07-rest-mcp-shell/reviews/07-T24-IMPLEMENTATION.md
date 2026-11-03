# T24 实施证据

日期：2026-11-03。合同：`07-T24-DESIGN.md`。状态：session 12项、transport 11项通过；server Clippy与本task格式门通过。代码已冻结交独立复签，无commit/push。

已读 TDD 与 planning-with-files。共享计划由主会话维护，本文件为 worker 独立 ledger。根 AGENTS.md 文件不存在，遵守会话提供的 AGENTS.md。Warp / ralph-loop 未暴露，不声称调用。

发现：原 owner map 是进程全局；真实 LocalSessionManager 在 router 内独立创建，owner 驱逐和 TTL 不触及底层 sessions。首轮仅将原 LocalSessionManager 移至既有 ServerState，以便测试访问同一真实 manager，尚未修改容量/回收行为。

RED 目标：真实 HTTP 257 次 initialize，直接检查 manager.sessions 峰值不能大于 256，shutdown 后为零。后续实施统一 wrapper、关闭路径、周期 TTL 与并发 pending 回归。

## 第一轮 RED

`CARGO_BUILD_JOBS=2 cargo test -p ontoforge-server --lib --locked real_http_sessions_never_exceed_capacity -- --nocapture`：测试 0 passed / 1 failed；真实失败 `real LocalSessionManager peak was 257, limit 256`，运行1.84秒，首次编译链接9m27s。日志 `target/phase7-t24-red.log`。原PowerShell末尾Get-Content掩盖shell退出码，已安排同命令重跑并显式exit传播Cargo退出码，不能把shell exit0视为测试通过。

rmcp 3.8.0 本地源码核查：LocalSessionManager.close_session先从真实map删除handle，再发送Close；不等待StreamableHttpService worker自然退出回调。因此统一create/close锁不形成callback join互等。wrapper将保存WorkerTransport cancellation token，在移除时先cancel，避免满channel关闭阻塞。初始化request lease与宿主cleanup queue处理取消；周期任务使用Weak manager与显式cancel/join，避免Arc环。

主会话批准扩大ownership：Cargo.toml仅增加已锁futures-util.workspace=true（重导出Stream），Cargo.lock仅派生依赖边；mcp.rs仅将global verify改为host manager verify。无新包/版本/feature。现有每tool重新认证和权限snapshot保留。

## 第二轮取消窗口 RED

`cargo test -p ontoforge-server --lib --locked cancelled_close_keeps_real_session_counted_until_removal -- --nocapture`，jobs2：exit101，0 passed / 1 failed，编译1m21s，测试0.00s。日志 `target/phase7-t24-close-red.log`。测试持有真实map写锁并手工poll close到Pending，确认token已cancel后取消close future，再释放锁并create。真实map left2/right1，证明先删owner再await底层关闭会遗失容量计数。

主会话与Claude Code接受A++：移除无界cleanup queue；唯一middleware Guard持有Slot<Token>，Drop同步cancel并Notify。reaper扫描cancelled或expired项。close先标记token cancelled、notify，但保留Entry；取得真实map写锁后同步双remove，途中无await，不向可能满的Close channel等待发送。create/close统一mutation锁，其他路径不在持inner锁时等待mutation。

rmcp边界：自定义慢initialize的handler await及worker等待初始化响应不响应token。生产handler使用立即完成的默认initialize；A++可及时清理真实map，测试将先断言清理，再释放受控慢handler并验证晚回调不复活、manager最终释放。drop handle不声称任意第三方handler已join。

检查记录：`cargo check -p ontoforge-server --offline` exit0（原queue版）；`cargo check -p ontoforge-server --tests --locked` exit0（A+中间版）；Clippy第一轮exit101（items_after_statements/default_trait_access），第二轮exit101（A+ unused_self），均已修正但最终Clippy待执行。上述check不是最终动态GREEN。

## A++ 动态 GREEN

`cargo test -p ontoforge-server --lib --locked state::session_tests -- --nocapture`，jobs2：exit0，11 passed / 0 failed / 0 ignored，测试1.85秒，编译链接1m16s。日志 `target/phase7-t24-green.log`。

- 默认容量：257次真实HTTP initialize，manager峰值≤256，shutdown后0。
- 并发pending：16个barrier同步create、小cap2，真实map始终≤2，被驱逐ID不能bind；shutdown拒绝后续create。
- 并发HTTP：24个barrier同步initialize、小cap4，检查真实manager、shutdown后0及Weak失效（任务释放manager）。
- 真实HTTP：驱逐旧ID GET404、DELETE真实清零、双host隔离；无人请求150ms TTL自动回收；连续错误owner不延长TTL；rmcp100ms idle自然退出回调同步清owner。
- 初始化：底层失败/取消、真实HTTP显式失败均无残留；真实HTTP慢initialize遭生产admit deadline取消后，reaper在handler仍阻塞时清零，随后释放handler，晚回调无复活且manager最终释放。
- 关闭取消窗口：同一可归因RED回归现通过，真实map容量1不再增长至2。

## 相邻回归与最终静态门

- `cargo test -p ontoforge-server --test mcp_transport --locked -- --nocapture`，jobs2：exit0，11 passed / 0 failed / 0 ignored，7.32秒；日志 `target/phase7-t24-transport.log`。真实签名token先在30秒clock-skew范围内被接受，7秒后POST/GET/DELETE全部401且engine calls无新增；SSE关闭、定向取消审计、已接受Action保持、stuck Action deadline继续关闭audit均通过。`shutdown_surfaces_owned_action_task_failure`输出的panic是原测试注入，测试自身通过。
- Clippy第三轮（`target/phase7-t24-clippy-final.log`）exit101：新增unit重复加载support模块、attrs默认值限定名。修复为复用既有rest测试support（主会话批准rest.test.rs只改模块可见性），并显式Map::default。
- `cargo clippy -p ontoforge-server --all-targets --locked -- -D warnings`，jobs2：exit0，7.98秒；日志 `target/phase7-t24-clippy-r4.log`。
- `rustfmt --check --edition 2024 --config skip_children=true` 对本task全部9个Rust文件：exit0；日志 `target/phase7-t24-fmt.log`（通过时为空）。包含rest.test.rs单行可见性修改，无其他目录格式化。
- 追加第12项：取消中的bind/verify返回Missing、blocked close被取消后无请求情况下reaper自动重试。这是已发现取消风险的验收补强，不再扩展测试范围。
- 最终 `cargo test -p ontoforge-server --lib --locked state::session_tests -- --nocapture`，jobs2：exit0，12 passed / 0 failed / 0 ignored，1.80秒（编译1m02s）；日志 `target/phase7-t24-green-final.log`。前述11项GREEN保留为实施历史，最终数量以本轮12项为准。

冻结文件共11个（含Cargo.toml/Cargo.lock、9个Rust文件），Git blob与SHA256逐文件记录在 `target/phase7-t24-fingerprints.txt`。生产manager blob `892360952af2e3f1459ec8bd366811f525456c38`，最终unit blob `b24ced57d7589f5ebba7c60888fdf41e51d1ce8b`。原有rest.test.rs只将support模块改为pub(crate)，用于消除重复fixture实例化。未提交、未推送，未修改共享规划。

跨lane协助检查：`cargo check -p ontoforge-server -p ontoforge-client -p ontoforge --tests --locked` 首轮exit101（SDK fixture动态SQL SqlSafeStr，已交原owner修），第二轮exit101（CLI fixture同类3处，已交原owner修），日志分别`target/phase7-t24-integrated-check.log`与`target/phase7-t24-integrated-check-r2.log`。这些不归因本task实现，不修改他人文件。

原owner修复后，第三轮同命令exit0，10.60秒；日志 `target/phase7-t24-integrated-check-r3.log`。仅证明三crate测试编译通过，SDK/CLI动态测试由各自lane执行。Cargo已明确归还主会话，无后台Cargo任务；本worker不再修改冻结代码。
