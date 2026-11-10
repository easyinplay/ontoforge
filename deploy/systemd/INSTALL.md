# Installing `ontoforge-webhook-worker.service`

This unit runs `ontoforge webhook-worker` under systemd: it starts the worker at boot
(`WantedBy=multi-user.target`) and restarts it automatically after an abnormal exit
(`Restart=on-failure`, `RestartSec=5s`, no rate-limit window).

## 1. Dedicated system account

```sh
sudo useradd --system --user-group --shell /usr/sbin/nologin --no-create-home ontoforge
```

The account has no login shell and no home directory; the unit's `ProtectHome=true`
keeps `/home` unreachable regardless.

## 2. Binary

Install the built `ontoforge` binary at the path the unit's `ExecStart` references:

```sh
sudo install -m 0755 -o root -g root ontoforge /usr/local/bin/ontoforge
```

## 3. Configuration

The worker reads `[store]` and `[webhook]` sections from
`/etc/ontoforge/ontoforge-server.toml`:

```sh
sudo install -d -m 0750 -o root -g ontoforge /etc/ontoforge
sudo install -m 0640 -o root -g ontoforge ontoforge-server.toml /etc/ontoforge/ontoforge-server.toml
```

A minimal configuration:

```toml
[store]
kind = "postgres"
url_file = "/etc/ontoforge/postgres-url"
schema_edit = "of_edit"
schema_sys = "of_sys"

[webhook]
target = "https://receiver.example.com/events"
poll_interval_ms = 200
request_timeout_ms = 1000
lease_duration_ms = 5000
batch_limit = 10
```

Never put the connection string into the unit file or `ontoforge-server.toml`
directly. Reference it with `[store].url_file` and keep that file in a separate
credential file with identical ownership and mode (`root:ontoforge`, `0640`):

```sh
sudo install -m 0640 -o root -g ontoforge /dev/null /etc/ontoforge/postgres-url
sudoedit /etc/ontoforge/postgres-url   # one line: postgres://user:pass@host:5432/dbname
```

`lease_duration_ms` must exceed `request_timeout_ms`; the worker refuses the
configuration otherwise.

## 4. Unit

```sh
sudo install -m 0644 -o root -g root ontoforge-webhook-worker.service /etc/systemd/system/ontoforge-webhook-worker.service
sudo systemd-analyze verify /etc/systemd/system/ontoforge-webhook-worker.service
sudo systemctl daemon-reload
sudo systemctl enable --now ontoforge-webhook-worker.service
```

`enable` installs only the boot-time start relationship (`multi-user.target.wants`
symlink) and `--now` performs the immediate first start. The automatic-restart
guarantee itself is not proven by `enable`: the actual auto-recovery after an
abnormal exit is verified by deployment verification — kill the worker's MainPID
(`sudo kill -9 "$(systemctl show -p MainPID --value ontoforge-webhook-worker.service)"`)
and confirm that `systemctl show -p NRestarts,InvocationID` reports a new invocation
after `RestartSec`, or perform a real host reboot and confirm the unit came up with
the new boot. This distinction is what the project UAT keeps under `VERDICT: PENDING`
for human signoff.

## 5. Inspection and removal

- Status and restart counter: `systemctl status ontoforge-webhook-worker.service`,
  `systemctl show -p NRestarts,InvocationID,Result ontoforge-webhook-worker.service`
- Logs: `journalctl -u ontoforge-webhook-worker.service -b`
- Remove: `systemctl disable --now ontoforge-webhook-worker.service`, delete the unit,
  the configuration directory, and `/usr/local/bin/ontoforge`, then `daemon-reload`
  and `userdel ontoforge`.
