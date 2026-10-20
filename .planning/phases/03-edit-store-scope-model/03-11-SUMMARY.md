---
phase: 03-edit-store-scope-model
plan: 11
subsystem: auth
tags: [row-predicate, sql-lowering, glob, like-escape, proptest, differential-test, trino, scope-merge]

requires:
  - phase: 03-edit-store-scope-model
    provides: "03-05 Pred IR, Kleene eval, ADR-0025; Phase 2 typed SQL printer and SQLite stand-in"
provides:
  - "ontoforge_auth::scope::lower: Pred to printer Expr with principal attributes substituted as literals"
  - "printer Expr::StringMatch / MatchKind with like_pattern (Trino LIKE ... ESCAPE) and glob_pattern (SQLite GLOB) as the only pattern escape exit"
  - "ontoforge_auth::scope::{merge, TypeScope, RoleTypeGrant, RowVisibility, LoweredScope}: multi-role merge, Rust-evaluable and lowerable"
  - "Rust-vs-SQLite differential proptest, 24-row / 58-case fixed corpus, 256 seeded predicates, ignored Trino test and ci/pg/rowpred_corpus.sql seed"
affects: [03-14, 03-16, 03-20, phase-04-query]

plan_head_before: a2c41b0d29584a285a2d64753bc5a3fae9ccdf38

actuals:
  tokens: 19300
  tasks: 3
  commits: 6

tech-stack:
  added: []
  patterns:
    - "Differential test harness: one in-memory SQLite connection per test thread, predicate SQL always from print_select"
    - "Fixed-seed predicate generation (ChaCha, seed [7;32]) shared by the SQLite test and the ignored Trino test"
    - "Mutation check: swapping GLOB for LIKE in the SQLite dialect makes rust_eval_equals_sqlite, the corpus test and the seeded test fail"

key-files:
  created:
    - crates/ontoforge-auth/src/scope/mod.rs
    - crates/ontoforge-auth/src/scope/lower.rs
    - crates/ontoforge-auth/src/scope/merge.rs
    - crates/ontoforge-auth/tests/rowpred_common/mod.rs
    - crates/ontoforge-auth/tests/rowpred_diff.rs
    - crates/ontoforge-auth/tests/rowpred_trino_it.rs
    - crates/ontoforge-auth/tests/scope_merge.rs
    - crates/ontoforge-auth/tests/data/rowpred_corpus.json
    - ci/pg/rowpred_corpus.sql
  modified:
    - crates/ontoforge-auth/src/lib.rs
    - crates/ontoforge-engine/src/printer/ast.rs
    - crates/ontoforge-engine/src/printer/dialect.rs
    - crates/ontoforge-engine/src/printer/escape.rs
    - crates/ontoforge-engine/src/printer/tests.rs
    - crates/ontoforge-engine/tests/printer_prop.rs

key-decisions:
  - "A StrMatch whose needle is not a string (missing attribute, or a shape the type rules forbid) lowers to <column> = NULL, i.e. Unknown, matching eval"
  - "A missing list attribute in `x in principal.attrs.l` lowers to <operand> = NULL (Unknown for every x); an empty list lowers to constant FALSE"
  - "merge folds ORs to the left, drops structurally equal predicates and lets Const(true) absorb; Const(false) is not simplified"
  - "No regression seed files are checked in: no counterexample was found against the final implementation, and the seeds written during RED described stub states"

patterns-established:
  - "Pattern text is built only in escape.rs (like_pattern, glob_pattern) and then goes through quote_string"

requirements-completed: []
requirements-addressed: [SCOPE-04, SCOPE-03, SCOPE-05, SCOPE-02]
# SCOPE-04: Rust == SQLite proven here; the Trino half runs only in CI (03-20), so it is not ticked.
# SCOPE-03 / SCOPE-05 / SCOPE-02: merge and lowering exist as library API; Role / RoleBinding loading, apply and scoped reads are 03-14 / 03-16 / Phase 4.
# REQUIREMENTS.md is left unticked on purpose (policy decided in 03-01).

coverage:
  - id: D1
    description: "A comparison, NULL column and empty IN list give the same three-valued result in Rust eval and in lowered SQL on SQLite"
    requirement: SCOPE-04
    verification:
      - kind: integration
        ref: "crates/ontoforge-auth/tests/rowpred_diff.rs#tenant_equality_matches"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/rowpred_diff.rs#empty_list_attr_is_false_on_both_sides"
        status: pass
    human_judgment: false
  - id: D2
    description: "StringMatch prints Trino LIKE ... ESCAPE and SQLite GLOB with escaped patterns, quotes the needle, rejects NUL"
    requirement: SCOPE-04
    verification:
      - kind: unit
        ref: "crates/ontoforge-engine/src/printer/tests.rs#string_match_escapes_wildcards_in_both_dialects"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/printer_prop.rs#string_match_matches_rust_on_sqlite"
        status: pass
    human_judgment: false
  - id: D3
    description: "Random type-correct predicates, rows and attributes agree between Rust and SQLite (64 cases by default; also 6000 once); the fixed corpus and 256 seeded predicates agree"
    requirement: SCOPE-04
    verification:
      - kind: integration
        ref: "crates/ontoforge-auth/tests/rowpred_diff.rs#rust_eval_equals_sqlite"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/rowpred_diff.rs#corpus_cases_match_on_sqlite"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/rowpred_diff.rs#fixed_seed_cases_match_on_sqlite"
        status: pass
    human_judgment: false
  - id: D4
    description: "The same corpus and seeded predicates run row by row on Trino against Rust eval"
    requirement: SCOPE-04
    verification: []
    human_judgment: true
    rationale: "rowpred_trino_it is #[ignore]d and needs ONTOFORGE_TRINO_URL; it was only compiled and listed here, and runs for the first time in CI (03-20)"
  - id: D5
    description: "Multi-role merge: no grant gives no scope; column x row isolation; shared column is the OR; equal predicates appear once; visible columns equal the union of true grants; lowered scope agrees with evaluate on SQLite"
    requirement: SCOPE-03
    verification:
      - kind: integration
        ref: "crates/ontoforge-auth/tests/scope_merge.rs#no_grant_is_invisible"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/scope_merge.rs#a_columns_times_b_rows_do_not_leak"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/scope_merge.rs#same_property_from_two_roles_is_or"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/scope_merge.rs#visible_columns_are_union_of_true_grants"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/scope_merge.rs#lowered_scope_matches_evaluate_on_sqlite"
        status: pass
    human_judgment: false

duration: not measured reliably (the executor process was restarted once during Task 3)
completed: 2026-10-20
status: complete
---

# Phase 3 Plan 11: Row predicate lowering, StringMatch and multi-role merge Summary

**A row predicate is lowered to the printer AST with principal attributes as literals (empty `IN` becomes `FALSE`, unknown becomes `= NULL`, string matches become Trino `LIKE ... ESCAPE` / SQLite `GLOB` through one escape exit), a differential proptest proves it equal to the Rust evaluator on SQLite, and `merge` builds a fail-closed `TypeScope` whose columns cannot leak across roles.**

## Performance

- **Tasks:** 3 of 3 (Task 1 is a tracer), 6 commits, 15 files (9 created, 6 modified)
- **Verification:** `cargo test --workspace --locked` no failures; `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo fmt --all --check` clean

## Accomplishments

- `scope::lower` implements the ADR-0025 rules: missing attribute is `NULL`, `x in []` is constant `FALSE`, missing list attribute is `<operand> = NULL`, And/Or chains are flattened.
- Printer `Expr::StringMatch`: Trino `LIKE '<p>' ESCAPE '\'` (escapes `\ % _`), SQLite `GLOB '<p>'` (`[ * ?` become one-character classes), both case sensitive and literal; patterns come only from `like_pattern` / `glob_pattern` and then `quote_string`. `sql_text_only_from_printer` still passes.
- `rust_eval_equals_sqlite` (64 cases default, `PROPTEST_CASES` adjustable) passed also at 6000 cases; `string_match_matches_rust_on_sqlite` passed at 3000. No counterexample was found against the final code.
- Mutation check: replacing `GLOB` by `LIKE` in the SQLite dialect fails all three differential tests, so the generators reach case-sensitivity differences.
- `rowpred_trino_it` (3 ignored tests plus one non-ignored env-var test) reads `demo.rowpred.rows` and compares it to the JSON rows, then the 58 corpus cases and the 256 seeded predicates against Rust. `ci/pg/rowpred_corpus.sql` is generated from the JSON with `jq`. The SQLite side of the same 256 seeded predicates runs locally (`fixed_seed_cases_match_on_sqlite`), so the generation code is exercised.
- `merge` / `TypeScope` as specified; module docs state the Phase 4 usage (row predicate ANDed with the user filter, column predicates only for outermost masking).

## Task Commits

| Task | Commit | What |
|------|--------|------|
| 1 RED | `f27422a` | `test(03-11)`: Harness and three tracer tests against a stub `lower` |
| 1 GREEN | `03f607b` | `feat(03-11)`: `lower` without string matching |
| 2 RED | `8c745ce` | `test(03-11)`: `StringMatch` node with unescaped placeholder patterns, printer/proptest/differential/corpus/Trino tests, PG seed |
| 2 GREEN | `57a1a8f` | `feat(03-11)`: `like_pattern`, `glob_pattern`, `StrMatch` lowering |
| 3 RED | `0e59b7f` | `test(03-11)`: `scope_merge` tests against stub `merge` |
| 3 GREEN | `21c62e8` | `feat(03-11)`: `merge`, `evaluate`, `lower` for `TypeScope` |

## TDD Gate Compliance

Each task has a `test(03-11)` commit before its `feat(03-11)` commit. RED evidence: Task 1, two of three tests failed on `left == right` assertions (SQLite Unknown versus expected True / False); Task 2, 5 printer unit tests, `string_match_matches_rust_on_sqlite`, `rust_eval_equals_sqlite`, the corpus and the seeded test failed on assertions; Task 3, 8 of 9 `scope_merge` tests failed (the proptest `visible_columns_are_union_of_true_grants` failed at `merge(...).expect(...)` on the stub's `None`, the rest on assertions). `gsd_run check tdd-red-evidence` was not run (cargo output is not the TAP it parses), as in 03-05.

## Deviations from Plan

**1. [Process] Proptest regression files not checked in.** The plan lists `printer_prop.proptest-regressions` (existing, unchanged) and `rowpred_diff.proptest-regressions` as files. Failure seeds written during RED came from stub code and were deleted before the RED commits (03-05 precedent); no failure occurred against the real implementation, so no new regression file exists. The corpus holds no historical counterexamples yet; any future one goes into `rowpred_corpus.json` and `ci/pg/rowpred_corpus.sql`.

**2. [Scope] Extra tests.** `identical_role_predicates_appear_once`, `a_role_without_filter_makes_the_row_predicate_true`, `a_role_with_only_the_key_column_still_makes_rows_visible`, `fixed_seed_cases_match_on_sqlite` and `missing_env_panics` were added beyond the plan's behavior list.

**3. [Rule 3 - Lint] Single-character bindings in `rowpred_trino_it`.** `clippy::many_single_char_names` forced descriptive names there.

**4. [Process] RED commits carry placeholder implementations** (stub `lower`, unescaped `like_pattern` / `glob_pattern`, stub `merge`) so the tests compile and fail on assertions.

**Total deviations:** 4, none touching behavior.

## Known Stubs

None. (`lower` maps a `Pred::StrMatch` whose needle is an `Operand::Column` to Unknown; `compile_row_filter` cannot produce it.)

## Threat Flags

None. T-03-31 (only printer AST, patterns via `like_pattern` / `glob_pattern` then `quote_string`, proptest with `'` and wildcards), T-03-32 (GLOB, mutation check), T-03-33 (merge tests and proptests), T-03-34 (differential proptest covers empty lists and missing attributes; Trino by CI corpus) are mitigated as planned.

## Issues Encountered

- The Bash tool halved backslashes in some heredocs and Python snippets, once writing a literal NUL into `escape.rs`; it was caught by reading the file and fixed with the Edit tool before any commit.
- Equivalence is only demonstrated on SQLite. Trino behavior (NULL typing in `NULL IN (...)`, `NULL IS NULL`, `LIKE ... ESCAPE`, PG connector pushdown) is unverified until 03-20 runs `rowpred_trino_it`. NaN, infinities and NUL are outside the guarantee as documented in the plan.

## Next Phase Readiness

03-14 maps `PredIssue` to ONT-V22 and builds `RoleTypeGrant`s from roles; 03-20 must run `ci/pg/rowpred_corpus.sql` on the `demo` database and `cargo test -p ontoforge-auth --test rowpred_trino_it --locked -- --ignored` with `ONTOFORGE_TRINO_URL`. Phase 4 consumes `lower` and `TypeScope`.

## Self-Check: PASSED

All created files exist; commits `f27422a`, `03f607b`, `8c745ce`, `57a1a8f`, `0e59b7f`, `21c62e8` are in `git log`.
