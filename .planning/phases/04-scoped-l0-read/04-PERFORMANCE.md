---
phase: 04-scoped-l0-read
status: verified
verdict: PASS
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

## Linux CI Acceptance

- Verdict: **PASS** for `QUERY-10` at code commit `e8d6d6ef1f5103c1b603a89aa66a92ada0ff5600` on `origin/gsd/phase-04-scoped-l0-read`.
- Immutable run: https://github.com/easyinplay/ontoforge/actions/runs/39818657834 (`workflow_dispatch`, completed/success). The isolated benchmark, integration, Windows test and Ubuntu test jobs all succeeded; the isolated job ran on its own Linux runner with fresh PostgreSQL/Trino services and no parallel test load inside that job.
- Raw timings and distributed plans: https://github.com/easyinplay/ontoforge/actions/runs/39818657834/artifacts/12443901512 (`phase4-query-bench-isolated`). GitHub artifact ZIP digest `sha256:4d1e109c88427cc19596ac2928bb905067ac581479bc38e1bd31526c2c5d921f`; downloaded `phase4-isolated.json` SHA256 `D1B6D3051785BF2D3CCA1AD1113DE849A51CEB299B94975386C8F6D7795AE757`.
- Integration plans: https://github.com/easyinplay/ontoforge/actions/runs/39818657834/artifacts/12443337882 (`phase4-query-plans`, ZIP digest `sha256:1ca9ef4e5f5c59e2abfa87ea16782971c7e10c2742f4282cbef48fb51d1787df`). All 137 downloaded plans have complete headers: 103 have one fragment and 34 have two.
- Smoke evidence: https://github.com/easyinplay/ontoforge/actions/runs/39818657834/artifacts/12443252430 (`phase4-query-bench-smoke`, ZIP digest `sha256:11d023bf79a8b9917182f35cc116f8e0ddec513240922b2e1b5138550ad8bfe2`); all four paths passed. Integration logs show `query_read_pg` 2/2, `trino_query_it` 5/5, smoke 1/1 and nonzero ignored-test discovery.
- Environment: Trino 483 PostgreSQL connector, PostgreSQL 17.11 direct edit store, Linux runner. The fixed aviation dataset has 10,000 benchmark rows plus seed/query corpus rows; 800 patches, 100 tombstones and 100 releases. The filtered workload uses `status == bench-hot` over 1,000 matching benchmark rows.
- Query shapes: unedited `Flight` pk `B-10000`; filtered `Flight` list ordered by `flightId` with `first=20`; `Flight` count aggregate over the same filter; fully patched PG-direct pk `B-00001`. Each path has 5 warmups and 40 measured samples, with all 180 samples retained and zero errors or typed limit errors. Nearest-rank p95 was recomputed from the downloaded measured samples.
- Frozen `ExecutionLimits`: 32 backend calls, 10,000 raw candidates, 2,048 backfill keys, 64 MiB decoded bytes. Frozen `AggregateLimits`: 1,024 groups, 4,096 buckets, 10,000 distinct values, 16 MiB distinct memory. These initial guardrails were neither raised nor hit and are not a latency SLA or ADR promise.

| Path | p50 (ms) | p95 (ms) | max (ms) | Required p95 (ms) | Verdict |
|------|---------:|---------:|---------:|------------------:|---------|
| source pk | 75.50 | 95.75 | 109.89 | <500 | PASS |
| filtered list page | 321.76 | 440.75 | 454.03 | <1000 | PASS |
| aggregate | 306.45 | 345.50 | 373.00 | <1500 | PASS |
| edited PG-direct pk | 0.73 | 0.87 | 0.96 | <50 | PASS |

The list and aggregate samples contain 450 distributed `EXPLAIN` plans, all with 1-2 fragments. The edited PG-direct path made zero source calls across all 45 warmup and measured samples.

## Security Fix Reverification

- Current code commit: `dd197063da260f8b92d180eafc733e0b7a81b54e` (the `ValidatedQuery` boundary fix). Matching [CI run 39823812039](https://github.com/easyinplay/ontoforge/actions/runs/39823812039) completed successfully: isolated benchmark, integration, Windows workspace tests, and Ubuntu workspace tests all passed. This supersedes the code-SHA coverage of run 39818657834.
- [Isolated raw artifact](https://github.com/easyinplay/ontoforge/actions/runs/39823812039/artifacts/12445083610): ZIP `sha256:605382f76bcd13b1e5a17b73425e79997eef8c66b5760f2d7bc5c585de9ae89b`; downloaded `phase4-isolated.json` SHA256 `6C4BCC60F12D32CC607ED4C5D2B814DEF8876800E9762ED5569927AB6E706A75`. Its embedded SHA and run URL match this CI run; guardrails were not raised and `limit_error_samples=0`.
- [Integration plans](https://github.com/easyinplay/ontoforge/actions/runs/39823812039/artifacts/12445557864): ZIP `sha256:e3b6a7b9c90e536dea53ed0c6f7516998f192ac91bec2d3337d61b031e180855`; 137 complete plans, 103 with one fragment and 34 with two. [Smoke artifact](https://github.com/easyinplay/ontoforge/actions/runs/39823812039/artifacts/12446380722): ZIP `sha256:80b511eef766967f3530b98f89097ab2ae26a1e6a6bb3ef7bf385e1e7703bfe9`.
- Real PostgreSQL query read tests passed 2/2, Trino/PostgreSQL query tests 5/5, and benchmark smoke 1/1. The isolated Linux job used its own PostgreSQL 17.11 and Trino 483 services. Each of four paths retained 5 warmups and 40 measured samples with zero errors; the p95 below was recomputed from the downloaded measured samples using nearest rank.

| Path | p50 (ms) | p95 (ms) | max (ms) | Required p95 (ms) | Verdict |
|------|---------:|---------:|---------:|------------------:|---------|
| source pk | 83.62 | 115.77 | 143.49 | <500 | PASS |
| filtered list page | 365.18 | 443.80 | 532.85 | <1000 | PASS |
| aggregate | 313.92 | 346.56 | 373.21 | <1500 | PASS |
| edited PG-direct pk | 0.73 | 0.85 | 0.87 | <50 | PASS |

The isolated list and aggregate paths retained 450 complete distributed plans, all with one fragment; edited PG-direct made zero source calls across 45 samples. A later production-code change requires another matching-SHA CI run before this evidence can cover it.

## Current-HEAD CI Reverification

- Code commit: `c39a971a25bb5ebf410bfdcd41243da4ab45c56c`. [CI run 39878005620](https://github.com/easyinplay/ontoforge/actions/runs/39878005620) matches this SHA; integration, isolated-query-bench, Ubuntu tests and Windows tests all passed. This run covers the S1/S2 changes and the S3 credential wiring.
- [Isolated artifact](https://github.com/easyinplay/ontoforge/actions/runs/39878005620/artifacts/12469521692): ZIP `sha256:7f6dce9588e5aaa89cfa04cd30deab87957f395acabec753ae0223a761a0f13a`; downloaded JSON SHA256 `C519EB94854D7F9BDD6CF803A9794DAEB3987C23709E9897D5FEAEF4631D2745`. Its embedded SHA and run URL match the run. `guardrails_raised=false` and `limit_error_samples=0`.
- [Integration plans](https://github.com/easyinplay/ontoforge/actions/runs/39878005620/artifacts/12468919173): ZIP `sha256:475a69e93588a121a361f9f60c84c61463f6f5956cb87eb785d44a33c020fd4e`; 137 complete plans, 103 with one fragment and 34 with two. [Smoke artifact](https://github.com/easyinplay/ontoforge/actions/runs/39878005620/artifacts/12469437044): ZIP `sha256:9a79d2302b8055f6080b8f82aa7bf4d345438275f50679054f5786dc3949c222`.
- Real PostgreSQL query tests passed 2/2, Trino/PostgreSQL query tests 5/5, and smoke 1/1. The isolated Linux job used fresh PostgreSQL 17.11 and Trino 483 services. Each path retained 5 warmups and 40 measured samples, with zero errors; p95 uses nearest rank on the measured samples. The frozen limits and dataset remain as described above.

| Path | p50 (ms) | p95 (ms) | max (ms) | Required p95 (ms) | Verdict |
|------|---------:|---------:|---------:|------------------:|---------|
| source pk | 81.81 | 101.01 | 122.34 | <500 | PASS |
| filtered list page | 335.17 | 456.53 | 465.15 | <1000 | PASS |
| aggregate | 256.06 | 287.81 | 321.22 | <1500 | PASS |
| edited PG-direct pk | 0.69 | 0.76 | 0.78 | <50 | PASS |

The isolated list and aggregate paths retained 450 distributed plans. Edited PG-direct made zero source calls across all 45 samples. No later production-code change is included in this report.
