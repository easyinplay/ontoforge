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
- [x] Wave 4 `04-05`：searchAround 与集合运算；SUMMARY、独立复审及聚焦测试通过。
- [x] Wave 5 `04-06`：exact scoped 聚合；SUMMARY、主 session 聚焦复跑通过。
- [x] Wave 6 `04-07`：真实 Trino/PG、distributed EXPLAIN 与 isolated p95；匹配commit的Linux CI run 39818657834、隔离artifact和四项阈值已核验。

## Security Remediation

- [x] S1 (T-04-22/08)：外部 crate 的两条 `compile_fail` doctest RED 证明可绕过构造/修改，GREEN 后字段限为 `pub(crate)`、外部测试经 `ValidatedQuery::new`；query 全量、Clippy、fmt 通过。新 SHA 的真实后端重验归 S4。
- [x] S2 (T-04-28)：ADR-0027 锁定可信 host 按 connector 注入 capability；wire 负例和 aggregate 7/7、fmt、Clippy 通过。现有 `sum_integer=false` 仅证明 planner 不选 partial pushdown；真实 SQL Server 端到端列为未来 host 接入门禁。
- [x] S3 (T-04-32)：Phase 4 CI 的 PostgreSQL 密码在 step 内逐轮生成、先 mask 再启动服务，Trino catalog 使用运行时副本；`nightly.yml` 保持原有 fixture。run 39878005620 的日志和 139 个 artifact 文件扫描无凭据命中；历史公开测试值的剩余风险见 `04-SECURITY.md`。
- [x] S4：`c39a971` 的 Linux CI 四 job、真实 PG/Trino、137 份 EXPLAIN 与 isolated p95 全部通过；完整证据见 `04-PERFORMANCE.md`。
- [ ] S5：安全复审 35/35 closed；完成 phase-level `04-VERIFICATION.md` 与 `$gsd-verify-work 4`；只有用户 UAT 通过才同步 ROADMAP/STATE 为完成。

## Acceptance

- 每条 QUERY-01..10 都有计划与可执行验证；核心叠加和鉴权逻辑标记 TDD。
- 计划含 production tracer、SQLite 与真实 Trino 的测试边界、越权 conformance、延迟预算与失败处理。
- 独立 plan-checker 通过；ROADMAP 只保留 phase 概览和 plan 指针，STATE 保持 digest。

## Next Step

完成 phase-level verification，再执行 `$gsd-verify-work 4` 取得用户 UAT 反馈；当前 CI 证据匹配 `c39a971`。
