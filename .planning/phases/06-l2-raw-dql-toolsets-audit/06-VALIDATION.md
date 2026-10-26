---
phase: "06"
slug: "l2-raw-dql-toolsets-audit"
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-10-26"
---

# Phase 6 Validation Strategy

本文件在 PLAN 生成前固定验证能力与缺口。任务编号、wave 和命令须在 PLAN checker 后对齐；尚未运行的测试不算通过。

## Test Infrastructure

| Property | Value |
| --- | --- |
| Framework | Rust `cargo test`，已有 SQLite conformance、PostgreSQL ignored target、Trino integration target |
| Config file | `Cargo.toml`、`.github/workflows/ci.yml`、`ci/query-services.sh` |
| Quick run command | 各 PLAN 任务的目标 `cargo test -p <crate> --locked <filter>` |
| Full suite command | `cargo test --workspace --locked`，另执行真实 PostgreSQL 和受限 Trino 483 ignored targets |
| Estimated runtime | PLAN 中测得并更新，不预估未建的 ACL fixture |

## Sampling Rate

- 每个任务提交后运行其 `<automated>` 命令，核心逻辑先保留 RED 失败证据，再运行 GREEN。
- 每个 wave 后运行受影响 crate 的完整测试；涉及外部服务时运行对应 ignored target，确认发现数大于 0。
- `$gsd-verify-work` 前运行 workspace 全量测试和两种真实服务门禁；独立记录通过的 commit SHA。
- 反馈延迟在首个 wave 实测后填写，当前不声称已有测量。

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 待 PLAN 指派 | 待定 | 待定 | L2-01..03 | 待定 | 104-ID 固定语料、AST 拒绝零 POST、Display body、行数/字节上限 | unit + Trino 483 | `cargo test -p ontoforge-engine --locked l2_` | 待 Wave 0 | pending |
| 待 PLAN 指派 | 待定 | 待定 | L2-04 | 待定 | 真实 ACL 拒绝、另一主体活跃 query 隔离、失败时 L2 disabled | Trino 483 integration | `cargo test -p ontoforge-engine --test trino_l2_acl_it --locked -- --ignored` | 待 Wave 0 | pending |
| 待 PLAN 指派 | 待定 | 待定 | AUDIT-01 | 待定 | 双后端持久化、保留期、队满/sink 失败/取消 fail-closed | unit + SQLite/PG conformance | `cargo test -p ontoforge-storage --test conformance_sqlite --locked audit_` | 待 Wave 0 | pending |
| 待 PLAN 指派 | 待定 | 待定 | AUDIT-01 | 待定 | pk/list/set/searchAround/aggregate 与六个 L2 入口均受审计门禁保护 | unit + fault injection | `cargo test -p ontoforge-query --locked audit_` | 待 Wave 0 | pending |
| 待 PLAN 指派 | 待定 | 待定 | SC-5 | 待定 | reload lint 失败不切换、scope hash 错误不写、快照版本一致 | regression | PLAN 指定现有 CLI/storage/query 测试命令 | 已有测试，增量待定 | pending |

## Wave 0 Requirements

- [ ] 将 104 个历史 ID 全部映射到仓库内机器可执行 fixture，标明 `constructed-equivalent` 与已知不支持原因；目前原文恢复 0 条，29 条等价探针尚在临时目录。
- [ ] 独立受限 Trino 483 ACL fixture 与 `trino_l2_acl_it` target，CI 确认确实执行。
- [ ] 审计 PG/SQLite migration、conformance 和故障注入用例的 RED 测试骨架。
- [ ] PLAN 写清每项具体路径、命令、测试发现数和先决服务。

## Manual-Only Verifications

无计划中的纯人工验收。生产环境 webhook 和 REST/MCP 外壳属于 Phase 7；Phase 6 的 Trino ACL 必须自动化实测。

## Validation Sign-Off

- [ ] 全部任务有 `<automated>` 验证或明确的 Wave 0 依赖。
- [ ] 连续任务之间保有自动化反馈，不出现三个连续任务无验证。
- [ ] Wave 0 的缺失 fixture 和 target 全部就位。
- [ ] 命令无 watch mode，真实服务测试实际运行且发现数大于 0。
- [ ] 反馈延迟已实测，最终将 frontmatter 标成 `nyquist_compliant: true`。

**Approval:** pending
