---
phase: 03-edit-store-scope-model
plan: 04
subsystem: storage
tags: [cli, reload, postgres, sqlx, sea-query, insta, embedded-pg, adr]

requires:
  - phase: 03-edit-store-scope-model
    provides: "03-03 ontoforge_storage::reload::reload, EditStore (SQLite), ddl::render, sql_exit.rs, tests/support"
provides:
  - "`ontoforge reload <DIR> --server-config <FILE> [--dry-run] [--profile] [--format] [--color]`, exit codes 0 / 1 / 3 (D-15)"
  - "EditStore on PostgreSQL: two schemas, connection-level search_path, sqlx migration recorded in schema_sys (STORE-01)"
  - "PostgreSQL dialect of ddl::render (text COLLATE \"C\", numeric(p, s), timestamptz, native arrays, jsonb, IF NOT EXISTS)"
  - "insta snapshots of the aviation DDL for both dialects"
  - "support::pg_store() (ONTOFORGE_PG_URL or --features embedded-pg) and tests/reload_pg.rs"
  - "ADR-0024 amendment: PostgreSQL system column types and defaults"
affects: [03-08, 03-10, 03-15, 03-16, 03-18, 03-20]

plan_head_before: 06b87f70f071d839487e1aa255abcdd59cf80ecf

actuals:
  tokens: 24000
  tasks: 2
  commits: 4

tech-stack:
  added: []
  patterns:
    - "System tables are addressed by bare name and resolved through search_path = schema_sys; edit tables always by schema-qualified name"
    - "Dialect-specific statement text lives in sql_exit.rs / ddl.rs; WriteTx::Postgres carries the edit schema so callers need no extra argument"
    - "Ignored PG tests panic without ONTOFORGE_PG_URL / embedded-pg; embedded server is shared through a Weak so it stops with the last test"

key-files:
  created:
    - crates/ontoforge/src/reload.rs
    - crates/ontoforge/tests/cli_reload.rs
    - crates/ontoforge-storage/migrations/postgres/0001_definitions.sql
    - crates/ontoforge-storage/tests/reload_pg.rs
    - crates/ontoforge-storage/src/snapshots/ontoforge_storage__ddl__tests__ddl_sqlite.snap
    - crates/ontoforge-storage/src/snapshots/ontoforge_storage__ddl__tests__ddl_postgres.snap
  modified:
    - crates/ontoforge/src/cli.rs
    - crates/ontoforge/src/lib.rs
    - crates/ontoforge/src/online.rs
    - crates/ontoforge-storage/src/store.rs
    - crates/ontoforge-storage/src/ddl.rs
    - crates/ontoforge-storage/src/sql_exit.rs
    - crates/ontoforge-storage/src/reload.rs
    - crates/ontoforge-storage/src/error.rs
    - crates/ontoforge-storage/tests/support/mod.rs
    - .planning/adr/0024-edit-store-physical-format.md

key-decisions:
  - "PostgreSQL system columns get the same defaults as SQLite (of_version / of_def_version bigint DEFAULT 0, of_deleted / of_released boolean DEFAULT false, of_log_seq bigint NULL); recorded as an ADR-0024 amendment"
  - "PG column types are emitted as raw custom type names so that COLLATE \"C\" sits before PRIMARY KEY / NOT NULL, the only valid position in PostgreSQL"
  - "Definition errors and configuration errors exit 1; StoreError::Config maps to ONT-V20 (exit 1), every other store failure to ONT-V26 (exit 3)"

patterns-established:
  - "CLI commands print the diagnostics first, then the result; JSON mode is one document {diagnostics, result}"

# STORE-01 needs the conformance suite on PostgreSQL (03-08) and STORE-02 needs migrate (03-15) and the visibility
# fence (03-18); this plan delivers the reload entry point and the PG backend only. REQUIREMENTS.md stays unticked
# (policy decided in 03-01).
requirements-completed: []
requirements-addressed: [STORE-01, STORE-02]

coverage:
  - id: D1
    description: "`ontoforge reload` builds the edit tables on SQLite, reports def_version 1, and reports `definitions unchanged` the second time"
    requirement: STORE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge/tests/cli_reload.rs#reload_creates_tables"
        status: pass
    human_judgment: false
  - id: D2
    description: "`--dry-run` prints the DDL and changes nothing"
    requirement: STORE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge/tests/cli_reload.rs#dry_run_prints_ddl_only"
        status: pass
    human_judgment: false
  - id: D3
    description: "A definition with a validate error exits 1 and never creates the store file"
    requirement: STORE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge/tests/cli_reload.rs#invalid_definitions_never_open_store"
        status: pass
    human_judgment: false
  - id: D4
    description: "Missing [store] is ONT-V20 with exit 1; an unusable store is ONT-V26 with exit 3; --format json carries {diagnostics, result}"
    requirement: STORE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge/tests/cli_reload.rs#missing_store_section"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_reload.rs#store_unavailable"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_reload.rs#json_output"
        status: pass
    human_judgment: false
  - id: D5
    description: "On PostgreSQL edit tables exist only in schema_edit, definition tables and _sqlx_migrations only in schema_sys; column types and the C collation match ADR-0024"
    requirement: STORE-01
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload_pg.rs#first_reload_on_pg_puts_edit_tables_in_edit_schema"
        status: pass
    human_judgment: false
  - id: D6
    description: "A second reload on PostgreSQL is a no-op (stored definition reads back equal from jsonb); an additive property adds two columns and definition version 2"
    requirement: STORE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload_pg.rs#second_reload_on_pg_is_noop"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/reload_pg.rs#additive_reload_on_pg_adds_columns"
        status: pass
    human_judgment: false
  - id: D7
    description: "Both dialects' DDL for the aviation example is fixed by insta snapshots"
    requirement: STORE-01
    verification:
      - kind: unit
        ref: "crates/ontoforge-storage/src/ddl.rs#ddl_snapshots"
        status: pass
    human_judgment: false
  - id: D8
    description: "ADR-0024 amendment records the PostgreSQL system column types and defaults decision"
    verification: []
    human_judgment: true
    rationale: "Document content is a design record; a reviewer judges wording and completeness"

duration: about 1h20
completed: 2026-10-20
status: complete
---

# Phase 3 Plan 4: `ontoforge reload` CLI and PostgreSQL edit store backend Summary

**`ontoforge reload` now validates, lints, opens the edit store from `[store]` and calls the library `reload` (exit 0 / 1 / 3, `--dry-run`, JSON result), and the same reload code runs on PostgreSQL with edit tables in `schema_edit`, definition tables and sqlx migrations in `schema_sys`, `text COLLATE "C"` strings and `IF NOT EXISTS` DDL; both dialects are fixed by insta snapshots and the PG tests ran green on a real server.**

## Performance

- **Duration:** about 1h20
- **Completed:** 2026-10-20
- **Tasks:** 2 of 2
- **Files:** 16 changed since plan start (6 created in the plan's scope plus 2 snapshots, 10 modified)
- **Verification:** `cargo test --workspace --locked` 726 passed / 0 failed / 15 ignored; `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo clippy -p ontoforge-storage --all-targets --features embedded-pg --locked -- -D warnings` clean; `cargo fmt --all --check` clean

## PostgreSQL tests: how they ran

- `cargo test -p ontoforge-storage --features embedded-pg --locked --test reload_pg -- --ignored` on this Windows 11 host: **3 passed** (concurrent run 4.4 s, `--test-threads=1` run 13.2 s). `postgresql_embedded` downloaded and started a server without Docker; no `postgres` process remained afterwards.
- A first run had one test fail with `DatabaseInitializationError("deadline has elapsed")`: the crate's default 5 s timeout is too short for initdb on this machine. `support::pg_store()` now sets the embedded settings timeout to 180 s.
- Without `ONTOFORGE_PG_URL` and without the feature the three tests panic with the instruction text (checked), so `--ignored` cannot pass silently.
- The default `cargo test --workspace` lists the three tests as ignored; the Linux CI job with a service container is 03-20.

## Decision: system column defaults on PostgreSQL (open item from 03-03)

PostgreSQL gets the same defaults as SQLite: `of_version` and `of_def_version` are `bigint NOT NULL DEFAULT 0`, `of_deleted` and `of_released` are `boolean NOT NULL DEFAULT false`, `of_log_seq` is `bigint NULL`. Reasons: (1) the repair path (`CREATE .. IF NOT EXISTS`, introspect, `ADD COLUMN` for missing columns) is one code path for both backends, and PostgreSQL refuses `ADD COLUMN .. NOT NULL` without a default on a non-empty table; (2) one `DdlStep` / `physical_json` describes both backends; (3) no cost, because PostgreSQL 11+ stores a constant default as metadata and Trino only reads values. The shape was already implied for `__set` columns; the system columns' PG types were not spelled out in ADR-0024, so the decision is recorded as an amendment note there (commit `1a964ff`). The accepted downside, a default hiding a writer that forgets `of_def_version`, is the same as on SQLite and is covered by writers setting the column explicitly and by the 03-08 conformance suite.

## Accomplishments

- `ontoforge reload` (`crates/ontoforge/src/reload.rs`): `lint_cmd::lint` first (any error, or a warning under `--deny-warnings`, exits 1 and returns before the store is touched), then `load_server_config` and `StoreSettings::from_config` (relative to the config file), a current-thread runtime, `EditStore::open`, `ontoforge_storage::reload::reload`. Output: dry run prints the DDL statements on stdout and `dry run: N DDL statement(s), nothing applied` on stderr; success prints `ok: def_version N applied (K DDL statement(s))` or `ok: definitions unchanged (def_version N)`. JSON mode prints one document with `diagnostics` and `result {defVersion, changed, dryRun, ddl}`.
- Exit codes: definition diagnostics (including the library's ONT-V25) and configuration errors (ONT-V20, also `StoreError::Config`) exit 1; every other store failure is ONT-V26 and exits 3. No message carries a connection string: `PgConnectOptions::from_str` failures are replaced by a fixed text and `sqlx::Error::Configuration` is mapped to `invalid connection settings`.
- `EditStore::open` on PostgreSQL: parse the secret URL, set `search_path = schema_sys` on every connection, `CREATE SCHEMA IF NOT EXISTS` for both schemas, then `sqlx::migrate!("./migrations/postgres")` so `_sqlx_migrations` lands in `schema_sys`. `WriteTx::Postgres` now carries `schema_edit`; `sql_exit.rs` has PostgreSQL arms for begin, DDL, built statements, scalar fetch, `table_columns` (information_schema, cast to text) and the definition read/write (`CAST(.. AS jsonb)` / `timestamptz` on write, `CAST(.. AS text)` on read).
- `ddl.rs`: dialect parameter through `render`, `create_sql`, `step_columns`, `add_column_sql`; PostgreSQL types emitted as raw custom names so the collation precedes the constraints (`"code" text COLLATE "C" NOT NULL PRIMARY KEY`); tables qualified by `schema_edit`; `ADD COLUMN IF NOT EXISTS`.
- Snapshots reviewed line by line against the ADR-0024 table: string `text COLLATE "C"`, decimal `numeric(12, 2)`, timestamp `timestamptz`, array `text[]`, geopoint `jsonb`, double `double precision`, date `date`, flags `bool NOT NULL DEFAULT FALSE`, SQLite `cost` `text COLLATE ontoforge_decimal`. Names such as `"ontoforge_edit"."of_obj_flight"` are as specified.

## Task Commits

| Task | Commit | What |
|------|--------|------|
| 1 RED | `9cfc799` | `test(03-04)`: six failing CLI tests (every one failed on `unrecognized subcommand 'reload'`, exit 2 instead of the expected code) |
| 1 GREEN | `6420212` | `feat(03-04)`: `ontoforge reload` subcommand |
| 2 | `f469d44` | `feat(03-04)`: PostgreSQL backend, dialect DDL, snapshots, `pg_store()`, `reload_pg` |
| 2 docs | `1a964ff` | `docs(03-04)`: ADR-0024 amendment |

## TDD Gate Compliance

Task 1: RED commit `9cfc799` precedes GREEN `6420212`; all six tests failed on the missing subcommand (an assertion on the exit code, not a build or load error). `gsd_run check tdd-red-evidence` was not run (it parses node TAP output, which `cargo test` does not emit).

Task 2: no separate RED commit. The snapshot test failed first (`ddl_snapshots` had no stored snapshot and failed until the reviewed `.snap` files were written) and the PG tests could not run before `pg_store()` existed, but the failing state was not committed on its own: the tests, the PostgreSQL implementation and the snapshots went in one `feat` commit. This is a TDD discipline gap for Task 2 and is flagged here rather than reconstructed afterwards.

## Deviations from Plan

**1. [Plan scope] `crates/ontoforge-storage/src/reload.rs` changed (storage-internal)**
- **Found during:** Task 2
- **Issue:** the plan forbids touching `reload.rs` unless PostgreSQL needs a flow change. `apply_step` called `ddl::create_sql(step)`, `ddl::step_columns(step)`, `ddl::add_column_sql(table, col)` and `table_columns(tx, None, table)`, none of which can know the dialect or the edit schema.
- **Fix:** `apply_step` reads `tx.backend()` and `tx.schema_edit()` and passes them on. The reload flow (diff, dry run, transaction, activation) is unchanged.
- **Commit:** `f469d44`

**2. [Plan scope] `crates/ontoforge-storage/src/error.rs` and `crates/ontoforge/src/online.rs` touched**
- `StoreError::postgres_unsupported` became dead code once the PG arms existed (clippy `-D warnings`), so it was removed. `online::config_diagnostic` became `pub(crate)` so the reload command reuses the ONT-V20 rendering (CR-C01: the configuration text is used for line and column only, never printed). Commits `f469d44` and `6420212`.

**3. [Interface] `pg_store()` returns `PgTestStore`, not `TestStore`**
- `TestStore` is SQLite-shaped (`TempDir`, file path, `side_pool`). `PgTestStore` has the same `store` field plus `schema_edit`, `schema_sys` and inspection helpers (`tables`, `columns`, `data_type`, `collation`, `active_version`, `version_rows`). 03-08 and 03-16 can use `.store` the same way; they need to import the new type name only if they annotate it.

**4. [Rule 1 - Bug] `deadline has elapsed` from the embedded server**
- Described above (180 s timeout). Commit `f469d44`.

**5. [Design choice within the plan] Embedded server lifetime**
- The plan says one embedded server per test binary. A process-wide static is never dropped in Rust, which would leave `postgres` running after the tests. The server is held through `Arc` by each `PgTestStore` and found again through a `Weak`, so concurrent tests share one server and the last one stops it (verified: no leftover process). With `--test-threads=1` each test starts its own server (13 s for three tests).

**6. [Scope] CONTRIBUTING.md not updated**
- The plan's `read_first` mentions CONTRIBUTING for the panic text, but the file is outside `files_modified` and has no PostgreSQL section. The panic message therefore carries the instructions itself; the CONTRIBUTING section and the CI service container belong to 03-20.

**Total deviations:** 6 (1 bug, 3 scope/interface, 2 design). No Cargo.toml changed.

## Auth Gates

None.

## Known Stubs

None.

## Threat Flags

None beyond the plan's register. T-03-09 (connection string never in errors or CLI output): `from_str` and configuration errors replaced with fixed text; no CLI test with a real PG URL exists yet, the PG path is covered at library level. T-03-71 is covered by `invalid_definitions_never_open_store`. T-03-72: `second_reload_on_pg_is_noop` covers the rerun; the `IF NOT EXISTS` forms are asserted in `postgres_add_column_is_qualified_and_idempotent` and the snapshot.

## Issues Encountered

- `ColumnDef::extra("COLLATE \"C\"")` would be written after `PRIMARY KEY`, which PostgreSQL rejects; hence the raw custom type names (unit test `postgres_collates_the_key_before_its_constraints`).
- The PG `information_schema` columns are domain typed; the introspection casts them to `text` before comparing with bound values and before decoding.
- `jsonb` and `timestamptz` columns cannot take a text parameter directly; the definition write casts in SQL and the read casts `jsonb` back to `text`.

## Next Phase Readiness

03-08 can build `conformance_pg` on `support::pg_store()` and the PostgreSQL arms of `sql_exit.rs` (DML helpers get added there). 03-10 adds length caps, ONT-V24 collisions and indexes on `naming.rs` / `physical.rs` and extends the snapshots. 03-15 (migrate) and 03-18 (reload lock, pending to active switch, Trino visibility probe) wrap `reload`'s transaction; 03-20 adds the CI PostgreSQL service container and the CONTRIBUTING section.

## Self-Check: PASSED

Created files found: `crates/ontoforge/src/reload.rs`, `crates/ontoforge/tests/cli_reload.rs`, `crates/ontoforge-storage/migrations/postgres/0001_definitions.sql`, `crates/ontoforge-storage/tests/reload_pg.rs`, both `.snap` files. Commits `9cfc799`, `6420212`, `f469d44`, `1a964ff` present in `git log`. Acceptance greps from the plan return matches (`COLLATE "C"` and `ontoforge_edit` in the snapshots, `migrate!("./migrations/postgres")` in `store.rs`, `ONTOFORGE_PG_URL` in `support/mod.rs`, `AssertSqlSafe` only in `sql_exit.rs`); `sql_exit_guard` passes; `reload_pg -- --ignored --list` lists 3 tests; no `.snap.new` remains.
