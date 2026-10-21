---
status: testing
phase: 03-edit-store-scope-model
source: [03-VERIFICATION.md]
started: 2026-10-21T18:40:00Z
updated: 2026-10-21T18:40:00Z
---

## Current Test

number: 1
name: Cache-flush isolation on Trino (see behavior_unverified_items in 03-VERIFICATION.md)
expected: |
  reload_trino_it fails when system.flush_metadata_cache() is skipped and passes when it runs, on a catalog with metadata caching enabled
awaiting: user response

## Tests

### 1. Cache-flush isolation on Trino (see behavior_unverified_items in 03-VERIFICATION.md)
expected: reload_trino_it fails when system.flush_metadata_cache() is skipped and passes when it runs, on a catalog with metadata caching enabled
result: [pending]

### 2. Push the gap-closure commits (03-21..03-23 and the review) and let the ci workflow run on Linux
expected: All three jobs green; in particular the cfg(unix) assertion in crates/ontoforge-auth/tests/dev_token.rs#missing_key_directory_is_created (new key directory has mode 0700) runs and passes, and the Linux-only PostgreSQL/Trino jobs pass at the new HEAD
result: [pending]

## Summary

total: 2
passed: 0
issues: 0
pending: 2
skipped: 0
blocked: 0

## Gaps
