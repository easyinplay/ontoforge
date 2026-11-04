-- Metadata only: never store source SQL, predicate values or returned rows.
CREATE TABLE of_read_audit (
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
    status TEXT NOT NULL CHECK (status IN ('success', 'denied', 'source_error', 'cancelled'))
);

CREATE INDEX of_ix_read_audit_at ON of_read_audit (at);
