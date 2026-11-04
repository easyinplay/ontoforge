---
phase: 03-edit-store-scope-model
plan: 18
subsystem: storage
tags: [reload, def-version-fence, lock-timeout, advisory-lock, single-flight, role-recheck, trino-probe, two-step-switch, printer, tdd]
status: complete

requires:
  - phase: 03-edit-store-scope-model
    provides: "03-04 reload; 03-10 index retention; 03-13 shared commit write path; 03-15 execute_steps / switch_definition shared with migrate; 03-16 scope_store, role_index_steps, advisory lock; 03-14 compile_scope"
provides:
  - "commit fence: first statement of every write transaction reads of_def_state.active_def_version (FOR SHARE on PostgreSQL); a commit for any other version is EditError::DefinitionStale, and a store whose cache lags (or that was opened before the first reload) loads the active definition from the database"
  - "reload under the definition lock: begin_definition_tx (SET LOCAL lock_timeout, then the advisory lock; BEGIN IMMEDIATE on SQLite), state row locked before any DDL, active definition / diff / roles recomputed inside the lock (single flight)"
  - "ReloadOptions { dry_run, roles: Option<RolesCheck>, probe: Option<ProbeContext> }, RolesCheck, ProbeTarget (impl for AnyEngine), ProbeContext, ReloadError::{LockTimeout, NotVisible}, MigrateError::{LockTimeout, NotVisible}, StoreError::Conflict, apply_migration_probed, store::definition_lock_key"
  - "two-step switch: insert_pending / activate; a pending version is reused by a rerun of the same definition and superseded by any other"
  - "engine: printer::print_flush_metadata_cache (printer/call.rs); source guard knows CALL"
  - "CLI: reload runs RolesCheck and the Trino probe, migrate takes the same lock/probe; LockTimeout and NotVisible are ONT-V26 with exit code 3"
affects: [03-19, 03-20, phase-04-query, phase-05-actions]

plan_head_before: 07736b9b1426cc94b3d324d84a8a683cc0da9666

actuals:
  tokens: 39000
  tasks: 3
  commits: 8

tech-stack:
  added: []
  patterns:
    - "Fence: the write transaction's first statement reads the state row (FOR SHARE); a switch takes the same row FOR UPDATE before it runs any DDL, so a commit that holds the fence and waits for a table lock can never deadlock with a DDL that holds the table lock and waits for the fence"
    - "Decide under the lock: everything a reload compares (active definition, diff, stored roles, existing indexes) is read again after the lock is taken"
    - "Pending version commits with the additive DDL; activation is a second short transaction that re-checks the active version and that its version is still pending"

key-files:
  created:
    - crates/ontoforge-engine/src/printer/call.rs
    - crates/ontoforge-storage/tests/reload_trino_it.rs
  modified:
    - crates/ontoforge-storage/src/commit.rs
    - crates/ontoforge-storage/src/reload.rs
    - crates/ontoforge-storage/src/migrate.rs
    - crates/ontoforge-storage/src/sql_exit.rs
    - crates/ontoforge-storage/src/store.rs
    - crates/ontoforge-storage/src/error.rs
    - crates/ontoforge-storage/src/scope_store.rs
    - crates/ontoforge-storage/tests/reload.rs
    - crates/ontoforge-storage/tests/reload_pg.rs
    - crates/ontoforge-storage/tests/scope_store.rs
    - crates/ontoforge-storage/tests/support/mod.rs
    - crates/ontoforge-engine/src/printer/mod.rs
    - crates/ontoforge-engine/src/printer/tests.rs
    - crates/ontoforge-engine/tests/trino_client.rs
    - crates/ontoforge-engine/tests/sql_text_only_from_printer.rs
    - crates/ontoforge/src/reload.rs
    - crates/ontoforge/src/migrate.rs
    - crates/ontoforge/tests/cli_reload.rs

key-decisions:
  - "The state row is locked FOR UPDATE before any DDL (reload and migrate). The RED run on PostgreSQL showed the deadlock (40P01) between a commit holding the fence and a reload holding a table lock; the plan's 'UPDATE of_def_state waits for in-flight writers' is kept, just taken earlier"
  - "SET LOCAL lock_timeout comes before the advisory lock (the plan listed them the other way round), so the wait for the advisory lock is bounded too; scope apply (03-16) had the same order and was swapped"
  - "The advisory lock key depends on the edit schema (store::definition_lock_key). Advisory locks belong to the database, and with one key two deployments, or concurrent test runs on one server, serialized each other's definition changes and made short-timeout stores fail"
  - "The D-18 role check runs inside the definition lock (scope_store::load_scope_in), not before it as the plan wrote, so a role applied between check and switch cannot slip through. Still before any DDL; the transaction is rolled back on failure"
  - "reload with roles always takes the lock path (no fast no-op return), because index reconciliation must run on an unchanged definition. Row filter indexes are only listed in ReloadOutcome.ddl when missing and do not create a definition version"
  - "Probe expectations are the tables a step creates and the non-array value columns it adds (the __set flag is not asked for), so a timeout reports e.g. of_obj_gate.zone. A change without such tables or columns (metadata only, array only) makes no call to Trino at all"
  - "migrate cannot use the pending-then-activate order: archive and rename steps are not additive, committing them first would break the still-active definition and a retry of the plan would fail. migrate switches in one transaction and waits for Trino afterwards; MigrateError::NotVisible means 'applied and active, Trino lags'. Interpretation of the plan's 'migrate takes the same probe path'"
  - "The probe's flush failing does not abort the wait (the cache expires by itself); a Trino that cannot be asked counts as 'nothing visible' and the error text is appended to the missing list"
  - "Trino client already returned an empty QueryOutput for a result without columns, so the task only added the wiremock tests (mutation check recorded below)"

patterns-established:
  - "Test stand-in for Trino: a scripted ProbeTarget (tests/reload.rs); PostgreSQL concurrency tests open a second EditStore on the same schemas (PgTestStore::open_another) to model another process with its own cache"

requirements-completed: [SCOPE-06]
requirements-addressed: [STORE-02, SCOPE-06, SCOPE-07, STORE-01]
# SCOPE-06: apply side (03-16) and reload side (this plan) both refuse a role whose columns do not cover its criteria, so it is ticked.
# STORE-02: lock_timeout, fence and the pending / probe / active switch are implemented and tested with a scripted Trino and on PostgreSQL, but the real
#   Trino CALL / visibility behaviour (RESEARCH A5) is only checked by the ignored reload_trino_it.rs on the Linux CI (03-20), and cross-version log
#   replay of migration_json is still ahead, so REQUIREMENTS.md stays unticked.
# SCOPE-07: the write side of 'a request uses the definition version it started with' is delivered (the fence); the read side is Phase 4/5. Unticked.
# STORE-01: the reload lock and the concurrency cases now run on both backends, but 03-19 (import) and 03-20 (Linux CI run of the PostgreSQL suite) are
#   still ahead. Unticked.

coverage:
  - id: D1
    description: "A commit validated against a definition version that is no longer active is refused by the database fence, also from a second process with a stale cache; a store opened before the first reload follows the database; after a refusal the store holds the active definition"
    requirement: SCOPE-07
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload.rs#stale_writer_rejected_after_switch"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload.rs#writer_without_a_loaded_definition_follows_the_database"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload_pg.rs#in_flight_commit_blocks_switch_on_pg"
        status: pass
    human_judgment: false
  - id: D2
    description: "Writers racing a switch only ever succeed or are refused as stale, and no log record under the old version follows one under the new (SQLite and PostgreSQL); a switch waits for commits in flight and no deadlock occurs"
    requirement: STORE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload.rs#reload_racing_commits_never_lets_an_old_write_through"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload_pg.rs#reload_racing_commits_never_lets_an_old_write_through_on_pg"
        status: pass
    human_judgment: false
  - id: D3
    description: "Two reloads of one new definition make one version (exactly one changes), on SQLite and PostgreSQL, with separate in-memory definitions"
    requirement: STORE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload.rs#concurrent_reloads_single_flight"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload_pg.rs#concurrent_reloads_single_flight_on_pg"
        status: pass
    human_judgment: false
  - id: D4
    description: "lock_timeout: a lock held by someone else (table lock, or the shared definition lock) gives LockTimeout for reload, migrate and (as SQLSTATE 55P03) scope apply within the configured time, and the old definition stays; two concurrent scope applies on PostgreSQL leave exactly one applied and audited"
    requirement: STORE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload_pg.rs#lock_timeout_keeps_old_definition"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload_pg.rs#definition_lock_is_shared_and_bounded_by_lock_timeout"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/scope_store.rs#pg_concurrent_scope_apply_serializes"
        status: pass
    human_judgment: false
  - id: D5
    description: "D-18: a reload that would break a stored role (ONT-V23) changes nothing and names role and type; the CLI exits 1 with ONT-V23; a dropped row filter index is recreated without a new version"
    requirement: SCOPE-06
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload.rs#reload_rechecks_applied_roles"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload.rs#reload_creates_row_filter_indexes"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_reload.rs#reload_reports_role_breakage"
        status: pass
    human_judgment: false
  - id: D6
    description: "Trino visibility probe with a scripted Trino: polls until the new column is visible, a timeout keeps a pending version and the old definition, a rerun reuses it, a different definition supersedes it, array columns and metadata-only changes are not waited for, the old definition serves writes during the wait (SQLite and PostgreSQL), migrate waits after its switch"
    requirement: STORE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload.rs#probe_waits_until_visible"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload.rs#probe_timeout_keeps_pending"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload.rs#array_columns_are_not_probed"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload.rs#the_old_definition_keeps_serving_while_the_probe_waits"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload.rs#migrate_waits_for_trino_after_the_switch"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload_pg.rs#pending_switch_on_pg_serves_the_old_definition_until_visible"
        status: pass
    human_judgment: false
  - id: D7
    description: "print_flush_metadata_cache prints a quoted CALL, only the printer can write it, and the Trino client turns a CALL answered without columns (also after a QUEUED page) into an empty QueryOutput"
    requirement: STORE-02
    verification:
      - kind: unit
        ref: "crates/ontoforge-engine/src/printer/tests.rs#flush_metadata_cache_quotes_the_catalog"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_client.rs#call_without_columns_returns_empty_output"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/sql_text_only_from_printer.rs#sql_text_only_from_printer"
        status: pass
    human_judgment: false
  - id: D8
    description: "Against a real Trino and PostgreSQL: reload with the probe makes the new tables readable through Trino, and a new column becomes visible within the probe timeout"
    requirement: STORE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload_trino_it.rs#reload_activates_once_trino_sees_the_new_tables"
        status: pass
    human_judgment: true
    rationale: "Ignored locally (no Trino on this machine); only listed with --list here. It runs on the Linux CI in plan 03-20, which is also where RESEARCH assumption A5 (CALL flush_metadata_cache through the client) is first checked. status: pass above means the test compiles and panics without ONTOFORGE_TRINO_URL; it has not run against Trino"
  - id: D9
    description: "The CLI decides when to probe (PostgreSQL store, [store.trino] catalog, selected profile on Trino)"
    requirement: STORE-02
    verification:
      - kind: unit
        ref: "crates/ontoforge/src/reload.rs#tests"
        status: pass
    human_judgment: false

duration: about 1 day wall clock including a terminal crash; roughly 4 h of work
completed: 2026-10-21
---

# Phase 3 Plan 18: Reload Hardening Summary

**A definition now becomes active only under the shared definition lock, after the roles still compile and (with a Trino in front of PostgreSQL) after Trino shows the new tables and columns, and every write is fenced in the database against the active `def_version`.**

## Accomplishments

- **Fence (T-03-58).** `EditStore::commit` reads `of_def_state.active_def_version` first (`FOR SHARE` on PostgreSQL), refuses any other version as `DefinitionStale` and loads the active definition from the database when its cache lags. Validation moved behind the fence.
- **Lock and single flight (T-03-60).** `begin_definition_tx` sets `lock_timeout` and takes the advisory lock; reload and migrate lock the state row before any DDL. `55P03` is `LockTimeout` (CLI: ONT-V26, exit 3). Two concurrent reloads make one version, on both backends.
- **D-18 (T-03-59).** With `RolesCheck`, the stored roles are compiled against the new definition inside the lock before any DDL; failure keeps the old definition. Row filter indexes are reconciled (add only) without a new version.
- **Trino probe (T-03-61, T-03-62).** `print_flush_metadata_cache`, `ProbeTarget` for `AnyEngine`, pending / activate switch, `NotVisible`, reuse and supersede of pending versions, CLI wiring, and the ignored Linux CI test.

## Task Commits

| Task | Name | Commit |
|------|------|--------|
| 1 | RED: stale writer, in-flight commit | a0d8915 |
| 1 | GREEN: fence | b53edfe |
| 2 | RED: single flight, lock timeout, roles, indexes | 61ec6bb |
| 2 | GREEN: lock, single flight, D-18, index reconciliation | 633da69 |
| 3 | RED: printer and client tests | 5c0e44d |
| 3 | GREEN: printer CALL | 13d63d9 |
| 3 | RED: probe tests | 279df66 |
| 3 | GREEN: probe, two-step switch, CLI, Trino IT | c62cc15 |

## TDD Gate Compliance

Every task has a `test(03-18)` commit before its `feat(03-18)` commit. Tests that did not fail first, and why:

- `reload_racing_commits_never_lets_an_old_write_through` (SQLite) passed at once because Task 1's fence already covered it. Mutation check: removing both version comparisons (fence and `validate`) makes it and `stale_writer_rejected_after_switch` fail. Removing only the fence comparison does not, because `validate` repeats the check (defence in depth).
- `call_without_columns_returns_empty_output` and `call_that_is_queued_first_still_ends_empty`: the client already behaved so; they failed in RED only because the printer was a stub. Mutation check: making `Collector::finish` require columns fails both.
- `metadata_only_change_needs_no_probe` guards an optimisation and passed at once.
- RED commits add the public types as empty shells so the tests compile and fail on behaviour (PostgreSQL RED runs showed the real double-activation, a 40P01 deadlock and unbounded waits).

## Deviations from Plan

**1. [Rule 1 - Bug] Deadlock between fence and DDL.** Found in the PostgreSQL RED run of `reload_racing_commits..._on_pg` (40P01). A commit holds the fence row and waits for a table lock; the DDL holds the table lock and then wants the state row. Fix: lock the state row first (`sql_exit::lock_def_state`) in reload and migrate. Commit 633da69.

**2. [Rule 1 - Bug] Advisory lock waits were unbounded, and the key was database-wide.** `lock_timeout` is now set before the advisory lock (also in scope apply), and the key depends on the edit schema (`store::definition_lock_key`). Found when a 200 ms store failed because other tests on the same server held the lock. Files: `store.rs`, `sql_exit.rs`, `scope_store.rs`. Commit 633da69.

**3. [Rule 4-adjacent, decided in place] migrate probes after its switch, not before.** See key-decisions; the plan's "same probe path" cannot apply to non-additive steps. `MigrateError::NotVisible` therefore means applied and active.

**4. [Plan deviation] D-18 check runs inside the lock** rather than before opening the transaction (closes a race with `scope apply`).

**5. [Rule 3] Files outside the plan's list:** `scope_store.rs` (`load_scope_in`, lock order), `store.rs`, `error.rs` (`StoreError::Conflict`), `sql_exit.rs`, `tests/support/mod.rs`, `tests/scope_store.rs`, `crates/ontoforge/src/migrate.rs`, engine `tests/sql_text_only_from_printer.rs` (CALL added to the guarded keywords).

**Total deviations:** 5 (2 bugs auto-fixed, 3 design adjustments). **Impact:** none on scope; the PostgreSQL behaviour is stricter than the plan.

## Open Items and Gaps

- **Trino against the real thing:** `reload_trino_it.rs` and RESEARCH A5 (`CALL ...flush_metadata_cache()` through the client) are unverified until the Linux CI (03-20). The CLI probe decision is covered by unit tests, the engine construction by compilation only.
- **Carried concurrency gaps:** concurrent `scope apply` on PostgreSQL is now tested (`pg_concurrent_scope_apply_serializes`); reload vs commit and reload vs reload are tested on SQLite and embedded PostgreSQL. Not covered: reload vs `scope apply` against each other (they share the lock; asserted only through the shared-key test), and SQLite `database is locked` after `busy_timeout` is still a plain `StoreError`, not `LockTimeout`.
- **Pending versions:** a pending version left by a failed probe stays pending if the user then reloads the old definition (nothing supersedes it until the next changing reload). Harmless; not cleaned.
- **docs:** the `NotVisible` help text points at `docs/edit-store.md` (03-20).

## Verification

- `cargo test --workspace --locked`: 947 passed, 0 failed, 62 ignored.
- `cargo clippy --workspace --all-targets --locked --features ontoforge-storage/embedded-pg -- -D warnings` and `cargo fmt --all --check`: clean.
- Embedded PostgreSQL (`--features embedded-pg -- --ignored`: reload_pg, scope_store, conformance_pg, migrate_pg), three consecutive runs: all pass (reload_pg 11, migrate_pg 3, conformance_pg 27, scope_store 3 per run).
- `reload_trino_it` lists 2 ignored tests and panics without `ONTOFORGE_TRINO_URL`.

## Known Stubs

None.

## Threat Flags

None beyond the plan's register. The CALL text is built only from a validated catalog name through `quote_ident` (T-03-62).

## Self-Check: PASSED

- Files exist: printer/call.rs, reload_trino_it.rs, this summary.
- Commits a0d8915, b53edfe, 61ec6bb, 633da69, 5c0e44d, 13d63d9, 279df66, c62cc15 are in `git log`.
- Acceptance greps: `LockType::Share` in commit.rs, `compile_scope` and `advisory_xact_lock` in reload.rs (via sql_exit), `pub fn print_flush_metadata_cache` in printer/mod.rs, ignored list contains `in_flight_commit_blocks_switch_on_pg`, `lock_timeout_keeps_old_definition`.
