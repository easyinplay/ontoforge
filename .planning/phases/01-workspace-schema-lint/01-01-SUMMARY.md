---
phase: 01-workspace-schema-lint
plan: 01
subsystem: schema-loader-cli
tags: [rust, serde-saphyr, serde_path_to_error, miette, insta, clap, workspace]

requires: []
provides:
  - "ADR-0012: user-confirmed definition document shape (option recommended)"
  - "Cargo workspace root (resolver 3, edition 2024, rust-version 1.98, workspace lints, all external dependency versions pinned)"
  - "ontoforge-schema: Diagnostic model with ONT-V01..V15 codes, SourceMap.locate, load_dir / load_from_memory, ObjectType IR, apiName newtypes"
  - "ontoforge CLI: validate with human (miette) and JSON output, exit codes 0 / 1"
  - "Data-driven invalid-example snapshot harness (examples/ontologies/invalid/<code>-<slug>/)"
affects: [01-02, 01-03, 01-04, 01-05, 01-06, 01-07, 01-08, 01-09, 01-10, 01-11, 01-12]

plan_head_before: 1e94cb8765b50eca91f987474f2522935d23853c
actuals:
  tokens: 22000
  tasks: 2
  commits: 3

tech-stack:
  added: [serde-saphyr 2.0, serde_path_to_error, schemars, indexmap, strsim, camino, miette, clap, insta, tempfile]
  patterns:
    - "Node tree (Spanned) -> serde_json::Value -> kind rewrap to externally tagged -> serde_path_to_error -> IR; spans live in SourceMap, never in the IR"
    - "Error codes travel through de::Error::custom via custom_de_error / parse_custom_de_error"
    - "Library crate has no miette; CLI render.rs maps Diagnostic to miette"

key-files:
  created:
    - .planning/adr/0012-definition-document-shape.md
    - Cargo.toml
    - Cargo.lock
    - rust-toolchain.toml
    - .gitattributes
    - crates/ontoforge-schema/src/diag.rs
    - crates/ontoforge-schema/src/ir/{mod,names,manifest,object,basetype}.rs
    - crates/ontoforge-schema/src/loader/{mod,discover,parse,dispatch,sourcemap}.rs
    - crates/ontoforge-schema/src/validate/{mod,objects}.rs
    - crates/ontoforge-schema/tests/diagnostics.rs
    - crates/ontoforge/src/{main,lib,cli,pipeline,render}.rs
    - crates/ontoforge/tests/{cli,invalid_examples}.rs
    - crates/ontoforge/tests/snapshots/cli__validate_unknown_field_human.snap
    - crates/ontoforge/tests/snapshots/invalid_examples__v04-unknown-field.snap
    - examples/ontologies/aviation/{ontology,objects}.yaml
    - examples/ontologies/invalid/v04-unknown-field/{ontology,objects}.yaml
  modified: []

key-decisions:
  - "Task 1 (user): option recommended. Link end apiName is the accessor on THAT end's objectType (opposite direction to Foundry wording); recorded in ADR-0012"
  - "LinkType / ActionType kinds report ONT-V03 'not implemented yet' until plans 01-05 / 01-06 add them; Interface / ValueType / SharedProperty report 'reserved and not supported in v1'"
  - "clippy result_large_err allowed workspace-wide: Diagnostic is the loader error type everywhere"

requirements-completed: [DX-03, SCHEMA-01, SCHEMA-06]

coverage:
  - id: D1
    description: "ontoforge validate on the aviation example exits 0 with empty stdout"
    requirement: SCHEMA-01
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge --test cli validate_aviation_succeeds_silently"
        status: pass
    human_judgment: false
  - id: D2
    description: "A misspelled key yields ONT-V04 at objects.yaml:3:1 with a did-you-mean help and exit code 1; underline sits on the key"
    requirement: SCHEMA-06
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge --test cli validate_unknown_field_points_at_the_misspelled_key"
        status: pass
      - kind: integration
        ref: "cargo test -p ontoforge --test invalid_examples (snapshot invalid_examples__v04-unknown-field)"
        status: pass
    human_judgment: false
  - id: D3
    description: "--format json emits exactly code/severity/message/file/line/column/span/help, no ANSI"
    requirement: SCHEMA-06
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge --test cli json_output_has_exactly_the_documented_fields_and_no_ansi"
        status: pass
    human_judgment: false
  - id: D4
    description: "Loader error mapping V01..V05, V07, V08, V10, V14 with node-level locations; YAML and JSON load to the same IR"
    requirement: SCHEMA-01
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-schema --test diagnostics (15 tests)"
        status: pass
    human_judgment: false
  - id: D5
    description: "Workspace root configuration (resolver 3, edition 2024, unsafe_code forbid, eol=lf) and no reference-repo identifiers"
    requirement: DX-03
    verification:
      - kind: other
        ref: "rg checks listed in plan acceptance criteria; cargo clippy --workspace --all-targets -- -D warnings"
        status: pass
    human_judgment: false

duration: n/a (continuation run; Task 1 time not recorded)
completed: 2026-10-15
status: complete
---

# Phase 1 Plan 01: Walking Skeleton Summary

**YAML/JSON to Spanned node tree to IR to `ontoforge validate` with node-level `file:line:col` diagnostics (ONT-V04 points at the misspelled key), in a Rust 1.98 edition-2024 workspace.**

## Performance

- **Duration:** not measured (the continuation segment started after Task 1 was answered; no start timestamp was recorded)
- **Completed:** 2026-10-15
- **Tasks:** 2 (Task 1 decision, Task 2 tracer)
- **Files modified:** 34 committed files by this plan (excluding `Cargo.lock`), 4151 inserted lines overall

## Task 1 outcome (checkpoint:decision)

User chose `recommended`: all of D-02, D-08 and RESEARCH A1 as proposed, including link end `apiName` being the accessor on that end's own `objectType`. Recorded in `.planning/adr/0012-definition-document-shape.md` (status accepted, 2026-10-15). Rejected alternatives: `foundry-link-direction`, `grouped-documents`, `tagged-value-source`.

## Accomplishments

- Workspace root with all external dependency versions fixed in one place, `unsafe_code = "forbid"`, clippy pedantic, `eol=lf` in `.gitattributes`, toolchain pinned to 1.98.0. Later plans do not need to touch the root `Cargo.toml` for dependencies.
- Loader pipeline proven in the real workspace: `serde-saphyr` `Spanned` tree, `kind` rewrap, `serde_path_to_error`, location lookup by `(DefKey, path, Part)`. D-11 node-level spans are exact: ONT-V04 lands on `primaryKy` at 3:1 with a 9-byte span.
- Error mapping for V01 (syntax, duplicate key, depth budget, JSON trailing comma, invalid UTF-8), V02 (shape), V03 (unknown, reserved, not yet implemented kinds), V04, V05, V06 (with did-you-mean for enum values), V07 (format and reserved words with canonical help), V08 (first-definition location in the message), V10 (primaryKey must be a property), V14 (manifest missing).
- CLI: human output through `GraphicalReportHandler` (nocolor theme when piped or `NO_COLOR`), JSON array output, `--deny-warnings`, 32 MiB worker thread, byte-length-preserving control character sanitization.

## Task Commits

1. **Task 1: Definition format decision** - no code; ADR committed as `4d36795` (docs)
2. **Task 2 (tracer, TDD):**
   - RED `58a214a` (test): workspace skeleton, diagnostic model, stubbed loader / renderer, fixtures, failing tests
   - GREEN `9aebf77` (feat): loader, IR, dispatch, SourceMap, renderer, snapshots

**Plan metadata:** added by the final docs commit (SUMMARY, STATE, ROADMAP).

RED evidence (before GREEN): `cargo test -p ontoforge --test cli` gave 4 failed / 1 passed (JSON parse of empty stdout, exit code 0 instead of 1); `-p ontoforge-schema --test diagnostics` gave 13 failed / 1 passed; `invalid_examples` failed with `v04-unknown-field: expected ONT-V04, got []`. Two tests passed at RED against the stubs: `validate_aviation_succeeds_silently` (stub returned 0 with empty stdout, so it is a regression guard only) and `yaml_and_json_load_to_the_same_ontology` (None == None; hardened with `is_some()` before the RED commit).

## Verification output (final run, after GREEN)

```
cargo test --workspace --no-fail-fast
  ontoforge lib:           3 passed
  ontoforge tests/cli:     5 passed
  ontoforge tests/invalid_examples: 1 passed
  ontoforge-schema lib:   23 passed
  ontoforge-schema tests/diagnostics: 15 passed
cargo clippy --workspace --all-targets -- -D warnings   -> exit 0
cargo fmt --all --check                                  -> clean
cargo run -q -p ontoforge -- validate examples/ontologies/aviation --color never
  -> stdout empty, exit 0 (stderr: ok: ontology `aviation` with 1 object type(s))
cargo run -q -p ontoforge -- validate examples/ontologies/invalid/v04-unknown-field --format json  -> exit 1
  jq -e '.[0].code == "ONT-V04" and .[0].file == "objects.yaml" and .[0].line == 3 and .[0].column == 1'  -> true
  jq -e '.[0].help | test("primaryKey")'                                                                  -> true
```

Human rendering of the v04 fixture (also the committed snapshot):

```
ONT-V04

  × unknown field `primaryKy`
   ╭─[objects.yaml:3:1]
 2 │ apiName: Airport
 3 │ primaryKy: code
   · ────┬────
   ·     ╰── here
 4 │ properties:
   ╰────
  help: did you mean `primaryKey`?
```

Snapshots contain no `C:\` or `/Users/` paths; `rg` for ADR-0002 forbidden identifiers over `crates`, `examples`, `Cargo.toml` found none.

## Interface changes (against the plan's `<interfaces>` block)

All listed signatures exist as specified. Additions and notes for downstream plans:

- `diag`: added `CustomDeError` and `parse_custom_de_error(&str) -> Option<CustomDeError>` (the decoder for `custom_de_error`). `Severity` also derives `PartialOrd, Ord`.
- `loader`: `pub const MANIFEST: &str = "ontology.yaml"`; `LoadOutput` derives `Debug`; `loader::sourcemap::FileId` is public (opaque). `SourceMap` helpers used inside the crate (`locate_in`, `register_def`, `set_manifest`, `raw_location`, `whole_file`) are `pub(crate)`. The `dispatch`, `parse`, `discover` modules are private; dispatch exposes `Ctx`, `dispatch`, `parse_manifest` to `loader/mod.rs` only.
- `dispatch::map_error` is the single place that maps serde errors to codes; plans 01-04..06 extend it rather than adding parallel mappers. `walk(root, path)` in `sourcemap.rs` returns the node and last key location for any lookup.
- `Definition` has only the `ObjectType` variant; the `match` in `loader::merge` must grow with `LinkType` and `ActionType`.
- `render`: `Format` and `ColorChoice` derive `clap::ValueEnum`; `sanitize` is public.
- `ontoforge::run(Cli) -> u8` also prints the stderr summary line; `pipeline::check` is the only orchestration entry point.
- Root `Cargo.toml` gained `result_large_err = "allow"` under `[workspace.lints.clippy]`.

## Decisions Made

- LinkType end `apiName` semantics and the rest of the document shape: ADR-0012.
- A `kind` that is declared in the format but not yet implemented (`LinkType`, `ActionType`) reports ONT-V03 "not implemented yet" so a file written ahead of plans 01-05 / 01-06 gets a clear message instead of a misleading "unknown kind".
- Unknown enum values outside `kind` (for example `type: strng`) are ONT-V06 with a did-you-mean help; inside a node that has `kind` they are ONT-V03.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Workspace clippy allow for `result_large_err`**
- **Found during:** Task 2 (clippy gate)
- **Issue:** `Diagnostic` is 128+ bytes and is the `Err` type of the loader functions; `-D warnings` failed with `clippy::result_large_err` (7 sites).
- **Fix:** added `result_large_err = "allow"` with a comment to `[workspace.lints.clippy]` instead of boxing the diagnostic everywhere.
- **Files modified:** `Cargo.toml`
- **Committed in:** 9aebf77

**2. [Rule 2 - Missing critical] Extra integration test file for loader behavior**
- **Found during:** Task 2 (RED design)
- **Issue:** the plan's behavior list needs loader tests (V03, V04, V05, V07, V08, V10, V14, JSON precheck, depth) but `files_modified` lists none for `ontoforge-schema/tests`.
- **Fix:** added `crates/ontoforge-schema/tests/diagnostics.rs` through the public `load_from_memory` API (15 tests), plus inline unit tests in `diag`, `names`, `parse`, `sourcemap`, `discover`, `render`. Also covers the plan's `deeply_nested_yaml_is_rejected_not_crashing` requirement for the library path.
- **Committed in:** 58a214a (RED), 9aebf77

**3. [Plan note] `Cargo.lock` does not yet contain `cel`**
- `cel` is declared in `[workspace.dependencies]` but no crate uses it yet, so it enters `Cargo.lock` when the first crate (`ontoforge-expr`) depends on it. All other declared versions (including `jsonschema`, `insta`, `tempfile`, `toml`, `globset`, `walkdir`, `regex`, `chrono`) are locked now.

---

**Total deviations:** 2 auto-fixed (1 Rule 3, 1 Rule 2) plus 1 informational note
**Impact on plan:** no scope creep; no architectural change.

## Issues Encountered

- The Task 1 blocker (HEAD on a protected branch) was resolved by the orchestrator before this run; the ledger base was moved to `1e94cb8` before the first commit here.
- An insta snapshot is missing on first run by design; generated with `INSTA_UPDATE=always`, then reviewed (content above) before commit.

## Known Stubs

None. Intentionally partial, scheduled in later plans: ObjectType fields beyond `apiName`, `displayName`, `description`, `primaryKey`, `properties` (01-04); LinkType and ActionType documents (01-05, 01-06); `include` globs, recursion and file size limits in `discover` (01-03); `RUST_MIN_STACK` test config and the 1 MiB-stack regression test (01-03).

## Threat Model Status

| Threat | Disposition | Status |
|---|---|---|
| T-01-01 deep nesting DoS | mitigate | `max_depth = 32` budget (test `nesting_beyond_the_budget_is_an_error`, `deeply_nested_yaml_is_rejected_not_crashing`); CLI runs in a 32 MiB thread. The 1 MiB-stack regression test is 01-03's. |
| T-01-02 YAML tags / merge keys | mitigate | all four strict options set in `parse::options()`; tests cover duplicate keys and `yes` staying a string. Merge-key and tag rejection are configured but not separately tested. |
| T-01-03 terminal escape injection | mitigate | `render::sanitize` applied to message, help and source text; unit-tested for byte-length preservation. No end-to-end CLI test with an escape in a fixture. |
| T-01-04 path disclosure | mitigate | `file` is the relative `/` path; no canonicalize call; snapshots checked for `C:\` and `/Users/`. |
| T-01-SC supply chain | mitigate | only crates from the RESEARCH audit; `Cargo.lock` committed. |

## Threat Flags

None. The CLI reads a directory given on the command line, which is the trust boundary already in the plan's register.

## Next Phase Readiness

Plans 01-02 and 01-03 can build on `load_dir`, `SourceMap::locate` and the snapshot harness. Adding an invalid example needs only a directory `examples/ontologies/invalid/<code>-<slug>/` and an accepted snapshot; no test code changes.

## Self-Check: PASSED

All created files listed above exist; commits `4d36795`, `58a214a`, `9aebf77` exist on `gsd/phase-1-workspace-schema-lint`; `commits: 3` measured with `git rev-list --count 1e94cb8..HEAD` before this SUMMARY commit.
