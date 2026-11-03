---
phase: 07-rest-mcp-shell
reviewed: 2026-11-03T08:05:00Z
depth: standard
review_base: 0a3e3596ab9bed093b2815cbc019128f0323a048
review_target: uncommitted-working-tree
scope_contract: RF-10
source_blobs:
  crates/ontoforge-api/src/dto.rs: 56edea469419c0cfd4870b3cca9833103181a90a
  crates/ontoforge-api/src/openapi.rs: 912426f9c85633fe55c136f69f5d48f58bbb6f23
  crates/ontoforge-api/tests/openapi_snapshot.rs: 4a949b9b086aa19280251a39e9b9c02ed9687b92
  crates/ontoforge-client/src/lib.rs: c53c7282a30ceff29a0c5b1d6c28aedca9f942a9
  crates/ontoforge-client/tests/contract.rs: ad812e31d3b42a6d9e5b75c267147b7b71437d61
  crates/ontoforge-client/tests/support/mod.rs: 71770f18e11773f0122884008924b4e4bf038d98
  clients/typescript/src/generated.ts: 93383a6bc5713f0f23c88a4bc4c14f4e0c6d7729
  clients/typescript/tests/contract.test.mjs: 98c9422d5ae444c5bc520d89e8f0c948252a880d
  clients/typescript/tests/health-types.ts: 31f309cd0598090c7116b2c35722ccbec411a005
  docs/api.md: db936a3522d8f37aaff5141d7d9ca85ec0faeb4b
  docs/openapi.json: a13cec8cd3d0eff074e68a78af976e8130e9fe99
files_reviewed: 11
files_reviewed_list:
  - crates/ontoforge-api/src/dto.rs
  - crates/ontoforge-api/src/openapi.rs
  - crates/ontoforge-api/tests/openapi_snapshot.rs
  - crates/ontoforge-client/src/lib.rs
  - crates/ontoforge-client/tests/contract.rs
  - crates/ontoforge-client/tests/support/mod.rs
  - clients/typescript/src/generated.ts
  - clients/typescript/tests/contract.test.mjs
  - clients/typescript/tests/health-types.ts
  - docs/api.md
  - docs/openapi.json
findings:
  critical: 0
  warning: 0
  info: 0
  total: 0
status: clean
original_finding: Spec-CR-10
original_finding_status: resolved_for_existing_l2_wire
phase_acceptance: pending
---

# Phase 7 Health Wire Recheck

RF-10 定向复签为 clean，无未解决 BLOCKER 或 WARNING。范围仅为上述 11 个冻结工作区文件的增量，以及 server health endpoint、TypeScript call/decode 和 Rust fixture 的必要调用链。生成文件按 wire 合同增量核对，不宣称逐读全部生成内容。没有审查或变更新 drift/health 判定，server health.rs 无工作区改动。

本 reviewer 只新增报告，未运行 Cargo/npm、未修改实现、共享 planning 或其他 lane 文件，未提交。

## Narrative Findings (AI reviewer)

无新增未解决发现。原 Spec CR-10 中“既有 L2 wire 被 SDK/类型丢失”的缺口已关闭。

## Static Evidence

| 边界 | 源码结论 |
| --- | --- |
| API DTO | dto.rs:373-385 为 HealthResponse 增加 Option<L2Health>；status 为 String，reason 为 Option<String>。两层 Option 为 None 时均省略序列化字段，保留既有无 L2 输出。 |
| OpenAPI / TS | openapi.rs 注册 L2Health；docs/openapi.json 的 HealthResponse.l2 非 required，L2Health 只有 status required，reason 可缺省/nullable；generated.ts:220-259 与此一致。health-types.ts 使用 public OntoforgeClient.health 返回类型访问两个字段。 |
| Rust 解码 | client/lib.rs:434-444 将存在的 l2 按 Option<L2Health> serde 解码；缺失/null 为 None，合法 status/reason 被保留，错误子字段类型不会被默默丢弃。既有 engineStatus/status 行为不在本轮改写。 |
| 未改变状态判断 | server/health.rs 继续决定 healthy/degraded/unhealthy、enabled/disabled 及稳定 reason_code。RF-10 没有新增 source target/SQL/底层异常字段，也没有将既有判定搬到客户端。 |
| 文档 | docs/api.md 的 Health wire 合同与上述省略/可选语义一致，没有承诺新增故障检测能力。 |

## Test Evidence And Limits

独立读取 target/phase7-health-wire-* 日志及实际断言，执行归属 owner：

- Rust RED：真实 HTTP response 有 l2，而 typed health 丢字段；1 passed/1 failed，EXIT=101。GREEN：6 unit、7 contract 全通过，EXIT=0。health 专用合同在真实 listener 上分别配置 Enabled、Disabled(Expired)、Disabled(NotConfigured)，比较原始 /health JSON 与 Rust DTO 重新序列化结果。
- API RED：实际生成 schema 缺 l2，0/1，EXIT=101。GREEN：6/6，EXIT=0；检查 l2 非 required、reason 可选，并核对签入 OpenAPI 文本等于静态 render。
- TypeScript RED：public health 类型访问产生 TS2339，属于类型合同 RED，不是 HTTP 行为 RED。GREEN：10/10、0 skipped、EXIT=0。启用/禁用两个用例使用注入 fetch 返回 payload，只证明解码透传与公共类型可用；未配置 L2 的省略则由既有真实 CLI HTTP 用例检查。不得把注入 payload 称为真实 L2 服务验收。
- TS 日志连续执行两次 generate，记录 Generation drift check: unchanged；owner 报告两次 hash 一致，本 reviewer 未重新生成。export、Clippy、format 日志均记录 EXIT=0。

Rust 的三态 HTTP 测试使用受控 L2 status provider，未启动真实 Trino ACL monitor。这里验证既有服务状态如何穿过 HTTP 与 SDK，不证明 Enabled/Expired 的真实检测机制、RF-01 新 drift 判定、整个 Phase 或跨平台全 workspace 验收。API/TS 对 null 的兼容不改变服务端未配置时省略 l2 的行为。
