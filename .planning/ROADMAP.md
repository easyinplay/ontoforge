# Roadmap: ontoforge

## Milestones

- 🚧 **v1.1 Gap Closure** — Phases 8-12（started 2026-11-08，in progress）
- ✅ **v1.0 MVP** — Phases 1-7、7.1、7.2（shipped 2026-11-08）

## Phases

- [x] **Phase 8: Production Webhook End-to-End Delivery** - CI 上把 outbox 事件端到端投递给真实 HTTP 接收端，覆盖 503 重试、`event_id` 去重与至少一次送达
- [ ] **Phase 9: Webhook Worker Supervision & Crash Recovery** - webhook-worker 异常退出的可观测信号与恢复投递，systemd unit 自动拉起为人工签字项
- [ ] **Phase 10: Audit Timing Test Split** - 纯计算断言与墙钟敏感集成验证分离，CI 不再依赖 `--test-threads=1`
- [ ] **Phase 11: Product-Layer Wording & Historical Annotation** - ADR-0032 通用化落到产品层措辞，Phase 02 陈旧 SUMMARY 加 superseded 注解
- [ ] **Phase 12: First Public Release** - 推送 v1.0 / v1.1 tag、产出 GitHub Release、发布 npm 包

<details>
<summary>✅ v1.0 MVP (Phases 1-7, 7.1, 7.2) — SHIPPED 2026-11-08</summary>

- [x] Phase 1: Workspace, Schema & Lint (14/14 plans) — completed 2026-11-07
- [x] Phase 2: Source Binding & Engine (11/11 plans) — completed 2026-11-08
- [x] Phase 3: Edit Store & Scope Model (23/23 plans) — completed 2026-10-22
- [x] Phase 4: Scoped L0 Read (7/7 plans) — completed 2026-10-23
- [x] Phase 5: Online Action (7/7 plans) — completed 2026-10-26
- [x] Phase 6: L2 Raw DQL, Toolsets & Audit (11/11 plans) — completed 2026-10-28
- [x] Phase 7: REST & MCP Shell (16/16 plans) — completed 2026-11-04
- [x] Phase 7.1: Source Cardinality Gap Closure (3/3 plans) — completed 2026-11-04
- [x] Phase 7.2: MinIO Contract Gap Closure (2/2 plans) — completed 2026-11-04

</details>

## Phase Details

### Phase 8: Production Webhook End-to-End Delivery

**Goal**: 运维者能在 Linux CI 上把 outbox 中的事件投递给真实 HTTP 接收端，并观察到重试、去重与至少一次送达语义
**Mode:** gap-closure
**Depends on:** Nothing (first phase of milestone v1.1)
**Requirements**: REL-01, REL-02
**Success Criteria** (what must be TRUE):

  1. Given outbox 中存在待投递事件, When `ontoforge webhook-worker` 在 GitHub Actions Linux runner 上对独立进程监听的真实 HTTP 接收端运行, Then 接收端记录到该事件载荷, 且 CI 日志留存接收端侧的收到证据
  2. Given 接收端对同一事件前若干次返回 503, When worker 按退避策略重试, Then 接收端转为 2xx 后该事件被成功投递, 且 CI 日志留存每次尝试的次数与退避间隔
  3. Given 同一 `event_id` 因重试或 outbox 重复行被投递多次, When 接收端按 `event_id` 计数, Then 该 `event_id` 被计为一次
  4. Given outbox 中有 N 条事件, When worker 排空队列, Then 接收端对每条事件至少观察到一次, 无事件被静默丢弃
  5. Given 本次联调的接收端实现, When 审阅 CI 配置与日志, Then 接收端是独立进程监听真实端口, 不使用内存内 mock 替代

**Plans**: 2/2 plans executed

Plans:
**Wave 1**
- [x] 08-01-PLAN.md — 接收端 harness、被 ignore 的端到端用例与 `phase8-webhook-e2e` CI job：单事件纵切片加多事件去重/至少一次送达矩阵

**Wave 2** *(blocked on Wave 1 completion)*
- [x] 08-02-PLAN.md — CI 证据归档三件套（README / index JSON / tar.gz）与 worker 运维文档里的端到端联调入口

### Phase 9: Webhook Worker Supervision & Crash Recovery

**Goal**: webhook-worker 异常退出后能被自动拉起并恢复投递, 机器可判定的崩溃信号有 CI 证据, systemd 的实际自动拉起行为由人工签字
**Mode:** gap-closure
**Depends on**: Phase 8
**Requirements**: REL-03
**Success Criteria** (what must be TRUE):

  1. Given worker 在派发中途被强制终止, When 监督方观察到该退出, Then CI 日志出现可观测信号（退出码 / 监督状态 / 重启事件）
  2. Given worker 崩溃时 outbox 仍有未送达事件, When worker 被重新拉起, Then 该事件从持久化 outbox 恢复并被送达, 无丢失
  3. Given 仓库交付 systemd unit 文件, When 运维者在 Linux 主机安装该 unit, Then unit 随系统启动拉起 worker, 并在异常退出后自动重启
  4. Given 本 phase 的机器可判定部分与人工部分, When 结论写入验收记录, Then 机器结果仅作证据, systemd 自动拉起的结论位保持 `VERDICT: PENDING` 直到人工签字, 执行者不代签

**Plans**: TBD

### Phase 10: Audit Timing Test Split

**Goal**: 审计的纯计算断言与墙钟敏感的集成验证分离, CI 不再依赖 `--test-threads=1`
**Mode:** gap-closure
**Depends on**: Nothing (independent of Phase 8 / 9)
**Requirements**: REL-04
**Success Criteria** (what must be TRUE):

  1. Given 16 MiB 行触发的最终字节上限断言, When 该断言在并行测试中运行, Then 它不再依赖墙钟, 单独运行与并行运行均稳定通过
  2. Given read audit 的 1 秒 pending 期限集成验证, When 它运行, Then 它是与纯计算断言分离的独立测试, 期限被超时时给出明确的失败原因
  3. Given CI 的 workspace 测试任务, When 移除 `--test-threads=1` 规避项后连续多次运行, Then 任务稳定为绿
  4. Given 一台较慢的机器, When 工作区测试以默认并行度运行, Then 不再出现由该断言拆分问题导致的 `AuditUnavailable` 波动

**Plans**: TBD

### Phase 11: Product-Layer Wording & Historical Annotation

**Goal**: 产品层措辞不再把私有数据源特指为 SQL Server, 归档 SUMMARY 的陈旧断言以注解方式追改而不改写 dated 快照
**Mode:** gap-closure
**Depends on**: Nothing (independent of Phase 8-10)
**Requirements**: REL-05, REL-06
**Success Criteria** (what must be TRUE):

  1. Given 读者检索仓库产品层中关于私有数据源的表述, When 查看 `private_fixtures.rs` / `CONTRIBUTING.md` / `docs/designs/trino-engine-sql-tiers.md`, Then 无任何表述把私有数据源特指为 SQL Server, 措辞统一为「私有 Trino 数据源」
  2. Given ADR-0032 的通用化结论, When 读者阅读受影响的代码注释与文档, Then 术语与 ADR-0032 一致, 不残留任何按数据库类型分叉的断言
  3. Given 读者打开归档的 `02-10-SUMMARY.md`, When 阅读文件开头, Then 有一条 superseded 注解指向当前状态（本地 warehouse 描述、测试计数）, 且 SUMMARY 正文作为 dated 快照未被改写
  4. Given 复核历史证据文件（里程碑审计、已归档 SUMMARY）, When 检查差异, Then 没有任何 dated 证据文件的正文被重写, 只有注解被新增

**Plans**: TBD

### Phase 12: First Public Release

**Goal**: 首次对外发布: 推送 v1.0 / v1.1 tag、产出 GitHub Release、发布 npm 包并回写发布记录
**Mode:** gap-closure
**Depends on**: Phase 8, Phase 9, Phase 10, Phase 11
**Requirements**: REL-07
**Success Criteria** (what must be TRUE):

  1. Given v1.1 验收已通过, When 执行发布, Then `v1.0` 与 `v1.1` 两个 tag 被推送到远端并在远端仓库可见
  2. Given tag 已推送, When 用户访问仓库的 Releases 页面, Then 存在 v1.1 的 GitHub Release 且附带 release notes
  3. Given npm registry, When 用户查询该包, Then 包已发布, 在 v1.1 版本可被安装
  4. Given 发布完成, When 查阅项目发布记录, Then `.planning/MILESTONES.md` 记录了 tag、Release 链接与 npm 包版本, 且 v1.1 的缺口收口状态不再声称存在开放缺口

**Plans**: TBD

## Archive

- 完整 roadmap（阶段目标、Success Criteria、plan 清单、Progress 表）：`.planning/milestones/v1.0-ROADMAP.md`
- v1 需求归档（79 项全部 Complete）：`.planning/milestones/v1.0-REQUIREMENTS.md`
- 阶段文档与验收证据：`.planning/milestones/v1.0-phases/`
- milestone 审计（结论 `gaps_found`，为 dated 快照）：`.planning/milestones/v1.0-audit/`，历史快照 `.planning/milestones/v1.0-MILESTONE-AUDIT.md`
- 里程碑总结与已知缺口：`.planning/MILESTONES.md`
