---
phase: 03-edit-store-scope-model
plan: 10
subsystem: storage
tags: [reload, indexes, naming, cardinality, sqlite, postgres, sea-query, tdd]
status: complete

requires:
  - phase: 03-edit-store-scope-model
    provides: "03-03 / 03-04 naming, PhysicalSchema, DdlStep, diff, reload, reload tests; 03-08 edit log system tables (reload_pg set asserts them)"
provides:
  - "ObjectTable.indexes (IndexDef, IndexKind): of_ix_<table>_<col> for `indexed: true`, partial unique of_ux_<table>_<col> on the oneToOne foreign key of the `to` end"
  - "DdlStep::CreateIndex / DropIndex on both dialects; the unique index is partial (WHERE <col>__set AND NOT of_deleted)"
  - "naming::{bounded, index_name, unique_index_name, TABLE_BASE_MAX, COLUMN_BASE_MAX, IDENT_MAX, OBJECT_SYSTEM_COLUMNS, LINK_SYSTEM_COLUMNS}; FNV-1a 32 bit truncation"
  - "ONT-V24 from PhysicalSchema::derive for shared table, column or index names and for properties equal to a system column"
  - "Index reconciliation in reload: additive, plain indexes never dropped, unique index dropped when its link relaxes, tightening over conflicting data fails and rolls back"
affects: [03-13, 03-15, 03-16, 03-18]

plan_head_before: 6616b7671d75f1419ea63c1204fd7ca81276df81

actuals:
  tokens: 15200
  tasks: 2
  commits: 4

tech-stack:
  added: []
  patterns:
    - "Index DDL is built with sea-query Index::create().if_not_exists().and_where(..); PostgreSQL keeps the index name unqualified (same schema as its table) and DROP INDEX is schema qualified"
    - "A plain index that leaves the definition stays in the stored PhysicalSchema (PhysicalSchema::keep_plain_indexes_of), so the physical record keeps describing the database and reload stays idempotent"

key-files:
  created: []
  modified:
    - crates/ontoforge-storage/src/naming.rs
    - crates/ontoforge-storage/src/physical.rs
    - crates/ontoforge-storage/src/ddl.rs
    - crates/ontoforge-storage/src/diff.rs
    - crates/ontoforge-storage/src/reload.rs
    - crates/ontoforge-storage/src/snapshots/ontoforge_storage__ddl__tests__ddl_sqlite.snap
    - crates/ontoforge-storage/src/snapshots/ontoforge_storage__ddl__tests__ddl_postgres.snap
    - crates/ontoforge-storage/tests/reload.rs
    - crates/ontoforge-storage/tests/reload_pg.rs

key-decisions:
  - "Plain indexes removed from the definition are kept in the stored physical schema instead of being re-derived away: otherwise `active.physical == physical` would be false on every later reload and each one would bump def_version without DDL"
  - "Index names are checked for collisions across the whole edit schema, not per table: PostgreSQL index names share the schema namespace, and truncation can fold two tables' index names together"
  - "A oneToOne foreign key that is the primary key gets no unique index (already unique); a plain index and a unique index on the same column are both created, as the plan states"
  - "A unique violation while creating a unique index (SQLSTATE 23505, SQLite 2067 / `UNIQUE constraint failed`) is rewritten into a StoreError::Query naming the link and `existing edits violate the oneToOne cardinality`; the CLI already maps non-config StoreError to ONT-V26"

requirements-completed: []
requirements-addressed: [STORE-02, STORE-05]
# STORE-02: this plan adds index reconciliation and name checks; the pending/active two-step switch, the Trino probe and
# the reload lock come with 03-18. STORE-05: only the database side (partial unique index) is delivered; the commit-path
# error mapping to CardinalityViolation and its concurrency test are 03-13. REQUIREMENTS.md stays unticked (policy of 03-01).

coverage:
  - id: D1
    description: "reload creates `indexed` indexes and the oneToOne partial unique index on SQLite and PostgreSQL"
    requirement: STORE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload.rs#aviation_reload_creates_indexes"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload_pg.rs#indexes_on_pg"
        status: pass
      - kind: snapshot
        ref: "crates/ontoforge-storage/src/ddl.rs#ddl_snapshots"
        status: pass
    human_judgment: false
  - id: D2
    description: "The database refuses a second live row with the same oneToOne key and accepts a deleted or unset one"
    requirement: STORE-05
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload.rs#one_to_one_unique_index_rejects_second_row"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload_pg.rs#indexes_on_pg"
        status: pass
    human_judgment: false
  - id: D3
    description: "Index reconciliation is additive: new index on second reload, plain index kept when `indexed` is removed, unique index dropped when the link relaxes"
    requirement: STORE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload.rs#index_added_on_second_reload"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload.rs#removing_indexed_keeps_index"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload.rs#relaxing_one_to_one_drops_unique_index"
        status: pass
    human_judgment: false
  - id: D4
    description: "Tightening to oneToOne over duplicate data fails, keeps the old definition and leaves no unique index"
    requirement: STORE-05
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload.rs#tightening_to_one_to_one_with_conflicts_keeps_old_definition"
        status: pass
    human_judgment: false
  - id: D5
    description: "Names above PostgreSQL's limit are cut with an FNV-1a hash; colliding or system-column-equal names are ONT-V24 before any DDL"
    requirement: STORE-02
    verification:
      - kind: unit
        ref: "crates/ontoforge-storage/src/naming.rs#bounded_names"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload.rs#snake_collision_is_v24"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload.rs#system_column_collision_is_v24"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload.rs#long_names_round_trip"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload_pg.rs#long_names_on_pg"
        status: pass
    human_judgment: false
  - id: D6
    description: "Link and precision changes are destructive (ONT-V25); a new many-to-many link creates its table; archived columns are not removals"
    requirement: STORE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload.rs#link_changes_are_destructive"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload.rs#precision_and_items_changes_are_destructive"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload.rs#new_many_to_many_link_creates_a_table"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-storage/src/diff.rs#archived_columns_are_not_removals"
        status: pass
    human_judgment: false

duration: "~1h (start time not captured)"
completed: 2026-10-20
---

# Phase 3 Plan 10: Reload indexes, physical name limits and cardinality enforcement Summary

**reload now builds `of_ix_` indexes for `indexed: true` and a partial unique `of_ux_` index on every oneToOne foreign key (so the database enforces 1:1), cuts physical names to PostgreSQL's 63 byte limit with an FNV-1a hash, and stops colliding names with ONT-V24 before any DDL.**

## Accomplishments

- `PhysicalSchema::derive` adds `ObjectTable.indexes` (serde default, old `physical_json` still reads): a `Plain` index per `indexed: true` non-key property and a `UniqueWhenSet` index on the `to` end's foreign key column of each oneToOne link. Indexes are sorted by name so equality and the stored JSON are deterministic.
- `ddl.rs` renders `CreateIndex` and `DropIndex` for both dialects. The unique index is `WHERE "<col>__set" = 1 AND "of_deleted" = 0` on SQLite and `WHERE "<col>__set" AND (NOT "of_deleted")` on PostgreSQL, built with sea-query's `IndexCreateStatement::and_where` (the partial index API exists in 1.0.2, so no template fallback was needed). Verified live on embedded PostgreSQL: `pg_indexes` shows `UNIQUE ... WHERE`, a second live row raises SQLSTATE 23505, a deleted row and an unset row are accepted.
- `diff.rs` creates indexes together with a new table (table step, then its index steps), creates a missing index on an existing table after the column steps, drops only a unique index that left the definition, and never drops a plain index. `reload.rs` carries a removed plain index over into the new physical schema (`keep_plain_indexes_of`) so the next reload is a no-op.
- `naming.rs`: `bounded` keeps names within the cap or emits `prefix_ + 8 hex` of the FNV-1a hash of the full name; table snake part is capped at 29 bytes, column at 30, index names at 63. Existing short names are unchanged, so the DDL snapshots only gained index lines.
- `derive` reports ONT-V24 (help "rename one of them") for duplicate table names, duplicate column names per table (pk, value, `__set`, system columns, `a_pk` / `b_pk`) and duplicate index names across the schema. `ABTest` / `AbTest` and a property `ofVersion` are covered. A test in `ddl.rs` ties `naming::OBJECT_SYSTEM_COLUMNS` / `LINK_SYSTEM_COLUMNS` to the columns the DDL actually creates.
- Tightening a link to oneToOne over duplicate data: the failing `CREATE UNIQUE INDEX` rolls the whole reload back and surfaces as `StoreError::Query` with the link name and "existing edits violate the oneToOne cardinality"; the CLI maps it to ONT-V26 through its existing non-config branch.

## Task Commits

1. Task 1 (tracer, TDD) RED: `c315ac6` test(03-10): four index cases fail on the missing indexes.
2. Task 1 GREEN: `bf826e0` feat(03-10): indexes, DdlStep index steps, diff, naming helpers, snapshots.
3. Task 2 (TDD) RED: `4d63a77` test(03-10): `snake_collision_is_v24` and `system_column_collision_is_v24` fail (reload returned Ok and created the same table twice).
4. Task 2 GREEN: `71f3944` feat(03-10): name collision detection, diff unit tests, PG tests.

Tracer gate: `HUMAN_VERIFY_MODE` is `end-of-phase` and the tracer `<verify>` is automated only, so it was re-run end to end after GREEN (`--test reload`, `--lib`) and passed before the Task 2 work began.

## Verification

- `cargo test --workspace --locked`: 794 passed, 0 failed, 31 ignored (70 result lines, all ok).
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: clean. `cargo fmt --all --check`: clean. No `.snap.new` under `crates/ontoforge-storage/src/snapshots`.
- PostgreSQL (embedded server, Windows 11, no Docker): `cargo test -p ontoforge-storage --features embedded-pg --test reload_pg -- --ignored` 5 passed; `--test conformance_pg -- --ignored` 11 passed. `reload_pg -- --ignored --list` lists 5 tests. `tests/reload_pg.rs` still asserts the three edit log system tables in the exact system-table set.
- Acceptance greps: `snake_collision_is_v24` and `tightening_to_one_to_one_with_conflicts_keeps_old_definition` each `1 passed`; `0x811c9dc5` / `0x01000193` appear on 2 lines of `naming.rs`; `CREATE UNIQUE INDEX IF NOT EXISTS` is in both snapshots; `pub struct IndexDef` is in `physical.rs`.

## TDD Gate Compliance

Both tasks have a `test(03-10)` commit before the `feat(03-10)` commit and no refactor commit. Honest notes on RED strength: Task 1 RED failed on all four index cases (no index existed). Task 2 RED had two real failures (the V24 cases); the other Task 2 cases (long names, link and precision changes, tightening, relaxing) already passed at RED because Task 1 GREEN had implemented `bounded`, the unique index and the link/type comparisons that the plan's own interface block puts in Task 1, and `ColumnType` equality already covered scale and array items. They are kept as regression cover. `gsd_run check tdd-red-evidence` reads node TAP output and `cargo test` does not produce it, so no evidence record was generated; the failing test names above are the evidence.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical functionality] Plain index retention in the stored physical schema**
- **Found during:** Task 1 (`removing_indexed_keeps_index`)
- **Issue:** The plan says a plain index that disappears from the definition produces no step and is "kept in the physical schema". `derive` only sees the new definition, so without carrying the index over, `active.physical == physical` is false on every later reload and each reload records a new definition version with no DDL.
- **Fix:** `PhysicalSchema::keep_plain_indexes_of(&active.physical)` runs in `reload` right after `derive`, keeping plain indexes whose column is still live. A unique index is not kept.
- **Files modified:** `physical.rs`, `reload.rs`
- **Commit:** `bf826e0`

**2. [Rule 2 - Missing critical functionality] Index names checked across the schema, not per table**
- **Found during:** Task 2
- **Issue:** The plan asks for per-table uniqueness of index names. PostgreSQL and SQLite index names live in a schema-wide namespace, and truncation can map two tables' index names to one string, so a per-table check would let `CREATE INDEX IF NOT EXISTS` silently skip the second index.
- **Fix:** One schema-wide uniqueness check over all index names.
- **Files modified:** `physical.rs`
- **Commit:** `71f3944`

**Total deviations:** 2 auto-fixed (2 missing critical functionality). **Impact:** no scope change; both keep reload idempotent and DDL correct.

### Notes (not deviations)

- `ddl.rs` needed `sea_query::ExprTrait` for `.and()` / `.not()` / `.eq()` on `Expr` in sea-query 1.0.2.
- Clippy's `unreadable_literal` forced the FNV constants into two `#[allow(.., reason = ..)]` constants written as `0x811c9dc5` / `0x01000193` so the acceptance grep matches.
- The new tests use `side_pool` / `side()` already in `tests/support`, so `tests/support/mod.rs` (not in this plan's file list) was not touched.
- `reload_pg.rs` now has 5 ignored tests (3 existing plus `indexes_on_pg`, `long_names_on_pg`), meeting the `>= 5` criterion.

## Known Stubs

None.

## Threat Flags

None. T-03-28 (name collision) is mitigated by the ONT-V24 check with tests for `ABTest` / `AbTest` and `ofVersion`; T-03-29 (1:1 broken by concurrent writes) has its database half in the partial unique index, proven on SQLite and PostgreSQL, with the commit-path mapping and concurrency test left to 03-13 as planned; T-03-30 remains accepted.

## Self-Check: PASSED

- Files modified exist: `naming.rs`, `physical.rs`, `ddl.rs`, `diff.rs`, `reload.rs`, both snapshots, `tests/reload.rs`, `tests/reload_pg.rs` (all present in the commit diffs `6616b76..71f3944`).
- Commits `c315ac6`, `bf826e0`, `4d63a77`, `71f3944` exist in `git log`.
- `git rev-list --count 6616b76..HEAD` before this SUMMARY commit: 4, matches `actuals.commits`.
