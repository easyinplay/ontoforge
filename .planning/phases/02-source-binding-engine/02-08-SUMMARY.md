---
phase: 02-source-binding-engine
plan: 08
subsystem: engine
tags: [rust, drift, bind-check, type-compatibility, decimal, timezone, docs, source-binding]

requires:
  - phase: 02-source-binding-engine
    provides: "02-05 的 compare_binding / describe_bindings / DescribedBinding / ExpectedColumn（precision / scale 已由 effective_decimal 填好）/ SqliteEngine::from_fixtures / plan_fixture_tables；02-07 的 lint 规则 ID（E13-E16、W17）"
provides:
  - "ontoforge_engine::bind_check::{DegradeReason, TargetHealth, DriftReport, drift_report, check_drift}：drift 库函数与 healthy / degraded 状态模型，按 object type / link 降级，其余照常"
  - "compare_binding 的完整 D-13 无损映射（compatibility）：整数只加宽、float / double、decimal 整数位规则、char 警告、timestamp 时区在线复核、array 按元素、geopoint / number / 未知类型为 error、绑定列大小写歧义为 error"
  - "docs/source-binding.md：绑定规范，含由测试逐行对照代码的类型兼容表、诊断码、退出码、服务配置、drift 模型与 fixture 模式的验证边界"
affects: [02-09, 02-10, phase-03, phase-04, phase-07]

plan_head_before: a4bc693aca202c7f94a8aeb87a6b588d3eddb8d1

actuals:
  tokens: 20900
  tasks: 3
  commits: 5

tech-stack:
  added: []
  patterns:
    - "compare_binding 是 CLI（validate --online）与 drift（check_drift）共用的唯一比对实现；drift 只是对每个 DescribedBinding 调它再按严重级归类"
    - "文档中的类型表由测试解析并逐行调用 compare_binding：文档是 D-13 的唯一来源，测试保证不矛盾"
    - "引擎健康与 schema drift 分开：只有 is_engine_wide() 的错误让 check_drift 返回 Err，单目标的元数据失败只降级该目标"

key-files:
  created:
    - crates/ontoforge-engine/src/bind_check/compat_tests.rs
    - crates/ontoforge-engine/tests/drift.rs
    - crates/ontoforge-engine/tests/doc_type_table.rs
    - docs/source-binding.md
  modified:
    - crates/ontoforge-engine/src/bind_check.rs
    - crates/ontoforge-engine/src/bind_check/tests.rs
    - .planning/WINDOWS.md

key-decisions:
  - "decimal 兼容用 s ≤ S 且 p - s ≤ P - S（ADR-0021 取代 D-13 字面读法）；P / S 只取自 ExpectedColumn，bind_check 中没有 38 / 10 字面量（E7）"
  - "decimal(p,s) 在 s > S 或整数位超出时都报 TypeIncompatible error；兼容但 p < P 或 s < S 报 PrecisionChanged warning；number 与无界 numeric 报 UnsupportedType error 并提示在 Trino 建 view 做 cast"
  - "timezone 复核：IANA 对带时区列、未声明对带时区列为 warning；aware 对无时区列、未声明对无时区列为 error"
  - "绑定列小写化后有多个候选物理列时报 CaseCollision error 并跳过类型比较（E13），与绑定无关的大小写冲突不报告"
  - "TargetHealth::Healthy 保留 warning 级 mismatch（E12）；单个目标元数据不可读为 Degraded { SourceUnreadable { error } }，error 为 EngineError 的 Display（E3）"
  - "check_drift 在 compare_binding 之外不重复实现任何判定；health_of 作为私有纯函数分离「有 error 即降级」的归类，便于单元测试"

patterns-established:
  - "表驱动兼容测试：compat_tests.rs 以 (属性声明, 时区, 源列类型, 判定) 一行一例，失败时一次列出全部不一致的行"
  - "文档表格测试带自检：用故意写错的一行确认检查本身会报告不一致"

requirements-completed: [BIND-03, BIND-02]

coverage:
  - id: D1
    description: "check_drift 描述选定 profile 的全部绑定并比对：aviation 全部健康；删掉 flights.csv 的一列后只有 Flight 为 Degraded { SourceSchemaDrift }（含 ColumnMissing），其余与 flightCrew 仍健康；editOnly 的 Airport 不出现"
    requirement: "BIND-03"
    verification:
      - kind: integration
        ref: "crates/ontoforge-engine/tests/drift.rs#aviation_is_healthy"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/drift.rs#missing_column_degrades_only_flight"
        status: pass
    human_judgment: false
  - id: D2
    description: "单个目标的元数据查询失败只让该目标为 Degraded { SourceUnreadable }，error 含 PERMISSION_DENIED，其余照常判定；表不存在为 SourceSchemaDrift；整轮引擎错误时 check_drift 返回 Err 而不是报告"
    requirement: "BIND-03"
    verification:
      - kind: integration
        ref: "crates/ontoforge-engine/tests/drift.rs#query_error_degrades_only_that_target"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/drift.rs#missing_table_is_drift"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/drift.rs#engine_error_is_not_a_report"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/drift.rs#degrade_reason_code"
        status: pass
    human_judgment: false
  - id: D3
    description: "只有 warning 级不匹配的目标保持 Healthy 并保留这些 warning（E12），有 error 的目标降级且 mismatches 含全部不匹配"
    requirement: "BIND-03"
    verification:
      - kind: unit
        ref: "crates/ontoforge-engine/src/bind_check/tests.rs#warnings_do_not_degrade"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/drift.rs#warnings_do_not_degrade"
        status: pass
    human_judgment: false
  - id: D4
    description: "D-13 完整无损映射：整数加宽、float / double、decimal 整数位规则与缺省 (38,10)、number 为 error、char 警告、array 按元素、geopoint 与未知类型为 error（help 含 view 与 cast）、link 两端按主键类型比对；integer 读 varchar 仍为 error"
    requirement: "BIND-02"
    verification:
      - kind: unit
        ref: "crates/ontoforge-engine/src/bind_check/compat_tests.rs#integers_only_widen"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-engine/src/bind_check/compat_tests.rs#decimal_integer_digits_and_scale"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-engine/src/bind_check/compat_tests.rs#decimal_without_precision_uses_the_effective_defaults"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-engine/src/bind_check/compat_tests.rs#arrays_are_compared_by_element"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-engine/src/bind_check/compat_tests.rs#unsupported_types_are_errors_with_a_view_hint"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-engine/src/bind_check/compat_tests.rs#link_ends_are_compared_with_the_primary_keys"
        status: pass
      - kind: integration
        ref: "cargo test -p ontoforge --test cli_online"
        status: pass
    human_judgment: false
  - id: D5
    description: "timezone 在线复核与绑定列大小写歧义：IANA / aware 对带 / 不带时区列的四种组合加未声明两种；多个只差大小写的候选为 CaseCollision error，定位到 source.table 或 columns.<key>.column，无关列冲突不报告"
    requirement: "BIND-02"
    verification:
      - kind: unit
        ref: "crates/ontoforge-engine/src/bind_check/compat_tests.rs#timestamps_are_checked_against_the_declared_zone"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-engine/src/bind_check/compat_tests.rs#case_collision"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-engine/src/bind_check/compat_tests.rs#warnings_point_at_the_binding_like_errors"
        status: pass
    human_judgment: false
  - id: D6
    description: "docs/source-binding.md 的类型兼容表（48 行）逐行与 compare_binding 一致；文档提到的每个 ONT 码都存在，且 V16-V20、E13-E16、W17 都有描述"
    requirement: "BIND-02"
    verification:
      - kind: integration
        ref: "crates/ontoforge-engine/tests/doc_type_table.rs#doc_type_table_matches_compare_binding"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/doc_type_table.rs#doc_diagnostic_codes_exist"
        status: pass
    human_judgment: false
  - id: D7
    description: "绑定规范文档的可读性与完整性（章节编排、示例是否贴合实际使用、行文是否清楚）"
    requirement: "BIND-02"
    verification: []
    human_judgment: true
    rationale: "测试只锁定类型表与诊断码；文档整体行文与示例的可用性需要人读一遍。CLI 的 --server-config、--check-pk 在文档中按 ADR-0021 描述，实现由 02-09 交付"

duration: 21min
completed: 2026-10-17
status: complete
---

# Phase 2 Plan 08: drift 状态模型、D-13 无损映射与绑定规范文档 Summary

**`check_drift` 把「有 error 级不匹配的目标降级为 `SourceSchemaDrift`、元数据不可读的单个目标降级为 `SourceUnreadable`、只有 warning 的目标保持 `Healthy { warnings }`」做成库函数；`compare_binding` 扩成 D-13 的完整无损映射（decimal 整数位规则、timezone 复核、char 警告、绑定列大小写歧义为 error）；`docs/source-binding.md` 的 48 行类型表由测试逐行对照代码。**

## Performance

- **Duration:** 约 21 min
- **Started:** 2026-10-16T21:28:54Z
- **Completed:** 2026-10-16T21:49Z（实现与最终门；SUMMARY 提交在其后）
- **Tasks:** 3（Task 1 tracer、Task 2 auto/tdd、Task 3 auto/tdd）
- **Files modified:** 6（相对 `a4bc693`，另有 `.planning/WINDOWS.md` 一条台账）

## Accomplishments

- **drift tracer 打通。** aviation 复制到 tempdir，经 `load_dir` -> `plan_fixture_tables` -> `SqliteEngine::from_fixtures` -> `check_drift`：四个 object type 与 `flightCrew` 全部 `Healthy { warnings: [] }`，editOnly 的 Airport 不出现；把 `flights.csv` 的 `delay_minutes` 列（表头与每行）删掉后只有 Flight 为 `Degraded { SourceSchemaDrift }`，其余照常。
- **E3 / E12。** 单个目标的 `physical` 为 `Err` 时只让它 `Degraded { SourceUnreadable { error } }`，其余照常；`Healthy` 保留 warning；整轮错误（用指向已关闭端口的 `AnyEngine::Trino` 构造出 `Unavailable`）时 `check_drift` 返回 `Err`，不产生报告。
- **D-13 完整映射（`compatibility`）。** 整数只加宽；`float` 只接受 `real`，`double` 接受 `real` / `double`；`string` 接受 `varchar`，`char` 为 `CharPadding` warning；decimal 用 `s ≤ S && p - s ≤ P - S`，兼容且 `p < P || s < S` 为 `PrecisionChanged` warning；`number` 与未知类型为 `UnsupportedType` error；timestamp 在线复核（IANA 对带时区列 warning、`aware` 对无时区列 error、未声明对无时区列 error）；array 递归元素；geopoint 一律 error。P / S 取自 `ExpectedColumn`，`bind_check.rs` 里没有 38 / 10 字面量（E7）。
- **E13 大小写歧义。** 绑定列有多个只差大小写的候选物理列时报一条 `CaseCollision` error（消息列出全部原名，说明列选择不确定），不再做类型比较；与绑定无关的冲突不报告。
- **绑定规范文档。** `docs/source-binding.md` 含绑定格式、backing、profile 选定、fixture 语义与上限、「fixture 模式的验证边界」（E15）、类型兼容表、decimal 整数位推导、timezone 复核、诊断码、命令行与退出码（含单 catalog 出错时只让该 catalog 的绑定得到 `ONT-V19` 且退出码为 3）、服务配置（只有 `engine = "trino" | "sqlite"`，没有库目录项，E1）与 drift 模型。
- **文档与代码锁定。** `doc_type_table.rs` 解析 `type-table` 标记之间的表格，每行构造最小 ontology 调 `compare_binding`；自带一个故意写错的行确认检查本身会报告不一致；第二个测试保证文档里每个 `ONT-` 码都存在于 `codes` 或 `KNOWN_RULE_IDS`，且 V16-V20、E13-E16、W17 都有描述。

## Task Commits

1. **Task 1: 端到端 tracer（tdd）**
   - RED `bd29993`（test）：`aviation_is_healthy`、`missing_column_degrades_only_flight`、`query_error_degrades_only_that_target`、`missing_table_is_drift`、`degrade_reason_code` 与单元测试 `warnings_do_not_degrade` 因断言失败；`engine_error_is_not_a_report` 在 RED 即通过（存根的 `check_drift` 已经是真实的 `describe_bindings` 调用，属回归保护）
   - GREEN `1df851d`（feat）
2. **Task 2: D-13 完整映射（tdd）**
   - RED `3dcd926`（test）：9 个新兼容表测试（`integers_only_widen` 等）、`case_collision`、`link_ends_are_compared_with_the_primary_keys`、`online_type_family_mismatch`（改为 integer 读 bigint 是 error）与 `drift.rs` 的 `warnings_do_not_degrade` 因断言失败
   - GREEN `2da6ea1`（feat）
3. **Task 3: 绑定规范文档与一致性测试（tdd）** `bb57a12`（docs）

**Plan metadata:** 本文件所在的 `docs(02-08)` 提交。无 REFACTOR 提交。

## Tracer 反馈门

`workflow.human_verify_mode = end-of-phase` 且 Task 1 的 `<verify>` 只含 `<automated>`、auto 模式未开：Task 1 提交前重跑 `cargo test -p ontoforge-engine --test drift`（6 通过）、`cargo test -p ontoforge --test cli_online`（9 通过）、`cargo clippy -p ontoforge-engine --all-targets -D warnings` 干净，然后继续 Task 2、3。

## Files Created/Modified

- `crates/ontoforge-engine/src/bind_check.rs` - `Compat` / `compatibility` / `decimal_compatibility` / `timezone_compatibility`、大小写歧义、`DegradeReason` / `TargetHealth` / `DriftReport` / `drift_report` / `check_drift`
- `crates/ontoforge-engine/src/bind_check/compat_tests.rs` - 表驱动兼容测试（新增）
- `crates/ontoforge-engine/src/bind_check/tests.rs` - `warnings_do_not_degrade`，`online_type_family_mismatch` 的族内断言更新
- `crates/ontoforge-engine/tests/drift.rs` - drift 端到端与纯函数测试（新增）
- `crates/ontoforge-engine/tests/doc_type_table.rs` - 文档类型表与诊断码一致性测试（新增）
- `docs/source-binding.md` - 绑定规范（新增）
- `.planning/WINDOWS.md` - 一条 deviation 台账（见下文已知限制）

## Decisions Made

见 frontmatter `key-decisions`。补充：`warnings` 的 `health_of` 归类拆成私有纯函数，是因为 Task 1 时 `compare_binding` 还不产生 warning，无法经 `drift_report` 观察到 warning 行为（见偏差 1）。

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - 阻塞] `warnings_do_not_degrade` 拆成两层、分属两个 task**
- **Found during:** Task 1
- **Issue:** 计划把「只有 warning 的目标为 Healthy 且保留 warning」放在 Task 1，用构造的 `DescribedBinding` 经 `drift_report` 验证。但 Task 1 时 `compare_binding` 只有跨族 error，没有任何会产生 warning 的输入。
- **Fix:** Task 1 把「有 error 即降级、否则保留 warning」抽成私有纯函数 `health_of`，用手工构造的 warning 级 `Mismatch` 做单元测试（`bind_check/tests.rs#warnings_do_not_degrade`）；经 `drift_report` 的版本（`drift.rs#warnings_do_not_degrade`，char(8) 主键列产生 `CharPadding`）在 Task 2 的 RED 提交里加入，Task 2 GREEN 转绿。
- **Files modified:** `crates/ontoforge-engine/src/bind_check.rs`、`bind_check/tests.rs`、`tests/drift.rs`
- **Committed in:** `bd29993`、`3dcd926`、`1df851d`

**2. [Rule 3 - 范围对齐] 兼容表测试放进新文件 `bind_check/compat_tests.rs`**
- **Found during:** Task 2
- **Issue:** 计划写「单元测试放 `bind_check.rs` 的 `#[cfg(test)] mod tests`」，但 02-05 已把该模块外置为 `bind_check/tests.rs`；兼容表的辅助函数与 500 行表驱动用例混进去会让原文件难读。
- **Fix:** 新增 `#[cfg(test)] mod compat_tests;`（与 `tests.rs` 同样是 `bind_check` 的子模块，可见性等价）；`tests.rs` 只更新一条族内断言并加 `warnings_do_not_degrade`。
- **Committed in:** `3dcd926`

**3. [Rule 3 - 阻塞] RED 提交带存根与 `allow(dead_code)`**
- **Found during:** Task 1 的 RED
- **Issue:** 要让测试因断言而不是编译错误失败，需要先有 `DriftReport` 等类型与签名；`health_of` 存根在 RED 时未被使用，clippy 的 `dead_code` 拦截提交。
- **Fix:** RED 提交带返回空结果的 `drift_report` 与 `health_of` 存根，`health_of` 加带原因的临时 `allow(dead_code)`，GREEN 提交中随实现一起移除。`gsd_run check tdd-red-evidence` 解析 node TAP，cargo 输出不是 TAP（与 02-01 到 02-07 相同），因此人工核对：每个目标测试都因对计划行为的断言失败。
- **Committed in:** `bd29993`、`1df851d`

**4. [Rule 1 - 计划条目与可达性不符] 计划里的 decimal 测试数字与规则不一致**
- **Found during:** Task 2 RED 编写
- **Issue:** 我最初写下的预期里 `decimal(3,3)`（`s=3 > S=2`）与缺省 decimal 对 `decimal(38,9)`（整数位 29 > 28）被标成 warning；按计划自己的规则 `s ≤ S 且 p - s ≤ P - S` 这两行都是 error。
- **Fix:** 测试与文档表格都按规则写成 error，并让文档测试锁定。
- **Committed in:** `3dcd926`、`bb57a12`

---

**Total deviations:** 4（2 个阻塞或范围对齐、1 个 RED 证据处理、1 个预期数字修正）
**Impact on plan:** 无范围蔓延；公开 API 与 interfaces 一致（`DegradeReason`、`TargetHealth`、`DriftReport`、`drift_report`、`check_drift`），没有改动计划之外的文件。

## TDD Gate Compliance

- Task 1：RED `bd29993`（`test(02-08)`）-> GREEN `1df851d`（`feat(02-08)`）。
- Task 2：RED `3dcd926` -> GREEN `2da6ea1`。
- Task 3：计划要求的 RED 是「对一个故意写错一行的表格确认失败」。我先写测试，再写文档并故意把 `integer` 读 `bigint` 标成 OK，运行后只有这一行被报告（`the document says OK, compare_binding says E`），修正为 E 后 2 通过；没有单独的 `test(02-08)` 提交，测试与文档同在 `bb57a12`。测试本身还带一个篡改行的自检，保证检查会报告不一致。
- 无 REFACTOR 提交。
- RED 中的意外 GREEN（已调查，不当作 RED 证据）：`engine_error_is_not_a_report`（见 Task Commits）；`doc_diagnostic_codes_exist`（文档写好之后才存在，无 RED 阶段，由「提到的码必须存在、必须描述」两个方向的断言约束）。

## Issues Encountered

- bash 里含单引号混杂的多段 heredoc 一次提交时，工具层报 `unexpected EOF`，整条命令未执行；改用 Write 工具落盘，按行号 `sed` 拼接大块替换时有一次多删一行导致 `doc_type_table.rs` 编译失败，立即修复并重跑。
- `drift` 测试 `engine_error_is_not_a_report` 对关闭端口的连接约 13 s（Trino 客户端的重试），不影响结果；整个 `drift` 目标约 14 s。
- Windows 上 `cargo test --workspace --locked` 需要较长超时，结果不受影响。

## Known Stubs

None。RED 提交里的 `drift_report` / `health_of` 存根全部在 GREEN 提交中替换。

## Known Limitations

- **fixture 模式下含 `float` 属性的绑定会得到 `ONT-V17`。** SQLite 替身（`sqlite/fixture.rs` 的 `physical_of`）把 `float` 与 `double` 属性都报告为 `double`，按 D-13 `float` 只接受 `real`，所以比对为 error。修复在 `sqlite/**`，属本 plan 禁止修改的范围；文档「fixture 模式的验证边界」已写明，`.planning/WINDOWS.md` 已登记（kind: deviation），建议并入 02-09 或 02-10。当前 examples 与测试 fixture 中没有 `float` 属性，所以没有测试失败。
- `--server-config`、`--check-pk` 在文档中按 ADR-0021 描述，CLI 实现由 02-09 交付；文档里 `CONTRIBUTING.md` 对 `ONTOFORGE_PRIVATE_FIXTURES` 的说明由 02-10 补充。

## Threat Flags

None。T-02-31：只接受无损映射，未知类型为 error，绑定列有多个大小写候选时为 error，表驱动测试与文档一致性测试覆盖；T-02-47：只有 `is_engine_wide()` 的错误让 `check_drift` 返回 `Err`，单目标失败为 `SourceUnreadable`，`Healthy` 保留 warning，`query_error_degrades_only_that_target`、`warnings_do_not_degrade` 覆盖；T-02-32：drift 只调 `describe_bindings`（元数据），不扫描数据，`--check-pk` 不属于 drift；T-02-33：新增消息只含 apiName 与物理名，经 `Diagnostic` 输出由 CLI 的 `render::sanitize` 处理。本 plan 没有新增网络端点、认证路径或信任边界。

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- 02-09 依赖的两条判定不变：`integer` 读 `varchar` 为 error（`integers_only_widen` 锁定）、列缺失为 error；`cli_online` 9 个测试保持通过。02-09 还需要决定 `float` 在替身中的报告（见 Known Limitations）。
- 02-10 的文档（`CONTRIBUTING.md`）需补 `ONTOFORGE_PRIVATE_FIXTURES` 的目录约定。
- Phase 7 的服务进程直接调用 `check_drift`，按 `DriftReport` 决定 `/health` 暴露与对应 object type 的读路径降级；启动与定时执行不在本 phase。

## Self-Check: PASSED

- 创建的文件（`bind_check/compat_tests.rs`、`tests/drift.rs`、`tests/doc_type_table.rs`、`docs/source-binding.md`）均存在于磁盘；`git diff --diff-filter=D a4bc693..HEAD` 无删除。
- 提交 `bd29993`、`1df851d`、`3dcd926`、`2da6ea1`、`bb57a12` 均存在，`git log --grep="(02-08)"` 返回 5 条，`git rev-list --count a4bc693..HEAD` 为 5；`test(02-08)` 在对应 `feat(02-08)` 之前。
- Task 1 acceptance（`pub async fn check_drift`、`SOURCE_SCHEMA_DRIFT`、`SOURCE_UNREADABLE`、`Healthy { warnings` 在 `bind_check.rs`；`missing_column_degrades_only_flight` 与 `query_error_degrades_only_that_target` 各 `1 passed`）、Task 2 acceptance（`fn compatibility`；五个变体名都出现在构造处；`cargo test -p ontoforge-engine bind_check` 20 通过；`case_collision` `1 passed`；`unwrap_or(38|10)` 在 `crates/ontoforge-engine/src` 下无匹配；`cli_online` 9 通过）、Task 3 acceptance（`type-table:start` / `type-table:end`、`SOURCE_SCHEMA_DRIFT`、`SOURCE_UNREADABLE`、`ONTOFORGE_PROFILE`、`p - s ≤ P - S`、`fixture 模式的验证边界` 在文档中有匹配；`sqlite_dir` 无匹配；`doc_type_table` `2 passed`）逐条重跑通过。
- 最终门：`cargo test --workspace --locked`（539 通过、0 失败，02-07 基线 517）、`cargo clippy --workspace --all-targets --locked -- -D warnings`、`cargo fmt --all --check` 全部通过；无 `.snap.new` 残留；`lib.rs`、`sqlite/**`、`trino/**`、`config.rs`、`crates/ontoforge/**` 均未改动。

---
*Phase: 02-source-binding-engine*
*Completed: 2026-10-17*
