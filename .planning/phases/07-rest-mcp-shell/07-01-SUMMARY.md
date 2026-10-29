---
phase: 07-rest-mcp-shell
plan: "01"
subsystem: api
tags: [rust, axum, http, auth, sqlite, audit, tdd]
requires:
  - phase: 02-source-binding-engine
    provides: 查询引擎和可信服务配置
  - phase: 03-edit-store-scope-model
    provides: definition、scope 与共享定义事务锁
  - phase: 04-scoped-l0-read
    provides: QueryService、逻辑对象读取与版本 token
  - phase: 06-l2-raw-dql-toolsets-audit
    provides: ReadAuditGuard、持久读取审计和显式 drain
provides:
  - 认证 HTTP pk 读取与 durable audit request_id 关联
  - 请求开始时固定 definition、roles、bindings 的短事务 snapshot
  - serve 入口、真实 listener、可注入关闭与持久 query signing key
affects: [07-02, 07-03, 07-06, 07-09, 07-12, 07-13, 07-14]
actuals:
  tokens: 79569
  tasks: 2
  commits: 8
commits: 8
plan_head_before: bdd9ad11915c6b73a108a0b72186772bba1bc7fb
plan_head_after: 1835945af3fd4087ba3be36d63d5af9de2b06801
tech-stack:
  added: [axum, rmcp, utoipa, utoipa-axum, tokio-util]
  patterns:
    - 认证先于 snapshot，事务结束后编译 scope 和访问 source
    - execute_audited 复制现有 guard ID，execute 保持兼容 wrapper
    - cfg(test) 加载 tests/unit 中的 lib 测试模块
key-files:
  created:
    - crates/ontoforge-server/tests/unit/tracer.test.rs
    - crates/ontoforge/tests/unit/serve.test.rs
    - crates/ontoforge/src/serve.rs
    - docs/api.md
  modified:
    - Cargo.toml
    - Cargo.lock
    - crates/ontoforge-server/Cargo.toml
    - crates/ontoforge-server/src/lib.rs
    - crates/ontoforge-storage/src/lib.rs
    - crates/ontoforge-query/src/lib.rs
    - crates/ontoforge-query/tests/audit_pk.rs
    - crates/ontoforge/Cargo.toml
    - crates/ontoforge/src/cli.rs
    - crates/ontoforge/src/lib.rs
    - crates/ontoforge-engine/src/config.rs
    - crates/ontoforge-engine/tests/server_config.rs
    - examples/server/ontoforge-server.dev.toml
key-decisions:
  - 现有审计 guard ID 通过兼容 QueryExecution 返回，客户端不能指定 audit ID
  - serve 强制读取独立持久 query signing key，旧命令配置保持兼容
  - 查询 key 沿用原 token wire，不隐式生成或轮换；dev 初始化归 07-12
patterns-established:
  - RequestSnapshot 复用 begin_definition_tx，锁内读 definition/scope，显式结束事务后返回
  - 认证和早期错误使用服务端 transport ID，不宣称存在 durable audit
requirements-completed: [API-01, API-03, SCOPE-07]
coverage:
  - id: D1
    description: 真实认证 HTTP 读取返回与持久审计一致的 request_id
    requirement: API-01
    verification:
      - kind: e2e
        ref: crates/ontoforge-server/tests/unit/tracer.test.rs#tracer_authenticated_pk_is_durable_after_shutdown
        status: pass
      - kind: e2e
        ref: crates/ontoforge-server/tests/unit/tracer.test.rs#tracer_unauthenticated_and_forged_context_are_denied
        status: pass
    human_judgment: false
  - id: D2
    description: 固定请求 snapshot，source Pending 不持有 metadata 锁
    requirement: SCOPE-07
    verification:
      - kind: e2e
        ref: crates/ontoforge-server/tests/unit/tracer.test.rs#tracer_snapshot_releases_lock_before_source
        status: pass
    human_judgment: false
  - id: D3
    description: serve 使用真实认证宿主、显式 drain 和独立持久查询 key
    verification:
      - kind: e2e
        ref: cargo test -p ontoforge --lib --locked serve_ -- --nocapture
        status: pass
      - kind: integration
        ref: cargo test -p ontoforge-engine --test server_config --locked -- --nocapture
        status: pass
    human_judgment: false
duration: 69.62min
completed: 2026-10-29
status: complete
---

# Phase 7 Plan 1: 认证 HTTP 读取与 serve 入口 Summary

**真实 TCP 认证读取贯通短事务 snapshot、QueryService 与持久审计，serve 使用独立持久查询签名密钥并在关闭时显式 drain。**

## Performance

- 证据窗口：`2026-10-28T23:41:04.8411350Z` 至 `2026-10-29T00:50:41.8219618Z`，69.62 分钟，包含合同检查点等待；不推断更早启动时间。
- 完成 2/2 tasks。统计 93 个 executor 所有文件，包括实现、测试、配置、API 文档、执行记录和原始 evidence，不包括本 SUMMARY 与主会话共享文档。
- `actuals.tokens` = `ceil(318275 / 4)`，按 ledger base 到最终工作内容的 owned realized diff 字符数计算，包含 evidence；不是 harness token 消耗。
- `commits` 由 `git rev-list --count bdd9ad1..1835945` 实测为 8，包含主会话两个批准合同的 docs commits，不包括本 SUMMARY 所在的后续 docs commit。

## Accomplishments

- `request_snapshot` 复用定义事务锁，一次捕获 active definition 与 scope，显式结束事务后才编译 scope、执行 source。Pending source 期间第二 snapshot 与 reload 均能完成。
- `execute_audited` 返回现有 guard 的 ID 和结果；原 `execute` 签名保持兼容。成功、拒绝和 finish 错误关联同 ID，reserve 失败没有 audit ID，取消仍沿用 guard Drop。
- `GET /api/objects/{object_type}/{pk}` 先认证，拒绝客户端可信上下文注入，只返回逻辑对象行；关闭重开文件 SQLite 后审计仍可读取并匹配响应 ID。
- CLI `serve` 接通真实 listener、配置、认证和引擎；可注入 shutdown，生产使用 Ctrl+C。监听失败与 audit drain 失败返回非零。
- `[query] signing_key_file` 相对配置解析，读取 32 bytes 至 64 KiB 的原始字节，监听前 fail closed。key 独立于 auth，正常重启复用，不打印、不创建、不轮换，不改变既有 token wire。

## Task Commits

| Task/节点 | Commit | 内容 |
|---|---|---|
| 01-1 RED | `7897e39` | 真实认证读取与审计关联行为测试 |
| 01-1 GREEN | `5df9f2a` | snapshot、QueryExecution 和认证 HTTP 接线 |
| 01-2 RED | `1df4a16` | serve 参数、认证 listener 与关闭行为 |
| 合同修订 | `74db66c` | 主会话锁定持久 query signing key 合同 |
| 01-2 追加 RED | `8aa4d46` | 查询 key 配置、独立性与重启行为 |
| 01-2 GREEN | `2e1a698` | serve 与持久 query signing key |
| 布局修订 | `137ffd1` | 主会话批准测试机械移动的范围 |
| 最终修复 | `1835945` | 测试布局、局部 lint 与全部本地 gate evidence |

## TDD Gate Compliance

所有 RED 都先真实编译，再运行目标断言；原生 cargo-nextest JUnit 经 `check tdd-red-evidence` 得到 `RED_EVIDENCE_OK`。未用编译失败、零用例、陈旧或手工修改的报告授权 GREEN。

| 能力 | 真实 RED | 最终 GREEN | 原始 evidence 前缀 |
|---|---|---|---|
| HTTP tracer | 3 executed / 3 failed | 3 passed / 0 ignored | `task1-cargo-red`、`task1-nextest-red`、`final-tracer-relocated` |
| Query audit ID | 3 executed / 1 passed / 2 failed | 12 passed / 0 ignored | `task1-query-cargo-red`、`task1-query-nextest-red`、`final-query-audit` |
| serve 入口 | 2 executed / 2 failed | 合并后 4 passed / 0 ignored | `task2-cargo-red`、`task2-nextest-red`、`final-serve-relocated` |
| key 跨启动 | 2 executed / 1 passed / 1 failed | 包含于上述 4 passed | `task2-query-key-cargo-red`、`task2-query-key-nextest-red` |
| query 配置 | 1 executed / 1 failed | server_config 35 passed | `task2-query-config-cargo-red`、`task2-query-config-nextest-red`、`task2-query-config-green` |

RED 中已通过的负向 key 和 reserve 失败断言没有被宣称为 RED。01-1 GREEN 提交后额外重跑 tracer feedback gate，3 passed；最后机械移动与 lint 调整后又实际执行两个指定 `--lib` 命令。

## Verification

全部日志与 metadata 位于 `07-01-evidence/`，metadata 包含真实退出码、UTC 起止与绝对日志路径。

| 命令/检查 | 最终结果 | Evidence |
|---|---|---|
| `cargo test -p ontoforge-server --lib --locked tracer_ -- --nocapture` | exit 0；3 passed | `final-tracer-relocated` |
| `cargo test -p ontoforge --lib --locked serve_ -- --nocapture` | exit 0；4 passed | `final-serve-relocated` |
| `cargo test -p ontoforge-query --test audit_pk --locked -- --nocapture` | exit 0；12 passed | `final-query-audit` |
| `cargo test -p ontoforge-engine --test server_config --locked -- --nocapture` | exit 0；35 passed | `task2-query-config-green` |
| `cargo test --workspace --locked -- --test-threads=1` | exit 0；117 suites，1266 passed，0 failed，106 ignored | `final-workspace-tests-rerun` |
| `cargo fmt --all --check` | exit 0 | `final-fmt-relocated` |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 | `final-clippy-pass` |
| `cargo clippy -p ontoforge-storage --features embedded-pg --all-targets --locked -- -D warnings` | exit 0 | `final-clippy-embedded` |
| `cargo check -p ontoforge-action --locked` | exit 0 | `final-action-check` |
| production mutation caller guard | 1 passed；workspace 中同组 4 passed | `final-production-guard`、`final-workspace-tests-rerun` |
| TLS dependency tree 否定断言 | 两命令均 exit 101；目标 package 不存在，符合 CI gate | `final-tls-openssl`、`final-tls-native` |

TLS 命令分别为 `cargo tree --workspace --locked --features ontoforge-storage/embedded-pg -e normal,build,dev -i openssl` 与相同参数的 `-i native-tls`。两个原始日志均明确报告指定 package ID 不存在；此特定结果满足否定断言，不把任意 tree 失败算作通过。

workspace 最终重跑为 `2026-10-29T00:40:47.5273483Z` 至 `2026-10-29T00:47:51.3490337Z`。1266 passed 包含 2 个 compile-fail doc-tests；106 ignored 包含 1 个既有 doc-test。PostgreSQL、独立 Trino 与私有 fixture 的既有 ignored 未执行，不计为通过；feature-gated 的 0-test target 也不计为覆盖。

## Deviations from Plan

主会话先批准 `execute_audited` 窄接口及可信持久 key 配置，再修改 PLAN 与所有权后恢复执行。两次合同检查点保存在 `07-01-execution.md`，没有自行选择临时生产密钥或扩大接口。

### Auto-fixed Issues

1. **[Rule 3 - Blocking] 独立测试 fixture 被生产写入口 guard 误识别。** 首次 workspace 在 Action guard 失败，原始 `final-workspace-tests` 保留。经 `137ffd1` 批准，将两个 `src/*.test.rs` 机械移动到 `tests/unit/`，cfg(test)/path 继续加载为原 lib tests。两个 blob 完全不变；`entrypoints.rs` 和 production allow-list 未修改。`1835945` 中均为 100% rename，没有意外删除。
2. **[Rule 1 - Bug] 新 serve 装配触发 workspace clippy。** 使用 `BTreeMap::new()` 和 if-let/let-else 修正局部表达式；未添加 lint allow。失败的 `final-clippy` 与 `final-clippy-green` 均保留且真实 exit 101，最终 `final-clippy-pass` exit 0，提交 `1835945`。

## Issues Encountered

- ralph-loop、Warp、Context7 MCP 和 ctx7 CLI 不可用；未安装替代工具，也未声称运行。实际使用 PowerShell/Cargo；planning-with-files 与 superpowers TDD 的执行过程见 `07-01-execution.md`。
- 没有发生认证操作 gate。开发 token 与查询 key 均由测试临时可信 fixture 提供；示例/API 文档只包含路径和生成命令，没有真实 key。
- stub 扫描没有发现本 slice 目标未接线、TODO/FIXME 或假数据出口。新增 endpoint、配置文件读取和审计路径均在批准 threat model 与 key 合同中，没有额外未登记的信任边界。

## Next Phase Readiness

- 后续 REST/MCP 可以复用宿主与 snapshot；list/aggregate 必须独立核查审计关联入口，不能把 pk 覆盖推广到全部读取。
- 完整进程退出矩阵归 `07-06/09/12/13`；dev seed/reset 与独立 key 一次初始化归 `07-12`，远程真实服务与发布证据归 `07-13/14`。这些没有被标记为当前通过，也不构成本 plan 未运行的 automated 命令。
- `requirements-completed` 按 PLAN 原样列出本 slice 的关联 ID，不表示整个 Phase 7 的 REST、MCP、client 或发布需求已经完成。共享 STATE、ROADMAP、REQUIREMENTS、task_plan、progress、findings 由主会话依据本 SUMMARY 更新。
- 本 executor 没有执行 merge、Release、npm publish 或变更 PR #7 的 Draft/OPEN 状态。

## Self-Check: PASSED

- 写入前检查 17 个实现/测试/配置/API 文档路径、执行记录和 evidence 均存在。
- `7897e39`、`5df9f2a`、`1df4a16`、`8aa4d46`、`2e1a698`、`1835945` 均经 `git merge-base --is-ancestor` 验证为 HEAD 祖先。
- 提交 guard 校验固定 root、非 detached/非 protected 分支与 `SEQUENTIAL=true`；ledger 实测 8 commits。
- 最终所有必要本地 gate 已结束，当前无活跃 Cargo session；未触碰用户无关 untracked 文件。
