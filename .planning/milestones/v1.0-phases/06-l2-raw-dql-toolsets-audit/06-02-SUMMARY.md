---
phase: 06-l2-raw-dql-toolsets-audit
plan: 02
subsystem: database
tags: [postgresql, sqlite, read-audit, conformance, tdd]
requires:
  - phase: 06-l2-raw-dql-toolsets-audit
    provides: "06-01 的有界审计 worker 与 SQLite pk tracer"
provides:
  - "PostgreSQL system schema 中的元数据审计表及事务批量 sink"
  - "SQLite/PG 的字段、保留期、批次失败与健康门禁 conformance"
affects: [06-05, 06-06, 06-08]
actuals:
  tokens: 5505
  tasks: 2
  commits: 4
commits: 4
plan_head_before: 6f98b4d81a5ea7acae2b937c77a17c54e69ad77a
plan_head_after: dff7b875b7595bd16ca31c69156f60a80da5b0fd
tech-stack:
  added: []
  patterns: ["单 worker 双后端事务 sink", "受理后 sink unhealthy 的 in-flight 关门"]
key-files:
  created:
    - crates/ontoforge-storage/migrations/postgres/0005_read_audit.sql
  modified:
    - crates/ontoforge-storage/src/read_audit.rs
    - crates/ontoforge-storage/tests/conformance/mod.rs
    - crates/ontoforge-storage/tests/conformance_pg.rs
key-decisions:
  - "PG 的 at 使用 timestamptz，条件与 role 数组使用 jsonb；系统 schema 连接的 search_path 隔离编辑表。"
  - "DELETE 保留期清理和 batch INSERT 在同一事务；任何失败均不留下半批。"
requirements-completed: []
coverage:
  - id: D1
    description: "双后端字段、保留期、system schema 和元数据零泄露"
    requirement: AUDIT-01
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/conformance/mod.rs#audit_batch_retention_parity"
        status: pass
    human_judgment: false
  - id: D2
    description: "双后端批次/清理失败关门及 PG 连接中断恢复"
    requirement: AUDIT-01
    verification:
      - kind: integration
        ref: "crates/ontoforge-storage/tests/conformance_pg.rs#audit_pg_connection_interruption_recovers"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-storage/tests/conformance/mod.rs#audit_batch_rejection_is_atomic"
        status: pass
    human_judgment: false
duration: 40min
completed: 2026-10-26
status: complete
---

# Phase 06 Plan 02: PostgreSQL Read Audit Summary

PostgreSQL now writes the same read-audit metadata as SQLite through the bounded worker, with transactionally coupled retention cleanup and fail-closed admission.

## Accomplishments

- PostgreSQL migration creates `of_read_audit` in `schema_sys` with a time index; it has no raw SQL or result columns and no append-only DELETE trigger.
- The existing worker handles both pool types with the same capacity, batch, pending-age and retry policy. PG stores timestamps as `timestamptz` and role/condition arrays as `jsonb`.
- An in-flight read whose sink becomes unhealthy now receives `AuditUnavailable` from `finish`; the guard releases its permit without exposing a result.
- Shared conformance checks batch values, 31/29-day retention boundary, system/edit schema separation, failed batch atomicity, cleanup failure and transient recovery. A PG-only test terminates the blocked audit connection by PID and confirms one durable row after retry.

## TDD Gate Compliance

- Task 1 RED `ab58435`: `audit_batch_retention_parity` ran against real embedded PostgreSQL and failed on `both backends must accept read audits` because the PG worker was disabled. The same test passed on SQLite. GREEN `9ee20cd` added the PG migration and sink; both backend targets passed.
- Task 2 RED `5477db9`: `audit_sink_failure_rejects_inflight_finish` ran on SQLite and embedded PG; both failed the assertion that an in-flight read must fail after sink health turns unavailable. GREEN `dff7b87` added the guard health check and covered batch/cleanup failure plus PG connection interruption. The RED failures were target assertions, not compilation or fixture failures.
- GSD `tdd-red-evidence` has no Rust libtest console adapter in this installation, so no machine `RED_EVIDENCE_OK` verdict is claimed. The actual failing cargo test outputs were inspected before implementation.

## Verification

- `cargo test -p ontoforge-storage --test conformance_sqlite --locked audit_ -- --list`: 4 discovered; `--nocapture`: 4 passed.
- `cargo test -p ontoforge-storage --features embedded-pg --test conformance_pg --locked audit_ -- --ignored --list`: 5 discovered; `--ignored --nocapture`: 5 passed on real embedded PostgreSQL.
- `cargo test -p ontoforge-query --test audit_pk --locked -- --nocapture`: 9 passed.
- `cargo clippy -p ontoforge-storage --features embedded-pg --all-targets --locked -- -D warnings`, `cargo fmt --all -- --check`, `git diff --check`: passed.

## Task Commits

1. Task 1 RED `ab58435` (`test(06-02): expose unavailable PostgreSQL read audit sink`)
2. Task 1 GREEN `9ee20cd` (`feat(06-02): persist read audits in PostgreSQL system schema`)
3. Task 2 RED `5477db9` (`test(06-02): expose in-flight audit release after sink failure`)
4. Task 2 GREEN `dff7b87` (`fix(06-02): fail closed for in-flight reads on audit sink faults`)

## Deviations from Plan

- The PG-only connection interruption test uses `pg_terminate_backend` on a worker PID proven to be waiting on the isolated test schema's audit table. This exercises real reconnect behavior without terminating other tests' connections.
- The Task 2 RED exposed a previously unspecified gap in `ReadAuditGuard::finish`: an already-reserved read could release a result after the worker reported unhealthy. The health check closes that gap.

## Limitations

- The audit queue acknowledges acceptance before durable persistence, consistent with `06-01`; sudden process or power loss was not simulated here.
- `AUDIT-01` remains pending for all public read entry points and the phase-level verification in later plans. This plan covers the storage sink and pk regression only.

## Self-Check: PASSED

The PostgreSQL migration, shared conformance and PG test files exist; all four task commits are ancestors of HEAD. The pre-plan HEAD `6f98b4d` yields four task commits, and the target SQLite/PG/query tests and static checks above passed.
