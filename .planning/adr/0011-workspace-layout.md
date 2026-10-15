# ADR-0011 Cargo workspace 布局（精简起步）

- status: accepted
- date: 2026-10-15

## Context
选型调研提出 22 个 crate。按 simplicity 原则先按稳定边界收敛，确有需要再拆。

## Decision
```
crates/
  ontoforge-schema/    # 基础类型 + IR + YAML/JSON loader + lint
  ontoforge-expr/      # CEL 封装
  ontoforge-query/     # ObjectSet AST + 编译到 sea-query
  ontoforge-storage/   # StorageProvider trait + sqlite / postgres（cargo feature）+ conformance 测试
  ontoforge-action/    # action 执行、edit log
  ontoforge-auth/      # authn + authz
  ontoforge-api/       # wire DTO（server 与 client 共享）
  ontoforge-server/    # axum REST + rmcp MCP
  ontoforge-client/    # Rust client
  ontoforge/           # CLI bin：validate / lint / serve / mcp
examples/ontologies/   # 示例 ontology（兼作测试 fixture）
```
edition 2024，resolver 3，rust-version 1.98。依赖方向：schema ← expr / query ← storage ← action ← server ← cli；api 只依赖 schema 与 query，client 不依赖 server 侧 crate。
