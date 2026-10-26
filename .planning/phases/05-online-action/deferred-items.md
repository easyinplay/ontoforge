# Phase 5 延后事项

- 2026-10-26：Phase 7 release 前，以真实部署的服务管理器、持久数据库和实际下游接收端联调 `ontoforge webhook-worker`：确认自动重启、503 重试、下游以 `event_id` 去重和至少一次送达。Phase 5 的 WSL 探针仅验证隔离环境机制，不能替代生产配置/接收端的验收。关键模块 PR 前另须执行正式 gstack `/review`；此前 `plan-eng-review` 因宿主 `AskUserQuestion` 不可用，只有独立工程审查替代记录。
- 2026-10-26：GSD `phase.complete 05` 非阻塞警告：`05-VALIDATION.md` 仍为 draft，尚未运行 Nyquist `/gsd-validate-phase`；未生成 `SECURITY.md`，尚未运行 `/gsd-secure-phase`。`05-03-SUMMARY.md` 正文中的 `src/lib.rs` 是相对于 `crates/ontoforge-action/` 的路径，GSD 扫描按仓库根解析而告警；frontmatter 的 `key-files` 已使用完整路径。三个警告均不计为 gate 通过。
- 2026-10-23：`cargo check -p ontoforge-action --locked -j 2` 在现有 `crates/ontoforge-query/src/execute.rs:30` 失败：此 feature 组合下无法使用 `tokio::join!`。计划中的测试及 clippy 目标可编译；不属于 05-06 Task 2 的修改范围。
- 2026-10-23：`cargo clippy -p ontoforge-query -p ontoforge-action --all-targets --locked -- -D warnings` 在现有 `action_pg.rs` 失败：`execute_with_test_gate` 受 `embedded-pg` 限制；同一命令增加 `--features embedded-pg` 后通过。不属于 05-06 Task 2 的修改范围。
- 2026-10-23：上述 `tokio::join!` normal feature 和 `action_pg.rs` test gate 已在 05-07 Task 2 修复；`cargo check -p ontoforge-action --locked` 与 `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` 通过。
- 2026-10-23（已解决）：首次 `cargo test --workspace --locked -j 2 -- --nocapture` 退出 101；05-06 的 `releaseObject` 引起 definition schema fixture 与无效 rule 快照不一致。修复于 `facf3c3`、`6a9f324`；目标测试分别 4/4、1/1，通过后 `cargo test --workspace --locked -j 2` 最终退出码 0。原始失败日志保留于 `target/gsd-05-07-workspace-test.log`。
