# 独立方案咨询：Phase 09 Webhook Worker Supervision & Crash Recovery

你是资深 Linux 服务工程 / Rust 测试架构师。请独立提出方案，不要受既有倾向影响。只输出方案与关键风险，不要改代码。

## 需求（验收标准，逐字）

Phase 09（REL-03）Success Criteria：

1. Given worker 在派发中途被强制终止, When 监督方观察到该退出, Then CI 日志出现可观测信号（退出码 / 监督状态 / 重启事件）
2. Given worker 崩溃时 outbox 仍有未送达事件, When worker 被重新拉起, Then 该事件从持久化 outbox 恢复并被送达, 无丢失
3. Given 仓库交付 systemd unit 文件, When 运维者在 Linux 主机安装该 unit, Then unit 随系统启动拉起 worker, 并在异常退出后自动重启
4. Given 本 phase 的机器可判定部分与人工部分, When 结论写入验收记录, Then 机器结果仅作证据, systemd 自动拉起的结论位保持 `VERDICT: PENDING` 直到人工签字, 执行者不代签

## 现状（已验证事实，文件:行号）

- `ontoforge webhook-worker --server-config <path>` 是既有子命令：`crates/ontoforge/src/lib.rs:37`；`webhook_worker.rs:13-25` 从 `[webhook]` 节读 `target / poll_interval_ms / request_timeout_ms / lease_duration_ms / batch_limit`。
- Outbox 租约：`crates/ontoforge-action/src/outbox.rs` — worker claim 事件时写 `worker_id + lease_until`；崩溃后事件带未过期租约，租约到期（测试配置 `lease_duration_ms = 5000`）后其他 worker 可重领。`record_retry` fencing 存在（旧 token 被抢占时写入被拒绝）。
- Phase 08 已有 Linux CI E2E 基建：`ci.yml:383` `phase8-webhook-e2e` job（ubuntu runner + postgres:17 service + `cargo build --example webhook_receiver`）；测试 `crates/ontoforge/tests/webhook_e2e_pg.rs` 用 `Command::new(env!("CARGO_BIN_EXE_ontoforge"))` spawn worker（`Process` guard kill+reap on drop），接收端是独立进程写 JSONL ledger（只记录不断言）；判定只从 ledger + `of_webhook_outbox` 终态取证；证据经 `ONTOFORGE_WEBHOOK_EVIDENCE_DIR` 打包成 artifact。
- GitHub ubuntu-latest runner 是完整 VM，`sudo systemctl` 可用（GitHub 自托管 runner 自身就以 systemd service 运行）。
- 仓库尚无 packaging/deploy 目录；unit 文件需新建。

## 约束

- 本机开发是 Windows 11，`cargo build/test` 必须过，不依赖 Docker；Linux 行为只在 CI 验证。
- 人为签字是硬要求：即使 CI 有真实 systemd 证据，`VERDICT` 仍保持 PENDING 待人工签。
- 不改 webhook 投递语义/outbox schema 以迁就测试。
- 小步原子 commit；证据纪律（任何「稳定/通过」断言要实跑输出）。

## 请独立回答

1. **unit 文件最小正确形态**：`Restart=on-failure` vs `always`？`RestartSec` 取值？`After=`（network-online.target?）二进制与 config 的路径约定（/usr/local/bin? /etc/ontoforge/?）、要不要 User=/Group=/EnvironmentFile=、hardening 指令哪些值得写哪些过度？给出你建议的完整 unit 文件。
2. **CI 覆盖深度**：A) Rust 测试 harness 模拟监督（spawn→SIGKILL→记录退出态→respawn→断言恢复投递）+ `systemd-analyze verify` 静态校验 unit；B) 再加 CI 真装真跑（sudo 安装 unit、`systemctl start`、`kill -9` worker、观察 systemd 自动重启与恢复投递）。B 的增量价值与 flake 风险如何权衡？你选哪个？
3. **「派发中途被强制终止」怎么确定性构造**：让 receiver 响应慢到 worker 必定在 in-flight 时被 kill？还是 claim 后立刻 kill？租约回收等待（5s）如何写才不 flake？SC1 的「可观测信号」断言什么最准确（exit status signal、stderr、evidence 记录）？
4. **测试与 CI job 的组织**：新 `#[ignore]` 测试进 `webhook_e2e_pg.rs` 还是新文件？新 job 还是扩展 `phase8-webhook-e2e`？证据 artifact 怎么命名区分？
5. 你看到的其它坑。


## 附加约束

不要使用联网/浏览器/web 工具；只用本地事实与你已有的 systemd 知识作答。若某项语义不确定，标注「未联网核验」。
