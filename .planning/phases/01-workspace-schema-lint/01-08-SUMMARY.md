---
phase: 01-workspace-schema-lint
plan: 08
subsystem: lint-framework
tags: [rust, lint, toml, config, insta, cli, miette, ont-e08, ont-e09]

requires:
  - phase: 01-workspace-schema-lint
    provides: "01-03 loader, SourceMap::locate and DefKey/Seg/Part; 01-04 ObjectType and basetype::pk_suitability; 01-05 LinkType; 01-06 ActionType and rules; 01-02 Diagnostic model, render and exit_code, did_you_mean"
provides:
  - "ontoforge_schema::lint::{Level, Finding, LintCx, LintRule, KNOWN_RULE_IDS, registry, run} with configured-level resolution and Finding to Diagnostic location"
  - "lint::config::{LintConfig, OptionValue, load_config, parse_config}: ontoforge.toml [lint] levels and [lint.options.\"ONT-xxx\"], every problem is ONT-V15 with a span; ONT-V ids cannot be configured"
  - "lint::words::split_camel"
  - "lint::naming::rules() and lint::structure::rules() registration seams for plans 01-10 and 01-11"
  - "ONT-E08 (action rule combinations) and ONT-E09 (base type use) with insta trigger and clean snapshots"
  - "ontoforge lint <DIR> [--config PATH] [--format human|json] [--color] [--deny-warnings]; ontoforge::lint_cmd::lint"
affects: [01-10, 01-11, 01-12, phase-03-actions]

plan_head_before: 60f1b73ea2dba5fef5b859d5731431fc9dc8da3e
actuals:
  tokens: 19360
  tasks: 3
  commits: 6

tech-stack:
  added: []
  patterns:
    - "Config read into a generic Raw tree whose every key and value is a toml::Spanned, then checked by hand: exact V15 positions without a second parse"
    - "Rules return Findings keyed by (DefKey, path, Part); only run() resolves a location, so a rule never touches files"
    - "Level resolution: configured level replaces every severity of the rule; otherwise the finding's own severity, then the rule's default level"
    - "Registration seams (naming::rules, structure::rules) keep parallel plans out of lint/mod.rs"

key-files:
  created:
    - crates/ontoforge-schema/src/lint/mod.rs
    - crates/ontoforge-schema/src/lint/config.rs
    - crates/ontoforge-schema/src/lint/words.rs
    - crates/ontoforge-schema/src/lint/e08.rs
    - crates/ontoforge-schema/src/lint/e09.rs
    - crates/ontoforge-schema/src/lint/naming/mod.rs
    - crates/ontoforge-schema/src/lint/structure/mod.rs
    - crates/ontoforge-schema/src/lint/snapshots/ontoforge_schema__lint__e08__tests__{triggers,clean}.snap
    - crates/ontoforge-schema/src/lint/snapshots/ontoforge_schema__lint__e09__tests__{triggers,clean}.snap
    - crates/ontoforge-schema/tests/lint_examples.rs
    - crates/ontoforge/src/lint_cmd.rs
    - crates/ontoforge/tests/cli_lint.rs
    - crates/ontoforge/tests/snapshots/cli_lint__e08_human.snap
  modified:
    - crates/ontoforge-schema/src/lib.rs
    - crates/ontoforge/src/cli.rs
    - crates/ontoforge/src/lib.rs

key-decisions:
  - "A configured level replaces the severity of every finding of a rule; ONT-E09 keeps its mixed warning/error output only when unconfigured"
  - "Unknown rule id help tries zero padding first (ONT-W1 suggests ONT-W01), then Jaro-Winkler; Jaro-Winkler alone ranks ONT-W10 above ONT-W01 for ONT-W1"
  - "Option values accepted from ontoforge.toml are integer, number, string and array of strings; the type a rule expects is the rule's concern (option_usize and friends fall back to the default on a mismatch)"
  - "Configuration errors and structural errors both stop the run before any rule executes; lint then reports only those errors"

requirements-completed: [LINT-01, LINT-03, LINT-04]

coverage:
  - id: D1
    description: "Lint framework: registry, level resolution (off, info, warn, error over finding severity over default level), Finding located through SourceMap, options reach the rule"
    requirement: LINT-01
    verification:
      - kind: unit
        ref: "cargo test -p ontoforge-schema --lib lint:: (level_resolution, findings_are_located_in_the_source, rule_options_reach_the_rule, registry_ids_are_known_and_unique)"
        status: pass
    human_judgment: false
  - id: D2
    description: "ontoforge.toml parsing: levels, options, unknown id / level / section / option and ONT-V ids are ONT-V15 with exact positions; root file, --config override, missing file handling"
    requirement: LINT-04
    verification:
      - kind: unit
        ref: "cargo test -p ontoforge-schema --lib lint::config (unknown_lint_rule_id_is_error, validate_codes_cannot_be_configured, invalid_level_is_error, unknown_section_is_error, undeclared_option_is_error, missing_config_file_uses_defaults, missing_override_path_is_error)"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_lint.rs (unknown_rule_id_in_config_fails, config_flag_overrides_root_file, validate_codes_cannot_be_turned_off_by_config)"
        status: pass
    human_judgment: false
  - id: D3
    description: "ONT-E08 and ONT-E09 per v1 IR, one insta trigger snapshot and one clean snapshot each; the E09 configured level overrides both severities"
    requirement: LINT-01
    verification:
      - kind: unit
        ref: "cargo test -p ontoforge-schema --lib lint::e0 (7 tests; snapshots ontoforge_schema__lint__e08__tests__{triggers,clean}, e09 likewise)"
        status: pass
    human_judgment: false
  - id: D4
    description: "ontoforge lint: validation first, rules only on a clean ontology; exit 1 on error, warning only with --deny-warnings, info never; JSON array without ANSI; human output with source location"
    requirement: LINT-03
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge --test cli_lint (14 tests incl. lint_error_rule_exits_1, warning_only_exit_codes, config_info_level_never_fails, lint_json_has_no_ansi, e08_human)"
        status: pass
    human_judgment: false
  - id: D5
    description: "aviation and aviation-json lint clean under the default configuration"
    requirement: LINT-01
    verification:
      - kind: integration
        ref: "cargo test -p ontoforge-schema --test lint_examples (aviation_lints_clean); cargo run -q -p ontoforge -- lint examples/ontologies/aviation --deny-warnings --format json -> [] exit 0"
        status: pass
    human_judgment: false
  - id: D6
    description: "Colored human output is readable in a real terminal (error code, file:line:col, source snippet underline, help line)"
    requirement: LINT-03
    verification:
      - kind: other
        ref: "automated: --color always emits ANSI escapes (od -c shows 033[33m before the code); NO_COLOR path covered by e08_human snapshot"
        status: pass
    human_judgment: true
    rationale: "Terminal color and Unicode border rendering needs a person in Windows Terminal (VALIDATION.md Manual-Only item); the plan marks it as a human-check"

duration: 9 min
completed: 2026-10-15
status: complete
---

# Phase 1 Plan 08: Lint framework, ontoforge.toml, E08/E09 and the lint command Summary

**`ontoforge lint` validates first, then runs a registry of configurable rules (E08 action rule combinations, E09 base type use) with levels from `ontoforge.toml` or `--config`, where every configuration mistake is a positioned ONT-V15 and ONT-V errors cannot be turned off.**

## Performance

- **Duration:** about 9 min (first RED commit 15:22Z, last commit 15:30Z by commit timestamps; the start time was not captured)
- **Tasks:** 3 (all TDD, each with a RED `test(01-08)` commit before its GREEN `feat(01-08)` commit)
- **Files:** 18 in the diff against the base commit (before this SUMMARY)

## Accomplishments

- `lint/mod.rs`: `Level`, `Finding`, `LintCx` (with `option_usize`, `option_f64`, `option_str_list`), `LintRule`, `KNOWN_RULE_IDS` (13), `registry()` (E08, E09, then `naming::rules()` and `structure::rules()`), `run()`. A rule only emits `(DefKey, path, Part)`; `run` turns it into a `Diagnostic` through `SourceMap::locate`, applies the level, and sorts. Test helper `lint_rule_on` loads in-memory files, asserts they validate, and runs one registered rule (panics if it is not registered).
- `lint/config.rs`: `ontoforge.toml` is deserialized into a generic `Raw` tree in which every key and value is a `toml::Spanned`, so each V15 points at the offending key (unknown rule, ONT-V id, unknown section, undeclared option) or value (bad level, bad option value). TOML syntax errors are V15 at `toml::de::Error::span()`. `load_config` reads `--config` (missing is V15) or `<root>/ontoforge.toml` (missing means defaults), enforces the 4 MiB limit, and registers the text in the `SourceMap` so the diagnostic renders with a snippet.
- `lint/words.rs`: `split_camel` (`ReleaseDate`, `code01`, `HTTPServer`, `FlightV2`, `isCurrent`).
- `lint/e08.rs`: addLink/removeLink on a non-manyToMany link (help names the `to` type and its foreign key), modify or delete after delete, duplicate createObject by primary key source. N/A items are written into the module doc comment.
- `lint/e09.rs`: discouraged primary key types as warning, nested array `items` as error, title key check kept via `is_title_suitable` (never fires in v1).
- `ontoforge lint`: `Command::Lint(LintArgs)`, `lint_cmd::lint` (`pipeline::check`, then config, then rules; either kind of error skips the rules), shared `report()` in `lib.rs` for both commands, human stderr summary in three forms (validation failed, configuration invalid, counts by severity).

## Task Commits

1. **Task 1 (TDD):** RED `833cf1c` (19 of 22 lint tests failed on assertions against stubs), GREEN `3ba4d6d`
2. **Task 2 (TDD):** RED `35e0a76` (6 of 7 tests failed: no findings, no snapshots), GREEN `e2c500d`
3. **Task 3 (TDD):** RED `a416ed9` (14 of 14 CLI tests failed: the subcommand did not exist), GREEN `fdb68ac`

**Plan metadata:** the docs commits that carry this file and the STATE/ROADMAP update.

## TDD Gate Compliance

RED before GREEN for all three tasks. Task 1 and Task 2 REDs contain compiling stubs (`run_rules`, `parse_config`, `load_config`, `split_camel`, the two `check` bodies) so the tests fail on assertions instead of compile errors; GREEN replaces them. `aviation_lints_clean` and `config_off_disables_the_rule` (E08) pass in RED by design (empty registry; stub rule never fires). No REFACTOR commits were needed.

## Verification output

```
cargo test --workspace                                              -> all ok (ontoforge-schema lib 55, action_types 26, diagnostics 15, link_types 17, lint_examples 1, load_examples 3, loader 28, object_types 14; ontoforge cli 5, cli_lint 14, invalid_examples 1, lib 3; ontoforge-expr analyze 22)
cargo clippy --workspace --all-targets -- -D warnings               -> clean
cargo fmt --all --check                                             -> clean
cargo test -p ontoforge-schema lint::                               -> 30 passed (lib)
cargo test -p ontoforge-schema --test lint_examples                 -> 1 passed
cargo test -p ontoforge-schema unknown_lint_rule_id_is_error        -> 1 passed
cargo test -p ontoforge --test cli_lint                             -> 14 passed
cargo run -q -p ontoforge -- lint examples/ontologies/aviation --deny-warnings --format json -> [] (exit 0)
cargo run -q -p ontoforge -- lint examples/ontologies/invalid/v04-unknown-field --format json | jq -e 'all(.[]; .code | startswith("ONT-V"))' -> true, lint exit 1
fd -e new . crates -H -I                                            -> no .snap.new
rg "Lint\(LintArgs" crates/ontoforge/src/cli.rs                     -> match
rg "pipeline::check" crates/ontoforge/src/lint_cmd.rs               -> match
rg -c '"ONT-E08"' e08.rs / '"ONT-E09"' e09.rs / pk_suitability e09.rs -> 3 / 2 / match
4 .snap files under lint/snapshots (e08 and e09, triggers and clean)
```

## Interface changes

- New public API in `ontoforge_schema::lint` as listed in the frontmatter. `parse_config` takes `&[&dyn LintRule]` (the plan wrote `&[&'static dyn LintRule]`); every caller that passes a `'static` slice still compiles.
- `LintConfig::level(id)` and `LintConfig::options(id)` are the read accessors; the fields stay private.
- New CLI surface: `ontoforge lint <DIR> [--config <PATH>] [--format human|json] [--color auto|always|never] [--deny-warnings]`. `ontoforge::lib` now has a shared `report()`; `validate` output and exit codes are unchanged.
- `pipeline.rs` and `Cargo.toml` / `Cargo.lock` were not touched (01-12 owns `pipeline.rs`).

## Decisions Made

See `key-decisions` in the frontmatter. No implementation choice contradicted an ADR: ONT-V15 is the code catalog entry planned in 01-01, and ONT-V structural errors never pass through level resolution (D-06).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug in plan-adjacent expectation] `did_you_mean` cannot suggest ONT-W01 for ONT-W1**
- **Found during:** Task 1 (`unknown_lint_rule_id_is_error`)
- **Issue:** The behavior requires the help for `"ONT-W1"` to suggest `ONT-W01`, but Jaro-Winkler ranks `ONT-W10` first (it shares a longer prefix), and edit distance ties `ONT-W01`, `ONT-W10`, `ONT-W11`.
- **Fix:** `zero_padded()` in `config.rs` tries the zero-padded form of a numeric id first (`ONT-W1` to `ONT-W01`), then falls back to `did_you_mean`.
- **Files modified:** crates/ontoforge-schema/src/lint/config.rs
- **Commit:** 3ba4d6d

**2. [Rule 3 - Blocking] clippy `doc_markdown` and an unused enum field**
- **Found during:** Task 1 (clippy gate)
- **Fix:** `PascalCase` in backticks in a doc comment; the `Raw::Bool(bool)` variant was dropped (a boolean maps to `Raw::Other`, which is rejected the same way as a level or option value).
- **Commit:** 3ba4d6d

### Small additions to plan wording (no behavior outside the plan)

- `parse_config` signature generalized from `&'static dyn LintRule` to `&dyn LintRule` so unit tests can pass local probe rules.
- Added tests beyond the behavior list: `lint_rule_on_panics_for_an_unregistered_rule` (also removes a dead-code warning on the helper before E08 exists), `config_options_parse`, `lint_must_be_a_table`, `toml_syntax_error_is_v15_at_its_span`, `every_problem_is_reported`, `option_for_unknown_rule_or_with_bad_value_is_error`, and CLI tests `config_error_level_fails`, `missing_config_flag_path_fails`, `validate_codes_cannot_be_turned_off_by_config`, `lint_summary_counts_by_severity`.
- Datetime values (toml hands them over as a one-key map) are folded into `Raw::Other` so they read as an unsupported value, not as a table with a private key.

**Total deviations:** 2 auto-fixed (1 Rule 1, 1 Rule 3), plus the additions above. No scope change.

## Issues Encountered

- Two long `bash` heredocs were rejected by the tool before running (unexpected EOF); nothing had been modified, and the files were written with the Write tool instead.
- The plan's `<human-check>` (colored output in Windows Terminal) cannot be done by the executor. Only the machine-checkable half was done (ANSI escapes present with `--color always`, none in JSON output, none with `NO_COLOR`).

## Known Stubs

None. The `naming::rules()` and `structure::rules()` bodies return an empty `Vec` by design: they are the registration seams that 01-10 and 01-11 fill.

## Threat Model Status

| Threat | Disposition | Status |
|---|---|---|
| T-01-26 config disables structural validation | mitigate | `ONT-V*` keys are V15 (`validate_codes_cannot_be_configured`); `lint()` returns before the config is consulted when validation has errors, and validate diagnostics never pass through `run_rules` (`validate_codes_cannot_be_turned_off_by_config`) |
| T-01-27 typo silently ignored | mitigate | unknown rule id, level, section, option, non-table `lint` / `options`, bad option value are all V15 with spans (config.rs tests) |
| T-01-28 `--config` reads any path | accept | local CLI; diagnostics show the path as given, `/`-normalized, never expanded |
| T-01-29 regex DoS | mitigate | E08 and E09 use no regular expressions; `words.rs` is a char scan |

## Threat Flags

None.

## Next Phase Readiness

- 01-10 (W01-W05) and 01-11 (W06-W11): add rule modules under `lint/naming/` and `lint/structure/` and return them from the `rules()` seam; use `split_camel`, `LintCx::option_*`, and `lint_rule_on` with insta snapshots as E08/E09 do. Until a W rule is registered, `[lint.options."ONT-Wxx"]` for it reports "takes no options" (V15), because the config checks keys against `option_keys()` of registered rules; this resolves itself when the rule lands. `aviation_lints_clean` covers new rules automatically.
- 01-12: `lint_cmd::lint` calls `pipeline::check`, so the V13 CEL errors it adds will stop lint without edits here. Temporary ontologies in `cli_lint.rs` carry no `submissionCriteria`, so they stay clean after the merge.
- Pending human check: view `cargo run -q -p ontoforge -- lint <dir with an E08 case>` in Windows Terminal (colors, borders, `NO_COLOR=1`).

## Self-Check: PASSED

Files verified present: lint/{mod,config,words,e08,e09}.rs, lint/naming/mod.rs, lint/structure/mod.rs, the 4 lint snapshots, tests/lint_examples.rs, lint_cmd.rs, tests/cli_lint.rs, tests/snapshots/cli_lint__e08_human.snap. Commits `833cf1c`, `3ba4d6d`, `35e0a76`, `e2c500d`, `a416ed9`, `fdb68ac` exist; `git rev-list --count 60f1b73..HEAD` = 6 before this SUMMARY commit.
