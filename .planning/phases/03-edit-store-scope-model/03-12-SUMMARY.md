---
phase: 03-edit-store-scope-model
plan: 12
subsystem: auth
tags: [dev-token, hs256, jsonwebtoken, getrandom, secrecy, clap, principal]

requires:
  - phase: 03-edit-store-scope-model
    provides: "03-07 Authenticator / AuthnError / Principal / principal_from_claims and the alg routing; 03-02 AuthConfig.dev and PrincipalAttrConfig; 03-04 CLI layout (cli.rs / lib.rs / reload.rs)"
provides:
  - "ontoforge_auth::authn::dev::{DEV_ISSUER, DevMode, DevKey, DevClaims, DevVerifier, issue_dev_token}: HS256 dev tokens, iss = aud = ontoforge-dev, principal id ontoforge-dev|<sub>"
  - "Authenticator::with_dev: the only way to get a dev route; from_config never builds a DevVerifier, even for a config that has [auth.dev]"
  - "ontoforge dev-token --server-config <FILE> --sub <SUB> [--groups] [--attr name=value]... [--ttl] [--key-file]: token on stdout, one line"
affects: [03-14, 03-16, phase-07-server]

plan_head_before: d4bbec7a39e9e980277c9df1d8b2414b963be521

actuals:
  tokens: 11700
  tasks: 2
  commits: 4

tech-stack:
  added: []
  patterns:
    - "Capability token (DevMode) instead of a boolean flag: a verifier that must not be switched on by configuration can only be built by code that holds a value only opt-in code constructs"
    - "Rejection tests carry a positive control (the same setup accepts a good token) so a broken fixture cannot make them pass"
    - "CLI validates every argument before it creates any file, so a rejected call leaves no key behind"

key-files:
  created:
    - crates/ontoforge-auth/src/authn/dev.rs
    - crates/ontoforge-auth/tests/dev_token.rs
    - crates/ontoforge/src/dev_token.rs
    - crates/ontoforge/tests/cli_dev_token.rs
  modified:
    - crates/ontoforge-auth/src/authn/mod.rs
    - crates/ontoforge-auth/src/authn/oidc.rs
    - crates/ontoforge/src/cli.rs
    - crates/ontoforge/src/lib.rs

key-decisions:
  - "Route by header alg only: HS256 goes to the dev verifier when one is attached, every other HMAC algorithm is UnsupportedAlgorithm, asymmetric tokens never reach the dev route; an HS256 token with another iss is UnknownIssuer and does not fall back to OIDC"
  - "--key-file is relative to the working directory (shell convention); a key_file from [auth.dev] is relative to the configuration file, as the config documents"
  - "An existing key file that is not exactly 32 bytes is a Config error and is left untouched; a failed write removes the half-written new file"
  - "The expiry shown on stderr is computed by the CLI next to the token's own exp, so the two can differ by up to one second"
  - "DevVerifier keeps its own copy of [auth.principal_attrs] (per the plan's interface) instead of borrowing the Authenticator's"

patterns-established:
  - "Dev tokens end in principal_from_claims, so ADR-0025 rule 2 (no null list elements) and the typed-attribute checks apply to them unchanged"

requirements-completed: [AUTH-02]

coverage:
  - id: D1
    description: "A developer generates a key and an HS256 token with ontoforge dev-token (groups from --groups, typed attributes from --attr by [auth.principal_attrs]); a dev verifier that the caller enabled authenticates it as principal ontoforge-dev|<sub> with those groups and attributes"
    requirement: AUTH-02
    verification:
      - kind: integration
        ref: "crates/ontoforge/tests/cli_dev_token.rs#issues_token_verifiable_by_dev_verifier"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_dev_token.rs#scalar_attrs_are_converted_by_declared_type"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/dev_token.rs#dev_token_round_trip"
        status: pass
    human_judgment: false
  - id: D2
    description: "The signing key is 32 OS random bytes created exclusively, reused when present, refused when its length is wrong, and 0600 on Unix"
    requirement: AUTH-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-auth/tests/dev_token.rs#key_file_is_created_once_and_reused"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/dev_token.rs#key_file_of_wrong_length_is_a_config_error"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_dev_token.rs#key_file_flag_wins_and_the_key_is_reused"
        status: pass
      - kind: unit
        ref: "crates/ontoforge-auth/tests/dev_token.rs#key_file_permissions_unix (cfg(unix); not run on this Windows host)"
        status: unknown
    human_judgment: true
    rationale: "The 0600 mode is only asserted by a cfg(unix) test that did not run on the Windows 11 development host; the Linux CI job is the first place it runs. The Windows ACL behavior is documented, not tested."
  - id: D3
    description: "Dev tokens are accepted only when the caller attached a verifier: a configuration with [auth.dev] alone rejects a validly signed dev token (T-03-35, the AUTH-02 prohibition)"
    requirement: AUTH-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-auth/tests/dev_token.rs#config_alone_never_enables_dev_tokens"
        status: pass
    human_judgment: false
  - id: D4
    description: "Algorithm routing and token limits: RS256 with iss ontoforge-dev, HS256 with another issuer or audience, HS512, wrong key, expired beyond the 30 s leeway, missing sub, and a list attribute with a null element are all rejected without leaking token or claim text; OIDC keeps working with dev enabled"
    requirement: AUTH-02
    verification:
      - kind: integration
        ref: "crates/ontoforge-auth/tests/dev_token.rs#rs256_token_with_dev_issuer_is_rejected"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/dev_token.rs#hs256_with_other_issuer_rejected"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/dev_token.rs#other_hmac_algorithms_are_not_accepted"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/dev_token.rs#expired_dev_token_rejected"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/dev_token.rs#list_attr_with_a_null_element_is_rejected"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/dev_token.rs#oidc_still_works_with_dev_enabled"
        status: pass
    human_judgment: false
  - id: D5
    description: "CLI argument limits: --ttl between 1m and 30d, undeclared or mistyped --attr refused, groups trimmed and deduplicated, no key source is exit 1, a rejected call creates no key file, the token never appears on stderr"
    requirement: AUTH-02
    verification:
      - kind: integration
        ref: "crates/ontoforge/tests/cli_dev_token.rs#ttl_bounds"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_dev_token.rs#rejected_arguments_leave_no_key_file_behind"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_dev_token.rs#undeclared_attr_is_rejected"
        status: pass
      - kind: integration
        ref: "crates/ontoforge/tests/cli_dev_token.rs#no_key_file_anywhere_fails"
        status: pass
    human_judgment: false

duration: 20min
completed: 2026-10-20
status: complete
---

# Phase 3 Plan 12: Dev Token Authentication Summary

**`ontoforge dev-token` mints HS256 tokens from a locally generated 32-byte key, and a `DevVerifier` that only opt-in code can construct maps them through the same claims-to-Principal path as OIDC; configuration alone never turns dev tokens on.**

## Performance

- **Duration:** ~20 min
- **Started:** 2026-10-20T04:00Z (approximate)
- **Completed:** 2026-10-20T04:20Z
- **Tasks:** 2 (tracer + auto, both TDD)
- **Files modified:** 8 (4 created, 4 modified)

## Accomplishments

- Tracer (Task 1): CLI creates the key (exclusive create, 32 bytes from `getrandom`), signs the token, and the library verifies it into `ontoforge-dev|alice` with groups and typed attributes, using the exact `principal_from_claims` that OIDC uses.
- `DevMode` is a capability value: `DevVerifier::new` requires it, `Authenticator::from_config` never builds a verifier, and `config_alone_never_enables_dev_tokens` proves a config with `[auth.dev]` rejects a valid dev token (AUTH-02 prohibition, T-03-35).
- Routing by header `alg` (T-03-36): HS256 reaches the dev verifier only when attached; HS384/HS512 and asymmetric-with-dev-issuer tokens are refused; no fallback between routes.
- CLI limits: `--ttl` 1m to 30d, `--attr` typed by `[auth.principal_attrs]` (later duplicate wins), groups trimmed and deduplicated, the token goes to stdout only, and arguments are validated before the key file is created.

## Task Commits

TDD, RED before GREEN for both tasks:

1. **Task 1 (tracer) RED:** `32fcc05` (test) - signatures with unimplemented bodies, CLI wired to a stub; the round trip, key reuse and CLI tests failed on the planned behavior
2. **Task 1 GREEN:** `2e2982b` (feat) - DevKey, issue_dev_token, DevVerifier, `Authenticator::with_dev`, `ontoforge dev-token`
3. **Task 2 RED:** `7e5c415` (test) - boundary and lock-in tests; 4 failed (`ttl_bounds`, `rejected_arguments_leave_no_key_file_behind`, reserved-claim override, empty subject)
4. **Task 2 GREEN:** `3c9a370` (feat) - ttl bounds and the two `issue_dev_token` guards

**Plan metadata:** committed with this SUMMARY (docs: complete plan)

## Files Created/Modified

- `crates/ontoforge-auth/src/authn/dev.rs` - `DevMode`, `DevKey`, `DevClaims`, `issue_dev_token`, `DevVerifier`, `DEV_ISSUER`
- `crates/ontoforge-auth/src/authn/mod.rs` - `pub mod dev`, `dev` field, `with_dev`, HS256 branch of the alg routing
- `crates/ontoforge-auth/src/authn/oidc.rs` - `map_error` is `pub(super)` so the dev route maps library errors identically
- `crates/ontoforge/src/dev_token.rs` - the subcommand
- `crates/ontoforge/src/cli.rs`, `crates/ontoforge/src/lib.rs` - `DevTokenArgs`, `Command::DevToken`, dispatch
- `crates/ontoforge-auth/tests/dev_token.rs` (15 tests), `crates/ontoforge/tests/cli_dev_token.rs` (11 tests)

## Decisions Made

See `key-decisions` above. The two worth knowing before Phase 7: `serve --dev` is the one place that should call `DevMode::enable_insecure()`, and it should pass `[auth.dev] key_file` to `DevKey::load_or_create`; and `--key-file` is relative to the working directory while the config's `key_file` is relative to the config file.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] `issue_dev_token` refuses reserved claim overrides and an empty subject**
- **Found during:** Task 2
- **Issue:** `DevClaims.attrs` was merged first and the fixed claims overwrote it, but a configured attribute claim named `nbf` (never set by the issuer) would delay the token, and an empty `sub` produced a token the verifier then rejects. A principal attribute mapped to `sub` or `exp` would silently lose its value.
- **Fix:** `issue_dev_token` returns `InvalidClaims` for an attribute claim in `iss, aud, sub, exp, iat, nbf, groups` and for an empty `sub`.
- **Files modified:** `crates/ontoforge-auth/src/authn/dev.rs`
- **Verification:** `issuing_refuses_a_claim_that_would_override_a_reserved_one`, `issuing_refuses_an_empty_subject`
- **Committed in:** `3c9a370`

---

**Total deviations:** 1 auto-fixed (1 missing critical)
**Impact on plan:** No scope change. Task 2's `--ttl` upper bound (and the 0 check) were deliberately implemented in the Task 2 GREEN commit rather than in the tracer so that `ttl_bounds` had a genuine RED, although the plan text lists them under Task 1's action; the end state is the same.

## Issues Encountered

- Most Task 2 lock-in tests (config alone, alg routing, issuer, audience, key, expiry, null list element, OIDC unaffected) already passed when their commit was made: the tracer's `Validation` settings and the alg routing give those behaviors. They are recorded as lock-in tests, each with a positive control, and are not claimed as RED.
- `key_file_permissions_unix` is `cfg(unix)` and was not executed on this Windows host.
- Concurrent first-time creation of the key file by two processes is not tested (the plan flags this as an assumption): the loser of `create_new` re-reads and can see a not-yet-written file as a wrong-length error.

## Known Stubs

None.

## Threat Flags

None. The only new surface is the CLI creating a key file, which T-03-37 in the plan's threat model already covers.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `Authenticator::with_dev` and `DevVerifier::new(DevMode::enable_insecure(), key, &attrs)` are ready for `serve --dev` in Phase 7.
- 03-14 / 03-16 can rely on a dev principal having the same shape as an OIDC one (groups, typed attrs, no null list elements).
- Keys are generated into temporary directories in tests and into the path the developer names; nothing generated is written into the repository.

## Self-Check: PASSED

- Files: `dev.rs`, `tests/dev_token.rs`, `src/dev_token.rs`, `tests/cli_dev_token.rs` exist.
- Commits: `32fcc05`, `2e2982b`, `7e5c415`, `3c9a370` found in `git log`.
- Acceptance criteria re-run: `issues_token_verifiable_by_dev_verifier` 1 passed; `config_alone_never_enables_dev_tokens` 1 passed; `rs256_token_with_dev_issuer_is_rejected` 1 passed; `DevToken(DevTokenArgs)` present in `cli.rs`; `create_new(true)` present in `dev.rs`; `DevVerifier::new` has no match in `authn/mod.rs`.
- Plan verification: `cargo test --workspace --locked` all ok (no failures); `cargo clippy --workspace --all-targets --locked -- -D warnings` clean; `cargo fmt --all --check` clean.

## TDD Gate Compliance

RED (`test(03-12)`) precedes GREEN (`feat(03-12)`) for both tasks. Task 1 RED tests failed on the planned behavior (`Config("not implemented")` / exit 1 with `error: not implemented`), not on compile errors. Task 2 RED contained four failing tests and a set of lock-in tests that passed on arrival, as noted above. No refactor commit.
