---
phase: 03-edit-store-scope-model
reviewed: 2026-10-21T12:40:00Z
depth: standard
files_reviewed: 90
partitions: [A storage write path, B storage reload/DDL/CI, C auth/expr, D CLI/engine/schema/docs]
findings:
  critical: 4
  warning: 28
  info: 25
  total: 57
status: issues_found
---

# Phase 03 代码审查报告

范围 146 个变更文件（diff base `d0a7fdba^`），按模块拆成 4 个分区并行审查，均为 standard 深度、只读代码未运行测试。
finding 编号带分区前缀（CR-A01 = 分区 A 的 CR-01），各分区原始 files_reviewed_list 见下方各节。

| 分区 | 范围 | Critical | Warning | Info |
|---|---|---|---|---|
| A | storage 写入路径、edit log、history、import、value、migrations | 1 | 5 | 6 |
| B | storage reload、migrate、DDL、naming、scope_store、sql_exit、CI | 1 | 10 | 4 |
| C | ontoforge-auth、ontoforge-expr | 0 | 7 | 5 |
| D | CLI、engine printer/sqlite、schema loader/IR、JSON schema、docs | 2 | 6 | 10 |


# 分区 A


## Phase 03 Part A: Code Review Report

**Reviewed:** 2026-10-21
**Depth:** standard (read every listed file in full; read `state.rs`, `sql_exit.rs` helpers, `reload.rs` switch path and `ddl.rs` system columns only to confirm or refute findings)
**Files Reviewed:** 17
**Status:** issues_found

### Summary

The write path is sound where it matters most. The `def_version` fence is correct: it is the first statement, it is `FOR SHARE` on PostgreSQL and `BEGIN IMMEDIATE` on SQLite, and `activate_definition` / `lock_def_state` order the state-row lock before DDL, so the deadlock the fence could cause is already handled. Rows are locked in a deterministic `RowKey` order. `seq` is allocated after the row locks. The header is written before its entries and snapshots, so the FK holds. The append-only triggers cover UPDATE, DELETE, TRUNCATE (PG) and `REPLACE` (SQLite with `recursive_triggers`). All identifiers go through `sea_query::Alias` or physical-schema names, and every value is bound or cast, so I found no SQL injection. History masking is correct in code: `visible` is the only gate between a stored value and the response, and `params_query` is only built for `CallerKind::Admin`. I found no panics on untrusted input in these files.

The defects are at the validation boundary and in operational robustness:

- `PropertyValue` accepts values that PostgreSQL later rejects, and it silently rounds JSON decimals through `f64`.
- The commit path has no lock timeout.
- PostgreSQL text reads depend on session settings.
- Commit size is unbounded and the history read can fail permanently on it.

Already-known gaps are not repeated: the create-only race in import, SQLite "database is locked" not mapped to LockTimeout, refusing 1:1 key swaps in one commit, and the PG `seq` gap caveat (documented in ADR-0026 section 5).

### Critical Issues

#### CR-A01: JSON numbers for `decimal` properties are routed through `f64` and silently rounded

**File:** `crates/ontoforge-storage/src/value.rs:326` (also the module contract at `:58-60`)
**Issue:** `from_json_as` reads a `decimal` given as a JSON number with `Value::Number(n) => V::Decimal(n.to_string())`. The workspace `serde_json` does not enable `arbitrary_precision` (`Cargo.toml:13` has only `preserve_order` and `raw_value`). A non-integer JSON number has therefore already been parsed into an `f64` by the time it reaches this line, and `to_string()` prints the shortest round-trip form of that `f64`.

Example: for a `decimal(38, 10)` property, the client JSON `12345678.1234567891` becomes the `f64` `12345678.123456789`. `normalize_decimal` then pads it to `12345678.1234567890`. The check passes and the 10th digit is silently wrong, written to the edit table, the append-only log and the history. Any number with more than about 15 to 17 significant digits is altered. `MAX_PRECISION` for decimals is 38 (`lint/binding/e13.rs`), so this is reachable by design.

This breaks the stated invariant that "A value that does not fit is an error and is never repaired by rounding or truncating." It is silent corruption of an immutable audit record. Phase 5 REST and MCP bodies and `ontoforge import` JSON files all arrive as `serde_json::Value`, so they all pass through here. The unit test `json_round_trips_every_type` only covers `12.5`, which is exact.

**Fix:** Do not accept a float-typed JSON number for a decimal unless it is provably exact. The least invasive option is to accept a JSON number only if it is an integer (`n.is_i64() || n.is_u64()`, which serde_json keeps exactly) and otherwise require the string form:
```rust
BaseType::Decimal => match v {
    Value::String(s) => Ok(V::Decimal(s.clone())),
    Value::Number(n) if n.is_i64() || n.is_u64() => Ok(V::Decimal(n.to_string())),
    Value::Number(_) => fail("send a decimal as a string such as \"12.50\"; a JSON fraction would pass through a binary float"),
    _ => wrong("a decimal string or number"),
},
```
The alternative is to enable `serde_json/arbitrary_precision` workspace-wide and keep the number's text. That has wider effects, so evaluate it separately. Add a test with a value of 17 or more significant digits.

### Warnings

#### WR-A01: Values that pass `PropertyValue::canonical` are rejected by PostgreSQL (NUL characters, year 0000)

**File:** `crates/ontoforge-storage/src/value.rs:177` (String), `:182` and `:264-268` (date), `:272-299` (timestamp); `crates/ontoforge-storage/src/commit.rs:656` (params); `crates/ontoforge-storage/src/import.rs:137-158`
**Issue:** The validator is backend-agnostic, but PostgreSQL has two limits that SQLite does not:

1. `text`, `text[]` and `jsonb` cannot hold U+0000. A string property, a string primary key, an array element or a nested string in `Commit::params` that contains `\0` passes validation. PG then fails with SQLSTATE 22021 (text) or 22P05 (jsonb, via the log `payload`, `pk`, `params` or the history `snapshot`). SQLite stores it happily.
2. PostgreSQL has no year 0. `parse_date("0000-01-01")` returns `Ok` because chrono accepts year 0, and `DateTime::parse_from_rfc3339("0000-01-01T00:00:00Z")` does too. A `+05:00` offset can also push `0001-01-01` into year 0 after the UTC conversion. PG rejects the cast with 22008.

Effects:
- The failure surfaces as an opaque `EditError::Store(StoreError::Query)` after validation, not as `EditError::Value`. The two backends disagree on what is valid.
- It breaks the import promise (`import.rs:133-134`) that the whole file is checked before anything is written. A NUL in row 5,000 of a CSV makes batch 5 fail on PG, and batches 0 to 4 stay committed (`ImportError::Partial`).
- Because `params` is stored "as it is" and unvalidated, a NUL in any Action parameter makes that commit unwritable on PG.

**Fix:** Validate to the intersection of both backends in `canonical_as` and in `check_shape`. Reject `\0` in `V::String` and in array string elements. Reject a date or timestamp with year below 1. Walk `commit.params` and reject any string or key containing `\0`:
```rust
(BaseType::String, V::String(s)) if s.contains('\0') => fail("a string cannot contain U+0000"),
```
Add a conformance case that runs on both backends.

#### WR-A02: The commit path has no lock timeout, so a blocked commit pins one of five pool connections

**File:** `crates/ontoforge-storage/src/commit.rs:142-163`, `:180-186` (fence `FOR SHARE`), `:788` (`FOR UPDATE`); `crates/ontoforge-storage/src/store.rs:260-263` (pool `max_connections(5)`)
**Issue:** `reload`, `migrate` and `scope apply` call `set_local_lock_timeout` from `[store] lock_timeout_ms`. `commit()` and `import` never do. On PostgreSQL a commit waits with no bound at three points:
- the fence `FOR SHARE` on `of_def_state`, while a `migrate` that holds the state row `FOR UPDATE` for a long data step;
- `ON CONFLICT` and `SELECT … FOR UPDATE` on a row held by a long transaction;
- table locks behind DDL.

Each waiting commit holds a pooled connection. Five stuck commits exhaust the pool. Every other call, including reads such as `history`, then fails after the default 30 s acquire timeout. The `lock_timeout` setting exists and is documented as protecting the system from exactly this, but the hot path ignores it.

**Fix:** In `commit()`, right after `begin_write` and before `fence`, call `sql_exit::set_local_lock_timeout(&mut tx, millis)` using `self.settings.lock_timeout`. Map SQLSTATE 55P03 to a retryable `EditError` variant, for example `EditError::LockTimeout`, so callers can answer 503 and not 500. Do the same in `existing_objects` / `existing_edges`.

#### WR-A03: PostgreSQL value reads depend on session settings that the connection does not pin

**File:** `crates/ontoforge-storage/src/commit.rs:1023-1030` (via `sql_exit::column_text` / `array_text`); `crates/ontoforge-storage/src/store.rs:257-259`
**Issue:** `timestamp_text` and `array_text` go out of their way to be independent of the session time zone (`AT TIME ZONE 'UTC'`). The other text reads are not independent of the session:
- `date::text` and `date[]::text[]` follow `DateStyle`.
- `double precision::text`, `real::text` and the matching array casts follow `extra_float_digits`. With a role- or database-level `extra_float_digits = 0` (older default, still common via `ALTER ROLE`), floats come back at 15 significant digits.

`open_postgres` only sets `search_path`. A deployment that has `datestyle = 'SQL, DMY'` or `extra_float_digits = 0` set on the role or database gets one of two outcomes:
- A decode failure (`StoreError::Corrupt`, "a stored value is not valid"). Every write to a type with a date column then fails.
- Silently shortened doubles. `lock_and_advance` loads the shortened values, and `RowState::snapshot()` writes them into the append-only `of_edit_history` snapshot for every untouched array or double column.

**Fix:** Pin the options on every connection in `open_postgres`:
```rust
.options([
    ("search_path", schema_sys),
    ("datestyle", "ISO"),
    ("extra_float_digits", "3"),
    ("timezone", "UTC"),
])
```
Verify that sqlx quotes option values correctly, or use an `after_connect` hook with `SET`.

#### WR-A04: A commit has no size bound, and the history read fails permanently for a large commit

**File:** `crates/ontoforge-storage/src/commit.rs:281-302` (`check_shape`); `crates/ontoforge-storage/src/history.rs:159-163`, `:360`
**Issue:** `check_shape` only requires `edits.len()` to fit in `i32`. It does not bound `params` size, `action` length, the number of edits or the pk and string sizes. The log is append-only (D-12), so one oversized commit is permanent. Any Action caller can write megabytes of `params` into every record and nothing can reclaim the space.

A second consequence is in `history()`. `seqs` is built from `located` without deduplication. A commit with N edits to the same object puts that `seq` N times into `is_in(seqs)`. With more than 32,766 (SQLite) or 65,535 (PG) bound parameters, `entry_query` fails. The history of that object then cannot be read, and the failure is permanent because the log cannot be changed. `import` caps a batch at 10,000, but `Store::commit` is a public API with no cap.

**Fix:**
- Add limits in `check_shape`: maximum edits (for example 10,000, the import maximum), `params.to_string().len()` (for example 1 MiB) and `action.len()` (for example 256).
- Deduplicate `seqs` in `history()` (`BTreeSet` or `dedup()`).
- Consider chunking the `IN` lists.

#### WR-A05: `OnExisting::Update` cannot update a subset of columns or clear a value

**File:** `crates/ontoforge-storage/src/import.rs:616-644` (`finish`), `:689-697` (`header`), `:727` (empty cell), `:760` (JSON null)
**Issue:** The documentation says `Update` will "Set the cells of the file on a live object". The reader does not know the mode. `header()` reports every required property that has no column as a problem, and `finish()` reports every row whose required cell is empty. Both apply even when the object already exists and the mode is `Update`.

An update file can therefore only be a full-row file that includes every required column with a value. An update such as `id,capacity` fails with "the required column `name` is missing". An empty CSV cell or JSON `null` means "not set" and is dropped, so a file cannot clear an optional property either. The user is told to use `update` in the error text (`decide`, `:1127-1129`), but it does not do what that text suggests.

**Fix:** Pass the mode into the readers. In `Update` mode, treat required columns as optional per row and enforce "required" only for rows that `decide` classifies as `Create`. Document that an empty cell leaves a value alone, or add an explicit clear marker. If this is deliberately out of scope, change the `OnExisting::Update` doc and the error text to say "full rows only".

### Info

#### IN-A01: Import takes a write transaction (`BEGIN IMMEDIATE`) only to read

**File:** `crates/ontoforge-storage/src/import.rs:1152`, `:1219`
**Issue:** `existing_objects` / `existing_edges` call `begin_write`, run up to 2,000 `SELECT` chunks (1M rows at 500 keys per chunk), then roll back. On SQLite that holds the single write lock for the whole lookup, so every other writer waits (and hits the 5 s busy timeout) while an import scans. Nothing is written in these transactions.
**Fix:** Use a plain read connection or transaction (a `begin_read` helper), as `history` and `read_log` do through the pool.

#### IN-A02: "A principal id never starts with `system:`" is documented but not enforced

**File:** `crates/ontoforge-storage/src/commit.rs:27-31` vs `:291-300`
**Issue:** `check_shape` accepts any `{issuer}|{subject}` with both parts non-empty, so `Principal("system:import|x")` is a valid actor stored as `system:import|x`. It cannot equal the exact reserved string, but any consumer that classifies system actors by the `system:` prefix would misclassify it.
**Fix:** Reject `id.starts_with("system:")` in `check_shape`, or drop the claim from the doc comment.

#### IN-A03: `read_log` and `replay` are public, unmasked and not marked as administration-only

**File:** `crates/ontoforge-storage/src/log.rs:172`, `:374`
**Issue:** `read_log` returns every record, including raw `params` and the new values of every column of every object, with no `CallerKind` or scope parameter. It is correct for replay, backup and audit tooling. D-13 says masking and `params` stay out of reach of ordinary callers, and nothing in the type or the doc comment says this API is outside that contract. A future REST or MCP handler could wire it to a normal route.
**Fix:** Add a doc line "unmasked; administration channel only". Optionally gate it behind a marker type, for example `AdminHandle`.

#### IN-A04: Import diagnostics echo raw cell and key text, including control characters

**File:** `crates/ontoforge-storage/src/import.rs:250-256`, `:656`, `:756`, `:942`
**Issue:** `shown()` truncates to 40 characters but does not strip control characters. A cell, unknown column name or JSON key containing ANSI escape sequences or newlines is printed verbatim into CLI diagnostics. Duplicate JSON object keys are resolved silently (last wins) by `serde_json::Value`, so a row with two `"price"` keys imports the second without a warning.
**Fix:** In `shown()`, replace `char::is_control` with `\u{FFFD}` or the escaped form. For duplicate keys, parse with a visitor that detects them, or document the behaviour.

#### IN-A05: Dead and misleading code in the commit path

**File:** `crates/ontoforge-storage/src/commit.rs:200-202` vs `:306-311`; `:288`, `:282-286`
**Issue:**
- `fence` returns `DefinitionStale` when `expected != active`, then `validate` repeats the same comparison with the same values. The second check can never fire.
- `EditError::Value { property: "edits" | "action" | "actor" | "set" | "unset" }` puts non-property names in the `property` field, so API consumers cannot tell a property from a commit-level field.
- `check_shape` stores the untrimmed `action` after testing the trimmed one.

**Fix:** Delete the second staleness check in `validate`. Add a `Shape(String)` variant, or a distinct error, for commit-level problems. Store the trimmed action or reject surrounding whitespace.

#### IN-A06: PostgreSQL append-only triggers are skipped under `session_replication_role = replica`

**File:** `crates/ontoforge-storage/migrations/postgres/0002_edit_log.sql:46-69`, `0003_scope.sql:41-45`
**Issue:** The triggers are created as plain `ENABLE` (origin) triggers. A session that sets `session_replication_role = replica` (superuser, or a role granted that setting) bypasses them. The ADR accepts that the owner can disable triggers and relies on `REVOKE`. This is a cheap extra layer against bulk-restore tooling and logical-replication-style sessions.
**Fix:** Add `ALTER TABLE of_edit_log ENABLE ALWAYS TRIGGER of_edit_log_no_update_delete;` (and the matching statements for the other triggers). Alternatively state in ADR-0026 section 8 that the setting is part of the same ownership boundary.

---

_Reviewed: 2026-10-21_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_

# 分区 B


## Phase 03 Part B: Code Review Report

**Reviewed:** 2026-10-21
**Depth:** standard
**Files Reviewed:** 14 (the two ADRs were read as the spec, not reviewed as code)
**Status:** issues_found

### Summary

Part B covers definition reload, migrate, DDL generation, physical naming, the scope store, the SQL exit and the CI wiring. The identifier handling is sound. Every DDL identifier goes through sea-query. The only hand-built SQL is `CREATE SCHEMA` (double quotes escaped), `PRAGMA table_info` (quoted) and `SET LOCAL lock_timeout` (integer only). I found no injection path. I checked that sea-query's PostgreSQL `DROP INDEX` qualifies the index with the table's schema (sea-query 1.0.2, `backend/postgres/index.rs:99-113`), so the drop-index steps do not silently no-op under the `schema_sys` search_path.

Naming holds up too. Names are ASCII-only (`names.rs:68`), so `bounded`'s byte slice cannot split a character. A bounded name is exactly `max` bytes. ONT-V24 catches the hash and prefix collisions between derived names.

The problems are in the locking and publishing protocol, in what `migrate` assumes about indexes and probes, and in one CI guard:

- One confirmed CI guard that cannot fail (CR-B01).
- A single-flight hole in the probe path of `reload` (WR-B01).
- A lock-order gap in `scope apply` (WR-B02).
- A stale-cache overwrite in `apply_drop_archived` (WR-B03).
- Role-created indexes that `migrate` never tracks (WR-B04).
- Probe expectations that can never be met for array columns after a rename or archive (WR-B05).
- A same-name `RENAME TO` (WR-B06).
- Role checks (D-18) missing on `migrate` and on `scope apply` under the lock (WR-B07, WR-B08).
- An archive suffix that does not equal the recorded `def_version` (WR-B09).
- A fail-open hash default (WR-B10).

The three known gaps from the brief are not repeated.

### Critical Issues

#### CR-B01: The "No OpenSSL" CI guard cannot fail for `openssl`

**File:** `.github/workflows/ci.yml:33`
**Issue:** The step runs under GitHub's default `bash -eo pipefail`. Its first line is `! cargo tree ... -i openssl`. Bash's `errexit` ignores a command whose status is inverted with `!` (the manual: "or if the command's return status is being inverted with !"). If `openssl` is in the graph, `cargo tree` exits 0, `!` turns that into 1, and the script continues. Only the last line decides the step's exit status, and that line checks `native-tls`. So the `openssl` half of the supply-chain gate (RESEARCH Pitfall 5) is inert. I confirmed this with `bash -c 'set -eo pipefail; ! true; echo reached; ! true'`: it prints `reached` and exits 1 only because of the final line.
**Fix:** Make each check an explicit conditional so it does not depend on `!` and `errexit`.
```yaml
run: |
  for pkg in openssl native-tls; do
    if cargo tree --workspace --locked --features ontoforge-storage/embedded-pg -e normal,build,dev -i "$pkg" >/dev/null 2>&1; then
      echo "::error::$pkg is in the dependency graph"
      exit 1
    fi
  done
```

### Warnings

#### WR-B01: Two concurrent identical reloads with a probe are not single-flight; the second fails with `Conflict`

**File:** `crates/ontoforge-storage/src/reload.rs:748-783` (also 417-443)
**Issue:** The `reload` docs say that two reloads of one new definition make one version, and that the second one finds it active and does nothing. This is false in the probe path. Reload A commits the DDL and the `pending` v2, releases the advisory lock, then waits for Trino. Reload B starts in that window. It sees v1 active, re-plans, and re-runs the idempotent DDL. `insert_pending` reuses v2, so B also waits for Trino. A publishes. B's `activate_pending` then sees `active (2) != pending.from_version (1)` and returns `StoreError::Conflict("... run the reload again")`. Every replica that starts or runs `reload` at the same time as another one can get a spurious hard failure, although the definition it wanted is now active.
**Fix:** In `activate_pending`, treat "the pending version is already the active one" as success, and return the definition without error.
```rust
let active = sql_exit::lock_def_state(tx).await?;
if active == pending.def_version {
    return Ok(()); // someone else published the very same version
}
```
`publish` already returns an `ActiveDefinition` built from `pending`, so no other change is needed. Keep the `Conflict` for the case where another version is active.

#### WR-B02: `scope apply` runs DDL without the state-row lock that the other definition changes take

**File:** `crates/ontoforge-storage/src/scope_store.rs:499-546` (compare `sql_exit.rs:659-673`)
**Issue:** The documented lock order is lock_timeout, advisory lock, state row, then DDL. `reload`, `migrate` and `drop-archived` all call `lock_def_state` before DDL. `apply_in` takes the first two locks and then runs `CREATE INDEX` statements (a SHARE lock on the table) without taking the state row. `lock_def_state`'s own comment explains why that matters: a commit holds the fence row and then touches tables, and DDL that already holds a table lock can deadlock with it. For a role that needs indexes on two types T1 and T2:
1. The apply runs `CREATE INDEX` on T1 and now holds SHARE on T1.
2. A multi-type commit has already written T2 (ROW EXCLUSIVE) and now wants T1, so it waits.
3. The apply then runs `CREATE INDEX` on T2 and waits for that commit's ROW EXCLUSIVE.

PostgreSQL's deadlock detector aborts one of the two after `deadlock_timeout`. A user write then fails with 40P01, or the apply fails halfway. Taking the state row first drains in-flight commits and holds new ones at their first statement, which is what makes the other paths safe.
**Fix:** Call `sql_exit::lock_def_state(tx).await?` right after `advisory_xact_lock` in `apply_in`. It is a no-op on SQLite.

#### WR-B03: `apply_drop_archived` overwrites the in-memory definition without `install_if_newer`

**File:** `crates/ontoforge-storage/src/migrate.rs:477`
**Issue:** `*store.active.write().await = Some(Arc::new(active));` is unconditional. `reload` and `apply_migration` use `install_if_newer`. Interleaving:
1. The drop-archived transaction commits v5.
2. A reload commits v6 and installs v6.
3. The drop-archived call takes the write lock and installs v5.

The cache then stays behind the database at v5 until the process restarts. The commit fence refuses every write validated against v5, so writes from this process fail permanently.
**Fix:** Use `store.install_if_newer(&Arc::new(active)).await`, as the other two paths do.

#### WR-B04: Role-created indexes are not in `PhysicalSchema`, so `migrate` leaves them on archived columns

**File:** `crates/ontoforge-storage/src/migrate.rs:981-1014` and `1038-1045`; `crates/ontoforge-storage/src/index.rs`; `crates/ontoforge-storage/src/reload.rs:301-302`
**Issue:** The indexes that `scope apply` and `reload` create for row filters are never recorded in `physical.indexes` (see `role_index_steps`, `existing_index_names`). `archive_column` and `archive_object_table` only drop indexes found in `self.old.objects[..].indexes`. Two consequences:
1. After a type change or removal, the role index stays on the archived column. A `CREATE INDEX IF NOT EXISTS of_ix_<t>_<col>` for the new column of the same name is skipped because the name exists. `missing_role_indexes` also sees the name as present and thinks the new column is indexed. The row filter then reads an unindexed column, against the intent of orchestrator decision 8.
2. On SQLite, `ALTER TABLE ... DROP COLUMN` fails for an indexed column. `plan_drop_archived` and `apply_drop_archived` run `DropArchivedColumn` for `<col>__arch_vN`, and the role index is still there (RENAME COLUMN carries it along). The whole transaction rolls back, and the archived data can never be dropped without a manual `DROP INDEX`. PostgreSQL drops such an index with the column, but the first problem still applies there.

**Fix:** In `archive_column`, `move_property` and `archive_object_table`, also drop any existing role index on the column by its computed name: `naming::index_name(&table.table, &column.column)`, emitted as `DropIndex` (which is `IF EXISTS`). The role steps will recreate it on the new column at the next `reload` or `scope apply`. Alternatively, record role indexes in `physical.indexes` when they are created.

#### WR-B05: `expected_visible` waits for array columns after a rename or archive and can never succeed

**File:** `crates/ontoforge-storage/src/reload.rs:628-633` (compare 614-622)
**Issue:** `AddPropertyColumn` and `CreateObjectTable` skip array columns, because Trino's PostgreSQL connector hides them without `postgresql.array-mapping`. The `ArchiveColumn` and `RenameColumn` arms add `archived` / `to_column` unconditionally. Those steps carry no column type, so the array case cannot be told apart. On the default Trino catalog, migrating (renaming or archiving) an array-typed property commits and then times out. The caller gets `MigrateError::NotVisible`, although the migration is applied and active, and the cause is an array column that Trino will never show.
**Fix:** Add the column type (or an `is_array` flag) to `MigrationStep::ArchiveColumn` and `RenameColumn` and skip arrays in `expected_visible`, as in the add arm. If you want to avoid changing the step format (and the plan hash), look the type up in the physical schema when computing `expected_visible`.

#### WR-B06: A declared object-type rename whose physical table name does not change emits `RENAME TO` with the same name

**File:** `crates/ontoforge-storage/src/migrate.rs:673-695`
**Issue:** `rename_object_types` always emits `RenameTable { from, to }`. Snake casing maps different apiNames to the same table name (`HTTPServer` and `HttpServer` both give `of_obj_http_server`; the code's own comment in `physical.rs` cites `ABTest` / `AbTest`). A case-style rename is the most likely reason to declare a rename. `ALTER TABLE x RENAME TO x` fails on PostgreSQL ("relation already exists") and on SQLite, so the plan can be reviewed but never applied. Without the declaration, the type is seen as removed plus added, and the data does not move. The property path already guards this case (`if rename.from_column.column != rename.to_column.column`, line 837).
**Fix:** Emit `RenameTable` only when `table.table != to_table`. When the names are equal, skip the table step and the index drop/create pair, but still move the entry in `old.objects` and the link ends.

#### WR-B07: `migrate` never runs the stored-role check (D-18 / SCOPE-06)

**File:** `crates/ontoforge-storage/src/migrate.rs:332-371`; `crates/ontoforge/src/migrate.rs:150`
**Issue:** `reload` rechecks the stored roles under the lock (`ReloadOptions::roles`). `apply_migration_probed` takes no role input at all. A migration removes or retypes properties and then activates the new ontology. A stored role whose `rowFilter` or column list names one of those properties stays in the store and now refers to an archived or retyped column. Whether that fails closed depends on later code, and the check that SCOPE-06 describes ("definition reload ... roles must cover the columns their criteria reference, otherwise error and not effective") is skipped on the path that changes the definition the most. The CLI `migrate` does not pass roles either.
**Fix:** Add a `roles: Option<RolesCheck>` parameter to `apply_migration_probed` and run `load_scope_in` + `compile_scope(new, ...)` inside `apply_in`, after `lock_def_state` and before `run`. Refuse the migration (as `ReloadError::Definition` / `MigrateError::Definition`) if a stored role no longer compiles. If it is intentional that migrations skip the check, say so in D-18 and in the `migrate` docs.

#### WR-B08: `scope_apply` trusts that its roles were compiled against the active definition, and nothing checks that under the lock

**File:** `crates/ontoforge-storage/src/scope_store.rs:499-546`; `crates/ontoforge/src/scope.rs:279-308`
**Issue:** The CLI compares the ontology on disk with the active definition in `check_active_definition`. It reads the in-memory copy of `store.active_definition()`, outside any lock and before `scope_apply`. `apply_in` then reads the active definition under the lock but uses it only for `role_index_steps`. That function silently skips a property that `physical` does not contain ("the caller checked", `index.rs`). If a reload or migrate switches the definition in between, roles that no longer compile are written without error and without an index. The plan hash does not catch this when the missing property only makes the index list shorter. The same window exists inside `reload` itself: with a probe, the role check ran under the first lock, and `publish` activates the pending version later without rechecking. A `scope apply` that lands in that gap is checked against the old ontology only. The risk is small because the change is additive.
**Fix:** Pass the ontology (or its serialized hash) that the scope was compiled against into `scope_apply`, and in `apply_in` compare it with `active.ontology` using `same_ontology`. Return an error like `PlanChanged` (or a new `DefinitionChanged`) when they differ. For the probe window, `activate_pending` could re-run the role check with the same `RolesCheck`.

#### WR-B09: The archive suffix is predicted as `active.def_version + 1`, not the version that is recorded

**File:** `crates/ontoforge-storage/src/migrate.rs:520`, `995-1011`; `crates/ontoforge-storage/src/sql_exit.rs:483-491`
**Issue:** `compute` sets `version = active.def_version + 1` and uses it for the `__arch_vN` names and for `ArchivedColumn::def_version` / `ArchivedTable::def_version`. `switch_definition` allocates the real version with `MAX(def_version) + 1`. The two differ whenever a `pending` or `superseded` row above the active one exists. That is the state the known gap "failed probe leaves its pending version" produces. Example: active 2, stuck pending 3, migrate. The columns are named `__arch_v3`, the recorded `def_version` of the archive entries is 3, and the definition row is 4. ADR-0024 point 3 defines N as the `def_version` that triggered the archive, and ADR-0026 replay reads these steps across versions. The names stay unique (versions only grow), so nothing collides, but the recorded `def_version` is wrong and any lookup by it will mismatch.
**Fix:** Allocate the version with `next_def_version` inside the transaction before computing the plan, and pass it into `compute`. Or make `compute` take `max(active.def_version, max pending) + 1` from the same read. Add a test with a stuck pending version.

#### WR-B10: `plan_drop_archived` turns a hash failure into an empty hash

**File:** `crates/ontoforge-storage/src/migrate.rs:457`
**Issue:** `plan_hash(...).unwrap_or_default()` yields `""` if serialization fails. `apply_drop_archived` compares the caller's hash with `plan.hash`, so a caller that passes `""` would then pass the check for a plan that deletes archived data. That is a fail-open default on the one operation that irreversibly deletes data. Serialization failure is unlikely, but the choice of failing open is wrong, and `plan_migration` returns an error for the same condition (lines 549-553).
**Fix:** Make `plan_drop_archived` return `Result<MigrationPlan, Vec<Diagnostic>>` (or `StoreError`), and have `check_hash` reject an empty expected hash.

### Info

#### IN-B01: `activate_definition` ignores the row count of the state update

**File:** `crates/ontoforge-storage/src/sql_exit.rs:650-656`
**Issue:** The version row's update is checked (`!= 1` is an error), but the `of_def_state` update returns `execute_built(...).map(|_| ())`. If the singleton row `id = 1` were missing, the version row would read `active` while `of_def_state` still points at the old version. Every caller takes `lock_def_state` first, which would fail on a missing row, so this is defensive only.
**Fix:** Check `rows_affected() == 1` and return `StoreError::Corrupt` otherwise.

#### IN-B02: Role change detection and the plan integrity hash use a non-cryptographic 64-bit FNV

**File:** `crates/ontoforge-storage/src/scope_store.rs:241`, `399`; `crates/ontoforge-storage/src/naming.rs:73`
**Issue:** `roles_changed` is decided by comparing FNV-1a-64 hashes of the role JSON. A role edit whose hash collides with the stored one is reported as unchanged and never written. The docs say the hash is not a defence against an adversary. The input is an authorization document, so a crafted collision by someone who can edit the scope directory (but not run `apply`) would hide a change from the reviewed plan. The risk is small. It matters only if the plan review is meant as a control against that person.
**Fix:** Use SHA-256 (the workspace already depends on a crypto stack through rustls) for `content_hash` and the plan hash, or compare the normalized JSON texts directly.

#### IN-B03: `scope_plan` and `load_scope` take the write lock on SQLite for a read

**File:** `crates/ontoforge-storage/src/scope_store.rs:420-447`
**Issue:** Both use `begin_write` (`BEGIN IMMEDIATE`) only to read, so on SQLite they block every writer for the duration, and `scope plan` can wait on `busy_timeout`.
**Fix:** Use a read transaction (a pooled connection with a plain `BEGIN`) for the read-only calls; keep `begin_write` for the reload path that needs the lock.

#### IN-B04: The CI Trino catalog does not enable metadata caching, so the reload probe test cannot fail on cache handling

**File:** `ci/trino/catalog/editstore.properties`
**Issue:** The JDBC connector's `metadata.cache-ttl` defaults to `0s`, and `metadata.cache-missing` to `false`. Nothing in this file sets them. In CI, Trino never caches the schema, so `reload_trino_it` passes whether or not `flush_metadata_cache()` runs, is ordered correctly, or the probe loops. This is the concrete cause behind the known gap "the flush pass cannot prove the cache was flushed", with a one-line fix. The comment on `postgresql.array-mapping` says production needs the same setting. The same applies to the cache TTL.
**Fix:** Add `metadata.cache-ttl=10m` and `metadata.cache-missing=true` to this catalog (for a negative cache entry that only a flush clears), and make `reload_trino_it` first look at a table, then add a column, then assert that the reload sees it.

---

_Reviewed: 2026-10-21_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_

# 分区 C


## Phase 3 Part C: Code Review Report (auth + expr)

**Reviewed:** 2026-10-21
**Depth:** standard
**Files Reviewed:** 22 in scope (+2 context reads)
**Status:** issues_found

### Summary

No blocker was found. The JWT route is pinned correctly: the algorithm comes from the header and is checked against an asymmetric-only allow-list. HMAC never reaches the OIDC route, and HS256 reaches the dev verifier only when a `DevVerifier` was attached in code. exp, iss, aud and sub are required, nbf is validated, and a JWKS key must match the token's algorithm family and any `alg` it declares. The JWKS address comes only from config, with no redirects, a body cap, a timeout, single-flight and a stale cap.

The row predicate pipeline matches ADR-0025. The parser is a whitelist, so unknown shapes become issues instead of passing through. Kleene logic, empty `IN` as FALSE, null-free lists, case-sensitive matching and Unknown being invisible are all implemented as the ADR specifies. Length and nesting are bounded, and parsing runs on a large-stack thread with panics converted to errors. SQL goes through `quote_string`, `like_pattern` and `glob_pattern`, so no injection path was found.

The findings below are mostly defense-in-depth gaps and hazards in the API shape. WR-C01 is the most concrete: a doc claim that is false on this machine.

### Warnings

#### WR-C01: Dev key file protection on Windows is wrong in the docs and absent in code; no permission check on Unix reads

**File:** `crates/ontoforge-auth/src/authn/dev.rs:79-82` (also 109-126)
**Issue:** The doc says that on Windows "the file inherits the ACL of its directory, which for a user's home or project directory restricts it to that user". I checked `icacls D:\DataSpellProjects\ontoforge` on this machine. The directory grants `NT AUTHORITY\Authenticated Users:(I)(M)` and `BUILTIN\Users:(I)(RX)`. A key created anywhere under a non-system drive therefore inherits Modify for every local user. The key is a symmetric secret, so whoever can read it can mint a token for any subject, including any group (`issue_dev_token` takes arbitrary `groups`).

On Unix the file is created with 0600, but `read_existing` never checks the mode of an existing file, so a 0644 key from a copy, a backup restore or a git checkout is accepted silently. `read_existing` also does not refuse a symlink.
**Fix:**
- On Unix, after `File::open`, call `file.metadata()?.permissions().mode() & 0o077` and fail (or at least warn loudly) when it is non-zero.
- On Windows, either set a protected DACL (owner only) on creation, or drop the claim from the doc and have `serve --dev` print a warning that the key is only as private as its directory.
- Reject a key file that is a symlink.

#### WR-C02: JWKS refresh throttle is recorded only after the fetch completes, so cancellation defeats `min_refresh`

**File:** `crates/ontoforge-auth/src/authn/jwks.rs:147-182`
**Issue:** `state.last_attempt` is written only after `self.fetch().await` returns. If the request future is dropped mid-fetch (client disconnect, handler timeout), the single-flight guard is released and `last_attempt` was never set. The next unknown `kid` then passes `decide()` as `Fetch` again. An attacker who sends tokens with random `kid`s and drops the connection right away makes the server hit the IdP with sequential requests. The rate is bounded only by the attacker, not by `min_refresh`. The module docs promise "Fetches are at least `min_refresh` apart".
**Fix:** Record the attempt before the await, or use a drop guard.
```rust
let attempt = self.clock.now();
self.state.write().unwrap_or_else(PoisonError::into_inner).last_attempt = Some(attempt);
let outcome = self.fetch().await;
```
The trade-off is that a cancelled fetch also delays a legitimate rotation fetch by up to `min_refresh`. That is acceptable and fail-closed.

#### WR-C03: `Authenticator::from_config` and `OidcVerifier::from_config` trust that the config layer already validated security limits

**File:** `crates/ontoforge-auth/src/authn/oidc.rs:36-86`, `crates/ontoforge-auth/src/authn/mod.rs:58`
**Issue:** The constraints live only in `ontoforge-engine/src/config.rs` `check_auth`. They are: https, or http on loopback only (`oidc_url_problem`); `leeway_secs <= 300`; `jwks_min_refresh_secs >= 1`; issuer without `|`; non-empty audience. Both `AuthConfig` and `OidcConfig` are public structs with public fields, and the auth crate does not re-check them. Any code path that builds `AuthConfig` programmatically (tests, a future embedding API, a reload path) can do the following:
- Configure `http://` JWKS on a non-loopback host. This allows MITM key substitution and complete authentication bypass.
- Set `jwks_min_refresh_secs = 0`, which disables the request-amplification throttle entirely.
- Set `leeway_secs` to a huge value.
- Use an issuer containing `|`, which makes the principal id `iss|sub` ambiguous (D-04).
- Use an empty audience.

**Fix:** Have `OidcVerifier::from_config` re-assert the cheap invariants and return `AuthnError::Config`. Ideally call a shared `engine::config` validator for `OidcConfig` instead of duplicating it.

#### WR-C04: `RowValues::value` returns `Null` for a property the row does not carry, which turns `x == null` into a fail-open

**File:** `crates/ontoforge-expr/src/rowpred/eval.rs:64-73`
**Issue:** The trait contract is "A property that is absent is `Scalar::Null`", and the `BTreeMap` impl returns `Null` for a missing key. ADR-0025 consequence 4 accepts that `object.region == null || ...` widens visibility when the value is genuinely null. This is different: if a caller builds the row map with a column missing (projection drops it, schema drift, a rename, a typo in the column mapping), the predicate evaluates against Null without any signal. A rule such as `object.region == null || object.region == principal.attrs.region` then makes every row visible, and `object.region != null && ...` hides every row. The SQL side would error on a missing column, so the Rust and SQL paths diverge on a fail-open.
`CompiledScope::row_filter_properties()` exists, but nothing forces a caller to use it.
**Fix:** Make the lookup fallible, `fn value(&self, p) -> Option<Scalar>`, and have `eval` return `Tri::Unknown` (or a distinct error) for `None`. Alternatively add a `TypeScope::required_properties()` and an `evaluate_checked` that verifies the row carries all of them before evaluating.

#### WR-C05: Criteria coverage ignores whole-object reads (`objects.<param>` with no property)

**File:** `crates/ontoforge-auth/src/scope/coverage.rs:64-88`
**Issue:** `problem()` flags a read only when the path has a third segment. For `objects.flight` (length 2), `path.segments.get(2)?` returns `None` and nothing is reported. A submission criterion that uses the whole object, for example `objects.flight == {'status': 'x', ...}`, `'status' in objects.flight`, or passing it to a function or macro, can reveal which properties exist and what their values are. The role need not have those properties in its allow-list. This is the same dry-run bisecting oracle the module header says ONT-V23 closes.
**Fix:** For `segments.len() == 2` on an object-reference parameter, treat it as reading every property: report V23 unless the role's allow-list covers all properties of the object type, or refuse the shape the way a dynamic index is refused. Add a test with `objects.flight == params.x`.

#### WR-C06: A row filter that fails to compile is represented as "no filter" (all rows), saved only by the all-or-nothing `Err` in `compile_scope`

**File:** `crates/ontoforge-auth/src/scope/compile.rs:180-184`, `crates/ontoforge-auth/src/scope/merge.rs:33,64`
**Issue:** `grant.row_filter.as_deref().and_then(|src| compile_filter(...))` maps a failed compile to `row: None`. In `RoleTypeGrant`, `None` means "grants every row", and `merge` turns it into `Pred::Const(true)`. Today this is safe only because `compile_scope` returns `Err` whenever any diagnostic exists, so such a grant is never part of a returned `CompiledScope`. But the types are all `pub` (`CompiledScope.roles`, `CompiledRole.types`, `RoleTypeGrant.row`), and the representation conflates "role has no filter" with "the filter was rejected". A later refactor (partial compile, skipping failed roles on reload per D-18, an "ignore invalid role" mode) silently becomes a privilege escalation.
**Fix:** Encode the distinction in the type. For example, make `RoleTypeGrant.row` an enum `{ AllRows, Filter(Pred) }` and have `check_role` return `Option<RoleTypeGrant>` (`None` on failure, with the grant dropped). Alternatively use `Pred::Const(false)` as the fail-closed placeholder.

#### WR-C07: NUL in a string literal passes `rowFilter` compilation but fails at SQL print time, so the Rust and SQL paths diverge

**File:** `crates/ontoforge-expr/src/rowpred/parse.rs:450-453` (consumer: `ontoforge-engine/src/printer/escape.rs:11-16,58`)
**Issue:** `limits::escape` accepts `\u0000`, `\x00` and `\000`, and the parser lowers the literal to a `Scalar::String` containing U+0000. The printer rejects NUL (`PrintError::NulInString`) for strings, and also for LIKE/GLOB needles. A role with `object.x == '\u0000'` therefore compiles cleanly and evaluates in Rust, but every Trino or SQLite query for that type errors at request time. The same happens for a JWT string claim containing NUL substituted into a lowered predicate. This is fail-closed (an error, not a leak), but it breaks the "one definition, two executions" guarantee and surfaces as a runtime error, not an ONT-V22.
**Fix:** In `Lowerer::operand`, reject string literals that contain `'\0'` with a PredIssue. In `attr_value` (principal.rs) reject string claim values and list elements containing NUL as `InvalidClaims`.

### Info

#### IN-C01: JWKS entries that declare an encryption algorithm are accepted as signature keys

**File:** `crates/ontoforge-auth/src/authn/jwks.rs:334-346`
**Issue:** The filter skips a key only when `use` is present and is not `sig`. A key with no `use` and `alg: RSA-OAEP` (an encryption key) fails `parse::<Algorithm>()`, so `alg` becomes `None`, which `usable()` reads as "no declared algorithm: any algorithm of the family". `key_ops` is also ignored. Exploiting this needs the IdP to publish such a key and the attacker to hold its private half, so it is hardening only.
**Fix:** If `jwk.common.key_algorithm` is `Some` but does not map to a signature `Algorithm`, skip the key. Also skip keys whose `key_ops` is present and lacks `verify`.

#### IN-C02: Dev key material is copied into non-zeroized buffers

**File:** `crates/ontoforge-auth/src/authn/dev.rs:117-125,134-155,200,232`
**Issue:** `SecretBox` zeroizes only its own copy. The `Vec<u8>` read buffer, the `[u8; 32]` stack copies, and the `EncodingKey` / `DecodingKey` (rebuilt from the secret on every verify) all hold the key bytes without zeroization.
**Fix:** Wrap the read buffer in `Zeroizing`, and build the `DecodingKey` once in `DevVerifier::new`.

#### IN-C03: Unused dependencies in `ontoforge-auth`

**File:** `crates/ontoforge-auth/Cargo.toml:18-26`
**Issue:** `serde`, `chrono` and `indexmap` are not referenced anywhere under `src/`. ADR-0023 says manifests were declared up front, so this may be deliberate, but they enlarge the audit surface of a security crate.
**Fix:** Remove them, or keep a comment naming the plan that will use them.

#### IN-C04: Error message in `list_source` contains a run of literal spaces

**File:** `crates/ontoforge-expr/src/rowpred/parse.rs:312`
**Issue:** The string `"... list-typed              `principal.attrs.<name>`"` has about 14 spaces where a `\` line continuation was probably intended. The text is user-visible in ONT-V22 and in snapshots.
**Fix:** Join the literal on one line.

#### IN-C05: Criteria coverage does not consider the role's row filter

**File:** `crates/ontoforge-auth/src/scope/coverage.rs:58-97`
**Issue:** The check covers the type grant and the property allow-list, but not the role's `rowFilter`. A role limited to `region == 'eu'` and granted an action whose criteria read `objects.flight.status` could probe `status` of an out-of-region flight through a dry run, if the action layer does not itself verify that the referenced object is visible to the caller. This is outside Part C and unverified, so check that the Phase 5 action path enforces row visibility on `ObjectReference` parameters.
**Fix:** Confirm in the action path that object-reference parameters are resolved through the caller's `TypeScope.evaluate`. If not, add a V23 variant for "criteria read a property of a type whose grant carries a row filter".

---

_Reviewed: 2026-10-21_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_

# 分区 D


## Phase 3, Part D: Code Review Report

**Reviewed:** 2026-10-21T12:40:00Z
**Depth:** standard
**Files Reviewed:** 37
**Status:** issues_found

### Summary

Part D covers the CLI commands, the engine SQL printer and SQLite stand-in, the schema loader and IR, the JSON schemas and two docs. I read every listed file and confirmed several findings by running the already built `target/debug/ontoforge.exe` against a scratch copy of `examples/` (scratchpad `ex1`). The scratch copy is outside the repo and nothing in the repo was modified.

What held up under review:

- **SQL printer (no injection found).** Every string goes through `quote_string` (doubles `'`, rejects NUL). Every identifier goes through `quote_ident` (doubles `"`, rejects NUL and empty). Numeric, date, timestamp and decimal literals are shape-validated and then quoted. `LIKE` and `GLOB` wildcards in a needle are escaped correctly for both dialects. `CALL "<catalog>".system.flush_metadata_cache()` quotes the catalog, so a name cannot end the statement or start another. The claim that backslash is not special in Trino or SQLite string literals is accurate.
- **Schema loader path handling.** `read_beneath` and `check_beneath` refuse symlinks and junctions on every path component and re-check with `canonicalize`. Reads are bounded by `take(max+1)`, and file count and total bytes are budgeted. Deeply nested YAML, binary files and missing directories produce diagnostics, not panics. This was tested.
- **Committed JSON schemas.** `ontoforge schema export` output is byte-identical to `schemas/*.json`.
- **Secrets.** Connection strings come only from `url_env` or `url_file`, and store errors do not echo them. The `unknown field password/token/url` path is handled well. The exception is CR-D02.

Two issues are blockers: the documented quickstart fails on a fresh checkout, and server-config errors echo rejected values, which can leak a pasted secret.

### Critical Issues

#### CR-D01: The documented quickstart fails on a fresh checkout (missing parent directories)

**File:** `examples/server/ontoforge-server.dev.toml:13,21`; `crates/ontoforge/src/dev_token.rs:89-91` (key creation lives in `crates/ontoforge-auth/src/authn/dev.rs:134-155`); `docs/scope.md:179`
**Issue:** The example config points the SQLite store at `../../target/ontoforge-dev/editstore.db` and the dev key at `../../target/ontoforge-dev/dev-token.key`. Its header says the key "is created on first use by `ontoforge dev-token`", and `docs/scope.md:179-181` uses this config for `reload`, `scope plan` and `scope apply`. Neither the key writer nor the SQLite opener creates the parent directory, and `target/ontoforge-dev/` does not exist in a clone. I ran both against a copy of the example with `target/` present but not `target/ontoforge-dev/`:

```
$ ontoforge dev-token --server-config examples/server/ontoforge-server.dev.toml --sub mia
error: invalid authentication configuration: dev key file `...\examples/server\../../target/ontoforge-dev/dev-token.key` cannot be created
$ ontoforge reload examples/ontologies/aviation --server-config examples/server/ontoforge-server.dev.toml
  × cannot connect to the edit store: error returned from database: (code: 14) unable to open database file
```

The first documented command of the roles-as-code flow therefore fails (exit 3), and the dev-token flow fails (exit 1). Both succeed once the directory is created by hand.
**Fix:** Create the parent directory before use, in both places.
```rust
// dev_token.rs, before DevKey::load_or_create
if let Some(parent) = key_path.parent().filter(|p| !p.as_str().is_empty()) {
    std::fs::create_dir_all(parent).map_err(|_| format!("cannot create `{parent}`"))?;
}
```
Do the same for the parent of the SQLite `path` in the store opener (`BackendSettings::Sqlite`). Alternatively, make the docs and example say "run `mkdir -p target/ontoforge-dev` first". Add a CLI test that runs the example config from an empty temp tree.

#### CR-D02: Server-config parse errors echo the rejected value, which can leak a pasted secret

**File:** `crates/ontoforge-engine/src/config.rs:397-439` (the final `ConfigError::new(raw).at(span)` at line 438)
**Issue:** Only the unknown keys `password`, `token`, `secret` and `url` are rewritten to a value-free message. Every other `toml::de::Error` message is passed through verbatim, and TOML's messages include the offending value. Header comments in `config.rs:4-6`, `config.rs:320-321`, `reload.rs:7-9` and `online.rs:359-362` promise that "texts name keys, variables and paths, never secret values". Reproduced with the built binary:

```toml
[trino.identities.ontology]
user = "x"
auth = "hunter2secret"        # password pasted into `auth`
```
```
{"code":"ONT-V20","message":"unknown variant `hunter2secret`, expected one of `none`, `basic`, `bearer`"}
```
and `lock_timeout_ms = "hunter2secret"` gives `invalid type: string "hunter2secret", expected u64`. These messages reach stdout and stderr, JSON output and CI logs. The project already classed the same exposure for source lines as critical (CR-C01), so this is the same class.
**Fix:** Do not forward the raw text for value-bearing error kinds. Keep the key and the expected type, and drop the value.
```rust
// from_toml_error: replace the last line
let safe = raw
    .split_once(", expected")                        // "unknown variant `x`, expected ..." -> keep the expectation
    .map_or("invalid value", |(_, rest)| rest)
    .trim();
ConfigError::new(format!("invalid value for this key: expected{safe}")).at(span)
```
A more robust alternative is to build the message from `span` and the key path only (`"`auth` has an invalid value"). Add a test that feeds a recognizable secret into each typed field and asserts it is absent from the output.

### Warnings

#### WR-D01: Dev signing key location and ACL claim are false for the recommended path

**File:** `examples/server/ontoforge-server.dev.toml:21`; doc comment at `crates/ontoforge-auth/src/authn/dev.rs:79-82`
**Issue:** The key doc says that on Windows the file "inherits the ACL of its directory, which for a user's home or project directory restricts it to that user". The example puts the key under the project's `target/`. On this machine, `D:\DataSpellProjects\ontoforge` carries an inherited `NT AUTHORITY\Authenticated Users:(M)` ACE (`icacls` output). That is the normal default for non-system volumes, so any local account can read the HS256 key and mint a dev token for any subject. Dev tokens are only accepted by a `--dev` server, but the key is symmetric and nothing warns the user.
**Fix:** On Windows, after creating the file, tighten the ACL (for example `icacls <file> /inheritance:r /grant:r %USERNAME%:F`). Or refuse and warn when the parent directory ACL is not private. At minimum, correct the doc comment and tell users in `docs/edit-store.md` to keep the key under their profile directory.

#### WR-D02: `reload --dry-run` and `migrate --plan` build the Trino engine and require its secrets

**File:** `crates/ontoforge/src/reload.rs:58-64` and `238-277`; `crates/ontoforge/src/migrate.rs:60-66`
**Issue:** `probe_engine` runs unconditionally, before the `dry_run` / `plan` distinction. For a PostgreSQL store with `[store.trino]` and a Trino profile, `build_trino_engines` resolves the Trino password or token (env var or file) and reads `ca_file`. A read-only preview then fails with ONT-V20 (exit 1) when those secrets are not present on the operator's machine, even though no Trino call can happen in a dry run.
**Fix:** Skip the probe when it cannot be used.
```rust
let probe = if args.dry_run { None } else { probe_engine(&server, ontology, profile)? ... };
// migrate: let probe = if args.plan { None } else { ... };
```

#### WR-D03: The scope loader silently drops files (case-sensitive extensions, symlinks, reserved names)

**File:** `crates/ontoforge-schema/src/loader/discover.rs:298-311` (extension match at 305), `155-204`, `47-56`
**Issue:** `select_scope` matches `"yaml" | "yml" | "json"` case-sensitively. `is_json_file` (line 29) compares case-insensitively, and the doc says "every YAML or JSON file" is selected. `roles.YAML` was reproduced as ignored with no diagnostic. The walk also skips symlinked files and symlinked directories without a message, so a scope directory mounted as a Kubernetes ConfigMap (all symlinks) loads nothing. `is_candidate` also drops a root `ontology.yaml`, any `ontoforge.toml`, hidden directories and `bindings/*/fixtures/**` for scope directories, where those rules make no sense. A silently dropped Role or RoleBinding shows up only as a "role not defined" error or as a `remove role` line in the plan. The store is made equal to the directory, so a dropped file can remove access that should stay.
**Fix:**
```rust
.is_some_and(|e| ["yaml", "yml", "json"].iter().any(|x| e.eq_ignore_ascii_case(x)))
```
Emit an info or warning diagnostic for each skipped symlink and each skipped file with an unknown extension. Use a scope-specific candidate filter that does not apply the manifest, lint-config or fixture rules.

#### WR-D04: Subjects and dev-token arguments are not normalised or validated

**File:** `crates/ontoforge-schema/src/loader/scope.rs:271-308`; `crates/ontoforge/src/dev_token.rs:78-83,134-143,100-104`
**Issue:** `subject_problem` rejects `char::is_control` and an all-blank group, but it accepts leading and trailing whitespace and format characters. With the built binary, `group: " lead "` and `group: "x\u200Fy"` were accepted and planned verbatim. Such a group never matches a token group, so the binding is silently inert. Users with whitespace around `iss` or `sub` behave the same way. `dev-token` checks only `--sub` for emptiness. `--sub 'mia|admin'` produced a principal `ontoforge-dev|mia|admin`, which the scope loader itself refuses as a `user` subject. `--groups 'corp|admins'` mints a dev group equal to the namespaced key of a real issuer, although docs/scope.md:173 says dev groups are "never prefixed". A raw `--sub` containing ESC is echoed unsanitised to the terminal.
**Fix:** Reject leading and trailing whitespace and `char::is_control` or Unicode `Cf` in subjects. In `dev_token`, reject `|` in `--sub`, reject `|` in `--groups` entries (or document and warn), and run `render::sanitize` on echoed values.

#### WR-D05: `normalize_timestamp` accepts a leading `+` in clock fields

**File:** `crates/ontoforge-engine/src/sqlite/fixture.rs:749-758`
**Issue:** The clock is checked only for length 8 and `:` at indexes 2 and 5. The numeric fields are parsed with `u32::parse`, which accepts a leading `+`. `2026-03-29 +1:00:00` and `2026-03-29 09:+5:00` are therefore accepted and normalised to valid timestamps. The function is public, and the edit store and import reuse it so that "a fixture, an Action and an import agree on every value" (line 647). The strictness of `parse_date` (digits only) is not matched here.
**Fix:**
```rust
let digits = |r: std::ops::Range<usize>| {
    let s = &clock[r];
    s.bytes().all(|c| c.is_ascii_digit()).then(|| s.parse::<u32>().ok()).flatten()
};
```
Use `digits(...)` in place of `part(...)`.

#### WR-D06: The audit "actor" is a spoofable and often incomplete environment string

**File:** `crates/ontoforge/src/scope.rs:308-321`
**Issue:** The actor written to `of_admin_audit` comes from `USERNAME`/`USER` and `COMPUTERNAME`/`HOSTNAME`. On Linux (the stated production target) `HOSTNAME` is a shell variable that bash does not export, so the record is typically `alice@unknown`. Both values are fully caller-controlled and not stripped of control characters, so a caller can forge or garble the log line. docs/scope.md:206 admits the identity is unauthenticated, but it does not mention the `unknown` fallback.
**Fix:** Use OS APIs for the account name and host (for example the `whoami` and `gethostname` crates, or `libc` getters without `unsafe` via a crate). Strip control characters from the result and cap its length. Mention the limits in the doc.

### Info

#### IN-D01: Check-then-open race on the symlink guard

**File:** `crates/ontoforge-schema/src/loader/mod.rs:211-238,245-276`
**Issue:** `check_beneath` and `read_limited(follow_links = false)` use `symlink_metadata`, then `File::open` follows links. A concurrent swap of a path component for a symlink between check and open defeats the guard. Impact is limited to someone who can already write the ontology directory, and reads are bounded.
**Fix:** If this is meant as a security boundary, open without following links (`O_NOFOLLOW` on Unix, or `openat`-style traversal with the `cap-std` crate) and `fstat` the opened handle.

#### IN-D02: Debug formatting in a user-facing message

**File:** `crates/ontoforge-engine/src/sqlite/fixture.rs:369`
**Issue:** `format!("the fixture cannot be read: {failure:?}")` renders `Io(NotFound)` or `TooLarge(Some(4194305))` to the operator.
**Fix:** Map `ReadFailure` to the same prose as `loader::read_text` (not found, not a regular file, over the limit).

#### IN-D03: Unknown SQLite value types silently become NULL

**File:** `crates/ontoforge-engine/src/sqlite/mod.rs:364`
**Issue:** `_ => Ok((Value::Null, 0))` turns any type name outside INTEGER, REAL, TEXT and BLOB into NULL. In a data-validation tool (`--check-pk` counts NULL keys) this can create or hide findings if sqlx ever reports another name.
**Fix:** Return `EngineError::Protocol(format!("unsupported SQLite type {type_name}"))`.

#### IN-D04: Dead public function

**File:** `crates/ontoforge/src/online.rs:94-98`
**Issue:** `pub fn validate_online` has no caller. `lib.rs:111` has a private function of the same name that calls `run_online`.
**Fix:** Delete it.

#### IN-D05: Docs and CLI text disagree with the code

**File:** `docs/edit-store.md:110,126,170`; `crates/ontoforge/src/cli.rs:13`; `crates/ontoforge/src/dev_token.rs:101-102`
**Issue:**
- Line 110 shows `CALL system.flush_metadata_cache()`. The code prints `CALL "<catalog>".system.flush_metadata_cache()`, and line 118 of the same doc already shows `<catalog>`.
- Line 126 says all commands read `--server-config` and accept `--format json` and `--profile`. `dev-token` has no `--format` or `--profile`, and `validate`'s `--server-config` is optional.
- Line 170 and the stderr line in `dev_token.rs:101-102` say the token is accepted by a server started with `--dev`. No `serve` command exists in this phase (Phase 7 per `dev.rs:51`), and the doc does not say so.
- `about = "Validate and lint ontology-as-code"` no longer describes a binary with reload, migrate, scope, import and dev-token.

**Fix:** Edit the text. State that `--dev` serving arrives in a later phase.

#### IN-D06: Scope-directory diagnostics are labelled as the ontology

**File:** `crates/ontoforge-schema/src/loader/discover.rs:195-200`; `crates/ontoforge-schema/src/loader/scope.rs:56-62,69-79`
**Issue:** `walk` reports a missing scope directory as "cannot list the ontology directory" with file `ontology.yaml`. The scope loader's internal-error path labels its diagnostic with file `.`. Both mislead for a scope directory.
**Fix:** Pass a label (`ontology` or `scope`) into `walk`, and use the scope root as the file.

#### IN-D07: JSON schemas are looser than the loader

**File:** `crates/ontoforge-schema/src/ir/names.rs:164-173,266-274`; `schemas/ontoforge-scope.schema.json:105-110`
**Issue:** Name patterns do not exclude the reserved words or CEL keywords the loader rejects (`PropertyName` and `ParameterName` reject `in`, `for` and others; all names reject `object`, `link`, ...). `RoleBinding.subjects` has no `minItems: 1`, although an empty list is ONT-V21. `Subject.user` has no pattern for `<iss>|<sub>`. Editors will therefore show no error for documents that fail `validate`.
**Fix:** Add `minItems: 1` and a user pattern (`^[^|\\s]+\\|[^|\\s]+$`) to the schema, and `not: { enum: [...] }` for the reserved words. Keep the committed copies in sync with the existing export test.

#### IN-D08: Minor parsing leniency in `dev-token`

**File:** `crates/ontoforge/src/dev_token.rs:157,214-216`
**Issue:** `--ttl +5m` parses (a leading `+` is accepted by `u64::parse`). A `string_list` attribute `a,,b` yields an empty-string element. Two attributes mapped to the same claim silently overwrite each other (`claims.insert`).
**Fix:** Require ASCII digits in the TTL, reject empty list elements, and error when a claim is set twice.

#### IN-D09: Duplicate dev-dependency

**File:** `crates/ontoforge/Cargo.toml:28,39`
**Issue:** `serde_json.workspace = true` appears in both `[dependencies]` and `[dev-dependencies]`.
**Fix:** Remove the dev-dependency entry.

#### IN-D10: The directory walk is unbounded before the file-count budget applies

**File:** `crates/ontoforge-schema/src/loader/discover.rs:158-204`; `crates/ontoforge-schema/src/loader/mod.rs:364,379`
**Issue:** `walk` collects every regular file under the root into a `Vec<String>` and only afterwards does `assemble` apply `MAX_FILES`. Pointing `scope plan` at a large tree (for example a home directory) walks and stores all of it. There is no depth limit.
**Fix:** Stop collecting after `MAX_FILES + 1` candidates and set a `max_depth` (for example 32).

---

_Reviewed: 2026-10-21T12:40:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
