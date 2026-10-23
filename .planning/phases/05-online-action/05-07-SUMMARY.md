---
phase: 05-online-action
plan: 07
subsystem: action-storage-ci
tags: [rust, outbox, webhook, postgresql, trino, tdd]
requires:
  - phase: 05-05
    provides: Action 原子命令、edit log、outbox 与幂等回执
  - phase: 05-06
    provides: 真实 Trino/PostgreSQL Action 测试
provides:
  - 带 lease/fencing token 的 durable webhook worker 和稳定 event_id 的最小版本化 payload
  - audited import 源文件 SHA-256、production 写入口回归守卫与非零 ignored-test CI gate
affects: [ACTION-07, ACTION-11, Phase-5-verification]
actuals:
  tokens: 16968
  tasks: 2
  commits: 5
commits: 5
plan_head_before: 61c66a90386547d9f660c43c17ccbe2b9edbcbf9
plan_head_after: ecb616f1c94350eaab6b72844ea405fe26decccc
tech-stack:
  added: []
  patterns: [短事务 claim 与条件 mark/retry, 每次 claim 新 fencing token, trusted webhook target, 非零 ignored-test discovery]
key-files:
  created:
    - crates/ontoforge-action/src/outbox.rs
    - crates/ontoforge-action/tests/outbox.rs
    - crates/ontoforge-action/tests/outbox_pg.rs
  modified:
    - crates/ontoforge-storage/src/commit.rs
    - crates/ontoforge-storage/src/import.rs
    - crates/ontoforge-action/tests/entrypoints.rs
    - .github/workflows/ci.yml
    - .github/workflows/nightly.yml
key-decisions:
  - 每次 claim 分配新 owner token；发送发生在数据库事务外，过期 owner 不得 mark/retry。
  - webhook target 只由可信 WebhookConfig 注入，payload 不包含对象值；receiver 按 event_id 去重。
  - PostgreSQL 集成测试使用显式 test-support feature，embedded-pg 不成为默认 feature。
requirements-completed: [ACTION-07, ACTION-11]
coverage:
  - id: ACTION-OUTBOX-RECOVERY
    description: 提交后崩溃、HTTP 失败、发送后标记前崩溃、lease 过期与并行 worker 保持至少一次投递和稳定 event_id。
    requirement: ACTION-11
    verification:
      - kind: integration
        ref: crates/ontoforge-action/tests/outbox.rs
        status: pass
      - kind: integration
        ref: crates/ontoforge-action/tests/outbox_pg.rs#postgres_outbox_
        status: pass
    human_judgment: false
  - id: ACTION-WRITE-ENTRY
    description: production 对象写调用只出现于 Action 和有源文件摘要的 audited import。
    requirement: ACTION-07
    verification:
      - kind: integration
        ref: crates/ontoforge-action/tests/entrypoints.rs
        status: pass
    human_judgment: false
  - id: ACTION-CI-GATES
    description: PostgreSQL 与 Trino ignored corpus 在 CI/nightly 中先非零发现，再实际执行。
    verification:
      - kind: other
        ref: cargo test -- --ignored --list; local filter counts 5/4/6/2/4
        status: pass
    human_judgment: false
duration: 72min
completed: 2026-10-23
status: complete
---

# Phase 5 Plan 07: outbox 与 CI 写入口验收 Summary

Action 提交与 outbox 同事务，worker 用租约和 fencing token 恢复通知；audited import 留存源文件摘要，CI 的真实后端测试拒绝零发现的假绿。

## Accomplishments

- worker 短事务 claim due row，事务外发送 HTTP，再以 `(event_id, lease_owner)` 条件 mark 或退避重试；旧 owner 不能覆盖新 attempt。payload 仅有版本、事件/日志/Action 身份及最小回执，target 来自 trusted `WebhookConfig`。
- CSV/JSON import 在读取同一份字节时计算 SHA-256，逐 batch 写入审计日志。source guard 扫描 production Rust 调用并限定 Action 与 audited import，排除 storage 的内部 SQL 事务提交。
- CI/nightly 对真实 PostgreSQL atomicity、race、idempotency、outbox 与真实 Trino Action 测试先做 ignored discovery，再执行；缺服务 URL 或零发现时失败。

## TDD Gate Compliance

- Task 1 RED `e1593f7`：`target/gsd-05-07-task1-red.json` 的原生 JUnit 目标测试因 `schema_version` 为 Null（期望 1）失败，`gsd check tdd-red-evidence` 判定 `RED_EVIDENCE_OK`；GREEN `f30d6ec`。
- Task 2 RED `f6820c9`：`target/gsd-05-07-task2-red.json` 的原生 JUnit 目标测试因 import `source_digest` 为 Null 失败，判定 `RED_EVIDENCE_OK`；GREEN `ecb616f`。source guard 的换行和 UFCS 自检在 GREEN 中补强。

## Verification

- 本次提交前 `cargo test -p ontoforge-action --locked -j 2 -- --nocapture`：默认实际执行 54 个测试，全通过；PostgreSQL 2 个与 Trino 4 个 ignored tests 不计入这 54 个。
- `cargo test -p ontoforge-storage --test import --locked -j 2 -- --nocapture`：SQLite 16/16，真实 PG 2 个 ignored。真实 PG outbox 2/2、atomicity 5/5、race 4/4、idempotency 6/6 和真实 Trino Action 4/4 曾在本计划执行时以服务环境实际运行；CI 同款 ignored filters 本次再确认非零发现数分别为 2、5、4、6、4。
- `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings`、`cargo fmt --all --check`、`git diff --check` 通过。
- `cargo test --workspace --locked -j 2 -- --nocapture` 退出 101：现有 `ontoforge --test cli_schema` 的 `schema_export_writes_committed_files` 报 `schemas/ontoforge-definition.schema.json differs from schemas/`。日志 `target/gsd-05-07-workspace-test.log`，已记 `deferred-items.md`；workspace suite 未通过。匹配提交 SHA 的远端 CI 尚待运行。

## Task Commits

- Task 1：`e1593f7` RED，`f30d6ec` GREEN。
- Task 2：`f6820c9` RED，`ecb616f` GREEN。
- 测量账本从 `61c66a9` 到 `ecb616f` 共 5 个提交；其中 `420dac3` 是主会话在任务执行中写入的 phase 进度记录。文档收尾另行提交。

## Deviations from Plan

- [Rule 1 - Bug] 真实 PostgreSQL 解码 outbox `attempt_count` 的 INT4 与 nullable `timestamptz` 绑定，修复于 Task 1 GREEN；PG outbox 2/2。
- [Rule 3 - Blocking] `ontoforge-query` 正常依赖缺 `tokio` macros；PG 竞态测试 helper 曾只受 `embedded-pg` 控制。Task 2 显式引入 `test-support` 并维持默认 feature 关闭；workspace all-targets Clippy 通过。
- 范围外 workspace schema fixture 不一致未在 05-07 修改，记录在 `deferred-items.md`。

## Known Stubs

None. 对本计划改动的源文件及测试扫描未发现阻止本计划目标的 placeholder、TODO 或跳过测试。

## Next Phase Readiness

ACTION-07/11 的本地行为与后端证据就位。Phase 5 正式 verify-work 和匹配代码 SHA 的 CI 仍需执行；workspace schema fixture 缺口必须单独处理后才能宣称全量测试通过。

## Self-Check: PASSED

outbox、入口测试、import 和 SUMMARY 文件存在；`e1593f7`、`f30d6ec`、`f6820c9`、`ecb616f` 均为当前 HEAD 的祖先。`git diff --check` 通过。
