---
phase: 06-l2-raw-dql-toolsets-audit
plan: 10
subsystem: trino-acl
tags: [trino, acl, lifecycle, fail-closed, tdd]
requires:
  - phase: 06-04
    provides: L2 ACL attestation and read-only SQL boundary
  - phase: 06-08
    provides: query integration and real Trino acceptance corpus
provides:
  - Explicit bounded L2 ACL monitor startup, periodic attestation and shutdown
  - Owner-only enable with cross-generation pending cleanup obligations
  - Stateful query-ID lifecycle regression and real Trino 483 evidence
affects: [07-rest-mcp-shell, L2-03, L2-04]
tech-stack:
  added: []
  patterns: [monitor lease, generation and epoch tickets, token-checked serial cleanup]
key-files:
  created:
    - crates/ontoforge-engine/src/trino/l2_monitor.rs
    - crates/ontoforge-engine/tests/l2_monitor.rs
  modified:
    - crates/ontoforge-engine/src/trino/l2_health.rs
    - crates/ontoforge-engine/src/trino/decode.rs
    - crates/ontoforge-engine/src/printer/l2.rs
    - crates/ontoforge-engine/tests/l2_health.rs
    - crates/ontoforge-engine/tests/trino_l2_acl_it.rs
    - crates/ontoforge-query/tests/definition_snapshot.rs
    - crates/ontoforge-query/tests/l2_discovery.rs
    - crates/ontoforge-query/tests/l2_execute.rs
key-decisions:
  - "ADR-0028 A/1: Phase 6 delivers library lifecycle; deployment owns bounded active_query_sql; Phase 7 owns service lifecycle and HTTP /health."
  - "Monitor ownership and successful cleanup are required to enable; manual checks cannot bypass an active or stopped monitor lease."
requirements-addressed: [L2-03, L2-04]
requirements-completed: []
actuals:
  tokens: 20668
  tasks: 2
  commits: 10
commits: 10
plan_head_before: cbbf87e64df1d715889260befe592a3db2df45c8
plan_head_after: 9738166f88d92e720b32f4de65a5a803e6e6dbc8
duration: 37min
completed: 2026-10-28
status: completed
final_gates:
  workspace: passed-e81127f
  clippy: passed-e81127f
  fmt: passed-e81127f
  tdd_review: passed-8-of-8-with-transient-read-risk
coverage:
  - id: D1
    description: Bounded startup, fail-closed lifecycle and fresh attestation recovery
    requirement: L2-03
    verification:
      - kind: integration
        ref: crates/ontoforge-engine/tests/l2_monitor.rs
        status: pass
      - kind: unit
        ref: trino::l2_monitor::tests::old_cleanup_response_and_owner_cannot_clear_or_cancel_new_probe
        status: pass
    human_judgment: false
  - id: D2
    description: Actual RUNNING isolation and cleanup on restricted and loose Trino 483
    requirement: L2-04
    verification:
      - kind: integration
        ref: crates/ontoforge-engine/tests/trino_l2_acl_it.rs
        status: pass
    human_judgment: false
---

# Phase 06 Plan 10: L2 ACL Monitor Summary

部署方有界活跃 query 驱动库级 startup/periodic attestation；只有同一 monitor owner 完成真实 RUNNING 隔离证明并确认清理后才能启用 L2。

## 状态与边界

实现和限定测试已完成。主会话正在执行唯一一次 workspace 回归，并统一负责 Clippy、fmt、T2/T1 状态同步和最终验收；这些 gate 尚未返回，因此本文暂不声明完整计划、Phase 6 或 L2-04 已完成。独立审查记录见 `06-10-REVIEW.md`。

## Accomplishments

- `L2AclConfig.active_query_sql: Option<String>` 属于 `[trino.l2_acl]`。部署方提供只读 workload；现有 TOML 未提供该字段时仍能解析，monitor 保持 disabled 并按周期重试。通过现有 AST/Display SQL gate、allowed catalog 限定和 LIMIT 2，不能访问 system/edit/outside catalog。
- startup 等待首轮 attestation；默认 interval 30s、整体 check_timeout 20s，计时包含 recheck/admission 锁等待。独立 attestation TTL 为 60s，query_max_execution_time/query_max_run_time 均为 25s。部署 SQL 若过快结束则不能证明活跃隔离，保持 disabled。
- 每轮自产新 query ID，从 QUEUED 等待到 RUNNING，在 L2 不可见观测前后由 ontology 对 `system.runtime.queries` 证实同 ID 实际 RUNNING。两个身份必须指向同一 coordinator；负探针要求对象已存在且精确 PERMISSION_DENIED。
- pending DELETE 由同一 engine 的 health 共享，跨 monitor generation 保留。新周期先清旧 query；取消串行、整体 2s 有界，只有对应 token 的 2xx/404 响应可清 slot。失败保留请求以便重试，旧 Drop/响应不能取消或清掉新 probe。
- generation/check_epoch 阻止旧检查覆盖新状态。monitor 注册期间 manual 成功不能启用；shutdown/drop 后 stopped 继续拒绝 manual 启用。shutdown 报告自身或继承的未完成清理，不能返回虚假成功。
- mock registry 按 ID/owner/state 建模，GET 读取请求 URI，DELETE 成功后移除 query；未知或取消 ID 不能伪造 RUNNING。真实 startup 清理断言使用独占 alias，避免与其他并行用例共享 SQL 前缀。

## API 用法

宿主使用同一对 engine 实例，并保留返回的 handle：

```rust,ignore
let ontology = Arc::new(engines.ontology);
let l2 = Arc::new(engines.l2.expect("configured L2"));
let monitor = l2.clone().start_l2_acl_monitor(ontology).await?;
// Serve requests using these same engine instances.
monitor.shutdown().await?;
```

默认配置使用 `start_l2_acl_monitor`；测试或宿主可使用 `start_l2_acl_monitor_with_options`，参数必须非零且不超过默认 interval/check_timeout 上限。handle 被 drop 或 startup future 被取消会立即禁用并尝试有界清理。进程退出时无法保证网络 DELETE 到达，远端 query 的两个 session 时间限制提供最终边界；不增加本地持久化或 DB。

Phase 7 仍需将该 handle 接到服务启动、持有、优雅关闭和 HTTP `/health`。本计划不创建 HTTP endpoint，也不宣称部署已接线。

## TDD Gate Compliance

全部 RED 都来自已编译、实际执行的行为测试，exitCode 101；对应 JSON 经 `gsd-tools.cjs check tdd-red-evidence` 返回 `RED_EVIDENCE_OK`：

| Evidence | 预期失败 |
| --- | --- |
| `06-10-RED-task1.{xml,junit.json}` | 已结束的 query 不能证明活跃隔离 |
| `06-10-RED-task2.{xml,junit.json}` | 默认 disabled 骨架不能满足 startup 完整证明 |
| `06-10-RED-CR01.{xml,junit.json}` | DELETE 500 后 manual 成功错误返回 Enabled |
| `06-10-RED-CR02.{xml,junit.json}` | 新 generation 遗忘旧 pending 并错误 Enabled |
| `06-10-RED-CR03.{xml,junit.json}` | 继承清理失败时 shutdown 错误返回 Ok(()) |

先提交 RED，再实现 GREEN。token/旧 owner 精确防线新增回归验证已有修复；`!old.is_finished()` 在替换 token 前断言真实响应仍在途，且中间无 await，避免延迟调度导致假绿。

## 验证证据

1. `cargo test -p ontoforge-engine --test l2_monitor --test l2_health --lib --locked -j 2 -- --nocapture`：unit 55/55、health 10/10、monitor 16/16，共 81 passed，0 failed。
2. 最后测试断言后：`cargo test -p ontoforge-engine --lib old_cleanup_response_and_owner_cannot_clear_or_cancel_new_probe --locked -j 2 -- --nocapture`：1 passed，0 failed。
3. `cargo test -p ontoforge-engine --test trino_l2_acl_it --locked -j 2 -- --ignored --nocapture`：真实 Trino 483 默认并行 6/6，8.60s。覆盖 restricted ACL、loose ACL fail-closed、short query 拒绝、startup 活跃 query 清理；既有 32 条 allow corpus 全部 FINISHED。
4. 主会话在 `80d053b` 执行 `cargo test --workspace --locked -j 2 -- --test-threads=1` 退出 0：117 个结果组、1251 passed、103 ignored、0 failed，编译/链接 27m57s。完整本地日志：`C:/Users/easyi/AppData/Local/Temp/ontoforge-phase06-10-workspace-final.log`。
5. Clippy 暴露测试 fixture 风格问题后，`5988da8`/`e81127f` 仅修正测试表达、提取 mock helper 和移动 unit module，生产文本不变。执行器在 `e81127f` 运行 unit 55/55、monitor 16/16，完整 workspace Clippy `-D warnings` 退出 0（25.79s），fmt/diff 通过。主会话在该最终快照重跑完整 workspace 退出 0：117 个结果组、1251 passed、103 ignored、0 failed，编译/链接 29m55s；日志 `C:/Users/easyi/AppData/Local/Temp/ontoforge-phase06-10-workspace-accepted.log`。最终原始 TDD review 重跑 8/8、block:false（5.87s）；间歇 Git 漏读风险仍保留在 findings，不宣称 GSD 已根治。Phase 6 目标验证另行裁定。

真实环境由主会话启动：restricted `127.0.0.1:58081`、loose `127.0.0.1:58082`，hive.sales fixture 已 seed。此 executor 没有清理服务，避免影响主会话后续验证。远端 CI 尚未执行。

## Task Commits

1. `eb69016` test：拒绝用已结束查询证明活跃隔离。
2. `e1a45da` feat：同 coordinator 与 RUNNING 前后验证，generation/check_epoch。
3. `539c967` chore：默认禁用 API 骨架供可运行行为 RED。
4. `30ff079` test：启动、周期恢复与取消清理生命周期。
5. `a7a161e` feat：有界启动、周期 ACL 证明、停止清理及真实测试。
6. `97d3eef` test：manual/restart 绕过清理 RED、stateful fixture 和真实测试 alias 隔离。
7. `0cedb99` fix：跨代清理义务与 owner-only enable。
8. `8006851` test：继承清理失败的 shutdown 返回合同 RED。
9. `5f32e7f` fix：shutdown 报错与旧响应/旧 owner 精确防线。
10. `9738166` test：固定旧响应回归的在途交错前提。

`commits` 通过 `.git/gsd-plan-head-before-06-10` 到 `plan_head_after` 的 `git rev-list --count` 实测为 10；文档收尾提交另计。actuals.tokens 为该区间实际 diff 82672 chars / 4 = 20668，不是模型 token 消耗。

## Final Gate Follow-Ups

原任务 ledger 保留上述 10 个提交。最终门禁额外暴露的测试表达及布局修正单列：

- `80d053b`：wiremock HTTP 方法使用类型化常量，SQL 源码边界守卫不变。
- `5988da8`：真实 ACL fixture 的冗余闭包改为等价方法引用。
- `e81127f`：mock DELETE 响应抽取 helper、字符串赋值使用 `clone_into`、unit module 原样移到文件末尾；生产文本逐字核对一致。

增量独立复审覆盖到 `e81127f`，开放发现 0，未增加 lint suppression；详 `06-10-REVIEW.md`。未为纯测试风格整理新增业务 TDD，原行为回归与完整 Clippy 作为验证。

## Deviations from Plan

### Auto-fixed Issues

1. [Rule 1 - Bug] 独立审查 CR-01：manual recheck 能绕过 monitor cleanup failure。增加 owner ticket 和 disabled 保留；RED `97d3eef`，GREEN `0cedb99`。
2. [Rule 1 - Bug] 独立审查 CR-02：重启遗失旧 pending。改为 engine 共享、token 校验和串行取消；RED `97d3eef`，GREEN `0cedb99`。
3. [Rule 1 - Bug] 补核发现继承 pending 时 shutdown 虚报成功；RED `8006851`，GREEN `5f32e7f`。
4. [Rule 1 - Bug] WR-01/WR-02：真实并行测试共享 SQL 前缀、mock 虚构活跃状态，`97d3eef` 修复，81 项及真实并行 6 项通过。

`ralph-loop` 本机未安装，没有声称调用。按既有明确目标执行测试、修复、验收循环。宽 engine/query 回归在链接阶段由本 executor 停止，未把它记作成功；主会话接管 workspace 回归避免重复 Cargo。

## Known Stubs

无阻碍本计划目标的 stub；新增生产代码未检出 TODO/FIXME/placeholder。Phase 7 服务接线是已批准的后续边界，不是本计划虚假占位。

## Threat Flags

| Flag | File | Description |
| --- | --- | --- |
| threat_flag: network-cleanup | `crates/ontoforge-engine/src/trino/l2_monitor.rs` | 新增部署 probe 的 POST/GET/DELETE 生命周期，复用现有 origin 校验、身份请求和 AST SQL 边界；限制两个 query session 时间上限，并以 owner/token 防止跨代误清理。 |

## Self-Check: PASSED

已确认新增 `l2_monitor.rs`、`tests/l2_monitor.rs` 和全部 5 组 RED 证据文件存在；上列 10 个提交均是当前 HEAD 的祖先。最终 workspace/Clippy/fmt 与 TDD review 已通过；本计划完成不替代 Phase 6 目标验证或 Phase 7 服务验收。
