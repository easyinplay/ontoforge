---
phase: 04-scoped-l0-read
plan: 06
subsystem: query
tags: [rust, aggregate, scope, source-partial, topn, distinct, tdd]
requires:
  - phase: 04-04
    provides: scoped overlay, keyset refill and request-owned WorkBudget
  - phase: 04-05
    provides: authorized nested ObjectSet and SearchAround execution
provides:
  - exact scoped count, sum, avg, min, max, distinct and groupBy results
  - connector-aware source partial, bounded TopN and Rust fallback strategies
  - independent SQLite oracle, property, security, limit and SQL-shape evidence
affects: [04-07, verify-work]
actuals:
  tokens: 30224
  tasks: 3
  commits: 19
commits: 19
plan_head_before: 4bf78ae85a6d49b8f515b0e44376f2234c2817ed
plan_head_after: 9c3aae0e7f5e6cb3ac20eba141bd2f65f785681e
tech-stack:
  added: []
  patterns: [typed source partial with old/new correction, bounded TopN refill, visible-value exact builder]
key-files:
  created:
    - crates/ontoforge-query/src/aggregate.rs
    - crates/ontoforge-query/tests/aggregate.rs
  modified:
    - crates/ontoforge-query/src/ast.rs
    - crates/ontoforge-query/src/compile.rs
    - crates/ontoforge-query/src/execute.rs
    - crates/ontoforge-query/src/lib.rs
    - crates/ontoforge-query/tests/security.rs
    - crates/ontoforge-query/tests/sql_shape.rs
key-decisions:
  - AggregateLimits and AggregateCapabilities are injected by the trusted host, never by AggregateRequest.
  - Source partials correct each edited source-old contribution with the visible merged-new contribution; avg carries sum and count.
  - Distinct and groupBy use the bounded Rust path when typed pushdown cannot prove exact results.
  - Nested aggregate pages dispatch through the existing authorized set and SearchAround executors with one WorkBudget.
patterns-established:
  - Aggregate success contains only complete buckets; cost, capability, numeric and backend failures return typed errors without partial data.
requirements-completed: [QUERY-07, QUERY-08, QUERY-09, QUERY-10]
coverage:
  - id: EXACT-MEASURES
    description: Scoped count, sum, avg, min, max and distinct across source and edits
    requirement: QUERY-07
    verification:
      - kind: integration
        ref: cargo test -p ontoforge-query --test aggregate --locked
        status: pass
    human_judgment: false
  - id: INDEPENDENT-ORACLE
    description: CSV full-materialization and 64-case old/new property reference
    requirement: QUERY-08
    verification:
      - kind: unit
        ref: cargo test -p ontoforge-query --lib --locked count_sum_avg_random_old_new_correction_matches_reference
        status: pass
      - kind: integration
        ref: cargo test -p ontoforge-query --test aggregate --locked group_distinct_matches_visible_merged_reference
        status: pass
    human_judgment: false
  - id: BOUNDED-SECURE-RESULT
    description: Inclusive group, bucket, distinct-value and distinct-memory limits with masked values and no partial response
    requirement: QUERY-09
    verification:
      - kind: integration
        ref: cargo test -p ontoforge-query --lib --test aggregate --test security --locked
        status: pass
    human_judgment: false
  - id: TYPED-SOURCE-SHAPE
    description: Typed single-table source partial and bounded TopN SQL shape
    requirement: QUERY-10
    verification:
      - kind: integration
        ref: cargo test -p ontoforge-query --test sql_shape --locked aggregate_topn
        status: pass
    human_judgment: false
duration: 62min
completed: 2026-10-22
status: complete
---

# Phase 4 Plan 06: Exact Scoped Aggregates Summary

Scoped aggregate requests now return exact visible buckets through source partial correction, bounded TopN or bounded Rust evaluation, with explicit errors when exact completion cannot be proved.

## Accomplishments

- Count/sum/avg source partials subtract source-old rows and add merged-new rows. Avg uses corrected sum and count; null and empty inputs retain typed null semantics.
- Min/max reuse scoped keyset refill and the request budget. Edited winners, tombstones, three invalid windows, per-row hidden values, all-null, equal-value ties and budget failure are covered.
- Distinct/groupBy use visible merged values. Tests cover edit group movement, masked-null groups, hidden-value mutation, nested Union, four production B/B+1 limits and counters shared across groups and measures.
- `double` distinct canonicalizes `-0.0` with `0.0`; non-finite backend numbers return `UnsupportedNumericValue` rather than `Source`.

## Task Commits

- Task 1: `6f4805c`, `bef390e`, `af24f6f`, `d528ee2`, `89a9cc6`, `42acabe`, `0806351`, `3b407d0` (contract, bounded builder, source partial and edit correction).
- Task 2: `ec6c0fa`, `c4edc0d`, `9c3aae0` (TopN RED/GREEN and tie test).
- Task 3: `5f69413`, `e3175b1`, `6246826`, `1ff6c0b`, `0a134a4`, `dcd8f37`, `04535b8` (distinct, errors, nested set and boundary tests).
- Initial plan record: `295fac1`.

## TDD Gate Compliance

- RED evidence files `04-06-red-*.json` preserve native libtest JUnit output; GSD returned `RED_EVIDENCE_OK` for each recorded target. The three later regressions cover `double` zero equality, non-finite numeric errors and nested Union dispatch.
- The RED tests failed on their intended behavior assertions. GREEN commits followed and the targets passed. Refactor was limited to formatting and local test cleanup.

## Verification

- `cargo test -p ontoforge-query --locked`: all query unit and integration targets passed, including aggregate 6/6, security 22/22 and sql_shape 6/6. The 64-case property test passed.
- `cargo clippy -p ontoforge-query --all-targets --locked -- -D warnings`: passed.
- `cargo fmt --all --check` and `git diff --check`: passed.
- No package was added; `Cargo.lock` did not change.

## Deviations from Plan

### Auto-fixed Issues

- [Rule 1 - Bug] Distinct counted `-0.0` and `0.0` separately. Typed canonicalization now treats them as one value (`5f69413`, `e3175b1`).
- [Rule 1 - Bug] Non-finite backend doubles surfaced as source failures. The numeric error category now propagates into aggregate errors (`6246826`, `1ff6c0b`).
- [Rule 1 - Bug] Nested set aggregates skipped the authorized set dispatcher and returned zero. Aggregate pages now use the existing set/SearchAround paths with the same budget (`0a134a4`, `dcd8f37`).

## Next Phase Readiness

`04-07` owns real PostgreSQL/Trino parity, distributed `EXPLAIN` and isolated p95 evidence. No production stub or unrun `04-06` verification remains.

## Self-Check: PASSED

`aggregate.rs`, its integration tests and this SUMMARY exist; every listed task commit is an ancestor of `HEAD`. GSD parsed all four coverage records without errors.
