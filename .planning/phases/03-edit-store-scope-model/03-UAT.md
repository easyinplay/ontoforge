---
status: complete
phase: 03-edit-store-scope-model
source: [03-VERIFICATION.md]
started: 2026-10-21T18:40:00Z
updated: 2026-10-22T02:27:00Z
---

## Current Test

All 2 UAT tests passed; PR CI on commit 1df4a7f passed on attempt 2.

## Tests

### 1. Cache-flush isolation on Trino (see behavior_unverified_items in 03-VERIFICATION.md)
expected: reload_trino_it fails when system.flush_metadata_cache() is skipped and passes when it runs, on a catalog with metadata caching enabled
result: pass
note: "2026-10-22 on isolated Trino 483/PostgreSQL (localhost:58080/55432): the new cached_column_requires_flush_before_definition_switch test failed as expected without catalog caching because NoFlush incorrectly allowed ReloadOutcome def_version Some(2). With metadata.cache-ttl=10m and metadata.cache-missing=true, the same test passed: after prewarming the v1 table, NoFlush left of_obj_flight.gate invisible and v1 active; the real flush made the column visible and activated v2. cargo test -p ontoforge-storage --test reload_trino_it --locked -- --ignored --test-threads=1 passed 3/3. cargo fmt --all --check, cargo clippy -p ontoforge-storage --test reload_trino_it --locked -- -D warnings, and git diff --check passed. PR CI run 39710470956 attempt 1 also ran the new case and passed reload_trino_it 3/3. The prior shared-cluster PERMISSION_DENIED remains documented in the previous revision."

### 2. Push the gap-closure commits (03-21..03-23 and the review) and let the ci workflow run on Linux
expected: All three jobs green; in particular the cfg(unix) assertion in crates/ontoforge-auth/tests/dev_token.rs#missing_key_directory_is_created (new key directory has mode 0700) runs and passes, and the Linux-only PostgreSQL/Trino jobs pass at the new HEAD
result: pass
note: "CI run 39710470956 at 1df4a7f: integration and test (ubuntu-latest) passed on attempt 1; test (windows-latest) failed in crates/ontoforge-storage/tests/reload.rs:1119 reload_racing_commits_never_lets_an_old_write_through with 'v2: Store(Query { sqlstate: Some(\"5\"), message: \"database is locked\" })'. Attempt 2 reran the failed Windows job and passed, leaving all three jobs green. The same existing SQLite race test failed on run 39582728729 attempt 1 and passed on attempt 2; record as a recurring flaky test for follow-up. The new Trino integration case passed in run 39710470956 attempt 1."

## Summary

total: 2
passed: 2
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps
