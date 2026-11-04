---
phase: 07-rest-mcp-shell
status: SECURED
asvs_level: 1
block_on: high
threats_open: 0
audited_head: f8851ca073fed34e80e605f0ac91880259df7e12
historical_audited_head: adc0227
incremental_scope: [INT-01, TLS-GUARD]
date: 2026-11-04
---

# Phase 7 安全核验

## 2026-11-04 增量复签

INT-01（L2-03，关联T-07-22）和TLS-GUARD均 CLOSED，当前 `threats_open=0`。独立安全审查见 [GAP-SECURITY-REVIEW.md](../../milestones/v1.0-audit/GAP-SECURITY-REVIEW.md)，本轮 verifier 另亲核生产diff、五个HTTP测试断言、原始RED/GREEN与最终双平台CI日志，详 [PHASE-07-REVERIFY.md](../../milestones/v1.0-audit/PHASE-07-REVERIFY.md)。原48项威胁继承下文历史证据，没有伪称全部重新审计。

| 增量 | 源码与行为证据 | 状态 |
|---|---|---|
| INT-01 / T-07-22 | `54a47dd`：可信ACL配置accessor→Enabled检查→六工具统一policy；默认/自定义catalog、defaults/qualified target、全部上游零发送、零fallback/audit与正向六工具；RED2→GREEN2，相关83/83，最终CI两OS新5项全部ok | CLOSED |
| TLS-GUARD / T-07-SC局部 | `ef7ac92`：单次完整图精确package匹配、Cargo错误fail-closed；verifier独立跑6个hermetic场景exit0，最终CI两OS的hermetic及真实helper均success | CLOSED |

最终CI `41759804733` 四必需job成功；release `41759804768` native3/3与aggregate成功、publish skipped；fresh服务/SDK43项及凭证扫描、清理通过。动态总账51项不用于重计威胁。原dynamic runner因HEAD变化exit1，独立provenance/scan/cleanup exit0保留为另一项证据。未证明真实环境发生过数据泄露；HTTP fixture验证应用fence。STORE-05与ENGINE-05全局缺口不在本复签范围。

## 历史审计记录（基线 adc0227）

2026-11-04早先发现INT-01时，本报告曾暂置OPEN_THREATS/high OPEN；原48/48不能覆盖自定义catalog。该暂停已由上方增量源码、测试与独立复签解除。以下时间、计数及基线维持当时事实。

独立 gsd-security-auditor 核对16份 PLAN，去重为48项（44 high、4 medium）；T24、T22及N3发现的T08修复均已独立复签CLOSED，当前48/48 CLOSED、threats_open=0。全部 disposition=mitigate，无 accepted/transfer 风险。初审基线e6ae310/b068ff5，增量至adc0227；精确最终指纹见reviews/07-T24-SECURITY-RECHECK.md、07-E6-SECURITY-RECHECK.md、07-T08-SECURITY-RECHECK.md，后者覆盖后续context/mcp/lib/state变更。2026-11-04最终fresh43/43、CI41669982375四必需job和native41669982362全部通过，详07-14-SUMMARY.md；不扩大为实际生产发布安全认证。

## T24 初审发现与关闭

T-07-24：lib.rs使用独立LocalSessionManager::default，mcp_sessions.rs仅限制owner HashMap容量256。owner eviction没有close_session接线，真实rmcp session可超过容量。锁定rmcp3.8.0 local.rs:74无容量判断；300秒idle和tower.rs:1637自然退出回收存在，故不报告永久泄漏。mcp_transport.rs:360仅测owner map。

关闭条件已通过真实session12项与transport11项，并由独立auditor复签：真实session创建前有界准入，初始化中资源计数；驱逐/TTL/DELETE/宿主关闭释放实际资源，真实HTTP验证容量峰值及旧session不可用。修复设计与第二意见见reviews/07-T24-DESIGN.md，最终证据及指纹见reviews/07-T24-SECURITY-RECHECK.md。

## Register

类别：S=Spoofing，T=Tampering，R=Repudiation，I=Information Disclosure，D=Denial of Service，E=Elevation of Privilege。路径均相对仓库根；证据行号绑定上述基线。后续修复复签应重新绑定。

| ID | 类别 | Severity | 状态 | 核心证据 |
|---|---|---|---|---|
| T-07-01 | S | high | CLOSED | crates/ontoforge-server/src/context.rs:270,311 |
| T-07-02 | T | high | CLOSED | crates/ontoforge-storage/src/request_snapshot.rs:24,38 |
| T-07-03 | R | high | CLOSED | crates/ontoforge-server/src/rest.rs:177; lifecycle.rs:65 |
| T-07-04 | T | high | CLOSED | crates/ontoforge-storage/src/reload.rs:186; tests/request_snapshot.rs:163,261 |
| T-07-05 | E | high | CLOSED | crates/ontoforge-action/src/lib.rs:215,298,312 |
| T-07-06 | T | high | CLOSED | crates/ontoforge-storage/src/commit.rs:617,632,667,671 |
| T-07-07 | I | high | CLOSED | crates/ontoforge-server/src/rest.rs:45,52,77,93,796 |
| T-07-08 | D | high | CLOSED | 最终context.rs共享RAII执行准入/control有界，mcp.rs执行前唯一take；真实50项GREEN与独立07-T08-SECURITY-RECHECK |
| T-07-09 | S | high | CLOSED | crates/ontoforge-server/src/context.rs:251,278,286 |
| T-07-10 | T | high | CLOSED; E1 pending | crates/ontoforge-server/src/rest.rs:457,481; crates/ontoforge-storage/src/commit.rs:617 |
| T-07-11 | E | high | CLOSED | crates/ontoforge-server/src/rest.rs:529,646; crates/ontoforge-storage/src/history.rs:125,224 |
| T-07-12 | R | high | CLOSED | crates/ontoforge-storage/src/commit.rs:678,714,724,728 |
| T-07-13 | I | medium | CLOSED | crates/ontoforge-api/src/openapi.rs:169,211 |
| T-07-14 | T | high | CLOSED | crates/ontoforge-api/src/dto.rs:5,65; tests/openapi_snapshot.rs:115,146 |
| T-07-15 | T | high | CLOSED | crates/ontoforge-api/tests/openapi_snapshot.rs:12; clients/typescript/package.json:7 |
| T-07-16 | E | high | CLOSED; E6 pending | crates/ontoforge-server/src/state.rs:91; mcp.rs:149; crates/ontoforge-engine/src/trino/mod.rs:456 |
| T-07-17 | R | high | CLOSED | crates/ontoforge-server/src/lifecycle.rs:48,56,65 |
| T-07-18 | D | high | CLOSED | crates/ontoforge-server/src/lifecycle.rs:9,49,57,65; lib.rs:178 |
| T-07-19 | E | high | CLOSED | crates/ontoforge-server/src/mcp_catalog.rs:37,66; mcp.rs:556 |
| T-07-20 | T | high | CLOSED; E2 pending | crates/ontoforge-server/src/mcp.rs:580,587,589,637 |
| T-07-21 | R | high | CLOSED; E2 pending | crates/ontoforge-server/src/rest.rs:481; mcp.rs:637; crates/ontoforge-storage/src/commit.rs:632 |
| T-07-22 | E | high | CLOSED; E6 GREEN | mcp_l2.rs:93纯gate、:126及:225-231预检后availability；详07-E6-SECURITY-RECHECK |
| T-07-23 | S | high | CLOSED; E3 pending | crates/ontoforge-server/src/context.rs:339,350,370; mcp.rs:109 |
| T-07-24 | D | high | CLOSED | 最终mcp_sessions.rs:104,125,171,240; lib.rs:210; context.rs:370,379；详T24复签报告 |
| T-07-25 | I | high | CLOSED | crates/ontoforge/src/mcp.rs:42,44,54,68,210 |
| T-07-26 | T | high | CLOSED; E3 pending | crates/ontoforge/src/mcp.rs:91,175 |
| T-07-27 | D | high | CLOSED | crates/ontoforge/src/mcp.rs:90,184; rmcp transport/worker.rs:214,216 |
| T-07-28 | S | high | CLOSED; E4 pending | crates/ontoforge-client/src/lib.rs:100,119,245,250 |
| T-07-29 | T | high | CLOSED; E7 pending | crates/ontoforge-client/src/lib.rs:256,267 |
| T-07-30 | I | medium | CLOSED | crates/ontoforge-client/src/lib.rs:48,283; Cargo.toml |
| T-07-31 | T | high | CLOSED; E5 pending | clients/typescript/src/codec.ts:14,38,47,69; tools/openapi-gen/generate.mjs:9 |
| T-07-32 | S | high | CLOSED; E4 pending | clients/typescript/src/index.ts:148,153,177 |
| T-07-33 | R | high | CLOSED; E7 pending | clients/typescript/src/index.ts:194; tests/contract.test.mjs:287,294 |
| T-07-34 | T | high | CLOSED | crates/ontoforge/src/dev.rs:36,61,84,122,236 |
| T-07-35 | S | high | CLOSED | crates/ontoforge/src/serve.rs:113,126; crates/ontoforge-server/src/context.rs:270 |
| T-07-36 | D | medium | CLOSED | crates/ontoforge/src/dev.rs:29,123,322 |
| T-07-37 | T | high | CLOSED; E2 pending | crates/ontoforge-server/tests/postgres_http_it.rs:54,71,78,183; storage snapshot PG race |
| T-07-38 | E | high | CLOSED; E6 pending | crates/ontoforge-server/tests/health_trino_it.rs:212,235,242,254 |
| T-07-39 | R | high | CLOSED | ci/server-services.sh:41,59,193; .github/workflows/ci.yml always cleanup |
| T-07-40 | T | high | CLOSED native通过 | ci/release-package.ps1:32,38,44,78,90,95,98; release-smoke.ps1:146; run41648281446 |
| T-07-41 | E | high | CLOSED | .github/workflows/release.yml:93,96 |
| T-07-42 | I | high | CLOSED | ci/release-package.ps1:52,61,63,66,72,90,95 |
| T-07-43 | T | high | CLOSED | crates/ontoforge-engine/src/printer/l2.rs:52,163,212,225 |
| T-07-44 | E | high | CLOSED | crates/ontoforge-query/src/l2.rs:383,437,443,450,464 |
| T-07-45 | R | medium | CLOSED | crates/ontoforge-query/src/l2.rs:391,425; tests/l2_execute.rs:599 |
| T-07-46 | E | high | CLOSED; E6 pending | crates/ontoforge/src/serve.rs:209; crates/ontoforge-server/src/mcp_l2.rs:327,347 |
| T-07-47 | T | high | CLOSED | crates/ontoforge-server/src/mcp_l2.rs:299,331; tests/mcp_l2_defaults.rs:485,554 |
| T-07-SC | T | high | CLOSED | 07-RESEARCH.md:51,55; Cargo/npm manifests与locks; ci/server-services.sh:181 |

同一行中的短文件名承接该行前一个完整目录。没有unregistered flags；15份SUMMARY未包含Threat Flags段，不推断其他攻击面不存在。

## 运行证据边界

10175f6完整CI41638673666四必需job通过，本地fresh36/36和cleanup exit0见07-13-SUMMARY。它们不替代E1–E7缺失场景。

run41648281446三native/aggregate成功，publish skipped，metadata headSha=e6ae31055c17d6d4005226b21c2d5991ac5a34ef；实际archive manifest的checkout commit=f315d4650114c449d9f857fccbe0b6db8d80b686，两者身份不同。主会话已下载三个archive重算SHA256一致，审计者复核保存的CI metadata。后续session生产变更仍须新head复验。
