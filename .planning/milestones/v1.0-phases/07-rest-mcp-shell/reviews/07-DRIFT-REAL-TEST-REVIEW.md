---
phase: 07-rest-mcp-shell
review_date: 2026-11-03
status: clean
findings: 0
initial_findings: 1
rechecked: 2026-11-03T11:35:16Z
rt01_status: closed
tests_executed: false
real_services_verified: false
phase_acceptance: pending
source_blobs:
  crates/ontoforge-server/tests/source_drift_trino_it.rs: c14cb6a04626323259e3f5ddf44f26ab6fd18e8b
  crates/ontoforge-server/tests/support/real.rs: 0c1776f1dba49d2810dd2a55dc705f9aeee1bc16
---

# RF-01 真实服务测试只读检查

首轮发现 1 项明确的 MCP 成功判定缺口，现已由文末独立复签关闭。首轮审查对象为 `7765177d76a68e0515f577780e8e9b5e48e8e4fa`，下列原始发现保留为历史记录；当前结论与 blob 见文末。测试主体使用真实生产 serve owner 和真实 Trino/PostgreSQL，没有把直接调用 check_drift 或 shutdown 结果当作 drift 行为成功。未编译或运行本 suite，不作动态通过结论。

## RT01：MCP 可用与恢复断言允许空结果或 transport 错误假阳性

- **分类**：WARNING，FIXABLE，P2，测试验收缺口；已关闭。
- **证据**：`source_drift_trino_it.rs:489-500` 的 mcp_get 丢弃 HTTP status，只解析任意 JSON；`:577`（Narrow/Local）和 `:610`（恢复 Wide）仅检查 `payload["error"].is_null()`。
- **可触发假阳性**：返回 `{"jsonrpc":"2.0","id":3,"result":{"structuredContent":null}}` 时并没有读到应有对象，当前成功断言仍通过；返回 HTTP 503 的普通 `{"code":"SHUTTING_DOWN",...}` transport envelope 时没有 error 键，也会通过。首个 Wide 拒绝断言能证明那次请求收到正确错误，不能替后续三次调用证明成功。
- **最小修复**：mcp_get 校验 HTTP status、`jsonrpc` 和对应请求 id；对三次预期成功的调用明确检查 `result.structuredContent.properties.value` 分别为 `stable`、`local`、`original`，并继续拒绝 JSON-RPC error。既有 `trino_http_it.rs:121-157` 已有 JSON-RPC envelope 检查，可沿用该模式，不需要新框架。
- **边界**：这是断言确实接受错误输出的问题，不是已证明生产服务会输出上述错误响应。未修改测试，已交主会话转原 owner。

## 已确认的检查项

| 检查项 | 静态结论 |
| --- | --- |
| 生产入口与周期 | `setup:295-308` 构造真实 ServerHost 并调用 serve；wrapper 仅委托真实 Trino execute/describe/status。用例不调用 check_drift、不直接安装报告、不缩短生产 interval。首次检测后，同版本 DROP/恢复不会制造新 key 唤醒，依赖生产周期。75s 是观察上限，不是精确 30s 周期的动态证明。 |
| 同表两个逻辑 target | `ontology:103-110` 将 Wide.value 绑定 drift_value、Narrow.value 绑定 stable_value，物理表完全相同；只 DROP drift_value。初始及恢复时 REST 明确核验三个对象的业务值，degraded 时 Narrow/Local REST 为 200。MCP 成功部分仍需 RT01 修复。 |
| SQL 计数隔离 | `MetadataBarrier::execute:48-51` 才增加 business_calls；describe_tables/status 直接调用 inner，相应 metadata/SELECT 1 不经过此计数器。Wide MCP 拒绝前后比较计数可检测该请求发送业务 SQL，probe 不会污染计数。不是全 suite 的 SQL 数量断言。 |
| PG 资源和权限范围 | schema 全部由本次小写 ULID 派生，source DDL 只作用于该 schema 的 source_rows，edit/sys schema 也独立；DROP 仅枚举这三个名字。未执行 GRANT、ALTER ROLE 或修改默认权限。仓库 CI demo catalog 使用 PostgreSQL postgres 连接，不缺少该环境的额外 grant；其他身份部署需自行满足已声明的测试环境条件。 |
| MCP 协议与拒绝审计 | get_object 与 mcp_catalog 的 GET_OBJECT_TOOL 一致，objectType/pk 形状正确，使用既有 2025-03-26 initialize、initialized、session 流程。Wide 拒绝检查稳定 code，随后在运行期间查询 PG durable audit，按 request_id 要求唯一 `(version, source_error)`，没有依赖 shutdown 后才落盘来证明拒绝。 |
| 双 store reload barrier | 包装器先读真实 metadata 再暂停返回；hold_next=2 分别拦旧轮和新轮。第二 store reload 有 5s 上限，能暴露 metadata await 持有定义锁；新版本先后两次必须 Pending，新轮释放后必须得到 SchemaDrift，audit def_version 必须为 current。整个断言过程未发送 shutdown，关闭错误无法替这些精确码和版本断言制造成功。 |
| 清理顺序 | 正常/断言 panic 路径中 Running Drop 发送 stop；外层释放测试 barrier、join host、关闭两个 audit、drain engine，之后才 DROP ULID schema。host 返回错误或超过 12s 会设置 shutdown_ok=false，suite 最终不能绿色；超时 fallback 的 abort 只证明 host task 被取消，不是独立 monitor 全部退出的动态证据。该 suite 不应承担已有独立 cancellation cleanup 用例的证明责任。 |

## 范围

读取并比较 `support/real.rs`、`support/pg_real.rs`、`trino_http_it.rs`、`health_trino_it.rs`、MCP constants 和 CI demo catalog 配置。两次核对目标测试/support 哈希均未变化。没有运行 Cargo、服务、SQL、权限变更或修改测试，只按 worker 文档所有权写本报告。core 后续生产变更和真实服务运行结果由相应 owner 提供，不由本报告推定。

首轮建议已实施，具体复签如下。

## RT01 独立复签：clean

复签时间为 2026-11-03T11:35:16Z。读取当前完整 `source_drift_trino_it.rs` 并两次核对 Git blob `c14cb6a04626323259e3f5ddf44f26ab6fd18e8b`；`support/real.rs` 仍为 `0c1776f1dba49d2810dd2a55dc705f9aeee1bc16`。本次没有运行 Cargo、SQL 或服务，没有修改测试代码。

- `mcp_get` 第 512-540 行明确要求 HTTP200、`jsonrpc=2.0`、与本次请求相同的 id，且 result/error 必须恰好存在一个。error 必须是对象并有整数 code；成功 result 必须是对象。因此 HTTP503 普通错误、缺失 JSON-RPC envelope、错误 id 或空 result 不再被成功路径接纳。
- `assert_mcp_object` 第 477-480 行要求不存在 error，并验证实际 structuredContent；`assert_object` 第 463-468 行要求对象形状、准确 objectType、pk、properties.id 和 properties.value。因此 `structuredContent:null`、空对象、错误目标/主键或错误业务值都失败。
- Narrow/Local 的 degraded 期间调用分别要求 `stable` / `local`；恢复 Wide 要求 `original`。REST 的初始、degraded、恢复及 reload 后可用路径均使用 `assert_rest_object`，检查 HTTP200 和同一组完整对象和值断言。原三次 MCP 成功判定假阳性已消除。
- Wide MCP 拒绝仍要求 `SOURCE_SCHEMA_DRIFT`，并新增明确 `apiName=get_object`；运行期间通过 request_id 查询 PG，要求唯一 `(def_version, source_error)`，随后核对 business_calls 未增加。metadata probe 与 raw status 不经过该业务 SQL 计数器。REST drift 拒绝保留 503/code、响应 header ID 与 body ID 对账及 durable audit 检查；本次未将其描述为完整逐字段错误 schema 测试。
- 关闭、schema 清理及双 store barrier 的执行结构未因成功断言补强而变化。观察与审计断言仍发生在 shutdown 前，清理不能补救空结果或错误响应。

**当前结果：** RT01 关闭，无新增 finding。这是测试断言可靠性的静态复签，不是两个 ignored 真实服务用例已运行通过的结论。transport owner 正在执行的真实服务结果应另行读取、记录；本报告的 `real_services_verified: false` 保持不变。

## CR01 冻结内容核对

- `ontoforge-action/src/source.rs` 仍为 `33af0bc044235488f725d4835625388137583c20`，与此前 clean 复签完全相同，未重新扩大生产逻辑审查。
- 当前 `ontoforge-server/tests/source_drift.rs` 为 `5bcedd1432891b797e4eedffb02b78d975681f5c`。已核对共预算用例新增终态断言：20 秒预算耗尽后、shutdown 之前读取 health 和 Flight 响应，再要求 health HTTP503、原 `status=unhealthy/engineStatus=available` 字段及 `SOURCE_CHECK_UNAVAILABLE`。未把 shutdown 结果当终态证据。
- 已读取 `target/phase7-drift-frozen-tests.log` 末段，实际显示 19 passed / 0 failed；该日志归执行 owner，本 reviewer 未运行测试。新增断言没有削弱 CR01 的授权矩阵或移除事务内行列权限检查。

本次仅更新本报告，保留原始发现与他人实现。未提交；RF01 整体和 Phase 7 验收仍 pending。
