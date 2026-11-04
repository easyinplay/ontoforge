-- One principal-scoped claim and one durable delivery event per Action batch.
-- Time values use UTC milliseconds, matching the edit log. A command may expire
-- without deleting its immutable log or delivery event.
CREATE TABLE of_action_command (
    principal_id TEXT NOT NULL,
    command_id TEXT NOT NULL,
    action TEXT NOT NULL,
    request_hash TEXT NOT NULL,
    action_def_hash TEXT NOT NULL,
    def_version INTEGER NOT NULL,
    chosen_current_time TEXT NOT NULL,
    event_id TEXT REFERENCES of_edit_log (event_id),
    receipt TEXT,
    expires_at TEXT NOT NULL,
    PRIMARY KEY (principal_id, command_id)
);

CREATE INDEX of_ix_action_command_expiry ON of_action_command (expires_at);

CREATE TABLE of_webhook_outbox (
    event_id TEXT PRIMARY KEY REFERENCES of_edit_log (event_id),
    payload TEXT NOT NULL,
    attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    next_attempt_at TEXT NOT NULL,
    lease_owner TEXT,
    lease_until TEXT,
    delivered_at TEXT,
    last_error TEXT
);

CREATE INDEX of_ix_webhook_outbox_ready
    ON of_webhook_outbox (next_attempt_at, lease_until)
    WHERE delivered_at IS NULL;
