# RT03 L2 monitor 测试观测修正

## 范围与计划

- 所有权：`crates/ontoforge/tests/cli_l2_identity.rs` 与本记录；不修改生产 API、L2 admission 策略、共享计划或 STATE。
- 已读取 `planning-with-files`，沿用 Phase 7 主计划；本 worker 仅维护独立证据文件。
- 固定 aviation 的五表 metadata SQL，精确验证 POST、当前 mock Host、statement endpoint、ontology user、source-drift trace；其他请求默认计入。
- 统一六工具负向窗口和正向身份断言，首次窗口显式确认启动 monitor 已发生。
- classifier 自测逐项破坏 method、origin、endpoint、user、trace、SQL，覆盖缺失、未知及畸形输入。
- 完成标准：现有 CI RED 对应 suite GREEN、目标 Clippy 和 fmt 通过，证据落本文件后交回主会话。

## Findings

- CI `41634382039` Ubuntu 日志 `target/phase7-ci-41634382039-ubuntu.log`：`cli_l2_identity.rs:409` 的 `is_empty()` 失败；此前六工具 JSON-RPC error 断言已通过。9 passed / 1 failed，exit 101。
- 根因是合法 source drift metadata 请求进入全局请求集；生产 `drift.rs` 使用 `source-drift` trace，`bind_check.rs` 按 catalog 分组且对 TableRef 排序去重。
- aviation 固定表为 `demo.aviation.{aircraft,flight_crew,flights,maintenance_records,pilots}`。
- wiremock 0.6.5 的 origin-form 请求使用 `http://localhost` 构造 Request.url；真实目标由 Host header 保留。因此 classifier 同时精确核对该 URL 与当前 mock 的 Host。
- 主会话与 Claude Code 已同意测试观测合同，见 `target/phase7-claude-l2-fixture-requests-review.json`。
- 已读取 `superpowers:test-driven-development`：本次只有测试修正，沿用真实 CI RED，不人为修改生产构造 RED。跳过重复 brainstorming，因为根因与修复合同均已锁定。
- Warp/ralph-loop 未暴露，使用 PowerShell 实际执行并按完成标准循环，不声称调用不可用工具。

## Progress

- 已核对源码、fixture、SQL printer 和 CI RED；实现前记录已落盘。
- 已实现精确观测 helper 与 classifier 自测；缺失或重复身份/Host/trace header 也默认计入，业务 SQL 在 ontology、l2、unknown 身份均不得豁免。
- `rustfmt --edition 2024 crates/ontoforge/tests/cli_l2_identity.rs`、目标 `git diff --check` exit 0。
- 已取得唯一 Cargo 槽，`CARGO_BUILD_JOBS=2` 运行完整 `cli_l2_identity` suite。

## Verification

- RED：既有 Ubuntu CI `41634382039`，9 passed / 1 failed，失败点为首次六工具之后的全局 `is_empty()`。
- GREEN：`cargo test -p ontoforge --test cli_l2_identity --locked -- --nocapture`，exit 0，11 passed / 0 failed / 0 ignored，测试耗时 61.61s。日志 `target/phase7-l2-monitor-test-fix.log`。
- 精确 startup monitor 匹配已在真实 mock 请求中通过；白名单 SQL 与当前 aviation metadata 输出一致。未从观察结果动态学习白名单。
- 原 case 的 NotVerified、AllowedProbeFailed、恢复证明及 61s 后 Expired 路径全部通过；六工具错误合同与正向身份检查均保留。
- 首次 `cargo clippy -p ontoforge --test cli_l2_identity --locked -- -D warnings` exit 101，日志 `target/phase7-l2-monitor-clippy.log`。仅 3 个测试 lint：redundant_closure_for_method_calls、default_trait_access、same_host 函数 104/100 行；已报告主会话。
- 后续最小清理：使用显式 HeaderValue 方法与 HeaderMap 类型；抽取重复六工具拒绝及非 monitor 计数断言，保持既有安全合同。清理后重跑 suite、Clippy 与 fmt。
- 最终目标 Clippy exit 0，日志 `target/phase7-l2-monitor-clippy-final.log`；未新增 lint allow。
- 最终源码 blob 为 `6b90a443d2535402a40a8e1a430bf0ee40d1fdbd`；`rustfmt --edition 2024 --check crates/ontoforge/tests/cli_l2_identity.rs` 与目标 `git diff --check` exit 0。
- 最终 suite exit 0，11 passed / 0 failed / 0 ignored，测试耗时 61.71s，日志 `target/phase7-l2-monitor-test-fix-final.log`。`assert_six_tools_rejected` 为每次调用独立采集 baseline、验证全部六工具 error、比较非 monitor 请求数，未弱化任何拒绝窗口。

## Handoff

- RT03 代码与定向验收完成，Cargo 槽已释放，等待主会话独立复签与整合。
- 本 worker 未 commit/push，未改生产实现或共享状态文件。
- 该 GREEN 是本地 Windows 定向证据；修复后 Ubuntu CI 的新运行仍由主会话安排，不以本地通过冒充远程通过。
