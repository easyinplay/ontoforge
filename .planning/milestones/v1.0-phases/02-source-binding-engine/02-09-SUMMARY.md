---
phase: 02-source-binding-engine
plan: 09
subsystem: cli
tags: [rust, cli, trino, wiremock, server-config, check-pk, exit-codes, sqlite-fixture]

requires:
  - phase: 02-source-binding-engine
    provides: "02-04 的 load_server_config / build_trino_engines / TrinoConfig::query_context / ConfigError span；02-05 的 validate_online、describe_bindings、compare_binding、plan_fixture_tables、SqliteEngine::from_fixtures；02-06 的 EngineError 分类与 is_engine_wide"
provides:
  - "`ontoforge validate --online --server-config <FILE>`：profile 映射到 trino 时经 ontology 身份的 TrinoEngine 校验，映射到 sqlite 时走 fixture 替身（E1：无库目录模式）"
  - "引擎侧失败 fail-closed：ONT-V19 + 退出码 3，无成功摘要；整轮中止时只输出 V19；单 catalog 错误只让该 catalog 的绑定得到 V19（E3）"
  - "服务配置错误为 ONT-V20，定位到配置文件 line:col"
  - "ontoforge_engine::pk_check::{PkFinding, PkQueries, pk_queries, check_primary_keys, display_value, MAX_DUPLICATE_SAMPLES}：重复值查询排除 NULL、NULL 单独计数（E17）"
  - "`--check-pk`（requires --online）：每个绑定的重复主键 / NULL 主键各一条 ONT-V18，定位 source.table"
  - "SQLite 替身对 float 属性报告 real（不再是 double）"
affects: [02-10, phase-04, phase-07]

plan_head_before: 7e56a83c2493975aba6f9c58221f5dc0624ea941

actuals:
  tokens: 18000
  tasks: 2
  commits: 6

tech-stack:
  added: []
  patterns:
    - "CLI 子进程测试在 multi_thread tokio 测试中用 spawn_blocking 启动，wiremock 在测试进程内模拟 Trino"
    - "compare_all 用 Result 区分整轮失败（Err = 唯一的 V19）与逐绑定结果；整轮失败时不输出任何其他诊断"
    - "--check-pk 逐绑定调用 check_primary_keys（单元素 keys），使单 catalog 错误只影响该绑定"
    - "引擎层查询只经 printer AST 构造，查询文本由 insta 快照按两种方言固定"

key-files:
  created:
    - crates/ontoforge-engine/src/pk_check.rs
    - crates/ontoforge-engine/tests/pk_check.rs
    - crates/ontoforge-engine/tests/snapshots/pk_check__pk_duplicates_sqlite.snap
    - crates/ontoforge-engine/tests/snapshots/pk_check__pk_duplicates_trino.snap
    - crates/ontoforge-engine/tests/snapshots/pk_check__pk_nulls_sqlite.snap
    - crates/ontoforge-engine/tests/snapshots/pk_check__pk_nulls_trino.snap
    - crates/ontoforge/tests/cli_online_trino.rs
    - crates/ontoforge/tests/cli_check_pk.rs
    - crates/ontoforge/tests/snapshots/cli_online_trino__unavailable_human.snap
  modified:
    - crates/ontoforge/src/cli.rs
    - crates/ontoforge/src/lib.rs
    - crates/ontoforge/src/online.rs
    - crates/ontoforge-engine/src/lib.rs
    - crates/ontoforge-engine/src/sqlite/fixture.rs
    - crates/ontoforge-engine/tests/sqlite_engine.rs
    - crates/ontoforge/tests/cli_online.rs
    - docs/source-binding.md
    - .planning/WINDOWS.md

key-decisions:
  - "整轮级引擎错误（不可达、认证失败）时 JSON 只含一条 ONT-V19，连「fixture 被忽略」的 V17 warning 也不输出：Trino 根本没被查询，任何其他结论都不成立"
  - "check_primary_keys 只返回有问题的绑定（空表与干净表无 finding）；CLI 逐绑定调用以隔离单 catalog 错误，接口仍是 plan 规定的 Result<Vec<PkFinding>, EngineError>"
  - "新增公开的 pk_queries / PkQueries 供快照测试直接打印两条查询，避免经 wiremock 间接取查询文本"
  - "more_duplicates 由行数 > 20 判定（查询 LIMIT 21、ctx.max_rows = 21），不依赖引擎的 truncated 标志"

requirements-completed: [BIND-03, ENGINE-01, ENGINE-02]

coverage:
  - id: D1
    description: "validate --online --server-config 把 profile 映射到 Trino（wiremock）完成在线校验：干净、缺列、类型不兼容、表缺失，fixture 被忽略为 V17 warning，请求带 trace token 与 client tags，未给 --check-pk 时无 GROUP BY"
    requirement: BIND-03
    verification:
      - kind: integration
        ref: "crates/ontoforge/tests/cli_online_trino.rs#trino_profile_clean"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_online_trino.rs#trino_profile_missing_column"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_online_trino.rs#trino_type_mismatch"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_online_trino.rs#trino_missing_table"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_online_trino.rs#sqlite_profile_uses_fixtures"
        status: pass
    human_judgment: false
  - id: D2
    description: "引擎不可用 / 认证失败 / 单 catalog 查询错误时 ONT-V19 + 退出码 3，无成功摘要；密钥不出现在输出中"
    requirement: ENGINE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge/tests/cli_online_trino.rs#trino_unavailable_exits_3"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_online_trino.rs#unauthorized_exits_3"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_online_trino.rs#one_catalog_query_error_is_v19_others_checked"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_online_trino.rs#secret_never_printed"
        status: pass
    human_judgment: false
  - id: D3
    description: "Trino 路径的 QueryContext 来自服务配置，--timeout 覆盖（E11）；trace token 前缀 ontoforge-cli-"
    requirement: ENGINE-01
    verification:
      - kind: integration
        ref: "crates/ontoforge/tests/cli_online_trino.rs#timeout_flag_is_sent"
        status: pass
      - kind: unit
        ref: "crates/ontoforge/src/online.rs#trino_context_takes_the_configured_limits"
        status: pass
    human_judgment: false
  - id: D4
    description: "服务配置错误为 ONT-V20 并定位到配置文件；--check-pk 无 --online 为用法错误退出码 2"
    requirement: BIND-03
    verification:
      - kind: integration
        ref: "crates/ontoforge/tests/cli_online_trino.rs#config_errors_are_v20_with_location"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_online_trino.rs#check_pk_requires_online"
        status: pass
    human_judgment: false
  - id: D5
    description: "--check-pk：重复值查询排除 NULL、NULL 单独计数，最多 20 个样例，fixture 替身与 Trino 两条路径，已有 V17 error 的绑定不扫描，退出码表 0/1/1/1/2/3"
    requirement: BIND-03
    verification:
      - kind: unit
        ref: "crates/ontoforge-engine/tests/pk_check.rs#pk_queries_are_printed"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-engine/tests/pk_check.rs#nulls_are_not_duplicates"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-engine/tests/pk_check.rs#twenty_groups_are_not_more"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_check_pk.rs#check_pk_on_trino"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_check_pk.rs#check_pk_reports_null_keys"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_check_pk.rs#exit_code_table"
        status: pass
    human_judgment: false
  - id: D6
    description: "fixture 模式下 float 属性报告为 real，不再被 D-13 表误判为 ONT-V17"
    requirement: BIND-03
    verification:
      - kind: unit
        ref: "crates/ontoforge-engine/tests/sqlite_engine.rs (float reported as real)"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_online.rs (float and double properties validate clean)"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-engine/tests/doc_type_table.rs"
        status: pass
    human_judgment: false

duration: 45min
completed: 2026-10-17
status: complete
---

# Phase 2 Plan 09: validate --online 接入 Trino 与 --check-pk Summary

**`validate --online --server-config` 经 TrinoEngine（wiremock 覆盖）校验绑定，引擎失败 fail-closed 为 ONT-V19 + 退出码 3、配置错误为定位到配置文件的 ONT-V20；`--check-pk` 用 printer 生成的「排除 NULL 的重复值查询 + 单独 NULL 计数」报告 ONT-V18。**

## Performance

- **Duration:** 约 45 min（首个 executor 约 05:50-05:59 +0800，被 API 错误中断；续跑约 08:15-08:50 +0800）
- **Started:** 2026-10-16T21:50:00Z（首个 executor）
- **Completed:** 2026-10-17T00:47:13Z
- **Tasks:** 2（另含编排器追加的 float 修复）
- **Files modified:** 18

## Accomplishments

- `--server-config` 的引擎选择：`trino` 用 ontology 身份的 TrinoEngine，`sqlite` 与不给配置时相同走 fixture 替身；没有任何指向现成 SQLite 库文件的入口（E1）。
- 退出码 3 覆盖不可达、认证失败、超时、协议与查询错误；不可达 / 认证失败时 JSON 只有一条 ONT-V19，human 模式 stderr 为 `online check could not complete: <原因>`，不含 `ok:`。
- 单 catalog 的 `PERMISSION_DENIED` 只让该 catalog 的绑定得到 V19，另一 catalog 的 V17 照常输出（E3）。
- `--check-pk`：两条查询都由 printer AST 构造，快照固定为 `... WHERE "pk" IS NOT NULL GROUP BY "pk" HAVING count(*) > 1 ORDER BY count(*) DESC NULLS LAST, "pk" ASC NULLS LAST LIMIT 21` 与 `SELECT count(*) AS "n" ... WHERE "pk" IS NULL`；NULL 主键不会成为一个「值为 NULL 的重复项」（E17）。
- 只扫描没有 V17 error、没有 V19 的 object 绑定；不带 `--check-pk` 时不发出任何 `GROUP BY` 查询（ADR-0015）。

## Task Commits

首个 executor（被 API 错误中断前）提交：

1. **float 修复 RED** - `4176db9` (test)：fixture 模式 float 属性的失败测试
2. **float 修复 GREEN** - `7ad4949` (fix)：编排器追加的修复，见 Deviations 1
3. **Task 1 RED** - `8f6fd38` (test)：`cli_online_trino.rs` 12 个失败测试

续跑 executor 提交：

4. **Task 1 GREEN** - `61912a8` (feat)：审查并补完首个 executor 留下的未提交 diff（`cli.rs`、`lib.rs`、`online.rs`）
5. **Task 2 RED** - `9bd5da9` (test)：`pk_check.rs` 与 `cli_check_pk.rs` 失败测试，`pk_check` 模块只含签名
6. **Task 2 GREEN** - `a4b3a4c` (feat)：`check_primary_keys` 与 CLI 编排

## TDD Gate Compliance

- Task 1：RED `8f6fd38` → GREEN `61912a8`。RED 时 `cli_online_trino` 因 `--server-config` 未知参数（clap 退出码 2）在各测试的退出码断言上失败。
- Task 2：RED `9bd5da9` → GREEN `a4b3a4c`。RED 证据：`cargo test -p ontoforge-engine --test pk_check` 5 failed（`pk_queries_are_printed`、`duplicates_and_nulls_found`、`twenty_groups_are_not_more`、`nulls_are_not_duplicates`、`display_value_is_short_text`，均为断言失败）；`cargo test -p ontoforge --test cli_check_pk` 4 failed（`check_pk_reports_duplicates` 第 208 行、`check_pk_reports_null_keys` 第 232 行、`check_pk_on_trino` 第 271 行、`exit_code_table` 第 362 行）。先通过的 3 个是防回归守卫（aviation 干净、未给 flag 不扫描、不匹配的绑定不扫描）。
- `gsd-tools check tdd-red-evidence` 只解析 node TAP 输出，不适用于 `cargo test`，因此 RED 证据记录在此而非经该工具校验。
- 无 REFACTOR 提交（无需要）。

## Files Created/Modified

- `crates/ontoforge/src/online.rs` - 引擎选择（`select_backend`）、V20 映射（`config_diagnostic`）、fixture 被忽略的 V17、`compare_all` / `check_keys` / `pk_diagnostics`、`engine_failure` 消息分类
- `crates/ontoforge/src/cli.rs` - `--check-pk`、`--server-config`（都 `requires = "online"`）
- `crates/ontoforge/src/lib.rs` - 退出码 3 时的 `engine_summary`
- `crates/ontoforge-engine/src/pk_check.rs` - 主键查重 / NULL 计数
- `crates/ontoforge-engine/src/sqlite/fixture.rs` - float 报告 real（Deviation 1）
- `crates/ontoforge/tests/cli_online_trino.rs`、`crates/ontoforge/tests/cli_check_pk.rs`、`crates/ontoforge-engine/tests/pk_check.rs` - 测试与快照

## Decisions Made

见 frontmatter `key-decisions`。

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] fixture 模式把 float 属性报告为 double**
- **Found during:** 02-08 之后、本 plan 执行前（编排器追加）
- **Issue:** `crates/ontoforge-engine/src/sqlite/fixture.rs` 把 `float` 与 `double` 映射到同一逻辑类型并报告物理 `double`，02-08 的 D-13 表因此把每个 fixture 模式的 `float` 属性判为 ONT-V17。
- **Fix:** `LogicalType` 增加 `Double`；`Float` → `PhysicalType::Real`，`Double` → `Double`；`docs/source-binding.md` 删除「fixture 模式的验证边界」中的 float 限制说明；`.planning/WINDOWS.md` 第 4 条标为 `fixed`。
- **Files modified:** `crates/ontoforge-engine/src/sqlite/fixture.rs`、`crates/ontoforge-engine/tests/sqlite_engine.rs`、`crates/ontoforge/tests/cli_online.rs`、`docs/source-binding.md`、`.planning/WINDOWS.md`
- **Verification:** `sqlite_engine`、`cli_online`、`doc_type_table` 在 `cargo test --workspace --locked` 中全部通过
- **Committed in:** `4176db9`（RED）、`7ad4949`（GREEN），均由首个 executor 提交

**2. [Rule 1 - Bug] 整轮中止时仍输出「fixture 被忽略」的 V17 warning**
- **Found during:** Task 1 GREEN（续跑审查未提交 diff 时）
- **Issue:** 首个 executor 的实现先生成 fixture warning 再比对，Trino 不可达 / 401 时 JSON 有 V17 warning + V19 两条，违反「JSON 只有 ONT-V19」（`trino_unavailable_exits_3`、`unauthorized_exits_3` 失败）。
- **Fix:** `compare_all` 返回 `Result<_, Diagnostic>`，`Err` 时只输出该 V19。
- **Files modified:** `crates/ontoforge/src/online.rs`
- **Committed in:** `61912a8`

**3. [Rule 1 - Bug] 不可达快照含平台相关文本**
- **Found during:** Task 1 GREEN
- **Issue:** Windows 上连接已释放端口的原因（`deadline has elapsed`）被 miette 折到 `│` 续行，测试只替换了首行，快照会随平台变化。
- **Fix:** 测试的规范化同时丢弃原因的续行；快照只剩 `Trino is unavailable: <reason>`，无端口号。
- **Files modified:** `crates/ontoforge/tests/cli_online_trino.rs`、`crates/ontoforge/tests/snapshots/cli_online_trino__unavailable_human.snap`
- **Committed in:** `61912a8`

**4. [Rule 3 - Blocking] clippy `-D warnings`**
- `duration_suboptimal_units`（`online.rs` 单元测试改用 `Duration::from_secs`，`61912a8`）与 `format_push_string`（`pk_check.rs` 测试改用 `writeln!`，`a4b3a4c`）。

### Acceptance criteria 说明

- `rg -n "sqlite_dir|open_dir" crates` 有 1 处命中：`crates/ontoforge-engine/tests/server_config.rs:224` 的 `profile_engine_has_no_library_path`（02-04 写的负向测试，断言 `sqlite_dir` 作为未知键被拒绝）。它是 E1 的守卫而非入口，代码中不存在库目录模式；未修改该 02-04 文件。

---

**Total deviations:** 4 auto-fixed（3 个 Rule 1、1 个 Rule 3）
**Impact on plan:** 均为正确性或门禁所需，无范围扩张。

## Issues Encountered

- 首个 executor 在 Task 1 GREEN 中途被 API 错误终止，留下未提交的 `cli.rs` / `lib.rs` / `online.rs` diff。续跑时逐段审查该 diff：引擎选择、V20 定位、fixture warning、QueryContext 均符合 plan，保留；只修正 Deviation 2。

## Verification

- `cargo test -p ontoforge --test cli_online_trino`：12 passed，无 `.snap.new`
- `cargo test -p ontoforge --lib --test cli_online --test cli`：7 + 8 + 10 passed
- `cargo test -p ontoforge-engine --test pk_check && cargo test -p ontoforge --test cli_check_pk`：6 passed、7 passed
- `cargo run -q -p ontoforge -- validate examples/ontologies/aviation --online --check-pk --format json --color never`：stdout `[]`，exit 0
- `cargo test --workspace --locked`：exit 0，52 个 test target 全部 ok
- `cargo clippy --workspace --all-targets --locked -- -D warnings`：exit 0
- `cargo fmt --all --check`：通过
- `sql_text_only_from_printer`：1 passed（`pk_check.rs` 不含 SQL 字面量）

## User Setup Required

None - 测试只用 wiremock，无需真实 Trino。

## Next Phase Readiness

- 02-10 可直接使用 `validate --online --server-config` 与 `--check-pk`。
- M:N link 的重复关联未检查（plan 明确不在本 phase）。drift 的定时执行属于 Phase 7。

---
*Phase: 02-source-binding-engine*
*Completed: 2026-10-17*

## Self-Check: PASSED
