---
phase: 03-edit-store-scope-model
plan: 09
subsystem: storage
tags: [edit-log, append-only, triggers, replay, adr, conformance, sqlite, postgres, tdd]
status: complete

requires:
  - phase: 03-edit-store-scope-model
    provides: "03-08 EditStore::commit, read_log, LogRecord/LogEntry, state::next_state/next_link_state, tests/conformance framework; 03-01 recursive_triggers = ON on every SQLite connection"
provides:
  - "Append-only triggers on of_edit_log, of_edit_log_entry, of_edit_history on both backends (STORE-03, D-12): UPDATE and DELETE refused everywhere, TRUNCATE refused on PostgreSQL, SQLite replace-style writes refused"
  - "PostgreSQL function of_reject_mutation() (SQLSTATE 23000), reusable by 03-16's of_admin_audit"
  - "log::replay and ReplayError: rebuilds the edit tables of a fresh store from read_log records, checks every version_after (D-10)"
  - "Conformance cases append_only_*, replay_*, same_commit_twice_creates_two_records (SQLite and PostgreSQL run the same bodies)"
  - "ADR-0026 edit log format"
affects: [03-13, 03-15, 03-16, 03-17, 03-20]

plan_head_before: 4d5b17f8c40b596a089065a6d2879a805aa166d8

actuals:
  tokens: 11800
  tasks: 2
  commits: 5

tech-stack:
  added: []
  patterns:
    - "Replay shares load_row / advance / update_row with commit (pub(crate) in commit.rs), so a transition rule lives in one place"
    - "Cases that exist for one backend are still listed in all_cases! and branch on the Harness, asserting the backend on the other side"
    - "Harness::sibling() builds a second empty store of the same kind for rebuild checks"

key-files:
  created:
    - .planning/adr/0026-edit-log-format.md
  modified:
    - crates/ontoforge-storage/migrations/sqlite/0002_edit_log.sql
    - crates/ontoforge-storage/migrations/postgres/0002_edit_log.sql
    - crates/ontoforge-storage/src/log.rs
    - crates/ontoforge-storage/src/commit.rs
    - crates/ontoforge-storage/tests/conformance/mod.rs

key-decisions:
  - "Replay applies all records in one transaction: a failed replay leaves the target untouched"
  - "A replay target must hold the same def_version as the records (EditError::DefinitionStale otherwise); cross-version replay waits for 03-15's migration_json"
  - "The RESEARCH A4 fallback was not needed: sqlx migrations run the plpgsql ($$) and SQLite BEGIN..END trigger scripts unchanged on both backends"

requirements-addressed: [STORE-03, STORE-01]
requirements-completed: []
# STORE-03: the log is immutable and replayable, but unset / tombstone / release / link remove entries only exist after
# 03-13, so the requirement's "edits" are not all writable yet. STORE-01 grows with every later conformance case.
# REQUIREMENTS.md stays unticked (policy decided in 03-01).

coverage:
  - id: D1
    description: "UPDATE and DELETE on the edit log, entry and history tables are refused on SQLite and PostgreSQL and change no row"
    requirement: STORE-03
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/conformance_sqlite.rs#append_only_rejects_update_and_delete"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/conformance_pg.rs#append_only_rejects_update_and_delete"
        status: pass
    human_judgment: false
  - id: D2
    description: "TRUNCATE of the log tables is refused on PostgreSQL (entry and history alone, all three together)"
    requirement: STORE-03
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/conformance_pg.rs#append_only_rejects_truncate_on_pg"
        status: pass
    human_judgment: false
  - id: D3
    description: "SQLite INSERT OR REPLACE, REPLACE INTO and ON CONFLICT DO UPDATE on the log tables are refused (recursive_triggers regression)"
    requirement: STORE-03
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/conformance_sqlite.rs#append_only_rejects_sqlite_replace_writes"
        status: pass
    human_judgment: false
  - id: D4
    description: "Replaying the log into a fresh store rebuilds every edit table column for column (including of_version, of_log_seq, of_def_version) and writes neither log nor history"
    requirement: STORE-03
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/conformance_sqlite.rs#replay_rebuilds_edit_store"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/conformance_pg.rs#replay_rebuilds_edit_store"
        status: pass
    human_judgment: false
  - id: D5
    description: "A tampered version_after stops the replay with Diverged { seq, ordinal } and leaves the target empty"
    requirement: STORE-03
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/conformance_sqlite.rs#replay_detects_version_divergence"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/conformance_pg.rs#replay_detects_version_divergence"
        status: pass
    human_judgment: false
  - id: D6
    description: "The same Commit submitted twice makes two log records with different event ids and version 2; the store does not deduplicate"
    requirement: STORE-01
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/conformance_sqlite.rs#same_commit_twice_creates_two_records"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/conformance_pg.rs#same_commit_twice_creates_two_records"
        status: pass
    human_judgment: false
  - id: D7
    description: "ADR-0026 records the edit log format from the 03-08 checkpoint, the append-only defences and the replay semantics"
    verification: []
    human_judgment: true
    rationale: "A design record; the text is judged by a reviewer, no test asserts it"

duration: "~32 min"
completed: 2026-10-20
---

# Phase 3 Plan 09: Append-only edit log, replay and ADR-0026 Summary

**The edit log, its entries and the history are now immutable on SQLite and PostgreSQL (triggers, including TRUNCATE and SQLite replace-style writes), and `log::replay` rebuilds a fresh store's edit tables column for column from the log while checking every logged version.**

## Accomplishments

- **Append-only (Task 1, tracer).** `0002_edit_log.sql` gains `BEFORE UPDATE` and `BEFORE DELETE` triggers on the three tables for SQLite (`RAISE(ABORT, 'table .. is append-only')`). For PostgreSQL it gains `of_reject_mutation()` (plpgsql, SQLSTATE `23000`), row triggers for UPDATE or DELETE and statement triggers for TRUNCATE. SQLite replace-style inserts delete the old row first; that DELETE reaches the trigger because every connection has `recursive_triggers = ON`.
- **Replay (Task 2).** `log::replay(records, target)` sorts by `seq`, resolves each entry against the target's active definition, and runs the commit path's `load_row`, `advance` (the same `next_state` and `next_link_state`) and `update_row`. The logged `version_after` must equal the computed version, otherwise `ReplayError::Diverged { seq, ordinal }`. `of_log_seq` keeps the original `seq` and `of_def_version` the logged value. The target's log and history are not written. Everything runs in one transaction.
- **Conformance.** Six new cases run on both backends from one body each: `append_only_rejects_update_and_delete`, `append_only_rejects_truncate_on_pg`, `append_only_rejects_sqlite_replace_writes`, `replay_rebuilds_edit_store`, `replay_detects_version_divergence`, `same_commit_twice_creates_two_records`. The two backend-specific ones assert the backend and return on the other side. The rebuild case commits five records over two object types and a link (one record with two edits of different kinds, one with two objects) and compares every column of all three edit tables, in pk order, between source and rebuilt store.
- **ADR-0026** written from 03-08's checkpoint conclusion: option-a layout plus supplements S1-S4, the research facts, seq-after-row-lock and the PostgreSQL live-tail caveat, `system:import`, replay semantics, the two defences (triggers and a deployment-time REVOKE) and the A4 outcome. It states that the format decision was taken by the orchestrator at the 03-08 checkpoint under the user's decision-autonomy preference.

## Task Commits

1. Task 1 RED `f96319e` test(03-09): append-only cases; 2 of the 3 targets fail (`UPDATE of of_edit_log must be refused`, `INSERT OR REPLACE INTO of_edit_log must be refused`), the PG-only case is a no-op green on SQLite by design.
2. Task 1 GREEN `511a672` feat(03-09): triggers in both `0002_edit_log.sql`. Tracer gate: `HUMAN_VERIFY_MODE` is `end-of-phase` and `<verify>` is automated only, so it was re-run end to end after GREEN (SQLite 3/3, PostgreSQL 14/14) and passed; expansion continued.
3. Task 2 RED `28d7ba8` test(03-09): replay cases against a `replay` stub; `replay_rebuilds_edit_store` and `replay_detects_version_divergence` fail with `Store(Config("replay is not implemented"))`.
4. Task 2 GREEN `163b494` feat(03-09): `replay`, plus `pub(crate)` on six items of `commit.rs`.
5. Task 2 `00b213d` docs(03-09): ADR-0026.

## Verification

- `cargo test --workspace --locked`: 852 passed, 0 failed, 37 ignored.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: clean. `cargo clippy -p ontoforge-storage --all-targets --features embedded-pg --locked -- -D warnings`: clean. `cargo fmt --all --check`: clean.
- `conformance_sqlite`: 17 passed. `conformance_pg --features embedded-pg -- --ignored` (embedded server, no Docker, Windows 11): 17 passed in about 6-9 s; `reload_pg` 5 passed. `conformance_pg -- --ignored --list` lists `append_only_rejects_truncate_on_pg`.
- `sql_exit_guard`: ok. `rg "pub async fn replay" src/log.rs` and `rg "status: accepted" .planning/adr/0026-edit-log-format.md` match; `BEFORE TRUNCATE` is in the PostgreSQL migration and `RAISE(ABORT` in the SQLite one.

## TDD Gate Compliance

Both tasks have RED (`test(03-09)`) before GREEN (`feat(03-09)`), no refactor commit. `gsd_run check tdd-red-evidence` classifies node TAP output that `cargo test` does not produce (same as 03-08), so no record file was generated; the failing test lists above are the evidence.

Two RED notes. The PG-only truncate case and the SQLite-only replace case pass on the other backend by design (they assert the backend and return). `same_commit_twice_creates_two_records` was green at RED: it characterises the commit path from 03-08, which already writes one record per commit (investigated per the fail-fast rule; no test or feature defect).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Replay needs the commit path's row helpers**
- **Found during:** Task 2
- **Issue:** `Target`, `RowState`, `load_row`, `advance`, `update_row` and `primary_key_def` were private to `commit.rs`; `log::replay` (required there by the plan) and the plan's "shared `next_state`" key link need them.
- **Fix:** made those items `pub(crate)`. No logic change. `commit.rs` is not in the plan's file list.
- **Files modified:** `crates/ontoforge-storage/src/commit.rs`
- **Commit:** `163b494`

**2. [Rule 3 - Blocking] Test definition had no way to create a `Stand`**
- **Found during:** Task 2 (RED)
- **Issue:** `Stand` had only its key `id`, and a `Set` needs at least one property, so the rebuild case could not edit a second object type.
- **Fix:** added an optional `label` string property to `Stand` in the conformance `DEFS`; extracted the existing all-types `Set` into `all_values()` so two cases share it. No existing case changed behaviour.
- **Files modified:** `crates/ontoforge-storage/tests/conformance/mod.rs`
- **Commit:** `28d7ba8`

### Plan interpretation

- A4 did not need the fallback; `store.rs` and `sql_exit.rs` are unchanged.
- TRUNCATE cases: PostgreSQL checks the foreign key from `of_edit_log_entry` before firing the trigger when `of_edit_log` is truncated alone, so the trigger is proven through the entry and history tables alone and through all three in one statement. Truncating `of_edit_log` alone is also refused, but by the foreign key.
- `replay` returns `EditError::DefinitionStale` when a record's `def_version` differs from the target's active one (the plan fixes the interface at `Diverged | Edit | Store`; this reuses `Edit`).

**Total deviations:** 2 auto-fixed (both Rule 3), 0 architectural. **Impact:** none on the plan's scope or interfaces.

## Known Stubs

None. Replay of `unset`, `tombstone`, `release` and link `remove` entries resolves them but the transitions return `InvalidTransition` until 03-13 implements them in `next_state` (the planned boundary from 03-08); replay shares that code and needs no change.

## Open items for later plans

- 03-13: after unset / tombstone / release / link remove exist, add replay conformance for them (the rebuild case today covers set and link add).
- 03-15: cross-version replay via `migration_json`.
- 03-16: `of_admin_audit` should create its triggers with `of_reject_mutation()` (PostgreSQL) and the SQLite equivalent.
- 03-20: document the deployment REVOKE (application role not the owner of the log tables; `REVOKE UPDATE, DELETE, TRUNCATE`), as ADR-0026 requires.

## Threat Flags

None beyond the plan's register. T-03-24 is mitigated by the triggers on both backends (conformance proves UPDATE, DELETE, TRUNCATE and replace-style writes are refused; the owner-level bypass is covered by the REVOKE requirement in ADR-0026). T-03-73 is mitigated by the `version_after` check in `replay` and the column-for-column rebuild case.

## Self-Check: PASSED

- Created and modified files exist: ADR-0026, both migrations, `log.rs`, `commit.rs`, `tests/conformance/mod.rs` (checked with `git ls-files`).
- Commits `f96319e`, `511a672`, `28d7ba8`, `163b494`, `00b213d` are in `git log`; `git rev-list --count 4d5b17f..HEAD` was 5 before this SUMMARY.
