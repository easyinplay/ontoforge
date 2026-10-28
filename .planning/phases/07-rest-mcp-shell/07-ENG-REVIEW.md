# Phase 7 Engineering Review

Date: 2026-10-28
Skill: plan-eng-review
Status: DONE_WITH_CONCERNS (设计审查完成，兼容性 spike 和执行验收仍待验证)

本文件审查 Phase 7 的设计与规划约束，不代表实现、测试或发行已完成。产品决策见 `07-CONTEXT.md`，需求和六项验收以 ROADMAP/REQUIREMENTS 为准。

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

Status: REVIEWED，D1 已接受；其余风险按下述既有合同约束接线，无新增产品范围或待决选项。

### D1: L2 部分故障的健康 HTTP 语义

依据：ADR-0028 要求 HTTP 告警但未决定状态码；`crates/ontoforge-engine/src/trino/l2_health.rs:41` 的 `L2HealthStatus` 区分 Enabled/Disabled，`status()` 还会在 TTL 过期时即时禁用。

| 方案 | 行为 | 取舍 |
|------|------|------|
| 1，已选择 | 仅 L2 故障而核心健康时 `/health` 200，正文 degraded/脱敏原因；核心故障时 503 | 保留核心流量，告警必须检查正文 |
| 2，未选择 | 任一已配置能力故障时 `/health` 503，核心 readiness 单列 | 状态码告警直接，但部署探针须区分整体健康与核心可用 |

两者均要求 L2 请求 fail-closed；未配置的可选 L2 不能被当成意外故障。Grok session 91385 的部分第二意见倾向分开进程存活与能力健康；其超时不算完成独立审查。

2026-10-28 用户答复 `1`，接受方案 1。规范性决策保存于 `07-CONTEXT.md` D-03；本节保留备选方案审计，不再询问同一问题。

### 接线约束与已核查证据

- 跨进程 reload 与缓存发布：`current_action_definition` 读数据库，服务现有 `active_definition` 是内存快照。须明确请求捕获与刷新合同，并覆盖外部 reload 后新请求、进行中请求和撤权。
- Action/history 一致性：`ActionService::execute_inner` 重新读取当前定义，`EditStore::history` 读取缓存。先检查既有 stale-definition 防护，不能直接判为漏洞，也不能宣称共享上下文已覆盖全部分支。
- MCP session 复用、身份变更、直接调用未列出的工具均需有拒绝测试；只过滤 `list_tools` 不足以授权。
- 正常关闭、启动失败和超时需要显式清理 monitor/audit。Grok 建议的 MCP request id 幂等方案不直接采纳，必须核对 Action command_id 合同。

源码复核后的最小设计：

- storage 增加 `request_snapshot()`，复用 `reload::begin_definition_tx` 的共同定义锁，在短事务中读取 definition、roles、bindings；返回后立即释放事务。PG 普通 begin() 下两次 SELECT 不等于一致快照，不能只把独立读包装进普通事务。证据：`scope_store.rs:281-312,420-424,507-544`、`sql_exit.rs:63-77`、`reload.rs:186-200`。
- Query/L2 直接接收该快照。Action 的准备、validate-only、replay/result lookup 与 history 增加接收固定定义的最小入口，共享原内部逻辑，保留旧 API wrapper；不增加后台轮询缓存线程。
- 提交 fence 保留，旧版本 Action 返回结构化冲突，禁止自动切换定义后重放写入。validate-only 仍走 fence、CAS 和 guard。证据：`commit.rs:615-625,661-674,755-780`；这履行既有 ADR-0016，而非新增失败策略。
- replay 采用本次请求权限检查，保留原 receipt/time，不绑定原定义版本，也不增加当前对象可见性重验。`ontoforge-action/tests/idempotency.rs:237-285,362-478` 已规定这些边界。
- MCP 每次 HTTP 请求及工具调用认证/授权，session ID 不充当凭据，跨主体复用 session 拒绝。stdio 只桥接同一 `/mcp`，凭据不进入 stdout、错误正文或跨 origin redirect；写入不自动重试。
- 停机先停止接入，等待已接受 Action 和其依赖读取完成；有界取消独立查询后排空审计，再显式关闭 monitor 和数据库。失败要可见，不能把 drain timeout 当作成功；具体配置上限由研究和测试落实。

```text
REST request -----------+
MCP HTTP/stdio bridge --+--> auth --> short coherent metadata snapshot
                                     | transaction ends here
                                     v
                                compiled principal scope
                                     |
                    +----------------+----------------+
                    v                v                v
              Query/history     Action pipeline    L2 service
              fixed definition  fixed preparation  attested + audit
                                commit fence        fail-closed
                                     |
                                edit log/receipt
```

现实故障：外部 reload/revoke、MCP session 被另一主体使用、响应丢失后 Action 重试、启动中 monitor 失败、SSE 长连接拖住退出。错误和测试映射见 `07-TEST-MATRIX.md`。

## 2. Code Quality Review

Status: REVIEWED。无独立于既有 ADR 的新取舍；以下是实现必须满足的组织边界。

- DTO 和结构化错误放 `ontoforge-api`，Rust client 复用；REST/MCP 适配共用 server 宿主入口，不复制 scope 或 Action 算法。
- 保留已有错误枚举及 apiName/request_id 脱敏，L0 不序列化原始 Trino/物理名称；L2 错误按 ADR-0019 处理。解析失败与 framework rejection 同样映射到 wire envelope。
- 动态 action 工具名和输入 schema 生成是唯一一份逻辑，`list_tools` 与 `call_tool` 必须复用授予/名称检查；不能通过未列出工具名直接越权。
- 快照入口只增加必要 public API；旧 API 作为 wrapper，共用实现，避免破坏库调用者。API crate 已依赖 query，研究需检查 DTO 是否意外迫使 client 引入后端执行能力，优先轻量 wire 类型和显式转换。
- dev seed 幂等且只初始化空状态，不覆盖持久 Action/历史；reset 只作用于显式 dev 数据目录，活跃实例必须拒绝并发 reset。
- 新增模块保持依赖方向，先 `rg --files` 核实真实布局；占位 doc comment 的 Phase 编号随实际交付纠正。无需无关重构。

## 3. Test Review

Status: REVIEWED。已产生 `07-TEST-MATRIX.md` 的分支图、故障矩阵和 SC 映射。Rust 现有 cargo test/insta/proptest；CI 含 Windows/Linux、SQLite 和 Linux 真实 PG/Trino。TS 测试工具须在研究后锁定。

核心 snapshot、认证/授权、Action 接线和生命周期强制 TDD，先有失败断言再实现。现有库级测试继续保留；新增服务级测试不能只调用库函数后声称验证 HTTP/MCP。

目前缺少的服务级证据属于 Phase 7 待交付项，已逐项纳入测试合同，不是假装已有 coverage。新工具动态列表须先 spike；端到端必须真实启动进程、持久化重启和 stdio，不用 mock 取代最终证据。无 LLM prompt/eval 产品改动。

## 4. Performance Review

Status: REVIEWED。无新增基础设施或性能目标取舍；以下限制进入 PLAN。

- metadata snapshot 读取短暂串行，这是复用现有锁的明确成本；锁不能覆盖 scope 编译、网络/source 读、MCP SSE 或响应发送。SQLite/PG 并发测试须证明长查询不阻塞后续 snapshot/reload。
- 每请求 scope 编译先保持正确，暂不增加有失效复杂度的授权缓存；只有实测需优化才按 definition+scope 内容指纹缓存不可变编译结果，不能按 session 缓存权限。
- 沿用 `ExecutionLimits` 的 backend calls/raw candidates/backfill keys/decoded bytes，以及 Action item/edit/body limits；HTTP/MCP 入口另加有界 body、并发、超时和结果 rows/bytes 限制，不能先完整聚合无限结果再截断。
- 历史保留 keyset 与同记录完整页的现有语义，wire 层仍须字节上限；元数据工具分页先做授权过滤。L2 discovery/execute 复用库层 allow-list 和 LIMIT。
- 不给每个 stdio bridge 启动独立 runtime/数据库；长连接单独纳入 drain，连接池和跨平台 TLS 保持现有约束。

## NOT in Scope

- GraphQL、ontology 级强类型 SDK、L1 SQL、管理写工具：既有 ADR 已延后，不能顺手加入 MCP。
- 新的后台 refresh 服务、分布式缓存、独立 stdio runtime：当前 request snapshot 和共享服务足以满足合同。
- 创建 GitHub Release、发布包、合并 PR #7：当前交付实现和可验证发行流程，外部发布仍按用户授权。

## Workstreams

| Step | Modules | Depends on |
|------|---------|------------|
| 真实 tracer 与请求快照 | storage/action/api/server/CLI | spike 与研究 |
| REST/OpenAPI 扩展 | api/server | tracer |
| MCP/bridge | server/CLI | tracer 与稳定 DTO |
| Rust/TS clients | client 与 TS package | 稳定 OpenAPI |
| 生命周期/dev/release 验收 | server/CLI/CI | 对应可运行功能 |

主宿主模块同一 lane 顺序修改；client 模块在合同稳定后可与宿主测试独立并行。当前无 worktree 切换；真正执行 wave 时按用户团队规则指定 ownership，不让 worker 同时改 server 核心。

## Implementation Tasks

- [ ] T1: 完成 rmcp 动态工具/schema、请求身份/session、取消和同进程 HTTP 集成 spike；失败须回到合同核查，不擅自换库。
- [ ] T2: 用 TDD 实现原子 metadata snapshot 和 Action/history 固定定义入口，覆盖双 store reload/revoke races，保留 fence 与 replay 合同。
- [ ] T3: 复用宿主实现 REST/MCP 适配、结构化错误、健康状态和有界生命周期，覆盖真实进程及两个 transport。
- [ ] T4: 由端点合同生成 TS 平台层、完善 Rust client、持久 dev 演示和三平台 release workflow，逐项映射六条 SC。

具体文件和可运行验收命令由研究与 GSD PLAN 锁定。本轮未产生范围外 TODO；没有向根目录追加新 backlog。

## Outside Voice

按用户已授权的 Grok 第二意见路线执行两轮咨询：91385 超时只有部分输出；41414 正常 exit 0。后者提醒一致读取、写入 fence 和 Action 依赖读取的关闭顺序，已用源码核查。history 新版本列、原定义 replay 绑定和 monitor 写 read_audit 的说法不符合现有实现/合同，未采纳。无尚未解决的产品决策分歧，完整事实核对见 findings。

## Completion Summary

- Step 0：完整范围沿用已有批准，未删 SDK/release。
- Architecture：D1 已接受；快照/生命周期风险已约束并映射测试，0 个待决选项。
- Code Quality：依赖方向、共享宿主和脱敏检查完成。
- Tests：分支图与 10 组验收已落盘；实际测试均待实施，未声称通过。
- Performance：锁持有边界、内存/调用预算和长连接排空已检查。
- 新范围外 TODO：0；已知静默失败且无测试/错误处理规划的 critical gaps：0。
- 设计结论：可进入研究与 PLAN；rmcp spike 和独立 plan-checker 是后续门槛。

## Sources

源码与 ADR 的相对路径见上文及 CONTEXT。Context7 本次返回的 primary sources：

- https://github.com/tokio-rs/axum/blob/main/axum/src/serve/mod.rs
- https://github.com/tokio-rs/axum/blob/main/axum/src/docs/middleware.md
- https://docs.rs/rmcp/latest/rmcp/handler/server/trait.ServerHandler.html
- https://docs.rs/rmcp/latest/rmcp/transport/streamable_http_server/tower/struct.StreamableHttpService.html
- https://docs.rs/rmcp/latest/rmcp/transport/streamable_http_server/session/trait.SessionManager.html

`latest` 文档只能证明检索时的接口说明；版本选择、锁定和运行结果仍待研究及 spike。

## GSTACK REVIEW REPORT

| Review | Runs | Status | Findings |
|--------|------|--------|----------|
| Step 0 + Architecture | 1 | Reviewed | D1 已接受；接线约束见正文 |
| Code Quality | 1 | Reviewed | 无新增待决取舍 |
| Tests | 1 | Planned, not executed | 10 组测试合同和分支图 |
| Performance | 1 | Reviewed | 短锁与有界预算 |
| Outside Voice | 2 | 1 success / 1 timeout | Grok 建议已逐项源码核查 |

VERDICT: DONE_WITH_CONCERNS。设计审查完成，0 unresolved decisions；允许进入研究/规划。未代表实现通过、阶段完成或允许发布。rmcp spike、TDD、独立 plan-checker 和正式服务验收仍为必需门槛。
