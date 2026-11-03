---
phase: 07-rest-mcp-shell
reviewed: 2026-11-03T10:17:18Z
specialist: api-contract
base: fd24beb36091bc4d9fc56e2984a4103ee923fe3f
head: 972c7be9cc3e91460d300b6f45fd814df023a539
status: issues_found
recheck_status: clean
rechecked: 2026-11-03T10:38:15Z
recheck_report: 07-HISTORY-OPENAPI-RECHECK.md
findings:
  critical: 0
  warning: 1
  info: 0
  total: 1
---

# Phase 07: gstack API Contract 专项增量审查

## 范围与限制

- 应用 `C:/Users/easyi/.agents/skills/gstack/review/specialists/api-contract.md`，固定范围为 `git diff fd24beb..972c7be`。所有源码、生成类型、文档证据均读取固定提交，未使用其他 worker 正在修改的工作树测试。
- 核对 Rust DTO/SDK、TypeScript SDK/生成类型、OpenAPI 源与 JSON、REST history/Action、MCP describe/Action/L0/L2 的本轮变化。追踪既有领域参数校验、source error 映射和 health 响应以确定实际 wire 行为。
- 未运行 Cargo、测试、服务器或生成器。固定提交的 JSON 经读取解析确认 history response key 集合为 `200, 401, 403, 404, 503`；测试文件仅作为合同与调用路径证据，未宣称本轮复跑通过。
- RF-01、RF-05、07-14 尚未实施，排除；GT01 已由 Testing 专项报告，不重复。没有以未发布产品存在旧客户端为前提要求兼容层或版本升级。

## Narrative Findings (AI reviewer)

### WR-01: WARNING：history 新错误路径未同步 OpenAPI 响应合同

```json
{"severity":"INFORMATIONAL","classification":"WARNING","confidence":99,"path":"crates/ontoforge-api/src/openapi.rs","line":137,"category":"api-contract","summary":"history 本轮通过 query_failure 保留预算、source timeout/cancel 和歧义对象错误，但 OpenAPI 的 history.responses 仍只有 200/401/403/404/503。实际新增或恢复的 400 COST_EXCEEDED、408 CANCELLED、409 AMBIGUOUS_OBJECT、504 DEADLINE_EXCEEDED 均不在声明中，docs/openapi.json 与 TypeScript operations.history 继承了该遗漏。按当前接口描述生成的响应分支无法表达这些实际行为。","fix":"在 history 的 utoipa responses 中增加 400、408、409、504，body 均为 dto::ErrorEnvelope；由源重新生成 docs/openapi.json 和 clients/typescript/src/generated.ts。新增明确断言这些 response status 及其 schema 的合同测试，避免仅比较生成物与源的 snapshot 同步测试掩盖遗漏。","fingerprint":"crates/ontoforge-api/src/openapi.rs:137:api-contract","specialist":"api-contract","evidence":"rest.rs:593-606 和 :640 将错误交给 query_failure；history_errors.rs:74-82 明确要求 504/408，:160-174 要求 400；error.rs:36 将 AmbiguousObject 映射409，query/src/lib.rs:522-523 在PK读取返回多行时产生该错误。OpenAPI history.responses:137-142 未声明上述状态，生成 TypeScript operations.history 亦无对应分支。"}
```

**文件：** `D:/DataSpellProjects/ontoforge/crates/ontoforge-api/src/openapi.rs:137-142`。

**实际合同影响：** 此处不是要求增加新错误行为。本轮 REST 已将过去被吞成 404 的错误恢复为正确类别，并增加完整 history 页预算拒绝；接口描述必须随之更新。当前手写 SDK 的通用 HTTP 错误处理仍可接收这些响应，本 finding 不声称 SDK 会因此崩溃。受影响的是 OpenAPI 的状态合同、生成类型以及使用这些声明生成的接口分支。

**证据强度：** 400、408、504 在固定提交的专用 HTTP 测试中已有明确断言，409 由既有 PK 查询和新 history 错误传播路径静态确认。全部均经同一 ErrorEnvelope 产生，无需更改 wire 形状。

**最小修复验收：** 源 OpenAPI 和生成 JSON 的 history responses 包含 `400, 408, 409, 504`，每项 `application/json` 指向 `ErrorEnvelope`；生成 TypeScript 同步具备这四个响应键。以现有 history 行为测试作为实现侧证据，并补状态集合断言。

## 其他接口核对

- Action 诊断的 `message` 与 `parameterErrors` 在 DTO、OpenAPI、生成 TypeScript、两种 SDK 中均为可选；Rust `u32` 与 TypeScript `bigint` 的实际诊断解码边界一致。仅完整可识别错误 envelope 执行新增诊断校验，普通 JSON HTTP 错误保留既定处理路径。
- health 的可选 `l2.status`、`l2.reason` 与当前服务端启用、禁用、未配置分支相符；不将 RF-01 尚未实施的 drift 行为纳入本结论。
- history 终页、同 seq 不拆分、1..500 软目标、规范化 JSON 字节预算在 REST/MCP 共享 helper 与文档之间未发现另一项确定矛盾。
- MCP ObjectReference 使用固定 ontology 的真实 PK 定义；普通参数使用与 ActionService 相同的 `check_literal`，数组元素 schema 使用对应类型。RF-05 平铺控制参数冲突排除。
- MCP 的新安全错误投影保留 `code/apiName/request_id`；已预留审计 ID 的观察与错误传播按当前固定提交核对。取消测试可信度归 GT01，不在本报告重复计数。

仅新增本专项报告；未修改实现、生成物、其他报告或共享 planning，未提交。

## WR-01 修复状态

2026-11-03 独立复签 clean，WR-01 已关闭。history 九个状态及 schema refs 在源声明、生成 JSON 和 TypeScript 中一致，合同测试先实际 RED 后 API 8 项 GREEN。具体 blob 绑定、日志与范围见 [07-HISTORY-OPENAPI-RECHECK.md](07-HISTORY-OPENAPI-RECHECK.md)。上方 `status: issues_found` 保留原固定范围审查事实，`recheck_status: clean` 表示本次修复复签结果。
