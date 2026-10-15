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
| 1-01-01 | 01 | 1 | （格式契约确认） | — | one-way 决策先经用户确认 | checkpoint:decision | — | n/a | ⬜ pending |
| 1-01-02 | 01 | 1 | SCHEMA-01, SCHEMA-06, DX-03 | T-01-01..04, T-01-SC | 终端控制字符净化；max_depth 32 + 32 MiB 线程 | CLI e2e + snapshot + unit | `cargo test -p ontoforge-schema && cargo test -p ontoforge --test cli --test invalid_examples` | ❌ W0 | ⬜ pending |
| 1-02-01 | 02 | 2 | DX-03 | T-01-SC | Cargo.lock 只含审计过的依赖 | workspace smoke | `cargo test --workspace` | ❌ W0 | ⬜ pending |
| 1-02-02 | 02 | 2 | DX-03 | T-01-05, T-01-06 | CI `permissions: contents: read` | file check | `rg -n "windows-latest" .github/workflows/ci.yml` | ❌ W0 | ⬜ pending |
| 1-03-01 | 03 | 2 | SCHEMA-01, SCHEMA-06 | T-01-07, T-01-08, T-01-10 | include 逃逸、符号链接、超大文件被拒绝 | integration | `cargo test -p ontoforge-schema --test loader` | ❌ W0 | ⬜ pending |
| 1-03-02 | 03 | 2 | SCHEMA-01, SCHEMA-06 | T-01-09, T-01-11 | 深层嵌套 YAML / JSON 在 1 MiB 栈上被拒绝而不崩溃；严格 JSON | integration + snapshot | `cargo test -p ontoforge-schema --test loader && cargo test -p ontoforge --test invalid_examples` | ❌ W0 | ⬜ pending |
| 1-04-01 | 04 | 2 | SCHEMA-02, SCHEMA-03 | T-01-12 | 未知字段报错不被忽略 | integration | `cargo test -p ontoforge-schema --test object_types` | ❌ W0 | ⬜ pending |
| 1-04-02 | 04 | 2 | SCHEMA-02, SCHEMA-06 | T-01-13 | 主键不变量为不可关闭的 V10 | integration + snapshot | `cargo test -p ontoforge-schema --test object_types && cargo test -p ontoforge --test invalid_examples` | ❌ W0 | ⬜ pending |
| 1-04-03 | 04 | 2 | SCHEMA-01, SCHEMA-02 | — | — | integration | `cargo test -p ontoforge-schema --test load_examples` | ❌ W0 | ⬜ pending |
| 1-05-01 | 05 | 3 | SCHEMA-04 | — | — | integration | `cargo test -p ontoforge-schema --test link_types` | ❌ W0 | ⬜ pending |
| 1-05-02 | 05 | 3 | SCHEMA-04, SCHEMA-06 | T-01-15, T-01-16 | 外键类型不一致与访问器重名为 hard error | integration + snapshot | `cargo test -p ontoforge-schema --test link_types && cargo test -p ontoforge --test invalid_examples` | ❌ W0 | ⬜ pending |
| 1-05-03 | 05 | 3 | SCHEMA-04, SCHEMA-01 | — | — | integration | `cargo test -p ontoforge-schema --test load_examples` | ❌ W0 | ⬜ pending |
| 1-06-01 | 06 | 4 | SCHEMA-05 | — | — | integration | `cargo test -p ontoforge-schema --test action_types` | ❌ W0 | ⬜ pending |
| 1-06-02 | 06 | 4 | SCHEMA-05, SCHEMA-06 | T-01-18..21 | action 不能修改主键；字面量严格校验 | integration + snapshot | `cargo test -p ontoforge-schema --test action_types && cargo test -p ontoforge --test invalid_examples` | ❌ W0 | ⬜ pending |
| 1-06-03 | 06 | 4 | SCHEMA-05, SCHEMA-01 | — | — | integration | `cargo test -p ontoforge-schema --test load_examples` | ❌ W0 | ⬜ pending |
| 1-07-01 | 07 | 5 | SCHEMA-05 | T-01-22, T-01-23 | 深层嵌套 CEL 在 1 MiB 栈上被拒绝而不崩溃 | unit + integration | `cargo test -p ontoforge-expr` | ❌ W0 | ⬜ pending |
| 1-07-02 | 07 | 5 | SCHEMA-05, SCHEMA-06 | T-01-24, T-01-25 | 只编译不求值 | integration + snapshot | `cargo test -p ontoforge-expr && cargo test -p ontoforge --test invalid_examples --test cli` | ❌ W0 | ⬜ pending |
| 1-08-01 | 08 | 5 | LINT-04 | T-01-26, T-01-27 | ONT-V 不可被配置关闭；未知规则 ID 报 V15 | unit | `cargo test -p ontoforge-schema lint::` | ❌ W0 | ⬜ pending |
| 1-08-02 | 08 | 5 | LINT-01 | — | — | unit + snapshot | `cargo test -p ontoforge-schema lint::e0` | ❌ W0 | ⬜ pending |
| 1-08-03 | 08 | 5 | LINT-01, LINT-03, LINT-04 | T-01-28 | JSON 输出无 ANSI | CLI e2e + snapshot（+ human-check：Windows Terminal 彩色输出） | `cargo test -p ontoforge --test cli_lint` | ❌ W0 | ⬜ pending |
| 1-09-01 | 09 | 6 | SCHEMA-07 | T-01-30, T-01-32 | schema 与 loader 不漂移 | integration | `cargo test -p ontoforge-schema --test schema_export` | ❌ W0 | ⬜ pending |
| 1-09-02 | 09 | 6 | SCHEMA-07 | T-01-31 | — | CLI e2e（+ human-check：编辑器补全） | `cargo test -p ontoforge --test cli_schema` | ❌ W0 | ⬜ pending |
| 1-10-01 | 10 | 6 | LINT-02 | T-01-33 | 固定正则，线性时间 | unit + snapshot | `cargo test -p ontoforge-schema lint::naming` | ❌ W0 | ⬜ pending |
| 1-10-02 | 10 | 6 | LINT-02 | T-01-34 | — | unit + snapshot | `cargo test -p ontoforge-schema lint::naming && cargo test -p ontoforge-schema --test lint_examples` | ❌ W0 | ⬜ pending |
| 1-11-01 | 11 | 6 | LINT-02, LINT-04 | T-01-35, T-01-36 | 选项键受规则声明约束 | unit + snapshot | `cargo test -p ontoforge-schema lint::structure` | ❌ W0 | ⬜ pending |
| 1-11-02 | 11 | 6 | LINT-02, DX-03 | T-01-37 | phase 门禁 | unit + snapshot + workspace | `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] 根 `Cargo.toml`、`rust-toolchain.toml`、`.gitattributes`、`.github/workflows/ci.yml`（planner 调整：不建 `.cargo/config.toml`。改为库内部在 32 MiB 作用域线程中解析 YAML 与编译 CEL，并用 1 MiB 栈线程的回归测试证明调用方栈大小无关，不再依赖调大测试线程栈的环境变量）
- [ ] 10 个 crate 的骨架和占位测试
- [ ] `ontoforge-schema` 的测试辅助函数 `load_from_memory(files)`（01-01）与 `lint_rule_on(rule_id, files, config)`（01-08，对应原 `lint_fixture`）
- [ ] `examples/ontologies/aviation/`、`aviation-json/`、`invalid/<case>/`（V01-V14 每个码至少一个样例；V15 是 `ontoforge.toml` 错误，由 01-08 的 `cli_lint` 与配置单元测试覆盖）
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
