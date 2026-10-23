# Phase 5 延后事项

- 2026-10-23：`cargo check -p ontoforge-action --locked -j 2` 在现有 `crates/ontoforge-query/src/execute.rs:30` 失败：此 feature 组合下无法使用 `tokio::join!`。计划中的测试及 clippy 目标可编译；不属于 05-06 Task 2 的修改范围。
- 2026-10-23：`cargo clippy -p ontoforge-query -p ontoforge-action --all-targets --locked -- -D warnings` 在现有 `action_pg.rs` 失败：`execute_with_test_gate` 受 `embedded-pg` 限制；同一命令增加 `--features embedded-pg` 后通过。不属于 05-06 Task 2 的修改范围。
- 2026-10-23：上述 `tokio::join!` normal feature 和 `action_pg.rs` test gate 已在 05-07 Task 2 修复；`cargo check -p ontoforge-action --locked` 与 `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` 通过。
- 2026-10-23：`cargo test --workspace --locked -j 2 -- --nocapture` 退出 101；`ontoforge --test cli_schema` 的 `schema_export_writes_committed_files` 检测 `schemas/ontoforge-definition.schema.json` 与导出结果不一致。05-07 未改 schema 生成或已提交 schema fixture，属于范围外缺口；证据在 `target/gsd-05-07-workspace-test.log`，不能将 workspace suite 标为通过。
