---
phase: 07-rest-mcp-shell
plan: "02"
subsystem: storage
tags: [rust, sqlite, postgresql, request-snapshot, action, tdd]
requires:
  - phase: 07-01
    provides: "请求定义与 scope snapshot 的基础入口"
provides:
  - "真实共同锁下的 SQLite metadata reload/apply/revoke 合同及同合同 PG 测试入口"
  - "固定准入定义的 Action replay、validate-only 与 history 验收及 replay 二次读取修复"
  - "本轮 RED/GREEN、workspace 回归及构建的原始证据"
affects: [07-04, 07-13, 07-14]
actuals:
  tokens: 49691
  tasks: 2
  commits: 10
plan_head_before: 091b8b53b94aaeaef625a9a28f8bb6ac122c1cf0
plan_head_after: 5e7f5fbb4ef2f2a11b7933648b0d216c521cb1ee
tech-stack:
  added: []
  patterns:
    - "两 store 共用 metadata 事务锁，以 Notify 固定 reader/writer 次序"
    - "admission snapshot 决定本次 Action 定义，replay/result 保留 winner receipt/time"
key-files:
  created:
    - .planning/phases/07-rest-mcp-shell/07-02-evidence/
  modified:
    - crates/ontoforge-action/src/lib.rs
    - crates/ontoforge-action/tests/frozen_definition.rs
    - crates/ontoforge-action/tests/idempotency_pg.rs
    - crates/ontoforge-storage/tests/request_snapshot.rs
    - crates/ontoforge-storage/tests/frozen_history.rs
    - .planning/phases/07-rest-mcp-shell/07-02-execution.md
key-decisions:
  - "沿用已批准的 snapshot API 合同，只删除 replay 对当前 Action 定义的重复读取"
  - "已正确的 snapshot/history/validate-only 行为补 characterization，不制造假 RED"
  - "最终一行 test-only 同 store 强化以定向 history 与最终 all-features Clippy 闭环"
requirements-completed: [SCOPE-07, API-01, MCP-06]
coverage:
  - id: D1
    description: "SQLite reload、scope apply、revoke 不混代，snapshot 返回前释放共同锁"
    requirement: SCOPE-07
    verification:
      - kind: integration
        ref: "07-02-evidence/storage-verified.log: request_snapshot 2 passed / 1 ignored"
        status: pass
    human_judgment: false
  - id: D2
    description: "固定定义的 submit/validate/replay/result 保留 fence/CAS/guard、本次授权及原 receipt/time"
    requirement: API-01
    verification:
      - kind: integration
        ref: "07-02-evidence/action-final.log: frozen_definition 3 passed; idempotency 9 passed"
        status: pass
    human_judgment: false
  - id: D3
    description: "同 external store 中旧 snapshot history 与当前 wrapper 使用各自定义并遵守当前 scope"
    requirement: MCP-06
    verification:
      - kind: integration
        ref: "07-02-evidence/history-same-store.log: frozen_history 1 passed"
        status: pass
    human_judgment: false
  - id: D4
    description: "真实 PostgreSQL metadata 竞争合同运行，交 07-13 强制执行"
    requirement: SCOPE-07
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/request_snapshot.rs: ignored PG contract"
        status: unknown
    human_judgment: true
    rationale: "本环境未运行真实 PG；需 07-13 的服务环境和执行证据，不能用人工确认替代测试"
duration: 32min
completed: 2026-10-29
status: complete
---

# Phase 07 Plan 02: 固定请求定义与验收修复 Summary

**修复 admission snapshot replay 的二次定义读取，并以共同锁竞争、零写入断言及固定 receipt/time 测试验证 Action/history 合同。**

## Scope

本 SUMMARY 记录已存在 07-02 实现的验收修复，不把历史实施日志当作本轮验收证据。`requirements-completed` 只表示 `[SCOPE-07, API-01, MCP-06]` 在本 PLAN 的 slice，本 Phase 全部 requirement 的完成状态由主会话在最终验收后派生。

## Performance

- 首条保留命令：2026-10-29T04:36:31Z；最终构建结束：2026-10-29T05:03:55.856Z；证据命令窗口约 27m24s。
- SUMMARY 计量时间：2026-10-29T05:08:07Z；本轮含收尾约 32min，初始化阅读发生更早。
- Tasks：2/2 本地验收完成；owned diff：56 个文件（5 个 Rust 文件、台账及 50 个证据文件）。
- `actuals.tokens`：从 ledger base 到 `plan_head_after` 的本执行者 owned diff 共 198764 字符，除以 4 为 49691，包含原始日志；排除共享状态/审查文档及本 SUMMARY。
- `actuals.commits`：`git rev-list --count 091b8b5..5e7f5fb` 实测 10，包含主会话并行文档提交，非本执行者独占数量；不包含随后 SUMMARY metadata commit。

## Accomplishments

- 两个 store 指向同一 SQLite 文件，以真实共同 metadata 锁和 Notify 覆盖 reload、scope apply、revoke；检查完整 definition/roles/bindings 代际与锁释放，复用同一合同到 ignored PG case。
- 真实 RED 复现外部删除 Action 后 replay 返回 `Err(ScopeDenied)`；生产只删除 replay 中的二次 `current_action_definition` 读取，沿用 admission snapshot。当前 grant、跨主体拒绝、原 receipt/time 断言保留。
- validate-only 成功、CAS、guard、stale 各分支均比较 edit/log/action_commands/outbox 基线；history 在持新定义的同 external store 对照旧 snapshot 与当前 wrapper，并覆盖 masking/revoke。
- workspace 单线程回归、fmt、default/all-features/embedded-pg Clippy 和 workspace build 均完成；TLS 图中无 `native-tls` 或 `openssl-sys`。

## Task Commits

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 02-1 | 双后端 metadata 共同锁及代际切换合同 | `df8b748` | `crates/ontoforge-storage/tests/request_snapshot.rs` |
| 02-2 RED | 外部移除 Action 后 replay 真实行为失败 | `930ee87` | `crates/ontoforge-action/tests/frozen_definition.rs` |
| 02-2 GREEN | replay 沿用准入快照，补 validate/history 合同 | `3c0fb76` | `crates/ontoforge-action/src/lib.rs`、frozen 测试 |
| 02-1/02-2 REFACTOR | 保留固定时钟/定义断言，收敛测试分配 | `fbf5b9f` | 三个测试文件及 `idempotency_pg.rs` |
| 02-2 强化 | 同 external store 对照 history 定义 | `366bb4c` | `crates/ontoforge-storage/tests/frozen_history.rs` |
| 证据 | 保存原始 RED/GREEN 与全部本地门禁 | `5e7f5fb` | `07-02-execution.md`、`07-02-evidence/` |

主会话审查/状态提交：`47f299b`、`16adf6b`、`0709e01`、`87f9367`。原五项审查问题均 resolved，WINDOWS#16 已由主会话关闭。最终审查见 `reviews/07-02-ACCEPTANCE-AUDIT.md`。

## Verification

所有 Cargo 子进程均设置 `CARGO_BUILD_JOBS=2`，保留增量 target，没有新增依赖。原始命令、退出码、UTC 时间及日志均存放在 `07-02-evidence/`。

| Evidence | Result |
|----------|--------|
| `replay-red.json`、`replay-red.junit.xml` | nextest 实际执行 1 个测试，1 个 assertion failed；非编译失败 |
| `tdd-red-gate.log/json` | `RED_EVIDENCE_OK`，exit 0 |
| `action-final.log/json` | frozen_definition 3 passed；idempotency 9 passed；0 ignored |
| `storage-verified.log/json` | frozen_history 1 passed；history_masking 12 passed / 1 ignored；request_snapshot 2 passed / 1 ignored |
| `history-same-store.log/json` | 最终 `366bb4c`：1 passed / 0 ignored |
| `workspace-tests.log/json`、`workspace-tests-totals.json` | 121 组，1279 passed / 0 failed / 107 ignored / 0 filtered；12m43s，构建 8m31s |
| `workspace-clippy-terminal.log/json` | `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` exit 0，约 17s |
| `workspace-clippy-default.log/json` | default workspace Clippy exit 0 |
| `embedded-pg-clippy.log/json` | embedded-pg all-targets Clippy exit 0 |
| `fmt.log/json` | `cargo fmt --all -- --check` exit 0 |
| `tls-native.log/json`、`tls-openssl.log/json` | 含 embedded-pg 的依赖图查询均 exit 101，返回 `did not match any packages`，对应依赖不存在 |
| `workspace-build.log/json` | `cargo build --workspace --locked` exit 0，1m18s |

workspace 全量测试锚定 `fbf5b9f`。随后只有 `366bb4c` 的一行 test-only 强化，生产代码未变；经主会话批准，以 `history-same-store` 和最终 all-features Clippy 覆盖该差异，不重复全量。构建门禁覆盖最终测试树。所有 Cargo 进程已结束。

## TDD Gate Compliance

- RED：`930ee87` 先提交可归因的 replay 行为失败，保留 nextest JSON/JUnit，并由 GSD `tdd-red-evidence` 判定 `RED_EVIDENCE_OK`。
- GREEN：`3c0fb76` 最小生产修复后，同一 replay 及原 idempotency 合同通过。
- REFACTOR：`fbf5b9f` 提取测试辅助函数并沿既有 `Box::pin(race(...))` 模式调整三处 PG 测试 future 分配；未删除断言或压制 lint，之后重复定向与完整门禁。
- Task 02-1 的共同锁行为已正确，仅新增 characterization；validate-only/history 新断言同样不制造假 RED。初次 snapshot import 编译错误和中间 Clippy 失败均保留，但不算业务 RED。
- 纯测试 lint 整理及最终一行测试强化未新增生产行为；TDD 跳过理由由主会话记录在共享 `findings.md`。
- Warp 与 `ralph-loop` 不可用，使用真实 PowerShell 执行，不声称调用，也不输出其 completion-promise。

## Decisions Made

采用已批准合同，无新增架构决策。沿用 wrappers、commit fence/CAS/guard 和当前授权；不增加历史 schema、不绑定原 receipt 与定义版本、不增加当前对象可见性重验。战略、Phase 和 brainstorming 本轮跳过，原因是继续已锁定 spec 的验收修复。

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] replay 再读当前定义导致固定快照请求拒绝**

- 发现于 Task 02-2，外部 store 移除 Action 后 admission snapshot replay 错误返回 `ScopeDenied`。
- 删除 `crates/ontoforge-action/src/lib.rs` 中重复的当前定义存在性查询，以真实 RED 和最终 12 个 Action 测试验证，commit `3c0fb76`。

**2. [Rule 3 - Blocking] 验收测试触发严格 Clippy**

- history 测试拆出辅助函数；三处 `idempotency_pg.rs` large_futures 沿已批准既有模式使用 `Box::pin`，commit `fbf5b9f`。
- 最终 workspace all-targets/all-features 与 embedded-pg Clippy 均 exit 0。无生产算法调整。

原历史实施中 history visibility 顺序修复只在 `07-02-execution.md` 历史区域保留，不冒充本轮新增修复。

## Known Stubs

无阻碍本 PLAN 目标的 stub。扫描未发现新增 TODO/FIXME 或占位实现；既有 `ActionError::ScopeDenied` 英文错误中的 `not available` 不是 stub。

## Deferred Issues

- 真实 PG 竞争合同及环境门控 PG/Trino/private 测试未执行；workspace 中 107 ignored 不计通过。真实服务验收由 07-13/14 承接，不能用本地 SQLite 或静态发现替代。
- 无新增生产 endpoint、auth 路径、文件访问或 schema 信任边界；T-07-04/05/06 的本地缓解已验证，PG 运行证据仍待后续 PLAN。
- 共享 WINDOWS、STATE、ROADMAP、REQUIREMENTS、task_plan/progress/findings 由主会话统一维护，本执行者遵守写入边界。

## Next Phase Readiness

本 PLAN 的本地 slice 可供后续 REST/MCP 调用。PR #7 按既定边界保持 Draft/OPEN，不合并、不发布。主会话接收本 SUMMARY 后同步共享进度；整个 Phase 的 requirement 完成须等待相应完整验收。

## Self-Check: PASSED

- 已检查本 PLAN 的生产/测试入口、RED/全量/构建证据及 SUMMARY 文件均存在。
- 已验证 Task Commits 表及主会话审查/状态的全部 10 个提交均为当前 HEAD 的祖先。
- `git diff --check` exit 0；本轮证据已提交，无意外 tracked file deletion。剩余无关 untracked 文件保持原状。
- 本地门禁全部完成；真实服务未跑项与 test-only 差异边界已显式记录，没有把 ignored 计为 passed。
