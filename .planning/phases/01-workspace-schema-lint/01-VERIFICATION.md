---
phase: 01-workspace-schema-lint
verified: 2026-11-07T07:46:42Z
status: passed
score: 102/102 accounted for (101 VERIFIED + 1 PASSED (override))
legacy_score: 10 VERIFIED + 1 editor override (11/11 accounted for)
plan_truths: 97
roadmap_success_criteria: 5
verified_truths: 101
machine_verified_truths: 101
judgment_unresolved: 0
behavior_unverified: 0
overrides_applied: 1
source_commit: f0681ae76e75bb163d3e0d590f69df49d0443655
verified_source_commit: 060019fe52005c6b85ae5c5f59da9868f464e579
historical_source_commit: 060019fe52005c6b85ae5c5f59da9868f464e579
uat_source_commit: 0148e922b42d61a48ded3af6dd3213ebcea32086
current_ci_run: 41994479295
covered_files: [".gitattributes",".github/workflows/ci.yml",".planning/adr/0002-naming-convention.md",".planning/adr/0003-ontology-definition-format.md",".planning/adr/0006-expression-language.md",".planning/adr/0011-workspace-layout.md",".planning/adr/0012-definition-document-shape.md",".planning/milestones/v1.0-audit/evidence/2026-11-05/ci-41994479295-final.json",".planning/milestones/v1.0-audit/evidence/2026-11-05/ci-41994479295-integration.log",".planning/milestones/v1.0-audit/evidence/2026-11-05/ci-41994479295-isolated-query-bench.log",".planning/milestones/v1.0-audit/evidence/2026-11-05/ci-41994479295-phase7-real-services.log",".planning/milestones/v1.0-audit/evidence/2026-11-05/ci-41994479295-ubuntu.log",".planning/milestones/v1.0-audit/evidence/2026-11-05/ci-41994479295-windows.log",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/01-VERIFICATION.before.md",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/CONTRACT-RESULTS.md",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/acceptance-contracts.json",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/anti-patterns.json",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/build-evidence.cjs",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/changed-source-inspection.json",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/collect.cjs",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/contract-index.json",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/contracts.json",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/covered-baseline.json",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/covered-file-inspection.json",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/evidence-inputs.json",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/executed-tests.json",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/inspect-details.cjs",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/mvp-story-validation.json",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/requirements-coverage.json",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/selected-test-source.json",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/source-assertions.json",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/summary-review.json",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/supplemental-inspection.json",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/truth-results.json",".planning/milestones/v1.0-audit/evidence/2026-11-05/report-resign-01/workspace-baseline.json",".planning/milestones/v1.0-audit/evidence/2026-11-05/reverify-local-results.json",".planning/milestones/v1.0-audit/evidence/2026-11-05/reverify-run.ps1",".planning/milestones/v1.0-audit/evidence/2026-11-05/reverify-workspace.log",".planning/phases/01-workspace-schema-lint/01-01-PLAN.md",".planning/phases/01-workspace-schema-lint/01-01-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-02-PLAN.md",".planning/phases/01-workspace-schema-lint/01-02-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-03-PLAN.md",".planning/phases/01-workspace-schema-lint/01-03-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-04-PLAN.md",".planning/phases/01-workspace-schema-lint/01-04-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-05-PLAN.md",".planning/phases/01-workspace-schema-lint/01-05-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-06-PLAN.md",".planning/phases/01-workspace-schema-lint/01-06-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-07-PLAN.md",".planning/phases/01-workspace-schema-lint/01-07-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-08-PLAN.md",".planning/phases/01-workspace-schema-lint/01-08-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-09-PLAN.md",".planning/phases/01-workspace-schema-lint/01-09-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-10-PLAN.md",".planning/phases/01-workspace-schema-lint/01-10-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-11-PLAN.md",".planning/phases/01-workspace-schema-lint/01-11-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-12-PLAN.md",".planning/phases/01-workspace-schema-lint/01-12-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-13-PLAN.md",".planning/phases/01-workspace-schema-lint/01-13-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-14-PLAN.md",".planning/phases/01-workspace-schema-lint/01-14-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-CONTEXT.md",".planning/phases/01-workspace-schema-lint/01-UAT.md",".planning/phases/01-workspace-schema-lint/01-VALIDATION.md",".vscode/settings.json",".zed/settings.json","CONTRIBUTING.md","Cargo.lock","Cargo.toml","ci/check-tls-deps.sh","ci/test_tls_deps.py","crates/ontoforge-expr/src/analyze.rs","crates/ontoforge-expr/src/check.rs","crates/ontoforge-expr/src/infer.rs","crates/ontoforge-expr/src/lib.rs","crates/ontoforge-expr/src/limits.rs","crates/ontoforge-expr/src/rowpred/eval.rs","crates/ontoforge-expr/src/rowpred/ir.rs","crates/ontoforge-expr/src/rowpred/mod.rs","crates/ontoforge-expr/src/rowpred/parse.rs","crates/ontoforge-expr/src/rowpred/value.rs","crates/ontoforge-expr/src/walk.rs","crates/ontoforge-expr/tests/analyze.rs","crates/ontoforge-expr/tests/criteria.rs","crates/ontoforge-schema/Cargo.toml","crates/ontoforge-schema/src/diag.rs","crates/ontoforge-schema/src/ir/action.rs","crates/ontoforge-schema/src/ir/attrs.rs","crates/ontoforge-schema/src/ir/basetype.rs","crates/ontoforge-schema/src/ir/binding.rs","crates/ontoforge-schema/src/ir/link.rs","crates/ontoforge-schema/src/ir/manifest.rs","crates/ontoforge-schema/src/ir/mod.rs","crates/ontoforge-schema/src/ir/names.rs","crates/ontoforge-schema/src/ir/object.rs","crates/ontoforge-schema/src/ir/scope.rs","crates/ontoforge-schema/src/lib.rs","crates/ontoforge-schema/src/lint/binding/e13.rs","crates/ontoforge-schema/src/lint/binding/e14.rs","crates/ontoforge-schema/src/lint/binding/e15.rs","crates/ontoforge-schema/src/lint/binding/e16.rs","crates/ontoforge-schema/src/lint/binding/mod.rs","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e13__tests__clean.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e13__tests__triggers.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e14__tests__clean.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e14__tests__triggers.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e15__tests__clean.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e15__tests__triggers.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e16__tests__clean.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e16__tests__triggers.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__w17__tests__clean.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__w17__tests__triggers.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__w17__tests__unreadable.snap","crates/ontoforge-schema/src/lint/binding/w17.rs","crates/ontoforge-schema/src/lint/config.rs","crates/ontoforge-schema/src/lint/e08.rs","crates/ontoforge-schema/src/lint/e09.rs","crates/ontoforge-schema/src/lint/mod.rs","crates/ontoforge-schema/src/lint/naming/mod.rs","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w01__tests__clean.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w01__tests__triggers.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w02__tests__clean.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w02__tests__triggers.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w03__tests__clean.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w03__tests__triggers.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w04__tests__clean.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w04__tests__triggers.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w05__tests__clean.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w05__tests__triggers.snap","crates/ontoforge-schema/src/lint/naming/w01.rs","crates/ontoforge-schema/src/lint/naming/w02.rs","crates/ontoforge-schema/src/lint/naming/w03.rs","crates/ontoforge-schema/src/lint/naming/w04.rs","crates/ontoforge-schema/src/lint/naming/w05.rs","crates/ontoforge-schema/src/lint/snapshots/ontoforge_schema__lint__e08__tests__clean.snap","crates/ontoforge-schema/src/lint/snapshots/ontoforge_schema__lint__e08__tests__triggers.snap","crates/ontoforge-schema/src/lint/snapshots/ontoforge_schema__lint__e09__tests__clean.snap","crates/ontoforge-schema/src/lint/snapshots/ontoforge_schema__lint__e09__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/mod.rs","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w06__tests__clean.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w06__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w07__tests__clean.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w07__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w08__tests__clean.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w08__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w09__tests__clean.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w09__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w10__tests__clean.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w10__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w11__tests__clean.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w11__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/w06.rs","crates/ontoforge-schema/src/lint/structure/w07.rs","crates/ontoforge-schema/src/lint/structure/w08.rs","crates/ontoforge-schema/src/lint/structure/w09.rs","crates/ontoforge-schema/src/lint/structure/w10.rs","crates/ontoforge-schema/src/lint/structure/w11.rs","crates/ontoforge-schema/src/lint/words.rs","crates/ontoforge-schema/src/loader/discover.rs","crates/ontoforge-schema/src/loader/dispatch.rs","crates/ontoforge-schema/src/loader/fixture.rs","crates/ontoforge-schema/src/loader/mod.rs","crates/ontoforge-schema/src/loader/parse.rs","crates/ontoforge-schema/src/loader/scope.rs","crates/ontoforge-schema/src/loader/sourcemap.rs","crates/ontoforge-schema/src/schema_export.rs","crates/ontoforge-schema/src/validate/actions.rs","crates/ontoforge-schema/src/validate/bindings.rs","crates/ontoforge-schema/src/validate/links.rs","crates/ontoforge-schema/src/validate/mod.rs","crates/ontoforge-schema/src/validate/objects.rs","crates/ontoforge-schema/tests/action_types.rs","crates/ontoforge-schema/tests/bindings.rs","crates/ontoforge-schema/tests/diagnostics.rs","crates/ontoforge-schema/tests/link_types.rs","crates/ontoforge-schema/tests/lint_examples.rs","crates/ontoforge-schema/tests/load_examples.rs","crates/ontoforge-schema/tests/loader.rs","crates/ontoforge-schema/tests/object_types.rs","crates/ontoforge-schema/tests/schema_export.rs","crates/ontoforge-schema/tests/scope_documents.rs","crates/ontoforge-schema/tests/snapshots/scope_documents__scope_diagnostics.snap","crates/ontoforge/Cargo.toml","crates/ontoforge/src/cli.rs","crates/ontoforge/src/lib.rs","crates/ontoforge/src/lint_cmd.rs","crates/ontoforge/src/main.rs","crates/ontoforge/src/pipeline.rs","crates/ontoforge/src/render.rs","crates/ontoforge/tests/cli.rs","crates/ontoforge/tests/cli_color.rs","crates/ontoforge/tests/cli_lint.rs","crates/ontoforge/tests/cli_schema.rs","crates/ontoforge/tests/invalid_examples.rs","crates/ontoforge/tests/snapshots/cli__validate_unknown_field_human.snap","crates/ontoforge/tests/snapshots/cli_lint__e08_human.snap","crates/ontoforge/tests/snapshots/cli_lint__e14_human.snap","crates/ontoforge/tests/snapshots/cli_online__online_missing_column_human.snap","crates/ontoforge/tests/snapshots/cli_online_trino__unavailable_human.snap","crates/ontoforge/tests/snapshots/invalid_examples__v01-json-trailing-comma.snap","crates/ontoforge/tests/snapshots/invalid_examples__v01-yaml-duplicate-key.snap","crates/ontoforge/tests/snapshots/invalid_examples__v02-missing-kind.snap","crates/ontoforge/tests/snapshots/invalid_examples__v03-function-rule.snap","crates/ontoforge/tests/snapshots/invalid_examples__v03-reserved-kind.snap","crates/ontoforge/tests/snapshots/invalid_examples__v04-unknown-field.snap","crates/ontoforge/tests/snapshots/invalid_examples__v05-missing-backing.snap","crates/ontoforge/tests/snapshots/invalid_examples__v05-missing-primary-key.snap","crates/ontoforge/tests/snapshots/invalid_examples__v06-precision-on-string.snap","crates/ontoforge/tests/snapshots/invalid_examples__v06-unknown-base-type.snap","crates/ontoforge/tests/snapshots/invalid_examples__v07-bad-object-type-name.snap","crates/ontoforge/tests/snapshots/invalid_examples__v08-duplicate-binding.snap","crates/ontoforge/tests/snapshots/invalid_examples__v08-duplicate-object-type.snap","crates/ontoforge/tests/snapshots/invalid_examples__v08-link-end-clashes-property.snap","crates/ontoforge/tests/snapshots/invalid_examples__v09-binding-unknown-target.snap","crates/ontoforge/tests/snapshots/invalid_examples__v09-link-unknown-object-type.snap","crates/ontoforge/tests/snapshots/invalid_examples__v09-rule-unknown-property.snap","crates/ontoforge/tests/snapshots/invalid_examples__v10-float-primary-key.snap","crates/ontoforge/tests/snapshots/invalid_examples__v10-primary-key-not-required.snap","crates/ontoforge/tests/snapshots/invalid_examples__v11-backing-on-one-to-many.snap","crates/ontoforge/tests/snapshots/invalid_examples__v11-foreign-key-type-mismatch.snap","crates/ontoforge/tests/snapshots/invalid_examples__v11-many-to-many-with-foreign-key.snap","crates/ontoforge/tests/snapshots/invalid_examples__v12-create-missing-primary-key.snap","crates/ontoforge/tests/snapshots/invalid_examples__v12-current-time-on-string.snap","crates/ontoforge/tests/snapshots/invalid_examples__v12-modify-assigns-primary-key.snap","crates/ontoforge/tests/snapshots/invalid_examples__v12-value-source-type-mismatch.snap","crates/ontoforge/tests/snapshots/invalid_examples__v13-comparison-type-mismatch.snap","crates/ontoforge/tests/snapshots/invalid_examples__v13-non-boolean-condition.snap","crates/ontoforge/tests/snapshots/invalid_examples__v13-objects-not-object-reference.snap","crates/ontoforge/tests/snapshots/invalid_examples__v13-syntax-error.snap","crates/ontoforge/tests/snapshots/invalid_examples__v13-unknown-parameter.snap","crates/ontoforge/tests/snapshots/invalid_examples__v14-include-escapes-root.snap","crates/ontoforge/tests/snapshots/invalid_examples__v16-binding-outside-bindings-dir.snap","crates/ontoforge/tests/snapshots/invalid_examples__v16-fixture-escapes-root.snap","crates/ontoforge/tests/snapshots/invalid_examples__v16-source-without-binding.snap","examples/ontologies/aviation-json/actions.json","examples/ontologies/aviation-json/bindings/dev/bindings.json","examples/ontologies/aviation-json/bindings/dev/fixtures/aircraft.csv","examples/ontologies/aviation-json/bindings/dev/fixtures/flight_crew.csv","examples/ontologies/aviation-json/bindings/dev/fixtures/flights.csv","examples/ontologies/aviation-json/bindings/dev/fixtures/maintenance_records.csv","examples/ontologies/aviation-json/bindings/dev/fixtures/pilots.csv","examples/ontologies/aviation-json/links.json","examples/ontologies/aviation-json/objects.json","examples/ontologies/aviation-json/ontology.yaml","examples/ontologies/aviation/actions.yaml","examples/ontologies/aviation/bindings/dev/aircraft.yaml","examples/ontologies/aviation/bindings/dev/fixtures/aircraft.csv","examples/ontologies/aviation/bindings/dev/fixtures/flight_crew.csv","examples/ontologies/aviation/bindings/dev/fixtures/flights.csv","examples/ontologies/aviation/bindings/dev/fixtures/maintenance_records.csv","examples/ontologies/aviation/bindings/dev/fixtures/pilots.csv","examples/ontologies/aviation/bindings/dev/flight-crew.yaml","examples/ontologies/aviation/bindings/dev/flight.yaml","examples/ontologies/aviation/bindings/dev/maintenance-record.yaml","examples/ontologies/aviation/bindings/dev/pilot.yaml","examples/ontologies/aviation/links.yaml","examples/ontologies/aviation/objects.yaml","examples/ontologies/aviation/ontology.yaml","examples/ontologies/invalid/v01-json-trailing-comma/objects.json","examples/ontologies/invalid/v01-json-trailing-comma/ontology.yaml","examples/ontologies/invalid/v01-yaml-duplicate-key/objects.yaml","examples/ontologies/invalid/v01-yaml-duplicate-key/ontology.yaml","examples/ontologies/invalid/v02-missing-kind/objects.yaml","examples/ontologies/invalid/v02-missing-kind/ontology.yaml","examples/ontologies/invalid/v03-function-rule/defs.yaml","examples/ontologies/invalid/v03-function-rule/ontology.yaml","examples/ontologies/invalid/v03-reserved-kind/objects.yaml","examples/ontologies/invalid/v03-reserved-kind/ontology.yaml","examples/ontologies/invalid/v04-unknown-field/objects.yaml","examples/ontologies/invalid/v04-unknown-field/ontology.yaml","examples/ontologies/invalid/v05-missing-backing/objects.yaml","examples/ontologies/invalid/v05-missing-backing/ontology.yaml","examples/ontologies/invalid/v05-missing-primary-key/objects.yaml","examples/ontologies/invalid/v05-missing-primary-key/ontology.yaml","examples/ontologies/invalid/v06-precision-on-string/objects.yaml","examples/ontologies/invalid/v06-precision-on-string/ontology.yaml","examples/ontologies/invalid/v06-unknown-base-type/objects.yaml","examples/ontologies/invalid/v06-unknown-base-type/ontology.yaml","examples/ontologies/invalid/v07-bad-object-type-name/objects.yaml","examples/ontologies/invalid/v07-bad-object-type-name/ontology.yaml","examples/ontologies/invalid/v08-duplicate-binding/bindings/dev/a.yaml","examples/ontologies/invalid/v08-duplicate-binding/bindings/dev/b.yaml","examples/ontologies/invalid/v08-duplicate-binding/objects.yaml","examples/ontologies/invalid/v08-duplicate-binding/ontology.yaml","examples/ontologies/invalid/v08-duplicate-object-type/a.yaml","examples/ontologies/invalid/v08-duplicate-object-type/b.yaml","examples/ontologies/invalid/v08-duplicate-object-type/ontology.yaml","examples/ontologies/invalid/v08-link-end-clashes-property/defs.yaml","examples/ontologies/invalid/v08-link-end-clashes-property/ontology.yaml","examples/ontologies/invalid/v09-binding-unknown-target/bindings/dev/flight.yaml","examples/ontologies/invalid/v09-binding-unknown-target/objects.yaml","examples/ontologies/invalid/v09-binding-unknown-target/ontology.yaml","examples/ontologies/invalid/v09-link-unknown-object-type/defs.yaml","examples/ontologies/invalid/v09-link-unknown-object-type/ontology.yaml","examples/ontologies/invalid/v09-rule-unknown-property/defs.yaml","examples/ontologies/invalid/v09-rule-unknown-property/ontology.yaml","examples/ontologies/invalid/v10-float-primary-key/objects.yaml","examples/ontologies/invalid/v10-float-primary-key/ontology.yaml","examples/ontologies/invalid/v10-primary-key-not-required/objects.yaml","examples/ontologies/invalid/v10-primary-key-not-required/ontology.yaml","examples/ontologies/invalid/v11-backing-on-one-to-many/defs.yaml","examples/ontologies/invalid/v11-backing-on-one-to-many/ontology.yaml","examples/ontologies/invalid/v11-foreign-key-type-mismatch/defs.yaml","examples/ontologies/invalid/v11-foreign-key-type-mismatch/ontology.yaml","examples/ontologies/invalid/v11-many-to-many-with-foreign-key/defs.yaml","examples/ontologies/invalid/v11-many-to-many-with-foreign-key/ontology.yaml","examples/ontologies/invalid/v12-create-missing-primary-key/defs.yaml","examples/ontologies/invalid/v12-create-missing-primary-key/ontology.yaml","examples/ontologies/invalid/v12-current-time-on-string/defs.yaml","examples/ontologies/invalid/v12-current-time-on-string/ontology.yaml","examples/ontologies/invalid/v12-modify-assigns-primary-key/defs.yaml","examples/ontologies/invalid/v12-modify-assigns-primary-key/ontology.yaml","examples/ontologies/invalid/v12-value-source-type-mismatch/defs.yaml","examples/ontologies/invalid/v12-value-source-type-mismatch/ontology.yaml","examples/ontologies/invalid/v13-comparison-type-mismatch/defs.yaml","examples/ontologies/invalid/v13-comparison-type-mismatch/ontology.yaml","examples/ontologies/invalid/v13-non-boolean-condition/defs.yaml","examples/ontologies/invalid/v13-non-boolean-condition/ontology.yaml","examples/ontologies/invalid/v13-objects-not-object-reference/defs.yaml","examples/ontologies/invalid/v13-objects-not-object-reference/ontology.yaml","examples/ontologies/invalid/v13-syntax-error/defs.yaml","examples/ontologies/invalid/v13-syntax-error/ontology.yaml","examples/ontologies/invalid/v13-unknown-parameter/defs.yaml","examples/ontologies/invalid/v13-unknown-parameter/ontology.yaml","examples/ontologies/invalid/v14-include-escapes-root/ontology.yaml","examples/ontologies/invalid/v16-binding-outside-bindings-dir/flight-binding.yaml","examples/ontologies/invalid/v16-binding-outside-bindings-dir/objects.yaml","examples/ontologies/invalid/v16-binding-outside-bindings-dir/ontology.yaml","examples/ontologies/invalid/v16-fixture-escapes-root/bindings/dev/flight.yaml","examples/ontologies/invalid/v16-fixture-escapes-root/objects.yaml","examples/ontologies/invalid/v16-fixture-escapes-root/ontology.yaml","examples/ontologies/invalid/v16-source-without-binding/bindings/dev/flight.yaml","examples/ontologies/invalid/v16-source-without-binding/objects.yaml","examples/ontologies/invalid/v16-source-without-binding/ontology.yaml","rust-toolchain.toml","schemas/ontoforge-definition.schema.json","schemas/ontoforge-manifest.schema.json","schemas/ontoforge-scope.schema.json","target/milestone-ci-41759804733-integration.log","target/milestone-ci-41759804733-ubuntu.log","target/milestone-ci-41759804733-windows.log"]
covered_digest: "v4:sha256:171fbe5a869367ec1e53a5adaa4fb4a0cbb2a16bbd7a6530ba37321025e902a2"
overrides:
  - must_have: "G-1-2：aviation 的任意 YAML document 输入 kind 可补全并标红未知字段"
    reason: "01-UAT Test 2 用户明确接受 pass；当前 UAT 记录仍保留未逐项观察的限制，因此沿用既有 override，不将其扩大解释为机器证据。"
    accepted_by: "user (01-UAT.md Test 2)"
    accepted_at: "2026-10-16T12:56:06Z"
re_verification:
  previous_status: human_needed
  previous_score: 11/11
  gaps_closed:
    - "01-UAT Test 4: 用户确认接受原创出处与禁止标识符边界。"
    - "01-UAT Test 5: 用户确认接受路径与用户名禁止项证据范围。"
    - "01-UAT Test 6: 用户确认 ONT-V 不得降级禁止项及具名 negative test 证据。"
  gaps_remaining: []
  regressions: []
prohibition_review:
  disposition: passed
  non_authoritative: false
  judgment_items: 3
  unresolved_human_items: 0
  evidence: ".planning/phases/01-workspace-schema-lint/01-UAT.md Tests 4-6"
  limitation: "三项 judgment 已有用户逐项确认；自动证据的边界仍按 UAT 原文保留。"
decision_coverage:
  honored: 18
  total: 18
  not_honored: []
---

# Phase 01: Workspace, Schema & Lint Verification Report

**Phase Goal:** As a ontology 开发者, I want to 用 YAML/JSON 编写 ontology，并通过 `ontoforge validate` / `ontoforge lint` 得到带源码定位的诊断，且仓库在 Windows 本机 `cargo test` 全绿, so that 我能在本机可靠地编写和验证定义.

**Verified:** 2026-11-07T07:46:42Z
**Status:** passed
**Re-verification:** Yes, after stale-verification regeneration and completed UAT review.

## User Flow Coverage (MVP)

| 用户步骤 | 期望 | 当前代码证据 | 状态 |
|---|---|---|---|
| 建立并编译 workspace | 原 Phase 1 的 10 个 `ontoforge-*` crate 使用 edition 2024 / Rust 1.98，Windows 本机无需 Docker 测试 | `Cargo.toml`、`rust-toolchain.toml`、`cargo metadata --locked`；原 10 个 crate 全部存在，当前第 11 个 `ontoforge-engine` 是后续 phase 增量；`cargo test --workspace --locked` exit 0 | VERIFIED |
| 编写 YAML/JSON 多文件 ontology | object type、base type、required、三种 link、action/CEL 合并为相同 IR | `loader::load_dir` / `load_from_memory`、`examples/ontologies/aviation` 与 `aviation-json`；当前 workspace 测试通过，CLI validate aviation 输出 `[]` | VERIFIED |
| 运行 `validate` 检查错误 | 诊断含错误码、相对文件名、line:col，退出码非 0 | 当前 CLI 反例输出 `ONT-V04`、`objects.yaml:3:1`、`primaryKey` help，exit 1；V13 反例输出 `ONT-V13`，exit 1 | VERIFIED |
| 运行 `lint` 配置规则 | E08/E09、W01-W11、配置严重级、JSON/人类可读输出可用 | 当前 registry、`lint_cmd`、配置解析与 CLI 测试；aviation `lint --deny-warnings --format json` 输出 `[]`，exit 0 | VERIFIED |
| 导出并使用 JSON Schema | 产物与 IR 同源，示例通过，典型错误被拒绝 | `schema_export.rs`、`schemas/*.json`、`ontoforge schema export`；本次导出两个文件均与入库文件逐字节一致 | VERIFIED |
| 在编辑器中编写多文档 YAML | 项目级 schema 关联、逐 document modeline、补全/错误提示 | `.zed/settings.json`、`.vscode/settings.json`、13 个 modeline 与回归测试；`01-UAT.md` Test 2 用户明确接受 pass，沿用既有 editor override | PASSED (override) |

**Score:** 102/102 accounted for: 101 VERIFIED + 1 PASSED (override). The accepted editor override is not machine verified.

## Roadmap Success Criteria

| # | Success Criterion | Status | Evidence |
|---|---|---|---|
| 1 | workspace、原 10 crate、edition 2024、Rust 1.98、Windows 无 Docker `cargo test --workspace` | VERIFIED | `cargo metadata --locked`；原 10 crate 保留；`cargo test --workspace --locked` exit 0；`cargo fmt --all --check` 与 locked Clippy exit 0 |
| 2 | 多文件 YAML/JSON 合并为单一 IR，覆盖 object、属性、link、action/CEL，YAML/JSON 等价 | VERIFIED | loader 实现与 `load_examples` / CLI 行为测试；aviation validate 与 JSON 版本均成功 |
| 3 | 错误有文件名与 line:col，非零退出 | VERIFIED | 当前 v04 CLI 输出 `ONT-V04`、`objects.yaml`、line 3 column 1、`primaryKey` help、exit 1；V13 反例同样 exit 1 |
| 4 | `lint` 覆盖 E 级与 W01-W11，支持双格式、退出码和规则配置 | VERIFIED | registry 注册 E08/E09/W01-W11；配置 V15 守卫；CLI aviation clean 与 workspace 测试通过 |
| 5 | JSON Schema 可导出，仓库示例通过 | VERIFIED | 本次 `schema export --out-dir` exit 0，两个导出文件与 `schemas/` 入库文件 identical |

## Required Artifacts

计划声明的 58 个 artifact 均通过 `verify.artifacts`，每个 artifact 均存在且无 stub issue。关键实现与 wiring 如下：

| Artifact / wiring | 当前证据 | 状态 |
|---|---|---|
| `loader/discover.rs`, `loader/parse.rs`, `loader/mod.rs` | include 过滤、严格 JSON、4 MiB 限制、32 MiB `spawn_scoped`、SourceMap | VERIFIED |
| `ir/object.rs`, `ir/link.rs`, `ir/action.rs`, `validate/*` | object/link/action IR 与 V05-V13 校验接线 | VERIFIED |
| `ontoforge-expr/src/{limits,analyze,walk,check,infer}.rs` | CEL 预检、隔离编译、作用域遍历、V13 引用/类型检查 | VERIFIED |
| `lint/{mod,config,e08,e09,naming,structure}.rs` | E08/E09/W01-W11 registry、配置和 Finding 定位 | VERIFIED |
| `schema_export.rs`, `schemas/*.json`, `.zed/settings.json`, `.vscode/settings.json` | schema 生成/写出、编辑器 schema 关联与逐 document modeline | VERIFIED |
| `crates/ontoforge/src/{main,lib,cli,pipeline,lint_cmd,render}.rs` | CLI validate/lint/schema 分派、输出和退出码 | VERIFIED |

## Key Link Verification

计划声明的 32 个 key link 全部通过 `verify.key-links`。关键链路：

| From | To | Via | Status |
|---|---|---|---|
| `ontoforge/src/main.rs` | `ontoforge/src/lib.rs` | 32 MiB worker thread 调用 `run` | WIRED |
| `ontoforge/src/pipeline.rs` | `ontoforge-schema/src/loader/mod.rs` | `load_dir` | WIRED |
| `ontoforge/src/pipeline.rs` | `ontoforge-expr/src/check.rs` | `check_ontology` | WIRED |
| `ontoforge/src/lint_cmd.rs` | `pipeline.rs` / schema lint | validate first, then config and rules | WIRED |
| `schema export` CLI | `schema_export::rendered_schemas` | writes both fixed artifacts | WIRED |
| editor configs/modelines | `schemas/ontoforge-definition.schema.json` | local schema paths and per-document modelines | WIRED |

## Data-Flow Trace (Level 4)

| Artifact | Data variable | Real source | Produces real data | Status |
|---|---|---|---|---|
| `loader` | ontology definitions | YAML/JSON files under the requested directory or `load_from_memory` inputs | Yes | FLOWING |
| `pipeline::check` | `Loaded` ontology and diagnostics | loader parse/dispatch/validate plus CEL analyzer | Yes | FLOWING |
| `lint_cmd::lint` | `Finding` / `Diagnostic` list | validated IR, `ontoforge.toml`, registered lint rules, SourceMap | Yes | FLOWING |
| `schema_export` | rendered schema JSON | `schemars` over current IR types | Yes | FLOWING |
| CLI renderer | human/JSON diagnostics | actual diagnostics and SourceMap source text | Yes | FLOWING |

## Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|---|---|---|---|
| Workspace tests | `cargo test --workspace --locked` | exit 0; all executed targets passed | PASS |
| Formatting | `cargo fmt --all --check` | exit 0 | PASS |
| Clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0, no warnings | PASS |
| Valid aviation ontology | `cargo run -q -p ontoforge -- validate examples/ontologies/aviation --format json` | exit 0, `[]` | PASS |
| Invalid field diagnostic | `cargo run -q -p ontoforge -- validate examples/ontologies/invalid/v04-unknown-field --format json` | exit 1, `ONT-V04`, `objects.yaml`, line 3 column 1, `primaryKey` help | PASS |
| Lint clean example | `cargo run -q -p ontoforge -- lint examples/ontologies/aviation --deny-warnings --format json` | exit 0, `[]` | PASS |
| CEL invalid reference | `cargo run -q -p ontoforge -- validate examples/ontologies/invalid/v13-unknown-parameter --format json` | exit 1, `ONT-V13`, `delayMinutes` suggestion | PASS |
| Deep YAML safety | `cargo test -p ontoforge-schema --test loader deeply_nested_yaml_is_rejected_not_crashing --locked` | 1 passed | PASS |
| Deep CEL safety | `cargo test -p ontoforge-expr --test analyze deeply_nested_expression_is_rejected_not_crashing --locked` | 1 passed | PASS |
| JSON Schema export | `cargo run -q -p ontoforge -- schema export --out-dir <temporary-dir>` | exit 0; both files byte-identical to `schemas/` | PASS |

## Probe Execution

N/A. No Phase 1 plan or summary declares a `scripts/*/tests/probe-*.sh` probe, and no conventional probe was found.

## Requirements Coverage

| Requirement | Status | Evidence |
|---|---|---|
| SCHEMA-01 | SATISFIED | loader multi-file/multi-document merge, aviation YAML/JSON equivalence, current CLI validation |
| SCHEMA-02 | SATISFIED | ObjectType fields and `effective_title_key`, object type tests |
| SCHEMA-03 | SATISFIED | all 13 base types, `required`, suitability and literal tests |
| SCHEMA-04 | SATISFIED | oneToOne/oneToMany/manyToMany IR and link validation tests |
| SCHEMA-05 | SATISFIED | action IR/rules/value sources and CEL V13 tests |
| SCHEMA-06 | SATISFIED | SourceMap, CLI line:col, V01-V13 fixtures and snapshots |
| SCHEMA-07 | SATISFIED | schema export, committed schema drift test, editor configuration/modelines; editor UX accepted by UAT override |
| LINT-01 | SATISFIED | E08/E09 behavior and nonzero error exit tests |
| LINT-02 | SATISFIED | W01-W11 registry, trigger/clean snapshots and aviation clean test |
| LINT-03 | SATISFIED | human and JSON rendering, ANSI/color tests, CLI lint tests |
| LINT-04 | SATISFIED | `ontoforge.toml`, `--config`, severity overrides and V15 rejection tests |
| DX-03 | SATISFIED | current Windows workspace test, fmt and locked Clippy gates; CI workflow covers Windows and Ubuntu |

No Phase 1 requirement is orphaned in `.planning/REQUIREMENTS.md`.

## Test Quality Audit

| Test scope | Active/skipped | Circular | Assertion strength | Verdict |
|---|---|---|---|---|
| `ontoforge-schema` loader/object/link/action/criteria/lint/schema tests | active; no disabled patterns found | no circular expected-value generation found | value and behavioral assertions | PASS |
| `ontoforge` CLI and invalid-example tests | active; no disabled patterns found | snapshots compare fixed expected diagnostics | behavioral/status/value assertions | PASS |
| `ontoforge-expr` analyze/criteria tests | active; no disabled patterns found | no circular fixture generation found | state-safety and value assertions | PASS |

No linked requirement is proved only by a disabled test. Test fixture writes use temporary directories and do not generate expected values from the system under test.

## Anti-Patterns Found

| File | Pattern | Severity | Impact |
|---|---|---|---|
| None | No unreferenced `TBD`, `FIXME`, or `XXX` markers in Phase 1 implementation inputs; no user-visible stub detected | Info | None |

The current codebase contains later-phase implementation files, but no Phase 1 blocker anti-pattern was found. The 11th workspace crate, `ontoforge-engine`, is a later-phase addition and does not remove any of the original 10 crates.

## Human Verification Required

None. The only deferred editor observation is represented by the accepted editor UAT override. The three judgment-tier prohibition items were explicitly confirmed in `01-UAT.md`:

- Test 4: `确认`
- Test 5: `接受`
- Test 6: `确认`

These confirmations close the previous `human_needed` routing without expanding the underlying automated evidence claims.

## Gaps Summary

No blocking gaps remain. The previous report's `human_needed` status was caused by three unresolved judgment-tier prohibition items. Current `01-UAT.md` records explicit user confirmations for all three, while the editor behavior remains transparently documented as the existing accepted override. All roadmap criteria and Phase 1 requirements are otherwise verified against current source and fresh Windows commands.

---

_Verified: 2026-11-07T07:46:42Z_
_Verifier: Claude (gsd-verifier)_
