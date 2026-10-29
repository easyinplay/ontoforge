---
phase: 07-rest-mcp-shell
plan: "05"
subsystem: api
tags: [openapi, utoipa, serde, rust, cli]
requires: [07-04]
provides: [openapi-31-static-document, dto-to-schema-contract, serve-openapi-export]
affects: [07-06, 07-11, 07-13, 07-14]
tech-stack:
  added: [utoipa-6.0.0]
  patterns: [static OpenAPI builder, recursive ToSchema no_recursion, deterministic file export]
key-files:
  created:
    - crates/ontoforge-api/src/openapi.rs
    - crates/ontoforge-api/tests/openapi_snapshot.rs
    - docs/openapi.json
  modified:
    - crates/ontoforge-api/Cargo.toml
    - Cargo.lock
    - crates/ontoforge-api/src/lib.rs
    - crates/ontoforge-api/src/dto.rs
    - crates/ontoforge/src/cli.rs
    - crates/ontoforge/src/serve.rs
    - crates/ontoforge/tests/unit/serve.test.rs
decisions:
  - OpenAPI remains explicitly pinned to 3.1.0 and is rendered from static platform DTOs.
  - Recursive ObjectSet and WhereClause schemas use utoipa no_recursion while retaining JSON references.
  - serve --export-openapi returns before config, store, engine, listener, or shutdown setup.
metrics:
  duration: 33m12s
  completed: 2026-10-29
  tasks: 2
  files: 7
  commits: 5
  plan_head_before: 51b17908eb10d9a4319ca334114955f82b9db7e7
  plan_head_after: 487073ba4c1ebfcdc0cbb6e09d3720fa49c264a0
  status: complete
actuals:
  tokens: 18400
  tasks: 2
  commits: 5
---

# Phase 7 Plan 5: OpenAPI 3.1 and Wire Contract Summary

Static OpenAPI 3.1 generation now covers the authenticated REST surface and can be exported from `serve` without opening a store or listener.

## TDD Evidence

### Task 05-1: DTO and OpenAPI contract

- RED command: `CARGO_BUILD_JOBS=2 cargo test -p ontoforge-api --test openapi_snapshot --locked -- --nocapture`
- RED result: exit 101; 3 tests were discovered, `openapi_31_all_operations_and_security` failed because `docs/openapi.json` did not exist, while the two serde wire tests passed. This was a semantic assertion failure, not a compile or discovery failure.
- RED commit: `e73623a`
- GREEN implementation: added `ToSchema` derives, recursive schema references, response DTOs, static `document()` and `render()`, bearer security, all REST paths, error responses, and checked-in JSON.
- GREEN command: `CARGO_BUILD_JOBS=2 cargo test -p ontoforge-api --test openapi_snapshot --locked -- --nocapture`
- GREEN result: exit 0, 4 passed, 0 failed, 0 ignored.
- GREEN commit: `44cac2d`
- Refactor: workspace formatting and rerun of the focused tests.
- Refactor commit: `487073b`

### Task 05-2: deterministic CLI export

- RED command: `CARGO_BUILD_JOBS=2 cargo test -p ontoforge --lib --locked openapi_ -- --nocapture`
- RED result: exit 101 after a real Cargo build; the named test ran and failed because `serve` did not accept `--export-openapi FILE`.
- RED commit: `f5e547e`
- GREEN implementation: made `server_config` optional when export is requested, added an early static write branch, and covered concurrent exports plus explicit write failures.
- GREEN command: `CARGO_BUILD_JOBS=2 cargo test -p ontoforge --lib --locked openapi_ -- --nocapture`
- GREEN result: exit 0, 1 passed, 0 failed, 0 ignored.
- GREEN commit: `3fafb7f`
- API regression command: `CARGO_BUILD_JOBS=2 cargo test -p ontoforge-api --test openapi_snapshot --locked -- --nocapture`
- API regression result: exit 0, 4 passed, 0 failed, 0 ignored.

## Verification

- `cargo fmt --all -- --check`: exit 0 after formatting.
- `docs/openapi.json` is byte-identical to `ontoforge_api::openapi::render()`.
- OpenAPI version is `3.1.0`; paths include ontology, object, query, aggregate, action, command result, history, and health schema contract.
- Recursive unions, nullable and required fields, deny-unknown-fields behavior, arbitrary JSON, Unicode, and integer wire encoding are covered by serde/schema tests.
- Concurrent export writes are deterministic and do not require configuration, database, engine, or listener initialization.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking issue] Generated `Cargo.lock` for the approved `utoipa` dependency.**
- **Found during:** Task 05-1 GREEN
- **Issue:** `--locked` correctly refused to build before the newly approved workspace dependency was recorded.
- **Fix:** Ran Cargo lock generation, then reran the original locked verification command.
- **Files modified:** `Cargo.lock`
- **Commit:** `44cac2d`

No contract gray areas were encountered. Warp and `ralph-loop` were unavailable in the environment and were not claimed as run.

## Self-Check: PASSED

- `crates/ontoforge-api/src/openapi.rs` exists.
- `crates/ontoforge-api/tests/openapi_snapshot.rs` exists.
- `docs/openapi.json` exists.
- Commits `e73623a`, `44cac2d`, `f5e547e`, `3fafb7f`, and `487073b` are ancestors of `HEAD`.
- Measured plan commit count from the persisted ledger is 5.
