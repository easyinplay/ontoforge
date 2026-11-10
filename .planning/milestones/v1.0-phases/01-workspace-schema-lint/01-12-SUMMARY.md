---
phase: 01-workspace-schema-lint
plan: 12
subsystem: expression-cel
tags: [rust, cel, ontoforge-expr, v13, type-inference, submission-criteria, adr-0006]

requires:
  - phase: 01-workspace-schema-lint
    provides: "01-07 analyze, member_paths, offset_to_line_col; 01-06 loader SourceMap and action IR; 01-08 lint_cmd calling pipeline::check"
provides:
  - "ontoforge_expr::check_ontology: per-criterion reference check and conservative type check, all findings as ONT-V13 errors located at the YAML condition value"
  - "ontoforge_expr::infer::{Ty, TypeEnv, TypeIssue, type_issues}: bool/int/double/string/list inference, everything else Any"
  - "pipeline::check runs validate, then CEL check; V13 is a hard error and clears LoadOutput.ontology"
  - "five invalid/v13-* fixtures with snapshots; ADR-0006 Spike S2 conclusion"
affects: [phase-3-action-evaluation]

plan_head_before: 18e440237de2f925ab6c815a27a2cfd664dd2ba9
actuals:
  tokens: 7100
  tasks: 3
  commits: 5

tech-stack:
  added: []
  patterns:
    - "Reference walk and type inference both run inside one analyze closure, so every AST recursion stays on the 32 MiB analyzer thread; check.rs handles only flat result lists"
    - "Diagnostics dedupe per condition by message, so repeated uses of the same bad path report once"
    - "Type inference treats comprehension macros structurally: accu_var takes the type of accu_init, the iteration variable the element type of iter_range, the node the type of result"

key-files:
  created:
    - crates/ontoforge-expr/src/check.rs
    - crates/ontoforge-expr/src/infer.rs
    - crates/ontoforge-expr/tests/criteria.rs
    - examples/ontologies/invalid/v13-syntax-error/
    - examples/ontologies/invalid/v13-unknown-parameter/
    - examples/ontologies/invalid/v13-objects-not-object-reference/
    - examples/ontologies/invalid/v13-non-boolean-condition/
    - examples/ontologies/invalid/v13-comparison-type-mismatch/
    - crates/ontoforge/tests/snapshots/invalid_examples__v13-syntax-error.snap
    - crates/ontoforge/tests/snapshots/invalid_examples__v13-unknown-parameter.snap
    - crates/ontoforge/tests/snapshots/invalid_examples__v13-objects-not-object-reference.snap
    - crates/ontoforge/tests/snapshots/invalid_examples__v13-non-boolean-condition.snap
    - crates/ontoforge/tests/snapshots/invalid_examples__v13-comparison-type-mismatch.snap
  modified:
    - crates/ontoforge-expr/src/lib.rs
    - crates/ontoforge/src/pipeline.rs
    - .planning/adr/0006-expression-language.md

key-decisions:
  - "pipeline::check sets LoadOutput.ontology to None whenever the combined diagnostics contain an error, keeping the documented invariant 'Some only when no error was found'; otherwise `validate` printed 'ok: ontology ...' next to a V13 error"
  - "Problems are deduplicated per condition by message rather than by raw path segments: `params.nope` and `params.nope.a` produce the same finding and are reported once"
  - "Position of a type issue is the cel node offset of the offending call, which for binary operators is the operator token (`params.delayMinutes == 'ten'` reports 1:21)"
  - "Reference messages embed the access path (`unknown parameter x in params.x`, `... in objects.x`) so a params and an objects problem on the same name stay distinct under dedupe"

requirements-completed: [SCHEMA-05, SCHEMA-06]

coverage:
  - id: D1
    description: "Unknown root variable, unknown params.<x>, objects.<x> not an objectReference parameter, unknown objects.<x>.<prop>, unknown user.<f> are ONT-V13 with did-you-mean help; comprehension variables are not reported; one finding per distinct message"
    requirement: SCHEMA-05
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-expr --test criteria (unknown_root_variable, unknown_parameter_suggests, objects_requires_object_reference_param, unknown_object_property_suggests, user_fields_are_id_and_groups, comprehension_variable_is_not_reported, same_path_reported_once)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Too long, too deep and syntactically invalid conditions each become one V13 error; the syntax error carries the in-expression position"
    requirement: SCHEMA-05
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-expr --test criteria (analyzer_errors_become_v13)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Inferable non-bool conditions and incompatible comparisons are V13; decimal, date, timestamp, geopoint, objectReference, unknown functions and unresolved references are Any and never raise a type error"
    requirement: SCHEMA-05
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-expr --test criteria (non_bool_condition, comparison_type_mismatch, unknown_types_are_any, comprehension_result_types, unknown_reference_is_any, builtin_functions_have_known_result_types)"
        status: pass
    human_judgment: false
  - id: D4
    description: "V13 file, line and column point at the YAML condition value and the message ends with the in-expression position"
    requirement: SCHEMA-06
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-expr --test criteria (diagnostic_points_at_condition_value); five invalid_examples snapshots with line 19 column 16"
        status: pass
    human_judgment: false
  - id: D5
    description: "validate runs the CEL check after the IR is built; aviation passes with an empty JSON array, v13 fixtures exit 1"
    requirement: SCHEMA-05
    verification:
      - kind: integration
        ref: "cargo run -q -p ontoforge -- validate examples/ontologies/aviation --format json (exit 0, []); cargo run -q -p ontoforge -- validate examples/ontologies/invalid/v13-unknown-parameter --format json (exit 1); cargo test -p ontoforge --test invalid_examples --test cli"
        status: pass
    human_judgment: false
  - id: D6
    description: "ADR-0006 records the Spike S2 conclusion in place of the open question"
    requirement: SCHEMA-05
    verification:
      - kind: command
        ref: "rg -n 'Spike 结论' .planning/adr/0006-expression-language.md (match); rg -n 'Open \\(spike\\)' (no match)"
        status: pass
    human_judgment: false

duration: 25min
completed: 2026-10-15
status: complete
---

# Phase 1 Plan 12: CEL criteria reference and type checking Summary

**Submission-criteria CEL is now statically checked at validate time: free variables, parameters, object properties and user fields are resolved against the ontology, and a conservative bool/int/double/string/list inference flags non-bool conditions and incompatible comparisons, all as unsuppressible ONT-V13 errors located at the YAML `condition` value.**

## Performance

- **Duration:** about 25 min
- **Completed:** 2026-10-15
- **Tasks:** 3 (Tasks 1 and 2 TDD: 2 test commits, 2 feat commits; Task 3 fixtures)
- **Files:** 3 source files and 1 test file in the crates, 10 fixture files, 5 snapshots, 1 ADR

## Accomplishments

- `check_ontology` iterates action types in BTreeMap order and criteria by index. Per condition it runs `analyze(src, |p| (member_paths(p, src), type_issues(p, src, env)))`, so both walks stay on the 32 MiB analyzer thread (T-01-38). Analyzer errors (too long, too deep, syntax) become one V13 and skip further checks for that condition.
- Reference rules follow D-09 exactly; suggestions use `ontoforge_schema::diag::did_you_mean`, so `ontoforge-expr` gains no `strsim` dependency and no `Cargo.toml` changed.
- `infer.rs`: `Ty { Bool, Int, Double, String, List, Any }`, `TypeEnv::for_action` (parameter types, object-reference property types, `user.id` string, `user.groups` list of string), comparison compatibility (int and double mix, Any matches all, lists compare element types), comprehension typing taken from the cel macro expansion.
- `pipeline::check` appends the CEL diagnostics, re-sorts, and clears `ontology` on any error.
- Five fixtures, each triggering exactly one V13; snapshots show line 19 column 16 (the opening quote of the condition) and a message tail such as `(at 1:21 in expression)`.
- ADR-0006: the `Open (spike)` section is replaced by `## Spike 结论（2026-10-15）`; Decision and Rejected untouched.

## Task Commits

1. **Task 1: reference checking and pipeline wiring** - `1f4217d` (test, RED), `2e20923` (feat, GREEN)
2. **Task 2: conservative type inference** - `bc67543` (test, RED), `cc2410d` (feat, GREEN)
3. **Task 3: V13 fixtures, snapshots, ADR-0006** - `9243f8b` (test)

**Plan metadata:** committed with this SUMMARY (docs).

## TDD Gate Compliance

Both TDD tasks have a `test(01-12)` commit before the `feat(01-12)` commit. RED for Task 1: `check_ontology` was a stub returning an empty list and 8 of 10 tests failed on assertions (the 2 passes are the clean-case tests). RED for Task 2: 4 of 16 tests failed on assertions (`non_bool_condition`, `comparison_type_mismatch`, `comprehension_result_types`, `builtin_functions_have_known_result_types`). No refactor commits were needed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `validate` human summary said "ok" next to a V13 error**
- **Found during:** Task 1, wiring `pipeline::check`
- **Issue:** `validate` prints `ok: ontology ...` when `LoadOutput.ontology` is `Some`; after appending V13 diagnostics the ontology was still `Some`, contradicting the documented "`Some` only when no error was found" contract.
- **Fix:** `pipeline::check` sets `out.ontology = None` when the combined diagnostics contain an error. `lint_cmd` already returns early on errors, so lint behaviour is unchanged.
- **Files modified:** `crates/ontoforge/src/pipeline.rs`
- **Commit:** `2e20923`

**2. [Rule 3 - Blocking] `aviation_criteria_pass` loads through `load_from_memory`, not `load_dir`**
- **Found during:** Task 1, writing `tests/criteria.rs`
- **Issue:** `load_dir` takes `camino::Utf8Path`; `ontoforge-expr` has no `camino` dependency and the plan forbids `Cargo.toml` changes.
- **Fix:** the test reads the four flat aviation YAML files with `std::fs` and calls `load_from_memory`. The `load_dir` path is covered by the aviation CLI check below.
- **Files modified:** `crates/ontoforge-expr/tests/criteria.rs`
- **Commit:** `1f4217d`

**Total deviations:** 2 auto-fixed (1 Rule 1, 1 Rule 3). **Impact:** none on scope.

## Interface notes

- `ParameterDef` exposes `param_type` (not `r#type` as the plan's interface sketch said) and `PropertyDef` exposes `base_type`; `check.rs` and `infer.rs` use the real field names.
- 01-07 files (`limits.rs`, `analyze.rs`, `walk.rs`) were not modified; no interface adjustment was needed.
- `cel` binary-operator calls report the operator token as their offset, so type issues point at the operator (`==`), not the left operand.
- Known limitation, by design (A7): idents like `int` or `string` used as type names (`type(x) == int`) are reported as unknown variables, because the root set is fixed to `params`, `user`, `objects`.

## Verification Results

- `cargo test -p ontoforge-expr --test criteria`: `running 16 tests` ... `test result: ok. 16 passed; 0 failed`
- `cargo test -p ontoforge --test cli --test invalid_examples`: `5 passed` and `1 passed`
- `cargo test --workspace`: every target `ok`, no `FAILED`, no `has overflowed its stack`
- `cargo clippy --workspace --all-targets -- -D warnings`: clean; `cargo fmt --all --check`: clean
- `cargo run -q -p ontoforge -- validate examples/ontologies/aviation --format json`: stdout `[]`, exit 0
- `cargo run -q -p ontoforge -- validate examples/ontologies/invalid/v13-unknown-parameter --format json`: exit 1, `jq -e 'any(.[]; .code == "ONT-V13")'` prints `true`
- Acceptance greps: `check_ontology` in `pipeline.rs`, `did_you_mean` and `type_issues` in `check.rs` match; `strsim` absent from `ontoforge-expr/Cargo.toml`; `Open (spike)` absent from ADR-0006, `Spike 结论` present; no `.snap.new` files; all five snapshots contain `ONT-V13`, the unknown-parameter one contains `delayMinutes`

## Known Stubs

None.

## Threat Flags

None. T-01-38 is mitigated as planned (walk and inference inside the `analyze` closure, no recursion in `check.rs`); T-01-39: messages echo only path segments and names, never the whole expression.

## Next Phase Readiness

Phase 3 evaluates expressions that are already guaranteed to reference only declared parameters, existing properties and `user.id` / `user.groups`. `TypeEnv` is the natural place to tighten Any types once Phase 3 injects real value types.

## Self-Check: PASSED

- Created files present on disk: `check.rs`, `infer.rs`, `tests/criteria.rs`, five `v13-*` fixture directories, five snapshots.
- Commits found in `git log`: `1f4217d`, `2e20923`, `bc67543`, `cc2410d`, `9243f8b`.
