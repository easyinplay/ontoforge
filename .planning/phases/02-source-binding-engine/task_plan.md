# Phase 2 task_plan（索引）

> 本文件只做索引，任务细节、验收标准、文件路径的唯一来源是各 `02-NN-PLAN.md`。
> 需求见 `.planning/REQUIREMENTS.md`（BIND-01..03、ENGINE-01..05），决策见 `02-CONTEXT.md` 与 ADR-0021（由 02-01 产出）。

| Wave | Plan | 目标 | Requirements | depends_on |
|------|------|------|--------------|------------|
| 1 | 02-01 | `backing` 必填 + Phase 1 fixture 迁移 + ADR-0021 | BIND-01 | — |
| 1 | 02-02 | `ontoforge-engine` crate：Engine trait、printer、SQLite 执行 | ENGINE-03, ENGINE-04 | — |
| 2 | 02-03 | `SourceBinding` kind、profile 选定、aviation 绑定 | BIND-01, BIND-02 | 02-01 |
| 2 | 02-04 | Trino client 主路径、server config、认证 | ENGINE-01, ENGINE-02 | 02-02 |
| 3 | 02-05 | tracer：`validate --online` 经 SQLite 替身 | BIND-03, ENGINE-04 | 02-02, 02-03, 02-04 |
| 3 | 02-06 | 不可用分类、重试、取消 | ENGINE-01, ENGINE-02 | 02-04 |
| 3 | 02-07 | lint ONT-E13..E16、ONT-W17 | BIND-02 | 02-03 |
| 4 | 02-08 | 类型兼容表、drift 模型、`docs/source-binding.md` | BIND-02, BIND-03 | 02-05 |
| 4 | 02-09 | Trino 在线校验、退出码 3、`--check-pk` | BIND-03, ENGINE-01, ENGINE-02 | 02-05, 02-06 |
| 5 | 02-10 | Linux CI（Trino + PG）、nightly Iceberg | ENGINE-05, ENGINE-01, ENGINE-03 | 02-08, 02-09 |

## 验收

Phase 完成 = 全部 PLAN 的 SUMMARY 落盘 + `02-VERIFICATION.md` passed + Windows / Ubuntu `cargo test --workspace --locked` 绿 + Linux 集成 job 实际执行 `--ignored` 测试。
