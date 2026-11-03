---
phase: 07-rest-mcp-shell
reviewed: 2026-11-03T08:59:41Z
depth: standard
review_base: f834d039749e39d0b2fdf3cac6153a3ae30aced1
review_target: frozen-working-tree-blobs
scope_contract: RF-03
source_blobs:
  crates/ontoforge-server/src/error.rs: 250b188e8599ad5e44da2b3bd6265c84cf3b895b
  crates/ontoforge-server/src/rest.rs: 68aec5d280cd154a1b76dd483ab5327cd8cebe48
  crates/ontoforge-server/tests/http_action_diagnostics.rs: 9fee3cbde7dad4427151f347b716944f3b74402e
  crates/ontoforge-api/src/dto.rs: 0b25551e0db3fc673f0d12c4de74260b948feba8
  crates/ontoforge-api/src/openapi.rs: eafbb09bc44a2c881951049a89a8641dcc4557c8
  crates/ontoforge-api/tests/openapi_snapshot.rs: 39374ac662dbbdd84d0fdb81815767974c5b54b0
  crates/ontoforge-client/src/lib.rs: b4bc156c75f58ff6f860c418329ed11678545041
  crates/ontoforge-client/tests/error_diagnostics.rs: b68ebc681a923a0a1f2d1b3bdd149228d770dc52
  clients/typescript/src/index.ts: 17b244d5590e985cdc25beb4564a63fde8887f0e
  clients/typescript/src/generated.ts: bc2958b88b3dc09b71384125ea1ff83a3d291710
  clients/typescript/tests/contract.test.mjs: f9e137465be212e323ace0c881629a3b8bae5665
  clients/typescript/tests/error-types.ts: cb56abf0a78048090d9a98727158a6e347672f4b
  docs/api.md: 245cb01212da9d3825350248b706758aed9fc346
  docs/openapi.json: 0d1d0a22c6a6c6a2898efa22052c5155e863d0aa
files_reviewed: 14
files_reviewed_list:
  - crates/ontoforge-server/src/error.rs
  - crates/ontoforge-server/src/rest.rs
  - crates/ontoforge-server/tests/http_action_diagnostics.rs
  - crates/ontoforge-api/src/dto.rs
  - crates/ontoforge-api/src/openapi.rs
  - crates/ontoforge-api/tests/openapi_snapshot.rs
  - crates/ontoforge-client/src/lib.rs
  - crates/ontoforge-client/tests/error_diagnostics.rs
  - clients/typescript/src/index.ts
  - clients/typescript/src/generated.ts
  - clients/typescript/tests/contract.test.mjs
  - clients/typescript/tests/error-types.ts
  - docs/api.md
  - docs/openapi.json
findings:
  critical: 0
  warning: 0
  info: 0
  total: 0
status: clean
original_findings:
  CR-01: resolved
phase_acceptance: pending
---

# Phase 7 Action Diagnostics Recheck

RF-03 当前冻结增量复签为 clean，无未解决 BLOCKER 或 WARNING。初轮发现的两套 SDK 普通 JSON 错误兼容回归已关闭；完整结构化 envelope 的畸形诊断仍被拒绝。

本轮仅增量核查 TS index/contract、Rust client lib/error_diagnostics 四文件；其余原 14 文件范围的 10 个 blob 与初轮复签一致，沿用原源码结论。rest.rs 仍仅 action_response、error.rs 仍仅 action_envelope；没有重做整个阶段或 RF-07 helper。只更新本报告，未运行 Cargo/npm、修改实现或共享 planning，未提交。

## Narrative Findings (AI reviewer)

无新增未解决发现。

### CR-01: [BLOCKER, resolved] 普通 JSON HTTP 错误保留旧兼容行为

**原问题：** TS 在确认 code/apiName/request_id 前校验 message/parameterErrors；Rust 在基础字段采用 alias/default 兜底时仍无条件 serde 解码诊断。普通 502 `{"message":{"reason":"upstream"}}` 因而分别变成 generic Error、ClientError::Decode，丢失原有 HTTP 错误类别。

**当前源码：**
- `clients/typescript/src/index.ts:77-90` 先检查三个必需字段为 string，未识别时返回 undefined。OntoforgeHttpError 保留 HTTP status、原解码 body、既有 code/status 或默认 Error.message；完整 envelope 才校验诊断。
- `crates/ontoforge-client/src/lib.rs:283-313` 先解析 code、apiName/api_name、request_id/requestId，recognized 成立后才解释诊断。未识别时保留 HttpStatus 与原基础字段 default/alias 行为，诊断置 None；没有改变 Rust 既有非 JSON 响应解析失败行为。

**新增行为证据：** 两 SDK 都通过真实 HTTP peer 返回五种普通 JSON 502 对照（缺失/部分/错误类型必需字段），验证仍是 typed HTTP 错误。TS 检查 status/body/envelope/message；Rust 检查 HttpStatus/502/code 与无诊断。Rust 还验证 legacy aliases 的合法诊断保留、同一 alias envelope 的坏诊断仍返回 Decode。TS 对完整 envelope 的坏 message/parameterErrors 继续拒绝。

## Preserved Scope Evidence

server action_envelope 仍只投影 ParameterErrors 的 itemOrdinal/name/reason 和 CriteriaFailed 的配置消息；InvalidRequest、Store、Commit 等无动态 Display。参数 reason 可回显调用者非法字面量，文档已说明；criteria 消息来自配置 failure_message。当前 HTTP Action 条数限额先于 ordinal 枚举，u32 投影有既有限额支撑。Rust serde/TS responseOrdinal 继续拒绝负数、超过 u32 和诊断字段错误类型。

DTO/OpenAPI/生成类型的可选诊断、TypeScript Error.message 原展示和 typed envelope 新接口没有被本轮兼容修复改变。原 server/SDK 真实 validateOnly 与提交失败用例继续覆盖参数和 criteria 两条路径。

## Actual Library Path And Limits

初轮已读取本地 openapi-fetch 0.17.0 的实际 export `dist/index.mjs`：非 2xx 路径无视 parseAs:text，仍 JSON.parse。现有 fetch wrapper 在库解析之前 raw text→decodeWire，保留 ordinal 为 bigint；库在无 middleware 时原样重抛 fetch 异常。本次只调整 envelope 识别顺序，未改 wrapper。

网络/Abort/response.text 读取失败在 decode try/catch 外；构造和每次发送前仍检查 lossless capability。纯文本/空 body 的 parse 失败保留原字符串再产生 HTTP 错误，静态路径未改变；本轮没有新增这两项专门动态测试，不将普通 JSON 回归夸大为它们的动态证据。既有 Abort 与运行时能力回归在 TS 14 项中继续通过。

## Dynamic Evidence

均为 reviewer 独立核读 owner 日志与实际测试断言，执行归属 owner：

| 本轮日志（target/phase7-action-diagnostics-envelope-*） | 结果 |
| --- | --- |
| ts-red.log | 1 passed/1 failed；真实普通 JSON peer 导致 instanceof OntoforgeHttpError 断言失败，EXIT=1 |
| ts-green.log | 14/14，0 skipped，EXIT=0；含普通 JSON 对照、畸形 envelope、Abort/lossless 既有回归 |
| rust-red.log | 3 passed/1 failed；普通 JSON 返回 Decode，EXIT=101 |
| rust-green.log | 6 unit + 7 contract + 4 diagnostics = 17/17，EXIT=0 |
| clippy.log / format.log | client Clippy 与 fmt 均 EXIT=0 |

初轮 RF-03 日志保留为历史证据：server 11/11、Rust 15/15、API 7/7、TS 13/13；此前还有 server/Rust/API 实际 RED、TS 类型 TS2339 RED，以及库错误体解析造成的 ts-error-codec-red。原 13 项 TS 绿色不覆盖兼容回归，因此由本轮 14 项替代其最终结论。

初轮全 server all-target Clippy 的 mcp_l2_defaults.rs 102/100 行阻断已在后续 MCP tranche 解除；`target/phase7-mcp-next-clippy.log` 已核读 Finished（2.88s），全目标 exit0 来自 owner 报告。它是独立 server 门禁，本轮 client Clippy/fmt 日志另有显式 EXIT=0，不混成 reviewer 亲自执行的全 workspace 检查。

这些是 Windows 定向真实 HTTP/SQLite 及可控坏响应证据，不代表整个 Phase 或跨平台整合 CI 已完成。

## Review History

初轮于 2026-11-03T08:40:27Z 以 TS index `283250a9`、Rust lib `cd88b903` 记录 CR-01。当前两 SDK 实现和对照测试完成增量复签，14 文件 blob 清单已更新，未把随后未审的工作区变化算入 clean。
