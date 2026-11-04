---
phase: "05"
slug: "online-action"
status: validated
nyquist_compliant: true
wave_0_complete: true
created: "2026-10-23"
validated: "2026-11-04"
evidence_head: "f8851ca073fed34e80e605f0ac91880259df7e12"
ci_run: 41759804733
task_coverage: "14/14 executable"
---

# Phase 05 Validation Strategy

本次完成历史task级覆盖审计，替换已失真的draft/W0 pending标记。依据为实际源码、具名测试断言及原始CI/fresh日志；未运行Cargo或启动服务。全量97-task历史矩阵保留在 `../../milestones/v1.0-audit/NYQUIST-02-03-05-VALIDATION.md`，这里是本阶段更新后的事实源。

## Test Infrastructure / Sampling

| Property | Value |
|---|---|
| Framework | Rust cargo test、tokio、proptest；HTTP使用wiremock |
| Quick command | `cargo test -p ontoforge-action --test action --locked`；按task选择具名测试 |
| Full command | `cargo test --workspace --locked -- --test-threads=1`；外部PG/Trino使用CI专门target与显式环境 |
| CI identity | run41759804733，head f8851ca，合成merge22913c8；四必需job success |
| Measured workspace | Windows150组1496passed/0failed/117ignored；Ubuntu150组1503/0/117；只计workspace step |
| Feedback latency | 窄测试运行见原始日志；全workspace含编译Windows约18m12s、Ubuntu约8m55s，不虚称满足旧180s估计 |

每task先用相应具名测试；每wave跑受影响crate/后端；收口使用已经完成的新HEAD CI，不重复全量执行。历史执行期每次反馈时延无法倒推，不声明当时每次都小于阈值；Nyquist结论是当前requirement→test覆盖，非虚构历史时序。

## Per-Task Verification Map

W/U/I/S是历史run41669982375四份原始日志；全部当前受核生产输入未变或已由新run覆盖。下表原有COVERED行复用前独立auditor的源码和冻结日志映射，不声称本worker重跑。新增G2/G3/G4使用本轮日志；具体行号与原名索引留在97-task矩阵。

| Task | Requirement | 行为 | 命名测试 | 可执行命令 | 结论 |
|---|---|---|---|---|---|
| 05-01-T1 | ACTION-02, ACTION-05, ACTION-06, ACTION-10, ACTION-11 | edit-only Action 批次贯穿请求、definition fence、command claim、edit log 与 outbox | `tests::tracer_commits_one_action_batch`; `action_request_wire_cannot_override_host_limits` | `cargo test -p ontoforge-action --lib --locked tracer_commits_one_action_batch -- --nocapture ; cargo test -p ontoforge-action --lib --locked -- --nocapture` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 05-01-T2 | ACTION-02, ACTION-05, ACTION-06, ACTION-10, ACTION-11 | PostgreSQL schema parity 与 batch happy-path conformance | `action_batch_writes_one_command_and_outbox`; `action_batch_principal_scopes_command_id`; `action_batch_stale_definition_leaves_no_action_rows`; `action_batch_migration_constraints_match_contract` | `cargo test -p ontoforge-storage --test conformance_sqlite --locked action_batch_ ; cargo test -p ontoforge-storage --test conformance_pg --locked action_batch_ -- --ignored` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 05-02-T1 | ACTION-01, ACTION-02, ACTION-03, ACTION-04, ACTION-05, ACTION-06, ACTION-08 | 参数、default、五类 rule 与确定性 batch/edit 摘要 | `params_report_all_errors`; `batch_ordinal_and_same_pk_order_are_stable`; `batch_all_rule_kinds_expand_in_declared_order` | `cargo test -p ontoforge-action --test action --locked params_ -- --nocapture ; cargo test -p ontoforge-action --test action --locked batch_ -- --nocapture` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 05-02-T2 | ACTION-01, ACTION-02, ACTION-03, ACTION-04, ACTION-05, ACTION-06, ACTION-08 | masked criteria、写前/写后 scope、link/release 与 validate-only | `validate_only_runs_full_pipeline_without_writes`; `scope_hidden_target_matches_missing_outward_shape`; `scope_masked_criteria_is_denied_before_evaluation`; `scope_criterion_false_uses_configured_message`; `scope_write_column_must_be_visible`; `scope_post_state_must_remain_visible`; `scope_action_grant_is_checked_before_missing_target`; `scope_link_requires_both_endpoints` | `cargo test -p ontoforge-action --test security --locked scope_ -- --nocapture ; cargo test -p ontoforge-action --test action --locked validate_only_ -- --nocapture ; cargo test -p ontoforge-action --locked -- --nocapture` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 05-03-T1 | ACTION-09 | 提取显式版本的共享 `_version` codec 并切换 query 签发 | `version_claims_are_canonical_and_format_tagged`; `version_query_uses_format_tag_and_changes_with_scope`; `token_conflicts_after_edit_scope_or_key_change`; `token_malformed_is_rejected_without_writes` | `cargo test -p ontoforge-query --test security --locked version_ -- --nocapture ; cargo test -p ontoforge-query --locked -- --nocapture` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 05-03-T2 | ACTION-09 | 锁后验证 token 与无 token observed-version CAS | `token_query_issued_is_accepted_by_unchanged_action_read_set`; `token_conflicts_after_edit_scope_or_key_change`; `token_definition_change_is_rejected_without_action_writes`; `observed_no_token_commits_with_current_read_set`; `action_cas_rejects_stale_no_token_without_side_effects`; `action_cas_conflict_rolls_back_version_zero_skeleton`; `source_backed_query_token_roundtrip_and_source_drift` | `cargo test -p ontoforge-action --test version --locked ; cargo test -p ontoforge-storage --test conformance_sqlite --locked action_cas_ ; cargo test -p ontoforge-storage --test conformance_pg --locked action_cas_ -- --ignored` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 05-04-T1 | ACTION-04, ACTION-08, ACTION-09 | 双后端逐阶段故障注入与全量 rollback conformance | `action_atomic_after_log_failure`; `action_atomic_after_edits_failure`; `action_atomic_after_command_failure`; `action_atomic_after_outbox_failure`; `action_atomic_success_changes_all_domains_once` | `cargo test -p ontoforge-storage --features test-support --test conformance_sqlite --locked action_atomic_ -- --nocapture ; cargo test -p ontoforge-storage --features test-support --test conformance_pg --locked action_atomic_ -- --ignored --nocapture` | COVERED: G3已闭合；新本地SQLite5/5、新CI双平台各5/5、真实PG5/5，全部实际执行。 |
| 05-04-T2 | ACTION-04, ACTION-08, ACTION-09 | 真实 PostgreSQL 同 pk token/no-token 竞态 | `concurrent_same_pk_with_token_has_one_winner`; `concurrent_same_pk_without_token_has_one_winner`; `concurrent_multirow_reverse_request_order_does_not_deadlock`; `concurrent_absent_edit_create_is_serialized` | `cargo test -p ontoforge-action --features test-support --test action_pg --locked concurrent_ -- --ignored --nocapture` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 05-05-T1 | ACTION-10 | PostgreSQL command claim winner/loser 与响应丢失重试 | `pg_same_key_same_params_has_one_winner_and_one_replay`; `pg_same_key_different_params_has_one_winner_and_one_reused`; `pg_winner_abort_allows_loser_to_claim`; `pg_response_lost_retry_reads_completed_receipt`; `pg_same_command_id_for_different_principals_is_independent` | `cargo test -p ontoforge-action --features test-support --test idempotency_pg --locked pg_ -- --ignored --nocapture` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 05-05-T2 | ACTION-10 | canonical fingerprint、过期与当前授权回放合同 | `expired_command_can_be_claimed_as_new_without_deleting_audit`; `fingerprint_ignores_key_order_but_preserves_explicit_default`; `replay_across_clock_and_definition_change_keeps_winner_time`; `replay_grant_removal_and_action_missing_deny_both_entries`; `replay_row_predicate_change_and_delete_keep_minimal_receipt`; `replay_cross_principal_lookup_matches_missing`; `replay_release_keeps_minimal_receipt_available` | `cargo test -p ontoforge-action --test idempotency --locked fingerprint_ -- --nocapture ; cargo test -p ontoforge-action --test idempotency --locked replay_ -- --nocapture ; cargo test -p ontoforge-action --test idempotency --locked expired_ -- --nocapture` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 05-06-T1 | ACTION-08, ACTION-09, ACTION-12 | fake Engine 固定 required、offline 与 observed-duplicate 三分支 | `all_patched_object_succeeds_with_engine_offline_and_zero_calls`; `source_required_offline_is_engine_unavailable_without_tx`; `release_source_visibility_guard`; `batch_deduplicates_source_keys_and_enforces_limit`; `source_duplicate_is_ambiguous_without_tx`; `source_unique_is_overlaid_once`; `source_backed_query_token_roundtrip_and_source_drift`; `later_alias_criterion_samples_shared_source_key`; `later_alias_release_samples_shared_source_key`; `later_item_token_samples_shared_source_key`; `source_guard_absent_target_insert_race` | `cargo test -p ontoforge-action --test source --locked ; cargo test -p ontoforge-action --features embedded-pg --test source --locked source_guard_absent_target_insert_race -- --exact --nocapture` | COVERED: G4已闭合；常规source/alias测试和真实PG source_guard_absent_target_insert_race均实际通过；本地及新CI精确1/1、非ignored。 |
| 05-06-T2 | ACTION-08, ACTION-09, ACTION-12 | 真实 Trino Action source 与 duplicate conformance | `trino_action_unique_source_executes`; `trino_action_source_unavailable_is_engine_unavailable`; `trino_action_duplicate_pk_is_ambiguous`; `trino_action_all_patched_does_not_contact_trino` | `cargo test -p ontoforge-action --test trino_action_it --locked trino_action_ -- --ignored --nocapture` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 05-07-T1 | ACTION-07, ACTION-11 | outbox lease、retry 与 crash recovery worker | `http_failure_retries_with_same_event_id`; `crash_after_success_before_mark_redelivers_same_event_id`; `crash_before_send_committed_event_survives_restart`; `lease_expiry_reclaims_and_stale_attempt_is_fenced`; `lease_parallel_workers_claim_one_attempt`; `payload_is_minimal_and_versioned`; `payload_target_is_only_trusted_configuration`; `postgres_outbox_parallel_workers_claim_one_generation`; `postgres_outbox_after_send_crash_reclaims_with_same_payload` | `cargo test -p ontoforge-action --test outbox --locked ; cargo test -p ontoforge-action --test outbox_pg --locked -- --ignored` | COVERED: 命名行为测试在冻结源码 CI 中通过 |
| 05-07-T2 | ACTION-07, ACTION-11 | production mutation 入口审计与 Action 后端 CI gates | `production_mutation_callers_are_action_import_and_read_audit`; `action_request_rejects_privileged_write_fields`; `import_log_has_reserved_identity_and_source_digest` | `cargo test -p ontoforge-action --test entrypoints --locked ; cargo test -p ontoforge --test cli_webhook_worker --locked` | COVERED: 命名行为测试在冻结源码 CI 中通过 |

## Wave 0 Closure

- [x] 所列生产模块与实际test target存在、接线完整；旧独立replay/append_only target已合并进conformance，不继续标MISSING。
- [x] Action harness、双后端逐阶段fault、PG双连接Barrier、wiremock/restart corpus已执行。
- [x] CI真实服务targets发现非零后实际运行；SQLite fault显式test-support；PG source guard非ignored精确执行。
- [x] G2 TLS guard真实workflow hermetic6/6、真实dependency helper和新双平台CI完成。

## Requirement Coverage / Limits

14个task全部COVERED，ACTION-01..12均有自动化行为证据。G3 SQLite fault与G4 PG absent-target证据缺口已关闭；nyquist_compliant:true。生产webhook接收端部署/服务管理器/真实下游联调仍是既有release前提，开发合同的CLI worker与真实PG/outbox证据不替代部署验收。

## Manual-Only Verifications

None for phase development scope。生产接收端部署沿既有release边界追踪，不新增人工审批。

## Validation Sign-Off

- [x] 每个task映射到行为、命名测试与可执行入口；静态任务单独说明。
- [x] 无三项连续可执行task缺少自动验证；无watch模式。
- [x] Wave0缺失引用全部按真实目标解析。
- [x] 运行证据区分ignored/list、历史CI、新CI及fresh运行。
- [x] 全需求自动化覆盖：ACTION-01..12已覆盖。

**Approval:** validated。此状态来自证据审计；不代表用户逐项手工观察。原始draft未作为完成证据。

## Validation Audit 2026-11-04

| Metric | Count |
|---|---|
| Gaps found | 2 |
| Resolved | 2 |
| Escalated | 0 |
