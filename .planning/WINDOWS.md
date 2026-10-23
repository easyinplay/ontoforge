---
schema_version: 1
open_count: 6
waived_count: 0
fixed_count: 4
total_count: 10
last_updated: 2026-10-23T13:36:20.358Z
---

# Broken Windows Ledger

> Cross-phase defect register. With `workflow.windows_enforce` enabled, `/gsd-ship` blocks while `open_count > 0`.
> Waive with `gsd-tools windows waive <id> "<reason>"` (reason required).
> Mark fixed with `gsd-tools windows fixed <id>`.

| id | phase | kind | file | line | description | status | reason | recorded_at | resolved_at |
|----|-------|------|------|------|-------------|--------|--------|-------------|-------------|
| 1 | 2 | stub | crates/ontoforge-engine/src/sqlite/mod.rs |  | SqliteEngine.describe_tables answers from an in-memory metadata map that nothing populates until plan 02-05 adds fixture loading (intentional) | fixed |  | 2026-10-16T19:03:31.155Z | 2026-10-16T20:41:31.074Z |
| 2 | 2 | deviation | crates/ontoforge-engine/src/printer/sqlite_ddl.rs |  | sqlite_ddl carries allow(dead_code) until plan 02-05 uses attach_memory/create_table/insert/query_only outside tests | fixed |  | 2026-10-16T19:03:31.601Z | 2026-10-16T20:41:31.547Z |
| 3 | 2 | deviation | crates/ontoforge/src/online.rs |  | ONT-V20 help text of validate --online names --server-config, a flag plan 02-09 adds; until then the flag does not exist | open |  | 2026-10-16T20:41:53.199Z |  |
| 4 | 02 | deviation | docs/source-binding.md |  | SQLite stand-in reports float properties as double, so fixture-mode --online flags float properties as ONT-V17 (float reads only real); fix is in sqlite/fixture.rs physical_of, outside 02-08 scope | fixed |  | 2026-10-16T21:49:10.933Z | 2026-10-16T21:55:10.377Z |
| 5 | 02 | unrun-verify | .github/workflows/ci.yml |  | integration job (PostgreSQL + Trino 483, trino_it / cli_trino_it --ignored) not yet run on GitHub; describe_types_golden and decode_golden are PROVISIONAL until the first run (E21) | open |  | 2026-10-17T01:06:08.093Z |  |
| 6 | 02 | unrun-verify | .github/workflows/nightly.yml |  | nightly Iceberg workflow (Trino 483/484, local-warehouse JDBC catalog, report-failure issue job) not yet run; needs a workflow_dispatch run (E21) | open |  | 2026-10-17T01:06:08.569Z |  |
| 7 | 05 | stub | crates/ontoforge-action/src/lib.rs | 186 | validate-only is explicitly refused by the edit-only tracer; planned in 05-02 | open |  | 2026-10-23T07:09:41.788Z |  |
| 8 | 05 | stub | crates/ontoforge-action/src/version.rs | 47 | source-backed token 在 05-03 无 source sampler，统一 VersionConflict；05-06 接入真实 source snapshot 与往返验收 | fixed |  | 2026-10-23T10:01:46.213Z | 2026-10-23T13:36:20.358Z |
| 9 | 05 | deviation | crates/ontoforge-action/tests/trino_action_it.rs |  | Task 2 真实集成断言首次有效运行即通过，缺少独立行为 RED。 | open |  | 2026-10-23T13:34:50.748Z |  |
| 10 | 05 | deviation | .planning/phases/05-online-action/deferred-items.md |  | 默认 feature 组合的 cargo check/clippy 因现有配置问题失败，已用 embedded-pg 验证计划代码。 | open |  | 2026-10-23T13:34:51.389Z |  |

````json
[
  {
    "id": 1,
    "kind": "stub",
    "phase": "2",
    "file": "crates/ontoforge-engine/src/sqlite/mod.rs",
    "line": null,
    "description": "SqliteEngine.describe_tables answers from an in-memory metadata map that nothing populates until plan 02-05 adds fixture loading (intentional)",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-10-16T19:03:31.155Z",
    "resolved_at": "2026-10-16T20:41:31.074Z"
  },
  {
    "id": 2,
    "kind": "deviation",
    "phase": "2",
    "file": "crates/ontoforge-engine/src/printer/sqlite_ddl.rs",
    "line": null,
    "description": "sqlite_ddl carries allow(dead_code) until plan 02-05 uses attach_memory/create_table/insert/query_only outside tests",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-10-16T19:03:31.601Z",
    "resolved_at": "2026-10-16T20:41:31.547Z"
  },
  {
    "id": 3,
    "kind": "deviation",
    "phase": "2",
    "file": "crates/ontoforge/src/online.rs",
    "line": null,
    "description": "ONT-V20 help text of validate --online names --server-config, a flag plan 02-09 adds; until then the flag does not exist",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-10-16T20:41:53.199Z",
    "resolved_at": null
  },
  {
    "id": 4,
    "kind": "deviation",
    "phase": "02",
    "file": "docs/source-binding.md",
    "line": null,
    "description": "SQLite stand-in reports float properties as double, so fixture-mode --online flags float properties as ONT-V17 (float reads only real); fix is in sqlite/fixture.rs physical_of, outside 02-08 scope",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-10-16T21:49:10.933Z",
    "resolved_at": "2026-10-16T21:55:10.377Z"
  },
  {
    "id": 5,
    "kind": "unrun-verify",
    "phase": "02",
    "file": ".github/workflows/ci.yml",
    "line": null,
    "description": "integration job (PostgreSQL + Trino 483, trino_it / cli_trino_it --ignored) not yet run on GitHub; describe_types_golden and decode_golden are PROVISIONAL until the first run (E21)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-10-17T01:06:08.093Z",
    "resolved_at": null
  },
  {
    "id": 6,
    "kind": "unrun-verify",
    "phase": "02",
    "file": ".github/workflows/nightly.yml",
    "line": null,
    "description": "nightly Iceberg workflow (Trino 483/484, local-warehouse JDBC catalog, report-failure issue job) not yet run; needs a workflow_dispatch run (E21)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-10-17T01:06:08.569Z",
    "resolved_at": null
  },
  {
    "id": 7,
    "kind": "stub",
    "phase": "05",
    "file": "crates/ontoforge-action/src/lib.rs",
    "line": 186,
    "description": "validate-only is explicitly refused by the edit-only tracer; planned in 05-02",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-10-23T07:09:41.788Z",
    "resolved_at": null,
    "milestone": null
  },
  {
    "id": 8,
    "kind": "stub",
    "phase": "05",
    "file": "crates/ontoforge-action/src/version.rs",
    "line": 47,
    "description": "source-backed token 在 05-03 无 source sampler，统一 VersionConflict；05-06 接入真实 source snapshot 与往返验收",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-10-23T10:01:46.213Z",
    "resolved_at": "2026-10-23T13:36:20.358Z",
    "milestone": null
  },
  {
    "id": 9,
    "kind": "deviation",
    "phase": "05",
    "file": "crates/ontoforge-action/tests/trino_action_it.rs",
    "line": null,
    "description": "Task 2 真实集成断言首次有效运行即通过，缺少独立行为 RED。",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-10-23T13:34:50.748Z",
    "resolved_at": null,
    "milestone": null
  },
  {
    "id": 10,
    "kind": "deviation",
    "phase": "05",
    "file": ".planning/phases/05-online-action/deferred-items.md",
    "line": null,
    "description": "默认 feature 组合的 cargo check/clippy 因现有配置问题失败，已用 embedded-pg 验证计划代码。",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-10-23T13:34:51.389Z",
    "resolved_at": null,
    "milestone": null
  }
]
````
