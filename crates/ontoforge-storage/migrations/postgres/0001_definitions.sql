-- Definition bookkeeping of the PostgreSQL edit store (ADR-0024).
-- Runs with `search_path` set to the system schema (`[store] schema_sys`), so the tables and
-- sqlx's `_sqlx_migrations` record land there and never in the schema Trino can read.
-- The per-type edit tables (of_obj_*, of_lnk_*) are created by `reload`, not by this migration.

CREATE TABLE of_def_state (
    id bigint PRIMARY KEY CHECK (id = 1),
    active_def_version bigint NOT NULL
);

INSERT INTO of_def_state (id, active_def_version) VALUES (1, 0);

CREATE TABLE of_def_version (
    def_version bigint PRIMARY KEY,
    status text NOT NULL CHECK (status IN ('pending', 'active', 'superseded')),
    ontology_json jsonb NOT NULL,
    physical_json jsonb NOT NULL,
    migration_json jsonb NULL,
    created_at timestamptz NOT NULL,
    activated_at timestamptz NULL
);
