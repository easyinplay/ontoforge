---
phase: 07-rest-mcp-shell
reviewed: 2026-11-03T12:00:51Z
depth: deep
scope: DC01-static-test-review
status: clean
files_reviewed: 1
files_reviewed_list:
  - crates/ontoforge-server/tests/source_drift_cleanup.rs
source_blobs:
  crates/ontoforge-server/tests/source_drift_cleanup.rs: 0dcf954763cbec905d795714e7cc8aa716dbf30b
findings:
  critical: 0
  warning: 0
  info: 0
  total: 0
tests_executed: false
execution_evidence_reviewed: true
execution_evidence_reviewed_at: 2026-11-03T12:13:22Z
execution_log: target/phase7-drift-cleanup-budget-final.log
observed_test_result: "2 passed; 0 failed; 0 ignored; finished in 0.18s"
real_services_verified: false
phase_acceptance: pending
---

# DC01 metadata DELETE 清理测试静态复签

## Narrative Findings (AI reviewer)

NO FINDINGS

## 审查范围与结论

读取完整 `source_drift_cleanup.rs`、已读 `07-DRIFT-FIX-DESIGN.md` 的关闭合同及 `target/phase7-claude-drift-cancellation-review.json`，并独立追踪当前生产 `ServerHost::initialize`、source monitor、`ServerLifecycle`、`TrinoEngine::describe_tables` 与 `QueryGuard/CancellationTasks`。两次核对测试 blob 均为 `0dcf954763cbec905d795714e7cc8aa716dbf30b`。

当前测试结构可以检验 Host → source monitor → describe_tables → raw engine tracker → HTTP DELETE drain。未发现让缺失 DELETE、错误 URI、host 提前结束或清理错误被吞掉仍满足全部断言的确定路径。本结论只评价静态测试设计；未运行 Cargo、rustfmt、测试或服务，不替代后续实际编译与 GREEN。

## 关键证据

| 检查点 | 源码证据与判断 |
| --- | --- |
| 真实生产入口 | `run_shutdown` 构造真实 `TrinoEngine` 和 `ServerHost`，通过 `host.serve` 启动。没有直接调用 `check_drift`、手工安装报告或直接调用 cancellation tracker。 |
| metadata 已在途 | POST 上游返回 query id 和同源 `nextUri`；GET handler 第 46-48 行记录 URI、通知 barrier 后保持 pending。host 测试在收到 GET barrier 后才发送 stop，并检查 GET 精确为 `[QUERY_PATH]`、仅一个 POST、SQL 含 `information_schema` 与目标表名。因而不能用尚未启动的 probe 或任意业务查询冒充 metadata 在途。 |
| 等待 DELETE 完成 | DELETE handler 第 55-58 行先记录 URI并通知，再等待零初始 permit 的 Semaphore。收到 DELETE barrier 后，第 164-168 行明确要求 host 在 100 ms 窗口内不能结束；随后才释放 DELETE。没有将请求开始等同于清理完成。 |
| 正常结果 | 204 分支要求 `run_shutdown(...).await.unwrap()`，错误退出不能算成功；释放 barrier 后的 host join 有 WAIT 上限并拒绝 panic。 |
| 失败传播 | 500 分支走同一完整 GET/DELETE 流程，再明确匹配 `Err(HostError::Shutdown)`，并非任意非成功都接受。生产 `send_delete` 只将成功或 404 视为成功，500 设置 tracker 的 sticky failure；生命周期 drain 将其传播到 Shutdown。 |
| 次数、URI、audit | host 结束后第 175 行要求 DELETE 列表恰为 `[QUERY_PATH]`，捕获缺失、重复或错误路径；第 176 行要求 audit unavailable。此前 GET 在途时已确认 audit available，因此不是未成功打开 audit 的空断言。该 fixture 无业务读取，不声称验证了额外 read audit 行。 |
| 资源所有权 | 测试中的 serving 与上游 listener task 都由 `AbortOnDropHandle` 持有；正常路径等待 host 真实结束后才离开 fixture。SQLite、HTTP port 和临时目录每次独立，两个用例不会共享同一查询记录。失败路径中的 abort 是测试清理兜底，不能绕过断言或变成成功。 |

## 明确边界

- 上游是受控 axum HTTP 服务，TrinoEngine 和 ServerHost 是真实生产实现；不是实际 Trino coordinator，也不冒称真实 Trino/PG 验收。
- 此用例在首次 metadata probe 尚未完成、HTTP host 尚未准入时触发 shutdown，验证启动过程的取消与共用生命周期 drain。它不单独证明周期 probe、已接受 Action 等待或多次 nextUri 变化的全部组合。
- 当前 barrier 前后分别有 WAIT 上限及一个 100 ms 未完成窗口。测试会检测挂起，但没有用单一 stopwatch 精确断言从 stop 开始的总耗时 ≤10 秒；原 10 秒 deadline 来自已追踪的生产生命周期。不能将测试设计描述为已经实测精确的 10 秒总上界。
- 500 分支验证失败分类和 audit 关闭；静态审查时未有运行结果，现已读取下节执行 owner 提供的两项 GREEN。该执行不改变受控 HTTP 上游与精确 10 秒强制上界的覆盖边界。
- CR01/RT01 不在本次范围内，未重新审查或改动它们的报告。

只新增本报告，未修改源码、运行 Cargo 或提交；后续若测试 blob 改变，应核对差异后再引用本结论。

## 最终运行证据补签

2026-11-03T12:13:22Z 已实际读取 `target/phase7-drift-cleanup-budget-final.log`。其中 `source_drift_cleanup` 段明确执行下列两个用例，均为 ok：

- `failed_metadata_delete_returns_shutdown_error_and_closes_audit`
- `shutdown_waits_for_inflight_metadata_delete_and_closes_audit`

日志结果为 `2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s`。再次读取测试 Git blob，仍为 `0dcf954763cbec905d795714e7cc8aa716dbf30b`，与本报告静态复签对象相同。

动态测试由执行 owner 运行，本 reviewer 只读取日志与核对 blob，未亲自运行 Cargo。同一日志中其他套件的结果不纳入本次 DC01 补签。0.18 秒是两个用例的实际合计运行时间，不等于对所有调度条件下 10 秒强制上界的单独时钟证明，也不代表真实 Trino coordinator 服务验证。DC01 结论维持 clean，Phase 7 整体验收仍由主会话处理。
