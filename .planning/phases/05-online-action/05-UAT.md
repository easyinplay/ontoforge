---
status: testing
phase: 05-online-action
source: [05-01-SUMMARY.md, 05-02-SUMMARY.md, 05-03-SUMMARY.md, 05-04-SUMMARY.md, 05-05-SUMMARY.md, 05-06-SUMMARY.md, 05-07-SUMMARY.md]
started: 2026-10-23T23:35:23+08:00
updated: 2026-10-23T23:35:23+08:00
---

## Current Test

number: 18
name: Phase 5 自动验收总体确认
expected: |
  Action 的参数、授权、原子写入、并发与幂等、按需 Trino、release、webhook 恢复及写入口约束均符合下面 17 项自动测试证据；没有未报告的实际使用偏差。
awaiting: user response

## Tests

### 1. Edit-only Action 单批次审计
expected: 一次提交写入一个批级审计回执和持久事件。
result: pass
source: automated
coverage_id: ACTION-TRACER

### 2. 双后端命令结构
expected: SQLite 与真实 PostgreSQL 的 command 和 outbox 表行为一致。
result: pass
source: automated
coverage_id: STORAGE-PARITY

### 3. 参数与规则执行
expected: 参数错误逐项返回，合法规则按顺序执行并形成原子批次回执。
result: pass
source: automated
coverage_id: ACTION-PARAMS-RULES

### 4. 隐藏目标与 criteria
expected: 遮蔽字段和越权写入被拒绝，隐藏与不存在目标的对外错误一致。
result: pass
source: automated
coverage_id: ACTION-MASKED-GUARD

### 5. Validate-only
expected: 执行同一授权和规则路径，不留下数据库写入或序号空洞。
result: pass
source: automated
coverage_id: ACTION-VALIDATE

### 6. Edit-only 版本令牌
expected: Query 签发的令牌可用于 Action，身份、scope、定义、edit 或密钥改变后失效。
result: pass
source: automated
coverage_id: ACTION-VERSION-TOKEN

### 7. 无令牌 CAS
expected: 未提供客户端令牌时仍核对观察版本，并回滚 version-0 skeleton。
result: pass
source: automated
coverage_id: ACTION-OBSERVED-CAS

### 8. 原子回滚
expected: 写入阶段故障在 SQLite 和 PostgreSQL 中恢复行、版本、日志、历史、命令与 outbox 快照。
result: pass
source: automated
coverage_id: ACTION-ATOMIC-ROLLBACK

### 9. PostgreSQL 并发竞争
expected: 同一目标的两个独立 store 只有一个成功，另一个收到 VersionConflict。
result: pass
source: automated
coverage_id: ACTION-PG-RACES

### 10. PostgreSQL 幂等
expected: 同键、异参、回滚、响应丢失及过期竞争均收敛到正确命令结果。
result: pass
source: automated
coverage_id: ACTION-IDEMPOTENCY-PG

### 11. 回放授权
expected: replay 与结果查询重查主体和当前 Action 权限，仅暴露最小历史回执。
result: pass
source: automated
coverage_id: ACTION-REPLAY-AUTH

### 12. Source-backed release
expected: releaseObject 在锁内核对写后可见性，被拒绝时没有持久化副作用。
result: pass
source: automated
coverage_id: ACTION-RELEASE-SOURCE

### 13. Source-backed 版本令牌
expected: 同源样本令牌可往返，source 漂移或 edit 插入竞争时拒写。
result: pass
source: automated
coverage_id: ACTION-SOURCE-TOKEN

### 14. 按需 Trino
expected: 真实 Trino 区分唯一、断连、观测重复 pk 与全 patch 零调用，失败前不写 PostgreSQL。
result: pass
source: automated
coverage_id: ACTION-ON-DEMAND-TRINO

### 15. Webhook 恢复
expected: crash、HTTP 失败、lease 过期与并行 worker 下至少一次投递且 event_id 稳定。
result: pass
source: automated
coverage_id: ACTION-OUTBOX-RECOVERY

### 16. Production 写入口
expected: 对象写入只经 Action 或带源文件摘要的 audited import。
result: pass
source: automated
coverage_id: ACTION-WRITE-ENTRY

### 17. CI 真实后端发现门槛
expected: PostgreSQL 和 Trino ignored tests 在 CI/nightly 有非零发现数及实际运行步骤。
result: pass
source: automated
coverage_id: ACTION-CI-GATES

### 18. Phase 5 自动验收总体确认
expected: 上述 Action 行为与实际使用预期一致，没有未报告的偏差。
result: [pending]

## Summary

total: 18
passed: 17
issues: 0
pending: 1
skipped: 0
blocked: 0

## Gaps

[none yet]
