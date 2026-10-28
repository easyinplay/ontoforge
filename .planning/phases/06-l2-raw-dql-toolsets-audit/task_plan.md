# Phase 6 task plan

## Goal

交付 ROADMAP Phase 6 的 L2-01..04、AUDIT-01、SCOPE-07 请求版本固定与五条 Success Criteria。ROADMAP、REQUIREMENTS 和 accepted ADR 是合同真相源；本文件只跟踪工作顺序、文件所有权和验收。

## Tasks

- [x] P0：从 Phase 5 已完成提交建立 `gsd/phase-06-l2-toolsets-audit` 本地分支；保留既有未跟踪内容。
- [x] P1：核对 ADR-0016/0017/0018/0019、spike 2、Trino 483 环境与已有 auth/engine/query/storage/CLI 入口；产出 `06-CONTEXT.md` 与 findings。旧语料非机器可执行，复验列入 P2 的前置检查。
- [x] P2：跨 L2 权限、SQL AST、审计 fail-closed 与 reload 的工程审查；`06-ENG-REVIEW.md` 为基于现有代码的独立复审，不冒充新的正式 gstack gate。`06-SPIKE-483.md` 记录 29 条新构造语料及 Trino 483 复验，未覆盖的 75 个历史 ID 与真实 ACL 验收留在 P3/P4。
- [x] P3：GSD research、pattern map 和 9 个可执行 PLAN 已落盘；独立 checker 第三轮 `VERIFICATION PASSED`，六个 requirement ID 与六项 CONTEXT 决策全覆盖。规划通过不等于实施测试通过。
- [x] P4：GSD execute-phase 依 wave 落实；9 个 PLAN 均有 SUMMARY（详各 SUMMARY）。核心 AST、安全自检、审计队列和管理通道按 TDD 实施，代码所有权以各 PLAN 为准。
- [x] P4b：WR-01 库级启动与周期 ACL attestation。用户已选部署方配置有界 probe SQL，`06-10-SUMMARY.md` 记录失败、过期、恢复、真实 Trino 483 负向 ACL 及最终 workspace/Clippy/fmt；Phase 7 的服务及 `/health` 接线由 ADR-0028 另行验收。
- [ ] P5：原 21/21 及 `20fd0005` CI 证据保留；正式 PR review 新增 CR-01，须完成 P4c 后增量复签，不将旧 CI 当作新实现证据。
- [ ] P4c：`06-11-PLAN.md` 修复 audit shutdown 的并发、取消后重试和失败结果保留；双后端 TDD、增量 review 与新 SHA CI 通过后再关闭阶段。

## Acceptance

- L2 的 Trino 侧身份安全无法证明时 fail-closed；AST 白名单、allow-list 与 `system` 排除有拒绝语料和真实 Trino 行为证据。
- 读取审计不含结果数据，队列积压与持续落库失败时返回 `AuditUnavailable`；管理变更受限且审计。
- 维持 T1 摘要、T2 细节、T3 测试，STATE 小于 100 行。

## Execute Order

| Wave | Plans | 主要文件所有权 | 前置 |
| --- | --- | --- | --- |
| 1 | `06-01`, `06-03` | storage/query 审计 tracer；engine SQL gate/corpus | P3 checker passed |
| 2 | `06-02`, `06-04`, `06-05` | PG 审计；engine ACL fixture；query L0 其余读 | Wave 1 |
| 3 | `06-06` | query L2 execute/explain | `06-01`, `06-03`, `06-04` |
| 4 | `06-07`, `06-09` | query L2 元数据发现；CLI 管理/快照回归 | `06-06` 及各自前置 |
| 5 | `06-08` | CI 受限 Trino/PG 实测门禁 | Wave 2-4 |

具体文件、TDD RED/GREEN、命令和验收以各 `06-*-PLAN.md` 为准；真实 ACL 与 PostgreSQL 通过前不得宣称 Phase 6 完成。

## Next Step

先执行 `06-11-PLAN.md`：旧实现的 shutdown 并发/取消重试会提前成功，须先取得行为 RED 再修复，审查和新 SHA CI 通过后增量复签。旧 21/21 及 `20fd0005` CI 作为历史证据保留。Phase 7 未启动，L2-04/SCOPE-07 整体 Pending，不执行合并；Phase 1–5 历史 fingerprint staleness 留给 milestone audit。
