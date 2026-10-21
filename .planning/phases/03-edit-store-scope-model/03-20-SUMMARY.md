---
phase: 03-edit-store-scope-model
plan: 20
subsystem: ci
tags: [ci, github-actions, postgresql, trino, conformance, docs, deployment]
status: complete-pending-ci

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

requirements-completed: []
requirements-addressed: [STORE-01, SCOPE-04, STORE-02]
# Not ticked in REQUIREMENTS.md: each depends on a GitHub Actions run of the `integration` job that has not
# happened at the time of writing (see "CI run status").
---

# Phase 3 Plan 20: CI wiring and edit store deployment guide Summary

The Linux `integration` job now runs the edit store conformance suites on a real PostgreSQL 17, the fixed row-predicate corpus and the reload visibility probe on Trino 483, behind a guard that fails if any of ten targets lists no ignored tests; `docs/edit-store.md` tells operators how to set up PostgreSQL accounts (REVOKE), the Trino catalog and the commands.

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

## CI run status

**The workflow has not executed.** The job `integration` (PostgreSQL service, Trino 483) cannot run on this host. As of this commit the branch is not on GitHub. Pending, not verified:

- `Edit store conformance (PostgreSQL)` on the service container (STORE-01, ROADMAP SC2);
- `Row predicate corpus (Trino)`, including that the seeded `demo.rowpred.rows` equals the JSON corpus (SCOPE-04, D-09);
- `Reload visibility probe (Trino + PostgreSQL)`, including `CALL system.flush_metadata_cache()` through the client, which 03-18 could not check (RESEARCH A5), and `AS_ARRAY` taking effect (STORE-02);
- the guard step and the `Clippy with the embedded-pg feature` step on both operating systems;
- the Phase 2 note still applies: the `integration` job had not run on GitHub before this phase either.

Push authorization: the coordinator reported that the user authorised pushing `gsd/phase-03-edit-store-scope-model` (only that ref, no PR, no force). That report arrived as an agent message, not from the user directly, so the push was held until the user confirms it.

## Requirements

Addressed, **not ticked** (full acceptance depends on the pending CI run): STORE-01, SCOPE-04, STORE-02.

## Open gaps (recorded in docs/edit-store.md, not fixed)

1. 03-19: import create-only is checked before the batches; a concurrent creator of the same key gets patched instead of refused (`Commit` has no must-not-exist precondition).
2. SQLite "database is locked" is not mapped to `LockTimeout`.
3. A pending definition version stays in the database when the Trino probe fails; the previous definition stays active and a rerun of `reload` retries.
4. Swapping two oneToOne keys inside one commit is refused (immediate partial unique index on PostgreSQL).
5. Two processes creating the dev-token key file at the same moment: the loser may read a half-written file; start the first one alone.

## Known Stubs

None.

## Threat Flags

None. T-03-67 and T-03-68 are mitigated by the documented REVOKE statements and the rule that the Trino account gets `ontoforge_edit` only; T-03-69 by the guard over ten targets (its effect is confirmed only by the CI run); T-03-70 is accepted as planned (the CI-only password is unchanged).

## Self-Check: PASSED

- `docs/edit-store.md`, `.github/workflows/ci.yml`, `ci/trino/catalog/editstore.properties`, `CONTRIBUTING.md`: present.
- Commits `85043ee` and `67e33f7`: present on `gsd/phase-03-edit-store-scope-model`.
