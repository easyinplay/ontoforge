---
phase: 02-source-binding-engine
plan: 05
subsystem: engine
tags: [rust, validate-online, source-binding, sqlite-standin, fixtures, decimal, bind-check, insta, wiremock]

requires:
  - phase: 02-source-binding-engine
    provides: "02-02 的 Engine / AnyEngine / printer / sqlite_ddl / split_decimal / EngineError.is_engine_wide；02-03 的 SourceBinding IR、bound_columns、effective_decimal、fixture_header、fixture_path 与 aviation dev 绑定；02-04 的 TrinoEngine 与按 catalog 批量的 describe_tables"
provides:
  - "ontoforge validate --online [--timeout MS]：dev profile 下用内存 SQLite 替身加载全部 fixture，比对绑定的物理表，不匹配报 ONT-V17 并定位到绑定文件 line:col，缺 fixture 报 ONT-V20，引擎失败报 ONT-V19 且退出码 3"
  - "ontoforge_engine::bind_check：BindingKey、MismatchKind、Mismatch、ExpectedColumn、expected_columns、compare_binding（表 / 列存在 + 类型族）、describe_bindings（按 catalog 一次请求、按绑定隔离非整轮错误）"
  - "ontoforge_engine::sqlite::fixture 与 SqliteEngine::from_fixtures：按类型严格转换（decimal 不舍入、timestamp 毫秒、数组 JSON 文本）、CSV / JSON、共享物理表合并、ATTACH 与字节上限、加载后 query_only"
  - "ontoforge::online：run_online / validate_online / engine_error_diagnostic / query_context（QueryContext::new + --timeout 覆盖）"
  - "aviation 与 aviation-json 的 dev fixture 各 23 条以内的虚构数据，覆盖 NULL、decimal(12,2) 边界、Europe/Berlin 拨钟日与三种偏移写法"
affects: [02-06, 02-07, 02-08, 02-09, 02-10, phase-04]

plan_head_before: 726cea17565bb7cfbfd16634330ed0b620f3d3c8

actuals:
  tokens: 28200
  tasks: 2
  commits: 4

tech-stack:
  added: []
  patterns:
    - "compare_binding 是 CLI 与后续 drift 检查的唯一比对实现，Mismatch 只带相对绑定文档的路径，定位由 CLI 经 SourceMap::locate 完成"
    - "引擎错误按 is_engine_wide() 分流：整轮中止 vs 复制到该 catalog 的每个绑定并继续"
    - "fixture 总是先在内存里转换完毕再建表插入，值只经 sqlx bind；建表前预检 ATTACH 数量与库名冲突"
    - "错误文本不复述单元格内容，只给文件、记录行号、列名与类型说明"

key-files:
  created:
    - crates/ontoforge/src/online.rs
    - crates/ontoforge/tests/cli_online.rs
    - crates/ontoforge/tests/snapshots/cli_online__online_missing_column_human.snap
    - crates/ontoforge-engine/src/bind_check.rs
    - crates/ontoforge-engine/src/bind_check/tests.rs
    - crates/ontoforge-engine/src/sqlite/fixture.rs
    - crates/ontoforge-engine/tests/bind_check_isolation.rs
  modified:
    - Cargo.lock
    - crates/ontoforge/Cargo.toml
    - crates/ontoforge/src/cli.rs
    - crates/ontoforge/src/lib.rs
    - crates/ontoforge-engine/src/lib.rs
    - crates/ontoforge-engine/src/sqlite/mod.rs
    - crates/ontoforge-engine/src/sqlite/tests.rs
    - crates/ontoforge-engine/src/printer/sqlite_ddl.rs
    - crates/ontoforge-engine/tests/sqlite_engine.rs
    - examples/ontologies/aviation/bindings/dev/fixtures/（5 个 csv，只追加行）
    - examples/ontologies/aviation-json/bindings/dev/fixtures/（5 个 csv，与 aviation 字节相同）

key-decisions:
  - "类型族（integer / float / decimal / string / boolean / date / timestamp / array / other）比对，跨族即 error；geopoint 按 string 族处理（文本存储），族内规则留给 02-08"
  - "fixture 的 decimal 超出 scale 的小数位或超出 precision - scale 的整数位一律拒绝，不舍入；precision 或 scale 超过 38 位直接拒绝，同时给规范化补零设上界"
  - "timestamp 小数秒超过 3 位拒绝而不是截断（计划写作规范化为毫秒；报告类型是 timestamp(3)，截断会静默丢信息）"
  - "整数单元格不接受前导 +（i64::from_str 会接受）；布尔只接受小写 true / false"
  - "替身对外报告的整数物理类型由属性类型的 min / max 推出（tinyint / smallint / integer / bigint），未映射的多余列报告为 varchar"
  - "ontoforge::online 在 validate_online 之外增加 run_online / OnlineRun，因为成功摘要行需要 profile 名与已比对绑定数；validate_online 保留为计划约定的薄封装"
  - "CSV 记录的 line 取 csv::Position::line（表头占第 1 行，首条数据记录为第 2 行）；JSON 取对象序号，消息里说明是第几个对象"

patterns-established:
  - "RED 先提交带存根的编译通过测试：存根返回空结果，使目标测试因断言而不是编译错误失败"
  - "断言错误文本不泄露数据时用显眼的伪密钥（s3cr3t-value）跨类型验证，不对任意输入做子串检查（范围上界等 schema 文本会误撞）"

requirements-completed: [BIND-03, ENGINE-04]

coverage:
  - id: D1
    description: "validate --online 在 dev profile 下用 SQLite 替身端到端跑通 aviation 与 aviation-json，退出码 0、输出 []，人类摘要含 online 与 profile `dev`；--timeout 只能与 --online 同用"
    requirement: "BIND-03"
    verification:
      - kind: integration
        ref: "crates/ontoforge/tests/cli_online.rs#online_aviation_is_clean"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_online.rs#timeout_requires_online"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_online.rs#offline_validate_unchanged"
        status: pass
      - kind: unit
        ref: "crates/ontoforge/src/online.rs#query_context_uses_defaults_and_timeout_flag"
        status: pass
    human_judgment: false
  - id: D2
    description: "绑定的显式列缺失定位到 column 值，同名默认映射的列缺失与表缺失定位到 source.table（含 human 快照），跨类型族报 ONT-V17 error，族内（integer 对 bigint）本 plan 不报"
    requirement: "BIND-03"
    verification:
      - kind: integration
        ref: "crates/ontoforge/tests/cli_online.rs#online_missing_explicit_column"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_online.rs#online_missing_implicit_column"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_online.rs#online_missing_column_human"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-engine/src/bind_check/tests.rs#online_missing_table"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-engine/src/bind_check/tests.rs#online_type_family_mismatch"
        status: pass
    human_judgment: false
  - id: D3
    description: "选定 profile 有不带 fixture 的绑定时 --online 报 ONT-V20 并提示 --server-config，不静默跳过"
    requirement: "BIND-03"
    verification:
      - kind: integration
        ref: "crates/ontoforge/tests/cli_online.rs#online_binding_without_fixture_is_v20"
        status: pass
    human_judgment: false
  - id: D4
    description: "describe_bindings 每个 catalog 只发一次元数据请求；非整轮错误只落到该 catalog 的绑定，其余绑定照常；整轮错误中止"
    requirement: "BIND-03"
    verification:
      - kind: integration
        ref: "crates/ontoforge-engine/tests/bind_check_isolation.rs#query_error_isolated_to_its_catalog"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/bind_check_isolation.rs#engine_wide_error_aborts_round"
        status: pass
    human_judgment: false
  - id: D5
    description: "fixture 按类型严格转换：空 CSV 单元格与 JSON null / 缺失键为 NULL，decimal 规范化且不舍入，整数范围，有限浮点，布尔、日期、timestamp 的本地与 aware 两种写法，数组 JSON 文本，错误带记录行号且不复述单元格"
    requirement: "ENGINE-04"
    verification:
      - kind: integration
        ref: "crates/ontoforge-engine/tests/sqlite_engine.rs#decimal_boundaries_12_2"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/sqlite_engine.rs#integer_ranges"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/sqlite_engine.rs#timestamps"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/sqlite_engine.rs#arrays"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/sqlite_engine.rs#json_fixture_loads"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/sqlite_engine.rs#csv_empty_cell_is_null_for_every_type"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/sqlite_engine.rs#fixture_error_has_line"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/sqlite_engine.rs#fixture_errors_never_repeat_cell_contents"
        status: pass
    human_judgment: false
  - id: D6
    description: "共享物理表只建一次并校验同一 fixture 与列类型一致，表名只差大小写视为同一表；ATTACH 上限 10 与库名冲突；全部 fixture 字节预算；加载后 query_only"
    requirement: "ENGINE-04"
    verification:
      - kind: integration
        ref: "crates/ontoforge-engine/tests/sqlite_engine.rs#shared_table_loaded_once"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/sqlite_engine.rs#shared_table_conflicts_are_errors"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/sqlite_engine.rs#attach_limit"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-engine/src/sqlite/tests.rs#total_fixture_bytes_capped"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-engine/src/sqlite/tests.rs#query_only_after_load"
        status: pass
    human_judgment: false
  - id: D7
    description: "aviation 与 aviation-json 的 fixture 逐字节相同，每个至少 20 条记录，含 decimal(12,2) 边界、2026-03-29 与 2026-10-25 拨钟日、+01:00 / +02:00 / Z 三种偏移、空数组，validate --online 保持干净"
    requirement: "ENGINE-04"
    verification:
      - kind: integration
        ref: "crates/ontoforge/tests/cli_online.rs#aviation_fixture_data"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_online.rs#aviation_json_fixtures_match_yaml_fixtures"
        status: pass
    human_judgment: false
  - id: D8
    description: "同一套 compare_binding 在真实 Trino 上的在线校验（线格式、information_schema 大小写、真实权限错误）"
    requirement: "BIND-03"
    verification: []
    human_judgment: true
    rationale: "本 plan 的 CLI 只走 SQLite 替身；Trino 路径的 CLI 与 golden 测试属 02-09 / 02-10。替身只证明列存在与值可转换，不证明真实源列的类型兼容（E15，文档在 02-08）"

duration: 30min
completed: 2026-10-17
status: complete
---

# Phase 2 Plan 05: validate --online、bind_check 与 SQLite 替身 fixture Summary

**`ontoforge validate --online` 把选定 profile 的 fixture 按类型严格加载进内存 SQLite 替身，按 catalog 批量描述物理表，用唯一的 `compare_binding` 比对表 / 列 / 类型族，并把不匹配映射成定位到绑定文件 line:col 的 ONT-V17（缺 fixture 为 ONT-V20，引擎失败为 ONT-V19 与退出码 3）；decimal 全程规范化字符串、从不舍入。**

## Performance

- **Duration:** 约 30 min
- **Started:** 2026-10-16T20:10:58Z
- **Completed:** 2026-10-16T20:40:48Z（实现与最终门；SUMMARY 提交在其后）
- **Tasks:** 2（Task 1 tracer、Task 2 auto/tdd）
- **Files modified:** 26（相对 `726cea1`，含 `Cargo.lock`）

## Accomplishments

- **tracer 打通。** 定义层（02-03）、引擎层（02-02）与 CLI 在一条线上：`check_with_profile` → `plan_fixture_tables` → `SqliteEngine::from_fixtures` → `describe_bindings` → `compare_binding` → ONT-V17。aviation 与 aviation-json 在 Windows 本机零外部依赖跑通，输出 `[]` 退出码 0。
- **唯一比对实现（E7 / E9）。** `expected_columns` 只是 `Ontology::bound_columns` 加 `PropertyDef::effective_decimal` 与 `timezone_for`，`bind_check.rs` 与 `sqlite/` 里没有 38 / 10 的字面量，decimal 文本语法只有 `value::split_decimal`，表头只有 `loader::fixture_header`，读取只有 `loader::read_limited(.., false, MAX_FILE_BYTES)`（E8 / E10）。
- **按 catalog 批量与按绑定隔离（E3 / E22）。** `describe_bindings` 每个 catalog 一次 `describe_tables`；`is_engine_wide()` 为假的错误复制到该 catalog 的每个绑定并继续，为真则中止整轮。wiremock 测试断言 3 个绑定 2 个 catalog 恰好 2 个 POST，`PERMISSION_DENIED` 只落到 `other` 的绑定，401 中止整轮。CLI 把单绑定错误转为定位到其 `source` 值的 V19，整轮错误转为不带位置的 V19，任一 V19 退出码 3。
- **fixture 语义（D-11 / ADR-0021）。** 空 CSV 单元格与 JSON `null` / 缺失键为 NULL；decimal 规范化到 scale 位、拒绝多余小数位与整数位、`-0` 为 `0.00`、JSON 中必须写成字符串；整数按属性范围；timestamp 规范化为 `YYYY-MM-DD HH:MM:SS.fff`，本地列拒绝偏移、aware 列要求 `Z` 或 `±HH:MM` 并规范化为 `+HH:MM`；数组为 JSON 数组文本并逐元素按元素类型转换；错误只含文件、记录行号、列名与类型说明。
- **共享表、上限（E18 / E23）。** 同一物理表（小写化的 catalog / schema / table）合并为一个 `FixtureTable`，必须引用同一 fixture 文件且同名列逻辑类型一致，列取并集；建表前预检 ATTACH ≤ 10 与 `catalog__schema` 库名冲突；全部 fixture 字节合计受 `MAX_TOTAL_BYTES` 约束（内部带预算参数的函数供测试）；全部加载完成后 `PRAGMA query_only = ON`。
- **02-02 遗留项已清。** `describe_tables` 现在由加载时登记的元数据回答（小写比较，键为调用方传入的 `TableRef`）；`sqlite_ddl` 的 `allow(dead_code)` 已删除。`.planning/WINDOWS.md` 的条目 1、2 标记为 fixed。
- **演示数据（D-12）。** aviation 每个 fixture 增至 22 到 43 条记录（表头逐字节不变，只追加），aviation-json 复制同样的文件。

## Task Commits

1. **Task 1: 端到端 tracer（tdd）**
   - RED `4dbac0b`（test）：4 个 CLI 测试因退出码为 0 而非 1 失败，4 个 `bind_check` 单元测试因存根 `compare_binding` 返回空失败，2 个 wiremock 测试因存根 `describe_bindings` 返回空失败，`query_context_uses_defaults_and_timeout_flag` 因 trace token 前缀失败
   - GREEN `a46f81f`（feat）
2. **Task 2: fixture 语义补齐与 aviation 演示数据（tdd）**
   - RED `4de35a8`（test）：13 个 `sqlite_engine` 测试与 `aviation_fixture_data` 因断言失败
   - GREEN `d60d78d`（feat）

**Plan metadata:** 本文件所在的 `docs(02-05)` 提交。无 REFACTOR 提交。

## Tracer 反馈门

`workflow.human_verify_mode = end-of-phase` 且 Task 1 的 `<verify>` 只含 `<automated>`、auto 模式未开：Task 1 提交前重跑三条 verify（`cli_online` 7 通过、`bind_check` 单元 7 通过、`bind_check_isolation` 2 通过；`validate examples/ontologies/aviation --online --format json --color never` 输出 `[]` 退出码 0；`sql_text_only_from_printer` 1 通过且 `clippy -p ontoforge -p ontoforge-engine --all-targets -D warnings` 干净），通过后继续 Task 2。

## Files Created/Modified

- `crates/ontoforge/src/online.rs` - `run_online` / `validate_online` 编排、V16 / V17 / V19 / V20 诊断映射、`query_context`
- `crates/ontoforge/src/{cli,lib}.rs` - `--online`、`--timeout`（`requires = "online"`），在线摘要行，`report` 拆出 `print_diagnostics`
- `crates/ontoforge-engine/src/bind_check.rs` - 比对、类型族、按 catalog 批量描述
- `crates/ontoforge-engine/src/sqlite/fixture.rs` - 计划、合并、CSV / JSON 解析、按类型转换与规范化
- `crates/ontoforge-engine/src/sqlite/mod.rs` - `from_fixtures`、预检、元数据登记、`query_only`、NULL 解码修正
- `crates/ontoforge/tests/cli_online.rs`、`tests/snapshots/cli_online__online_missing_column_human.snap`、`crates/ontoforge-engine/tests/{bind_check_isolation,sqlite_engine}.rs`、`src/bind_check/tests.rs`、`src/sqlite/tests.rs` - 测试
- `examples/ontologies/{aviation,aviation-json}/bindings/dev/fixtures/*.csv` - 追加的虚构数据行

## Decisions Made

见 frontmatter `key-decisions`。

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] SQLite 替身把 NULL 读回为列的缺省值**
- **Found during:** Task 2 RED（`csv_empty_cell_is_null_for_every_type`、`unmapped_columns_are_kept_as_text`）
- **Issue:** 02-02 的 `decode` 用 `ValueRef::type_info()` 判断类型，而 sqlx 对 NULL 值返回的是列的声明类型，随后 `try_get` 得到 `""` / `0` / `0.0`。空单元格因此读回为非 NULL。
- **Fix:** `decode` 先检查 `raw.is_null()`，是则直接返回 `Value::Null`。
- **Files modified:** `crates/ontoforge-engine/src/sqlite/mod.rs`
- **Verification:** 上述两个测试转绿，`cargo test --workspace --locked` 458 通过。
- **Committed in:** `d60d78d`

**2. [Rule 2 - 缺失的关键校验] decimal 的 precision / scale 上界**
- **Found during:** Task 2 实现规范化补零
- **Issue:** schema 层不校验 `precision` / `scale` 的范围；`scale` 取极大值时规范化会按它补零，等于由 ontology 作者控制分配大小。
- **Fix:** `normalize_decimal` 对超过 38 位的 precision 或 scale 直接拒绝（Trino 的上限），补零因此有界。
- **Files modified:** `crates/ontoforge-engine/src/sqlite/fixture.rs`
- **Committed in:** `d60d78d`

**3. [Rule 1 - 测试预期错误] 对任意输入做「消息不含输入」的子串检查**
- **Found during:** Task 2 GREEN
- **Issue:** `fx_rejects` 对长度 ≥ 3 的输入断言消息不含输入，但范围上界（`2147483648` 在 `-2147483648` 内）与示例文本（`2026-03-29`、`["a","b"]`）会误撞，实现并无泄露。
- **Fix:** 去掉通用检查，改为 `fixture_errors_never_repeat_cell_contents`：用伪密钥 `s3cr3t-value` 对每种类型、JSON 路径与数组元素验证。
- **Files modified:** `crates/ontoforge-engine/tests/sqlite_engine.rs`
- **Committed in:** `d60d78d`

**4. [Rule 3 - 阻塞] clippy pedantic**
- **Found during:** Task 1 GREEN 的 clippy 门
- **Issue:** `unnecessary_wraps`（Task 1 的原样文本转换占位）与测试里的 `redundant_closure_for_method_calls`、`manual_let_else`。
- **Fix:** 占位函数带原因的临时 `allow`，Task 2 实现后随占位一起消失；其余按 lint 建议改写。
- **Committed in:** `a46f81f`、`d60d78d`

**5. [Rule 3 - 范围对齐] 测试文件组织与 `run_online`**
- **Found during:** Task 1
- **Issue:** 计划把 bind_check 单元测试写在 `bind_check.rs`、`query_error_*` 之外的 sqlite 单元测试写在 `sqlite/mod.rs`；长测试内联会让这些文件膨胀（含多行 YAML 常量，用 shell 缩进会破坏内容）。另外成功摘要需要 profile 名与绑定数，`validate_online` 的 `(LoadOutput, u8)` 签名装不下。
- **Fix:** 用 `#[cfg(test)] mod tests;` 指向 `bind_check/tests.rs` 与 `sqlite/tests.rs`（与 02-02 相同，可见性等价于内联）；新增 `OnlineRun` / `run_online`，`validate_online` 保留为薄封装。
- **Committed in:** `4dbac0b`、`a46f81f`

**6. [Rule 3 - 阻塞] RED 证据格式**
- **Found during:** 两个 task 的 RED
- **Issue:** `gsd_run check tdd-red-evidence` 解析 node TAP，cargo 输出不是 TAP（与 02-01 到 02-04 相同）。
- **Fix:** 人工核对：每个目标测试都因对计划行为的断言失败，而非编译错误。为让测试可编译，RED 提交带签名存根（`compare_binding` / `describe_bindings` / `plan_fixture_tables` 返回空，`from_fixtures` 返回空引擎，`run_online` 只做离线检查，`query_context` 不覆盖超时）。

---

**Total deviations:** 6（1 个 bug、1 个缺失的关键校验、1 个测试预期错误、2 个阻塞项、1 个 RED 证据处理）
**Impact on plan:** 无范围蔓延；公开 API 与 interfaces 一致，`OnlineRun` / `run_online` 是新增，其余按计划。

## TDD Gate Compliance

- Task 1：RED `4dbac0b`（`test(02-05)`）→ GREEN `a46f81f`（`feat(02-05)`）。
- Task 2：RED `4de35a8` → GREEN `d60d78d`。
- 无 REFACTOR 提交。
- RED 中的意外 GREEN（已调查，不当作 RED 证据）：`online_aviation_is_clean`、`offline_validate_unchanged`、`timeout_requires_online`（clap 的 `requires` 在 Task 1 之前已由 `cli.rs` 的字段在 RED 提交中给出，属回归锁定）；`expected_columns_*`、`matching_table_has_no_mismatch`、`column_names_are_compared_in_lower_case`（存根 `compare_binding` 对匹配表本就返回空）；`header_only_csv_loads_an_empty_table`、`ragged_records_are_refused_with_their_line`（Task 1 的 CSV 读取已满足）；`query_only_after_load`、`total_fixture_bytes_capped`（`from_fixtures_with_budget` 与 `query_only` 在 Task 1 的 `from_fixtures` 中已实现，因为 `SqliteEngine` 的加载入口只有一个）；`aviation_json_fixtures_match_yaml_fixtures`（两份旧数据本就相同）。
- 局限：unix 符号链接 fixture 的拒绝仍只由 02-03 的 `#[cfg(unix)]` 测试覆盖，本 plan 的 `read_limited(.., false, ..)` 路径在 Windows 上未用链接实测。

## Issues Encountered

- 一次 bash 里的 `python - <<EOF` 空输入把整条命令挂起超过 120 s（与 02-04 相同的陷阱）；已用 `taskkill` 结束两个 `python -` 空闲进程，之后大段文本一律用 Write 工具落盘。当时没有任何仓库文件被半写入（`Cargo.toml` 的 sed 在进程结束后执行了一次，已核对没有重复行）。
- 含多行字符串常量的测试文件不能用 shell 缩进处理（会改变 YAML 内容），已改用 Write / 追加方式。
- Windows 上 `cargo test --workspace --locked` 需要较长超时，结果不受影响。

## Known Stubs

None。`MismatchKind` 的 `PrecisionChanged`、`TimezoneMismatch`、`CharPadding`、`CaseCollision`、`UnsupportedType` 五个变体按 interfaces 声明，本 plan 不产生它们，由 02-08 填充，属契约而非占位值。ONT-V20 的 help 文字提到 `--server-config`，该参数由 02-09 添加；这一前向引用已登记到 `.planning/WINDOWS.md`（kind: deviation）。02-02 登记的两条（`describe_tables` 元数据来源、`sqlite_ddl` 的 `allow(dead_code)`）已修复并标记 fixed。

## Threat Flags

None。T-02-18（值注入）：fixture 值只经 sqlx `bind`，DDL / INSERT 文本只来自 `printer::sqlite_ddl`，`sql_text_only_from_printer` 仍通过；T-02-19：`query_only_after_load`；T-02-20：单文件 4 MiB、记录 100 000、总字节预算、ATTACH 10 与库名冲突预检，均有测试；T-02-45：`query_error_isolated_to_its_catalog` 与 `engine_wide_error_aborts_round`；T-02-21：诊断统一经 `render` 的 `sanitize`；T-02-22：V17 消息只含 apiName 与物理名，fixture 错误不复述单元格（`fixture_errors_never_repeat_cell_contents`）。本 plan 没有新增网络端点或认证路径；新增的文件访问就是计划威胁模型里的 fixture 读取。

## User Setup Required

None - no external service configuration required。

## Next Phase Readiness

- 02-06 / 02-07 与本 plan 没有文件冲突：本 plan 未改 `trino/`、`config.rs`、`crates/ontoforge-schema/**`；aviation fixture 表头逐字节不变，02-07 的 W17 读取表头不受影响。
- 02-08 在 `compare_binding` 上补族内规则（精度、时区、char 填充、大小写冲突、不支持类型），并写文档说明 fixture 模式只验证列存在与值可转换（E15）。
- 02-09 为 `--online` 加 `--server-config` 的 Trino 路径（复用 `describe_bindings`、`engine_error_diagnostic`、`query_context`、退出码 3）并实现 `--check-pk`；`FixtureTable.def_names` 已为 `serve --dev` 的多绑定共享表准备好。
- 待后续覆盖：真实 Trino 的线格式与 `information_schema` 行为属 02-10 的 Linux CI golden 测试；替身与 Trino 的查询语义一致性（QUERY-08）在 Phase 4 验证；夏令时不存在或重复的本地时间原样保存，换算属 Phase 4 读路径。

## Self-Check: PASSED

- 创建的文件（`online.rs`、`bind_check.rs`、`bind_check/tests.rs`、`sqlite/fixture.rs`、`cli_online.rs`、快照、`bind_check_isolation.rs`）均存在于磁盘。
- 提交 `4dbac0b`、`a46f81f`、`4de35a8`、`d60d78d` 均存在，`git log --grep=02-05` 返回 4 条，`test(02-05)` 在 `feat(02-05)` 之前；`git diff --diff-filter=D` 对每个提交无删除。
- Task 1 acceptance（`pub fn validate_online`、`codes::V17`、`pub online: bool`、`query_only`、快照含 `ONT-V17` 与 `bindings/dev/flight.yaml`、`online_missing_explicit_column` 与 `query_error_isolated_to_its_catalog` 各 `1 passed`、`bound_columns` / `effective_decimal` / `fixture_header` / `read_limited` 正向匹配、`unwrap_or(38|10)` 无匹配）与 Task 2 acceptance（`decimal_boundaries_12_2` `1 passed`、`shared_table` 2 通过、`total_fixture_bytes_capped` `1 passed`、`split_decimal` 在 `fixture.rs`、`9999999999.99` 与 `2026-10-25` 在 fixture 中、`diff -r` 退出码 0、对 HEAD 的 diff 无被删除的 fixture 行）逐条重跑通过。
- 计划级 verification：`validate --online` 对 aviation 与 aviation-json 输出 `[]` 退出码 0；`sql_text_only_from_printer` 通过。
- 最终门：`cargo test --workspace --locked`（458 通过、0 失败，基线 421，新增 37）、`cargo clippy --workspace --all-targets --locked -- -D warnings`、`cargo fmt --all --check` 全部通过；无 `.snap.new` 残留。

---
*Phase: 02-source-binding-engine*
*Completed: 2026-10-17*
