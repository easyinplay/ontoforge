---
phase: 07-rest-mcp-shell
plan: "08"
subsystem: mcp
tags: [rmcp, l2, sql:raw, allow-list, audit, session-auth]

requires:
  - phase: 07-rest-mcp-shell
    provides: MCP core/full tool catalog, Action tools, RequestContext authentication, typed L2 EXPLAIN service
provides:
  - Six authenticated `sql:raw` MCP tools with typed EXPLAIN formats and filtered discovery results
  - Bounded legacy session owner bindings with per-request reauthentication, TTL, capacity eviction, and delete cleanup API
  - 2026-07-28 stateless metadata rejection and transport cancellation/reconnect coverage
affects: [07-09, 07-13, 07-16, MCP, L2]

actuals:
  tokens: 8230
  tasks: 2
  commits: 3
plan_head_before: c80082cb90b5928a6c20fba5da8b0b54cd56e60b
plan_head_after: bfbf4f652fa348950687501b09f80d8e307d2ede
commits: 3

tech-stack:
  added: []
  patterns:
    - Generic Engine adapter reusing AST L2 validation, typed ExplainType, execution limits, and ReadAuditGuard
    - Scope-derived catalog/schema resolution with stable sorted/deduplicated discovery output
    - Atomic session owner admission with bounded TTL and oldest-entry eviction

key-files:
  created:
    - crates/ontoforge-server/src/mcp_l2.rs
    - crates/ontoforge-server/src/mcp_sessions.rs
    - crates/ontoforge-server/tests/mcp_l2.rs
    - crates/ontoforge-server/tests/mcp_transport.rs
  modified:
    - crates/ontoforge-server/src/mcp.rs
    - crates/ontoforge-server/src/mcp_catalog.rs

key-decisions:
  - "Expose raw L2 adapters through the existing generic Engine boundary; use validate_l2 and typed ExplainType instead of constructing EXPLAIN SQL in MCP dispatch."
  - "Treat rmcp authentication failures as JSON-RPC errors carried in HTTP 200 responses, and assert the error envelope in transport tests."
  - "Bind legacy session IDs to the freshly authenticated principal on every handler request; retain no permissions in the owner map."

requirements-completed: [MCP-01, MCP-06, MCP-07, MCP-08, L2-04, SCOPE-07]

coverage:
  - id: D1
    description: "Six sql:raw tools are listed with mcp-trino-compatible names, schemas, typed EXPLAIN formats, and scope-filtered discovery."
    requirement: MCP-06
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-server --test mcp_l2 --locked -- --nocapture"
        status: pass
    human_judgment: false
  - id: D2
    description: "Legacy sessions reauthenticate each request, reject owner mismatch, expire/evict bounded bindings, and remove on delete; 2026-07-28 requests require metadata."
    requirement: MCP-01
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-server --test mcp_transport --locked -- --nocapture"
        status: pass
    human_judgment: false
  - id: D3
    description: "Existing MCP HTTP and Action contracts remain green after L2 and session changes."
    requirement: MCP-07
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-server --tests --locked -- --nocapture"
        status: pass
    human_judgment: false

duration: 38m
completed: 2026-10-29
status: complete
---

# Phase 7 Plan 8: MCP L2 and HTTP Session Isolation Summary

**Authenticated `sql:raw` MCP discovery/execution with typed EXPLAIN, audited limits, and bounded legacy session ownership.**

## Performance

- **Duration:** 38m
- **Started:** 2026-10-29T15:37:37Z
- **Completed:** 2026-10-29T16:15:28Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments

- Added `execute_query`, `list_catalogs`, `list_schemas`, `list_tables`, `get_table_schema`, and `explain_query` to the `sql:raw` tool catalog, including exact argument schemas and four typed EXPLAIN formats.
- Routed raw execution and discovery through AST validation, scope allow-list checks, execution limits, and durable read audit; discovery output is sorted, deduplicated, and filtered before returning.
- Added bounded legacy session owner admission and per-request authentication checks, plus transport coverage for 2026-07-28 metadata, deletion, expiry, capacity eviction, cancellation, and reconnect behavior.

## Task Commits

1. **Task 08-1: 接齐六个 sql:raw 工具和授权结果边界** - `df0614f` (test), `89b4a8b` (feat)
2. **Task 08-2: 使旧 session 与新无 session 协议逐请求认证** - `bfbf4f6` (feat)

## Files Created/Modified

- `crates/ontoforge-server/src/mcp_l2.rs` - Generic six-tool L2 adapter, typed EXPLAIN parsing, discovery filtering, limits, and audit.
- `crates/ontoforge-server/src/mcp_sessions.rs` - TTL/capacity-bounded `SessionOwners` admission map.
- `crates/ontoforge-server/src/mcp.rs` - Raw tool dispatch and session owner checks after authentication.
- `crates/ontoforge-server/src/mcp_catalog.rs` - Raw tool constants and JSON schemas.
- `crates/ontoforge-server/tests/mcp_l2.rs` - Six-name/schema and revoke baseline tests.
- `crates/ontoforge-server/tests/mcp_transport.rs` - Legacy/new protocol and session lifecycle HTTP tests.

## Decisions Made

- Reused the approved L2 AST validator and typed EXPLAIN printer so MCP adapters cannot bypass SQL policy or audit paths.
- Kept session state to owner and lifecycle timestamps only; authorization is rebuilt from the current request snapshot.
- Recorded rmcp's JSON-RPC error-over-HTTP-200 behavior explicitly in tests instead of asserting transport-level 401 for handler authentication errors.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added missing MCP L2 and session modules and test entry points**
- **Found during:** Task 08-1 initialization
- **Issue:** All four plan-listed production/test files were absent, so the existing crate had no raw tool or session implementation surface.
- **Fix:** Added the modules and exposed them through `mcp.rs` path declarations, preserving the existing `lib.rs` boundary.
- **Files modified:** `crates/ontoforge-server/src/mcp.rs`, `crates/ontoforge-server/src/mcp_l2.rs`, `crates/ontoforge-server/src/mcp_sessions.rs`, tests.
- **Verification:** Targeted MCP L2 and transport tests plus full server tests passed.
- **Committed in:** `89b4a8b`, `bfbf4f6`

---

**Total deviations:** 1 auto-fixed (Rule 3)
**Impact on plan:** Missing files were the planned production artifacts; no dependency or unrelated scope was added.

## Issues Encountered

- `cargo clippy -p ontoforge-server --all-targets --locked -- -D warnings` remains blocked by two pre-existing `clippy::doc_markdown` diagnostics in `crates/ontoforge-api/src/openapi.rs:202,208`; that file is outside 07-08 ownership and was left unchanged.
- Warp and `ralph-loop` were unavailable in this executor environment; execution used PowerShell, `apply_patch`, Cargo, and real HTTP fixture tests.

## User Setup Required

None - no external service configuration required for this plan. Real Trino validation remains with 07-13.

## Next Phase Readiness

07-09 and 07-13 can consume the six raw tool names and typed EXPLAIN dispatch. 07-16 remains responsible for trusted deployment defaults and qualified table context. The existing Clippy diagnostics should be addressed in the owning API plan or a later cleanup plan.

## Self-Check: PASSED

- SUMMARY file exists at `.planning/phases/07-rest-mcp-shell/07-08-SUMMARY.md`.
- Task commits `df0614f`, `89b4a8b`, and `bfbf4f6` are ancestors of `HEAD`.
- Measured plan commit count is 3 from ledger base `c80082cb90b5928a6c20fba5da8b0b54cd56e60b`.

---
*Phase: 07-rest-mcp-shell*
*Completed: 2026-10-29*
