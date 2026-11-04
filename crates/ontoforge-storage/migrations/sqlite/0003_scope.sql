-- Users, roles, role bindings and the admin audit of the SQLite edit store (SCOPE-01, SCOPE-07;
-- D-01..D-04, plan 03-16).
--
-- `of_role` and `of_role_binding` are written by `scope apply` only, and `of_admin_audit` is append
-- only like the edit log. `of_user` holds a principal's identity and its last login, never a
-- permission or a claim (D-04). Timestamps are `YYYY-MM-DDTHH:MM:SS.fffZ` (UTC).

CREATE TABLE of_user (
    principal_id TEXT PRIMARY KEY,
    issuer TEXT NOT NULL,
    subject TEXT NOT NULL,
    created_at TEXT NOT NULL,
    last_login_at TEXT NOT NULL
);

CREATE TABLE of_role (
    api_name TEXT PRIMARY KEY,
    ir_json TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    applied_at TEXT NOT NULL
);

CREATE TABLE of_role_binding (
    role TEXT NOT NULL REFERENCES of_role (api_name),
    subject_kind TEXT NOT NULL CHECK (subject_kind IN ('user', 'group')),
    subject TEXT NOT NULL,
    PRIMARY KEY (role, subject_kind, subject)
);

CREATE TABLE of_admin_audit (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    at TEXT NOT NULL,
    actor TEXT NOT NULL,
    action TEXT NOT NULL,
    plan_json TEXT NOT NULL,
    before_hash TEXT NOT NULL,
    after_hash TEXT NOT NULL
);

-- Append only, as for the edit log (0002): a BEFORE trigger on UPDATE and on DELETE refuses every
-- rewrite. The triggers are no boundary against the owner of the database file.
CREATE TRIGGER of_admin_audit_no_update BEFORE UPDATE ON of_admin_audit
BEGIN
    SELECT RAISE(ABORT, 'table of_admin_audit is append-only');
END;

CREATE TRIGGER of_admin_audit_no_delete BEFORE DELETE ON of_admin_audit
BEGIN
    SELECT RAISE(ABORT, 'table of_admin_audit is append-only');
END;
