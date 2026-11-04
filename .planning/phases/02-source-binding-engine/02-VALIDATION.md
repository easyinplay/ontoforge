---
phase: "2"
slug: "source-binding-engine"
status: validated
nyquist_compliant: false
wave_0_complete: true
created: "2026-10-16"
updated: "2026-11-04"
source_head: f8851ca073fed34e80e605f0ac91880259df7e12
---

# Phase 2 Validation

本轮已完成28个task的当前源码覆盖审计，因此从draft改为validated。26个COVERED、1个PARTIAL、1个纯文档SKIP；仍为`nyquist_compliant: false`，不能把真实local Iceberg通过当成ADR0017/02-10 PLAN要求的MinIO通过。

## Evidence and Execution Boundary

本 verifier 未执行Cargo/Docker。动态日志由同一milestone唯一runner与GitHub CI生成，本轮独立核对命名结果和源码身份。U=`target/milestone-ci-41759804733-ubuntu.log`；I=`target/milestone-ci-41759804733-integration.log`；W=`target/milestone-ci-41759804733-windows.log`。U/I:n是本轮亲查原始日志行号，全部引用实际`... ok`，不以存在、discovery或ignored当通过。最终CI head f8851ca，Windows/Ubuntu/integration/realservices四必需job均success。

当前命名映射承接 `.planning/milestones/v1.0-audit/NYQUIST-02-03-05-VALIDATION.md` 的逐task/源码审计，重新对照最终日志；原文件保留历史snapshot，不修改其他owner产物。

## Test Infrastructure

- `cargo test` + insta/proptest/wiremock，SQLite无需外部服务。
- 普通回归：`cargo test --workspace --locked -- --test-threads=1`，最终CI W/U已成功。
- 真实Trino/PG：`trino_it`与`cli_trino_it`显式`--ignored`，实际计数非零。
- 最窄Iceberg：`cargo test -p ontoforge-engine --test trino_it --locked iceberg_table_is_readable -- --ignored --exact --nocapture`；必须先准备catalog/CTAS并设两个环境变量。
- 原计划120s为热构建下的反馈目标，不冒充本次完整CI实测耗时；完整CI含编译与全suite超过120s。没有watch-mode。

## Per-Task Verification Map

| Task | Requirement | Behavior | Named tests / direct evidence | Original executable command | Status |
|---|---|---|---|---|---|
| 02-01-T1 | BIND-01 | 端到端 tracer：backing 从定义文件经 IR、校验、JSON Schema 到 `ontoforge validate`，并一次迁移全部既有 fixture | `object_without_backing_is_v05` (U:3028); `unknown_backing_value_is_v06` (U:3033); `many_to_many_without_backing_is_v05` (U:2939); `one_to_many_with_backing_is_v11` (U:2940); `many_to_many_with_backing_loads` (U:2937); `aviation_backing_values` (U:3016) | `原 PLAN 静态/决策检查，非动态测试` | COVERED |
| 02-01-T2 | BIND-01 | ADR-0021、ADR-0015 状态行、REQUIREMENTS traceability 与 ROADMAP 成功标准 1 对齐 | N/A：ADR-0021/0015/0022与traceability静态对账 | `原 PLAN 静态/决策检查，非动态测试` | SKIP (非运行文档任务) |
| 02-01-T3 | BIND-01 | wave 2 共用的 schema 侧前置：公开的有界读取 read_limited、ProfileName 与诊断码 V16..V20 | `loader::tests::read_limited_custom_limit` (U:2823); `loader::tests::read_limited_follows_links_only_when_asked` (U:2824); `ir::names::tests::profile_name_rules` (U:2698); `ir::names::tests::profile_name_schema_has_pattern` (U:2699) | `cargo test -p ontoforge-schema` | COVERED |
| 02-02-T1 | ENGINE-03, ENGINE-04 | 端到端 tracer：printer 生成带字面量的 SELECT，经 Engine trait 在 SQLite 替身上执行并回读 | `printed_literal_round_trips_through_sqlite` (U:2239); `nul_in_string_is_print_error` (U:2237); `empty_identifier_is_print_error` (U:2226); `status_is_available` (U:2244); `error_codes_are_upper_snake` (U:2228); `engine_wide_classification` (U:2227); `query_context_defaults` (U:2240); `describe_tables_rejects_mixed_catalogs` (U:2224) | `cargo test -p ontoforge-engine --test sqlite_engine` | COVERED |
| 02-02-T2 | ENGINE-03, ENGINE-04 | printer 补齐 Phase 2 AST 子集与方言差异，PhysicalType 解析，SQLite DDL 文本 | `printer::tests::print_trino_and_sqlite` (U:1991); `value::tests::physical_type_parse_table` (U:2016); `sqlite::tests::sqlite_ddl_round_trip` (U:2002); `sqlite::tests::max_rows_truncates` (U:2000); `sqlite::tests::max_bytes_truncates` (U:1999); `value::tests::split_decimal_rules` (U:2017); `invalid_literals_are_rejected` (U:2149); `i64_min_is_printed_without_overflow` (U:2147); `i64_min_reads_back_as_integer_on_sqlite` (U:2148) | `cargo test -p ontoforge-engine` | COVERED |
| 02-02-T3 | ENGINE-03, ENGINE-04 | proptest 注入性质测试与「SQL 只来自 printer」源码级守护 | `string_literal_keeps_statement_structure` (U:2162); `string_literal_round_trips_on_sqlite` (U:2163); `identifier_round_trips_on_sqlite` (U:2160); `ansi_dialect_has_no_backslash_escape` (U:2159); `sql_text_only_from_printer` (U:2212) | `cargo test -p ontoforge-engine --test printer_prop --test sql_text_only_from_printer` | COVERED |
| 02-03-T1 | BIND-01, BIND-02 | 端到端 tracer：一个 Flight 的 SourceBinding 文件经 IR、loader、校验到 `ontoforge validate` | `flight_binding_loads_under_dev_profile` (U:2880); `binding_outside_bindings_dir_is_v16` (U:2870); `definition_inside_bindings_dir_is_v16` (U:2874); `bad_profile_dir_is_v16` (U:2868); `duplicate_binding_is_v08` (U:2875); `unknown_target_is_v09` (U:2899); `fixtures_dir_is_not_scanned` (U:2879); `top_level_fixtures_dir_still_loaded` (U:2893); `ir::object::tests::effective_decimal_defaults` (U:2705); `bound_columns_object_and_link` (U:2871); `loader::discover::tests::fixture_path_rules` (U:2796); `aviation_yaml_and_json_produce_identical_ir` (U:2967) | `cargo test -p ontoforge-schema ; cargo test -p ontoforge --test cli --test cli_schema --test invalid_examples` | COVERED |
| 02-03-T2 | BIND-01, BIND-02 | profile 选定与绑定完备性：`--profile` / ONTOFORGE_PROFILE / defaultProfile、check_profile、全部 aviation 绑定 | `source_type_without_binding_is_v16` (U:2890); `edit_only_type_with_binding_is_v16` (U:2876); `one_to_many_link_binding_is_v16` (U:2884); `many_to_many_without_binding_is_v16` (U:2882); `no_profile_selected_is_v16` (U:2883); `all_edit_only_needs_no_profile` (U:2867); `profile_precedence` (U:1325); `aviation_is_clean_under_dev` (U:1322); `unknown_profile_from_flag_is_v16_with_suggestion` (U:2897); `unknown_profile_from_manifest_points_at_default_profile` (U:2898) | `cargo test -p ontoforge-schema --test bindings && cargo test -p ontoforge --test cli --test cli_lint --test invalid_examples` | COVERED |
| 02-03-T3 | BIND-01, BIND-02 | 结构性校验收口：列映射、物理标识符、fixture 引用、timezone 取值、precision / scale 位置、MaintenanceRecord.cost 与 invalid 样例 | `column_key_unknown_property_is_v09` (U:2872); `link_columns_need_from_and_to` (U:2881); `physical_identifier_rules` (U:2886); `fixture_reference_rules` (U:2878); `oversized_fixture_is_v16` (U:2885); `loader::fixture::tests::fixture_header_rules` (U:2804); `timezone_value_rules` (U:2892); `precision_on_non_decimal_is_v06` (U:2887); `aviation_cost_property` (U:2962) | `原 PLAN 静态/决策检查，非动态测试` | COVERED |
| 02-04-T1 | ENGINE-01, ENGINE-02 | 端到端 tracer：`SELECT 1` 的 PrintedSql 经 TrinoEngine 走完 POST → GET nextUri → QueryOutput（wiremock） | `select_one_round_trip` (U:2271); `request_headers` (U:2269); `data_spans_pages` (U:2256); `foreign_next_uri_is_rejected` (U:2265); `query_error_maps_to_query` (U:2268); `set_session_header_is_ignored` (U:2272); `bad_trace_token_is_config_error` (U:2253) | `cargo test -p ontoforge-engine --test trino_client ; cargo clippy -p ontoforge-engine --all-targets -- -D warnings && cargo test -p ontoforge-engine --test sql_text_only_from_printer` | COVERED |
| 02-04-T2 | ENGINE-01, ENGINE-02 | ontoforge-server.toml、密钥引用、三种认证、自定义 CA 与两个服务身份 | `parses_full_example` (U:2184); `plaintext_password_is_rejected` (U:2185); `unknown_key_is_rejected` (U:2205); `basic_over_http_is_rejected` (U:2172); `library_rejects_credentials_over_http` (U:2177); `library_rejects_cross_scheme_origin` (U:2178); `next_uri_scheme_mismatch_is_rejected` (U:2267); `profile_key_must_be_profile_name` (U:2188); `query_context_from_config` (U:2189); `basic_requires_exactly_one_secret_ref` (U:2173); `secret_from_env` (U:2192); `secret_from_file_follows_symlink_and_trims` (U:2193); `secret_never_in_debug_or_errors` (U:2194); `trino::headers::tests::basic_and_bearer_headers` (U:2012); `two_identities_use_separate_clients` (U:2275); `ca_file_is_loaded` (U:2174) | `cargo test -p ontoforge-engine --test server_config --test trino_client` | COVERED |
| 02-04-T3 | ENGINE-01, ENGINE-02 | 完整值解码、按 catalog 批量的 describe_tables（information_schema）与 status() 探测 | `decode_types` (U:2258); `describe_tables_batches_one_catalog` (U:2262); `describe_tables_lowercases_names` (U:2263); `describe_missing_tables_absent` (U:2259); `describe_truncated_metadata_is_error` (U:2264); `describe_mixed_catalogs_is_config_error` (U:2260); `status_available_and_unavailable` (U:2273) | `cargo test -p ontoforge-engine` | COVERED |
| 02-05-T1 | BIND-03, ENGINE-04 | 端到端 tracer：`ontoforge validate --online` 经 SQLite 替身比对 aviation 的全部绑定，不匹配定位到绑定文件 | `online_aviation_is_clean` (U:1449); `online_missing_explicit_column` (U:1452); `online_missing_implicit_column` (U:1453); `bind_check::tests::online_missing_table` (U:1980); `bind_check::tests::online_type_family_mismatch` (U:1981); `online_binding_without_fixture_is_v20` (U:1450); `online_missing_column_human` (U:1451); `offline_validate_unchanged` (U:1448); `query_error_isolated_to_its_catalog` (U:2025); `engine_wide_error_aborts_round` (U:2024); `online::tests::query_context_uses_defaults_and_timeout_flag` (U:1293) | `cargo test -p ontoforge --test cli_online && cargo test -p ontoforge-engine bind_check && cargo test -p ontoforge-engine --test bind_check_isolation ; cargo test -p ontoforge-engine --test sql_text_only_from_printer && cargo clippy -p ontoforge -p ontoforge-engine --all-targets -- -D warnings` | COVERED |
| 02-05-T2 | BIND-03, ENGINE-04 | fixture 语义补齐（JSON、NULL、按类型严格转换、decimal 不舍入、时区、数组、上限）与 aviation 演示数据 | `csv_empty_cell_is_null_for_every_type` (U:2222); `json_fixture_loads` (U:2236); `decimal_boundaries_12_2` (U:2223); `integer_ranges` (U:2235); `booleans_and_dates` (U:2221); `fixture_error_has_line` (U:2230); `attach_limit` (U:2220); `sqlite::tests::query_only_after_load` (U:2001); `shared_table_loaded_once` (U:2243); `shared_table_conflicts_are_errors` (U:2242); `sqlite::tests::total_fixture_bytes_capped` (U:2004); `aviation_fixture_data` (U:1445); `aviation_json_fixtures_match_yaml_fixtures` (U:1446) | `cargo test -p ontoforge-engine --test sqlite_engine && cargo test -p ontoforge --test cli_online` | COVERED |
| 02-06-T1 | ENGINE-01, ENGINE-02 | 端到端 tracer：Trino 不可达经错误分类成为 `EngineError::Unavailable`，`status()` 报告不可用 | `closed_port_is_unavailable` (U:2315); `dns_failure_is_unavailable` (U:2317); `server_starting_up_is_unavailable` (U:2337); `unauthorized_is_not_unavailable` (U:2341); `syntax_error_is_query` (U:2339); `status_reports_unavailable` (U:2338) | `cargo test -p ontoforge-engine --test trino_resilience` | COVERED |
| 02-06-T2 | ENGINE-01, ENGINE-02 | 协议规定的重试：429 Retry-After、GET 的 502 / 503 / 504 退避、POST 只在可证明未被接受时重试 | `get_503_then_success` (U:2323); `get_502_and_504_are_retried` (U:2322); `exhausted_503_is_unavailable` (U:2319); `retry_after_is_honoured` (U:2335); `post_502_is_not_retried` (U:2330); `post_503_starting_up_is_retried` (U:2331); `post_plain_503_is_not_retried` (U:2332); `retries_respect_timeout` (U:2333) | `cargo test -p ontoforge-engine --test trino_resilience` | COVERED |
| 02-06-T3 | ENGINE-01, ENGINE-02 | 三个取消触发点（超时、断连、行数 / 字节上限）与单页大小上限 | `timeout_sends_delete` (U:2340); `dropped_future_sends_delete` (U:2318); `finished_query_sends_no_delete` (U:2321); `max_rows_truncates_and_deletes` (U:2328); `max_bytes_truncates_and_deletes` (U:2327); `oversized_page_is_protocol_error` (U:2329); `trino::cancel::tests::dropping_an_unfinished_guard_without_a_runtime_does_not_panic` (U:2006) | `cargo test -p ontoforge-engine --test trino_resilience --test trino_client` | COVERED |
| 02-07-T1 | BIND-02 | 端到端 tracer：绑定缺 timezone → ONT-E14 → `ontoforge lint` 定位到绑定文件 | `lint::binding::e14::tests::e14_multi_profile` (U:2713); `e14_human` (U:1412); `e14_can_be_disabled` (U:1411); `aviation_lints_clean` (U:2955) | `cargo test -p ontoforge-schema lint:: && cargo test -p ontoforge-schema --test lint_examples && cargo test -p ontoforge --test cli_lint` | COVERED |
| 02-07-T2 | BIND-02 | ONT-E13 decimal 精度、ONT-E15 collation、ONT-E16 geopoint | `lint::binding::e13::tests::e13_can_be_downgraded` (U:2707); `lint::binding::e13::tests::triggers` (U:2708); `lint::binding::e15::tests::triggers` (U:2717); `lint::binding::e16::tests::triggers` (U:2719) | `cargo test -p ontoforge-schema lint:: && cargo test -p ontoforge-schema --test lint_examples` | COVERED |
| 02-07-T3 | BIND-02 | 加载期捕获 fixture 表头（计入总预算）与 ONT-W17（D-03 的离线缺列提示，表头不可读时不静默） | `loader::tests::fixture_header_is_captured` (U:2821); `lint::binding::w17::tests::w17_unreadable_header_warns` (U:2724); `loader::tests::fixture_bytes_count_toward_budget` (U:2820); `lint::binding::w17::tests::w17_off` (U:2723); `aviation_lints_clean` (U:2955) | `cargo test -p ontoforge-schema && cargo test -p ontoforge --test cli_lint` | COVERED |
| 02-08-T1 | BIND-03, BIND-02 | 端到端 tracer：fixture 缺列经 SQLite 替身与 check_drift 使 Flight 降级为 SourceSchemaDrift，其余类型健康 | `aviation_is_healthy` (U:2040); `missing_column_degrades_only_flight` (U:2043); `bind_check::tests::warnings_do_not_degrade` (U:1982); `warnings_do_not_degrade` (U:2047); `query_error_degrades_only_that_target` (U:2045); `engine_error_is_not_a_report` (U:2042); `degrade_reason_code` (U:2041) | `cargo test -p ontoforge-engine --test drift` | COVERED |
| 02-08-T2 | BIND-03, BIND-02 | D-13 完整无损映射表、decimal 整数位规则、timezone 复核、char warning、绑定列大小写歧义 error、link 列比对 | `bind_check::compat_tests::case_collision` (U:1964); `bind_check::compat_tests::decimal_integer_digits_and_scale` (U:1965); `bind_check::compat_tests::timestamps_are_checked_against_the_declared_zone` (U:1971); `bind_check::compat_tests::integers_only_widen` (U:1968); `bind_check::compat_tests::arrays_are_compared_by_element` (U:1963); `bind_check::compat_tests::link_ends_are_compared_with_the_primary_keys` (U:1969) | `cargo test -p ontoforge-engine bind_check && cargo test -p ontoforge-engine --test drift ; cargo test -p ontoforge --test cli_online` | COVERED |
| 02-08-T3 | BIND-03, BIND-02 | 绑定规范文档 docs/source-binding.md 与文档类型表一致性测试 | `doc_type_table_matches_compare_binding` (U:2033) | `cargo test -p ontoforge-engine --test doc_type_table` | COVERED |
| 02-09-T1 | BIND-03, ENGINE-01, ENGINE-02 | 端到端 tracer：`--server-config` 把 dev 映射到 Trino，CLI 经 TrinoEngine（wiremock）完成在线校验；不可用退出码 3，配置错误定位到配置文件 | `trino_profile_clean` (U:1473); `trino_profile_missing_column` (U:1474); `trino_type_mismatch` (U:1475); `trino_missing_table` (U:1472); `one_catalog_query_error_is_v19_others_checked` (U:1466); `timeout_flag_is_sent` (U:1471); `trino_unavailable_exits_3` (U:1476); `unauthorized_exits_3` (U:1477); `secret_never_printed` (U:1468); `config_errors_are_v20_with_location` (U:1464); `check_pk_requires_online` (U:1463); `sqlite_profile_uses_fixtures` (U:1469) | `cargo test -p ontoforge --test cli_online_trino ; cargo test -p ontoforge --test cli_online --test cli` | COVERED |
| 02-09-T2 | BIND-03, ENGINE-01, ENGINE-02 | `--check-pk`（重复值查询排除 NULL、NULL 单独计数）与退出码表 | `pk_queries_are_printed` (U:2137); `duplicates_and_nulls_found` (U:2135); `nulls_are_not_duplicates` (U:2136); `check_pk_clean_on_aviation` (U:1336); `check_pk_reports_duplicates` (U:1339); `check_pk_reports_null_keys` (U:1340); `check_pk_on_trino` (U:1338); `exit_code_table` (U:1343) | `cargo test -p ontoforge-engine --test pk_check && cargo test -p ontoforge --test cli_check_pk` | COVERED |
| 02-10-T1 | ENGINE-05, ENGINE-01, ENGINE-03 | 端到端 tracer：PR 的 integration 作业起 PostgreSQL + Trino 483、种入 aviation 数据，一条 `--ignored` 测试在真实 Trino 上查询并描述表 | `select_one` (I:1047); `describe_demo_flights` (I:1044); `missing_env_panics` (U:1555) | `cargo test -p ontoforge-engine --test trino_it --locked -- --ignored` | COVERED |
| 02-10-T2 | ENGINE-05, ENGINE-01, ENGINE-03 | 真实 Trino 上的 golden、printer 固定语料、超时取消、catalog 可见性与 CLI 端到端；执行数量门禁 | `describe_types_golden` (I:1046); `decode_golden` (I:1049); `printer_corpus_round_trips` (I:1051); `timeout_cancels_server_query` (I:1050); `catalogs_visible` (I:1043); `drift_is_healthy_on_ci_trino` (I:1048); `validate_online_against_ci_trino` (I:1086) | `cargo test -p ontoforge-engine --test trino_it --locked -- --ignored ; cargo test -p ontoforge --test cli_trino_it --locked -- --ignored` | COVERED |
| 02-10-T3 | ENGINE-05, ENGINE-01, ENGINE-03 | nightly Iceberg 工作流（483 / 484，失败开 issue）、私有 fixture 入口与 CONTRIBUTING 说明 | `a_set_needs_ontology_and_server_config` (U:1597)；local Iceberg实跑已补齐，MinIO/S3仍缺实现与执行证据 | `cargo test -p ontoforge --test private_fixtures --locked ; cargo test -p ontoforge-engine --test trino_it --locked iceberg_table_is_readable -- --ignored --exact --nocapture (G1 环境)` | PARTIAL (MinIO规范偏差未决) |

## Wave 0 Requirements

- [x] engine crate与workspace依赖、测试目录已实现并由CI编译执行。
- [x] schema bindings测试与invalid样例存在且运行。
- [x] cli_online与bind_check_isolation真实调用SQLite/wiremock并运行。
- [x] ci/trino/catalog、seed、nightly文件存在并接线；MinIO特定合同另列PARTIAL。
- [x] backing迁移与当前定义/快照回归覆盖。

## Gap and Manual Boundary

**G1运行证据缺口已关闭（local路径）**：`target/milestone-audit-evidence-20261104/trino-{483,484}-ctas.log`每版为`CREATE TABLE: 29 rows`，fixture日志记录`ONTOFORGE_TRINO_ICEBERG=1`，对应`trino-*-iceberg.log`各`1 passed; 0 failed; 0 ignored`。亲读测试：非空columns、source非0、copied/source完全相等。普通integration的同名ok提前return，不计入本证明。

**仍开放的02-10-T3**：accepted ADR0017与PLAN要求MinIO，当前`ci/trino/iceberg/iceberg.properties`是JDBC+local warehouse。未有accepted修订批准替代，未运行MinIO/S3。独立盲评 `target/iceberg-fixture-independent-opinion.json`结论也是规范偏差与覆盖缺口，非已证产品bug。最小收口是执行受信MinIO路径的483/484实际验收，或需求所有者明确接受后以正式决策修订此历史合同；本报告不代做决定。

公开fixture不含公司数据属judgment-tier：当前源码与隔离fixture支持non-authoritative判断，用户授权自动比较意见不等于人工核验过数据来源。保留`unverified-prohibition — human review recommended`；未访问真实公司SQL Server，不新增运行声明。

## Validation Sign-Off

- [x] 28个task都有执行证据或明确非运行理由。
- [x] 27个运行/CI task均有命名测试与接线，唯MinIO覆盖仍PARTIAL。
- [x] Wave 0原“missing”引用已核对，现有代码可运行。
- [x] Windows/Ubuntu与真实Trino/PG最终CI通过。
- [ ] ADR0017/02-10-T3的MinIO合同闭环。
- [ ] `nyquist_compliant: true`，需上述缺口关闭后重验。

## Validation Audit 2026-11-04

| Metric | Count |
|---|---|
| Tasks audited | 28 |
| Covered | 26 |
| Partial | 1 |
| Non-runtime skip | 1 |
| Current local Iceberg versions passed | 2 |
| Unresolved MinIO contract | 1 |
