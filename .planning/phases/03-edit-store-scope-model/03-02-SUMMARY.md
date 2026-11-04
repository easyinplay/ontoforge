---
phase: 03-edit-store-scope-model
plan: 02
subsystem: schema-config
tags: [serde, toml, json-schema, diagnostics, oidc, cel, principal-attrs]

requires:
  - phase: 03-edit-store-scope-model
    provides: "03-01 workspace dependencies and crate graph (no manifest edits needed here)"
provides:
  - "PropertyDef.indexed (YAML key indexed, default false) in the IR and in schemas/ontoforge-definition.schema.json"
  - "Diagnostic codes ONT-V21..V27, allocated once, V-level and not suppressible by ontoforge.toml"
  - "ontoforge_schema::ir::AttrType (string, integer, boolean, string_list, integer_list)"
  - "ontoforge_expr::rowpred::value: Scalar, AttrValue, AttrValueError, Attrs (missing attribute reads as Null)"
  - "ontoforge-server.toml [store] and [auth] sections with ONT-V20 validation and line:col spans"
affects: [03-03, 03-04, 03-05, 03-07, 03-10, phase-04-query]

plan_head_before: c8b2b1327957bd56b7267ac45d991883cf142620

actuals:
  tokens: 11041
  tasks: 2
  commits: 4

tech-stack:
  added: []
  patterns:
    - "Config errors point at the offending value (Checker::value_span with key and array-index path), not only at the key"
    - "Plaintext connection strings are rejected through deny_unknown_fields plus a dedicated help text, same as password/token"
    - "Phase 3 diagnostics are V-level so no lint configuration can silence scope, row-predicate or destructive-change failures"

key-files:
  created:
    - crates/ontoforge-schema/src/ir/attrs.rs
    - crates/ontoforge-expr/src/rowpred/mod.rs
    - crates/ontoforge-expr/src/rowpred/value.rs
  modified:
    - crates/ontoforge-schema/src/ir/object.rs
    - crates/ontoforge-schema/src/ir/mod.rs
    - crates/ontoforge-schema/src/diag.rs
    - crates/ontoforge-schema/tests/object_types.rs
    - schemas/ontoforge-definition.schema.json
    - examples/ontologies/aviation/objects.yaml
    - examples/ontologies/aviation-json/objects.json
    - crates/ontoforge-expr/src/lib.rs
    - crates/ontoforge-engine/src/config.rs
    - crates/ontoforge-engine/tests/server_config.rs

key-decisions:
  - "ONT-V21..V27 are V-level rather than the RESEARCH draft E17..E19: E rules can be disabled in ontoforge.toml, but SCOPE-06, D-07 and STORE-02 require failures that cannot be turned off"
  - "OIDC loopback exemption: http is accepted for localhost and loopback IPs only; issuer containing '|' is rejected so 03-07 can namespace groups as {iss}|{group}"
  - "A PostgreSQL connection string has no plaintext key: url is an unknown field and gets a help text naming url_env / url_file"

patterns-established:
  - "Checker::value_span(path) with Seg::Key / Seg::Idx for errors inside [[array.of.tables]]"

# STORE-02, SCOPE-02 and AUTH-01 are addressed (definition-layer and configuration-layer inputs only).
# None is fully delivered: DDL generation and migrate (STORE-02), role documents and row predicates
# (SCOPE-02), JWKS fetch and token verification (AUTH-01) land in later plans.
# REQUIREMENTS.md is left unticked on purpose (policy decided in 03-01).
requirements-completed: []
requirements-addressed: [STORE-02, SCOPE-02, AUTH-01]

coverage:
  - id: D1
    description: "A property can declare indexed: true (default false); it reaches the IR and the exported definition JSON Schema"
    requirement: STORE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-schema/tests/object_types.rs#indexed_defaults_to_false"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-schema/tests/object_types.rs#indexed_true_is_kept"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-schema/tests/schema_export.rs (schemas/ byte comparison and example validation)"
        status: pass
    human_judgment: false
  - id: D2
    description: "indexed with a non-boolean value is ONT-V06 located at the value (line 7, column 36)"
    requirement: STORE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-schema/tests/object_types.rs#indexed_must_be_boolean"
        status: pass
    human_judgment: false
  - id: D3
    description: "The aviation example (YAML and JSON twins) declares Flight.status indexed and still validates with no diagnostics"
    requirement: STORE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-schema/tests/load_examples.rs#aviation_yaml_and_json_produce_identical_ir"
        status: pass
      - kind: other
        ref: "cargo run -q -p ontoforge --locked -- validate examples/ontologies/aviation --format json --color never -> []"
        status: pass
    human_judgment: false
  - id: D4
    description: "Diagnostic codes ONT-V21..V27 exist with the planned meanings"
    verification:
      - kind: unit
        ref: "crates/ontoforge-schema/src/diag.rs#phase3_codes"
        status: pass
    human_judgment: false
  - id: D5
    description: "ontoforge-server.toml [store]: sqlite or postgres, connection string only via url_env / url_file, schema names, timeouts and [store.trino] catalog validated with located ONT-V20 errors"
    requirement: STORE-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-engine/tests/server_config.rs#store_sqlite_defaults"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/server_config.rs#store_postgres_requires_exactly_one_url_source"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/server_config.rs#store_plaintext_url_is_rejected"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/server_config.rs#store_schema_names_are_validated"
        status: pass
    human_judgment: false
  - id: D6
    description: "ontoforge-server.toml [auth]: several [[auth.oidc]] issuers with defaults, asymmetric-only algorithm whitelist, https or loopback issuer / jwks_uri, unique issuer without '|', principal_attrs with declared types, [auth.dev]"
    requirement: AUTH-01
    verification:
      - kind: integration
        ref: "crates/ontoforge-engine/tests/server_config.rs#oidc_rejects_symmetric_and_none_algorithms"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/server_config.rs#oidc_issuer_rules"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/server_config.rs#oidc_two_issuers_with_defaults"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-engine/tests/server_config.rs#principal_attrs_types"
        status: pass
    human_judgment: false
  - id: D7
    description: "Principal attribute runtime values: a missing attribute reads as Null; a list holding Null is rejected"
    requirement: SCOPE-02
    verification:
      - kind: unit
        ref: "crates/ontoforge-expr/src/rowpred/value.rs#attrs_missing_is_null"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-expr/src/rowpred/value.rs#attrs_list_rejects_null_element"
        status: pass
    human_judgment: false

duration: 30min
completed: 2026-10-20
status: complete
---

# Phase 3 Plan 2: Definition and configuration prerequisites Summary

**Property `indexed` flag from YAML through IR, JSON Schema and `validate`; ONT-V21..V27 allocated as non-suppressible codes; `AttrType` plus `Attrs` runtime values for principal attributes; and `[store]` / `[auth]` sections in `ontoforge-server.toml` with located ONT-V20 validation (no plaintext connection string, asymmetric-only OIDC algorithms).**

## Performance

- **Duration:** about 30 min (start time was not recorded; estimated from the first RED commit at 01:48 and the last commit at 02:03, plus reading)
- **Completed:** 2026-10-20
- **Tasks:** 2 of 2 (Task 1 is a tracer)
- **Files modified:** 13 (3 created)

## Accomplishments

- `Flight.status` declares `indexed: true` in both example twins; `ontoforge validate examples/ontologies/aviation` prints `[]`; `schemas/ontoforge-definition.schema.json` carries `indexed`.
- Seven diagnostic codes exist so later plans (03-05 V22, 03-06 V21 / V23, 03-08 V24, 03-12 V25, V26, 03-14 V27) do not touch `diag.rs` again.
- `ServerConfig` gained `store` and `auth`; every check reports ONT-V20 with a span on the offending value, including inside `[[auth.oidc]]` entries via a new array-aware `Checker::value_span`.
- `AttrType` lives in `ontoforge_schema::ir::attrs`; `Scalar` / `AttrValue` / `Attrs` live in `ontoforge_expr::rowpred::value`, so 03-05 and 03-07 share them without depending on each other.

## Task Commits

1. **Task 1: Tracer, `indexed` end to end and ONT-V21..V27** (TDD)
   - RED: `17a7f43` (test)
   - GREEN: `87609b9` (feat)
2. **Task 2: AttrType, Attrs, `[store]` and `[auth]`** (TDD)
   - RED: `588a089` (test)
   - GREEN: `9b673cc` (feat)

**Plan metadata:** committed separately after this file (docs commit).

Tracer gate: auto mode off, `human_verify_mode` is `end-of-phase` and the tracer `<verify>` is automated only, so the verify was re-run (`cargo test -p ontoforge-schema --locked` and `validate`), passed, and expansion continued.

## TDD evidence

- **RED, Task 1 (`17a7f43`):** the three `indexed_*` integration tests and `phase3_codes` do not compile (`no field indexed on type PropertyDef`, missing `codes::V21`). This is compile-level RED, which is how a missing field surfaces in Rust. Behavioral RED for the one case that can be observed without the field, `indexed: "yes"`: `ontoforge validate` on a scratch ontology returned `ONT-V04` at 6:27 (unknown field) where the test expects `ONT-V06` at the value. Not a strict `check tdd-red-evidence` assertion-level RED; recorded here instead.
- **RED, Task 2 (`588a089`):** `server_config` fails to compile (`unresolved imports ... StoreConfig, StoreKind, AuthConfig`, `ontoforge_schema::ir::AttrType`; `no field store on ServerConfig`). Same compile-level caveat. Several rejection tests would pass trivially against `deny_unknown_fields` (an unknown `[store]` is already an error), which is why they additionally assert message content and the span text.
- **GREEN:** `87609b9` and `9b673cc`; all 32 `server_config` tests and the 2 `value.rs` tests pass.
- **Gate check:** `test(03-02)` precedes `feat(03-02)` for both tasks.

## Verification

- `cargo test --workspace --locked`: exit 0, 57 test binaries `ok`, no failures.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: clean. `cargo fmt --all --check`: clean.
- `cargo test -p ontoforge-schema --locked indexed_must_be_boolean`: `1 passed`.
- `cargo test -p ontoforge-engine --locked --test server_config oidc_rejects_symmetric_and_none_algorithms` and `store_postgres_requires_exactly_one_url_source`: each `1 passed`.
- No `.snap.new` files and no IR snapshot changes (the schema crate has no IR snapshot for objects; the snapshot criterion holds vacuously).
- Acceptance quirk: `rg "pub enum Scalar|pub enum AttrValue|pub struct Attrs"` prints 4 lines, not 3, because `pub enum AttrValueError` also matches the `AttrValue` prefix. All three named types exist.

## Decisions Made

- V-level for ONT-V21..V27 (see key-decisions); rationale recorded in the `codes` module doc.
- Config errors point at the value, not the key, for new sections. Existing `[trino]` checks keep key spans.
- `oidc` algorithm errors are one per offending entry, each naming the entry (algorithm names are not secrets).
- `url_file`, `key_file`, `ca_file` stay raw strings in the config types; consumers join them with the configuration directory (same as the existing `ca_file`), so this plan adds no file I/O.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Mirrored `indexed: true` into the JSON twin of the aviation example**
- **Found during:** Task 1 (GREEN)
- **Issue:** `aviation_yaml_and_json_produce_identical_ir` failed because only `examples/ontologies/aviation/objects.yaml` had the new flag; the plan's file list omitted `examples/ontologies/aviation-json/objects.json`.
- **Fix:** Added `"indexed": true` to `Flight.status` in the JSON twin.
- **Files modified:** `examples/ontologies/aviation-json/objects.json`
- **Verification:** `load_examples` suite passes (6 tests).
- **Committed in:** `87609b9`

**2. [Rule 1 - Bug] Test column miscounted**
- **Found during:** Task 1 (GREEN)
- **Issue:** My `indexed_must_be_boolean` expected column 35; the diagnostic correctly points at `"yes"` on line 7, column 36 (span length 5).
- **Fix:** Corrected the test to `(7, 36)`.
- **Files modified:** `crates/ontoforge-schema/tests/object_types.rs`
- **Committed in:** `87609b9`

**3. [Rule 1 - Bug] clippy `doc_markdown` on the new V21 doc comment**
- **Found during:** Task 1
- **Fix:** Backticked `Role` / `RoleBinding` in `diag.rs`.
- **Committed in:** `87609b9`

---

**Total deviations:** 3 auto-fixed (1 blocking, 2 bugs)
**Impact on plan:** No scope change. The JSON twin edit is required by an existing parity test.

## Issues Encountered

- A large inline shell heredoc mixing several files failed to parse twice (`unexpected EOF`); nothing ran. I switched to the Write and Edit tools. No effect on the repository.
- The plan commit ledger from the executor protocol (`gsd-plan-head-before-03-02`) was not created before the first commit; `plan_head_before` above is the `HEAD` recorded at the start of this run (`c8b2b13`), and `git rev-list --count c8b2b13..HEAD` is 4, matching `actuals.commits`.

## Known Stubs

None. `rowpred/mod.rs` states that parsing and evaluation arrive in 03-05; that is scope, not a placeholder behind a goal.

## Threat Flags

None. Mitigations T-03-04 (no plaintext `url`, test `store_plaintext_url_is_rejected`), T-03-05 (asymmetric whitelist, https or loopback only) and T-03-06 (unique issuers without `|`) are implemented as planned. `OidcConfig.ca_file` and `DevAuthConfig.key_file` are new file references that are only parsed here; the code that reads them (03-07) must apply the existing size and symlink rules.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- 03-05 can build `Pred` on `rowpred::value` and `AttrType`; 03-07 can read `AuthConfig` and the namespaced-group assumption (issuer without `|`).
- 03-03 / 03-04 can read `StoreConfig`; they must resolve `path`, `url_file` relative to the configuration directory and treat `url_env` through `resolve_secret`.
- Codes V21..V27 are ready; no later plan needs to edit `diag.rs` for them.

---
*Phase: 03-edit-store-scope-model*
*Completed: 2026-10-20*

## Self-Check: PASSED

- Files exist: `crates/ontoforge-schema/src/ir/attrs.rs`, `crates/ontoforge-expr/src/rowpred/mod.rs`, `crates/ontoforge-expr/src/rowpred/value.rs`, `.planning/phases/03-edit-store-scope-model/03-02-SUMMARY.md`
- Commits exist: `17a7f43`, `87609b9`, `588a089`, `9b673cc`
- `git rev-list --count c8b2b13..HEAD` = 4 (matches `actuals.commits`)
