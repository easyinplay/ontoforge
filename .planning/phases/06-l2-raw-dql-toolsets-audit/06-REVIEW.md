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
  warning: 0
  info: 0
  total: 0
status: clean
---

# Phase 6: Code Review Report

**Reviewed:** 2026-10-27T21:36:32Z
**Depth:** deep
**Files Reviewed:** 34
**Status:** clean

## Summary

原 CR-01 与 WR-02 的关闭证据见各自修复提交和 `06-VALIDATION.md`。WR-01 的库级缺口由 `06-10` 补齐并经独立增量复审关闭，详 `06-10-REVIEW.md`；该增量报告拥有新发现、修复和最终快照证据。Phase 7 服务接线及 `/health` 按 ADR-0028 独立阻断验收。当前开放 findings 为 0，Phase 6 总验收与远端 CI 仍单独裁定。

## Narrative Findings (AI reviewer)

### WR-01 [WARNING]: ACL 自检没有生产启动及周期复检接线

**File:** `crates/ontoforge-engine/src/trino/mod.rs:129-145`、`crates/ontoforge-engine/src/trino/l2_health.rs:62-75`
**Issue:** `build_trino_engines` 构造的 L2 实例初始为 `NotVerified`；唯一能置为 `Enabled` 的 `recheck_l2_acl` 在仓库内仅由测试调用。即便外层调用一次，`status()` 也在 60 秒后返回 `Expired`；没有生产调度再次提供仍然活跃的其他身份 query ID。这保持 fail-closed，但当前库接入后 L2 默认不可用，并且单次自检后最多可用 60 秒。Phase 7 的外壳接线尚未实现，L2-04 的启动与周期行为仍依赖该工作。
**Fix:** 在正式 L2 入口启用前接入启动 attestation，维护另一身份的可验证活跃 query，并按小于 60 秒的间隔复检；复检失败保持禁用，覆盖启动、过期、恢复的端到端测试。若此职责明确划归 Phase 7，在其验收中保留阻塞项。

**Resolution:** 库级缺口已关闭，最终独立复审为 `06-10-REVIEW.md` 的 `9738166` 快照；API、配置、TDD 和真实默认并行 Trino 证据见 `06-10-SUMMARY.md`。服务生命周期与 HTTP 告警由 ADR-0028、Phase 7 SC6 保留阻断，L2-04 整体仍 Pending。本段保留原始发现作为历史，不代表当前开放问题。

---

_Reviewed: 2026-10-27T21:36:32Z_
_Reviewer: the agent (gsd-code-reviewer)_
_Depth: deep_
