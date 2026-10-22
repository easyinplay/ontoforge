# Phase 4 findings

- Phase 3 的 `03-UAT.md` 为 `status: complete`、2/2；`03-VERIFICATION.md` 为 `passed`、8/8。V2 SQLite CI 跟进后的 PR HEAD `c57165d` 在 run 39715917662 首跑三 job 通过。
- Phase 4 范围由 QUERY-01..10 和 ROADMAP success criteria 锁定；ADR-0020、ADR-0018 与设计文档已有 Eng Review 决策，当前没有必须重新询问用户的 Phase 级灰区。战略层与 Phase 讨论跳过，因为此阶段延续既定产品方向、设计选择已有 accepted ADR。
- `ontoforge-query` 当前仅占位，`lib.rs` 与 Cargo description 仍提 `sea-query` 编译；ADR-0020 已取代该方案，规划与实施须以 ADR 为准。
- GSD Core runtime identity 为 `@opengsd/gsd-core` 1.18.0。`init.phase-op 4` 显示 phase 目录尚不存在；`init.plan-phase 4 --ingest ...` 显示 QUERY-01..10、research/checker 开启。全局 model defaults 比已安装 agent frontmatter 新，但项目配置优先，未改动全局安装。
- 当前只写规划文档，TDD 尚不适用；进入核心叠加、授权、令牌与聚合实现时须按 red-green-refactor 执行。
- `04-RESEARCH.md` 发现 storage 的写事务读取会创建 version 0 行，Phase 4 须新增无副作用的有界 edit/link 读取 API；engine typed AST 尚无列遮蔽 `CASE`；单表 AST 不能独自证明 Trino ≤2 fragment，真实 Trino `EXPLAIN` 需入验证。
- `04-PATTERNS.md` 映射了 engine `Select`/`Engine`、auth `TypeScope::evaluate/lower`、storage edit 解码与现有 proptest 差分模式；缺少 Rust 叠加、补取、keyset、聚合的现成实现。
- 研究的成本与游标问题按已接受 ADR 的安全边界规划：结果行/字节上限产生 `truncated`；无法保证完整结果的工作预算耗尽和高成本 distinct 返回明确错误；游标不得跨主体、scope、定义或排序上下文复用。具体阈值和密钥注入位置由 planner 写成可测试合同。
