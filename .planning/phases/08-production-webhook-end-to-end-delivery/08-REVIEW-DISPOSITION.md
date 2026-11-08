---
phase: 08
review: 08-REVIEW.md
titles: json
findings:
  - id: WR-01
    severity: warning
    disposition: open
    title: "陈旧的 `receiver.ready` 会让本次用例指向一个已死的回环端口"
  - id: WR-02
    severity: warning
    disposition: open
    title: "ledger 是追加写的，用例从不清空，第二次运行必然在去重集合断言上失败"
  - id: WR-03
    severity: warning
    disposition: open
    title: "接收端在互斥锁之外写 ledger，文件顺序可以与 `seq` 顺序脱钩"
  - id: WR-04
    severity: warning
    disposition: open
    title: "Impossible-state guard — 「接收端是独立进程」的 PID 断言无法失败"
  - id: IN-01
    severity: info
    disposition: open
    title: "`read_ledger` 与 `ledger_lines` 是两份几乎相同的读文件逻辑，语义却不同"
  - id: IN-02
    severity: info
    disposition: open
    title: "`ci/verify-evidence-archive.sh` 的错误归因不精确"
  - id: IN-03
    severity: info
    disposition: open
    title: "`craft_symlink_tar` 写的 ustar magic 不是 POSIX 形式"
  - id: IN-04
    severity: info
    disposition: open
    title: "新的 ignored 用例目标没有登记进共享的「ignored 用例确实存在」清单"
  - id: IN-05
    severity: info
    disposition: open
    title: "ready 文件的临时名用 `with_extension(\"tmp\")`，同目录下同 stem 的两个文件会互相覆盖"
  - id: IN-06
    severity: info
    disposition: open
    title: "用例在 PostgreSQL 里建的两个 schema 用完不删"
open: 10
total: 10
recorded: 2026-11-08T20:37:39.240Z
---

# Phase 08: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| WR-01 | warning | open | - |
| WR-02 | warning | open | - |
| WR-03 | warning | open | - |
| WR-04 | warning | open | - |
| IN-01 | info | open | - |
| IN-02 | info | open | - |
| IN-03 | info | open | - |
| IN-04 | info | open | - |
| IN-05 | info | open | - |
| IN-06 | info | open | - |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
