---
phase: 07-rest-mcp-shell
reviewed: 2026-11-03T08:23:15Z
depth: standard
review_base: ff9751382f8906550fbba700d1719295dfe39683
review_target: frozen-history-helper-and-function
scope_contract: RF-07-REST-helper
source_blobs:
  crates/ontoforge-server/src/history_page.rs: 67a00a5f76433ea694c5f5b6928e3ab23d67b18c
  crates/ontoforge-server/src/lib.rs: 302202e05be14523af3d98ae2eab7b21cef1f9a3
  crates/ontoforge-server/src/rest.rs: 6c2c127b6f93fd0f8cf24798770453ea991b96fb
  crates/ontoforge-server/tests/history_pagination.rs: 5e5424de493171bb33e6519c3763aeb55df7df5e
history_function_sha256: 09C267C123106E94C60BB99F095772EE82AC3918B14C2363C4A293348FB7F006
files_reviewed: 4
files_reviewed_list:
  - crates/ontoforge-server/src/history_page.rs
  - crates/ontoforge-server/src/lib.rs
  - crates/ontoforge-server/src/rest.rs
  - crates/ontoforge-server/tests/history_pagination.rs
findings:
  critical: 0
  warning: 0
  info: 0
  total: 0
status: clean
remaining_scope: RF-07-MCP-integration
phase_acceptance: pending
---

# Phase 7 History Pagination Recheck

RF-07 的 REST/helper slice 复签为 clean，无未解决 BLOCKER 或 WARNING。MCP 尚未接入，不据此关闭 RF-07 整体。

读取新 helper/test 全文、lib.rs 单行模块注册、rest.rs 的 history hunk，并追踪原 history_entry、query_failure、storage 排序/完整记录扩展和固定 snapshot 调用链。rest.rs 的 source blob 仅记录读入时版本；并发 RF-03 action_response 不在本轮范围。上列函数 SHA256 对 LF 规范化文本从 `pub async fn history<` 到 `pub(crate) fn history_entry` 前、TrimEnd 后的 UTF-8 计算，作为 history 范围的稳定依据。

仅新增报告，未运行 Cargo、修改实现或共享 planning，未提交。

## Narrative Findings (AI reviewer)

无新增未解决发现。

## Static Evidence

| 合同 | 核实结果 |
| --- | --- |
| 完整 seq 组 | history_page.rs:43-47 遇同 seq 后续条目时不尝试截页；只有组尾通过预算才更新 retained。storage 按 log_seq/ordinal 降序返回，并扩展所选末组的剩余 edits，因此组连续且不拆分。 |
| 条数与真实继续 | :12 先 clamp(1,500)；末组可以跨越软条数上限。:17-25 以已读末组 seq、limit=1 向更旧记录 lookahead；最后一页 false/null，非终页游标为最后实际保留组的 seq。 |
| UTF-8 精确预算 | :35-62 累加规范化 HistoryEntry JSON 字节长度，加 index 个逗号（n 条为 n−1），再加相同 truncated/cursor 的空 entries 页 envelope。保留了 entries 方括号、字段、布尔值和游标开销；checked_add 拒绝溢出。 |
| 不跳组 | 后续组超预算时返回此前完整组，cursor 停在该已返回组，未返回组仍位于下一次 beforeSeq 范围。首个完整组超预算直接 CostExceeded，不返回部分组或跳去较旧组；空页亦检查 envelope 本身预算。 |
| 固定上下文与错误 | 主读取和 lookahead 均复用同一 HistoryRequest 的 scope/current_row 与同一 ActiveDefinition，仅修改 limit/before_seq。两处失败都映射 NotFound/Store，不能把 lookup 故障当作没有后续记录。REST 继续使用 RF-12 的 id/query_failure，CostExceeded 为安全错误。 |

固定 snapshot 指 definition/scope，不代表两次历史数据查询持有同一数据库 MVCC 事务；本 slice 未新增此保证。历史审计仍关联前置 query.pk，保留 RF-12 已说明的状态语义。

## Dynamic Evidence

独立核读 owner 的 target/phase7-history-pagination-* 日志与相应断言，未复跑：

| 日志 | 结果 |
| --- | --- |
| red.log | 0 passed、7 failed；逐项出现实际分页/预算断言失败，非编译失败。巨大的条目对照按对应失败段与摘要核读，未逐字审阅全部重复 payload。 |
| green.log | 7/7，0 failed，7.27s |
| regression.log | history_errors 7、history_pagination 7、http_actions 3、http_contract 3，总计 20/20 |
| final.log | history_errors 7/7 与 history_pagination 7/7，总计 14/14；分别 0.96s、6.90s |
| clippy.log / fmt.log | Clippy 有 Finished dev profile（1.00s），fmt 为空；exit0 来自 owner 报告，日志未保存独立 EXIT 标记 |

测试通过真实 HTTP listener 与 SQLite commit/history 建组：覆盖 limit=0、limit=1 但同组两 edits、501 个独立组在 limit=1000 下按 500+1 续取、终页 false/null、中文 UTF-8 页恰等预算与少 1 byte、三组按 bytes 连续取回、首组过大安全报错。每次响应的 ID 与 shutdown 后 durable audit 对账，业务 commit 数未增加。

未单独动态注入“首读成功而仅 lookahead 失败”的时序；该路径由两处相同错误传播的静态核查支持。没有声称这些 SQLite 定向用例证明 PostgreSQL/Trino、MCP 接入或全 Phase 验收。
