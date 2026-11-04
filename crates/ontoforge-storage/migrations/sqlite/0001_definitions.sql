-- Definition bookkeeping of the SQLite edit store (ADR-0024).
-- The per-type edit tables (of_obj_*, of_lnk_*) are created by `reload`, not by this migration.

CREATE TABLE of_def_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    active_def_version INTEGER NOT NULL
);

INSERT INTO of_def_state (id, active_def_version) VALUES (1, 0);

CREATE TABLE of_def_version (
    def_version INTEGER PRIMARY KEY,
    status TEXT NOT NULL CHECK (status IN ('pending', 'active', 'superseded')),
    ontology_json TEXT NOT NULL,
    physical_json TEXT NOT NULL,
    migration_json TEXT NULL,
    created_at TEXT NOT NULL,
    activated_at TEXT NULL
);
