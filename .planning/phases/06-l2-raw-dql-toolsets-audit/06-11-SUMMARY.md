---
phase: 06-l2-raw-dql-toolsets-audit
plan: 11
subsystem: database
tags: [rust, tokio, sqlx, sqlite, postgresql, audit, cancellation, tdd]
requires:
  - phase: 06-01
    provides: ReadAudit worker 和关闭生命周期
  - phase: 06-02
    provides: 双后端审计持久化合同
provides:
  - 并发和取消后重试的 shutdown 均等待 worker 持久化完成
  - shutdown 缓存成功、sink 失败和 JoinError 终态
affects: [AUDIT-01, phase-06-final-review]
actuals:
  tokens: 8768
  tasks: 1
  commits: 2
plan_head_before: 59390fdc0d5d2de768f45a90c96bed8621b454e2
plan_head_after: 7813e7fb591df1bd2eb607b1a31511492a62c9e3
tech-stack:
  added: []
  patterns:
    - async mutex 内借用 JoinHandle，取消调用方时保留 worker 所有权
    - 显式 Running/Finished 状态缓存关闭结果
key-files:
  created:
    - .planning/phases/06-l2-raw-dql-toolsets-audit/06-11-SUMMARY.md
  modified:
    - crates/ontoforge-storage/src/read_audit.rs
    - crates/ontoforge-storage/tests/conformance/mod.rs
key-decisions:
  - 沿用计划的 Running/Finished 状态，不增加 task、依赖或公开 API
  - 测试通过明确 poll Pending 和实际 durable rows 证明合同，避免概率竞态
requirements-completed: [AUDIT-01]
coverage:
  - id: D1
    description: 并发关闭等待持久化完成
    requirement: AUDIT-01
    verification:
      - kind: integration
        ref: crates/ontoforge-storage/tests/conformance/mod.rs#audit_concurrent_shutdown_waits_for_durability
        status: pass
    human_judgment: false
  - id: D2
    description: 取消首次 shutdown 后重试仍等待持久化完成
    requirement: AUDIT-01
    verification:
      - kind: integration
        ref: crates/ontoforge-storage/tests/conformance/mod.rs#audit_cancelled_shutdown_retry_waits_for_durability
        status: pass
    human_judgment: false
  - id: D3
    description: 重复 shutdown 保留 sink 错误和成功终态
    requirement: AUDIT-01
    verification:
      - kind: integration
        ref: crates/ontoforge-storage/tests/conformance/mod.rs#audit_shutdown_failure_is_sticky
        status: pass
    human_judgment: false
  - id: D4
    description: 重复 shutdown 保留 worker JoinError
    requirement: AUDIT-01
    verification:
      - kind: unit
        ref: crates/ontoforge-storage/src/read_audit.rs#read_audit::tests::audit_shutdown_join_error_is_sticky
        status: pass
    human_judgment: false
duration: 12min
completed: 2026-10-28
status: complete
---

# Phase 6 Plan 11: 审计关闭并发与取消安全 Summary

**ReadAudit 保留可取消等待的 worker handle，并缓存持久化完成结果，关闭调用不再提前成功或丢失错误。**

## Accomplishments

- 修复 CR-01：原实现对 worker Option 执行 take，后续调用把缺少 handle 当作成功。
- Running 状态在 async mutex 内通过可变引用 await JoinHandle；取消 shutdown future 会释放锁并保留 handle。
- worker 返回或 join 失败后写入 Finished(Result)，所有后续调用返回同一终态。
- 保持先关闭 admission、再取走根 sender、最后等待 worker 的既有顺序。
- 三个共享回归用例在 SQLite 和真实 PostgreSQL 上运行；独立单元测试覆盖实际 abort 引起的 JoinError。

## Task Commits

1. Task 1 RED: `0bb29a3`，`test(06-11): 复现审计关闭的并发取消与错误丢失`。
2. Task 1 GREEN: `7813e7f`，`feat(06-11): 保留审计关闭句柄并缓存终态`。

actuals 从磁盘 ledger 的基准到 `plan_head_after` 实测得到 2 个提交；tokens 是该范围自有源码和证据 diff 的字符数除以 4 向上取整，不是模型 token 计数。最后的 SUMMARY 文档提交不计入此实现范围。

## TDD Gate Compliance

- RED 在生产修改前提交。SQLite audit 为 5 passed、3 failed；真实 PostgreSQL audit 为 6 passed、3 failed；JoinError 单测为 0 passed、1 failed，退出码均为 101。
- 语义核验：并发与取消测试都在已接受 guard 尚未 finish 时实际得到提前返回；sink 与 JoinError 测试都在首次 Err 后实际得到重复调用 Ok。目标断言确实执行，失败不是编译、fixture 或零测试造成。
- 四份真实 JUnit evidence 经 GSD `check tdd-red-evidence` 返回 `RED_EVIDENCE_OK`。JoinError evidence 首次 target 名未采用 classifier 的 class#method 形式，修正 target 字段后通过；原始 XML 未修改。
- GREEN 仅增加显式 worker 状态和缓存结果。亲读锁定 Tokio 1.53.3 的官方 `runtime/task/join.rs` 文档，确认借用 `&mut JoinHandle<T>` 等待的取消安全合同。
- REFACTOR 无新增修改，当前实现已是计划限定的最小状态转换。
- 主会话回报已执行 GSD `check tdd-review-checkpoint 06`：`block:false`，9 plans，0 violations，已识别本计划 RED/GREEN；原始 gate 证据由主会话保存。

## Verification

命令在本机 PowerShell 中串行执行，以下 GREEN 和质量检查均退出 0：

| 命令 | 结果 | 证据 |
| --- | --- | --- |
| `cargo test -p ontoforge-storage --test conformance_sqlite --locked -j 2 audit_ -- --nocapture` | 8 passed，0 failed，1.13s | `06-11-GREEN-sqlite.log` |
| `cargo test -p ontoforge-storage --test conformance_pg --locked -j 2 audit_ -- --ignored --nocapture` | 9 passed，0 failed，1.48s | `06-11-GREEN-pg.log` |
| `cargo test -p ontoforge-storage --lib --locked -j 2 audit_shutdown_join_error_is_sticky -- --nocapture` | 1 passed，0 failed | `06-11-GREEN-join-error.log` |
| `cargo clippy -p ontoforge-storage --all-targets --locked -j 2 -- -D warnings` | 通过，38.72s | `06-11-CLIPPY.log` |
| `cargo fmt --all -- --check` | 通过 | `06-11-FMT.log` |
| `git diff --check` | 通过 | `06-11-DIFF-CHECK.log` |

PostgreSQL 使用实际 17.11 临时容器 `ontoforge-pg-06-11`，绑定 `127.0.0.1:55433`；运行前确认就绪，结束后删除本任务创建的容器，见 `06-11-FIXTURE-CLEANUP.log`。日志中的 `read audit sink did not recover before the pending age limit` 来自故障注入用例，相关断言通过。未输出或保存测试 DSN/密码。

RED 证据为同目录 `06-11-RED-{sqlite,pg,join-error}.log`、两个原始 XML 和四个 `06-11-RED-*-junit.json`。

## Deviations from Plan

实现没有超出计划。执行环境没有可调用的 `ralph-loop`，未声称运行该工具；以明确完成标准、RED/GREEN 实跑及提交完成本子任务。没有通过 Warp 运行，实际终端为 PowerShell。Grok 第二意见等待 55 秒超时，没有可引用意见；已决策设计仍按主会话给定的方案执行。

## Issues Encountered

无新增依赖、公开 API、网络端点、数据库 schema 或信任边界变化。检查新增 diff 未发现 TODO、FIXME、占位或未接线数据源。无任务内跳过测试。

## Next Phase Readiness

本有界修复及本地 focused 验证已完成。Phase 6 的独立增量 review、当前修复 SHA 的 Ubuntu/Windows workspace 和 integration CI，以及 verifier 复签由主会话负责；这些最终门禁尚未在本 SUMMARY 中宣称通过。PR 不合并，Phase 7 暂缓。

按计划单写约束，本执行器不更新共享 STATE、ROADMAP、REQUIREMENTS、task_plan、progress、findings 或审查报告；主会话从本文件蒸馏状态。

## Self-Check: PASSED

- 两个源码文件、SUMMARY 及三份 GREEN 日志均已确认存在。
- `0bb29a3` 与 `7813e7f` 均已通过 `git merge-base --is-ancestor <hash> HEAD` 检查。
- GREEN 提交不含 tracked 文件删除；`git diff --check` 再次通过。
