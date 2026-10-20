---
gsd_state_version: "1.0"
current_phase: 03
current_phase_name: Edit Store & Scope Model
status: executing
stopped_at: Completed 03-08-PLAN.md
last_updated: "2026-10-20T03:34:07.982Z"
last_activity: 2026-10-20
last_activity_desc: Phase 03 execution started
state_head: 98ae1e2527c8bee46486faa216944681a14897eb
progress:
  total_phases: 7
  completed_phases: 0
  total_plans: 44
  completed_plans: 33
  percent: 0
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-10-19)

**Core value:** 定义一次 ontology，应用和 agent 就能通过 REST 与 MCP 对同一份对象图做受控读写：写入一律经 Action 校验与授权，每次写入都可审计。
**Current focus:** Phase 03 — Edit Store & Scope Model

## Current Position

Phase: 03 (Edit Store & Scope Model) — EXECUTING
Plan: 10 of 20
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

Last session: 2026-10-20T03:34:07.852Z
Stopped at: Completed 03-08-PLAN.md
Resume file: None
