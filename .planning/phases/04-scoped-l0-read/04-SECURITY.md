---
phase: 04-scoped-l0-read
status: secured
verdict: SECURED
asvs_level: 1
block_on: high
threats_total: 35
threats_closed: 35
threats_open: 0
---

# Phase 4 Security Audit

The PLAN threat registers contain 35 unique IDs, counting the repeated `T-04-SC` once. The independent read-only reaudit found 35 closed and zero open for the current Phase 4 library and CI boundary at commit `c39a971a25bb5ebf410bfdcd41243da4ab45c56c`. [Matching CI run 39878005620](https://github.com/easyinplay/ontoforge/actions/runs/39878005620) passed all four jobs. This security verdict does not replace phase-level verification or user UAT.

| Threat | Disposition | Evidence |
|--------|-------------|----------|
| T-04-22/08 | External code cannot construct or mutate `ValidatedQuery` around validation. | Two RED/GREEN external-crate `compile_fail` doctests passed after the `pub(crate)` change in `dd19706`; matching CI passed. |
| T-04-28 | Wire clients cannot set aggregate capabilities; the trusted host supplies them per connector. | `AggregateRequest` rejects injected `capabilities`; `sum_integer=false` disables partial pushdown; aggregate tests 7/7 and ADR-0027. No production host caller currently exists. |
| T-04-32 | Phase 4 CI credentials are generated per job and masked before services start; uploaded artifacts are scanned. | `ci/query-services.sh`, `.github/workflows/ci.yml`, run `39878005620`; integration and isolated logs showed no unmasked current password or old fixed value. Scans of all 139 downloaded artifact entries found no credential match. |

The other threat IDs remain closed by the scoped validation and masking, bounded edit reads, shared budgets, signed cursor context, exact visible-value aggregation, real backend conformance, strict distributed plans, isolated benchmark evidence, and pinned CI dependencies documented in `04-01` through `04-07` SUMMARY files. Three artifact ZIP digests and current performance details are in `04-PERFORMANCE.md`.

Residual boundary: a future SQL Server host must verify its connector-to-capability mapping and a real `sum(integer)=false` end-to-end case before using aggregate pushdown. `.github/workflows/nightly.yml` still uses the previously public disposable `ontoforge-ci-only` fixture value; nightly does not execute the Phase 4 query corpus or upload its artifacts. That separate fixture risk remains recorded and is not claimed to be remediated by this audit.
