---
phase: "04"
slug: "scoped-l0-read"
status: planned
nyquist_compliant: false
wave_0_complete: false
created: "2026-10-22"
---

# Phase 4 Validation Strategy

## Test Infrastructure

| Property | Value |
|----------|-------|
| Framework | Rust `cargo test` and workspace `proptest` |
| Config file | `Cargo.toml`, `.github/workflows/ci.yml` |
| Quick run command | `cargo test -p ontoforge-query --locked` |
| Full suite command | `cargo fmt --all --check`; `cargo clippy --workspace --all-targets --locked -- -D warnings`; `cargo test --workspace --locked` |
| Estimated runtime | Measure after Wave 0; current query crate is a placeholder |

## Sampling Rate

- After each task: run the focused test target introduced by that task, plus `cargo test -p ontoforge-query --locked` once the crate has coverage.
- After each wave: run formatting, Clippy and workspace tests; run real Trino/PostgreSQL integration tests in the Linux CI integration job.
- Before Phase 4 verification: require local tests, actual CI execution of ignored integration tests and an isolated p95 report.
- Maximum feedback latency: keep the focused local test under 120 seconds; measure and revise after Wave 0.

## Per-Requirement Verification Map

| Requirement | Candidate automated command after Wave 0 | Boundary | Status |
|-------------|------------------------------------------|----------|--------|
| QUERY-01 | `cargo test -p ontoforge-query --test pk_read --locked` | SQLite plus real Trino/PG pk case | planned in 04-01/07 |
| QUERY-02 | `cargo test -p ontoforge-query --test sqlite_query --locked` | SQLite differential and Trino corpus | planned in 04-02/07 |
| QUERY-03 | `cargo test -p ontoforge-query --test sqlite_query --locked` | SQLite and Trino token corpus | planned in 04-02/07 |
| QUERY-04 | `cargo test -p ontoforge-query --test overlay_prop --locked` | Rust model and Trino/PG refill case | planned in 04-04/07 |
| QUERY-05 | `cargo test -p ontoforge-query --test search_around --locked` | FK and M:N with real Trino/PG | planned in 04-05/07 |
| QUERY-06 | `cargo test -p ontoforge-query --test set_ops --locked` | Scoped Rust reference model | planned in 04-05/07 |
| QUERY-07 | `cargo test -p ontoforge-query --test aggregate --locked` | Model plus Trino/PG fixed corpus | planned in 04-06/07 |
| QUERY-08 | `cargo test -p ontoforge-query --test overlay_prop --locked` | Proptest plus CI `trino_query_it` | planned in 04-01..07 |
| QUERY-09 | `cargo test -p ontoforge-query --test security --locked` | Token, authorization, error and cap corpus | planned in 04-01/04/06/07 |
| QUERY-10 | `cargo test -p ontoforge-query --test sql_shape --locked` | Real Trino `EXPLAIN` and isolated p95 | planned in 04-02/06/07 |

Task IDs, wave assignments and threat refs are in the checked `04-*-PLAN.md` files. Test targets above must be created in Wave 0 before their commands run; real backend and performance results remain execution gates.

## Wave 0 Requirements

- [ ] Establish query test targets for overlay properties, SQLite semantics, Trino integration, security and SQL shape.
- [ ] Ensure CI actually runs ignored Trino/PG query tests and fails if the test target is absent.
- [ ] Keep TDD red-green-refactor for overlay, scope, cursor, HMAC and aggregation behavior.

## Manual-Only Verifications

| Behavior | Requirement | Reason | Evidence |
|----------|-------------|--------|----------|
| Quiet or isolated cluster p95 | QUERY-10 | CI shared load cannot establish the stated latency budget | Save sample count, raw timings, connector, dataset, edit coverage and Trino plan/fragment data |

## Validation Sign-Off

- [ ] Every planned task has an automated check or an explicit Wave 0 prerequisite.
- [ ] No three consecutive tasks lack automated verification.
- [ ] Real Trino/PG tests and fragment assertions run in CI.
- [ ] Set `nyquist_compliant: true` only after plans and tests agree.
