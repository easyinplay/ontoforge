---
phase: 07-rest-mcp-shell
plan: "12"
subsystem: infra
tags: [rust, clap, sqlite, dev-server, persistence, auth, embedded-assets]
requires:
  - phase: 07-rest-mcp-shell
    provides: authenticated REST Action/history host and persistent query signing key contract
provides:
  - persistent `serve --dev` data directory with explicit reset
  - cross-process dev marker and stale-lock recovery
  - embedded aviation ontology, scope and SQLite fixtures in the binary
  - lifecycle tests for restart, idempotent seed, reset and isolation
affects: [07-13, 07-14, docs, release]
actuals:
  tokens: 8336
  tasks: 2
  commits: 2
commits: 2
plan_head_before: 166afeb5d8b1bf581492efe63123ea5250df6ca7
plan_head_after: 8efb9cb97f668926c1bf38a6c2edd6c6a2a1afee
tech-stack:
  added: [getrandom.workspace]
  patterns: [marker-bounded reset, create_new process lock, include_str embedded assets]
key-files:
  created:
    - crates/ontoforge/src/dev.rs
    - crates/ontoforge/src/dev_assets.rs
    - crates/ontoforge/tests/dev_lifecycle.rs
    - docs/dev.md
  modified:
    - crates/ontoforge/src/cli.rs
    - crates/ontoforge/src/lib.rs
    - crates/ontoforge/src/serve.rs
    - crates/ontoforge/Cargo.toml
    - Cargo.lock
key-decisions:
  - "保留现有 ServeArgs 字面量兼容性，新增 dev CLI 状态通过 parser 静态选项读取。"
  - "auth key 与 query signing key 使用不同持久文件；reset 同时重建二者使旧 token 失效。"
patterns-established:
  - "仅空 store 执行 definition/scope seed，已有 store 只复用持久数据。"
  - "reset 先校验 marker、canonical root 和独占锁，再删除明确 dev assets。"
requirements-completed: [DX-01, DX-02]
coverage:
  - id: D1
    description: "serve --dev restart preserves Action receipt and history"
    requirement: DX-01
    verification:
      - kind: integration
        ref: "crates/ontoforge/tests/dev_lifecycle.rs#dev_restart_preserves_action_history_and_receipt"
        status: pass
    human_judgment: false
  - id: D2
    description: "empty seed is idempotent and reset is explicit"
    requirement: DX-02
    verification:
      - kind: integration
        ref: "crates/ontoforge/tests/dev_lifecycle.rs#dev_empty_seed_is_idempotent_and_reset_is_explicit"
        status: pass
    human_judgment: false
  - id: D3
    description: "active instance and unsafe reset paths are rejected"
    requirement: DX-02
    verification:
      - kind: integration
        ref: "crates/ontoforge/tests/dev_lifecycle.rs#dev_active_instance_and_unsafe_reset_are_rejected"
        status: pass
    human_judgment: false
  - id: D4
    description: "copied binary starts with embedded assets outside checkout"
    requirement: DX-01
    verification:
      - kind: integration
        ref: "crates/ontoforge/tests/dev_lifecycle.rs#dev_distributed_binary_query_action_history_without_checkout"
        status: pass
    human_judgment: false
duration: 28m
completed: 2026-10-30
status: complete
---

# Phase 07 Plan 12: 持久 dev 服务、显式 reset 与嵌入演示 Summary

**`serve --dev` 现在以单二进制嵌入 aviation demo，并持久保存 edit store、Action receipt、history、认证 key 和独立 query signing key。**

## Performance

- **Duration:** 28m
- **Started:** 2026-10-29T17:58:00Z
- **Completed:** 2026-10-29T18:26:00Z
- **Tasks:** 2
- **Files modified:** 9

## Accomplishments

- 新增默认 `.ontoforge/dev` 与 `--data-dir`，空 store 才 seed ontology、scope 和 source fixtures。
- 增加 marker、canonical path、symlink rejection、跨进程独占锁和 stale lock recovery；`--reset` 仅在显式 `--dev` 下执行。
- 真实进程测试覆盖 Action → receipt/history → restart、重复 seed、reset、活跃实例拒绝和脱离 checkout 的复制 binary。

## Task Commits

1. **Task 1: 使 dev 初始化幂等、默认持久并安全显式 reset** - `d8cc151`
2. **Task 2: 将完整示例嵌入二进制并验证脱离仓库演示** - `8efb9cb`

## Verification Evidence

- `CARGO_BUILD_JOBS=2 cargo test -p ontoforge --test dev_lifecycle --locked -- --nocapture --test-threads=1`: **5 passed, 0 failed, 0 ignored**。
- `cargo test -p ontoforge --lib serve::tests --locked -- --nocapture --test-threads=1`: **5 passed, 0 failed**。
- `cargo fmt --all -- --check`: passed。
- `cargo clippy -p ontoforge --all-targets --locked -- -D warnings`: 未通过范围外既有问题，`crates/ontoforge-api/src/openapi.rs:202,208` 的 `clippy::doc_markdown`；本计划未修改该文件。

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added direct `getrandom` workspace dependency**

- **Found during:** Task 1
- **Issue:** 独立 query signing key 需要操作系统随机源，`ontoforge` crate 未直接声明已在 workspace 锁定的 `getrandom`。
- **Fix:** 增加 `getrandom.workspace = true` 并更新 `Cargo.lock`，不引入新包版本。
- **Files modified:** `crates/ontoforge/Cargo.toml`, `Cargo.lock`
- **Verification:** lifecycle tests and `cargo check -p ontoforge` pass。
- **Committed in:** `d8cc151`

**Total deviations:** 1 auto-fixed (Rule 2)

## Issues Encountered

- 全包 `clippy -D warnings` 被未触及的 OpenAPI 文档 lint 阻断，已记录为范围外 deferred issue；focused tests、fmt 和 serve unit tests 均通过。

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

07-13/07-14 可以使用 `docs/dev.md` 和生成的 `server.toml` 启动真实 dev demo。PR #7 仍保持 Draft、OPEN。

## Self-Check: PASSED

- `d8cc151` and `8efb9cb` are ancestors of `HEAD`.
- All created files exist, including `docs/dev.md` and `07-12-SUMMARY.md`.
- Plan ledger measured `2` commits from `166afeb` to `8efb9cb`.

---
*Phase: 07-rest-mcp-shell*
*Completed: 2026-10-30*
