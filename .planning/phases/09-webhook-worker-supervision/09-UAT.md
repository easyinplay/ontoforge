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
evidence: |
  真机执行（2026-11-10，宿主 IdeaCentreGeekPro-17IRR / WSL2 Ubuntu-26.04 /
  kernel 6.18.33.2-microsoft-standard-WSL2 / systemd 259 (259.5-0ubuntu3.4)，
  受验收 commit = main `05785bd`；SQLite store 变体——生产语义同 PG 路径，
  差异已在下注明）：
  1) 安装按 INSTALL.md 逐条执行：ontoforge 系统账户（nologin）、
     /usr/local/bin/ontoforge (root:root 0755)、/etc/ontoforge/ontoforge-server.toml
     (root:ontoforge 0640，[store] sqlite path=/var/lib/ontoforge/edit.db)、
     仓库原始 unit → `systemd-analyze verify` 通过 → `enable --now` active。
  2) 首事件（种入 of_webhook_outbox）经 worker 投递：receiver ledger
     `WSLUAT-49CB41FBE045458A` 204，`delivered_at` 落库 —— 投递链路真机端到端通。
  3) `wsl --shutdown` 冷启动：boot_id `ec44145e…` → `86d2384e-abcf-4db3-8f03-ad5746733a3e`
     （真重启）；新 boot 中 worker `ActiveState=active` + 新 InvocationID
     `b1538f4c271447acb58e3a2d9c810f19`（MainPID=130，零人工干预）；
     journal `systemd[1]: Started ontoforge-webhook-worker.service` ——
     随系统启动拉起已证。
  4) `kill -9 MainPID=126`（InvocationID `b46a4e79…`）→ 观察期捕获
     `SubState=auto-restart`（监督可观测信号）→ 8s 内 `NRestarts=1`、
     新 InvocationID `2734a7d6b3e84970ab350e7379581499`、`MainPID=539`；
     重启后种入的 `WSLUAT-95804E8CDD214E42` 投递成功（ledger 204 + delivered_at）
     —— 异常退出自动恢复 + outbox 事件无丢失已证。
  5) 验收运行中还观察到一次自发崩溃-自愈（seed 期 SQLite 锁竞争 → worker 崩 →
     systemd 自动重启恢复），佐证 Restart=on-failure 行为真实生效。
  证据文件：target/phase9-wsl-uat/（kill-restart.json、journal.txt、ledger.jsonl、
  systemctl-show-*、systemd-analyze-verify.txt、host-info.txt）。
  注：本验收使用 SQLite store 而非持久化 PostgreSQL——unit/重启/投递语义相同；
  PG 变体已由 CI phase9-webhook-supervision 两次实跑覆盖（artifact 13733185293）。
  测试后 unit 已 disable（不留自启），安装文件保留可复核。
result: pending — 等待人工签字（执行证据已备，签字人复核后落 VERDICT）

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
