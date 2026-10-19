---
phase: 02-source-binding-engine
reviewed: 2026-10-17T01:19:34Z
depth: standard
diff_base: d1b3c1c
files_reviewed: 99
parts: [cli, schema, engine]
findings:
  critical: 2
  warning: 19
  info: 22
  total: 43
status: issues_found
---

# Phase 2 Code Review

99 个源码文件按区域拆成 3 个 reviewer 并行审查（cli 25、schema 38、engine 36），finding ID 前缀 C- / S- / E- 对应区域。下文为三份报告正文的原样合并。


---

# Part: cli

# Phase 02: Code Review Report (part: CLI / CI)

**Reviewed:** 2026-10-17T01:15:02Z
**Depth:** standard
**Files Reviewed:** 25
**Status:** issues_found

## Narrative Findings (AI reviewer)

## Summary

审查范围：`ontoforge` CLI 的 `validate --online` 路径（`--server-config` / `--check-pk` / `--timeout` / 退出码 3）、profile 选择、workspace `Cargo.toml`、`clippy.toml`、`ci.yml` + `nightly.yml`，以及 `ci/` 下的 Trino / PostgreSQL 配置。

总体：退出码表、V19 隔离（单 catalog 失败不拖垮其它 catalog）、GitHub Actions 权限最小化（`contents: read`，只有 `report-failure` 有 `issues: write` 且不 checkout）、actions 钉 SHA、matrix 值走 `env` 不直接插进 shell，这些都做对了。

主要问题：

1. **CR-C01（已复现）**：server 配置里出现明文 `password` / `token`，或 `url` 带 userinfo 时，human 格式会把含密钥的源码行原样打到 stdout。这正是 plaintext-secret 拒绝规则要防的场景，现有 `secret_never_printed` / `config_errors_are_v20_with_location` 测试只覆盖 JSON 格式，所以没测出来。
2. **WR-C01（已复现）**：`--online` 且没选中 profile 时，`run_online` 在 let-else 里 `take()` 掉了 ontology，summary 分支 `(Some(ontology), None)` 永远到不了，输出 `0 problem(s) found`，exit 0。
3. `--check-pk` 中途遇到引擎级失败时会丢弃已经算出来的 V17/V18；`--deny-warnings` 和 Trino 后端组合时永远失败；`--timeout 0` 没被拒绝；超时 cancel 的 DELETE 可能随 runtime drop 一起丢掉。
4. CI：容器镜像没钉 digest；真实 Trino 上的 `--check-pk` 只跑过「无发现」路径（seed 表有 PRIMARY KEY 约束）。

## Critical Issues

### CR-C01: server 配置错误的 human 输出泄漏明文密钥（password / token / URL userinfo）

**File:** `crates/ontoforge/src/online.rs:351-364`
```rust
        let text = read_limited(path.as_std_path(), true, MAX_FILE_BYTES)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok());
        if let Some(text) = &text {
            sources.add_file(file, text.clone());
        }
```
**Issue:** 配置加载失败时，整份 `ontoforge-server.toml` 文本被注册进 `SourceMap`。`render::to_report`（`render.rs:65-79`）随后把 miette source snippet 渲染出来：带 `here` 标记的那一行加上下文行。明文密钥被拒时，V20 恰好指向 `password` / `token` 这个 key，或指向 `url` key（userinfo 场景），所以密钥值会原样打到 stdout，CI 日志和终端录屏都会留下。本地已复现（`--color never`，默认 human 格式）：

```
  × plaintext secrets are not accepted in ontoforge-server.toml (found `password`)
   ╭─[.../s.toml:6:1]
 5 │ user = "u"
 6 │ password = "SuperSecretPw"
```
```
  × `url` must not carry user information
 2 │ url = "https://alice:Hunter2Secret@127.0.0.1:9"
```
`tests/cli_online_trino.rs:576-589` 的 plaintext 用例只跑 `--format json`（JSON 不带 snippet）。`secret_never_printed` 走的是 `token_env` 正常路径，也覆盖不到。`private_fixtures.rs:91-98` 用 human 格式，失败时会把整个 stdout 打进测试输出，等于把同一泄漏带进私有环境的日志。
另外，第二次 `read_limited` 是 TOCTOU：读到的文本可能和 `load_server_config` 解析时的文本不一致，span 会错位。

**Fix:** server 配置文本不进 `SourceMap`，只用它算行列号（`config_diagnostic` 已经接受 `text` 参数，`line` / `column` 照常填），渲染时不附 snippet。如果要保留 snippet，就必须先把 span 所在行的 value 部分打码。同时让 `load_server_config` 在 `Err` 里带回已读到的文本，避免第二次读取：
```rust
let (config, _text) = load_server_config(path).map_err(|(errors, text)| {
    // Never register the server configuration as a source: a rejected line may hold a
    // secret, and the human renderer prints the lines around a span.
    errors.iter().map(|e| config_diagnostic(e, file, text.as_deref())).collect::<Vec<_>>()
})?;
```
`render::to_report` 里，`sources.text(&d.file)` 拿不到文本时不附 snippet，只给 `file:line:col`。再补一个回归测试：human 格式 + `password = "<SECRET>"` 以及 `url = "https://u:<SECRET>@h"`，断言 stdout/stderr 都不含 `<SECRET>`。

## Warnings

### WR-C01: `--online` 且无 profile 时 summary 分支不可达，输出 "0 problem(s) found"

**File:** `crates/ontoforge/src/online.rs:58` / `crates/ontoforge/src/lib.rs:102-105`
```rust
    let (Some(ontology), Some(profile)) = (out.ontology.take(), profile.clone()) else {
```
**Issue:** `out.ontology.take()` 在模式匹配前就执行了。`profile` 为 `None` 时走 else 分支，取出来的 ontology 随临时 tuple 一起被丢弃，返回的 `out.ontology` 恒为 `None`。于是 `lib.rs` 的 `(Some(ontology), None) if run.code == 0 => "ok: ... no profile selected, nothing to check online"` 永远到不了，用户看到的是自相矛盾的 `0 problem(s) found`，exit 0。本地已复现（manifest 不带 `defaultProfile`，`validate <dir> --online`）。该分支没有测试覆盖。
**Fix:**
```rust
let Some(profile) = profile.clone() else {
    let code = render::exit_code(&out.diagnostics, deny_warnings);
    return OnlineRun { out, code, profile: None, bindings: 0 };
};
let Some(ontology) = out.ontology.take() else { /* same early return */ };
```
另外补一个 CLI 测试，断言 stderr 包含 `no profile selected`。还要决定一件事：用户显式传了 `--server-config` / `--check-pk` 却没有 profile 时，exit 0 是否合理（可以考虑报 V20）。

### WR-C02: `--check-pk` 阶段的引擎级失败会丢弃已算出的 V17/V18

**File:** `crates/ontoforge/src/online.rs:242-245`, `281`
```rust
    if check_pk {
        found.extend(check_keys(engine, ctx, check, &pk_keys).await?);
    }
```
**Issue:** describe 阶段已经成功，`found` 里可能有真实的 V17 error/warning；`check_keys` 里也可能已经积累了前几个 binding 的 V18。任一 binding 的 PK 扫描遇到 `Unavailable` / `Unauthorized`（例如长扫描中 Trino 重启），`?` 和 `return Err(...)` 就把这些全部丢掉，只剩一条位于 `ontology.yaml` 的 V19。`check_online:202` 的注释写的是「A round that never reached Trino reports nothing but the failure」，可这里这一轮已经到过 Trino，和注释的前提不符。operator 修完 Trino 重跑之前，看不到任何 schema 漂移信息。
**Fix:** 让 `check_keys` 返回 `(Vec<Diagnostic>, Option<Diagnostic>)`，或者在 `compare_all` 里把 V19 追加进 `found`，不要直接 `?`：
```rust
if check_pk {
    match check_keys(engine, ctx, check, &pk_keys).await {
        Ok(more) => found.extend(more),
        Err(failure) => found.push(failure), // exit code 3 still follows from the V19
    }
}
```
`check_keys` 遇到引擎级错误时，也应带回已经收集到的 `found`。

### WR-C03: Trino 后端对每个带 fixture 的 binding 发 V17 warning，`--deny-warnings` 因此永远失败，V17 语义也被稀释

**File:** `crates/ontoforge/src/online.rs:441-476`, `197-199`
```rust
            Diagnostic::new(
                codes::V17,
                Severity::Warning,
                format!(
                    "the fixture of `{}` is ignored because profile `{profile}` is checked against Trino",
```
**Issue:** 同时支持 SQLite stand-in 和 Trino 的 ontology（仓库自带的 aviation 就是，5 个 binding 全带 fixture）用 `--server-config` 跑时，每个 binding 都会多一条 warning。`cli_trino_it.rs:79-86` 断言的正是这 5 条 warning。结果是 `validate --online --server-config ... --deny-warnings` 对这类 ontology 恒为 exit 1，CI gate 根本没法开 deny-warnings。另一方面，ONT-V17 的含义是「binding 与物理表不匹配」，「fixture 被忽略」是一条纯运行信息，复用 V17 后，按 code 过滤的下游（JSON 消费者、`--deny-warnings`）分不清真漂移和这条噪声。
**Fix:** 降为 `Severity::Info`（`exit_code` 不受 deny-warnings 影响），或者分配独立 code；最简单的做法是只在 stderr summary 里提一句「N fixture(s) ignored」。相应更新 `cli_trino_it.rs` 和 `cli_online_trino.rs:234-241` 的断言。

### WR-C04: `--timeout 0` 被接受，Trino 路径会以 "engine unavailable" 语义 exit 3

**File:** `crates/ontoforge/src/cli.rs:65-67`
```rust
    #[arg(long, value_name = "MS", requires = "online")]
    pub timeout: Option<u64>,
```
**Issue:** server 配置对 `query_timeout_ms = 0` 报错（`config.rs:332`），CLI flag 却接受 0。`query_context` 把 `ctx.timeout` 设成 `Duration::ZERO`，`tokio::time::timeout(ZERO, ..)` 在第一次 pending 就超时，于是每个 binding 都是 V19 "timed out after 0 ms"，exit 3。这其实是用法错误（应为 exit 2），却被报成引擎故障，CI 脚本据此会误判 Trino 不可用。（SQLite 路径完全不看 timeout，见 IN-C03。）
**Fix:**
```rust
#[arg(long, value_name = "MS", requires = "online",
      value_parser = clap::value_parser!(u64).range(1..))]
pub timeout: Option<u64>,
```

### WR-C05: 超时 / 中断时的 Trino cancel（DELETE）可能随 CLI 的 runtime drop 一起丢掉

**File:** `crates/ontoforge/src/online.rs:171-217`
```rust
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    ...
    runtime.block_on(async {
```
**Issue:** `QueryGuard::drop`（`trino/cancel.rs:61-69`）通过 `Handle::try_current().spawn(send_delete(..))` fire-and-forget 地取消查询。CLI 用的是 `current_thread` runtime：最后一个 binding 的查询超时（`Timeout` 不是 engine-wide，round 会继续并很快结束）后，`block_on` 立即返回，`check_online` 返回时 `runtime` 被 drop，还没被 poll 到的 `send_delete` task 会被直接取消。这意味着最常见的 CLI 超时场景（单 catalog、大表 `--check-pk`）根本不会发出 DELETE，查询还会在 Trino 上继续跑到 `query_max_execution_time`（client timeout + slack）。置信度中等：依据是 tokio runtime drop 语义，未在真实 Trino 上验证。
**Fix:** 在 `block_on` 之后给后台 task 一个有界的收尾窗口：
```rust
let result = runtime.block_on(async { ... });
runtime.shutdown_timeout(Duration::from_secs(2)); // lets pending cancels go out
result
```
或者在 `block_on` 末尾 `tokio::task::yield_now().await` 若干次 / 等待 guard 暴露的 join handle。

### WR-C06: CI 容器镜像只钉 tag 不钉 digest，与 actions 的 SHA 钉住策略不一致

**File:** `.github/workflows/ci.yml:36`, `66`; `.github/workflows/nightly.yml:23`, `62`
```yaml
        image: postgres:17
...
        run: docker run -d --name trino --network host -v "$PWD/ci/trino/catalog:/etc/trino/catalog:ro" trinodb/trino:483
```
**Issue:** 注释明确说 "Third-party actions are pinned to full commit SHAs"，可第三方容器镜像 `postgres:17`（浮动的 major tag，minor 版本会静默变化）和 `trinodb/trino:483` / `:484` 都只钉了可变 tag。镜像跑的是 `--network host`，能访问 runner 网络和工作目录挂载。tag 被覆盖或供应链被投毒时，CI 结果不可复现；ENGINE-05 声称验证的是 "Trino 483"，实际跑的是 tag 当下指向的任意镜像。
**Fix:** 改用 `postgres:17.x@sha256:<digest>` 和 `trinodb/trino:483@sha256:<digest>`。nightly 的 matrix 可以写成 `{version: "483", digest: "sha256:..."}`，再拼成 `"trinodb/trino:$TRINO_VERSION@$TRINO_DIGEST"`（依然走 env）。由 Dependabot/Renovate 的 docker 生态负责更新。

### WR-C07: 真实 Trino 上的 `--check-pk` 只跑过「无发现」路径

**File:** `ci/pg/seed.sql:32`, `56`, `98`, `120`, `148`; `crates/ontoforge/tests/cli_trino_it.rs:51-86`
```sql
    tail_number varchar(16) PRIMARY KEY,
```
**Issue:** seed 的每张表都有 PostgreSQL `PRIMARY KEY` 约束，所以 CI Trino 上的 `--check-pk` 永远查不到重复或 NULL。`cli_trino_it` 只断言 exit 0，`ontoforge-engine/tests/trino_it.rs` 也不调用 `check_primary_keys`（`pk_check.rs` 只有 SQL golden 快照和 wiremock）。PK 检查的核心价值是 GROUP BY/HAVING 在 Trino 方言下真能返回重复值，并且 `display_value` 能解码 Trino 的 bigint/varchar 结果，这条路径在真实引擎上从未执行，SQL 方言或解码回归会在 CI 中保持绿色。
**Fix:** seed 里加一张无约束的表（例如 `aviation.flights_dup`，复制几行并插入 NULL key），在 `cli_trino_it` 中用一个临时 ontology 绑定它，断言 V18 的 duplicate 和 NULL 两条都出现，exit 1。

## Info

### IN-C01: 死代码与魔法数

**File:** `crates/ontoforge/src/online.rs:88-91`; `crates/ontoforge/src/lib.rs:97`
```rust
pub fn validate_online(args: &ValidateArgs) -> (LoadOutput, u8) {
```
```rust
            _ if run.code == 3 => engine_summary(&run.out),
```
**Issue:** `online::validate_online` 在整个 workspace 中没有调用方（`lib.rs` 里同名的私有函数调的是 `run_online`），名字相同还容易误导。`lib.rs` 硬编码 `3`，没有复用 `online::EXIT_ENGINE`（它是私有的）。`engine_error_diagnostic` 是 `pub`，但只在模块内使用。
**Fix:** 删除 `online::validate_online`，把 `EXIT_ENGINE` 改为 `pub const` 后在 `lib.rs` 中引用，`engine_error_diagnostic` 降为私有。

### IN-C02: 重复主键数量在截断后报错

**File:** `crates/ontoforge/src/online.rs:313-319`
```rust
                    "`{name}` has {} duplicate primary key value(s) in `{table}`: {values}{more}",
                    finding.duplicates.len()
```
**Issue:** `duplicates` 已被截断到 `MAX_DUPLICATE_SAMPLES`（20），`more_duplicates` 为真时消息仍写 "has 20 duplicate primary key value(s) ... and more"，前后矛盾（实际至少 21 个）。
**Fix:** `more_duplicates` 时写成 "at least {MAX_DUPLICATE_SAMPLES + 1}"，或改为「showing N」措辞。

### IN-C03: `--timeout` 对 SQLite stand-in 无效，但帮助文本没有说明

**File:** `crates/ontoforge/src/cli.rs:65`
```rust
    /// Query timeout for the online check, in milliseconds.
```
**Issue:** `ontoforge-engine/src/sqlite/` 里没有任何 timeout 处理。本地 `validate aviation --online --timeout 0` 返回 exit 0，说明该 flag 在 fixture 模式下被静默忽略。
**Fix:** 帮助文本注明 "applies to Trino"，或在 SQLite 引擎中用 `tokio::time::timeout` 包住查询。

### IN-C04: `actions/checkout` 未设 `persist-credentials: false`

**File:** `.github/workflows/ci.yml:20`, `54`; `.github/workflows/nightly.yml:41`
```yaml
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
```
**Issue:** 同一 job 随后会运行 PR 代码（build.rs、测试、`--network host` 容器），而 `GITHUB_TOKEN` 留在 `.git/config` 里。token 是 `contents: read`，风险有限，但没有必要把它留给不受信代码。
**Fix:** 在各 checkout 步骤加 `with: { persist-credentials: false }`。

### IN-C05: `clippy.toml` 的 TLS 禁令可以被绕过

**File:** `clippy.toml:3-8`
```toml
disallowed-methods = [
    { path = "reqwest::ClientBuilder::danger_accept_invalid_certs", ...
```
**Issue:** D-09 只禁了 `danger_*` 系列。`reqwest::ClientBuilder::use_preconfigured_tls` 配合自定义 rustls verifier（`rustls::ClientConfig::dangerous()`）同样能跳过证书校验，却不在禁令内。
**Fix:** 加上 `reqwest::ClientBuilder::use_preconfigured_tls` 和 `rustls::ClientConfig::dangerous`（一旦 rustls 成为直接依赖）。

### IN-C06: `closed_port()` 存在端口复用竞态，可能导致 flaky

**File:** `crates/ontoforge/tests/cli_online_trino.rs:198-201`; `crates/ontoforge/tests/cli_check_pk.rs:172-175`
```rust
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    listener.local_addr().expect("addr").port()
```
**Issue:** listener 在函数返回时就关闭了。并行测试（多个 wiremock `MockServer::start()`）可能恰好拿到同一端口，"unavailable" 断言（exit 3）偶发失败。
**Fix:** 在 CLI 子进程跑完之前一直持有 listener，但不 `accept`（连接会挂起直到 connect timeout）。或者改用保留地址，例如 `http://127.0.0.1:1`（特权端口通常无人监听）。

### IN-C07: `report-failure` 会为任意分支上手动触发的失败开 issue

**File:** `.github/workflows/nightly.yml:105-108`
```yaml
  report-failure:
    needs: iceberg
    if: ${{ failure() }}
```
**Issue:** `workflow_dispatch` 可以在任意分支上触发。实验分支的失败也会在 `nightly-failure` issue 上留评论，噪声会掩盖主干上真正的 nightly 回归。
**Fix:** `if: ${{ failure() && github.ref == format('refs/heads/{0}', github.event.repository.default_branch) }}`。


_Reviewed: 2026-10-17T01:15:02Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_

---

# Part: schema

# Phase 02: Code Review Report (part: ontoforge-schema)

**Reviewed:** 2026-10-17
**Depth:** standard
**Files Reviewed:** 38
**Status:** issues_found

## Narrative Findings (AI reviewer)

## Summary

审查范围：`ontoforge-schema` crate 在 `d1b3c1c..HEAD` 引入的 SourceBinding IR、loader（discover / fixture / budget）、binding 校验与 lint 规则 E13–E16 / W17，以及对应测试。

主要结论：

- fixture 路径的安全检查只做字符串层面的 `..` 归一化，加上对**最后一个路径分量**的 `symlink_metadata`。中间目录如果是 symlink / junction，读取就会离开 ontology 目录。已在本机用 Windows junction 复现：`ontoforge lint` 读到了 ontology 目录之外的 CSV 表头。engine 的 SQLite 替身也通过同一个 `fixture_path` + `read_limited` 读取完整行数据，这直接违反了 `read_limited` 文档里 "a checked-in link cannot point the loader at ... a file outside the ontology" 的承诺（CR-S01）。
- E8 要求"fixture 表头只解析一次"，但 `fixture_header` 与 engine `load_table` 在 BOM、空列名、重名列上的语义不一致：W17 对 BOM JSON 误报（已复现），对空列名 / 重名列漏报（WR-S01）。
- `fixture_path` 接受落在 `bindings/<p>/fixtures/` 之外的 `.json`，这类文件同时会被当作定义文件解析，导致加载失败（已复现 `ONT-V02`，WR-S02）。
- 还有几处 binding 校验缺口：link 端 `from: {}` 没有写 column 也能通过（WR-S03）；同一 binding 内多个 key 映射到同一物理列不报错（WR-S04）；decimal 精度合法性只靠可降级、且 `validate` / 服务端不执行的 lint E13 兜底（WR-S05）。

复现记录（只在 temp 目录操作，没有改源码）：

```text
# junction: <root>/bindings/dev/fixtures -> <outside>/secret (secret.csv header "nope,password")
$ ontoforge lint <root> --profile dev
ONT-W17  ⚠ fixture `fixtures/secret.csv` has no column `id` for `id`     <- 读到了 root 外文件的表头

# bindings/dev/rows.json 作为 fixture
$ ontoforge validate <root> --profile dev
ONT-V02  × document has no `kind` field   [bindings/dev/rows.json:1:2]

# fixtures/rows.json 以 UTF-8 BOM 开头
$ ontoforge lint <root> --profile dev
ONT-W17  ⚠ cannot read the header of fixture `fixtures/rows.json`: the JSON fixture is not valid JSON (line 1, column 1)
```

## Critical Issues

### CR-S01: fixture 路径经中间目录 symlink / junction 逃逸出 ontology 目录（BLOCKER）

**File:** `crates/ontoforge-schema/src/loader/discover.rs:85-131`, `crates/ontoforge-schema/src/loader/mod.rs:197-203`, `crates/ontoforge-schema/src/loader/mod.rs:221-236`

**Issue:** `fixture_path` 是纯字符串运算：

```rust
/// Resolve `fixture_file` ... Pure string arithmetic: no file system access
pub fn fixture_path(binding_file: &str, fixture_file: &str) -> Result<String, String> {
```

之后 `probe_disk` 和 `read_limited(.., false, ..)` 只对完整路径调用 `symlink_metadata`：

```rust
    let meta = if follow_links {
        std::fs::metadata(path)
    } else {
        std::fs::symlink_metadata(path)
    }
```

`lstat` 只是不跟随**最后一个**分量，中间分量照常解析。所以只要仓库里提交一个目录 symlink（Linux 上 git 原生支持；Windows 上是 junction），比如 `bindings/dev/fixtures -> /`，再写 `fixture: { file: fixtures/srv/app/secrets/x.csv }`，就能绕过 "stays inside the ontology directory" 检查。`walk()` 不进入 symlink 目录，这只保护了定义文件，保护不了 fixture。

影响：
1. schema 侧 `capture_fixture_headers` 会读 root 外文件，W17 诊断能反映其表头内容（已复现）。
2. `crates/ontoforge-engine/src/sqlite/fixture.rs:159,341` 用同一个 `fixture_path` + `read_limited(root.join(file), false, ..)` 把**全部行**载入 SQLite 替身，再经 REST / MCP 提供查询。结果是：任何不受信任的 ontology 仓库，都能读出服务进程可读的任意 `.csv` / 顶层数组 `.json` 文件。
3. `read_limited` 文档（CR-03）声称已防住这种情况，现有测试 `symlinked_fixture_is_v16` 只覆盖了最后一个分量是文件 symlink 的情形，所以给出了错误的安全感。

**Fix:** 实际 open 之前，对 resolved 路径的每个前缀逐级做 `symlink_metadata`，拒绝 symlink / reparse point；或者 canonicalize 后校验前缀。最好由 schema 提供唯一的 `open_beneath(root, rel)`，engine 一并复用：

```rust
/// Refuse any symbolic link or junction on the way from `root` to `rel`.
fn check_no_links(root: &Utf8Path, rel: &str) -> Result<(), ReadFailure> {
    let mut at = root.to_path_buf();
    for seg in rel.split('/') {
        at.push(seg);
        let meta = std::fs::symlink_metadata(at.as_std_path())
            .map_err(|e| ReadFailure::Io(e.kind()))?;
        if meta.file_type().is_symlink() {
            return Err(ReadFailure::NotRegular);
        }
    }
    // Defense in depth: the canonical path must still be under the canonical root.
    let real = std::fs::canonicalize(root.join(rel)).map_err(|e| ReadFailure::Io(e.kind()))?;
    let base = std::fs::canonicalize(root).map_err(|e| ReadFailure::Io(e.kind()))?;
    if !real.starts_with(&base) {
        return Err(ReadFailure::NotRegular);
    }
    Ok(())
}
```

`probe_disk`、`read_text`（fixture 分支）、engine `load_table` 都应调用它。另一种做法是按项目"优先复用现成 crate"的原则引入 `cap-std`（`Dir::open_ambient_dir(root)?.open(rel)` 自带 beneath 语义）。同时补测试：Unix 下用目录 symlink，Windows 下用 `mklink /J` junction（无需管理员权限）。

## Warnings

### WR-S01: `fixture_header` 与 engine 的表头语义不一致：W17 对 BOM JSON 误报，对空列名 / 重名列漏报

**File:** `crates/ontoforge-schema/src/loader/fixture.rs:15-59`（对照 `crates/ontoforge-engine/src/sqlite/fixture.rs:351-378`）

**Issue:** 模块文档写的是 "The only implementation of fixture header semantics"（E8），实际上 engine 在调用前后各自加了一套规则：
- engine 对**所有格式**先做 `text.strip_prefix('\u{feff}')`；schema 只在 `csv_header` 剥 BOM，`json_header` 直接 `serde_json::from_str(text)`。结果是带 BOM 的 JSON fixture（Windows 记事本默认产出）engine 能加载，W17 却报 "cannot read the header ... not valid JSON (line 1, column 1)"（已复现）。
- engine 拒绝空列名（`"the fixture has a column without a name"`）和忽略大小写后重名的列；`fixture_header` 都接受，W17 因此显示干净（例如 `id,,name` 或 `id,ID`），到 engine 加载时才失败。这与 E20 的 "a broken fixture never makes the lint look clean" 相悖。

**Fix:** 把这三条规则下沉到 `fixture_header`，engine 删掉自己的副本：

```rust
pub fn fixture_header(file: &str, text: &str) -> Result<Vec<String>, String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let header = match extension.as_deref() { /* csv / json as now */ }?;
    let mut seen = std::collections::HashSet::new();
    for name in &header {
        if name.is_empty() { return Err("the fixture has a column without a name".into()); }
        if !seen.insert(name.to_lowercase()) {
            return Err("the fixture has two columns whose names differ only in case".into());
        }
    }
    Ok(header)
}
```

### WR-S02: 位于 `bindings/<p>/fixtures/` 之外的 JSON fixture 会被同时当成定义文件解析，加载失败

**File:** `crates/ontoforge-schema/src/loader/discover.rs:74-77`, `crates/ontoforge-schema/src/loader/discover.rs:85-131`

**Issue:** `fixture_path` 接受 root 内任意位置的 `.json`，单元测试里也明确断言 `"./a/../rows.JSON"` 解析成 `bindings/dev/rows.JSON` 是合法的。但 `is_candidate` 只排除 `bindings/<p>/fixtures/` 下的文件：

```rust
    parts.len() >= 4 && parts[0] == BINDINGS_DIR && parts[2] == FIXTURES_DIR
```

所以 `bindings/dev/rows.json` 或 `data/rows.json` 会被默认 include `**/*.json` 选中，当作定义文档解析，得到 `ONT-V02 document has no kind field`（已复现）。按 `FixtureRef` 文档合法的写法，实际无法加载。

**Fix:** 二选一，并让文档与之一致：(a) `fixture_path` 要求结果位于 `bindings/<profile>/fixtures/` 下，路径在别处时报 V16 并给出 help；(b) 在 `assemble` 中先解析所有 binding 的 fixture 引用，把这些路径从定义候选集中剔除（这要求两阶段加载，复杂度更高）。推荐 (a)。

### WR-S03: link binding 写 `from: {}` / `to: {}` 而不写 `column` 也能通过"必须映射到列"的检查

**File:** `crates/ontoforge-schema/src/validate/bindings.rs:112-125`

**Issue:** 检查只看 key 是否存在：

```rust
            for end in LINK_ENDS {
                if !binding.columns.contains_key(end) {
```

`columns: { from: { collation: caseSensitive }, to: { column: pilot_id } }` 能通过校验。之后 `bound_columns` 经 `column_for("from")` 回落到字面列名 `from`，与错误信息 "must map the `from` end to a column" 的意图相反。实际后果是读一个几乎不存在的 `from` 列，要到 online 检查才会暴露。

**Fix:**

```rust
if binding.columns.get(end).and_then(|c| c.column.as_deref()).is_none() {
    // same V16 diagnostic
}
```

### WR-S04: 同一 binding 内多个 key 映射到同一物理列时不报错，engine 静默取第一个类型

**File:** `crates/ontoforge-schema/src/ir/binding.rs:167-200`, `crates/ontoforge-schema/src/validate/bindings.rs:87-140`

**Issue:** 以 `columns: { a: { column: b } }` 为例，如果对象同时有属性 `b`（按默认取同名列 `b`），或两个属性都显式写了 `column: x`，`bound_columns` 就会返回两个同名 `column`。校验没有检查这种情况。engine `load_table` 用 `.find(|c| c.name.to_lowercase() == ...)` 只取第一个匹配的 logical type，两个属性类型不同时，第二个属性实际读到的是按第一个类型转换出来的值，不会有任何诊断。

**Fix:** 在 `column_keys`（或新增 `distinct_columns`）里对 `ontology.bound_columns(binding)` 按 `column.to_lowercase()` 去重，冲突时报 V16，位置指向后出现的那个 key：

```rust
let mut seen: HashMap<String, &str> = HashMap::new();
for c in ontology.bound_columns(binding) {
    if let Some(first) = seen.insert(c.column.to_lowercase(), c.key) {
        out.push(Diagnostic::error(codes::V16,
            format!("`{}` and `{first}` are both read from column `{}`", c.key, c.column),
            &at(&["columns", c.key], Part::Key)));
    }
}
```

### WR-S05: decimal precision / scale 的合法性只由可降级的 lint E13 保证，`validate` 与服务端路径不检查

**File:** `crates/ontoforge-schema/src/validate/objects.rs:121-156`, `crates/ontoforge-schema/src/lint/binding/e13.rs:115-147`

**Issue:** `decimal_arguments` 只检查 precision / scale 是否放在了 decimal 属性上，注释说 "Their value ranges are lint territory (E13)"。但 E13 可以用 `"ONT-E13" = "off"` 关掉（测试 `e13_can_be_downgraded` 有覆盖），而且 `ontoforge validate` 和 engine / server 加载路径根本不执行 lint（只有 `lint_cmd.rs` 调 `lint::run`）。于是 `precision: 0`、`precision: 1000`、`scale > precision` 都能通过 `validate`，`load_dir` 返回 `Some(ontology)`。engine `physical_of` 中的 `u8::try_from(*precision).unwrap_or(u8::MAX)` 会把 1000 静默钳成 255，生成任何引擎都不合法的 `decimal(255, …)`。这是类型不变量，不是风格问题。

**Fix:** 把范围检查（`1..=38`、`scale <= precision`）移到 `validate/objects.rs::decimal_arguments` 作为 V06 硬错误，E13 只保留"依赖默认 scale"这类提示性内容；或者至少让 engine 在 `effective_decimal()` 越界时返回错误，不要 `unwrap_or(u8::MAX)`。

## Info

### IN-S01: 诊断原样回显用户值；物理名 / fixture 路径只拒绝 Cc，不拒绝 bidi 控制符

**File:** `crates/ontoforge-schema/src/validate/bindings.rs:151`, `:185`, `:207`; `crates/ontoforge-schema/src/loader/discover.rs:96`

**Issue:** `format!("fixture file `{}` cannot be used: {reason}", fixture.file)` 会把含控制符的原始路径放进 message（触发这条诊断的原因恰恰就是含控制符），`zone` 与 `columns` 的 key 也一样。CLI 的 `render::sanitize` 兜住了终端显示，但 JSON 输出和将来的 REST / MCP / 日志拿到的是原文。另外 `physical_names` 用 `char::is_control`（只覆盖 Cc），U+202E 等 bidi 字符（Cf）能进入 catalog / table / column 名，与注释 "this keeps them readable" 不符，`render::sanitize` 已经列出了这组字符。

**Fix:** 拒绝的是不可读的值时，message 里不要回显原值，或回显 `{:?}` 转义形式；`physical_names` / `fixture_path` 的拒绝集合与 `render::sanitize` 的 bidi 列表对齐（可抽成 schema 里的共享 `is_unsafe_char`）。

### IN-S02: 注释 "E rules first" 已失效

**File:** `crates/ontoforge-schema/src/lint/mod.rs:162-169`

**Issue:** `/// All rules, E rules first.` 但 `rules.extend(binding::rules());` 把 E13–E16 追加在 W 规则之后。输出会再排序，所以功能上没有影响，只是注释误导。

**Fix:** 把 binding 的 E 规则插到 naming 之前，或者把注释改为 "order does not matter; output is sorted"。

### IN-S03: 重复实现：`all_bindings` 有两份，fixture 路径解析分散在四处

**File:** `crates/ontoforge-schema/src/validate/bindings.rs:15`, `crates/ontoforge-schema/src/lint/binding/mod.rs:22`, `crates/ontoforge-schema/src/validate/bindings.rs:71,204`, `crates/ontoforge-schema/src/loader/mod.rs:396`

**Issue:** `fn all_bindings(ontology: &Ontology) -> impl Iterator<...>` 逐字重复。"locate binding file，再 `fixture_path`" 这一步在 schema 内出现三次，engine 又有一次。CR-S01 修复时必须四处同步改，很容易漏掉。

**Fix:** 在 `ir::binding` 上提供 `Ontology::all_bindings()`，在 `SourceMap` 上提供 `resolved_fixture(def_name, &FixtureRef) -> Result<String, String>`，所有调用方复用。

### IN-S04: `bindings` / `fixtures` 目录名按大小写敏感比较，在大小写不敏感的文件系统上行为不一致

**File:** `crates/ontoforge-schema/src/loader/discover.rs:60-77`

**Issue:** `parts[0] == BINDINGS_DIR && parts[2] == FIXTURES_DIR` 是精确比较。在 Windows / macOS 上，`bindings/dev/Fixtures/rows.json` 会被当作定义文件解析（报 V02），`Bindings/dev/x.yaml` 报 "must be placed under `bindings/<profile>/`"。manifest 名已经按 WR-06 处理过大小写，这里没有。

**Fix:** 像 `miscased_manifest` 一样对错写大小写的目录给出 V14 / V16 并提示改名，或用 `eq_ignore_ascii_case` 判断后再报错。

### IN-S05: symlink 测试只覆盖最后一个分量，且只在 Unix 上运行

**File:** `crates/ontoforge-schema/tests/bindings.rs:684-715`

**Issue:** `#[cfg(unix)] fn symlinked_fixture_is_v16` 只测了 `fixtures/link.csv -> real.csv`，没有覆盖目录 symlink 或 junction（CR-S01 的场景），Windows 本机也从不运行。`tests/loader.rs:250-258` 已有跨平台 helper 和"无权限则跳过"的模式，可以直接复用。

**Fix:** 增加 `bindings/dev/fixtures` 为目录 symlink（Unix）/ junction（Windows，`mklink /J` 不需要权限）的用例，断言得到 V16 "not a regular file" 一类错误；engine 侧加同样的用例。


_Reviewed: 2026-10-17_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_

---

# Part: engine

# Phase 02: Code Review Report (part: engine)

**Reviewed:** 2026-10-17T01:17:29Z
**Depth:** standard
**Files Reviewed:** 36
**Status:** issues_found

## Summary

审查范围是 `ontoforge-engine` crate：Trino 语句协议客户端（重试、取消、nextUri 同源校验、TLS）、typed SQL printer、SQLite 替身与 fixture 加载、`bind_check` / drift、`--check-pk`、`ontoforge-server.toml` 配置解析。

整体结论：核心安全面做得扎实，没有发现 BLOCKER。SQL 文本只能经 `PrintedSql` 产出，标识符与字符串字面量统一走 `quote_ident` / `quote_string`，没有发现注入路径。nextUri 有同源加 scheme 校验，拒绝 userinfo；redirect 关闭；Basic / Bearer 在库层强制 https；`Retry-After` 有上限；页面字节有上限；`Debug` 不泄露凭据；POST 重试收窄（E16）也已落实。

问题集中在四处：
1. 配置错误信息可能把误填的密钥原样回显（WR-E01）。
2. `[trino].client_tags` 校验了但从未生效（WR-E02）。
3. SQLite 替身对 decimal / timestamp 字面量的比较语义与 Trino 不一致，Phase 4 一旦生成过滤就会静默给出错误结果（WR-E03）。
4. DoS 面：SQLite 替身忽略超时（WR-E04）；默认 64 MiB 页面整页解析成 `serde_json::Value`，内存会放大（WR-E05）；类型字符串无深度限制递归解析（WR-E06）。

另外，`SourceUnreadable` 原样携带 Trino 错误文本，Phase 7 上 `/health` 后会成为信息泄露面（WR-E07）。

## Narrative Findings (AI reviewer)

## Warnings

### WR-E01: 误把密钥值写进 `*_env` / `*_file` 时，错误信息会原样回显该值

**File:** `crates/ontoforge-engine/src/config.rs:459-466`（同类：`config.rs:469-471`、`config.rs:394-442`）

```rust
None => Err(ConfigError::new(format!(
    "environment variable `{name}` is not set"
))),
```

**Issue:** `check_identity` 只统计 `password_env` / `token_env` 是否存在，不校验其内容是否像一个环境变量名。运维人员常见的误操作是 `password_env = "Pa55w0rd!"`（把值当成了变量名）。这时 `resolve_secret` 会产出 ``environment variable `Pa55w0rd!` is not set``，经 `build_trino_engines` 转成 V20 诊断，打印到终端或 CI 日志，明文密钥就泄露了。`token_file = "<jwt>"` 同理，会出现在 ``cannot read secret file `<dir>/<jwt>` `` 中。模块文档承诺 "Texts name keys, variables and paths, never secret values"，这条路径违反了该承诺。

**Fix:** 在 `check_identity` 中校验变量名，不合法时报错但不回显内容：
```rust
fn is_env_name(s: &str) -> bool {
    let mut b = s.bytes();
    matches!(b.next(), Some(c) if c == b'_' || c.is_ascii_alphabetic())
        && b.all(|c| c == b'_' || c.is_ascii_alphanumeric())
}
// in check_identity:
for (key, v) in [("password_env", &identity.password_env), ("token_env", &identity.token_env)] {
    if v.as_deref().is_some_and(|n| !is_env_name(n)) {
        problem(format!("identity `{label}`: `{key}` must be an environment variable name ([A-Za-z_][A-Za-z0-9_]*)"));
    }
}
```
`resolve_secret` 中同样先校验，不合法时只说 "`password_env` is not a valid variable name"。

### WR-E02: `[trino].client_tags` 配置项被校验但从未使用（静默失效）

**File:** `crates/ontoforge-engine/src/config.rs:59-60, 92-100, 376-385`；调用方 `crates/ontoforge/src/online.rs:119-122`

```rust
pub fn query_context(&self, trace_token: String, client_tags: Vec<String>) -> QueryContext {
```

**Issue:** `TrinoConfig.client_tags` 有默认值 `["ontoforge"]`，也做了格式校验，但 `query_context` 使用调用方传入的 `client_tags` 参数，完全忽略 `self.client_tags`。CLI 固定传 `vec!["ontoforge", "validate-online"]`。grep 全仓库，`client_tags` 字段只在 `config.rs` 中被读来做校验。运维在 `ontoforge-server.toml` 写 `client_tags = ["team-a"]` 用于 Trino resource group 路由时，配置被接受却不生效，没有任何提示。

**Fix:** 让配置的 tag 生效，并与调用方 tag 合并去重：
```rust
pub fn query_context(&self, trace_token: String, extra_tags: Vec<String>) -> QueryContext {
    let mut client_tags = self.client_tags.clone();
    for t in extra_tags { if !client_tags.contains(&t) { client_tags.push(t); } }
    QueryContext { trace_token, client_tags, /* ... */ }
}
```
如果不打算支持这个配置项，就删除该字段（`deny_unknown_fields` 会让旧配置显式报错）。

### WR-E03: SQLite 替身中 decimal / timestamp 字面量与存储形态不一致，比较结果与 Trino 不同

**File:** `crates/ontoforge-engine/src/printer/dialect.rs:231-249`；`crates/ontoforge-engine/src/sqlite/fixture.rs:286-291, 536-537, 651-665, 751`

```rust
Literal::Decimal(t) => { ...; typed(trino, "DECIMAL", t) }   // Sqlite: plain '1.5'
...
_ => SqliteStorage::Text,                                      // decimal / timestamp stored as TEXT
...
let mut out = format!("{date} {time}.{millis}");               // stored '2026-01-01 00:00:00.000'
```

**Issue:** fixture 把 decimal 规范化为固定 scale 的 TEXT（`1.5` 存为 `'1.50'`），把 timestamp 规范化为 `YYYY-MM-DD HH:MM:SS.fff` 的 TEXT（aware 列带 `+HH:MM`）。printer 在 `Dialect::Sqlite` 下把字面量原样输出为字符串。列是 TEXT affinity，SQLite 因此做字典序比较：
- `"amount" = '1.5'` 匹配不到 `'1.50'`，而 Trino 中 `DECIMAL '1.5' = 1.50` 为真；
- `"amount" > '9.00'` 时 `'10.00'` 判为更小；负数顺序同样错乱；
- `"ts" = '2026-01-01 00:00:00'` 匹配不到 `'...00.000'`，而 `"ts" > '2026-01-01 00:00:00'` 对相等时刻反而为真；
- aware 列中不同 offset 的同一瞬间不相等，排序也错。

`Value` 的文档写了 "numeric comparison is a later phase's business"，但 printer 已经对外公开接受 `Literal::Decimal` / `Literal::Timestamp` 并能对 SQLite 打印。替身作为 Trino 的测试替代，会静默返回与 Trino 不同的行，测试绿而生产错。Phase 2 现有调用（pk 检查）只用到 `GROUP BY` / 等值分组，暂未触发。

**Fix:** 二选一，并补对照测试（同一 AST 在 SQLite 替身与 Trino goldens 上结果一致）：
- 在 `Dialect::Sqlite` 下把 `Literal::Timestamp` 规范化为 fixture 的毫秒形态（补齐 `.fff`，超过 3 位小数报 `PrintError`）；`Literal::Decimal` 在 SQLite 下先返回新的 `PrintError::UnsupportedInDialect`，等 Phase 4 定义 decimal 比较语义（例如按 scale 存为放大后的 INTEGER）再放开；
- 或者在 ADR / printer 文档中明确 SQLite 替身不支持这两类字面量的过滤与排序，并在 printer 中硬性拒绝。

### WR-E04: SQLite 替身的 `execute` 完全忽略 `ctx.timeout`，并在整个查询期间持有唯一连接的锁

**File:** `crates/ontoforge-engine/src/sqlite/mod.rs:280-333`

```rust
let mut conn = self.conn.lock().await;
let statement = (&mut *conn).prepare(sql_text).await.map_err(map_sqlx)?;
```

**Issue:** `Engine` 契约中 `QueryContext.timeout` 是每个查询的上限，`TrinoEngine` 用 `tokio::time::timeout` 落实了，`SqliteEngine` 没有。一条代价高的查询（10 万行 fixture 上的分组排序，或 Phase 4 生成的笛卡尔类查询）会无限运行，同时持有 `tokio::sync::Mutex`，排在后面的所有请求（包括 `status()`）一起阻塞。`status()` 也因此可能永不返回。CLI 的 `--timeout` 在替身模式下同样不生效。

**Fix:**
```rust
match tokio::time::timeout(ctx.timeout, async {
    let mut conn = self.conn.lock().await;
    /* existing body */
}).await {
    Ok(r) => r,
    Err(_) => Err(EngineError::Timeout { after_ms: u64::try_from(ctx.timeout.as_millis()).unwrap_or(u64::MAX) }),
}
```
sqlx 的 sqlite 后端在 worker 线程执行，future 被 drop 后语句仍可能跑完，所以还需要配合 `sqlite3_progress_handler` 或 `interrupt`（sqlx `LockedSqliteHandle`）真正中止。至少要让调用方按时拿到 `Timeout`。

### WR-E05: 默认 `max_page_bytes` 为 64 MiB，整页先缓冲再解析成 `serde_json::Value`，内存放大没有上限约束

**File:** `crates/ontoforge-engine/src/config.rs:80-82`；`crates/ontoforge-engine/src/trino/client.rs:194-207`；`crates/ontoforge-engine/src/trino/mod.rs:275-305`

```rust
fn default_max_page_bytes() -> usize {
    64 * 1024 * 1024
}
```

**Issue:** `read_body` 把整页（最多 64 MiB）读进 `Vec<u8>`，然后 `serde_json::from_slice` 把 `data: Vec<Vec<serde_json::Value>>` 完整物化。小单元格（如 `1,`、`null,`）的 `Value` 对象约 32 B 起，加上每行 `Vec` 的开销，膨胀可达 10 到 20 倍，单个查询单页峰值可达数百 MB 到 1 GB 级。`ctx.max_rows` / `max_bytes` 只在解析完成后的 `Collector::absorb` 中生效，限制不到这部分峰值。服务端（Phase 7）多个并发查询时，一个返回大页的 coordinator（或在 TLS 被正确配置之外的中间层）就能把进程打到 OOM。`max_bytes` 默认是 16 MiB，页面上限却是它的 4 倍，两者不协调。

**Fix:** 把默认 `max_page_bytes` 降到与 `max_bytes` 同量级（例如 8 到 16 MiB），并在 `check_trino` 校验 `max_page_bytes <= max_bytes * k`；`data` 改为 `Vec<Box<serde_json::value::RawValue>>`（需开 `raw_value` feature），在 `absorb` 中按行先计 `raw.get().len()` 再按需解析，达到 `max_rows` / `max_bytes` 后剩余行不再解析。

### WR-E06: `PhysicalType::parse` 对 `array(...)` 无深度限制递归，服务端返回的类型字符串可以导致栈溢出

**File:** `crates/ontoforge-engine/src/value.rs:43-49`（调用点：`trino/decode.rs:41`、`trino/mod.rs:537`）

```rust
let start = text.len() - inner.len() - 1;
return Self::Array(Box::new(Self::parse(&text[start..start + inner.len()])));
```

**Issue:** 类型文本来自 Trino 响应（`columns[].type` 和 `information_schema.columns.data_type`），属于网络输入。`"array(" * N + "x" + ")" * N` 每层 7 字节，64 MiB 的页面上限允许约 900 万层嵌套。这会导致递归 `parse` 栈溢出，`Box<PhysicalType>` 的递归 `Drop` 也会栈溢出。Rust 中栈溢出直接 abort 整个进程，无法当作 `EngineError` 处理。另外每层都对剩余文本 `to_ascii_lowercase()`，复杂度是 O(N²)。JSON 数据本身受 serde_json 的 128 层限制，但这条类型字符串路径没有任何限制。触发需要恶意或被攻破的 coordinator（或网关），但后果是整进程崩溃，修复成本很低。

**Fix:**
```rust
const MAX_TYPE_DEPTH: usize = 32;
pub fn parse(s: &str) -> Self { Self::parse_at(s, 0) }
fn parse_at(s: &str, depth: usize) -> Self {
    let text = s.trim();
    if depth >= MAX_TYPE_DEPTH || text.len() > 4096 { return Self::Other(text.chars().take(64).collect()); }
    /* ... recurse with depth + 1 ... */
}
```

### WR-E07: `DegradeReason::SourceUnreadable` 原样携带 Trino 错误文本，Phase 7 暴露到 `/health` 时会泄露内部细节

**File:** `crates/ontoforge-engine/src/bind_check.rs:663-669`；`crates/ontoforge-engine/src/trino/client.rs:177-185`；`crates/ontoforge-engine/src/error.rs:250-252`

```rust
reason: DegradeReason::SourceUnreadable {
    error: e.to_string(),
},
```

**Issue:** `EngineError::Query.message` 是 Trino 服务端 `error.message` 的原文，常见内容包括 metastore 地址（`thrift://10.0.0.5:9083`）、S3 / HDFS 路径、connector 内部异常，以及类型转换失败时的数据值（`Cannot cast 'xxx' to ...`）。`Unavailable.reason` 由 `transport_failure` 拼出完整 cause chain，也带内部主机名和 IP。`compare_binding` 的文档承诺 "Messages carry only apiNames and physical names, never data"，`SourceUnreadable` 却绕过了这一约束。`check_drift` 的文档写明结果将在 Phase 7 暴露到 `/health`，届时匿名或低权限调用方可以读到内部拓扑和数据片段。

**Fix:** `DegradeReason::SourceUnreadable` 只保留稳定字段，例如 `{ code: &'static str /* e.code() */, error_name: Option<String> }`，完整文本只写服务端日志。或者明确约定 `error` 字段仅供 CLI / 管理端使用，在 Phase 7 `/health` 序列化时剥离，并在类型文档中写明。

## Info

### IN-E01: `sql_text_only_from_printer` 守卫漏检 `AssertSqlSafe(`、小写关键字和 raw string

**File:** `crates/ontoforge-engine/tests/sql_text_only_from_printer.rs:13, 28-29`

```rust
let sql_literal = Regex::new(&format!(r#""(?:{KEYWORDS})\b"#)).unwrap();
```

**Issue:** 正则只匹配以大写关键字开头的普通字符串字面量。`r"SELECT ..."`、`"select ..."`、`format!("{} FROM t", x)`、以及在 printer 之外对任意 `String` 调用 `sqlx::query(AssertSqlSafe(...))` 都能通过扫描。`sqlite/mod.rs` 中已有 2 处 `AssertSqlSafe(` 调用（目前参数都来自 `PrintedSql`，是合法的）。
**Fix:** 增加对 `AssertSqlSafe\(` 的扫描，只允许参数形如 `<ident>.as_str().to_owned()` 且该 ident 类型为 `PrintedSql`（白名单出现位置）；正则加 `(?i)` 并覆盖 `r#?"`。

### IN-E02: SQLite `decode` 对未知存储类型静默返回 `Value::Null`

**File:** `crates/ontoforge-engine/src/sqlite/mod.rs:271`

```rust
_ => Ok((Value::Null, 0)),
```

**Issue:** 如果 sqlx 升级后对值的 `type_info().name()` 改报 `BOOLEAN` / `NUMERIC` / `DATETIME` 等名称，非空值会被静默变成 NULL，测试与 pk 检查都可能给出假阴性。
**Fix:** 改为 `Value::Unsupported(serde_json::Value::String(format!("<sqlite {type_name}>")))`，或者返回 `EngineError::Protocol`。

### IN-E03: `check_primary_keys` 忽略 `truncated`；PrintError 被映射为 engine-wide 的 `Config`

**File:** `crates/ontoforge-engine/src/pk_check.rs:138-152`

```rust
let queries = pk_queries(&table, pk, engine.dialect())
    .map_err(|e| EngineError::Config(e.to_string()))?;
```

**Issue:** (1) 某个绑定的列名无法打印（`PrintError`）时被映射为 `Config`，而 `Config` 属于 `is_engine_wide`，CLI `check_keys` 会因此中止整轮 `--check-pk`，与 E3 按绑定隔离的原则不符。(2) duplicates 查询的 `out.truncated` 未检查：被 `max_bytes` 截断时 `more_duplicates` 报 `false`，属于假阴性。
**Fix:** 改用 `EngineError::Print(e)`（非 engine-wide）；`let more_duplicates = out.truncated || duplicates.len() > MAX_DUPLICATE_SAMPLES;`。

### IN-E04: 在线检查接受 `timestamp(6/9)` 列，fixture 却以「会静默丢信息」为由拒绝超过 3 位的小数秒

**File:** `crates/ontoforge-engine/src/bind_check.rs:285-288`；`crates/ontoforge-engine/src/sqlite/fixture.rs:740-749`

```rust
PhysicalType::Timestamp { with_time_zone, .. } => {
    timezone_compatibility(expected.timezone.as_deref(), *with_time_zone)
```

**Issue:** `docs/source-binding.md:182` 把 `aware` 加 `timestamp(6) with time zone` 记为 OK，这是有意的设计。但 D-13 的原则是「只接受无损读取」，fixture 侧又把 timestamp 定为毫秒精度，两边对 timestamp 精度的立场不一致。Phase 4 如果按毫秒物化属性，会截断微秒值。
**Fix:** 在 ADR 中明确 timestamp 属性的精度。如果是毫秒，`precision > 3` 应给 `PrecisionChanged` warning。

### IN-E05: 替身按 Unicode `to_lowercase` 建表，查询按原始大小写引用，非 ASCII 标识符查不到表

**File:** `crates/ontoforge-engine/src/sqlite/mod.rs:99, 162-168`；`crates/ontoforge-engine/src/printer/dialect.rs:97-101`

```rust
let target = lower(&table.table);
```

**Issue:** 建表用 Unicode 小写（`"café"`），而 `pk_check` / 查询打印绑定的原始 `TableRef`（`"Café"`）。SQLite 的标识符大小写折叠只覆盖 ASCII，于是报 `no such table`。`describe_tables` 用 `lower()` 查元数据，不受影响，所以问题只在执行查询时暴露。
**Fix:** 替身执行前对 `TableRef` 做同样的 `lower()`。或者在 printer 的 `Dialect::Sqlite` 分支对 schema / table 统一 `to_lowercase()`，与 fixture 建表规则对齐。

### IN-E06: POST 进行中被 drop 或超时，coordinator 上的查询无法取消

**File:** `crates/ontoforge-engine/src/trino/mod.rs:214-223`；`crates/ontoforge-engine/src/trino/cancel.rs:57-70`

```rust
let mut page = client::send(
    || { Ok(self.request(Method::POST, self.statement_url.clone(), ctx, true)?.body(...)) },
```

**Issue:** 只有拿到第一个 `nextUri` 之后 guard 才有 DELETE。POST 已被 coordinator 接受、但响应尚未读完时发生超时或调用方 drop，查询会继续运行，直到 `query_max_execution_time`（timeout 加 5 s）。已有 session 限时兜底，影响有限。
**Fix:** 在文档中注明这个窗口。或者从响应头 / 部分 body 中尽早解析 `id` 并构造 `DELETE /v1/query/{id}`（需要额外权限，可选）。

### IN-E07: 公开的 `QueryContext.timeout` 接近 `Duration::MAX` 时，`server_limit_secs` 整数溢出

**File:** `crates/ontoforge-engine/src/trino/headers.rs:163-165`

```rust
ctx.timeout.as_secs() + u64::from(ctx.timeout.subsec_nanos() > 0) + SESSION_LIMIT_SLACK_SECS
```

**Issue:** `QueryContext` 字段是 `pub`，库调用方可以传 `Duration::MAX`。debug 构建下 panic；release 构建下回绕成 `query_max_execution_time=5s`，长查询会被服务端提前杀掉。
**Fix:** 改用 `saturating_add`，并把结果 clamp 到合理上限（例如 1 年）。

### IN-E08: 秘密文件与环境变量的中间副本未清零；秘密文件只去掉一个换行

**File:** `crates/ontoforge-engine/src/config.rs:459-482`

```rust
let value = text
    .strip_suffix("\r\n")
    .or_else(|| text.strip_suffix('\n'))
    .unwrap_or(&text);
```

**Issue:** `bytes` / `text` / `env(name)` 返回的 `String` 在 drop 时没有 zeroize，只有最终的 `SecretString` 会清零。文件末尾有多余空白或两个换行时，密钥带着空白被发送，认证失败，而且报错不会提示原因。
**Fix:** 使用 `zeroize::Zeroizing<Vec<u8>>` / `Zeroizing<String>` 持有中间值；尾部改为 `trim_end_matches(['\r', '\n'])`，或在报错 help 中提示这一点。

### IN-E09: `describe_tables` 复用查询的 `max_rows`，大 catalog 会整体变成 `SourceUnreadable`

**File:** `crates/ontoforge-engine/src/trino/mod.rs:389-409`

```rust
if output.truncated {
    return Err(EngineError::Protocol(format!(
```

**Issue:** 元数据查询沿用业务查询的 `max_rows`（默认 10 000）。一个 catalog 下绑定的表总列数超过这个值时，该 catalog 的所有绑定都被判为不可读，而且无法单独调大元数据查询的上限。
**Fix:** 元数据查询使用独立的 ctx，例如 `max_rows = usize::MAX` 并只受 `max_bytes` 约束。或者按表分批（每批 N 个表）发 `information_schema` 查询。

### IN-E10: `check_transport` 中的小瑕疵

**File:** `crates/ontoforge-engine/src/trino/mod.rs:66-76, 168-172`

```rust
let host = origin.host_str().unwrap_or("<none>");
if origin.host_str().is_none() {
```

**Issue:** (1) `host` 在 `is_none` 检查之前计算，`"<none>"` 分支是死代码。(2) `url` 为 `http` 时配置的 `ca_pem` 被静默忽略，运维可能误以为 TLS 已生效。(3) 没有对 extra origin 的 path / query 做库层校验，这部分只在 `check_trino` 中做，绕过 `parse_server_config` 直接调用 `build_trino_engines` 或 `TrinoEngine::new` 时不受约束。
**Fix:** 先 `let Some(host) = origin.host_str() else { return config(...) };`。`http` 加 `ca_pem` 时返回 `Config` 错误。把 bare-origin 校验下沉到 `check_transport`。


_Reviewed: 2026-10-17T01:17:29Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
