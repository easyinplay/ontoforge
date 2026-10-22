---
phase: 03-edit-store-scope-model
plan: 16
subsystem: storage
tags: [scope, roles-as-code, plan-apply, admin-audit, append-only, user-record, row-filter-index, cli, sqlite, postgres, tdd]
status: complete

requires:
  - phase: 03-edit-store-scope-model
    provides: "03-06 load_scope_dir / ScopeSet / SourceMap; 03-07 Principal; 03-09 append-only trigger pattern and of_reject_mutation(); 03-10 naming::index_name and DdlStep::CreateIndex; 03-12 [auth] config; 03-14 compile_scope, row_filter_properties, attr_types; 03-15 fnv1a64_hex, read_active_definition_in, CLI reload/migrate structure"
provides:
  - "ontoforge_storage::scope_store::{ScopePlan, BindingRow, SubjectKind, AppliedScope, ScopeApplyError, LoginRecorder, normalize_role, normalize_scope}; EditStore::{load_scope, scope_plan, scope_apply, record_login}"
  - "System tables of_user, of_role, of_role_binding, of_admin_audit (migrations/<backend>/0003_scope.sql); of_admin_audit is append only (SQLite BEFORE UPDATE/DELETE triggers; PostgreSQL row triggers plus a statement-level BEFORE TRUNCATE trigger on of_reject_mutation())"
  - "index::role_index_steps and sql_exit::{DEFINITION_LOCK_KEY, advisory_xact_lock, set_local_lock_timeout, fetch_all_cells, existing_index_names}, reusable by 03-18"
  - "CLI: ontoforge scope plan <DIR> --ontology <DIR> --server-config <FILE> and scope apply ... --plan-hash <HASH>, exit codes 0 / 1 / 3"
  - "examples/server/ontoforge-server.dev.toml (SQLite store below target/, homeAirports attribute, dev key file) and the Role / RoleBinding section of docs/scope.md"
affects: [03-17, 03-18, 03-19, 03-20, phase-04-query, phase-07-rest]

plan_head_before: 3efa7fdc62216d1e3f4023b75630bb8fa04dabab

actuals:
  tokens: 26500
  tasks: 2
  commits: 4

tech-stack:
  added: []
  patterns:
    - "A plan is a pure function of (desired scope, stored scope, active physical schema, existing index names); every list is sorted and the hash is FNV-1a 64 over the camelCase JSON of all other fields, so it does not depend on YAML order"
    - "apply re-plans inside the write transaction under the definition advisory lock and refuses any hash other than the reviewed one before it writes (T-03-54); the roles, bindings, indexes and the audit record are one commit"
    - "Full sync: delete bindings, delete roles, insert / update roles, insert bindings, create missing indexes, append audit (foreign key of_role_binding.role -> of_role)"
    - "Offline check before the store is opened: lint ontology, load scope dir, load server config, compile_scope; any diagnostic is exit 1 with the store untouched"

key-files:
  created:
    - crates/ontoforge-storage/src/scope_store.rs
    - crates/ontoforge-storage/src/index.rs
    - crates/ontoforge-storage/migrations/sqlite/0003_scope.sql
    - crates/ontoforge-storage/migrations/postgres/0003_scope.sql
    - crates/ontoforge-storage/tests/scope_store.rs
    - crates/ontoforge/src/scope.rs
    - crates/ontoforge/tests/cli_scope.rs
    - examples/server/ontoforge-server.dev.toml
  modified:
    - crates/ontoforge-storage/src/lib.rs
    - crates/ontoforge-storage/src/sql_exit.rs
    - crates/ontoforge-storage/src/store.rs
    - crates/ontoforge-storage/tests/reload_pg.rs
    - crates/ontoforge/src/cli.rs
    - crates/ontoforge/src/lib.rs
    - crates/ontoforge/src/reload.rs
    - docs/scope.md

key-decisions:
  - "Only scope_store.rs names of_role, of_role_binding and of_admin_audit; tests/scope_store.rs scans src/ (comments included) and fails when another file does (SCOPE-07 / D-03, T-03-52)"
  - "Plan lists only indexes that do not exist yet (existing names are read from sqlite_master / pg_indexes inside the transaction), so a second plan is empty. In the aviation example Flight.status is indexed: true already, so crewScheduler's departureAirportCode is the one index apply creates"
  - "Indexes created by apply are not recorded in PhysicalSchema; they are IF NOT EXISTS and only ever added, so reload neither drops nor re-creates them (orchestrator decision 8)"
  - "scope_plan on a store with no active definition returns a plan without index DDL instead of an error; scope_apply returns NoActiveDefinition, and the CLI asks for ontoforge reload before either"
  - "The CLI compares the loaded ontology with the active definition as JSON values and refuses on a difference, so a scope is only ever compiled against the definition its predicates will run on"
  - "A no-change apply is allowed and still appends an audit record: every apply is one auditable event"
  - "of_role_binding.role references of_role(api_name): a binding cannot outlive its role and the apply order (bindings out, roles out, roles in, bindings in) is forced by the schema"

patterns-established:
  - "sql_exit::DEFINITION_LOCK_KEY is the one advisory lock for reload, migrate and scope apply; 03-18 wires reload and migrate to it"
  - "CLI commands that need both [store] and [auth] use reload::server_settings; store_settings is its store-only wrapper"

# SCOPE-01 / 02 / 06 / 07: this plan delivers the admin channel (plan / apply / audit / user record) and the apply-side criteria check
# through 03-14. Group sync from OIDC at request time, scoped reads and action authorization (Phase 4 / 5), the reload re-check
# and the request-pinned definition version (03-18) and the REST read-only role view (Phase 7) are still open, so
# REQUIREMENTS.md stays unticked (policy decided in 03-01).
requirements-completed: []
requirements-addressed: [SCOPE-01, SCOPE-07, SCOPE-02, SCOPE-06]

coverage:
  - id: D1
    description: "scope plan lists add role / add binding / create index and a 16 digit plan hash; scope apply --plan-hash writes the aviation scope to the store; a second plan shows no changes; the audit actor is <user>@<host>"
    requirement: SCOPE-01
    verification:
      - kind: integration
        ref: "crates/ontoforge/tests/cli_scope.rs#plan_then_apply_aviation_scope"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/scope_store.rs#apply_writes_roles_bindings_and_audit"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/scope_store.rs#load_scope_round_trips"
        status: pass
    human_judgment: false
  - id: D2
    description: "A scope that does not compile (ONT-V22) never reaches the store: plan and apply exit 1 with the diagnostic and of_role, of_role_binding and of_admin_audit stay empty; an ontology that differs from the active definition, a missing reload and a missing [store] section are refused"
    requirement: SCOPE-06
    verification:
      - kind: integration
        ref: "crates/ontoforge/tests/cli_scope.rs#invalid_scope_writes_nothing"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_scope.rs#ontology_differing_from_the_active_definition_is_refused"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_scope.rs#scope_before_reload_asks_for_reload"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_scope.rs#missing_store_section_is_a_configuration_error"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_scope.rs#unusable_store_exits_3"
        status: pass
    human_judgment: false
  - id: D3
    description: "Full sync: removed roles and bindings (including every binding of a removed role) are listed in the plan and deleted by apply; an empty directory plans the removal of everything; a stale plan hash is refused with nothing written or audited"
    requirement: SCOPE-01
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/scope_store.rs#full_sync_removes_bindings_and_roles"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/scope_store.rs#stale_plan_hash_rejected"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_scope.rs#plan_lists_removals_and_a_stale_hash_is_refused"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/scope_store.rs#pg_scope_apply"
        status: pass
    human_judgment: false
  - id: D4
    description: "Plan, after_hash and stored form do not depend on the order of roles, properties, actions, sqlLevels, toolsets, l2 lists or binding subjects, nor on repeated list entries (SCOPE-02 ordering)"
    requirement: SCOPE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/scope_store.rs#order_independent_hash"
        status: pass
    human_judgment: false
  - id: D5
    description: "Roles and bindings are written only through scope apply (source guard), and the admin audit cannot be updated, deleted or (PostgreSQL) truncated"
    requirement: SCOPE-07
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/scope_store.rs#scope_tables_written_only_by_scope_store"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/scope_store.rs#admin_audit_is_append_only"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/scope_store.rs#pg_audit_is_append_only_and_login_is_recorded"
        status: pass
    human_judgment: false
  - id: D6
    description: "apply creates the index a row predicate needs (of_ix_of_obj_flight_departure_airport_code), skips the one that exists, and fails with NoActiveDefinition before any reload"
    requirement: SCOPE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/scope_store.rs#apply_creates_row_filter_indexes"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/scope_store.rs#apply_without_active_definition_fails"
        status: pass
    human_judgment: false
  - id: D7
    description: "First authentication creates a minimal user record (five columns, created_at written once); LoginRecorder writes at most once per interval and principal"
    requirement: SCOPE-01
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/scope_store.rs#record_login_creates_user_once"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/scope_store.rs#login_recorder_writes_once_per_interval"
        status: pass
    human_judgment: false
  - id: D8
    description: "docs/scope.md documents Role and RoleBinding, plan / apply, full sync, the offline check, audit, groups from the JWT groups claim, dev token groups, read-only REST and multi-role visibility"
    requirement: SCOPE-01
    verification:
      - kind: manual
        ref: "docs/scope.md#Role 与 RoleBinding（roles-as-code）"
        status: pass
    human_judgment: true
    rationale: "Prose accuracy against the implementation was checked by reading the code paths; no test asserts the wording"

duration: 1h 50m
completed: 2026-10-20
---

# Phase 3 Plan 16: Scope Plan and Apply Summary

**`ontoforge scope plan|apply` syncs a scope directory into the edit store as a reviewed, hash-bound full sync: offline `compile_scope` first, then roles, bindings, row predicate indexes and one append-only audit record in a single transaction, with a minimal `of_user` record per authenticated principal.**

## Accomplishments

- `EditStore::scope_plan` / `scope_apply` (D-01, D-02): the plan lists added, changed and removed roles, added and removed bindings (every binding of a removed role is listed) and the missing row predicate indexes, with the hash of the stored and of the resulting scope and a plan hash. `scope_apply` re-plans inside the write transaction under `DEFINITION_LOCK_KEY` (PostgreSQL advisory lock plus `SET LOCAL lock_timeout`; SQLite `BEGIN IMMEDIATE`), refuses any other hash with `PlanChanged` and otherwise writes everything and one `of_admin_audit` row (actor, plan JSON, before / after hash) in one commit (T-03-49, T-03-50, T-03-54).
- `normalize_role` / `normalize_scope`: list fields sorted and deduplicated before storage and hashing, subjects sorted by kind and name, empty bindings dropped (SCOPE-02 ordering).
- Migrations `0003_scope.sql` for SQLite and PostgreSQL: `of_user`, `of_role`, `of_role_binding` and `of_admin_audit`; the audit table is append only with the 03-09 trigger pattern, including the PostgreSQL TRUNCATE trigger (T-03-50).
- `index::role_index_steps`: plain `CREATE INDEX IF NOT EXISTS` for the properties row predicates read, skipping the primary key and indexes the physical schema or the database already has. Indexes are only ever added.
- Source guard test: only `scope_store.rs` names the three scope tables (D-03, T-03-52). `record_login` / `LoginRecorder` write the five-column `of_user` row, `created_at` on insert only, at most once per 60 s per principal in a process (D-04, T-03-53).
- CLI `ontoforge scope plan|apply` with human and JSON output (camelCase), exit codes 0 / 1 / 3, and the offline check before the store is opened. `examples/server/ontoforge-server.dev.toml` and the `docs/scope.md` section make the aviation example runnable end to end.

## Task Commits

| Task | Name | Commit |
|------|------|--------|
| 1 RED | tracer tests, 0003 migrations, example config, stub scope_store / scope.rs | 8478ba1 |
| 1 GREEN | scope_store plan / apply / login, index reconciliation, sql_exit locks, CLI scope | 80fa071 |
| 2 RED | full sync, stale hash, order, audit, source guard, login, PG and CLI cases (pass on arrival, see deviation 3) | 258ca37 |
| 2 docs | Role and RoleBinding section in docs/scope.md | d084d6f |

## Deviations from Plan

### Auto-fixed / design adjustments

**1. [Design] `scope_plan` without an active definition returns a plan, not an error**
- **Found during:** Task 1 design
- **Issue:** The interface gives `scope_plan` a `StoreError` only, but index DDL needs the active physical schema.
- **Fix:** With no active definition the plan has no index DDL; `scope_apply` returns `NoActiveDefinition`; the CLI refuses both with the reload hint. `apply_without_active_definition_fails` covers it.
- **Commit:** 80fa071

**2. [Interface] Additions to the sketched interface**
- `normalize_scope`, `ScopePlan::is_empty`, camelCase `serde` on `ScopePlan`, `sql_exit::{fetch_all_cells, existing_index_names}` and `reload::server_settings` (the CLI needs `[store]` and `[auth]` from one config load). `of_role_binding.role` has a foreign key on `of_role(api_name)`. The `EditStore.settings` `allow(dead_code)` of 03-02 is gone because `lock_timeout` is read now.
- **Commit:** 8478ba1, 80fa071

**3. [Scope] Task 2 behaviour was implemented in the Task 1 GREEN commit**
- Full sync, hash check, normalization, login and the audit triggers came with the tracer. Task 2's RED commit is therefore tests that passed on arrival (one test fixture bug found and fixed in them: the loader rejects a duplicate subject with ONT-V21, so the repeated entry moved out of `ROLES_B`). To show the guards can fail, `scope_store.rs`, the SQLite migration and `scope.rs` were mutated and restored: dropping `normalize_role`'s sorting failed `order_independent_hash`; skipping binding deletes failed `full_sync_removes_bindings_and_roles`; disabling the hash compare failed `stale_plan_hash_rejected`; a recorder that never throttles failed `login_recorder_writes_once_per_interval`; updating `created_at` on conflict failed `record_login_creates_user_once`; removing the audit triggers failed `admin_audit_is_append_only`; naming `of_admin_audit` in `index.rs` failed `scope_tables_written_only_by_scope_store`; skipping the active-definition comparison failed `ontology_differing_from_the_active_definition_is_refused`.

**4. [Rule 1 - Bug] `docs/scope.md` showed a role layout the schema rejects**
- **Found during:** Task 2 docs
- **Issue:** The rowFilter example and the sentence before it used `grants.objectTypes.<Type>`; `Role` has `objectTypes` at the top and `deny_unknown_fields`.
- **Fix:** Example and sentence now use `objectTypes.<Type>.rowFilter`.
- **Commit:** d084d6f

**5. [Test maintenance] `reload_pg.rs` lists the new system tables**
- The ignored PostgreSQL test asserts the exact system table set; it now includes `of_admin_audit`, `of_role`, `of_role_binding` and `of_user`.
- **Commit:** 80fa071

**Total deviations:** 5 (1 design, 1 interface, 1 TDD scope, 1 bug fix in docs, 1 test maintenance). **Impact:** none on behaviour or on the plan's truths.

## TDD Gate Compliance

Task 1: RED (8478ba1) failed on the assertions of three storage tests and two CLI tests against stubs (empty plan, 0 rows, exit 1 without output); GREEN is 80fa071. Task 2: no independently red tests, see deviation 3; mutation checks recorded there. No REFACTOR commit.

## Known Stubs

None.

## Threat Flags

None beyond the plan. `EditStore::record_login` and `LoginRecorder` are the new write path for `of_user`; nothing calls them yet (the server is Phase 7), and they store only the five columns of D-04. T-03-51 (CLI operator identity is the OS user) stays accepted as the plan says.

## Verification

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`: clean, all passing (storage `tests/scope_store.rs` 11 passed, 2 ignored; `cli_scope.rs` 8 passed).
- PostgreSQL (embedded server, Windows 11, no Docker): `cargo test -p ontoforge-storage --locked --features embedded-pg --test scope_store --test reload_pg --test migrate_pg --test conformance_pg -- --ignored`: scope_store 2 passed, reload_pg 5 passed, migrate_pg 3 passed, conformance_pg 27 passed.
- `sql_exit_guard` passes: no SQL text outside `sql_exit.rs`.

## Issues Encountered

- Not covered by a test: two concurrent `scope apply` calls on PostgreSQL (the advisory lock is taken and `lock_timeout` set, but no test races two applies); `sql_exit::set_local_lock_timeout` with `lock_timeout` of zero.
- Plan 03-16 runs `ontoforge scope` against whatever store the config names; the example config writes under `target/ontoforge-dev/`, which must exist before `ontoforge reload` (SQLite creates the file, not the directory).

## Next Phase Readiness

03-18 can wire `advisory_xact_lock(DEFINITION_LOCK_KEY)` and `set_local_lock_timeout` into `reload` and `migrate`, and call `index::role_index_steps` for the reload-side reconciliation. Phase 4 reads roles through `EditStore::load_scope` and `compile_scope` (stored roles go through `sources: None`). The server (Phase 7) calls `LoginRecorder::record` after authentication.

## Self-Check: PASSED
