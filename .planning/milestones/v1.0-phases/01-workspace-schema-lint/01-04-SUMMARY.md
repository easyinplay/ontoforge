---
phase: 01-workspace-schema-lint
plan: 04
subsystem: schema-object-types
tags: [rust, schema, validation, object-type, base-type, aviation-example]

requires:
  - phase: 01-workspace-schema-lint
    provides: "01-01 IR, SourceMap, dispatch error mapping, snapshot harness; 01-03 recursive discovery and 32 MiB loader thread"
provides:
  - "ObjectType with aliases, hidden, titleKey (ObjectType::effective_title_key) and PropertyDef with displayName, description, items"
  - "BaseType::as_str, PkSuitability and pk_suitability (single table, shared by V10 and the later lint E09), is_title_suitable"
  - "validate/objects.rs: V10 (primary key missing, not required, Invalid type; unknown titleKey), V05 (array without items), V06 (items on non-array)"
  - "aviation example with 5 object types (YAML) and the equivalent aviation-json (objects.json); load_examples.rs as the long-term YAML = JSON regression"
  - "6 invalid fixtures with snapshots: v05, v06, v07, v08, v10 x2"
affects: [01-05, 01-06, 01-08, 01-09, 01-10, 01-11]

plan_head_before: efbbfe6651dabe7ba4279aae13060af87edd77b4
actuals:
  tokens: 10100
  tasks: 3
  commits: 6

tech-stack:
  added: []
  patterns:
    - "Structural checks locate through SourceMap::locate with DefKey + path + Part; one helper closure per object type"
    - "Suitability tables live next to the type they classify so validate and lint cannot drift"

key-files:
  created:
    - crates/ontoforge-schema/tests/object_types.rs
    - crates/ontoforge-schema/tests/load_examples.rs
    - examples/ontologies/aviation-json/{ontology.yaml,objects.json}
    - examples/ontologies/invalid/v05-missing-primary-key/{ontology,objects}.yaml
    - examples/ontologies/invalid/v06-unknown-base-type/{ontology,objects}.yaml
    - examples/ontologies/invalid/v07-bad-object-type-name/{ontology,objects}.yaml
    - examples/ontologies/invalid/v08-duplicate-object-type/{ontology,a,b}.yaml
    - examples/ontologies/invalid/v10-primary-key-not-required/{ontology,objects}.yaml
    - examples/ontologies/invalid/v10-float-primary-key/{ontology,objects}.yaml
    - crates/ontoforge/tests/snapshots/invalid_examples__{v05-missing-primary-key,v06-unknown-base-type,v07-bad-object-type-name,v08-duplicate-object-type,v10-primary-key-not-required,v10-float-primary-key}.snap
  modified:
    - crates/ontoforge-schema/src/ir/object.rs
    - crates/ontoforge-schema/src/ir/basetype.rs
    - crates/ontoforge-schema/src/loader/dispatch.rs
    - crates/ontoforge-schema/src/validate/objects.rs
    - crates/ontoforge-schema/tests/loader.rs
    - examples/ontologies/aviation/objects.yaml

key-decisions:
  - "Primary key must declare required: true explicitly (plan override of ADR-0012 / RESEARCH A3 'implicit required'); IR required is always as written"
  - "titleKey stays Option in the IR; effective_title_key() supplies the primaryKey default, so YAML and JSON IRs are strictly equal"
  - "Discouraged primary key types (date, timestamp, boolean, byte, long) and nested arrays are not validate errors; they are left to lint E09"

requirements-completed: [SCHEMA-02, SCHEMA-03, SCHEMA-06]

coverage:
  - id: D1
    description: "ObjectType accepts apiName, primaryKey, titleKey (default primaryKey), displayName, description, aliases, hidden; properties accept type, required, displayName, description, items"
    requirement: SCHEMA-02
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-schema --test object_types (object_type_optional_fields_default, full_object_type_round_trips)"
        status: pass
    human_judgment: false
  - id: D2
    description: "All 13 v1 base types parse; unknown base type is V06 with a nearest-name suggestion; non-string value for a string field gets a quoting hint"
    requirement: SCHEMA-03
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-schema --test object_types (all_v1_base_types_parse, unknown_base_type_suggests_nearest, string_field_given_number_suggests_quotes, pk_suitability_table)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Primary key must exist, be required and not be float/double/decimal/geopoint/array (V10); titleKey must exist (V10); array needs items (V05); items only on arrays (V06); discouraged keys and nested arrays are not errors"
    requirement: SCHEMA-06
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-schema --test object_types (primary_key_must_be_required, invalid_primary_key_types_are_v10, discouraged_primary_key_is_not_a_validate_error, title_key_must_exist, array_requires_items, items_only_for_arrays, nested_array_is_not_a_validate_error)"
        status: pass
      - kind: integration
        ref: "cargo test -p ontoforge --test invalid_examples (v05, v06, v07, v08, v10 x2 snapshots)"
        status: pass
    human_judgment: false
  - id: D4
    description: "V07 gives the canonical spelling and V08 cites the first definition across files"
    requirement: SCHEMA-06
    verification:
      - kind: integration
        ref: "invalid_examples__v07-bad-object-type-name.snap (help: write it as `Flight`); invalid_examples__v08-duplicate-object-type.snap (first defined at a.yaml:2:10); object_types::duplicate_object_type_reports_first_location"
        status: pass
    human_judgment: false
  - id: D5
    description: "aviation (YAML) and aviation-json load to equal Ontology values with no diagnostics, independent of file order"
    requirement: SCHEMA-01
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-schema --test load_examples (3 passed)"
        status: pass
      - kind: other
        ref: "cargo run -q -p ontoforge -- validate examples/ontologies/aviation --format json -> []; validate aviation-json --color never -> exit 0"
        status: pass
    human_judgment: false

duration: not timed
completed: 2026-10-15
status: complete
---

# Phase 1 Plan 04: Object type fields, base types and structural validation Summary

**Object types now carry the full SCHEMA-02 field set and all 13 base types, `validate` rejects broken primary keys, title keys and `items` with located diagnostics (V10, V05, V06), and the 5-type aviation example loads to the same IR from YAML and from JSON.**

## Accomplishments

- `ir/object.rs`: `aliases`, `hidden`, `title_key` and `effective_title_key()`; `PropertyDef` gained `display_name`, `description`, `items`. Every field has a doc comment that becomes the JSON Schema description. `deny_unknown_fields` unchanged.
- `ir/basetype.rs`: `BaseType::as_str`, `PkSuitability { Valid, Discouraged, Invalid }`, `pk_suitability` (string/integer/short valid; date/timestamp/boolean/byte/long discouraged; float/double/decimal/geopoint/array invalid), `is_title_suitable` (array judged by its element type).
- `validate/objects.rs` rewritten around three checks per object type. Locations: missing primary key at `/primaryKey` value; non-required primary key at the property key (`objects.yaml:5:3`); Invalid type at the `/primaryKey` value; titleKey at its value; missing `items` at the property `type` value (`7:17`); stray `items` at the `items` key (`6:25`). did-you-mean for primaryKey and titleKey.
- Aviation: `Airport`, `Aircraft`, `Flight`, `Pilot`, `MaintenanceRecord` in `aviation/objects.yaml`; `aviation-json/` has the same manifest and an equivalent `objects.json` array. `load_examples.rs` loads whole directories, so 01-05 and 01-06 can add files without editing it.
- Six invalid fixtures with reviewed snapshots (key lines: v05 `1:7 missing required field primaryKey`; v06 `5:17 unknown value strng`, help `did you mean string?`; v07 `2:10`, help ``write it as `Flight` ``; v08 `b.yaml:2:10 ... first defined at a.yaml:2:10`; v10 `5:3 primary key property code must be required`; v10 `3:13 ... has type double`).

## Task Commits

1. **Task 1 (TDD):** RED `9474488`, GREEN `d9dd92f`
2. **Task 2 (TDD):** RED `9c76a28`, GREEN `30d18d0`
3. **Task 3 (TDD):** RED `5871b3d`, GREEN `2a2380a`

RED evidence: Task 1, the test target did not compile against the old IR (12 errors: missing `items`, `display_name`, `title_key`, `pk_suitability`, ...). Task 2, 5 of 14 tests failed at run time (`primary_key_must_be_required`, `invalid_primary_key_types_are_v10`, `title_key_must_exist`, `array_requires_items`, `items_only_for_arrays`); the other 9 passed as regression guards (V08 across files and the nested array were already correct). Task 3, `aviation_yaml_and_json_produce_identical_ir` and `aviation_examples_have_no_diagnostics` failed with `ONT-V14` (missing `aviation-json`); the file-order test passed against the old one-type aviation.

## Verification output

```
cargo test --workspace                                  -> all ok (ontoforge 3 + cli 5 + invalid_examples 1; ontoforge-schema lib 25, diagnostics 15, loader 28, load_examples 3, object_types 14; each skeleton crate 1)
cargo clippy --workspace --all-targets -- -D warnings   -> clean
cargo fmt --all --check                                 -> clean
cargo run -q -p ontoforge -- validate examples/ontologies/aviation --format json   -> [] (exit 0)
cargo run -q -p ontoforge -- validate examples/ontologies/aviation-json --color never -> stdout empty, exit 0 (stderr: ok: ontology `aviation` with 5 object type(s))
rg -c "^kind: ObjectType" examples/ontologies/aviation/objects.yaml -> 5
jq -e 'length == 5' examples/ontologies/aviation-json/objects.json   -> true
fd -e new . crates                                      -> no .snap.new left
rg "kn_id|BKN|ODL|Domain Pack" crates examples          -> no match
```

## Interface changes

- New public API in `ontoforge_schema::ir`: `ObjectType::{aliases, hidden, title_key, effective_title_key}`, `PropertyDef::{display_name, description, items}`, `basetype::{PkSuitability, pk_suitability, is_title_suitable}`, `BaseType::as_str`.
- The error mapping in `dispatch.rs` already produced did-you-mean for unknown base types; only the non-string hint text changed to "quote the value if it is meant to be a string". The v04 snapshots did not change.

## Decisions Made

See `key-decisions` in the frontmatter. ADR-0012 still says "主键属性隐含 required: true"; this plan intentionally chose the explicit form (plan Claude's Discretion). The ADR text is immutable, so the divergence is recorded here and a later docs plan should note it in the user-facing format reference.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Existing loader tests used primary keys without `required: true`**
- **Found during:** Task 2 (full test run after the V10 rule)
- **Issue:** `properties_keep_declared_order` and `multi_document_yaml_and_json_array_are_both_loaded` in `crates/ontoforge-schema/tests/loader.rs` (from 01-03) defined non-required primary keys and started failing with V10, the intended new behavior.
- **Fix:** added `required: true` to those fixtures (lines 448, 449, 458, 459); no assertion changed.
- **Files modified:** crates/ontoforge-schema/tests/loader.rs
- **Commit:** 30d18d0

**2. [Rule 3 - Blocking] clippy `doc_markdown` and `format_push_string`**
- **Found during:** Task 1 (clippy gate)
- **Fix:** backticked `PascalCase` in a doc comment; built the 13-type test document with `[..].concat()` instead of `push_str(&format!(..))`.
- **Commit:** d9dd92f

**3. [Plan note] Extra manifest `description` in the new invalid fixtures**
- The fixtures follow the existing `v04-unknown-field` style (manifest with a one-line description); the plan did not require it.

**Total deviations:** 2 auto-fixed (1 Rule 1, 1 Rule 3), 1 informational. No scope change; the parallel-wave rule about not touching 01-03 loader files was respected (only `tests/loader.rs`, a fixture-only edit, was changed).

## Issues Encountered

- A shell heredoc turned `'\\'` into `'\'` in `load_examples.rs`, which did not compile; replaced by `std::path::MAIN_SEPARATOR` before the RED commit.
- `v05-missing-primary-key` reports no `help` (serde's missing-field path does not add one). Not required by the plan; left as is.

## Known Stubs

None.

## Threat Model Status

| Threat | Disposition | Status |
|---|---|---|
| T-01-12 unknown fields | mitigate | `deny_unknown_fields` kept on `ObjectType` and `PropertyDef`; v04 fixture and snapshot still V04 |
| T-01-13 primary key invariant | mitigate | missing, not required and Invalid type are all V10 and not suppressible; four tests plus two fixtures |
| T-01-14 echo of user input in help | mitigate | help strings built from the user's property names go through the existing CLI `sanitize`; JSON output is escaped by serde_json. No new end-to-end test with a control character |

## Threat Flags

None.

## Next Phase Readiness

01-05 and 01-06 can add `LinkType` / `ActionType` arms to `loader::merge` and `Definition`; `pk_suitability` is ready for the lint E09 rule in 01-08, and `aviation` / `aviation-json` can receive link and action files without editing `load_examples.rs`.

## Self-Check: PASSED

Files verified present: tests/object_types.rs, tests/load_examples.rs, aviation-json/{ontology.yaml,objects.json}, the 6 invalid fixture directories and 6 snapshots, modified IR/validate sources. Commits `9474488`, `d9dd92f`, `9c76a28`, `30d18d0`, `5871b3d`, `2a2380a` exist; `git rev-list --count efbbfe6..HEAD` = 6 before this SUMMARY commit.
