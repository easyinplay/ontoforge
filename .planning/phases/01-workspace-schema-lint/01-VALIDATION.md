---
phase: "1"
slug: "workspace-schema-lint"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
status: validated
nyquist_compliant: true
wave_0_complete: true
created: "2026-10-15"
validated: "2026-11-04"
---

# Phase 1 — Validation Strategy

> 本 phase 的验证契约，规定执行过程中多久跑一次测试、每个需求用什么测试证明。完整测试映射见 `01-RESEARCH.md` 的 `## Validation Architecture` 节。

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust 内置 `cargo test` + `insta` 1.49.0（快照）+ `jsonschema`（dev，`default-features = false`） |
| **Config file** | 根 `Cargo.toml`、`rust-toolchain.toml`、`.gitattributes`；按最终计划未建 `.cargo/config.toml`，库内部隔离大栈，调用方 1 MiB 栈回归测试已通过 |
| **Quick run command** | `cargo test -p ontoforge-schema` |
| **Full suite command** | `cargo test --workspace` |
| **Gate extras** | `cargo clippy --workspace --all-targets -- -D warnings`；`cargo fmt --all --check` |
| **Observed runtime** | 新增颜色测试 warm 1.79s；本次冷编译 schema/expr 2m08s、CLI 4m27s |

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
| 1-01-01 | 01 | 1 | （格式契约确认） | — | one-way 决策先经用户确认 | checkpoint:decision | ADR-0012（accepted） | n/a | SKIP：历史用户决策，无运行时行为 |
| 1-01-02 | 01 | 1 | SCHEMA-01, SCHEMA-06, DX-03 | T-01-01..04, T-01-SC | 终端控制字符净化；max_depth 32 + 32 MiB 线程 | CLI e2e + snapshot + unit | `cargo test -p ontoforge-schema && cargo test -p ontoforge --test cli --test invalid_examples` | yes | green |
| 1-02-01 | 02 | 2 | DX-03 | T-01-SC | 锁定依赖与可复现 workspace 构建 | workspace smoke | `cargo test --workspace` | yes | green |
| 1-02-02 | 02 | 2 | DX-03 | T-01-05, T-01-06 | CI `permissions: contents: read` | file check | `rg -n "windows-latest" .github/workflows/ci.yml` | yes | green |
| 1-03-01 | 03 | 2 | SCHEMA-01, SCHEMA-06 | T-01-07, T-01-08, T-01-10 | include 逃逸、符号链接、超大文件被拒绝 | integration | `cargo test -p ontoforge-schema --test loader` | yes | green |
| 1-03-02 | 03 | 2 | SCHEMA-01, SCHEMA-06 | T-01-09, T-01-11 | 深层嵌套 YAML / JSON 在 1 MiB 栈上被拒绝而不崩溃；严格 JSON | integration + snapshot | `cargo test -p ontoforge-schema --test loader && cargo test -p ontoforge --test invalid_examples` | yes | green |
| 1-04-01 | 04 | 2 | SCHEMA-02, SCHEMA-03 | T-01-12 | 未知字段报错不被忽略 | integration | `cargo test -p ontoforge-schema --test object_types` | yes | green |
| 1-04-02 | 04 | 2 | SCHEMA-02, SCHEMA-06 | T-01-13 | 主键不变量为不可关闭的 V10 | integration + snapshot | `cargo test -p ontoforge-schema --test object_types && cargo test -p ontoforge --test invalid_examples` | yes | green |
| 1-04-03 | 04 | 2 | SCHEMA-01, SCHEMA-02 | — | — | integration | `cargo test -p ontoforge-schema --test load_examples` | yes | green |
| 1-05-01 | 05 | 3 | SCHEMA-04 | — | — | integration | `cargo test -p ontoforge-schema --test link_types` | yes | green |
| 1-05-02 | 05 | 3 | SCHEMA-04, SCHEMA-06 | T-01-15, T-01-16 | 外键类型不一致与访问器重名为 hard error | integration + snapshot | `cargo test -p ontoforge-schema --test link_types && cargo test -p ontoforge --test invalid_examples` | yes | green |
| 1-05-03 | 05 | 3 | SCHEMA-04, SCHEMA-01 | — | — | integration | `cargo test -p ontoforge-schema --test load_examples` | yes | green |
| 1-06-01 | 06 | 4 | SCHEMA-05 | — | — | integration | `cargo test -p ontoforge-schema --test action_types` | yes | green |
| 1-06-02 | 06 | 4 | SCHEMA-05, SCHEMA-06 | T-01-18..21 | action 不能修改主键；字面量严格校验 | integration + snapshot | `cargo test -p ontoforge-schema --test action_types && cargo test -p ontoforge --test invalid_examples` | yes | green |
| 1-06-03 | 06 | 4 | SCHEMA-05, SCHEMA-01 | — | — | integration | `cargo test -p ontoforge-schema --test load_examples` | yes | green |
| 1-07-01 | 07 | 3 | SCHEMA-05 | T-01-22, T-01-25 | 深层嵌套 CEL 在 1 MiB 栈上被拒绝而不崩溃；超长、未闭合字符串不进入编译器；预检按 CEL 词法跳过字符串 | integration | `cargo test -p ontoforge-expr --test analyze` | yes | green |
| 1-07-02 | 07 | 3 | SCHEMA-05 | T-01-23 | 遍历在大栈线程内；comprehension 变量不算自由变量 | integration | `cargo test -p ontoforge-expr && cargo clippy -p ontoforge-expr --all-targets -- -D warnings` | yes | green |
| 1-08-01 | 08 | 5 | LINT-04 | T-01-26, T-01-27 | ONT-V 不可被配置关闭；未知规则 ID 报 V15 | unit | `cargo test -p ontoforge-schema lint::` | yes | green |
| 1-08-02 | 08 | 5 | LINT-01 | — | — | unit + snapshot | `cargo test -p ontoforge-schema lint::e0` | yes | green |
| 1-08-03 | 08 | 5 | LINT-01, LINT-03, LINT-04 | T-01-28 | JSON 输出无 ANSI | CLI e2e + snapshot（+ human-check：Windows Terminal 彩色输出） | `cargo test -p ontoforge --test cli_lint` | yes | green |
| 1-09-01 | 09 | 6 | SCHEMA-07 | T-01-30, T-01-32 | schema 与 loader 不漂移 | integration | `cargo test -p ontoforge-schema --test schema_export` | yes | green |
| 1-09-02 | 09 | 6 | SCHEMA-07 | T-01-31 | — | CLI e2e（+ human-check：编辑器补全） | `cargo test -p ontoforge --test cli_schema` | yes | green |
| 1-10-01 | 10 | 6 | LINT-02 | T-01-33 | 固定正则，线性时间 | unit + snapshot | `cargo test -p ontoforge-schema lint::naming` | yes | green |
| 1-10-02 | 10 | 6 | LINT-02 | T-01-34 | — | unit + snapshot | `cargo test -p ontoforge-schema lint::naming && cargo test -p ontoforge-schema --test lint_examples` | yes | green |
| 1-11-01 | 11 | 6 | LINT-02, LINT-04 | T-01-35, T-01-36 | 选项键受规则声明约束 | unit + snapshot | `cargo test -p ontoforge-schema lint::structure` | yes | green |
| 1-11-02 | 11 | 6 | LINT-02, DX-03 | T-01-37 | phase 门禁 | unit + snapshot + workspace | `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check` | yes | green |
| 1-12-01 | 12 | 5 | SCHEMA-05, SCHEMA-06 | T-01-24, T-01-39 | 只编译不求值；V13 为 hard error，定位到 condition 值 | integration + CLI e2e | `cargo test -p ontoforge-expr --test criteria && cargo test -p ontoforge --test cli --test invalid_examples` | yes | green |
| 1-12-02 | 12 | 5 | SCHEMA-05 | T-01-38 | 类型推断在 analyze 的大栈线程内运行；不可推断即 Any | integration | `cargo test -p ontoforge-expr` | yes | green |
| 1-12-03 | 12 | 5 | SCHEMA-05, SCHEMA-06 | — | — | snapshot + CLI e2e | `cargo test -p ontoforge --test invalid_examples --test cli && cargo clippy -p ontoforge-expr -p ontoforge --all-targets -- -D warnings` | yes | green |


| 1-13-01 | 13 | 6 | LINT-04 | T-01-40 | TOML 日期与溢出值拒绝为 V15，保留字段 span | unit + integration | `cargo test -p ontoforge-schema --locked lint::config; cargo test -p ontoforge --test cli_lint --locked` | yes | green |
| 1-13-02 | 13 | 6 | SCHEMA-06, LINT-03 | T-01-41 | acronym 建议正确；auto 颜色环境按契约；JSON 无 ANSI | unit + CLI integration | `cargo test -p ontoforge-schema --locked ir::names; cargo test -p ontoforge --test cli_color --locked` | yes | green |
| 1-13-03 | 13 | 6 | SCHEMA-06 | — | 中文字符列、mid-char clamp、BOM/CRLF 与 CEL 位置保持 | unit + integration | `cargo test -p ontoforge-schema --locked; cargo test -p ontoforge-expr --test analyze --locked` | yes | green |
| 1-14-01 | 14 | 7 | SCHEMA-07 | T-01-42, T-01-43 | 两份真实配置严格解析、本地 schema、manifest 排除 | integration | `cargo test -p ontoforge-schema --test schema_export --locked editor_configs_associate_definition_schema_with_examples` | yes | green（WARNING：配置层） |
| 1-14-02 | 14 | 7 | SCHEMA-07 | T-01-44 | 每 document 的 modeline 存在且指向真实 schema；示例 IR 不变 | integration | `cargo test -p ontoforge-schema --locked --test schema_export --test load_examples` | yes | green |
| 1-14-03 | 14 | 7 | SCHEMA-07, DX-03 | — | 文档与配置相符；三项质量门禁有实际 CI 结果 | integration + file check | `cargo test --workspace --locked -- --test-threads=1`（冻结源码 CI）；`rg -n '编辑器接入' CONTRIBUTING.md` | yes | green（WARNING：编辑器见 UAT） |

*Status: green = 行为测试已运行通过；SKIP = 已解释的非运行时决策。命令的本次运行与冻结源码 CI 证据见下文，不把 ignored 计作 passed。*

---

## Wave 0 Requirements

- [x] 根 `Cargo.toml`、`rust-toolchain.toml`、`.gitattributes`、`.github/workflows/ci.yml`（planner 调整：不建 `.cargo/config.toml`。改为库内部在 32 MiB 作用域线程中解析 YAML 与编译 CEL，并用 1 MiB 栈线程的回归测试证明调用方栈大小无关，不再依赖调大测试线程栈的环境变量）
- [x] 10 个 crate 的骨架和占位测试
- [x] `ontoforge-schema` 的测试辅助函数 `load_from_memory(files)`（01-01）与 `lint_rule_on(rule_id, files, config)`（01-08，对应原 `lint_fixture`）
- [x] `examples/ontologies/aviation/`、`aviation-json/`、`invalid/<case>/`（V01-V14 每个码至少一个样例；V15 是 `ontoforge.toml` 错误，由 01-08 的 `cli_lint` 与配置单元测试覆盖）
- [x] 必测项：`deeply_nested_expression_is_rejected_not_crashing`、`deeply_nested_yaml_is_rejected_not_crashing`、`json_trailing_comma_is_rejected`、`yaml_yes_no_stay_strings`、`diagnostic_paths_use_forward_slash`、`properties_keep_declared_order`、`unknown_lint_rule_id_is_error`

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| miette 彩色输出在 Windows Terminal 中可读 | SCHEMA-06 | 人眼阅读效果 | `01-UAT.md` Test 1 已 pass；本次新增 CLI 颜色测试只证明 ANSI 开关与 JSON 输出行为 |

---

## Validation Sign-Off

- [x] 所有 task 都有 `<automated>` 验证命令，或依赖 Wave 0 中的测试
- [x] 采样连续性：不能连续 3 个 task 没有自动化验证
- [x] Wave 0 覆盖所有 MISSING 引用
- [x] 不使用 watch 模式参数
- [x] warm 单项反馈 < 120s；本次冷编译 schema/expr 2m08s、CLI 4m27s，冷编译不满足 120s，记为 WARNING
- [x] frontmatter 中设置 `nyquist_compliant: true`

**Approval:** automated validation passed（2026-11-04）；既有人工记录不扩大解释，见 WARNING。

## Nyquist 审计工作记录（2026-11-04）

按 `planning-with-files` 的持久化要求，本次审计的计划、发现与执行进度集中记录在本验证契约，遵守 auditor 仅允许修改测试、fixture 与 VALIDATION.md 的写入边界。

- 范围：Phase 1 的 14 份 PLAN / SUMMARY、现有 VALIDATION / VERIFICATION / UAT、schema / expr / CLI 测试；生产实现只读。
- 计划：先把 12 个 requirement 与既有测试对应，再执行实际测试；仅在行为覆盖缺失时新增测试；最后据实更新映射与 Sign-Off。
- 初始发现：原映射 29 行仍为 pending，缺少 01-13 与 01-14；已有测试文件存在。历史 `299 passed` 不作为本次运行结果。
- 人工证据：`01-UAT.md` Test 1、Test 3 已 pass；Test 2 由用户接受收口，但 note 明确补全弹窗、bogus 标红与 JSON manifest 无误报没有逐一观察。本次不扩大此证据的含义。
- 进度：schema/expr 399 passed，CLI 定向套件 52 passed，合计 451 passed / 0 failed / 0 ignored；新增颜色测试单独重跑 1 passed。
- 工具边界：本会话无 Warp / ralph-loop callable capability，实际运行通过 PowerShell / Cargo；不声称执行了不可用工具。

### 审计分类（新增测试前）

| Requirement / Task | 分类 | 行为依据 / 缺口 |
|---|---|---|
| SCHEMA-01 | COVERED | loader 的多文件、include、顺序无关与 YAML/JSON 等价测试实际通过 |
| SCHEMA-02、SCHEMA-03 | COVERED | object_types 的完整字段、13 种 base type、required、主键约束测试实际通过 |
| SCHEMA-04 | COVERED | link_types 的三种基数、两端 accessor、外键类型与重名拒绝测试实际通过 |
| SCHEMA-05 | COVERED | action_types、analyze、criteria 的参数、默认值、五种规则、词法边界、1 MiB 栈与 V13 测试实际通过 |
| SCHEMA-06 | COVERED | diagnostics、loader、criteria 对错误码、文件、字符列与字节 span 的断言实际通过；CLI 输出验证进行中 |
| SCHEMA-07 | COVERED（WARNING） | schema_export 的真实 schema 校验、入库一致性与配置 / modeline 测试实际通过；编辑器 popup 不由这些测试证明 |
| LINT-01、LINT-02、LINT-04 | COVERED | E08/E09、W01-W11 的触发 / clean / override 与配置拒绝测试实际通过；CLI 退出码验证进行中 |
| LINT-03 / 1-13-02 | PARTIAL | 既有 CLI 测试均用固定 NO_COLOR；缺少 auto 对空 NO_COLOR、CLICOLOR、CLICOLOR_FORCE、TERM 的真实进程断言 |
| DX-03 | COVERED（WARNING） | 历史 Windows workspace 自动化证据存在；本次 schema/expr 实跑通过，全 workspace 本次不冒称已重跑 |

已为 `1-13-02` 新增 `crates/ontoforge/tests/cli_color.rs`，6 个进程场景对 ONT-V04、exit 1、ANSI 有无与 JSON 内容作实质断言，首次运行与独立重跑均通过，状态 FILLED。


## 最终验证证据（2026-11-04）

### 本次运行

| Command | 真实结果 | 覆盖范围 |
|---|---|---|
| `cargo test -p ontoforge-schema -p ontoforge-expr --locked` | exit 0；399 passed / 0 failed / 0 ignored | schema lib 139；action 28；bindings 32；diagnostics 16；links 21；lint examples 1；load examples 6；loader 34；objects 21；schema export 14；scope 16；expr lib 3、analyze 21、criteria 20、rowpred eval 14、subset 13 |
| `cargo test -p ontoforge --locked --lib --test cli --test cli_lint --test cli_schema --test invalid_examples --test cli_color` | exit 0；52 passed / 0 failed / 0 ignored | lib 21、cli 8、cli_lint 17、cli_schema 4、invalid_examples 1、cli_color 1 |
| `$env:CARGO_BUILD_JOBS = '2'; cargo test -p ontoforge --test cli_color --locked` | exit 0；1 passed，warm 1.79s wall time | 新增测试独立重跑；6 个环境组合逐一断言 |
| `cargo metadata --no-deps --format-version 1 --locked` | exit 0；11 个 crate，均 edition 2024 / rust-version 1.98 | Phase 1 的 10 个 crate 仍在；新增 engine 属后续 phase，不要求回退至 10 |
| `rg -n 'windows-latest|ubuntu-latest|contents: read|cargo fmt --all --check|cargo clippy --workspace --all-targets|cargo test --workspace' .github/workflows/ci.yml` | exit 0 | 当前 CI 双平台、只读权限、三项质量门禁 |

`399` 包含当前这两个 crate 的后续 phase 测试，不把它宣称为 Phase 1 原始测试数量。`invalid_examples` 是 1 个数据驱动测试，内部遍历真实 invalid 样例，不把每个样例重复计成独立 test。

### 全 workspace 证据复用

- `target/phase7-ci-41669982375.json`：run 41669982375，head `adc02278b5cc55111def9a30a1c3b778485e8279`，conclusion `success`；2026-11-03 16:02:06Z 至 16:32:12Z。
- `target/phase7-ci-41669982375-windows.log` 与 `-windows-counts.json`：1490 passed / 0 failed / 117 ignored。
- `target/phase7-ci-41669982375-ubuntu.log` 与 `-ubuntu-counts.json`：1497 passed / 0 failed / 117 ignored。
- CI 三项门禁（fmt、clippy、workspace test）均 success。`git diff --name-only adc0227..HEAD` 只列 `.planning/`，生产源码与该 CI 的源码一致；本次新增测试另有本机运行证据。
- 本次没有重跑全 workspace。额外排队的 `cargo test --workspace --lib --locked crate_name_has_ontoforge_prefix` 在触发无必要的后续 crate 编译后取消，不计为通过，不用于证明 DX-03。
- 本机终端与 CI 日志均为真实运行证据；target 中下载的 CI 日志不是本次新增的 commit 文件。远端 run ID 作为长期定位依据。

## Gap 闭环与 WARNING

| Gap / Task | 初始状态 | 最小测试 / 实际行为 | 最终状态 | Debug iterations |
|---|---|---|---|---|
| G-01-COLOR / 1-13-02 / LINT-03 | PARTIAL | `cli_color.rs::color_environment_controls_human_output_and_never_colors_json`：forced color 有 ANSI；空 NO_COLOR 不压制 forced color；非空 NO_COLOR 覆盖 forced color；CLICOLOR=0 无 ANSI；TERM=dumb 无 ANSI；forced color 下 JSON 仍可解析、ONT-V04 / objects.yaml / exit 1 正确 | FILLED / green | 0/3（首次通过，另独立复跑通过） |
| 原表 29 行与遗漏 01-13/01-14 的验证映射 | no_automated_command / stale status | 35 个 task 对应 34 个自动化 task 与 1 个历史决策；12 个 requirement 均有实际绿色测试证据 | FILLED；checkpoint 显式 SKIP | 0/3 |

- **WARNING（SCHEMA-07）**：`schema_export` 证明 schema、配置与 modeline 行为；真实编辑器弹窗不能由 globset 或 schema validator 代替。`01-UAT.md` Test 2 用户已接受 pass，但其 note 明确补全弹窗、bogus 标红与 aviation-json manifest 无误报未逐项观察。本审计保留该限定，不新建 manual-only 来获取绿色状态。
- **WARNING（loader 环境）**：既有 symlink / 非 UTF-8 文件名测试在环境不支持时会提前 return，本机显示 ok 不能单独证明分支执行。最新 Ubuntu CI 中同名测试实际显示 ok；本审计未修改测试以掩盖该边界。
- **WARNING（反馈时间）**：冷编译超 120s，warm 新增测试独立命令 wall time 1.79s。通过功能测试不能证明冷编译时延满足原采样目标。
- **SKIP（1-01-01）**：格式选择属于历史用户决策；ADR-0012 已 accepted，没有可重跑的运行时行为。其产生的 API / 格式契约已由后续行为测试覆盖。
- 未发现实现 bug；无 BLOCKER / ESCALATED；未修改实现、配置、schema、既有 fixture 或快照。

## Validation Audit

| Metric | Value |
|---|---|
| Date | 2026-11-04 |
| Gaps found | 1 个真实行为缺口 + 1 组失效映射 |
| Resolved | 2 |
| Escalated | 0 |
| Requirement coverage | 12 COVERED（上述 WARNING 保留） |
| Task map | 34 green + 1 justified SKIP |
| Test files created | `crates/ontoforge/tests/cli_color.rs` |
| Implementation files modified | 0 |

本次 `planning-with-files` 闭环已记录于本文件：计划、初始分类、运行结果、gap 状态与限定证据均已持久化。生产实现保持只读，未提交 git commit。

## Validation Audit 2026-11-04

| Metric | Count |
|---|---|
| Gaps found | 2 |
| Resolved | 2 |
| Escalated | 0 |
