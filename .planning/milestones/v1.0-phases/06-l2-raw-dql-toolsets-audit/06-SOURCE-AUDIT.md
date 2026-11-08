# Phase 6 source coverage audit

此表记录 GOAL、REQ、RESEARCH、CONTEXT 四源到执行计划的映射；`06-CONTEXT.md` 的 Deferred Ideas 和 RESEARCH 明确 Phase 7/下个 milestone 的内容不计缺口。

| SOURCE | ID | Feature/Requirement | Plan | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| GOAL | - | allow-list 内读 Trino、ACL 证明、全部公开读审计与管理边界 | 01-09 | COVERED | Phase 6 库健康状态，HTTP /health 属 Phase 7 |
| REQ | L2-01 | sql:raw 查询和 catalog/schema/table 枚举 | 06,07 | COVERED | 六个库级操作 |
| REQ | L2-02 | AST 白名单及重序列化发送 | 03,06,08 | COVERED | 104-ID corpus 与真 483 行为 |
| REQ | L2-03 | allow-list、edit、system 硬排除 | 03,04,06,07 | COVERED | 应用 + Trino 双重授权 |
| REQ | L2-04 | Trino ACL 自检失败禁用/健康告警 | 04,08 | COVERED | 库状态供 Phase 7 路由复用 |
| REQ | AUDIT-01 | 全公开读元数据审计、异步队列、fail-closed、保留期 | 01,02,05,06,07,08,09 | COVERED | SQLite/PG，故障与快照 |
| REQ | SCOPE-07 | Phase 6 请求版本固定：在途 L0/L2 读取与审计使用开始时 definition/scope 快照 | 09 | COVERED | Phase 3 管理写入与审计已交付；Phase 7 MCP 工具限制不在本阶段 |
| RESEARCH | - | 私有 PrintedSql 构造、visitor AST/table args 与唯一受控入口 | 03,06 | COVERED | D-01/D-03 |
| RESEARCH | - | G11 有损 ROW、G16 OFFSET/LIMIT 改写、104 ID 0 原文/29 已试/75 未试 | 03,08 | COVERED | 源 provenance 和真 Trino 测试 |
| RESEARCH | - | 独立 ACL，另一主体活跃 query、权限拒绝与撤权复检 | 04,08 | COVERED | 04 自包含 trino-l2 服务启动与真实集成测试，08 接 CI；非现有无 ACL 容器 |
| RESEARCH | - | 双后端审计表、flush、保留清理、故障健康和取消 | 01,02,05 | COVERED | 01 固定容量 1024、batch 128、flush 250 ms、pending 1000 ms、保留 30 天及边界校验；实际 PG/SQLite conformance |
| RESEARCH | - | 六库操作及 L0 pk/list/set/searchAround/aggregate 逐项审计 | 01,05,06,07 | COVERED | 内部 read_pk_source 不双记 |
| RESEARCH | - | reload/scope 与请求快照、管理审计 | 09 | COVERED | 既有 CLI 通道 |
| RESEARCH | - | 真实 PG/Trino ignored target 非零发现与 workspace 回归 | 08 | COVERED | CI gate |
| CONTEXT | D-01 | SQL raw 授权、单条 SELECT/WITH、LIMIT、Display、硬排除 | 03,06,07 | COVERED | |
| CONTEXT | D-02 | 独立身份，默认禁用，受限 ACL 和活跃查询自检 | 04,06,08 | COVERED | 04 启动/实测，08 CI；X-Trino-User 可选 |
| CONTEXT | D-03 | 保留类型化 printer 与 PrintedSql 私有构造 | 03,06 | COVERED | |
| CONTEXT | D-04 | 元数据审计、双后端、队列保留期与 AuditUnavailable | 01,02,05,06,07,08,09 | COVERED | pk/execute_query 逐项断言成功、拒绝、源错误、取消及结果门禁；其他入口见 05/06/07 |
| CONTEXT | D-05 | reload、role/scope 管理与请求快照 | 09 | COVERED | |
| CONTEXT | D-06 | 104-ID 语料、Trino 483 差分与真实 ACL | 03,04,08 | COVERED | |

Spec-less probe 的 11 个 unresolved 将作为实施断言复核：L2-01 的三项在 06/07，L2-02 在 03/08，L2-03 的五项在 03/04/07，L2-04 在 04/08，AUDIT-01 在 01/02/05。三项 unclassified 分别假设：L2-02 的有损 Display 一律拒绝，L2-04 的不确定 ACL 证明保持 disabled，AUDIT-01 的 sink 失效优先拒读；若具体 probe 与 accepted ADR 冲突，先回流用户决策，不能改写锁定合同。
