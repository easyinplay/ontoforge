---
phase: 07-rest-mcp-shell
reviewed: 2026-11-03
role: security
review_base: fd24beb36091bc4d9fc56e2984a4103ee923fe3f
review_head: 972c7be9cc3e91460d300b6f45fd814df023a539
review_mode: read-only-incremental
status: clean
findings: []
independence: partial-prior-RF03-and-RF10-implementation
phase_acceptance: pending
---

# Phase 7 gstack Security 增量审查

```text
NO FINDINGS
```

本结论仅表示固定 `git diff fd24beb..972c7be` 中未确认新增、可操作的安全缺陷，不表示整个 Phase 7 已完成或不存在任何漏洞。审查使用 `gstack/review/specialists/security.md` 与 `gstack/review/checklist.md`，以该提交内容为准，未将当前未提交 planning 修改纳入审查。

## 独立性声明

本 reviewer 参与过 RF-03 Action 诊断及 SDK 实现，也参与过 RF-10 health wire 修复。因此本报告并非对这些代码完全 fresh 的审查，不能替代其他 fresh reviewer 的交叉复核。dev、history 和 MCP 的具体实现由其他 owner 完成；本轮重新读取其固定 diff、实现及安全相关调用方。

## 静态证据

以下路径与行号均对应 `972c7be`。

| 边界 | 核查证据 |
| --- | --- |
| dev 生命周期锁 | `crates/ontoforge/src/dev.rs:83` 在取得锁后重新校验目录与 marker，并在 reset 写入前检查已有 key；`:300-327` 拒绝 lock symlink、使用不截断的文件句柄和 `try_lock()`，不再依赖 PID 或 unlink 竞争。`serve.rs` 将 `DevState` 保留在服务生命周期内。 |
| 私有目录与密钥 | `dev.rs:45-55` 拒绝 Unix group/other 可访问的已有根目录，`:111-119` 新目录使用 0700，`:193-215` 新 key 使用 0600 与 `create_new(true)`，`:218-233` 对已有 key 的不安全权限拒绝启动，且检查先于 reset 删除。 |
| 认证与授权入口 | `crates/ontoforge-server/src/lib.rs:165-169` 保留 MCP 认证中间件；`context.rs:290-360` 拒绝伪造 principal/roles/scope/limits headers、验证 session owner；`mcp.rs:92-129` 每次请求重建主体与 scope，`:547-557` 在 dispatch 前核对 scope 生成的工具目录。未发现新 allow-by-default 路径。 |
| MCP context 错误 | `mcp.rs:35-71` 仅解析最多 4096 bytes、允许固定 code 集合；request ID 来自合法服务端 header 或可信 fallback，不采用 body 中的 request_id，不透传原始 body/details。 |
| L0/L2 审计与错误 | `mcp.rs:303,363,407,459` 的 reserve observer 更新共享 trace；`mcp_l2.rs:40-61` 穷举 EngineError 并输出固定错误码，`:134-137,234-237` 在 source await 前公布 reserve ID，finish/source 错误继续携带该 ID。外层取消分支使用同一 trace，未把底层 Display 发往 wire。 |
| Action 诊断允许范围 | `error.rs:78-108` 仅投影 ParameterErrors 和 CriteriaFailed；REST `rest.rs:685-692` 与 MCP `mcp.rs:642-651` 共用该投影。Store/Commit/InvalidRequest 的原始文本未新增输出。参数 reason 可含调用者自己的非法字面量，不能称为完全脱敏文本；criteria message 来源为配置。 |
| history 权限与预算 | `rest.rs:590-641`、`mcp.rs:416-502` 保留受 scope 限制的当前对象读取及固定 definition；两入口都使用 Ordinary caller。共享 `history_page.rs:6-92` 在规范化页上计算预算，不拆同 seq 记录；底层 `storage/src/history.rs:220` 仍按当前可见列过滤，原始 Action params 仅 Admin 分支读取。 |
| SDK 不可信响应 | `clients/typescript/src/index.ts:77-93`、`crates/ontoforge-client/src/lib.rs:283-321` 先识别完整 envelope，再严格解析诊断；普通 JSON HTTP 错误保持原分类。TS 错误正文在 fetch wrapper 中以无损 codec 解析，没有 eval、动态模板执行或新增网络目的地；Rust 保留既有 aliases。 |

核读了相关负向测试源码：dev 多实例/空锁/reset/Unix 权限与 dangling symlink；MCP hidden-tool、真实 listener 的底层敏感 sentinel、L2 allow-list 与 audit unavailable；REST history 审计关联与错误类别；Action Store/Commit/InvalidRequest 不输出底层文本；SDK 普通 502 JSON 与畸形 structured envelope 对照。测试源码存在与断言覆盖不等于本轮实际执行通过。

## 范围与证据局限

- 本轮没有运行 Cargo、测试、服务、外部扫描或渗透验证；没有修改实现、提交或推送。已有复签报告中的动态结果归其原执行者，本报告没有把它们记作本轮重跑。
- 审查主线是增量认证/授权、错误与审计信任边界、dev 文件安全；未对全仓库全部历史代码作安全认证，也未扩展为性能或完整 API 合同审查。
- dev 的同权限恶意进程持续替换路径、父目录全树及 Windows ACL 未进行动态威胁验证；本结论不扩张为目录句柄级防替换保证。
- L2 测试使用的可控 HTTP source 不等于实际生产 Trino 集群。本轮静态读取共享 trace 的取消路径，不宣称取消时 JSON 错误一定能送达已断开的客户端。
- RF-01 drift owner、RF-05 MCP 参数命名空间和 07-14 发布工作明确未实施，本轮不重复列为 finding，也不视为已关闭。
- 未发现需要主会话转交 Claude Code 澄清的新增安全灰区。
