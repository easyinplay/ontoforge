---
phase: 06-l2-raw-dql-toolsets-audit
plan: 03
subsystem: engine
tags: [sqlparser, trino, l2, authorization]
requires:
  - phase: 06-l2-raw-dql-toolsets-audit
    provides: "ADR-0017/0018 与 06-CONTEXT 的 L2 SQL 合同"
provides:
  - "单条 SELECT/WITH 的受控 AST、关系授权和私有 PrintedSql Display 边界"
  - "104 个 constructed-equivalent G/B ID 的生产 gate 分类语料"
affects: [06-06, 06-09]
actuals:
  tokens: 11631
  tasks: 2
  commits: 6
commits: 6
plan_head_before: 10f272452ebf18b92ebb69fc3bd5d28ab27eea47
plan_head_after: dae2735c9958b9f3eca04e69d8aa6a5b14185598
tech-stack:
  added: []
  patterns: ["私有 PrintedSql 构造", "白名单 AST visitor", "受控 LIMIT/OFFSET 改写"]
key-files:
  created:
    - crates/ontoforge-engine/src/printer/l2.rs
    - crates/ontoforge-engine/tests/l2_gate.rs
    - crates/ontoforge-engine/tests/data/l2_corpus.json
  modified:
    - crates/ontoforge-engine/src/printer/mod.rs
    - crates/ontoforge-engine/Cargo.toml
    - Cargo.lock
key-decisions:
  - "未知表达式与二元运算符拒绝；GenericDialect 将 G08 箭头解析为 BinaryOp::Arrow。"
  - "G11 ROW 类型 Display 有损，拒绝；G16 OFFSET/LIMIT 经 AST 改写且 Trino 483 实测后放行。"
requirements-completed: [L2-02, L2-03]
coverage:
  - id: D1
    description: "L2 AST、关系授权及 Display-only gate"
    requirement: L2-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-engine/tests/l2_gate.rs#l2_gate_"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/sql_text_only_from_printer.rs#sql_text_only_from_printer"
        status: pass
    human_judgment: false
  - id: D2
    description: "104-ID constructed-equivalent corpus 与已知 Trino 差分"
    requirement: L2-03
    verification:
      - kind: integration
        ref: "crates/ontoforge-engine/tests/l2_gate.rs#l2_corpus_"
        status: pass
      - kind: manual_procedural
        ref: "Trino 483 G16 query 20261026_072806_00018_jtvdc"
        status: pass
    human_judgment: false
duration: 30min
completed: 2026-10-26
status: complete
---

# Phase 6 Plan 03: L2 SQL gate 与 corpus Summary

仅校验后的 `sqlparser` Display 可构造 `PrintedSql`；104 个历史 ID 均有可执行的构造等价样例和生产 gate 断言。

## Performance

- **Duration:** 30 min（15:01 至 15:31，Asia/Shanghai）
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments

- 单语句 SELECT/WITH 的 AST、CTE、物理 relation、catalog/schema grant、edit/system 硬排除、输入与结果上限及私有构造边界已实现。拒绝路径不产生 Trino POST。
- 104 个唯一 G/B ID 均为 `constructed-equivalent`；0 条原始 SQL 恢复。每条记录历史类别、当前预期和原因，并逐条运行生产 gate。G25/G29/G30/G37 的 parser 可用性偏差明确记为拒绝，不把历史 G 标签当作当前放行依据。
- G11 有损 ROW 类型拒绝；G16 改写为 `OFFSET 10 ROWS FETCH FIRST 5 ROWS ONLY`；B53 仅发送无注释的 Display。

## Task Commits

- Task 1 RED：`307ef2e`、`3a26919`；GREEN：`e69ecaf`。
- Task 2 RED：`315a9d7`、`1e85008`；GREEN：`dae2735`。

## TDD Gate Compliance

- Task 1 首个 RED 提交含仅返回拒绝的编译脚手架；后续 `3a26919` 的 offset 断言针对行为失败，再由 `e69ecaf` 实现。首个 RED 不应被描述为纯测试提交。
- Task 2 的 `315a9d7` 在仅有 G01 的 fixture 上失败于 `1 != 104`；补全 fixture 后 `1e85008` 的分类断言因 G08 `expected unsupported, got allow` 失败。`dae2735` 收紧二元运算符白名单，6 项 corpus 测试通过。

## Verification

- `cargo test -p ontoforge-engine --test l2_gate --locked -- --nocapture`：16/16 通过，其中 6 项 corpus 测试。
- `cargo test -p ontoforge-engine --test sql_text_only_from_printer --locked`：1/1 通过。
- `cargo test -p ontoforge-engine --locked -j 2`：所有非 ignored 测试通过；`trino_it` 的 9 个 CI 环境测试仍按原有配置 ignored。
- `cargo clippy -p ontoforge-engine --all-targets --locked -- -D warnings`、`cargo fmt --all -- --check`、`git diff --check` 均通过。
- 本机健康的 `trinodb/trino:483`：`SELECT version()` FINISHED（`20261026_072805_00017_jtvdc`）；G16 改写 Display FINISHED、0 行（`20261026_072806_00018_jtvdc`）；B53 安全 Display FAILED、`CATALOG_NOT_FOUND`（`20261026_072806_00019_jtvdc`）。本轮 3 次只读 POST，2 FINISHED、1 FAILED。B53 失败只反映容器没有 `hive` catalog，不能证明 ACL 安全。

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] G08 箭头表达式被宽泛二元运算符分支放行**
- **Found during:** Task 2 104-ID 分类测试。
- **Issue:** `GenericDialect` 将 `x -> x + 1` 解析为 `BinaryOperator::Arrow`，不是 `Expr::Lambda`。
- **Fix:** 仅允许明确列出的常规算术、比较、连接和布尔运算符。
- **Files modified:** `crates/ontoforge-engine/src/printer/l2.rs`、`crates/ontoforge-engine/tests/l2_gate.rs`。
- **Verification:** 104-ID 分类及完整 engine 测试通过。
- **Committed in:** `dae2735`。

## Next Phase Readiness

`06-06` 可接入 `validate_l2` 与 `explain_l2`。真实受限 Trino ACL 自检仍属于后续 Phase 6 门禁；本轮容器无 ACL，不以此实测声称授权策略已在 Trino 侧证明。

## Self-Check: PASSED

三个新建文件、SUMMARY 及六个任务提交均已核实存在于当前 HEAD 历史。
