---
status: testing
phase: 09-webhook-worker-supervision
source: [09-01-PLAN.md]
started: 2026-11-10T02:49:42Z
updated: 2026-11-10T02:49:42Z
---

## Current Test

[testing]
expected: |
  CI job `phase9-webhook-supervision` 在 PR 上跑出首个实跑结果后回填：harness 用例与
  systemd 用例的机器可判定证据（supervisor.jsonl、outbox 快照、journal、systemctl show）
  按 commit 绑定回填进测试 1-3；机器结果仅作证据，不代签。
awaiting: CI run on commit df55c32 (or later on this branch)

## Tests

### 1. SC1：worker 派发中途被强制终止 → CI 日志出现可观测信号
expected: |
  `killed_worker_recovers_undelivered_event_after_respawn`：`ExitStatusExt::signal() == Some(9)`
  为主断言，`supervisor.jsonl` 出现含 case/old_pid/observed_signal/new_pid 的一行。
  `systemd_restarts_killed_worker_and_redelivers`：`kill -9 $MainPID` 后 `systemctl show`
  证据（systemctl-show-after-restart.txt）显示 `NRestarts` 递增且 `InvocationID` 变化，
  `journal.txt` 留存该 unit 的崩溃/重启记录。
evidence: |
  待回填：CI run URL + phase9-webhook-supervision artifact 中 harness/supervisor.jsonl、
  systemd/systemctl-show-*.txt、systemd/journal.txt。
result: pending

### 2. SC2：崩溃时 outbox 有未送达事件 → 重新拉起后从持久化 outbox 恢复并送达，无丢失
expected: |
  两个用例均断言：kill 后 `outbox-after-kill.json`（harness）显示 `delivered_at IS NULL`
  且 `lease_owner` 非空、`lease_until > now`（fixture 未构成目标场景即 fail 并说明）；
  重新拉起后 ledger 出现 `attempt_seq >= 2` 的 2xx 记录，`outbox-final.json` 显示
  `delivered_at` 非空。不断言恰好一次（屏障后重投的重复记录合法）。
evidence: |
  待回填：CI artifact 中 harness/outbox-after-kill.json、outbox-final.json、ledger.jsonl；
  systemd/outbox-final.json、ledger.jsonl。
result: pending

### 3. SC3 机器可判定部分：安装仓库 unit → 异常退出由 systemd 自动重启
expected: |
  `systemd_restarts_killed_worker_and_redelivers` 在 ubuntu-latest（systemd as PID 1）上：
  真实 install unit/binary/配置 → `systemd-analyze verify` 通过 → `enable --now` →
  屏障持有的首投出现后 `kill -9 MainPID` → 不再发任何 start/restart → 60s 内
  `NRestarts` 递增且 `InvocationID` 变化（systemctl-show-after-restart.txt）→
  事件重送达。测试自身清理：stop/disable、删 unit/config/binary、userdel。
evidence: |
  待回填：CI artifact 中 systemd/systemctl-cat.txt、systemd-analyze-verify.txt、
  systemctl-show-initial/after-restart/final.txt、journal.txt、supervisor.jsonl。
result: pending

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
passed: 0
issues: 0
pending: 4
skipped: 0
blocked: 0

## Gaps

- 测试 1-3 的机器可判定结论依赖 CI job `phase9-webhook-supervision` 的首个实跑结果；
  本地 Windows 仅完成编译级验证（`cargo build --workspace --locked`、`cargo test --test
  webhook_supervision_pg --locked` 编译通过（unix cfg 下 0 用例）、`cargo clippy
  --workspace --all-targets --locked -- -D warnings`、`cargo fmt --all --check` 全绿）。
- 测试 4 为人工验收：真实重启拉起（SC3 的 boot 拉起面）与最终结论位均保持
  `VERDICT: PENDING` 直至人工签字。
