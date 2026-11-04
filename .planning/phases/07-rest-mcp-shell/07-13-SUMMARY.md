---
phase: 07-rest-mcp-shell
plan: "13"
status: complete
subsystem: integration
tags: [postgres, trino, ci, real-services]
requires: [07-09, 07-10, 07-11, 07-15, 07-16]
provides: [server-services-ci-framework, real-service-test-entrypoints, local-real-service-evidence]
affects: [API-01, API-03, MCP-01, MCP-06, MCP-07, MCP-08, L2-04, SCOPE-07]
---

# Phase 07 Plan 13 Summary

2026-11-03 07-13验收完成。最新head10175f6的CI41638673666四项必需job全部success（Windows、Ubuntu、integration、phase7-real-services）；本地fresh矩阵36/36、生成/typecheck/build、artifact scan与cleanup全部exit0。RF01最终定向27/27、五包Clippy与独立复签通过，HF01/RT02/RT03整合闭环。元数据target/phase7-ci-41638673666.json；详`reviews/07-DRIFT-GSTACK-REVIEW.md`与`reviews/07-DRIFT-REAL-SERVICE-TESTS.md`。07-14及Phase级验证另计。

整合9e7df9c的CI41634382039暴露RT02/RT03两项旧测试适配。RT02提交d49d0ad，真实1/1 GREEN；RT03提交41d142b，最终11/11（含61秒expiry）GREEN；均独立复签clean，生产未变，详对应TEST-RECHECK报告。

## Implemented

- `29375e8`：RF01单owner/精确版本view/全source出口gate/health及清理；AB01保留Action领域预算；FT01与DC01补齐固定view及DELETE链路，见独立修复与复签报告。
- `fb814df`：两项真实source drift测试加入现有named discovery/执行CI门禁，缺失/ignored不作为通过。

- `a733759`：真实 PostgreSQL 独立 schema、双 store、reload/revoke barrier、原子写入计数、持久审计与 HTTP Action/history/result 合同。
- `2057c33`：真实 Trino source barrier、snapshot/reload、L2 MCP 六工具、四种 EXPLAIN、ACL deny、61 秒 proof expiry/recovery、core 503/recovery、shutdown Action dependency/audit。
- `0aa3e67`：独立 TCP 读断开后的 cancellation、durable audit `cancelled`/`row_count=0` 与无业务写入断言。
- `6325a8b`、`b2e7cc9`、`aba451b`：PG/Trino/L2 服务生命周期、seed、named-suite discovery、Rust/TypeScript gates、credential scan、always cleanup 与 CRLF 兼容。
- `4b0b18d`：SDK发送前无损JSON能力验证，移除公开raw绕过入口；8项contract通过。
- `3f5b5db`、`150a6d2`：MCP参数schema修复与server lint收尾，14项相邻测试通过。
- `9e6d858`、`7fa2401`：Rust receipt序号checked conversion、15项client lint和history分页参数修复，最终4 unit+6 HTTP contract通过。
- `c0abe96`：CLI纯lint最小修改已提交；本地完整Clippy/既有行为回归仍待结果。
- `ec15dd2`、`77382e7`：取消读任务清理及精确audit断言、容器内seed、Windows网络/运行binary隔离接线与Cargo jobs=2。
- `02a9c4e`、`fd24beb`：Windows私有隐藏console owner/stop helper、uname平台识别与后台Ctrl+C处理恢复；11项真实进程回归与完整服务脚本通过，独立review clean，已接入Windows CI。

## Earlier Local Verification (2026-10-31)

- `cargo test -p ontoforge-server --test postgres_http_it --locked -- --ignored --nocapture --test-threads=1`: **2 passed, 0 failed**。
- `cargo test -p ontoforge-storage --test request_snapshot --locked -- --ignored --nocapture --test-threads=1`: **1 passed, 0 failed**。
- `cargo test -p ontoforge-server --test trino_http_it --locked -- --ignored --nocapture --test-threads=1`: **2 passed, 0 failed**。
- `cargo test -p ontoforge-server --test health_trino_it --locked -- --ignored --nocapture --test-threads=1`: **2 passed, 0 failed**，包含真实 61 秒 TTL。
- 三个 test target `--no-run`、named discovery、`cargo fmt --all -- --check`、`bash -n ci/server-services.sh`、`git diff --check`: 通过。
- 本地 Git Bash/native Docker 启动 PostgreSQL、Trino 483、受限 L2 和 loose ACL 对照实例；L2 端口 `58081/58082` readiness 与 seed 通过。

## Final Local Verification (2026-11-03)

- `cargo test --workspace --locked -- --test-threads=1`: exit0，1373 passed、0 failed、113 ignored（含doc-tests）；`target/phase7-final-workspace-test-20261103.log`。该证据早于本轮RF审查修复，不证明之后新增Rust改动已全量通过。
- 全workspace all-targets Clippy、embedded-pg Clippy、fmt与TLS依赖检查通过。
- `python -B -m unittest discover -s ci -p test_windows_server.py -v`: 11/11，17.867s，`target/phase7-windows-platform-tests.log`。
- `unset OS; bash ci/server-services.sh verify`: `fd24beb`完整exit0；PG2、snapshot1、Trino2、health2、ACL6、Rust client6、TS9均通过且无skip，生成/typecheck/build与credential scan通过。
- Windows owner state记录`phase=exited, exit_code=0`；目标server进程和四测试容器均不存在。快照与named logs：`target/phase7-windows-final-evidence-20261103/`；完整输出/退出码：`target/phase7-windows-final-services-20261103.log`、同名`.exit`。
- 独立复签：`reviews/07-13-STAFF-RECHECK.md`和`reviews/07-13-WINDOWS-HELPER-REVIEW.md`均无剩余BLOCKER。

## Earlier CI Evidence

2026-11-03 Staff五项修复已冻结并独立复签，该审查范围0剩余BLOCKER，详`reviews/07-13-STAFF-RECHECK.md`。提交链为`513a2f6`、`100976a`、`cf2a589`、`9aeb3ae`、`0dd3995`、`f835752`。后续正式双轴审查的RF缺口独立跟踪，不由早先复签关闭。

1. CI run `41577377433`（head `d649260`，PR merge测试commit `55309c0`）全部4个必需job通过。专项artifact位于`target/phase7-linux-ci-41577377433/`，Rust named suites共19项、TS9项通过；credential scan、上传和cleanup全部success。
2. CI run `41593120599`（head `fd24beb`）全部4必需job success，含Windows helper新增回归；job元数据已保存`target/phase7-ci-41593120599.json`。
3. 本地全workspace regression已exit0（1373 passed、0 failed、113 ignored，含doc-tests），full Clippy与embedded-pg Clippy通过；远程artifact credential scan已success。最终交叉复签已落盘。
4. CI run41600147824的4必需job全部success，PR head0a3e359；Linux实际测试synthetic merge dcaf6dd，包含dev unit2/2和生命周期12/12（4项Unix专项）。元数据`target/phase7-ci-41600147824.json`，Linux原始job日志`target/phase7-dev-linux-ci-41600147824.log`。

## Review Fix Delivery (2026-11-03)

| 提交 | 修复与证据 |
|---|---|
| 5e00281、0a3e359 | dev生命周期OS锁/锁内复核/Unix权限及symlink；Windows2+8、Linux2+12，详reviews/07-DEV-SAFETY-RECHECK.md |
| 24422dc、f834d03 | history安全错误与审计ID、完整记录预算分页；综合20项，详reviews/07-HISTORY-ERROR-RECHECK.md与07-HISTORY-PAGINATION-RECHECK.md |
| 4f9d159 | L2 health DTO/SDK完整保留；Rust13/API6/TS10，详reviews/07-HEALTH-WIRE-RECHECK.md |
| ff97513、86e3648 | MCP metadata/参数类型/预算、L0/L2安全错误与审计、分页接线；最终唯一46项，详reviews/07-MCP-L0-RECHECK.md与07-MCP-FINAL-RECHECK.md |
| d3c80f6 | Action参数/criteria诊断与SDK兼容，server11/Rust17/API7/TS14，详reviews/07-ACTION-DIAGNOSTICS-RECHECK.md |

各slice Clippy与格式检查通过；主会话全workspace fmt --check、git diff --check通过。上述测试集合有交集，不相加为唯一总数。MCP取消/关闭真实transport没有JSON返回，证据为source trace-token与durable audit对账及reserve成功后、首次await前同步observer，不能宣称错误JSON已送达。

## Remaining Blockers

- 本plan无剩余阻塞。isolated-query-bench按条件skipped，不是本plan必需job；未将其视为实跑。Phase级需求仍待07-14与最终verify归并。

## 验收范围

- 本plan的真实服务/CI验收已完成，允许进入07-14。
- `L2-04` / `SCOPE-07`保留Phase级最终验证流程，发布、npm和PR合并未执行。

## Next Step

执行07-14实际三平台artifact构建及完整安装smoke。Phase 7 verify在全部PLAN验收后执行。

## Self-Check: PASSED

已核对当前head、四项必需CI、本地完整命名非零suite、最终blob复签及资源清理；原12项审查缺口均闭环。所有日志边界与未执行的发布操作明确，未用旧head绿色替代本次验收。
