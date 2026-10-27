---
phase: 06-l2-raw-dql-toolsets-audit
reviewed: 2026-10-27T21:36:32Z
depth: deep
files_reviewed: 34
files_reviewed_list:
  - .github/workflows/ci.yml
  - ci/query-services.sh
  - ci/trino/l2/access-control.properties
  - ci/trino/l2/config.properties
  - ci/trino/l2/rules.json
  - ci/trino/l2/seed.sql
  - crates/ontoforge-action/tests/entrypoints.rs
  - crates/ontoforge-engine/Cargo.toml
  - crates/ontoforge-engine/src/config.rs
  - crates/ontoforge-engine/src/printer/l2.rs
  - crates/ontoforge-engine/src/printer/mod.rs
  - crates/ontoforge-engine/src/trino/l2_health.rs
  - crates/ontoforge-engine/src/trino/mod.rs
  - crates/ontoforge-engine/tests/l2_gate.rs
  - crates/ontoforge-engine/tests/trino_l2_acl_it.rs
  - crates/ontoforge-query/Cargo.toml
  - crates/ontoforge-query/src/aggregate.rs
  - crates/ontoforge-query/src/execute.rs
  - crates/ontoforge-query/src/l2.rs
  - crates/ontoforge-query/src/lib.rs
  - crates/ontoforge-query/tests/l2_discovery.rs
  - crates/ontoforge-query/tests/l2_execute.rs
  - crates/ontoforge-storage/migrations/postgres/0005_read_audit.sql
  - crates/ontoforge-storage/migrations/postgres/0006_audit_truncated.sql
  - crates/ontoforge-storage/migrations/sqlite/0005_read_audit.sql
  - crates/ontoforge-storage/migrations/sqlite/0006_audit_truncated.sql
  - crates/ontoforge-storage/src/lib.rs
  - crates/ontoforge-storage/src/read_audit.rs
  - crates/ontoforge-storage/src/sql_exit.rs
  - crates/ontoforge-storage/src/store.rs
  - crates/ontoforge-storage/tests/conformance/mod.rs
  - crates/ontoforge-storage/tests/conformance_pg.rs
  - crates/ontoforge-storage/tests/scope_store.rs
  - crates/ontoforge/tests/cli_reload.rs
findings:
  critical: 0
  warning: 1
  info: 0
  total: 1
status: issues_found
---

# Phase 6: Code Review Report

**Reviewed:** 2026-10-27T21:36:32Z
**Depth:** deep
**Files Reviewed:** 34
**Status:** issues_found

## Summary

`CR-01` 已由 `0d3d0bf` / `5fd876d` 修复，SQLite 在途 guard 关闭测试通过。复审 `d8ed219` / `bc43cbe` 后，`WR-02` 已关闭：CI 的受限 Trino 483 测试现在执行 corpus 中全部 32 条 `allow` SQL，检查完成结果与行数；本地 `l2_corpus_each_case_runs_production_gate` 通过（1 passed）。父会话报告真实 Trino 测试 3/3 通过。`evaluation-scope` 原先返回 `degraded`（`no-task-commit-rows`）。当前仅剩 ACL 启动及周期复检接线 Warning。

## Narrative Findings (AI reviewer)

### WR-01 [WARNING]: ACL 自检没有生产启动及周期复检接线

**File:** `crates/ontoforge-engine/src/trino/mod.rs:129-145`、`crates/ontoforge-engine/src/trino/l2_health.rs:62-75`
**Issue:** `build_trino_engines` 构造的 L2 实例初始为 `NotVerified`；唯一能置为 `Enabled` 的 `recheck_l2_acl` 在仓库内仅由测试调用。即便外层调用一次，`status()` 也在 60 秒后返回 `Expired`；没有生产调度再次提供仍然活跃的其他身份 query ID。这保持 fail-closed，但当前库接入后 L2 默认不可用，并且单次自检后最多可用 60 秒。Phase 7 的外壳接线尚未实现，L2-04 的启动与周期行为仍依赖该工作。
**Fix:** 在正式 L2 入口启用前接入启动 attestation，维护另一身份的可验证活跃 query，并按小于 60 秒的间隔复检；复检失败保持禁用，覆盖启动、过期、恢复的端到端测试。若此职责明确划归 Phase 7，在其验收中保留阻塞项。

---

_Reviewed: 2026-10-27T21:36:32Z_
_Reviewer: the agent (gsd-code-reviewer)_
_Depth: deep_
