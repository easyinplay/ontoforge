# ADR-0004 存储：StorageProvider trait，Postgres 生产 / SQLite dev-test

- status: superseded by ADR-0016（2026-10-16）
- date: 2026-10-15
- 依据：`.planning/research/rust-stack.md` §2

## Decision
- `StorageProvider` trait + `capabilities()`；两个后端：PostgreSQL（生产）、SQLite（dev / test / 单二进制演示）。均用 `sqlx`。
- 一套 conformance 测试两个后端都跑；Windows 本机 `cargo test` 默认只需 SQLite，零外部依赖。
- 通用表设计：对象属性存 JSON（PG `jsonb` / SQLite JSON1），ontology 变更不触发 DDL；link 独立成表，基数用 partial unique index 强制。
- 写入只经 Action；每次提交写入 append-only `of_edit` 与 `of_object_history`。edit log 格式在 MVP 定死（scenario / branch 后续以其为覆盖层）。
- SQLite 定位为 dev/test，不承诺生产（可逆决定，未来可升级）。
- 本机 PG 测试用 `postgresql_embedded`，Linux CI 用 service container。
