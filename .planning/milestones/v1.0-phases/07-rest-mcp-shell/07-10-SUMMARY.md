---
phase: 07-rest-mcp-shell
plan: "10"
subsystem: rust-client
tags: [rest, client, reqwest, contract, tdd, api-04]
requires: [07-05, 07-06]
provides: [rust-rest-client, rust-client-contract]
affects: [07-13, 07-14]
tech-stack:
  added: [reqwest-json]
  patterns: [shared-wire-dto-decoding, explicit-command-replay, structured-client-errors]
key-files:
  created:
    - crates/ontoforge-client/tests/contract.rs
    - crates/ontoforge-client/tests/support/mod.rs
  modified:
    - crates/ontoforge-client/Cargo.toml
    - Cargo.lock
    - crates/ontoforge-client/src/lib.rs
decisions:
  - Rust client remains handwritten and depends only on ontoforge-api plus lightweight HTTP/serde/error crates in production.
  - Write requests use caller-supplied command_id and never retry implicitly; replay is an explicit get_command_result call.
  - ClientError retains HTTP status, code, apiName, request_id, and the complete shared ErrorEnvelope.
metrics:
  duration: 1h05m
  completed: 2026-10-29
  tasks: 2
  files: 5
  commits: 4
  plan_head_before: 5b01f16cbb6dede5777b586865c2a6c2ed2731cb
  plan_head_after: 14607ce812057ef365ec18b82e9d63a21758898b
  status: complete
actuals:
  tokens: 10546
  tasks: 2
  commits: 4
---

# Phase 07 Plan 10: Rust REST Client Summary

`ontoforge-client` now exposes typed asynchronous metadata, primary-key, query, aggregate, action, validation, history, command-result, and health calls over real HTTP while preserving structured wire errors and explicit replay semantics.

## TDD Evidence

### Task 10-1: metadata and read client

- RED command: `CARGO_BUILD_JOBS=2 cargo test -p ontoforge-client --test contract --locked -- --nocapture`
- Initial RED was blocked by the missing lock entry; `cargo check -p ontoforge-client` generated the lock for the approved workspace dependencies.
- Behavior RED then reached the real listener and failed because the client API and fixture were not implemented.
- GREEN command: `CARGO_BUILD_JOBS=2 cargo test -p ontoforge-client --test contract --locked -- --nocapture`
- GREEN result: 5 passed, 0 failed, 0 ignored, including `rust_client_metadata_pk_query_aggregate_health` and `rust_client_instances_and_concurrent_cancellation_are_isolated`.
- Commit: `9d82714`

### Task 10-2: action, history, result, and security boundaries

- RED cases covered action validation, explicit replay, changed-parameter conflict, invalid bearer isolation, timeout cancellation, history decoding, and numeric/pagination preservation.
- Initial implementation exposed two wire compatibility issues: action receipts serialize nested `item_ordinal`/`edits`, and history arrays may be omitted. The decoder was corrected to preserve the shared DTO shape without flattening errors or numbers.
- GREEN command: `CARGO_BUILD_JOBS=2 cargo test -p ontoforge-client --test contract --locked -- --nocapture`
- GREEN result: 5 passed, 0 failed, 0 ignored, including `rust_client_action_validate_history_result_and_replay`, `rust_client_auth_scope_revoke_stale_and_transport_errors`, and `rust_client_wire_numbers_and_all_endpoint_groups`.
- Commit: `7f43e05`

## Verification

- `cargo fmt --all -- --check`: passed.
- Final real HTTP contract suite: 5 passed, 0 failed, 0 ignored.
- `cargo check -p ontoforge-client --locked`: passed.
- `cargo clippy -p ontoforge-client --all-targets --locked -- -D warnings` was attempted. It is blocked by pre-existing `ontoforge-api/src/openapi.rs` `clippy::doc_markdown` findings and additional pedantic findings in the new client; no unrelated API files were changed.
- Production dependencies are limited to `ontoforge-api`, `reqwest` with `json`, `serde`, `serde_json`, and `thiserror`; server/storage/auth/engine dependencies are dev-only fixture dependencies.
- Warp and `ralph-loop` were unavailable in this environment and were not claimed as run.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking issue] Updated `Cargo.lock` for the approved client dependencies.**
- **Found during:** Task 10-1 RED
- **Issue:** `--locked` rejected the new `reqwest` JSON feature and client dependency entries before lock generation.
- **Fix:** Ran `cargo check -p ontoforge-client` with `CARGO_BUILD_JOBS=2`, then reran the locked contract command.
- **Files modified:** `Cargo.lock`
- **Commit:** `9d82714`

**2. [Rule 1 - Bug] Decoded the server's nested action receipt wire shape and optional history arrays.**
- **Found during:** Task 10-2 GREEN
- **Issue:** Server receipts contain `items[].edits[]` and history may omit `maskedChanges`/`unset`; strict decoding initially rejected valid responses.
- **Fix:** Mapped the first edit into the shared API receipt projection and defaulted omitted history arrays to empty vectors while retaining all numeric/string values.
- **Files modified:** `crates/ontoforge-client/src/lib.rs`
- **Commit:** `9d82714`

No contract gray areas were encountered. Plan-scoped untracked files were all intentional test artifacts and were committed.

## Deferred Issues

- Workspace/server Clippy remains blocked by pre-existing OpenAPI documentation lint findings and concurrent Phase 7 MCP changes; this plan's package check and contract tests pass.

## Self-Check: PASSED

- `crates/ontoforge-client/src/lib.rs` exists.
- `crates/ontoforge-client/tests/contract.rs` exists.
- `crates/ontoforge-client/tests/support/mod.rs` exists.
- Commits `9d82714` and `7f43e05` are ancestors of `HEAD`.
- Final contract command executed all five named tests with 5 passed, 0 failed, 0 ignored.
