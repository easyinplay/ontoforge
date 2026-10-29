# 07-03 Task 03-1 验收抽查

- 日期：2026-10-29。
- 范围：真实 REST 测试与 `d31cf91` audited wrappers，只读核查；本审查未运行测试。
- 结论：1 项验收缺口；所查查询路径未发现可证明的 production bug。03-2 observed/deadline 改动仍在施工，不属于本轮通过结论。

## 待修复

- `07-03-PLAN.md:136` 要求 metadata 提供引擎脱敏状态。`ontoforge-api/src/dto.rs:188` Metadata 与 `ontoforge-server/src/rest.rs:98` ontology 响应没有字段，`tests/unit/rest.test.rs:19` 无断言。补稳定逻辑状态 DTO，仅可用/不可用，不暴露 `EngineStatus::Unavailable.reason`；真实 listener 断言状态及敏感 reason 不泄漏。

## 已核查覆盖

| 合同 | 证据 |
|------|------|
| 真实认证 listener/source/store | server `tests/support/mod.rs:76-147`、`:205-242` 创建持久 store、真实 SQLite source、verifier、TCP listener 并发送 Bearer HTTP。 |
| 全部 L0 操作 | `tests/unit/rest.test.rs:31-68` 对 pk/filter/search/searchAround/union/intersect/subtract/aggregate 断言具体值、PK 或分桶。 |
| 排序与分页 | 同文件 `:88-143` 验证相等排序、nullable、空/单项、三页 cursor；9 MiB 真实对象产生短页并继续读取下一页。 |
| reload/revoke/inflight | 同文件 `:151-222` 用 Engine Notify 固定次序，旧请求成功且审计保留旧 def_version，新请求撤权；管理事务受 timeout 检查。 |
| audited wrappers | `d31cf91` execute/aggregate 各单次 reserve，旧入口透传 result，保留 status/finish/Drop；REST 断言八次数据操作对应八条审计及 list ID。 |
| 实际 GREEN | `../07-03-evidence/rest-green.log` 三个指定测试，3 passed、0 failed、0 ignored；对应 JSON exitCode 0。本轮仅读取，未重跑。 |

在途用例 reload 只修改 description；固定版本证据为持久审计 def_version 以及 context/QueryService 传递同一 active 的源码路径。此处不扩大为字段语义变化的实测结论。
