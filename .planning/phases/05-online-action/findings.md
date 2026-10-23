# Phase 5 findings

- 2026-10-23：ROADMAP 已定义 Phase 5 Goal、五项 Success Criteria 与 ACTION-01..12；`.planning/phases/05-online-action/` 原先不存在。当前分支从已完成的 Phase 4 提交 `9c4ba02` 建立，尚无 Phase 5 代码改动。
- 2026-10-23：Action 涉及单事务 edit/edit log/`action_commands`/outbox、行列 scope、乐观并发与幂等、按需 Trino，是跨 query/auth/storage 的高可靠性工作。Phase 级 discuss 与架构审查触发；具体合同需先核对 ADR 和现有代码，不在此推定。
- 2026-10-23：只读调查确认 `ontoforge-action` 仍为占位；schema 已有 Action IR、rule 和参数的配置期校验，auth 已有 scope/criteria 列覆盖，query 已有 HMAC `_version` 与按需 source 的 pk 读。运行时参数校验、criteria、写前/写后授权、批量与回执均待实现。
- 2026-10-23：`EditStore::commit` 自行开启并结束事务，`WriteTx` 仅 crate 内可用。Phase 5 要先确定 storage 内部的组合事务入口，才能在锁内核验读集与版本，并原子写 edit、edit log、`action_commands`、outbox。存储层重复 `Commit` 会生成两条日志；ADR-0026 已将幂等归给 Action 层。
- 2026-10-23：待讨论的行为合同：批量是一个 `command_id`/一条日志/一个总回执，还是逐项身份与回执但共享事务；全 patch 的 source-backed 对象写入时，`AmbiguousObject` 检查与按需 Trino 如何同时成立；重放前重新鉴权的精确范围。ACTION-01 的运行时约束应与当前 schema 能力及未来 SCHEMAX-04 划界。ADR-0018 已定前置目标不可见与不存在同错，写后不可见为 `ScopeDenied`；v1 lint 已禁止同 Action 中新建对象供后续 rule 引用。
- 2026-10-23：验证重点为 SQLite/真实 PG 双后端事务回滚与并发，Trino 可用/不可用分流，遮蔽 criteria 的 dry-run 防探测，`command_id` 重放重新授权，outbox 重试。现有 storage conformance、query token、auth scope 测试可复用。
- 2026-10-23：Grok 只读第二意见支持以整个批次为幂等单元、批级回执与单条 outbox 事件，并指出“全 patch 跳过内容读”与“重复 pk 检查”语义不同。其全局唯一 `command_id`、逐 edit 日志、本地 PK registry 建议不符合 ADR-0018/0026 与现有结构，不直接采纳。已向用户提出批量粒度选择。
- 2026-10-23：`docs/designs/trino-engine-sql-tiers.md` 的 `batch` action 指 Nessie/Iceberg 写入模式，已推迟到下一 milestone；ACTION-05 的在线批量调用是 Phase 5 范围，两者不能混同。
- 2026-10-23：Grok 对 source 重复 pk 与离线写入的定向复核指出：无 PK registry、现有全 patch 跳 Trino、ADR-0018 允许 source 采样后漂移三者并存时，无法对未读取的 source 状态作绝对唯一性承诺。ACTION-12 已精确为“本次 Action 的 source 读取观测到重复 pk 时拒写”，仅含已编辑对象的路径保持 Trino 离线可执行；另一选择是每次 source-backed 写入都查重，但会改变 ROADMAP 的离线成功标准。
- 2026-10-23：GSD researcher 已交付 `05-RESEARCH.md`（`66b7bef`）。发现独立审查 E1 的 claim→fence 顺序违反现有定义栅栏首语句约束，已改为 fence→claim；该顺序必须有真实 PG 并发测试。研究的 Validation Architecture 已用于 `05-VALIDATION.md` 草案。
- 2026-10-23：独立架构审查 `05-ENG-REVIEW.md` 找到事务组合、共享 `_version`、删除后回执鉴权、同 `command_id` 并发争用四项 plan blocker；正式 gstack `plan-eng-review` 因当前无 AskUserQuestion 变体未通过 gate。Grok 对回执重鉴权的第二意见支持“主体限定、当前 action 权、最小审计摘要”，不以已删除行当前可见性否决回执。已在 CONTEXT 中定下此规则；ACTION-12 已按 source 实际观测语义精确化。
- 2026-10-23：`05-PATTERNS.md` 已映射 16 个候选文件，14 个有 tracked source analog；webhook worker 与 command fingerprint 没有现成实现。Grok 对旧 `_version` 兼容问题建议直接失效旧格式；现有 query 令牌不含 TTL，ADR-0018 已规定密钥轮换使旧令牌冲突。采纳新格式直接失效并要求客户端重新读取；不采纳新增 TTL、nonce 和强制轮换密钥，避免扩大当前合同。决策见 CONTEXT D-07。
- 2026-10-23：Grok 对正式 `plan-eng-review` 受 `AskUserQuestion` 缺失阻塞的第二意见建议以 `05-ENG-REVIEW.md` E1-E7 + GSD plan-checker 作为本 Phase 一次性替代治理；正式 gate 仍标 BLOCKED，关键模块 PR 前回补 `plan-eng-review` 和 `/review`。采纳前提是计划逐项闭合 E1-E7、checker 通过、无新架构级灰区；否则重新澄清。这不是后续 Phase 自动跳过 gstack 的先例，且不得把替代审查写作正式 gate PASS。
