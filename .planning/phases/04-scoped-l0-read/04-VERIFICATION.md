---
phase: 04-scoped-l0-read
verified: 2026-10-23T00:45:51Z
status: passed
score: 36/36 must-haves verified
covered_files:
  - .github/workflows/ci.yml
  - .planning/phases/04-scoped-l0-read/04-01-PLAN.md
  - .planning/phases/04-scoped-l0-read/04-01-SUMMARY.md
  - .planning/phases/04-scoped-l0-read/04-02-PLAN.md
  - .planning/phases/04-scoped-l0-read/04-02-SUMMARY.md
  - .planning/phases/04-scoped-l0-read/04-03-PLAN.md
  - .planning/phases/04-scoped-l0-read/04-03-SUMMARY.md
  - .planning/phases/04-scoped-l0-read/04-04-PLAN.md
  - .planning/phases/04-scoped-l0-read/04-04-SUMMARY.md
  - .planning/phases/04-scoped-l0-read/04-05-PLAN.md
  - .planning/phases/04-scoped-l0-read/04-05-SUMMARY.md
  - .planning/phases/04-scoped-l0-read/04-06-PLAN.md
  - .planning/phases/04-scoped-l0-read/04-06-SUMMARY.md
  - .planning/phases/04-scoped-l0-read/04-07-PLAN.md
  - .planning/phases/04-scoped-l0-read/04-07-SUMMARY.md
  - ci/pg/query_corpus.sql
  - ci/query-services.sh
  - crates/ontoforge-engine/src/printer/ast.rs
  - crates/ontoforge-engine/src/printer/mod.rs
  - crates/ontoforge-query/src/aggregate.rs
  - crates/ontoforge-query/src/ast.rs
  - crates/ontoforge-query/src/compile.rs
  - crates/ontoforge-query/src/cursor.rs
  - crates/ontoforge-query/src/execute.rs
  - crates/ontoforge-query/src/lib.rs
  - crates/ontoforge-query/src/link.rs
  - crates/ontoforge-query/src/overlay.rs
  - crates/ontoforge-query/src/set.rs
  - crates/ontoforge-query/tests/query_bench_it.rs
  - crates/ontoforge-query/tests/trino_query_it.rs
  - crates/ontoforge-storage/src/query_read.rs
covered_digest: "v3:sha256:7e19156409807f2d6749524e9219007517de8d4088c3ac1ac0460310d9f8143e"
behavior_unverified: 0
overrides_applied: 0
---

# Phase 4: Scoped L0 Read Verification

**Phase Goal:** 调用者只能在自己的 scope 内读取对象；Rust 编排 pk、过滤、分页、遍历、集合与聚合叠加，Trino 只承担单表查询。
**Code SHA:** `c39a971a25bb5ebf410bfdcd41243da4ab45c56c`
**Automated verdict:** passed. 这只证明 Phase 4 库边界及匹配 SHA 的 CI 验收。用户随后委托代理自测；`04-UAT.md` 记录 23/23 通过，Phase 4 由 GSD transition 关闭。

## Goal Achievement

| # | ROADMAP success criterion | Status | Code and behavioral evidence |
|---|---|---|---|
| 1 | 已编辑对象 PG 直读，未编辑对象 Trino pk；tombstone、release、重复 pk 语义正确 | VERIFIED | `QueryService::execute` 先 `read_object_edit`，按需 `read_source`；`pk_read` 的 tombstone/release/ambiguity tests 与真实 `trino_query_it::fixed_semantics_and_cursor_refill` 断言具体值。 |
| 2 | WhereClause、token 搜索、排序及有界 keyset refill 正确 | VERIFIED | `ast.rs` 列全 12 种谓词；`ValidatedQuery::new` 校验类型与复杂度；`execute.rs` 合并候选、补取及 `has_more`；`sqlite_query::every_where_operator_validates_and_evaluates`、`security` cursor/refill 与真实语义 corpus 通过。 |
| 3 | searchAround、集合运算及 exact aggregate 可用 | VERIFIED | `link.rs` 读取 scoped FK/M:N，`set.rs` 先求授权 key 再走 `execute_list_internal`；`aggregate.rs` 对 source partial 做 old/new correction，min/max TopN refill、distinct/groupBy exact 或 typed error；独立 reference tests、真实 aggregate corpus 通过。 |
| 4 | scope、物理名、token 与结果上限安全 | VERIFIED | `overlay.rs::eligible_row` 在合并行上求 `TypeScope::evaluate` 后过滤可见列；typed SQL CASE 遮蔽；`cursor.rs` HMAC 绑定上下文；`QueryError` 公开错误固定；`security` 与真实多 role/physical-error corpus 通过。 |
| 5 | Trino 单表、1-2 fragment，隔离 p95 达标 | VERIFIED | `compile_source_select` 只建一个 `TableRef`；当前 HEAD 的 CI run `39878005620` 四个 job 成功；137 份 distributed EXPLAIN 中 103 份为 1 fragment、34 份为 2 fragments；四路 p95 独立重算均低于预算。 |

**Score:** 5/5 ROADMAP criteria，另 31/31 PLAN truth；合并为 36/36。七份 PLAN 的 truth 数分别为 5、4、4、5、4、5、4。所有涉及状态变化的 truth 均有命名行为测试，未仅凭符号存在判定。

## Required Artifacts and Wiring

GSD `verify.artifacts` 初查 24/26，`verify.key-links` 初查 18/19。三项静态匹配失败经源码核查均已实现并接线，最终为 **26/26 artifacts、19/19 links**：

| Static finding | Manual code check | Verdict |
|---|---|---|
| `EditStore::read_object_edit` export 未匹配 | `query_read.rs:347` 的 `impl EditStore` 公有方法调用 `read_object_edits`；`lib.rs:260` 的 pk 路径调用它。 | WIRED |
| `QueryService::execute_list` export 未匹配 | `execute.rs:398` 的公有方法 dispatch 到 set、searchAround 与常规 list；`set.rs:68` 又回到 `execute_list_internal`。 | WIRED |
| set -> execute 的旧 pattern 未匹配 | PLAN pattern 使用旧名 `evaluate_authorized_keys|execute_pk_batch`；实际 `set.rs:56` 调 `authorized_keys`，`set.rs:68` 调 `execute_list_internal`，目标仍使用正常 scoped hydration。 | WIRED |

真实数据流：`Trino Engine::execute` 的单表 source rows 与 `EditStore` 的 PG edits 进入 Rust overlay，再经过行列 scope、最终投影、游标和输出上限；`query_bench_it.rs` 与 `trino_query_it.rs` 对同一路径调用。未发现固定空结果或孤立实现。

## Requirements and Test Quality

| Requirement | Result | Direct evidence |
|---|---|---|
| QUERY-01 | SATISFIED | `pk_read`, `trino_query_it::fixed_semantics_and_cursor_refill` |
| QUERY-02 | SATISFIED | `sqlite_query` 全运算符 test，真实 `where_search_set_and_relationship_corpus` |
| QUERY-03 | SATISFIED | `sqlite_query` any/all token test，真实搜索 corpus |
| QUERY-04 | SATISFIED | `security` cursor/refill tests，`overlay_prop` reference |
| QUERY-05 | SATISFIED | `search_around` FK/M:N tests，真实 relationship corpus |
| QUERY-06 | SATISFIED | `set_ops` fixed/proptest，真实 set corpus |
| QUERY-07 | SATISFIED | `aggregate` independent reference、TopN、group/distinct tests，真实 aggregate corpus |
| QUERY-08 | SATISFIED | SQLite differential、proptest；真实 PG 2/2、Trino/PG 5/5 |
| QUERY-09 | SATISFIED | `security` 与真实多 role、token replay、physical-error tests |
| QUERY-10 | SATISFIED | `sql_shape`、137 distributed plans、isolated timings |

七份 PLAN 覆盖全部 `QUERY-01..10`，无 orphaned requirement。相关普通测试无 disabled marker；真实后端测试以 `#[ignore]` 隔离，并由 CI 明确 `--ignored` 执行且 discovery guard 确认非零。reference 与 proptest 使用独立集合/叠加/聚合算法及固定期望值，断言具体值和跨页行为；未发现由被测实现生成 expected fixture 的循环测试。CONTEXT 决策覆盖查询为 7/7。

## Behavioral Spot Checks

| Check | Result |
|---|---|
| `cargo test -p ontoforge-query --test pk_read tombstone_is_not_found_and_release_reads_source --locked -- --exact` | 1 passed |
| `cargo test -p ontoforge-query --test sqlite_query every_where_operator_validates_and_evaluates --locked -- --exact` | 1 passed |
| `cargo test -p ontoforge-query --test security cursor_rejects_tamper_and_context_replay --locked -- --exact` | 1 passed |
| `cargo test -p ontoforge-query --test set_ops set_result_reuses_normal_hydration_order_and_cursor --locked -- --exact` | 1 passed |
| `cargo test -p ontoforge-query --test aggregate count_sum_avg_match_full_materialization_reference --locked -- --exact` | 1 passed |

Matching [CI run 39878005620](https://github.com/easyinplay/ontoforge/actions/runs/39878005620) reports `integration`, `isolated-query-bench`, Ubuntu tests and Windows tests successful at current HEAD. Integration logs show `query_read_pg` 2/2, `trino_query_it` 5/5, smoke 1/1; ignored-target discovery lists query targets as 2, 5 and 1 tests. The downloaded isolated JSON SHA256 is `C519EB94854D7F9BDD6CF803A9794DAEB3987C23709E9897D5FEAEF4631D2745`, matching `04-PERFORMANCE.md`.

| Path | Warmups / measured | Independent nearest-rank p95 | Budget | Errors |
|---|---:|---:|---:|---:|
| source pk | 5 / 40 | 101.01 ms | <500 ms | 0 |
| filtered list | 5 / 40 | 456.53 ms | <1000 ms | 0 |
| aggregate | 5 / 40 | 287.81 ms | <1500 ms | 0 |
| edited PG-direct pk | 5 / 40 | 0.76 ms | <50 ms | 0; 0 source calls in 45 samples |

No phase-declared `probe-*.sh` exists. Source/test/CI files scanned for unreferenced `TBD`, `FIXME`, `XXX` and implementation placeholders; none found. `04-SECURITY.md` records 35/35 threats closed for this library and CI boundary; future SQL Server host capability mapping and the separate nightly fixture credential remain explicitly scoped residuals, not verified here.

## Human Verification

N/A for this internal library phase: the defined Phase 4 semantics have direct automated behavioral and real-backend checks. 用户委托代理完成 Test 1 后，`04-UAT.md` 记录 23/23 自动验收通过；`phase uat-passed 4 --require-verification` 返回 true。此处是后续验收结果，不改变本报告原有的代码级验证结论。

## Gaps

No implementation blocker or behavior-unverified truth found. UAT 记录无 gap；后续若发现回归，按新缺口重新验证。

_Verifier: gsd-verifier; 2026-10-23T00:45:51Z_
