---
phase: 03-edit-store-scope-model
verified: 2026-11-04T02:55:02Z
status: passed
score: 8/8 must-haves verified
covered_files: [".gitattributes",".github/workflows/ci.yml",".planning/adr/0002-naming-convention.md",".planning/adr/0016-storage-system-data-and-typed-edit-store.md",".planning/adr/0018-authz-scope-and-sql-tiers.md",".planning/adr/0023-phase3-crate-dependency-direction.md",".planning/adr/0024-edit-store-physical-format.md",".planning/adr/0025-row-predicate-semantics.md",".planning/adr/0026-edit-log-format.md",".planning/phases/03-edit-store-scope-model/03-01-PLAN.md",".planning/phases/03-edit-store-scope-model/03-01-SUMMARY.md",".planning/phases/03-edit-store-scope-model/03-02-PLAN.md",".planning/phases/03-edit-store-scope-model/03-02-SUMMARY.md",".planning/phases/03-edit-store-scope-model/03-03-PLAN.md",".planning/phases/03-edit-store-scope-model/03-03-SUMMARY.md",".planning/phases/03-edit-store-scope-model/03-04-PLAN.md",".planning/phases/03-edit-store-scope-model/03-04-SUMMARY.md",".planning/phases/03-edit-store-scope-model/03-05-PLAN.md",".planning/phases/03-edit-store-scope-model/03-05-SUMMARY.md",".planning/phases/03-edit-store-scope-model/03-06-PLAN.md",".planning/phases/03-edit-store-scope-model/03-06-SUMMARY.md",".planning/phases/03-edit-store-scope-model/03-07-PLAN.md",".planning/phases/03-edit-store-scope-model/03-07-SUMMARY.md",".planning/phases/03-edit-store-scope-model/03-08-PLAN.md",".planning/phases/03-edit-store-scope-model/03-08-SUMMARY.md",".planning/phases/03-edit-store-scope-model/03-09-PLAN.md",".planning/phases/03-edit-store-scope-model/03-09-SUMMARY.md",".planning/phases/03-edit-store-scope-model/03-10-PLAN.md",".planning/phases/03-edit-store-scope-model/03-10-SUMMARY.md",".planning/phases/03-edit-store-scope-model/03-11-PLAN.md",".planning/phases/03-edit-store-scope-model/03-11-SUMMARY.md",".planning/phases/03-edit-store-scope-model/03-12-PLAN.md",".planning/phases/03-edit-store-scope-model/03-12-SUMMARY.md",".planning/phases/03-edit-store-scope-model/03-13-PLAN.md",".planning/phases/03-edit-store-scope-model/03-13-SUMMARY.md",".planning/phases/03-edit-store-scope-model/03-14-PLAN.md",".planning/phases/03-edit-store-scope-model/03-14-SUMMARY.md",".planning/phases/03-edit-store-scope-model/03-15-PLAN.md",".planning/phases/03-edit-store-scope-model/03-15-SUMMARY.md",".planning/phases/03-edit-store-scope-model/03-16-PLAN.md",".planning/phases/03-edit-store-scope-model/03-16-SUMMARY.md",".planning/phases/03-edit-store-scope-model/03-17-PLAN.md",".planning/phases/03-edit-store-scope-model/03-17-SUMMARY.md",".planning/phases/03-edit-store-scope-model/03-18-PLAN.md",".planning/phases/03-edit-store-scope-model/03-18-SUMMARY.md",".planning/phases/03-edit-store-scope-model/03-19-PLAN.md",".planning/phases/03-edit-store-scope-model/03-19-SUMMARY.md",".planning/phases/03-edit-store-scope-model/03-20-PLAN.md",".planning/phases/03-edit-store-scope-model/03-20-SUMMARY.md",".planning/phases/03-edit-store-scope-model/03-21-PLAN.md",".planning/phases/03-edit-store-scope-model/03-21-SUMMARY.md",".planning/phases/03-edit-store-scope-model/03-22-PLAN.md",".planning/phases/03-edit-store-scope-model/03-22-SUMMARY.md",".planning/phases/03-edit-store-scope-model/03-23-PLAN.md",".planning/phases/03-edit-store-scope-model/03-23-SUMMARY.md",".planning/phases/03-edit-store-scope-model/03-CONTEXT.md",".planning/phases/03-edit-store-scope-model/03-UAT.md",".planning/phases/03-edit-store-scope-model/03-VALIDATION.md","CONTRIBUTING.md","Cargo.lock","Cargo.toml","ci/check-tls-deps.sh","ci/pg/rowpred_corpus.sql","ci/pg/seed.sql","ci/query-services.sh","ci/server-services.sh","ci/test_tls_deps.py","ci/trino/catalog/editstore.properties","crates/ontoforge-auth/Cargo.toml","crates/ontoforge-auth/src/authn/dev.rs","crates/ontoforge-auth/src/authn/jwks.rs","crates/ontoforge-auth/src/authn/mod.rs","crates/ontoforge-auth/src/authn/oidc.rs","crates/ontoforge-auth/src/authn/principal.rs","crates/ontoforge-auth/src/lib.rs","crates/ontoforge-auth/src/scope/compile.rs","crates/ontoforge-auth/src/scope/coverage.rs","crates/ontoforge-auth/src/scope/lower.rs","crates/ontoforge-auth/src/scope/merge.rs","crates/ontoforge-auth/src/scope/mod.rs","crates/ontoforge-auth/tests/criteria_coverage.rs","crates/ontoforge-auth/tests/crypto_backend.rs","crates/ontoforge-auth/tests/data/README.md","crates/ontoforge-auth/tests/data/oidc_es256.pem","crates/ontoforge-auth/tests/data/oidc_rs256.pem","crates/ontoforge-auth/tests/data/rowpred_corpus.json","crates/ontoforge-auth/tests/dev_token.rs","crates/ontoforge-auth/tests/oidc.rs","crates/ontoforge-auth/tests/rowpred_common/mod.rs","crates/ontoforge-auth/tests/rowpred_diff.rs","crates/ontoforge-auth/tests/rowpred_trino_it.rs","crates/ontoforge-auth/tests/scope_compile.rs","crates/ontoforge-auth/tests/scope_merge.rs","crates/ontoforge-auth/tests/snapshots/scope_compile__scope_compile_diagnostics_json.snap","crates/ontoforge-engine/Cargo.toml","crates/ontoforge-engine/src/config.rs","crates/ontoforge-engine/src/lib.rs","crates/ontoforge-engine/src/printer/ast.rs","crates/ontoforge-engine/src/printer/call.rs","crates/ontoforge-engine/src/printer/dialect.rs","crates/ontoforge-engine/src/printer/escape.rs","crates/ontoforge-engine/src/printer/l2.rs","crates/ontoforge-engine/src/printer/mod.rs","crates/ontoforge-engine/src/printer/snapshots/ontoforge_engine__printer__tests__print_sqlite.snap","crates/ontoforge-engine/src/printer/snapshots/ontoforge_engine__printer__tests__print_trino.snap","crates/ontoforge-engine/src/printer/sqlite_ddl.rs","crates/ontoforge-engine/src/printer/tests.rs","crates/ontoforge-engine/src/trino/cancel.rs","crates/ontoforge-engine/src/trino/classify.rs","crates/ontoforge-engine/src/trino/client.rs","crates/ontoforge-engine/src/trino/decode.rs","crates/ontoforge-engine/src/trino/headers.rs","crates/ontoforge-engine/src/trino/l2_health.rs","crates/ontoforge-engine/src/trino/l2_monitor.rs","crates/ontoforge-engine/src/trino/mod.rs","crates/ontoforge-expr/Cargo.toml","crates/ontoforge-expr/src/analyze.rs","crates/ontoforge-expr/src/check.rs","crates/ontoforge-expr/src/infer.rs","crates/ontoforge-expr/src/lib.rs","crates/ontoforge-expr/src/limits.rs","crates/ontoforge-expr/src/rowpred/eval.rs","crates/ontoforge-expr/src/rowpred/ir.rs","crates/ontoforge-expr/src/rowpred/mod.rs","crates/ontoforge-expr/src/rowpred/parse.rs","crates/ontoforge-expr/src/rowpred/value.rs","crates/ontoforge-expr/src/walk.rs","crates/ontoforge-expr/tests/analyze.rs","crates/ontoforge-expr/tests/common/mod.rs","crates/ontoforge-expr/tests/criteria.rs","crates/ontoforge-expr/tests/rowpred_eval.proptest-regressions","crates/ontoforge-expr/tests/rowpred_eval.rs","crates/ontoforge-expr/tests/rowpred_subset.rs","crates/ontoforge-schema/src/diag.rs","crates/ontoforge-schema/src/ir/action.rs","crates/ontoforge-schema/src/ir/attrs.rs","crates/ontoforge-schema/src/ir/basetype.rs","crates/ontoforge-schema/src/ir/binding.rs","crates/ontoforge-schema/src/ir/link.rs","crates/ontoforge-schema/src/ir/manifest.rs","crates/ontoforge-schema/src/ir/mod.rs","crates/ontoforge-schema/src/ir/names.rs","crates/ontoforge-schema/src/ir/object.rs","crates/ontoforge-schema/src/ir/scope.rs","crates/ontoforge-schema/src/lib.rs","crates/ontoforge-schema/src/lint/binding/e13.rs","crates/ontoforge-schema/src/lint/binding/e14.rs","crates/ontoforge-schema/src/lint/binding/e15.rs","crates/ontoforge-schema/src/lint/binding/e16.rs","crates/ontoforge-schema/src/lint/binding/mod.rs","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e13__tests__clean.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e13__tests__triggers.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e14__tests__clean.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e14__tests__triggers.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e15__tests__clean.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e15__tests__triggers.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e16__tests__clean.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__e16__tests__triggers.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__w17__tests__clean.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__w17__tests__triggers.snap","crates/ontoforge-schema/src/lint/binding/snapshots/ontoforge_schema__lint__binding__w17__tests__unreadable.snap","crates/ontoforge-schema/src/lint/binding/w17.rs","crates/ontoforge-schema/src/lint/config.rs","crates/ontoforge-schema/src/lint/e08.rs","crates/ontoforge-schema/src/lint/e09.rs","crates/ontoforge-schema/src/lint/mod.rs","crates/ontoforge-schema/src/lint/naming/mod.rs","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w01__tests__clean.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w01__tests__triggers.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w02__tests__clean.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w02__tests__triggers.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w03__tests__clean.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w03__tests__triggers.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w04__tests__clean.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w04__tests__triggers.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w05__tests__clean.snap","crates/ontoforge-schema/src/lint/naming/snapshots/ontoforge_schema__lint__naming__w05__tests__triggers.snap","crates/ontoforge-schema/src/lint/naming/w01.rs","crates/ontoforge-schema/src/lint/naming/w02.rs","crates/ontoforge-schema/src/lint/naming/w03.rs","crates/ontoforge-schema/src/lint/naming/w04.rs","crates/ontoforge-schema/src/lint/naming/w05.rs","crates/ontoforge-schema/src/lint/snapshots/ontoforge_schema__lint__e08__tests__clean.snap","crates/ontoforge-schema/src/lint/snapshots/ontoforge_schema__lint__e08__tests__triggers.snap","crates/ontoforge-schema/src/lint/snapshots/ontoforge_schema__lint__e09__tests__clean.snap","crates/ontoforge-schema/src/lint/snapshots/ontoforge_schema__lint__e09__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/mod.rs","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w06__tests__clean.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w06__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w07__tests__clean.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w07__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w08__tests__clean.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w08__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w09__tests__clean.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w09__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w10__tests__clean.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w10__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w11__tests__clean.snap","crates/ontoforge-schema/src/lint/structure/snapshots/ontoforge_schema__lint__structure__w11__tests__triggers.snap","crates/ontoforge-schema/src/lint/structure/w06.rs","crates/ontoforge-schema/src/lint/structure/w07.rs","crates/ontoforge-schema/src/lint/structure/w08.rs","crates/ontoforge-schema/src/lint/structure/w09.rs","crates/ontoforge-schema/src/lint/structure/w10.rs","crates/ontoforge-schema/src/lint/structure/w11.rs","crates/ontoforge-schema/src/lint/words.rs","crates/ontoforge-schema/src/loader/discover.rs","crates/ontoforge-schema/src/loader/dispatch.rs","crates/ontoforge-schema/src/loader/fixture.rs","crates/ontoforge-schema/src/loader/mod.rs","crates/ontoforge-schema/src/loader/parse.rs","crates/ontoforge-schema/src/loader/scope.rs","crates/ontoforge-schema/src/loader/sourcemap.rs","crates/ontoforge-schema/src/schema_export.rs","crates/ontoforge-schema/src/validate/actions.rs","crates/ontoforge-schema/src/validate/bindings.rs","crates/ontoforge-schema/src/validate/links.rs","crates/ontoforge-schema/src/validate/mod.rs","crates/ontoforge-schema/src/validate/objects.rs","crates/ontoforge-schema/tests/scope_documents.rs","crates/ontoforge-server/src/context.rs","crates/ontoforge-server/src/history_page.rs","crates/ontoforge-server/tests/postgres_http_it.rs","crates/ontoforge-storage/Cargo.toml","crates/ontoforge-storage/migrations/postgres/0001_definitions.sql","crates/ontoforge-storage/migrations/postgres/0002_edit_log.sql","crates/ontoforge-storage/migrations/postgres/0003_scope.sql","crates/ontoforge-storage/migrations/postgres/0004_action.sql","crates/ontoforge-storage/migrations/postgres/0005_read_audit.sql","crates/ontoforge-storage/migrations/postgres/0006_audit_truncated.sql","crates/ontoforge-storage/migrations/sqlite/0001_definitions.sql","crates/ontoforge-storage/migrations/sqlite/0002_edit_log.sql","crates/ontoforge-storage/migrations/sqlite/0003_scope.sql","crates/ontoforge-storage/migrations/sqlite/0004_action.sql","crates/ontoforge-storage/migrations/sqlite/0005_read_audit.sql","crates/ontoforge-storage/migrations/sqlite/0006_audit_truncated.sql","crates/ontoforge-storage/src/commit.rs","crates/ontoforge-storage/src/ddl.rs","crates/ontoforge-storage/src/diff.rs","crates/ontoforge-storage/src/error.rs","crates/ontoforge-storage/src/history.rs","crates/ontoforge-storage/src/import.rs","crates/ontoforge-storage/src/index.rs","crates/ontoforge-storage/src/lib.rs","crates/ontoforge-storage/src/log.rs","crates/ontoforge-storage/src/migrate.rs","crates/ontoforge-storage/src/naming.rs","crates/ontoforge-storage/src/physical.rs","crates/ontoforge-storage/src/query_read.rs","crates/ontoforge-storage/src/read_audit.rs","crates/ontoforge-storage/src/reload.rs","crates/ontoforge-storage/src/request_snapshot.rs","crates/ontoforge-storage/src/scope_store.rs","crates/ontoforge-storage/src/snapshots/ontoforge_storage__ddl__tests__ddl_postgres.snap","crates/ontoforge-storage/src/snapshots/ontoforge_storage__ddl__tests__ddl_sqlite.snap","crates/ontoforge-storage/src/sql_exit.rs","crates/ontoforge-storage/src/state.rs","crates/ontoforge-storage/src/store.rs","crates/ontoforge-storage/src/value.rs","crates/ontoforge-storage/tests/conformance/mod.rs","crates/ontoforge-storage/tests/conformance_pg.rs","crates/ontoforge-storage/tests/conformance_sqlite.rs","crates/ontoforge-storage/tests/frozen_history.rs","crates/ontoforge-storage/tests/history_masking.rs","crates/ontoforge-storage/tests/import.rs","crates/ontoforge-storage/tests/migrate.rs","crates/ontoforge-storage/tests/migrate_pg.rs","crates/ontoforge-storage/tests/model/mod.rs","crates/ontoforge-storage/tests/model_prop.proptest-regressions","crates/ontoforge-storage/tests/model_prop.rs","crates/ontoforge-storage/tests/query_read.rs","crates/ontoforge-storage/tests/query_read_pg.rs","crates/ontoforge-storage/tests/reload.rs","crates/ontoforge-storage/tests/reload_pg.rs","crates/ontoforge-storage/tests/reload_trino_it.rs","crates/ontoforge-storage/tests/request_snapshot.rs","crates/ontoforge-storage/tests/scope_store.rs","crates/ontoforge-storage/tests/sql_exit_guard.rs","crates/ontoforge-storage/tests/sqlite_connection.rs","crates/ontoforge-storage/tests/support/mod.rs","crates/ontoforge/Cargo.toml","crates/ontoforge/src/cli.rs","crates/ontoforge/src/dev_token.rs","crates/ontoforge/src/import.rs","crates/ontoforge/src/lib.rs","crates/ontoforge/src/main.rs","crates/ontoforge/src/migrate.rs","crates/ontoforge/src/reload.rs","crates/ontoforge/src/scope.rs","crates/ontoforge/tests/cli_dev_token.rs","crates/ontoforge/tests/cli_import.rs","crates/ontoforge/tests/cli_migrate.rs","crates/ontoforge/tests/cli_reload.rs","crates/ontoforge/tests/cli_scope.rs","docs/edit-store.md","docs/scope.md","examples/ontologies/aviation/actions.yaml","examples/ontologies/aviation/bindings/dev/aircraft.yaml","examples/ontologies/aviation/bindings/dev/fixtures/aircraft.csv","examples/ontologies/aviation/bindings/dev/fixtures/flight_crew.csv","examples/ontologies/aviation/bindings/dev/fixtures/flights.csv","examples/ontologies/aviation/bindings/dev/fixtures/maintenance_records.csv","examples/ontologies/aviation/bindings/dev/fixtures/pilots.csv","examples/ontologies/aviation/bindings/dev/flight-crew.yaml","examples/ontologies/aviation/bindings/dev/flight.yaml","examples/ontologies/aviation/bindings/dev/maintenance-record.yaml","examples/ontologies/aviation/bindings/dev/pilot.yaml","examples/ontologies/aviation/links.yaml","examples/ontologies/aviation/objects.yaml","examples/ontologies/aviation/ontology.yaml","examples/server/ontoforge-server.dev.toml","rust-toolchain.toml","schemas/ontoforge-definition.schema.json","schemas/ontoforge-manifest.schema.json","schemas/ontoforge-scope.schema.json","target/milestone-ci-41759804733-integration.log","target/milestone-ci-41759804733-ubuntu.log","target/milestone-ci-41759804733-windows.log"]
covered_digest: "v4:sha256:ecdf7376469d028d3b5f2b78f6b5f1bc2d2a0dfbc9047cbbde6f63c25dbfe5b3"
behavior_unverified: 0
overrides_applied: 0
human_verification: []
deferred:
  - truth: "STORE-05 source OneToOne基数报告"
    addressed_in: "Phase 7.1"
    evidence: "ROADMAP Phase7.1 SC1报告source冲突、SC2真实CLI退出码。全需求PARTIAL，edit子合同通过。"
decision_coverage:
  honored: 18
  total: 18
  not_honored: []
re_verification:
  previous_status: passed
  previous_score: 8/8
  trigger: "当前源码、canonical stale、实际测试与原始CI日志重新验证"
  gaps_closed:
    - "历史fingerprint覆盖不足与过期，已按真实实现重新建立证据"
    - "旧CI TLS guard fail-open，已由真实workflow hermetic负测及真实tree查询关闭"
  gaps_remaining: []
  regressions: []
---

# Phase 3: Edit Store & Scope Model 验证报告

**Phase Goal:** As a 平台管理员, I want to 让 edit store 按 object type 生成强类型表并安全演进、所有写入有不可变 edit log，配置用户 / 组 / 角色与 scope 规则，并使同一份 CEL 行谓词的 SQL 编译与 Rust 单行求值结果一致, so that 存储和权限行为可以安全演进并接受审计.

**结论：passed，8/8 must-haves verified。** 本次重新读取实现与断言，并亲核冻结源码 `adc0227` 的 CI 41669982375 原始日志。没有依赖 SUMMARY 的“已完成”作为行为证据，也没有把 fingerprint stale 当作功能失败。当前 storage/auth/expr/CLI Phase03 源码与该冻结点一致；当前 INT-01 engine accessor / MCP 修复与新增 CI 入口的通过状态由 orchestrator 单独验证。

旧报告 fingerprint 仅覆盖 CI、cache fixture、一份测试和规划文件，未充分覆盖实现。本次扩大到真实生产实现、测试及关联配置。共用审计、原始日志行号、自动判断授权见 `.planning/milestones/v1.0-audit/PHASE-01-03-REVERIFY.md`。

## User Flow Coverage

| Step | Expected | Evidence | Status |
|---|---|---|---|
| reload与migrate定义 | 增量DDL、破坏性拒绝或归档、失败保旧 | 真实SQLite/PG reload/migrate与Trino缓存可见性测试 | VERIFIED |
| 写入并读历史 | patch/version/log/history一致且不可改写 | 共享conformance、独立model、history_masking与fresh PG HTTP history | VERIFIED |
| 配置角色并认证 | 主体合并、列行不泄露、SQL/Rust一致 | scope/认证测试与真实Trino differential | VERIFIED |
| Outcome：安全演进且可审计 | 定义栅栏、权限重检与真实日志生效 | 五SC及三个closure truth均具行为证据；source报告按下述边界保留 | VERIFIED |

## Deferred Requirement Boundary

STORE-05全需求PARTIAL：edit侧OneToOne唯一索引/并发FK约束已通过，source基数报告未实现。ROADMAP Phase7.1 SC1明确安排 `validate --online --check-cardinality` 的source冲突报告，SC2安排真实CLI退出码测试。按Step9b显式deferred；不标全需求SATISFIED、不补requirements-completed、不称业务回归。Phase03八条既定truth通过，milestone仍必须完成Phase7.1。

## 2026-11-04 最终源码复签

核验HEAD `f8851ca073fed34e80e605f0ac91880259df7e12`。CI41759804733已独立查询为completed/success，Windows、Ubuntu、integration、phase7-real-services四必需job全部success；合成merge为22913c8e79ae98b6604109d6e05a15f27cd50292。亲读 `target/milestone-ci-41759804733-{windows,ubuntu,integration}.log`，workspace分别150组1496/0/117、150组1503/0/117；ignored不计PASS，后续fault/corpus不重复累计。

新cli_color测试在双平台实际通过；TLS hermetic各6/6，真实helperstep成功。SQLite fault5、PG source_guard1均实际执行。01/03受核生产模块相对adc0227无变更，共享CI与cli_color增量已由新run覆盖。旧41669982375仅作历史。

动态51passed的原runner因HEAD改变确实exit1；独立final-check exit0证明唯一变化是测试type alias、动态输入相同、凭证扫描及自有资源清理成功，不改写原runner结果。本轮仅核证和文档复签，没有运行Cargo、服务或提交。历史Claude Code同向意见保留为历史NON-AUTHORITATIVE依据，未新增沿用旧提示协议的第二意见。

## Goal Achievement

### Observable Truths

保留既有五条 ROADMAP SC 与三个 gap-closure truth。PLAN细项由后续 coverage 表映射，不缩减SC。

| # | Truth | Status | 当前实现与行为证据 |
|---|---|---|---|
| 1 | SC1：强类型 object/M:N edit表、reload新增DDL、破坏性变更拒绝并可migrate归档、失败保留旧定义 | VERIFIED | PhysicalSchema/ddl/diff → reload definition lock → prepare/role check → additive commit → probe → publish；migrate锁内复核role再执行。CI PostgreSQL reload 11/11、migrate 4/4，含 lock_timeout保旧、pending保旧、并发版本栅栏；SQLite对应tests通过 |
| 2 | SC2：SQLite/PostgreSQL共享conformance、log/history仅追加 | VERIFIED | 两后端调用相同conformance宏；commit同一事务append header/entry/history；真实PG45/45包含append_only_update/delete/truncate、log/history与replay；Windows SQLite43/44。ignored不当作pass |
| 3 | SC3：set_mask、tombstone、release、版本单调由独立model proptest覆盖 | VERIFIED | model_prop逐步比较 Model.apply 与真实store.commit的错误类别、receipt版本、每行，并重放log至新store；显式assert版本单调、release清空、tombstone仅release退出。windows.log:3920 `model_matches_store` 1/1 |
| 4 | SC4：角色类型/列/行/action/SQL/toolset，限同行/principal.attrs；缺属性不可见，多role不泄露 | VERIFIED | compile_scope验证引用；merge对每列OR仅授予该列role的谓词；evaluate要求Tri::True。scope_compile21、scope_merge9、rowpred eval/subset，含 `a_columns_times_b_rows_do_not_leak` 通过 |
| 5 | SC5：CEL SQL/Rust差分、criteria在reload/role变更检查、OIDC与dev认证 | VERIFIED | lower真实SQL经SQLite/Trino执行对比eval；Trino3/3；criteria coverage whole-object检查全列；reload、scope apply、migrate共用compile_scope；OIDC31、dev_token16（Windows）等通过 |
| 6 | 全新checkout示例dev配置可运行 | VERIFIED | EditStore::open创建父目录，DevKey创建目录（Unix0700）；CLI fresh-tree reload/dev-token以及scope plan tests实际通过，文件与值断言完整 |
| 7 | decimal不经f64舍入，行/log/history保真 | VERIFIED | value::from_json_as仅收decimal string或is_i64/is_u64整数；fraction拒绝。共享conformance用12345678.1234567891逐处比较，真实PG every_value_type_round_trips通过；import非法fraction整批不写入 |
| 8 | 开启Trino metadata cache时，必须flush并见新列后才切换 | VERIFIED | NoFlush只跳过flush仍真实describe；预热v1，断言NotVisible、active v1，真实flush后v2和gate可读。integration.log:1115-1119该用例及reload_trino_it共3/3；旧红绿对照留在03-UAT |

**Score：8/8；behavior_unverified：0。** 每条状态转换/顺序不变量都有实际执行的具名测试，不以symbol存在代替行为证明。

### Required Artifacts / Key Links

本次对 23 份 PLAN 运行 canonical artifact/link检查：89 个 artifact声明全部通过；55 个 key-link声明中54个pattern通过。03-03的 `ddl.rs → sql_exit.rs` 直接pattern误报，经手工追踪真实路径通过，不计为缺实现。

| Artifact / 链路 | 状态 | 实质内容与调用 |
|---|---|---|
| physical/ddl/diff/index/naming | VERIFIED | ontology生成强类型schema、两种方言DDL、变更diff、索引与物理名；reload/migrate真实调用 |
| reload.rs → sql_exit.rs | WIRED | execute_steps/apply_step → execute_ddl；definition lock、lock_timeout、pending/probe/publish有真实事务顺序 |
| migrate CLI → apply_migration_probed | WIRED | CLI传Some(RolesCheck)；apply_in先lock_def_state再load_scope_in/compile_scope，随后执行DDL |
| commit/state/log/history | VERIFIED / FLOWING | 输入edit读DB当前行与版本，事务写patch/log/history；不可见history在查询log前拒绝，其余真实query后按scope遮蔽 |
| auth scope compile/merge/lower | VERIFIED | Role/RoleBinding从真实定义与store读入；PrincipalScope参与criteria与history访问；无静态授权替身 |
| authn OIDC/JWKS/dev | VERIFIED | OIDC验证签名与claims、缓存JWKS；dev仅with_dev显式构造后启用，from_config dev=None |
| CLI reload/migrate/scope/import/dev-token | WIRED | 入口调用真实库；import限制editOnly并通过commit记录system:import |
| CI → PG/Trino | WIRED | PostgreSQL env由query-services设置，固定rowpred corpus seed，ignored suite显式执行且有非零发现守卫；Trino cache fixture 10m/missing=true/AS_ARRAY |

### Data-Flow Trace（含后继真实接线）

| 值 | 来源与消费者 | 状态 |
|---|---|---|
| 当前definition/scope | request_snapshot同一metadata lock事务读active与scope，commit/rollback后返回Arc数据；server context::build据此compile_scope | FLOWING |
| history响应 | history_with_snapshot读取真实header/entry，Admin才查params；visible_columns用当前行与调用者scope遮蔽；server history_page传请求固定active | FLOWING |
| row visibility | CEL编译Pred；Rust eval与实际SQL数据库结果对比；Unknown不为True | FLOWING |
| log/history/version | commit真实行更新后append immutable record/snapshot并返回receipt；失败rollback | FLOWING |

`postgres_http_it.rs:130` 的 `pg_http_mixed_snapshot_reload_revoke_and_history` 本轮读了完整断言：暂停真实Trino边界、外部reload/revoke需在5秒内完成、旧请求返回旧权限、新请求404、metadata新版本、read audit保留旧def_version。services.log:611实际通过，证明后继固定请求版本的演进仍接到Phase03存储原语。历史遮蔽另有history_masking及HTTP/MCP分页测试，本测试名称含history并不单独证明所有history功能。

### Requirements / PLAN Coverage

| Requirement | PLAN | 实现 / 实际测试 | 结论 |
|---|---|---|---|
| STORE-01 | 01/03/04/08/09/13/20/21 | 两后端同conformance；PG45，SQLite43；值域校验与decimal | SATISFIED |
| STORE-02 | 02/03/04/10/15/18/20 | reload/migrate与CLI；PG reload11/migrate4、Trino3；旧定义保持/栅栏/归档 | SATISFIED |
| STORE-03 | 08/09/21 | commit/log/history、append-only触发器、replay；实际PG负向SQL拒绝 | SATISFIED |
| STORE-04 | 17 | history_masking 12 SQLite+1 PG；冻结definition读取和当前scope遮蔽 | SATISFIED |
| STORE-05 | 10/13 + Phase7.1 | edit侧1:1部分唯一索引、commit基数校验、并发FK竞争仅一胜已有证据；source报告缺失，归Phase7.1补齐 | PARTIAL（全需求）；edit子合同通过 |
| STORE-06 | 19/21 | import真实JSON/CSV→canonical→commit；editOnly、create-only、system:import | SATISFIED |
| STORE-07 | 08/13 | state/reference model/proptest + backend conformance | SATISFIED |
| SCOPE-01 | 06/07/14/16 | Role/RoleBinding加载、scope plan/apply、user/group/JWT groups、管理审计 | SATISFIED |
| SCOPE-02 | 02/05/06/11/14/16 | schema scope_documents16、compile21、rowpred subset/eval | SATISFIED |
| SCOPE-03 | 11 | per-column predicate merge、9 merge tests及SQL差分 | SATISFIED |
| SCOPE-04 | 05/11/20 | SQLite diff6、Trino corpus/seeded3 | SATISFIED |
| SCOPE-05 | 05/11/14 | no_grant不可见、缺属性Unknown不通过、非法谓词编译拒绝 | SATISFIED |
| SCOPE-06 | 14/16/18/22 | criteria整对象全列、reload/scope/migrate共用compile；PG migrate拒绝保持旧版本 | SATISFIED |
| SCOPE-07 | 16/18 | 管理写入锁与审计；后继request_snapshot证据补足演进；MCP工具限制仍属Phase07合同 | SATISFIED（Phase03范围） |
| AUTH-01 | 01/02/07 | OIDC31、JWKS取回/缓存/错误路径 | SATISFIED |
| AUTH-02 | 12/23 | dev16、CLI12、fresh-tree与Unix权限断言 | SATISFIED |

16项均有PLAN归属，无orphan。PLAN01依赖方向/供应链、PLAN02定义与server config、PLAN10索引、PLAN20部署文档等细项已核代码/配置，不能仅用aggregate测试数量代替其接线。

### Behavioral Evidence / Test Quality

本 verifier 没有运行Cargo、PG或Trino。以下为本轮亲读原始日志后的限定复用，代码比较证明Phase03功能源码与被测冻结版本相同：

| 测试 / 命令范围 | 原始运行结果 |
|---|---|
| CI41669982375 Windows workspace step | 1490 passed / 0 failed / 117 ignored；fmt/clippy成功 |
| PostgreSQL conformance/history/import/migrate/reload/scope | 45 + 1 + 2 + 4 + 11 + 3 passed，全部0 failed |
| Trino rowpred / reload | 3 + 3 passed，0 ignored |
| model_prop | model_matches_store 1 passed；独立model逐步骤断言 |
| scope_merge / criteria / OIDC / dev | 分别9 / 6 / 31 / 16通过（Windows）；其测试源码断言真实拒绝/值/状态 |
| 后继PG HTTP snapshot/revoke | services.log中具名test通过；代码验证旧请求与新请求分离 |

PG/Trino ignored是外部依赖分组，CI明确--ignored且有发现数量守卫，不能在Windows默认suite里计作passed。独立model不调用生产state函数求expected；SQL differential在真实数据库执行，固定语料外另有seeded/proptest；没有发现循环expected生成。无未引用TBD/FIXME/XXX。

### Prohibitions / CI guard

- dev token：`Authenticator::from_config` 默认dev=None，配置独自启用被拒绝，具名negative tests通过。
- SQLite行谓词不得更宽：真实SQLite/Trino/Rust差分，含大小写、百分号/下划线字面、NULL/缺属性。
- import不得写source-backed、不得默认覆盖：源码检查backing与on-existing，negative tests实际通过。
- 禁openssl/native-tls：冻结CI原始log明确两包 `did not match any packages`，Cargo.lock与冻结版本相同。旧 `! cargo tree` 守卫会把查询失败当成功，是本轮明确发现并修复的CI门禁缺陷；当前 helper 单次完整图、精确包名匹配、查询异常fail-closed；本 verifier 重跑 hermetic 6/6（0.439s）、bash -n / diff-check均exit0，主会话真实helper日志为 tls_real_exit=0。测试从真实workflow提取命令，断言完整参数和单次调用，构成已接线enforcement。新CI41759804733已经双平台实际执行新增step，旧run不承担该证明。

### Decision Coverage / Probe / Human Verification

Decision coverage本次18/18 honored，无not_honored。无Phase03声明的probe shell，N/A。03-UAT两项已pass：真实缓存隔离红绿与旧CI平台验收；本次不伪称再启动服务。基础设施需求均有程序化行为证据，人工队列为空。

### Deferred / Advisory

SCOPE-07的MCP无变更工具由Phase07验收；其当前INT-01与本报告不相互替代。定义版本固定的后继路径已经亲核，但本报告不会据此把Phase07整体改成passed。

保留历史advisory：既有dev key文件权限/符号链接加固、部分配置类型错误可能回显拒绝值、公共apply_migration调用者需主动传role check、decimal诊断空格。旧Windows SQLite race曾首跑database is locked、重跑通过；冻结CI同名测试通过只证明该次执行，不宣称flaky风险消失。旧TLS guard问题另行修复，不再沿用为“安全通过”的证据。没有新增未经可复现实证就升级的BLOCKER。

### 历史与fingerprint边界

旧2026-10-22报告8/8、CI39710470956与03-UAT仍保留为历史来源；本轮behavior依据已推进至41669982375。03-VALIDATION现为validated：55task=50可执行COVERED+5静态SKIP，G2闭合；STORE-05全需求source报告缺失，nyquist_compliant:false。

fingerprint自动纳入本phase PLAN/SUMMARY，另显式纳入storage/auth/expr的真实实现与tests、相关schema/engine/CLI边界、Cargo依赖与CI/服务fixture、UAT/CONTEXT/ADR及部署文档。全局STATE/ROADMAP/REQUIREMENTS仅作状态digest不入指纹；需求合同已在报告列出。新HEAD四必需CI job已经success；INT-01 transport由Phase07独立验收。

---

_Verified: 2026-11-04T02:55:02Z_
_Verifier: gsd-verifier_
