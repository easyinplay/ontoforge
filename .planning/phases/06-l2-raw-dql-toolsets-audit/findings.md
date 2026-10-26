# Phase 6 findings

- 2026-10-26：ROADMAP Phase 6 范围为 L2-01..04、AUDIT-01 和五条 Success Criteria；spike 2 要在 Trino 483 上复验。ADR-0018/0019 与 `docs/designs/trino-engine-sql-tiers.md` 已锁定双 Trino 身份、SQL AST 白名单、只读权限自检、全级别读审计、管理通道边界。规划前需核对旧语料是否与当前 `sqlparser` 及 Trino 483 一致。
- 2026-10-26：GSD `init.phase-op 6` 的 expected_phase_dir 为 `.planning/phases/06-l2-raw-dql-toolsets-audit/`；ROADMAP 的 `**Details**` 原指向较短的 `06-l2-toolsets-audit/`，现已修正。
- 2026-10-26：代码调查：auth 已有 `sql:raw`/L2 allow-list，engine 已能建独立 L2 Trino 身份，但 `PrintedSql` 私有构造阻止原始 Display SQL 直接执行；L2 校验器、启动自检、读审计表/队列和 `AuditUnavailable` 均尚无。QueryService 的 pk/list/aggregate 是公开读入口，server/api 仍为占位。现有 `reload` 先 lint，`scope apply` 锁内复算计划哈希并写 `of_admin_audit`，可复用，不能把 CLI 环境用户名当远程认证主体。
- 2026-10-26：Trino 483 容器 `ontoforge-trino-05-06` healthy，`127.0.0.1:58080/v1/info` 报 483，临时用户的 `SELECT 1` FINISHED。容器没有 access-control 配置，不能验证 L2 真 ACL；104 条 spike 语料只剩 Markdown，临时实验代码已删除，需重建并区分原文/等价用例。
- 2026-10-26：Grok 第二意见经 ADR/REQUIREMENTS 过滤：拒绝「审计完整 SQL」（AUDIT-01 只要哈希/元数据）、「强制用户身份透传」（设计是可选）、「Phase 6 同时暴露 REST/MCP」（Phase 7）及「现有无 ACL 容器作为授权证据」。采纳固定对抗语料、越界 SQL 到 Trino 零发送、独立受限 ACL、审计队列故障注入。队满拒绝无法再可靠写入同一 sink，不能将此时审计条数等于所有尝试数作为验收。原 `/plan-eng-review` 于 `docs/designs/trino-engine-sql-tiers.md` Eng Review Gate 标记 CLEAR (PLAN)，Phase 6 仍须按现代码做针对性工程复审。
- 2026-10-26：本阶段只整理文档与只读探查，尚未修改业务代码，TDD 实施待 Phase 6 具体任务。`superpowers:brainstorming` 子任务触发判断将在 PLAN 任务切分后逐项执行；当前已由 accepted ADR 锁定对外合同，无新的用户问题。
