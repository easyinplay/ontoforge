# MCP 实际执行准入修复记录

合同：`07-MCP-EXECUTION-ADMISSION-FIX.md`。2026-11-03启动，2026-11-04最终验收。状态：50项动态通过，Clippy/fmt通过，代码冻结交独立复签。

已按TDD与planning-with-files恢复同phase计划，遵守主会话单写共享规划。worker只维护本ledger及授权文件。Cargo唯一owner，CARGO_BUILD_JOBS=2。Warp/ralph-loop不可用，不声称调用。无commit/push。

RED真相源 `target/phase7-explain-server-tests-r2.log`：mcp_l2_typed_explain_concurrency_limit_releases_permit，exit101，0 passed/1 failed。真实ServerHost cap1，首请求已阻塞于上游，第二请求200而期望429。修复不改变L2执行策略，只修复HTTP/rmcp执行槽所有权。

实现约束：共享business semaphore；MCP outer reservation有界auth/body；auth/owner后typed control分类；固定1control槽；permit通过Parts进入handler唯一take，Action转owned task。GET与精确DELETE按合同处理。新测试只覆盖共享业务槽、control真实取消/非法不绕过、Action持槽、失败路径资源回收与有界body，不扩大产品配置。

新增独立RED：`cargo test -p ontoforge-server --test mcp_execution_admission --locked mcp_read_holds_shared_slot_but_real_cancel_and_sse_still_work -- --nocapture`，jobs2，exit101；0passed/1failed，0.13秒。MCP真实read阻塞后REST请求200!=429，确认跨transport共享槽提前释放。日志`target/phase7-mcp-admission-red.log`。编译成功，不以编译错误作为RED。

第一轮实现检查：`cargo check -p ontoforge-server --tests --locked` exit101，pattern guard移动notification与unused Duration import；仅编译错误，修复后执行动态测试。`cargo test -p ontoforge-server --test mcp_execution_admission --locked -- --nocapture` exit101，3passed/1failed，0.23秒。MCP→REST、满槽真实取消/SSE/伪造control、Action保持及错误资源回收通过；REST→MCP仍200!=429（`target/phase7-mcp-admission-green.log`）。原因：REST Body extractor在业务await前丢Parts，outer必须继续持自己Arc直到next.run返回，只drop不take。已报告主会话与Staff复签，等待最小修正接纳。

控制分类使用原bytes反序列化必需字段Envelope、必需RequestId Target、rmcp CancelledNotificationParam与完整ClientJsonRpcMessage；Value仅检查id成员是否存在，不能用Value消除重复后再解释control。新增raw重复method/params/requestId否定及伪造Mcp-Method、id:null、batch/无requestId、子路径否定验证。

主会话取得CC AGREE（`target/phase7-rest-permit-lifetime-claude-review.json`）后修正outer REST保同一Arc至next.run/timeout返回，只drop不take。MCP通过Parts本次请求保活，无需保外层旧reservation，避免cancel从business转control后旧business仍被outer占用。此次REST失败是实现阶段遗漏，不列为既有生产缺陷。

最终实现预检 `cargo clippy -p ontoforge-server --all-targets --locked -- -D warnings`，jobs2：exit0，23.36秒；日志 `target/phase7-mcp-admission-clippy.log`。正在统一执行新integration4项、新slow-body unit、原lib/transport/typed L2相邻测试；未得到动态结果前不标通过。

## 最终动态与静态证据

`CARGO_BUILD_JOBS=2 cargo test -p ontoforge-server --test mcp_execution_admission --test mcp_transport --test mcp_l2_defaults --lib --locked -- --nocapture`：exit0，50 passed / 0 failed / 0 ignored；编译3m46s。日志 `target/phase7-mcp-admission-green-r2.log`。

| 套件 | 结果 | 耗时 | 关键行为 |
|---|---|---|---|
| lib | 25/25 | 9.08秒 | T24原12项、REST既有合同、真实慢body固定1control reservation及deadline释放 |
| mcp_execution_admission | 4/4 | 0.23秒 | REST↔MCP双向抢同一业务槽、非法control不绕过、满槽真实cancel、SSE、DELETE、Action取消/断连后继续持槽、失败/超body释放 |
| mcp_l2_defaults | 10/10 | 61.34秒 | 原N3四typed格式cap1真实RED转GREEN；零额外上游/业务audit；真实proof expiry与恢复 |
| mcp_transport | 11/11 | 7.25秒 | 认证/owner/token expiry、SSE、定向取消、owned Action与shutdown deadline保持 |

慢body测试通过真实ServerHost启动阻塞query占业务槽，再以raw TCP仅发送部分Content-Length body，观察该host真实control semaphore=0作为barrier。第二cancel立即429；原请求deadline产生504并释放control，随后cancel实际触发engine.cancelled且仅目标read留1条cancelled audit。没有fake Semaphore替代生产准入。

`shutdown_surfaces_owned_action_task_failure`的panic是既有故障注入，测试本身通过。真实proof expiry超过60秒是既有TTL验收，无hang。

最终Clippy：上述all-targets命令exit0，后续未改代码。最终fmt：`rustfmt --check --edition 2024 --config skip_children=true`对7个修改/新增Rust文件exit0，日志`target/phase7-mcp-admission-fmt.log`（通过时为空）。

## 冻结与交接

生产4文件：context.rs `12de800b01837b3b8ec8e26b819c82d382e0ac82`、mcp.rs `7b154b0100f8e9433ab95061369d075cd8d1539e`、lib.rs `d14d825c8e60199fe983bd472fa4c0e66837c5e1`、state.rs `c6a2287eb87f6b06759a8c9a493e7cd8ca4faad0`。测试3文件及全部SHA256见`target/phase7-mcp-admission-fingerprints.txt`。

T24 manager未修改；既有mcp_l2_defaults.rs未修改；unit/mcp_sessions.test.rs只把admit state的Duration改ServerLimits，deadline300ms保持。新test文件mcp_execution_admission.rs与unit/mcp_admission.test.rs。没有新增依赖、配置或control queue。

Cargo已明确归还主会话，所有动态验证命令结束。独立Staff/security复签正在以冻结指纹和本轮原始日志闭环。无commit/push，不改共享规划。
