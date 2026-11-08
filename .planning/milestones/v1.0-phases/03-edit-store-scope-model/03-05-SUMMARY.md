---
phase: 03-edit-store-scope-model
plan: 05
subsystem: auth
tags: [cel, row-predicate, kleene, three-valued-logic, proptest, adr, scope]

requires:
  - phase: 03-edit-store-scope-model
    provides: "03-02 rowpred::value (Scalar, AttrValue, Attrs) and AttrType; Phase 1 ontoforge_expr::analyze"
provides:
  - "ontoforge_expr::compile_row_filter: CEL text to an owned Pred, same-row and type checked, all issues collected with expression-relative line:col"
  - "ontoforge_expr::{Pred, Operand, ListSource, CmpOp, MatchKind} with referenced_properties, referenced_attrs and a re-parsable canonical Display"
  - "ontoforge_expr::{Tri, RowValues, eval}: Kleene single-row evaluation, unknown is invisible"
  - "ADR-0025 row predicate semantics (supersedes ADR-0018's 'missing attribute is false' sentence)"
  - "docs/scope.md row predicate section (rowFilter), including the explicit-null-branch consequence"
affects: [03-07, 03-11, 03-14, 03-16, phase-04-query]

plan_head_before: ab84ab499445cb7f50f43fd00be6469c5246ac21

actuals:
  tokens: 26000
  tasks: 3
  commits: 5

tech-stack:
  added: []
  patterns:
    - "cel is used to parse only; the AST is lowered inside the analyze closure to an owned Pred and never evaluated by the cel runtime"
    - "Row predicate type families (bool, int, double, string); int and double never mix; opaque property types only reach == null"
    - "proptest with FileFailurePersistence::WithSource and a checked-in regressions file; generators build only type-correct predicates so display_round_trip checks parse(display(p)) == p"

key-files:
  created:
    - crates/ontoforge-expr/src/rowpred/ir.rs
    - crates/ontoforge-expr/src/rowpred/parse.rs
    - crates/ontoforge-expr/src/rowpred/eval.rs
    - crates/ontoforge-expr/tests/common/mod.rs
    - crates/ontoforge-expr/tests/rowpred_subset.rs
    - crates/ontoforge-expr/tests/rowpred_eval.rs
    - crates/ontoforge-expr/tests/rowpred_eval.proptest-regressions
    - .planning/adr/0025-row-predicate-semantics.md
    - docs/scope.md
  modified:
    - crates/ontoforge-expr/src/lib.rs
    - crates/ontoforge-expr/src/rowpred/mod.rs
    - .planning/adr/0018-authz-scope-and-sql-tiers.md

key-decisions:
  - "D-06 fixed with four concrete rules (checkpoint option-a): x in [] is False even for Null x; no null elements in list literals or list attributes; string matching is case-sensitive with literal wildcards (SQLite GLOB, not LIKE); fail-closed comes only from 'Unknown is invisible', with no special case for missing attributes"
  - "ADR-0025 supersedes only the sentence in ADR-0018 saying a missing attribute makes the predicate false; ADR-0018's status line carries the note and its Decision text is untouched"
  - "A predicate outside the D-07 subset is the non-suppressible V-level ONT-V22, not an E-level lint (recorded in ADR-0025 section 6; the mapping itself is 03-14)"
  - "A bare property is not a predicate: object.flag must be written object.flag == true"

patterns-established:
  - "Mutation check for law tests: temporarily breaking the empty-list rule in eval makes kleene_laws fail with a minimal counterexample, then restored"

# SCOPE-04 is demonstrated for the Rust evaluator only; the SQL lowering and the Rust-vs-SQLite differential are 03-11.
# SCOPE-05 here covers missing attributes as Null and out-of-subset errors; the unclassified entries are carried by 03-14.
# SCOPE-02 here is the rowFilter syntax and types; Role / RoleBinding grants land in 03-14 / 03-16.
# REQUIREMENTS.md is left unticked on purpose (policy decided in 03-01).
requirements-completed: []
requirements-addressed: [SCOPE-04, SCOPE-05, SCOPE-02]

coverage:
  - id: D1
    description: "object.tenantId == principal.attrs.tenantId compiles to Cmp(Column, Eq, Attr) and evaluates True / False for matching / mismatching tenants"
    requirement: SCOPE-04
    verification:
      - kind: integration
        ref: "crates/ontoforge-expr/tests/rowpred_eval.rs#tenant_equality_visible"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-expr/tests/rowpred_eval.rs#tenant_mismatch_false"
        status: pass
    human_judgment: false
  - id: D2
    description: "A missing principal attribute makes the comparison Unknown and the row invisible; a null column under != is Unknown"
    requirement: SCOPE-05
    verification:
      - kind: integration
        ref: "crates/ontoforge-expr/tests/rowpred_eval.rs#missing_attr_is_unknown_and_invisible"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-expr/tests/rowpred_eval.rs#null_column_not_equal_is_unknown"
        status: pass
    human_judgment: false
  - id: D3
    description: "An explicit == null branch makes null rows visible, including to a principal without the attribute (accepted consequence)"
    requirement: SCOPE-04
    verification:
      - kind: integration
        ref: "crates/ontoforge-expr/tests/rowpred_eval.rs#explicit_null_branch_visible"
        status: pass
    human_judgment: false
  - id: D4
    description: "The D-07 subset is accepted with the expected Pred shape (six comparisons, && || !, null on either side, in with literal and attribute lists, startsWith / endsWith / contains, two columns, constants)"
    requirement: SCOPE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-expr/tests/rowpred_subset.rs#accepts_the_six_comparisons"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-expr/tests/rowpred_subset.rs#accepts_in_lists_literal_and_attribute"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-expr/tests/rowpred_subset.rs#accepts_string_matching"
        status: pass
    human_judgment: false
  - id: D5
    description: "Everything outside the subset or violating the same-row and type rules is rejected with exactly one positioned issue (39 sources); several problems in one expression are all reported; syntax and size problems are issues too"
    requirement: SCOPE-05
    verification:
      - kind: integration
        ref: "crates/ontoforge-expr/tests/rowpred_subset.rs#rejects_everything_outside_the_subset"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-expr/tests/rowpred_subset.rs#all_issues_are_collected"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-expr/tests/rowpred_eval.rs#unknown_root_rejected"
        status: pass
    human_judgment: false
  - id: D6
    description: "x in [] is False even for Null x; a missing list attribute is Unknown, an empty one False; string matching is case-sensitive, wildcards are literal, empty needle matches non-null strings"
    requirement: SCOPE-04
    verification:
      - kind: integration
        ref: "crates/ontoforge-expr/tests/rowpred_eval.rs#in_empty_list_is_false_even_for_null"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-expr/tests/rowpred_eval.rs#in_attribute_list_follows_the_same_rules"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-expr/tests/rowpred_eval.rs#string_match_is_case_sensitive_and_literal"
        status: pass
    human_judgment: false
  - id: D7
    description: "compile(display(p)) == p for random type-correct predicates (256 cases; also run at 5000 cases once)"
    requirement: SCOPE-04
    verification:
      - kind: integration
        ref: "crates/ontoforge-expr/tests/rowpred_eval.rs#display_round_trip"
        status: pass
    human_judgment: false
  - id: D8
    description: "Kleene laws hold on random predicates, rows and attributes: De Morgan both ways, double negation, commutativity, unknown under !, IsNull two-valued and complementary, IN equals the OR of equalities, IN [] is False"
    requirement: SCOPE-04
    verification:
      - kind: integration
        ref: "crates/ontoforge-expr/tests/rowpred_eval.rs#kleene_laws"
        status: pass
    human_judgment: false
  - id: D9
    description: "ADR-0025 and docs/scope.md state the D-06 differences from standard CEL, the supersession of ADR-0018's sentence, and the explicit-null-branch consequence with an example"
    verification: []
    human_judgment: true
    rationale: "Document content is a design record and user documentation; a reviewer judges wording and completeness"

duration: about 1h20
completed: 2026-10-20
status: complete
---

# Phase 3 Plan 5: Row predicate parsing, Kleene evaluation and ADR-0025 Summary

**A CEL `rowFilter` is now parsed with `cel`, lowered to an owned `Pred` IR with same-row and strict type checks (all problems reported with expression-relative line:col), and evaluated in Rust with SQL three-valued logic where Unknown is invisible; the semantics are fixed in ADR-0025 and `docs/scope.md`, and `compile(display(p)) == p` and the Kleene laws are locked by proptests.**

## Performance

- **Duration:** about 1h20 for Tasks 2 and 3; Task 1 was a decision checkpoint resolved by the orchestrator
- **Completed:** 2026-10-20
- **Tasks:** 3 of 3
- **Files:** 12 changed (9 created, 3 modified)
- **Verification:** `cargo test --workspace --locked` all green; `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo fmt --all --check` clean

## Accomplishments

- `compile_row_filter` runs inside `analyze` (32 MiB thread, length and nesting precheck, panic to `ExprError::Internal`), lowers `cel::common::ast` to `Pred`, and maps `ExprError` to `PredIssue` (syntax errors keep line:col, size and limit errors have no position).
- Operands carry a static type: value families bool / int / double / string, `Opaque(BaseType)` for date, timestamp, decimal, array and geopoint (only reachable through `== null`, geopoint excluded), `List(Fam)` for list attributes (only right of `in`), and `Null`. `int` and `double` never mix.
- `eval` implements Kleene `And` / `Or` / `Not`, null-aware comparison (byte order for strings, `partial_cmp` for doubles so NaN is Unknown), `IsNull`, SQL `IN` (empty list False, missing list attribute Unknown) and case-sensitive literal `StrMatch`.
- `Pred::Display` produces canonical CEL that re-parses to an equal `Pred`; `Not(Not(x))` is printed `!(!x)` because the CEL grammar folds a bare `!!`.
- Mutation check: changing the empty-list branch of `in_items` to Unknown makes `kleene_laws` fail with a minimal counterexample; restored afterwards. `display_round_trip` and `kleene_laws` also passed once at `PROPTEST_CASES=5000`.
- Evidence recorded in ADR-0025 was re-measured here: SQLite 3.51.0 `NULL IN ()` = 0, `'a' LIKE 'A'` = 1 versus `'a' GLOB 'A'` = 0, `'a*c' GLOB 'a[*]c'` = 1, `NULL IN (1,2)` and `1 IN (2,NULL)` NULL, `1 IN (1,NULL)` 1; and `cel` 0.15 with `x = null` gives `x == 'a'` false and `x != 'a'` true.

## Task Commits

| Task | Commit | What |
|------|--------|------|
| 1 | n/a | checkpoint:decision resolved by the orchestrator: option-a |
| 2 RED | `3f810dd` | `test(03-05)`: six failing tracer tests, full `Pred` IR and `Tri`, stub `compile_row_filter` and `eval` |
| 2 GREEN | `759b174` | `feat(03-05)`: lowering, same-row and family checks, Kleene evaluation of Cmp / IsNull / And / Or / Not |
| 3 RED | `81e0f9b` | `test(03-05)`: subset corpora, `in` and string-match cases, `display_round_trip` and `kleene_laws` |
| 3 GREEN | `58ea23b` | `feat(03-05)`: `in`, string matching, describe(), evaluator `IN` and `StrMatch` |
| 3 docs | `f92a598` | `docs(03-05)`: ADR-0025, ADR-0018 status note, `docs/scope.md` |

## TDD Gate Compliance

Both TDD tasks have a `test(03-05)` commit before the `feat(03-05)` commit. Task 2 RED: all 6 tests in `rowpred_eval` failed on assertions or `expect` messages against the stub (`not implemented` issue, unknown-root position `None`); none failed on a compile or load error. Task 3 RED: 3 of 12 `rowpred_subset` tests (`accepts_string_matching`, `accepts_in_lists_literal_and_attribute`, `issue_messages_say_what_to_change`) and 7 of 14 `rowpred_eval` tests (`in_*`, `string_match_*`, `display_round_trip`, `kleene_laws`) failed on assertions; the remaining tests passed because the Task 2 implementation already covered them. `gsd_run check tdd-red-evidence` was not run: it parses node TAP output and `cargo test` does not emit it, so the failing-test lists above are the evidence. The plan is `type: execute`, so no plan-level `## TDD Gate` section applies.

## Deviations from Plan

### Auto-fixed / adjusted

**1. [Rule 3 - Blocking] Shared fixture module `tests/common/mod.rs`**
- **Issue:** `rowpred_subset.rs` and `rowpred_eval.rs` both need the same `Doc` object type and declared attributes; duplicating 60 lines in two files was worse. `tests/common/mod.rs` is not a test target, so it adds none.
- **Files:** `crates/ontoforge-expr/tests/common/mod.rs` (not in the plan's file list)

**2. [Rule 1 - plan test adjusted] `'abc'.startsWith(object.code)` yields two issues**
- **Issue:** the plan lists it among single-issue rejections, but both the target (a literal) and the argument (a column) are wrong, and the plan also requires collecting all issues.
- **Fix:** the one-issue corpus uses `'abc'.startsWith('b')`; a separate test `a_literal_target_with_a_column_needle_reports_both` asserts 2 issues for the plan's spelling.

**3. [Rule 1 - plan test adjusted] `object.tags == null` and `object.location == null`**
- The plan's corpus lists them together as "接受 / 拒绝". They are split: the accept corpus tests `== null` on `when`, `day`, `cost`, `tags`, `flag`, `n`, `d`; the reject corpus has `object.location == null`.

**4. [Rule 3 - lint] `#[allow(clippy::should_implement_trait)]` on `Tri::not`**
- The plan's interface fixes the method name `not`; clippy wants `std::ops::Not`. The allow carries a comment pointing at the interface instead of adding a second API.

**5. [Scope] Task 2 GREEN already lowered all six comparisons, `&&`, `||`, `!` and `true` / `false`**
- The plan asks the tracer for `==`, `!=`, `||` only. The shared `cmp` and `pred` code made the rest free; Task 3 added `in`, string matching and the fuller messages. No Task 3 test was affected.

**6. [Checkpoint resolution] ADR-0018 status line edited**
- Allowed by the checkpoint resolution and the ADR-0008 precedent: only the status line gained "「缺失属性求值为 false」一句 superseded by ADR-0025"; the Decision text is untouched. Not in the plan's file list.

**7. [Process] `rowpred_eval.proptest-regressions` is header-only**
- The RED run wrote failure seeds from the stub; they were deleted before the RED commit because they describe a state that never existed in a shipped build. The committed file holds proptest's comment header and gains seeds on the first real failure.

**Total deviations:** 7 (none touched a Cargo.toml or another crate's source).

## Auth Gates

None.

## Known Stubs

None.

## Threat Flags

None. The mitigations for T-03-11 (Kleene + Unknown invisible, law proptest and behavior cases), T-03-12 (root whitelist, existing property, `principal.id` / `groups` rejected, reject corpus), T-03-13 (only via `analyze`, `syntax_and_size_problems_are_issues_too`) and T-03-14 (`rg "Program::compile" crates/ontoforge-expr/src/rowpred` has no match) are in place.

## Issues Encountered

- `python3` on this host is the Windows Store stub and silently does nothing; the first scripted edit of `eval.rs` did not apply and was noticed because the tracer still returned Unknown. Scripts were rerun with `python`.
- `prop_oneof!` returns a `TupleUnion` that cannot call `.boxed()` through a generic wrapper; `prop::option::weighted` replaced the hand-rolled `maybe`.

## Next Phase Readiness

03-11 can lower `Pred` (all variants, `Operand::Attr` substituted as a literal, empty `InList` to constant `FALSE`, `StrMatch` to `GLOB` / escaped `LIKE`) and difference-test it against `eval`. 03-14 maps `PredIssue` to ONT-V22 and decides the per-role E-level lint for D-08 collations. 03-07 must reject JWT list claims with null elements at mapping time (ADR-0025 rule 2). 03-16 extends `docs/scope.md` with Role / RoleBinding and apply.

## Self-Check: PASSED

Files found: all 9 created paths above; commits `3f810dd`, `759b174`, `81e0f9b`, `58ea23b`, `f92a598` present in `git log`.
