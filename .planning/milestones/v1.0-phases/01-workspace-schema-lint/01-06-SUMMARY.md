---
phase: 01-workspace-schema-lint
plan: 06
subsystem: schema-action-types
tags: [rust, schema, validation, action-type, rules, value-source, aviation-example]

requires:
  - phase: 01-workspace-schema-lint
    provides: "01-03 loader (dispatch, merge, SourceMap); 01-04 ObjectType and aviation objects; 01-05 LinkType, validate/links.rs pattern, aviation links, load_examples.rs"
provides:
  - "ir::action::{ActionType, ParameterDef, ParamType, Rule, CreateObjectRule, ModifyObjectRule, DeleteObjectRule, LinkRule, ValueSource, Empty, SubmissionCriterion} (ADR-0012 shape, deny_unknown_fields, schemars)"
  - "Definition::ActionType and Ontology.action_types"
  - "dispatch rewrite of rules[*] from {kind: x, ..} to {x: {..}}; kind: function is ONT-V03, unknown rule kind is ONT-V03 with a near name, missing kind is ONT-V02"
  - "basetype::{ValueClass, value_class, check_literal}: numeric class folding and strict literal validation, reusable by Phase 3"
  - "validate/actions.rs: V09 (object type, property, link, parameter, objectReference target) and V12 (primary key and required coverage, primary key modification, parameter kind and link end types, value source vs property type, static and default literals, currentUser/currentTime targets, empty rules, empty criteria)"
  - "aviation and aviation-json with assignPilot, delayFlight, recordMaintenance"
  - "6 invalid fixtures with snapshots: v03, v09, v12 x4"
affects: [01-07, 01-08, 01-09, 01-10, 01-11, 01-12, phase-03-actions, phase-07-mcp]

plan_head_before: 470ed87ec256cc1df1863ed5b983282f1410a98c
actuals:
  tokens: 19480
  tasks: 3
  commits: 5

tech-stack:
  added: []
  patterns:
    - "Rule kind rewrite is a second rewrite site next to the document root; error paths survive because to_segs already drops enum segments that name no original key"
    - "Validation context struct (Cx) carries ontology, action and DefKey, so every check locates through one rule_at(i, [keys], part) helper"
    - "A reference that does not resolve is reported once and the dependent type checks are skipped"

key-files:
  created:
    - crates/ontoforge-schema/src/ir/action.rs
    - crates/ontoforge-schema/src/validate/actions.rs
    - crates/ontoforge-schema/tests/action_types.rs
    - examples/ontologies/aviation/actions.yaml
    - examples/ontologies/aviation-json/actions.json
    - examples/ontologies/invalid/{v03-function-rule,v09-rule-unknown-property,v12-create-missing-primary-key,v12-value-source-type-mismatch,v12-modify-assigns-primary-key,v12-current-time-on-string}/{ontology,defs}.yaml
    - crates/ontoforge/tests/snapshots/invalid_examples__{v03-function-rule,v09-rule-unknown-property,v12-create-missing-primary-key,v12-value-source-type-mismatch,v12-modify-assigns-primary-key,v12-current-time-on-string}.snap
  modified:
    - crates/ontoforge-schema/src/ir/mod.rs
    - crates/ontoforge-schema/src/ir/basetype.rs
    - crates/ontoforge-schema/src/loader/mod.rs
    - crates/ontoforge-schema/src/loader/dispatch.rs
    - crates/ontoforge-schema/src/validate/mod.rs
    - crates/ontoforge-schema/tests/load_examples.rs

key-decisions:
  - "objectReference parameters cannot be assigned to properties with { param: x } (V12, help suggests a string parameter carrying the primary key); { static: null } is V12; modifyObject on the primary key is V12 (all as fixed by the plan)"
  - "Numeric assignability is strictly by class: short and byte into integer, float into double are accepted; long into integer, integer into long and anything into decimal are not (RESEARCH scale-property-limits mapping)"
  - "rules is a required field (missing is V05); an empty list is V12"
  - "Defs fields in loader/mod.rs renamed objects/links/actions to satisfy clippy struct_field_names"

requirements-completed: [SCHEMA-05, SCHEMA-01, SCHEMA-06]

coverage:
  - id: D1
    description: "ActionType parses parameters (base types, objectReference, array with items, required, default), five rule kinds in order, four value sources, and submission criteria into the IR"
    requirement: SCHEMA-05
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-schema --test action_types (parses_all_rule_kinds, parses_all_value_sources, parameter_types)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Rule-level syntax errors keep exact locations: function rule V03, unknown rule kind V03 with suggestion, missing kind V02, unknown field inside a rule V04, two-key value source V06"
    requirement: SCHEMA-06
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-schema --test action_types (function_rule_is_v03, unknown_rule_kind_suggests, rule_missing_kind_is_v02, unknown_field_inside_rule_is_v04, value_source_with_two_keys_is_v06)"
        status: pass
      - kind: integration
        ref: "invalid_examples__v03-function-rule.snap (defs.yaml:14:11)"
        status: pass
    human_judgment: false
  - id: D3
    description: "References to missing object types, properties, links, parameters and objectReference targets are V09 with near-name help; dependent checks are skipped"
    requirement: SCHEMA-06
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-schema --test action_types (references_must_exist, object_reference_target_must_exist)"
        status: pass
      - kind: integration
        ref: "invalid_examples__v09-rule-unknown-property.snap (defs.yaml:17:7, did you mean `status`?)"
        status: pass
    human_judgment: false
  - id: D4
    description: "Type and combination errors are V12: createObject missing primary key or required properties, modifyObject assigning the primary key, non-objectReference object parameter, link end type mismatch, value source vs property type with numeric class folding, currentUser on non-string, currentTime on non-timestamp, objectReference param assigned, static null and invalid static or default literals, empty rules and criteria"
    requirement: SCHEMA-05
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-schema --test action_types (26 tests, 18 of them added in the validation RED commit)"
        status: pass
      - kind: integration
        ref: "invalid_examples snapshots v12-create-missing-primary-key (16:5), v12-value-source-type-mismatch (18:24), v12-modify-assigns-primary-key (18:7), v12-current-time-on-string (17:30)"
        status: pass
    human_judgment: false
  - id: D5
    description: "check_literal and value_class are single-point implementations: byte/integer range, decimal strings, date (YYYY-MM-DD), RFC 3339 timestamp, geopoint ranges, array elements"
    requirement: SCHEMA-05
    verification:
      - kind: unit
        ref: "cargo test -p ontoforge-schema --test action_types -- literal_validation value_class_folds_numeric_types"
        status: pass
    human_judgment: false
  - id: D6
    description: "aviation has assignPilot, delayFlight and recordMaintenance in YAML and JSON with equal IR and no diagnostics"
    requirement: SCHEMA-01
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-schema --test load_examples (3 passed; 5 object types, 5 link types, 3 action types)"
        status: pass
      - kind: other
        ref: "cargo run -q -p ontoforge -- validate examples/ontologies/aviation --format json -> [] exit 0; same for aviation-json"
        status: pass
    human_judgment: false

duration: 12 min
completed: 2026-10-15
status: complete
---

# Phase 1 Plan 06: Action types, rule value sources and the aviation actions Summary

**Action types (parameters, five ordered rule kinds, four single-key value sources, submission criteria) now load into the IR and are statically checked with located V09/V12 diagnostics; aviation ships assignPilot, delayFlight and recordMaintenance in YAML and JSON with equal IR.**

## Performance

- **Duration:** about 12 min
- **Started:** 2026-10-15T15:03Z (approximate; the start time was not captured, the base commit is dated 15:03Z)
- **Completed:** 2026-10-15T15:15Z
- **Tasks:** 3 (two TDD, one plain)
- **Files modified:** 29 in the diff against the base commit (before this SUMMARY)

## Accomplishments

- `ir/action.rs`: `ActionType`, `ParameterDef`, `ParamType` (13 base-type unit variants plus `ObjectReference`), `Rule` (externally tagged, `#[schemars(tag = "kind")]`), the four rule structs, `ValueSource` (`Param`, `Static`, `CurrentUser(Empty)`, `CurrentTime(Empty)`), `SubmissionCriterion`. `Definition::ActionType` and `Ontology.action_types` added; `loader::merge` handles duplicate action types (V08) through the existing arm structure.
- `loader/dispatch.rs`: `rewrite_rules` turns each `rules[i]` from `{kind: x, ..}` into `{x: {..}}`. `kind: function` is V03 ("function-backed rules are not supported in v1"), a rule without `kind` or that is not a mapping is V02, an unknown kind falls through to the existing unknown-variant mapping and gets V03 with a near name.
- `ir/basetype.rs`: `ValueClass` / `value_class` (byte, short, integer one class; float, double one class) and `check_literal` (integer ranges by type, decimal number or decimal string, strict `YYYY-MM-DD`, RFC 3339 via chrono, geopoint `lat,lon` ranges, array elements by `items`, null always rejected).
- `validate/actions.rs`: parameter checks (objectReference target V09, array `items` V05/V06, default literal V12, no default on an object reference), submission criteria (empty condition or failureMessage V12), empty rules V12, and per-rule checks for the five kinds including primary key and required coverage on `createObject`, primary key protection on `modifyObject`, link end type matching for `addLink` / `removeLink`, and value source vs property type compatibility.
- Six invalid fixtures with reviewed snapshots; aviation actions in both formats.

## Task Commits

1. **Task 1 (TDD):** RED `a33b226` (8 tests, all failed on `kind ActionType is not implemented yet` or the first assertion on it), GREEN `bea68c4`
2. **Task 2 (TDD):** RED `81a2595` (18 of 26 tests failed on assertions; `literal_validation` failed at `300 as Byte should be rejected` against a stub, `references_must_exist` at `diagnostics: []`), GREEN `ec52a1b`
3. **Task 3:** `b49a647`

**Plan metadata:** the docs commit that carries this file.

## TDD Gate Compliance

RED then GREEN commits exist for both TDD tasks (`test(01-06)` before `feat(01-06)`). The Task 2 RED commit contains stub `value_class` / `check_literal` functions in `basetype.rs` so the test target compiles and the failures are assertions on the planned behavior rather than a compile error; GREEN replaces the stubs. Nothing needed a REFACTOR commit.

## Verification output

```
cargo test --workspace                                  -> all ok (ontoforge-schema: lib 25, action_types 26, diagnostics 15, link_types 17, load_examples 3, loader 28, object_types 14; ontoforge: cli 5, invalid_examples 1; skeleton crates 1 each)
cargo clippy --workspace --all-targets -- -D warnings   -> clean
cargo fmt --all --check                                 -> clean
cargo test -p ontoforge-schema --test action_types -- unknown_field_inside_rule_is_v04 -> 1 passed
cargo test -p ontoforge-schema --test action_types -- literal_validation               -> 1 passed
cargo test -p ontoforge --test invalid_examples         -> 1 passed (22 fixtures)
cargo run -q -p ontoforge -- validate examples/ontologies/aviation --format json       -> [] (exit 0)
cargo run -q -p ontoforge -- validate examples/ontologies/aviation-json --format json  -> [] (exit 0)
rg -c "^kind: ActionType" examples/ontologies/aviation/actions.yaml                    -> 3
jq -e 'length == 3' examples/ontologies/aviation-json/actions.json                     -> true
fd -e new . crates -H -I                                -> no .snap.new
rg "kn_id|BKN|ODL|Domain Pack" crates examples          -> no match
```

## Interface changes

- New public API in `ontoforge_schema::ir`: the action types listed above, `ActionTypeName` / `ParameterName` re-exports, `basetype::{ValueClass, value_class, check_literal}`.
- Breaking for exhaustive matches (none exist outside this crate): `Definition::ActionType`, `Ontology::action_types`.
- `ONT-V09` and `ONT-V12` are now emitted for actions; `ONT-V03` is also emitted for `kind: function`. No code was added or renamed.

## Decisions Made

See `key-decisions` in the frontmatter. No implementation choice contradicted ADR-0012 or ADR-0013; ADR-0013 is why `createObject` coverage is "required properties plus the primary key" (a primary key without `required: true` is already V10).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] clippy `struct_field_names` on the loader's `Defs`**
- **Found during:** Task 1 (clippy gate)
- **Issue:** with three fields all ending in `_types`, `-D warnings` rejected the struct.
- **Fix:** fields renamed `objects`, `links`, `actions`.
- **Files modified:** crates/ontoforge-schema/src/loader/mod.rs
- **Commit:** bea68c4

**2. [Rule 3 - Blocking] clippy `redundant_closure_for_method_calls` and `useless_format`**
- **Found during:** Task 2 (clippy gate)
- **Fix:** `LinkTypeName::as_str` instead of a closure; a plain string literal instead of `format!`.
- **Files modified:** crates/ontoforge-schema/src/validate/actions.rs
- **Commit:** ec52a1b

**3. [Rule 1 - Bug in plan-adjacent test data] column and span expectations written by hand**
- **Found during:** Tasks 1 and 2
- **Issue:** a hand-counted column (20 instead of 19) and the assumption that a mapping node's span covers the whole `{}` were wrong; the parser locates a flow mapping at its opening brace with a one-byte span.
- **Fix:** the expectations in `action_types.rs` were recounted from the YAML text; the implementation was not changed.
- **Commit:** bea68c4, ec52a1b

**4. [Rule 2 - Missing guard] `load_examples` did not prove the actions were loaded**
- **Found during:** Task 3
- **Fix:** one assertion, `action_types.len() == 3`, added next to the object and link counts (the plan listed the file under read_first only).
- **Files modified:** crates/ontoforge-schema/tests/load_examples.rs
- **Commit:** b49a647

### Small additions to plan wording (no behavior outside the plan)

- The V09 location for an unknown objectReference target is the target name (`parameters.<p>.type.objectReference`), more precise than the `type` node the plan named.
- Array parameter `items` follow the property rules (V05 missing, V06 on a non-array), mirroring `validate/objects.rs`.
- The shared test fixture gained `speed: double` and `ceiling: long` on `Flight` to cover float-into-double and long-into-integer.

**Total deviations:** 4 (2 Rule 3, 1 Rule 1 in test data, 1 Rule 2). No scope change.

## Issues Encountered

- One long `bash` heredoc was rejected by the tool before running (unexpected EOF); nothing had been modified, and the test text was added through a temporary file instead.

## Known Stubs

None. The Task 2 RED stubs in `basetype.rs` were replaced in GREEN.

## Threat Model Status

| Threat | Disposition | Status |
|---|---|---|
| T-01-18 modifyObject changes primary key | mitigate | V12 with test `modify_cannot_assign_primary_key` and fixture `v12-modify-assigns-primary-key`; no suppression path |
| T-01-19 static and default literal tampering | mitigate | `check_literal` rejects range, format and type errors for `static` and `default`; tests `literal_validation`, `static_literal_must_match_property_type`, `parameter_default_must_be_valid_literal` |
| T-01-20 currentUser misuse | mitigate | currentUser only on string, currentTime only on timestamp (V12); not evaluated in this phase |
| T-01-21 chrono parse cost | accept | linear parse; input bounded by the 4 MiB file limit |

## Threat Flags

None.

## Next Phase Readiness

01-07 (already merged) and 01-12 can read `ActionType.submission_criteria[*].condition` and `parameters` from the IR; `objects.<name>` in CEL resolves through `ParamType::ObjectReference`. 01-08 (lint) can rely on: modifying the primary key is V12 and not E08, rules run in list order, an unused parameter (the `reason` parameter of `delayFlight`) is currently silent. Phase 3 can call `basetype::check_literal` and `value_class` for runtime parameter validation.

## Self-Check: PASSED

Files verified present: ir/action.rs, validate/actions.rs, tests/action_types.rs, aviation/actions.yaml, aviation-json/actions.json, the 6 invalid fixture directories and 6 snapshots. Commits `a33b226`, `bea68c4`, `81a2595`, `ec52a1b`, `b49a647` exist; `git rev-list --count 470ed87..HEAD` = 5 before this SUMMARY commit.
