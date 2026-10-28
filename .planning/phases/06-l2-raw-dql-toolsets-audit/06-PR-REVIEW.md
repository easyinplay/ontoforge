---
phase: 06-l2-raw-dql-toolsets-audit
reviewed: 2026-10-28T11:49:43Z
depth: deep
diff_base: bb9aeb62e537f39332604083d589df1764ca1f89
broad_review_head: 20fd0005bc93707d2363231f36b4d5ec43da06fb
fix_head: 7813e7fb591df1bd2eb607b1a31511492a62c9e3
snapshot_head: 2c9eee6461a412ead8f998c7fa038acaa458b2f9
files_reviewed: 45
files_reviewed_list:
  - .github/workflows/ci.yml
  - ci/query-services.sh
  - ci/trino/l2/access-control.properties
  - ci/trino/l2/config.properties
  - ci/trino/l2/rules.json
  - ci/trino/l2/seed.sql
  - crates/ontoforge-action/tests/entrypoints.rs
  - crates/ontoforge-engine/Cargo.toml
  - crates/ontoforge-engine/src/config.rs
  - crates/ontoforge-engine/src/printer/l2.rs
  - crates/ontoforge-engine/src/printer/mod.rs
  - crates/ontoforge-engine/src/trino/decode.rs
  - crates/ontoforge-engine/src/trino/l2_health.rs
  - crates/ontoforge-engine/src/trino/l2_monitor.rs
  - crates/ontoforge-engine/src/trino/mod.rs
  - crates/ontoforge-engine/tests/data/l2_corpus.json
  - crates/ontoforge-engine/tests/l2_gate.rs
  - crates/ontoforge-engine/tests/l2_health.rs
  - crates/ontoforge-engine/tests/l2_monitor.rs
  - crates/ontoforge-engine/tests/trino_l2_acl_it.rs
  - crates/ontoforge-query/Cargo.toml
  - crates/ontoforge-query/src/aggregate.rs
  - crates/ontoforge-query/src/execute.rs
  - crates/ontoforge-query/src/l2.rs
  - crates/ontoforge-query/src/lib.rs
  - crates/ontoforge-query/tests/audit_l0.rs
  - crates/ontoforge-query/tests/audit_pk.rs
  - crates/ontoforge-query/tests/definition_snapshot.rs
  - crates/ontoforge-query/tests/l2_discovery.rs
  - crates/ontoforge-query/tests/l2_execute.rs
  - crates/ontoforge-query/tests/security.rs
  - crates/ontoforge-storage/migrations/postgres/0005_read_audit.sql
  - crates/ontoforge-storage/migrations/postgres/0006_audit_truncated.sql
  - crates/ontoforge-storage/migrations/sqlite/0005_read_audit.sql
  - crates/ontoforge-storage/migrations/sqlite/0006_audit_truncated.sql
  - crates/ontoforge-storage/src/lib.rs
  - crates/ontoforge-storage/src/read_audit.rs
  - crates/ontoforge-storage/src/sql_exit.rs
  - crates/ontoforge-storage/src/store.rs
  - crates/ontoforge-storage/tests/conformance/mod.rs
  - crates/ontoforge-storage/tests/conformance_pg.rs
  - crates/ontoforge-storage/tests/reload_pg.rs
  - crates/ontoforge-storage/tests/scope_store.rs
  - crates/ontoforge-storage/tests/support/mod.rs
  - crates/ontoforge/tests/cli_reload.rs
findings:
  critical: 0
  warning: 0
  info: 0
  total: 0
resolved_findings: 1
status: clean
blocking: false
local_gate: pass
external_codex: unavailable
remote_ci: in_progress
workflow_status: COMPLETE_LOCAL_GATE
quality_score: 10.0
---

# Phase 06: PR Review

## Summary

正式执行 `gstack/review/SKILL.md` 的本地适用步骤。宽范围审查固定 `20fd0005`，随后增量审查 RED `0bb29a3` 与修复 `7813e7f`。交付 `2c9eee6` 相对 `7813e7f` 无 source/CI 差异，关联同一实现结论。

原 CR-01 已关闭，当前无未解决 BLOCKER 或 WARNING。本地 gate 通过；外部 Codex CLI unavailable，不能声明跨模型 clean。新 CI run `40682870456` 截至复核时仍 `in_progress`，本报告不是远端 CI 通过或合并授权。

## Narrative Findings (AI reviewer)

当前实现快照无未解决 finding。以下保留首次发现与关闭证据，避免把已修复缺陷从历史中删除。

### CR-01: [BLOCKER, RESOLVED] Audit shutdown 的取消重试或并发调用会提前报告持久化成功

**File:** `D:/DataSpellProjects/ontoforge/crates/ontoforge-storage/src/read_audit.rs:230`，原始快照 `20fd0005`。

**分类:** FIXABLE。**置信度:** 9/10。主审查者与独立 adversarial subagent 分别核对当前源码后确认。

**Issue:** `shutdown()` 在等待 worker 完成之前取走唯一 `JoinHandle`，随后任何调用都将 `None` 当作成功完成。`None` 实际还可能表示另一个调用正在等待，或该等待已被取消。首次 worker 返回错误之后，该错误也不会被保存。此行为违反第 226 行的 durable drain 合同，调用者据此关闭 runtime 时存在已受理审计记录丢失风险。

```rust
let Some(handle) = self.worker.lock().await.take() else {
    return Ok(());
};
handle.await.map_err(|_| AuditUnavailable)?
```

**确定性触发路径:**

1. `reserve()` 一个 guard 并保持其存活。guard 持有 sender，因此 worker 不能结束接收循环。
2. 开始首次 `shutdown()`，poll 到它正在等待 worker。该调用已取走 `JoinHandle`。
3. 用 `tokio::time::timeout` 取消首次调用。handle 被 drop，worker 继续运行，共享 worker 字段保持 `None`。
4. 再次调用 `shutdown()`，立即得到 `Ok(())`，但第 1 步的 guard 仍未 finish，记录尚未持久化。

无需取消也存在并发路径：保持 guard，首次 shutdown 等待期间第二次 shutdown 提前返回成功。另一路径是触发现有审计 sink fault，使首次 shutdown 返回 `Err`；第二次调用仍返回 `Ok(())`，掩盖已发生的失败。

**相关证据:** `read_audit.rs:239` 的 guard 持有 sender；`run_worker` 只有在 receiver 返回 `None` 后才执行最终 flush 并退出。`crates/ontoforge-storage/tests/conformance/mod.rs:2630` 只覆盖单次 shutdown 与 in-flight guard；`:2756` 和 `:2779` 只检查首次失败 drain，没有验证重复调用。

**Fix:** 用共享状态区分 Running 和 Completed。将 `JoinHandle` 保存在受 mutex 保护的 Running 状态中，借用 `&mut JoinHandle` 等待，确保取消等待时 handle 仍归共享状态所有；worker 完成后缓存 `Result<(), AuditUnavailable>`，后续调用返回相同结果。并发调用应等待同一个完成结果。单纯检查 `health().available` 无法区分成功 shutdown 与 worker failure。

**行为验收:**

- 持有 guard，首次 shutdown timeout 后重试；重试必须在 guard finish 和入库之前保持 pending，完成后实际查询审计行。
- 两个并发 shutdown 都必须等待 guard finish 和持久化，不能有一个提前成功。
- sink failure 导致 drain 失败后，重复 shutdown 必须继续返回错误。
- 完成一次成功 drain 后，重复 shutdown 应稳定成功。
- 将以上用例加入共享 SQLite/PostgreSQL conformance，保留现有单次 in-flight drain 用例。

**关闭复核:** RED `0bb29a3` 已把上述三条行为失败及 JoinError 失败固定为可编译、命中计划断言的回归；主审读取了提交内 JUnit/classifier 与原始日志。GREEN `7813e7f` 将 worker 状态改为 `Running(JoinHandle)` / `Finished(Result)`：`:235` 持有 mutex，`:239` 借用 `&mut JoinHandle` 等待，`:242` 在无后续 await 的同一 poll 中缓存最终结果。取消等待不再转移 handle 所有权，并发 caller 等待同一终态，worker 错误和 JoinError 都被保留。

主审与最终独立 Red Team 均重读修复及新测试，确认测试先显式 poll 到 Pending，再制造并发或取消；guard finish 后还检查实际持久化 principal，重复成功与重复失败均有断言。本机锁定依赖 Tokio `1.53.3` 的 `src/runtime/task/join.rs:29-36` 明确说明 `&mut JoinHandle` 的 cancel safety，与此实现一致。

主审未自行运行 Cargo。已直接读取修复提交内原始 GREEN 日志：`06-11-GREEN-sqlite.log` 为 8/8，`06-11-GREEN-pg.log` 为真实 PostgreSQL 9/9，`06-11-GREEN-join-error.log` 为 1/1；另读 storage Clippy/fmt/diff-check 成功日志，且主审独立执行固定修复 diff 的 `git diff --check`，退出码 0。CR-01 在 `7813e7f` 关闭。

## Review Evidence

- PR #7 首次独立核对为 `OPEN`、`isDraft: true`，base 为 `gsd/phase-05-online-action`，当时 head 为 `20fd0005`。未发送 GitHub 消息，未合并。
- 已 fetch base 并重新确认交付快照的 merge-base 为 `bb9aeb62e537f39332604083d589df1764ca1f89`。实现审查范围合计 45 个 source/CI 文件，清单见 frontmatter。宽范围由主审和六位独立 specialist 合并覆盖，重要边界源码读全文，测试按相关场景与断言追踪；不声明对所有既有测试无差别逐行再审。
- 旧 GitHub Actions run `40675450974` 的 `headSha` 为 `20fd0005`。`integration`、`test (ubuntu-latest)`、`test (windows-latest)` 均 `success`；`isolated-query-bench` 为 `skipped`。通过 `gh run view ... --json ...` 独立核对，CI 于 `2026-10-28T11:13:35Z` 完成。该结果不作为 `7813e7f` 修复后的完整 CI 证据。
- integration 的 PostgreSQL read audit conformance、独立 L2 ACL Trino 483、L2 corpus 等步骤成功。Linux/Windows 的 workspace test、fmt、Clippy 和 corpus 步骤成功。CI 成功不覆盖 CR-01 的取消重试/并发条件。
- 最近测试改动为 PostgreSQL system table 断言加入 `of_read_audit`，以及两次大行读取之间等待前一审计排空；未确认这些修改引入新的正确性缺陷。
- Greptile 两个评论 API 查询均成功，返回 0 条 Greptile 评论；没有向 GitHub 发送消息。
- 新 CI run `40682870456` 的 `headSha` 已独立核对为 `2c9eee6461a412ead8f998c7fa038acaa458b2f9`，创建时间 `2026-10-28T11:49:03Z`，报告复核时 `status: in_progress`、尚无 conclusion。

## Scope And Plan Audit

Scope Check: CLEAN。实现仍限于 Phase 6 的 library L2 raw DQL、toolsets、read audit、ACL attestation 和对应 CI。ADR-0028 明确保留 Phase 7 的服务启动接线、HTTP `/health` 与 REST/MCP，因此未把这些 deferred 项当作范围缺失。

意图来源为固定快照 `task_plan.md`、`06-CONTEXT.md`、`06-10-PLAN.md`、PR body 与 base 到实现的 commit messages。P0/P1/P2/P3 的分支、合同和规划文件均存在；本次审查不重新批准历史规划。P4/P4b 的交付路径在 engine/query/storage 和测试差异中可追踪。P4 的 shutdown 合同在首次审查为 PARTIAL，现已由 06-11 的 RED/GREEN 和增量复核关闭。P5 的本地 review 已完成，匹配新交付 SHA 的完整远端 CI 仍未完成。

没有 `TODOS.md` 或 `package.json`，没有可用 `slop:diff`，相应可选扫描不适用。root `CONTRIBUTING.md` 的现有 fmt/Clippy/test 和集成测试描述未发现与该 source 差异直接冲突的事实；未修改文档。已读取项目 learnings 并用于检查 table-function 下推绕过、sqlparser Display 与 Trino 实测之间的边界。

## Independent Specialists

`gstack-specialist-stats` 返回 0 reviews analyzed，没有使用 adaptive gating；`gstack-review-read` 返回 NO_REVIEWS，没有抑制历史 skipped finding。

| Specialist | Scope | 结果 |
| --- | --- | --- |
| Testing | audit_l0/audit_pk、双后端 conformance、L2 execute/discovery/health/monitor、104-ID/真实 Trino 与 CI 断言 | 无新增 finding；原 CR-01 known/open，由 06-11 单独修复 |
| Maintainability | L0 audit/WorkBudget 消费者、L2 validator/discovery/health/monitor、worker 状态与错误传播 | 无新增 finding |
| Security | AST/Display、allow-list、身份隔离、origin/redirect、metadata/hash/bind、CI 凭据和 ACL fixture | 无新增 finding |
| Performance | 结果上限、Collector、queue/backpressure、monitor deadline、DB 调用边界 | 未证明违反明确正确性或预算合同的新问题；纯性能按 v1 scope 排除 |
| Data Migration | 四条迁移、字段绑定、旧行升级、sys search_path、原子回滚与双后端约束 | 无新增 finding |
| API Contract | library public API、错误码/转换、发现结果/截断及变更外消费者 | 无新增 finding；HTTP/MCP 是 deferred scope |
| Design | 无 frontend 差异 | scope skipped |
| Red Team | 在六位 specialist 结果之后检查交界失败、audit/ACL 生命周期及 7813e7f 修复 | 无新增 finding；独立确认 CR-01 关闭 |

每位适用 specialist 均以独立 subagent 执行，因平台 4 slots 分批运行，全部使用固定提交和只读工具，未运行 Cargo 或数据库。最初独立 adversarial subagent 找出的 CR-01 已完成主审重读核验；最终 Red Team 还独立挑战了修复和测试。合并后当前 findings 为 0，quality score 按技能公式为 10.0；这不是不存在未知缺陷的证明。

## gstack Execution And Limits

- 已执行适用的本地 Step 0/1 base 检查、Step 1.5 scope/plan 对账、checklist 双 pass、Greptile triage、消费者与 enum/error completeness 检查、六位 specialist、最终 Red Team、独立 adversarial、修复处置与增量复审。源码审查同时覆盖 SQL/data safety、并发、LLM trust boundary、shell injection、字段/类型/时间边界、CI 分发和文档一致性。
- 外部 Codex CLI adversarial attempt 使用 `--ephemeral --ignore-user-config --ignore-rules -s read-only`，退出码 1，返回 `401 Unauthorized` 及 `Missing bearer or basic authentication in header`。该隔离配置的 attempt 未取得任何模型审查结果，不能推断日常 Codex 配置也失效。此前 overload attempt 也不算通过。
- 外部 Codex adversarial/structured lane 按技能 Error handling 记录为非阻塞 unavailable，不再重试；本地适用 Review Army 已完成，未形成外部或跨模型通过结论。
- 按 orchestrator 后续授权，使用规范 `gstack-review-log` 持久化本地 review 与 adversarial 结果；记录外部 unavailable，不能把鉴权失败记为 pass。`telemetry` 与 `artifacts_sync_mode` 均为 off。未改 source 或共享 STATE，未提交或合并，未重跑 Cargo workspace；测试 evidence 来源已分别标明。
- Phase 7 的 service lifecycle wiring、HTTP `/health`、REST/MCP 与工具过滤按 ADR-0028 单独验收。CR-01 属现有库级 `ReadAudit::shutdown()` 合同，不能推迟为 Phase 7 wiring。

**Recommendation:** 本地 code review gate 已通过。继续等待 `2c9eee6` 的完整 CI 与正式 verifier 增量签署；当前报告不授权合并，也不关闭 Phase 7 deferred 合同。
