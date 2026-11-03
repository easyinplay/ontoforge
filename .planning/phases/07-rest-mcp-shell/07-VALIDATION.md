---
phase: "07"
slug: "rest-mcp-shell"
status: passed
nyquist_compliant: true
wave_0_complete: true
created: "2026-10-29"
updated: "2026-11-04"
---

# Phase 7 Validation Strategy

## 当前状态与证据边界

2026-11-04：16 份 PLAN、32 个 task 已有实际自动化入口及动态证据。T24、E1–E7、T08 与 N1/N2/N3 的覆盖缺口已闭合；最终源码为 `adc02278b5cc55111def9a30a1c3b778485e8279`。本地fresh服务/SDK、三平台native及CI `41669982375` 四个必需job全部通过；32个task动态门已对账，因此 `nyquist_compliant: true`。GSD `07-UAT.md`已complete、13/13，uat-only passed=true（commit `975f78e`）；最终Phase goal结论由独立VERIFICATION归并。

PLAN 与 `fails_when` 是行为合同，SUMMARY 与原始日志记录实测，本文只维护映射。历史 RED 保留并由明确后继 GREEN 闭合，旧日志中的进行中状态以本表最终证据为准；不把编译成功或测试名称当动态通过。不同套件有重叠，不相加宣称唯一测试总数。

| 证据 | 最终实测范围 | 边界 / 真相源 |
|---|---|---|
| B | CI `41669982375` completed/success，Windows test、Ubuntu test、integration、phase7-real-services四必需job全部success | Ubuntu workspace1497/0/117、Windows1490/0/117（passed/failed/ignored），各149个suite result，计数见`target/phase7-ci-41669982375-{ubuntu,windows}-counts.json`；两个平台完整日志同前缀；旧CI仅历史基线 |
| R | release `41669982362` 三 native + aggregate success，publish skipped | source `adc0227`，打包 commit `b75d2949f9db1276ce07f4864a6206f7932bb11b`，两者 tree 均 `711d68e9956316d34f27aca95be87f13ab953007`；详情 `07-14-SUMMARY.md` |
| T / T08 | T24 session12/transport11；最终 admission 合集 lib25 + integration4 + L2defaults10 + transport11 = 50/50，零失败/ignored | `target/phase7-mcp-admission-green-r2.log`；冻结7文件及独立复签见 `reviews/07-T08-SECURITY-RECHECK.md` |
| S / E6 | HTTP safety3、Actions10、MCP contract16；E6 L2 core2/defaults9、六工具拒绝矩阵1；最终 L2defaults10 包含 typed limit | `reviews/07-SERVER-SECURITY-EVIDENCE.md`、`reviews/07-E6-SECURITY-RECHECK.md`；真实 PG 增量由 F 闭合 |
| K / F | 最终 fresh 实际43例：真实服务17、Rust9、TS17；`fresh_verify_exit=0`，生成/strict typecheck/build、credential scan、cleanup 全通过 | `target/phase7-final-adc0227-verify.log`；实际 commit.txt=`adc0227`，TS零failed/cancelled/skipped；cleanup owner state exited/0 |
| E3 / N1 / N2 | 真实 stdio11/11，另有 Mia 双 transport 八次拒绝定向1/1；在线坏 token、真实302 trap零连接、完整成功链及无隐式重放 | `target/phase7-bridge-security-cli-r2.log`、`target/phase7-bridge-security-mia.log`；Spec复签 `reviews/07-FINAL-SPEC-REVIEW.md` |
| N3 | query l2_execute15/15，四格式 timeout/取消隔离；HTTP四格式并发上限在 T08 最终10/10 defaults中转 GREEN | `target/phase7-explain-query-tests.log`、`target/phase7-mcp-admission-green-r2.log` |
| Review | Spec、Staff/security 与 Standards增量复签无剩余finding；48/48 threats CLOSED；末尾简化建议0 | `reviews/07-FINAL-SPEC-REVIEW.md`、`reviews/07-FINAL-STAFF-REVIEW.md`、`reviews/07-T08-SECURITY-RECHECK.md`、`reviews/07-FINAL-SIMPLIFICATION.md` |

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

16 份 PLAN 共 32 个 task。原 PLAN 行为与 automated command 保留，状态由上表最终证据派生；所有32项已由G1–G6最终动态门对账。N1/N2/N3 是已闭合覆盖缺口，G1–G6 是集成验收门。

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---|---|---|---|---|---|---|---|---|---|
| 07-01-1 | 07-01 | 1 | API-01,API-03,SCOPE-07 | T-07-01,T-07-02,T-07-03 | 真实 HTTP PK 值、拒绝伪造上下文/坏 token、shutdown 后 request_id/def_version 持久 audit；source barrier 期间另一 store reload 成功 | RED→GREEN contract/integration | `cargo test -p ontoforge-server --lib --locked tracer_ -- --nocapture` | `crates/ontoforge-server/tests/unit/tracer.test.rs`，由 lib 引用 | GREEN：已验证基线；最终四必需CI job通过 |
| 07-01-2 | 07-01 | 1 | API-01,API-03,SCOPE-07 | T-07-01,T-07-02,T-07-03 | 真实 listener 认证/关闭与端口释放；坏 config/address/占用端口非零；DROP audit 表使 drain 非零 | RED→GREEN contract/integration | `cargo test -p ontoforge --lib --locked serve_ -- --nocapture` | `crates/ontoforge/tests/unit/serve.test.rs`，由 serve/lib 引用 | GREEN：已验证基线；最终四必需CI job通过 |
| 07-02-1 | 07-02 | 2 | SCOPE-07,API-01,MCP-06 | T-07-04,T-07-05,T-07-06 | 手工持有共同 metadata lock，read/write 同时 Pending；reload/scope_apply 只能见完整世代；snapshot 返回后另一 store reload 不阻塞；PG 同 helper | RED→GREEN contract/integration | `cargo test -p ontoforge-storage --test request_snapshot --locked -- --nocapture` | crates/ontoforge-storage/tests/request_snapshot.rs（已存在） | GREEN：已验证基线；最终四必需CI job通过 |
| 07-02-2 | 07-02 | 2 | SCOPE-07,API-01,MCP-06 | T-07-04,T-07-05,T-07-06 | 固定定义 submit/validate stale、CAS/guard 零写；replay/current grant/original receipt/time；history 新旧定义、当前 scope 遮蔽 | RED→GREEN contract/integration | `cargo test -p ontoforge-action --test frozen_definition --test idempotency --locked -- --nocapture`<br>`cargo test -p ontoforge-storage --test frozen_history --test history_masking --locked -- --nocapture` | crates/ontoforge-action/tests/frozen_definition.rs（已存在）<br>crates/ontoforge-storage/tests/frozen_history.rs（已存在） | GREEN：已验证基线；最终四必需CI job通过 |
| 07-03-1 | 07-03 | 3 | API-01,API-03 | T-07-07,T-07-08,T-07-09 | 真实 REST 全 L0、去重/排序/null/keyset；source barrier 内 reload/revoke 后在途保留旧版本，下一请求隐藏且 audit 对账 | RED→GREEN contract/integration | `cargo test -p ontoforge-server --lib --locked rest_ -- --nocapture` | `crates/ontoforge-server/tests/unit/rest.test.rs`，由 lib 引用 | GREEN：已验证基线；最终四必需CI job通过 |
| 07-03-2 | 07-03 | 3 | API-01,API-03 | T-07-07,T-07-08,T-07-09 | 真实框架 JSON/unknown/UTF8/413/404/405 拒绝；16 并发 request_id；rows/UTF8 bytes/deadline/disconnect/concurrency/audit 故障边界 | RED→GREEN contract/integration | `cargo test -p ontoforge-server --test http_contract --locked -- --nocapture` | `crates/ontoforge-server/tests/http_contract.rs` → `tests/unit/http_contract.test.rs` | GREEN：已验证基线；最终四必需CI job通过 |
| 07-04-1 | 07-04 | 4 | API-01,API-03,MCP-06,SCOPE-07 | T-07-10,T-07-11,T-07-12 | 真实 validate/submit/result/history；eventId/committedAt 相同，maskedChanges 留字段名不泄漏值；完整 history 分页另见 history_pagination | RED→GREEN contract/integration | `cargo test -p ontoforge-server --test http_actions --locked -- --nocapture` | crates/ontoforge-server/tests/http_actions.rs（已存在）<br>crates/ontoforge-server/tests/support/mod.rs（已存在） | GREEN：已验证基线；最终四必需CI job通过 |
| 07-04-2 | 07-04 | 4 | API-01,API-03,MCP-06,SCOPE-07 | T-07-10,T-07-11,T-07-12 | 真实同 command race、当前授权拒绝；source barrier 后第二 store reload，submit/validate 均 DEFINITION_STALE、五表及业务行零变更且不重试 | RED→GREEN contract/integration | `cargo test -p ontoforge-server --test http_action_safety --locked -- --nocapture` | crates/ontoforge-server/tests/http_action_safety.rs（已存在） | GREEN：S 本轮 HTTP safety 3/3；PG 另见 13-1 |
| 07-05-1 | 07-05 | 5 | API-02,API-01,API-03 | T-07-13,T-07-14,T-07-15 | 静态 render 与受控 OpenAPI 完全一致；3.1 paths/security、recursive union/unknown fields、Unicode/整数、完整 receipt/history/error/health schema | RED→GREEN contract/integration | `cargo test -p ontoforge-api --test openapi_snapshot --locked -- --nocapture` | crates/ontoforge-api/tests/openapi_snapshot.rs（已存在） | GREEN：已验证基线；最终四必需CI job通过 |
| 07-05-2 | 07-05 | 5 | API-02,API-01,API-03 | T-07-13,T-07-14,T-07-15 | 三个无 config/store 的并发导出字节一致；缺父目录返回非零；render snapshot 无漂移 | RED→GREEN contract/integration | `cargo test -p ontoforge --lib --locked openapi_ -- --nocapture`<br>`cargo test -p ontoforge-api --test openapi_snapshot --locked -- --nocapture` | `crates/ontoforge/tests/unit/serve.test.rs`；`crates/ontoforge-api/tests/openapi_snapshot.rs` | GREEN：已验证基线；最终四必需CI job通过 |
| 07-06-1 | 07-06 | 6 | L2-04,API-01,API-03 | T-07-16,T-07-17,T-07-18 | 真实 health HTTP 200/503/recovery；占用 listener 零 monitor probe；真实 proof lifecycle 与 drift owner 由 health_trino/source_drift suites 补强 | RED→GREEN contract/integration | `cargo test -p ontoforge-server --test health_lifecycle_it --locked -- --nocapture` | crates/ontoforge-server/tests/health_lifecycle_it.rs（已存在） | GREEN：已验证基线；最终四必需CI job通过 |
| 07-06-2 | 07-06 | 6 | L2-04,API-01,API-03 | T-07-16,T-07-17,T-07-18 | CLI audit drain/失败非零；真实 accepted Action、定向读取消、SSE、stuck deadline/owned task failure 由 mcp_transport 与 health_trino_it 补强 | RED→GREEN contract/integration | `cargo test -p ontoforge --test cli_shutdown --locked -- --nocapture --test-threads=1`<br>`cargo test -p ontoforge-server --test mcp_transport --locked -- --nocapture`；真实环境见 13-2 | crates/ontoforge/tests/cli_shutdown.rs（已存在） | GREEN：已验证基线；最终四必需CI job通过 |
| 07-07-1 | 07-07 | 7 | MCP-01,MCP-02,MCP-03,MCP-04,MCP-05,MCP-06,MCP-07,MCP-08,SCOPE-07 | T-07-19,T-07-20,T-07-21 | 真实 core/full 请求参数、metadata/history REST 等价；同 session source barrier 跨 reload/revoke，下一调用零 source 且旧 def_version audit | RED→GREEN contract/integration | `cargo test -p ontoforge-server --test mcp_contract --locked -- --nocapture` | crates/ontoforge-server/tests/mcp_contract.rs（已存在）<br>crates/ontoforge-server/tests/support/mod.rs（已存在） | GREEN：S 本轮 contract 16/16 |
| 07-07-2 | 07-07 | 7 | MCP-01,MCP-02,MCP-03,MCP-04,MCP-05,MCP-06,MCP-07,MCP-08,SCOPE-07 | T-07-19,T-07-20,T-07-21 | 真实 nested params/schema/defaults；REST/MCP 同 command barrier race 完整 receipt 相同且五表各一次；旧 schema 进入 fence、新 schema 下一请求生效 | RED→GREEN contract/integration | `cargo test -p ontoforge-server --test mcp_actions --locked -- --nocapture` | crates/ontoforge-server/tests/mcp_actions.rs（已存在） | GREEN：S 本轮 Actions 10/10；底层 fence/CAS/guard 见 02-2 |
| 07-08-1 | 07-08 | 8 | MCP-01,MCP-06,MCP-07,MCP-08,L2-04,SCOPE-07 | T-07-22,T-07-23,T-07-24 | 六工具名字/schema/allowlist；同 session 撤权/60s proof expiry 拒绝零 endpoint/source 请求，health 一致且恢复准入 | RED→GREEN contract/integration | `cargo test -p ontoforge-server --test mcp_l2 --locked -- --nocapture` | crates/ontoforge-server/tests/mcp_l2.rs（已存在） | GREEN：S 本轮 L2 2/2 + defaults 9/9；F真实服务复验通过 |
| 07-08-2 | 07-08 | 8 | MCP-01,MCP-06,MCP-07,MCP-08,L2-04,SCOPE-07 | T-07-22,T-07-23,T-07-24 | 真实 HTTP manager 容量≤256、TTL/DELETE/取消/初始化清理、host 隔离；过期签名 token 的 POST/GET/DELETE 401；定向取消、SSE、Action 不重放 | RED→GREEN contract/integration | `cargo test -p ontoforge-server --test mcp_transport --locked -- --nocapture`<br>`cargo test -p ontoforge-server --lib --locked state::session_tests -- --nocapture` | `crates/ontoforge-server/tests/mcp_transport.rs`；`crates/ontoforge-server/tests/unit/mcp_sessions.test.rs` | GREEN：T24基线及T08最终50/50相邻回归，独立复签CLOSED |
| 07-09-1 | 07-09 | 10 | MCP-01,MCP-06,MCP-07,SCOPE-07 | T-07-25,T-07-26,T-07-27 | bridge_configuration_disables_reinit_and_rejects_url_credentials；bridge_configuration_bounds_concurrency_and_redacts_errors；真实双 ID 取消隔离/EOF | RED→GREEN contract/integration | `cargo test -p ontoforge --lib --locked bridge_ -- --nocapture` | `crates/ontoforge/src/mcp.rs`；`crates/ontoforge/tests/cli_stdio.rs` | GREEN：E3/N2 stdio11/11，真实坏token与302 trap；G5通过 |
| 07-09-2 | 07-09 | 10 | MCP-01,MCP-06,MCP-07,SCOPE-07 | T-07-25,T-07-26,T-07-27 | 真实 stdio query/取消/EOF、absent server；新 live_stdio_closed_upstream_session_never_reinitializes_or_replays、live_stdio_disconnected_before_forward_never_retries_or_commits、live_stdio_lost_commit_response_requires_explicit_replay 首轮 fixture 失败后修正；N1/N2 补测与独立复签均已通过 | RED→GREEN contract/integration | `cargo test -p ontoforge --test cli_stdio --locked -- --nocapture --test-threads=1` | crates/ontoforge/tests/cli_stdio.rs（已存在） | GREEN：E3/N1/N2 stdio11/11 + Mia定向1/1；G5通过 |
| 07-10-1 | 07-10 | 7 | API-04 | T-07-28,T-07-29,T-07-30 | 全部真实 HTTP 读端点、L2 health；双有效主体 metadata 隔离、实际 scope_apply 撤权、两个 source barrier 请求只取消一个 | RED→GREEN contract/integration | `cargo test -p ontoforge-client --test contract --locked -- --nocapture` | crates/ontoforge-client/tests/contract.rs（已存在）<br>crates/ontoforge-client/tests/support/mod.rs（已存在） | GREEN：K 本轮 Rust 9/9 |
| 07-10-2 | 07-10 | 7 | API-04 | T-07-28,T-07-29,T-07-30 | 真实 Action/history/result/replay/分页/数值；发送前断连和提交后丢响应计数1，完整持久表和业务版本一次提交，显式 replay 原 receipt | RED→GREEN contract/integration | `cargo test -p ontoforge-client --test contract --locked -- --nocapture` | crates/ontoforge-client/tests/contract.rs（已存在） | GREEN：K 本轮 Rust 9/9 |
| 07-11-1 | 07-11 | 7 | API-05 | T-07-31,T-07-32,T-07-33 | 生成 types/strict typecheck/真实 HTTP 读取；双有效主体与 SQLite 实际撤权；响应在途 AbortSignal 隔离，未宣称服务端执行取消 | RED→GREEN contract/integration | `npm --prefix clients/typescript run test:contract`<br>`npm --prefix clients/typescript run typecheck`<br>`npm --prefix clients/typescript run check:generated` | clients/typescript/tests/contract.test.mjs（已存在） | GREEN：F最终binary TS17/17、生成/strict/build通过 |
| 07-11-2 | 07-11 | 7 | API-05 | T-07-31,T-07-32,T-07-33 | 公开 query u64 max 与 codec 不舍入、unsafe number 发送前拒绝、decimal 字符串；故障 proxy 单次发送/完整持久行与显式 replay；全部端点/诊断合同 | RED→GREEN contract/integration | `npm --prefix clients/typescript run test:contract`<br>`npm --prefix clients/typescript run check:generated`<br>`npm --prefix clients/typescript run typecheck` | clients/typescript/tests/contract.test.mjs（已存在） | GREEN：F最终binary TS17/17、生成/strict/build通过 |
| 07-12-1 | 07-12 | 11 | DX-01,DX-02 | T-07-34,T-07-35,T-07-36 | 真实 dev 子进程重启 receipt/history 保留、显式 reset/token 更新；OS lock/崩溃恢复/危险目录与 Unix 权限防护 | RED→GREEN contract/integration | `cargo test -p ontoforge --test dev_lifecycle --locked -- --nocapture --test-threads=1` | crates/ontoforge/tests/dev_lifecycle.rs（已存在） | GREEN：已验证基线；最终四必需CI job通过 |
| 07-12-2 | 07-12 | 11 | DX-01,DX-02 | T-07-34,T-07-35,T-07-36 | 隔离 cwd binary 启动；完整 query→Action→history→restart→reset 由 release-smoke 实际 archive 验证，不把启动存活等同完整调用 | RED→GREEN contract/integration | `cargo test -p ontoforge --test dev_lifecycle --locked -- --nocapture --test-threads=1` | crates/ontoforge/tests/dev_lifecycle.rs（已存在） | GREEN：已验证基线；最终四必需CI job通过 |
| 07-13-1 | 07-13 | 11 | API-01,API-03,MCP-01,MCP-06,MCP-07,MCP-08,L2-04,SCOPE-07 | T-07-37,T-07-38,T-07-39 | 真实 PG snapshot/revoke/history/原子写；新增 definition fence 与 REST/MCP race 已真实实跑；真实 Trino 长读短锁、六工具/四 EXPLAIN/ACL | RED→GREEN contract/integration | `cargo test -p ontoforge-server --test postgres_http_it --locked -- --ignored --nocapture --test-threads=1`<br>`cargo test -p ontoforge-storage --test request_snapshot --locked -- --ignored --nocapture --test-threads=1`<br>`cargo test -p ontoforge-server --test trino_http_it --locked -- --ignored --nocapture --test-threads=1` | crates/ontoforge-server/tests/postgres_http_it.rs（已存在）<br>crates/ontoforge-server/tests/trino_http_it.rs（已存在）<br>crates/ontoforge-server/tests/support/mod.rs（已存在） | GREEN：F真实PG4、snapshot1、Trino2；两项新增PG已实跑 |
| 07-13-2 | 07-13 | 11 | API-01,API-03,MCP-01,MCP-06,MCP-07,MCP-08,L2-04,SCOPE-07 | T-07-37,T-07-38,T-07-39 | 真实 health proof 61s expiry/recovery、core failure、Action dependency drain/cancel/audit；CI named 非零发现、SDK、credential scan 与 cleanup | RED→GREEN contract/integration | `bash ci/server-services.sh verify` | crates/ontoforge-server/tests/health_trino_it.rs（已存在） | GREEN：F drift2、health2、ACL6及完整服务/SDK链 exit0 |
| 07-14-1 | 07-14 | 12 | DX-04,DX-01,DX-02,API-04,API-05 | T-07-40,T-07-41,T-07-42 | 真实三 native binary/archive/manifest/checksum；source HEAD 与 manifest commit 区分，错 commit/target fail closed | artifact/process smoke | `pwsh -File ci/release-package.ps1 -Target x86_64-pc-windows-msvc -OutDir target/release-dist` | ci/release-package.ps1（已存在） | GREEN：R最终同tree三native构建、archive/hash/aggregate通过 |
| 07-14-2 | 07-14 | 12 | DX-04,DX-01,DX-02,API-04,API-05 | T-07-40,T-07-41,T-07-42 | 真实 archive 隔离 cwd query/Action/history/restart/reset/stdio query/清理；坏 checksum/commit/version/target/额外文件拒绝；publish skipped | artifact/process smoke | `pwsh -File ci/release-smoke.ps1 -ArtifactDir target/release-dist -ExpectedTarget x86_64-pc-windows-msvc` | ci/release-smoke.ps1（已存在） | GREEN：R实际包完整安装smoke通过；publish skipped；G5通过 |
| 07-15-1 | 07-15 | 6 | MCP-07 | T-07-43,T-07-44,T-07-45 | 四 ExplainType 精确 TYPE SQL 与旧 DISTRIBUTED wrapper；结构化 table/defaults/qualified/quoted 名称与注入拒绝 | RED→GREEN contract/integration | `cargo test -p ontoforge-engine --test l2_explain --test l2_gate --locked -- --nocapture` | crates/ontoforge-engine/tests/l2_explain.rs（已存在） | GREEN：已验证基线；最终四必需CI job通过 |
| 07-15-2 | 07-15 | 6 | MCP-07 | T-07-43,T-07-44,T-07-45 | 四格式成功输出与单次 audit；旧 wrapper 拒绝和取消已有基线；四格式真实timeout/取消隔离与HTTP并发数量上限均由N3/T08闭合 | RED→GREEN contract/integration | `cargo test -p ontoforge-query --test l2_execute --locked -- --nocapture` | crates/ontoforge-query/tests/l2_execute.rs（已存在） | GREEN：N3 query15/15 + T08四格式limit（defaults10/10） |
| 07-16-1 | 07-16 | 9 | MCP-07,SCOPE-07 | T-07-46,T-07-47 | 可信 catalog/schema defaults、qualified>explicit>default；真实 listener 的 metadata SQL 使用部署 defaults | RED→GREEN contract/integration | `cargo test -p ontoforge-server --test mcp_l2_defaults --locked mcp_l2_optional_context_defaults_and_qualified_override -- --nocapture` | crates/ontoforge-server/tests/mcp_l2_defaults.rs（已存在） | GREEN：S 本轮 defaults 9/9 |
| 07-16-2 | 07-16 | 9 | MCP-07,SCOPE-07 | T-07-46,T-07-47 | 缺 context/越界 allowlist/非法 SQL/format/qualified 在 availability 前拒绝；六工具缺 scope 拒绝零 status/execute/describe/audit；E6 RED→GREEN | RED→GREEN contract/integration | `cargo test -p ontoforge-server --test mcp_l2_defaults --locked -- --nocapture` | crates/ontoforge-server/tests/mcp_l2_defaults.rs（已存在） | GREEN：S 本轮 defaults 9/9 + 六工具拒绝补矩阵 1/1 |

## 历史覆盖缺口闭合

| ID | 原缺口 | 后继 GREEN 与复签 |
|---|---|---|
| N1 | 真实 stdio 成功链及不同主体直接调用证据不足 | stdio11/11；Alice describe/query/validate/Action/history/receipt，Mia经MCP和REST共8次拒绝后完整持久行/原receipt不变，定向1/1；Spec CLOSED |
| N2 | 无真实302及在线坏token拒绝 | 在线host 401、零业务请求；302 origin已收bearer但trap零连接；有界退出/无secret；stdio11/11、Spec CLOSED |
| N3 / T08 | typed EXPLAIN只有顺序成功；真实cap1第二请求200而非429 | query15/15覆盖四格式timeout/定向cancel/独立并发；T08修复后原四格式cap测试GREEN，最终admission50/50；Spec/Staff/security CLOSED |

E3首轮5 passed/3 failed为协议假设与receipt读取fixture错误；按实际Initialize/session和`structuredContent.receipt`修正，后继11/11闭合。E6为越权调用提前触发availability probe的真实RED，授权预检前移后L2矩阵GREEN。T08为实际执行期permit提前释放的真实RED；修复草稿REST extractor释放Arc的回归同样保留RED，以outer只drop与handler唯一take后的50/50闭合。T24两轮RED均由session12/transport11及后继相邻回归闭合。详情分别保留在对应实施/复签报告，不删除失败日志。

## 最终集成门

| Gate | 实测证据 | 当前状态 |
|---|---|---|
| G1 | `target/phase7-bridge-security-cli-r2.log` 11/11；Mia定向1/1；独立Spec复签 | GREEN |
| G2 | final fresh postgres_http_it 4/4，包含definition fence与REST/MCP race | GREEN |
| G3 | `target/phase7-final-adc0227-verify.log`：服务17+Rust9+TS17=43，fresh_verify_exit=0；CI phase7-real-services success；scan/cleanup通过 | GREEN |
| G4 | 同一final fresh binary的TS17/17，check:generated/typecheck/build通过，无需重复运行 | GREEN |
| G5 | CI `41669982375` 四必需job全部success，Windows/Ubuntu完整日志已保存；T24/E6/T08/N1/N2/N3复签完成，简化无建议 | GREEN |
| G6 | release `41669982362` 三native+aggregate success；三个archive hash独立核验；source与打包commit同tree，publish skipped | GREEN |

本地fresh退出由主会话session31544确认0；`commit.txt`为`adc02278b5cc55111def9a30a1c3b778485e8279`。脚本finish强制执行credential scan与cleanup，退出0证明二者完成；实测Docker仅剩用户既有vennuap-casdoor，runtime/server-windows.N5DO2s/state.json为exited/exit_code0。没有将用户既有容器当本任务遗留。

## Wave 0 Requirements

- [x] 真实 listener/auth/audit fixture 与非空 server contract suites 已存在。
- [x] storage/action snapshot race fixtures 已存在，SQLite/PG 共用锁行为合同。
- [x] OpenAPI、Rust/TS clients 的生成与 contract 入口已存在；TS 不再是“待创建”。
- [x] CLI 子进程、health/shutdown 与 release package/smoke 入口已存在。

`wave_0_complete: true` 只表示基础设施齐备；32个task的G5已由四必需CI job全部success关闭。ignored服务测试已经在F与CI服务门实际执行，不能从workspace ignored计数推导通过。

## Manual-Only Verifications

无纯人工项、无已豁免项。三平台使用 native CI 自动验收。历史N缺口与G5均已自动化闭合，无需转为manual-only。Release upload、npm publish、PR merge 不属于本轮已授权验证的执行项。

## Validation Sign-Off

- [x] 32 个 task 均有可执行 automated verify，现有文件/入口已按实际布局更新。
- [x] 每个 PLAN 保有 `fails_when`，零发现或 required case 未执行不能通过；旧名称的替代与缺口已明确。
- [x] 不存在连续三个 task 无自动化入口。
- [x] PG/Trino、clients、native 历史与本轮定向日志边界可追溯。
- [x] 已记录局部实测反馈时长；不虚构最终完整矩阵时长。
- [x] N1/N2/N3 补齐并取得动态 GREEN，T24/E6/T08及独立复签闭合。
- [x] G1至G6最终对账全部通过，四必需CI job均success，独立安全/代码审查复签完成。
- [x] 32个task全部GREEN，动态门齐全后设置 `nyquist_compliant: true`。

Approval: approved 2026-11-04

## Validation Audit

- 2026-11-03历史时点：State A 审核；32 task 的全部 pending/缺失路径表已更新。发现 3 个原计划覆盖事实缺口（N1/N2 同属 stdio，N3 为 typed EXPLAIN），新增 bridge 待修正后 GREEN、PG 已有测试仅待实跑，未重复列为缺失。N1/N2/N3 已批准并由既有 CLI worker 与 EXPLAIN worker 实施，均 in_progress。本文仅更新映射，不运行 Cargo/服务、不改测试或实现；最终状态由主会话据新增证据更新。

- 2026-11-04等待期间：最终fresh43例exit0、native成功、N1/N2/N3/T08闭合，Windows未终局时曾保留nyquist=false。
- 2026-11-04最终：CI41669982375四必需job全部success，G5关闭；32task及G1–G6全部GREEN后设置status=passed、nyquist=true。
