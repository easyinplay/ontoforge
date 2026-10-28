---
phase: "07"
slug: "rest-mcp-shell"
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-10-29"
---

# Phase 7 Validation Strategy

## Test Infrastructure

| Property | Value |
|----------|-------|
| Framework | Rust cargo test, insta, proptest; Node client contract tests |
| Config file | Cargo.toml, .github/workflows/ci.yml; TS config 待创建 |
| 当前 quick command | `cargo test --manifest-path .planning/phases/07-rest-mcp-shell/spikes/rmcp-shell/Cargo.toml --locked` |
| 实施 quick command | `cargo test -p ontoforge-server --locked`，Wave 0 先创建非空测试入口 |
| Full suite command | `cargo test --workspace --locked -- --test-threads=1`，另运行 PG/Trino ignored suites、TS 与发行矩阵 |
| Estimated runtime | 正式 suite 尚未测量；执行时记录，不编造时限 |

## Sampling Rate

- 每个 task：先记录针对该行为的 RED，再记录 GREEN，拒绝 0 tests。
- 每个 wave：受影响 crate tests、workspace regression，以及适用的 fmt/clippy/TLS gate。
- `$gsd-verify-work` 前：完整矩阵必须有对应日志，未跑的环境不得标为 passed。
- 详细行为合同见 `07-TEST-MATRIX.md`，正式 task/plan/wave 映射由 planner 在 PLAN 生成后补全。

## Per-Task Verification Map

16份PLAN共32个task。revision1将原08-3全部行为拆至16-1/16-2；10/11显式依赖06的真实health实现。所有运行状态为pending；新测试由所属task先创建并形成可归因RED，再实现GREEN。Threat Ref为同plan STRIDE登记，精确行为见behavior与fails_when。

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---|---|---|---|---|---|---|---|---|---|
| 07-01-1 | 07-01 | 1 | API-01,API-03,SCOPE-07 | T-07-01,T-07-02,T-07-03 | tracer_authenticated_pk_is_durable_after_shutdown; tracer_unauthenticated_and_forged_context_are_denied; tracer_snapshot_releases_lock_before_source | RED→GREEN contract/integration | `cargo test -p ontoforge-server --lib --locked tracer_ -- --nocapture` | inline #[cfg(test)]由本task创建 | pending |
| 07-01-2 | 07-01 | 1 | API-01,API-03,SCOPE-07 | T-07-01,T-07-02,T-07-03 | serve_args_validate_config_and_address; serve_real_listener_auth_and_shutdown | RED→GREEN contract/integration | `cargo test -p ontoforge --lib --locked serve_ -- --nocapture` | inline #[cfg(test)]由本task创建 | pending |
| 07-02-1 | 07-02 | 2 | SCOPE-07,API-01,MCP-06 | T-07-04,T-07-05,T-07-06 | snapshot_sqlite_never_mixes_definition_roles_bindings; snapshot_lock_is_released_before_compile_or_network; snapshot_pg_never_mixes_definition_roles_bindings | RED→GREEN contract/integration | `cargo test -p ontoforge-storage --test request_snapshot --locked -- --nocapture` | crates/ontoforge-storage/tests/request_snapshot.rs: 缺失，由本task创建 | pending |
| 07-02-2 | 07-02 | 2 | SCOPE-07,API-01,MCP-06 | T-07-04,T-07-05,T-07-06 | frozen_action_stale_and_validate_keep_fence_cas_guard; frozen_replay_uses_current_grant_keeps_original_receipt; frozen_history_keeps_request_definition_and_current_scope | RED→GREEN contract/integration | `cargo test -p ontoforge-action --test frozen_definition --test idempotency --locked -- --nocapture`<br>`cargo test -p ontoforge-storage --test frozen_history --test history_masking --locked -- --nocapture` | crates/ontoforge-action/tests/frozen_definition.rs: 缺失，由本task创建<br>crates/ontoforge-storage/tests/frozen_history.rs: 缺失，由本task创建 | pending |
| 07-03-1 | 07-03 | 3 | API-01,API-03 | T-07-07,T-07-08,T-07-09 | rest_metadata_and_all_l0_operations; rest_equal_sort_keys_empty_nullable_and_pagination; rest_reload_and_revoke_freeze_inflight_only | RED→GREEN contract/integration | `cargo test -p ontoforge-server --lib --locked rest_ -- --nocapture` | inline #[cfg(test)]由本task创建 | pending |
| 07-03-2 | 07-03 | 3 | API-01,API-03 | T-07-07,T-07-08,T-07-09 | http_framework_rejections_are_structured; http_error_utf8_ids_and_l0_redaction; http_rows_bytes_concurrency_timeout_and_audit_fail_closed | RED→GREEN contract/integration | `cargo test -p ontoforge-server --test http_contract --locked -- --nocapture` | crates/ontoforge-server/tests/http_contract.rs: 缺失，由本task创建 | pending |
| 07-04-1 | 07-04 | 4 | API-01,API-03,MCP-06,SCOPE-07 | T-07-10,T-07-11,T-07-12 | http_action_validate_submit_history_result; http_action_empty_invalid_and_masked_history | RED→GREEN contract/integration | `cargo test -p ontoforge-server --test http_actions --locked -- --nocapture` | crates/ontoforge-server/tests/http_actions.rs: 缺失，由本task创建<br>crates/ontoforge-server/tests/support/mod.rs: 缺失，由本task创建 | pending |
| 07-04-2 | 07-04 | 4 | API-01,API-03,MCP-06,SCOPE-07 | T-07-10,T-07-11,T-07-12 | http_same_command_race_one_commit_original_receipt; http_replay_revoke_cross_principal_and_changed_params; http_stale_and_validate_conflicts_never_retry | RED→GREEN contract/integration | `cargo test -p ontoforge-server --test http_action_safety --locked -- --nocapture` | crates/ontoforge-server/tests/http_action_safety.rs: 缺失，由本task创建 | pending |
| 07-05-1 | 07-05 | 5 | API-02,API-01,API-03 | T-07-13,T-07-14,T-07-15 | openapi_31_all_operations_and_security; openapi_recursive_nullable_required_and_unknown_fields; wire_numbers_unicode_and_variant_names | RED→GREEN contract/integration | `cargo test -p ontoforge-api --test openapi_snapshot --locked -- --nocapture` | crates/ontoforge-api/tests/openapi_snapshot.rs: 缺失，由本task创建 | pending |
| 07-05-2 | 07-05 | 5 | API-02,API-01,API-03 | T-07-13,T-07-14,T-07-15 | openapi_export_without_store_is_deterministic | RED→GREEN contract/integration | `cargo test -p ontoforge --lib --locked openapi_ -- --nocapture`<br>`cargo test -p ontoforge-api --test openapi_snapshot --locked -- --nocapture` | inline #[cfg(test)]由本task创建 | pending |
| 07-06-1 | 07-06 | 6 | L2-04,API-01,API-03 | T-07-16,T-07-17,T-07-18 | health_unconfigured_optional_l2_is_healthy; health_l2_failure_and_expiry_are_200_degraded; health_core_failure_is_503_and_recovery_is_visible; startup_partial_failure_cleans_monitor_and_store | RED→GREEN contract/integration | `cargo test -p ontoforge-server --test health_lifecycle_it --locked -- --nocapture` | crates/ontoforge-server/tests/health_lifecycle_it.rs: 缺失，由本task创建 | pending |
| 07-06-2 | 07-06 | 6 | L2-04,API-01,API-03 | T-07-16,T-07-17,T-07-18 | shutdown_finishes_accepted_action_before_dependency_cancel; shutdown_cancels_reads_sse_then_durably_drains_audit; shutdown_timeout_sink_and_cleanup_errors_exit_nonzero | RED→GREEN contract/integration | `cargo test -p ontoforge --test cli_shutdown --locked -- --nocapture --test-threads=1` | crates/ontoforge/tests/cli_shutdown.rs: 缺失，由本task创建 | pending |
| 07-07-1 | 07-07 | 7 | MCP-01,MCP-02,MCP-03,MCP-04,MCP-05,MCP-06,MCP-07,MCP-08,SCOPE-07 | T-07-19,T-07-20,T-07-21 | mcp_core_full_describe_query_history_equal_rest; mcp_list_filters_before_keyset_and_direct_call_denies; mcp_inflight_snapshot_and_revoke_on_next_call | RED→GREEN contract/integration | `cargo test -p ontoforge-server --test mcp_contract --locked -- --nocapture` | crates/ontoforge-server/tests/mcp_contract.rs: 缺失，由本task创建<br>crates/ontoforge-server/tests/support/mod.rs: 缺失，由本task创建 | pending |
| 07-07-2 | 07-07 | 7 | MCP-01,MCP-02,MCP-03,MCP-04,MCP-05,MCP-06,MCP-07,MCP-08,SCOPE-07 | T-07-19,T-07-20,T-07-21 | mcp_dynamic_action_schema_matches_real_validation; mcp_validate_only_keeps_fence_cas_guards_and_zero_writes; mcp_rest_same_command_race_and_receipt | RED→GREEN contract/integration | `cargo test -p ontoforge-server --test mcp_actions --locked -- --nocapture` | crates/ontoforge-server/tests/mcp_actions.rs: 缺失，由本task创建 | pending |
| 07-08-1 | 07-08 | 8 | MCP-01,MCP-06,MCP-07,MCP-08,L2-04,SCOPE-07 | T-07-22,T-07-23,T-07-24 | mcp_l2_all_six_names_parameters_and_allowlist; mcp_l2_gate_expiry_revoke_and_audit | RED→GREEN contract/integration | `cargo test -p ontoforge-server --test mcp_l2 --locked -- --nocapture` | crates/ontoforge-server/tests/mcp_l2.rs: 缺失，由本task创建 | pending |
| 07-08-2 | 07-08 | 8 | MCP-01,MCP-06,MCP-07,MCP-08,L2-04,SCOPE-07 | T-07-22,T-07-23,T-07-24 | mcp_old_session_reauth_owner_ttl_capacity_delete; mcp_new_protocol_without_session_requires_metadata; mcp_cancel_disconnect_sse_do_not_replay_action | RED→GREEN contract/integration | `cargo test -p ontoforge-server --test mcp_transport --locked -- --nocapture` | crates/ontoforge-server/tests/mcp_transport.rs: 缺失，由本task创建 | pending |
| 07-09-1 | 07-09 | 10 | MCP-01,MCP-06,MCP-07,SCOPE-07 | T-07-25,T-07-26,T-07-27 | bridge_uses_same_service_no_redirect_or_reinit; bridge_parallel_ids_cancel_and_eof | RED→GREEN contract/integration | `cargo test -p ontoforge --lib --locked bridge_ -- --nocapture` | inline #[cfg(test)]由本task创建 | pending |
| 07-09-2 | 07-09 | 10 | MCP-01,MCP-06,MCP-07,SCOPE-07 | T-07-25,T-07-26,T-07-27 | stdio_real_process_describe_query_action_dryrun_history; stdio_server_absent_bad_token_redirect_and_expired_session; stdio_long_query_cancel_eof_and_shutdown | RED→GREEN contract/integration | `cargo test -p ontoforge --test cli_stdio --locked -- --nocapture --test-threads=1` | crates/ontoforge/tests/cli_stdio.rs: 缺失，由本task创建 | pending |
| 07-10-1 | 07-10 | 7 | API-04 | T-07-28,T-07-29,T-07-30 | rust_client_metadata_pk_query_aggregate_health; rust_client_instances_and_concurrent_cancellation_are_isolated | RED→GREEN contract/integration | `cargo test -p ontoforge-client --test contract --locked -- --nocapture` | crates/ontoforge-client/tests/contract.rs: 缺失，由本task创建<br>crates/ontoforge-client/tests/support/mod.rs: 缺失，由本task创建 | pending |
| 07-10-2 | 07-10 | 7 | API-04 | T-07-28,T-07-29,T-07-30 | rust_client_action_validate_history_result_and_replay; rust_client_auth_scope_revoke_stale_and_transport_errors; rust_client_wire_numbers_and_all_endpoint_groups | RED→GREEN contract/integration | `cargo test -p ontoforge-client --test contract --locked -- --nocapture` | crates/ontoforge-client/tests/contract.rs: 缺失，由本task创建 | pending |
| 07-11-1 | 07-11 | 7 | API-05 | T-07-31,T-07-32,T-07-33 | ts_client_generated_metadata_pk_query_aggregate_health; ts_client_typecheck_generation_and_instance_abort | RED→GREEN contract/integration | `npm --prefix clients/typescript run test:contract`<br>`npm --prefix clients/typescript run typecheck`<br>`npm --prefix clients/typescript run check:generated` | clients/typescript/tests/contract.test.mjs: 缺失，由本task创建 | pending |
| 07-11-2 | 07-11 | 7 | API-05 | T-07-31,T-07-32,T-07-33 | ts_lossless_integer_decimal_boundary_roundtrips; ts_client_action_validate_history_result_and_replay; ts_client_auth_scope_stale_errors_limits_and_runtime_capability | RED→GREEN contract/integration | `npm --prefix clients/typescript run test:contract`<br>`npm --prefix clients/typescript run check:generated`<br>`npm --prefix clients/typescript run typecheck` | clients/typescript/tests/contract.test.mjs: 缺失，由本task创建 | pending |
| 07-12-1 | 07-12 | 11 | DX-01,DX-02 | T-07-34,T-07-35,T-07-36 | dev_restart_preserves_action_history_and_receipt; dev_empty_seed_is_idempotent_and_reset_is_explicit; dev_active_instance_and_unsafe_reset_are_rejected | RED→GREEN contract/integration | `cargo test -p ontoforge --test dev_lifecycle --locked -- --nocapture --test-threads=1` | crates/ontoforge/tests/dev_lifecycle.rs: 缺失，由本task创建 | pending |
| 07-12-2 | 07-12 | 11 | DX-01,DX-02 | T-07-34,T-07-35,T-07-36 | dev_distributed_binary_query_action_history_without_checkout; dev_demo_restart_and_reset_preserve_then_reinitialize | RED→GREEN contract/integration | `cargo test -p ontoforge --test dev_lifecycle --locked -- --nocapture --test-threads=1` | crates/ontoforge/tests/dev_lifecycle.rs: 缺失，由本task创建 | pending |
| 07-13-1 | 07-13 | 11 | API-01,API-03,MCP-01,MCP-06,MCP-07,MCP-08,L2-04,SCOPE-07 | T-07-37,T-07-38,T-07-39 | pg_http_mixed_snapshot_reload_revoke_and_history; pg_http_action_validate_stale_replay_atomicity; trino_http_long_read_releases_snapshot_lock; trino_http_and_mcp_six_l2_tools_all_formats | RED→GREEN contract/integration | `cargo test -p ontoforge-server --test postgres_http_it --locked -- --ignored --nocapture --test-threads=1`<br>`cargo test -p ontoforge-storage --test request_snapshot --locked -- --ignored --nocapture --test-threads=1`<br>`cargo test -p ontoforge-server --test trino_http_it --locked -- --ignored --nocapture --test-threads=1` | crates/ontoforge-server/tests/postgres_http_it.rs: 缺失，由本task创建<br>crates/ontoforge-server/tests/trino_http_it.rs: 缺失，由本task创建<br>crates/ontoforge-server/tests/support/mod.rs: 缺失，由本task创建 | pending |
| 07-13-2 | 07-13 | 11 | API-01,API-03,MCP-01,MCP-06,MCP-07,MCP-08,L2-04,SCOPE-07 | T-07-37,T-07-38,T-07-39 | real_health_l2_failure_expiry_recovery_keeps_core; real_shutdown_action_dependency_audit_and_cleanup; ci_phase7_requires_named_nonzero_suites | RED→GREEN contract/integration | `bash ci/server-services.sh verify` | crates/ontoforge-server/tests/health_trino_it.rs: 缺失，由本task创建 | pending |
| 07-14-1 | 07-14 | 12 | DX-04,DX-01,DX-02,API-04,API-05 | T-07-40,T-07-41,T-07-42 | 实际binary/archive/checksum及隔离cwd完整smoke | artifact/process smoke | `pwsh -File ci/release-package.ps1 -Target x86_64-pc-windows-msvc -OutDir target/release-dist` | ci/release-package.ps1: 缺失，由本task创建 | pending |
| 07-14-2 | 07-14 | 12 | DX-04,DX-01,DX-02,API-04,API-05 | T-07-40,T-07-41,T-07-42 | 实际binary/archive/checksum及隔离cwd完整smoke | artifact/process smoke | `pwsh -File ci/release-smoke.ps1 -ArtifactDir target/release-dist -ExpectedTarget x86_64-pc-windows-msvc` | ci/release-smoke.ps1: 缺失，由本task创建 | pending |
| 07-15-1 | 07-15 | 6 | MCP-07 | T-07-43,T-07-44,T-07-45 | l2_explain_four_types_preserve_validated_ast; l2_table_name_defaults_qualification_and_injection | RED→GREEN contract/integration | `cargo test -p ontoforge-engine --test l2_explain --test l2_gate --locked -- --nocapture` | crates/ontoforge-engine/tests/l2_explain.rs: 缺失，由本task创建 | pending |
| 07-15-2 | 07-15 | 6 | MCP-07 | T-07-43,T-07-44,T-07-45 | l2_explain_all_types_share_admission_and_audit; l2_explain_old_wrapper_and_rejection_contract; l2_explain_all_types_timeout_cancel_concurrency | RED→GREEN contract/integration | `cargo test -p ontoforge-query --test l2_execute --locked -- --nocapture` | crates/ontoforge-query/tests/l2_execute.rs: 已存在，扩展 | pending |
| 07-16-1 | 07-16 | 9 | MCP-07,SCOPE-07 | T-07-46,T-07-47 | mcp_l2_optional_context_defaults_and_qualified_override | RED→GREEN contract/integration | `cargo test -p ontoforge-server --test mcp_l2_defaults --locked mcp_l2_optional_context_defaults_and_qualified_override -- --nocapture` | crates/ontoforge-server/tests/mcp_l2_defaults.rs: 缺失，由本task创建 | pending |
| 07-16-2 | 07-16 | 9 | MCP-07,SCOPE-07 | T-07-46,T-07-47 | mcp_l2_missing_context_and_outside_allowlist_deny_before_send | RED→GREEN contract/integration | `cargo test -p ontoforge-server --test mcp_l2_defaults --locked -- --nocapture` | crates/ontoforge-server/tests/mcp_l2_defaults.rs: 缺失，由本task创建 | pending |

补充门：11的typecheck必须tsc无diagnostic，check:generated必须真实重生成无diff；14以实际archive/manifest/checksum/binary smoke为判据，不套用libtest零用例规则。13真实TTL测试需覆盖产品60秒期限，归独立有界integration job，尚未实际测量耗时。真实Trino四EXPLAIN需记录VALIDATE/IO响应及source执行证据；不新增audit format字段或迁移。

## Wave 0 Requirements

- [ ] 创建真实 listener/auth/audit fixture 与非空 server contract suites。
- [ ] 创建 storage/action snapshot race fixtures，SQLite 与 PG 同合同。
- [ ] 创建 OpenAPI、Rust/TS clients 的生成与 contract test 入口。
- [ ] 创建 CLI 子进程、health/shutdown 与 release smoke 测试入口。
- 新测试路径必须在所属 PLAN task 创建后再运行；不得用缺失路径或零测试作为通过证据。

## Manual-Only Verifications

当前无指定纯人工验收。三平台构建由 CI 自动化；本机只装 Windows target，远端 CI 未完成前保持 pending。用户此前接受的产品合同不重复询问。

## Validation Sign-Off

- [ ] 所有 task 具有 automated verify 或明确 Wave 0 依赖。
- [ ] 每条 runnable verify 有 fails_when，且不能零测试通过。
- [ ] 不存在连续三个 task 无自动化验证。
- [ ] PG/Trino、clients、release 证据可追溯。
- [ ] 反馈耗时经实际测量。
- [ ] nyquist_compliant 由正式审核设置，本文件不提前宣称通过。

Approval: pending
