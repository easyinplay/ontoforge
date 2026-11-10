---
phase: 03-edit-store-scope-model
plan: 08
subsystem: storage
tags: [edit-log, commit, conformance, sqlite, postgres, sea-query, tdd]
status: complete

requires:
  - phase: 03-edit-store-scope-model
    provides: "03-03 EditStore, WriteTx, PhysicalSchema, sql_exit; 03-04 PostgreSQL backend, support::pg_store(), embedded-pg"
provides:
  - "EditStore::commit: one transaction locks or creates edit rows, computes the new state, then allocates seq, writes rows, log entries and history snapshots (STORE-03)"
  - "PropertyValue with canonical form and the single JSON encoding of the edit log (value.rs)"
  - "State machine entry points next_state (Set on absent or live rows) and next_link_state (Add)"
  - "System tables of_edit_log, of_edit_log_entry, of_edit_history on both backends (migrations 0002_edit_log.sql, no triggers yet)"
  - "EditStore::read_log(after_seq, limit)"
  - "tests/conformance framework: all_cases! expands each case to a SQLite test and an ignored PostgreSQL test (STORE-01)"
affects: [03-09, 03-10, 03-13, 03-15, 03-17, 03-18, 03-19, 03-20]

plan_head_before: aaf4cf8f0e1fccb2962d7ab507212f4823c4af7b

actuals:
  tokens: 30400
  tasks: 2
  commits: 2

tech-stack:
  added: []
  patterns:
    - "Typed edit-table I/O goes through sql_exit: CellKind / Cell decode a row without driver types, typed_text casts text-borne values on PostgreSQL, fetch_all_as decodes tuples on both backends"
    - "Rows are locked in RowKey order (kind, name, pk JSON) in every commit; seq is allocated only after all row locks"
    - "Conformance cases are plain async fns over a Harness; a new case is a function plus one line in all_cases!"

key-files:
  created:
    - crates/ontoforge-storage/src/value.rs
    - crates/ontoforge-storage/src/state.rs
    - crates/ontoforge-storage/src/commit.rs
    - crates/ontoforge-storage/src/log.rs
    - crates/ontoforge-storage/src/history.rs
    - crates/ontoforge-storage/migrations/sqlite/0002_edit_log.sql
    - crates/ontoforge-storage/migrations/postgres/0002_edit_log.sql
    - crates/ontoforge-storage/tests/conformance/mod.rs
    - crates/ontoforge-storage/tests/conformance_sqlite.rs
    - crates/ontoforge-storage/tests/conformance_pg.rs
  modified:
    - crates/ontoforge-storage/src/lib.rs
    - crates/ontoforge-storage/src/sql_exit.rs
    - crates/ontoforge-storage/tests/support/mod.rs
    - crates/ontoforge-storage/tests/reload_pg.rs

key-decisions:
  - "Edit log physical format: the plan's interfaces layout plus supplements S1-S4 (below), fixed by the Task 1 checkpoint (option-a) and written first by this plan"
  - "A float value is rounded to the nearest f32 once, in PropertyValue::canonical, so the log, the history and both databases hold the same number"
  - "A property may be set to null unless it is required (null masks the source value, __set stays true); an empty Set and a Principal actor without `iss|sub` are refused"
  - "On PostgreSQL every array is bound as text[] and cast to its element type; on SQLite arrays and geopoints are JSON text"

requirements-completed: []
requirements-addressed: [STORE-03, STORE-01, STORE-07]
# STORE-03 says "immutable" and the append-only triggers come with 03-09; STORE-01 grows with every later conformance
# case; STORE-07 needs the full transition table and the reference model proptest in 03-13. REQUIREMENTS.md stays
# unticked (policy decided in 03-01).

coverage:
  - id: D1
    description: "A set commit creates the edit row (values, __set flags, of_version, of_def_version, of_log_seq) and returns seq and versions"
    requirement: STORE-03
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/conformance_sqlite.rs#commit_set_creates_edit_row"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/conformance_pg.rs#commit_set_creates_edit_row"
        status: pass
    human_judgment: false
  - id: D2
    description: "One commit appends one log record with actor, action, def_version, params, ULID event_id, UTC millisecond committed_at and one entry per edit holding new values only"
    requirement: STORE-03
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/conformance_sqlite.rs#commit_appends_one_log_record"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/conformance_pg.rs#commit_appends_one_log_record"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/conformance_sqlite.rs#two_edits_of_one_object_bump_each_version"
        status: pass
    human_judgment: false
  - id: D3
    description: "A history snapshot per entry, including after a second commit that reads every stored value type back from the database"
    requirement: STORE-03
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/conformance_sqlite.rs#commit_writes_history_snapshot"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/conformance_sqlite.rs#every_value_type_round_trips"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/conformance_pg.rs#every_value_type_round_trips"
        status: pass
    human_judgment: false
  - id: D4
    description: "Set patches only the given properties; invalid values, a stale def_version, an empty commit and a bad actor leave no log record, no history row and no edit row"
    requirement: STORE-07
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/conformance_sqlite.rs#second_set_patches_only_given_properties"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/conformance_sqlite.rs#invalid_values_are_rejected_without_writes"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/conformance_sqlite.rs#stale_def_version_rejected"
        status: pass
    human_judgment: false
  - id: D5
    description: "The same eleven conformance cases pass on SQLite and on PostgreSQL (embedded server)"
    requirement: STORE-01
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-storage --features embedded-pg --test conformance_pg -- --ignored"
        status: pass
    human_judgment: false
  - id: D6
    description: "The edit log format is fixed in the Task 1 checkpoint and recorded here for ADR-0026 (03-09)"
    verification: []
    human_judgment: true
    rationale: "A one-way design record; the ADR text is written by 03-09 and judged by a reviewer"

duration: "~1h (continuation run; start time not captured)"
completed: 2026-10-20
---

# Phase 3 Plan 08: Commit write path and edit log Summary

**One transaction per commit writes the typed edit rows, one `of_edit_log` record with its entries and one history snapshot per entry, on SQLite and PostgreSQL, proven by 11 conformance cases that run unchanged on both.**

## Accomplishments

- `EditStore::commit` validates without a transaction (non-empty edits, actor shape, `def_version` against the cached active definition, types, properties, canonical values, pk not in Set), then in one transaction inserts a version 0 skeleton row (`ON CONFLICT DO NOTHING`), reads it `FOR UPDATE`, applies `next_state`, inserts the log header with `RETURNING seq`, updates the rows, and inserts entries and snapshots. Any error rolls back, including `MissingRequired` raised after the skeleton insert (asserted: no gate row remains).
- `PropertyValue` (value.rs) is the only encoding of values in log, history and typed columns. `canonical` pads decimals to the property scale without rounding, validates `YYYY-MM-DD`, converts timestamps to UTC milliseconds (offset required, more than 3 fractional digits refused), range-checks byte/short/integer, geopoint, array elements.
- Conformance framework: `all_cases!(factory[, ignore = ".."])` in `tests/conformance/mod.rs`; `conformance_sqlite.rs` runs it, `conformance_pg.rs` runs the same list `#[ignore]`d. A `Harness` enum wraps the SQLite or PG test store and offers raw `rows()` reads (every cell as text) for the system and edit tables.
- Beyond the six plan cases, five more: two edits of one object in one commit (versions 1, 2), every value type round trip (this is the case that proves the typed read-back on both backends), `system:import` actor, `read_log` paging, link add.

## Edit log format (input for ADR-0026, written by 03-09)

Tables (`migrations/<backend>/0002_edit_log.sql`; PG types in brackets):

- `of_edit_log(seq` INTEGER PK AUTOINCREMENT [`bigint GENERATED ALWAYS AS IDENTITY`], `event_id` text unique, `actor`, `action`, `committed_at` text [`timestamptz`], `def_version`, `params` JSON text [`jsonb`])
- `of_edit_log_entry(log_seq` FK, `ordinal`, `kind` CHECK object/link, `type_name`, `pk` JSON (object: pk scalar; link: `[a, b]`), `op` CHECK set/unset/tombstone/release/link_add/link_remove, `payload` JSON NULL (set: property to new value; unset: list of names; else null), `version_after`; PK `(log_seq, ordinal))`
- `of_edit_history(log_seq, ordinal, kind, type_name, pk, version, snapshot)`; index `of_ix_edit_history_object (type_name, pk, version)`. Object snapshot `{"deleted","released","set":{prop:value}}`, link snapshot `{"deleted"}` (S4). No foreign key from history to entry (as in the approved layout).
- Actors: `Principal("{iss}|{sub}")` or `system:import` (no `|`, so no collision). Params stored verbatim (T-03-27 accepted).

Supplements fixed by the checkpoint:

- **S1** A timestamp is `YYYY-MM-DDTHH:MM:SS.fffZ` in log, history and pk. The SQLite edit column stores `YYYY-MM-DD HH:MM:SS.fffZ` (space, because `timestamp_key` in `ontoforge-engine/src/sqlite/mod.rs` parses `%Y-%m-%d %H:%M:%S` and silently falls back to text order on `T`). PG uses `timestamptz`. All are rendered from one canonical `PropertyValue::Timestamp`. Offset required, converted to UTC, more than 3 fractional digits rejected.
- **S2** Conformance and replay compare JSON columns (`params`, `payload`, `snapshot`) as parsed `serde_json::Value`, never as text (PG `jsonb` reorders keys; the workspace `serde_json` has `preserve_order`). The conformance cases do this (`json_cell`, `Value` equality).
- **S3** `committed_at` is generated in Rust (`Utc::now()`, `SecondsFormat::Millis`); `read_log` returns the same text on both backends (PG: `to_char(.. AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"')`).
- **S4** Link entry snapshot is `{"deleted": bool}`; object entry snapshot is `{"deleted","released","set"}`.

Research facts honoured: `sea-query-sqlx` 0.9.1 `SqlxBinder::build_sqlx` (as in sql_exit); `sea-query` 1.0.2 `returning_col`, `lock(LockType::Update)` (a no-op on the SQLite builder, so it is added unconditionally), `OnConflict::column(..).do_nothing()`; the SQLite binder panics on `Value::Array`, so SQLite arrays and geopoints are bound as JSON text; no `rust_decimal`/`bigdecimal` features, so PG decimal, date and timestamp values are bound as text with `CAST(.. AS numeric(p, s))` etc.

Ordering note for 03-09 and any tailing consumer: on PostgreSQL `seq` is allocated after the row locks, so per-pk replay order equals version order. A live tail through `read_log(after_seq)` can still observe a higher `seq` before a lower `seq` that belongs to a different pk (the identity sequence is not commit ordered). This is harmless for 03-09's offline replay, which sorts by `seq`; a live follower must not assume `seq` gaps are final.

## Task Commits

1. Task 1 (checkpoint:decision): resolved by the orchestrator, option-a plus S1-S4; no code.
2. Task 2 (tracer, TDD) RED: `f904bce` test(03-08): 11 conformance cases on stub bodies, all 11 fail on the missing behavior.
3. Task 2 GREEN: `8fa53d3` feat(03-08): commit write path, log, history, migrations, typed values.

Tracer gate: `HUMAN_VERIFY_MODE` is `end-of-phase` and `<verify>` is automated only, so it was re-run end to end after GREEN and passed (SQLite 11/11, PG 11/11); expansion is plan 03-09.

## Verification

- `cargo test --workspace --locked`: 777 passed, 0 failed, 29 ignored.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo clippy -p ontoforge-storage --all-targets --features embedded-pg --locked -- -D warnings`: clean. `cargo fmt --all --check`: clean.
- `conformance_sqlite`: 11 passed. `conformance_pg --features embedded-pg -- --ignored` (embedded server, no Docker, Windows 11): 11 passed in about 5-7 s; `reload_pg` 3 passed. `conformance_pg -- --ignored --list` lists the 11 ignored cases.
- `sql_exit_guard`: ok; `AssertSqlSafe` appears only in `sql_exit.rs`; `returning_col` is in `commit.rs`.

## TDD Gate Compliance

RED `f904bce` (test) precedes GREEN `8fa53d3` (feat); no refactor commit (the clippy and fmt fixes went into GREEN before it was committed). RED evidence: all 11 targets failed in `commit_ok`/assertions against the stub (`commit is not implemented`, empty log). `gsd_run check tdd-red-evidence` classifies node TAP output, which `cargo test` does not produce, so the record was not generated; the failing test list above is the evidence.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `reload_pg` asserted the exact set of system tables**
- **Found during:** Task 2, full PG run
- **Issue:** `first_reload_on_pg_puts_edit_tables_in_edit_schema` expected only `_sqlx_migrations`, `of_def_state`, `of_def_version` in `schema_sys`; migration 0002 adds the three log tables.
- **Fix:** the expected set now lists `of_edit_history`, `of_edit_log`, `of_edit_log_entry`. The file belongs to 03-10's `tests/reload*.rs`; this tree is sequential, so 03-10 will see the change.
- **Files modified:** `crates/ontoforge-storage/tests/reload_pg.rs`
- **Commit:** `8fa53d3`

**2. [Rule 3 - Blocking] Test support needed a PG side connection**
- **Issue:** the conformance harness inspects tables from outside the store, and `PgTestStore::side` was private.
- **Fix:** `PgTestStore::side()` accessor in `tests/support/mod.rs` (not in the plan's file list).
- **Commit:** `f904bce`

### Plan interpretation

- `PropertyValue::canonical` is added next to the interface's `check` (which now delegates to it), because commit must write the canonical value, not only validate it.
- `append_header` and its `NewRecord` live in `commit.rs` (the acceptance criterion greps `returning` in `commit.rs`); `append_entry` is in `log.rs`, `append_snapshot` in `history.rs`.
- PostgreSQL arrays are all bound as `text[]` and cast to the element type, not only decimal, date and timestamp arrays. One rule, no native-array variants; PG casts text to every element type.
- Reading a row back on PostgreSQL renders timestamps and timestamp arrays with `to_char(.. AT TIME ZONE 'UTC', ..)` so the result does not depend on the session time zone; `store.rs` is untouched.
- Extra validation not in the plan: empty `Set` refused, `Principal` must be `iss|sub`, empty action refused, a non-many-to-many link is `UnknownType`.

**Total deviations:** 2 auto-fixed (both Rule 3, both in test code), 0 architectural. **Impact:** none on the format or the plan's scope.

## Known Stubs

None. `next_state` returns `InvalidTransition` for unset, tombstone and release, and `next_link_state` for remove and for adding an already linked pair: this is the planned boundary (03-13 implements the full table and the reference model), not a stub; `update_row` rewrites columns for `Set` only, with a comment where 03-13 extends it.

## Open items for later plans

- 03-13: Unset/Tombstone/Release in `next_state` and the matching column rewrites in `commit.rs::update_row`; link Remove; referential checks for link endpoints (none today: a link may name pks that do not exist).
- 03-09: append-only triggers appended to `0002_edit_log.sql`, replay, ADR-0026 from the format section above.
- 03-18: the `def_version` check is against the in-process `active` cache only; the database-side fence is 03-18.
- No concurrent-commit test exists yet (two transactions on one pk); the lock order (RowKey) and `FOR UPDATE` are in place, the proof is not.

## Threat Flags

None beyond the plan's register: values reach SQL only as bound parameters or typed casts of bound text (T-03-26), the guard test still holds, and the whole commit is one transaction (T-03-25).

## Self-Check: PASSED

- Created files exist: all ten listed under key-files.created (checked with `git ls-files`).
- Commits `f904bce` and `8fa53d3` are in `git log`; `git rev-list --count aaf4cf8..HEAD` was 2 before this SUMMARY.
