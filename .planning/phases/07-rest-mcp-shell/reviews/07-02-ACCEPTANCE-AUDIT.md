# 07-02 Acceptance Audit

日期：2026-10-29。独立 `gsd-code-reviewer` 只读审查，未运行重复 workspace 测试。结论：验收未通过，原完成结论撤销，保留实现记录。

| 级别 | 问题 | 最小修复与验收 |
|------|------|----------------|
| BLOCKER | `action/src/lib.rs:314` replay 命中后重读 current_action_definition，绕过传入 snapshot | replay 使用传入 active；snapshot 捕获后外部移除 Action，再 submit replay 必须返回原 receipt。旧 wrapper 当前定义拒绝仍成立 |
| WARNING | `storage/tests/request_snapshot.rs:36` 仅顺序 seed 后单读，无竞态和代际切换 | 双 store 共同锁/barrier，覆盖 reload/scope_apply/revoke 交错、旧 snapshot 不变、新 snapshot 见完整提交代际 |
| WARNING | PG case 吞掉 request_snapshot 错误，没有断言 | 复用完整 metadata race 合同，成功值/代际断言；保留 ignored，07-13 真实执行 |
| WARNING | frozen fence/CAS/guard 测试只触发 stale，expected_versions 为空，零写只数 log | 分别触发 validate success、CAS/guard/stale 拒绝，每次比较 edit/log/command/outbox 基线 |
| WARNING | frozen history 没有定义切换或遮蔽/撤权，仅看对象 version | 旧 snapshot + 外部 reload，当前 scope 隐藏值与不可见拒绝；不要把对象 version 当 definition version |

补充：现有原 idempotency 测试确实覆盖变 clock/定义后 receipt、授权撤销/Action 移除/跨主体与最小 receipt；原 history_masking 覆盖 keyset/整记录页。缺口是这些行为与 admission snapshot/外部变更的组合。返回后锁释放用例有有效断言，未报告为缺陷。已有正确行为应记录 characterization，不制造假 RED。
