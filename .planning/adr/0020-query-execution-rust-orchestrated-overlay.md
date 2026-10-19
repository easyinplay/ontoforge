# ADR-0020 查询执行：Rust 编排的叠加读，Trino 只跑单表查询

- status: accepted
- date: 2026-10-16
- supersedes: ADR-0005（ObjectSet AST 作为 wire 格式与编译入口保留；「编译成 sea-query、递归 CTE、SQLite 与 PG 结果等价」部分被取代）
- 依据：`.planning/research/spike-03-trino-overlay-latency.md`、`.planning/research/spike-03b-scoped-overlay.md`；Eng Review D2 / D12 / D22 / D24 / D25

## Context
对象状态 = source 行（Trino）按属性叠加 edit（PG，ADR-0016）。spike 3：Trino 内叠加的 join 在繁忙集群上因多 stage 调度退化到 2–4s，单 fragment 查询仍在 1s 内。spike 3b：按属性 patch 的 Trino 内叠加需 7 个 fragment；edit 分支 `LEFT JOIN source` 会广播 source 全表；动态过滤 / IN 超过 256 个值对 JDBC source 不再生效；Rust 两段归并在安静集群与 Trino 内叠加同量级（0.36–0.94s），且每条 Trino 查询 ≤ 2 fragment。resource group 只管准入排队，实测排队 ≤ 1ms，解决不了调度瓶颈。

## Decision
- **ObjectSet AST** 仍是 wire 格式与查询入口（filter / search / searchAround / 集合运算 / aggregate / orderBy / page）。
- **执行由 Rust 编排**，Trino 只接收单表查询（形态约束：单表、≤ 2 fragment，由编译器强制）：
  - get-by-pk / Action 前置读：PG 直读 edit → 需要时 Trino 单表 pk 查 source（`LIMIT 2`）→ Rust 合并。
  - 列表 / 过滤：Trino 单表 source 谓词 + TopN（over-fetch）与 PG 直连 edit 候选并行；被压制或需合并的 pk 按 ≤ 256 一批回查；Rust 合并、按 pk 去重、keyset 分页，补取循环直到凑满或 source 耗尽（`has_more` 判定结束）。
  - 聚合：`count` / `sum` / `avg` 分解为「source 聚合下推 − pk 分批修正 + PG 聚合」；`min` / `max` 走 source TopN 补取；`count(distinct)` 走分组计数分解，代价过高时报错并提示收窄，不静默回退。注意 SQL Server connector 不下推 `sum(integer)`，编译器按 connector 能力表选择改写。
  - searchAround / link：FK link 由合并后的对象决定；M:N link 的 source join 表单表查询 + PG link edit 表合并；目标对象按 pk 分批取。
- **scoped 编译形态**：用户过滤 `x op v` 改写为 `(OR 授予 x 的 role 谓词) AND x op v` 作用在原列，CASE 遮蔽只放在最外层投影（spike 3b：朴素形态中位 2.5s、改写形态 312ms，结果逐行一致）。同一 CEL 行谓词在 Rust 内对合并后的行求值（ADR-0018）。
- **一致性**：非快照一致（跨批次可能看到并发提交），文档明示；快照一致分页随 Iceberg 接入。
- **全文搜索**（原 QUERY-03）在 Trino 上降级为 `LIKE` / 正则 token 匹配，不提供相关度排序（tantivy 留 v2）。
- **Trino 内叠加 SQL** 只作为不承诺延迟的备选，不进入 L0 主路径。
- **延迟预算**（安静或隔离集群，繁忙共享集群只监测不承诺）：pk 读 p95 < 500ms；列表 / 过滤 p95 < 1s；聚合 p95 < 1.5s；Action p95 < 1s；已编辑对象 PG 直读 p95 < 50ms。不以 Trino resource group 为前提。
- **测试**：Rust 合并逻辑是唯一的生产叠加实现；proptest 对照内存参考模型，SQLite 替身与 Trino 分别验证单表查询语义（完整 proptest 只跑 SQLite，Trino 端跑固定语料 + 历史反例）；繁忙时段延迟对照列为 CI / 私有 fixture 验证项。

## Rejected
- Trino 内叠加作为主路径（ADR-0005 / 原 D2）：7 fragment，繁忙退化，广播与 256 值两个结构性陷阱。
- 以专用 Trino resource group 保障预算：只影响准入排队，实测排队 ≤ 1ms。
- 编译到 sea-query + 递归 CTE：业务数据不在 PG。

## Consequences
REQUIREMENTS 的 QUERY-08 改为「Rust 叠加对照参考模型、SQLite 替身与 Trino 的单表语义一致」。叠加规则只有一份生产实现，差分测试面缩小；Rust 编排需要补取循环、分批并行、聚合分解与 connector 能力表。
