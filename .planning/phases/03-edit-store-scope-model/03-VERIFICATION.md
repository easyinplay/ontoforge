---
phase: 03-edit-store-scope-model
verified: 2026-10-21T04:45:00Z
status: gaps_found
score: 4/8 must-haves verified
covered_files:
  - ".github/workflows/ci.yml"
  - ".planning/REQUIREMENTS.md"
  - ".planning/adr/0023-phase3-crate-dependency-direction.md"
  - ".planning/adr/0024-edit-store-physical-format.md"
  - ".planning/adr/0025-row-predicate-semantics.md"
  - ".planning/adr/0026-edit-log-format.md"
  - ".planning/phases/03-edit-store-scope-model/03-01-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-01-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-02-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-02-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-03-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-03-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-04-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-04-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-05-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-05-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-06-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-06-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-07-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-07-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-08-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-08-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-09-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-09-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-10-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-10-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-11-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-11-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-12-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-12-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-13-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-13-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-14-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-14-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-15-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-15-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-16-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-16-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-17-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-17-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-18-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-18-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-19-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-19-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-20-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-20-SUMMARY.md"
  - "CONTRIBUTING.md"
  - "Cargo.lock"
  - "Cargo.toml"
  - "ci/pg/rowpred_corpus.sql"
  - "ci/trino/catalog/editstore.properties"
  - "crates/ontoforge-auth/Cargo.toml"
  - "crates/ontoforge-auth/src/authn/dev.rs"
  - "crates/ontoforge-auth/src/authn/jwks.rs"
  - "crates/ontoforge-auth/src/authn/mod.rs"
  - "crates/ontoforge-auth/src/authn/oidc.rs"
  - "crates/ontoforge-auth/src/authn/principal.rs"
  - "crates/ontoforge-auth/src/lib.rs"
  - "crates/ontoforge-auth/src/scope/compile.rs"
  - "crates/ontoforge-auth/src/scope/coverage.rs"
  - "crates/ontoforge-auth/src/scope/lower.rs"
  - "crates/ontoforge-auth/src/scope/merge.rs"
  - "crates/ontoforge-auth/src/scope/mod.rs"
  - "crates/ontoforge-auth/tests/criteria_coverage.rs"
  - "crates/ontoforge-auth/tests/crypto_backend.rs"
  - "crates/ontoforge-auth/tests/data/README.md"
  - "crates/ontoforge-auth/tests/data/oidc_es256.pem"
  - "crates/ontoforge-auth/tests/data/oidc_rs256.pem"
  - "crates/ontoforge-auth/tests/data/rowpred_corpus.json"
  - "crates/ontoforge-auth/tests/dev_token.rs"
  - "crates/ontoforge-auth/tests/oidc.rs"
  - "crates/ontoforge-auth/tests/rowpred_common/mod.rs"
  - "crates/ontoforge-auth/tests/rowpred_diff.rs"
  - "crates/ontoforge-auth/tests/rowpred_trino_it.rs"
  - "crates/ontoforge-auth/tests/scope_compile.rs"
  - "crates/ontoforge-auth/tests/scope_merge.rs"
  - "crates/ontoforge-auth/tests/snapshots/scope_compile__scope_compile_diagnostics_json.snap"
  - "crates/ontoforge-engine/src/config.rs"
  - "crates/ontoforge-engine/src/printer/ast.rs"
  - "crates/ontoforge-engine/src/printer/call.rs"
  - "crates/ontoforge-engine/src/printer/dialect.rs"
  - "crates/ontoforge-engine/src/printer/escape.rs"
  - "crates/ontoforge-engine/src/printer/mod.rs"
  - "crates/ontoforge-engine/src/printer/sqlite_ddl.rs"
  - "crates/ontoforge-engine/src/printer/tests.rs"
  - "crates/ontoforge-engine/src/sqlite/fixture.rs"
  - "crates/ontoforge-engine/src/sqlite/mod.rs"
  - "crates/ontoforge-engine/tests/printer_prop.rs"
  - "crates/ontoforge-engine/tests/server_config.rs"
  - "crates/ontoforge-engine/tests/sql_text_only_from_printer.rs"
  - "crates/ontoforge-engine/tests/trino_client.rs"
  - "crates/ontoforge-expr/Cargo.toml"
  - "crates/ontoforge-expr/src/lib.rs"
  - "crates/ontoforge-expr/src/rowpred/eval.rs"
  - "crates/ontoforge-expr/src/rowpred/ir.rs"
  - "crates/ontoforge-expr/src/rowpred/mod.rs"
  - "crates/ontoforge-expr/src/rowpred/parse.rs"
  - "crates/ontoforge-expr/src/rowpred/value.rs"
  - "crates/ontoforge-expr/tests/common/mod.rs"
  - "crates/ontoforge-expr/tests/rowpred_eval.proptest-regressions"
  - "crates/ontoforge-expr/tests/rowpred_eval.rs"
  - "crates/ontoforge-expr/tests/rowpred_subset.rs"
  - "crates/ontoforge-schema/src/diag.rs"
  - "crates/ontoforge-schema/src/ir/attrs.rs"
  - "crates/ontoforge-schema/src/ir/mod.rs"
  - "crates/ontoforge-schema/src/ir/names.rs"
  - "crates/ontoforge-schema/src/ir/object.rs"
  - "crates/ontoforge-schema/src/ir/scope.rs"
  - "crates/ontoforge-schema/src/lib.rs"
  - "crates/ontoforge-schema/src/loader/discover.rs"
  - "crates/ontoforge-schema/src/loader/dispatch.rs"
  - "crates/ontoforge-schema/src/loader/mod.rs"
  - "crates/ontoforge-schema/src/loader/scope.rs"
  - "crates/ontoforge-schema/src/loader/sourcemap.rs"
  - "crates/ontoforge-schema/src/schema_export.rs"
  - "crates/ontoforge-schema/tests/object_types.rs"
  - "crates/ontoforge-schema/tests/schema_export.rs"
  - "crates/ontoforge-schema/tests/scope_documents.rs"
  - "crates/ontoforge-schema/tests/snapshots/scope_documents__scope_diagnostics.snap"
  - "crates/ontoforge-storage/Cargo.toml"
  - "crates/ontoforge-storage/migrations/postgres/0001_definitions.sql"
  - "crates/ontoforge-storage/migrations/postgres/0002_edit_log.sql"
  - "crates/ontoforge-storage/migrations/postgres/0003_scope.sql"
  - "crates/ontoforge-storage/migrations/sqlite/0001_definitions.sql"
  - "crates/ontoforge-storage/migrations/sqlite/0002_edit_log.sql"
  - "crates/ontoforge-storage/migrations/sqlite/0003_scope.sql"
  - "crates/ontoforge-storage/src/commit.rs"
  - "crates/ontoforge-storage/src/ddl.rs"
  - "crates/ontoforge-storage/src/diff.rs"
  - "crates/ontoforge-storage/src/error.rs"
  - "crates/ontoforge-storage/src/history.rs"
  - "crates/ontoforge-storage/src/import.rs"
  - "crates/ontoforge-storage/src/index.rs"
  - "crates/ontoforge-storage/src/lib.rs"
  - "crates/ontoforge-storage/src/log.rs"
  - "crates/ontoforge-storage/src/migrate.rs"
  - "crates/ontoforge-storage/src/naming.rs"
  - "crates/ontoforge-storage/src/physical.rs"
  - "crates/ontoforge-storage/src/reload.rs"
  - "crates/ontoforge-storage/src/scope_store.rs"
  - "crates/ontoforge-storage/src/snapshots/ontoforge_storage__ddl__tests__ddl_postgres.snap"
  - "crates/ontoforge-storage/src/snapshots/ontoforge_storage__ddl__tests__ddl_sqlite.snap"
  - "crates/ontoforge-storage/src/sql_exit.rs"
  - "crates/ontoforge-storage/src/state.rs"
  - "crates/ontoforge-storage/src/store.rs"
  - "crates/ontoforge-storage/src/value.rs"
  - "crates/ontoforge-storage/tests/conformance/mod.rs"
  - "crates/ontoforge-storage/tests/conformance_pg.rs"
  - "crates/ontoforge-storage/tests/conformance_sqlite.rs"
  - "crates/ontoforge-storage/tests/history_masking.rs"
  - "crates/ontoforge-storage/tests/import.rs"
  - "crates/ontoforge-storage/tests/migrate.rs"
  - "crates/ontoforge-storage/tests/migrate_pg.rs"
  - "crates/ontoforge-storage/tests/model/mod.rs"
  - "crates/ontoforge-storage/tests/model_prop.proptest-regressions"
  - "crates/ontoforge-storage/tests/model_prop.rs"
  - "crates/ontoforge-storage/tests/reload.rs"
  - "crates/ontoforge-storage/tests/reload_pg.rs"
  - "crates/ontoforge-storage/tests/reload_trino_it.rs"
  - "crates/ontoforge-storage/tests/scope_store.rs"
  - "crates/ontoforge-storage/tests/sql_exit_guard.rs"
  - "crates/ontoforge-storage/tests/sqlite_connection.rs"
  - "crates/ontoforge-storage/tests/support/mod.rs"
  - "crates/ontoforge/Cargo.toml"
  - "crates/ontoforge/src/cli.rs"
  - "crates/ontoforge/src/dev_token.rs"
  - "crates/ontoforge/src/import.rs"
  - "crates/ontoforge/src/lib.rs"
  - "crates/ontoforge/src/migrate.rs"
  - "crates/ontoforge/src/online.rs"
  - "crates/ontoforge/src/reload.rs"
  - "crates/ontoforge/src/scope.rs"
  - "crates/ontoforge/tests/cli_dev_token.rs"
  - "crates/ontoforge/tests/cli_import.rs"
  - "crates/ontoforge/tests/cli_migrate.rs"
  - "crates/ontoforge/tests/cli_reload.rs"
  - "crates/ontoforge/tests/cli_schema.rs"
  - "crates/ontoforge/tests/cli_scope.rs"
  - "docs/designs/trino-engine-sql-tiers.md"
  - "docs/edit-store.md"
  - "docs/scope.md"
  - "examples/data/aviation/airports.csv"
  - "examples/data/aviation/airports.json"
  - "examples/ontologies/aviation-json/bindings/dev/bindings.json"
  - "examples/ontologies/aviation-json/objects.json"
  - "examples/ontologies/aviation/bindings/dev/flight.yaml"
  - "examples/ontologies/aviation/objects.yaml"
  - "examples/scopes/aviation/bindings.yaml"
  - "examples/scopes/aviation/roles.yaml"
  - "examples/server/ontoforge-server.dev.toml"
  - "schemas/ontoforge-definition.schema.json"
  - "schemas/ontoforge-scope.schema.json"
covered_digest: "v1:sha256:0dcd9b981709e4ba9f55f34a4d32079f2d13d9190453f7c4c9496a04c703960e"
behavior_unverified: 1
overrides_applied: 0
re_verification: null
gaps:
  - truth: "A decimal value is never rounded: what a client sends is what the typed edit row, the append-only log and the history snapshot hold (SC1 strong typing, SC2 immutable log, STORE-03, STORE-06; 03-08 and 03-19 value rule)"
    status: failed
    reason: "PropertyValue::from_json_as takes a JSON number for a decimal through serde_json::Number::to_string(). The workspace serde_json has no arbitrary_precision, so a fractional number is already an f64. Reproduced: input 12345678.1234567891 is printed as 12345678.12345679 (review CR-A01, confirmed independently). The rounded value is padded to scale, passes validation and is written to the edit row, the immutable log and the history. ontoforge import of a JSON file reaches this path today (import.rs:763, :921); Phase 5/7 REST and MCP bodies will too. Only 12.5 is covered by a unit test, so the suite cannot see it."
    artifacts:
      - path: "crates/ontoforge-storage/src/value.rs"
        issue: "line 326: Value::Number(n) => V::Decimal(n.to_string()) accepts a binary-float number for a decimal"
    missing:
      - "Accept a JSON number for a decimal only when it is an exact integer (n.is_i64() || n.is_u64()); otherwise fail with 'send a decimal as a string such as \"12.50\"' (or enable serde_json/arbitrary_precision workspace-wide and evaluate separately)"
      - "Regression test with 17 or more significant digits for both the JSON import path and PropertyValue::from_json, in the shared conformance cases"
  - truth: "The documented dev quickstart works from a fresh checkout (SC1 'load the example ontology', SC5 'dev token authenticates', AUTH-02)"
    status: failed
    reason: "examples/server/ontoforge-server.dev.toml points the SQLite store and the dev key at ../../target/ontoforge-dev/, which does not exist in a clone. Neither DevKey::load_or_create nor the SQLite opener creates the parent directory. Reproduced on a scratch copy with target/ present: 'ontoforge dev-token ...' exits 1 ('dev key file ... cannot be created'); 'ontoforge reload examples/ontologies/aviation --server-config ...' exits 3 ('unable to open database file', ONT-V26). Both succeed after mkdir -p target/ontoforge-dev. docs/scope.md uses this config for reload, scope plan and scope apply (review CR-D01)."
    artifacts:
      - path: "crates/ontoforge/src/dev_token.rs"
        issue: "no create_dir_all for the key file's parent before DevKey::load_or_create"
      - path: "crates/ontoforge-storage/src/store.rs"
        issue: "the SQLite opener does not create the parent directory of [store] path"
      - path: "examples/server/ontoforge-server.dev.toml"
        issue: "header claims the key is created on first use; it is not, on a fresh checkout"
    missing:
      - "create_dir_all of the parent in both places (or document and script 'mkdir -p target/ontoforge-dev')"
      - "CLI test that runs the example config from an empty temp tree"
  - truth: "Criteria column coverage (SCOPE-06 / ONT-V23) holds on every path that changes the definition or the role set, and for every read a criterion can make (SC5)"
    status: partial
    reason: "Two verified holes. (1) 'ontoforge migrate' is the only way to activate a destructive definition change, and it never runs the stored-role check: apply_migration_probed has no roles input and crates/ontoforge/src/migrate.rs does not pass any, while reload does (ReloadOptions::roles, D-18). A role that names a retyped or archived column stays 'applied' (review WR-B07, confirmed by search: no match for roles, RolesCheck, load_scope_in or compile_scope in storage src/migrate.rs, and none for roles in crates/ontoforge/src/migrate.rs). (2) coverage::problem() returns None for a bare 'objects.<param>' read (segments.get(2)? short-circuits), although the criteria checker accepts it (expr/check.rs object_problem with property None). Reproduced: an action with condition 'params.delayMinutes > 0 && objects.flight != null' granted to a role whose allow-list does not name every Flight property passes 'scope plan' (review WR-C05); the same condition with 'objects.flight.aircraftTailNumber' is correctly refused with ONT-V23."
    artifacts:
      - path: "crates/ontoforge-storage/src/migrate.rs"
        issue: "apply_migration_probed takes no RolesCheck and does not call load_scope_in/compile_scope under the lock"
      - path: "crates/ontoforge/src/migrate.rs"
        issue: "CLI does not load the stored roles for the check"
      - path: "crates/ontoforge-auth/src/scope/coverage.rs"
        issue: "line ~82: let property = path.segments.get(2)?; a whole-object read yields no diagnostic"
    missing:
      - "roles: Option<RolesCheck> on apply_migration_probed, checked after lock_def_state and before the steps run; CLI passes it; test mirroring reload_rechecks_applied_roles"
      - "Treat a bare objects.<param> read as reading every property: ONT-V23 unless the allow-list covers all properties of the object type; test with objects.flight == params.x"
deferred:
  - truth: "SCOPE-07 remainder: every request uses the definition version it started with (read side); agent MCP exposes no mutation tool; role/scope changes reach only the administration channel over REST/MCP"
    addressed_in: "Phase 6 and Phase 7"
    evidence: "ROADMAP Phase 6 success criterion 5: 'role / scope 只能经管理通道或 L3 修改并审计；定义 reload 先 validate / lint，失败不切换；每个请求固定使用开始时的定义版本'; Phase 7 criterion 2: 'list_tools ... L3 与任何变更工具不出现'. Phase 3 delivered the write-side fence (reload/commit def_version), scope_store as the only writer of of_role/of_role_binding (source guard test) and the append-only of_admin_audit."
behavior_unverified_items:
  - truth: "Reload makes new tables/columns visible in Trino before the new definition becomes active, including when Trino's JDBC metadata cache is on (STORE-02, Trino flush step)"
    test: "Run reload_trino_it against a Trino catalog with metadata.cache-ttl=10m and metadata.cache-missing=true (not set in ci/trino/catalog/editstore.properties), looking at a table first, then adding a column, then reloading"
    expected: "The reload activates only after Trino shows the new column; with the flush call removed from wait_until_visible the test fails (it does not today)"
    why_human: "The green CI run (2 tests in reload_trino_it) cannot tell 'the flush worked' from 'nothing was cached': the CI catalog has the default metadata.cache-ttl=0s, and a failing flush is tolerated by the probe. 03-20 SUMMARY and review IN-B04 record the same. The polling and ordering are proven with a scripted Trino (reload.rs probe_* tests) and a real one, but the cache-flush effect is not isolated."
human_verification:
  - test: "Cache-flush isolation on Trino (see behavior_unverified_items)"
    expected: "reload_trino_it fails when system.flush_metadata_cache() is skipped and passes when it runs, on a catalog with metadata caching enabled"
    why_human: "Needs a Trino with a non-default catalog setting; one-line CI catalog change plus a test-first-look-then-add-column step"
---

# Phase 3: Edit Store & Scope Model Verification Report

**Phase Goal:** edit store 按 object type 生成强类型表并安全演进，所有写入有不可变 edit log；用户 / 组 / 角色与 scope 规则可配置，同一份 CEL 行谓词既能编译成 SQL 也能在 Rust 中对单行求值且结果一致
**Verified:** 2026-10-21T04:45:00Z
**Status:** gaps_found
**Re-verification:** No, initial verification
**Mode:** mvp phase in ROADMAP, but the goal is not a user story; verified with the standard goal-backward method (the MVP user-story guard was not applied).

## Verdict

The phase is substantially delivered. Four of the five roadmap Success Criteria are proven by passing tests, a green CI run and my own end-to-end CLI runs. Fixes are needed before the phase can be called done:

1. a silent decimal-rounding defect that writes wrong numbers into the immutable log (CR-A01);
2. a documented quickstart that fails on a fresh checkout (CR-D01);
3. two real holes in the SCOPE-06 criteria-coverage guarantee (migrate path, bare `objects.<param>` read).

All four are small, local fixes. None needs a redesign. One behavior (Trino metadata cache flush) is present and wired but not isolated by any test; it goes to human verification.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC1: Loading the example ontology creates an edit table per object type and per M:N link; additive changes apply on reload; destructive changes are stopped (ONT-V25) and `ontoforge migrate --plan/--apply` completes them and archives old columns; a failed reload keeps the old definition | ✓ VERIFIED | Ran the built binary on a scratch copy of `examples/`: `reload` applied def_version 1 with 8 DDL statements; changing `Aircraft.seatCapacity` to `string` made `reload` exit 1 with ONT-V25 and "the edit store was not changed"; `migrate --plan` listed `archive column ... seat_capacity -> seat_capacity__arch_v3` + `add column`; `migrate --apply --plan-hash <hash>` gave `def_version 3 applied`; `--plan-hash 0000` was refused ("the migration plan changed"). Tests: `reload` 32, `migrate` 13, `cli_reload` 7, `cli_migrate` 9 all pass. Bound to a pre-created `target/ontoforge-dev/`; see gap 2 for the fresh-checkout failure. |
| 2 | SC2: One conformance suite passes on SQLite and PostgreSQL; edit log and history tables are append-only | ✓ VERIFIED | `all_cases!` generates `conformance_sqlite` (27 pass locally) and `conformance_pg` (27 pass in CI on PostgreSQL 17, run 39541882392, log lines "running 27 tests ... 27 passed"). Triggers on all three log tables in `migrations/{sqlite,postgres}/0002_edit_log.sql` (BEFORE UPDATE/DELETE; PG also BEFORE TRUNCATE); cases `append_only_rejects_update_and_delete`, `_truncate_on_pg`, `_sqlite_replace_writes`. Guard step "Ignored tests are actually there" green (10 targets, none zero). |
| 3 | SC3: Overlay semantics (per-property patch, set mask, tombstone, release, monotonic version) are covered by a proptest against an in-memory reference model | ✓ VERIFIED | `tests/model_prop.rs` `model_matches_store`: 1 to 30 commits of 1 to 3 edits over Gate/Stand objects and links with set/unset/tombstone/release/link add-remove; reference model in `tests/model/mod.rs`; compares outcome class, all rows and versions, then replays the log into a fresh store; 64 cases default, regression file committed. Passes. |
| 4 | SC4: Roles grant object types, property allow-list, CEL row predicate (same-row properties and `principal.attrs` only), actions, SQL levels, toolsets; absent attribute is false; multi-role merge has no "A's columns x B's rows" leak | ✓ VERIFIED | `scope_documents` 16, `scope_compile` 21, `scope_merge` 9 (`a_columns_times_b_rows_do_not_leak`, `no_grant_is_invisible`, property test `expected()` oracle), `rowpred_eval` 14 (`missing_attr_is_unknown_and_invisible`), `rowpred_subset` 13. `merge.rs`: `row = OR(R_i)`, `columns[x] = OR(R_i where x in C_i)`; no grant gives `None` (invisible). |
| 5 | SC5: The same CEL row predicate gives the same result as SQL and in Rust (differential tests); criteria column-coverage lint is enforced at reload and on role change; OIDC JWT and dev token both authenticate | ✗ FAILED (partial) | Differential: `rowpred_diff` 6 pass locally (Rust vs SQLite, corpus + 256 seeded predicates); CI `rowpred_trino_it` 3 pass on Trino 483 (log: "3 passed ... 25.66s"). OIDC: `oidc` 31 pass (alg none, HS256-with-public-key, tampered, expired, wrong iss/aud, header key URLs never followed). Dev token: `dev_token` 15 + `cli_dev_token` 11 pass. Coverage lint exists and is wired into `scope plan/apply` and `reload`, but has two holes (gap 3): the migrate path skips it, and a bare `objects.<param>` read is not flagged. |
| 6 | Dev quickstart: the documented example config works from a fresh checkout (SC1 first step, SC5 dev token, AUTH-02) | ✗ FAILED | Reproduced `dev-token` exit 1 and `reload` exit 3 until `target/ontoforge-dev/` is created by hand (gap 2). |
| 7 | A decimal value is never rounded on its way into the typed row, the immutable log and the history (SC1 "强类型", SC2 "不可变 log", 03-08 / 03-19 value rule) | ✗ FAILED | `value.rs:326` plus `Cargo.toml:13` (no `arbitrary_precision`). Independent repro in a scratch crate with the workspace's `serde_json` features: parsing `12345678.1234567891` and calling `Number::to_string()` prints `12345678.12345679` (gap 1). |
| 8 | Reload makes new tables/columns visible in Trino before the switch, including through the Trino metadata cache (STORE-02) | ⚠️ PRESENT_BEHAVIOR_UNVERIFIED | Probe, ordering, pending/active switch, timeout keeping the old definition: `reload.rs` `probe_*` (scripted Trino) and CI `reload_trino_it` 2 pass on real Trino 483. The flush call's effect is not isolated (CI catalog caches nothing; flush failure is tolerated). Human item below. |

**Score:** 4/8 truths verified (1 present, behavior-unverified; 3 failed or partial)

### Decisions on the code-review findings

The review (`03-REVIEW.md`: 4 critical, 28 warning, 25 info) was read in full. Each critical was reproduced or read in code, then classified:

| Finding | Decision | Reason |
|---|---|---|
| CR-A01 decimal via f64 | **Must-have gap** (gap 1) | Reproduced. Breaks the stated "never rounded" invariant on the append-only log, reachable through the shipped `ontoforge import` JSON path. One-line fix. |
| CR-D01 quickstart fails on fresh checkout | **Must-have gap** (gap 2) | Reproduced both commands. The first documented command of the SC1 flow and the AUTH-02 flow fails. The mechanism works once the directory exists, so this is a first-run defect, not a design failure. Two `create_dir_all` calls. |
| CR-B01 OpenSSL CI guard cannot fail for `openssl` | **Advisory** | Verified the claim (`bash -c 'set -eo pipefail; ! true; echo reached; ! true'` prints `reached`). The outcome the must-have names holds today: `cargo tree -i openssl` and `-i native-tls` both report "did not match any packages". Only the first line of the guard is inert, so a future OpenSSL dependency would be missed. Fix before the next dependency change. |
| CR-D02 config parse errors echo the rejected value | **Advisory (fix soon)** | Reproduced (`lock_timeout_ms = "hunter2secret"` prints the string). Threat T-03-04 (PG connection string) holds: `url`/`password`/`token`/`secret` are rewritten value-free and only `url_env`/`url_file` are accepted. The leak needs a secret pasted into an unrelated typed field. Header comments claiming "never secret values" are wrong and should be fixed with the code. |

Warnings promoted to gap 3 because they break SCOPE-06 / SC5 as written: WR-B07 and WR-C05 (both verified above). Warnings kept as advisory: WR-A01 (NUL and year 0 accepted by validation, rejected by PostgreSQL; conformance cannot see it), WR-A02 (no lock timeout on the commit path), WR-A03 (PG text reads depend on `DateStyle` and `extra_float_digits`; can silently shorten doubles into history snapshots), WR-A04 (unbounded commit size), WR-A05 (`OnExisting::Update` needs full rows), WR-B01..B06, WR-B08..B10, WR-C01..C04, C06, C07, WR-D01..D06. WR-A01 and WR-A03 are the most worth fixing in the same pass as gap 1, since they share the value-validation boundary.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/ontoforge-storage/src/{commit,log,history,import,migrate,reload,scope_store,sql_exit,state,store,ddl,diff,physical,naming,index,value}.rs` | edit store | ✓ VERIFIED | 10,026 lines, no stubs; exercised by the storage test binaries (conformance, model_prop, reload, migrate, import, history_masking, scope_store); no TBD/FIXME/XXX/TODO/HACK/unimplemented markers in any file changed by the phase |
| `crates/ontoforge-storage/migrations/{sqlite,postgres}/000{1,2,3}_*.sql` | definitions, log, scope tables with append-only triggers | ✓ VERIFIED | Triggers present on log, entry, history (and audit via 0003); tested on both backends |
| `crates/ontoforge-expr/src/rowpred/` | CEL subset to `Pred`, Kleene eval | ✓ VERIFIED | `rowpred_eval`, `rowpred_subset` pass |
| `crates/ontoforge-auth/src/{authn,scope}/` | OIDC/JWKS, dev token, scope compile, merge, lower, coverage | ✓ VERIFIED with gap 3 | `coverage.rs` hole |
| `crates/ontoforge/src/{reload,migrate,scope,import,dev_token}.rs` | CLI | ✓ VERIFIED with gaps 2 and 3 | `cli_*` suites pass |
| `schemas/ontoforge-scope.schema.json`, `examples/scopes/aviation`, `examples/server/ontoforge-server.dev.toml` | scope documents | ✓ VERIFIED | `scope plan` of the example worked on a scratch copy; the dev config has the gap 2 defect |
| `docs/edit-store.md`, `docs/scope.md`, `.planning/adr/0023..0026` | operator guide, ADRs | ✓ VERIFIED | all four ADRs `status: accepted` |
| `.github/workflows/ci.yml`, `ci/trino/catalog/editstore.properties` | PG conformance, Trino corpus and probe | ✓ VERIFIED | run 39541882392 green on all 3 jobs |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `commit()` | `of_def_state` fence | first statement, `FOR SHARE` (PG) / `BEGIN IMMEDIATE` (SQLite) | ✓ WIRED | `stale_writer_rejected_after_switch`, `reload_racing_commits_never_lets_an_old_write_through` (both backends) |
| `reload` | stored roles (D-18) | `ReloadOptions::roles` under the lock | ✓ WIRED | `reload_rechecks_applied_roles`, CLI `reload_reports_role_breakage` |
| `migrate` | stored roles | none | ✗ NOT_WIRED | gap 3 |
| `scope apply` | `compile_scope` + `of_admin_audit` | `scope_store::apply_in` | ✓ WIRED | `apply_writes_roles_bindings_and_audit`, `admin_audit_is_append_only`; source guard `scope_tables_written_only_by_scope_store` |
| row predicate | SQL (Trino/SQLite) and Rust eval | `lower()` and `eval()` over one `Pred` | ✓ WIRED | `rowpred_diff` plus Trino corpus in CI |
| `TypeScope::evaluate` | `history()` masking | same function on the current row | ✓ WIRED | `history_masking` 12 pass (+1 PG in CI) |
| `import` | `Store::commit` as `system:import` | batches through the fence | ✓ WIRED | `import` 14 pass, `cli_import` 8 pass |
| JSON number | decimal column | `from_json_as` | ✗ LOSSY | gap 1 |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| edit rows / log / history | `PropertyValue` | `canonical_as` then bound/cast SQL | Yes, except decimal from a fractional JSON number | ⚠️ gap 1 |
| `history()` response | visible columns | current row via `TypeScope::evaluate`, snapshot values | Yes | ✓ FLOWING |
| `scope plan` | role/binding diff | directory loader vs `of_role`/`of_role_binding` | Yes | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Workspace tests | `cargo test --workspace --locked` | exit 0; 84 test binaries; 971 passed, 0 failed, 64 ignored (the ignored ones are the PG/Trino suites, run in CI) | ✓ PASS |
| Reload creates tables | `ontoforge reload examples/ontologies/aviation --server-config ...` (scratch copy, dir pre-created) | `ok: def_version 1 applied (8 DDL statement(s))` | ✓ PASS |
| Destructive change blocked | change `seatCapacity` type, reload | ONT-V25, exit 1, store untouched | ✓ PASS |
| Migrate plan/apply/hash guard | `migrate --plan`, `--apply --plan-hash`, wrong hash | plan with archive step; `def_version 3 applied`; wrong hash refused | ✓ PASS |
| Scope plan on the example | `scope plan examples/scopes/aviation ...` | add 3 roles, 3 bindings, 1 index, plan hash | ✓ PASS |
| Coverage lint, property read | criteria reads `objects.flight.aircraftTailNumber`, role lacks it | ONT-V23 | ✓ PASS |
| Coverage lint, bare object read | criteria reads `objects.flight != null` | plan accepted | ✗ FAIL (gap 3) |
| Quickstart from a fresh tree | `dev-token` and `reload` with the example config, no `target/ontoforge-dev` | exit 1 and exit 3 | ✗ FAIL (gap 2) |
| Decimal fidelity | `serde_json` parse of `12345678.1234567891`, `Number::to_string()` | `12345678.12345679` | ✗ FAIL (gap 1) |
| No OpenSSL / native-tls | `cargo tree ... -i openssl`, `-i native-tls` | both "did not match any packages" | ✓ PASS |

CI evidence (inspected with `gh run view 39541882392` and its job log): head `c250125`, event `pull_request`, conclusion success; `integration`, `test (ubuntu-latest)`, `test (windows-latest)` all green. Commits after `c250125` change only `.planning/` files and one line of `docs/edit-store.md` (`git diff --stat c250125 HEAD`), so the run covers the verified code. The PG suites run in the `integration` job: `conformance_pg` 27, `reload_pg` 11, `scope_store` 3, `migrate_pg` 3, `import` 2, `history_masking` 1 (47 total, matching the 47/47 on embedded PostgreSQL), plus `rowpred_trino_it` 3 and `reload_trino_it` 2 on Trino 483.

### Probe Execution

Step 7c: SKIPPED. The phase declares no probes (`scripts/` does not exist and no PLAN or SUMMARY names a `probe-*.sh`).

### Requirements Coverage

All 16 IDs in the roadmap appear in at least one PLAN's `requirements` frontmatter; none is orphaned. Judged on evidence, not on the checkbox.

| Requirement | Source Plans | Checkbox now | Verdict | Evidence |
|-------------|--------------|--------------|---------|----------|
| STORE-01 | 03-01, 03-03, 03-04, 03-08, 03-09, 03-13, 03-20 | ticked | ✓ SATISFIED | Same suite on SQLite and PG17 (27/27 each); reload, scope, migrate, history, import PG suites pass in CI. Caveat: PG-only validation limits (WR-A01) are not in the shared cases. |
| STORE-02 | 03-02, 03-03, 03-04, 03-10, 03-15, 03-18, 03-20 | unticked | ✓ SATISFIED, one human item open | Auto DDL, ONT-V25 block, migrate archive/rename/drop-archived, fence, `lock_timeout` on PG, pending/probe/active switch all tested. The three reasons given in 03-20 for leaving it unticked: cross-version log replay is not in the requirement text (it is an ADR-0026 extension); SQLite "database is locked" is not mapped to `LockTimeout` (dev backend, PG proven); the Trino flush is not isolated (truth 8, human). Recommend ticking after the human item. |
| STORE-03 | 03-08, 03-09 | unticked | ✓ SATISFIED, with gap 1 | One log record per commit with actor, action, time, params, entries; append-only on both backends; replay reproduces edit tables (`Diverged` check). Decimal fidelity defect noted. |
| STORE-04 | 03-17 | unticked | ✓ SATISFIED | `EditStore::history` with D-13 masking, `NotFound` identical for invisible/absent/ungranted, `params` admin-only; library API per D-13, shells are Phase 7. |
| STORE-05 | 03-10, 03-13 | unticked | ✓ SATISFIED | oneToOne partial unique index plus `CardinalityViolation` and `concurrent_fk_claims_one_wins`; source-side cardinality is a lint/validation matter. |
| STORE-06 | 03-19 | ticked | ⚠️ PARTIAL (gap 1) | `ontoforge import` into editOnly types/links as `system:import`; CSV path pads decimals from text correctly; JSON path rounds fractional decimals. |
| STORE-07 | 03-08, 03-13 | ticked | ✓ SATISFIED | `model_prop`, conformance transition cases. |
| SCOPE-01 | 03-06, 03-07, 03-14, 03-16 | unticked | ✓ SATISFIED | `of_user`/`of_role`/`of_role_binding`, `scope plan/apply` with user and group bindings, groups from the JWT `groups` claim (issuer-prefixed), `record_login`. `cli_scope` 8, `scope_store` 11 (+3 PG in CI). |
| SCOPE-02 | 03-02, 03-05, 03-06, 03-11, 03-14, 03-16 | unticked | ✓ SATISFIED | Role schema covers types, allow-list, `rowFilter`, actions, `sqlLevels`, `toolsets`, `l2`; compile and snapshot tests. |
| SCOPE-03 | 03-11 | unticked | ✓ SATISFIED | `merge.rs` plus `scope_merge` including the property test. |
| SCOPE-04 | 03-05, 03-11, 03-20 | ticked | ✓ SATISFIED | Rust vs SQLite locally; Rust vs Trino 483 in CI on the same corpus and seeded predicates. |
| SCOPE-05 | 03-05, 03-11, 03-14 | unticked | ✓ SATISFIED | No grant is invisible; missing principal attribute is Unknown and therefore invisible; uncompilable predicate is ONT-V22 at plan/apply/reload. Advisories C04, C06, C07 are hardening, not current leaks. |
| SCOPE-06 | 03-14, 03-16, 03-18 | ticked | ⚠️ PARTIAL (gap 3) | Holds at `scope apply` and `reload` for property reads; not on `migrate`, not for a bare object read. The tick should be withdrawn until gap 3 closes. |
| SCOPE-07 | 03-16, 03-18 | unticked | ⚠️ PARTIAL, remainder deferred | Phase 3 part delivered: `scope_store.rs` is the only writer (source guard), audited and append-only; definition fence on writes; reload is deployer-only. Request-pinned version on reads and "no mutation tool in MCP" belong to Phases 4 to 7 (see `deferred`). Correctly left unticked. |
| AUTH-01 | 03-01, 03-02, 03-07 | ticked | ✓ SATISFIED | `oidc` 31 tests: asymmetric-only allow-list, required claims, JWKS from config only, no redirects, size cap, single-flight, stale cap. |
| AUTH-02 | 03-12 | ticked | ⚠️ PARTIAL (gap 2) | Token minting and verification work and are tested; the documented example config fails on a fresh checkout. |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/ontoforge-storage/src/value.rs` | 326 | JSON float number accepted for decimal | 🛑 Blocker | gap 1 |
| `crates/ontoforge-auth/src/scope/coverage.rs` | ~82 | `get(2)?` skips a whole-object read | 🛑 Blocker | gap 3 |
| `.github/workflows/ci.yml` | 33 | `! cmd` under `errexit` cannot fail for the first line | ⚠️ Warning | latent; outcome currently true |
| `crates/ontoforge-engine/src/config.rs` | 438 | parse error text echoes the rejected value | ⚠️ Warning | secret pasted into a typed field is printed |
| `crates/ontoforge-auth/src/scope/compile.rs` | 180-184 | failed filter compile maps to `row: None` ("all rows") | ⚠️ Warning | safe only because `compile_scope` returns `Err`; type should say so (WR-C06) |
| `crates/ontoforge-expr/src/rowpred/eval.rs` | 64-73 | absent row property reads as Null | ⚠️ Warning | caller bug widens visibility on `x == null \|\| ...` (WR-C04) |

No `TBD`, `FIXME`, `XXX`, `TODO`, `HACK`, `PLACEHOLDER` or `unimplemented!` marker exists in any code, config, doc or CI file changed by the phase.

### Regression Check (Phases 1 and 2)

Phase 01 (`status: passed`, 10/11) and Phase 02 (`status: passed`, 15/15) verification reports were read. Their test suites are part of the 971 passing workspace tests (for example `cli_lint` 17, `cli_check_pk` 8, `cli_online` 12, `cli_online_trino` 15, `cli_schema` 4 all pass), and `cli_schema` (schema export) passes. No regression found.

### Human Verification Required

#### 1. Trino metadata cache flush is actually effective

**Test:** Add `metadata.cache-ttl=10m` and `metadata.cache-missing=true` to `ci/trino/catalog/editstore.properties`. Change `reload_trino_it` to query the table first, then add a column, then reload. Run it once as is, and once with the `flush_metadata_cache` call removed from the probe.
**Expected:** Passes with the flush, fails without it.
**Why human:** Needs a Trino with a cache-enabled catalog; today's green run cannot distinguish a working flush from an empty cache (03-20 SUMMARY assumption A5, review IN-B04).

### Gaps Summary

Three gaps block `passed`:

1. **Decimal rounding (STORE-03 / STORE-06 / SC1-SC2).** Change `Value::Number(n)` for decimals to accept integers only, add a 17-digit test on the JSON import path and `from_json`.
2. **Fresh-checkout quickstart (AUTH-02 / SC1).** Create the parent directory of the dev key and of the SQLite store file; add a CLI test from an empty tree.
3. **SCOPE-06 coverage holes (SC5).** Run the stored-role check inside `migrate`; treat a bare `objects.<param>` as reading every property.

Suggested closure: one `/gsd-plan-phase 3 --gaps` round with three small plans, folding in the cheap neighbours WR-A01 and WR-A03 (same value boundary) and CR-B01 (CI guard) and CR-D02 (config error text). After that, re-verify; tick STORE-02 once the human item passes, and withdraw the SCOPE-06 tick until gap 3 is closed.

---

_Verified: 2026-10-21T04:45:00Z_
_Verifier: Claude (gsd-verifier)_
