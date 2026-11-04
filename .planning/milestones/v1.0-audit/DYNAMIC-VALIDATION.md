# 当前源码动态补证

## 状态

- 2026-11-04：动态补证完成，51 个实际执行测试全部通过；凭证扫描、Windows owner 正常退出、本轮容器清理均通过。原 runner 最终 exit 1 原因是执行期间允许的测试文件专属 commit 改变 HEAD，保留真实结果，另有独立源码等价与清理检查 exit 0。
- 本文件为 worker 独占证据账本；共享 task_plan/progress/findings 由主会话维护。
- 已调用并读取 `planning-with-files`，恢复本 milestone 的 task_plan/progress/findings。

## 执行计划与验收

1. SQLite：`conformance_sqlite` + `test-support`，执行 `action_atomic_`，日志必须至少 5 个通过，不能只有 discovery。
2. PostgreSQL：复用 `server-services.sh` 隔离 fixture 的 `ONTOFORGE_PG_URL`，`source_guard_absent_target_insert_race` + `embedded-pg`，非 ignored，必须 1 个通过。
3. 当前 commit：原始 `server-services.sh` 的 start/verify/finish 函数，真实 PostgreSQL/Trino/SDK 全矩阵；按实际日志计数，不预填 43。
4. Iceberg：原始 nightly 固定 Trino 483/484 digests，使用 `ci/pg/iceberg_catalog.sql` 和 `ci/trino/iceberg`，逐版本真实 CTAS flights；设置两个必需环境变量后执行 `iceberg_table_is_readable`，每版本必须 1 个通过。
5. 每轮执行 secret scan，退出服务确认 owner exit 0，清理本轮创建容器。保留日志与 runner，不接触用户现有容器。

## 准备发现

- `server-services.sh` 支持 source；Windows 服务使用 `ci/windows-server.py` 的私有隐藏 console、PID creation-time 核验和 job owner。
- `source.rs` 中该 PG 测试受 `embedded-pg` feature 控制但未标 ignored；共享 support 优先采用外部 PG URL。
- `trino_it.rs::iceberg_table_is_readable` 在缺少 Iceberg 环境变量时提前返回，因此单独普通 integration 的 ok 不能证明 Iceberg。
- 本机 Git Bash 位于 `C:/Program Files/Git/bin/bash.exe`；系统默认 bash 为 WSL launcher，不能混用。Git Bash 入口必须 unset OS。
- `target/phase7-tools/jq.exe` 已存在。静态端口预检未见 5432/8080/18080/58081/58082 监听；D 盘空闲 105263747072 bytes。
- 仅准备 runner，不更改生产代码、CI、旧 Phase 报告。TDD 不适用：这是执行既有测试和生成一次性 fixture runner，未改业务实现。
- `ralph-loop`/Warp 不可用，遵循主计划中明确验收闭环，不声称已调用。

## 运行证据

已收到主会话明确开始信号，唯一 Cargo/Docker owner 已授予本 worker，`CARGO_BUILD_JOBS=2`。Reviewed HEAD 为 `868643cb0d131b9a7f1fdf2a16a53d1ac445efe8`；两个 runner 已通过 Git Bash `bash -n`。入口要求 commit 参数，并在执行前后核对 HEAD 与生产/CI tracked diff，防止不同源码混入同一证据。

- Runner：`target/milestone-audit-dynamic.sh`、`target/milestone-audit-iceberg.sh`。
- Runtime：`target/milestone-audit-runtime-20261104`。
- Evidence：`target/milestone-audit-evidence-20261104`。
- 服务矩阵资源：PostgreSQL + 3 个 Trino，后续 Iceberg 每轮 PostgreSQL + 1 个 Trino；两阶段之间先清理。
- Docker 健康预检在开始信号后执行；Trino 484 固定镜像本轮实际拉取成功。

### 第一阶段：SQLite

- `conformance_sqlite`、feature `test-support`、filter `action_atomic_`：实际 5 passed / 0 failed / 0 ignored，exit 0。
- 命名：`action_atomic_after_edits_failure`、`action_atomic_after_command_failure`、`action_atomic_after_log_failure`、`action_atomic_success_changes_all_domains_once`、`action_atomic_after_outbox_failure`。
- 构建 1m42s，测试 0.11s；原始日志 `target/milestone-audit-evidence-20261104/sqlite-action-atomic.log`。
- 执行时 Docker Server 29.7.2、内存 8327913472 bytes；仅原有 `vennuap-casdoor`。当前开始隔离服务 fixture。

### 执行中的源码身份边界

- 主会话报告 Ubuntu CI 对既有 `crates/ontoforge/tests/cli_color.rs` 的 tuple 类型触发 `clippy::type_complexity`，拟只做 type alias 修复。
- 本 worker 同意该测试专属文件与文档可在动态补证期间提交，因为此文件不参与本 runner 的命令集合；不授权修改生产源码、fixture、CI 或 Cargo.lock。
- `start-commit.txt` 固定为 `868643cb0d131b9a7f1fdf2a16a53d1ac445efe8`；原 verify 的 `commit.txt` 保留其实际运行时 HEAD。
- 不修改运行中的 Bash 内容。若 HEAD 改变，原 runner 最终恒定 HEAD guard 将真实 exit 1；单独检查起止 SHA 的完整差异后报告源码等价范围。不能把新 `cli_color` 测试列为本轮动态执行通过。

### 第二阶段：真实 PostgreSQL source guard

- 精确命名 `source_guard_absent_target_insert_race`，`--features embedded-pg -- --exact --nocapture`，未加 `--ignored`。
- 实际 1 passed / 0 failed / 0 ignored，exit 0；构建 5m06s，测试 0.39s。
- 继承已启动隔离 PostgreSQL 的 `ONTOFORGE_PG_URL`；源码 `pg_url()` 优先使用该变量，本轮未启动嵌入 PostgreSQL。
- 原始日志 `target/milestone-audit-evidence-20261104/source-guard.log`。服务 fixture 仍用于后续完整矩阵，清理待整个矩阵完成。
- 已独立核对 `868643c → f8851ca073fed34e80e605f0ac91880259df7e12` 唯一差异为 `crates/ontoforge/tests/cli_color.rs` 类型别名；其他 tracked 文件零差异。

### 第三阶段：完整服务矩阵（通过）

原 `ci/server-services.sh` 的 start/verify/cleanup 直接复用，未复制或弱化断言。verify 记录 `commit.txt` 为 `f8851ca073fed34e80e605f0ac91880259df7e12`。

| 目标 | 已完成实际通过数 | failed / ignored | 日志 |
| --- | ---: | --- | --- |
| postgres_http_it | 4 | 0 / 0 | postgres_http_it.log |
| source_drift_trino_it | 2 | 0 / 0 | source_drift_trino_it.log |
| request_snapshot（PG ignored 子集） | 1 | 0 / 0 | request_snapshot.log |
| trino_http_it | 2 | 0 / 0 | trino_http_it.log |
| health_trino_it | 2 | 0 / 0 | health_trino_it.log |
| trino_l2_acl_it | 6 | 0 / 0 | trino_l2_acl_it.log |
| Rust SDK contract | 9 | 0 / 0 | rust-client.log |
| TypeScript SDK contract | 17 | 0 / 0 skipped | typescript-contract.log |

完整矩阵实际通过 43；TypeScript check:generated、typecheck、build 均 exit 0。所有日志位于 `target/milestone-audit-evidence-20261104`。

`check_artifacts`（PG 密码与 3 种 token）通过后进入 cleanup；`server-cleanup.log` 明确 exit 0。Windows owner 的 state 为 `phase=exited, exit_code=0`，PID 40556，creation_time 134382323919596161。第一轮 PostgreSQL/3 个 Trino 已清理，后续独立 Iceberg fixture 已开始。

### 第四阶段：真实 Iceberg 483/484（通过）

| Trino | 固定 image digest | CTAS | 精确命名测试结果 |
| --- | --- | --- | --- |
| 483 | `sha256:db58cc93e593a2706553745f276bb119c9810e69918be56ecde088ba7ccb0534` | CREATE TABLE: 29 rows | `iceberg_table_is_readable`：1 passed / 0 failed / 0 ignored，0.66s |
| 484 | `sha256:2704eb53935af5e582d63d6033e9bf99d20c325e9b0e30b1016ef59a4773979f` | CREATE TABLE: 29 rows | `iceberg_table_is_readable`：1 passed / 0 failed / 0 ignored，0.63s |

- 两版本 `/v1/info` 均返回对应版本与 `starting=false`；原始 `trino-483-info.json`、`trino-484-info.json`。
- fixture 沿用 nightly 固定 PostgreSQL `17.11@sha256:2d2b8998d31037bf721cfdf764d76ba74171b4fab3431b7f72c27c56ddbdf9e3`、`ci/pg/iceberg_catalog.sql` 与 `ci/trino/iceberg/iceberg.properties`，只将本地 Docker 网络 host 和随机密码注入临时 catalog。
- 每版本重建 Iceberg JDBC catalog 数据库，并使用独立 warehouse；真实执行 schema 创建与 flights CTAS。日志 `trino-{483,484}-ctas.log`。
- 两版本均明确导出 `ONTOFORGE_TRINO_URL` 与 `ONTOFORGE_TRINO_ICEBERG=1`，运行 `cargo test -p ontoforge-engine --test trino_it --locked iceberg_table_is_readable -- --ignored --exact --nocapture`；这是访问并比较真实 Iceberg/source 行数的路径，不是提前 return 的 ok。
- 只执行该目标中需要补证的精确测试，未重复全 target。日志 `trino-{483,484}-iceberg.log`、`trino-{483,484}-fixture.log`。

## 最终验证与限制

- 测试合计：SQLite 5 + PG source guard 1 + 完整服务矩阵 43 + Iceberg 2 = **51 passed，0 failed**；所选测试全部实际执行。
- 原 `target/milestone-audit-dynamic.sh` 实际返回 **exit 1**，最后输出 `HEAD changed during validation`。原因是主会话在运行中提交了唯一 `cli_color.rs` 类型别名修复；这是 provenance guard 的真实结果，未改写成 runner exit 0。
- 独立 `target/milestone-audit-final-check.sh` 确认开始 SHA `868643cb0d131b9a7f1fdf2a16a53d1ac445efe8`、verify/final SHA `f8851ca073fed34e80e605f0ac91880259df7e12`；完整起止差异只含 `crates/ontoforge/tests/cli_color.rs`，具体 diff 保存在 `source-equivalence.diff`。所有本轮动态 suite 输入源码、fixture、CI、Cargo.lock 均相同；生产与 clients/tools tracked 工作树零 diff。
- 新 `cli_color.rs` 的运行与 CI 不在本 worker 证据范围，不能据此称其已通过；由主会话的新 HEAD CI 负责。
- 最终重新扫描完整 evidence 目录，两轮 PG 凭证及服务 3 种 token 均未命中，scan exit 0；凭证文件仅存在私有 runtime，未输出到证据或对话。
- Windows owner 已确认正常 exit 0；`server-cleanup.log`、Iceberg `cleanup.log` 为 exit 0。本轮 `postgres`、`trino`、`ontoforge-trino-l2`、`ontoforge-trino-l2-loose` 均不存在；Docker 仅剩原有 `vennuap-casdoor`。
- 最终目标端口 5432/8080/18080/58081/58082 无监听，未发现 Cargo/rustc 进程。
- 独立 provenance/scan/container cleanup 检查 **exit 0**，原始 `final-check.log`；不代表上面的原 runner exit 1 被消除。
- 未触发远程 nightly、issue/comment、Release/npm 发布，也未改生产代码、CI 或旧 Phase 报告。本 worker 未提交。

**交付状态：COMPLETE**。Cargo/Docker owner 可归还主会话。
