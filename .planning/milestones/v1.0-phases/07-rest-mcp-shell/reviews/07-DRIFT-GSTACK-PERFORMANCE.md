---
phase: 07-rest-mcp-shell
specialist: gstack-performance
reviewed: 2026-11-03T11:43:41Z
diff_base: 4d203bfab24293de28375524c605cd41955112df
source_head: 4d203bfab24293de28375524c605cd41955112df
status: no_findings
findings: []
dynamic_performance_tested: false
tests_executed: false
---

# RF-01 Performance 专项审查

NO FINDINGS

本轮审查 RF-01 工作树相对 `4d203bf` 的相关 Rust 增量，以及新增 `crates/ontoforge-server/src/drift.rs`。采用 `C:/Users/easyi/.agents/skills/gstack/review/specialists/performance.md`，并以 ADR-0030 和 `07-DRIFT-FIX-DESIGN.md` 的已接受边界为准。未发现可独立登记的新高置信度问题；此结论不表示动态性能、全部功能或发布门已验证。

## 静态证据与结论边界

| 检查项 | 实际证据 | 置信度与边界 |
| --- | --- | --- |
| 单 task、单 slot | `crates/ontoforge-server/src/state.rs:43` 在 owner mutex 内拒绝重复安装；`drift.rs:154` 仅启动一个 task；`:167-180` 串行等待每轮；`:78-80` 仅保存一个报告 | 高：未发现并发 metadata round 或按版本增长的报告缓存 |
| 锁不跨 await | `drift.rs:95-108` 在通知前释放 slot；`:113-124` 仅同步替换状态；`:199-201` 先取 JoinHandle 再 await；`:216-225` 的观察锁在 metadata 调用前释放；`state.rs:43-76` 无 await | 高：针对本增量持有的 Mutex；未宣称已测锁竞争或尾延迟 |
| interval 与 Notify | `drift.rs:156-157` 在首轮前设置 30s interval 并使用 Skip；`:168-179` 在相同 task 消费 tick/Notify，再执行一轮；`:109` 使用 notify_one | 高：接受既定 Notify 合并语义；不把允许的唤醒后补跑或无 TTL 重新登记为缺陷 |
| 20s 整轮预算 | `drift.rs:208-211` 在 snapshot 前建立 timeout_at；`:250-252` 传剩余预算；`bind_check.rs:560-578` 每个 catalog 重算剩余时间，并在零预算时不再发 probe | 高：静态追踪证明计时范围包含 snapshot 和全部异步 metadata 等待；未测大 ontology 下同步反序列化/报告构造的不可抢占执行时间 |
| 快照与请求 gate 成本 | `context.rs:268-294` 在已有 request_snapshot 上捕获 view；`state.rs:59-70` 捕获 Arc；`drift.rs:43-63` 仅查当前报告中的逻辑 target；`health.rs:19-22` 额外读取真实提交 snapshot | 高：未发现每次业务 gate 执行 metadata 网络探测或 clone 完整报告。`request_snapshot.rs:24-44` 读取定义与 scope 并结束事务；匿名 health 与周期检查使用该契约，尚无实测数据支持进一步性能缺陷判断 |
| 上游调用与收尾 | `bind_check.rs:549-578` 按 catalog 合并表请求；`trino/mod.rs:474-514` 每 catalog 一次 describe；`lifecycle.rs:76-104` 在现有 monitor 阶段同时等待 L2/drift，随后 drain raw engine | 高：未发现本增量引入逐 target 的 metadata 请求或额外关闭时间窗口 |

已沿 Query PK、列表首批/后续、聚合、M:N 和 Action 的新增 gate 调用及 REST/MCP 接线读取增量。AB01（Action 误继承 HTTP/query 预算）由主会话已登记，本轮不重复登记；主会话确认审查结束时处于已授权的真实 suite mutation RED 窗口（临时单行 source gate bypass），本报告仅绑定原生产 `context.rs` blob `bfc178c59d5e8de31a0d7f36197bcc5b31be82ec`，不将临时变化视为 AB01 生产修复，也不替代未来修复后的复审。

静态读取了 `crates/ontoforge-engine/tests/drift_budget.rs` 和 server 的周期/整轮预算测试代码以理解已有断言，未运行 Cargo、服务、测试、基准或负载工具，未把测试源码存在当作测试通过。工作区无 `AGENTS.md`、`.codegraph/`、`.codex/skills/`、`.agents/skills/`，采用委派消息所带项目规则；agent-skills bootstrap 查询无额外 skill 输出。

## 审查时工作树 blob

以下为 `git hash-object` 的 Git blob ID，指向本轮实际读取的版本，工作树后续变化需单独复审。

```text
e7eacbb94dadf727e3d0bd1ad9e8a9c60c74b568 crates/ontoforge-server/src/drift.rs
50ff35009b09f204fcabb2b51d61f11b71c4310e crates/ontoforge-server/src/state.rs
bfc178c59d5e8de31a0d7f36197bcc5b31be82ec crates/ontoforge-server/src/context.rs
da21a2c4b3c10fdfb0c2f811c891354ad0f9e18f crates/ontoforge-server/src/health.rs
38005f3943f73da00026c8fa9ff3c4b08f12c7d8 crates/ontoforge-server/src/lib.rs
17b7f4e8f794459661e73b19f8d5833797734dc1 crates/ontoforge-server/src/lifecycle.rs
355e023f16c89a06f38382f8354c79ac18f796fd crates/ontoforge-engine/src/bind_check.rs
f3e144e92dfa347a54a3d8a8ffdf2d478d931999 crates/ontoforge-engine/src/lib.rs
94c8aa38ac79db1833ff5e9ce3f2d7ac0c79879a crates/ontoforge-query/src/lib.rs
9bf78c07f5e50cc11c77e8e07fe2c6e13ac32fb4 crates/ontoforge-query/src/execute.rs
174483df7809faeff921c09626b396a0ac489c96 crates/ontoforge-query/src/aggregate.rs
bec626bb411c9d0bb467a82d382a9b01ab8e325f crates/ontoforge-query/src/link.rs
68218b54e954014e998c17081a2bf427502c5de2 crates/ontoforge-action/src/lib.rs
33af0bc044235488f725d4835625388137583c20 crates/ontoforge-action/src/source.rs
4ba27e85a1e512c7adb69aa1d190682e36fde939 crates/ontoforge-server/src/rest.rs
aed54c866f87162a95e76c9553da9b63d1d859e2 crates/ontoforge-server/src/mcp.rs
439ce1ce1eafe6cbe638f92fb2725925c51d7616 crates/ontoforge/src/serve.rs
5c602225210c26be2299a45f59d9886d9c258aeb .planning/adr/0030-source-drift-host-health.md
6d8ca0ca2f851558f822ff303431bfcd6201d66e .planning/phases/07-rest-mcp-shell/reviews/07-DRIFT-FIX-DESIGN.md
```
