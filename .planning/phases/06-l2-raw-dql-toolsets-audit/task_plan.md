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
- [ ] P5：GSD verify-work 与目标验证、代码审查、匹配 SHA 的 CI；通过后由 GSD 更新 ROADMAP/STATE/REQUIREMENTS。正式关键模块 PR gate 单独记录。当前 CI 已暴露 `of_read_audit` 期望遗漏与 workspace 审计时序失败，需先完成窄范围修正/复验。

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

正式目标验证已增量复签为 `human_needed`、19/21（详 `06-VERIFICATION.md`）。H1/H2 与最终 Clippy 已凭提交 `ecb9883` 的原始日志核验，仅剩 H3 当前交付 SHA 的远端 CI。Codex/gstack 新一轮 review 因服务过载中断，未记为 clean；先 push 当前分支并创建 draft PR 触发 `pull_request` CI，不执行合并。远端通过且正式 review 完成后需由 verifier 再签署，才能关闭 Phase 6。Phase 7 服务接线及 `/health` 保留阻断验收，L2-04/SCOPE-07 整体 Pending。GSD 间歇 Git 漏读尚未根治，见 findings。
