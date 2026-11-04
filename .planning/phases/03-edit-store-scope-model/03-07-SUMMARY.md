---
phase: 03-edit-store-scope-model
plan: 07
subsystem: auth
tags: [oidc, jwt, jwks, jsonwebtoken, wiremock, rs256, es256, principal]

requires:
  - phase: 03-edit-store-scope-model
    provides: "03-01 jsonwebtoken on aws_lc_rs and the ontoforge-auth manifest; 03-02 AuthConfig / OidcConfig / PrincipalAttrConfig; 03-05 Attrs / AttrValue / Scalar and ADR-0025 rule 2 (no null list elements)"
provides:
  - "ontoforge_auth::{Authenticator::{from_config, authenticate}, AuthnError, Principal, PrincipalId}: bearer token to Principal, id '{iss}|{sub}', groups, typed attrs"
  - "ontoforge_auth::authn::jwks::{JwksCache, JwksPolicy, Clock, SystemClock}: own JWKS cache with TTL clamp, single flight, throttle, negative cache, stale-if-error, size and time limits"
  - "Algorithm routing by token header alg against a per-issuer allow-list; HS* and none have no OIDC route (03-12 adds the dev branch next to it)"
  - "Test-only RS256 and ES256 keys under crates/ontoforge-auth/tests/data with a README"
affects: [03-12, 03-14, 03-16, phase-07-server]

plan_head_before: e2be29ff940bc9668863512c81db6ea30c2ee9a7

actuals:
  tokens: 17400
  tasks: 2
  commits: 4

tech-stack:
  added: []
  patterns:
    - "Injectable Clock trait (std Instant) instead of tokio test-util, so cache timing tests need no Cargo.toml change"
    - "Single-flight fetch: RwLock state for the read path, tokio Mutex held across the fetch, state re-checked after acquiring it"
    - "Error texts are fixed strings plus configured names; no token, claim value, URL or key material"

key-files:
  created:
    - crates/ontoforge-auth/src/authn/mod.rs
    - crates/ontoforge-auth/src/authn/oidc.rs
    - crates/ontoforge-auth/src/authn/jwks.rs
    - crates/ontoforge-auth/src/authn/principal.rs
    - crates/ontoforge-auth/tests/oidc.rs
    - crates/ontoforge-auth/tests/data/README.md
    - crates/ontoforge-auth/tests/data/oidc_rs256.pem
    - crates/ontoforge-auth/tests/data/oidc_es256.pem
  modified:
    - crates/ontoforge-auth/src/lib.rs

key-decisions:
  - "A kid whose key has another key type than the token alg, or declares another alg, is UnknownKey (no usable key), not UnsupportedAlgorithm; this makes a cross-issuer forgery with a shared kid fail as UnknownKey as the plan expects"
  - "A claim whose JSON value is null is treated as not provided (attribute Null); a present claim of the wrong type, or a list with a null or foreign element, rejects the token (ADR-0025 rule 2, T-03-22)"
  - "An undecodable header, including alg 'none' and unknown algorithm names, is Malformed: jsonwebtoken cannot represent them, and the matrix test accepts Malformed or UnsupportedAlgorithm"
  - "Wrong iss maps to UnknownIssuer also with a single trusted issuer, so the error does not depend on how many issuers are configured"
  - "min_refresh applies to every fetch after the first, including TTL expiry and a failed cold start, so a failing identity provider is not hit per request"
  - "Redirects are disabled for the JWKS client (same as the Trino client); a redirecting key set endpoint is a configuration error"

patterns-established:
  - "Verify-only token header use: only alg and kid are read; jku, x5u and jwk are never touched, proven by a test that asserts the attacker endpoint got 0 requests"

# AUTH-01 is delivered at library level, which is what the plan's success criteria define; the server middleware that calls it arrives in Phase 7.
# SCOPE-01 is addressed only for 'groups can come from the OIDC groups claim'; user / group / role management is 03-14 / 03-16, so it stays unticked.
requirements-completed: [AUTH-01]
requirements-addressed: [SCOPE-01]

coverage:
  - id: D1
    description: "An RS256 or ES256 token from an OIDC issuer authenticates to a Principal with id '{iss}|{sub}', groups from the groups claim and typed attrs from [auth.principal_attrs]; an absent or null attr claim is Null"
    requirement: AUTH-01
    verification:
      - kind: integration
        ref: "crates/ontoforge-auth/tests/oidc.rs#rs256_token_becomes_principal"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/oidc.rs#es256_token_becomes_principal"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/oidc.rs#missing_attr_claim_is_absent"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/oidc.rs#null_attr_claim_is_absent"
        status: pass
    human_judgment: false
  - id: D2
    description: "JWKS is fetched from the configured jwks_uri once and reused; TTL follows Cache-Control max-age clamped to [60 s, 24 h]; a kid miss refetches once, later unknown kids are throttled and remembered; concurrent misses share one fetch"
    requirement: AUTH-01
    verification:
      - kind: integration
        ref: "crates/ontoforge-auth/tests/oidc.rs#jwks_is_fetched_once_for_repeated_tokens"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/oidc.rs#cache_control_max_age_is_clamped"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/oidc.rs#kid_miss_refetches_once_then_throttles"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/oidc.rs#concurrent_kid_miss_is_single_flight"
        status: pass
    human_judgment: false
  - id: D3
    description: "An identity provider outage keeps the old key set in use until jwks_max_stale_secs, then fails closed and recovers; a response over 1 MiB is rejected and not cached, exactly 1 MiB is accepted"
    requirement: AUTH-01
    verification:
      - kind: integration
        ref: "crates/ontoforge-auth/tests/oidc.rs#stale_keys_survive_idp_outage"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/oidc.rs#oversized_jwks_rejected"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/oidc.rs#jwks_of_exactly_one_mebibyte_is_accepted"
        status: pass
    human_judgment: false
  - id: D4
    description: "Token attacks are rejected without leaking token or claim text: alg none, HS256 with public key material as secret, algorithm outside the issuer allow-list, tampered payload, header jku/x5u/jwk never followed, missing kid, garbage, expiry beyond leeway, nbf in the future, wrong issuer, wrong audience, missing exp/aud/iss/sub"
    requirement: AUTH-01
    verification:
      - kind: integration
        ref: "crates/ontoforge-auth/tests/oidc.rs#alg_none_is_rejected"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/oidc.rs#hs256_with_the_public_key_as_secret_is_rejected"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/oidc.rs#header_key_urls_are_never_followed"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/oidc.rs#tampered_token_is_rejected"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/oidc.rs#algorithm_outside_the_issuer_allow_list_is_rejected"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/oidc.rs#missing_required_claims_are_rejected"
        status: pass
    human_judgment: false
  - id: D5
    description: "Attribute claims are mapped strictly: wrong scalar type, list with null or foreign element, non-array groups claim, float or oversized integer reject the token (ADR-0025 rule 2); empty list is a value"
    requirement: SCOPE-01
    verification:
      - kind: integration
        ref: "crates/ontoforge-auth/tests/oidc.rs#list_claim_with_null_or_foreign_element_is_rejected"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/oidc.rs#scalar_claim_of_the_wrong_type_is_rejected"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/oidc.rs#integer_and_boolean_attrs_are_typed_strictly"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/oidc.rs#groups_claim_must_be_an_array_of_strings"
        status: pass
    human_judgment: false
  - id: D6
    description: "With several trusted issuers groups are namespaced '{iss}|{group}', the unverified iss only selects the verifier, and a token claiming issuer B but signed with issuer A's key is rejected even when both use the same kid"
    requirement: SCOPE-01
    verification:
      - kind: integration
        ref: "crates/ontoforge-auth/tests/oidc.rs#two_issuers_namespace_groups"
        status: pass
      - kind: integration
        ref: "crates/ontoforge-auth/tests/oidc.rs#unknown_issuer_is_rejected_when_several_are_trusted"
        status: pass
    human_judgment: false

duration: 50min
completed: 2026-10-19
status: complete
---

# Phase 3 Plan 07: OIDC Authentication Summary

**OIDC JWT authentication for RS256/ES256/EdDSA issuers: alg-routed verification against a per-issuer allow-list, a self-written JWKS cache (TTL clamp, single flight, throttle, negative cache, stale-if-error, 1 MiB / 10 s limits) and claim mapping to a Principal with namespaced groups and strictly typed attributes, proven against a wiremock identity provider.**

## Performance

- **Duration:** about 50 min (start time was not recorded; estimated)
- **Completed:** 2026-10-19
- **Tasks:** 2 (a tracer and an expansion), each as a RED then GREEN commit
- **Files modified:** 9 (8 created, 1 modified)

## Accomplishments

- `Authenticator::authenticate` takes a bearer token to a `Principal` (`id = "{iss}|{sub}"`, `groups`, `attrs`). Only `alg` and `kid` are read from the header; the algorithm is checked against the issuer's allow-list before a `Validation` with exactly that one algorithm is built, and `exp`, `iss`, `aud`, `sub` are required, `nbf` validated, leeway from config.
- `JwksCache` implements the policy of ADR-0008 / RESEARCH Pattern 8 with an injectable `Clock`: Cache-Control TTL clamp, single-flight refetch, `min_refresh` throttle for every fetch after the first, negative cache for unknown kids (bounded to 256 entries), stale-if-error up to `max_stale`, 1 MiB body cap, 10 s total timeout, redirects off.
- Attribute mapping implements the 03-05 decision: a list claim with a null or non-matching element rejects the token; `Attrs::insert` stays as second line of defence.
- 31 tests in `tests/oidc.rs` (plus 3 unit tests in the crate), all running against local wiremock servers; no network, no Docker.

## Task Commits

1. **Task 1: tracer, wiremock JWKS to Principal** - `9df4453` (test, RED), `5c69385` (feat, GREEN)
2. **Task 2: JWKS cache policy and rejection matrix** - `82bf157` (test, RED), `0d1fbea` (feat, GREEN)

**Plan metadata:** the docs commit that carries this file (docs: complete plan).

Tracer gate: `human_verify_mode` is `end-of-phase` and the tracer verify is automated only, so it was re-run after the GREEN commit (3 passed) and expansion continued.

## Files Created/Modified

- `crates/ontoforge-auth/src/authn/mod.rs` - `Authenticator`, `AuthnError`, header routing and issuer selection
- `crates/ontoforge-auth/src/authn/oidc.rs` - per-issuer verifier, `from_config` (HTTP client, `ca_file`, policy), library error mapping
- `crates/ontoforge-auth/src/authn/jwks.rs` - `JwksCache`, `JwksPolicy`, `Clock`, key set parsing
- `crates/ontoforge-auth/src/authn/principal.rs` - `Principal`, `PrincipalId`, claims to groups / attrs
- `crates/ontoforge-auth/src/lib.rs` - module doc and re-exports
- `crates/ontoforge-auth/tests/oidc.rs` - end-to-end and rejection tests
- `crates/ontoforge-auth/tests/data/{README.md,oidc_rs256.pem,oidc_es256.pem}` - freshly generated test-only keys

## Decisions Made

See `key-decisions` above. The two that affect later plans: a `null` attribute claim reads as absent (Null, hence invisible under D-06) rather than a rejection, and `AuthnError::UnknownKey` also covers a kid whose key type or declared alg does not fit the token. 03-12 adds the HS256 dev branch at the spot in `Authenticator::authenticate` where symmetric algorithms are now refused.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] No tokio `test-util`, so no `tokio::time::pause()`**
- **Found during:** Task 2 design
- **Issue:** The plan allows either an injectable clock or `tokio::time::pause()`; the latter needs the `test-util` feature and Cargo.toml edits are out of bounds for this plan.
- **Fix:** `Clock` trait over `std::time::Instant` with a public `JwksCache::with_clock`; tests move a hand-driven clock.
- **Files modified:** `crates/ontoforge-auth/src/authn/jwks.rs`, `crates/ontoforge-auth/tests/oidc.rs`
- **Verification:** `cache_control_max_age_is_clamped`, `kid_miss_refetches_once_then_throttles`, `stale_keys_survive_idp_outage` pass
- **Committed in:** `0d1fbea`

**2. [Rule 1 - Bug] Own test bugs found by the RED run**
- **Found during:** Task 2 RED run with a temporary shim
- **Issue:** `assert_clean` flagged every error for the empty token (`contains("")`), and a family-mismatched key returned `UnsupportedAlgorithm` where the plan expects `InvalidSignature` or `UnknownKey` for the shared-kid cross-issuer forgery.
- **Fix:** skip the leak check for an empty token; make a key that does not fit the alg `UnknownKey`.
- **Files modified:** `crates/ontoforge-auth/tests/oidc.rs`, `crates/ontoforge-auth/src/authn/jwks.rs`
- **Committed in:** `82bf157`, `0d1fbea`

---

**Total deviations:** 2 auto-fixed (1 blocking, 1 bug)
**Impact on plan:** none on scope; both keep the plan's stated behavior.

## Issues Encountered

- A first attempt to patch the Rust source with an inline `python3` script produced no change on this host; the edit was redone with the editor tools. No effect on the result.
- The RED commit of Task 2 does not compile (the `Clock` import is missing), as the 03-06 RED commits did; the behavior-level RED was checked with an uncommitted shim (6 of 31 tests failed, 4 of them the cache policy tests).

## Known Stubs

None.

## Threat Flags

None. Mitigations T-03-18 to T-03-23 are covered by tests: algorithm confusion (`alg_none_is_rejected`, `hs256_with_the_public_key_as_secret_is_rejected`, allow-list test), header key URLs (`header_key_urls_are_never_followed`), claim and issuer checks plus cross-issuer keys (`wrong_*`, `two_issuers_namespace_groups`), kid flood (`kid_miss_refetches_once_then_throttles`, `concurrent_kid_miss_is_single_flight`), attribute escalation (list and scalar type tests), error leakage (`assert_clean` on every rejection).

## Edge Coverage

From the plan's flagged assumption: leeway use-up (exp 20 s in the past accepted, 40 s rejected with the 30 s default), JWKS of exactly 1 MiB accepted and 1 MiB + 1 byte rejected, `max-age` of 60 s and 24 h clamps, a second kid miss inside and outside the `min_refresh` window, and two issuers sharing the kid `k1` are tested. Not tested by design: clock drift beyond leeway, one kid with two key types in a key set (dropped as ambiguous, covered by `ambiguous_and_non_signature_keys_are_not_used` for the duplicate case), interoperability with Keycloak or Entra ID (Phase 7).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- 03-12 can add the dev HS256 branch in `Authenticator::authenticate` without touching the OIDC path.
- 03-14 / 03-16 consume `Principal { id, groups, attrs }`; the first-login user record (D-04) is a storage function in 03-16, `ontoforge-auth` does not depend on storage.
- The 1 MiB / 10 s / redirect-off JWKS client settings are fixed in `OidcVerifier::from_config`; only `jwks_min_refresh_secs` and `jwks_max_stale_secs` come from the config.

---
*Phase: 03-edit-store-scope-model*
*Completed: 2026-10-19*

## Self-Check: PASSED

- Created files exist: authn/{mod,oidc,jwks,principal}.rs, tests/oidc.rs, tests/data/{README.md,oidc_rs256.pem,oidc_es256.pem} (verified with `[ -f ]`)
- Commits `9df4453`, `5c69385`, `82bf157`, `0d1fbea` present in `git log`
- Acceptance criteria re-run: kid_miss, single_flight, stale, header_key_urls each `1 passed`; oidc target `31 passed` (>= 20); no `Validation::new(header.alg)`; `set_required_spec_claims` present in oidc.rs
- `cargo test --workspace --locked` exit 0 (63 test targets ok); `cargo clippy --workspace --all-targets --locked -- -D warnings` clean; `cargo fmt --all --check` clean; `cargo tree -i openssl` and `-i native-tls` match no package
