---
phase: 01-workspace-schema-lint
verified: 2026-10-16T00:00:00Z
status: human_needed
score: 4/4 must-haves verified
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
  - "Cargo.toml"
  - "crates/ontoforge-schema/src/lib.rs"
  - "crates/ontoforge/src/main.rs"
  - "schemas/ontoforge-definition.schema.json"
  - "schemas/ontoforge-manifest.schema.json"
covered_digest: "v1:sha256:b322fa0dac6491995b1c47ce7d46838fd5674cdb688d4bc33a3ef8ffc5784012"
behavior_unverified: 0
overrides_applied: 0
human_verification:
  - test: "在 Windows Terminal 中运行 `ontoforge validate examples/ontologies/invalid/v13-unknown-parameter` 与 `ontoforge lint`（不带 --color never）"
    expected: "miette fancy 渲染的颜色、框线字符、源码高亮可读，无乱码"
    why_human: "终端渲染的可读性是视觉判断；自动化只验证了 `--color never` 与 JSON 输出的内容"
  - test: "在 VS Code（安装 Red Hat YAML 扩展）中打开 `examples/ontologies/aviation/objects.yaml`"
    expected: "文件首行 `# yaml-language-server: $schema=...` 指向 `schemas/ontoforge-definition.schema.json`，键名补全与未知字段提示生效"
    why_human: "编辑器补全依赖 IDE 与扩展，无法在命令行验证；schema 本身已验证与 `ontoforge schema export` 产物逐字节一致，且 aviation 文档通过 schema 校验"
  - test: "把分支推到 GitHub，观察 `.github/workflows/ci.yml` 在 windows-latest 与 ubuntu-latest 上的运行"
    expected: "fmt / clippy / test 三步在两个平台均通过（action 已按 SHA 固定，`--locked`）"
    why_human: "分支尚未推送；Linux 平台上的行为（符号链接、非 UTF-8 文件名测试走 Unix 分支）本机无法验证"
---

# Phase 1: Workspace, Schema & Lint 验证报告

**Phase Goal:** 开发者可以用 YAML/JSON 编写 ontology，并通过 `ontoforge validate` / `ontoforge lint` 得到带源码定位的诊断；仓库在 Windows 本机 `cargo test` 全绿
**Verified:** 2026-10-16
**Status:** human_needed（所有自动化可验证项通过，仅剩 3 个已知的人工项）
**Re-verification:** 否，首次验证

## 结论

阶段目标在代码库中达成。验证没有依赖 SUMMARY 的叙述，全部结论来自本次在主 checkout 上实际运行的命令。未发现 gap，未发现 debt marker（`TBD` / `FIXME` / `XXX` / `todo!` / `unimplemented!` 在 `crates`、`examples`、`.github`、`schemas`、`Cargo.toml` 中均无命中）。

## Goal Achievement

### Observable Truths（来自 ROADMAP Success Criteria）

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | cargo workspace 按 ADR-0011 建好 10 个 `ontoforge-*` crate（edition 2024、resolver 3、rust-version 1.98），Windows 本机不依赖 Docker 执行 `cargo test --workspace` 全部通过 | VERIFIED | `crates/` 下 10 个目录（`ontoforge`、`-action`、`-api`、`-auth`、`-client`、`-expr`、`-query`、`-schema`、`-server`、`-storage`）；根 `Cargo.toml`：`resolver = "3"`、`edition = "2024"`、`rust-version = "1.98"`、`unsafe_code = "forbid"`、clippy pedantic。`cargo test --workspace` exit 0，汇总 **297 passed, 0 failed, 0 ignored**。`cargo clippy --workspace --all-targets -- -D warnings` exit 0；`cargo fmt --all --check` exit 0。仓库内无 Docker 依赖。path 依赖方向与 ADR-0011 一致（api 依赖 schema + query，client 只依赖 api，server 依赖 action/auth/api/query，CLI 依赖 schema + expr） |
| 2 | `ontoforge validate examples/ontologies/<示例>/` 把多文件的 object type / link type（1:1、1:N、M:N）/ action type 合并为单一 IR；YAML 与 JSON 写法得到相同 IR | VERIFIED | `validate examples/ontologies/aviation` 与 `aviation-json` 均 exit 0，输出 `ok: ontology aviation with 5 object type(s)`。`links.yaml` 含 oneToMany（3 条）、oneToOne（`aircraftCurrentMaintenance`）、manyToMany（`flightCrew`）。`actions.yaml` 含 `assignPilot`（addLink + CEL）、`delayFlight`（modifyObject + CEL）、`recordMaintenance`（createObject + currentUser / currentTime）。测试 `aviation_yaml_and_json_produce_identical_ir` 断言两份 IR `assert_eq!`，5 object / 5 link / 3 action；`aviation_file_order_does_not_matter` 通过 |
| 3 | 定义有错时，CLI 输出带文件名与 line:col 的诊断，退出码非 0 | VERIFIED | `examples/ontologies/invalid/` 下 27 个样例逐个运行 `validate`，全部 exit 1，每个都产出 JSON 诊断，含 `code` / `file` / `line` / `column`（例：v04 `objects.yaml:3:1`、v09 `defs.yaml:19:19` 带 `did you mean Flight?`、v08 `b.yaml:2:10` 并指向首次定义 `a.yaml:2:10`、v13 在 CEL 字符串内指向条件、v01 YAML 重复键与 JSON 尾逗号均有定位）。人类可读输出是 miette 渲染的源码片段加 help；`--format json` 输出 `{code,severity,message,file,line,column,span{offset,length},help}` 数组（D-10） |
| 4 | `ontoforge lint` 运行 ONT-E 规则与 ONT-W01–W11，人类可读与 JSON 两种格式；E 级问题退出码非 0；配置可按规则 ID 关闭或调整严重级 | VERIFIED | `lint/` 下有 `e08.rs`、`e09.rs`、`naming/w01..w05.rs`、`structure/w06..w11.rs`，lib 测试按规则枚举到每条规则至少 2 个测试（E08 4、E09 5、W01–W11 各 2–6）。实测：`lint aviation --deny-warnings --format json` 输出 `[]` exit 0；构造含复数名与 `dtCreated` 的 ontology，得到 ONT-W01、ONT-W03 带 `objects.yaml:70:10` / `74:3` 定位，warning 时 exit 0，`--deny-warnings` 时 exit 1；构造对 oneToMany 做 addLink 的 action，得到 ONT-E08 error，exit 1，help 给出替代写法；在 `ontoforge.toml` 设 `"ONT-W01" = "off"`、`"ONT-W03" = "error"` 后，W01 消失、W03 变 error、exit 1 |

**Score:** 4/4 truths verified，0 个 behavior-unverified。

### SCHEMA-07 / JSON Schema 导出

`ontoforge schema export --out-dir <仓库外临时目录>` exit 0，生成 `ontoforge-definition.schema.json` 与 `ontoforge-manifest.schema.json`；`diff -r` 对比入库的 `schemas/`：**逐字节一致**。`schema_export` 测试覆盖 draft 2020-12、`kind` 的 `oneOf` / const 判别、rules 的 kind 判别、入库快照是否最新、渲染确定性、aviation 文档通过 schema 校验。

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/*`（10 个） | ADR-0011 workspace | VERIFIED | 骨架 crate 各有占位测试，全部通过；`ontoforge-schema`、`ontoforge-expr`、`ontoforge` 为实质实现 |
| `examples/ontologies/aviation` + `aviation-json` | D-16 示例，YAML / JSON 等价 | VERIFIED | 5 个 object type（Airport、Aircraft、Flight、Pilot、MaintenanceRecord）、5 个 link、3 个 action；超出 D-16 的最低要求 |
| `examples/ontologies/invalid/*` | D-16 诊断 fixture | VERIFIED | 27 个样例，覆盖 ONT-V01–V14 |
| `schemas/*.schema.json` | SCHEMA-07 入库产物 | VERIFIED | 与 `schema export` 产物一致 |
| `.github/workflows/ci.yml` | D-18 双平台 CI | VERIFIED（文件层面） | matrix windows-latest + ubuntu-latest，fmt / clippy / test，`permissions: contents: read`，三个 action 固定到完整 SHA，clippy / test 带 `--locked`；实际运行见 human_verification |
| `.gitattributes` | D-17 | VERIFIED | `* text=auto eol=lf` 及按扩展名的 eol=lf |

### Key Link Verification

| From | To | Via | Status |
|------|----|-----|--------|
| CLI `validate` / `lint` | `ontoforge-schema` loader + validate + lint | 直接调用并按 Diagnostic 渲染 | WIRED（实测输出） |
| submission criteria | `ontoforge-expr` | 在 validate 内编译、引用检查、类型推断，错误成为 ONT-V13 并回映到 YAML 字符串位置 | WIRED（v13-* 五个样例实测） |
| lint 配置 `ontoforge.toml` | lint 规则严重级 | 实测 off / error 生效 | WIRED |
| `schemars` 导出 | `schemas/` | `schema export`，测试守住快照 | WIRED |

### Requirements Coverage

PLAN frontmatter 中出现的 requirement ID 并集：DX-03、SCHEMA-01、SCHEMA-02、SCHEMA-03、SCHEMA-04、SCHEMA-05、SCHEMA-06、SCHEMA-07、LINT-01、LINT-02、LINT-03、LINT-04，共 12 个，与 ROADMAP 声明及 REQUIREMENTS.md 的 Traceability 表（Phase 1 共 12 项，均为 Complete）完全一致。无遗漏，无 orphan。

| Requirement | Source Plan | Status | Evidence |
|-------------|-------------|--------|----------|
| SCHEMA-01 | 01-01、01-03、01-06 | SATISFIED | 多文件 + include glob 合并为单一 IR；YAML 与 JSON 同 IR |
| SCHEMA-02 | 01-04 | SATISFIED | object type 的 apiName / primaryKey / titleKey / displayName / description / aliases / hidden；V05、V07、V10 样例 |
| SCHEMA-03 | 01-04 | SATISFIED | 13 个 v1 base type 与 required；V06 样例给近似名 |
| SCHEMA-04 | 01-05 | SATISFIED | 三种基数，两端各有 apiName；V08、V09、V11 样例 |
| SCHEMA-05 | 01-06、01-07、01-12 | SATISFIED | 参数、五种 rule、CEL submission criteria；V03、V09、V12、V13 样例 |
| SCHEMA-06 | 01-01、01-03、01-04、01-05、01-06、01-12、01-13 | SATISFIED | 全部 27 个 invalid 样例带文件名与 line:col |
| SCHEMA-07 | 01-09 | SATISFIED | schema export 与入库一致 |
| LINT-01 | 01-08 | SATISFIED | E08 / E09，error 时 exit 1 |
| LINT-02 | 01-10、01-11 | SATISFIED | W01–W11 模块与测试齐全，W01 / W03 实测 |
| LINT-03 | 01-08、01-13 | SATISFIED | miette 与 JSON 两种格式 |
| LINT-04 | 01-08、01-13 | SATISFIED | `ontoforge.toml` `[lint]` 实测生效；`--config` 存在 |
| DX-03 | 01-01、01-02、01-11 | SATISFIED | Windows 本机 `cargo test --workspace` 297 passed，无 Docker |

### CONTEXT 决策 D-01..D-18

| 决策 | 结论 | 依据 |
|------|------|------|
| D-01 manifest + include glob | 已遵守 | `ontology.yaml` manifest；V14（include 越界 `v14-include-escapes-root`）；loader 测试 34 条 |
| D-02 `kind` 判别、多文档 | 已遵守 | V02、V03（`v03-reserved-kind`）样例；ADR-0012；schema `oneOf` 测试 |
| D-03 属性内联、link 独立 | 已遵守 | aviation 示例结构 |
| D-04 命名规则 | 已遵守 | V07、V08；保留字由 `RESERVED_API_NAMES` 补齐 |
| D-05 未知字段报错加建议 | 已遵守 | v04：`did you mean primaryKey?` |
| D-06 validate 与 lint 分层 | 已遵守 | lint 只在 validate 无 error 后运行；V 码不可配置 |
| D-07 / D-08 / D-09 action 形状与 CEL 上下文 | 已遵守 | aviation 的 3 个 action；`params` / `user` / `objects` 固定上下文，V13 引用检查 |
| D-10 诊断模型与退出码 | 已遵守 | 字段齐全；error exit 1；`--deny-warnings` exit 1 |
| D-11 节点级 span | 已遵守 | v04 指向 key，v09 指向 value 节点，JSON 同样有 line:col |
| D-12 E08 / E09 | 已遵守 | 只实现这两条；E08 实测 |
| D-13 W01–W11 静态部分 | 已遵守 | W06 无空值率分支；W07 基于属性名与类型集合 |
| D-14 `ontoforge.toml` 与 `--config` | 已遵守 | 实测 |
| D-15 每规则一模块与 fixture 快照 | 已遵守 | 每规则一文件，`insta` 快照 |
| D-16 示例 | 已遵守 | 见 Artifacts |
| D-17 workspace 与 lint 与 `.gitattributes` | 已遵守 | 见 Truth 1 |
| D-18 CI | 文件已落地 | 实际运行待推送（human_verification） |

### ADR 符合性

- **ADR-0012**：文档形状、LinkType 端 apiName 语义（端 apiName 是该端 objectType 上的访问器）、值来源单键 map、`kind` 重写后反序列化，实现与示例一致；ADR-0003 的 Spike 节已替换为 S1 结论并指向 ADR-0012。
- **ADR-0013**：主键必须显式 `required: true`，`v10-primary-key-not-required` 实测 ONT-V10 于 `objects.yaml:5:3`；ADR-0012 正文已注明被取代。
- **ADR-0014**：参数与属性 apiName 不得为 CEL 保留字，`CEL_RESERVED_WORDS` 共 21 个词，用于 `ParameterName` 与 `PropertyName`（`ir/names.rs`），报 ONT-V07；ADR 状态 accepted，已回答 REVIEW-FIX 提出的补 ADR 建议。

### 代码审查修复（01-REVIEW-FIX.md）逐项核对

16 项均有独立 commit 且代码中可找到对应实现，全套测试 297 通过：

| 项 | 代码证据 |
|----|----------|
| CR-01 W08 | `w08.rs` 中 `MAX_COMBINATIONS = 500` 与按组合去重；commits `afa6a4d`、`2c40911` |
| CR-02 W07 | `w07.rs` 簇合并（cluster）；commits `ac7dd19`、`75bb116` |
| CR-03 符号链接 | `loader/mod.rs::read_limited`（有界读取），manifest 与根 `ontoforge.toml` 不跟随链接 |
| CR-04 文件名过滤 | `render.rs::sanitize` 用于 NamedSource 与 code |
| WR-01 | `ontoforge-expr/src/infer.rs` int / double 混合算术报 V13 |
| WR-02 | `walk.rs` `MemberPath.dynamic` 与字面量下标检查 |
| WR-03 | `ir/names.rs::CEL_RESERVED_WORDS` |
| WR-04 | `ir/basetype.rs::widens_to` |
| WR-05 | `lint/mod.rs::OptionKind` |
| WR-06 | `discover.rs` 忽略 ASCII 大小写比较 manifest 与 `ontoforge.toml` |
| WR-07 | `MAX_FILES = 10_000`、`MAX_TOTAL_BYTES = 64 MiB` |
| WR-08 | `ExprError::ParserLimit` 及解析器消息映射 |
| WR-09 | `e09.rs` 遍历 action 参数的嵌套 array |
| WR-10 | `ci.yml` SHA 固定与 `--locked` |
| WR-11 | `sanitize` 处理孤立 CR 与双向控制字符 |
| WR-12 | `discover.rs` 非 UTF-8 名称报 V14 |

**说明**：REVIEW-FIX 将 CR-01、CR-02、WR-01、WR-02、WR-04 标为「语义变化，需人工确认」。这些变化均有对应回归测试（W08 / W07 的规模与超时测试、CEL 类型与引用测试、赋值拓宽测试）并通过，未发现与 CONTEXT 或 ROADMAP 冲突。W08 现在按两两交集定义共享集合，与 01-11-PLAN 中「3 个及以上类型共享 3 个及以上属性」的描述在审计字段类真实场景下等价，仅在「三方交集严格小于任何两两交集」的边角场景下少报；已写入模块文档。属于已知且已记录的规则语义调整，不构成 gap。

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| 工作区测试 | `cargo test --workspace` | exit 0，297 passed / 0 failed | PASS |
| clippy | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 | PASS |
| fmt | `cargo fmt --all --check` | exit 0 | PASS |
| validate YAML / JSON 示例 | `ontoforge validate examples/ontologies/aviation{,-json}` | 均 exit 0 | PASS |
| lint 示例 | `ontoforge lint examples/ontologies/aviation --deny-warnings --format json` | `[]`，exit 0 | PASS |
| invalid 样例 | 27 个目录逐个 `validate` | 全部 exit 1，均含 line:col | PASS |
| schema 导出 | `schema export --out-dir <temp>` 后 `diff -r` | 无差异 | PASS |
| lint W / E / 配置 | 临时副本构造 W01、W03、E08 与 toml 覆盖 | 严重级、定位、退出码均符合预期 | PASS |

### Probe Execution

SKIPPED：本 phase 的 PLAN 与 SUMMARY 没有声明 `probe-*.sh`，仓库中也没有 `scripts/*/tests/probe-*.sh`。

### Anti-Patterns Found

无。`TBD` / `FIXME` / `XXX` / `todo!` / `unimplemented!` 零命中。其余 7 个 crate 是 CONTEXT 明确允许的骨架（能编译、有占位测试），不属于 stub 缺陷。

### Deferred Items

无需要登记的 deferred gap。W06 空值率检测已在 CONTEXT 的 Deferred 节登记，属于有意推迟，不是缺陷。

## Human Verification Required

### 1. miette 彩色输出在 Windows Terminal 中的可读性

**Test:** 在 Windows Terminal 中运行 `ontoforge validate examples/ontologies/invalid/v13-unknown-parameter` 与对带 warning 的 ontology 运行 `ontoforge lint`，不加 `--color never`。
**Expected:** 颜色、框线字符、源码高亮可读，无乱码。
**Why human:** 视觉判断，自动化只验证了无色与 JSON 内容。

### 2. VS Code YAML 补全

**Test:** 在装有 Red Hat YAML 扩展的 VS Code 中打开 `examples/ontologies/aviation/objects.yaml`。
**Expected:** 首行 modeline 指向 `schemas/ontoforge-definition.schema.json`，键名补全与未知字段提示生效。
**Why human:** 依赖 IDE 与扩展。schema 内容已验证与导出产物一致，aviation 文档通过 schema 校验。

### 3. GitHub Actions 双平台运行

**Test:** 推送分支，观察 `ci.yml` 在 `windows-latest` 与 `ubuntu-latest` 上的结果。
**Expected:** fmt / clippy / test 全绿。
**Why human:** 分支未推送；Unix 路径（符号链接与非 UTF-8 文件名测试的 Unix 分支）本机无法执行。

## Gaps Summary

无 gap。12 个 requirement ID 全部有实现与测试证据，ROADMAP 4 条 Success Criteria 全部实测通过，D-01..D-18 与 ADR-0012/0013/0014 均已遵守，CR-01..04 与 WR-01..12 的修复均在代码中。

簿记提醒（不属于 gap，由 orchestrator 处理）：`.planning/ROADMAP.md` 中 Phase 1 的复选框仍为未勾选，`.planning/STATE.md` 仍显示 `status: executing` / `completed_phases: 0`，REQUIREMENTS.md 的 Phase 1 条目已标 Complete。

---

_Verified: 2026-10-16_
_Verifier: Claude (gsd-verifier)_
