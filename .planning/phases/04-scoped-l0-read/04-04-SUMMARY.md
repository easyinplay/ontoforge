---
phase: 04-scoped-l0-read
plan: 04
subsystem: query
tags: [rust, overlay, keyset, cursor, hmac, work-budget, tdd]
requires:
  - phase: 04-02
    provides: validated single-table source candidates and final Rust predicates
  - phase: 04-03
    provides: bounded edit keyset scans and typed object edits
provides:
  - scoped source/edit list overlay with bounded refill and 256-key backfill
  - signed normalized keyset cursor bound to principal, attributes, scope, definition, type and order
  - shared request work budget and final masked row/byte caps
affects: [04-05, 04-06, 04-07]
actuals:
  tokens: 28620
  tasks: 3
  commits: 14
commits: 14
plan_head_before: 204cb6bbf436f7b3846a33fc8a559269da9a352e
plan_head_after: ee46d5b4dc9ba1d45bda97ed254dc4111735e28b
tech-stack:
  added: []
  patterns: [independent overlay reference, bounded dual-lane keyset scan, eligible-row lookahead, signed cursor context]
key-files:
  created:
    - crates/ontoforge-query/src/overlay.rs
    - crates/ontoforge-query/src/cursor.rs
    - crates/ontoforge-query/src/execute.rs
    - crates/ontoforge-query/tests/overlay_prop.rs
  modified:
    - crates/ontoforge-query/src/lib.rs
    - crates/ontoforge-query/tests/security.rs
key-decisions:
  - Complete the bounded edit lane before source early-stop; use source-native keyset and eligible Rust lookahead.
  - Scan case-insensitive source order by PK to exhaustion before Rust ordering; budget failure is explicit.
  - Bind cursor context to effective scope and principal attributes because attributes can change scope evaluation without changing roles.
patterns-established:
  - Candidate lanes share one WorkBudget; no partial page is returned on budget failure.
  - Final output caps count complete masked ObjectRows including version after overlay and authorization.
requirements-completed: [QUERY-04, QUERY-08, QUERY-09]
coverage:
  - id: LIST-OVERLAY
    description: Independent reference and scoped source/edit overlay with refill, tombstone, release and stable ordering
    requirement: QUERY-08
    verification:
      - kind: integration
        ref: cargo test -p ontoforge-query --test overlay_prop --test security --locked
        status: pass
    human_judgment: false
  - id: BOUNDED-PAGE
    description: Parallel first candidates, bounded keyset windows, 256-key backfill and eligible-row has_more
    requirement: QUERY-04
    verification:
      - kind: integration
        ref: cargo test -p ontoforge-query --locked
        status: pass
    human_judgment: false
  - id: CURSOR-AND-LIMITS
    description: Context-bound HMAC cursor, shared work budget and final masked row/byte caps
    requirement: QUERY-09
    verification:
      - kind: integration
        ref: cargo test -p ontoforge-query --test security --locked
        status: pass
    human_judgment: false
duration: 46min
completed: 2026-10-22
status: complete
---

# Phase 4 Plan 04: Scoped List Overlay Summary

Source and edit candidates now produce one authorized, ordered list page with bounded refill, signed resumption and separate work/output limits.

## Accomplishments

- The independent `overlay_prop` reference covers partial set/null/unset, edit-only rows, tombstone/release, duplicate candidates, merged-row scope and property grants. Production comparison uses typed total order with canonical PK tie-break; Decimal comparison is exact over canonical values up to 38 digits.
- `execute_list` starts source and edit futures together, scans the bounded edit lane, backfills source by at most 256 keys per call, and evaluates overlay/scope/filter before sorting. Sortable source uses an exclusive keyset and stops only when its frontier is past an eligible lookahead row. Case-insensitive ordering scans by PK before Rust sorting.
- Cursor payload and HMAC bind position, normalized order, principal ID, attributes, effective scope, definition and object type. Work budget failure returns `WorkBudgetExceeded` without a partial page; only final row/byte caps set `truncated`.

## TDD Gate Compliance

- Task 1 RED `0c0dd2d` failed the independent overlay reference; GREEN `5c04205` passed. The property target now passes 11/11, including numeric Decimal order against a separate `normalize_decimal`/`i128` oracle.
- Task 2 RED `8eaf828` covered four exact default budget boundaries; RED `438ec60` covered real SQLite pagination; RED `d408a69` caught an unnecessary source window; RED `b1f0ef6` caught a source false-positive returned as `Source`. GREEN `88932c0` and `15b7ff6` completed the work budget and list executor. Scripted first-lane poll, 257-key backfill, three false-positive windows and two-lane budget cases were added as regression evidence for behavior already implemented.
- Task 3 RED `611757b` covered cursor mutation and replay; RED `ccdf737` caught missing initial source cursor seek. RED `ceba54b` caught Decimal lexical order, changed principal attributes and conflicting duplicate backfill rows. GREEN `04e1992` fixed those assertions. `ee46d5b` added a two-lane budget boundary regression.

## Verification

- `cargo test -p ontoforge-query --locked`: library 1/1, overlay 11/11, PK 10/10, security 21/21, SQL shape 5/5 and SQLite query 10/10. No failed or ignored tests in these targets.
- `cargo clippy -p ontoforge-query --all-targets --locked -- -D warnings`, `cargo fmt --all --check` and `git diff --check` passed.
- Budget evidence has two layers: `WorkBudget` tests charge each Phase 4 default exactly at B and reject B+1 for all four dimensions; the actual `execute_list` dual-lane test succeeds at calls=3/raw=2/backfill=1/exact decoded bytes and returns only `WorkBudgetExceeded` when calls, raw, bytes or both raw and bytes exceed limits. A real 257-edit scan produces 256+1 backfill batches; lowering the shared backfill budget to 256 fails before the second batch.

## Deviations from Plan

- `[Rule 1 - Bug]` Source frontier construction previously treated a backend false-positive scope row as a source error. It now computes the source's masked ordering keys independently of final eligibility (`15b7ff6`); the three-window refill test guards this case.
- `[Rule 1 - Bug]` Cursor scope fingerprint did not account for changed principal attributes; same-ID replay now fails closed (`04e1992`).
- `[Rule 1 - Bug]` Backfill could overwrite a different source row for the same PK; all candidate origins now reject conflicting duplicates (`04e1992`).
- `[Rule 1 - Bug]` Decimal strings sorted lexically in Rust; canonical scaled values now compare numerically (`04e1992`).

## Known Stubs

None.

## Next Phase Readiness

The shared list overlay and `WorkBudget` are ready for Phase 4 traversal, set and aggregate paths. Those paths still need their own end-to-end shared-budget and authorization checks in later plans.

## Self-Check: PASSED

- All six created or modified files listed above exist.
- Every cited RED/GREEN and regression commit is an ancestor of `ee46d5b`.
- `204cb6b..ee46d5b` contains 14 commits, matching `commits` and `actuals.commits`.
