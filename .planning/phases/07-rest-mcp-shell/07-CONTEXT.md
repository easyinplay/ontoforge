# Phase 7: REST & MCP Shell - Context

**Gathered:** 2026-10-28
**Status:** Ready for engineering review

<domain>
## Phase Boundary

交付范围和验收标准以 `.planning/ROADMAP.md` Phase 7 与 `.planning/REQUIREMENTS.md` 为准。REST、OpenAPI 3.1、Rust/TS client、MCP、dev 演示、三平台 release 均保留；L2-04 与 SCOPE-07 须有本阶段服务级证据才能关闭。
</domain>

<decisions>
## Implementation Decisions

### MCP stdio 运行方式

- **D-01:** stdio 作为认证桥接，连接已经运行的 `serve`，共享服务状态、权限和审计。MCP 客户端使用前需要先启动 `serve`。

### dev 数据生命周期

- **D-02:** `serve --dev` 默认保留 Action 修改和历史，需要时显式重置。重启不能静默恢复初始数据。

### 工程审查事项

健康状态已接受，见下节。优雅关闭、桥接认证与取消边界、跨进程 reload 和每请求定义快照继续接受工程审查。Grok 输出属于第二意见，不代表用户接受或实现已经验证。

### 服务健康契约

- **D-03:** 用户选择工程审查 D1 的选项 1：仅已配置的 L2 故障、L0 查询和 Action 仍正常时，`/health` 返回 HTTP 200，正文标记 `degraded` 并暴露脱敏的 L2 禁用原因，监控须检查正文。核心能力故障时返回 HTTP 503。L2 故障期间继续 fail-closed；未配置的可选 L2 不算意外故障。
</decisions>

<canonical_refs>
## Canonical References

研究、规划和实施前须读取下列合同；已接受 ADR 不在本阶段重新选型。

- `.planning/ROADMAP.md` Phase 7：六项 Success Criteria。
- `.planning/REQUIREMENTS.md`：API-01..05、MCP-01..08、DX-01/02/04、L2-04、SCOPE-07。
- `docs/designs/trino-engine-sql-tiers.md`：已批准的产品边界、SQL 分层与部署约束。
- `.planning/adr/0007-api-surface.md`：axum、utoipa、rmcp 与同进程 `/mcp`。
- `.planning/adr/0008-auth.md`：认证边界。
- `.planning/adr/0009-client-sdk.md`：共享 Rust DTO/手写 client、TS 平台层生成方案。
- `.planning/adr/0018-authz-scope-and-sql-tiers.md`：授权与 SQL 分层。
- `.planning/adr/0019-mcp-toolsets-and-l2-tools.md`：主体 toolset、六个 L2 工具、分页与错误边界。
- `.planning/adr/0028-l2-acl-health-phase-boundary.md`：L2 monitor、证明过期与 `/health` 验收。
- `.planning/research/ref-mcp-trino.md`：工具名称和参数参考；只借设计，不复制实现。
- `.planning/phases/06-l2-raw-dql-toolsets-audit/06-VERIFICATION.md`：上游库级交付证据。
</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- `crates/ontoforge-storage/src/store.rs`：`EditStore::open`、迁移、文件 SQLite 和 read audit worker。
- `crates/ontoforge-auth/src/authn/dev.rs`、`crates/ontoforge-auth/src/authn/mod.rs`：dev key、dev verifier、请求认证。
- `crates/ontoforge-query/src/lib.rs`、`crates/ontoforge-query/src/l2.rs`：已有 QueryService / L2Service。
- `crates/ontoforge-action/src/lib.rs`：ActionService、validate-only 和幂等命令。
- `crates/ontoforge-storage/src/history.rs`：当前可见行约束下的历史读取。
- `crates/ontoforge-engine/src/trino/l2_monitor.rs`、`crates/ontoforge-engine/src/trino/l2_health.rs`：启动/周期 attestation 和可脱敏的状态原因。

### Established Patterns

- 沿用已有 scope、Action、审计和 fail-closed 逻辑；外壳负责认证、捕获上下文、调用及错误映射。
- API、server、client crate 仍为占位；CLI 尚无 `Serve` 分支。
- SQLite fixture 引擎是只读源替身；Action 与历史保存在独立 edit store，不能混淆两者生命周期。

### Integration Points

- 服务必须显式持有 monitor 并排空 read audit，不能依赖 Drop 完成关闭。
- 跨进程 reload 的服务缓存刷新、Action/history 的定义读取入口须有并发测试，不能只沿用 Query 的快照测试后宣称全部请求满足版本固定。
</code_context>

<specifics>
## Specific Ideas

无额外产品扩展。PR #7 保持 Draft、OPEN，不合并。
</specifics>

<deferred>
## Deferred Ideas

无本次新增的范围外想法。ontology 级强类型 SDK 仍按 ADR-0009 留待 MVP 之后，不能与本阶段 Rust/TS 平台层 client 混淆。
</deferred>
