---
phase: 07-rest-mcp-shell
reviewed: 2026-11-03T11:49:53Z
specialist: api-contract
scope: RF-01 working-tree increment from 4d203bf
base: 4d203bfab24293de28375524c605cd41955112df
source_head: 4d203bfab24293de28375524c605cd41955112df
status: no_new_findings
findings:
  critical: 0
  warning: 0
  total: 0
dynamic_tests_executed: false
---

# RF-01 API Contract 专项审查

NO FINDINGS

本次仅审查 `4d203bf` 至当前 working tree 的 RF-01 crates 增量及新增 `crates/ontoforge-server/src/drift.rs`。结论表示未找到新增且可证实的 API 合同缺失，不代表动态验收通过，也不重复审查此前已关闭的 50 文件增量。

## 合同依据与证据

以已接受的 ADR-0030、ADR-0031 及 `07-DRIFT-FIX-DESIGN.md` 为准；使用 `C:/Users/easyi/.agents/skills/gstack/review/specialists/api-contract.md` 检查 API 合同。未运行 Cargo、listener 或真实服务验收。

| 检查面 | 当前源码证据 | 静态结论置信度 |
|---|---|---|
| REST source 错误 | `crates/ontoforge-engine/src/lib.rs:106`、`crates/ontoforge-server/src/error.rs:30`、`:44`、`:66`、`:116` | 高：四种 `SourceTargetError` 保留独立 code，Query/Aggregate/Action 均映射 503 |
| MCP source 错误 | `crates/ontoforge-server/src/mcp.rs:221`、`:304`、`:547`、`:650`；`crates/ontoforge-server/src/error.rs:8`、`:84` | 高：沿用 JSON-RPC error，data 含 code/apiName/request_id；本增量不改变 HTTP transport |
| 实际 source 出口 | `crates/ontoforge-query/src/lib.rs:520`；`execute.rs:747`、`:900`；`aggregate.rs:787`；`link.rs:304`；`crates/ontoforge-action/src/source.rs:120` | 高：逻辑 target gate 位于 source execute 之前；Action 保留 SourceTarget 错误类别 |
| history 与间接读取 | `crates/ontoforge-server/src/rest.rs:586`、`:601`、`:623`；`crates/ontoforge-server/src/mcp.rs:422`；`crates/ontoforge-query/src/execute.rs:670` | 高：history 的现状读取与集合/backfill 复用带 gate 的 Query 路径 |
| 无需 source 的路径 | `crates/ontoforge-query/src/lib.rs:343`、`:350`；`crates/ontoforge-action/src/source.rs:102` | 高：PK 完整 patch 与 Action 无需 source 的条件保留；未增加 type 级别无条件拒绝 |
| health wire | `crates/ontoforge-server/src/health.rs:15`、`:23`、`:44`、`:48`；`crates/ontoforge-server/src/drift.rs:69` | 高：完整 target 降级报告为 200/degraded，Pending/Failed 为 503，engineStatus 仍取 raw engine；无 target/物理名泄漏 |
| 固定版本与 view | `crates/ontoforge-server/src/context.rs:195`、`:292`；`crates/ontoforge-server/src/drift.rs:95`、`:119` | 高：恢复后的 wrapper 调用 drift gate，请求 view 由固定 active definition 捕获，报告按 version/profile 精确匹配 |

## 排除与限制

- AB01（Action 继承 HTTP/query bounds）由主会话登记且已有独立测试任务，本文件不重复登记；本结论不宣称 AB01 已修复。
- 审查中见到的 `context.rs` 单行 bypass 为已声明的 RED mutation；最终重新读取当前文件，确认第 195 行恢复 `self.drift.check(target)?;`，hash 与主会话提供的原始 hash 一致。
- 下列 hash 为当前内容的 Git blob hash（`git hash-object`），不表示对象已写入 Git object database，不能据此假定 `git show <hash>` 可用。
- 动态测试未代跑。错误 transport、零上游发送、审计关联及并发隔离的动态验收仍须由各测试 lane 提供。

## 审查快照

| 文件 | Blob hash |
|---|---|
| `crates/ontoforge-server/src/context.rs` | `bfc178c59d5e8de31a0d7f36197bcc5b31be82ec` |
| `crates/ontoforge-server/src/drift.rs` | `e7eacbb94dadf727e3d0bd1ad9e8a9c60c74b568` |
| `crates/ontoforge-server/src/error.rs` | `dd1572eed41f6e96f76f796e990323debbff6e62` |
| `crates/ontoforge-server/src/health.rs` | `da21a2c4b3c10fdfb0c2f811c891354ad0f9e18f` |
| `crates/ontoforge-server/src/mcp.rs` | `aed54c866f87162a95e76c9553da9b63d1d859e2` |
| `crates/ontoforge-server/src/rest.rs` | `4ba27e85a1e512c7adb69aa1d190682e36fde939` |
| `crates/ontoforge-server/src/state.rs` | `50ff35009b09f204fcabb2b51d61f11b71c4310e` |
| `crates/ontoforge-server/src/lib.rs` | `38005f3943f73da00026c8fa9ff3c4b08f12c7d8` |
| `crates/ontoforge-engine/src/lib.rs` | `f3e144e92dfa347a54a3d8a8ffdf2d478d931999` |
| `crates/ontoforge-engine/src/bind_check.rs` | `355e023f16c89a06f38382f8354c79ac18f796fd` |
| `crates/ontoforge-query/src/lib.rs` | `94c8aa38ac79db1833ff5e9ce3f2d7ac0c79879a` |
| `crates/ontoforge-query/src/execute.rs` | `9bf78c07f5e50cc11c77e8e07fe2c6e13ac32fb4` |
| `crates/ontoforge-query/src/aggregate.rs` | `174483df7809faeff921c09626b396a0ac489c96` |
| `crates/ontoforge-query/src/link.rs` | `bec626bb411c9d0bb467a82d382a9b01ab8e325f` |
| `crates/ontoforge-action/src/source.rs` | `33af0bc044235488f725d4835625388137583c20` |
