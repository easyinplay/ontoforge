---
phase: 05-online-action
plan: 05
subsystem: action-storage
tags: [rust, sqlite, postgresql, idempotency, authorization, tdd]
requires:
  - phase: 05-04
    provides: atomic Action command, log, edit and outbox transaction with PostgreSQL race coverage
provides:
  - principal-scoped command winner, loser, rollback and bounded wait on real PostgreSQL
  - versioned canonical client request fingerprint and seven-day command expiry
  - current grant and active definition checks before minimum historical receipt disclosure
affects: [05-06, 05-07]
actuals:
  tokens: 13720
  tasks: 2
  commits: 5
commits: 5
plan_head_before: 9daebcd72852f43652a05e4c18a1f87419733132
plan_head_after: 3096bf96ebb96380781b7698c169707bedcb8569
tech-stack:
  added: []
  patterns: [database unique claim, post-conflict fresh read, canonical SHA-256 request hash, injected UTC clock]
key-files:
  created:
    - crates/ontoforge-action/src/fingerprint.rs
    - crates/ontoforge-action/tests/idempotency.rs
    - crates/ontoforge-action/tests/idempotency_pg.rs
  modified:
    - crates/ontoforge-action/src/lib.rs
    - crates/ontoforge-action/tests/action_pg.rs
    - crates/ontoforge-storage/src/commit.rs
key-decisions:
  - Expired command keys are conditionally retired and replaced inside the claim transaction; historical log and outbox remain intact.
  - Replay reads the current database definition, including another process's changes, and returns only the original principal's minimum receipt under the supplied current action grant.
  - Client presence and recursive JSON key order are encoded in a versioned request fingerprint; chosen server time and command ID are excluded.
requirements-completed: [ACTION-10]
coverage:
  - id: ACTION-IDEMPOTENCY-PG
    description: PostgreSQL unique command claim converges on one winner across same-key, different-parameter, abort, response-loss and expiry races.
    requirement: ACTION-10
    verification:
      - kind: integration
        ref: cargo test -p ontoforge-action --test idempotency_pg --features embedded-pg --locked -- --ignored --nocapture
        status: pass
    human_judgment: false
  - id: ACTION-REPLAY-AUTH
    description: SQLite replay and result lookup enforce the principal, current action grant and database definition while preserving a minimum historical receipt.
    requirement: ACTION-10
    verification:
      - kind: integration
        ref: cargo test -p ontoforge-action --features embedded-pg --locked
        status: pass
    human_judgment: false
duration: 71min
completed: 2026-10-23
status: complete
---

# Phase 5 Plan 05: Principal-Scoped Action Idempotency Summary

Durable command claims now determine the winner on SQLite and real PostgreSQL; authorized retries return the original minimum batch receipt without another edit, log or outbox write.

## Accomplishments

- PostgreSQL contenders use the database unique key. A loser ends its transaction and reads the committed winner in a fresh snapshot; rollback permits another winner, and claim waiting is bounded.
- The request fingerprint sorts recursive JSON object keys, keeps array order and explicit parameter presence, and excludes command ID and server-selected time. A test clock fixes the seven-day expiry boundary and the original chosen time in receipts.
- Replay and `get_command_result` are principal-scoped and check the current action grant and active database definition. Revoked grants or removed actions deny disclosure; row predicate changes, deletion and a released-row flag do not remove the minimum historical summary.
- An expired command row is replaced in the claim transaction without deleting the old edit log or outbox. Concurrent PostgreSQL replacement has one new event and one command row.

## TDD Gate Compliance

- Task 1 RED `17832b2` ran a real PostgreSQL deletion/retry case and failed the intended replay assertion. Its native JUnit record `target/gsd-05-05-task1-red.json` passed `tdd-red-evidence` as `RED_EVIDENCE_OK`. GREEN `f15333c` resolved the winner/loser and fresh-snapshot path.
- Task 2 RED `a3959d4` demonstrated that an expired key replayed an old event; RED `f69f2a1` demonstrated that an instance with a stale definition disclosed a receipt after another process removed the action. Native JUnit records `target/gsd-05-05-task2-red.json` and `target/gsd-05-05-task2-external-red.json` were each classified `RED_EVIDENCE_OK`, with the intended assertion failing. GREEN `3096bf9` passed the focused corpus and full Action suite.
- The plan ledger measures five commits from `9daebcd` to `3096bf9`. RED precedes each task's GREEN; no test failure was inferred from a build error.

## Verification

- `cargo test -p ontoforge-action --features embedded-pg --locked`: 34 non-ignored tests passed. SQLite `idempotency` corpus: 9/9.
- Real PostgreSQL `idempotency_pg --ignored`: 6/6; `action_pg --ignored`: 4/4 after the Clippy fix.
- `cargo test -p ontoforge-storage --locked -j 2`: 48/48 unit tests and all non-ignored integration tests passed. The first unrestricted build was interrupted because Windows started 15 large linker jobs; the limited rerun exited 0.
- `cargo clippy -p ontoforge-action -p ontoforge-storage --all-targets --features ontoforge-action/embedded-pg --locked -- -D warnings`, `cargo fmt --all --check` and `git diff --check` passed.

## Deviations from Plan

- [Rule 1 - Bug] A different process could remove an action while a service instance still held an old active definition. Action execution and receipt lookup now read the database's current definition; the dedicated RED test covers both entry points.
- [Rule 3 - Blocking lint] The larger Action future made four existing `action_pg` race tests exceed Clippy's `large_futures` threshold. Those call sites box the race future; all four real PostgreSQL cases still pass.

## Boundary

The release replay test changes an `of_released` row flag after the original command. The current `editOnly` Action type rejects actual `Release`, so this test does not claim source-backed release execution. The source-backed release path belongs to 05-06.

## Self-Check: PASSED

The SUMMARY and three created source/test files exist; all five task commits are ancestors of HEAD. The persisted ledger records five commits between the stated before/after SHAs. No task code remains uncommitted; unrelated pre-existing untracked files and the phase progress edit are left for the main session.
