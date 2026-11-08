---
phase: 01-workspace-schema-lint
plan: 10
subsystem: lint-naming-rules
tags: [rust, lint, naming, regex, insta, ont-w01, ont-w02, ont-w03, ont-w04, ont-w05]

requires:
  - phase: 01-workspace-schema-lint
    provides: "01-08 lint framework (LintRule, Finding, registry, naming::rules() seam, split_camel, lint_rule_on); 01-13 heck and shared offset_to_line_col"
provides:
  - "ONT-W01 generic or plural object type names, ONT-W02 ambiguous generic property names, ONT-W03 encoded property names, ONT-W04 mixed date naming styles, ONT-W05 pipeline bookkeeping properties"
  - "lint::naming::rules() returns the five rules; every rule is default warn, no options"
  - "insta trigger and clean snapshots for each rule under lint/naming/snapshots"
affects: [01-11, 01-12]

plan_head_before: 258086ba0a745c93f161d6275c52d12f79a6e132
actuals:
  tokens: 9400
  tasks: 2
  commits: 4

tech-stack:
  added: []
  patterns:
    - "One module per rule (w01..w05) with a static RULE; shared Finding builders (type_finding, property_finding) and off_hint live in naming/mod.rs"
    - "Test fixtures built by naming::testing::objects(specs) so each rule test lists only type and property names"

key-files:
  created:
    - crates/ontoforge-schema/src/lint/naming/w01.rs
    - crates/ontoforge-schema/src/lint/naming/w02.rs
    - crates/ontoforge-schema/src/lint/naming/w03.rs
    - crates/ontoforge-schema/src/lint/naming/w04.rs
    - crates/ontoforge-schema/src/lint/naming/w05.rs
    - crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w0{1,2,3,4,5}__tests__{triggers,clean}.snap
  modified:
    - crates/ontoforge-schema/src/lint/naming/mod.rs

key-decisions:
  - "W01 generic means every word is generic (Data, DataRecord), not 'one or two words'; W03 field pattern is anchored (^field[A-Z0-9]); both tightenings come from the plan and are written in the module docs"
  - "Every naming help text ends with the way to switch the rule off, because W01, W02 and W05 are heuristics"
  - "W04 treats a name that fits neither style (birthday) as neutral; on equal counts the prefix style (dateOfXxx) is reported"
  - "split_camel kept; convert_case not adopted (see Reuse decision)"

requirements-completed: [LINT-02]

coverage:
  - id: D1
    description: "W01 reports all-generic and plural object type names, not Status, Address, Series, OrderItem, MaintenanceRecord"
    requirement: LINT-02
    verification:
      - kind: unit
        ref: "cargo test -p ontoforge-schema --lib lint::naming::w01 (triggers, clean; insta snapshots)"
        status: pass
    human_judgment: false
  - id: D2
    description: "W02 reports a bare generic property next to a same-suffix property on the same type only"
    requirement: LINT-02
    verification:
      - kind: unit
        ref: "cargo test -p ontoforge-schema --lib lint::naming::w02 (triggers, clean; insta snapshots)"
        status: pass
    human_judgment: false
  - id: D3
    description: "W03 reports dt-prefix, n plus two capitals, two trailing digits and ^field[A-Z0-9]; airfieldCode, level2, nCount stay clean"
    requirement: LINT-02
    verification:
      - kind: unit
        ref: "cargo test -p ontoforge-schema --lib lint::naming::w03 (triggers, clean; insta snapshots)"
        status: pass
    human_judgment: false
  - id: D4
    description: "W04 reports the minority date naming style across object types; tie reports the prefix style; string properties ignored"
    requirement: LINT-02
    verification:
      - kind: unit
        ref: "cargo test -p ontoforge-schema --lib lint::naming::w04 (triggers, equal_counts_report_the_b_style, clean)"
        status: pass
    human_judgment: false
  - id: D5
    description: "W05 reports pipeline bookkeeping word runs; createdAt, flightNumber, receivedBy stay clean"
    requirement: LINT-02
    verification:
      - kind: unit
        ref: "cargo test -p ontoforge-schema --lib lint::naming::w05 (triggers, clean; insta snapshots)"
        status: pass
    human_judgment: false
  - id: D6
    description: "aviation example still lints clean with the five rules registered"
    requirement: LINT-02
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-schema --test lint_examples (aviation_lints_clean); cargo run -q -p ontoforge -- lint examples/ontologies/aviation --deny-warnings --format json -> [] exit 0"
        status: pass
    human_judgment: false

duration: 15 min
completed: 2026-10-16
status: complete
---

# Phase 1 Plan 10: Naming lint rules ONT-W01 to ONT-W05 Summary

**Five naming warnings (generic or plural type names, ambiguous generic properties, encoded names, mixed date styles, pipeline bookkeeping fields) run through `ontoforge lint` via `naming::rules()`, each with a trigger and a clean insta snapshot, and the aviation example stays at zero findings.**

## Performance

- **Duration:** about 15 min (first commit 00:34:57 +08:00, last 00:40:00; the start time was not captured)
- **Tasks:** 2, both TDD with a RED `test(01-10)` commit before the GREEN `feat(01-10)` commit
- **Files:** 16 in the diff against the base commit (before this SUMMARY)

## Accomplishments

- `naming/mod.rs` registers W01 to W05 and holds the shared pieces: `type_finding` (apiName value), `property_finding` (key under `properties`), `off_hint`, `last_word`, and the test-only `testing::{objects, run, flagged}` helpers. `lint/mod.rs`, `structure/`, `Cargo.toml` and `Cargo.lock` were not touched.
- W01: all words generic, or a plural last word by suffix heuristic with an 18-word singular whitelist; the help proposes the singular spelling.
- W02: bare `value/type/status/date/name/quantity/score` is reported only when another property of the same object type ends in the same word; the message names the twins.
- W03: four fixed regexes held in a `LazyLock<[Regex; 4]>`; the message states which pattern matched (T-01-33).
- W04: classifies `date` and `timestamp` properties across all object types as suffix style or prefix style and reports the minority.
- W05: `split_camel` words are searched for seven bookkeeping runs; the `sequenceNumber` trade-off is in the module doc.

## Task Commits

1. **Task 1 (TDD):** RED `80ca867` (6 of 6 tests failed: the stub rules found nothing), GREEN `b65151b`
2. **Task 2 (TDD):** RED `6020a70` (5 of 5 new tests failed), GREEN `3d35d7e`

## TDD Gate Compliance

RED before GREEN for both tasks. Each RED commit contains compiling stub rules registered in `rules()`, so the failures are assertion failures and missing snapshots rather than compile errors. No REFACTOR commits were needed.

## Reuse decision

Question from the project rule: can `convert_case` replace `split_camel` (REUSE-AUDIT CONSIDER-LATER)? No, `split_camel` stays.

- crates.io (queried 2026-10-16): `convert_case` 0.12.0, 140,187,443 recent downloads, updated 2026-08-27, one runtime dependency `unicode-segmentation`.
- Neither `convert_case` nor `unicode-segmentation` is in `Cargo.lock`. Adopting it adds two new crates to audit and download, and needs `Cargo.toml` / `Cargo.lock` edits that this wave-6 plan is forbidden to make.
- Semantics would have to be re-created with a hand-picked boundary set (acronym, digit-letter both ways, lower-upper) and then proven equal to the `split_camel_cases` test; there is no gain over 25 lines that are already tested and have a real caller (W01, W02, W04, W05).
- `heck` (already a workspace dep) cannot do it either: per the audit it does not split at letter/digit boundaries.

Revisit only if a later rule needs configurable boundaries.

## Verification output

```
cargo test -p ontoforge-schema --lib lint::naming                -> 11 passed (w01 2, w02 2, w03 2, w04 3, w05 2)
cargo test -p ontoforge-schema --test lint_examples              -> 1 passed
cargo test --workspace                                           -> all ok (ontoforge-schema lib 70, cli_lint 14, ...)
cargo clippy --workspace --all-targets -- -D warnings            -> clean
cargo fmt --all --check                                          -> clean
cargo run -q -p ontoforge -- lint examples/ontologies/aviation --deny-warnings --format json -> [] exit 0
fd -e new . crates -H -I                                         -> no .snap.new
rg -n "w04::RULE|w05::RULE" lint/naming/mod.rs                   -> match (line 17)
10 .snap files under lint/naming/snapshots (w01..w05, triggers and clean)
```

## Decisions Made

See `key-decisions` in the frontmatter. No choice contradicts an ADR; no regex backtracks (`regex` crate only).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug in plan fixture] `nTotal` cannot trigger `^n[A-Z]{2,}`**
- **Found during:** Task 1 GREEN (`w03::tests::triggers` failed with `nTotal` missing from the flagged list)
- **Issue:** The behavior list names `nTotal` as a trigger and `nCount` ("only one capital after n") as clean, but both have exactly one capital, so the pattern required by the must_haves (`^n[A-Z]{2,}`) cannot treat them differently. The clean case and the pattern agree with each other; the trigger fixture was the error.
- **Fix:** trigger fixture uses `nTOTAL`; the regex is unchanged. `nTotal` and `nCount` both pass, which the module doc states.
- **Files modified:** crates/ontoforge-schema/src/lint/naming/w03.rs
- **Commit:** b65151b

**2. [Rule 3 - Blocking] clippy `format_push_string`, `format_collect` and `doc_markdown`**
- **Found during:** Task 1 (clippy gate)
- **Fix:** the test fixture builder joins lines instead of appending `format!` output; `snake_case` in backticks.
- **Commit:** b65151b

### Small additions to plan wording

- W04 message names both styles and one example of the majority; the help says which style to follow. The plan listed "examples of both styles" in the message and "unify to the majority" in the help.
- Added test `equal_counts_report_the_b_style` for the tie rule.

**Total deviations:** 2 auto-fixed (1 Rule 1, 1 Rule 3), plus the additions above. No scope change.

## Issues Encountered

- A stray `python -` with an empty heredoc hung a Bash call and was killed (PID 34560, started by this run); a delayed `mv` from the same call duplicated the tail of `w01.rs` once, which was repaired before any build or commit used it.

## Known Stubs

None.

## Threat Model Status

| Threat | Disposition | Status |
|---|---|---|
| T-01-33 regex DoS in W03 | mitigate | four compile-time-fixed patterns, `regex` crate (linear time); names are capped at 100 characters by V07 |
| T-01-34 W04 cross-type collection | accept | one linear pass over all properties |

## Threat Flags

None.

## Next Phase Readiness

- `[lint.options."ONT-W0x"]` for W01 to W05 now reports "takes no options" (V15), which is correct because none declares options.
- 01-11 fills `structure::rules()`; the union stays clean because `aviation_lints_clean` asserts zero findings per registered rule.
- Heuristic caveats to remember when users report false positives: W01 plural suffix guess, W05 `sequenceNumber`.

## Self-Check: PASSED

Files verified present: lint/naming/{mod,w01,w02,w03,w04,w05}.rs and the 10 snapshots. Commits `80ca867`, `b65151b`, `6020a70`, `3d35d7e` exist; `git rev-list --count 258086b..HEAD` = 4 before this SUMMARY commit.
