# ADR-0009 客户端 SDK

- status: accepted
- date: 2026-10-15

## Decision
- Rust client 手写，与服务端共享 `ontoforge-api` DTO。
- TS 平台层：`openapi-typescript` + `openapi-fetch` 从 OpenAPI 生成。
- ontology 级强类型 SDK（类 OSDK）codegen 放在 MVP 之后。
- 不用 `progenitor`（只支持 OpenAPI 3.0.x，与 utoipa 3.1 不兼容）。
