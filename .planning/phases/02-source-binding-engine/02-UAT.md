---
status: testing
phase: 02-source-binding-engine
source: [02-VERIFICATION.md]
started: 2026-10-17T00:00:00Z
updated: 2026-10-17T00:00:00Z
---

## Current Test

number: 1
name: PR `integration` 作业首跑
expected: |
  分支推送并开 PR 后，`integration` 作业中 `trino_it` 与 `cli_trino_it` 的 `--ignored` 测试实际执行数各 ≥ 1 且全部通过（SC2、SC4、SC5）
awaiting: user response

## Tests

### 1. PR `integration` 作业首跑
expected: `trino_it` 与 `cli_trino_it` 的 `--ignored` 测试实际执行并全部通过
result: [pending]

### 2. 首次 nightly（Trino 483、484 矩阵）
expected: 手动触发 nightly，两个版本的 Iceberg 作业都通过
result: [pending]

### 3. E21 golden 定稿
expected: 按首跑日志修正 `describe_types_golden` 与 `decode_golden`，删除 `crates/ontoforge-engine/tests/trino_it.rs` 中的 `PROVISIONAL` 注释并提交
result: [pending]

### 4. WR-C06 镜像 digest 可拉取
expected: CI 中按 digest 钉住的 `postgres:17.11` 与 `trinodb/trino:483/484` 均拉取成功
result: [pending]

### 5. WR-C07 真实 Trino 上的 `--check-pk`
expected: `check_pk_finds_duplicate_and_null_keys_on_ci_trino` 在 CI 中通过
result: [pending]

### 6. WR-C03 决策
expected: 决定是否修订 ADR-0021 第 7 条（推荐：带 fixture 的绑定映射到 Trino 时降为 Info），并按决定落地
result: [pending]

## Summary

total: 6
passed: 0
issues: 0
pending: 6
skipped: 0
blocked: 0

## Gaps
