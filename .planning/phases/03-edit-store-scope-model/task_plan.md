# Phase 3 task_plan（索引）

> 本文件只做索引，任务细节、验收标准、文件路径的唯一来源是各 `03-NN-PLAN.md`。
> 需求见 `.planning/REQUIREMENTS.md`（STORE-01..07、SCOPE-01..07、AUTH-01..02），决策见 `03-CONTEXT.md`（D-01..D-18）与本 phase 产出的 ADR-0023..0026。

| Wave | Plan | 目标 | Requirements | depends_on | Checkpoint |
|------|------|------|--------------|------------|------------|
| 1 | 03-01 | 依赖与 crate 依赖方向（ADR-0023）、供应链守卫 | STORE-01, AUTH-01 | — | legitimacy（sea-query-sqlx、getrandom） |
| 1 | 03-02 | `indexed`、ONT-V21..V27、主体属性类型、`[store]` / `[auth]` | STORE-02, SCOPE-02, AUTH-01 | — | — |
| 2 | 03-03 | tracer：`reload` 库函数在 SQLite 建 edit 表、SQL 出口守卫（ADR-0024） | STORE-02, STORE-01 | 03-01, 03-02 | set_mask 物理格式（one-way） |
| 2 | 03-05 | 行谓词 `Pred` 与 Kleene 求值（ADR-0025、`docs/scope.md`） | SCOPE-04, SCOPE-05, SCOPE-02 | 03-01, 03-02 | D-06 语义（one-way） |
| 2 | 03-06 | Role / RoleBinding 文档、scope 加载器、JSON Schema | SCOPE-01, SCOPE-02 | 03-02 | — |
| 2 | 03-07 | OIDC JWT 与 JWKS 缓存 | AUTH-01, SCOPE-01 | 03-01, 03-02 | — |
| 3 | 03-04 | `ontoforge reload` CLI、PG 后端、两种方言 DDL 快照 | STORE-01, STORE-02 | 03-03 | — |
| 3 | 03-11 | 行谓词降为 SQL、`StringMatch`、差分 proptest、多 role 合并 | SCOPE-04, SCOPE-03, SCOPE-05, SCOPE-02 | 03-05 | — |
| 4 | 03-08 | commit 写路径、edit log、conformance 框架 | STORE-03, STORE-01, STORE-07 | 03-04 | D-10 / D-11 日志格式（one-way） |
| 4 | 03-10 | 索引、1:1 部分唯一索引、物理名上限 ONT-V24 | STORE-02, STORE-05 | 03-04 | — |
| 4 | 03-12 | dev token 与 `ontoforge dev-token` | AUTH-02 | 03-07, 03-04 | — |
| 4 | 03-14 | scope 编译（V21 / V22 / V23、D-08）与 `for_principal` | SCOPE-02, SCOPE-05, SCOPE-06, SCOPE-01 | 03-05, 03-06, 03-07, 03-11 | — |
| 5 | 03-09 | append-only 触发器、重放、重复提交（ADR-0026） | STORE-03, STORE-01 | 03-08 | — |
| 5 | 03-15 | `ontoforge migrate --plan/--apply`、改名、归档、清理 | STORE-02 | 03-08, 03-10, 03-12 | — |
| 6 | 03-13 | 完整叠加状态机、参考模型 proptest、基数与并发 | STORE-07, STORE-05, STORE-01 | 03-09, 03-10 | — |
| 6 | 03-16 | `ontoforge scope plan|apply`、管理审计、user 记录 | SCOPE-01, SCOPE-07, SCOPE-02, SCOPE-06 | 03-09, 03-14, 03-15 | — |
| 7 | 03-17 | 变更历史 API 与 D-13 遮蔽 | STORE-04 | 03-11, 03-13 | — |
| 7 | 03-18 | reload 加固：栅栏、lock_timeout、D-18、Trino 可见探测 | STORE-02, SCOPE-06, SCOPE-07 | 03-13, 03-15, 03-16 | — |
| 7 | 03-19 | `ontoforge import`（editOnly） | STORE-06 | 03-13, 03-16 | — |
| 8 | 03-20 | Linux CI（PG conformance、Trino 语料与探测）、`docs/edit-store.md` | STORE-01, SCOPE-04, STORE-02 | 03-11, 03-16, 03-18 | human-check（CI 首跑） |
| G1 | 03-21 | gap 1：decimal 不舍入（CR-A01），并入 WR-A01、WR-A03 | STORE-03, STORE-06, STORE-01 | — | — |
| G1 | 03-22 | gap 3：migrate 复核 role（WR-B07）、整对象 criteria 读取（WR-C05） | SCOPE-06 | — | — |
| G2 | 03-23 | gap 2：示例配置全新检出可用（CR-D01），记录 Windows 密钥 ACL 缺口 | AUTH-02 | 03-21（同改 store.rs、docs/edit-store.md） | — |

## 验收

Phase 完成 = 23 个 PLAN 的 SUMMARY 落盘 + `03-VERIFICATION.md` passed + Windows / Ubuntu `cargo test --workspace --locked` 绿 + PR `integration` 作业中 PG conformance、Trino 行谓词语料与 reload 可见探测全部实跑通过（ignored 守卫计数 > 0）。ROADMAP Phase 3 的 5 条 Success Criteria 与 must_haves 的映射见各 PLAN。

## 当前 UAT 补证任务

- [x] V1（STORE-02）：隔离验证 Trino 元数据缓存 flush。修改 `ci/trino/catalog/editstore.properties`、`crates/ontoforge-storage/tests/reload_trino_it.rs`、`.github/workflows/ci.yml`；独立 Trino/PostgreSQL 中缓存开启且预热后，无 flush 时新列不可见、旧定义仍 active，真实 flush 后新列可见且新定义 active；串行集成测试 3/3 通过，PR CI run 39712341919 三 job 通过。证据见 `03-UAT.md`；此项不是新的 `gap_closure` PLAN。

## Phase 3 完成后的 CI 稳定性跟进

- [ ] V2：诊断并稳定 `reload_racing_commits_never_lets_an_old_write_through` 在 Windows CI 上的 `SQLITE_BUSY`。所有权：`crates/ontoforge-storage/tests/reload.rs`。已确认竞争路径并完成仅对 SQLite code `5` 的有界重试；其他错误立即失败，并发写入、旧版本拒绝和日志单调断言保留。本地目标测试 20/20、`reload` 测试集 32/32 通过；待 PR 的 Windows、Ubuntu、integration CI。此项是已完成 Phase 3 的测试稳定性跟进，不重新打开阶段验收。

## 规划中的自决项（orchestrator / planner，用户可否决）

- 诊断码用 ONT-V21..V27（不可关闭）而非 RESEARCH 草案的 E17..E19（03-02 objective 记录理由；D-07「报 E」的细化见 03-CONTEXT D-07 的 Refinement 注记）。
- 多 issuer 受信时组名写作 `{iss}|{group}`；`[[auth.oidc]]` 为数组（orchestrator 决定 8）。
- dev token 为 HS256、密钥 32 字节、只能经 `DevMode` 凭证启用（03-12）。
- edit store 取值编码：timestamp 为 UTC 毫秒精度且输入必须带偏移（03-08 checkpoint 确认）。
- plan-checker iteration 1（2026-10-20）：原 reload plan 拆为 03-03（库级 tracer + 物理格式）/ 03-04（CLI + PG），原 edit log plan 拆为 03-08（格式 checkpoint + commit）/ 03-09（append-only + 重放），其余顺延编号（原 04..06 → 05..07，原 08..18 → 10..20）；03-01、03-06、03-11 的文件数判定写在各自 objective。
