# ADR-0006 表达式语言：CEL

- status: accepted
- date: 2026-10-15

## Decision
action submission criteria、参数校验、派生默认值用 CEL，crate `cel`（原 `cel-interpreter` 已改名）。进程内求值，非图灵完备，可静态检查。

## Rejected
`rhai`（图灵完备脚本，难以静态分析、有安全面）；自研表达式（成本高）。

## Open (spike)
`cel` 是否支持声明式变量类型检查；不支持则只做引用存在性检查。
