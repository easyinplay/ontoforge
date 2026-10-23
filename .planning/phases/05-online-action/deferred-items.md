# Phase 5 延后事项

- 2026-10-23：`cargo check -p ontoforge-action --locked -j 2` 在现有 `crates/ontoforge-query/src/execute.rs:30` 失败：此 feature 组合下无法使用 `tokio::join!`。计划中的测试及 clippy 目标可编译；不属于 05-06 Task 2 的修改范围。
- 2026-10-23：`cargo clippy -p ontoforge-query -p ontoforge-action --all-targets --locked -- -D warnings` 在现有 `action_pg.rs` 失败：`execute_with_test_gate` 受 `embedded-pg` 限制；同一命令增加 `--features embedded-pg` 后通过。不属于 05-06 Task 2 的修改范围。
