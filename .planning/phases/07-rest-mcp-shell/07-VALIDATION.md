---
phase: "07"
slug: "rest-mcp-shell"
status: in_progress
nyquist_compliant: false
wave_0_complete: true
created: "2026-10-29"
updated: "2026-11-03"
---

# Phase 7 Validation Strategy

## 当前状态与证据边界

本次为 State A 回溯审核：16 份 PLAN、15 份 SUMMARY 与当前测试源码交叉核对，32 个 task 均已有自动化入口；07-14 的证据暂以 `reviews/07-14-IMPLEMENTATION.md` 为准。当前仍是 `in_progress`，不得用旧 head 的 GREEN 关闭本轮缺口或最终集成门。

证据真相源：每个 PLAN 定义行为与 `fails_when`；对应 SUMMARY 记录已验收基线；本轮 T24、Server、SDK、Bridge 的动态日志及其 `reviews/*-EVIDENCE.md` 记录新增覆盖。本文只维护映射、证据边界和未完成门，不复制实施历史。COVERED 表示所列行为已有源码断言和实跑证据，后续代码仍受最终 head 门约束；PARTIAL 表示存在明确行为缺口或新增测试待实跑，不能按名称或编译成功判绿。

| 证据 | 已验证范围 | 当前限制 |
|---|---|---|
| B：`07-01` 至 `07-13`、`07-15`、`07-16` SUMMARY；`07-13-SUMMARY.md` | CI `41638673666` 的 head `10175f6` 四必需 job GREEN；早期本地 workspace 1373 passed / 0 failed / 113 ignored 仅作历史基线 | 不是当前工作树或最终 head 的 workspace 证明；ignored 必须由服务门单独执行 |
| R：`reviews/07-14-IMPLEMENTATION.md` | native CI `41648281446` 三平台和 aggregate success；source head `e6ae310`，实际 archive manifest commit `f315d465`；publish skipped | T24/E6 后的最终 head 需重新构建及 smoke；旧三平台不再标“未运行” |
| T：`reviews/07-T24-IMPLEMENTATION.md`、`reviews/07-T24-SECURITY-RECHECK.md` | `target/phase7-t24-green-final.log` session 12/12；`target/phase7-t24-transport.log` transport 11/11；真实 manager 容量、自动回收、取消窗口、token 过期已覆盖 | 仅该冻结改动和定向 suite，非完整 workspace |
| S：`reviews/07-SERVER-SECURITY-EVIDENCE.md` | HTTP safety 3/3、Actions 10/10、MCP contract 16/16；E6 修复后 L2 defaults 9/9 + L2 core 2/2；六工具拒绝补矩阵 1/1 | 两个新增 PG ignored 尚待真实服务；旧失败日志已由后续 GREEN 对应覆盖，不能把首轮 exit 101 当成功 |
| K：`reviews/07-SDK-SECURITY-EVIDENCE.md` | Rust contract 最终 9/9；TS contract 17/17；双主体、撤权、在途取消、u64、丢响应不重试与显式 replay | TS 使用已有 debug binary 副本 SHA256 `859DA06A4DA61CCD9EA9B1C81B45060CDAF484DA04C622ABB33A4BD8B68478D7`；必须用最终源码新 binary 复验 |
| E3：`reviews/07-BRIDGE-SECURITY-EVIDENCE.md` | 新增真实 session 关闭、发送前断连、提交后丢响应三项源码与持久行断言已就绪，跨 crate check 已成功 | 首轮 5 passed / 3 failed（协议假设与 receipt fixture 错误）；按实际 Initialize/session 修正中，尚无最终 GREEN；N1/N2 进行中 |

## Test Infrastructure

| Property | Value |
|---|---|
| Framework | Rust cargo test、insta、proptest；Node 内置 `node:test`；PowerShell native artifact smoke |
| Config / entry | `Cargo.toml`、`.github/workflows/ci.yml`、`.github/workflows/release.yml`；`clients/typescript/package.json` 已有 build/typecheck/test:contract/check:generated；生成器为 `tools/openapi-gen/package.json` |
| Quick command | 按 task 使用下表非空 target/filter；server 常规集合可用 `cargo test -p ontoforge-server --locked` |
| Full suite | `cargo test --workspace --locked -- --test-threads=1`；另跑 `bash ci/server-services.sh verify` 与三平台 release workflow |
| 静态门 | `cargo fmt --all --check`、workspace all-targets Clippy、embedded-pg Clippy、TLS 依赖禁用门；以 `.github/workflows/ci.yml` 为准 |
| 实测反馈 | T24 session 1.80s、transport 7.32s；Rust SDK 0.46s、TS 1.874s；L2 defaults 61.38s，包含真实 60s proof TTL；native 首次 Windows release 编译 15m28s。编译时间与测试时间分开记录，不推算当前完整矩阵耗时 |

## Sampling Rate

- 新业务缺陷先可归因 RED，再 GREEN；已有实现的补证据允许直接 GREEN，编译错误不冒充行为 RED。
- 每个 task 执行下表目标及 PLAN `fails_when`，指定行为未执行、0 tests、required case ignored/skipped、exit 非零均不能通过。历史测试重命名必须映射实际行为，禁止继续要求不存在的旧名称。
- 每个 wave 使用受影响 target；最终冻结 head 执行完整 workspace、服务、SDK、native 与审查门。真实 PG/Trino、61 秒 TTL 不用 mock 或 `--no-run` 替代。
- `07-TEST-MATRIX.md` 与各 PLAN 是行为合同；本次查阅源码断言，不以 SUMMARY 自报或函数名单独证明覆盖。

## Per-Task Verification Map

16 份 PLAN 共 32 个 task。表内“基线”引用对应 `07-NN-SUMMARY.md` 及 B；新增证据 T/S/K/E3/R 见上表。所有文件均已创建。下文 N1/N2/N3 是额外审核出的实际覆盖缺口，G1 至 G6 是尚待运行或复签的门，两者分开跟踪。

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---|---|---|---|---|---|---|---|---|---|
| 07-01-1 | 07-01 | 1 | API-01,API-03,SCOPE-07 | T-07-01,T-07-02,T-07-03 | 真实 HTTP PK 值、拒绝伪造上下文/坏 token、shutdown 后 request_id/def_version 持久 audit；source barrier 期间另一 store reload 成功 | RED→GREEN contract/integration | `cargo test -p ontoforge-server --lib --locked tracer_ -- --nocapture` | `crates/ontoforge-server/tests/unit/tracer.test.rs`，由 lib 引用 | COVERED：已验证基线；最终 head 待 G5 |
| 07-01-2 | 07-01 | 1 | API-01,API-03,SCOPE-07 | T-07-01,T-07-02,T-07-03 | 真实 listener 认证/关闭与端口释放；坏 config/address/占用端口非零；DROP audit 表使 drain 非零 | RED→GREEN contract/integration | `cargo test -p ontoforge --lib --locked serve_ -- --nocapture` | `crates/ontoforge/tests/unit/serve.test.rs`，由 serve/lib 引用 | COVERED：已验证基线；最终 head 待 G5 |
| 07-02-1 | 07-02 | 2 | SCOPE-07,API-01,MCP-06 | T-07-04,T-07-05,T-07-06 | 手工持有共同 metadata lock，read/write 同时 Pending；reload/scope_apply 只能见完整世代；snapshot 返回后另一 store reload 不阻塞；PG 同 helper | RED→GREEN contract/integration | `cargo test -p ontoforge-storage --test request_snapshot --locked -- --nocapture` | crates/ontoforge-storage/tests/request_snapshot.rs（已存在） | COVERED：已验证基线；最终 head 待 G5 |
| 07-02-2 | 07-02 | 2 | SCOPE-07,API-01,MCP-06 | T-07-04,T-07-05,T-07-06 | 固定定义 submit/validate stale、CAS/guard 零写；replay/current grant/original receipt/time；history 新旧定义、当前 scope 遮蔽 | RED→GREEN contract/integration | `cargo test -p ontoforge-action --test frozen_definition --test idempotency --locked -- --nocapture`<br>`cargo test -p ontoforge-storage --test frozen_history --test history_masking --locked -- --nocapture` | crates/ontoforge-action/tests/frozen_definition.rs（已存在）<br>crates/ontoforge-storage/tests/frozen_history.rs（已存在） | COVERED：已验证基线；最终 head 待 G5 |
| 07-03-1 | 07-03 | 3 | API-01,API-03 | T-07-07,T-07-08,T-07-09 | 真实 REST 全 L0、去重/排序/null/keyset；source barrier 内 reload/revoke 后在途保留旧版本，下一请求隐藏且 audit 对账 | RED→GREEN contract/integration | `cargo test -p ontoforge-server --lib --locked rest_ -- --nocapture` | `crates/ontoforge-server/tests/unit/rest.test.rs`，由 lib 引用 | COVERED：已验证基线；最终 head 待 G5 |
| 07-03-2 | 07-03 | 3 | API-01,API-03 | T-07-07,T-07-08,T-07-09 | 真实框架 JSON/unknown/UTF8/413/404/405 拒绝；16 并发 request_id；rows/UTF8 bytes/deadline/disconnect/concurrency/audit 故障边界 | RED→GREEN contract/integration | `cargo test -p ontoforge-server --test http_contract --locked -- --nocapture` | `crates/ontoforge-server/tests/http_contract.rs` → `tests/unit/http_contract.test.rs` | COVERED：已验证基线；最终 head 待 G5 |
| 07-04-1 | 07-04 | 4 | API-01,API-03,MCP-06,SCOPE-07 | T-07-10,T-07-11,T-07-12 | 真实 validate/submit/result/history；eventId/committedAt 相同，maskedChanges 留字段名不泄漏值；完整 history 分页另见 history_pagination | RED→GREEN contract/integration | `cargo test -p ontoforge-server --test http_actions --locked -- --nocapture` | crates/ontoforge-server/tests/http_actions.rs（已存在）<br>crates/ontoforge-server/tests/support/mod.rs（已存在） | COVERED：已验证基线；最终 head 待 G5 |
| 07-04-2 | 07-04 | 4 | API-01,API-03,MCP-06,SCOPE-07 | T-07-10,T-07-11,T-07-12 | 真实同 command race、当前授权拒绝；source barrier 后第二 store reload，submit/validate 均 DEFINITION_STALE、五表及业务行零变更且不重试 | RED→GREEN contract/integration | `cargo test -p ontoforge-server --test http_action_safety --locked -- --nocapture` | crates/ontoforge-server/tests/http_action_safety.rs（已存在） | COVERED：S 本轮 HTTP safety 3/3；PG 另见 13-1 |
| 07-05-1 | 07-05 | 5 | API-02,API-01,API-03 | T-07-13,T-07-14,T-07-15 | 静态 render 与受控 OpenAPI 完全一致；3.1 paths/security、recursive union/unknown fields、Unicode/整数、完整 receipt/history/error/health schema | RED→GREEN contract/integration | `cargo test -p ontoforge-api --test openapi_snapshot --locked -- --nocapture` | crates/ontoforge-api/tests/openapi_snapshot.rs（已存在） | COVERED：已验证基线；最终 head 待 G5 |
| 07-05-2 | 07-05 | 5 | API-02,API-01,API-03 | T-07-13,T-07-14,T-07-15 | 三个无 config/store 的并发导出字节一致；缺父目录返回非零；render snapshot 无漂移 | RED→GREEN contract/integration | `cargo test -p ontoforge --lib --locked openapi_ -- --nocapture`<br>`cargo test -p ontoforge-api --test openapi_snapshot --locked -- --nocapture` | `crates/ontoforge/tests/unit/serve.test.rs`；`crates/ontoforge-api/tests/openapi_snapshot.rs` | COVERED：已验证基线；最终 head 待 G5 |
| 07-06-1 | 07-06 | 6 | L2-04,API-01,API-03 | T-07-16,T-07-17,T-07-18 | 真实 health HTTP 200/503/recovery；占用 listener 零 monitor probe；真实 proof lifecycle 与 drift owner 由 health_trino/source_drift suites 补强 | RED→GREEN contract/integration | `cargo test -p ontoforge-server --test health_lifecycle_it --locked -- --nocapture` | crates/ontoforge-server/tests/health_lifecycle_it.rs（已存在） | COVERED：已验证基线；最终 head 待 G5 |
| 07-06-2 | 07-06 | 6 | L2-04,API-01,API-03 | T-07-16,T-07-17,T-07-18 | CLI audit drain/失败非零；真实 accepted Action、定向读取消、SSE、stuck deadline/owned task failure 由 mcp_transport 与 health_trino_it 补强 | RED→GREEN contract/integration | `cargo test -p ontoforge --test cli_shutdown --locked -- --nocapture --test-threads=1`<br>`cargo test -p ontoforge-server --test mcp_transport --locked -- --nocapture`；真实环境见 13-2 | crates/ontoforge/tests/cli_shutdown.rs（已存在） | COVERED：已验证基线；最终 head 待 G5 |
| 07-07-1 | 07-07 | 7 | MCP-01,MCP-02,MCP-03,MCP-04,MCP-05,MCP-06,MCP-07,MCP-08,SCOPE-07 | T-07-19,T-07-20,T-07-21 | 真实 core/full 请求参数、metadata/history REST 等价；同 session source barrier 跨 reload/revoke，下一调用零 source 且旧 def_version audit | RED→GREEN contract/integration | `cargo test -p ontoforge-server --test mcp_contract --locked -- --nocapture` | crates/ontoforge-server/tests/mcp_contract.rs（已存在）<br>crates/ontoforge-server/tests/support/mod.rs（已存在） | COVERED：S 本轮 contract 16/16 |
| 07-07-2 | 07-07 | 7 | MCP-01,MCP-02,MCP-03,MCP-04,MCP-05,MCP-06,MCP-07,MCP-08,SCOPE-07 | T-07-19,T-07-20,T-07-21 | 真实 nested params/schema/defaults；REST/MCP 同 command barrier race 完整 receipt 相同且五表各一次；旧 schema 进入 fence、新 schema 下一请求生效 | RED→GREEN contract/integration | `cargo test -p ontoforge-server --test mcp_actions --locked -- --nocapture` | crates/ontoforge-server/tests/mcp_actions.rs（已存在） | COVERED：S 本轮 Actions 10/10；底层 fence/CAS/guard 见 02-2 |
| 07-08-1 | 07-08 | 8 | MCP-01,MCP-06,MCP-07,MCP-08,L2-04,SCOPE-07 | T-07-22,T-07-23,T-07-24 | 六工具名字/schema/allowlist；同 session 撤权/60s proof expiry 拒绝零 endpoint/source 请求，health 一致且恢复准入 | RED→GREEN contract/integration | `cargo test -p ontoforge-server --test mcp_l2 --locked -- --nocapture` | crates/ontoforge-server/tests/mcp_l2.rs（已存在） | COVERED：S 本轮 L2 2/2 + defaults 9/9；真实部署复验 G3 |
| 07-08-2 | 07-08 | 8 | MCP-01,MCP-06,MCP-07,MCP-08,L2-04,SCOPE-07 | T-07-22,T-07-23,T-07-24 | 真实 HTTP manager 容量≤256、TTL/DELETE/取消/初始化清理、host 隔离；过期签名 token 的 POST/GET/DELETE 401；定向取消、SSE、Action 不重放 | RED→GREEN contract/integration | `cargo test -p ontoforge-server --test mcp_transport --locked -- --nocapture`<br>`cargo test -p ontoforge-server --lib --locked state::session_tests -- --nocapture` | `crates/ontoforge-server/tests/mcp_transport.rs`；`crates/ontoforge-server/tests/unit/mcp_sessions.test.rs` | COVERED：T session 12/12、transport 11/11 |
| 07-09-1 | 07-09 | 10 | MCP-01,MCP-06,MCP-07,SCOPE-07 | T-07-25,T-07-26,T-07-27 | bridge_configuration_disables_reinit_and_rejects_url_credentials；bridge_configuration_bounds_concurrency_and_redacts_errors；真实双 ID 取消隔离/EOF | RED→GREEN contract/integration | `cargo test -p ontoforge --lib --locked bridge_ -- --nocapture` | `crates/ontoforge/src/mcp.rs`；`crates/ontoforge/tests/cli_stdio.rs` | PARTIAL：配置单测/并发取消基线；真实 redirect 拒绝缺口 N2 in_progress |
| 07-09-2 | 07-09 | 10 | MCP-01,MCP-06,MCP-07,SCOPE-07 | T-07-25,T-07-26,T-07-27 | 真实 stdio query/取消/EOF、absent server；新 live_stdio_closed_upstream_session_never_reinitializes_or_replays、live_stdio_disconnected_before_forward_never_retries_or_commits、live_stdio_lost_commit_response_requires_explicit_replay 首轮 fixture 失败后修正；N1/N2 补测 in_progress | RED→GREEN contract/integration | `cargo test -p ontoforge --test cli_stdio --locked -- --nocapture --test-threads=1` | crates/ontoforge/tests/cli_stdio.rs（已存在） | PARTIAL：E3 三项修正后待 GREEN（G1）；成功链 N1、在线认证/redirect N2 补测 in_progress |
| 07-10-1 | 07-10 | 7 | API-04 | T-07-28,T-07-29,T-07-30 | 全部真实 HTTP 读端点、L2 health；双有效主体 metadata 隔离、实际 scope_apply 撤权、两个 source barrier 请求只取消一个 | RED→GREEN contract/integration | `cargo test -p ontoforge-client --test contract --locked -- --nocapture` | crates/ontoforge-client/tests/contract.rs（已存在）<br>crates/ontoforge-client/tests/support/mod.rs（已存在） | COVERED：K 本轮 Rust 9/9 |
| 07-10-2 | 07-10 | 7 | API-04 | T-07-28,T-07-29,T-07-30 | 真实 Action/history/result/replay/分页/数值；发送前断连和提交后丢响应计数1，完整持久表和业务版本一次提交，显式 replay 原 receipt | RED→GREEN contract/integration | `cargo test -p ontoforge-client --test contract --locked -- --nocapture` | crates/ontoforge-client/tests/contract.rs（已存在） | COVERED：K 本轮 Rust 9/9 |
| 07-11-1 | 07-11 | 7 | API-05 | T-07-31,T-07-32,T-07-33 | 生成 types/strict typecheck/真实 HTTP 读取；双有效主体与 SQLite 实际撤权；响应在途 AbortSignal 隔离，未宣称服务端执行取消 | RED→GREEN contract/integration | `npm --prefix clients/typescript run test:contract`<br>`npm --prefix clients/typescript run typecheck`<br>`npm --prefix clients/typescript run check:generated` | clients/typescript/tests/contract.test.mjs（已存在） | COVERED：K 本轮 TS 17/17；最终 binary 复验 G4 |
| 07-11-2 | 07-11 | 7 | API-05 | T-07-31,T-07-32,T-07-33 | 公开 query u64 max 与 codec 不舍入、unsafe number 发送前拒绝、decimal 字符串；故障 proxy 单次发送/完整持久行与显式 replay；全部端点/诊断合同 | RED→GREEN contract/integration | `npm --prefix clients/typescript run test:contract`<br>`npm --prefix clients/typescript run check:generated`<br>`npm --prefix clients/typescript run typecheck` | clients/typescript/tests/contract.test.mjs（已存在） | COVERED：K 本轮 TS 17/17；最终 binary 复验 G4 |
| 07-12-1 | 07-12 | 11 | DX-01,DX-02 | T-07-34,T-07-35,T-07-36 | 真实 dev 子进程重启 receipt/history 保留、显式 reset/token 更新；OS lock/崩溃恢复/危险目录与 Unix 权限防护 | RED→GREEN contract/integration | `cargo test -p ontoforge --test dev_lifecycle --locked -- --nocapture --test-threads=1` | crates/ontoforge/tests/dev_lifecycle.rs（已存在） | COVERED：已验证基线；最终 head 待 G5 |
| 07-12-2 | 07-12 | 11 | DX-01,DX-02 | T-07-34,T-07-35,T-07-36 | 隔离 cwd binary 启动；完整 query→Action→history→restart→reset 由 release-smoke 实际 archive 验证，不把启动存活等同完整调用 | RED→GREEN contract/integration | `cargo test -p ontoforge --test dev_lifecycle --locked -- --nocapture --test-threads=1` | crates/ontoforge/tests/dev_lifecycle.rs（已存在） | COVERED：已验证基线；最终 head 待 G5 |
| 07-13-1 | 07-13 | 11 | API-01,API-03,MCP-01,MCP-06,MCP-07,MCP-08,L2-04,SCOPE-07 | T-07-37,T-07-38,T-07-39 | 真实 PG snapshot/revoke/history/原子写；新增 definition fence 与 REST/MCP race 尚待实跑；真实 Trino 长读短锁、六工具/四 EXPLAIN/ACL | RED→GREEN contract/integration | `cargo test -p ontoforge-server --test postgres_http_it --locked -- --ignored --nocapture --test-threads=1`<br>`cargo test -p ontoforge-storage --test request_snapshot --locked -- --ignored --nocapture --test-threads=1`<br>`cargo test -p ontoforge-server --test trino_http_it --locked -- --ignored --nocapture --test-threads=1` | crates/ontoforge-server/tests/postgres_http_it.rs（已存在）<br>crates/ontoforge-server/tests/trino_http_it.rs（已存在）<br>crates/ontoforge-server/tests/support/mod.rs（已存在） | PARTIAL：B 真实服务基线；新增 PG 两项待跑 G2/G3 |
| 07-13-2 | 07-13 | 11 | API-01,API-03,MCP-01,MCP-06,MCP-07,MCP-08,L2-04,SCOPE-07 | T-07-37,T-07-38,T-07-39 | 真实 health proof 61s expiry/recovery、core failure、Action dependency drain/cancel/audit；CI named 非零发现、SDK、credential scan 与 cleanup | RED→GREEN contract/integration | `bash ci/server-services.sh verify` | crates/ontoforge-server/tests/health_trino_it.rs（已存在） | COVERED：B 旧 head 完整服务 CI；最终 head 待 G3/G5 |
| 07-14-1 | 07-14 | 12 | DX-04,DX-01,DX-02,API-04,API-05 | T-07-40,T-07-41,T-07-42 | 真实三 native binary/archive/manifest/checksum；source HEAD 与 manifest commit 区分，错 commit/target fail closed | artifact/process smoke | `pwsh -File ci/release-package.ps1 -Target x86_64-pc-windows-msvc -OutDir target/release-dist` | ci/release-package.ps1（已存在） | COVERED：R 三 native 旧 head 已绿；最终 head 待 G6 |
| 07-14-2 | 07-14 | 12 | DX-04,DX-01,DX-02,API-04,API-05 | T-07-40,T-07-41,T-07-42 | 真实 archive 隔离 cwd query/Action/history/restart/reset/stdio query/清理；坏 checksum/commit/version/target/额外文件拒绝；publish skipped | artifact/process smoke | `pwsh -File ci/release-smoke.ps1 -ArtifactDir target/release-dist -ExpectedTarget x86_64-pc-windows-msvc` | ci/release-smoke.ps1（已存在） | COVERED：R archive/hash/完整 smoke 旧 head 已绿；最终 head 待 G6 |
| 07-15-1 | 07-15 | 6 | MCP-07 | T-07-43,T-07-44,T-07-45 | 四 ExplainType 精确 TYPE SQL 与旧 DISTRIBUTED wrapper；结构化 table/defaults/qualified/quoted 名称与注入拒绝 | RED→GREEN contract/integration | `cargo test -p ontoforge-engine --test l2_explain --test l2_gate --locked -- --nocapture` | crates/ontoforge-engine/tests/l2_explain.rs（已存在） | COVERED：已验证基线；最终 head 待 G5 |
| 07-15-2 | 07-15 | 6 | MCP-07 | T-07-43,T-07-44,T-07-45 | 四格式成功输出与单次 audit；旧 wrapper 拒绝和取消已有基线；原 all_types_timeout_cancel_concurrency 仅顺序成功请求，N3 待补 | RED→GREEN contract/integration | `cargo test -p ontoforge-query --test l2_execute --locked -- --nocapture` | crates/ontoforge-query/tests/l2_execute.rs（已存在） | PARTIAL：四格式成功路径基线；timeout/cancel/concurrency 缺口 N3 |
| 07-16-1 | 07-16 | 9 | MCP-07,SCOPE-07 | T-07-46,T-07-47 | 可信 catalog/schema defaults、qualified>explicit>default；真实 listener 的 metadata SQL 使用部署 defaults | RED→GREEN contract/integration | `cargo test -p ontoforge-server --test mcp_l2_defaults --locked mcp_l2_optional_context_defaults_and_qualified_override -- --nocapture` | crates/ontoforge-server/tests/mcp_l2_defaults.rs（已存在） | COVERED：S 本轮 defaults 9/9 |
| 07-16-2 | 07-16 | 9 | MCP-07,SCOPE-07 | T-07-46,T-07-47 | 缺 context/越界 allowlist/非法 SQL/format/qualified 在 availability 前拒绝；六工具缺 scope 拒绝零 status/execute/describe/audit；E6 RED→GREEN | RED→GREEN contract/integration | `cargo test -p ontoforge-server --test mcp_l2_defaults --locked -- --nocapture` | crates/ontoforge-server/tests/mcp_l2_defaults.rs（已存在） | COVERED：S 本轮 defaults 9/9 + 六工具拒绝补矩阵 1/1 |

## 尚未覆盖的原计划行为

以下为源码核对确认的独立事实，不包含已有测试仅待运行的项。三项均已由主会话亲核并经 Claude Code AGREE/REFINE，当前为 `in_progress`，尚未 GREEN。N1/N2 的合同指针为 `reviews/07-E3-BRIDGE-DESIGN.md` 的“实码更正”，N3 为 `reviews/07-EXPLAIN-EVIDENCE-PLAN.md`。修复限于原 PLAN 合同；不得将缺口改列 manual-only 来取得绿色。

| ID | Task / 合同 | 当前证据不足 | 最小补齐与判据 |
|---|---|---|---|
| N1 | 07-09-2：真实 stdio describe/query/Action/dryrun/history | `crates/ontoforge/tests/cli_stdio.rs` 已无原 `stdio_real_process_describe_query_action_dryrun_history`；新 E3 覆盖 Action/replay，取消测试只调用 query；`ci/release-smoke.ps1` 的 stdio 片段只有 initialize/query | 复用真实 CLI/host，补双有效主体 scope/list/describe/query、成功 validateOnly 零持久变更、Action 后 history 实值与 receipt/event/time；与 REST 等价，stdout 逐行合法协议，EOF 退出及 stderr 无 secret |
| N2 | 07-09-1/07-09-2：禁 redirect、真实 bad token 拒绝 | `cli_stdio.rs:354` 仅请求未监听端口和含 credentials URL；命名中的 redirect 不是实际 3xx。`src/mcp.rs` 单测只检查 URL/config，E3 新增场景也未触发 3xx 或在线坏 bearer | 在线 host + 坏 token 真实拒绝；受控 HTTP 302 到 trap，trap 请求数为 0；有界退出、stderr 无 token。不要求新增 transport 实现 |
| N3 | 07-15-2：四 ExplainType 的 timeout/cancel/concurrency 上限与单次 audit | `crates/ontoforge-query/tests/l2_execute.rs:656` 的 `l2_explain_all_types_timeout_cancel_concurrency` 仅逐个返回成功和 audit.len=4；`:599` 也是成功路径。旧 wrapper 取消和 execute timeout 无法证明该名称承诺的 typed 分支 | 在既有 fixture 对四格式参数化施加 delay/取消/满并发 permit，断言有界超时/取消/拒绝、无额外发送或重复 audit；复用入口并实际运行同 target。只补原 PLAN 明列的三类行为 |

## 当前待运行与复签门

| Gate | 命令 / 执行入口 | 关闭标准 |
|---|---|---|
| G1 | `cargo test -p ontoforge --test cli_stdio --locked -- --nocapture --test-threads=1` | E3 三项及 N1/N2 补齐后的全部真实子进程用例非零 GREEN；记录源版本、日志和退出码 |
| G2 | `cargo test -p ontoforge-server --test postgres_http_it --locked -- --ignored --nocapture --test-threads=1` | 既有两项和新增 `pg_http_definition_stale_barrier_has_zero_business_writes`、`pg_mcp_rest_same_command_race_one_persistent_receipt` 均实际执行，至少四项非零、无 skip；需真实 PG/Trino fixture |
| G3 | `bash ci/server-services.sh verify`，以及 CI `phase7-real-services` | 最终 head 的 PG/snapshot/Trino/health/ACL/source drift/Rust/TS named suites、生成/typecheck/build、credential scan、cleanup 全部通过；需记录实际 binary/commit，不能沿用 10175f6 |
| G4 | 最终源码 `cargo build -p ontoforge --locked` 后，设置该 binary 的 `ONTOFORGE_BIN`；`npm --prefix clients/typescript run test:contract`、`run typecheck`、`run check:generated` | 当前 17 项 TS 合同全部执行、生成无 diff、strict 无 diagnostic；若 G3 已使用同一新 binary 完成这些门，不重复运行 |
| G5 | `cargo test --workspace --locked -- --test-threads=1`；`.github/workflows/ci.yml` 的 Windows/Ubuntu、integration、phase7-real-services 必需 job；fmt/Clippy/embedded-pg/TLS | 最终冻结 head 证据齐全；N3 用 `cargo test -p ontoforge-query --test l2_execute --locked -- --nocapture` 补齐且 GREEN；独立 review/security 复签覆盖 T24/E6 与新增行为 |
| G6 | `.github/workflows/release.yml` 的三个 native job 与 aggregate；打包/安装命令见 14-1/14-2 | 最终 head 对应 archive/manifest/checksum 与完整安装 smoke 均通过，记录 source/实际 checkout commit；发布保持未执行，不把 publish skipped 当 required 验证失败 |

Bridge 首轮日志 `target/phase7-bridge-security-cli.log` 为 5 passed / 3 failed，原因与修订合同见 E3 evidence/design。生产 bridge 实际使用 Initialize/session；不能沿用初稿的默认 modern/discover 假设，receipt 取 `structuredContent.receipt`。这是 fixture/协议假设更正，未据此宣称生产 bug。

本轮已经读取的动态日志：`target/phase7-server-security-tests.log`（HTTP 3 passed，随后 Actions 首轮失败）、`target/phase7-server-security-retest.log`（Actions 10、contract 16 passed，随后 E6 失败）、`target/phase7-e6-preflight-green.log`（L2 2+9 passed）、`target/phase7-e6-six-tools-green.log`（1 passed）、`target/phase7-sdk-runtime/rust-contract-r2.log`（9 passed）、`target/phase7-sdk-runtime/ts-contract.log`（17 passed）。失败及后继修复归因保留在各 lane evidence；不将重叠用例相加为唯一总数。

## Wave 0 Requirements

- [x] 真实 listener/auth/audit fixture 与非空 server contract suites 已存在。
- [x] storage/action snapshot race fixtures 已存在，SQLite/PG 共用锁行为合同。
- [x] OpenAPI、Rust/TS clients 的生成与 contract 入口已存在；TS 不再是“待创建”。
- [x] CLI 子进程、health/shutdown 与 release package/smoke 入口已存在。

`wave_0_complete: true` 只表示基础设施齐备，不表示 N1/N2/N3 或最终门已通过。未运行的 ignored 与新测试不得按源码存在标绿。

## Manual-Only Verifications

无纯人工项、无已豁免项。三平台使用 native CI 自动验收。当前缺口均可自动化，继续归入 N/G 清单，不转为 manual-only。Release upload、npm publish、PR merge 不属于本轮已授权验证的执行项。

## Validation Sign-Off

- [x] 32 个 task 均有可执行 automated verify，现有文件/入口已按实际布局更新。
- [x] 每个 PLAN 保有 `fails_when`，零发现或 required case 未执行不能通过；旧名称的替代与缺口已明确。
- [x] 不存在连续三个 task 无自动化入口。
- [x] PG/Trino、clients、native 历史与本轮定向日志边界可追溯。
- [x] 已记录局部实测反馈时长；不虚构最终完整矩阵时长。
- [ ] N1/N2/N3 补齐并取得动态 GREEN。
- [ ] G1 至 G6 在最终 head 对账通过，安全/代码审查复签完成。
- [ ] 正式验收更新 `nyquist_compliant`；目前保持 false。

Approval: in_progress

## Validation Audit

- 2026-11-03：State A 审核；32 task 的全部 pending/缺失路径表已更新。发现 3 个原计划覆盖事实缺口（N1/N2 同属 stdio，N3 为 typed EXPLAIN），新增 bridge 待修正后 GREEN、PG 已有测试仅待实跑，未重复列为缺失。N1/N2/N3 已批准并由既有 CLI worker 与 EXPLAIN worker 实施，均 in_progress。本文仅更新映射，不运行 Cargo/服务、不改测试或实现；最终状态由主会话据新增证据更新。
