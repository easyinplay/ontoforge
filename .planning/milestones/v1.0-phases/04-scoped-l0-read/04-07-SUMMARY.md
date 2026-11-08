---
phase: 04-scoped-l0-read
plan: 07
subsystem: query
tags: [postgresql, trino, integration, benchmark, ci]
requires:
  - phase: 04-05
    provides: scoped traversal and set operations
  - phase: 04-06
    provides: exact scoped aggregation
provides:
  - real Trino/PostgreSQL fixed corpus and distributed EXPLAIN verification
  - fail-closed Linux CI integration, smoke and isolated benchmark jobs
  - traceable Phase 4 performance evidence
affects: [verify-work, QUERY-08, QUERY-09, QUERY-10]
actuals:
  tasks: 3
  commits: 13
commits: 13
plan_head_before: 1a1effd
plan_head_after: e8d6d6ef1f5103c1b603a89aa66a92ada0ff5600
tech-stack:
  added: [actions/upload-artifact]
  patterns: [captured executed SQL EXPLAIN, raw sample artifact, matching-commit CI gate]
key-files:
  created:
    - ci/pg/query_corpus.sql
    - crates/ontoforge-query/tests/trino_query_it.rs
    - crates/ontoforge-query/tests/query_bench_it.rs
    - .planning/phases/04-scoped-l0-read/04-PERFORMANCE.md
  modified:
    - .github/workflows/ci.yml
    - crates/ontoforge-engine/src/printer/mod.rs
    - crates/ontoforge-query/tests/security.rs
    - crates/ontoforge-query/tests/sql_shape.rs
key-decisions:
  - Explain the SQL captured at the Engine boundary to prove the executed source shapes.
  - Treat shared integration timings as smoke only; use isolated Linux samples for thresholds.
requirements-completed: [QUERY-08, QUERY-09, QUERY-10]
coverage:
  - id: REAL-BACKEND-CORPUS
    description: Fixed semantic and security corpus against real Trino 483 and PostgreSQL 17
    requirement: QUERY-08
    verification:
      - kind: integration
        ref: https://github.com/easyinplay/ontoforge/actions/runs/39818657834
        status: pass
    human_judgment: false
  - id: REAL-BACKEND-SECURITY
    description: Multi-role, replay and physical-error conformance on real backends
    requirement: QUERY-09
    verification:
      - kind: integration
        ref: https://github.com/easyinplay/ontoforge/actions/runs/39818657834
        status: pass
    human_judgment: false
  - id: ISOLATED-PERFORMANCE
    description: Four isolated p95 thresholds and complete distributed EXPLAIN plans
    requirement: QUERY-10
    verification:
      - kind: integration
        ref: .planning/phases/04-scoped-l0-read/04-PERFORMANCE.md
        status: pass
    human_judgment: false
completed: 2026-10-23
status: complete
---

# Phase 4 Plan 07 Summary

The real-backend corpus, CI gates and isolated performance benchmark passed on commit `e8d6d6ef1f5103c1b603a89aa66a92ada0ff5600` in run https://github.com/easyinplay/ontoforge/actions/runs/39818657834. Integration ran `query_read_pg` 2/2, `trino_query_it` 5/5 and benchmark smoke 1/1. The isolated job saved all 5 warmups and 40 measured samples for each of four paths, with p95 95.75/440.75/345.50/0.87 ms. Full evidence and artifact digests are in `04-PERFORMANCE.md`.

Task 1 added the fixed source/edit corpus, captured the SQL actually sent to Trino and checked real distributed plans and security behavior. Task 2 wired Linux integration, nonzero ignored-test discovery and three artifacts. Task 3 added the isolated benchmark with strict sample, fragment and edited-PG-direct guards. No core query behavior changed, so TDD was skipped as recorded in `findings.md`.

The plan's PowerShell preflight originally treated CLI YAML output as an array and falsely rejected the workflow. Joining the lines fixed the check; the remote ref and run SHA then matched exactly. Phase 4 UAT and phase-level verification remain separate from this plan completion.
