---
phase: "1"
slug: "workspace-schema-lint"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-10-15"
---

# Phase 1 — Validation Strategy

> 本 phase 的验证契约，规定执行过程中多久跑一次测试、每个需求用什么测试证明。完整测试映射见 `01-RESEARCH.md` 的 `## Validation Architecture` 节。

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust 内置 `cargo test` + `insta` 1.49.0（快照）+ `jsonschema`（dev，`default-features = false`） |
| **Config file** | 无，Wave 0 新建根 `Cargo.toml`、`rust-toolchain.toml`、`.cargo/config.toml`、`.gitattributes` |
| **Quick run command** | `cargo test -p ontoforge-schema` |
| **Full suite command** | `cargo test --workspace` |
| **Gate extras** | `cargo clippy --workspace --all-targets -- -D warnings`；`cargo fmt --all --check` |
| **Estimated runtime** | 约 60 秒（首次编译 `cel` 更慢） |

---

## Sampling Rate

- **每个 task commit 后：** 跑 `cargo test -p <被改动的 crate>`
- **每个 plan wave 结束后：** 跑 `cargo test --workspace` 和 clippy
- **执行 `/gsd-verify-work` 之前：** 全套件必须绿，`fmt --check` 也要通过
- **最大反馈延迟：** 120 秒

---

## Per-Task Verification Map

由 planner 按 task 填写；每个需求对应的测试命令见 `01-RESEARCH.md` §Validation Architecture 的 Phase Requirements → Test Map。

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 1-xx-xx | xx | x | SCHEMA-01..07, LINT-01..04, DX-03 | 见 PLAN threat_model | 深层嵌套输入被拒绝而不是导致进程崩溃 | unit / snapshot / CLI | 见 RESEARCH 测试映射 | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] 根 `Cargo.toml`、`rust-toolchain.toml`、`.cargo/config.toml`、`.gitattributes`、`.github/workflows/ci.yml`
- [ ] 10 个 crate 的骨架和占位测试
- [ ] `ontoforge-schema` 的测试辅助函数 `load_from_memory(files)` 与 `lint_fixture(rule, name)`
- [ ] `examples/ontologies/aviation/`、`aviation-json/`、`invalid/<case>/`（每个 V 码至少一个样例）
- [ ] 必测项：`deeply_nested_expression_is_rejected_not_crashing`、`deeply_nested_yaml_is_rejected_not_crashing`、`json_trailing_comma_is_rejected`、`yaml_yes_no_stay_strings`、`diagnostic_paths_use_forward_slash`、`properties_keep_declared_order`、`unknown_lint_rule_id_is_error`

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| miette 彩色输出在 Windows Terminal 中可读 | SCHEMA-06 | 自动化测试只能对比无色输出 | 在 Windows Terminal 里对 `examples/ontologies/invalid/` 中的样例运行 `ontoforge validate`，确认源码片段和下划线位置正确 |

---

## Validation Sign-Off

- [ ] 所有 task 都有 `<automated>` 验证命令，或依赖 Wave 0 中的测试
- [ ] 采样连续性：不能连续 3 个 task 没有自动化验证
- [ ] Wave 0 覆盖所有 MISSING 引用
- [ ] 不使用 watch 模式参数
- [ ] 反馈延迟 < 120s
- [ ] frontmatter 中设置 `nyquist_compliant: true`

**Approval:** pending
