# Phase 8: Production Webhook End-to-End Delivery - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-11-08
**Phase:** 8-Production Webhook End-to-End Delivery
**Areas discussed:** 接收端 harness 的形态与归属, 证据通道, 生产保真度, 验证规模与 CI 编排

---

## 侦察前置结论（讨论开始前的事实核对）

讨论前对代码库做了核对，避免把已知的东西当灰区来问：

- `crates/ontoforge/tests/cli_webhook_worker.rs` 已经跑真实 CLI 二进制 + 真实 HTTP + 503 重试 + 重启恢复三条用例，且 CI 的 `test` job 矩阵含 `ubuntu-latest`、跑 `cargo test --workspace --locked -- --test-threads=1`、该测试无 `#[ignore]` → **它今天已经在 GitHub Actions 的 Linux runner 上跑**。
- 因此本 phase 的真实缺口是三处：接收端是**进程内**的 `wiremock::MockServer`（不满足「独立进程」）；接收端数的是**原始请求数**而非按 `event_id` 去重；worker store 是 SQLite（PG 只在库级 `outbox_pg.rs` 覆盖）。
- 退避算法、`event_id` 双重标识（头 + payload）、`[webhook]` 五个配置键、接收端无认证 —— 全部已存在，**不需要新写**。
- 仓库内目前**没有任何 systemd 相关内容**，systemd unit 属 Phase 9。

---

## 接收端 harness 的形态与归属

| Option | Description | Selected |
|--------|-------------|----------|
| Rust 独立 binary（推荐） | 新增极小的 Rust binary，只做按 event_id 去重计数 + 暴露计数。与项目同构、无额外运行时依赖、cargo 直接构建；代价：增加 workspace 面积，且它自己也要被测。 | ✓ |
| Python 独立进程 | 用 python3 写几十行 HTTP 接收端，CI 里直接跑。零编译、迭代快、CI 已有 python3；代价：Rust 项目的 CI 引入 Python 依赖，接收端逻辑不受 cargo test 保护。 | |
| 容器化服务 | CI 里 docker 起接收端容器。最接近真实部署形态；代价：重、启动慢、容器内不易实现 event_id 去重计数。 | |
| 复用 wiremock 拆进程 | 把 wiremock server 拆到另一个进程。复用现有 mock 语义；代价：仍不是真实业务接收端，dedup 语义靠 mock 拼装。 | |

**User's choice:** Rust 独立 binary（推荐）
**Notes:** 用户进一步拍板归属为 `examples/`，不进发行包。

### 接收端 binary 的位置与发行包归属

| Option | Description | Selected |
|--------|-------------|----------|
| examples/，不进发行包（推荐） | `crates/ontoforge/examples/` 下，`cargo build --example` 构建，不进发行包、不新增 workspace member。理由：它是测试 harness，不是产品能力；Phase 12 首次发布时不应该多出一个消费者能调用的面。 | ✓ |
| 新建独立 crate | `crates/ontoforge-webhook-receiver`。职责独立、可单独测试；代价：workspace 成员 +1，CI 构建面变大。 | |
| 做成 ontoforge 子命令 | `ontoforge webhook-receiver`。与 worker 同二进制；代价：测试工具进了发行包，Phase 12 首次发布就多一个对外 subcommand。 | |

**User's choice:** examples/，不进发行包（推荐）
**Notes:** 与 D-01 合并为一条决策。

---

## 证据通道

| Option | Description | Selected |
|--------|-------------|----------|
| 写 ledger 文件（推荐） | 接收端把收到的 event_id 列表写到文件（如 /tmp/received.jsonl），CI 用 jq 断言。证据是原始记录，可人工复核，不依赖接收端自己会报错；代价：CI 多一步文件传递。 | ✓ |
| 暴露 /counts 端点 | 接收端额外开 GET /counts 返回 JSON（总请求数 / 唯一 event_id 数 / 每个 id 的尝试次数），CI curl 取后断言。CI 侧断言直观；代价：证据是接收端自报的聚合值，不是原始记录。 | |
| 接收端自断言退出码 | 接收端自己读参数做断言，不满足就非零退出，CI 只看退出码。CI 最简单；代价：失败时只知道「不满足」，不知道具体差在哪，与项目「结论位固定 VERDICT: PENDING、证据要能人工复核」的习惯相左。 | |

**User's choice:** 写 ledger 文件（推荐）
**Notes:** 补充约束：ledger 保留每次收到的请求的原始记录，不只是聚合计数。

---

## 生产保真度

| Option | Description | Selected |
|--------|-------------|----------|
| Postgres + HTTP（推荐） | worker 走 PG（CI 已有 PG service container，与生产一致），接收端走 http://localhost。store 路径完全真实，且 TLS 证书信任不引入新变量；代价：需要 PG service container，CI job 重一些。 | ✓ |
| SQLite + HTTP | worker 与接收端都走 SQLite + http://localhost。轻、无 service container、跑得快；代价：worker 的 store 路径与生产不同，deferred item 里的「持久数据库」只满足一半。 | |
| Postgres + HTTPS | worker 走 PG，接收端走 https。reqwest 当前没有自定义 CA 的配置钩子，这会变成要新增能力（可能超出 phase 边界）。 | |
| SQLite 与 Postgres 都跑 | 先 SQLite + HTTP 跑通端到端语义，再加一条 PG 变体。反馈快，两层都覆盖；代价：CI job 多一个。 | |

**User's choice:** Postgres + HTTP（推荐）
**Notes:** 用户在讨论前已指出「github 上的 ci 不是有 Linux 吗」，据此确认生产 webhook 联调的执行路径是 GitHub Actions Linux runner。

---

## 验证规模与 CI 编排

| Option | Description | Selected |
|--------|-------------|----------|
| 多事件 + 1 次重试（推荐） | 提交 3-5 条不同事件（batch_limit 默认 50 足够），验证「无静默丢弃」；只验证第 1 次重试（退避 1s）。总耗时几秒级，CI job 可控。 | ✓ |
| 多事件 + 2 次重试 | 提交 3-5 条事件，验证连续 2 次重试（退避 1s + 2s = 3s 等待）。能证明退避在递增，但 CI 时长变长。 | |
| 单事件 + 1 次重试 | 只提交 1 条事件。代价：无法覆盖 success criterion 4（无事件被静默丢弃）。 | |
| 多事件 + 4 次重试 | 验证到第 4 次重试（1+2+4+8=15s 等待）。证据最厚；代价：CI job 明显变长，退避上限 300s 的完整路径仍覆盖不到。 | |

**User's choice:** 多事件 + 1 次重试（推荐）
**Notes:** 具体条数留给 executor 决定。

### CI job 的排法

| Option | Description | Selected |
|--------|-------------|----------|
| 新增专用 job（推荐） | 在 ci.yml 新增一个专用 job（如 phase8-webhook-e2e），起 PG service container + 接收端进程 + worker 二进制，只跑这一条端到端用例。与现有 job 解耦，不拖慢其他 job。 | ✓ |
| 并入现有 webhook-pg job | 它已经在跑 PG + outbox_pg。不新增 job；代价：该 job 职责会从「库级 PG outbox 测试」变成「库级 + 二进制级端到端」，职责变杂。 | |
| 新增 job 但只跑 main | 避开 PR 的 CI 时间成本。代价：PR 上看不到这条回归保护，问题要到合并后才暴露。 | |

**User's choice:** 新增专用 job（推荐）
**Notes:** 在 PR 与 main push 上都跑。

---

## Claude's Discretion

以下由 Claude / planner / executor 自主决定，用户明示可自行决定（详见 CONTEXT.md 的 `### Claude's Discretion`）：

- ledger 的具体字段结构与落盘路径
- 接收端进程在 CI job 里的启停方式与清理保证
- 复用 `ci/query-services.sh postgres` 起 PG service container 的具体接法
- 事件条数的具体取值（≥3）与 `batch_limit` 取值
- 是否为接收端的 `event_id` 去重逻辑补 `cargo test` 单元测试
- CI job 的超时预算与 job 名
- 证据文件是否归档进 `08-evidence/`

## Deferred Ideas

- **接收端的 HTTPS / TLS 支持** —— `reqwest` 需新增自定义受信 CA 的配置钩子，属新增产品能力，超出 Phase 8 边界
- **接收端产品化** —— 做成 `ontoforge webhook-receiver` 子命令或独立 crate 供用户自测；Phase 12 首次发布刻意不包含
