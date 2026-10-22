---
phase: 04-scoped-l0-read
status: pending_remote_ci
verdict: not_verified
---

# Phase 4 Performance Evidence

## Local Preflight

- Code commit: `21ca37fbba2e989f6cb2778604954d1bc4170d23`.
- Environment: local Windows Docker Desktop, PostgreSQL 17.11 and Trino 483; this is not the required isolated Linux CI job.
- Raw evidence: `target/query-bench/phase4-isolated.json` (ignored local artifact), SHA256 `2488B11DC3BCB94B44FD2D4C616F6D4BF8C5C00130763263B0791E0A60C1169E`.
- Dataset: 10,000 benchmark rows in the aviation source table; 800 patch, 100 tombstone and 100 release edits. The table also contains existing aviation seed/query corpus rows.
- Frozen limits: `ExecutionLimits` 32 backend calls, 10,000 raw candidates, 2,048 backfill keys, 64 MiB decoded bytes; `AggregateLimits` 1,024 groups, 4,096 buckets, 10,000 distinct values, 16 MiB distinct memory. No limit was raised or hit in this local run; these guards are not latency targets.
- Each path: 5 warmups and 40 measured samples; nearest-rank p95. All samples are retained in the raw JSON, with zero errors and zero typed limit errors.

| Path | Local p95 (ms) | Required Linux CI p95 (ms) | Local result |
|------|---------------:|---------------------------:|--------------|
| source pk | 88.55 | <500 | preflight pass |
| filtered list page | 379.16 | <1000 | preflight pass |
| aggregate | 338.81 | <1500 | preflight pass |
| edited PG-direct pk | 1.07 | <50 | preflight pass |

List and aggregate produced 450 complete distributed `EXPLAIN` plans across warmups and measured samples, each with 1-2 fragments. Edited PG-direct used zero source calls in all 45 samples.

## Remote Gate

`QUERY-10` is not verified. `origin` has no `gsd/phase-04-scoped-l0-read` ref; `ONTOFORGE_PHASE4_REMOTE_REF` and `ONTOFORGE_PHASE4_COMMIT_SHA` are unset. No matching-commit Linux integration or isolated workflow run, immutable run URL, or downloadable artifact digest exists. Do not mark this report PASS until the read-only preflight, matching SHA dispatch, successful Linux jobs, complete raw artifact and all four thresholds are verified.
