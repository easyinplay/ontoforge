# Phase 4: Scoped L0 Read - Pattern Map

**Mapped:** 2026-10-22
**Scope:** QUERY-01..10; file names below are candidate splits, not locked contracts.
**Analogs:** All cited source paths are git-tracked. `04-RESEARCH.md` did not exist at mapping time.

## File Classification

| Candidate file | Role | Data flow | Closest tracked analog | Match |
|---|---|---|---|---|
| `crates/ontoforge-query/src/ast.rs` | model | transform | `crates/ontoforge-engine/src/printer/ast.rs` | role match |
| `crates/ontoforge-query/src/compile.rs` | service | transform | `crates/ontoforge-auth/src/scope/lower.rs` | flow match |
| `crates/ontoforge-query/src/overlay.rs` | service | batch | `crates/ontoforge-storage/src/state.rs` | partial |
| `crates/ontoforge-query/src/scope.rs` | service | transform | `crates/ontoforge-auth/src/scope/merge.rs` | close |
| `crates/ontoforge-query/src/execute.rs` | service | request-response | `crates/ontoforge-engine/src/lib.rs` | partial |
| `crates/ontoforge-storage/src/query_read.rs` | service | CRUD | `crates/ontoforge-storage/src/commit.rs` | close |
| `crates/ontoforge-query/tests/model_prop.rs` and reference model | test | batch | `crates/ontoforge-storage/tests/model_prop.rs`, `tests/model/mod.rs` | close |
| `crates/ontoforge-query/tests/single_table.rs` | test | request-response | `crates/ontoforge-engine/tests/sqlite_engine.rs`, `tests/printer_prop.rs` | close |
| `crates/ontoforge-query/Cargo.toml`, `src/lib.rs` | config | transform | existing same files | exact |

These paths are a planner-ready layout, not a requirement to create every file. Group related operations when that reduces complexity.

## Pattern Assignments

### Query AST and compiler

**Copy shape from** `crates/ontoforge-engine/src/printer/ast.rs:12-23,32-65,100-115`: typed `Select`, `Expr`, `AggregateFunc`, and `OrderItem`. Keep public `ObjectSet`/`WhereClause` in logical property names. Lower to the existing physical SQL AST at the last boundary; do not accept pre-rendered SQL. `printer::print_select` is the only SQL text constructor (`crates/ontoforge-engine/src/printer/mod.rs:20-24,48-49`). `Select.from: Option<TableRef>` supports one table, but the Phase 4 compiler must independently enforce one physical source table and the fragment ceiling.

**Copy predicate lowering style from** `crates/ontoforge-auth/src/scope/lower.rs:17-52,74-113`: recursive typed expression construction, literal conversion and explicit operator mapping. `crates/ontoforge-expr/src/rowpred/ir.rs:16-102` is the closest logical predicate IR; its `Pred` is the scope CEL representation, not a full replacement for the client `WhereClause` operators.

### Scope integration

**Reuse directly** `CompiledScope::for_principal` at `crates/ontoforge-auth/src/scope/compile.rs:445-484`, then `TypeScope::evaluate` and `TypeScope::lower` at `crates/ontoforge-auth/src/scope/merge.rs:101-137`. The merge implementation at `merge.rs:60-80` combines role rows with OR and column grants per row. `TypeScope::evaluate` applies the same Rust `ontoforge_expr::eval` used by scope tests; `lower` supplies printer expressions for source pushdown. The query layer must validate requested filter/order/group/aggregate columns against grants before execution, AND the row predicate with the user predicate, and mask only at final projection. No existing function performs that complete query authorization.

**Error precedent:** `crates/ontoforge-storage/src/history.rs:79-90,100-139` coalesces absent, unauthorized and hidden objects to `NotFound` before reading history; its `HistoryRequest` at lines 30-43 passes scope, attrs and current row explicitly. Use the same non-disclosure behavior for L0 pk reads and sanitize outward errors of physical names.

### Edit read and overlay

**Copy physical mapping from** `crates/ontoforge-storage/src/physical.rs:41-55,96-125`: `ObjectTable` exposes pk, value and `__set` columns; `LinkTable` exposes M:N endpoint keys. **Copy decode/query construction from** `crates/ontoforge-storage/src/commit.rs:771-873` (`load_row`, `load_object`, `decode_object`, `read_expr` at line 1043). These are crate-private and transaction-oriented today. Add a public, bounded read API in storage rather than reaching into private SQL helpers from query. `EditStore::active_definition` at `store.rs:238` provides the loaded ontology and physical schema; `EditStore::load_scope` at `scope_store.rs:420` is the persisted scope entry.

**Overlay state precedent:** `crates/ontoforge-storage/src/state.rs:33-49,51-158` defines `EditRow`/`LinkRow`, tombstone and release transitions. The query overlay must use `__set` flags per property, suppress tombstones, expose source after release, and distinguish edit-only objects. There is no existing Rust source-plus-edit list merge, keyset refill, decomposed aggregation, or searchAround implementation to copy. Follow `04-CONTEXT.md` D-02 through D-06 and verify against an independent reference model.

### Trino / SQLite execution

**Reuse** `Engine`, `QueryContext`, `QueryOutput`, and `AnyEngine` at `crates/ontoforge-engine/src/lib.rs:51-132`: `QueryContext` has timeout, row and byte caps; `QueryOutput.truncated` reports engine truncation. `print_select(&select, engine.dialect())` at `printer/mod.rs:48-49` and `engine.execute(&sql, &ctx)` keep both backends on one AST. `crates/ontoforge-engine/src/sqlite/mod.rs:450-525` is the SQLite execution analogue. `crates/ontoforge-engine/src/printer/ast.rs:14-23` supports projection, filter, grouping, ordering and limit. Current `AggregateFunc` at lines 100-107 omits distinct, so distinct needs an explicit planned implementation or cost error. The L0 page/byte cap and `truncated` must also account for post-overlay results, independently of `QueryOutput.truncated`.

### Tests and crate wiring

**Reference model:** `crates/ontoforge-storage/tests/model/mod.rs:1-15,42-75,159` holds deliberately independent semantics; `crates/ontoforge-storage/tests/model_prop.rs:424-450,478-485` runs each generated step against model and real SQLite store. Apply the same test shape to source/edit overlay, tombstone/release, refill, ordering, set operations and aggregation.

**Differential corpus:** `crates/ontoforge-auth/tests/rowpred_diff.rs:102-145` compares Rust predicate evaluation to SQLite on generated and fixed cases. `crates/ontoforge-engine/tests/printer_prop.rs:202-245` and `tests/sqlite_engine.rs:9-55` show typed `Select` execution through SQLite; add fixed Trino single-table cases only where a Trino fixture is available. `crates/ontoforge-query/src/lib.rs:1-13` is a placeholder, and `Cargo.toml:1-12` still describes sea-query compilation. Update crate metadata/dependencies when the module boundary is set; ADR-0020 and the existing engine printer govern the implementation.

## Shared Patterns

| Concern | Exact source | Phase 4 use |
|---|---|---|
| Logical-to-physical boundary | `crates/ontoforge-engine/src/printer/ast.rs:12-23`; `printer/mod.rs:48-49` | Build typed single-table `Select`, then print by dialect. |
| Row/column authorization | `crates/ontoforge-auth/src/scope/merge.rs:60-80,101-137` | Merge roles once, evaluate merged rows in Rust, lower scope for source SQL, mask final projection. |
| Predicate three-valued behavior | `crates/ontoforge-expr/src/rowpred/eval.rs:14-76` | Use `Tri::is_true()` for visibility; compare source and overlay behavior. |
| Physical edit decoding | `crates/ontoforge-storage/src/commit.rs:787-873` | Preserve `__set`, deleted/released and typed pk semantics in a bounded public read API. |
| Read disclosure | `crates/ontoforge-storage/src/history.rs:79-90,100-139` | Collapse hidden and missing pk outcomes; avoid physical names in outward errors. |
| Result bounds | `crates/ontoforge-engine/src/lib.rs:25-35,51-85` | Enforce rows/bytes at engine and final L0 result; propagate truncation. |

## No Close Analog

| Behavior | Reason |
|---|---|
| `ObjectSet` wire AST and complete `WhereClause` | The printer AST is physical SQL; scope `Pred` is restricted CEL. |
| Rust source/edit list overlay with refill and keyset cursor | Storage has edit state transitions, but no merged query path. |
| Decomposed aggregates, distinct cost decision, link traversal | No existing query service covers these operations. |
| HMAC `_version` token | No verified query-version implementation in the inspected crates. |

## Metadata

**Search scope:** `crates/ontoforge-query`, `ontoforge-engine`, `ontoforge-storage`, `ontoforge-auth`, `ontoforge-expr`; Phase 4 context, roadmap and requirements.
**Tracked-source gate:** `git ls-files` checked cited primary analog paths.
**Scope limit:** This is a pattern map, not an implementation or a copy of ADRs.
