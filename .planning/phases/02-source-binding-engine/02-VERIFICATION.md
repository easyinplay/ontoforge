---
phase: 02-source-binding-engine
verified: 2026-10-17T03:45:22Z
status: human_needed
score: 11/15 must-haves verified
covered_files:
  - ".github/workflows/ci.yml"
  - ".github/workflows/nightly.yml"
  - ".planning/REQUIREMENTS.md"
  - ".planning/phases/02-source-binding-engine/02-01-PLAN.md"
  - ".planning/phases/02-source-binding-engine/02-01-SUMMARY.md"
  - ".planning/phases/02-source-binding-engine/02-02-PLAN.md"
  - ".planning/phases/02-source-binding-engine/02-02-SUMMARY.md"
  - ".planning/phases/02-source-binding-engine/02-03-PLAN.md"
  - ".planning/phases/02-source-binding-engine/02-03-SUMMARY.md"
  - ".planning/phases/02-source-binding-engine/02-04-PLAN.md"
  - ".planning/phases/02-source-binding-engine/02-04-SUMMARY.md"
  - ".planning/phases/02-source-binding-engine/02-05-PLAN.md"
  - ".planning/phases/02-source-binding-engine/02-05-SUMMARY.md"
  - ".planning/phases/02-source-binding-engine/02-06-PLAN.md"
  - ".planning/phases/02-source-binding-engine/02-06-SUMMARY.md"
  - ".planning/phases/02-source-binding-engine/02-07-PLAN.md"
  - ".planning/phases/02-source-binding-engine/02-07-SUMMARY.md"
  - ".planning/phases/02-source-binding-engine/02-08-PLAN.md"
  - ".planning/phases/02-source-binding-engine/02-08-SUMMARY.md"
  - ".planning/phases/02-source-binding-engine/02-09-PLAN.md"
  - ".planning/phases/02-source-binding-engine/02-09-SUMMARY.md"
  - ".planning/phases/02-source-binding-engine/02-10-PLAN.md"
  - ".planning/phases/02-source-binding-engine/02-10-SUMMARY.md"
  - "CONTRIBUTING.md"
  - "Cargo.toml"
  - "ci/ontoforge-server.ci.toml"
  - "ci/pg/iceberg_catalog.sql"
  - "ci/pg/seed.sql"
  - "ci/trino/catalog/demo.properties"
  - "ci/trino/catalog/editstore.properties"
  - "ci/trino/catalog/tpch.properties"
  - "ci/trino/iceberg/iceberg.properties"
  - "clippy.toml"
  - "crates/ontoforge-engine/Cargo.toml"
  - "crates/ontoforge-engine/src/bind_check.rs"
  - "crates/ontoforge-engine/src/bind_check/compat_tests.rs"
  - "crates/ontoforge-engine/src/bind_check/tests.rs"
  - "crates/ontoforge-engine/src/config.rs"
  - "crates/ontoforge-engine/src/error.rs"
  - "crates/ontoforge-engine/src/lib.rs"
  - "crates/ontoforge-engine/src/pk_check.rs"
  - "crates/ontoforge-engine/src/printer/ast.rs"
  - "crates/ontoforge-engine/src/printer/dialect.rs"
  - "crates/ontoforge-engine/src/printer/escape.rs"
  - "crates/ontoforge-engine/src/printer/mod.rs"
  - "crates/ontoforge-engine/src/printer/sqlite_ddl.rs"
  - "crates/ontoforge-engine/src/printer/tests.rs"
  - "crates/ontoforge-engine/src/sqlite/fixture.rs"
  - "crates/ontoforge-engine/src/sqlite/mod.rs"
  - "crates/ontoforge-engine/src/sqlite/tests.rs"
  - "crates/ontoforge-engine/src/trino/cancel.rs"
  - "crates/ontoforge-engine/src/trino/classify.rs"
  - "crates/ontoforge-engine/src/trino/client.rs"
  - "crates/ontoforge-engine/src/trino/decode.rs"
  - "crates/ontoforge-engine/src/trino/headers.rs"
  - "crates/ontoforge-engine/src/trino/mod.rs"
  - "crates/ontoforge-engine/src/value.rs"
  - "crates/ontoforge-engine/tests/bind_check_isolation.rs"
  - "crates/ontoforge-engine/tests/doc_type_table.rs"
  - "crates/ontoforge-engine/tests/drift.rs"
  - "crates/ontoforge-engine/tests/pk_check.rs"
  - "crates/ontoforge-engine/tests/printer_literals.rs"
  - "crates/ontoforge-engine/tests/printer_prop.rs"
  - "crates/ontoforge-engine/tests/server_config.rs"
  - "crates/ontoforge-engine/tests/sql_text_only_from_printer.rs"
  - "crates/ontoforge-engine/tests/sqlite_engine.rs"
  - "crates/ontoforge-engine/tests/trino_client.rs"
  - "crates/ontoforge-engine/tests/trino_it.rs"
  - "crates/ontoforge-engine/tests/trino_resilience.rs"
  - "crates/ontoforge-expr/tests/criteria.rs"
  - "crates/ontoforge-schema/Cargo.toml"
  - "crates/ontoforge-schema/src/diag.rs"
  - "crates/ontoforge-schema/src/ir/binding.rs"
  - "crates/ontoforge-schema/src/ir/link.rs"
  - "crates/ontoforge-schema/src/ir/manifest.rs"
  - "crates/ontoforge-schema/src/ir/mod.rs"
  - "crates/ontoforge-schema/src/ir/names.rs"
  - "crates/ontoforge-schema/src/ir/object.rs"
  - "crates/ontoforge-schema/src/lint/binding/e13.rs"
  - "crates/ontoforge-schema/src/lint/binding/e14.rs"
  - "crates/ontoforge-schema/src/lint/binding/e15.rs"
  - "crates/ontoforge-schema/src/lint/binding/e16.rs"
  - "crates/ontoforge-schema/src/lint/binding/mod.rs"
  - "crates/ontoforge-schema/src/lint/binding/w17.rs"
  - "crates/ontoforge-schema/src/lint/config.rs"
  - "crates/ontoforge-schema/src/lint/e08.rs"
  - "crates/ontoforge-schema/src/lint/e09.rs"
  - "crates/ontoforge-schema/src/lint/mod.rs"
  - "crates/ontoforge-schema/src/lint/naming/mod.rs"
  - "crates/ontoforge-schema/src/lint/structure/mod.rs"
  - "crates/ontoforge-schema/src/loader/discover.rs"
  - "crates/ontoforge-schema/src/loader/dispatch.rs"
  - "crates/ontoforge-schema/src/loader/fixture.rs"
  - "crates/ontoforge-schema/src/loader/mod.rs"
  - "crates/ontoforge-schema/src/loader/sourcemap.rs"
  - "crates/ontoforge-schema/src/schema_export.rs"
  - "crates/ontoforge-schema/src/validate/bindings.rs"
  - "crates/ontoforge-schema/src/validate/links.rs"
  - "crates/ontoforge-schema/src/validate/mod.rs"
  - "crates/ontoforge-schema/src/validate/objects.rs"
  - "crates/ontoforge-schema/tests/action_types.rs"
  - "crates/ontoforge-schema/tests/bindings.rs"
  - "crates/ontoforge-schema/tests/diagnostics.rs"
  - "crates/ontoforge-schema/tests/link_types.rs"
  - "crates/ontoforge-schema/tests/load_examples.rs"
  - "crates/ontoforge-schema/tests/loader.rs"
  - "crates/ontoforge-schema/tests/object_types.rs"
  - "crates/ontoforge-schema/tests/schema_export.rs"
  - "crates/ontoforge/Cargo.toml"
  - "crates/ontoforge/src/cli.rs"
  - "crates/ontoforge/src/lib.rs"
  - "crates/ontoforge/src/lint_cmd.rs"
  - "crates/ontoforge/src/online.rs"
  - "crates/ontoforge/src/pipeline.rs"
  - "crates/ontoforge/src/render.rs"
  - "crates/ontoforge/tests/cli.rs"
  - "crates/ontoforge/tests/cli_check_pk.rs"
  - "crates/ontoforge/tests/cli_lint.rs"
  - "crates/ontoforge/tests/cli_online.rs"
  - "crates/ontoforge/tests/cli_online_trino.rs"
  - "crates/ontoforge/tests/cli_trino_it.rs"
  - "crates/ontoforge/tests/private_fixtures.rs"
  - "docs/source-binding.md"
  - "schemas/ontoforge-definition.schema.json"
  - "schemas/ontoforge-manifest.schema.json"
covered_digest: "v1:sha256:1fe3e2d8bb03a0987f56872b5e17a5f82ed605155d323703dec65c56dbf6eb15"
behavior_unverified: 0
overrides_applied: 0
human_verification:
  - test: "推送分支并开 PR，查看 ci.yml 的 `integration` 作业日志"
    expected: "Postgres 17.11 与 Trino 483（按 digest）拉取成功；`trino_it` 与 `cli_trino_it` 的 `--ignored` 测试实际执行（非 0 个）且全部通过；`Ignored tests are actually there` 步骤通过"
    why_human: "本机 Windows 无 Docker / Trino；真实 Trino 行为（SC2 的真实 Trino 校验、SC4 的真实 Trino 执行、SC5）只能在 Linux CI 证实（02-VALIDATION.md Manual-Only、eng review E21）"
  - test: "等待首次 nightly 运行（Trino 483 与 484 两个矩阵），查看 Iceberg 作业"
    expected: "两个矩阵都通过；`iceberg_table_is_readable` 在 `ONTOFORGE_TRINO_ICEBERG=1` 下实际执行并通过；失败时 `report-failure` job 能开 / 追加 issue"
    why_human: "nightly 只在 GitHub Actions 定时触发后才有运行记录；本机无法运行 Iceberg + MinIO + Trino"
  - test: "按首次 CI 日志定稿 `crates/ontoforge-engine/tests/trino_it.rs` 中 `describe_types_golden` 与 `decode_golden` 的 inline snapshot，删除两处 `// PROVISIONAL: confirm against first CI run (E21)` 注释并提交"
    expected: "`rg -n PROVISIONAL crates` 无输出；定稿后的 golden 在 PR 与 nightly 上均为绿"
    why_human: "golden 预期值是推导值，只有真实 Trino 输出能证实（E21、02-10 success_criteria）"
  - test: "确认 WR-C06：CI 与 nightly 中 pin 的镜像 digest 可被拉取"
    expected: "`postgres:17.11@sha256:2d2b8998...` 与 `trinodb/trino:483@sha256:db58cc93...`（以及 nightly 484 的 digest）均拉取成功，作业未因 manifest unknown 失败"
    why_human: "digest 正确性只能由 registry 实际拉取证实"
  - test: "确认 WR-C07：CI `integration` 作业中 `check_pk_finds_duplicate_and_null_keys_on_ci_trino` 通过"
    expected: "对 `demo.aviation.flights_dup` 的 `--check-pk` 退出码 1，恰好两条 ONT-V18（`F-DUP-1` ×2 与 `2 row(s) with a NULL primary key`）"
    why_human: "真实 Trino 方言下的查重查询与 bigint / varchar 解码只能在 CI 证实（02-REVIEW-FIX.md 标为 pending-ci）"
  - test: "决定 WR-C03：是否修订 ADR-0021 第 7 条（服务配置把 profile 映射到 Trino 而绑定带 fixture 时报 ONT-V17 warning）"
    expected: "用户在 (a) 改为 Info、(b) 分配独立 code 仍为 warning、(c) 只在 summary 计数、(d) 保持现状 中拍板；若修订则补 ADR 与代码"
    why_human: "修法与已锁定的 ADR-0021 决策冲突，需要用户决定；当前行为使 aviation 这类带 fixture 的 ontology 在 `--server-config` + `--deny-warnings` 下恒为 exit 1"
---

# Phase 2: Source Binding & Engine Verification Report

**Phase Goal:** 开发者可以把 object type / M:N link 绑定到 Trino 物理表，ontoforge 经自写 client 连接已有 Trino 并校验绑定；Windows 本机用 SQLite 替身零外部依赖跑通
**Verified:** 2026-10-17T03:45:22Z（HEAD `3fe5ea1`，分支 `gsd/phase-02-source-binding-engine`，未推送，无 upstream）
**Status:** human_needed
**Re-verification:** No（initial verification）

## MVP Mode 说明

ROADMAP 标注 Phase 2 为 `Mode: mvp`，但 phase goal 不是 User Story 格式：`gsd-tools query user-story.validate` 返回 `valid: false`（缺 `As a ...`、`I want to ...`、`so that ...`）。按 orchestrator 指示仍完成核查，下表按目标中隐含的开发者流程（角色：开发者，结果：绑定被校验、本机零外部依赖跑通）给出 User Flow Coverage。若要严格走 MVP 框架，需用 `/gsd-mvp-phase 2` 把 goal 改写成 User Story（Phase 1 同样未采用该格式）。

## User Flow Coverage

| # | 步骤 | 预期 | 代码库证据 | 状态 |
|---|------|------|-----------|------|
| 1 | 开发者在定义中声明 `backing`，在 `bindings/<profile>/` 写 `SourceBinding` | 加载通过，契约违反报 E | `examples/ontologies/aviation/objects.yaml`（Airport editOnly，其余 source）、`bindings/dev/*.yaml`；`ontoforge validate` / `lint` 对 aviation 与 aviation-json 均 exit 0 | ✓ |
| 2 | 本机 `validate --online`（无服务配置）经 SQLite 替身比对绑定 | 全部绑定被检查，无不匹配 exit 0 | 实跑：`ok: ontology \`aviation\`, profile \`dev\`, 5 binding(s) checked online`，exit 0 | ✓ |
| 3 | 绑定写错列 / 主键重复时报告 | V17 定位到列值 exit 1；`--check-pk` 报 V18 | 实跑（临时副本）：`ONT-V17 ... bindings/dev/pilot.yaml:9:23`，exit 1；`ONT-V18 ... \`E-001\` ×2` 与 `1 row(s) with a NULL primary key`，exit 1 | ✓ |
| 4 | `--server-config` 指向 Trino，Trino 不可达 | fail-closed ONT-V19，exit 3 | 实跑（`http://127.0.0.1:1`）：`ONT-V19 Trino is unavailable: ... (os error 10061)`，exit 3 | ✓ |
| 5 | 对真实 Trino 校验绑定 | 列存在、类型兼容、`--check-pk` 在真实 Trino 上工作 | 仅 wiremock 覆盖（`cli_online_trino` 14 passed、`cli_check_pk` 8 passed）；`cli_trino_it` 为 `#[ignore]`，CI 未运行 | ? 待 CI |

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC1：定义以 `backing: source \| editOnly` 显式声明来源，`SourceBinding` 按 profile 分目录；decimal / timezone / collation / geopoint 契约违反时 lint 报 E；示例仍 lint clean | ✓ VERIFIED | `crates/ontoforge-schema/src/lint/binding/e13.rs`..`e16.rs`、`w17.rs` 经 `lint/mod.rs:168 rules.extend(binding::rules())` 注册；每条规则有 triggers / clean insta 快照；`KNOWN_RULE_IDS` 含 E13..E16、W17；invalid 样例 `v05-missing-backing`、`v11-backing-on-one-to-many`、`v16-*`、`v08-duplicate-binding`、`v09-binding-unknown-target`；`schemas/ontoforge-definition.schema.json` required 含 `backing`；aviation / aviation-json 的 validate 与 lint 实跑均 exit 0 |
| 2 | SC2：`validate --online` 对真实 Trino 校验绑定（列存在、类型兼容）并报告不匹配；`--check-pk` 报告重复主键 | ? UNCERTAIN（本机部分 VERIFIED） | 本机：SQLite 替身与 wiremock Trino 路径完整（见 User Flow 2-4）；`bind_check.rs` 的 `compare_binding` 被 CLI 与 `check_drift` 共用；`pk_check` 6 passed、`cli_check_pk` 8 passed。真实 Trino：`cli_trino_it::validate_online_against_ci_trino` 与 `check_pk_finds_duplicate_and_null_keys_on_ci_trino` 为 `#[ignore]`，从未执行 |
| 3 | SC3：Trino client 执行查询、按超时与客户端断连取消、带 trace token 与 client tags；不可达返回 `EngineUnavailable` | ✓ VERIFIED | `trino/mod.rs` POST `/v1/statement` + nextUri 循环；`headers.rs::statement_headers` 发 `x-trino-trace-token`、`x-trino-client-tags`、`x-trino-time-zone: UTC`、`x-trino-session`；`online.rs:126` token 为 `ontoforge-cli-<ULID>`。取消不变量由行为测试证实（全量运行中通过）：`timeout_sends_delete`、`dropped_future_sends_delete`（断言 DELETE 路径 `/v1/statement/q1/1` 且带 `x-trino-user`）、`max_rows_truncates_and_deletes`、`finished_query_sends_no_delete`、`timed_out_query_is_cancelled_before_exit`；不可达：`closed_port_is_unavailable`、`dns_failure_is_unavailable`、`exhausted_503_is_unavailable`，以及 CLI 实跑 exit 3 |
| 4 | SC4：自有 printer 生成的 SQL 经 proptest 与真实 Trino / SQLite 执行验证；SQLite 替身在 Windows `cargo test` 全绿 | ? UNCERTAIN（本机部分 VERIFIED） | 本机：`printer_prop` 5 passed（结构不变、SQLite 往返、标识符往返、NUL 拒绝）、`sql_text_only_from_printer` 1 passed、`sqlite_engine` 28 passed；`PrintedSql::new` 无可见性修饰（`printer/mod.rs`）；`escape.rs` 是唯一出口。全量 `cargo test --workspace --locked`：55 个 test result，594 passed / 0 failed / 12 ignored。真实 Trino：`printer_corpus_round_trips` 为 `#[ignore]`，未执行 |
| 5 | SC5：Linux CI 以 Trino + PG service container 跑集成测试，nightly 加跑 Iceberg | ? UNCERTAIN | 产物齐全且接线正确：`ci.yml` 的 `integration` job（postgres service `ports: ["5432:5432"]`、`docker run` Trino 483 按 digest、`--ignored` 两个目标、ignored 计数断言）；`nightly.yml` 483 / 484 矩阵、Iceberg、`report-failure` 独占 `issues: write`、workflow 级 `contents: read`。分支未推送（`no upstream configured`），作业从未运行 |
| 6 | 02-01：backing 必填（V05 / V06 / V11）、Phase 1 fixture 迁移语义零漂移（E19）、ADR-0021、`read_limited`、`ProfileName` 与 V16..V20 | ✓ VERIFIED | ADR-0021 `status: accepted`；自 phase 2 起点 `63097fc` 修改的 24 个既有 `.snap` 中，code / severity / message / help 行唯一变化是 `help` 的 kind 列表加入 `SourceBinding`（新 kind 的必然结果，非迁移漂移）；`invalid_examples` 测试全绿 |
| 7 | 02-02：engine crate、`PrintedSql` 私有构造、`describe_tables` 按 catalog、`is_engine_wide`、单连接流式截断 | ✓ VERIFIED | `error.rs:100` `is_engine_wide` 恰为 Unavailable / Unauthorized / Cancelled / Config；`lib.rs:92` `Engine` trait 只收 `&PrintedSql`；`describe_mixed_catalogs_is_config_error` 通过 |
| 8 | 02-03：`SourceBinding` 定义层、profile 选定顺序、V08 / V09 / V16、fixture 路径不逃逸 | ✓ VERIFIED | `ir/binding.rs`、`validate/bindings.rs`；invalid 样例 `v16-fixture-escapes-root`；CR-S01 修复 `loader::check_beneath` 拒绝中间目录 symlink / junction（`bindings.rs`、`sqlite_engine.rs` 回归测试） |
| 9 | 02-04：服务配置、密钥只经 `*_env` / `*_file`、无 TLS 跳过、nextUri 同源、两个身份独立 client | ✓ VERIFIED | `clippy.toml` 禁 4 个 `danger_*` 方法；`server_config` 18 passed；`foreign_next_uri_is_rejected`、`next_uri_scheme_mismatch_is_rejected`、`two_identities_use_separate_clients`、`set_session_header_is_ignored` 通过 |
| 10 | 02-05：fixture 替身、ONT-V17 定位、单绑定错误隔离（V19 + exit 3）、无 fixture 绑定提示 V20 | ✓ VERIFIED | `cli_online` 12 passed、`bind_check_isolation` 2 passed；V17 定位实跑为 `bindings/dev/pilot.yaml:9:23`（指向 `column` 值） |
| 11 | 02-06：不可用分类、有界重试、POST 只在可证明未接受时重试、超时 / 断连 / 上限取消 | ✓ VERIFIED | `trino_resilience` 27 passed，含 `post_plain_503_is_not_retried`、`post_502_is_not_retried`、`post_503_starting_up_is_retried`、`retry_after_is_capped`、`retries_respect_timeout`、`oversized_page_is_protocol_error` |
| 12 | 02-07：ONT-E13..E16 与 ONT-W17（含不可读 fixture 不静默） | ✓ VERIFIED | `w17__tests__unreadable.snap` 存在；规则可按 ID 调级（`e13.rs:170`、`e14.rs:286` 测试） |
| 13 | 02-08：drift 库函数、D-13 兼容表、文档与代码一致 | ✓ VERIFIED | `bind_check.rs:770 check_drift`、`DegradeReason::{SourceSchemaDrift, SourceUnreadable}`；`drift` 8 passed、`compat_tests.rs`；`doc_type_table` 2 passed（逐行对照 `docs/source-binding.md`） |
| 14 | 02-09：`--server-config`、退出码 3、密钥不打印、trace token、`--check-pk` 排除 NULL、`--check-pk` 无 `--online` 为用法错误 | ✓ VERIFIED | 实跑 `--check-pk` 无 `--online` exit 2；`cli_online_trino` 含 `rejected_plaintext_secret_never_printed`（CR-C01 回归）；实跑 NULL 主键单独计数、不出现 NULL 伪重复 |
| 15 | 02-10 完成门禁（E21）：PR `integration` 与首次 nightly 实跑通过，golden 按首跑定稿、删除 `PROVISIONAL` | ? UNCERTAIN | `crates/ontoforge-engine/tests/trino_it.rs:262` 与 `:364` 仍有 `// PROVISIONAL: confirm against first CI run (E21)`；CI 未运行。按 02-VALIDATION.md 与 02-10 success_criteria，此项未满足前不得写 passed |

**Score:** 11/15 truths verified（0 present-behavior-unverified；4 项待真实 CI 证实）

### Required Artifacts

`gsd-tools query verify.artifacts` 对 10 个 PLAN：50 个 artifact 中 49 个通过。唯一未通过项为 `crates/ontoforge-engine/src/trino/client.rs` 缺 pattern `v1/statement`：URL 拼接实际在 `trino/mod.rs:80`（`statement_url(&conn.url)`），`select_one_round_trip`、`request_headers` 等 wiremock 测试证实 POST 路径正确，判为 pattern 位置误报，artifact ✓ VERIFIED。

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/ontoforge-engine/` | 引擎 crate（printer / sqlite / trino / bind_check / pk_check / config） | ✓ VERIFIED | 23 个源文件、6697 行；均有测试覆盖 |
| `crates/ontoforge-schema/src/ir/binding.rs`、`validate/bindings.rs`、`lint/binding/*` | 绑定定义层与类型契约 lint | ✓ VERIFIED | 已注册、有快照 |
| `crates/ontoforge/src/online.rs` | `validate --online` / `--server-config` / `--check-pk` | ✓ VERIFIED | CLI 实跑通过 |
| `docs/source-binding.md` | 绑定规范 | ✓ VERIFIED | `doc_type_table` 对照代码 |
| `.planning/adr/0021-source-binding-files-and-backing.md` | 决策记录 | ✓ VERIFIED | `status: accepted` |
| `.github/workflows/ci.yml`、`nightly.yml`、`ci/**` | Linux 集成与 nightly | ✓ 存在且接线（执行待 CI） | 见 truth 5 |
| `examples/ontologies/aviation/bindings/dev/**` | 示例绑定与 fixture | ✓ VERIFIED | 5 个绑定、5 个 CSV |

### Key Link Verification

`gsd-tools query verify.key-links`：28 条中 26 条通过。2 条未通过均为 pattern 误报：

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `crates/ontoforge-engine/src/lib.rs` | `trino/mod.rs` | `AnyEngine::Trino` | ✓ WIRED | 代码用 `Self::Trino(e)`（`lib.rs:124/135/146/153`）；`online.rs:393` 构造 `AnyEngine::Trino(engines.ontology)` |
| `lint/binding/e14.rs` | `loader/sourcemap.rs` | `DefKind::SourceBinding` | ✓ WIRED | 经 `lint/binding/mod.rs:33 binding_def` 间接使用 `DefKind::SourceBinding` |
| `online.rs` | `bind_check::compare_binding` / `check_drift` | 共用比较 | ✓ WIRED | `compare_all` → `describe_bindings` → `binding_diagnostics` |
| `online.rs` | `pk_check::check_primary_keys` | `--check-pk` | ✓ WIRED | `online.rs:248-251`；不匹配表不扫描（`usable` 判定，设计如此） |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `online.rs::compare_all` | `described` | `Engine::describe_tables`（SQLite：fixture 建表；Trino：`information_schema.columns`） | 是（SQLite 实跑；Trino 经 wiremock） | ✓ FLOWING（真实 Trino 待 CI） |
| `online.rs::check_keys` | V18 诊断 | `pk_check` 两条 printer 生成的查询 | 是（实跑报出 `E-001` ×2 与 NULL 计数） | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| 全量测试（只跑一次） | `cargo test --workspace --locked` | EXIT=0；55 个 test result，594 passed / 0 failed / 12 ignored | ✓ PASS |
| 离线校验 | `ontoforge validate examples/ontologies/aviation` | `ok: ontology \`aviation\` with 5 object type(s), profile \`dev\``，exit 0 | ✓ PASS |
| lint clean | `ontoforge lint examples/ontologies/aviation`（及 aviation-json） | `has no lint findings`，exit 0 | ✓ PASS |
| SQLite 替身在线校验 | `ontoforge validate examples/ontologies/aviation --online`（及 aviation-json） | `5 binding(s) checked online`，exit 0 | ✓ PASS |
| `--check-pk` JSON | `ontoforge validate examples/ontologies/aviation --online --check-pk --format json` | `[]`，exit 0 | ✓ PASS |
| 列不匹配 | 临时副本改 `fullName: { column: full_nam }` 后 `--online` | `ONT-V17` 定位 `bindings/dev/pilot.yaml:9:23`，exit 1 | ✓ PASS |
| 重复 / NULL 主键 | 临时副本追加 `E-001` 与空主键行后 `--online --check-pk` | 两条 `ONT-V18`，exit 1；不带 `--check-pk` 时 exit 0 | ✓ PASS |
| 用法错误 | `validate --check-pk`（无 `--online`） | exit 2 | ✓ PASS |
| Trino 不可达 | `--server-config`（`url = "http://127.0.0.1:1"`） | `ONT-V19 Trino is unavailable: ... (os error 10061)`，exit 3，耗时 13.9 s | ✓ PASS |
| 真实 Trino 集成 | `cargo test -p ontoforge-engine --test trino_it -- --ignored` | 本机无 Trino，未运行 | ? SKIP（human） |

### Probe Execution

本 phase 未声明 `scripts/*/tests/probe-*.sh`，仓库中也不存在此类 probe。Step 7c: SKIPPED（无 probe）。

### Requirements Coverage

PLAN frontmatter 声明的 ID 并集为 BIND-01、BIND-02、BIND-03、ENGINE-01、ENGINE-02、ENGINE-03、ENGINE-04、ENGINE-05，与 REQUIREMENTS.md 中映射到 Phase 2 的 8 个 ID 完全一致，无 ORPHANED。

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| BIND-01 | 02-01、02-03 | 独立 `SourceBinding` 文件 + 必填 `backing` | ✓ SATISFIED | truth 1、6、8 |
| BIND-02 | 02-03、02-07、02-08 | 类型契约违反时 lint 报 E | ✓ SATISFIED | truth 1、12、13 |
| BIND-03 | 02-05、02-08、02-09 | `validate --online`、`--check-pk`、drift 降级模型 | ✓ SATISFIED（Phase 2 范围） | truth 10、13、14；启动 / 定时 drift 执行按 traceability 属 Phase 7 |
| ENGINE-01 | 02-04、02-06、02-09、02-10 | 自写 Trino client、超时、断连取消、trace token、两个身份 | ? NEEDS HUMAN | 客户端行为经 wiremock 证实（truth 3、9、11）；真实 Trino 服务端取消（`timeout_cancels_server_query`）待 CI |
| ENGINE-02 | 02-04、02-06、02-09 | fail-closed `EngineUnavailable` | ✓ SATISFIED（Phase 2 范围） | truth 3、11；`/health` 与 describe 暴露按 traceability 属 Phase 7 API-01 |
| ENGINE-03 | 02-02、02-10 | 自有 printer，proptest 与真实执行验证 | ? NEEDS HUMAN | proptest 与 SQLite 执行通过；真实 Trino 语料往返待 CI |
| ENGINE-04 | 02-02、02-05 | SQLite 替身支撑 Windows `cargo test` | ✓ SATISFIED（Phase 2 范围） | 594 passed / 0 failed；`serve --dev` 属 Phase 7 DX-01 |
| ENGINE-05 | 02-10 | Linux CI Trino + PG，nightly Iceberg，私有 fixture 入口 | ? NEEDS HUMAN | 工作流与 `private_fixtures.rs` 存在；仓库无私有数据；执行待 CI |

REQUIREMENTS.md 中 ENGINE-01 / 03 / 05 仍为 `[ ]` Pending，与上表一致。

### Anti-Patterns Found

扫描自 phase 2 起点（`63097fc`）以来修改的 104 个非规划文件。

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| （全部） | — | `TBD` / `FIXME` / `XXX` | — | 无匹配，debt marker 门禁通过 |
| `crates/ontoforge-engine/tests/trino_it.rs` | 262、364 | `PROVISIONAL` golden | ⚠️ Warning | 真实 Trino 预期值未定稿；属 E21 门禁，列入 human_verification |
| `crates/ontoforge/src/online.rs` | `ignored_fixtures` | WR-C03（fixture 被忽略的 V17 warning 让 `--deny-warnings` 在 Trino 配置下恒为 exit 1） | ⚠️ Warning | 与 ADR-0021 第 7 条锁定决策一致，需要用户决定是否修订 |
| `crates/ontoforge/src/online.rs` | — | 不可达 Trino + 较短 `--timeout`（实测 3000 ms）在 Windows 上报 `the query timed out` 而非 `Trino is unavailable` | ℹ️ Info | Windows 对拒绝连接的 TCP 重试约 2 s / 次，重试耗尽前已超时；两条路径都是 ONT-V19 + exit 3，fail-closed 成立，只是原因文字不同 |
| `.planning/ROADMAP.md` | Phase 7 SC | REQUIREMENTS traceability 把 `/health` 与 describe（ENGINE-02）、启动 / 定时 drift（BIND-03）指向 Phase 7，但 Phase 7 的 5 条 SC 未明文覆盖这两项 | ℹ️ Info | 不影响 Phase 2；Phase 7 plan 时应补进 SC，避免掉落 |
| `02-VALIDATION.md` | frontmatter | `status: draft`、`nyquist_compliant: false`，Per-Task 表全为 pending | ℹ️ Info | 文档未回填；不影响代码验证结论 |

### Human Verification Required

### 1. PR `integration` 作业首跑

**Test:** 推送分支并开 PR，查看 `ci.yml` 的 `integration` 作业日志。
**Expected:** 镜像按 digest 拉取成功；`trino_it` 与 `cli_trino_it` 的 `--ignored` 测试实际执行且全部通过；`Ignored tests are actually there` 通过。
**Why human:** 本机无 Docker / Trino，SC2、SC4、SC5 中的真实 Trino 部分只能由 Linux CI 证实。

### 2. 首次 nightly（483、484）

**Test:** 等首次 nightly 运行，查看两个矩阵的 Iceberg 作业。
**Expected:** 都通过，`iceberg_table_is_readable` 实际执行（`ONTOFORGE_TRINO_ICEBERG=1`）。
**Why human:** 需要 GitHub Actions 定时触发。

### 3. golden 定稿（E21）

**Test:** 按首跑日志更新 `describe_types_golden` 与 `decode_golden`，删除两处 `PROVISIONAL` 注释并提交。
**Expected:** `rg -n PROVISIONAL crates` 无输出，PR 与 nightly 均绿。
**Why human:** 预期值为推导值。

### 4. WR-C06 镜像 digest

**Test:** 看 CI 拉取镜像的步骤。
**Expected:** 无 `manifest unknown`。
**Why human:** 只能由 registry 实拉证实。

### 5. WR-C07 真实 Trino 查重

**Test:** 看 `check_pk_finds_duplicate_and_null_keys_on_ci_trino` 结果。
**Expected:** exit 1，恰好两条 ONT-V18。
**Why human:** 真实 Trino 方言与解码。

### 6. WR-C03 决策

**Test:** 决定是否修订 ADR-0021 第 7 条。
**Expected:** 选定方案并落 ADR / 代码，或明确保持现状。
**Why human:** 与已锁定决策冲突，需要用户拍板。

### Gaps Summary

本机可验证的 must-have 全部成立，未发现 gap：绑定定义层、类型契约 lint、SQLite 替身在线校验、`--check-pk`、Trino client 的协议 / 取消 / 重试 / 分类、drift 库函数、文档与代码一致性都有实现、有接线、有通过的测试，CLI 实跑结果与 SUMMARY 声明一致（594 passed / 0 failed / 12 ignored 复现成功）。

未通过的 4 条 truth（SC2 与 SC4 的真实 Trino 部分、SC5、E21 完成门禁）都依赖首次 PR `integration` 作业与首次 nightly 运行。分支尚未推送，这些作业从未执行，`trino_it.rs` 的两组 golden 仍标 `PROVISIONAL`。按 02-VALIDATION.md Manual-Only 与 02-10 success_criteria，在这些 CI 运行通过、golden 定稿提交之前，本 phase 不能标为 passed。

---

_Verified: 2026-10-17T03:45:22Z_
_Verifier: Claude (gsd-verifier)_
