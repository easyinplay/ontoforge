---
phase: "2"
slug: "source-binding-engine"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-10-16"
---

# Phase 2 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> 需求到测试的映射见 `02-RESEARCH.md` § Validation Architecture，本文件不复制。

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` + `insta` + `proptest` + `wiremock` + `sqlx` sqlite（测试内造表） |
| **Config file** | none；真实 Trino 测试以 `#[ignore]` 门控，`PROPTEST_CASES` 调例数 |
| **Quick run command** | `cargo test -p <被改动的 crate>` |
| **Full suite command** | `cargo test --workspace --locked` |
| **Linux-only integration** | `cargo test -p ontoforge-engine --test trino_it --locked -- --ignored` |
| **Estimated runtime** | ~120 seconds（冷构建除外） |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p <被改动的 crate>`
- **After every plan wave:** Run `cargo test --workspace --locked` + `cargo clippy --workspace --all-targets --locked -- -D warnings` + `cargo fmt --all --check`
- **Before `/gsd-verify-work`:** Full suite must be green（Windows 与 Ubuntu），Linux 集成 job 实际执行 ≥ 1 个 `--ignored` 测试
- **Max feedback latency:** 120 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| （planner 生成 PLAN.md 后由 plan / validate-phase 回填） | | | | | | | | | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/ontoforge-engine/`（Cargo.toml、lib.rs、测试目录）与 workspace 依赖项
- [ ] `crates/ontoforge-schema/tests/bindings.rs` 与 `examples/ontologies/invalid/` 下的绑定 / 类型契约样例目录
- [ ] `crates/ontoforge/tests/cli_online.rs`（tempdir 内用 `sqlx` 造 SQLite 表，不依赖 Docker）
- [ ] `ci/trino/catalog/*.properties`、`ci/pg/seed.sql`、`.github/workflows/nightly.yml`
- [ ] 既有 fixture 的 `backing` 迁移作为首个 plan

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| 真实 Trino / Iceberg 行为 | ENGINE-01, ENGINE-03, ENGINE-05 | 本机无 Docker / Trino，只能在 Linux CI 执行 | 查看 PR CI 的 Trino job 与 nightly Iceberg job 日志，确认 `--ignored` 测试实际运行且通过 |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
