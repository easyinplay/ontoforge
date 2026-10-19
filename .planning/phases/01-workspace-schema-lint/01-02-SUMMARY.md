---
phase: 01-workspace-schema-lint
plan: 02
subsystem: workspace-ci
tags: [rust, cargo-workspace, github-actions, insta, cel]

requires:
  - phase: 01-workspace-schema-lint
    provides: "01-01 workspace root, ontoforge-schema, ontoforge CLI, ADR-0012"
provides:
  - "10-crate workspace per ADR-0011 (8 new compilable crate skeletons with CRATE_NAME and a placeholder test)"
  - "Cargo.lock containing the cel 0.15.0 dependency tree (no further lock changes expected in this phase)"
  - ".github/workflows/ci.yml: fmt, clippy -D warnings, test on windows-latest and ubuntu-latest, contents: read"
  - "CONTRIBUTING.md: local gates, insta flow, fixture naming, diagnostic code rules"
  - "ADR-0003 Spike S1 conclusion replacing the Open (spike) section"
affects: [01-03, 01-04, 01-07, 01-12]

plan_head_before: aed4bf1ca3f6330c125bce9c32e086488e99dc45
actuals:
  tokens: 9000
  tasks: 2
  commits: 2

tech-stack:
  added: [cel 0.15.0 (via ontoforge-expr)]
  patterns:
    - "Skeleton crates: doc comment + CRATE_NAME const + one prefix test, no placeholder types"
    - "Path dependencies encode the ADR-0011 direction: client -> api only; server -> action/auth/api/query"

key-files:
  created:
    - crates/ontoforge-{expr,query,storage,action,auth,api,server,client}/Cargo.toml
    - crates/ontoforge-{expr,query,storage,action,auth,api,server,client}/src/lib.rs
    - .github/workflows/ci.yml
    - CONTRIBUTING.md
  modified:
    - crates/ontoforge/Cargo.toml
    - Cargo.lock
    - .planning/adr/0003-ontology-definition-format.md

key-decisions:
  - "No new decisions; dependency edges taken literally from the plan, which follows ADR-0011"

requirements-completed: [DX-03]

coverage:
  - id: D1
    description: "All 10 ontoforge-* crates compile and run a passing test; cargo test --workspace passes on Windows without Docker"
    requirement: DX-03
    verification:
      - kind: integration
        ref: "cargo test --workspace"
        status: pass
      - kind: other
        ref: "cargo metadata --no-deps --format-version 1 | jq -e '[.packages[].name | select(startswith(\"ontoforge\"))] | length == 10'"
        status: pass
    human_judgment: false
  - id: D2
    description: "Crate dependency direction follows ADR-0011 (client depends only on api; cli depends on expr)"
    requirement: DX-03
    verification:
      - kind: other
        ref: "cargo tree -p ontoforge-client --edges normal --depth 1 shows only ontoforge-api"
        status: pass
    human_judgment: false
  - id: D3
    description: "CI workflow runs the three gates on windows-latest and ubuntu-latest with contents: read and no secrets"
    requirement: DX-03
    verification:
      - kind: other
        ref: "rg checks from the plan <verify> and acceptance_criteria (all matched; secrets. and Open (spike) absent)"
        status: pass
    human_judgment: true
  - id: D4
    description: "CI actually executes green on both platforms after the first push"
    requirement: DX-03
    verification: []
    human_judgment: true

duration: n/a (not timed)
completed: 2026-10-15
status: complete
---

# Phase 1 Plan 02: Full workspace, CI and contribution conventions Summary

**The workspace now has all 10 ADR-0011 crates compiling with a passing placeholder test each, `cel` is locked in `Cargo.lock`, and a Windows + Ubuntu GitHub Actions workflow runs fmt, clippy and tests.**

## Accomplishments

- Added `ontoforge-expr`, `-query`, `-storage`, `-action`, `-auth`, `-api`, `-server`, `-client`. Each `lib.rs` is a doc comment naming the owning phase, `pub const CRATE_NAME`, and one test asserting the `ontoforge-` prefix. No business code, no empty traits or structs.
- Dependency edges: expr -> schema, cel (dev: insta); query -> schema; storage -> schema, query; action -> storage, expr; auth -> schema; api -> schema, query; server -> action, auth, api, query; client -> api; CLI additionally -> expr. No new external crates beyond `cel`, which was already declared in the root `[workspace.dependencies]` by 01-01.
- `ci.yml` per D-18: matrix `windows-latest` / `ubuntu-latest`, `fail-fast: false`, toolchain `1.98.0`, top-level `permissions: contents: read`, no secrets, `pull_request` trigger only.
- `CONTRIBUTING.md` (Chinese): gates, `INSTA_UPDATE=always`, CI does not write snapshots, LF line endings, `Command::env` instead of `set_var`, `examples/ontologies/invalid/<code>-<slug>/`, diagnostic codes never reused, ADR-0002 naming, ADR-0012 as format source.
- ADR-0003: `## Open (spike)` replaced by `## Spike 结论（2026-10-15）` with the RESEARCH text and a pointer to ADR-0012; status stays accepted.

## Task Commits

1. **Task 1: remaining 8 crate skeletons** - `2ea8957` (feat)
2. **Task 2: CI, CONTRIBUTING, ADR-0003** - `eb9a4fe` (docs)

**Plan metadata:** added by the final docs commit (SUMMARY, STATE, ROADMAP, REQUIREMENTS).

## Verification output

```
cargo build --workspace            -> Finished (cel 0.15.0, antlr4rust 0.6.0 and nom 7.1.3 compiled)
cargo test --workspace             -> every ontoforge-* crate has a "Running unittests" line, all "test result: ok"
  ontoforge lib 3, cli 5, invalid_examples 1; ontoforge-schema lib 23, diagnostics 15;
  each of expr/query/storage/action/auth/api/server/client: 1 passed; no FAILED
cargo metadata ... | jq -e '[...] | length == 10'   -> true
cargo clippy --workspace --all-targets -- -D warnings   -> exit 0 (after fixing doc_markdown, see below)
cargo fmt --all --check                                  -> exit 0
cargo tree -p ontoforge-client --edges normal --depth 1  -> ontoforge-client -> ontoforge-api only
git show --name-only --format= 2ea8957 | rg -x "Cargo.toml"  -> no match (root Cargo.toml untouched; Cargo.lock only)
rg windows-latest / ubuntu-latest / "contents: read" / clippy line / fmt / test / toolchain "1.98.0" / fail-fast: false  -> all match
rg "secrets\." ci.yml -> none; rg "Open \(spike\)" ADR-0003 -> none
rg "serde_path_to_error|saphyr" ADR-0003 -> matches; rg "examples/ontologies/invalid/|Command::env" CONTRIBUTING.md -> both match
```

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] clippy `doc_markdown` on skeleton doc comments**
- **Found during:** Task 1 (clippy gate with pedantic lints)
- **Issue:** `ObjectSet` and `StorageProvider` in crate doc comments lacked backticks; `-D warnings` failed on `ontoforge-query`.
- **Fix:** wrapped both identifiers in backticks before the Task 1 commit.
- **Files modified:** crates/ontoforge-query/src/lib.rs, crates/ontoforge-storage/src/lib.rs
- **Commit:** 2ea8957

**Total deviations:** 1 auto-fixed. No scope change.

## Not verified here

- The CI workflow has not been executed: there is no push yet. Both-platform green is confirmed only after the first push (D4 above). The action tags (`actions/checkout@v7`, `Swatinem/rust-cache@v2`) come from RESEARCH and were not re-queried.
- Ubuntu test behaviour is untested locally (Windows host only).

## Known Stubs

The 8 skeleton crates are intentional placeholders per D-17 (only `CRATE_NAME` and a prefix test). Each doc comment names the phase that fills it: expr 01-07 / 01-12, storage Phase 2, action Phase 3, query Phase 4, auth Phase 5, api / server / client Phase 6, MCP part of server Phase 7. They do not block this plan's goal, so no WINDOWS.md entries were written.

## Threat Flags

None. T-01-05 mitigated (read-only permissions, no secrets, no `pull_request_target`); T-01-SC mitigated (only `cel` 0.15.0 added, lock file committed); T-01-06 accepted as planned.

## Self-Check: PASSED

- Files present: crates/ontoforge-{expr,query,storage,action,auth,api,server,client}/{Cargo.toml,src/lib.rs}, .github/workflows/ci.yml, CONTRIBUTING.md
- Commits found in `git log`: 2ea8957, eb9a4fe
- `git rev-list --count aed4bf1..HEAD` = 2 at the time of writing
