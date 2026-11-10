---
phase: 01-workspace-schema-lint
plan: 11
subsystem: lint-structure-rules
tags: [rust, lint, structure, regex, insta, ont-w06, ont-w07, ont-w08, ont-w09, ont-w10, ont-w11]

requires:
  - phase: 01-workspace-schema-lint
    provides: "01-08 lint framework (LintRule, Finding, LintCx options, registry, structure::rules() seam, lint_rule_on); 01-10 naming rules pattern and split_camel users"
provides:
  - "ONT-W06 God Object (static part), W07 Silo, W08 Rule of three, W09 Action Sprawl, W10 Time Machine, W11 generic link names"
  - "lint::structure::rules() returns the six rules; every rule is default warn"
  - "Options: [lint.options.\"ONT-W06\"] max_properties, [lint.options.\"ONT-W07\"] silo_prefixes, [lint.options.\"ONT-W09\"] max_actions_per_type"
  - "insta trigger and clean snapshots for each rule under lint/structure/snapshots (12 files)"
  - "Phase 1 gate (DX-03) green: cargo test --workspace, clippy -D warnings, fmt --check"
affects: [01-12]

plan_head_before: c0920b5a4eb1eaa0f78c4042160104c7883b0033
actuals:
  tokens: 14000
  tasks: 2
  commits: 4

tech-stack:
  added: []
  patterns:
    - "One module per rule (w06..w11) with a static RULE; shared type_finding and off_hint live in structure/mod.rs, property and link findings are built in the one rule that needs them"
    - "Test fixtures built by structure::testing::{object, join, run, run_with, flagged}; the primary key name is a parameter so cross-type rules (W07, W08) control which keys are shared"
    - "Cross-type rules group before comparing: W07 by primary key (name, type), W08 by keys held by at least three types (T-01-35)"

key-files:
  created:
    - crates/ontoforge-schema/src/lint/structure/w06.rs
    - crates/ontoforge-schema/src/lint/structure/w07.rs
    - crates/ontoforge-schema/src/lint/structure/w08.rs
    - crates/ontoforge-schema/src/lint/structure/w09.rs
    - crates/ontoforge-schema/src/lint/structure/w10.rs
    - crates/ontoforge-schema/src/lint/structure/w11.rs
    - crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w{06,07,08,09,10,11}__tests__{triggers,clean}.snap
  modified:
    - crates/ontoforge-schema/src/lint/structure/mod.rs

key-decisions:
  - "W06 counts any string property whose last word is type or subtype as a discriminator, including a bare `type`; the discriminator branch needs more than max_properties / 2 properties, so a small type with `type` stays clean"
  - "W07 compares only types with the same primary key name and base type, and reports a similar pair on the later type by apiName; a type can be reported twice (prefix and similarity) because the two findings carry different messages"
  - "W08 merges by exact shared property set: four types sharing the same three properties give one finding; a type that shares a superset with only some of them gives a second finding with its own set"
  - "W09 counts createObject, modifyObject and deleteObject owners only; addLink and removeLink do not make an action belong to a type"
  - "Every structure help text ends with the way to switch the rule off, as in the naming rules"

requirements-completed: [LINT-02, DX-03]

coverage:
  - id: D1
    description: "W06 reports more than max_properties properties, or a string *Type/*Subtype discriminator above half the limit; null-rate branch not implemented (D-13)"
    requirement: LINT-02
    verification:
      - kind: unit
        ref: "cargo test -p ontoforge-schema --lib lint::structure::w06 (triggers, clean, max_properties_option_applies, unknown_option_key_is_rejected; insta snapshots)"
        status: pass
    human_judgment: false
  - id: D2
    description: "W07 reports silo prefixes and same-primary-key types with Jaccard >= 0.6 and at least 4 properties each; silo_prefixes option applies"
    requirement: LINT-02
    verification:
      - kind: unit
        ref: "cargo test -p ontoforge-schema --lib lint::structure::w07 (triggers, clean, silo_prefixes_option_applies; insta snapshots)"
        status: pass
    human_judgment: false
  - id: D3
    description: "W08 reports a property set of 3 or more shared by 3 or more types once per set"
    requirement: LINT-02
    verification:
      - kind: unit
        ref: "cargo test -p ontoforge-schema --lib lint::structure::w08 (triggers, clean, a_shared_set_is_reported_once_for_four_types; insta snapshots)"
        status: pass
    human_judgment: false
  - id: D4
    description: "W09 reports an object type with more than max_actions_per_type actions and single-property set/update modifyObject actions; max_actions_per_type option applies"
    requirement: LINT-02
    verification:
      - kind: unit
        ref: "cargo test -p ontoforge-schema --lib lint::structure::w09 (triggers, clean, max_actions_per_type_option_applies, create_and_modify_count_for_the_same_type; insta snapshots)"
        status: pass
    human_judgment: false
  - id: D5
    description: "W10 reports version, revision, isCurrent properties and year / v-number type names; W11 reports related* and link<digit> link and end names case-insensitively"
    requirement: LINT-02
    verification:
      - kind: unit
        ref: "cargo test -p ontoforge-schema --lib lint::structure::w10 and lint::structure::w11 (triggers, clean, matching_ignores_case; insta snapshots)"
        status: pass
    human_judgment: false
  - id: D6
    description: "aviation example still lints clean with W01 to W11 registered"
    requirement: LINT-02
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-schema --test lint_examples (aviation_lints_clean); cargo run -q -p ontoforge -- lint examples/ontologies/aviation --deny-warnings --format json -> [] exit 0"
        status: pass
    human_judgment: false
  - id: D7
    description: "Phase 1 gate on Windows 11 without Docker: cargo test --workspace, clippy -D warnings, fmt --check"
    requirement: DX-03
    verification:
      - kind: integration
        ref: "cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check"
        status: pass
    human_judgment: false

duration: 8 min
completed: 2026-10-16
status: complete
---

# Phase 1 Plan 11: Structure lint rules ONT-W06 to ONT-W11 Summary

**Six structure warnings (God Object, Silo, Rule of three, Action Sprawl, Time Machine, generic link names) run through `ontoforge lint` via `structure::rules()`, three of them with thresholds configurable in `ontoforge.toml`, each with a trigger and a clean insta snapshot; with 01-10 this completes W01 to W11 and the Phase 1 gate is green.**

## Performance

- **Duration:** about 8 min (first commit 00:43:59 +08:00, last 00:48:36; the start time was not captured)
- **Tasks:** 2, both TDD with a RED `test(01-11)` commit before the GREEN `feat(01-11)` commit
- **Files:** 19 in the diff against the base commit (before this SUMMARY)

## Accomplishments

- `structure/mod.rs` registers W06 to W11 and holds `off_hint`, `type_finding` and the test-only `testing::{object, join, run, run_with, flagged}`. `lint/mod.rs`, `lint/config.rs`, `lint/naming/`, `Cargo.toml` and `Cargo.lock` were not touched.
- W06: property count above `max_properties` (default 60), or a string discriminator whose last `split_camel` word is `type` / `subtype` above half the limit. The null-rate branch is Deferred and said so in the module doc (D-13).
- W07: first name word in `silo_prefixes` (default hr, sales, billing, crm) with at least two words; and pairs with the same primary key name and type, at least 4 properties each, Jaccard of `(name, base type)` sets at least 0.6, reported on the later type with the similarity to two decimals. Types are grouped by primary key before the pairwise comparison.
- W08: keys held by fewer than three types are dropped first, then triples of the remaining candidates are intersected; each distinct shared set is reported once on the first type by `apiName`, listing all types and properties.
- W09: per object type, the set of actions that create it or modify / delete through an `objectReference` parameter, against `max_actions_per_type` (default 10); plus `^(set|update)[A-Z]` actions whose only rule is a `modifyObject` assigning one property.
- W10: `version` / `revision` / `isCurrent` property keys and object type names matching `(\d{4}|[vV]\d+)$`. W11: link and end `apiName` matching `(?i)^(related|link\d)`; the "missing direction" part of the original rule is left to `ONT-V05` and the module doc says so.

## Task Commits

1. **Task 1 (TDD):** RED `0fbb79e` (9 of 10 tests failed on assertions against stub rules; the 10th is the V15 config test), GREEN `db6ee1c`
2. **Task 2 (TDD):** RED `781a788` (9 of 9 new tests failed on assertions and missing snapshots), GREEN `69bb151`

## TDD Gate Compliance

RED before GREEN for both tasks. Each RED commit contains compiling stub rules registered in `rules()`, so failures are assertion failures and missing snapshots, not compile errors (RED run with `INSTA_UPDATE=no`, so no `.snap.new` file was written). Snapshots were created in GREEN with `INSTA_UPDATE=always` and read before committing. No REFACTOR commits were needed.

## Reuse decision

No new crate. `regex` (already a dependency of `ontoforge-schema`, linear-time) covers W09 to W11; `split_camel` from `lint/words.rs` covers W06 and W07; `LintCx::option_usize` / `option_str_list` cover the three options. `did_you_mean` and `heck` have no role in these rules. Jaccard is a two-line formula over `BTreeSet::intersection`, so no similarity crate was considered.

## Verification output

```
cargo test -p ontoforge-schema --lib lint::structure          -> 19 passed (w06 4, w07 3, w08 3, w09 4, w10 2, w11 3)
cargo test -p ontoforge-schema --test lint_examples           -> 1 passed (aviation_lints_clean)
cargo test --workspace                                        -> all ok (ontoforge-schema lib 89, action_types 26, diagnostics 15, link_types 17, lint_examples 1, load_examples 3, loader 28, object_types 14, schema_export 7; ontoforge cli 5, cli_lint 14, cli_schema 4, invalid_examples 1, lib 3; ontoforge-expr analyze 21, criteria 16)
cargo clippy --workspace --all-targets -- -D warnings         -> clean
cargo fmt --all --check                                       -> clean
cargo run -q -p ontoforge -- lint examples/ontologies/aviation --deny-warnings --format json -> [] exit 0
fd -e new . crates -H -I                                      -> no .snap.new
12 .snap files under lint/structure/snapshots (w06..w11, triggers and clean)
rg -n -i "kn_id|\bbkn\b|domain pack" crates examples schemas   -> no match
rg -n "\bfo-[a-z]" crates examples                             -> no match
rg -n -i "[a-z]:\\|/users/|/home/" crates/ontoforge/tests/snapshots crates/ontoforge-schema/src/lint -> no match
```

## Decisions Made

See `key-decisions` in the frontmatter. No choice contradicts an ADR; names are ontoforge's own and no regex backtracks.

## Deviations from Plan

None - plan executed exactly as written.

Small additions to the plan wording, with no behavior outside it: tests `max_properties_option_applies` and `unknown_option_key_is_rejected` (W06), `silo_prefixes_option_applies` (W07), `a_shared_set_is_reported_once_for_four_types` (W08), `max_actions_per_type_option_applies` and `create_and_modify_count_for_the_same_type` (W09), `matching_ignores_case` (W11). The plan's `RelatedThing` case-insensitivity check is a direct test of the matcher, because an end `apiName` must be camelCase and cannot start with a capital in a valid fixture.

## Issues Encountered

None. The phase gate needed no fixes; no root cause lay in another plan's files.

## Known Stubs

None.

## Threat Model Status

| Threat | Disposition | Status |
|---|---|---|
| T-01-35 W07 / W08 cross-type cost | mitigate | W07 compares only within primary key groups; W08 drops keys held by fewer than three types and only types with at least three such keys enter the triple search |
| T-01-36 option values | mitigate | undeclared option keys are V15 through 01-08 config validation (`unknown_option_key_is_rejected`); a value of the wrong type falls back to the default |
| T-01-37 regex DoS | mitigate | three fixed patterns behind `LazyLock`, `regex` crate (linear time) |

## Threat Flags

None.

## Next Phase Readiness

- LINT-02 is complete: W01 to W05 (01-10) and W06 to W11 (this plan) are all registered; `[lint.options."ONT-W06"]`, `"ONT-W07"` and `"ONT-W09"` are accepted, every other W rule reports "takes no options".
- Heuristic caveats for false-positive reports: W07 prefix list is short by default; W08 counts the primary key like any other property, so three types that all use `id` as their key already share one property; W09 treats a setter-named action with one assigned property as a defect even when the name is intentional.
- This is the last plan of Phase 1; the verify step (`/gsd-verify-work`) is next.

## Self-Check: PASSED

Files verified present: lint/structure/{mod,w06,w07,w08,w09,w10,w11}.rs and the 12 snapshots. Commits `0fbb79e`, `db6ee1c`, `781a788`, `69bb151` exist; `git rev-list --count c0920b5..HEAD` = 4 before this SUMMARY commit.
