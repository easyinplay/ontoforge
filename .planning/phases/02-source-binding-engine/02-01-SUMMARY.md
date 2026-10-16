---
phase: 02-source-binding-engine
plan: 01
subsystem: schema
tags: [backing, ir, ontology-as-code, fixture-migration, adr, profile-name, read-limited, diagnostics]

requires:
  - phase: 01-workspace-schema-lint
    provides: loader / dispatch / SourceMap、IR 与 validate 管线、invalid 样例与 insta 快照基线
provides:
  - "Backing 枚举（source | editOnly）；ObjectType.backing 必填，LinkType.backing 对 manyToMany 必填、其他 cardinality 禁止"
  - "Phase 1 全部定义 fixture、内联测试定义与 27 个 invalid 样例迁移完成，快照语义零漂移"
  - "ADR-0021（绑定文件、backing、profile、decimal / timezone 规则、诊断码与退出码分配、engine crate）"
  - "ontoforge_schema::loader::{read_limited(path, follow_links, max_bytes), ReadFailure} 公开"
  - "ontoforge_schema::ir::ProfileName 与 diag::codes::{V16..V20}"
affects: [02-03, 02-04, 02-05, 02-06, 02-07, 02-08, 02-09, 02-10, phase-03, phase-07]

plan_head_before: d1b3c1c7a09dbead8e4cd51bf2dd7197b5dcdfe9

actuals:
  tokens: 24900
  tasks: 3
  commits: 5

tech-stack:
  added: []
  patterns:
    - "必填字段不加 serde default，缺失由 dispatch 映射为 ONT-V05"
    - "跨字段 / cardinality 相关的字段规则放在 validate::links，按 Part::Key / Part::Value 定位"
    - "测试里给 GOOD 常量追加属性行统一用 good_with(...)，backing 行固定在文档末尾以保持既有行号"

key-files:
  created:
    - crates/ontoforge-schema/src/ir/binding.rs
    - .planning/adr/0021-source-binding-files-and-backing.md
    - examples/ontologies/invalid/v05-missing-backing/
    - examples/ontologies/invalid/v11-backing-on-one-to-many/
    - crates/ontoforge/tests/snapshots/invalid_examples__v05-missing-backing.snap
    - crates/ontoforge/tests/snapshots/invalid_examples__v11-backing-on-one-to-many.snap
  modified:
    - crates/ontoforge-schema/src/ir/object.rs
    - crates/ontoforge-schema/src/ir/link.rs
    - crates/ontoforge-schema/src/ir/mod.rs
    - crates/ontoforge-schema/src/ir/names.rs
    - crates/ontoforge-schema/src/validate/links.rs
    - crates/ontoforge-schema/src/loader/mod.rs
    - crates/ontoforge-schema/src/lint/config.rs
    - crates/ontoforge-schema/src/diag.rs
    - schemas/ontoforge-definition.schema.json
    - .planning/adr/0015-source-binding-and-type-contract.md
    - .planning/REQUIREMENTS.md
    - .planning/ROADMAP.md

key-decisions:
  - "backing 必填且无默认；缺失一律 ONT-V05，link 上多写为 ONT-V11（ADR-0021，ADR-0012 向后兼容惯例的有意例外）"
  - "links::check 在 end_types 之前先做 backing 检查：该检查不依赖两端 object type，缺失类型时也一并报告"
  - "fixture 迁移统一写 editOnly，aviation 例外（Airport editOnly，其余 object type 与 flightCrew 为 source）"
  - "read_limited 的仓库内容读取固定 follow_links=false，只有部署方给出的路径可跟随（T-02-48）"

patterns-established:
  - "迁移脚本放仓库外临时目录、以参数接收仓库路径、跑完删除，规则原文记入 SUMMARY"
  - "快照漂移检查覆盖 JSON 快照的 code / severity / message / help 与 human 快照的码行、× 行、help: 行"

requirements-completed: [BIND-01]

coverage:
  - id: D1
    description: "ObjectType 与 manyToMany LinkType 的 backing 声明贯通 IR、校验与 JSON Schema（缺失 V05、非法值 V06、非 M:N link 多写 V11）"
    requirement: "BIND-01"
    verification:
      - kind: unit
        ref: "crates/ontoforge-schema/tests/object_types.rs#object_without_backing_is_v05"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-schema/tests/object_types.rs#unknown_backing_value_is_v06"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-schema/tests/link_types.rs#many_to_many_without_backing_is_v05"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-schema/tests/link_types.rs#one_to_many_with_backing_is_v11"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/invalid_examples.rs（v05-missing-backing、v11-backing-on-one-to-many 快照）"
        status: pass
    human_judgment: false
  - id: D2
    description: "Phase 1 全部 fixture（27 个 invalid 样例、aviation 两份示例、内联测试定义）迁移，既有快照 code / severity / message / help 与 human 快照语义行零漂移"
    verification:
      - kind: other
        ref: "git diff -U0 HEAD -- '*.snap' | rg 的 code/severity/message/help/ONT-/×/help: 模式，无匹配"
        status: pass
      - kind: integration
        ref: "cargo test --workspace --locked（312 通过、0 失败）"
        status: pass
    human_judgment: false
  - id: D3
    description: "aviation 示例 backing 值：Airport editOnly，其余 4 个 object type 与 flightCrew 为 source；aviation 与 aviation-json 加载为相同 IR，validate 无诊断"
    verification:
      - kind: unit
        ref: "crates/ontoforge-schema/tests/object_types.rs#aviation_backing_values"
        status: pass
      - kind: integration
        ref: "cargo run -q -p ontoforge -- validate examples/ontologies/aviation --format json --color never（输出 []，退出码 0）"
        status: pass
    human_judgment: false
  - id: D4
    description: "ADR-0021 记录 13 个决策要点与 Rejected / Consequences；ADR-0015 状态行、REQUIREMENTS traceability 与 ROADMAP 成功标准 1 与之对齐"
    verification:
      - kind: other
        ref: "Task 2 的三条 rg verify 命令（全部退出码 0）"
        status: pass
    human_judgment: true
    rationale: "ADR 文字是否准确表达锁定决策需要人读；rg 只能证明关键字面存在"
  - id: D5
    description: "wave 2 共用的 schema 前置：公开的 read_limited(path, follow_links, max_bytes)、ProfileName（非法 → ONT-V16）、诊断码 V16..V20"
    requirement: "BIND-01"
    verification:
      - kind: unit
        ref: "crates/ontoforge-schema/src/loader/mod.rs#read_limited_custom_limit"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-schema/src/ir/names.rs#profile_name_rules"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-schema/src/ir/names.rs#profile_name_schema_has_pattern"
        status: pass
    human_judgment: false

duration: 35min
completed: 2026-10-17
status: complete
---

# Phase 2 Plan 01: backing 必填、fixture 迁移、ADR-0021 与 wave 2 共用前置 Summary

**ObjectType / manyToMany LinkType 的 `backing: source | editOnly` 必填贯通 IR、校验与 JSON Schema，Phase 1 的 27 个 invalid 样例与全部内联定义一次迁移且快照语义零漂移；同时落地 ADR-0021、公开的有界读取 `read_limited`、`ProfileName` 与诊断码 V16..V20。**

## Performance

- **Duration:** 约 35 min（启动时未记录起点，按首个提交时间与读取阶段估算）
- **Started:** 约 2026-10-16T18:10Z
- **Completed:** 2026-10-16T18:45Z（本机时间 2026-10-17 02:45 +08:00）
- **Tasks:** 3（Task 1 tracer、Task 2 auto、Task 3 auto/tdd）
- **Files modified:** 84（相对 `d1b3c1c`，含 23 个更新的快照与 2 个新快照）

## Accomplishments

- `Backing` 枚举与 `ObjectType.backing`（必填，无 serde default）、`LinkType.backing: Option<Backing>`；`validate::links::backing` 对 manyToMany 缺失报 V05（定位 `cardinality` 值，help `add \`backing: source\` or \`backing: editOnly\``），对 oneToOne / oneToMany 多写报 V11（定位 `backing` 键）。
- 全部既有 fixture 迁移，`ontoforge validate examples/ontologies/aviation` 输出 `[]`，aviation 与 aviation-json 仍为相同 IR。
- `schemas/ontoforge-definition.schema.json` 重新导出（+31 行，ObjectType required 含 backing，LinkType 含可选 backing）。
- ADR-0021 一次落盘 13 个决策要点；ADR-0015 追加 1 行状态；BIND-01 文字与 ENGINE-02 / BIND-03 / ENGINE-04 traceability、ROADMAP 成功标准 1 对齐（E6）。
- `loader::read_limited` / `ReadFailure` 公开并参数化 `max_bytes`；`ProfileName` 与 V16..V20 在 wave 1 就位，02-03 与 02-04 可并行共用。

## Task Commits

1. **Task 1: tracer（backing 端到端 + fixture 迁移）**
   - RED `f131989`（test）：6 个行为测试，目标测试均因断言失败（缺 backing 无 V05、`backing` 被报 V04 等）
   - GREEN `bcafe8d`（feat）：IR、links 校验、全部 fixture 迁移、快照更新、两个新 invalid 样例、schema 导出
2. **Task 2: ADR-0021 与文档对齐** - `79f69f5`（docs）
3. **Task 3: read_limited / ProfileName / V16..V20（tdd）**
   - RED `b199cfc`（test）：`read_limited_custom_limit`、`profile_name_rules`、`profile_name_schema_has_pattern`、`profile_name_deserializes_through_serde` 断言失败
   - GREEN `9feb588`（feat）

**Plan metadata:** 本文件所在的 `docs(02-01)` 提交。未做 REFACTOR 提交（无需要）。

## Tracer 反馈门

`workflow.human_verify_mode = end-of-phase` 且 Task 1 的 `<verify>` 只含 `<automated>`：Task 1 提交前重跑全部 verify（`cargo test --workspace --locked` 全绿、快照漂移 `rg` 无匹配、validate aviation 输出 `[]`、clippy 与 fmt 通过）后继续 Task 2、3。

## 迁移规则原文（Task 1 步骤 4，脚本在仓库外临时目录，已删除）

- **目标：** `git ls-files` 中位于 `examples/`、`crates/` 下，含 ObjectType 定义或 `cardinality: manyToMany` 的 LinkType 定义的文件（YAML 文档、JSON 对象、Rust 测试源里的内联 YAML / JSON 字符串），排除 `docs/` 与 `.planning/`。
- **YAML：** 把文件按只含 `---` 的行拆成文档；对每个 `kind: ObjectType` 文档、以及含 `cardinality: manyToMany` 的 `kind: LinkType` 文档，在最后一个非空行之后插入 `backing: editOnly`（顶格，与 `kind:` 同缩进）。放在文档末尾使快照里已有诊断的行号尽量不变。已含 `backing:` 的文档跳过。
- **JSON：** 对 2 空格缩进数组（`  {` … `  },?`）或顶格单对象（`{` … `}`）逐块处理，满足同样条件的块在最后一个成员后加逗号并追加成员 `"backing": "editOnly"`。
- **aviation 例外：** `aviation/objects.yaml` 与 `aviation-json/objects.json` 中 Airport 写 `editOnly`，Aircraft / Flight / Pilot / MaintenanceRecord 写 `source`；两份示例的 flightCrew 写 `source`。
- **Rust 内联：** 对多行字符串字面量用同一套“文档末尾追加”规则（第二个一次性脚本，按文档边界 `---` 或字面量结束行切分）；单行 `\n` 转义字面量、`GOOD` 常量与 `object()` / `objects()` 测试 helper 手工追加。迁移后用 `cargo test --workspace` 找出漏网项并补齐。
- **语法故意损坏的样例**只要插入后不改变原有错误即可（`v01-json-trailing-comma` 在 `properties` 后追加成员，V01 位置不变）。

## 27 个 invalid 样例目录的处理结果

| # | 目录 | 结果 |
|---|------|------|
| 1 | v01-json-trailing-comma | 已迁移（objects.json，V01 位置不变） |
| 2 | v01-yaml-duplicate-key | 已迁移 |
| 3 | v02-missing-kind | 无需迁移（文档无 `kind`，不含 ObjectType） |
| 4 | v03-function-rule | 已迁移 |
| 5 | v03-reserved-kind | 无需迁移（kind 为保留字，不含 ObjectType） |
| 6 | v04-unknown-field | 已迁移 |
| 7 | v05-missing-primary-key | 已迁移 |
| 8 | v06-unknown-base-type | 已迁移 |
| 9 | v07-bad-object-type-name | 已迁移 |
| 10 | v08-duplicate-object-type | 已迁移（a.yaml、b.yaml） |
| 11 | v08-link-end-clashes-property | 已迁移（2 个 ObjectType 文档） |
| 12 | v09-link-unknown-object-type | 已迁移（2 个 ObjectType 文档） |
| 13 | v09-rule-unknown-property | 已迁移 |
| 14 | v10-float-primary-key | 已迁移 |
| 15 | v10-primary-key-not-required | 已迁移 |
| 16 | v11-foreign-key-type-mismatch | 已迁移（2 个 ObjectType 文档） |
| 17 | v11-many-to-many-with-foreign-key | 已迁移（2 个 ObjectType + 1 个 manyToMany LinkType） |
| 18 | v12-create-missing-primary-key | 已迁移 |
| 19 | v12-current-time-on-string | 已迁移 |
| 20 | v12-modify-assigns-primary-key | 已迁移 |
| 21 | v12-value-source-type-mismatch | 已迁移 |
| 22 | v13-comparison-type-mismatch | 已迁移 |
| 23 | v13-non-boolean-condition | 已迁移 |
| 24 | v13-objects-not-object-reference | 已迁移 |
| 25 | v13-syntax-error | 已迁移 |
| 26 | v13-unknown-parameter | 已迁移 |
| 27 | v14-include-escapes-root | 无需迁移（只有 manifest） |

迁移后目录总数 29（27 + 新增 `v05-missing-backing`、`v11-backing-on-one-to-many`）。24 个目录迁移、3 个无需迁移、0 个跳过。

## 被跳过的文件（有意不加 backing）

这些字面量故意构造为在加载 `backing` 之前就出错，或不会被反序列化成 IR，加了反而改变测试意图：

- `crates/ontoforge-schema/tests/loader.rs`：第 340 行（`primaryKy` 未知字段）、391 行（深嵌套 YAML，V01）、422 / 434 / 532 / 542 行（JSON 语法或未知字段）、460 / 493 行（重复键，V01）。
- `crates/ontoforge-schema/tests/diagnostics.rs`：136 行（重复键）、151 行（JSON 尾逗号）、173 行（`code:` 空值，保持原错误位置）。
- `crates/ontoforge/tests/cli_lint.rs`：231 行（`validate_codes_cannot_be_turned_off_by_config`，要求 V05 存在；缺失字段变为 `backing` 但码不变）。
- `crates/ontoforge-schema/src/loader/sourcemap.rs`：224 行（只测位置，不反序列化成 IR）。
- `crates/ontoforge-schema/src/lint/structure/w11.rs`：该 helper 只生成 oneToMany link，不需要 backing。

## 快照审阅（E19）

`git diff -U0 HEAD -- '*.snap'` 共 23 个快照（8 个 lint、15 个 invalid 样例）变化，全部是 31 对 `line` / `offset`；对 code / severity / message / help / `ONT-` 码行 / `×` 行 / `help:` 行的 `rg` 无匹配。两个 human 快照（`cli_lint__e08_human.snap`、`cli__validate_unknown_field_human.snap`）零变化。lint 规则测试里 `e09` 的硬编码行号 `(9, 18)` 更新为 `(10, 20)`，`diagnostics.rs` 重复定义断言行号 `8` 更新为 `9`（仅数字）。

## Decisions Made

- 迁移统一写 `editOnly`：对不含绑定的 fixture 语义中立（无数据来源声明），aviation 按 orchestrator 已决设置 source。
- `backing` 检查放在 `links::check` 的 `end_types` 之前，并改写模块 doc；该检查不依赖两端 object type，缺类型时同时报告比二次修复更省一轮。
- 新增 invalid 样例不写 yaml-language-server modeline（见偏差 1）。
- `read_limited` 仓库内容读取固定 `follow_links=false`，doc 写明部署方路径才可跟随（T-02-48）。

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - 计划前提不成立] 新 invalid 样例未写 modeline**
- **Found during:** Task 1 步骤 5
- **Issue:** 计划要求“定义文件首行与每个 `---` 之后写定义 modeline（与既有 invalid 样例同样的相对路径）”，但仓库内 27 个既有 invalid 样例没有任何 `yaml-language-server` 行（`rg -c yaml-language-server examples/ontologies/invalid` 无匹配），也没有测试检查 invalid 目录的 modeline（`aviation_definition_documents_start_with_modeline` 只针对 aviation）。
- **Fix:** 沿用既有 invalid 样例的写法，不加 modeline。
- **Files modified:** `examples/ontologies/invalid/v05-missing-backing/`、`v11-backing-on-one-to-many/`
- **Verification:** `cargo test -p ontoforge --test invalid_examples` 通过，快照与预期码一致。
- **Committed in:** `bcafe8d`

**2. [Rule 3 - 阻塞] 测试里 `GOOD` 常量被追加属性行**
- **Found during:** Task 1 迁移后全量测试
- **Issue:** `diagnostics.rs` 与 `object_types.rs` 里多个测试用 `format!("{GOOD}  extra: ...")` 在 `GOOD` 之后追加属性行；`GOOD` 末尾加了 `backing: editOnly` 后追加行落在 `backing` 之后，成为非法 YAML。
- **Fix:** 两个文件新增 `good_with(properties)` helper，把属性行插到 `backing` 之前；`all_v1_base_types_parse` 把 `backing` 放到循环追加之后。我自己写的 RED 测试同步改成对 `GOOD` 做 `replace`。
- **Files modified:** `crates/ontoforge-schema/tests/diagnostics.rs`、`crates/ontoforge-schema/tests/object_types.rs`
- **Verification:** `cargo test -p ontoforge-schema` 全绿。
- **Committed in:** `bcafe8d`

**3. [Rule 3 - 阻塞] RED 证据格式**
- **Found during:** Task 1 与 Task 3 RED 阶段
- **Issue:** `gsd_run check tdd-red-evidence` 解析的是 node TAP 输出，cargo 的输出不是 TAP，无法产出 `RED_EVIDENCE_OK` 记录。
- **Fix:** 改为人工核对：RED 阶段每个目标测试都因断言失败而非编译错误或加载崩溃（Task 3 为让测试可编译先提交了签名 stub：`read_limited` 忽略 `max_bytes`、`ProfileName` 接受任何字符串，随后失败的是断言）。见下“TDD Gate Compliance”。
- **Verification:** RED 提交前 `cargo test` 输出的失败断言已逐条检查。

---

**Total deviations:** 3（1 个计划前提修正，2 个阻塞项处理）
**Impact on plan:** 无范围蔓延；偏差 1 只影响两个新样例文件的首行，偏差 2 只影响测试辅助写法。

## TDD Gate Compliance

- Task 1：RED `f131989`（`test(02-01)`）→ GREEN `bcafe8d`（`feat(02-01)`），RED 时 6 个行为测试中除 `unknown_backing_value_is_v06` 外都因断言失败（V04 代替 V05 / V06 / V11、缺字段无诊断、aviation 无 backing）；`both_backing_values_load` 与 `one_to_one_with_backing_is_v11` 为补充用例。
- Task 3：RED `b199cfc` → GREEN `9feb588`。
- 无 REFACTOR 提交。
- 局限：`read_limited_follows_links_only_when_asked` 是 `#[cfg(unix)]`，在本机 Windows 上不执行，需 Linux CI 覆盖。

## Issues Encountered

- Windows 上 `cargo test --workspace` 超过默认 2 分钟的工具超时，改用后台或更长超时运行，结果不受影响。
- 过程中残留的 `.snap.new` 文件（早先失败运行生成）已全部删除，最终 `fd` 检查无 `.snap.new`。

## Authentication Gates

None。

## Known Stubs

None。Task 3 RED 阶段的 `ProfileName` 与 `read_limited` stub 已在 GREEN 提交中替换为实现。

## Threat Flags

None。新增公开 `read_limited` 已按 T-02-48 在 doc 中约束仓库内容读取策略；本 plan 没有新增网络端点、认证路径或信任边界。

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- wave 1 完成。02-02 之后的 plan 可在全绿基线（`cargo test --workspace --locked` 312 通过、clippy 与 fmt 干净）上开工。
- 02-03 与 02-04（wave 2，并行）直接使用 `ProfileName`、`diag::codes::V16..V20` 与 `loader::read_limited`，不再修改 `diag.rs` 与有界读取。
- BIND-01 仍需 02-03 的 `SourceBinding` 绑定文件才算整体完成；本 plan 只交付 `backing` 声明与前置。
- 待后续覆盖：unix 符号链接测试需 Linux CI 实跑。

## Self-Check: PASSED

- 创建的文件（`ir/binding.rs`、两个新 invalid 样例目录、两个新快照、ADR-0021）均存在。
- 提交 `f131989`、`bcafe8d`、`79f69f5`、`b199cfc`、`9feb588` 均存在，`git log --grep=02-01` 返回 5 条。
- 最终门：`cargo test --workspace --locked`（312 通过、0 失败）、`cargo clippy --workspace --all-targets --locked -- -D warnings`、`cargo fmt --all --check` 全部通过。
- Task 1 acceptance、Task 2 三条 verify、Task 3 acceptance 逐条重跑通过；`git diff --diff-filter=D` 对 `d1b3c1c..HEAD` 无意外删除。

---
*Phase: 02-source-binding-engine*
*Completed: 2026-10-17*
