-- One principal-scoped claim and one durable delivery event per Action batch.
-- Runs with search_path set to the system schema, like the earlier migrations.
-- An expired command may be removed without deleting the immutable log or outbox event.
CREATE TABLE of_action_command (
    principal_id text NOT NULL,
    command_id text NOT NULL,
    action text NOT NULL,
    request_hash text NOT NULL,
    action_def_hash text NOT NULL,
    def_version bigint NOT NULL,
    chosen_current_time timestamptz NOT NULL,
    event_id text REFERENCES of_edit_log (event_id),
    receipt jsonb,
    expires_at timestamptz NOT NULL,
    PRIMARY KEY (principal_id, command_id)
);

CREATE INDEX of_ix_action_command_expiry ON of_action_command (expires_at);

CREATE TABLE of_webhook_outbox (
    event_id text PRIMARY KEY REFERENCES of_edit_log (event_id),
    payload jsonb NOT NULL,
    attempt_count integer NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    next_attempt_at timestamptz NOT NULL,
    lease_owner text,
    lease_until timestamptz,
    delivered_at timestamptz,
    last_error text
);

CREATE INDEX of_ix_webhook_outbox_ready
    ON of_webhook_outbox (next_attempt_at, lease_until)
    WHERE delivered_at IS NULL;
