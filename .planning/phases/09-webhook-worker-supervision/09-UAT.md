---
status: testing
phase: 09-webhook-worker-supervision
source: [09-01-PLAN.md]
started: 2026-11-10T02:49:42Z
updated: 2026-11-10T03:45:00Z
---

## Current Test

number: 4
name: 真实重启拉起与结论位的人工签署（VERDICT: PENDING）

## Tests

### 1. SC1：worker 派发中途被强制终止 → CI 日志出现可观测信号
expected: |
  `killed_worker_recovers_undelivered_event_after_respawn`：`ExitStatusExt::signal() == Some(9)`
  为主断言，`supervisor.jsonl` 出现含 case/old_pid/observed_signal/new_pid 的一行。
  `systemd_restarts_killed_worker_and_redelivers`：`kill -9 $MainPID` 后 `systemctl show`
  证据显示 `NRestarts` 递增且 `InvocationID` 变化，`journal.txt` 留存该 unit 的崩溃/重启记录。
evidence: |
  CI run 42749316959（commit 8c5205d），job phase9-webhook-supervision（id 129608910327）
  → 2 passed / 0 failed / 15.01s；artifact `phase9-webhook-supervision`（ID 13733185293）。
  harness/supervisor.jsonl：
  `{"case":"sigkill_mid_dispatch","supervisor":"test-process","event_id":"01M71R20TZ0WQRF2DBP5W1Y6WM","old_pid":10887,"observed_signal":9,"new_pid":10892}`
  systemd/supervisor.jsonl：
  `{"case":"sigkill_mid_dispatch","supervisor":"systemd","unit":"ontoforge-webhook-worker.service","event_id":"01M71R268YGRR26W7N6R4173QQ","old_pid":11135,"observed_signal":9,"initial_invocation_id":"bd7adde6abb14ab69cc9c0bfa9d3a56f","restarted_invocation_id":"eaa1396e87e943f9a5c29e9423855d48","nrestarts":"1","new_pid":11183}`
result: pass

### 2. SC2：崩溃时 outbox 有未送达事件 → 重新拉起后从持久化 outbox 恢复并送达，无丢失
expected: |
  两个用例均断言：kill 后 outbox 行 `delivered_at IS NULL` 且租约在持
  （fixture 未构成目标场景即 fail 并说明）；重新拉起后 ledger 出现
  `attempt_seq >= 2` 的 2xx 记录，outbox 终态 `delivered_at` 非空。
  不断言恰好一次（重复投递合法）。
evidence: |
  同一 run/artifact。systemd/ledger.jsonl：同一 `event_id`
  `01M71R268YGRR26W7N6R4173QQ` 两条记录——`attempt_seq:1 status 204`
  （03:29:42.862，屏障持有的在途请求）与 `attempt_seq:2 status 204`
  （03:29:48.186，systemd 重启后重投）；systemd/outbox-final.json：
  `delivered_at: 2026-11-10T03:29:48.187Z`、`attempt_count: 0`、`lease_owner: null`
  （终态已完成、租约释放）。harness 侧同构证据在 harness/ 目录。
result: pass

### 3. SC3 机器可判定部分：安装仓库 unit → 异常退出由 systemd 自动重启
expected: |
  在 ubuntu-latest（systemd as PID 1）上：真实 install unit/binary/配置 →
  `systemd-analyze verify` → `enable --now` → 屏障持有的首投出现后 `kill -9 MainPID`
  → 不再发任何 start/restart → 60s 内 `NRestarts` 递增且 `InvocationID` 变化 →
  事件重送达。测试自身清理：stop/disable、删 unit/config/binary、userdel。
evidence: |
  同一 run/artifact：`NRestarts: "1"`、`initial_invocation_id bd7adde…` →
  `restarted_invocation_id eaa1396e…`、new_pid 11183；systemd/journal.txt、
  systemctl cat/show 输出同包。`enable`/`is-enabled` 仅证明启动关系已安装，
  真实重启主机拉起属测试 4 人工验收，不在本测试范围。
result: pass

### 4. SC4 + SC3 人工部分：真实重启拉起与结论位的人工签署（VERDICT: PENDING）
expected: |
  人工验收测试（签字绑定 commit / 主机 / systemd 版本 / 证据）：
  1) 环境：持久化 PostgreSQL + 可达 receiver（可为 `webhook_receiver` example 或真实端点）、
     一台 systemd as PID 1 的 Linux 主机；记录 `systemctl --version` 与主机标识。
  2) 按 deploy/systemd/INSTALL.md 安装：ontoforge 系统账户、二进制、
     ontoforge-server.toml、[store].url_file 凭据文件、unit → verify → enable --now。
  3) `systemctl reboot`（或宿主机层面硬重启）→ 新 boot 后确认
     `systemctl is-active ontoforge-webhook-worker.service` 为 active 且产生新
     InvocationID（随系统启动拉起）。
  4) `sudo kill -9 $(systemctl show -p MainPID --value ontoforge-webhook-worker.service)` →
     不手动干预 → `NRestarts` 递增、新 InvocationID，outbox 事件继续投递（异常退出自动恢复）。
  5) 对照本 phase 的 CI artifact 复核测试 1-3 的证据链。
  结论位：VERDICT: PENDING——执行者与 verifier 均不代签；签字时须写明 commit、
  主机 / systemd 版本与证据指针。
result: pending

## Summary

total: 4
passed: 3
issues: 0
pending: 1
skipped: 0
blocked: 0

## Gaps

- 测试 1-3 的机器证据绑定 CI run 42749316959（commit 8c5205d）；后续 commit
  81ffbdf 仅修 clippy lint（assert!/too_many_lines allow），不改变被测语义，
  相同 job 将在该 commit 的 run 中复跑。
- 测试 4 为人工验收：真实重启拉起（SC3 的 boot 拉起面）与最终结论位均保持
  `VERDICT: PENDING` 直至人工签字。
