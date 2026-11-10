# Phase 08 Coverage

## API Coverage

No external API integration: 本 phase 不接入任何第三方 API 或 SDK——HTTP 接收端是本 phase 自写的第一方测试 harness，PostgreSQL 是既有 CI job 已接入的存储后端，本 phase 只复用它的 service container，不新增任何外部服务的调用能力。

判定依据（`api-coverage.cjs` 对本 phase 的 ROADMAP 小节与两份 PLAN 正文扫描结果）：`detected=false`，无 signals。脚本没有命中任何「集成动词 + 外部 API 名词」同句组合；本文件按无集成声明格式给出可复核的理由，而不是凭空造矩阵行。

## Multi-Source Coverage Audit

| SOURCE | ID | Feature/Requirement | Plan | Status | Notes |
|---|---|---|---|---|---|
| GOAL | — | 运维者在 Linux CI 上把 outbox 事件投递给真实 HTTP 接收端，并观察到重试、去重与至少一次送达 | 08-01/08-02 | COVERED | ROADMAP 的 5 条 success criteria 全部落在 08-01-01/08-01-02 的机器可判定断言与 08-02-01 的证据归档 |
| REQ | REL-01 | 把 outbox 事件投递给独立进程监听的真实 HTTP 接收端，CI 日志留存接收端收到事件的证据 | 08-01/08-02 | COVERED | 08-01-01 交付接收端、端到端用例与 CI job；08-02-01 归档 ledger |
| REQ | REL-02 | 503 后按退避重试至成功；同一 event_id 被接收端只计一次；整体至少一次送达 | 08-01/08-02 | COVERED | 08-01-01 覆盖退避与 2xx 转正；08-01-02 覆盖去重集合相等与无静默丢弃 |
| CONTEXT | D-01 | 接收端实现为 `crates/ontoforge/examples/` 下的 Rust example，用 `cargo build --example` 构建，不进发行包、不新增 workspace member、不作为子命令对外暴露 | 08-01-01 | COVERED | gate 断言 `crates/ontoforge/src/cli.rs` 与 `crates/ontoforge/src/main.rs` 均无该符号 |
| CONTEXT | D-02 | 接收端把每一次收到的请求追加写入 JSONL ledger；不自己断言、不以退出码表达成败、不暴露聚合端点 | 08-01-01 | COVERED | 判定全部落在 08-01-02 的测试侧 |
| CONTEXT | D-03 | worker 的 store 用 PostgreSQL（`url_env`），接收端走明文 HTTP，不覆盖 HTTPS | 08-01-01 | COVERED | `StoreConfig` 为 `kind = "postgres"` 加 `url_env`；target 是 `http://127.0.0.1:<port>/events` |
| CONTEXT | D-04 | 提交多条（≥3 条）不同事件验证无静默丢弃；只验证第 1 次重试 | 08-01-01/08-01-02 | COVERED | 08-01-02 提交三条；退避只断言第 1 次的约 1 秒间隔 |
| CONTEXT | D-05 | `.github/workflows/ci.yml` 新增独立 job，在 PR 与 main push 上都跑，不并入现有 job | 08-01-01 | COVERED | workflow 级触发条件已覆盖，job 不加额外 `if:` |
| CONTEXT | CF-01 | 投递是至少一次语义；去重是接收端责任；同一事件重试时 event_id 不变 | 08-01-02 | COVERED | 唯一集合相等 + `header_event_id == payload_event_id == event_id` |
| CONTEXT | CF-02 | `target` 仅从可信配置读取，Action 请求不能修改；接收端无认证 | 08-01-01 | COVERED | target 写在受信 toml；接收端不做任何认证 |
| CONTEXT | CF-03 | store 层不去重，幂等键是 Action 层的 `command_id`；`event_id` 是唯一 ULID | 08-01-01/08-01-02 | COVERED | 三条提交用不同 `command_id` 且不同主键；worker/store 未新增按 event_id 的去重 |
| CONTEXT | CF-04 | 生产部署必须由服务管理器在进程退出后重启 worker | — | 不适用 | 明确属 Phase 9（REL-03），本 phase 不交付 systemd unit，也不代签其结论位 |
| RESEARCH | Standard Stack | 接收端用 axum 实现，作 dev-dependency，不进发行包 | 08-01-01 | COVERED | workspace 已 pin `=0.8.9`，`ontoforge-server` 已用同一 pin，依赖图不变，TLS 守卫保持绿 |
| RESEARCH | Validation Architecture | 端到端用例为 `#[ignore]`，由 CI job 用 `--ignored` 命中，并带零用例发现守卫 | 08-01-01 | COVERED | `awk` 计数为 0 时 `::error::` 并退出 1 |
| RESEARCH | Pitfall 2 | `ci/query-services.sh postgres` 只起容器、不建 `editstore` 库 | 08-01-01 | COVERED | job 内补 `psql ... -c 'CREATE DATABASE editstore'` |
| RESEARCH | Pitfall 5/6/7 | 退避间隔须 ≥0.9s ≤30s；轮询 ledger 而非固定 sleep；检查子进程存活并透出 stderr | 08-01-01/08-01-02 | COVERED | 逐条断言与 `try_wait()` 早退检测 |
| RESEARCH | Pitfall 9/10 | `lease_duration_ms > request_timeout_ms`、`batch_limit ∈ 1..=100`、接收端必须在 Windows 上可编译 | 08-01-01 | COVERED | 取值固定 1000/5000/10；本机 `cargo build --example` 与 `cargo clippy --all-targets` 是硬 gate |
| RESEARCH | Open Question 1/3/4/5 | 证据归档纳入；axum 而非手写 HTTP；3 条事件 + `batch_limit = 10`；不为接收端去重逻辑补单元测试；不引入 `postgresql_embedded` | 08-01/08-02 | COVERED | 归档在 08-02-01；单元测试不做，去重正确性由唯一集合相等这一条覆盖 |
| RESEARCH | Security Domain | 上传前扫描证据目录确认不含 job 凭据 | 08-01-01/08-02-01 | COVERED | CI 侧 `check-artifacts target/phase8-webhook`；归档侧再复扫连接串形态 |

无 MISSING 项。明确排除（不算缺口）：CONTEXT 的 Deferred Ideas（接收端 HTTPS/TLS、接收端产品化）；REL-03（Phase 9）、REL-04（Phase 10）、REL-05/06（Phase 11）、REL-07（Phase 12）；RESEARCH Open Question 4 与 5 的「不做」决定。

## Dependencies 与规划证据

| Plan | Needs | Creates | Wave | Checkpoint |
|---|---|---|---|---|
| 08-01 | 既有的 outbox 投递链路、worker CLI、store 与 config 校验、CI service container 脚本 | 接收端 harness、端到端用例、`phase8-webhook-e2e` job | 1 | 无 |
| 08-02 | 08-01 的绿色 CI run 与 `phase8-webhook-ledger` artifact | `08-evidence/` 三件套、worker 运维文档新增小节 | 2 | 无 |

两个 plan 串行：08-02 的归档材料只能来自 08-01 跑出的 CI run，08-01 未落地时 08-02 无输入。校准查询 `estimate-calibration` 返回 factor=1、sample_count=0、confidence=low，估计分别为 70000 与 24000 tokens（`workflow.smart_zone_tokens` 为 100000）。`assumption-delta scan` 返回 `detected=false`，本 phase 未改变任何核心身份模型，未触发身份模型决策。`api-coverage` 检测返回 `detected=false`，本文件按无集成声明格式给出理由。

未检测到 ORM schema 变更：本 phase 不改任何 schema 文件，PostgreSQL 的 `of_webhook_outbox` 表结构由既有 migration 建立且本 phase 不改动。未新增 npm / pip / cargo package install 任务：`axum` 已是 workspace pin 的既有依赖，`cargo` 一律带 `--locked`。

当前树路径已由 `git ls-files` 核实（`crates/ontoforge/Cargo.toml`、`.github/workflows/ci.yml`、`ci/query-services.sh`、`crates/ontoforge/tests/cli_webhook_worker.rs`、`crates/ontoforge-action/tests/outbox_pg.rs` 均为 tracked 源）；新增的 `crates/ontoforge/examples/webhook_receiver.rs`、`crates/ontoforge/tests/webhook_e2e_pg.rs` 与 `08-evidence/` 当前不存在，不能用文件缺失充当 RED 证据。grep 型 gate 的基线已在 plan 里逐条标注（at HEAD 均为 0）。
