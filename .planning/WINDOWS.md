---
schema_version: 1
open_count: 15
waived_count: 0
fixed_count: 7
total_count: 22
last_updated: 2026-11-09T14:30:00Z
---

# Broken Windows Ledger

> Cross-phase defect register. With `workflow.windows_enforce` enabled, `/gsd-ship` blocks while `open_count > 0`.
> Waive with `gsd-tools windows waive <id> "<reason>"` (reason required).
> Mark fixed with `gsd-tools windows fixed <id>`.

| id | phase | kind | file | line | description | status | reason | recorded_at | resolved_at |
|----|-------|------|------|------|-------------|--------|--------|-------------|-------------|
| 1 | 2 | stub | crates/ontoforge-engine/src/sqlite/mod.rs |  | SqliteEngine.describe_tables answers from an in-memory metadata map that nothing populates until plan 02-05 adds fixture loading (intentional) | fixed |  | 2026-10-16T19:03:31.155Z | 2026-10-16T20:41:31.074Z |
| 2 | 2 | deviation | crates/ontoforge-engine/src/printer/sqlite_ddl.rs |  | sqlite_ddl carries allow(dead_code) until plan 02-05 uses attach_memory/create_table/insert/query_only outside tests | fixed |  | 2026-10-16T19:03:31.601Z | 2026-10-16T20:41:31.547Z |
| 3 | 2 | deviation | crates/ontoforge/src/online.rs |  | ONT-V20 help text of validate --online names --server-config, a flag plan 02-09 adds; until then the flag does not exist | open |  | 2026-10-16T20:41:53.199Z |  |
| 4 | 02 | deviation | docs/source-binding.md |  | SQLite stand-in reports float properties as double, so fixture-mode --online flags float properties as ONT-V17 (float reads only real); fix is in sqlite/fixture.rs physical_of, outside 02-08 scope | fixed |  | 2026-10-16T21:49:10.933Z | 2026-10-16T21:55:10.377Z |
| 5 | 02 | unrun-verify | .github/workflows/ci.yml |  | integration job (PostgreSQL + Trino 483, trino_it / cli_trino_it --ignored) not yet run on GitHub; describe_types_golden and decode_golden are PROVISIONAL until the first run (E21) | open |  | 2026-10-17T01:06:08.093Z |  |
| 6 | 02 | unrun-verify | .github/workflows/nightly.yml |  | nightly Iceberg workflow (Trino 483/484, local-warehouse JDBC catalog, report-failure issue job) not yet run; needs a workflow_dispatch run (E21) | open |  | 2026-10-17T01:06:08.569Z |  |
| 7 | 05 | stub | crates/ontoforge-action/src/lib.rs | 186 | validate-only is explicitly refused by the edit-only tracer; planned in 05-02 | open |  | 2026-10-23T07:09:41.788Z |  |
| 8 | 05 | stub | crates/ontoforge-action/src/version.rs | 47 | source-backed token 在 05-03 无 source sampler，统一 VersionConflict；05-06 接入真实 source snapshot 与往返验收 | fixed |  | 2026-10-23T10:01:46.213Z | 2026-10-23T13:36:20.358Z |
| 9 | 05 | deviation | crates/ontoforge-action/tests/trino_action_it.rs |  | Task 2 真实集成断言首次有效运行即通过，缺少独立行为 RED。 | open |  | 2026-10-23T13:34:50.748Z |  |
| 10 | 05 | deviation | .planning/phases/05-online-action/deferred-items.md |  | 默认 feature 组合的 cargo check/clippy 因现有配置问题失败，已用 embedded-pg 验证计划代码。 | open |  | 2026-10-23T13:34:51.389Z |  |
| 11 | 06 | stub | crates/ontoforge-storage/src/read_audit.rs | 166 | PostgreSQL worker explicitly fails closed until 06-02 provides its system table and sink | open |  | 2026-10-26T06:51:40.852Z |  |
| 12 | 06 | skipped-test | crates/ontoforge-storage/tests/scope_store.rs |  | pg_audit_is_append_only_and_login_is_recorded: requires ONTOFORGE_PG_URL or embedded-pg feature | open |  | 2026-10-27T19:13:33.710Z |  |
| 13 | 06 | skipped-test | crates/ontoforge-storage/tests/scope_store.rs |  | pg_concurrent_scope_apply_serializes: requires ONTOFORGE_PG_URL or embedded-pg feature | open |  | 2026-10-27T19:13:44.675Z |  |
| 14 | 06 | skipped-test | crates/ontoforge-storage/tests/scope_store.rs |  | pg_scope_apply: requires ONTOFORGE_PG_URL or embedded-pg feature | open |  | 2026-10-27T19:13:52.718Z |  |
| 15 | 07 | skipped-test | .planning/phases/07-rest-mcp-shell/07-03-SUMMARY.md |  | 07-03 workspace: 107 ignored (PG/Trino/private/p95/doctest); external-service evidence pending 07-13/14. Full names and reasons: 07-03-evidence/workspace-tests-jobs2.log. | open |  | 2026-10-29T04:32:00.805Z |  |
| 16 | 07 | deviation | crates/ontoforge-action/tests/idempotency_pg.rs |  | 07-03 all-features Clippy: large_futures at lines 242/257/398 (16464 bytes); approved minimal Box::pin allocation fix in 07-02 PLAN if still present after replay repair. | fixed |  | 2026-10-29T04:32:02.063Z | 2026-10-29T04:49:36.824Z |
| 17 | 02 | unrun-verify | .planning/phases/02-source-binding-engine/02-11-HUMAN-ACCEPTANCE.md |  | G-02-B / G-02-C / G-02-D 三项 judgment 的人工验收尚未进行；02-11 只交付机器可判定部分与问题清单，三条 VERDICT 仍为 PENDING，待下一轮 /gsd-verify-work 02 收集 | open |  | 2026-11-08T05:31:00.586Z |  |
| 18 | 02 | unmet-truth | crates/ontoforge/tests/cli_check_cardinality.rs | 437 | 全工作区并行负载下 cardinality_budget_starts_after_precheck_and_pk 与 cardinality_total_deadline_is_shared 两个 300 ms deadline 用例偶发超时失败；隔离重跑 13 passed / 0 failed，全量 --no-fail-fast 重跑 1528 passed / 0 failed，判定为时序 flake 而非回归 | fixed | Phase 10 已把两个 deadline E2E 用例预算放宽到秒级（timeout 5000/3000ms，慢查延迟 60s，四 mock 延迟 1500ms），语义不变；security.rs 混用测试拆分为确定性用例并新增 BEGIN EXCLUSIVE 锁触发的 pending-age 期限测试；ci.yml:59 已移除 --test-threads=1。实跑：cargo test -p ontoforge --test cli_check_cardinality → 13 passed；cargo test --workspace --locked（默认并行度）→ 全部通过（exit 0），见 gsd/phase-10-audit-timing-split | 2026-11-08T08:47:21.377Z | 2026-11-09T14:30:00Z |
| 19 | 08 | unrun-verify | crates/ontoforge-action/src/outbox.rs | 157 | Phase 08 原端到端用例仅覆盖第一次失败后的重试。FA-02 的计算退避实际从第 10 次已持久化失败起达到 300 秒上限，不是到第 17 次才封顶；outbox schema 不存在 last_attempt_at 列。2026-11-09 已补充 attempts 1..20 的 SQLite 运行时验证，含 attempts 17..20。既有 CI job（ci.yml:73）自 Phase 05 起已每次运行该用例至 attempt 10 | fixed | 独立复核通过原测试的只读内存派生版本，将循环上限及请求数断言均扩展到 20，调用原生产 worker/store 完成同一事件连续 20 次 503。逐次计数、请求总数、退避区间及未到期不领取断言全部通过，1 passed / 0 failed / 0.32s。随后循环上限与请求数断言已固化进 tests/outbox.rs（1..=20），本机实测 cargo test -p ontoforge-action --locked --test outbox → 11 passed / 0 failed / 0.97s。证据限于 SQLite 和注入的 poll 时间，不包含 PostgreSQL 高次重试、生产墙钟调度、多 worker lease 竞争或溢出边界 | 2026-11-08T19:33:14.558Z | 2026-11-09T13:38:03.090Z |
| 20 | 08 | unrun-verify | .planning/phases/08-production-webhook-end-to-end-delivery/08-evidence/ci-evidence-index.json |  | Phase 08 CI 证据归档的来源需人工交叉复核：本地已机器判定 run 42515808955 含结论为 success 的 phase8-webhook-e2e job 且 head_sha 30a05a8 是当前 HEAD 的祖先、晚于首次引入该 job 的 commit f95767a，但归档内容是否真出自该 run 只能由 CI run 的 URL 与 run id 人工确认（FA-03 backstop） | open |  | 2026-11-08T20:17:44.865Z |  |
| 21 | 08 | deviation | crates/ontoforge-action/src/outbox.rs | 159 | WebhookWorker 调用 store.record_retry(...)（commit.rs:306-313 返回 Result<bool, StoreError>）只用 ? 传播 Err，丢弃 Ok(false)（fencing 拒绝：旧 lease token 已被抢占）。fenced worker 会把事件当已处理；at-least-once 语义下允许重复投递故语义无害，但计数与接收端请求序号可能不对应 | open |  | 2026-11-09T13:38:03.090Z |  |
| 22 | 08 | deviation | crates/ontoforge-storage/migrations/postgres/0004_action.sql | 23 | PostgreSQL 的 attempt_count 为 integer(i32)，比 Rust 的 u32 更早触顶：计数达 i32::MAX 后下一次失败的 checked_add(u32) 仍成功，但 record_retry 写入 2^31 超出列范围致事务失败，该行成为 poisoned row（每次 claim→send→record_retry 写失败→poll_at 返回 Err）；claim 侧 commit.rs:249-251 的 i32→u32 try_into 失败亦返回 StoreError::Corrupt。实际不可达（需约 21 亿次失败），属 schema 与 Rust 类型宽度不一致；SQLite 侧 INTEGER(i64) 可走到 u32::MAX | open |  | 2026-11-09T13:38:03.090Z |  |

````json
[
  {
    "id": 1,
    "kind": "stub",
    "phase": "2",
    "file": "crates/ontoforge-engine/src/sqlite/mod.rs",
    "line": null,
    "description": "SqliteEngine.describe_tables answers from an in-memory metadata map that nothing populates until plan 02-05 adds fixture loading (intentional)",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-10-16T19:03:31.155Z",
    "resolved_at": "2026-10-16T20:41:31.074Z"
  },
  {
    "id": 2,
    "kind": "deviation",
    "phase": "2",
    "file": "crates/ontoforge-engine/src/printer/sqlite_ddl.rs",
    "line": null,
    "description": "sqlite_ddl carries allow(dead_code) until plan 02-05 uses attach_memory/create_table/insert/query_only outside tests",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-10-16T19:03:31.601Z",
    "resolved_at": "2026-10-16T20:41:31.547Z"
  },
  {
    "id": 3,
    "kind": "deviation",
    "phase": "2",
    "file": "crates/ontoforge/src/online.rs",
    "line": null,
    "description": "ONT-V20 help text of validate --online names --server-config, a flag plan 02-09 adds; until then the flag does not exist",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-10-16T20:41:53.199Z",
    "resolved_at": null
  },
  {
    "id": 4,
    "kind": "deviation",
    "phase": "02",
    "file": "docs/source-binding.md",
    "line": null,
    "description": "SQLite stand-in reports float properties as double, so fixture-mode --online flags float properties as ONT-V17 (float reads only real); fix is in sqlite/fixture.rs physical_of, outside 02-08 scope",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-10-16T21:49:10.933Z",
    "resolved_at": "2026-10-16T21:55:10.377Z"
  },
  {
    "id": 5,
    "kind": "unrun-verify",
    "phase": "02",
    "file": ".github/workflows/ci.yml",
    "line": null,
    "description": "integration job (PostgreSQL + Trino 483, trino_it / cli_trino_it --ignored) not yet run on GitHub; describe_types_golden and decode_golden are PROVISIONAL until the first run (E21)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-10-17T01:06:08.093Z",
    "resolved_at": null
  },
  {
    "id": 6,
    "kind": "unrun-verify",
    "phase": "02",
    "file": ".github/workflows/nightly.yml",
    "line": null,
    "description": "nightly Iceberg workflow (Trino 483/484, local-warehouse JDBC catalog, report-failure issue job) not yet run; needs a workflow_dispatch run (E21)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-10-17T01:06:08.569Z",
    "resolved_at": null
  },
  {
    "id": 7,
    "kind": "stub",
    "phase": "05",
    "file": "crates/ontoforge-action/src/lib.rs",
    "line": 186,
    "description": "validate-only is explicitly refused by the edit-only tracer; planned in 05-02",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-10-23T07:09:41.788Z",
    "resolved_at": null,
    "milestone": null
  },
  {
    "id": 8,
    "kind": "stub",
    "phase": "05",
    "file": "crates/ontoforge-action/src/version.rs",
    "line": 47,
    "description": "source-backed token 在 05-03 无 source sampler，统一 VersionConflict；05-06 接入真实 source snapshot 与往返验收",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-10-23T10:01:46.213Z",
    "resolved_at": "2026-10-23T13:36:20.358Z",
    "milestone": null
  },
  {
    "id": 9,
    "kind": "deviation",
    "phase": "05",
    "file": "crates/ontoforge-action/tests/trino_action_it.rs",
    "line": null,
    "description": "Task 2 真实集成断言首次有效运行即通过，缺少独立行为 RED。",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-10-23T13:34:50.748Z",
    "resolved_at": null,
    "milestone": null
  },
  {
    "id": 10,
    "kind": "deviation",
    "phase": "05",
    "file": ".planning/phases/05-online-action/deferred-items.md",
    "line": null,
    "description": "默认 feature 组合的 cargo check/clippy 因现有配置问题失败，已用 embedded-pg 验证计划代码。",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-10-23T13:34:51.389Z",
    "resolved_at": null,
    "milestone": null
  },
  {
    "id": 11,
    "kind": "stub",
    "phase": "06",
    "file": "crates/ontoforge-storage/src/read_audit.rs",
    "line": 166,
    "description": "PostgreSQL worker explicitly fails closed until 06-02 provides its system table and sink",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-10-26T06:51:40.852Z",
    "resolved_at": null,
    "milestone": null
  },
  {
    "id": 12,
    "kind": "skipped-test",
    "phase": "06",
    "file": "crates/ontoforge-storage/tests/scope_store.rs",
    "line": null,
    "description": "pg_audit_is_append_only_and_login_is_recorded: requires ONTOFORGE_PG_URL or embedded-pg feature",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-10-27T19:13:33.710Z",
    "resolved_at": null,
    "milestone": null
  },
  {
    "id": 13,
    "kind": "skipped-test",
    "phase": "06",
    "file": "crates/ontoforge-storage/tests/scope_store.rs",
    "line": null,
    "description": "pg_concurrent_scope_apply_serializes: requires ONTOFORGE_PG_URL or embedded-pg feature",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-10-27T19:13:44.675Z",
    "resolved_at": null,
    "milestone": null
  },
  {
    "id": 14,
    "kind": "skipped-test",
    "phase": "06",
    "file": "crates/ontoforge-storage/tests/scope_store.rs",
    "line": null,
    "description": "pg_scope_apply: requires ONTOFORGE_PG_URL or embedded-pg feature",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-10-27T19:13:52.718Z",
    "resolved_at": null,
    "milestone": null
  },
  {
    "id": 15,
    "kind": "skipped-test",
    "phase": "07",
    "file": ".planning/phases/07-rest-mcp-shell/07-03-SUMMARY.md",
    "line": null,
    "description": "07-03 workspace: 107 ignored (PG/Trino/private/p95/doctest); external-service evidence pending 07-13/14. Full names and reasons: 07-03-evidence/workspace-tests-jobs2.log.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-10-29T04:32:00.805Z",
    "resolved_at": null,
    "milestone": null
  },
  {
    "id": 16,
    "kind": "deviation",
    "phase": "07",
    "file": "crates/ontoforge-action/tests/idempotency_pg.rs",
    "line": null,
    "description": "07-03 all-features Clippy: large_futures at lines 242/257/398 (16464 bytes); approved minimal Box::pin allocation fix in 07-02 PLAN if still present after replay repair.",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-10-29T04:32:02.063Z",
    "resolved_at": "2026-10-29T04:49:36.824Z",
    "milestone": null
  },
  {
    "id": 17,
    "kind": "unrun-verify",
    "phase": "02",
    "file": ".planning/phases/02-source-binding-engine/02-11-HUMAN-ACCEPTANCE.md",
    "line": null,
    "description": "G-02-B / G-02-C / G-02-D 三项 judgment 的人工验收尚未进行；02-11 只交付机器可判定部分与问题清单，三条 VERDICT 仍为 PENDING，待下一轮 /gsd-verify-work 02 收集",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-08T05:31:00.586Z",
    "resolved_at": null,
    "milestone": "v1.0"
  },
  {
    "id": 18,
    "kind": "unmet-truth",
    "phase": "02",
    "file": "crates/ontoforge/tests/cli_check_cardinality.rs",
    "line": 437,
    "description": "全工作区并行负载下 cardinality_budget_starts_after_precheck_and_pk 与 cardinality_total_deadline_is_shared 两个 300 ms deadline 用例偶发超时失败；隔离重跑 13 passed / 0 failed，全量 --no-fail-fast 重跑 1528 passed / 0 failed，判定为时序 flake 而非回归",
    "status": "fixed",
    "reason": "Phase 10 已把两个 deadline E2E 用例预算放宽到秒级（timeout 5000/3000ms，慢查延迟 60s，四 mock 延迟 1500ms），语义不变；security.rs 混用测试拆分为确定性用例并新增 BEGIN EXCLUSIVE 锁触发的 pending-age 期限测试；ci.yml:59 已移除 --test-threads=1。实跑：cargo test -p ontoforge --test cli_check_cardinality → 13 passed；cargo test --workspace --locked（默认并行度）→ 全部通过（exit 0），见 gsd/phase-10-audit-timing-split",
    "recorded_at": "2026-11-08T08:47:21.377Z",
    "resolved_at": "2026-11-09T14:30:00Z",
    "milestone": "v1.0"
  },
  {
    "id": 19,
    "kind": "unrun-verify",
    "phase": "08",
    "file": "crates/ontoforge-action/src/outbox.rs",
    "line": 157,
    "description": "Phase 08 原端到端用例仅覆盖第一次失败后的重试。FA-02 的计算退避实际从第 10 次已持久化失败起达到 300 秒上限，不是到第 17 次才封顶；outbox schema 不存在 last_attempt_at 列。2026-11-09 已补充 attempts 1..20 的 SQLite 运行时验证，含 attempts 17..20。既有 CI job（ci.yml:73）自 Phase 05 起已每次运行该用例至 attempt 10",
    "status": "fixed",
    "reason": "独立复核通过原测试的只读内存派生版本，将循环上限及请求数断言均扩展到 20，调用原生产 worker/store 完成同一事件连续 20 次 503。逐次计数、请求总数、退避区间及未到期不领取断言全部通过，1 passed / 0 failed / 0.32s。随后循环上限与请求数断言已固化进 tests/outbox.rs（1..=20），本机实测 cargo test -p ontoforge-action --locked --test outbox → 11 passed / 0 failed / 0.97s。证据限于 SQLite 和注入的 poll 时间，不包含 PostgreSQL 高次重试、生产墙钟调度、多 worker lease 竞争或溢出边界",
    "recorded_at": "2026-11-08T19:33:14.558Z",
    "resolved_at": "2026-11-09T13:38:03.090Z",
    "milestone": "v1.1"
  },
  {
    "id": 20,
    "kind": "unrun-verify",
    "phase": "08",
    "file": ".planning/phases/08-production-webhook-end-to-end-delivery/08-evidence/ci-evidence-index.json",
    "line": null,
    "description": "Phase 08 CI 证据归档的来源需人工交叉复核：本地已机器判定 run 42515808955 含结论为 success 的 phase8-webhook-e2e job 且 head_sha 30a05a8 是当前 HEAD 的祖先、晚于首次引入该 job 的 commit f95767a，但归档内容是否真出自该 run 只能由 CI run 的 URL 与 run id 人工确认（FA-03 backstop）",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-08T20:17:44.865Z",
    "resolved_at": null,
    "milestone": "v1.1"
  },
  {
    "id": 21,
    "kind": "deviation",
    "phase": "08",
    "file": "crates/ontoforge-action/src/outbox.rs",
    "line": 159,
    "description": "WebhookWorker 调用 store.record_retry(...)（commit.rs:306-313 返回 Result<bool, StoreError>）只用 ? 传播 Err，丢弃 Ok(false)（fencing 拒绝：旧 lease token 已被抢占）。fenced worker 会把事件当已处理；at-least-once 语义下允许重复投递故语义无害，但计数与接收端请求序号可能不对应",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-09T13:38:03.090Z",
    "resolved_at": null,
    "milestone": "v1.1"
  },
  {
    "id": 22,
    "kind": "deviation",
    "phase": "08",
    "file": "crates/ontoforge-storage/migrations/postgres/0004_action.sql",
    "line": 23,
    "description": "PostgreSQL 的 attempt_count 为 integer(i32)，比 Rust 的 u32 更早触顶：计数达 i32::MAX 后下一次失败的 checked_add(u32) 仍成功，但 record_retry 写入 2^31 超出列范围致事务失败，该行成为 poisoned row（每次 claim→send→record_retry 写失败→poll_at 返回 Err）；claim 侧 commit.rs:249-251 的 i32→u32 try_into 失败亦返回 StoreError::Corrupt。实际不可达（需约 21 亿次失败），属 schema 与 Rust 类型宽度不一致；SQLite 侧 INTEGER(i64) 可走到 u32::MAX",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-09T13:38:03.090Z",
    "resolved_at": null,
    "milestone": "v1.1"
  }
]
````
