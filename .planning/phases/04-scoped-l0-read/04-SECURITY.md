---
phase: 04-scoped-l0-read
status: open_threats
verdict: OPEN_THREATS
asvs_level: 1
block_on: high
threats_total: 35
threats_closed: 33
threats_open: 2
---

# Phase 4 Security Audit

The PLAN threat registers contain 35 unique IDs, counting the repeated `T-04-SC` once. The initial read-only audit found 31 closed and four open threats. T-04-22/08 were subsequently fixed with a RED/GREEN external-crate compile-fail regression; two high-severity threats remain open. CI run `39818657834` predates that production-code fix and must be repeated against the new commit.

| Threat | Severity | Open control | Evidence |
|--------|----------|--------------|----------|
| T-04-28 | high | `execute_aggregate` accepts caller-supplied `AggregateCapabilities`; no production path derives them from a trusted binding. | `crates/ontoforge-query/src/aggregate.rs:872`, `crates/ontoforge-schema/src/ir/binding.rs:35`, `crates/ontoforge-query/src/compile.rs:412` |
| T-04-32 | high | The disposable PostgreSQL CI password appears in integration and isolated job logs. The PLAN calls for credentials to be absent from artifacts/logs. | `.github/workflows/ci.yml:75`, CI run `39818657834` |

T-04-22 (high) and T-04-08 (medium) are CLOSED: external callers cannot construct or mutate `ValidatedQuery` fields, and the pagination integration test now calls `ValidatedQuery::new`. Both `compile_fail` doctests failed before the change because bypass code compiled; afterward 2/2 passed. The query crate's non-ignored tests, Clippy, fmt, and `git diff --check` passed locally. Matching-SHA CI run `39823812039` passed all four jobs; real backend and performance details are in `04-PERFORMANCE.md`.

The remaining threat IDs are closed by scoped validation and masking, bounded edit reads, shared budgets, signed cursor context, exact visible-value aggregation, real backend conformance, strict distributed plans, isolated benchmark evidence, and pinned CI dependencies. The relevant source and tests are in `04-01` through `04-07` SUMMARY files. This report does not claim a phase PASS until the open controls are resolved and retested.
