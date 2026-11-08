# API Coverage — Trino client REST protocol

> Full coverage by default. Opt-outs are explicit, reasoned decisions.
> Scope: the Trino client protocol (`/v1/statement`) that `ontoforge-engine` implements in Phase 2 (ADR-0017, spike 1). Plans: 02-04, 02-06, 02-10.

| capability | decision | reason |
|---|---|---|
| submit-statement (POST /v1/statement) | INTEGRATE | |
| poll-next-uri (GET nextUri) | INTEGRATE | |
| cancel-query (DELETE nextUri) | INTEGRATE | |
| retry-429-retry-after | INTEGRATE | |
| retry-502-503-504 | INTEGRATE | |
| header-user (X-Trino-User) | INTEGRATE | |
| header-source (X-Trino-Source) | INTEGRATE | |
| header-trace-token (X-Trino-Trace-Token) | INTEGRATE | |
| header-client-tags (X-Trino-Client-Tags) | INTEGRATE | |
| header-time-zone (X-Trino-Time-Zone) | INTEGRATE | |
| header-session (X-Trino-Session) | INTEGRATE | |
| auth-none | INTEGRATE | |
| auth-basic | INTEGRATE | |
| auth-bearer-jwt | INTEGRATE | |
| tls-custom-ca | INTEGRATE | |
| decode-typed-values (columns.type) | INTEGRATE | |
| query-error-mapping (error.errorName) | INTEGRATE | |
| server-info-probe (GET /v1/info) | OPT-OUT | not needed: Engine::status() probes with SELECT 1 through the statement protocol; CI readiness polling uses curl, not the client |
| auth-kerberos | OPT-OUT | explicitly out of scope: CONTEXT Deferred Ideas (on demand later) |
| impersonation (X-Trino-Original-User, per-principal X-Trino-User) | OPT-OUT | explicitly out of scope: CONTEXT Deferred Ideas, depends on JWT design |
| header-catalog-schema (X-Trino-Catalog / X-Trino-Schema) | OPT-OUT | not needed: printer always emits fully qualified table names |
| header-role (X-Trino-Role) | OPT-OUT | not needed yet: service identities rely on Trino-side grants; role selection belongs to Phase 6 L2 design |
| header-client-info | OPT-OUT | not needed: client tags already identify the source |
| header-client-capabilities | OPT-OUT | not needed: no capability-dependent response format is consumed |
| header-resource-estimate | OPT-OUT | not needed: resource groups are not a premise (ADR-0017, spike 3b) |
| header-extra-credential | OPT-OUT | not needed: connectors use their own catalog credentials |
| header-language | OPT-OUT | not needed: messages are not localized by Trino for ontoforge |
| session-state-headers (X-Trino-Set-* / Clear-Session / Set-Role) | OPT-OUT | explicitly out of scope: client is stateless by design (spike 1 point 8), responses are ignored |
| transactions (X-Trino-Transaction-Id / Started-Transaction-Id) | OPT-OUT | explicitly out of scope: writes go to PostgreSQL, never through Trino (spike 1 point 9) |
| prepared-statements (X-Trino-Prepared-Statement) | OPT-OUT | explicitly out of scope: SQL comes from the typed printer as text (ADR-0017) |
| spooling-protocol (X-Trino-Query-Data-Encoding, segments, ackUri) | OPT-OUT | not needed yet: results are bounded by max_rows / max_bytes (ADR-0021); re-evaluate for large exports (spike 1 section 6) |
| oauth2-interactive-flow | OPT-OUT | not needed: ontoforge uses service identities with pre-issued bearer tokens |
