---
phase: "06"
slug: "l2-raw-dql-toolsets-audit"
status: draft
nyquist_compliant: false
wave_0_complete: true
created: "2026-10-26"
---

# Phase 6 Validation Strategy

本文件映射 9 个 PLAN 的 18 个任务到自动化测试。代码审查提出的 CR-01 和 WR-02 已补充测试与实测，WR-01 的生产接线仍缺；Phase 6 目标验收与远端 GitHub Actions 运行状态由 `06-VERIFICATION.md` 和匹配 SHA 的 CI 记录单独裁定。

## Test Infrastructure

| Property | Value |
| --- | --- |
| Framework | Rust `cargo test`，已有 SQLite conformance、PostgreSQL ignored target、Trino integration target |
| Config file | `Cargo.toml`、`.github/workflows/ci.yml`、`ci/query-services.sh` |
| Quick run command | 各 PLAN 任务的目标 `cargo test -p <crate> --locked <filter>` |
| Full suite command | `cargo test --workspace --locked`，另执行真实 PostgreSQL 和受限 Trino 483 ignored targets |
| Measured runtime | Windows workspace 单线程约 3 分钟；Trino ACL 约 3 秒，embedded PostgreSQL audit 约 10 秒（详 `06-08-SUMMARY.md`） |

## Sampling Rate

- 每个任务提交后运行其 `<automated>` 命令，核心逻辑先保留 RED 失败证据，再运行 GREEN。
- 每个 wave 后运行受影响 crate 的完整测试；涉及外部服务时运行对应 ignored target，确认发现数大于 0。
- 原代码 SHA `ea695ed480d56744835bf69d505e5531eef61314` 的 workspace 全量测试和两种真实服务门禁详 `06-08-SUMMARY.md`。后续补测及当前 `106a5aa` 的完整 workspace 本地结果见下方复验记录；远端 CI 匹配 SHA 证据仍待对账。
- 任务级测试在各 PLAN 的 SUMMARY 中记录，下面只保留索引和结果，不复制测试日志。

## Per-Task Verification Map

| Task | Requirement | Automated target | Local evidence |
| --- | --- | --- | --- |
| `06-01` T1 | AUDIT-01 | `ontoforge-query/tests/audit_pk.rs#audit_pk_` | 4 passed；`06-01-SUMMARY.md` |
| `06-01` T2 | AUDIT-01 | `ontoforge-query/tests/audit_pk.rs#audit_worker_`、`ontoforge-storage/tests/conformance/mod.rs#audit_shutdown_drains_inflight_guard` | 原 5 passed；新增双后端在途 shutdown 测试通过，CR-01 closed |
| `06-02` T1 | AUDIT-01 | `ontoforge-storage/tests/conformance_{sqlite,pg}.rs#audit_` | SQLite 5/5、真实 PG 6/6（含新增在途 shutdown）；原基线见 `06-02-SUMMARY.md` |
| `06-02` T2 | AUDIT-01 | `ontoforge-storage/tests/conformance/mod.rs#audit_`、`ontoforge-storage/tests/conformance_pg.rs#audit_` | 双后端故障及在途 shutdown 测试通过；`0d3d0bf`、`5fd876d` |
| `06-03` T1 | L2-02, L2-03 | `ontoforge-engine/tests/l2_gate.rs#l2_gate_` | 10/10；`06-03-SUMMARY.md` |
| `06-03` T2 | L2-02, L2-03 | `ontoforge-engine/tests/l2_gate.rs#l2_corpus_` | 104-ID 等价语料 6/6；`06-03-SUMMARY.md` |
| `06-04` T1 | L2-04 | `ontoforge-engine/tests/l2_health.rs#l2_health_` | 库级自检通过；生产启动/周期接线缺失，详 WR-01 |
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

## Wave 0 Requirements

- [x] 104 个历史 ID 映射为仓库内机器可执行的 `constructed-equivalent` fixture；原始 SQL 恢复 0 条，历史偏差见 `06-03-SUMMARY.md`。
- [x] `crates/ontoforge-query/tests/audit_pk.rs` 承载 Wave 1 RED/GREEN 测试。
- [x] 独立受限 Trino 483 ACL fixture 与 `trino_l2_acl_it` target 已本地运行，CI 已接线。
- [x] 审计 PG/SQLite migration、conformance 和故障注入测试已运行。
- [x] 18 个任务各有自动化命令；发现数和运行结果详各 SUMMARY。

## Manual-Only Verifications

无计划中的纯人工验收。生产环境 webhook 和 REST/MCP 外壳属于 Phase 7；Phase 6 的 Trino ACL 必须自动化实测。

## Gap Analysis

| Review | Requirement | Status | Missing automated proof |
| --- | --- | --- | --- |
| CR-01 | AUDIT-01 | CLOSED | `0d3d0bf` 新增 `conformance/mod.rs#audit_shutdown_drains_inflight_guard`；`5fd876d` 分离 admission 与 sink 健康。SQLite 5/5、真实 PG 6/6，均覆盖在途 guard 与并发 shutdown 后持久行可见；sink 失败仍 fail-closed。 |
| WR-01 | L2-04 | PARTIAL | `recheck_l2_acl` 仅测试调用；生产启动与小于 60 秒的复检调度及 `/health` 接线没有端到端目标。若 Phase 7 负责接线，须在其验收中保留阻断项；不能把库级单测等同生产接线。 |
| WR-02 | L2-02, L2-03 | CLOSED | `d8ed219`、`bc43cbe` 使 32 条 allow constructed-equivalent 均经 `validate_l2` 的 Display SQL 发送至受限 Trino 483，32/32 FINISHED 并核对行数；104-ID 分类 6/6，拒绝仍为零 POST。原文恢复仍为 0 条。 |

CR-01 与 WR-02 的复审见 `06-REVIEW.md`（Critical 0、Warning 1）。WR-01 保持开放。

## Validation Sign-Off

- [ ] 18/18 任务有 `<automated>` 命令；CR-01、WR-02 已补证，L2-04 仍有 WR-01 生产接线缺口。
- [x] 每任务有自动化反馈，没有三个连续任务无验证。
- [x] Wave 0 fixture 和 target 已就位。
- [x] 命令无 watch mode；本地真实 Trino ACL 3/3、PG audit 6/6，发现数均大于 0。
- [ ] 反馈耗时见 `06-08-SUMMARY.md`；gap 关闭后才能标记 `nyquist_compliant: true`。

**Approval:** WR-01 pending；本地完整 workspace 退出 0，远端 CI 与 Phase 6 目标验收待独立对账。

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
