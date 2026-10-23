# Phase 05: Online Action - Pattern Map

**Mapped:** 2026-10-23  
**Files analyzed:** 16 candidate new/modified files  
**Analogs found:** 14 / 16 candidate files

## File Classification

The paths below are planning candidates inferred from `05-CONTEXT.md` and `05-RESEARCH.md`, not a mandated split. The planner may consolidate modules while preserving ownership boundaries.

| New/Modified File | Role | Data Flow | Closest Tracked Analog | Match |
|---|---|---|---|---|
| `crates/ontoforge-action/src/lib.rs` | provider | request-response | `crates/ontoforge-query/src/lib.rs` | role-match |
| `crates/ontoforge-action/src/execute.rs` | service | request-response / batch | `crates/ontoforge-query/src/lib.rs` | data-flow match |
| `crates/ontoforge-action/src/params.rs` | utility | transform | `crates/ontoforge-schema/src/ir/action.rs` | partial |
| `crates/ontoforge-action/src/rules.rs` | service | transform / batch | `crates/ontoforge-storage/src/commit.rs` | data-flow match |
| `crates/ontoforge-action/src/criteria.rs` | service | request-response | `crates/ontoforge-auth/src/scope/merge.rs` | partial |
| `crates/ontoforge-action/src/outbox.rs` | service | event-driven | none | none |
| `crates/ontoforge-action/Cargo.toml` | config | build | `crates/ontoforge-query/Cargo.toml` | role-match |
| `crates/ontoforge-query/src/lib.rs` | service | request-response | itself | exact |
| `crates/ontoforge-storage/src/commit.rs` | service | CRUD / batch | itself | exact |
| `crates/ontoforge-storage/src/store.rs` | provider | CRUD | itself | exact |
| `crates/ontoforge-storage/src/sql_exit.rs` | utility | CRUD | itself | exact |
| `crates/ontoforge-storage/migrations/sqlite/0004_action.sql` | migration | CRUD | `crates/ontoforge-storage/migrations/sqlite/0002_edit_log.sql` | exact |
| `crates/ontoforge-storage/migrations/postgres/0004_action.sql` | migration | CRUD | `crates/ontoforge-storage/migrations/postgres/0002_edit_log.sql` | exact |
| `crates/ontoforge-storage/tests/conformance/mod.rs` | test | CRUD / batch | itself | exact |
| `crates/ontoforge-action/tests/action.rs` | test | request-response | `crates/ontoforge-query/tests/pk_read.rs` | role-match |
| `crates/ontoforge-action/tests/trino_action_it.rs` | test | request-response | `crates/ontoforge-query/tests/trino_query_it.rs` | role-match |

`05-RESEARCH.md` also proposes an Action PostgreSQL race test and possible worker restart test. Their final paths are planner discretion; use the PG conformance and query integration analogs below. No new `schema` or `auth` source file is required by the current IR contract.

## Pattern Assignments

### Action runtime: `lib.rs`, `execute.rs`, `params.rs`, `rules.rs`, `criteria.rs`

**Analogs:** `crates/ontoforge-query/src/lib.rs:225-320`, `crates/ontoforge-schema/src/ir/action.rs:16-31,37-48,98-115,155-193`, `crates/ontoforge-auth/src/scope/merge.rs:101-115`, `crates/ontoforge-storage/src/commit.rs:53-102,615-664`.

**Inputs and rule order** (`schema/src/ir/action.rs:23-31,98-115`):

```rust
pub parameters: IndexMap<ParameterName, ParameterDef>,
pub rules: Vec<Rule>,
pub submission_criteria: Vec<SubmissionCriterion>,
```

`Rule` variants already cover `CreateObject`, `ModifyObject`, `DeleteObject`, `AddLink`, `RemoveLink` (`action.rs:105-115`); `ValueSource` covers `Param`, `Static`, `CurrentUser`, `CurrentTime` (`:158-166`). Expand batch in item order then rule order, preserving an item/rule/edit ordinal map; do not define a second rule IR. `ParameterDef` has `param_type`, `items`, `required`, and `default` (`:37-48`); runtime errors must be per parameter. Use `BaseType::check_literal` in `crates/ontoforge-schema/src/ir/basetype.rs:99-165` for base values, and check object references against scoped reads.

**Scope guard** (`auth/src/scope/merge.rs:101-115`):

```rust
let visible = eval(&self.row, row, attrs).is_true();
let columns = if visible {
    self.columns.iter()
        .filter(|(_, p)| eval(p, row, attrs).is_true())
        .map(|(c, _)| c.clone()).collect()
} else { BTreeSet::new() };
```

Use this actual row-specific `RowVisibility` for criteria reads, pre-state write columns, and post-state checks. The static criteria coverage check in `crates/ontoforge-auth/src/scope/coverage.rs:20-50` is a definition-time guard, not the runtime row check. Keep hidden target and absent target on the same public error path.

**Batch result** (`storage/src/commit.rs:95-102,646-663`): `Receipt { seq, event_id, versions }` follows original `Commit::edits` order even though rows lock in key order. Map `versions` back through the recorded item/edit ordinals for the batch receipt. No existing Action runtime implements the full ①-⑨ pipeline; `execute.rs` is a new orchestration boundary.

### Query integration: `crates/ontoforge-query/src/lib.rs`

**Token codec** (`:153-179,215-223`):

```rust
let message = version_message(fields)?;
jsonwebtoken::crypto::sign(
    &message, &EncodingKey::from_secret(&self.key), Algorithm::HS256,
)
```

`version_message` length-prefixes each field with `u64::to_be_bytes`. Extract one versioned, canonical input builder shared by query issuance and Action verification. The current `version_for` at `:334-365` privately uses `format!("{:?}", self.scope)` and therefore is not a stable format to duplicate.

**Conditional source read** (`:260-284,388-424`):

```rust
let source = if ty.backing == Backing::Source && !all_patched {
    charge(&mut calls, 1, self.limits.max_backend_calls)?;
    self.read_source(&object_type, &canonical_pk).await?
} else { None };
```

`read_source` uses `limit: Some(2)` (`:409`) and rejects `output.rows.len() > 1` as `AmbiguousObject` (`:423-424`). Reuse this sampling decision for ACTION-12. Distinguish engine unavailable from other source errors when exposing Action errors; current query maps all execution failures to `QueryError::Source` (`:415-419`).

### Storage transaction: `commit.rs`, `store.rs`, `sql_exit.rs`

**Current transaction and rollback** (`commit.rs:142-163`):

```rust
check_shape(&commit)?;
let mut tx = self.begin_write().await?;
let result = match self.fence(&mut tx, commit.def_version).await {
    Ok(def) => match validate(&def, &commit) {
        Ok(plan) => write(&mut tx, &def, &commit, plan).await,
        Err(error) => Err(error),
    },
    Err(error) => Err(error),
};
match result {
    Ok(receipt) => { sql_exit::commit(tx).await?; Ok(receipt) }
    Err(error) => { sql_exit::rollback(tx).await; Err(error) }
}
```

Extract/reuse the private steps inside a storage-owned Action entry point. `WriteTx` is crate-private (`store.rs:141-148`); retain that restriction. The definition fence is explicitly the first write-transaction statement (`commit.rs:165-186`), and SQLite takes its writer lock at `BEGIN IMMEDIATE` (`sql_exit.rs:53-69`). Resolve command claim versus fence ordering in the plan without silently changing the existing fence invariant.

**Stable locks and one log** (`commit.rs:218-262,521-543,615-664`): `RowKey` orders `(kind, name, canonical pk)`; `sort_plan` is stable, so repeated edits to one row remain ordered. `lock_and_advance` loads each row once and advances each edit in memory. `write` creates one `event_id`, appends one header, writes final rows, then restores original ordinal order for log/history entries. The Action path must compare the locked row versions with pre-read observations and run scope/criteria checks before completing that same transaction. A validate-only attempt must roll back the version-0 skeleton inserted by `load_row` (`:770-831`).

**SQL boundary** (`sql_exit.rs:1-37,163-184`): all SQL text reaches `sqlx` through `sql_exit`; use `sea-query` builders and `sql_exit::build`, `execute_built`, and typed fetch functions for command/outbox operations. The `tests/sql_exit_guard.rs` guard explicitly prevents new raw SQL execution elsewhere.

### Migrations: SQLite and PostgreSQL `0004_action.sql`

**Analogs:** `crates/ontoforge-storage/migrations/sqlite/0002_edit_log.sql:8-27` and `crates/ontoforge-storage/migrations/postgres/0002_edit_log.sql:9-28`.

```sql
CREATE TABLE of_edit_log (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id TEXT NOT NULL UNIQUE,
    actor TEXT NOT NULL,
    action TEXT NOT NULL,
    committed_at TEXT NOT NULL,
    def_version INTEGER NOT NULL,
    params TEXT NOT NULL
);
```

SQLite uses `INTEGER`/JSON `TEXT`; PostgreSQL uses `bigint`/`jsonb`/`timestamptz` (`postgres/0002_edit_log.sql:9-17`). Give command rows a database unique `(principal_id, command_id)` constraint and outbox rows a stable `event_id` key, with receipt/payload stored in the appropriate backend JSON representation. Preserve the existing one-log-per-commit invariant and append-only log rules; command expiry must not delete log/outbox history.

### Tests: conformance, Action integration, Trino integration

**Dual backend** (`storage/tests/conformance_sqlite.rs:5-8`, `conformance_pg.rs:9-15`): both invoke `conformance::all_cases!`, with PG cases ignored until an actual server is supplied. Add shared atomicity/idempotency cases in `storage/tests/conformance/mod.rs`; keep real two-connection PG races in an ignored PG integration target and execute them in CI. The current harness has `sqlite()` and `pg()` constructors (`conformance/mod.rs:43-47`) and `commit_ok` (`:386`).

**Fake source engine** (`query/tests/pk_read.rs:39-79`): `StaticEngine` stores rows, counts calls with `AtomicUsize`, records SQL, and implements `Engine::execute`. Copy this fixture shape to prove duplicate detection, source-required failure, and zero engine calls for all-patched reads. The Action Trino test should mirror the tracked `query/tests/trino_query_it.rs` target and its ignored integration convention.

## Shared Patterns

### Authentication and disclosure

`crates/ontoforge-auth/src/scope/merge.rs:101-115` is the row/column visibility primitive; `crates/ontoforge-auth/src/scope/coverage.rs:20-50` detects criteria property reads at definition time. Action replay and `get_command_result` need current action-grant authorization before returning the stored audit-only receipt. Deleted/released targets cannot be re-evaluated from a current row; apply the D-02 audit-summary policy and never return previous property values.

### Errors and rollback

`crates/ontoforge-storage/src/commit.rs:104-133` defines typed edit errors; `:153-161` rolls back on every failure. Map hidden/not-found and masked criteria to public non-disclosing Action errors, retain `VersionConflict`, `CommandIdReused`, `AmbiguousObject`, and `EngineUnavailable` as distinct contracts. Do not expose `EditError` messages containing hidden state directly.

### Database and event identity

`crates/ontoforge-storage/src/sql_exit.rs:1-37` is the sole SQL exit. `crates/ontoforge-storage/src/commit.rs:628-663` generates one `event_id` and returns it with log sequence and versions. Store that same ID with the durable outbox payload, then send network requests only after commit.

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| `crates/ontoforge-action/src/outbox.rs` | service | event-driven | No existing webhook lease/retry worker; use RESEARCH.md ACTION-11 contract and existing storage transaction/SQL patterns. |
| Action command fingerprint helper (path chosen by planner) | utility | transform | No existing canonical client-request fingerprint; do not reuse `_version` HMAC input or JSON key-order-dependent text. |

## Metadata

**Analog search scope:** tracked `crates/ontoforge-action`, `ontoforge-auth`, `ontoforge-schema`, `ontoforge-query`, `ontoforge-storage`, `ontoforge-expr`; migration and test directories.  
**Tracked-source gate:** analog paths above appear in `git ls-files`; no runtime mirror path used.  
**Pattern extraction date:** 2026-10-23.
