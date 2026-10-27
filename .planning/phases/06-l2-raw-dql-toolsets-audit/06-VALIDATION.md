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
| 06-01-01 | 01 | 1 | AUDIT-01 | T-06-01/02 | pk 成功、拒绝、源错误、取消及故障时无结果释放 | SQLite integration | `cargo test -p ontoforge-query --test audit_pk --locked audit_pk_` | 待 Wave 0 | pending |
| 06-01-02 | 01 | 1 | AUDIT-01 | T-06-01/02 | worker 积压、退出、恢复与 drain | SQLite fault injection | `cargo test -p ontoforge-query --test audit_pk --locked audit_worker_` | 待 Wave 0 | pending |
| 待后续 wave 对齐 | 05-09 | 2-5 | AUDIT-01 | 各 PLAN | list/set/searchAround/aggregate 与六个 L2 入口均受审计门禁保护 | integration + fault injection | 各 `06-*-PLAN.md` 的 `<automated>` | 待 Wave 0 | pending |
| 待 PLAN 指派 | 待定 | 待定 | SC-5 | 待定 | reload lint 失败不切换、scope hash 错误不写、快照版本一致 | regression | PLAN 指定现有 CLI/storage/query 测试命令 | 已有测试，增量待定 | pending |

## Wave 0 Requirements

- [ ] 将 104 个历史 ID 全部映射到仓库内机器可执行 fixture，标明 `constructed-equivalent` 与已知不支持原因；目前原文恢复 0 条，29 条等价探针尚在临时目录。
- [ ] `crates/ontoforge-query/tests/audit_pk.rs` 为 Wave 1 RED 路径；避免 Rust `src/lib.rs` 内联测试无法被 GSD RED commit 路径门禁识别。
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

## 06-08 实施后证据（2026-10-28）

本地代码 SHA `ea695ed480d56744835bf69d505e5531eef61314`；远端 GitHub Actions 尚未触发。完整命令、偏差与失败历史详见 `06-08-SUMMARY.md`。

| 门禁 | 发现数 | 本地实际结果 |
| --- | ---: | --- |
| 独立受限/宽松对照 Trino 483 `trino_l2_acl_it --ignored` | 3 | 3 passed，约 3 秒；G16/G17 Display 通过受限身份执行，G11 被拒绝 |
| 104-ID corpus `l2_corpus_` | 6 | 6 passed；104 个 constructed-equivalent ID 分类覆盖 |
| PostgreSQL `conformance_pg audit_ --features embedded-pg --ignored` | 5 | 5 passed，约 10 秒 |
| SQLite `conformance_sqlite audit_` | 4 | 4 passed |
| query `audit_pk`、`audit_l0` | 9、10 | 9、9 passed；后者另 1 项需外部 PG，普通运行 ignored |
| engine `l2_gate` | 16 | 16 passed |
| Windows workspace `--test-threads=1` | 116 个结果组 | 1231 passed，98 ignored，0 failed |

`cargo fmt --all --check`、`cargo clippy --workspace --all-targets --locked -- -D warnings`、`git diff --check` 通过。默认并行 workspace 的两次失败和稳定方案详见 SUMMARY；单线程方式仍有慢机器墙钟风险，见 `deferred-items.md`。本文件的 phase-level sign-off 保持 pending，待 `$gsd-verify-work 06` 与远端 CI 证据对账。
