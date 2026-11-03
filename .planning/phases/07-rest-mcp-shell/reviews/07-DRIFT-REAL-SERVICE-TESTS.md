---
fix: RF-01
status: fresh-integration-passed
tests_executed: true
production_modified: false
---

## 已编写的独立 suite

新增 `crates/ontoforge-server/tests/source_drift_trino_it.rs`，默认两个 `#[ignore]` 用例：

- `real_source_drift_periodic_target_isolation_and_recovery`
- `real_source_drift_external_reload_does_not_publish_old_metadata`

已阅读 ADR-0030、`07-DRIFT-FIX-DESIGN.md`、`07-DRIFT-ENG-REVIEW.md`、现有 `health_trino_it.rs` / `trino_http_it.rs` / `postgres_http_it.rs` 及真实服务 helper、`ci/server-services.sh`。复用 `support/real.rs` 的 env/HTTP helper，没有新增或修改 support 文件；服务通过真实 `ServerHost::serve` 启动。

## 实际代码中的验收路径

第一个用例创建独立 PostgreSQL source 表，由真实 Trino `demo` catalog 读取。同表绑定 `Wide`、`Narrow` 两个逻辑 target，分别依赖 `drift_value`、`stable_value`；另有 `Local` editOnly。先检查正常 REST/read/health，再 DROP 仅 Wide 需要的列，等待生产周期检查形成 `SOURCE_SCHEMA_DRIFT`。随后检查 REST 503、安全 envelope 与持久 audit ID、MCP get_object 拒绝且无业务 SQL、Narrow/Local 仍可用、匿名 health 精确为 200 degraded 且无 target/物理名。恢复列和值后等待生产周期恢复正常。

第二个用例使用两个真实 PostgreSQL `EditStore`。包装真实 Trino 的 `describe_tables`，先获取真实 metadata，再以 Notify/Semaphore 阻塞返回。旧轮保持健康 metadata，第二 store 在 5 秒上限内 reload 要求新列的新定义。新请求必须 Pending；释放旧轮后再次拦住新轮，确认旧健康报告未被复用。释放新轮后必须得到新版本 drift 与对应版本的 durable audit，其他 target/editOnly 保留服务。

该包装器不直接执行 `check_drift`，不替换 metadata 内容，不使用新公开 monitor 配置。实际周期仍为合同的 30 秒、每轮 20 秒；条件观察上限 75 秒，250 ms 轮询。跨版本顺序由 metadata barrier 控制，不以 sleep 推测探测状态。

## 环境与资源边界

- `ONTOFORGE_PG_URL`：允许创建/drop 专属 schema 的测试 PostgreSQL editstore 数据库连接。
- `ONTOFORGE_TRINO_URL`：真实 Trino；其 `demo` catalog 必须指向同一 PostgreSQL 的 `demo` 数据库，沿用 `ci/server-services.sh`。
- 不依赖外部运行的 Ontoforge server/token；suite 自建 host、auth key 和 token。
- edit/sys/source schema 分别使用 `of_drift_<ULID>_edit`、`_sys`、`_source`，每次用例独享。source DDL 仅作用于自建 `source_rows`。
- 用例 panic 由外层 task 捕获，先发送 shutdown、释放测试 barrier、等待 host 停止并 drain audit/cancellation，再 DROP 明确的 suite schema。不会 ALTER 或 DROP 用户原有表/schema。
- PG URL/凭据不输出；连接构造失败使用静态错误说明。

## 动态执行与证据限制

RT01 预审修正已落实到测试源码：`mcp_get` 必须收到 HTTP 200、JSON-RPC 2.0、匹配的请求 id，且 error/result 恰好出现一个；错误需要合法 JSON-RPC 数值 code。MCP 成功必须有对象形状的 `result.structuredContent`，并精确匹配 `objectType`、`pk=row-1`、`properties.id=row-1` 和 fixture 业务值。REST 的初始、隔离和恢复成功路径同样核对对象身份与业务值，不再只检查 HTTP 200。拒绝继续检查安全 data code/apiName、真实 audit ID 和零上游业务 SQL。

协议口径沿主会话已收敛结论：REST drift 拒绝为 HTTP 503；MCP 工具拒绝保持 HTTP 200 与 JSON-RPC error/data。第二意见见 `target/phase7-claude-mcp-drift-status.json`、`target/phase7-claude-real-test-assertions.json`。

2026-11-03 使用独立 `RUNNER_TEMP=target/phase7-drift-real-runtime-20261103` 启动真实 PostgreSQL/Trino，所有 Cargo 均为 `CARGO_BUILD_JOBS=2`，按独占槽位串行执行。首次 start 因 PATH 缺少 jq 在容器操作前退出；加入现有 `target/phase7-tools` 后成功。凭据未输出。执行命令与结果：

| 验证 | 命令 | 结果 | 日志 |
| --- | --- | --- | --- |
| discovery | `cargo test -p ontoforge-server --test source_drift_trino_it --locked -- --ignored --list` | exit 0，发现上述两个命名用例 | `target/phase7-drift-real-discovery.log` |
| mutation RED | `cargo test -p ontoforge-server --test source_drift_trino_it --locked -- --ignored --nocapture --test-threads=1` | exit 101，0 passed / 2 failed / 0 ignored，106.65s | `target/phase7-drift-real-red.log` |
| 恢复后 GREEN | 同上 | exit 0，2 passed / 0 failed / 0 ignored，92.08s | `target/phase7-drift-real-green.log` |
| HF01 fixture RED | `cargo test -p ontoforge-server --test health_trino_it --locked real_health_l2_failure_expiry_recovery_keeps_core -- --ignored --nocapture --test-threads=1` | exit 101，0 passed / 1 failed / 0 ignored，5.29s | `target/phase7-drift-real-health-red.log` |
| 定向 Clippy | `cargo clippy -p ontoforge-server --test source_drift_trino_it --test health_trino_it -- -D warnings` | exit 0，29.01s | `target/phase7-drift-real-clippy.log` |

RED 的临时 mutation 经主会话与 core 授权，只旁路 `ScopedEngine::check_source_target` 的 `self.drift.check(target)?;`。双 store 用例实际得到 `SOURCE_UNAVAILABLE`，期待 `SOURCE_CHECK_PENDING`；周期用例在 75s 内未观察到 `SOURCE_SCHEMA_DRIFT`，此结果不表示 monitor 未运行。恢复后 `context.rs` 的 Git blob 精确为 `bfc178c59d5e8de31a0d7f36197bcc5b31be82ec`，SHA256 精确为 `F2EE5C5BCC9522E957433083F6B222367BFFA301AC73DB0D0B5C9B798AB4A4A0`，与 mutation 前备份相同。此后已归还 core 文件所有权；这些 hash 只证明恢复时点，不能用于覆盖后续 core 修复。

GREEN 前补强 `wait_state` 的 timeout 诊断，记录最后 HTTP 与字段值，未改变观察上限或业务判定。rustfmt 与 `bash -n ci/server-services.sh` 通过。`ci/server-services.sh` 已接入 source_drift suite 的命名 discovery/执行，完整矩阵尚待 fresh 验证。

## HF01 fixture 修正与待办

原 health 用例以 L2 ACL TPCH engine 承担 aviation ontology 的 CoreProbe，生产 monitor 认定 source drift；RED 在 `health_trino_it.rs:214` 得到 `"degraded"`，期待 `"healthy"`。最小修正为从 `ONTOFORGE_TRINO_URL` 构建普通 ontology engine 供 CoreProbe 使用，原 ACL engine 继续供 `recheck_l2_acl`。健康断言、loose/disconnected 行为不变。该修正已通过定向 Clippy，HF01 GREEN 尚未执行，等待 core Action source deadline 修复冻结后运行 fresh 完整 `ci/server-services.sh verify`。

DC01 已获主会话批准，两个非 ignored 用例已写入 `source_drift_cleanup.rs` 并动态通过。使用真实 ServerHost、真实 TrinoEngine、SQLite EditStore 与本地 axum 受控 HTTP 上游。metadata POST 返回合法 query id/nextUri，GET barrier 证明 in-flight；shutdown 触发精确 query 的一次 DELETE，DELETE 阻塞时 host 必须等待，释放 204 后 audit 关闭，DELETE 500 必须得到有界 `HostError::Shutdown`。该项为 test-only 验收扩充，受控 DELETE 延迟/失败为故障注入，不执行生产 mutation，不声称 TDD bugfix。

主会话指定整合顺序：core 释放 Cargo 后先执行 DC01 与必要的目标 Clippy/fmt，再冻结测试供独立复签；由主会话完成原子 commit/push 触发新 HEAD CI 后，收到通知才启动 fresh 完整 verify，使本地 `commit.txt` 与远程 CI 对齐。HF01 GREEN 保持 pending。

## 最终定向冻结验证

2026-11-03 core 释放独占槽位后，以 `CARGO_BUILD_JOBS=2` 执行 `cargo test -p ontoforge-server --test action_source_budget --test source_drift_cleanup --test source_drift --locked -- --nocapture`，exit 0。Action source budget 5 passed、source drift 20 passed、DC01 2 passed，三组均 0 failed / 0 ignored，运行时间分别为 0.97s、0.78s、0.18s；日志 `target/phase7-drift-cleanup-budget-final.log`。source drift 中的 `controlled metadata panic` 是预期故障注入，对应测试通过。

本轮覆盖 core 最后整理的 `source_drift.rs` 与 Action validate-only 零 edit/log 新断言。测试无需修改。此前 core 已执行包含 DC01 的五包 all-target Clippy，exit 0，日志 `target/phase7-action-budget-clippy.log`；本轮再执行 `cargo fmt --all -- --check` 与 `git diff --check`，均 exit 0，没有重复无变更的 Clippy。

| 最终测试文件 | Git blob | SHA256 |
| --- | --- | --- |
| `action_source_budget.rs` | `84634688a5feca5d3b29477b7035c16f697cc18f` | `04DD63379763699B180B6490248D2A0D0417843DF53E961D5FEE628C00AE17B2` |
| `source_drift.rs` | `45083f2ee5caeb8bbeaac7bc0f2eddb6acc3521b` | `EA8457EEC3523513CEFD9DF83A8179C18426D5A9C79632956BBBEAFA461B62F4` |
| `source_drift_cleanup.rs` | `0dcf954763cbec905d795714e7cc8aa716dbf30b` | `1700F362607E46C8B157B9A37E2C63308DCE8404A2AC143764FD0EDB21D2462D` |

测试冻结，Cargo 槽位释放，等待主会话复签、commit/push 和 fresh verify 通知。

## Fresh 整合与 RT02

在主会话 push 后确认 HEAD 为 `9e7df9c772588e0b35e8d471ec4a5784f068ca50`，使用独立 `target/phase7-drift-final-9e7df9c-20261103-run2` 启动 `ci/server-services.sh verify`。CLI fresh build/start 成功；PG HTTP 2/2、真实 drift 2/2（92.04s）、PG snapshot 1/1、Trino 六工具用例通过。`trino_http_long_read_releases_snapshot_lock` 在第二 read 的 engine barrier 超时，与远程 CI 同源；Trino suite 为 1 passed / 1 failed / 0 ignored，完整脚本 exit 101。日志 `target/phase7-drift-final-verify.log`，证据 `target/phase7-services/evidence/commit.txt` 精确匹配该 HEAD。EXIT cleanup 后仅剩 `vennuap-casdoor`。后续 HF01、ACL、SDK 检查尚未执行。

首次 shell 参数路由未进入原生 Git Bash，在容器操作前因缺少 Cargo 退出；改为 PowerShell 显式调用 `C:/Program Files/Git/bin/bash.exe -lc`，验证 MINGW64 与 Cargo 路径后在上述新目录重跑。没有生产环境或用户容器操作。

RT02 根因：reload 后第二 read 取得新版本，而 monitor 尚未发布该版本报告，source gate 返回 Pending，未进入测试想观察的 engine barrier。Claude Code 已 AGREE（`target/phase7-claude-reload-readiness-review.json`），主会话授权仅修改 `trino_http_it.rs`：核对 reload/current snapshot 版本，第二业务 read 前有界轮询 `/health`，接受 HTTP 200、engineStatus available、status healthy/degraded，超时保留最后响应；不增加业务 read、不要求全部 target 健康、不修改生产 gate、保持 audit 精确两条、先取消后 shutdown 与零业务写断言。

最小 patch 为 31 insertions / 1 deletion。使用新 `RUNNER_TEMP=target/phase7-rt02-runtime-20261103` 启动真实服务后执行 `cargo test -p ontoforge-server --test trino_http_it --locked trino_http_long_read_releases_snapshot_lock -- --ignored --nocapture --test-threads=1`：exit 0，1 passed / 0 failed / 0 ignored / 1 filtered out，1.04s。随后 `cargo clippy -p ontoforge-server --test trino_http_it --locked -- -D warnings`、`cargo fmt --all -- --check`、`git diff --check` 均 exit 0。证据 `target/phase7-rt02-green.log` 末行：`rt02_exit=0 artifact_scan_exit=0 cleanup_exit=0`；容器仅剩 `vennuap-casdoor`，Cargo 已释放。

RT02 冻结文件 Git blob 为 `bcd0855464feda3652e80115f9461991e54fb2c5`，SHA256 为 `AD94E3458EFFB231C5A9E1612B53DEBCFDFD7B95813BFCA0ABADFB67CDCF5CF7`。已有真实 RED 和修正后 GREEN，无生产 mutation；等待主会话复签、commit/push 后再针对新 HEAD fresh 完整矩阵。HF01/ACL/SDK 的本轮整合结果仍 pending。

## 清理证据与剩余范围

最终 fresh 完整矩阵已在 HEAD `10175f6fadf114d487d112ebb391292a25df38cb` 通过，`target/phase7-services/evidence/commit.txt` 与执行前后 HEAD 精确一致。使用新 `RUNNER_TEMP=target/phase7-final-10175f6-20261103`，重新 build/copy/start 当前 CLI；`bash ci/server-services.sh verify` 最终 exit 0，日志 `target/phase7-final-10175f6-verify.log`（已复制到 evidence）。全部命名 discovery 检查通过。

| 最终矩阵目标 | 动态结果 | 用例运行时间 |
| --- | --- | --- |
| PostgreSQL HTTP | 2 passed / 0 failed / 0 ignored | 4.52s |
| source drift 真实服务 | 2 passed / 0 failed / 0 ignored | 92.01s |
| PostgreSQL request snapshot | 1 passed / 0 failed / 0 ignored，2 filtered out | 0.76s |
| Trino HTTP/MCP（含 RT02） | 2 passed / 0 failed / 0 ignored | 7.07s |
| health（含 HF01 与 shutdown） | 2 passed / 0 failed / 0 ignored | 65.15s |
| Trino L2 ACL/monitor | 6 passed / 0 failed / 0 ignored | 4.61s |
| Rust client contract | 7 passed / 0 failed / 0 ignored | 0.40s |
| TypeScript contract | 14 passed / 0 failed / 0 skipped | 16.74s |

最终共 36 例通过，其中真实服务 suites 15 例、Rust client 7 例、TypeScript 14 例。TypeScript 两个目录 `npm ci`、`check:generated`、`typecheck`、`build`、`test:contract` 全部 exit 0，生成文件无差异。`tools/openapi-gen` 安装日志报告 3 moderate severity vulnerabilities，未执行依赖升级或 audit fix；客户端依赖安装报告 0 vulnerabilities。HF01 GREEN 和 RT02 完整矩阵 GREEN 已闭环，不再 pending。

verify 的 EXIT trap 已执行日志扫描与服务清理。随后将完整总日志复制至 evidence，再显式执行 `check-artifacts` 与幂等 `cleanup`，均 exit 0；日志 `target/phase7-final-10175f6-cleanup.log` 为 `artifact_scan_exit=0 cleanup_exit=0`。最终 `docker ps` 仅剩 `vennuap-casdoor`，runtime 下 `server.pid`、`server.windows-state`、`services.started` 均已移除；Cargo/session 已结束。未修改生产、未 commit/push，主会话负责共享 planning 和 phase 状态。

真实用例结束后只读查询 `pg_namespace WHERE nspname LIKE 'of_drift_%'`，结果为 `demo|0`、`editstore|0`，见 `target/phase7-drift-real-schema-cleanup.log`。自有日志已复制至 `target/phase7-services/evidence/`；`bash ci/server-services.sh check-artifacts` 与 `bash ci/server-services.sh cleanup` 均 exit 0，见 `target/phase7-drift-real-cleanup.log` 的 `artifact_scan_exit=0 cleanup_exit=0`。最后容器列表仅剩 `vennuap-casdoor`，未操作该容器。所有工具 session 已结束，Cargo 槽位已释放。

本真实 suite 不覆盖全部 Query/Action/source hook 出口、monitor panic、engine-wide 权限故障；定向 source drift 的覆盖见其独立报告。完整整合矩阵已通过，phase 是否关闭由主会话汇总其他验收决定。未新增公共 drift 配置，无持久生产修改，未改共享 planning，未 commit/push。Warp/ralph-loop 在本环境不可用，未声称调用。
