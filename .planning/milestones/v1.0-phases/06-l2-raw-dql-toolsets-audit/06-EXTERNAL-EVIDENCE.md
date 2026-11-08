# Phase 6 External Execution Evidence

## Scope

2026-10-28 补齐 `06-VERIFICATION.md` 的 H1/H2 外部运行证据及最终 workspace Clippy 原始输出。本次没有修改实现、测试、fixture 或验收合同，也没有重跑完整 workspace。

- 最终实现：`e81127fa41099365c44c2c42750024efad6653cb`，提交时间 `2026-10-28T02:20:29Z`。
- PG 执行时 HEAD：`d6903afe512dd73af28e1c1696f96af804948f58`。
- Trino/Clippy 执行时 HEAD：`c674e32af766eefa7042936bd8edde9a4c8b4d30`。
- 各外部日志记录 `git diff e81127f HEAD -- crates ci .github Cargo.toml Cargo.lock`，输出为空；上述 HEAD 之间的变化均不影响本次被验实现。
- 复用最终 workspace 原始日志明确列出的同名 test executable，避免重新链接。原日志：`C:/Users/easyi/AppData/Local/Temp/ontoforge-phase06-10-workspace-accepted.log`，SHA256 `B53F76BD2FC9E1C2ED40B489CBBCACB35E0000E9218176F0DD15F4F869F0E4D8`。

## Binary Correspondence

| Target | Final workspace log | Binary mtime UTC | Binary SHA256 |
| --- | --- | --- | --- |
| `conformance_pg-39e883f11e6a6c0e.exe` | line 1696 | `2026-10-28T02:43:50Z` | `453A0ACC8A4C423F7ED526D532758664BDA9A0B9C721F33E4FB93A773DF7D421` |
| `trino_l2_acl_it-0af57445ba4215b1.exe` | line 864 | `2026-10-28T02:48:44Z` | `C4DEC5B77F8188264AB76734436B0B90650F2B1881283587622119E06EC14DAF` |

二进制路径、大小、mtime、hash 均记录在对应原始日志。Cargo fingerprint 两个 target 的 features 均为 `[]`；mtime 晚于最终实现提交，最终 workspace 原始日志在 `02:54:59Z` 写完并列出相同 executable。这里的对应依据是原始构建运行日志、文件元数据及无源码差异，不将 mtime 单独视为构建证明。

## H1: PostgreSQL Audit

复用已有 `ontoforge-pg-05-06`，连接端口 `55432`、数据库 `editstore`。版本查询直接返回 PostgreSQL `17.11`。连接凭据只在当前子进程环境内设置，未输出或写入日志。

```powershell
target/debug/deps/conformance_pg-39e883f11e6a6c0e.exe audit_ --ignored --list
target/debug/deps/conformance_pg-39e883f11e6a6c0e.exe audit_ --ignored --nocapture
```

发现 6 项，默认并行运行 **6 passed、0 failed、0 ignored，1.62s，exit 0**：

- `audit_batch_retention_parity`
- `audit_sink_failure_rejects_inflight_finish`
- `audit_batch_rejection_is_atomic`
- `audit_cleanup_rejection_rolls_back_batch`
- `audit_pg_connection_interruption_recovers`
- `audit_shutdown_drains_inflight_guard`

日志中的两次 `read audit sink did not recover before the pending age limit` 是故障注入路径的输出；对应测试均通过，不从日志中删除。

## H2: Trino 483 ACL And Display

通过现有 `ci/query-services.sh trino-l2` 启动独立受限/宽松对照实例，分别映射 `58081`、`58082`。两次 `/v1/info` 直接返回 `nodeVersion.version=483`、`starting=false`。原 `58080` Trino 不变。

```powershell
$env:ONTOFORGE_TRINO_L2_URL = 'http://127.0.0.1:58081'
$env:ONTOFORGE_TRINO_L2_LOOSE_URL = 'http://127.0.0.1:58082'
target/debug/deps/trino_l2_acl_it-0af57445ba4215b1.exe --ignored --list
target/debug/deps/trino_l2_acl_it-0af57445ba4215b1.exe --ignored --nocapture
```

发现 6 项，默认并行运行 **6 passed、0 failed、0 ignored，9.91s，exit 0**：

- `trino_l2_acl_hides_known_active_other_query_and_attests_engine`
- `trino_l2_acl_loosened_rule_fails_closed`
- `trino_l2_acl_rejects_existing_outside_and_edit_tables`
- `trino_l2_monitor_loose_acl_fails_closed`
- `trino_l2_monitor_short_query_does_not_prove_active_isolation`
- `trino_l2_monitor_startup_cancels_real_active_probe`

原始输出有 32 条 `FINISHED`，覆盖本 target 发送的全部 allow Display SQL；测试同时检查拒绝路径、真实 RUNNING 前后观测、短查询不能证明隔离及 startup cleanup。

## Workspace Clippy

```powershell
cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings
```

新日志直接记录 `Finished dev profile`，Cargo 输出 9.30s，计时 9.84s，`clippy_exit_code=0`。未采用旧 `ontoforge-phase06-10-clippy-final-fixed.log` 作为 GREEN 证据。

## Raw Logs

路径均相对于当前 phase 目录；日志保留完整 stdout/stderr、命令、SHA、发现数和退出码。
`.evidence/.gitattributes` 仅对本目录日志设置 `-text` 和 `whitespace=cr-at-eol`，保留 Windows 原始 CRLF；已核对 Git index 与磁盘日志字节一致，避免跨机器 checkout 改变以下 hash。

| File | SHA256 |
| --- | --- |
| `.evidence/external-pg-audit-20261028.log` | `7B5988A761068E98F60A35A66152DF85BF6A51BA7DE0ED59B2F649BFE4189279` |
| `.evidence/external-trino-acl-20261028.log` | `36A81A389A1262D2933DEF476E068369D9679C5FE535371314F3F99DEB2D9259` |
| `.evidence/clippy-workspace-20261028.log` | `89C635C6C657D6E38E9C81C45F348B4CB1C5B9E56DCF1A9FF7ED048FB012E0C5` |
| `.evidence/trino-fixture-start-20261028.log` | `9CAD2F3D229B944D22225B22ACA86E87A1A6B9BAE7278B2014CAACDAEF608AAA` |
| `.evidence/external-cleanup-20261028.log` | `8181213775AE4B4AB4C62A773747B279B50D22FB9418A1FBC1E06AC081883636` |

## Cleanup And Remaining Boundary

- 已删除本轮创建的两个容器：`ontoforge-trino-l2` (`3738891665a2...`)、`ontoforge-trino-l2-loose` (`bbb8d2815a4c...`)；cleanup 日志列出删除结果及保留服务。
- 原 `ontoforge-trino-05-06`、`ontoforge-pg-05-06` 与 `vennuap-casdoor` 保持运行。
- 新目录 `C:/Users/easyi/AppData/Local/Temp/trino-l2-catalog.XkH7lfZ3` 已核验为 Temp 直接子目录，含 5 个普通文件，无链接或嵌套目录。删除命令被自动审批拒绝：`rejected: blocked by policy`。没有重试或换方式绕过，目录保留；该拒绝发生在进程启动前，不在 cleanup 命令日志中。
- 旧已拒绝清理目录 `trino-l2-catalog.UGBYEziV` 未触碰。
- 本次三个 exec session `55026`、`15990`、`95952` 均正常 exit 0，没有必要运行中 session。
- 业务和测试未修改，跳过新的业务 TDD；`ralph-loop` 不可用，未声称运行。
- H3 当前交付 SHA 的远端 CI 不由这些本地结果代替。`06-VERIFICATION.md` 需由 verifier 重新审阅证据、签署 fingerprint 后更新；本文件不自行改变 phase status 或整体需求状态。
