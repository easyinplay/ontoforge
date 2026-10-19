---
phase: 01-workspace-schema-lint
plan: 09
subsystem: schema-export
tags: [rust, schemars, jsonschema, json-schema, cli, editor-integration]
status: complete

requires:
  - phase: 01-workspace-schema-lint
    provides: "01-02..01-06 IR types with schemars derives (Definition/Rule tag = kind, apiName newtype patterns); 01-08 ontoforge CLI skeleton"
provides:
  - "ontoforge_schema::schema_export::{definition_schema, manifest_schema, rendered_schemas}: draft 2020-12 schemas derived from the IR"
  - "ontoforge schema export [--out-dir <DIR>] (default schemas): writes both schema files, creates the directory, exit 1 on IO failure"
  - "schemas/ontoforge-definition.schema.json and schemas/ontoforge-manifest.schema.json committed snapshots"
  - "yaml-language-server $schema modeline on the four aviation YAML files"
affects: [phase-2-and-later (any IR change must re-run schema export)]

plan_head_before: bb334f604abe8ffb027e9d6fe2d0d3bf56373c64
actuals:
  tokens: 4500
  tasks: 2
  commits: 2

tech-stack:
  added: []
  patterns:
    - "Export-only wrapper enum DefinitionFile (untagged One(Box<Definition>) | Many(Vec<Definition>)) shapes the root schema without touching the loader"
    - "Committed generated artifact guarded by a byte-equality test with ONTOFORGE_UPDATE_SCHEMAS=1 as the regenerate switch (same convention as INSTA_UPDATE)"

key-files:
  created:
    - crates/ontoforge-schema/src/schema_export.rs
    - crates/ontoforge-schema/tests/schema_export.rs
    - crates/ontoforge/tests/cli_schema.rs
    - schemas/ontoforge-definition.schema.json
    - schemas/ontoforge-manifest.schema.json
  modified:
    - crates/ontoforge-schema/src/lib.rs
    - crates/ontoforge/src/cli.rs
    - crates/ontoforge/src/lib.rs
    - examples/ontologies/aviation/ontology.yaml
    - examples/ontologies/aviation/objects.yaml
    - examples/ontologies/aviation/links.yaml
    - examples/ontologies/aviation/actions.yaml

key-decisions:
  - "No `$schema` key is supported inside definition documents (plan decision): JSON definition files may be arrays, YAML uses the comment modeline"
  - "The IR doc comments of Definition and Rule describe loader internals; schema_export overrides those two `$defs` descriptions with user-facing text instead of editing ir/ (outside this plan's files)"
  - "DefinitionFile::One boxes Definition to satisfy clippy::large_enum_variant; the exported schema is identical"

requirements-completed: [SCHEMA-07]

coverage:
  - id: D1
    description: "Definition and manifest JSON Schemas (draft 2020-12) generated from the IR; Definition and Rule are oneOf branches pinned by `kind` const; branches reject unknown fields"
    requirement: SCHEMA-07
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-schema --test schema_export (definition_schema_is_draft_2020_12, definition_variants_are_discriminated_by_kind, rule_variants_are_discriminated_by_kind, rendering_is_deterministic)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Committed schemas/*.json are byte-identical to the IR-generated output"
    requirement: SCHEMA-07
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-schema --test schema_export (committed_schemas_are_up_to_date)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Every aviation YAML document, every aviation-json file and both manifests validate; v04, v06, v07 invalid samples fail the definition schema"
    requirement: SCHEMA-07
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-schema --test schema_export (aviation_documents_validate, invalid_examples_fail_schema)"
        status: pass
    human_judgment: false
  - id: D4
    description: "ontoforge schema export writes both files into --out-dir (created when missing), identical to schemas/, exit 1 on IO failure; aviation still validates with modelines"
    requirement: SCHEMA-07
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge --test cli_schema (schema_export_writes_committed_files, schema_export_creates_missing_dir, schema_export_reports_io_failure, aviation_still_validates_with_modelines)"
        status: pass
    human_judgment: false
  - id: D5
    description: "Editor completion narrowing for oneOf + const in a real YAML language server"
    requirement: SCHEMA-07
    verification: []
    human_judgment: true
    rationale: "RESEARCH A9: narrowing depends on the editor extension version; needs VS Code with the Red Hat YAML extension, not available in this headless run"
---

# Phase 1 Plan 09: JSON Schema export Summary

**JSON Schema (draft 2020-12) derived from the IR via schemars, committed under `schemas/` with a byte-equality drift test, exported by `ontoforge schema export`, and wired to the aviation YAML files through a yaml-language-server modeline.**

## Accomplishments

- `schema_export.rs` generates the definition schema (single Definition or array of them, `oneOf` on `kind` const for ObjectType/LinkType/ActionType and the five rule kinds) and the manifest schema; output is deterministic pretty JSON ending in a newline.
- `committed_schemas_are_up_to_date` fails with a message pointing at `cargo run -p ontoforge -- schema export` when the IR drifts; `ONTOFORGE_UPDATE_SCHEMAS=1` rewrites the files instead.
- All aviation and aviation-json documents and both manifests pass their schema; the v04, v06, v07 invalid samples are rejected. No IR change was needed (the existing `deny_unknown_fields` and apiName `pattern` already cover them).
- `ontoforge schema export [--out-dir DIR]` (default `schemas`) creates the directory, prints each written path to stderr, exits 1 on IO failure; re-export produced no diff against the files committed in Task 1.
- Aviation YAML files carry `# yaml-language-server: $schema=../../../schemas/...` on line 1; comment-only, `aviation_lints_clean`-style tests and the full workspace suite are unaffected.

## Task Commits

1. Task 1: IR-derived schemas, committed snapshots, drift and example tests: `ad94e7d`
2. Task 2: `schema export` subcommand, CLI tests, aviation modelines: `9c7421e`

## Verification (real output)

- `cargo test -p ontoforge-schema --test schema_export`: 7 passed, 0 failed
- `cargo test -p ontoforge --test cli_schema`: 4 passed, 0 failed
- `cargo test -p ontoforge-schema --test schema_export --test load_examples`: 3 + 7 passed
- `cargo test --workspace`: all targets `ok`, 0 failed
- `cargo clippy -p ontoforge-schema -p ontoforge --all-targets -- -D warnings`: clean
- `cargo fmt --all --check`: clean
- Acceptance: the three `jq -e` checks on the schemas succeed; `rg "pub mod schema_export"` and `rg "Schema\(SchemaArgs"` match; all four aviation files contain `yaml-language-server`; `cargo run -q -p ontoforge -- schema export --out-dir target/schema-check` exits 0 and `cmp` of both files against `schemas/` exits 0.

## Human check (editor completion)

Not performed: this run is headless and has no VS Code with the Red Hat YAML extension. The check (completion in `objects.yaml`, validation hint on a misspelled `kind`) remains open for the phase verifier / UAT. Static evidence only: the exported schema contains the `oneOf` with `properties.kind.const` per branch, which is the shape RESEARCH A9 flags as extension-dependent; if narrowing proves weak, the follow-up is post-processing to `if/then`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical] Loader-internal text in exported descriptions**
- **Found during:** Task 1 (inspecting generated schema)
- **Issue:** the doc comments of `Definition` and `Rule` explain the loader's `{kind: X}` rewrite and appeared as editor hover text.
- **Fix:** `describe_def` in `schema_export.rs` replaces those two `$defs` descriptions with user-facing sentences; `ir/` untouched (outside this plan's files).
- **Files modified:** crates/ontoforge-schema/src/schema_export.rs, schemas/ontoforge-definition.schema.json
- **Commit:** ad94e7d

**2. [Rule 3 - Blocking] clippy::large_enum_variant on DefinitionFile**
- **Found during:** Task 1 verification
- **Fix:** `One(Box<Definition>)`; schema output unchanged (committed files still byte-equal).
- **Commit:** ad94e7d

**3. [Extra test] `schema_export_reports_io_failure`**
- Added beyond the plan's behavior list to cover the exit-code-1 requirement in Task 2 step 1.

**Total deviations:** 2 auto-fixed, 1 added test. **Impact:** none on scope or interfaces.

## Known Stubs

None.

## Threat Flags

None. `--out-dir` write surface is the one already registered as T-01-31; `jsonschema` is used with `default-features = false` and the schemas contain only local `$ref`s (T-01-32).

## Issues Encountered

None blocking. The `docs/` directory remains untracked by instruction.

## Next

Ready for 01-10 and 01-11 (same wave; no shared files beyond the comment-only aviation modeline).

## Self-Check: PASSED

- Created files exist: schema_export.rs, tests/schema_export.rs, tests/cli_schema.rs, both schemas/*.json.
- Commits `ad94e7d` and `9c7421e` exist on `gsd/phase-01-workspace-schema-lint`; `git rev-list --count bb334f6..HEAD` = 2 at SUMMARY time.
