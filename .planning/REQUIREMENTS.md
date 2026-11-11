# Requirements: ontoforge

**Milestone:** v1.1 Gap Closure（缺口收口）
**Defined:** 2026-11-08
**Core Value:** 定义一次 ontology，应用和 agent 就能通过 REST 与 MCP 两个同等地位的出口，对同一份对象图做受控的读写：写入一律经 Action 校验与授权，每次写入都可审计。

「开发者」指编写 ontology 的人，「运维者」指部署并运行 ontoforge 的人，「接收端」指接收 webhook 事件的下游 HTTP 服务。

v1.1 是收口 milestone：范围来自 v1.0 关闭时逐条记录的 4 项残余缺口（见 `.planning/MILESTONES.md` 的 Known Gaps 与 `.planning/STATE.md` 的 Deferred Items），不含新功能。

## v1 Requirements

### 生产 webhook 联调

- [x] **REL-01**: 运维者能在 Linux CI 上运行 `ontoforge webhook-worker`，把 outbox 中的事件投递到**真实 HTTP 接收端**（独立进程监听端口），CI 日志中留下接收端收到事件的证据
- [x] **REL-02**: 接收端返回 503 时，webhook-worker 按退避策略重试直至成功或触达上限；同一 `event_id` 在重试与重复投递场景下被接收端只计一次，整体保证至少一次送达
- [x] **REL-03**: webhook-worker 进程异常退出时，部署形态能自动拉起并恢复投递；CI 验证异常退出的可观测信号，unit 的实际自动拉起行为由人工验收签字（结论位 `VERDICT: PENDING`）

### 审计与测试收口

- [x] **REL-04**: `security::cap_counts_final_masked_row_and_version` 的纯计算断言与依赖墙钟的时间敏感集成验证分离，CI 不再需要 `--test-threads=1` 即可稳定通过

### 措辞与文档追改

- [x] **REL-05**: 仓库产品层的代码注释与文档不再把私有数据源特指为 SQL Server（`private_fixtures.rs` / `CONTRIBUTING.md` / `docs/designs/trino-engine-sql-tiers.md`），措辞统一为「私有 Trino 数据源」
- [x] **REL-06**: 归档的 `02-10-SUMMARY.md` 对陈旧断言（本地 warehouse 描述、测试计数）加 superseded 注解指向当前状态，SUMMARY 正文作为 dated 快照保留不被改写

### 首次对外发布

- [x] **REL-07**: v1.1 验收通过后，v1.0 与 v1.1 tag 推送到远端、GitHub Release 产出、npm 包发布成功，发布记录写回 `.planning/MILESTONES.md`

## Deferred Requirements

v2.0 及以后的需求不纳入本 milestone，编号与正文见 `.planning/milestones/v1.0-REQUIREMENTS.md` 的 `## v2 Requirements` 节：

- 湖仓写入与联邦计算：LAKE-01..03、SQL-01
- ontology 表达力扩展：SCHEMAX-01..05
- 治理闭环：GOV-01
- 检索与运行时：SEARCH-01..02、FUNC-01..02、SUB-01、SCEN-01、BRANCH-01、SDK-01、GQL-01
- 安全：SEC-02、SEC-03

## Out of Scope

| Feature | Reason |
|---------|--------|
| v2.0 的功能方向（湖仓写入、ontology 表达力、治理、检索运行时） | 范围过大，已排入 v2.0；v1.1 只做收口，不加新功能 |
| 改写 v1.0 里程碑审计文件正文 | 审计文件是 dated 快照证据，只新增复核记录，不改原文 |
| systemd 之外的进程守护方案（supervisord / s6 / 自建拉起器） | 生产部署目标为 Linux，systemd 是既定形态 |
| 追溯 Phase 01 G-1-2 的编辑器体验人工观察边界 | 已由 Phase 1 gap closure 自动化补全，剩余部分是不可自动化的观察边界 |
| 清理 v1.0 全部文档数值漂移 | 只处理与 v1.0 关闭记录直接对应的 4 项缺口 |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| REL-01 | Phase 8 | Complete |
| REL-02 | Phase 8 | Complete |
| REL-03 | Phase 9 | Pending |
| REL-04 | Phase 10 | Pending |
| REL-05 | Phase 11 | Pending |
| REL-06 | Phase 11 | Pending |
| REL-07 | Phase 12 | Pending |

**Coverage:**
- v1 requirements: 7 total
- Mapped to phases: 7
- Unmapped: 0

---
*Requirements defined: 2026-11-08*
*Last updated: 2026-11-08 after roadmap creation (v1.1, Phase 8-12 mapped 7/7)*


## v2.0 Admin Console (Phases 13-17)

- [ ] **CON-01**: 全部管理写（定义发布/source 启停/role 绑定/outbox 重试）经受保护管理 Action，授权规则不可被普通 action 编辑提权
- [ ] **CON-02**: 数据源凭据经服务端 url_env/url_file 引用解析且永不明文回传，引用白名单 + 连接测试 SSRF 防护 + 全输出脱敏
- [ ] **CON-03**: source 配置 DB 持久化（revision/enabled），TOML 仅 bootstrap+引用白名单；显式导入接管无双写；desired/applied 状态分离
- [ ] **CON-04**: OIDC 自托管 IdP 登录 + 服务端会话（HttpOnly/Secure/SameSite/CSRF/撤销），dev-token 仅限开发模式，首管理员 CLI bootstrap
- [ ] **CON-05**: ontology 定义以 change-set 校验发布（引用完整性+乐观锁+破坏性阻断），变更与审计可靠关联
- [ ] **CON-06**:  SPA（React+Vite+TS+AntD+TanStack Query+仓内 SDK），同源服务端会话，发行物含预构建静态资源且离线可建
