-- Users, roles, role bindings and the admin audit of the PostgreSQL edit store (SCOPE-01,
-- SCOPE-07; D-01..D-04, plan 03-16). Runs with `search_path` set to the system schema, like 0001.
--
-- `of_role` and `of_role_binding` are written by `scope apply` only, and `of_admin_audit` is append
-- only like the edit log. `of_user` holds a principal's identity and its last login, never a
-- permission or a claim (D-04).

CREATE TABLE of_user (
    principal_id text PRIMARY KEY,
    issuer text NOT NULL,
    subject text NOT NULL,
    created_at timestamptz NOT NULL,
    last_login_at timestamptz NOT NULL
);

CREATE TABLE of_role (
    api_name text PRIMARY KEY,
    ir_json jsonb NOT NULL,
    content_hash text NOT NULL,
    applied_at timestamptz NOT NULL
);

CREATE TABLE of_role_binding (
    role text NOT NULL REFERENCES of_role (api_name),
    subject_kind text NOT NULL CHECK (subject_kind IN ('user', 'group')),
    subject text NOT NULL,
    PRIMARY KEY (role, subject_kind, subject)
);

CREATE TABLE of_admin_audit (
    seq bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    at timestamptz NOT NULL,
    actor text NOT NULL,
    action text NOT NULL,
    plan_json jsonb NOT NULL,
    before_hash text NOT NULL,
    after_hash text NOT NULL
);

-- Append only, with the trigger function of 0002. The statement-level trigger covers TRUNCATE,
-- which row triggers never see.
CREATE TRIGGER of_admin_audit_no_update_delete BEFORE UPDATE OR DELETE ON of_admin_audit
    FOR EACH ROW EXECUTE FUNCTION of_reject_mutation();
CREATE TRIGGER of_admin_audit_no_truncate BEFORE TRUNCATE ON of_admin_audit
    FOR EACH STATEMENT EXECUTE FUNCTION of_reject_mutation();
