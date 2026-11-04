---
phase: 04-scoped-l0-read
verified: 2026-11-04T02:57:39Z
source_head: f8851ca073fed34e80e605f0ac91880259df7e12
status: passed
score: 36/36 must-haves verified
covered_files: [".github/workflows/ci.yml",".planning/phases/04-scoped-l0-read/04-01-PLAN.md",".planning/phases/04-scoped-l0-read/04-01-SUMMARY.md",".planning/phases/04-scoped-l0-read/04-02-PLAN.md",".planning/phases/04-scoped-l0-read/04-02-SUMMARY.md",".planning/phases/04-scoped-l0-read/04-03-PLAN.md",".planning/phases/04-scoped-l0-read/04-03-SUMMARY.md",".planning/phases/04-scoped-l0-read/04-04-PLAN.md",".planning/phases/04-scoped-l0-read/04-04-SUMMARY.md",".planning/phases/04-scoped-l0-read/04-05-PLAN.md",".planning/phases/04-scoped-l0-read/04-05-SUMMARY.md",".planning/phases/04-scoped-l0-read/04-06-PLAN.md",".planning/phases/04-scoped-l0-read/04-06-SUMMARY.md",".planning/phases/04-scoped-l0-read/04-07-PLAN.md",".planning/phases/04-scoped-l0-read/04-07-SUMMARY.md","Cargo.lock","Cargo.toml","ci/check-tls-deps.sh","ci/ontoforge-server.ci.toml","ci/pg/iceberg_catalog.sql","ci/pg/query_corpus.sql","ci/pg/rowpred_corpus.sql","ci/pg/seed.sql","ci/query-services.sh","ci/release-package.ps1","ci/release-smoke.ps1","ci/server-services.sh","ci/test_tls_deps.py","ci/test_windows_server.py","ci/trino/catalog/demo.properties","ci/trino/catalog/editstore.properties","ci/trino/catalog/tpch.properties","ci/trino/iceberg/iceberg.properties","ci/trino/l2/access-control.properties","ci/trino/l2/config.properties","ci/trino/l2/rules.json","ci/trino/l2/seed.sql","ci/windows-server.py","crates/ontoforge-auth/src/authn/dev.rs","crates/ontoforge-auth/src/authn/jwks.rs","crates/ontoforge-auth/src/authn/mod.rs","crates/ontoforge-auth/src/authn/oidc.rs","crates/ontoforge-auth/src/authn/principal.rs","crates/ontoforge-auth/src/lib.rs","crates/ontoforge-auth/src/scope/compile.rs","crates/ontoforge-auth/src/scope/coverage.rs","crates/ontoforge-auth/src/scope/lower.rs","crates/ontoforge-auth/src/scope/merge.rs","crates/ontoforge-auth/src/scope/mod.rs","crates/ontoforge-engine/Cargo.toml","crates/ontoforge-engine/src/bind_check.rs","crates/ontoforge-engine/src/bind_check/compat_tests.rs","crates/ontoforge-engine/src/bind_check/tests.rs","crates/ontoforge-engine/src/config.rs","crates/ontoforge-engine/src/error.rs","crates/ontoforge-engine/src/lib.rs","crates/ontoforge-engine/src/pk_check.rs","crates/ontoforge-engine/src/printer/ast.rs","crates/ontoforge-engine/src/printer/call.rs","crates/ontoforge-engine/src/printer/dialect.rs","crates/ontoforge-engine/src/printer/escape.rs","crates/ontoforge-engine/src/printer/l2.rs","crates/ontoforge-engine/src/printer/mod.rs","crates/ontoforge-engine/src/printer/snapshots/ontoforge_engine__printer__tests__print_sqlite.snap","crates/ontoforge-engine/src/printer/snapshots/ontoforge_engine__printer__tests__print_trino.snap","crates/ontoforge-engine/src/printer/sqlite_ddl.rs","crates/ontoforge-engine/src/printer/tests.rs","crates/ontoforge-engine/src/sqlite/fixture.rs","crates/ontoforge-engine/src/sqlite/mod.rs","crates/ontoforge-engine/src/sqlite/tests.rs","crates/ontoforge-engine/src/trino/cancel.rs","crates/ontoforge-engine/src/trino/classify.rs","crates/ontoforge-engine/src/trino/client.rs","crates/ontoforge-engine/src/trino/decode.rs","crates/ontoforge-engine/src/trino/headers.rs","crates/ontoforge-engine/src/trino/l2_health.rs","crates/ontoforge-engine/src/trino/l2_monitor.rs","crates/ontoforge-engine/src/trino/mod.rs","crates/ontoforge-engine/src/value.rs","crates/ontoforge-engine/tests/bind_check_isolation.rs","crates/ontoforge-engine/tests/data/l2_corpus.json","crates/ontoforge-engine/tests/data/test-ca.pem","crates/ontoforge-engine/tests/doc_type_table.rs","crates/ontoforge-engine/tests/drift.rs","crates/ontoforge-engine/tests/drift_budget.rs","crates/ontoforge-engine/tests/l2_explain.rs","crates/ontoforge-engine/tests/l2_gate.rs","crates/ontoforge-engine/tests/l2_health.rs","crates/ontoforge-engine/tests/l2_monitor.rs","crates/ontoforge-engine/tests/pk_check.rs","crates/ontoforge-engine/tests/printer_literals.rs","crates/ontoforge-engine/tests/printer_prop.proptest-regressions","crates/ontoforge-engine/tests/printer_prop.rs","crates/ontoforge-engine/tests/server_config.rs","crates/ontoforge-engine/tests/snapshots/pk_check__pk_duplicates_sqlite.snap","crates/ontoforge-engine/tests/snapshots/pk_check__pk_duplicates_trino.snap","crates/ontoforge-engine/tests/snapshots/pk_check__pk_nulls_sqlite.snap","crates/ontoforge-engine/tests/snapshots/pk_check__pk_nulls_trino.snap","crates/ontoforge-engine/tests/sql_text_only_from_printer.rs","crates/ontoforge-engine/tests/sqlite_engine.rs","crates/ontoforge-engine/tests/trino_client.rs","crates/ontoforge-engine/tests/trino_it.rs","crates/ontoforge-engine/tests/trino_l2_acl_it.rs","crates/ontoforge-engine/tests/trino_resilience.rs","crates/ontoforge-expr/src/analyze.rs","crates/ontoforge-expr/src/check.rs","crates/ontoforge-expr/src/infer.rs","crates/ontoforge-expr/src/lib.rs","crates/ontoforge-expr/src/limits.rs","crates/ontoforge-expr/src/rowpred/eval.rs","crates/ontoforge-expr/src/rowpred/ir.rs","crates/ontoforge-expr/src/rowpred/mod.rs","crates/ontoforge-expr/src/rowpred/parse.rs","crates/ontoforge-expr/src/rowpred/value.rs","crates/ontoforge-expr/src/walk.rs","crates/ontoforge-query/Cargo.toml","crates/ontoforge-query/src/aggregate.rs","crates/ontoforge-query/src/ast.rs","crates/ontoforge-query/src/compile.rs","crates/ontoforge-query/src/cursor.rs","crates/ontoforge-query/src/execute.rs","crates/ontoforge-query/src/l2.rs","crates/ontoforge-query/src/lib.rs","crates/ontoforge-query/src/link.rs","crates/ontoforge-query/src/overlay.rs","crates/ontoforge-query/src/set.rs","crates/ontoforge-query/tests/aggregate.rs","crates/ontoforge-query/tests/audit_l0.rs","crates/ontoforge-query/tests/audit_pk.rs","crates/ontoforge-query/tests/definition_snapshot.rs","crates/ontoforge-query/tests/l2_discovery.rs","crates/ontoforge-query/tests/l2_execute.rs","crates/ontoforge-query/tests/overlay_prop.proptest-regressions","crates/ontoforge-query/tests/overlay_prop.rs","crates/ontoforge-query/tests/pk_read.rs","crates/ontoforge-query/tests/query_bench_it.rs","crates/ontoforge-query/tests/search_around.rs","crates/ontoforge-query/tests/security.rs","crates/ontoforge-query/tests/set_ops.rs","crates/ontoforge-query/tests/sql_shape.rs","crates/ontoforge-query/tests/sqlite_query.rs","crates/ontoforge-query/tests/trino_query_it.rs","crates/ontoforge-storage/src/commit.rs","crates/ontoforge-storage/src/ddl.rs","crates/ontoforge-storage/src/diff.rs","crates/ontoforge-storage/src/error.rs","crates/ontoforge-storage/src/history.rs","crates/ontoforge-storage/src/import.rs","crates/ontoforge-storage/src/index.rs","crates/ontoforge-storage/src/lib.rs","crates/ontoforge-storage/src/log.rs","crates/ontoforge-storage/src/migrate.rs","crates/ontoforge-storage/src/naming.rs","crates/ontoforge-storage/src/physical.rs","crates/ontoforge-storage/src/query_read.rs","crates/ontoforge-storage/src/read_audit.rs","crates/ontoforge-storage/src/reload.rs","crates/ontoforge-storage/src/request_snapshot.rs","crates/ontoforge-storage/src/scope_store.rs","crates/ontoforge-storage/src/snapshots/ontoforge_storage__ddl__tests__ddl_postgres.snap","crates/ontoforge-storage/src/snapshots/ontoforge_storage__ddl__tests__ddl_sqlite.snap","crates/ontoforge-storage/src/sql_exit.rs","crates/ontoforge-storage/src/state.rs","crates/ontoforge-storage/src/store.rs","crates/ontoforge-storage/src/value.rs","crates/ontoforge-storage/tests/query_read.rs","crates/ontoforge-storage/tests/query_read_pg.rs","examples/ontologies/aviation/actions.yaml","examples/ontologies/aviation/bindings/dev/aircraft.yaml","examples/ontologies/aviation/bindings/dev/fixtures/aircraft.csv","examples/ontologies/aviation/bindings/dev/fixtures/flight_crew.csv","examples/ontologies/aviation/bindings/dev/fixtures/flights.csv","examples/ontologies/aviation/bindings/dev/fixtures/maintenance_records.csv","examples/ontologies/aviation/bindings/dev/fixtures/pilots.csv","examples/ontologies/aviation/bindings/dev/flight-crew.yaml","examples/ontologies/aviation/bindings/dev/flight.yaml","examples/ontologies/aviation/bindings/dev/maintenance-record.yaml","examples/ontologies/aviation/bindings/dev/pilot.yaml","examples/ontologies/aviation/links.yaml","examples/ontologies/aviation/objects.yaml","examples/ontologies/aviation/ontology.yaml"]
covered_digest: "v4:sha256:72554a02f7f3981ee40a92fcb8283255549b347138919a7ef7f6ad4d559d9179"
behavior_unverified: 0
overrides_applied: 0
decision_coverage: {honored: 7, total: 7, not_honored: []}
human_verification: []
re_verification:
  previous_status: passed
  previous_score: 36/36
  gaps_closed:
    - "当前源码的隔离 Linux p95、真实 Trino/PG 语义与分片数补证"
    - "MVP User Story 格式已等义规范并由 owner query 验证"
  gaps_remaining: []
  regressions: []
---

# Phase 4: Scoped L0 Read Verification Report

**Phase Goal:** As a 调用者, I want to 仅在自己的 scope 内通过 pk、过滤、排序分页、searchAround、集合运算与聚合读取对象，由 Rust 编排叠加且 Trino 只承担单表查询, so that 各种读取方式都遵守一致的授权边界.
**Status:** passed
**Re-verification:** 历史结论的当前源码复验。旧 SUMMARY/VERIFICATION 的 passed 不是本次行为证据。

## User Flow Coverage

| Step | Expected | Evidence | Status |
|---|---|---|---|
| 按 pk 读取 source / edited 对象 | 得到 scoped overlay，隐藏与删除不可探测 | U:2569-2579；I:1198 | VERIFIED |
| 过滤、搜索、排序并续页 | 同一授权边界、无重复遗漏、有界填页 | U:2609-2623、2661-2668；I:1198/1201 | VERIFIED |
| 读取关系、集合与聚合 | 结果等同独立参考模型且仅含有权数据 | U:2461-2466、2595-2600、2636-2640；I:1197/1201 | VERIFIED |
| 尝试跨 scope、篡改 token、探测物理信息 | 拒绝或不可区分错误 | U:2607-2629；I:1199/1200 | VERIFIED |
| 达到用户故事 outcome | 读取方式遵守一致授权边界 | 上述 caller-visible 语料穿过真实 Trino/PG 与 SQLite 路径 | VERIFIED |

`query user-story.validate --story <ROADMAP goal> --pick valid` 本次返回 `true`。本 phase 是库级读取能力，无新增 UI 外观检查。

## Evidence Provenance

本 verifier 只读核查现有源码、原始日志、artifact 和 GSD owner queries；本轮未重跑 Cargo/Docker、未启动服务、未声称 Warp/ralph-loop。当前冻结源码为 `f8851ca073fed34e80e605f0ac91880259df7e12`。

- **U**：`target/milestone-ci-41759804733-ubuntu.log`，以下 U:n 是原始日志行号。
- **W**：`target/milestone-ci-41759804733-windows.log`。
- **I**：`target/milestone-ci-41759804733-integration.log`。
- CI `41759804733` 的 head 为 f8851ca，Windows、Ubuntu、integration、phase7-real-services 必需 jobs 均 success；workspace、Clippy、fmt 真实执行。ignored 行不作测试通过证据。
- Benchmark run `41759457559` 的 **isolated-query-bench job success**；该 run 其他普通 test jobs 因旧 Clippy type complexity 失败，整个 run 为 failure。不得把独立性能 job 成功写成整个 run 成功。
- Benchmark 原始 SHA 为 `868643cb0d131b9a7f1fdf2a16a53d1ac445efe8`；本次完整 `git diff 868643c f8851ca` 仅 `crates/ontoforge/tests/cli_color.rs` 的 ColorCase 类型别名（2 additions/1 deletion），生产源码、CI、Cargo.lock、fixture 与性能输入均相同。f885 新测试由最终 CI 覆盖。
- 旧 `04-PERFORMANCE.md` 只描述历史源码；本报告以下当前 artifact 为新性能证据。当前产物不声称真实私有生产数据的延迟保证。

## Goal Achievement

### Observable Truths

5 条 roadmap SC 与31条 PLAN 细化 truth 全部保留；SC 是整体合同，PLAN 是细节，不以旧汇总省略。状态变更/取消/分页 invariant 的证据均包含实际 passing test。

| # | Truth | Status | Evidence |
|---|---|---|---|
| SC1 | 已编辑对象经 PG 直读返回，未编辑对象经 Trino 单表 pk 查询返回，tombstone 不可见、release 后回到 source；重复 source pk 返回 `AmbiguousObject` | VERIFIED | 04-01、04-03；U:2569/2577/2578/2579；I:1167-1168 与1198；benchmark edited路径45样本零source calls |
| SC2 | WhereClause 全部运算符、排序与 keyset 分页（`has_more` 判定结束，补取循环凑满页）在示例数据上正确，结果按 pk 去重；String 搜索按 token 匹配 | VERIFIED | 04-02、04-04；U:2661/2668 与2613-2623；I:1198/1201 |
| SC3 | searchAround、集合运算可用；count / sum / avg 分解聚合与朴素叠加结果相等，min / max / distinct 按 ADR-0020 处理 | VERIFIED | 04-05、04-06；U:2463-2466、2597-2599、2638-2640；I:1197/1201 |
| SC4 | 越权 conformance（换 ID、伪造主体属性、多 role 列 × 行交叉、谓词 / 聚合反推、令牌反推）全部被拒绝；L0 输入输出与错误不含物理名；结果超限带 `truncated` | VERIFIED | 04-01、04-02、04-04、04-06；U:2607-2629；I:1199/1200 |
| SC5 | 编译器保证每条 Trino 查询单表、≤ 2 fragment；安静或隔离集群下 pk 读 p95 < 500ms、列表 p95 < 1s、聚合 p95 < 1.5s（私有 fixture 实测记录） | VERIFIED | typed单表compiler + distributed EXPLAIN + isolated job；原始数据及重算见下表；公开合成航空fixture，未声称私有生产集群性能 |
| 04-01.1 | 客户端按 pk 读取时先走 edit store 的无副作用只读 API；只有 source-backed 对象仍需 source 列时才发一条 typed `LIMIT 2` source 查询（D-02，QUERY-01） | VERIFIED | query/lib.rs::execute / read_object_edit；U:2569 edited_pk_uses_store_only、U:2579 unedited_pk_uses_single_source_query、U:2577 LIMIT 2/重复 pk |
| 04-01.2 | tombstone 与越权对象都表现为 `NotFound`，release 回到 source，重复 source pk 表现为 `AmbiguousObject`；错误只含 apiName（D-03，QUERY-01/09） | VERIFIED | query/lib.rs overlay；U:2578 tombstone_is_not_found_and_release_reads_source、U:2625 pk_probe_is_indistinguishable |
| 04-01.3 | pk 结果只投影调用者在该合并行上可见且请求的属性，并携带绑定 principal/type/pk/definition/source/edit 状态的 HMAC `_version`（D-03/D-06，QUERY-09） | VERIFIED | query/lib.rs::TokenCodec 与 scope projection；U:2627 version_changes_with_principal_scope_definition_and_row、U:2629 version_query_uses_format_tag_and_changes_with_scope |
| 04-01.4 | storage 读取不存在的 key 不插入 version 0 行、不加写锁、不改变 edit log 或定义状态（QUERY-08） | VERIFIED | storage/query_read.rs SELECT-only；U:2572 missing_read_has_no_side_effect、U:3608 get_batch_and_scan_preserve_rows_log_and_definition |
| 04-01.5 | `ExecutionLimits` 由可信 host 构造并在请求开始冻结；Phase 4 初始默认是32 backend calls、10,000 raw candidates、2,048累计backfill keys、64 MiB decoded bytes，wire不能提供或调高，零值/溢出拒绝构造（QUERY-09） | VERIFIED | query/lib.rs::ExecutionLimits / frozen QueryService；U:2570 execution_limits_phase4_defaults_are_exact、U:2571 execution_limits_reject_zero_and_overflow |
| 04-02.1 | ObjectSet/WhereClause 是 serde wire contract，完整覆盖 eq/gt/gte/lt/lte/isNull/in/startsWith/containsAnyTerm/containsAllTerms/and/or/not，并在编译前按 ontology 类型检查（D-01，QUERY-02） | VERIFIED | query/ast.rs / compile.rs；U:2661 every_where_operator_validates_and_evaluates、U:2665 null_unknown_and_wrong_types_are_checked |
| 04-02.2 | 用户谓词只能读取该行上被某个适用 role 授予的逻辑列；source 下推把 column grant 与 predicate 绑定，合并后仍用同一 Rust 语义复核，filter/sort 不产生 role 行×列交叉（D-03，QUERY-09） | VERIFIED | compile.rs grant predicate 与 execute.rs merged-row recheck；U:2664 multi_role_grant_is_bound_to_its_row、U:2667 sqlite_multi_role_column_grant_does_not_cross_rows |
| 04-02.3 | String token search 采用空白 term 的 any/all 匹配、无 relevance order；ASCII corpus 在 Rust/SQLite/Trino 上一致，无法安全下推的形态留给 bounded Rust recheck（D-05，QUERY-03/08） | VERIFIED | query predicate evaluator 与 typed SQL；U:2668 term_search_is_whitespace_any_or_all、U:2666 sqlite_matches_rust_fixed_corpus；I:1201 where_search_set_and_relationship_corpus |
| 04-02.4 | L0 source compiler 只能产生一个 `TableRef`、没有 JOIN/CTE/UNION/raw SQL；typed printer 支持 outer CASE mask 与 typed distributed EXPLAIN（D-01，QUERY-10） | VERIFIED | compile.rs typed Select 单表，printer/ast.rs CASE/EXPLAIN；sql_shape tests 与 I:1197 aggregate_corpus_and_distributed_explain；benchmark 450 原始计划均1 fragment |
| 04-03.1 | query crate 可按 logical type 与 canonical pk 批量读取 edit patches，单批硬上限 256，且 scan 使用 stable keyset、显式 limit，不做 offset/full-table materialization（D-02，QUERY-04） | VERIFIED | storage/query_read.rs；U:3604 batch_accepts_256_keys_and_all_value_types、U:3606 batch_rejects_more_than_256_before_sql、U:3613 scan_uses_keyset_and_preserves_flags |
| 04-03.2 | M:N link edit 可按任一 endpoint 有界读取，返回 typed endpoint keys、deleted/version；FK 不在 storage 建第二份关系表（D-05，QUERY-05） | VERIFIED | storage/query_read.rs endpoint 查询；U:3610 link_from_or_to_endpoint_is_bounded_and_preserves_removed_edges、U:3609 link_fk_request_is_rejected_with_logical_name |
| 04-03.3 | 所有 read APIs 在 SQLite/PG 都只执行 SELECT，不插 version 0 skeleton、不锁 FOR UPDATE、不改变 log/definition/table row count（QUERY-08） | VERIFIED | storage/query_read.rs；U:3608 get_batch_and_scan_preserve_rows_log_and_definition、U:3611 link_reads_leave_row_log_and_definition_counts_unchanged；I:1167-1168 双后端 parity |
| 04-03.4 | storage error/public structs 使用 logical names；physical schema/table/column 只在 crate-private query builder 与日志 source 中存在（QUERY-09） | VERIFIED | crate-private builder 与 public error mapping；U:3607 corrupt_property_error_uses_logical_name、U:2624 physical_names_never_escape |
| 04-04.1 | source 与 edit candidates 并行取得后，由唯一 Rust production overlay 合并、去重、scope/recheck、排序，并以最多 256 keys 的 backfill/refill 填页（D-01/D-02，QUERY-04/08） | VERIFIED | query/execute.rs / overlay.rs CandidateSet 与 refill；U:2561 scope_and_predicate_run_on_merged_row、U:2613 pagination_backfill_batches_never_exceed_256 |
| 04-04.2 | keyset cursor 按规范化排序值与 pk 恢复，HMAC 绑定 principal、scope fingerprint、definition、object type 与 ordering；上下文不一致或被篡改时 fail closed（D-06，QUERY-04/09） | VERIFIED | query/cursor.rs 绑定规范化上下文；U:2611 cursor_rejects_tamper_and_context_replay、U:2610 cursor_rejects_same_principal_when_attributes_change、U:2609 null/multikey分页 |
| 04-04.3 | 分页继续补取到 page 满或所有 candidate lane 耗尽；work budget 耗尽返回 typed explicit error，不把不完整结果伪装成正常页（D-02，QUERY-09） | VERIFIED | execute.rs loop + shared budget；U:2619 pagination_refills_across_three_false_positive_windows、U:2615 pagination_budget_exhaustion_returns_no_partial_page |
| 04-04.4 | `has_more` 由经 overlay/scope/recheck 后实际存在下一行决定；`truncated` 只表示最终 row/byte cap，不表示 refill 或 budget 状态（D-06，QUERY-04/09） | VERIFIED | execute.rs lookahead/cap；U:2623 pagination_stops_after_eligible_lookahead、U:2612 final_row_and_byte_caps_truncate、U:2618 pagination_list_source_keyset_and_has_more |
| 04-04.5 | 同一请求共享冻结的`ExecutionLimits`：32 backend calls、10,000 raw candidates、2,048累计backfill keys、64 MiB decoded bytes；逐维B可成功、B+1无partial失败，256仍是单批keys上限（QUERY-09） | VERIFIED | query/execute.rs共享请求budget；U:2560 pagination_execution_limit_boundaries_are_inclusive、U:2622 pagination_source_and_edit_share_all_work_budgets |
| 04-05.1 | FK searchAround 读取合并后对象的logical FK；M:N 将source join-table edges与PG link edits在Rust合并，Add/Remove与deleted语义遵守ADR-0024（D-05，QUERY-05） | VERIFIED | query/link.rs reads logical FK / source+edit edges；U:2599 outbound_fk_uses_scoped_merged_object_value、U:2598 many_to_many_merges_source_and_link_edits_fixed |
| 04-05.2 | 任何related target都通过正常的scoped pk/list read hydrate；隐藏、tombstone或目标row scope拒绝时不返回且不可由错误区分（D-03/D-05，QUERY-05/09） | VERIFIED | link.rs normal execute_list hydration；U:2597 many_to_many_hydrates_257_targets_in_bounded_batches；I:1201 traversal corpus |
| 04-05.3 | union/intersect/subtract只对相同logical object type的已授权pk集合运算，随后复用正常overlay/projection/order/caps；集合代数不能成为scope或definition旁路（QUERY-06/09） | VERIFIED | query/set.rs:56 authorized_keys → :68 execute_list_internal；U:2640 valid_same_type_set_and_cross_type_rejection、U:2639 set_result_reuses_normal_hydration_order_and_cursor |
| 04-05.4 | FK、M:N与set operations各有独立reference corpus，candidate重复、edit移动、multi-role与空集合保持确定、去重且有界（D-07，QUERY-08） | VERIFIED | tests/search_around.rs 独立 edge oracle / set_ops.rs 独立 BTreeSet oracle；U:2638 set_ops_match_independent_btreeset_model_fixed、U:2595 duplicate_edits_use_highest_version_independent_of_input_order；property tests 同日志通过 |
| 04-06.1 | count/sum/avg 用source partial加edited-key old/new contribution correction得到exact merged result；avg由sum+count组成，不能平均两个avg（D-04，QUERY-07） | VERIFIED | query/aggregate.rs source partial + old/new correction；U:2463 count_sum_avg_match_full_materialization_reference；独立 CSV full-materialization oracle |
| 04-06.2 | min/max以typed single-table TopN source windows与全部edit candidates合并，持续补取直到证明winner或返回显式work/cost error；不能因首个candidate失效返回错误值（D-04，QUERY-07/10） | VERIFIED | aggregate.rs TopN refill；U:2465 min_max_edited_values_can_win_or_remove_winner、U:2466 min_max_refill_past_invalid_top_rows；I:1197 |
| 04-06.3 | distinct与groupBy只使用scope后可见值；connector能力或预算不足以证明exact结果时返回typed explicit error，永不以`truncated`或partial buckets代替完成（D-03/D-04，QUERY-07/09） | VERIFIED | aggregate.rs exact scoped group/distinct + typed limits；U:2464 group_distinct_matches_visible_merged_reference；I:1197 |
| 04-06.4 | aggregate fixed/property corpus覆盖SQLite reference、multi-role row×column、null、edited membership/value/group移动与connector能力选择（D-07，QUERY-08） | VERIFIED | tests/aggregate.rs 与 library property corpus；U:2461 count_sum_avg_connector_capability_selects_supported_plan、U:2463-2466；I:1197 |
| 04-06.5 | 可信host冻结的Phase 4初始`AggregateLimits`为1,024 groups、4,096 buckets、请求总计10,000 distinct values、16 MiB distinct memory；与`ExecutionLimits`共享同一请求budget，wire不可调高（QUERY-09） | VERIFIED | aggregate limits frozen by host；U:2462 count_sum_avg_limits_phase4_defaults_are_exact、U:2460 aggregate_wire_request_rejects_capabilities、U:2607 aggregate_invalid_names_and_limits_fail_before_backend_access |
| 04-07.1 | 真实Linux CI在PostgreSQL 17 + Trino 483上实际执行storage `query_read_pg`与query `trino_query_it` ignored tests，且独立discovery guard证明每个target至少有一个ignored test（D-07，QUERY-08） | VERIFIED | .github/workflows/ci.yml integration；I:1167-1170 query_read_pg 2 passed、I:1197-1203 trino_query_it 5 passed；ignored discovery guard作业success |
| 04-07.2 | 每种source-producing query shape都对真实Trino执行`EXPLAIN (TYPE DISTRIBUTED)`，parser必须识别至少1个且不超过2个fragment；无法解析也失败（D-01，QUERY-10） | VERIFIED | tests/trino_query_it.rs 真实 EXPLAIN；I:1197；isolated artifact重算450 plan fragment_count全部1，解析失败会fail |
| 04-07.3 | 真实backend corpus覆盖multi-role、overlay、cursor、traversal、set、aggregate与physical-error消毒，SQLite expected与Trino/PG结果逐项一致（QUERY-08/09） | VERIFIED | I:1198 fixed_semantics_and_cursor_refill、I:1200 real_security_roles_and_cursor_replay、I:1199 real_physical_errors_are_sanitized、I:1201 relationship/set corpus |
| 04-07.4 | 专用无并行负载的Linux CI run以固定dataset、至少5次warmup和40次measured samples证明pk/list/aggregate/edited-PG-direct p95预算，并保存raw timings、plan、环境与run URL（D-07，QUERY-10） | VERIFIED | 41759457559 isolated-query-bench job success；4×(5 warmup+40 measured)，p95重算123.194768/425.338425/374.549715/0.913019ms，edited source_calls全0；原始artifact SHA256与源码等价证明见性能节 |

**Score:** 36/36；behavior_unverified: 0。无新增 coincidence reliance 或 backstop 项。

### Required Artifacts / Key Link Verification

前 owner 已逐7份PLAN执行 artifact/key-link owner query，实际读源码纠偏；本次复用该审计并检查当前源码身份与最终行为日志。自动匹配24/26 artifacts、18/19 links；下列3个不匹配均由真实定义和调用消解，不能当自动通过。

| Artifact / link | Actual implementation | Status |
|---|---|---|
| storage read API export pattern | `query_read.rs:347` impl 的公开 `read_object_edit` → `read_object_edits`，query/lib实际消费 | VERIFIED / WIRED |
| query list export pattern | `execute.rs:506` impl 公开 `execute_list` → audited → with_budget → dispatch | VERIFIED / WIRED |
| set旧函数名pattern | `set.rs:56` authorized_keys → `:68` execute_list_internal，U:2639正常hydration | VERIFIED / WIRED |
| `ast.rs / compile.rs / overlay.rs / execute.rs` | wire→类型检查→单表typedSQL→真实source/edit候选→overlay/recheck→页面 | VERIFIED |
| `link.rs / set.rs / aggregate.rs / cursor.rs` | 都通过正常scoped路径，非孤立模块；上方每条truth列出实际测试 | VERIFIED |
| `ci.yml / ci/query-services.sh / tests/trino_query_it.rs / tests/query_bench.rs` | PostgreSQL/Trino集成、discovery、isolated入口有实际job与artifact | VERIFIED |

### Data-Flow Trace (Level 4)

| Rendered value | Real data chain | Status |
|---|---|---|
| pk row / _version | read_object_edit SELECT + required source LIMIT 2 → overlay → per-row grant → projection → TokenCodec | FLOWING |
| list / has_more / cursor | source typed SELECT + edit keyset → CandidateSet → bounded backfill → scope/recheck → lookahead/sort → cursor | FLOWING |
| related / set rows | scoped FK/edge/source+edit → authorized_keys → normal list hydration | FLOWING |
| aggregate | source partial / TopN + edit old/new corrections → scoped exact aggregate or typed failure | FLOWING |

不存在以硬编码空数组替代查询结果的路径；初始空集合后续由真实 store / Engine 填充。

### Behavioral Spot-Checks and Current Performance

本次 spot-check 为原始行为日志的独立校验，没有重复执行全 suite。U、I 的精确命名与行号见truth表。真实后端：I:1167-1170 storage 2 passed，I:1197-1203 query 5 passed，全部0 failed/0 ignored。

原始 artifact：`target/milestone-bench-41759457559/phase4-isolated.json`。
SHA256：`13c530fb55b756d01be417926b719801c8544fedfe9a6b049edea1ced572df99`。

| Path | Warmup / measured | 独立 nearest-rank p95 ms | Strict threshold ms | Source calls/sample | Status |
|---|---:|---:|---:|---:|---|
| source_pk | 5 / 40 | 123.194768 | <500 | 1 | PASS |
| filtered_list_page | 5 / 40 | 425.338425 | <1000 | 5 | PASS |
| aggregate | 5 / 40 | 374.549715 | <1500 | 5 | PASS |
| edited_pg_direct_pk | 5 / 40 | 0.913019 | <50 | 0 | PASS |

对每路40个elapsed_ms排序，取ceil(0.95×40)=38位重算；全部180个样本error=null且limit_error=false。完整450个plan文本逐份用 `^Fragment [0-9]+ ` 重算，均1个，等于artifact字段；无不可解析或>2 fragment。source_pk本身通过真实语义/shape集成检查，artifact该路径不附plan，未将空plans误报为一个分片。

`guardrails_raised=false`、`limit_error_known=true`、`limit_error_samples=0`。ExecutionLimits为32/10000/2048/67108864，AggregateLimits为1024/4096/10000/16777216，均未调高。环境为isolated Linux、Trino483、PG17.11、10,000条合成航空benchmark行、1,000 edited（800 patch/100 tombstone/100 release），并非吞吐或所有生产集群SLA。

### Probe Execution

未声明 `scripts/*/tests/probe-*.sh`。真实EXPLAIN、benchmark与后端corpus已按项目专用目标执行并独立复核原始产物；无以SUMMARY PASS计数代替probe。

### Requirements Coverage

| Requirement | Plans | Status | Evidence |
|---|---|---|---|
| QUERY-01 | 04-01 | SATISFIED | pk edit/source/tombstone/release/ambiguity truth |
| QUERY-02 | 04-02 | SATISFIED | 运算符矩阵与SQLite/Trino parity |
| QUERY-03 | 04-02 | SATISFIED | whitespace any/all term corpus |
| QUERY-04 | 04-03/04 | SATISFIED | batch/keyset/refill/cursor/budget |
| QUERY-05 | 04-03/05 | SATISFIED | FK/M:N与双后端link reads |
| QUERY-06 | 04-05 | SATISFIED | 独立BTreeSet oracle及正常hydration |
| QUERY-07 | 04-06 | SATISFIED | exact aggregate/TopN/reference |
| QUERY-08 | 04-01..07 | SATISFIED | 独立参考模型、SQLite/PG/Trino corpus |
| QUERY-09 | 04-01..07 | SATISFIED | scope、防探测、HMAC、消毒、caps |
| QUERY-10 | 04-02/06/07 | SATISFIED | 单表typed SQL、EXPLAIN、当前isolated p95 |

全部10个ID在PLAN和当前traceability中对应，无orphan。STORE-05的source cardinality独立缺口归Phase07.1，本报告不覆盖、更不宣布milestone全部需求满足。

### Decision Coverage

All trackable CONTEXT.md decisions are honored by shipped artifacts.
本次GSD `check.decision-coverage-verify`：7/7，not_honored=[]，warning-only gate。

### Test Quality / Anti-Patterns

已复用前owner逐源码/断言审计，并对新增性能证据复核：aggregate oracle从CSV独立全量计算，set独立BTreeSet，关系独立edge模型，不从production输出生成expected。property、fixed、真实后端测试均有值或行为断言。真实后端ignored目标由integration显式执行；没有仅disabled测试证明需求。

当前query/engine生产代码与storage/query_read、online.rs无未引用TBD/FIXME/XXX或placeholder stub。新增取消drain、请求snapshot、audit取消不是仅grep判断：最终CI的对应named tests均通过，审计轨迹见 `.planning/milestones/v1.0-audit/PHASE-02-04-REVERIFY.md`。

### Advisory (New Scope, Unevidenced)

None。没有新增无确定证据的架构意见推翻旧完成项。

### Human Verification Required

无未决的本phase人工项。历史UAT不扩大为本轮人工实测；当前基础库行为已由真实输入输出、值断言与独立运行证据覆盖。性能结论限于记录环境。

### Gaps Summary

Phase04当前目标已证实；本轮关闭的是当前源码证据缺口。报告不替代Phase07.1的STORE-05验收。

_Verifier: gsd-verifier_
