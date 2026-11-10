#!/usr/bin/env python3
"""Seed one synthetic outbox event for the phase9 WSL UAT."""
import json, sqlite3, sys, time, uuid

DB = "/var/lib/ontoforge/edit.db"
event_id = "WSLUAT-" + uuid.uuid4().hex[:16].upper()
now = time.strftime("%Y-%m-%dT%H:%M:%S.000Z", time.gmtime())

con = sqlite3.connect(DB)
try:
    cur = con.cursor()
    cur.execute(
        "INSERT INTO of_edit_log (event_id, actor, action, committed_at, def_version, params)"
        " VALUES (?,?,?,?,?,?)",
        (event_id, "wsl-uat-seed", "seed_event", now, 1, "{}"),
    )
    payload = json.dumps({
        "schema_version": 1, "event_id": event_id, "log_seq": cur.lastrowid,
        "committed_at": now, "principal_id": "wsl-uat-seed",
        "action": "seed_event", "receipt": {"event_id": event_id},
    })
    cur.execute(
        "INSERT INTO of_webhook_outbox (event_id, payload, attempt_count, next_attempt_at)"
        " VALUES (?,?,0,?)",
        (event_id, payload, now),
    )
    con.commit()
finally:
    con.close()
print(event_id)
