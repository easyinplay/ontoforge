# Phase 4 task plan

## Goal

按 `.planning/ROADMAP.md` 的 Phase 4 交付 QUERY-01..10。当前先完成研究、GSD plan-phase、独立计划检查，再进入 execute。详细实现任务以本目录 `04-*-PLAN.md` 为真相源；本文件只跟踪计划与执行顺序。

## Tasks

- [x] P0：隔离 `gsd/phase-04-scoped-l0-read` 分支，核对 Phase 3 UAT 2/2、VERIFICATION 8/8 与新 HEAD CI 三 job；前置依赖：Phase 3 完成。
- [x] P1：从 `.planning/adr/0020-query-execution-rust-orchestrated-overlay.md` ingest 决策到 `04-CONTEXT.md`，关联 scope、edit store 与设计文档；验收：不重开已有 ADR 的决定。
- [x] P2：研究 `crates/ontoforge-query/`、`crates/ontoforge-engine/`、`crates/ontoforge-storage/`、`crates/ontoforge-auth/` 的复用点与风险，产出 `04-RESEARCH.md`、`04-PATTERNS.md` 与 `04-VALIDATION.md` 草案；依赖 P1。
- [x] P3：GSD planner 生成 7 份 `04-*-PLAN.md`，独立 checker 核对 QUERY-01..10、Trino 查询形态、scope 安全与验证命令并给出 `VERIFICATION PASSED`；依赖 P2。
- [x] P4：把 PLAN 的所有权、wave 顺序和验收命令蒸馏到本文件，更新 `progress.md`、`04-VALIDATION.md`、`ROADMAP.md` 与 `.planning/STATE.md`；依赖 P3。

## Execution Order

| Wave | Plan | 主文件所有权 | 验收入口 |
|------|------|--------------|----------|
| 1 | `04-01-PLAN.md` | `crates/ontoforge-query/src/lib.rs`、`crates/ontoforge-storage/src/query_read.rs`、`tests/pk_read.rs`、`tests/security.rs` | `cargo test -p ontoforge-query --test pk_read --test security --locked` |
| 2 | `04-02-PLAN.md` | `crates/ontoforge-query/src/{ast,compile}.rs`、`crates/ontoforge-engine/src/printer/`、`tests/sqlite_query.rs`、`tests/sql_shape.rs` | `cargo test -p ontoforge-query --test sqlite_query --test sql_shape --locked` |
| 2 | `04-03-PLAN.md` | `crates/ontoforge-storage/src/query_read.rs`、`tests/query_read.rs`、`tests/query_read_pg.rs` | `cargo test -p ontoforge-storage --test query_read --locked` |
| 3 | `04-04-PLAN.md` | `crates/ontoforge-query/src/{overlay,execute,cursor}.rs`、`tests/overlay_prop.rs` | `cargo test -p ontoforge-query --test overlay_prop --test security --locked` |
| 4 | `04-05-PLAN.md` | `crates/ontoforge-query/src/{link,set}.rs`、`tests/search_around.rs`、`tests/set_ops.rs` | `cargo test -p ontoforge-query --test search_around --test set_ops --locked` |
| 5 | `04-06-PLAN.md` | `crates/ontoforge-query/src/aggregate.rs`、`tests/aggregate.rs` | `cargo test -p ontoforge-query --test aggregate --test sql_shape --test security --locked` |
| 6 | `04-07-PLAN.md` | `tests/trino_query_it.rs`、`tests/query_bench_it.rs`、`.github/workflows/ci.yml`、`04-PERFORMANCE.md` | Linux CI 实跑 Trino/PG ignored tests、distributed `EXPLAIN`、isolated p95 |

同一 wave 的 `04-02` 与 `04-03` 分别持有 query/compiler 和 storage/read 文件；共享 `lib.rs` 接线只由 `04-02` 主导，`04-03` 只改 storage。各 plan 的逐 task 文件清单和 RED 测试是真相源。

## Execution Progress

- [x] Wave 1 `04-01`：scoped pk tracer 与安全边界；RED、GREEN、SUMMARY 和聚焦测试完成。
- [x] Wave 2 `04-02`、`04-03`：typed compiler/printer 与 storage 只读批量 API。
- [x] Wave 3 `04-04`：Rust overlay、refill、cursor 与预算；SUMMARY、独立复审和聚焦测试通过。
- [ ] Wave 4 `04-05`：searchAround 与集合运算。
- [ ] Wave 5 `04-06`：exact scoped 聚合。
- [ ] Wave 6 `04-07`：真实 Trino/PG、distributed EXPLAIN 与 isolated p95。

## Acceptance

- 每条 QUERY-01..10 都有计划与可执行验证；核心叠加和鉴权逻辑标记 TDD。
- 计划含 production tracer、SQLite 与真实 Trino 的测试边界、越权 conformance、延迟预算与失败处理。
- 独立 plan-checker 通过；ROADMAP 只保留 phase 概览和 plan 指针，STATE 保持 digest。

## Next Step

按 `04-05-PLAN.md` 已对齐D19的两端scope + FK属性可见合同继续 Wave 4；set 和 M:N 纯 edge overlay 已提交至 `aa6a112`，FK/M:N 对外遍历、target hydration 和集成验收仍待完成。真实 PG parity、Trino `EXPLAIN` 和 isolated p95 仍由 Wave 6 验收。
