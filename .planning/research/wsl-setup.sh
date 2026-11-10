#!/usr/bin/env bash
# Phase 09 UAT test 4 — real-machine install (runs as root in WSL)
set -euo pipefail
EVID=/root/phase9-evidence
mkdir -p "$EVID"

# 1. service account
id ontoforge &>/dev/null || useradd --system --no-create-home --shell /usr/sbin/nologin ontoforge

# 2. binary
install -o root -g root -m 0755 /home/easyinplay/ontoforge/target/debug/ontoforge /usr/local/bin/ontoforge

# 3. config dirs
mkdir -p /etc/ontoforge /var/lib/ontoforge
chown ontoforge:ontoforge /var/lib/ontoforge
chmod 0750 /var/lib/ontoforge

# 4. receiver ledger dir (receiver runs as easyinplay)
mkdir -p /home/easyinplay/phase9-receiver && chown easyinplay:easyinplay /home/easyinplay/phase9-receiver

# 5. server config — sqlite store + webhook to local receiver
cat > /etc/ontoforge/ontoforge-server.toml <<'EOF'
[store]
kind = "sqlite"
path = "/var/lib/ontoforge/edit.db"

[webhook]
target = "http://127.0.0.1:18923/events"
poll_interval_ms = 100
request_timeout_ms = 4000
lease_duration_ms = 5000
batch_limit = 1
EOF
chown root:ontoforge /etc/ontoforge/ontoforge-server.toml
chmod 0640 /etc/ontoforge/ontoforge-server.toml

# 6. install the shipped unit verbatim
install -o root -g root -m 0644 /mnt/d/DataSpellProjects/ontoforge/deploy/systemd/ontoforge-webhook-worker.service /etc/systemd/system/ontoforge-webhook-worker.service

# 7. verify + enable
systemd-analyze verify /etc/systemd/system/ontoforge-webhook-worker.service 2>&1 | tee "$EVID/systemd-analyze-verify.txt"
systemctl daemon-reload
systemctl enable --now ontoforge-webhook-worker.service 2>&1 | tee "$EVID/enable.txt"
systemctl cat ontoforge-webhook-worker.service | tee "$EVID/systemctl-cat.txt"
systemctl show ontoforge-webhook-worker.service -p MainPID,InvocationID,NRestarts,ActiveState,ExecMainStatus | tee "$EVID/systemctl-show-initial.txt"
echo "SETUP_DONE"
