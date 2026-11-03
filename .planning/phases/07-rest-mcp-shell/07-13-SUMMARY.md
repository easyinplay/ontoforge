---
phase: 07-rest-mcp-shell
plan: "13"
status: executing
subsystem: integration
tags: [postgres, trino, ci, real-services]
requires: [07-09, 07-10, 07-11, 07-15, 07-16]
provides: [server-services-ci-framework, real-service-test-entrypoints, local-real-service-evidence]
affects: [API-01, API-03, MCP-01, MCP-06, MCP-07, MCP-08, L2-04, SCOPE-07]
---

# Phase 07 Plan 13 Summary

2026-11-03 Windows helper方案A已通过。正式双轴review的12项发现现已定向验证并独立关闭10项，RF-01/RF-05仍待契约选择与实施，详`reviews/07-REVIEW-FIX-CONTRACT.md`。整合代码已冻结至86e3648，新CI待跑；旧CI绿色不替代当前head，也不推导Phase完成。

## Implemented

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

- RF-01：drift周期owner/版本与source gate仍未实施；部分target漂移的health状态待用户选择，设计见reviews/07-DRIFT-FIX-DESIGN.md。
- RF-05：MCP控制/业务参数信封待用户选择，尚未改变现有平铺入参。
- 当前整合head的完整CI尚待运行，不能由各slice定向GREEN代替。

## Not Complete

- 不更新 ROADMAP 的 07-13 checkbox。
- 不关闭 `L2-04` / `SCOPE-07`。
- 不进入 07-14 release 计划，直到本轮剩余完整门禁通过。

## Next Step

关闭`reviews/07-REVIEW-FIX-CONTRACT.md`中的RF审查缺口，补新增改动的定向回归、Linux权限实测与独立复签，之后按影响范围完成整体验证。07-14依赖仍未解除；Phase 7 verify在全部PLAN验收后执行。
