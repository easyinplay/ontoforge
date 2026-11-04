# Phase 7 Test Matrix

Date: 2026-10-28
Source: plan-eng-review
Status: PLANNED，以下不是已通过测试。

## 请求分支图

```text
REST /mcp / stdio
  |
  +-- malformed/body limit --> structured error, no dispatch
  +-- missing/invalid/expired auth --> denied, no data
  v
atomic definition + roles + bindings snapshot
  +-- DB/lock/config failure --> fail-closed, sanitized error
  v
principal scope / toolset
  +-- absent grant or cross-principal session --> denied
  +-- list --> filter before pagination, no admin tools
  v
operation
  +-- metadata/query/history --> frozen definition, masked output, bounded page
  +-- Action validate --> full guards/fence, no edits/log/command/outbox
  +-- Action submit --> claim/fence/guard/commit --> receipt and history
  |     +-- stale definition --> conflict, zero business writes, no auto-retry
  |     +-- lost response --> same command replay or result lookup
  |     +-- wrong principal/changed grant --> deny without object leakage
  +-- L2 --> allow-list + fresh attestation + audit reservation
        +-- disabled/expired/audit unavailable --> no query admission
        +-- failure/success/cancel --> durable audit outcome
```

## 生命周期分支图

```text
startup --> config/store/definition/auth --> engine + L2 monitor --> serve
   |                                               |
   +-- failure --> explicit cleanup/error           +-- failed probe --> L2 disabled
                                                       /health 200+degraded
                                                       recovery --> enabled

shutdown --> stop intake --> finish accepted Actions + dependencies
                     --> cancel/drain independent reads and long connections
                     --> durable audit drain --> monitor cleanup --> close DB
                           |
                           +-- timeout/sink/cleanup error --> explicit failed exit
```

## 验收矩阵

| ID | 入口/用户流程 | 故障与边界 | 自动化要求 | SC |
|----|---------------|------------|------------|----|
| T01 | REST 每端点组 | 元数据、pk、query/filter/search/searchAround/aggregate、Action、历史、result lookup；未知字段/非法 JSON/body 超限 | route contract +真实 listener；错误 envelope/request_id，无原始物理名 | 1 |
| T02 | definition/scope 快照 | SQLite/PG 双 store reload、scope apply、roles/bindings 交错；新请求见已提交变更，进行中读保持旧快照 | TDD gates，不用 sleep 代替确定同步；长查询不持有 metadata 锁 | 4 |
| T03 | Action 写与重试 | validate-only、旧版本 fence、并发相同 command、异参、过期、丢响应、跨主体、撤权后 replay/result | SQLite/PG；保留原 receipt/time，拒绝分支零业务写；不自动重试 | 1,4 |
| T04 | MCP 动态工具 | core/full/sql:raw 主体差异、动态 Action schema、直接猜工具名、分页 | rmcp spike 先行；list 和 call 双重授权；六个 L2 名称/参数对齐 | 2,3 |
| T05 | 两种 MCP transport | stdio 桥接未启动服务、无效 token、跨主体 session、expiry、断连、取消、redirect origin | 实际启动 stdio 子进程和 /mcp；stdout 仅协议；同身份与 REST 结果一致 | 2,4 |
| T06 | 结果边界 | 列遮蔽、跨角色行列、隐藏对象、历史当前可见性、keyset/truncated/rows/bytes | JSON 断言、无越权副作用；短页不冒充末页 | 1,3,4 |
| T07 | /health 与真实 L2 | 初始/失败/过期/恢复，核心故障与未配置 L2 分开，禁用原因脱敏 | HTTP 200+degraded 对应 L2 拒绝；核心故障503；真实 PG/Trino integration | 6 |
| T08 | 启动/停机 | 端口占用、初始化中断、Ctrl+C、活跃 Action、查询取消、SSE、audit sink 失败、monitor cleanup 失败 | 真进程、有界退出、持久 audit；不能仅测试 Drop 或用退出码0掩盖失败 | 6 |
| T09 | Windows dev 演示 | 空目录 seed、重复启动、Action→历史→重启、显式 reset、活跃实例 reset 拒绝 | Windows 无 Docker/Trino；真实文件 SQLite；修改与历史跨重启保留 | 5 |
| T10 | clients 与发行 | Rust/TS 每端点组、OpenAPI3.1 snapshot drift、三平台 binary artifact | 编译/类型检查/contract tests，Windows/Linux/macOS release build/checksum；不在验收中发布真实 release | 1,5 |

执行命令和具体测试文件由 RESEARCH/PLAN 锁定。正式阶段 VERIFICATION 必须引用真实输出；本文件不维护通过计数，不替代 GSD VALIDATION。
