---
phase: 07-rest-mcp-shell
plan: "06"
subsystem: server-lifecycle
tags: [health, l2, monitor, shutdown, tdd]
requires: [07-05]
provides: [d03-health-contract, l2-monitor-host-ownership, bounded-shutdown]
affects: [07-08, 07-13, 07-14]
tech-stack:
  added: []
  patterns: [server-state-registry, d03-health-mapping, bounded-lifecycle-drain]
key-files:
  created:
    - crates/ontoforge-server/src/state.rs
    - crates/ontoforge-server/src/health.rs
    - crates/ontoforge-server/src/lifecycle.rs
    - crates/ontoforge-server/tests/health_lifecycle_it.rs
    - crates/ontoforge/tests/cli_shutdown.rs
  modified:
    - crates/ontoforge-server/src/lib.rs
    - crates/ontoforge/src/serve.rs
decisions:
  - D-03 maps core availability to HTTP 503 and configured L2-only failures to HTTP 200 degraded with stable reason codes.
  - L2 monitor ownership is explicit in server state and shutdown stops admission before waiting for accepted work and draining read audit.
  - Shutdown uses a trusted five-second action drain deadline; timeout or cleanup errors return nonzero.
metrics:
  duration: 2h40m
  completed: 2026-10-29
  tasks: 2
  files: 7
  commits: 9
  plan_head_before: 65dfc2b883fb36c71b4c0c0e3017631b884d75d7
  plan_head_after: 4f77441b048a03a9d84e9fcbd293bcbb6827eee3
  status: complete
actuals:
  tokens: 12500
  tasks: 2
  commits: 9
---

# Phase 7 Plan 6: Health, L2 Monitor, and Bounded Shutdown Summary

`/health` now exposes the accepted D-03 status contract, the host retains L2 monitor ownership, and serve shutdown drains accepted work and read audit with a bounded deadline.

## TDD Evidence

### Task 06-1: D-03 health and monitor state

- RED command: `CARGO_BUILD_JOBS=2 cargo test -p ontoforge-server --test health_lifecycle_it --locked -- --nocapture`
- RED result: exit 101; 4 tests discovered, 3 HTTP assertions failed with the existing 404 fallback because `/health` was not routed, while the shutdown cleanup test passed.
- RED commit: `878b5ce`
- GREEN implementation: added server state registry, D-03 health mapping, `/health` routing, stable L2 reason codes, explicit monitor ownership, and lifecycle drain ordering.
- GREEN command: `CARGO_BUILD_JOBS=2 cargo test -p ontoforge-server --test health_lifecycle_it --locked -- --nocapture`
- GREEN result: exit 0; 4 passed, 0 failed, 0 ignored.
- Refactor/format: `cargo fmt --all -- --check` passed after formatting.
- GREEN commit: `84f9cdb`

### Task 06-2: bounded CLI shutdown

- RED commit: `f9d5b9b` added the three real-listener shutdown tests. The first deterministic sink-failure harness run exposed that dropping an unused audit table leaves no queued record; the fixture was corrected to issue a read after dropping the table, so the failure is attributable to audit cleanup rather than timing.
- GREEN command: `CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=target/task06 cargo test -p ontoforge --test cli_shutdown --locked -- --nocapture --test-threads=1`
- GREEN result: exit 0; 8 tests ran, 8 passed, 0 failed, 0 ignored. The three plan cases all executed: accepted Action completion, durable audit drain, and nonzero sink cleanup failure.
- Production commit: `f94915f`
- Refactor/format commit: `7785acf`

## Verification

- Final health rerun: `CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=target/task06 cargo test -p ontoforge-server --test health_lifecycle_it --locked -- --nocapture`, exit 0, 4/4 passed.
- Final CLI rerun: `CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=target/task06 cargo test -p ontoforge --test cli_shutdown --locked -- --nocapture --test-threads=1`, exit 0, 8/8 passed.
- `cargo fmt --all -- --check`: passed after the final formatting commit.
- `CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=target/task06 cargo check -p ontoforge-server -p ontoforge --locked`: exit 0.
- Real Trino startup, expiry, recovery, and drift evidence remain deferred to 07-13 as required by the plan.
- Warp and `ralph-loop` were unavailable and were not claimed as run.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking issue] Added a separate Cargo target directory for verification.**
- **Found during:** Task 06-2 verification
- **Issue:** The shared target directory was locked by the parent session's workspace regression command.
- **Fix:** Re-ran commands with `CARGO_TARGET_DIR=target/task06`; no target directory was cleared.
- **Files modified:** none

### Shared Branch Ledger Note

The repository uses `worktrees=false`. The persisted plan ledger was created at `65dfc2b`; the parent session concurrently committed 07-15 changes on the same branch while this plan was executing. The required measured ledger count is therefore 9 and includes those concurrent commits. Task-owned commits are `878b5ce`, `84f9cdb`, `f9d5b9b`, `f94915f`, and `7785acf`.

## Deferred Issues

- Real Trino health and drift proof is deferred to 07-13.
- Workspace-wide regression and all-features Clippy were not rerun here because the parent session owned the concurrent workspace gate; package checks and both plan verification commands passed.

## Self-Check: PASSED

- All five created source/test artifacts exist.
- Task commits `878b5ce`, `84f9cdb`, `f9d5b9b`, `f94915f`, and `7785acf` are ancestors of `HEAD`.
- Final health and CLI commands exited 0 with all named tests executed and none ignored.
- No plan-scoped stubs or placeholder implementations were found.
