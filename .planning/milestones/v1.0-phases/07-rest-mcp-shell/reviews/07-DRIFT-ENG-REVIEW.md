---
status: accepted_for_implementation
source_head: 972c7be9cc3e91460d300b6f45fd814df023a539
date: 2026-11-03
---

# RF01 增量工程审查

应用gstack plan-eng-review四个审查维度，沿用既有Phase7范围。用户已授权Claude Code同方向意见自动推进，覆盖skill的逐项确认流程；本轮没有需要用户裁决的实质分歧。实施真相仍由新测试与独立复签决定。

## Scope challenge 与已有实现

跨engine/query/action/server的接线超过8文件，原因是必须覆盖各个真实source出口。复用check_drift、BindingTarget、request_snapshot、ScopedEngine、ServerState、现有audit reserve与10s关闭流程；删除新EngineRegistry、第二Trino client、额外owned wrapper、source health DTO/计数字段、公开drift配置的扩展。这样保留一个monitor和一个不可变请求view，减少概念数量。

## Architecture

```text
committed request_snapshot -> fixed definition/profile -> immutable drift view
                                  |                         |
raw host engine <- one monitor <- Notify/tick                |
                    |                                      v
             versioned single slot             auth/audit -> source target hook
                    |                                      |
                    v                                      v
             anonymous health                     ScopedEngine -> raw engine
```

- exact-key拒绝旧版本报告；单任务无重叠probe，在途请求持不可变view。外部reload靠request_snapshot发现，不能依赖store.active_definition缓存。
- 只在实际source发送前检查逻辑target，同表不同target不可混用。Action在owned async块内构造ScopedEngine，完整patch不走source便不受影响。
- 生产启动seam在serve_with_limits内建立owner，首次探测先于accept；失败路径必须回收。周期任务panic/异常结束转CheckFailed，不留下永久healthy。
- 不把target数量作为readiness依据。完整报告的target degradation均200 degraded；engine/status失败或无报告503，按ADR0030。

## Code quality

- `check_source_target`默认放行让库模式继续工作；server唯一wrapper持固定view，query/action明确调用hook并保留稳定错误类别。禁止把PrintSql解析为target。
- 错误类型穷举到QueryError/AggregateError/ActionError与REST/MCP，不Display底层detail。不新增未经要求的Action read_audit体系；现有受审计Query拒绝必须保留唯一原audit ID，Action拒绝无edit/log/outbox。
- owner任务生命周期与状态读取分别承担调度和请求快照，锁内禁止storage/network await。公开接口只加入必要内部hook，不建注册服务。

## Tests 与失败模式

| 路径 | 实际失败 | 必需测试/结果 |
|---|---|---|
| 首次/周期probe | 表列漂移、metadata拒绝 | 真实listener隔离目标、周期恢复；真实Trino ALTER恢复 |
| snapshot/reload | 旧probe延迟返回 | barrier双store版本隔离、旧view固定，新key Pending |
| PK/list/backfill/aggregate/link | 漏接source hook | 覆盖全部出口、拒绝时零业务SQL |
| Action source/validate-only | 裸engine绕过或压扁错误 | REST/MCP稳定错误、零业务写；完整patch成功 |
| health | SELECT1抹掉probe失败或泄露对象 | 200/503矩阵、匿名无名称/计数、故障恢复 |
| monitor | overlap、失败/异常退出、通知风暴 | 单task、Skip tick、Notify合并、异常转CheckFailed |
| startup/shutdown | 初始snapshot失败、在途probe不释放 | 清理owner/cancellation；原10s总预算内结束，DELETE失败非零 |

既有TDD与真实服务矩阵见07-DRIFT-FIX-DESIGN.md。行为RED必须可归因，不以编译错误或零测试代替。测试fixture需准确describe元数据，不通过默认关闭monitor维持旧绿。

## Performance

单周期整轮共享20s，不给每catalog各20s；30s interval使用Skip，避免完成后再sleep累加。Notify单permit合并Pending唤醒；请求只短锁捕获Arc，不每请求重新发metadata探测。报告单slot及在途Arc，不保留无界历史版本缓存。无新增drift TTL。

## NOT in scope

自动DDL/reset/绑定纠偏、pk查重、第二engine client、按数量改变health、额外匿名source字段、额外public配置均不需要。三平台发行继续由07-14负责，当前依赖07-13验收。P01 history lookahead性能观察留原review，不扩storage接口；无实测回归。

## Implementation Tasks

- [ ] T1：一个executor完成RF01 core链（engine hook/check_drift泛型、query所有source出口、Action错误映射），先行为RED；文件范围见设计第5节。
- [ ] T2：同executor完成monitor/ScopedEngine/host启动关闭/health/REST-MCP接线及fixture适配；依赖RF05和OpenAPI修复冻结、Cargo槽交接。
- [ ] T3：定向测试/lint与真实服务验收，独立复签；全部满足后才能关闭RF01与07-13。

顺序实施；共享Cargo与rest/mcp源码不允许并发owner修改。当前worker可先读/写独立测试草案，生产修改和Cargo等前一owner冻结。

## GSTACK REVIEW REPORT

| Runs | Status | Findings |
|---|---|---|
| 主会话Architecture/Quality/Tests/Performance | accepted | 缩减wire/client/config扩展，补monitor异常与全部target规则 |
| Claude Code源码独立审查 | AGREE_WITH_REFINEMENT | target隔离、嵌套params、固定版本及既有关闭预算同方向 |
| Claude Code补充收敛 | AGREE | 全部target仍200 degraded、无新增匿名字段、借用wrapper、task异常处理 |

VERDICT: 可实施，未代表动态验证通过。CROSS-MODEL: Claude Code实际配置MiniMax-M3.1-Flash[1m]；原始日志target/phase7-claude-contract-review.json和phase7-claude-followup.json。UNRESOLVED: 无产品方向分歧；实现中发现新灰区先返回主会话获取第二意见。
