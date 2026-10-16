---
phase: 01-workspace-schema-lint
verified: 2026-10-16T12:00:00Z
status: human_needed
score: 10/11 must-haves verified
covered_files:
  - ".github/workflows/ci.yml"
  - ".planning/REQUIREMENTS.md"
  - ".planning/phases/01-workspace-schema-lint/01-01-PLAN.md"
  - ".planning/phases/01-workspace-schema-lint/01-01-SUMMARY.md"
  - ".planning/phases/01-workspace-schema-lint/01-02-PLAN.md"
  - ".planning/phases/01-workspace-schema-lint/01-02-SUMMARY.md"
  - ".planning/phases/01-workspace-schema-lint/01-03-PLAN.md"
  - ".planning/phases/01-workspace-schema-lint/01-03-SUMMARY.md"
  - ".planning/phases/01-workspace-schema-lint/01-04-PLAN.md"
  - ".planning/phases/01-workspace-schema-lint/01-04-SUMMARY.md"
  - ".planning/phases/01-workspace-schema-lint/01-05-PLAN.md"
  - ".planning/phases/01-workspace-schema-lint/01-05-SUMMARY.md"
  - ".planning/phases/01-workspace-schema-lint/01-06-PLAN.md"
  - ".planning/phases/01-workspace-schema-lint/01-06-SUMMARY.md"
  - ".planning/phases/01-workspace-schema-lint/01-07-PLAN.md"
  - ".planning/phases/01-workspace-schema-lint/01-07-SUMMARY.md"
  - ".planning/phases/01-workspace-schema-lint/01-08-PLAN.md"
  - ".planning/phases/01-workspace-schema-lint/01-08-SUMMARY.md"
  - ".planning/phases/01-workspace-schema-lint/01-09-PLAN.md"
  - ".planning/phases/01-workspace-schema-lint/01-09-SUMMARY.md"
  - ".planning/phases/01-workspace-schema-lint/01-10-PLAN.md"
  - ".planning/phases/01-workspace-schema-lint/01-10-SUMMARY.md"
  - ".planning/phases/01-workspace-schema-lint/01-11-PLAN.md"
  - ".planning/phases/01-workspace-schema-lint/01-11-SUMMARY.md"
  - ".planning/phases/01-workspace-schema-lint/01-12-PLAN.md"
  - ".planning/phases/01-workspace-schema-lint/01-12-SUMMARY.md"
  - ".planning/phases/01-workspace-schema-lint/01-13-PLAN.md"
  - ".planning/phases/01-workspace-schema-lint/01-13-SUMMARY.md"
  - ".planning/phases/01-workspace-schema-lint/01-14-PLAN.md"
  - ".planning/phases/01-workspace-schema-lint/01-14-SUMMARY.md"
  - ".vscode/settings.json"
  - ".zed/settings.json"
  - "CONTRIBUTING.md"
  - "Cargo.toml"
  - "crates/ontoforge-schema/src/lib.rs"
  - "crates/ontoforge-schema/tests/schema_export.rs"
  - "crates/ontoforge/src/main.rs"
  - "examples/ontologies/aviation/actions.yaml"
  - "examples/ontologies/aviation/links.yaml"
  - "examples/ontologies/aviation/objects.yaml"
  - "schemas/ontoforge-definition.schema.json"
  - "schemas/ontoforge-manifest.schema.json"
covered_digest: "v1:sha256:1a414b08d4af61b1cd0d346d4046b65a0b722b773e66152720b97eaec55ae165"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: human_needed
  previous_score: 4/4 must-haves verified
  trigger: "UAT Test 2 报告 gap G-1-2；执行了 gap-closure plan 01-14"
  gaps_closed:
    - "G-1-2：多文档 YAML 的第 2 个及之后的 document 没有 schema 关联（代码与配置层面已关闭；真实编辑器内复测待人工）"
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "UAT Test 2 复测（Zed，Windows）：在 Zed 中打开仓库根目录（已打开则执行 `editor: restart language server` 让 `.zed/settings.json` 生效），打开 `examples/ontologies/aviation/objects.yaml`，在文件末尾追加一行 `---` 和一行 `kind: ` 后按 Ctrl+Space；再在该 document 中加一行 `bogus: 1`；最后打开 `examples/ontologies/aviation-json/ontology.yaml`"
    expected: "补全列出 ObjectType、LinkType、ActionType；`bogus` 字段被标红；`aviation-json/ontology.yaml` 没有 schema 错误提示。选 ObjectType 后在新行按 Ctrl+Space，只出现 ObjectType 的字段（apiName、primaryKey、properties 等）"
    why_human: "补全弹窗与编辑器内诊断只能在真实编辑器中观察。本次 headless 驱动与 Zed 同版本的 yaml-language-server 1.25.0 已证明服务端行为正确，但 Zed 是否把 `.zed/settings.json` 的 `lsp.yaml-language-server.settings` 传给服务端、`./schemas/...` 相对路径在 Zed 内是否按工作区根解析，只有真实 Zed 能确认。通过后把 01-UAT.md Test 2 的 result 从 issue 改为 pass"
  - test: "（可选，VS Code）装有 Red Hat YAML 扩展时打开仓库，重复上一条的追加 document 操作"
    expected: "与 Zed 相同的补全与标红；`aviation-json/ontology.yaml` 无误报"
    why_human: "依赖 IDE 与扩展。`.vscode/settings.json` 与 Zed 配置经测试断言逐字相同，但 VS Code 侧未实机观察"
---

# Phase 1: Workspace, Schema & Lint 验证报告

**Phase Goal:** 开发者可以用 YAML/JSON 编写 ontology，并通过 `ontoforge validate` / `ontoforge lint` 得到带源码定位的诊断；仓库在 Windows 本机 `cargo test` 全绿
**Verified:** 2026-10-16
**Status:** human_needed（所有可自动化验证的项通过；G-1-2 在代码与配置层面关闭，剩 UAT Test 2 的真实编辑器复测）
**Re-verification:** 是，gap-closure plan 01-14 之后。上一份报告 4/4，人工项 3 个；其中 miette 渲染与 CI 双平台两项已在 UAT 中 pass（CI 引用 PR #1 run 38834599192），未再列入。

## 结论

阶段目标在代码库中达成。G-1-2 的根因（yaml-language-server 的 modeline 只作用于所在 document）已由 01-14 在三处同时处理：项目级 `yaml.schemas`（Zed 与 VS Code）、aviation 每个 document 的 modeline、两个回归测试。本次未依赖 SUMMARY 叙述，结论来自本次在主 checkout 上实际执行的命令，包括用与 Zed 相同版本的 yaml-language-server 做的 headless LSP 复现。

未发现 gap，未发现 debt marker（`TBD` / `FIXME` / `XXX` 在 CONTRIBUTING.md、`.zed`、`.vscode`、`schema_export.rs`、aviation 示例中零命中）。无 regression。

## 本次实测记录

| 项 | 命令 | 结果 |
|----|------|------|
| 工作区测试 | `cargo test --workspace --locked` | exit 0，35 个 `test result` 汇总行，**299 passed / 0 failed / 0 ignored**（上次 297，增加的正是 01-14 的 2 个新测试） |
| 新测试 | 日志检索 | `editor_configs_associate_definition_schema_with_examples ... ok`、`aviation_definition_documents_start_with_modeline ... ok` |
| validate YAML / JSON | `validate examples/ontologies/aviation`、`aviation-json` | 均 exit 0，`ok: ontology aviation with 5 object type(s)` |
| lint | `lint examples/ontologies/aviation --deny-warnings --format json` | `[]`，exit 0 |
| schema 导出一致性 | `schema export --out-dir <temp>` 后 `diff -r` 对 `schemas/` | 逐字节一致 |
| 快照 | `git status --porcelain` 于两个 snapshots 目录 | 空（insta 快照无变更） |
| 工作树 | `git status --porcelain` | 仅 `?? docs/`（与任务开始时相同，非本 phase 产物） |

clippy 与 fmt 本次未重跑；上一份报告已记录 exit 0，01-14 只改了一个测试文件（经 `cargo test` 编译通过），SUMMARY 称两者通过。此处如实标注为「未独立重跑」。

## Goal Achievement

### Observable Truths

来源：ROADMAP Success Criteria 1-5（合同），加 01-14 PLAN `must_haves.truths`（6 条，其中第 1 条为 G-1-2 的编辑器 truth）。

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC1：10 个 `ontoforge-*` crate、edition 2024、resolver 3、rust-version 1.98，Windows 本机无 Docker 执行 `cargo test --workspace` 全部通过 | VERIFIED | 本次 `cargo test --workspace --locked` exit 0，299 passed / 0 failed。crate 结构与 workspace 配置见上一份报告，未变化 |
| 2 | SC2：多文件 YAML 合并为单一 IR，YAML 与 JSON 写法同 IR | VERIFIED | 本次 `validate` 对 `aviation`、`aviation-json` 均 exit 0；`aviation_yaml_and_json_produce_identical_ir` 在全套测试中通过。01-14 加的 10 行 modeline 是注释，`git diff --numstat b56e2dd HEAD -- examples/ontologies` 为 `2 0` / `4 0` / `4 0`（仅新增，无删除），IR 不变 |
| 3 | SC3：定义有错时输出文件名与 line:col，退出码非 0 | VERIFIED | invalid 样例快照测试全部通过且快照目录无变更；上一份报告实测 27 个样例逐个 exit 1 并带 line:col，01-14 未触碰这些文件 |
| 4 | SC4：`lint` 运行 E / W 规则，两种输出格式，E 级退出码非 0，配置可关闭或调严重级 | VERIFIED | 本次 `lint aviation --deny-warnings --format json` 输出 `[]` exit 0；lint 规则测试在全套中通过。规则实测记录见上一份报告，未回退 |
| 5 | SC5：可导出 JSON Schema，示例通过该 Schema 校验 | VERIFIED | `schema export` 产物与入库 `schemas/` 逐字节一致；`aviation_documents_validate` 等 schema_export 测试通过 |
| 6 | G-1-2 核心 truth：aviation 的 objects / links / actions 任意 document（含文件末尾新追加的）输入 `kind:` 可补全 ObjectType / LinkType / ActionType，未知字段被标红 | VERIFIED（服务端层面）；真实编辑器待人工 | headless LSP 驱动 yaml-language-server 1.25.0（Zed 实装版本）加载提交的 `.zed/settings.json` 的 `yaml.schemas`：`objects_append_kind` 用例（UAT 复现场景）得到 3 项补全 `ObjectType \| LinkType \| ActionType` 与 5 条诊断；`single_key_narrow` 补全收窄为 9 个 ObjectType 字段。剥除所有 modeline、只留 settings 关联时结果相同，证明 settings 本身足以覆盖追加的 document。真实 Zed 的复测见 human_verification |
| 7 | `.zed/settings.json` 与 `.vscode/settings.json` 是严格 JSON，`yaml.schemas` 映射相同，键为 `./schemas/ontoforge-definition.schema.json`，第一个 glob 为 `examples/ontologies/**/*.yaml` | VERIFIED | 两文件内容已读。`jq -e` 对两份文件的映射断言均 `true`、rc=0；`keys == ["lsp"]` 与 `keys == ["yaml.schemas"]` 均 true；`git ls-files` 列出两文件，`git check-ignore` 均 rc=1（未被忽略）；`schemas/ontoforge-definition.schema.json` 存在 |
| 8 | 所有 `ontology.yaml` 不被关联到定义 schema（排除 glob），`aviation/ontology.yaml` 仍由自己的 manifest modeline 校验，无 modeline 的 manifest 不出现误报 | VERIFIED | 用规划期的 `probe.js`（真实文件名 `ontology.yaml`）在当前仓库副本上重跑：带排除项时 `aviation-json/ontology.yaml`（无 modeline）0 诊断、`other/ontology.yaml`（无 modeline，含 bogus 键）0 诊断、`aviation/ontology.yaml` 仍报 `bogusManifestKey is not allowed`（由 manifest schema）。去掉排除项的对照组：无 modeline 的 manifest 各出现 4 条定义 schema 误报。排除项有效，且 REVIEW 的 IN-02（globset 不等于 yaml-language-server 语义）被此实测覆盖 |
| 9 | aviation 每个 YAML document 第一行都是定义 modeline（objects 5、links 5、actions 3，共 13，新增 10），validate / lint --deny-warnings 仍 clean，YAML / JSON IR 等价 | VERIFIED | `rg -c` 得 modeline 5 / 5 / 3，`^---$` 为 4 / 4 / 2；每个 `---` 的下一行均为定义 modeline（rg -A1 输出核对）；`aviation/ontology.yaml` 第 1 行为 manifest modeline；validate / lint 结果见上表 |
| 10 | CONTRIBUTING.md 有「编辑器接入」一节，写明 modeline 只作用于所在 document（vscode-yaml#1066）、多文档靠项目 `yaml.schemas`、JetBrains 的 JSON Schema Mappings、`kind: ` 后需 Ctrl+Space | VERIFIED | `## 编辑器接入` 位于第 45 行（共 7 个 `## ` 节）；第 48 行含 issues/1066 链接，第 49 行含 `.zed/settings.json`、`.vscode/settings.json` 与排除 glob，第 52 行含 JSON Schema Mappings，第 53 行含 Ctrl+Space；引用的两个测试名在 `schema_export.rs` 中真实存在（220、332 行） |
| 11 | 回归测试守住配置与 modeline：配置改坏、schema 改名、glob 漏示例、新 document 漏 modeline 都使 `cargo test` 失败 | VERIFIED | 读了两个测试的全部源码。配置测试：`serde_json` 严格解析；`only_key` 逐层断言键集合恰为 lsp → yaml-language-server → settings → yaml → schemas，及 `yaml.schemas`（防可执行设置混入，T-01-42）；两份映射 `assert_eq`；每个键须以 `./schemas/` 开头、文件存在且在 `rendered_schemas()` 内（防远程 URL，T-01-43）；第一个 glob 恰为锁定值；walkdir 遍历 `examples/ontologies` 全部 `.yaml`，`ontology.yaml` 必须不被覆盖、其余必须被覆盖，并断言 aviation 三文件在覆盖集合内（防空遍历）。modeline 测试：首行、每个 `---` 的下一行、modeline 数等于 `yaml_documents().len()`、manifest 首行、两个相对路径可解析（T-01-44）。测试在全套中通过，断言有实质内容，非占位 |

**Score:** 10/11（truth 6 在服务端层面 VERIFIED，真实编辑器内行为按 Step 8 计为待人工，不计入 verified）。0 个 behavior-unverified。

注：上一份报告只列了 SC1-4，把 SC5 并入「SCHEMA-07」小节说明。本次按 ROADMAP 把 SC5 单列为 truth 5。

### Data-Flow / Wiring（01-14 范围）

| 链路 | 状态 | 说明 |
|------|------|------|
| `.zed/settings.json` → `schemas/ontoforge-definition.schema.json` | WIRED | 键相对工作区根；测试断言文件存在；headless LSP 实测解析成功 |
| `.vscode/settings.json` → 同上 | WIRED | 与 Zed 映射逐字相同（`assert_eq` 守住） |
| `schema_export.rs` 测试 → 两份配置、aviation 文件 | WIRED | 测试读取真实文件，walkdir 遍历真实目录 |
| aviation 每个 document 的 modeline → schema | WIRED | 相对路径 `../../../schemas/...` 从 `examples/ontologies/aviation` 解析到存在文件（测试断言） |
| 导出 schema → 与入库 `schemas/` | WIRED | `diff -r` 一致；`committed_schemas_are_up_to_date` 守住 |

### Requirements Coverage

PLAN frontmatter 的 requirement ID 并集：DX-03、SCHEMA-01..07、LINT-01..04，共 12 个。01-14 声明 `requirements: [SCHEMA-07]`。ROADMAP 声明同样 12 个，REQUIREMENTS.md Traceability 表中 Phase 1 的 12 项均为 Complete。无遗漏，无 orphan。

| Requirement | Source Plan | Status | Evidence |
|-------------|-------------|--------|----------|
| SCHEMA-01 | 01-01、01-03、01-06 | SATISFIED | 多文件加 include glob 合并，YAML / JSON 同 IR（本次 validate 与 IR 等价测试） |
| SCHEMA-02 | 01-04 | SATISFIED | object type 全部字段，invalid 样例快照通过 |
| SCHEMA-03 | 01-04 | SATISFIED | 13 个 base type 与 required |
| SCHEMA-04 | 01-05 | SATISFIED | 三种基数 |
| SCHEMA-05 | 01-06、01-07、01-12 | SATISFIED | 参数、rule、CEL submission criteria |
| SCHEMA-06 | 01-01、01-03、01-04、01-05、01-06、01-12、01-13 | SATISFIED | 全部 invalid 样例带 line:col |
| SCHEMA-07 | 01-09、**01-14** | SATISFIED（代码 / 配置层面）；编辑器内复测待人工 | 导出 schema 与入库一致；01-14 让多文档 YAML 的每个 document 在编辑器服务端关联到 schema，补全「用于编辑器补全」的承诺。UAT Test 2 未在真实 Zed 重跑，故该需求的编辑器用途以人工项收口 |
| LINT-01 | 01-08 | SATISFIED | E08 / E09 |
| LINT-02 | 01-10、01-11 | SATISFIED | W01..W11 |
| LINT-03 | 01-08、01-13 | SATISFIED | miette 与 JSON 两种格式；UAT Test 1 pass |
| LINT-04 | 01-08、01-13 | SATISFIED | `ontoforge.toml` `[lint]` |
| DX-03 | 01-01、01-02、01-11 | SATISFIED | 本次 `cargo test --workspace --locked` 299 passed，无 Docker |

### Required Artifacts（01-14）

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `.zed/settings.json` | Zed 项目级 `yaml.schemas` | VERIFIED | 存在、实质内容、入库、被测试读取 |
| `.vscode/settings.json` | VS Code 项目级 `yaml.schemas` | VERIFIED | 同上 |
| `crates/ontoforge-schema/tests/schema_export.rs` | 两个回归测试 | VERIFIED | 215 行新增，断言实质 |
| aviation `objects` / `links` / `actions.yaml` | 每 document 头部 modeline | VERIFIED | 5 / 5 / 3 |
| `CONTRIBUTING.md` | 「编辑器接入」一节 | VERIFIED | 6 条要点齐全，与配置一致 |

提交核对：`a839ac2`、`3908f7b`、`82ca319` 在 `git log` 中存在；`git diff --stat b56e2dd HEAD`（排除 `.planning`）为 7 个文件、259 行新增，与 PLAN 的 `files_modified` 完全一致，无计划外文件。未改 `Cargo.toml` / `Cargo.lock`。

### Anti-Patterns Found

无。01-14 范围内的 debt marker 零命中。REVIEW 的 IN-02 指出的脆弱性（`only_key` 要求每层只有一个键，日后在两份配置里加别的项目级设置会使测试失败）是 PLAN 里 T-01-42 的刻意取舍，CONTRIBUTING 已说明意图，不构成 gap。

### 关于 01-REVIEW.md（增量复审：0 critical / 7 warning / 3 info）

按任务要求视为 advisory，逐条核对是否破坏 must-have：

- WR-01（单文件超 1024 个 document 的消息映射）、WR-03（任意非 UTF-8 文件名变 V14）、WR-04（`Recursion limit of` 子串误判）、WR-06（V07 建议名可能是 CEL 关键字）：规则边角与健壮性问题，不影响 SC1-5 与 12 个 requirement 的验收路径。
- WR-02、WR-07、IN-03：性能与输出量放大（W08 输出、渲染开销、W07 两两比较），输入需构造到数十万类型或数 MB，不在 v1 示例范围，不影响 must-have。
- WR-05：`sanitize` 保留 `\n`，Linux 文件名含换行时可伪造报告头。这是 CR-04 修复的残余，属于安全加固，不破坏 Windows 本机验收；建议后续 phase 或 quick 任务处理。
- IN-01：symlink / 非 UTF-8 测试在环境不支持时静默通过；CI 已在 ubuntu-latest 实跑通过（UAT Test 3）。
- IN-02：已由本次 headless 实测补足（见 truth 8）。

结论：无一条使 must-have 失败。7 个 warning 建议在合适的时机登记为技术债，但不阻塞 Phase 1。

### Deferred Items

无需要登记的 deferred gap。W06 空值率检测属 CONTEXT 已登记的有意推迟。

### Probe Execution

SKIPPED：PLAN 与 SUMMARY 没有声明 `probe-*.sh`，仓库中也没有 `scripts/*/tests/probe-*.sh`。

### Behavioral Spot-Checks（含 headless LSP）

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| 工作区测试 | `cargo test --workspace --locked` | 299 passed / 0 failed | PASS |
| validate / lint 示例 | `ontoforge validate`、`lint --deny-warnings --format json` | exit 0，`[]` | PASS |
| schema 导出一致 | `schema export` 后 `diff -r` | 无差异 | PASS |
| UAT 复现场景 | `drive.js`（yaml-language-server 1.25.0，settings 取自提交的 `.zed/settings.json`）`append_kind` | 3 项补全、5 条诊断 | PASS |
| 追加 document 的未知字段 | `append_unknown` | `Property primaryKy is not allowed.` | PASS |
| 既有 document 的未知字段 | `doc1_doc2_unknown`（剥除 modeline 后同样） | bogusDoc1、bogusDoc2 均被报 | PASS |
| 按 kind 收窄 | `single_key_narrow` | 9 个 ObjectType 字段 | PASS |
| manifest 排除 | `probe.js`，带 / 不带排除项对照 | 带：无 modeline 的 manifest 0 误报；不带：各 4 条误报 | PASS |

说明：LSP 驱动脚本在 `C:/tmp/of-yamlls`、`C:/tmp/of-planner-probe`（仓库外，不属于 phase 产物）。第一次用 `drive.js` 测排除项时，该脚本把用例写到 `case_*.yaml` 而不是 `ontology.yaml`，排除 glob 不会命中，得到的「仍有误报」是测法错误，不是配置缺陷；改用按真实文件名建用例的 `probe.js` 后结果正确，上表采用后者。服务端 `completionProvider` 无 `triggerCharacters`，所以 `kind: ` 后需要 Ctrl+Space，CONTRIBUTING 已写明。

## Human Verification Required

### 1. UAT Test 2 复测：Zed 中的 YAML 补全与标红

**Test:** 在 Zed 中打开仓库根目录（已打开则执行 `editor: restart language server`），打开 `examples/ontologies/aviation/objects.yaml`，在末尾追加 `---` 和 `kind: `，按 Ctrl+Space；再在该 document 中加 `bogus: 1`；最后打开 `examples/ontologies/aviation-json/ontology.yaml`。
**Expected:** 补全列出 ObjectType、LinkType、ActionType；`bogus` 被标红；`aviation-json/ontology.yaml` 无 schema 错误。选 ObjectType 后新行 Ctrl+Space 只出现 ObjectType 字段。
**Why human:** 补全弹窗与编辑器内诊断只能在真实编辑器观察；Zed 对 `.zed/settings.json` 的实际加载与相对路径解析无法在命令行验证。通过后把 01-UAT.md Test 2 改为 pass，并归档 `.planning/debug/g-1-2-yaml-schema-completion.md`（01-14 明确不处理归档）。

### 2. （可选）VS Code 同样复测

**Test:** 装有 Red Hat YAML 扩展，重复上述操作。
**Expected:** 同 Zed。
**Why human:** 依赖 IDE 与扩展。

## Gaps Summary

无 gap。G-1-2 要求的四项缺失（项目级 `yaml.schemas` 配置、每个 `---` 后的 modeline、CONTRIBUTING 说明、回归测试）全部在代码库中存在且被测试守住；12 个 requirement ID 全部有证据；ROADMAP 5 条 Success Criteria 全部通过；全套 299 个测试通过。仅剩真实 Zed 中的 UAT Test 2 复测，因此状态为 `human_needed` 而非 `passed`。

簿记提醒（不属于 gap，由 orchestrator 处理）：`.planning/debug/g-1-2-yaml-schema-completion.md` 的 status 仍为 diagnosed，待复测通过后归档；01-UAT.md 的 Test 2 与 G-1-2 状态待复测后更新。

---

_Verified: 2026-10-16_
_Verifier: Claude (gsd-verifier)_
