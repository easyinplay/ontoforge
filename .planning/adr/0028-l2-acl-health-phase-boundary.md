# ADR-0028 L2 ACL 健康状态的阶段边界

status: accepted
date: 2026-10-28

## Decision

Phase 6 交付库级 L2 ACL 启动与周期 attestation、fail-closed 状态及可供宿主读取的禁用原因。Phase 7 的服务启动必须调用并持有该生命周期，HTTP `/health` 暴露禁用原因及告警，并以端到端测试阻断验收。L2-04 的整体需求只有两阶段证据齐备后才可标记 Complete。

成功证明超过现有 60 秒期限、探针失败或身份可见性无法证明时 L2 仍禁用；不采用 last-known-good 放行。Phase 6 不把库级状态单测冒充 HTTP 告警证据。

## Open implementation decision

每次自检须证明 ontology 身份的已知 query 在观测期间活跃且 L2 身份不可见。CI 的 TPCH 大表不是生产通用探针。部署方提供有界探针 SQL 与库级生命周期，或改变活跃性合同，需在实施前分别验证并明确选择；任何证据不足保持禁用。Grok 第二意见倾向前者，建议服务端运行上限、同周期验证、取消和故障分类。

## References

- `.planning/adr/0018-authz-scope-and-sql-tiers.md` 的 Trino ACL 与 fail-closed 约束保持有效。
- `.planning/phases/06-l2-raw-dql-toolsets-audit/06-REVIEW.md` 的 WR-01。
