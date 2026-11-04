---
phase: 03-edit-store-scope-model
plan: 21
subsystem: storage
tags: [decimal, serde_json, postgres, sqlite, conformance, validation, sqlx, edit-log]
gap_closure: true

requires:
  - phase: 03-edit-store-scope-model
    provides: "PropertyValue / commit / import / conformance 骨架（03-08、03-19）"
provides:
  - "decimal 的 JSON 数字只收整数，带小数的数字以 ValueError 拒绝，不再经 f64 舍入"
  - "PostgreSQL 存不下的值（U+0000、0 年 date / timestamp、UTC 越界）在校验阶段由两个后端同样拒绝"
  - "pub fn pg_connect_options 固定 DateStyle、TimeZone、extra_float_digits 与 search_path"
affects: [03-22, 03-23, phase-5-rest, phase-7-mcp]

plan_head_before: 0b749cdac37df56b6b4f52fa9caf108a4f92217d

actuals:
  tokens: 6800
  tasks: 3
  commits: 6

tech-stack:
  added: []
  patterns:
    - "值校验取 SQLite 与 PostgreSQL 值域的交集，不按后端分支"
    - "连接选项提取为 pub fn（pg_connect_options），测试与 open_postgres 共用同一组选项"

key-files:
  created: []
  modified:
    - crates/ontoforge-storage/src/value.rs
    - crates/ontoforge-storage/src/commit.rs
    - crates/ontoforge-storage/src/store.rs
    - crates/ontoforge-storage/tests/import.rs
    - crates/ontoforge-storage/tests/conformance/mod.rs
    - crates/ontoforge-storage/tests/conformance_pg.rs
    - crates/ontoforge-storage/tests/support/mod.rs
    - docs/edit-store.md

key-decisions:
  - "decimal 输入规则：字符串或 is_i64 / is_u64 的 JSON 数字；不打开工作区级 serde_json 任意精度 feature（plan 明确排除）"
  - "NUL 与年份范围按两后端交集校验；timestamp 上界取 9999 年以保持规范文本为 4 位年份"
  - "WR-A03 按实测归类为 disproved，固定只作纵深防御"

requirements-completed: [STORE-03, STORE-06, STORE-01]

coverage:
  - id: D1
    description: "decimal 的带小数 JSON 数字被拒绝并提示写成字符串；整数 JSON 数字与字符串逐字保存"
    requirement: STORE-03
    verification:
      - kind: unit
        ref: "crates/ontoforge-storage/src/value.rs#decimal_json_numbers_must_be_exact_integers"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/import.rs#json_decimals_are_never_rounded"
        status: pass
    human_judgment: false
  - id: D2
    description: "17 位以上有效数字的 decimal 在 edit 行、日志 payload、历史快照中逐字一致（SQLite 与 PostgreSQL）"
    requirement: STORE-06
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/conformance/mod.rs#every_value_type_round_trips (conformance_sqlite, conformance_pg)"
        status: pass
    human_judgment: false
  - id: D3
    description: "PostgreSQL 存不下的值在校验阶段以 EditError::Value 拒绝，两后端结论一致；import 对含 NUL 的文件整体拒绝"
    requirement: STORE-01
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/conformance/mod.rs#values_postgres_cannot_store_are_refused_on_every_backend (conformance_sqlite, conformance_pg)"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-storage/src/value.rs#postgres_limits_are_validated"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/import.rs#nul_in_a_cell_is_refused_before_anything_is_written"
        status: pass
    human_judgment: false
  - id: D4
    description: "open_postgres 的连接选项显式固定会话设置，守卫测试在冲突的数据库级缺省下直接读到固定值"
    requirement: STORE-01
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/conformance_pg.rs#pg_sessions_use_the_pinned_settings"
        status: pass
    human_judgment: false

duration: 约 45min
completed: 2026-10-21
status: complete
---

# Phase 3 Plan 21: decimal 保真与 PostgreSQL 值域校验 Summary

**decimal 的 JSON 数字只收整数（带小数的数字要求写成字符串），NUL 与 0 年 date / timestamp 在两后端校验阶段同样被拒绝，PG 连接显式固定 DateStyle / TimeZone / extra_float_digits**

## Performance

- **Duration:** 约 45 min
- **Completed:** 2026-10-21T07:55Z
- **Tasks:** 3（均为 TDD，各有 RED 与 GREEN 提交）
- **Files modified:** 8

## Accomplishments

- gap 1（CR-A01）关闭：`from_json_as` 的 Decimal 分支只接受字符串或 `is_i64` / `is_u64` 的数字。未修复时 `12345678.1234567891` 被 f64 舍成 `12345678.1234567900` 后通过校验并写进只能追加的日志。`docs/edit-store.md` 的 import 一节补了 decimal 写法。
- WR-A01 关闭：`canonical_as` 拒绝含 U+0000 的字符串（属性值、字符串主键、数组元素同一分支），`date()` 拒绝 0 年，`timestamp()` 换算 UTC 后要求年份在 0001..9999；`check_shape` 拒绝 action、principal actor、params 的字符串值与对象键中的 U+0000，`json_contains_nul` 用显式栈迭代。未修复时 SQLite 接受这些提交，PostgreSQL 返回 `Store(Query { sqlstate: "22021" })`；修复后两个后端同一 conformance 用例都返回 `EditError::Value`。
- WR-A03 按实测处理（见下节），`pg_connect_options` 提取为 `pub fn`，与 `open_postgres` 共用。

## WR-A03 实测

未固定（`pg_connect_options` 仅设 `search_path`）时，守卫测试在 embedded PG 上、数据库级缺省为 `DateStyle 'SQL, DMY'`、`extra_float_digits 0`、`TimeZone 'America/New_York'` 的专用数据库里读到：

unpinned SHOW DateStyle = ISO, MDY
unpinned SHOW extra_float_digits = 2
unpinned SHOW TimeZone = UTC

对照连接（`extra_float_digits(None)`，即不发送 sqlx 的启动参数）上 `SHOW extra_float_digits` 为 `0`，证明数据库级缺省确实生效，所以上面三行是 sqlx 启动参数压过 `ALTER DATABASE` 的结果。`search_path` 也与 `schema_sys` 一致。未固定运行唯一失败的断言是 `extra_float_digits`（实际 2，期望 3），而 2 是 sqlx 的缺省值，对 PostgreSQL 12 及以上与 3 一样输出最短往返值，按「已不冲突」处理。

WR-A03: disproved

审查描述的失败（数据库级缺省改写 date 与浮点数组的 text 读回）在锁定的 sqlx 0.9.0 下不会发生：启动包的 `DateStyle`、`TimeZone` 与 `extra_float_digits = 2`（sqlx-postgres `connection/establish.rs` 第 25-38 行、`options/mod.rs` 第 97 行）已压过数据库级缺省。本 plan 的固定只算纵深防御（不再依赖 sqlx 的缺省值），守卫测试不作为该缺陷的回归证明引用。固定后守卫测试通过：`DateStyle = ISO, MDY`、`extra_float_digits = 3`、`TimeZone = UTC`、`search_path = schema_sys`。

## Task Commits

1. **Task 1（tracer）: decimal 保真** - RED `0e892fa` (test)，GREEN `299a86a` (fix)
2. **Task 2: PG 存不下的值** - RED `49060ac` (test)，GREEN `3b71e5f` (fix)
3. **Task 3: PG 会话设置固定** - 观察运行 `2e52daf` (test)，GREEN `af495d4` (fix)

**Plan metadata:** 见随后的 `docs(03-21)` 提交。

RED 证据：Task 1 的 value 单测因 `from_json` 返回 `Decimal("12345678.1234567900")` 而在 `expect_err` 处失败，import 测试因导入成功（期望数据问题）而失败；Task 2 的 value 单测、import 测试与 SQLite conformance 因提交被接受而失败，PG conformance 因 `Store(Query { sqlstate: Some("22021"), message: "invalid byte sequence for encoding \"UTF8\": 0x00" })` 而失败；Task 3 见上节。Task 1 的 conformance 扩展按 plan 在到达时就通过（字符串路径本来逐字保存），是两后端的保真守卫。

## Files Created/Modified

- `crates/ontoforge-storage/src/value.rs` - decimal JSON 数字规则、NUL 与年份范围校验、单测
- `crates/ontoforge-storage/src/commit.rs` - `check_shape` 的 NUL 检查与 `json_contains_nul`
- `crates/ontoforge-storage/src/store.rs` - `pg_connect_options`，固定会话设置
- `crates/ontoforge-storage/tests/conformance/mod.rs` - `Gate.amount` decimal(38, 10)、新用例 `values_postgres_cannot_store_are_refused_on_every_backend`
- `crates/ontoforge-storage/tests/conformance_pg.rs` - `pg_sessions_use_the_pinned_settings`
- `crates/ontoforge-storage/tests/import.rs` - `json_decimals_are_never_rounded`、`nul_in_a_cell_is_refused_before_anything_is_written`
- `crates/ontoforge-storage/tests/support/mod.rs` - `PgTestStore::url()`
- `docs/edit-store.md` - import 一节的 decimal 写法说明

## Decisions Made

- decimal 只收整数 JSON 数字；放宽（任意精度 feature）是向后兼容的，留给以后。
- timestamp 上界 9999 年是为了保持规范文本 4 位年份（ADR-0026 第 2 节），下界是 PostgreSQL 没有 0 年。
- 守卫测试的 `show` 辅助接收完整的 `SHOW ...` 静态 SQL，使每条断言在源码中可检索。

## Deviations from Plan

None - plan executed exactly as written.

（`cargo fmt` 在 Task 2 GREEN 时改动了 RED 提交里的 `tests/conformance/mod.rs` 排版，随 GREEN 提交一并入库；这是格式化，不是计划外工作。）

## Issues Encountered

- 第一次 `cargo test --workspace --locked` 在 `ontoforge-engine --test drift` 失败后中止；单独重跑该 target 8 个测试全过，再用 `--no-fail-fast` 跑整个工作区无失败项。判断为与本 plan 无关的偶发（该 crate 未被本 plan 修改），未能复现，未做根因调查。

## Verification Results

- `cargo test -p ontoforge-storage --locked`：全部 target ok（lib 48 passed，conformance_sqlite 28 passed，import 16 passed 等）
- `cargo test -p ontoforge-storage --features embedded-pg --locked --test conformance_pg --test import -- --ignored`：conformance_pg 29 passed，import 2 passed
- `cargo test --workspace --locked --no-fail-fast`：无 FAILED
- `cargo clippy --workspace --all-targets --locked -- -D warnings` 与 `cargo clippy -p ontoforge-storage --features embedded-pg --all-targets --locked -- -D warnings`：通过
- `cargo fmt --all --check`：通过
- `rg --glob Cargo.toml arbitrary_precision`：无匹配

## Known Stubs

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

gap 1 与 WR-A01 / WR-A03 已关闭；03-22、03-23 不依赖本 plan 的接口变化。

## Self-Check: PASSED

- 8 个修改文件均存在；6 个提交（`0e892fa`、`299a86a`、`49060ac`、`3b71e5f`、`2e52daf`、`af495d4`）均在 `git log` 中。
- TDD 门：每个 task 的 `test(03-21)` 提交先于对应的 `fix(03-21)` 提交。
