# API Coverage — Action webhook and source reads

> Full coverage by default. Opt-outs are explicit, reasoned decisions.
> Scope: Phase 5's outbound webhook HTTP contract and Action reads through the existing Trino Engine. The host supplies the webhook target; Action requests cannot choose one. Phase 2 owns the Trino client protocol matrix in `../02-source-binding-engine/COVERAGE.md`.

| capability | decision | reason |
|---|---|---|
| webhook-post-json-to-trusted-target | INTEGRATE | |
| webhook-event-id-header-and-stable-payload | INTEGRATE | |
| webhook-success-on-2xx | INTEGRATE | |
| webhook-timeout-and-non-2xx-retry | INTEGRATE | |
| webhook-lease-recovery-and-at-least-once-delivery | INTEGRATE | |
| webhook-redirect-following | OPT-OUT | explicitly rejected: a configured target must not redirect event delivery to another host |
| webhook-request-selected-url-or-headers | OPT-OUT | explicitly rejected: only trusted host configuration selects the target and the wire request cannot override it |
| webhook-response-body-processing | OPT-OUT | not needed: 2xx acknowledges the stored event; the worker does not consume a response document |
| webhook-exactly-once-delivery | OPT-OUT | not achievable across HTTP acknowledgement and database mark; receivers deduplicate using event_id (ACTION-11) |
| action-source-read-via-existing-trino-engine | INTEGRATE | |
| action-source-writeback-via-trino | OPT-OUT | explicitly out of scope: Action writes edit store in PostgreSQL, never the source through Trino |
| action-rest-and-mcp-entrypoints | OPT-OUT | explicitly deferred to Phase 7; Phase 5 exposes the internal Action service contract |
