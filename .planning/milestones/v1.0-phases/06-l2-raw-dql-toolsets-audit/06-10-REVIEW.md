---
phase: 06-l2-raw-dql-toolsets-audit
plan: 10
reviewed: 2026-10-28T02:22:04Z
depth: deep
diff_base: 6e7fe8f
snapshot_head: e81127fa41099365c44c2c42750024efad6653cb
files_reviewed: 11
files_reviewed_list:
  - crates/ontoforge-engine/src/printer/l2.rs
  - crates/ontoforge-engine/src/trino/decode.rs
  - crates/ontoforge-engine/src/trino/l2_health.rs
  - crates/ontoforge-engine/src/trino/l2_monitor.rs
  - crates/ontoforge-engine/src/trino/mod.rs
  - crates/ontoforge-engine/tests/l2_health.rs
  - crates/ontoforge-engine/tests/l2_monitor.rs
  - crates/ontoforge-engine/tests/trino_l2_acl_it.rs
  - crates/ontoforge-query/tests/definition_snapshot.rs
  - crates/ontoforge-query/tests/l2_discovery.rs
  - crates/ontoforge-query/tests/l2_execute.rs
findings:
  critical: 0
  warning: 0
  info: 0
  total: 0
resolved_findings:
  critical: 3
  warning: 2
  total: 5
status: clean
---

# Phase 06-10: Code Review Report

## Narrative Findings (AI reviewer)

最终快照复审未发现未解决的正确性、安全或测试可靠性缺陷。初轮 3 项 BLOCKER 和 2 项 WARNING 均已关闭；下列历史记录保留触发条件、修复位置和回归证据，不属于当前开放 findings。

## Resolved Findings

### CR-01: [BLOCKER] manual recheck 能绕过 monitor 的清理失败

**File:** `D:/DataSpellProjects/ontoforge/crates/ontoforge-engine/src/trino/l2_health.rs:105-111,120-129,198-210`
**Confidence:** 10/10
**Issue:** `begin(None)` 给公开 manual recheck 分配同一 generation 下的合法 ticket；`complete` 仅验证 generation、check_epoch 和 stopped，成功分支直接写 `Enabled`，未区分 monitor owner。精确交错：monitor 启动完成检查，DELETE 返回 500，状态为 `CleanupFailed`；宿主持有该 monitor；manual recheck 使用另一个已确认 RUNNING 的外部 query ID 成功；其 `complete(ticket, Enabled)` 重新开放 L2，但 monitor 的 pending DELETE 从未得到 2xx/404。该结果违反已批准的 monitor 租约及清理失败必须 fail-closed 合同。
**Evidence:** `if (current.generation, current.check_epoch) == ticket && !current.stopped`；manual 成功分支 `Ok(()) => L2HealthStatus::Enabled`。
**Fix:** 在 health 状态锁内区分 owner ticket。存在 monitor 租约时，manual 只能诊断或禁用，不能写 `Enabled`；只有实际 monitor cycle 完整检查并成功清理后才能启用。无 monitor 时保留原 manual attestation 行为。
**Regression:** monitor DELETE=500 后保持 handle，显式创建外部 RUNNING query，再调用 manual recheck；状态必须仍为 Disabled，公开 `execute` 不得提交 SQL。随后仅允许成功完成清理的 monitor 新周期恢复。
**Resolution:** 已关闭。`0cedb99` 在 `l2_health.rs:114-149` 引入 owner ticket，并在同一状态锁内拒绝 monitor 租约期间的 manual Enabled 写入；`l2_health.rs:171-180` 保证失败 shutdown 后 manual 也不能重新启用。`tests/l2_monitor.rs:609-638` 覆盖两种交错。已读取 `97d3eef` 的 RED JUnit，原断言实际收到 `Enabled`；执行者报告最终 monitor 16/16 GREEN。

### CR-02: [BLOCKER] 重启 monitor 丢失旧 generation 的清理义务

**File:** `D:/DataSpellProjects/ontoforge/crates/ontoforge-engine/src/trino/l2_monitor.rs:40-51,64-73,116,202-213`
**Confidence:** 10/10
**Issue:** pending probe 仅属于单个 monitor，`shutdown` 清理失败仍先调用 `stop` 释放 monitor 注册，Drop 的重试也失败后没有 engine 级持有者。新 monitor 使用 `Arc::new(PendingProbe::default())`，其首轮 `pending.cancel()` 把 None 视为清理成功；新 probe 成功清理后可 Enabled，而旧 query 的清理从未确认。按补充进 `06-10-PLAN.md` 的已批准约束，pending 清理义务必须跨 generation 保留，重启不能绕过 fail-closed。
**Evidence:** `let pending = Arc::new(PendingProbe::default());`；`let Some(delete) = delete else { return Ok(()); };`；`stop` 设置 `current.monitoring = false`。
**Fix:** 同一 engine 持有跨 generation 的 pending 状态。新 monitor 必须先清旧 probe，清理串行化并携带捕获的 query/token，只有匹配时才能清除 slot；旧 Drop 仅针对自己的 generation，不能删除或取消新 probe。
**Regression:** 旧 DELETE 持续失败时停止并重启，新 monitor 不得提交新 probe 或 Enabled；允许旧 DELETE 成功后方可提交、检查和清理新 probe。延迟旧 DELETE 响应及旧 Drop，证明它们不能清除或取消新 generation 的 pending query。
**Resolution:** 已关闭。`0cedb99` 将 pending 放入同一 engine 的 health 状态，`l2_monitor.rs:113-126` 复用它；`:203-246` 串行清理且仅清除匹配 token；`:329-335` 先清继承义务再提交新 probe。`tests/l2_monitor.rs:641-674` 覆盖重启失败、禁止新 POST、恢复后启用。`5f32e7f` 的 `l2_monitor.rs:256-307` 证明旧响应和旧 owner 不能清除或取消新 probe，`9738166` 进一步在替换 token 前断言旧响应仍 pending。已读取 `97d3eef` RED JUnit 的 `Some(Enabled)` 与 `CleanupFailed` 差异；执行者报告 monitor 16/16 及最后加固 unit 1/1 GREEN。

### CR-03: [BLOCKER] 继承清理失败时 shutdown 错报成功

**File:** `D:/DataSpellProjects/ontoforge/crates/ontoforge-engine/src/trino/l2_monitor.rs:40-52,174-187`
**Confidence:** 10/10
**Issue:** CR-02 修复后，新 monitor 会继承旧 generation 的 pending，但 shutdown 的 `cancel(Some(new_generation))` 按 owner 过滤旧条目并返回成功。旧 DELETE 持续失败时，第二个 monitor 的 shutdown 因此可能返回 `Ok(())`，违反 shutdown 必须报告未确认清理的合同。
**Fix:** 保持 owner 限定取消，并在返回成功前检查当前 generation 及更早的清理义务是否已清除；旧 handle 不得删除更新 generation 的 query。
**Regression:** `tests/l2_monitor.rs:702-722` 创建跨 generation 未解决 DELETE，要求第二个 shutdown 返回错误。
**Resolution:** 已关闭。执行者通过 `8006851` 先添加 RED；已读取其 JUnit JSON，断言为 `shutdown must report inherited unresolved cleanup`，原结果为 `Ok(())`。`5f32e7f` 在 `l2_monitor.rs:45-48` 调用 `confirm_cleared_through`，后者在 `:174-187` 检查仍 pending 的旧或自身 generation；最终 monitor 16/16 GREEN 由执行者报告。

### WR-01: [WARNING] 真实清理测试会等待其他并行用例的 query

**File:** `D:/DataSpellProjects/ontoforge/crates/ontoforge-engine/tests/trino_l2_acl_it.rs:335-357`
**Confidence:** 9/10
**Issue:** 清理断言使用 `query LIKE 'SELECT count(*) FROM demo.sf100000.orders%'` 查询整个 coordinator。第 197-203 行的旧活跃 query 测试提交相同 SQL；`.github/workflows/ci.yml:153` 运行 ignored suite 时未设置串行线程。两个用例并行时，自身 probe 已正确取消也会因另一个测试的 RUNNING query 超过 3 秒而误失败。
**Fix:** 为该测试配置独占 SQL alias 并仅匹配该独占语句，或追踪自身 query ID，避免共享前缀匹配。
**Regression:** 使用 CI 的默认并行 ignored suite 运行两个用例，清理断言不能观察到其他用例的 probe。
**Resolution:** 已关闭。`97d3eef` 在 `trino_l2_acl_it.rs:335-345` 为 probe 和清理观测统一使用独占 `monitor_startup_probe` alias。执行者以 CI 默认并行模式实跑真实 Trino suite 6/6 GREEN，耗时 8.60s；32 条 allow SQL 均为 FINISHED。

### WR-02: [WARNING] mock 能为已取消或不存在的 query 提供假活跃证明

**File:** `D:/DataSpellProjects/ontoforge/crates/ontoforge-engine/tests/l2_monitor.rs:33-36,45-51,70-85`
**Confidence:** 10/10
**Issue:** DELETE 只递增计数，不改变 query 状态；runtime 查询直接把 SQL 中的任意 ID 回显为 RUNNING；GET 按全局最新 submissions 生成 ID，未按 URI 查找 query。因而将 cleanup 错误地提前到 `run_checks` 前，或观察错误 ID，成功与周期恢复测试仍能获得假的 RUNNING 证明。该 fixture 无法阻断本次核心生命周期合同的回归。
**Fix:** fixture 维护 query ID、owner 和状态 registry。POST 创建条目，GET 按 URI 读取，成功 DELETE 转终态，runtime 查询只返回实际存在且身份可见的条目；manual 外部 query 必须显式 seed。增加 DELETE 响应仍 pending 时不得 Enabled 的断言。
**Regression:** fixture 自测需证明未知 ID 无行、已取消 query 不再 RUNNING、查询其他 ID 不会回显；成功 monitor 必须在真实活动状态下完成前后观测，且等待 DELETE 成功后启用。
**Resolution:** 已关闭。`97d3eef` 在 `tests/l2_monitor.rs:30-153` 使用包含 ID、owner、state 的 registry；POST 创建、GET 按 URI、成功 DELETE 移除，runtime 仅查询存在且身份可见的 ID，manual query 显式 seed。`:678-698` 证明 DELETE 响应尚 pending 时不能 Enabled。当前 registry 分支及所有调用已复核，执行者报告 monitor 16/16 GREEN。

## Review Scope And Limits

本轮先执行 GSD 高置信缺陷检查，再应用 `C:/Users/easyi/.agents/skills/gstack/review/SKILL.md`、`checklist.md` 及 security、testing、maintainability、API contract、red-team 检查项。独立 specialist 串行复核 security/adversarial、testing/maintainability 及 red-team；新增结论均由主 reviewer 对当前源代码独立核验。未提供 fallow structural substrate。

审查基线为 `git diff 6e7fe8f -- crates/ontoforge-engine crates/ontoforge-query/tests`，包含已提交与未提交内容。按 ADR-0028 和 `06-10-PLAN.md` 限定为 Phase 6 库级增量，Phase 7 lifecycle 持有和 HTTP `/health` 不属于本次签收。未审查无关 docs 改动。仓库根目录不存在 `AGENTS.md`、`.codex/skills/`、`.agents/skills/` 或 `.codegraph/`，已遵循会话提供的项目规范；GSD `agent-skills gsd-code-reviewer --skills-only` 未返回配置 skill。范围文件均未被 gitignore 排除。

完整修复复审基线为 `9738166f88d92e720b32f4de65a5a803e6e6dbc8`，最新增量签署快照为 `e81127fa41099365c44c2c42750024efad6653cb`；从该基线到最新快照的增量仅涉及下述测试 matcher、closure、unit module 重排及 fixture 整理。受影响源文件 `git status --short` 无输出。SHA256：

| File | SHA256 |
| --- | --- |
| `src/trino/l2_health.rs` | `A3D1EF043468BDD30FBAD3F4FC3AED07A878AA77284A21CBF2FAE5830A7B06A3` |
| `src/trino/l2_monitor.rs` | `5D7E63F134F67E9231E0EB82ADA268FEB0F9415D32F367794A300F974F716A8B` |
| `tests/l2_monitor.rs` | `33C9976F5B165EF98348F729AF08163F2CE7382B6BC0D24182D19499F5B3AACD` |
| `tests/trino_l2_acl_it.rs` | `0CE4DAB88AA3B61AF8BE2B2DDC4957CC579167D36E8554E0E4973A54CC0BAF0A` |

上表路径均相对 `crates/ontoforge-engine/`。

本 reviewer 未运行 Cargo，以避免和执行者竞争编译资源；未把其报告的测试结果当作本 reviewer 亲自执行的结果。已读取 3 个 BLOCKER 的 RED JUnit 证据，并独立检查当前修复和回归断言。执行者报告 `cargo test -p ontoforge-engine --test l2_monitor --test l2_health --lib --locked -j2` 得到 lib 55/55、health 10/10、monitor 16/16 GREEN；最后加固 unit 单独 1/1 GREEN。真实 Trino 默认并行 6/6 GREEN，32 条 allow SQL 均 FINISHED。已执行限定范围 `git diff --check`，无输出。本 reviewer 只写本报告与经授权的 `06-SECURITY.md` 增量，未修改 production 或 tests；workspace/Clippy 验证由主会话负责。

正式 PR 尚未创建。本轮没有执行正式 PR 的 base/remote/评论核验、外部 Codex CLI gate、CI 状态门禁或发布审批；没有推送、外部评论、插件升级或 routing 修改，也没有写入声称正式 PR gate 已完成的记录。Performance 不在本次 v1 审查范围。

## Re-review

在上述最终快照上，5 项历史发现均已关闭，当前开放 findings 为 0。结论仅覆盖明确列出的 11 个文件和 Phase 6 库级合同，不构成正式 PR 门禁或 Phase 7 服务生命周期签收。

### HTTP Matcher Incremental Re-review

2026-10-28T01:39:03Z 直接核对 `git show 80d053b` 和当前源码：`crates/ontoforge-engine/src/trino/l2_monitor.rs:258,263` 的 `method("DELETE")` 改为 `method(reqwest::Method::DELETE.as_str())`，均位于 `:249` 开始的 `#[cfg(test)]` 模块。两者仍向 wiremock 提供相同的 HTTP DELETE 方法；旧响应延迟、路径、断言和 production 逻辑均未改动，未发现新的正确性、安全或测试可靠性风险。

主会话报告 workspace 首轮 SQL 源码边界守卫误识别 HTTP 字面量，原守卫保持不变，修正后的定向守卫 1/1 通过。本次仅签署这两处测试表达修正及上述精确 commit/SHA256；前述 focused/真实 Trino GREEN 仍是修正前基线的执行者证据。本 reviewer 未运行 Cargo，最新 workspace 与 Clippy 的最终结果由主会话负责，本报告不提前签署这两个门禁。未运行 `ralph-loop`（本机未安装）。

### Closure Incremental Re-review

2026-10-28T02:15:09Z 直接核对 `git show 5988da8`：唯一变更为 `crates/ontoforge-engine/tests/trino_l2_acl_it.rs:353` 的 `is_none_or(|rows| rows.is_empty())` 改为 `is_none_or(Vec::is_empty)`。这是 `Option<&Vec<Value>>` 上相同方法的直接引用，None、空数组和非空数组的判定均保持一致；循环退出条件、断言及 production 代码未变，未发现新的正确性、安全或测试可靠性风险。

主会话报告 `80d053b` 的 workspace 为 1251 passed、103 ignored、0 failed；该结果属于本次单行修正前的快照。本次仅签署 `5988da8b21147b87363606677087e8ee158d3779` 及上表精确 SHA256 的单行测试表达修正；最终快照的定向编译、Clippy 和增量 workspace 由主会话执行，本 reviewer 未运行 Cargo，未认定这些最终门禁已通过。当前开放 findings 仍为 0。

### Test Structure Incremental Re-review

2026-10-28T02:22:04Z 核对 `git diff 5988da8 e81127f`，仅涉及 `src/trino/l2_monitor.rs` 和 `tests/l2_monitor.rs`。对 Git 内容统一 LF、剔除模块分隔尾部空白后分别比较，迁移的 unit module 和移除它后的 production 文本均逐字一致。unit module 现位于 `src/trino/l2_monitor.rs:397-457`，旧响应隔离用例从 `:404` 开始；`run_cycle` 现从 `:249` 开始，历史修复记录中的行号属于当时快照。

`tests/l2_monitor.rs:51-76` 的 `cancel_response` 原样提取 DELETE 分支，`:81` 立即返回该 helper 结果；计数、query ID、blocked/status 判定、成功移除 registry、延迟响应的顺序与条件保持一致。`:116,120,122,146` 的 `clone_into` 向同一 `String` 写入相同状态值。没有新增 `allow`/`expect` lint suppression，没有 production 行为变化，也未发现新的正确性、安全或测试可靠性风险。

执行器报告本快照 unit 55/55、monitor 16/16，共 71 passed；完整 workspace Clippy `-D warnings` exit 0 (25.79s)，fmt/diff exit 0。本 reviewer 未运行 Cargo，仅独立核实 diff、等价性、源码 SHA256 与限定范围 `git diff --check`。最终 workspace 回归仍由主会话执行，当前为 pending，不能把 `80d053b` 的 1251 passed 归入本快照。增量签署只覆盖上述重排与 fixture 整理及精确快照 `e81127fa41099365c44c2c42750024efad6653cb`；当前开放 findings 为 0。
