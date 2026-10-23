---
phase: 05-online-action
verified: 2026-10-23T17:17:15Z
status: human_needed
score: 5/5 must-haves verified
behavior_unverified: 0
overrides_applied: 0
covered_files: [".github/workflows/ci.yml",".github/workflows/nightly.yml",".planning/phases/05-online-action/05-01-PLAN.md",".planning/phases/05-online-action/05-01-SUMMARY.md",".planning/phases/05-online-action/05-02-PLAN.md",".planning/phases/05-online-action/05-02-SUMMARY.md",".planning/phases/05-online-action/05-03-PLAN.md",".planning/phases/05-online-action/05-03-SUMMARY.md",".planning/phases/05-online-action/05-04-PLAN.md",".planning/phases/05-online-action/05-04-SUMMARY.md",".planning/phases/05-online-action/05-05-PLAN.md",".planning/phases/05-online-action/05-05-SUMMARY.md",".planning/phases/05-online-action/05-06-PLAN.md",".planning/phases/05-online-action/05-06-SUMMARY.md",".planning/phases/05-online-action/05-07-PLAN.md",".planning/phases/05-online-action/05-07-SUMMARY.md","Cargo.lock","crates/ontoforge-action/Cargo.toml","crates/ontoforge-action/src/criteria.rs","crates/ontoforge-action/src/fingerprint.rs","crates/ontoforge-action/src/lib.rs","crates/ontoforge-action/src/outbox.rs","crates/ontoforge-action/src/params.rs","crates/ontoforge-action/src/rules.rs","crates/ontoforge-action/src/source.rs","crates/ontoforge-action/src/version.rs","crates/ontoforge-action/tests/action.rs","crates/ontoforge-action/tests/action_pg.rs","crates/ontoforge-action/tests/entrypoints.rs","crates/ontoforge-action/tests/idempotency.rs","crates/ontoforge-action/tests/idempotency_pg.rs","crates/ontoforge-action/tests/outbox.rs","crates/ontoforge-action/tests/outbox_pg.rs","crates/ontoforge-action/tests/security.rs","crates/ontoforge-action/tests/source.rs","crates/ontoforge-action/tests/trino_action_it.rs","crates/ontoforge-action/tests/version.rs","crates/ontoforge-engine/src/config.rs","crates/ontoforge-query/Cargo.toml","crates/ontoforge-query/src/cursor.rs","crates/ontoforge-query/src/lib.rs","crates/ontoforge-query/tests/pk_read.rs","crates/ontoforge-query/tests/security.rs","crates/ontoforge-schema/src/ir/action.rs","crates/ontoforge-schema/src/ir/mod.rs","crates/ontoforge-schema/src/lint/e08.rs","crates/ontoforge-schema/src/lint/structure/w09.rs","crates/ontoforge-schema/src/loader/dispatch.rs","crates/ontoforge-schema/src/validate/actions.rs","crates/ontoforge-schema/tests/action_types.rs","crates/ontoforge-storage/Cargo.toml","crates/ontoforge-storage/migrations/postgres/0004_action.sql","crates/ontoforge-storage/migrations/sqlite/0004_action.sql","crates/ontoforge-storage/src/commit.rs","crates/ontoforge-storage/src/import.rs","crates/ontoforge-storage/tests/conformance/mod.rs","crates/ontoforge/WEBHOOK-WORKER.md","crates/ontoforge/src/cli.rs","crates/ontoforge/src/lib.rs","crates/ontoforge/src/webhook_worker.rs","crates/ontoforge/tests/cli_webhook_worker.rs","crates/ontoforge/tests/snapshots/invalid_examples__v03-function-rule.snap","schemas/ontoforge-definition.schema.json"]
covered_digest: "v3:sha256:dea30394659522cab748fe4d88b7ff08c93ec664a30f683115b6d5b9969035ce"
re_verification:
  previous_status: gaps_found
  previous_score: 1/5
  gaps_closed:
    - "同一 RowKey 后续别名的 criteria、release 和 _version source 需求"
    - "pk sampler 的 host row budget 合同及重复 pk 两行采样边界"
    - "webhook worker 的生产 CLI 入口与持续投递"
    - "05-02 required artifact 修正为实际实现的 lib.rs"
  gaps_remaining: []
  regressions: []
  additional_checks:
    - "d4517e0 outbox 单条 claim、慢响应完成后计时与 batch limit：SQLite 11/11，embedded-pg 2/2"
human_verification:
  - test: "匹配当前提交的远端 CI/nightly 高风险测试实际运行"
    expected: "真实 PostgreSQL/Trino Action、outbox ignored targets 的发现数非零且执行通过。"
    why_human: "gh run list 对当前分支返回 []，本地执行不证明远端 CI 运行。"
  - test: "确认真实部署中的 worker 常驻及下游接收"
    expected: "服务管理器重启退出的 webhook-worker；真实下游收到带稳定 event_id 的通知。"
    why_human: "进程级 wiremock 测试证明实现路径，无法证明部署配置与真实下游。05-UAT.md 的总体确认仍待答复。"
---

# Phase 5: Online Action Verification

**Phase Goal:** As a caller with Action permission, I want to modify the object graph through scoped online Actions, so that each change is atomic, auditable, safe to retry, and notifies downstream systems.
**Status:** human_needed. 五条 ROADMAP 行为合同已有代码与测试证据；远端 CI 和实际部署仍待确认。
**Re-verification:** 上次 1/5、四项 gap，本次 5/5、无代码 gap。

## User Flow Coverage

| Step | Expected | Codebase evidence | Status |
| --- | --- | --- | --- |
| 授权调用 | 参数逐项报错，criteria 与 scope 遮蔽生效 | params/criteria/guard 及新增后别名 source criteria 测试通过 | VERIFIED |
| 原子写入 | PG 同事务提交 edit、log、command、outbox，失败回滚 | commit_action_guarded_inner、独立运行的 PG 故障矩阵 | VERIFIED |
| 读取与重试 | Query 可见新值，CAS、令牌、command_id 正确 | PG race、幂等及新增后 item 令牌测试通过 | VERIFIED |
| 通知下游 | 运行中的 worker 投递 event_id，失败重试、重启恢复 | 独立 CLI 进程调用 poll_at；三个进程级测试通过 | VERIFIED (部署待确认) |

## Observable Truths

| # | ROADMAP Success Criterion | Status | Evidence |
| --- | --- | --- | --- |
| 1 | 参数错误、PG 原子批次、validate-only | VERIFIED | prepare_edits、commit_action_guarded_inner；先前独立 PG action_atomic_after_outbox_failure 通过。 |
| 2 | 写列、criteria、post-state、link/release scope | VERIFIED | source::prepare 对每别名算 required/release 后仅 guard read 去重；later_alias_criterion_samples_shared_source_key、later_alias_release_samples_shared_source_key 均通过。 |
| 3 | 同 pk CAS、过期令牌、command_id 重放 | VERIFIED | 先前 PG race/幂等目标通过；later_item_token_samples_shared_source_key 通过。 |
| 4 | Trino 按需，已编辑目标可离线 | VERIFIED | source::prepare 仅在缺 source 等条件成立时采样；先前真实 Trino duplicate/offline 两项通过；本轮别名测试证实后续需求仍触发一次采样。 |
| 5 | 提交后下一次读取可见，webhook 至少一次 | VERIFIED | 先前 Action 后 Query overlay 验证；CLI worker 三测试确认 HTTP event_id、503 retry、重启恢复。d4517e0 的 SQLite outbox 11/11、embedded-pg outbox 2/2 独立通过，覆盖慢响应后的重试时间、逐条 claim 与 lease fencing。 |

**Score:** 5/5 truths verified；behavior_unverified: 0。

### Advisory (New Scope, Unevidenced)

None. 未发现新范围的无证据 blocker。

## Required Artifacts And Key Links

| Artifact/link | Status | Evidence |
| --- | --- | --- |
| source.rs -> read_pk_source -> Engine | WIRED | 逐别名检查 source demand；snapshots 按 RowKey 缓存实际 Engine 返回；seen 只控制 guard read。 |
| QueryService -> read_pk_source | WIRED | 传入 max_raw_candidates.min(3)；pk_read_obeys_host_raw_candidate_budget 在 budget=1 时通过。 |
| ActionService -> storage commit | WIRED | 锁内 guard、edit/log/command/outbox 同事务；真实 PG 回滚目标已通过。 |
| lib.rs -> webhook_worker.rs -> WebhookWorker | WIRED | CLI 分派、可信 [webhook] 配置与同一 EditStore；循环 poll_at，失败继续、Ctrl+C 退出。 |
| Worker -> storage -> HTTP | FLOWING | 每次短事务 claim 一条，due cutoff 固定；lease 随处理进度计算，HTTP 完成后 mark/retry 计时，owner token 约束更新。CLI wiremock 观察到 HTTP 与 event_id；SQLite/PG 目标通过。 |
| 05-02 required artifact | VERIFIED | 05-02 PLAN frontmatter 改为实际实现的 ontoforge-action/src/lib.rs，verify.artifacts 4/4，verify.key-links 3/3。正文两处 execute.rs 为文档残留，不构成功能 gap。 |

read_pk_source 的 SQL 固定 LIMIT 2，Action 专用 QueryContext.max_rows=3 容许观测第二行以拒绝重复 pk。TrinoConfig.max_rows 是配置生成普通 QueryContext 的值，当前 ActionService 并未接收该配置；旧报告将它直接视为 Action 采样合同的依据不足。QueryService 的 host row budget=1 有本轮行为测试。若将 TrinoConfig.max_rows 明确规定为全部内部采样的全局上限，必须先解决 max_rows=1 与重复 pk 检测的合同冲突。

## Requirements Coverage

| IDs | Status | Evidence or limit |
| --- | --- | --- |
| ACTION-01..06 | SATISFIED | 参数、规则、批次、validate-only 与 PG 原子性测试。 |
| ACTION-07 | SATISFIED within repository | production 写入入口扫描仅允许 Action 与 audited import；无法约束仓库外自建 host。 |
| ACTION-08..09 | SATISFIED | scope/link/release 与令牌测试，新增同 RowKey 后别名三目标通过。 |
| ACTION-10 | SATISFIED | SQLite/PG 幂等、失权和响应丢失重试目标已运行。 |
| ACTION-11 | SATISFIED in code | CLI 常驻 worker 发送、重试、重启恢复三目标通过；真实部署前提见 WEBHOOK-WORKER.md。 |
| ACTION-12 | SATISFIED | 真实 Trino duplicate/offline 目标先前通过；后别名按需采样本轮通过。 |

## Behavioral Spot Checks

| Command | Result |
| --- | --- |
| cargo test -p ontoforge-action --test source later_ --locked -j 2 | 3 passed, 0 failed |
| cargo test -p ontoforge-query --test pk_read pk_read_obeys_host_raw_candidate_budget --locked -j 2 -- --exact | 1 passed, 0 failed |
| cargo test -p ontoforge --test cli_webhook_worker --locked -j 2 | 3 passed, 0 failed |
| cargo test -p ontoforge-action --test outbox --locked -j 2 | 11 passed, 0 failed；含新增慢 503、第二条未提前 claim、batch limit 三项 |
| cargo test -p ontoforge-action --features embedded-pg --test outbox_pg --locked -j 2 -- --ignored | 2 passed, 0 failed；真实本地 PostgreSQL |
| 先前八条目标，包括真实 PG/Trino 故障、竞态、duplicate/offline | 各 1 passed；详见前次复验记录，本轮未重复运行。 |

## Anti-Patterns And Gates

d4517e0 的失败 503 在 HTTP 完成后起算退避；同轮 due cutoff 固定，防止立即重试；逐条 claim 避免后续事件 lease 在前一个慢请求期间耗尽。无环境变量的 PG 命令只触发测试前置检查，改用 embedded-pg 后 2/2 通过。相关实现文件未发现未关联 TBD/FIXME/XXX。ci.yml/nightly.yml 含 ignored test 非零发现及真实 PG/Trino 运行命令；gh run list --branch gsd/phase-05-online-action --limit 5 返回 []，没有匹配当前提交的远端运行证据。正式 gstack gate 与 05-UAT.md 总体确认尚无通过证据，不计入通过。

## Human Verification Required

1. 在当前 commit 的远端 CI/nightly 检查真实 PostgreSQL/Trino、outbox ignored targets 的发现数非零且执行成功；本地目标测试不能证明远端 workflow 成功。
2. 在真实部署确认服务管理器维持 webhook-worker，进程重启后真实下游收到稳定 event_id；wiremock 仅证明本地执行路径。05-UAT.md 的总确认项仍待答复。

## Gaps Summary

四项旧 gap 已关闭或重新判定合同范围。无可复现的代码 blocker；尚需完成上述人工/外部 gate 才能将 Phase 5 标记 passed。

---

_Verified: 2026-10-23T17:17:15Z_
_Verifier: gsd-verifier; no commit._
