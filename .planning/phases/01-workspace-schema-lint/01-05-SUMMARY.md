---
phase: 01-workspace-schema-lint
plan: 05
subsystem: schema-link-types
tags: [rust, schema, validation, link-type, cardinality, aviation-example]

requires:
  - phase: 01-workspace-schema-lint
    provides: "01-03 loader (discovery, dispatch, merge, SourceMap); 01-04 ObjectType fields, pk_suitability, validate/objects.rs, aviation/aviation-json objects and load_examples.rs"
provides:
  - "ir::link::{Cardinality, LinkEnd, LinkType} (serde camelCase, deny_unknown_fields, schemars, accessor semantics in doc comments)"
  - "Definition::LinkType and Ontology.link_types (BTreeMap<LinkTypeName, LinkType>, serde default)"
  - "loader merge over a Defs struct: duplicate ObjectType or LinkType is V08 with the first location"
  - "validate/links.rs: V09 (end object type, foreignKey property, with did-you-mean), V11 (manyToMany foreignKey, missing foreignKey, foreignKey on from end, foreignKey type vs from-end primary key type), V08 (end accessor vs property, accessor vs accessor on the same object type)"
  - "aviation and aviation-json with five link types (3 oneToMany, 1 oneToOne, 1 manyToMany)"
  - "4 invalid fixtures with snapshots: v09, v11 x2, v08 (link end)"
affects: [01-06, 01-07, 01-08, 01-09, 01-10, 01-11]

plan_head_before: 6c9df1238ff287cd8307d8162633ac7f1382efc4
actuals:
  tokens: 6600
  tasks: 3
  commits: 5

tech-stack:
  added: []
  patterns:
    - "merge() derives (DefKind, label, apiName, duplicate) from the Definition variant, so a new kind adds one match arm"
    - "A link with a missing end type reports V09 only; later checks take the resolved ObjectType pair and cannot run without it"
    - "Cross-link state (accessors taken per object type) is one map threaded through the BTreeMap-ordered loop, which keeps the 'later definition is reported' rule deterministic"

key-files:
  created:
    - crates/ontoforge-schema/src/ir/link.rs
    - crates/ontoforge-schema/src/validate/links.rs
    - crates/ontoforge-schema/tests/link_types.rs
    - examples/ontologies/aviation/links.yaml
    - examples/ontologies/aviation-json/links.json
    - examples/ontologies/invalid/v09-link-unknown-object-type/{ontology,defs}.yaml
    - examples/ontologies/invalid/v11-many-to-many-with-foreign-key/{ontology,defs}.yaml
    - examples/ontologies/invalid/v11-foreign-key-type-mismatch/{ontology,defs}.yaml
    - examples/ontologies/invalid/v08-link-end-clashes-property/{ontology,defs}.yaml
    - crates/ontoforge/tests/snapshots/invalid_examples__{v09-link-unknown-object-type,v11-many-to-many-with-foreign-key,v11-foreign-key-type-mismatch,v08-link-end-clashes-property}.snap
  modified:
    - crates/ontoforge-schema/src/ir/mod.rs
    - crates/ontoforge-schema/src/loader/mod.rs
    - crates/ontoforge-schema/src/loader/dispatch.rs
    - crates/ontoforge-schema/src/validate/mod.rs
    - crates/ontoforge-schema/tests/load_examples.rs

key-decisions:
  - "Link end apiName is the accessor on THAT end's objectType (ADR-0012, user-confirmed); V08 accessor namespaces are therefore keyed by the end's own objectType"
  - "A link whose end object type is missing is skipped entirely after V09, including the accessor check of its other, valid end"
  - "foreignKey with an unknown property is V09 (reference), a type mismatch is V11; a from-end primary key that is missing or not in properties is left to V10"
  - "dispatch.rs error mapping untouched: LinkType only needed to be added to IMPLEMENTED_KINDS"

requirements-completed: [SCHEMA-04, SCHEMA-06]

coverage:
  - id: D1
    description: "LinkType is a standalone kind: LinkType document with oneToOne, oneToMany and manyToMany, both ends written in it, an apiName per direction"
    requirement: SCHEMA-04
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-schema --test link_types (parses_all_three_cardinalities, link_ir_is_typed)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Malformed link documents map to the existing codes: unknown cardinality V06 with suggestion, unknown end field V04 with suggestion, duplicate link V08 with first location, non camelCase link apiName V07 with canonical spelling"
    requirement: SCHEMA-06
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-schema --test link_types (unknown_cardinality_is_v06, link_end_unknown_field_is_v04, duplicate_link_type_is_v08, link_type_api_name_must_be_camel_case)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Unknown end object type or foreignKey property is V09 with a near-name suggestion; checks for the rest of that link are skipped"
    requirement: SCHEMA-06
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-schema --test link_types (unknown_end_object_type_is_v09, unknown_foreign_key_property_is_v09, link_checks_are_skipped_for_missing_types)"
        status: pass
      - kind: integration
        ref: "invalid_examples__v09-link-unknown-object-type.snap (defs.yaml:19:19, help did you mean `Flight`?)"
        status: pass
    human_judgment: false
  - id: D4
    description: "Cardinality and foreignKey consistency is V11: manyToMany with a key, oneToOne/oneToMany without one, key on the from end, key type different from the from-end primary key type (no numeric folding)"
    requirement: SCHEMA-06
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-schema --test link_types (many_to_many_with_foreign_key_is_v11, one_to_many_without_foreign_key_is_v11, foreign_key_on_from_end_is_v11, foreign_key_type_must_match_primary_key)"
        status: pass
      - kind: integration
        ref: "invalid_examples__v11-many-to-many-with-foreign-key.snap (17:64), invalid_examples__v11-foreign-key-type-mismatch.snap (19:66, message names integer, string and Airport.code)"
        status: pass
    human_judgment: false
  - id: D5
    description: "End accessors are unique per object type and differ from its properties (V08), including self links; the later definition in apiName order is reported"
    requirement: SCHEMA-06
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-schema --test link_types (end_name_clashes_with_property_is_v08, duplicate_end_names_on_same_type_is_v08, self_link_ends_must_differ, self_link_with_distinct_ends_is_valid)"
        status: pass
      - kind: integration
        ref: "invalid_examples__v08-link-end-clashes-property.snap (19:36)"
        status: pass
    human_judgment: false
  - id: D6
    description: "aviation has 5 link types over the 3 cardinalities, and aviation (YAML) and aviation-json load to equal IR with no diagnostics"
    requirement: SCHEMA-04
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-schema --test load_examples (3 passed; yaml == json, 5 object types, 5 link types)"
        status: pass
      - kind: other
        ref: "cargo run -q -p ontoforge -- validate examples/ontologies/aviation --format json -> [] exit 0; same for aviation-json"
        status: pass
    human_judgment: false

duration: 8 min
completed: 2026-10-15
status: complete
---

# Phase 1 Plan 05: Link types, cardinality checks and the aviation links Summary

**Link types (1:1, 1:N with a foreign key, M:N with a system join) are now a first-class definition kind with located V09/V11/V08 diagnostics, and the aviation example carries five of them in both YAML and JSON with equal IR.**

## Accomplishments

- `ir/link.rs`: `Cardinality { OneToOne, OneToMany, ManyToMany }`, `LinkEnd`, `LinkType`; every field has a doc comment that states the accessor semantics (an end's `apiName` lives on that end's own `objectType`, the reverse of Foundry's wording) and where `foreignKey` may appear.
- `ir/mod.rs` and `loader`: `Definition::LinkType`, `Ontology.link_types`, `LinkType` added to `IMPLEMENTED_KINDS`. `merge` now collects into a `Defs` struct and treats duplicate ObjectType and LinkType uniformly (V08, message `duplicate LinkType \`x\`; first defined at a.yaml:2:10`).
- `validate/links.rs`, run after `objects::check`:
  - V09 at the `objectType` value (`links.yaml:5:19`, help `did you mean \`Flight\`?`) and at the `foreignKey` value when it is not a property of the `to` type.
  - V11 for manyToMany with a key (at the key value), a missing key (at the `cardinality` value), a key on the `from` end, and a key type that differs from the `from` primary key type (at the key value; message `foreignKey \`departureAirportCode\` has type \`integer\`, but the primary key \`Airport.code\` has type \`string\``).
  - V08 at the later end's `apiName` value for an accessor equal to a property or to an earlier link's accessor on the same object type; links are visited in apiName order.
- Four invalid fixtures with reviewed snapshots (v09 `defs.yaml:19:19`; v11 many-to-many `17:64`; v11 key type `19:66`; v08 `19:36`).
- Aviation: `flightDepartureAirport`, `flightArrivalAirport`, `aircraftFlights` (oneToMany), `aircraftCurrentMaintenance` (oneToOne), `flightCrew` (manyToMany) in `links.yaml` (5 documents) and `links.json` (5-element array).

## Task Commits

1. **Task 1 (TDD):** RED `e9cefb7`, GREEN `c0b45cd`
2. **Task 2 (TDD):** RED `ef69478`, GREEN `2177fce`
3. **Task 3:** `87cb800`

RED evidence: Task 1, all 5 tests failed on assertions against `ONT-V03 kind LinkType is not implemented yet` (0 passed, 5 failed); the test file reads the IR through `serde_json::to_value` so it compiled against the old IR. Task 2, 10 of 17 tests failed with `left: 0, right: 1` on the diagnostic count; the 7 that passed are the Task 1 tests plus `self_link_with_distinct_ends_is_valid`, a regression guard.

## Verification output

```
cargo test --workspace                                  -> all ok (ontoforge-schema: lib 25, diagnostics 15, link_types 17, load_examples 3, loader 28, object_types 14; ontoforge: cli 5, invalid_examples 1, ...; skeleton crates 1 each)
cargo clippy --workspace --all-targets -- -D warnings   -> clean
cargo fmt --all --check                                 -> clean
cargo test -p ontoforge-schema --test link_types -- parses_all_three_cardinalities -> 1 passed
cargo test -p ontoforge --test invalid_examples         -> 1 passed (10 fixtures)
cargo run -q -p ontoforge -- validate examples/ontologies/aviation --format json       -> [] (exit 0)
cargo run -q -p ontoforge -- validate examples/ontologies/aviation-json --format json  -> [] (exit 0)
rg -c "^kind: LinkType" examples/ontologies/aviation/links.yaml                        -> 5
jq -e 'length == 5 and (map(.cardinality) | sort | unique == [...])' aviation-json/links.json -> true
fd -e new . crates -H -I                                -> no .snap.new
rg "kn_id|BKN|ODL|Domain Pack" crates examples          -> no match
```

## Interface changes

- New public API in `ontoforge_schema::ir`: `Cardinality`, `LinkEnd`, `LinkType`, `LinkTypeName`/`LinkEndName` re-exports, `Definition::LinkType`, `Ontology::link_types` (the last two are breaking for exhaustive matches; none exist outside this crate).
- `ONT-V08`, `ONT-V09`, `ONT-V11` are now emitted. No code was added or renamed.

## Decisions Made

See `key-decisions` in the frontmatter. The accessor direction follows ADR-0012 as confirmed by the user; no implementation choice contradicted an ADR.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug in plan test data] `oneToSome` does not suggest `oneToMany`**
- **Found during:** Task 1 (GREEN run of `unknown_cardinality_is_v06`)
- **Issue:** The plan asks for `cardinality: oneToSome` to suggest `oneToMany`. The shared `did_you_mean` (case-insensitive Jaro-Winkler) scores `onetosome` higher against `onetoone` than `onetomany`, and the helper returned `did you mean \`oneToOne\`?`.
- **Fix:** The test uses `oneToMan`, which suggests `oneToMany`. The shared helper was not changed (it also serves keys, base types and kinds).
- **Files modified:** crates/ontoforge-schema/tests/link_types.rs
- **Commit:** c0b45cd

**2. [Rule 3 - Blocking] clippy `redundant_closure_for_method_calls`**
- **Found during:** Tasks 1 and 2 (clippy gate)
- **Fix:** replaced `|n| n.as_str()` with `ObjectTypeName::as_str` / `PropertyName::as_str` in `links.rs` and the new test.
- **Commit:** c0b45cd, 2177fce

**3. [Rule 2 - Missing guard] `load_examples` did not prove the links were loaded**
- **Found during:** Task 3
- **Issue:** The YAML = JSON equality test would also pass if both directories silently dropped their link files.
- **Fix:** `aviation_yaml_and_json_produce_identical_ir` now also asserts 5 object types and 5 link types (4 added lines; the plan said the file needs no change, so this is an addition, not a fix of a failure).
- **Files modified:** crates/ontoforge-schema/tests/load_examples.rs
- **Commit:** 87cb800

**Total deviations:** 3 (1 Rule 1 in test data, 1 Rule 3, 1 Rule 2). No scope change.

## Issues Encountered

- Column numbers I wrote into the first drafts of several tests were miscounted by hand (for example 62 instead of 64 for the manyToMany `foreignKey` value). Each was recounted from the YAML text before the expectation was changed; the implementation was not adjusted to fit.
- A stray no-op Bash call (an empty `python` heredoc) hung in the background for the 120 s timeout; it touched nothing.

## Known Stubs

None.

## Threat Model Status

| Threat | Disposition | Status |
|---|---|---|
| T-01-15 foreignKey vs primary key type | mitigate | V11 is a hard error with exact `BaseType` equality (no numeric folding); test and fixture fix the behavior |
| T-01-16 accessor vs property name | mitigate | V08 rejects both accessor = property and accessor = accessor on one object type; three tests and one fixture |
| T-01-17 many links, duplicate check cost | accept | one `HashMap` lookup per end, linear in the number of links |

## Threat Flags

None.

## Next Phase Readiness

01-06 can add an `ActionType` arm next to `LinkType` in `loader::merge` (`Defs`, `IMPLEMENTED_KINDS`, `Definition`); `addLink` / `removeLink` rules can resolve link names through `Ontology.link_types` and `validate::links` shows the located-V09 pattern. `aviation` / `aviation-json` can receive action files without editing `load_examples.rs`, apart from its count assertions.

## Self-Check: PASSED

Files verified present: ir/link.rs, validate/links.rs, tests/link_types.rs, aviation/links.yaml, aviation-json/links.json, the 4 invalid fixture directories and 4 snapshots. Commits `e9cefb7`, `c0b45cd`, `ef69478`, `2177fce`, `87cb800` exist; `git rev-list --count 6c9df12..HEAD` = 5 before this SUMMARY commit.
