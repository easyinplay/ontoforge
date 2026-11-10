#!/usr/bin/env bash
# Phase 09 UAT: SIGKILL the worker MainPID, observe systemd auto-restart, verify redelivery.
set -uo pipefail
EVID=/root/phase9-evidence
UNIT=ontoforge-webhook-worker.service

OLD_PID=$(systemctl show -p MainPID "$UNIT" | cut -d= -f2)
OLD_INV=$(systemctl show -p InvocationID "$UNIT" | cut -d= -f2)
echo "killing MainPID=$OLD_PID InvocationID=$OLD_INV"
kill -9 "$OLD_PID"
sleep 1
systemctl show "$UNIT" -p ActiveState,SubState,NRestarts
sleep 8
echo "--- after auto-restart ---"
systemctl show "$UNIT" -p ActiveState,SubState,InvocationID,MainPID,NRestarts,ExecMainStatus | tee "$EVID/systemctl-show-after-kill.txt"
NEW_PID=$(systemctl show -p MainPID "$UNIT" | cut -d= -f2)
NEW_INV=$(systemctl show -p InvocationID "$UNIT" | cut -d= -f2)
NEW_RST=$(systemctl show -p NRestarts "$UNIT" | cut -d= -f2)
echo "{\"case\":\"real-kill9-auto-restart\",\"unit\":\"$UNIT\",\"old_pid\":$OLD_PID,\"old_invocation\":\"$OLD_INV\",\"new_pid\":$NEW_PID,\"new_invocation\":\"$NEW_INV\",\"nrestarts\":$NEW_RST}" | tee "$EVID/kill-restart.json"

# seed one more event post-restart to prove delivery continues
EID=$(python3 /mnt/d/DataSpellProjects/ontoforge/.planning/research/wsl-seed.py)
echo "seeded post-restart event: $EID"
sleep 4
echo "--- ledger tail ---"
tail -2 /home/easyinplay/phase9-receiver/ledger.jsonl | tee "$EVID/ledger-after-kill.txt"
python3 - <<'PY'
import sqlite3
con = sqlite3.connect("/var/lib/ontoforge/edit.db")
print("outbox:", con.execute(
  "SELECT event_id,attempt_count,delivered_at IS NOT NULL FROM of_webhook_outbox").fetchall())
PY
echo "KILL_TEST_DONE"
