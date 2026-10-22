---
gsd_state_version: "1.0"
current_phase: 4
current_phase_name: Scoped L0 Read
status: planning
stopped_at: Phase 4 plans verified, ready to execute 04-01
last_updated: "2026-10-22T12:19:49+08:00"
last_activity: 2026-10-22
last_activity_desc: Phase 4 seven plans passed independent checker
state_head: a712ab27421a9d235f6a5931c5e00b06267a3aab
progress:
  total_phases: 7
  completed_phases: 3
  total_plans: 54
  completed_plans: 47
  percent: 43
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-10-19)

**Core value:** 定义一次 ontology，应用和 agent 就能通过 REST 与 MCP 对同一份对象图做受控读写：写入一律经 Action 校验与授权，每次写入都可审计。
**Current focus:** Phase 4 — Scoped L0 Read

## Current Position

Phase: 4 — Scoped L0 Read
Plan: 04-01 of 04-07, queued
Status: Planning
Last activity: 2026-10-22 — Phase 4 plans passed checker; see `phases/04-scoped-l0-read/task_plan.md`

Progress: [████░░░░░░] 43%

## Performance Metrics

逐 plan 的耗时与文件数见各 `phases/*/NN-*-SUMMARY.md`。

## Accumulated Context

### Decisions

决策全文见 `.planning/adr/`，摘要见 PROJECT.md Key Decisions；phase 实施细节见各 SUMMARY。

- [Phase 2]: 绑定独立成 `SourceBinding` 文件、`backing` 必填无默认（ADR-0021）
- [Phase 2]: 被忽略的 fixture 诊断降为 Info，code 仍为 ONT-V17（ADR-0022）
- [Phase 2]: nightly Iceberg 用 Trino 本地文件系统代替 MinIO；换 S3 兼容镜像属供应链决策，留给用户（详: `phases/02-source-binding-engine/02-10-SUMMARY.md`）
- [Design]: Trino 主引擎 + 分级 SQL，ADR-0015..0020（详: `docs/designs/trino-engine-sql-tiers.md`）
- [Phase 03]: [Phase 03-01]: Crate direction storage -> schema/expr/auth/engine, auth -> schema/expr/engine (ADR-0023); jsonwebtoken aws_lc_rs only; postgresql_embedded test-only under embedded-pg
- [Phase 03]: [Phase 03-02]: ONT-V21..V27 are V-level (not suppressible via ontoforge.toml); OIDC issuer must not contain '|'; PG connection string only via url_env/url_file
- [Phase 03]: [Phase 03-03]: edit store set_mask is one <col>__set boolean per non-pk property (ADR-0024, option-a); no set_mask bigint or slot registry; SQLite flag/system columns default 0 so reload can repair tables
- [Phase 03]: [Phase 03-05]: Row predicates parse with cel but evaluate with own Kleene Pred evaluator (ADR-0025); x in [] is False, no null list elements, string match case-sensitive with literal wildcards, no special case for missing attributes; supersedes ADR-0018's 'missing attribute is false'
- [Phase 03]: 03-06: an empty scope directory is ONT-V14, not an empty ScopeSet; TypeGrant.properties is required but may be empty (03-14 judges it) — A mistyped scope path must not silently grant nothing; allow-list content is an ontology-level check
- [Phase 03]: 03-07: key of another type or declared alg is UnknownKey; null attr claim reads as absent; list claim with null/foreign element rejects the token — Cross-issuer forgery with shared kid fails closed; absent attribute is invisible under D-06; ADR-0025 rule 2
- [Phase 03]: PostgreSQL system columns share SQLite defaults (of_version/of_def_version bigint DEFAULT 0, of_deleted/of_released boolean DEFAULT false); ADR-0024 amended — One repair path (ADD COLUMN after interrupted reload) and one physical_json for both backends; no cost on PG 11+
- [Phase 03]: 03-08: edit log format = plan layout + S1-S4 (UTC ms timestamps, SQLite edit column with space separator, JSON compared as values, committed_at from Rust); float rounded to f32 once; PG arrays bound as text[] — One canonical PropertyValue encoding for log, history and both backends; seq allocated after row locks keeps per-pk replay order
- [Phase 03]: 03-10: removed plain indexes stay in the stored PhysicalSchema (keep_plain_indexes_of) so reload stays idempotent; index names checked schema-wide for ONT-V24
- [Phase 03]: 03-14: example dev binding collation moved from Flight.status to flightNumber so the example dispatcher rowFilter satisfies D-08 — A rowFilter cannot read a caseInsensitive column (ONT-V22); the shipped dispatcher role filters on status
- [Phase 03]: 03-14: computed object index in a submission criterion is ONT-V23 for any role granted the action (fail-closed) — Which property a criterion reads cannot be proven statically, so a dry run could probe a hidden column
- [Phase 03]: Replay applies all log records in one transaction and requires the target to hold the records' def_version — A failed replay leaves the target untouched; cross-version replay waits for 03-15 migration_json (ADR-0026)
- [Phase 03]: 03-15: indexes on a column or table are dropped when it is archived, not at clean-up, because index names are schema-global — A kept index would make CREATE INDEX IF NOT EXISTS on a re-added column of the same name a silent no-op
- [Phase 03]: 03-13: a commit writes each row once in its final state in lock order; replay shares that path (ADR-0026 section 7 amended); swapping two oneToOne keys in one commit is a CardinalityViolation
- [Phase 03]: 03-16: only scope_store.rs names of_role / of_role_binding / of_admin_audit (source guard); scope apply is a full sync bound to a reviewed plan hash, re-planned in the write transaction under the definition advisory lock — SCOPE-07 / D-01..D-03; indexes created for row predicates are only ever added and are not recorded in the physical schema
- [Phase 03]: 03-18: reload/migrate lock the state row before DDL, advisory lock key per edit schema, lock_timeout set before the advisory lock; migrate probes Trino after its single-transaction switch — Avoids fence/DDL deadlock (40P01), unbounded lock waits and cross-deployment serialization; archive/rename steps cannot be committed ahead of the switch
- [Phase 03]: 03-19: import checks the whole file before writing and treats a deleted object or removed edge as existing in every mode — An import is not a general write door: create-only by default, skip and update are explicit, nothing brings back a deleted object, a failed batch is reported so a skip rerun finishes the file; existence is read before the batches (a concurrent creator between read and commit is not refused, see SUMMARY)
- [Phase 03]: 03-21: decimal 的 JSON 数字只收整数，带小数要求写成字符串；NUL 与 0 年 date / timestamp 按两后端交集在校验阶段拒绝；WR-A03 实测为 disproved，会话设置固定仅作纵深防御
- [Phase 03]: 03-22: migrate --apply 在定义锁内复核已 apply 的 role(roles: Option<&RolesCheck>);objects.<param> 整体读取按读取该类型全部属性检查 allow-list — SCOPE-06 / D-18 延伸到 migrate;关闭 WR-B07 与 WR-C05
- [Phase 03]: 03-23: 目录创建放在库里（EditStore::open、DevKey::load_or_create），CLI 与 serve --dev 共用；Windows 密钥 ACL 不改，记为已知缺口 7

### Pending Todos

None yet.

### Blockers/Concerns

- [Ops]: 测试凭据待轮换（`ontoforge_trino_ro`、postgres），测试 catalog `ontoforge_pg` 待删除
- [Phase 2]: 非阻塞安全 observations 待后续 phase 跟进（详: `phases/02-source-binding-engine/02-SECURITY.md` Observations）

## Deferred Items

| Category | Item | Status | Deferred At | Milestone |
|----------|------|--------|-------------|-----------|
| *(none)* | | | | |

## Session Continuity

Last session: 2026-10-21T08:48:07.463Z
Stopped at: Phase 4 plans verified, ready to execute 04-01
Resume file: None
