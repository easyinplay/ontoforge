---
phase: 03-edit-store-scope-model
plan: 03
subsystem: storage
tags: [sqlx, sea-query, sqlite, ddl, reload, edit-store, adr, source-guard]

requires:
  - phase: 03-edit-store-scope-model
    provides: "03-01 storage crate manifest, sqlite_connect_options, engine collations; 03-02 [store] config, ONT-V21..V27, PropertyDef.indexed"
provides:
  - "ontoforge_storage::reload::reload: derive physical schema, diff, apply additive DDL, record a definition version (D-15)"
  - "EditStore::open on SQLite (system migration, active definition cache); PostgreSQL refused with StoreError::Config until 03-04"
  - "PhysicalSchema / ObjectTable / LinkTable / ColumnType, serializable into of_def_version.physical_json"
  - "diff() with DestructiveChange -> ONT-V25 (help points at `ontoforge migrate --plan`)"
  - "sql_exit.rs as the only AssertSqlSafe site, enforced by tests/sql_exit_guard.rs"
  - "ADR-0024 edit store physical format (option-a: one <col>__set boolean column per non-pk property)"
affects: [03-04, 03-08, 03-10, 03-15, 03-18, phase-04-query]

plan_head_before: 2b23a05df3be39e600e6d5c7856962a4b74a1671

actuals:
  tokens: 19000
  tasks: 3
  commits: 4

tech-stack:
  added: []
  patterns:
    - "All SQL text goes through src/sql_exit.rs; DDL and DML come from sea-query, identifiers are quoted by it"
    - "reload steps are re-entrant: CREATE ... IF NOT EXISTS, then table_info introspection and ADD COLUMN only for missing columns"
    - "Definition equality for the no-op check compares serde_json::Value of the ontology (mapping key order ignored)"

key-files:
  created:
    - crates/ontoforge-storage/src/error.rs
    - crates/ontoforge-storage/src/naming.rs
    - crates/ontoforge-storage/src/physical.rs
    - crates/ontoforge-storage/src/ddl.rs
    - crates/ontoforge-storage/src/diff.rs
    - crates/ontoforge-storage/src/reload.rs
    - crates/ontoforge-storage/src/sql_exit.rs
    - crates/ontoforge-storage/migrations/sqlite/0001_definitions.sql
    - crates/ontoforge-storage/tests/support/mod.rs
    - crates/ontoforge-storage/tests/reload.rs
    - crates/ontoforge-storage/tests/sql_exit_guard.rs
    - .planning/adr/0024-edit-store-physical-format.md
  modified:
    - crates/ontoforge-storage/src/lib.rs
    - crates/ontoforge-storage/src/store.rs

key-decisions:
  - "set_mask is one `<col>__set` boolean column per non-pk property (option-a, ADR-0024); no `set_mask bigint`, no `of_property_slot` registry"
  - "A new required property, a changed column type (byte vs short distinct, decimal precision), a changed pk, a removed property or type, and changed link ends are all destructive (ONT-V25)"
  - "SQLite of_version / of_deleted / of_released / of_def_version carry DEFAULT 0 so reload can repair a table that lacks them with ADD COLUMN"
  - "Physical-name overlength and collision diagnostics use ONT-V24 (03-02 numbering), not RESEARCH E18/E19"

patterns-established:
  - "Guard test pattern for storage: keyword-literal, AssertSqlSafe and OR REPLACE / REPLACE INTO scans with a self-check sample"

# STORE-02 is demonstrated on SQLite only (additive DDL, destructive refusal, idempotency); the PostgreSQL
# dialect, the CLI, migrate and the visibility fence land in 03-04 / 03-15 / 03-18. STORE-01 needs the PostgreSQL
# backend and conformance tests. REQUIREMENTS.md is left unticked on purpose (policy decided in 03-01).
requirements-completed: []
requirements-addressed: [STORE-02, STORE-01]

coverage:
  - id: D1
    description: "reload creates of_obj_<snake> and of_lnk_<snake> tables with <col>__set flags, of_ system columns and a recorded definition version"
    requirement: STORE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload.rs#first_reload_creates_tables"
        status: pass
    human_judgment: false
  - id: D2
    description: "A second reload of the same definition (also after reopening the store) runs no DDL and keeps the definition version"
    requirement: STORE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload.rs#second_reload_is_noop"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload.rs#reload_after_reopen_is_noop"
        status: pass
    human_judgment: false
  - id: D3
    description: "A new optional property becomes exactly two ADD COLUMN statements and definition version 2; a metadata-only change records a version without DDL"
    requirement: STORE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload.rs#additive_property_adds_columns"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload.rs#metadata_only_change_records_a_version_without_ddl"
        status: pass
    human_judgment: false
  - id: D4
    description: "Removed property, changed type, new required property, removed type and changed pk are rejected with ONT-V25 and leave tables and the active version untouched"
    requirement: STORE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload.rs#destructive_changes_are_rejected"
        status: pass
    human_judgment: false
  - id: D5
    description: "dry_run returns the DDL and creates no table and no of_def_version row"
    requirement: STORE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload.rs#dry_run_changes_nothing"
        status: pass
    human_judgment: false
  - id: D6
    description: "A table left half created by an interrupted run is completed without repeating columns"
    requirement: STORE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload.rs#reload_resumes_after_partial_ddl"
        status: pass
    human_judgment: false
  - id: D7
    description: "No source file except sql_exit.rs writes a SQL keyword literal or AssertSqlSafe; no OR REPLACE / REPLACE INTO anywhere in src"
    requirement: STORE-01
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/sql_exit_guard.rs#sql_text_only_from_sql_exit"
        status: pass
    human_judgment: false
  - id: D8
    description: "ADR-0024 records the physical format decision, the dialect type table and the PG schema split"
    verification: []
    human_judgment: true
    rationale: "Document content is a design record; a reviewer judges wording and completeness"

duration: about 1h30 (continuation after the Task 1 checkpoint)
completed: 2026-10-20
status: complete
---

# Phase 3 Plan 3: Edit store physical format and SQLite reload tracer Summary

**`reload` is now a library function that turns an ontology into strongly typed SQLite edit tables (`of_obj_*`, `of_lnk_*`, one `<col>__set` boolean per non-key property), applies additive changes as re-entrant DDL in one transaction with a recorded definition version, and refuses destructive changes with ONT-V25 before any statement runs; the format is fixed in ADR-0024 and SQL text is confined to `sql_exit.rs` by a source guard.**

## Performance

- **Duration:** about 1h30 for Tasks 2 and 3; Task 1 was a decision checkpoint resolved by the orchestrator
- **Completed:** 2026-10-20
- **Tasks:** 3 of 3
- **Files:** 14 changed (12 created, 2 modified)
- **Verification:** `cargo test --workspace --locked` 636 passed / 0 failed; `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo fmt --all --check` clean

## Accomplishments

- `EditStore::open` (SQLite) runs the `0001_definitions` system migration and reads the active definition back (ontology and physical schema deserialize equal, so a reload after reopen is a no-op).
- `PhysicalSchema::derive` maps every `BaseType` one to one onto `ColumnType` (byte and short stay distinct, decimal keeps precision and scale, arrays nest) and builds M:N link tables `(a_pk, b_pk, of_deleted, of_version, of_def_version, of_log_seq)`.
- `diff` splits changes into additive steps and `DestructiveChange` values; `DestructiveChange::diagnostic` gives a location-free ONT-V25 with the `ontoforge migrate --plan` help.
- `reload` handles dry run, unchanged, additive, destructive and half-created tables; PostgreSQL arms return `StoreError::Config`, never panic.
- `tests/sql_exit_guard.rs` caught a keyword literal in a `ddl.rs` unit test on its first run, which proved the scan works; the literal was reworded rather than the scan loosened.

## Task Commits

| Task | Commit | What |
|------|--------|------|
| 1 | n/a | checkpoint:decision resolved by the orchestrator: option-a |
| 2 RED | `0760fbc` | `test(03-03)`: failing reload tracer tests plus store, naming, physical, sql_exit scaffolding; `diff` / `ddl` / `reload` stubs |
| 2 GREEN | `8a28f22` | `feat(03-03)`: diff, SQLite DDL rendering, reload |
| 3 test | `b94bb70` | `test(03-03)`: SQL exit source guard |
| 3 docs | `0e977ab` | `docs(03-03)`: ADR-0024 |

## TDD Gate Compliance

RED commit `0760fbc` precedes GREEN commit `8a28f22`. In the RED run all 8 tests in `tests/reload.rs` failed on the stub (`Store(Config("reload is not implemented yet"))` at the first `reload` call or at the assertion that expects a definition error); no test failed on a compile or load error. `gsd_run check tdd-red-evidence` was not run: it parses node TAP output and `cargo test` does not emit it, so the RED run is evidenced by the failing-test list above instead.

## Deviations from Plan

### Auto-fixed / adjusted

**1. [Checkpoint resolution] `of_property_slot` and `set_mask bigint` dropped**
- **Found during:** Task 2 action step 1 and the Task 3 ADR text, which describe both options
- **Fix:** implemented option-a only; ADR-0024 lists option-b under Rejected with the reasons given by the orchestrator
- **Files:** `.planning/adr/0024-edit-store-physical-format.md`, `crates/ontoforge-storage/src/{physical,ddl}.rs`

**2. [Rule 3 - Blocking] SQLite `of_def_version` (and the other flag columns) carry `DEFAULT 0`**
- **Found during:** Task 2, `reload_resumes_after_partial_ddl`
- **Issue:** the plan declares `of_def_version INTEGER NOT NULL`; SQLite refuses `ADD COLUMN` for a `NOT NULL` column without a default, so a table lacking it could not be repaired
- **Fix:** `DEFAULT 0` on the SQLite `NOT NULL` system columns; writers always set `of_def_version`. Recorded in ADR-0024 point 7. 03-04 decides whether PG needs the same.

**3. [Rule 1 - plan test adjusted] Partial-DDL fixture also pre-creates the key column**
- **Issue:** the plan says the leftover table "already has a `name` column"; a primary key column cannot be added to an existing SQLite table, and a half-created table without its key is not repairable
- **Fix:** the test pre-creates `code` (pk) and `name`; `apply_step` returns `StoreError::Corrupt` when a key column is missing. In SQLite DDL is transactional, so this state arises only from a manual change.

**4. [Plan test count] Primary key change reports two diagnostics**
- **Issue:** `destructive_changes_are_rejected` assumed one diagnostic per case. Changing the pk from `code` to `name` yields `PrimaryKeyChanged` plus `PropertyRemoved(name)` (the new pk is no longer a plain property).
- **Fix:** the case asserts 2; all codes are ONT-V25.

**5. [Layout] System-table helpers live in `sql_exit.rs`**
- `next_def_version`, `supersede_active`, `activate_definition` and `read_active_definition` build their statements with sea-query inside `sql_exit.rs`, so `reload.rs` and `store.rs` contain no SQL at all. The plan lists only execute helpers there; this keeps the guard simple.

**Total deviations:** 5 (none touched files outside the plan's list or any Cargo.toml).

## Auth Gates

None.

## Known Stubs

None. The PostgreSQL arms (`Db::Postgres`, `WriteTx::Postgres`, `Backend::Postgres` in `ddl::render`) are intentional placeholders resolved by 03-04; `EditStore::open` refuses a PostgreSQL store with a clear error, so they are unreachable.

## Threat Flags

None. No new network endpoint, auth path or trust boundary beyond the plan's threat model (T-03-07 source guard, T-03-08 destructive refusal, T-03-10 re-entrant DDL are all covered by tests above).

## Issues Encountered

- `sea-query` 1.0 needs `ExprTrait` in scope for `Expr::eq`; without it `.eq` resolves to `PartialEq`.
- `sea-query`'s `ColumnDef` does not expose its name or key flag, so `ddl.rs` wraps each in a `Col { name, key, def }` for the introspection-based repair.

## Next Phase Readiness

03-04 can add the PostgreSQL dialect to `ddl::render`, the PG branch of `sql_exit`, the `ontoforge reload` CLI and the PG system migration against ADR-0024. 03-10 adds length caps, ONT-V24 collisions and indexes on top of `naming.rs` and `physical.rs`. 03-18 adds the reload lock and the pending to active switch around `reload`'s transaction.

## Self-Check: PASSED

Files found: all 12 created paths above; commits `0760fbc`, `8a28f22`, `b94bb70`, `0e977ab` present in `git log`.
