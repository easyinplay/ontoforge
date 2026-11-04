---
phase: 07-rest-mcp-shell
reviewed: 2026-11-03T12:05:38Z
depth: deep
specialist: gstack-red-team
diff_base: 4d203bfab24293de28375524c605cd41955112df
status: clean
scope_status: final_rf01_increment_only
tests_executed: false
files_reviewed: 27
files_reviewed_list:
  - crates/ontoforge-action/src/lib.rs
  - crates/ontoforge-action/src/source.rs
  - crates/ontoforge-engine/Cargo.toml
  - crates/ontoforge-engine/src/bind_check.rs
  - crates/ontoforge-engine/src/lib.rs
  - crates/ontoforge-query/src/aggregate.rs
  - crates/ontoforge-query/src/execute.rs
  - crates/ontoforge-query/src/lib.rs
  - crates/ontoforge-query/src/link.rs
  - crates/ontoforge-server/Cargo.toml
  - crates/ontoforge-server/src/context.rs
  - crates/ontoforge-server/src/drift.rs
  - crates/ontoforge-server/src/error.rs
  - crates/ontoforge-server/src/health.rs
  - crates/ontoforge-server/src/lib.rs
  - crates/ontoforge-server/src/lifecycle.rs
  - crates/ontoforge-server/src/mcp.rs
  - crates/ontoforge-server/src/rest.rs
  - crates/ontoforge-server/src/state.rs
  - crates/ontoforge/src/serve.rs
  - crates/ontoforge-server/tests/action_source_budget.rs
  - crates/ontoforge-server/tests/source_drift_cleanup.rs
  - crates/ontoforge-engine/tests/drift_budget.rs
  - crates/ontoforge-server/tests/unit/drift.test.rs
  - crates/ontoforge-server/tests/health_trino_it.rs
  - crates/ontoforge/tests/cli_shutdown.rs
  - ci/server-services.sh
findings:
  critical: 0
  warning: 0
  info: 0
  total: 0
---

# RF01 gstack Red Team 最终增量复核

## Narrative Findings (AI reviewer)

NO FINDINGS

未发现新增、可达且可证实的问题。本结论仅覆盖下列工作树快照的 RF01 增量，尤其是 AB01 最终修复；不表示整个仓库无缺陷、动态验证完成或发布门通过。没有重新登记 DS01、CR01，也没有修改 ADR-0030/0031 已接受的合同。

## 审查范围与方法

遵循 `C:/Users/easyi/.agents/skills/gstack/review/specialists/red-team.md`。阅读相对 `4d203bf` 的相关完整 tracked diff、新 `drift.rs`，并沿调用链核对实际 source 出口、Action 生命周期、请求固定快照、错误传播、monitor 发布和关闭。已读 Security、Maintainability、Performance、API specialist 的意见作为线索，结论来自当前源码。

工作区没有 `AGENTS.md`、`.codegraph/`、`.codex/skills/`、`.agents/skills/`，采用委派上下文所附规则；GSD agent-skills 查询未返回额外 skill。遵守 `.gitignore`，未发现 `.codexignore`。仅按主会话指令读取指定 `target/` 日志作为证据，不将其纳入 source 范围。

## 重点调用链与证据

| 检查面 | 当前源码证据 | 结论边界 |
| --- | --- | --- |
| Action 领域预算 | `context.rs:214-216` 直接调用 `inner.execute(sql, ctx)`；Query 分支从 `:217` 收紧预算 | Action 保留原 timeout/rows/bytes；共用 gate 位于 `:199-205`，不受 purpose 分支绕过 |
| REST/MCP 用途与所有权 | `rest.rs:448/472`、`mcp.rs:617` 为 Action；其余五处 wrapper 为 Query | 正式 Action 的 captured drift view 随 owned task 保存；REST validateOnly 同样走 Action purpose |
| source gate 覆盖 | `query/src/lib.rs:520`、`execute.rs:747/900`、`aggregate.rs:787`、`link.rs:304` | PK、首批/后续列表、backfill、aggregate、M:N 均在相应 execute 前检查逻辑 target |
| 授权与错误传播 | `action/src/source.rs:56-65/120-126`；`server/src/error.rs:30/44/66/116` | type grant 在 source 读取前检查；source 错误保留固定类别，不把底层 metadata detail 放入响应 |
| 固定请求视图 | `context.rs:303`；`drift.rs:91-117` | 请求用同一 active definition 捕获 Arc；报告 key 精确匹配 version/profile，发布要求 observed key 相同 |
| 失败与关闭 | `drift.rs:128-180/197-203/207-271`；`lifecycle.rs:76-130` | panic/失败进入不可用状态；停止任务后走 raw engine cancellation drain；复用关闭总预算 |
| 整轮 timeout | `drift.rs:208-211/250-252`；`bind_check.rs:560-578` | snapshot 与 metadata 处于外层整轮 deadline，catalog probe 获得剩余 timeout；无剩余预算不发送新 probe |

以上是静态追踪结果，不是运行时安全保证。只保存一个报告、旧请求不可变 view、无 TTL 等均属于已接受合同，不作为新 finding。

## 测试证据与限制

完整阅读 `action_source_budget.rs`：四种 REST/MCP deadline/断连场景先等待 source barrier，再结束响应等待；完成后读取原 command receipt 并 replay，另开数据库连接核对 receipt/edit/log/outbox；第五例检查两种 validateOnly 与 Query 的预算对照。已亲读主会话指定日志：

- `target/phase7-action-budget-red.log`：0 passed / 5 failed，正式 Action receipt 为 404，validateOnly 失败。
- `target/phase7-action-budget-green.log`：5 passed / 0 failed / 0 ignored。

这些日志由执行 lane 运行，本 reviewer 未运行 Cargo、服务或测试，不声称自行完成 RED/GREEN。

补充静态读取 `source_drift.rs:1004-1157` 的 FT01：旧 v1 列表 barrier、新 v2 degraded 报告、新请求 503、旧请求第二次 gate/SQL、两版本 durable audit 均有断言。其读取快照 blob 为 `45083f2ee5caeb8bbeaac7bc0f2eddb6acc3521b`，只审查该新增场景及 fixture，不宣称全文件重审。完整读取 DC01 `source_drift_cleanup.rs`：真实 TrinoEngine 对受控 HTTP upstream，阻塞 metadata GET/DELETE，并分别检查 shutdown 等待与 DELETE 失败结果。本报告不把 FT01/DC01 的测试源码存在当作动态通过；最终结果由主会话归集。

## 审查快照

以下为工作树 `git hash-object` 值，不表示已提交，也不保证 Git object database 中已有该对象。末次读取包含 `EnginePurpose` 的新增说明注释。后续生产内容变化须重新评估。

| 文件 | Blob |
| --- | --- |
| `crates/ontoforge-action/src/lib.rs` | `68218b54e954014e998c17081a2bf427502c5de2` |
| `crates/ontoforge-action/src/source.rs` | `33af0bc044235488f725d4835625388137583c20` |
| `crates/ontoforge-engine/Cargo.toml` | `47f061e641f35e54cea5ee1af3695b939379c085` |
| `crates/ontoforge-engine/src/bind_check.rs` | `355e023f16c89a06f38382f8354c79ac18f796fd` |
| `crates/ontoforge-engine/src/lib.rs` | `f3e144e92dfa347a54a3d8a8ffdf2d478d931999` |
| `crates/ontoforge-query/src/aggregate.rs` | `174483df7809faeff921c09626b396a0ac489c96` |
| `crates/ontoforge-query/src/execute.rs` | `9bf78c07f5e50cc11c77e8e07fe2c6e13ac32fb4` |
| `crates/ontoforge-query/src/lib.rs` | `94c8aa38ac79db1833ff5e9ce3f2d7ac0c79879a` |
| `crates/ontoforge-query/src/link.rs` | `bec626bb411c9d0bb467a82d382a9b01ab8e325f` |
| `crates/ontoforge-server/Cargo.toml` | `97c68cf3cc7350adabe76d33f1df09acd85abc39` |
| `crates/ontoforge-server/src/context.rs` | `5f55345d24301b8b43d8ff669d51e32ae556f3ec` |
| `crates/ontoforge-server/src/drift.rs` | `e7eacbb94dadf727e3d0bd1ad9e8a9c60c74b568` |
| `crates/ontoforge-server/src/error.rs` | `dd1572eed41f6e96f76f796e990323debbff6e62` |
| `crates/ontoforge-server/src/health.rs` | `da21a2c4b3c10fdfb0c2f811c891354ad0f9e18f` |
| `crates/ontoforge-server/src/lib.rs` | `38005f3943f73da00026c8fa9ff3c4b08f12c7d8` |
| `crates/ontoforge-server/src/lifecycle.rs` | `17b7f4e8f794459661e73b19f8d5833797734dc1` |
| `crates/ontoforge-server/src/mcp.rs` | `c0cd841d97aeb63f6805bfb66de15ff1e49e8465` |
| `crates/ontoforge-server/src/rest.rs` | `e39b93f6131187488233bd728adc2118b4b6420c` |
| `crates/ontoforge-server/src/state.rs` | `50ff35009b09f204fcabb2b51d61f11b71c4310e` |
| `crates/ontoforge/src/serve.rs` | `439ce1ce1eafe6cbe638f92fb2725925c51d7616` |
| `crates/ontoforge-server/tests/action_source_budget.rs` | `5353696a5da48ed311c36349c99d33fd9e329d66` |
| `crates/ontoforge-server/tests/source_drift_cleanup.rs` | `0dcf954763cbec905d795714e7cc8aa716dbf30b` |
| `crates/ontoforge-engine/tests/drift_budget.rs` | `f347b661d4df651a5e1e8b488b394498c2f8fee1` |
| `crates/ontoforge-server/tests/unit/drift.test.rs` | `ae1e75f1a0690fcd3513d5e71c5674da8a295a53` |
| `crates/ontoforge-server/tests/health_trino_it.rs` | `1ab45dc6bde84fd9714ec62de3c99fa28ee1c901` |
| `crates/ontoforge/tests/cli_shutdown.rs` | `7c572132c55ca48e9a43f858f96ab2d9ba4dced2` |
| `ci/server-services.sh` | `02b8ce31973e7a9974eeab49887998700c9a9ac7` |

仅创建本报告，未修改 source、未 commit。
