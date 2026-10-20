-- The edit log and the object history of the PostgreSQL edit store (D-10, D-11; format fixed in
-- the plan 03-08 checkpoint, recorded in ADR-0026 by plan 03-09). Append-only triggers follow in
-- 03-09. Runs with `search_path` set to the system schema, like 0001.
--
-- One log record per successful commit; one entry per edit; one history snapshot per entry.
-- JSON values are `jsonb` in the encoding of `PropertyValue::to_json`; `committed_at` is
-- `timestamptz` and is read back as `YYYY-MM-DDTHH:MM:SS.fffZ` (UTC).

CREATE TABLE of_edit_log (
    seq bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    event_id text NOT NULL UNIQUE,
    actor text NOT NULL,
    action text NOT NULL,
    committed_at timestamptz NOT NULL,
    def_version bigint NOT NULL,
    params jsonb NOT NULL
);

CREATE TABLE of_edit_log_entry (
    log_seq bigint NOT NULL REFERENCES of_edit_log (seq),
    ordinal integer NOT NULL,
    kind text NOT NULL CHECK (kind IN ('object', 'link')),
    type_name text NOT NULL,
    pk jsonb NOT NULL,
    op text NOT NULL CHECK (op IN ('set', 'unset', 'tombstone', 'release', 'link_add', 'link_remove')),
    payload jsonb NULL,
    version_after bigint NOT NULL,
    PRIMARY KEY (log_seq, ordinal)
);

CREATE TABLE of_edit_history (
    log_seq bigint NOT NULL,
    ordinal integer NOT NULL,
    kind text NOT NULL CHECK (kind IN ('object', 'link')),
    type_name text NOT NULL,
    pk jsonb NOT NULL,
    version bigint NOT NULL,
    snapshot jsonb NOT NULL,
    PRIMARY KEY (log_seq, ordinal)
);

CREATE INDEX of_ix_edit_history_object ON of_edit_history (type_name, pk, version);
