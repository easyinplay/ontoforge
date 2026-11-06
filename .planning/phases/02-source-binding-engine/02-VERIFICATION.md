---
phase: 02-source-binding-engine
verified: 2026-11-06T03:52:00Z
source_head: f8851ca073fed34e80e605f0ac91880259df7e12
status: gaps_found
score: 101/103 must-haves verified
covered_files: [".github/workflows/ci.yml",".github/workflows/nightly.yml",".planning/phases/02-source-binding-engine/02-01-PLAN.md",".planning/phases/02-source-binding-engine/02-01-SUMMARY.md",".planning/phases/02-source-binding-engine/02-02-PLAN.md",".planning/phases/02-source-binding-engine/02-02-SUMMARY.md",".planning/phases/02-source-binding-engine/02-03-PLAN.md",".planning/phases/02-source-binding-engine/02-03-SUMMARY.md",".planning/phases/02-source-binding-engine/02-04-PLAN.md",".planning/phases/02-source-binding-engine/02-04-SUMMARY.md",".planning/phases/02-source-binding-engine/02-05-PLAN.md",".planning/phases/02-source-binding-engine/02-05-SUMMARY.md",".planning/phases/02-source-binding-engine/02-06-PLAN.md",".planning/phases/02-source-binding-engine/02-06-SUMMARY.md",".planning/phases/02-source-binding-engine/02-07-PLAN.md",".planning/phases/02-source-binding-engine/02-07-SUMMARY.md",".planning/phases/02-source-binding-engine/02-08-PLAN.md",".planning/phases/02-source-binding-engine/02-08-SUMMARY.md",".planning/phases/02-source-binding-engine/02-09-PLAN.md",".planning/phases/02-source-binding-engine/02-09-SUMMARY.md",".planning/phases/02-source-binding-engine/02-10-PLAN.md",".planning/phases/02-source-binding-engine/02-10-SUMMARY.md","CONTRIBUTING.md","Cargo.lock","Cargo.toml","ci/check-tls-deps.sh","ci/ontoforge-server.ci.toml","ci/pg/iceberg_catalog.sql","ci/pg/query_corpus.sql","ci/pg/rowpred_corpus.sql","ci/pg/seed.sql","ci/query-services.sh","ci/release-package.ps1","ci/release-smoke.ps1","ci/server-services.sh","ci/test_tls_deps.py","ci/test_windows_server.py","ci/trino/catalog/demo.properties","ci/trino/catalog/editstore.properties","ci/trino/catalog/tpch.properties","ci/trino/iceberg/iceberg.properties","ci/trino/l2/access-control.properties","ci/trino/l2/config.properties","ci/trino/l2/rules.json","ci/trino/l2/seed.sql","ci/windows-server.py","clippy.toml","crates/ontoforge-engine/Cargo.toml","crates/ontoforge-engine/src/bind_check.rs","crates/ontoforge-engine/src/bind_check/compat_tests.rs","crates/ontoforge-engine/src/bind_check/tests.rs","crates/ontoforge-engine/src/config.rs","crates/ontoforge-engine/src/error.rs","crates/ontoforge-engine/src/lib.rs","crates/ontoforge-engine/src/pk_check.rs","crates/ontoforge-engine/src/printer/ast.rs","crates/ontoforge-engine/src/printer/call.rs","crates/ontoforge-engine/src/printer/dialect.rs","crates/ontoforge-engine/src/printer/escape.rs","crates/ontoforge-engine/src/printer/l2.rs","crates/ontoforge-engine/src/printer/mod.rs","crates/ontoforge-engine/src/printer/snapshots/ontoforge_engine__printer__tests__print_sqlite.snap","crates/ontoforge-engine/src/printer/snapshots/ontoforge_engine__printer__tests__print_trino.snap","crates/ontoforge-engine/src/printer/sqlite_ddl.rs","crates/ontoforge-engine/src/printer/tests.rs","crates/ontoforge-engine/src/sqlite/fixture.rs","crates/ontoforge-engine/src/sqlite/mod.rs","crates/ontoforge-engine/src/sqlite/tests.rs","crates/ontoforge-engine/src/trino/cancel.rs","crates/ontoforge-engine/src/trino/classify.rs","crates/ontoforge-engine/src/trino/client.rs","crates/ontoforge-engine/src/trino/decode.rs","crates/ontoforge-engine/src/trino/headers.rs","crates/ontoforge-engine/src/trino/l2_health.rs","crates/ontoforge-engine/src/trino/l2_monitor.rs","crates/ontoforge-engine/src/trino/mod.rs","crates/ontoforge-engine/src/value.rs","crates/ontoforge-engine/tests/bind_check_isolation.rs","crates/ontoforge-engine/tests/data/l2_corpus.json","crates/ontoforge-engine/tests/data/test-ca.pem","crates/ontoforge-engine/tests/doc_type_table.rs","crates/ontoforge-engine/tests/drift.rs","crates/ontoforge-engine/tests/drift_budget.rs","crates/ontoforge-engine/tests/l2_explain.rs","crates/ontoforge-engine/tests/l2_gate.rs","crates/ontoforge-engine/tests/l2_health.rs","crates/ontoforge-engine/tests/l2_monitor.rs","crates/ontoforge-engine/tests/pk_check.rs","crates/ontoforge-engine/tests/printer_literals.rs","crates/ontoforge-engine/tests/printer_prop.proptest-regressions","crates/ontoforge-engine/tests/printer_prop.rs","crates/ontoforge-engine/tests/server_config.rs","crates/ontoforge-engine/tests/snapshots/pk_check__pk_duplicates_sqlite.snap","crates/ontoforge-engine/tests/snapshots/pk_check__pk_duplicates_trino.snap","crates/ontoforge-engine/tests/snapshots/pk_check__pk_nulls_sqlite.snap","crates/ontoforge-engine/tests/snapshots/pk_check__pk_nulls_trino.snap","crates/ontoforge-engine/tests/sql_text_only_from_printer.rs","crates/ontoforge-engine/tests/sqlite_engine.rs","crates/ontoforge-engine/tests/trino_client.rs","crates/ontoforge-engine/tests/trino_it.rs","crates/ontoforge-engine/tests/trino_l2_acl_it.rs","crates/ontoforge-engine/tests/trino_resilience.rs","crates/ontoforge-schema/Cargo.toml","crates/ontoforge-schema/src/diag.rs","crates/ontoforge-schema/src/ir/action.rs","crates/ontoforge-schema/src/ir/attrs.rs","crates/ontoforge-schema/src/ir/basetype.rs","crates/ontoforge-schema/src/ir/binding.rs","crates/ontoforge-schema/src/ir/link.rs","crates/ontoforge-schema/src/ir/manifest.rs","crates/ontoforge-schema/src/ir/mod.rs","crates/ontoforge-schema/src/ir/names.rs","crates/ontoforge-schema/src/ir/object.rs","crates/ontoforge-schema/src/ir/scope.rs","crates/ontoforge-schema/src/lib.rs","crates/ontoforge-schema/src/lint/binding/e13.rs","crates/ontoforge-schema/src/lint/binding/e14.rs","crates/ontoforge-schema/src/lint/binding/e15.rs","crates/ontoforge-schema/src/lint/binding/e16.rs","crates/ontoforge-schema/src/lint/binding/mod.rs","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e13__tests__clean.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e13__tests__triggers.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e14__tests__clean.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e14__tests__triggers.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e15__tests__clean.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e15__tests__triggers.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e16__tests__clean.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e16__tests__triggers.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__w17__tests__clean.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__w17__tests__triggers.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__w17__tests__unreadable.snap","crates/ontoforge-schema/src/lint/binding/w17.rs","crates/ontoforge-schema/src/lint/config.rs","crates/ontoforge-schema/src/lint/e08.rs","crates/ontoforge-schema/src/lint/e09.rs","crates/ontoforge-schema/src/lint/mod.rs","crates/ontoforge-schema/src/lint/naming/mod.rs","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w01__tests__clean.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w01__tests__triggers.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w02__tests__clean.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w02__tests__triggers.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w03__tests__clean.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w03__tests__triggers.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w04__tests__clean.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w04__tests__triggers.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w05__tests__clean.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w05__tests__triggers.snap","crates/ontoforge-schema/src/lint/naming/w01.rs","crates/ontoforge-schema/src/lint/naming/w02.rs","crates/ontoforge-schema/src/lint/naming/w03.rs","crates/ontoforge-schema/src/lint/naming/w04.rs","crates/ontoforge-schema/src/lint/naming/w05.rs","crates/ontoforge-schema/src/lint/snapshots/ontoforge_schema__lint__e08__tests__clean.snap","crates/ontoforge-schema/src/lint/snapshots/ontoforge_schema__lint__e08__tests__triggers.snap","crates/ontoforge-schema/src/lint/snapshots/ontoforge_schema__lint__e09__tests__clean.snap","crates/ontoforge-schema/src/lint/snapshots/ontoforge_schema__lint__e09__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/mod.rs","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w06__tests__clean.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w06__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w07__tests__clean.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w07__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w08__tests__clean.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w08__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w09__tests__clean.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w09__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w10__tests__clean.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w10__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w11__tests__clean.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w11__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/w06.rs","crates/ontoforge-schema/src/lint/structure/w07.rs","crates/ontoforge-schema/src/lint/structure/w08.rs","crates/ontoforge-schema/src/lint/structure/w09.rs","crates/ontoforge-schema/src/lint/structure/w10.rs","crates/ontoforge-schema/src/lint/structure/w11.rs","crates/ontoforge-schema/src/lint/words.rs","crates/ontoforge-schema/src/loader/discover.rs","crates/ontoforge-schema/src/loader/dispatch.rs","crates/ontoforge-schema/src/loader/fixture.rs","crates/ontoforge-schema/src/loader/mod.rs","crates/ontoforge-schema/src/loader/parse.rs","crates/ontoforge-schema/src/loader/scope.rs","crates/ontoforge-schema/src/loader/sourcemap.rs","crates/ontoforge-schema/src/schema_export.rs","crates/ontoforge-schema/src/validate/actions.rs","crates/ontoforge-schema/src/validate/bindings.rs","crates/ontoforge-schema/src/validate/links.rs","crates/ontoforge-schema/src/validate/mod.rs","crates/ontoforge-schema/src/validate/objects.rs","crates/ontoforge-schema/tests/action_types.rs","crates/ontoforge-schema/tests/bindings.rs","crates/ontoforge-schema/tests/diagnostics.rs","crates/ontoforge-schema/tests/link_types.rs","crates/ontoforge-schema/tests/lint_examples.rs","crates/ontoforge-schema/tests/load_examples.rs","crates/ontoforge-schema/tests/loader.rs","crates/ontoforge-schema/tests/object_types.rs","crates/ontoforge-schema/tests/schema_export.rs","crates/ontoforge-schema/tests/scope_documents.rs","crates/ontoforge-schema/tests/snapshots/scope_documents__scope_diagnostics.snap","crates/ontoforge/src/cli.rs","crates/ontoforge/src/dev.rs","crates/ontoforge/src/dev_assets.rs","crates/ontoforge/src/dev_token.rs","crates/ontoforge/src/import.rs","crates/ontoforge/src/lib.rs","crates/ontoforge/src/lint_cmd.rs","crates/ontoforge/src/main.rs","crates/ontoforge/src/mcp.rs","crates/ontoforge/src/migrate.rs","crates/ontoforge/src/online.rs","crates/ontoforge/src/pipeline.rs","crates/ontoforge/src/reload.rs","crates/ontoforge/src/render.rs","crates/ontoforge/src/scope.rs","crates/ontoforge/src/serve.rs","crates/ontoforge/src/webhook_worker.rs","crates/ontoforge/tests/cli.rs","crates/ontoforge/tests/cli_check_pk.rs","crates/ontoforge/tests/cli_color.rs","crates/ontoforge/tests/cli_dev_token.rs","crates/ontoforge/tests/cli_import.rs","crates/ontoforge/tests/cli_l2_identity.rs","crates/ontoforge/tests/cli_lint.rs","crates/ontoforge/tests/cli_migrate.rs","crates/ontoforge/tests/cli_online.rs","crates/ontoforge/tests/cli_online_trino.rs","crates/ontoforge/tests/cli_reload.rs","crates/ontoforge/tests/cli_schema.rs","crates/ontoforge/tests/cli_scope.rs","crates/ontoforge/tests/cli_shutdown.rs","crates/ontoforge/tests/cli_stdio.rs","crates/ontoforge/tests/cli_trino_it.rs","crates/ontoforge/tests/cli_webhook_worker.rs","crates/ontoforge/tests/dev_lifecycle.rs","crates/ontoforge/tests/invalid_examples.rs","crates/ontoforge/tests/private_fixtures.rs","crates/ontoforge/tests/snapshots/cli__validate_unknown_field_human.snap","crates/ontoforge/tests/snapshots/cli_lint__e08_human.snap","crates/ontoforge/tests/snapshots/cli_lint__e14_human.snap","crates/ontoforge/tests/snapshots/cli_online__online_missing_column_human.snap","crates/ontoforge/tests/snapshots/cli_online_trino__unavailable_human.snap","crates/ontoforge/tests/snapshots/invalid_examples__v01-json-trailing-comma.snap","crates/ontoforge/tests/snapshots/invalid_examples__v01-yaml-duplicate-key.snap","crates/ontoforge/tests/snapshots/invalid_examples__v02-missing-kind.snap","crates/ontoforge/tests/snapshots/invalid_examples__v03-function-rule.snap","crates/ontoforge/tests/snapshots/invalid_examples__v03-reserved-kind.snap","crates/ontoforge/tests/snapshots/invalid_examples__v04-unknown-field.snap","crates/ontoforge/tests/snapshots/invalid_examples__v05-missing-backing.snap","crates/ontoforge/tests/snapshots/invalid_examples__v05-missing-primary-key.snap","crates/ontoforge/tests/snapshots/invalid_examples__v06-precision-on-string.snap","crates/ontoforge/tests/snapshots/invalid_examples__v06-unknown-base-type.snap","crates/ontoforge/tests/snapshots/invalid_examples__v07-bad-object-type-name.snap","crates/ontoforge/tests/snapshots/invalid_examples__v08-duplicate-binding.snap","crates/ontoforge/tests/snapshots/invalid_examples__v08-duplicate-object-type.snap","crates/ontoforge/tests/snapshots/invalid_examples__v08-link-end-clashes-property.snap","crates/ontoforge/tests/snapshots/invalid_examples__v09-binding-unknown-target.snap","crates/ontoforge/tests/snapshots/invalid_examples__v09-link-unknown-object-type.snap","crates/ontoforge/tests/snapshots/invalid_examples__v09-rule-unknown-property.snap","crates/ontoforge/tests/snapshots/invalid_examples__v10-float-primary-key.snap","crates/ontoforge/tests/snapshots/invalid_examples__v10-primary-key-not-required.snap","crates/ontoforge/tests/snapshots/invalid_examples__v11-backing-on-one-to-many.snap","crates/ontoforge/tests/snapshots/invalid_examples__v11-foreign-key-type-mismatch.snap","crates/ontoforge/tests/snapshots/invalid_examples__v11-many-to-many-with-foreign-key.snap","crates/ontoforge/tests/snapshots/invalid_examples__v12-create-missing-primary-key.snap","crates/ontoforge/tests/snapshots/invalid_examples__v12-current-time-on-string.snap","crates/ontoforge/tests/snapshots/invalid_examples__v12-modify-assigns-primary-key.snap","crates/ontoforge/tests/snapshots/invalid_examples__v12-value-source-type-mismatch.snap","crates/ontoforge/tests/snapshots/invalid_examples__v13-comparison-type-mismatch.snap","crates/ontoforge/tests/snapshots/invalid_examples__v13-non-boolean-condition.snap","crates/ontoforge/tests/snapshots/invalid_examples__v13-objects-not-object-reference.snap","crates/ontoforge/tests/snapshots/invalid_examples__v13-syntax-error.snap","crates/ontoforge/tests/snapshots/invalid_examples__v13-unknown-parameter.snap","crates/ontoforge/tests/snapshots/invalid_examples__v14-include-escapes-root.snap","crates/ontoforge/tests/snapshots/invalid_examples__v16-binding-outside-bindings-dir.snap","crates/ontoforge/tests/snapshots/invalid_examples__v16-fixture-escapes-root.snap","crates/ontoforge/tests/snapshots/invalid_examples__v16-source-without-binding.snap","crates/ontoforge/tests/stdio_security_support/mod.rs","crates/ontoforge/tests/unit/dev.test.rs","crates/ontoforge/tests/unit/serve.test.rs","docs/source-binding.md","examples/ontologies/aviation-json/actions.json","examples/ontologies/aviation-json/bindings/dev/bindings.json","examples/ontologies/aviation-json/bindings/dev/fixtures/aircraft.csv","examples/ontologies/aviation-json/bindings/dev/fixtures/flight_crew.csv","examples/ontologies/aviation-json/bindings/dev/fixtures/flights.csv","examples/ontologies/aviation-json/bindings/dev/fixtures/maintenance_records.csv","examples/ontologies/aviation-json/bindings/dev/fixtures/pilots.csv","examples/ontologies/aviation-json/links.json","examples/ontologies/aviation-json/objects.json","examples/ontologies/aviation-json/ontology.yaml","examples/ontologies/aviation/actions.yaml","examples/ontologies/aviation/bindings/dev/aircraft.yaml","examples/ontologies/aviation/bindings/dev/fixtures/aircraft.csv","examples/ontologies/aviation/bindings/dev/fixtures/flight_crew.csv","examples/ontologies/aviation/bindings/dev/fixtures/flights.csv","examples/ontologies/aviation/bindings/dev/fixtures/maintenance_records.csv","examples/ontologies/aviation/bindings/dev/fixtures/pilots.csv","examples/ontologies/aviation/bindings/dev/flight-crew.yaml","examples/ontologies/aviation/bindings/dev/flight.yaml","examples/ontologies/aviation/bindings/dev/maintenance-record.yaml","examples/ontologies/aviation/bindings/dev/pilot.yaml","examples/ontologies/aviation/links.yaml","examples/ontologies/aviation/objects.yaml","examples/ontologies/aviation/ontology.yaml","schemas/ontoforge-definition.schema.json","schemas/ontoforge-manifest.schema.json","schemas/ontoforge-scope.schema.json"]
covered_digest: "v4:sha256:f54f70cf20d7c185133f52923855d75a14f3b8f5e4e6f803da8cad8d474136f2"
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
decision_coverage: {honored: 15, total: 15, not_honored: []}
re_verification:
  previous_status: passed
  previous_score: 15/15
  gaps_closed:
    - "当前local Iceberg 483/484真实CTAS与精确命名测试证据已补齐"
    - "MVP User Story已等义规范并经owner query返回true"
    - "当前f885源码Windows/Ubuntu/Trino/PG回归证据已补齐"
  gaps_remaining:
    - "ENGINE-05的私有SQL Server fixture与数据来源仍未在本轮执行或人工验收；MinIO/S3子范围已由Phase07.2独立验收闭合"
  regressions: []
gaps:
  - truth: "nightly 工作流在 Trino 483 与 484 上额外跑 Iceberg（JDBC catalog + MinIO），失败时由独立的 `report-failure` job 自动开 issue（同一标签下已有未关闭 issue 则追加评论）；`issues: write` 只授予这个 job（GitHub Actions 的 permissions 只能设在 workflow 或 job 层），测试 job 只有 `contents: read`（E25）"
    status: partial
    reason: "Phase07.2已对MinIO/S3、JDBC Iceberg、483/484、restart、S3对象与清理完成独立9/9验收；本报告不把该子范围扩展为私有SQL Server环境或数据来源人工验收。"
    artifacts:
      - path: "ci/trino/iceberg/iceberg.properties"
        issue: "fs.local.enabled=true、local:///warehouse；无MinIO/S3 endpoint与warehouse"
      - path: ".github/workflows/nightly.yml"
        issue: "当前Iceberg fixture挂载本地warehouse，没有MinIO启动/验证路径"
      - path: ".planning/adr/0017-engine-and-trino-connection.md"
        issue: "accepted决策仍要求JDBC catalog+MinIO"
    missing:
      - "按现有合同实现受信MinIO路径并在Trino483/484实际验收，或由需求所有者接受后以正式决策修订该合同；当前不能自行豁免"
unverified_prohibitions:
  - plan: "02-03"
    statement: "公开仓库中的示例、绑定、fixture、测试与CI种子不得出现公司物理名或真实业务数据"
    verification: judgment
    verdict: "NON-AUTHORITATIVE: 当前源码及虚构fixture声明支持该判断，无法凭grep证明全部数据原创来源"
    flag: "unverified-prohibition — human review recommended"
  - plan: "02-10"
    statement: "公开CI不得连接公司Trino集群或出现公司catalog/表名/数据"
    verification: judgment
    verdict: "NON-AUTHORITATIVE: workflow使用隔离本地fixture且不调用private_fixtures；未作公司环境人工核验"
    flag: "unverified-prohibition — human review recommended"
---

# Phase 2: Source Binding & Engine Verification Report

**Phase Goal:** As a ontology 开发者, I want to 把 object type / M:N link 绑定到 Trino 物理表，由 ontoforge 经自写 client 连接已有 Trino 并校验绑定，并在 Windows 本机用 SQLite 替身零外部依赖跑通, so that 定义能够使用真实数据源且支持本机验证.
**Status:** gaps_found
**Re-verification:** 历史报告的当前源码复验，原15/15是汇总，不是98条PLAN truth的逐项行为证明。

## User Flow Coverage

| Step | Expected | Evidence | Status |
|---|---|---|---|
| 编写source/editOnly与profile binding | 合法定义加载，错误带精确诊断 | bindings/object/link tests；U:2880/3028/2939/3033 | VERIFIED |
| Windows本机在线校验fixture | SQLite替身无外部依赖，列缺失/类型转换失败可定位 | cli_online、sqlite_engine；最终W workspace成功 | VERIFIED |
| 指向Trino校验及显式检查PK | 真实描述列/类型与重复/NULL PK诊断 | I:1085/1086；U:1474/1475 | VERIFIED |
| 不可达/超时/取消 | fail-closed并取消已接受查询 | U:2315/2318/2340；I:1050 | VERIFIED |
| Iceberg连接器回归 | 当前local JDBC fixture可读，计划MinIO仍须闭环 | 两版本各CTAS29行与精确test1；MinIO无实现 | FAILED（MinIO合同） |
| 用户故事outcome | 定义可使用真实源并支持本机验证 | 核心功能成立；phase完整合同仍有下述BLOCKER | PARTIAL |

本次 `query user-story.validate --story <ROADMAP goal> --pick valid` 返回 `true`。不再沿用旧报告“valid:false仍通过”的豁免。

## Evidence Provenance

只读复核，不重跑Cargo/Docker，不启动服务，不声称Warp/ralph-loop。当前源码 `f8851ca073fed34e80e605f0ac91880259df7e12`；精确命名日志与断言是证据，SUMMARY只用于定位。

- **U**：`target/milestone-ci-41759804733-ubuntu.log`；**W**：同前缀 `windows.log`；**I**：同前缀 `integration.log`。U/I:n是原始日志行号。
- CI `41759804733` head f8851ca：Windows、Ubuntu、integration、phase7-real-services四必需jobs均success；包括workspace、fmt、Clippy。ignored/discovery不等于执行通过。
- 当前local Iceberg原始证据：`target/milestone-audit-evidence-20261104/trino-{483,484}-{ctas,fixture,iceberg}.log`、`trino-*-info.json`，执行脚本 `target/milestone-audit-iceberg.sh`。
- 动态runner原始exit **1**：执行期间HEAD从868643c变到f8851ca。完整diff只含 `crates/ontoforge/tests/cli_color.rs` 类型别名（2增1删），生产/CI/lock/fixture与本动态命令输入完全等价；独立 `final-check.log` exit0只证明provenance/扫描/清理，不改写原runner状态。新cli_color由最终f885 CI覆盖。
- 旧历史nightly `39221684753`与UAT只说明历史运行，本轮local双版本证据不冒称新触发GitHub nightly。历史失败通知分支记录在旧UAT/报告，不扩写为本轮实际开issue。
- 先前独立owner已读全部源码/逐PLAN artifact、link与决策；本次继承 `PHASE-02-04-REVERIFY.md` 可审计事实并独立核查新增证据。

## Goal Achievement

### Observable Truths

5条roadmap SC + 98条PLAN细化truth，共103条。SC为总合同、PLAN为更细要求；两处warning字面由accepted ADR-0022覆盖，明确计PASSED(override)。1条FAILED、2条数据来源judgment为UNCERTAIN，其余100条含2 override计入verified。实际行为未执行数为0；UNCERTAIN不伪装成代码stub。

| # | Truth | Status | Evidence |
|---|---|---|---|
| SC1 | 定义以 `backing: source \| editOnly` 显式声明数据来源，`SourceBinding` 绑定文件（按 profile 分目录）声明 source 绑定；decimal 精度、无时区 timestamp 的 `timezone`、collation、geopoint 不支持等契约违反时 lint 报 E，示例 ontology 仍 lint clean | VERIFIED | 02-01/03/07；U:3028/2939/3033 backing负向、lint binding trigger/clean与U:2955 aviation_lints_clean |
| SC2 | `ontoforge validate --online` 对真实 Trino 校验绑定（列存在、类型兼容）并报告不匹配；`--check-pk` 报告重复主键 | VERIFIED | 02-05/09；I:1085 check_pk_finds_duplicate_and_null_keys_on_ci_trino、I:1086 validate_online_against_ci_trino；U:1474/1475不匹配 |
| SC3 | Trino client 能执行查询、按超时与客户端断连取消、带 trace token 与 client tags；Trino 不可达时返回 `EngineUnavailable` | VERIFIED | 02-04/06；U:2269 headers、U:2340 timeout DELETE、U:2318 future drop DELETE；I:1050真实取消 |
| SC4 | 自有 printer 生成的 SQL 经 proptest（任意字符串值往返不改变结构）与真实 Trino / SQLite 执行验证；SQLite 替身在 Windows `cargo test` 全绿 | VERIFIED | 02-02；printer_prop独立AST+SQLite roundtrip、I:1051真实Trino corpus；W最终workspace成功 |
| SC5 | Linux CI 以 Trino + PG service container 跑集成测试，nightly 加跑 Iceberg | VERIFIED | CI integration实际Trino+PG；nightly配置仍有483/484 Iceberg，本轮两个local fixture真实CTAS/测试通过；MinIO具体PLAN偏差另列FAILED |
| 02-01.1 | 每个 ObjectType 文档必须声明 `backing: source \| editOnly`；缺失时 `ontoforge validate` 报 ONT-V05 `missing required field \\`backing\\``，带文件名与 line:col（D-02） | VERIFIED | ir/object.rs required backing + validate/objects；U:3028 object_without_backing_is_v05，真实诊断快照 |
| 02-01.2 | `cardinality: manyToMany` 的 LinkType 必须声明 backing，缺失报 ONT-V05；oneToOne / oneToMany 写了 backing 报 ONT-V11（D-02：1:1 / 1:N 跟随外键，不单独绑定） | VERIFIED | ir/link.rs / validate/links；U:2939 many_to_many_without_backing_is_v05、U:2940/2942 non-M:N backing拒绝 |
| 02-01.3 | backing 的非法取值报 ONT-V06，help 列出 `source`、`editOnly` | VERIFIED | validate/objects；U:3033 unknown_backing_value_is_v06 |
| 02-01.4 | aviation 示例中 Airport 为 editOnly，Aircraft / Flight / Pilot / MaintenanceRecord 与 flightCrew 为 source；aviation 与 aviation-json 仍加载为相同 IR，`ontoforge validate` 无诊断 | VERIFIED | examples双格式定义；U:3016 aviation_backing_values；load_examples双格式IR比较 |
| 02-01.5 | Phase 1 全部 27 个 invalid 样例、示例与 Rust 内联测试定义已迁移（SUMMARY 逐个列出 27 个目录的处理结果）；迁移前后每个已有快照的 code / severity / message / help 不变，human 快照的码行、`×` 消息行与 `help:` 行同样不变，只允许行号、列号、偏移量与源码摘录行变化（E19） | VERIFIED | 历史迁移审计保持27个invalid目录语义；当前invalid_examples快照全通过，未把后续新需求引入的快照演进称为原27迁移差异 |
| 02-01.6 | 导出的 `schemas/ontoforge-definition.schema.json` 中 ObjectType 的 required 含 backing，LinkType 含可选 backing | VERIFIED | schema_export.rs / definition schema required；schema_export测试与当前JSON Schema相符 |
| 02-01.7 | ADR-0021 记录绑定文件形状、backing 必填（作为 ADR-0012 向后兼容惯例的有意例外）、profile 选定顺序、decimal 规则、timezone 解释、诊断码与退出码分配、新 engine crate；ADR-0015 标注被取代的内联 source 条款 | VERIFIED | ADR-0021 accepted及ADR-0015 superseded条款；后继ADR-0022明确修订ignored fixture诊断级别 |
| 02-01.8 | REQUIREMENTS.md 的 BIND-01 文字改为独立绑定文件 + backing 显式声明；ENGINE-02、BIND-03、ENGINE-04 三行 traceability 各自注明 Phase 2 的交付范围与后续 phase 指针（`/health` 与 describe → Phase 7 API-01；启动 / 定时 drift → Phase 7；`serve --dev` → Phase 7 DX-01）；ROADMAP Phase 2 成功标准 1 改为「`SourceBinding` 绑定文件 + 必填 `backing`」（E6） | VERIFIED | 当前REQUIREMENTS BIND-01与ENGINE-02/BIND-03/ENGINE-04 traceability，ROADMAP SC1；Phase7负责后继运行接线 |
| 02-01.9 | `ontoforge_schema::loader::read_limited(path, follow_links, max_bytes)` 与 `ReadFailure` 公开，上限与是否跟随符号链接由调用方传入，loader 与 lint 配置读取的既有行为不变；02-04（服务配置、密钥文件、CA）与 02-05（fixture）复用它，不再各写一份有界读取（E10） | VERIFIED | loader::read_limited→config/fixture复用；U:2823 custom_limit、U:2824 follows_links_only_when_asked、U:2825 missing_and_directory |
| 02-01.10 | `ProfileName`（`^[a-z][a-z0-9_-]{0,31}$`，非法 → ONT-V16）与诊断码常量 ONT-V16..V20 在 wave 1 落地，wave 2 并行的 02-03（绑定与 manifest）与 02-04（服务配置 `profiles` 键）共用同一个类型（E14） | VERIFIED | ir/names.rs ProfileName与diag.rs常量；U:2888 profile_name_length_boundaries、U:2188 profile_key_must_be_profile_name |
| 02-02.1 | 新 crate `ontoforge-engine` 是 workspace 成员（`[lints] workspace = true`），依赖 `ontoforge-schema`、不依赖 `ontoforge-query`；Windows 本机 `cargo test -p ontoforge-engine` 不需要 Docker 即全绿（ENGINE-04） | VERIFIED | workspace Cargo.toml→engine/Cargo.toml；当前W workspace与Clippy成功，engine普通tests不需Docker |
| 02-02.2 | `Engine` trait 的 `execute` 只接受 `PrintedSql`；`PrintedSql::new` 是 `printer` 模块私有函数（不带任何可见性修饰，子模块 `printer::dialect` / `printer::sqlite_ddl` 可调用），printer 之外的构造由编译器拒绝；`sqlite_ddl` 为 `pub(crate)`；源码扫描测试保留作兜底（ENGINE-03，E4） | VERIFIED | printer/mod.rs PrintedSql私有构造；sql_text_only_from_printer源码guard与编译器可见性边界 |
| 02-02.3 | `Engine::describe_tables(tables, ctx)` 一次描述同一 catalog 下的多张表，返回 `BTreeMap<TableRef, Vec<PhysicalColumn>>`，不存在的表不在 map 中；跨 catalog 的输入返回 `EngineError::Config`（供 02-05 按 catalog 批量取元数据，E22） | VERIFIED | lib.rs Engine / sqlite+trino实现；U:2224 describe_tables_rejects_mixed_catalogs、U:2262 describe_tables_batches_one_catalog |
| 02-02.4 | `EngineError` 实现 `Clone`；`EngineError::is_engine_wide()` 对 Unavailable、Unauthorized、Cancelled、Config 为真（整轮中止），对 Query、Timeout、Protocol、Print、Fixture 为假（只影响对应目标）（E3） | VERIFIED | error.rs Clone / is_engine_wide；U:2227 engine_wide_classification、U:2228 error_codes_are_upper_snake |
| 02-02.5 | `QueryContext::new(trace_token, client_tags)` 以 `DEFAULT_QUERY_TIMEOUT_MS`（30 000）、`DEFAULT_MAX_ROWS`（10 000）、`DEFAULT_MAX_BYTES`（16 MiB）为缺省，这三个常量是查询上限缺省值的唯一定义（E11） | VERIFIED | lib.rs DEFAULT_* / QueryContext；U:2240 query_context_defaults |
| 02-02.6 | decimal 文本语法 `^-?[0-9]+(\\.[0-9]+)?$` 只有一个实现 `value::split_decimal`，printer 的字面量校验与 02-05 的 fixture 转换共用（E7） | VERIFIED | value.rs::split_decimal被printer和sqlite/fixture调用；value split_decimal_rules测试通过 |
| 02-02.7 | 字符串字面量与标识符只经 `quote_string` / `quote_ident` 两个函数输出：`'` 写成 `''`、`\"` 写成 `\"\"`；含 NUL 的字符串、空或含 NUL 的标识符返回 `PrintError` 而不是输出 SQL | VERIFIED | printer/escape.rs；U:2237 nul_in_string_is_print_error、U:2226 empty_identifier_is_print_error |
| 02-02.8 | printer 的输出经 proptest 验证：任意（不含 NUL 的）字符串作为 WHERE 右值时，sqlparser 解析出恰好 1 条语句、右值等于原字符串、把该值换成固定串后 AST 结构不变；同一字面量在 SQLite 上 `SELECT` 回读等于原字符串；任意非空标识符作别名时 SQLite 回读的列名等于原串 | VERIFIED | printer_prop.rs独立sqlparser AST/SQLite roundtrip，U string_literal_keeps_statement_structure/string_literal_round_trips_on_sqlite/identifier_round_trips_on_sqlite实际ok |
| 02-02.9 | printer 对两种方言输出不同：Trino 表引用为 `\"cat\".\"sch\".\"tbl\"`，SQLite 为 `\"cat__sch\".\"tbl\"`；Trino 的 decimal / double / date / timestamp 字面量带类型前缀；两种方言的 ORDER BY 一律显式写 NULLS FIRST / NULLS LAST；非有限 double 被拒绝；i64::MIN 不被打印成会溢出的整数文本 | VERIFIED | printer/dialect.rs/escape.rs typed literals/null ordering；printer_literals与printer unit snapshots通过 |
| 02-02.10 | SQLite 替身的执行在单个连接上进行（ATTACH 与内存库是每连接状态）；`execute` 用 sqlx 的流式 `fetch` 逐行读取，至多读到第 `max_rows + 1` 行即停止，行数超过 `max_rows` 或累计 TEXT / BLOB 字节超过 `max_bytes` 时截断并置 `truncated = true`，内存占用不随结果集总量增长（E24） | VERIFIED | sqlite/mod.rs单连接fetch与限额；U sqlite::tests::max_rows_truncates / max_bytes_truncates / query_only_after_load通过 |
| 02-02.11 | engine crate 中 `src/printer/` 之外不存在以 SQL 关键字开头的字符串字面量（源码级测试守护） | VERIFIED | tests/sql_text_only_from_printer.rs实际源码扫描命名test通过 |
| 02-03.1 | 开发者可以在 `bindings/<profile>/` 下写 `kind: SourceBinding` 文档，把 object type 或 manyToMany link 绑定到 `source: { catalog, schema, table }`；profile 由目录名推出，定义文件中不出现任何物理名（D-01、D-04） | VERIFIED | loader/dispatch / ir/binding；U:2880 flight_binding_loads_under_dev_profile |
| 02-03.2 | `columns` 只需列出与属性同名列不同的映射，未列出的属性按同名列匹配（D-03）；M:N link 绑定的 `columns` 必须恰好有 `from` 与 `to` | VERIFIED | Ontology::bound_columns/validate/bindings；U:2871 bound_columns_object_and_link、U:2881 link_columns_need_from_and_to |
| 02-03.3 | SourceBinding 出现在 `bindings/<profile>/` 之外、ontology 定义出现在 `bindings/` 下、profile 目录名不符合 `[a-z][a-z0-9_-]{0,31}`（02-01 的 `ProfileName`），均报 ONT-V16 并带 line:col | VERIFIED | loader/discover路径分类；U:2870 binding_outside_bindings_dir_is_v16、U:2874 definition_inside_bindings_dir_is_v16、U:2868 bad_profile_dir_is_v16 |
| 02-03.4 | 同一 profile 内对同一 target 的重复绑定报 ONT-V08（指出首次定义位置）；绑定指向不存在的 object type / link / 属性报 ONT-V09 并给近似名 | VERIFIED | validate/bindings；U:2875 duplicate_binding_is_v08、U:2899 unknown_target_is_v09、U:2872 column_key_unknown_property_is_v09 |
| 02-03.5 | 选定 profile 的顺序为 `--profile` > `ONTOFORGE_PROFILE` > manifest `defaultProfile`（D-05）；`backing: source` 的类型在选定 profile 中没有绑定、`backing: editOnly` 的类型有绑定（任一 profile）、选定 profile 不存在或未选定 profile 而存在 source 类型，均报 ONT-V16（D-02） | VERIFIED | CLI profile precedence与validate/bindings完整性；U profile_precedence、U:2890 source_type_without_binding_is_v16、U:2876 edit_only_type_with_binding_is_v16 |
| 02-03.6 | `fixture.file` 相对绑定文件所在目录解析，只允许 `.csv` / `.json`，不得逃出 ontology 根目录，必须是不超过 4 MiB 的常规文件，否则 ONT-V16；存在性只查文件元数据（磁盘）或文件表（内存），不读取内容（E23） | VERIFIED | fixture元数据与canonical路径约束；U:2878 fixture_reference_rules、U:2885 oversized_fixture_is_v16、U:2891 symlinked_fixture_is_v16 |
| 02-03.7 | 只有 `bindings/<profile>/fixtures/` 下的文件不被当作定义文件扫描；其他位置名为 `fixtures` 的目录（如 ontology 根下的 `fixtures/`）照常加载（E5） | VERIFIED | loader/discover唯一排除位置；U:2798 only_binding_fixture_directories_are_excluded、U:2893 top_level_fixtures_dir_still_loaded |
| 02-03.8 | `PropertyDef::effective_decimal()` 与常量 `DEFAULT_DECIMAL_PRECISION`（38）/ `DEFAULT_DECIMAL_SCALE`（10）是 decimal 缺省的唯一定义，02-05 / 02-07 / 02-08 只调用它（E7） | VERIFIED | ir/object.rs effective_decimal唯一默认；engine/fixture、bind_check、lint e13共用；相关decimal兼容/边界测试通过 |
| 02-03.9 | `loader::fixture_header(file, text)` 是 fixture 表头语义的唯一实现：CSV 只解析首条记录并去掉 UTF-8 BOM，JSON 要求顶层为对象数组、取各对象键的并集（首次出现顺序）；02-05 的引擎加载与 02-07 的 W17 共用（E8） | VERIFIED | loader/fixture.rs共享header；U:2803 fixture_header_matches_the_engine_rules、U:2804 fixture_header_rules |
| 02-03.10 | `Ontology::bound_columns(binding)` 是期望物理列名推导的唯一实现：object 绑定逐属性给出 `column_for(key)`、是否显式映射与属性定义，M:N 绑定给出 `from` / `to` 及对应端主键属性；02-07 的 W17 与 02-05 的 `bind_check::expected_columns` 复用，引擎只补物理类型（E9） | VERIFIED | ir/binding.rs bound_columns→lint W17和bind_check；U:2871 bound_columns_object_and_link |
| 02-03.11 | `timezone` 只接受 IANA 时区名或 `aware`，`precision` / `scale` 只允许出现在 decimal（或元素为 decimal 的 array）属性上，否则分别报 ONT-V16 / ONT-V06 | VERIFIED | validate/bindings/objects；U:2892 timezone_value_rules、U:2887 precision_on_non_decimal_is_v06 |
| 02-03.12 | aviation 与 aviation-json 的 dev profile 绑定全部 source 类型与 flightCrew，`ontoforge validate` 与 `ontoforge lint` 对两者均无诊断，二者 IR 相同 | VERIFIED | examples双格式fixture/definitions，U:2955 aviation_lints_clean、U:2963 aviation_dev_profile_binds_every_source_type |
| 02-03.13 | 库函数不读取环境变量：profile 的环境变量由 clap 的 `env` 读取后以参数传入（`set_var` 禁令，RESEARCH Pitfall 9） | VERIFIED | clap env→参数传递；schema loader无读env profile分支；clippy禁止set_var |
| 02-04.1 | `TrinoEngine` 经自写 HTTP client 执行 `PrintedSql`：`POST {url}/v1/statement` 后循环 `GET nextUri` 直到响应不再含 `nextUri`，跨页拼接 `columns` 与 `data`（ENGINE-01） | VERIFIED | trino/mod.rs statement_url→client::send→GET loop；U:2271 select_one_round_trip、U:2256 data_spans_pages；I:1047真实SELECT |
| 02-04.2 | 每个请求无状态地带上 `X-Trino-User`、`X-Trino-Source: ontoforge`、`X-Trino-Trace-Token`（来自 QueryContext）、`X-Trino-Client-Tags`、`X-Trino-Time-Zone: UTC` 与 `X-Trino-Session: query_max_execution_time=<n>s`；响应中的 `X-Trino-Set-*` 不回写任何状态 | VERIFIED | trino/headers.rs；U:2269 request_headers、U:2272 set_session_header_is_ignored |
| 02-04.3 | 支持三种认证：`none`（只发 `X-Trino-User`）、`basic`（用户名密码，仅 https）、`bearer`（JWT / OAuth2 token，仅 https）；http URL 搭配 basic 或 bearer 在配置期报错，并且 `TrinoEngine::new` 在构造期同样拒绝——库调用方直接构造 `TrinoConnection` 也无法绕过（D-08，E2） | VERIFIED | config.rs与TrinoEngine::new双层拒绝；U:2172 basic_over_http_is_rejected、U:2177 library_rejects_credentials_over_http |
| 02-04.4 | `ontoforge-server.toml` 解析：`[trino]` 连接参数、`[trino.identities.ontology]` 与可选 `[trino.identities.l2]`、`[profiles.<name>] engine = \"trino\" \| \"sqlite\"`（`sqlite` 即 fixture 驱动的内存替身；profile 键经 02-01 的 `ProfileName` 校验，E14；不提供指向现成库文件的目录项，E1）；未知键报错；出现明文 `password` / `token` 键时报错并提示 `*_env` / `*_file`（D-06、D-07） | VERIFIED | config.rs serde deny_unknown_fields / ProfileName；U:2184 parses_full_example、U:2185 plaintext_password_is_rejected、U:2205 unknown_key_is_rejected、U:2187 profile_engine_has_no_library_path |
| 02-04.5 | 密钥只能以 `*_env`（环境变量名）或 `*_file`（文件路径）引用；`*_file` 经 `ontoforge_schema::loader::read_limited(path, true, 64 KiB)` 读取（允许符号链接、上限 64 KiB），去掉末尾换行；读取失败的错误只含路径或变量名，不含内容（D-07、Pitfall 11，E10） | VERIFIED | config.rs SecretRef→read_limited；U:2192 secret_from_env、U:2193 secret_from_file_follows_symlink_and_trims、U:2194 secret_never_in_debug_or_errors |
| 02-04.6 | `TrinoConfig` 的 `query_timeout_ms` / `max_rows` / `max_bytes` 缺省取 02-02 的 `DEFAULT_*` 常量；`TrinoConfig::query_context(trace_token, client_tags)` 用配置值构造 QueryContext，不在别处硬编码（E11） | VERIFIED | TrinoConfig::query_context复用DEFAULT_*；U:2189 query_context_from_config |
| 02-04.7 | TLS 可配置自定义 CA（`ca_file`，与平台根证书合并）；不存在任何跳过证书校验的配置项或代码路径，clippy `disallowed-methods` 禁止相关 reqwest 方法（D-09） | VERIFIED | config.rs CA读取并加入root，clippy.toml禁止danger_accept_invalid；U:2174 ca_file_is_loaded；最终TLS dependency guard与Clippy成功 |
| 02-04.8 | 两个服务身份各自构建独立的 `reqwest::Client`（独立连接池与凭据），本 phase 只解析与实例化 l2 身份，不启用（D-10） | VERIFIED | build_trino_engines独立client；U:2275 two_identities_use_separate_clients；本phase不启用L2，下游Phase6/7另验 |
| 02-04.9 | `nextUri` 必须与配置的 `url`（或 `next_uri_origins` 中显式列出的 origin）同 scheme / host / port，否则返回 `EngineError::Protocol` 且不向该地址发送任何请求；`next_uri_origins` 的 scheme 必须与 `url` 相同（配置期与 `TrinoEngine::new` 都拒绝 https → http 的降级 origin，E2）；客户端不跟随 HTTP 重定向 | VERIFIED | checked_next_uri+redirect::Policy::none；U:2265 foreign_next_uri_is_rejected、U:2267 next_uri_scheme_mismatch_is_rejected、U:2178 library_rejects_cross_scheme_origin |
| 02-04.10 | `describe_tables` 对同一 catalog 的多张表只发一条 printer 生成的 `information_schema.columns` 查询（WHERE 为各表 `(table_schema, table_name)` 条件的 Or / And 组合），按请求的 `TableRef` 归组返回；catalog / schema 不存在时对应表不在结果中；结果被 `max_rows` / `max_bytes` 截断时返回错误而不是不完整的列集（E22）；`status()` 用 `SELECT 1` 探测，失败为 `EngineStatus::Unavailable`（ENGINE-02 的 status 部分） | VERIFIED | trino/mod.rs describe/status经typed printer；U:2262 batching、U:2259 missing、U:2264 truncated_metadata_is_error、U:2273 status_available_and_unavailable |
| 02-04.11 | 值按列的 `type` 解码为 `Value`；不支持的类型或与类型不符的 JSON 值降级为 `Value::Unsupported(原 JSON)`，不让整条查询失败 | VERIFIED | trino/decode.rs；U:2258 decode_types、U:2257 decode_special_doubles_and_zone_forms；I:1049 decode_golden |
| 02-05.1 | `ontoforge validate --online examples/ontologies/aviation`（dev profile、无 `--server-config`）用内存 SQLite 替身加载全部 fixture，比对每个绑定的物理表，无不匹配时退出码 0（BIND-03、ENGINE-04、ADR-0021 引擎选择规则） | VERIFIED | online.rs select SQLite→fixture→compare_all；cli_online实际测试通过；U:1469 sqlite_profile_uses_fixtures |
| 02-05.2 | 绑定的显式列在物理表中不存在时报 ONT-V17 error，定位到绑定文件中该 `column` 值的 line:col；同名默认映射的列不存在时定位到 `source.table`；表不存在时定位到 `source.table`（D-14），human 与 JSON 两种格式都可输出 | VERIFIED | online.rs diagnostic span映射；cli_online human/JSON快照；U:1474 trino_profile_missing_column |
| 02-05.3 | 属性类型与源列类型分属不同类型族（如 integer 属性对 varchar 列）时报 ONT-V17 error；族内的加宽 / 精度 / 时区规则在 02-08 | VERIFIED | bind_check compare_binding按族；U:1475 trino_type_mismatch |
| 02-05.4 | 选定 profile 有不带 `fixture` 的绑定且未给 `--server-config` 时，`--online` 报 ONT-V20 并提示 `--server-config`，不静默跳过该绑定（ADR-0021 open question 2） | VERIFIED | online.rs显式MissingFixture→V20；cli_online无fixture负向测试通过 |
| 02-05.5 | 替身把 fixture 加载进 `ATTACH` 的 `catalog__schema` 库（D-11），列的物理类型由绑定 + IR 推出；加载完成后执行 `PRAGMA query_only = ON`，之后任何写入失败。因此 fixture 模式的 `--online` 只验证列存在与值可按类型转换，不证明真实源列的类型兼容（文档在 02-08，E15） | VERIFIED | sqlite/fixture ATTACH→query_only；U sqlite::tests::query_only_after_load；docs/source-binding.md明确IR推导边界 |
| 02-05.6 | 单个绑定或单个 catalog 的非整轮引擎错误（`is_engine_wide()` 为假：Query、Timeout、Protocol 等）只让对应绑定得到一条定位到其 `source` 值的 ONT-V19，其余绑定照常比对并报告；`is_engine_wide()` 为真的错误（不可用、认证失败等）中止整轮；出现任一 ONT-V19 时退出码为 3（BIND-03「其余照常」，E3） | VERIFIED | bind_check_isolation按catalog QueryError/engine_wide区分；U:1466 one_catalog_query_error_is_v19_others_checked、U:1476/1477 |
| 02-05.7 | `describe_bindings` 按 catalog 分组，每个 catalog 只调用一次 `Engine::describe_tables`（E22） | VERIFIED | bind_check::describe_bindings按catalog分组；U:2262 describe_tables_batches_one_catalog和bind_check_isolation batching测试 |
| 02-05.8 | 多个绑定指向同一物理表（按小写化的 catalog / schema / table 判定）时替身只建一张表：它们必须引用同一个 fixture 文件，且共享列名的逻辑类型一致，否则 ONT-V16；列为各绑定期望列的并集（E18） | VERIFIED | sqlite/fixture表映射合并；U:2243 shared_table_loaded_once、U:2242 shared_table_conflicts_are_errors |
| 02-05.9 | 期望列由 02-03 的 `Ontology::bound_columns` 推导、decimal 精度取 `PropertyDef::effective_decimal()`、fixture 表头取 `loader::fixture_header`、fixture 文件经 `read_limited(path, false, MAX_FILE_BYTES)` 读取、decimal 文本经 `value::split_decimal` 切分；本 plan 不再自写这些逻辑（E7–E10） | VERIFIED | sqlite/fixture依次调用bound_columns/effective_decimal/fixture_header/read_limited/split_decimal，前owner调用链审计与U边界测试 |
| 02-05.10 | fixture 路径的 QueryContext 由 `QueryContext::new` 的缺省构造，`--timeout <MS>`（只能与 `--online` 同用）覆盖超时（E11） | VERIFIED | QueryContext::new→online timeout覆盖；U:2240 query_context_defaults、U:1471 timeout_flag_is_sent |
| 02-05.11 | CSV 空单元格对所有类型都是 NULL，JSON fixture 用真 `null`；CSV 中 array 单元格为 JSON 数组文本（ADR-0021 open question 8） | VERIFIED | sqlite/fixture typed cell conversion；U:2222 csv_empty_cell_is_null_for_every_type、U:2236 json_fixture_loads、U:2219 arrays |
| 02-05.12 | decimal fixture 值以规范化字符串保存、从不经过浮点：小数位多于 scale 或整数位多于 precision - scale 的值被拒绝（ONT-V16）而不是舍入；`1.5` 在 decimal(12,2) 下规范化为 `1.50`；JSON fixture 中的 decimal 必须写成字符串 | VERIFIED | sqlite/fixture decimal字符串规范化无f64；U:2223 decimal_boundaries_12_2、相关value::split_decimal_rules |
| 02-05.13 | aviation fixture 每个 type 有几十行虚构数据，覆盖 NULL、decimal(12,2) 边界（`9999999999.99`、`-9999999999.99`、`0.01`）、Europe/Berlin 夏令时切换前后的本地时间与带偏移的 aware 时间，不含重复主键；两个示例的 fixture 逐字节相同（D-12） | UNCERTAIN (WARNING; judgment) | examples/aviation与aviation-json真实虚构fixture；loader/engine样例与timestamps/decimal测试，I:1086真实在线验证 |
| 02-06.1 | Trino 不可达（连接被拒、DNS 失败、TLS 握手失败、重试耗尽后的 502 / 503 / 504、`SERVER_STARTING_UP` / `SERVER_SHUTTING_DOWN`）一律返回 `EngineError::Unavailable { retryable: true, .. }`，错误码 `ENGINE_UNAVAILABLE`；401 / 403 返回 `Unauthorized`；查询自身的错误返回 `Query { error_name }`（ENGINE-02 fail-closed 的引擎侧） | VERIFIED | trino/classify.rs与client.rs；U:2315 closed_port_is_unavailable、U:2317 dns_failure_is_unavailable、U:2337/2336 starting/shutdown、U:2341 unauthorized、U:2339 query |
| 02-06.2 | `status()` 在上述不可用情形下返回 `EngineStatus::Unavailable { reason }`，reason 不含凭据 | VERIFIED | TrinoEngine::status；U:2338 status_reports_unavailable；错误消毒不输出SecretString |
| 02-06.3 | GET nextUri 遇 502 / 503 / 504 按 50 ms 起、翻倍、上限 1 s 的退避重试至多 5 次；429 按 `Retry-After`（秒，上限 5 s，缺省 1 s）重试；POST 只在三种能证明语句未被接受的情形重试：429、503 且响应体 `error.errorName` 为 `SERVER_STARTING_UP`、连接未建立；普通 503（含网关返回的 503）、502、504 一律不重试并返回错误，避免把已提交的昂贵查询再提交一次（E16，RESEARCH A13、spike 1 第 100 行）；全部重试受 QueryContext 超时约束 | VERIFIED | client.rs bounded retry；U:2322 get_502_and_504_are_retried、U:2332 post_plain_503_is_not_retried、U:2331 post_503_starting_up_is_retried、U:2333 retries_respect_timeout |
| 02-06.4 | 超过 QueryContext 超时：向当前 `nextUri` 发送 `DELETE` 后返回 `EngineError::Timeout`（ENGINE-01 超时取消） | VERIFIED | execute_unchecked timeout→QueryGuard::cancel_now；U:2340 timeout_sends_delete、U:1470 timed_out_query_is_cancelled_before_exit；I:1050真实服务端取消 |
| 02-06.5 | 调用方丢弃正在执行的查询 future（客户端断连）时，guard 在 Drop 中向当前 `nextUri` 发送 `DELETE`（ENGINE-01 断连取消） | VERIFIED | QueryGuard::drop→CancellationTasks；U:2318 dropped_future_sends_delete、U:2313 cancellation_drain_survives_cancel_now_waiter_drop |
| 02-06.6 | 累计行数超过 `max_rows` 或累计数据字节超过 `max_bytes` 时停止拉取、`DELETE` 当前 `nextUri`，返回已取得的结果并置 `truncated = true`（ADR-0021 整批返回）；单个响应页超过 `max_page_bytes` 时按块读取即中止并 `DELETE`，返回 `Protocol` 错误 | VERIFIED | run页/行/字节limit→cancel；U:2328 max_rows_truncates_and_deletes、U:2327 max_bytes_truncates_and_deletes、U:2329 oversized_page_is_protocol_error |
| 02-06.7 | `DELETE` 只发往通过同源校验的 `nextUri`，带与查询相同的认证头 | VERIFIED | nextUri先校验后保存到guard、同headers构造；U:2265 foreign_next_uri_is_rejected、U:2267 scheme mismatch、U:2340 timeout DELETE |
| 02-07.1 | ONT-E13：decimal 属性（含元素为 decimal 的 array）的生效精度不在 1..=38、或生效 scale 大于生效 precision 时报 error；生效值取 02-03 的 `PropertyDef::effective_decimal()`（缺省 38 / 10 的唯一定义，E7），只写 `precision: 5` 时因缺省 scale 10 > 5 而报错并提示显式写 scale | VERIFIED | lint/binding/e13→effective_decimal；e13::tests触发/clean snapshots真实通过 |
| 02-07.2 | ONT-E14：source-backed 对象的每个 timestamp 属性，在每个 profile 的绑定 `columns` 条目中都必须声明 `timezone`（IANA 名或 `aware`），缺失报 error 并定位到该绑定文件；在非 timestamp 属性上声明 `timezone` 也报 error（ADR-0021 open question 1 的离线规则） | VERIFIED | e14→binding_def(SourceBinding)定位；e14::tests触发/clean snapshots真实通过 |
| 02-07.3 | ONT-E15：`collation` 只能用于 string（或元素为 string 的 array）属性及其对应的 link 列，否则 error（BIND-02 的可选 collation 契约） | VERIFIED | e15 string/array-string collation；e15::tests触发/clean snapshots真实通过 |
| 02-07.4 | ONT-E16：`backing: source` 的 object type 含 geopoint 属性（或元素为 geopoint 的 array）时报 error（ADR-0015：geopoint 不支持 source 绑定） | VERIFIED | e16 source geopoint/array规则；e16::tests触发/clean snapshots真实通过 |
| 02-07.5 | ONT-W17：绑定带 fixture 时，期望的物理列（由 02-03 的 `Ontology::bound_columns` 推导，E9）不在 fixture 表头中（表头由 02-03 的 `loader::fixture_header` 解析，E8；大小写不敏感）报 warning，这是 D-03「属性在绑定表中找不到对应列」的离线可判定部分；在线不存在的列由 `validate --online` 报 ONT-V17 | VERIFIED | w17→bound_columns+fixture_header；w17::tests触发/clean与case-insensitive测试通过 |
| 02-07.6 | fixture 表头无法读取或解析（编码错误、JSON 不是对象数组、超过 4 MiB、超出总字节预算）时，ONT-W17 给出一条定位到绑定 `fixture.file` 值的 warning 说明无法离线检查及原因，不静默跳过（E20） | VERIFIED | w17 unreadable产生warning；U:2822 fixture_that_is_not_utf8_is_recorded_as_unreadable、w17 unreadable snapshots |
| 02-07.7 | 加载期为 W17 读取的 fixture 字节与定义文件一起计入 `MAX_TOTAL_BYTES` 总预算；同一 fixture 文件被多个绑定引用时只读一次（E23） | VERIFIED | loader fixture bytes共同预算+dedup；U:2820 fixture_bytes_count_toward_budget、U:2826 shared_fixture_is_counted_once |
| 02-07.8 | 新规则可在 `ontoforge.toml` 中按 ID 关闭或调级；`KNOWN_RULE_IDS` 含 ONT-E13..E16 与 ONT-W17，编号避开 foundry spec 已占用的 E10-E12 / W12-W16 | VERIFIED | lint registry与config ID注册；U:2792 registry_ids_are_known_and_unique、U:2790 level_resolution |
| 02-07.9 | aviation 与 aviation-json 在默认配置下 lint 结果为空；每条新规则有触发与不触发两组 insta 快照 | VERIFIED | U:2955 aviation_lints_clean；新规则各trigger/clean snapshots；两个示例同步 |
| 02-08.1 | drift 校验以库函数交付：`check_drift` 描述选定 profile 的全部绑定并比对，有 error 级不匹配的 object type / link 为 `Degraded { reason: SourceSchemaDrift }`，只有 warning 的为 `Healthy { warnings }` 并保留这些精度 / 时区 / char 提示（E12），完全匹配的为 `Healthy { warnings: [] }`；CLI（`validate --online`）与 drift 共用同一个 `compare_binding`（D-15） | VERIFIED | check_drift→compare_binding状态；U:2040 aviation_is_healthy、U:2043 missing_column_degrades_only_flight、U:2047 warnings_do_not_degrade；I:1048 |
| 02-08.2 | `check_drift` 只在 `is_engine_wide()` 的引擎错误（不可用、认证失败等）时返回 `EngineError`，不产生「全部降级」或「全部健康」的报告（引擎健康与 schema drift 是两件事）；单个目标的元数据查询失败只让该目标为 `Degraded { reason: SourceUnreadable }`，其余目标照常判定（BIND-03「其余照常」，E3） | VERIFIED | check_drift engine_wide vs目标错误；U:2042 engine_error_is_not_a_report、U:2045 query_error_degrades_only_that_target |
| 02-08.3 | 类型兼容只接受无损映射（D-13）：整数只可加宽（integer 属性对 bigint 列为 error）；float 只接受 real，double 接受 real / double；string 接受 varchar / char，char 给 warning；date 只接受 date；array 按元素递归；geopoint 与未知类型为 error 并提示在 Trino 建 view 做 cast；跨类型族一律 error（与 02-05 一致） | VERIFIED | bind_check/compat_tests.rs独立表驱动expected；integer/float/string/array跨族及lossless tests通过 |
| 02-08.4 | decimal 兼容：声明 decimal(P,S) 对源列 decimal(p,s) 当且仅当 `s ≤ S` 且 `p - s ≤ P - S`；兼容且 `p < P` 或 `s < S` 时 warning（精度变化），否则 error；`number` 与无界 numeric 为 error（ADR-0021）；P / S 取 `ExpectedColumn` 中由 `PropertyDef::effective_decimal()` 得到的值，本 plan 不写缺省数字（E7） | VERIFIED | bind_check decimal P/S无损条件；compat_tests表驱动decimal与effective_decimal tests通过 |
| 02-08.5 | timezone 在线复核：声明 IANA 而源列带时区为 warning；声明 `aware` 而源列无时区为 error；源列无时区且未声明为 error | VERIFIED | bind_check timestamp aware/IANA比较；compat_tests timezone cases通过 |
| 02-08.6 | 绑定列小写化后匹配到多个物理列（如 `Status` 与 `status`）时为 error（CaseCollision，列选择不确定，E13）；与绑定列无关的大小写冲突不报告；manyToMany 绑定的 from / to 列按两端主键类型比对 | VERIFIED | bind_check lowercase collision与M:N主键；bindings/compat测试实际通过 |
| 02-08.7 | `docs/source-binding.md` 写明绑定格式、profile、fixture 语义、fixture 模式下 `--online` 的验证边界（物理类型由 IR 推导，只验证列存在与值转换，不证明真实源列类型兼容，E15）、类型映射表、诊断码与退出码、服务配置（不含库目录项，E1）与 drift 模型；文档中的类型映射表由测试逐行对照 `compare_binding` 的结果，文档与代码不能矛盾 | VERIFIED | docs/source-binding.md类型表→tests/doc_type_table逐行调用compare_binding，命名test通过 |
| 02-09.1 | `ontoforge validate --online --server-config <file>` 按服务配置把选定 profile 映射到引擎：`trino` 用 ontology 服务身份的 TrinoEngine 校验绑定，`sqlite` 用 fixture 驱动的内存替身（与不给配置时相同）；不提供指向现成 SQLite 库文件的目录模式（E1）（D-06、ADR-0021 open question 2） | VERIFIED | online.rs config→build_trino_engines / SQLite fallback只在显式sqlite；U:1473 trino_profile_clean、U:1469 sqlite_profile_uses_fixtures |
| 02-09.2 | 配置把 profile 映射到 Trino 而绑定带 `fixture` 时，对每个这样的绑定给 ONT-V17 warning（fixture 被忽略），绝不改用 SQLite 替身 | PASSED (override: ADR-0022) | 后继接受ADR-0022将warning改为Info；online.rs Severity::Info；U:1465 ignored_fixture_info_passes_deny_warnings，按后继合同验证 |
| 02-09.3 | 引擎侧原因导致在线校验无法完成（Trino 不可达、认证失败、超时、协议或查询错误）时输出 ONT-V19 并以退出码 3 结束，不输出成功摘要；绑定不匹配仍是退出码 1（D-14）。不可达与认证失败中止整轮；单个 catalog 的查询错误只让该 catalog 的绑定得到 ONT-V19，其他绑定的 V17 照常输出，退出码仍为 3（E3） | VERIFIED | online.rs status precedence；U:1476 trino_unavailable_exits_3、U:1477 unauthorized_exits_3、U:1466 per-catalog error |
| 02-09.4 | `ontoforge-server.toml` 的错误（未知键、明文密钥、http + 密码、不合法的 profile 名、缺 profile 映射）输出为 ONT-V20，定位到配置文件的 line:col，退出码 1 | VERIFIED | config_diagnostic→V20/span；U:1464 config_errors_are_v20_with_location |
| 02-09.5 | 密钥值（经 `*_env` / `*_file` 注入）在任何情况下都不出现在 stdout / stderr（D-07） | VERIFIED | SecretString与诊断消毒；U:1468 secret_never_printed、U:1467 rejected_plaintext_secret_never_printed |
| 02-09.6 | 发往 Trino 的每个请求带 `X-Trino-Trace-Token: ontoforge-cli-<ULID>` 与 `X-Trino-Client-Tags: ontoforge,validate-online`（ENGINE-01 的 trace token）；Trino 路径的 QueryContext 由 `TrinoConfig::query_context` 构造，`--timeout` 覆盖超时（E11） | VERIFIED | online.rs QueryContext trace ULID/tags→headers；U:2269 request_headers、U:1471 timeout_flag_is_sent |
| 02-09.7 | `--check-pk`（只能与 `--online` 同用，否则 clap 用法错误退出码 2）对每个 object 绑定执行两条查询：重复值查询 `WHERE pk IS NOT NULL GROUP BY pk HAVING count(*) > 1 LIMIT 21`（排除 NULL，E17）与单独的 `pk IS NULL` 计数；重复或 NULL 主键报 ONT-V18 error（最多列出 20 个样例值），两行以上 NULL 主键只体现为 NULL 计数、不会出现一个值为 NULL 的伪重复；不带 `--check-pk` 时绝不执行这类全表查询（BIND-03、ADR-0015） | VERIFIED | check_keys仅flag时调用pk_check typed查询；U:1342 check_pk_without_flag_runs_no_scan、U:2136 nulls_are_not_duplicates、U:1463 check_pk_requires_online；I:1085 |
| 02-09.8 | CLI 的类型不兼容与表缺失测试经 wiremock Trino（information_schema 返回指定的列类型或 0 行）覆盖，重复 / NULL 主键经 fixture 替身与 wiremock 覆盖（E1：不再需要预先建好的 SQLite 库文件） | VERIFIED | wiremock metadata错误与fixture pk；U:1474/1475/1472、U:1339/1340 |
| 02-10.1 | PR 的 CI 新增 Linux `integration` 作业：PostgreSQL service container（显式 `ports: [\"5432:5432\"]` 把端口发布到 runner，host 网络的 Trino 才能连 `localhost:5432`，E25）+ checkout 后 `docker run` 的 Trino 483（catalog 目录从仓库挂载），种入 aviation 演示数据后运行 `--ignored` 集成测试（ENGINE-05、D9） | VERIFIED | ci.yml→ci/query-services.sh显式runner端口+seed+Trino483 digest；I integration实际2+5+engine命名tests；host端口合同成立 |
| 02-10.2 | Trino 配置两个 PostgreSQL catalog：source（`demo`）与 edit store（`editstore`），另有 `tpch` 用于超时取消测试；集成测试断言 `demo` 与 `editstore` 都可见 | VERIFIED | ci/trino/catalog demo/editstore/tpch；I:1043 catalogs_visible |
| 02-10.3 | 集成测试在真实 Trino 上验证：一次查询、`describe_tables` 的类型 golden（同一 catalog 一次批量描述，E22）、值解码 golden、printer 固定语料（引号、反斜杠、`--`、`/*`、`;`、`U&` 文本、换行、emoji、超长串）的往返、超时后服务端查询被取消（ENGINE-01、ENGINE-03、D23） | VERIFIED | trino_it真实值/类型/字符串/取消断言；I:1046/1049/1051/1050 |
| 02-10.4 | `ontoforge validate --online --check-pk --server-config ci/ontoforge-server.ci.toml examples/ontologies/aviation` 在 CI 的 Trino 上退出码 0（只有 fixture 被忽略的 warning） | PASSED (override: ADR-0022) | 后继ADR-0022 ignored fixture改Info；I:1086 validate_online_against_ci_trino、I:1085 check_pk duplicate/null实际通过 |
| 02-10.5 | 集成测试在缺少 `ONTOFORGE_TRINO_URL` 时 panic 而不是静默通过；CI 另有一步断言 `--ignored` 实际执行的测试数大于 0（RESEARCH Pitfall 8） | VERIFIED | trino_it::trino缺URL panic，ci discovery每target非零；当前integration ignored-tests gate success；不把Iceberg提前return计通过 |
| 02-10.6 | nightly 工作流在 Trino 483 与 484 上额外跑 Iceberg（JDBC catalog + MinIO），失败时由独立的 `report-failure` job 自动开 issue（同一标签下已有未关闭 issue 则追加评论）；`issues: write` 只授予这个 job（GitHub Actions 的 permissions 只能设在 workflow 或 job 层），测试 job 只有 `contents: read`（E25） | FAILED (BLOCKER) | Iceberg JDBC+local warehouse当前真实483/484各CTAS29行+test1；MinIO字面偏差另行记录；nightly report-failure权限只job范围 |
| 02-10.7 | Phase 2 完成门禁：推送后 PR 的 `integration` 作业与首次 nightly 运行（483、484 两个矩阵）都实际跑通，两个集成测试目标的 `--ignored` 实际执行数各 ≥ 1；`describe_types_golden` 与 `decode_golden` 的预期值按首跑日志定稿并提交后，才允许写 `02-VERIFICATION.md` 的 passed（E21） | VERIFIED | 历史首跑golden定稿与当前I:1046/1049继续通过；PROVISIONAL已删除；483/484当前Iceberg精确测试通过，原历史nightly记录保留 |
| 02-10.8 | SQL Server 等公司环境只经 `ONTOFORGE_PRIVATE_FIXTURES` 指向仓库外目录的私有测试验证，CI 不运行它，仓库中不出现任何私有绑定或数据（ENGINE-05） | UNCERTAIN (WARNING; judgment) | private_fixtures入口受ignore/env门控且公开workflow不调用；公司数据来源为judgment，non-authoritative，未声称私有SQL Server已实跑 |
| 02-10.9 | Windows 本机 `cargo test --workspace` 仍不需要 Docker 即全绿（集成测试全部 `#[ignore]`） | VERIFIED | 当前W workspace / Clippy / fmt成功；普通tests不启动外部服务，真实backend仅ignored独立job执行 |

**Score:** 100/103；behavior_unverified=0；overrides_applied=2。两个judgment禁止项保留醒目标记，不能被自动意见一致静默吸收为passed。

### Required Artifacts / Key Links

前owner对10份PLAN实际执行owner queries：53 artifacts中51自动匹配，31 links中29自动匹配。本次读取现有代码与最终命名日志复核；静态误匹配按下表人工定位，真正缺失的MinIO合同单列BLOCKER。

| Artifact / link | Actual wiring | Status |
|---|---|---|
| trino/client.rs缺v1/statement pattern | trino/mod.rs:268 statement_url→base.join("v1/statement")→client::send | WIRED |
| AnyEngine::Trino pattern | engine/lib.rs Self::Trino(e)逐方法分发；CLI实际构造AnyEngine::Trino | WIRED |
| e14缺DefKind::SourceBinding pattern | e14.rs:23→binding/mod.rs:33..36 binding_def→该DefKind | WIRED |
| ci.yml缺trinodb/trino:483 pattern | workflow→ci/query-services.sh→固定483digest | WIRED |
| schema binding/loader/validate/lint | 真实定义→profile binding→类型/完整性→diagnostic/source span | VERIFIED |
| engine printer/sqlite/trino/config/bind_check/pk_check | CLI与库真实调用，实际SQLite/wiremock/Trino日志逐truth列明 | VERIFIED |
| docs/source-binding.md / ADR0021/0022 | 类型表测试对照实现，Info后继决策有用户历史接受记录 | VERIFIED |
| private_fixtures.rs | 仓库外env入口与ignore门控；公开workflow不调用 | WIRED；公司环境未实跑 |
| MinIO/S3 warehouse | current local路径没有S3配置与MinIO服务 | MISSING（合同特定路径，BLOCKER） |

### Data-Flow Trace (Level 4)

| Value | Source / chain | Status |
|---|---|---|
| online绑定列与类型诊断 | Engine::describe_tables → SQLite fixture元数据 / Trino information_schema → compare_binding → span诊断 | FLOWING |
| PK重复与NULL诊断 | 仅显式flag → typed SELECT GROUP BY/NULL count → QueryOutput →最多20样本诊断 | FLOWING |
| drift健康状态 | catalog metadata→compare_binding→Healthy warnings /目标Degraded；engine_wide错误独立传播 | FLOWING |
| Iceberg行数 | 真实Trino SELECT→JDBC catalog+local文件→Value::Long，与PG source count比较 | FLOWING（local，非MinIO） |

无静态空数组代替真实查询路径；fixture是声明的SQLite开发替身，不冒称真实源列类型证明。

### Behavioral Spot-Checks

| Behavior | Executed evidence inspected | Result |
|---|---|---|
| schema/binding/CLI/printer/SQLite | 最终W/U workspace精确命名日志；完整逐task矩阵见02-VALIDATION | PASS |
| Trino metadata/值/SQL/取消 | I:1043-1053 9 passed，其中普通Iceberg提前return不作Iceberg证据；I:1085-1088 CLI2 passed | PASS（明确排除空验证） |
| Trino483 Iceberg local | 精确命令见下；CTAS29 rows，1 passed/0 failed/0 ignored，0.66s | PASS |
| Trino484 Iceberg local | CTAS29 rows，1 passed/0 failed/0 ignored，0.63s | PASS |
| MinIO/S3 | 当前fixture无该路径，本轮未执行 | FAILED（实现/合同缺口） |

精确执行命令：
`cargo test -p ontoforge-engine --test trino_it --locked iceberg_table_is_readable -- --ignored --exact --nocapture`。
本 verifier亲读 `trino_it.rs:626`：缺ICEBERG变量会提前return；专用runner:53实际导出 `ONTOFORGE_TRINO_URL` 与 `ONTOFORGE_TRINO_ICEBERG=1`，fixture日志与CTAS日志一致。测试断言columns非空、source不为0、copied/source完全相等；未仅凭test result通过。

| Evidence | SHA256 |
|---|---|
| trino-483-ctas.log / trino-484-ctas.log（内容相同） | a87f652b6c27483a8fc19d696c0992f16a5796b6444f57601b801cb6d37b201a |
| trino-483-fixture.log | 221fdf6dee83e59fd6edc27794ebb7b76c122d83f459f4bc557e656e2b4dd961 |
| trino-484-fixture.log | c1676f4ae720350aa4e09f6224d925df8483a2944002f4755d2fd7f723ec7b64 |
| trino-483-iceberg.log | d250dc030e611fcd937d2bfc811dafc43487c6d1e0a51e14646ff4c16a72bb24 |
| trino-484-iceberg.log | 1c589143bf0eb5db09d9f8eb6fc4390405ad6bedd40cd02d01b53705399ef24e |

固定Trino483 digest `db58cc93e593a2706553745f276bb119c9810e69918be56ecde088ba7ccb0534`；484 digest `2704eb53935af5e582d63d6033e9bf99d20c325e9b0e30b1016ef59a4773979f`。两个info.json对应版本且starting=false。

### Probe Execution

未声明scripts/*/tests/probe-*.sh；项目真实集成目标与Iceberg补证按上述实际命令记录。未把本地runner称作远程nightly。

### Requirements Coverage

| Requirement | Source Plans | Status | Evidence / boundary |
|---|---|---|---|
| BIND-01 | 02-01/03 | SATISFIED（功能） | backing/profile/binding tests；公共数据来源judgment单列 |
| BIND-02 | 02-03/07/08 | SATISFIED | 类型契约lint/compat/document表 |
| BIND-03 | 02-05/08/09 | SATISFIED（Phase02） | CLI/PK/drift库；启动周期接线属Phase7 |
| ENGINE-01 | 02-04/06/09/10 | SATISFIED | HTTP分页/认证/timeout/drop/headers真实测试 |
| ENGINE-02 | 02-04/06/09 | SATISFIED（Phase02） | fail-closed/status；health/describe属Phase7 |
| ENGINE-03 | 02-02/10 | SATISFIED | typed printer/proptest/真实往返 |
| ENGINE-04 | 02-02/05 | SATISFIED（Phase02） | Windows SQLite；serve-dev属Phase7 |
| ENGINE-05 | 02-10 | BLOCKED / PARTIAL | Trino+PG与local Iceberg已证实，ADR0017的MinIO合同仍缺；私有SQL Server无本轮执行记录 |

8个ID均在PLAN与需求traceability对应，无orphan。不把STORE-05的source cardinality缺口归到02；该项由Phase07.1独立验收。

### Decision Coverage

All trackable CONTEXT.md decisions are honored by shipped artifacts.
本次GSD owner query：15/15、not_honored=[]。这是模糊匹配的warning-only gate，不足以覆盖本报告已发现的ADR0017/MinIO偏差。

### Prohibitions

| Plans / tier | Enforcement and evidence | Verdict |
|---|---|---|
| 02-01 / test：禁止默认backing | required字段+V05，U:3028/2939 | VERIFIED |
| 02-04 / test：禁止凭据异源发送 | 构造期HTTPS、checked_next_uri、禁redirect，U:2177/2178/2265/2267 | VERIFIED |
| 02-04 / test：禁止泄密 | SecretString+错误/CLI消毒，U:2194/1467/1468 | VERIFIED |
| 02-04 / test：禁止TLS绕过 | 无配置字段+Clippy方法禁令，最终CI guard/Clippy成功 | VERIFIED |
| 02-06 / test：禁止隐藏Trino故障 | typed Unavailable、无fallback，U:2315/2338/1476 | VERIFIED |
| 02-09 / test：禁止在线假成功 | engine分派与exit3，U:1466/1476/1477 | VERIFIED |
| 02-09 / test：禁止隐式全表PK扫描 | flag guard，U:1342；drift只有metadata | VERIFIED |
| 02-03 / judgment：公开数据来源 | 虚构航空fixture/中性物理名；未人工证明来源 | UNCERTAIN；NON-AUTHORITATIVE；unverified-prohibition — human review recommended |
| 02-10 / judgment：公开CI不接公司源 | 隔离localhost/digest服务，private测试未被workflow调用 | UNCERTAIN；NON-AUTHORITATIVE；unverified-prohibition — human review recommended |

用户已授权意见同向自动推进；`target/milestone-tls-and-judgment-review.json`的AGREE是流程授权与源码判断，不是来源人工验收。本报告不再索取重复操作许可，也不伪造human pass。两个旗标按verifier高优先级规则保留，MinIO关闭后仍不得静默标passed。

### Test Quality / Anti-Patterns

前owner已检查真实断言、disabled测试与数据链；本轮专门反证普通Iceberg早退、旧p95 smoke不足和后继取消状态变化。实际取消/drift budget分别有 `cancellation_drain_*`（U:2311-2314）与drift_budget命名测试；没有用符号存在替代行为。

当前生产engine/query、storage/query_read与online扫描无未跟踪TBD/FIXME/XXX。fixture里的“Sam Placeholder”是声明虚构人物，不是业务stub。真实后端ignored目标由integration/专用runner实际执行；私有fixture仍不算真实公司环境已测。

### Advisory (New Scope, Unevidenced)

None。MinIO是既有PLAN/ADR合同的可观察缺失，不是新增无证据架构偏好。两个judgment旗标不伪称产品失败。

### Human Verification Required

1. **公开fixture来源**：确认示例、测试、种子数据确为可公开虚构数据，无公司物理名/业务数据。源码扫描无法证明全部来源。
2. **公开CI边界**：确认部署环境与私有fixture保持隔离，无公司源被公开workflow触达。当前workflow与隔离运行支持non-authoritative判断。

历史02-10的延迟human-check已由CI/Iceberg实际证据和旧UAT覆盖其原测试范围，未重复制造运行许可。MinIO合同选择由主会话单独处理，本 verifier不代替用户决策。

### Gaps Summary

**1个BLOCKER**：accepted ADR0017与02-10.6仍要求MinIO，而当前只有local warehouse。独立盲评 `target/iceberg-fixture-independent-opinion.json`一致识别“规范偏差+相应覆盖缺口”，不能从local成功推导MinIO成功，也没有证明产品MinIO运行失败。

最小闭环：按原合同补受信MinIO的483/484真实验收，或需求所有者明确接受后新增正式修订/覆盖记录。后续ROADMAP无明确覆盖该项，不能自动defer。另保留2个non-authoritative judgment旗标。02-VALIDATION现为validated、26 COVERED/1 PARTIAL/1文档SKIP、nyquist_compliant=false。

_Verifier: gsd-verifier_
