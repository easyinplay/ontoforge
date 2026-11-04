---
phase: 04-scoped-l0-read
plan: 03
subsystem: storage
tags: [rust, sqlite, postgresql, keyset, edit-overlay, tdd]
requires:
  - phase: 04-01
    provides: single-key read-only edit lookup and shared typed row decoder
provides:
  - bounded object edit batch and stable keyset scan with version-zero filtering
  - bounded M:N edit endpoint scans with typed keys, deleted state and version
  - SQLite conformance and ignored real PostgreSQL parity targets
affects: [04-04, 04-05, 04-07]
actuals:
  tokens: 12078
  tasks: 2
  commits: 4
commits: 4
plan_head_before: 413dd5f55b61b758afdbf23b3aac07f6046f620c
plan_head_after: 605ebcafba408565ef3bc6766cac94008f54c39f
tech-stack:
  added: []
  patterns: [pool-only typed SELECT, limit-plus-one keyset page, shared edit value decoder]
key-files:
  created:
    - crates/ontoforge-storage/tests/query_read.rs
    - crates/ontoforge-storage/tests/query_read_pg.rs
  modified:
    - crates/ontoforge-storage/src/query_read.rs
    - crates/ontoforge-storage/src/sql_exit.rs
    - crates/ontoforge-storage/src/commit.rs
key-decisions:
  - Object batches preserve the caller's key order and duplicate positions; SQL predicates deduplicate canonical keys.
  - Object and M:N scans use physical key comparisons inside storage while exposing only typed logical keys and opaque cursors.
patterns-established:
  - Single-key object read delegates to the bounded batch and shared decode path.
  - All read queries run through pool-only SELECT and one SQL exit helper; no write transaction or loader is involved.
requirements-completed: [QUERY-04, QUERY-05, QUERY-08, QUERY-09]
coverage:
  - id: OBJECT-EDIT-READ
    description: Bounded ordered object batch and keyset scan, typed values, tombstone and release, version-zero filtering
    requirement: QUERY-04
    verification:
      - kind: integration
        ref: cargo test -p ontoforge-storage --test query_read --locked
        status: pass
    human_judgment: false
  - id: LINK-EDIT-READ
    description: Bounded from/to M:N endpoint scans preserve removed edges and reject FK links by logical name
    requirement: QUERY-05
    verification:
      - kind: integration
        ref: cargo test -p ontoforge-storage --test query_read --locked link_
        status: pass
    human_judgment: false
  - id: EDIT-READ-ISOLATION
    description: SQLite read operations preserve row, log and definition counts and keep physical identifiers out of errors
    requirement: QUERY-08
    verification:
      - kind: integration
        ref: cargo test -p ontoforge-storage --locked
        status: pass
    human_judgment: false
duration: 34min
completed: 2026-10-22
status: complete
---

# Phase 4 Plan 03: Bounded Edit Read Summary

Object and M:N edit candidates now have typed, bounded pool-only read APIs for Rust overlay, with stable keyset pages and no version-zero candidates.

## Accomplishments

- `read_object_edits(active, type_name, keys) -> Vec<Option<ObjectEdit>>` preserves request order; at most 256 keys enter a batch. `scan_object_edits(active, type_name, after, limit) -> EditScanPage` uses physical pk `>` plus `ORDER BY` and `LIMIT limit+1`, with `limit` in 1..=256.
- `read_link_edits(active, link_name, LinkEndpoint::{From,To}, after, limit) -> LinkScanPage` scans only M:N edit tables. Typed `LinkEdit` carries from/to keys, deleted and version; FK link requests fail by logical link name.
- The single-key API delegates to batch decoding. Both backends reuse `commit` value conversion and the new pool-only bounded row fetch. SQLite tests cover all BaseType families, null versus unset, flags, pagination, bounds, zero side effects and logical-only errors.

## TDD Gate Compliance

- Task 1 RED `c0cb883`: `version_zero_skeleton_is_not_an_edit_and_read_does_not_mutate` executed and failed at its planned assertion (`version 0 is not an overlay candidate`). GREEN `8008d79`: the shared batch decoder filters version zero; object target 7/7 then passed.
- Task 2 RED `9c6785e`: `link_from_or_to_endpoint_is_bounded_and_preserves_removed_edges` executed against an empty API scaffold and failed at its planned value assertion (`left: []`, `right: [Int(1), Int(2)]`). GREEN `605ebca`: typed endpoint reads and PostgreSQL ignored target; SQLite object/link target 11/11 then passed.
- Final `cargo test -p ontoforge-storage --locked` passed every non-ignored target, including 48/48 library, 28/28 SQLite conformance and 11/11 new read tests. `cargo test -p ontoforge-query --locked` passed pk 10/10, security 6/6, SQL shape 5/5 and SQLite query 10/10. Storage Clippy with `-D warnings`, fmt check, SQL exit guard and `git diff --check` passed.

## PostgreSQL Status

`cargo test -p ontoforge-storage --test query_read_pg --locked -- --ignored --list` listed two parity tests. `ONTOFORGE_PG_URL` is unset locally; an actual ignored run failed loudly with the harness's missing-server message. Real PostgreSQL object/link parity remains for the 04-07 Linux service gate; no parity pass is claimed here.

## Task Commits

1. Task 1 RED `c0cb883`; GREEN `8008d79`.
2. Task 2 RED `9c6785e`; GREEN `605ebca`.

## Deviations from Plan

- `[Rule 2 - Critical]` Added `fetch_all_cells_read` in `sql_exit.rs`, replacing the obsolete single-row pool helper, so bounded reads stay inside the one SQL exit boundary without write transactions.
- `[Rule 1 - Bug]` The shared `decode_object` corrupt-value error named a physical column. It now names the logical property; `corrupt_property_error_uses_logical_name` guards the outward contract.
- The planned RED name `batch_preserves_requested_keys_and_decodes_every_type` is split into request-order/null and all-value-type tests for clearer failures. The 257-key test proves bounds before SQL by dropping the edit table first and observing the logical bounds error.

## Known Stubs

None. The Task 2 RED scaffold was replaced in the GREEN commit.

## Self-Check: PASSED

Both new test files, the storage API and this SUMMARY exist; all four task commits are ancestors of HEAD. `plan_head_before..plan_head_after` measures four commits, and the scoped diff is 48,313 characters (12,078 estimated tokens).
