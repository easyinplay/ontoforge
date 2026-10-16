---
phase: 02-source-binding-engine
plan: 06
subsystem: engine
tags: [rust, trino, reqwest, wiremock, retry, cancellation, timeout, fail-closed]

requires:
  - phase: 02-source-binding-engine
    provides: "02-04 的 TrinoEngine / client::send_once / Collector / nextUri 同源校验；02-02 的 EngineError 全部变体与 QueryContext"
provides:
  - "trino/classify.rs：HTTP 状态 / reqwest 错误 / Trino errorName 到 EngineError 与重试决策的唯一映射表（Outcome、classify_status、classify_transport、classify_query_error、transport_failure、MAX_RETRIES）"
  - "trino/cancel.rs：QueryGuard，超时 / 上限 / 协议错误经 cancel_now、调用方丢弃 future 经 Drop 向最新 nextUri 发 DELETE"
  - "client::send：带重试循环的单请求入口（GET 的 502/503/504 退避、429 Retry-After、POST 仅在能证明语句未被接受时重试、连接未建立重试）"
  - "TrinoEngine::execute 受 QueryContext.timeout 约束（含全部重试），max_bytes 截断，单页上限中止并取消"
affects: [02-07, 02-08, 02-09, 02-10, phase-04, phase-07]

plan_head_before: 32a6e2a78808fa772c45c5499e527085a7a3ce16

actuals:
  tokens: 14000
  tasks: 3
  commits: 6

tech-stack:
  added: []
  patterns:
    - "所有状态码 / 传输 / errorName 到 EngineError 的映射集中在 classify.rs 一个文件，client.rs 与 mod.rs 不再自己造 Unavailable"
    - "cancel 请求在拿到每个已校验 nextUri 时就预先构造好（带认证头）存进 guard，guard 自身不接触凭据"
    - "wiremock 无法构造不带 Content-Length 的分块响应，这类测试用 std::net 的裸 TCP 服务器"

key-files:
  created:
    - crates/ontoforge-engine/src/trino/classify.rs
    - crates/ontoforge-engine/src/trino/cancel.rs
    - crates/ontoforge-engine/tests/trino_resilience.rs
  modified:
    - crates/ontoforge-engine/src/trino/client.rs
    - crates/ontoforge-engine/src/trino/mod.rs
    - crates/ontoforge-engine/src/trino/decode.rs

key-decisions:
  - "QueryGuard::advance 接收预构造的 DELETE RequestBuilder 而非 Option<Url>：guard 不必持有 Auth（SecretString），且 guard 里只可能出现经同源校验的 nextUri 对应的请求"
  - "classify_transport 去掉计划签名里的 is_post 参数：连接未建立对 GET 与 POST 同样可重试，参数无用；改为按值接收 reqwest::Error（需要 without_url 去掉含 query id 的 URL）"
  - "服务端已报告失败的 Query 错误不发 DELETE（查询在服务端已结束），其余任何错误、超时、截断都会 cancel_now"
  - "max_bytes 以保留行的紧凑 JSON 字节数累计（行级粒度，第一个放不下的行起丢弃并 truncated）"
  - "retries_respect_timeout 测试从 Task 2 移到 Task 3：它依赖 Task 3 才有的整体超时"

patterns-established:
  - "Retry-After 只解析整数秒，上限 5 s，缺省或无法解析为 1 s；退避 50 ms 起翻倍上限 1 s，最多 5 次重试（共 6 次请求）"
  - "POST 的 503 先读响应体取 error.errorName，只有 SERVER_STARTING_UP 才重试（E16）"

requirements-completed: [ENGINE-01, ENGINE-02]

coverage:
  - id: D1
    description: "Trino 不可达（连接被拒、DNS、SERVER_STARTING_UP / SERVER_SHUTTING_DOWN 页与 503 体）一律 ENGINE_UNAVAILABLE retryable，401 / 403 为 Unauthorized，查询错误为 Query；status() 报告不可用且 reason 不含凭据"
    requirement: "ENGINE-02"
    verification:
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_resilience.rs#closed_port_is_unavailable"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_resilience.rs#dns_failure_is_unavailable"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_resilience.rs#server_starting_up_is_unavailable"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_resilience.rs#server_shutting_down_is_unavailable"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_resilience.rs#http_503_with_shutting_down_body_is_unavailable"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_resilience.rs#unauthorized_is_not_unavailable"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_resilience.rs#syntax_error_is_query"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_resilience.rs#status_reports_unavailable"
        status: pass
    human_judgment: false
  - id: D2
    description: "协议规定的重试：GET 502/503/504 退避至多 5 次，429 按 Retry-After（上限 5 s），POST 只在 429、503 + SERVER_STARTING_UP、连接未建立时重试，普通 503 / 502 / 504 不重试并说明原因（E16）"
    requirement: "ENGINE-01"
    verification:
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_resilience.rs#get_503_then_success"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_resilience.rs#get_502_and_504_are_retried"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_resilience.rs#exhausted_503_is_unavailable"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_resilience.rs#retry_after_is_honoured"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_resilience.rs#retry_after_is_capped"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_resilience.rs#post_502_is_not_retried"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_resilience.rs#post_503_starting_up_is_retried"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_resilience.rs#post_plain_503_is_not_retried"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_resilience.rs#connect_failure_is_retried_with_backoff"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-engine/src/trino/classify.rs#tests"
        status: pass
    human_judgment: false
  - id: D3
    description: "超时取消：超过 QueryContext.timeout（重试也算在内）向当前 nextUri 发 DELETE 后返回 Timeout；尚无 nextUri 时不发 DELETE"
    requirement: "ENGINE-01"
    verification:
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_resilience.rs#timeout_sends_delete"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_resilience.rs#hanging_submission_times_out_without_delete"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_resilience.rs#retries_respect_timeout"
        status: pass
    human_judgment: false
  - id: D4
    description: "断连取消：调用方丢弃（abort）查询 future 时 guard 的 Drop 向当前 nextUri 发带相同 X-Trino-User 的 DELETE；正常完成或服务端报错的查询不发 DELETE；无 tokio runtime 时 drop 不 panic"
    requirement: "ENGINE-01"
    verification:
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_resilience.rs#dropped_future_sends_delete"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_resilience.rs#finished_query_sends_no_delete"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_resilience.rs#failed_query_sends_no_delete"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-engine/src/trino/cancel.rs#dropping_an_unfinished_guard_without_a_runtime_does_not_panic"
        status: pass
    human_judgment: false
  - id: D5
    description: "行数 / 字节上限与单页上限：超过 max_rows 或 max_bytes 停止拉取、DELETE 当前 nextUri、返回已取得结果并置 truncated；未达上限不截断不取消；单页按块读取超过 max_page_bytes（不依赖 Content-Length）返回 Protocol 并 DELETE"
    requirement: "ENGINE-01"
    verification:
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_resilience.rs#max_rows_truncates_and_deletes"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_resilience.rs#max_bytes_truncates_and_deletes"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_resilience.rs#limits_not_reached_do_not_truncate"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/trino_resilience.rs#oversized_page_is_protocol_error"
        status: pass
    human_judgment: false
  - id: D6
    description: "真实 Trino 上 DELETE nextUri 的返回码（204 / 404）与服务端资源释放、网关 503 的真实形态"
    requirement: "ENGINE-01"
    verification: []
    human_judgment: true
    rationale: "本 plan 只在 wiremock 与裸 TCP 服务器上验证；真实 Trino 的行为由 02-10 的 Linux CI golden 测试兜底（RESEARCH A5）"

duration: 11min
completed: 2026-10-17
status: complete
---

# Phase 2 Plan 06: Trino 重试、取消与 Unavailable 分类 Summary

**Trino client 的故障行为现在确定：所有不可达情形归为 `ENGINE_UNAVAILABLE`，GET 按协议退避重试而 POST 只重发能证明未被接受的提交（E16），超时、客户端断连、行数 / 字节上限与超大页四种情形都会 `DELETE` 当前 `nextUri`。**

## Performance

- **Duration:** 11 min
- **Started:** 2026-10-16T20:46:36Z
- **Completed:** 2026-10-16T20:57:26Z（实现与最终门；SUMMARY 提交在其后）
- **Tasks:** 3（Task 1 tracer、Task 2 auto/tdd、Task 3 auto/tdd）
- **Files modified:** 6（相对 `32a6e2a`：新增 `classify.rs`、`cancel.rs`、`trino_resilience.rs`，修改 `client.rs`、`mod.rs`、`decode.rs`；`Cargo.toml` 与 `Cargo.lock` 未改）

## Accomplishments

- tracer 先打通最薄路径：`classify.rs` 成为 HTTP 状态、`reqwest::Error`、页内 / 503 体 `errorName` 到 `EngineError` 的唯一映射。连接被拒、DNS、TLS 握手、`SERVER_STARTING_UP` / `SERVER_SHUTTING_DOWN`（200 页内或 503 体）全部是 `Unavailable { retryable: true }`，401 / 403 是 `Unauthorized`，其余 errorName 是 `Query`，`status()` 在这些情形下得到 `Unavailable` 且 reason 不含 `Authorization`。传输错误文本去掉 URL（含 query id），保留原因链。
- 重试：GET 的 502 / 503 / 504 与连接未建立按 50 ms 起翻倍、上限 1 s 退避，最多 5 次重试（共 6 次请求），用尽后 `Unavailable`，reason 写明 `after 6 attempts`；429 按 `Retry-After` 整数秒等待（缺省 1 s，上限 5 s，日期格式或乱码也按 1 s）。POST 的 503 先读响应体取 `error.errorName`，只有 `SERVER_STARTING_UP` 才重试；普通 503（网关 HTML、空体）、502、504 一次都不重发，reason 写明 `not retried because the statement may have been accepted`。
- 取消：`QueryGuard` 在拿到每个已校验的 `nextUri` 时保存预先构造（带认证头）的 `DELETE`，`execute` 在超时、截断、协议 / 不可用 / 认证等错误时 `cancel_now`（2 s 上限，2xx 与 404 都视为成功），在调用方丢弃 future 时由 `Drop` 经 `Handle::try_current()` fire-and-forget 发出，拿不到 runtime 则放弃。正常完成、服务端已报告 `Query` 错误的查询不发 `DELETE`。
- 超时：整个分页与重试循环由 `tokio::time::timeout(ctx.timeout, ..)` 包住，重试不越过超时；提交阶段挂起（还没有 `nextUri`）时返回 `Timeout` 且不发 `DELETE`。
- 上限：`Collector` 增加 `max_bytes`（保留行的紧凑 JSON 字节数，行级粒度），与 `max_rows` 同样置 `truncated` 并 `cancel_now`；单页读取沿用逐块累计，超过 `max_page_bytes` 即停止读取返回 `Protocol` 并 `DELETE`，裸 TCP 分块响应（无 `Content-Length`）的测试证明不依赖该头。
- 最终门：`cargo test --workspace --locked` 492 通过、0 失败（02-04 SUMMARY 的基线 421 加 02-05 与本 plan；本 plan 新增 34：`trino_resilience` 27、`classify` 单元 5、`cancel` 单元 2），`cargo clippy --workspace --all-targets --locked -- -D warnings` 与 `cargo fmt --all --check` 通过。

## Task Commits

1. **Task 1: 端到端 tracer（tdd）**
   - RED `b4c38c7`（test）：8 个测试，其中 3 个因断言失败（`SERVER_STARTING_UP` / `SERVER_SHUTTING_DOWN` 页返回 `Query`、503 体返回 `Protocol`），另 5 个是回归锁定
   - GREEN `fe72d4f`（feat）
2. **Task 2: 协议规定的重试（tdd）**
   - RED `cb56e85`（test）：9 个新测试，8 个因断言失败，`post_502_is_not_retried` 在 RED 时已通过
   - GREEN `3fc6c0c`（feat）
3. **Task 3: 三个取消触发点与单页上限（tdd）**
   - RED `f8fc627`（test）：10 个新测试，7 个因断言失败（timeout、hanging 提交、retries_respect_timeout、dropped future、max_rows、max_bytes、oversized 的 DELETE 断言），`finished_query_sends_no_delete`、`failed_query_sends_no_delete`、`limits_not_reached_do_not_truncate` 为回归锁定
   - GREEN `9b6f9c4`（feat）

**Plan metadata:** 本文件所在的 `docs(02-06)` 提交。无 REFACTOR 提交。

## Tracer 反馈门

`workflow.human_verify_mode = end-of-phase`，Task 1 的 `<verify>` 只含 `<automated>`，auto 模式未开：Task 1 提交前重跑 `cargo test -p ontoforge-engine --test trino_resilience --test trino_client`（27 通过）与 `cargo clippy`，通过后继续展开。

## Files Created/Modified

- `crates/ontoforge-engine/src/trino/classify.rs` - `Outcome`、`classify_status`、`classify_query_error`、`classify_transport`、`transport_failure`、`MAX_RETRIES`，含 5 个单元测试
- `crates/ontoforge-engine/src/trino/cancel.rs` - `QueryGuard`（`advance` / `finish` / `cancel_now` / `Drop`），含 2 个单元测试
- `crates/ontoforge-engine/src/trino/client.rs` - `send`（重试循环）、`exchange`（单次交换并分类）、POST 503 体探测、逐块读取
- `crates/ontoforge-engine/src/trino/mod.rs` - `execute`（超时与 guard 收尾）、`run`（分页循环）、`Collector` 的 `max_bytes`、`json_len`
- `crates/ontoforge-engine/src/trino/decode.rs` - 移除 `WireError::into_engine_error`（错误映射并入 `classify.rs`）
- `crates/ontoforge-engine/tests/trino_resilience.rs` - 27 个 wiremock / 裸 TCP 测试

## Decisions Made

- `QueryGuard::advance` 收预构造的 `DELETE` 请求而不是 `Option<Url>`：guard 不碰 `Auth`，且其中只可能是经同源校验的 `nextUri` 对应的请求（T-02-27）。
- 服务端已报告的 `Query` 错误不发 `DELETE`；其余错误（含 `Unavailable`、`Unauthorized`、`Protocol`）一律 `cancel_now`，最多多花 2 s。
- `max_bytes` 按保留行的紧凑 JSON 字节数累计，行级而非页级截断；单个放不下的首行得到空结果加 `truncated`。
- 连接未建立的重试对 GET 与 POST 一视同仁，不论是拒绝、DNS 还是 TLS 握手失败（reqwest 都报为 connect 错误）。

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - 范围对齐] `classify_transport` 签名与 `advance` 参数偏离 interfaces**
- **Found during:** Task 2、Task 3
- **Issue:** interfaces 写的是 `classify_transport(err: &reqwest::Error, is_post, attempt)` 与 `advance(next: Option<Url>)`。`is_post` 在规则里没有区别（连接未建立对两者都重试）会成为未使用参数；`&reqwest::Error` 无法 `without_url`，错误文本会带含 query id 的 URL；`advance(Url)` 要求 guard 持有认证。
- **Fix:** `classify_transport(err: reqwest::Error, attempt)`；另设 `transport_failure(err)` 供读取响应体中途的错误使用；`advance` 收预构造的 `DELETE` 请求。
- **Files modified:** `trino/classify.rs`、`trino/cancel.rs`、`trino/mod.rs`
- **Committed in:** `3fc6c0c`、`9b6f9c4`

**2. [Rule 3 - 阻塞] 计划外触碰 `decode.rs`，`retries_respect_timeout` 移到 Task 3**
- **Found during:** Task 1、Task 3
- **Issue:** `WireError::into_engine_error` 在 `decode.rs`，不经 `classify` 就会让 `SERVER_STARTING_UP` 页变成 `Query`；`retries_respect_timeout` 依赖 Task 3 才有的整体超时，放在 Task 2 无法转绿。
- **Fix:** 删除该方法，映射并入 `classify_query_error`（`decode.rs` 不在计划文件清单也不在并行约束的禁改清单内）；测试放进 Task 3 的 RED。
- **Committed in:** `fe72d4f`、`f8fc627`

**3. [Rule 3 - 阻塞] RED 证据格式**
- **Found during:** Task 1、2、3 RED
- **Issue:** `gsd_run check tdd-red-evidence` 解析 node TAP，cargo 输出不是 TAP（与 02-01、02-02、02-04 相同）。
- **Fix:** 人工核对每个目标测试都因对计划行为的断言失败，而非编译错误（本 plan 的测试只用公开 API，RED 时无需存根即可编译）。
- **Verification:** RED 提交前逐条检查 `cargo test` 输出。

**4. [Rule 1 - 计划前提不成立] 若干回归锁定测试在 RED 时已通过**
- **Found during:** Task 1、2、3 RED
- **Issue:** 02-04 的 `send_once` 已把连接失败映射为 `Unavailable`、401 / 403 映射为 `Unauthorized`，POST 的 502 本就不重试，所以 `closed_port_is_unavailable`、`dns_failure_is_unavailable`、`unauthorized_is_not_unavailable`、`syntax_error_is_query`、`status_reports_unavailable`、`post_502_is_not_retried`、`finished_query_sends_no_delete` 等在 RED 时就是绿的。
- **Fix:** 保留为回归锁定，不当作 RED 证据；各 Task 的 RED 由确实失败的测试承担。

---

**Total deviations:** 4（3 个阻塞 / 范围对齐、1 个前提修正）
**Impact on plan:** 公开行为与 must_haves 一致；偏离只在 crate 内部签名与测试放置，无范围蔓延、无新依赖。

## Issues Encountered

- 一个 bash heredoc 里含大段 Rust 与内嵌 Python heredoc，shell 报 `unexpected EOF`，未写入任何文件；之后大段测试文本改用 Edit 工具落盘。
- Windows 上对关闭端口的连接失败每次约耗满 `connect_timeout`：`closed_port_is_unavailable` 现在是 6 次尝试加 1.55 s 退避，整个 `trino_resilience` 目标约 7.7 s，低于计划的 10 s 上限但余量不大。
- `status()` 自带 5 s 超时，在 Windows 上对关闭端口的重试总时长（约 7.6 s）长于它，因此该情形下 reason 是 `query timed out after 5000 ms` 而不是连接被拒的原因链；仍是 `Unavailable` 且不含凭据。

## Known Stubs

None。

## Threat Flags

None。T-02-23（失控查询：超时 / 断连 / 上限三个触发点都 `DELETE`）、T-02-24（逐块累计，超 `max_page_bytes` 立即中止，裸 TCP 分块测试不依赖 `Content-Length`）、T-02-25（Retry-After 上限 5 s，重试受 `ctx.timeout` 约束）、T-02-26（POST 仅在能证明未被接受时重试，`post_plain_503_is_not_retried` / `post_503_starting_up_is_retried`）、T-02-27（guard 只持有经同源校验的 `nextUri` 的 `DELETE`）的缓解均已落地并有测试。无计划外的端点或认证路径。

## User Setup Required

None - no external service configuration required。

## Next Phase Readiness

- 02-08 / 02-09 可以依赖 `EngineError::Unavailable` 的完整分类：`is_engine_wide()` 对 `Unavailable`、`Unauthorized`、`Cancelled` 为真，CLI 映射为 ONT-V19 + 退出码 3 的代码在 02-09。
- 待后续覆盖：真实 Trino 上 `DELETE nextUri` 的 204 / 404、网关 503 的真实形态（RESEARCH A5）属 02-10 的 Linux CI golden 测试；`status()` 的 reason 在关闭端口 + 重试超过 5 s 时是超时文本，若 `/health` 需要更精确的原因，由 Phase 7 决定是否给 `status()` 单独的快速探测路径。

## Self-Check: PASSED

- 创建的文件（`classify.rs`、`cancel.rs`、`tests/trino_resilience.rs`）均存在于磁盘，工作树中只剩开工前就有的未跟踪路径。
- 提交 `b4c38c7`、`fe72d4f`、`cb56e85`、`3fc6c0c`、`f8fc627`、`9b6f9c4` 均存在，`git rev-list --count 32a6e2a..HEAD` 为 6；每个提交 `git diff --diff-filter=D` 无意外删除。
- 三个 Task 的 acceptance 逐条重跑通过（`SERVER_STARTING_UP` / `SERVER_SHUTTING_DOWN` / `Retry-After` / `MAX_RETRIES` / `impl Drop for QueryGuard` / `try_current` / `chunk()` 的 rg 匹配，`closed_port_is_unavailable`、`post_502_is_not_retried`、`post_503_starting_up_is_retried`、`post_plain_503_is_not_retried`、`dropped_future_sends_delete` 各 1 passed）。
- 最终门：`cargo test --workspace --locked`（492 通过、0 失败）、`cargo clippy --workspace --all-targets --locked -- -D warnings`、`cargo fmt --all --check` 全部通过；`trino_resilience` 连续运行 4 次稳定；`Cargo.lock` 未改。

---
*Phase: 02-source-binding-engine*
*Completed: 2026-10-17*
