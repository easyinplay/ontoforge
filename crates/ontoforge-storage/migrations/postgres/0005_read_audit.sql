-- Read audit metadata belongs to the system schema. Unlike admin audit, retention deletes rows.
CREATE TABLE of_read_audit (
    seq bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    at timestamptz NOT NULL,
    request_id text NOT NULL UNIQUE,
    principal text NOT NULL,
    tool text NOT NULL,
    object_type text,
    condition_api_names jsonb NOT NULL,
    effective_roles jsonb NOT NULL,
    sql_sha256 text NOT NULL,
    row_count bigint NOT NULL CHECK (row_count >= 0),
    elapsed_ms bigint NOT NULL CHECK (elapsed_ms >= 0),
    def_version bigint NOT NULL,
    status text NOT NULL CHECK (status IN ('success', 'denied', 'source_error', 'cancelled'))
);

CREATE INDEX of_ix_read_audit_at ON of_read_audit (at);
