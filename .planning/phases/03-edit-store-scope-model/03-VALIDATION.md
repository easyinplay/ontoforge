---
phase: "03"
slug: "edit-store-scope-model"
status: validated
nyquist_compliant: false
wave_0_complete: true
created: "2026-10-19"
validated: "2026-11-04"
evidence_head: "f8851ca073fed34e80e605f0ac91880259df7e12"
ci_run: 41759804733
task_coverage: "50/50 executable; 5 static tasks"
requirement_coverage: partial
remaining_requirement: "STORE-05 source cardinality report; Phase 7.1"
---

# Phase 03 Validation Strategy

本次完成历史task级覆盖审计，替换已失真的draft/W0 pending标记。依据为实际源码、具名测试断言及原始CI/fresh日志；未运行Cargo或启动服务。全量97-task历史矩阵保留在 `../../milestones/v1.0-audit/NYQUIST-02-03-05-VALIDATION.md`，这里是本阶段更新后的事实源。

## Test Infrastructure / Sampling

| Property | Value |
|---|---|
| Framework | Rust cargo test、tokio、proptest；HTTP使用wiremock |
| Quick command | `cargo test -p ontoforge-auth --test scope_merge --locked`；按task选择具名测试 |
| Full command | `cargo test --workspace --locked -- --test-threads=1`；外部PG/Trino使用CI专门target与显式环境 |
| CI identity | run41759804733，head f8851ca，合成merge22913c8；四必需job success |
| Measured workspace | Windows150组1496passed/0failed/117ignored；Ubuntu150组1503/0/117；只计workspace step |
| Feedback latency | 窄测试运行见原始日志；全workspace含编译Windows约18m12s、Ubuntu约8m55s，不虚称满足旧180s估计 |

每task先用相应具名测试；每wave跑受影响crate/后端；收口使用已经完成的新HEAD CI，不重复全量执行。历史执行期每次反馈时延无法倒推，不声明当时每次都小于阈值；Nyquist结论是当前requirement→test覆盖，非虚构历史时序。

## Per-Task Verification Map

W/U/I/S是历史run41669982375四份原始日志；全部当前受核生产输入未变或已由新run覆盖。下表原有COVERED行复用前独立auditor的源码和冻结日志映射，不声称本worker重跑。新增G2/G3/G4使用本轮日志；具体行号与原名索引留在97-task矩阵。

| Task | Requirement | 行为 | 命名测试 | 可执行命令 | 结论 |
|---|---|---|---|---|---|
| 03-01-T1 | STORE-01, AUTH-01 | 人工核验 sea-query-sqlx 与 getrandom（安装前的 legitimacy 门禁） | 无命名动态测试；见结论 | `原 PLAN 静态/决策检查，非动态测试` | SKIP: 安装前人工 legitimacy checkpoint，SUMMARY 明记 approved；不能把历史批准冒充动态测试。 |
| 03-01-T2 | STORE-01, AUTH-01 | 端到端 tracer：依赖与 crate 图落地，storage 打开带 recursive_triggers 与 engine collation 的 SQLite 连接，auth 用 aws-lc-rs 往返一次 JWT | `every_pooled_connection_has_recursive_triggers`; `engine_collations_are_registered`; `foreign_keys_on`; `hs256_round_trip_does_not_panic` | `cargo test -p ontoforge-storage --test sqlite_connection && cargo test -p ontoforge-auth --test crypto_backend` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-01-T3 | STORE-01, AUTH-01 | 供应链guard及ADR依赖方向 | `test_neither_package_is_allowed`; `test_openssl_only_is_rejected`; `test_native_tls_only_is_rejected`; `test_both_packages_are_rejected`; `test_cargo_failure_is_rejected`; `test_similar_package_names_are_allowed` | `python -B -m unittest discover -s ci -p test_tls_deps.py -v`; `bash ci/check-tls-deps.sh` | COVERED: G2已闭合；新CI双平台6/6且真实helper通过，查询失败和禁包均fail-closed。 |
| 03-02-T1 | STORE-02, SCOPE-02, AUTH-01 | 端到端 tracer：`indexed: true` 从定义文件经 IR、JSON Schema 到 `ontoforge validate`，并一次分配 ONT-V21..V27 | `indexed_defaults_to_false`; `indexed_true_is_kept`; `indexed_must_be_boolean`; `diag::tests::phase3_codes` | `cargo test -p ontoforge-schema --locked` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-02-T2 | STORE-02, SCOPE-02, AUTH-01 | 主体属性类型与运行期取值（D-05），`ontoforge-server.toml` 的 `[store]` 与 `[auth]` 段及校验 | `rowpred::value::tests::attrs_missing_is_null`; `rowpred::value::tests::attrs_list_rejects_null_element`; `store_sqlite_defaults`; `store_postgres_requires_exactly_one_url_source`; `store_sqlite_rejects_url_and_postgres_rejects_path`; `store_schema_names_are_validated`; `oidc_two_issuers_with_defaults`; `oidc_rejects_symmetric_and_none_algorithms`; `oidc_issuer_rules`; `principal_attrs_types`; `unknown_key_in_store_or_auth_is_v20` | `cargo test -p ontoforge-engine --locked --test server_config && cargo test -p ontoforge-expr --locked` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-03-T1 | STORE-02, STORE-01 | 确认 edit store 的物理格式（set_mask 编码、命名、取值编码、PG schema 划分） | 无命名动态测试；见结论 | `原 PLAN 静态/决策检查，非动态测试` | SKIP: 物理格式决策 checkpoint，ADR-0024 accepted；后续 reload/conformance 验证选定合同。 |
| 03-03-T2 | STORE-02, STORE-01 | 端到端 tracer：`reload` 库函数在 SQLite edit store 中按定义建表并记录定义版本（含 diff、dry-run 与中断可重入） | `first_reload_creates_tables`; `second_reload_is_noop`; `additive_property_adds_columns`; `destructive_changes_are_rejected`; `dry_run_changes_nothing`; `reload_resumes_after_partial_ddl` | `cargo test -p ontoforge-storage --locked --test reload` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-03-T3 | STORE-02, STORE-01 | SQL 文本出口源码守卫与 ADR-0024 | `sql_text_only_from_sql_exit` | `cargo test -p ontoforge-storage --locked --test sql_exit_guard` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-04-T1 | STORE-01, STORE-02 | 端到端 tracer：`ontoforge reload` 子命令在 SQLite edit store 上建表、dry-run 与退出码 | `reload_creates_tables`; `dry_run_prints_ddl_only`; `missing_store_section`; `store_unavailable`; `invalid_definitions_never_open_store`; `json_output` | `cargo test -p ontoforge --locked --test cli_reload` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-04-T2 | STORE-01, STORE-02 | PostgreSQL 后端（两个 schema、`COLLATE "C"`、`IF NOT EXISTS`）、两种方言 DDL 快照与 `pg_store()` | `ddl::tests::ddl_snapshots`; `first_reload_on_pg_puts_edit_tables_in_edit_schema`; `second_reload_on_pg_is_noop`; `additive_reload_on_pg_adds_columns` | `cargo test -p ontoforge-storage --locked --lib ddl_snapshots ; cargo test -p ontoforge-storage --test reload_pg --locked -- --ignored` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-05-T1 | SCOPE-04, SCOPE-05, SCOPE-02 | 确认 D-06 三值语义的具体规则（Kleene 真值表、`in` 与空列表、字符串匹配、Unknown 不可见） | 无命名动态测试；见结论 | `原 PLAN 静态/决策检查，非动态测试` | SKIP: 三值语义决策 checkpoint，ADR-0025 accepted；同 plan T2/T3 和 03-11 动态验证。 |
| 03-05-T2 | SCOPE-04, SCOPE-05, SCOPE-02 | 端到端 tracer：`object.tenantId == principal.attrs.tenantId` 从 CEL 文本经解析、同行 / 类型检查到 `Pred`，再经 Kleene 求值得到可见性 | `tenant_equality_visible`; `tenant_mismatch_false`; `missing_attr_is_unknown_and_invisible`; `null_column_not_equal_is_unknown`; `explicit_null_branch_visible`; `unknown_root_rejected` | `cargo test -p ontoforge-expr --locked --test rowpred_eval` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-05-T3 | SCOPE-04, SCOPE-05, SCOPE-02 | 完整 D-07 子集与类型规则、拒绝语料、Display 往返与 Kleene 定律 proptest、ADR-0025 与 `docs/scope.md` | `display_round_trip`; `kleene_laws`; `string_match_is_case_sensitive_and_literal` | `cargo test -p ontoforge-expr --locked` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-06-T1 | SCOPE-01, SCOPE-02 | 端到端 tracer：scope 目录经解析、kind 分发到 `ScopeSet` IR，错误带 line:col | `loads_aviation_scope_example`; `unknown_field_is_v04_with_location`; `binding_to_unknown_role_is_v21`; `toolset_value_is_checked`; `multi_document_file_loads` | `cargo test -p ontoforge-schema --locked --test scope_documents ; cargo test -p ontoforge-schema --locked` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-06-T2 | SCOPE-01, SCOPE-02 | scope JSON Schema 导出、跨目录 kind 的 V03 提示、subjects 规则与诊断快照 | `role_in_ontology_dir_is_v03`; `object_type_in_scope_dir_is_v03`; `subject_rules`; `duplicate_role_is_v08`; `second_binding_for_role_is_v08`; `committed_schemas_are_up_to_date`; `scope_schema_is_the_third_exported_file` | `cargo test -p ontoforge-schema --locked && cargo test -p ontoforge --locked --test cli_schema` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-07-T1 | AUTH-01, SCOPE-01 | 端到端 tracer：wiremock 提供 JWKS，RS256 令牌经 alg 路由、签名与 claims 校验成为带 groups 与 attrs 的 Principal | `rs256_token_becomes_principal`; `missing_attr_claim_is_absent`; `jwks_is_fetched_once_for_repeated_tokens` | `cargo test -p ontoforge-auth --locked --test oidc` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-07-T2 | AUTH-01, SCOPE-01 | JWKS 缓存策略（TTL 夹取、kid 未命中单飞与节流、负缓存、stale-if-error、体积 / 超时上限）与完整拒绝矩阵 | `cache_control_max_age_is_clamped`; `kid_miss_refetches_once_then_throttles`; `concurrent_kid_miss_is_single_flight`; `stale_keys_survive_idp_outage`; `oversized_jwks_rejected`; `header_key_urls_are_never_followed`; `two_issuers_namespace_groups` | `cargo test -p ontoforge-auth --locked` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-08-T1 | STORE-03, STORE-01, STORE-07 | 确认 edit log 的物理格式（D-10 / D-11 的具体列、取值的 JSON 编码、历史快照结构） | 无命名动态测试；见结论 | `原 PLAN 静态/决策检查，非动态测试` | SKIP: edit log 格式决策 checkpoint，ADR-0026 accepted；T2 与 03-09 动态验证。 |
| 03-08-T2 | STORE-03, STORE-01, STORE-07 | 端到端 tracer：一次 set 提交写出 edit 行、日志头、entry 与历史快照，并能按 seq 读回；conformance 框架在 SQLite 与 PG 上运行 | `commit_set_creates_edit_row`; `commit_appends_one_log_record`; `commit_writes_history_snapshot`; `second_set_patches_only_given_properties`; `invalid_values_are_rejected_without_writes`; `stale_def_version_rejected` | `cargo test -p ontoforge-storage --test conformance_sqlite --locked ; cargo test -p ontoforge-storage --test conformance_pg --locked -- --ignored` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-09-T1 | STORE-03, STORE-01 | 端到端 tracer：append-only 触发器在两个后端拒绝改写日志（含 TRUNCATE 与 SQLite 替换式写入） | `append_only_rejects_update_and_delete`; `append_only_rejects_truncate_on_pg`; `append_only_rejects_sqlite_replace_writes` | `cargo test -p ontoforge-storage --test conformance_sqlite --locked append_only ; cargo test -p ontoforge-storage --test conformance_pg --locked append_only -- --ignored` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-09-T2 | STORE-03, STORE-01 | 重放重建 edit store、版本分歧检测、重复提交与 ADR-0026 | `replay_rebuilds_edit_store`; `replay_detects_version_divergence`; `same_commit_twice_creates_two_records` | `cargo test -p ontoforge-storage --locked --test conformance_sqlite` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-10-T1 | STORE-02, STORE-05 | 端到端 tracer：aviation reload 建出 `indexed` 普通索引与 oneToOne 外键部分唯一索引，数据库拒绝违反 1:1 的第二行 | `aviation_reload_creates_indexes`; `one_to_one_unique_index_rejects_second_row`; `index_added_on_second_reload`; `removing_indexed_keeps_index`; `ddl::tests::ddl_snapshots` | `cargo test -p ontoforge-storage --locked --test reload ; cargo test -p ontoforge-storage --locked --lib` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-10-T2 | STORE-02, STORE-05 | 物理名上限与碰撞（ONT-V24）、link / 精度类破坏性判定、基数收紧失败保留旧定义、PG 用例 | `naming::tests::bounded_names`; `snake_collision_is_v24`; `system_column_collision_is_v24`; `long_names_round_trip`; `link_changes_are_destructive`; `precision_and_items_changes_are_destructive`; `tightening_to_one_to_one_with_conflicts_keeps_old_definition`; `relaxing_one_to_one_drops_unique_index`; `indexes_on_pg`; `long_names_on_pg` | `cargo test -p ontoforge-storage --locked --test reload && cargo test -p ontoforge-storage --locked --lib naming` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-11-T1 | SCOPE-04, SCOPE-03, SCOPE-05, SCOPE-02 | 端到端 tracer：一个比较谓词代入主体属性、降为 printer AST、在 SQLite 上执行，三值结果与 Rust eval 相同 | `tenant_equality_matches`; `null_column_comparison_is_unknown_on_both_sides`; `empty_list_attr_is_false_on_both_sides` | `cargo test -p ontoforge-auth --locked --test rowpred_diff` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-11-T2 | SCOPE-04, SCOPE-03, SCOPE-05, SCOPE-02 | printer 的 `StringMatch` 与两方言转义出口、完整差分 proptest、Trino 固定语料测试与 PG 种子 | `string_match_matches_rust_on_sqlite`; `rust_eval_equals_sqlite`; `corpus_cases_match_on_sqlite`; `trino_table_equals_the_corpus_rows`; `trino_matches_rust_on_the_corpus_cases`; `trino_matches_rust_on_seeded_predicates` | `cargo test -p ontoforge-engine --test printer_prop --locked ; cargo test -p ontoforge-auth --test rowpred_diff --locked ; cargo test -p ontoforge-auth --test rowpred_trino_it --locked -- --ignored` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-11-T3 | SCOPE-04, SCOPE-03, SCOPE-05, SCOPE-02 | 多 role 合并 `TypeScope`：fail-closed、列 × 行不泄露、合并后的 SQL 与 Rust 一致 | `no_grant_is_invisible`; `role_without_row_filter_sees_all_rows`; `a_columns_times_b_rows_do_not_leak`; `same_property_from_two_roles_is_or`; `visible_columns_are_union_of_true_grants`; `lowered_scope_matches_evaluate_on_sqlite` | `cargo test -p ontoforge-auth --locked --test scope_merge` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-12-T1 | AUTH-02 | 端到端 tracer：`ontoforge dev-token` 生成密钥与令牌，显式启用的 dev 验证器把它认证为带 groups 与属性的 Principal | `dev_token_round_trip`; `key_file_is_created_once_and_reused`; `issues_token_verifiable_by_dev_verifier`; `undeclared_attr_is_rejected` | `cargo test -p ontoforge-auth --locked --test dev_token && cargo test -p ontoforge --locked --test cli_dev_token` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-12-T2 | AUTH-02 | 配置不能启用 dev、alg 路由矩阵、密钥与令牌的边界 | `config_alone_never_enables_dev_tokens`; `rs256_token_with_dev_issuer_is_rejected`; `hs256_with_other_issuer_rejected`; `wrong_key_rejected`; `expired_dev_token_rejected`; `missing_sub_rejected`; `oidc_still_works_with_dev_enabled`; `key_file_permissions_unix`; `ttl_bounds` | `cargo test -p ontoforge-auth --locked && cargo test -p ontoforge --locked --test cli_dev_token` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-13-T1 | STORE-07, STORE-05, STORE-01 | 端到端 tracer：unset、tombstone、release 与 M:N link add / remove 经 commit 写入 edit 表与日志 | `source_object_lifecycle`; `edit_only_rules`; `link_add_remove_toggle`; `log_records_every_op` | `cargo test -p ontoforge-storage --locked --test conformance_sqlite` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-13-T2 | STORE-07, STORE-05, STORE-01 | 独立参考模型与 proptest：每步与真实 store 逐行相等，序列结束后日志重放逐列相等 | `model_matches_store` | `cargo test -p ontoforge-storage --locked --test model_prop` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-13-T3 | STORE-07, STORE-05, STORE-01 | 1:1 基数违反映射、M:N 重复添加、同 pk 与同外键的并发提交（两后端） | `one_to_one_violation_is_reported`; `duplicate_link_add_keeps_one_live_row`; `concurrent_commits_on_same_pk_serialize`; `concurrent_fk_claims_one_wins` | `cargo test -p ontoforge-storage --test conformance_sqlite --locked ; cargo test -p ontoforge-storage --test conformance_pg --locked -- --ignored` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-14-T1 | SCOPE-02, SCOPE-05, SCOPE-06, SCOPE-01 | 端到端 tracer：示例 scope 对 aviation 编译为 CompiledScope，主体按绑定解析出合并后的 PrincipalScope；错误的 rowFilter 定位到 YAML | `aviation_scope_compiles`; `principal_with_two_roles_gets_merged_flight_scope`; `principal_without_roles_sees_nothing`; `user_subject_binding`; `bad_row_filter_is_v22_located` | `cargo test -p ontoforge-auth --locked --test scope_compile` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-14-T2 | SCOPE-02, SCOPE-05, SCOPE-06, SCOPE-01 | criteria 列覆盖（ONT-V23）、collation 检查（D-08）与其余引用 / 一致性规则（ONT-V21） | `collation_case_insensitive_is_v22`; `undeclared_attr_is_v22`; `empty_object_types_grants_nothing`; `all_errors_reported_together`; `stored_roles_diagnostics_without_sources`; `scope_compile_diagnostics_json` | `cargo test -p ontoforge-auth --locked --test criteria_coverage --test scope_compile` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-15-T1 | STORE-02 | 端到端 tracer：删除带数据的属性 → reload 被 ONT-V25 拦下 → `migrate --plan` → `migrate --apply` 归档旧列 → reload 恢复 | `archive_removed_property`; `apply_with_stale_hash_is_rejected`; `reapply_after_success_is_rejected`; `migrate_plan_then_apply`; `apply_requires_plan_hash` | `cargo test -p ontoforge-storage --locked --test migrate && cargo test -p ontoforge --locked --test cli_migrate ; cargo test -p ontoforge-storage --locked --test reload && cargo test -p ontoforge --locked --test cli_reload` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-15-T2 | STORE-02 | 改名声明（属性 / type）、改类型、改 pk、删 type 与 `--drop-archived` 清理 | `rename_property_moves_data`; `rename_with_type_change_is_rejected`; `rename_object_type_moves_table`; `invalid_rename_decl_is_rejected`; `type_change_archives_and_adds`; `primary_key_change_archives_table`; `removed_type_archived`; `drop_archived_cleans_up`; `drop_archived_plan_and_apply` | `cargo test -p ontoforge-storage --locked --test migrate && cargo test -p ontoforge --locked --test cli_migrate` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-16-T1 | SCOPE-01, SCOPE-07, SCOPE-02, SCOPE-06 | 端到端 tracer：`ontoforge scope plan` → `ontoforge scope apply` 把示例 scope 写入系统库并留下一条审计 | `plan_then_apply_aviation_scope`; `apply_writes_roles_bindings_and_audit`; `apply_creates_row_filter_indexes`; `load_scope_round_trips`; `invalid_scope_writes_nothing` | `cargo test -p ontoforge-storage --locked --test scope_store && cargo test -p ontoforge --locked --test cli_scope` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-16-T2 | SCOPE-01, SCOPE-07, SCOPE-02, SCOPE-06 | 全量同步删除、hash 防漂移、顺序无关、审计不可改、唯一写入口守卫、user 记录（D-04）与文档 | `full_sync_removes_bindings_and_roles`; `stale_plan_hash_rejected`; `order_independent_hash`; `admin_audit_is_append_only`; `scope_tables_written_only_by_scope_store`; `record_login_creates_user_once`; `apply_without_active_definition_fails`; `pg_scope_apply` | `cargo test -p ontoforge-storage --locked --test scope_store && cargo test -p ontoforge --locked --test cli_scope` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-17-T1 | STORE-04 | 端到端 tracer：dispatcher 调用者读取 Flight 的历史，可见列给值、被遮蔽列只标「有变更」 | `ordinary_caller_sees_visible_values_only`; `serialized_history_never_contains_masked_values`; `params_hidden_from_ordinary_caller` | `cargo test -p ontoforge-storage --locked --test history_masking` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-17-T2 | STORE-04 | 不可见 ≡ 不存在 ≡ 无授权、当前可见列决定遮蔽、各操作类型与分页 | `invisible_nonexistent_and_ungranted_are_identical`; `visibility_uses_current_row`; `unset_tombstone_release_entries`; `pagination_with_before_seq`; `link_entries_are_not_object_history` | `cargo test -p ontoforge-storage --locked --test history_masking` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-18-T1 | STORE-02, SCOPE-06, SCOPE-07 | 端到端 tracer：reload 切换到新定义后，持旧 def_version 的提交被数据库栅栏拒绝 | `stale_writer_rejected_after_switch`; `in_flight_commit_blocks_switch_on_pg` | `cargo test -p ontoforge-storage --locked --test reload stale_writer_rejected_after_switch ; cargo test -p ontoforge-storage --locked --test conformance_sqlite` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-18-T2 | STORE-02, SCOPE-06, SCOPE-07 | lock_timeout 与 advisory 单飞、D-18 的 role 复核与行谓词属性索引对账、CLI 装配 | `concurrent_reloads_single_flight`; `lock_timeout_keeps_old_definition`; `reload_rechecks_applied_roles`; `reload_creates_row_filter_indexes`; `reload_reports_role_breakage` | `cargo test -p ontoforge-storage --locked --test reload && cargo test -p ontoforge --locked --test cli_reload ; cargo test -p ontoforge-storage --locked --test migrate --test scope_store` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-18-T3 | STORE-02, SCOPE-06, SCOPE-07 | Trino 元数据刷新与可见探测（两段式切换）、CALL 的 printer 出口与 client 无列结果、Linux CI 集成测试 | `call_without_columns_returns_empty_output`; `probe_waits_until_visible`; `probe_timeout_keeps_pending`; `array_columns_are_not_probed`; `cached_column_requires_flush_before_definition_switch` | `cargo test -p ontoforge-engine --test trino_client --locked ; cargo test -p ontoforge-storage --test reload --locked ; cargo test -p ontoforge-storage --test reload_trino_it --locked -- --ignored --test-threads=1` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-19-T1 | STORE-06 | 端到端 tracer：`ontoforge import` 把 aviation 的 Airport CSV 写入 edit store，每批在 edit log 中以 system:import 记入 | `imports_airports_csv`; `batches_follow_batch_size`; `geopoint_and_decimal_cells` | `cargo test -p ontoforge-storage --locked --test import && cargo test -p ontoforge --locked --test cli_import ; cargo test -p ontoforge-engine --locked && cargo test -p ontoforge-storage --locked --test conformance_sqlite` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-19-T2 | STORE-06 | JSON 与 M:N link 导入、已存在对象的三种模式、source-backed 拒绝、校验与上限、部分提交报告 | `json_import_matches_csv`; `editonly_link_import`; `source_backed_type_is_rejected`; `existing_objects_block_create_only`; `tombstoned_object_is_never_revived`; `file_level_validation`; `partial_failure_is_reported` | `cargo test -p ontoforge-storage --locked --test import && cargo test -p ontoforge --locked --test cli_import` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-20-T1 | STORE-01, SCOPE-04, STORE-02 | 端到端 tracer：PR 的 integration 作业在真实 PostgreSQL 上运行 edit store conformance，并守卫 ignored 用例确实被执行 | `commit_appends_one_log_record`; `append_only_rejects_truncate_on_pg`; `every_value_type_round_trips` | `cargo test -p ontoforge-storage --test conformance_pg --test reload_pg --test scope_store --test migrate_pg --test history_masking --test import --locked -- --ignored` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-20-T2 | STORE-01, SCOPE-04, STORE-02 | Trino 行谓词语料与 reload 可见探测步骤、embedded-pg 编译守卫、`docs/edit-store.md` 与 CONTRIBUTING | `trino_matches_rust_on_seeded_predicates`; `cached_column_requires_flush_before_definition_switch` | `cargo test -p ontoforge-auth --test rowpred_trino_it --locked -- --ignored ; cargo test -p ontoforge-storage --test reload_trino_it --locked -- --ignored --test-threads=1` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-21-T1 | STORE-03, STORE-06, STORE-01 | 端到端 tracer：一个 17 位以上有效数字的 decimal 从 JSON 导入到 edit 行、日志与历史都不被舍入 | `value::tests::decimal_json_numbers_must_be_exact_integers`; `value::tests::json_round_trips_every_type`; `json_decimals_are_never_rounded`; `every_value_type_round_trips` | `cargo test -p ontoforge-storage --locked --lib decimal_json_numbers_must_be_exact_integers ; cargo test -p ontoforge-storage --locked --test import json_decimals_are_never_rounded ; cargo test -p ontoforge-storage --locked` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-21-T2 | STORE-03, STORE-06, STORE-01 | PostgreSQL 存不下的值在校验阶段拒绝，两个后端结论一致（WR-A01） | `values_postgres_cannot_store_are_refused_on_every_backend`; `value::tests::postgres_limits_are_validated`; `nul_in_a_cell_is_refused_before_anything_is_written` | `cargo test -p ontoforge-storage --locked --test conformance_sqlite values_postgres_cannot_store_are_refused_on_every_backend ; cargo test -p ontoforge-storage --features embedded-pg --locked --test conformance_pg --test import -- --ignored ; cargo test -p ontoforge-storage --locked` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-21-T3 | STORE-03, STORE-06, STORE-01 | PostgreSQL 连接显式固定会话设置，守卫测试直接断言会话上的取值（WR-A03） | `pg_sessions_use_the_pinned_settings` | `cargo test -p ontoforge-storage --features embedded-pg --locked --test conformance_pg pg_sessions_use_the_pinned_settings -- --ignored` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-22-T1 | SCOPE-06 | 端到端 tracer：`ontoforge migrate --apply` 遇到会破坏已 apply role 的变更时拒绝并保持旧定义 | `migrate_rechecks_applied_roles`; `migrate_with_roles_applies_when_the_roles_still_compile`; `migrate_refuses_a_change_that_breaks_an_applied_role`; `reload_reports_role_breakage` | `cargo test -p ontoforge-storage --locked --test migrate ; cargo test -p ontoforge --locked --test cli_migrate migrate_refuses_a_change_that_breaks_an_applied_role ; cargo test -p ontoforge-storage --locked --test reload` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-22-T2 | SCOPE-06 | 嵌入式 PostgreSQL 上的同一复核，与运维文档 | `pg_migrate_rechecks_applied_roles`; `migrate_rechecks_applied_roles` | `cargo test -p ontoforge-storage --features embedded-pg --locked --test migrate_pg -- --ignored` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-22-T3 | SCOPE-06 | 整对象读取 `objects.<param>` 按读取全部属性检查 allow-list（WR-C05） | `whole_object_read_needs_every_property`; `covered_criteria_compile`; `hidden_criteria_column_is_v23_at_the_action_entry`; `ungranted_object_type_is_v23`; `criteria_without_object_reads_need_no_properties`; `dynamic_object_path_is_v23_fail_closed` | `cargo test -p ontoforge-auth --locked --test criteria_coverage` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-23-T1 | AUTH-02 | 端到端 tracer：全新目录树里按示例配置 `reload` 与 `scope plan` 成功 | `example_config_reload_works_from_a_fresh_tree`; `open_creates_the_parent_directories_of_the_sqlite_file` | `cargo test -p ontoforge --locked --test cli_reload example_config_reload_works_from_a_fresh_tree ; cargo test -p ontoforge-storage --locked --test sqlite_connection open_creates_the_parent_directories_of_the_sqlite_file ; cargo test -p ontoforge --locked --test cli_reload` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-23-T2 | AUTH-02 | dev 密钥首次创建时建立上级目录，并改正 Windows ACL 的说法 | `missing_key_directory_is_created`; `example_config_dev_token_works_from_a_fresh_tree`; `rejected_arguments_leave_no_key_file_behind`; `no_key_file_anywhere_fails`; `key_file_is_created_once_and_reused`; `key_file_of_wrong_length_is_a_config_error` | `cargo test -p ontoforge-auth --locked --test dev_token ; cargo test -p ontoforge --locked --test cli_dev_token` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-23-T3 | AUTH-02 | 示例配置头注释与 docs/edit-store.md：首次使用行为、目录权限与 Windows 密钥 ACL 已知缺口 | 无命名动态测试；见结论 | `cargo test -p ontoforge --locked --test cli_reload --test cli_dev_token` | SKIP: dev 目录权限文档说明任务；行为由 T1/T2 测试，Windows ACL 既有边界不是新增验收范围。 |

## Wave 0 Closure

- [x] 所列生产模块与实际test target存在、接线完整；旧独立replay/append_only target已合并进conformance，不继续标MISSING。
- [x] SQLite/PG共享conformance、独立model、SQL差分与scope/auth corpus已执行。
- [x] CI真实服务targets发现非零后实际运行；SQLite fault显式test-support；PG source guard非ignored精确执行。
- [x] G2 TLS guard真实workflow hermetic6/6、真实dependency helper和新双平台CI完成。

## Requirement Coverage / Limits

55个task中50个可执行行为均COVERED，5个历史决策/纯文档task为SKIP，未用SKIP隐藏运行缺口。G2已闭合。STORE-05的edit侧唯一索引、基数校验及并发FK竞争已覆盖；source OneToOne基数报告仍未实现、无自动测试，故全需求PARTIAL并保持nyquist_compliant:false。ROADMAP Phase7.1 SC1/2明确安排报告与CLI退出码测试，闭合后再由该phase独立验收。

SCOPE-07的MCP管理工具限制属Phase7；此处只覆盖管理写侧与定义栅栏，不借其后继验收扩大本phase。Windows embedded PostgreSQL自行下载/启动仍无本轮实跑；真实PG后端由Linux CI及本地外部PG证明，不将feature Clippy写成Windows embedded启动成功。旧Windows SQLite lock flaky仍保留风险记录。

## Manual-Only Verifications

可选平台能力：Windows embedded PostgreSQL自动下载/启动。尚未运行，不是用人工项消除STORE-05缺口；source报告由Phase7.1实现和自动化验证。

## Validation Sign-Off

- [x] 每个task映射到行为、命名测试与可执行入口；静态任务单独说明。
- [x] 无三项连续可执行task缺少自动验证；无watch模式。
- [x] Wave0缺失引用全部按真实目标解析。
- [x] 运行证据区分ignored/list、历史CI、新CI及fresh运行。
- [ ] 全需求自动化覆盖：STORE-05 source报告仍缺；保持false。

**Approval:** validated (partial requirement coverage)。此状态来自证据审计；不代表用户逐项手工观察。原始draft未作为完成证据。

## Validation Audit 2026-11-04

| Metric | Count |
|---|---|
| Gaps found | 2 |
| Resolved | 1 |
| Escalated | 1 |
