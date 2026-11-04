# ADR-0023 Phase 3 crate 依赖方向修正

- status: accepted
- date: 2026-10-20
- decider: Claude（按用户「决策自主」偏好落地 research 推荐）
- 修订：ADR-0011（crate 布局中 storage 与 auth 的依赖）；`docs/designs/trino-engine-sql-tiers.md`「Crate 布局」节的依赖方向行
- 依据：`.planning/phases/03-edit-store-scope-model/03-RESEARCH.md` 依赖方向节、设计文档 Crate 布局

## Context
设计文档把依赖方向写成 `schema/expr → auth → query → engine/storage → action → api/server`，ADR-0011 的占位 manifest 里 `ontoforge-storage` 依赖 `ontoforge-query`。Phase 3 的实现需要的方向与此不同：

- 行谓词要降为 printer AST（D-06 / SCOPE-04），所以 `ontoforge-auth` 必须依赖 `ontoforge-engine`。
- 历史遮蔽（D-13）与 reload 时对 role 的复核（D-18）需要 `TypeScope` / `compile_scope`，所以 `ontoforge-storage` 必须依赖 `ontoforge-auth`。
- Phase 4 的叠加读要读 edit store，所以 `ontoforge-query` 依赖 `ontoforge-storage`，而不是反过来。

## Decision
Phase 3 起的依赖方向：

- `schema ◄ expr`（expr 依赖 schema）
- `engine → schema`
- `auth → schema, expr, engine`
- `storage → schema, expr, auth, engine`
- `query`（Phase 4）`→ storage, auth, engine`
- `action → storage, auth, expr`

依赖图无环。`ontoforge-storage` 不再依赖 `ontoforge-query`。CLI crate `ontoforge` 增加对 `ontoforge-auth` 与 `ontoforge-storage` 的依赖。

## Rejected
- **保留占位的 storage → query**：Phase 4 叠加读需要读 edit store，方向相反，保留会在 Phase 4 形成环。

## Consequences
- Phase 3 的全部新依赖与 feature（sqlx 的 postgres / migrate / macros / json / chrono / tls-rustls-aws-lc-rs、sea-query、sea-query-sqlx、jsonwebtoken 的 aws_lc_rs、getrandom、postgresql_embedded）与全部 crate manifest 在 plan 03-01 一次声明；后续 plan 不修改任何 `Cargo.toml` 与 `Cargo.lock`，它们的 cargo 命令一律带 `--locked`。
- `ontoforge-engine` 把 `DECIMAL_COLLATION`、`TIMESTAMP_COLLATION`、`compare_decimal`、`compare_timestamp` 改为公开导出，storage 的 SQLite 连接与 SQLite 替身共用同一实现。
- CI `test` 作业增加 `cargo tree -i openssl` / `-i native-tls` 守卫（含 `embedded-pg` feature），依赖图中出现第二个 TLS / crypto 栈即失败。
- 设计文档的依赖方向行追加指向本 ADR 的说明。
