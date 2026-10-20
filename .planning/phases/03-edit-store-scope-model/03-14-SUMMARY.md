---
phase: 03-edit-store-scope-model
plan: 14
subsystem: auth
tags: [scope, roles-as-code, compile, row-filter, criteria-coverage, collation, diagnostics, fail-closed]

requires:
  - phase: 03-edit-store-scope-model
    provides: "03-05 compile_row_filter / PredContext / PredIssue; 03-06 ScopeSet loader and SourceMap; 03-07 Principal; 03-11 merge / RoleTypeGrant / TypeScope; 03-02 AttrType and [auth.principal_attrs]"
provides:
  - "ontoforge_auth::scope::{compile_scope, ScopeInputs, CompiledScope, CompiledRole, PrincipalScope, attr_types}: the one offline check of a scope directory against ontology, profile bindings and declared attributes; every problem is reported, none stops the others"
  - "CompiledScope::for_principal: user / group subject resolution (a role reached twice counts once), per-type merge, union of actions / sqlLevels / L2 / toolsets"
  - "CompiledScope::row_filter_properties: properties read by row predicates per type, for the 03-16 / 03-18 index reconciliation"
  - "scope::coverage::criteria_coverage (ONT-V23, pub(crate)): reusable by the reload re-check in 03-18"
  - "ONT-V21 / V22 / V23 producers with YAML file:line:col (or no position when sources is None)"
affects: [03-16, 03-18, 03-20, phase-04-query, phase-05-actions]

plan_head_before: 47cc0e42d72e28a238072ea3bce744957ed934f1

actuals:
  tokens: 12700
  tasks: 2
  commits: 5

tech-stack:
  added: []
  patterns:
    - "A Locator maps (role, path) to a SourceLocation and yields the all-zero location when there is no SourceMap, so one Diagnostic::error call serves both located and unlocated diagnostics"
    - "Criteria coverage is a plain function over one Role taking a position callback; apply and reload call the same code"
    - "Collation check runs only on a predicate that compiled, against the selected profile or the manifest defaultProfile"

key-files:
  created:
    - crates/ontoforge-auth/src/scope/compile.rs
    - crates/ontoforge-auth/src/scope/coverage.rs
    - crates/ontoforge-auth/tests/scope_compile.rs
    - crates/ontoforge-auth/tests/criteria_coverage.rs
    - crates/ontoforge-auth/tests/snapshots/scope_compile__scope_compile_diagnostics_json.snap
  modified:
    - crates/ontoforge-auth/src/scope/mod.rs
    - examples/ontologies/aviation/bindings/dev/flight.yaml
    - examples/ontologies/aviation-json/bindings/dev/bindings.json

key-decisions:
  - "The example dev binding moved `collation: caseInsensitive` from Flight.status to Flight.flightNumber: the shipped dispatcher role filters on status, which D-08 forbids on a case-insensitive column (see Deviations)"
  - "A computed index (`objects.flight[params.field]`) and a bare `objects` read are ONT-V23 even when the role grants everything: fail-closed, as the plan specifies"
  - "An unlisted action argument that is not an objectReference parameter is skipped in coverage; ONT-V13 owns that error"
  - "A RoleBinding naming a role missing from the ScopeSet is ONT-V21 in compile_scope too (the loader already catches it for files; stored scopes in 03-18 do not go through the loader)"
  - "row_filter_properties lists only types where some row predicate reads a property"

patterns-established:
  - "Diagnostics for roles from the system library (sources None) carry file \"\", line 0, column 0 and name role, type and property in the message"

# SCOPE-01 / 02 / 05 / 06: compile_scope and for_principal are library API with tests on the aviation example and in-memory corpora.
# Persisting and applying a scope is 03-16, the reload re-check is 03-18, scoped reads and action authorization are Phase 4 / 5.
# REQUIREMENTS.md is left unticked on purpose (policy decided in 03-01).
requirements-completed: []
requirements-addressed: [SCOPE-02, SCOPE-05, SCOPE-06, SCOPE-01]

coverage:
  - id: D1
    description: "The aviation example scope compiles against the aviation ontology (profile dev); three roles, dispatcher Flight grant carries a compiled row predicate"
    requirement: SCOPE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-auth/tests/scope_compile.rs#aviation_scope_compiles"
        status: pass
    human_judgment: false
  - id: D2
    description: "A principal holding dispatcher and crewScheduler gets a merged Flight scope: dispatcher's row is visible, the crew scheduler's columns do not show through it; actions are the union"
    requirement: SCOPE-01
    verification:
      - kind: integration
        ref: "crates/ontoforge-auth/tests/scope_compile.rs#principal_with_two_roles_gets_merged_flight_scope"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/scope_compile.rs#user_subject_binding"
        status: pass
    human_judgment: false
  - id: D3
    description: "Fail-closed: a principal without roles sees no type and holds no action; an empty `objectTypes` grants nothing; `properties: []` is ONT-V21 (no primary key)"
    requirement: SCOPE-05
    verification:
      - kind: integration
        ref: "crates/ontoforge-auth/tests/scope_compile.rs#principal_without_roles_sees_nothing"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/scope_compile.rs#empty_object_types_grants_nothing"
        status: pass
    human_judgment: false
  - id: D4
    description: "A rowFilter outside the D-07 subset or reading an undeclared principal attribute is ONT-V22 located at the YAML value with the position inside the expression; a column the selected profile declares caseInsensitive is ONT-V22 (D-08); the check is skipped without a profile"
    requirement: SCOPE-05
    verification:
      - kind: integration
        ref: "crates/ontoforge-auth/tests/scope_compile.rs#bad_row_filter_is_v22_located"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/scope_compile.rs#undeclared_attr_is_v22"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/scope_compile.rs#collation_case_insensitive_is_v22"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/scope_compile.rs#collation_check_uses_the_selected_profile"
        status: pass
    human_judgment: false
  - id: D5
    description: "ONT-V21 reference rules: unknown type, unknown / duplicate property, allow-list without primary key, unknown action, sql:raw toolset or L2 without the raw level, L2 schema not `catalog.schema`"
    requirement: SCOPE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-auth/tests/scope_compile.rs#unknown_object_type_is_v21"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/scope_compile.rs#unknown_property_is_v21"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/scope_compile.rs#allow_list_without_primary_key_is_v21"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/scope_compile.rs#duplicate_allow_list_entry_is_v21"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/scope_compile.rs#unknown_action_is_v21"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/scope_compile.rs#raw_toolset_without_raw_level_is_v21"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/scope_compile.rs#l2_without_raw_level_is_v21"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/scope_compile.rs#l2_schema_must_be_catalog_dot_schema"
        status: pass
    human_judgment: false
  - id: D6
    description: "Criteria column coverage (ONT-V23): a hidden criteria column, an ungranted type and a computed index are refused at the role's `actions` entry; criteria that read no object need nothing"
    requirement: SCOPE-06
    verification:
      - kind: integration
        ref: "crates/ontoforge-auth/tests/criteria_coverage.rs#hidden_criteria_column_is_v23_at_the_action_entry"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/criteria_coverage.rs#ungranted_object_type_is_v23"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/criteria_coverage.rs#dynamic_object_path_is_v23_fail_closed"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/criteria_coverage.rs#criteria_without_object_reads_need_no_properties"
        status: pass
    human_judgment: false
  - id: D7
    description: "All errors of one scope are reported together in file / line / column / code order; without a SourceMap the diagnostics are unlocated but name role, type and property; the error corpus is pinned by a snapshot"
    requirement: SCOPE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-auth/tests/scope_compile.rs#all_errors_reported_together"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/scope_compile.rs#stored_roles_diagnostics_without_sources"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/scope_compile.rs#scope_compile_diagnostics_json"
        status: pass
    human_judgment: false

duration: 25 min
completed: 2026-10-20
status: complete
---

# Phase 3 Plan 14: Scope compilation and criteria coverage Summary

**`compile_scope` turns a role directory plus ontology, profile bindings and declared principal attributes into a `CompiledScope` or into every ONT-V21 / V22 / V23 problem at once (with YAML line and column), and `for_principal` resolves a caller's user and group roles into a fail-closed, merged `PrincipalScope`.**

## Performance

- **Duration:** 25 min (2026-10-20T04:26:47Z to 04:51Z)
- **Tasks:** 2 of 2 (Task 1 is a tracer), 5 commits, 8 files (5 created, 3 modified)
- **Verification:** `cargo test --workspace --locked` no failures (74 test binaries ok), `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo fmt --all --check` clean, no `.snap.new` left

## Accomplishments

- Tracer: the aviation example scope compiles under profile `dev`; a principal in `dispatchers` and `crew-schedulers` gets Flight visible for a `SCHEDULED` row at `MUC` while `aircraftTailNumber` (crew scheduler only, whose row predicate is False) stays hidden; `ontoforge-dev|mia` resolves through a `user` subject to `maintenanceLead` with `Raw` and `SqlRaw`.
- A `rowFilter` of `object.status.size() > 0` is one ONT-V22 at line 15, column 16 of `roles.yaml` (the value, not the key), message "... (at 1:19 in expression)".
- `criteria_coverage` reads `objects.<param>.<prop>` with `member_paths` inside `analyze`; the role needs the parameter's object type and the property in its allow-list, a computed index is refused whatever the role grants.
- D-08: a predicate that reads a column the selected profile (or `defaultProfile`) declares `caseInsensitive` is ONT-V22 with the plan's help text; no profile, no check.
- `compile_scope` never stops at the first error; the diagnostics are sorted by `sort_diagnostics`.

## Task Commits

| Task | Commit | What |
|------|--------|------|
| 1 RED | `4659e5c` | `test(03-14)`: tracer tests against placeholder `compile_scope` / `for_principal` |
| 1 GREEN | `745b3d2` | `feat(03-14)`: type / allow-list / rowFilter checks, `for_principal`, `row_filter_properties`, `attr_types` |
| 2 RED | `44701bc` | `test(03-14)`: V21 / V22 / V23 tests, snapshot test, placeholder `criteria_coverage` |
| 2 fix | `1ee2430` | `fix(03-14)`: example collation moved off `Flight.status` (see Deviations) |
| 2 GREEN | `44e3935` | `feat(03-14)`: `criteria_coverage`, collation check, action / SQL grant checks, snapshot |

## TDD Gate Compliance

Both tasks have a `test(03-14)` commit before their `feat(03-14)` commit. RED evidence: Task 1, 4 of 5 `scope_compile` tests failed on assertions (empty role list, `expect_err` on an accepted bad filter); Task 2, 3 of 5 `criteria_coverage` tests and 9 of 21 `scope_compile` tests failed (`expect_err` / assertion on the missing diagnostics; the snapshot test failed before reaching the snapshot). `gsd_run check tdd-red-evidence` was not run (cargo output is not the TAP it parses), as in 03-05 / 03-11. Some Task 2 tests (`undeclared_attr_is_v22`, `unknown_object_type_is_v21`, `unknown_property_is_v21`, `allow_list_without_primary_key_is_v21`, `duplicate_allow_list_entry_is_v21`, `empty_object_types_grants_nothing`, `raw_with_l2_and_toolset_compiles`) already passed in the Task 2 RED commit because Task 1 implements those checks; they are regression pins for the plan's behavior list.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Aviation example contradicted D-08**
- **Found during:** Task 2 (planning the collation check)
- **Issue:** `examples/ontologies/aviation/bindings/dev/flight.yaml` declared `status: { collation: caseInsensitive }`, and the example dispatcher role (03-06) filters on `object.status`. With D-08 enforced, the plan's tracer expectation (the example scope compiles under profile `dev`) cannot hold.
- **Fix:** moved the case-insensitive collation to `flightNumber` in the YAML and JSON example bindings, with a comment saying why `status` stays case sensitive. The dispatcher role and the 03-06 tests are untouched. `docs/source-binding.md` keeps its generic excerpt (it is not a copy of the example file, and no test reads it).
- **Files modified:** examples/ontologies/aviation/bindings/dev/flight.yaml, examples/ontologies/aviation-json/bindings/dev/bindings.json
- **Verification:** full workspace test run before and after, only the expected RED tests failed before the fix commit.
- **Commit:** `1ee2430`

**2. [Rule 2 - Missing critical] RoleBinding to an undefined role is ONT-V21 in `compile_scope`**
- **Found during:** Task 2
- **Issue:** the loader catches a binding to an undefined role for files, but a `ScopeSet` read back from the system library (03-18) does not pass the loader; the bindings map would otherwise silently keep a dangling entry.
- **Fix:** `compile_scope` reports it at the binding's `role` value. No dedicated test (a `ScopeSet` with a dangling binding cannot come from the loader); covered by code path only.
- **Commit:** `44e3935`

**Total deviations:** 2 auto-fixed (1 bug in an example, 1 missing check). **Impact:** none on the plan's scope; one example file pair changed outside the listed `files_modified`.

## Known Stubs

None.

## Threat Flags

None. T-03-42 (criteria coverage in `criteria_coverage`, computed paths refused), T-03-43 (D-08 check), T-03-44 (`for_principal` matches exact user id or exact group; no role means the type is absent; tested with a roleless principal and an empty role) and T-03-45 (primary key required in every allow-list) are mitigated as planned.

## Issues Encountered

- `python3` is not available on this host (Windows store stub), so a scripted edit failed and was redone with the Edit tool. A multi-line `cat <<'EOF'` in the Bash tool also failed to parse once; nothing was applied.
- The reload-side use of `criteria_coverage` (same function, `sources: None`) is not wired here; it is 03-18's task. The `sources: None` path of `compile_scope` is tested.
- Assumptions flagged by the plan hold as implemented: a role that does not grant a type leaves it invisible; an attribute missing for a principal evaluates to Unknown and hides the rows (inherited from `eval`); a role reached by user and group counts once (`BTreeSet`). Action rule writes are not checked here (Phase 5).

## Next Phase Readiness

- 03-16 can call `compile_scope` before persisting a scope and store `CompiledScope`'s source `ScopeSet`; 03-18 can call it with `sources: None` and `criteria_coverage` stays crate-private, so reload goes through `compile_scope`.
- `row_filter_properties` gives 03-16 / 03-18 the per-type property sets for the ADR-0016 index reconciliation.
- Phase 4 consumes `PrincipalScope.types` (`TypeScope::evaluate` / `lower`).

## Self-Check: PASSED

- Files exist: scope/compile.rs, scope/coverage.rs, tests/scope_compile.rs, tests/criteria_coverage.rs, the snapshot, and the two example binding files.
- Commits `4659e5c`, `745b3d2`, `44701bc`, `1ee2430`, `44e3935` are in `git log`.
- Acceptance greps: `pub fn compile_scope`, `codes::V23` and `member_paths` match; `criteria_coverage` 5 passed, `collation_case_insensitive_is_v22` 1 passed.
- `cargo test --workspace --locked`, clippy `-D warnings` and `fmt --check` clean; no `.snap.new`.
