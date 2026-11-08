---
phase: 01-workspace-schema-lint
plan: 13
subsystem: schema-lint-cli
tags: [rust, toml, heck, anstream, reuse-audit, gap-closure]

requires:
  - phase: 01-workspace-schema-lint
    provides: "01-08 lint/config.rs and render ColorChoice; 01-07 walk.rs byte offsets; 01-03 parse.rs char columns; 01-09 names::canonical consumers"
provides:
  - "lint/config.rs parses ontoforge.toml through toml::de::DeTable / DeValue; the private datetime key dependency is gone"
  - "names::canonical built on heck 0.5 (HTTPServer -> httpServer, non-ASCII names get no suggestion)"
  - "--color auto decided by anstream::AutoStream::choice (NO_COLOR, CLICOLOR, CLICOLOR_FORCE, TERM)"
  - "ontoforge_schema::diag::offset_to_line_col: the single byte-offset to line:col implementation"
affects: [01-10, 01-11]

plan_head_before: d021687c5c7f7d8de97615451f0e3e7c619b4f81
actuals:
  tokens: 3000
  tasks: 3
  commits: 4

tech-stack:
  added: [heck 0.5.0, anstream 1.0.0]
  patterns:
    - "Workspace dependencies declared once in the root Cargo.toml; both crates were already in Cargo.lock, so no new download"
    - "Position math lives in diag.rs; expr keeps only a usize shim (walk::char_pos) for its public pair types"

key-files:
  created: []
  modified:
    - Cargo.toml
    - Cargo.lock
    - crates/ontoforge-schema/Cargo.toml
    - crates/ontoforge-schema/src/lint/config.rs
    - crates/ontoforge-schema/src/ir/names.rs
    - crates/ontoforge-schema/src/loader/parse.rs
    - crates/ontoforge-schema/src/diag.rs
    - crates/ontoforge-expr/src/walk.rs
    - crates/ontoforge-expr/src/infer.rs
    - crates/ontoforge-expr/tests/analyze.rs
    - crates/ontoforge/Cargo.toml
    - crates/ontoforge/src/lib.rs

key-decisions:
  - "offset_to_line_col returns (u32, u32) like SourceLocation and floors a mid-char offset to a char boundary (config.rs already did this); walk.rs keeps a pub(crate) char_pos shim so MemberPath.pos and TypeIssue.pos stay (usize, usize) and no public type or test signature changes"
  - "The offset_to_line_col unit test moved from ontoforge-expr/tests/analyze.rs to diag.rs next to the function; its three assertions are unchanged, two clamping cases added"
  - "Datetime and out-of-range integers in an option value fall into the existing 'other type' branch of option_value, covered by a new test"

requirements-completed: [LINT-04, SCHEMA-06, LINT-03]

coverage:
  - id: D1
    description: "lint config parsing uses toml::de::DeTable; Raw, RawVisitor and DATETIME_KEY are removed and behaviour (V15 diagnostics, spans) is unchanged"
    requirement: LINT-04
    verification:
      - kind: command
        ref: "rg -n 'toml_private|DATETIME_KEY|RawVisitor' crates (0 matches); cargo test -p ontoforge-schema lint:: ; cargo test -p ontoforge --test cli_lint"
        status: pass
    human_judgment: false
  - id: D2
    description: "canonical() suggests httpServer for HTTPServer and nothing misleading for non-ASCII names; link_types and the v07 snapshot pass unchanged"
    requirement: SCHEMA-06
    verification:
      - kind: unit
        ref: "cargo test -p ontoforge-schema (names tests, tests/link_types.rs); RED commit 8d972fa failed first"
        status: pass
    human_judgment: false
  - id: D3
    description: "--color auto goes through anstream::AutoStream::choice; --color flag and help text unchanged; JSON output stays ANSI-free"
    requirement: LINT-03
    verification:
      - kind: integration
        ref: "crates/ontoforge/src/lib.rs ColorChoice::Auto branch; cargo test -p ontoforge"
        status: pass
    human_judgment: false
  - id: D4
    description: "One offset_to_line_col definition shared by parse.rs, config.rs and walk.rs; no snapshot content change"
    requirement: SCHEMA-06
    verification:
      - kind: command
        ref: "rg -n 'fn offset_to_line_col|fn line_col' crates (1 definition, diag.rs); git status shows no .snap change, find '*.snap.new' empty; cargo test --workspace"
        status: pass
    human_judgment: false

duration: not measured (continuation after an interrupted run)
completed: 2026-10-16
status: complete
---

# Phase 1 Plan 13: Replace hand-written code with existing crates Summary

**Lint config parsing, apiName suggestions, `--color auto` and line:col conversion now rest on `toml::de::DeTable`, `heck`, `anstream` and a single `diag::offset_to_line_col`, with no diagnostic code, position, JSON shape or snapshot changed.**

## Performance

- **Tasks:** 3 (Task 2 TDD: one test commit, one feat commit)
- **Commits:** 4, measured from `plan_head_before`
- **Net diff:** 129 insertions, 155 deletions across 12 files, including the 2 workspace dependency lines and 2 lockfile lines
- **Run history:** Tasks 1 and 2 were committed by an executor that was killed by an API auth error; this continuation verified Task 2 against its acceptance criteria, then did Task 3 and the closeout

## Accomplishments

- Task 1: `parse_config` uses `DeTable::parse`; the matching in `Parser` is on `DeValue`; integers go through `i64::from_str_radix(i.as_str(), i.radix())`. About 100 lines of custom `Deserialize` machinery removed, and the dependency on toml's private datetime key (T-01-40) is gone.
- Task 2: `canonical` uses heck's `to_upper_camel_case` / `to_lower_camel_case` and keeps the `well_formed && reserved.is_none() && out != raw` filter. `lib.rs::report` decides `ColorChoice::Auto` with `anstream::AutoStream::choice(&std::io::stdout()) != anstream::ColorChoice::Never`, so an empty `NO_COLOR` no longer disables color and `CLICOLOR`, `CLICOLOR_FORCE` and `TERM=dumb` are honoured. The JSON path never consults color (T-01-41).
- Task 3: `diag::offset_to_line_col(text, byte_offset) -> (u32, u32)` replaces the three copies (`precheck_json` in `parse.rs`, `Parser::at` in `config.rs`, `walk::offset_to_line_col`). `ontoforge-expr` already depended on `ontoforge-schema`; no dependency added.

## Task Commits

1. **Task 1: DeTable config parsing** - `b1b9b11` (refactor)
2. **Task 2: heck and anstream** - `8d972fa` (test, RED), `2ba8a1d` (feat, GREEN)
3. **Task 3: offset_to_line_col dedupe** - `07a0e06` (refactor)

**Plan metadata:** committed with this SUMMARY (docs).

## TDD Gate Compliance

Task 2: `test(01-13)` commit `8d972fa` (acronym and non-ASCII cases failing) precedes `feat(01-13)` commit `2ba8a1d`. No refactor commit needed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Public `walk::offset_to_line_col` could not simply be deleted**
- **Found during:** Task 3
- **Issue:** `MemberPath.pos`, `TypeIssue.pos`, `check.rs` problems and `analyze.rs` use `(usize, usize)`, and `tests/analyze.rs` has a helper typed `Option<(usize, usize)>`. The shared function returns `(u32, u32)` as the plan specified, so deleting the walk copy outright would have rippled through the public API and tests.
- **Fix:** `walk.rs` keeps a 3-line `pub(crate) fn char_pos` that widens the shared result; `infer.rs` and `Walker::pos` call it. The old unit test moved to `diag.rs` (assertions unchanged, two clamping cases added) and its import in `tests/analyze.rs` was removed.
- **Files modified:** `crates/ontoforge-expr/src/walk.rs`, `crates/ontoforge-expr/src/infer.rs`, `crates/ontoforge-expr/tests/analyze.rs`
- **Commit:** `07a0e06`

**Total deviations:** 1 auto-fixed (Rule 3). **Impact:** removes `ontoforge_expr::walk::offset_to_line_col` from the public surface of a crate no other crate imports; no behaviour change.

## Verification Results

- `cargo test --workspace`: exit 0, 35 test binaries `ok`, 242 tests passed, no `FAILED`
- `cargo clippy --workspace --all-targets -- -D warnings`: `Finished`, no warnings
- `cargo fmt --all --check`: exit 0
- `rg -n "fn offset_to_line_col|fn line_col" crates`: one definition (`ontoforge-schema/src/diag.rs`) plus its unit test name
- `rg -n "toml_private|DATETIME_KEY|RawVisitor" crates`: 0 matches
- `git show 2ba8a1d -- Cargo.lock`: only `"anstream"` and `"heck"` added to dependency lists, no new package entries
- No `.snap` file modified, no `.snap.new` file present

## Known Stubs

None.

## Threat Flags

None. T-01-40 mitigated by Task 1 (public `DeValue::Datetime`); T-01-41 unchanged (JSON path does not use color).

## Next Phase Readiness

Plans 01-10 and 01-11 can call `names::canonical` / `did_you_mean` through their unchanged public signatures and may use `diag::offset_to_line_col` for any new position math.

## Self-Check: PASSED

- Modified files present on disk, `diag.rs` defines `offset_to_line_col`.
- Commits in `git log`: `b1b9b11`, `8d972fa`, `2ba8a1d`, `07a0e06`.
