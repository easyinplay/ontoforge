---
phase: 04-scoped-l0-read
plan: 02
subsystem: query
tags: [rust, typed-sql, scope, sqlite, tdd]
requires:
  - phase: 04-01
    provides: scoped pk tracer, QueryRequest, QueryService and final row visibility
provides:
  - bounded logical ObjectSet and WhereClause wire AST with ontology and scope validation
  - pure Rust typed predicate and case-insensitive whitespace term recheck
  - single-table typed source compiler with per-row column grants and CASE projection
  - typed CASE and distributed EXPLAIN printer APIs
affects: [04-04, 04-05, 04-06, 04-07]
actuals:
  tokens: 15748
  tasks: 3
  commits: 5
commits: 5
plan_head_before: 53dcafed50c52a3ef4301b622f261d8a5a2dbf74
plan_head_after: 0b7424260ad2ecbc47008e886dc91f99886b437b
tech-stack:
  added: []
  patterns: [logical-only wire AST, bounded iterative validation, CASE-wrapped column grants, typed Select-only SQL boundary]
key-files:
  created:
    - crates/ontoforge-query/src/ast.rs
    - crates/ontoforge-query/src/compile.rs
    - crates/ontoforge-query/tests/sqlite_query.rs
    - crates/ontoforge-query/tests/sql_shape.rs
  modified:
    - crates/ontoforge-query/src/lib.rs
    - crates/ontoforge-engine/src/printer/ast.rs
    - crates/ontoforge-engine/src/printer/dialect.rs
    - crates/ontoforge-engine/src/printer/mod.rs
    - crates/ontoforge-engine/src/printer/tests.rs
    - crates/ontoforge-engine/src/printer/snapshots/ontoforge_engine__printer__tests__print_sqlite.snap
    - crates/ontoforge-engine/src/printer/snapshots/ontoforge_engine__printer__tests__print_trino.snap
key-decisions:
  - Non-ASCII and case-insensitive token searches use row-scope-only source candidates and bounded Rust recheck because the existing typed printer has no safe case-folded match expression.
  - A predicate atom is wrapped in CASE with its own column grant; hidden values remain Unknown under NOT and cannot borrow another role's row grant.
  - QueryRequest retains its 04-01 struct-literal shape; ordering and page metadata enter through ValidatedQuery until 04-04 unifies execution.
patterns-established:
  - Compiler accepts validated logical names and emits one typed Select with no raw SQL field or construction path.
  - Source masks selected columns and keeps logical aliases; final merged-row scope remains authoritative.
requirements-completed: [QUERY-02, QUERY-03, QUERY-08, QUERY-09, QUERY-10]
coverage:
  - id: QUERY-OPERATORS
    description: 13 WhereClause operator families, typed validation, null semantics and term matching
    requirement: QUERY-02
    verification:
      - kind: integration
        ref: cargo test -p ontoforge-query --test sqlite_query --locked
        status: pass
    human_judgment: false
  - id: QUERY-SOURCE-SHAPE
    description: one-table scoped candidates, column-grant masks, CASE printer and typed EXPLAIN
    requirement: QUERY-10
    verification:
      - kind: integration
        ref: cargo test -p ontoforge-query --test sql_shape --locked
        status: pass
      - kind: unit
        ref: cargo test -p ontoforge-engine --locked printer
        status: pass
    human_judgment: false
duration: 22min
completed: 2026-10-22
status: complete
---

# Phase 4 Plan 02: Scoped Logical Query Compiler Summary

`ObjectSet` now carries bounded, typed filters and token search; source plans remain single-table typed `Select` values, and hidden columns cannot influence predicates through another role's row grant.

## Accomplishments

- Added serde logical AST for `Pk`, `Type`, `Where`, and `Search`, plus all 13 `WhereClause` families, order and page types. Iterative validation checks ontology types, global grants, depth, nodes, IN size, selection, sort and fetch window before a backend call.
- Added a Rust recheck evaluator with canonical property comparisons and Kleene null behavior. SQLite fixed-corpus tests compare source candidates plus Rust recheck against Rust-only expected membership for all 13 operator families; a real SQLite multi-role case verifies hidden values cannot pass either equality or NOT filters.
- Added `Expr::CaseWhen` and `print_explain_distributed(&Select, Dialect)`; both dialect snapshots include CASE, and typed arms retain identifier and literal escaping.

## TDD Gate Compliance

- Printer RED `6e93d44`: `case_when_masks_only_projection` and `distributed_explain_wraps_typed_select` ran and failed on `UnsupportedDialect`, not compilation. GREEN `df6d838`: printer suite passed 11/11.
- Query RED `6ad1ed5`: `every_where_operator_validates_and_evaluates`, `term_search_is_whitespace_any_or_all`, `limits_reject_before_recursion` and source-shape tests ran and failed on the deliberate `InvalidQuery` scaffold. GREEN `18f2b65`: focused query targets passed. A subsequent unbounded-window test failed on its assertion before the bound was implemented.
- Final test expansion `0b74242` locked all source-producing variants, CASE escaping and both printer snapshots. No separate refactor commit was needed.

## Task Commits

1. Printer RED `6e93d44`; GREEN `df6d838`.
2. Logical validation RED `6ad1ed5`; query/compiler GREEN `18f2b65`.
3. Source-shape and snapshot coverage `0b74242`.

## Verification

- `cargo test -p ontoforge-query --locked`: pk 10/10, security 6/6, SQLite query 10/10 and SQL shape 4/4 at the full run. The final focused rerun after adding the all-variants check passed SQLite query 10/10 and SQL shape 5/5.
- `cargo test -p ontoforge-engine --locked printer`: 12/12 printer unit tests and the SQL text boundary test passed. `cargo test -p ontoforge-engine --test printer_prop --locked`: 6/6.
- `cargo clippy -p ontoforge-query -p ontoforge-engine --all-targets --locked -- -D warnings`, `cargo fmt --all --check` and `git diff --check` passed.

## Deviations from Plan

- `[Rule 2 - Critical]` Added a trusted `max_page=10_000` compiler bound after a RED test showed `u64::MAX` could reach source `LIMIT`. This keeps the caller's fetch window bounded before SQL construction.
- `[Rule 2 - Critical]` Atom-level CASE guarding preserves three-valued semantics under NOT and OR. A plain `grant AND predicate` rewrite would make hidden values observable under negation.
- Existing `QueryRequest { set, select }` remains source compatible with 04-01; `ValidatedQuery` holds order and page until the 04-04 execution API integrates them.

## Known Stubs

None. `ObjectSet::Type`, `Where` and `Search` are validated and compiled here; `QueryService::execute` remains the 04-01 pk tracer until 04-04 wires list execution. Real Trino fragment evidence is assigned to 04-07.

## Next Phase Readiness

04-04 can consume `ValidatedQuery`, `SourceSelect` and Rust recheck without SQL strings. Source-only windowing is deliberately separate from edit candidate refill; the latter must use its own edit lane and final merged-row visibility.

## Self-Check: PASSED

Created source/test files and this SUMMARY exist. All five task commits are ancestors of HEAD; `plan_head_before..plan_head_after` measures exactly five commits. No tracked file deletion was found.
