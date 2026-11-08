---
phase: 02-source-binding-engine
verified: 2026-11-08T09:17:46Z
source_head: 87f216d8882efad8db3d0ba43514538236bf60e2
status: human_needed
score: 107/109 must-haves verified
covered_files: [".github/workflows/ci.yml",".github/workflows/nightly.yml",".planning/phases/02-source-binding-engine/02-01-PLAN.md",".planning/phases/02-source-binding-engine/02-01-SUMMARY.md",".planning/phases/02-source-binding-engine/02-02-PLAN.md",".planning/phases/02-source-binding-engine/02-02-SUMMARY.md",".planning/phases/02-source-binding-engine/02-03-PLAN.md",".planning/phases/02-source-binding-engine/02-03-SUMMARY.md",".planning/phases/02-source-binding-engine/02-04-PLAN.md",".planning/phases/02-source-binding-engine/02-04-SUMMARY.md",".planning/phases/02-source-binding-engine/02-05-PLAN.md",".planning/phases/02-source-binding-engine/02-05-SUMMARY.md",".planning/phases/02-source-binding-engine/02-06-PLAN.md",".planning/phases/02-source-binding-engine/02-06-SUMMARY.md",".planning/phases/02-source-binding-engine/02-07-PLAN.md",".planning/phases/02-source-binding-engine/02-07-SUMMARY.md",".planning/phases/02-source-binding-engine/02-08-PLAN.md",".planning/phases/02-source-binding-engine/02-08-SUMMARY.md",".planning/phases/02-source-binding-engine/02-09-PLAN.md",".planning/phases/02-source-binding-engine/02-09-SUMMARY.md",".planning/phases/02-source-binding-engine/02-10-PLAN.md",".planning/phases/02-source-binding-engine/02-10-SUMMARY.md",".planning/phases/02-source-binding-engine/02-11-HUMAN-ACCEPTANCE.md",".planning/phases/02-source-binding-engine/02-11-PLAN.md",".planning/phases/02-source-binding-engine/02-11-RECONCILIATION.md",".planning/phases/02-source-binding-engine/02-11-SUMMARY.md",".planning/phases/02-source-binding-engine/02-11-source-scan.txt","CONTRIBUTING.md","Cargo.lock","Cargo.toml","ci/check-tls-deps.sh","ci/ontoforge-server.ci.toml","ci/pg/iceberg_catalog.sql","ci/pg/query_corpus.sql","ci/pg/rowpred_corpus.sql","ci/pg/seed.sql","ci/query-services.sh","ci/release-package.ps1","ci/release-smoke.ps1","ci/server-services.sh","ci/test_tls_deps.py","ci/test_windows_server.py","ci/trino/catalog/demo.properties","ci/trino/catalog/editstore.properties","ci/trino/catalog/tpch.properties","ci/trino/iceberg/iceberg.properties","ci/trino/l2/access-control.properties","ci/trino/l2/config.properties","ci/trino/l2/rules.json","ci/trino/l2/seed.sql","ci/windows-server.py","clippy.toml","crates/ontoforge-engine/Cargo.toml","crates/ontoforge-engine/src/bind_check.rs","crates/ontoforge-engine/src/bind_check/compat_tests.rs","crates/ontoforge-engine/src/bind_check/tests.rs","crates/ontoforge-engine/src/config.rs","crates/ontoforge-engine/src/error.rs","crates/ontoforge-engine/src/lib.rs","crates/ontoforge-engine/src/pk_check.rs","crates/ontoforge-engine/src/printer/ast.rs","crates/ontoforge-engine/src/printer/call.rs","crates/ontoforge-engine/src/printer/dialect.rs","crates/ontoforge-engine/src/printer/escape.rs","crates/ontoforge-engine/src/printer/l2.rs","crates/ontoforge-engine/src/printer/mod.rs","crates/ontoforge-engine/src/printer/snapshots/ontoforge_engine__printer__tests__print_sqlite.snap","crates/ontoforge-engine/src/printer/snapshots/ontoforge_engine__printer__tests__print_trino.snap","crates/ontoforge-engine/src/printer/sqlite_ddl.rs","crates/ontoforge-engine/src/printer/tests.rs","crates/ontoforge-engine/src/sqlite/fixture.rs","crates/ontoforge-engine/src/sqlite/mod.rs","crates/ontoforge-engine/src/sqlite/tests.rs","crates/ontoforge-engine/src/trino/cancel.rs","crates/ontoforge-engine/src/trino/classify.rs","crates/ontoforge-engine/src/trino/client.rs","crates/ontoforge-engine/src/trino/decode.rs","crates/ontoforge-engine/src/trino/headers.rs","crates/ontoforge-engine/src/trino/l2_health.rs","crates/ontoforge-engine/src/trino/l2_monitor.rs","crates/ontoforge-engine/src/trino/mod.rs","crates/ontoforge-engine/src/value.rs","crates/ontoforge-engine/tests/bind_check_isolation.rs","crates/ontoforge-engine/tests/data/l2_corpus.json","crates/ontoforge-engine/tests/data/test-ca.pem","crates/ontoforge-engine/tests/doc_type_table.rs","crates/ontoforge-engine/tests/drift.rs","crates/ontoforge-engine/tests/drift_budget.rs","crates/ontoforge-engine/tests/l2_explain.rs","crates/ontoforge-engine/tests/l2_gate.rs","crates/ontoforge-engine/tests/l2_health.rs","crates/ontoforge-engine/tests/l2_monitor.rs","crates/ontoforge-engine/tests/pk_check.rs","crates/ontoforge-engine/tests/printer_literals.rs","crates/ontoforge-engine/tests/printer_prop.proptest-regressions","crates/ontoforge-engine/tests/printer_prop.rs","crates/ontoforge-engine/tests/server_config.rs","crates/ontoforge-engine/tests/snapshots/pk_check__pk_duplicates_sqlite.snap","crates/ontoforge-engine/tests/snapshots/pk_check__pk_duplicates_trino.snap","crates/ontoforge-engine/tests/snapshots/pk_check__pk_nulls_sqlite.snap","crates/ontoforge-engine/tests/snapshots/pk_check__pk_nulls_trino.snap","crates/ontoforge-engine/tests/sql_text_only_from_printer.rs","crates/ontoforge-engine/tests/sqlite_engine.rs","crates/ontoforge-engine/tests/trino_client.rs","crates/ontoforge-engine/tests/trino_it.rs","crates/ontoforge-engine/tests/trino_l2_acl_it.rs","crates/ontoforge-engine/tests/trino_resilience.rs","crates/ontoforge-schema/Cargo.toml","crates/ontoforge-schema/src/diag.rs","crates/ontoforge-schema/src/ir/action.rs","crates/ontoforge-schema/src/ir/attrs.rs","crates/ontoforge-schema/src/ir/basetype.rs","crates/ontoforge-schema/src/ir/binding.rs","crates/ontoforge-schema/src/ir/link.rs","crates/ontoforge-schema/src/ir/manifest.rs","crates/ontoforge-schema/src/ir/mod.rs","crates/ontoforge-schema/src/ir/names.rs","crates/ontoforge-schema/src/ir/object.rs","crates/ontoforge-schema/src/ir/scope.rs","crates/ontoforge-schema/src/lib.rs","crates/ontoforge-schema/src/lint/binding/e13.rs","crates/ontoforge-schema/src/lint/binding/e14.rs","crates/ontoforge-schema/src/lint/binding/e15.rs","crates/ontoforge-schema/src/lint/binding/e16.rs","crates/ontoforge-schema/src/lint/binding/mod.rs","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e13__tests__clean.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e13__tests__triggers.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e14__tests__clean.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e14__tests__triggers.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e15__tests__clean.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e15__tests__triggers.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e16__tests__clean.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e16__tests__triggers.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__w17__tests__clean.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__w17__tests__triggers.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__w17__tests__unreadable.snap","crates/ontoforge-schema/src/lint/binding/w17.rs","crates/ontoforge-schema/src/lint/config.rs","crates/ontoforge-schema/src/lint/e08.rs","crates/ontoforge-schema/src/lint/e09.rs","crates/ontoforge-schema/src/lint/mod.rs","crates/ontoforge-schema/src/lint/naming/mod.rs","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w01__tests__clean.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w01__tests__triggers.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w02__tests__clean.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w02__tests__triggers.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w03__tests__clean.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w03__tests__triggers.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w04__tests__clean.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w04__tests__triggers.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w05__tests__clean.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w05__tests__triggers.snap","crates/ontoforge-schema/src/lint/naming/w01.rs","crates/ontoforge-schema/src/lint/naming/w02.rs","crates/ontoforge-schema/src/lint/naming/w03.rs","crates/ontoforge-schema/src/lint/naming/w04.rs","crates/ontoforge-schema/src/lint/naming/w05.rs","crates/ontoforge-schema/src/lint/snapshots/ontoforge_schema__lint__e08__tests__clean.snap","crates/ontoforge-schema/src/lint/snapshots/ontoforge_schema__lint__e08__tests__triggers.snap","crates/ontoforge-schema/src/lint/snapshots/ontoforge_schema__lint__e09__tests__clean.snap","crates/ontoforge-schema/src/lint/snapshots/ontoforge_schema__lint__e09__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/mod.rs","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w06__tests__clean.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w06__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w07__tests__clean.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w07__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w08__tests__clean.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w08__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w09__tests__clean.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w09__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w10__tests__clean.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w10__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w11__tests__clean.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w11__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/w06.rs","crates/ontoforge-schema/src/lint/structure/w07.rs","crates/ontoforge-schema/src/lint/structure/w08.rs","crates/ontoforge-schema/src/lint/structure/w09.rs","crates/ontoforge-schema/src/lint/structure/w10.rs","crates/ontoforge-schema/src/lint/structure/w11.rs","crates/ontoforge-schema/src/lint/words.rs","crates/ontoforge-schema/src/loader/discover.rs","crates/ontoforge-schema/src/loader/dispatch.rs","crates/ontoforge-schema/src/loader/fixture.rs","crates/ontoforge-schema/src/loader/mod.rs","crates/ontoforge-schema/src/loader/parse.rs","crates/ontoforge-schema/src/loader/scope.rs","crates/ontoforge-schema/src/loader/sourcemap.rs","crates/ontoforge-schema/src/schema_export.rs","crates/ontoforge-schema/src/validate/actions.rs","crates/ontoforge-schema/src/validate/bindings.rs","crates/ontoforge-schema/src/validate/links.rs","crates/ontoforge-schema/src/validate/mod.rs","crates/ontoforge-schema/src/validate/objects.rs","crates/ontoforge-schema/tests/action_types.rs","crates/ontoforge-schema/tests/bindings.rs","crates/ontoforge-schema/tests/diagnostics.rs","crates/ontoforge-schema/tests/link_types.rs","crates/ontoforge-schema/tests/lint_examples.rs","crates/ontoforge-schema/tests/load_examples.rs","crates/ontoforge-schema/tests/loader.rs","crates/ontoforge-schema/tests/object_types.rs","crates/ontoforge-schema/tests/schema_export.rs","crates/ontoforge-schema/tests/scope_documents.rs","crates/ontoforge-schema/tests/snapshots/scope_documents__scope_diagnostics.snap","crates/ontoforge/src/cli.rs","crates/ontoforge/src/dev.rs","crates/ontoforge/src/dev_assets.rs","crates/ontoforge/src/dev_token.rs","crates/ontoforge/src/import.rs","crates/ontoforge/src/lib.rs","crates/ontoforge/src/lint_cmd.rs","crates/ontoforge/src/main.rs","crates/ontoforge/src/mcp.rs","crates/ontoforge/src/migrate.rs","crates/ontoforge/src/online.rs","crates/ontoforge/src/pipeline.rs","crates/ontoforge/src/reload.rs","crates/ontoforge/src/render.rs","crates/ontoforge/src/scope.rs","crates/ontoforge/src/serve.rs","crates/ontoforge/src/webhook_worker.rs","crates/ontoforge/tests/cli.rs","crates/ontoforge/tests/cli_check_pk.rs","crates/ontoforge/tests/cli_color.rs","crates/ontoforge/tests/cli_dev_token.rs","crates/ontoforge/tests/cli_import.rs","crates/ontoforge/tests/cli_l2_identity.rs","crates/ontoforge/tests/cli_lint.rs","crates/ontoforge/tests/cli_migrate.rs","crates/ontoforge/tests/cli_online.rs","crates/ontoforge/tests/cli_online_trino.rs","crates/ontoforge/tests/cli_reload.rs","crates/ontoforge/tests/cli_schema.rs","crates/ontoforge/tests/cli_scope.rs","crates/ontoforge/tests/cli_shutdown.rs","crates/ontoforge/tests/cli_stdio.rs","crates/ontoforge/tests/cli_trino_it.rs","crates/ontoforge/tests/cli_webhook_worker.rs","crates/ontoforge/tests/dev_lifecycle.rs","crates/ontoforge/tests/invalid_examples.rs","crates/ontoforge/tests/private_fixtures.rs","crates/ontoforge/tests/snapshots/cli__validate_unknown_field_human.snap","crates/ontoforge/tests/snapshots/cli_lint__e08_human.snap","crates/ontoforge/tests/snapshots/cli_lint__e14_human.snap","crates/ontoforge/tests/snapshots/cli_online__online_missing_column_human.snap","crates/ontoforge/tests/snapshots/cli_online_trino__unavailable_human.snap","crates/ontoforge/tests/snapshots/invalid_examples__v01-json-trailing-comma.snap","crates/ontoforge/tests/snapshots/invalid_examples__v01-yaml-duplicate-key.snap","crates/ontoforge/tests/snapshots/invalid_examples__v02-missing-kind.snap","crates/ontoforge/tests/snapshots/invalid_examples__v03-function-rule.snap","crates/ontoforge/tests/snapshots/invalid_examples__v03-reserved-kind.snap","crates/ontoforge/tests/snapshots/invalid_examples__v04-unknown-field.snap","crates/ontoforge/tests/snapshots/invalid_examples__v05-missing-backing.snap","crates/ontoforge/tests/snapshots/invalid_examples__v05-missing-primary-key.snap","crates/ontoforge/tests/snapshots/invalid_examples__v06-precision-on-string.snap","crates/ontoforge/tests/snapshots/invalid_examples__v06-unknown-base-type.snap","crates/ontoforge/tests/snapshots/invalid_examples__v07-bad-object-type-name.snap","crates/ontoforge/tests/snapshots/invalid_examples__v08-duplicate-binding.snap","crates/ontoforge/tests/snapshots/invalid_examples__v08-duplicate-object-type.snap","crates/ontoforge/tests/snapshots/invalid_examples__v08-link-end-clashes-property.snap","crates/ontoforge/tests/snapshots/invalid_examples__v09-binding-unknown-target.snap","crates/ontoforge/tests/snapshots/invalid_examples__v09-link-unknown-object-type.snap","crates/ontoforge/tests/snapshots/invalid_examples__v09-rule-unknown-property.snap","crates/ontoforge/tests/snapshots/invalid_examples__v10-float-primary-key.snap","crates/ontoforge/tests/snapshots/invalid_examples__v10-primary-key-not-required.snap","crates/ontoforge/tests/snapshots/invalid_examples__v11-backing-on-one-to-many.snap","crates/ontoforge/tests/snapshots/invalid_examples__v11-foreign-key-type-mismatch.snap","crates/ontoforge/tests/snapshots/invalid_examples__v11-many-to-many-with-foreign-key.snap","crates/ontoforge/tests/snapshots/invalid_examples__v12-create-missing-primary-key.snap","crates/ontoforge/tests/snapshots/invalid_examples__v12-current-time-on-string.snap","crates/ontoforge/tests/snapshots/invalid_examples__v12-modify-assigns-primary-key.snap","crates/ontoforge/tests/snapshots/invalid_examples__v12-value-source-type-mismatch.snap","crates/ontoforge/tests/snapshots/invalid_examples__v13-comparison-type-mismatch.snap","crates/ontoforge/tests/snapshots/invalid_examples__v13-non-boolean-condition.snap","crates/ontoforge/tests/snapshots/invalid_examples__v13-objects-not-object-reference.snap","crates/ontoforge/tests/snapshots/invalid_examples__v13-syntax-error.snap","crates/ontoforge/tests/snapshots/invalid_examples__v13-unknown-parameter.snap","crates/ontoforge/tests/snapshots/invalid_examples__v14-include-escapes-root.snap","crates/ontoforge/tests/snapshots/invalid_examples__v16-binding-outside-bindings-dir.snap","crates/ontoforge/tests/snapshots/invalid_examples__v16-fixture-escapes-root.snap","crates/ontoforge/tests/snapshots/invalid_examples__v16-source-without-binding.snap","crates/ontoforge/tests/stdio_security_support/mod.rs","crates/ontoforge/tests/unit/dev.test.rs","crates/ontoforge/tests/unit/serve.test.rs","docs/source-binding.md","examples/ontologies/aviation-json/actions.json","examples/ontologies/aviation-json/bindings/dev/bindings.json","examples/ontologies/aviation-json/bindings/dev/fixtures/aircraft.csv","examples/ontologies/aviation-json/bindings/dev/fixtures/flight_crew.csv","examples/ontologies/aviation-json/bindings/dev/fixtures/flights.csv","examples/ontologies/aviation-json/bindings/dev/fixtures/maintenance_records.csv","examples/ontologies/aviation-json/bindings/dev/fixtures/pilots.csv","examples/ontologies/aviation-json/links.json","examples/ontologies/aviation-json/objects.json","examples/ontologies/aviation-json/ontology.yaml","examples/ontologies/aviation/actions.yaml","examples/ontologies/aviation/bindings/dev/aircraft.yaml","examples/ontologies/aviation/bindings/dev/fixtures/aircraft.csv","examples/ontologies/aviation/bindings/dev/fixtures/flight_crew.csv","examples/ontologies/aviation/bindings/dev/fixtures/flights.csv","examples/ontologies/aviation/bindings/dev/fixtures/maintenance_records.csv","examples/ontologies/aviation/bindings/dev/fixtures/pilots.csv","examples/ontologies/aviation/bindings/dev/flight-crew.yaml","examples/ontologies/aviation/bindings/dev/flight.yaml","examples/ontologies/aviation/bindings/dev/maintenance-record.yaml","examples/ontologies/aviation/bindings/dev/pilot.yaml","examples/ontologies/aviation/links.yaml","examples/ontologies/aviation/objects.yaml","examples/ontologies/aviation/ontology.yaml","schemas/ontoforge-definition.schema.json","schemas/ontoforge-manifest.schema.json","schemas/ontoforge-scope.schema.json"]
covered_digest: "v4:sha256:a08125dba3c3ace953e0dc3e0da2b5c3b9cf3f3fcb719235288eef306b897afd"
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
  previous_status: gaps_found
  previous_score: 101/103 must-haves verified
  gaps_closed:
    - "02-10.6（nightly Iceberg 合同：JDBC catalog + MinIO/S3 + 483/484 矩阵 + 独立 report-failure job + 权限范围）——上一轮的 FAILED (BLOCKER) 是基于 `fs.local.enabled` / `local:///warehouse` 的旧源码状态；当前源码 `ci/trino/iceberg/iceberg.properties:9-13` 已是 `s3://warehouse/` + `fs.s3.enabled=true` + `s3.endpoint=http://minio:9000`，无本地回退键，本轮逐条 file:line 重核为 VERIFIED"
    - "上一轮 VERIFICATION.md 的正文与分数自相矛盾（WR-01 / WR-02）——本次重写整份报告，以当前源码事实取代"
  gaps_remaining:
    - "G-02-B 公开数据来源（02-03 judgment 禁止项）：机器可判定部分已完成（45 个物理标识符全为中性词，SUSPECTED NON-NEUTRAL 为空），结论位显式保持 VERDICT: PENDING，待人工签字"
    - "G-02-C 公开 CI 不接公司源（02-10 judgment 禁止项）：机器可判定部分已完成（端点枚举 + 门控 C-1 PASS），结论位 VERDICT: PENDING"
    - "G-02-D 私有 Trino 数据源边界 / ONTOFORGE_PRIVATE_FIXTURES（02-10.8 / ENGINE-05 judgment 子句）：机器可判定部分已完成（门控 D-1 双门控 + D-2 文档），结论位 VERDICT: PENDING"
  regressions: []
unverified_prohibitions:
  - plan: "02-03"
    statement: "公开仓库中的示例、绑定、fixture、测试与CI种子不得出现公司物理名或真实业务数据"
    verification: judgment
    verdict: "NON-AUTHORITATIVE: 45 个物理标识符经只读扫描全部落在中性词表且 `## SUSPECTED NON-NEUTRAL` 为空；标识符中性不等于数据原创，机器无法区分「虚构但看起来真实」与「真实但已脱敏」"
    flag: "unverified-prohibition — human review recommended"
  - plan: "02-10"
    statement: "公开CI不得连接公司Trino集群或出现公司catalog/表名/数据"
    verification: judgment
    verdict: "NON-AUTHORITATIVE: 公开 workflow 不引用 private_fixtures / ONTOFORGE_PRIVATE_FIXTURES（门控 C-1 退出码 1），公开面端点串全部为回环地址、容器网络服务名或仓库内桶前缀；runner 实际注入的环境变量与网络出口不可由源码证明"
    flag: "unverified-prohibition — human review recommended"
human_verification:
  - test: "G-02-B：核对公开仓库的示例 ontology、绑定文件、CI 种子与测试夹具，确认其数据（不只是标识符）确为可公开的虚构数据，不含任何公司业务数据"
    expected: "对三个问题之一作答「是 / 否 / 不适用 + 理由」：示例、绑定、fixture、测试与CI种子全部为可公开虚构数据"
    why_human: "标识符中性是必要条件而非充分条件；机器无法判定数据行的来源与授权"
  - test: "G-02-C：核对公开 CI 与公司源的隔离，确认部署环境、runner 配置与私有 fixture 目录均未被公开 workflow 触达"
    expected: "作答「是 / 否 / 不适用 + 理由」：公开 workflow 在实际运行中不会连接公司 Trino 集群、不会读取公司 catalog/表名/数据"
    why_human: "workflow 文本可证明「未引用」，不能证明 runner 上的环境变量注入、secret 与网络出口"
  - test: "G-02-D：核对私有 Trino 数据源边界与仓库外目录的真实内容，确认仓库外目录当前不含公司绑定数据、且 `ONTOFORGE_PRIVATE_FIXTURES` 入口的约定与实际部署一致"
    expected: "作答「是 / 否 / 不适用 + 理由」：SQL Server 只经仓库外私有 fixture 验证、CI 不运行该目标、仓库内无任何私有绑定或数据"
    why_human: "仓库外目录内容、git 历史中是否曾提交后删除私有绑定、以及文档与实际部署形态的一致性都不能由仓库内源码证明"
---

# Phase 2: Source Binding & Engine Verification Report

**Phase Goal:** As a ontology 开发者, I want to 把 object type / M:N link 绑定到 Trino 物理表，由 ontoforge 经自写 client 连接已有 Trino 并校验绑定，并在 Windows 本机用 SQLite 替身零外部依赖跑通, so that 定义能够使用真实数据源且支持本机验证.
**Mode:** mvp
**Verified:** 2026-11-08T09:17:46Z
**Source HEAD:** `87f216d8882efad8db3d0ba43514538236bf60e2`
**Status:** `human_needed`
**Re-verification:** Yes — after gap-closure round 02-11

---

## Goal Achievement

### Verdict Summary

上一轮报告的 `gaps_found` 由**一个 BLOCKER + 两个 judgment 旗标**构成。本轮结论：

- **BLOCKER 已闭合。** 上一轮 02-10.6 记为 `FAILED (BLOCKER)`，理由是 `ci/trino/iceberg/iceberg.properties` 只有 `fs.local.enabled=true` / `local:///warehouse`、缺 MinIO。**该描述对应的是旧源码状态**。当前源码该文件已改为 JDBC catalog + `s3://warehouse/` + `fs.s3.enabled=true` + `s3.endpoint=http://minio:9000`，且**无任何本地文件系统回退键**；`nightly.yml` 的 483/484 矩阵、独立 `report-failure` job、`issues: write` 唯一归属也都在位。本轮逐条以 `file:line` 独立复核（见下表 A1..A5），全部成立。该变更由 Phase 07.2 引入（`07.2-VERIFICATION.md` truth 1/3/4/6/8 记录了真实双版本 MinIO 运行证据）。
- **两个 judgment 项未被闭合，也未被误判为失败。** G-02-B / G-02-C / G-02-D 的机器可判定部分已由 02-11 的一次只读扫描完成并落盘；但它们的**结论位**在 `02-11-HUMAN-ACCEPTANCE.md` 中显式保持 `VERDICT: PENDING`（恰好 3 处）。这些是「原理上无法由机器判定、需人工签字」的判断项，按 ADR-550 D4 走 judgment-tier 软门：记为 human verification item，不静默吸收为 closed，也不判失败。
- 因此本轮 verdict = **`human_needed`**：无缺口、无 blocker，但有 3 条待人工作答的判断项（其中 2 条同时是 2 个 judgment-tier 禁止项）。

### Observable Truths — Roadmap Success Criteria（5 条）

| # | Truth | Status | Evidence |
|---|---|---|---|
| SC1 | 定义以 `backing: source \| editOnly` 显式声明数据来源，`SourceBinding` 绑定文件（按 profile 分目录）声明 source 绑定；decimal 精度、无时区 timestamp 的 `timezone`、collation、geopoint 不支持等契约违反时 lint 报 E，示例 ontology 仍 lint clean | ✓ VERIFIED | 02-01/03/07；`crates/ontoforge-schema/src/lint/binding/`（e13/e14/e16 + snapshots）；示例 ontology lint clean |
| SC2 | `ontoforge validate --online` 对真实 Trino 校验绑定（列存在、类型兼容）并报告不匹配；`--check-pk` 报告重复主键 | ✓ VERIFIED | 02-05/09；`crates/ontoforge/tests/cli_online.rs`、`cli_check_pk.rs`、`cli_trino_it.rs` |
| SC3 | Trino client 能执行查询、按超时与客户端断连取消、带 trace token 与 client tags；Trino 不可达时返回 `EngineUnavailable` | ✓ VERIFIED | 02-04/06；`crates/ontoforge-engine/src/trino/`（`cancellation_drain_*` 具名测试、timeout/drop、headers 断言） |
| SC4 | 自有 printer 生成的 SQL 经 proptest（任意字符串值往返不改变结构）与真实 Trino / SQLite 执行验证；SQLite 替身在 Windows `cargo test` 全绿 | ✓ VERIFIED | 02-02；printer prop 独立 AST + SQLite roundtrip；本轮回归 `cargo test --workspace --locked --offline --no-fail-fast` = **1528 passed / 0 failed / 119 ignored**（152 个 target，退出码 0） |
| SC5 | Linux CI 以 Trino + PG service container 跑集成测试，nightly 加跑 Iceberg | ✓ VERIFIED | `.github/workflows/ci.yml`（Trino/PG/online corpus）+ `.github/workflows/nightly.yml`（483/484 Iceberg + MinIO）；实跑证据见 07.2 的原始 run `41833307824`（job `126585725657` / `126585726171` / `126592749564`） |

### Observable Truths — PLAN 细化（104 条，按 plan 统计）

上一轮报告已逐条核对 02-01..02-10 的 98 条细化 truth。本轮为 re-verification，对已 VERIFIED 项做存在性 + 基本健全性回归抽查，对上一轮的 1 条 FAILED 与 2 条 UNCERTAIN 做完整复核，并新增核对 02-11 的 6 条。

| Plan | truth 数 | 本轮判定 |
|---|---|---|
| 02-01 | 10 | 全部 VERIFIED（回归抽查通过） |
| 02-02 | 11 | 全部 VERIFIED（回归抽查通过） |
| 02-03 | 13 | 12 VERIFIED + 1 UNCERTAIN（`02-05.13` 公开数据来源 judgment） |
| 02-04 | 11 | 全部 VERIFIED |
| 02-05 | 13 | 12 VERIFIED + 1 UNCERTAIN（即 02-03 的公开数据来源判断，跨 plan 登记） |
| 02-06 | 7 | 全部 VERIFIED |
| 02-07 | 9 | 全部 VERIFIED（其中 2 条计入 override） |
| 02-08 | 7 | 全部 VERIFIED |
| 02-09 | 8 | 全部 VERIFIED |
| 02-10 | 9 | 8 VERIFIED + 1 UNCERTAIN（`02-10.8` 私有 Trino 数据源边界 judgment） |
| 02-11 | 6 | 全部 VERIFIED（本轮逐条独立复核，见下表） |
| **合计** | **104** | **102 VERIFIED + 2 UNCERTAIN** |

**Score:** 107/109（5 条 SC + 104 条 PLAN truth）；2 条 UNCERTAIN 为 judgment 待签，不计入 verified；`behavior_unverified: 0`（无「present + wired 但行为未执行」的状态转移类 truth）。

### 02-11 的 6 条 truth（本轮逐条独立复核）

| # | Truth | Status | 独立复核证据 |
|---|---|---|---|
| 02-11.1 | 02-10.6 的 nightly Iceberg 合同在**当前源码**上逐条可核，每条断言带 `file:line` 记录在 `02-11-RECONCILIATION.md` | ✓ VERIFIED | 独立重跑 5 条断言（见「Behavioral Spot-Checks」表 A1..A5），与 `02-11-RECONCILIATION.md:35-39` 的判定逐条一致 |
| 02-11.2 | `02-11-RECONCILIATION.md` 写明 07.2 的 `passed` 只覆盖 MinIO/S3 + PG JDBC Iceberg 子范围，**不**闭合 ENGINE-05 私有 Trino 数据源边界；并交叉引用 07.2 的具体证据文件与行号 | ✓ VERIFIED | `02-11-RECONCILIATION.md:59-89`（`## Phase 07.2 Cross-Reference` + 「边界声明（原文，不得改写）」）；所引 `07.2-VERIFICATION.md:43/44/49` 逐行核对为 truth 2 / truth 3 / truth 8，内容与引用描述一致 |
| 02-11.3 | 记录**所有**可观测的实现路径偏差，并注明是与 02-10 计划文本的偏差还是与 02-10.6 truth 本身的偏差 | ✓ VERIFIED | `02-11-RECONCILIATION.md:43-56` 登记 D-01（services.postgres 迁移）、D-02（`fs.native-s3.enabled` → `fs.s3.enabled`、endpoint 主机名）、D-03（`report-failure` 触发条件收窄）、D-04（编排迁入 `ci/iceberg-services.sh`），每条均标注「不影响 truth 成立 / REFINED」 |
| 02-11.4 | 机器可判定的那一半由一次只读扫描落到 `02-11-source-scan.txt`，每个观测到的物理标识符与 `02-11-RECONCILIATION.md` 声明的中性词表**逐行相等** | ✓ VERIFIED | 本轮用脚本比对：scan 段 45 个标识符与 `## NEUTRAL VOCABULARY` 段 45 个词**集合完全相等**、无单侧多余；`## SUSPECTED NON-NEUTRAL` 为「空。」 |
| 02-11.5 | G-02-B / G-02-C / G-02-D 各自被转写成一个可判定的人工验收问题，每个条目有「问题原文 / 支撑证据 file:line / 机器不可判定的部分 / VERDICT 占位」四个字段，三条占位一律 `VERDICT: PENDING`，无人代签 | ✓ VERIFIED | `02-11-HUMAN-ACCEPTANCE.md` 三个子节（G-02-B / G-02-C / G-02-D），`VERDICT: PENDING` 恰好 3 处（文件内 :44 / :60 / :78），`## Explicitly Unverifiable by Machine` 与 `## Recording` 节明示「登记不由本 plan 或执行者预填」 |
| 02-11.6 | 本 plan **不引入任何产品代码变更**：`crates/`、`ci/`、`.github/`、`examples/` 与已跟踪 `docs/` 自起始 HEAD `9a6eaffe` 至当前工作树的 `git diff --name-only` 为空；`02-VERIFICATION.md` / `02-UAT.md` / `02-CONTEXT.md` / `02-RESEARCH.md` / `02-PATTERNS.md` / `REQUIREMENTS.md` / `ROADMAP.md` 均不被修改 | ✓ VERIFIED | `git diff 9a6eaffe060e93df1c19c79c7b25d3946a7d3229 --name-only -- crates/ ci/ .github/ examples/ docs/` 输出为空；同一 diff 全量列表中不含 `02-VERIFICATION.md` / `02-UAT.md` / `02-CONTEXT.md` / `02-RESEARCH.md` / `02-PATTERNS.md` / `REQUIREMENTS.md` / `ROADMAP.md`。工作树中另有 `.planning/STATE.md` 与 phase `progress.md` 的未提交改动，属 GSD orchestration / 进度记账，不在 product code 范围内、不影响本条判定 |

### 上一轮 3 条非 VERIFIED 项的本轮判定

| Truth | 上一轮 | 本轮 | 依据 |
|---|---|---|---|
| `02-10.6` nightly Iceberg 合同 | ✗ FAILED (BLOCKER) | ✓ **VERIFIED** | 见下方 A1..A5 五条断言的独立 file:line 复核；MinIO/S3 合同在当前源码中成立，实跑证据由 07.2 提供 |
| `02-05.13` / 02-03 公开数据来源 | ? UNCERTAIN (judgment) | ? **UNCERTAIN（需人工）** | 机器可判定部分已完成（45 标识符中性、`SUSPECTED NON-NEUTRAL` 空）；数据行来源与授权机器无法判定，结论位仍为 `VERDICT: PENDING` |
| `02-10.8` 私有 Trino 数据源边界 | ? UNCERTAIN (judgment) | ? **UNCERTAIN（需人工）** | 机器可判定部分已完成（门控 D-1 环境变量 + `#[ignore]` 双门控、D-2 贡献文档）；仓库外目录真实内容、git 历史与实际部署形态无法由仓库内源码证明，结论位仍为 `VERDICT: PENDING` |

---

## Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `.planning/phases/02-source-binding-engine/02-11-RECONCILIATION.md` | G-02-A 的可引用调和证据 | ✓ VERIFIED | 18698 bytes；A1..A5 判定表 + D-01..D-04 偏差台账 + 07.2 交叉引用 + 边界声明 + 45 词中性词表 + 本轮未执行清单 |
| `.planning/phases/02-source-binding-engine/02-11-HUMAN-ACCEPTANCE.md` | G-02-B/C/D 的人工验收清单 | ✓ VERIFIED | 12285 bytes；3 个子节 × 4 字段；`VERDICT: PENDING` 恰好 3 处 |
| `.planning/phases/02-source-binding-engine/02-11-source-scan.txt` | 只读扫描原始输出 | ✓ VERIFIED | 18858 bytes；`==== IDENTIFIERS ====`（45）/ `==== ENDPOINTS ====`（7）/ `==== PRIVATE PATH GATE ====` / `==== PROVENANCE BOUNDARY ====` 四段齐备 |
| `.planning/phases/02-source-binding-engine/02-11-SUMMARY.md` | 产出索引与四个 gap 的状态声明 | ✓ VERIFIED | 12460 bytes；G-02-A「已调和」、B/C/D「未关闭」逐条声明，未代签 |
| `ci/trino/iceberg/iceberg.properties` | JDBC catalog + S3/MinIO warehouse | ✓ VERIFIED | 13 行；JDBC 六键 :3-8、warehouse `s3://warehouse/` :9、`fs.s3.enabled=true` :10、`s3.endpoint=http://minio:9000` :11；**无** `fs.local.enabled` / `local:///warehouse` |
| `.github/workflows/nightly.yml` | 483/484 矩阵 + 独立 report-failure job | ✓ VERIFIED | 矩阵 :22 / :24；执行入口 :45；`report-failure:` :79；`permissions: issues: write` :83-84（全文件非注释行中唯一一处） |
| `crates/ontoforge/tests/private_fixtures.rs` | 私有 fixture 入口双门控 | ✓ VERIFIED | `FIXTURES_VAR` :14；缺变量即 panic :44 及其断言；`#[ignore = "needs ONTOFORGE_PRIVATE_FIXTURES"]` :69 |
| `CONTRIBUTING.md` | 私有 fixture 约定 | ✓ VERIFIED | 私有目录在仓库之外 :89；运行方式与退出码约定 :91；`ci/` 与 `crates/` 下的公开 fixture 与测试均为公开通用名 |

## Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| `02-11-source-scan.txt` | `02-11-RECONCILIATION.md` | `OBSERVED IDENTIFIERS` 段与 `NEUTRAL VOCABULARY` 段逐行相等 | ✓ WIRED | 本轮脚本比对：两侧各 45 个标识符，集合完全相等、无单侧多余；词表升序 |
| `02-11-RECONCILIATION.md` | `07.2-VERIFICATION.md` | 07.2 must-haves 表与 ENGINE-05 traceability 行被逐条引用并写明覆盖边界 | ✓ WIRED | 引用 `07.2-VERIFICATION.md:43/44/49`（truth 2 / 3 / 8）与 `:136` / `:174` 的边界原文；逐行核对内容相符 |
| `02-11-HUMAN-ACCEPTANCE.md` | `02-11-source-scan.txt` | 每个 G-02-x 条目的 `EVIDENCE:` 字段指回扫描文件的具体行 | ✓ WIRED | 9 条 `file:line` 证据引用，全部落在 scan 文件的 IDENTIFIERS / ENDPOINTS / PRIVATE PATH GATE 三段内 |
| `ci/trino/iceberg/iceberg.properties` | `ci/iceberg-services.sh` | runner 复制 catalog 目录并注入临时凭据后只读挂载 | ✓ WIRED | `ci/iceberg-services.sh:438`（纳入 catalog 目录）、`:471`（`-v ... :/etc/trino/catalog:ro`）、`:491`（`test -r /etc/trino/catalog/iceberg.properties`） |
| `crates/ontoforge-engine/src/trino/` | `.github/workflows/ci.yml` | CI 的 Trino 在线 corpus 调用同一 engine client | ✓ WIRED | `ci.yml:184/188`（`trino_it` / `cli_trino_it` `--ignored`）、`:176-180`（`trino_l2_acl_it` 数量门禁） |

## Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|---|---|---|---|---|
| `ci/trino/iceberg/iceberg.properties` | `default-warehouse-dir` | `s3://warehouse/`（MinIO 容器） | ✓ | ✓ FLOWING — 07.2 实测 `metadata_location` 以 `s3://warehouse/` 开头且桶内 metadata/data 对象非空 |
| `crates/ontoforge-engine/src/printer/` | 打印 SQL | 类型化 AST → SQL 文本 | ✓ | ✓ FLOWING — proptest 往返 + 真实 Trino / SQLite 执行 |
| `crates/ontoforge/tests/cli_check_pk.rs` | PK 查重结果 | 真实 Trino 查询 | ✓ | ✓ FLOWING（CI `--ignored` 执行） |
| `02-11-source-scan.txt` | 标识符 / 端点清单 | 一次只读源码扫描 | ✓ | ✓ FLOWING — 逐条带 `file:line` 来源，与当前源码一致（本轮抽样复核 `ci/trino/l2/seed.sql:5` 的 `ordеrs` 同形异码行、`ci/trino/l2/rules.json:18-19`、`ci/pg/seed.sql:172`） |

## Behavioral Spot-Checks

均为只读结构门禁，单条 < 10 秒，不启动容器、不连外部服务、不改状态。

| Behavior | Command | Result | Status |
|---|---|---|---|
| A1 483/484 矩阵 + 执行入口 | `grep -n 'trino: "48\|iceberg-services.sh nightly' .github/workflows/nightly.yml` | `:22`=483、`:24`=484、`:45` 跑 `ci/iceberg-services.sh nightly --trino "${{ matrix.trino }}"` | ✓ PASS |
| A2 JDBC catalog 六键 | `cat -n ci/trino/iceberg/iceberg.properties` | `:3-8` catalog type / catalog-name / driver-class / connection-url / connection-user / connection-password | ✓ PASS |
| A3 MinIO warehouse 且无本地回退 | `cat -n ci/trino/iceberg/iceberg.properties` + 键存在性检查 | `:9` `s3://warehouse/`、`:10` `fs.s3.enabled=true`、`:11` `s3.endpoint=http://minio:9000`、`:12` path-style、`:13` region；`fs.local.enabled` / `local:///warehouse` **不存在** | ✓ PASS |
| A4 独立 report-failure job 三步行为 | `grep -n 'report-failure:\|gh issue list\|gh issue comment\|gh issue create' .github/workflows/nightly.yml` | `:79` 独立 job；`:92` list → `:94` comment → `:97` create | ✓ PASS |
| A5 权限范围唯一 | `rg -v '^\s*#' .github/workflows/nightly.yml \| rg -c 'issues: write'` | = 1（`:84`）；workflow 级 `:10-11` 仅 `contents: read`；`iceberg` job 块内无 job 级 `permissions:` | ✓ PASS |
| C-1 公开 workflow 不引用私有入口 | `rg -n "private_fixtures\|ONTOFORGE_PRIVATE_FIXTURES" .github/` | 退出码 1（无匹配） | ✓ PASS |
| D-1 私有入口双门控 | `rg -n "ONTOFORGE_PRIVATE_FIXTURES\|#\[ignore" crates/ontoforge/tests/private_fixtures.rs` | `:14` / `:44` / `:69` 三处命中 | ✓ PASS |
| D-2 贡献文档约定 | `rg -n "ONTOFORGE_PRIVATE_FIXTURES" CONTRIBUTING.md` | `:89` / `:91` 两处命中 | ✓ PASS |
| 标识符词表双向相等 | 脚本比对 scan 段与 vocabulary 段 | 45 == 45，集合完全相等 | ✓ PASS |
| 零产品代码变更 | `git diff 9a6eaffe --name-only -- crates/ ci/ .github/ examples/ docs/` | 输出为空 | ✓ PASS |
| Windows workspace 回归 | `cargo test --workspace --locked --offline --no-fail-fast` | 1528 passed / 0 failed / 119 ignored，152 个 target，退出码 0 | ✓ PASS（`.planning/WINDOWS.md` 登记了 `cli_check_cardinality.rs` 两个 300 ms deadline 用例在全负载下的偶发 flake，隔离重跑绿） |

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
| BIND-01 | 02-01 / 02-03 / 02-11 | `backing` 必填 + 按 profile 分目录的 `SourceBinding` 绑定文件 | ✓ SATISFIED（功能） | binding 定义层 / profile 完备性 / aviation 绑定齐全；公开数据来源 judgment 单列为待人工（见 Human Verification #1） |
| BIND-02 | 02-03 / 02-07 / 02-08 | 类型契约（decimal 精度、无时区 timestamp 的 `timezone`、可选 collation、geopoint 不支持）违反时 lint 报 E | ✓ SATISFIED | `crates/ontoforge-schema/src/lint/binding/` e13/e14/e16 + 快照 + 兼容表文档 `docs/source-binding.md` |
| BIND-03 | 02-05 / 02-08 / 02-09 | `validate --online` 校验绑定、`--check-pk` 查重、drift 状态模型 | ✓ SATISFIED（Phase 02 范围） | CLI 在线校验与 PK 查重；drift 库函数与 `SourceSchemaDrift` 降级状态模型在位；**启动与定时 drift 执行属 Phase 7**（`REQUIREMENTS.md:195` 已如此登记） |
| ENGINE-01 | 02-04 / 02-06 / 02-09 / 02-10 | 自写 Trino client、超时、断连取消、trace token、两个服务身份 | ✓ SATISFIED | `crates/ontoforge-engine/src/trino/`；真实取消与 headers 断言测试 |
| ENGINE-02 | 02-04 / 02-06 / 02-09 | Trino 不可用时 fail-closed（`EngineUnavailable`），`/health` 与 describe 暴露引擎状态 | ✓ SATISFIED（Phase 02 范围） | 错误分类与 `status()` 在位；**`/health` 与 describe 暴露属 Phase 7**（`REQUIREMENTS.md:197` 已如此登记） |
| ENGINE-03 | 02-02 / 02-10 | 自有 printer 生成 SQL，值只经转义函数输出，注入防护由 proptest 与真实执行验证 | ✓ SATISFIED | printer prop 独立 AST + SQLite roundtrip；真实 Trino corpus |
| ENGINE-04 | 02-02 / 02-05 | SQLite 替身支撑 Windows 本机 `cargo test` | ✓ SATISFIED（Phase 02 范围） | 本机 1528 passed / 0 failed；**`serve --dev` 属 Phase 7**（`REQUIREMENTS.md:198` 已如此登记） |
| ENGINE-05 | 02-10 / 02-11 | Linux CI 跑 Trino + PG 集成测试，nightly 加跑 Iceberg；SQL Server 由私有 fixture 验证 | ◐ PARTIAL → 需人工 | **已闭合部分**：CI Trino + PG 集成测试（`ci.yml`）、nightly Iceberg + MinIO/S3 合同（当前源码 + 07.2 实跑证据）；**未闭合部分**：私有 Trino 数据源子句为 judgment，结论位 `VERDICT: PENDING`（`REQUIREMENTS.md:36` 仍为未勾选、`:200` 记为 Partial，与本判定一致） |

8 个 requirement ID 全部在 02-01..02-11 的 PLAN frontmatter 中被声明（BIND-01 ×3、BIND-02 ×4、BIND-03 ×4、ENGINE-01 ×4、ENGINE-02 ×4、ENGINE-03 ×2、ENGINE-04 ×2、ENGINE-05 ×2），且与 `REQUIREMENTS.md:193-200` 的 traceability 表逐行对应，**无 orphan**。

## Decision Coverage

`02-CONTEXT.md` 的可追踪决策由 GSD owner query 判定 15/15 honored（上一轮已核）。本轮额外确认 ADR-0022（`accepted`）确实把被忽略的 fixture 诊断从 Warning 降为 Info（`Severity::Info`、码仍为 `ONT-V17`），因此上一轮的 2 条 override 继续成立并被携带。

ADR-0017 要求的「JDBC catalog + MinIO」与当前源码一致，上一轮标记的合同偏差（`fs.local.enabled` / `local:///warehouse`）已不存在。

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
| 02-03 / judgment：公开数据来源 | 45 标识符全部中性、`SUSPECTED NON-NEUTRAL` 为空 | ? UNCERTAIN — NON-AUTHORITATIVE，`unverified-prohibition — human review recommended`，结论位 `VERDICT: PENDING` |
| 02-10 / judgment：公开 CI 不接公司源 | 端点枚举 + 门控 C-1（`.github/` 无私有入口引用） | ? UNCERTAIN — NON-AUTHORITATIVE，`unverified-prohibition — human review recommended`，结论位 `VERDICT: PENDING` |

两条 judgment 旗标**保持未关闭**：机器证据已到位，但结论需人签；不因门禁通过而静默吸收。

## Test Quality / Anti-Patterns

对 02-10 / 02-11 的 phase 产出做了 debt-marker 扫描：`TBD` / `FIXME` / `XXX` **零命中**（无未跟踪债务标记）。上一轮指出的「用符号存在替代行为」问题在 02-10 之后已由具名行为测试补齐（`cancellation_drain_*`、`drift_budget_*`、具名 `iceberg_table_is_readable` 等）。

## Advisory (New Scope, Unevidenced)

以下是 review / 历史产物层面的准确性瑕疵，**不影响 phase goal**，且多数落在本轮无写权限的历史产物上，如实登记但不阻断。

| # | Finding | Category | Why Advisory | 证据状态 |
|---|---|---|---|---|
| A-1 | `02-10-SUMMARY.md` 的多处描述已与当前源码不符：`:131` / `:147` / `:161` 仍写 nightly 用「本地 warehouse」与 `fs.local.enabled` / `local.location=local:///tmp/iceberg`；`:88` / `:149` 的计数写「573 passed / 0 failed / 11 ignored」（当前实测 1528 / 0 / 119） | other | 属历史 SUMMARY 的陈旧断言，不在本次写入范围（约束禁止修改 `*-SUMMARY.md`）；本报告已以当前源码事实取代 | 已由本轮源码核对确认 |
| A-2 | `02-10-SUMMARY.md:143` 描述的 PR `integration` 作业使用 `services.postgres` + `5432:5432`，当前 `ci.yml` 实际改为 `ci/query-services.sh postgres`（`:99`）等编排 | architectural | 同上；`02-11-RECONCILIATION.md` 的 D-01 只登记了 nightly 侧的编排迁移，未覆盖 PR `integration` 侧（对应 review 的 WR-06，disposition 仍为 open） | 已由本轮源码核对确认 |
| A-3 | `02-REVIEW-DISPOSITION.md` 中 12 条 warning / info 的 disposition 仍为 `open`（WR-01 / WR-02 已由本报告重写消解；WR-03..WR-09、IN-01..IN-03 涉及历史 SUMMARY 与 02-11 产物的引用细节） | other | 代码评审台账的闭环动作超出本轮 verifier 的写入范围 | 已核对 |
| A-4 | `02-11-SUMMARY.md:66` 把 7 条端点串概括为「全部指向 loopback / 容器内 `minio` / 仓库内桶前缀」，未提 `host.docker.internal`；scan 文件自身已在 `==== ENDPOINTS ====` 的补充节与 `## PROVENANCE BOUNDARY ====` 中如实登记该 Docker 标准宿主机名（`ci/query-services.sh:64` 的 sed 改写模板） | other | 属概括措辞不够精确，底层证据文件已完整披露；`host.docker.internal` 指向宿主机回环，非公司端点 | 已核对 |
| A-5 | `02-11-RECONCILIATION.md` / `02-11-SUMMARY.md` 中「13 条 G-A1..G-A5」的计数与门禁编号表述不自洽（对应 review 的 IN-01 / IN-02 / IN-03） | other | 计数描述瑕疵；逐条门禁与 file:line 证据本身经本轮独立复核全部成立 | 已核对 |

## Human Verification Required

3 项（G-02-B / G-02-C / G-02-D），全部来自 `02-11-HUMAN-ACCEPTANCE.md` 的三个子节，结论位当前为 `VERDICT: PENDING`。

### 1. G-02-B — 公开仓库的数据来源（02-03 禁止项）

**Test:** 核对 `examples/`、`ci/`（`ci/pg/seed.sql`、`ci/trino/l2/seed.sql`、`ci/query-services.sh`）、`crates/**/tests/` 的示例与种子数据，确认其**数据行**（不只是标识符）确为可公开的虚构数据，不含公司业务数据。
**Expected:** 作答「是 / 否 / 不适用 + 理由」——示例、绑定、fixture、测试与 CI 种子全部为可公开虚构数据。
**Why human:** 标识符中性是必要条件而非充分条件；机器无法区分「虚构但看起来真实」与「真实但已脱敏」（`02-11-source-scan.txt` 的 `## PROVENANCE BOUNDARY ====` 节已自陈此边界）。

### 2. G-02-C — 公开 CI 与公司源的隔离（02-10 禁止项）

**Test:** 核对公开 CI 的实际运行环境，确认部署环境、runner 配置与私有 fixture 目录均未被公开 workflow 触达。
**Expected:** 作答「是 / 否 / 不适用 + 理由」——公开 workflow 在实际运行中不会连接公司 Trino 集群，不会读取公司 catalog / 表名 / 数据。
**Why human:** workflow 文本可证明「未引用」（门控 C-1 退出码 1），不能证明 runner 上被注入的环境变量、secret 与网络出口（门禁 C-1 自陈不能证明的三项）。

### 3. G-02-D — 私有 Trino 数据源边界与 `ONTOFORGE_PRIVATE_FIXTURES`（02-10.8 / ENGINE-05）

**Test:** 核对仓库外 `ONTOFORGE_PRIVATE_FIXTURES` 目录的真实内容与 `CONTRIBUTING.md:85-91` 的约定，确认仓库外目录当前不含公司绑定数据、且该入口的运行约定与实际部署形态一致。
**Expected:** 作答「是 / 否 / 不适用 + 理由」——SQL Server 只经仓库外私有 fixture 验证、CI 不运行该测试目标、仓库内不出现任何私有绑定或数据。
**Why human:** 仓库外目录内容、git 历史中是否曾提交后删除过私有绑定、文档与实际部署形态的一致性，均不可由仓库内源码证明（门禁 D-1 / D-2 自陈不能证明的三项）。

**登记方式：** 在 `02-11-HUMAN-ACCEPTANCE.md` 对应子节把末行 `VERDICT: PENDING` 替换为「接受结论」或「拒绝结论」（附理由与日期），并同步更新本报告的 `unverified_prohibitions` 与 `human_verification` 节。三条结论全部签字前，本 phase 维持 `human_needed`。

## Gaps Summary

**无 BLOCKER、无 gaps。** 上一轮的 1 个 BLOCKER（`02-10.6` nightly Iceberg 合同）经本轮逐条 `file:line` 独立复核后判定为 VERIFIED：其「失败」描述对应的是 Phase 07.2 之前的旧源码状态，当前 `ci/trino/iceberg/iceberg.properties` 已是 JDBC catalog + MinIO/S3 warehouse 且无本地回退。

上一轮的 2 个 judgment 旗标（02-03 公开数据来源、02-10 公开 CI 边界）与 02-10.8 私有 Trino 数据源边界（ENGINE-05 子句）**既未被机器闭合、也未被判为失败**：机器可判定部分已由 02-11 的只读扫描完成并落盘，结论位显式保持 `VERDICT: PENDING`，等待人工签字。据此本 phase 的准确 verdict 是 **`human_needed`**，而非 `passed`（存在 human verification 项），也非 `gaps_found`（无失败 truth、无缺件 artifact、无断链）。

**Deferred：** 无。后续 milestone 阶段（07 / 07.1 / 07.2）均未在 ROADMAP 中承诺闭合本 phase 的三条 judgment 结论；`07.2-VERIFICATION.md` 的 `### Deferred Items` 节明确写明「Phase 01/02 历史 judgment 和私有 SQL Server 来源边界从未纳入 07.2 新增目标，不能用此处 passed 清除」，因此不构成可 defer 的匹配项。

---

_Verified: 2026-11-08T09:17:46Z_
_Verifier: Claude (gsd-verifier)_
_覆盖指纹由 `gsd-tools query verification.fingerprint` 计算并逐字复制；本轮未修改任何产品代码。_
