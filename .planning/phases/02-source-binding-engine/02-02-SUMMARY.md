---
phase: 02-source-binding-engine
plan: 02
subsystem: engine
tags: [rust, sql-printer, sqlite, sqlx, proptest, sqlparser, injection-defense, engine-trait]

requires:
  - phase: 02-source-binding-engine
    provides: "02-01 的 backing / ProfileName / read_limited / 诊断码前置与全绿基线（312 个测试）"
provides:
  - "新 crate ontoforge-engine：Engine trait（execute / describe_tables / status）、AnyEngine、TableRef、QueryContext、QueryOutput、EngineStatus"
  - "类型化 printer：PrintedSql 由 printer 模块私有构造器保证，quote_string / quote_ident 是值与标识符进入 SQL 文本的唯一出口"
  - "Phase 2 AST 子集（SELECT 列 / count / 聚合、单表 FROM、WHERE、GROUP BY、HAVING、显式 NULLS 的 ORDER BY、LIMIT）与 Trino / SQLite 两种方言"
  - "EngineError / PrintError（Clone），is_engine_wide() 分类；DEFAULT_QUERY_TIMEOUT_MS / DEFAULT_MAX_ROWS / DEFAULT_MAX_BYTES 与 QueryContext::new"
  - "value::{Value, PhysicalType::{parse, render}, PhysicalColumn, Column, DecimalParts, split_decimal}"
  - "SqliteEngine（单连接、流式 execute、按 catalog 的 describe_tables、status）与 crate 内 sqlite_ddl"
  - "本 phase 全部新 workspace 依赖一次声明（reqwest、tokio、sqlx、secrecy、csv、chrono-tz、ulid、wiremock、proptest、sqlparser、futures-util）"
affects: [02-04, 02-05, 02-08, 02-09, 02-10, phase-04]

plan_head_before: 6c9d2afd036562d92b5ac05f465faefc755f006b

actuals:
  tokens: 19800
  tasks: 3
  commits: 5

tech-stack:
  added: [reqwest 0.13.5, tokio 1.53.3, sqlx 0.9.0 (sqlite), secrecy 0.10.3, csv 1.4.0, chrono-tz 0.10.4, ulid 3.0.0, futures-util 0.3.34, wiremock 0.6.5, proptest 1.12.0, sqlparser 0.63.0]
  patterns:
    - "SQL 文本只能来自 printer：PrintedSql::new 不带可见性修饰，只有 printer 模块及其子模块可调用，Engine::execute 只收 PrintedSql"
    - "AST 节点携带原始值，转义只在打印阶段经 quote_string / quote_ident 发生，不存在已转义字符串类型"
    - "替身单连接 + 流式 fetch，读到第 max_rows + 1 行或累计字节超限即停并置 truncated"
    - "sqlx 0.9 的动态 SQL 经 AssertSqlSafe(...).into_sql_str() 传入，PrintedSql 是该断言的类型层依据"

key-files:
  created:
    - crates/ontoforge-engine/Cargo.toml
    - crates/ontoforge-engine/src/lib.rs
    - crates/ontoforge-engine/src/error.rs
    - crates/ontoforge-engine/src/value.rs
    - crates/ontoforge-engine/src/printer/mod.rs
    - crates/ontoforge-engine/src/printer/ast.rs
    - crates/ontoforge-engine/src/printer/escape.rs
    - crates/ontoforge-engine/src/printer/dialect.rs
    - crates/ontoforge-engine/src/printer/sqlite_ddl.rs
    - crates/ontoforge-engine/src/printer/tests.rs
    - crates/ontoforge-engine/src/printer/snapshots/ontoforge_engine__printer__tests__print_trino.snap
    - crates/ontoforge-engine/src/printer/snapshots/ontoforge_engine__printer__tests__print_sqlite.snap
    - crates/ontoforge-engine/src/sqlite/mod.rs
    - crates/ontoforge-engine/src/sqlite/tests.rs
    - crates/ontoforge-engine/tests/sqlite_engine.rs
    - crates/ontoforge-engine/tests/printer_literals.rs
    - crates/ontoforge-engine/tests/printer_prop.rs
    - crates/ontoforge-engine/tests/printer_prop.proptest-regressions
    - crates/ontoforge-engine/tests/sql_text_only_from_printer.rs
  modified:
    - Cargo.toml
    - Cargo.lock

key-decisions:
  - "Engine trait 的方法写成返回 impl Future + Send，实现处用 async fn；SqliteEngine::describe_tables 无 await，用 allow(clippy::unused_async_trait_impl) 并写明原因"
  - "execute 先 prepare 取列元数据再 fetch 行流，零行结果也带列名；列 physical 为 PhysicalType::Other(声明类型名，NULL 型为空串)"
  - "Timestamp 字面量的小数部分接受 1 到 9 位数字（计划写作 .fff），秒为 60 的闰秒显式拒绝"
  - "sqlite_ddl 整个模块 allow(dead_code)，02-05 开始使用后移除"
  - "测试放在 src/printer/tests.rs 与 src/sqlite/tests.rs（cfg(test) 子模块文件），可见性等价于内联 mod tests，未放宽任何可见性"

patterns-established:
  - "多方言 printer 的括号策略：And / Or 的每个操作数加括号，Not 写成 NOT (..)，比较与 IN / IS NULL 的布尔型操作数加括号"
  - "破坏性变异验证：临时去掉转义，确认性质测试失败后用 git checkout 恢复该文件，不提交"

requirements-completed: [ENGINE-03, ENGINE-04]

coverage:
  - id: D1
    description: "Engine trait、AnyEngine、QueryContext 缺省常量、EngineError 的 code 与 is_engine_wide 分类、按 catalog 批量的 describe_tables（跨 catalog 返回 Config）"
    requirement: "ENGINE-04"
    verification:
      - kind: integration
        ref: "crates/ontoforge-engine/tests/sqlite_engine.rs#engine_wide_classification"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/sqlite_engine.rs#query_context_defaults"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/sqlite_engine.rs#describe_tables_rejects_mixed_catalogs"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/sqlite_engine.rs#error_codes_are_upper_snake"
        status: pass
    human_judgment: false
  - id: D2
    description: "PrintedSql 由编译器保证只来自 printer；值与标识符只经 quote_string / quote_ident；NUL、空标识符、非有限 double、非法 decimal / date / timestamp、空 IN / AND / OR / projection 被拒绝"
    requirement: "ENGINE-03"
    verification:
      - kind: integration
        ref: "crates/ontoforge-engine/tests/sqlite_engine.rs#printed_literal_round_trips_through_sqlite"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/printer_literals.rs#invalid_literals_are_rejected"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/printer_literals.rs#empty_lists_are_rejected"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/sql_text_only_from_printer.rs#sql_text_only_from_printer"
        status: pass
    human_judgment: false
  - id: D3
    description: "注入性质：任意字符串作 WHERE 右值时 sqlparser 解析出一条语句、值不变、替换右值后 AST 结构不变；SQLite 回读字面量与标识符不变"
    requirement: "ENGINE-03"
    verification:
      - kind: integration
        ref: "crates/ontoforge-engine/tests/printer_prop.rs#string_literal_keeps_statement_structure"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/printer_prop.rs#string_literal_round_trips_on_sqlite"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/printer_prop.rs#identifier_round_trips_on_sqlite"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/printer_prop.rs#ansi_dialect_has_no_backslash_escape"
        status: pass
    human_judgment: false
  - id: D4
    description: "Trino 与 SQLite 的方言差异（表引用、类型化字面量、显式 NULLS、布尔与 i64::MIN 的写法）由快照与断言固定"
    requirement: "ENGINE-04"
    verification:
      - kind: unit
        ref: "crates/ontoforge-engine/src/printer/tests.rs#print_trino_and_sqlite"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/printer_literals.rs#trino_literals_carry_type_prefixes"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/printer_literals.rs#i64_min_is_printed_without_overflow"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/printer_literals.rs#order_by_always_spells_out_nulls"
        status: pass
    human_judgment: false
  - id: D5
    description: "PhysicalType::parse / render 与 split_decimal 的唯一实现"
    verification:
      - kind: unit
        ref: "crates/ontoforge-engine/src/value.rs#physical_type_parse_table"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-engine/src/value.rs#split_decimal_rules"
        status: pass
    human_judgment: false
  - id: D6
    description: "SQLite 替身单连接、DDL 文本经 printer、流式 execute 按 max_rows / max_bytes 截断，Windows 本机无 Docker 全绿"
    requirement: "ENGINE-04"
    verification:
      - kind: unit
        ref: "crates/ontoforge-engine/src/sqlite/tests.rs#sqlite_ddl_round_trip"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-engine/src/sqlite/tests.rs#max_rows_truncates"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-engine/src/sqlite/tests.rs#max_bytes_truncates"
        status: pass
      - kind: other
        ref: "cargo test --workspace --locked（340 通过、0 失败）"
        status: pass
    human_judgment: false
  - id: D7
    description: "Trino 端的转义语义（U& 转义、超长串、反斜杠）在真实服务上的行为"
    requirement: "ENGINE-03"
    verification: []
    human_judgment: true
    rationale: "本 plan 只能在 SQLite 与 sqlparser 上验证；Trino 端固定语料属 02-10，需要真实 Trino"

duration: 15min
completed: 2026-10-17
status: complete
---

# Phase 2 Plan 02: ontoforge-engine 骨架、类型化 printer 与 SQLite 替身 Summary

**新 crate `ontoforge-engine`：`PrintedSql` 由 printer 模块私有构造器在编译期保证 SQL 只来自类型化 printer，两个转义函数是值与标识符的唯一出口，proptest（sqlparser 结构不变 + SQLite 回读）与源码扫描锁住不变量，单连接流式取数的 SQLite 替身让 Windows 本机无 Docker 跑通全部测试。**

## Performance

- **Duration:** 15 min
- **Started:** 2026-10-16T18:48:35Z
- **Completed:** 2026-10-16T19:03:17Z（实现与最终门；SUMMARY 提交在其后）
- **Tasks:** 3（Task 1 tracer、Task 2 auto/tdd、Task 3 auto/tdd）
- **Files modified:** 21（相对 `6c9d2af`，含 `Cargo.lock`）

## Accomplishments

- `Engine` trait 一开始就是按 catalog 批量的 `describe_tables`（E22）；`EngineError` 为 `Clone`，`is_engine_wide()` 把 Unavailable / Unauthorized / Cancelled / Config 与其余变体分开（E3）；三个 `DEFAULT_*` 常量是查询上限缺省的唯一定义（E11）。
- `PrintedSql::new` 不带可见性修饰，printer 之外无法构造（E4）；`quote_string` / `quote_ident` 为 `pub(super)`；`sqlite_ddl` 为 `pub(crate)`；`tests/sql_text_only_from_printer.rs` 扫描 `src/printer/` 之外的 SQL 关键字字面量与 `PrintedSql::new(` 调用，并先验证扫描器自身能发现违规。
- printer 覆盖 information_schema 查询、`--check-pk` 查询与替身 DDL 所需的形态；快照固定两种方言：Trino `"demo"."aviation"."flights"` 与 `DECIMAL '1.50'`，SQLite `"demo__aviation"."flights"` 与 `'1.50'`；ORDER BY 一律显式 NULLS；`i64::MIN` 打印为 `(-9223372036854775807 - 1)`。
- `value::split_decimal` 是 decimal 文本语法的唯一实现，printer 的字面量校验已经使用它，02-05 的 fixture 转换复用（E7）。
- 替身 `execute` 用 sqlx `fetch` 行流，读到第 `max_rows + 1` 行或累计 TEXT / BLOB 字节超限即停；源码中没有 `fetch_all`（E24）。
- 全部新 workspace 依赖一次声明，后续 plan 不再改根依赖表；`cargo test --workspace --locked` 340 通过（基线 312，新增 28）。

## Task Commits

1. **Task 1: 端到端 tracer（tdd）**
   - RED `075db99`（test）：8 个行为测试因断言失败（存根 print 返回空串、`code()` 返回空串、缺省值为 0、`status` 为 Unavailable 等）
   - GREEN `6e3b6ad`（feat）
2. **Task 2: 完整 AST 子集、方言、PhysicalType、SQLite DDL（tdd）**
   - RED `8a12c7a`（test）：13 个测试失败（`print_trino_and_sqlite`、`physical_type_parse_table`、`split_decimal_rules`、`sqlite_ddl_round_trip`、`max_rows_truncates`、`max_bytes_truncates` 与 `printer_literals` 的 7 个）
   - GREEN `3f9a8b3`（feat，含两份快照）
3. **Task 3: proptest 与源码守护（tdd）** - `81b9de8`（test），验证方式见下面的 TDD Gate Compliance

**Plan metadata:** 本文件所在的 `docs(02-02)` 提交。无 REFACTOR 提交。

## Files Created/Modified

- `crates/ontoforge-engine/src/lib.rs` - Engine trait、AnyEngine、TableRef、QueryContext 与缺省常量
- `crates/ontoforge-engine/src/error.rs` - EngineError、PrintError、`code()`、`is_engine_wide()`
- `crates/ontoforge-engine/src/value.rs` - Value、PhysicalType（parse / render）、PhysicalColumn、Column、split_decimal
- `crates/ontoforge-engine/src/printer/{mod,ast,escape,dialect,sqlite_ddl}.rs` - printer 全部
- `crates/ontoforge-engine/src/sqlite/mod.rs` - SqliteEngine
- `crates/ontoforge-engine/tests/{sqlite_engine,printer_literals,printer_prop,sql_text_only_from_printer}.rs` - 集成与性质测试
- `Cargo.toml`、`Cargo.lock` - 本 phase 全部新依赖

## Decisions Made

- `Engine` trait 方法返回 `impl Future<Output = ..> + Send`，实现处用 `async fn`；`describe_tables` 在替身里没有 `.await`，加 `allow(clippy::unused_async_trait_impl)` 并写明原因，不为过 lint 改签名。
- `execute` 先 `prepare` 取列信息，再对同一语句 `fetch`；这样零行结果也返回列名（Trino 的行为），而不是只在有行时才知道列。
- Timestamp 字面量小数位放宽到 1 到 9 位（计划 `.fff`），避免 02-05 的 fixture 在不同精度下被误拒；校验仍是全数字且经 `quote_string` 输出。秒为 60 显式拒绝，chrono 会把它当闰秒接受而 Trino 不接受。
- `chrono-tz` 以 `default-features = false` 声明且能解析、能编译，但本 plan 没有使用它，`FromStr` 是否可用留给第一个使用它的 plan 验证（RESEARCH A2）。
- `futures-util` 以 `default-features = false` 声明，`TryStreamExt::try_next` 可用，无需默认 feature。

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - 阻塞] clippy pedantic 两处**
- **Found during:** Task 1 GREEN 的 clippy 门
- **Issue:** `format_push_string`（`sql.push_str(&format!(..))`）与 `unused_async_trait_impl`（`describe_tables` 无 await）在 `-D warnings` 下失败。
- **Fix:** LIMIT 改为 `push_str` 两步；`describe_tables` 加带原因的 `allow`。
- **Files modified:** `printer/dialect.rs`、`sqlite/mod.rs`
- **Committed in:** `6e3b6ad`

**2. [Rule 1 - 计划前提不成立] SQLite 3.51.3 把裸写的 `-9223372036854775808` 读成整数**
- **Found during:** Task 2 RED
- **Issue:** 计划称裸写会在 SQLite 变成 REAL。`i64_min_reads_back_as_integer_on_sqlite` 在存根（输出裸数字）下就通过，属意外 GREEN，不能证明 printer 的特殊写法。
- **Fix:** 保留该回读测试作回归，另加 `i64_min_is_printed_without_overflow` 断言打印文本为 `(-9223372036854775807 - 1)`，后者在 RED 阶段失败。特殊写法本身对 Trino 才是必需的。
- **Files modified:** `tests/printer_literals.rs`
- **Committed in:** `8a12c7a`

**3. [Rule 3 - 阻塞] RED 证据格式**
- **Found during:** Task 1、Task 2 RED
- **Issue:** `gsd_run check tdd-red-evidence` 解析 node TAP 输出，cargo 输出不是 TAP（与 02-01 相同）。
- **Fix:** 人工核对：每个目标测试都因对计划行为的断言失败，而非编译错误或加载崩溃；Task 1、Task 2 为让测试可编译先提交了签名存根。Task 2 的 `max_rows_truncates` 与 `max_bytes_truncates` 在 RED 时因存根 DDL 不建表而在 `execute` 报 `no such table`，不是对截断逻辑的断言失败，截断逻辑本身在 Task 1 已实现，这两个测试在 Task 2 GREEN 后才首次有意义地通过。
- **Verification:** RED 提交前 `cargo test` 输出已逐条检查。

**4. [Rule 3 - 范围对齐] 测试文件组织**
- **Found during:** Task 2
- **Issue:** 计划写 `#[cfg(test)] mod tests`（内联）。长测试内联会让 `printer/mod.rs`、`sqlite/mod.rs` 膨胀，且 shell 里追加大段文本不稳。
- **Fix:** 用 `#[cfg(test)] mod tests;` 指向 `printer/tests.rs` 与 `sqlite/tests.rs`；子模块可见性与内联相同，没有放宽任何可见性。新增 `tests/printer_literals.rs` 放不需要 crate 内可见性的方言与字面量测试。
- **Committed in:** `8a12c7a`

**5. [Rule 2 - 补充] Task 1 临时 `PrintError::Unsupported` 变体**
- **Found during:** Task 1 步骤 4
- **Issue:** 计划要求未支持分支返回明确的 `PrintError`，而 interfaces 的变体表里没有对应项。
- **Fix:** Task 1 临时加 `Unsupported(&'static str)`，Task 2 补齐全部分支后删除，最终公开的 `PrintError` 与 interfaces 一致。
- **Committed in:** `6e3b6ad` 加入，`3f9a8b3` 删除

---

**Total deviations:** 5（1 个 lint 阻塞、1 个计划前提修正、1 个 RED 证据处理、1 个文件组织、1 个临时变体）
**Impact on plan:** 无范围蔓延；公开 API 与 interfaces 一致。

## TDD Gate Compliance

- Task 1：RED `075db99`（`test(02-02)`）→ GREEN `6e3b6ad`（`feat(02-02)`）。
- Task 2：RED `8a12c7a` → GREEN `3f9a8b3`。
- Task 3：只有 `81b9de8`（`test(02-02)`），没有后随的 `feat`。这是因为 Task 3 的被测行为（转义、`PrintedSql` 私有性）已在 Task 1、2 实现，测试一写出就通过，按 tdd.md 的“意外 GREEN”规则做了调查而不是跳过：计划本身要求用故意不转义的临时实现验证。结果：去掉 `quote_ident` 的引号加倍后 `identifier_round_trips_on_sqlite` 失败（最小反例 `"` ）；再去掉 `quote_string` 的引号加倍后 `string_literal_keeps_statement_structure`、`string_literal_round_trips_on_sqlite`、`identifier_round_trips_on_sqlite` 三项都失败（最小反例 `E'`）；往 `src/` 放一个含 `"DROP TABLE x"` 的临时文件后 `sql_text_only_from_printer` 失败并列出 `zz_violation.rs:1`。三处临时改动均已撤销（`git checkout -- escape.rs`、删除临时文件），`printer_prop.proptest-regressions` 恢复为只含注释头的版本，变异产生的种子未提交。
- 无 REFACTOR 提交。

## Issues Encountered

- 在这台机器上 bash 里的大段 heredoc 偶发 `unexpected EOF while looking for matching` 解析错误（含单引号的长命令），此时整条命令不执行；改用 Write / Edit 工具创建文件，没有产生半写入文件。
- Windows 上 `cargo test --workspace` 较慢，使用较长超时运行，结果不受影响。

## Known Stubs

- `crates/ontoforge-engine/src/sqlite/mod.rs`：`SqliteEngine.describe_tables` 从一个内存元数据 map 回答，目前没有任何代码写入该 map，所以对任何非空输入都返回空 map（“表不存在”）。这是有意的：fixture 加载与元数据写入属 02-05（`from_fixtures`），本 plan 的目标（Engine 契约与 printer）不依赖它。已登记到 `.planning/WINDOWS.md`。
- `crates/ontoforge-engine/src/printer/sqlite_ddl.rs`：模块级 `allow(dead_code)`，02-05 开始在非测试代码里使用后移除。已登记到 `.planning/WINDOWS.md`。

## Threat Flags

None。T-02-04 / T-02-05 / T-02-06 / T-02-SC 的缓解均已落地：转义出口与 `PrintedSql` 私有性、数值与时间字面量校验、流式有界取数、只用 RESEARCH 审计为 OK 的依赖与 `Cargo.lock` 入库（本 plan 全程 `--locked` 通过）。本 plan 没有新增网络端点、认证路径或文件访问；`reqwest`、`secrecy`、`csv`、`chrono-tz` 已声明但尚未被代码使用。

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- 02-04（Trino client）可直接实现 `Engine`，`AnyEngine` 追加 `Trino` 变体；`TrinoConfig` 缺省引用 `DEFAULT_*`。
- 02-05（fixture 加载与在线校验）使用 `sqlite_ddl`（届时去掉 `allow(dead_code)`）、`split_decimal`、`describe_tables` 的按 catalog 批量契约，并用 `is_engine_wide()` 分流。
- 02-09（`--check-pk`）用现有 printer 子集拼查询。
- 待后续覆盖：Trino 端的转义固定语料（U& 转义、超长串）属 02-10，Windows 本机无法覆盖；`chrono-tz` 的 `default-features = false` 能否支持 `Tz::from_str` 需在第一个使用它的 plan 里确认。

## Self-Check: PASSED

- 创建的文件（`ontoforge-engine` 下 19 个，含两份 `.snap`）均存在于磁盘；`git ls-files crates/ontoforge-engine` 与 key-files 对应。
- 提交 `075db99`、`6e3b6ad`、`8a12c7a`、`3f9a8b3`、`81b9de8` 均存在，`git log --grep=02-02` 返回 5 条；`git diff --diff-filter=D` 对每个提交无意外删除。
- Task 1、Task 2 的 acceptance 逐条重跑通过（`cargo metadata` 三条 jq、`rg` 正负向检查、快照内容、`physical_type_parse_table` / `max_rows_truncates` / `max_bytes_truncates` 各 `1 passed`）；Task 3 acceptance：`printer_prop` 5 个测试通过（要求 ≥ 4），`sql_text_only_from_printer` `1 passed`。
- 最终门：`cargo test --workspace --locked`（340 通过、0 失败）、`cargo clippy --workspace --all-targets --locked -- -D warnings`、`cargo fmt --all --check` 全部通过；无 `.snap.new` 残留。

---
*Phase: 02-source-binding-engine*
*Completed: 2026-10-17*
