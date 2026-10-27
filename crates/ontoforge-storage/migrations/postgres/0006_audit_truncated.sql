ALTER TABLE of_read_audit DROP CONSTRAINT of_read_audit_status_check;
ALTER TABLE of_read_audit ADD CONSTRAINT of_read_audit_status_check
    CHECK (status IN ('success', 'truncated', 'denied', 'source_error', 'cancelled'));
