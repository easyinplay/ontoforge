---
phase: 07-rest-mcp-shell
reviewed: 2026-11-03T12:44:54Z
scope: RT03-test-only
status: clean
confidence: 99
files_reviewed: 1
files_reviewed_list:
  - crates/ontoforge/tests/cli_l2_identity.rs
source_blobs:
  crates/ontoforge/tests/cli_l2_identity.rs: 6b90a443d2535402a40a8e1a430bf0ee40d1fdbd
findings:
  critical: 0
  warning: 0
  info: 0
  total: 0
tests_executed_by_reviewer: false
execution_log: target/phase7-l2-monitor-test-fix-final.log
phase_acceptance: pending
---

# RT03 L2 monitor 请求分类测试复签

## Narrative Findings (AI reviewer)

NO FINDINGS

## 静态结论

只读审查当前 `cli_l2_identity.rs` 相对 HEAD 的测试增量、周边六工具测试及 `target/phase7-claude-l2-fixture-requests-review.json`。初稿 blob `2a9367bfca530f0cc0bd549272663f66ef0d5d99` 经机械 lint 清理和重复断言提取后，最终 blob 为 `6b90a443d2535402a40a8e1a430bf0ee40d1fdbd`，已独立重读增量并核对。

| 关注点 | 证据与结论 |
| --- | --- |
| 默认计入 | `non_monitor_requests` 仅对 `is_source_monitor == true` 取反过滤，未知请求默认保留；不存在按身份或 SQL 子串统一豁免。 |
| 完整精确条件 | 第 96-114 行同时要求 POST、wiremock 记录的 `http://localhost/v1/statement` 完整 URL、Host 等于当前 MockServer origin authority、单一 `x-trino-user=ontology`、单一 `x-trino-trace-token=source-drift`，以及完整 metadata SQL 字节匹配。Host/user/trace 都通过整个 `get_all` 序列比较，缺失、不同值及重复值均不接受。 |
| 固定 SQL | 第 74-94 行的期望 SQL固定 demo.information_schema.columns、aviation schema 及 aircraft/flight_crew/flights/maintenance_records/pilots 五张表，含完整投影、条件和排序。不会因包含 information_schema 就排除另一条查询。期望字符串独立写在测试中，没有调用被测生成器计算自身答案。 |
| 逐条件反例 | classifier 自测保留一个完整合法 probe，再分别破坏 origin、method、URL path/query/host；对 Host/user/trace 分别检查缺失、错误、重复；对 SQL 检查其他 schema/table、不同 metadata 查询、业务 SQL、未知/空 body、无效 UTF-8。正确 ontology/trace 等 headers 搭配业务 SQL 时明确不排除。 |
| startup monitor 确实存在 | `same_host_l2_unverified_failed_and_expired_proofs_reject_all_six_tools` 首窗口明确调用 `wait_for_source_monitor`。该 helper 最多等待 3 秒，未观察到完整精确 probe 时 assert 失败，并输出实际请求；不以超时跳过或视作成功。 |
| 六工具负向保持 | 新 `assert_six_tools_rejected` 在三个调用位置共用：unproved 配置组合、health-only 首窗口、NotVerified/AllowedProbeFailed/Expired 窗口。每次仍实际调用六工具，逐个要求 JSON-RPC error 对象，然后要求窗口前后非 monitor 请求数量相同。任何身份的业务 SQL或其他未知请求都会增加该数量并失败。 |
| 正向业务身份保持 | 证明有效后的六工具调用仍要求无 error；过滤后的业务请求至少增加六条，并逐条要求 `x-trino-user=l2`。仅精确 monitor 被移除，ontology 身份发出的业务 SQL仍会保留并使身份断言失败。 |
| lint 清理没有削弱检查 | `HeaderValue::as_bytes` 替代等价 closure、显式 HeaderMap 默认值，以及提取三个重复断言块，均保持原匹配和计数语义。此次没有新增 lint allow。 |

## 已读取运行证据

- 初稿 `target/phase7-l2-monitor-test-fix.log` 实际 11 passed / 0 failed / 0 ignored，61.61 秒。随后原 `phase7-l2-monitor-clippy.log` 有三项机械 lint 错误，未将该次检查记为通过。
- 最终 `target/phase7-l2-monitor-clippy-final.log` 显示目标检查 `Finished`，1.43 秒，无诊断；主会话/owner 记录 exit0。
- 最终 `target/phase7-l2-monitor-test-fix-final.log` 实际为 `11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 61.71s`，包含 classifier 自测以及最后的同 host proof-expiry 用例。没有把中途 10 项通过或“running for over 60 seconds”当成整套成功。

结论为 clean，置信度 99%。本补丁只修测试的请求观测口径，不改生产 source monitor、L2 admission、SQL 或身份策略。以上测试由 owner 执行，本 reviewer 仅读取日志和核对代码/blob，未运行 Cargo、修改源码或提交；不将本地最终通过外推为修复后的 Ubuntu/Windows CI 已通过。
