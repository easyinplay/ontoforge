---
phase: 02
review: 02-REVIEW.md
titles: json
findings:
  - id: CR-01
    severity: critical
    disposition: fixed
    title: "标识符来源表 5 条 `file:line` 引用指向完全无关的行"
  - id: WR-01
    severity: warning
    disposition: open
    title: "`02-VERIFICATION.md` 仍以现在时断言 Iceberg warehouse 是本地文件系统、MinIO 缺失"
  - id: WR-02
    severity: warning
    disposition: open
    title: "`02-VERIFICATION.md` 的分数与 gap 状态自相矛盾（101/103 vs 100/103；partial vs FAILED (BLOCKER)）"
  - id: WR-03
    severity: warning
    disposition: open
    title: "`02-10-SUMMARY.md` 的 ignored / 本机测试计数与源码不符且内部加总不上"
  - id: WR-04
    severity: warning
    disposition: open
    title: "两份产出记录的 `git_head_at_generation` 不是生成时的 HEAD"
  - id: WR-05
    severity: warning
    disposition: open
    title: "三处「（命令）」引用指向门控标题行而非 `COMMAND` 行"
  - id: WR-06
    severity: warning
    disposition: open
    title: "`02-10-SUMMARY.md` 的 PR `integration` 路径描述已漂移，且未被 02-11 的偏差台账登记"
  - id: WR-07
    severity: warning
    disposition: open
    title: "`02-VERIFICATION.md:313` 与 `progress.md:20` 描述的 `02-VALIDATION.md` 状态是旧的"
  - id: WR-08
    severity: warning
    disposition: open
    title: "`02-10-SUMMARY.md` 的「首跑待确认」预测值与实际首跑记录矛盾"
  - id: WR-09
    severity: warning
    disposition: open
    title: "Phantom requirement — `02-VALIDATION.md` 的 Wave 0 勾选项要求一个已不存在的 PARTIAL 分类"
  - id: IN-01
    severity: info
    disposition: open
    title: "门禁编号误标：「13 条 G-A1..G-A5」与未定义的「G-A2b」"
  - id: IN-02
    severity: info
    disposition: open
    title: "「plan 起始 HEAD」在同一 plan 的两份产出里取值不同"
  - id: IN-03
    severity: info
    disposition: open
    title: "`commits: 3` 的「实测」方法在文中给出的时点复现不出 3"
open: 13
total: 13
recorded: 2026-11-08T09:09:17.687Z
---

# Phase 02: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| CR-01 | critical | open | - |
| WR-01 | warning | open | - |
| WR-02 | warning | open | - |
| WR-03 | warning | open | - |
| WR-04 | warning | open | - |
| WR-05 | warning | open | - |
| WR-06 | warning | open | - |
| WR-07 | warning | open | - |
| WR-08 | warning | open | - |
| WR-09 | warning | open | - |
| IN-01 | info | open | - |
| IN-02 | info | open | - |
| IN-03 | info | open | - |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.

## Fixed Issues

| ID | Source | 说明 |
|---|---|---|
| CR-01 | 本次修正提交 | `02-11-source-scan.txt` 标识符来源表 5 条 `file:line` 引用修正：`ci/trino/l2/rules.json:19` → `:18`（`Orders` / `orders` 规则行）、`:20` → `:19`（`queries` / `system` 第二处）、`ci/pg/seed.sql:32` → `:172`（`cardinality_records`）；同文件说明段第 223 行同引用一并修正。词表成员、中性判定与 `## SUSPECTED NON-NEUTRAL` 为空的结论不受影响。
