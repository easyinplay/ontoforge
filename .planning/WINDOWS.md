---
schema_version: 1
open_count: 1
waived_count: 0
fixed_count: 2
total_count: 3
last_updated: 2026-10-16T20:41:53.199Z
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
  }
]
````
