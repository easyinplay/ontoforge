---
phase: 01-workspace-schema-lint
verified: 2026-11-05T14:22:22Z
status: human_needed
score: 102/102 contract truths checked (101 VERIFIED; 1 prior override)
legacy_score: 11/11 original must-haves checked (10 VERIFIED; 1 prior override)
plan_truths: 97
roadmap_success_criteria: 5
verified_truths: 102
machine_verified_truths: 101
judgment_unresolved: 3
behavior_unverified: 0
overrides_applied: 1
covered_files: [".gitattributes",".github/workflows/ci.yml",".planning/adr/0002-naming-convention.md",".planning/adr/0003-ontology-definition-format.md",".planning/adr/0006-expression-language.md",".planning/adr/0011-workspace-layout.md",".planning/adr/0012-definition-document-shape.md",".planning/milestones/v1.0-audit/evidence/2026-11-05/ci-41994479295-final.json",".planning/milestones/v1.0-audit/evidence/2026-11-05/ci-41994479295-integration.log",".planning/milestones/v1.0-audit/evidence/2026-11-05/ci-41994479295-isolated-query-bench.log",".planning/milestones/v1.0-audit/evidence/2026-11-05/ci-41994479295-phase7-real-services.log",".planning/milestones/v1.0-audit/evidence/2026-11-05/ci-41994479295-ubuntu.log",".planning/milestones/v1.0-audit/evidence/2026-11-05/ci-41994479295-windows.log",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/01-VERIFICATION.before.md",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/CONTRACT-RESULTS.md",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/acceptance-contracts.json",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/anti-patterns.json",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/build-evidence.cjs",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/changed-source-inspection.json",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/collect.cjs",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/contract-index.json",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/contracts.json",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/covered-baseline.json",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/covered-file-inspection.json",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/evidence-inputs.json",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/executed-tests.json",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/inspect-details.cjs",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/mvp-story-validation.json",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/requirements-coverage.json",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/selected-test-source.json",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/source-assertions.json",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/summary-review.json",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/supplemental-inspection.json",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/truth-results.json",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/workspace-baseline.json",".planning/milestones/v1.0-audit/evidence/2026-11-05/reverify-local-results.json",".planning/milestones/v1.0-audit/evidence/2026-11-05/reverify-run.ps1",".planning/milestones/v1.0-audit/evidence/2026-11-05/reverify-workspace.log",".planning/phases/01-workspace-schema-lint/01-01-PLAN.md",".planning/phases/01-workspace-schema-lint/01-01-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-02-PLAN.md",".planning/phases/01-workspace-schema-lint/01-02-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-03-PLAN.md",".planning/phases/01-workspace-schema-lint/01-03-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-04-PLAN.md",".planning/phases/01-workspace-schema-lint/01-04-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-05-PLAN.md",".planning/phases/01-workspace-schema-lint/01-05-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-06-PLAN.md",".planning/phases/01-workspace-schema-lint/01-06-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-07-PLAN.md",".planning/phases/01-workspace-schema-lint/01-07-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-08-PLAN.md",".planning/phases/01-workspace-schema-lint/01-08-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-09-PLAN.md",".planning/phases/01-workspace-schema-lint/01-09-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-10-PLAN.md",".planning/phases/01-workspace-schema-lint/01-10-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-11-PLAN.md",".planning/phases/01-workspace-schema-lint/01-11-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-12-PLAN.md",".planning/phases/01-workspace-schema-lint/01-12-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-13-PLAN.md",".planning/phases/01-workspace-schema-lint/01-13-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-14-PLAN.md",".planning/phases/01-workspace-schema-lint/01-14-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-CONTEXT.md",".planning/phases/01-workspace-schema-lint/01-UAT.md",".planning/phases/01-workspace-schema-lint/01-VALIDATION.md",".vscode/settings.json",".zed/settings.json","CONTRIBUTING.md","Cargo.lock","Cargo.toml","ci/check-tls-deps.sh","ci/test_tls_deps.py","crates/ontoforge-expr/src/analyze.rs","crates/ontoforge-expr/src/check.rs","crates/ontoforge-expr/src/infer.rs","crates/ontoforge-expr/src/lib.rs","crates/ontoforge-expr/src/limits.rs","crates/ontoforge-expr/src/rowpred/eval.rs","crates/ontoforge-expr/src/rowpred/ir.rs","crates/ontoforge-expr/src/rowpred/mod.rs","crates/ontoforge-expr/src/rowpred/parse.rs","crates/ontoforge-expr/src/rowpred/value.rs","crates/ontoforge-expr/src/walk.rs","crates/ontoforge-expr/tests/analyze.rs","crates/ontoforge-expr/tests/criteria.rs","crates/ontoforge-schema/Cargo.toml","crates/ontoforge-schema/src/diag.rs","crates/ontoforge-schema/src/ir/action.rs","crates/ontoforge-schema/src/ir/attrs.rs","crates/ontoforge-schema/src/ir/basetype.rs","crates/ontoforge-schema/src/ir/binding.rs","crates/ontoforge-schema/src/ir/link.rs","crates/ontoforge-schema/src/ir/manifest.rs","crates/ontoforge-schema/src/ir/mod.rs","crates/ontoforge-schema/src/ir/names.rs","crates/ontoforge-schema/src/ir/object.rs","crates/ontoforge-schema/src/ir/scope.rs","crates/ontoforge-schema/src/lib.rs","crates/ontoforge-schema/src/lint/binding/e13.rs","crates/ontoforge-schema/src/lint/binding/e14.rs","crates/ontoforge-schema/src/lint/binding/e15.rs","crates/ontoforge-schema/src/lint/binding/e16.rs","crates/ontoforge-schema/src/lint/binding/mod.rs","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e13__tests__clean.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e13__tests__triggers.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e14__tests__clean.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e14__tests__triggers.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e15__tests__clean.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e15__tests__triggers.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e16__tests__clean.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e16__tests__triggers.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__w17__tests__clean.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__w17__tests__triggers.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__w17__tests__unreadable.snap","crates/ontoforge-schema/src/lint/binding/w17.rs","crates/ontoforge-schema/src/lint/config.rs","crates/ontoforge-schema/src/lint/e08.rs","crates/ontoforge-schema/src/lint/e09.rs","crates/ontoforge-schema/src/lint/mod.rs","crates/ontoforge-schema/src/lint/naming/mod.rs","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w01__tests__clean.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w01__tests__triggers.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w02__tests__clean.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w02__tests__triggers.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w03__tests__clean.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w03__tests__triggers.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w04__tests__clean.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w04__tests__triggers.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w05__tests__clean.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w05__tests__triggers.snap","crates/ontoforge-schema/src/lint/naming/w01.rs","crates/ontoforge-schema/src/lint/naming/w02.rs","crates/ontoforge-schema/src/lint/naming/w03.rs","crates/ontoforge-schema/src/lint/naming/w04.rs","crates/ontoforge-schema/src/lint/naming/w05.rs","crates/ontoforge-schema/src/lint/snapshots/ontoforge_schema__lint__e08__tests__clean.snap","crates/ontoforge-schema/src/lint/snapshots/ontoforge_schema__lint__e08__tests__triggers.snap","crates/ontoforge-schema/src/lint/snapshots/ontoforge_schema__lint__e09__tests__clean.snap","crates/ontoforge-schema/src/lint/snapshots/ontoforge_schema__lint__e09__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/mod.rs","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w06__tests__clean.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w06__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w07__tests__clean.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w07__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w08__tests__clean.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w08__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w09__tests__clean.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w09__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w10__tests__clean.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w10__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w11__tests__clean.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w11__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/w06.rs","crates/ontoforge-schema/src/lint/structure/w07.rs","crates/ontoforge-schema/src/lint/structure/w08.rs","crates/ontoforge-schema/src/lint/structure/w09.rs","crates/ontoforge-schema/src/lint/structure/w10.rs","crates/ontoforge-schema/src/lint/structure/w11.rs","crates/ontoforge-schema/src/lint/words.rs","crates/ontoforge-schema/src/loader/discover.rs","crates/ontoforge-schema/src/loader/dispatch.rs","crates/ontoforge-schema/src/loader/fixture.rs","crates/ontoforge-schema/src/loader/mod.rs","crates/ontoforge-schema/src/loader/parse.rs","crates/ontoforge-schema/src/loader/scope.rs","crates/ontoforge-schema/src/loader/sourcemap.rs","crates/ontoforge-schema/src/schema_export.rs","crates/ontoforge-schema/src/validate/actions.rs","crates/ontoforge-schema/src/validate/bindings.rs","crates/ontoforge-schema/src/validate/links.rs","crates/ontoforge-schema/src/validate/mod.rs","crates/ontoforge-schema/src/validate/objects.rs","crates/ontoforge-schema/tests/action_types.rs","crates/ontoforge-schema/tests/bindings.rs","crates/ontoforge-schema/tests/diagnostics.rs","crates/ontoforge-schema/tests/link_types.rs","crates/ontoforge-schema/tests/lint_examples.rs","crates/ontoforge-schema/tests/load_examples.rs","crates/ontoforge-schema/tests/loader.rs","crates/ontoforge-schema/tests/object_types.rs","crates/ontoforge-schema/tests/schema_export.rs","crates/ontoforge-schema/tests/scope_documents.rs","crates/ontoforge-schema/tests/snapshots/scope_documents__scope_diagnostics.snap","crates/ontoforge/Cargo.toml","crates/ontoforge/src/cli.rs","crates/ontoforge/src/lib.rs","crates/ontoforge/src/lint_cmd.rs","crates/ontoforge/src/main.rs","crates/ontoforge/src/pipeline.rs","crates/ontoforge/src/render.rs","crates/ontoforge/tests/cli.rs","crates/ontoforge/tests/cli_color.rs","crates/ontoforge/tests/cli_lint.rs","crates/ontoforge/tests/cli_schema.rs","crates/ontoforge/tests/invalid_examples.rs","crates/ontoforge/tests/snapshots/cli__validate_unknown_field_human.snap","crates/ontoforge/tests/snapshots/cli_lint__e08_human.snap","crates/ontoforge/tests/snapshots/cli_lint__e14_human.snap","crates/ontoforge/tests/snapshots/cli_online__online_missing_column_human.snap","crates/ontoforge/tests/snapshots/cli_online_trino__unavailable_human.snap","crates/ontoforge/tests/snapshots/invalid_examples__v01-json-trailing-comma.snap","crates/ontoforge/tests/snapshots/invalid_examples__v01-yaml-duplicate-key.snap","crates/ontoforge/tests/snapshots/invalid_examples__v02-missing-kind.snap","crates/ontoforge/tests/snapshots/invalid_examples__v03-function-rule.snap","crates/ontoforge/tests/snapshots/invalid_examples__v03-reserved-kind.snap","crates/ontoforge/tests/snapshots/invalid_examples__v04-unknown-field.snap","crates/ontoforge/tests/snapshots/invalid_examples__v05-missing-backing.snap","crates/ontoforge/tests/snapshots/invalid_examples__v05-missing-primary-key.snap","crates/ontoforge/tests/snapshots/invalid_examples__v06-precision-on-string.snap","crates/ontoforge/tests/snapshots/invalid_examples__v06-unknown-base-type.snap","crates/ontoforge/tests/snapshots/invalid_examples__v07-bad-object-type-name.snap","crates/ontoforge/tests/snapshots/invalid_examples__v08-duplicate-binding.snap","crates/ontoforge/tests/snapshots/invalid_examples__v08-duplicate-object-type.snap","crates/ontoforge/tests/snapshots/invalid_examples__v08-link-end-clashes-property.snap","crates/ontoforge/tests/snapshots/invalid_examples__v09-binding-unknown-target.snap","crates/ontoforge/tests/snapshots/invalid_examples__v09-link-unknown-object-type.snap","crates/ontoforge/tests/snapshots/invalid_examples__v09-rule-unknown-property.snap","crates/ontoforge/tests/snapshots/invalid_examples__v10-float-primary-key.snap","crates/ontoforge/tests/snapshots/invalid_examples__v10-primary-key-not-required.snap","crates/ontoforge/tests/snapshots/invalid_examples__v11-backing-on-one-to-many.snap","crates/ontoforge/tests/snapshots/invalid_examples__v11-foreign-key-type-mismatch.snap","crates/ontoforge/tests/snapshots/invalid_examples__v11-many-to-many-with-foreign-key.snap","crates/ontoforge/tests/snapshots/invalid_examples__v12-create-missing-primary-key.snap","crates/ontoforge/tests/snapshots/invalid_examples__v12-current-time-on-string.snap","crates/ontoforge/tests/snapshots/invalid_examples__v12-modify-assigns-primary-key.snap","crates/ontoforge/tests/snapshots/invalid_examples__v12-value-source-type-mismatch.snap","crates/ontoforge/tests/snapshots/invalid_examples__v13-comparison-type-mismatch.snap","crates/ontoforge/tests/snapshots/invalid_examples__v13-non-boolean-condition.snap","crates/ontoforge/tests/snapshots/invalid_examples__v13-objects-not-object-reference.snap","crates/ontoforge/tests/snapshots/invalid_examples__v13-syntax-error.snap","crates/ontoforge/tests/snapshots/invalid_examples__v13-unknown-parameter.snap","crates/ontoforge/tests/snapshots/invalid_examples__v14-include-escapes-root.snap","crates/ontoforge/tests/snapshots/invalid_examples__v16-binding-outside-bindings-dir.snap","crates/ontoforge/tests/snapshots/invalid_examples__v16-fixture-escapes-root.snap","crates/ontoforge/tests/snapshots/invalid_examples__v16-source-without-binding.snap","examples/ontologies/aviation-json/actions.json","examples/ontologies/aviation-json/bindings/dev/bindings.json","examples/ontologies/aviation-json/bindings/dev/fixtures/aircraft.csv","examples/ontologies/aviation-json/bindings/dev/fixtures/flight_crew.csv","examples/ontologies/aviation-json/bindings/dev/fixtures/flights.csv","examples/ontologies/aviation-json/bindings/dev/fixtures/maintenance_records.csv","examples/ontologies/aviation-json/bindings/dev/fixtures/pilots.csv","examples/ontologies/aviation-json/links.json","examples/ontologies/aviation-json/objects.json","examples/ontologies/aviation-json/ontology.yaml","examples/ontologies/aviation/actions.yaml","examples/ontologies/aviation/bindings/dev/aircraft.yaml","examples/ontologies/aviation/bindings/dev/fixtures/aircraft.csv","examples/ontologies/aviation/bindings/dev/fixtures/flight_crew.csv","examples/ontologies/aviation/bindings/dev/fixtures/flights.csv","examples/ontologies/aviation/bindings/dev/fixtures/maintenance_records.csv","examples/ontologies/aviation/bindings/dev/fixtures/pilots.csv","examples/ontologies/aviation/bindings/dev/flight-crew.yaml","examples/ontologies/aviation/bindings/dev/flight.yaml","examples/ontologies/aviation/bindings/dev/maintenance-record.yaml","examples/ontologies/aviation/bindings/dev/pilot.yaml","examples/ontologies/aviation/links.yaml","examples/ontologies/aviation/objects.yaml","examples/ontologies/aviation/ontology.yaml","examples/ontologies/invalid/v01-json-trailing-comma/objects.json","examples/ontologies/invalid/v01-json-trailing-comma/ontology.yaml","examples/ontologies/invalid/v01-yaml-duplicate-key/objects.yaml","examples/ontologies/invalid/v01-yaml-duplicate-key/ontology.yaml","examples/ontologies/invalid/v02-missing-kind/objects.yaml","examples/ontologies/invalid/v02-missing-kind/ontology.yaml","examples/ontologies/invalid/v03-function-rule/defs.yaml","examples/ontologies/invalid/v03-function-rule/ontology.yaml","examples/ontologies/invalid/v03-reserved-kind/objects.yaml","examples/ontologies/invalid/v03-reserved-kind/ontology.yaml","examples/ontologies/invalid/v04-unknown-field/objects.yaml","examples/ontologies/invalid/v04-unknown-field/ontology.yaml","examples/ontologies/invalid/v05-missing-backing/objects.yaml","examples/ontologies/invalid/v05-missing-backing/ontology.yaml","examples/ontologies/invalid/v05-missing-primary-key/objects.yaml","examples/ontologies/invalid/v05-missing-primary-key/ontology.yaml","examples/ontologies/invalid/v06-precision-on-string/objects.yaml","examples/ontologies/invalid/v06-precision-on-string/ontology.yaml","examples/ontologies/invalid/v06-unknown-base-type/objects.yaml","examples/ontologies/invalid/v06-unknown-base-type/ontology.yaml","examples/ontologies/invalid/v07-bad-object-type-name/objects.yaml","examples/ontologies/invalid/v07-bad-object-type-name/ontology.yaml","examples/ontologies/invalid/v08-duplicate-binding/bindings/dev/a.yaml","examples/ontologies/invalid/v08-duplicate-binding/bindings/dev/b.yaml","examples/ontologies/invalid/v08-duplicate-binding/objects.yaml","examples/ontologies/invalid/v08-duplicate-binding/ontology.yaml","examples/ontologies/invalid/v08-duplicate-object-type/a.yaml","examples/ontologies/invalid/v08-duplicate-object-type/b.yaml","examples/ontologies/invalid/v08-duplicate-object-type/ontology.yaml","examples/ontologies/invalid/v08-link-end-clashes-property/defs.yaml","examples/ontologies/invalid/v08-link-end-clashes-property/ontology.yaml","examples/ontologies/invalid/v09-binding-unknown-target/bindings/dev/flight.yaml","examples/ontologies/invalid/v09-binding-unknown-target/objects.yaml","examples/ontologies/invalid/v09-binding-unknown-target/ontology.yaml","examples/ontologies/invalid/v09-link-unknown-object-type/defs.yaml","examples/ontologies/invalid/v09-link-unknown-object-type/ontology.yaml","examples/ontologies/invalid/v09-rule-unknown-property/defs.yaml","examples/ontologies/invalid/v09-rule-unknown-property/ontology.yaml","examples/ontologies/invalid/v10-float-primary-key/objects.yaml","examples/ontologies/invalid/v10-float-primary-key/ontology.yaml","examples/ontologies/invalid/v10-primary-key-not-required/objects.yaml","examples/ontologies/invalid/v10-primary-key-not-required/ontology.yaml","examples/ontologies/invalid/v11-backing-on-one-to-many/defs.yaml","examples/ontologies/invalid/v11-backing-on-one-to-many/ontology.yaml","examples/ontologies/invalid/v11-foreign-key-type-mismatch/defs.yaml","examples/ontologies/invalid/v11-foreign-key-type-mismatch/ontology.yaml","examples/ontologies/invalid/v11-many-to-many-with-foreign-key/defs.yaml","examples/ontologies/invalid/v11-many-to-many-with-foreign-key/ontology.yaml","examples/ontologies/invalid/v12-create-missing-primary-key/defs.yaml","examples/ontologies/invalid/v12-create-missing-primary-key/ontology.yaml","examples/ontologies/invalid/v12-current-time-on-string/defs.yaml","examples/ontologies/invalid/v12-current-time-on-string/ontology.yaml","examples/ontologies/invalid/v12-modify-assigns-primary-key/defs.yaml","examples/ontologies/invalid/v12-modify-assigns-primary-key/ontology.yaml","examples/ontologies/invalid/v12-value-source-type-mismatch/defs.yaml","examples/ontologies/invalid/v12-value-source-type-mismatch/ontology.yaml","examples/ontologies/invalid/v13-comparison-type-mismatch/defs.yaml","examples/ontologies/invalid/v13-comparison-type-mismatch/ontology.yaml","examples/ontologies/invalid/v13-non-boolean-condition/defs.yaml","examples/ontologies/invalid/v13-non-boolean-condition/ontology.yaml","examples/ontologies/invalid/v13-objects-not-object-reference/defs.yaml","examples/ontologies/invalid/v13-objects-not-object-reference/ontology.yaml","examples/ontologies/invalid/v13-syntax-error/defs.yaml","examples/ontologies/invalid/v13-syntax-error/ontology.yaml","examples/ontologies/invalid/v13-unknown-parameter/defs.yaml","examples/ontologies/invalid/v13-unknown-parameter/ontology.yaml","examples/ontologies/invalid/v14-include-escapes-root/ontology.yaml","examples/ontologies/invalid/v16-binding-outside-bindings-dir/flight-binding.yaml","examples/ontologies/invalid/v16-binding-outside-bindings-dir/objects.yaml","examples/ontologies/invalid/v16-binding-outside-bindings-dir/ontology.yaml","examples/ontologies/invalid/v16-fixture-escapes-root/bindings/dev/flight.yaml","examples/ontologies/invalid/v16-fixture-escapes-root/objects.yaml","examples/ontologies/invalid/v16-fixture-escapes-root/ontology.yaml","examples/ontologies/invalid/v16-source-without-binding/bindings/dev/flight.yaml","examples/ontologies/invalid/v16-source-without-binding/objects.yaml","examples/ontologies/invalid/v16-source-without-binding/ontology.yaml","rust-toolchain.toml","schemas/ontoforge-definition.schema.json","schemas/ontoforge-manifest.schema.json","schemas/ontoforge-scope.schema.json","target/milestone-ci-41759804733-integration.log","target/milestone-ci-41759804733-ubuntu.log","target/milestone-ci-41759804733-windows.log"]
covered_digest: "v4:sha256:5752098c8f87c1f3524d3b7901cfd1f5049eb49ada764b21e5219eaa5cbef893"
human_verification:
  - test: "明确接受或进一步核查01-01原创出处禁止项"
    expected: "确认没有参考仓库代码复制或标识符沿用；现有禁名扫描范围与原创出处不能完备证明的限制已知"
    why_human: "judgment-tier prohibition仅有NON-AUTHORITATIVE自动判断，历史AGREE不是逐项人工resolution"
  - test: "明确接受或进一步核查01-01路径与用户名禁止项"
    expected: "接受相对路径链路、测试与产品快照扫描的证据范围，或要求补查未覆盖产物"
    why_human: "局部确定性证据存在，但原合同verification为judgment，不能静默当作人工确认"
  - test: "明确接受或进一步核查01-08不得降级ONT-V禁止项"
    expected: "确认结构校验先于配置，ONT-V ID配置被拒绝；具名negative test已通过"
    why_human: "有行为证据支撑，但原judgment-tier处置需要显式resolution；自动授权不等同人工观察"
overrides:
  - must_have: "G-1-2：aviation 的任意 YAML document 输入 kind 可补全并标红未知字段"
    reason: "01-UAT Test 2 用户明确接受pass；只观察到新增document关联schema，补全弹窗、bogus标红、JSON manifest无误报未逐项观察，保留该限制"
    accepted_by: "user (01-UAT.md Test 2)"
    accepted_at: "2026-10-16T12:56:06Z"
decision_coverage:
  honored: 18
  total: 18
  not_honored: []
re_verification:
  previous_status: human_needed
  previous_score: 11/11
  trigger: "Complete original-contract verification after five covered implementation inputs moved; previous native routing stale"
  gaps_closed: []
  gaps_remaining: []
  regressions: []
prohibition_review:
  disposition: human_needed
  non_authoritative: true
  judgment_items: 3
  unresolved_human_items: 3
  new_authorization: false
  evidence: ".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/CONTRACT-RESULTS.md"
  limitation: "unverified-prohibition: human review required; continuation is not an override"
---

# Phase 01: Workspace, Schema & Lint Verification Report

**Phase Goal:** As a ontology 开发者, I want to 用 YAML/JSON 编写 ontology，并通过 `ontoforge validate` / `ontoforge lint` 得到带源码定位的诊断，且仓库在 Windows 本机 `cargo test` 全绿, so that 我能在本机可靠地编写和验证定义.

**Verified:** 2026-11-05T14:22:22Z
**Status:** human_needed
**Re-verification:** 完整原合同复核，不是仅签 fingerprint。最终 HEAD 为 060019fe52005c6b85ae5c5f59da9868f464e579。

## User Flow Coverage (MVP)

ROADMAP mode=mvp；owner `query user-story.validate` 返回 valid=true，原始回执见证据目录的 mvp-story-validation.json。

| 用户步骤 | 期望 | 实际数据流与已有执行证据 | 状态 |
|---|---|---|---|
| 在本机建立、编译 workspace | 原10 crate、Rust 1.98/2024/resolver3，无 Docker 默认测试 | 11份当前 manifest 逐份检查；原10均保留，engine 增量由 ADR0021、依赖方向由 ADR0023 明确修订。最终HEAD Windows CI、本地 workspace 已真实执行 | VERIFIED |
| 编写 YAML/JSON 多文件定义 | object/base type/required、三种 link、action/CEL 合为同一 IR | CLI -> pipeline -> load_dir -> discover/read/parse/dispatch -> typedIR + SourceMap；真实双格式航空示例完整IR、5object/5link/3action断言通过 | VERIFIED |
| 运行 validate 检查错误 | 相对文件名、line:col、错误码、非零退出 | 实际 invalid fixture 经 SourceMap -> Diagnostic -> human/JSON renderer；unknown-field 子进程断言 ONT-V04、objects.yaml、3:1、primaryKey help、exit1 | VERIFIED |
| 运行 lint 配置规则 | E/W01-W11、双输出、严重级及退出值正确 | 先结构/CEL错误早退，再 load_config -> registry -> rules -> Finding -> SourceMap -> Diagnostic -> render；触发/clean/override/deny-warnings 测试通过 | VERIFIED |
| 导出并使用 JSON Schema | 实际例子通过，非法输入被拒绝 | typedIR schema_for -> rendered_schemas -> fs::write；独立入库字节对照、真实双格式输入及反例测试通过 | VERIFIED |
| 在编辑器编写新 document | 关联 schema、补全和诊断 | 原 UAT Test2 已接受的 G-1-2 override；本轮只核实配置、modeline 与测试，不伪称新的实机观察 | PASSED (override) |

以上证明技术路径，不解除三个 judgment prohibition；完整 phase 仍不可标 passed。

## Goal Achievement / Contract Scope

完整读取14份 PLAN、14份 SUMMARY；SUMMARY仅作为叙事/文件/历史接口索引，不作为通过证据。完整合同、具体源码断言与具名测试映射在 `CONTRACT-RESULTS.md`、`truth-results.json`、`source-assertions.json`、`selected-test-source.json`；全量原文及 artifact/link owner 回执在 `contracts.json`。

- 原口径11项：10 VERIFIED +1原 editor override，11/11保持，未删除原 must-have。
- 细粒度97条 PLAN truth：96 VERIFIED +1原 editor override；另5条 ROADMAP SC均 VERIFIED。102是索引条目数，有语义重叠，不是102个去重用户行为。
- 3项 judgment 单列 UNCERTAIN/WARNING，不在机器验证分子；1项01-13 backstop保留独立历史差异及最终HEAD测试证据，不凭符号存在认定。
- 14份任务验收合同中的34个 acceptance block、132条条款及 verification/success_criteria 全部纳入映射。历史提交约束与当前运行合同分开，见后文兼容变化。
- 12个 requirements 无 orphan；18/18已追踪 CONTEXT decisions honored。无新增 BLOCKER、FAILED truth 或未解决的新 gray area。

### ROADMAP Success Criteria

| ID | 原合同 | 状态 | PLAN映射与证据 |
|---|---|---|---|
| SC1 | cargo workspace 按 ADR-0011 建好 10 个 `ontoforge-*` crate（edition 2024、resolver 3、rust-version 1.98），Windows 11 本机不依赖 Docker 执行 `cargo test --workspace` 全部通过；此条作为后续每个 phase 的回归门槛 | VERIFIED | 01-01.9, 01-02.1, 01-02.2, 01-02.3, 01-02.4, 01-11.8；原10crate均存在；第11个engine由ADR0021修订，依赖方向由ADR0023修订；最终HEAD本地workspace1528/0/119，CI两平台fmt/clippy/workspace真实成功。 |
| SC2 | `ontoforge validate examples/ontologies/<示例>/` 把分散在多个文件中的 object type、属性 base type 与 required、link type（1:1 / 1:N / M:N）、action type（参数、rules、submission criteria）合并加载为单一 IR；同一 ontology 的 YAML 与 JSON 写法得到相同 IR | VERIFIED | 01-03.1, 01-03.9, 01-04.8, 01-05.6, 01-06.6；真实aviation与aviation-json经load_dir后完整IR相等，明确断言5object/5link/3action，不是两个None相等。 |
| SC3 | 定义有错（重复 apiName、未知 base type、link 指向不存在的 object type 等）时，CLI 输出带文件名与 line:col 的诊断，退出码非 0 | VERIFIED | 01-01.3, 01-01.4, 01-03.8, 01-04.6, 01-05.3, 01-12.5；unknown-field CLI断言exit1、objects.yaml:3:1、primaryKey；invalid_examples逐目录要求预期code存在并比较独立入库快照。 |
| SC4 | `ontoforge lint` 运行适用于 v1 schema 范围的 ONT-E 规则与 ONT-W01–W11，输出人类可读（带源码定位）与 JSON 两种格式；有 E 级问题时退出码非 0；配置文件可按规则 ID 关闭规则或调整严重级 | VERIFIED | 01-08.1, 01-08.2, 01-08.3, 01-08.4, 01-08.5, 01-10.6, 01-11.7；lint先结构/CEL后配置，registry实际扩展E08/E09和W01-W11，触发/clean快照及off/info/error、deny-warnings、human/JSON子进程断言均有最终HEAD执行。 |
| SC5 | 开发者可以导出 ontology 定义的 JSON Schema，仓库内示例 ontology 通过该 Schema 校验 | VERIFIED | 01-09.1, 01-09.2, 01-09.3, 01-09.4, 01-09.5；Schema由typedIR实际生成、CLI写出；schema_export独立入库字节相等、真实双格式文档通过及invalid反例拒绝均执行；不把editor override当machineUX证明。 |

### 原11项完整保留

| 原索引 | must-have | 本轮状态 | 细粒度合同映射 |
|---|---|---|---|
| L1 | SC1：原10crate/工具链及Windows无Docker回归 | VERIFIED | SC1 |
| L2 | SC2：多文件IR与YAML/JSON等价 | VERIFIED | SC2 |
| L3 | SC3：错误定位与非零退出 | VERIFIED | SC3 |
| L4 | SC4：E/W01-W11、双输出与规则配置 | VERIFIED | SC4 |
| L5 | SC5：导出schema和真实例子校验 | VERIFIED | SC5 |
| L6 | G-1-2：新document编辑器补全、收窄与标红 | PASSED (override) | 01-14.1 |
| L7 | 两份严格JSON编辑器映射相同 | VERIFIED | 01-14.2 |
| L8 | manifest不误用definition schema | VERIFIED | 01-14.3 |
| L9 | 逐document modeline与clean/等价 | VERIFIED | 01-14.4 |
| L10 | CONTRIBUTING编辑器章节准确 | VERIFIED | 01-14.5 |
| L11 | 错误配置/schema/modeline由实质测试守住 | VERIFIED | 01-14.6 |

### 全部 PLAN 范围

| PLAN | truth数 | artifacts | key links | 最终状态 |
|---|---|---|---|---|
| 01-01 | 9 | 8 | 3 | VERIFIED |
| 01-02 | 6 | 4 | 2 | VERIFIED |
| 01-03 | 10 | 3 | 2 | VERIFIED |
| 01-04 | 8 | 5 | 2 | VERIFIED |
| 01-05 | 6 | 3 | 2 | VERIFIED |
| 01-06 | 6 | 4 | 2 | VERIFIED |
| 01-07 | 5 | 4 | 3 | VERIFIED |
| 01-08 | 8 | 7 | 3 | VERIFIED |
| 01-09 | 6 | 4 | 2 | VERIFIED |
| 01-10 | 7 | 3 | 2 | VERIFIED |
| 01-11 | 8 | 4 | 2 | VERIFIED |
| 01-12 | 7 | 4 | 3 | VERIFIED |
| 01-13 | 5 | N/A (exit66) | N/A (exit66) | VERIFIED |
| 01-14 | 6 | 5 | 4 | 5 VERIFIED +1 prior override |

## Required Artifacts / Key Links / Data-Flow Trace

58/58声明 artifact、32/32声明 key link 的 owner 检查通过；01-13没有声明块，exit66为 N/A，非机器通过项，另核实 DeTable/heck/anstream/共享行列换算的实际调用、测试和历史差异。全部旧 covered 315个输入存在并逐一检查及冻结hash，不用文件存在代替行为。

| 起点 | 实质与接线 | 数据来源 / 防空壳证据 |
|---|---|---|
| main.rs / cli.rs | 32 MiB入口线程，Cli::parse -> run -> validate/pipeline::check_with_profile | 输入目录进入实际文件读取，非静态 IR 或硬编码成功 |
| loader/discover/parse/dispatch | glob、安全路径、大小限制、严格JSON、kind分派、类型构造与 SourceMap | load_examples/loader/diagnostics 的真实目录、反例和相等性断言 |
| validate/objects/links/actions + expr | 实际遍历属性、FK、accessor、action scope/CEL；词法/长度/深度预检和隔离编译 | object_types/link_types/action_types/analyze/criteria 的错误码、结果类型、panic/边界断言 |
| lint_cmd / config / registry | 结构错误先返回，配置只进入合法 lint ID；E08/E09与 naming/structure 规则产生真实 Finding | E/W触发与 clean 快照、off/info/error、deny-warnings及ONT-V反例 |
| render / diag | Diagnostic 实际序列化，ANSI环境处理、相对定位和退出码 | exact fields、固定坐标、human snapshot、color环境子进程断言 |
| schema_export / editor settings / modeline | typedIR生成schema、CLI写盘；两份strict JSON映射、manifest排除、逐document modeline | 独立入库schema对照、真实valid/invalid输入、逐document覆盖测试；UX边界仍为原override |

完整58个 artifact 和32个 key link 的 from/to/via 及 owner 回执在 contracts.json；97条逐项源码路径、断言与对应测试在 truth-results.json，不以此汇总表替代完整映射。

## 五个变化源码的专门回归

| 文件 | 原因与当前具体断言 | 行为/历史证据 |
|---|---|---|
| CONTRIBUTING.md | 编辑器章节124-132仍覆盖单document modeline、两份映射glob、manifest排除、JetBrains和Ctrl+Space；后续新增集成/MinIO说明不破坏章节 | schema_export逐document/strict settings/manifest排除断言；原阶段7个二级标题与现8个分开，不虚报当前计数仍为7 |
| Cargo.toml | resolver3、edition2024、rust-version1.98、unsafe_code=forbid；全部11份manifest核实原10存在；后续engine/依赖扩展有ADR0021/0023 | 最终HEAD两平台workspace、fmt、clippy真实成功；01-02提交不改根manifest另以git show核实，非沿用SUMMARY |
| crates/ontoforge-schema/src/diag.rs | 相对既有基线的W18增量；ONT-V/ONT-E code、Diagnostic字段与共享字节到字符列转换仍接线 | exact JSON fields、固定3:1坐标、中文/CRLF/BOM定位、invalid与human snapshots |
| crates/ontoforge/src/cli.rs | ValidateArgs.check_cardinality的long参数 requires=online；不更改offline validate/lint/schema分派 | cardinality_flag_contract、no_flag_means_no_cardinality_scan，以及完整离线CLI成功/反例 |
| crates/ontoforge/src/lib.rs | online分支统计W18 warning；不改变offline pipeline/config/render | warning_output_and_exit_codes，3个cardinality测试均有Windows/Ubuntu/local三份成功记录；不据此扩展重审07.1 |

changed-source-inspection.json保留git历史与diff；supplemental-inspection.json保留额外断言/ADR/测试内容；最终源码相对HEAD无产品修改。

### 历史验收与后续兼容变化

- 01-03 Task1的“不调用canonicalize”是原阶段实现约束。当前 loader/mod.rs:261/262 确实调用 canonicalize，Phase02 CR-S01提交82e6dde及独立02-REVIEW-FIX记录是拒绝中间junction/symlink的安全加固。仅用于内部containment，Diagnostic.file仍由relative SourceMap产生；不虚称当前grep零匹配。既有TOCTOU边界不扩大为本轮新缺陷或全面安全保证。
- 原10crate和原依赖方向与当前11crate/后续方向变化有独立accepted ADR修订；不是本轮静默接受偏差或新增override。
- 01-13 d021687..07a0e06的快照diff为空，锁文件只增加anstream/heck依赖连线；已独立核实JSON字段/固定坐标/快照执行，不能仅凭SUMMARY认定backstop，也不伪称本轮执行过历史RED。
- 01-14提交前numstat/章节数按当时提交核实，不把历史7章节要求误作永久禁止增文档。编辑器章节及所有原回归仍在。

## Behavioral Evidence (Existing Executions, Not Re-runs)

本轮没有运行Rust测试或CI、没有启动服务、操作容器或清target。复用最终HEAD的真实原始执行日志；证据目录上一级的 ci-41994479295-final.json 明确headSha为 060019fe52005c6b85ae5c5f59da9868f464e579，status=completed、conclusion=success，五个job成功。Windows/Ubuntu workspace和fmt/clippy步骤均有原始日志及元数据，不以早期CI冒充当前HEAD。

本地 `reverify-workspace.log` 原始头记录同一HEAD及命令 `cargo test --workspace --locked -- --test-threads=1`；2026-11-05T08:43:50.3003373Z 至 08:51:34.9365322Z，exit_code=0，按原始summary合计 **1528 passed /0 failed /119 ignored**。ignored不算通过；这是默认无Docker测试，未声称私有服务测试全部被执行。runner的INSTA_UPDATE=no且未设置ONTOFORGE_UPDATE_SCHEMAS；入库snapshot/schema与HEAD无diff。

164个选定具名测试逐一读取实质assert/expect，无ignore，并各自找到WindowsCI、UbuntuCI和local-workspace成功记录。executed-tests.json共4887条成功记录跨多个job，不能解释为4887个独立测试。selected-test-source.json包含每个具名测试的源码行、断言、job、日志路径、行号及原始输出。额外3个cardinality兼容测试在supplemental-inspection.json，不混入164计数。

| 行为 | 具名测试 / 既有local日志锚点 | 结论 |
|---|---|---|
| 正常目录真实验证 | validate_aviation_succeeds_silently / reverify-workspace.log:51 | PASS，子进程exit0且静默 |
| 错误定位与退出 | validate_unknown_field_points_at_the_misspelled_key / :54 | PASS，V04/objects.yaml/3:1/help/exit1 |
| JSON线格式 | json_output_has_exactly_the_documented_fields_and_no_ansi / :48 | PASS，字段集合与无ANSI |
| 终端颜色环境 | color_environment_controls_human_output_and_never_colors_json / :94 | PASS，6种环境断言；人眼显示沿原UAT Test1 |
| 编译隔离与资源限制 | closure_panic、deeply_nested、too_long相关用例 / selected-test-source.json exact records | PASS，源码实质断言和三路成功记录，不以存在代替状态不变量 |
| criteria定位与类型 | analyzer_errors、comparison、comprehension、location、nonbool相关用例 / selected-test-source.json | PASS，类型/定位/失败结果断言 |

对所有14份PLAN/SUMMARY搜索无本phase声明的shell probe；Probe Execution为N/A。本轮复用执行日志符合明确“不重跑Rust/CI”边界，不伪称新测试执行。

## Requirements Coverage

| PLAN | Requirement ID | Description | Status | 人工边界说明 |
|---|---|---|---|---|
| 01-01-PLAN.md, 01-02-PLAN.md, 01-11-PLAN.md | DX-03 | Windows 本机 `cargo test` 不依赖 Docker 即可全部通过 | SATISFIED_TECHNICAL | 技术执行证据不解除3项judgment；phase保持human_needed |
| 01-01-PLAN.md, 01-03-PLAN.md, 01-06-PLAN.md | SCHEMA-01 | 开发者可以用 YAML（或等价 JSON）在多个文件中定义一个 ontology，`ontoforge validate` 把它们合并加载为单一 IR | SATISFIED_TECHNICAL | 仅技术覆盖，不解除原创出处、路径/用户名、ONT-V三项judgment或原editor观察限制 |
| 01-01-PLAN.md, 01-03-PLAN.md, 01-04-PLAN.md, 01-05-PLAN.md, 01-06-PLAN.md, 01-12-PLAN.md, 01-13-PLAN.md | SCHEMA-06 | 定义有错时，开发者看到带文件名与 line:col 的诊断信息 | SATISFIED_TECHNICAL | 路径/用户名judgment仍为UNCERTAIN/WARNING，需显式人工resolution |
| 01-04-PLAN.md | SCHEMA-02 | 开发者可以定义 object type：`apiName`、primary key、title key、displayName、description、aliases、hidden | SATISFIED_TECHNICAL | 仅技术覆盖，不解除原创出处、路径/用户名、ONT-V三项judgment或原editor观察限制 |
| 01-04-PLAN.md | SCHEMA-03 | 开发者可以为属性选择 base type：Byte / Short / Integer / Long / Float / Double / Decimal、String、Boolean、Date、Timestamp、Array、Geopoint，并声明 required | SATISFIED_TECHNICAL | 仅技术覆盖，不解除原创出处、路径/用户名、ONT-V三项judgment或原editor观察限制 |
| 01-05-PLAN.md | SCHEMA-04 | 开发者可以定义 link type：1:1、1:N（外键属性）、M:N（join 定义），两个方向各有 apiName | SATISFIED_TECHNICAL | 仅技术覆盖，不解除原创出处、路径/用户名、ONT-V三项judgment或原editor观察限制 |
| 01-06-PLAN.md, 01-07-PLAN.md, 01-12-PLAN.md | SCHEMA-05 | 开发者可以定义 action type：参数（类型、required、默认值）、rules（create / modify / delete object，add / remove link）、submission criteria（CEL） | SATISFIED_TECHNICAL | 仅技术覆盖，不解除原创出处、路径/用户名、ONT-V三项judgment或原editor观察限制 |
| 01-08-PLAN.md | LINT-01 | `ontoforge lint` 对 ontology 运行 E 级规则（适用于 v1 schema 范围的 ONT-E 规则），有 E 级问题时退出码非 0 | SATISFIED_TECHNICAL | 仅技术覆盖，不解除原创出处、路径/用户名、ONT-V三项judgment或原editor观察限制 |
| 01-08-PLAN.md, 01-13-PLAN.md | LINT-03 | lint 结果可输出为人类可读格式（带源码定位）和 JSON 格式 | SATISFIED_TECHNICAL | 仅技术覆盖，不解除原创出处、路径/用户名、ONT-V三项judgment或原editor观察限制 |
| 01-08-PLAN.md, 01-13-PLAN.md | LINT-04 | 开发者可以在配置中按规则 ID 关闭或调整严重级 | SATISFIED_TECHNICAL | ONT-V不可关闭/降级judgment仍为UNCERTAIN/WARNING，需显式人工resolution |
| 01-09-PLAN.md, 01-14-PLAN.md | SCHEMA-07 | 开发者可以导出 ontology 定义的 JSON Schema，用于编辑器补全与 LLM 生成 | SATISFIED_TECHNICAL | 沿01-UAT Test2既有editor override，不代表新增实机观察；3项judgment不解除 |
| 01-10-PLAN.md, 01-11-PLAN.md | LINT-02 | `ontoforge lint` 运行 W01–W11 命名与结构警告 | SATISFIED_TECHNICAL | 仅技术覆盖，不解除原创出处、路径/用户名、ONT-V三项judgment或原editor观察限制 |

12个ID全由PLAN声明；第二列固定为Requirement ID，Status列保留SATISFIED_TECHNICAL供派生解析器读取。技术覆盖不代表完整milestone人工门已通过；3项judgment仍待逐项显式人工resolution，既有editor override不扩大。详细需求原文及来源在requirements-coverage.json。

## Anti-Patterns / Advisory

产品旧covered范围未引用TBD/FIXME/XXX为0，限定禁名扫描为0，产品snapshot/schema绝对路径扫描为0，选定164测试被ignore为0。空集合初始化/测试fixture不是用户输出stub，loader实际读盘与SourceMap链已追踪。grep仅证明限定范围，没有证明完整原创出处。symlink或非UTF8测试可因OS能力早退，不把一个ok扩大成所有平台分支均被执行。无新增BLOCKER或WARNING技术缺陷；保留三个原judgment WARNING。无新的coincidental-reliance结论或后移原phase义务。

## Human Verification Required

| ID | 原合同禁止项 | 当前证据与限制 | 处置 |
|---|---|---|---|
| J1 (01-01) | 代码、示例、快照与文档不得出现 ADR-0002 列出的参考仓库标识符，也不得复制参考仓库代码 | ADR0002指定标识符在产品范围扫描0命中；无法证明完整原创出处或未复制。需逐项人工resolution，本任务无新授权。 | UNCERTAIN / WARNING；需要逐项显式人工resolution |
| J2 (01-01) | 诊断输出、入库快照与生成产物不得包含本机绝对路径或用户名 | relative路径链、diagnostic_paths_use_forward_slash、JSON字段与快照扫描支持；原verification judgment仍需人工resolution，不把构建日志绝对路径混同产品泄露。 | UNCERTAIN / WARNING；需要逐项显式人工resolution |
| J3 (01-08) | ontoforge.toml 与任何 CLI 参数都不得关闭或降级 ONT-V 结构性错误 | lint_cmd先has_errors再load_config；KNOWN_RULE_IDS排除ONT-V；validate_codes_cannot_be_turned_off_by_config及validate_codes_cannot_be_configured已真实通过，但不代替judgment人工确认。 | UNCERTAIN / WARNING；需要逐项显式人工resolution |

三项均保留 `unverified-prohibition: human review required`。用户要求继续、自动执行授权、历史AGREE、negative test通过均不构成人工override。本任务没有新授权，不能从human_needed改为passed。

PLAN共3个延期human-check已收取：01-08终端可读性沿01-UAT Test1既有pass；01-09与01-14编辑器体验合并沿Test2原override，保持未逐项观察的限制，不重开已接受体验。三项禁止项不受这些记录影响。01-13 backstop有独立运行和历史证据，不能吞掉judgment项。

## Coverage / Freshness / Bounded Delivery

原报告原字节保存在证据目录的01-VERIFICATION.before.md，SHA256=bc964c6652a1e22c91a0abfb3cdaacf009b651c35fdd97d3f3796f4d4c5455ae；covered-baseline.json保留原315个covered与旧digest。**315 ->346，原315全保留，删除0，新增31**。新增仅冻结独立证据与最终HEAD真实日志，不添加会继续变化的matrix/T1/PWF/main-*.json。完整差异和原/新digest见covered-set-diff.json；官方fingerprint原始回执见fingerprint-results.json。

fingerprint只在完成全部合同、源码、既有执行日志核实后生成，covered_files和covered_digest直接取owner输出。报告自身不进入fingerprint，后验native gate/RESULT回执也不作为自依赖。native status及写后校验见native-gate-results.json、acceptance-check.json。

Phase02只读核实：旧MinIO FAILED已由07.2闭合；不改02原gaps_found判定，也不解除02-05.13原创/02-10.8私有SQLServer边界。本轮未沿用撤回审计。没有改生产、其他报告、matrix、T1/PWF，没有commit/push/publish/archive。

ralph-loop/Warp不可用，没有伪称调用；使用完整“原合同 -> 当前实现/反证 -> 真实执行 -> 人工边界 -> 覆盖差异 -> native gate”验收循环。TDD不适用于这次纯报告/既有证据复核，没有实现代码或新测试。本证据目录是有界T2记录，不更新用户禁止写入的PWF/T1。

**结论：human_needed。** 技术合同未发现新缺陷，三个judgment待显式人工决定。COMPLETE仅指本次有界复核和报告交付，不代表phase、milestone或人工门通过。

_Verifier: gsd-verifier; full original-contract evidence review._
