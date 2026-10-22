---
phase: 04-scoped-l0-read
plan: 01
subsystem: query
tags: [rust, sqlite, scope, overlay, hmac]
requires:
  - phase: 03-edit-store
    provides: EditStore、ActiveDefinition 与对象 patch 物理格式
provides:
  - scoped pk ObjectSet 读取与唯一的 Rust source/edit 叠加路径
  - 无副作用的 EditStore::read_object_edit
  - HMAC _version、统一 outward 错误和最终行/字节限界
affects: [04-02, 04-03, 04-04, 04-05, 04-06, 04-07]
actuals:
  tokens: 14576
  tasks: 2
  commits: 4
commits: 4
plan_head_before: 26a5f492ef49a2872364d6165bdb14b62cd90439
plan_head_after: 1208ac0e988abe20d495aba3078b44e229543492
tech-stack:
  added: []
  patterns: [typed single-table Select, edit-first read, Rust overlay, post-overlay scope, HS256 version]
key-files:
  created:
    - crates/ontoforge-storage/src/query_read.rs
    - crates/ontoforge-query/tests/pk_read.rs
    - crates/ontoforge-query/tests/security.rs
  modified:
    - Cargo.lock
    - crates/ontoforge-query/Cargo.toml
    - crates/ontoforge-query/src/lib.rs
    - crates/ontoforge-storage/src/commit.rs
    - crates/ontoforge-storage/src/lib.rs
    - crates/ontoforge-storage/src/sql_exit.rs
key-decisions:
  - source-backed 对象只有完整 patch 可在 source 缺失时返回；部分 patch 缺 source 统一 NotFound
  - TokenCodec 只输出 HS256 MAC，验证时由调用方重新提供完整 canonical context
patterns-established:
  - edit-first pk lookup 仅在需要 source 列时执行 typed LIMIT 2 查询
  - 在合并行上执行 TypeScope::evaluate，再投影并计算完整 ObjectRow 的字节限界
requirements-completed: [QUERY-01, QUERY-08, QUERY-09]
coverage:
  - id: PK-READ
    description: scoped pk 读取、source/edit 叠加与无副作用 edit 查询
    requirement: QUERY-01
    verification:
      - kind: integration
        ref: cargo test -p ontoforge-query --locked
        status: pass
      - kind: unit
        ref: cargo test -p ontoforge-storage --locked query_read
        status: pass
    human_judgment: false
  - id: PK-SECURITY
    description: 探测错误同形、HMAC 绑定与最终结果限界
    requirement: QUERY-09
    verification:
      - kind: integration
        ref: cargo test -p ontoforge-query --locked
        status: pass
    human_judgment: false
duration: 27min
completed: 2026-10-22
status: complete
---

# Phase 4 Plan 01: Scoped PK Read Summary

按 pk 的 L0 查询现已从只读 edit 查询出发，按需执行一条 typed `LIMIT 2` source 查询，在 Rust 合并后应用行列 scope，并返回绑定完整读取上下文的 HMAC `_version`。

## Accomplishments

- `EditStore::read_object_edit` 直接通过只读 pool 查询，缺失 key 不写 version 0 行；source-backed 部分 patch 在 source 缺失时返回 `NotFound`。
- source/edit 叠加后执行 `TypeScope::evaluate`；缺失、tombstone 和越权对象的 outward `NotFound` 相同，重复 source pk 返回 `AmbiguousObject`。
- `TokenCodec::verify_version` 验证 HS256 签名；`QueryPage::bounded` 只返回满足行数和完整 JSON 字节上限的完整行。

## TDD Gate Compliance

- Task 1 RED：`8a33af9` 提交 `pk_read` 行为语料；缺失的 edit/source/overlay 行为在 focused 测试中失败。GREEN：`0a87e7e` 实现只读存储边界、typed source 查询与叠加。
- Task 2 RED：`24ae68e` 提交安全语料；token 验证和结果限界行为失败。GREEN：`1208ac0` 实现验证、错误 code、限界及部分 patch 缺 source 的反例修复。
- 最终 `cargo test -p ontoforge-query --locked`：`pk_read` 10/10，`security` 6/6。`cargo test -p ontoforge-storage --locked query_read`：1/1。`cargo test -p ontoforge-storage --test sql_exit_guard --locked`：1/1。
- `cargo clippy -p ontoforge-query --all-targets --locked -- -D warnings`、`cargo fmt --all --check` 和 `git diff --check` 均通过。

## Task Commits

1. Task 1 RED：`8a33af9`。
2. Task 1 GREEN：`0a87e7e`。
3. Task 2 RED：`24ae68e`。
4. Task 2 GREEN：`1208ac0`。

## Deviations from Plan

- `[Rule 3 - Blocking]` Cargo.lock 更新 `ontoforge-query` 的依赖边；没有新增 package 或变更版本。`--locked` 验证通过。
- `[Rule 2 - Critical]` 为保持只读存储边界，`sql_exit.rs` 增加只读 pool 的 `fetch_cells_read`，`commit.rs` 的 `decode_object` 改为 `pub(crate)` 供查询解码复用。
- `[Rule 1 - Bug]` 部分 patch 在 source 行缺失时曾返回凭空合成的对象；补充 `partial_edit_without_source_is_not_found` RED 反例并修复，最终回归通过。

## Known Stubs

无。计划修改文件未发现 TODO、FIXME、占位结果或跳过的测试。

## Next Phase Readiness

后续 list/link/aggregate 可复用 query contract、只读 edit 边界、scope 后置检查、token 与结果限界。此计划只验证 SQLite stand-in；真实 Trino/PG 和 p95 证据由后续计划负责。

## Self-Check: PASSED

3 个创建的代码/测试文件与 SUMMARY 均存在；4 个任务 commit 均是当前 HEAD 的祖先，`plan_head_before..plan_head_after` 实测为 4 个 commit。
