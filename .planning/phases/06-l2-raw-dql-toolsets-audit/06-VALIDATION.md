---
phase: "06"
slug: "l2-raw-dql-toolsets-audit"
status: draft
nyquist_compliant: false
wave_0_complete: true
created: "2026-10-26"
---

# Phase 6 Validation Strategy

本文件映射 11 个 PLAN 的 21 个任务到自动化测试。原覆盖证据保留；正式 `06-PR-REVIEW.md` 新增 shutdown 并发/取消重试 CR-01，`06-11` 行为测试与实际运行证据待补，覆盖状态回 draft。服务生命周期及 HTTP `/health` 由 Phase 7 验收（ADR-0028）。本文件仅签收自动化覆盖，TDD 门禁、Phase 6 目标验收与远端 CI 状态单独裁定。

## Test Infrastructure

| Property | Value |
| --- | --- |
| Framework | Rust `cargo test`，已有 SQLite conformance、PostgreSQL ignored target、Trino integration target |
| Config file | `Cargo.toml`、`.github/workflows/ci.yml`、`ci/query-services.sh` |
| Quick run command | 各 PLAN 任务的目标 `cargo test -p <crate> --locked <filter>` |
| Full suite command | `cargo test --workspace --locked`，另执行真实 PostgreSQL 和受限 Trino 483 ignored targets |
| Measured runtime | 最终 Windows workspace 编译/链接 29m55s，测试另需约 3 分钟；真实 Trino 6 项 8.60s；旧基线详 `06-08-SUMMARY.md` |

## Sampling Rate

- 每个任务提交后运行其 `<automated>` 命令，核心逻辑先保留 RED 失败证据，再运行 GREEN。
- 每个 wave 后运行受影响 crate 的完整测试；涉及外部服务时运行对应 ignored target，确认发现数大于 0。
- 原代码 SHA `ea695ed480d56744835bf69d505e5531eef61314` 的 workspace 全量测试和两种真实服务门禁详 `06-08-SUMMARY.md`。后续补测及当前 `106a5aa` 的完整 workspace 本地结果见下方复验记录；远端 CI 匹配 SHA 证据仍待对账。
- 任务级测试在各 PLAN 的 SUMMARY 中记录，下面只保留索引和结果，不复制测试日志。

## Per-Task Verification Map

| Task | Requirement | Automated target | Local evidence |
| --- | --- | --- | --- |
| `06-11` T1 | AUDIT-01 | 共享 `conformance/mod.rs#audit_shutdown_`、storage worker unit | Pending：并发关闭、取消重试、成功/失败缓存与 JoinError；详 `06-11-PLAN.md` |
| `06-01` T1 | AUDIT-01 | `ontoforge-query/tests/audit_pk.rs#audit_pk_` | 4 passed；`06-01-SUMMARY.md` |
| `06-01` T2 | AUDIT-01 | `ontoforge-query/tests/audit_pk.rs#audit_worker_`、`ontoforge-storage/tests/conformance/mod.rs#audit_shutdown_drains_inflight_guard` | 原 5 passed；新增双后端在途 shutdown 测试通过，CR-01 closed |
| `06-02` T1 | AUDIT-01 | `ontoforge-storage/tests/conformance_{sqlite,pg}.rs#audit_` | SQLite 5/5、真实 PG 6/6（含新增在途 shutdown）；原基线见 `06-02-SUMMARY.md` |
| `06-02` T2 | AUDIT-01 | `ontoforge-storage/tests/conformance/mod.rs#audit_`、`ontoforge-storage/tests/conformance_pg.rs#audit_` | 双后端故障及在途 shutdown 测试通过；`0d3d0bf`、`5fd876d` |
| `06-03` T1 | L2-02, L2-03 | `ontoforge-engine/tests/l2_gate.rs#l2_gate_` | 10/10；`06-03-SUMMARY.md` |
| `06-03` T2 | L2-02, L2-03 | `ontoforge-engine/tests/l2_gate.rs#l2_corpus_` | 104-ID 等价语料 6/6；`06-03-SUMMARY.md` |
| `06-04` T1 | L2-04 | `ontoforge-engine/tests/l2_health.rs#l2_health_` | 库级自检通过；startup/periodic 库级缺口已由 `06-10` 补齐，服务接线归 Phase 7 |
| `06-04` T2 | L2-03, L2-04 | `ontoforge-engine/tests/trino_l2_acl_it.rs#trino_l2_acl_` | 真实受限 Trino 483 3/3；`06-04-SUMMARY.md` |
| `06-05` T1 | AUDIT-01 | `ontoforge-query/tests/audit_l0.rs#audit_list_` | list/set/searchAround 及 PG-only 通过；`06-05-SUMMARY.md` |
| `06-05` T2 | AUDIT-01 | `ontoforge-query/tests/audit_l0.rs#audit_aggregate_` | 聚合/故障目标通过；`06-05-SUMMARY.md` |
| `06-06` T1 | L2-01..04, AUDIT-01 | `ontoforge-query/tests/l2_execute.rs#l2_execute_` | execute 授权、审计、截断通过；`06-06-SUMMARY.md` |
| `06-06` T2 | L2-01..04, AUDIT-01 | `ontoforge-query/tests/l2_execute.rs#l2_explain_` | explain 与审计通过，target 总计 12/12；`06-06-SUMMARY.md` |
| `06-07` T1 | L2-01, L2-03, L2-04, AUDIT-01 | `ontoforge-query/tests/l2_discovery.rs#l2_catalog_` | catalog/schema 与故障测试通过；`06-07-SUMMARY.md` |
| `06-07` T2 | L2-01, L2-03, L2-04, AUDIT-01 | `ontoforge-query/tests/l2_discovery.rs#l2_table_` | table/schema 与故障测试通过，target 总计 9/9；`06-07-SUMMARY.md` |
| `06-08` T1 | L2-01..04 | `ontoforge-engine/tests/{trino_l2_acl_it,l2_gate}.rs` | ACL 3/3、分类 6/6；32 条 allow 经生产 gate 在受限 Trino 483 全部 FINISHED，WR-02 closed |
| `06-08` T2 | AUDIT-01 | `ontoforge-storage/tests/conformance_pg.rs#audit_`、workspace | `06-08-SUMMARY.md` 基线 PG 5/5、workspace 1231 passed；补测 PG 6/6，当前本地 workspace 退出 0 |
| `06-09` T1 | AUDIT-01, SCOPE-07 | `ontoforge/tests/cli_{reload,scope}.rs`、`ontoforge-storage/tests/scope_store.rs` | 8/8、8/8、12/12；`06-09-SUMMARY.md` |
| `06-09` T2 | L2-01, SCOPE-07 | `ontoforge-query/tests/definition_snapshot.rs` | L0/L2 并发快照 2/2；`06-09-SUMMARY.md` |
| `06-10` T1 | L2-03, L2-04 | `ontoforge-engine/tests/{l2_health,l2_monitor}.rs` | lib 55/55、health 10/10、monitor 16/16 GREEN；5 组有效 RED，详 `06-10-SUMMARY.md` |
| `06-10` T2 | L2-03, L2-04 | `ontoforge-engine/tests/{l2_monitor,trino_l2_acl_it}.rs` | 真实受限/松 ACL 默认并行 6/6、32 条 allow 全部 FINISHED；最终 workspace 1251 passed/103 ignored/0 failed，Clippy/fmt 通过 |

## Wave 0 Requirements

- [x] 104 个历史 ID 映射为仓库内机器可执行的 `constructed-equivalent` fixture；原始 SQL 恢复 0 条，历史偏差见 `06-03-SUMMARY.md`。
- [x] `crates/ontoforge-query/tests/audit_pk.rs` 承载 Wave 1 RED/GREEN 测试。
- [x] 独立受限 Trino 483 ACL fixture 与 `trino_l2_acl_it` target 已本地运行，CI 已接线。
- [x] 审计 PG/SQLite migration、conformance 和故障注入测试已运行。
- [x] 20 个任务各有自动化命令；原 18 个任务结果详各 SUMMARY，`06-10` 目标与真实测试结果见 `06-10-SUMMARY.md`。

## Manual-Only Verifications

无计划中的纯人工验收。生产环境 webhook 和 REST/MCP 外壳属于 Phase 7；Phase 6 的 Trino ACL 必须自动化实测。

## Gap Analysis

| Review | Requirement | Status | Missing automated proof |
| --- | --- | --- | --- |
| CR-01 | AUDIT-01 | CLOSED | `0d3d0bf` 新增 `conformance/mod.rs#audit_shutdown_drains_inflight_guard`；`5fd876d` 分离 admission 与 sink 健康。SQLite 5/5、真实 PG 6/6，均覆盖在途 guard 与并发 shutdown 后持久行可见；sink 失败仍 fail-closed。 |
| WR-01 | L2-04 | CLOSED (library) | `06-10-SUMMARY.md` 的生命周期、真实活跃性和清理测试通过，独立增量复审 `06-10-REVIEW.md` clean。Phase 7 服务与 `/health` 仍按 ADR-0028 阻断验收，L2-04 整体 Pending。 |
| WR-02 | L2-02, L2-03 | CLOSED | `d8ed219`、`bc43cbe` 使 32 条 allow constructed-equivalent 均经 `validate_l2` 的 Display SQL 发送至受限 Trino 483，32/32 FINISHED 并核对行数；104-ID 分类 6/6，拒绝仍为零 POST。原文恢复仍为 0 条。 |

原三项发现均已关闭，详 `06-REVIEW-DISPOSITION.md`；增量五项发现也已关闭，详 `06-10-REVIEW.md`。Phase 6 总验收不由审查计数代替。

## Validation Sign-Off

- [x] 20/20 任务有 `<automated>` 命令且通过；`06-10` 目标 GREEN 和真实 ACL 已通过，最终 workspace/Clippy/fmt 通过，Phase 7 接线独立验收。
- [x] 每任务有自动化反馈，没有三个连续任务无验证。
- [x] Wave 0 fixture 和 target 已就位。
- [x] 命令无 watch mode；本地真实 Trino ACL 3/3、PG audit 6/6，发现数均大于 0。
- [x] 反馈耗时见各 SUMMARY；静态覆盖增量审计无新 BLOCKER，最终测试及 Clippy/fmt 通过。

**Approval:** 库级自动化覆盖通过。最终 `e81127f` 的 workspace 为 1251 passed、103 ignored、0 failed，Clippy/fmt 通过；TDD 门禁漏读诊断、远端 CI 与 Phase 6 目标验收独立对账。此签收不代表 L2-04 整体完成。

## 06-08 实施后证据（2026-10-28）

本节为旧代码 SHA `ea695ed480d56744835bf69d505e5531eef61314` 的历史基线；远端 GitHub Actions 尚未触发。完整命令、偏差与失败历史详见 `06-08-SUMMARY.md`。

| 门禁 | 发现数 | 本地实际结果 |
| --- | ---: | --- |
| 独立受限/宽松对照 Trino 483 `trino_l2_acl_it --ignored` | 3 | 3 passed，约 3 秒；G16/G17 Display 通过受限身份执行，G11 被拒绝 |
| 104-ID corpus `l2_corpus_` | 6 | 6 passed；104 个 constructed-equivalent ID 分类覆盖 |
| PostgreSQL `conformance_pg audit_ --features embedded-pg --ignored` | 5 | 5 passed，约 10 秒 |
| SQLite `conformance_sqlite audit_` | 4 | 4 passed |
| query `audit_pk`、`audit_l0` | 9、10 | 9、9 passed；后者另 1 项需外部 PG，普通运行 ignored |
| engine `l2_gate` | 16 | 16 passed |
| Windows workspace `--test-threads=1` | 116 个结果组 | 1231 passed，98 ignored，0 failed |

`cargo fmt --all --check`、`cargo clippy --workspace --all-targets --locked -- -D warnings`、`git diff --check` 在上述旧 SHA 通过。默认并行 workspace 的两次失败和稳定方案详见 SUMMARY；单线程方式仍有慢机器墙钟风险，见 `deferred-items.md`。

## 2026-10-28 补测

- 当前代码提交 `106a5aa`：`cargo test --workspace --locked -j 2 -- --test-threads=1` 退出 0，`cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings`、`cargo fmt --all -- --check`、`git diff --check` 退出 0。首轮完整回归曾因 reload 测试把一次 stale 拒绝算进成功次数而失败；共用 helper 修正后目标连续 6 次通过，完整 workspace 通过。此证据仅覆盖本地，未产生远端 CI 结果。
- `06-09` test-only PLAN 分类已显式更正为 `execute`，保留 task 级 TDD RED mutation 与 GREEN 测试；`node C:\Users\easyi\.codex\gsd-core\bin\gsd-tools.cjs check tdd-review-checkpoint 06` 返回 `block: false`、7/7 TDD PLAN 通过。WR-01 仍开放，不能签署 validation。

- `0d3d0bf`/`5fd876d`：新增双后端共用 `audit_shutdown_drains_inflight_guard` 与 `audit_sink_failure_rejects_inflight_finish`；主会话实测 SQLite audit 5/5、真实 PG audit 6/6。`ReadAudit::shutdown()` 停止新准入，但已准入 guard 可在 sink 健康时完成入队与 drain。
- `d8ed219`/`bc43cbe`：受限 Trino 483 ACL 3/3、104-ID corpus 分类 6/6，32 条 allow constructed-equivalent 均经生产 gate 实际 FINISHED。G38/B51 初次真实执行失败后修正等价 fixture，未改生产 AST gate。拒绝路径零 POST 仍由 `l2_gate` 覆盖。
- 上述真实服务补测与当前完整 workspace 本地结果各自记录；仍无远端 GitHub Actions 运行证据，WR-01 未关闭。

## Validation Audit 2026-10-27

| Metric | Count |
|---|---|
| Resolved | 0 |
| Escalated | 3 |
| Gaps found | 3 |

## Validation Audit 2026-10-27

| Metric | Count |
|---|---|
| Resolved | 2 |
| Gaps found | 1 |
| Escalated | 1 |

## Validation Audit 2026-10-28

| Metric | Count |
|---|---|
| Gaps found | 0 |
| Resolved | 3 |
| Escalated | 0 |
