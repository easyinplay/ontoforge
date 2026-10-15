# ADR-0006 表达式语言：CEL

- status: accepted
- date: 2026-10-15

## Decision
action submission criteria、参数校验、派生默认值用 CEL，crate `cel`（原 `cel-interpreter` 已改名）。进程内求值，非图灵完备，可静态检查。

## Rejected
`rhai`（图灵完备脚本，难以静态分析、有安全面）；自研表达式（成本高）。

## Spike 结论（2026-10-15）
`cel` 0.15.0 无声明式变量类型检查。静态检查范围定为：语法、自由变量根必须 ∈ {`params`, `user`, `objects`}、`params.<name>` 必须是已声明参数、`objects.<name>.<prop>` 必须是 objectReference 参数且属性存在于目标 object type、`user.<field>` 必须 ∈ {`id`, `groups`}、顶层结果类型在可推断时必须为 bool、比较两侧在可推断时类型兼容；类型推断为 `ontoforge-expr` 自建的保守推断（未知即 Any，不报错）。`references()` 对 comprehension 变量有误报，改用自写 AST 遍历。解析必须在大栈线程内执行并预限长度与嵌套深度（debug + Windows 主线程括号嵌套 ≥ 10 层栈溢出）。
