---
phase: "04"
slug: "scoped-l0-read"
status: validated
nyquist_compliant: true
wave_0_complete: true
created: "2026-10-22"
validated: "2026-10-23"
---

# Phase 4 Validation

## Test Infrastructure

| Property | Value |
|----------|-------|
| Framework | Rust `cargo test`, `proptest`, real Trino/PostgreSQL ignored integration targets |
| Config file | `Cargo.toml`, `.github/workflows/ci.yml` |
| Local command | `cargo test --workspace --locked -j 4 -q` |
| Real backend command | CI `integration` runs `query_read_pg`, `trino_query_it` and benchmark smoke with `--ignored` |
| Performance command | CI `isolated-query-bench` runs `query_bench_it` with `--ignored` on a dedicated Linux runner |

The local workspace command passed on 2026-10-23 (exit code 0). After the S1 boundary fix, [CI run 39823812039](https://github.com/easyinplay/ontoforge/actions/runs/39823812039) passed on `headSha` `dd197063da260f8b92d180eafc733e0b7a81b54e`: `integration`, `isolated-query-bench`, Windows test and Ubuntu test all concluded `success`. Ignored targets are absent from the ordinary local workspace run and executed in CI. The prior run `39818657834` covers the earlier `e8d6d6e` commit only.

## Per-Task Verification Map

| Task | Requirement | Behavioral check | Automated command or evidence | Status |
|------|-------------|------------------|-------------------------------|--------|
| 04-01-1 | QUERY-01, QUERY-08, QUERY-09 | Edited/source pk overlay, scope, read-only store and limits | `cargo test -p ontoforge-query --test pk_read --locked`; storage `query_read` | green |
| 04-01-2 | QUERY-09 | HMAC tamper/replay, indistinguishable probes, sanitized errors and final caps | `cargo test -p ontoforge-query --test security --locked` | green |
| 04-02-1 | QUERY-02, QUERY-03, QUERY-09 | WhereClause matrix, token search and bounded validation | `cargo test -p ontoforge-query --test sqlite_query --locked` | green |
| 04-02-2 | QUERY-09, QUERY-10 | Typed CASE masking and distributed EXPLAIN printer | `cargo test -p ontoforge-engine --locked printer` | green |
| 04-02-3 | QUERY-02, QUERY-03, QUERY-08, QUERY-10 | SQLite differential, role masking and single-table SQL | `cargo test -p ontoforge-query --test sqlite_query --test sql_shape --locked` | green |
| 04-03-1 | QUERY-04, QUERY-08 | Bounded, side-effect-free object edit batch/keyset read | `cargo test -p ontoforge-storage --test query_read --locked` | green |
| 04-03-2 | QUERY-05, QUERY-08, QUERY-09 | M:N endpoint reads and SQLite/PostgreSQL parity | storage `query_read`; CI `query_read_pg` 2/2 | green |
| 04-04-1 | QUERY-04, QUERY-08 | Independent overlay reference and property corpus | `cargo test -p ontoforge-query --test overlay_prop --locked` | green |
| 04-04-2 | QUERY-04, QUERY-09 | Keyset refill, 256-key backfill and shared B/B+1 budgets | `cargo test -p ontoforge-query --test overlay_prop --test security --locked` | green |
| 04-04-3 | QUERY-04, QUERY-09 | Cursor replay, null/multikey pages and complete-row caps | `cargo test -p ontoforge-query --test security --locked` | green |
| 04-05-1 | QUERY-05, QUERY-08, QUERY-09 | Scoped FK/M:N traversal, edges and pagination | `cargo test -p ontoforge-query --test search_around --locked` | green |
| 04-05-2 | QUERY-06, QUERY-08, QUERY-09 | Authorized set algebra against independent oracle | `cargo test -p ontoforge-query --test set_ops --locked` | green |
| 04-06-1 | QUERY-07, QUERY-08 | Count/sum/avg source-old and merged-new correction | `cargo test -p ontoforge-query --test aggregate --locked`; library property test | green |
| 04-06-2 | QUERY-07, QUERY-10 | Min/max TopN refill and bounded single-table SQL | `cargo test -p ontoforge-query --test aggregate --test sql_shape --locked` | green |
| 04-06-3 | QUERY-07, QUERY-09 | Exact distinct/groupBy, masking and no partial over-limit result | `cargo test -p ontoforge-query --test aggregate --test security --locked` | green |
| 04-07-1 | QUERY-08, QUERY-09, QUERY-10 | Real Trino/PG semantic and security corpus with distributed plans | CI `trino_query_it` 5/5; 137 plan entries | green |
| 04-07-2 | QUERY-08, QUERY-09 | CI executes ignored tests and rejects zero-test discovery | CI `integration` job and plan/smoke artifacts | green |
| 04-07-3 | QUERY-10 | Four p95 thresholds, 1-2 fragments and PG-direct zero source calls | CI `isolated-query-bench`; `04-PERFORMANCE.md` | green |

Security remediation S1 (T-04-22/08) adds two external-crate `compile_fail` doctests for direct `ValidatedQuery` construction and mutation. Both failed before the fix because the bypass code compiled, then passed 2/2 after field visibility was restricted. `search_around` 6/6, all non-ignored query crate tests, focused Clippy/fmt, and the matching-SHA CI passed. T-04-28/32 remain security-audit decisions, not behavioral coverage gaps.

## Requirement Coverage

| Requirement | Evidence | Status |
|-------------|----------|--------|
| QUERY-01 | `pk_read`, real Trino/PG corpus | COVERED |
| QUERY-02 | `sqlite_query` operator matrix, real corpus | COVERED |
| QUERY-03 | `sqlite_query` term cases, real corpus | COVERED |
| QUERY-04 | `overlay_prop`, `security` cursor/refill cases | COVERED |
| QUERY-05 | `search_around`, storage link reads, real corpus | COVERED |
| QUERY-06 | `set_ops` fixed/property oracle, real corpus | COVERED |
| QUERY-07 | `aggregate` independent oracle, TopN and real corpus | COVERED |
| QUERY-08 | Reference/property tests, SQLite differential and real Trino/PG CI | COVERED |
| QUERY-09 | `security`, real multi-role/replay/physical-error corpus | COVERED |
| QUERY-10 | `sql_shape`, distributed plans and isolated p95 artifact | COVERED |

## Manual-Only Verifications

None. The p95 check is automated in the dedicated Linux CI job. Its result describes that measured environment, not a general latency guarantee. Sample counts, timings, fragment counts and artifact digests are recorded in `04-PERFORMANCE.md`.

## Validation Sign-Off

- [x] All 18 planned tasks have an executed automated check.
- [x] All 10 requirements have behavioral coverage.
- [x] Real Trino/PostgreSQL ignored tests and distributed fragment checks ran in matching-commit CI.
- [x] Local workspace tests passed; isolated performance samples passed all four strict p95 thresholds.

## Validation Audit 2026-10-22

| Metric | Count |
|---|---|
| Gaps found | 0 |
| Resolved | 0 |
| Escalated | 0 |
