---
phase: 02-source-binding-engine
iteration: 1
parts: 3
status: partial
fixed: 20
skipped: 1
pending_ci: 0
skipped_ids: [WR-C05]
pending_ci_ids: []
resolved_after_iteration: [WR-C03, WR-C07]
updated: 2026-10-19
---

# Phase 2 Review Fix

修复范围：02-REVIEW.md 的 2 条 Critical + 19 条 Warning，分 3 轮顺序执行（schema+critical / engine / cli+ci）。Info 未纳入。

- WR-C03 skipped：修法与 ADR-0021 第 7 条冲突，待用户决定。
- WR-C05 skipped：不可复现（超时分支已同步 cancel），加了守护测试 bc506a6。
- WR-C07 pending-ci：真实 Trino 的 `--check-pk` 检测路径待首次 CI `integration` 作业确认。
- WR-S05 部分修复：引擎侧拒绝越界 decimal；validate 侧保持 ADR-0021（lint E13），Phase 7 serve 时再评估。

门禁（HEAD c474b39，本机 Windows）：cargo test 594 passed / 0 failed / 12 ignored；clippy -D warnings、fmt --check 通过。

---

# Part 1

# Phase 02: Code Review Fix Report (part 1)

**Fixed at:** 2026-10-17T02:06:26Z
**Source review:** .planning/phases/02-source-binding-engine/02-REVIEW.md
**Iteration:** 1
**Scope:** CR-C01, CR-S01, WR-S01, WR-S02, WR-S03, WR-S04, WR-S05。其余 finding 留给后续 fixer pass，本轮没有改动。

**Summary:**
- Findings in scope: 7
- Fixed: 7（其中 WR-S05 只做了部分修复，原因见下）
- Skipped: 0

工作方式：按 orchestrator 指令直接在主工作树 `gsd/phase-02-source-binding-engine` 分支上修改，没有建 worktree。每个 finding 一个原子 commit。每个回归测试都做过「修复前必失败」验证：把被修文件临时换成 `HEAD` 版本（`git show HEAD:<file> > <file>`）跑新测试，确认失败后再恢复修复版。

## Fixed Issues

### CR-C01: server 配置错误的 human 输出泄漏明文密钥

**Files modified:** `crates/ontoforge-engine/src/config.rs`, `crates/ontoforge-engine/tests/server_config.rs`, `crates/ontoforge/src/online.rs`, `crates/ontoforge/src/render.rs`, `crates/ontoforge/tests/cli_online_trino.rs`
**Commit:** cb60e7c
**Applied fix:**
- `select_backend` 不再把 `ontoforge-server.toml` 的文本注册进 `SourceMap`，human renderer 因此拿不到 snippet，被拒的 `password` / `token` 行和 `url` userinfo 行都不会再打印出来。
- `load_server_config` 的错误类型改为 `(Vec<ConfigError>, Option<String>)`，失败时把已解析的文本一起带回，只用来算行列号。原来的第二次读取（TOCTOU）随之删除。
- `render::to_report`：有行号但没有 source 文本的诊断，在消息后追加 `at file:line:col`，定位信息仍然保留。
- 回归测试 `rejected_plaintext_secret_never_printed`：在 human 和 JSON 两种格式下分别验证 `password = "<SECRET>"`、`token = "<SECRET>"`、`url = "https://alice:<SECRET>@..."` 三种写法，断言 stdout / stderr 都不含密钥，且 human 输出仍带 `<cfg>:` 定位。修复前该测试失败，输出里出现 `6 │ password = "Hunter2SuperSecret"`。另补了单测 `located_diagnostic_without_source_names_the_place`，以及 `load_server_config` 错误分支带回文本的断言。

### CR-S01: fixture 路径经中间目录 symlink / junction 逃逸出 ontology 目录

**Files modified:** `crates/ontoforge-schema/src/loader/mod.rs`, `crates/ontoforge-schema/tests/bindings.rs`, `crates/ontoforge-engine/src/sqlite/fixture.rs`, `crates/ontoforge-engine/tests/sqlite_engine.rs`
**Commit:** 82e6dde
**Applied fix:**
- 新增 `loader::check_beneath(root, rel)`：从 `root` 到目标逐个路径分量做 `symlink_metadata`，遇到 `is_symlink()` 就拒绝（Windows 上 junction 同样算 symlink）。作为纵深防御，还会 canonicalize 目标和 root，并校验前缀包含关系。新增 `loader::read_beneath` = `check_beneath` + `read_limited(.., false, ..)`。
- schema 的 `probe_disk`（V16 存在性 / 大小检查）、`read_text`（定义文件和 fixture 表头），以及 engine SQLite 替身的 `load_table`（`sqlite/fixture.rs`），全部改走这条检查。
- 没有检查 `FILE_ATTRIBUTE_REPARSE_POINT` 全集。原因是 OneDrive 等云文件也是 reparse point，全部拒绝会误伤，只拒绝 name-surrogate 类（symlink / junction）。
- 回归测试：schema 的 `fixture_behind_directory_link_is_v16` 断言 `ONT-V16` "not a regular file" 且 ontology 为 `None`；engine 的 `fixture_behind_directory_link_is_refused` 绕过 schema 直接把计划交给 `SqliteEngine::from_fixtures`，断言加载被拒。两者都用 `#[cfg(windows)]` helper（`cmd /C mklink /J`）和 `#[cfg(unix)]` helper（`std::os::unix::fs::symlink`）建目录链接。在本机 Windows 上修复前两个测试都失败，junction 逃逸已复现。
- 残留风险：检查与 open 之间仍有 TOCTOU 窗口。要彻底关闭需要 `cap-std` 一类 beneath-open 实现，本轮没有引入新依赖。

### WR-S01: `fixture_header` 与 engine 的表头语义不一致

**Files modified:** `crates/ontoforge-schema/src/loader/fixture.rs`, `crates/ontoforge-engine/src/sqlite/fixture.rs`
**Commit:** e6f07aa
**Applied fix:** `fixture_header` 对所有格式先剥 UTF-8 BOM，并拒绝空列名和忽略大小写后重名的列（报错文案沿用 engine 原文）。engine 删掉自己那份规则，只为解析行数据保留剥 BOM。回归单测 `fixture_header_matches_the_engine_rules` 覆盖带 BOM 的 JSON、`id,,name`、`id,ID`，以及 JSON 空键和大小写重名键。

### WR-S02: 位于 `bindings/<p>/fixtures/` 之外的 fixture 会被当成定义文件解析

**Files modified:** `crates/ontoforge-schema/src/loader/discover.rs`, `crates/ontoforge-schema/tests/bindings.rs`, `docs/source-binding.md`
**Commit:** 28a1a16
**Applied fix:** 采用审查推荐的方案 (a)：`fixture_path` 要求解析结果落在 `bindings/<profile>/fixtures/` 下，否则报 `ONT-V16`，消息里给出要求的位置。允许子目录，也允许引用其他 profile 的 fixtures 目录。单测和 `fixture_reference_rules` 补了 `rows.json`、`../../data/rows.csv` 等用例。`docs/source-binding.md` 原来写的是「放在别处的 `.json` fixture 会被当作定义文件读取」，已同步改为「解析到别处报 `ONT-V16`」，避免文档与代码矛盾。

### WR-S03: link binding 写 `from: {}` 不写 `column` 也能通过

**Files modified:** `crates/ontoforge-schema/src/validate/bindings.rs`, `crates/ontoforge-schema/tests/bindings.rs`, `crates/ontoforge-schema/src/lint/binding/e15.rs`
**Commit:** d9239b0
**Applied fix:** link 的两端都必须写显式的 `column`，否则报 `ONT-V16`。回归用例放在 `link_columns_need_from_and_to`：`from: { collation: caseSensitive }`。现有单测 `e15_link_end` 用的正是这种被禁写法（两端只写 `collation`），已补上 `column`，测试意图不变。说明：第一次提交时测试门禁没有串进提交链，`e15_link_end` 失败的状态被提交了；随后用 `git commit --amend` 把 `e15.rs` 的修正并进同一个 commit，最终 hash 为 d9239b0。

### WR-S04: 同一 binding 内多个 key 映射到同一物理列不报错

**Files modified:** `crates/ontoforge-schema/src/validate/bindings.rs`, `crates/ontoforge-schema/tests/bindings.rs`
**Commit:** 2b13610
**Applied fix:** 新增 `distinct_columns`：对 `ontology.bound_columns(binding)` 按 `column.to_lowercase()` 去重，冲突时报 `ONT-V16`，消息为 "`a` and `b` are both read from column `x`"。定位优先指向冲突双方中显式写了 `column` 的那个，两个都显式时指向后出现的；都不显式时（只有大小写不同的属性名才会出现）指向 `target`。回归测试 `two_keys_on_one_column_is_v16` 覆盖属性显式指向另一属性默认列（大小写不同）和 link 两端同列两种情况。

### WR-S05: decimal precision / scale 合法性只由可降级的 lint E13 保证

**Files modified:** `crates/ontoforge-engine/src/sqlite/fixture.rs`, `crates/ontoforge-engine/tests/sqlite_engine.rs`
**Commit:** b65ed21
**Status:** fixed: partial, requires human verification
**Applied fix:** 审查的首选方案是把范围检查移到 `validate/objects.rs` 作为 V06 硬错误，这和锁定决策冲突，没有采用：ADR-0021 第 4 条写明「合法范围 1 ≤ precision ≤ 38、0 ≤ scale ≤ precision（ONT-E13）」，REQUIREMENTS BIND-02 写明「违反时 lint 报 E」，02-CONTEXT 也把 ONT-E13 列为 lint 规则。实际采用的是审查给出的备选方案：SQLite 替身在 `load_table` 开头检查每个 decimal 列（含 array 元素），`precision` 不在 1..=38 或 `scale > precision` 时返回 `FixtureError`，不再把 1000 静默钳成 `decimal(255, ..)`。回归测试 `out_of_range_decimal_type_is_refused` 用纯表头 CSV（没有单元格需要转换，所以原有的逐格检查不会触发）验证 `decimal(1000,2)`、`decimal(0,0)`、`decimal(12,13)` 和 `array<decimal(40,2)>`，修复前测试失败。仍未覆盖的部分：`ontoforge validate` 和服务端加载路径仍然接受越界 decimal。是否要修改 ADR-0021，把范围检查提升为 validate 硬错误，需要用户决定。

## Verification

所有门禁都在**主工作树**（`D:/DataSpellProjects/ontoforge`，分支 `gsd/phase-02-source-binding-engine`，HEAD `b65ed21`）的 Windows 11 本机上运行，结果可以直接在该 checkout 复现：

- `cargo test --workspace --locked`：exit 0；55 个 test binary 的 `test result` 全部为 `ok`，合计 passed=580、failed=0、ignored=11。
- `cargo clippy --workspace --all-targets --locked -- -D warnings`：exit 0（`Finished`，无 warning）。
- `cargo fmt --all --check`：exit 0，无输出。
- `git status --short` 只剩原有的 untracked 项（`.review-diagnostics/`、`docs/README.md`、`docs/foundry/`、`docs/images/`），没有动过。


_Fixed: 2026-10-17T02:06:26Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_

---

# Part 2

# Phase 2 Review Fix — Part 2（engine）

fixer 在写本报告前被会话结束中断；7 条修复均已逐条提交，本报告由 orchestrator 依据提交记录补写，门禁由 orchestrator 在 `5e1dd3c` 上重跑。

| Finding | Commit | 修复 |
|---------|--------|------|
| WR-E01 | 92df48d | 误填进 `*_env` / `*_file` 键的密钥值不再回显到错误信息 |
| WR-E02 | f6482f9 | `[trino].client_tags` 随每个查询发送 |
| WR-E03 | 01814db | SQLite 替身按值比较 decimal 与 timestamp，与 Trino 语义一致 |
| WR-E04 | 65fe07b | SQLite 替身遵守 `ctx.timeout` |
| WR-E05 | b6b8dbe | 限制单个 Trino 页面可占用的内存 |
| WR-E06 | d868cb4 | `PhysicalType::parse` 限制 `array(...)` 嵌套深度 |
| WR-E07 | 5e1dd3c | `SourceUnreadable` 的可对外暴露字段不含引擎原文 |

## 门禁（orchestrator 在本机 Windows 实跑）

- `cargo test --workspace --locked`：590 passed，0 failed，11 ignored
- `cargo clippy --workspace --all-targets --locked -- -D warnings`：exit 0
- `cargo fmt --all --check`：exit 0

未逐条核对每个回归测试在修复前失败（fixer 中断，过程记录不在磁盘上）。

---

# Part 3

# Phase 02: Code Review Fix Report (part 3, CLI / CI)

**Source review:** `.planning/phases/02-source-binding-engine/02-REVIEW.md`（CLI / CI 区的 WR-C01～WR-C07）
**Iteration:** 1

按 orchestrator 指令直接在主工作树 `gsd/phase-02-source-binding-engine` 分支上修改，没有建 worktree。每条 finding 一个原子 commit。代码类修复都先写回归测试，确认修复前失败，再改代码。

| Finding | 结果 | Commit |
|---------|------|--------|
| WR-C01 | fixed | 79f9649 |
| WR-C02 | fixed | 3564d4d |
| WR-C03 | skipped（与 ADR-0021 第 7 条冲突）；2026-10-19 resolved，见 Addendum | 4d681aa, 05362e5, 4950d00 |
| WR-C04 | fixed | 5c640ab |
| WR-C05 | skipped（不可复现，补了守护测试） | bc506a6 |
| WR-C06 | fixed | 3cfdc66 |
| WR-C07 | pending-ci；2026-10-19 verified on CI，见 Addendum | c474b39 |

## Fixed Issues

### WR-C01: `--online` 且无 profile 时 summary 输出 "0 problem(s) found"

**Files modified:** `crates/ontoforge/src/online.rs`, `crates/ontoforge/tests/cli_online.rs`
**Commit:** 79f9649
**Applied fix:** `run_online` 先确认有 profile，再从 `out` 里 `take()` ontology。没有 profile 时 ontology 留在 `out` 里，`lib.rs` 的 `(Some(ontology), None)` 分支能够命中，输出 `ok: ontology ..., no profile selected, nothing to check online`。两个提前返回共用一个 `offline_only` 闭包，`profile` 字段的语义与原来一致。
**Regression test:** `online_without_profile_says_so`：manifest 不写 `defaultProfile`，只有一个 `editOnly` 类型，断言 exit 0、stderr 含 `no profile selected`、不含 `problem(s) found`。修复前失败，stderr 为 `0 problem(s) found`。
**Not changed:** 审查提出的附带问题（显式传了 `--server-config` / `--check-pk` 却没有 profile 时是否应报 V20）属于产品决策，本轮没有改，保持 exit 0。

### WR-C02: `--check-pk` 阶段的引擎级失败丢弃已算出的 V17/V18

**Files modified:** `crates/ontoforge/src/online.rs`, `crates/ontoforge/tests/cli_check_pk.rs`
**Commit:** 3564d4d
**Applied fix:** `check_keys` 的返回类型从 `Result<Vec<Diagnostic>, Diagnostic>` 改为 `Vec<Diagnostic>`。遇到引擎级错误时，把 V19 追加到已收集的结果后面，然后停止扫描（引擎已不可用，继续扫只会得到同样的失败）。`compare_all` 不再对它使用 `?`。V19 仍然决定 exit 3。describe 阶段的失败仍按原逻辑，只报那一条 V19。
**Regression test:** `check_pk_engine_failure_keeps_earlier_findings`（wiremock）：同一 catalog 下有 Airport 和 Flight 两个类型，Airport 先扫描，返回重复键 `FRA ×2`；Flight 的 PK 查询返回 401。断言 exit 3，结果中有 1 条 V19（authentication）、1 条位于 `airport.yaml` 的 V18、2 条 fixture-ignored V17 warning。修复前失败，输出只剩 V19。

### WR-C04: `--timeout 0` 被接受并以 exit 3 报告

**Files modified:** `crates/ontoforge/src/cli.rs`, `crates/ontoforge/tests/cli_online.rs`, `docs/source-binding.md`
**Commit:** 5c640ab
**Applied fix:** `--timeout` 加上 `value_parser = clap::value_parser!(u64).range(1..)`，传 0 时由 clap 报用法错误并 exit 2，与 server 配置中 `query_timeout_ms = 0` 被拒绝的行为一致。帮助文本和 `docs/source-binding.md` 第 257 行都注明了「至少为 1」。
**Regression test:** `zero_timeout_is_a_usage_error`：断言 exit 2，stderr 含 `--timeout`。修复前失败：SQLite 替身路径直接 exit 0。

### WR-C06: CI 容器镜像只钉 tag 不钉 digest

**Files modified:** `.github/workflows/ci.yml`, `.github/workflows/nightly.yml`, `CONTRIBUTING.md`
**Commit:** 3cfdc66
**Applied fix:** 2026-10-17 通过 Docker Hub registry HTTP API（`auth.docker.io/token` 加 `HEAD /v2/<repo>/manifests/<tag>` 的 `Docker-Content-Digest`）取得以下真实 digest：

| 镜像 | digest |
|------|--------|
| `postgres:17.11`（即当时 `postgres:17` 所解析到的 digest，逐个比对 17.5～17.11 后确认） | `sha256:2d2b8998d31037bf721cfdf764d76ba74171b4fab3431b7f72c27c56ddbdf9e3` |
| `trinodb/trino:483` | `sha256:db58cc93e593a2706553745f276bb119c9810e69918be56ecde088ba7ccb0534` |
| `trinodb/trino:484` | `sha256:2704eb53935af5e582d63d6033e9bf99d20c325e9b0e30b1016ef59a4773979f` |

三个 digest 都是 multi-arch index，已确认其中包含 `linux/amd64`。nightly 的 matrix 改为 `include: [{trino, digest}]`，`TRINO_DIGEST` 通过 env 传入，拼成 `trinodb/trino:$TRINO_VERSION@$TRINO_DIGEST`。CONTRIBUTING 的手动复现命令使用相同的钉住值。

**Verification:** 本机没有 `actionlint`，只用 PyYAML 解析了两个 workflow，结构正确（jobs 与 matrix include 都符合预期）。镜像能否实际拉取，要到第一次 CI 运行才能确认。
**Residual:** 仓库没有 Dependabot / Renovate 配置，digest 需要手动更新。nightly.yml 的注释写明了刷新方法（`docker buildx imagetools inspect` 输出中的 `Digest:` 行）。

## Pending CI

### WR-C07: 真实 Trino 上的 `--check-pk` 只跑过「无发现」路径

**Files modified:** `ci/pg/seed.sql`, `crates/ontoforge/tests/cli_trino_it.rs`, `CONTRIBUTING.md`
**Commit:** c474b39
**Status:** pending-first-CI
**Applied fix:** seed 末尾新增 `aviation.flights_dup(flight_id varchar(16), status text)`。这张表没有 PRIMARY KEY 约束，数据为 `F-DUP-1` ×2、`F-UNIQUE` ×1、NULL ×2。新增 `#[ignore]` 测试 `check_pk_finds_duplicate_and_null_keys_on_ci_trino`：在临时 ontology 里把 `Flight` 绑定到这张表，使用 `ci/ontoforge-server.ci.toml` 跑 `--check-pk`，断言 exit 1，并且恰好有两条 V18：一条含 `1 duplicate primary key value(s)` 和 `` `F-DUP-1` ×2 ``、不含 `F-UNIQUE`，另一条含 `2 row(s) with a NULL primary key`。aviation 原有的 5 张表都是在 `trino_it` 中显式列出的，新表不影响已有测试。
**本机能做的验证：** 测试能编译，`--ignored --list` 列出了 2 个 ignored 测试（CI 的「Ignored tests are actually there」检查会继续通过）。另外用同一个 ontology 加内容相同的 CSV fixture 在 SQLite 替身上跑了一遍，两条 V18 的消息文本与断言完全一致，exit 1。真实 Trino 方言下的查询和 bigint/varchar 解码，只能在第一次 CI `integration` 作业中确认。

## Skipped Issues

### WR-C03: Trino 后端对每个带 fixture 的 binding 发 V17 warning

**File:** `crates/ontoforge/src/online.rs`（`ignored_fixtures`）
**Reason:** 与锁定决策冲突。ADR-0021 第 7 条写明「配置映射到 Trino 而绑定带 `fixture` 时给 W（open question 2）」，severity 被锁定为 warning。审查给出的方案（降为 `Severity::Info`，或只在 stderr summary 提一句）都会推翻这条决策。只换成独立 code 而保持 warning，只能解决「V17 语义被稀释」，`--deny-warnings` 恒为 exit 1 的问题依旧，而且要在 code 表里新增一个号段，同样需要拍板。
**Original issue:** aviation 一类同时带 fixture 的 ontology，用 `--server-config` 跑时每个 binding 都会多一条 V17 warning，`--deny-warnings` 因此恒为 exit 1。
**需要用户决定：** 是否修订 ADR-0021 第 7 条。可选方案：(a) 改为 Info；(b) 分配独立 code，仍为 warning；(c) 只在 summary 中计数。

### WR-C05: 超时时的 Trino cancel 随 CLI runtime drop 丢失

**File:** `crates/ontoforge-engine/src/trino/mod.rs:358-364`, `crates/ontoforge-engine/src/trino/cancel.rs`
**Reason:** 不可复现。`TrinoEngine::execute` 在超时分支执行的是 `guard.cancel_now().await`：DELETE 在 `execute` 返回之前同步发出并等待完成（最多 `DELETE_TIMEOUT` = 2 s）。这段代码从 9b6f9c4 起就存在，审查时也是如此。`QueryGuard::drop` 中 fire-and-forget 的 spawn 只在调用方中途丢弃 query future 时才会走到，而 CLI 用 `block_on` 把每个 future 都跑完，不会在中途丢弃。
**Guard test (commit bc506a6):** `timed_out_query_is_cancelled_before_exit`（wiremock）：POST 返回 `nextUri`，该地址的 GET 延迟 30 s，CLI 以 `--timeout 500` 运行。断言 exit 3、V19 含 `timed out`、CLI 在 20 s 内退出，并且 wiremock 恰好收到 1 次 DELETE。当前代码下测试通过。为证明测试能区分对错，临时把超时分支改成只 `drop(guard)`（即审查假设的机制），测试失败，收到的 DELETE 次数为 0；随后已还原该改动，`mod.rs` 没有改动进入提交。

## Verification

所有门禁都在**主工作树**（`D:/DataSpellProjects/ontoforge`，分支 `gsd/phase-02-source-binding-engine`，HEAD `c474b39`）的 Windows 11 本机上运行，可以在该 checkout 直接复现：

- `cargo test --workspace --locked`：exit 0。55 个 `test result: ok`，合计 passed=594、failed=0、ignored=12（新增的 ignored 测试是 `cli_trino_it` 的 WR-C07 用例）。
- `cargo clippy --workspace --all-targets --locked -- -D warnings`：exit 0（`Finished`，无 warning）。
- `cargo fmt --all --check`：exit 0，无输出。
- `git status --short` 只剩原有的 untracked 项（`.review-diagnostics/`、`02-REVIEW-FIX-part1.md`、`02-REVIEW-FIX-part2.md`、`docs/README.md`、`docs/foundry/`、`docs/images/`），这些都没有动过。本报告未提交。


_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_

---

## Addendum (2026-10-19)

- WR-C03 resolved：用户在 UAT 02 测试 6 选定方案 (a)，被忽略的 fixture 诊断降为 Info（code 仍为 ONT-V17），决策见 ADR-0022（`.planning/adr/0022-ignored-fixture-diagnostic-is-info.md`）。Commits：4d681aa（test）、05362e5（fix）、4950d00（ADR）。
- WR-C07 verified：`check_pk_finds_duplicate_and_null_keys_on_ci_trino` 在 CI run 39219827829 的 `integration` 作业通过；nightly run 39221684753 通过。
- 剩余 skipped 只有 WR-C05（不可复现，守护测试 bc506a6），因此 status 保持 `partial`。
