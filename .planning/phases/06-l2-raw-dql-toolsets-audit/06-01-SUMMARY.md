---
phase: 06-l2-raw-dql-toolsets-audit
plan: 01
subsystem: database
tags: [sqlite, read-audit, query, tdd]
requires:
  - phase: 03-edit-store-scope-model
    provides: EditStore migrations, definition snapshots and scope grants
  - phase: 04-scoped-l0-read
    provides: QueryService pk read entry point
provides:
  - SQLite of_read_audit table with retention and indexed timestamp
  - Bounded read-audit acceptance and asynchronous batch sink for public pk reads
  - Fail-closed audit health, cancellation and shutdown behavior
affects: [06-02, 06-05, 06-06, 06-09]
actuals:
  tokens: 12004
  tasks: 2
  commits: 4
commits: 4
plan_head_before: 2ad7e7eeb34ed504ce44c34db6815de0e18354c0
plan_head_after: 98203990c80ac9a85b182874440fceade38211a0
tech-stack:
  added: []
  patterns: [reserved audit permit before source read, cancellation-safe audit guard, bounded SQLite batch worker]
key-files:
  created:
    - crates/ontoforge-storage/migrations/sqlite/0005_read_audit.sql
    - crates/ontoforge-storage/src/read_audit.rs
    - crates/ontoforge-query/tests/audit_pk.rs
  modified:
    - crates/ontoforge-storage/src/lib.rs
    - crates/ontoforge-storage/src/store.rs
    - crates/ontoforge-query/src/lib.rs
key-decisions:
  - "EditStore owns the audit worker; QueryService keeps its existing public fields."
  - "Asynchronous audit acceptance precedes result release but is not a durability acknowledgement."
  - "Persistent SQLite sink failure closes admission and returns an error from shutdown."
requirements-completed: [AUDIT-01]
coverage:
  - id: D1
    description: SQLite pk reads record success, denial, source error and cancellation without result data or raw SQL
    requirement: AUDIT-01
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-query --test audit_pk --locked audit_pk_ -- --nocapture"
        status: pass
    human_judgment: false
  - id: D2
    description: Bounded audit worker closes admission on queue or sink failure, drains on shutdown and recovers after transient failure
    requirement: AUDIT-01
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-query --test audit_pk --locked audit_worker_ -- --nocapture"
        status: pass
    human_judgment: false
duration: 34min
completed: 2026-10-26
status: complete
---

# Phase 06 Plan 01: SQLite Pk Read Audit Summary

SQLite pk reads now reserve bounded audit capacity, submit metadata before returning a result, and fail closed when the worker cannot accept further reads.

## Accomplishments

- `of_read_audit` records principal, logical query metadata, definition version, request ID, SQL hash or `no-sql`, status, row count and elapsed time. The integration test checks the database for raw SQL and result leakage.
- `QueryService::execute` reserves capacity for the public pk read and records success, denied, source_error or cancelled; a full queue or unhealthy sink returns `AuditUnavailable` without a new result.
- The SQLite worker batches writes, exposes pending age and health, drains accepted records during orderly shutdown and reports failure after a persistent sink error. A later store open starts a fresh worker.

## TDD Evidence

- Task 1 RED `0981150`: `audit_pk_success_records_one_metadata_row` asserted that a successful pk read creates the durable SQLite audit table and one metadata row. Before the migration and audit wiring, that target assertion failed. GREEN `a531947`: migration, `ReadAudit`, `EditStore` ownership and `QueryService` gate made the pk audit tests pass.
- Task 2 RED `8faa2c9`: `audit_worker_persistent_sink_failure_stops_and_reports_failed_drain` removed the table after accepting a read and required `shutdown()` to return an error within three seconds; the prior worker retried indefinitely, so the target timed out. GREEN `9820399`: bounded retries, error propagation and tests for transient recovery, oldest pending age, orderly drain, permit recovery and configuration bounds.
- The prior RED runs were completed before this continuation; their raw console transcripts are not retained in the worktree. The RED commits preserve the assertions. No refactor-only commit was needed.

## Verification

- `cargo test -p ontoforge-query --test audit_pk --locked audit_worker_ -- --list`: 5 tests discovered.
- `cargo test -p ontoforge-query --test audit_pk --locked audit_worker_ -- --nocapture`: 5 passed.
- `cargo test -p ontoforge-query --locked -j 2`: all enabled unit, integration and doc tests passed, including 9 `audit_pk` tests. Six existing external-backend tests remained ignored because their dedicated Trino/PostgreSQL URLs were not provided.
- `cargo clippy -p ontoforge-query -p ontoforge-storage --all-targets --locked -- -D warnings`, `cargo fmt --check` and `git diff --check`: passed.

## Task Commits

1. Task 1 RED: `0981150` (`test(06-01): add failing pk read audit assertion`)
2. Task 1 GREEN: `a531947` (`feat(06-01): audit pk reads before releasing results`)
3. Task 2 RED: `8faa2c9` (`test(06-01): expose persistent audit worker drain failure`)
4. Task 2 GREEN: `9820399` (`fix(06-01): bound failed audit worker drain`)

## Decisions Made

- A failed batch retry remains bounded by `max_pending_age`; its pending permits are released when the worker exits, and subsequent reads fail closed.
- The SQLite pk slice satisfies this plan's audit coverage. `AUDIT-01` as a phase-wide requirement remains pending until PostgreSQL and the other public read paths are implemented and verified in later Phase 6 plans.

## Deviations from Plan

None. The extra worker assertions are within Task 2's recovery, age, configuration and cancellation acceptance criteria.

## Known Stubs

| File | Line | Reason |
| --- | ---: | --- |
| `crates/ontoforge-storage/src/read_audit.rs` | 166 | PostgreSQL worker explicitly fails closed until `06-02` provides its system table and sink. This does not affect the SQLite pk tracer. |

## Limitations

- Async audit acceptance is not a durable-write acknowledgement. The approximately one-second pending-age gate was measured with a blocked SQLite writer; sudden power loss was not simulated.
- Phase 6 verification must also cover PostgreSQL, all public read entry points and real Trino/ACL behavior before `AUDIT-01` can be considered complete across the product.

## Self-Check: PASSED

All six changed files exist; `0981150`, `a531947`, `8faa2c9` and `9820399` are ancestors of HEAD; the recorded pre-plan HEAD yields four task commits. The intentional PostgreSQL stub is tracked in `.planning/WINDOWS.md` as entry 11.
