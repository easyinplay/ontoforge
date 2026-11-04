---
phase: 03-edit-store-scope-model
plan: 01
subsystem: infra
tags: [cargo, sqlx, sea-query, sea-query-sqlx, jsonwebtoken, aws-lc-rs, sqlite, ci-guard, adr]

requires:
  - phase: 02-source-binding-engine
    provides: ontoforge-engine SQLite stand-in with decimal / timestamp collations
provides:
  - All Phase 3 workspace dependencies and crate manifests declared once (later plans use --locked)
  - ontoforge_storage::store::sqlite_connect_options (recursive_triggers, WAL, busy_timeout, foreign_keys, engine collations)
  - ontoforge_engine::sqlite public exports DECIMAL_COLLATION, TIMESTAMP_COLLATION, compare_decimal, compare_timestamp
  - jsonwebtoken on the aws_lc_rs backend, proven by an HS256 round trip
  - CI guard against openssl / native-tls in the dependency graph
  - ADR-0023 crate dependency direction
affects: [03-02, 03-03, 03-07, 03-08, 03-09, phase-04-query]

plan_head_before: 8d03090da97cca114bd7e8d4ef627bd3d96c9751

actuals:
  tokens: 4850
  tasks: 3
  commits: 3

tech-stack:
  added: [sea-query 1.0.2, sea-query-sqlx 0.9.1, jsonwebtoken 11.1.0 (aws_lc_rs), getrandom 0.4.3, postgresql_embedded 0.21.0 (optional, embedded-pg), sqlx postgres/migrate/macros/json/chrono/tls-rustls-aws-lc-rs, chrono clock]
  patterns:
    - "One plan owns every Cargo.toml and Cargo.lock edit of the phase; later plans run cargo with --locked"
    - "Edit store SQLite connections reuse the engine's collation functions, so stand-in and store share semantics"

key-files:
  created:
    - crates/ontoforge-storage/src/store.rs
    - crates/ontoforge-storage/tests/sqlite_connection.rs
    - crates/ontoforge-auth/tests/crypto_backend.rs
    - .planning/adr/0023-phase3-crate-dependency-direction.md
  modified:
    - Cargo.toml
    - Cargo.lock
    - crates/ontoforge-storage/Cargo.toml
    - crates/ontoforge-storage/src/lib.rs
    - crates/ontoforge-auth/Cargo.toml
    - crates/ontoforge-expr/Cargo.toml
    - crates/ontoforge/Cargo.toml
    - crates/ontoforge-engine/src/printer/sqlite_ddl.rs
    - crates/ontoforge-engine/src/sqlite/mod.rs
    - .github/workflows/ci.yml
    - docs/designs/trino-engine-sql-tiers.md

key-decisions:
  - "Dependency direction: storage -> schema, expr, auth, engine; auth -> schema, expr, engine; storage no longer depends on query (ADR-0023)"
  - "jsonwebtoken uses only the aws_lc_rs backend; no second crypto stack"
  - "postgresql_embedded has default features off and is compiled only under the test feature embedded-pg"

patterns-established:
  - "Supply-chain guard: cargo tree -i openssl / native-tls must report 'did not match any packages' (CI step and local check)"

# STORE-01 and AUTH-01 are addressed (prerequisites only) but NOT complete: conformance tests, JWKS fetch and
# the edit store itself land in later plans (03-07, 03-09, 03-13). REQUIREMENTS.md is left unchanged on purpose.
requirements-completed: []
requirements-addressed: [STORE-01, AUTH-01]

coverage:
  - id: D1
    description: "Every pooled SQLite edit store connection has recursive_triggers on and foreign_keys on"
    requirement: STORE-01
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/sqlite_connection.rs#every_pooled_connection_has_recursive_triggers"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/sqlite_connection.rs#foreign_keys_on"
        status: pass
    human_judgment: false
  - id: D2
    description: "Edit store SQLite connections register the engine's decimal and timestamp collations ('1.5' equals '1.50')"
    requirement: STORE-01
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/sqlite_connection.rs#engine_collations_are_registered"
        status: pass
    human_judgment: false
  - id: D3
    description: "jsonwebtoken signs and verifies HS256 with the aws_lc_rs backend without panicking"
    requirement: AUTH-01
    verification:
      - kind: integration
        ref: "crates/ontoforge-auth/tests/crypto_backend.rs#hs256_round_trip_does_not_panic"
        status: pass
    human_judgment: false
  - id: D4
    description: "Dependency graph has no openssl or native-tls, including with the embedded-pg feature"
    requirement: STORE-01
    verification:
      - kind: other
        ref: "cargo tree --workspace --locked --features ontoforge-storage/embedded-pg -e normal,build,dev -i openssl (and -i native-tls) -> 'did not match any packages'"
        status: pass
    human_judgment: false
  - id: D5
    description: "ADR-0023 records the Phase 3 crate dependency direction and the design doc points to it"
    verification: []
    human_judgment: true
    rationale: "Document content is a design record; a reviewer judges wording and completeness"

duration: 40min (continuation after the Task 1 checkpoint)
completed: 2026-10-20
status: complete
---

# Phase 3 Plan 1: Dependencies, crate graph and tracer Summary

**All Phase 3 dependencies declared once, crate graph corrected (storage -> auth / expr / engine), an SQLite edit store connection with recursive_triggers and the engine's decimal / timestamp collations, an aws-lc-rs JWT round trip, and a CI guard that keeps openssl / native-tls out of the graph.**

## Performance

- **Duration:** about 40 min for the continuation (Tasks 2 and 3); Task 1 was a human checkpoint in an earlier run
- **Completed:** 2026-10-20
- **Tasks:** 3 of 3
- **Files modified:** 15 (including `Cargo.lock`)

## Accomplishments

- Root `Cargo.toml` carries every dependency and feature Phase 3 needs; 03-02..03-20 do not touch a manifest or the lockfile.
- `ontoforge-storage` no longer depends on `ontoforge-query`; it depends on schema, expr, auth and engine. `ontoforge-auth` depends on schema, expr, engine and jsonwebtoken. `cargo metadata` checks for all three acceptance conditions pass.
- `sqlite_connect_options` yields connections with `recursive_triggers = ON`, `foreign_keys`, 5 s busy timeout, WAL, and the engine's two collations (same functions as the stand-in).
- jsonwebtoken 11.1.0 panics without a crypto provider (observed in RED); with the `aws_lc_rs` feature the HS256 round trip passes.
- CI `test` job has the step `No OpenSSL or native-tls in the dependency graph`; the same commands pass locally with `embedded-pg` enabled.

## Task Commits

1. **Task 1: Human verification of sea-query-sqlx and getrandom** - no commit (checkpoint, resolved with `approved`)
2. **Task 2: Tracer (TDD)**
   - RED: `84ddf12` (test) - manifests plus failing tests
   - GREEN: `6994756` (feat) - connect options, engine exports, `aws_lc_rs` feature
   - REFACTOR: none needed
3. **Task 3: Supply-chain guard, ADR-0023, design doc pointer** - `e6d4823` (chore)

**Plan metadata:** committed separately after this file (docs commit).

## Task 1 evidence (legitimacy gate)

Gathered before the user replied `approved`:

| Crate | Version | Yanked | Repository | License | Owners | Downloads |
|-------|---------|--------|------------|---------|--------|-----------|
| sea-query-sqlx | 0.9.1 | no | github.com/SeaQL/sea-query | MIT OR Apache-2.0 | tyt2y3 | about 2.26M, created 2025-08-07 |
| getrandom | 0.4.3 | no | github.com/rust-random/getrandom | MIT OR Apache-2.0 | dhardy, github:rust-random:maintainers | about 2.30B |

`Cargo.lock` already contained `getrandom 0.4.3` and `aws-lc-rs 1.18.2` and no `openssl` / `native-tls`.

Confirmed in Task 2 against the real crate (the open item from the checkpoint): `cargo metadata` shows sea-query-sqlx 0.9.1 requires `sea-query ^1.0.0` and `sqlx ^0.9`, and the workspace compiled with the feature set from the plan. No crate was substituted. `SqlxBinder` / `build_sqlx` usage is not exercised in this plan (first used in 03-08); the crate resolves and compiles against sqlx 0.9.0.

## TDD evidence

- **RED:** `cargo test -p ontoforge-storage --test sqlite_connection` failed on the two targeted tests: `every_pooled_connection_has_recursive_triggers` (assertion `left: 0, right: 1`) and `engine_collations_are_registered` (`no such collation sequence: ontoforge_decimal`). `cargo test -p ontoforge-auth --test crypto_backend` failed with the jsonwebtoken panic "Could not automatically determine the process-level CryptoProvider". Each failure is on the planned behavior, not a compile or discovery error. `foreign_keys_on` passed in RED because sqlx enables foreign keys by default; it is kept as a regression guard.
- **GREEN:** all 3 storage tests and the auth test pass.
- **Gate check:** `test(03-01)` commit `84ddf12` precedes `feat(03-01)` commit `6994756`.

## Verification

- `cargo test --workspace --locked`: exit 0, 57 test binaries "ok", no failures.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: clean.
- `cargo fmt --all --check`: clean.
- `cargo tree --workspace --locked --features ontoforge-storage/embedded-pg -e normal,build,dev -i openssl` and `-i native-tls`: both "did not match any packages"; the bash guard (`! ... && ! ...`) exits 0.
- `cargo metadata` acceptance checks for storage (no query; has auth, engine, expr) and auth (has engine, jsonwebtoken): all true.

## Decisions Made

- Feature names in the plan (`tls-rustls-aws-lc-rs` for sqlx, `["tokio", "theseus", "tls-rustls-aws-lc-rs"]` for postgresql_embedded, `aws_lc_rs` and `use_pem` for jsonwebtoken, sea-query backend / `with-*` / `postgres-array` features) all resolved as written; no correction was needed (RESEARCH A3 held for the combination that was compiled). `postgresql_embedded` is only compiled under `embedded-pg`, so its feature set is resolved by the lockfile and the `cargo tree` run with that feature, not by running it.
- ADR-0023 dated 2026-10-20 (local date of execution).

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- `Cargo.toml` is stored with CRLF in the working tree; git normalizes to LF on commit and prints a warning. No content effect.

## Known Stubs

None. The `ontoforge-auth` crate still has only its placeholder `lib.rs` and a smoke test; that is the plan's scope (implementation arrives in 03-05..03-07), not a stub hiding a goal.

## Threat Flags

None. No new network endpoints, auth paths or trust-boundary file access beyond the plan's threat model (T-03-SC, T-03-01, T-03-02, T-03-03 mitigated as planned: human legitimacy gate, no second TLS stack guard, embedded-pg test-only feature, `recursive_triggers` on every pooled connection).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- 03-02 and later plans can use `--locked`; every dependency they need is already in `Cargo.lock`.
- `sqlite_connect_options` is ready for 03-03 / 03-09 (the append-only trigger itself lands in 03-09).
- `postgresql_embedded` was only resolved, never launched, in this plan; the first plan that starts it (PG conformance) should expect the runtime download of PostgreSQL binaries.

---
*Phase: 03-edit-store-scope-model*
*Completed: 2026-10-20*

## Self-Check: PASSED

- Files exist: `crates/ontoforge-storage/src/store.rs`, `crates/ontoforge-storage/tests/sqlite_connection.rs`, `crates/ontoforge-auth/tests/crypto_backend.rs`, `.planning/adr/0023-phase3-crate-dependency-direction.md`
- Commits exist: `84ddf12`, `6994756`, `e6d4823`
- `git rev-list --count 8d03090..HEAD` = 3 (matches `actuals.commits`)
