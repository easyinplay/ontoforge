# ADR-0027 聚合 connector capability 的可信 host 边界

- status: accepted
- date: 2026-10-23
- decider: Codex orchestrator（用户要求继续推进；Grok 独立第二意见与现有代码、PLAN 核对）
- 细化：ADR-0020；覆盖 `04-06-PLAN.md` 中“capability 来自 trusted binding”的来源表述
- 依据：`04-06-SUMMARY.md`、`04-SECURITY.md` T-04-28、`crates/ontoforge-query/src/{ast,aggregate,compile}.rs`

## Context

Phase 4 实施把 `AggregateCapabilities` 作为可信 Rust host 的 `execute_aggregate` 参数；`AggregateRequest` wire 无此字段，当前调用仅在库测试中。`SourceBinding` 不含 capability 元数据。原 PLAN 写 trusted binding，与已实施边界不一致。SQL Server connector 的 `sum(integer)` 不可下推，需要由 capability 表选择 fallback 或显式错误。

## Decision

可信 host 按 connector 构造并注入 `AggregateCapabilities`，wire 请求不得声明或覆盖它。`SourceBinding` 继续描述物理表与列映射，不增加第二份 capability 来源。未知 connector 或缺少可信表项时不得假定全部支持，须拒绝错误下推。当前 Phase 4 只提供库 API；host 接入属于后续阶段的强制验收，不将测试中的 `all_supported()` 视作生产 connector 探测。

接入验收：host 的静态 connector 映射不读取 wire；未知 connector fail closed；SQL Server `sum(integer)=false` 端到端证明无错误下推；`AggregateRequest` 持续拒绝 capability 字段。当前库验收用 wire 负例、现有 SQL Server false planner 反例和公开调用面审计。

## Rejected

- 立即扩展 `SourceBinding`：当前没有生产 host 消费该字段，且与 host 参数形成双事实源，需要额外优先级与冲突规则。
- 按 catalog 名称猜测 connector：名称不是可信能力声明。

## Consequences

库的公开 Rust API 将调用者视为可信 host；未来 host 接入时必须按本 ADR 的验收项证明映射来源。若 host 允许不可信代码直接传入 `AggregateCapabilities`，本决议不能关闭该新边界风险，须重新审计。
