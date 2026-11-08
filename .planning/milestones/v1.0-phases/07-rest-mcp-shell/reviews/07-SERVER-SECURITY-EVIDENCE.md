# E1 / E2 / E6 Server 动态验收补证据

日期：2026-11-03。状态：E1/E2 本地动态通过；E6 发送顺序缺口已 RED→GREEN，L2 回归与 server Clippy 通过；真实 PG 服务待最终门禁。共享 planning 由主会话单写。

## 范围与执行计划

- E1：真实 REST 在 source barrier 后由第二 EditStore reload；submit / validate 返回 DEFINITION_STALE，五张业务表零增量，无自动重试；正常提交恰好一次。
- E2：真实 REST / MCP 同主体、同 commandId、同参数并发，原 receipt / event / time 一致，五张业务表恰好一次；真实 MCP 在途 snapshot 跨 reload 与后续撤权。
- E6：同 session 撤销 sql:raw / grant 与真实 proof expiry，拒绝调用产生零上游请求，health 与 proof 状态一致，恢复后重新准入，defaults / qualified 参数不绕过。

## 发现与约束

- `http_action_safety.rs` 原 stale 用例仅正常 submit / validate；`mcp_actions.rs` 原 race 用例仅 schema 名称；`mcp_contract.rs` 原 snapshot 用例仅两个 catalog。
- `support/mod.rs` ControlledEngine 可在真实 source execute 处阻塞。L2 原 fixture 的 mcp-tag SQL 记录不足以发现丢 tag 或误走身份，现新增 endpoint 全 statement 计数，同时核对 source engine calls。新测试不安装后台 L2 monitor，显式 attestation / 恢复发生在拒绝测量窗口外，零发送断言不排除任何身份或 tag。
- `l2_health.rs` 生产 proof TTL 固定 60 秒且使用 std::time::Instant，没有公共短 TTL 注入；expiry 用真实时间覆盖，不能以伪造 health provider 替代 gate。
- 未找到仓库根 AGENTS.md，按会话已提供的 AGENTS 指令执行。此次跳过战略/Phase/brainstorming，因为已批准 T24 设计明确限定补证据边界，无新增产品或架构决策。
- 已调用并读取 planning-with-files。已有行为补证据可直接 GREEN；发现生产缺陷再报告真实 RED。尚未改生产代码。
- Cargo 由 T24 worker 独占，当前不运行 Cargo、不启动服务。Warp / ralph-loop 未提供，不声称运行，也不输出其 completion-promise。

## 测试映射

| 项目 | 测试入口 | 断言 |
|---|---|---|
| E1 SQLite | `http_action_safety::http_stale_and_validate_conflicts_never_retry` | submit / validate 各捕获旧定义，source barrier 后第二 store reload；DEFINITION_STALE、五表零增量、每请求 source execute 仅一次；正常显式提交与 replay 五表增量各 1 |
| E1 PG | `postgres_http_it::pg_http_definition_stale_barrier_has_zero_business_writes` | 第二 PG store reload；两个模式 DEFINITION_STALE；五表零增量 |
| E2 SQLite | `mcp_actions::mcp_rest_same_command_race_and_receipt` | REST/MCP 各到达 barrier 再同时放行；同主体同参数同 command；完整 receipt/event/time 与显式 replay/result 一致；五表各 1 |
| E2 PG | `postgres_http_it::pg_mcp_rest_same_command_race_one_persistent_receipt` | REST source 阻塞期间 MCP 提交；REST 恢复返回相同 receipt；再 replay，PG 五表始终各 1 |
| E2 schema | `mcp_actions::mcp_action_schema_execution_keeps_admitted_definition` | 旧 schema 校验后在 source barrier reload 新 required 参数；在途到旧 definition fence；下一次 list 呈现新 schema，旧参数调用发送前失败，五表零增量 |
| E2 scope | `mcp_contract::mcp_inflight_snapshot_and_revoke_on_next_call` | 真实 MCP 在途读取跨第二 store reload/revoke 仍成功；同 session 下一读取拒绝且零 source 调用；持久 audit 保留旧 def_version |
| E6 revoke | `mcp_l2_defaults::mcp_l2_same_session_revoke_defaults_and_qualified_zero_send` | toolset / SQL level 连带合法依赖 / binding / allowlist 四模式；同 session defaults、qualified 名称拒绝，endpoint 总请求及 source 调用零增量；健康 proof 不受主体撤权影响；恢复实际成功并 audit |
| E6 expiry | `mcp_l2_defaults::mcp_l2_same_session_real_proof_expiry_zero_send_and_recovery` | 真实 61 秒过期；health EXPIRED/degraded；同 session 三类调用零 endpoint/source 增量；真实 recheck 恢复 enabled/healthy，实际请求重新准入 |

五表是 `of_edit_log`、`of_edit_log_entry`、`of_edit_history`、`of_action_command`、`of_webhook_outbox`，SQLite 使用专用 `support/security_evidence.rs`，PG 复用现有 counts。

拒绝窗口额外比较上述五表及所有 `of_obj_*` / `of_lnk_*` 完整业务行快照，防止计数漏掉 UPDATE。正常 race / stale 后正常提交断言仅 Flight 新增一行、`of_version = 1`、`status = DELAYED`、`delay_minutes = 5`，其他 overlay 原样；后续显式 replay 完整快照不变。PG 通过 JSONB 聚合采集全部行；SQLite 逐列保留 NULL/文本值并按 rowid 排序。所有新场景 HTTP client 使用 30 秒 timeout，session/rpc 另有请求 deadline。E1 每轮 source 请求及最后正常提交前有 10 秒 health readiness 等待，避免 reload 后 drift Pending 干扰 definition fence。

## 验证记录与待运行命令

- 已运行 `rustfmt --edition 2024 --config skip_children=true`，限定上述 5 个 test 文件及专用 helper，exit 0。这只验证格式/语法解析，不替代编译或行为测试。
- 未运行 Cargo，未启动服务，无 RED/GREEN 结论；当前只有代码证据。既有行为补证据按已批准设计不人为制造 RED。

```text
cargo test -p ontoforge-server --locked --test http_action_safety --test mcp_actions --test mcp_contract --test mcp_l2_defaults -- --nocapture
cargo test -p ontoforge-server --locked --test postgres_http_it pg_http_definition_stale_barrier_has_zero_business_writes -- --ignored --exact --nocapture
cargo test -p ontoforge-server --locked --test postgres_http_it pg_mcp_rest_same_command_race_one_persistent_receipt -- --ignored --exact --nocapture
```

PG 入口需要 `ONTOFORGE_PG_URL` 与 `ONTOFORGE_TRINO_URL`，Trino 已加载 aviation source fixtures，PG 账号可创建/清理隔离 schema。每例通过现有 run_case 创建两 store 并启动真实 host。ignored 未运行不能计为通过。L2 受控 fixture 启动真实 HTTP endpoint 并使用生产 TrinoEngine / ACL attestation / MCP gate，不替代真实部署 ACL 全服务门禁。

## 2026-11-03 动态运行

Cargo ownership 经主会话从 SDK worker 交接，所有命令 `CARGO_BUILD_JOBS=2`，PowerShell 实际运行。

| 命令 / 日志 | 结果 |
|---|---|
| 上述四 targets 联跑；`target/phase7-server-security-tests.log` | exit 101；HTTP 3 passed；Actions 9 passed / 1 failed。失败为测试误将 MCP `structuredContent` 与裸 REST receipt 对比 |
| `cargo test -p ontoforge-server --locked --test mcp_actions --test mcp_contract --test mcp_l2_defaults -- --nocapture`；`target/phase7-server-security-retest.log` | exit 101；Actions 10 passed、Contract 16 passed、L2 defaults 7 passed / 1 failed。expiry 用例真实运行 61 秒并通过 |
| `cargo clippy -p ontoforge-server --locked --test http_action_safety --test mcp_actions --test mcp_contract --test mcp_l2_defaults --test postgres_http_it -- -D warnings`；`target/phase7-server-security-clippy.log` | exit 0，16.93s；包含 PG 测试编译，未执行 PG |
| owned 六文件 `rustfmt --edition 2024 --check --config skip_children=true` 与 `git diff --check` | exit 0 |

测试 shape 修正已读取 `mcp.rs` 的 `json!({"receipt": ...})` 生产合同，将 SQLite / PG race 比较路径统一为 `structuredContent.receipt`，仍严格比较完整 receipt。主会话 CC 复核 AGREE：`target/phase7-receipt-test-claude-review.json`。此失败不是生产行为 RED。

### E6 可归因生产 RED

`mcp_l2_same_session_revoke_defaults_and_qualified_zero_send` 的 endpoint 全请求计数断言失败：实际 16、预期 15。此前 toolset / SQL level / binding 撤权与恢复已完成；清空 allowlist 后 `get_table_schema` 在目标权限检查前先执行 `gate()`。`gate()` 调用 `TrinoEngine::status()`，发送 trace `ontoforge-status` 的同步 `SELECT 1`，之后 `resolve_table` 才拒绝。

该 SQL 属于被测调用引发的实际请求，不是后台 monitor，不允许以 tag 过滤消除。主会话已确认生产顺序缺口，待 CC 综合批准最小修复：先纯 scope / policy / 参数 / allowlist 校验，再 availability，最后 audit / 执行；发送侧 fresh proof 保持。当前没有修改生产代码。

真实 PG / Trino 服务尚未启动，两个新增 PG ignored 入口仍未执行，交最终 fresh `ci/server-services.sh verify` 验收。当前不标 E6 整体 passed。

## E6 修复执行计划

主会话已批准 `07-E6-PREFLIGHT-FIX.md`，扩大 ownership 至 `src/mcp_l2.rs`，并重新授予唯一 Cargo ownership。已读取 `superpowers:test-driven-development` 与 writing-good-tests 参考；战略/Phase/brainstorming 按已批准 bug 修复合同跳过。

1. 追加非法 SQL / EXPLAIN format / 缺 defaults / 越界 defaults / qualified identifier 的真实 adapter 测试，记录 status / execute / describe / audit 调用零值；先跑旧实现 RED。
2. `gate` 仅返回 scope/policy；独立 availability 放入 `run_sql` reserve 前与 metadata resolve 后，保留发送侧 proof enforcement。
3. 运行 L2 defaults 与 L2 catalog 回归（包括真实 61 秒 expiry），限定 fmt 与 server Clippy；记录 GREEN 后归还 Cargo。

## E6 修复结果

生产最终 blob：`src/mcp_l2.rs` = `667102e12decd2fae4381579202df92c71d0c3e2`。`gate()` 变为纯 policy；`ensure_available()` 集中在 `run_sql` 的 audit reserve 前、metadata `resolve_table` 后，六工具参数和权限拒绝不发健康请求。既有发送侧 fresh attestation 保持。测试最终 blob：`mcp_l2_defaults.rs` = `186e4199883eaa1c93fadb766f2c74631308196b`。

| 阶段 | 实际命令与日志 | 结果 |
|---|---|---|
| RED | `cargo test -p ontoforge-server --locked --test mcp_l2_defaults mcp_l2_local_rejections_skip_status_execute_and_audit -- --exact --nocapture`；`target/phase7-e6-preflight-red.log` | exit 101；真实拒绝后 status_calls 10 != 0；0 passed / 1 failed |
| GREEN | `cargo test -p ontoforge-server --locked --test mcp_l2_defaults --test mcp_l2 -- --nocapture`；`target/phase7-e6-preflight-green.log` | exit 0；2 + 9 = 11 passed，0 failed / ignored，包含真实 61.38 秒 expiry/recovery、revoke、cancel/audit |
| 最终补矩阵 | 同 RED 命令；`target/phase7-e6-six-tools-green.log` | exit 0；1 passed，新增缺 SqlRaw / 缺 Raw 两种 scope 各六工具的 Denied、零 status/execute/describe/audit 断言；仅测试扩展，生产 blob 不变 |
| Clippy | `cargo clippy -p ontoforge-server --locked --all-targets -- -D warnings`；`target/phase7-e6-preflight-clippy.log` | exit 0，43.45s |
| fmt / diff | owned 7 文件 `rustfmt --edition 2024 --check --config skip_children=true`；`target/phase7-e6-preflight-fmt.log`，以及 `git diff --check` | exit 0，fmt 无诊断 |

非法 SQL、非法 EXPLAIN format、缺 defaults、越界 defaults、qualified 名称均覆盖本地拒绝零副作用。此次未重复与 L2 无关的 Action / SDK suite；E1/E2 之前的 29 项通过证据不变。新 PG ignored 仍未真实运行。Cargo 已交还主会话，未 commit/push，等待主会话独立复签与最终服务验收。
