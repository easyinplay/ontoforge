# API Coverage — OIDC token validation (JWKS) and Trino PostgreSQL connector procedures

> Full coverage by default. Opt-outs are explicit, reasoned decisions.
> Scope: the external surfaces Phase 3 integrates as a resource server: an OIDC IdP's JWKS endpoint and the JWT validation rules (RFC 7517 / 7519, ADR-0008; plans 03-02, 03-06, 03-10), and the Trino PostgreSQL connector procedure used by the reload visibility probe (plan 03-16). The detector's own signal ("Commit API") is internal; the matrix covers the surfaces that are genuinely external.

| capability | decision | reason |
|---|---|---|
| jwks-fetch (GET configured jwks_uri) | INTEGRATE | |
| jwks-cache-control-max-age (clamped 60 s .. 24 h) | INTEGRATE | |
| jwks-key-rotation (kid miss single-flight refetch, throttled, negative cache) | INTEGRATE | |
| jwks-stale-if-error (bounded by jwks_max_stale_secs) | INTEGRATE | |
| jwks-custom-ca (ca_file) | INTEGRATE | |
| jwt-alg-rs256 / rs384 / rs512 | INTEGRATE | |
| jwt-alg-ps256 / ps384 / ps512 | INTEGRATE | |
| jwt-alg-es256 / es384 | INTEGRATE | |
| jwt-alg-eddsa | INTEGRATE | |
| claim-iss / aud / exp / sub (required) | INTEGRATE | |
| claim-nbf (validated when present) | INTEGRATE | |
| claim-groups (configurable groups_claim) | INTEGRATE | |
| claim-to-principal-attrs mapping (typed, D-05) | INTEGRATE | |
| multiple-trusted-issuers (issuer-namespaced groups) | INTEGRATE | |
| oidc-discovery (.well-known/openid-configuration) | OPT-OUT | not needed: jwks_uri is configured explicitly, so the service never trusts a discovery document to choose key URLs |
| token-introspection (RFC 7662) | OPT-OUT | not needed: JWTs are verified locally against the JWKS (ADR-0008) |
| userinfo-endpoint | OPT-OUT | not needed: groups and attributes come only from token claims (D-02, D-05) |
| authorization-code / device / interactive login flows | OPT-OUT | explicitly out of scope: ontoforge is a resource server; clients obtain tokens from the IdP |
| refresh-tokens | OPT-OUT | explicitly out of scope: a client-side concern |
| jwe-encrypted-tokens | OPT-OUT | not needed yet: no deployment requirement; signed JWTs only |
| header-key-sources (jku / x5u / embedded jwk) | OPT-OUT | explicitly rejected for security: keys come only from the configured JWKS (T-03-19) |
| symmetric-algorithms-on-oidc-path (HS256 / HS384 / HS512) | OPT-OUT | explicitly rejected for security: algorithm confusion; HS256 is only the dev-token path behind DevMode (03-10) |
| token-revocation / back-channel-logout | OPT-OUT | not needed yet: short-lived tokens; revisit with the Phase 7 service if a deployment requires it |
| mtls-bound-tokens (RFC 8705) | OPT-OUT | not needed yet: no deployment requirement |
| trino-postgresql-procedure system.flush_metadata_cache() | INTEGRATE | |
| trino-metadata-describe (existing Engine::describe_tables) | INTEGRATE | |
| trino-postgresql-procedure system.execute and other procedures | OPT-OUT | explicitly out of scope: the edit store is written through PostgreSQL directly, never through Trino; L2 forbids CALL (ADR-0018) |
