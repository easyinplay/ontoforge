---
status: partial
phase: 03-edit-store-scope-model
source: [03-VERIFICATION.md]
started: 2026-10-21T18:40:00Z
updated: 2026-10-21T19:45:00Z
---

## Current Test

[testing paused — 1 items outstanding]

## Tests

### 1. Cache-flush isolation on Trino (see behavior_unverified_items in 03-VERIFICATION.md)
expected: reload_trino_it fails when system.flush_metadata_cache() is skipped and passes when it runs, on a catalog with metadata caching enabled
result: blocked
blocked_by: third-party
reason: "Run 2026-10-21 against the shared Trino 483 (https://192.168.11.91:8443, user root) with a temporary catalog editstore (postgresql, metadata.cache-ttl=10m, metadata.cache-missing=true) over a temporary PG database. reload_activates_once_trino_sees_the_new_tables passed. a_new_column_is_visible_when_its_definition_becomes_active failed with NotVisible { missing: [\"of_obj_flight.gate\", \"(flush_metadata_cache failed: query failed (PERMISSION_DENIED): Access Denied: Cannot execute procedure editstore.system.flush_metadata_cache)\"] }. This proves the no-flush half (with caching on and no effective flush, the new column stays invisible and reload refuses to activate). The with-flush half could not run because root lacks permission to execute the procedure on this cluster. Catalog, database and the local test-helper patch were removed afterwards."

### 2. Push the gap-closure commits (03-21..03-23 and the review) and let the ci workflow run on Linux
expected: All three jobs green; in particular the cfg(unix) assertion in crates/ontoforge-auth/tests/dev_token.rs#missing_key_directory_is_created (new key directory has mode 0700) runs and passes, and the Linux-only PostgreSQL/Trino jobs pass at the new HEAD
result: pass
note: "CI run 39582728729 at 342247b. Attempt 2: test (windows-latest), test (ubuntu-latest) and integration all success; missing_key_directory_is_created ran and passed on ubuntu-latest. Attempt 1 failed only on windows-latest in crates/ontoforge-storage/tests/reload.rs:1119 reload_racing_commits_never_lets_an_old_write_through with 'v2: Store(Query { sqlstate: Some(\"5\"), message: \"database is locked\" })'; the failed job passed on rerun. SQLite locking code (busy_timeout 5s) is unchanged since 4684ae7, so this is recorded as a flaky race test to watch, not a gap."

## Summary

total: 2
passed: 1
issues: 0
pending: 0
skipped: 0
blocked: 1

## Gaps
