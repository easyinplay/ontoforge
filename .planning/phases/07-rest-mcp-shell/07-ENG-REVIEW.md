# Phase 7 Engineering Review

Date: 2026-10-28
Skill: plan-eng-review
Status: IN_PROGRESS

本文件是工程审查工作记录，不是已通过报告或可执行 PLAN。产品决策见 `07-CONTEXT.md`，需求和六项验收以 ROADMAP/REQUIREMENTS 为准。

## Step 0: Scope Challenge

### 现有能力

| 子问题 | 已有实现 | Phase 7 增量 |
|--------|----------|---------------|
| 数据与持久化 | EditStore、SQLite/PG、迁移、历史、read audit | 服务启动与关闭、dev 路径和 reset |
| 权限与写入 | Authenticator、compiled scope、QueryService、ActionService、L2Service | 同源请求上下文、REST/MCP 适配 |
| L2 健康 | monitor、60 秒 TTL、禁用原因、真实 Trino 合同 | 持有生命周期、HTTP 状态、故障恢复 E2E |
| 外部接口 | API/server/client 三个 crate 仅占位 | DTO、axum/OpenAPI、rmcp、Rust/TS client |
| 发行 | Windows/Linux CI 和真实服务 integration | 三平台 release 构建与产物 |

### 最小范围与复杂度

完整 Phase 7 必然超过 8 文件：仅三个 crate 的 manifest 与 lib 就是 6 个文件，还需 workspace manifest/lock、CLI、TS client 与 release workflow。复杂度阈值触发，但不是增加独立服务的理由。沿用同一个 `serve` runtime，stdio 桥接该服务，复用底层逻辑。

用户已批准完整 Phase 7 范围；遵守既有授权，不再次请求批准同一范围，不擅自延后平台层 clients 或 release。建议按 DTO/共享宿主、REST、MCP、clients/dev、生命周期/发行拆成有依赖的可执行 PLAN；这些只是工作分组，尚非最终计划。

### 标准能力核查

- [Layer 1] Context7 本次从 axum 官方仓库文档查到 `serve(...).with_graceful_shutdown(...)`、middleware 与 request extensions；不自建 HTTP dispatcher。长连接和审计排空仍须单独验证。
- [Layer 1] Context7 本次从 rmcp docs.rs 文档查到 `ServerHandler::list_tools`、`StreamableHttpService`、HTTP Parts 的 extensions 和 SessionManager。动态列表、每次调用授权、身份与 session 绑定仍须 spike，文档查询不能冒充编译或安全测试。
- 已锁定的 utoipa/OpenAPI 与 TS 方案来自 ADR；具体版本和 OpenAPI schema 支持留给研究/依赖解析，不用检索示例推断项目已兼容。

### TODO 与完整性

仓库根无 `TODOS.md`。既有 Phase 5 发布前 webhook/审查事项与 Phase 1-5 fingerprint 复核仍按 STATE 的指针处理；本阶段不重新打开已完成阶段。

六项 SC 全保留，尤其双 client 每端点组、stdio 与 HTTP 授权一致性、6 个 L2 工具、Windows dev 持久化、真实健康故障/恢复与三平台发行。

### Distribution

现有 `.github/workflows/` 只有 `ci.yml` 和 `nightly.yml`，尚无 release workflow。PLAN 必须包含 Linux/macOS/Windows 构建、目标矩阵、打包与 checksum、安装/使用说明。编写或验证发布流程不代表已获授权创建 release、发布包或合并 PR #7。

## 1. Architecture Review

Status: IN_PROGRESS，D1 已发送，尚未收到答复。

### D1: L2 部分故障的健康 HTTP 语义

依据：ADR-0028 要求 HTTP 告警但未决定状态码；`crates/ontoforge-engine/src/trino/l2_health.rs:41` 的 `L2HealthStatus` 区分 Enabled/Disabled，`status()` 还会在 TTL 过期时即时禁用。

| 方案 | 行为 | 取舍 |
|------|------|------|
| 1，推荐，待答复 | 仅 L2 故障而核心健康时 `/health` 200，正文 degraded/脱敏原因；核心故障时 503 | 保留核心流量，告警必须检查正文 |
| 2，待答复 | 任一已配置能力故障时 `/health` 503，核心 readiness 单列 | 状态码告警直接，但部署探针须区分整体健康与核心可用 |

两者均要求 L2 请求 fail-closed；未配置的可选 L2 不能被当成意外故障。Grok session 91385 的部分第二意见倾向分开进程存活与能力健康；其超时不算完成独立审查。

### 待评估的接线风险

- 跨进程 reload 与缓存发布：`current_action_definition` 读数据库，服务现有 `active_definition` 是内存快照。须明确请求捕获与刷新合同，并覆盖外部 reload 后新请求、进行中请求和撤权。
- Action/history 一致性：`ActionService::execute_inner` 重新读取当前定义，`EditStore::history` 读取缓存。先检查既有 stale-definition 防护，不能直接判为漏洞，也不能宣称共享上下文已覆盖全部分支。
- MCP session 复用、身份变更、直接调用未列出的工具均需有拒绝测试；只过滤 `list_tools` 不足以授权。
- 正常关闭、启动失败和超时需要显式清理 monitor/audit。Grok 建议的 MCP request id 幂等方案不直接采纳，必须核对 Action command_id 合同。

## 2. Code Quality Review

Status: PENDING，须在 Architecture 的待决问题处理后进入。

## 3. Test Review

Status: PENDING。Rust 现有 cargo test/insta/proptest；CI 含 Windows/Linux、SQLite 和 Linux 真实 PG/Trino。后续须绘制请求/关闭分支与验收图并映射测试，不能把已有库级测试算作服务级证据。

## 4. Performance Review

Status: PENDING。

## Sources

源码与 ADR 的相对路径见上文及 CONTEXT。Context7 本次返回的 primary sources：

- https://github.com/tokio-rs/axum/blob/main/axum/src/serve/mod.rs
- https://github.com/tokio-rs/axum/blob/main/axum/src/docs/middleware.md
- https://docs.rs/rmcp/latest/rmcp/handler/server/trait.ServerHandler.html
- https://docs.rs/rmcp/latest/rmcp/transport/streamable_http_server/tower/struct.StreamableHttpService.html
- https://docs.rs/rmcp/latest/rmcp/transport/streamable_http_server/session/trait.SessionManager.html

`latest` 文档只能证明检索时的接口说明；版本选择、锁定和运行结果仍待研究及 spike。
