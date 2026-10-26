# Phase 6 task plan

## Goal

交付 ROADMAP Phase 6 的 L2-01..04、AUDIT-01 与五条 Success Criteria。ROADMAP、REQUIREMENTS 和 accepted ADR 是合同真相源；本文件只跟踪工作顺序、文件所有权和验收。

## Tasks

- [x] P0：从 Phase 5 已完成提交建立 `gsd/phase-06-l2-toolsets-audit` 本地分支；保留既有未跟踪内容。
- [x] P1：核对 ADR-0016/0017/0018/0019、spike 2、Trino 483 环境与已有 auth/engine/query/storage/CLI 入口；产出 `06-CONTEXT.md` 与 findings。旧语料非机器可执行，复验列入 P2 的前置检查。
- [x] P2：跨 L2 权限、SQL AST、审计 fail-closed 与 reload 的工程审查；`06-ENG-REVIEW.md` 为基于现有代码的独立复审，不冒充新的正式 gstack gate。`06-SPIKE-483.md` 记录 29 条新构造语料及 Trino 483 复验，未覆盖的 75 个历史 ID 与真实 ACL 验收留在 P3/P4。
- [ ] P3：GSD plan-phase 研究并拆成可执行 PLAN；同步具体文件路径、依赖顺序、TDD RED/GREEN 和真实 Trino/PostgreSQL 验收；独立 checker 通过后更新本计划与 progress。
- [ ] P4：GSD execute-phase 依 wave 落实；核心 AST、安全自检、审计队列和管理通道按 TDD 实施，代码所有权以各 PLAN 为准。
- [ ] P5：GSD verify-work 与目标验证、代码审查、匹配 SHA 的 CI；通过后由 GSD 更新 ROADMAP/STATE/REQUIREMENTS。正式关键模块 PR gate 单独记录。

## Acceptance

- L2 的 Trino 侧身份安全无法证明时 fail-closed；AST 白名单、allow-list 与 `system` 排除有拒绝语料和真实 Trino 行为证据。
- 读取审计不含结果数据，队列积压与持续落库失败时返回 `AuditUnavailable`；管理变更受限且审计。
- 维持 T1 摘要、T2 细节、T3 测试，STATE 小于 100 行。

## Next Step

执行 P3：GSD research 先评估 SQL Display 语义缺口、固定 corpus、真实 Trino ACL、PG/SQLite 审计方案，再生成 PLAN 并经独立 checker 验证。
