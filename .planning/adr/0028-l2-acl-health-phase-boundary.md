# ADR-0028 L2 ACL 健康状态的阶段边界

status: accepted
date: 2026-10-28

## Decision

Phase 6 交付库级 L2 ACL 启动与周期 attestation、fail-closed 状态及可供宿主读取的禁用原因。Phase 7 的服务启动必须调用并持有该生命周期，HTTP `/health` 暴露禁用原因及告警，并以端到端测试阻断验收。L2-04 的整体需求只有两阶段证据齐备后才可标记 Complete。

成功证明超过现有 60 秒期限、探针失败或身份可见性无法证明时 L2 仍禁用；不采用 last-known-good 放行。Phase 6 不把库级状态单测冒充 HTTP 告警证据。

## Probe Source

2026-10-28 用户选择选项 1：部署方提供有界只读探针 SQL，保留真实活跃 query 合同。每轮由 ontology 身份提交新 query，同周期确认活跃状态及 L2 不可见；配置缺失、过早结束、未知状态、超时或取消失败均禁用。服务端 execution/run time 上限覆盖执行及排队时间；客户端取消仅发送到验证过的 origin。CI 的 TPCH 大表仅为测试配置，不硬编码到生产。Grok 第二意见作为设计输入，真实 Trino 483 测试作为协议证据。

## References

- `.planning/adr/0018-authz-scope-and-sql-tiers.md` 的 Trino ACL 与 fail-closed 约束保持有效。
- `.planning/phases/06-l2-raw-dql-toolsets-audit/06-REVIEW.md` 的 WR-01。
