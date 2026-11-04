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
