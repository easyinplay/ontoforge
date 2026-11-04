---
status: mapping-partial
scope: historical-nyquist-02-03-05
snapshot: adc02278b5cc55111def9a30a1c3b778485e8279
ci_run: 41669982375
created: 2026-11-04
original_validation_files_modified: false
new_tests_run: false
---

# Phase 2 / 3 / 5 Nyquist 审计映射

本文件只做历史 phase 的当前证据映射，不宣告新 HEAD 整体验证通过。GSD `validate-phase` 的 discovery/cross-reference/gap analysis 已执行；原 Phase VALIDATION、测试与实现均未修改，未启动 Cargo、数据库或服务，未提交。当前另有 INT-01 MCP catalog 修复进行中，最终源码必须另行建立验证证据。

## 证据口径

- 读取 40 个 PLAN、40 个 SUMMARY、3 个 VALIDATION、3 个 VERIFICATION，逐 task 提取 requirement、行为、验证命令与偏差。项目 `.codex/skills`、`.agents/skills` 不存在；GSD `agent-skills gsd-nyquist-auditor --skills-only` 无注入项。
- `COVERED` 表示 task 行为已有对应命名测试，所列冻结源码日志明确 `... ok`。测试文件存在、旧 SUMMARY 宣称通过、`--list`、`ignored`、默认退出码 0 均不独立计为通过。
- 这些是复用 CI 的动态证据，不是本 subagent 新跑的测试。CI checkout 为 merge `b75d294`，日志写明将 `adc02278b5cc55111def9a30a1c3b778485e8279` merge 入 base；不得写成当前本地 HEAD 的运行结果。
- W=`target/phase7-ci-41669982375-windows.log`；U=`target/phase7-ci-41669982375-ubuntu.log`；I=`target/phase7-ci-41669982375-integration.log`；S=`target/phase7-ci-41669982375-services.log`。冒号后为原始日志行号。每个测试的源文件与证据索引在文末。
- `target/phase7-services/evidence/final-adc0227-verify.log` 已读，属于 Phase 7 fresh 服务证据；它没有下列 G3/G4 命名测试，不能用 fresh 总数补 Phase 5 缺口。
- Unix-only 权限/符号链接用 U 证明，不把 Windows 未编译这些测试判为失败；PG 必须以 I 中实际 `ok` 为准。
- COVERED 是所述 phase 行为覆盖结论，不表示库内所有可能行为均穷尽证明。性质测试依赖有限样本；03-18 既有 Windows SQLite lock flaky 记录保留 WARNING，当前 W 中该测试通过不抹掉历史风险。

## 汇总

| Phase | Tasks | COVERED | PARTIAL | MISSING | SKIP（非运行行为） |
|---|---:|---:|---:|---:|---:|
| 02 | 28 | 26 | 1 | 0 | 1 |
| 03 | 55 | 49 | 1 | 0 | 5 |
| 05 | 14 | 12 | 2 | 0 | 0 |
| 合计 | 97 | 87 | 4 | 0 | 6 |

6 个 SKIP 是历史决策/纯文档 task，不是把未验证产品行为转成 manual-only。4 个 PARTIAL 是证据/测试支路缺口，尚无本轮执行失败，不能伪造 BLOCKER 或「已修复」。其中 G2 需要增加负向 guard 行为测试，其余先运行已存在测试或取得有效 nightly 原始日志。

## 必要补证及失败判据

| Gap | Task / Requirement | 当前真实证据 | 最小后续动作 | fails_when / 状态 |
|---|---|---|---|---|
| G1 | 02-10-T3 / ENGINE-05 | I:1018 的 `iceberg_table_is_readable ... ok`，但 `crates/ontoforge-engine/tests/trino_it.rs:626` 在 `ONTOFORGE_TRINO_ICEBERG != 1` 时直接 return；当前 integration 未设该变量。旧 nightly 39221684753 不证明当前源码。 | 在最终源码上运行或取得 `.github/workflows/nightly.yml` 的 483/484 两矩阵日志；必须证明配置 `ONTOFORGE_TRINO_ICEBERG=1` 且 `iceberg_table_is_readable` 运行。 | 任一矩阵失败、0 测试、没启用 Iceberg、列集为空、源表为空或复制行数不一致均未通过。WARNING / PARTIAL。 |
| G2 | 03-01-T3 / STORE-01, AUTH-01（供应链 guard） | U:846-847、W:830-831 显示 openssl/native-tls 当前不存在。`.github/workflows/ci.yml` 的两行 `! cargo tree ...` 延续旧 CR-B01；当前树无匹配不能证明发现 openssl 时 job 会失败。 | 新增 hermetic shell/CI guard 测试，用 stub cargo 令 openssl 查询 exit 0、native-tls 查询 exit 1，执行现有 bash guard 并断言其最终 exit 非 0；另覆盖两者均无匹配。交主会话处理，当前未执行或修改。 | 不允许两条否定命令中前一条失败被后一条成功掩盖。WARNING / PARTIAL；本轮未运行负向测试，未将静态发现升级为已实测 BLOCKER。 |
| G3 | 05-04-T1 / ACTION-04,08,09 | I:761-765 真实 PG 五项 `action_atomic_*` 通过；W/U 普通 workspace 未启用 `test-support`，SQLite 五项未出现在日志。 | `cargo test -p ontoforge-storage --features test-support --test conformance_sqlite --locked action_atomic_ -- --nocapture`，由唯一 Cargo owner 串行执行。 | 必须实际 5 passed；after_log/edits/command/outbox 任一故障留下 edit/log/history/command/outbox 写入，或成功路径重复写入即失败。WARNING / PARTIAL。 |
| G4 | 05-06-T1 / ACTION-08,09,12 | `crates/ontoforge-action/tests/source.rs:629` 的 `source_guard_absent_target_insert_race` 仅 `cfg(feature = "embedded-pg")`，没有 `#[ignore]`；W/U/I/S 均无运行记录。 | `cargo test -p ontoforge-action --features embedded-pg --test source --locked source_guard_absent_target_insert_race -- --exact --nocapture`，必须不带 `--ignored`。 | 必须实际 1 passed；独立连接插入后 Action 未返回 VersionConflict、Engine 次数不等于 1、增加 Action receipt/outbox、日志不只含并发写入均失败。WARNING / PARTIAL。 |

### G4 现有 fixture 与 Windows 运行条件

`crates/ontoforge-storage/tests/support/mod.rs:269` 的 `pg_url()` 优先使用非空 `ONTOFORGE_PG_URL`；只有缺失/空值才启动 embedded PostgreSQL。因此 Windows 可复用后续 fresh PostgreSQL，必要环境变量仅 `ONTOFORGE_PG_URL`，不需要 Trino。测试使用 CountingEngine，`embedded-pg` feature 只是打开 cfg 并传递 test-support。账号须能创建测试 schema；fixture 生成独立 ULID schema pair，使用第二连接制造真实竞态。不要打印连接串或环境值。

### G1 有效 Iceberg 入口

nightly 的 `iceberg` job 对 Trino 483、484 分别启动 PostgreSQL、建立 `demo`/`editstore`/`iceberg_catalog` 数据库并执行 `ci/pg/iceberg_catalog.sql`，合并 `ci/trino/catalog/*.properties` 与 `ci/trino/iceberg/*.properties`，挂载 `/tmp/iceberg` 本地 warehouse，再 CTAS `iceberg.aviation.flights`。`Engine integration tests (with Iceberg)` 同时设置 `ONTOFORGE_TRINO_URL` 和 `ONTOFORGE_TRINO_ICEBERG=1`，执行 `cargo test -p ontoforge-engine --test trino_it --locked -- --ignored`。专用最窄命令是 `cargo test -p ontoforge-engine --test trino_it --locked iceberg_table_is_readable -- --ignored --exact --nocapture`，但裸设变量不能替代以上真实 catalog/数据准备。检查 483/484 两份日志的 effective env、命名测试与实际计数。本 subagent 没有触发工作流。

## metadata draft 与真正覆盖缺口

三个原 VALIDATION 都是 `status: draft`、`nyquist_compliant: false`、`wave_0_complete: false`；旧 W0 的文件不存在/待创建标记多数已经失真。Phase 3 原表仅到 03-20，未纳入 03-21..23 gap closure；Phase 5 用 plan 级粗表而没有 task 级证据。应在 G1..G4 处理后回填映射与审计状态，不能只改 frontmatter 变绿。

已经核对的命名差异不属于 MISSING：`unknown_profile_is_v16` 拆为 flag/manifest 两测试；`delete_without_runtime_is_skipped` 对应 `dropping_an_unfinished_guard_without_a_runtime_does_not_panic`；`replay_after_sequence` 在 `model_matches_store` 内实际逐列比较；Phase 5 的多数名称加 `scope_`、`batch_`、`token_`、`action_batch_` 前缀。default 与后项失败回滚在 `batch_ordinal_and_same_pk_order_are_stable` 中有实际断言；legacy/token篡改在 `token_conflicts_after_edit_scope_or_key_change` 和 malformed 测试中验证，不以名称不一致虚报缺失。

Phase 2 私有 SQL Server fixture 是仓库外可选入口，其缺变量与目录发现 harness 可验证，真实公司数据没有本轮证据。Phase 3 Windows embedded PostgreSQL 启动的旧 manual check 仍无本次实跑证据，Linux 真 PG + Windows feature clippy 不能证明 Windows embedded 启动；这是环境 caveat，不改写 STORE-01 的双后端运行证据。

Phase 3 SCOPE-07 的读侧版本固定/MCP 无变更工具在原 VERIFICATION 中移交 Phase 6/7，不能用 Phase 3 写侧栅栏测试代替其最终验证。Phase 5 生产 webhook 实际配置/服务管理器/真实下游联调保持发布前 DEFERRED；本轮没有部署授权，没有部署，也不将其扩为新业务范围。`cli_webhook_worker`、wiremock、PG outbox 证明开发合同，不能声称生产外部接收已验证。

## 逐 task requirement → test → command

名称带完整 module 时保持原名；无 module 的名称在文末索引给真实源文件与 CI 行号。测试列表来自 PLAN 行为名与已核对的重命名/合并实现；`missing_env_panics` 这类同名测试的候选源路径全部保留，须结合 target 判断，不能用别 crate 同名测试替代。

### Phase 02

| Task | Requirement | 行为 | 命名测试 | 可执行命令 | 结论 |
|---|---|---|---|---|---|
| 02-01-T1 | BIND-01 | 端到端 tracer：backing 从定义文件经 IR、校验、JSON Schema 到 `ontoforge validate`，并一次迁移全部既有 fixture | `object_without_backing_is_v05`; `unknown_backing_value_is_v06`; `many_to_many_without_backing_is_v05`; `one_to_many_with_backing_is_v11`; `many_to_many_with_backing_loads`; `aviation_backing_values` | `原 PLAN 静态/决策检查，非动态测试` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 02-01-T2 | BIND-01 | ADR-0021、ADR-0015 状态行、REQUIREMENTS traceability 与 ROADMAP 成功标准 1 对齐 | 无命名动态测试；见结论 | `原 PLAN 静态/决策检查，非动态测试` | SKIP: 历史 ADR/REQUIREMENTS/ROADMAP 对账任务，无独立运行行为；BIND-01 由同 plan T1/T3 验证。非以 manual-only 消除行为 gap。 |
| 02-01-T3 | BIND-01 | wave 2 共用的 schema 侧前置：公开的有界读取 read_limited、ProfileName 与诊断码 V16..V20 | `loader::tests::read_limited_custom_limit`; `loader::tests::read_limited_follows_links_only_when_asked`; `ir::names::tests::profile_name_rules`; `ir::names::tests::profile_name_schema_has_pattern` | `cargo test -p ontoforge-schema` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 02-02-T1 | ENGINE-03, ENGINE-04 | 端到端 tracer：printer 生成带字面量的 SELECT，经 Engine trait 在 SQLite 替身上执行并回读 | `printed_literal_round_trips_through_sqlite`; `nul_in_string_is_print_error`; `empty_identifier_is_print_error`; `status_is_available`; `error_codes_are_upper_snake`; `engine_wide_classification`; `query_context_defaults`; `describe_tables_rejects_mixed_catalogs` | `cargo test -p ontoforge-engine --test sqlite_engine` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 02-02-T2 | ENGINE-03, ENGINE-04 | printer 补齐 Phase 2 AST 子集与方言差异，PhysicalType 解析，SQLite DDL 文本 | `printer::tests::print_trino_and_sqlite`; `value::tests::physical_type_parse_table`; `sqlite::tests::sqlite_ddl_round_trip`; `sqlite::tests::max_rows_truncates`; `sqlite::tests::max_bytes_truncates`; `value::tests::split_decimal_rules`; `invalid_literals_are_rejected`; `i64_min_is_printed_without_overflow`; `i64_min_reads_back_as_integer_on_sqlite` | `cargo test -p ontoforge-engine` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 02-02-T3 | ENGINE-03, ENGINE-04 | proptest 注入性质测试与「SQL 只来自 printer」源码级守护 | `string_literal_keeps_statement_structure`; `string_literal_round_trips_on_sqlite`; `identifier_round_trips_on_sqlite`; `ansi_dialect_has_no_backslash_escape`; `sql_text_only_from_printer` | `cargo test -p ontoforge-engine --test printer_prop --test sql_text_only_from_printer` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 02-03-T1 | BIND-01, BIND-02 | 端到端 tracer：一个 Flight 的 SourceBinding 文件经 IR、loader、校验到 `ontoforge validate` | `flight_binding_loads_under_dev_profile`; `binding_outside_bindings_dir_is_v16`; `definition_inside_bindings_dir_is_v16`; `bad_profile_dir_is_v16`; `duplicate_binding_is_v08`; `unknown_target_is_v09`; `fixtures_dir_is_not_scanned`; `top_level_fixtures_dir_still_loaded`; `ir::object::tests::effective_decimal_defaults`; `bound_columns_object_and_link`; `loader::discover::tests::fixture_path_rules`; `aviation_yaml_and_json_produce_identical_ir` | `cargo test -p ontoforge-schema ; cargo test -p ontoforge --test cli --test cli_schema --test invalid_examples` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 02-03-T2 | BIND-01, BIND-02 | profile 选定与绑定完备性：`--profile` / ONTOFORGE_PROFILE / defaultProfile、check_profile、全部 aviation 绑定 | `source_type_without_binding_is_v16`; `edit_only_type_with_binding_is_v16`; `one_to_many_link_binding_is_v16`; `many_to_many_without_binding_is_v16`; `no_profile_selected_is_v16`; `all_edit_only_needs_no_profile`; `profile_precedence`; `aviation_is_clean_under_dev`; `unknown_profile_from_flag_is_v16_with_suggestion`; `unknown_profile_from_manifest_points_at_default_profile` | `cargo test -p ontoforge-schema --test bindings && cargo test -p ontoforge --test cli --test cli_lint --test invalid_examples` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 02-03-T3 | BIND-01, BIND-02 | 结构性校验收口：列映射、物理标识符、fixture 引用、timezone 取值、precision / scale 位置、MaintenanceRecord.cost 与 invalid 样例 | `column_key_unknown_property_is_v09`; `link_columns_need_from_and_to`; `physical_identifier_rules`; `fixture_reference_rules`; `oversized_fixture_is_v16`; `loader::fixture::tests::fixture_header_rules`; `timezone_value_rules`; `precision_on_non_decimal_is_v06`; `aviation_cost_property` | `原 PLAN 静态/决策检查，非动态测试` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 02-04-T1 | ENGINE-01, ENGINE-02 | 端到端 tracer：`SELECT 1` 的 PrintedSql 经 TrinoEngine 走完 POST → GET nextUri → QueryOutput（wiremock） | `select_one_round_trip`; `request_headers`; `data_spans_pages`; `foreign_next_uri_is_rejected`; `query_error_maps_to_query`; `set_session_header_is_ignored`; `bad_trace_token_is_config_error` | `cargo test -p ontoforge-engine --test trino_client ; cargo clippy -p ontoforge-engine --all-targets -- -D warnings && cargo test -p ontoforge-engine --test sql_text_only_from_printer` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 02-04-T2 | ENGINE-01, ENGINE-02 | ontoforge-server.toml、密钥引用、三种认证、自定义 CA 与两个服务身份 | `parses_full_example`; `plaintext_password_is_rejected`; `unknown_key_is_rejected`; `basic_over_http_is_rejected`; `library_rejects_credentials_over_http`; `library_rejects_cross_scheme_origin`; `next_uri_scheme_mismatch_is_rejected`; `profile_key_must_be_profile_name`; `query_context_from_config`; `basic_requires_exactly_one_secret_ref`; `secret_from_env`; `secret_from_file_follows_symlink_and_trims`; `secret_never_in_debug_or_errors`; `trino::headers::tests::basic_and_bearer_headers`; `two_identities_use_separate_clients`; `ca_file_is_loaded` | `cargo test -p ontoforge-engine --test server_config --test trino_client` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 02-04-T3 | ENGINE-01, ENGINE-02 | 完整值解码、按 catalog 批量的 describe_tables（information_schema）与 status() 探测 | `decode_types`; `describe_tables_batches_one_catalog`; `describe_tables_lowercases_names`; `describe_missing_tables_absent`; `describe_truncated_metadata_is_error`; `describe_mixed_catalogs_is_config_error`; `status_available_and_unavailable` | `cargo test -p ontoforge-engine` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 02-05-T1 | BIND-03, ENGINE-04 | 端到端 tracer：`ontoforge validate --online` 经 SQLite 替身比对 aviation 的全部绑定，不匹配定位到绑定文件 | `online_aviation_is_clean`; `online_missing_explicit_column`; `online_missing_implicit_column`; `bind_check::tests::online_missing_table`; `bind_check::tests::online_type_family_mismatch`; `online_binding_without_fixture_is_v20`; `online_missing_column_human`; `offline_validate_unchanged`; `query_error_isolated_to_its_catalog`; `engine_wide_error_aborts_round`; `online::tests::query_context_uses_defaults_and_timeout_flag` | `cargo test -p ontoforge --test cli_online && cargo test -p ontoforge-engine bind_check && cargo test -p ontoforge-engine --test bind_check_isolation ; cargo test -p ontoforge-engine --test sql_text_only_from_printer && cargo clippy -p ontoforge -p ontoforge-engine --all-targets -- -D warnings` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 02-05-T2 | BIND-03, ENGINE-04 | fixture 语义补齐（JSON、NULL、按类型严格转换、decimal 不舍入、时区、数组、上限）与 aviation 演示数据 | `csv_empty_cell_is_null_for_every_type`; `json_fixture_loads`; `decimal_boundaries_12_2`; `integer_ranges`; `booleans_and_dates`; `fixture_error_has_line`; `attach_limit`; `sqlite::tests::query_only_after_load`; `shared_table_loaded_once`; `shared_table_conflicts_are_errors`; `sqlite::tests::total_fixture_bytes_capped`; `aviation_fixture_data`; `aviation_json_fixtures_match_yaml_fixtures` | `cargo test -p ontoforge-engine --test sqlite_engine && cargo test -p ontoforge --test cli_online` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 02-06-T1 | ENGINE-01, ENGINE-02 | 端到端 tracer：Trino 不可达经错误分类成为 `EngineError::Unavailable`，`status()` 报告不可用 | `closed_port_is_unavailable`; `dns_failure_is_unavailable`; `server_starting_up_is_unavailable`; `unauthorized_is_not_unavailable`; `syntax_error_is_query`; `status_reports_unavailable` | `cargo test -p ontoforge-engine --test trino_resilience` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 02-06-T2 | ENGINE-01, ENGINE-02 | 协议规定的重试：429 Retry-After、GET 的 502 / 503 / 504 退避、POST 只在可证明未被接受时重试 | `get_503_then_success`; `get_502_and_504_are_retried`; `exhausted_503_is_unavailable`; `retry_after_is_honoured`; `post_502_is_not_retried`; `post_503_starting_up_is_retried`; `post_plain_503_is_not_retried`; `retries_respect_timeout` | `cargo test -p ontoforge-engine --test trino_resilience` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 02-06-T3 | ENGINE-01, ENGINE-02 | 三个取消触发点（超时、断连、行数 / 字节上限）与单页大小上限 | `timeout_sends_delete`; `dropped_future_sends_delete`; `finished_query_sends_no_delete`; `max_rows_truncates_and_deletes`; `max_bytes_truncates_and_deletes`; `oversized_page_is_protocol_error`; `trino::cancel::tests::dropping_an_unfinished_guard_without_a_runtime_does_not_panic` | `cargo test -p ontoforge-engine --test trino_resilience --test trino_client` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 02-07-T1 | BIND-02 | 端到端 tracer：绑定缺 timezone → ONT-E14 → `ontoforge lint` 定位到绑定文件 | `lint::binding::e14::tests::e14_multi_profile`; `e14_human`; `e14_can_be_disabled`; `aviation_lints_clean` | `cargo test -p ontoforge-schema lint:: && cargo test -p ontoforge-schema --test lint_examples && cargo test -p ontoforge --test cli_lint` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 02-07-T2 | BIND-02 | ONT-E13 decimal 精度、ONT-E15 collation、ONT-E16 geopoint | `lint::binding::e13::tests::e13_can_be_downgraded`; `lint::binding::e13::tests::triggers`; `lint::binding::e15::tests::triggers`; `lint::binding::e16::tests::triggers` | `cargo test -p ontoforge-schema lint:: && cargo test -p ontoforge-schema --test lint_examples` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 02-07-T3 | BIND-02 | 加载期捕获 fixture 表头（计入总预算）与 ONT-W17（D-03 的离线缺列提示，表头不可读时不静默） | `loader::tests::fixture_header_is_captured`; `lint::binding::w17::tests::w17_unreadable_header_warns`; `loader::tests::fixture_bytes_count_toward_budget`; `lint::binding::w17::tests::w17_off`; `aviation_lints_clean` | `cargo test -p ontoforge-schema && cargo test -p ontoforge --test cli_lint` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 02-08-T1 | BIND-03, BIND-02 | 端到端 tracer：fixture 缺列经 SQLite 替身与 check_drift 使 Flight 降级为 SourceSchemaDrift，其余类型健康 | `aviation_is_healthy`; `missing_column_degrades_only_flight`; `bind_check::tests::warnings_do_not_degrade`; `warnings_do_not_degrade`; `query_error_degrades_only_that_target`; `engine_error_is_not_a_report`; `degrade_reason_code` | `cargo test -p ontoforge-engine --test drift` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 02-08-T2 | BIND-03, BIND-02 | D-13 完整无损映射表、decimal 整数位规则、timezone 复核、char warning、绑定列大小写歧义 error、link 列比对 | `bind_check::compat_tests::case_collision`; `bind_check::compat_tests::decimal_integer_digits_and_scale`; `bind_check::compat_tests::timestamps_are_checked_against_the_declared_zone`; `bind_check::compat_tests::integers_only_widen`; `bind_check::compat_tests::arrays_are_compared_by_element`; `bind_check::compat_tests::link_ends_are_compared_with_the_primary_keys` | `cargo test -p ontoforge-engine bind_check && cargo test -p ontoforge-engine --test drift ; cargo test -p ontoforge --test cli_online` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 02-08-T3 | BIND-03, BIND-02 | 绑定规范文档 docs/source-binding.md 与文档类型表一致性测试 | `doc_type_table_matches_compare_binding` | `cargo test -p ontoforge-engine --test doc_type_table` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 02-09-T1 | BIND-03, ENGINE-01, ENGINE-02 | 端到端 tracer：`--server-config` 把 dev 映射到 Trino，CLI 经 TrinoEngine（wiremock）完成在线校验；不可用退出码 3，配置错误定位到配置文件 | `trino_profile_clean`; `trino_profile_missing_column`; `trino_type_mismatch`; `trino_missing_table`; `one_catalog_query_error_is_v19_others_checked`; `timeout_flag_is_sent`; `trino_unavailable_exits_3`; `unauthorized_exits_3`; `secret_never_printed`; `config_errors_are_v20_with_location`; `check_pk_requires_online`; `sqlite_profile_uses_fixtures` | `cargo test -p ontoforge --test cli_online_trino ; cargo test -p ontoforge --test cli_online --test cli` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 02-09-T2 | BIND-03, ENGINE-01, ENGINE-02 | `--check-pk`（重复值查询排除 NULL、NULL 单独计数）与退出码表 | `pk_queries_are_printed`; `duplicates_and_nulls_found`; `nulls_are_not_duplicates`; `check_pk_clean_on_aviation`; `check_pk_reports_duplicates`; `check_pk_reports_null_keys`; `check_pk_on_trino`; `exit_code_table` | `cargo test -p ontoforge-engine --test pk_check && cargo test -p ontoforge --test cli_check_pk` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 02-10-T1 | ENGINE-05, ENGINE-01, ENGINE-03 | 端到端 tracer：PR 的 integration 作业起 PostgreSQL + Trino 483、种入 aviation 数据，一条 `--ignored` 测试在真实 Trino 上查询并描述表 | `select_one`; `describe_demo_flights`; `missing_env_panics` | `cargo test -p ontoforge-engine --test trino_it --locked -- --ignored` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 02-10-T2 | ENGINE-05, ENGINE-01, ENGINE-03 | 真实 Trino 上的 golden、printer 固定语料、超时取消、catalog 可见性与 CLI 端到端；执行数量门禁 | `describe_types_golden`; `decode_golden`; `printer_corpus_round_trips`; `timeout_cancels_server_query`; `catalogs_visible`; `drift_is_healthy_on_ci_trino`; `validate_online_against_ci_trino` | `cargo test -p ontoforge-engine --test trino_it --locked -- --ignored ; cargo test -p ontoforge --test cli_trino_it --locked -- --ignored` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 02-10-T3 | ENGINE-05, ENGINE-01, ENGINE-03 | nightly Iceberg 工作流（483 / 484，失败开 issue）、私有 fixture 入口与 CONTRIBUTING 说明 | `a_set_needs_ontology_and_server_config` | `cargo test -p ontoforge --test private_fixtures --locked ; cargo test -p ontoforge-engine --test trino_it --locked iceberg_table_is_readable -- --ignored --exact --nocapture (G1 环境)` | PARTIAL / WARNING G1: 私有 fixture harness 有通过测试；Iceberg 在 I:1018 的 ok 提前返回，缺当前源码 nightly 483/484 有效证据。 |

### Phase 03

| Task | Requirement | 行为 | 命名测试 | 可执行命令 | 结论 |
|---|---|---|---|---|---|
| 03-01-T1 | STORE-01, AUTH-01 | 人工核验 sea-query-sqlx 与 getrandom（安装前的 legitimacy 门禁） | 无命名动态测试；见结论 | `原 PLAN 静态/决策检查，非动态测试` | SKIP: 安装前人工 legitimacy checkpoint，SUMMARY 明记 approved；不能把历史批准冒充动态测试。 |
| 03-01-T2 | STORE-01, AUTH-01 | 端到端 tracer：依赖与 crate 图落地，storage 打开带 recursive_triggers 与 engine collation 的 SQLite 连接，auth 用 aws-lc-rs 往返一次 JWT | `every_pooled_connection_has_recursive_triggers`; `engine_collations_are_registered`; `foreign_keys_on`; `hs256_round_trip_does_not_panic` | `cargo test -p ontoforge-storage --test sqlite_connection && cargo test -p ontoforge-auth --test crypto_backend` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 03-01-T3 | STORE-01, AUTH-01 | 供应链守卫（无 openssl / native-tls）、ADR-0023 与设计文档依赖方向指针 | 无命名动态测试；见结论 | `cargo tree --workspace --locked --features ontoforge-storage/embedded-pg -e normal,build,dev -i openssl ; cargo tree --workspace --locked --features ontoforge-storage/embedded-pg -e normal,build,dev -i native-tls (两者预期无匹配；G2 另验 guard)` | PARTIAL / WARNING G2: U:846-847/W:830-831 证实当前没有 openssl/native-tls；shell guard 的反向失败分支没有行为证据，旧 VERIFICATION 已记录 CR-B01。 |
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

### Phase 05

| Task | Requirement | 行为 | 命名测试 | 可执行命令 | 结论 |
|---|---|---|---|---|---|
| 05-01-T1 | ACTION-02, ACTION-05, ACTION-06, ACTION-10, ACTION-11 | edit-only Action 批次贯穿请求、definition fence、command claim、edit log 与 outbox | `tests::tracer_commits_one_action_batch`; `action_request_wire_cannot_override_host_limits` | `cargo test -p ontoforge-action --lib --locked tracer_commits_one_action_batch -- --nocapture ; cargo test -p ontoforge-action --lib --locked -- --nocapture` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 05-01-T2 | ACTION-02, ACTION-05, ACTION-06, ACTION-10, ACTION-11 | PostgreSQL schema parity 与 batch happy-path conformance | `action_batch_writes_one_command_and_outbox`; `action_batch_principal_scopes_command_id`; `action_batch_stale_definition_leaves_no_action_rows`; `action_batch_migration_constraints_match_contract` | `cargo test -p ontoforge-storage --test conformance_sqlite --locked action_batch_ ; cargo test -p ontoforge-storage --test conformance_pg --locked action_batch_ -- --ignored` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 05-02-T1 | ACTION-01, ACTION-02, ACTION-03, ACTION-04, ACTION-05, ACTION-06, ACTION-08 | 参数、default、五类 rule 与确定性 batch/edit 摘要 | `params_report_all_errors`; `batch_ordinal_and_same_pk_order_are_stable`; `batch_all_rule_kinds_expand_in_declared_order` | `cargo test -p ontoforge-action --test action --locked params_ -- --nocapture ; cargo test -p ontoforge-action --test action --locked batch_ -- --nocapture` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 05-02-T2 | ACTION-01, ACTION-02, ACTION-03, ACTION-04, ACTION-05, ACTION-06, ACTION-08 | masked criteria、写前/写后 scope、link/release 与 validate-only | `validate_only_runs_full_pipeline_without_writes`; `scope_hidden_target_matches_missing_outward_shape`; `scope_masked_criteria_is_denied_before_evaluation`; `scope_criterion_false_uses_configured_message`; `scope_write_column_must_be_visible`; `scope_post_state_must_remain_visible`; `scope_action_grant_is_checked_before_missing_target`; `scope_link_requires_both_endpoints` | `cargo test -p ontoforge-action --test security --locked scope_ -- --nocapture ; cargo test -p ontoforge-action --test action --locked validate_only_ -- --nocapture ; cargo test -p ontoforge-action --locked -- --nocapture` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 05-03-T1 | ACTION-09 | 提取显式版本的共享 `_version` codec 并切换 query 签发 | `version_claims_are_canonical_and_format_tagged`; `version_query_uses_format_tag_and_changes_with_scope`; `token_conflicts_after_edit_scope_or_key_change`; `token_malformed_is_rejected_without_writes` | `cargo test -p ontoforge-query --test security --locked version_ -- --nocapture ; cargo test -p ontoforge-query --locked -- --nocapture` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 05-03-T2 | ACTION-09 | 锁后验证 token 与无 token observed-version CAS | `token_query_issued_is_accepted_by_unchanged_action_read_set`; `token_conflicts_after_edit_scope_or_key_change`; `token_definition_change_is_rejected_without_action_writes`; `observed_no_token_commits_with_current_read_set`; `action_cas_rejects_stale_no_token_without_side_effects`; `action_cas_conflict_rolls_back_version_zero_skeleton`; `source_backed_query_token_roundtrip_and_source_drift` | `cargo test -p ontoforge-action --test version --locked ; cargo test -p ontoforge-storage --test conformance_sqlite --locked action_cas_ ; cargo test -p ontoforge-storage --test conformance_pg --locked action_cas_ -- --ignored` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 05-04-T1 | ACTION-04, ACTION-08, ACTION-09 | 双后端逐阶段故障注入与全量 rollback conformance | `action_atomic_after_log_failure`; `action_atomic_after_edits_failure`; `action_atomic_after_command_failure`; `action_atomic_after_outbox_failure`; `action_atomic_success_changes_all_domains_once` | `cargo test -p ontoforge-storage --features test-support --test conformance_sqlite --locked action_atomic_ -- --nocapture ; cargo test -p ontoforge-storage --features test-support --test conformance_pg --locked action_atomic_ -- --ignored --nocapture` | PARTIAL / WARNING G3: PG action_atomic 五例 I:761-765 通过；SQLite 对应 feature 分支没有当前日志。 |
| 05-04-T2 | ACTION-04, ACTION-08, ACTION-09 | 真实 PostgreSQL 同 pk token/no-token 竞态 | `concurrent_same_pk_with_token_has_one_winner`; `concurrent_same_pk_without_token_has_one_winner`; `concurrent_multirow_reverse_request_order_does_not_deadlock`; `concurrent_absent_edit_create_is_serialized` | `cargo test -p ontoforge-action --features test-support --test action_pg --locked concurrent_ -- --ignored --nocapture` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 05-05-T1 | ACTION-10 | PostgreSQL command claim winner/loser 与响应丢失重试 | `pg_same_key_same_params_has_one_winner_and_one_replay`; `pg_same_key_different_params_has_one_winner_and_one_reused`; `pg_winner_abort_allows_loser_to_claim`; `pg_response_lost_retry_reads_completed_receipt`; `pg_same_command_id_for_different_principals_is_independent` | `cargo test -p ontoforge-action --features test-support --test idempotency_pg --locked pg_ -- --ignored --nocapture` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 05-05-T2 | ACTION-10 | canonical fingerprint、过期与当前授权回放合同 | `expired_command_can_be_claimed_as_new_without_deleting_audit`; `fingerprint_ignores_key_order_but_preserves_explicit_default`; `replay_across_clock_and_definition_change_keeps_winner_time`; `replay_grant_removal_and_action_missing_deny_both_entries`; `replay_row_predicate_change_and_delete_keep_minimal_receipt`; `replay_cross_principal_lookup_matches_missing`; `replay_release_keeps_minimal_receipt_available` | `cargo test -p ontoforge-action --test idempotency --locked fingerprint_ -- --nocapture ; cargo test -p ontoforge-action --test idempotency --locked replay_ -- --nocapture ; cargo test -p ontoforge-action --test idempotency --locked expired_ -- --nocapture` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 05-06-T1 | ACTION-08, ACTION-09, ACTION-12 | fake Engine 固定 required、offline 与 observed-duplicate 三分支 | `all_patched_object_succeeds_with_engine_offline_and_zero_calls`; `source_required_offline_is_engine_unavailable_without_tx`; `release_source_visibility_guard`; `batch_deduplicates_source_keys_and_enforces_limit`; `source_duplicate_is_ambiguous_without_tx`; `source_unique_is_overlaid_once`; `source_backed_query_token_roundtrip_and_source_drift`; `later_alias_criterion_samples_shared_source_key`; `later_alias_release_samples_shared_source_key`; `later_item_token_samples_shared_source_key` | `cargo test -p ontoforge-action --test source --locked ; cargo test -p ontoforge-action --features embedded-pg --test source --locked source_guard_absent_target_insert_race -- --exact --nocapture` | PARTIAL / WARNING G4: source 常规/别名路径通过；source_guard_absent_target_insert_race 不在四份日志中，embedded-pg cfg 隐藏该例。 |
| 05-06-T2 | ACTION-08, ACTION-09, ACTION-12 | 真实 Trino Action source 与 duplicate conformance | `trino_action_unique_source_executes`; `trino_action_source_unavailable_is_engine_unavailable`; `trino_action_duplicate_pk_is_ambiguous`; `trino_action_all_patched_does_not_contact_trino` | `cargo test -p ontoforge-action --test trino_action_it --locked trino_action_ -- --ignored --nocapture` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 05-07-T1 | ACTION-07, ACTION-11 | outbox lease、retry 与 crash recovery worker | `http_failure_retries_with_same_event_id`; `crash_after_success_before_mark_redelivers_same_event_id`; `crash_before_send_committed_event_survives_restart`; `lease_expiry_reclaims_and_stale_attempt_is_fenced`; `lease_parallel_workers_claim_one_attempt`; `payload_is_minimal_and_versioned`; `payload_target_is_only_trusted_configuration`; `postgres_outbox_parallel_workers_claim_one_generation`; `postgres_outbox_after_send_crash_reclaims_with_same_payload` | `cargo test -p ontoforge-action --test outbox --locked ; cargo test -p ontoforge-action --test outbox_pg --locked -- --ignored` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 05-07-T2 | ACTION-07, ACTION-11 | production mutation 入口审计与 Action 后端 CI gates | `production_mutation_callers_are_action_import_and_read_audit`; `action_request_rejects_privileged_write_fields`; `import_log_has_reserved_identity_and_source_digest` | `cargo test -p ontoforge-action --test entrypoints --locked ; cargo test -p ontoforge --test cli_webhook_worker --locked` | COVERED: 命名行为测试在冻结源码 CI 中通过 |

## 命名测试源位置与实际通过日志索引

| Test | 源位置 | 动态证据（首次通过行） |
|---|---|---|
| `a_columns_times_b_rows_do_not_leak` | `crates/ontoforge-auth/tests/scope_merge.rs:66` | W:2241; U:2262 |
| `a_set_needs_ontology_and_server_config` | `crates/ontoforge/tests/private_fixtures.rs:54` | W:1928; U:1948 |
| `action_atomic_after_command_failure` | `crates/ontoforge-storage/tests/conformance/mod.rs:2206` | I:761 |
| `action_atomic_after_edits_failure` | `crates/ontoforge-storage/tests/conformance/mod.rs:2201` | I:763 |
| `action_atomic_after_log_failure` | `crates/ontoforge-storage/tests/conformance/mod.rs:2196` | I:762 |
| `action_atomic_after_outbox_failure` | `crates/ontoforge-storage/tests/conformance/mod.rs:2211` | I:764 |
| `action_atomic_success_changes_all_domains_once` | `crates/ontoforge-storage/tests/conformance/mod.rs:2216` | I:765 |
| `action_batch_migration_constraints_match_contract` | `crates/ontoforge-storage/tests/conformance/mod.rs:2457` | W:3791; U:3814; I:583 |
| `action_batch_principal_scopes_command_id` | `crates/ontoforge-storage/tests/conformance/mod.rs:2332` | W:3792; U:3815; I:584 |
| `action_batch_stale_definition_leaves_no_action_rows` | `crates/ontoforge-storage/tests/conformance/mod.rs:2364` | W:3793; U:3816; I:585 |
| `action_batch_writes_one_command_and_outbox` | `crates/ontoforge-storage/tests/conformance/mod.rs:2048` | W:3794; U:3817; I:586 |
| `action_cas_conflict_rolls_back_version_zero_skeleton` | `crates/ontoforge-storage/tests/conformance/mod.rs:2420` | W:3795; U:3818; I:588 |
| `action_cas_rejects_stale_no_token_without_side_effects` | `crates/ontoforge-storage/tests/conformance/mod.rs:2386` | W:3796; U:3819; I:589 |
| `action_request_rejects_privileged_write_fields` | `crates/ontoforge-action/tests/entrypoints.rs:132` | W:1965; U:1985 |
| `action_request_wire_cannot_override_host_limits` | `crates/ontoforge-action/tests/action.rs:45` | W:1948; U:1968 |
| `additive_property_adds_columns` | `crates/ontoforge-storage/tests/reload.rs:156` | W:3952; U:3975 |
| `additive_reload_on_pg_adds_columns` | `crates/ontoforge-storage/tests/reload_pg.rs:153` | I:659 |
| `admin_audit_is_append_only` | `crates/ontoforge-storage/tests/scope_store.rs:445` | W:4025; U:4048 |
| `all_edit_only_needs_no_profile` | `crates/ontoforge-schema/tests/bindings.rs:506` | W:3196; U:3218 |
| `all_errors_reported_together` | `crates/ontoforge-auth/tests/scope_compile.rs:383` | W:2214; U:2235 |
| `all_patched_object_succeeds_with_engine_offline_and_zero_calls` | `crates/ontoforge-action/tests/source.rs:326` | W:2046; U:2066 |
| `ansi_dialect_has_no_backslash_escape` | `crates/ontoforge-engine/tests/printer_prop.rs:172` | W:2489; U:2510 |
| `append_only_rejects_sqlite_replace_writes` | `crates/ontoforge-storage/tests/conformance/mod.rs:1183` | W:3797; U:3820; I:587 |
| `append_only_rejects_truncate_on_pg` | `crates/ontoforge-storage/tests/conformance/mod.rs:1144` | W:3798; U:3821; I:590 |
| `append_only_rejects_update_and_delete` | `crates/ontoforge-storage/tests/conformance/mod.rs:1108` | W:3799; U:3822; I:591 |
| `apply_creates_row_filter_indexes` | `crates/ontoforge-storage/tests/scope_store.rs:163` | W:4026; U:4049 |
| `apply_requires_plan_hash` | `crates/ontoforge/tests/cli_migrate.rs:155` | W:1764; U:1780 |
| `apply_with_stale_hash_is_rejected` | `crates/ontoforge-storage/tests/migrate.rs:170` | W:3889; U:3912 |
| `apply_without_active_definition_fails` | `crates/ontoforge-storage/tests/scope_store.rs:607` | W:4027; U:4050 |
| `apply_writes_roles_bindings_and_audit` | `crates/ontoforge-storage/tests/scope_store.rs:114` | W:4028; U:4051 |
| `archive_removed_property` | `crates/ontoforge-storage/tests/migrate.rs:110` | W:3890; U:3913 |
| `array_columns_are_not_probed` | `crates/ontoforge-storage/tests/reload.rs:1374` | W:3953; U:3976 |
| `attach_limit` | `crates/ontoforge-engine/tests/sqlite_engine.rs:752` | W:2550; U:2571 |
| `aviation_backing_values` | `crates/ontoforge-schema/tests/object_types.rs:290` | W:3344; U:3367 |
| `aviation_cost_property` | `crates/ontoforge-schema/tests/load_examples.rs:129` | W:3290; U:3313 |
| `aviation_fixture_data` | `crates/ontoforge/tests/cli_online.rs:241` | W:1780; U:1796 |
| `aviation_is_clean_under_dev` | `crates/ontoforge/tests/cli.rs:206` | W:1664; U:1680 |
| `aviation_is_healthy` | `crates/ontoforge-engine/tests/drift.rs:111` | W:2370; U:2391 |
| `aviation_json_fixtures_match_yaml_fixtures` | `crates/ontoforge/tests/cli_online.rs:272` | W:1781; U:1797 |
| `aviation_lints_clean` | `crates/ontoforge-schema/tests/lint_examples.rs:16` | W:3283; U:3306 |
| `aviation_reload_creates_indexes` | `crates/ontoforge-storage/tests/reload.rs:374` | W:3954; U:3977 |
| `aviation_scope_compiles` | `crates/ontoforge-auth/tests/scope_compile.rs:90` | W:2216; U:2237 |
| `aviation_yaml_and_json_produce_identical_ir` | `crates/ontoforge-schema/tests/load_examples.rs:21` | W:3295; U:3318 |
| `bad_profile_dir_is_v16` | `crates/ontoforge-schema/tests/bindings.rs:148` | W:3197; U:3219 |
| `bad_row_filter_is_v22_located` | `crates/ontoforge-auth/tests/scope_compile.rs:160` | W:2217; U:2238 |
| `bad_trace_token_is_config_error` | `crates/ontoforge-engine/tests/trino_client.rs:363` | W:2583; U:2604 |
| `basic_over_http_is_rejected` | `crates/ontoforge-engine/tests/server_config.rs:189` | W:2502; U:2523 |
| `basic_requires_exactly_one_secret_ref` | `crates/ontoforge-engine/tests/server_config.rs:320` | W:2503; U:2524 |
| `batch_all_rule_kinds_expand_in_declared_order` | `crates/ontoforge-action/tests/action.rs:220` | W:1949; U:1969 |
| `batch_deduplicates_source_keys_and_enforces_limit` | `crates/ontoforge-action/tests/source.rs:577` | W:2047; U:2067 |
| `batch_ordinal_and_same_pk_order_are_stable` | `crates/ontoforge-action/tests/action.rs:426` | W:1950; U:1970 |
| `batches_follow_batch_size` | `crates/ontoforge-storage/tests/import.rs:226` | W:3867; U:3890 |
| `bind_check::compat_tests::arrays_are_compared_by_element` | `crates/ontoforge-engine/src/bind_check/compat_tests.rs:231` | W:2293; U:2314 |
| `bind_check::compat_tests::case_collision` | `crates/ontoforge-engine/src/bind_check/compat_tests.rs:352` | W:2294; U:2315 |
| `bind_check::compat_tests::decimal_integer_digits_and_scale` | `crates/ontoforge-engine/src/bind_check/compat_tests.rs:151` | W:2295; U:2316 |
| `bind_check::compat_tests::integers_only_widen` | `crates/ontoforge-engine/src/bind_check/compat_tests.rs:105` | W:2298; U:2319 |
| `bind_check::compat_tests::link_ends_are_compared_with_the_primary_keys` | `crates/ontoforge-engine/src/bind_check/compat_tests.rs:447` | W:2299; U:2320 |
| `bind_check::compat_tests::timestamps_are_checked_against_the_declared_zone` | `crates/ontoforge-engine/src/bind_check/compat_tests.rs:206` | W:2301; U:2322 |
| `bind_check::tests::online_missing_table` | `crates/ontoforge-engine/src/bind_check/tests.rs:110` | W:2310; U:2331 |
| `bind_check::tests::online_type_family_mismatch` | `crates/ontoforge-engine/src/bind_check/tests.rs:173` | W:2311; U:2332 |
| `bind_check::tests::warnings_do_not_degrade` | `crates/ontoforge-engine/tests/drift.rs:386` | W:2312; U:2333 |
| `binding_outside_bindings_dir_is_v16` | `crates/ontoforge-schema/tests/bindings.rs:113` | W:3199; U:3221 |
| `binding_to_unknown_role_is_v21` | `crates/ontoforge-schema/tests/scope_documents.rs:69` | W:3390; U:3413 |
| `booleans_and_dates` | `crates/ontoforge-engine/tests/sqlite_engine.rs:637` | W:2551; U:2572 |
| `bound_columns_object_and_link` | `crates/ontoforge-schema/tests/bindings.rs:257` | W:3200; U:3222 |
| `ca_file_is_loaded` | `crates/ontoforge-engine/tests/server_config.rs:468` | W:2504; U:2525 |
| `cache_control_max_age_is_clamped` | `crates/ontoforge-auth/tests/oidc.rs:333` | W:2158; U:2179 |
| `cached_column_requires_flush_before_definition_switch` | `crates/ontoforge-storage/tests/reload_trino_it.rs:230` | I:1115 |
| `call_without_columns_returns_empty_output` | `crates/ontoforge-engine/tests/trino_client.rs:931` | W:2585; U:2606 |
| `catalogs_visible` | `crates/ontoforge-engine/tests/trino_it.rs:553` | I:1016 |
| `check_pk_clean_on_aviation` | `crates/ontoforge/tests/cli_check_pk.rs:185` | W:1678; U:1694 |
| `check_pk_on_trino` | `crates/ontoforge/tests/cli_check_pk.rs:252` | W:1680; U:1696 |
| `check_pk_reports_duplicates` | `crates/ontoforge/tests/cli_check_pk.rs:201` | W:1681; U:1697 |
| `check_pk_reports_null_keys` | `crates/ontoforge/tests/cli_check_pk.rs:225` | W:1682; U:1698 |
| `check_pk_requires_online` | `crates/ontoforge/tests/cli_online_trino.rs:758` | W:1798; U:1814 |
| `closed_port_is_unavailable` | `crates/ontoforge-engine/tests/trino_resilience.rs:79` | W:2645; U:2666 |
| `collation_case_insensitive_is_v22` | `crates/ontoforge-auth/tests/scope_compile.rs:326` | W:2218; U:2239 |
| `column_key_unknown_property_is_v09` | `crates/ontoforge-schema/tests/bindings.rs:529` | W:3201; U:3223 |
| `commit_appends_one_log_record` | `crates/ontoforge-storage/tests/conformance/mod.rs:461` | W:3808; U:3831; I:598 |
| `commit_set_creates_edit_row` | `crates/ontoforge-storage/tests/conformance/mod.rs:421` | W:3809; U:3832; I:601 |
| `commit_writes_history_snapshot` | `crates/ontoforge-storage/tests/conformance/mod.rs:493` | W:3810; U:3833; I:602 |
| `committed_schemas_are_up_to_date` | `crates/ontoforge-schema/tests/schema_export.rs:114` | W:3373; U:3396 |
| `concurrent_absent_edit_create_is_serialized` | `crates/ontoforge-action/tests/action_pg.rs:334` | I:784 |
| `concurrent_commits_on_same_pk_serialize` | `crates/ontoforge-storage/tests/conformance/mod.rs:1938` | W:3811; U:3834; I:603 |
| `concurrent_fk_claims_one_wins` | `crates/ontoforge-storage/tests/conformance/mod.rs:1991` | W:3812; U:3835; I:606 |
| `concurrent_kid_miss_is_single_flight` | `crates/ontoforge-auth/tests/oidc.rs:418` | W:2159; U:2180 |
| `concurrent_multirow_reverse_request_order_does_not_deadlock` | `crates/ontoforge-action/tests/action_pg.rs:321` | I:786 |
| `concurrent_reloads_single_flight` | `crates/ontoforge-storage/tests/reload.rs:971` | W:3955; U:3978 |
| `concurrent_same_pk_with_token_has_one_winner` | `crates/ontoforge-action/tests/action_pg.rs:294` | I:783 |
| `concurrent_same_pk_without_token_has_one_winner` | `crates/ontoforge-action/tests/action_pg.rs:308` | I:785 |
| `config_alone_never_enables_dev_tokens` | `crates/ontoforge-auth/tests/dev_token.rs:288` | W:2133; U:2153 |
| `config_errors_are_v20_with_location` | `crates/ontoforge/tests/cli_online_trino.rs:678` | W:1799; U:1815 |
| `corpus_cases_match_on_sqlite` | `crates/ontoforge-auth/tests/rowpred_diff.rs:116` | W:2192; U:2213 |
| `covered_criteria_compile` | `crates/ontoforge-auth/tests/criteria_coverage.rs:55` | W:2114; U:2134 |
| `crash_after_success_before_mark_redelivers_same_event_id` | `crates/ontoforge-action/tests/outbox.rs:460` | W:2005; U:2025 |
| `crash_before_send_committed_event_survives_restart` | `crates/ontoforge-action/tests/outbox.rs:377` | W:2006; U:2026 |
| `criteria_without_object_reads_need_no_properties` | `crates/ontoforge-auth/tests/criteria_coverage.rs:89` | W:2115; U:2135 |
| `csv_empty_cell_is_null_for_every_type` | `crates/ontoforge-engine/tests/sqlite_engine.rs:385` | W:2552; U:2573 |
| `data_spans_pages` | `crates/ontoforge-engine/tests/trino_client.rs:176` | W:2586; U:2607 |
| `ddl::tests::ddl_snapshots` | `crates/ontoforge-storage/src/ddl.rs:623` | W:3686; U:3709 |
| `decimal_boundaries_12_2` | `crates/ontoforge-engine/tests/sqlite_engine.rs:475` | W:2553; U:2574 |
| `decode_golden` | `crates/ontoforge-engine/tests/trino_it.rs:297` | I:1022 |
| `decode_types` | `crates/ontoforge-engine/tests/trino_client.rs:493` | W:2588; U:2609 |
| `definition_inside_bindings_dir_is_v16` | `crates/ontoforge-schema/tests/bindings.rs:138` | W:3203; U:3225 |
| `degrade_reason_code` | `crates/ontoforge-engine/tests/drift.rs:313` | W:2371; U:2392 |
| `describe_demo_flights` | `crates/ontoforge-engine/tests/trino_it.rs:147` | I:1017 |
| `describe_missing_tables_absent` | `crates/ontoforge-engine/tests/trino_client.rs:788` | W:2589; U:2610 |
| `describe_mixed_catalogs_is_config_error` | `crates/ontoforge-engine/tests/trino_client.rs:876` | W:2590; U:2611 |
| `describe_tables_batches_one_catalog` | `crates/ontoforge-engine/tests/trino_client.rs:681` | W:2592; U:2613 |
| `describe_tables_lowercases_names` | `crates/ontoforge-engine/tests/trino_client.rs:755` | W:2593; U:2614 |
| `describe_tables_rejects_mixed_catalogs` | `crates/ontoforge-engine/tests/sqlite_engine.rs:158` | W:2554; U:2575 |
| `describe_truncated_metadata_is_error` | `crates/ontoforge-engine/tests/trino_client.rs:847` | W:2594; U:2615 |
| `describe_types_golden` | `crates/ontoforge-engine/tests/trino_it.rs:230` | I:1019 |
| `destructive_changes_are_rejected` | `crates/ontoforge-storage/tests/reload.rs:202` | W:3956; U:3979 |
| `dev_token_round_trip` | `crates/ontoforge-auth/tests/dev_token.rs:64` | W:2134; U:2154 |
| `diag::tests::phase3_codes` | `crates/ontoforge-schema/src/diag.rs:255` | W:3020; U:3041 |
| `display_round_trip` | `crates/ontoforge-expr/tests/rowpred_eval.rs:618` | W:2740; U:2761 |
| `dns_failure_is_unavailable` | `crates/ontoforge-engine/tests/trino_resilience.rs:105` | W:2647; U:2668 |
| `doc_type_table_matches_compare_binding` | `crates/ontoforge-engine/tests/doc_type_table.rs:154` | W:2363; U:2384 |
| `drift_is_healthy_on_ci_trino` | `crates/ontoforge-engine/tests/trino_it.rs:583` | I:1020 |
| `drop_archived_cleans_up` | `crates/ontoforge-storage/tests/migrate.rs:657` | W:3891; U:3914 |
| `drop_archived_plan_and_apply` | `crates/ontoforge/tests/cli_migrate.rs:253` | W:1767; U:1783 |
| `dropped_future_sends_delete` | `crates/ontoforge-engine/tests/trino_resilience.rs:647` | W:2648; U:2669 |
| `dry_run_changes_nothing` | `crates/ontoforge-storage/tests/reload.rs:271` | W:3957; U:3980 |
| `dry_run_prints_ddl_only` | `crates/ontoforge/tests/cli_reload.rs:71` | W:1819; U:1835 |
| `duplicate_binding_is_v08` | `crates/ontoforge-schema/tests/bindings.rs:172` | W:3204; U:3226 |
| `duplicate_link_add_keeps_one_live_row` | `crates/ontoforge-storage/tests/conformance/mod.rs:1889` | W:3813; U:3836; I:605 |
| `duplicate_role_is_v08` | `crates/ontoforge-schema/tests/scope_documents.rs:214` | W:3391; U:3414 |
| `duplicates_and_nulls_found` | `crates/ontoforge-engine/tests/pk_check.rs:128` | W:2465; U:2486 |
| `dynamic_object_path_is_v23_fail_closed` | `crates/ontoforge-auth/tests/criteria_coverage.rs:100` | W:2116; U:2136 |
| `e14_can_be_disabled` | `crates/ontoforge/tests/cli_lint.rs:374` | W:1746; U:1762 |
| `e14_human` | `crates/ontoforge/tests/cli_lint.rs:360` | W:1747; U:1763 |
| `edit_only_rules` | `crates/ontoforge-storage/tests/conformance/mod.rs:1561` | W:3814; U:3837; I:607 |
| `edit_only_type_with_binding_is_v16` | `crates/ontoforge-schema/tests/bindings.rs:420` | W:3205; U:3227 |
| `editonly_link_import` | `crates/ontoforge-storage/tests/import.rs:382` | W:3868; U:3891 |
| `empty_identifier_is_print_error` | `crates/ontoforge-engine/tests/sqlite_engine.rs:74` | W:2556; U:2577 |
| `empty_list_attr_is_false_on_both_sides` | `crates/ontoforge-auth/tests/rowpred_diff.rs:68` | W:2193; U:2214 |
| `empty_object_types_grants_nothing` | `crates/ontoforge-auth/tests/scope_compile.rs:350` | W:2221; U:2242 |
| `engine_collations_are_registered` | `crates/ontoforge-storage/tests/sqlite_connection.rs:37` | W:4053; U:4076 |
| `engine_error_is_not_a_report` | `crates/ontoforge-engine/tests/drift.rs:293` | W:2372; U:2393 |
| `engine_wide_classification` | `crates/ontoforge-engine/tests/sqlite_engine.rs:116` | W:2557; U:2578 |
| `engine_wide_error_aborts_round` | `crates/ontoforge-engine/tests/bind_check_isolation.rs:164` | W:2354; U:2375 |
| `error_codes_are_upper_snake` | `crates/ontoforge-engine/tests/sqlite_engine.rs:89` | W:2558; U:2579 |
| `every_pooled_connection_has_recursive_triggers` | `crates/ontoforge-storage/tests/sqlite_connection.rs:21` | W:4054; U:4077 |
| `every_value_type_round_trips` | `crates/ontoforge-storage/tests/conformance/mod.rs:890` | W:3815; U:3838; I:608 |
| `example_config_dev_token_works_from_a_fresh_tree` | `crates/ontoforge/tests/cli_dev_token.rs:225` | W:1694; U:1710 |
| `example_config_reload_works_from_a_fresh_tree` | `crates/ontoforge/tests/cli_reload.rs:307` | W:1820; U:1836 |
| `exhausted_503_is_unavailable` | `crates/ontoforge-engine/tests/trino_resilience.rs:364` | W:2649; U:2670 |
| `existing_objects_block_create_only` | `crates/ontoforge-storage/tests/import.rs:529`; `crates/ontoforge/tests/cli_import.rs:237` | W:1712; U:1728 |
| `exit_code_table` | `crates/ontoforge/tests/cli_check_pk.rs:428` | W:1685; U:1701 |
| `expired_command_can_be_claimed_as_new_without_deleting_audit` | `crates/ontoforge-action/tests/idempotency.rs:136` | W:1985; U:2005 |
| `expired_dev_token_rejected` | `crates/ontoforge-auth/tests/dev_token.rs:411` | W:2135; U:2155 |
| `explicit_null_branch_visible` | `crates/ontoforge-expr/tests/rowpred_eval.rs:114` | W:2741; U:2762 |
| `file_level_validation` | `crates/ontoforge-storage/tests/import.rs:647` | W:3871; U:3894 |
| `fingerprint_ignores_key_order_but_preserves_explicit_default` | `crates/ontoforge-action/tests/idempotency.rs:179` | W:1986; U:2006 |
| `finished_query_sends_no_delete` | `crates/ontoforge-engine/tests/trino_resilience.rs:798` | W:2651; U:2672 |
| `first_reload_creates_tables` | `crates/ontoforge-storage/tests/reload.rs:73` | W:3958; U:3981 |
| `first_reload_on_pg_puts_edit_tables_in_edit_schema` | `crates/ontoforge-storage/tests/reload_pg.rs:73` | I:658 |
| `fixture_error_has_line` | `crates/ontoforge-engine/tests/sqlite_engine.rs:694` | W:2560; U:2581 |
| `fixture_reference_rules` | `crates/ontoforge-schema/tests/bindings.rs:629` | W:3207; U:3229 |
| `fixtures_dir_is_not_scanned` | `crates/ontoforge-schema/tests/bindings.rs:233` | W:3208; U:3230 |
| `flight_binding_loads_under_dev_profile` | `crates/ontoforge-schema/tests/bindings.rs:87` | W:3209; U:3231 |
| `foreign_keys_on` | `crates/ontoforge-storage/tests/sqlite_connection.rs:63` | W:4055; U:4078 |
| `foreign_next_uri_is_rejected` | `crates/ontoforge-engine/tests/trino_client.rs:275` | W:2595; U:2616 |
| `full_sync_removes_bindings_and_roles` | `crates/ontoforge-storage/tests/scope_store.rs:225` | W:4030; U:4053 |
| `geopoint_and_decimal_cells` | `crates/ontoforge-storage/tests/import.rs:256` | W:3872; U:3895 |
| `get_502_and_504_are_retried` | `crates/ontoforge-engine/tests/trino_resilience.rs:345` | W:2652; U:2673 |
| `get_503_then_success` | `crates/ontoforge-engine/tests/trino_resilience.rs:325` | W:2653; U:2674 |
| `header_key_urls_are_never_followed` | `crates/ontoforge-auth/tests/oidc.rs:738` | W:2164; U:2185 |
| `hidden_criteria_column_is_v23_at_the_action_entry` | `crates/ontoforge-auth/tests/criteria_coverage.rs:60` | W:2117; U:2137 |
| `hs256_round_trip_does_not_panic` | `crates/ontoforge-auth/tests/crypto_backend.rs:13` | W:2126; U:2146 |
| `hs256_with_other_issuer_rejected` | `crates/ontoforge-auth/tests/dev_token.rs:364` | W:2137; U:2157 |
| `http_failure_retries_with_same_event_id` | `crates/ontoforge-action/tests/outbox.rs:419` | W:2007; U:2027 |
| `i64_min_is_printed_without_overflow` | `crates/ontoforge-engine/tests/printer_literals.rs:179` | W:2477; U:2498 |
| `i64_min_reads_back_as_integer_on_sqlite` | `crates/ontoforge-engine/tests/printer_literals.rs:189` | W:2478; U:2499 |
| `identifier_round_trips_on_sqlite` | `crates/ontoforge-engine/tests/printer_prop.rs:280` | W:2490; U:2511 |
| `import_log_has_reserved_identity_and_source_digest` | `crates/ontoforge-action/tests/entrypoints.rs:154` | W:1966; U:1986 |
| `imports_airports_csv` | `crates/ontoforge/tests/cli_import.rs:79` | W:1713; U:1729 |
| `in_flight_commit_blocks_switch_on_pg` | `crates/ontoforge-storage/tests/reload_pg.rs:334` | I:663 |
| `index_added_on_second_reload` | `crates/ontoforge-storage/tests/reload.rs:425` | W:3959; U:3982 |
| `indexed_defaults_to_false` | `crates/ontoforge-schema/tests/object_types.rs:320` | W:3349; U:3372 |
| `indexed_must_be_boolean` | `crates/ontoforge-schema/tests/object_types.rs:334` | W:3350; U:3373 |
| `indexed_true_is_kept` | `crates/ontoforge-schema/tests/object_types.rs:327` | W:3351; U:3374 |
| `indexes_on_pg` | `crates/ontoforge-storage/tests/reload_pg.rs:200` | I:661 |
| `integer_ranges` | `crates/ontoforge-engine/tests/sqlite_engine.rs:523` | W:2565; U:2586 |
| `invalid_definitions_never_open_store` | `crates/ontoforge/tests/cli_reload.rs:102` | W:1821; U:1837 |
| `invalid_literals_are_rejected` | `crates/ontoforge-engine/tests/printer_literals.rs:74` | W:2479; U:2500 |
| `invalid_rename_decl_is_rejected` | `crates/ontoforge-storage/tests/migrate.rs:492` | W:3892; U:3915 |
| `invalid_scope_writes_nothing` | `crates/ontoforge/tests/cli_scope.rs:187` | W:1843; U:1859 |
| `invalid_values_are_rejected_without_writes` | `crates/ontoforge-storage/tests/conformance/mod.rs:589` | W:3817; U:3840; I:610 |
| `invisible_nonexistent_and_ungranted_are_identical` | `crates/ontoforge-storage/tests/history_masking.rs:366` | W:3847; U:3870 |
| `ir::names::tests::profile_name_rules` | `crates/ontoforge-schema/src/ir/names.rs:282` | W:3028; U:3049 |
| `ir::names::tests::profile_name_schema_has_pattern` | `crates/ontoforge-schema/src/ir/names.rs:310` | W:3029; U:3050 |
| `ir::object::tests::effective_decimal_defaults` | `crates/ontoforge-schema/src/ir/object.rs:113` | W:3035; U:3056 |
| `issues_token_verifiable_by_dev_verifier` | `crates/ontoforge/tests/cli_dev_token.rs:92` | W:1696; U:1712 |
| `json_decimals_are_never_rounded` | `crates/ontoforge-storage/tests/import.rs:306` | W:3874; U:3897 |
| `json_fixture_loads` | `crates/ontoforge-engine/tests/sqlite_engine.rs:419` | W:2566; U:2587 |
| `json_import_matches_csv` | `crates/ontoforge-storage/tests/import.rs:355` | W:3875; U:3898 |
| `json_output` | `crates/ontoforge/tests/cli_reload.rs:124` | W:1822; U:1838 |
| `jwks_is_fetched_once_for_repeated_tokens` | `crates/ontoforge-auth/tests/oidc.rs:314` | W:2167; U:2188 |
| `key_file_is_created_once_and_reused` | `crates/ontoforge-auth/tests/dev_token.rs:93` | W:2140; U:2160 |
| `key_file_of_wrong_length_is_a_config_error` | `crates/ontoforge-auth/tests/dev_token.rs:136` | W:2141; U:2161 |
| `key_file_permissions_unix` | `crates/ontoforge-auth/tests/dev_token.rs:503` | U:2162 |
| `kid_miss_refetches_once_then_throttles` | `crates/ontoforge-auth/tests/oidc.rs:376` | W:2169; U:2190 |
| `kleene_laws` | `crates/ontoforge-expr/tests/rowpred_eval.rs:627` | W:2746; U:2767 |
| `later_alias_criterion_samples_shared_source_key` | `crates/ontoforge-action/tests/source.rs:460` | W:2048; U:2068 |
| `later_alias_release_samples_shared_source_key` | `crates/ontoforge-action/tests/source.rs:494` | W:2049; U:2069 |
| `later_item_token_samples_shared_source_key` | `crates/ontoforge-action/tests/source.rs:526` | W:2050; U:2070 |
| `lease_expiry_reclaims_and_stale_attempt_is_fenced` | `crates/ontoforge-action/tests/outbox.rs:506` | W:2009; U:2029 |
| `lease_parallel_workers_claim_one_attempt` | `crates/ontoforge-action/tests/outbox.rs:565` | W:2010; U:2030 |
| `library_rejects_credentials_over_http` | `crates/ontoforge-engine/tests/server_config.rs:218` | W:2507; U:2528 |
| `library_rejects_cross_scheme_origin` | `crates/ontoforge-engine/tests/server_config.rs:237` | W:2508; U:2529 |
| `link_add_remove_toggle` | `crates/ontoforge-storage/tests/conformance/mod.rs:1618` | W:3819; U:3842; I:612 |
| `link_changes_are_destructive` | `crates/ontoforge-storage/tests/reload.rs:588` | W:3960; U:3983 |
| `link_columns_need_from_and_to` | `crates/ontoforge-schema/tests/bindings.rs:565` | W:3210; U:3232 |
| `link_entries_are_not_object_history` | `crates/ontoforge-storage/tests/history_masking.rs:659` | W:3849; U:3872 |
| `lint::binding::e13::tests::e13_can_be_downgraded` | `crates/ontoforge-schema/src/lint/binding/e13.rs:169` | W:3037; U:3058 |
| `lint::binding::e13::tests::triggers` | `crates/ontoforge-schema/src/lint/e09.rs:180`; `crates/ontoforge-schema/src/lint/binding/e13.rs:145`; `crates/ontoforge-schema/src/lint/binding/e14.rs:128`; `crates/ontoforge-schema/src/lint/binding/e15.rs:108`; `crates/ontoforge-schema/src/lint/binding/e16.rs:111`; `crates/ontoforge-schema/src/lint/binding/w17.rs:140` | W:3038; U:3059 |
| `lint::binding::e14::tests::e14_multi_profile` | `crates/ontoforge-schema/src/lint/binding/e14.rs:195` | W:3043; U:3064 |
| `lint::binding::e15::tests::triggers` | `crates/ontoforge-schema/src/lint/e09.rs:180`; `crates/ontoforge-schema/src/lint/binding/e13.rs:145`; `crates/ontoforge-schema/src/lint/binding/e14.rs:128`; `crates/ontoforge-schema/src/lint/binding/e15.rs:108`; `crates/ontoforge-schema/src/lint/binding/e16.rs:111`; `crates/ontoforge-schema/src/lint/binding/w17.rs:140` | W:3047; U:3068 |
| `lint::binding::e16::tests::triggers` | `crates/ontoforge-schema/src/lint/e09.rs:180`; `crates/ontoforge-schema/src/lint/binding/e13.rs:145`; `crates/ontoforge-schema/src/lint/binding/e14.rs:128`; `crates/ontoforge-schema/src/lint/binding/e15.rs:108`; `crates/ontoforge-schema/src/lint/binding/e16.rs:111`; `crates/ontoforge-schema/src/lint/binding/w17.rs:140` | W:3049; U:3070 |
| `lint::binding::w17::tests::w17_off` | `crates/ontoforge-schema/src/lint/binding/w17.rs:263` | W:3053; U:3074 |
| `lint::binding::w17::tests::w17_unreadable_header_warns` | `crates/ontoforge-schema/src/lint/binding/w17.rs:212` | W:3054; U:3075 |
| `load_scope_round_trips` | `crates/ontoforge-storage/tests/scope_store.rs:198` | W:4031; U:4054 |
| `loader::discover::tests::fixture_path_rules` | `crates/ontoforge-schema/src/loader/discover.rs:362` | W:3126; U:3147 |
| `loader::fixture::tests::fixture_header_rules` | `crates/ontoforge-schema/src/loader/fixture.rs:80` | W:3134; U:3155 |
| `loader::tests::fixture_bytes_count_toward_budget` | `crates/ontoforge-schema/src/loader/mod.rs:801` | W:3150; U:3171 |
| `loader::tests::fixture_header_is_captured` | `crates/ontoforge-schema/src/loader/mod.rs:757` | W:3151; U:3172 |
| `loader::tests::read_limited_custom_limit` | `crates/ontoforge-schema/src/loader/mod.rs:914` | W:3153; U:3174 |
| `loader::tests::read_limited_follows_links_only_when_asked` | `crates/ontoforge-schema/src/loader/mod.rs:946` | U:3175 |
| `loads_aviation_scope_example` | `crates/ontoforge-schema/tests/scope_documents.rs:25` | W:3393; U:3416 |
| `lock_timeout_keeps_old_definition` | `crates/ontoforge-storage/tests/reload_pg.rs:397` | I:665 |
| `log_records_every_op` | `crates/ontoforge-storage/tests/conformance/mod.rs:1679` | W:3821; U:3844; I:614 |
| `long_names_on_pg` | `crates/ontoforge-storage/tests/reload_pg.rs:275` | I:662 |
| `long_names_round_trip` | `crates/ontoforge-storage/tests/reload.rs:534` | W:3961; U:3984 |
| `lowered_scope_matches_evaluate_on_sqlite` | `crates/ontoforge-auth/tests/scope_merge.rs:218` | W:2245; U:2266 |
| `many_to_many_with_backing_loads` | `crates/ontoforge-schema/tests/link_types.rs:402` | W:3265; U:3288 |
| `many_to_many_without_backing_is_v05` | `crates/ontoforge-schema/tests/link_types.rs:386` | W:3267; U:3290 |
| `many_to_many_without_binding_is_v16` | `crates/ontoforge-schema/tests/bindings.rs:403` | W:3211; U:3233 |
| `max_bytes_truncates_and_deletes` | `crates/ontoforge-engine/tests/trino_resilience.rs:867` | W:2657; U:2678 |
| `max_rows_truncates_and_deletes` | `crates/ontoforge-engine/tests/trino_resilience.rs:837` | W:2658; U:2679 |
| `migrate_plan_then_apply` | `crates/ontoforge/tests/cli_migrate.rs:97` | W:1769; U:1785 |
| `migrate_rechecks_applied_roles` | `crates/ontoforge-storage/tests/migrate.rs:842` | W:3893; U:3916 |
| `migrate_refuses_a_change_that_breaks_an_applied_role` | `crates/ontoforge/tests/cli_migrate.rs:370` | W:1770; U:1786 |
| `migrate_with_roles_applies_when_the_roles_still_compile` | `crates/ontoforge-storage/tests/migrate.rs:888` | W:3894; U:3917 |
| `missing_attr_claim_is_absent` | `crates/ontoforge-auth/tests/oidc.rs:281` | W:2172; U:2193 |
| `missing_attr_is_unknown_and_invisible` | `crates/ontoforge-expr/tests/rowpred_eval.rs:88` | W:2747; U:2768 |
| `missing_column_degrades_only_flight` | `crates/ontoforge-engine/tests/drift.rs:137` | W:2373; U:2394 |
| `missing_env_panics` | `crates/ontoforge-engine/tests/trino_it.rs:114`; `crates/ontoforge/tests/cli_trino_it.rs:27`; `crates/ontoforge/tests/private_fixtures.rs:42` | W:1890; U:1906 |
| `missing_key_directory_is_created` | `crates/ontoforge-auth/tests/dev_token.rs:110` | W:2143; U:2164 |
| `missing_store_section` | `crates/ontoforge/tests/cli_reload.rs:84` | W:1823; U:1839 |
| `missing_sub_rejected` | `crates/ontoforge-auth/tests/dev_token.rs:431` | W:2144; U:2165 |
| `model_matches_store` | `crates/ontoforge-storage/tests/model_prop.rs:484` | W:3920; U:3943 |
| `multi_document_file_loads` | `crates/ontoforge-schema/tests/scope_documents.rs:89` | W:3394; U:3417 |
| `naming::tests::bounded_names` | `crates/ontoforge-storage/src/naming.rs:152` | W:3696; U:3719 |
| `next_uri_scheme_mismatch_is_rejected` | `crates/ontoforge-engine/tests/trino_client.rs:406` | W:2597; U:2618 |
| `no_grant_is_invisible` | `crates/ontoforge-auth/tests/scope_merge.rs:48` | W:2246; U:2267 |
| `no_key_file_anywhere_fails` | `crates/ontoforge/tests/cli_dev_token.rs:327` | W:1699; U:1715 |
| `no_profile_selected_is_v16` | `crates/ontoforge-schema/tests/bindings.rs:492` | W:3212; U:3234 |
| `nul_in_a_cell_is_refused_before_anything_is_written` | `crates/ontoforge-storage/tests/import.rs:701` | W:3877; U:3900 |
| `nul_in_string_is_print_error` | `crates/ontoforge-engine/tests/sqlite_engine.rs:65` | W:2567; U:2588 |
| `null_column_comparison_is_unknown_on_both_sides` | `crates/ontoforge-auth/tests/rowpred_diff.rs:63` | W:2195; U:2216 |
| `null_column_not_equal_is_unknown` | `crates/ontoforge-expr/tests/rowpred_eval.rs:100` | W:2748; U:2769 |
| `nulls_are_not_duplicates` | `crates/ontoforge-engine/tests/pk_check.rs:159` | W:2466; U:2487 |
| `object_type_in_scope_dir_is_v03` | `crates/ontoforge-schema/tests/scope_documents.rs:132` | W:3395; U:3418 |
| `object_without_backing_is_v05` | `crates/ontoforge-schema/tests/object_types.rs:259` | W:3356; U:3379 |
| `observed_no_token_commits_with_current_read_set` | `crates/ontoforge-action/tests/version.rs:221` | W:2073; U:2093 |
| `offline_validate_unchanged` | `crates/ontoforge/tests/cli_online.rs:177` | W:1783; U:1799 |
| `oidc_issuer_rules` | `crates/ontoforge-engine/tests/server_config.rs:692` | W:2510; U:2531 |
| `oidc_rejects_symmetric_and_none_algorithms` | `crates/ontoforge-engine/tests/server_config.rs:677` | W:2512; U:2533 |
| `oidc_still_works_with_dev_enabled` | `crates/ontoforge-auth/tests/dev_token.rs:346` | W:2145; U:2166 |
| `oidc_two_issuers_with_defaults` | `crates/ontoforge-engine/tests/server_config.rs:659` | W:2513; U:2534 |
| `one_catalog_query_error_is_v19_others_checked` | `crates/ontoforge/tests/cli_online_trino.rs:381` | W:1801; U:1817 |
| `one_to_many_link_binding_is_v16` | `crates/ontoforge-schema/tests/bindings.rs:438` | W:3213; U:3235 |
| `one_to_many_with_backing_is_v11` | `crates/ontoforge-schema/tests/link_types.rs:409` | W:3268; U:3291 |
| `one_to_one_unique_index_rejects_second_row` | `crates/ontoforge-storage/tests/reload.rs:396` | W:3966; U:3989 |
| `one_to_one_violation_is_reported` | `crates/ontoforge-storage/tests/conformance/mod.rs:1842` | W:3822; U:3845; I:616 |
| `online::tests::query_context_uses_defaults_and_timeout_flag` | `crates/ontoforge/src/online.rs:594` | W:1635; U:1651 |
| `online_aviation_is_clean` | `crates/ontoforge/tests/cli_online.rs:95` | W:1784; U:1800 |
| `online_binding_without_fixture_is_v20` | `crates/ontoforge/tests/cli_online.rs:149` | W:1785; U:1801 |
| `online_missing_column_human` | `crates/ontoforge/tests/cli_online.rs:166` | W:1786; U:1802 |
| `online_missing_explicit_column` | `crates/ontoforge/tests/cli_online.rs:115` | W:1787; U:1803 |
| `online_missing_implicit_column` | `crates/ontoforge/tests/cli_online.rs:134` | W:1788; U:1804 |
| `open_creates_the_parent_directories_of_the_sqlite_file` | `crates/ontoforge-storage/tests/sqlite_connection.rs:73` | W:4056; U:4079 |
| `order_independent_hash` | `crates/ontoforge-storage/tests/scope_store.rs:421` | W:4033; U:4056 |
| `ordinary_caller_sees_visible_values_only` | `crates/ontoforge-storage/tests/history_masking.rs:246` | W:3850; U:3873 |
| `oversized_fixture_is_v16` | `crates/ontoforge-schema/tests/bindings.rs:664` | W:3214; U:3236 |
| `oversized_jwks_rejected` | `crates/ontoforge-auth/tests/oidc.rs:493` | W:2176; U:2197 |
| `oversized_page_is_protocol_error` | `crates/ontoforge-engine/tests/trino_resilience.rs:979` | W:2659; U:2680 |
| `pagination_with_before_seq` | `crates/ontoforge-storage/tests/history_masking.rs:552` | W:3852; U:3875 |
| `params_hidden_from_ordinary_caller` | `crates/ontoforge-storage/tests/history_masking.rs:307` | W:3853; U:3876 |
| `params_report_all_errors` | `crates/ontoforge-action/tests/action.rs:92` | W:1951; U:1971 |
| `parses_full_example` | `crates/ontoforge-engine/tests/server_config.rs:121` | W:2514; U:2535 |
| `partial_failure_is_reported` | `crates/ontoforge-storage/tests/import.rs:829` | W:3878; U:3901 |
| `payload_is_minimal_and_versioned` | `crates/ontoforge-action/tests/outbox.rs:45` | W:2011; U:2031 |
| `payload_target_is_only_trusted_configuration` | `crates/ontoforge-action/tests/outbox.rs:616` | W:2012; U:2032 |
| `pg_migrate_rechecks_applied_roles` | `crates/ontoforge-storage/tests/migrate_pg.rs:278` | I:651 |
| `pg_response_lost_retry_reads_completed_receipt` | `crates/ontoforge-action/tests/idempotency_pg.rs:207` | I:799 |
| `pg_same_command_id_for_different_principals_is_independent` | `crates/ontoforge-action/tests/idempotency_pg.rs:341` | I:798 |
| `pg_same_key_different_params_has_one_winner_and_one_reused` | `crates/ontoforge-action/tests/idempotency_pg.rs:256` | I:801 |
| `pg_same_key_same_params_has_one_winner_and_one_replay` | `crates/ontoforge-action/tests/idempotency_pg.rs:240` | I:802 |
| `pg_scope_apply` | `crates/ontoforge-storage/tests/scope_store.rs:642` | I:675 |
| `pg_sessions_use_the_pinned_settings` | `crates/ontoforge-storage/tests/conformance_pg.rs:94` | I:621 |
| `pg_winner_abort_allows_loser_to_claim` | `crates/ontoforge-action/tests/idempotency_pg.rs:293` | I:803 |
| `physical_identifier_rules` | `crates/ontoforge-schema/tests/bindings.rs:598` | W:3215; U:3237 |
| `pk_queries_are_printed` | `crates/ontoforge-engine/tests/pk_check.rs:114` | W:2467; U:2488 |
| `plaintext_password_is_rejected` | `crates/ontoforge-engine/tests/server_config.rs:148` | W:2515; U:2536 |
| `plan_then_apply_aviation_scope` | `crates/ontoforge/tests/cli_scope.rs:152` | W:1848; U:1864 |
| `post_502_is_not_retried` | `crates/ontoforge-engine/tests/trino_resilience.rs:430` | W:2660; U:2681 |
| `post_503_starting_up_is_retried` | `crates/ontoforge-engine/tests/trino_resilience.rs:443` | W:2661; U:2682 |
| `post_plain_503_is_not_retried` | `crates/ontoforge-engine/tests/trino_resilience.rs:465` | W:2662; U:2683 |
| `postgres_outbox_after_send_crash_reclaims_with_same_payload` | `crates/ontoforge-action/tests/outbox_pg.rs:160` | I:818 |
| `postgres_outbox_parallel_workers_claim_one_generation` | `crates/ontoforge-action/tests/outbox_pg.rs:126` | I:817 |
| `precision_and_items_changes_are_destructive` | `crates/ontoforge-storage/tests/reload.rs:645` | W:3967; U:3990 |
| `precision_on_non_decimal_is_v06` | `crates/ontoforge-schema/tests/bindings.rs:833` | W:3216; U:3238 |
| `primary_key_change_archives_table` | `crates/ontoforge-storage/tests/migrate.rs:568` | W:3895; U:3918 |
| `principal_attrs_types` | `crates/ontoforge-engine/tests/server_config.rs:736` | W:2516; U:2537 |
| `principal_with_two_roles_gets_merged_flight_scope` | `crates/ontoforge-auth/tests/scope_compile.rs:100` | W:2224; U:2245 |
| `principal_without_roles_sees_nothing` | `crates/ontoforge-auth/tests/scope_compile.rs:133` | W:2225; U:2246 |
| `printed_literal_round_trips_through_sqlite` | `crates/ontoforge-engine/tests/sqlite_engine.rs:49` | W:2569; U:2590 |
| `printer::tests::print_trino_and_sqlite` | `crates/ontoforge-engine/src/printer/tests.rs:91` | W:2321; U:2342 |
| `printer_corpus_round_trips` | `crates/ontoforge-engine/tests/trino_it.rs:428` | I:1024 |
| `probe_timeout_keeps_pending` | `crates/ontoforge-storage/tests/reload.rs:1315` | W:3968; U:3991 |
| `probe_waits_until_visible` | `crates/ontoforge-storage/tests/reload.rs:1276` | W:3969; U:3992 |
| `production_mutation_callers_are_action_import_and_read_audit` | `crates/ontoforge-action/tests/entrypoints.rs:70` | W:1967; U:1987 |
| `profile_key_must_be_profile_name` | `crates/ontoforge-engine/tests/server_config.rs:266` | W:2518; U:2539 |
| `profile_precedence` | `crates/ontoforge/tests/cli.rs:150` | W:1667; U:1683 |
| `query_context_defaults` | `crates/ontoforge-engine/tests/sqlite_engine.rs:147` | W:2570; U:2591 |
| `query_context_from_config` | `crates/ontoforge-engine/tests/server_config.rs:294` | W:2519; U:2540 |
| `query_error_degrades_only_that_target` | `crates/ontoforge-engine/tests/drift.rs:232` | W:2375; U:2396 |
| `query_error_isolated_to_its_catalog` | `crates/ontoforge-engine/tests/bind_check_isolation.rs:112` | W:2355; U:2376 |
| `query_error_maps_to_query` | `crates/ontoforge-engine/tests/trino_client.rs:297` | W:2598; U:2619 |
| `reapply_after_success_is_rejected` | `crates/ontoforge-storage/tests/migrate.rs:188` | W:3896; U:3919 |
| `record_login_creates_user_once` | `crates/ontoforge-storage/tests/scope_store.rs:531` | W:4037; U:4060 |
| `rejected_arguments_leave_no_key_file_behind` | `crates/ontoforge/tests/cli_dev_token.rs:247` | W:1700; U:1716 |
| `relaxing_one_to_one_drops_unique_index` | `crates/ontoforge-storage/tests/reload.rs:726` | W:3970; U:3993 |
| `release_source_visibility_guard` | `crates/ontoforge-action/tests/source.rs:339` | W:2051; U:2071 |
| `reload_creates_row_filter_indexes` | `crates/ontoforge-storage/tests/reload.rs:1064` | W:3972; U:3995 |
| `reload_creates_tables` | `crates/ontoforge/tests/cli_reload.rs:50` | W:1824; U:1840 |
| `reload_rechecks_applied_roles` | `crates/ontoforge-storage/tests/reload.rs:1016` | W:3974; U:3997 |
| `reload_reports_role_breakage` | `crates/ontoforge/tests/cli_reload.rs:194` | W:1825; U:1841 |
| `reload_resumes_after_partial_ddl` | `crates/ontoforge-storage/tests/reload.rs:292` | W:3975; U:3998 |
| `removed_type_archived` | `crates/ontoforge-storage/tests/migrate.rs:605` | W:3897; U:3920 |
| `removing_indexed_keeps_index` | `crates/ontoforge-storage/tests/reload.rs:453` | W:3976; U:3999 |
| `rename_object_type_moves_table` | `crates/ontoforge-storage/tests/migrate.rs:431` | W:3898; U:3921 |
| `rename_property_moves_data` | `crates/ontoforge-storage/tests/migrate.rs:297` | W:3899; U:3922 |
| `rename_with_type_change_is_rejected` | `crates/ontoforge-storage/tests/migrate.rs:405` | W:3901; U:3924 |
| `replay_across_clock_and_definition_change_keeps_winner_time` | `crates/ontoforge-action/tests/idempotency.rs:237` | W:1977; U:1997 |
| `replay_cross_principal_lookup_matches_missing` | `crates/ontoforge-action/tests/idempotency.rs:482` | W:1988; U:2008 |
| `replay_detects_version_divergence` | `crates/ontoforge-storage/tests/conformance/mod.rs:1389` | W:3824; U:3847; I:617 |
| `replay_grant_removal_and_action_missing_deny_both_entries` | `crates/ontoforge-action/tests/idempotency.rs:289` | W:1990; U:2010 |
| `replay_rebuilds_edit_store` | `crates/ontoforge-storage/tests/conformance/mod.rs:1361` | W:3825; U:3848; I:619 |
| `replay_release_keeps_minimal_receipt_available` | `crates/ontoforge-action/tests/idempotency.rs:523` | W:1991; U:2011 |
| `replay_row_predicate_change_and_delete_keep_minimal_receipt` | `crates/ontoforge-action/tests/idempotency.rs:414` | W:1992; U:2012 |
| `request_headers` | `crates/ontoforge-engine/tests/trino_client.rs:115` | W:2599; U:2620 |
| `retries_respect_timeout` | `crates/ontoforge-engine/tests/trino_resilience.rs:626` | W:2663; U:2684 |
| `retry_after_is_honoured` | `crates/ontoforge-engine/tests/trino_resilience.rs:379` | W:2665; U:2686 |
| `role_in_ontology_dir_is_v03` | `crates/ontoforge-schema/tests/scope_documents.rs:103` | W:3397; U:3420 |
| `role_without_row_filter_sees_all_rows` | `crates/ontoforge-auth/tests/scope_merge.rs:53` | W:2247; U:2268 |
| `rowpred::value::tests::attrs_list_rejects_null_element` | `crates/ontoforge-expr/src/rowpred/value.rs:107` | W:2678; U:2699 |
| `rowpred::value::tests::attrs_missing_is_null` | `crates/ontoforge-expr/src/rowpred/value.rs:101` | W:2679; U:2700 |
| `rs256_token_becomes_principal` | `crates/ontoforge-auth/tests/oidc.rs:231` | W:2177; U:2198 |
| `rs256_token_with_dev_issuer_is_rejected` | `crates/ontoforge-auth/tests/dev_token.rs:326` | W:2147; U:2168 |
| `rust_eval_equals_sqlite` | `crates/ontoforge-auth/tests/rowpred_diff.rs:108` | W:2196; U:2217 |
| `same_commit_twice_creates_two_records` | `crates/ontoforge-storage/tests/conformance/mod.rs:1426` | W:3827; U:3850; I:618 |
| `same_property_from_two_roles_is_or` | `crates/ontoforge-auth/tests/scope_merge.rs:114` | W:2248; U:2269 |
| `scope_action_grant_is_checked_before_missing_target` | `crates/ontoforge-action/tests/security.rs:365` | W:2030; U:2050 |
| `scope_compile_diagnostics_json` | `crates/ontoforge-auth/tests/scope_compile.rs:412` | W:2228; U:2249 |
| `scope_criterion_false_uses_configured_message` | `crates/ontoforge-action/tests/security.rs:195` | W:2031; U:2051 |
| `scope_hidden_target_matches_missing_outward_shape` | `crates/ontoforge-action/tests/security.rs:142` | W:2033; U:2053 |
| `scope_link_requires_both_endpoints` | `crates/ontoforge-action/tests/security.rs:385` | W:2034; U:2054 |
| `scope_masked_criteria_is_denied_before_evaluation` | `crates/ontoforge-action/tests/security.rs:173` | W:2035; U:2055 |
| `scope_post_state_must_remain_visible` | `crates/ontoforge-action/tests/security.rs:233` | W:2037; U:2057 |
| `scope_schema_is_the_third_exported_file` | `crates/ontoforge-schema/tests/schema_export.rs:442` | W:3381; U:3404 |
| `scope_tables_written_only_by_scope_store` | `crates/ontoforge-storage/tests/scope_store.rs:473` | W:4038; U:4061 |
| `scope_write_column_must_be_visible` | `crates/ontoforge-action/tests/security.rs:279` | W:2039; U:2059 |
| `second_binding_for_role_is_v08` | `crates/ontoforge-schema/tests/scope_documents.rs:223` | W:3399; U:3422 |
| `second_reload_is_noop` | `crates/ontoforge-storage/tests/reload.rs:123` | W:3977; U:4000 |
| `second_reload_on_pg_is_noop` | `crates/ontoforge-storage/tests/reload_pg.rs:137` | I:666 |
| `second_set_patches_only_given_properties` | `crates/ontoforge-storage/tests/conformance/mod.rs:531` | W:3828; U:3851; I:622 |
| `secret_from_env` | `crates/ontoforge-engine/tests/server_config.rs:340` | W:2522; U:2543 |
| `secret_from_file_follows_symlink_and_trims` | `crates/ontoforge-engine/tests/server_config.rs:353` | W:2523; U:2544 |
| `secret_never_in_debug_or_errors` | `crates/ontoforge-engine/tests/server_config.rs:422` | W:2524; U:2545 |
| `secret_never_printed` | `crates/ontoforge/tests/cli_online_trino.rs:614` | W:1803; U:1819 |
| `select_one` | `crates/ontoforge-engine/tests/trino_it.rs:135` | I:1021 |
| `select_one_round_trip` | `crates/ontoforge-engine/tests/trino_client.rs:78` | W:2601; U:2622 |
| `serialized_history_never_contains_masked_values` | `crates/ontoforge-storage/tests/history_masking.rs:288` | W:3855; U:3878 |
| `server_starting_up_is_unavailable` | `crates/ontoforge-engine/tests/trino_resilience.rs:125` | W:2667; U:2688 |
| `set_session_header_is_ignored` | `crates/ontoforge-engine/tests/trino_client.rs:325` | W:2602; U:2623 |
| `shared_table_conflicts_are_errors` | `crates/ontoforge-engine/tests/sqlite_engine.rs:873` | W:2572; U:2593 |
| `shared_table_loaded_once` | `crates/ontoforge-engine/tests/sqlite_engine.rs:844` | W:2573; U:2594 |
| `snake_collision_is_v24` | `crates/ontoforge-storage/tests/reload.rs:522` | W:3978; U:4001 |
| `source_backed_query_token_roundtrip_and_source_drift` | `crates/ontoforge-action/tests/source.rs:402` | W:2052; U:2072 |
| `source_backed_type_is_rejected` | `crates/ontoforge-storage/tests/import.rs:479`; `crates/ontoforge/tests/cli_import.rs:203` | W:1717; U:1733 |
| `source_duplicate_is_ambiguous_without_tx` | `crates/ontoforge-action/tests/source.rs:297` | W:2053; U:2073 |
| `source_object_lifecycle` | `crates/ontoforge-storage/tests/conformance/mod.rs:1510` | W:3829; U:3852; I:623 |
| `source_required_offline_is_engine_unavailable_without_tx` | `crates/ontoforge-action/tests/source.rs:253` | W:2055; U:2075 |
| `source_type_without_binding_is_v16` | `crates/ontoforge-schema/tests/bindings.rs:379` | W:3219; U:3241 |
| `source_unique_is_overlaid_once` | `crates/ontoforge-action/tests/source.rs:312` | W:2056; U:2076 |
| `sql_text_only_from_printer` | `crates/ontoforge-engine/tests/sql_text_only_from_printer.rs:27` | W:2542; U:2563 |
| `sql_text_only_from_sql_exit` | `crates/ontoforge-storage/tests/sql_exit_guard.rs:54` | W:4046; U:4069 |
| `sqlite::tests::max_bytes_truncates` | `crates/ontoforge-engine/src/sqlite/tests.rs:123` | W:2329; U:2350 |
| `sqlite::tests::max_rows_truncates` | `crates/ontoforge-engine/src/sqlite/tests.rs:108` | W:2330; U:2351 |
| `sqlite::tests::query_only_after_load` | `crates/ontoforge-engine/src/sqlite/tests.rs:151` | W:2331; U:2352 |
| `sqlite::tests::sqlite_ddl_round_trip` | `crates/ontoforge-engine/src/sqlite/tests.rs:65` | W:2332; U:2353 |
| `sqlite::tests::total_fixture_bytes_capped` | `crates/ontoforge-engine/src/sqlite/tests.rs:394` | W:2334; U:2355 |
| `sqlite_profile_uses_fixtures` | `crates/ontoforge/tests/cli_online_trino.rs:768` | W:1804; U:1820 |
| `stale_def_version_rejected` | `crates/ontoforge-storage/tests/conformance/mod.rs:808` | W:3830; U:3853; I:624 |
| `stale_keys_survive_idp_outage` | `crates/ontoforge-auth/tests/oidc.rs:445` | W:2179; U:2200 |
| `stale_plan_hash_rejected` | `crates/ontoforge-storage/tests/scope_store.rs:275` | W:4039; U:4062 |
| `stale_writer_rejected_after_switch` | `crates/ontoforge-storage/tests/reload.rs:812` | W:3979; U:4002 |
| `status_available_and_unavailable` | `crates/ontoforge-engine/tests/trino_client.rs:892` | W:2603; U:2624 |
| `status_is_available` | `crates/ontoforge-engine/tests/sqlite_engine.rs:83` | W:2574; U:2595 |
| `status_reports_unavailable` | `crates/ontoforge-engine/tests/trino_resilience.rs:236` | W:2668; U:2689 |
| `store_postgres_requires_exactly_one_url_source` | `crates/ontoforge-engine/tests/server_config.rs:586` | W:2528; U:2549 |
| `store_schema_names_are_validated` | `crates/ontoforge-engine/tests/server_config.rs:632` | W:2530; U:2551 |
| `store_sqlite_defaults` | `crates/ontoforge-engine/tests/server_config.rs:562` | W:2531; U:2552 |
| `store_sqlite_rejects_url_and_postgres_rejects_path` | `crates/ontoforge-engine/tests/server_config.rs:603` | W:2532; U:2553 |
| `store_unavailable` | `crates/ontoforge/tests/cli_reload.rs:92` | W:1826; U:1842 |
| `stored_roles_diagnostics_without_sources` | `crates/ontoforge-auth/tests/scope_compile.rs:398` | W:2229; U:2250 |
| `string_literal_keeps_statement_structure` | `crates/ontoforge-engine/tests/printer_prop.rs:238` | W:2492; U:2513 |
| `string_literal_round_trips_on_sqlite` | `crates/ontoforge-engine/tests/printer_prop.rs:258` | W:2493; U:2514 |
| `string_match_is_case_sensitive_and_literal` | `crates/ontoforge-expr/tests/rowpred_eval.rs:243` | W:2750; U:2771 |
| `string_match_matches_rust_on_sqlite` | `crates/ontoforge-engine/tests/printer_prop.rs:208` | W:2494; U:2515 |
| `subject_rules` | `crates/ontoforge-schema/tests/scope_documents.rs:175` | W:3401; U:3424 |
| `syntax_error_is_query` | `crates/ontoforge-engine/tests/trino_resilience.rs:218` | W:2669; U:2690 |
| `system_column_collision_is_v24` | `crates/ontoforge-storage/tests/reload.rs:528` | W:3980; U:4003 |
| `tenant_equality_matches` | `crates/ontoforge-auth/tests/rowpred_diff.rs:54` | W:2197; U:2218 |
| `tenant_equality_visible` | `crates/ontoforge-expr/tests/rowpred_eval.rs:52` | W:2751; U:2772 |
| `tenant_mismatch_false` | `crates/ontoforge-expr/tests/rowpred_eval.rs:75` | W:2752; U:2773 |
| `tests::tracer_commits_one_action_batch` | `crates/ontoforge-action/src/lib.rs:610` | W:1940; U:1960 |
| `tightening_to_one_to_one_with_conflicts_keeps_old_definition` | `crates/ontoforge-storage/tests/reload.rs:689` | W:3982; U:4005 |
| `timeout_cancels_server_query` | `crates/ontoforge-engine/tests/trino_it.rs:468` | I:1023 |
| `timeout_flag_is_sent` | `crates/ontoforge/tests/cli_online_trino.rs:441` | W:1806; U:1822 |
| `timeout_sends_delete` | `crates/ontoforge-engine/tests/trino_resilience.rs:571` | W:2670; U:2691 |
| `timezone_value_rules` | `crates/ontoforge-schema/tests/bindings.rs:809` | W:3220; U:3243 |
| `token_conflicts_after_edit_scope_or_key_change` | `crates/ontoforge-action/tests/version.rs:268` | W:2074; U:2094 |
| `token_definition_change_is_rejected_without_action_writes` | `crates/ontoforge-action/tests/version.rs:235` | W:2075; U:2095 |
| `token_malformed_is_rejected_without_writes` | `crates/ontoforge-action/tests/version.rs:332` | W:2076; U:2096 |
| `token_query_issued_is_accepted_by_unchanged_action_read_set` | `crates/ontoforge-action/tests/version.rs:207` | W:2077; U:2097 |
| `tombstoned_object_is_never_revived` | `crates/ontoforge-storage/tests/import.rs:579` | W:3882; U:3905 |
| `toolset_value_is_checked` | `crates/ontoforge-schema/tests/scope_documents.rs:79` | W:3402; U:3425 |
| `top_level_fixtures_dir_still_loaded` | `crates/ontoforge-schema/tests/bindings.rs:243` | W:3221; U:3244 |
| `trino::cancel::tests::dropping_an_unfinished_guard_without_a_runtime_does_not_panic` | `crates/ontoforge-engine/src/trino/cancel.rs:179` | W:2336; U:2357 |
| `trino::headers::tests::basic_and_bearer_headers` | `crates/ontoforge-engine/src/trino/headers.rs:133` | W:2342; U:2363 |
| `trino_action_all_patched_does_not_contact_trino` | `crates/ontoforge-action/tests/trino_action_it.rs:469` | I:1237 |
| `trino_action_duplicate_pk_is_ambiguous` | `crates/ontoforge-action/tests/trino_action_it.rs:452` | I:1238 |
| `trino_action_source_unavailable_is_engine_unavailable` | `crates/ontoforge-action/tests/trino_action_it.rs:425` | I:1239 |
| `trino_action_unique_source_executes` | `crates/ontoforge-action/tests/trino_action_it.rs:358` | I:1236 |
| `trino_matches_rust_on_seeded_predicates` | `crates/ontoforge-auth/tests/rowpred_trino_it.rs:219` | I:1089 |
| `trino_matches_rust_on_the_corpus_cases` | `crates/ontoforge-auth/tests/rowpred_trino_it.rs:203` | I:1088 |
| `trino_missing_table` | `crates/ontoforge/tests/cli_online_trino.rs:357` | W:1807; U:1823 |
| `trino_profile_clean` | `crates/ontoforge/tests/cli_online_trino.rs:216` | W:1808; U:1824 |
| `trino_profile_missing_column` | `crates/ontoforge/tests/cli_online_trino.rs:290` | W:1809; U:1825 |
| `trino_table_equals_the_corpus_rows` | `crates/ontoforge-auth/tests/rowpred_trino_it.rs:195` | I:1087 |
| `trino_type_mismatch` | `crates/ontoforge/tests/cli_online_trino.rs:322` | W:1810; U:1826 |
| `trino_unavailable_exits_3` | `crates/ontoforge/tests/cli_online_trino.rs:532` | W:1811; U:1827 |
| `ttl_bounds` | `crates/ontoforge/tests/cli_dev_token.rs:194` | W:1702; U:1718 |
| `two_identities_use_separate_clients` | `crates/ontoforge-engine/tests/trino_client.rs:432` | W:2605; U:2626 |
| `two_issuers_namespace_groups` | `crates/ontoforge-auth/tests/oidc.rs:930` | W:2182; U:2203 |
| `type_change_archives_and_adds` | `crates/ontoforge-storage/tests/migrate.rs:539` | W:3902; U:3925 |
| `unauthorized_exits_3` | `crates/ontoforge/tests/cli_online_trino.rs:586` | W:1812; U:1828 |
| `unauthorized_is_not_unavailable` | `crates/ontoforge-engine/tests/trino_resilience.rs:198` | W:2671; U:2692 |
| `undeclared_attr_is_rejected` | `crates/ontoforge/tests/cli_dev_token.rs:178` | W:1703; U:1719 |
| `undeclared_attr_is_v22` | `crates/ontoforge-auth/tests/scope_compile.rs:293` | W:2230; U:2251 |
| `ungranted_object_type_is_v23` | `crates/ontoforge-auth/tests/criteria_coverage.rs:76` | W:2118; U:2138 |
| `unknown_backing_value_is_v06` | `crates/ontoforge-schema/tests/object_types.rs:268` | W:3361; U:3384 |
| `unknown_field_is_v04_with_location` | `crates/ontoforge-schema/tests/scope_documents.rs:53` | W:3403; U:3426 |
| `unknown_key_in_store_or_auth_is_v20` | `crates/ontoforge-engine/tests/server_config.rs:768` | W:2534; U:2555 |
| `unknown_key_is_rejected` | `crates/ontoforge-engine/tests/server_config.rs:175` | W:2535; U:2556 |
| `unknown_profile_from_flag_is_v16_with_suggestion` | `crates/ontoforge-schema/tests/bindings.rs:455` | W:3225; U:3248 |
| `unknown_profile_from_manifest_points_at_default_profile` | `crates/ontoforge-schema/tests/bindings.rs:469` | W:3226; U:3249 |
| `unknown_root_rejected` | `crates/ontoforge-expr/tests/rowpred_eval.rs:124` | W:2753; U:2774 |
| `unknown_target_is_v09` | `crates/ontoforge-schema/tests/bindings.rs:201` | W:3227; U:3250 |
| `unset_tombstone_release_entries` | `crates/ontoforge-storage/tests/history_masking.rs:491` | W:3856; U:3879 |
| `user_subject_binding` | `crates/ontoforge-auth/tests/scope_compile.rs:142` | W:2234; U:2255 |
| `validate_online_against_ci_trino` | `crates/ontoforge/tests/cli_trino_it.rs:42` | I:1059 |
| `validate_only_runs_full_pipeline_without_writes` | `crates/ontoforge-action/tests/action.rs:645` | W:1952; U:1972 |
| `value::tests::decimal_json_numbers_must_be_exact_integers` | `crates/ontoforge-storage/src/value.rs:615` | W:3722; U:3745 |
| `value::tests::json_round_trips_every_type` | `crates/ontoforge-storage/src/value.rs:554` | W:3726; U:3749 |
| `value::tests::physical_type_parse_table` | `crates/ontoforge-engine/src/value.rs:271` | W:2346; U:2367 |
| `value::tests::postgres_limits_are_validated` | `crates/ontoforge-storage/src/value.rs:654` | W:3729; U:3752 |
| `value::tests::split_decimal_rules` | `crates/ontoforge-engine/src/value.rs:344` | W:2347; U:2368 |
| `values_postgres_cannot_store_are_refused_on_every_backend` | `crates/ontoforge-storage/tests/conformance/mod.rs:716` | W:3832; U:3855; I:626 |
| `version_claims_are_canonical_and_format_tagged` | `crates/ontoforge-query/tests/security.rs:2081` | W:2958; U:2979 |
| `version_query_uses_format_tag_and_changes_with_scope` | `crates/ontoforge-query/tests/security.rs:2173` | W:2959; U:2980 |
| `visibility_uses_current_row` | `crates/ontoforge-storage/tests/history_masking.rs:430` | W:3857; U:3880 |
| `visible_columns_are_union_of_true_grants` | `crates/ontoforge-auth/tests/scope_merge.rs:208` | W:2249; U:2270 |
| `warnings_do_not_degrade` | `crates/ontoforge-engine/tests/drift.rs:386` | W:2377; U:2398 |
| `whole_object_read_needs_every_property` | `crates/ontoforge-auth/tests/criteria_coverage.rs:196` | W:2119; U:2139 |
| `wrong_key_rejected` | `crates/ontoforge-auth/tests/dev_token.rs:398` | W:2148; U:2169 |

## 后续最小动作与写入边界

1. 唯一 Cargo owner 在最终代码树串行补 G3（SQLite 5 个）和 G4（真实 PG 1 个），记录 named test、非零执行数、完整 exit 状态；失败须回流主会话 CC，不放宽断言。
2. 主会话获取最终源码有效 nightly 483/484 原始日志解决 G1；不要复用旧 SHA 39221684753，不把普通 integration 的早退 ok 计为 Iceberg通过。
3. G2 由主会话安排 hermetic 负向 guard 验证；若复现失败，修复属实现/CI文件写入，不能交本只读 auditor直接改。
4. 补证后再更新原 Phase VALIDATION 的 task map、Wave 0、status、审计轨迹；6 个历史决策/文档 task 保留 N/A/SKIP 理由。原 `nyquist_compliant` 不应仅因现有 87 行覆盖自动全绿。
5. 生产 webhook 实际部署边界继续 DEFERRED，发布前按原部署合同验收。

Files for review: `.planning/milestones/v1.0-audit/NYQUIST-02-03-05-VALIDATION.md`。没有新测试文件，没有实现修改，没有提交。
