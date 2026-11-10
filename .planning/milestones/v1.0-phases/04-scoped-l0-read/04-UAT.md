---
status: complete
phase: 04-scoped-l0-read
source: [04-01-SUMMARY.md, 04-02-SUMMARY.md, 04-03-SUMMARY.md, 04-04-SUMMARY.md, 04-05-SUMMARY.md, 04-06-SUMMARY.md, 04-07-SUMMARY.md]
started: 2026-10-23T00:48:00Z
updated: 2026-10-23T01:32:35.385Z
---

## Current Test

[testing complete]

## Tests

### 1. Phase 4 端到端自动验收
expected: Phase 4 的五项 Success Criteria、22 项自动覆盖与代码 SHA c39a971 的真实后端 CI 结果通过。
result: pass
source: automated
evidence: `cargo test -p ontoforge-query --locked` exit 0; CI run 39878005620 四个 job 成功；137 份 distributed EXPLAIN 为 103 份单 fragment、34 份双 fragment；隔离 p95 为 101.01/456.53/287.81/0.76 ms，全部低于阈值；`04-VERIFICATION.md` 36/36。

### 2. scoped pk 读取、source/edit 叠加与无副作用 edit 查询
expected: scoped pk 读取、source/edit 叠加与无副作用 edit 查询
result: pass
source: automated
coverage_id: PK-READ

### 3. 探测错误同形、HMAC 绑定与最终结果限界
expected: 探测错误同形、HMAC 绑定与最终结果限界
result: pass
source: automated
coverage_id: PK-SECURITY

### 4. 13 WhereClause operator families, typed validation, null semantics and term matching
expected: 13 WhereClause operator families, typed validation, null semantics and term matching
result: pass
source: automated
coverage_id: QUERY-OPERATORS

### 5. one-table scoped candidates, column-grant masks, CASE printer and typed EXPLAIN
expected: one-table scoped candidates, column-grant masks, CASE printer and typed EXPLAIN
result: pass
source: automated
coverage_id: QUERY-SOURCE-SHAPE

### 6. Bounded ordered object batch and keyset scan, typed values, tombstone and release, version-zero filtering
expected: Bounded ordered object batch and keyset scan, typed values, tombstone and release, version-zero filtering
result: pass
source: automated
coverage_id: OBJECT-EDIT-READ

### 7. Bounded from/to M:N endpoint scans preserve removed edges and reject FK links by logical name
expected: Bounded from/to M:N endpoint scans preserve removed edges and reject FK links by logical name
result: pass
source: automated
coverage_id: LINK-EDIT-READ

### 8. SQLite read operations preserve row, log and definition counts and keep physical identifiers out of errors
expected: SQLite read operations preserve row, log and definition counts and keep physical identifiers out of errors
result: pass
source: automated
coverage_id: EDIT-READ-ISOLATION

### 9. Independent reference and scoped source/edit overlay with refill, tombstone, release and stable ordering
expected: Independent reference and scoped source/edit overlay with refill, tombstone, release and stable ordering
result: pass
source: automated
coverage_id: LIST-OVERLAY

### 10. Parallel first candidates, bounded keyset windows, 256-key backfill and eligible-row has_more
expected: Parallel first candidates, bounded keyset windows, 256-key backfill and eligible-row has_more
result: pass
source: automated
coverage_id: BOUNDED-PAGE

### 11. Context-bound HMAC cursor, shared work budget and final masked row/byte caps
expected: Context-bound HMAC cursor, shared work budget and final masked row/byte caps
result: pass
source: automated
coverage_id: CURSOR-AND-LIMITS

### 12. Merged outbound and predicate-based inbound FK traversal with per-row FK visibility and missing-grant equivalence
expected: Merged outbound and predicate-based inbound FK traversal with per-row FK visibility and missing-grant equivalence
result: pass
source: automated
coverage_id: FK-TRAVERSAL

### 13. Source and edit M:N edges, scoped targets, three-page cursor and 256+1 hydration
expected: Source and edit M:N edges, scoped targets, three-page cursor and 256+1 hydration
result: pass
source: automated
coverage_id: MN-TRAVERSAL

### 14. Same-type scoped operand algebra against an independent BTreeSet oracle
expected: Same-type scoped operand algebra against an independent BTreeSet oracle
result: pass
source: automated
coverage_id: SET-ALGEBRA

### 15. Fixed and property reference models for M:N edge overlay and all three set operators
expected: Fixed and property reference models for M:N edge overlay and all three set operators
result: pass
source: automated
coverage_id: INDEPENDENT-REFERENCES

### 16. Shared nested node and work budgets, same-shape denied targets, single-table source and normal final caps
expected: Shared nested node and work budgets, same-shape denied targets, single-table source and normal final caps
result: pass
source: automated
coverage_id: COMPOSED-BOUNDS

### 17. Scoped count, sum, avg, min, max and distinct across source and edits
expected: Scoped count, sum, avg, min, max and distinct across source and edits
result: pass
source: automated
coverage_id: EXACT-MEASURES

### 18. CSV full-materialization and 64-case old/new property reference
expected: CSV full-materialization and 64-case old/new property reference
result: pass
source: automated
coverage_id: INDEPENDENT-ORACLE

### 19. Inclusive group, bucket, distinct-value and distinct-memory limits with masked values and no partial response
expected: Inclusive group, bucket, distinct-value and distinct-memory limits with masked values and no partial response
result: pass
source: automated
coverage_id: BOUNDED-SECURE-RESULT

### 20. Typed single-table source partial and bounded TopN SQL shape
expected: Typed single-table source partial and bounded TopN SQL shape
result: pass
source: automated
coverage_id: TYPED-SOURCE-SHAPE

### 21. Fixed semantic and security corpus against real Trino 483 and PostgreSQL 17
expected: Fixed semantic and security corpus against real Trino 483 and PostgreSQL 17
result: pass
source: automated
coverage_id: REAL-BACKEND-CORPUS

### 22. Multi-role, replay and physical-error conformance on real backends
expected: Multi-role, replay and physical-error conformance on real backends
result: pass
source: automated
coverage_id: REAL-BACKEND-SECURITY

### 23. Four isolated p95 thresholds and complete distributed EXPLAIN plans
expected: Four isolated p95 thresholds and complete distributed EXPLAIN plans
result: pass
source: automated
coverage_id: ISOLATED-PERFORMANCE

## Summary

total: 23
passed: 23
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps

[none yet]
