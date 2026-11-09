---
status: testing
phase: 08-production-webhook-end-to-end-delivery
source: [08-VERIFICATION.md]
started: 2026-11-08T20:45:48Z
updated: 2026-11-09T13:37:00Z
---

## Current Test

number: 2
name: Phase 8 结论位的人工签署（项目固定流程）
expected: |
  对照 CI run 42515808955 的日志与 phase8-webhook-ledger artifact 的 ledger.jsonl 人工复核：6 条原始记录、3 条事件各 2 次尝试（首条 503、末条 204）、相邻间隔 1.157s / 1.183s / 1.026s 落在 [0.9s, 30s]、唯一 event_id 集合与 PostgreSQL of_webhook_outbox 的 3 行完全相等；据此把结论位由 VERDICT: PENDING 改为人工签署（机器可判定结果只作证据，执行者与 verifier 均不代签）。
awaiting: user response

## Tests

### 1. FA-02：退避上限 300 秒（attempt 10 起恒定；已由可控时钟运行时验证）
expected: |
  对同一 event_id 连续失败 20 次（wiremock 持续 503，可控时钟驱动 WebhookWorker.poll_at），观察 of_webhook_outbox 的 next_attempt_at 推进。
  期望：第 k 次已记录失败的退避 = min(2^(k-1), 300) 秒，即 k=1..9 为 1/2/4/8/16/32/64/128/256 秒，k>=10 起恒为 300 秒（1_i64 << (attempts - 1).min(16) 与 .min(300) 的运行时效果；.min(16) 仅为移位溢出保护，在 .min(300) 之后不可观测）。
  依据：crates/ontoforge-action/src/outbox.rs:157-158；
  运行时证据：crates/ontoforge-action/tests/outbox.rs retry_backoff_caps_at_five_minutes_without_leaking_response（循环上限扩至 20 并固化进本分支，实测 cargo test -p ontoforge-action --locked --test outbox → 11 passed / 0 failed / 0.97s）。
  未覆盖声明：该证据为 SQLite + 可控时钟 + 单 worker；PostgreSQL 后端与多 worker 租约抢占不在本条范围内，但退避公式与 claim/retry 代码路径与后端无关。
result: pass

### 2. Phase 8 结论位的人工签署（项目固定流程）
expected: 对照 CI run 42515808955 的日志与 phase8-webhook-ledger artifact 的 ledger.jsonl 人工复核：6 条原始记录、3 条事件各 2 次尝试（首条 503、末条 204）、相邻间隔 1.157s / 1.183s / 1.026s 落在 [0.9s, 30s]、唯一 event_id 集合与 PostgreSQL of_webhook_outbox 的 3 行完全相等；据此把结论位由 VERDICT: PENDING 改为人工签署（机器可判定结果只作证据，执行者与 verifier 均不代签）。
result: [pending]

## Summary

total: 2
passed: 1
issues: 0
pending: 1
skipped: 0
blocked: 0

## Gaps

FA-02 已补充 attempts 1..20 的 SQLite 运行时验证，覆盖 attempts 17..20；这不是 PostgreSQL 生产端到端的高次重试证据。其余 15 条沿用原验证报告，本次未重新核验。测试 2 的人工签署仍待执行，VERDICT 保持 PENDING。

非阻塞 warning（不影响本 phase 的证据链，均已记入 08-VERIFICATION.md 与 08-REVIEW-DISPOSITION.md）：

- W-A 字面口径偏差：ROADMAP SC2 写「CI 日志留存每次尝试的次数与退避间隔」，实测逐次尝试数据经 artifact 留存而非日志正文（测试无 println!，receiver/worker 日志均 0 字节）。证据链完整，载体与字面有偏差。
- W-B ~ W-E：08-REVIEW.md 的 WR-01 ~ WR-04（stale ready 文件、ledger 不清空、seq 与文件顺序解耦、assert_ne! 恒真），只影响本地可重跑性，不影响 CI 证据。WR-04 意味着进程独立性应采信 remote_addr 而非该断言。
- W-F：08-02-SUMMARY.md:159 称归档 ledger 出自「多事件矩阵 run」，实际是 run 42515808955（ledger 时间戳 19:36 vs 18:34）；同文件 D3 行与索引记录正确，仅该行叙述有误。
- W-G：.planning/WINDOWS.md 第 20 条（FA-03 归档来源）仍 open。verifier 已用 ledger received_at 落在 run 42515808955 job 日志窗口内这一确定性证据把 FA-03 升级为机器可判定，但关闭该条仍属人工决定。
