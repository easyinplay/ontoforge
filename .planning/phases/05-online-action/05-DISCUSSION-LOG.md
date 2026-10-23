# Phase 5: Online Action - Discussion Log

> Audit trail only. Planning decisions are in `05-CONTEXT.md`.

**Date:** 2026-10-23
**Areas discussed:** 批量粒度、source 重复 pk 与离线执行、重放鉴权、参数约束

## 批量粒度

| Option | Description | Selected |
|--------|-------------|----------|
| 批级原子命令 | 一个 `command_id`、一条含多 entry 的 edit log、批级回执与事件 | Yes |
| 逐项身份 | 每项独立 `command_id` 与回执，共享事务 | No |

**Basis:** Grok 推荐批级幂等；ADR-0018 规定主体作用域，ADR-0026 规定一次 Commit 一条日志，因此修正 Grok 的全局键和逐 edit 日志建议。

## source 重复 pk 与离线执行

| Option | Description | Selected |
|--------|-------------|----------|
| 观测到即拒写 | 仅当 Action 实际读 source 时，重复 pk 返回 `AmbiguousObject`；已编辑对象可离线执行 | Yes |
| 每次查 source | 全 patch 的 source-backed 对象也要求 Trino 在线查重 | No |

**Basis:** Grok 定向复核认为无 PK registry 时两项绝对承诺不能并存；结合 ADR-0018 的 source 采样模型，选择观测语义。此项是 ACTION-12 的显式解释，后续审查须核验。

## 其他边界

- 重放重新检查当前 action 执行权及回执字段披露权限，不再次写入，不撤销已提交效果；删除对象后仍可按权限查询原回执。
- ACTION-01 使用现有 Action IR；SCHEMAX-04 的新 value types 留待后续。
- 原有 ADR 决定前置目标不可见与不存在同错，同 Action 新建对象不能供后续 rule 引用。

## Decision Authority

用户指示未决事项先取 Grok 第二意见。本次选择由主会话结合该意见与已接受的 ADR、代码合同作出，未将 Grok 建议直接视为项目决策。
