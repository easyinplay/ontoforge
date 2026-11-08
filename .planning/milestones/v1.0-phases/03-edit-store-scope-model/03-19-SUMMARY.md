---
phase: 03-edit-store-scope-model
plan: 19
subsystem: storage
tags: [import, csv, json, system-edit, edit-log, create-only, batching, cli, stor-06, tdd]
status: complete

requires:
  - phase: 03-edit-store-scope-model
    provides: "03-08 shared commit write path, Actor::System(SystemActor::Import), PropertyValue; 03-13 link edits; 03-16 CLI structure; 03-18 def_version fence in commit"
provides:
  - "ontoforge_storage::import::{ImportTarget, OnExisting, ImportOptions, ImportReport, ImportError, import} (library API; Phase 7 / tooling can call it)"
  - "PropertyValue::from_text: CSV cell to canonical value for every property type"
  - "ontoforge_engine::sqlite::fixture::{normalize_decimal, parse_date} public; the edit store's decimal and date rules are now the engine's"
  - "`ontoforge import <DIR> --server-config <FILE> (--type <T> | --link <L>) --file <csv|json> [--on-existing error|skip|update] [--batch-size N] [--max-rows N]`"
  - "examples/data/aviation/airports.{csv,json}"
affects: [03-20, phase-05-actions, phase-07-rest]

plan_head_before: 5e0406c946402d8a6cd53546e047482372694381

actuals:
  tokens: 29200
  tasks: 2
  commits: 5

tech-stack:
  added: []
  patterns:
    - "Read the whole file, convert and check every row, collect up to 100 ONT-V27 diagnostics with file and line, and only then write"
    - "RowReader trait: one CSV / JSON driver, one reader per target (objects, edges); the decision per row (create / update / skip / refuse) is one pure function"
    - "Existing rows are looked up in chunks of 500 keys inside a write transaction that is rolled back, with the commit path's own bind / read_expr / cell_json, so the lookup agrees with what a commit writes on both backends"

key-files:
  created:
    - crates/ontoforge-storage/src/import.rs
    - crates/ontoforge-storage/tests/import.rs
    - crates/ontoforge/src/import.rs
    - crates/ontoforge/tests/cli_import.rs
    - examples/data/aviation/airports.csv
    - examples/data/aviation/airports.json
  modified:
    - crates/ontoforge-storage/src/lib.rs
    - crates/ontoforge-storage/src/value.rs
    - crates/ontoforge-storage/src/commit.rs
    - crates/ontoforge-engine/src/sqlite/fixture.rs
    - crates/ontoforge/src/cli.rs
    - crates/ontoforge/src/lib.rs

key-decisions:
  - "Existence means an edit row with of_version > 0. A tombstoned object and a removed edge count as existing in every mode and are never brought back: error lists them, skip leaves them, update refuses the whole file"
  - "Edges follow the same three modes. An existing live edge has nothing to update, so update treats it as skipped; a repeat of the same file is refused in error mode and writes nothing in skip mode"
  - "update only sets the cells the file has; an empty cell or a missing key never unsets a property (an import cannot clear a value)"
  - "An object that has no value besides its key cannot be created (commit refuses an empty set), so such a row is an ONT-V27 problem of the file, not a late commit failure"
  - "A batch failure always returns ImportError::Partial, also when no batch was written before it (committed is empty); ImportError::Edit is only for a target that is not in the definition"
  - "The file is checked against the edit store's active definition, not against the directory. The CLI compares the two and adds an ONT-V26 warning when they differ"
  - "JSON items have no line, so their problems carry `item N:` in the message and line 0; CSV problems carry the real line"
  - "Batch index in params and in Partial is counted from 0"

patterns-established:
  - "Mutation evidence for tests that pass on arrival is recorded in the SUMMARY (see TDD Gate Compliance)"

requirements-completed: [STORE-06]
# STORE-06: editOnly objects and editOnly many-to-many links import from CSV and JSON as system:import commits in the edit log; the stand-in
#   engine data half of the requirement is delivered by the Phase 2 binding fixtures (orchestrator decision 3, plan truth 1). 03-19 is the only plan
#   that declares STORE-06. The user documentation of the command goes into docs/edit-store.md in 03-20 and is not part of the acceptance.

coverage:
  - id: D1
    description: "A CSV of an editOnly object type becomes system:import commits: actor and action are system:import, params hold file name, batch and row count, an empty cell is not set"
    requirement: STORE-06
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/import.rs#airports_csv_is_imported_as_system_edits"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_import.rs#imports_airports_csv"
        status: pass
    human_judgment: false
  - id: D2
    description: "Rows are committed in batches of --batch-size (3, 3, 1 for 7 rows and 3), batch size outside 1..=10000 is refused"
    requirement: STORE-06
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/import.rs#batches_follow_batch_size"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/import.rs#limits_are_enforced"
        status: pass
    human_judgment: false
  - id: D3
    description: "Cell conversion follows the edit store's value rules: geopoint lat,lon range checked, decimal padded and never rounded, decimal and date through the engine's public functions"
    requirement: STORE-06
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/import.rs#geopoint_and_decimal_cells"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-storage/src/value.rs#tests::text_cells_convert_to_canonical_values"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-storage/src/value.rs#tests::text_cells_that_do_not_fit_are_refused"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/import.rs#pg_cells_and_keys_of_every_kind"
        status: pass
    human_judgment: false
  - id: D4
    description: "A JSON array of objects imports to exactly the rows the CSV of the same data gives; a non-array, a non-object item and an unknown key are problems with their item number"
    requirement: STORE-06
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/import.rs#json_import_matches_csv"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/import.rs#empty_and_repeated_imports_are_stable"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_import.rs#imports_airports_json"
        status: pass
    human_judgment: false
  - id: D5
    description: "An editOnly many-to-many link imports from/to rows as link_add edits; ends are converted with the key types of the two object types; the same edges again are refused or skipped"
    requirement: STORE-06
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/import.rs#editonly_link_import"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_import.rs#link_import_and_partial_failure"
        status: pass
    human_judgment: false
  - id: D6
    description: "Prohibition: source backed object types and links, and links that are not many-to-many, are refused with nothing written; existing objects are never overwritten unless --on-existing update is chosen"
    requirement: STORE-06
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/import.rs#source_backed_type_is_rejected"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/import.rs#existing_objects_block_create_only"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_import.rs#source_backed_type_is_rejected"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_import.rs#existing_objects_block_create_only"
        status: pass
    human_judgment: false
  - id: D7
    description: "skip and update act per the mode, an update bumps of_version and keeps cells the file lacks, a tombstoned object is never revived and its file writes nothing in error and update mode"
    requirement: STORE-06
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/import.rs#existing_objects_block_create_only"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/import.rs#tombstoned_object_is_never_revived"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/import.rs#pg_import_modes_and_links"
        status: pass
    human_judgment: false
  - id: D8
    description: "The whole file is checked before any write: unknown, repeated and missing columns, empty required cells, repeated keys, bad cells, non-UTF-8, extension, max rows (exact limit accepted, one more refused), max bytes, 100 problems at most; a header-only file and a byte order mark are accepted"
    requirement: STORE-06
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/import.rs#file_level_validation"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/import.rs#limits_are_enforced"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/import.rs#at_most_one_hundred_problems_are_listed"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/import.rs#empty_and_repeated_imports_are_stable"
        status: pass
    human_judgment: false
  - id: D9
    description: "A batch that fails after others were written returns Partial with the committed seqs and the failed batch; the CLI prints both and says to rerun with --on-existing skip, which finishes the file; a commit validated against a stale definition is refused by the 03-18 fence"
    requirement: STORE-06
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/import.rs#partial_failure_is_reported"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/import.rs#import_goes_through_the_definition_fence"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_import.rs#link_import_and_partial_failure"
        status: pass
    human_judgment: false
  - id: D10
    description: "CLI behaviour: exit codes 0 / 1 / 2 / 3, reload hint when the store holds no definition, unknown type, clap refusals, JSON result, warning when the directory differs from the stored definition"
    requirement: STORE-06
    verification:
      - kind: integration
        ref: "crates/ontoforge/tests/cli_import.rs#command_line_mistakes_are_reported"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_import.rs#json_output_reports_the_result"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_import.rs#a_directory_that_was_not_reloaded_is_flagged"
        status: pass
    human_judgment: false

duration: about 1 h 20 min
completed: 2026-10-21
---

# Phase 3 Plan 19: Import of editOnly Data Summary

**`ontoforge import` reads a CSV or JSON file, checks every row of it against the edit store's active definition, and only then writes: editOnly objects and editOnly many-to-many links go in as batched `system:import` commits through the shared commit path, create-only by default, with `skip` and `update` as explicit choices and a failed batch reported so that a `skip` rerun finishes the file.**

## Accomplishments

- **Library** (`ontoforge_storage::import`): `import(store, target, file, opts)` for `ImportTarget::Object` and `ImportTarget::Link`. A `RowReader` trait drives one CSV reader (streaming, size-capped) and one JSON reader (array of objects) for both targets. Problems are collected as ONT-V27 diagnostics with file and line (JSON: `item N`), 100 at most; nothing is written when there is one. Existing rows are looked up in chunks of 500 keys, then a pure `decide` function turns (mode, existing, has values) into create, update, skip or a problem. Rows go out as `Commit { actor: Actor::System(SystemActor::Import), action: "system:import", params: {file, batch, rows} }` in batches of `batch_size`, through `EditStore::commit`, so the 03-18 fence, the log, the history snapshots and the one-to-one checks apply unchanged.
- **Values**: `PropertyValue::from_text` for every property type. `normalize_decimal` and `parse_date` of the engine fixture are public and `value.rs` now calls them (its own copies are gone), so a fixture, an Action and an import agree on decimals and dates. The engine message "a fixture value is never rounded" became "a value is never rounded".
- **CLI**: `ontoforge import` with `--type` / `--link` (exclusive, one required), `--file`, `--on-existing`, `--batch-size 1..=10000`, `--max-rows`, `--profile`, `--format`, `--color`. Exit 0 / 1 / 3 as `reload`; a store without a definition says to run `ontoforge reload`; a directory that differs from the stored definition adds an ONT-V26 warning.
- **Data**: `examples/data/aviation/airports.csv` and `.json` (five invented airports, one without a city).

## Task Commits

| Task | Name | Commit |
|------|------|--------|
| 1 | RED: airports CSV, batches, cell conversion (shells + failing tests) | 290505c |
| 1 | GREEN: CSV objects, from_text, public engine functions, CLI | 303bfc1 |
| 2 | RED: JSON, links, modes, limits, failed batch | 8122849 |
| 2 | GREEN: JSON, links, skip / update, Partial, PostgreSQL cases | 331d44d |
| 2 | test: directory-differs warning | 4a01cea |

## TDD Gate Compliance

Every task has a `test(03-19)` commit before its `feat(03-19)` commit. RED commits carry the public types and the CLI as shells (the import returned an empty report, `from_text` an error), so the tests failed on behaviour.

- **Task 1 RED**: `airports_csv_is_imported_as_system_edits` (`left: 0, right: 5`), `batches_follow_batch_size` (`left: 0, right: 7`), `geopoint_and_decimal_cells` (index out of bounds on the empty log), `text_cells_convert_to_canonical_values`, CLI `imports_airports_csv` (exit 1). `text_cells_that_do_not_fit_are_refused` passed on arrival (the stub refuses everything).
- **Task 2 RED**: 7 of 14 storage tests and 3 of 7 CLI tests failed. Passed on arrival because the Task 1 tracer already covered them: `source_backed_type_is_rejected` (storage and CLI), `limits_are_enforced`, `file_level_validation`, `import_goes_through_the_definition_fence`, `geopoint_and_decimal_cells`, `batches_follow_batch_size`. `at_most_one_hundred_problems_are_listed` failed for a real defect of the Task 1 code: the "more problems follow" note was never added because reading stopped first (`Problems::stop_reading` now records it).
- **Mutations** (each applied, the named test failed, then restored from a saved copy):

| Mutation | Test that failed |
|---|---|
| object backing check removed (`if false`) | storage `source_backed_type_is_rejected`, CLI `source_backed_type_is_rejected` |
| link backing check removed | storage `source_backed_type_is_rejected` |
| link cardinality check removed | storage `source_backed_type_is_rejected` |
| `data_rows > max_rows` to `>=` | `limits_are_enforced` |
| byte limit check removed | `limits_are_enforced` |
| missing required column not reported | `file_level_validation` |
| unknown column not reported | `file_level_validation` |
| update of a deleted object no longer refused | `tombstoned_object_is_never_revived` |
| skip acts as update | `existing_objects_block_create_only`, `tombstoned_object_is_never_revived`, `partial_failure_is_reported`, `empty_and_repeated_imports_are_stable` |
| `from_text` without the final `canonical` check | `text_cells_convert_to_canonical_values`, `text_cells_that_do_not_fit_are_refused` |
| fence and `validate` version comparisons removed in `commit.rs` | `import_goes_through_the_definition_fence` |
| "directory differs" warning never pushed | `a_directory_that_was_not_reloaded_is_flagged` |

Tests added after GREEN without a RED run: the two PostgreSQL cases (`pg_import_modes_and_links`, `pg_cells_and_keys_of_every_kind`) and the directory-differs test; the last one has the mutation above, the PostgreSQL cases passed on first run against the implementation.

## Deviations from Plan

**1. [Rule 3 - Blocking] `commit.rs` visibility.** `bind`, `read_expr` and `cell_json` became `pub(crate)` so the existing-row lookup converts keys exactly as a commit writes them (the plan's file list does not name `commit.rs`). No behaviour change. Commit 303bfc1.

**2. [Interface] Engine error text.** `normalize_decimal` said "a fixture value is never rounded"; since the edit store shows the text to import users it now says "a value is never rounded". No test or snapshot depended on it. Commit 303bfc1.

**3. [Interface] `ImportError::Partial` for every batch failure.** The plan sketched `Partial` for later batches; a failure of the first batch (for example `DefinitionStale`) is also `Partial` with an empty `committed`, so the caller learns the batch index in one place. `ImportError::Edit` is only used for a target that is not in the definition. Commit 331d44d.

**4. [Interface] JSON positions.** A JSON item has no line; its problems read `item N: ...` with `line` 0. CSV problems carry the real line. Commit 331d44d.

**5. [Rule 1 - Bug] Truncation note.** Found by the RED test of Task 2 (see TDD Gate Compliance); fixed in 331d44d.

**6. [Scope] Additions beyond the plan.** Edge modes (`--on-existing` applies to links too, plan only named objects), the warning when the directory differs from the stored definition, the two PostgreSQL cases and the CLI tests for JSON output, clap refusals and the reload hint.

**Total deviations:** 6 (1 blocking-fix, 1 bug, 3 interface, 1 scope). **Impact:** none on the plan's truths.

## Known Gaps and Not Verified

- **Create-only is checked before the batches, not inside them.** Existing keys are read once, before the first commit. An actor that creates the same key between that read and the commit of its batch gets its cells patched by the import instead of a refusal, because `Commit` has no "must not exist" flag and 03-08 fixed that API. A single operator running a CLI makes this unlikely; Phase 5 Actions that create objects are the realistic concurrent writer. Closing it needs a precondition on the commit (not done).
- The CLI was exercised on SQLite only; PostgreSQL was exercised through the library (`pg_import_modes_and_links`, `pg_cells_and_keys_of_every_kind`) with string, integer and timestamp keys and edges. Decimal, date and float keys were not looked up on PostgreSQL.
- Memory: the checked rows are held in memory until the write phase (up to 1 000 000 rows or 256 MiB of file); the CSV is streamed, the JSON is read whole.
- An empty string cannot be imported for a `string` property (an empty cell means "not set"), and an import never unsets a property.
- The `--deny-warnings` option only applies to the definition lint at the start, not to the "directory differs" warning.
- The user documentation of the command is left to `docs/edit-store.md` in 03-20.

## Verification

- `cargo test --workspace --locked`: 970 passed, 0 failed, 64 ignored.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` (also with `--features ontoforge-storage/embedded-pg`) and `cargo fmt --all --check`: clean.
- Embedded PostgreSQL (`--features embedded-pg -- --ignored`, Windows 11, no Docker): `import` 2 passed, `conformance_pg` 27, `history_masking` 1, `reload_pg` 11, `migrate_pg` 3, `scope_store` 3, all passed.
- Task 1 verify (`import` 3 tests, `cli_import` 1 test, engine and `conformance_sqlite`) re-run before expansion: passed. Task 2 acceptance: `source_backed_type_is_rejected`, `existing_objects_block_create_only`, `tombstoned_object_is_never_revived` each `1 passed`; the `import` target has 14 passing tests (plan asks for 9 or more).
- One full-workspace run was started twice in one command line, ran past the 600 s foreground limit and was moved to the background by the harness; it was waited for and finished with exit 0. No test hung.

## Issues Encountered

None open. A heredoc with Rust lifetimes (`'a`) broke the shell tool twice; those files were written with the editor tool instead.

## Known Stubs

None.

## Threat Flags

None beyond the plan's register. T-03-63 (import as a general write door): editOnly targets only, create-only default, `system:import` in the log. T-03-64 (huge file): 256 MiB, 1 000 000 rows, streamed CSV, 100 problems. T-03-65 (bad values): the same `PropertyValue` conversion and `canonical` check as every commit, all rows before the first write. T-03-66 (no record): one log record per batch with file name and batch. Messages quote at most 40 characters of a cell and the CLI renderer strips terminal controls.

## Self-Check: PASSED

- Files exist: `crates/ontoforge-storage/src/import.rs`, `crates/ontoforge-storage/tests/import.rs`, `crates/ontoforge/src/import.rs`, `crates/ontoforge/tests/cli_import.rs`, `examples/data/aviation/airports.csv`, `examples/data/aviation/airports.json`.
- Commits 290505c, 303bfc1, 8122849, 331d44d, 4a01cea are in `git log`; `git rev-list --count 5e0406c..HEAD` was 5 before this summary.
- Acceptance greps: `pub fn normalize_decimal` in fixture.rs, `normalize_decimal` in value.rs, `SystemActor::Import` in import.rs, `Import(ImportArgs)` in cli.rs.
