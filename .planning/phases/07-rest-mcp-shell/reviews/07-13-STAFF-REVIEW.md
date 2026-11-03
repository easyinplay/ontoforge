---
phase: 07-rest-mcp-shell
reviewed: 2026-10-31T09:50:48Z
depth: deep
review_head: 54414ebd369ebd446b654e99c71dd742925ecd16
initial_head: 7fa24015d0ccd96cf6edf3ae88a34ac030549d51
diff_base: c9f823cf7b0cf01a382a3e3c21c912887bdabb94
base_branch: gsd/phase-06-l2-toolsets-audit
files_reviewed: 21
files_reviewed_list:
  - clients/typescript/src/index.ts
  - clients/typescript/src/codec.ts
  - crates/ontoforge-api/src/dto.rs
  - crates/ontoforge-server/src/lib.rs
  - crates/ontoforge-server/src/context.rs
  - crates/ontoforge-server/src/state.rs
  - crates/ontoforge-server/src/error.rs
  - crates/ontoforge-server/src/health.rs
  - crates/ontoforge-server/src/lifecycle.rs
  - crates/ontoforge-server/src/mcp.rs
  - crates/ontoforge-server/src/mcp_actions.rs
  - crates/ontoforge-server/src/mcp_catalog.rs
  - crates/ontoforge-server/src/mcp_l2.rs
  - crates/ontoforge-server/src/mcp_sessions.rs
  - crates/ontoforge-storage/src/request_snapshot.rs
  - crates/ontoforge/src/serve.rs
  - crates/ontoforge/src/mcp.rs
  - crates/ontoforge-server/tests/mcp_transport.rs
  - crates/ontoforge-server/tests/mcp_l2.rs
  - crates/ontoforge/tests/cli_stdio.rs
  - crates/ontoforge/tests/cli_shutdown.rs
findings:
  critical: 5
  warning: 0
  info: 0
  total: 5
status: issues_found
review_protocol: gstack-review-spawned-session
---

# Phase 7: Paranoid Staff Engineer Review

结论：发现 4 个安全与生命周期 BLOCKER，以及 1 个 API wire BLOCKER。均来自当前源码和 caller/callee 的静态确认；本 lane 未运行新动态复现，不能把下列建议测试写成已执行 RED。Draft PR 可以承载修复与 CI，当前报告不支持合并或发行。

## Scope And Evidence

- 基线为 `gsd/phase-06-l2-toolsets-audit`，merge-base 为 frontmatter 的 `diff_base`。审查期间 HEAD 从 `7fa2401` 前进到 `54414eb`；最终定位重新核对于后者工作树，其他 lane 的 CLI lint 改动仍可能继续。
- 业务范围为已实现的 07-01..12、07-15、07-16 与验收中的 07-13。07-14 发行仍是已登记 pending，不作为新缺陷。
- 读取 gstack `review/SKILL.md`、核心 checklist、Testing、Maintainability、Security、API Contract checklist；串行应用这些视角。4 slots 已满，按主会话指令未新增 specialist/Red Team subagent。没有运行外部 Codex CLI、Grok、Greptile、远程 CI，不能宣称存在这些独立意见。
- 读取 `07-CONTEXT.md`、`07-TEST-MATRIX.md`、07-13 计划、07-08/07-15 相关合同、ADR-0008/0018/0019。工作目录无 `AGENTS.md`、`.codegraph/`、`.codex/skills/`、`.agents/skills/`；适用用户提供的 AGENTS 规则。GSD `query agent-skills gsd-code-reviewer --skills-only` 未返回额外 skill。
- 上述 21 文件完整读取；另外针对 caller/callee 读取 `rest.rs`、Rust client、storage receipt/history、engine dispatcher/config/Trino、真实 Trino HTTP/health fixture、rmcp 3.8.0 registry source 的相关部分。没有完成全分支 88 个非 planning 变更文件的逐文件深审。
- 已消费 `07-13-GATE-REVIEW.md` 与 SDK/MCP 复签；原 CR-01/02/03、Rust ordinal、Rust history URL 修复不重复报告为 unresolved。本报告的新 API wire 问题与已修的 bigint/raw/ordinal/URL 根因不同。

## Narrative Findings (AI reviewer)

### CR-01: [BLOCKER] L2 在真实 serve 组合中使用 ontology 身份，绕过独立 ACL 证明

**Confidence:** 10/10，静态调用链确认，未动态复现。

**File:** `D:/DataSpellProjects/ontoforge/crates/ontoforge/src/serve.rs:178`

**Evidence:**

```rust
AnyEngine::Trino(engines.ontology)
host = host.with_l2_status_provider(move || l2.l2_status());
```

前者位于 `serve.rs:178`，交给 `ServerHost::new` 的唯一 `engine`；后者位于 `:205`，独立 `l2` 只留在健康状态闭包。`ServerHost` 无第二执行 engine；`mcp.rs:98` 将 `self.host.engine` 交给 `McpL2Tools`。`mcp_l2.rs:53` 仅调用普通 `engine.status()`，`:92` 的 `self.engine.execute(&sql, &ctx).await` 和 `:189` 的 `describe_tables` 都发送到 ontology engine。`AnyEngine` 仅直接委派，未做身份路由。Trino `execute` 只有 `self.l2_health` 为 Some 才取得 admission lock 并拒绝过期证明，ontology engine 的该字段为 None。

**Issue:** core 健康时，具备 `sql:raw` grant 的请求仍能在独立 L2 未证明、故障或过期期间执行；实际 Trino 身份也是 ontology。此时 `/health` 的 degraded 与执行准入矛盾，独立 L2 凭据和 ACL 自检不再构成执行边界。

**Fix:** 为 host 显式持有独立 L2 执行能力，六工具只能使用该能力及其 engine 内 admission/证明锁；未配置也必须拒绝。保留 L0/Action 的 ontology engine，不能仅检查 provider 然后继续用 ontology 身份执行。

**Required Regression:** 真实 CLI assembly 验证 Trino 请求身份；同一 host 在 L2 初始未证明、失败、过期时六工具均不发查询，REST L0 仍正常。`trino_http_it.rs:495-505` 当前直接用 `engines.l2` 构 host，绕过了有缺陷的 CLI 组合；`health_trino_it.rs:173` 的 core fixture 没有同时调用被禁用的 L2 工具。

### CR-02: [BLOCKER] MCP transport 的 initialize、GET、DELETE 与控制通知绕过认证和初始 owner 绑定

**Confidence:** 10/10，静态调用链确认，未动态复现。

**File:** `D:/DataSpellProjects/ontoforge/crates/ontoforge-server/src/lib.rs:147`

**Evidence:** `.nest_service("/mcp", mcp_service)` 直接挂载 rmcp；外层 `context::admit` 只处理并发与 deadline。应用认证只在 `mcp.rs:53-78` 的 `request_context` 中，由 `list_tools` 和 `call_tool` 调用；session 在首次工具调用才 `admit(session, ctx.principal.id.as_str())`。

rmcp 3.8.0 `tower.rs:2355-2387` 的 `handle_delete` 只取 session ID 和协议版本便 `close_session`；`:1844-1964` 的 `handle_get` 只检查协议、session、event ID 后返回 SSE。它们没有应用鉴权回调。initialize 与 cancellation notification 同样不调用工具 handler。

**Issue:** 无 token 请求可以初始化 session；掌握 session ID 的无 token 或不同主体可 DELETE 会话、接入 GET/SSE，控制通知也不经过逐请求认证。初始身份没有原子绑定；owner entry TTL/容量淘汰后，旧的真实 transport session 仍可被另一主体在下次工具调用重新绑定。已获知的 session ID 被当成凭据使用，违反 07-08 明确的每 POST/GET/DELETE 认证合同。

**Fix:** 在进入整个 `/mcp` transport 前验证 token 与 forged headers，对所有方法/通知验证当前身份；initialize 创建会话时原子绑定初始 owner，之后 GET/POST/DELETE 验证绑定。owner 过期/淘汰应同时使 transport session 不可用；DELETE/关闭同步回收，不能静默重新认领活跃 session。

**Required Regression:** A 初始化后，无 token 与 B 的 GET/DELETE/cancel 被拒绝且 A 的 session 不受影响；无 token initialize 被拒绝；首次工具调用不能由 B 抢绑，TTL/容量边界不能跨主体认领。现有 `mcp_transport.rs:54-95` 只用 invalid token 调 tools/list、用合法 token DELETE，不能证明上述边界。

### CR-03: [BLOCKER] stdio 取消使用错误 ID，server 读 handler 也未消费取消 token

**Confidence:** 10/10，静态调用链确认，未动态复现。

**File:** `D:/DataSpellProjects/ontoforge/crates/ontoforge/src/mcp.rs:155`

**Evidence:**

```rust
let request_id = context.id.clone();
let task = tokio::spawn(async move { handle.await_response().await });
```

`:163-167` 随后将该本地 ID 发给远端。rmcp `service.rs:876` 使用 `self.request_id_provider.next_request_id()` 另生远端 ID，`:900-905` 用它发送，`:917-924` 将它放入公开的 `handle.id`；`ORIGINATING_REQUEST` 只是 extensions 信息，没有 ID 反向映射。

**Issue:** 本地用字符串、非连续数字或不同并发顺序时，取消发往错误 ID，原请求继续运行，还可能取消同 session 的另一远端请求。即使修正 ID，server `mcp.rs` 的 `call_tool/call_read/call_l2` 从未监听 `context.ct`；rmcp `service.rs:1634-1639` 只调用 `ct.cancel()`，不会强行终止应用 handler，因此长读仍继续，审计可记为 success。

**Fix:** 在 move handle 前保存 `handle.id`，正确转发该远端 ID；server 对独立读/L2 的执行 future 与当前请求取消 token 做有序选择，使 engine 清理及 durable cancellation audit 完成。已接受的写 Action 必须独立注册并等待，不能套用读取消逻辑将其 abort；取消不得自动重发写请求。

**Required Regression:** 活跃真实桥接中同时发送本地字符串 ID 与不同数字 ID 的阻塞读；取消其中一个，验证对应远端读被取消、其余请求完成，审计恰有一次 cancelled。另验证取消已接受 Action 不产生半提交或重放。现有 `cli_stdio.rs:38-52` 全程连接 `127.0.0.1:1`，在 HTTP initialization 已失败，根本未进入 `BridgeService::handle_request`；名为取消的 unit test 只测 config/redact。

### CR-04: [BLOCKER] 优雅关闭先等待 SSE，导致永远无法进入 cleanup deadline

**Confidence:** 10/10，静态生命周期确认，未动态复现。

**File:** `D:/DataSpellProjects/ontoforge/crates/ontoforge-server/src/lib.rs:176`

**Evidence:**

```rust
let served = axum::serve(listener, router)
    .with_graceful_shutdown(shutdown)
    .await;
let lifecycle = lifecycle::ServerLifecycle::new(&self).shutdown().await;
```

rmcp config 在 `lib.rs:143-145` 用 default 新建独立 `cancellation_token`，没有连接 shutdown 信号。rmcp SSE body 由该 token 控制结束。`lifecycle.rs:32-35` 的 stop_admission/5 秒 timeout 位于上述 await 之后，且 timeout 只包 `wait_actions`。

**Issue:** 一个仍打开的 MCP SSE 连接即可让 axum graceful await 持续等待，永远不到 audit drain、monitor shutdown 和 lifecycle deadline。独立长读也未在 shutdown 时主动取消。停机无法满足有界退出合同，当前关闭测试不能证明该路径。

**Fix:** 使 shutdown 在等待 HTTP 全部连接结束之前停止 admission 并触发独立读/SSE 的取消；已接受 Action 及其依赖先完成，再 drain audit 与关闭 monitor。将整体关闭放在明确 deadline 内，超时/cleanup 失败返回失败退出，不能只把后置 wait_actions 包进 timeout。

**Required Regression:** 保持真实 `/mcp` SSE body 和阻塞读连接，发 shutdown 后在 deadline 内结束连接与服务，验证 read cancelled 审计持久化以及已接受 Action 完整提交。`cli_shutdown.rs:47-76` 名为 reads_sse 的测试实际上先完成普通 HTTP 请求后才 shutdown；`mcp_transport.rs:127-141` 只调用 tools/list，均未开启 SSE。

### CR-05: [BLOCKER] HTTP response 与公开 DTO 不一致，SDK 静默丢失回执和历史字段

**Confidence:** 10/10，序列化与解码源码确认，未动态复现。

**File:** `D:/DataSpellProjects/ontoforge/crates/ontoforge-server/src/rest.rs:674`

**Evidence:** `receipt_json` 仅 rename 顶层 command/event/time 字段。storage `commit.rs:139-154` 的 item 实际是 `{item_ordinal, edits:[{rule_ordinal,edit_ordinal,kind,type_name,pk,operation,version_after}]}`；公开 DTO `dto.rs:336-343` 却是 `{itemOrdinal,objectType,pk,version}`。TS `index.ts:95` 只将 `decodeWire` 结果 cast 为 DTO，未转换 shape。Rust client `lib.rs:480-484` 用 `edits.first()`，把 item 的其余 edits 直接丢弃。

历史同源错误：`rest.rs:638` 直接把 storage `HistoryEntry` 放进 JSON；storage `history.rs:47-67` 未设 rename_all，输出 `event_id/committed_at/masked_changes`。API `dto.rs:312-325` 宣告 camelCase；Rust client `lib.rs:453-465` 按 `eventId/committedAt/maskedChanges` 解码，缺失时回默认空字符串/空数组。TS 得到的宣告属性则是 undefined。

**Issue:** 客户端依据 OpenAPI 读取 item 的 objectType/version 或历史 eventId/committedAt/maskedChanges 时得不到真实数据。Rust client 对多个 edits 的 Action 丢失回执，对历史静默丢失标识、时间与遮蔽信息；请求仍被报为成功。

**Fix:** 统一 wire DTO、OpenAPI、Rust 与 TS。history 用显式 DTO 转换确保 camelCase 完整字段。receipt 先由主会话确认是否需要保留完整 edits；当前 storage 支持一 item 多 edits，不能无声选第一条。若保留完整结构，扩充共享 DTO/schema 并使 SDK 对完整结构解码；不能靠 TS cast 或返回默认值掩盖不一致。

**Required Regression:** 真实 HTTP Action 含一 item 两条 edits，经 Rust/TS 校验每条回执及 commandResult 一致；真实 history 校验 event/time/maskedChanges 实值，不能只断言 entries.length。receipt 结构决策标记 `STATUS: NEEDS_CLARIFICATION`，history 字段修复无需改变既有已宣告合同。

## Fix Contract

| 边界 | 修复必须保持的行为 |
| --- | --- |
| L2 | 独立受限身份与 engine 内 proof/admission 锁；未配置、未证明、过期、故障均拒绝；L0/Action 保留 core 身份。 |
| MCP 身份 | 所有 transport 方法/通知重新认证；session 绑定初始主体，淘汰不准重新认领；每次工具调用取当前 scope snapshot。 |
| 取消 | 映射真实远端 ID，只取消目标独立读；取消不能误杀并发请求、已接受 Action 或 Action 的 source 依赖，也不能自动 retry。 |
| 关闭 | 先停 admission；已接受 Action 完成；独立读/SSE 终止；audit 持久化；monitor cleanup；全程 deadline，失败退出可见。 |
| wire | response/schema/两个 SDK 同合同；不存在字段与解码失败不能静默替成空值；多 edits 不准无声丢失。 |

## Verification Limits

- 未运行完整 workspace、PG/Trino 服务或新定向测试，避免与冻结执行中的 CI lane 冲突。上述代码链证明不等于可归因的动态 RED/GREEN。
- 主会话提供的已完成结果为 SDK 8/8、Rust 4 unit+6 HTTP、MCP 14/14；本 lane 没有复跑，且弱断言/不同 assembly 不能排除本报告发现。
- 第一轮真实服务指定用例绿，但尾部 shell exit 1；清理后第二轮冻结 verify 正在运行。本报告不将第一轮或第二轮整体记为 passed；Linux CI 也未获得本 lane 的通过证据。
- 没有使用可用但未运行的外部 CLI 冒充第二模型复审。已有独立 SDK/MCP 复签只覆盖其各自 scope，不为本报告新发现背书。
- 仅新增本报告，不修改实现、共享 planning 状态或其他 lane 文件。依据 reviewer 的只读/不提交约束，提交由 orchestrator 负责。

Recommendation: Fix CR-01 and CR-02 before merging because raw SQL currently reaches the ontology identity and MCP transport control paths bypass authentication; cancellation, shutdown, and wire findings also require regression evidence.
