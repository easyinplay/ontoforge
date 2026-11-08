---
phase: 07-rest-mcp-shell
reviewed: 2026-11-03T11:15:00Z
depth: deep
base: 4d203bfab24293de28375524c605cd41955112df
scope: working-tree-RF01
status: issues_found
recheck_status: clean
rechecked: 2026-11-03T11:25:37Z
cr01_status: closed
phase_acceptance: pending
files_reviewed: 25
files_reviewed_list:
  - crates/ontoforge-engine/Cargo.toml
  - crates/ontoforge-engine/src/lib.rs
  - crates/ontoforge-engine/src/bind_check.rs
  - crates/ontoforge-engine/tests/drift_budget.rs
  - crates/ontoforge-query/src/lib.rs
  - crates/ontoforge-query/src/execute.rs
  - crates/ontoforge-query/src/aggregate.rs
  - crates/ontoforge-query/src/link.rs
  - crates/ontoforge-action/src/lib.rs
  - crates/ontoforge-action/src/source.rs
  - crates/ontoforge-server/Cargo.toml
  - crates/ontoforge-server/src/drift.rs
  - crates/ontoforge-server/src/state.rs
  - crates/ontoforge-server/src/context.rs
  - crates/ontoforge-server/src/lib.rs
  - crates/ontoforge-server/src/lifecycle.rs
  - crates/ontoforge-server/src/error.rs
  - crates/ontoforge-server/src/health.rs
  - crates/ontoforge-server/src/rest.rs
  - crates/ontoforge-server/src/mcp.rs
  - crates/ontoforge-server/tests/source_drift.rs
  - crates/ontoforge-server/tests/unit/drift.test.rs
  - crates/ontoforge-server/tests/source_drift_trino_it.rs
  - crates/ontoforge/src/serve.rs
  - crates/ontoforge/tests/cli_shutdown.rs
findings:
  critical: 1
  warning: 0
  info: 0
  total: 1
---

# RF-01 合同与 source 全出口独立复签

## 范围

必读并采用 ADR-0030、ADR-0031、`07-DRIFT-FIX-DESIGN.md`、`07-DRIFT-ENG-REVIEW.md`。审查从 `4d203bf` 到当前工作树的 drift 变化，包含新文件；依赖追踪另外读取 Action criteria、scope 编译和覆盖检查、Query set/L2、既有 fixture。未运行 Cargo、测试、服务器或修改源码。RF-05、GT01、WR01 已关闭问题不重复计数。

## Narrative Findings (AI reviewer)

### CR-01: BLOCKER：Action 在引用类型授权前暴露 drift 状态

**File:** `D:/DataSpellProjects/ontoforge/crates/ontoforge-action/src/source.rs:115-123`

**Issue:** 合法 scope 可以授予一个无 submission criteria 的 Action，却不授予其 ObjectReference 指向的类型。`execute_inner` 只在 `lib.rs:298-305` 检查 Action grant；`prepare_edits` 随后于 `lib.rs:506-509` 调用 `source::prepare`。后者对缺少 type grant 的引用没有拒绝：`required_properties` 仅在 grant 存在时添加 scope 属性，不存在则继续。无完整 patch 的 source 引用因此先进入 `read_pk_source` 的新 drift hook，`source.rs:121` 将其精确状态向外传播。

真正的对象授权位于稍后的 `commit_action_guarded` / `criteria::authorize`（`lib.rs:386-388`），其中 `criteria.rs:58` 才要求类型 grant。结果是同一未授权调用在正常 source 下最终返回 NOT_FOUND，在 drift / unreadable / Pending / CheckFailed 下提前得到对应稳定状态码，泄露不可访问目标的运行状态。REST 和 MCP 的正式提交及 validateOnly 都使用该路径。scope 编译不会阻止该配置：`auth/src/scope/compile.rs:192-219` 只验证 Action 存在并调用 criteria coverage；`coverage.rs:37-58` 只遍历实际 submission criteria，对无 criteria 的 Action 不要求引用类型 grant。

**Fix:** 在 source 准备前对所有 ObjectReference 做可静态判断的类型授权，缺失 `scope.types[type_name]` 时沿用既有 `NotFound` / `ScopeDenied` 语义，且不得进入 drift hook 或 source SQL；不要试图用 source 读取完成之前无法获知的完整行可见性代替这项前置类型检查。补充真实 REST/MCP 正式与 validateOnly 用例：授予无 criteria 的 Action、不给引用类型 grant，分别在正常及 degraded source 下验证同一安全拒绝、零业务 SQL、无业务写入和无 drift 状态泄露。

**证据边界:** 本 finding 由当前源码调用链独立确认，尚未执行本场景的动态复现。它指出新 gate 错误类别在授权前暴露的问题，不声称未授权业务写入已经成功。

## Source 出口与错误传播核对

| 路径 | 当前实现及静态结论 | 已有测试证据边界 |
| --- | --- | --- |
| PK / history PK 前读 | `query/lib.rs` 在 `read_pk_source_with_audit` 执行前检查实际 binding target；类型 grant 已检查，完整 patch 不取 source | `startup_drift_rejects_source_before_sql_and_preserves_audit_id` 验证 503、零业务 SQL、唯一 audit ID 及 source_error；history 复用同链，本轮未新增其 drift 动态用例 |
| list 首批 | `execute_list_internal` 在首个 source future 创建前检查 ObjectType；原 WorkBudget/audit 保持 | list 拒绝测试检查 503/稳定码/零 SQL |
| list 后续与 backfill | `read_source_select` 每次检查对应 ObjectType | `later_list_page_and_backfill_recheck_the_logical_target` 两分支通过可控 inner hook 在第二次拒绝，验证两个 hook 与一次业务 SQL；它不宣称在途 immutable view 随周期变化 |
| aggregate | partial 直接发送前检查 binding；delta backfill、fallback、min/max 复用已检查的 source select/list 路径；SourceTarget 保留为 source_error | aggregate 拒绝测试检查响应码与零 SQL；未逐个动态覆盖所有策略 |
| M:N / searchAround | link 每次 source 页检查 LinkType，包含首批并发 source/edit 读取；base/target 对象路径复用列表 hook | searchAround 测试验证 base PK 可发送一次，link gate 拒绝后 join SQL 为零 |
| Action 正式与 validateOnly | REST 与 MCP 均构造携固定 view 的 ScopedEngine；source error 类型不被压成 EngineUnavailable，source 独立完整 patch 可通过 | REST 两模式有拒绝与无新增业务 commit断言，完整 patch 有读取和正式 Action 成功断言；CR-01 所述缺失 type grant 情况未覆盖 |
| REST 错误 | Query/Aggregate/Action SourceTarget 全部映射 503 和四种稳定码；受审计 Query 通过 observer 保留原 ID | PK ID 精确对账已运行；list/aggregate 当前测试未逐项断言 audit ID |
| MCP 错误 | read/aggregate/action 现有安全 JSON-RPC error.data 保留 code/apiName/request_id；无需改变 HTTP transport | 新真实服务 suite 只有 get_object drift 动态场景，尚未运行；本轮未见 MCP Action 正式/validateOnly drift 用例 |

这些静态接线不能替代尚缺的行为验收，也不抵消 CR-01。

## Owner、预算与 health 核对

- `serve_with_limits` 在生产 listener seam 完成首次有界检查后才接受 HTTP；单 monitor 持有单 slot，key 精确匹配 `(def_version, effective_profile)`。短锁不包 storage/network await，旧请求捕获 Arc view，已观察到更高版本时拒绝旧轮发布。
- raw host engine 执行 metadata probe；30 秒 Skip interval、20 秒整轮 timeout、Notify 合并唤醒。公共 `describe_bindings` 仍每次使用原 QueryContext timeout，只有 drift 私有路径递减共用预算；零预算不发 probe。
- 无 source 直接 NotApplicable，不发 metadata。完整报告有任意数量 degraded target 时 health 为 200 degraded；raw engine 失败、Pending、CheckFailed/EngineFailed 为 503。`engineStatus` 仍来自 raw status；匿名 body 沿用 status/engineStatus/可选 l2，不增加目标名、版本或计数。
- task 异常退出设置 stopped，后续任何版本得到 CheckFailed；重复 owner 拒绝；shutdown 将 drift 和 L2 monitor 纳入既有 8 秒 monitor 阶段及 10 秒总截止，随后 drain raw engine cancellation。CLI 在启动 engine/monitor 前绑定端口，端口冲突不会遗留 probe owner。

## 已读取的动态证据

- `target/phase7-drift-exits.log`：早期 source suite 12/12 GREEN。
- `target/phase7-drift-final-focused.log`：后续 CLI shutdown 9/9、engine drift budget 2/2、source drift 15/15 GREEN。panic 用例包含预期的 `controlled metadata panic`，其测试结果为 ok，不将该受控 panic 当失败。
- `target/phase7-drift-owner-green.log`：exact-key/inflight view 与 owner-exit 两项单元测试通过。
- `target/phase7-drift-budget-red.log`：原 public timeout 被递减和零预算仍发 probe，两项实际断言失败。
- `target/phase7-drift-cli-red.log`：端口占用时实际发生一次 probe，测试断言失败；当前 focused GREEN 包含该用例。
- `target/phase7-drift-lifecycle.log` 中旧 full-patch fixture 格式错误为准备阶段失败，未计作功能 RED；后续 12/15 项 GREEN 已包含修正后的用例。
- `target/phase7-drift-clippy.log` 为早期检查完成记录，不能代替后续完整修改的最终 lint；`phase7-drift-fmt.log` 为空，不能独立证明 exit code。

**待验证：** 两个 `source_drift_trino_it.rs` 测试仍为 ignored/未运行，不能写成真实 Trino/PG 通过。其源码包含同物理表双 logical target 隔离、ALTER/恢复、双 PostgreSQL store reload barrier 与 Pending、原 ID/版本 source_error；目前仅静态审阅。真实服务 DELETE cleanup 也不由这些结果证明。仍需解决 CR-01 并补对应行为证据后重新复签，RF-01 整体验收未完成。

## 审查读取时的 Git blob

```text
crates/ontoforge-engine/Cargo.toml 47f061e641f35e54cea5ee1af3695b939379c085
crates/ontoforge-engine/src/lib.rs f3e144e92dfa347a54a3d8a8ffdf2d478d931999
crates/ontoforge-engine/src/bind_check.rs 355e023f16c89a06f38382f8354c79ac18f796fd
crates/ontoforge-engine/tests/drift_budget.rs f347b661d4df651a5e1e8b488b394498c2f8fee1
crates/ontoforge-query/src/lib.rs 94c8aa38ac79db1833ff5e9ce3f2d7ac0c79879a
crates/ontoforge-query/src/execute.rs 9bf78c07f5e50cc11c77e8e07fe2c6e13ac32fb4
crates/ontoforge-query/src/aggregate.rs 174483df7809faeff921c09626b396a0ac489c96
crates/ontoforge-query/src/link.rs bec626bb411c9d0bb467a82d382a9b01ab8e325f
crates/ontoforge-action/src/lib.rs 68218b54e954014e998c17081a2bf427502c5de2
crates/ontoforge-action/src/source.rs a41954b05581938016668acd2ed9a6d82321c1b3
crates/ontoforge-server/Cargo.toml 97c68cf3cc7350adabe76d33f1df09acd85abc39
crates/ontoforge-server/src/drift.rs e7eacbb94dadf727e3d0bd1ad9e8a9c60c74b568
crates/ontoforge-server/src/state.rs 50ff35009b09f204fcabb2b51d61f11b71c4310e
crates/ontoforge-server/src/context.rs bfc178c59d5e8de31a0d7f36197bcc5b31be82ec
crates/ontoforge-server/src/lib.rs 38005f3943f73da00026c8fa9ff3c4b08f12c7d8
crates/ontoforge-server/src/lifecycle.rs 17b7f4e8f794459661e73b19f8d5833797734dc1
crates/ontoforge-server/src/error.rs dd1572eed41f6e96f76f796e990323debbff6e62
crates/ontoforge-server/src/health.rs da21a2c4b3c10fdfb0c2f811c891354ad0f9e18f
crates/ontoforge-server/src/rest.rs 4ba27e85a1e512c7adb69aa1d190682e36fde939
crates/ontoforge-server/src/mcp.rs aed54c866f87162a95e76c9553da9b63d1d859e2
crates/ontoforge-server/tests/source_drift.rs a14ff0301467da322affe11e9ac3cb5a067fe7cf
crates/ontoforge-server/tests/unit/drift.test.rs ae1e75f1a0690fcd3513d5e71c5674da8a295a53
crates/ontoforge-server/tests/source_drift_trino_it.rs 7765177d76a68e0515f577780e8e9b5e48e8e4fa
crates/ontoforge/src/serve.rs 439ce1ce1eafe6cbe638f92fb2725925c51d7616
crates/ontoforge/tests/cli_shutdown.rs 7c572132c55ca48e9a43f858f96ab2d9ba4dced2
```

后续 lint 或修复改变上述 blob 时，应审阅差异并追加复签，不将本次结果自动套用到新内容。仅写本报告，未提交。

写报告后再次核对 25 个 blob：仅 `server/src/lib.rs` 发生变化，已重读其完整增量；limits 验证恢复为启动 monitor 前直接返回错误，未改变本次 source 接线或 CR-01 判断。上表已更新至新 blob，其余 24 项当次相符。

## CR-01 修复独立复签

**结论：** CR-01 已关闭，本 finding 的复签为 clean；RF-01 整体验收仍为 pending。上方 `status: issues_found` 和 Critical 计数保留首轮发现，当前关闭状态由 `cr01_status: closed` / `recheck_status: clean` 及本节表达。

**复签 blob：**

```text
crates/ontoforge-action/src/source.rs 33af0bc044235488f725d4835625388137583c20
crates/ontoforge-server/tests/source_drift.rs f0df3b266260e301eee3186e139fc10341d8f40d
```

source 准备循环现在先取已解析为 canonical `ObjectTypeName` 的引用类型，复用原 `denied` 分支，并在 `read_object_edit`、`read_pk_source` 和 drift hook 前检查 `scope.types.contains_key(type_name)`。缺少类型 grant 立即 `NotFound` / `ScopeDenied`；它不把存在 edit key、行可见或列可写等条件替换成这一静态检查。最终 `commit_action_guarded` 中 `version::authorize` 与 `criteria::authorize` 保持原状，完整 patch 的合法 Action 仍经过既有权限验证。

**已核对行为证据：**

- `target/phase7-drift-action-auth-red.log` 是编译成功后的真实行为 RED。测试已收集 REST/MCP 两模式响应，但在首个 REST 漂移态断言处失败：期望 404，实际 503 / `SOURCE_SCHEMA_DRIFT`。该日志不证明每个分支都分别独立失败。
- `target/phase7-drift-action-auth-green.log` 为 19 passed / 0 failed。新增 `action_type_authorization_precedes_source_drift_for_rest_and_mcp` 清除 Action submission criteria，并通过真实 scope 编译/应用建立只有 Action grant 的合法配置；健康/漂移两种 metadata 情况 × REST/MCP × 正式/validateOnly 共八个拒绝路径均执行。REST 为 404 NOT_FOUND，MCP 为 HTTP200 + JSON-RPC NOT_FOUND；source 业务 SQL 次数为零。
- `mcp_authorized_action_preserves_drift_code_in_both_modes_without_side_effects` 保留有效类型 grant，验证两种 MCP Action 模式仍返回 HTTP200 + `SOURCE_SCHEMA_DRIFT`、正确 apiName 和非空 request_id；source 业务 SQL 为零。
- 两项新增测试在停止 host 后核对 edit log 仍只有原 fixture commit，read audit 仍为零；未为 Action 发明额外 read_audit 合同。前文“MCP Action drift 未覆盖”的缺口由这两项测试关闭。
- 当前 `source_drift.rs` 在上述 GREEN 后进行了格式展开；已重读新增测试和 fixture，分支、断言及调用顺序未改变。记录当前复签 blob，未将 19/19 日志冒称为对后续全部文件修改的最终完整验收。

已阅读 Claude Code 同方向意见，但关闭依据是本 reviewer 对实际源码与测试日志的独立核对。未运行 Cargo、未修改实现。相邻 lint、真实 Trino/PG 与最终所有 blob 的冻结核对仍需主会话完成；不因本 finding 关闭而宣称 RF-01 或 Phase 7 完成。
