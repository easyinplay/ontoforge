---
phase: 07-rest-mcp-shell
plan: "09"
subsystem: api
tags: [mcp, stdio, rmcp, reqwest, json-rpc, tdd]
requires:
  - phase: 07-rest-mcp-shell
    provides: authenticated /mcp host, MCP sessions, Rust CLI serve lifecycle
provides:
  - authenticated `ontoforge mcp` stdio bridge to an existing `/mcp` host
  - bounded rmcp client/server proxy with cancellation and EOF shutdown handling
  - CLI child-process safety tests and MCP API documentation
affects: [07-12, 07-13, 07-14]
actuals:
  tokens: 6520
  tasks: 2
  commits: 5
  plan_head_before: 752a20d595ccab3cc6415b2e13e5e790328d20a5
  plan_head_after: f89e5b52fd48b65207675916451542d80aba8984
tech-stack:
  added: [rmcp client/server transport, reqwest client]
  patterns: [serve-client-to-serve-server proxy, token redaction, no redirect, explicit session expiry failure]
key-files:
  created:
    - crates/ontoforge/src/mcp.rs
    - crates/ontoforge/tests/cli_stdio.rs
  modified:
    - crates/ontoforge/Cargo.toml
    - Cargo.lock
    - crates/ontoforge/src/cli.rs
    - crates/ontoforge/src/lib.rs
    - docs/api.md
key-decisions:
  - "stdio uses rmcp `serve_client` over Streamable HTTP and `serve_server` over AsyncRwTransport, so it reuses the running host's store, auth, scope and audit paths."
  - "HTTP redirects are disabled and `reinit_on_expired_session(false)` is mandatory; callers own explicit command_id replay decisions."
  - "Tokens are read from `ONTOFORGE_TOKEN` or `--token-file`, never from argv, and are redacted from bridge diagnostics."
patterns-established:
  - "Bridge cancellation maps the local RequestContext cancellation token to a remote MCP cancelled notification."
  - "CLI stdio diagnostics stay on stderr while stdout remains reserved for JSON-RPC frames."
requirements-completed: [MCP-01, MCP-06, MCP-07, SCOPE-07]
coverage:
  - id: D1
    description: "MCP CLI bridge configures authenticated same-origin HTTP transport with redirect and transparent session reinitialization disabled."
    requirement: MCP-01
    verification:
      - kind: unit
        ref: "cargo test -p ontoforge --lib --locked bridge_ -- --nocapture"
        status: pass
    human_judgment: false
  - id: D2
    description: "Real ontoforge child processes reject absent servers, malformed credential URLs and bad credentials without leaking secrets or writing non-protocol stdout."
    requirement: MCP-07
    verification:
      - kind: e2e
        ref: "cargo test -p ontoforge --test cli_stdio --locked -- --nocapture --test-threads=1"
        status: pass
    human_judgment: false
  - id: D3
    description: "Positive REST/HTTP/stdio equivalence against a running serve process."
    requirement: SCOPE-07
    verification:
      - kind: manual_procedural
        ref: "07-13/07-14 real service and Trino evidence"
        status: unknown
    human_judgment: true
    rationale: "This plan's Windows child-process harness covered rejection and bounded EOF paths; positive shared-store operation remains assigned to the real-service plans."
duration: 1h10m
completed: 2026-10-30
status: complete
---

# Phase 07 Plan 09: MCP stdio bridge summary

**Authenticated stdio JSON-RPC proxy to an existing `serve /mcp` host with explicit session failure, cancellation forwarding and secret-safe process behavior.**

## Performance

- **Duration:** 1h10m
- **Started:** 2026-10-30T00:41:00+08:00
- **Completed:** 2026-10-30T01:51:00+08:00
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments

- Added `Command::Mcp` and `McpArgs` with `ONTOFORGE_TOKEN` or `--token-file` authentication and strict `/mcp` URL validation.
- Implemented an rmcp client/server proxy that connects stdio to the already-running HTTP host, preserves request IDs, maps cancellation, and closes on EOF without creating a second store or engine.
- Added child-process tests for absent server, bad token, credential-bearing URL rejection, EOF timing, stdout protocol cleanliness and stderr secret redaction; documented REST/MCP/stdio auth and replay boundaries.

## TDD Evidence

### Task 09-1

- RED: `cargo test -p ontoforge --lib --locked bridge_ -- --nocapture` found and failed both named bridge tests with 0 passed, 2 failed.
- GREEN: the same command passed with 2 passed, 0 failed, 0 ignored.
- Implementation commit: `c2653a9`.

### Task 09-2

- RED: `cargo test -p ontoforge --test cli_stdio --locked -- --nocapture --test-threads=1` found and failed all three named process tests with 0 passed, 3 failed.
- GREEN: the same command passed with 3 passed, 0 failed, 0 ignored.
- Implementation commit: `33f6ad5`; formatting commit: `f89e5b5`.

## Task Commits

1. **Task 09-1 RED** - `88a8e59` (`test`)
2. **Task 09-1 GREEN** - `c2653a9` (`feat`)
3. **Task 09-2 RED** - `1c98fcc` (`test`)
4. **Task 09-2 GREEN** - `33f6ad5` (`test`)
5. **Task 09-2 formatting** - `f89e5b5` (`style`)

## Files Created/Modified

- `crates/ontoforge/src/mcp.rs` - token loading, URL validation, rmcp HTTP client transport, stdio server proxy and cancellation mapping.
- `crates/ontoforge/src/cli.rs` - `Command::Mcp` and `McpArgs`.
- `crates/ontoforge/src/lib.rs` - MCP command dispatch.
- `crates/ontoforge/Cargo.toml`, `Cargo.lock` - approved `rmcp` and `reqwest` package dependencies.
- `crates/ontoforge/tests/cli_stdio.rs` - real `ontoforge` child-process safety and bounded shutdown tests.
- `docs/api.md` - REST, MCP HTTP and stdio authentication, error and replay guidance.

## Decisions Made

- The bridge uses rmcp service layers instead of a hand-written dual mutex relay. This preserves rmcp request concurrency and lets the service cancellation token produce a remote cancelled notification without blocking the opposite direction.
- `server-url` rejects userinfo, query and fragment components and only accepts `/mcp`, preventing credentials from being sent through URL parsing or redirects.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added missing lock entries and direct crate dependencies.**
- **Found during:** Task 09-1 GREEN
- **Issue:** `--locked` rejected the newly used `reqwest` and `rmcp` dependencies for `ontoforge`.
- **Fix:** Added the approved workspace dependencies and regenerated `Cargo.lock` with `CARGO_BUILD_JOBS=2 cargo check -p ontoforge`.
- **Files modified:** `crates/ontoforge/Cargo.toml`, `Cargo.lock`
- **Verification:** Locked bridge and full test suites passed.
- **Committed in:** `c2653a9`

**2. [Rule 1 - Bug] Replaced a dual-mutex raw transport relay after finding a bidirectional lock deadlock.**
- **Found during:** Task 09-1 integration test expansion
- **Issue:** Holding one transport mutex during `receive()` blocked the reverse direction from sending responses.
- **Fix:** Proxied the HTTP client through rmcp `serve_client` and exposed it through an rmcp `serve_server` service over stdio; this keeps independent request handling and cancellation capacity.
- **Files modified:** `crates/ontoforge/src/mcp.rs`
- **Verification:** Focused bridge suite and full `ontoforge` tests passed.
- **Committed in:** `c2653a9`

**Total deviations:** 2 auto-fixed (Rule 3 dependency wiring, Rule 1 transport deadlock).
**Impact on plan:** No second runtime, store, engine or database was introduced.

## Issues Encountered

- The positive shared-store REST/HTTP/stdio workflow requires a long-lived configured `serve` process and is assigned to the real-service plans 07-13/07-14. This plan's child-process suite intentionally verifies failure, cancellation/EOF bounds and secret handling without inventing a second fixture lifecycle.
- `cargo test -p ontoforge --tests --locked -- --nocapture --test-threads=1` passed. Existing Trino/private fixture cases remained ignored by their environment gates. `cargo fmt --all -- --check` passed after formatting.
- Warp and `ralph-loop` were unavailable in this executor environment and were not run.

## User Setup Required

None - the bridge reads an existing service token from `ONTOFORGE_TOKEN` or `--token-file`.

## Next Phase Readiness

07-12/07-13/07-14 can run the positive shared-store and release evidence against this bridge. The CLI entry point, auth boundary, cancellation behavior and docs are ready; real Trino interaction remains assigned to 07-13.

## Self-Check: PASSED

- Summary file exists at `.planning/phases/07-rest-mcp-shell/07-09-SUMMARY.md`.
- All five plan commits are ancestors of `HEAD`: `88a8e59`, `c2653a9`, `1c98fcc`, `33f6ad5`, `f89e5b5`.
- Focused bridge tests executed 2 named tests with 2 passed, 0 failed, 0 ignored.
- Focused stdio process tests executed 3 named tests with 3 passed, 0 failed, 0 ignored.
- Measured ledger: `plan_head_before=752a20d595ccab3cc6415b2e13e5e790328d20a5`, `plan_head_after=f89e5b52fd48b65207675916451542d80aba8984`, `commits=5`.

---
*Phase: 07-rest-mcp-shell*
*Plan: 09*
*Completed: 2026-10-30*
