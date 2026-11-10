---
phase: 08-production-webhook-end-to-end-delivery
plan: "01"
subsystem: testing
tags: [webhook, outbox, postgres, ci, axum, end-to-end, retry, ledger]

requires:
  - phase: 05-online-action
    provides: webhook outbox、`X-Ontoforge-Event-Id` 请求头、`1 << (attempts-1)` 退避与 `ontoforge webhook-worker` CLI 入口
  - phase: 07-rest-mcp-shell
    provides: 真实 CLI 二进制的 spawn 模式、CI 的 PostgreSQL service container 与 artifact 惯例
provides:
  - 独立进程 HTTP 接收端 example target（只写 JSONL ledger，不做断言、不暴露聚合端点、不以退出码表达成败）
  - 被 `#[ignore]` 的 PostgreSQL 端到端用例（提交 Action → 起接收端与 worker → 轮询 ledger → 跨来源交叉取证）
  - 独立的 `phase8-webhook-e2e` CI job（起 PG、建 editstore 库、构建 example、跑被 ignore 的用例、凭据扫描、上传 ledger artifact、清理容器）
  - Phase 8 的 RED / GREEN 两态 CI 证据与本机 ledger 留证
affects: [08-02-evidence-archive, phase-09-webhook-supervision, phase-12-first-public-release]

actuals:
  tokens: 8087
  tasks: 2
  commits: 3

plan_head_before: a40248e6d40b18da2d9efa2eee88f4ebb2a8ee7d
plan_head_after: ae8956dd280f6a3c1c51a06a84d1d7a82f9ffa23
commits: 3

tech-stack:
  added:
    - "axum（`ontoforge` 的 dev-dependency，workspace pin `=0.8.9`；examples 可用，不进发行包依赖集）"
  patterns:
    - "独立进程接收端 + 进程编排收进 Rust 测试（`Child` 加 `impl Drop { kill; wait }`），CI shell 不起后台进程"
    - "判定只从接收端原始 ledger 与 PostgreSQL `of_webhook_outbox` 终态两处独立取证"

key-files:
  created:
    - crates/ontoforge/examples/webhook_receiver.rs
    - crates/ontoforge/tests/webhook_e2e_pg.rs
  modified:
    - crates/ontoforge/Cargo.toml
    - Cargo.lock
    - .github/workflows/ci.yml
    - .planning/WINDOWS.md

key-decisions:
  - "接收端以 example target 形态存在：不是 `[[bin]]`、不是 `ontoforge` 子命令、不新增 workspace member（D-01）；发行打包脚本只收 binary 与 manifest，example 天然不入包"
  - "判定完全在 CI 侧：接收端只记录原始事实，不做断言、不暴露 `/counts`、不以退出码表达成败（D-02）"
  - "`[webhook]` 只写 `WebhookWorkerConfig` 的五个键，`deny_unknown_fields` 下多一个键整份配置就解析失败"
  - "CI job 用 `if-no-files-found: warn` 上传 artifact，避免测试失败被第二个 artifact 错误掩盖"
  - "本机验证口径是编译级；端到端用例是 `#[ignore]` 的 CI-only 用例（Windows 本机无 Docker，D-03 要求 PG）"

requirements-completed: [REL-01, REL-02]

coverage:
  - id: D1
    description: "独立进程 HTTP 接收端：绑定 127.0.0.1 的 OS 分配端口，逐请求追加一行 JSONL ledger"
    requirement: REL-01
    verification:
      - kind: integration
        ref: "crates/ontoforge/tests/webhook_e2e_pg.rs#webhook_worker_delivers_to_an_independent_receiver_over_postgres"
        status: pass
      - kind: e2e
        ref: "CI run 42510584474 / job phase8-webhook-e2e (https://github.com/easyinplay/ontoforge/actions/runs/42510584474)"
        status: pass
    human_judgment: false
  - id: D2
    description: "单事件纵切片：同一 event_id 的第 1 次投递得 503，worker 按 1 秒退避重试后转 2xx，相邻间隔落在 [0.9s, 30s]"
    requirement: REL-02
    verification:
      - kind: e2e
        ref: "CI run 42508002310 / job phase8-webhook-e2e，ledger 两行 503→204，间隔 1.020 s"
        status: pass
    human_judgment: false
  - id: D3
    description: "多事件去重集合与至少一次送达：三条事件的唯一 event_id 集合与提交集合完全相等，无事件被静默丢弃"
    requirement: REL-02
    verification:
      - kind: e2e
        ref: "CI run 42510584474 / job phase8-webhook-e2e，ledger 六个原始请求、三个唯一 event_id"
        status: pass
    human_judgment: false
  - id: D4
    description: "跨来源交叉取证：PostgreSQL `of_webhook_outbox` 恰好三行且 delivered_at 非空、attempt_count=1、last_error='http_status_503'；接收端是独立进程；ledger 不含连接串与密码"
    requirement: REL-02
    verification:
      - kind: integration
        ref: "crates/ontoforge/tests/webhook_e2e_pg.rs#assert_outbox_evidence"
        status: pass
    human_judgment: false
  - id: D5
    description: "`phase8-webhook-e2e` CI job 与 ledger artifact 上传，含上传前的凭据扫描"
    requirement: REL-01
    verification:
      - kind: other
        ref: "CI run 42510584474，artifact phase8-webhook-ledger 已上传（check-artifacts 先于上传执行）"
        status: pass
    human_judgment: false
  - id: D6
    description: "人工复核 ledger 原始记录与 CI 日志并签署 Phase 8 的结论位"
    requirement: REL-01
    verification: []
    human_judgment: true
    rationale: "项目惯例：机器可判定结果只作证据，结论位固定 `VERDICT: PENDING`，执行者不代签；REL-03 的 systemd 自动拉起属 Phase 9，不在本 phase"

# Metrics
duration: 3h 12m
completed: 2026-11-09
status: complete
---

# Phase 8 Plan 1: Production Webhook End-to-End Delivery Summary

**独立进程 HTTP 接收端 + 真实 `ontoforge webhook-worker` 二进制 + 真实 PostgreSQL 的一条端到端投递链路，把 503 重试、`event_id` 去重与至少一次送达变成可从原始记录复算的 CI 证据**

## Performance

- **Duration:** 3h 12m
- **Started:** 2026-11-08T16:21:02Z
- **Completed:** 2026-11-08T19:33:29Z
- **Tasks:** 2
- **Files modified:** 6
- **PR:** https://github.com/easyinplay/ontoforge/pull/12

## Accomplishments

- **接收端 harness（`crates/ontoforge/examples/webhook_receiver.rs`）**：独立 OS 进程，绑 `127.0.0.1` 的 OS 分配端口，把完整 target URL 写进 ready-file，每次请求追加恰好一行 JSONL（`seq` / `received_at` / `method` / `path` / `header_event_id` / `payload_event_id` / `event_id` / `attempt_seq` / `status_code` / `body_len` / `remote_addr`）。逐条无缓冲写入并 flush，读侧轮询能立刻看到最新一行。`event_id` 优先取 `x-ontoforge-event-id` 请求头、回退取 payload；请求体不是 JSON 时 `payload_event_id` 记为 null 且照常记录。接收端只记录、不断言，不暴露聚合端点，也不以退出码表达成败。
- **端到端用例（`crates/ontoforge/tests/webhook_e2e_pg.rs`）**：`#[ignore]` 的跨进程集成测试。PG 提交 Action 产生真实 outbox 行，把接收端与 worker 作为两个独立子进程拉起（`Child` + `impl Drop { kill; wait }`，CI shell 里没有后台进程），轮询 ledger 直到证据齐备，再从 PostgreSQL 侧取第二份独立证据。缺 `ONTOFORGE_PG_URL` 或 `ONTOFORGE_WEBHOOK_RECEIVER_BIN` 时 panic，不静默跳过。
- **`phase8-webhook-e2e` CI job**：起 PG service container、建 `editstore` 库、构建 example、跑被 ignore 的用例（先 `--list` 断言零用例即失败）、上传前跑 `check-artifacts` 凭据扫描、以 `if-no-files-found: warn` 上传 `phase8-webhook-ledger` artifact、`always()` 清理容器。
- **RED / GREEN 两态都留了 CI 证据**（见下）。

## CI 证据

| 状态 | Commit | CI run | `phase8-webhook-e2e` |
|---|---|---|---|
| RED | `f95767a` | https://github.com/easyinplay/ontoforge/actions/runs/42505885675 | failure（行为断言失败） |
| GREEN | `a023760` | https://github.com/easyinplay/ontoforge/actions/runs/42508002310 | success |
| 多事件矩阵 | `ae8956d` | https://github.com/easyinplay/ontoforge/actions/runs/42510584474 | success |

RED 的失败输出（`webhook_e2e_pg.rs:343`，非编译错误、非零用例发现）：

```
thread 'webhook_worker_delivers_to_an_independent_receiver_over_postgres' panicked at crates/ontoforge/tests/webhook_e2e_pg.rs:343:13:
the receiver recorded no requests at all:
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 60.25s
```

GREEN 与多事件矩阵的 ledger 原始记录（来自 `phase8-webhook-ledger` artifact）：

```jsonl
{"seq":1,"received_at":"2026-11-08T18:34:24.445926081+00:00","method":"POST","path":"/events","header_event_id":"01M6Y71C4QXGE07WS1XNPTEZVP","payload_event_id":"01M6Y71C4QXGE07WS1XNPTEZVP","event_id":"01M6Y71C4QXGE07WS1XNPTEZVP","attempt_seq":1,"status_code":503,"body_len":555,"remote_addr":"127.0.0.1:47872"}
{"seq":2,"received_at":"2026-11-08T18:34:24.457306838+00:00",...,"event_id":"01M6Y71C4YHKFZ40BGSEZPPTF7","attempt_seq":1,"status_code":503,...}
{"seq":3,"received_at":"2026-11-08T18:34:24.466510815+00:00",...,"event_id":"01M6Y71C580VBFKHRDR55HEF26","attempt_seq":1,"status_code":503,...}
{"seq":4,"received_at":"2026-11-08T18:34:25.482500028+00:00",...,"event_id":"01M6Y71C4QXGE07WS1XNPTEZVP","attempt_seq":2,"status_code":204,...}
{"seq":5,"received_at":"2026-11-08T18:34:25.484954808+00:00",...,"event_id":"01M6Y71C4YHKFZ40BGSEZPPTF7","attempt_seq":2,"status_code":204,...}
{"seq":6,"received_at":"2026-11-08T18:34:25.486984305+00:00",...,"event_id":"01M6Y71C580VBFKHRDR55HEF26","attempt_seq":2,"status_code":204,...}
```

从原始记录复算：

- 退避间隔（相邻两次 `received_at` 之差）：1.0366 s / 1.0276 s / 1.0205 s，全部落在 `[0.9s, 30s]`，证明走的是 `1 << (attempts-1)` 的退避路径而非忙重试。
- 去重集合：ledger 里的唯一 `event_id` 集合 = 提交的 3 个 `event_id` 集合（3 个，多一条或少一条都失败）。
- 至少一次送达：每个 `event_id` 至少一条 2xx；每条的首条 503、末条 204。
- `header_event_id == payload_event_id == event_id`，且与 `of_webhook_outbox.event_id` 一致。
- PostgreSQL 侧独立证据：`of_webhook_outbox` 恰好三行，全部 `delivered_at` 非空、`attempt_count = 1`、`last_error = 'http_status_503'`。
- 接收端是独立进程（子进程 pid ≠ 测试进程 pid），每条记录的 `remote_addr` 以 `127.0.0.1:` 开头；ledger 每行既不含连接串也不含密码。

本机留证：`target/phase8-webhook/red-ci-run.txt`、`green-ci-run.txt`、`multi-event-ci-run.txt`（`target/` 被 gitignore；归档进 `08-evidence/` 由 plan 08-02 完成）。

## Task Commits

1. **Task 08-01-01（RED）** - `f95767a` (test) — 接收端存根 + 完整断言的端到端用例 + `phase8-webhook-e2e` job
2. **Task 08-01-01（GREEN）** - `a023760` (feat) — 接收端逐请求写 ledger，按 `event_id` 计数首尝试
3. **Task 08-01-02** - `ae8956d` (test) — 多事件去重集合、至少一次送达与跨来源交叉取证

## Files Created/Modified

- `crates/ontoforge/examples/webhook_receiver.rs` — 独立进程接收端（example target）：绑回环端口、写 ready-file、逐请求写 JSONL ledger、同一 `event_id` 首次 503 其后 204
- `crates/ontoforge/tests/webhook_e2e_pg.rs` — 被 `#[ignore]` 的端到端编排与全部断言（PG 提交、两个子进程、轮询 ledger、交叉取证）
- `crates/ontoforge/Cargo.toml` — `[dev-dependencies]` 增加 `axum.workspace = true`（examples 可用；不进发行依赖集）
- `Cargo.lock` — 同步刷新：`[[package]] name = "ontoforge"` 的依赖列表多一条 `"axum",`，零删除行
- `.github/workflows/ci.yml` — 新增独立 job `phase8-webhook-e2e`
- `.planning/WINDOWS.md` — 修复被前一条目阻塞的跨 phase 缺陷登记（见 Deviations），并登记 FA-02

## Decisions Made

- **接收端以 example target 形态存在**，不注册为 `ontoforge` 子命令、不新增 workspace member。`grep -c 'webhook_receiver' crates/ontoforge/src/cli.rs` 与 `.../main.rs` 均为 0，发行打包脚本只收 binary 与 manifest，example 天然不入包（D-01）。
- **`axum` 只进 `[dev-dependencies]`**：examples 可用 dev-dependencies，而放进 `[dependencies]` 会把 axum 带进 `ontoforge` 的发布依赖集，违反 D-01。workspace 已 pin `axum = "=0.8.9"`，依赖图不变，`ci/check-tls-deps.sh` 的 openssl / native-tls 守卫保持绿。
- **进程编排全部收进 Rust 测试**，CI shell 只负责起 PG 容器、跑一次 `cargo test`、上传 artifact、清理容器。没有 `cmd &`、没有 pid 文件、没有 job 挂起风险。
- **本机验证口径是编译级**：Windows 本机无 Docker 且 D-03 要求 PG，端到端用例是 CI-only；本机跑 `cargo build --example` / `cargo clippy --all-targets` / `cargo test --workspace` / `cargo fmt --check`。
- **不在 worker 或 store 层加任何 `event_id` 去重或抑制**：去重责任只在接收端，store 层的幂等键仍是 Action 层的 `command_id`（CF-01 / CF-03 / PR-08）。

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] 修复被前一条目阻塞的 `.planning/WINDOWS.md` 缺陷登记**
- **Found during:** Task 08-01-02（登记 FA-02 未执行的退避上限时）
- **Issue:** 登记表的第 18 条用了词表外的 `kind: flaky-test`，并且它的渲染表格与围栏 JSON 块不一致（`line` 与 `reason` 两格）。`gsd_run windows append` 因此在读取阶段就报 `Ledger entry 18 has invalid kind` / `Ledger table ... disagrees with the fenced JSON entries`，**所有后续 phase 的 stub / 未执行验证记录都会被静默丢弃**——这正是该登记表存在的原因被绕过。
- **Fix:** 把第 18 条的 `kind` 改为词表内的 `unmet-truth`（描述文字保留"判定为时序 flake 而非回归"这一原始判断），并让渲染表格与 JSON 块逐格一致（`line` 取 437，`reason` 补齐）。登记表随即恢复可写，随后成功登记本 phase 的 FA-02 条目（第 19 条）。
- **Files modified:** `.planning/WINDOWS.md`
- **Verification:** `gsd_run windows status` 返回 `ok: true`（open_count 13 → 14，total_count 18 → 19）；`renderTable(JSON)` 与磁盘表格逐字节相同。
- **Committed in:** 随 plan metadata 提交。

**2. [Rule 3 - Blocking] 清理撑满磁盘的 `target/` 构建产物，恢复链接**
- **Found during:** Task 08-01-01（第一次 `cargo test --workspace` 之后）
- **Issue:** D 盘 932 GB 用满（可用 4.7 MB），`cargo test --workspace` 以 `LNK1180: insufficient disk space to complete link` 与 `LNK1318: Unexpected PDB error` 失败。占用来自历史遗留的 `target/debug`、`target/release` 与 `target/x86_64-pc-windows-msvc`（三者都在 gitignore 内，无跟踪文件）。
- **Fix:** 删除这三个构建目录（`target/` 下其余历史证据日志保留），磁盘可用空间恢复到约 787 GB；随后按 `CARGO_INCREMENTAL=0` 重新完整构建并跑通全部本地门。
- **Files modified:** 无（只删 gitignore 的构建产物）
- **Verification:** `cargo test --workspace --locked --offline` 退出码 0，153 条 `test result: ok`，无 `FAILED`。
- **Committed in:** 无（不产生仓库变更）

### TDD Gate Compliance

- **计划级 RED → GREEN 序列齐备**：`test(08-01)`（`f95767a`）先于 `feat(08-01)`（`a023760`），两者都真实存在于分支历史中。
- **RED 是真实的行为断言失败，不是编译错误、不是零用例发现**：接收端存根绑端口、写 ready-file、对所有 POST 答 204 但不写 ledger，CI 的 `phase8-webhook-e2e` job 以 `the receiver recorded no requests at all` 变红（`test result: FAILED. 0 passed; 1 failed`）。RED run 与失败输出留证在 `target/phase8-webhook/red-ci-run.txt`。
- **RED commit 的范围超出纯测试文件（计划内的设计，非绕门）**：RED 提交同时包含 `crates/ontoforge/Cargo.toml` 的 `axum` dev-dependency、刷新后的 `Cargo.lock` 与新增的 `phase8-webhook-e2e` CI job。原因是 RED 态必须有一个**能跑起来并产生行为断言失败**的 CI job，若把 CI job 放到 GREEN 提交里，RED 态就无从产生。这是 plan 08-01-01 第五步明文规定的过程（"先提交可运行的接收端存根……连同上面那份完整断言的测试，让 CI 的 `phase8-webhook-e2e` job 以行为断言失败变红"），不是为通过 RED-commit 路径门而做的拆分。
- **Rust 的 RED-commit 路径门看不见 `crates/*/tests/` 下的测试文件**（GSD 已知限制，`references/tdd.md` §Framework setup 的 "Known gap — Rust"）：`crates/ontoforge/tests/webhook_e2e_pg.rs` 不匹配门禁的路径模式，因此门禁无法从路径上区分 RED 提交与普通源码提交。此处未伪造通过的 RED 证据，真实的两态由上表的两个 CI run 承担。
- **Task 08-01-02 没有自己的 RED 提交**：08-01-02 是对断言矩阵的扩展（三条事件、去重集合相等、跨来源取证、独立进程与无凭据断言），其断言对象的行为已由 08-01-01 的 GREEN 提交实现，不引入新的生产行为；plan 对 08-01-02 的 action 也只规定"在同一个测试文件内扩展，不新增文件"，没有规定实现步骤。因此 08-01-02 是一次 `test(08-01)` 提交，其验证是 CI 上该提交 `headSha` 的绿 run（42510584474）。

## 未验证边界

- **FA-01（backstop）**：接收端对非 JSON 请求体的容忍性。plan 说该条只能由源码审阅确认；本次除源码审阅外，还在本机做了实际观测——向接收端 POST 一个非 JSON 体，它记下 `header_event_id: null`、`payload_event_id: null`、`event_id: null`、`status_code: 204`，进程不 panic，ledger 照常多一行。端到端用例本身只发送 worker 产生的合法 payload。
- **FA-02（backstop）**：退避上限与第 k 次重试时序。按 D-04 本 phase 只验证第 1 次重试（约 1 秒，实测 1.02–1.04 秒）；第 17 次失败起固定 300 秒退避只由 `crates/ontoforge-action/src/outbox.rs` 的 `1_i64 << (attempts - 1).min(16)` 与 `.min(300)` 公式保证，未被实际执行。该条已登记到 `.planning/WINDOWS.md`（第 19 条，`unrun-verify`），供 ship 前人工复核。
- **REL-03（systemd 自动拉起）** 不属本 phase（Phase 9）。

## Issues Encountered

- **磁盘满导致链接失败**（见 Deviations 第 2 条）：首次 `cargo test --workspace` 因 D 盘占满而链接失败。清掉 gitignore 的构建产物后恢复正常，与本次改动无关。
- **axum 的 `Uri` 没有 `method()`**：接收端最初用 `uri.method()` 取 HTTP 方法，编译失败。改用 axum 的 `Method` extractor（`FromRequestParts`），`ledger` 的 `method` 字段因此记录真实请求方法而不是硬编码字符串。

## Known Stubs

无。接收端与端到端用例均为完整实现，扫描无 `TODO` / `FIXME` / `placeholder` / 空实现残留。

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plan 08-02（CI 证据归档三件套 + `WEBHOOK-WORKER.md` 的端到端联调入口）可以直接开工：三个 CI run 的 URL 与 ledger artifact 都已就位，artifact 名 `phase8-webhook-ledger`，artifact id `13629302004`（多事件矩阵那次）。
- `crates/ontoforge/WEBHOOK-WORKER.md` 的「端到端联调」小节由 08-02 追加；本 plan 没有改写它。
- Phase 8 的整体结论位保持 `VERDICT: PENDING`，等人工复核 ledger 与 CI 日志后签字。REL-01 / REL-02 的机器可判定部分已全部跑通，但按 shared-ID 门（08-02 同样声明了这两条需求），两条需求要等 08-02 的 SUMMARY 落盘后才标记 Complete。
- 阻塞项：无。Phase 9（systemd 自动拉起）可以并行推进。

## Self-Check: PASSED

- `crates/ontoforge/examples/webhook_receiver.rs`、`crates/ontoforge/tests/webhook_e2e_pg.rs` 存在
- `f95767a`、`a023760`、`ae8956d` 三个提交均在 HEAD 的祖先链上
- 两个 task 的全部 `<automated>` 门重跑通过：`cargo build -p ontoforge --example webhook_receiver --locked`、`cargo test -p ontoforge --test webhook_e2e_pg --locked -- --ignored --list`（1 条用例）、`cargo fmt --all --check`、`cargo clippy --workspace --all-targets --locked -- -D warnings`、`cargo test --workspace --locked --offline`（153 ok / 0 failed）、四条 `ci.yml` grep 门、两条 `webhook_receiver` 未注册子命令门、`Cargo.lock` 的 `"axum",` 边门与零删除门、`cargo tree -p ontoforge --locked --offline`、PR-07 未改被验证对象门、CI `headSha` 绿 run 门

---
*Phase: 08-production-webhook-end-to-end-delivery*
*Completed: 2026-11-09*
