---
phase: 03-edit-store-scope-model
verified: 2026-10-22T02:11:26Z
status: human_needed
score: 8/8 must-haves verified
covered_files:
  - ".github/workflows/ci.yml"
  - ".planning/phases/03-edit-store-scope-model/03-01-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-01-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-02-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-02-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-03-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-03-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-04-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-04-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-05-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-05-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-06-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-06-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-07-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-07-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-08-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-08-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-09-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-09-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-10-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-10-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-11-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-11-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-12-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-12-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-13-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-13-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-14-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-14-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-15-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-15-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-16-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-16-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-17-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-17-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-18-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-18-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-19-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-19-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-20-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-20-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-21-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-21-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-22-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-22-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-23-PLAN.md"
  - ".planning/phases/03-edit-store-scope-model/03-23-SUMMARY.md"
  - ".planning/phases/03-edit-store-scope-model/03-UAT.md"
  - "ci/trino/catalog/editstore.properties"
  - "crates/ontoforge-storage/tests/reload_trino_it.rs"
covered_digest: "v3:sha256:5e18518fc7d4c97e6be70eef32ee6605bb4168875968d5db95a060e6d6e5cd41"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: human_needed
  previous_score: 7/8
  gaps_closed:
    - "Trino metadata cache flush effect is isolated by a red/green integration test"
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "Run PR CI on the commit containing the cache-isolation test, cache-enabled catalog, and serial integration command"
    expected: "test (ubuntu-latest), test (windows-latest), and integration succeed; integration runs all three reload_trino_it cases with metadata caching enabled"
    why_human: "The new changes are still uncommitted locally and have not run in PR CI. CI run 39582728729 at 342247b predates these changes and is not evidence for them."
deferred:
  - truth: "SCOPE-07 read-side request version pinning and MCP mutation-tool exclusion"
    addressed_in: "Phase 6 and Phase 7"
    evidence: "ROADMAP Phase 6 success criterion 5 covers request version pinning; Phase 7 criterion 2 excludes mutation tools."
---

# Phase 3: Edit Store & Scope Model Verification Report

**Phase Goal:** edit store 按 object type 生成强类型表并安全演进，所有写入有不可变 edit log；用户 / 组 / 角色与 scope 规则可配置，同一份 CEL 行谓词既能编译成 SQL 也能在 Rust 中对单行求值且结果一致
**Verified:** 2026-10-22T02:11:26Z
**Status:** human_needed

## Goal Achievement

本次为增量复核。上一版报告的三个代码 gap 已关闭且没有本轮反证；下文保留它们的测试和接线证据。本轮重点检查先前唯一未验证的 Trino 缓存行为，以及 UAT、CI 的接线。ROADMAP 将 Phase 3 标为 `mvp`，但 Goal 不是 User Story 格式；这是既有规划格式问题，本次沿用既有 8 条 truth 复核。

### Previous Gap Closure

下表保留上一轮已完成的代码级验证证据。这些测试在上轮验证时运行通过，本轮没有重复运行未改动的模块。

| Gap | Closure evidence | Test evidence |
|-----|------------------|---------------|
| decimal 经 f64 舍入（CR-A01、STORE-03、STORE-06） | `value.rs` 仅接受整数 JSON number，其余 decimal 要求字符串，`canonical_as` 保持精度；U+0000、非法日期和年份越界也被拒绝 | `value::` 14、`import` 16、`conformance_sqlite` 28、`conformance_pg` 29 通过，含 `12345678.1234567891` 往返和两后端非法值检查 |
| 全新 checkout 的 dev 配置失败（CR-D01、AUTH-02） | `EditStore::open` 创建 SQLite 文件父目录；`DevKey::create` 创建密钥目录，Unix 新目录 mode 0700 | `example_config_reload_works_from_a_fresh_tree`、`example_config_dev_token_works_from_a_fresh_tree`、`open_creates_the_parent_directories_of_the_sqlite_file`、`missing_key_directory_is_created` 通过 |
| migrate 不复核已存 role（WR-B07、SCOPE-06） | `roles_check()` 经 `apply_migration_probed(.., Some(roles))` 到事务内 `load_scope_in` + `compile_scope(new)`，不合法时拒绝迁移 | SQLite `migrate_rechecks_applied_roles`、`migrate_with_roles_applies_when_the_roles_still_compile`、CLI `migrate_refuses_a_change_that_breaks_an_applied_role` 和 PostgreSQL `pg_migrate_rechecks_applied_roles` 通过 |
| 整对象读取绕过 ONT-V23（WR-C05、SCOPE-06） | `coverage::problem()` 对 `objects.<param>` 要求所有属性均在 allow-list；缺少类型信息时 fail closed | `criteria_coverage` 6 通过，含 `whole_object_read_needs_every_property`；`docs/scope.md` 已同步 |
| Trino metadata cache flush 效果未隔离 | `NoFlush` 与真实 `describe` 的对照用例，加上 10m metadata cache 和串行 CI 接线 | 无缓存时红测 exit 1；缓存开启时目标用例 1/1、整文件 3/3 通过，见 `03-UAT.md` |

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC1: 强类型 edit 表、DDL 演进、破坏性变更迁移、失败保留旧定义 | VERIFIED | 前次实测 `reload`、`migrate`、CLI 测试；本轮检查 `reload.rs` 的 pending、probe、publish 顺序和新集成用例。 |
| 2 | SC2: SQLite/PostgreSQL conformance，log/history append-only | VERIFIED | 前次 SQLite 28、embedded PostgreSQL 29 个用例通过；相关代码本轮未改。 |
| 3 | SC3: patch 叠加语义经参考模型 proptest 覆盖 | VERIFIED | 前次 `model_prop` 通过；相关代码本轮未改。 |
| 4 | SC4: role、allow-list、CEL、action/toolset、多 role 合并不泄露 | VERIFIED | 前次 `scope_compile` 21、`scope_merge` 9、`rowpred_eval` 14、`rowpred_subset` 13 通过；相关代码本轮未改。 |
| 5 | SC5: CEL SQL/Rust 差分、criteria lint、OIDC/dev token | VERIFIED | 前次差分、lint、OIDC/dev token 用例通过；相关代码本轮未改。 |
| 6 | 全新 checkout 的 dev 配置可运行 | VERIFIED | 前次 CLI 全新目录集成用例通过；相关代码本轮未改。 |
| 7 | decimal 不经 f64 舍入写入行、log、history | VERIFIED | 前次单元、import、SQLite/PostgreSQL conformance 通过；相关代码本轮未改。 |
| 8 | STORE-02: 开启 Trino metadata cache 时，新列须在 flush 后可见才能切换定义 | VERIFIED | `cached_column_requires_flush_before_definition_switch` 预热 v1 表，再用 `NoFlush` 保持真实 `describe`、只跳过 flush；断言 `NotVisible(of_obj_flight.gate)` 与 active v1，真实 flush 后断言 v2 active 且 Trino 能读 `gate`。独立 Trino 483/PostgreSQL 本地红测在无缓存时失败（`NoFlush` 意外允许 v2），启用 `metadata.cache-ttl=10m`、`metadata.cache-missing=true` 后绿测 1/1，全文件串行 3/3。 |

**Score:** 8/8 truths verified; 0 behavior-unverified。第 8 条的本地运行结果由执行单元记录在 `03-UAT.md`，原始终端日志未持久化；本次 verifier 没有并发重跑共享 catalog 的 flush 测试。

### Review Findings And Residual Risks

上一轮代码审查中 6 条定向问题（CR-A01、WR-A01、WR-A03、WR-B07、WR-C05、CR-D01）已有代码和测试证据支持关闭。仍适用的风险如下，不把它们误写成已解决。

| Finding | Decision | Evidence / follow-up |
|---------|----------|----------------------|
| WR-01：migrate 的 role 复核与 `scope apply` 口径互斥 | Advisory | 对被 role 引用的属性重命名需先撤引用、迁移、再加回；当前路径 fail closed，文档仍应明确这一步。 |
| WR-02：dev 签名密钥的文件级保护不足 | Advisory | `--dev` 才接受 dev token；`docs/edit-store.md` 已列为已知缺口 7。Unix 目录 mode 0700 不能替代既有文件权限或符号链接检查。 |
| CR-B01：CI OpenSSL 守卫首行在 `errexit` 下不会失败 | Advisory | `.github/workflows/ci.yml:32-34` 的 `! cargo tree` 仍有该缺陷；上轮 `cargo tree -i openssl` / `-i native-tls` 都返回 "did not match any packages"。下次改依赖前应修守卫。 |
| CR-D02：server 配置解析错误可能回显被拒绝的值 | Advisory | `config.rs` 的值回显路径未改；连接串敏感字段已有遮蔽，但误贴到其他字段仍可暴露。下次触碰配置解析时应修。 |
| IN-01、IN-04 | Advisory | `value.rs` 的 decimal 拒绝文案存在 18 个连续空格；公共 `apply_migration` 默认不复核 role，migrate 也不为新列补 role 索引。 |
| 其余遗留 warning（WR-A02/A04/A05、B01..B06、B08..B10、C02..C04、C06/C07、D02..D06） | Advisory | 前次逐条判为加固项；其中 WR-B03、WR-B10 的 hash 失败降为空串与 WR-C04 的缺失行属性按 Null 求值应在 Phase 4/5 前优先处理。 |

### Required Artifacts And Wiring

| Artifact / link | Status | Evidence |
|-----------------|--------|----------|
| `ci/trino/catalog/editstore.properties` | VERIFIED | 第 12-13 行开启 10m TTL 和 missing metadata 缓存；CI Trino 483 挂载该目录。 |
| `reload_trino_it.rs` | VERIFIED | `NoFlush` 只覆写 `flush_metadata`，`describe` 委托给真实 `AnyEngine`；新用例预热、失败保持旧定义、真实 flush 切换并读取新列。三个用例均 `#[ignore]`，CI 用 `--ignored` 执行。 |
| `.github/workflows/ci.yml` -> `reload_trino_it` | WIRED | `Reload visibility probe` 设置 Trino/PG URL，运行 `--ignored --test-threads=1`；串行避免一个测试的 catalog-wide flush 清掉另一个测试的缓存。 |
| `reload.rs` -> Trino -> PostgreSQL edit 表 | WIRED / FLOWING | `wait_until_visible` 先 `flush_metadata`，再 `describe`、比对真实表列；不足时返回 `NotVisible`。`publish` 成功后才使 pending 版本 active。测试使用独立 PG schema 和真实 Trino 查询。 |
| `03-UAT.md` | VERIFIED | 两项 UAT 均记录通过；第 1 项准确区分本地红绿与新改动尚未跑 PR CI。 |

其余 Phase 3 交付物与接线沿用上一轮验证的逐项结果：

| Artifact / link | Status | Evidence |
|-----------------|--------|----------|
| `crates/ontoforge-storage/src/{value,commit,store,migrate,reload,...}.rs` | VERIFIED | edit store、DDL、迁移、reload、decimal 的实现均被对应测试覆盖；无占位实现。 |
| `crates/ontoforge-auth/src/scope/coverage.rs` | VERIFIED | 整对象读取分支检查全部属性，`whole_object_read_needs_every_property` 通过。 |
| `crates/ontoforge-auth/src/authn/dev.rs` | VERIFIED | `create_key_dir` 创建目录，CLI 全新目录测试通过。 |
| `crates/ontoforge/src/migrate.rs` | WIRED | CLI `roles_check()` 传入 `apply_migration_probed(.., Some(roles))`，后者在事务内 `load_scope_in` + `compile_scope`；SQLite、CLI、PostgreSQL 测试通过。 |
| `reload` -> 已存 role | WIRED | `ReloadOptions::roles`，`reload_rechecks_applied_roles` 通过。 |
| JSON number -> decimal 列 / log / history | WIRED / FLOWING | `from_json_as` 仅允许精确整数 number；`PropertyValue::Decimal` 经 `canonical_as` 写入真实行、log、历史，两个后端 conformance 逐字比较。 |
| `open_postgres` -> 会话设置 | WIRED | `pg_connect_options` 固定 `DateStyle`、`TimeZone`、`extra_float_digits`，`pg_sessions_use_the_pinned_settings` 通过。 |
| `commit()` -> def_version 栅栏、import、history 掩码、scope 审计 | WIRED | 前轮 `history_masking` 12、`scope_store` 11、`import` 16 通过。 |
| `examples/server/ontoforge-server.dev.toml`、`docs/edit-store.md`、`docs/scope.md` | VERIFIED | 示例配置在全新树可用；文档覆盖整对象读取、migrate 复核和 dev token 已知缺口。 |

### Behavioral Spot-Checks And Test Quality

| Check | Result | Status |
|-------|--------|--------|
| 无缓存 catalog 的新单测 | `NoFlush` 返回 v2，测试 exit 1；证明测试可识别未缓存环境 | RED（预期） |
| 缓存开启后的单测 | 1/1 通过，v1 保持 active 后真实 flush 激活 v2 | PASS（执行单元本地） |
| `cargo test -p ontoforge-storage --test reload_trino_it --locked -- --ignored --test-threads=1` | 3/3 通过 | PASS（执行单元本地） |
| `cargo fmt --all --check`、目标 Clippy、`git diff --check` | 均通过 | PASS（执行单元本地） |
| PR CI 对当前未提交变更 | 尚未运行；旧 run 39582728729 只证明 342247b | PENDING |

`#[ignore]` 用于要求外部 Trino/PG 的测试，CI 显式运行并检查用例数量；它不是失效的 requirement 测试。新测试的 expected 不由被测 `reload` 生成：v1/v2 和 `gate` 可见性是明确断言。修改文件没有未引用的 `TBD`、`FIXME`、`XXX` 标记。没有 Phase 3 声明的 probe shell 脚本。

前轮验证的测试矩阵如下。这些结果证明此前代码版本；本轮 Trino 缓存改动的证据单独列在上表，旧 CI 不用于证明新改动。

| Behavior | Command / target | Prior result | Status |
|----------|------------------|--------------|--------|
| 工作区全量回归 | `cargo test --workspace --locked --no-fail-fast` | 84 suites，984 passed，0 failed，67 ignored；当时由 orchestrator 在 HEAD 执行 | PASS（前轮引用） |
| decimal 单元 | `cargo test -p ontoforge-storage --lib value::` | 14 passed | PASS（前轮） |
| SQLite 往返、import、migrate、reload、连接 | `conformance_sqlite`、`import`、`migrate`、`reload`、`sqlite_connection` | 28 + 16 + 15 + 32 + 4 passed | PASS（前轮） |
| 覆盖 lint 与 dev token | `criteria_coverage`、`dev_token` | 6 + 16 passed | PASS（前轮） |
| CLI 全新树与 migrate 复核 | `cli_reload`、`cli_dev_token`、`cli_migrate`、`cli_scope` | 8 + 12 + 10 + 8 passed | PASS（前轮） |
| embedded PostgreSQL | `conformance_pg`、`migrate_pg`（`--features embedded-pg -- --ignored`） | 29 + 4 passed | PASS（前轮） |
| 差分、合并、编译、OIDC、叠加 proptest、掩码、scope 审计 | `rowpred_diff` 6、`scope_merge` 9、`scope_compile` 21、`oidc` 31、`model_prop` 1、`history_masking` 12、`scope_store` 11，另有 `ontoforge-expr` | 全部通过，0 failed | PASS（前轮） |
| 禁止 OpenSSL / native-tls | `cargo tree --workspace --locked --features ontoforge-storage/embedded-pg -e normal,build,dev -i openssl`，另跑 `-i native-tls` | 两者 "did not match any packages" | PASS（前轮） |

Probe Execution：Phase 3 PLAN / SUMMARY 未声明 `scripts/*/tests/probe-*.sh`，本 gate 不适用。

### Requirements Coverage

ROADMAP Phase 3 的 16 个 requirement ID 均由 03-01..03-23 PLAN 声明，没有 orphan。STORE-02 的 Trino 缓存分支本轮已得到本地行为证据。STORE-01、03..07、SCOPE-01..06、AUTH-01..02 维持上一版代码和测试证据；SCOPE-07 的读侧和 MCP 剩余范围由 Phase 6/7 明确承接。`REQUIREMENTS.md` 的若干 checkbox / traceability 状态仍滞后于验证结果，应由 orchestrator 对账。

| Requirement | Source Plans | REQUIREMENTS.md now | Verdict | Evidence |
|-------------|--------------|---------------------|---------|----------|
| STORE-01 | 03-01, 03-03, 03-04, 03-08, 03-09, 03-13, 03-20, 03-21 | ticked | SATISFIED | SQLite/PostgreSQL 同一 conformance；PG 值域校验也进入共享用例。 |
| STORE-02 | 03-02, 03-03, 03-04, 03-10, 03-15, 03-18, 03-20 | unticked | SATISFIED locally; CI pending | 自动 DDL、ONT-V25、migrate 归档、栅栏、`lock_timeout`、pending/probe/active 均有测试；本轮缓存隔离红绿与 3/3 实测关闭第 8 条 truth，当前改动的 PR CI 尚待运行。 |
| STORE-03 | 03-08, 03-09, 03-21 | ticked | SATISFIED | 每次提交一条不可变 log；追加专用触发器；decimal 保真。 |
| STORE-04 | 03-17 | unticked | SATISFIED | `EditStore::history` 带 D-13 掩码，`history_masking` 12 通过；REST/MCP 外壳属 Phase 7。 |
| STORE-05 | 03-10, 03-13 | unticked | SATISFIED | 一对一部分唯一索引、`CardinalityViolation`、PG `concurrent_fk_claims_one_wins`。 |
| STORE-06 | 03-19, 03-21 | ticked | SATISFIED | `ontoforge import` 作为 `system:import` 提交；JSON/CSV decimal 逐字保存。 |
| STORE-07 | 03-08, 03-13 | unticked，traceability 仍为 Gaps Found | SATISFIED | `model_prop` 与 conformance 转换用例通过。 |
| SCOPE-01 | 03-06, 03-07, 03-14, 03-16 | unticked | SATISFIED | `of_user`、`of_role`、`of_role_binding`、`scope plan/apply`、JWT `groups`；`cli_scope` 8、`scope_store` 11。 |
| SCOPE-02 | 03-02, 03-05, 03-06, 03-11, 03-14, 03-16 | unticked | SATISFIED | role schema 与编译快照测试。 |
| SCOPE-03 | 03-11 | unticked | SATISFIED | `merge.rs` 与 `scope_merge` 9。 |
| SCOPE-04 | 03-05, 03-11, 03-20 | unticked，traceability 仍为 Gaps Found | SATISFIED | Rust 对 SQLite 本地 6 通过；Rust 对 Trino 483 的旧 CI 3 通过，相关代码本轮未改。 |
| SCOPE-05 | 03-05, 03-11, 03-14 | unticked | SATISFIED | 无授权不可见、缺失属性为 Unknown、不可编译谓词在 plan/apply/reload 为 ONT-V22。 |
| SCOPE-06 | 03-14, 03-16, 03-18, 03-22 | ticked | SATISFIED | reload、scope apply、migrate --apply 三处复核；整对象读取已拦。 |
| SCOPE-07 | 03-16, 03-18 | unticked | PARTIAL; remainder deferred | 请求固定定义版本（读侧）与 MCP 无变更工具属 Phase 6/7；Phase 3 完成写侧栅栏与 scope 审计。 |
| AUTH-01 | 03-01, 03-02, 03-07 | unticked，traceability 仍为 Gaps Found | SATISFIED | `oidc` 31 通过。 |
| AUTH-02 | 03-12, 03-23 | ticked | SATISFIED | 全新树上的 `dev-token` 与 `reload` 测试通过；旧 CI run 39582728729 在 342247b 上另验证 Unix 0700 断言。 |

上表 checkbox 与 traceability 是当前文件内容，不等同于验收结论。REQUIREMENTS.md 的对账由 orchestrator 处理；本 verifier 没有修改该文件。

### Anti-Patterns And Regression Check

| File | Location | Finding | Severity | Impact |
|------|----------|---------|----------|--------|
| `crates/ontoforge-storage/src/value.rs` | 341 | 错误文案中有 18 个连续空格 | Info | 只影响 ONT-V27 诊断可读性（IN-01）。 |
| `.github/workflows/ci.yml` | 32-34 | `! cmd` 在 `errexit` 下不能让首行失败 | Warning | OpenSSL 供应链守卫可能失效；当前依赖树检查结果仍为无匹配（CR-B01）。 |
| `crates/ontoforge-engine/src/config.rs` | 397-439 附近 | 解析错误回显被拒绝的值 | Warning | 密钥误贴进类型化字段可能被打印（CR-D02）。 |
| `crates/ontoforge-auth/src/authn/dev.rs` | 106-130 附近 | 不校验既有密钥文件权限或符号链接 | Warning | dev-only 密钥仍需加固（WR-02）。 |

前轮对阶段触及文件扫描没有未引用的 `TBD`、`FIXME`、`XXX`；本轮修改的测试、catalog、CI 文件也没有。第一次验证已通过的 SC2 conformance、SC3 proptest、SC4 scope 合并、SC5 差分/OIDC、历史掩码、import 和 scope 审计未被本轮修改；新 Trino 测试的 3/3 本地结果提供本轮唯一新增行为回归证据。工作区全量测试和旧 CI 均早于本轮未提交变更，不作本轮全量绿测声明。

### Decision Coverage

`check.decision-coverage-verify` 检查了 `03-CONTEXT.md` 的 18 条可跟踪决策：18/18 honored，`not_honored` 为空。该 gate 非阻塞。

### Human Verification Required

当前代码、catalog、CI 命令改动尚未跑 PR CI。需要把这些改动纳入同一个提交并确认 `test (ubuntu-latest)`、`test (windows-latest)`、`integration` 在该提交上通过，尤其要确认缓存开启的 `reload_trino_it` 三个用例。旧 CI run 39582728729 的通过不能替代这一步。

### Gaps Summary

没有代码层面的剩余 gap，也没有新回归证据。状态保留为 `human_needed`，仅因当前改动的跨平台 PR CI 尚未验证；CI 通过后可再将阶段状态转为 `passed`。

---

_Verified: 2026-10-22T02:11:26Z_
_Verifier: gsd-verifier_
