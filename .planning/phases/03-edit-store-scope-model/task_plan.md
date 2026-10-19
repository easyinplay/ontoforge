# Phase 3 task_plan（索引）

> 本文件只做索引，任务细节、验收标准、文件路径的唯一来源是各 `03-NN-PLAN.md`。
> 需求见 `.planning/REQUIREMENTS.md`（STORE-01..07、SCOPE-01..07、AUTH-01..02），决策见 `03-CONTEXT.md`（D-01..D-18）与本 phase 产出的 ADR-0023..0026。

| Wave | Plan | 目标 | Requirements | depends_on | Checkpoint |
|------|------|------|--------------|------------|------------|
| 1 | 03-01 | 依赖与 crate 依赖方向（ADR-0023）、供应链守卫 | STORE-01, AUTH-01 | — | legitimacy（sea-query-sqlx、getrandom） |
| 1 | 03-02 | `indexed`、ONT-V21..V27、主体属性类型、`[store]` / `[auth]` | STORE-02, SCOPE-02, AUTH-01 | — | — |
| 2 | 03-03 | tracer：`ontoforge reload` 建 edit 表（SQLite / PG，ADR-0024） | STORE-01, STORE-02 | 03-01, 03-02 | set_mask 物理格式（one-way） |
| 2 | 03-04 | 行谓词 `Pred` 与 Kleene 求值（ADR-0025、`docs/scope.md`） | SCOPE-04, SCOPE-05, SCOPE-02 | 03-01, 03-02 | D-06 语义（one-way） |
| 2 | 03-05 | Role / RoleBinding 文档、scope 加载器、JSON Schema | SCOPE-01, SCOPE-02 | 03-02 | — |
| 2 | 03-06 | OIDC JWT 与 JWKS 缓存 | AUTH-01, SCOPE-01 | 03-01, 03-02 | — |
| 3 | 03-07 | commit、edit log、append-only、重放（ADR-0026） | STORE-03, STORE-01, STORE-07 | 03-03 | D-10 / D-11 日志格式（one-way） |
| 3 | 03-08 | 索引、1:1 部分唯一索引、物理名上限 ONT-V24 | STORE-02, STORE-05 | 03-03 | — |
| 3 | 03-09 | 行谓词降为 SQL、`StringMatch`、差分 proptest、多 role 合并 | SCOPE-04, SCOPE-03, SCOPE-05, SCOPE-02 | 03-04 | — |
| 3 | 03-10 | dev token 与 `ontoforge dev-token` | AUTH-02 | 03-06, 03-03 | — |
| 4 | 03-11 | 完整叠加状态机、参考模型 proptest、基数与并发 | STORE-07, STORE-05, STORE-01 | 03-07, 03-08 | — |
| 4 | 03-12 | scope 编译（V21 / V22 / V23、D-08）与 `for_principal` | SCOPE-02, SCOPE-05, SCOPE-06, SCOPE-01 | 03-04, 03-05, 03-06, 03-09 | — |
| 4 | 03-13 | `ontoforge migrate --plan/--apply`、改名、归档、清理 | STORE-02 | 03-07, 03-08, 03-10 | — |
| 5 | 03-14 | `ontoforge scope plan|apply`、管理审计、user 记录 | SCOPE-01, SCOPE-07, SCOPE-02, SCOPE-06 | 03-12, 03-13 | — |
| 5 | 03-15 | 变更历史 API 与 D-13 遮蔽 | STORE-04 | 03-09, 03-11 | — |
| 6 | 03-16 | reload 加固：栅栏、lock_timeout、D-18、Trino 可见探测 | STORE-02, SCOPE-06, SCOPE-07 | 03-11, 03-13, 03-14 | — |
| 6 | 03-17 | `ontoforge import`（editOnly） | STORE-06 | 03-11, 03-14 | — |
| 7 | 03-18 | Linux CI（PG conformance、Trino 语料与探测）、`docs/edit-store.md` | STORE-01, SCOPE-04, STORE-02 | 03-09, 03-14, 03-16 | human-check（CI 首跑） |

## 验收

Phase 完成 = 18 个 PLAN 的 SUMMARY 落盘 + `03-VERIFICATION.md` passed + Windows / Ubuntu `cargo test --workspace --locked` 绿 + PR `integration` 作业中 PG conformance、Trino 行谓词语料与 reload 可见探测全部实跑通过（ignored 守卫计数 > 0）。ROADMAP Phase 3 的 5 条 Success Criteria 与 must_haves 的映射见各 PLAN。

## 规划中的自决项（orchestrator / planner，用户可否决）

- 诊断码用 ONT-V21..V27（不可关闭）而非 RESEARCH 草案的 E17..E19（03-02 objective 记录理由）。
- 多 issuer 受信时组名写作 `{iss}|{group}`；`[[auth.oidc]]` 为数组（orchestrator 决定 8）。
- dev token 为 HS256、密钥 32 字节、只能经 `DevMode` 凭证启用（03-10）。
- edit store 取值编码：timestamp 为 UTC 毫秒精度且输入必须带偏移（03-07 checkpoint 确认）。
