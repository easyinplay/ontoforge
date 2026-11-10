# Project Retrospective

*A living document updated after each milestone. Lessons feed forward into future planning.*

## Milestone: v1.0 — MVP

**Shipped:** 2026-11-08
**Phases:** 9 | **Plans:** 94 | **Sessions:** 未能从仓库产物统计

### What Was Built

- **Phase 1 — Workspace, Schema & Lint**：YAML/JSON 合并加载为单一 IR，`ontoforge validate` / `lint` 输出带 file:line:col 的诊断（ONT-E 与 ONT-W01..W11），`ontoforge schema export` 导出 JSON Schema 供编辑器补全。
- **Phase 2 + 07.2 — Source Binding & Engine**：`SourceBinding` 与必填 `backing`、类型契约 lint（ONT-E13..E16）、自写 Trino client、自有 SQL printer（proptest 锁结构不变）、SQLite 替身、Windows 本机零 Docker 全绿；Linux CI 跑 Trino + PG service container，nightly 加跑 Iceberg，MinIO / S3 精确合同由 07.2 补证闭合。
- **Phase 3 — Edit Store & Scope Model**：按 object type 生成强类型 edit 表、append-only 不可变 edit log 与 `log::replay` 重建、CEL 行谓词的 SQL 编译与 Rust 单行求值差分 proptest 一致、OIDC JWT + dev token 认证、`ontoforge reload / migrate / scope / import` 管理面。
- **Phase 4 + 07.1 — Scoped L0 Read**：Rust 编排 source/edit 叠加的 pk 读、过滤、排序与 keyset 分页、searchAround、集合运算、聚合分解与 HMAC `_version` 令牌；07.1 补齐 source OneToOne 基数报告（整轮总预算、有界诊断）。
- **Phase 5 — Online Action**：Action 鉴权管线、单事务写 edit + edit log + `action_commands` + webhook outbox、并发令牌与 `VersionConflict`、`command_id` 主体作用域幂等。
- **Phase 6 — L2 Raw DQL, Toolsets & Audit**：allow-list 内只读 Trino DQL 的 AST 白名单、catalog / schema 限制、L2 身份受限证明失败即禁用并告警、读元数据审计 fail-closed。
- **Phase 7 — REST & MCP Shell**：OpenAPI 3.1 快照测试、Rust / TypeScript client、按主体过滤的 MCP toolset、每个 action type 自动暴露为 MCP 工具、`ontoforge serve --dev` 单二进制持久化 dev 演示、三平台发行包。

### What Worked

- **纵切片重排先于实现**：ADR-0015..0020 把 Phase 2–7 重排为「数据源绑定与引擎 → edit store 与 scope → scoped L0 读 → online Action → L2 / toolset / 审计 → REST 与 MCP 外壳」，每层都在上一层合同上直接叠加，没有返工。
- **Windows 本机零外部依赖门槛**：SQLite 替身 + `postgresql_embedded` 让 94 个 plan 的验收在本机可跑，真实 Trino / PG 交给 Linux CI service container，nightly 再加 Iceberg。
- **独立 verifier 按原合同重新签署**：Phase 03–07 与 07.1 不是刷新 fingerprint，而是逐条核对源码、具名测试断言与当前 HEAD 执行日志后重新签署。
- **gap closure 独立成 phase**：07.1（source 基数）与 07.2（MinIO 合同）作为插入 phase 单独立项，保留原 phase 的合同边界与证据来源。
- **机器可判定与人工 judgment 显式切分**：结论位固定 `VERDICT: PENDING`，执行者不代签，G-02-B/C/D 于 2026-11-08 由用户逐条签字。

### What Was Inefficient

- **需求措辞按厂商分列直接变成未闭合合同**：Phase 02 早期把「私有 SQL Server fixture」写进 ENGINE-05 合同，导致 07.2 关闭 MinIO 子范围后仍需 Phase 02 在 2026-11-08 单独验收闭环，多花一轮审计与人工签字（最终由 ADR-0032 通用化收口）。
- **「生产环境」证据从 phase 早期就被推到 deferred**：Phase 05 的真实下游接收端联调、Phase 06 的审计时序项在 phase 内没有替身也没有闭环，到 milestone close 时仍是开放项，只能以 acknowledge 承接为技术债。
- **报告门与真实行为脱节需要额外一轮**：2026-11-05 的 milestone 审计以「严格报告门」口径判定 8 项 unsatisfied（全部由当时 Phase 02 的 `gaps_found` 驱动），Phase 02 单独闭环后这 8 项即失效，审计文件本身却只能作为 dated 快照保留。
- **历史 SUMMARY 快照陈旧**：Phase 02 的 `02-10-SUMMARY.md` 里的本地 warehouse 描述与测试计数在后续改动后不再准确，只能作为快照保留并由 STATE.md 的 Blockers 显式披露。
- **Phase 03 的历史证据边界需要一次用户显式 override 才落定**（03-01.1 安装前人工批准缺独立留档）。

### Patterns Established

- **ADR 驱动的合同分层**：ADR-0002（命名）、ADR-0011（workspace 布局）、ADR-0016（数据分层）、ADR-0018（scope 授权模型）、ADR-0023（依赖 legitimacy 与 crate 依赖方向）、ADR-0032（私有数据源不按数据库类型分列）构成 v1.0 的稳定决策底座。
- **报告门由独立 verifier 签署**：每 phase 的 VERIFICATION 分数是「按原合同逐条核对」的产物，不可由 fingerprint 刷新获得绿色。
- **证据边界显式声明**：CI job 真实执行数、ignored 数、historical 与 current 的区分在 SUMMARY / VERIFICATION / UAT 中逐项写明，不把局部绿灯当整体通过。
- **milestone 收尾形态**：phase 目录归档到 `.planning/milestones/v[X.Y]-phases/`，ROADMAP 折叠为 milestone 分组，REQUIREMENTS 归档后 `git rm` 以保留历史，残余项走 `audit-open acknowledge` + STATE.md `## Deferred Items` 披露。

### Key Lessons

1. 需求措辞里「按产品或厂商类型分列」会直接变成未闭合合同；描述外部系统能力边界时按能力（私有 Trino 数据源）而非按数据库类型分列。
2. 跨 phase 的 judgment 必须在最早暴露它的 phase 就给出证据契约（机器可判定 / 人工签字两段式），否则会在 milestone 审计时集中回流成 gap。
3. 成功标准里若包含「生产环境证据」（真实下游接收端、慢机器时序），要么在 phase 内提供替身并写明边界，要么显式排进下一 milestone，不要留成 phase 末期的隐性欠账。
4. 独立 verifier 按原合同重新签署比刷新 fingerprint 更能防止报告门与真实行为脱节。
5. milestone 关闭不应改写 dated 审计结论：用 acknowledge + Deferred Items 承接残余项，让审计文件保持可追溯，同时在 MILESTONES.md 的 Known Gaps 里如实写明审计结论仍是 `gaps_found`。

### Cost Observations

- Model mix: 未能从仓库产物统计。
- Sessions: 未能从仓库产物统计。
- Commits: 1,049（`b4d11f6` → `b9a3700`，2026-10-15 → 2026-11-08）。
- Notable: 9 个 phase、94 个 plan、115 个 task 在 25 天内完成并逐 phase 独立验收；可计量的开销集中在「验收与复签」而非「实现」——Phase 03（23 plans）与 Phase 07（16 plans）各经历了一轮完整合同复核，而 Phase 02 额外经历了 milestone 审计、Phase 07.2 子范围补证与单独验收闭环三轮。

---

## Cross-Milestone Trends

### Process Evolution

| Milestone | Sessions | Phases | Key Change |
|-----------|----------|--------|------------|
| v1.0 | 未统计 | 9 | 从「定义层」到「双出口」的完整纵切片落地；phase 完成后由独立 verifier 按原合同复签，gap closure 走插入 phase 而非改写原合同；milestone 收尾引入 audit-open acknowledge + Deferred Items 承接残余项 |

### Cumulative Quality

| Milestone | Tests | Coverage | Zero-Dep Additions |
|-----------|-------|----------|-------------------|
| v1.0 | 本地 workspace 1528 passed / 0 failed / 119 ignored（2026-11-05 审计口径，`milestones/v1.0-audit/FINAL-REVERIFY.md`）；CI 41994479295：Ubuntu 1535 / 0 / 119、Windows 1528 / 0 / 119 | 未测量（仓库无覆盖率工具接入记录） | 不适用（依赖治理见 ADR-0011 / ADR-0023；本机零 Docker 门槛由 SQLite 替身达成） |

### Top Lessons (Verified Across Milestones)

1. 合同措辞按能力而非按厂商分列，才能在外部系统能力边界变化时用一次 ADR 通用化收口全部相关合同（ADR-0032）。
2. 「机器可判定 / 人工 judgment」显式切分加固定 `VERDICT: PENDING` 结论位，是 milestone 级审计能收敛的前提。
3. phase 内未闭环的 deferred 项会原样漂移到 milestone close；把「生产环境证据」在成功标准阶段就分类，是控制漂移的唯一有效时点。
