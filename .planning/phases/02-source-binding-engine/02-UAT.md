---
status: partial
phase: 02-source-binding-engine
source: [02-VERIFICATION.md]
started: 2026-10-17T00:00:00Z
updated: 2026-10-19T00:00:00Z
---

## Current Test

[testing paused — 1 item outstanding]

## Tests

### 1. PR `integration` 作业首跑
expected: `trino_it` 与 `cli_trino_it` 的 `--ignored` 测试实际执行并全部通过
result: pass
note: "CI run 39219827829：trino_it 9 passed、cli_trino_it 2 passed（首跑 39218898185 因 seed.sql 引号 bug 失败，修复 a4fdfea）"

### 2. 首次 nightly（Trino 483、484 矩阵）
expected: 手动触发 nightly，两个版本的 Iceberg 作业都通过
result: blocked
blocked_by: server
reason: "nightly.yml 只在默认分支上可 workflow_dispatch / schedule（gh 返回 HTTP 404 not found on the default branch），需 PR #2 合并后触发"

### 3. E21 golden 定稿
expected: 按首跑日志修正 `describe_types_golden` 与 `decode_golden`，删除 `crates/ontoforge-engine/tests/trino_it.rs` 中的 `PROVISIONAL` 注释并提交
result: pass
note: "两个 golden 在首跑原样通过，PROVISIONAL 标记已移除"

### 4. WR-C06 镜像 digest 可拉取
expected: CI 中按 digest 钉住的 `postgres:17.11` 与 `trinodb/trino:483/484` 均拉取成功
result: pass
note: "run 39219827829 按 digest 拉取 postgres / trino 镜像成功"

### 5. WR-C07 真实 Trino 上的 `--check-pk`
expected: `check_pk_finds_duplicate_and_null_keys_on_ci_trino` 在 CI 中通过
result: pass
note: "check_pk_finds_duplicate_and_null_keys_on_ci_trino ok（run 39219827829）"

### 6. WR-C03 决策
expected: 决定是否修订 ADR-0021 第 7 条（推荐：带 fixture 的绑定映射到 Trino 时降为 Info），并按决定落地
result: pass
note: "选定方案 (a)：ADR-0022 修订 ADR-0021 第 7 条；落地 commit 4d681aa / 05362e5 / 4950d00"

## Summary

total: 6
passed: 5
issues: 0
pending: 0
skipped: 0
blocked: 1

## Gaps
