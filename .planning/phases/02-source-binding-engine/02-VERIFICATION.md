---
phase: 02-source-binding-engine
verified: 2026-10-19T08:32:39Z
status: passed
score: 15/15 must-haves verified
covered_files:
  - ".github/workflows/ci.yml"
  - ".github/workflows/nightly.yml"
  - ".planning/REQUIREMENTS.md"
  - ".planning/adr/0022-ignored-fixture-diagnostic-is-info.md"
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
  - ".planning/phases/02-source-binding-engine/02-UAT.md"
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
covered_digest: "v1:sha256:88af27a98b44656b0e52056802037fbd907df188c881af6b9ff535682875eea5"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: human_needed
  previous_score: 11/15
  gaps_closed:
    - "SC2：validate --online 与 --check-pk 对真实 Trino 生效（CI cli_trino_it 2 passed）"
    - "SC4：printer 语料在真实 Trino 上往返（CI printer_corpus_round_trips ok）"
    - "SC5：Linux CI Trino + PG 集成测试与 nightly Iceberg 实跑通过"
    - "02-10 完成门禁（E21）：golden 经首跑证实，PROVISIONAL 标记已删除"
    - "WR-C03：选定方案 (a)，ADR-0022 落地，ignored-fixture 诊断降为 Info"
  gaps_remaining: []
  regressions: []
---

# Phase 2: Source Binding & Engine Verification Report

**Phase Goal:** 开发者可以把 object type / M:N link 绑定到 Trino 物理表，ontoforge 经自写 client 连接已有 Trino 并校验绑定；Windows 本机用 SQLite 替身零外部依赖跑通
**Verified:** 2026-10-19T08:32:39Z（`main` HEAD `8bf3de2`，含已合并的 PR #2 与 PR #4）
**Status:** passed
**Re-verification:** Yes（上次 2026-10-17T03:45:22Z，HEAD `3fe5ea1`，status `human_needed`，11/15）

## MVP Mode 说明

ROADMAP 标注 Phase 2 为 `Mode: mvp`，但 phase goal 不是 User Story 格式（`user-story.validate` 为 `valid: false`，与上次相同）。核查仍按目标中隐含的开发者流程给出 User Flow Coverage。若要严格走 MVP 框架，需用 `/gsd-mvp-phase 2` 改写 goal；不影响本次结论。

## User Flow Coverage

| # | 步骤 | 预期 | 代码库证据 | 状态 |
|---|------|------|-----------|------|
| 1 | 开发者在定义中声明 `backing`，在 `bindings/<profile>/` 写 `SourceBinding` | 加载通过，契约违反报 E | `examples/ontologies/aviation/bindings/dev/*.yaml`；本次实跑 `ontoforge validate` / `lint` 对 aviation 与 aviation-json 均 exit 0 | ✓ |
| 2 | 本机 `validate --online`（无服务配置）经 SQLite 替身比对绑定 | 全部绑定被检查，无不匹配 exit 0 | 本次实跑：`ok: ontology \`aviation\`, profile \`dev\`, 5 binding(s) checked online`，exit 0 | ✓ |
| 3 | 绑定写错列 / 主键重复时报告 | V17 定位到列值 exit 1；`--check-pk` 报 V18 | 上次实跑结果（V17 `bindings/dev/pilot.yaml:9:23`；V18 `E-001` ×2 与 NULL 计数），相关代码未变；`cli_online`、`cli_check_pk` 本次全量运行通过 | ✓ |
| 4 | `--server-config` 指向 Trino，Trino 不可达 | fail-closed ONT-V19，exit 3 | 上次实跑 exit 3；`closed_port_is_unavailable`、`dns_failure_is_unavailable` 本次全量运行通过 | ✓ |
| 5 | 对真实 Trino 校验绑定 | 列存在、类型兼容、`--check-pk` 在真实 Trino 上工作 | CI 实跑：`validate_online_against_ci_trino ... ok`、`check_pk_finds_duplicate_and_null_keys_on_ci_trino ... ok`（PR run `39220900176`、main push run `39222552400`、nightly run `39221684753` 的 483 / 484 两个 job） | ✓ |

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC1：定义以 `backing: source \| editOnly` 显式声明来源，`SourceBinding` 按 profile 分目录；decimal / timezone / collation / geopoint 契约违反时 lint 报 E；示例仍 lint clean | ✓ VERIFIED | 回归检查：`lint/binding/e13.rs`..`e16.rs`、`w17.rs` 未改动且注册不变；本次实跑 aviation / aviation-json 的 `validate` 与 `lint` 均 exit 0（`has no lint findings`）；`invalid_examples` 等快照测试在本次全量运行中通过 |
| 2 | SC2：`validate --online` 对真实 Trino 校验绑定（列存在、类型兼容）并报告不匹配；`--check-pk` 报告重复主键 | ✓ VERIFIED | 真实 Trino：CI `integration` 中 `cli_trino_it` `2 passed; 0 failed`（PR run `39219827829`、`39220900176`；main HEAD `8bf3de2` 的 push run `39222552400`）；`check_pk_finds_duplicate_and_null_keys_on_ci_trino` 对 `demo.aviation.flights_dup` 通过；`validate_online_against_ci_trino` 断言 ignored-fixture 诊断 `severity == "info"`（WR-C03 后）在真实 Trino 上通过。本机：SQLite 替身与 wiremock 路径照旧全绿 |
| 3 | SC3：Trino client 执行查询、按超时与客户端断连取消、带 trace token 与 client tags；不可达返回 `EngineUnavailable` | ✓ VERIFIED | 回归检查：`trino/` 源码自上次核查未改动；`timeout_sends_delete`、`dropped_future_sends_delete`、`timed_out_query_is_cancelled_before_exit`、`closed_port_is_unavailable` 等在本次全量运行中通过；真实 Trino 上 `timeout_cancels_server_query ... ok`（CI integration 与 nightly 483 / 484） |
| 4 | SC4：自有 printer 生成的 SQL 经 proptest 与真实 Trino / SQLite 执行验证；SQLite 替身在 Windows `cargo test` 全绿 | ✓ VERIFIED | 本机 `cargo test --workspace --locked`：EXIT=0，55 个 test result，595 passed / 0 failed / 12 ignored（12 = `trino_it` 9 + `cli_trino_it` 2 + `private_fixtures` 1）；`printer_prop`、`sqlite_engine` 在内。真实 Trino：`printer_corpus_round_trips ... ok`、`decode_golden ... ok`（CI integration 与 nightly 两个矩阵）。CI `test (windows-latest)` 在 run `39222552400` 为 success |
| 5 | SC5：Linux CI 以 Trino + PG service container 跑集成测试，nightly 加跑 Iceberg | ✓ VERIFIED | PR / push `integration` job：postgres `17.11@sha256:2d2b8998...` 与 `trinodb/trino:483@sha256:db58cc93...` 按 digest 拉取成功，`trino_it` 9 passed、`cli_trino_it` 2 passed，`Ignored tests are actually there` 报 `trino_it: 9 ignored tests`、`cli_trino_it: 2 ignored tests`。nightly run `39221684753`（workflow_dispatch，head `2ecdf66`）：`iceberg (483, ...)` 与 `iceberg (484, sha256:2704eb53...)` 均 success，`ONTOFORGE_TRINO_ICEBERG: 1` 下 `iceberg_table_is_readable ... ok`（该测试在变量为 1 时才执行 describe + 行数比对，见 `trino_it.rs:626-649`），`report-failure` skipped。`2ecdf66..8bf3de2` 的非 `.planning` diff 为空，nightly 实测内容即 main HEAD 内容 |
| 6 | 02-01：backing 必填（V05 / V06 / V11）、Phase 1 fixture 迁移语义零漂移（E19）、ADR-0021、`read_limited`、`ProfileName` 与 V16..V20 | ✓ VERIFIED | 回归：相关文件未改动，`verify.artifacts` 02-01 8/8、`verify.key-links` 2/2；全量测试通过 |
| 7 | 02-02：engine crate、`PrintedSql` 私有构造、`describe_tables` 按 catalog、`is_engine_wide`、单连接流式截断 | ✓ VERIFIED | 回归：02-02 artifacts 7/7、key-links 3/3；全量测试通过 |
| 8 | 02-03：`SourceBinding` 定义层、profile 选定顺序、V08 / V09 / V16、fixture 路径不逃逸 | ✓ VERIFIED | 回归：02-03 artifacts 8/8、key-links 3/3；全量测试通过 |
| 9 | 02-04：服务配置、密钥只经 `*_env` / `*_file`、无 TLS 跳过、nextUri 同源、两个身份独立 client | ✓ VERIFIED | 回归：02-04 artifacts 4/5、key-links 2/3，未通过的两项与上次相同且仍为 pattern 位置误报（`trino/mod.rs:194 base.join("v1/statement")`；`lib.rs:124/135/146 Self::Trino(e)`、`online.rs:393 AnyEngine::Trino(...)`）；全量测试通过 |
| 10 | 02-05：fixture 替身、ONT-V17 定位、单绑定错误隔离（V19 + exit 3）、无 fixture 绑定提示 V20 | ✓ VERIFIED | 回归：02-05 artifacts 5/5、key-links 5/5；全量测试通过 |
| 11 | 02-06：不可用分类、有界重试、POST 只在可证明未接受时重试、超时 / 断连 / 上限取消 | ✓ VERIFIED | 回归：02-06 artifacts 3/3、key-links 2/2；`trino_resilience` 在本次全量运行中通过 |
| 12 | 02-07：ONT-E13..E16 与 ONT-W17（含不可读 fixture 不静默） | ✓ VERIFIED | 回归：02-07 artifacts 4/4、key-links 4/5，未通过项与上次相同（`e14.rs` 经 `lint/binding/mod.rs:33 binding_def` 间接使用 `SourceBinding`，误报） |
| 13 | 02-08：drift 库函数、D-13 兼容表、文档与代码一致 | ✓ VERIFIED | 回归：02-08 artifacts 4/4、key-links 2/2；`doc_type_table` 通过；真实 Trino 上 `drift_is_healthy_on_ci_trino ... ok`；`docs/source-binding.md:235`、`:255` 已同步 V17 可为 info（ADR-0022） |
| 14 | 02-09：`--server-config`、退出码 3、密钥不打印、trace token、`--check-pk` 排除 NULL、`--check-pk` 无 `--online` 为用法错误 | ✓ VERIFIED | 本次实跑 `validate --check-pk`（无 `--online`）exit 2；`--online --check-pk --format json` 输出 `[]` exit 0；WR-C03 新增 `cli_online_trino.rs:271 ignored_fixture_info_passes_deny_warnings`（`--deny-warnings` 下 exit 0、唯一诊断 `ONT-V17` / `info`）通过；`online.rs:469-471` 为 `Severity::Info` |
| 15 | 02-10 完成门禁（E21）：PR `integration` 与首次 nightly 实跑通过，golden 按首跑定稿、删除 `PROVISIONAL` | ✓ VERIFIED | `rg -n PROVISIONAL crates ci .github` 无输出；`4a0c106` 把两处注释改为 `// Confirmed against the first CI run, run 39219827829 (E21)`，golden 值未改（首跑原样通过）；`describe_types_golden`、`decode_golden` 在 PR、main push 与 nightly 483 / 484 中均 ok |

**Score:** 15/15 truths verified（0 present-behavior-unverified）

### Required Artifacts

`gsd-tools query verify.artifacts` 对 10 个 PLAN：50 个 artifact 中 49 个通过，与上次相同。唯一未通过项为 `crates/ontoforge-engine/src/trino/client.rs` 缺 pattern `v1/statement`：URL 拼接在 `trino/mod.rs:184-194`（`base.join("v1/statement")`），判为 pattern 位置误报，artifact ✓ VERIFIED。

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/ontoforge-engine/` | 引擎 crate（printer / sqlite / trino / bind_check / pk_check / config） | ✓ VERIFIED | 源码自上次核查未改动；本机与 CI 测试全绿 |
| `crates/ontoforge-schema/src/ir/binding.rs`、`validate/bindings.rs`、`lint/binding/*` | 绑定定义层与类型契约 lint | ✓ VERIFIED | 未改动，快照测试通过 |
| `crates/ontoforge/src/online.rs` | `validate --online` / `--server-config` / `--check-pk` | ✓ VERIFIED | WR-C03 唯一改动：ignored-fixture 诊断 `Severity::Info`（`05362e5`） |
| `docs/source-binding.md` | 绑定规范 | ✓ VERIFIED | V17 严重级描述与 ADR-0022 一致 |
| `.planning/adr/0021-source-binding-files-and-backing.md`、`.planning/adr/0022-ignored-fixture-diagnostic-is-info.md` | 决策记录 | ✓ VERIFIED | 均 `status: accepted`；0022 修订 0021 第 7 条 |
| `.github/workflows/ci.yml`、`nightly.yml`、`ci/**` | Linux 集成与 nightly | ✓ VERIFIED | 实跑通过（truth 5）；`seed.sql` 用 `\cd :fixtures`（`a4fdfea`）；`iceberg.properties` `local.location=/tmp/iceberg`、`default-warehouse-dir=local:///warehouse`（`2ecdf66`） |
| `crates/ontoforge/tests/private_fixtures.rs` | 私有 fixture 入口（ENGINE-05） | ✓ VERIFIED | `#[ignore]`，本机列为 1 个 ignored；`.github/workflows` 不引用 |
| `examples/ontologies/aviation/bindings/dev/**` | 示例绑定与 fixture | ✓ VERIFIED | 实跑 5 个绑定在线检查通过 |

### Key Link Verification

`gsd-tools query verify.key-links`：28 条中 26 条通过，与上次相同；2 条未通过均为 pattern 误报：

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `crates/ontoforge-engine/src/lib.rs` | `trino/mod.rs` | `AnyEngine::Trino` | ✓ WIRED | `lib.rs:124/135/146` 用 `Self::Trino(e)`；`online.rs:393` 构造 `AnyEngine::Trino(engines.ontology)` |
| `lint/binding/e14.rs` | `loader/sourcemap.rs` | `DefKind::SourceBinding` | ✓ WIRED | `e14.rs:3/23` 调用 `binding_def`（`lint/binding/mod.rs:33`） |
| `online.rs` | `bind_check::compare_binding` / `check_drift` | 共用比较 | ✓ WIRED | 真实 Trino 上 `validate_online_against_ci_trino` 与 `drift_is_healthy_on_ci_trino` 通过 |
| `online.rs` | `pk_check::check_primary_keys` | `--check-pk` | ✓ WIRED | 真实 Trino 上 `check_pk_finds_duplicate_and_null_keys_on_ci_trino` 通过 |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `online.rs::compare_all` | `described` | `Engine::describe_tables`（SQLite：fixture 建表；Trino：`information_schema.columns`） | 是（SQLite 本次实跑；真实 Trino 经 CI `validate_online_against_ci_trino`） | ✓ FLOWING |
| `online.rs::check_keys` | V18 诊断 | `pk_check` 两条 printer 生成的查询 | 是（真实 Trino 经 CI `check_pk_finds_duplicate_and_null_keys_on_ci_trino`） | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| 全量测试（只跑一次） | `cargo test --workspace --locked` | EXIT=0；55 个 test result，595 passed / 0 failed / 12 ignored（上次 594，新增 WR-C03 测试） | ✓ PASS |
| clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | `Finished`，exit 0，无 warning | ✓ PASS |
| fmt | `cargo fmt --all --check` | exit 0 | ✓ PASS |
| 离线校验 | `ontoforge validate examples/ontologies/aviation` | `ok: ontology \`aviation\` with 5 object type(s), profile \`dev\``，exit 0 | ✓ PASS |
| lint clean | `ontoforge lint examples/ontologies/aviation`（及 aviation-json） | `has no lint findings`，exit 0 | ✓ PASS |
| SQLite 替身在线校验 | `ontoforge validate examples/ontologies/aviation --online` | `5 binding(s) checked online`，exit 0 | ✓ PASS |
| `--check-pk` JSON | `ontoforge validate examples/ontologies/aviation --online --check-pk --format json` | `[]`，exit 0 | ✓ PASS |
| 用法错误 | `ontoforge validate examples/ontologies/aviation --check-pk` | exit 2 | ✓ PASS |
| 真实 Trino 集成（PR） | `gh run view 39220900176` integration 日志 | `trino_it` 9 passed、`cli_trino_it` 2 passed | ✓ PASS |
| 真实 Trino 集成（main HEAD） | `gh run view 39222552400`（push，`8bf3de2`） | 3 个 job 全 success；integration 9 + 2 passed | ✓ PASS |
| nightly Iceberg | `gh run view 39221684753` | 483 / 484 均 success，各 9 + 2 passed，`iceberg_table_is_readable ... ok` | ✓ PASS |

### Probe Execution

本 phase 未声明 `scripts/*/tests/probe-*.sh`，仓库中也不存在此类 probe。Step 7c: SKIPPED（无 probe）。

### Requirements Coverage

PLAN frontmatter 声明的 ID 并集为 BIND-01、BIND-02、BIND-03、ENGINE-01、ENGINE-02、ENGINE-03、ENGINE-04、ENGINE-05，与 REQUIREMENTS.md 中映射到 Phase 2 的 8 个 ID 完全一致，无 ORPHANED。

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| BIND-01 | 02-01、02-03 | 独立 `SourceBinding` 文件 + 必填 `backing` | ✓ SATISFIED | truth 1、6、8 |
| BIND-02 | 02-03、02-07、02-08 | 类型契约违反时 lint 报 E | ✓ SATISFIED | truth 1、12、13 |
| BIND-03 | 02-05、02-08、02-09 | `validate --online`、`--check-pk`、drift 降级模型 | ✓ SATISFIED（Phase 2 范围） | truth 2、10、13、14；启动 / 定时 drift 执行按 traceability 属 Phase 7 |
| ENGINE-01 | 02-04、02-06、02-09、02-10 | 自写 Trino client、超时、断连取消、trace token、两个身份 | ✓ SATISFIED | truth 3、9、11；真实 Trino 服务端取消 `timeout_cancels_server_query` 在 CI 通过 |
| ENGINE-02 | 02-04、02-06、02-09 | fail-closed `EngineUnavailable` | ✓ SATISFIED（Phase 2 范围） | truth 3、11；`/health` 与 describe 暴露按 traceability 属 Phase 7 API-01 |
| ENGINE-03 | 02-02、02-10 | 自有 printer，proptest 与真实执行验证 | ✓ SATISFIED | truth 4；`printer_corpus_round_trips` 在真实 Trino 上通过 |
| ENGINE-04 | 02-02、02-05 | SQLite 替身支撑 Windows `cargo test` | ✓ SATISFIED（Phase 2 范围） | 595 passed / 0 failed；`serve --dev` 属 Phase 7 DX-01 |
| ENGINE-05 | 02-10 | Linux CI Trino + PG，nightly Iceberg，私有 fixture 入口 | ✓ SATISFIED | truth 5；私有 fixture 入口 `private_fixtures.rs` 与 CONTRIBUTING.md 存在。SQL Server 实测依赖仓库外私有数据，按设计不进公开仓库，本报告无该运行记录（见 Info） |

REQUIREMENTS.md 中 ENGINE-01 / 03 / 05 仍为 `[ ]`、traceability 为 `Pending`，与本报告结论不一致，需在 phase 收尾时更新（见 Anti-Patterns Info）。

### Anti-Patterns Found

扫描自上次核查（`3fe5ea1`）以来修改的 9 个非规划文件（`nightly.yml`、`seed.sql`、`iceberg.properties`、`trino_it.rs`、`online.rs`、`cli_check_pk.rs`、`cli_online_trino.rs`、`cli_trino_it.rs`、`docs/source-binding.md`）。

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| （上述 9 个文件） | — | `TBD` / `FIXME` / `XXX` / `TODO` / `HACK` / `PLACEHOLDER` / `PROVISIONAL` | — | 无匹配，debt marker 门禁通过 |
| `.planning/REQUIREMENTS.md` | 32、34、36、196、198、200 | ENGINE-01 / 03 / 05 仍标 Pending | ℹ️ Info | 状态文档未从产物派生；phase 收尾时勾选。注意 REQUIREMENTS.md 在 `covered_files` 中，更新后 `covered_digest` 会失效，需重新计算 |
| `.planning/ROADMAP.md` | 15 | Phase 2 checkbox 仍为 `[ ]` | ℹ️ Info | phase 收尾时更新 |
| `02-REVIEW-FIX.md` | frontmatter、17-19、151、155、204 | 仍为 `status: partial`、WR-C03 skipped、WR-C07 `pending-ci` | ℹ️ Info | 与现状矛盾：WR-C03 已由 ADR-0022 落地，WR-C07 已在 CI 通过（`02-UAT.md` 测试 5、6）；建议回填或加指针 |
| `.planning/adr/0021-source-binding-files-and-backing.md` | — | 无指向 ADR-0022 的修订标记 | ℹ️ Info | ADR 不可变原则下可接受；读 0021 第 7 条的人不会看到已被修订 |
| `02-VALIDATION.md` | frontmatter | `status: draft`、`nyquist_compliant: false` | ℹ️ Info | 与上次相同，未回填；不影响代码结论 |
| `.planning/ROADMAP.md` | Phase 7 SC | `/health` 与 describe（ENGINE-02）、启动 / 定时 drift（BIND-03）指向 Phase 7，但 Phase 7 SC 未明文覆盖 | ℹ️ Info | 与上次相同；Phase 7 plan 时补进 SC |
| `crates/ontoforge/src/online.rs` | — | 不可达 Trino + 较短 `--timeout` 在 Windows 上报 `the query timed out` 而非 `Trino is unavailable` | ℹ️ Info | 与上次相同；两条路径都是 ONT-V19 + exit 3，fail-closed 成立 |

### Human Verification Required

无。上次列出的 6 项均已解决，证据如下（逐项本次复核，非沿用 `02-UAT.md` 的叙述）：

| # | 上次项目 | 结论 | 复核证据 |
|---|----------|------|----------|
| 1 | PR `integration` 作业首跑 | 已解决 | run `39219827829`、`39220900176`（pull_request）与 `39222552400`（push `8bf3de2`）conclusion `success`；integration 日志 `trino_it` 9 passed、`cli_trino_it` 2 passed，ignored 计数 9 / 2 |
| 2 | 首次 nightly（483、484） | 已解决 | 首跑 run `39220997866`（`f36eb3a`）两个 job failure，`report-failure` success 并开出 issue #3（已 CLOSED），说明失败通报路径可用；修复 `2ecdf66` 后 run `39221684753` 两个 iceberg job success，`ONTOFORGE_TRINO_ICEBERG: 1`，`iceberg_table_is_readable ... ok`。该 run 是对分支 `fix/02-nightly-iceberg-local` 的 workflow_dispatch，非 main 上的定时触发；`2ecdf66` 与 main HEAD 的非规划内容 diff 为空 |
| 3 | golden 定稿（E21） | 已解决 | `rg -n PROVISIONAL crates ci .github` 无输出；`4a0c106` 仅改注释，golden 值经首跑原样通过 |
| 4 | WR-C06 镜像 digest | 已解决 | 日志 `Digest: sha256:2d2b8998...`（postgres 17.11）、`sha256:db58cc93...`（trino 483）拉取成功；nightly `iceberg (484, sha256:2704eb53...)` success |
| 5 | WR-C07 真实 Trino 查重 | 已解决 | `check_pk_finds_duplicate_and_null_keys_on_ci_trino ... ok`（PR、main push、nightly 483 / 484） |
| 6 | WR-C03 决策 | 已解决 | 用户选 (a)（`02-UAT.md` 测试 6）；ADR-0022 `status: accepted`（`4950d00`）；`online.rs:471 Severity::Info`（`05362e5`）；测试 `4d681aa`：本机 `ignored_fixture_info_passes_deny_warnings` 通过，真实 Trino 上 `validate_online_against_ci_trino` 断言 `severity == "info"` 通过；`docs/source-binding.md:235/255` 同步 |

### Gaps Summary

无 gap。ROADMAP Phase 2 的 5 条 Success Criteria 与 10 个 PLAN 的 must-have 全部有实现、有接线、有通过的测试，真实 Trino 部分由 main HEAD `8bf3de2` 的 push CI（run `39222552400`）与 PR CI 直接证实，nightly Iceberg 由 run `39221684753` 证实（所测内容与 main HEAD 一致）。本机门禁：`cargo test --workspace --locked` 595 passed / 0 failed / 12 ignored，clippy `-D warnings` 与 `fmt --check` 均通过。

剩余事项都是规划文档未回填（REQUIREMENTS / ROADMAP 勾选、`02-REVIEW-FIX.md`、`02-VALIDATION.md`），不影响目标达成。另有两点提示：nightly 尚无 main 上的 cron 定时运行记录（只有修复前在 main 上手动触发、已失败的 run `39220997866`，和修复后在分支上手动触发的成功 run）；ENGINE-05 的 SQL Server 私有 fixture 实测不在公开仓库内，本报告无该运行记录。

---

_Verified: 2026-10-19T08:32:39Z_
_Verifier: Claude (gsd-verifier)_
