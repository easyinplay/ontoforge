# Webhook worker

Action 提交将事件与 edit log 原子写入 outbox。投递由独立的常驻进程执行；生产部署必须用服务管理器在进程退出后重启它。

在可信的 `ontoforge-server.toml` 中增加：

```toml
[store]
kind = "postgres"
url_env = "ONTOFORGE_PG_URL"

[webhook]
target = "https://receiver.example.com/events"
poll_interval_ms = 1000
request_timeout_ms = 5000
lease_duration_ms = 15000
batch_limit = 50
```

运行 `ontoforge webhook-worker --server-config ontoforge-server.toml`。进程使用同一 edit store，按配置间隔轮询；数据库轮询失败后继续尝试，Ctrl+C 退出。网络或非 2xx 响应会按 outbox 的退避时间重试。进程崩溃后重新启动会拾取未确认或 lease 过期的事件。

投递是至少一次语义。接收端须按 `X-Ontoforge-Event-Id` 或 JSON payload 的 `event_id` 去重；同一事件重试时该 ID 不变。`target` 仅从可信配置读取，Action 请求不能修改它。Phase 7 的 REST/MCP 宿主上线时仍须保证一个 worker 运行，或在宿主内驱动同一个 outbox worker。

## 端到端联调（CI）

这条联调跑在 `.github/workflows/ci.yml` 的 `phase8-webhook-e2e` job 上，PR 与 main push 都触发。该 job 起一个 PostgreSQL service container、建 `editstore` 库，先构建接收端，再跑那条被 `#[ignore]` 的端到端用例：

```bash
cargo build -p ontoforge --example webhook_receiver --locked
cargo test -p ontoforge --test webhook_e2e_pg --locked -- --ignored --nocapture
```

用例是 `#[ignore]` 的，只有在这个 job 里被 `--ignored` 命中才执行；缺 `ONTOFORGE_PG_URL` 或 `ONTOFORGE_WEBHOOK_RECEIVER_BIN` 时它直接失败，而不是静默跳过。

需要的环境变量：

| 变量 | 含义 |
|---|---|
| `ONTOFORGE_PG_URL` | PostgreSQL 连接串，由该 job 的 PostgreSQL 脚本生成 |
| `ONTOFORGE_WEBHOOK_RECEIVER_BIN` | 指向已构建的接收端二进制 |
| `ONTOFORGE_WEBHOOK_EVIDENCE_DIR` | 可选：ledger 与两个子进程日志的落盘目录，默认 `target/phase8-webhook` |

证据落盘：ledger 是 JSONL，每收到一次请求追加一行，接收端只记录原始事实、不做断言。CI 在上传前用 `bash ci/query-services.sh check-artifacts target/phase8-webhook` 扫描证据目录里的凭据，随后以 artifact 名 `phase8-webhook-ledger` 上传该目录。

接收端 harness 的位置与边界：代码在 `crates/ontoforge/examples/webhook_receiver.rs`，以 example target 形式构建。它是 CI 内部的测试 harness，**不是** `ontoforge` 的子命令，不进发行包，不作为对外能力——用户不需要也不能依赖它。

投递语义与去重责任见上文：投递是至少一次语义，去重是接收端的责任。
