---
status: ready_for_real_services
date: 2026-11-03
implementation_started: true
tests_executed: true
---

# RF01 实施记录

合同：ADR-0030、07-DRIFT-FIX-DESIGN.md、07-DRIFT-ENG-REVIEW.md。主会话拥有共享 planning 和提交；本 executor 仅负责 RF01 源码、测试与本记录。core/host源码和本地测试已冻结，Cargo现交还主会话安排真实服务lane，构建始终使用CARGO_BUILD_JOBS=2。本executor未commit/push。

## 修复前入口

- `ontoforge-engine/src/bind_check.rs`：`describe_bindings`、`check_drift` 需要最小泛型化。
- `ontoforge-server/src/context.rs`：现有 `ScopedEngine` 可携带固定 view，`RequestContext` 通过真实 `request_snapshot` 捕获定义。
- `ontoforge-server/src/rest.rs`：Action validate-only 与提交路径直接 `.with_engine(&*host.engine)`；`mcp.rs` Action 同样直连 raw engine。
- `ontoforge-server/src/health.rs`：当前只以 live engine status 判断 core，没有 drift report。
- `ontoforge-server/src/lifecycle.rs`：现有 10s 总预算，monitor 截至第 8s，engine cancellation drain 截至第 10s，接入必须复用此预算。

## 实施与验证顺序

1. 建立真实行为 RED：listener 经生产 serve seam 读取 schema 已漂移的 target 应返回 `SOURCE_SCHEMA_DRIFT`/503，业务 SQL 为零，原 read audit ID 保留。
2. Core：默认 `Engine::check_source_target(BindingTarget)` hook；Query PK/list 首批、后续/backfill、aggregate、M:N 全部出口；Action source 保留四类稳定错误。
3. Host：单 owner、单 slot、精确 `(def_version, profile)` 固定 view；首次及周期 raw engine probe，20s 整轮预算、30s Skip interval、Notify 合并。
4. 接线：ScopedEngine、REST/MCP Action、health 200 degraded / 503 矩阵、启动失败与原 10s shutdown 清理。
5. 定向回归：目标隔离、纯 edit 路径、audit 与零 Action 写入、reload barrier、单 task/取消/panic、真实 fixture metadata；lint。
6. 真实 Trino ALTER 漂移与恢复、真实 PG 双 store reload、真实 shutdown 证据另列，未实际运行保持 pending。

## 实际文件边界

- Engine：`lib.rs`、`bind_check.rs`、`Cargo.toml` 的测试feature、`tests/drift_budget.rs`。
- Query：`lib.rs`、`execute.rs`、`aggregate.rs`、`link.rs`。
- Action：`lib.rs`、`source.rs`。
- Server：新 `drift.rs`；`state.rs`、`lib.rs`、`context.rs`、`lifecycle.rs`、`health.rs`、`rest.rs`、`mcp.rs`、`error.rs`。
- CLI：`serve.rs` 中必要启动失败清理。
- Tests：新 `server/tests/source_drift.rs`、`server/tests/unit/drift.test.rs`，既有 `ontoforge/tests/cli_shutdown.rs` 的端口冲突回归。既有fixture均继续真实describe元数据，未关闭monitor。server `Cargo.toml`仅启用锁定Tokio测试feature。

## 证据

`cargo test -p ontoforge-server --test source_drift --locked -- --nocapture`：RED exit101，4项真实行为失败（0 passed / 4 failed / 0 ignored），编译成功。日志 `target/phase7-drift-red.log`：PK/list错误地返回200；全部target漂移health仍healthy；metadata Unauthorized健康仍200。接入后同命令GREEN exit0，4 passed / 0 failed，日志 `target/phase7-drift-green.log`。曾发现首次shutdown pin借用不满足axum `'static`，改owned `Box::pin`后通过；该编译错误不作RED证据。

owner异常退出、新版本/旧view、周期起点与整轮预算、CLI启动失败清理均已实现并有下列证据。主会话批准engine/server的 `Cargo.toml` 仅dev-dependencies启用现有锁定Tokio `test-util`，未升级依赖。Warp/ralph-loop 未暴露，不声称执行completion promise。

- `target/phase7-drift-owner-red.log`：2项运行，owner结束后新版本仍Pending为可归因行为RED；添加停止标记后 `phase7-drift-owner-green.log` 2/2通过。unit现已按项目规范移动至 `tests/unit/drift.test.rs`。
- `target/phase7-drift-exits.log`：12/12通过，覆盖初始probe取消/回收、初始snapshot失败/audit drain、30s起点/双catalog共20s、source不可读、完整patch读取+Action无需source、list第二批/backfill、M:N searchAround及原4项。
- full patch测试准备阶段修正SQLite raw timestamp/array到合法typed edit值，准备失败不作行为RED。no-source/实际panic/重复owner用例已在focused suite通过。
- 主会话Claude Code意见收窄budget作用域：公共 `describe_bindings` 保留CLI每次查询timeout；私有helper供 `check_drift` 共享deadline。`phase7-drift-budget-red.log` 2项真实失败（public预算错误递减、0预算仍发probe），helper修复后在focused suite 2/2通过，paused clock精确比较public `[5s,5s]` 与drift `[5s,4.98s]`。
- 依据Context7 CLI检索及本机锁定Tokio 1.54.1 `time/clock.rs`核对 `pause()`、`advance(...).await`；Context7摘要中同步advance示例与锁定源码不符，以源码为准。
- `target/phase7-drift-cli-red.log`：有效L2 active_query_sql配置+occupied listener，真实记录1次上游probe，期望0，行为RED。第一版fixture未配置active_query_sql导致无probe，已纠正fixture，不将该先前通过当作RED。
- CLI现提前bind，并在初始化失败显式排空store、L2 owner和raw engine；共用server既有 `SHUTDOWN_TIMEOUT`。新增 `initialize` 与 `serve_initialized` 私有方法只分离启动和监听阶段，保持生产公共入口。
- `target/phase7-drift-clippy.log`：engine/query/action/server/CLI五包 `--all-targets --locked -- -D warnings` exit0，包含另一worker真实服务测试编译。`target/phase7-drift-fmt.log`：全workspace格式检查exit0。
- `target/phase7-drift-final-focused.log`：命令为 `cargo test -p ontoforge-engine -p ontoforge-server -p ontoforge --test drift_budget --test source_drift --test cli_shutdown --locked -- --nocapture`。MCP按主会话确认保留HTTP200+JSON-RPC error，REST source拒绝HTTP503。
- 合并focused已结束：CLI shutdown 9/9、drift budget 2/2、source drift 15/15，exit0。真实Trino/PG仍未运行。
- 独立复签DS01：limits.validate失败错误清理尚未取得的owner，导致重复非法启动关闭已有host。`phase7-drift-ds01-red.log` 2/2真实失败；恢复纯校验return后 `phase7-drift-ds01-green.log` source17/17通过，既有host继续读取并持久写audit。
- 独立复签Action权限顺序：仅Action grant无目标type grant时，source::prepare在最终authorize之前消费drift。`phase7-drift-action-auth-red.log`记录预期NOT_FOUND、实际SOURCE_SCHEMA_DRIFT的行为RED。经主会话Claude Code最终AGREE后最小前移type grant检查，后续事务行列授权不变；`phase7-drift-action-auth-green.log` source19/19通过。健康与漂移两场景、REST/MCP正式/validate-only均NOT_FOUND，零业务SQL/写入/read_audit；有type的MCP双模式保持drift码及request_id。
- 相邻回归 `target/phase7-drift-regression.log` exit0：health_lifecycle 4、http_actions 3、http_contract 3、mcp_contract 16、mcp_transport 11，共37/37。
- 最终 `target/phase7-drift-clippy-final.log` exit0：五包all-target `-D warnings`。`target/phase7-drift-fmt-final.log` exit0：全workspace格式检查。`git diff --check` 无错误。
- 最终 `target/phase7-drift-frozen-tests.log` exit0：source19/19，包含整轮超时后的HTTP503、原health字段与SOURCE_CHECK_UNAVAILABLE断言。source unit exact-key/owner退出2例的GREEN在 `phase7-drift-owner-green.log`；迁移到规范test路径后经all-target编译检查。

## 剩余验收

- 真实Trino ALTER漂移/恢复、非默认物理列、同表不同logical target隔离：pending，另一worker拥有 `source_drift_trino_it.rs`。
- 真实PG双store/barrier下旧轮与新版本隔离、在途view：pending，单位状态测试不替代真实PG证据。
- 真实Trino在途metadata取消与DELETE清理、最终本轮CI门禁：pending，由主会话安排真实服务执行与整合验证。
- 主会话已通知独立spec CR01当前blob clean、Safety DS01 resolved，正式复签归各review文件；本记录不冒充最终Phase7验收。

## 自检

新增文件均已落盘；新增测试无TODO/FIXME/placeholder。未增加endpoint、API DTO、OpenAPI或客户端wire字段；没有默认关闭monitor或新TTL/配置。已保留其他worker修改及共享planning所有权。

真实Trino/PG独立新测试由另一worker拥有 `source_drift_trino_it.rs`，尚未实跑；本记录不代表完整验收。
