---
status: testing
phase: 08-production-webhook-end-to-end-delivery
source: [08-VERIFICATION.md]
started: 2026-11-08T20:45:48Z
updated: 2026-11-08T20:45:48Z
---

## Current Test

number: 1
name: FA-02 —— 第 17 次失败起固定 300 秒退避（本 phase 从未在运行时执行）
expected: |
  让接收端对同一 event_id 连续失败 >= 17 次（持续 503），观察 of_webhook_outbox 的 next_attempt_at 与 last_attempt_at 之差。
  期望：第 17 次失败之后间隔恒为 300 秒（1_i64 << (attempts - 1).min(16) 与 .min(300) 的运行时效果），不再指数增长。
  依据：crates/ontoforge-action/src/outbox.rs 的退避公式。已登记 .planning/WINDOWS.md 第 19 条（unrun-verify，open）。
awaiting: user response

## Tests

### 1. FA-02：第 17 次失败起固定 300 秒退避（未执行）
expected: 让接收端对同一 event_id 连续失败 >= 17 次（持续 503），观察 of_webhook_outbox 的 next_attempt_at 与 last_attempt_at 之差。第 17 次失败之后间隔恒为 300 秒，不再指数增长。
result: [pending]

### 2. Phase 8 结论位的人工签署（项目固定流程）
expected: 对照 CI run 42515808955 的日志与 phase8-webhook-ledger artifact 的 ledger.jsonl 人工复核：6 条原始记录、3 条事件各 2 次尝试（首条 503、末条 204）、相邻间隔 1.157s / 1.183s / 1.026s 落在 [0.9s, 30s]、唯一 event_id 集合与 PostgreSQL of_webhook_outbox 的 3 行完全相等；据此把结论位由 VERDICT: PENDING 改为人工签署（机器可判定结果只作证据，执行者与 verifier 均不代签）。
result: [pending]

## Summary

total: 2
passed: 0
issues: 0
pending: 2
skipped: 0
blocked: 0

## Gaps

无 blocker gap。15/16 条 must-have 已由仓库源码 + Linux CI run + 冻结归档三重证据判定通过；剩余 1 条（FA-02）是声明式 backstop —— 源码里有公式，但按 D-04 本 phase 只执行了第 1 次重试（实测 1.026–1.183 秒），「presence != behavior」，故转人工。

非阻塞 warning（不影响本 phase 的证据链，均已记入 08-VERIFICATION.md 与 08-REVIEW-DISPOSITION.md）：

- W-A 字面口径偏差：ROADMAP SC2 写「CI 日志留存每次尝试的次数与退避间隔」，实测逐次尝试数据经 artifact 留存而非日志正文（测试无 println!，receiver/worker 日志均 0 字节）。证据链完整，载体与字面有偏差。
- W-B ~ W-E：08-REVIEW.md 的 WR-01 ~ WR-04（stale ready 文件、ledger 不清空、seq 与文件顺序解耦、assert_ne! 恒真），只影响本地可重跑性，不影响 CI 证据。WR-04 意味着进程独立性应采信 remote_addr 而非该断言。
- W-F：08-02-SUMMARY.md:159 称归档 ledger 出自「多事件矩阵 run」，实际是 run 42515808955（ledger 时间戳 19:36 vs 18:34）；同文件 D3 行与索引记录正确，仅该行叙述有误。
- W-G：.planning/WINDOWS.md 第 20 条（FA-03 归档来源）仍 open。verifier 已用 ledger received_at 落在 run 42515808955 job 日志窗口内这一确定性证据把 FA-03 升级为机器可判定，但关闭该条仍属人工决定。
