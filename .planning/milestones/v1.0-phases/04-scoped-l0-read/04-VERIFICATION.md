---
phase: 04-scoped-l0-read
verified: 2026-11-05T11:05:30.233Z
source_head: 060019fe52005c6b85ae5c5f59da9868f464e579
ci_run: 41994479295
status: passed
score: 36/36 must-haves verified
covered_files:
  - ".github/workflows/ci.yml"
  - ".planning/REQUIREMENTS.md"
  - ".planning/ROADMAP.md"
  - ".planning/adr/0020-query-execution-rust-orchestrated-overlay.md"
  - ".planning/milestones/v1.0-audit/evidence/2026-11-05/ci-41994479295-final.json"
  - ".planning/milestones/v1.0-audit/evidence/2026-11-05/ci-41994479295-integration.log"
  - ".planning/milestones/v1.0-audit/evidence/2026-11-05/ci-41994479295-isolated-query-bench.log"
  - ".planning/milestones/v1.0-audit/evidence/2026-11-05/ci-41994479295-phase7-real-services.log"
  - ".planning/milestones/v1.0-audit/evidence/2026-11-05/ci-41994479295-ubuntu.log"
  - ".planning/milestones/v1.0-audit/evidence/2026-11-05/ci-41994479295-windows.log"
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
  - ".planning/phases/03-edit-store-scope-model/03-VALIDATION.md"
  - ".planning/phases/04-scoped-l0-read/04-01-PLAN.md"
  - ".planning/phases/04-scoped-l0-read/04-01-SUMMARY.md"
  - ".planning/phases/04-scoped-l0-read/04-02-PLAN.md"
  - ".planning/phases/04-scoped-l0-read/04-02-SUMMARY.md"
  - ".planning/phases/04-scoped-l0-read/04-03-PLAN.md"
  - ".planning/phases/04-scoped-l0-read/04-03-SUMMARY.md"
  - ".planning/phases/04-scoped-l0-read/04-04-PLAN.md"
  - ".planning/phases/04-scoped-l0-read/04-04-SUMMARY.md"
  - ".planning/phases/04-scoped-l0-read/04-05-PLAN.md"
  - ".planning/phases/04-scoped-l0-read/04-05-SUMMARY.md"
  - ".planning/phases/04-scoped-l0-read/04-06-PLAN.md"
  - ".planning/phases/04-scoped-l0-read/04-06-SUMMARY.md"
  - ".planning/phases/04-scoped-l0-read/04-07-PLAN.md"
  - ".planning/phases/04-scoped-l0-read/04-07-SUMMARY.md"
  - ".planning/phases/04-scoped-l0-read/04-VALIDATION.md"
  - "Cargo.lock"
  - "Cargo.toml"
  - "ci/check-tls-deps.sh"
  - "ci/ontoforge-server.ci.toml"
  - "ci/pg/iceberg_catalog.sql"
  - "ci/pg/query_corpus.sql"
  - "ci/pg/rowpred_corpus.sql"
  - "ci/pg/seed.sql"
  - "ci/query-services.sh"
  - "ci/release-package.ps1"
  - "ci/release-smoke.ps1"
  - "ci/server-services.sh"
  - "ci/test_tls_deps.py"
  - "ci/test_windows_server.py"
  - "ci/trino/catalog/demo.properties"
  - "ci/trino/catalog/editstore.properties"
  - "ci/trino/catalog/tpch.properties"
  - "ci/trino/iceberg/iceberg.properties"
  - "ci/trino/l2/access-control.properties"
  - "ci/trino/l2/config.properties"
  - "ci/trino/l2/rules.json"
  - "ci/trino/l2/seed.sql"
  - "ci/windows-server.py"
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
  - "crates/ontoforge-engine/Cargo.toml"
  - "crates/ontoforge-engine/src/bind_check.rs"
  - "crates/ontoforge-engine/src/bind_check/compat_tests.rs"
  - "crates/ontoforge-engine/src/bind_check/tests.rs"
  - "crates/ontoforge-engine/src/config.rs"
  - "crates/ontoforge-engine/src/error.rs"
  - "crates/ontoforge-engine/src/lib.rs"
  - "crates/ontoforge-engine/src/pk_check.rs"
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
  - "crates/ontoforge-engine/src/sqlite/fixture.rs"
  - "crates/ontoforge-engine/src/sqlite/mod.rs"
  - "crates/ontoforge-engine/src/sqlite/tests.rs"
  - "crates/ontoforge-engine/src/trino/cancel.rs"
  - "crates/ontoforge-engine/src/trino/classify.rs"
  - "crates/ontoforge-engine/src/trino/client.rs"
  - "crates/ontoforge-engine/src/trino/decode.rs"
  - "crates/ontoforge-engine/src/trino/headers.rs"
  - "crates/ontoforge-engine/src/trino/l2_health.rs"
  - "crates/ontoforge-engine/src/trino/l2_monitor.rs"
  - "crates/ontoforge-engine/src/trino/mod.rs"
  - "crates/ontoforge-engine/src/value.rs"
  - "crates/ontoforge-engine/tests/bind_check_isolation.rs"
  - "crates/ontoforge-engine/tests/data/l2_corpus.json"
  - "crates/ontoforge-engine/tests/data/test-ca.pem"
  - "crates/ontoforge-engine/tests/doc_type_table.rs"
  - "crates/ontoforge-engine/tests/drift.rs"
  - "crates/ontoforge-engine/tests/drift_budget.rs"
  - "crates/ontoforge-engine/tests/l2_explain.rs"
  - "crates/ontoforge-engine/tests/l2_gate.rs"
  - "crates/ontoforge-engine/tests/l2_health.rs"
  - "crates/ontoforge-engine/tests/l2_monitor.rs"
  - "crates/ontoforge-engine/tests/pk_check.rs"
  - "crates/ontoforge-engine/tests/printer_literals.rs"
  - "crates/ontoforge-engine/tests/printer_prop.proptest-regressions"
  - "crates/ontoforge-engine/tests/printer_prop.rs"
  - "crates/ontoforge-engine/tests/server_config.rs"
  - "crates/ontoforge-engine/tests/snapshots/pk_check__pk_duplicates_sqlite.snap"
  - "crates/ontoforge-engine/tests/snapshots/pk_check__pk_duplicates_trino.snap"
  - "crates/ontoforge-engine/tests/snapshots/pk_check__pk_nulls_sqlite.snap"
  - "crates/ontoforge-engine/tests/snapshots/pk_check__pk_nulls_trino.snap"
  - "crates/ontoforge-engine/tests/sql_text_only_from_printer.rs"
  - "crates/ontoforge-engine/tests/sqlite_engine.rs"
  - "crates/ontoforge-engine/tests/trino_client.rs"
  - "crates/ontoforge-engine/tests/trino_it.rs"
  - "crates/ontoforge-engine/tests/trino_l2_acl_it.rs"
  - "crates/ontoforge-engine/tests/trino_resilience.rs"
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
  - "crates/ontoforge-query/Cargo.toml"
  - "crates/ontoforge-query/src/aggregate.rs"
  - "crates/ontoforge-query/src/ast.rs"
  - "crates/ontoforge-query/src/compile.rs"
  - "crates/ontoforge-query/src/cursor.rs"
  - "crates/ontoforge-query/src/execute.rs"
  - "crates/ontoforge-query/src/l2.rs"
  - "crates/ontoforge-query/src/lib.rs"
  - "crates/ontoforge-query/src/link.rs"
  - "crates/ontoforge-query/src/overlay.rs"
  - "crates/ontoforge-query/src/set.rs"
  - "crates/ontoforge-query/tests/aggregate.rs"
  - "crates/ontoforge-query/tests/audit_l0.rs"
  - "crates/ontoforge-query/tests/audit_pk.rs"
  - "crates/ontoforge-query/tests/definition_snapshot.rs"
  - "crates/ontoforge-query/tests/l2_discovery.rs"
  - "crates/ontoforge-query/tests/l2_execute.rs"
  - "crates/ontoforge-query/tests/overlay_prop.proptest-regressions"
  - "crates/ontoforge-query/tests/overlay_prop.rs"
  - "crates/ontoforge-query/tests/pk_read.rs"
  - "crates/ontoforge-query/tests/query_bench_it.rs"
  - "crates/ontoforge-query/tests/search_around.rs"
  - "crates/ontoforge-query/tests/security.rs"
  - "crates/ontoforge-query/tests/set_ops.rs"
  - "crates/ontoforge-query/tests/sql_shape.rs"
  - "crates/ontoforge-query/tests/sqlite_query.rs"
  - "crates/ontoforge-query/tests/trino_query_it.rs"
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
  - "crates/ontoforge-storage/tests/query_read.rs"
  - "crates/ontoforge-storage/tests/query_read_pg.rs"
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
covered_digest: "v4:sha256:3681e160db363f03ba5c016e516d8c1f23d77e5e89f803b33a2d960dc3cf497f"
behavior_unverified: 0
overrides_applied: 0
decision_coverage: {"honored":7,"total":7,"not_honored":[]}
re_verification:
  previous_status: passed
  previous_score: 36/36
  trigger: "最终HEAD的原完整合同重新核验；不是仅刷新fingerprint"
  gaps_closed: ["native stale的8个moved文件已逐项审计并映射最终真实日志"]
  gaps_remaining: []
  regressions: []
human_verification: []
---

# Phase 4: Scoped L0 Read 验证报告

**Phase Goal:** As a 调用者, I want to 仅在自己的 scope 内通过 pk、过滤、排序分页、searchAround、集合运算与聚合读取对象，由 Rust 编排叠加且 Trino 只承担单表查询, so that 各种读取方式都遵守一致的授权边界.

**Status:** passed；**Score:** 36/36；**Re-verification:** 是，完整合同复签。旧原文已按字节保存在证据目录，不以旧passed或SUMMARY作为行为证明。

## User Flow Coverage

| 用户步骤 | 预期结果 | 当前证据 | 判断 |
|---|---|---|---|
| 调用者按pk选择属性读取 | edited经PG；其余按需source；tombstone/越权不可探测 | 04-01；当前pk/security测试与PG/Trino具名corpus | VERIFIED |
| 过滤、搜索、排序并续页 | scope一致、HMAC上下文绑定、补取有界且无重复遗漏 | 04-02/03/04；property、SQLite、真实Trino/refill | VERIFIED |
| 关系/集合/聚合读取 | 正常授权hydration与独立reference等价 | 04-05/06；真实relationship/set/aggregate corpus | VERIFIED |
| 检验延迟和SQL形态 | 单表、strict EXPLAIN解析；isolated四路p95达标 | 04-07；最终isolated模式、5+40样本的具名测试与artifact上传 | VERIFIED |
| Outcome：各种读取方式遵守一致授权边界 | 真实输入输出保持scope/错误/版本/限额合同 | 下方36项逐项证据，不以全局CI绿替代 | VERIFIED |

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
| SC1 | 已编辑对象经 PG 直读返回，未编辑对象经 Trino 单表 pk 查询返回，tombstone 不可见、release 后回到 source；重复 source pk 返回 `AmbiguousObject` | VERIFIED | 04-01/03：edited_pk_uses_store_only、tombstone/release/ambiguity与PG/Trino corpus |
| SC2 | WhereClause 全部运算符、排序与 keyset 分页（`has_more` 判定结束，补取循环凑满页）在示例数据上正确，结果按 pk 去重；String 搜索按 token 匹配 | VERIFIED | 04-02/04：every_where_operator_validates_and_evaluates、pagination补取/cursor/property和真实corpus |
| SC3 | searchAround、集合运算可用；count / sum / avg 分解聚合与朴素叠加结果相等，min / max / distinct 按 ADR-0020 处理 | VERIFIED | 04-05/06：edge/BTreeSet/CSV独立reference，真实aggregate/relationship corpus |
| SC4 | 越权 conformance（换 ID、伪造主体属性、多 role 列 × 行交叉、谓词 / 聚合反推、令牌反推）全部被拒绝；L0 输入输出与错误不含物理名；结果超限带 `truncated` | VERIFIED | 04-01/02/04/06：pk_probe_is_indistinguishable、physical_names_never_escape、token/context/caps |
| SC5 | 编译器保证每条 Trino 查询单表、≤ 2 fragment；安静或隔离集群下 pk 读 p95 < 500ms、列表 p95 < 1s、聚合 p95 < 1.5s（私有 fixture 实测记录） | VERIFIED | 04-02/06/07：single-table shape、真实EXPLAIN、最终isolated四路具名性能测试 |

### PLAN must_haves 逐项结果

每条保留原文；source/artifact/链路详对应E组，完整具名测试、源码行号和全部actual execution在`truth-results.json`。不是按task完成率推断。SC是整体验收，PLAN细化包含额外行为，因此全部列入分母；旧03的8条汇总仅作别名映射，不再代替134条合同。

| ID | Truth | 判断 | 当前执行或静态证据 |
|---|---|---|---|
| 04-01.1 | 客户端按 pk 读取时先走 edit store 的无副作用只读 API；只有 source-backed 对象仍需 source 列时才发一条 typed `LIMIT 2` source 查询（D-02，QUERY-01） | VERIFIED | windows:2586 edited_pk_uses_store_only; windows:2596 unedited_pk_uses_single_source_query；E04-01 |
| 04-01.2 | tombstone 与越权对象都表现为 `NotFound`，release 回到 source，重复 source pk 表现为 `AmbiguousObject`；错误只含 apiName（D-03，QUERY-01/09） | VERIFIED | windows:2595 tombstone_is_not_found_and_release_reads_source; windows:2642 pk_probe_is_indistinguishable；E04-01 |
| 04-01.3 | pk 结果只投影调用者在该合并行上可见且请求的属性，并携带绑定 principal/type/pk/definition/source/edit 状态的 HMAC `_version`（D-03/D-06，QUERY-09） | VERIFIED | windows:2644 version_changes_with_principal_scope_definition_and_row; windows:2646 version_query_uses_format_tag_and_changes_with_scope；E04-01 |
| 04-01.4 | storage 读取不存在的 key 不插入 version 0 行、不加写锁、不改变 edit log 或定义状态（QUERY-08） | VERIFIED | windows:2589 missing_read_has_no_side_effect; windows:3623 get_batch_and_scan_preserve_rows_log_and_definition；E04-01 |
| 04-01.5 | `ExecutionLimits` 由可信 host 构造并在请求开始冻结；Phase 4 初始默认是32 backend calls、10,000 raw candidates、2,048累计backfill keys、64 MiB decoded bytes，wire不能提供或调高，零值/溢出拒绝构造（QUERY-09） | VERIFIED | windows:2587 execution_limits_phase4_defaults_are_exact; windows:2588 execution_limits_reject_zero_and_overflow；E04-01 |
| 04-02.1 | ObjectSet/WhereClause 是 serde wire contract，完整覆盖 eq/gt/gte/lt/lte/isNull/in/startsWith/containsAnyTerm/containsAllTerms/and/or/not，并在编译前按 ontology 类型检查（D-01，QUERY-02） | VERIFIED | windows:2678 every_where_operator_validates_and_evaluates；E04-02 |
| 04-02.2 | 用户谓词只能读取该行上被某个适用 role 授予的逻辑列；source 下推把 column grant 与 predicate 绑定，合并后仍用同一 Rust 语义复核，filter/sort 不产生 role 行×列交叉（D-03，QUERY-09） | VERIFIED | windows:2569 hidden_columns_never_reach_sort_filter_or_projection; windows:2667 scope_row_and_filter_are_anded_without_role_crossing；E04-02 |
| 04-02.3 | String token search 采用空白 term 的 any/all 匹配、无 relevance order；ASCII corpus 在 Rust/SQLite/Trino 上一致，无法安全下推的形态留给 bounded Rust recheck（D-05，QUERY-03/08） | VERIFIED | integration:1203 where_search_set_and_relationship_corpus；E04-02 |
| 04-02.4 | L0 source compiler 只能产生一个 `TableRef`、没有 JOIN/CTE/UNION/raw SQL；typed printer 支持 outer CASE mask 与 typed distributed EXPLAIN（D-01，QUERY-10） | VERIFIED | windows:2665 all_source_variants_have_one_table_and_logical_aliases; windows:2668 source_select_has_exactly_one_table；E04-02 |
| 04-03.1 | query crate 可按 logical type 与 canonical pk 批量读取 edit patches，单批硬上限 256，且 scan 使用 stable keyset、显式 limit，不做 offset/full-table materialization（D-02，QUERY-04） | VERIFIED | windows:3619 batch_accepts_256_keys_and_all_value_types; windows:3628 scan_uses_keyset_and_preserves_flags；E04-03 |
| 04-03.2 | M:N link edit 可按任一 endpoint 有界读取，返回 typed endpoint keys、deleted/version；FK 不在 storage 建第二份关系表（D-05，QUERY-05） | VERIFIED | windows:3624 link_fk_request_is_rejected_with_logical_name; windows:3625 link_from_or_to_endpoint_is_bounded_and_preserves_removed_edges；E04-03 |
| 04-03.3 | 所有 read APIs 在 SQLite/PG 都只执行 SELECT，不插 version 0 skeleton、不锁 FOR UPDATE、不改变 log/definition/table row count（QUERY-08） | VERIFIED | windows:3623 get_batch_and_scan_preserve_rows_log_and_definition; integration:1170 sqlite_and_postgres_object_reads_match；E04-03 |
| 04-03.4 | storage error/public structs 使用 logical names；physical schema/table/column 只在 crate-private query builder 与日志 source 中存在（QUERY-09） | VERIFIED | windows:3622 corrupt_property_error_uses_logical_name；E04-03 |
| 04-04.1 | source 与 edit candidates 并行取得后，由唯一 Rust production overlay 合并、去重、scope/recheck、排序，并以最多 256 keys 的 backfill/refill 填页（D-01/D-02，QUERY-04/08） | VERIFIED | windows:2573 overlay_matches_independent_reference; windows:2613 edge_overlay_matches_independent_reference; windows:2630 pagination_backfill_batches_never_exceed_256; windows:2638 pagination_refills_after_tombstone_and_patch_moves_row_into_filter；E04-04 |
| 04-04.2 | keyset cursor 按规范化排序值与 pk 恢复，HMAC 绑定 principal、scope fingerprint、definition、object type 与 ordering；上下文不一致或被篡改时 fail closed（D-06，QUERY-04/09） | VERIFIED | windows:2626 cursor_multikey_null_pages_have_no_duplicates_or_omissions; windows:2628 cursor_rejects_tamper_and_context_replay；E04-04 |
| 04-04.3 | 分页继续补取到 page 满或所有 candidate lane 耗尽；work budget 耗尽返回 typed explicit error，不把不完整结果伪装成正常页（D-02，QUERY-09） | VERIFIED | windows:2632 pagination_budget_exhaustion_returns_no_partial_page; windows:2636 pagination_refills_across_three_false_positive_windows；E04-04 |
| 04-04.4 | `has_more` 由经 overlay/scope/recheck 后实际存在下一行决定；`truncated` 只表示最终 row/byte cap，不表示 refill 或 budget 状态（D-06，QUERY-04/09） | VERIFIED | windows:2629 final_row_and_byte_caps_truncate; windows:2640 pagination_stops_after_eligible_lookahead；E04-04 |
| 04-04.5 | 同一请求共享冻结的`ExecutionLimits`：32 backend calls、10,000 raw candidates、2,048累计backfill keys、64 MiB decoded bytes；逐维B可成功、B+1无partial失败，256仍是单批keys上限（QUERY-09） | VERIFIED | windows:2577 pagination_execution_limit_boundaries_are_inclusive; windows:2639 pagination_source_and_edit_share_all_work_budgets；E04-04 |
| 04-05.1 | FK searchAround 读取合并后对象的logical FK；M:N 将source join-table edges与PG link edits在Rust合并，Add/Remove与deleted语义遵守ADR-0024（D-05，QUERY-05） | VERIFIED | windows:2615 many_to_many_merges_source_and_link_edits_fixed; windows:2616 outbound_fk_uses_scoped_merged_object_value；E04-05 |
| 04-05.2 | 任何related target都通过正常的scoped pk/list read hydrate；隐藏、tombstone或目标row scope拒绝时不返回且不可由错误区分（D-03/D-05，QUERY-05/09） | VERIFIED | windows:2614 many_to_many_hydrates_257_targets_in_bounded_batches；E04-05 |
| 04-05.3 | union/intersect/subtract只对相同logical object type的已授权pk集合运算，随后复用正常overlay/projection/order/caps；集合代数不能成为scope或definition旁路（QUERY-06/09） | VERIFIED | windows:2656 set_result_reuses_normal_hydration_order_and_cursor; windows:2657 valid_same_type_set_and_cross_type_rejection；E04-05 |
| 04-05.4 | FK、M:N与set operations各有独立reference corpus，candidate重复、edit移动、multi-role与空集合保持确定、去重且有界（D-07，QUERY-08） | VERIFIED | windows:2613 edge_overlay_matches_independent_reference; windows:2654 set_ops_match_independent_btreeset_model; windows:2655 set_ops_match_independent_btreeset_model_fixed；E04-05 |
| 04-06.1 | count/sum/avg 用source partial加edited-key old/new contribution correction得到exact merged result；avg由sum+count组成，不能平均两个avg（D-04，QUERY-07） | VERIFIED | windows:2480 count_sum_avg_match_full_materialization_reference；E04-06 |
| 04-06.2 | min/max以typed single-table TopN source windows与全部edit candidates合并，持续补取直到证明winner或返回显式work/cost error；不能因首个candidate失效返回错误值（D-04，QUERY-07/10） | VERIFIED | windows:2482 min_max_edited_values_can_win_or_remove_winner; windows:2483 min_max_refill_past_invalid_top_rows；E04-06 |
| 04-06.3 | distinct与groupBy只使用scope后可见值；connector能力或预算不足以证明exact结果时返回typed explicit error，永不以`truncated`或partial buckets代替完成（D-03/D-04，QUERY-07/09） | VERIFIED | windows:2481 group_distinct_matches_visible_merged_reference；E04-06 |
| 04-06.4 | aggregate fixed/property corpus覆盖SQLite reference、multi-role row×column、null、edited membership/value/group移动与connector能力选择（D-07，QUERY-08） | VERIFIED | windows:2478 count_sum_avg_connector_capability_selects_supported_plan; windows:2480 count_sum_avg_match_full_materialization_reference；E04-06 |
| 04-06.5 | 可信host冻结的Phase 4初始`AggregateLimits`为1,024 groups、4,096 buckets、请求总计10,000 distinct values、16 MiB distinct memory；与`ExecutionLimits`共享同一请求budget，wire不可调高（QUERY-09） | VERIFIED | windows:2477 aggregate_wire_request_rejects_capabilities; windows:2479 count_sum_avg_limits_phase4_defaults_are_exact；E04-06 |
| 04-07.1 | 真实Linux CI在PostgreSQL 17 + Trino 483上实际执行storage `query_read_pg`与query `trino_query_it` ignored tests，且独立discovery guard证明每个target至少有一个ignored test（D-07，QUERY-08） | VERIFIED | integration:1169 sqlite_and_postgres_link_reads_match; integration:1170 sqlite_and_postgres_object_reads_match; integration:1200 fixed_semantics_and_cursor_refill；E04-07 |
| 04-07.2 | 每种source-producing query shape都对真实Trino执行`EXPLAIN (TYPE DISTRIBUTED)`，parser必须识别至少1个且不超过2个fragment；无法解析也失败（D-01，QUERY-10） | VERIFIED | windows:2605 truncated_distributed_explain_is_rejected_before_fragment_count; integration:1199 aggregate_corpus_and_distributed_explain；E04-07 |
| 04-07.3 | 真实backend corpus覆盖multi-role、overlay、cursor、traversal、set、aggregate与physical-error消毒，SQLite expected与Trino/PG结果逐项一致（QUERY-08/09） | VERIFIED | integration:1200 fixed_semantics_and_cursor_refill; integration:1201 real_physical_errors_are_sanitized; integration:1202 real_security_roles_and_cursor_replay; integration:1203 where_search_set_and_relationship_corpus；E04-07 |
| 04-07.4 | 专用无并行负载的Linux CI run以固定dataset、至少5次warmup和40次measured samples证明pk/list/aggregate/edited-PG-direct p95预算，并保存raw timings、plan、环境与run URL（D-07，QUERY-10） | VERIFIED | integration:1230 phase4_real_backend_p95；E04-07 |

**Score:** 36/36；behavior_unverified=0。所有状态转换、refill/cap/cursor不变量均有当前具名测试执行。

### Required Artifacts / Key Links

原生检查在`04-contracts.json`。24/26 artifact pattern、18/19 link pattern通过。read_object_edit与execute_list是impl内方法，公开定义及实际调用存在，自动export matcher漏报；set的旧函数名pattern已改为authorized_keys→execute_list_internal，实际hydrate与测试均在用。三处均人工追踪消解，不是静默标自动PASS。

| E组 | 实现与接线 | 判断 |
|---|---|---|
| E04-01 | QueryService::execute→read_object_edit→按需LIMIT 2 source→overlay→TypeScope.evaluate→投影/TokenCodec；只读store API。 crates/ontoforge-storage/src/query_read.rs；crates/ontoforge-query/src/lib.rs；crates/ontoforge-query/tests/pk_read.rs | substantive / WIRED |
| E04-02 | wire ast→compile类型与grant校验→typed single TableRef；Rust merged-row复核，token any/all不做相关度。 crates/ontoforge-query/src/ast.rs；crates/ontoforge-query/src/compile.rs；crates/ontoforge-engine/src/printer/ast.rs；crates/ontoforge-query/tests/sql_shape.rs | substantive / WIRED |
| E04-03 | query_read typed SELECT，≤256 batch与stable keyset scan；FK无额外关系表，PG/SQLite逐值比。 crates/ontoforge-storage/src/query_read.rs；crates/ontoforge-storage/tests/query_read.rs；crates/ontoforge-storage/tests/query_read_pg.rs | substantive / WIRED |
| E04-04 | execute并行候选→overlay去重→backfill/refill→scope/recheck→lookahead/cap；cursor HMAC和共享work budget。 crates/ontoforge-query/src/overlay.rs；crates/ontoforge-query/src/execute.rs；crates/ontoforge-query/src/cursor.rs；crates/ontoforge-query/tests/overlay_prop.rs | substantive / WIRED |
| E04-05 | link source edges+edits→authorized target keys→normal execute_list_internal hydration；set同一路径。 crates/ontoforge-query/src/link.rs；crates/ontoforge-query/src/set.rs；crates/ontoforge-query/tests/search_around.rs；crates/ontoforge-query/tests/set_ops.rs | substantive / WIRED |
| E04-06 | partial sum/count old/new correction；TopN补取；exact distinct/group与typed error；独立CSV/BTreeSet/reference模型。 crates/ontoforge-query/src/aggregate.rs；crates/ontoforge-query/tests/aggregate.rs；crates/ontoforge-query/tests/sql_shape.rs；crates/ontoforge-query/tests/security.rs | substantive / WIRED |
| E04-07 | 当前integration环境/非零discovery/具名真实语义+EXPLAIN；独立isolated job执行四路5+40样本并上传raw artifact。 crates/ontoforge-query/tests/trino_query_it.rs；crates/ontoforge-query/tests/query_bench_it.rs；.github/workflows/ci.yml；.planning/phases/04-scoped-l0-read/04-PERFORMANCE.md | substantive / WIRED |

### Data-Flow Trace (Level 4)

| 数据 | 真实来源与消费者 | 判断 |
|---|---|---|
| pk/_version | lib.rs:307读真实edit，按需typed source LIMIT2，合并/evaluate/投影再TokenCodec | FLOWING |
| list/cursor/has_more | source typed SELECT与edit keyset→overlay→bounded backfill/refill→scope/recheck→lookahead/cap | FLOWING |
| related/set | scoped FK/edge/source/edit→authorized_keys→execute_list_internal正常hydration | FLOWING |
| aggregate | 真实source partial/TopN与edit old/new correction→exact scoped value或typed error | FLOWING |

### Behavioral Spot-Checks / Probe Execution

本轮没有新执行测试，以下是独立读取既有真实命令及对应actual输出；不冒充新跑named test。没有PLAN声明的probe-*.sh，probe N/A。

| 合同 | 既有真实执行与结果 | 判断 |
|---|---|---|
| 03存储/授权基础（04的前置边界） | I:567 PG环境；PG conformance45、reload11、migrate4、scope3、history1、import2 actual pass，均0 failed/0 ignored | VERIFIED（各测试仅证明各自合同） |
| CEL与缓存可见性 | I:1105-1106/1135-1136显式两URL；1116-1118三项Trino差分、1144缓存隔离actual ok | VERIFIED |
| 当前scoped读与PG替身一致 | I:1162/1187-1188环境；query_read_pg 2和trino_query_it 5实际通过，1199-1203含EXPLAIN/安全/关系/集合 | VERIFIED |
| 后继snapshot/锁释放 | S:603/631/643具名actual ok；server-services require_env并启动真实后端，不是缺env return | VERIFIED（边界原语） |
| 隔离性能 | B:601精确cargo query_bench_it命令；613-616注入PG/Trino/isolated/artifact dir；四路p95 107.648791/472.588841/407.732135/1.127974ms，严格低于500/1000/1500/50 | VERIFIED（记录环境） |

真实环境与早退核查详`service-environment-evidence.json`及`GOVERNANCE-AND-BOUNDARIES.md`。rowpred/trino_query/reload_trino/query_bench的required helpers缺env均panic；PG support缺两种来源时失败。PLAN03-20的两个延后human-check仅要求看CI步骤与非零计数，当前原始I日志逐目标满足并有actual run，已关闭，不声称再次人工视觉UAT。

性能源码`query_bench_it.rs:723`的isolated分支确实5warmup+40measured；:762测试写raw JSON再assert所有路径passed，逐样本error不得存在。B:1228确认artifact13400156425上传成功，zip SHA256 `bb4104a29e00af3a078f8718eb4e123a17657ee262951341efb341f720b66499`。本worker没有下载最新raw JSON或独立重算每个sample/fragment；本判断依据当前具名断言、actual mode/env/output/upload，不冒称重算，不沿用旧run的123/425/374ms。EXPLAIN解析器及真实corpus拒绝0、>2或截断fragment。结论只适用于记录的专用Linux、Trino483/PG17 fixture，不是任意生产集群SLA。

### Requirements Coverage

| Requirement | 原始描述 | 支持PLAN | 判断 |
|---|---|---|---|
| QUERY-01 | 客户端可以按 primary key 获取单个对象（已编辑对象 PG 直读，未编辑对象 Trino 单表查询），并可指定返回的属性子集 | 04-01 | SATISFIED（实现合同） |
| QUERY-08 | Rust 叠加实现对照内存参考模型的 proptest 通过，SQLite 替身与 Trino 的单表查询语义一致 | 04-01/04-02/04-03/04-04/04-05/04-06/04-07 | SATISFIED（实现合同） |
| QUERY-09 | 读结果带不透明的 HMAC `_version` 令牌；L0 输入、输出与错误不含物理名；结果按行数与字节上限截断并标 `truncated` | 04-01/04-02/04-03/04-04/04-05/04-06/04-07 | SATISFIED（实现合同） |
| QUERY-02 | 客户端可以用 WhereClause（eq / gt / gte / lt / lte / isNull / in / startsWith / containsAnyTerm / containsAllTerms / and / or / not）过滤对象 | 04-02 | SATISFIED（实现合同） |
| QUERY-03 | 客户端可以对 String 属性做 token 匹配搜索（Trino 上无相关度排序） | 04-02 | SATISFIED（实现合同） |
| QUERY-10 | 每条 Trino 查询单表、≤ 2 fragment；安静或隔离集群下 pk 读 p95 < 500ms、列表 p95 < 1s、聚合 p95 < 1.5s、已编辑对象 PG 直读 p95 < 50ms | 04-02/04-06/04-07 | SATISFIED（实现合同） |
| QUERY-04 | 客户端可以对结果排序，并用 keyset 游标分页遍历（`has_more` 判定结束） | 04-03/04-04 | SATISFIED（实现合同） |
| QUERY-05 | 客户端可以沿 link 遍历（searchAround），也可以获取单个对象的关联对象 | 04-03/04-05 | SATISFIED（实现合同） |
| QUERY-06 | 客户端可以对 object set 做 union / intersect / subtract | 04-05 | SATISFIED（实现合同） |
| QUERY-07 | 客户端可以对 object set 做聚合（count / sum / avg / min / max / distinct），支持 groupBy | 04-06 | SATISFIED（实现合同） |

全部10个ID在PLAN与REQUIREMENTS相符，无orphan。REQUIREMENTS checkbox/审计矩阵不是行为证据。STORE-05的source基数报告不属于本phase验收，本报告不替07.1复签或重算milestone。

### Prohibitions / Test Quality / Anti-Patterns

PLAN没有prohibitions项。测试expected来自独立overlay/edge/BTreeSet/CSV full-materialization reference，不由production输出倒推expected。

原覆盖407个不同文件全部存在；受核Rust源码无未引用TBD/FIXME/XXX或placeholder，未发现wired却静态空数据的路径。外部依赖ignored测试由专门CI目标actual执行，workspace ignored不作pass。状态初始空集合不是stub，真实query/store结果随后填充。静态pattern例外均已追接实际调用。没有可观察的缺实现、断链或新增BLOCKER；旧Windows race单次通过不等于消除flaky风险。

### Decision Coverage / Advisory

GSD native decision gate exit=0：7/7 honored，not_honored=[]，warning-only。本轮无新增未经确定证据支持的BLOCKER。保留历史advisory：dev-key Windows ACL/既有文件权限及symlink加固，公共apply_migration调用者须传RolesCheck的集成责任，既有SQLite race flaky。它们不作为本轮已修复事实。

### Human Verification Required

无未决项。已具名执行的基础库/CLI/真实后端语料覆盖本phase；没有新增UI、外观或私有生产fixture验收。

### Fingerprint / Covered Set Preservation

原生`query verification.fingerprint`输出原样用于frontmatter。Phase04原covered 181项、新209项，删除0项，原治理文件删除0项；原集合是新集合子集，未过滤.planning。完整差集与命令输出在`covered-set-diff.json`和`fingerprint-results.json`。原生moved 8项均在`changed-source-inspection.json`与治理证据表逐项解释。

本报告自身不进入指纹；没有把其他VERIFICATION报告加入依赖。最新gate结果另存`native-gate-results.json`，该结果文件不反向纳入指纹，避免自引用。ROADMAP/REQUIREMENTS与原全部治理文件保留，GSD自行处理planning-root inert语义，本worker没有手工过滤或拼造digest。

### Gaps Summary

36/36逐项具备实现、接线、真实数据流与具名执行证据，签passed。范围仅Phase04，不代表Phase03人工证据、07.1或整个v1.0已通过。

_Verified: 2026-11-05T11:05:30.233Z_
_Verifier: gsd-verifier_
