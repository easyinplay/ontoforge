---
phase: 03-edit-store-scope-model
plan: 06
subsystem: auth
tags: [scope, roles-as-code, yaml, json-schema, serde, loader, diagnostics]

requires:
  - phase: 03-edit-store-scope-model
    provides: "03-02 AttrType and rowpred value types; Phase 1 loader (parse, dispatch, SourceMap, budgets) and schema_export"
provides:
  - "ontoforge_schema::ir::{Role, TypeGrant, SqlLevel, L2Grant, Toolset, RoleBinding, Subject, ScopeDefinition, ScopeSet, RoleName}"
  - "ontoforge_schema::{load_scope_dir, load_scope_from_memory, ScopeLoadOutput}: scope directory to ScopeSet with file:line:col diagnostics, no manifest"
  - "DefKind::{Role, RoleBinding} in the SourceMap (RoleBinding registered under its role name)"
  - "dispatch.rs generalised to document sets: ONT-V03 for a Role in an ontology directory and an ontology kind in a scope directory"
  - "schemas/ontoforge-scope.schema.json as the third file of 'ontoforge schema export' (schema_export::scope_schema)"
  - "examples/scopes/aviation/{roles,bindings}.yaml: dispatcher, crewScheduler, maintenanceLead and their bindings (A's columns x B's rows case)"
affects: [03-07, 03-14, 03-16, phase-04-query, phase-05-actions]

plan_head_before: 50ae6bde93c5a7faf8a7bac136d058da69af32f4

actuals:
  tokens: 12100
  tasks: 2
  commits: 4

tech-stack:
  added: []
  patterns:
    - "A second document family reuses the ontology loader pipeline: dispatch is parameterised by DocSet, isolated() is generic over the output type, the in-memory reader is shared"
    - "Externally tagged Subject enum so serde_path_to_error keeps the path to a bad subject"
    - "Subject structure rules run after deserialization with SourceMap positions; the binding-to-role reference check runs after all files are read, so file order does not matter"

key-files:
  created:
    - crates/ontoforge-schema/src/ir/scope.rs
    - crates/ontoforge-schema/src/loader/scope.rs
    - crates/ontoforge-schema/tests/scope_documents.rs
    - crates/ontoforge-schema/tests/snapshots/scope_documents__scope_diagnostics.snap
    - schemas/ontoforge-scope.schema.json
    - examples/scopes/aviation/roles.yaml
    - examples/scopes/aviation/bindings.yaml
  modified:
    - crates/ontoforge-schema/src/ir/mod.rs
    - crates/ontoforge-schema/src/ir/names.rs
    - crates/ontoforge-schema/src/lib.rs
    - crates/ontoforge-schema/src/loader/mod.rs
    - crates/ontoforge-schema/src/loader/dispatch.rs
    - crates/ontoforge-schema/src/loader/discover.rs
    - crates/ontoforge-schema/src/loader/sourcemap.rs
    - crates/ontoforge-schema/src/schema_export.rs
    - crates/ontoforge-schema/tests/schema_export.rs
    - crates/ontoforge/tests/cli_schema.rs

key-decisions:
  - "Subject rules (ONT-V21) and the cross-directory ONT-V03 check live in the Task 1 loader commit, because dispatch and merge are where they naturally sit; Task 2 added the tests that pin them"
  - "An empty scope directory (no yaml/yml/json file) is ONT-V14 instead of an empty ScopeSet, so a mistyped path cannot silently grant nothing"
  - "A group may contain one '|' (namespaced '<iss>|<group>'); only an empty or control-character group is rejected, as the plan specifies. A user must be exactly two non-empty parts around one '|' and may not contain control characters"
  - "TypeGrant.properties is required (a missing key is ONT-V05); an empty allow-list is accepted here and judged by 03-14"
  - "The diagnostics snapshot is the JSON form of the Diagnostic list (the human renderer lives in the CLI crate); named scope_diagnostics rather than scope_diagnostics_human to avoid a misleading name"

patterns-established:
  - "Scope selection (discover::select_scope) is is_candidate plus a yaml/yml/json extension filter, so skipped directories, ontoforge.toml and the byte budgets behave like the ontology loader"

# SCOPE-01 and SCOPE-02 are addressed at document level only: the file format, loading and diagnostics.
# Reference checks against the ontology (type, property, action, pk in allow-list, rowFilter compile) are 03-14; persistence is 03-16; token-to-principal group mapping is 03-07.
# REQUIREMENTS.md is left unticked on purpose (policy decided in 03-01).
requirements-completed: []
requirements-addressed: [SCOPE-01, SCOPE-02]

coverage:
  - id: D1
    description: "A scope directory of Role and RoleBinding documents loads into a ScopeSet; the aviation example yields three roles, their bindings, the dispatcher row filter and the maintenanceLead sql:raw toolset"
    requirement: SCOPE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-schema/tests/scope_documents.rs#loads_aviation_scope_example"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-schema/tests/scope_documents.rs#multi_document_file_loads"
        status: pass
    human_judgment: false
  - id: D2
    description: "Format errors carry file:line:col: a misspelled rowfilter is ONT-V04 with 'did you mean rowFilter', an unknown toolset is ONT-V06 at the value, a binding to an undefined role is ONT-V21 at the role name regardless of file order"
    requirement: SCOPE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-schema/tests/scope_documents.rs#unknown_field_is_v04_with_location"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-schema/tests/scope_documents.rs#toolset_value_is_checked"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-schema/tests/scope_documents.rs#binding_to_unknown_role_is_v21"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-schema/tests/scope_documents.rs#binding_may_precede_its_role_in_file_order"
        status: pass
    human_judgment: false
  - id: D3
    description: "RoleBinding subject rules (D-02, D-04): empty subjects, repeated subject, user without or with extra '|', empty side, empty group are ONT-V21 at the offending subject; a '<iss>|<group>' group is accepted"
    requirement: SCOPE-01
    verification:
      - kind: integration
        ref: "crates/ontoforge-schema/tests/scope_documents.rs#subject_rules"
        status: pass
    human_judgment: false
  - id: D4
    description: "Duplicate role and a second RoleBinding for one role are ONT-V08 naming the first definition's file:line:col"
    requirement: SCOPE-01
    verification:
      - kind: integration
        ref: "crates/ontoforge-schema/tests/scope_documents.rs#duplicate_role_is_v08"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-schema/tests/scope_documents.rs#second_binding_for_role_is_v08"
        status: pass
    human_judgment: false
  - id: D5
    description: "Documents in the wrong directory are ONT-V03 with a help naming the right one (Role / RoleBinding in an ontology directory, ObjectType / SourceBinding in a scope directory); an empty scope directory is ONT-V14"
    requirement: SCOPE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-schema/tests/scope_documents.rs#role_in_ontology_dir_is_v03"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-schema/tests/scope_documents.rs#object_type_in_scope_dir_is_v03"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-schema/tests/scope_documents.rs#empty_scope_directory_is_reported"
        status: pass
    human_judgment: false
  - id: D6
    description: "ontoforge schema export writes a third file ontoforge-scope.schema.json that matches schemas/, discriminates Role / RoleBinding by kind with additionalProperties false, accepts the aviation scope example and rejects misspelled keys, unknown toolsets and ontology kinds"
    requirement: SCOPE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-schema/tests/schema_export.rs#committed_schemas_are_up_to_date"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-schema/tests/schema_export.rs#scope_variants_are_discriminated_by_kind"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-schema/tests/schema_export.rs#aviation_scope_documents_validate"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-schema/tests/schema_export.rs#invalid_scope_documents_fail_schema"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_schema.rs#schema_export_writes_committed_files"
        status: pass
    human_judgment: false
  - id: D7
    description: "The diagnostics for a corpus of scope errors (V03, V04, V08, V21 in several shapes) are pinned with file, line, column and message"
    requirement: SCOPE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-schema/tests/scope_documents.rs#scope_diagnostics"
        status: pass
    human_judgment: false

# Metrics
duration: 21min
completed: 2026-10-19
status: complete
---

# Phase 3 Plan 06: Scope Documents Summary

**Roles-as-code: `kind: Role` / `kind: RoleBinding` files in their own directory load into a `ScopeSet` through the ontology loader's pipeline, with located ONT-V03/04/06/08/21 diagnostics and a third exported JSON Schema.**

## Performance

- **Duration:** 21 min
- **Started:** 2026-10-19T18:47:40Z
- **Completed:** 2026-10-19T19:08:48Z
- **Tasks:** 2
- **Files modified:** 17 (7 created, 10 modified)

## Accomplishments

- `load_scope_dir` / `load_scope_from_memory` reuse the loader's byte and file budgets, path safety, large-stack thread and span-carrying parser; no manifest, every `yaml` / `yml` / `json` file below the root is a candidate.
- `dispatch.rs` is parameterised by document set; the `map_error` mapping and the `kind` rewrite are unchanged, so a typo such as `rowfilter` gets the same V04 plus did-you-mean as in an ontology file (threat T-03-16).
- `RoleBinding` rules: at least one subject, no repeated subject, `user` is `<iss>|<sub>`, `group` is non-empty (namespaced `<iss>|<group>` accepted, threat T-03-17); a binding to an undefined role is reported at the `role` value after all files are read.
- `schemas/ontoforge-scope.schema.json` is generated by `ontoforge schema export` and guarded by the existing drift test; the aviation example carries one scope modeline per YAML document.
- Aviation example: dispatcher (status-based row filter, one column set), crewScheduler (`departureAirportCode in principal.attrs.homeAirports`, different Flight columns) and maintenanceLead (raw SQL, L2 `demo` / `demo.aviation`, `sql:raw` toolset) with bindings to groups and one `ontoforge-dev|mia` user.

## Task Commits

1. **Task 1: tracer, scope directory to ScopeSet with line:col errors**
   - RED `cdf7907` (test): scope IR, stub loader, example, five failing behaviour tests
   - GREEN `b9a7951` (feat): dispatch generalisation, scope loader, subject rules, V08 / V21 checks
2. **Task 2: scope JSON Schema, V03 hints, subject rules, snapshot**
   - RED `4c0e0ae` (test): schema export tests with a stub `scope_schema`, cli_schema expecting three files, V03 / V08 / subject / snapshot tests
   - GREEN `62df00a` (feat): `scope_schema()`, `ScopeFile`, committed `schemas/ontoforge-scope.schema.json`

**Plan metadata:** recorded in the `docs(03-06)` commit that carries this file.

## Files Created/Modified

- `crates/ontoforge-schema/src/ir/scope.rs` - Role, TypeGrant, SqlLevel, L2Grant, Toolset, RoleBinding, Subject, ScopeDefinition, ScopeSet
- `crates/ontoforge-schema/src/ir/names.rs` - `RoleName` (camelCase, same rules as `LinkTypeName`)
- `crates/ontoforge-schema/src/loader/scope.rs` - loader, merge, subject rules, role reference check
- `crates/ontoforge-schema/src/loader/dispatch.rs` - `DocSet`, `dispatch_scope`, misplaced-kind V03
- `crates/ontoforge-schema/src/loader/mod.rs` - generic `isolated`, shared in-memory reader
- `crates/ontoforge-schema/src/loader/discover.rs` - `select_scope`
- `crates/ontoforge-schema/src/loader/sourcemap.rs` - `DefKind::{Role, RoleBinding}`
- `crates/ontoforge-schema/src/schema_export.rs` - `scope_schema`, third `rendered_schemas` entry
- `schemas/ontoforge-scope.schema.json` - exported scope schema
- `examples/scopes/aviation/{roles,bindings}.yaml` - example scope
- `crates/ontoforge-schema/tests/{scope_documents,schema_export}.rs`, `.../snapshots/scope_documents__scope_diagnostics.snap`, `crates/ontoforge/tests/cli_schema.rs` - tests

## Decisions Made

See `key-decisions` in the frontmatter. The two with consequences for later plans: an empty scope directory is an error (03-14 and the CLI can rely on a non-empty `ScopeSet` when `scope` is `Some`), and `TypeGrant.properties` is required but may be empty (03-14 owns the empty-allow-list judgement).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Subject rules and cross-directory V03 landed in Task 1's commit**
- **Found during:** Task 1
- **Issue:** The plan lists the subject rules and the V03 hints under Task 2, but the dispatch generalisation and the merge step that Task 1 requires are exactly where they are implemented; splitting them would have meant writing the code twice.
- **Fix:** Implemented them with Task 1; Task 2's tests for them (`subject_rules`, `role_in_ontology_dir_is_v03`, `object_type_in_scope_dir_is_v03`, `duplicate_role_is_v08`, `second_binding_for_role_is_v08`, snapshot) therefore passed on first run in the Task 2 RED commit. Only the schema export tests were genuinely red there.
- **Files modified:** crates/ontoforge-schema/src/loader/{dispatch,scope}.rs
- **Commit:** b9a7951

**2. [Rule 2 - Missing critical] Empty scope directory reported as ONT-V14**
- **Found during:** Task 1
- **Issue:** A mistyped scope path would load as an empty `ScopeSet` without any diagnostic.
- **Fix:** `assemble_scope` reports "no scope documents found" when no file is selected and no listing error exists.
- **Commit:** b9a7951; test `empty_scope_directory_is_reported`.

---

**Total deviations:** 2 auto-fixed (1 blocking ordering, 1 missing critical)
**Impact on plan:** no scope creep; both stay inside the files the plan lists.

## TDD Gate Compliance

Both tasks have a `test(03-06)` commit before the `feat(03-06)` commit. RED evidence: Task 1 failed on five assertions (stub returned no diagnostics and no scope); Task 2 failed on five schema-export tests against a stub `scope_schema` (kind discrimination, invalid-example rejection, drift, modeline path, draft header). The stubs exist only in the RED commits.

## Issues Encountered

None blocking. The `walk` helper still words its listing error as "cannot list the ontology directory"; it is shared with the ontology loader and was left alone.

## Known Stubs

None.

## Threat Flags

None. The only new surface is reading files from a scope directory, covered by T-03-15 (shared budgets and path safety, tested via the loader's existing limits).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- 03-14 can take a `ScopeSet` plus `SourceMap` and cross-check against an `Ontology`, locating YAML positions via `DefKey { Role | RoleBinding, name }`.
- 03-16 can persist `ScopeSet`; 03-07 must normalise token groups with the same `<iss>|<group>` rule.
- `ontoforge scope plan` is referenced by the V03 help text but the subcommand belongs to a later plan.

## Self-Check: PASSED

- Files exist: ir/scope.rs, loader/scope.rs, tests/scope_documents.rs, the snapshot, schemas/ontoforge-scope.schema.json and both example files.
- Commits exist: cdf7907, b9a7951, 4c0e0ae, 62df00a.
- `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo fmt --all --check` clean; no `.snap.new` files.

---
*Phase: 03-edit-store-scope-model*
*Completed: 2026-10-19*
