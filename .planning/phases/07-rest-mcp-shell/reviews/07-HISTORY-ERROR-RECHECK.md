---
phase: 07-rest-mcp-shell
reviewed: 2026-11-03T07:57:41Z
depth: standard
review_base: 5e00281c81119929ed2657b5bb58ed5cc4713c5d
review_target: uncommitted-working-tree
scope_contract: RF-12
source_blobs:
  crates/ontoforge-server/src/rest.rs: 18b527ffbb719d169db5258df0e7d78ba082d37b
  crates/ontoforge-server/tests/history_errors.rs: d86bb4a2fcc283130931afba91f531691cd30a79
files_reviewed: 7
files_reviewed_list:
  - crates/ontoforge-server/src/rest.rs
  - crates/ontoforge-server/tests/history_errors.rs
  - crates/ontoforge-server/tests/support/mod.rs
  - crates/ontoforge-server/src/error.rs
  - crates/ontoforge-server/src/context.rs
  - crates/ontoforge-query/src/lib.rs
  - crates/ontoforge-storage/src/history.rs
findings:
  critical: 0
  warning: 0
  info: 0
  total: 0
status: clean
original_finding: Standards-CR-05
original_finding_status: resolved_for_RF-12
phase_acceptance: pending
---

# Phase 7 History Error Recheck

RF-12 定向复签为 clean，无未解决 BLOCKER 或 WARNING。检查范围为冻结 blob 中 rest.rs 的 history diff、新测试全文，以及 query_failure、审计、history storage 错误与测试 fixture 的必要调用链；依赖文件仅读取相关路径，不宣称全文件或整个 Phase 审查完成。

依据为 07-REVIEW-FIX-CONTRACT.md 的 RF-12、07-03-PLAN.md:62-69 既有审计/HTTP 映射合同、docs/api.md:46-48 的请求 ID 合同。未修改实现、DTO/MCP 或共享 planning，未运行 Cargo，只写此报告。RF-07 分页预算和其他 lane 不属本轮范围。

## Narrative Findings (AI reviewer)

无新增未解决发现。原 Standards CR-05 的 RF-12 部分已有下列源码与行为证据。

## Static Evidence

| 边界 | 当前实现与断言 |
| --- | --- |
| 审计 ID | rest.rs:593 从 QueryExecution.request_id 取已预留 ID，仅无 reservation 时回退 transport ID。之后成功、空对象、授权不可见、PK 截断、query/history 故障均使用此 ID。已有 observer 同步更新 RequestTrace，保留宿主 deadline 关联路径。 |
| PK 截断与不存在 | :595-605 对 truncated 先返回 400 COST_EXCEEDED；正常空结果或 QueryError::NotFound 保留 404。测试 :161 以 1 byte 限额触发截断，:58 单独请求真正不存在的 PK。 |
| query 故障 | :605 复用 query_failure。QueryError::Source/Store/AuditUnavailable 为 503；ScopedEngine 记录明确 Timeout/Cancelled 时分别返回 504 DEADLINE_EXCEEDED、408 CANCELLED。没有根据 Display 文本推断类别。 |
| history storage 故障 | :644-652 穷举 HistoryError::NotFound→404、Store(_)→503 STORE_UNAVAILABLE；StoreError 内容不进入响应。读取继续使用请求冻结 definition、scope 和当前可见行，未扩大授权。 |
| 脱敏和持久证据 | history_errors.rs:19-43 对完整 code/apiName/request_id envelope 做相等断言，含 source secret sentinel；:97/:120 实际删除 SQLite 对象表/历史表分别触发两层 store 故障。support/mod.rs:271 后在宿主 shutdown 完成后查询真实 of_read_audit，并检查业务 commit 数未变。 |
| 审计准入失败 | 测试 :143 关闭 audit worker 后请求 source-backed 对象，要求 503 AUDIT_UNAVAILABLE、source calls=0、无伪造持久审计记录。 |

## Audit Semantics And Explicit Limits

history_log_store_failure 的 HTTP 结果为 503，但关联审计仍为 **tool=query.pk、status=success、row_count=1**：QueryService 在 query/src/lib.rs:287-299 完成当前可见对象查询及其审计后，才进入 history_with_snapshot。该审计证明前置 PK 查询成功，不证明整个 history 端点成功；后续 history store 故障没有独立的最终请求审计状态。

此边界与 RF-12 的“沿用既有模式、不扩 storage 接口”及 07-03-PLAN.md:62-69 的“单次 reserve、保留原 finish/status”一致，故不作为本轮新增缺陷。PK 截断的 HTTP 400 同样保留底层 success/0，是该计划明确接受的语义。若后续需要整个 history 请求的最终结果审计，应独立变更审计合同，不能把本次 ID 对账当作已具备该能力。

## Dynamic Evidence

以下日志已由本 reviewer 独立读取；命令执行归属 owner，本 reviewer 未复跑：

| 日志 | 核读结果 |
| --- | --- |
| target/phase7-history-errors-red.log | 0 passed、7 failed；实际断言暴露错误 404、成功/不存在 ID 不一致及截断误报404，非编译 RED |
| target/phase7-history-errors-green.log | 7 passed、0 failed，0.95s；同名用例均执行 |
| target/phase7-history-errors-adjacent.log | http_actions 3/3，http_contract 3/3；含历史遮蔽与既有 HTTP 限额/错误回归 |
| target/phase7-history-errors-clippy.log | Finished dev profile，24.84s，无诊断错误；退出状态由 owner 核验 |
| target/phase7-history-errors-fmt.log | 空输出；单凭空文件不能独立证明退出码，fmt 成功依 owner 报告 |

这些证据使用真实 HTTP listener、SQLite store、可控 Engine，并未证明 PostgreSQL/Trino 全链路或全 workspace 已在当前工作区通过。报告的 clean 只关闭 RF-12 定向源码发现。
