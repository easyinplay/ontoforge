---
gsd_state_version: "1.0"
current_phase: 03
current_phase_name: Edit Store & Scope Model
status: executing
stopped_at: Completed 03-18-PLAN.md
last_updated: "2026-10-21T01:45:26.509Z"
last_activity: 2026-10-20
last_activity_desc: Phase 03 execution started
state_head: c62cc15cbe346f7d3e60e75b7c4b9ec3a5f12f8b
progress:
  total_phases: 7
  completed_phases: 0
  total_plans: 44
  completed_plans: 42
  percent: 0
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-10-19)

**Core value:** 定义一次 ontology，应用和 agent 就能通过 REST 与 MCP 对同一份对象图做受控读写：写入一律经 Action 校验与授权，每次写入都可审计。
**Current focus:** Phase 03 — Edit Store & Scope Model

## Current Position

Phase: 03 (Edit Store & Scope Model) — EXECUTING
Plan: 19 of 20
Status: Ready to execute
Last activity: 2026-10-20 — Phase 03 execution started

Progress: [██████████░░░░░░░░░░] 24/44 plans ([░░░░░░░░░░] 0%); phases 2/7 complete

## Performance Metrics

逐 plan 的耗时与文件数见各 `phases/*/NN-*-SUMMARY.md`。

| Plan | Duration | Tasks | Files |
|------|----------|-------|-------|
| Phase 03 P01 | 40min | 3 tasks | 15 files |
| Phase 03 P02 | 30min | 2 tasks | 13 files |
| Phase 03 P03 | 90min | 3 tasks | 14 files |
| Phase 03 P05 | 80min | 3 tasks | 12 files |
| Phase 03 P06 | 21 min | 2 tasks | 17 files |
| Phase 03 P07 | 50 min | 2 tasks | 9 files |
| Phase 03 P04 | 80 min | 2 tasks | 16 files |
| Phase 03 P11 | n/a | 3 tasks | 15 files |
| Phase 03 P08 | ~1h | 2 tasks | 14 files |
| Phase 03 P10 | ~1h | 2 tasks | 9 files |
| Phase 03 P12 | 20 min | 2 tasks | 8 files |
| Phase 03 P14 | 25 min | 2 tasks | 8 files |
| Phase 03 P09 | 32 min | 2 tasks | 6 files |
| Phase 03 P15 | 2h | 2 tasks | 14 files |
| Phase 03 P13 | 55 min | 3 tasks | 9 files |
| Phase 03 P16 | 1h 50m | 2 tasks | 16 files |
| Phase 03 P17 | 35 min | 2 tasks | 3 files |
| Phase 03 P18 | 4h | 3 tasks | 20 files |

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

Last session: 2026-10-21T01:45:26.347Z
Stopped at: Completed 03-18-PLAN.md
Resume file: None
