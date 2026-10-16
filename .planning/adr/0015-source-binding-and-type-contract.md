# ADR-0015 数据源绑定与类型契约

- status: accepted
- date: 2026-10-16
- 扩展：ADR-0003（IR 新增可选字段，向后兼容；不取代）
- 依据：`docs/designs/trino-engine-sql-tiers.md` §1、Eng Review D8 / D18 / D23

## Context
业务数据留在企业已有平台，经 Trino 读取；ontology 需要把 object type 映射到 Trino 物理表，且同一个值在 Trino、PG edit store、Rust 叠加三处必须比较得一样。

## Decision
- object type 增加可选 `source: {catalog, schema, table, columns}`；未列出的属性按同名列。无 `source` 的 object type 是 edit-only。M:N link 的 join 表同样可绑定。
- 物理名只出现在定义文件与 L2 / L3；L0 的输入、输出、错误一律只用 apiName。
- 类型契约：
  - `decimal` 增加可选 `precision` / `scale`（缺省 38, 10）；Rust 侧用支持 38 位的十进制类型比较，传输一律规范化字符串。
  - source 列为不带时区的 timestamp 时，绑定必须声明 `timezone`，否则 lint E。
  - `geopoint` 本阶段不支持 source 绑定（lint E）；`array` ↔ Trino array / PG array。
  - 绑定可声明列的 `collation`；声明为大小写不敏感的列，编译器禁止下推字符串比较与排序。
- 主键契约：声明即信任。按 pk 读取时 source 查询带 `LIMIT 2`，返回多行即对该 pk 报 `AmbiguousObject` 并拒绝针对它的 Action；列表结果在 Rust 侧按 pk 去重并告警；type 级降级只由 `ontoforge validate --online --check-pk` 显式触发。
- `validate --online` 对 Trino `information_schema` 校验绑定（列存在、类型兼容）；启动与定时 drift 校验不匹配时该 object type 降级（`SourceSchemaDrift`），其余照常。

## Rejected
- Decimal / Timestamp 全程按字符串透传：Rust 路径排序与范围比较会与 Trino 不一致。
- 启动与定时全量查重 pk：亿级湖仓表每次全表扫描，占用共享集群。
- 发现一次重复 pk 即降级整个 type：一次 ETL 抖动就让整类对象下线。

## Consequences
Phase 1 IR、JSON Schema 与 lint 需新增上述字段与规则；绑定规范文档写明类型映射表。
