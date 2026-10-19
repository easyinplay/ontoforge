# ADR-0014 参数与属性 apiName 不得使用 CEL 保留字

- status: accepted
- date: 2026-10-16
- amends: ADR-0012（收紧命名约束，其余内容不变）

## Context
代码审查 WR-03 发现：参数可以命名为 CEL 关键字（如 `in`），通过 validate，但 submission criteria 里写 `params.in` 会报 CEL 语法错误。ADR-0012 的定义格式是对外契约，越早收紧迁移成本越低。

## Decision
参数 apiName 与属性 apiName 不得为 CEL 保留字（共 21 个，如 `in`、`for`、`null`、`true`），违反时报 ONT-V07（hard error，复用现有 apiName 校验码，不新增错误码）。link 端 apiName 暂不纳入：v1 的 CEL 上下文（`params` / `user` / `objects`）不引用 link 端名。

## Consequences
- 更严格的规则将来可以向后兼容地放宽，反方向会使已有定义失效，所以现在收紧。
- 若后续版本让 CEL 引用 link 端（如遍历），需要把 `LinkEndName` 一并纳入本约束。
- 实现与测试见 `.planning/phases/01-workspace-schema-lint/01-REVIEW-FIX.md`（WR-03）。
