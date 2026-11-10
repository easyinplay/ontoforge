---
phase: 01-workspace-schema-lint
plan: 07
subsystem: expression-cel
tags: [rust, cel, ontoforge-expr, stack-safety, ast-walk, submission-criteria]

requires:
  - phase: 01-workspace-schema-lint
    provides: "01-02 ontoforge-expr crate skeleton with cel and ontoforge-schema dependencies already declared"
provides:
  - "ontoforge_expr::limits::precheck with MAX_EXPR_CHARS = 4096 and MAX_EXPR_NESTING = 32, lexing strings, comments and numbers the way cel 0.15 does"
  - "ontoforge_expr::analyze(src, f): precheck, then compile (never evaluate) and run f on one scoped 32 MiB thread; ExprError { TooLong, TooDeep, Syntax, Internal }"
  - "ontoforge_expr::walk::{member_paths, MemberPath, offset_to_line_col}: scope-aware member path collection with 1-based line and char column"
affects: [01-12]

plan_head_before: 0444752bdc31cddf264edf5ebc1095b3efa1689a
actuals:
  tokens: 6100
  tasks: 2
  commits: 4

tech-stack:
  added: []
  patterns:
    - "Every recursion over untrusted expression text (cel parser, AST walk, Program drop) runs on one library-owned scoped 32 MiB thread; callers never compile on their own stack"
    - "Precheck is a hand-written lexer mirror of CEL.g4 that rejects anything the real lexer would reject, so a tokenization mismatch can only over-reject, never hide nesting"
    - "Walker keeps its own scope stack instead of cel's references()"

key-files:
  created:
    - crates/ontoforge-expr/src/limits.rs
    - crates/ontoforge-expr/src/analyze.rs
    - crates/ontoforge-expr/src/walk.rs
    - crates/ontoforge-expr/tests/analyze.rs
  modified:
    - crates/ontoforge-expr/src/lib.rs

key-decisions:
  - "cel SourceInfo::offset_for returns 0-based UTF-8 byte offsets (verified: `'é' == params.x` gave column 9 until converted); offset_to_line_col therefore takes a byte offset and returns a char column"
  - "Precheck also rejects invalid escape sequences, not only unterminated strings: after an invalid escape the ANTLR lexer recovers by restarting outside the string, which would let our scanner hide brackets the compiler sees"
  - "`//` comments and numbers are lexed in the precheck as well: a quote inside a comment, or `1r'\\'`, otherwise flips string state relative to the real lexer"
  - "A comprehension's accu_init is walked in the outer scope; iter_var, iter_var2 and accu_var are in scope only for loop_cond, loop_step and result"
  - "A `has()` test select nested inside a select chain (`has(a.b).c`) ends the chain: it is walked as its own path"

requirements-completed: [SCHEMA-05]

coverage:
  - id: D1
    description: "Expressions over 4096 chars or 32 bracket levels are rejected before the compiler runs; brackets inside single, double, triple-quoted and raw strings, and inside comments, are lexed as cel lexes them; unterminated strings and invalid escapes are Syntax errors without compiling"
    requirement: SCHEMA-05
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-expr --test analyze (too_long_expression_is_rejected_without_compiling, max_length_expression_is_accepted, brackets_inside_strings_do_not_count, triple_quoted_and_raw_strings_follow_cel_lexing, raw_prefix_requires_a_token_boundary, comments_do_not_open_strings, invalid_escape_is_rejected_without_compiling, unterminated_string_is_rejected_without_compiling)"
        status: pass
    human_judgment: false
  - id: D2
    description: "No expression shape crashes the process: 33 nested levels are rejected and 31 compile on a 1 MiB caller stack, unary, ternary and addition chains stay safe, a panicking caller closure becomes ExprError::Internal"
    requirement: SCHEMA-05
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-expr --test analyze (deeply_nested_expression_is_rejected_not_crashing, flat_recursive_forms_do_not_crash_on_a_small_stack, closure_panic_becomes_internal)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Syntax errors carry a 1-based line:col relative to the expression (`params.x +` is 1:11)"
    requirement: SCHEMA-05
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-expr --test analyze (syntax_error_has_expression_position)"
        status: pass
    human_judgment: false
  - id: D4
    description: "member_paths returns free-variable member paths with comprehension variables scoped out, has() paths, full select chains, index and call breaks, and line and char-column positions"
    requirement: SCHEMA-05
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-expr --test analyze (walk::comprehension_vars_are_scoped, walk::nested_comprehension_scopes, walk::has_macro_yields_member_path, walk::select_chain_is_full_path, walk::bare_identifier_is_a_path, walk::index_breaks_path, walk::positions_are_line_and_char_column, walk::map_list_and_method_targets_are_walked, walk::has_on_comprehension_variable_yields_nothing, walk::offset_to_line_col_takes_byte_offsets_and_counts_char_columns)"
        status: pass
    human_judgment: false

duration: 15min
completed: 2026-10-15
status: complete
---

# Phase 1 Plan 07: CEL base (precheck, isolated compile, scope-aware walk) Summary

**`ontoforge-expr` now compiles submission-criteria conditions without evaluating them, behind a lexer-faithful length and nesting precheck and a library-owned 32 MiB thread, and lists member paths with comprehension variables correctly scoped out.**

## Performance

- **Duration:** about 15 min
- **Completed:** 2026-10-15T23:02+08:00
- **Tasks:** 2 (both TDD: 2 test commits, 2 feat commits)
- **Files modified:** 5 (4 created, 1 modified), all in `crates/ontoforge-expr`

## Accomplishments

- `precheck` rejects over-long input (`TooLong`) and over-deep bracket nesting (`TooDeep`, limit 32) before any compiler call. Its scanner follows `CEL.g4` for single, double, triple-quoted and raw (`r`, `br`) strings, `//` comments and numeric tokens.
- `analyze` compiles and runs the caller closure on one `std::thread::scope` 32 MiB thread. The 1 MiB-stack regression test passes: 31 levels compile, 33 return `TooDeep`, thread joins cleanly.
- `member_paths` walks the cel AST with its own scope stack. `params.items.all(i, i.qty > 0)` yields only `params.items`; `has(params.note)` yields `params.note`; positions are 1-based line and char column.

## Task Commits

1. **Task 1: precheck and isolated compile** - `1519258` (test, RED), `5ff3765` (feat, GREEN)
2. **Task 2: scope-aware member path walk** - `fee555d` (test, RED), `2c35083` (feat, GREEN)

**Plan metadata:** committed with this SUMMARY (docs).

## TDD Gate Compliance

Both tasks have a `test(01-07)` commit before the `feat(01-07)` commit. RED was run against compiling stubs and failed on assertions: Task 1 11/11 tests failed, Task 2 8/8 `walk::*` tests failed (the 12 earlier tests stayed green). No refactor commits were needed.

## Files Created/Modified

- `crates/ontoforge-expr/src/limits.rs` - `MAX_EXPR_CHARS`, `MAX_EXPR_NESTING`, `precheck` and the lexer-mirroring `Scanner`
- `crates/ontoforge-expr/src/analyze.rs` - `ExprError` with English `Display`, `analyze` with the scoped 32 MiB thread
- `crates/ontoforge-expr/src/walk.rs` - `member_paths`, `MemberPath`, `offset_to_line_col`
- `crates/ontoforge-expr/src/lib.rs` - module declarations and re-exports (`analyze`, `ExprError`, `MAX_EXPR_CHARS`, `MAX_EXPR_NESTING`, `member_paths`, `MemberPath`)
- `crates/ontoforge-expr/tests/analyze.rs` - 22 tests in the `analyze` target

## Interface changes

Signatures match the plan's `<interfaces>` block. Two clarifications for 01-12:

- `offset_to_line_col(src, offset)` takes a **byte** offset (what `SourceInfo::offset_for` returns) and returns a char column. The plan left the unit open.
- `walk` is also reachable as `ontoforge_expr::walk::offset_to_line_col`; only `member_paths` and `MemberPath` are re-exported at the crate root.
- `ExprError` additionally implements `std::error::Error`.

## Decisions Made

See `key-decisions` in the frontmatter. The first is the one 01-12 depends on: cel offsets are bytes.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical functionality] Precheck lexes comments, numbers and escapes, not just strings**
- **Found during:** Task 1 (reading `CEL.g4`)
- **Issue:** The plan described skipping string literals only. Without `//` comment handling, `// '''` followed by 33 nested parens and a second `// '''` is read as one string by the scanner while the real lexer sees real brackets (T-01-22 bypass). Likewise `1r'\'` is a raw string for the real lexer (number, then `r'...'`) and an escaped quote for an identifier-glued scanner. After an invalid escape the ANTLR lexer recovers outside the string, so the scanner would again hide brackets the compiler counts.
- **Fix:** Scanner tokenizes whole identifiers (raw only for `r` / `br`, any case, at a token boundary), skips numeric tokens including `0x` hex, skips `//` comments, and returns `Syntax` for unterminated strings and invalid escape sequences (grammar `ESC_SEQ` set).
- **Files modified:** `crates/ontoforge-expr/src/limits.rs`
- **Verification:** `comments_do_not_open_strings`, `raw_prefix_requires_a_token_boundary`, `invalid_escape_is_rejected_without_compiling`
- **Commit:** `5ff3765`

**2. [Rule 1 - Bug] Byte vs char offsets in positions**
- **Found during:** Task 2 GREEN, first run of `positions_are_line_and_char_column` (got column 9, expected 8)
- **Issue:** `offset_to_line_col` first assumed char offsets; cel 0.15 reports UTF-8 byte offsets.
- **Fix:** convert via `char_indices`; unit test renamed and adjusted.
- **Files modified:** `crates/ontoforge-expr/src/walk.rs`, `crates/ontoforge-expr/tests/analyze.rs`
- **Commit:** `2c35083`

**Total deviations:** 2 auto-fixed (1 Rule 2, 1 Rule 1). **Impact:** none on scope; both tighten what the plan asked for. Extra tests beyond the plan's 15: 7 (`raw_prefix_requires_a_token_boundary`, `comments_do_not_open_strings`, `invalid_escape_is_rejected_without_compiling`, `flat_recursive_forms_do_not_crash_on_a_small_stack`, `walk::map_list_and_method_targets_are_walked`, `walk::has_on_comprehension_variable_yields_nothing`, `walk::offset_to_line_col_takes_byte_offsets_and_counts_char_columns`).

## Issues Encountered

An inline `python -` heredoc in one shell command hung waiting on stdin; I killed that process (PID 49672, its own child) and the rest of the command completed. No repository impact.

## Verification Results

- `cargo test -p ontoforge-expr --test analyze`: `running 22 tests` ... `test result: ok. 22 passed; 0 failed`
- `cargo test -p ontoforge-expr --test analyze -- deeply_nested_expression_is_rejected_not_crashing`: `1 passed`
- `cargo test -p ontoforge-expr --test analyze -- triple_quoted_and_raw_strings_follow_cel_lexing`: `1 passed`
- `cargo clippy --workspace --all-targets -- -D warnings`: clean; `cargo fmt --all --check`: clean; `cargo test --workspace`: all targets ok
- `rg -n "4096" limits.rs`, `rg -n "spawn_scoped" analyze.rs`, `rg -n "Comprehension" walk.rs`: match; `rg -n 'references\(\)' crates/ontoforge-expr/src`: no match
- No `has overflowed its stack` in any output

## Known Stubs

None.

## Threat Flags

None. T-01-22, T-01-23 and T-01-25 mitigations are in place as planned; no new network, auth or file surface.

## Next Phase Readiness

01-12 can call `analyze(src, |p| member_paths(p, src))`, map `ExprError::Syntax { line, column, .. }` into ONT-V13 messages, and dedupe `MemberPath` by `segments`. `member_paths` must only be called inside the `analyze` closure (it recurses per AST level).

## Self-Check: PASSED
