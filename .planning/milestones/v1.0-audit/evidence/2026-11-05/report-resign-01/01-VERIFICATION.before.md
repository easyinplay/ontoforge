---
phase: 01-workspace-schema-lint
verified: 2026-11-04T02:55:02Z
status: human_needed
score: 11/11 must-haves verified
covered_files: [".gitattributes",".github/workflows/ci.yml",".planning/adr/0002-naming-convention.md",".planning/adr/0003-ontology-definition-format.md",".planning/adr/0006-expression-language.md",".planning/adr/0011-workspace-layout.md",".planning/adr/0012-definition-document-shape.md",".planning/phases/01-workspace-schema-lint/01-01-PLAN.md",".planning/phases/01-workspace-schema-lint/01-01-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-02-PLAN.md",".planning/phases/01-workspace-schema-lint/01-02-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-03-PLAN.md",".planning/phases/01-workspace-schema-lint/01-03-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-04-PLAN.md",".planning/phases/01-workspace-schema-lint/01-04-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-05-PLAN.md",".planning/phases/01-workspace-schema-lint/01-05-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-06-PLAN.md",".planning/phases/01-workspace-schema-lint/01-06-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-07-PLAN.md",".planning/phases/01-workspace-schema-lint/01-07-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-08-PLAN.md",".planning/phases/01-workspace-schema-lint/01-08-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-09-PLAN.md",".planning/phases/01-workspace-schema-lint/01-09-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-10-PLAN.md",".planning/phases/01-workspace-schema-lint/01-10-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-11-PLAN.md",".planning/phases/01-workspace-schema-lint/01-11-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-12-PLAN.md",".planning/phases/01-workspace-schema-lint/01-12-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-13-PLAN.md",".planning/phases/01-workspace-schema-lint/01-13-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-14-PLAN.md",".planning/phases/01-workspace-schema-lint/01-14-SUMMARY.md",".planning/phases/01-workspace-schema-lint/01-CONTEXT.md",".planning/phases/01-workspace-schema-lint/01-UAT.md",".planning/phases/01-workspace-schema-lint/01-VALIDATION.md",".vscode/settings.json",".zed/settings.json","CONTRIBUTING.md","Cargo.lock","Cargo.toml","ci/check-tls-deps.sh","ci/test_tls_deps.py","crates/ontoforge-expr/src/analyze.rs","crates/ontoforge-expr/src/check.rs","crates/ontoforge-expr/src/infer.rs","crates/ontoforge-expr/src/lib.rs","crates/ontoforge-expr/src/limits.rs","crates/ontoforge-expr/src/rowpred/eval.rs","crates/ontoforge-expr/src/rowpred/ir.rs","crates/ontoforge-expr/src/rowpred/mod.rs","crates/ontoforge-expr/src/rowpred/parse.rs","crates/ontoforge-expr/src/rowpred/value.rs","crates/ontoforge-expr/src/walk.rs","crates/ontoforge-expr/tests/analyze.rs","crates/ontoforge-expr/tests/criteria.rs","crates/ontoforge-schema/Cargo.toml","crates/ontoforge-schema/src/diag.rs","crates/ontoforge-schema/src/ir/action.rs","crates/ontoforge-schema/src/ir/attrs.rs","crates/ontoforge-schema/src/ir/basetype.rs","crates/ontoforge-schema/src/ir/binding.rs","crates/ontoforge-schema/src/ir/link.rs","crates/ontoforge-schema/src/ir/manifest.rs","crates/ontoforge-schema/src/ir/mod.rs","crates/ontoforge-schema/src/ir/names.rs","crates/ontoforge-schema/src/ir/object.rs","crates/ontoforge-schema/src/ir/scope.rs","crates/ontoforge-schema/src/lib.rs","crates/ontoforge-schema/src/lint/binding/e13.rs","crates/ontoforge-schema/src/lint/binding/e14.rs","crates/ontoforge-schema/src/lint/binding/e15.rs","crates/ontoforge-schema/src/lint/binding/e16.rs","crates/ontoforge-schema/src/lint/binding/mod.rs","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e13__tests__clean.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e13__tests__triggers.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e14__tests__clean.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e14__tests__triggers.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e15__tests__clean.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e15__tests__triggers.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e16__tests__clean.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e16__tests__triggers.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__w17__tests__clean.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__w17__tests__triggers.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__w17__tests__unreadable.snap","crates/ontoforge-schema/src/lint/binding/w17.rs","crates/ontoforge-schema/src/lint/config.rs","crates/ontoforge-schema/src/lint/e08.rs","crates/ontoforge-schema/src/lint/e09.rs","crates/ontoforge-schema/src/lint/mod.rs","crates/ontoforge-schema/src/lint/naming/mod.rs","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w01__tests__clean.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w01__tests__triggers.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w02__tests__clean.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w02__tests__triggers.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w03__tests__clean.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w03__tests__triggers.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w04__tests__clean.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w04__tests__triggers.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w05__tests__clean.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w05__tests__triggers.snap","crates/ontoforge-schema/src/lint/naming/w01.rs","crates/ontoforge-schema/src/lint/naming/w02.rs","crates/ontoforge-schema/src/lint/naming/w03.rs","crates/ontoforge-schema/src/lint/naming/w04.rs","crates/ontoforge-schema/src/lint/naming/w05.rs","crates/ontoforge-schema/src/lint/snapshots/ontoforge_schema__lint__e08__tests__clean.snap","crates/ontoforge-schema/src/lint/snapshots/ontoforge_schema__lint__e08__tests__triggers.snap","crates/ontoforge-schema/src/lint/snapshots/ontoforge_schema__lint__e09__tests__clean.snap","crates/ontoforge-schema/src/lint/snapshots/ontoforge_schema__lint__e09__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/mod.rs","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w06__tests__clean.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w06__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w07__tests__clean.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w07__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w08__tests__clean.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w08__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w09__tests__clean.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w09__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w10__tests__clean.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w10__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w11__tests__clean.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w11__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/w06.rs","crates/ontoforge-schema/src/lint/structure/w07.rs","crates/ontoforge-schema/src/lint/structure/w08.rs","crates/ontoforge-schema/src/lint/structure/w09.rs","crates/ontoforge-schema/src/lint/structure/w10.rs","crates/ontoforge-schema/src/lint/structure/w11.rs","crates/ontoforge-schema/src/lint/words.rs","crates/ontoforge-schema/src/loader/discover.rs","crates/ontoforge-schema/src/loader/dispatch.rs","crates/ontoforge-schema/src/loader/fixture.rs","crates/ontoforge-schema/src/loader/mod.rs","crates/ontoforge-schema/src/loader/parse.rs","crates/ontoforge-schema/src/loader/scope.rs","crates/ontoforge-schema/src/loader/sourcemap.rs","crates/ontoforge-schema/src/schema_export.rs","crates/ontoforge-schema/src/validate/actions.rs","crates/ontoforge-schema/src/validate/bindings.rs","crates/ontoforge-schema/src/validate/links.rs","crates/ontoforge-schema/src/validate/mod.rs","crates/ontoforge-schema/src/validate/objects.rs","crates/ontoforge-schema/tests/action_types.rs","crates/ontoforge-schema/tests/bindings.rs","crates/ontoforge-schema/tests/diagnostics.rs","crates/ontoforge-schema/tests/link_types.rs","crates/ontoforge-schema/tests/lint_examples.rs","crates/ontoforge-schema/tests/load_examples.rs","crates/ontoforge-schema/tests/loader.rs","crates/ontoforge-schema/tests/object_types.rs","crates/ontoforge-schema/tests/schema_export.rs","crates/ontoforge-schema/tests/scope_documents.rs","crates/ontoforge-schema/tests/snapshots/scope_documents__scope_diagnostics.snap","crates/ontoforge/Cargo.toml","crates/ontoforge/src/cli.rs","crates/ontoforge/src/lib.rs","crates/ontoforge/src/lint_cmd.rs","crates/ontoforge/src/main.rs","crates/ontoforge/src/pipeline.rs","crates/ontoforge/src/render.rs","crates/ontoforge/tests/cli.rs","crates/ontoforge/tests/cli_color.rs","crates/ontoforge/tests/cli_lint.rs","crates/ontoforge/tests/cli_schema.rs","crates/ontoforge/tests/invalid_examples.rs","crates/ontoforge/tests/snapshots/cli__validate_unknown_field_human.snap","crates/ontoforge/tests/snapshots/cli_lint__e08_human.snap","crates/ontoforge/tests/snapshots/cli_lint__e14_human.snap","crates/ontoforge/tests/snapshots/cli_online__online_missing_column_human.snap","crates/ontoforge/tests/snapshots/cli_online_trino__unavailable_human.snap","crates/ontoforge/tests/snapshots/invalid_examples__v01-json-trailing-comma.snap","crates/ontoforge/tests/snapshots/invalid_examples__v01-yaml-duplicate-key.snap","crates/ontoforge/tests/snapshots/invalid_examples__v02-missing-kind.snap","crates/ontoforge/tests/snapshots/invalid_examples__v03-function-rule.snap","crates/ontoforge/tests/snapshots/invalid_examples__v03-reserved-kind.snap","crates/ontoforge/tests/snapshots/invalid_examples__v04-unknown-field.snap","crates/ontoforge/tests/snapshots/invalid_examples__v05-missing-backing.snap","crates/ontoforge/tests/snapshots/invalid_examples__v05-missing-primary-key.snap","crates/ontoforge/tests/snapshots/invalid_examples__v06-precision-on-string.snap","crates/ontoforge/tests/snapshots/invalid_examples__v06-unknown-base-type.snap","crates/ontoforge/tests/snapshots/invalid_examples__v07-bad-object-type-name.snap","crates/ontoforge/tests/snapshots/invalid_examples__v08-duplicate-binding.snap","crates/ontoforge/tests/snapshots/invalid_examples__v08-duplicate-object-type.snap","crates/ontoforge/tests/snapshots/invalid_examples__v08-link-end-clashes-property.snap","crates/ontoforge/tests/snapshots/invalid_examples__v09-binding-unknown-target.snap","crates/ontoforge/tests/snapshots/invalid_examples__v09-link-unknown-object-type.snap","crates/ontoforge/tests/snapshots/invalid_examples__v09-rule-unknown-property.snap","crates/ontoforge/tests/snapshots/invalid_examples__v10-float-primary-key.snap","crates/ontoforge/tests/snapshots/invalid_examples__v10-primary-key-not-required.snap","crates/ontoforge/tests/snapshots/invalid_examples__v11-backing-on-one-to-many.snap","crates/ontoforge/tests/snapshots/invalid_examples__v11-foreign-key-type-mismatch.snap","crates/ontoforge/tests/snapshots/invalid_examples__v11-many-to-many-with-foreign-key.snap","crates/ontoforge/tests/snapshots/invalid_examples__v12-create-missing-primary-key.snap","crates/ontoforge/tests/snapshots/invalid_examples__v12-current-time-on-string.snap","crates/ontoforge/tests/snapshots/invalid_examples__v12-modify-assigns-primary-key.snap","crates/ontoforge/tests/snapshots/invalid_examples__v12-value-source-type-mismatch.snap","crates/ontoforge/tests/snapshots/invalid_examples__v13-comparison-type-mismatch.snap","crates/ontoforge/tests/snapshots/invalid_examples__v13-non-boolean-condition.snap","crates/ontoforge/tests/snapshots/invalid_examples__v13-objects-not-object-reference.snap","crates/ontoforge/tests/snapshots/invalid_examples__v13-syntax-error.snap","crates/ontoforge/tests/snapshots/invalid_examples__v13-unknown-parameter.snap","crates/ontoforge/tests/snapshots/invalid_examples__v14-include-escapes-root.snap","crates/ontoforge/tests/snapshots/invalid_examples__v16-binding-outside-bindings-dir.snap","crates/ontoforge/tests/snapshots/invalid_examples__v16-fixture-escapes-root.snap","crates/ontoforge/tests/snapshots/invalid_examples__v16-source-without-binding.snap","examples/ontologies/aviation-json/actions.json","examples/ontologies/aviation-json/bindings/dev/bindings.json","examples/ontologies/aviation-json/bindings/dev/fixtures/aircraft.csv","examples/ontologies/aviation-json/bindings/dev/fixtures/flight_crew.csv","examples/ontologies/aviation-json/bindings/dev/fixtures/flights.csv","examples/ontologies/aviation-json/bindings/dev/fixtures/maintenance_records.csv","examples/ontologies/aviation-json/bindings/dev/fixtures/pilots.csv","examples/ontologies/aviation-json/links.json","examples/ontologies/aviation-json/objects.json","examples/ontologies/aviation-json/ontology.yaml","examples/ontologies/aviation/actions.yaml","examples/ontologies/aviation/bindings/dev/aircraft.yaml","examples/ontologies/aviation/bindings/dev/fixtures/aircraft.csv","examples/ontologies/aviation/bindings/dev/fixtures/flight_crew.csv","examples/ontologies/aviation/bindings/dev/fixtures/flights.csv","examples/ontologies/aviation/bindings/dev/fixtures/maintenance_records.csv","examples/ontologies/aviation/bindings/dev/fixtures/pilots.csv","examples/ontologies/aviation/bindings/dev/flight-crew.yaml","examples/ontologies/aviation/bindings/dev/flight.yaml","examples/ontologies/aviation/bindings/dev/maintenance-record.yaml","examples/ontologies/aviation/bindings/dev/pilot.yaml","examples/ontologies/aviation/links.yaml","examples/ontologies/aviation/objects.yaml","examples/ontologies/aviation/ontology.yaml","examples/ontologies/invalid/v01-json-trailing-comma/objects.json","examples/ontologies/invalid/v01-json-trailing-comma/ontology.yaml","examples/ontologies/invalid/v01-yaml-duplicate-key/objects.yaml","examples/ontologies/invalid/v01-yaml-duplicate-key/ontology.yaml","examples/ontologies/invalid/v02-missing-kind/objects.yaml","examples/ontologies/invalid/v02-missing-kind/ontology.yaml","examples/ontologies/invalid/v03-function-rule/defs.yaml","examples/ontologies/invalid/v03-function-rule/ontology.yaml","examples/ontologies/invalid/v03-reserved-kind/objects.yaml","examples/ontologies/invalid/v03-reserved-kind/ontology.yaml","examples/ontologies/invalid/v04-unknown-field/objects.yaml","examples/ontologies/invalid/v04-unknown-field/ontology.yaml","examples/ontologies/invalid/v05-missing-backing/objects.yaml","examples/ontologies/invalid/v05-missing-backing/ontology.yaml","examples/ontologies/invalid/v05-missing-primary-key/objects.yaml","examples/ontologies/invalid/v05-missing-primary-key/ontology.yaml","examples/ontologies/invalid/v06-precision-on-string/objects.yaml","examples/ontologies/invalid/v06-precision-on-string/ontology.yaml","examples/ontologies/invalid/v06-unknown-base-type/objects.yaml","examples/ontologies/invalid/v06-unknown-base-type/ontology.yaml","examples/ontologies/invalid/v07-bad-object-type-name/objects.yaml","examples/ontologies/invalid/v07-bad-object-type-name/ontology.yaml","examples/ontologies/invalid/v08-duplicate-binding/bindings/dev/a.yaml","examples/ontologies/invalid/v08-duplicate-binding/bindings/dev/b.yaml","examples/ontologies/invalid/v08-duplicate-binding/objects.yaml","examples/ontologies/invalid/v08-duplicate-binding/ontology.yaml","examples/ontologies/invalid/v08-duplicate-object-type/a.yaml","examples/ontologies/invalid/v08-duplicate-object-type/b.yaml","examples/ontologies/invalid/v08-duplicate-object-type/ontology.yaml","examples/ontologies/invalid/v08-link-end-clashes-property/defs.yaml","examples/ontologies/invalid/v08-link-end-clashes-property/ontology.yaml","examples/ontologies/invalid/v09-binding-unknown-target/bindings/dev/flight.yaml","examples/ontologies/invalid/v09-binding-unknown-target/objects.yaml","examples/ontologies/invalid/v09-binding-unknown-target/ontology.yaml","examples/ontologies/invalid/v09-link-unknown-object-type/defs.yaml","examples/ontologies/invalid/v09-link-unknown-object-type/ontology.yaml","examples/ontologies/invalid/v09-rule-unknown-property/defs.yaml","examples/ontologies/invalid/v09-rule-unknown-property/ontology.yaml","examples/ontologies/invalid/v10-float-primary-key/objects.yaml","examples/ontologies/invalid/v10-float-primary-key/ontology.yaml","examples/ontologies/invalid/v10-primary-key-not-required/objects.yaml","examples/ontologies/invalid/v10-primary-key-not-required/ontology.yaml","examples/ontologies/invalid/v11-backing-on-one-to-many/defs.yaml","examples/ontologies/invalid/v11-backing-on-one-to-many/ontology.yaml","examples/ontologies/invalid/v11-foreign-key-type-mismatch/defs.yaml","examples/ontologies/invalid/v11-foreign-key-type-mismatch/ontology.yaml","examples/ontologies/invalid/v11-many-to-many-with-foreign-key/defs.yaml","examples/ontologies/invalid/v11-many-to-many-with-foreign-key/ontology.yaml","examples/ontologies/invalid/v12-create-missing-primary-key/defs.yaml","examples/ontologies/invalid/v12-create-missing-primary-key/ontology.yaml","examples/ontologies/invalid/v12-current-time-on-string/defs.yaml","examples/ontologies/invalid/v12-current-time-on-string/ontology.yaml","examples/ontologies/invalid/v12-modify-assigns-primary-key/defs.yaml","examples/ontologies/invalid/v12-modify-assigns-primary-key/ontology.yaml","examples/ontologies/invalid/v12-value-source-type-mismatch/defs.yaml","examples/ontologies/invalid/v12-value-source-type-mismatch/ontology.yaml","examples/ontologies/invalid/v13-comparison-type-mismatch/defs.yaml","examples/ontologies/invalid/v13-comparison-type-mismatch/ontology.yaml","examples/ontologies/invalid/v13-non-boolean-condition/defs.yaml","examples/ontologies/invalid/v13-non-boolean-condition/ontology.yaml","examples/ontologies/invalid/v13-objects-not-object-reference/defs.yaml","examples/ontologies/invalid/v13-objects-not-object-reference/ontology.yaml","examples/ontologies/invalid/v13-syntax-error/defs.yaml","examples/ontologies/invalid/v13-syntax-error/ontology.yaml","examples/ontologies/invalid/v13-unknown-parameter/defs.yaml","examples/ontologies/invalid/v13-unknown-parameter/ontology.yaml","examples/ontologies/invalid/v14-include-escapes-root/ontology.yaml","examples/ontologies/invalid/v16-binding-outside-bindings-dir/flight-binding.yaml","examples/ontologies/invalid/v16-binding-outside-bindings-dir/objects.yaml","examples/ontologies/invalid/v16-binding-outside-bindings-dir/ontology.yaml","examples/ontologies/invalid/v16-fixture-escapes-root/bindings/dev/flight.yaml","examples/ontologies/invalid/v16-fixture-escapes-root/objects.yaml","examples/ontologies/invalid/v16-fixture-escapes-root/ontology.yaml","examples/ontologies/invalid/v16-source-without-binding/bindings/dev/flight.yaml","examples/ontologies/invalid/v16-source-without-binding/objects.yaml","examples/ontologies/invalid/v16-source-without-binding/ontology.yaml","rust-toolchain.toml","schemas/ontoforge-definition.schema.json","schemas/ontoforge-manifest.schema.json","schemas/ontoforge-scope.schema.json","target/milestone-ci-41759804733-integration.log","target/milestone-ci-41759804733-ubuntu.log","target/milestone-ci-41759804733-windows.log"]
covered_digest: "v4:sha256:ab5ef91bd70deeaba8abb80fd7352d53165111c05d99b47edda07a0ccb904a6c"
behavior_unverified: 0
overrides_applied: 1
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
decision_coverage:
  honored: 18
  total: 18
  not_honored: []
re_verification:
  previous_status: passed
  previous_score: 10/11
  trigger: "当前源码、canonical stale、实际测试与原始CI日志重新验证"
  gaps_closed:
    - "历史fingerprint覆盖不足与过期，已按真实实现重新建立证据"
    - "旧CI TLS guard fail-open，已由真实workflow hermetic负测及真实tree查询关闭"
  gaps_remaining: []
  regressions: []
overrides:
  - must_have: "G-1-2：aviation 的任意 YAML document 输入 kind 可补全并标红未知字段"
    reason: "01-UAT Test 2 用户明确接受pass；只观察到新增document关联schema，补全弹窗、bogus标红、JSON manifest无误报未逐项观察，保留该限制"
    accepted_by: "user (01-UAT.md Test 2)"
    accepted_at: "2026-10-16T12:56:06Z"
prohibition_review:
  disposition: autonomous_flagged
  authority: "历史同向第二意见授权与Claude Code AGREE；并非逐项人工resolution"
  non_authoritative: true
  judgment_items: 3
  unresolved_human_items: 3
  evidence: "target/milestone-tls-and-judgment-review.json"
  limitation: "unverified-prohibition — human review recommended：grep不能完全证明原创出处，自动接受不等同于完备出处证明"
---

# Phase 1: Workspace, Schema & Lint 验证报告

**Phase Goal:** As a ontology 开发者, I want to 用 YAML/JSON 编写 ontology，并通过 `ontoforge validate` / `ontoforge lint` 得到带源码定位的诊断，且仓库在 Windows 本机 `cargo test` 全绿, so that 我能在本机可靠地编写和验证定义.

**结论：human_needed。** 本次重新核对当前源码、实际接线和原始运行日志；11 条既有合并 truth 中 10 条 VERIFIED，编辑器用户体验一条沿用用户明确接受的 override。没有剩余功能 gap。三项judgment-tier prohibition保留NON-AUTHORITATIVE与unverified-prohibition标记，按verifier规则进入human_needed；11/11行为truth不变，不伪称功能回归或逐项人工观察。完整审计过程与共用证据见 `.planning/milestones/v1.0-audit/PHASE-01-03-REVERIFY.md`。

本次为历史合同 re-verification。旧报告 frontmatter 为 passed、正文仍为 human_needed，已按真实 UAT 接受记录统一；不重开用户已收口的 Test 2。ROADMAP现已整理为合法User Story，下面追踪实际Outcome，保留原用户接受与未观察限制。

## User Flow Coverage

| Step | Expected | Evidence | Status |
|---|---|---|---|
| 编写定义并validate | YAML/JSON同IR，有效定义无错 | pipeline/loader真实aviation与等价测试 | VERIFIED |
| 检查错误与lint | 文件line:col、非零退出码、可配置规则 | diagnostics/invalid_examples/cli_lint与CLI实际输出 | VERIFIED |
| 导出schema辅助编写 | schema有效、编辑器关联 | schema_export真实配置测试，编辑器三项未观察沿既有用户override | VERIFIED / accepted boundary |
| Outcome：本机可靠编写和验证 | Windows无外部服务workspace通过 | 新W1496passed；CLI/snapshot命名GREEN与既有UAT | VERIFIED |

## 2026-11-04 最终源码复签

核验HEAD `f8851ca073fed34e80e605f0ac91880259df7e12`。CI41759804733已独立查询为completed/success，Windows、Ubuntu、integration、phase7-real-services四必需job全部success；合成merge为22913c8e79ae98b6604109d6e05a15f27cd50292。亲读 `target/milestone-ci-41759804733-{windows,ubuntu,integration}.log`，workspace分别150组1496/0/117、150组1503/0/117；ignored不计PASS，后续fault/corpus不重复累计。

新cli_color测试在双平台实际通过；TLS hermetic各6/6，真实helperstep成功。SQLite fault5、PG source_guard1均实际执行。01/03受核生产模块相对adc0227无变更，共享CI与cli_color增量已由新run覆盖。旧41669982375仅作历史。

动态51passed的原runner因HEAD改变确实exit1；独立final-check exit0证明唯一变化是测试type alias、动态输入相同、凭证扫描及自有资源清理成功，不改写原runner结果。本轮仅核证和文档复签，没有运行Cargo、服务或提交。历史Claude Code同向意见保留为历史NON-AUTHORITATIVE依据，未新增沿用旧提示协议的第二意见。

## Goal Achievement

### Observable Truths

按 ROADMAP SC1–5 及旧报告的 01-14 合同保持 11 条；01-01..13 的细项在后面的 PLAN coverage 逐组对应到这些 SC，未因旧报告只列 gap closure 而跳过基础能力。

| # | Truth | Status | 当前证据 |
|---|---|---|---|
| 1 | SC1：ADR-0011 的 10 个 crate、edition 2024、resolver 3、Rust 1.98；Windows 无 Docker workspace 测试通过 | VERIFIED | Cargo 配置仍符合；原 10 crate 均在，后继新增 engine 不构成回退。原始 CI 41669982375 的 workspace step 1490 passed / 0 failed / 117 ignored；Ubuntu 1497/0/117。新 HEAD 的完整回归由 orchestrator 另行记录，不把旧 run 当作新修复结果 |
| 2 | SC2：多文件 object/link/action 合并为 IR，YAML/JSON 等价 | VERIFIED | `pipeline.rs:23` 起调用 `load_dir`；loader 实际 discover/parse/dispatch/validate。真实例子测试比较完整 IR 与 5 object / 5 link / 3 action，CI windows.log:3295 通过；本次既有 CLI validate aviation 返回 `[]`、exit 0 |
| 3 | SC3：错误带文件名及 line:col、退出码非零 | VERIFIED | dispatch → SourceMap.locate → Diagnostic → render；本次 CLI invalid/v04 输出 ONT-V04、objects.yaml、3:1、span 34/9、primaryKey 建议、exit 1；CI diagnostics/invalid_examples 为值级和快照断言 |
| 4 | SC4：v1 E/W01–W11、human/JSON、E 退出非零、规则级配置 | VERIFIED | `lint_cmd.rs:20` 先结构校验再 config+registry；registry 汇入 e08/e09、naming、structure，配置覆盖 severity；CLI 正负测试和规则测试通过；本次 lint aviation --deny-warnings 输出 `[]`、exit 0 |
| 5 | SC5：导出 JSON Schema 且真实例子通过 | VERIFIED | schema_export 生成真实 IR schema；tests/schema_export 读取入库 schema 和例子校验，不返回静态成功；CI windows.log:3373 等证明入库一致、valid/invalid 对照 |
| 6 | G-1-2：新增 YAML document 能补全 kind、收窄字段并标红未知字段 | PASSED (override) | 01-UAT Test 2：用户明确“要不这个先pass吧”；截图只证实新 document 已关联 schema。补全弹窗、bogus 标红、aviation-json manifest 无误报未逐项观察；历史 headless LSP 结果不伪称编辑器实机观察 |
| 7 | Zed / VS Code 严格 JSON、相同本地 yaml.schemas 映射 | VERIFIED | 真实两个 settings 文件被 schema_export 测试读取，only_key、assert_eq、本地路径与 exported schemas 断言；CI windows.log:3376 通过 |
| 8 | manifest 不被误关联到 definition schema | VERIFIED | 映射含 manifest 排除；测试遍历真实例子断言 ontology.yaml 不覆盖。yaml-language-server 1.25.0 的历史 headless 正反对照保留；本轮配置与该验证后未发生语义变化，用户接受边界见 T6 |
| 9 | aviation 每 document modeline 与 clean / IR 等价保持 | VERIFIED | tests/schema_export 对首行、每个 --- 的下一行、document 数量、schema 可解析路径逐一断言；CI windows.log:3370/3295，本次 validate/lint 都 clean |
| 10 | CONTRIBUTING 编辑器说明准确 | VERIFIED | 编辑器接入章节包含单 document modeline、项目映射、manifest 排除、JetBrains JSON Schema Mappings 与 Ctrl+Space；与实际 settings 和 schema 路径一致 |
| 11 | 配置/schema/modeline 错误有实质回归断言 | VERIFIED | 配置测试逐层 strict JSON、known schema、本地文件、实际遍历非空、全部示例覆盖；modeline 测试逐 document 比对。不是仅检查文件存在 |

**Score：11/11（含 1 个用户接受 override），behavior_unverified：0。** T6 是显式接受的观察边界，不算一次本轮实机测试。

### PLAN / Requirements Coverage

| PLAN | 需求与新增细项 | 实现 / 测试证据 | 状态 |
|---|---|---|---|
| 01-01 / 02 | SCHEMA-01/06、DX-03；manifest、kind、命名、诊断JSON、workspace/CI | loader/dispatch/sourcemap/diag、CLI/render、Cargo/CI；cli/diagnostics/invalid_examples；ADR-0012 accepted | VERIFIED |
| 01-03 | SCHEMA-01/06；glob、路径逃逸、symlink、4 MiB、严格JSON、栈隔离、BOM/CRLF/中文列、顺序 | loader/discover/parse/mod；34 个 loader 测试及 diagnostics；内部 32 MiB spawn_scoped、调用方 1 MiB 测试 | VERIFIED |
| 01-04 | SCHEMA-02/03；完整 object 字段、13 base types、required、PK/title约束 | ir/object/basetype、validate/objects；object_types、load_examples、invalid fixtures | VERIFIED |
| 01-05 | SCHEMA-04；3 种 link cardinality、端字段、FK类型/访问器唯一性 | ir/link、validate/links；link_types 21、真实5 links | VERIFIED |
| 01-06 / 07 / 12 | SCHEMA-05/06；参数/规则/值来源、CEL预检/作用域遍历/类型推断、ONT-V13 | validate/actions + expr analyze/limits/walk/criteria；action_types 28、analyze 21、criteria 20；长度/深度/词法/闭包隔离的实际断言 | VERIFIED |
| 01-08 | LINT-01/03/04；E08/E09、退出码、配置拒绝未知/ONT-V ID | lint_cmd、lint/config/e08/e09；cli_lint 17、配置单元测试 | VERIFIED |
| 01-09 / 14 | SCHEMA-07；schema导出、编辑器关联 | schema_export 14、cli_schema 4、真实配置/fixtures；编辑器限制 T6 | VERIFIED / accepted boundary |
| 01-10 / 11 | LINT-02/04、DX-03；W01–W11、阈值选项、clean例子 | lint/naming/structure 全部注册且有触发/clean/override测试；lint_examples | VERIFIED |
| 01-13 | LINT-04、SCHEMA-06、LINT-03；DeTable、heck、anstream、共享行列换算 | 实际调用已读；旧私有 RawVisitor/DATETIME_KEY 移除；`cli_color` 本 verifier 单项执行 1/1；字节/中文列 tests 与历史提交差异 | VERIFIED |

12 个 requirement ID 全部由 PLAN 声明；与 REQUIREMENTS 对照无 orphan。历史格式 checkpoint 是已接受决策，不当作运行时测试。01-13 的“无新增下载”按该计划历史锁文件/提交语境理解，不要求后继 phase 禁止增加依赖。

### Required Artifacts / Key Links / Data Flow

`query verify.artifacts`、`query verify.key-links` 对 14 份 PLAN 本次实际运行：58/58 artifact 声明、32/32 key link 声明通过；01-13 没有 artifact/key_links block，N/A，不计作通过项。另手工追踪：

| 起点 | 实际路径 | 结果 |
|---|---|---|
| CLI validate | main/lib → pipeline → schema loader → expr.check_ontology → Diagnostic | WIRED；真实文件数据 |
| lint | pipeline errors early return → load_config → registry.run → SourceMap.locate | WIRED；结构错误不可配置绕过 |
| 输出 | Diagnostic + sources.text → JSON serialization / miette NamedSource | FLOWING；不是硬编码空结果，clean时 [] 是真实零诊断 |
| schema export | IR/schema_export → 入库 schema → 例子校验 → 编辑器映射 | WIRED；真实导出与配置读取 |
| 栈与位置 | loader/analyze spawn_scoped；diag.offset_to_line_col → parse/config/walk | WIRED；非独立未调用 helper |

### Behavioral Spot-Checks

本 verifier 不运行 Cargo 或服务。本次直接执行已有编译产物：

| 命令 | 结果 |
|---|---|
| `target/debug/deps/cli_color-4d01ddf867a0fcb3.exe --exact color_environment_controls_human_output_and_never_colors_json --nocapture` | exit 0，1 passed / 0 failed / 0 ignored，0.64s；6 个颜色环境场景及 JSON 无ANSI |
| `target/debug/ontoforge.exe validate examples/ontologies/aviation --format json` | `[]`，exit 0 |
| `target/debug/ontoforge.exe validate examples/ontologies/invalid/v04-unknown-field --format json` | ONT-V04 / objects.yaml / 3:1，exit 1 |
| `target/debug/ontoforge.exe lint examples/ontologies/aviation --deny-warnings --format json` | `[]`，exit 0 |

源码主体自 `adc0227` 无 Phase01 功能差异；`81788e4` 的颜色测试新增由本轮单项执行覆盖。01-VALIDATION 的 451 passed 是 Nyquist 执行单元的运行，复用而非本 verifier 重跑；新完整 workspace、TLS guard 与 INT-01 验证边界单独记录于共享审计。

### Test Quality / Anti-Patterns

schema/expr 需求测试未找到 ignore；真实 invalid fixture 与 expected code/line/column/JSON keys/IR 数量断言达到值级。schema 导出比较的是独立入库契约并校验实际例子，不把新输出即时写成 expected。无未引用 TBD/FIXME/XXX。symlink/非UTF-8分支在不支持的平台可提前返回，本机 ok 不扩大解释为已覆盖该系统能力；Ubuntu CI 有对应运行结果。

### Prohibitions

原字段已复核：01-01-PLAN.md:104（DX-03）、:111（SCHEMA-06）、01-08-PLAN.md:81（LINT-04）均为 `verification: "judgment"`。三者的 `status: "resolved"` 是规划决策状态，非人工验收完成。路径与ONT-V已有明确自动行为证据；源码原创出处只有限定范围扫描。这3个flag表示judgment处置未闭环，不能误写为3个功能测试缺口。

| PLAN / 禁止项 | 证据与处置 |
|---|---|
| 01-01：不沿用参考库标识符、不复制其代码 | 对列举产品目录和文件扫描禁名零命中；原创出处无法仅凭 grep 完全证明。NON-AUTHORITATIVE 自动判断；unverified-prohibition，历史同向授权不等同逐项人工resolution |
| 01-01：诊断/快照/生成产物不含本机绝对路径或用户名 | 相对文件生成链路、CLI/loader路径断言与快照扫描；NON-AUTHORITATIVE 自动判断，保留扫描范围，不将构建日志路径混同产品输出 |
| 01-08：配置/CLI不得关闭降级ONT-V | lint先校验；配置拒绝V ID；`validate_codes_cannot_be_turned_off_by_config` 值级 negative test；NON-AUTHORITATIVE 自动判断有确定行为支持 |
| 01-13 backstop：诊断 code/位置/JSON形状与既有快照不因该refactor变化 | `git diff --name-only d021687 07a0e06 -- ':(glob)**/*.snap'` 零输出；diag模型未改变，位置换算有独立文字坐标断言，当前快照tests通过；VERIFIED |

第二意见原始记录：`target/milestone-tls-and-judgment-review.json`，结果 `AGREE`。判断项保留“unverified-prohibition — human review recommended”的非权威证据标识，历史同向自动推进授权保留，但不能替代逐项人工resolution；本次按高优先级verifier规则显式标human_needed，不把自动判断写成完备原创性证明。

### Decision Coverage / Probe / Human Verification

Decision coverage 本次工具实跑 18/18 honored，0 not_honored。无本 Phase 声明的 shell probe，Probe Execution N/A。01-UAT Test1/Test3 pass，Test2用户明确接受，其三个编辑器未观察项沿既有override，不重开。另有3项judgment-tier prohibition在frontmatter列为human_verification；它们是自动判断处置边界，不能由旧UAT代替。

### Advisory（re-verification）

历史 loader/W07/W08 性能、特殊文件名净化、strict编辑器配置键约束的风险仍为 advisory，不从已通过SC推导“任何输入都安全”。旧报告的 warning 并未在本次全部重跑或声称修复。新 TLS guard 的真实问题已由本轮共享 CI 加固处理：本 verifier 实跑 hermetic 6/6（0.439s），bash -n、diff-check通过；主会话真实tree helper日志 tls_real_exit=0。其两个OS新CI已经通过hermetic与真实helper，旧guard只作历史缺陷。

### 历史证据与覆盖边界

2026-10-16 的 299 passed、yaml-language-server 1.25.0 headless 对照与 UAT 是历史证据；本次分清其来源，避免旧“10/11但passed”矛盾。canonical fingerprint 覆盖本阶段真实实现、fixture、测试、约束ADR、UAT/VALIDATION、Cargo与CI；自动加入自身PLAN/SUMMARY。只做概览digest的全局ROADMAP/STATE/REQUIREMENTS不入指纹，其SC/需求本轮已读取并在本报告列明。当前新CI入口已经41759804733实际执行。

---

_Verified: 2026-11-04T02:55:02Z_
_Verifier: gsd-verifier_
