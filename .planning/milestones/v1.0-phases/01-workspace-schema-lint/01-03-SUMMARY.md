---
phase: 01-workspace-schema-lint
plan: 03
subsystem: schema-loader
tags: [rust, globset, walkdir, serde-saphyr, serde_json, loader, path-safety]

requires:
  - phase: 01-workspace-schema-lint
    provides: "01-01 loader, SourceMap, Diagnostic model, snapshot harness"
provides:
  - "include-glob driven recursive discovery shared by load_dir and load_from_memory (loader::discover::select)"
  - "V14 for escaping/invalid include patterns, empty match, files over MAX_FILE_BYTES (4 MiB)"
  - "load_dir / load_from_memory run on a 32 MiB scoped thread; a panic becomes a V01 diagnostic"
  - "JSON precheck with character-based line:col and cleaned messages; V01 message cleanup for duplicate key and depth budget"
  - "crates/ontoforge-schema/tests/loader.rs (28 boundary tests) and 5 new invalid fixtures with snapshots"
affects: [01-04, 01-05, 01-06, 01-10, 01-11]

plan_head_before: 965b667f513b5df9f61ca7c64172099303f1487a
actuals:
  tokens: 8100
  tasks: 2
  commits: 4

tech-stack:
  added: []
  patterns:
    - "One selection function (discover::select) for disk and memory sources; walk only lists candidates"
    - "Library-internal stack isolation: callers never depend on their own stack size"

key-files:
  created:
    - crates/ontoforge-schema/tests/loader.rs
    - examples/ontologies/invalid/v01-yaml-duplicate-key/{ontology,objects}.yaml
    - examples/ontologies/invalid/v01-json-trailing-comma/{ontology.yaml,objects.json}
    - examples/ontologies/invalid/v02-missing-kind/{ontology,objects}.yaml
    - examples/ontologies/invalid/v03-reserved-kind/{ontology,objects}.yaml
    - examples/ontologies/invalid/v14-include-escapes-root/ontology.yaml
    - crates/ontoforge/tests/snapshots/invalid_examples__{v01-yaml-duplicate-key,v01-json-trailing-comma,v02-missing-kind,v03-reserved-kind,v14-include-escapes-root}.snap
  modified:
    - crates/ontoforge-schema/src/loader/discover.rs
    - crates/ontoforge-schema/src/loader/mod.rs
    - crates/ontoforge-schema/src/loader/parse.rs

key-decisions:
  - "include is validated before matching: any bad pattern yields only its V14 diagnostics and loads no files, so one typo does not produce a cascade of unrelated errors"
  - "The 'no definition files matched' V14 is suppressed when the manifest failed to load or the directory listing failed (those errors already explain it)"
  - "serde_json's 'key must be a string' is rewritten to 'expected a quoted key; JSON does not allow a trailing comma' because it is the message for both causes"

requirements-completed: [SCHEMA-01, SCHEMA-06]

coverage:
  - id: D1
    description: "Definition files in nested directories merge into one IR; include globs choose the files; manifest, ontoforge.toml, dot directories, target and node_modules are never definitions"
    requirement: SCHEMA-01
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-schema --test loader (nested_files_are_merged, dot_dirs_and_build_dirs_are_skipped, manifest_and_lint_config_are_not_definitions, custom_include_restricts_files, load_from_memory_applies_same_include_filter)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Path safety: .. or absolute include, invalid glob, symlinks not followed, files over 4 MiB, non-UTF-8 files"
    requirement: SCHEMA-01
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-schema --test loader (include_with_parent_segment_is_rejected, absolute_include_is_rejected, invalid_glob_is_rejected, symlinked_file_is_not_followed, oversized_file_is_rejected, oversized_in_memory_file_is_rejected, non_utf8_file_is_v01)"
        status: pass
      - kind: integration
        ref: "cargo test -p ontoforge --test invalid_examples (snapshot invalid_examples__v14-include-escapes-root)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Syntax errors carry exact line:col (BOM, CRLF, tab, CJK) and the span slices the offending text; JSON trailing comma and comments are V01"
    requirement: SCHEMA-06
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-schema --test loader (bom_crlf_tab_and_cjk_locations_are_exact, json_syntax_error_column_counts_characters, json_trailing_comma_is_rejected, json_comment_is_rejected, duplicate_key_is_v01_with_location, merge_key_is_v01, unsupported_tag_is_v01, v01_message_has_no_parser_position_suffix)"
        status: pass
    human_judgment: false
  - id: D4
    description: "200-level nested YAML or JSON on a 1 MiB stack returns V01 and does not crash"
    requirement: SCHEMA-06
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-schema --test loader -- deeply_nested (2 passed)"
        status: pass
    human_judgment: false
  - id: D5
    description: "IR determinism: property order kept, YAML multi-document and JSON array both load, file order does not change the Ontology, diagnostic paths use /"
    requirement: SCHEMA-01
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-schema --test loader (properties_keep_declared_order, multi_document_yaml_and_json_array_are_both_loaded, empty_yaml_documents_are_skipped, file_order_does_not_change_ir, diagnostic_paths_use_forward_slash, yaml_yes_no_stay_strings)"
        status: pass
    human_judgment: false

duration: not timed
completed: 2026-10-15
status: complete
---

# Phase 1 Plan 03: Loader discovery, path safety and parser hardening Summary

**The manifest `include` globs now drive recursive, symlink-free, size-limited discovery shared by disk and memory sources, and every load runs on a 32 MiB scoped thread, so 200-level nested input is a V01 diagnostic on a 1 MiB caller stack instead of an abort.**

## Accomplishments

- `discover.rs` rewritten: `walk` (walkdir, `follow_links(false)`, prunes `.`-prefixed dirs, `target`, `node_modules`) lists candidates; `select` applies the manifest `include` (default `**/*.yaml`, `**/*.yml`, `**/*.json`, `literal_separator(true)`), always excludes the root `ontology.yaml` and any `ontoforge.toml`, and sorts the result. `load_from_memory` normalizes `\` to `/` and goes through the same `select`.
- V14 cases: pattern starting with `/` or `\`, drive letter, or a `..` segment (located at the pattern in the manifest, for example `ontology.yaml:3:11` in the fixture); invalid glob (globset error kind); nothing matched; file over `MAX_FILE_BYTES` (4 MiB, checked from metadata before reading, and from byte length for memory input). Non-UTF-8 stays V01.
- `parse.rs`: JSON precheck now reports character-based columns (serde_json counts bytes) and clamps the byte offset to a char boundary; ` at line L column C` is stripped; duplicate-key message becomes ``duplicate mapping key `apiName` ``; depth-budget breach becomes `document nests deeper than 32 levels`.
- `loader/mod.rs`: `isolated()` runs the whole load in `thread::scope` + `Builder::stack_size(32 << 20).spawn_scoped`; a panic or spawn failure returns a V01 "internal error while loading" diagnostic.
- Fixtures and snapshots for V01 (duplicate key, JSON trailing comma), V02, V03, V14. `RUST_MIN_STACK` is not used.

## Task Commits

1. **Task 1 (TDD):** RED `3c8acf8` (test), GREEN `5b03b4e` (feat)
2. **Task 2 (TDD):** RED `37ed731` (test), GREEN `c388da8` (feat)

RED evidence: Task 1, `cargo test -p ontoforge-schema --test loader` gave 3 passed / 10 failed (the 3 passes are regression guards for behavior that already held: non-UTF-8, dot dirs at root, symlink). Task 2, the 3 position/message tests failed (`duplicate mapping key: apiName, set DuplicateKeyPolicy in Options if acceptable`, JSON column 33 bytes vs 25 chars) and both `deeply_nested_*` tests aborted the test binary with `thread '<unknown>' has overflowed its stack` on the 1 MiB thread.

## Verification output

```
cargo test -p ontoforge-schema --test loader     -> 28 passed
cargo test -p ontoforge-schema --test loader -- deeply_nested   -> 2 passed
cargo test -p ontoforge-schema --test loader -- diagnostic_paths_use_forward_slash   -> 1 passed
cargo test -p ontoforge --test invalid_examples  -> 1 passed (7 snapshots; no .snap.new left)
cargo test --workspace                           -> all ok (ontoforge 3+5+1, ontoforge-schema 25 lib + 15 diagnostics + 28 loader, each skeleton crate 1)
cargo clippy --workspace --all-targets -- -D warnings -> clean
cargo fmt --all --check                          -> clean
rg "canonicalize" crates/ontoforge-schema/src    -> no match
rg "RUST_MIN_STACK" crates Cargo.toml            -> no match
rg "IgnoredAny|max_depth" parse.rs; rg "spawn_scoped" mod.rs; rg "follow_links\(false\)" discover.rs; rg "MAX_FILE_BYTES" mod.rs -> all match
rg "kn_id|BKN|ODL|Domain Pack" crates examples   -> no match
```

New snapshots (key lines): v01 duplicate key `objects.yaml:3:1` ``duplicate mapping key `apiName` ``; v01 JSON `objects.json:7:3` `expected a quoted key; JSON does not allow a trailing comma`; v02 `objects.yaml:1:1` `document has no kind field`; v03 `objects.yaml:1:7` `kind Interface is reserved and not supported in v1`; v14 `ontology.yaml:3:11` `include pattern ../**/*.yaml escapes the ontology root`.

## Interface changes

- `loader::MAX_FILE_BYTES` is a public constant.
- `discover::is_definition_file` and the old `discover(root)` are gone; callers inside the crate use `walk` + `select`. These modules are private, so no external API changed.
- `assemble` now takes a `Listing` and a reader closure; 01-04..06 only touch `merge` / `dispatch`, which are unchanged.

## Decisions Made

See `key-decisions` in the frontmatter.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] JSON trailing-comma message was misleading**
- **Found during:** Task 2 (fixture snapshot review)
- **Issue:** serde_json reports `key must be a string` for an object trailing comma, which does not tell the author what to fix.
- **Fix:** rewrote that exact message in `parse::json_message`.
- **Files modified:** crates/ontoforge-schema/src/loader/parse.rs
- **Commit:** c388da8

**2. [Rule 3 - Blocking] clippy `doc_markdown` and `redundant_closure_for_method_calls`**
- **Found during:** Task 2 (clippy gate)
- **Fix:** backticked `serde_json` in a doc comment; used the method path in the test.
- **Commit:** c388da8

**Total deviations:** 2 auto-fixed (1 Rule 1, 1 Rule 3). No scope change.

## Issues Encountered

- A Python text-mode rewrite of three source files on Windows produced CRLF working copies; git normalizes them to LF through `.gitattributes`, and the working copies were converted back with `sed` before the final run. Index content is LF.

## Known Stubs

None.

## Threat Model Status

| Threat | Disposition | Status |
|---|---|---|
| T-01-07 include escape | mitigate | `escapes_root` rejects `/`, `\`, drive letter, `..`; V14 located at the pattern; tests for three absolute forms and `..` |
| T-01-08 symlinks | mitigate | `follow_links(false)`, only `is_file()` entries listed; `symlinked_file_is_not_followed` (returns early when the OS refuses to create a symlink; it ran and passed here, see note) |
| T-01-09 deep nesting | mitigate | `max_depth = 32`, 32 MiB scoped thread, 1 MiB-stack tests for YAML and JSON (100 and 200 levels) |
| T-01-10 oversized input | mitigate | `MAX_FILE_BYTES` 4 MiB on disk and memory paths; serde-saphyr budget defaults untouched |
| T-01-11 lax JSON | mitigate | strict `serde_json` precheck for `.json` |

Note: the symlink test passes vacuously on a machine without symlink permission. Whether this Windows host created the link was not recorded; the Linux CI job will exercise it.

## Threat Flags

None.

## Next Phase Readiness

01-04 can extend `ObjectType` IR and add root-level example files; discovery picks them up through the default include. The `Definition` match in `loader::merge` still grows with `LinkType` / `ActionType` in 01-05 / 01-06.

## Self-Check: PASSED

Files verified present: tests/loader.rs, the 5 fixture directories, the 5 snapshots, discover.rs, mod.rs, parse.rs. Commits `3c8acf8`, `5b03b4e`, `37ed731`, `c388da8` exist; `git rev-list --count 965b667..HEAD` = 4 before this SUMMARY commit.
