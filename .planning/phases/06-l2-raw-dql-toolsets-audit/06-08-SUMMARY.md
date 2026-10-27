---
phase: 06-l2-raw-dql-toolsets-audit
plan: 08
subsystem: testing
tags: [ci, trino, postgres, acl, corpus, audit]
requires:
  - phase: 06-l2-raw-dql-toolsets-audit
    provides: "06-02 的 PostgreSQL 审计 conformance、06-03 的 104-ID corpus、06-04 的独立 Trino ACL fixture"
provides:
  - "CI 非零发现后执行独立 Trino 483 ACL 与 corpus 行为、PostgreSQL read audit"
  - "Windows workspace 回归和测试写入调用白名单对账"
affects: [phase-06-verify, phase-07]
actuals:
  tokens: 1457
  tasks: 2
  commits: 3
commits: 3
plan_head_before: 4699e80b02bdd9985073e9e35552e12ea99c5fbd
plan_head_after: ea695ed480d56744835bf69d505e5531eef61314
tech-stack:
  added: []
  patterns: ["独立 ACL/宽松对照服务的 CI 非零发现门禁", "真实服务与 corpus 分类分别执行"]
key-files:
  created: []
  modified:
    - .github/workflows/ci.yml
    - crates/ontoforge-engine/tests/trino_l2_acl_it.rs
    - crates/ontoforge-action/tests/entrypoints.rs
key-decisions:
  - "CI 对受限 Trino 与 PG audit 先计数后执行，不接受零发现。"
  - "Windows workspace 测试使用单线程 libtest，以免 16 MiB 测试与 1 秒审计期限竞争。"
requirements-completed: [L2-01, L2-02, L2-03, L2-04, AUDIT-01]
coverage:
  - id: D1
    description: "独立受限 Trino 483 的 ACL、自检、G11 拒绝和 G16/G17 Display 真执行"
    requirement: L2-04
    verification:
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_l2_acl_it.rs#trino_l2_acl_"
        status: pass
    human_judgment: false
  - id: D2
    description: "104-ID corpus 的生产 gate 分类与零 POST 检查"
    requirement: L2-01
    verification:
      - kind: integration
        ref: "crates/ontoforge-engine/tests/l2_gate.rs#l2_corpus_"
        status: pass
    human_judgment: false
  - id: D3
    description: "真实 PostgreSQL read audit conformance 非零发现和执行"
    requirement: AUDIT-01
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/conformance_pg.rs#audit_"
        status: pass
    human_judgment: false
duration: 69min
completed: 2026-10-28
status: complete
---

# Phase 6 Plan 08: 真实服务 CI 门禁 Summary

CI 接入独立受限 Trino 483、宽松规则对照、104-ID corpus 和 PostgreSQL 审计的非零发现与实际执行。

## Performance

- **Duration:** 69 min（03:21 至 04:30，Asia/Shanghai；含 Windows workspace 首次链接）
- **Tasks:** 2
- **Files modified:** 3 个代码/CI 文件，另有本 SUMMARY、VALIDATION 与 deferred-items 记录

## Accomplishments

- CI 保留已有无 ACL Trino，另启动 `ci/query-services.sh trino-l2` 的受限/宽松对照容器。真实 ACL 测试发现 3 项并执行；经 `recheck_l2_acl` 启用的 `TrinoEngine::execute` 发送 G16/G17 的 Display SQL，G11 在发送前拒绝。
- Windows/Linux test job 对 104-ID corpus 过滤发现 6 项后实际运行；PG integration job 对 `audit_` 过滤发现 5 项后实际运行。发现数为零或服务缺失均非零退出。
- 修正 Phase 6 审计事务写入与既有 `entrypoints` 精确白名单的对账；Windows workspace 全量单线程回归通过。

## Task Commits

- Task 1：`8b97ef8`；Clippy 整理：`4581a8f`。
- Task 2：`ea695ed`。

## Verification

- 实测代码 HEAD `ea695ed480d56744835bf69d505e5531eef61314`；受限 Trino ACL `--ignored --list` 发现 3 项、`--ignored --nocapture` 3/3 通过；corpus `l2_corpus_` 发现 6 项、6/6 通过，完整 `l2_gate` 16/16 通过。Trino 483 的 G16 返回 0 行、G17 返回 1 行；越界 catalog 返回 `PERMISSION_DENIED`。
- `conformance_pg audit_ --features embedded-pg --ignored --list` 发现 5 项，实际 PostgreSQL 5/5 通过；SQLite audit 4/4，query `audit_pk` 9/9、`audit_l0` 9/9（另 1 项按原设计需外部 PG，普通运行 ignored）。
- 最终 `cargo test --workspace --locked -- --test-threads=1` 通过：116 个结果组、1231 passed、98 ignored、0 failed（约 3 分钟，Windows）。`cargo clippy --workspace --all-targets --locked -- -D warnings`、`cargo fmt --all --check`、`git diff --check` 通过。
- 未触发 GitHub Actions 远端 workflow；本机在 Docker Trino 483 和 embedded PostgreSQL 上执行了 CI 对应的目标。远端 GitHub SHA 的执行状态仍需在 PR/推送后核对。

## TDD

本计划仅接线 CI 和补充已有集成测试，没有改动核心业务实现；按项目判据跳过新 RED/GREEN 周期。相关测试、真实服务和 workspace 回归均实际执行。主会话负责将该跳过理由同步到其独占的 `findings.md`。

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Phase 6 审计事务未列入写入调用白名单**
- 初次 `cargo test --workspace --locked` 在 `production_mutation_callers_are_action_and_audited_import` 失败：`ontoforge-storage/src/read_audit.rs:commit` 实际 2 次，预期映射缺失。只修改 `crates/ontoforge-action/tests/entrypoints.rs` 的测试名和精确预期计数，不改生产写入逻辑；目标重跑 4/4 通过。提交 `ea695ed`。

**2. [Rule 3 - Blocking] 16 MiB 测试受并行审计墙钟约束影响**
- 第二次默认并行 workspace 在 `security::cap_counts_final_masked_row_and_version` 的首个 `.unwrap()` 得到 `AuditUnavailable`；该用例单独运行通过。审计最长 pending 1 秒是 fail-closed 生产合同。CI 的 workspace libtest 改为 `--test-threads=1`，最终全量通过；不放宽审计期限。Grok 第二意见同意短期串行，指出慢机器残余波动和未来提取纯计算断言的必要性。提交 `ea695ed`，后续风险见 `deferred-items.md`。

**3. [Rule 1 - Bug] 新增 Trino 测试违反 Clippy**
- 长函数和布尔转整数 lint 经提取 corpus helper 修正；完整 Clippy 通过。提交 `4581a8f`。

## Self-Check: PASSED

三个任务提交均为当前 HEAD 的祖先；上述三个代码/CI 文件存在，未删除跟踪文件。SUMMARY、VALIDATION 与 deferred-items 已落盘。
