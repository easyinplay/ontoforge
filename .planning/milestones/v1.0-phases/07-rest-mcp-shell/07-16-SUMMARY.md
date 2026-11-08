---
phase: 07-rest-mcp-shell
plan: "16"
subsystem: api
tags: [rust, mcp, rmcp, l2, trino, scope, tdd]

requires:
  - phase: 07-rest-mcp-shell
    provides: MCP L2 adapters, typed table resolver, fresh request scope, audit and listener host
provides:
  - Trusted `--mcp-catalog` and `--mcp-schema` deployment defaults wired through `serve`
  - Readonly `McpDefaults` stored per `ServerState`/`EditStore`
  - Qualified table precedence and fail-closed MCP metadata admission
  - Real listener and adapter contract coverage for defaults, overrides, invalid identifiers and zero-send denials
affects: [07-09, 07-13, 07-14, mcp-l2, serve]

actuals:
  tokens: 4822
  tasks: 2
  commits: 5
commits: 5
plan_head_before: a360b29cf5f0d4e6a6df771db6f2f0f023511644
plan_head_after: 785207d50cd877c7003db2c67b3db93ef443319a

tech-stack:
  added: []
  patterns: [custom clap FromArgMatches compatibility adapter, per-store readonly deployment context, typed resolver before engine admission]

key-files:
  created:
    - crates/ontoforge-server/tests/mcp_l2_defaults.rs
    - .planning/phases/07-rest-mcp-shell/07-16-red-task1.log
    - .planning/phases/07-rest-mcp-shell/07-16-red-task2.log
  modified:
    - crates/ontoforge/src/cli.rs
    - crates/ontoforge/src/serve.rs
    - crates/ontoforge-server/src/state.rs
    - crates/ontoforge-server/src/mcp_l2.rs

key-decisions:
  - "Keep ServeArgs' existing struct-literal shape; a manual clap adapter accepts the two new flags and stores parsed trusted defaults in a process-local readonly snapshot."
  - "Key deployment defaults by the existing EditStore-owned ServerState, so MCP handlers consume the same host state without a second service or SQL executor."
  - "Qualified table identifiers are parsed by the 07-15 typed resolver before allow-list checks; missing context is a dedicated structured error and never reaches the engine."

patterns-established:
  - "MCP metadata calls resolve deployment defaults and explicit arguments before constructing PrintedSql or TableRef."
  - "Invalid identifiers, missing context and out-of-scope targets fail before audit execution reaches the engine."

requirements-completed: [MCP-07, SCOPE-07]

coverage:
  - id: D1
    description: "MCP L2 metadata tools consume trusted deployment catalog/schema defaults; explicit and qualified identifiers follow the required precedence."
    requirement: MCP-07
    verification:
      - kind: integration
        ref: "crates/ontoforge-server/tests/mcp_l2_defaults.rs#mcp_l2_optional_context_defaults_and_qualified_override"
        status: pass
    human_judgment: false
  - id: D2
    description: "Missing context, malformed identifiers and allow-list violations are rejected before any upstream engine call."
    requirement: SCOPE-07
    verification:
      - kind: integration
        ref: "crates/ontoforge-server/tests/mcp_l2_defaults.rs#mcp_l2_missing_context_and_outside_allowlist_deny_before_send"
        status: pass
    human_judgment: false
  - id: D3
    description: "The production MCP listener path consumes trusted deployment defaults for a real authenticated request."
    requirement: MCP-07
    verification:
      - kind: e2e
        ref: "crates/ontoforge-server/tests/mcp_l2_defaults.rs#mcp_l2_defaults_real_listener_consumes_deployment_context"
        status: pass
    human_judgment: false

duration: 43m
completed: 2026-10-30
status: complete
---

# Phase 7 Plan 16 Summary

Trusted MCP L2 deployment defaults now flow from `serve` into readonly host state, while qualified identifiers and all denial paths are resolved before upstream execution.

## Performance

- **Duration:** 43 minutes
- **Started:** 2026-10-30T00:25:00+08:00
- **Completed:** 2026-10-30T01:08:00+08:00
- **Tasks:** 2
- **Files modified:** 5 plan files, plus evidence logs

## Accomplishments

- Added `--mcp-catalog` and `--mcp-schema` without breaking existing `ServeArgs` struct literals or adding dependencies.
- Stored deployment-owned `McpDefaults` in `ServerState` keyed by the existing `EditStore` host resource and consumed them from the MCP L2 adapter.
- Enforced qualified table > explicit argument > deployment default precedence through `resolve_l2_table_name`, with schema/catalog allow-list checks after resolution.
- Added explicit missing-context errors and zero-send checks for invalid identifiers and out-of-scope metadata requests.
- Verified the same default path through an authenticated MCP listener request.

## Task Commits

1. **Task 16-1 RED** - `f05d58e` (`test`)
2. **Task 16-1 GREEN** - `2bb82fd` (`feat`)
3. **Task 16-2 RED** - `0feddc8` (`test`)
4. **Task 16-2 GREEN** - `3f12f10` (`feat`)
5. **Listener coverage** - `785207d` (`test`)

## Files Created/Modified

- `crates/ontoforge/src/cli.rs` - manual clap adapter accepting trusted MCP defaults while preserving the public struct shape.
- `crates/ontoforge/src/serve.rs` - deployment defaults injection during host startup.
- `crates/ontoforge-server/src/state.rs` - readonly per-host `McpDefaults` state and store-keyed access.
- `crates/ontoforge-server/src/mcp_l2.rs` - typed context resolution, precedence, allow-list admission and structured denials.
- `crates/ontoforge-server/tests/mcp_l2_defaults.rs` - adapter and real listener contract tests.

## TDD Evidence

### Task 16-1

- RED command and output are preserved in `07-16-red-task1.log`: 2 tests discovered, both failed semantically because qualified identifiers were forwarded as table text and missing context still used the old principal-derived resolution.
- GREEN command: `CARGO_BUILD_JOBS=2 cargo test -p ontoforge-server --test mcp_l2_defaults --locked -- --nocapture`.
- GREEN result: 3 tests passed, 0 failed, 0 ignored after the listener coverage was added.

### Task 16-2

- RED command and output are preserved in `07-16-red-task2.log`: the target assertion failed because missing context was wrapped as the generic invalid-request error.
- GREEN command: `CARGO_BUILD_JOBS=2 cargo test -p ontoforge-server --test mcp_l2_defaults --locked -- --nocapture`.
- GREEN result: missing context now uses the dedicated `MissingContext` error and the full defaults suite passes.

## Verification

- `cargo test -p ontoforge-server --test mcp_l2_defaults --locked -- --nocapture`: pass, 3 passed, 0 failed, 0 ignored.
- `cargo test -p ontoforge-server --tests --locked -- --nocapture`: pass for all server unit/integration suites, including MCP transport and defaults tests.
- `cargo test -p ontoforge --tests --locked -- --nocapture`: pass for all CLI suites; Trino/private fixture tests remained intentionally ignored by their existing environment gates.
- `cargo run -p ontoforge --locked -- serve --help`: pass; help lists `--mcp-catalog` and `--mcp-schema`.
- `cargo fmt --all -- --check`: pass.
- `cargo clippy -p ontoforge-server --test mcp_l2_defaults --locked -- -D warnings`: blocked by pre-existing out-of-scope `crates/ontoforge-api/src/openapi.rs:202,208` `clippy::doc_markdown` findings; no 07-16 file was implicated.
- Warp and `ralph-loop` were unavailable in this environment and were not run or claimed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Preserved `ServeArgs` construction compatibility.**

- **Found during:** Task 16-1 GREEN verification.
- **Issue:** Adding fields directly to `ServeArgs` broke existing integration fixtures that construct the struct by name.
- **Fix:** Implemented a manual `clap::FromArgMatches` adapter with an internal parser for the two new flags; the public struct remains unchanged while `serve` receives the parsed readonly defaults.
- **Files modified:** `crates/ontoforge/src/cli.rs`, `crates/ontoforge/src/serve.rs`.
- **Verification:** `cargo test -p ontoforge --tests --locked -- --nocapture` passed, and `serve --help` lists both flags.
- **Committed in:** `2bb82fd`.

**Total deviations:** 1 auto-fixed Rule 3 compatibility issue.
**Impact on plan:** No dependency, service ownership, SQL executor or authorization scope was added.

## Issues Encountered

- The focused and full Cargo suites serialized behind Windows package/build locks; all required commands completed successfully.
- The requested clippy gate remains blocked by pre-existing OpenAPI doc lint findings outside the plan files. This is deferred to the owning API work.

## Known Stubs

None in files created or modified by this plan.

## User Setup Required

None. No new external service or dependency configuration is required.

## Next Phase Readiness

07-09 can consume the same listener defaults and fail-closed resolver contract. Real Trino six-tool and four-EXPLAIN evidence remains assigned to 07-13/07-14; this plan provides authenticated listener coverage using the existing SQLite fixture.

## Self-Check: PASSED

- Created file exists: `.planning/phases/07-rest-mcp-shell/07-16-SUMMARY.md`.
- Task commits are ancestors of `HEAD`: `f05d58e`, `2bb82fd`, `0feddc8`, `3f12f10`, `785207d`.
- Required defaults suite passed with 3 tests and 0 ignored.
- Measured ledger values recorded: `plan_head_before=a360b29cf5f0d4e6a6df771db6f2f0f023511644`, `plan_head_after=785207d50cd877c7003db2c67b3db93ef443319a`, `commits=5`.

---
*Phase: 07-rest-mcp-shell*
*Plan: 16*
*Completed: 2026-10-30*
