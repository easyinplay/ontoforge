# ADR-0005 查询层：自有 ObjectSet AST → sea-query

- status: accepted
- date: 2026-10-15
- 依据：`.planning/research/rust-stack.md` §3

## Decision
- ObjectSet AST（filter / search / searchAround / union / intersect / subtract / aggregate / orderBy / page）自定义，同时作为 wire 格式。
- 编译器方言无关，生成 `sea-query`，再由后端选 PG / SQLite 方言；遍历用递归 CTE。
- 全文搜索 MVP 用后端原生（PG `tsvector` / SQLite FTS5），`tantivy` 留 v2。
- 编译器与集合代数律用 `proptest` 验证，并做 SQLite 与 PG 结果等价测试。
