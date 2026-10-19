---
phase: 02-source-binding-engine
plan: 07
subsystem: schema
tags: [lint, source-binding, decimal, timezone, collation, geopoint, fixture, insta]

requires:
  - phase: 02-source-binding-engine
    provides: "02-03 的 SourceBinding IR、PropertyDef::effective_decimal、Ontology::bound_columns、loader::fixture_header / fixture_path、DefKind::SourceBinding 与 binding_def_name；02-01 的 MAX_FILE_BYTES / MAX_TOTAL_BYTES"
provides:
  - "lint 规则 ONT-E13（decimal 精度 / scale）、ONT-E14（timestamp 必须声明 timezone）、ONT-E15（collation 只用于 string）、ONT-E16（source 对象不得含 geopoint）、ONT-W17（fixture 表头缺绑定列）"
  - "lint::binding::rules() 注册接缝；KNOWN_RULE_IDS 扩到 18 个；LintCx.sources"
  - "SourceMap 的 fixture 表头旁路表（FixtureColumns = Result<Vec<String>, String>、set_fixture_columns、fixture_columns），加载期按总字节预算读取并记录表头或无法读取的原因"
affects: [02-08, 02-09, 02-10, phase-03, phase-04]

plan_head_before: d2b94a4886728cca5130b223023a131330c91243

actuals:
  tokens: 15100
  tasks: 3
  commits: 6

tech-stack:
  added: []
  patterns:
    - "绑定类 lint 规则放在 lint/binding/，共享 helper（all_bindings、binding_def、element_type、describe）在 binding/mod.rs，规则逐条加载 bound_columns 而不自己拼列名"
    - "加载期把 fixture 表头写入 SourceMap 旁路表：先用探测闭包得到大小做预算预检，再用 assemble 同一读取闭包读文本，用完即丢，只保留表头或原因"
    - "无法检查时报告而不是静默跳过：表头不可读由 ONT-W17 给出定位到 fixture.file 的 warning"

key-files:
  created:
    - crates/ontoforge-schema/src/lint/binding/mod.rs
    - crates/ontoforge-schema/src/lint/binding/e13.rs
    - crates/ontoforge-schema/src/lint/binding/e14.rs
    - crates/ontoforge-schema/src/lint/binding/e15.rs
    - crates/ontoforge-schema/src/lint/binding/e16.rs
    - crates/ontoforge-schema/src/lint/binding/w17.rs
    - crates/ontoforge-schema/src/lint/binding/snapshots/（11 个 .snap：e13 / e14 / e15 / e16 各 triggers + clean，w17 triggers + clean + unreadable）
    - crates/ontoforge/tests/snapshots/cli_lint__e14_human.snap
  modified:
    - crates/ontoforge-schema/src/lint/mod.rs
    - crates/ontoforge-schema/src/loader/mod.rs
    - crates/ontoforge-schema/src/loader/sourcemap.rs
    - crates/ontoforge/tests/cli_lint.rs

key-decisions:
  - "ONT-W17 复用同一规则覆盖「表头不可读」分支（ADR-0021 由 02-01 同步记录），不新开码"
  - "E13 对每个属性最多报一条：精度越界时不再比较 scale；只写 precision 且小于缺省 scale 时定位 precision 并提示显式写 scale，提示里的缺省值取自 effective_decimal 而非写死"
  - "E14 对缺 timezone 的 timestamp：绑定里已有该列条目时定位 columns.<p> 键，否则定位 target 键；非 timestamp 属性上的 timezone 定位在 timezone 键"
  - "fixture 表头在加载最后一步（校验无错误之后）才捕获，ontology 为 None 时不读 fixture"

patterns-established:
  - "新增 lint 规则组的清单：binding/mod.rs 的 rules()、lint/mod.rs 的 KNOWN_RULE_IDS 长度与 registry_ids_are_known_and_unique、triggers + clean 快照"
  - "fixture 字节预算：预检大小 -> 读取 -> 累计，同一路径只读一次（E23）"

requirements-completed: [BIND-02]

coverage:
  - id: D1
    description: "ONT-E14：每个 profile 的 source 绑定中，timestamp 属性（含元素为 timestamp 的 array 与 M:N link 端）缺 timezone 报 error 并定位到绑定文件；非 timestamp 属性上声明 timezone 也报 error；可按 ID 关闭"
    requirement: "BIND-02"
    verification:
      - kind: unit
        ref: "crates/ontoforge-schema/src/lint/binding/e14.rs#triggers"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-schema/src/lint/binding/e14.rs#e14_multi_profile"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_lint.rs#e14_human"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_lint.rs#e14_can_be_disabled"
        status: pass
    human_judgment: false
  - id: D2
    description: "ONT-E13 decimal 精度 1..=38 与 scale <= precision（缺省值取 effective_decimal，只写 precision: 5 时提示显式写 scale）；ONT-E15 collation 只用于 string 与 string 数组及对应 link 列；ONT-E16 backing: source 的 object type 含 geopoint 报 error；均可降级"
    requirement: "BIND-02"
    verification:
      - kind: unit
        ref: "crates/ontoforge-schema/src/lint/binding/e13.rs#triggers"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-schema/src/lint/binding/e13.rs#e13_can_be_downgraded"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-schema/src/lint/binding/e15.rs#triggers"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-schema/src/lint/binding/e16.rs#triggers"
        status: pass
    human_judgment: false
  - id: D3
    description: "ONT-W17：绑定带 fixture 时，bound_columns 推导的期望列不在 fixture 表头（大小写不敏感）报 warning；表头无法读取或解析时给出定位到 fixture.file 的 warning 而不是静默跳过；可按 ID 关闭"
    requirement: "BIND-02"
    verification:
      - kind: unit
        ref: "crates/ontoforge-schema/src/lint/binding/w17.rs#triggers"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-schema/src/lint/binding/w17.rs#w17_unreadable_header_warns"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-schema/src/lint/binding/w17.rs#w17_off"
        status: pass
    human_judgment: false
  - id: D4
    description: "加载期捕获 fixture 表头：CSV 取首行（去 BOM）、JSON 取键并集，非 UTF-8 / 结构错误记原因；fixture 字节与定义文件共用 MAX_TOTAL_BYTES，超出者不读取，同一文件只读一次"
    requirement: "BIND-02"
    verification:
      - kind: unit
        ref: "crates/ontoforge-schema/src/loader/mod.rs#fixture_header_is_captured"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-schema/src/loader/mod.rs#fixture_bytes_count_toward_budget"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-schema/src/loader/mod.rs#shared_fixture_is_counted_once"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-schema/src/loader/mod.rs#fixture_that_is_not_utf8_is_recorded_as_unreadable"
        status: pass
    human_judgment: false
  - id: D5
    description: "aviation 与 aviation-json 在默认配置与 --deny-warnings 下 lint 结果为空；KNOWN_RULE_IDS 含 ONT-E13..E16 与 ONT-W17 且与 registry 一致"
    requirement: "BIND-02"
    verification:
      - kind: integration
        ref: "crates/ontoforge-schema/tests/lint_examples.rs#aviation_lints_clean"
        status: pass
      - kind: integration
        ref: "cargo run -q -p ontoforge -- lint examples/ontologies/aviation --deny-warnings --format json --color never"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-schema/src/lint/mod.rs#registry_ids_are_known_and_unique"
        status: pass
    human_judgment: false

duration: 25min
completed: 2026-10-17
status: complete
---

# Phase 2 Plan 07: 绑定类型契约 lint（E13-E16、W17）Summary

**ONT-E13（decimal 精度 / scale）、ONT-E14（timestamp 必须声明 timezone，定位到绑定文件）、ONT-E15（collation 只用于 string）、ONT-E16（source 对象不得含 geopoint）与 ONT-W17（fixture 表头缺绑定列，表头不可读时也有定位的提示）全部以可配置的 lint 规则落地；加载期按 `MAX_TOTAL_BYTES` 总预算只读一次 fixture 并把表头写入 `SourceMap` 旁路表。**

## Performance

- **Duration:** 约 25 min
- **Started:** 2026-10-16T21:02:02Z
- **Completed:** 2026-10-16T21:27Z（实现与最终门；SUMMARY 提交在其后）
- **Tasks:** 3（Task 1 tracer、Task 2 auto/tdd、Task 3 auto/tdd）
- **Files modified:** 22（相对 `d2b94a4`，含 11 个 lint 快照与 1 个 CLI 快照）

## Accomplishments

- **ONT-E14（tracer）。** 遍历每个 profile 的绑定，用 `Ontology::bound_columns` 取列，元素类型为 timestamp 的列必须有 `timezone`；已有列条目时定位 `columns.<p>` 键，没有条目时定位 `target` 键；非 timestamp 列写了 `timezone` 定位在该键。manyToMany link 的 `from` / `to` 按对端主键类型处理。`ontoforge lint` 的 human 输出落在 `bindings/dev/flight.yaml:5:3`，`"ONT-E14" = "off"` 后退出码 0。
- **ONT-E13 / E15 / E16。** E13 对所有 object type 的 decimal（含 `items: decimal`）用 `effective_decimal()` 判断，不写任何缺省数字；E15 用同一份 `bound_columns` 判断对象属性和 link 端；E16 只看 `backing: source` 的 object type，数组元素为 geopoint 时定位 `items`。
- **加载期 fixture 表头。** `assemble` 在校验无错误之后，对每个不同 fixture 路径先用探测闭包得到大小做预算预检，再用与定义文件相同的读取闭包读入、调用 02-03 的 `fixture_header`，结果按 `{profile}/{target}` 写入 `SourceMap`；超预算者记 `fixture skipped: the ontology byte budget ...`，非 UTF-8 等读取失败记原因；文本用完即丢。
- **ONT-W17。** 期望列来自 `bound_columns`，小写比较；显式映射的缺列定位到 `columns.<key>.column` 值，同名默认的缺列定位到 `fixture.file`；表头为 `Err` 时只报一条 `cannot read the header of fixture ...`，不产生逐列 finding。
- **框架。** `LintCx` 新增 `pub sources: &SourceMap`，`KNOWN_RULE_IDS` 扩到 18（避开 foundry spec 已占用的 E10-E12 / W12-W16），`registry()` 合并 `binding::rules()`。

## Task Commits

1. **Task 1: tracer（ONT-E14）**
   - RED `4c2478e`（test）：`triggers`、`e14_multi_profile`、`e14_array_of_timestamps`、`e14_link_end_with_timestamp_key` 与 CLI `e14_human` 均因断言失败（空规则桩不产出 finding / 退出码 0 != 1）；`e14_can_be_disabled` 属回归保护，RED 时即通过；`clean` 因缺快照失败，GREEN 时补写
   - GREEN `6b84113`（feat）
2. **Task 2: E13 / E15 / E16（tdd）**
   - RED `3f51a0a`：`e13` triggers / downgrade、`e15` triggers / link_end、`e16` triggers 因 finding 数量断言失败，三个 `clean` 因缺快照失败
   - GREEN `605255f`
3. **Task 3: fixture 表头捕获与 ONT-W17（tdd）**
   - RED `30e9ae3`：4 个 loader 测试与 `w17` 的 triggers / unreadable / off 因断言失败（旁路表为空、规则桩无输出），`clean` 因缺快照失败；`w17_json_fixture_header_is_the_union_of_keys` 属回归保护，RED 时即通过
   - GREEN `c8a03bf`

**Plan metadata:** 本文件所在的 `docs(02-07)` 提交。无 REFACTOR 提交。

## Tracer 反馈门

`workflow.human_verify_mode = end-of-phase` 且 Task 1 的 `<verify>` 只含 `<automated>`：提交前重跑 `cargo test -p ontoforge-schema lint::`（76 通过）、`cargo test -p ontoforge-schema --test lint_examples`（1 通过）、`cargo test -p ontoforge --test cli_lint`（17 通过），无 `.snap.new` 残留，然后继续 Task 2、3。

## Decisions Made

见 frontmatter `key-decisions`。补充：`KNOWN_RULE_IDS` 按 E 在前、W 在后重排（E08 E09 E13..E16 W01..W11 W17），不依赖顺序的既有测试不受影响。

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - 计划条目与可达性不符] E13 中「precision 未写时定位 type」不可达**
- **Found during:** Task 2 实现
- **Issue:** 精度越界只能在写出 `precision` 时发生（缺省 38 在范围内），计划里「未写时定位 type」的分支永远走不到。
- **Fix:** 精度越界一律定位 `precision` 值，且一个属性最多一条 finding（精度越界时不再比较 scale，避免 `precision: 0` 同时触发缺省 scale 10 > 0 的重复报告）。
- **Files modified:** `crates/ontoforge-schema/src/lint/binding/e13.rs`
- **Committed in:** `605255f`

**2. [Rule 1 - 定位更精确] E16 对数组元素定位 `items` 而非 `type`**
- **Found during:** Task 2 实现
- **Issue:** 计划写「定位 `properties.<p>.type` 值」，但 `items: geopoint` 的数组里 `type` 是 `array`，真正的 geopoint 在 `items`。
- **Fix:** 标量定位 `type`，数组定位 `items`；快照与测试固定。
- **Files modified:** `crates/ontoforge-schema/src/lint/binding/e16.rs`
- **Committed in:** `605255f`

**3. [Rule 3 - 阻塞] RED 提交需要规则桩与存根才能编译测试**
- **Found during:** 每个 task 的 RED
- **Issue:** `lint_rule_on` 在规则未注册时直接 panic，测试无法在断言层面失败；`gsd_run check tdd-red-evidence` 解析 node TAP 输出，cargo 输出不是 TAP（与 02-01、02-02、02-03 相同）。
- **Fix:** RED 提交带空 `check` 的规则桩、`SourceMap` 旁路表的数据结构与 `LintCx.sources`，使测试因对计划行为的断言失败而不是编译错误或 panic 失败；人工核对每个目标测试的失败位置。
- **Files modified:** 各 task 的 RED 提交
- **Committed in:** `4c2478e`、`3f51a0a`、`30e9ae3`

---

**Total deviations:** 3 auto-fixed（2 个计划条目修正、1 个 RED 证据处理）
**Impact on plan:** 无范围蔓延；公开接口与 interfaces 一致，`FixtureColumns`、`set_fixture_columns`、`fixture_columns`、`LintCx.sources`、`KNOWN_RULE_IDS: [&str; 18]` 均按计划。

## TDD Gate Compliance

- Task 1：RED `4c2478e`（`test(02-07)`）-> GREEN `6b84113`（`feat(02-07)`）。
- Task 2：RED `3f51a0a` -> GREEN `605255f`。
- Task 3：RED `30e9ae3` -> GREEN `c8a03bf`。
- 无 REFACTOR 提交。
- RED 中的意外 GREEN（已调查）：`e14_can_be_disabled`（规则本来就不产出 finding，GREEN 后对真实实现有约束力）、`w17_json_fixture_header_is_the_union_of_keys`（桩不产出 finding，GREEN 后由 `fixture_header` 的键并集语义约束）；各 `clean` 快照测试在 RED 时因快照文件不存在而失败，GREEN 时写入并审阅。

## Issues Encountered

- bash 里把含 `\x27` / 大量引号的多段 heredoc 一次提交时，工具层的命令解析报 `unexpected EOF`，整条命令未执行；改用 Write 工具逐文件写入。一条以 `python -` 空 heredoc 起头的命令卡住（Windows 上 `python` 别名等待 stdin），已用 `taskkill /T` 终止进程树，终止后工作树无变更（`git status` 干净）。
- Windows 上 `cargo test --workspace` 需要较长超时，结果不受影响。

## Authentication Gates

None。

## Known Stubs

None。三次 RED 提交里的规则桩与存根全部在对应 GREEN 提交中替换。`.planning/WINDOWS.md` 没有新增条目。

## Threat Flags

None。T-02-28（可配置的 lint，结构性 V 错误不可配置）、T-02-29（单个 fixture 4 MiB 上限、fixture 与定义文件共用 `MAX_TOTAL_BYTES`、同一文件只读一次，`fixture_bytes_count_toward_budget` 与 `shared_fixture_is_counted_once` 覆盖）、T-02-49（表头不可读时 `w17_unreadable_header_warns` 与 `fixture_that_is_not_utf8_is_recorded_as_unreadable`）、T-02-30（诊断文本只含绑定里写的名字与结构性原因，`fixture_header` 的错误不回显数据，经 `render` 输出）均已落地。本 plan 没有新增网络端点、认证路径或信任边界。

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- 02-08（decimal 兼容规则）可直接使用 `effective_decimal` 与本 plan 的 E13 约束；`validate --online` 的 ONT-V17 仍负责真实表缺列。
- 待后续覆盖：JSON fixture 为空数组时 `fixture_header` 返回空表头，W17 会对每个绑定列报缺列；如这在实际使用中太吵，可在 02-10 的文档里说明或改为单条提示。
- 公开文档 `docs/source-binding.md` 尚未创建，计划在后续 plan 交付；本 plan 引入的 5 条规则编号需在那里列出。

## Self-Check: PASSED

- 创建的文件（`lint/binding/{mod,e13,e14,e15,e16,w17}.rs`、11 个 lint 快照、`cli_lint__e14_human.snap`）均存在于磁盘；`git diff --diff-filter=D d2b94a4..HEAD` 无删除。
- 提交 `4c2478e`、`6b84113`、`3f51a0a`、`605255f`、`30e9ae3`、`c8a03bf` 均存在，`git log --grep="(02-07)"` 返回 6 条，`git rev-list --count d2b94a4..HEAD` 为 6。
- Task 1 / 2 / 3 的 acceptance 逐条重跑通过：`rg` 正向检查（`"ONT-E1x"`、`KNOWN_RULE_IDS: [&str; 18]`、`pub sources: &'a SourceMap`、`fn fixture_columns`、`fixture_header` 在 `loader/mod.rs`、`bound_columns` 在 `w17.rs`、`effective_decimal` 在 `e13.rs`），负向检查（`unwrap_or(38|10)` 与 `ReaderBuilder` 在 lint 与 `loader/mod.rs` 下无匹配）；`e13_can_be_downgraded`、`fixture_bytes_count_toward_budget` 各 `1 passed`；`cargo test -p ontoforge-schema w17` 通过 5 个。
- 最终门：`cargo test --workspace --locked`（517 通过、0 失败，本 plan 之前为 02-06 的基线）、`cargo clippy --workspace --all-targets --locked -- -D warnings`、`cargo fmt --all --check` 全部通过；`ontoforge lint examples/ontologies/aviation` 与 `aviation-json` 在 `--deny-warnings --format json` 下输出 `[]` 退出码 0；无 `.snap.new` 残留；`Cargo.toml` / `Cargo.lock` / `examples/**` / `crates/ontoforge/src/**` 与 02-03 的共享函数文件均未改动。

---
*Phase: 02-source-binding-engine*
*Completed: 2026-10-17*
