# Phase 7 动态证据缺口

日期2026-11-03，来源最终GSD安全核验。以下均pending；它们不等于已证明的生产漏洞，不计入07-SECURITY的threats_open。实施分工与CC取舍见07-T24-DESIGN。

| ID / 威胁 | 确认的现有证据边界 | 动态验收标准 | 复用入口 / owner |
|---|---|---|---|
| E1 / T10 | http_action_safety.rs:73仅正常submit/validate；PG stale _version不是definition fence | HTTP捕获旧定义后barrier、另一store reload，submit/validate拒绝DefinitionStale、业务表零写且无重试 | support/pg_real.rs:407、postgres_http_it.rs:143、action frozen_definition.rs:249；server_security_evidence |
| E2 / T20,T21,T37 | mcp_actions.rs:245仅schema名；mcp_contract.rs:804仅两个catalog | 同主体同command REST/MCP真实并发，原receipt/event/time一致、log/claim/history/outbox恰好一次；MCP在途固定schema/执行snapshot，提交fence、下一请求新授权 | support/mod.rs、source_drift.rs:872,893、PG counts；server_security_evidence |
| E3 / T23,T26 | transport现有无效bearer/DELETE与owner-map单测，stdio缺expiry/丢响应计数 | 合法token到期后各HTTP方法拒绝；bridge session失效不自动initialize/replay；执行前断连与提交后丢响应均计数无隐式重发 | cli_stdio.rs:27、mcp_transport.rs:272,296；bridge后续lane，T24共验资源 |
| E4 / T28,T32 | Rust/TS仅有效/无效bearer，Rust字符串revoked不是实际撤权，TS发送前abort | 双有效不同grants主体并发隔离；实际撤权后原client下一请求拒绝，另一主体正常；取消一个在途请求不影响另一请求 | Rust contract fixture、TS dev fixture/bob token；sdk_security_evidence |
| E5 / T31 | TS覆盖2^53/i64但未覆盖u64 max | 18446744073709551615n经codec/公开client端点精确保留，unsafe number拒绝，decimal不变 | TS contract.test.mjs:258,393；sdk_security_evidence |
| E6 / T16,T22,T38,T46 | 真实ACL/health expiry已测，mcp_l2.rs:65仍静态，缺同session贯通 | 同session撤sql:raw/grant和proof过期后调用拒绝、该调用上游零发送，health一致、恢复准入；defaults/qualified不可绕过 | mcp_l2_defaults.rs:28,596、health_trino_it.rs:168；server_security_evidence |
| E7 / T29,T33 | 正常submit/replay已测，缺丢响应计数与完整持久增量 | Rust/TS写遇transport error/提交后丢响应仅发送1次，显式replay原receipt/time，完整业务表/事务事件无第二份提交 | Rust contract.rs:154、TS contract.test.mjs:271、PG counts/故障proxy；sdk_security_evidence |

## 验收纪律

用真实listener/生产client/生产handler和可控barrier，mock仅可作上游故障源，不能以静态catalog冒充行为。计数区分合法monitor与业务请求。补证据可直接GREEN；发现实现缺陷需真实RED→GREEN。Cargo串行/jobs2，完整CI在集成head执行。各lane独立报告测试/日志/限制，主会话根据证据更新本表。
