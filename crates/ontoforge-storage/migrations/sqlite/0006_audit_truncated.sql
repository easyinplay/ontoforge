-- Rebuild the standalone metadata table to widen its status constraint.
CREATE TABLE of_read_audit_new (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    at TEXT NOT NULL,
    request_id TEXT NOT NULL UNIQUE,
    principal TEXT NOT NULL,
    tool TEXT NOT NULL,
    object_type TEXT,
    condition_api_names TEXT NOT NULL,
    effective_roles TEXT NOT NULL,
    sql_sha256 TEXT NOT NULL,
    row_count INTEGER NOT NULL CHECK (row_count >= 0),
    elapsed_ms INTEGER NOT NULL CHECK (elapsed_ms >= 0),
    def_version INTEGER NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('success', 'truncated', 'denied', 'source_error', 'cancelled'))
);
INSERT INTO of_read_audit_new (
    seq, at, request_id, principal, tool, object_type, condition_api_names,
    effective_roles, sql_sha256, row_count, elapsed_ms, def_version, status
) SELECT seq, at, request_id, principal, tool, object_type, condition_api_names,
         effective_roles, sql_sha256, row_count, elapsed_ms, def_version, status
  FROM of_read_audit;
DROP TABLE of_read_audit;
ALTER TABLE of_read_audit_new RENAME TO of_read_audit;
CREATE INDEX of_ix_read_audit_at ON of_read_audit (at);
