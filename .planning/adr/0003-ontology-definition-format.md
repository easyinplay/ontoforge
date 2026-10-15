# ADR-0003 Ontology 定义格式：YAML / JSON → 单一 IR

- status: accepted
- date: 2026-10-15
- 依据：`.planning/research/rust-stack.md` §1

## Decision
- ontology-as-code 以 YAML 为主、JSON 等价，二者只解析到同一份 IR（`ontoforge-schema`）。
- 解析：`serde-saphyr`（带 span，配合 `miette` 输出 line:col 诊断）；不用已 deprecated 的 `serde_yaml`。
- 导出 JSON Schema（`schemars`）供编辑器补全与 LLM 生成。
- IR 是唯一契约，运行时、存储、API、codegen 均只读 IR（避免 AST/运行时双 schema）。

## Rejected
GraphQL SDL 方言（directive 表达行为能力弱、工具链需自建）、KDL（生态小、LLM 不熟）、Rust macro DSL（非 Rust 用户无法编写）。

## Open (spike)
`serde-saphyr` 能否按 IR 路径取 span。
