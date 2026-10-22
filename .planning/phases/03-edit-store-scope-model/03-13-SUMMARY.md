---
phase: 03-edit-store-scope-model
plan: 13
subsystem: storage
tags: [edit-store, overlay, state-machine, tombstone, release, one-to-one, concurrency, proptest, replay, sqlite, postgres, tdd]
status: complete

requires:
  - phase: 03-edit-store-scope-model
    provides: "03-08 commit / next_state / log; 03-09 append-only log and replay; 03-10 IndexKind::UniqueWhenSet partial unique index on a oneToOne foreign key"
provides:
  - "Complete transition table next_state / next_link_state (unset, tombstone, release, link add / remove, editOnly vs source backed), documented in the state.rs module comment"
  - "EditError::{NotFound, Tombstoned, InvalidTransition, ReleaseOnEditOnly, MissingRequired, CardinalityViolation} produced by commit; a unique violation of a oneToOne foreign key maps to CardinalityViolation { link }"
  - "commit::{Planned, Advanced, lock_and_advance, write_rows, sort_plan}: the one lock / advance / write path shared by commit and replay"
  - "tests/model (independent reference model) and tests/model_prop.rs (model_matches_store: model vs SQLite after every commit, then log replay), regression seed file"
  - "conformance cases on both backends: source_object_lifecycle, edit_only_rules, link_add_remove_toggle, link_remove_rules, log_records_every_op, replay_rebuilds_overlay_operations, one_to_one_violation_is_reported, duplicate_link_add_keeps_one_live_row, concurrent_commits_on_same_pk_serialize, concurrent_fk_claims_one_wins"
affects: [03-17, 03-19, 03-20]

plan_head_before: 61ba8c384fb9113a504e753716eee362d577ce76

actuals:
  tokens: 23700
  tasks: 3
  commits: 4

tech-stack:
  added: []
  patterns:
    - "A commit locks and advances its rows in lock order (kind, name, pk JSON), takes seq, then writes each row once in the state the commit leaves it; only property columns that differ from the locked row are written (a property that is no longer set gets NULL and a lowered __set)"
    - "Replay of a record uses the same lock_and_advance / write_rows path, so the partial unique index sees the same write order as the original commit"
    - "Reference model in tests/model never imports storage::state; it clones itself, applies a commit in lock order, then simulates the write phase to check the oneToOne rule per written row"

key-files:
  created:
    - crates/ontoforge-storage/tests/model/mod.rs
    - crates/ontoforge-storage/tests/model_prop.rs
    - crates/ontoforge-storage/tests/model_prop.proptest-regressions
  modified:
    - crates/ontoforge-storage/src/state.rs
    - crates/ontoforge-storage/src/commit.rs
    - crates/ontoforge-storage/src/log.rs
    - crates/ontoforge-storage/tests/conformance/mod.rs
    - crates/ontoforge-storage/tests/support/mod.rs
    - .planning/adr/0026-edit-log-format.md

key-decisions:
  - "Transition gaps the plan left open, now fixed in state.rs and the model: release on an editOnly type is ReleaseOnEditOnly from every state (including absent and tombstoned); absent + unset is NotFound for editOnly and InvalidTransition for source backed; absent + release is InvalidTransition; tombstoned + unset / tombstone is InvalidTransition; released + tombstone is allowed; a tombstone keeps the properties that were set, a release clears all of them; link add on a live edge and remove on a removed edge are allowed and add one to the version"
  - "Unset of a required property is MissingRequired only on an editOnly type (a source backed object falls back to the source value); the guard moved from validation into next_state so replay shares it. An empty unset is rejected like an empty set"
  - "A commit writes each row once, in its final state, in lock order; the oneToOne partial unique index is checked per written row. A commit that moves a key between two objects succeeds only when the object that gives the key up sorts first; two objects swapping keys in one commit is a CardinalityViolation (an immediate partial unique index cannot be deferred on PostgreSQL)"
  - "Replay applies a record's entries in the same lock order and writes each row once, instead of entry by entry in ordinal order; ADR-0026 section 7 is amended"
  - "Link backing for the state machine comes from LinkType.backing (None reads as editOnly); remove on an absent editOnly link is NotFound, on an absent source backed link it creates the suppressing row"

patterns-established:
  - "State-derived column writes: update_row compares the locked row with the final state instead of translating each op into columns, so a new op cannot forget a column"

requirements-completed: [STORE-07]
requirements-addressed: [STORE-07, STORE-05, STORE-01]
# STORE-05: edit store side (oneToOne on write, oneToMany FK is single valued) is delivered; the other half of the
# requirement text (source side cardinality reported by lint / validation queries) is not in this plan, so REQUIREMENTS.md
# keeps it unticked. STORE-01: this plan adds conformance cases and concurrency on both backends, but the requirement is
# the whole backend contract (03-18 locking and fence follow), so it stays unticked as well (policy of 03-01).

coverage:
  - id: D1
    description: "unset, tombstone (no revive), release (explicit, source backed only), set after release, versions strictly +1 per write"
    requirement: STORE-07
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/conformance/mod.rs#source_object_lifecycle"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/conformance/mod.rs#edit_only_rules"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-storage/src/state.rs#tests"
        status: pass
    human_judgment: false
  - id: D2
    description: "M:N link add / remove toggle with a tombstone-style suppressing row, duplicate add keeps one live row"
    requirement: STORE-07
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/conformance/mod.rs#link_add_remove_toggle"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/conformance/mod.rs#link_remove_rules"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/conformance/mod.rs#duplicate_link_add_keeps_one_live_row"
        status: pass
    human_judgment: false
  - id: D3
    description: "Independent reference model proptest: outcome class, versions and every edit and link row equal after each commit; the log replays into an identical store (SC3)"
    requirement: STORE-07
    verification:
      - kind: property
        ref: "crates/ontoforge-storage/tests/model_prop.rs#model_matches_store"
        status: pass
    human_judgment: false
  - id: D4
    description: "oneToOne enforced on write with CardinalityViolation, failed commits leave no edit row, log or history"
    requirement: STORE-05
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/conformance/mod.rs#one_to_one_violation_is_reported"
        status: pass
    human_judgment: false
  - id: D5
    description: "Concurrent commits on one pk serialize (versions 1, 2, seq order equals version order); concurrent claims of one foreign key value leave exactly one winner"
    requirement: STORE-01
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/conformance/mod.rs#concurrent_commits_on_same_pk_serialize"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/conformance/mod.rs#concurrent_fk_claims_one_wins"
        status: pass
    human_judgment: false
  - id: D6
    description: "Replay reproduces unset, tombstone, release and link add / remove entries"
    requirement: STORE-07
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/conformance/mod.rs#replay_rebuilds_overlay_operations"
        status: pass
    human_judgment: false
---

# Phase 3 Plan 13: Overlay state machine, reference model and concurrency Summary

**Complete overlay state machine (unset, tombstone, release, M:N link add / remove) with oneToOne enforcement through the partial unique index, a commit / replay path that writes each row once in lock order, and an independent reference-model proptest that also replays the log.**

## Performance

- **Duration:** about 55 min (first commit 14:34, last task commit 15:07, +0800; the setup before it was not timed)
- **Tasks:** 3 (tracer, model proptest, cardinality and concurrency)
- **Files:** 6 source / test files modified, 3 created, 1 ADR amended

## Accomplishments

- `state.rs` holds the full transition table (module comment is the spec). editOnly and source backed types differ in four places: release, tombstone / unset on an absent row, and the required-property guard on unset.
- `commit.rs` no longer translates ops into columns: `update_row` diffs the locked row against the final state and writes only changed property columns, so unset and release (NULL value, lowered `__set`) need no op specific code. Each row is written once per commit.
- A unique violation on a `UniqueWhenSet` index is mapped to `EditError::CardinalityViolation { link }` by finding the oneToOne link whose foreign key column the index covers (PG SQLSTATE 23505, SQLite 2067 / "UNIQUE constraint failed").
- `replay` now locks, advances and writes a record exactly like `commit` (shared `lock_and_advance` / `write_rows`), and checks `version_after` per entry against the planned list.
- `tests/model` is a map-based model written from the table; `model_matches_store` compares it with SQLite after every one of 1 to 30 commits (1 to 3 edits, 3 pks per type, both links) and replays the log into a second store, comparing every column including `of_log_seq`.

## Task Commits

1. **Task 1 RED:** `68d3776` test(03-13): six conformance cases fail with `InvalidTransition` on unset / tombstone / release / link remove (behaviour not yet implemented)
2. **Task 1 GREEN:** `5bfe7dc` feat(03-13): transition table, shared lock / write path, cardinality mapping, replay on the same path
3. **Task 2:** `ab42270` test(03-13): reference model, `model_matches_store`, regression seed
4. **Task 3:** `51c328b` test(03-13): oneToOne, duplicate add and concurrency cases, `Arc<EditStore>` in test support

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Replay wrote entries in ordinal order, commit writes in lock order**
- **Found during:** Task 1 design, confirmed by a mutation in Task 2
- **Issue:** With the oneToOne partial unique index checked per written row, a valid commit `[set 8 gate=A1, tombstone 7 (holds A1)]` writes 7 first at commit time. Replaying its entries by ordinal would write 8 first and fail with a spurious `CardinalityViolation`. Intermediate states of a row edited twice in one commit had the same problem.
- **Fix:** `Planned` / `RowKey` / `sort_plan`, `lock_and_advance` and `write_rows` are shared; each row is written once in its final state. ADR-0026 section 7 amended.
- **Files modified:** `src/commit.rs`, `src/log.rs`, `.planning/adr/0026-edit-log-format.md`
- **Verification:** `model_prop` fails with `replay ... CardinalityViolation` when `sort_plan` is commented out of replay (mutation M5), passes otherwise
- **Commit:** `5bfe7dc`, `ab42270`

**2. [Rule 1 - Bug] Unset of a required property was rejected for every type**
- **Found during:** Task 1
- **Issue:** `plan_object_op` returned `MissingRequired` for any type, but a source backed object that unsets a required property just falls back to the source value; only editOnly has nothing to fall back to.
- **Fix:** the guard lives in `next_state` and applies to editOnly only; an empty unset is rejected as `Value` like an empty set.
- **Files modified:** `src/state.rs`, `src/commit.rs`
- **Commit:** `5bfe7dc`

**3. [Rule 3 - Blocking] Test fixtures**
- **Issue:** the conformance ontology had an editOnly `Stand` and no oneToOne link, so source backed rules and 1:1 could not be exercised.
- **Fix:** `Stand` is source backed with a `gate` string property; added source backed M:N `gateSlots` and oneToOne `gateStand` (foreign key `Stand.gate`). `STAND_COLUMNS` gains `gate`, `gate__set`. `tests/support/mod.rs` (not in the plan's file list) holds `Arc<EditStore>` so the concurrency cases can `tokio::spawn`.
- **Files modified:** `tests/conformance/mod.rs`, `tests/support/mod.rs`
- **Commit:** `68d3776`, `51c328b`

**Total deviations:** 3 auto-fixed (2 bug, 1 blocking). **Impact:** the shared write path is wider than the plan's `commit.rs` mention, but replay needed it for correctness; no public API change beyond `next_link_state` taking the link backing.

## TDD Gate Compliance

- Task 1: RED `68d3776` (six cases failed on `InvalidTransition`), GREEN `5bfe7dc`.
- Tasks 2 and 3: the tests passed on arrival because the behaviour they pin (state machine, cardinality mapping, replay path) was implemented in Task 1's GREEN, which the shared write path required. No separate RED commit exists for them. Mutation checks stand in; each was applied, failed the target test, then reverted:
  - M1 tombstoned + set allowed (`state.rs`): `model_matches_store` fails at step 11 to 14 ("model says Err(Tombstoned), store says Ok"); the shrunk case is saved as the regression seed
  - M2 release keeps the set properties (`state.rs`): fails "rows after step 6"
  - M3 unsorted replay (`log.rs`): fails with `Edit(CardinalityViolation)` during replay
  - M4 unique-violation mapping disabled (`commit.rs`): `one_to_one_violation_is_reported` and `concurrent_fk_claims_one_wins` fail
- Not mutation-detectable: `BEGIN IMMEDIATE` vs `BEGIN` in `sql_exit.rs` gives the same result in the concurrency cases (a commit starts with an INSERT, so a deferred transaction takes the write lock at its first statement). The cases verify the outcome (versions, seq order, one winner), not the lock mode.

## Flagged assumptions (from the plan) and what was decided

- unset of a never-set property is a no-op that still adds one to the version (state.rs test `unset_clears_only_the_named_properties`).
- Source backed tombstone on an absent row is allowed without checking the source (Phase 5 front read checks it).
- editOnly tombstone is permanent: release is `ReleaseOnEditOnly`, set is `Tombstoned`.
- Two edits of one object in a commit apply in commit order; edits of different rows are independent, so only the write order (lock order) is observable, through the oneToOne index.
- Table revisions: none needed to the plan's table; the gaps listed under key-decisions were filled and written into both `state.rs` and the model.

## Known Stubs

None.

## Threat Flags

None. No new endpoint, auth path or schema; T-03-39 (tombstone revive, version regress) is covered by the model proptest and invariants, T-03-40 and T-03-41 by the concurrency and no-trace assertions on both backends.

## Verification

- `cargo test --workspace --locked`: all pass, 0 failures (`model_prop` 64 cases; also run once with `PROPTEST_CASES=300`)
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: clean
- `cargo fmt --all --check`: clean
- `cargo test -p ontoforge-storage --locked --features embedded-pg --test conformance_pg -- --ignored`: 27 passed (embedded PostgreSQL), concurrency cases repeated 3 times without flakiness
- `cargo test -p ontoforge-storage --locked --test conformance_pg -- --ignored --list | rg -c "concurrent_"` is 2
- Acceptance greps: `ReleaseOnEditOnly` in `state.rs`; no `use ontoforge_storage::state` in `tests/model/mod.rs`; `replay` in `model_prop.rs`

## Issues Encountered

- `python3` on this host is the Microsoft Store stub (exits 49 without output); scripted edits that appeared to succeed had not run. They were redone with `python` or the Edit tool. No file was left in a mutated state (`git status` clean for `src/` before each commit).

## Next Phase Readiness

03-17 (history reads) can rely on `of_edit_history` snapshots for all six ops (`{"deleted", "released", "set"}` objects, `{"deleted"}` links). 03-19 (import) commits through the same state machine. Open for later plans: swapping two oneToOne keys inside one commit is refused (limitation noted under key-decisions); the model proptest runs on SQLite only, the PG backend is covered by the conformance cases.

## Self-Check: PASSED

- Created files exist: `tests/model/mod.rs`, `tests/model_prop.rs`, `tests/model_prop.proptest-regressions`
- Commits found: `68d3776`, `5bfe7dc`, `ab42270`, `51c328b`
