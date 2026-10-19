---
phase: "3"
slug: "edit-store-scope-model"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-10-19"
---

# Phase 3 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Requirement → test map source: `03-RESEARCH.md` § Validation Architecture.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` + proptest 1.12.0 + insta 1.49.0 + wiremock 0.6.5 |
| **Config file** | none — proptest failure persistence follows `printer_prop.rs` (`FileFailurePersistence::WithSource("proptest-regressions")`) |
| **Quick run command** | `cargo test -p ontoforge-expr -p ontoforge-auth -p ontoforge-storage --locked` |
| **Full suite command** | `cargo test --workspace --locked` (Windows and Linux, SQLite only) |
| **Estimated runtime** | ~180 seconds (proptest: 256 cases pure functions, 64 cases with SQLite) |

---

## Sampling Rate

- **After every task commit:** `cargo test -p <changed crate> --locked` (or `--test <name>` for the single file)
- **After every plan wave:** `cargo test --workspace --locked` + `cargo clippy --workspace --all-targets --locked -- -D warnings` + `cargo fmt --all --check`
- **Before `/gsd-verify-work`:** full suite green on Windows; Linux CI `integration` job PG conformance + Trino fixed corpus green
- **Max feedback latency:** 180 seconds

---

## Per-Task Verification Map

Task IDs filled in by the planner (2026-10-20). Test targets follow the plans; the research draft's separate `replay` / `append_only` targets were folded into the shared conformance harness (`tests/conformance/mod.rs`, run by `conformance_sqlite` and `conformance_pg`) and are selected by test-name filter.

| Requirement | Plan / Task | Test Type | Automated Command | File Exists | Status |
|-------------|-------------|-----------|-------------------|-------------|--------|
| STORE-01 | 03-03 T2-T3, 03-07 T2-T3, 03-11 T3, 03-18 T1 | integration | `cargo test -p ontoforge-storage --test conformance_sqlite --locked`; PG: `cargo test -p ontoforge-storage --test conformance_pg --test reload_pg --test scope_store --locked -- --ignored` (`ONTOFORGE_PG_URL` or `--features embedded-pg`) | ❌ W0 | ⬜ pending |
| STORE-02 | 03-03 T2, 03-08 T1-T2, 03-13 T1-T2, 03-16 T1-T3 | integration | `cargo test -p ontoforge-storage --test reload --test migrate --locked`; `cargo test -p ontoforge --test cli_reload --test cli_migrate --locked`; Trino: `cargo test -p ontoforge-storage --test reload_trino_it --locked -- --ignored` | ❌ W0 | ⬜ pending |
| STORE-03 | 03-07 T2-T3 | integration | `cargo test -p ontoforge-storage --test conformance_sqlite --locked commit_appends_one_log_record`; `... replay`; `... append_only` | ❌ W0 | ⬜ pending |
| STORE-04 | 03-15 T1-T2 | integration | `cargo test -p ontoforge-storage --test history_masking --locked` | ❌ W0 | ⬜ pending |
| STORE-05 | 03-08 T1, 03-11 T1/T3 | integration | `cargo test -p ontoforge-storage --test reload --locked one_to_one_unique_index_rejects_second_row`; `cargo test -p ontoforge-storage --test conformance_sqlite --locked concurrent_fk_claims_one_wins` | ❌ W0 | ⬜ pending |
| STORE-06 | 03-17 T1-T2 | integration | `cargo test -p ontoforge-storage --test import --locked`; `cargo test -p ontoforge --test cli_import --locked` | ❌ W0 | ⬜ pending |
| STORE-07 | 03-07 T2, 03-11 T1-T2 | property | `cargo test -p ontoforge-storage --test model_prop --locked` | ❌ W0 | ⬜ pending |
| SCOPE-01 / SCOPE-07 | 03-05, 03-06, 03-12, 03-14 T1-T2, 03-16 T1 | integration | `cargo test -p ontoforge --test cli_scope --locked`; `cargo test -p ontoforge-storage --test scope_store --locked` | ❌ W0 | ⬜ pending |
| SCOPE-02 | 03-05 T1-T2, 03-12 T1-T2 | unit/snapshot | `cargo test -p ontoforge-schema --test scope_documents --locked`; `cargo test -p ontoforge-auth --test scope_compile --locked` | ❌ W0 | ⬜ pending |
| SCOPE-03 | 03-09 T3 | property + example | `cargo test -p ontoforge-auth --test scope_merge --locked` | ❌ W0 | ⬜ pending |
| SCOPE-04 | 03-04 T2-T3, 03-09 T1-T2, 03-18 T2 | property / integration | `cargo test -p ontoforge-expr --test rowpred_eval --locked`; `cargo test -p ontoforge-auth --test rowpred_diff --locked`; Trino: `cargo test -p ontoforge-auth --test rowpred_trino_it --locked -- --ignored` | ❌ W0 | ⬜ pending |
| SCOPE-05 | 03-04 T3, 03-09 T3, 03-12 T2 | unit | `cargo test -p ontoforge-expr --test rowpred_subset --locked` | ❌ W0 | ⬜ pending |
| SCOPE-06 | 03-12 T2, 03-14 T1, 03-16 T2 | unit/integration | `cargo test -p ontoforge-auth --test criteria_coverage --locked`; `cargo test -p ontoforge-storage --test reload --locked reload_rechecks_applied_roles` | ❌ W0 | ⬜ pending |
| AUTH-01 | 03-06 T1-T2 | unit (wiremock) | `cargo test -p ontoforge-auth --test oidc --locked` | ❌ W0 | ⬜ pending |
| AUTH-02 | 03-10 T1-T2 | unit | `cargo test -p ontoforge-auth --test dev_token --locked`; `cargo test -p ontoforge --test cli_dev_token --locked` | ❌ W0 | ⬜ pending |
| guard | 03-01 T3, 03-03 T3 | static | `cargo test -p ontoforge-storage --test sql_exit_guard --locked`; `cargo tree --workspace --features ontoforge-storage/embedded-pg -i openssl` reports "did not match any packages" | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] workspace dependency / feature changes; `cargo tree -i openssl` guard
- [ ] `crates/ontoforge-storage/tests/conformance.rs` generic harness + `conformance_sqlite.rs` + `conformance_pg.rs` (`#[ignore]`)
- [ ] reference model + `model_prop.rs`, `replay.rs`, `append_only.rs`, `sql_exit_guard.rs`
- [ ] `crates/ontoforge-expr/tests/rowpred_subset.rs`; `crates/ontoforge-auth/tests/{rowpred_diff,scope_merge,criteria_coverage,oidc,dev_token}.rs`
- [ ] SQLite row-set helper for differential tests
- [ ] CI `integration` job: PG conformance + Trino corpus steps; ignored-test count guard
- [ ] aviation example: Role / RoleBinding (multi-role column × row case) and editOnly import data
- [ ] `ci/trino/catalog/editstore.properties` adds `postgresql.array-mapping=AS_ARRAY`

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `postgresql_embedded` conformance on Windows 11 | STORE-01 | Downloads PG binaries; not run in default suite | `cargo test -p ontoforge-storage --features embedded-pg --test conformance_pg -- --ignored` |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 180s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
