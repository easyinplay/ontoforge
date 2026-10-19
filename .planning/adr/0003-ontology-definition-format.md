# ADR-0003 Ontology 定义格式：YAML / JSON → 单一 IR

- status: accepted（IR 数据源绑定与类型契约由 ADR-0015 扩展，2026-10-16）
- date: 2026-10-15
- 依据：`.planning/research/rust-stack.md` §1

## Decision
- ontology-as-code 以 YAML 为主、JSON 等价，二者只解析到同一份 IR（`ontoforge-schema`）。
- 解析：`serde-saphyr`（带 span，配合 `miette` 输出 line:col 诊断）；不用已 deprecated 的 `serde_yaml`。
- 导出 JSON Schema（`schemars`）供编辑器补全与 LLM 生成。
- IR 是唯一契约，运行时、存储、API、codegen 均只读 IR（避免 AST/运行时双 schema）。

## Rejected
GraphQL SDL 方言（directive 表达行为能力弱、工具链需自建）、KDL（生态小、LLM 不熟）、Rust macro DSL（非 Rust 用户无法编写）。

## Spike 结论（2026-10-15）
`serde-saphyr` 2.0.0 的 `Spanned<T>` 在 `#[serde(tag)]` / `untagged` / `flatten` 内丢失 span，`deny_unknown_fields` 错误无位置，故不能「直接反序列化进 IR 并按路径取 span」。决定：用 `Spanned` 构建节点树（YAML 与 JSON 同一路径）→ `serde_json::Value` → `serde_path_to_error` → 路径回查 span。IR 不含 span，span 存于并行 `SourceMap`。Plan B：`saphyr` 0.2.2 `MarkedYaml`（已验证 key/value span）。`.json` 文件额外用 `serde_json` 做严格语法预检（YAML 解析器会接受尾逗号与注释）。

字段级文档形状见 ADR-0012。
