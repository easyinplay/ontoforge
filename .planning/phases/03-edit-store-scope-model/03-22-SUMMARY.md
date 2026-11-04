---
phase: 03-edit-store-scope-model
plan: 22
subsystem: storage
tags: [migrate, scope, roles, criteria-coverage, ONT-V21, ONT-V23, sqlite, postgres, d-18]
gap_closure: true

requires:
  - phase: 03-edit-store-scope-model
    provides: "reload 的 D-18 role 复核（03-18）、criteria 列覆盖（03-14 / 03-15）、migrate 骨架（03-10）"
provides:
  - "apply_migration_probed 的 roles 参数：定义锁内、任何步骤之前按新定义重新编译存下的 role，不成立为 MigrateError::Definition"
  - "ontoforge migrate --apply 始终复核已 apply 的 role，不成立退出码 1，定义与 edit store 不变"
  - "coverage::problem 把整对象读取 objects.<param> 视为读取该类型的全部属性，allow-list 缺任一属性报 ONT-V23"
affects: [03-23, phase-5-rest, phase-7-mcp]

plan_head_before: 478148b25b98c99049a953811799fed768b42687

actuals:
  tokens: 9400
  tasks: 3
  commits: 7

tech-stack:
  added: []
  patterns:
    - "migrate 的 role 复核与 reload 同口径：load_scope_in + compile_scope(new)，两处各约 10 行，不抽取共用块"
    - "整对象读取用 2 段非动态路径识别（walk.rs 的 select 只产出最长成员链），类型缺失按 fail-closed 复用静态不可判定消息"

key-files:
  created: []
  modified:
    - crates/ontoforge-storage/src/migrate.rs
    - crates/ontoforge/src/migrate.rs
    - crates/ontoforge-storage/tests/migrate.rs
    - crates/ontoforge-storage/tests/reload.rs
    - crates/ontoforge/tests/cli_migrate.rs
    - crates/ontoforge-storage/tests/migrate_pg.rs
    - crates/ontoforge-auth/src/scope/coverage.rs
    - crates/ontoforge-auth/tests/criteria_coverage.rs
    - docs/scope.md

key-decisions:
  - "roles 取 Option<&RolesCheck>（orchestrator 草稿写的是 Option<RolesCheck>），与同为可选的 probe 一致，语义不变"
  - "apply_migration（无 probe）不做 role 复核并在 doc 写明，CLI 走 apply_migration_probed；migrate --plan 不预检 role"
  - "整对象读取的缺失属性按定义顺序列在消息里，help 指向直接比较某一列"

requirements-completed: [SCOPE-06]

coverage:
  - id: D1
    description: "migrate --apply 在定义锁内、步骤之前复核已 apply 的 role，破坏 role 的变更以 ONT-V21 拒绝，定义与 edit 表不变；不做复核时同一迁移可应用"
    requirement: SCOPE-06
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/migrate.rs#migrate_rechecks_applied_roles"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/migrate.rs#migrate_with_roles_applies_when_the_roles_still_compile"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_migrate.rs#migrate_refuses_a_change_that_breaks_an_applied_role"
        status: pass
    human_judgment: false
  - id: D2
    description: "同一复核在嵌入式 PostgreSQL 的定义事务内生效，被拒绝的迁移不留归档列，active def_version 不变"
    requirement: SCOPE-06
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/migrate_pg.rs#pg_migrate_rechecks_applied_roles"
        status: pass
    human_judgment: false
  - id: D3
    description: "提交条件整体读取对象参数（objects.flight == params.x、objects.flight != null）时，allow-list 缺该类型任一属性报 ONT-V23；全含则编译通过"
    requirement: SCOPE-06
    verification:
      - kind: integration
        ref: "crates/ontoforge-auth/tests/criteria_coverage.rs#whole_object_read_needs_every_property"
        status: pass
    human_judgment: false

duration: 约 40min
completed: 2026-10-21
status: complete
---

# Phase 3 Plan 22: migrate 的 role 复核与整对象读取覆盖 Summary

**`ontoforge migrate --apply` 在定义锁内按新定义重新编译已 apply 的 role，不成立即以 ONT-V21 / V22 / V23 拒绝；`objects.<param>` 整体读取按读取该类型全部属性检查 allow-list**

## Performance

- **Duration:** 约 40 min
- **Completed:** 2026-10-21
- **Tasks:** 3（Task 1 tracer、Task 2 与 Task 3 均为 TDD）
- **Files modified:** 9

## Accomplishments

- gap 3a（WR-B07）关闭：`apply_migration_probed` 增加 `roles: Option<&RolesCheck>`。`apply_in` 在 `lock_def_state`、`check_hash` 与「空计划且定义相同」的提前返回之后、`run` 之前，`roles` 为 Some 时 `scope_store::load_scope_in` 再 `compile_scope(new)`，失败映射为 `MigrateError::Definition`。CLI 的 `roles_check()` 用 `attr_types(server.config.auth)` 与 `ProfileName` 构造 `RolesCheck`，`--apply` 始终传入。`migrate --plan` 与 `--drop-archived` 路径不变。
- gap 3b（WR-C05）关闭：`coverage::problem` 增加 `ontology` 参数。原 `path.segments.get(2)?` 对不带属性的 `objects.<param>` 直接返回 None；现在 2 段路径取该类型全部属性，不在 `grant.properties` 的列在消息里逐个列出（V23）。类型缺失按 fail-closed 复用「cannot be determined statically」。`compile_scope` 是 scope plan / apply、reload 与 migrate 共用的入口，所以三条路径同时生效。
- `docs/scope.md` 增加「定义变更时的复核」一段（先改 scope、`scope apply`，再 `migrate`；`--plan` 不预检）与整对象读取的一句说明。

## Task Commits

1. **Task 1（tracer）: migrate --apply 的 role 复核** - RED `7c92fc5` (test)，GREEN `39d6092` (fix)，REFACTOR `f3cbd4d` (refactor，见 Deviations)
2. **Task 2: PG 上的同一复核与文档** - 测试 `43082da` (test，带 PG RED 摘录)，文档 `7526c67` (docs)
3. **Task 3: 整对象读取** - RED `421e8e7` (test)，GREEN `6432321` (fix)

**Plan metadata:** 见随后的 `docs(03-22)` 提交。`git rev-list --count 478148b..HEAD` 在写本文件前为 7（`plan_head_before` 即 `478148b`）。

## RED 证据

- Task 1（SQLite）：`migrate_rechecks_applied_roles` 因 `expected a definition error, got Ok(2)` 失败，同文件另一用例 `migrate_with_roles_applies_when_the_roles_still_compile` 在 RED 时就通过（本来就是不被拒绝的路径）。CLI：`migrate_refuses_a_change_that_breaks_an_applied_role` 因 `assertion left == right failed: stderr: ok: def_version 2 applied  left: Some(0) right: Some(1)` 失败。RED 提交里 `roles` 参数已加但未使用，CLI 暂传 `None`。
- Task 2（PG，临时去掉复核，把 `if let Some(check) = roles` 改成 `roles.filter(|_| false)`）：

```
RED (PG, recheck removed):
cargo test -p ontoforge-storage --features embedded-pg --locked --test migrate_pg pg_migrate_rechecks_applied_roles -- --ignored
test pg_migrate_rechecks_applied_roles ... FAILED
thread 'pg_migrate_rechecks_applied_roles' panicked at crates\ontoforge-storage\tests\migrate_pg.rs:326:9:
expected a definition error, got Ok(2)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.05s
```

  还原后 `git --no-pager diff --exit-code -- crates/ontoforge-storage/src/migrate.rs` 退出码 0，同一命令通过。该摘录已原样写入提交 `43082da` 的 commit message，`<verify>` 里的 `RED (PG, recheck removed):` 与 `test result: FAILED` 两条机械检查均命中。
- Task 3：`whole_object_read_needs_every_property` 因 `the scope must not compile`（allow-list 缺 `gate` 仍然编译通过）在 `expect_err` 处失败。

## 验证结果（实际输出）

- `cargo test -p ontoforge-storage --locked --test migrate`：`test result: ok. 15 passed; 0 failed`
- `cargo test -p ontoforge --locked --test cli_migrate`：`test result: ok. 10 passed; 0 failed`（含 `migrate_refuses_a_change_that_breaks_an_applied_role`）
- `cargo test -p ontoforge-storage --locked --test reload`：`test result: ok. 32 passed; 0 failed`（两个 probe 调用点补 `None` 后仍通过）
- `cargo test -p ontoforge-storage --features embedded-pg --locked --test migrate_pg -- --ignored`：`test result: ok. 4 passed; 0 failed`
- `cargo test -p ontoforge-auth --locked --test criteria_coverage`：`test result: ok. 6 passed; 0 failed`
- `cargo test --workspace --locked`：退出码 0，84 个 `test result: ok` 目标，合计 980 passed、0 failed（`ontoforge-engine --test drift` 本次通过，8 passed，未复现 03-21 的偶发失败）
- `cargo clippy --workspace --all-targets --locked -- -D warnings`：无告警；`cargo fmt --all --check`：通过
- 验收：`rg "scope_store::load_scope_in" crates/ontoforge-storage/src/migrate.rs` 命中（第 405 行）；`rg RolesCheck crates/ontoforge/src/migrate.rs` 命中；`rg "segments.get\(2\)\?"` 无匹配（退出码 1）；`rg "whole object" coverage.rs` 命中；`rg "objects.flight != null" docs/scope.md` 与 `rg "migrate --apply" docs/scope.md` 命中

## Files Created/Modified

- `crates/ontoforge-storage/src/migrate.rs` - `roles` 参数、锁内 role 复核、模块文档与 doc 注释
- `crates/ontoforge/src/migrate.rs` - `roles_check()`、`execute` 传 `Some(roles)`、模块文档
- `crates/ontoforge-storage/tests/migrate.rs` - aviation + scope 的两个 SQLite 用例
- `crates/ontoforge-storage/tests/reload.rs` - 两个 probe 调用点补 `None`
- `crates/ontoforge/tests/cli_migrate.rs` - CLI 用例
- `crates/ontoforge-storage/tests/migrate_pg.rs` - `pg_migrate_rechecks_applied_roles`
- `crates/ontoforge-auth/src/scope/coverage.rs` - 整对象读取规则
- `crates/ontoforge-auth/tests/criteria_coverage.rs` - `whole_object_read_needs_every_property`
- `docs/scope.md` - 定义变更复核与整对象读取说明

## Decisions Made

- `roles` 取引用 `Option<&RolesCheck>`（草稿为 `Option<RolesCheck>`），与 `probe` 一致。
- 复核直接内联在 `apply_in`，不抽取 reload 的块：`reload.rs` 不动，两处各约 10 行。
- `apply_migration` 不复核，doc 写明；只有 CLI 的 `--apply` 与显式传 `Some` 的调用方复核。

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] clippy `too_many_lines` 使 `run` 超过 100 行**
- **Found during:** Task 1 GREEN 之后的 `cargo clippy --workspace --all-targets -D warnings`
- **Issue:** 在 `crates/ontoforge/src/migrate.rs` 的 `run` 里内联构造 `RolesCheck` 后函数为 108 行。GREEN 提交 `39d6092` 的 shell 命令链没有在 clippy 失败时中止，该提交因此带着这条告警。
- **Fix:** 把构造逻辑抽成 `roles_check(server, args)`，无行为变化。
- **Files modified:** `crates/ontoforge/src/migrate.rs`
- **Verification:** `cargo clippy --workspace --all-targets --locked -- -D warnings` 无告警；`cli_migrate` 10 passed
- **Commit:** `f3cbd4d`

**Total deviations:** 1 auto-fixed（1 个 Rule 1）。**Impact:** 仅 lint，无行为影响。

## Issues Encountered

- Task 1 的 tracer 反馈门：`<verify>` 的三条命令在 GREEN 与 REFACTOR 后都已重跑并通过，之后才进入 Task 2、3。
- 无需用户确认的 gray area，未触发 `STATUS: NEEDS_CLARIFICATION`。

## Known Stubs

None.

## Threat Flags

None. 本 plan 没有引入新的网络端点、认证路径或信任边界；T-03-79 到 T-03-81 按 threat_model 的 `mitigate` 落地（锁内复核、CLI 始终传 `RolesCheck`、整对象读取按全部属性检查），T-03-82 按 `accept` 处理并在 `docs/scope.md` 写明顺序。

## Next Phase Readiness

03-VERIFICATION 的 key link「migrate -> stored roles」由 NOT_WIRED 变为 WIRED，gaps[2] 的 3a 与 3b 都有对应的失败测试转绿。可进入 03-23。

## Self-Check: PASSED

- 9 个修改文件均存在；7 个提交（`7c92fc5`、`39d6092`、`f3cbd4d`、`43082da`、`7526c67`、`421e8e7`、`6432321`）均在 `git log` 中。
- TDD 门：`test(03-22)` 先于对应的 `fix(03-22)`（Task 1、Task 3）；Task 2 的 RED 由去掉复核的显式运行证明，摘录在 `43082da`。
