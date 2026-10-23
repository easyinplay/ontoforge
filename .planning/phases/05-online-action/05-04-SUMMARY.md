---
phase: 05-online-action
plan: 04
subsystem: action-storage
tags: [rust, sqlite, postgresql, rollback, concurrency, cas, tdd]
requires:
  - phase: 05-01
    provides: storage-owned Action transaction, command claim and outbox
  - phase: 05-03
    provides: shared version token and lock-after-read CAS
provides:
  - deterministic test-only faults after four Action persistence stages
  - SQLite and real PostgreSQL rollback snapshots and success controls
  - two-connection PostgreSQL Action races with post-preparation synchronization
affects: [05-05, 05-06, 05-07]
actuals:
  tokens: 6376
  tasks: 2
  commits: 3
commits: 3
plan_head_before: 2c3cf559b11c61386376eb0da488998724ec4d5f
plan_head_after: e01a81433e67c9df1321749c91069d8184866d88
tech-stack:
  added: []
  patterns: [feature-gated transaction faults, post-preparation race barrier, cross-backend conformance]
key-files:
  created:
    - crates/ontoforge-action/tests/action_pg.rs
  modified:
    - crates/ontoforge-storage/src/commit.rs
    - crates/ontoforge-storage/tests/conformance/mod.rs
    - crates/ontoforge-storage/Cargo.toml
    - crates/ontoforge-action/src/lib.rs
    - crates/ontoforge-action/Cargo.toml
    - Cargo.lock
key-decisions:
  - Keep the fault selector behind test-support and route every injected error through the existing transaction rollback.
  - Synchronize PostgreSQL contenders after Action preparation so no-token races compare the same observed version.
  - Retain the existing RowKey locks and CAS because all four real PostgreSQL races passed without a production behavior change.
requirements-completed: []
coverage:
  - id: ACTION-ATOMIC-ROLLBACK
    description: Four Action write-stage failures restore row, version, log, history, command and outbox snapshots on both backends.
    requirement: ACTION-04
    verification:
      - kind: integration
        ref: cargo test -p ontoforge-storage --features test-support --test conformance_sqlite --locked action_atomic_ -- --nocapture
        status: pass
      - kind: integration
        ref: cargo test -p ontoforge-storage --features test-support,embedded-pg --test conformance_pg --locked action_atomic_ -- --ignored --nocapture
        status: pass
    human_judgment: false
  - id: ACTION-PG-RACES
    description: Two independent PostgreSQL stores have one Action winner and one VersionConflict for token, no-token, reverse-order multirow and absent-target insert races.
    requirement: ACTION-09
    verification:
      - kind: integration
        ref: cargo test -p ontoforge-action --features embedded-pg --test action_pg --locked concurrent_ -- --ignored --nocapture
        status: pass
    human_judgment: false
duration: 35min
completed: 2026-10-23
status: complete
---

# Phase 5 Plan 04: Action Rollback and PostgreSQL Race Summary

SQLite and real PostgreSQL now prove full Action rollback after log, edit rows, command receipt and outbox writes; synchronized two-connection PostgreSQL races confirm the existing lock and CAS behavior.

## Accomplishments

- `test-support` exposes `ActionWriteStage` and `commit_action_with_fault` only to integration builds. The normal Action entry point has no fault selector. Each error enters `finish_action` and rolls back the same transaction.
- The conformance matrix compares edit rows including version and deletion flags, log header/entries/history, command claims and receipts, and outbox rows against a pre-fault baseline. Reusing the same command after rollback succeeds.
- Four real PostgreSQL races use separate stores and a barrier after Action preparation. Each yields exactly one committed Action and one `VersionConflict`, one artifact set, and no timeout. Reverse request order finishes without deadlock; concurrent creation from an absent target rejects the loser via version-zero CAS.

## TDD Gate Compliance

- Task 1 RED `25d00f2`: SQLite `action_atomic_after_log_failure` failed its intended `AfterLog` assertion because the no-op test hook committed. `target/gsd-05-04-task1-red.json` was classified `RED_EVIDENCE_OK`; semantic inspection confirmed the target assertion, not a build or fixture failure. Real PostgreSQL ran the same ignored target and failed the same assertion (1 executed, 1 failed). GREEN `5d629cb` made both five-case matrices pass.
- Task 2 race assertions passed on their first real PostgreSQL run with the existing 05-03 CAS. This is a regression confirmation, not a fabricated RED. The post-preparation barrier made the no-token observation deterministic; the test-only work was committed in `e01a814`.
- The plan ledger measured three commits from `2c3cf55` through `e01a814`, with the required RED commit preceding implementation.

## Verification

- SQLite `action_atomic_`: 5 passed, 0 failed. Real PostgreSQL `action_atomic_ --ignored`: 5 passed, 0 failed, 0 ignored.
- Real PostgreSQL `concurrent_same_pk_ --ignored`: 2 passed; `concurrent_multirow_ --ignored`: 1 passed; `concurrent_guard_absent_ --ignored`: 1 passed.
- `cargo test -p ontoforge-storage --features test-support,embedded-pg --lib --locked`: 48 passed. `cargo test -p ontoforge-action --test version --locked observed_ -- --nocapture`: 1 passed.
- `cargo clippy -p ontoforge-storage -p ontoforge-action --features ontoforge-storage/test-support,ontoforge-action/embedded-pg --all-targets --locked -- -D warnings`, `cargo fmt --all --check`, and `git diff --check`: passed.

## Deviations from Plan

- The existing storage test support module was reused from the Action integration test. Three existing workspace packages became Action dev dependencies; `Cargo.lock` changed only the Action dependency list, with no new package or version.
- Existing 05-03 CAS already satisfied the race behavior. No production lock or isolation contract was changed. A feature-gated `execute_with_test_gate` provides deterministic test timing after preparation.
- The missing target case exercises `createObject` from version zero, with an explicit absent-row assertion before the two concurrent requests. It establishes absent-key CAS behavior; it does not claim a separate source-backed guard-read scenario.

## Self-Check: PASSED

The created PostgreSQL test and this SUMMARY exist; `25d00f2`, `5d629cb`, and `e01a814` are ancestors of HEAD. The measured ledger has three committed changes and no uncommitted task code. Phase-level ACTION-08/09 completion still depends on later Phase 5 plans and verify-work.
