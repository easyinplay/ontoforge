---
phase: 03-edit-store-scope-model
verified: 2026-11-05T11:31:37.586Z
source_head: 060019fe52005c6b85ae5c5f59da9868f464e579
ci_run: 41994479295
status: passed
score: 139/139 must-haves verified
covered_files:
  - ".gitattributes"
  - ".github/workflows/ci.yml"
  - ".planning/REQUIREMENTS.md"
  - ".planning/ROADMAP.md"
  - ".planning/adr/0002-naming-convention.md"
  - ".planning/adr/0016-storage-system-data-and-typed-edit-store.md"
  - ".planning/adr/0018-authz-scope-and-sql-tiers.md"
  - ".planning/adr/0020-query-execution-rust-orchestrated-overlay.md"
  - ".planning/adr/0023-phase3-crate-dependency-direction.md"
  - ".planning/adr/0024-edit-store-physical-format.md"
  - ".planning/adr/0025-row-predicate-semantics.md"
  - ".planning/adr/0026-edit-log-format.md"
  - ".planning/milestones/v1.0-audit/evidence/2026-11-05/ci-41994479295-final.json"
  - ".planning/milestones/v1.0-audit/evidence/2026-11-05/ci-41994479295-integration.log"
  - ".planning/milestones/v1.0-audit/evidence/2026-11-05/ci-41994479295-isolated-query-bench.log"
  - ".planning/milestones/v1.0-audit/evidence/2026-11-05/ci-41994479295-phase7-real-services.log"
  - ".planning/milestones/v1.0-audit/evidence/2026-11-05/ci-41994479295-ubuntu.log"
  - ".planning/milestones/v1.0-audit/evidence/2026-11-05/ci-41994479295-windows.log"
  - ".planning/milestones/v1.0-audit/evidence/2026-11-05/phase03-approval-override/03-VERIFICATION.human-needed.before.md"
  - ".planning/milestones/v1.0-audit/evidence/2026-11-05/phase03-approval-override/authorization.md"
  - ".planning/milestones/v1.0-audit/evidence/2026-11-05/phase03-approval-override/baseline.json"
  - ".planning/milestones/v1.0-audit/evidence/2026-11-05/phase03-approval-override/decision.json"
  - ".planning/milestones/v1.0-audit/evidence/2026-11-05/phase03-approval-override/input-drift-check.json"
  - ".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-03-04/03-VERIFICATION.before.md"
  - ".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-03-04/03-contracts.json"
  - ".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-03-04/04-VERIFICATION.before.md"
  - ".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-03-04/04-contracts.json"
  - ".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-03-04/CONTRACT-RESULTS.md"
  - ".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-03-04/GOVERNANCE-AND-BOUNDARIES.md"
  - ".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-03-04/changed-source-inspection.json"
  - ".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-03-04/collect.cjs"
  - ".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-03-04/covered-file-inspection.json"
  - ".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-03-04/evidence-groups.json"
  - ".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-03-04/evidence-inputs.json"
  - ".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-03-04/executed-tests.json"
  - ".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-03-04/governance-inputs.json"
  - ".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-03-04/inspect.cjs"
  - ".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-03-04/selected-test-source.json"
  - ".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-03-04/service-environment-evidence.json"
  - ".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-03-04/truth-results.json"
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
  - ".planning/phases/03-edit-store-scope-model/03-21-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-21-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-22-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-22-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-23-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-23-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-CONTEXT.md"
  - ".planning/phases/03-edit-store-scope-model/03-UAT.md"
  - ".planning/phases/03-edit-store-scope-model/03-VALIDATION.md"
  - ".planning/phases/04-scoped-l0-read/04-VALIDATION.md"
  - "CONTRIBUTING.md"
  - "Cargo.lock"
  - "Cargo.toml"
  - "ci/check-tls-deps.sh"
  - "ci/pg/rowpred_corpus.sql"
  - "ci/pg/seed.sql"
  - "ci/query-services.sh"
  - "ci/server-services.sh"
  - "ci/test_tls_deps.py"
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
  - "crates/ontoforge-engine/Cargo.toml"
  - "crates/ontoforge-engine/src/config.rs"
  - "crates/ontoforge-engine/src/lib.rs"
  - "crates/ontoforge-engine/src/printer/ast.rs"
  - "crates/ontoforge-engine/src/printer/call.rs"
  - "crates/ontoforge-engine/src/printer/dialect.rs"
  - "crates/ontoforge-engine/src/printer/escape.rs"
  - "crates/ontoforge-engine/src/printer/l2.rs"
  - "crates/ontoforge-engine/src/printer/mod.rs"
  - "crates/ontoforge-engine/src/printer/snapshots/ontoforge_engine__printer__tests__print_sqlite.snap"
  - "crates/ontoforge-engine/src/printer/snapshots/ontoforge_engine__printer__tests__print_trino.snap"
  - "crates/ontoforge-engine/src/printer/sqlite_ddl.rs"
  - "crates/ontoforge-engine/src/printer/tests.rs"
  - "crates/ontoforge-engine/src/trino/cancel.rs"
  - "crates/ontoforge-engine/src/trino/classify.rs"
  - "crates/ontoforge-engine/src/trino/client.rs"
  - "crates/ontoforge-engine/src/trino/decode.rs"
  - "crates/ontoforge-engine/src/trino/headers.rs"
  - "crates/ontoforge-engine/src/trino/l2_health.rs"
  - "crates/ontoforge-engine/src/trino/l2_monitor.rs"
  - "crates/ontoforge-engine/src/trino/mod.rs"
  - "crates/ontoforge-expr/Cargo.toml"
  - "crates/ontoforge-expr/src/analyze.rs"
  - "crates/ontoforge-expr/src/check.rs"
  - "crates/ontoforge-expr/src/infer.rs"
  - "crates/ontoforge-expr/src/lib.rs"
  - "crates/ontoforge-expr/src/limits.rs"
  - "crates/ontoforge-expr/src/rowpred/eval.rs"
  - "crates/ontoforge-expr/src/rowpred/ir.rs"
  - "crates/ontoforge-expr/src/rowpred/mod.rs"
  - "crates/ontoforge-expr/src/rowpred/parse.rs"
  - "crates/ontoforge-expr/src/rowpred/value.rs"
  - "crates/ontoforge-expr/src/walk.rs"
  - "crates/ontoforge-expr/tests/analyze.rs"
  - "crates/ontoforge-expr/tests/common/mod.rs"
  - "crates/ontoforge-expr/tests/criteria.rs"
  - "crates/ontoforge-expr/tests/rowpred_eval.proptest-regressions"
  - "crates/ontoforge-expr/tests/rowpred_eval.rs"
  - "crates/ontoforge-expr/tests/rowpred_subset.rs"
  - "crates/ontoforge-schema/src/diag.rs"
  - "crates/ontoforge-schema/src/ir/action.rs"
  - "crates/ontoforge-schema/src/ir/attrs.rs"
  - "crates/ontoforge-schema/src/ir/basetype.rs"
  - "crates/ontoforge-schema/src/ir/binding.rs"
  - "crates/ontoforge-schema/src/ir/link.rs"
  - "crates/ontoforge-schema/src/ir/manifest.rs"
  - "crates/ontoforge-schema/src/ir/mod.rs"
  - "crates/ontoforge-schema/src/ir/names.rs"
  - "crates/ontoforge-schema/src/ir/object.rs"
  - "crates/ontoforge-schema/src/ir/scope.rs"
  - "crates/ontoforge-schema/src/lib.rs"
  - "crates/ontoforge-schema/src/lint/binding/e13.rs"
  - "crates/ontoforge-schema/src/lint/binding/e14.rs"
  - "crates/ontoforge-schema/src/lint/binding/e15.rs"
  - "crates/ontoforge-schema/src/lint/binding/e16.rs"
  - "crates/ontoforge-schema/src/lint/binding/mod.rs"
  - "crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e13__tests__clean.snap"
  - "crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e13__tests__triggers.snap"
  - "crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e14__tests__clean.snap"
  - "crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e14__tests__triggers.snap"
  - "crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e15__tests__clean.snap"
  - "crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e15__tests__triggers.snap"
  - "crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e16__tests__clean.snap"
  - "crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e16__tests__triggers.snap"
  - "crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__w17__tests__clean.snap"
  - "crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__w17__tests__triggers.snap"
  - "crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__w17__tests__unreadable.snap"
  - "crates/ontoforge-schema/src/lint/binding/w17.rs"
  - "crates/ontoforge-schema/src/lint/config.rs"
  - "crates/ontoforge-schema/src/lint/e08.rs"
  - "crates/ontoforge-schema/src/lint/e09.rs"
  - "crates/ontoforge-schema/src/lint/mod.rs"
  - "crates/ontoforge-schema/src/lint/naming/mod.rs"
  - "crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w01__tests__clean.snap"
  - "crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w01__tests__triggers.snap"
  - "crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w02__tests__clean.snap"
  - "crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w02__tests__triggers.snap"
  - "crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w03__tests__clean.snap"
  - "crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w03__tests__triggers.snap"
  - "crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w04__tests__clean.snap"
  - "crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w04__tests__triggers.snap"
  - "crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w05__tests__clean.snap"
  - "crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w05__tests__triggers.snap"
  - "crates/ontoforge-schema/src/lint/naming/w01.rs"
  - "crates/ontoforge-schema/src/lint/naming/w02.rs"
  - "crates/ontoforge-schema/src/lint/naming/w03.rs"
  - "crates/ontoforge-schema/src/lint/naming/w04.rs"
  - "crates/ontoforge-schema/src/lint/naming/w05.rs"
  - "crates/ontoforge-schema/src/lint/snapshots/ontoforge_schema__lint__e08__tests__clean.snap"
  - "crates/ontoforge-schema/src/lint/snapshots/ontoforge_schema__lint__e08__tests__triggers.snap"
  - "crates/ontoforge-schema/src/lint/snapshots/ontoforge_schema__lint__e09__tests__clean.snap"
  - "crates/ontoforge-schema/src/lint/snapshots/ontoforge_schema__lint__e09__tests__triggers.snap"
  - "crates/ontoforge-schema/src/lint/structure/mod.rs"
  - "crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w06__tests__clean.snap"
  - "crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w06__tests__triggers.snap"
  - "crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w07__tests__clean.snap"
  - "crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w07__tests__triggers.snap"
  - "crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w08__tests__clean.snap"
  - "crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w08__tests__triggers.snap"
  - "crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w09__tests__clean.snap"
  - "crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w09__tests__triggers.snap"
  - "crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w10__tests__clean.snap"
  - "crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w10__tests__triggers.snap"
  - "crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w11__tests__clean.snap"
  - "crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w11__tests__triggers.snap"
  - "crates/ontoforge-schema/src/lint/structure/w06.rs"
  - "crates/ontoforge-schema/src/lint/structure/w07.rs"
  - "crates/ontoforge-schema/src/lint/structure/w08.rs"
  - "crates/ontoforge-schema/src/lint/structure/w09.rs"
  - "crates/ontoforge-schema/src/lint/structure/w10.rs"
  - "crates/ontoforge-schema/src/lint/structure/w11.rs"
  - "crates/ontoforge-schema/src/lint/words.rs"
  - "crates/ontoforge-schema/src/loader/discover.rs"
  - "crates/ontoforge-schema/src/loader/dispatch.rs"
  - "crates/ontoforge-schema/src/loader/fixture.rs"
  - "crates/ontoforge-schema/src/loader/mod.rs"
  - "crates/ontoforge-schema/src/loader/parse.rs"
  - "crates/ontoforge-schema/src/loader/scope.rs"
  - "crates/ontoforge-schema/src/loader/sourcemap.rs"
  - "crates/ontoforge-schema/src/schema_export.rs"
  - "crates/ontoforge-schema/src/validate/actions.rs"
  - "crates/ontoforge-schema/src/validate/bindings.rs"
  - "crates/ontoforge-schema/src/validate/links.rs"
  - "crates/ontoforge-schema/src/validate/mod.rs"
  - "crates/ontoforge-schema/src/validate/objects.rs"
  - "crates/ontoforge-schema/tests/scope_documents.rs"
  - "crates/ontoforge-server/src/context.rs"
  - "crates/ontoforge-server/src/history_page.rs"
  - "crates/ontoforge-server/tests/postgres_http_it.rs"
  - "crates/ontoforge-storage/Cargo.toml"
  - "crates/ontoforge-storage/migrations/postgres/0001_definitions.sql"
  - "crates/ontoforge-storage/migrations/postgres/0002_edit_log.sql"
  - "crates/ontoforge-storage/migrations/postgres/0003_scope.sql"
  - "crates/ontoforge-storage/migrations/postgres/0004_action.sql"
  - "crates/ontoforge-storage/migrations/postgres/0005_read_audit.sql"
  - "crates/ontoforge-storage/migrations/postgres/0006_audit_truncated.sql"
  - "crates/ontoforge-storage/migrations/sqlite/0001_definitions.sql"
  - "crates/ontoforge-storage/migrations/sqlite/0002_edit_log.sql"
  - "crates/ontoforge-storage/migrations/sqlite/0003_scope.sql"
  - "crates/ontoforge-storage/migrations/sqlite/0004_action.sql"
  - "crates/ontoforge-storage/migrations/sqlite/0005_read_audit.sql"
  - "crates/ontoforge-storage/migrations/sqlite/0006_audit_truncated.sql"
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
  - "crates/ontoforge-storage/src/query_read.rs"
  - "crates/ontoforge-storage/src/read_audit.rs"
  - "crates/ontoforge-storage/src/reload.rs"
  - "crates/ontoforge-storage/src/request_snapshot.rs"
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
  - "crates/ontoforge-storage/tests/frozen_history.rs"
  - "crates/ontoforge-storage/tests/history_masking.rs"
  - "crates/ontoforge-storage/tests/import.rs"
  - "crates/ontoforge-storage/tests/migrate.rs"
  - "crates/ontoforge-storage/tests/migrate_pg.rs"
  - "crates/ontoforge-storage/tests/model/mod.rs"
  - "crates/ontoforge-storage/tests/model_prop.proptest-regressions"
  - "crates/ontoforge-storage/tests/model_prop.rs"
  - "crates/ontoforge-storage/tests/query_read.rs"
  - "crates/ontoforge-storage/tests/query_read_pg.rs"
  - "crates/ontoforge-storage/tests/reload.rs"
  - "crates/ontoforge-storage/tests/reload_pg.rs"
  - "crates/ontoforge-storage/tests/reload_trino_it.rs"
  - "crates/ontoforge-storage/tests/request_snapshot.rs"
  - "crates/ontoforge-storage/tests/scope_store.rs"
  - "crates/ontoforge-storage/tests/sql_exit_guard.rs"
  - "crates/ontoforge-storage/tests/sqlite_connection.rs"
  - "crates/ontoforge-storage/tests/support/mod.rs"
  - "crates/ontoforge/Cargo.toml"
  - "crates/ontoforge/src/cli.rs"
  - "crates/ontoforge/src/dev_token.rs"
  - "crates/ontoforge/src/import.rs"
  - "crates/ontoforge/src/lib.rs"
  - "crates/ontoforge/src/main.rs"
  - "crates/ontoforge/src/migrate.rs"
  - "crates/ontoforge/src/reload.rs"
  - "crates/ontoforge/src/scope.rs"
  - "crates/ontoforge/tests/cli_dev_token.rs"
  - "crates/ontoforge/tests/cli_import.rs"
  - "crates/ontoforge/tests/cli_migrate.rs"
  - "crates/ontoforge/tests/cli_reload.rs"
  - "crates/ontoforge/tests/cli_scope.rs"
  - "docs/edit-store.md"
  - "docs/scope.md"
  - "examples/ontologies/aviation/actions.yaml"
  - "examples/ontologies/aviation/bindings/dev/aircraft.yaml"
  - "examples/ontologies/aviation/bindings/dev/fixtures/aircraft.csv"
  - "examples/ontologies/aviation/bindings/dev/fixtures/flight_crew.csv"
  - "examples/ontologies/aviation/bindings/dev/fixtures/flights.csv"
  - "examples/ontologies/aviation/bindings/dev/fixtures/maintenance_records.csv"
  - "examples/ontologies/aviation/bindings/dev/fixtures/pilots.csv"
  - "examples/ontologies/aviation/bindings/dev/flight-crew.yaml"
  - "examples/ontologies/aviation/bindings/dev/flight.yaml"
  - "examples/ontologies/aviation/bindings/dev/maintenance-record.yaml"
  - "examples/ontologies/aviation/bindings/dev/pilot.yaml"
  - "examples/ontologies/aviation/links.yaml"
  - "examples/ontologies/aviation/objects.yaml"
  - "examples/ontologies/aviation/ontology.yaml"
  - "examples/server/ontoforge-server.dev.toml"
  - "rust-toolchain.toml"
  - "schemas/ontoforge-definition.schema.json"
  - "schemas/ontoforge-manifest.schema.json"
  - "schemas/ontoforge-scope.schema.json"
  - "target/milestone-ci-41759804733-integration.log"
  - "target/milestone-ci-41759804733-ubuntu.log"
  - "target/milestone-ci-41759804733-windows.log"
covered_digest: "v4:sha256:4eed4c7e8f8d2e39225dc2ee5e253cade83dae50de448093f84a64a9f0d58a4a"
behavior_unverified: 0
overrides_applied: 1
overrides:
  - must_have: "sea-query-sqlx 与 getrandom（名称来源 [ASSUMED]）只在人工核验 crates.io / docs.rs 页面通过后才写入根 `Cargo.toml`；未核验前本 plan 不执行任何 install"
    reason: "当前会话用户明确接受03-01.1安装前人工批准缺独立留档的历史证据边界并授权单项override；未提供或找到新历史批准记录，本轮授权不是过去安装前批准，不豁免其他合同、行为或安全测试。"
    accepted_by: "当前会话用户（项目/需求负责人）"
    accepted_at: "2026-11-05T11:26:39.587Z"
decision_coverage: {"honored":18,"total":18,"not_honored":[]}
re_verification:
  previous_status: human_needed
  previous_score: 138/139
  trigger: "仅03-01.1本轮显式授权的历史证据边界决议；其余138项与旧345项输入未漂移"
  gaps_closed: ["03-01.1等待开发者明确决议项已由本轮单项PASSED (override)解决；并未找到新历史批准记录"]
  gaps_remaining: []
  regressions: []
human_verification: []
deferred:
  - truth: "STORE-05 source OneToOne基数报告属于Phase07.1独立验收"
    addressed_in: "Phase 7.1"
    evidence: "ROADMAP Phase7.1 SC1/SC2与REQUIREMENTS明确分配source报告；本次只重签03的edit子合同，不替代07.1报告"
---

# Phase 3: Edit Store & Scope Model 验证报告

**Phase Goal:** As a 平台管理员, I want to 让 edit store 按 object type 生成强类型表并安全演进、所有写入有不可变 edit log，配置用户 / 组 / 角色与 scope 规则，并使同一份 CEL 行谓词的 SQL 编译与 Rust 单行求值结果一致, so that 存储和权限行为可以安全演进并接受审计.

**Status:** passed；**Score:** 139/139（138 VERIFIED + 1 PASSED (override)）；**Re-verification:** 是。本轮仅处置03-01.1的显式授权决议；前轮完整合同与真实行为证据保持。human_needed旧报告已按字节快照，不以本轮授权补造过去的安装前批准。

## User Flow Coverage

| 用户步骤 | 预期结果 | 当前证据 | 判断 |
|---|---|---|---|
| 管理员从新checkout执行reload、scope plan、dev-token | 创建store/目录与强类型表；配置可认证 | 03-23、03-04、03-12的CLI具名测试，当前双平台日志 | VERIFIED |
| 管理员变更定义与角色 | 失败保旧、破坏性变更需migrate、scope不合法不生效 | 03-03/10/14/15/16/18/22；PG migrate/reload与角色负测 | VERIFIED |
| 提交patch并查询历史 | 单调版本、append-only日志、按当前scope遮蔽 | 03-08/09/13/17/21；shared conformance与独立model | VERIFIED |
| 调用者以OIDC或dev身份读取 | CEL SQL/Rust一致；多role列行不泄露 | 03-05/07/11/12，当前Trino差分和认证测试 | VERIFIED |
| Outcome：安全演进且可审计 | 上述结果真实持久化并遵守权限 | 核心源码、CLI及真实PG/Trino执行支持用户故事；唯一历史流程WARNING已由当前用户明确接受证据边界并授权单项override | VERIFIED（运行能力）；报告passed（含1项override，非历史批准证明） |

## Evidence Provenance

- 当前HEAD和最终CI的headSha均为`060019fe52005c6b85ae5c5f59da9868f464e579`，run `41994479295`五个job success。身份文件为`.planning/milestones/v1.0-audit/evidence/2026-11-05/ci-41994479295-final.json`。
- 日志缩写U/W/I/S/B分别是同目录`ci-41994479295-{ubuntu,windows,integration,phase7-real-services,isolated-query-bench}.log`。表中job:line为actual `test <name> ... ok`，不包括ignored/discovery。原始文件SHA256在`report-resign-03-04/evidence-inputs.json`。
- 本worker只运行只读git、GSD query与证据采集/报告写入，没有重新执行Cargo、测试、容器或服务，没有触发CI，没有commit/push/发布/归档。现有日志允许复用，但每条truth分别匹配源码与执行。
- 完整PLAN/artifact/link/decision原生结果、源函数片段、逐条JSON及changed/env判断位于`.planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-03-04/`；详`CONTRACT-RESULTS.md`和`GOVERNANCE-AND-BOUNDARIES.md`。
- 两个MVP User Story均经`query user-story.validate`确认valid=true。既有UAT只作历史，不声称本轮再次人工操作。

## Goal Achievement

### ROADMAP Success Criteria

| ID | 原始SC合同 | 判断 | 支持的PLAN与行为 |
|---|---|---|---|
| SC1 | 加载示例 ontology 后生成每个 object type 与 M:N link type 的 edit 表；新增类定义变更在 reload 时自动执行，破坏性变更被 lint 拦下且 `ontoforge migrate --plan/--apply` 可完成并归档旧列；reload 失败时保留旧定义 | VERIFIED | 03-03/04/10/15/18/22/23：强类型表、atomic reload、真实PG migrate/reload与缓存隔离 |
| SC2 | 同一套 conformance 测试在 SQLite（Windows 默认）与 PostgreSQL 上通过；edit log 与历史表只能追加 | VERIFIED | 03-08/09/13/17：同一all_cases macro，当前PG45与SQLite conformance，append-only实际拒绝 |
| SC3 | 按属性 patch 的叠加语义（set_mask、tombstone、release、版本单调）由内存参考模型的 proptest 覆盖 | VERIFIED | 03-13/21：model_matches_store逐步比较错误/receipt/行及log replay，非production输出生成expected |
| SC4 | 角色可授予 object type、属性 allow-list、CEL 行谓词、action、SQL 级别与 toolset；行谓词只能引用同行属性与 `principal.attrs`，缺失属性按 false；多 role 合并不产生「A 的列 × B 的行」泄露 | VERIFIED | 03-05/06/11/14：Pred限制与Unknown不可见，a_columns_times_b_rows_do_not_leak、scope merge property |
| SC5 | 同一 CEL 行谓词的 SQL 编译结果与 Rust 单行求值经差分测试一致；criteria 列覆盖 lint 在 reload 与 role 变更时生效；OIDC JWT 与 dev token 均可认证 | VERIFIED | 03-07/11/12/14/16/18/22：OIDC/JWKS/dev负测，criteria whole-object覆盖与真实Trino/Rust差分 |

### PLAN must_haves 逐项结果

每条保留原文；source/artifact/链路详对应E组，完整具名测试、源码行号和全部actual execution在`truth-results.json`。不是按task完成率推断。SC是整体验收，PLAN细化包含额外行为，因此全部列入分母；旧03的8条汇总仅作别名映射，不再代替134条合同。

| ID | Truth | 判断 | 当前执行或静态证据 |
|---|---|---|---|
| 03-01.1 | sea-query-sqlx 与 getrandom（名称来源 [ASSUMED]）只在人工核验 crates.io / docs.rs 页面通过后才写入根 `Cargo.toml`；未核验前本 plan 不执行任何 install | PASSED (override) | Override: 当前会话用户明确接受03-01.1安装前人工批准缺独立留档的历史证据边界并授权单项override；未提供或找到新历史批准记录，本轮授权不是过去安装前批准，不豁免其他合同、行为或安全测试。 accepted by 当前会话用户（项目/需求负责人） on 2026-11-05T11:26:39.587Z；唯一授权输入：.planning/milestones/v1.0-audit/evidence/2026-11-05/phase03-approval-override/authorization.md。独立历史记录仍缺失；E03-01 |
| 03-01.2 | 根 `Cargo.toml` 一次性声明本 phase 全部新依赖与 feature（sqlx 增 postgres / migrate / macros / json / chrono / tls-rustls-aws-lc-rs，sea-query 1.0.2，sea-query-sqlx 0.9.1，jsonwebtoken 11.1.0 只开 aws_lc_rs，getrandom 0.4.3，chrono 增 clock，postgresql_embedded 0.21.0 关闭默认 feature）；03-02..03-20 不再修改任何 Cargo.toml 与 Cargo.lock（它们的 cargo 命令一律带 `--locked`） | VERIFIED | Cargo.toml/Cargo.lock 锁定既定版本与 feature；changed-source-inspection.json 的 phase03ManifestHistory exit=0 且 stdout 为空，e6d4823..1df4a7f 的受管 manifests 未改变；当前 CI cargo 命令含 --locked。不能据此补出 01.1 的人工批准。；E03-01 |
| 03-01.3 | crate 依赖方向为 storage → {schema, expr, auth, engine}、auth → {schema, expr, engine}，`ontoforge-storage` 不再依赖 `ontoforge-query`，依赖图无环；修正记录在 ADR-0023 并在设计文档的依赖方向行指向它 | VERIFIED | Cargo 依赖为 storage → schema/expr/auth/engine；auth → schema/expr/engine；ADR-0023 accepted，docs/designs/trino-engine-sql-tiers.md 指向 ADR；workspace 编译成功排除依赖环。；E03-01 |
| 03-01.4 | `ontoforge_storage::store::sqlite_connect_options` 打开的每个连接都 `PRAGMA recursive_triggers = 1`，并注册与 SQLite 替身同一实现的 `ontoforge_decimal` / `ontoforge_timestamp` collation（`'1.5'` 与 `'1.50'` 在该 collation 下相等） | VERIFIED | windows:3745 engine_collations_are_registered; windows:3746 every_pooled_connection_has_recursive_triggers；E03-01 |
| 03-01.5 | jsonwebtoken 选定 aws-lc-rs 后端：HS256 签发再校验的往返不 panic；不引入第二个 crypto 栈 | VERIFIED | windows:1795 hs256_round_trip_does_not_panic；E03-01 |
| 03-01.6 | `cargo tree --workspace --features ontoforge-storage/embedded-pg -i openssl` 与 `-i native-tls` 都报「did not match any packages」；CI `test` 作业有同名守卫步骤 | VERIFIED | ci/check-tls-deps.sh 从 cargo tree 全图精确匹配包名并对查询失败 fail-closed；ci/test_tls_deps.py 六个 hermetic 反例与真实 CI helper step 均成功，见 service/tls 证据。旧 -i 报错字符串命令是已替换的守卫机制，不作为当前行为证据。；E03-01 |
| 03-02.1 | 开发者可以在属性上写 `indexed: true`（缺省 false）；IR `PropertyDef.indexed` 进入 `schemas/ontoforge-definition.schema.json`；`indexed: "yes"` 报 ONT-V06 并带 line:col（ADR-0016 索引规则的定义层输入） | VERIFIED | windows:2032 count_alias_cannot_change_indexed_fk_sample_order; windows:3036 indexed_defaults_to_false; windows:3037 indexed_must_be_boolean; windows:3038 indexed_true_is_kept; windows:3668 removing_indexed_keeps_index；E03-02 |
| 03-02.2 | aviation 示例的 `Flight.status` 声明 `indexed: true`，`ontoforge validate examples/ontologies/aviation` 仍然通过 | VERIFIED | examples/ontologies/aviation/objects.yaml 的 Flight.status.indexed=true；当前示例 CLI reload 与 schema drift/示例测试通过。；E03-02 |
| 03-02.3 | 新增诊断码 ONT-V21..V27（scope 文档、行谓词、criteria 列覆盖、物理名、破坏性变更、edit store 操作、import 数据），均为不可经 `ontoforge.toml` 关闭的 V 级；码永不复用 | VERIFIED | schema/src/diag.rs 定义 V21..V27，validate 诊断不可通过 lint 配置静默；新增 W18 不覆盖任何 V 码；config/scope/reload 的具名负测覆盖不可生效路径。；E03-02 |
| 03-02.4 | `ontoforge-server.toml` 支持 `[store]`（kind sqlite \| postgres、path 或 url_env / url_file、schema_edit / schema_sys、lock_timeout_ms、probe_timeout_ms、`[store.trino] catalog`）与 `[auth]`（`[[auth.oidc]]` 多个 issuer、`[auth.principal_attrs.<name>]` 的 claim 与类型、`[auth.dev] key_file`），未知键与非法值报 ONT-V20 并带 line:col（per D-05） | VERIFIED | windows:2216 store_postgres_with_trino_catalog; windows:2218 store_sqlite_defaults; windows:2221 unknown_key_in_store_or_auth_is_v20；E03-02 |
| 03-02.5 | PG 连接串只能经 `url_env` / `url_file` 引用，不能明文写入配置（沿用 Phase 2 D-06 / D-07） | VERIFIED | windows:2214 store_plaintext_url_is_rejected；E03-02 |
| 03-02.6 | OIDC 配置只接受非对称算法白名单，`HS*` 与 `none` 报 ONT-V20；issuer 必须是 https（回环地址可 http）、不含 `\|`、互不重复 | VERIFIED | windows:2197 oidc_issuer_rules; windows:2199 oidc_rejects_symmetric_and_none_algorithms；E03-02 |
| 03-02.7 | 主体属性类型 `AttrType`（string / integer / boolean / string_list / integer_list）定义在 `ontoforge_schema::ir::attrs`；运行期取值 `Scalar` / `AttrValue` / `Attrs` 定义在 `ontoforge_expr::rowpred::value`，`Attrs::get` 对未声明或缺失的属性返回 `AttrValue::Null`（per D-05），list 取值不得含 Null 元素 | VERIFIED | windows:2203 principal_attrs_types; windows:2434 missing_attr_is_unknown_and_invisible；E03-02 |
| 03-03.1 | `ontoforge_storage::reload::reload` 在 SQLite edit store 中为每个 object type 建一张 `of_obj_<snake>` 表、为每个 M:N link type 建一张 `of_lnk_<snake>` 表（per D-17，前缀遵守 ADR-0002 的 `of_`），并写入 `of_def_version` 行与 `of_def_state.active_def_version` | VERIFIED | windows:3650 first_reload_creates_tables; windows:3657 new_many_to_many_link_creates_a_table；E03-03 |
| 03-03.2 | edit 表的 set_mask 物理编码按 Task 1 checkpoint 的结论落地（推荐：每个非 pk 属性一个 `<col>__set` 布尔列）；系统列一律 `of_` 前缀：`of_version`、`of_deleted`、`of_released`、`of_def_version`、`of_log_seq`；link 表为 `(a_pk, b_pk, of_deleted, of_version, of_def_version, of_log_seq)`，主键 `(a_pk, b_pk)` | VERIFIED | windows:3377 ddl::tests::create_table_declares_collation_flags_and_system_columns; integration:605 every_value_type_round_trips；E03-03 |
| 03-03.3 | reload 是 `ontoforge_storage::reload::reload` 库函数（per D-15）：CLI（03-04）与 Phase 7 服务进程都只调用它；`ReloadOptions { dry_run: true }` 只返回将执行的 DDL，不建表、不写 `of_def_version` | VERIFIED | windows:3649 dry_run_changes_nothing；E03-03 |
| 03-03.4 | 同一定义连续 reload 两次，第二次不产生任何 DDL、`def_version` 不变（STORE-02 idempotency）；SQLite 上 DDL 执行可重入：先内省 `table_info` 再 ADD COLUMN，中途失败后重跑能完成 | VERIFIED | windows:3667 reload_resumes_after_partial_ddl; windows:3669 second_reload_is_noop；E03-03 |
| 03-03.5 | 新增 object type / M:N link / 可空属性在 reload 时自动生成 DDL；删属性、删 type、改属性类型（含 decimal 精度）、改 pk、新增 required 属性、link 端点变化一律判为破坏性，报 ONT-V25（指向 `ontoforge migrate --plan`）且不执行任何 DDL、不切换定义（STORE-02、orchestrator 决定 2） | VERIFIED | windows:3648 destructive_changes_are_rejected；E03-03 |
| 03-03.6 | store 层按 `Backend` / `Db` 枚举区分 SQLite 与 PostgreSQL（STORE-01 的同一份 store 代码）；本 plan 的 PG 分支返回明确的 `StoreError::Config`、不 panic，PG 实现在 03-04 | VERIFIED | integration:655 first_reload_on_pg_puts_edit_tables_in_edit_schema；E03-03 |
| 03-03.7 | storage 发往数据库的 SQL 文本只经 `sql_exit.rs`：其他 src 文件不得出现 SQL 关键字开头的字符串字面量与 `AssertSqlSafe`，也不得出现 SQLite 的替换式冲突写法（源码守卫测试） | VERIFIED | windows:3738 sql_text_only_from_sql_exit；E03-03 |
| 03-03.8 | edit store 物理格式（set_mask 编码、命名与长度上限、系统列、两种方言的取值编码与列类型、PG 两个 schema、SQLite 独立文件）经 Task 1 checkpoint 确认并记录于 ADR-0024，03-04 的 PG 方言按此实现 | VERIFIED | ADR-0024 accepted 记录 orchestrator 的 checkpoint、__set/命名/63-byte/方言/schema 决定；physical/ddl/store 当前编码对应。这里核对治理记录与实现，不把 SUMMARY 当批准。；E03-03 |
| 03-04.1 | `ontoforge reload <dir> --server-config <file>` 对 aviation 示例在 SQLite edit store 中建出 edit 表并报告 `def_version 1`（STORE-02）；CLI 只做加载 → validate + lint → 打开 store → 调用 `ontoforge_storage::reload::reload` → 输出（per D-15），validate 或 E 级 lint 错误时不打开 store、不执行 DDL | VERIFIED | windows:1485 invalid_definitions_never_open_store; windows:1488 reload_creates_tables; windows:3650 first_reload_creates_tables；E03-04 |
| 03-04.2 | `ontoforge reload --dry-run` 只打印将执行的 DDL，不建表、不写 `of_def_version`（per D-15） | VERIFIED | windows:1483 dry_run_prints_ddl_only；E03-04 |
| 03-04.3 | 退出码与诊断：成功 0；定义诊断（含库函数返回的 ONT-V25）与缺 `[store]` 的 ONT-V20 为 1；store 不可用（ONT-V26）为 3；`--format json` 输出 `{"diagnostics": [...], "result": {"defVersion", "changed", "dryRun", "ddl"}}` | VERIFIED | windows:1303 json_output_has_exactly_the_documented_fields_and_no_ansi; windows:1379 json_output_reports_the_result; windows:1436 plan_json_output; windows:1486 json_output; windows:1487 missing_store_section；E03-04 |
| 03-04.4 | PostgreSQL 后端：edit 表建在 `schema_edit`（缺省 `ontoforge_edit`），系统表与 sqlx 迁移记录在 `schema_sys`（缺省 `ontoforge_sys`）；string 列 `COLLATE "C"`；DDL 用 `IF NOT EXISTS`，中途失败后重跑能完成；同一 reload 测试在 PG 上经 `ONTOFORGE_PG_URL` 或 `--features embedded-pg` 通过（STORE-01） | VERIFIED | integration:655 first_reload_on_pg_puts_edit_tables_in_edit_schema; integration:656 additive_reload_on_pg_adds_columns；E03-04 |
| 03-04.5 | 两种方言的 DDL 由 insta 快照固定，PG 列类型与 ADR-0024 一致 | VERIFIED | windows:3377 ddl::tests::create_table_declares_collation_flags_and_system_columns；E03-04 |
| 03-04.6 | `support::pg_store()` 在两种 PG 来源都缺失时 panic，ignored 的 PG 测试不得静默通过；03-08 起的 `conformance_pg` 复用它 | VERIFIED | storage/tests/support/mod.rs 的 pg_store 对 ONTOFORGE_PG_URL 或 embedded-pg 两路均缺失时 panic；当前 integration 已设置 URL，具名 PG 测试非早退。；E03-04 |
| 03-05.1 | 行谓词只用 cel 0.15 解析（经 `ontoforge_expr::analyze` 的大栈线程），随即降为自有 `Pred` IR；求值不使用 cel 运行时（其 `null == 'a'` 为 false、`null != 'a'` 为 true，与 D-06 相反） | VERIFIED | windows:2457 rejects_everything_outside_the_subset；E03-05 |
| 03-05.2 | Rust 单行求值器实现 SQL 三值（Kleene）逻辑（per D-06）：比较任一侧为 Null → Unknown；`== null` / `!= null` 恒为二值；`&&` / `\|\|` / `!` 按 Kleene 真值表；`x in [...]`：有相等元素 → True，列表为空 → False（x 为 Null 时也为 False），x 为 Null 且列表非空 → Unknown，否则 False；`x in principal.attrs.<list>` 而该属性缺失（Null）→ Unknown；字符串匹配任一侧为 Null → Unknown；最终 Unknown 视为不可见 | VERIFIED | windows:2430 in_empty_list_is_false_even_for_null; windows:2432 in_literal_list_is_three_valued; windows:2433 kleene_laws；E03-05 |
| 03-05.3 | 因此 `object.x != 'a'` 在 x 为 null 时不可见；`object.region == null \|\| object.region == principal.attrs.region` 让未分配区域的对象可见——这一与标准 CEL 不同的语义写入 `docs/scope.md` 与 ADR-0025（per D-06） | VERIFIED | windows:2428 explicit_null_branch_visible; windows:2435 null_column_not_equal_is_unknown；E03-05 |
| 03-05.4 | 允许的子集（per D-07）：`==` `!=` `<` `<=` `>` `>=`、`&&` `\|\|` `!`、`== null` / `!= null`、`in`（右侧为列表字面量或 list 型 `principal.attrs`）、`startsWith` / `endsWith` / `contains`（目标必须是 string 属性）；拒绝宏（all / exists / exists_one / map / filter）、`has()`、`matches`、算术、`size`、三元、bytes / uint 字面量与其他函数，各给出带表达式内 line:col 的 `PredIssue` | VERIFIED | windows:2450 accepts_string_matching; windows:2457 rejects_everything_outside_the_subset；E03-05 |
| 03-05.5 | 同行限制（ADR-0018 / D12）：自由变量根只允许 `object` 与 `principal.attrs`；`object.<name>` 必须是该 type 的现存属性；`principal.attrs.<name>` 必须在 server config 声明（D-05）；`principal.id` / `principal.groups` 本 phase 不开放 | VERIFIED | windows:2440 unknown_root_rejected；E03-05 |
| 03-05.6 | 类型检查比 `infer.rs` 更严：比较两侧必须同族（bool / 整数族 / double 族 / string），int 与 double 不得混比；`<` `<=` `>` `>=` 只用于 string 与数值；date / timestamp / decimal / array / geopoint 属性不得出现在比较、`in` 与字符串匹配中（`== null` / `!= null` 除 geopoint 外均可）；列表字面量元素同族且不含 null | VERIFIED | windows:2436 ordering_follows_value_families; windows:2457 rejects_everything_outside_the_subset；E03-05 |
| 03-05.7 | 缺失的主体属性在求值时为 Null（per D-05），fail-closed 来自「Unknown 不可见」而不是对缺失属性特判（SCOPE-05） | VERIFIED | windows:2434 missing_attr_is_unknown_and_invisible；E03-05 |
| 03-05.8 | `Pred` 的 Display 输出可再次解析的规范 CEL 文本，`compile(display(p)) == p` 由 proptest 覆盖 | VERIFIED | windows:2427 display_round_trip；E03-05 |
| 03-06.1 | 管理员可以在独立的 scope 目录（不与 ontology 定义或 `SourceBinding` 混放）用 YAML / JSON 写 `kind: Role` 与 `kind: RoleBinding`（per D-01），`ontoforge_schema::loader::load_scope_dir` 把目录合并为 `ScopeSet` IR，错误带文件名与 line:col | VERIFIED | windows:3080 loads_aviation_scope_example; windows:3085 scope_diagnostics；E03-06 |
| 03-06.2 | `Role` 的授权项字段：`objectTypes.<Type>.properties`（属性 allow-list）与可选 `rowFilter`（CEL 文本）、`actions`、`sqlLevels`（raw / admin，L0 隐含）、`l2.catalogs` / `l2.schemas`（`catalog.schema`）、`toolsets`（core / full / sql:raw，ADR-0019）；全部 camelCase + `deny_unknown_fields`（ADR-0012，SCOPE-02） | VERIFIED | windows:3089 toolset_value_is_checked; windows:3090 unknown_field_is_v04_with_location；E03-06 |
| 03-06.3 | `RoleBinding` 把一个 role 绑定到 subjects：`user: "<iss>\|<sub>"`（principal_id，per D-04）或 `group: <name>`（per D-02，组成员关系不落库，来自 JWT `groups` claim）；subjects 为空、同一 binding 内 subject 重复、user 不是恰好一个 `\|` 分隔的非空两段、binding 指向未定义的 role → ONT-V21；同名 role 或同一 role 的两个 RoleBinding 文档 → ONT-V08 | VERIFIED | windows:3077 binding_to_unknown_role_is_v21; windows:3078 duplicate_role_is_v08; windows:3086 second_binding_for_role_is_v08; windows:3088 subject_rules；E03-06 |
| 03-06.4 | ontology 目录中出现 Role / RoleBinding、scope 目录中出现 ObjectType / LinkType / ActionType / SourceBinding → ONT-V03，help 指出应放的目录 | VERIFIED | windows:3082 object_type_in_scope_dir_is_v03; windows:3084 role_in_ontology_dir_is_v03; windows:3087 source_binding_in_scope_dir_is_v03；E03-06 |
| 03-06.5 | `ontoforge schema export` 导出第三个文件 `ontoforge-scope.schema.json`；示例 scope 文档通过该 schema 校验；`schemas/` 与代码一致由 drift 测试守护 | VERIFIED | windows:3060 committed_schemas_are_up_to_date; windows:3068 scope_schema_is_the_third_exported_file；E03-06 |
| 03-06.6 | 示例 `examples/scopes/aviation/` 含三个虚构 role（dispatcher、crewScheduler、maintenanceLead）与它们的 RoleBinding；dispatcher 与 crewScheduler 对 Flight 授予不同的列集与不同的行谓词，构成「A 的列 × B 的行」交叉用例 | VERIFIED | windows:1910 a_columns_times_b_rows_do_not_leak; windows:3080 loads_aviation_scope_example；E03-06 |
| 03-07.1 | 客户端持 OIDC IdP 签发的 JWT（RS256 / ES256 / EdDSA 等非对称算法）经 `Authenticator::authenticate` 认证为 `Principal`；JWKS 从 `ontoforge-server.toml` 配置的 `jwks_uri` 自动拉取并缓存（AUTH-01，ADR-0008 自写缓存） | VERIFIED | windows:1829 es256_token_becomes_principal; windows:1836 jwks_is_fetched_once_for_repeated_tokens; windows:1846 rs256_token_becomes_principal；E03-07 |
| 03-07.2 | `Principal.id` = `"{iss}\|{sub}"`（per D-04），同时保留 issuer 与 subject；`Principal.groups` 来自配置的 `groups_claim`（per D-02，组成员关系不落库）；多个 issuer 受信时组名规范化为 `"{iss}\|{group}"`，单个 issuer 时为裸组名（orchestrator 决定 8） | VERIFIED | windows:1846 rs256_token_becomes_principal; windows:1851 two_issuers_namespace_groups；E03-07 |
| 03-07.3 | `Principal.attrs` 按 `[auth.principal_attrs]` 的 claim → 属性映射与类型生成（per D-05）；claim 缺失 → 属性缺失（求值为 Null）；claim 类型与声明不符、list claim 含 null 或异类元素 → 拒绝该令牌 | VERIFIED | windows:1840 list_claim_with_null_or_foreign_element_is_rejected; windows:1841 missing_attr_claim_is_absent; windows:1847 scalar_claim_of_the_wrong_type_is_rejected；E03-07 |
| 03-07.4 | 验证器按令牌头的 `alg` 路由，不看未验证 payload 决定算法：只接受该 issuer 配置的非对称算法白名单；`none`、`HS*` 在 OIDC 路径一律拒绝；永不跟随令牌头的 `jku` / `x5u` / `jwk`，JWKS 地址只来自配置 | VERIFIED | windows:1824 alg_none_is_rejected; windows:1825 algorithm_outside_the_issuer_allow_list_is_rejected; windows:1833 header_key_urls_are_never_followed；E03-07 |
| 03-07.5 | `iss`、`aud`、`exp`、`sub` 必填并校验，`nbf` 若存在则校验，leeway 取配置（缺省 30 s）；过期、错 issuer、错 audience、签名错、未知 issuer 都返回不含令牌内容的错误 | VERIFIED | windows:1830 expired_token_is_rejected; windows:1842 missing_required_claims_are_rejected; windows:1843 not_yet_valid_token_is_rejected; windows:1853 wrong_audience_is_rejected; windows:1854 wrong_issuer_is_rejected；E03-07 |
| 03-07.6 | JWKS 缓存：TTL 取响应 `Cache-Control: max-age` 并夹在 [60 s, 24 h]、缺省 1 h；`kid` 未命中时单飞重拉，两次重拉至少间隔 `jwks_min_refresh_secs`，未知 kid 负缓存；拉取失败时继续使用旧密钥集直到 `jwks_max_stale_secs` 后 fail-closed；响应体上限 1 MiB、请求超时 10 s | VERIFIED | windows:1827 cache_control_max_age_is_clamped; windows:1828 concurrent_kid_miss_is_single_flight; windows:1838 kid_miss_refetches_once_then_throttles; windows:1845 oversized_jwks_rejected; windows:1848 stale_keys_survive_idp_outage；E03-07 |
| 03-08.1 | 一次 `EditStore::commit` 在单个数据库事务内完成：锁定或创建 edit 行 → 计算新状态 → 写 `of_edit_log` 头行取得 `seq` → 写 `of_edit_log_entry` → 写 `of_edit_history` 快照；任一步失败整体回滚，不留下 edit 行或日志（STORE-03） | VERIFIED | integration:596 commit_appends_one_log_record; integration:597 commit_set_creates_edit_row; integration:599 commit_writes_history_snapshot; integration:607 invalid_values_are_rejected_without_writes；E03-08 |
| 03-08.2 | 一条 log 记录 = 一次成功提交（per D-11）：全局单调 `seq`、ULID `event_id`、actor（principal_id 或保留的 `system:import`）、action、提交时间（Rust 侧生成）、`def_version`、Action 参数原样 JSON；`seq` 在全部行锁取得之后才分配，因此同一 pk 的 seq 顺序与版本顺序一致 | VERIFIED | integration:596 commit_appends_one_log_record; integration:600 concurrent_commits_on_same_pk_serialize；E03-08 |
| 03-08.3 | 每条 entry 只存新值与操作类型（per D-10）：`(kind, type_name, pk, op ∈ {set, unset, tombstone, release, link_add, link_remove}, payload, version_after)`；set 的 payload 是「设置的属性 → 新值」，unset 的 payload 是属性名列表，其余为 null；不存旧值、不复制 source 值 | VERIFIED | integration:611 log_records_every_op; integration:618 second_set_patches_only_given_properties；E03-08 |
| 03-08.4 | edit 行的值按属性类型校验后写入（byte / short / integer 的范围、decimal 精度与小数位、date / timestamp 规范文本、array 元素类型、geopoint 经纬度范围），pk 不可经 set 修改；不符返回 `EditError`，不写入 | VERIFIED | integration:605 every_value_type_round_trips; integration:607 invalid_values_are_rejected_without_writes；E03-08 |
| 03-08.5 | conformance 框架：`tests/conformance/mod.rs` 的每个用例经 `all_cases!` 同时生成 SQLite 实跑目标与 PG ignored 目标（STORE-01），后续 plan 只在 `conformance/mod.rs` 中追加用例 | VERIFIED | integration:605 every_value_type_round_trips；E03-08 |
| 03-08.6 | edit log 的具体物理格式（三张表的列、取值 JSON 编码、unset payload、保留主体 `system:import`、历史快照结构）经 Task 1 checkpoint 确认后才首次写入（per D-10 / D-11，one-way）；ADR-0026 由 03-09 记录 | VERIFIED | ADR-0026 accepted 的 decider/Context 记录 03-08 checkpoint 与 S1-S4；postgres/sqlite 0002_edit_log.sql、log/history 当前列与约束对应，三张表由真实 shared conformance 验证。；E03-08 |
| 03-09.1 | edit log、entry 与历史表只能追加（per D-12）：PG 与 SQLite 都以触发器拒绝 UPDATE / DELETE，PG 另以语句级触发器拒绝 TRUNCATE，SQLite 的替换式写入在 `recursive_triggers = ON` 下同样被拒；同一 conformance 用例在两个后端通过 | VERIFIED | integration:585 append_only_rejects_sqlite_replace_writes; integration:587 append_only_rejects_truncate_on_pg; integration:588 append_only_rejects_update_and_delete；E03-09 |
| 03-09.2 | 按 seq 顺序把 edit log 重放到一个全新 store，重建出的 edit 表与原 store 逐列相等（per D-10），重放时计算出的版本与 entry 记录的 `version_after` 不一致即报错 | VERIFIED | integration:614 replay_detects_version_divergence; integration:615 replay_rebuilds_edit_store；E03-09 |
| 03-09.3 | 同一个 `Commit` 提交两次产生两条 log 记录、版本各 +1，store 层不去重（幂等键是 Phase 5 的 `command_id`）；SQLite 与 PG 结果相同（STORE-01 idempotency） | VERIFIED | integration:616 same_commit_twice_creates_two_records；E03-09 |
| 03-09.4 | ADR-0026 记录 edit log 格式（依据 03-08 Task 1 checkpoint 结论与实际落地的三张表）、seq 在行锁之后分配的理由、保留主体 `system:import`、重放语义与 append-only 的两道防线 | VERIFIED | ADR-0026 记录三张表、行锁后分配 seq、system:import、version_after 重放校验和 append-only/部署权限双边界，当前 commit/log/migration 与其一致。；E03-09 |
| 03-10.1 | reload 按 ADR-0016 的索引规则建索引：pk 为主键；声明 `indexed: true` 的属性建 `of_ix_<table>_<col>`；oneToOne link 的外键属性在 `to` 端 edit 表上建部分唯一索引 `of_ux_<table>_<col>`（条件：该列已设置且行未删除），使 1:1 基数在 edit store 写入时由数据库强制（STORE-05） | VERIFIED | windows:3646 aviation_reload_creates_indexes; windows:3658 one_to_one_unique_index_rejects_second_row；E03-10 |
| 03-10.2 | 索引对账是加法式的：新增的索引在 reload 时 `IF NOT EXISTS` 创建；普通索引从不删除（可能被 role 行谓词依赖，03-16 / 03-18）；link 从 oneToOne 放宽为 oneToMany 时删除对应唯一索引（只删索引、不丢数据）；收紧为 oneToOne 时若已有 edit 数据违反唯一性，reload 失败、保留旧定义并报 ONT-V26 | VERIFIED | windows:3662 relaxing_one_to_one_drops_unique_index; windows:3668 removing_indexed_keeps_index; windows:3674 tightening_to_one_to_one_with_conflicts_keeps_old_definition；E03-10 |
| 03-10.3 | 物理名超过 PG 63 字节限制时按 ADR-0024 截断并附 FNV-1a 哈希；截断后两个定义映射到同一表 / 列 / 索引名，或属性列与系统列（`of_*`）同名（如属性 `ofVersion`），reload 报 ONT-V24 且不执行 DDL | VERIFIED | windows:3653 long_names_round_trip; windows:3670 snake_collision_is_v24; windows:3672 system_column_collision_is_v24；E03-10 |
| 03-10.4 | diff 覆盖 link 级变化：新增 M:N link → 建表；删除 link、改端点 type → ONT-V25；decimal 精度 / 小数位或 array 元素类型变化 → ONT-V25；已归档列（`archived_columns`）不参与「删除」判断 | VERIFIED | windows:3652 link_changes_are_destructive; windows:3657 new_many_to_many_link_creates_a_table; windows:3659 precision_and_items_changes_are_destructive；E03-10 |
| 03-11.1 | 同一 `Pred` 经 `ontoforge_auth::scope::lower` 降为 printer AST、打印为 SQLite SQL 执行的三值结果，与 `ontoforge_expr::rowpred::eval` 对同一行同一主体属性的结果逐一相等；完整 proptest 只跑 Rust vs SQLite（per D-09 / D23，SCOPE-04） | VERIFIED | windows:1861 corpus_cases_match_on_sqlite; windows:1865 rust_eval_equals_sqlite；E03-11 |
| 03-11.2 | Trino 端跑固定种子语料 + 历史反例（`tests/data/rowpred_corpus.json`）：`rowpred_trino_it` 在 Linux CI 对 `demo.rowpred.rows` 逐行比较 Trino 与 Rust 的三值结果（per D-09，03-20 接入 CI） | VERIFIED | integration:1117 trino_matches_rust_on_the_corpus_cases; integration:1118 trino_matches_rust_on_seeded_predicates；E03-11 |
| 03-11.3 | 主体属性在编译期代入为字面量：缺失 → `NULL`；空列表 → 常量 FALSE（printer 拒绝空 `IN ()`，Trino 不接受）；任何未知值的比较降为 `<列> = NULL`，三值结果为 Unknown，与 Rust 一致 | VERIFIED | windows:1862 empty_list_attr_is_false_on_both_sides; windows:1864 null_column_comparison_is_unknown_on_both_sides；E03-11 |
| 03-11.4 | 字符串匹配只经 printer 的新节点 `Expr::StringMatch` 输出：Trino 方言为 `LIKE '<模式>' ESCAPE '\'`（转义 `\`、`%`、`_`），SQLite 方言为 `GLOB '<模式>'`（`[`→`[[]`、`*`→`[*]`、`?`→`[?]`）；模式文本最后仍经 `quote_string` 出口；两方言都区分大小写、通配符按字面 | VERIFIED | windows:2181 string_match_matches_rust_on_sqlite; integration:1117 trino_matches_rust_on_the_corpus_cases；E03-11 |
| 03-11.5 | 多 role 合并（ADR-0018，SCOPE-03）：`TypeScope.row = OR(R_i)`，`TypeScope.columns[x] = OR(R_i where x ∈ C_i)`；没有任何 role 授予该 type 时不构造 TypeScope（fail-closed，SCOPE-05）；行可见 = row 为 True，列 x 可见 = 行可见且 columns[x] 为 True；同一结构既可 Rust 求值也可降为 SQL，供 Phase 4 拼装「role 谓词 AND 原列过滤、遮蔽只在投影层」 | VERIFIED | windows:1914 lowered_scope_matches_evaluate_on_sqlite; windows:1915 no_grant_is_invisible；E03-11 |
| 03-11.6 | 「A 的列 × B 的行」不泄露：只满足 role A 谓词的行上，只属于 role B 列集的列不可见（专项用例 + proptest：可见列集恰为 ∪{C_i \| R_i(row) 为 True}） | VERIFIED | windows:1910 a_columns_times_b_rows_do_not_leak; windows:1918 visible_columns_are_union_of_true_grants；E03-11 |
| 03-11.7 | 同一属性被两个 role 授予时，列谓词是两 role 谓词的 OR；两 role 谓词结构相同时只出现一次（SCOPE-02 adjacency） | VERIFIED | windows:1913 identical_role_predicates_appear_once; windows:1917 same_property_from_two_roles_is_or；E03-11 |
| 03-12.1 | 开发者无需外部 IdP，用 `ontoforge dev-token --server-config <file> --sub alice --groups dispatchers --attr homeAirports=FRA,MUC` 生成本地令牌（AUTH-02）；dev token 的 groups 由 CLI 指定（per D-02） | VERIFIED | windows:1360 issues_token_verifiable_by_dev_verifier；E03-12 |
| 03-12.2 | dev token 为 HS256，`iss` 与 `aud` 固定为 `ontoforge-dev`，principal_id 为 `ontoforge-dev\|<sub>`（per D-04）；属性按 `[auth.principal_attrs]` 的 claim 名与类型写入，验证时复用与 OIDC 相同的 claims → Principal 映射（per D-05） | VERIFIED | windows:1365 scalar_attrs_are_converted_by_declared_type; windows:1803 dev_token_round_trip；E03-12 |
| 03-12.3 | 签名密钥是 32 字节 OS 随机数（getrandom），首次使用时以独占创建方式写入 `--key-file` 或 `[auth.dev] key_file`；Unix 上权限 0600，Windows 依赖用户目录 ACL（文档注明）；已有密钥文件被复用 | VERIFIED | windows:1809 key_file_is_created_once_and_reused; ubuntu:1826 key_file_permissions_unix；E03-12 |
| 03-12.4 | dev 验证器只能由调用方显式持有 `DevMode` 凭证构造；`Authenticator::from_config` 即使读到 `[auth.dev]` 段也不启用 dev 验证，此时 HS256 令牌一律被拒（Phase 7 的 `serve --dev` 是唯一预期的持有者） | VERIFIED | windows:1802 config_alone_never_enables_dev_tokens；E03-12 |
| 03-12.5 | 验证器按令牌头 `alg` 路由：HS256 → dev 路径（仅在已启用时），非对称 → OIDC 路径；HS256 令牌的 `iss` 不是 `ontoforge-dev`、签名错、过期 → 拒绝 | VERIFIED | windows:1804 expired_dev_token_rejected; windows:1806 hs256_with_other_issuer_rejected; windows:1817 wrong_key_rejected；E03-12 |
| 03-13.1 | 叠加按属性 patch（STORE-07）：edit 只存设置过的属性，未设置的属性跟随 source；unset 清除对应 `__set` 与取值；tombstone 压制 source 行且不会被 set「复活」（返回 `Tombstoned`）；release 是显式状态（清除全部 `__set`、回到纯 source），只对 source-backed type 合法；版本每次成功写入严格 +1、永不重置或减小 | VERIFIED | integration:619 source_object_lifecycle; windows:3612 model_matches_store；E03-13 |
| 03-13.2 | 状态转移表（实现与参考模型共同的规格）：不存在 + set → 创建（editOnly 须含全部 required 属性）；不存在 + tombstone → source-backed 允许（压制 source 行）、editOnly 为 `NotFound`；live + set / unset / tombstone → 允许；live + release → source-backed 允许、editOnly 为 `ReleaseOnEditOnly`；tombstoned + release → 回到纯 source（source-backed）；tombstoned + set / unset / tombstone → 拒绝；released + set → 重新 live；released + tombstone → 允许；released + unset / release → 拒绝；editOnly 的 unset 不得清除 required 属性 | VERIFIED | integration:604 edit_only_rules; integration:619 source_object_lifecycle; windows:3612 model_matches_store；E03-13 |
| 03-13.3 | M:N link 边与对象同一叠加规则（D19）：add → `of_deleted = false`，remove → `of_deleted = true`（保留行以压制 source join 行），两者可反复切换且每次版本 +1；同一边连续 add 两次只有一行 live 行、版本 2（STORE-05 idempotency） | VERIFIED | integration:601 duplicate_link_add_keeps_one_live_row; integration:609 link_add_remove_toggle；E03-13 |
| 03-13.4 | oneToOne 基数在 edit store 写入时强制（STORE-05）：两个对象把同一外键值设为已设置时，第二次提交返回 `EditError::CardinalityViolation { link }` 且整个提交回滚；oneToMany 的外键天然单值；source 侧基数不在本 phase | VERIFIED | integration:603 concurrent_fk_claims_one_wins; integration:612 one_to_one_violation_is_reported；E03-13 |
| 03-13.5 | 内存参考模型 proptest（SC3）：随机提交序列同时作用于朴素参考模型与真实 SQLite store，每步的成功 / 失败类别、全部 edit 行与 link 行逐一相等；序列结束后把日志重放到新 store 与原 store 逐列相等 | VERIFIED | windows:3612 model_matches_store；E03-13 |
| 03-13.6 | 并发（STORE-01 concurrency）：两个并发提交修改同一 pk 在 SQLite（`BEGIN IMMEDIATE`）与 PG（行锁）上都串行化，最终版本为 2，两条日志的 seq 顺序与 `version_after` 顺序一致；两个并发提交争用同一 oneToOne 外键值时恰好一个成功，失败者不留任何 edit 行或日志 | VERIFIED | integration:600 concurrent_commits_on_same_pk_serialize; integration:603 concurrent_fk_claims_one_wins；E03-13 |
| 03-14.1 | `compile_scope` 是 scope 校验与编译的同一个函数（per D-01：apply 前离线执行，不通过则整体不生效）：输入 ontology、选定 profile、`[auth.principal_attrs]` 的类型、`ScopeSet` 与可选 `SourceMap`，输出 `CompiledScope` 或全部诊断（不在第一个错误处停止） | VERIFIED | windows:1883 all_errors_reported_together; windows:1885 aviation_scope_compiles；E03-14 |
| 03-14.2 | ONT-V21：role 引用不存在的 object type / 属性 / action、属性 allow-list 不含该 type 的主键（orchestrator 决定 4）、allow-list 重复、`toolsets` 含 `sql:raw` 或 `l2` 非空而 `sqlLevels` 不含 `raw`、`l2.schemas` 不是 `catalog.schema` 形式 | VERIFIED | windows:1884 allow_list_without_primary_key_is_v21; windows:1892 l2_without_raw_level_is_v21; windows:1901 unknown_object_type_is_v21；E03-14 |
| 03-14.3 | ONT-V22：`rowFilter` 超出 D-07 子集、类型错误、引用非同行属性或未在 `ontoforge-server.toml` 声明的 `principal.attrs`（per D-05 / D-07）；或引用了在选定 profile 绑定中声明 `collation: caseInsensitive` 的列（per D-08）；诊断定位到 `rowFilter` 值并在消息中给出表达式内行列 | VERIFIED | windows:1886 bad_row_filter_is_v22_located; windows:1887 collation_case_insensitive_is_v22; windows:1899 undeclared_attr_is_v22；E03-14 |
| 03-14.4 | ONT-V23（criteria 列覆盖，SCOPE-06）：role 被授予某 action 时，该 action 每条 submission criteria 中 `objects.<param>.<prop>` 引用的属性必须在该 role 对参数所指 type 的 allow-list 内，`objects.<param>` 必须指向该 role 可见的 type；无法静态确定的动态路径一律报错（fail-closed）；诊断定位到 role 的 `actions` 条目 | VERIFIED | windows:1785 dynamic_object_path_is_v23_fail_closed; windows:1786 hidden_criteria_column_is_v23_at_the_action_entry；E03-14 |
| 03-14.5 | fail-closed（SCOPE-05）：主体没有任何 role 授予某 type 时，`PrincipalScope.types` 中没有该 type（不可见）；role 的 `objectTypes` 为空 = 该 role 不授予任何 type；省略 `rowFilter` = 该 role 对该 type 的全部行 | VERIFIED | windows:1890 empty_object_types_grants_nothing; windows:1894 principal_without_roles_sees_nothing；E03-14 |
| 03-14.6 | `CompiledScope::for_principal` 按 `RoleBinding` 解析主体的 role：`user` subject 与 `principal.id` 相等或 `group` subject 属于 `principal.groups`（D-02，多 issuer 时组名已带 issuer 前缀）；同一 role 被多条 subject 命中只计一次；types 经 03-11 的 `merge` 合并，actions / sqlLevels / l2 / toolsets 取并集 | VERIFIED | windows:1893 principal_with_two_roles_gets_merged_flight_scope; windows:1903 user_subject_binding；E03-14 |
| 03-14.7 | `CompiledScope::row_filter_properties` 给出每个 type 被行谓词引用的属性集合，供 03-16 / 03-18 的索引对账（ADR-0016「行谓词引用的属性」） | VERIFIED | windows:3664 reload_creates_row_filter_indexes；E03-14 |
| 03-15.1 | 被 reload 以 ONT-V25 拦下的破坏性变更可经 `ontoforge migrate --plan` 查看确定的步骤清单与 plan hash，再经 `ontoforge migrate --apply --plan-hash <hash>` 执行；apply 在同一事务内重算 plan，hash 不一致即拒绝且不执行任何 DDL（防 TOCTOU） | VERIFIED | windows:1433 migrate_plan_then_apply; windows:3581 apply_with_stale_hash_is_rejected；E03-15 |
| 03-15.2 | 旧列归档 = 同表改名为带 `def_version` 后缀的归档列（per D-16）：`<col>` → `<col>__arch_v<N>`、`<col>__set` → `<col>__set__arch_v<N>`，保留类型、不搬数据、读路径忽略（记录在 `PhysicalSchema.archived_columns`）；删除的 type 整表改名为 `of_obj_<x>__arch_v<N>`；改属性类型 = 归档旧列 + 新建同名空列；改 pk = 归档整表 + 新建表 | VERIFIED | windows:3582 archive_removed_property; windows:3587 primary_key_change_archives_table; windows:3589 removed_type_archived; windows:3594 type_change_archives_and_adds; integration:646 archive_removed_property_on_pg；E03-15 |
| 03-15.3 | 改名在 diff 上表现为删 + 增（per D-14）；`--rename <Type>.<old>=<new>` 与 `--rename <OldType>=<NewType>` 显式声明时把 edit 数据随列 / 表改名搬到新名下（类型必须相同，否则报错）；未声明时旧列归档、新列为空 | VERIFIED | windows:3590 rename_object_type_moves_table; windows:3591 rename_property_moves_data; windows:3595 undeclared_rename_archives_and_adds_an_empty_column；E03-15 |
| 03-15.4 | 清理归档列只经显式子命令 `ontoforge migrate --drop-archived --plan` / `--apply`（per D-16）；SQLite 先删除归档列上的索引再 `DROP COLUMN` | VERIFIED | windows:1431 drop_archived_plan_and_apply; windows:3583 drop_archived_cleans_up；E03-15 |
| 03-15.5 | migrate 与 reload 共用同一个 DDL 执行与定义切换路径（03-18 的 `lock_timeout`、advisory 锁、栅栏与 Trino 可见探测因此同时作用于 migrate）；新的 `of_def_version` 行在 `migration_json` 中记录本次执行的步骤与 rename 声明，供跨版本重放使用 | VERIFIED | windows:3656 migrate_waits_for_trino_after_the_switch；E03-15 |
| 03-15.6 | migrate 成功后对同一定义 reload 不再报 ONT-V25；对已执行的 plan 再次 `--apply --plan-hash <旧 hash>` 因 plan 已变（为空）而被拒绝，不会重复归档（STORE-02 idempotency） | VERIFIED | windows:3588 reapply_after_success_is_rejected；E03-15 |
| 03-16.1 | 管理员用 `ontoforge scope plan <scope-dir> --ontology <dir> --server-config <file>` 查看将要发生的 role 增 / 改 / 删与绑定增 / 删（含将删除的绑定，per D-02 全量同步）及 plan hash，再用 `ontoforge scope apply ... --plan-hash <hash>` 写入系统库（per D-01，SCOPE-01） | VERIFIED | windows:1512 plan_then_apply_aviation_scope; windows:3722 full_sync_removes_bindings_and_roles；E03-16 |
| 03-16.2 | apply 之前离线执行 03-14 的 `compile_scope`（行谓词校验与 criteria 列覆盖，per D-01 / SCOPE-06）；任一诊断 → 退出码 1，系统库不发生任何写入；ontology 与 edit store 当前生效定义不一致时同样拒绝（提示先 reload） | VERIFIED | windows:1507 invalid_scope_writes_nothing; windows:1510 ontology_differing_from_the_active_definition_is_refused；E03-16 |
| 03-16.3 | apply 在单个事务内：重算 plan 并比对 hash → 写 `of_role`（规范化 IR JSON + 内容 hash）、`of_role_binding` → 为行谓词引用的属性建 `of_ix_<table>_<col>` 索引（只增不删，orchestrator 决定 8）→ 追加一条 `of_admin_audit`（actor = OS 用户 @ 主机、plan JSON、前后 hash）；`of_admin_audit` 与 edit log 一样只能追加（触发器） | VERIFIED | windows:3718 apply_creates_row_filter_indexes; windows:3720 apply_writes_roles_bindings_and_audit; windows:3721 audit_insert_failure_rolls_back_scope_and_indexes；E03-16 |
| 03-16.4 | apply 是系统库中 role / 绑定的唯一写入口（per D-03，SCOPE-07）：storage 中只有 `scope_store.rs` 写 `of_role` / `of_role_binding`，源码守卫测试强制；Phase 7 的 REST 管理端对 role 只读 | VERIFIED | windows:3730 scope_tables_written_only_by_scope_store；E03-16 |
| 03-16.5 | 首次认证的主体经 `EditStore::record_login(&Principal)` 自动建 user 记录，只存 principal_id、issuer、subject、创建时间与最近登录时间，不存任何权限或 claim（per D-04）；同一主体的最近登录时间在进程内至多 60 秒写一次 | VERIFIED | windows:3724 login_recorder_writes_once_per_interval; windows:3729 record_login_creates_user_once；E03-16 |
| 03-16.6 | plan 输出与内容 hash 与 YAML 中的书写顺序无关：role 按 apiName、绑定按 (role, kind, subject) 排序；IR 中的列表字段（properties、actions、sqlLevels、toolsets、l2）在存储与哈希前排序去重（SCOPE-02 ordering） | VERIFIED | windows:3725 order_independent_hash；E03-16 |
| 03-16.7 | `docs/scope.md` 增「Role 与 RoleBinding」一节：目录约定、字段、plan / apply、全量同步、审计、组来自 JWT `groups` claim（多 issuer 时带 issuer 前缀）、dev token 的组、REST 只读 | VERIFIED | docs/scope.md 的 Role/RoleBinding、目录、全量同步、审计、issuer groups、dev token、REST只读均对应 scope_store/CLI 和 auth claims 实现。；E03-16 |
| 03-17.1 | 客户端（经 Phase 7 外壳）可以查询某个对象的变更历史：`EditStore::history` 按 seq 从新到旧返回该对象的每次变更（seq、event_id、actor、action、提交时间、操作类型、变更后版本），支持 `limit`（≤ 500）与 `before_seq` 游标（STORE-04；本 phase 交付库级 API） | VERIFIED | windows:3543 pagination_keeps_entries_of_one_commit_together; windows:3544 pagination_with_before_seq；E03-17 |
| 03-17.2 | 历史按当前 scope 遮蔽（per D-13）：调用者须对该对象**当前**可见；不可见、不存在、调用者对该 type 无任何授权三种情况返回同一个 `HistoryError::NotFound`（Display 相同） | VERIFIED | windows:3538 invisible_check_precedes_store_access; windows:3539 invisible_nonexistent_and_ungranted_are_identical；E03-17 |
| 03-17.3 | 每条历史只返回当前可见列的值，可见列由当前行经同一 `TypeScope::evaluate` 求出（不是历史行）；其余被改动的列只出现在 `masked_changes` 中、不给值；序列化后的响应不含任何被遮蔽列的值 | VERIFIED | windows:3547 serialized_history_never_contains_masked_values; windows:3549 visibility_uses_current_row；E03-17 |
| 03-17.4 | Action 原始参数只对管理通道（`CallerKind::Admin`）返回，普通调用者的每条历史 `params` 为 None（per D-13） | VERIFIED | windows:3545 params_hidden_from_ordinary_caller；E03-17 |
| 03-18.1 | `def_version` 栅栏（ADR-0016 / D20）：每次提交在写事务开始时读取 `of_def_state.active_def_version`（PG `FOR SHARE`），与 `Commit.def_version` 不等即 `DefinitionStale` 并整体回滚；reload / migrate 的切换 `UPDATE of_def_state` 等待在途写事务结束，之后旧定义版本的写入一律被拒（SCOPE-07「每个请求固定使用开始时的定义版本」的写侧） | VERIFIED | windows:3671 stale_writer_rejected_after_switch; integration:659 in_flight_commit_blocks_switch_on_pg; integration:665 reload_racing_commits_never_lets_an_old_write_through_on_pg；E03-18 |
| 03-18.2 | reload 原子生效（STORE-02）：DDL 事务前置 `SET LOCAL lock_timeout`（取 `[store] lock_timeout_ms`）与 `pg_advisory_xact_lock`（与 migrate、scope apply 共用同一把锁）；锁等待超时（SQLSTATE 55P03）→ `ReloadError::LockTimeout`、保留旧定义（CLI 输出 ONT-V26、退出码 3） | VERIFIED | integration:658 definition_lock_is_shared_and_bounded_by_lock_timeout; integration:663 lock_timeout_keeps_old_definition；E03-18 |
| 03-18.3 | 两个 reload 并发时单飞：第二个等待第一个完成后看到新的生效定义，diff 为空、不重复建表、`def_version` 只增加一次；SQLite 上由 `BEGIN IMMEDIATE` 保证（STORE-02 concurrency） | VERIFIED | windows:3647 concurrent_reloads_single_flight; integration:661 concurrent_reloads_single_flight_on_pg；E03-18 |
| 03-18.4 | reload 时对系统库中已 apply 的 role 重跑 `compile_scope`（行谓词校验与 criteria 列覆盖，per D-18 / SCOPE-06）；任一失败则整体不切换、不执行任何 DDL，保留旧定义，诊断指明 role 与 type；通过时为行谓词引用的属性对账索引（只增不删） | VERIFIED | windows:3664 reload_creates_row_filter_indexes; windows:3666 reload_rechecks_applied_roles；E03-18 |
| 03-18.5 | 配置了 Trino 的 PG edit store（profile 引擎为 trino 且 `[store.trino] catalog` 存在）：DDL 提交后定义版本先处于 `pending`，reload 执行 `CALL <catalog>.system.flush_metadata_cache()`，再用 `Engine::describe_tables` 轮询直到新表与新列（数组列除外）对 Trino 可见才切换为 `active`；在 `probe_timeout_ms` 内不可见 → `ReloadError::NotVisible`，不切换（已提交的 DDL 是加法、对旧定义无害，重跑幂等） | VERIFIED | integration:664 pending_switch_on_pg_serves_the_old_definition_until_visible; integration:1144 cached_column_requires_flush_before_definition_switch；E03-18 |
| 03-18.6 | Trino 对 `CALL` 返回的无列结果被 client 正确处理为空的 `QueryOutput`（wiremock 测试）；`CALL` 文本只由 printer 生成 | VERIFIED | windows:1987 printer::tests::flush_metadata_cache_is_a_call_of_the_catalog_procedure; windows:1988 printer::tests::flush_metadata_cache_quotes_the_catalog; windows:1989 printer::tests::flush_metadata_cache_rejects_an_empty_catalog; windows:2272 call_without_columns_returns_empty_output；E03-18 |
| 03-18.7 | migrate 走与 reload 相同的锁、栅栏、探测与切换路径（同一组测试断言 migrate 也受 `lock_timeout` 与探测约束） | VERIFIED | windows:3656 migrate_waits_for_trino_after_the_switch；E03-18 |
| 03-19.1 | 开发者可以用 `ontoforge import <ontology-dir> --server-config <file> --type <ObjectType> --file <csv\|json>` 导入 editOnly object type 的对象，或用 `--link <linkType>` 导入 editOnly M:N link 的边（STORE-06，orchestrator 决定 3）；替身引擎数据仍由 Phase 2 的绑定 fixture 提供 | VERIFIED | windows:1377 imports_airports_csv; windows:3560 editonly_link_import；E03-19 |
| 03-19.2 | 导入作为系统 edit 记入 edit log（per D-11）：actor 与 action 都是保留主体 `system:import`，params 记录文件名、批次序号与行数；每批（缺省 1 000 行、`--batch-size` 1..=10 000）一次提交，`def_version` 取当前生效版本 | VERIFIED | windows:3557 airports_csv_is_imported_as_system_edits; windows:3559 batches_follow_batch_size；E03-19 |
| 03-19.3 | 整个文件先完整解析与校验，再写入：类型转换（与 edit store 取值规则一致，decimal / date 复用 engine fixture 的公开转换函数）、required 属性、文件内主键重复、未知列、行数上限（缺省 1 000 000，`--max-rows`）任一失败 → ONT-V27（带文件名与行号）且不写入任何行 | VERIFIED | windows:3563 file_level_validation; windows:3566 json_decimals_are_never_rounded；E03-19 |
| 03-19.4 | 已存在对象的处理显式选择：`--on-existing error`（缺省，create-only：任何已存在的 edit 对象 → ONT-V27 列出主键、不写入）、`skip`（跳过已存在者，可用于中断后续跑）、`update`（对已存在者执行 set）；tombstoned 对象在任何模式下都不被复活 | VERIFIED | windows:3562 existing_objects_block_create_only; windows:3574 tombstoned_object_is_never_revived；E03-19 |
| 03-19.5 | 跨批次不是单事务：某一批失败时之前的批次已提交，CLI 报告已提交的批次与失败位置；以 `--on-existing skip` 重跑可续完（导入二次执行的结果确定） | VERIFIED | windows:1380 link_import_and_partial_failure; windows:3559 batches_follow_batch_size；E03-19 |
| 03-20.1 | PR 的 Linux `integration` 作业在真实 PostgreSQL 17 上运行 edit store 的全部 PG 用例（`conformance_pg`、`reload_pg`、`scope_store` 的 ignored 用例），与 Windows / Ubuntu `test` 作业的 SQLite 用例是同一套 conformance（STORE-01、ROADMAP SC2） | VERIFIED | integration:605 every_value_type_round_trips; integration:663 lock_timeout_keeps_old_definition; integration:672 pg_scope_apply；E03-20 |
| 03-20.2 | 同一作业对 Trino 483 运行行谓词固定语料（`rowpred_trino_it`，per D-09）与 reload 可见探测（`reload_trino_it`，STORE-02）；`ci/pg/rowpred_corpus.sql` 在 `demo` 库中建立语料表 | VERIFIED | integration:1117 trino_matches_rust_on_the_corpus_cases; integration:1144 cached_column_requires_flush_before_definition_switch；E03-20 |
| 03-20.3 | 「ignored 测试确实存在」守卫覆盖全部新增目标（每个目标 `--ignored --list` 计数 > 0），绿色结果不可能意味着什么都没跑 | VERIFIED | integration 原始日志 I:1324/1327/1330/1333/1336/1339/1342/1345/1348/1351 给出非零 discovery；同一 target 另有 actual test ok 与 summary，不把 --list 当执行。；E03-20 |
| 03-20.4 | `ci/trino/catalog/editstore.properties` 设置 `postgresql.array-mapping=AS_ARRAY`，使 edit 表的数组列对 Trino 可见（Pitfall 7） | VERIFIED | ci/trino/catalog/editstore.properties 保留 postgresql.array-mapping=AS_ARRAY 和 metadata.cache-ttl=10m/metadata.cache-missing=true；当前缓存隔离具名测试 I:1144 通过。；E03-20 |
| 03-20.5 | `test` 作业额外以 `--features embedded-pg` 跑一次 storage 的 clippy，防止本机 PG 测试路径失修；默认 `cargo test --workspace` 仍只依赖 SQLite（Windows 本机零外部依赖） | VERIFIED | ci.yml test 双平台的 clippy --features embedded-pg step 在最终 CI 均成功；workspace 默认运行的 PG/Trino cases 仍为 ignored，不计入这些后端通过数。；E03-20 |
| 03-20.6 | `docs/edit-store.md` 写明部署要求：PG 两个 schema 与账号最小权限（服务进程对日志 / 历史 / 审计表只有 INSERT 与 SELECT，REVOKE UPDATE / DELETE / TRUNCATE；触发器对表所有者不是安全边界）、Trino catalog 只读 edit schema 且不开数据 / 结果缓存、数组映射、reload / migrate / scope / import / dev-token 的用法与退出码、`lock_timeout_ms` 与 `probe_timeout_ms` | VERIFIED | docs/edit-store.md 保留 PG schema/minimum privileges/REVOKE 与 owner trigger 非安全边界、Trino 只读与缓存、数组、各 CLI 用法/退出码和 timeout。当前 Iceberg/MinIO 文档变化属后继合同，不改 Phase03 表格式。；E03-20 |
| 03-21.1 | decimal 属性收到带小数的 JSON 数字时以 ValueError 拒绝，提示把 decimal 写成字符串（如 "12.50"），不再经 f64 舍入；整数 JSON 数字（serde_json 的 is_i64 / is_u64）与十进制字符串按原文进入 edit 行、edit log 与历史快照（gap 1，CR-A01，STORE-03 / STORE-06，D-10） | VERIFIED | windows:3414 value::tests::decimal_json_numbers_must_be_exact_integers; windows:3566 json_decimals_are_never_rounded；E03-21 |
| 03-21.2 | 17 位以上有效数字的 decimal（12345678.1234567891，decimal(38, 10)）经 commit 写入后，在 SQLite 与 PostgreSQL 上 edit 行、日志 payload、下一次提交的历史快照三处逐字相同（共享 conformance 用例 every_value_type_round_trips） | VERIFIED | integration:605 every_value_type_round_trips；E03-21 |
| 03-21.3 | `ontoforge import` 的 JSON 文件里带小数的 decimal 数字报 ONT-V27、整个文件不写入；同一个值写成字符串时逐字保存；20 位整数 JSON 数字原样保存（import.rs 第 763 行的属性路径） | VERIFIED | windows:3566 json_decimals_are_never_rounded；E03-21 |
| 03-21.4 | 含 U+0000 的字符串（属性值、字符串主键、数组元素）、年份 0000 的 date、换算到 UTC 后不在 0001..9999 年内的 timestamp、含 U+0000 的 Action 参数字符串或键、action 名与 actor，都在校验阶段以 EditError::Value 拒绝且不写入；SQLite 与 PostgreSQL 结论相同，PostgreSQL 不再在校验通过之后以 StoreError 失败（WR-A01） | VERIFIED | integration:623 values_postgres_cannot_store_are_refused_on_every_backend；E03-21 |
| 03-21.5 | `open_postgres` 使用的连接选项（`pg_connect_options`）在 search_path 旁固定 DateStyle、TimeZone 与 extra_float_digits；即使数据库级缺省为 DateStyle 'SQL, DMY'、extra_float_digits 0、TimeZone 'America/New_York'，会话上 `SHOW` 读到的也是 `ISO, MDY`、`3`、`UTC`（WR-A03） | VERIFIED | integration:622 pg_sessions_use_the_pinned_settings；E03-21 |
| 03-22.1 | `ontoforge migrate --apply` 在定义锁内、执行任何迁移步骤之前，按新定义重新编译 `scope apply` 存下的 role；任一 role 不再成立（如 allow-list 引用了被删除的属性）时以 ONT-V21 / V22 / V23 拒绝、退出码 1，定义不切换、edit 表不变（gap 3a，WR-B07，D-18 / SCOPE-06） | VERIFIED | windows:1434 migrate_refuses_a_change_that_breaks_an_applied_role; integration:648 pg_migrate_rechecks_applied_roles；E03-22 |
| 03-22.2 | 同一迁移在不做 role 复核时可以应用，说明拒绝来自 role 复核；与 role 无关的破坏性变更在带 role 复核时照常应用 | VERIFIED | windows:3586 migrate_with_roles_applies_when_the_roles_still_compile；E03-22 |
| 03-22.3 | SQLite 与嵌入式 PostgreSQL 上行为一致：被拒绝的迁移不留下归档列，active def_version 不变 | VERIFIED | integration:648 pg_migrate_rechecks_applied_roles；E03-22 |
| 03-22.4 | 提交条件整体读取对象参数（`objects.<param>`，不带属性，如 `objects.flight != null`、`objects.flight == params.x`）时，视为读取该类型的全部属性：role 的 allow-list 不含该类型的全部属性就报 ONT-V23；scope plan / apply、reload 与 migrate 的复核都经同一个 compile_scope 生效（gap 3b，WR-C05，D-01） | VERIFIED | windows:1788 whole_object_read_needs_every_property；E03-22 |
| 03-23.1 | 在没有 `target/ontoforge-dev/` 的全新检出中，按 `examples/server/ontoforge-server.dev.toml` 运行 `ontoforge reload examples/ontologies/aviation` 退出码 0，并创建 `target/ontoforge-dev/editstore.db`；随后 `ontoforge scope plan` 也成功（gap 2，CR-D01，SC1 第一步） | VERIFIED | windows:1484 example_config_reload_works_from_a_fresh_tree；E03-23 |
| 03-23.2 | 在同样的全新检出中，`ontoforge dev-token --server-config examples/server/ontoforge-server.dev.toml --sub mia` 退出码 0，stdout 只有一行 token，并创建 32 字节的 `target/ontoforge-dev/dev-token.key`（gap 2，AUTH-02） | VERIFIED | windows:1358 example_config_dev_token_works_from_a_fresh_tree；E03-23 |
| 03-23.3 | SQLite edit store 打开时创建 `[store] path` 缺少的上级目录；dev 密钥首次创建时创建缺少的上级目录，Unix 上新建目录的权限为 0700。参数被拒绝时仍不留下任何文件 | VERIFIED | windows:1364 rejected_arguments_leave_no_key_file_behind; windows:1812 missing_key_directory_is_created; windows:3748 open_creates_the_parent_directories_of_the_sqlite_file；E03-23 |
| 03-23.4 | 示例配置头注释与 docs/edit-store.md 如实描述首次使用时的行为；代码注释与文档都不再声称 Windows 上的密钥文件只有本人可读，并把这一限制记为已知缺口（WR-C01 / WR-D01，本 plan 不改 ACL） | VERIFIED | examples/server/ontoforge-server.dev.toml 与 docs/edit-store.md 说明自动创建父目录及 Windows 用户目录 ACL 限制；不声称本轮修复 Windows ACL。；E03-23 |

**Score:** 139/139 = 138 VERIFIED（133条PLAN + 5条SC）+ 1 PASSED (override)；behavior_unverified=0，overrides_applied=1。03-01.1不伪装成历史批准已证实或行为已测；其余138项证据及分数未变化。旧fresh-tree/decimal/cache三条closure分别由03-23、03-21、03-18.5纳入，未丢弃。

### Required Artifacts / Key Links

原生检查在`03-contracts.json`。89/89 artifact pattern通过；54/55 link pattern通过。03-03 ddl→sql_exit直接pattern未命中，但真实链为ddl返回DDL→reload.execute_steps/apply_step→sql_exit::execute_ddl，已读实际调用与sql_exit_guard；不把pattern当缺实现。

| E组 | 实现与接线 | 判断 |
|---|---|---|
| E03-01 | 依赖、SQLite连接选项与供应链守卫；历史批准独立留档缺失仍为事实，本轮明确决议单独判 PASSED (override)。 Cargo.toml；crates/ontoforge-storage/src/store.rs；.planning/adr/0023-phase3-crate-dependency-direction.md | substantive / WIRED |
| E03-02 | Object IR/schema 与 server config 校验；ONTV20/21..27 未被新 W18 替换。 crates/ontoforge-schema/src/ir/object.rs；crates/ontoforge-schema/src/ir/attrs.rs；crates/ontoforge-expr/src/rowpred/value.rs；crates/ontoforge-engine/src/config.rs；crates/ontoforge-schema/src/diag.rs | substantive / WIRED |
| E03-03 | physical→diff→reload.execute_steps→sql_exit；dry-run/破坏性拒绝/幂等保持旧定义。 crates/ontoforge-storage/src/physical.rs；crates/ontoforge-storage/src/diff.rs；crates/ontoforge-storage/src/reload.rs；crates/ontoforge-storage/src/sql_exit.rs；crates/ontoforge-storage/tests/sql_exit_guard.rs；.planning/adr/0024-edit-store-physical-format.md | substantive / WIRED |
| E03-04 | CLI加载校验后打开store并调用reload；PG双schema、DDL快照与缺env失败路径。 crates/ontoforge/src/reload.rs；crates/ontoforge-storage/migrations/postgres/0001_definitions.sql；crates/ontoforge-storage/tests/support/mod.rs；crates/ontoforge-storage/tests/reload_pg.rs | substantive / WIRED |
| E03-05 | CEL只作解析，自有Pred求值，SQL三值语义与受限引用；Display roundtrip property。 crates/ontoforge-expr/src/rowpred/ir.rs；crates/ontoforge-expr/src/rowpred/parse.rs；crates/ontoforge-expr/src/rowpred/eval.rs；docs/scope.md；.planning/adr/0025-row-predicate-semantics.md | substantive / WIRED |
| E03-06 | 独立scope loader→ScopeSet/Role/RoleBinding；schema第三导出与示例授权交叉。 crates/ontoforge-schema/src/ir/scope.rs；crates/ontoforge-schema/src/loader/scope.rs；schemas/ontoforge-scope.schema.json；examples/scopes/aviation/roles.yaml | substantive / WIRED |
| E03-07 | OIDC header算法白名单、claim验证、JWKS限额/单飞/缓存与issuer组命名。 crates/ontoforge-auth/src/authn/mod.rs；crates/ontoforge-auth/src/authn/principal.rs；crates/ontoforge-auth/src/authn/jwks.rs；crates/ontoforge-auth/tests/oidc.rs | substantive / WIRED |
| E03-08 | commit校验→单事务锁行→seq→行/entry/history→commit；任一步错误rollback。 crates/ontoforge-storage/src/commit.rs；crates/ontoforge-storage/src/log.rs；crates/ontoforge-storage/src/value.rs；crates/ontoforge-storage/migrations/postgres/0002_edit_log.sql；crates/ontoforge-storage/tests/conformance/mod.rs | substantive / WIRED |
| E03-09 | 0002迁移触发器→append-only；read_log→replay，version_after拒绝不一致。 crates/ontoforge-storage/migrations/postgres/0002_edit_log.sql；crates/ontoforge-storage/migrations/sqlite/0002_edit_log.sql；crates/ontoforge-storage/src/log.rs；.planning/adr/0026-edit-log-format.md | substantive / WIRED |
| E03-10 | physical/naming/diff/index→reload；部分唯一索引、只增索引和放宽基数删除unique。 crates/ontoforge-storage/src/naming.rs；crates/ontoforge-storage/src/physical.rs；crates/ontoforge-storage/src/ddl.rs | substantive / WIRED |
| E03-11 | Pred→scope.lower→typed printer→SQLite/Trino，与Rust eval逐行比；per-column role OR。 crates/ontoforge-engine/src/printer/ast.rs；crates/ontoforge-engine/src/printer/escape.rs；crates/ontoforge-auth/src/scope/lower.rs；crates/ontoforge-auth/src/scope/merge.rs；crates/ontoforge-auth/tests/rowpred_diff.rs；ci/pg/rowpred_corpus.sql | substantive / WIRED |
| E03-12 | CLI DevKey→HS256→显式DevMode；config本身不启用；Unix权限与Windows限制区分。 crates/ontoforge-auth/src/authn/dev.rs；crates/ontoforge/src/dev_token.rs | substantive / WIRED |
| E03-13 | state→commit overlay转换，与独立Model.apply逐步值/错误/receipt/log replay比较；双后端并发。 crates/ontoforge-storage/src/state.rs；crates/ontoforge-storage/tests/model/mod.rs；crates/ontoforge-storage/tests/model_prop.rs | substantive / WIRED |
| E03-14 | compile_scope共用完整诊断；whole-object criteria覆盖、fail-closed与selected profile约束。 crates/ontoforge-auth/src/scope/compile.rs；crates/ontoforge-auth/src/scope/coverage.rs；crates/ontoforge-auth/tests/criteria_coverage.rs | substantive / WIRED |
| E03-15 | CLI plan hash→锁内重算/角色复核→归档/rename→共用reload探测发布；拒绝旧hash。 crates/ontoforge-storage/src/migrate.rs；crates/ontoforge/src/migrate.rs；crates/ontoforge-storage/tests/migrate.rs | substantive / WIRED |
| E03-16 | scope plan/apply→scope_store单事务/元数据锁/索引/append audit；login去节流。 crates/ontoforge-storage/src/scope_store.rs；crates/ontoforge-storage/src/index.rs；crates/ontoforge-storage/migrations/postgres/0003_scope.sql；crates/ontoforge/src/scope.rs；examples/server/ontoforge-server.dev.toml | substantive / WIRED |
| E03-17 | history_with_snapshot先visible_columns，再真实entry/history查询，按当前可见列mask；Admin才取params。 crates/ontoforge-storage/src/history.rs；crates/ontoforge-storage/tests/history_masking.rs | substantive / WIRED |
| E03-18 | commit def fence；reload/migrate/scope同定义锁；pending/flush/describe/publish与旧版本服务。 crates/ontoforge-storage/src/reload.rs；crates/ontoforge-engine/src/printer/call.rs；crates/ontoforge-storage/tests/reload_trino_it.rs | substantive / WIRED |
| E03-19 | import完整文件校验→editOnly/create-only→分批commit/system:import，跨批非原子明确报告。 crates/ontoforge-storage/src/import.rs；crates/ontoforge/src/import.rs；examples/data/aviation/airports.csv | substantive / WIRED |
| E03-20 | ci.yml→query-services PG/Trino→实际ignored执行与discovery→deployment docs；不把全局绿作为证据。 .github/workflows/ci.yml；docs/edit-store.md；ci/trino/catalog/editstore.properties | substantive / WIRED |
| E03-21 | decimal保真与PG可表示域/固定会话设置，否定JSON浮点舍入。 crates/ontoforge-storage/src/value.rs；crates/ontoforge-storage/src/commit.rs；crates/ontoforge-storage/src/store.rs；crates/ontoforge-storage/tests/conformance/mod.rs；crates/ontoforge-storage/tests/conformance_pg.rs | substantive / WIRED |
| E03-22 | CLI传RolesCheck；migrate.apply_in锁内compile_scope后才DDL；全对象criteria读取需全列。 crates/ontoforge-storage/src/migrate.rs；crates/ontoforge/src/migrate.rs；crates/ontoforge-auth/src/scope/coverage.rs；crates/ontoforge-storage/tests/migrate_pg.rs | substantive / WIRED |
| E03-23 | fresh-tree reload/scope plan/dev-token与父目录权限，输入拒绝无文件副作用。 crates/ontoforge-storage/src/store.rs；crates/ontoforge-auth/src/authn/dev.rs；examples/server/ontoforge-server.dev.toml；docs/edit-store.md | substantive / WIRED |

### Data-Flow Trace (Level 4)

| 数据 | 真实来源与消费者 | 判断 |
|---|---|---|
| 定义/角色 | request_snapshot在同一metadata锁内读active/scope，结束事务后编译与网络；后继S:603/631/643证明读请求不混版本且释放锁 | FLOWING |
| edit/log/history/version | commit:542真实事务写edit和append log/entry/history，错误rollback；history_with_snapshot:120真实查询后当前scope mask，普通caller不取params | FLOWING |
| 行可见性 | CEL→Pred→scope.lower→真实SQLite/Trino；同一Rust eval三值判断，缺属性Unknown不通过 | FLOWING |

### Behavioral Spot-Checks / Probe Execution

本轮没有新执行测试，以下是独立读取既有真实命令及对应actual输出；不冒充新跑named test。没有PLAN声明的probe-*.sh，probe N/A。

| 合同 | 既有真实执行与结果 | 判断 |
|---|---|---|
| 03存储/授权基础（04的前置边界） | I:567 PG环境；PG conformance45、reload11、migrate4、scope3、history1、import2 actual pass，均0 failed/0 ignored | VERIFIED（各测试仅证明各自合同） |
| CEL与缓存可见性 | I:1105-1106/1135-1136显式两URL；1116-1118三项Trino差分、1144缓存隔离actual ok | VERIFIED |
| 当前scoped读与PG替身一致 | I:1162/1187-1188环境；query_read_pg 2和trino_query_it 5实际通过，1199-1203含EXPLAIN/安全/关系/集合 | VERIFIED |
| 后继snapshot/锁释放 | S:603/631/643具名actual ok；server-services require_env并启动真实后端，不是缺env return | VERIFIED（边界原语） |

真实环境与早退核查详`service-environment-evidence.json`及`GOVERNANCE-AND-BOUNDARIES.md`。rowpred/trino_query/reload_trino/query_bench的required helpers缺env均panic；PG support缺两种来源时失败。PLAN03-20的两个延后human-check仅要求看CI步骤与非零计数，当前原始I日志逐目标满足并有actual run，已关闭，不声称再次人工视觉UAT。

### Requirements Coverage

| Requirement | 原始描述 | 支持PLAN | 判断 |
|---|---|---|---|
| STORE-01 | edit store 在 PostgreSQL（生产）或 SQLite（dev / test）上运行，两个后端通过同一套 conformance 测试 | 03-01/03-03/03-04/03-08/03-09/03-13/03-20/03-21 | SATISFIED（实现合同） |
| AUTH-01 | 客户端可以用 OIDC 签发的 JWT 认证（JWKS 自动拉取与缓存） | 03-01/03-02/03-07 | SATISFIED（实现合同） |
| STORE-02 | 新增类定义变更在 reload 时自动生成 DDL；破坏性变更被 lint 拦下，经 `ontoforge migrate --plan/--apply` 完成并归档旧列；reload 原子生效（Trino 元数据可见后才切换、`lock_timeout`、`def_version` 栅栏） | 03-02/03-03/03-04/03-10/03-15/03-18/03-20 | SATISFIED（实现合同） |
| SCOPE-02 | 角色可授予可见 object type、属性 allow-list、CEL 行谓词（只引用同行属性与 `principal.attrs`）、action 执行权、SQL 级别与 MCP toolset | 03-02/03-05/03-06/03-11/03-14/03-16 | SATISFIED（实现合同） |
| SCOPE-04 | 同一 CEL 行谓词编译成 SQL 与在 Rust 中对单行求值的结果一致（差分测试覆盖） | 03-05/03-11/03-20 | SATISFIED（实现合同） |
| SCOPE-05 | 无授权 = 不可见（fail-closed）；缺失的主体属性使相关谓词为 false；无法编译的谓词在 validate 时报错 | 03-05/03-11/03-14 | SATISFIED（实现合同） |
| SCOPE-01 | 管理员可以管理用户、组（可同步自 OIDC `groups` claim）与角色，角色绑定到用户或组 | 03-06/03-07/03-14/03-16 | SATISFIED（实现合同） |
| STORE-03 | 每次成功提交的写入都追加到不可变 edit log，记录 actor、action、时间、参数与 edits | 03-08/03-09/03-21 | SATISFIED（实现合同） |
| STORE-07 | 叠加按属性 patch：edit 只存设置过的属性，未设置的属性跟随 source；tombstone 不复活，release 显式且版本单调递增 | 03-08/03-13 | SATISFIED（实现合同） |
| STORE-05 | link 基数（1:1、1:N）在 edit store 写入时强制；source 数据的基数由 lint / 校验查询报告，不阻塞读取 | 03-10/03-13 | SATISFIED（edit子合同）；source独立归07.1 |
| SCOPE-03 | 多角色合并为 scoped 基关系（行 = 各角色谓词之 OR，列按授予它的角色谓词遮蔽），所有表达式只能绑定到该关系，不产生跨角色列 × 行泄露 | 03-11 | SATISFIED（实现合同） |
| AUTH-02 | dev 模式下开发者可以用 CLI 生成本地 token，无需外部 IdP | 03-12/03-23 | SATISFIED（实现合同） |
| SCOPE-06 | 定义 reload 与角色变更时，被授予某 action 的角色列集必须覆盖其 criteria 引用的列，否则报错不生效 | 03-14/03-16/03-18/03-22 | SATISFIED（实现合同） |
| SCOPE-07 | 角色 / scope 只能经管理通道或 L3 修改并审计；ontology 定义只经部署方 reload；agent MCP 无变更工具；每个请求固定使用开始时的定义版本 | 03-16/03-18 | SATISFIED（03管理写/存储snapshot边界）；MCP/transport归06/07 |
| STORE-04 | 客户端可以查询某个对象的变更历史 | 03-17 | SATISFIED（实现合同） |
| STORE-06 | 开发者可以用 `ontoforge import` 导入 edit-only 对象与替身引擎数据，导入作为系统 edit 记入 edit log；业务数据摄取由 Trino ETL（L3）负责 | 03-19/03-21 | SATISFIED（实现合同） |

全部16个ID在PLAN与REQUIREMENTS相符，无orphan。REQUIREMENTS checkbox/审计矩阵不是行为证据。03-VALIDATION仍保留历史source报告未完成/Nyquist false，当前REQUIREMENTS已分配07.1 Complete；本报告保留文件并指出其时间边界，不越权重写治理文件或替07.1签署，也不把历史partial洗成全需求通过。

### Prohibitions / Test Quality / Anti-Patterns

四项test-tier禁止项逐一核验：TLS/helper + U:453-461六项hermetic反例和真实图；SQLite不可更宽 + SQLite/Rust property与当前Trino同语料；dev不可仅配置启用 + config_alone_never_enables_dev_tokens；import不可写source-backed/默认覆盖 + source_backed_type_is_rejected/existing_objects_block_create_only。各项均有wired enforcement，不凭status:resolved放行。

原覆盖407个不同文件全部存在；受核Rust源码无未引用TBD/FIXME/XXX或placeholder，未发现wired却静态空数据的路径。外部依赖ignored测试由专门CI目标actual执行，workspace ignored不作pass。状态初始空集合不是stub，真实query/store结果随后填充。静态pattern例外均已追接实际调用。没有可观察的缺实现、断链或新增BLOCKER；旧Windows race单次通过不等于消除flaky风险。

### Decision Coverage / Advisory

GSD native decision gate exit=0：18/18 honored，not_honored=[]，warning-only。本轮无新增未经确定证据支持的BLOCKER。保留历史advisory：dev-key Windows ACL/既有文件权限及symlink加固，公共apply_migration调用者须传RolesCheck的集成责任，既有SQLite race flaky。它们不作为本轮已修复事实。

### Human Verification Required

无剩余项。前轮唯一人工项是03-01.1等待提供独立原记录或开发者明确接受证据边界，并非运行行为测试。当前用户已针对这一明确问题回复“授权”；本轮按GSD Step3b记录单项PASSED (override)，此等待决议已resolved。其他人工测试不得被override压制；本轮核对原列表仅此1项、behavior_unverified=0、无其他UNCERTAIN/FAILED或未解决人工项，因此决策树允许status为passed。

### 03-01.1 单项授权决议

- 唯一授权事实：`.planning/milestones/v1.0-audit/evidence/2026-11-05/phase03-approval-override/authorization.md`；未修改该文件。
- accepted_by：当前会话用户（项目/需求负责人）；accepted_at：2026-11-05T11:26:39.587Z，使用本轮授权记录时间，不冒充历史批准时间。
- reason：当前会话用户明确接受03-01.1安装前人工批准缺独立留档的历史证据边界并授权单项override；未提供或找到新历史批准记录，本轮授权不是过去安装前批准，不豁免其他合同、行为或安全测试。
- 处置只作用于完整原文对应的03-01.1，不外推Phase01/02、ENGINE-05、其他安全或行为合同。没有提供、找到或重建新的安装前人工批准证据；原03-01-SUMMARY声明不升级为独立记录。
- GSD references/verification-overrides.md允许将明确接受的单项偏差计为PASSED (override)，其余人工项仍独立判断；本次是原人工项所等待的明确决议，不是用override抹去别的未测试行为。
- 本轮未运行Rust/CI、安装依赖、修改源码或操作容器；TDD不适用于此纯授权记录与验收处置。主会话已调用PWF并维护共享计划；ralph-loop/Warp不可用，未声称调用。

### Fingerprint / Covered Set Preservation

前轮human_needed报告及其完整345项covered集合、逐文件raw SHA、native human_needed结果和原digest保存在本轮`phase03-approval-override/03-VERIFICATION.human-needed.before.md`与`baseline.json`。快照与原文件逐字节相同；旧`report-resign-03-04/`全部只读，旧machine回执继续表示当时的138/139与human_needed，不覆盖或伪称当时已授权。

本轮重新核对旧345项native fingerprint与原digest一致；前轮318项受管源码/治理记录与8项原执行输入raw hash均无漂移。保留其余138项原判断/原证据；仅03-01.1行及其派生状态、分数、人工待决说明和override记录改变。

以原345项并上5项必要授权/决议输入调用native `query verification.fingerprint`，输出covered_files/covered_digest原样写入frontmatter。当前covered为350项，removed=[]，原治理项全部保留，不过滤.planning；本报告自身及最终fingerprint/diff/gate等派生回执不进入自身指纹，也不新增其他VERIFICATION依赖。

本轮证据目录为`.planning/milestones/v1.0-audit/evidence/2026-11-05/phase03-approval-override/`。`decision.json`保存完整must_have、授权四字段和适用范围，`input-drift-check.json`保存138项不变与原输入对账，`fingerprint.json`、`covered-diff.json`、`native-gate.json`为本轮新回执。ROADMAP/REQUIREMENTS和原全部治理文件保留，GSD自行处理planning-root inert语义，不手工拼造digest。

### Gaps Summary

运行能力与5条SC继续有原源码和当前HEAD真实日志支持；133条PLAN运行/静态验收保持VERIFIED。唯一历史人工批准独立记录缺失仍公开保留，但当前用户已明确接受该历史证据边界并授权单项PASSED (override)。总分139/139由138 VERIFIED + 1 PASSED (override)构成，不是139项均有新测试或历史批准证明。无剩余人工项、行为未验证项或BLOCKER，故本轮报告status为passed；不自动认定milestone、矩阵或发布通过。

_Verified: 2026-11-05T11:31:37.586Z_
_Verifier: gsd-verifier_
