---
phase: 02-source-binding-engine
plan: 03
subsystem: schema
tags: [source-binding, profile, ontology-as-code, fixture, decimal, timezone, clap-env, json-schema]

requires:
  - phase: 02-source-binding-engine
    provides: "02-01 的 Backing、ProfileName、诊断码 V16..V20、read_limited 与全绿基线（312 个测试）；02-02 声明的 csv、chrono-tz 工作区依赖"
provides:
  - "kind: SourceBinding 的 IR、loader 与 JSON Schema：target（objectType / linkType 外部标签）、source、columns、fixture；profile 由 bindings/<profile>/ 目录推出"
  - "profile 选定顺序 --profile > ONTOFORGE_PROFILE > defaultProfile（clap env 读取，库函数不读环境变量）与 validate::check_profile 的完备性校验（ONT-V16）"
  - "加载期结构校验：columns 键、link 的 from / to、物理名、timezone 取值、fixture 路径与元数据探测、precision / scale 位置（ONT-V06 / V08 / V09 / V16）"
  - "共享函数：PropertyDef::effective_decimal、Ontology::bound_columns、loader::fixture_header、loader::fixture_path / profile_dir"
  - "aviation 与 aviation-json 的 dev profile 绑定（4 个 object type 与 flightCrew）、5 个 fixture、MaintenanceRecord.cost、6 个新 invalid 样例"
affects: [02-04, 02-05, 02-06, 02-07, 02-08, 02-09, 02-10, phase-03, phase-04, phase-07]

plan_head_before: 946d4c0d5c89472865a7d0eacb857095e4d55d73

actuals:
  tokens: 35750
  tasks: 3
  commits: 6

tech-stack:
  added: [chrono-tz 0.10.4（进入 ontoforge-schema）, csv 1.4.0（进入 ontoforge-schema）, clap feature "env"]
  patterns:
    - "绑定文档注册为 DefKey { kind: SourceBinding, api_name: \"{profile}/{target}\" }，诊断经 SourceMap::locate 得到绑定文件 line:col"
    - "文件系统相关的检查（fixture 存在性）以探测闭包注入 assemble：磁盘用 symlink_metadata，内存查文件表，validate 层只拿到结果"
    - "profile 名只作为已加载 map 的 key，任何路径都不由它拼出"
    - "profile 相关的校验（check_profile）与 profile 无关的校验（validate::bindings::check）分开，前者由 pipeline 调用，后者在 load_dir 内运行"

key-files:
  created:
    - crates/ontoforge-schema/src/validate/bindings.rs
    - crates/ontoforge-schema/src/loader/fixture.rs
    - crates/ontoforge-schema/tests/bindings.rs
    - examples/ontologies/aviation/bindings/dev/{flight,aircraft,pilot,maintenance-record,flight-crew}.yaml
    - examples/ontologies/aviation/bindings/dev/fixtures/{flights,aircraft,pilots,maintenance_records,flight_crew}.csv
    - examples/ontologies/aviation-json/bindings/dev/bindings.json
    - examples/ontologies/aviation-json/bindings/dev/fixtures/（与 aviation 字节相同的 5 个 csv）
    - examples/ontologies/invalid/{v16-binding-outside-bindings-dir,v16-source-without-binding,v16-fixture-escapes-root,v08-duplicate-binding,v09-binding-unknown-target,v06-precision-on-string}/
    - crates/ontoforge/tests/snapshots/invalid_examples__{v16-binding-outside-bindings-dir,v16-source-without-binding,v16-fixture-escapes-root,v08-duplicate-binding,v09-binding-unknown-target,v06-precision-on-string}.snap
  modified:
    - crates/ontoforge-schema/src/ir/{binding,mod,object,manifest}.rs
    - crates/ontoforge-schema/src/loader/{mod,dispatch,discover,sourcemap}.rs
    - crates/ontoforge-schema/src/validate/{mod,objects}.rs
    - crates/ontoforge-schema/src/schema_export.rs
    - crates/ontoforge-schema/Cargo.toml
    - crates/ontoforge/src/{cli,lib,pipeline,lint_cmd}.rs
    - crates/ontoforge/tests/{cli,cli_lint}.rs
    - crates/ontoforge-schema/tests/{load_examples,schema_export}.rs
    - Cargo.toml
    - Cargo.lock
    - schemas/ontoforge-definition.schema.json
    - schemas/ontoforge-manifest.schema.json
    - examples/ontologies/{aviation,aviation-json}/{ontology.yaml,objects.yaml|objects.json}

key-decisions:
  - "SourceBinding 的 target 用外部标签枚举（{ objectType: X } / { linkType: y }），columns 值总是对象形式（计划已锁，ADR-0003 spike）"
  - "profile 名不合法一律 ONT-V16，来源不论目录、manifest、--profile 还是环境变量；--profile 的非法名与未知名无文件位置，挂在 ontology.yaml（without_location）"
  - "fixture 存在性与大小只经 symlink_metadata（磁盘）或文件表（内存）探测，不读内容；该检查只在合并阶段无错误时运行，与 validate 同层级"
  - "fixture 路径的任一段含 ':' 一律拒绝（防 NTFS 备用数据流与盘符写法），且最后一段必须是文件名（不接受 '.' / '..' / 空）"
  - "ONTOFORGE_PROFILE 为空串时 clap 把它当作给定的空名，报 ONT-V16 非法 profile 名，不静默忽略"
  - "chrono-tz 在 default-features = false 下 Tz::from_str 可用（区分大小写），RESEARCH A2 已验证"

patterns-established:
  - "新增 kind 的清单：KINDS / IMPLEMENTED_KINDS 长度、DefKind、merge 分支、JSON Schema 重新导出、依赖 kind 列表的既有快照与 schema_export 分支数断言一并更新"
  - "测试子进程辅助函数统一 env_remove(\"ONTOFORGE_PROFILE\")，需要时 Command::env 显式给出，测试进程自身从不改环境"

requirements-completed: [BIND-01, BIND-02]

coverage:
  - id: D1
    description: "kind: SourceBinding 加载进 IR：位置规则（仅 bindings/<profile>/、profile 名合法）、重复（V08）、未知 target（V09）、bindings/<profile>/fixtures/ 不被当作定义文件而其他 fixtures 目录照常加载"
    requirement: "BIND-01"
    verification:
      - kind: integration
        ref: "crates/ontoforge-schema/tests/bindings.rs#flight_binding_loads_under_dev_profile"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-schema/tests/bindings.rs#binding_outside_bindings_dir_is_v16"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-schema/tests/bindings.rs#duplicate_binding_is_v08"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-schema/tests/bindings.rs#top_level_fixtures_dir_still_loaded"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-schema/tests/bindings.rs#fixtures_dir_is_not_scanned"
        status: pass
    human_judgment: false
  - id: D2
    description: "profile 选定顺序（--profile > ONTOFORGE_PROFILE > defaultProfile）与完备性：source 类型缺绑定、editOnly 被绑定、非 manyToMany link 被绑定、未知 profile、未选定 profile 均报 ONT-V16"
    requirement: "BIND-01"
    verification:
      - kind: integration
        ref: "crates/ontoforge/tests/cli.rs#profile_precedence"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-schema/tests/bindings.rs#source_type_without_binding_is_v16"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-schema/tests/bindings.rs#no_profile_selected_is_v16"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-schema/tests/bindings.rs#edit_only_type_with_binding_is_v16"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_lint.rs#lint_honors_the_profile_flag_and_variable"
        status: pass
    human_judgment: false
  - id: D3
    description: "加载期结构校验：columns 键必须是属性（V09），link 恰有 from / to（V16 / V09），物理名非空无控制字符，timezone 为 IANA 名或 aware，fixture 路径不逃出根目录且为 csv / json，fixture 存在性只查元数据，precision / scale 只用于 decimal"
    requirement: "BIND-02"
    verification:
      - kind: integration
        ref: "crates/ontoforge-schema/tests/bindings.rs#column_key_unknown_property_is_v09"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-schema/tests/bindings.rs#link_columns_need_from_and_to"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-schema/tests/bindings.rs#fixture_reference_rules"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-schema/tests/bindings.rs#oversized_fixture_is_v16"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-schema/tests/bindings.rs#timezone_value_rules"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-schema/tests/bindings.rs#precision_on_non_decimal_is_v06"
        status: pass
    human_judgment: false
  - id: D4
    description: "wave 3 共用的 schema 侧唯一实现：effective_decimal（38 / 10 各自独立缺省）、bound_columns（期望列名推导）、fixture_header（CSV 首条记录去 BOM、JSON 键并集）、fixture_path / profile_dir"
    verification:
      - kind: unit
        ref: "crates/ontoforge-schema/src/ir/object.rs#effective_decimal_defaults"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-schema/tests/bindings.rs#bound_columns_object_and_link"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-schema/src/loader/fixture.rs#fixture_header_rules"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-schema/src/loader/discover.rs#fixture_path_rules"
        status: pass
    human_judgment: false
  - id: D5
    description: "aviation 与 aviation-json 的 dev profile 绑定全部 source 类型与 flightCrew，validate 与 lint 均无诊断，二者 IR 相同；MaintenanceRecord.cost decimal(12,2)"
    requirement: "BIND-01"
    verification:
      - kind: integration
        ref: "crates/ontoforge/tests/cli.rs#aviation_is_clean_under_dev"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-schema/tests/load_examples.rs#aviation_dev_profile_binds_every_source_type"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-schema/tests/load_examples.rs#aviation_yaml_and_json_produce_identical_ir"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-schema/tests/schema_export.rs#aviation_bindings_validate"
        status: pass
    human_judgment: false
  - id: D6
    description: "6 个新 invalid 样例各自产生目录名前缀对应的码，快照固定 code / severity / message / help / 位置；schemas/ 与 IR 同步"
    verification:
      - kind: integration
        ref: "crates/ontoforge/tests/invalid_examples.rs（6 个新快照）"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-schema/tests/schema_export.rs#committed_schemas_are_up_to_date"
        status: pass
    human_judgment: false
  - id: D7
    description: "符号链接 fixture 不被跟随（unix）"
    requirement: "BIND-02"
    verification:
      - kind: integration
        ref: "crates/ontoforge-schema/tests/bindings.rs#symlinked_fixture_is_v16"
        status: unknown
    human_judgment: true
    rationale: "测试是 #[cfg(unix)]，Windows 本机不执行；需要 Linux CI 实跑。Windows 上由 probe_disk 的 symlink_metadata 路径同样覆盖目录与不存在的情形"

duration: 35min
completed: 2026-10-17
status: complete
---

# Phase 2 Plan 03: SourceBinding 绑定文件、profile 选定与结构校验 Summary

**新 kind `SourceBinding` 把 object type 与 manyToMany link 绑定到 `source: { catalog, schema, table }`：profile 由 `bindings/<profile>/` 目录推出，选定顺序 `--profile` > `ONTOFORGE_PROFILE` > `defaultProfile`，source 类型缺绑定、editOnly 被绑定、列映射、物理名、fixture 路径、timezone 与 decimal 位置全部在加载期以带 line:col 的 V06 / V08 / V09 / V16 报告；fixture 存在性只查文件元数据。**

## Performance

- **Duration:** 约 35 min
- **Started:** 2026-10-16T19:08:56Z
- **Completed:** 2026-10-16T19:43:39Z（实现与最终门；SUMMARY 提交在其后）
- **Tasks:** 3（Task 1 tracer、Task 2 auto/tdd、Task 3 auto/tdd）
- **Files modified:** 72（相对 `946d4c0`，含 `Cargo.lock`、2 个 schema、6 个快照、示例与 fixture）

## Accomplishments

- **绑定定义层。** `SourceBinding` / `BindingTarget` / `SourceRef` / `ColumnBinding` / `Collation` / `FixtureRef` / `ProfileBindings` 全部 `deny_unknown_fields` + camelCase，字段 doc 进入 JSON Schema。`Manifest.defaultProfile`、`Ontology.{default_profile, bindings}`、`PropertyDef.{precision, scale}` 落地。
- **位置规则。** SourceBinding 只能在 `bindings/<profile>/` 下，定义文档不能出现在 `bindings/` 下，profile 目录名不合 `[a-z][a-z0-9_-]{0,31}`，三者都是 ONT-V16 定位在 `kind` 值；同 profile 同 target 重复为 V08（消息带首次定义的 `file:line:col`）。`bindings/<profile>/fixtures/` 在 `is_candidate` 里路径级排除，`SKIPPED_DIRS` 不变，ontology 根下或 `defs/` 下的 `fixtures/` 目录照常加载（E5）。
- **profile 与完备性。** `pipeline::check_with_profile` 选定 profile 后调用 `validate::check_profile`；source 类型（含 `backing: source` 的 M:N link）在选定 profile 里没有绑定、未选定 profile、未知 profile（带 `did_you_mean`，来自 manifest 时定位 `defaultProfile` 值）均为 V16。editOnly 类型有绑定（任一 profile）与非 M:N link 有绑定在加载期报 V16。全 editOnly 的 ontology 不需要 profile。
- **结构校验。** `columns` 键必须是属性（V09 + 近似名）；link 绑定缺 `from` / `to` 为 V16、多余键为 V09；catalog / schema / table / column 非空且无控制字符；timezone 为 IANA 名或 `aware`；fixture 路径经 `fixture_path` 规范化，不得逃出根目录、必须是 `.csv` / `.json`；`precision` / `scale` 出现在非 decimal 属性上为 V06。
- **fixture 存在性（E23）。** `assemble` 接收探测闭包：磁盘 `symlink_metadata`（常规文件、不跟随链接、≤ 4 MiB），内存查文件表；内容不读。
- **共享函数（E7 / E8 / E9）。** `effective_decimal` 与 `DEFAULT_DECIMAL_PRECISION` / `DEFAULT_DECIMAL_SCALE`、`Ontology::bound_columns`、`loader::fixture_header` 各有唯一实现，02-05 与 02-07 只调用。
- **CLI。** `ProfileArgs`（`--profile`，env `ONTOFORGE_PROFILE`）flatten 进 `validate` 与 `lint`；成功摘要追加 ``, profile `dev` ``。`std::env::var` 在 schema crate 与 pipeline 里为零。
- **示例。** aviation 与 aviation-json 各有 `defaultProfile: dev`、5 个 dev 绑定、5 个字节相同的 fixture、`MaintenanceRecord.cost`；`ontoforge validate` 与 `lint` 对二者输出 `[]`。

## Task Commits

1. **Task 1: tracer（Flight 的 SourceBinding 端到端）**
   - RED `38b39d6`（test）：14 个集成测试中 13 个因断言失败（缺 kind 时全部 V03），`effective_decimal_defaults`、`fixture_path_rules`、`profile_dir_*`、`only_binding_fixture_directories_are_excluded` 断言失败；`top_level_fixtures_dir_still_loaded` 属回归保护，RED 时即通过
   - GREEN `31d339c`（feat）
2. **Task 2: profile 选定与完备性（tdd）**
   - RED `d6310d2`（test）：7 个 check_profile / 加载期 V16 测试失败，CLI 的 `profile_precedence`、`validate_summary_names_the_profile`、`lint_honors_the_profile_flag_and_variable` 因 `--profile` 未知而失败，`aviation_dev_profile_binds_every_source_type` 失败
   - GREEN `f2ef62f`（feat）
3. **Task 3: 结构校验收口（tdd）**
   - RED `810505d`（test）：`fixture_header_rules`、`column_key_unknown_property_is_v09`、`link_columns_need_from_and_to`、`physical_identifier_rules`、`fixture_reference_rules`、`oversized_fixture_is_v16`、`timezone_value_rules`、`precision_on_non_decimal_is_v06`、`aviation_cost_property`、`v06-precision-on-string` 失败
   - GREEN `96d36b7`（feat）

**Plan metadata:** 本文件所在的 `docs(02-03)` 提交。无 REFACTOR 提交。

## Tracer 反馈门

`workflow.human_verify_mode = end-of-phase` 且 Task 1 的 `<verify>` 只含 `<automated>`：Task 1 提交前重跑三条 verify（`cargo test -p ontoforge-schema` 全绿；`validate examples/ontologies/aviation --format json` 输出 `[]` 退出码 0；`cargo test -p ontoforge --test cli --test cli_schema --test invalid_examples` 通过且无 `.snap.new`）后继续 Task 2、3。

## Decisions Made

见 frontmatter `key-decisions`。补充：editOnly / 非 M:N 绑定的 V16 与 V09 一样定位在 `target.<objectType|linkType>` 的名字值上（比整个 `target` 映射更精确）；`Checked.profile` 在加载失败时为 `None`。

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - 测试预期错误] Task 2 清单中的“清单外” 测试：manifest 来源未知 profile 的近似名**
- **Found during:** Task 3 GREEN 前复核 Task 2 测试
- **Issue:** 我在 Task 2 RED 写的 `unknown_profile_from_manifest_points_at_default_profile` 用 `prdo` 对 `dev` 期望 `did you mean \`dev\``，jaro-winkler 对 `prdo` / `dev` 约 0.53，低于 `did_you_mean` 的 0.8，实现正确而预期错误。
- **Fix:** 测试改用笔误 `devv`（得分 ≥ 0.9），实现不变。
- **Files modified:** `crates/ontoforge-schema/tests/bindings.rs`
- **Committed in:** `96d36b7`

**2. [Rule 3 - 阻塞] 新 kind 使既有断言与快照需要更新**
- **Found during:** Task 1 GREEN
- **Issue:** `KINDS` 增加 `SourceBinding` 后，`v02-missing-kind` 快照的 help（`add \`kind: <ObjectType | LinkType | ActionType>\``）与 `schema_export` 测试的 `Definition` 分支数（3）、kind 清单均随之变化。
- **Fix:** 快照仅 help 一行变化（审阅 `git diff -U0`）；`definition_variants_are_discriminated_by_kind` 改为 4 个分支；`schema_export.rs` 的两处描述文字加上 SourceBinding。
- **Files modified:** `crates/ontoforge/tests/snapshots/invalid_examples__v02-missing-kind.snap`、`crates/ontoforge-schema/tests/schema_export.rs`、`crates/ontoforge-schema/src/schema_export.rs`
- **Committed in:** `31d339c`

**3. [Rule 3 - 阻塞] fixture 探测使 Task 1 的 3 个测试需要补 fixture 文件**
- **Found during:** Task 3 GREEN
- **Issue:** Task 3 开始探测 fixture 存在性后，Task 1 里引用 `fixtures/flights.csv` 却未提供该文件的 `unknown_target_is_v09`、`same_target_in_two_profiles_is_not_a_duplicate`、`profile_name_length_boundaries` 出现额外 V16。
- **Fix:** 测试补上对应 profile 目录下的 `fixtures/flights.csv`。
- **Files modified:** `crates/ontoforge-schema/tests/bindings.rs`
- **Committed in:** `96d36b7`

**4. [Rule 1 - 计划条目与真实形态不符] Pilot 的“同名默认映射”只能是 camelCase 列**
- **Found during:** Task 2 示例
- **Issue:** 计划要求每个 object 绑定至少一个属性依赖同名默认映射，但 Pilot 的四个属性（employeeId、fullName、licenseNumber、flightHours）都是多词 camelCase，snake_case 物理列不会与之同名。
- **Fix:** Pilot 绑定不列 `licenseNumber`，其 fixture 表头保留 `licenseNumber`（YAML 注释说明“该表这一列保持 camelCase”）。Aircraft（model）、Flight（tags）、MaintenanceRecord（summary、cost）用单词属性。`aviation_dev_profile_binds_every_source_type` 断言每个绑定至少有一个 `explicit == false` 的列。
- **Files modified:** `examples/ontologies/aviation/bindings/dev/pilot.yaml`、两份 `pilots.csv`、`bindings.json`
- **Committed in:** `f2ef62f`

**5. [Rule 3 - 阻塞] RED 证据格式**
- **Found during:** 每个 task 的 RED
- **Issue:** `gsd_run check tdd-red-evidence` 解析 node TAP 输出，cargo 输出不是 TAP（与 02-01、02-02 相同）。
- **Fix:** 人工核对：每个目标测试都因对计划行为的断言失败，而非编译错误；为让测试可编译，RED 提交带数据类型声明与签名存根（`effective_decimal` 返回 (0,0)、`bound_columns` 返回空、`fixture_path` 返回 Err、`check_profile` 返回空、`fixture_header` 返回 Err）。

---

**Total deviations:** 5（2 个测试预期 / 条目修正、2 个阻塞项、1 个 RED 证据处理）
**Impact on plan:** 无范围蔓延；公开 API 与 interfaces 一致。

## TDD Gate Compliance

- Task 1：RED `38b39d6`（`test(02-03)`）→ GREEN `31d339c`（`feat(02-03)`）。
- Task 2：RED `d6310d2` → GREEN `f2ef62f`。
- Task 3：RED `810505d` → GREEN `96d36b7`。
- 无 REFACTOR 提交。
- RED 中的意外 GREEN（已调查）：`top_level_fixtures_dir_still_loaded`（E5 回归保护，行为本就存在）、`aviation_is_clean_under_dev`（完备性检查尚未存在时示例当然无诊断，GREEN 后随示例补全才有实际约束力）、`fixture_header_errors_do_not_echo_data`（存根的固定错误文本不含数据，GREEN 后对真实实现有约束力）。
- 局限：`symlinked_fixture_is_v16` 是 `#[cfg(unix)]`，Windows 本机不执行，需 Linux CI 覆盖。

## Issues Encountered

- bash 中带反斜杠转义的长 heredoc 与 perl 单引号 heredoc 会吞掉一层 `\`（`"a\\x07"` 变成 `"a\x07"`，YAML 解析因此报 V01）；测试源码改用 Edit 工具直接修正。
- Windows 上 `cargo test --workspace` 需要较长超时，结果不受影响。

## Authentication Gates

None。

## Known Stubs

None。本 plan 的 RED 存根全部在 GREEN 提交中替换。`.planning/WINDOWS.md` 没有新增条目。

## Threat Flags

None。T-02-07（fixture 路径遍历：纯字符串规范化、扩展名白名单、元数据探测、不跟随链接、不读内容）、T-02-08（profile 名只作 map key）、T-02-09（控制字符在加载期拒绝）、T-02-10（示例只用 `demo` / `aviation` 与虚构数据）均已落地。本 plan 没有新增网络端点或认证路径。

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- 02-04（Trino client）与本 plan 无文件冲突，可独立完成；02-05（fixture 加载与在线校验）直接使用 `Ontology::bound_columns`、`loader::fixture_header`、`PropertyDef::effective_decimal`、`ProfileBindings` 与 `Checked`，不再修改 schema 侧文件。
- 02-07（lint）使用 `fixture_header` 与 `bound_columns` 实现 W17，`effective_decimal` 实现 E13，`timezone_for` 实现 E14；schema crate 的 `csv`、`chrono-tz` 依赖已声明，02-07 不再改 Cargo 文件。
- 待后续覆盖：unix 符号链接测试需 Linux CI 实跑；fixture 内容读取与总字节预算由 02-07 负责；同一物理表被两个 object type 绑定在定义层允许，fixture 去重建表与冲突报错属 02-05（E18）。
- 公开文档 `docs/source-binding.md` 尚未创建，计划在后续 plan 交付。

## Self-Check: PASSED

- 创建的文件（`ir/binding.rs`、`validate/bindings.rs`、`loader/fixture.rs`、`tests/bindings.rs`、5 个 YAML 绑定、5+5 个 fixture、`bindings.json`、6 个 invalid 样例目录与快照）均存在于磁盘。
- 提交 `38b39d6`、`31d339c`、`d6310d2`、`f2ef62f`、`810505d`、`96d36b7` 均存在，`git log --grep=02-03` 返回 6 条；`git diff --diff-filter=D` 对 `946d4c0..HEAD` 无删除。
- Task 1 / 2 / 3 的 acceptance 逐条重跑通过（`rg` 正负向检查、`cmp` fixture、modeline 路径可解析、`fd` 计数 5 与 6、`profile_precedence` `1 passed`、`fixture_header_rules` `1 passed`、`oversized_fixture_is_v16` `1 passed`、`names.rs` 与 `diag.rs` 无 diff）。
- 最终门：`cargo test --workspace --locked`（384 通过、0 失败，基线 340，新增 44）、`cargo clippy --workspace --all-targets --locked -- -D warnings`、`cargo fmt --all --check` 全部通过；无 `.snap.new` 残留；`Cargo.lock` 仅 `ontoforge-schema` 依赖表 +2 行。

---
*Phase: 02-source-binding-engine*
*Completed: 2026-10-17*
