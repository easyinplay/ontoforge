---
phase: 03-edit-store-scope-model
plan: 15
subsystem: storage
tags: [migrate, reload, archive, rename, plan-hash, cli, sqlite, postgres, sea-query, tdd]
status: complete

requires:
  - phase: 03-edit-store-scope-model
    provides: "03-03 / 03-04 reload, diff (DestructiveChange), PhysicalSchema.archived_columns / archived_tables; 03-08 commit (test data); 03-09 append-only edit log (migrate never touches it); 03-10 index retention (keep_plain_indexes_of); 03-12 CLI structure"
provides:
  - "ontoforge_storage::migrate::{RenameDecl, MigrationStep, MigrationPlan, MigrateError, plan_migration, apply_migration, plan_drop_archived, apply_drop_archived}"
  - "reload::{execute_steps, switch_definition} (pub(crate)), shared by reload and migrate; switch_definition writes of_def_version.migration_json"
  - "Archive naming <col>__arch_v<N>, <col>__set__arch_v<N>, of_obj_<x>__arch_v<N> (naming::archived_column / archived_set_column / archived_table) and naming::fnv1a64_hex"
  - "PhysicalSchema::keep_archived_of: reload sees archived columns and tables as part of the schema, so reload after migrate is a no-op"
  - "sql_exit::read_active_definition_in: the active definition read inside the write transaction"
  - "CLI: ontoforge migrate <DIR> --server-config <FILE> (--plan | --apply --plan-hash <HASH>) [--rename <DECL>]... [--drop-archived]"
affects: [03-16, 03-18, 03-20]

plan_head_before: df787b84ca6459051838e31b230528a5a511e732

actuals:
  tokens: 27000
  tasks: 2
  commits: 4

tech-stack:
  added: []
  patterns:
    - "A plan is made by applying rename declarations to a copy of the active physical schema, then running the same diff reload runs; additive output becomes Ddl steps, destructive output becomes archive steps"
    - "apply re-plans inside the write transaction (BEGIN IMMEDIATE on SQLite) from the definition read in that transaction and compares the FNV-1a 64 hash before any DDL"
    - "DDL for rename, drop column and drop table is built with sea-query (Table::alter().rename_column, drop_column, Table::rename, Table::drop); no SQL text outside sql_exit.rs"

key-files:
  created:
    - crates/ontoforge-storage/src/migrate.rs
    - crates/ontoforge-storage/tests/migrate.rs
    - crates/ontoforge-storage/tests/migrate_pg.rs
    - crates/ontoforge/src/migrate.rs
    - crates/ontoforge/tests/cli_migrate.rs
  modified:
    - crates/ontoforge-storage/src/lib.rs
    - crates/ontoforge-storage/src/ddl.rs
    - crates/ontoforge-storage/src/naming.rs
    - crates/ontoforge-storage/src/physical.rs
    - crates/ontoforge-storage/src/reload.rs
    - crates/ontoforge-storage/src/sql_exit.rs
    - crates/ontoforge/src/cli.rs
    - crates/ontoforge/src/lib.rs
    - crates/ontoforge/src/reload.rs

key-decisions:
  - "Indexes on a column are dropped when the column is archived (a Ddl DropIndex step before ArchiveColumn), not when the archive is cleaned up: index names are schema-global, so a kept index named of_ix_<table>_<col> would make CREATE INDEX IF NOT EXISTS on a re-added column of the same name a silent no-op. SQLite DROP COLUMN then never meets an index"
  - "RenameTable and RenameColumn rebuild the indexes of the moved table or column as explicit Ddl DropIndex / CreateIndex steps around the rename, because index names carry the table and column name; retained plain indexes survive a rename"
  - "A table whose primary key changed is archived and created again; every other destructive change and additive step of that table is dropped from the plan. A link table whose end key columns changed is archived and created again even when its ends did not change"
  - "A rename declaration whose old name is gone and whose new name exists is taken as applied already, so repeating a call plans nothing and the reviewed hash is refused with PlanChanged (STORE-02 idempotency); a declaration with neither name is ONT-V25"
  - "Rename declarations are refused when the type differs, when a property would become required, when the primary key differs, or when either name collides (T-03-48)"
  - "The plan hash covers fromDefVersion, renames, steps and the new definition as JSON with sorted keys, so it does not depend on the order of the definition files"
  - "MigrationStep carries the table and column names it acts on (not in the interface sketch) so a step is self-contained for execution and for the later cross-version replay that reads migration_json; plan, steps and declarations serialize with camelCase fields"
  - "migrate refuses to run before any definition is active (ONT-V25 pointing at reload); an empty plan whose hash matches is a no-op that writes no definition version"

patterns-established:
  - "execute_steps / switch_definition are the single DDL execution and definition switch path; 03-18 adds lock_timeout, advisory lock, fence and the Trino probe there and they cover migrate as well"

requirements-completed: []
requirements-addressed: [STORE-02]
# STORE-02: this plan delivers migrate --plan/--apply, archive, rename declarations and --drop-archived. The pending/active
# two-step switch, reload lock and Trino probe are 03-18, and cross-version log replay reads migration_json later, so
# REQUIREMENTS.md stays unticked (policy of 03-01).

coverage:
  - id: D1
    description: "A removed property blocks reload with ONT-V25, migrate --plan lists one ArchiveColumn with a plan hash, --apply archives the column with its data, reload then reports no change"
    requirement: STORE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/migrate.rs#archive_removed_property"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_migrate.rs#migrate_plan_then_apply"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/migrate_pg.rs#archive_removed_property_on_pg"
        status: pass
    human_judgment: false
  - id: D2
    description: "apply refuses a stale hash and a repeated apply of an applied plan (TOCTOU, idempotency); nothing is changed"
    requirement: STORE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/migrate.rs#apply_with_stale_hash_is_rejected"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/migrate.rs#reapply_after_success_is_rejected"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_migrate.rs#apply_with_a_wrong_hash_exits_1"
        status: pass
    human_judgment: false
  - id: D3
    description: "Rename declarations move the data with the column or table (indexes rebuilt); without a declaration the old column is archived and the new one is empty; a type change under a rename is refused"
    requirement: STORE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/migrate.rs#rename_property_moves_data"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/migrate.rs#undeclared_rename_archives_and_adds_an_empty_column"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/migrate.rs#rename_rebuilds_the_index_of_the_column"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/migrate.rs#rename_object_type_moves_table"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/migrate.rs#rename_with_type_change_is_rejected"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/migrate.rs#invalid_rename_decl_is_rejected"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/migrate_pg.rs#rename_property_and_type_on_pg"
        status: pass
    human_judgment: false
  - id: D4
    description: "Type change archives the old column and adds an empty one with its index; a primary key change archives the table; a removed type and a removed many-to-many link archive their tables"
    requirement: STORE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/migrate.rs#type_change_archives_and_adds"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/migrate.rs#primary_key_change_archives_table"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/migrate.rs#removed_type_archived"
        status: pass
    human_judgment: false
  - id: D5
    description: "--drop-archived is the only way archived data is deleted; it needs its own plan hash, removes archived columns and tables, and leaves the edit log untouched"
    requirement: STORE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/migrate.rs#drop_archived_cleans_up"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_migrate.rs#drop_archived_plan_and_apply"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/migrate_pg.rs#primary_key_change_then_drop_archived_on_pg"
        status: pass
    human_judgment: false
  - id: D6
    description: "The CLI parses --plan / --apply / --plan-hash / --rename / --drop-archived, prints steps and the plan hash (human and JSON), and uses reload's exit codes"
    requirement: STORE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge/tests/cli_migrate.rs#apply_requires_plan_hash"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_migrate.rs#plan_and_apply_are_exclusive_and_one_is_required"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_migrate.rs#bad_rename_declarations"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_migrate.rs#plan_json_output"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_migrate.rs#migrate_before_any_reload_is_refused"
        status: pass
    human_judgment: false
  - id: D7
    description: "The reload path was refactored onto execute_steps / switch_definition without a behaviour change; migration_json is recorded for migrate"
    requirement: STORE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload.rs"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload_pg.rs"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_reload.rs"
        status: pass
    human_judgment: false

duration: 2h
completed: 2026-10-20
---

# Phase 3 Plan 15: Migrate Summary

**`ontoforge migrate` turns the destructive changes `reload` refuses (ONT-V25) into a reviewed, hash-bound plan that archives columns and tables instead of deleting them, with explicit rename declarations and a separate `--drop-archived` clean-up.**

## Accomplishments

- `plan_migration` / `apply_migration`: rename declarations are applied to a copy of the active physical schema, the same `diff` that `reload` uses runs against it, and the result becomes `MigrationStep`s plus an FNV-1a 64 hash. `apply` re-plans inside the write transaction from the definition read in that transaction and refuses any other hash before running DDL (T-03-46). A repeated apply of an applied plan is refused because the plan is empty by then.
- Archive semantics (D-16, ADR-0024): `<col>` -> `<col>__arch_v<N>` and `<col>__set` -> `<col>__set__arch_v<N>` by `RENAME COLUMN`, whole tables to `<table>__arch_v<N>`; records go to `PhysicalSchema.archived_columns` / `archived_tables`, so reload after a migration reports no change. Edit data and edit log rows are never touched.
- Rename declarations (D-14) for properties and object types move the data (`RenameColumn`, `RenameTable`), rebuild the affected indexes and refuse a different type, a new `required`, a primary-key rename or colliding names (T-03-48).
- `reload` now runs through `execute_steps` and `switch_definition`; migrate shares them, so 03-18's lock timeout, advisory lock, fence and Trino probe cover migrate. `migration_json` of the new `of_def_version` row holds the plan.
- `--drop-archived --plan/--apply` removes archived columns and tables under its own hash (T-03-47).
- CLI `ontoforge migrate` with human and JSON output and reload's exit codes (0 / 1 / 3; clap usage errors 2).

## Task Commits

| Task | Name | Commit |
|------|------|--------|
| 1 RED | tracer tests, stub migrate API | ef42d38 |
| 1 GREEN | plan/apply, archive, shared reload path, CLI | fda8315 |
| 2 RED | rename / type / key / removal / clean-up tests, PG tests, camelCase JSON assertion | cef9eb0 |
| 2 GREEN | camelCase plan, step and declaration JSON | 4f17c6b |

## Deviations from Plan

### Auto-fixed / design adjustments

**1. [Design] Indexes are dropped at archive time, not at clean-up time**
- **Found during:** Task 1 design
- **Issue:** The plan says the SQLite clean-up drops indexes on archived columns before `DROP COLUMN`. A kept index keeps its name, which blocks `CREATE INDEX IF NOT EXISTS` on a re-added column of the same name (type change with `indexed: true`) without an error.
- **Fix:** `ArchiveColumn` / `ArchiveTable` are preceded by explicit `DropIndex` steps. `drop_archived_cleans_up` covers an archived column that had an index.
- **Commit:** fda8315

**2. [Interface] Steps carry more fields than the interface sketch**
- `ArchiveColumn` and `RenameColumn` also carry `table`; `RenameColumn` carries the property and column names. Needed to execute a step without the schema. Same for `ArchiveTable` (kept as sketched).
- **Commit:** fda8315

**3. [Scope] Task 2 behaviour was implemented in the Task 1 GREEN commit**
- The planner is one pass over the diff, so rename, type change, key change, removal and clean-up came with the tracer. Task 2's RED commit therefore consisted of tests that mostly passed on arrival; the one real RED is `plan_json_output` (plan fields were snake_case). To show the other tests can fail I mutated `migrate.rs` four ways (drop the `__set` rename, skip the index drop, accept a type-changing rename, forget archived tables); each mutation failed 1 to 3 of the new tests, and the file was restored.

## TDD Gate Compliance

Task 1: RED (ef42d38, three assertion failures on the stub) then GREEN (fda8315). Task 2: RED (cef9eb0) has one failing test for the right reason; the rest of its tests were not independently red, see deviation 3. No REFACTOR commit.

## Known Stubs

None.

## Threat Flags

None. The new surface is the `migrate` subcommand against the configured edit store; T-03-46, T-03-47 and T-03-48 are mitigated as the plan states.

## Verification

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`: clean, all passing (storage `tests/migrate.rs` 13 passed, `cli_migrate.rs` 9 passed).
- PostgreSQL via `--features embedded-pg -- --ignored`: `migrate_pg` 3 passed, `reload_pg` 5 passed, `conformance_pg` 17 passed.
- `sql_exit_guard` passes: no SQL text outside `sql_exit.rs`.

## Issues Encountered

- A combined clippy plus workspace test run was killed once (exit 137, memory); rerun split and with `-j 4`, no code impact.
- Edge not covered by a test: PostgreSQL keeps the `_pkey` index name of a renamed or archived table. The pk-change PG case (archive, then create a table of the old name) passes, so no collision showed up.

## Next Phase Readiness

03-16 can reuse `naming::fnv1a64_hex`. 03-18 hooks the lock and fence into `execute_steps` / `switch_definition`. Cross-version log replay (ADR-0026) can read `migration_json`, which now holds `{fromDefVersion, renames, steps, hash}` for migrate rows and is null for reload rows.

## Self-Check: PASSED

- Created files exist: migrate.rs (storage, CLI), tests/migrate.rs, tests/migrate_pg.rs, cli_migrate.rs.
- Commits ef42d38, fda8315, cef9eb0, 4f17c6b are on the branch.
