---
phase: "2"
slug: "source-binding-engine"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-10-16"
---

# Phase 2 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> 需求到测试的映射见 `02-RESEARCH.md` § Validation Architecture，本文件不复制。

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` + `insta` + `proptest` + `wiremock` + `sqlx` sqlite（测试内造表） |
| **Config file** | none；真实 Trino 测试以 `#[ignore]` 门控，`PROPTEST_CASES` 调例数 |
| **Quick run command** | `cargo test -p <被改动的 crate>` |
| **Full suite command** | `cargo test --workspace --locked` |
| **Linux-only integration** | `cargo test -p ontoforge-engine --test trino_it --locked -- --ignored` |
| **Estimated runtime** | ~120 seconds（冷构建除外） |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p <被改动的 crate>`
- **After every plan wave:** Run `cargo test --workspace --locked` + `cargo clippy --workspace --all-targets --locked -- -D warnings` + `cargo fmt --all --check`
- **Before `/gsd-verify-work`:** Full suite must be green（Windows 与 Ubuntu），Linux 集成 job 实际执行 ≥ 1 个 `--ignored` 测试
- **Max feedback latency:** 120 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 02-01-T1 | 02-01 | 1 | BIND-01 | T-02-01, T-02-02 | 缺 backing 即 V05，不默认；JSON 与 human 快照语义零漂移（E19） | integration + snapshot | `cargo test --workspace` | ✅ 框架在 | ⬜ pending |
| 02-01-T2 | 02-01 | 1 | BIND-01 | — | ADR / REQUIREMENTS / ROADMAP 与代码一致（E6） | doc check | `rg -n "status: accepted" .planning/adr/0021-source-binding-files-and-backing.md` | ❌ W0 | ⬜ pending |
| 02-01-T3 | 02-01 | 1 | BIND-01 | T-02-48 | 共享有界读取、ProfileName、V16..V20（E10、E14） | unit | `cargo test -p ontoforge-schema` | ✅ 框架在 | ⬜ pending |
| 02-02-T1 | 02-02 | 1 | ENGINE-03, ENGINE-04 | T-02-04, T-02-06 | PrintedSql printer 私有构造（E4）、替身流式取数（E24）、错误分类（E3） | integration | `cargo test -p ontoforge-engine --test sqlite_engine` | ❌ W0 | ⬜ pending |
| 02-02-T2 | 02-02 | 1 | ENGINE-03 | T-02-05 | 字面量经唯一的 split_decimal 等校验后经 quote_string（E7） | unit + snapshot | `cargo test -p ontoforge-engine` | ❌ W0 | ⬜ pending |
| 02-02-T3 | 02-02 | 1 | ENGINE-03 | T-02-04 | 注入性质测试 | proptest + source scan | `cargo test -p ontoforge-engine --test printer_prop --test sql_text_only_from_printer` | ❌ W0 | ⬜ pending |
| 02-03-T1 | 02-03 | 2 | BIND-01 | T-02-07 | fixture 路径不逃逸；只排除 bindings/<p>/fixtures（E5） | integration | `cargo test -p ontoforge-schema` | ❌ W0 | ⬜ pending |
| 02-03-T2 | 02-03 | 2 | BIND-01 | T-02-08 | profile 名不进路径 | CLI e2e | `cargo test -p ontoforge --test cli --test cli_lint --test invalid_examples` | ✅ 需更新 | ⬜ pending |
| 02-03-T3 | 02-03 | 2 | BIND-01, BIND-02 | T-02-07, T-02-09 | 控制字符拒绝；fixture 存在性只查元数据（E23） | invalid 样例 + unit | `cargo test --workspace` | ✅ 框架在 | ⬜ pending |
| 02-04-T1 | 02-04 | 2 | ENGINE-01 | T-02-11 | nextUri 同源 | wiremock | `cargo test -p ontoforge-engine --test trino_client` | ❌ W0 | ⬜ pending |
| 02-04-T2 | 02-04 | 2 | ENGINE-01 | T-02-12, T-02-13, T-02-14, T-02-44, T-02-16 | 密钥不外泄、无跳过 TLS、凭据只走 https 且库层也拒绝（E2） | unit + wiremock | `cargo test -p ontoforge-engine --test server_config --test trino_client` | ❌ W0 | ⬜ pending |
| 02-04-T3 | 02-04 | 2 | ENGINE-01, ENGINE-02 | — | 按 catalog 批量描述、截断即错误（E22） | wiremock | `cargo test -p ontoforge-engine` | ❌ W0 | ⬜ pending |
| 02-05-T1 | 02-05 | 3 | BIND-03, ENGINE-04 | T-02-18, T-02-45 | 值只经 bind；单 catalog 错误隔离（E3） | CLI e2e + wiremock | `cargo test -p ontoforge --test cli_online && cargo test -p ontoforge-engine --test bind_check_isolation` | ❌ W0 | ⬜ pending |
| 02-05-T2 | 02-05 | 3 | ENGINE-04 | T-02-19, T-02-20 | query_only、单文件与总量上限（E23）、共享表合并（E18） | integration + unit | `cargo test -p ontoforge-engine --test sqlite_engine && cargo test -p ontoforge-engine shared_table` | ❌ W0 | ⬜ pending |
| 02-06-T1 | 02-06 | 3 | ENGINE-02 | — | fail-closed 分类 | wiremock + closed port | `cargo test -p ontoforge-engine --test trino_resilience` | ❌ W0 | ⬜ pending |
| 02-06-T2 | 02-06 | 3 | ENGINE-01 | T-02-25, T-02-26 | 重试有界；POST 普通 503 不重试（E16） | wiremock | `cargo test -p ontoforge-engine --test trino_resilience` | ❌ W0 | ⬜ pending |
| 02-06-T3 | 02-06 | 3 | ENGINE-01 | T-02-23, T-02-24 | 三个触发点 DELETE | wiremock | `cargo test -p ontoforge-engine --test trino_resilience --test trino_client` | ❌ W0 | ⬜ pending |
| 02-07-T1 | 02-07 | 3 | BIND-02 | — | — | lint snapshot + CLI | `cargo test -p ontoforge-schema lint:: && cargo test -p ontoforge --test cli_lint` | ✅ 框架在 | ⬜ pending |
| 02-07-T2 | 02-07 | 3 | BIND-02 | — | decimal 缺省只取 effective_decimal（E7） | lint snapshot | `cargo test -p ontoforge-schema lint::` | ✅ 框架在 | ⬜ pending |
| 02-07-T3 | 02-07 | 3 | BIND-02 | T-02-29, T-02-49 | 表头解析有界且计入总预算（E23）；不可读时不静默（E20） | unit + snapshot | `cargo test -p ontoforge-schema` | ✅ 框架在 | ⬜ pending |
| 02-08-T1 | 02-08 | 4 | BIND-03 | T-02-32, T-02-47 | drift 不全表扫描；单目标不可读只降级该目标（E3）；healthy 保留 warning（E12） | integration | `cargo test -p ontoforge-engine --test drift` | ❌ W0 | ⬜ pending |
| 02-08-T2 | 02-08 | 4 | BIND-03, BIND-02 | T-02-31 | 只接受无损映射；大小写歧义为 error（E13） | unit（表驱动） | `cargo test -p ontoforge-engine bind_check` | ❌ W0 | ⬜ pending |
| 02-08-T3 | 02-08 | 4 | BIND-02 | — | 文档与代码一致；写明 fixture 模式验证边界（E15） | doc-driven test | `cargo test -p ontoforge-engine --test doc_type_table` | ❌ W0 | ⬜ pending |
| 02-09-T1 | 02-09 | 4 | BIND-03, ENGINE-02, ENGINE-01 | T-02-34, T-02-35 | 退出码 3、密钥不打印、单 catalog 错误隔离（E3） | CLI + wiremock | `cargo test -p ontoforge --test cli_online_trino` | ❌ W0 | ⬜ pending |
| 02-09-T2 | 02-09 | 4 | BIND-03 | T-02-37, T-02-46 | 只在 --check-pk 时查重；NULL 不算重复（E17） | CLI + fixture / wiremock | `cargo test -p ontoforge-engine --test pk_check && cargo test -p ontoforge --test cli_check_pk` | ❌ W0 | ⬜ pending |
| 02-10-T1 | 02-10 | 5 | ENGINE-05 | T-02-40, T-02-41, T-02-43 | 缺环境变量 panic；PG 端口显式发布（E25） | ignored integration + workflow check | `cargo test -p ontoforge-engine --test trino_it` | ❌ W0 | ⬜ pending |
| 02-10-T2 | 02-10 | 5 | ENGINE-05, ENGINE-01, ENGINE-03 | T-02-43 | 执行数量门禁；golden 首跑定稿（E21） | ignored integration（Linux CI） | `cargo test -p ontoforge-engine --test trino_it -- --ignored`（CI） | ❌ W0 | ⬜ pending |
| 02-10-T3 | 02-10 | 5 | ENGINE-05 | T-02-39, T-02-42 | 最小权限（issues: write 只在 report-failure job，E25）、私有数据不进 CI | workflow check | `rg -n "report-failure:" .github/workflows/nightly.yml` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/ontoforge-engine/`（Cargo.toml、lib.rs、测试目录）与 workspace 依赖项
- [ ] `crates/ontoforge-schema/tests/bindings.rs` 与 `examples/ontologies/invalid/` 下的绑定 / 类型契约样例目录
- [ ] `crates/ontoforge/tests/cli_online.rs`（tempdir 内写 ontology、绑定与 CSV fixture，经 SQLite 替身运行，不依赖 Docker）与 `crates/ontoforge-engine/tests/bind_check_isolation.rs`（wiremock）
- [ ] `ci/trino/catalog/*.properties`、`ci/pg/seed.sql`、`.github/workflows/nightly.yml`
- [ ] 既有 fixture 的 `backing` 迁移作为首个 plan

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| 真实 Trino / Iceberg 行为 | ENGINE-01, ENGINE-03, ENGINE-05 | 本机无 Docker / Trino，只能在 Linux CI 执行 | 查看 PR CI 的 Trino job 与 nightly Iceberg job 日志，确认 `--ignored` 测试实际运行且通过 |
| Phase 2 完成门禁（eng review E21） | ENGINE-05 | golden 预期值是推导值，只有真实 CI 运行能证实 | PR `integration` 作业与首次 nightly（483、484）都实跑通过，`trino_it` 与 `cli_trino_it` 的 `--ignored` 实际执行数各 ≥ 1；golden 按首跑日志定稿、删除 `PROVISIONAL` 注释并提交后，才可写 `02-VERIFICATION.md` passed |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
