---
phase: 04-scoped-l0-read
plan: 05
subsystem: query
tags: [rust, search-around, set-algebra, scope, keyset, work-budget, tdd]
requires:
  - phase: 04-04
    provides: scoped source/edit list overlay, keyset cursor, 256-key backfill and shared WorkBudget
provides:
  - FK and M:N traversal over merged objects and bounded source/edit edges
  - same-type authorized PK union, intersect and subtract with normal target hydration
  - fixed, property and integration evidence for traversal, set algebra and scope isolation
affects: [04-06, 04-07]
actuals:
  tokens: 28412
  tasks: 2
  commits: 39
commits: 39
plan_head_before: f8a3c4a4e47c8103f7b71f29ff14d804c0a8298a
plan_head_after: 4d489c4fcb12e6badef9dcfc846acc7c32b53b4a
tech-stack:
  added: []
  patterns: [authorized operands before algebra, normal scoped target hydration, bounded dual-lane link scan]
key-files:
  created:
    - crates/ontoforge-query/src/link.rs
    - crates/ontoforge-query/src/set.rs
    - crates/ontoforge-query/tests/search_around.rs
    - crates/ontoforge-query/tests/set_ops.rs
  modified:
    - crates/ontoforge-query/src/ast.rs
    - crates/ontoforge-query/src/compile.rs
    - crates/ontoforge-query/src/execute.rs
    - crates/ontoforge-query/src/lib.rs
    - crates/ontoforge-query/src/overlay.rs
key-decisions:
  - FK traversal uses the merged row's per-row visible foreign key; M:N requires row-visible source and target endpoints.
  - M:N has Add/Remove link edits and no independent link grant or release operation, consistent with ADR-0024.
  - SearchAround can compose with nested sets; every operand shares the frozen scope and WorkBudget.
patterns-established:
  - Relationship membership yields authorized target keys, then the 04-04 list executor hydrates, orders and caps rows.
  - M:N source and edit lanes poll together initially; later pages use bounded keyset scans.
requirements-completed: [QUERY-05, QUERY-06, QUERY-08, QUERY-09]
coverage:
  - id: FK-TRAVERSAL
    description: Merged outbound and predicate-based inbound FK traversal with per-row FK visibility and missing-grant equivalence
    requirement: QUERY-05
    verification:
      - kind: integration
        ref: cargo test -p ontoforge-query --test search_around --locked
        status: pass
    human_judgment: false
  - id: MN-TRAVERSAL
    description: Source and edit M:N edges, scoped targets, three-page cursor and 256+1 hydration
    requirement: QUERY-05
    verification:
      - kind: integration
        ref: cargo test -p ontoforge-query --test search_around --locked
        status: pass
    human_judgment: false
  - id: SET-ALGEBRA
    description: Same-type scoped operand algebra against an independent BTreeSet oracle
    requirement: QUERY-06
    verification:
      - kind: integration
        ref: cargo test -p ontoforge-query --test set_ops --locked
        status: pass
    human_judgment: false
  - id: INDEPENDENT-REFERENCES
    description: Fixed and property reference models for M:N edge overlay and all three set operators
    requirement: QUERY-08
    verification:
      - kind: integration
        ref: cargo test -p ontoforge-query --test search_around --test set_ops --locked
        status: pass
    human_judgment: false
  - id: COMPOSED-BOUNDS
    description: Shared nested node and work budgets, same-shape denied targets, single-table source and normal final caps
    requirement: QUERY-09
    verification:
      - kind: integration
        ref: cargo test -p ontoforge-query --locked
        status: pass
    human_judgment: false
duration: 3h32min
completed: 2026-10-22
status: complete
---

# Phase 4 Plan 05: Scoped Traversal and Set Algebra Summary

FK and M:N traversal and same-type set algebra now resolve authorized keys before the shared scoped list executor produces the final page.

## Accomplishments

- FK outbound reads the merged, row-visible FK; inbound uses the target's scoped FK predicate, so edits can move membership. Patch, nullable FK, release and absent/false FK grant cases are covered with identical serialized empty responses.
- M:N merges source join-table edges with Add/Remove edits by canonical endpoint pair. Its first source/edit reads poll concurrently; source SQL stays on one table with keyset and limit. The real 257-target test observes 256+1 hydration batches.
- Union, intersect and subtract operate on authorized same-type keys. Nested SearchAround and sets share depth, node, operand and WorkBudget limits. Final rows reuse 04-04 ordering, cursor, projection, version and caps.
- Three traversal pages preserve order and has_more. Serialized empty ListPage responses for tombstoned, unknown and row-hidden targets are identical.

## TDD Gate Compliance

- Task 1 RED commits `3cc1c61`, `450b86e`, `56d1a8f`, `b40ae54` and `3f3f888` preceded FK, M:N, scope, pagination and nested-base implementations. GREEN commits include `cfdf4ae`, `c41340f`, `94dfe55` and `6e95671`. `2b85ec6` adds the nullable FK regression.
- Review RED `686b2dc` reproduced `InvalidQuery` for a missing FK grant; GREEN `4d489c4` makes absent grants serialize identically to false grants in both traversal directions.
- Task 2 RED commits `b700304`, `5517966`, `ead98f6`, `b7fa215`, `20679c2` and `a3edbf2` preceded the set evaluator, nested validator and bounded target hydration. GREEN commits include `6d773f0`, `e73ee26`, `fcdc9c8` and `8df02f8`.

## Verification

- `cargo test -p ontoforge-query --locked`: library 1/1, overlay 11/11, PK 10/10, searchAround 6/6, security 21/21, set ops 5/5, SQL shape 5/5 and SQLite query 10/10. No failed or ignored tests.
- `cargo clippy -p ontoforge-query --all-targets --locked -- -D warnings`, `cargo fmt --all --check` and `git diff --check` passed.
- Source/edit concurrent polling is exercised by the shared scripted two-future test. The traversal integration corpus checks two scope endpoints, three pages, serialized target-denial equivalence, nested budget accumulation, source single-table SQL and exact 256+1 target batches.

## Deviations from Plan

- [Rule 1 - Bug] Final traversal and set hydration originally scanned all target rows despite having authorized keys. `8df02f8` sends those keys through the shared 256-key source/edit path; the 257-target test verifies the boundary.
- [Rule 2 - Missing critical functionality] SearchAround had to serve as a nested set operand and a set had to serve as its base to preserve the planned composition contract. `6e95671` and `8df02f8` share validation and execution budgets across both directions.
- [Rule 1 - Bug] An absent FK grant returned `InvalidQuery` outbound or `NotFound` inbound while a false grant returned an empty page. `4d489c4` treats both as invisible FK membership; `686b2dc` verifies the complete serialized response shape.

## Known Stubs

None.

## Next Phase Readiness

The bounded scoped list finalizer can be reused by 04-06 aggregation and 04-07 read delivery. GSD state and roadmap synchronization is owned by the phase orchestrator.

## Self-Check: PASSED

- All nine code files in `key-files` and this SUMMARY exist.
- Every cited RED/GREEN commit is an ancestor of `4d489c4`.
- `f8a3c4a..4d489c4` contains 39 commits, matching `commits` and `actuals.commits`.
