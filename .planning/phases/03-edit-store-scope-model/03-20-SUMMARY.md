---
phase: 03-edit-store-scope-model
plan: 20
subsystem: ci
tags: [ci, github-actions, postgresql, trino, conformance, docs, deployment]
status: complete

requires:
  - phase: 03-edit-store-scope-model
    provides: "03-08/09/13 conformance_pg, 03-04/10/18 reload_pg and reload_trino_it, 03-11 rowpred_trino_it and ci/pg/rowpred_corpus.sql, 03-16 scope_store"
provides:
  - "integration job step `Edit store conformance (PostgreSQL)` (six PG test targets, ONTOFORGE_PG_URL -> editstore)"
  - "integration job steps `Row predicate corpus (Trino)` and `Reload visibility probe (Trino + PostgreSQL)`; rowpred_corpus.sql seeded into `demo`"
  - "test job step `Clippy with the embedded-pg feature`"
  - "ignored-tests guard over ten targets"
  - "ci/trino/catalog/editstore.properties: postgresql.array-mapping=AS_ARRAY"
  - "docs/edit-store.md (deployment and operations), CONTRIBUTING.md section on PG / Trino integration targets"
affects: [phase-04, phase-05]

plan_head_before: ab6c095c99b6cd245dbc95d797b4dc5e032bd768

actuals:
  tokens: 6000
  tasks: 2
  commits: 2

key-files:
  created:
    - docs/edit-store.md
  modified:
    - .github/workflows/ci.yml
    - ci/trino/catalog/editstore.properties
    - CONTRIBUTING.md

key-decisions:
  - "The PG conformance step also runs migrate_pg, history_masking and import, not only the three targets the plan names: they hold #[ignore]d PostgreSQL tests (3, 1 and 2) that no CI step would otherwise run"
  - "The guard loop uses a bash array instead of one long `for` line, so adding a target is a one-line diff"
  - "The PG step sets ONTOFORGE_PG_URL per step, not job-wide, so only the steps that need it see it"

requirements-completed: [STORE-01, SCOPE-04]
requirements-addressed: [STORE-01, SCOPE-04, STORE-02]
# STORE-01 and SCOPE-04 are ticked on the green CI run (see "CI run"). STORE-02 is not: see "Requirements".
---

# Phase 3 Plan 20: CI wiring and edit store deployment guide Summary

The Linux `integration` job (green on its first run) now runs the edit store conformance suites on a real PostgreSQL 17, the fixed row-predicate corpus and the reload visibility probe on Trino 483, behind a guard that fails if any of ten targets lists no ignored tests; `docs/edit-store.md` tells operators how to set up PostgreSQL accounts (REVOKE), the Trino catalog and the commands.

## What was done

- **Task 1 (tracer), `85043ee`:** `Edit store conformance (PostgreSQL)` step with `ONTOFORGE_PG_URL=postgres://postgres:ontoforge-ci-only@localhost:5432/editstore`, before the Trino steps; guard extended; `postgresql.array-mapping=AS_ARRAY` added to the `editstore` catalog (Pitfall 7).
- **Task 2, `67e33f7`:** `rowpred_corpus.sql` seeding, the two Trino steps, guard entries for `reload_trino_it` and `rowpred_trino_it`, the embedded-pg clippy step in the `test` job, `docs/edit-store.md`, the CONTRIBUTING section.
- TDD: skipped. Both tasks are CI configuration and documentation, there is no behaviour in Rust to drive with a test. The Rust sources are untouched (the plan forbids it).

## Deviations from Plan

**1. [Rule 2 - Missing critical functionality] Three more PostgreSQL targets in the CI step and the guard.** `migrate_pg` (3 ignored tests), `history_masking` (1) and `import` (2) are `#[ignore]`d PostgreSQL tests from earlier plans; without this change they would never run in CI. Commit `85043ee`.

**2. [Process] Task 1 precondition (branch pushed) was unmet and the tracer gate carries a `<human-check>`.** The executor stopped with a checkpoint before committing; the coordinator relayed the decision to finish both tasks and push at the end. Task 1 was committed after that message. The human-check on the PR is replaced by the CI run described below.

No auto-fixed bugs; no Rust source changed.

## Verification run locally (Windows 11, no Docker)

| Check | Result |
|---|---|
| `cargo fmt --all --check` | clean |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | clean |
| `cargo clippy -p ontoforge-storage --features embedded-pg --all-targets --locked -- -D warnings` | clean |
| `cargo test --workspace --locked` | 971 passed, 0 failed, 64 ignored |
| Embedded PostgreSQL, `--features embedded-pg -- --ignored` | `conformance_pg` 27, `reload_pg` 11, `scope_store` 3, `migrate_pg` 3, `history_masking` 1, `import` 2: all 47 passed |
| Guard loop (extracted from the workflow and run with bash) | `--ignored --list` counts: conformance_pg 27, reload_pg 11, scope_store 3, migrate_pg 3, history_masking 1, import 2, reload_trino_it 2, rowpred_trino_it 3 (the two engine / CLI targets were already guarded) |
| Workflow YAML | parses with PyYAML; `bash -n` accepts every `run:` block |
| Names the workflow references | `ONTOFORGE_PG_URL`, `ONTOFORGE_TRINO_URL`, `--features embedded-pg`, the ten test targets and `ci/pg/rowpred_corpus.sql` all exist in the code / tree |
| Plan acceptance greps | all match (`conformance_pg`, `rowpred_trino_it`, `reload_trino_it`, `features embedded-pg`, `rowpred_corpus.sql`, `REVOKE UPDATE, DELETE, TRUNCATE`, `array-mapping`, `ontoforge_sys`) |

Embedded PG runs each suite in one local server; CI uses a service container, so the first CI run is the real check of the connection URL, the shared `editstore` database across suites and the schema isolation per test.

## CI run

Run: https://github.com/easyinplay/ontoforge/actions/runs/39541882392 (head `c250125`, draft PR https://github.com/easyinplay/ontoforge/pull/6). The branch was pushed and the draft PR opened by the orchestrator on the user's direct approval. Conclusion: success.

| Job | Result |
|---|---|
| `integration` | success |
| `test (ubuntu-latest)` | success (fmt, no-OpenSSL guard, clippy, `Clippy with the embedded-pg feature`, `cargo test --workspace`) |
| `test (windows-latest)` | success (same steps) |

`integration` steps: Seed PostgreSQL (success, includes `rowpred_corpus.sql`), `Edit store conformance (PostgreSQL)` (success), Start Trino 483 and Wait for Trino (success), Engine and CLI integration tests (success, 9 and 2 passed), `Row predicate corpus (Trino)` (success), `Reload visibility probe (Trino + PostgreSQL)` (success), `Ignored tests are actually there` (success), `Trino logs` (skipped, only on failure).

Per-target results from the log:

| Target | Result |
|---|---|
| `conformance_pg` | 27 passed |
| `reload_pg` | 11 passed |
| `scope_store` | 3 passed |
| `migrate_pg` | 3 passed |
| `import` | 2 passed |
| `history_masking` | 1 passed |
| `rowpred_trino_it` | 3 passed in 25.66 s: `trino_table_equals_the_corpus_rows`, `trino_matches_rust_on_the_corpus_cases`, `trino_matches_rust_on_seeded_predicates` |
| `reload_trino_it` | 2 passed in 1.52 s: `reload_activates_once_trino_sees_the_new_tables`, `a_new_column_is_visible_when_its_definition_becomes_active` |

The guard printed ignored-test counts of 9, 2, 27, 11, 3, 3, 1, 2, 2, 3 for the ten targets, none zero. The 47 PostgreSQL tests that passed on embedded PostgreSQL locally passed on the PostgreSQL 17 service container with the same counts. No first-run fix was needed, so no fix round was used.

**RESEARCH A5 (`CALL <catalog>.system.flush_metadata_cache()` through the client):** not contradicted, but not isolated either. The probe tolerates a failing flush and reports it only when the probe times out (`wait_until_visible`), and the CI Trino catalog has no metadata cache configured. Both Trino tests passed, so Trino showed the new tables and the new column (`gate`) within the 60 s probe timeout. That does not distinguish "the flush worked" from "there was nothing cached". The log has no flush error. Recorded as a gap in `docs/edit-store.md`.

## Requirements

- **STORE-01 ticked.** The same `all_cases!` conformance suite passes on SQLite (Windows and Ubuntu `test` jobs, plus local) and on PostgreSQL 17 (`conformance_pg` 27/27 in CI), and the reload, scope, migrate, history and import PostgreSQL suites pass in CI.
- **SCOPE-04 ticked.** Rust against SQLite differential (03-11, `rowpred_diff`: corpus and 256 seeded predicates) passes in the workspace tests, and the same corpus and the same 256 predicates match the Rust evaluator on Trino 483 in CI, with the seeded `demo.rowpred.rows` equal to the JSON corpus.
- **STORE-02 not ticked.** (1) Cross-version replay of the log through `migration_json` is still ahead (03-18, 03-15). (2) The flush call itself is not isolated by the green run (A5 above). (3) On SQLite `lock_timeout_ms` is not applied to "database is locked" (open gap 2), so the lock-timeout part of the acceptance holds on PostgreSQL only.

## Open gaps (recorded in docs/edit-store.md, not fixed)

1. 03-19: import create-only is checked before the batches; a concurrent creator of the same key gets patched instead of refused (`Commit` has no must-not-exist precondition).
2. SQLite "database is locked" is not mapped to `LockTimeout`.
3. A pending definition version stays in the database when the Trino probe fails; the previous definition stays active and a rerun of `reload` retries.
4. Swapping two oneToOne keys inside one commit is refused (immediate partial unique index on PostgreSQL).
5. Two processes creating the dev-token key file at the same moment: the loser may read a half-written file; start the first one alone.

## Known Stubs

None.

## Threat Flags

None. T-03-67 and T-03-68 are mitigated by the documented REVOKE statements and the rule that the Trino account gets `ontoforge_edit` only; T-03-69 by the guard over ten targets (confirmed by the green run: ten targets, all non-zero); T-03-70 is accepted as planned (the CI-only password is unchanged).

## Self-Check: PASSED

- `docs/edit-store.md`, `.github/workflows/ci.yml`, `ci/trino/catalog/editstore.properties`, `CONTRIBUTING.md`: present.
- Commits `85043ee`, `67e33f7` and `c250125`: present on `gsd/phase-03-edit-store-scope-model`; the CI run above is on `c250125`.
