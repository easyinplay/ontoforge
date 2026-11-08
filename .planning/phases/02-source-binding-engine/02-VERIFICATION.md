---
phase: 02-source-binding-engine
verified: 2026-11-08T09:52:58Z
source_head: 54e13c8d303438514fd76c9c194fbbb07538f934
status: passed
score: 109/109 must-haves verified
covered_files: [".github/workflows/ci.yml", ".github/workflows/nightly.yml", ".planning/phases/02-source-binding-engine/02-01-PLAN.md", ".planning/phases/02-source-binding-engine/02-01-SUMMARY.md", ".planning/phases/02-source-binding-engine/02-02-PLAN.md", ".planning/phases/02-source-binding-engine/02-02-SUMMARY.md", ".planning/phases/02-source-binding-engine/02-03-PLAN.md", ".planning/phases/02-source-binding-engine/02-03-SUMMARY.md", ".planning/phases/02-source-binding-engine/02-04-PLAN.md", ".planning/phases/02-source-binding-engine/02-04-SUMMARY.md", ".planning/phases/02-source-binding-engine/02-05-PLAN.md", ".planning/phases/02-source-binding-engine/02-05-SUMMARY.md", ".planning/phases/02-source-binding-engine/02-06-PLAN.md", ".planning/phases/02-source-binding-engine/02-06-SUMMARY.md", ".planning/phases/02-source-binding-engine/02-07-PLAN.md", ".planning/phases/02-source-binding-engine/02-07-SUMMARY.md", ".planning/phases/02-source-binding-engine/02-08-PLAN.md", ".planning/phases/02-source-binding-engine/02-08-SUMMARY.md", ".planning/phases/02-source-binding-engine/02-09-PLAN.md", ".planning/phases/02-source-binding-engine/02-09-SUMMARY.md", ".planning/phases/02-source-binding-engine/02-10-PLAN.md", ".planning/phases/02-source-binding-engine/02-10-SUMMARY.md", ".planning/phases/02-source-binding-engine/02-11-HUMAN-ACCEPTANCE.md", ".planning/phases/02-source-binding-engine/02-11-PLAN.md", ".planning/phases/02-source-binding-engine/02-11-RECONCILIATION.md", ".planning/phases/02-source-binding-engine/02-11-SUMMARY.md", ".planning/phases/02-source-binding-engine/02-11-source-scan.txt", ".planning/phases/02-source-binding-engine/02-REVIEW-DISPOSITION.md", ".planning/phases/02-source-binding-engine/02-UAT.md", "CONTRIBUTING.md", "Cargo.lock", "Cargo.toml", "ci/check-tls-deps.sh", "ci/ontoforge-server.ci.toml", "ci/pg/iceberg_catalog.sql", "ci/pg/query_corpus.sql", "ci/pg/rowpred_corpus.sql", "ci/pg/seed.sql", "ci/query-services.sh", "ci/release-package.ps1", "ci/release-smoke.ps1", "ci/server-services.sh", "ci/test_tls_deps.py", "ci/test_windows_server.py", "ci/trino/catalog/demo.properties", "ci/trino/catalog/editstore.properties", "ci/trino/catalog/tpch.properties", "ci/trino/iceberg/iceberg.properties", "ci/trino/l2/access-control.properties", "ci/trino/l2/config.properties", "ci/trino/l2/rules.json", "ci/trino/l2/seed.sql", "ci/windows-server.py", "clippy.toml", "crates/ontoforge-engine/Cargo.toml", "crates/ontoforge-engine/src/bind_check.rs", "crates/ontoforge-engine/src/bind_check/compat_tests.rs", "crates/ontoforge-engine/src/bind_check/tests.rs", "crates/ontoforge-engine/src/config.rs", "crates/ontoforge-engine/src/error.rs", "crates/ontoforge-engine/src/lib.rs", "crates/ontoforge-engine/src/pk_check.rs", "crates/ontoforge-engine/src/printer/ast.rs", "crates/ontoforge-engine/src/printer/call.rs", "crates/ontoforge-engine/src/printer/dialect.rs", "crates/ontoforge-engine/src/printer/escape.rs", "crates/ontoforge-engine/src/printer/l2.rs", "crates/ontoforge-engine/src/printer/mod.rs", "crates/ontoforge-engine/src/printer/snapshots/ontoforge_engine__printer__tests__print_sqlite.snap", "crates/ontoforge-engine/src/printer/snapshots/ontoforge_engine__printer__tests__print_trino.snap", "crates/ontoforge-engine/src/printer/sqlite_ddl.rs", "crates/ontoforge-engine/src/printer/tests.rs", "crates/ontoforge-engine/src/sqlite/fixture.rs", "crates/ontoforge-engine/src/sqlite/mod.rs", "crates/ontoforge-engine/src/sqlite/tests.rs", "crates/ontoforge-engine/src/trino/cancel.rs", "crates/ontoforge-engine/src/trino/classify.rs", "crates/ontoforge-engine/src/trino/client.rs", "crates/ontoforge-engine/src/trino/decode.rs", "crates/ontoforge-engine/src/trino/headers.rs", "crates/ontoforge-engine/src/trino/l2_health.rs", "crates/ontoforge-engine/src/trino/l2_monitor.rs", "crates/ontoforge-engine/src/trino/mod.rs", "crates/ontoforge-engine/src/value.rs", "crates/ontoforge-engine/tests/bind_check_isolation.rs", "crates/ontoforge-engine/tests/data/l2_corpus.json", "crates/ontoforge-engine/tests/data/test-ca.pem", "crates/ontoforge-engine/tests/doc_type_table.rs", "crates/ontoforge-engine/tests/drift.rs", "crates/ontoforge-engine/tests/drift_budget.rs", "crates/ontoforge-engine/tests/l2_explain.rs", "crates/ontoforge-engine/tests/l2_gate.rs", "crates/ontoforge-engine/tests/l2_health.rs", "crates/ontoforge-engine/tests/l2_monitor.rs", "crates/ontoforge-engine/tests/pk_check.rs", "crates/ontoforge-engine/tests/printer_literals.rs", "crates/ontoforge-engine/tests/printer_prop.proptest-regressions", "crates/ontoforge-engine/tests/printer_prop.rs", "crates/ontoforge-engine/tests/server_config.rs", "crates/ontoforge-engine/tests/snapshots/pk_check__pk_duplicates_sqlite.snap", "crates/ontoforge-engine/tests/snapshots/pk_check__pk_duplicates_trino.snap", "crates/ontoforge-engine/tests/snapshots/pk_check__pk_nulls_sqlite.snap", "crates/ontoforge-engine/tests/snapshots/pk_check__pk_nulls_trino.snap", "crates/ontoforge-engine/tests/sql_text_only_from_printer.rs", "crates/ontoforge-engine/tests/sqlite_engine.rs", "crates/ontoforge-engine/tests/trino_client.rs", "crates/ontoforge-engine/tests/trino_it.rs", "crates/ontoforge-engine/tests/trino_l2_acl_it.rs", "crates/ontoforge-engine/tests/trino_resilience.rs", "crates/ontoforge-schema/Cargo.toml", "crates/ontoforge-schema/src/diag.rs", "crates/ontoforge-schema/src/ir/action.rs", "crates/ontoforge-schema/src/ir/attrs.rs", "crates/ontoforge-schema/src/ir/basetype.rs", "crates/ontoforge-schema/src/ir/binding.rs", "crates/ontoforge-schema/src/ir/link.rs", "crates/ontoforge-schema/src/ir/manifest.rs", "crates/ontoforge-schema/src/ir/mod.rs", "crates/ontoforge-schema/src/ir/names.rs", "crates/ontoforge-schema/src/ir/object.rs", "crates/ontoforge-schema/src/ir/scope.rs", "crates/ontoforge-schema/src/lib.rs", "crates/ontoforge-schema/src/lint/binding/e13.rs", "crates/ontoforge-schema/src/lint/binding/e14.rs", "crates/ontoforge-schema/src/lint/binding/e15.rs", "crates/ontoforge-schema/src/lint/binding/e16.rs", "crates/ontoforge-schema/src/lint/binding/mod.rs", "crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e13__tests__clean.snap", "crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e13__tests__triggers.snap", "crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e14__tests__clean.snap", "crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e14__tests__triggers.snap", "crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e15__tests__clean.snap", "crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e15__tests__triggers.snap", "crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e16__tests__clean.snap", "crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e16__tests__triggers.snap", "crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__w17__tests__clean.snap", "crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__w17__tests__triggers.snap", "crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__w17__tests__unreadable.snap", "crates/ontoforge-schema/src/lint/binding/w17.rs", "crates/ontoforge-schema/src/lint/config.rs", "crates/ontoforge-schema/src/lint/e08.rs", "crates/ontoforge-schema/src/lint/e09.rs", "crates/ontoforge-schema/src/lint/mod.rs", "crates/ontoforge-schema/src/lint/naming/mod.rs", "crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w01__tests__clean.snap", "crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w01__tests__triggers.snap", "crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w02__tests__clean.snap", "crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w02__tests__triggers.snap", "crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w03__tests__clean.snap", "crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w03__tests__triggers.snap", "crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w04__tests__clean.snap", "crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w04__tests__triggers.snap", "crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w05__tests__clean.snap", "crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w05__tests__triggers.snap", "crates/ontoforge-schema/src/lint/naming/w01.rs", "crates/ontoforge-schema/src/lint/naming/w02.rs", "crates/ontoforge-schema/src/lint/naming/w03.rs", "crates/ontoforge-schema/src/lint/naming/w04.rs", "crates/ontoforge-schema/src/lint/naming/w05.rs", "crates/ontoforge-schema/src/lint/snapshots/ontoforge_schema__lint__e08__tests__clean.snap", "crates/ontoforge-schema/src/lint/snapshots/ontoforge_schema__lint__e08__tests__triggers.snap", "crates/ontoforge-schema/src/lint/snapshots/ontoforge_schema__lint__e09__tests__clean.snap", "crates/ontoforge-schema/src/lint/snapshots/ontoforge_schema__lint__e09__tests__triggers.snap", "crates/ontoforge-schema/src/lint/structure/mod.rs", "crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w06__tests__clean.snap", "crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w06__tests__triggers.snap", "crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w07__tests__clean.snap", "crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w07__tests__triggers.snap", "crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w08__tests__clean.snap", "crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w08__tests__triggers.snap", "crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w09__tests__clean.snap", "crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w09__tests__triggers.snap", "crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w10__tests__clean.snap", "crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w10__tests__triggers.snap", "crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w11__tests__clean.snap", "crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w11__tests__triggers.snap", "crates/ontoforge-schema/src/lint/structure/w06.rs", "crates/ontoforge-schema/src/lint/structure/w07.rs", "crates/ontoforge-schema/src/lint/structure/w08.rs", "crates/ontoforge-schema/src/lint/structure/w09.rs", "crates/ontoforge-schema/src/lint/structure/w10.rs", "crates/ontoforge-schema/src/lint/structure/w11.rs", "crates/ontoforge-schema/src/lint/words.rs", "crates/ontoforge-schema/src/loader/discover.rs", "crates/ontoforge-schema/src/loader/dispatch.rs", "crates/ontoforge-schema/src/loader/fixture.rs", "crates/ontoforge-schema/src/loader/mod.rs", "crates/ontoforge-schema/src/loader/parse.rs", "crates/ontoforge-schema/src/loader/scope.rs", "crates/ontoforge-schema/src/loader/sourcemap.rs", "crates/ontoforge-schema/src/schema_export.rs", "crates/ontoforge-schema/src/validate/actions.rs", "crates/ontoforge-schema/src/validate/bindings.rs", "crates/ontoforge-schema/src/validate/links.rs", "crates/ontoforge-schema/src/validate/mod.rs", "crates/ontoforge-schema/src/validate/objects.rs", "crates/ontoforge-schema/tests/action_types.rs", "crates/ontoforge-schema/tests/bindings.rs", "crates/ontoforge-schema/tests/diagnostics.rs", "crates/ontoforge-schema/tests/link_types.rs", "crates/ontoforge-schema/tests/lint_examples.rs", "crates/ontoforge-schema/tests/load_examples.rs", "crates/ontoforge-schema/tests/loader.rs", "crates/ontoforge-schema/tests/object_types.rs", "crates/ontoforge-schema/tests/schema_export.rs", "crates/ontoforge-schema/tests/scope_documents.rs", "crates/ontoforge-schema/tests/snapshots/scope_documents__scope_diagnostics.snap", "crates/ontoforge/src/cli.rs", "crates/ontoforge/src/dev.rs", "crates/ontoforge/src/dev_assets.rs", "crates/ontoforge/src/dev_token.rs", "crates/ontoforge/src/import.rs", "crates/ontoforge/src/lib.rs", "crates/ontoforge/src/lint_cmd.rs", "crates/ontoforge/src/main.rs", "crates/ontoforge/src/mcp.rs", "crates/ontoforge/src/migrate.rs", "crates/ontoforge/src/online.rs", "crates/ontoforge/src/pipeline.rs", "crates/ontoforge/src/reload.rs", "crates/ontoforge/src/render.rs", "crates/ontoforge/src/scope.rs", "crates/ontoforge/src/serve.rs", "crates/ontoforge/src/webhook_worker.rs", "crates/ontoforge/tests/cli.rs", "crates/ontoforge/tests/cli_check_pk.rs", "crates/ontoforge/tests/cli_color.rs", "crates/ontoforge/tests/cli_dev_token.rs", "crates/ontoforge/tests/cli_import.rs", "crates/ontoforge/tests/cli_l2_identity.rs", "crates/ontoforge/tests/cli_lint.rs", "crates/ontoforge/tests/cli_migrate.rs", "crates/ontoforge/tests/cli_online.rs", "crates/ontoforge/tests/cli_online_trino.rs", "crates/ontoforge/tests/cli_reload.rs", "crates/ontoforge/tests/cli_schema.rs", "crates/ontoforge/tests/cli_scope.rs", "crates/ontoforge/tests/cli_shutdown.rs", "crates/ontoforge/tests/cli_stdio.rs", "crates/ontoforge/tests/cli_trino_it.rs", "crates/ontoforge/tests/cli_webhook_worker.rs", "crates/ontoforge/tests/dev_lifecycle.rs", "crates/ontoforge/tests/invalid_examples.rs", "crates/ontoforge/tests/private_fixtures.rs", "crates/ontoforge/tests/snapshots/cli__validate_unknown_field_human.snap", "crates/ontoforge/tests/snapshots/cli_lint__e08_human.snap", "crates/ontoforge/tests/snapshots/cli_lint__e14_human.snap", "crates/ontoforge/tests/snapshots/cli_online__online_missing_column_human.snap", "crates/ontoforge/tests/snapshots/cli_online_trino__unavailable_human.snap", "crates/ontoforge/tests/snapshots/invalid_examples__v01-json-trailing-comma.snap", "crates/ontoforge/tests/snapshots/invalid_examples__v01-yaml-duplicate-key.snap", "crates/ontoforge/tests/snapshots/invalid_examples__v02-missing-kind.snap", "crates/ontoforge/tests/snapshots/invalid_examples__v03-function-rule.snap", "crates/ontoforge/tests/snapshots/invalid_examples__v03-reserved-kind.snap", "crates/ontoforge/tests/snapshots/invalid_examples__v04-unknown-field.snap", "crates/ontoforge/tests/snapshots/invalid_examples__v05-missing-backing.snap", "crates/ontoforge/tests/snapshots/invalid_examples__v05-missing-primary-key.snap", "crates/ontoforge/tests/snapshots/invalid_examples__v06-precision-on-string.snap", "crates/ontoforge/tests/snapshots/invalid_examples__v06-unknown-base-type.snap", "crates/ontoforge/tests/snapshots/invalid_examples__v07-bad-object-type-name.snap", "crates/ontoforge/tests/snapshots/invalid_examples__v08-duplicate-binding.snap", "crates/ontoforge/tests/snapshots/invalid_examples__v08-duplicate-object-type.snap", "crates/ontoforge/tests/snapshots/invalid_examples__v08-link-end-clashes-property.snap", "crates/ontoforge/tests/snapshots/invalid_examples__v09-binding-unknown-target.snap", "crates/ontoforge/tests/snapshots/invalid_examples__v09-link-unknown-object-type.snap", "crates/ontoforge/tests/snapshots/invalid_examples__v09-rule-unknown-property.snap", "crates/ontoforge/tests/snapshots/invalid_examples__v10-float-primary-key.snap", "crates/ontoforge/tests/snapshots/invalid_examples__v10-primary-key-not-required.snap", "crates/ontoforge/tests/snapshots/invalid_examples__v11-backing-on-one-to-many.snap", "crates/ontoforge/tests/snapshots/invalid_examples__v11-foreign-key-type-mismatch.snap", "crates/ontoforge/tests/snapshots/invalid_examples__v11-many-to-many-with-foreign-key.snap", "crates/ontoforge/tests/snapshots/invalid_examples__v12-create-missing-primary-key.snap", "crates/ontoforge/tests/snapshots/invalid_examples__v12-current-time-on-string.snap", "crates/ontoforge/tests/snapshots/invalid_examples__v12-modify-assigns-primary-key.snap", "crates/ontoforge/tests/snapshots/invalid_examples__v12-value-source-type-mismatch.snap", "crates/ontoforge/tests/snapshots/invalid_examples__v13-comparison-type-mismatch.snap", "crates/ontoforge/tests/snapshots/invalid_examples__v13-non-boolean-condition.snap", "crates/ontoforge/tests/snapshots/invalid_examples__v13-objects-not-object-reference.snap", "crates/ontoforge/tests/snapshots/invalid_examples__v13-syntax-error.snap", "crates/ontoforge/tests/snapshots/invalid_examples__v13-unknown-parameter.snap", "crates/ontoforge/tests/snapshots/invalid_examples__v14-include-escapes-root.snap", "crates/ontoforge/tests/snapshots/invalid_examples__v16-binding-outside-bindings-dir.snap", "crates/ontoforge/tests/snapshots/invalid_examples__v16-fixture-escapes-root.snap", "crates/ontoforge/tests/snapshots/invalid_examples__v16-source-without-binding.snap", "crates/ontoforge/tests/stdio_security_support/mod.rs", "crates/ontoforge/tests/unit/dev.test.rs", "crates/ontoforge/tests/unit/serve.test.rs", "docs/source-binding.md", "examples/ontologies/aviation-json/actions.json", "examples/ontologies/aviation-json/bindings/dev/bindings.json", "examples/ontologies/aviation-json/bindings/dev/fixtures/aircraft.csv", "examples/ontologies/aviation-json/bindings/dev/fixtures/flight_crew.csv", "examples/ontologies/aviation-json/bindings/dev/fixtures/flights.csv", "examples/ontologies/aviation-json/bindings/dev/fixtures/maintenance_records.csv", "examples/ontologies/aviation-json/bindings/dev/fixtures/pilots.csv", "examples/ontologies/aviation-json/links.json", "examples/ontologies/aviation-json/objects.json", "examples/ontologies/aviation-json/ontology.yaml", "examples/ontologies/aviation/actions.yaml", "examples/ontologies/aviation/bindings/dev/aircraft.yaml", "examples/ontologies/aviation/bindings/dev/fixtures/aircraft.csv", "examples/ontologies/aviation/bindings/dev/fixtures/flight_crew.csv", "examples/ontologies/aviation/bindings/dev/fixtures/flights.csv", "examples/ontologies/aviation/bindings/dev/fixtures/maintenance_records.csv", "examples/ontologies/aviation/bindings/dev/fixtures/pilots.csv", "examples/ontologies/aviation/bindings/dev/flight-crew.yaml", "examples/ontologies/aviation/bindings/dev/flight.yaml", "examples/ontologies/aviation/bindings/dev/maintenance-record.yaml", "examples/ontologies/aviation/bindings/dev/pilot.yaml", "examples/ontologies/aviation/links.yaml", "examples/ontologies/aviation/objects.yaml", "examples/ontologies/aviation/ontology.yaml", "schemas/ontoforge-definition.schema.json", "schemas/ontoforge-manifest.schema.json", "schemas/ontoforge-scope.schema.json"]
covered_digest: "v4:sha256:c06dfae7a71bdcfaa43fd42f25c80e61e42d13542d53137a0cd88a8a67c3bcfa"
behavior_unverified: 0
overrides_applied: 2
overrides:
  - must_have: "配置把 profile 映射到 Trino 而绑定带 `fixture` 时，对每个这样的绑定给 ONT-V17 warning（fixture 被忽略），绝不改用 SQLite 替身"
    reason: "后继 accepted ADR-0022 明确将 ignored fixture 诊断从 warning 改为 Info；02-UAT 测试6记录用户已选方案(a)"
    accepted_by: "用户（历史02-UAT测试6；ADR-0022）"
    accepted_at: "2026-10-19T00:00:00Z"
  - must_have: "`ontoforge validate --online --check-pk --server-config ci/ontoforge-server.ci.toml examples/ontologies/aviation` 在 CI 的 Trino 上退出码 0（只有 fixture 被忽略的 warning）"
    reason: "同一 accepted ADR-0022 的后继诊断合同；真实CLI保持exit0且fixture被忽略，诊断级别为Info"
    accepted_by: "用户（历史02-UAT测试6；ADR-0022）"
    accepted_at: "2026-10-19T00:00:00Z"
re_verification:
  previous_status: human_needed
  previous_score: 107/109 must-haves verified
  gaps_closed:
    - "G-02-B（02-03 judgment 禁止项「公开仓库中的示例、绑定、fixture、测试与 CI 种子不得出现公司物理名或真实业务数据」）：2026-11-08 人工签字，`02-11-HUMAN-ACCEPTANCE.md:45` 为 `VERDICT: ACCEPTED（2026-11-08 人工签字）`，理由见 `02-UAT.md` 第 7 项 `note:`；本轮复核机器可判定部分仍成立（45 个物理标识符与中性词表双向逐行相等、`## SUSPECTED NON-NEUTRAL` 为空、新一轮独立扫描未发现新增可疑物理名）"
    - "G-02-C（02-10 judgment 禁止项「公开 CI 不得连接任何公司 Trino 集群，也不得在工作流、日志或种子中出现公司 catalog / 表名 / 数据」）：2026-11-08 人工签字，`02-11-HUMAN-ACCEPTANCE.md:62` 为 `VERDICT: ACCEPTED（2026-11-08 人工签字）`，理由见 `02-UAT.md` 第 8 项 `note:`；本轮复核门控 C-1 仍 PASS（`.github/` 内 `private_fixtures` / `ONTOFORGE_PRIVATE_FIXTURES` 零命中），端点串仍全为 loopback / 容器内 `minio` / 仓库内桶前缀"
    - "G-02-D（02-10.8 / ENGINE-05 私有 Trino 数据源边界）：2026-11-08 人工签字，`02-11-HUMAN-ACCEPTANCE.md:81` 为 `VERDICT: ACCEPTED（2026-11-08 人工签字）`，理由见 `02-UAT.md` 第 9 项 `note:`（通用表述依 ADR-0032）；本轮复核门控 D-1（`private_fixtures.rs:14/21/69` 环境变量 + `#[ignore]` 双门控）与 D-2（`CONTRIBUTING.md:89/91`）仍 PASS，公开 workflow 不运行该 target"
  gaps_remaining: []
  regressions: []
resolved_prohibitions:
  - plan: "02-03"
    statement: "公开仓库中的示例、绑定、fixture、测试与CI种子数据不得出现任何公司的 catalog / schema / 表名或真实业务数据"
    verification: judgment
    verdict: "ACCEPTED（2026-11-08 人工签字）— G-02-B；机器可判定部分独立复核通过（45 标识符全中性、`SUSPECTED NON-NEUTRAL` 为空、本轮新增扫描无漂移）；结论由人工给出，不再标记 unverified-prohibition"
    accepted_by: "用户（02-UAT.md 第 7 项）"
    accepted_at: "2026-11-08"
    evidence: ".planning/phases/02-source-binding-engine/02-11-HUMAN-ACCEPTANCE.md:45；.planning/phases/02-source-binding-engine/02-UAT.md:45-51"
  - plan: "02-10"
    statement: "公开CI不得连接任何公司Trino集群，也不得在工作流、日志或种子中出现公司catalog/表名/数据"
    verification: judgment
    verdict: "ACCEPTED（2026-11-08 人工签字）— G-02-C；机器可判定部分独立复核通过（门控 C-1 退出码 1、7 条端点串无公司集群地址）；结论由人工给出，不再标记 unverified-prohibition"
    accepted_by: "用户（02-UAT.md 第 8 项）"
    accepted_at: "2026-11-08"
    evidence: ".planning/phases/02-source-binding-engine/02-11-HUMAN-ACCEPTANCE.md:62；.planning/phases/02-source-binding-engine/02-UAT.md:53-59"
human_accepted_judgments:
  - id: "G-02-B"
    test: "核对公开仓库的示例 ontology、绑定文件、CI 种子与测试夹具，确认其数据（不只是标识符）确为可公开的虚构数据，不含任何公司业务数据"
    verdict: "ACCEPTED（2026-11-08 人工签字）"
    reason: "人工接受：公开仓库的示例 ontology、绑定文件、CI 种子与测试夹具数据均为可公开虚构数据，不含公司业务数据"
    accepted_by: "用户"
    accepted_at: "2026-11-08"
    evidence: "02-11-HUMAN-ACCEPTANCE.md:45（VERDICT 行）；02-UAT.md:50（note 行）；机器可判定部分见 02-11-source-scan.txt:25/:217 与 02-11-RECONCILIATION.md:91"
  - id: "G-02-C"
    test: "核对公开 CI 与公司源的隔离，确认部署环境、runner 配置与私有 fixture 目录均未被公开 workflow 触达"
    verdict: "ACCEPTED（2026-11-08 人工签字）"
    reason: "人工接受：公开 workflow 不连接公司 Trino 集群、不读取公司 catalog / 表名 / 数据"
    accepted_by: "用户"
    accepted_at: "2026-11-08"
    evidence: "02-11-HUMAN-ACCEPTANCE.md:62（VERDICT 行）；02-UAT.md:58（note 行）；机器可判定部分见 02-11-source-scan.txt:158-165（门控 C-1 RESULT: PASS）"
  - id: "G-02-D"
    test: "核对私有 Trino 数据源边界与仓库外目录的真实内容，确认仓库外目录当前不含公司绑定数据、且 `ONTOFORGE_PRIVATE_FIXTURES` 入口的约定与实际部署一致"
    verdict: "ACCEPTED（2026-11-08 人工签字）"
    reason: "人工接受（通用表述见 ADR-0032）：私有环境下的 Trino 数据源只经仓库外私有 fixture 验证，公开 CI 不运行该 target，仓库内无任何私有绑定或数据"
    accepted_by: "用户"
    accepted_at: "2026-11-08"
    evidence: "02-11-HUMAN-ACCEPTANCE.md:81（VERDICT 行）；02-UAT.md:66（note 行）；机器可判定部分见 crates/ontoforge/tests/private_fixtures.rs:14/21/69 与 CONTRIBUTING.md:89/91"
advisory:
  - finding: "ROADMAP.md 的 Phase 2 计划计数已由 `10/10 plans executed` 更新为 `11/11 plans complete`（由 `gsd_run query phase.complete 02` 派生），但逐条计划清单仍只列 02-01..02-10，未单列 02-11 这份 gap-closure plan"
    category: other
    reason: "ROADMAP.md 不在本轮写入范围；不影响 goal 判定，登记以备后续补记"
    evidence_status: "confirmed（gsd-tools query roadmap.get-phase 2 输出）"
  - finding: "ADR-0032 的「私有数据源不按数据库类型单列」通用化只落到 planning 层；产品层仍点名 SQL Server：`crates/ontoforge/tests/private_fixtures.rs:1`（模块注释）、`CONTRIBUTING.md:85`（小节标题）、`docs/designs/trino-engine-sql-tiers.md:339,374`"
    category: other
    reason: "ADR-0032 本身只要求 ENGINE-05 / 02-11-HUMAN-ACCEPTANCE.md / 02-VERIFICATION.md 三处改为通用表述；产品代码与已跟踪 docs 不在本轮写入范围。代码里确实不存在按数据库类型分叉的路径（门控条件是「是否私有环境」），故不构成功能缺陷"
    evidence_status: "confirmed（grep 全树命中点逐条列出）"
  - finding: "`02-10-SUMMARY.md` 的若干陈旧断言仍在（本地 warehouse / `fs.local.enabled` 描述、ignored 与本机测试计数 `573 passed / 0 failed / 11 ignored`）；PR `integration` 侧编排迁移未被 02-11 的偏差台账登记（review 的 WR-03 / WR-06 / WR-08）"
    category: other
    reason: "历史 SUMMARY 产物不在本轮写入范围；本报告以当前源码事实取代其结论"
    evidence_status: "confirmed（上一轮已核对，本轮产品代码未变，结论仍成立）"
  - finding: "`02-11-SUMMARY.md` 把 7 条端点串概括为「全部指向 loopback / 容器内 minio / 仓库内桶前缀」，未提 `host.docker.internal`；且「13 条 G-A1..G-A5」的计数与门禁编号表述不自洽（review 的 IN-01..IN-03）"
    category: other
    reason: "概括措辞不够精确；底层证据文件 `02-11-source-scan.txt` 的 `==== ENDPOINTS ====` 补充节已如实登记 `host.docker.internal`（`ci/query-services.sh:64`，Docker 标准宿主机名，指向回环）"
    evidence_status: "confirmed"
  - finding: "`02-REVIEW.md:40` 的审查约束写着「三处 `VERDICT: PENDING` 必须仍为 `PENDING`」——该约束描述的是签字前的状态，已被 2026-11-08 的三处人工签字取代"
    category: other
    reason: "review 记录反映签字前的事实，不需追改；仅提示读者不要据此认为状态矛盾"
    evidence_status: "confirmed"
  - finding: "工作树中存在未跟踪的 `docs/README.md`、`docs/foundry/`、`docs/images/`（Palantir 公开文档的本地镜像，含 199 页 markdown + 357 张图片）"
    category: other
    reason: "未跟踪文件不在本仓库已提交面、不属于 Phase 02 交付物、不在本次指纹覆盖集内；内容为公开站点镜像，无公司数据。仅作提示：若将来提交需先确认许可证与体积"
    evidence_status: "confirmed（`docs/README.md` 自述为公开站点逐字镜像，fetched 2026-10-08 / 2026-10-15）"

post_verification_bookkeeping:
  - file: ".planning/phases/02-source-binding-engine/02-11-HUMAN-ACCEPTANCE.md"
    change: "frontmatter only: `status: pending-human-review` → `status: accepted`，并新增 `accepted: 2026-11-08`；三处 `VERDICT: ACCEPTED（2026-11-08 人工签字）` 结论行与正文其余内容逐字未变（因 frontmatter 增行，结论行号由 :44 / :61 / :80 移至 :45 / :62 / :81）"
    effect_on_verdict: "none — 结论位内容与签字位置未变"
  - file: ".planning/phases/02-source-binding-engine/02-REVIEW-DISPOSITION.md"
    change: "正文汇总表 CR-01 / WR-01 / WR-02 由 `open` 改为 `fixed`；frontmatter 计数块 `open: 13` → `fixed: 3` / `open: 10`（`total: 13` 不变）；无 finding 增删"
    effect_on_verdict: "none — 台账与 frontmatter 对齐，CR-01 的修复内容本轮已独立复核为真"
  - file: ".planning/REQUIREMENTS.md"
    change: "ENGINE-05 勾选框 `[ ]` → `[x]`，:200 的 traceability 由 `Partial: ...` 改为 `Complete`（该文件不在本报告覆盖集内，属并行记账）"
    effect_on_verdict: "none — 与本报告 ENGINE-05 = SATISFIED 的判定方向一致"
---

# Phase 2: Source Binding & Engine Verification Report

**Phase Goal:** As a ontology 开发者, I want to 把 object type / M:N link 绑定到 Trino 物理表，由 ontoforge 经自写 client 连接已有 Trino 并校验绑定，并在 Windows 本机用 SQLite 替身零外部依赖跑通, so that 定义能够使用真实数据源且支持本机验证.
**Mode:** mvp
**Verified:** 2026-11-08T09:52:58Z
**Source HEAD:** `54e13c8d303438514fd76c9c194fbbb07538f934`
**Status:** `passed`
**Re-verification:** Yes — after human-judgment closure round（上一轮 `human_needed` → 本轮 `passed`）

---

## User Flow Coverage

User story（ROADMAP Phase 2 goal，`mode: mvp`；`user-story.validate` 返回 `valid: true`）：
«As a ontology 开发者, I want to 把 object type / M:N link 绑定到 Trino 物理表，由 ontoforge 经自写 client 连接已有 Trino 并校验绑定，并在 Windows 本机用 SQLite 替身零外部依赖跑通, so that 定义能够使用真实数据源且支持本机验证.»

| Step | Expected（开发者视角） | Evidence（当前源码） | Status |
|---|---|---|---|
| 1. 在 object type / M:N link 上写 `backing: source`，并在 `bindings/<profile>/` 下写 `kind: SourceBinding` 文档 | 绑定被识别并载入定义层；`backing: editOnly` 的类型不允许有 source 绑定 | `crates/ontoforge-schema/src/loader/dispatch.rs:17,19`（`KINDS` 含 `SourceBinding`）；`crates/ontoforge-schema/src/ir/binding.rs:35,116,119`；`crates/ontoforge-schema/src/validate/bindings.rs:308-355`（editOnly → 报错并提示改 `backing: source`） | ✓ |
| 2. 跑 `ontoforge validate` / `ontoforge lint` | 类型契约违规（decimal 精度、无时区 timestamp 缺 `timezone`、collation、geopoint）报 E；示例 ontology lint clean | `crates/ontoforge-schema/src/lint/binding/`（`e13.rs` / `e14.rs` / `e15.rs` / `e16.rs` / `w17.rs` + snapshots）；`lint/binding/mod.rs:11-18` 注册全部规则；`crates/ontoforge-schema/src/diag.rs:219,224`（`ONT-V17` / `ONT-V19`） | ✓ |
| 3. 跑 `ontoforge validate --online`（不给 server-config） | 本机零外部依赖地用 SQLite 替身逐绑定比对物理表，无不匹配退出码 0 | `crates/ontoforge/src/online.rs`；`crates/ontoforge-engine/src/sqlite/{mod.rs,fixture.rs}`；`crates/ontoforge/tests/cli_online.rs` | ✓ |
| 4. 跑 `ontoforge validate --online --server-config <toml>` 指向真实 Trino | 自写 client 连 Trino，校验列存在与类型兼容；不匹配报 `ONT-V17` 并定位到绑定文件 | `crates/ontoforge-engine/src/trino/{client.rs,decode.rs,headers.rs}`；`crates/ontoforge/src/online.rs:568-594`；`crates/ontoforge/tests/cli_online.rs`、`crates/ontoforge/tests/cli_trino_it.rs` | ✓ |
| 5. 跑 `ontoforge validate --online --check-pk` | 报告重复主键（NULL 不算重复） | `crates/ontoforge/src/lib.rs:118-130`；`crates/ontoforge/src/online.rs:242`；`crates/ontoforge/tests/cli_check_pk.rs` | ✓ |
| 6. Trino 不可达时 | 明确 fail-closed（`ENGINE_UNAVAILABLE` / 退出码 3），不静默改用替身、不输出成功摘要 | `crates/ontoforge-engine/src/error.rs:84`（`ENGINE_UNAVAILABLE`）；`crates/ontoforge/src/online.rs:124`（ONT-V19）；`crates/ontoforge-engine/tests/trino_resilience.rs`（`timeout_sends_delete` / `dropped_future_sends_delete` / `cancellation_drain_*`） | ✓ |
| 7. 在 Windows 本机跑 `cargo test --workspace` | 零外部依赖全绿（SQLite 替身） | 本轮回归 `cargo test --workspace --locked --offline` 退出码 0；同日 `--no-fail-fast` = 1528 passed / 0 failed / 119 ignored（152 个 target） | ✓ |
| 8. 在 Linux CI / nightly 上验证真实 Trino + PG + Iceberg | PR `integration` 跑 Trino + PG 集成测试；nightly 483/484 跑 Iceberg 并自动开 issue | `.github/workflows/ci.yml:99,154,176-180,184,188`；`.github/workflows/nightly.yml:22,24,45,79-97`；`ci/trino/iceberg/iceberg.properties:3-13`；07.2 实跑 run `41833307824` | ✓ |

**[outcome] 子句判定**：「定义能够使用真实数据源（Step 4/5/8）且支持本机验证（Step 3/7）」——两项均在代码与实跑证据中成立。

---

## Goal Achievement

### Verdict Summary

上一轮（`human_needed`，107/109）的 verdict 由 **3 条待人工签字的 judgment 项**构成。本轮结论：

- **三条 judgment 项已由人签字关闭。** `02-11-HUMAN-ACCEPTANCE.md` 的三处结论位现为 `VERDICT: ACCEPTED（2026-11-08 人工签字）`（`:45` G-02-B / `:62` G-02-C / `:81` G-02-D），每条附理由行并指向 `02-UAT.md` 第 7/8/9 项的 `note:`。`02-UAT.md` 为 `status: complete`，9/9 passed、0 issues。
- **机器可判定部分本轮独立重做，未发现漂移。** 产品代码（`crates/` / `ci/` / `.github/` / `examples/` / 已跟踪 `docs/`）自上一轮 source head `87f216d8` 起**逐字未变**（`git diff --name-only 87f216d8..54e13c8` 只含 8 个 `.planning/` 文档），因此上一轮的源码级证据全部仍然成立；本轮仍逐条重跑了 A1..A5、C-1、D-1、D-2、词表双向 diff，并**新增**一次独立标识符扫描（无新增可疑物理名）。
- **两条 judgment-tier 禁止项（02-03 / 02-10）随之关闭**，从 `unverified_prohibitions` 迁到 `resolved_prohibitions`，`human_verification` 节为空。
- **覆盖集在写盘后出现纯记账性改动，verifier 已刷新。** `02-11-HUMAN-ACCEPTANCE.md` 的 frontmatter 由 `status: pending-human-review` 改为 `status: accepted` 并新增 `accepted: 2026-11-08`；`02-REVIEW-DISPOSITION.md` 的正文汇总表与 frontmatter 计数块改为与 frontmatter 一致（CR-01 / WR-01 / WR-02 = `fixed`，计数 `fixed: 3` / `open: 10` / `total: 13`）。两处**均未触及**三处 `VERDICT: ACCEPTED` 结论行、正文其余内容或任何 must-have 证据；`.planning/REQUIREMENTS.md` 的 ENGINE-05 勾选与 traceability 也已回填为 `[x]` / `Complete`（工作树改动）。本报告因此刷新覆盖指纹并同步受影响的引用行号（`02-11-HUMAN-ACCEPTANCE.md` 的结论行因 frontmatter 增行由 `:44` / `:61` / `:80` 移至 `:45` / `:62` / `:81`）。
- 无 FAILED truth、无缺件 artifact、无断链、无 blocker anti-pattern、无未决 human item → 本轮 verdict = **`passed`**（109/109）。

### Observable Truths — Roadmap Success Criteria（5 条）

| # | Truth | Status | Evidence |
|---|---|---|---|
| SC1 | 定义以 `backing: source \| editOnly` 显式声明数据来源，`SourceBinding` 绑定文件（按 profile 分目录）声明 source 绑定；decimal 精度、无时区 timestamp 的 `timezone`、collation、geopoint 不支持等契约违反时 lint 报 E，示例 ontology 仍 lint clean | ✓ VERIFIED | `crates/ontoforge-schema/src/loader/dispatch.rs:17,19`；`ir/binding.rs:35,116,119`；`validate/bindings.rs:308-355`；`lint/binding/{e13,e14,e15,e16,w17}.rs` + `lint/binding/mod.rs:11-18`；`diag.rs:219,224` |
| SC2 | `ontoforge validate --online` 对真实 Trino 校验绑定（列存在、类型兼容）并报告不匹配；`--check-pk` 报告重复主键 | ✓ VERIFIED | `crates/ontoforge/src/online.rs`（`ONT-V17` 定位到绑定文件、`:568-594` fixture-info 路径）；`crates/ontoforge/tests/{cli_online.rs,cli_online_trino.rs,cli_check_pk.rs,cli_trino_it.rs}` |
| SC3 | Trino client 能执行查询、按超时与客户端断连取消、带 trace token 与 client tags；Trino 不可达时返回 `EngineUnavailable` | ✓ VERIFIED | `crates/ontoforge-engine/src/trino/{client.rs,cancel.rs,headers.rs,classify.rs}`；`headers.rs:54-55` 注释列出 `X-Trino-User` / `X-Trino-Source` / `X-Trino-Trace-Token` / `X-Trino-Client-Tags` / `X-Trino-Time-Zone` / session；具名行为测试 `trino_resilience.rs:571,626,647,714,738,754,773`；`error.rs:84` `ENGINE_UNAVAILABLE` |
| SC4 | 自有 printer 生成的 SQL 经 proptest（任意字符串值往返不改变结构）与真实 Trino / SQLite 执行验证；SQLite 替身在 Windows `cargo test` 全绿 | ✓ VERIFIED | `crates/ontoforge-engine/tests/printer_prop.rs`（`proptest!` + 独立 AST 往返）与 `printer_prop.proptest-regressions`；`crates/ontoforge-engine/src/sqlite/`；本轮回归 `cargo test --workspace --locked --offline` 退出码 0，同日 `--no-fail-fast` = 1528 passed / 0 failed / 119 ignored |
| SC5 | Linux CI 以 Trino + PG service container 跑集成测试，nightly 加跑 Iceberg | ✓ VERIFIED | `.github/workflows/ci.yml:99,154,168,176-180,184,188`；`.github/workflows/nightly.yml:22,24,45,79-97`；`ci/trino/iceberg/iceberg.properties:3-13`；`ci/iceberg-services.sh:435-491`；实跑证据见 07.2 的原始 run `41833307824`（job `126585725657` / `126585726171` / `126592749564`） |

### Observable Truths — PLAN 细化（104 条，按 plan 统计）

上一轮已逐条核对 02-01..02-10 的 98 条细化 truth。本轮为 re-verification：对已 VERIFIED 项做存在性 + 基本健全性回归抽查，对上一轮的 2 条 UNCERTAIN 做完整复核，并新增核对 02-11 的 6 条。

| Plan | truth 数 | 本轮判定 |
|---|---|---|
| 02-01 | 10 | 全部 VERIFIED（回归抽查通过） |
| 02-02 | 11 | 全部 VERIFIED（回归抽查通过） |
| 02-03 | 13 | 12 VERIFIED + 1 VERIFIED（human-accepted）——第 13 号 truth 与「公开数据来源」禁止项共同承载 G-02-B，本轮由人工签字闭合 |
| 02-04 | 11 | 全部 VERIFIED |
| 02-05 | 13 | 全部 VERIFIED（其中 1 条与 02-03 共享 G-02-B 结论） |
| 02-06 | 7 | 全部 VERIFIED |
| 02-07 | 9 | 全部 VERIFIED（其中 2 条计入 override） |
| 02-08 | 7 | 全部 VERIFIED |
| 02-09 | 8 | 全部 VERIFIED |
| 02-10 | 9 | 8 VERIFIED + 1 VERIFIED（human-accepted）——第 8 号 truth「私有 Trino 数据源边界」由 G-02-D 签字闭合 |
| 02-11 | 6 | 全部 VERIFIED（本轮逐条独立复核，见下表） |
| **合计** | **104** | **104 VERIFIED**（其中 2 条为 human-accepted 闭合） |

**Score:** 109/109（5 条 SC + 104 条 PLAN truth）；`behavior_unverified: 0`（无「present + wired 但行为未执行」的状态转移类 truth）。

### 上一轮 2 条非 VERIFIED 项的本轮判定

| Truth | 上一轮 | 本轮 | 依据 |
|---|---|---|---|
| `02-03` / `02-05` 公开数据来源 judgment | ? UNCERTAIN (judgment) | ✓ **VERIFIED（human-accepted）** | `02-11-HUMAN-ACCEPTANCE.md:45` = `VERDICT: ACCEPTED（2026-11-08 人工签字）`；理由 `02-UAT.md:50`；机器部分本轮重做（45 标识符 ↔ 45 词双向逐行相等、`SUSPECTED NON-NEUTRAL` 为空、独立新增扫描无漂移） |
| `02-10.8` 私有 Trino 数据源边界 | ? UNCERTAIN (judgment) | ✓ **VERIFIED（human-accepted）** | `02-11-HUMAN-ACCEPTANCE.md:81` = `VERDICT: ACCEPTED（2026-11-08 人工签字）`；理由 `02-UAT.md:66`（通用表述依 ADR-0032）；机器部分本轮重做（门控 D-1 / D-2 均 PASS） |

> 措辞说明：按 `accepted` ADR-0032，`sqlserver` 只是 Trino 暴露的一类 catalog，代码里没有任何按数据库类型分叉的路径。本报告自本轮起一律使用「私有 Trino 数据源」的通用表述；上一轮报告中「SQL Server 由私有 fixture 验证」「SQL Server 只经仓库外私有 fixture 验证」两处已按 ADR-0032 改写。

### 02-11 的 6 条 truth（本轮逐条独立复核）

| # | Truth | Status | 独立复核证据 |
|---|---|---|---|
| 02-11.1 | 02-10.6 的 nightly Iceberg 合同在**当前源码**上逐条可核，每条断言带 `file:line` 记录在 `02-11-RECONCILIATION.md` | ✓ VERIFIED | 本轮重跑 5 条断言（Behavioral Spot-Checks A1..A5），与 `02-11-RECONCILIATION.md:35-39` 判定逐条一致 |
| 02-11.2 | `02-11-RECONCILIATION.md` 写明 07.2 的 `passed` 只覆盖 MinIO/S3 + PG JDBC Iceberg 子范围，**不**闭合 ENGINE-05 私有 Trino 数据源边界；并交叉引用 07.2 的具体证据文件与行号 | ✓ VERIFIED | `02-11-RECONCILIATION.md:59-89`（`## Phase 07.2 Cross-Reference` + 边界声明原文）；所引 `07.2-VERIFICATION.md:43/44/49` 逐行核对为 truth 2 / 3 / 8，内容相符 |
| 02-11.3 | 记录**所有**可观测的实现路径偏差，并注明是与 02-10 计划文本的偏差还是与 02-10.6 truth 本身的偏差 | ✓ VERIFIED | `02-11-RECONCILIATION.md:43-56` 登记 D-01（services.postgres 迁移）、D-02（`fs.native-s3.enabled` → `fs.s3.enabled`、endpoint 主机名）、D-03（`report-failure` 触发条件收窄）、D-04（编排迁入 `ci/iceberg-services.sh`），每条均标注「不影响 truth 成立 / REFINED」 |
| 02-11.4 | 机器可判定的那一半由一次只读扫描落到 `02-11-source-scan.txt`，每个观测到的物理标识符与 `02-11-RECONCILIATION.md` 声明的中性词表**逐行相等** | ✓ VERIFIED | 本轮用脚本比对两侧 `<!-- BEGIN IDENTIFIERS -->` 块：各 45 条，**序列完全相同**（`identical sequence: True`），无单侧多余；`## SUSPECTED NON-NEUTRAL` 为「空。」 |
| 02-11.5 | G-02-B / G-02-C / G-02-D 各自被转写成一个可判定的人工验收问题，每个条目有「问题原文 / 支撑证据 file:line / 机器不可判定的部分 / VERDICT 占位」四个字段 | ✓ VERIFIED | `02-11-HUMAN-ACCEPTANCE.md` 三子节字段齐备；结论位现为 3 处 `VERDICT: ACCEPTED（2026-11-08 人工签字）`（`:45` / `:62` / `:81`），每处附理由行 |
| 02-11.6 | 本 plan **不引入任何产品代码变更** | ✓ VERIFIED | `git diff --name-only 87f216d8..54e13c8` 只含 8 个 `.planning/` 文档，`crates/` `ci/` `.github/` `examples/` 已跟踪 `docs/` **零变更**；`02-VERIFICATION.md` / `02-UAT.md` / `02-CONTEXT.md` / `02-RESEARCH.md` / `02-PATTERNS.md` / `REQUIREMENTS.md` / `ROADMAP.md` 均未被该 plan 修改 |

---

## Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `.planning/phases/02-source-binding-engine/02-11-RECONCILIATION.md` | G-02-A 的可引用调和证据 | ✓ VERIFIED | 18698 bytes；A1..A5 判定表 + D-01..D-04 偏差台账 + 07.2 交叉引用 + 边界声明 + 45 词中性词表 + 本轮未执行清单 |
| `.planning/phases/02-source-binding-engine/02-11-HUMAN-ACCEPTANCE.md` | G-02-B/C/D 的人工验收清单与签字结论 | ✓ VERIFIED | 3 子节 × 4 字段；结论位 3 处 `VERDICT: ACCEPTED（2026-11-08 人工签字）`（`:45` / `:62` / `:81`）；frontmatter 已回填为 `status: accepted` + `accepted: 2026-11-08` |
| `.planning/phases/02-source-binding-engine/02-11-source-scan.txt` | 只读扫描原始输出 | ✓ VERIFIED | 18858 bytes；`==== IDENTIFIERS ====`（45）/ `==== ENDPOINTS ====`（7）/ `==== PRIVATE PATH GATE ====` / `==== PROVENANCE BOUNDARY ====` 四段齐备 |
| `.planning/phases/02-source-binding-engine/02-UAT.md` | 人工验收结果 | ✓ VERIFIED | `status: complete`，9/9 passed、0 issues；第 7/8/9 项的 `note:` 给出 G-02-B/C/D 的人工理由 |
| `.planning/phases/02-source-binding-engine/02-11-SUMMARY.md` | 产出索引与四个 gap 的状态声明 | ✓ VERIFIED | 12460 bytes；G-02-A「已调和」、B/C/D 在该 plan 执行时点为「未关闭」——属签字前的历史快照，本轮以 `02-11-HUMAN-ACCEPTANCE.md` 的签字行为准 |
| `ci/trino/iceberg/iceberg.properties` | JDBC catalog + S3/MinIO warehouse | ✓ VERIFIED | 13 行；JDBC 六键 `:3-8`、warehouse `s3://warehouse/` `:9`、`fs.s3.enabled=true` `:10`、`s3.endpoint=http://minio:9000` `:11`、path-style `:12`、region `:13`；**无** `fs.local.enabled` / `local:///warehouse`（本轮 grep 退出码 1） |
| `.github/workflows/nightly.yml` | 483/484 矩阵 + 独立 report-failure job | ✓ VERIFIED | 矩阵 `:22` / `:24`；执行入口 `:45`；`report-failure:` `:79`；`permissions: issues: write` `:83-84`（非注释行中恰好 1 处） |
| `crates/ontoforge/tests/private_fixtures.rs` | 私有 fixture 入口双门控 | ✓ VERIFIED | `FIXTURES_VAR` `:14`；缺变量即 panic `:20-21`；`#[ignore = "needs ONTOFORGE_PRIVATE_FIXTURES"]` `:69` |
| `CONTRIBUTING.md` | 私有 fixture 约定 | ✓ VERIFIED | 私有目录在仓库之外 `:89`；运行方式与退出码约定 `:91` |

## Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| `02-11-source-scan.txt` | `02-11-RECONCILIATION.md` | `OBSERVED IDENTIFIERS` 段与 `NEUTRAL VOCABULARY` 段逐行相等 | ✓ WIRED | 本轮脚本：两侧各 45 条、序列完全相同（`identical sequence: True`）；词表升序 |
| `02-11-RECONCILIATION.md` | `07.2-VERIFICATION.md` | 07.2 must-haves 表与 ENGINE-05 traceability 行被逐条引用并写明覆盖边界 | ✓ WIRED | 引用 `07.2-VERIFICATION.md:43/44/49`（truth 2 / 3 / 8）与 `:136` / `:174` 的边界原文；逐行核对内容相符 |
| `02-11-HUMAN-ACCEPTANCE.md` | `02-11-source-scan.txt` | 每个 G-02-x 条目的 `EVIDENCE:` 字段指回扫描文件的具体行 | ✓ WIRED | 9 条 `file:line` 证据引用，全部落在 scan 文件的 IDENTIFIERS / ENDPOINTS / PRIVATE PATH GATE 三段内 |
| `02-11-HUMAN-ACCEPTANCE.md` | `02-UAT.md` | 三处 `VERDICT` 的「理由」行指向 UAT 对应项 | ✓ WIRED | 人工签字理由行 → `02-UAT.md:50`（第 7 项）/ `:58`（第 8 项）/ `:66`（第 9 项），一一对应 |
| `ci/trino/iceberg/iceberg.properties` | `ci/iceberg-services.sh` | runner 复制 catalog 目录并注入临时凭据后只读挂载 | ✓ WIRED | `ci/iceberg-services.sh:435-443`（复制 catalog）、`:471`（`-v ... :/etc/trino/catalog:ro`）、`:491`（`test -r /etc/trino/catalog/iceberg.properties`） |
| `crates/ontoforge-engine/src/trino/` | `.github/workflows/ci.yml` | CI 的 Trino 在线 corpus 调用同一 engine client | ✓ WIRED | `ci.yml:184`（`trino_it --ignored`）、`:188`（`cli_trino_it --ignored`）、`:176-180`（`trino_l2_acl_it` 数量门禁） |
| `crates/ontoforge/src/online.rs` | `crates/ontoforge-engine/src/sqlite/` | 无 `--server-config` 时用 SQLite 替身执行绑定比对 | ✓ WIRED | `crates/ontoforge/tests/cli_online.rs`（无 server-config 路径）+ `crates/ontoforge-engine/src/sqlite/{mod.rs,fixture.rs}` |

## Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|---|---|---|---|---|
| `ci/trino/iceberg/iceberg.properties` | `default-warehouse-dir` | `s3://warehouse/`（MinIO 容器） | ✓ | ✓ FLOWING — 07.2 实测 `metadata_location` 以 `s3://warehouse/` 开头且桶内 metadata/data 对象非空 |
| `crates/ontoforge-engine/src/printer/` | 打印 SQL | 类型化 AST → SQL 文本 | ✓ | ✓ FLOWING — proptest 往返 + 真实 Trino / SQLite 执行 |
| `crates/ontoforge/tests/cli_check_pk.rs` | PK 查重结果 | 真实 Trino 查询 | ✓ | ✓ FLOWING（CI `--ignored` 执行；`02-UAT.md` 第 5 项记录 run 39219827829 通过） |
| `02-11-source-scan.txt` | 标识符 / 端点清单 | 一次只读源码扫描 | ✓ | ✓ FLOWING — 本轮**重验了 CR-01 修正后的 5 条 `file:line`**：`ci/trino/l2/rules.json:18` = `"table": "orders\|Orders\|shipments\|items\|ordеrs"` 规则行 ✓；`:19` = `{ "user": "l2", "catalog": "system", "schema": "runtime", "table": "queries" }` ✓；`ci/pg/seed.sql:172` = `CREATE TABLE aviation.cardinality_records (` ✓；`ci/trino/l2/seed.sql:5` = `CREATE TABLE hive.sales."ordеrs" AS SELECT 2 id;` ✓（第 4 字符为西里尔 U+0435） |

## Behavioral Spot-Checks

均为只读结构门禁，单条 < 10 秒，不启动容器、不连外部服务、不改状态。

| Behavior | Command | Result | Status |
|---|---|---|---|
| A1 483/484 矩阵 + 执行入口 | `grep -n 'trino: "48\|iceberg-services.sh' .github/workflows/nightly.yml` | `:22`=483、`:24`=484、`:45` 跑 `ci/iceberg-services.sh nightly --trino "${{ matrix.trino }}"` | ✓ PASS |
| A2 JDBC catalog 六键 | `cat -n ci/trino/iceberg/iceberg.properties` | `:3-8` catalog type / catalog-name / driver-class / connection-url / connection-user / connection-password | ✓ PASS |
| A3 MinIO warehouse 且无本地回退 | `cat -n` + `grep -n 'fs.local.enabled\|local:///'` | `:9` `s3://warehouse/`、`:10` `fs.s3.enabled=true`、`:11` `s3.endpoint=http://minio:9000`、`:12` path-style、`:13` region；`fs.local.enabled` / `local:///` **不存在**（grep 退出码 1） | ✓ PASS |
| A4 独立 report-failure job 三步行为 | `grep -n 'report-failure:\|gh issue list\|gh issue comment\|gh issue create' .github/workflows/nightly.yml` | `:79` 独立 job；`:92` list → `:94` comment → `:97` create | ✓ PASS |
| A5 权限范围唯一 | `grep -v '^\s*#' nightly.yml \| grep -c 'issues: write'` | = 1（`:84`）；workflow 级 `:10-11` 仅 `contents: read`；`iceberg` job 块内无 job 级 `permissions:` | ✓ PASS |
| C-1 公开 workflow 不引用私有入口 | `grep -rn 'private_fixtures\|ONTOFORGE_PRIVATE_FIXTURES' .github/` | 退出码 1（无匹配） | ✓ PASS |
| D-1 私有入口双门控 | `grep -n 'ONTOFORGE_PRIVATE_FIXTURES\|#\[ignore' crates/ontoforge/tests/private_fixtures.rs` | `:14` / `:20-21` / `:69` 三处命中 | ✓ PASS |
| D-2 贡献文档约定 | `grep -n 'ONTOFORGE_PRIVATE_FIXTURES' CONTRIBUTING.md` | `:89` / `:91` 两处命中 | ✓ PASS |
| 标识符词表双向相等 | 脚本比对两侧 `<!-- BEGIN IDENTIFIERS -->` 块 | 45 == 45，**序列完全相同** | ✓ PASS |
| 端点枚举无公司集群地址 | `grep -rn 'jdbc:[a-z]+://\|s3://' ci/ .github/ examples/` | 4 条业务端点全为 `localhost:5432/{demo,editstore,iceberg_catalog}` 与 `s3://warehouse/`；另有 `ci/query-services.sh:64` 的 sed 改写模板（含 `host.docker.internal`） | ✓ PASS |
| 本轮新增：独立标识符扫描无漂移 | 脚本扫 `examples/` + `ci/` + `crates/` 的 `catalog:/schema:/table:` 与 `CREATE SCHEMA/TABLE/VIEW` 位置标识符，与 45 词表求差 | 差集逐条人工复核：`slot_crew`（lint 测试夹具，航空域）、`demo__aviation`（SQLite attach 内存库别名）、`not_a_real_phase4_table`（自述假表）、`fx_table` / `fx_ref` / `object_table`（Rust 函数名，非物理标识符）、`of_edit_log` / `of_role_binding` / `information_schema` / `iceberg_tables`（系统表）——**无公司专有物理名** | ✓ PASS |
| CR-01 修正后的 `file:line` 重验 | `sed -n '14,22p' ci/trino/l2/rules.json`、`sed -n '168,175p' ci/pg/seed.sql`、`sed -n '1,8p' ci/trino/l2/seed.sql` | 4 处引用内容与 scan 文件描述逐字相符 | ✓ PASS |
| 具名行为测试存在性 | `grep -n 'cancellation_drain\|timeout_sends_delete\|dropped_future_sends_delete' crates/ontoforge-engine/tests/trino_resilience.rs` | `:571` / `:647` / `:714` / `:738` / `:754` / `:773` 六处具名测试；`drift_budget.rs:45,87` 两处 | ✓ PASS |
| Windows workspace 回归 | `cargo test --workspace --locked --offline`（orchestrator 已跑，本轮只引用） | 退出码 0，无失败 target；同日 `--no-fail-fast` = 1528 passed / 0 failed / 119 ignored（152 个 target） | ✓ PASS（`.planning/WINDOWS.md:35` 登记 `cli_check_cardinality.rs` 两个 300 ms deadline 用例为全负载下偶发 flake，隔离重跑 13 passed / 0 failed；本轮确认该登记**仍在**，未新增 flake） |
| 产品代码零漂移 | `git diff --name-only 87f216d8..54e13c8` | 只含 8 个 `.planning/` 文档；`crates/` `ci/` `.github/` `examples/` 已跟踪 `docs/` 零变更 | ✓ PASS |

## Probe Execution

本 phase 无 `scripts/*/tests/probe-*.sh` 形态的探针。02-11 采用的是**只读结构门禁**（G-A1..G-A5、C-1、D-1、D-2），已在上表逐条执行并全部通过。

| Gate | Scope | Result | Status |
|---|---|---|---|
| G-A1..G-A5 | `ci/trino/iceberg/iceberg.properties`、`.github/workflows/nightly.yml`、`ci/iceberg-services.sh` | 5/5 通过（结构判定） | PASS |
| C-1 | `.github/` 全目录 | 无私有入口引用 | PASS |
| D-1 / D-2 | `crates/ontoforge/tests/private_fixtures.rs`、`CONTRIBUTING.md` | 双门控 + 文档约定在位 | PASS |

**边界声明：** 结构门禁证明「合同在当前源码中成立」，**不**证明「nightly 在某次运行中真的跑绿」。后者由 07.2 的既有原始 run `41833307824`（483 job `126585725657`、484 job `126585726171`、report-failure job `126592749564`）承担。本报告不把结构门禁的通过表述为实跑通过。

## Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|---|---|---|---|---|
| BIND-01 | 02-01 / 02-03 / 02-11 | `backing` 必填 + 按 profile 分目录的 `SourceBinding` 绑定文件 | ✓ SATISFIED | `loader/dispatch.rs:17,19`；`ir/binding.rs`；`validate/bindings.rs:308-355`；02-03 truth 1-12 |
| BIND-02 | 02-03 / 02-07 / 02-08 | 类型契约（decimal 精度、无时区 timestamp 的 `timezone`、可选 collation、geopoint 不支持）违反时 lint 报 E | ✓ SATISFIED | `lint/binding/{e13,e14,e15,e16}.rs` + 快照 + 兼容表文档 `docs/source-binding.md` |
| BIND-03 | 02-05 / 02-08 / 02-09 | `validate --online` 校验绑定、`--check-pk` 查重、drift 状态模型 | ✓ SATISFIED（Phase 02 范围） | CLI 在线校验与 PK 查重；`crates/ontoforge-engine/src/bind_check.rs:604`（`DegradeReason::SourceSchemaDrift`）与 `tests/drift.rs:147`；**启动与定时 drift 执行属 Phase 7**（`REQUIREMENTS.md:195` 已如此登记） |
| ENGINE-01 | 02-04 / 02-06 / 02-09 / 02-10 | 自写 Trino client、超时、断连取消、trace token、两个服务身份 | ✓ SATISFIED | `crates/ontoforge-engine/src/trino/`；`trino_resilience.rs` 具名取消测试；`headers.rs:54-55` |
| ENGINE-02 | 02-04 / 02-06 / 02-09 | Trino 不可用时 fail-closed（`EngineUnavailable`），`/health` 与 describe 暴露引擎状态 | ✓ SATISFIED（Phase 02 范围） | `error.rs:84`（`ENGINE_UNAVAILABLE`）、`trino/classify.rs`、`config.rs:193-197`（`AuthKind::{None,Basic,Bearer}`）；**`/health` 与 describe 暴露属 Phase 7**（`REQUIREMENTS.md:197` 已如此登记） |
| ENGINE-03 | 02-02 / 02-10 | 自有 printer 生成 SQL，值只经转义函数输出，注入防护由 proptest 与真实执行验证 | ✓ SATISFIED | `tests/printer_prop.rs` + `printer_prop.proptest-regressions`；`printer/escape.rs`；真实 Trino corpus（`02-UAT.md` 第 3 项 golden 首跑原样通过） |
| ENGINE-04 | 02-02 / 02-05 | SQLite 替身支撑 Windows 本机 `cargo test` | ✓ SATISFIED（Phase 02 范围） | 本机 1528 passed / 0 failed；`src/sqlite/`；**`serve --dev` 属 Phase 7**（`REQUIREMENTS.md:198` 已如此登记） |
| ENGINE-05 | 02-10 / 02-11 | Linux CI 跑 Trino + PG 集成测试，nightly 加跑 Iceberg；**私有环境下的 Trino 数据源由仓库外私有 fixture 验证** | ✓ SATISFIED | **已闭合部分**：CI Trino + PG 集成测试（`ci.yml:99,154,184,188`）、nightly Iceberg + MinIO/S3 合同（当前源码 + 07.2 实跑证据）；**私有 Trino 数据源子句**：仓库外私有 fixture 入口（`crates/ontoforge/tests/private_fixtures.rs:14/21/69` 环境变量 + `#[ignore]` 双门控）与约定文档（`CONTRIBUTING.md:89/91`）在位，公开 CI 不运行该 target（门控 C-1），**并由人工于 2026-11-08 签字接受**（G-02-D，`02-11-HUMAN-ACCEPTANCE.md:81`）。按 ADR-0032，私有源表述为「私有环境下的 Trino 数据源」，不点名具体 catalog 类型 |

8 个 requirement ID 全部在 02-01..02-11 的 PLAN frontmatter 中被声明（BIND-01 ×3、BIND-02 ×3、BIND-03 ×3、ENGINE-01 ×4、ENGINE-02 ×3、ENGINE-03 ×2、ENGINE-04 ×2、ENGINE-05 ×2，共 22 处声明），且与 `REQUIREMENTS.md:193-200` 的 traceability 表逐行对应，**无 orphan**。

> 计数更正：上一轮报告把 BIND-02 与 ENGINE-02 记为「×4」，实际各为 ×3（BIND-02 出现在 02-03 / 02-07 / 02-08；ENGINE-02 出现在 02-04 / 02-06 / 02-09）。ID 集合与覆盖判定不受影响。

> 记账已对齐：本报告写盘后，`.planning/REQUIREMENTS.md:36` 的 ENGINE-05 勾选框已由 `[ ]` 改为 `[x]`，`:200` 的 traceability 行已由 `Partial: ... 私有 Trino 数据源保留02-10.8历史judgment` 改为 `Complete`，与本报告的 ENGINE-05 判定一致（工作树改动，未提交）。

## Decision Coverage

`02-CONTEXT.md` 的可追踪决策由 GSD owner query 判定 15/15 honored。ADR-0022（`accepted`）确实把被忽略的 fixture 诊断从 Warning 降为 Info（`Severity::Info`、码仍为 `ONT-V17`），因此 2 条 override 继续成立并被携带。

ADR-0017 要求的「JDBC catalog + MinIO」与当前源码一致，上一轮标记的合同偏差（`fs.local.enabled` / `local:///warehouse`）已不存在。

**ADR-0032（`accepted`，2026-11-08）** 决定 `sqlserver` 只是 Trino 暴露的一类 catalog，与 `mysql` / `hive` / `iceberg` 并列，代码里没有任何按数据库类型分叉的路径。本轮逐条复核该决定与代码一致：`crates/ontoforge/tests/private_fixtures.rs` 的门控条件是「是否私有环境」（`ONTOFORGE_PRIVATE_FIXTURES` + `#[ignore]`），不是「是否 SQL Server」。ADR-0032 要求的通用化已落到 `REQUIREMENTS.md:36`（该项现已勾选并记为 `Complete`）与 `02-11-HUMAN-ACCEPTANCE.md`；本报告的 ENGINE-05 行与私有源子句本轮起一律采用「私有 Trino 数据源」表述。产品层残留的 SQL Server 字样登记为 Advisory（不在本轮写入范围，且不构成功能分叉）。

## Prohibitions

| Plans / tier | Enforcement and evidence | Verdict |
|---|---|---|
| 02-01 / test：禁止默认 backing | required 字段 + `ONT-V05`；负向测试 | ✓ VERIFIED |
| 02-04 / test：禁止凭据异源发送 | 构造期 HTTPS、`checked_next_uri`、禁 redirect | ✓ VERIFIED |
| 02-04 / test：禁止泄密 | `SecretString` + 错误 / CLI 消毒 | ✓ VERIFIED |
| 02-04 / test：禁止 TLS 绕过 | 无配置字段 + Clippy 方法禁令 | ✓ VERIFIED |
| 02-06 / test：禁止隐藏 Trino 故障 | typed `Unavailable`、无 fallback | ✓ VERIFIED |
| 02-09 / test：禁止在线假成功 | engine 分派与 exit 3 | ✓ VERIFIED |
| 02-09 / test：禁止隐式全表 PK 扫描 | flag guard；drift 只有 metadata | ✓ VERIFIED |
| 02-03 / judgment：公开数据来源 | 45 标识符全部中性、`SUSPECTED NON-NEUTRAL` 为空、本轮独立新增扫描无漂移 | ✓ **ACCEPTED（2026-11-08 人工签字，G-02-B）** — 机器部分本轮重做通过，结论由人工给出；不再标记 `unverified-prohibition` |
| 02-10 / judgment：公开 CI 不接公司源 | 端点枚举 + 门控 C-1（`.github/` 无私有入口引用） | ✓ **ACCEPTED（2026-11-08 人工签字，G-02-C）** — 机器部分本轮重做通过，结论由人工给出；不再标记 `unverified-prohibition` |

两条 judgment-tier 禁止项已由人工签字关闭，登记迁至 frontmatter 的 `resolved_prohibitions`。本轮未出现新的禁止项缺口。

## Test Quality / Anti-Patterns

对 phase 产出做了 debt-marker 扫描：`TBD` / `FIXME` / `XXX` 在本轮涉及的产出中**零真实命中**（`02-REVIEW.md:40` 与本报告自身文本命中的是「扫描 `TODO` / `TBD`」这类描述性字面量，非债务标记；`02-REVIEW.md` 不在本轮修改集内）。

行为依赖型 truth（状态转移 / 取消 / 清理 / 顺序不变式）均由具名行为测试覆盖，本轮确认测试存在性：`trino_resilience.rs` 的 `timeout_sends_delete` / `dropped_future_sends_delete` / `cancellation_drain_*`（4 条）、`drift_budget.rs` 的 2 条、nightly 侧的具名 `iceberg_table_is_readable`（`ci/iceberg-services.sh:203-205` 以 `--list` 精确断言该测试名存在）。故 `behavior_unverified: 0`——没有「present + wired 但行为未执行」的 truth。

## Human Judgment Closure

上一轮的 3 条 human verification item 全部已由人签字关闭，本轮无待办 human 项：

| Item | 结论 | 签字位置 | 理由 | 机器可判定部分本轮复核 |
|---|---|---|---|---|
| G-02-B 公开仓库数据来源 | ✓ ACCEPTED | `02-11-HUMAN-ACCEPTANCE.md:45` | `02-UAT.md:50`：示例 ontology、绑定文件、CI 种子与测试夹具数据均为可公开虚构数据，不含公司业务数据 | 45 ↔ 45 逐行相等、`SUSPECTED NON-NEUTRAL` 为空、新增独立扫描无漂移 |
| G-02-C 公开 CI 与公司源的隔离 | ✓ ACCEPTED | `02-11-HUMAN-ACCEPTANCE.md:62` | `02-UAT.md:58`：公开 workflow 不连接公司 Trino 集群、不读取公司 catalog / 表名 / 数据 | 门控 C-1 退出码 1；端点串全为 loopback / 容器内 `minio` / 仓库内桶前缀 |
| G-02-D 私有 Trino 数据源边界与 `ONTOFORGE_PRIVATE_FIXTURES` | ✓ ACCEPTED | `02-11-HUMAN-ACCEPTANCE.md:81` | `02-UAT.md:66`（通用表述见 ADR-0032）：私有环境下的 Trino 数据源只经仓库外私有 fixture 验证，公开 CI 不运行该 target，仓库内无任何私有绑定或数据 | 门控 D-1（`private_fixtures.rs:14/21/69`）与 D-2（`CONTRIBUTING.md:89/91`）均 PASS |

签字记录的方式与边界仍以 `02-11-HUMAN-ACCEPTANCE.md` 的 `## Recording` 节为准：结论位由人工替换为同格式的接受结论并附理由与日期，**不由本 plan 或执行者代签**；本轮 verifier 只做登记与状态同步，未改写任何结论位。该文件的 frontmatter 已相应回填为 `status: accepted` + `accepted: 2026-11-08`（本报告写盘后的记账性改动，不触及三处结论行）。

## Advisory

以下为不影响 phase goal 的准确性/记账瑕疵，如实登记但不阻断。本轮为 re-verification，全部为**已核实的确定性证据**（非无证据的新增范围问题）。完整清单见 frontmatter `advisory:`。

| # | Finding | Category | Why Advisory | 证据状态 |
|---|---|---|---|---|
| A-1 | `ROADMAP.md` 的 Phase 2 计划计数已更新为 `11/11 plans complete`，但逐条清单仍只列 02-01..02-10（未单列 02-11 gap-closure plan） | other | 计数已与 VERIFICATION `passed` 对齐；逐条清单缺项不影响 goal 判定 | 已核对 |
| A-2 | ADR-0032 的通用化只落到 planning 层，产品层 `private_fixtures.rs:1`、`CONTRIBUTING.md:85`、`docs/designs/trino-engine-sql-tiers.md:339,374` 仍点名 SQL Server | other | ADR-0032 只要求三处 planning 文档改写；代码无按类型分叉 | 已核对 |
| A-3 | `02-10-SUMMARY.md` 的陈旧断言（本地 warehouse 描述、测试计数、PR integration 编排漂移）仍在 | other | 历史 SUMMARY 不在本轮写入范围；本报告以当前源码事实取代 | 已核对 |
| A-4 | `02-11-SUMMARY.md` 的端点概括未提 `host.docker.internal`；门禁计数表述不自洽 | other | 底层 scan 文件已完整披露 | 已核对 |
| A-5 | `02-REVIEW.md:40` 的审查约束「三处 `VERDICT: PENDING` 必须仍为 `PENDING`」已被签字取代 | other | review 记录反映签字前状态 | 已核对 |
| A-6 | 工作树中未跟踪的 `docs/README.md`、`docs/foundry/`、`docs/images/`（Palantir 公开文档镜像，199 页 + 357 图） | other | 未跟踪、不在已提交面、不属于 Phase 02 交付物、不在指纹覆盖集内 | 已核对 |

## Gaps Summary

**无 BLOCKER、无 gaps、无待办 human verification item。** 本轮为 re-verification：上一轮唯一的非通过原因是 3 条 judgment 项结论位为 `VERDICT: PENDING`；这三条已于 2026-11-08 由人工签字为 `VERDICT: ACCEPTED`，机器可判定部分本轮独立重做仍全部通过。104 条 plan truth + 5 条 roadmap SC 全部 VERIFIED，8 个 requirement ID 全部 SATISFIED 且无 orphan，无缺件 artifact、无断链、无 stub、无未跟踪债务标记。

**覆盖集刷新：** 本报告写盘后，`.planning/REQUIREMENTS.md`（ENGINE-05 勾选与 traceability 回填）、`02-11-HUMAN-ACCEPTANCE.md`（frontmatter `status: accepted`）与 `02-REVIEW-DISPOSITION.md`（正文汇总表与计数块对齐）三份文件发生了记账性改动。三处改动**不触及**任何 must-have 证据、三处 `VERDICT: ACCEPTED` 结论行或任何产品代码，因此本轮 verdict 不变；verifier 已重算覆盖指纹（`covered_digest` 更新为 `v4:sha256:c06dfae7a71bdcfaa43fd42f25c80e61e42d13542d53137a0cd88a8a67c3bcfa`）并同步受影响的行号引用。三项对应的历史 advisory（A-1 REQUIREMENTS.md 记账、A-4 验收清单 frontmatter、A-5 处置台账不一致）已随之消解，本轮不再登记。

**Deferred：** 无。后续 milestone 阶段（07 / 07.1 / 07.2）均未在 ROADMAP 中承诺闭合本 phase 的 judgment 结论；`07.2-VERIFICATION.md` 的 `### Deferred Items` 节明确写明 Phase 01/02 的历史 judgment 不能用该阶段的 `passed` 清除。该边界已由本轮人工签字正面闭合，不构成可 defer 的匹配项。

---

_Verified: 2026-11-08T09:52:58Z_
_Verifier: Claude (gsd-verifier)_
_覆盖指纹由 `gsd-tools query verification.fingerprint` 计算并逐字复制；本轮未修改任何产品代码，未启动容器、未连接任何外部服务。_
