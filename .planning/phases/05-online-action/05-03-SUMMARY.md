---
phase: 05-online-action
plan: 03
subsystem: action-versioning
tags: [rust, action, query, hs256, optimistic-concurrency, sqlite, postgresql, tdd]
requires:
  - phase: 05-02
    provides: Action guard read set and storage-owned locked transaction
provides:
  - shared versioned canonical Query and Action `_version` codec
  - locked editOnly Action token verification with uniform VersionConflict
  - no-token observed edit version CAS across SQLite and PostgreSQL
affects: [05-04, 05-05, 05-06, 05-07]
actuals:
  tokens: 13513
  tasks: 2
  commits: 5
commits: 5
plan_head_before: 520bbb80d1accae8c818970b593850d49201e142
plan_head_after: 0d58c378e1bd56db8cba0b8c79e50bac014cb609
tech-stack:
  added: [ontoforge-action dependency edges to existing ontoforge-query and test-only ontoforge-engine]
  patterns: [trusted host TokenCodec injection, version verification after storage CAS under stable locks]
key-files:
  created:
    - crates/ontoforge-action/src/version.rs
    - crates/ontoforge-action/tests/version.rs
  modified:
    - crates/ontoforge-query/src/cursor.rs
    - crates/ontoforge-query/src/lib.rs
    - crates/ontoforge-query/tests/security.rs
    - crates/ontoforge-action/src/lib.rs
    - crates/ontoforge-storage/tests/conformance/mod.rs
key-decisions:
  - ActionService requires the trusted host's shared TokenCodec; all token failures map to VersionConflict.
  - Source-backed tokens fail closed until the 05-06 source sampler is available; 05-03 proves editOnly round trips.
  - Storage compares transaction-prepared edit versions after locking the complete read/edit set, even without a client token.
requirements-completed: [ACTION-09]
coverage:
  - id: ACTION-VERSION-TOKEN
    description: Query-issued editOnly token is accepted unchanged and rejected after identity, scope, definition, edit or key changes.
    requirement: ACTION-09
    verification:
      - kind: integration
        ref: cargo test -p ontoforge-action --test version --locked -- --nocapture
        status: pass
      - kind: integration
        ref: cargo test -p ontoforge-query --test security --locked version_ -- --nocapture
        status: pass
    human_judgment: false
  - id: ACTION-OBSERVED-CAS
    description: Missing client token still enforces observed edit versions under full RowKey locks and rolls back version-zero skeletons.
    requirement: ACTION-09
    verification:
      - kind: integration
        ref: cargo test -p ontoforge-storage --test conformance_sqlite --locked action_cas_ -- --nocapture
        status: pass
      - kind: integration
        ref: cargo test -p ontoforge-storage --features embedded-pg --test conformance_pg --locked action_cas_ -- --ignored --nocapture
        status: pass
    human_judgment: false
duration: 46min
completed: 2026-10-23
status: complete
---

# Phase 5 Plan 03: Action Version and CAS Summary

Query 与 Action 共用带 `v1.` 标签的 canonical `_version`，Action 在 storage 稳定锁定的读集上验证 editOnly token；无 token 请求仍执行 observed edit version CAS。

## Accomplishments

- `VersionClaims` 绑定 principal identity/attributes、effective TypeScope、logical type/pk、definition、source 与 edit；JSON/map 输入规范化，旧裸签名 token 不再验证。
- `ActionService::new` 必须由可信 host 注入 Query 使用的 `TokenCodec`；伪造、旧格式、key rotation、scope/principal/definition/edit 变化均稳定返回 `VersionConflict`，冲突不写 command/outbox。
- 事务外保存的 edit 版本在 storage 锁齐后比较。SQLite 和真实 embedded PostgreSQL 均验证旧版本拒绝、无额外 log/command/outbox、version-0 skeleton 回滚。

## TDD Gate Compliance

- Task 1 RED `8e46d9f` 的 Query 格式目标断言失败，证据 `target/gsd-05-03-task1-red.json` 为 `RED_EVIDENCE_OK`；GREEN `2452dfb`。
- Task 2 RED `f294a17` 的伪造 token 目标断言失败，证据 `target/gsd-05-03-task2-red.json` 为 `RED_EVIDENCE_OK`；GREEN `0d58c37`。
- 计划 ledger 从 `520bbb8` 到 `0d58c37` 测得 5 个提交，包含主会话合同文档提交 `e57f1c7`。

## Verification

- `cargo test -p ontoforge-action --locked`：lib 3/3、action 6/6、security 10/10、version 5/5。
- `cargo test -p ontoforge-query --test security --locked version_ -- --nocapture`：3/3。
- `cargo test -p ontoforge-storage --test conformance_sqlite --locked action_cas_ -- --nocapture`：2/2。
- `cargo test -p ontoforge-storage --features embedded-pg --test conformance_pg --locked action_cas_ -- --ignored --nocapture`：真实 embedded PostgreSQL 2/2。
- `cargo clippy -p ontoforge-query -p ontoforge-action -p ontoforge-storage --all-targets --locked -- -D warnings`、`cargo fmt --all --check`、`git diff --check`：通过。

## Known Stubs

- `crates/ontoforge-action/src/version.rs:47`：source-backed token 在 05-03 无 source sampler，故统一 `VersionConflict`；05-06 接入真实事务前 source snapshot 并验证 Query→Action 往返及 Engine call count。此处 fail closed 是 05-03 的既定合同，不将 source-backed 验收记为通过（`WINDOWS.md` #8）。

## Deviations from Plan

- 为真实 Query→Action 集成测试增加 test-only `ontoforge-engine` dependency edge；只复用现有 workspace package，没有安装新包。
- `ActionService` 实际入口仍在 `src/lib.rs`，沿用 05-02 的模块结构；共享 codec 放在 Query `cursor.rs`。无新的存储 schema 或网络入口。

## Next Plan Readiness

05-06 需提供 source sampler，并证明 source-backed token 的真实往返、一次事务前 source 观察和零次事务内 source 重读。

## Self-Check: PASSED

共享 codec、Action adapter、跨 crate 测试和本 SUMMARY 均存在；Task 1/2 RED/GREEN 提交为当前 HEAD 祖先；ledger 从 `520bbb8` 到 `0d58c37` 实测 5 个提交，代码改动已提交。
