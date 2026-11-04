-- The edit log and the object history of the SQLite edit store (D-10, D-11; format fixed in the
-- plan 03-08 checkpoint, recorded in ADR-0026 by plan 03-09). The three tables are append only (D-12).
--
-- One log record per successful commit; one entry per edit; one history snapshot per entry.
-- Values are stored as JSON text in the encoding of `PropertyValue::to_json`; a timestamp is
-- `YYYY-MM-DDTHH:MM:SS.fffZ` (UTC) everywhere in this file.

CREATE TABLE of_edit_log (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id TEXT NOT NULL UNIQUE,
    actor TEXT NOT NULL,
    action TEXT NOT NULL,
    committed_at TEXT NOT NULL,
    def_version INTEGER NOT NULL,
    params TEXT NOT NULL
);

CREATE TABLE of_edit_log_entry (
    log_seq INTEGER NOT NULL REFERENCES of_edit_log (seq),
    ordinal INTEGER NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('object', 'link')),
    type_name TEXT NOT NULL,
    pk TEXT NOT NULL,
    op TEXT NOT NULL CHECK (op IN ('set', 'unset', 'tombstone', 'release', 'link_add', 'link_remove')),
    payload TEXT NULL,
    version_after INTEGER NOT NULL,
    PRIMARY KEY (log_seq, ordinal)
);

CREATE TABLE of_edit_history (
    log_seq INTEGER NOT NULL,
    ordinal INTEGER NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('object', 'link')),
    type_name TEXT NOT NULL,
    pk TEXT NOT NULL,
    version INTEGER NOT NULL,
    snapshot TEXT NOT NULL,
    PRIMARY KEY (log_seq, ordinal)
);

CREATE INDEX of_ix_edit_history_object ON of_edit_history (type_name, pk, version);

-- Append only (D-12). A BEFORE trigger on UPDATE and on DELETE refuses every rewrite. `INSERT OR
-- REPLACE` and `REPLACE INTO` delete the old row first; that DELETE reaches the trigger because
-- every connection runs with `recursive_triggers = ON` (see `sqlite_connect_options`). An upsert
-- (`ON CONFLICT DO UPDATE`) is an UPDATE. The triggers are no boundary against the owner of the
-- database file (`DROP TRIGGER`); deployments also keep the application account from rewriting
-- history (ADR-0026).

CREATE TRIGGER of_edit_log_no_update BEFORE UPDATE ON of_edit_log
BEGIN
    SELECT RAISE(ABORT, 'table of_edit_log is append-only');
END;

CREATE TRIGGER of_edit_log_no_delete BEFORE DELETE ON of_edit_log
BEGIN
    SELECT RAISE(ABORT, 'table of_edit_log is append-only');
END;

CREATE TRIGGER of_edit_log_entry_no_update BEFORE UPDATE ON of_edit_log_entry
BEGIN
    SELECT RAISE(ABORT, 'table of_edit_log_entry is append-only');
END;

CREATE TRIGGER of_edit_log_entry_no_delete BEFORE DELETE ON of_edit_log_entry
BEGIN
    SELECT RAISE(ABORT, 'table of_edit_log_entry is append-only');
END;

CREATE TRIGGER of_edit_history_no_update BEFORE UPDATE ON of_edit_history
BEGIN
    SELECT RAISE(ABORT, 'table of_edit_history is append-only');
END;

CREATE TRIGGER of_edit_history_no_delete BEFORE DELETE ON of_edit_history
BEGIN
    SELECT RAISE(ABORT, 'table of_edit_history is append-only');
END;
