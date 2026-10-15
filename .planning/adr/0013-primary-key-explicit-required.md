# ADR-0013 主键属性必须显式声明 `required: true`

- status: accepted
- date: 2026-10-15
- supersedes: ADR-0012 §ObjectType 中「主键属性隐含 `required: true`」一句（其余内容不变）

## Context
ADR-0012 写的是主键属性隐含 `required: true`。plan 01-04 实现时改为要求显式声明：主键属性缺少 `required: true` 时报 ONT-V10。

## Decision
主键属性必须显式写 `required: true`，否则报 ONT-V10（hard error）。

## Rationale
- 定义文件写什么就是什么，读者不用记隐含规则。
- 更严格的规则以后可以向后兼容地放宽为隐式；反方向会让已有定义文件失效。
- ONT-V10 的诊断能直接指出缺失的字段并给出修复写法。

## Consequences
示例和 fixture 中所有主键属性都写 `required: true`。JSON Schema 的 description 需要说明这一要求（由 plan 01-09 体现）。
