---
phase: 03-edit-store-scope-model
verified: 2026-10-21T18:30:00Z
status: human_needed
score: 7/8 must-haves verified
covered_files:
  - ".github/workflows/ci.yml"
  - ".planning/REQUIREMENTS.md"
  - ".planning/adr/0023-phase3-crate-dependency-direction.md"
  - ".planning/adr/0024-edit-store-physical-format.md"
  - ".planning/adr/0025-row-predicate-semantics.md"
  - ".planning/adr/0026-edit-log-format.md"
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
  - "CONTRIBUTING.md"
  - "Cargo.lock"
  - "Cargo.toml"
  - "ci/pg/rowpred_corpus.sql"
  - "ci/trino/catalog/editstore.properties"
  - "crates/ontoforge-auth/Cargo.toml"
  - "crates/ontoforge-auth/src/authn/dev.rs"
  - "crates/ontoforge-auth/src/authn/jwks.rs"
  - "crates/ontoforge-auth/src/authn/mod.rs"
  - "crates/ontoforge-auth/src/authn/oidc.rs"
  - "crates/ontoforge-auth/src/authn/principal.rs"
  - "crates/ontoforge-auth/src/lib.rs"
  - "crates/ontoforge-auth/src/scope/compile.rs"
  - "crates/ontoforge-auth/src/scope/coverage.rs"
  - "crates/ontoforge-auth/src/scope/lower.rs"
  - "crates/ontoforge-auth/src/scope/merge.rs"
  - "crates/ontoforge-auth/src/scope/mod.rs"
  - "crates/ontoforge-auth/tests/criteria_coverage.rs"
  - "crates/ontoforge-auth/tests/crypto_backend.rs"
  - "crates/ontoforge-auth/tests/data/README.md"
  - "crates/ontoforge-auth/tests/data/oidc_es256.pem"
  - "crates/ontoforge-auth/tests/data/oidc_rs256.pem"
  - "crates/ontoforge-auth/tests/data/rowpred_corpus.json"
  - "crates/ontoforge-auth/tests/dev_token.rs"
  - "crates/ontoforge-auth/tests/oidc.rs"
  - "crates/ontoforge-auth/tests/rowpred_common/mod.rs"
  - "crates/ontoforge-auth/tests/rowpred_diff.rs"
  - "crates/ontoforge-auth/tests/rowpred_trino_it.rs"
  - "crates/ontoforge-auth/tests/scope_compile.rs"
  - "crates/ontoforge-auth/tests/scope_merge.rs"
  - "crates/ontoforge-auth/tests/snapshots/scope_compile__scope_compile_diagnostics_json.snap"
  - "crates/ontoforge-engine/src/config.rs"
  - "crates/ontoforge-engine/src/printer/ast.rs"
  - "crates/ontoforge-engine/src/printer/call.rs"
  - "crates/ontoforge-engine/src/printer/dialect.rs"
  - "crates/ontoforge-engine/src/printer/escape.rs"
  - "crates/ontoforge-engine/src/printer/mod.rs"
  - "crates/ontoforge-engine/src/printer/sqlite_ddl.rs"
  - "crates/ontoforge-engine/src/printer/tests.rs"
  - "crates/ontoforge-engine/src/sqlite/fixture.rs"
  - "crates/ontoforge-engine/src/sqlite/mod.rs"
  - "crates/ontoforge-engine/tests/printer_prop.rs"
  - "crates/ontoforge-engine/tests/server_config.rs"
  - "crates/ontoforge-engine/tests/sql_text_only_from_printer.rs"
  - "crates/ontoforge-engine/tests/trino_client.rs"
  - "crates/ontoforge-expr/Cargo.toml"
  - "crates/ontoforge-expr/src/lib.rs"
  - "crates/ontoforge-expr/src/rowpred/eval.rs"
  - "crates/ontoforge-expr/src/rowpred/ir.rs"
  - "crates/ontoforge-expr/src/rowpred/mod.rs"
  - "crates/ontoforge-expr/src/rowpred/parse.rs"
  - "crates/ontoforge-expr/src/rowpred/value.rs"
  - "crates/ontoforge-expr/tests/common/mod.rs"
  - "crates/ontoforge-expr/tests/rowpred_eval.proptest-regressions"
  - "crates/ontoforge-expr/tests/rowpred_eval.rs"
  - "crates/ontoforge-expr/tests/rowpred_subset.rs"
  - "crates/ontoforge-schema/src/diag.rs"
  - "crates/ontoforge-schema/src/ir/attrs.rs"
  - "crates/ontoforge-schema/src/ir/mod.rs"
  - "crates/ontoforge-schema/src/ir/names.rs"
  - "crates/ontoforge-schema/src/ir/object.rs"
  - "crates/ontoforge-schema/src/ir/scope.rs"
  - "crates/ontoforge-schema/src/lib.rs"
  - "crates/ontoforge-schema/src/loader/discover.rs"
  - "crates/ontoforge-schema/src/loader/dispatch.rs"
  - "crates/ontoforge-schema/src/loader/mod.rs"
  - "crates/ontoforge-schema/src/loader/scope.rs"
  - "crates/ontoforge-schema/src/loader/sourcemap.rs"
  - "crates/ontoforge-schema/src/schema_export.rs"
  - "crates/ontoforge-schema/tests/object_types.rs"
  - "crates/ontoforge-schema/tests/schema_export.rs"
  - "crates/ontoforge-schema/tests/scope_documents.rs"
  - "crates/ontoforge-schema/tests/snapshots/scope_documents__scope_diagnostics.snap"
  - "crates/ontoforge-storage/Cargo.toml"
  - "crates/ontoforge-storage/migrations/postgres/0001_definitions.sql"
  - "crates/ontoforge-storage/migrations/postgres/0002_edit_log.sql"
  - "crates/ontoforge-storage/migrations/postgres/0003_scope.sql"
  - "crates/ontoforge-storage/migrations/sqlite/0001_definitions.sql"
  - "crates/ontoforge-storage/migrations/sqlite/0002_edit_log.sql"
  - "crates/ontoforge-storage/migrations/sqlite/0003_scope.sql"
  - "crates/ontoforge-storage/src/commit.rs"
  - "crates/ontoforge-storage/src/ddl.rs"
  - "crates/ontoforge-storage/src/diff.rs"
  - "crates/ontoforge-storage/src/error.rs"
  - "crates/ontoforge-storage/src/history.rs"
  - "crates/ontoforge-storage/src/import.rs"
  - "crates/ontoforge-storage/src/index.rs"
  - "crates/ontoforge-storage/src/lib.rs"
  - "crates/ontoforge-storage/src/log.rs"
  - "crates/ontoforge-storage/src/migrate.rs"
  - "crates/ontoforge-storage/src/naming.rs"
  - "crates/ontoforge-storage/src/physical.rs"
  - "crates/ontoforge-storage/src/reload.rs"
  - "crates/ontoforge-storage/src/scope_store.rs"
  - "crates/ontoforge-storage/src/snapshots/ontoforge_storage__ddl__tests__ddl_postgres.snap"
  - "crates/ontoforge-storage/src/snapshots/ontoforge_storage__ddl__tests__ddl_sqlite.snap"
  - "crates/ontoforge-storage/src/sql_exit.rs"
  - "crates/ontoforge-storage/src/state.rs"
  - "crates/ontoforge-storage/src/store.rs"
  - "crates/ontoforge-storage/src/value.rs"
  - "crates/ontoforge-storage/tests/conformance/mod.rs"
  - "crates/ontoforge-storage/tests/conformance_pg.rs"
  - "crates/ontoforge-storage/tests/conformance_sqlite.rs"
  - "crates/ontoforge-storage/tests/history_masking.rs"
  - "crates/ontoforge-storage/tests/import.rs"
  - "crates/ontoforge-storage/tests/migrate.rs"
  - "crates/ontoforge-storage/tests/migrate_pg.rs"
  - "crates/ontoforge-storage/tests/model/mod.rs"
  - "crates/ontoforge-storage/tests/model_prop.proptest-regressions"
  - "crates/ontoforge-storage/tests/model_prop.rs"
  - "crates/ontoforge-storage/tests/reload.rs"
  - "crates/ontoforge-storage/tests/reload_pg.rs"
  - "crates/ontoforge-storage/tests/reload_trino_it.rs"
  - "crates/ontoforge-storage/tests/scope_store.rs"
  - "crates/ontoforge-storage/tests/sql_exit_guard.rs"
  - "crates/ontoforge-storage/tests/sqlite_connection.rs"
  - "crates/ontoforge-storage/tests/support/mod.rs"
  - "crates/ontoforge/Cargo.toml"
  - "crates/ontoforge/src/cli.rs"
  - "crates/ontoforge/src/dev_token.rs"
  - "crates/ontoforge/src/import.rs"
  - "crates/ontoforge/src/lib.rs"
  - "crates/ontoforge/src/migrate.rs"
  - "crates/ontoforge/src/online.rs"
  - "crates/ontoforge/src/reload.rs"
  - "crates/ontoforge/src/scope.rs"
  - "crates/ontoforge/tests/cli_dev_token.rs"
  - "crates/ontoforge/tests/cli_import.rs"
  - "crates/ontoforge/tests/cli_migrate.rs"
  - "crates/ontoforge/tests/cli_reload.rs"
  - "crates/ontoforge/tests/cli_schema.rs"
  - "crates/ontoforge/tests/cli_scope.rs"
  - "docs/designs/trino-engine-sql-tiers.md"
  - "docs/edit-store.md"
  - "docs/scope.md"
  - "examples/data/aviation/airports.csv"
  - "examples/data/aviation/airports.json"
  - "examples/ontologies/aviation-json/bindings/dev/bindings.json"
  - "examples/ontologies/aviation-json/objects.json"
  - "examples/ontologies/aviation/bindings/dev/flight.yaml"
  - "examples/ontologies/aviation/objects.yaml"
  - "examples/scopes/aviation/bindings.yaml"
  - "examples/scopes/aviation/roles.yaml"
  - "examples/server/ontoforge-server.dev.toml"
  - "schemas/ontoforge-definition.schema.json"
  - "schemas/ontoforge-scope.schema.json"
covered_digest: "v1:sha256:33a2fede65ce18b629e0d62407da40f7dd8a2506687433bdb883745f394b5eb4"
behavior_unverified: 1
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 4/8
  gaps_closed:
    - "gap 1: decimal rounded through f64 (CR-A01), closed by 03-21"
    - "gap 2: dev quickstart fails on a fresh checkout (CR-D01), closed by 03-23"
    - "gap 3: criteria coverage holes, migrate path (WR-B07) and bare objects.<param> read (WR-C05), closed by 03-22"
  gaps_remaining: []
  regressions: []
behavior_unverified_items:
  - truth: "Reload makes new tables/columns visible in Trino before the new definition becomes active, including when Trino's JDBC metadata cache is on (STORE-02, Trino flush step)"
    test: "Run reload_trino_it against a Trino catalog with metadata.cache-ttl=10m and metadata.cache-missing=true (ci/trino/catalog/editstore.properties still has neither), looking at a table first, then adding a column, then reloading; once as is, once with the flush_metadata_cache call removed from the probe"
    expected: "Passes with the flush, fails without it"
    why_human: "The green CI run cannot tell 'the flush worked' from 'nothing was cached': the CI catalog has the default metadata.cache-ttl=0s and a failing flush is tolerated by the probe. Unchanged since the first verification (ci/ and reload_trino_it.rs have no diff between 47364cf and HEAD). Needs a Trino with a non-default catalog setting."
human_verification:
  - test: "Cache-flush isolation on Trino (see behavior_unverified_items)"
    expected: "reload_trino_it fails when system.flush_metadata_cache() is skipped and passes when it runs, on a catalog with metadata caching enabled"
    why_human: "Needs a Trino with a cache-enabled catalog; one-line CI catalog change plus a look-first-then-add-column step"
  - test: "Push the 29 local commits (gap closure 03-21..03-23 and the review) and let the ci workflow run on Linux"
    expected: "All three jobs green; in particular the cfg(unix) assertion in crates/ontoforge-auth/tests/dev_token.rs#missing_key_directory_is_created (new key directory has mode 0700) runs and passes, and the Linux-only PostgreSQL/Trino jobs pass at the new HEAD"
    why_human: "The last CI run (39543562659) is on 4684ae7, before the gap commits. The Unix 0700 branch is not compiled on Windows. I ran the PostgreSQL suites locally on embedded PostgreSQL (see Spot-Checks), so the PostgreSQL side is covered; the Unix branch and the Trino jobs are not. The 0700 mode is hardening, not part of any Success Criterion."
deferred:
  - truth: "SCOPE-07 remainder: every request uses the definition version it started with (read side); agent MCP exposes no mutation tool; role/scope changes reach only the administration channel over REST/MCP"
    addressed_in: "Phase 6 and Phase 7"
    evidence: "ROADMAP Phase 6 success criterion 5: '定义 reload 先 validate / lint，失败不切换；每个请求固定使用开始时的定义版本'; Phase 7 criterion 2: 'L3 与任何变更工具不出现'. Phase 3 delivered the write-side fence, scope_store as the only writer of of_role / of_role_binding (source guard test) and the append-only of_admin_audit."
---

# Phase 3: Edit Store & Scope Model Verification Report

**Phase Goal:** edit store 按 object type 生成强类型表并安全演进，所有写入有不可变 edit log；用户 / 组 / 角色与 scope 规则可配置，同一份 CEL 行谓词既能编译成 SQL 也能在 Rust 中对单行求值且结果一致
**Verified:** 2026-10-21T18:30:00Z
**Status:** human_needed
**Re-verification:** Yes, after gap closure plans 03-21, 03-22, 03-23 (previous report: `git show 47364cf:.planning/phases/03-edit-store-scope-model/03-VERIFICATION.md`, status gaps_found, 4/8)
**Mode:** ROADMAP marks the phase `mvp`, but the goal line is not a user story; the standard goal-backward method was used (MVP user-story guard not applied), same as the first verification.

## Verdict

三个 gap 全部关闭，没有回归，没有新的 blocker。五条 Success Criterion 都有通过的测试或我自己的运行作证。状态是 `human_needed` 而不是 `passed`，因为仍有两项人工确认：

1. Trino 元数据缓存 flush 的效果仍没有被任何测试隔离（第一次验证就记下的 present-but-behavior-unverified 项，三个 gap 计划都没有碰 `ci/` 与 `reload_trino_it.rs`）。
2. 三个 gap 计划的 29 个提交还没有推送，最近一次 CI（39543562659）在它们之前。`cfg(unix)` 下的 0700 目录权限断言本机 Windows 不编译，只能由 Linux CI 确认。这一项是纵深防御，不属于任何 Success Criterion。

## Re-check of every previous gap

| # | Previous gap (gaps[] entry in 47364cf) | Closed? | Evidence I collected myself |
|---|---|---|---|
| 1 | decimal 经 f64 舍入，写进 edit 行 / 不可变 log / 历史 (CR-A01, STORE-03, STORE-06) | **Closed** | `crates/ontoforge-storage/src/value.rs:339` 现在是 `Value::Number(n) if n.is_i64() \|\| n.is_u64()`，其余数字走 `fail(...)`（"send a decimal as a string such as \"12.50\""）。`cargo test -p ontoforge-storage --lib value::` 14 通过，含 `decimal_json_numbers_must_be_exact_integers`。`--test import` 16 通过，含 `json_decimals_are_never_rounded`。`--test conformance_sqlite` 28 通过，含带 `12345678.1234567891`（decimal(38,10)）的 `every_value_type_round_trips`。**PostgreSQL 侧**：我用 `--features embedded-pg -- --ignored` 本机跑了 `conformance_pg` 29 通过（同一用例）。附带的 WR-A01 也关闭：`value.rs:179` 拒绝 U+0000，`:271` 拒绝 0 年 date，`:306` 限定 UTC 年份 1..9999，`commit.rs:301-316` 的 `check_shape` 拒绝 action / actor / params 里的 NUL；`values_postgres_cannot_store_are_refused_on_every_backend` 在两个后端都通过。 |
| 2 | 示例 dev 配置在全新检出上失败 (CR-D01, AUTH-02) | **Closed** | `EditStore::open` 的 SQLite 分支在连接前 `create_dir_all(parent)`（`store.rs:191`）；`DevKey::create` 调 `create_key_dir`（`dev.rs`，Unix `DirBuilder.mode(0o700)`，其他平台 `create_dir_all`）。CLI 测试 `example_config_reload_works_from_a_fresh_tree`（`cli_reload.rs:304`）与 `example_config_dev_token_works_from_a_fresh_tree`（`cli_dev_token.rs:225`）把真实 `examples/server/ontoforge-server.dev.toml` 复制进一个空临时树，断言 `target/` 不存在，然后 `reload` 退出码 0、写出 `editstore.db`、紧接着 `scope plan` 成功、`dev-token` 退出码 0 且创建 32 字节的 `dev-token.key`。两个测试以及 `sqlite_connection::open_creates_the_parent_directories_of_the_sqlite_file`、`dev_token::missing_key_directory_is_created` 我本机都跑过，通过。 |
| 3a | `ontoforge migrate` 不复核已存 role (WR-B07, SCOPE-06) | **Closed** | `apply_migration_probed` 新增 `roles: Option<&RolesCheck>`（`storage/src/migrate.rs:365`）；`apply_in` 在 `lock_def_state`、`check_hash`、空计划提前返回之后、`run` 之前 `load_scope_in` + `compile_scope(new)`，失败映射为 `MigrateError::Definition`（`:403-414`）。CLI 的 `roles_check()` 构造 `RolesCheck`，`execute()` 对 `--apply` 传 `Some(roles)`（`cli/migrate.rs:150-176`）。测试 `migrate_rechecks_applied_roles`、`migrate_with_roles_applies_when_the_roles_still_compile`、CLI `migrate_refuses_a_change_that_breaks_an_applied_role` 在 SQLite 通过，`pg_migrate_rechecks_applied_roles` 在 embedded PostgreSQL 通过。 |
| 3b | 整对象读取 `objects.<param>` 绕过 ONT-V23 (WR-C05, SCOPE-06) | **Closed** | `coverage.rs` 的 `problem()` 在 `path.segments.get(2)` 为 `None` 时不再短路返回，而是要求 allow-list 覆盖该对象类型的全部属性，缺哪个就报 ONT-V23 并列出缺失属性；类型缺失按 fail-closed 走 `undetermined()`。`criteria_coverage` 6 通过，含 `whole_object_read_needs_every_property`。`docs/scope.md` 的「apply 前的离线校验」与「定义变更时的复核」已同步。 |
| - | Human item: Trino 缓存 flush 未隔离 | **Still open** | `git diff --stat 47364cf HEAD -- ci/ .github/ .../reload_trino_it.rs` 无输出。仍在 `human_verification`。 |

## Goal Achievement

### Observable Truths

ROADMAP Success Criteria 为合同；第 6 至 8 条沿用第一次验证的补充 truth。

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC1: 加载示例 ontology 生成每个 object type 与 M:N link 的 edit 表；新增类变更在 reload 自动执行；破坏性变更被拦（ONT-V25），`migrate --plan/--apply` 完成并归档旧列；reload 失败保留旧定义 | VERIFIED | 第一次验证已用构建出的二进制端到端跑过 reload / ONT-V25 / migrate plan+apply+hash 守卫。本轮回归：`reload` 32、`migrate` 15、`cli_reload` 8、`cli_migrate` 10 全部通过。第一次验证里"只有预先建好 `target/ontoforge-dev/` 才能跑"的限制已由 gap 2 的关闭解除。 |
| 2 | SC2: 同一 conformance 套件在 SQLite 与 PostgreSQL 上通过；edit log 与历史表只能追加 | VERIFIED | `conformance_sqlite` 28 通过；`conformance_pg` 29 通过（我本机 embedded PostgreSQL 实跑，含 `append_only_rejects_update_and_delete`、`append_only_rejects_truncate_on_pg`、`pg_sessions_use_the_pinned_settings`）。上一轮 CI 在 PG 17 上 27/27，本轮多出 2 个由 03-21 新增的共享 / PG 用例。 |
| 3 | SC3: 叠加语义（set_mask、tombstone、release、版本单调）由内存参考模型的 proptest 覆盖 | VERIFIED | `model_prop` `model_matches_store` 通过（12.6s），参考模型在 `tests/model/mod.rs`，回归文件已提交。 |
| 4 | SC4: 角色授予 object type、属性 allow-list、CEL 行谓词（同行属性与 `principal.attrs`）、action、SQL 级别、toolset；缺失属性为 false；多 role 合并无"A 的列 x B 的行"泄露 | VERIFIED | `scope_compile` 21、`scope_merge` 9（含 `a_columns_times_b_rows_do_not_leak` 与 oracle 属性测试）、`rowpred_eval` 14、`rowpred_subset` 13、`history_masking` 12 全部通过。 |
| 5 | SC5: 同一 CEL 行谓词的 SQL 编译与 Rust 单行求值经差分测试一致；criteria 列覆盖 lint 在 reload 与 role 变更时生效；OIDC JWT 与 dev token 均可认证 | VERIFIED | 差分：`rowpred_diff` 6 通过（本机 Rust 对 SQLite）；Trino 483 的 `rowpred_trino_it` 3 通过见上一轮 CI 39541882392 / 39543562659，其代码在三个 gap 计划中未改。覆盖 lint：现在在 `scope plan/apply`、`reload`、`migrate --apply` 三处都生效，整对象读取也被拦（gap 3 关闭）。认证：`oidc` 31、`dev_token` 16、`cli_dev_token` 12 通过。 |
| 6 | Dev quickstart: 文档里的示例配置在全新检出上可用 (SC1 第一步、SC5 dev token、AUTH-02) | VERIFIED | gap 2 关闭，见上表。 |
| 7 | decimal 在进入类型化行、不可变 log、历史快照的途中从不被舍入 (SC1 "强类型"、SC2 "不可变 log") | VERIFIED | gap 1 关闭，见上表。边界是"字符串或整数 JSON 数字"，超过 u64 的整数也会被 serde_json 解析成 f64 而被拒绝，不会静默舍入。 |
| 8 | reload 在切换前让新表 / 新列在 Trino 可见，包括 Trino 元数据缓存开启时 (STORE-02) | PRESENT_BEHAVIOR_UNVERIFIED | 探测、顺序、pending/active 切换、超时保留旧定义由 `reload.rs` 的 `probe_*`（脚本化 Trino）和真实 Trino 483 的 `reload_trino_it` 2 个测试覆盖。flush 调用本身的效果没有被隔离（CI catalog 默认 `metadata.cache-ttl=0s`，flush 失败被探测容忍）。见 Human Verification。 |

**Score:** 7/8 truths verified (1 present, behavior-unverified)

### Decisions on the code-review findings (03-REVIEW.md, commit caa3935)

审查文档逐条读过。上一轮的 6 条定向发现（CR-A01、WR-A01、WR-A03、WR-B07、WR-C05、CR-D01）我已用代码和测试独立确认为关闭。其余逐项判定：

| Finding | Decision | Reason |
|---|---|---|
| **WR-01** migrate 的 role 复核与 `scope apply` 口径互斥（重命名被 role 引用的属性无法一步完成） | Advisory，建议后续处理 | 我读了 `docs/scope.md` 与 `apply_in`：`scope apply` 要求 `--ontology` 等于当前生效定义，migrate 复核按新定义编译，所以对被引用属性的重命名需要先撤引用、迁移、再加回，中间 role 暂时失去该列。方向是 fail-closed（拒绝而不是放行），文档已写明撤引用的出路，只是没提重命名。它不违反任何 Success Criterion 或需求文本，SC5 要的是"lint 在 reload 与 role 变更时生效"，现在生效。不阻塞，但值得在 Phase 3 收尾前补一句文档（重命名需两次 `scope apply`）或在后续 phase 里支持 `--for-migration`。 |
| **WR-02** dev 签名密钥缺少文件级保护（Windows ACL 继承、Unix 不校验既有文件权限与符号链接） | Advisory | dev token 只被 `--dev` 启动的服务接受，配置不能单独启用；限制已写进 `docs/edit-store.md` 已知缺口 7，`dev.rs` 注释已不再声称 owner-only。AUTH-02 的文本是"无需外部 IdP 即可生成本地 token"，成立。 |
| **CR-B01**（遗留）CI 的 openssl 守卫第一行不会失败 | Advisory | 我重读了 `.github/workflows/ci.yml:32-34`，两行 `! cargo tree ... -i` 原样未变，`set -e` 下第一行的取反失败不会终止步骤，缺陷属实。但它约束的结果当前成立：我跑 `cargo tree --workspace --locked --features ontoforge-storage/embedded-pg -e normal,build,dev -i openssl` 与 `-i native-tls`，两者都是 "did not match any packages"。下一次改依赖之前修。 |
| **CR-D02**（遗留）server 配置解析错误回显被拒绝的值 | Advisory | `config.rs` 自 47364cf 起无 diff。T-03-04 的结论（连接串里的 `url` / `password` / `token` / `secret` 被改写为不含值的消息，只接受 `url_env` / `url_file`）仍成立；泄漏需要把密钥误贴进一个不相关的类型化字段。建议下一个触碰 `config.rs` 的计划里修，并更正头注释里"never secret values"的措辞。 |
| IN-01 到 IN-06 | Advisory | IN-01（decimal 拒绝文案里 18 个连续空格，`value.rs:341`）我读到了，纯文案，会出现在 ONT-V27 诊断里，建议一并修。IN-04（公共 `apply_migration` 默认不做 role 复核；migrate 不为新列补 role 索引）是 Phase 5 / 7 入口复制的隐患，记入后续。其余是测试清理与预览命令的副作用，不影响需求。 |
| 其余遗留 warning（WR-A02/A04/A05、B01 到 B06、B08 到 B10、C02 到 C04、C06、C07、D02 到 D06） | Advisory | 文件在 47364cf 之后未变；第一次验证已逐条判为加固项。再次核对：没有任何一条让某个 Success Criterion 或需求文本失败。WR-B03、WR-B10（`plan_drop_archived` 把哈希失败变成空串）、WR-C04（缺失行属性按 Null 求值）是最值得先修的三条，都应在 Phase 4 或 Phase 5 开始前处理。 |

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/ontoforge-storage/src/{value,commit,store,migrate,reload,...}.rs` | edit store | VERIFIED | 03-21 / 03-22 / 03-23 触及 `value.rs`、`commit.rs`、`store.rs`、`migrate.rs`；全部被对应测试覆盖并通过；无 stub |
| `crates/ontoforge-auth/src/scope/coverage.rs` | 覆盖 lint | VERIFIED | 整对象读取分支实现，非占位 |
| `crates/ontoforge-auth/src/authn/dev.rs` | dev 密钥 | VERIFIED | `create_key_dir` 创建上级目录 |
| `crates/ontoforge/src/migrate.rs` | migrate CLI | VERIFIED | `roles_check` 实装并传入 |
| `examples/server/ontoforge-server.dev.toml` | 示例配置 | VERIFIED | 在空树上可用（两个 CLI 测试） |
| `docs/edit-store.md`、`docs/scope.md` | 运维文档 | VERIFIED | 与代码同步（整对象读取、migrate 复核、已知缺口 7） |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `ontoforge migrate --apply` | 已存 role | `roles_check()` -> `apply_migration_probed(.., Some(roles))` -> `apply_in` 的 `load_scope_in` + `compile_scope` | WIRED | 03-22；SQLite、CLI、PostgreSQL 三层测试 |
| `reload` | 已存 role | `ReloadOptions::roles` | WIRED | 回归：`reload_rechecks_applied_roles` 通过 |
| `coverage::problem` | 整对象读取 | `segments.get(2)` 为 None 时检查全部属性 | WIRED | `whole_object_read_needs_every_property` |
| JSON 数字 | decimal 列 | `from_json_as` 的 `is_i64 \|\| is_u64` | WIRED | 单元、import、conformance 三层 |
| `open_postgres` | 会话设置 | `pg_connect_options` 固定 `DateStyle`、`TimeZone`、`extra_float_digits` | WIRED | `pg_sessions_use_the_pinned_settings` 在 embedded PG 通过 |
| `EditStore::open` / `DevKey::create` | 上级目录 | `create_dir_all` / `create_key_dir` | WIRED | 全新树 CLI 测试 |
| `commit()` | `of_def_state` 栅栏、`import`、`history` 掩码、`scope apply` 审计 | 见 47364cf 报告 | WIRED | 回归测试均通过（`history_masking` 12、`scope_store` 11、`import` 16） |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| edit 行 / log / 历史中的 decimal | `PropertyValue::Decimal` | 字符串或整数 JSON 数字，经 `canonical_as` 补齐 scale | 是，逐字一致（conformance 三处比较，两个后端） | FLOWING |
| `migrate --apply` 的 role 复核 | 已存 role | `scope_store::load_scope_in(tx)` 读事务内的系统表 | 是 | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| 工作区全量测试（orchestrator 在 HEAD 跑） | `cargo test --workspace --locked --no-fail-fast` | exit 0；84 suites；984 passed；0 failed；67 ignored（需 Docker / Trino / 外部 PG） | PASS（引用，未重跑） |
| decimal 单元测试 | `cargo test -p ontoforge-storage --lib value::` | 14 passed | PASS |
| SQLite 往返、import、migrate、reload、连接 | `--test conformance_sqlite --test import --test migrate --test reload --test sqlite_connection` | 28 + 16 + 15 + 32 + 4 passed | PASS |
| 覆盖 lint 与 dev token | `-p ontoforge-auth --test criteria_coverage --test dev_token` | 6 + 16 passed | PASS |
| CLI 全新树与 migrate 复核 | `-p ontoforge --test cli_reload --test cli_dev_token --test cli_migrate --test cli_scope` | 8 + 12 + 10 + 8 passed | PASS |
| PostgreSQL（embedded） | `-p ontoforge-storage --features embedded-pg --test conformance_pg --test migrate_pg -- --ignored` | 29 + 4 passed | PASS |
| 回归：差分 / 合并 / 编译 / OIDC / 叠加 proptest / 掩码 / 表达式 | `rowpred_diff` 6, `scope_merge` 9, `scope_compile` 21, `oidc` 31, `model_prop` 1, `history_masking` 12, `scope_store` 11, ontoforge-expr 全部 | 全部 passed，0 failed | PASS |
| 无 OpenSSL / native-tls | `cargo tree --workspace --locked --features ontoforge-storage/embedded-pg -e normal,build,dev -i openssl` / `-i native-tls` | 两者 "did not match any packages" | PASS |

### Probe Execution

Step 7c: SKIPPED。阶段未声明 probe（没有 `scripts/*/tests/probe-*.sh`，PLAN / SUMMARY 也未提到）。

### Requirements Coverage

16 个 ID（STORE-01..07、SCOPE-01..07、AUTH-01..02）都出现在某个 PLAN 的 `requirements` frontmatter 里，没有 orphan。按证据判定，不按 checkbox 判定。

| Requirement | Source Plans | REQUIREMENTS.md now | Verdict | Evidence |
|-------------|--------------|---------------------|---------|----------|
| STORE-01 | 03-01, 03-03, 03-04, 03-08, 03-09, 03-13, 03-20, 03-21 | ticked | SATISFIED | 两个后端同一 conformance；PG 值域校验已并入共享用例（WR-A01 关闭） |
| STORE-02 | 03-02, 03-03, 03-04, 03-10, 03-15, 03-18, 03-20 | unticked | SATISFIED, one human item open | 自动 DDL、ONT-V25、migrate 归档、栅栏、`lock_timeout`、pending/probe/active 切换都有测试；唯一缺口是 Trino flush 效果未隔离（truth 8）。人工项通过后再勾选 |
| STORE-03 | 03-08, 03-09, 03-21 | ticked | SATISFIED | 每次提交一条不可变 log 记录；追加专用触发器；decimal 保真已修 |
| STORE-04 | 03-17 | unticked | SATISFIED | `EditStore::history` 带 D-13 掩码，`history_masking` 12 通过。库 API 层交付，REST / MCP 外壳属于 Phase 7。建议勾选 |
| STORE-05 | 03-10, 03-13 | unticked | SATISFIED | 一对一部分唯一索引、`CardinalityViolation`、`concurrent_fk_claims_one_wins`（PG 实跑通过）。建议勾选 |
| STORE-06 | 03-19, 03-21 | ticked | SATISFIED | `ontoforge import` 作为 `system:import` 提交；JSON 与 CSV 路径的 decimal 都逐字保存 |
| STORE-07 | 03-08, 03-13 | unticked（traceability 表写 "Gaps Found"，已过时） | SATISFIED | `model_prop` 与 conformance 转换用例。建议勾选并改表 |
| SCOPE-01 | 03-06, 03-07, 03-14, 03-16 | unticked | SATISFIED | `of_user` / `of_role` / `of_role_binding`、`scope plan/apply`、JWT `groups`。`cli_scope` 8、`scope_store` 11 |
| SCOPE-02 | 03-02, 03-05, 03-06, 03-11, 03-14, 03-16 | unticked | SATISFIED | role schema 与编译快照测试。建议勾选 |
| SCOPE-03 | 03-11 | unticked | SATISFIED | `merge.rs` 与 `scope_merge` 9。建议勾选 |
| SCOPE-04 | 03-05, 03-11, 03-20 | unticked（traceability 写 "Gaps Found"，已过时） | SATISFIED | Rust 对 SQLite 本机 6 通过；Rust 对 Trino 483 的 CI 3 通过，代码未变。建议勾选并改表 |
| SCOPE-05 | 03-05, 03-11, 03-14 | unticked | SATISFIED | 无授权不可见、缺失属性为 Unknown、不可编译谓词在 plan / apply / reload 为 ONT-V22。建议勾选 |
| SCOPE-06 | 03-14, 03-16, 03-18, 03-22 | ticked | SATISFIED | reload、scope apply、migrate --apply 三处都复核；整对象读取已拦。第一次验证建议撤回的勾选现在可以保留 |
| SCOPE-07 | 03-16, 03-18 | unticked | PARTIAL, remainder deferred | Phase 3 部分交付；请求固定定义版本（读侧）与 MCP 无变更工具属于 Phase 6 / 7（见 `deferred`）。保持不勾选 |
| AUTH-01 | 03-01, 03-02, 03-07 | unticked（traceability 写 "Gaps Found"，已过时） | SATISFIED | `oidc` 31 通过。建议勾选并改表 |
| AUTH-02 | 03-12, 03-23 | ticked | SATISFIED | 全新树上的 `dev-token` 与 `reload` 测试通过 |

我不修改 REQUIREMENTS.md；上表"建议勾选"由 orchestrator 在本报告落定后决定。

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/ontoforge-storage/src/value.rs` | 341 | 错误文案中间夹 18 个连续空格（续行写成了行内空格） | Info | 只影响 ONT-V27 诊断的可读性（IN-01） |
| `.github/workflows/ci.yml` | 32-34 | `! cmd` 在 `errexit` 下不能让第一行失败 | Warning | 守卫对 `openssl` 失效；当前结果仍成立（CR-B01，已核实） |
| `crates/ontoforge-engine/src/config.rs` | ~397-439 | 解析错误回显被拒绝的值 | Warning | 误贴密钥会被打印（CR-D02） |
| `crates/ontoforge-auth/src/authn/dev.rs` | 106-130 | 不校验既有密钥文件的权限、符号链接 | Warning | dev-only（WR-02） |

阶段触及的代码、配置、文档、CI 文件中没有 `TBD`、`FIXME`、`XXX` 标记（对 `git diff --name-only 47364cf HEAD` 的 crates / examples / docs / ci / .github 文件 grep，无命中）。

### Regression Check

第一次验证通过的全部类别（SC2 conformance、SC3 proptest、SC4 scope 与合并、SC5 差分与 OIDC、历史掩码、import、scope 审计）在本轮重新跑过，没有失败。Phase 1 与 Phase 2 的测试包含在 orchestrator 的 984 通过里，0 失败。

## Human Verification Required

### 1. Trino 元数据缓存 flush 的效果

**Test:** 在 `ci/trino/catalog/editstore.properties` 加 `metadata.cache-ttl=10m` 与 `metadata.cache-missing=true`；让 `reload_trino_it` 先查询表，再加列，再 reload。跑两次：原样一次，去掉探测里的 `flush_metadata_cache` 调用一次。
**Expected:** 有 flush 时通过，没有时失败。
**Why human:** 需要缓存开启的 Trino catalog；当前绿色的 CI 分不清"flush 起效"和"本来就没缓存"（03-20 SUMMARY 假设 A5，审查 IN-B04）。

### 2. 推送并在 Linux CI 上确认 gap closure 的 29 个提交

**Test:** 推送分支，等 `ci` workflow 的 `test (ubuntu-latest)`、`test (windows-latest)`、`integration` 三个 job 完成。
**Expected:** 全绿，特别是 `dev_token.rs#missing_key_directory_is_created` 里 `#[cfg(unix)]` 的 0700 断言真正运行并通过。
**Why human:** 最近一次 CI 在这些提交之前；Unix 分支在 Windows 不编译。PostgreSQL 侧我已用 embedded PostgreSQL 本机跑过，不依赖这一步；这一步确认 Unix 分支与 Trino 作业。0700 只是纵深防御，不影响任何 Success Criterion。

## Gaps Summary

没有 gap。三个上一轮 gap 均关闭并有独立证据（单元、集成、CLI 全新树、PostgreSQL 实跑）。剩下的是两项人工确认和一批 advisory：建议在关闭 Phase 3 之前补 WR-01 的文档一句话、修 CI openssl 守卫（CR-B01）和 IN-01 文案；其余遗留 warning 在 Phase 4 / 5 开始前择要处理。

---

_Verified: 2026-10-21T18:30:00Z_
_Verifier: Claude (gsd-verifier)_
