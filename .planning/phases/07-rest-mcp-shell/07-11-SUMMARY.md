---
phase: 07-rest-mcp-shell
plan: "11"
subsystem: api
tags: [openapi, typescript, openapi-fetch, bigint, json.rawJSON, contract-tests]

requires:
  - phase: 07-rest-mcp-shell
    provides: OpenAPI 3.1 REST snapshot, authenticated serve runtime, Action/history/result routes
provides:
  - TypeScript 7 client package with strict typecheck/build and openapi-fetch wrappers
  - Isolated OpenAPI generator workspace using openapi-typescript 7.13.0 with TypeScript 5.9.3
  - Lossless bigint JSON codec with native rawJSON/context.source capability gate
  - Real serve contract coverage for REST reads, writes, replay, auth, limits, and abort isolation
affects: [07-13, API-05, release]

actuals:
  tokens: 21148
  tasks: 2
  commits: 3

tech-stack:
  added: [openapi-typescript 7.13.0, openapi-fetch 0.17.0, typescript 7.0.2, generator typescript 5.9.3]
  patterns: [isolated generator workspace, bigint OpenAPI integer transform, parseAs text plus lossless decode, explicit command replay]

key-files:
  created:
    - tools/openapi-gen/package.json
    - tools/openapi-gen/package-lock.json
    - tools/openapi-gen/generate.mjs
    - clients/typescript/package.json
    - clients/typescript/package-lock.json
    - clients/typescript/src/generated.ts
    - clients/typescript/src/index.ts
    - clients/typescript/src/codec.ts
    - clients/typescript/tests/contract.test.mjs
  modified: []

key-decisions:
  - "主 client 保留 TypeScript 7.0.2；openapi-typescript 7.13.0 通过独立 TypeScript 5.9.3 generator workspace 运行。"
  - "整数 schema 映射为 bigint，响应先以 text 读取再用 native context.source 解码；缺少 rawJSON 或 context.source 时显式失败。"
  - "写入只由调用者显式提供 commandId 并显式 replay，client 不自动重试。"

patterns-established:
  - "每个 client 实例独立持有 bearer，调用级 AbortSignal 直接传递给 openapi-fetch。"
  - "生成漂移通过 generator workspace 重生成并比较 checked-in generated.ts。"

requirements-completed: [API-05]

coverage:
  - id: D1
    description: "OpenAPI 3.1 generated TypeScript types and callable openapi-fetch client"
    requirement: API-05
    verification:
      - kind: integration
        ref: "npm --prefix clients/typescript run check:generated"
        status: pass
      - kind: integration
        ref: "clients/typescript/tests/contract.test.mjs#ts_client_generated_metadata_pk_query_aggregate_health"
        status: pass
    human_judgment: false
  - id: D2
    description: "Lossless integer and decimal wire codec with runtime capability rejection"
    requirement: API-05
    verification:
      - kind: unit
        ref: "clients/typescript/tests/contract.test.mjs#ts_lossless_integer_decimal_boundary_roundtrips"
        status: pass
      - kind: unit
        ref: "clients/typescript/tests/contract.test.mjs#ts_client_auth_scope_stale_errors_limits_and_runtime_capability"
        status: pass
    human_judgment: false
  - id: D3
    description: "Real HTTP Action validation, explicit replay, history/result lookup, bearer isolation and abort"
    requirement: API-05
    verification:
      - kind: e2e
        ref: "npm --prefix clients/typescript run test:contract"
        status: pass
      - kind: integration
        ref: "clients/typescript/tests/contract.test.mjs#ts_client_action_validate_history_result_and_replay"
        status: pass
      - kind: integration
        ref: "clients/typescript/tests/contract.test.mjs#ts_client_typecheck_generation_and_instance_abort"
        status: pass
    human_judgment: false
---

# Phase 07 Plan 11 Summary

**OpenAPI-generated TypeScript REST client with bigint-preserving JSON codec and real `serve --dev` contract coverage**

## Performance

- **Duration:** approximately 1h 15m across the resumed execution lane
- **Started:** 2026-10-29T15:00:00Z (execution resumed after the approved generator decision)
- **Completed:** 2026-10-30T03:55:03Z
- **Tasks:** 2
- **Files modified:** 9 plan-owned files

## Accomplishments

- Added reproducible `tools/openapi-gen` workspace so `openapi-typescript@7.13.0` runs with compatible TypeScript 5.9.3 while the client package keeps TypeScript 7.0.2.
- Generated all OpenAPI integer fields as `bigint` and exposed typed metadata, pk, query, aggregate, health, Action, validation, history, and command-result methods through `openapi-fetch`.
- Added native lossless JSON encoding/decoding for integer tokens, preserving decimal strings and Unicode; unsafe numbers and runtimes without `JSON.rawJSON` or reviver `context.source` fail before silent rounding.
- Added real listener tests that start `ontoforge serve --dev` and execute all named API-05 client behaviors, including explicit command replay with one history entry, auth/scope denial, malformed/oversized bodies, bearer isolation, and AbortSignal cancellation.

## Task Commits

1. **Task 11-1: 交付生成 types 与可调用 TS 读取 client** - `70a9ed9`
2. **Task 11-2: 无损整数 codec 与全部写入/历史/错误合同** - `5c76afe`
3. **Dependency isolation fix: 主包移除 generator-only dependency** - pending

**Plan metadata:** pending parent execution lane metadata commit.

## Files Created/Modified

- `tools/openapi-gen/generate.mjs` - official OpenAPI generator API with integer-to-bigint transform.
- `tools/openapi-gen/package.json` / `package-lock.json` - isolated compatible generator dependencies.
- `clients/typescript/src/generated.ts` - checked-in OpenAPI 3.1 types.
- `clients/typescript/src/index.ts` - typed `openapi-fetch` wrapper, bearer isolation, typed HTTP errors, and abort support.
- `clients/typescript/src/codec.ts` - lossless bigint JSON wire codec and runtime capability gate.
- `clients/typescript/tests/contract.test.mjs` - real `serve --dev` contract and boundary tests.

## Decisions Made

方案 A was implemented as approved: TypeScript 7.0.2 remains the main package compiler, while a separate generator workspace resolves the bare `typescript` import to TypeScript 5.9.3 required by `openapi-typescript@7.13.0`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Resolved Node test cargo lookup on Windows**
- **Found during:** Task 11-1 contract verification
- **Issue:** npm-launched Node did not inherit the PowerShell cargo command lookup, so the test bootstrap could not build the Rust binary when needed.
- **Fix:** Reused an existing `target/debug/ontoforge.exe` when present and resolved `cargo.exe` from `%USERPROFILE%\\.cargo\\bin` as the fallback.
- **Files modified:** `clients/typescript/tests/contract.test.mjs`
- **Verification:** Real `serve --dev` process started and all 5 contract tests passed.
- **Committed in:** `70a9ed9`

---

**Total deviations:** 1 auto-fixed (Rule 3)
**Impact on plan:** No wire or API contract change; only Windows test process discovery was hardened.

## Issues Encountered

- The initial approved dependency pair was incompatible: `openapi-typescript@7.13.0` requires a TypeScript 5.x compiler API while TypeScript 7.0.2 exposes a different runtime API. The approved isolated generator workspace resolves this without changing the client compiler contract.
- The full Cargo workspace regression gate was not rerun in this lane because the parent Phase 7 executor owns the shared Rust regression gate. No Rust files were changed by this plan.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

07-13 can consume the client package and generator lockfiles. `npm ci` succeeded independently in both workspaces, followed by generation, typecheck, build, and the real HTTP contract suite.

## Self-Check: PASSED

- All 9 plan-owned files exist after the final commits.
- `70a9ed9` and `5c76afe` are ancestors of `HEAD`.
- `npm --prefix clients/typescript run typecheck`: passed.
- `npm --prefix clients/typescript run build`: passed.
- `npm --prefix clients/typescript run check:generated`: passed with no generated diff.
- `npm --prefix clients/typescript run test:contract`: 5 passed, 0 failed, 0 skipped.

---
*Phase: 07-rest-mcp-shell*
*Plan: 11*
*Completed: 2026-10-30*
