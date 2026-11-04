---
phase: 07-rest-mcp-shell
reviewed: 2026-10-31T09:23:42Z
depth: standard
files_reviewed: 18
files_reviewed_list:
  - ci/query-services.sh
  - ci/server-services.sh
  - .github/workflows/ci.yml
  - clients/typescript/src/codec.ts
  - clients/typescript/src/index.ts
  - clients/typescript/package.json
  - crates/ontoforge-server/src/lib.rs
  - crates/ontoforge-server/src/context.rs
  - crates/ontoforge-server/src/state.rs
  - crates/ontoforge-server/src/lifecycle.rs
  - crates/ontoforge-server/src/rest.rs
  - crates/ontoforge-server/src/mcp.rs
  - crates/ontoforge-server/src/mcp_catalog.rs
  - crates/ontoforge-storage/src/read_audit.rs
  - crates/ontoforge-server/tests/postgres_http_it.rs
  - crates/ontoforge-server/tests/health_trino_it.rs
  - crates/ontoforge-server/tests/support/pg_real.rs
  - crates/ontoforge-server/tests/mcp_contract.rs
findings:
  critical: 3
  warning: 0
  info: 0
  total: 3
status: issues_found
---

# Phase 7: Code Review Report

**Reviewed:** 2026-10-31T09:23:42Z
**Depth:** standard
**Files Reviewed:** 18
**Status:** issues_found

## Narrative Findings (AI reviewer)

### CR-01: [BLOCKER] Unsupported JSON runtime can submit an Action before reporting failure

**File:** `D:/DataSpellProjects/ontoforge/clients/typescript/src/index.ts:69-75`

**Issue:** Client construction and the actual fetch boundary never call `assertLosslessRuntime`. `codec.ts:38-41` checks runtime support only when encoding a bigint, whereas `decodeWire` unconditionally checks support at `codec.ts:65-66`. An Action containing ordinary strings and safe numbers therefore reaches the server on a runtime without `JSON.rawJSON` or reviver `context.source`. After the server successfully commits, the success-response decoder throws `LosslessCodecError`. The caller sees failure for a committed write and may retry without preserving an idempotency key. This violates the accepted requirement to reject unsupported runtimes before sending requests.

**Fix:** Check both runtime capabilities during client construction and immediately before each fetch. Add a fetch counter regression test for an Action containing no bigint: disable each capability separately, assert `LosslessCodecError`, and assert zero fetch calls. Also test a client created before capability removal so the send-time check is covered.

**Evidence:** Direct source trace. A standalone runtime reproduction was not executed in this read-only lane because TypeScript dependencies were not installed when reviewed; the CI lane owns dependency setup.

### CR-02: [BLOCKER] Public raw client silently bypasses lossless integer decoding

**File:** `D:/DataSpellProjects/ontoforge/clients/typescript/src/index.ts:57`

**Issue:** The public `raw: Client<paths>` is returned unchanged at lines 94-95. The wrapper methods explicitly request text and pass it through `decodeWire`, but a normal `client.raw.GET(...)` uses openapi-fetch's default JSON parsing. An integer token such as `9007199254740993` is rounded to a JavaScript number while the generated response contract exposes bigint. This public API therefore loses data and lies about its runtime type. It also bypasses the capability checks used by `decodeWire`.

**Fix:** Remove the public raw escape hatch and retain the typed endpoint methods, or enforce lossless serialization and deserialization for every raw request and response. Add a fake-fetch regression returning an int64 value beyond the safe-integer boundary and assert the exact bigint value through every supported public endpoint path. The smallest reliable change for this unpublished API is to remove public `raw`.

**Evidence:** Direct source trace through `createClient`, public interface, returned object, and wrapper-specific `parseAs: 'text'` options. No network request was made by this review lane.

### CR-03: [BLOCKER] MCP core tools reject their own advertised argument schemas

**File:** `D:/DataSpellProjects/ontoforge/crates/ontoforge-server/src/mcp.rs:154-165`

**Issue:** `call_read` unconditionally requires a top-level `objectType` and looks it up before dispatching the tool. `mcp_catalog.rs:151` advertises `describe` with an empty object and `additionalProperties: false`; lines 117-118 advertise query, object_set, searchAround, and aggregate with only `request`. Consequently, valid calls such as `describe({})` and `query({request: ...})` fail with `objectType is required` before reaching their implementation. Supplying the extra field contradicts the advertised schema. Existing MCP contract tests inspect tool listings and denied calls, so they do not catch this failure.

**Fix:** Move object-type lookup into the get_object and history branches. Let describe use only the snapshot, and let query/aggregate derive the type from their converted request set. Add real MCP transport calls for describe, query, object_set, searchAround, and aggregate using exactly the advertised arguments, then assert successful structured results.

**Evidence:** Direct source trace from catalog schema through `call_tool` to `call_read`. The failure is unconditional for valid inputs and does not depend on external service state. A new runtime reproduction was not executed in this lane.

## Verification Limits

- Review is against the live working tree at the timestamp above. CI and Trino lanes are still changing shared files; later edits require re-review of affected findings.
- `trino_http_it.rs` and helper/contract/generator files were consulted as supporting context but are not included in the fully reviewed file list above.
- Real PostgreSQL/Trino integration, TypeScript contracts, and Linux CI were not executed by this lane. Their completion cannot be inferred from this report.
- No structural pre-pass or external reviewer evidence block was supplied. No source files were changed and no commit was created.

---

_Reviewer: the agent (gsd-code-reviewer)_
_Depth: standard_

## 后续修复指针（2026-10-31）

本报告保留最初审查结果。CR-01/CR-02由`4b0b18d`修复，独立复签见`07-13-SDK-RECHECK.md`；CR-03由`3f5b5db`修复，独立复签见`07-13-MCP-RECHECK.md`。交叉审查新增Rust receipt序号边界与history分页发现，分别由`9e6d858`、`7fa2401`完成真实RED/GREEN；history独立追加复签仍待结果。完整服务/workspace/Linux CI门禁以`../07-13-SUMMARY.md`为准，原三项修复不代表Phase总体验收通过。
