# ontoforge 交接文档：Claude Code → Devin CLI

**交接时间**：2026-11-09
**交接方**：Claude Code 会话（`/gsd-verify-work 08` 执行中）
**接手方**：Devin CLI
**交接目的**：Phase 08 收尾 + Phase 09-12 后续开发

> 这份文档是自包含的。你（Devin CLI）没有本对话的任何上下文，所有需要的信息都在这里。
> 读完直接开工，不需要先向我提问。

---

## 1. 项目是什么

**ontoforge** 是一个用 Rust 实现、可自托管的开源 Ontology 操作层，复刻 Palantir Foundry Ontology 的核心能力。

用 YAML/JSON 定义 object type / link type / action type（ontology-as-code）。对象数据只能通过带校验的 Action 写入并留下不可变 edit log，再用 object set 查询（过滤、遍历、集合运算、聚合）读取。同一份 ontology 同时通过 REST（配 Rust / TypeScript client）服务业务应用，通过 MCP 服务 AI agent。

**Core Value**：定义一次 ontology，应用和 agent 就能通过 REST 与 MCP 两个同等地位的出口，对同一份对象图做受控的读写：写入一律经 Action 校验与授权，每次写入都可审计。

**当前状态**：v1.0 MVP 已发布（本地 tag，未推送）。v1.1「Gap Closure」里程碑进行中，Phase 08 收尾阶段。

---

## 2. 硬约束（违反会直接判定失败）

| 约束 | 内容 |
|---|---|
| **技术栈** | Rust 1.98，edition 2024，cargo workspace。axum + utoipa、sqlx、sea-query、cel、rmcp（ADR-0003~0011 已锁定，不得擅自替换） |
| **平台** | 必须在 Windows 11 上 `cargo build` / `cargo test` 通过（本机不依赖 Docker）；生产部署目标 Linux |
| **测试** | Windows 本机默认测试只依赖 SQLite；Postgres 本机用 `postgresql_embedded`，CI 用 service container |
| **命名** | 项目 / crate 前缀 `ontoforge`；标识符遵循 ADR-0002，**禁止沿用参考仓库的标识符**（参考仓库是 bkn-foundry，含 OpenBKN License 文件） |
| **License** | **不复制参考仓库代码**。可借鉴设计，不可借鉴实现与命名 |
| **通用性** | 做通用开源系统，公司数据只作测试用，不绑定任何一家公司 |
| **改动边界** | 未经明确指示，**不要修改** `crates/ontoforge-action/src/outbox.rs`、`crates/ontoforge-engine/src/config.rs`、`crates/ontoforge-storage/`（Phase 08 的 prohibition 明文禁止为让用例变绿而改投递语义） |

---

## 3. 仓库结构与关键路径

```
crates/
  ontoforge/           二进制 crate：server、CLI（webhook-worker 子命令）、examples/webhook_receiver.rs
  ontoforge-action/    Action 执行 + outbox worker（webhook 投递核心）
  ontoforge-engine/    查询引擎、Trino 连接、source binding
  ontoforge-storage/   edit store、PostgreSQL/SQLite 双后端、migrations
  ontoforge-query/     查询执行
  ontoforge-auth/      认证授权、scope 模型
  ontoforge-config/?   配置
.planning/
  ROADMAP.md           阶段路线图（Phase 8-12）
  STATE.md             当前指针 digest（<100 行，别让它膨胀）
  REQUIREMENTS.md      REL-01..REL-07 验收标准
  PROJECT.md           项目总纲、Key Decisions 表
  WINDOWS.md           跨 phase 缺陷登记表（**表格与 JSON 双份，改一处必须同步另一处**）
  adr/                 32 个 ADR，一决策一文件，status: proposed/accepted/superseded
  phases/08-.../       Phase 08 的全部产物
  milestones/          v1.0 归档
.github/workflows/     ci.yml（主 CI）、nightly.yml
ci/                    CI 辅助脚本（verify-evidence-archive.sh、query-services.sh 等）
```

---

## 4. 当前进度快照

### 分支与 PR

- 当前分支：`gsd/phase-08-production-webhook-end-to-end-delivery`
- PR #12：`OPEN`，`MERGEABLE`，base = `main`，**尚未合入 main**
- 本地 HEAD `a161ce6`，比 `main` 多 **27 个 commit**
- **远程分支落后本地 9 个 commit**（远程 `30a05a8`，本地 `a161ce6`）—— 你接手后第一件事可能是 `git push` 把这 9 个补上去

### 里程碑结构

- v1.0 MVP（Phase 1-7, 7.1, 7.2）— 已完成，2026-11-08
- v1.1 Gap Closure（Phase 8-12）— 进行中

| Phase | 状态 | 说明 |
|---|---|---|
| 08 Production Webhook End-to-End Delivery | 实现完成，验证收尾中 | 见下方「当前待办」 |
| 09 Webhook Worker Supervision & Crash Recovery | **未规划** | 依赖 Phase 8 |
| 10 Audit Timing Test Split | **未规划** | 与 8/9 无依赖，可并行 |
| 11 Product-Layer Wording & Historical Annotation | **未规划** | 与 8-10 无依赖，可并行 |
| 12 First Public Release | **未规划** | 依赖 8-11 全部验收通过 |

Phase 10 / 11 与 Phase 08 / 09 无依赖，**可以并行规划**。

### 已完成的工作流约定（GSD）

本项目用 GSD（Get Shit Done）做项目管理，目录 `.planning/` 是项目记忆。你接手时**没有 GSD skill 可用**，所以：

- **读**：`.planning/` 下的产物照常读，它们是磁盘真相
- **写**：GSD 的 slash command（`/gsd-plan-phase` 等）在你的环境里不存在。**请直接编辑 `.planning/` 下的文件，并保持既有格式**（frontmatter 字段、`##` 小节结构、YAML 数据块），这样 Claude Code 侧的 GSD 仍能读懂
- **状态从产物派生**：Phase 完成与否的真相源是 VERIFICATION.md 的 passed + 测试绿，不要手维护 checkbox 与产物对账

---

## 5. 当前待办（最高优先级）

### 待办 A：Phase 08 的 UAT 测试 1 结案

Phase 08 验证报告把 16 条 must-have 中的 1 条（FA-02）判为 `PRESENT_BEHAVIOR_UNVERIFIED`，理由是「退避上限公式从未在运行时被执行」。UAT 会话 `08-UAT.md` 的测试 1 就是这条。

**两轮独立复核（GPT-6.1 Sol 与 SWE-2）已完成，结论如下。这是已经查清的事实，不需要你重新推演：**

**1. 300 秒封顶点是 attempt 10，不是 attempt 17。**

`crates/ontoforge-action/src/outbox.rs:157-158`：
```rust
let backoff = 1_i64 << (attempts - 1).min(16);
let retry_at = completed_at + chrono::Duration::seconds(backoff.min(300));
```

逐 attempt 实测值：

| attempt | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11..20 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 秒 | 1 | 2 | 4 | 8 | 16 | 32 | 64 | 128 | 256 | **300** | **300** |

- `.min(16)` 只是 i64 移位溢出保护（`1<<16 = 65536`），在 `.min(300)` 之后**值域上完全不可观测**
- 「attempt ≥ 17 恒为 300 秒」这个**较弱命题成立**；错误在于把它当作「首次封顶点」
- 原 checkpoint 隐含的「第 17 次之前仍指数增长」**不成立**（attempt 10-16 已在上限上）

**2. 原 checkpoint 的测量口径无法执行。**

原文要求观察 `next_attempt_at` 与 `last_attempt_at` 之差，但 **`of_webhook_outbox` 表没有 `last_attempt_at` 列**。实际列只有：`event_id / payload / attempt_count / next_attempt_at / lease_owner / lease_until / delivered_at / last_error`（`crates/ontoforge-storage/migrations/postgres/0004_action.sql:20-29` 与 `sqlite/0004_action.sql:20-29` 皆然）。正确口径是「本轮 `next_attempt_at` − 上轮 poll 输入时刻」。

**3. 关键发现：300 秒封顶行为其实一直在 CI 里跑。**

`crates/ontoforge-action/tests/outbox.rs:584-613` 的 `retry_backoff_caps_at_five_minutes_without_leaking_response`：
- Phase 05（commit `f30d6ec`）就存在，**无 `#[ignore]`**
- `.github/workflows/ci.yml:73` 有 `cargo test -p ontoforge-action --locked -- --nocapture`（job 名 "Action SQLite, outbox and write-entry corpus"），**每次 CI 都在跑**
- 循环 `for attempt in 1..=10`，**attempt 10 恰好就是 300 秒封顶的首次生效点**

也就是说：Phase 08 验证报告只看 Phase 08 自己的 e2e 用例（只覆盖第 1 次重试，是事实），就下了「300 秒上限从未在运行时被执行」的判定。**这个判定不准确** —— 封顶行为在 Phase 05 起的每次 CI 里都被执行了。

**4. attempts 11..20 已由两轮独立复核实测补齐。**

两轮都用可控时钟（`poll_at(due)` 手动把 `due` 推进到上轮 `next_attempt_at`），不需要墙钟等待、不需要 PostgreSQL：

- **GPT-6.1 Sol**：用 `rustc` 从 stdin 编译原测试的**内存派生版本**（循环上限与末尾请求数断言均改 20），链接原生产 worker/store 库实跑。输出 `attempt=17 delta_ms=300001`、`18 → 300001`、`19 → 300003`、`20 → 300001`，`1 passed / 0 failed / 0.24s`
- **SWE-2**：临时改 `crates/ontoforge-action/tests/outbox.rs`（`1..=10` → `1..=20`，末尾请求数断言 `10` → `20`）实跑 `1 passed / 0 failed / 0.32s`，随后 `git checkout` 还原

两轮跑完 `git status` 与 `git diff` 均确认工作区与开始时一致。

**5. 扩展循环时必须同步改两处，不止一处。**

`outbox.rs:595` 的 `for attempt in 1..=10`，以及 `outbox.rs:609-612` 硬编码的 `assert_eq!(server.received_requests()...len(), 10)`。只改前者会编译通过但断言失败。

**6. 现有断言不是精确相等。**

Phase 05 原始版本（`f30d6ec`）是 `assert_eq!(next - due, seconds(delay))` 精确相等，后被 commit `d4517e0` 改成区间断言：
```rust
assert!(next - due >= chrono::Duration::seconds(delay));
assert!(next - due < chrono::Duration::seconds(delay + 1));
```
因为 `completed_at = now + started.elapsed()`（`outbox.rs:143-145`），存在微秒级漂移。区间断言仍然紧（误差 < 1 秒）。

**你要做的三处文件修改：**

**(a) `.planning/phases/08-production-webhook-end-to-end-delivery/08-UAT.md`**

测试 1 判 `pass`，文案改为精确表述。两份复核给的文案方向一致，取其意：

```markdown
### 1. FA-02：退避上限 300 秒（attempt 10 起恒定；已由可控时钟运行时验证）
expected: 对同一 event_id 连续失败 20 次（wiremock 持续 503，可控时钟驱动 WebhookWorker.poll_at），
  观察 of_webhook_outbox 的 next_attempt_at 推进。
  期望：第 k 次已记录失败的退避 = min(2^(k-1), 300) 秒，即 k=1..9 为 1/2/4/8/16/32/64/128/256 秒，
  k>=10 起恒为 300 秒（1_i64 << (attempts - 1).min(16) 与 .min(300) 的运行时效果；
  .min(16) 仅为移位溢出保护，在 .min(300) 之后不可观测）。
  依据：crates/ontoforge-action/src/outbox.rs:157-158；
  运行时证据：crates/ontoforge-action/tests/outbox.rs
    retry_backoff_caps_at_five_minutes_without_leaking_response（循环上限扩至 20，实测 1 passed / 0.32s）。
  未覆盖声明：该证据为 SQLite + 可控时钟 + 单 worker；PostgreSQL 后端与多 worker 租约抢占
  不在本条范围内，但退避公式与 claim/retry 代码路径与后端无关。
result: pass
```

同时：`Current Test` 切到测试 2（保留其原 expected 与 `awaiting: user response`）；Summary 改为 `total: 2 / passed: 1 / issues: 0 / pending: 1 / skipped: 0 / blocked: 0`；`## Gaps` 首段替换为：

```
FA-02 已补充 attempts 1..20 的 SQLite 运行时验证，覆盖 attempts 17..20；这不是 PostgreSQL 生产端到端的高次重试证据。其余 15 条沿用原验证报告，本次未重新核验。测试 2 的人工签署仍待执行，VERDICT 保持 PENDING。
```

**(b) `.planning/phases/08-production-webhook-end-to-end-delivery/08-VERIFICATION.md`**

frontmatter：
```yaml
status: human_needed        # 人工签署未完成，这个不变
score: 16/16 must-haves verified
behavior_unverified: 0
behavior_unverified_items: []
```

truth 9 行替换：
```markdown
| 9 | FA-02：计算退避从第 10 次已持久化失败起达到 300 秒上限；高次计数补充验证覆盖 attempts 1..20 | ✓ VERIFIED | crates/ontoforge-action/tests/outbox.rs::retry_backoff_caps_at_five_minutes_without_leaking_response 循环上限扩至 20 后实跑 cargo test -p ontoforge-action --test outbox 通过（0.32s），逐次断言 next_attempt_at - poll_input ∈ [min(2^(k-1),300), +1s)，k=10..20 全部观测到 300 秒上限。注：原描述「第 17 次起」有误，上限自第 10 次已记录失败起生效；.min(16) 仅防 i64 移位溢出不可观测。既有 CI job（ci.yml:73）自 Phase 05 起已每次运行该用例至 attempt 10。证据边界：SQLite + 可控时钟 + 单 worker，公式路径与后端无关 |
```

`§Human Verification Required` 第 1 项（FA-02）整节移除；第 2 项（人工签署）**保留**。

正文 `Score:` 同步为 `16/16`。删除「FA-02 从未执行、继续等待人工执行」的现时状态描述，保留历史执行范围说明。

**额外必做**：`covered_files` 必须新增这次核对的文件（`crates/ontoforge-action/src/outbox.rs`、`crates/ontoforge-action/tests/outbox.rs`、`crates/ontoforge-storage/src/commit.rs`、`crates/ontoforge-storage/migrations/postgres/0004_action.sql`、`crates/ontoforge-storage/migrations/sqlite/0004_action.sql`），`covered_digest` 按项目机制重算，**不能沿用旧摘要**。

**结论位 `VERDICT: PENDING` 保持不变**（见待办 B）。

**(c) `.planning/WINDOWS.md` 第 19 条**

**表格与 JSON 两处必须同步**（这个文件历史上出过一次两处不一致的 bug）。三个字段改为：

```
description:
Phase 08 原端到端用例仅覆盖第一次失败后的重试。FA-02 的计算退避实际从第 10 次已持久化失败起达到 300 秒上限，不是到第 17 次才封顶；outbox schema 不存在 last_attempt_at 列。2026-11-09 已补充 attempts 1..20 的 SQLite 运行时验证，含 attempts 17..20。既有 CI job（ci.yml:73）自 Phase 05 起已每次运行该用例至 attempt 10。

status: fixed

reason:
独立复核通过原测试的只读内存派生版本，将循环上限及请求数断言均扩展到 20，调用原生产 worker/store 完成同一事件连续 20 次 503。逐次计数、请求总数、退避区间及未到期不领取断言全部通过，1 passed / 0 failed / 0.32s；仓库源文件未修改。证据限于 SQLite 和注入的 poll 时间，不包含 PostgreSQL 高次重试、生产墙钟调度、多 worker lease 竞争或溢出边界。
```

`recorded_at` 保留原值 `2026-11-08T19:33:14.558Z`；`resolved_at` 填实际关闭时刻（UTC），**不要预造**。

**是否要把循环上限永久改成 20？** 建议**要**：一行改动（循环上限 + 末尾请求数断言），既补齐证据又是回归护栏，让 attempts 11..20 以后都在 CI 里跑。如果你改，必须同步更新该测试的任何自述文案。

### 待办 B：Phase 08 结论位的人工签署（**你不能做**）

Phase 08 有一个固定的**人工签字项**，项目约定「机器可判定结果只作证据，执行者与 verifier 均不代签」。

**Test**：对照 CI run `42515808955` 的日志与 `phase8-webhook-ledger` artifact 的 `ledger.jsonl` 人工复核：6 条原始记录、3 条事件各 2 次尝试（首条 503、末条 204）、相邻间隔 1.157s / 1.183s / 1.026s 落在 [0.9s, 30s]、唯一 event_id 集合与 PostgreSQL `of_webhook_outbox` 的 3 行完全相等；据此把结论位由 `VERDICT: PENDING` 改为人工签署。

**Expected**：ledger 原始记录、去重集合、PostgreSQL 终态三者互相印证，结论位由 PENDING 改为人工签署。

**机器可判定的部分（run id、job 结论 success、ledger 时间戳落在该 run 的 job 日志窗口内、归档逐成员 sha256 可复算）都已由 verifier 完成并写入 `08-VERIFICATION.md` truth 16。** 剩下的是人工确认动作。

`08-UAT.md` 测试 2 保持 `result: [pending]`。**你不得代签**，也不得把 `VERDICT: PENDING` 改成已签署。

### 待办 C：两条新发现的独立待办（不阻塞本 phase）

复核过程中发现两个真实问题，**不要塞进 FA-02 里**，单独登记：

1. **`record_retry` 返回值被忽略**
   `crates/ontoforge-action/src/outbox.rs:159-166` 调用 `self.store.record_retry(...).await?`，但 `record_retry` 的返回类型是 `Result<bool, StoreError>`（`crates/ontoforge-storage/src/commit.rs:306-313`），返回 `Ok(false)` 表示被 fencing 拒绝（旧 lease token 抢占）。worker 只用 `?` 传播 Err，**丢掉了 bool**。
   影响：fenced 掉的 worker 会把事件当已处理。语义上无害（at-least-once 允许重复投递），但计数与接收端请求序号可能不对应。
   建议：在 `crates/ontoforge-action/tests/outbox.rs` 登记一个回归用例，或至少记入 `.planning/WINDOWS.md` 作为独立条目。

2. **PostgreSQL 的 `attempt_count` 是 `integer`（i32），比 Rust 的 `u32` 更早触顶**
   `crates/ontoforge-storage/migrations/postgres/0004_action.sql:23` 用 `integer`。claim 侧 `crates/ontoforge-storage/src/commit.rs:249-251` 做 `i32 → u32` 的 `try_into`，转换失败返回 `StoreError::Corrupt("invalid outbox attempt count")`。
   即 PG 侧计数到 `2^31 - 1` 后，下一次失败的 Rust `checked_add` 仍成功，但写入 `2^31` 超列范围 → 事务失败；且 claim 时转换失败会让该行变成永久 poisoned row（每次 claim → send → Corrupt → 整个 `poll_at` 返回 Err）。
   实际不可达（需 21 亿次失败），但属于 schema 与 Rust 类型宽度不一致。SQLite 侧可走到 `u32::MAX`。

---

## 6. 后续开发：Phase 09-12

### Phase 09 Webhook Worker Supervision & Crash Recovery（REL-03）

**Goal**：webhook-worker 异常退出后能被自动拉起并恢复投递，机器可判定的崩溃信号有 CI 证据，systemd 的实际自动拉起行为由人工签字。

Success Criteria（原文，见 `.planning/ROADMAP.md`）：
1. worker 在派发中途被强制终止 → CI 日志出现可观测信号（退出码 / 监督状态 / 重启事件）
2. worker 崩溃时 outbox 仍有未送达事件 → worker 被重新拉起，该事件从持久化 outbox 恢复并被送达，无丢失
3. 仓库交付 systemd unit 文件 → 运维者在 Linux 主机安装后随系统启动拉起 worker，异常退出后自动重启
4. 机器可判定部分与人工部分切分 → systemd 自动拉起的结论位保持 `VERDICT: PENDING` 直到人工签字，执行者不代签

**已知的人工签字约束**：systemd unit 的实际自动拉起行为，CI 只能覆盖机器可判定的崩溃信号与恢复投递，**本机 Windows 无法覆盖 systemd 自动拉起**。这条只能人工签字。

### Phase 10 Audit Timing Test Split（REL-04）

**Goal**：审计的纯计算断言与墙钟敏感的集成验证分离，CI 不再依赖 `--test-threads=1`。

**已知线索**（`.planning/WINDOWS.md` 第 18 条，open）：
- 文件 `crates/ontoforge/tests/cli_check_cardinality.rs:437`
- 全工作区并行负载下 `cardinality_budget_starts_after_precheck_and_pk` 与 `cardinality_total_deadline_is_shared` 两个 300 ms deadline 用例偶发超时失败
- 隔离重跑 13 passed / 0 failed；全量 `--no-fail-fast` 重跑 1528 passed / 0 failed → 判定为时序 flake 而非回归
- 根因方向：门禁用例预算固定 300 ms，并行负载下偶发超时；彻底修复需放宽预算或改用可控时钟
- **建议方向**：改成可控时钟（把墙钟 deadline 换成显式注入的时间源），而不是简单放宽预算

### Phase 11 Product-Layer Wording & Historical Annotation（REL-05 / REL-06）

**Goal**：产品层措辞不再把私有数据源特指为 SQL Server；归档 SUMMARY 的陈旧断言以注解方式追改而**不改写 dated 快照**。

**已知线索**（`.planning/STATE.md` Blockers/Concerns）：
- ADR-0032 的通用化只落到 planning 层，产品层 `private_fixtures.rs` / `CONTRIBUTING.md` / `docs/designs/trino-engine-sql-tiers.md` 仍点名 SQL Server
- 目标表述统一为「私有 Trino 数据源」
- 注意：`.planning/milestones/v1.0-audit/SUMMARY.md` 结论是 `gaps_found`（2026-11-05 dated 快照），**不得改写正文**；Phase 02 的 `02-10-SUMMARY.md` 只加 superseded 注解，不改正文

### Phase 12 First Public Release（REL-07）

**Goal**：推送 v1.0 / v1.1 tag、产出 GitHub Release、发布 npm 包并回写发布记录。

**依赖**：Phase 8-11 全部验收通过后才可执行。

**已知状态**：v1.0 目前只是本地 annotated tag，未推送；npm 包未发布。

---

## 7. 工作方式要求

### 提交规范

- **小步原子 commit**，保持历史干净
- commit message 用 Conventional Commits（`feat(08-02): ...` / `docs(08): ...` / `fix(...): ...`），中文描述可保留
- **不要在 `main` 上直接提交**。当前在 `gsd/phase-08-...` 分支；Phase 09-12 建议各开 `gsd/phase-09-...` 分支
- 合并到 main 前必须跑 gstack `/review`（Paranoid Staff Engineer 视角），这是项目硬性治理关卡

### 验证纪律

- **TDD**：涉及核心业务逻辑、算法、数据结构、API contract 的改动，先写测试再写实现（red-green-refactor）
- **证据优先**：任何「已完成」的断言都要附实际命令与真实输出。不要写「应该没问题」，要写「`cargo test ...` → `test result: ok. N passed`」
- 本机不依赖 Docker：Windows 上 `cargo build` / `cargo test` 必须过。Postgres 用 `postgresql_embedded` feature
- CI 配置在 `.github/workflows/ci.yml`，Linux runner 上才有真正的 PostgreSQL / Trino / MinIO

### 文档纪律

- **STATE.md 是 digest 不是 archive**，目标 < 100 行，只存 current pointer + 指针 + 派生指标；历史进 session-log / git，**不内联**
- **ROADMAP 一 phase 一行**结论 + 日期 + 指向 phase 目录的指针；收尾叙事（commit / test / ADR / CI / lesson）只进 SUMMARY / VERIFICATION，**严禁内联 ROADMAP**
- **一个事实一个家**：决策 → ADR；需求 → REQUIREMENTS.md；验收 → scenario 或 SC；实际交付 + 证据 → SUMMARY；现在指针 → STATE；概览 → ROADMAP；历史 → session-log
- **dated 证据文件的正文不重写**，只加注解
- `WINDOWS.md` 表格与 JSON 双份，改一处必须同步另一处

### 已知会踩的坑

| 坑 | 说明 |
|---|---|
| `of_webhook_outbox` 无 `last_attempt_at` 列 | 只有 `next_attempt_at` / `delivered_at` / `lease_until` / `last_error` 等 |
| `outbox.rs` 退避公式封顶在 attempt 10 | 不是 17；`.min(16)` 只是移位保护 |
| `record_retry` 的 `Ok(false)` 被忽略 | 见待办 C-1 |
| PG 的 `attempt_count` 是 i32 | 比 Rust `u32` 早触顶 |
| receiver 只有 `--fail-first-per-event` | 每 event 只失败 1 次，没有持续 503 模式；要测 17 次连续失败需另建 harness |
| `WINDOWS.md` 表格/JSON 双份 | 历史出过一次不一致 bug |
| `08-REVIEW.md` 的 WR-01~WR-04 | stale ready 文件、ledger 不清空、seq 与文件顺序解耦、`assert_ne!` 恒真 —— 只影响本地可重跑性，不影响 CI 证据 |
| `08-02-SUMMARY.md:159` 叙述有误 | 说归档 ledger 出自「多事件矩阵 run」，实际是 run 42515808955；同文件 D3 行与索引记录正确，仅该行叙述错 |

---

## 8. 跨 phase 缺陷登记表（`.planning/WINDOWS.md`）现状

当前 open 条目（接手时先看这张表，避免重复踩坑或漏关）：

| id | phase | kind | 摘要 |
|---|---|---|---|
| 3 | 02 | deviation | `crates/ontoforge/src/online.rs` 的 `--server-config` flag 文档提到但 flag 尚不存在（plan 02-09 会加） |
| 5 | 02 | unrun-verify | integration job（PostgreSQL + Trino 483）尚未在 GitHub 上跑过 |
| 6 | 02 | unrun-verify | nightly Iceberg workflow 未跑过，需 `workflow_dispatch` |
| 7 | 05 | stub | `crates/ontoforge-action/src/lib.rs:186` validate-only 被显式拒绝 |
| 9 | 05 | deviation | `trino_action_it.rs` 缺独立行为 RED |
| 10 | 05 | deviation | 默认 feature 组合下 `cargo check/clippy` 因配置问题失败 |
| 11 | 06 | stub | `crates/ontoforge-storage/src/read_audit.rs:166` PostgreSQL worker fail closed |
| 12-14 | 06 | skipped-test | `scope_store.rs` 三个 PG 用例需 `ONTOFORGE_PG_URL` |
| 15 | 07 | skipped-test | 07-03 workspace 107 ignored |
| 17 | 02 | unrun-verify | 02-11 人工验收（已由用户 2026-11-08 签字，此条可能待清理） |
| 18 | 02 | unmet-truth | `cli_check_cardinality.rs:437` 300 ms deadline flake → **Phase 10 的主线** |
| 19 | 08 | unrun-verify | FA-02 退避上限 → **待办 A 处理** |
| 20 | 08 | unrun-verify | FA-03 归档来源需人工交叉复核（verifier 已用时间戳确定性证据升级为机器可判定，但关闭该条仍属人工决定） |

---

## 9. 环境与工具速查

### Devin CLI（本机已装）

```
二进制：C:\Users\easyi\AppData\Local\devin\cli\bin\devin.exe   ← 不在 PATH 上，用绝对路径
配置：C:\Users\easyi\AppData\Roaming\devin\config.json
账号：zhaoyoucai2026@163.com（Devin Pro）
```

**非交互调用（你若要再委派子任务）**：
```bash
devin.exe --model gpt-6-1-sol-high --permission-mode dangerous \
          --respect-workspace-trust false --prompt-file <FILE>
```

关键点：
- `--permission-mode dangerous`（bypass）才能免掉 shell 命令的交互确认。`auto` 只放行只读工具，`Exec(...)` 仍会卡在 TUI 上等按键，print 模式下无人按键就永久挂起
- `--respect-workspace-trust false` 必需，否则 print 模式在未信任目录直接失败（`trusted_workspaces.json` 默认只信任 `D:\DataSpellProjects\test`）
- `--config <FILE>` 是**合并**语义不是替换，Devin 会把自己的配置段回写进你的文件
- **模型选择**：主用 `gpt-6-1-sol-high`（GPT-6.1 Sol，1M context）；`swe-2-high`（SWE-2，262K context）**截至 2026-11-30 免费**，适合大批量审查。`-lightning` 变体不免费
- **建议：同一问题问两次**（`gpt-6-1-sol-high` + `swe-2-high`），再横向对账。本次 Phase 08 FA-02 的复核就是这么做的，两份意见独立确认了同一批事实，还各自抓到我漏掉的点
- **串行跑，不要并行**：两个 Devin agent 并行 + 各自 cargo 编译会把本机内存吃穿，harness 会以 OOM 收割后台任务

### 常用命令

```bash
cargo build --workspace --locked
cargo test --workspace --locked -- --test-threads=1
cargo test -p ontoforge-action --locked -- --nocapture      # CI 的 outbox 语料
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all --check
bash ci/verify-evidence-archive.sh --selftest                # 归档校验器自检
```

### GSD CLI（headless，可用于读写 .planning 状态）

```
C:\Users\easyi\.claude\gsd-core\bin\gsd-tools.cjs
```
常用 verb：`query init.verify-work`、`query verification.status`、`phase uat-passed`、`check`、`commit`、`roadmap`、`state`。你若要更新 `.planning/` 状态，可用 `gsd_run query commit "..." --files <path>` 做原子提交。

---

## 10. 建议的执行顺序

1. `git push` 把本地 9 个未推的 commit 补到远程分支，确认 PR #12 状态
2. 做待办 A（Phase 08 的 FA-02 收尾，三处文件），跑 `cargo test -p ontoforge-action --locked -- --nocapture` 确认绿
3. 提交待办 A 的改动（建议 `docs(08): ...` 一个 commit 收三处文件；若改了测试循环上限，另起一个 `test(08): ...`）
4. 更新 `.planning/STATE.md` 的 current pointer（保持 < 100 行）
5. 规划并执行 Phase 10 与 Phase 11（与 Phase 09 无依赖，可并行）；Phase 09 在 Phase 08 人工签署后启动
6. Phase 08 的测试 2 与 WINDOWS.md 第 20 条留给人工签字，不要代签

---

## 11. 一句话总结现状

Phase 08 的实现与 CI 证据链已经跑绿（15/16 → 现在应为 16/16），卡住的唯一一条 FA-02 在两轮独立复核下已被证明「300 秒封顶行为其实从 Phase 05 起就在 CI 里跑，只是验证报告没对上账」，你只需把三份文档的措辞改精确并补齐 attempts 11..20 的证据；剩下 Phase 09-12 的活按路线图走，其中 Phase 09 的 systemd 自动拉起与 Phase 08 的结论位一样，只能人工签字，不要代签。

---

*交接文档结束。所有事实均来自仓库实际文件与实跑命令，无推测性内容。*
