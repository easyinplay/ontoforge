---
phase: 02-source-binding-engine
plan: 04
subsystem: engine
tags: [rust, trino, reqwest, rustls, wiremock, secrecy, server-config, information-schema, clippy-disallowed-methods]

requires:
  - phase: 02-source-binding-engine
    provides: "02-01 的 read_limited / ProfileName / parse_custom_de_error；02-02 的 Engine trait、printer、QueryContext 与 DEFAULT_* 常量、PhysicalType、全部新 workspace 依赖"
provides:
  - "ontoforge_engine::trino::{TrinoEngine, TrinoConnection, Auth}：自写 Trino client（POST /v1/statement、GET nextUri 分页、无状态头、nextUri 同源校验），impl Engine，AnyEngine::Trino"
  - "TrinoEngine::new 的库层传输约束（E2）：Basic / Bearer 只允许 https，额外 origin 必须与 url 同 scheme，url 不得带 userinfo"
  - "ontoforge_engine::config：ServerConfig / TrinoConfig（含 query_context）/ Identities / IdentityConfig / AuthKind / ProfileEngine / EngineKind / ConfigError / EnvLookup / parse_server_config / load_server_config / resolve_secret / TrinoEngines / build_trino_engines"
  - "按列类型的值解码（不匹配降级为 Value::Unsupported）、按 catalog 批量的 describe_tables（information_schema.columns）、status() 探测"
  - "clippy.toml：disallowed-methods 拦截 reqwest 的四个 danger_accept_invalid_* 方法（D-09）"
affects: [02-05, 02-06, 02-08, 02-09, 02-10, phase-04, phase-07]

plan_head_before: 0acc23dd89973bfbd6a1d0f39c94b2d3c4f8f1e9

actuals:
  tokens: 23400
  tasks: 3
  commits: 6

tech-stack:
  added: []
  patterns:
    - "每个 TrinoEngine 实例独占一个 reqwest::Client（redirect::Policy::none、独立连接池与凭据），服务身份之间不共享状态"
    - "请求头逐请求组装，X-Trino-Session 只随 POST 发送，响应里的 X-Trino-Set-* 一律忽略"
    - "凭据传输约束放在 TrinoEngine::new（最后防线），配置层只负责带 span 的友好报错"
    - "配置错误的 span 通过二次 DeTable::parse 按键路径定位，语义校验错误也能指到键"
    - "密钥只经 SecretString；错误文本只含变量名、路径或键名；序列化错误不引用单元格内容"

key-files:
  created:
    - clippy.toml
    - crates/ontoforge-engine/src/config.rs
    - crates/ontoforge-engine/src/trino/mod.rs
    - crates/ontoforge-engine/src/trino/client.rs
    - crates/ontoforge-engine/src/trino/headers.rs
    - crates/ontoforge-engine/src/trino/decode.rs
    - crates/ontoforge-engine/tests/trino_client.rs
    - crates/ontoforge-engine/tests/server_config.rs
    - crates/ontoforge-engine/tests/data/test-ca.pem
  modified:
    - crates/ontoforge-engine/src/lib.rs

key-decisions:
  - "TrinoEngine 内部把 TrinoConnection 装进 Box：加入 Trino 变体后 AnyEngine 触发 clippy::large_enum_variant（304 对 96 字节），装箱后 AnyEngine::Trino(TrinoEngine) 的形状与计划一致"
  - "execute 在本 plan 内就实现 max_rows 截断（第 max_rows + 1 行出现即置 truncated 并停止翻页），因为 describe_tables 的「截断即错误」依赖它；max_bytes、DELETE 取消、重试与超时仍归 02-06"
  - "额外加固：nextUri 带 userinfo 时拒绝；TrinoEngine::new 拒绝无 host、带 userinfo 的 url 与空 user；响应页读取受 max_page_bytes 限制"
  - "错误文本不引用 URL（含 query id）：reqwest 错误经 without_url 后再拼接来源链；QueryResults 解析失败只报分类与行列"
  - "next_uri_origins 与 client_tags 在配置期的报错不回显条目内容（条目可能含 userinfo），只给序号或长度"
  - "测试用 CA 是用 openssl 生成的自签证书，私钥在生成后立即丢弃，仓库内只有证书"

patterns-established:
  - "wiremock 协议测试断言「不发出请求」用 received_requests().len()，断言头用逐请求的 HeaderMap 而不是匹配器"
  - "对 clippy.toml 的拦截用临时探针验证：临时调用四个被禁方法，确认四条 disallowed_methods 报错后再删除探针"

requirements-completed: [ENGINE-01, ENGINE-02]

coverage:
  - id: D1
    description: "TrinoEngine 经自写 client 完成 POST + GET nextUri 分页，拼接 columns / data，query error 映射为 EngineError::Query，每请求无状态头，X-Trino-Set-* 被忽略"
    requirement: "ENGINE-01"
    verification:
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_client.rs#select_one_round_trip"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_client.rs#request_headers"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_client.rs#data_spans_pages"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_client.rs#query_error_maps_to_query"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_client.rs#set_session_header_is_ignored"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_client.rs#bad_trace_token_is_config_error"
        status: pass
    human_judgment: false
  - id: D2
    description: "凭据不外送：nextUri 异源 / 异 scheme 不发请求；Basic / Bearer 在 http 上以及额外 origin 降级 scheme 在配置期与 TrinoEngine::new 都被拒绝"
    requirement: "ENGINE-01"
    verification:
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_client.rs#foreign_next_uri_is_rejected"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_client.rs#next_uri_scheme_mismatch_is_rejected"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/server_config.rs#library_rejects_credentials_over_http"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/server_config.rs#library_rejects_cross_scheme_origin"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/server_config.rs#basic_over_http_is_rejected"
        status: pass
    human_judgment: false
  - id: D3
    description: "ontoforge-server.toml 解析：未知键与明文密钥带 span 报错，profile 键经 ProfileName 校验，查询上限缺省取 DEFAULT_* 并由 query_context 构造"
    requirement: "ENGINE-01"
    verification:
      - kind: integration
        ref: "crates/ontoforge-engine/tests/server_config.rs#parses_full_example"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/server_config.rs#plaintext_password_is_rejected"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/server_config.rs#unknown_key_is_rejected"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/server_config.rs#profile_key_must_be_profile_name"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/server_config.rs#query_context_from_config"
        status: pass
    human_judgment: false
  - id: D4
    description: "密钥引用：*_env 经注入的 EnvLookup、*_file 经 read_limited（64 KiB、可跟随符号链接、去末尾换行）；密钥不进 Debug 与任何错误文本；三种认证头与自定义 CA、两个独立服务身份"
    requirement: "ENGINE-01"
    verification:
      - kind: integration
        ref: "crates/ontoforge-engine/tests/server_config.rs#secret_from_env"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/server_config.rs#secret_from_file_follows_symlink_and_trims"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/server_config.rs#secret_never_in_debug_or_errors"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-engine/src/trino/headers.rs#basic_and_bearer_headers"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/server_config.rs#ca_file_is_loaded"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_client.rs#two_identities_use_separate_clients"
        status: pass
    human_judgment: false
  - id: D5
    description: "不存在跳过证书校验的配置项或代码路径：配置结构 deny_unknown_fields，clippy.toml 拦截四个 reqwest danger_accept_invalid_* 方法"
    requirement: "ENGINE-01"
    verification:
      - kind: other
        ref: "临时探针调用四个被禁方法，cargo clippy -p ontoforge-engine -- -D warnings 报 4 条 disallowed_methods，探针删除后 clippy 干净"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/server_config.rs#unknown_key_is_rejected"
        status: pass
    human_judgment: false
  - id: D6
    description: "值解码、按 catalog 批量的 describe_tables 与 status() 探测"
    requirement: "ENGINE-02"
    verification:
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_client.rs#decode_types"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_client.rs#decode_special_doubles_and_zone_forms"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_client.rs#describe_tables_batches_one_catalog"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_client.rs#describe_missing_tables_absent"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_client.rs#describe_truncated_metadata_is_error"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_client.rs#status_available_and_unavailable"
        status: pass
    human_judgment: false
  - id: D7
    description: "真实 Trino 上的线格式（timestamp with time zone 两种形态、NaN / Infinity 字符串、information_schema 大小写、errorName 取值）"
    requirement: "ENGINE-01"
    verification: []
    human_judgment: true
    rationale: "本 plan 只在 wiremock 上验证，线格式依据 spike 1 与文档，未连真实 Trino；由 02-10 的 Linux CI golden 测试兜底（RESEARCH A3、A11）"

duration: 21min
completed: 2026-10-17
status: complete
---

# Phase 2 Plan 04: Trino client、服务配置与 describe_tables Summary

**自写 Trino client（reqwest 0.13 + rustls）：POST + GET nextUri 分页、含 scheme 的 nextUri 同源校验、无状态头、none / basic / bearer 三种认证（凭据仅 https，库层强制）、自定义 CA 且 clippy 禁止一切跳过校验的方法，配合带 span 的 `ontoforge-server.toml` 解析、`*_env` / `*_file` 密钥引用、按 catalog 批量的 `information_schema` describe 与 `status()` 探测。**

## Performance

- **Duration:** 21 min
- **Started:** 2026-10-16T19:46:59Z
- **Completed:** 2026-10-16T20:07:43Z（实现与最终门；SUMMARY 提交在其后）
- **Tasks:** 3（Task 1 tracer、Task 2 auto/tdd、Task 3 auto/tdd）
- **Files modified:** 10（相对 `0acc23d`；`Cargo.toml` 与 `Cargo.lock` 未改）

## Accomplishments

- tracer 先打通 `PrintedSql` 到 `QueryOutput` 的最薄路径：POST 后循环 GET nextUri，`columns` 取首次出现的页，`data` 跨页累加；页内 `error` 映射为 `EngineError::Query { error_name }`；401 / 403 为 `Unauthorized`，连接失败为 `Unavailable { retryable: true }`，其余非 200 为 `Protocol`。「发送一次请求并得到 `QueryResults`」是 `client::send_once` 单独函数，02-06 可以直接包重试与取消。
- 每个请求都带 `X-Trino-User`、`X-Trino-Source: ontoforge`、`X-Trino-Trace-Token`、`X-Trino-Client-Tags`、`X-Trino-Time-Zone: UTC`，只有 POST 带 `X-Trino-Session: query_max_execution_time=<ceil(timeout)+5>s`。trace token 与 tag 限定 `[A-Za-z0-9._:-]`、长度 128 / 64，非法值在发请求前返回 `EngineError::Config`，测试同时覆盖 129 拒绝、128 接受。
- 凭据不外送有两层：`nextUri` 必须与 `url` 或显式 `next_uri_origins` 同 scheme / host / port（另拒绝带 userinfo 的 nextUri），异源与 https 降级到 http 的测试都断言目标 mock 收到 0 个请求；`TrinoEngine::new` 拒绝 Basic / Bearer + http、跨 scheme 的额外 origin、无 host 或带 userinfo 的 url（E2，直接构造 `TrinoConnection` 也绕不过）。
- `ontoforge-server.toml`：`deny_unknown_fields`，明文 `password` / `token` / `secret` 改写为 plaintext 提示并给出 `*_env` / `*_file` help，span 指向该键；profile 键走 `ProfileName`（E14），非法键的 message / help 由 `parse_custom_de_error` 解出；`profiles` 里 `engine = "trino"` 要求有 `[trino]`；语义错误（http + 凭据、origin scheme 不同、密钥引用个数不对）按键路径二次定位 span。
- 密钥只经 `SecretString`；`*_file` 用 02-01 的 `read_limited(path, true, 64 KiB)`，配置文件与 `ca_file` 用 `MAX_FILE_BYTES`（E10），crate 内没有第二份有界读取实现。`secret_never_in_debug_or_errors` 把含 `s3cr3t-value` 的明文键、超限文件、空文件、不存在文件、http + 凭据的各类错误与 `Debug` 输出一起断言不含密钥。
- 自定义 CA 经 `reqwest::Certificate::from_pem_bundle` + `tls_certs_merge` 与平台根证书合并；`clippy.toml` 拦截 `danger_accept_invalid_certs` / `_hostnames` 及其 `tls_danger_*` 新名，用临时探针确认四条都会报错（D-09）。
- 值解码覆盖 boolean、整型族、real / double（含 `"NaN"` / `"Infinity"` / `"-Infinity"`）、decimal / number（保持字符串）、varchar / char、date、timestamp、timestamp with time zone（zone id 与偏移量两种形态原样保留）、array 递归；map 等未建模类型与形状不符的单元格降级为 `Value::Unsupported`，不让查询失败。
- `describe_tables` 对同一 catalog 只发一条由 printer 生成的 `information_schema.columns` 查询（多表 `Or` 组合 `And`，单表直接用 `And`，过滤值小写，结果按调用方传入的 `TableRef` 归组）；`CATALOG_NOT_FOUND` / `SCHEMA_NOT_FOUND` 返回空 map，被 `max_rows` 截断返回 `Protocol`（含 catalog 名与 `truncated`），跨 catalog 返回 `Config` 且不发请求；`status()` 用 `SELECT 1`（5 s、max_rows 1）探测。
- 最终门：`cargo test --workspace --locked` 421 通过、0 失败（02-02 的基线 340，本 plan 的 `ontoforge-engine` 新增 37 个测试，其余来自 02-03），`cargo clippy --workspace --all-targets --locked -- -D warnings` 与 `cargo fmt --all --check` 通过。

## Task Commits

1. **Task 1: 端到端 tracer（tdd）**
   - RED `e769e75`（test）：8 个行为测试，存根 `execute` 返回 `Cancelled`，全部因断言失败
   - GREEN `77e4f75`（feat）
2. **Task 2: 服务配置、密钥引用、认证、自定义 CA 与双身份（tdd）**
   - RED `6e0a445`（test）：17 个配置 / 构造期测试加 `basic_and_bearer_headers`，函数存根使其全部因断言失败
   - GREEN `24fe5d9`（feat，含 `clippy.toml`）
3. **Task 3: 完整解码、describe_tables、status（tdd）**
   - RED `c796da2`（test）：9 个测试失败（解码 2、describe 5、status 1，另 `describe_mixed_catalogs_is_config_error` 在空输入断言上失败）
   - GREEN `ce7d58b`（feat）

**Plan metadata:** 本文件所在的 `docs(02-04)` 提交。无 REFACTOR 提交。

## Tracer 反馈门

`workflow.human_verify_mode = end-of-phase` 且 Task 1 的 `<verify>` 只含 `<automated>`、auto 模式未开：Task 1 提交前重跑 `cargo test -p ontoforge-engine --test trino_client`（8 通过）、`cargo clippy -p ontoforge-engine --all-targets -- -D warnings` 与 `sql_text_only_from_printer`，通过后继续展开。

## Files Created/Modified

- `crates/ontoforge-engine/src/trino/mod.rs` - `Auth`、`TrinoConnection`、`TrinoEngine`（传输约束、CA、execute 循环、describe_tables、status）
- `crates/ontoforge-engine/src/trino/client.rs` - `send_once`：一次请求、状态码映射、有界读取、错误文本不含 URL
- `crates/ontoforge-engine/src/trino/headers.rs` - 每请求头组装、trace token / tag 校验、`apply_auth` 及其单元测试
- `crates/ontoforge-engine/src/trino/decode.rs` - `QueryResults` 线格式与按类型解码
- `crates/ontoforge-engine/src/config.rs` - 配置结构、`parse_server_config`、`load_server_config`、`resolve_secret`、`build_trino_engines`
- `crates/ontoforge-engine/src/lib.rs` - `pub mod config` / `pub mod trino`、`AnyEngine::Trino`
- `clippy.toml` - D-09 的 `disallowed-methods`
- `crates/ontoforge-engine/tests/{trino_client,server_config}.rs`、`tests/data/test-ca.pem` - 协议、配置与 CA 测试

## Decisions Made

- 装箱 `TrinoConnection` 而不是给 `AnyEngine` 加 `allow(large_enum_variant)`：结构体内部装箱不改公开形状，`AnyEngine::Trino(TrinoEngine)` 与计划的 rg 检查一致。
- `execute` 现在就实现行数截断，而不是等 02-06：`describe_tables` 的「截断即错误」需要 `truncated`，且没有行数上限的循环会无界累加。字节上限、`DELETE` 取消、重试、整体超时留给 02-06。
- 逐请求不设 `reqwest` 超时：计划把超时与取消归 02-06，本 plan 内一个不响应的网关会让 `execute` 挂住，02-06 必须补上。
- `status()` 的 client tag 用 `ontoforge`（计划只规定 trace token、超时与 max_rows），使其落入同一个 resource group。
- 错误文本的泄露面收紧超出计划的部分：reqwest 错误去掉 URL（含 query id），`QueryResults` 解析失败不引用单元格内容，origin / tag 的配置错误不回显条目。

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - 阻塞] clippy pedantic**
- **Found during:** Task 1、2、3 的 clippy 门
- **Issue:** `large_enum_variant`（`AnyEngine` 加 Trino 变体后 304 对 96 字节）、`unused_async_trait_impl`（Task 1 的 describe_tables / status 占位）、`needless_pass_by_value`（`read_error`、测试里的 `page`）、`single_match_else`、`useless_format`、`needless_borrow`。
- **Fix:** 装箱连接；占位处带原因的 `allow`，Task 3 实现后随占位一起消失；其余按 lint 建议改写。
- **Files modified:** `trino/mod.rs`、`config.rs`、`tests/server_config.rs`、`tests/trino_client.rs`
- **Committed in:** `77e4f75`、`24fe5d9`、`ce7d58b`

**2. [Rule 3 - 阻塞] RED 证据格式**
- **Found during:** Task 1、2、3 RED
- **Issue:** `gsd_run check tdd-red-evidence` 解析 node TAP，cargo 输出不是 TAP（与 02-01、02-02 相同）。
- **Fix:** 人工核对每个目标测试都因对计划行为的断言失败，而非编译错误；为可编译先提交了签名存根。Task 1 的第一版存根返回 `Protocol`，使 `foreign_next_uri_is_rejected` 意外 GREEN，已改为返回 `Cancelled` 后再提交 RED。
- **Verification:** RED 提交前逐条检查 `cargo test` 输出。

**3. [Rule 1 - 计划前提不成立] 两个回归锁定测试在 RED 时已通过**
- **Found during:** Task 2 RED
- **Issue:** `next_uri_scheme_mismatch_is_rejected` 与 `two_identities_use_separate_clients` 测试的是 Task 1 已实现的行为（nextUri 比较本就含 scheme，`TrinoEngine` 本就独占 client），RED 时就通过，属意外 GREEN。
- **Fix:** 保留为回归锁定，不当作 Task 2 的 RED 证据；Task 2 的 RED 由配置、构造期与认证头测试承担。

**4. [Rule 2 - 补充] 计划外的加固**
- **Found during:** Task 1、2
- **Issue:** 计划未覆盖：nextUri 携带 userinfo、url 无 host 或带 userinfo、空 user、响应页大小无上限、配置里 `next_uri_origins` 条目含路径。
- **Fix:** 对应地拒绝（`checked_next_uri`、`check_transport`、`send_once` 的 `max_page_bytes`、配置期的 bare origin 检查）；Task 1 之外无新依赖。
- **Committed in:** `77e4f75`、`24fe5d9`

**5. [Rule 3 - 范围对齐] `max_rows` 截断提前到本 plan**
- **Found during:** Task 3
- **Issue:** 见 Decisions Made。计划把行数 / 字节上限整体归 02-06，但 `describe_truncated_metadata_is_error` 需要 `QueryOutput.truncated`。
- **Fix:** 只在 `Collector` 里实现 `max_rows`；`max_bytes` 字段暂未使用，留给 02-06。
- **Committed in:** `77e4f75`

---

**Total deviations:** 5（2 个阻塞、1 个前提修正、1 个补充、1 个范围对齐）
**Impact on plan:** 无范围蔓延；公开 API 与 interfaces 一致，额外加固都在计划指定的文件内。

## TDD Gate Compliance

- Task 1：RED `e769e75`（`test(02-04)`）→ GREEN `77e4f75`（`feat(02-04)`）。
- Task 2：RED `6e0a445` → GREEN `24fe5d9`；意外 GREEN 的两个测试见偏差 3。
- Task 3：RED `c796da2` → GREEN `ce7d58b`。
- 无 REFACTOR 提交。

## Issues Encountered

- 一次 bash heredoc 里夹了对 `python -` 的调用（无输入），命令挂起超过 120 s 被移入后台；已用 `taskkill` 结束该 bash 进程树，仓库没有半写入文件。之后大段文本一律用 Write 工具落盘。
- Git for Windows 的 `openssl` 把 `-subj "/CN=.."` 当 POSIX 路径改写，并且不认 `/tmp`；用 `MSYS_NO_PATHCONV=1` 与 `cygpath -m` 解决。
- Windows 上 `cargo test --workspace` 超过默认 2 分钟的工具超时，改用 590 s 超时运行，结果不受影响。
- `test-ca.pem` 在 Windows 工作区为 CRLF，git 提交时提示会转为 LF，PEM 解析两者都接受。

## Known Stubs

None。Task 1 的 `describe_tables` / `status` 占位已在 Task 3 替换。`TrinoConfig.max_bytes` 已进入 `QueryContext` 但 `TrinoEngine::execute` 尚未按字节截断，这是 02-06 的范围，不是占位值。

## Threat Flags

None。T-02-11（nextUri 同源）、T-02-12（密钥不进错误 / Debug / 请求体）、T-02-13（TLS 校验不可关）、T-02-14（http + 凭据，库层强制）、T-02-44（https 降级 origin）、T-02-15（头注入）、T-02-16（有界读取）、T-02-17（忽略 `X-Trino-Set-*`）的缓解均已落地并有测试。本 plan 新增的网络出口（对配置的 Trino 发请求）就是计划威胁模型的对象，没有计划外的端点或认证路径。

## User Setup Required

None - no external service configuration required。真实部署需要在环境里提供 `*_env` 指向的变量或 `*_file` 指向的密钥文件，那是 02-08 CLI 接入时的事。

## Next Phase Readiness

- 02-05 / 02-08 / 02-09 可以用 `build_trino_engines` 得到 `TrinoEngine`，用 `TrinoConfig::query_context` 构造查询上下文，用按 catalog 批量的 `describe_tables` 做在线校验，用 `is_engine_wide()` 分流错误。
- 02-06 需要补：重试（502 / 503 / 504 与 429）、整体超时、`DELETE nextUri` 取消、客户端断连取消、`max_bytes` 截断，以及 `Unavailable` 的完整分类；入口是 `client::send_once` 与 `Collector`。目前没有逐请求超时，一个不应答的网关会让 `execute` 挂住。
- 待后续覆盖：真实 Trino 上的线格式与 `information_schema` 行为属 02-10 的 Linux CI golden 测试；Windows 上符号链接测试在无权限创建链接时自动跳过该断言。

## Self-Check: PASSED

- 创建的文件（`clippy.toml`、`config.rs`、`trino/{mod,client,headers,decode}.rs`、两个测试文件、`tests/data/test-ca.pem`）均存在于磁盘。
- 提交 `e769e75`、`77e4f75`、`6e0a445`、`24fe5d9`、`c796da2`、`ce7d58b` 均存在，`git log --grep=02-04` 返回 6 条；`git diff --diff-filter=D` 对每个提交无意外删除。
- Task 1 acceptance（`Policy::none`、`X-Trino-Trace-Token`、`query_max_execution_time`、`Trino(trino::TrinoEngine)`、`foreign_next_uri_is_rejected` 1 passed）、Task 2 acceptance（clippy.toml 两条 rg、`SecretString`、四个目标测试各 1 passed、`fn read_limited` / `sqlite_dir` / `30_?000` 负向检查无匹配）、Task 3 acceptance（`information_schema`、`print_select`、`CATALOG_NOT_FOUND`、三个目标测试各 1 passed）逐条重跑通过。
- 最终门：`cargo test --workspace --locked`（421 通过、0 失败）、`cargo clippy --workspace --all-targets --locked -- -D warnings`、`cargo fmt --all --check` 全部通过；无 `.snap.new` 残留，`Cargo.lock` 未改。

---
*Phase: 02-source-binding-engine*
*Completed: 2026-10-17*
