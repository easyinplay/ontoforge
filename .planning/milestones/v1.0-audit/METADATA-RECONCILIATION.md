# SUMMARY completion metadata 对账

日期：2026-11-04。状态：已按授权完成6项P03补录，5份SUMMARY解析与diff检查通过；STORE-05保持未完成，P06/07待后续授权。

## 范围与边界

本任务依照 `planning-with-files` 使用既有 `v1.0-audit` 计划，worker维护本文件及主会话授权的03-20/17/16/11/14-SUMMARY.md。对账对象为独立 `REQUIREMENTS-AUDIT.md` / `requirements-matrix.json` 的 15 个 completion metadata gap；6 份 SUMMARY 缺字段与27份显式空字段是独立统计，不意味着应逐份补完成声明。

保留历史 `completed` 日期和原子 slice 的历史语义。后补声明必须明确 `reconciled` 日期、累计交付范围与独立证据；本任务不修改源码、VERIFICATION、全局 REQUIREMENTS 或 STATE，不提交、不运行 Cargo 或服务。

## 计划

- [x] 读取独立三源审计、矩阵及主会话计划。
- [x] 核对最终交付 SUMMARY / PLAN 与最新独立 verifier。
- [x] 检查 GSD `summary-extract` 支持字段，形成最小修改清单。
- [x] 主会话确认5份P03 SUMMARY ownership，完成6项授权元数据修正与解析核验。

## 已确认事实

- 原始审计快照为 `fa81ca0271fb1d620e8217824a0bf2619e3235ed`，64/79 是历史三源声明覆盖，不是当前实现通过率。
- 原始缺失 IDs：STORE-02/04/05、SCOPE-01/02/03/05、L2-01/02/03、API-02、MCP-02/03/04/05。
- Phase 3 当前 canonical 已重新验证，后补声明可依据新证据评估；Phase 6/7 尚需按最新报告区分。
- 读取工具的首次 GSD 路径猜测不存在；已发现本机安装目录为 `C:/Users/easyi/.codex/gsd-core`，下一步直接查该目录。

## 验证与限制

本任务为文档元数据对账，跳过 TDD；验收使用字段解析、证据链接和三源语义检查。Warp 与 ralph-loop 未提供，沿用主计划明确的验收闭环，不声称调用这两项工具。

## 字段协议与三源语义

已读取 `C:/Users/easyi/.codex/gsd-core/bin/lib/commands.cjs:2910`，`summary-extract` 将 `requirements-completed` 或兼容的 `requirements_completed` 输出为 `requirements_completed`；缺字段与空数组都输出 `[]`，不会读取 `requirements-addressed`、`requirements` 或自定义 reconciliation 字段作为完成。

实跑以下两次 `summary-extract --fields requirements_completed` 均 exit 0：03-20 输出 `[STORE-01, SCOPE-04]`；07-05 输出 `[]`。因此最小方案仍使用既有 `requirements-completed`，不发明第二个机器完成字段；另加 `requirements-reconciled: "2026-11-04"` 和紧邻字段的说明，以及正文 `## 2026-11-04 元数据对账`，显式记下新增 IDs、累计证据和限制。`requirements-reconciled` 为人工审阅注释性字段，不声称 GSD 消费它。

原 `completed`、原 commit、原测试记录、原“当时未完成”段落保留；新段落说明完成声明是本次独立复核后的累计判断，不把当年的原子 slice 改写为全需求已完成。三源分别是：REQUIREMENTS 的当前归属/勾选、当前 VERIFICATION 的独立行为判断、SUMMARY 的 dated 交付归并；SUMMARY 不能替代另外两源。

## 可现在补录的 6 项（5 份文件）

以下6项已获主会话授权并执行。全部由新 [03-VERIFICATION.md](../../phases/03-edit-store-scope-model/03-VERIFICATION.md) 的逐项需求表与 [PHASE-01-03-REVERIFY.md](PHASE-01-03-REVERIFY.md) 支持；不是旧 SUMMARY 自证。报告时间为 `2026-11-04T02:11:14Z`，但后续STORE-05纠正与本次SUMMARY改动需要verifier复签；本文件不宣称改动后的canonical freshness已通过。

| ID | 建议 SUMMARY / 字段变化 | 累计交付证据与历史边界 |
|---|---|---|
| STORE-02 | [03-20-SUMMARY.md](../../phases/03-edit-store-scope-model/03-20-SUMMARY.md)：原 `[STORE-01, SCOPE-04]` 追加 STORE-02 | 此计划是 reload/migrate 的真实 PG/Trino CI 收口，PLAN 已含 STORE-02。新 verifier SC1/SC8、需求表明确 reload11/migrate4/Trino3、真实 cache flush 隔离与栅栏。保留旧 Requirements 中 not ticked 和当时三个理由；新段落引用当前累计结论，不声称每个历史 advisory 已修复。 |
| STORE-04 | [03-17-SUMMARY.md](../../phases/03-edit-store-scope-model/03-17-SUMMARY.md)：`[]` → `[STORE-04]` | 最终 history 库交付，原文明确等待 Phase7 暴露；新 P03 verifier 不只核库，独立 ledger 已亲核 `pg_http_mixed_snapshot_reload_revoke_and_history` 与 server `history_page → history_with_snapshot`。补录写明后继 transport 证据，不能冒称03-17当时已完成HTTP。 |
| SCOPE-01 | [03-16-SUMMARY.md](../../phases/03-edit-store-scope-model/03-16-SUMMARY.md)：与下一行合并为 `[SCOPE-01, SCOPE-02]` | 03-06/07/14/16 累计 Role/RoleBinding、OIDC groups、plan/apply、审计，由新 verifier 逐项SATISFIED。保留原文 request-time/downstream 待办为历史。 |
| SCOPE-02 | 同 03-16 | 最终持久化与管理员入口组合03-02/05/06/11/14，角色权限由 compile_scope 完整校验。新 verifier需求表单独确认；不扩大为Phase4/5/7全部已完成。 |
| SCOPE-03 | [03-11-SUMMARY.md](../../phases/03-edit-store-scope-model/03-11-SUMMARY.md)：`[]` → `[SCOPE-03]` | merge/column predicate 的本体交付文件；新 verifier 明确9 merge tests及SQL差分，ledger亲核多role不产生列×行泄露。保留原文当时loading/apply/scoped reads尚待后继，dated段引用后继累计验证。 |
| SCOPE-05 | [03-14-SUMMARY.md](../../phases/03-edit-store-scope-model/03-14-SUMMARY.md)：`[]` → `[SCOPE-05]` | compile_scope 为03-05/11后最终非法谓词/授权构建关口。新 verifier明确no_grant不可见、缺属性Unknown不通过、编译拒绝。缺属性的实际语义按ADR-0025，不把旧简写false扩称为任意显式null分支均拒绝。 |

选择对应最终交付点而不是把这6项集中塞入03-23：03-23只关闭AUTH-02/dev目录问题；这样做会脱离PLAN范围。同样保留早期03-01..10等prerequisite的显式空数组。

## 暂不补录的 9 项

| ID | 拟用最终交付 SUMMARY | 等待条件 |
|---|---|---|
| STORE-05 | 保持03-13原metadata；source收口归 [Phase7.1](../../phases/07.1-source-cardinality-gap-closure/07.1-CONTEXT.md) | 独立核查确认source报告缺失，03-VERIFICATION已修正为PARTIAL（全需求）、edit子合同通过。本轮绝不追加completed；待Phase7.1实际行为验收后记录其交付。 |
| L2-01 | [06-08-SUMMARY.md](../../phases/06-l2-raw-dql-toolsets-audit/06-08-SUMMARY.md)，与L2-02合并数组 | PLAN已覆盖，原SUMMARY明确等待真实远端CI/Phase6总验证。等待新版06-VERIFICATION与必要CI/服务证据；不填06-06/07原子slice。 |
| L2-02 | 同06-08 | 同上；AST白名单与Display、104-ID corpus须沿用真实证据及constructed-equivalent出处边界。 |
| L2-03 | [07-13-SUMMARY.md](../../phases/07-rest-mcp-shell/07-13-SUMMARY.md)，增加当前收口的completed声明 | P06库和P07 transport共同承担。只有新的P06/P07独立报告、最终CI/真实服务门及INT-01关闭均成立后，才能以dated增补记录本次跨phase最终集成验收。07-13原PLAN未列L2-03，因此须在增补明确其来自本次审计新增P07责任，不倒写原PLAN；如主会话认为需单独task SUMMARY，应先选定新文件再改。当前全局仍unchecked，禁止现在补完成。 |
| API-02 | [07-05-SUMMARY.md](../../phases/07-rest-mcp-shell/07-05-SUMMARY.md)：增加 `[API-02]` | 最终静态OpenAPI交付点，PLAN和原TDD直证完整。暂等新版07-VERIFICATION归并最终源码/CI，不能用旧80/80已撤回头部冒称当前passed。 |
| MCP-02 | [07-07-SUMMARY.md](../../phases/07-rest-mcp-shell/07-07-SUMMARY.md)：合并增加 `[MCP-02, MCP-03, MCP-04, MCP-05]` | describe完整结构、真实scoped内容；等新P07报告。 |
| MCP-03 | 同07-07 | object-set/filter/search/searchAround/aggregate与分页/授权；等新P07报告。 |
| MCP-04 | 同07-07 | 动态Action schema及后继07-13 schema/validator修复；保留07-07当时Clippy未通过与后续归属，不将其改写。 |
| MCP-05 | 同07-07 | validate-only、并发与跨transport零业务写；等新P07报告及PG真实证据。 |

07-13作为L2-03增补家的选择是待主会话确认的ownership/归属建议；不由worker自扩范围。当前6项metadata已补、8项待最终证据、1项STORE-05为已确认source缺口；这不是功能通过率。

## 6 份缺字段 SUMMARY 的处理

07-05与07-07按上表等待后补明确完成项；07-13仅在跨phase新收口有明确授权时增补。07-04、07-06、07-10没有造成这15项全局缺口，不为消除缺字段计数批量加完成项或空数组。它们现有PLAN/实际交付记录继续保留；后续若做格式规范化，应作为独立动机说明。原27份显式空中仅上表4份P03与1份P06最终交付文件拟改变，其余保留。

## ownership 与执行顺序

1. 当前worker独占本ledger及已授权5份P03 SUMMARY；其他候选等证据和主会话归属确认。
2. 修改每份 frontmatter 的既有completed数组或新增字段，并追加dated证据段；不改原历史正文、不改PLAN、VERIFICATION、全局REQUIREMENTS/STATE。
3. 对实际修改逐份运行GSD `summary-extract --fields requirements_completed` 与 `git diff --check`，核对ID总数、保留原ID和证据路径存在。
4. **fingerprint依赖：** 新03及旧07的`covered_files`均包含SUMMARY。补录后canonical可能转stale，必须通知对应verifier复核纯文档diff后重新计算fingerprint。worker不得自行更新VERIFICATION；不能把metadata变更说成业务退化，也不能直接冒充行为重跑。
5. 三源矩阵由主会话或原审计owner重新生成。旧REQUIREMENTS-AUDIT是有时间/HEAD边界的快照，不在此处覆盖其15项原始统计；当前结果通过新增对账和新版审计表达。

## 本轮验收

- [x] 15个缺失ID逐一定位，不批量填27个空字段。
- [x] 6项已补、8项待证据与1项真实缺口明确区分；STORE-05已由主会话确认并归Phase7.1。
- [x] 读取GSD实际解析实现，并对已有/缺失字段各执行一次成功解析。
- [x] 保留旧完成日期、原子slice历史语义、动态证据来源与canonical fingerprint依赖。
- [x] 主会话确认SUMMARY ownership，实施获准修改后复核。

## 2026-11-04 实施与验收结果

实际修改文件及GSD `summary-extract --fields requirements_completed` 输出（5次均exit 0）：

| SUMMARY | 完整输出数组 | 本轮新增ID |
|---|---|---|
| 03-20 | `[STORE-01, SCOPE-04, STORE-02]` | STORE-02 |
| 03-17 | `[STORE-04]` | STORE-04 |
| 03-16 | `[SCOPE-01, SCOPE-02]` | SCOPE-01、SCOPE-02 |
| 03-11 | `[SCOPE-03]` | SCOPE-03 |
| 03-14 | `[SCOPE-05]` | SCOPE-05 |

5份均添加`requirements-reconciled: "2026-11-04"`、英文frontmatter历史语义说明和中文dated证据段。Python结构检查5/5通过：相对HEAD的原`completed`行不变、日期字段/dated段各唯一、增补证据链接存在。`git diff --check`通过；diff仅新增归并段、增加日期/说明、替换授权的completed数组。未修改03-13或Phase6/7 SUMMARY，未更新VERIFICATION/fingerprint、未提交、未运行Cargo/Docker/服务。

一次ledger补丁因上下文只写了半段而未匹配，工具原子拒绝，随后使用完整行成功应用；无部分误写。

下一步由主会话统一交verifier核对文档diff并复签fingerprint，再由审计owner重算三源矩阵。本worker本轮不推进其他metadata范围。
