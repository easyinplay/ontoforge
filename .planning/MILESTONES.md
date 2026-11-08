# Milestones

## v1.0 MVP (Shipped: 2026-11-08)

**Delivered:** ontoforge v1.0 交付完整 ontology 操作层：用 YAML/JSON 定义本体，绑定真实 Trino 数据源，经受校验的 Action 写入并留下不可变 edit log，再通过 scope 授权的 object set 查询读取，同一份本体经 REST 与 MCP 两个平级出口对业务应用与 AI agent 开放。

**Phases completed:** 9 phases（Phase 1-7、7.1、7.2），94 plans，115 tasks

**Key accomplishments:**
- ontology-as-code 与 lint 引擎：YAML/JSON 合并加载为单一 IR，`ontoforge validate` / `lint` 输出带 file:line:col 的诊断（ONT-E / ONT-W01..W11），导出 JSON Schema 供编辑器补全（Phase 1，14/14 plans）。
- 数据源绑定与 Trino 引擎：`SourceBinding` + 必填 `backing`、类型契约 lint（ONT-E13..E16）、自写 Trino client 与自有 SQL printer、SQLite 替身、Windows 零 Docker 全绿、Linux CI 跑 Trino + PG 与 nightly Iceberg（Phase 2 + 07.2，ENGINE-05 精确合同闭合）。
- edit store 与 scope 模型：按 object type 生成强类型 edit 表、append-only 不可变 edit log 与 `log::replay`、CEL 行谓词的 SQL 编译与 Rust 单行求值差分 proptest 一致、OIDC JWT 与 dev token 认证（Phase 3，23/23 plans）。
- scoped L0 读与 online Action：Rust 编排 source/edit 叠加的 pk 读、过滤、排序分页、searchAround、集合运算与聚合；写入只经 Action，带并发令牌、`command_id` 主体作用域幂等与事务性 webhook outbox（Phase 4 + Phase 5）。
- L2 Raw DQL、toolsets 与读审计：allow-list 内只读 Trino DQL 的 AST 白名单、catalog/schema 限制、L2 身份受限证明失败即禁用、读元数据审计 fail-closed（Phase 6，11/11 plans）。
- REST + MCP 外壳与发行：OpenAPI 3.1 快照测试、Rust / TypeScript client、按主体过滤的 MCP toolset、每个 action type 自动暴露为 MCP 工具、`ontoforge serve --dev` 单二进制持久化 dev 演示、三平台发行包（Phase 7，16/16 plans）。

> 完整的 56 条 SUMMARY one-liner 清单保留在 `.planning/milestones/v1.0-phases/*/*-SUMMARY.md`，此处只保留 6 条里程碑级要点。

**Stats:**
- 9 phases，94 plans，115 tasks
- 1,809 files changed（其中非 `.planning/` 610 files，148,151 insertions(+)）、`.planning/milestones/` 为归档产物
- 126,982 lines of Rust（`crates/**.rs`，tracked，11 个 crate）
- 25 days from start to ship（2026-10-15 → 2026-11-08），1,049 commits

**Git range:** `b4d11f6`（docs: initialize project）→ `b9a3700`（docs(02): Phase 02 验收闭环与阶段收尾记账）

**Closeout type:** `override_closeout`
**Known verification overrides:** 3 newly acknowledged, 0 carried forward from a prior close (see STATE.md Deferred Items)

**### Known Gaps**

里程碑审计（`.planning/milestones/v1.0-audit/SUMMARY.md`，dated 2026-11-05）的结论仍是 `gaps_found`，本条 milestone 关闭没有改写该审计文件，也不宣称审计通过。当前残余缺口：

1. **Phase 05 — 生产 webhook 联调未执行**：以真实部署的服务管理器、持久数据库和实际下游接收端联调 `ontoforge webhook-worker`（自动重启、503 重试、按 `event_id` 去重、至少一次送达）仍未做；WSL 探针只验证隔离环境机制，不能替代生产配置与接收端的验收。
2. **Phase 06 — 审计时间敏感项**：`security::cap_counts_final_masked_row_and_version` 用 16 MiB 行触发固定最终字节上限，与 read audit 的 1 秒 pending 期限共用墙钟；纯计算断言与时间敏感集成验证尚未拆开，CI 现用 `--test-threads=1`，不能声称慢机器上完全消除波动。
3. **Phase 07.2 / Phase 02 — ADR-0032 通用化只落到 planning 层**：产品层 `private_fixtures.rs` / `CONTRIBUTING.md` / `docs/designs/trino-engine-sql-tiers.md` 仍点名 SQL Server（代码无按类型分叉，不构成功能缺陷）。
4. **Phase 02 — `02-10-SUMMARY.md` 陈旧断言未追改**：本地 warehouse 描述与测试计数与当前状态不符，历史 SUMMARY 作为快照保留。
5. **审计报告门口径已过时**：审计记录的 8 项 unsatisfied 全部由当时 Phase 02 的 `gaps_found` 驱动；Phase 02 于 2026-11-08 单独闭环（`passed` 109/109、UAT 9/9）后该口径已失效，但审计文件作为 dated 证据未被改写。
6. **Phase 01 — G-1-2 编辑器体验观察限制**仍以 `01-UAT.md` Test 2 原文为准；该项已由 Phase 1 gap closure（01-14）落地自动化补全，剩余为人工观察边界。

上述 3 项审计开放项（Phase 07.2 的 UAT 状态串误报、Phase 05 的 webhook 联调、Phase 06 的审计时序项）已于 2026-11-08 经 `audit-open acknowledge` 记录为延后事项，见 STATE.md `## Deferred Items`。

**What's next:** 下一 milestone（候选 v2.0）尚未排期，方向候选见 `.planning/PROJECT.md` 的 Next Milestone Goals：湖仓写入与联邦计算（LAKE-01..03、SQL-01）、ontology 表达力扩展（SCHEMAX-01..05）、治理闭环（GOV-01）、检索与运行时（SEARCH / FUNC / SUB / SCEN / BRANCH / SDK / GQL）。建议先把上列 Known Gaps 作为 v1.x 技术债进入 backlog，再由 `/gsd-new-milestone` 重新讨论范围。

---
