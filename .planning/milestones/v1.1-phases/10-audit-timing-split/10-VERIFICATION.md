---
phase: 10-audit-timing-split
verified: 2026-11-10
verdict: ACCEPTED
---

# Phase 10 Verification — Audit Timing Test Split

## VERDICT: ACCEPTED（机器证据；REL-04）

REL-04 验收闭合：security.rs 纯计算断言与墙钟敏感集成验证分离，CI 不再需要
`--test-threads=1`。

## Truths

| # | Truth | 证据 |
|---|-------|------|
| 1 | 16MiB 字节上限断言为确定性纯测试（无墙钟等待） | `security.rs` 拆为 `cap_counts_final_visible_row_and_version` + `cap_ignores_masked_large_source_value`，各用独立 fixture + 单次 `execute_list`，mask 查询前配好 |
| 2 | read-audit pending 期限有专属确定性测试 | `audit_pending_age_limit_fails_closed`：`open_with_read_audit` 注入小容量 + `BEGIN EXCLUSIVE` 锁制造 sink 失败 → 15s 边界等 `AuditUnavailable` + durable 0 行 |
| 3 | CI 移除 `--test-threads=1` 后默认并行稳定 | `.github/workflows/ci.yml` workspace 测试裸跑；ignored PG/Trino 共享服务 job 保留串行（已说明理由）；PR #14 合并前 CI 全绿 |
| 4 | cardinality E2E 语义不变、余量放宽 | `cli_check_cardinality.rs` 180ms→1500ms / 200ms timeout 放宽；13 测试全过；WINDOWS#18 → fixed |

## 证据

- `cargo test -p ontoforge-query --test security` → 25 passed
- `cargo test -p ontoforge --test cli_check_cardinality` → 13 passed
- `cargo test --workspace --locked`（默认并行）→ 全绿（本机 + CI PR #14 run）
- 第二意见（gpt-6-1-sol）已采纳：一 fixture 一查询拆法、sink 真实故障触发期限、
  `pending==0` ≠ 落盘、`start_paused` 对 `std::time::Instant` 无效
- prompt 归档 `.planning/research/phase10-second-opinion-prompt.md`

## 交付物

- `crates/ontoforge-query/tests/security.rs`（测试拆分 + audit 期限测试）
- `crates/ontoforge/tests/cli_check_cardinality.rs`（余量放宽）
- `.github/workflows/ci.yml`（workspace job 去串行）
- WINDOWS#18 关闭（`.planning/WINDOWS.md`）
