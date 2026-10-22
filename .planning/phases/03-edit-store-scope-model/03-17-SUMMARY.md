---
phase: 03-edit-store-scope-model
plan: 17
subsystem: storage
tags: [history, masking, scope, edit-log, d-13, store-04, security, tdd]
status: complete

requires:
  - phase: 03-edit-store-scope-model
    provides: "03-08 of_edit_history / of_edit_log / of_edit_log_entry and history::append_snapshot; 03-11 TypeScope::evaluate and merge; 03-13 full operation set (unset, tombstone, release, link edits); 03-14 PrincipalScope inputs (Attrs); ADR-0025 (Unknown is invisible); ADR-0026 (edit log format)"
provides:
  - "ontoforge_storage::history::{CallerKind, HistoryRequest, HistoryEntry, HistoryError} and EditStore::history (library API for the Phase 7 REST / MCP shells)"
  - "tests/history_masking.rs: 12 SQLite cases and one ignored PostgreSQL case"
affects: [03-18, 03-19, 03-20, phase-04-query, phase-07-rest]

plan_head_before: 0979d2aa655a373284e0aeec8d027e7681e8068b

actuals:
  tokens: 10300
  tasks: 2
  commits: 5

tech-stack:
  added: []
  patterns:
    - "Visibility is decided from the current row through TypeScope::evaluate before the store is touched; the same HistoryError::NotFound covers no scope, no object and a hidden object"
    - "build_entry is the only gate between a stored value and the response: a property outside the evaluated column set leaves as a name in masked_changes, never as a value"
    - "Raw Action params are not even queried unless CallerKind::Admin"
    - "before_seq pages by record: the rest of the last record's edits to the object is appended, so a page can exceed the limit but never loses an entry"

key-files:
  created:
    - crates/ontoforge-storage/tests/history_masking.rs
  modified:
    - crates/ontoforge-storage/src/history.rs
    - crates/ontoforge-storage/src/lib.rs

key-decisions:
  - "history is read from three single-table queries (of_edit_history to locate, of_edit_log_entry for op / payload / version_after, of_edit_log for the header) instead of a join, so the unqualified sql_exit helpers (timestamp_text, column_text) are reused and sql_exit.rs stays untouched"
  - "The caller's primary key is canonicalised against the active definition before it is matched; a value that does not fit the key type is NotFound"
  - "A visible object with no edit returns an empty list, not NotFound (plan edge assumption)"
  - "Archived columns (gone from the current definition) are not in the evaluated column set, so they only ever appear in masked_changes (plan edge assumption)"

patterns-established:
  - "DynRow newtype adapts a caller's &dyn RowValues to evaluate(&impl RowValues)"

# STORE-04: this plan delivers the library-level history read with D-13 masking. The REST / MCP exposure is Phase 7, so the
# requirement is addressed here but REQUIREMENTS.md stays unticked (policy decided in 03-01).
requirements-completed: []
requirements-addressed: [STORE-04]

coverage:
  - id: D1
    description: "EditStore::history returns the changes of one object newest first with seq, event id, actor, action, commit time, operation and version after; visible columns carry their value"
    requirement: STORE-04
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/history_masking.rs#ordinary_caller_sees_visible_values_only"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/history_masking.rs#unset_tombstone_release_entries"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/history_masking.rs#link_entries_are_not_object_history"
        status: pass
    human_judgment: false
  - id: D2
    description: "A masked column's value never reaches the serialized response; the column is only named in masked_changes"
    requirement: STORE-04
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/history_masking.rs#serialized_history_never_contains_masked_values"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/history_masking.rs#unset_tombstone_release_entries"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/history_masking.rs#pg_history_masks_and_pages"
        status: pass
    human_judgment: false
  - id: D3
    description: "Masking follows the current row, not the scope at the time of the change (a column another role granted for an old row state stays masked)"
    requirement: STORE-04
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/history_masking.rs#visibility_uses_current_row"
        status: pass
    human_judgment: false
  - id: D4
    description: "Unseen object, missing object and ungranted type give the same HistoryError::NotFound with the same text, with and without history, and the decision is made before the store is consulted"
    requirement: STORE-04
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/history_masking.rs#invisible_nonexistent_and_ungranted_are_identical"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/history_masking.rs#invisible_check_precedes_store_access"
        status: pass
    human_judgment: false
  - id: D5
    description: "Raw Action params are returned to CallerKind::Admin only; an ordinary caller gets None and the key is absent from the JSON"
    requirement: STORE-04
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/history_masking.rs#params_hidden_from_ordinary_caller"
        status: pass
    human_judgment: false
  - id: D6
    description: "Paging: before_seq cursor, limit 0 reads as 1, limit above 500 reads as 500, edits of one commit stay together, a visible object without edits gives an empty list"
    requirement: STORE-04
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/history_masking.rs#pagination_with_before_seq"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/history_masking.rs#limit_zero_counts_as_one"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/history_masking.rs#pagination_keeps_entries_of_one_commit_together"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/history_masking.rs#visible_object_without_edits_has_empty_history"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-storage/src/history.rs#tests::limit_is_between_one_and_the_maximum"
        status: pass
    human_judgment: false

duration: 35 min
completed: 2026-10-20
---

# Phase 3 Plan 17: Masked Object History Summary

**`EditStore::history` reads the change history of one object newest first and shows values only for the columns the caller's `TypeScope` grants on the current row; every other changed column is named in `masked_changes`, and raw Action params go to `CallerKind::Admin` only.**

## Accomplishments

- `history.rs` read side (STORE-04, D-13): `CallerKind`, `HistoryRequest`, `HistoryEntry` (serialisable, `op` as its stored text, `params` skipped when `None`), `HistoryError` and `EditStore::history`. Visibility comes from `TypeScope::evaluate` on the caller's current row (through a small `DynRow` adapter) and is decided before any store access; no scope, no row and a hidden row are one `HistoryError::NotFound` (T-03-56).
- Entries are located through `of_edit_history (type_name, pk, version)`, then op / payload / version come from `of_edit_log_entry` and the header from `of_edit_log`, all built with `sea-query` and run through `sql_exit::fetch_all_as`. Only `kind = 'object'` rows are read.
- `build_entry` sends each property of a `set` payload to `values` or `masked_changes` and each name of an `unset` payload to `unset` or `masked_changes` according to the evaluated column set (T-03-55). The params query is only built for `Admin` (T-03-57).
- Paging: `limit` is clamped to 1..=500, `before_seq` is a strict upper bound on `seq`, and the rest of the last record's edits to the object is appended so one commit that edited the object twice is never split across pages.
- Tests: 12 SQLite cases plus `pg_history_masks_and_pages` (`--ignored`, passed on the embedded server). The fixture holds the store and a guard so both backends share the helpers.

## Task Commits

| Task | Name | Commit |
|------|------|--------|
| 1 RED | tracer tests and a stub `history` returning no entries | 0a81e5f |
| 1 GREEN | masked history read, params for Admin only | b843466 |
| 2 RED | invisible = absent, current-row masking, operations, paging (limit 0 and same-commit paging fail) | b45beae |
| 2 GREEN | limit clamp to 1..=500, same-record completion of a page | 3d54746 |
| 2 test | PostgreSQL case, fixture shared by both backends | 015f5e4 |

## Deviations from Plan

**1. [Rule 3 - Blocking] `mod history;` made `pub mod history;` in `lib.rs`**
- **Found during:** Task 1 RED
- **Issue:** The plan forbids touching `lib.rs` (03-19 owns it that wave) but `history` was a private module, so the integration test target and Phase 7 could not name `ontoforge_storage::history::*`. The plan's own artifact path requires that.
- **Fix:** One-line visibility change. Plans run sequentially on the main tree, so there is no merge conflict with 03-19's `pub mod import;`. 03-19 should keep the line when it edits `lib.rs`.
- **Files modified:** `crates/ontoforge-storage/src/lib.rs`
- **Commit:** 0a81e5f

**2. [Interface] Additions to the sketched interface**
- `HistoryError::Store` has `#[from] StoreError`; `HistoryEntry.op` serialises as its stored text through `serialize_with` (with a targeted `clippy::trivially_copy_pass_by_ref` allow, the signature is serde's); `EditOp` itself is unchanged because `log.rs` is outside this plan's files.
- A request with no loaded definition gives `HistoryError::Store(Config)` for a caller who can see the object, `NotFound` for one who cannot.
- **Commit:** b843466

**3. [Scope] Task 2 tests that passed on arrival**
- Task 1 GREEN already implemented the whole read path, so of the Task 2 tests only `limit_zero_counts_as_one` and `pagination_keeps_entries_of_one_commit_together` were red (they are additions beyond the plan's list; the plan's `limit 0 → 1` rule and cursor loss became tests). The rest passed on arrival, so mutation checks (all restored with `git checkout`) show they can fail.

**4. [Test addition] `pg_history_masks_and_pages`**
- Not in the plan; the history queries use jsonb casts and `to_char` timestamps that SQLite never exercises. It stays inside `history_masking.rs` and is `#[ignore]`d like the other PostgreSQL cases.
- **Commit:** 015f5e4

**Total deviations:** 4 (1 blocking-fix, 1 interface, 1 TDD scope, 1 added test). **Impact:** none on the plan's truths.

## TDD Gate Compliance

Task 1: RED 0a81e5f failed on the planned assertions against a stub that returns `Ok(vec![])` (`left: 0, right: 3` in `ordinary_caller_sees_visible_values_only` and `params_hidden_from_ordinary_caller`, the empty-list assertion in `serialized_history_never_contains_masked_values`); GREEN b843466. Task 2: RED b45beae, two tests failed on behaviour (`limit_zero_counts_as_one`: `left: 0, right: 1`; `pagination_keeps_entries_of_one_commit_together`: `left: [(1, 2)], right: [(1, 2), (1, 1)]`); GREEN 3d54746. No REFACTOR commit.

Mutation checks (security first), each restored afterwards; the named test failed every time:

| Mutation of `history.rs` | Test that failed |
|---|---|
| masked set values also go to `values` (`visible.contains(..) \|\| true`) | `serialized_history_never_contains_masked_values`, `ordinary_caller_sees_visible_values_only` |
| params queried and returned for Ordinary callers (`Admin \|\| true`) | `params_hidden_from_ordinary_caller` |
| visibility checked after the definition lookup | `invisible_check_precedes_store_access` |
| column set taken from the scope, not evaluated on the current row | `visibility_uses_current_row` |
| missing scope returns a different error than a hidden row | `invisible_nonexistent_and_ungranted_are_identical`, `invisible_check_precedes_store_access` |
| unset names always shown (`\|\| true`) | `unset_tombstone_release_entries` |

Not killable, by design: dropping the `kind = 'object'` filter changes nothing observable, because link keys are JSON arrays and object keys are scalars, and link type names (camelCase) cannot equal object type names (PascalCase). The filter is defence in depth and `link_entries_are_not_object_history` documents the intent.

## Known Stubs

None.

## Threat Flags

None beyond the plan's register. T-03-55, T-03-56 and T-03-57 are mitigated and covered by the tests above. `masked_changes` still tells a caller that an unseen column changed at some time; that is D-13's stated design.

## Verification

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`: clean, no failures. `history_masking`: 12 passed, 1 ignored. `sql_exit_guard` passes (no SQL text outside `sql_exit.rs`).
- PostgreSQL (embedded server, Windows 11, no Docker): `cargo test -p ontoforge-storage --locked --features embedded-pg --test history_masking -- --ignored`: 1 passed.
- Acceptance criteria re-run: `serialized_history_never_contains_masked_values`, `params_hidden_from_ordinary_caller`, `invisible_nonexistent_and_ungranted_are_identical`, `visibility_uses_current_row` each `1 passed`; `rg "pub async fn history"` matches `history.rs`.

## Issues Encountered

- The caller supplies `current_row` (source overlay included); `history` cannot check that it is the real current row. Phase 4 / 7 own that, as the plan says.
- Not covered by a test: a history read racing a commit (SQLite and PostgreSQL both give a consistent per-query view, but the three queries are not one snapshot), and a page whose last record holds hundreds of edits to one object.
- Link history (M:N edges) is not exposed; the plan puts it in Phase 7's endpoint design.

## Next Phase Readiness

03-18 and 03-19 are independent of `history.rs`; 03-19 should keep `pub mod history;` when it adds `pub mod import;`. Phase 7 can call `EditStore::history` with the principal's `TypeScope`, the overlaid current row and `CallerKind::Admin` for the administration channel.

## Self-Check: PASSED
