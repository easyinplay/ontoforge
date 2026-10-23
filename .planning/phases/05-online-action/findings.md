# Phase 5 findings

- 2026-10-23：ROADMAP 已定义 Phase 5 Goal、五项 Success Criteria 与 ACTION-01..12；`.planning/phases/05-online-action/` 原先不存在。当前分支从已完成的 Phase 4 提交 `9c4ba02` 建立，尚无 Phase 5 代码改动。
- 2026-10-23：Action 涉及单事务 edit/edit log/`action_commands`/outbox、行列 scope、乐观并发与幂等、按需 Trino，是跨 query/auth/storage 的高可靠性工作。Phase 级 discuss 与架构审查触发；具体合同需先核对 ADR 和现有代码，不在此推定。
- 2026-10-23：只读调查确认 `ontoforge-action` 仍为占位；schema 已有 Action IR、rule 和参数的配置期校验，auth 已有 scope/criteria 列覆盖，query 已有 HMAC `_version` 与按需 source 的 pk 读。运行时参数校验、criteria、写前/写后授权、批量与回执均待实现。
- 2026-10-23：`EditStore::commit` 自行开启并结束事务，`WriteTx` 仅 crate 内可用。Phase 5 要先确定 storage 内部的组合事务入口，才能在锁内核验读集与版本，并原子写 edit、edit log、`action_commands`、outbox。存储层重复 `Commit` 会生成两条日志；ADR-0026 已将幂等归给 Action 层。
- 2026-10-23：待讨论的行为合同：批量是一个 `command_id`/一条日志/一个总回执，还是逐项身份与回执但共享事务；全 patch 的 source-backed 对象写入时，`AmbiguousObject` 检查与按需 Trino 如何同时成立；重放前重新鉴权的精确范围。ACTION-01 的运行时约束应与当前 schema 能力及未来 SCHEMAX-04 划界。ADR-0018 已定前置目标不可见与不存在同错，写后不可见为 `ScopeDenied`；v1 lint 已禁止同 Action 中新建对象供后续 rule 引用。
- 2026-10-23：验证重点为 SQLite/真实 PG 双后端事务回滚与并发，Trino 可用/不可用分流，遮蔽 criteria 的 dry-run 防探测，`command_id` 重放重新授权，outbox 重试。现有 storage conformance、query token、auth scope 测试可复用。
- 2026-10-23：Grok 只读第二意见支持以整个批次为幂等单元、批级回执与单条 outbox 事件，并指出“全 patch 跳过内容读”与“重复 pk 检查”语义不同。其全局唯一 `command_id`、逐 edit 日志、本地 PK registry 建议不符合 ADR-0018/0026 与现有结构，不直接采纳。已向用户提出批量粒度选择。
- 2026-10-23：`docs/designs/trino-engine-sql-tiers.md` 的 `batch` action 指 Nessie/Iceberg 写入模式，已推迟到下一 milestone；ACTION-05 的在线批量调用是 Phase 5 范围，两者不能混同。
- 2026-10-23：Grok 对 source 重复 pk 与离线写入的定向复核指出：无 PK registry、现有全 patch 跳 Trino、ADR-0018 允许 source 采样后漂移三者并存时，无法对未读取的 source 状态作绝对唯一性承诺。建议 ACTION-12 将 `AmbiguousObject` 精确定为“本次 Action 的 source 读取观测到重复 pk 时拒写”，仅含已编辑对象的路径保持 Trino 离线可执行。这是需求语义收窄，尚待用户裁决；另一选择是每次 source-backed 写入都查重，但会改变 ROADMAP 的离线成功标准。
