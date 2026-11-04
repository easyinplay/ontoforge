#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$ROOT"
export CARGO_BUILD_JOBS=2 MSYS_NO_PATHCONV=1
export PATH="$ROOT/target/phase7-tools:$PATH"
if command -v cygpath >/dev/null; then
  export ICEBERG_BASH="$(cygpath -w "$BASH")"
  PYTHON=python
else
  export ICEBERG_BASH="$BASH"
  PYTHON=python3
fi
# Git Bash cannot deliver POSIX TERM to native Python handlers. A marker lets
# the Python owner stop its child and finish cleanup before Bash exits.
signal_file=$(mktemp)
if command -v cygpath >/dev/null; then
  export ICEBERG_SIGNAL_FILE="$(cygpath -w "$signal_file")"
else
  export ICEBERG_SIGNAL_FILE="$signal_file"
fi
child=""
forward_signal() {
  printf '%s' "$1" > "$signal_file"
  if [[ -n "$child" ]]; then
    while kill -0 "$child" 2>/dev/null; do wait "$child" || true; done
  fi
  exit "$1"
}
trap 'forward_signal 143' TERM
trap 'forward_signal 130' INT
trap 'rm -f -- "$signal_file"' EXIT
"$PYTHON" - "$@" <<'PY' &
import hashlib
from contextlib import contextmanager
import json
import os
from pathlib import Path
import re
import secrets
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import urllib.request

ROOT = Path.cwd()
SOURCE = "9e49d5e7a648f00e26f2246f4dc28e6b07f8c84a"
BUILDER = "docker.io/library/golang:1.24.8-bookworm@sha256:39e2f344b68e61144a5de3d3bd166ea746ee8e1965e6fc2ad046df5cd6c8369e"
IMAGE = "ontoforge-minio:" + SOURCE
PG = "postgres:17.11@sha256:2d2b8998d31037bf721cfdf764d76ba74171b4fab3431b7f72c27c56ddbdf9e3"
TRINO = {
    "483": "trinodb/trino:483@sha256:db58cc93e593a2706553745f276bb119c9810e69918be56ecde088ba7ccb0534",
    "484": "trinodb/trino:484@sha256:2704eb53935af5e582d63d6033e9bf99d20c325e9b0e30b1016ef59a4773979f",
}
LABEL = "ontoforge.iceberg.owner"
END = time.monotonic() + 3300
CHILD = None
IMPLEMENTATION_HASHES = {name: hashlib.sha256((ROOT / name).read_bytes()).hexdigest() for name in (
    "ci/iceberg-services.sh", "ci/minio/Dockerfile", "ci/minio/bucket.go", "ci/trino/iceberg/iceberg.properties",
    "crates/ontoforge-engine/tests/trino_it.rs")}


class Failure(Exception):
    def __init__(self, message, code=1):
        super().__init__(message)
        self.code = code if code > 0 else 1


def require(condition, message):
    if not condition:
        raise Failure(message)


def stop_child():
    if CHILD is not None and CHILD.poll() is None:
        if os.name == "nt":
            subprocess.run(["taskkill", "/PID", str(CHILD.pid), "/T", "/F"],
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=10)
        else:
            os.killpg(CHILD.pid, signal.SIGTERM)
        try:
            CHILD.wait(timeout=5)
        except subprocess.TimeoutExpired:
            if os.name != "nt":
                os.killpg(CHILD.pid, signal.SIGKILL)
            CHILD.kill()
            CHILD.wait(timeout=5)


def interrupted(signum, frame):
    stop_child()
    raise Failure("run interrupted", 128 + signum)


for sig in (signal.SIGTERM, signal.SIGINT):
    signal.signal(sig, interrupted)


def call(args, *, limit=120, log=None, data=None, check=True):
    global CHILD
    remaining = min(limit, END - time.monotonic())
    require(remaining > 0, "run deadline exceeded")
    CHILD = subprocess.Popen([os.environ["ICEBERG_BASH"], "--noprofile", "--norc", "-c", 'exec "$@"',
                              "iceberg", *map(str, args)], stdin=subprocess.PIPE,
                             stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                             start_new_session=os.name != "nt")
    try:
        deadline = time.monotonic() + remaining
        while True:
            marker = os.environ.get("ICEBERG_SIGNAL_FILE")
            requested = Path(marker).read_text() if marker else ""
            if requested:
                Path(marker).write_text("")
                stop_child()
                raise Failure("run interrupted", int(requested))
            try:
                out, err = CHILD.communicate(data, timeout=min(0.2, max(0.001, deadline - time.monotonic())))
                break
            except subprocess.TimeoutExpired:
                data = None
                if time.monotonic() >= deadline:
                    raise
        code = CHILD.returncode
    except subprocess.TimeoutExpired:
        stop_child()
        raise Failure("command deadline exceeded", 124)
    finally:
        CHILD = None
    if log is not None:
        Path(log).write_bytes(out + err)
    if check and code:
        raise Failure("command failed: " + str(args[0]), code)
    return code, out.decode("utf-8", errors="replace"), err.decode("utf-8", errors="replace")


def docker(*args, **kwargs):
    return call(["docker", *args], **kwargs)


def private(path, directory=False):
    if os.name != "nt":
        path.chmod(0o700 if directory else 0o600)
        require(path.stat().st_mode & 0o077 == 0, "private permissions failed")
        return
    envname = "ICEBERG_ACL_PATH"
    os.environ[envname] = str(path)
    # PowerShell 7's inherited module path can prevent Windows PowerShell 5
    # from loading its own ACL cmdlets.
    module_path = os.environ.pop("PSModulePath", None)
    script = "$ErrorActionPreference='Stop'; $p=$env:ICEBERG_ACL_PATH; " \
             "$sid=[Security.Principal.WindowsIdentity]::GetCurrent().User; " \
             "$rights=if(Test-Path -LiteralPath $p -PathType Container){':(OI)(CI)F'}else{':F'}; " \
             "& icacls.exe $p /inheritance:r /grant:r ('*'+$sid.Value+$rights) | Out-Null; if($LASTEXITCODE){exit $LASTEXITCODE}; " \
             "$actual=Get-Acl -LiteralPath $p; if(-not $actual.AreAccessRulesProtected){exit 2}; " \
             "if($actual.GetOwner([Security.Principal.SecurityIdentifier]).Value -ne $sid.Value){exit 4}; " \
             "$trusted=@($sid.Value,'S-1-5-18','S-1-5-32-544','S-1-3-4'); " \
             "$directory=Test-Path -LiteralPath $p -PathType Container; " \
             "foreach($r in $actual.Access){ " \
             "if($r.AccessControlType -ne 'Allow' -or $r.IsInherited -or $r.PropagationFlags -ne 'None'){exit 5}; " \
             "if($trusted -notcontains $r.IdentityReference.Translate([Security.Principal.SecurityIdentifier]).Value){exit 3}; " \
             "if($directory -and $r.InheritanceFlags -ne 'ContainerInherit, ObjectInherit'){exit 6}; " \
             "if(-not $directory -and $r.InheritanceFlags -ne 'None'){exit 7}}"
    try:
        call(["powershell.exe", "-NoProfile", "-NonInteractive", "-Command", script])
    finally:
        os.environ.pop(envname, None)
        if module_path is not None:
            os.environ["PSModulePath"] = module_path


def write_json(path, value):
    path = Path(path)
    tmp = path.with_name(path.name + ".new")
    with os.fdopen(os.open(tmp, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600), "w", encoding="utf-8") as output:
        output.write(json.dumps(value, indent=2) + "\n")
    tmp.replace(path)


@contextmanager
def owner_lock():
    lock = Path(tempfile.gettempdir()) / "ontoforge-iceberg.lock"
    token = secrets.token_hex(24)
    require(call(["mkdir", str(lock)], check=False)[0] == 0, "another Iceberg owner holds the lock")
    try:
        (lock / "owner").write_text(token)
        os.environ["ICEBERG_LOCK_TOKEN"] = token
        yield
    finally:
        os.environ.pop("ICEBERG_LOCK_TOKEN", None)
        require((lock / "owner").read_text() == token, "lock owner changed")
        (lock / "owner").unlink()
        lock.rmdir()


def exact(directory, suffix=""):
    require(os.environ.get("ONTOFORGE_TRINO_ICEBERG") == "1", "Iceberg environment guard failed")
    require(bool(os.environ.get("ONTOFORGE_TRINO_URL", "").strip()), "Trino URL guard failed")
    directory.mkdir(parents=True, exist_ok=True)
    command = ["cargo", "test", "-p", "ontoforge-engine", "--test", "trino_it", "--locked",
               "iceberg_table_is_readable", "--", "--ignored", "--exact"]
    _, found, _ = call(command + ["--list"], limit=900, log=directory / ("iceberg-discovery" + suffix + ".log"))
    require(re.findall(r"^(.+): test\r?$", found, re.M) == ["iceberg_table_is_readable"],
            "exact discovery must contain exactly one target")
    _, result, _ = call(command + ["--nocapture"], limit=900, log=directory / ("iceberg-exact" + suffix + ".log"))
    require(len(re.findall(r"^test result: ok\. 1 passed; 0 failed; 0 ignored;", result, re.M)) == 1,
            "exact execution did not pass one unignored test")


def validate(directory, phase="before"):
    counts = json.loads((directory / ("counts-" + phase + ".json")).read_text())
    require(counts == {"source": 29, "copied": 29}, "row counts must both equal 29")
    locations = (directory / ("metadata-locations.txt" if phase == "before" else "metadata-locations-after.txt")).read_text().splitlines()
    require(len(locations) == 1 and locations[0].startswith("s3://warehouse/"), "metadata must use S3")
    location = locations[0][len("s3://warehouse/"):]
    require("/metadata/" in location, "invalid metadata object")
    prefix = location.split("/metadata/")[0] + "/"
    objects = json.loads((directory / ("objects-" + phase + ".json")).read_text())
    require(any(o["key"] == location and o["size"] > 0 for o in objects), "metadata object is absent or empty")
    require(any(o["key"].startswith(prefix + "data/") and o["size"] > 0 for o in objects),
            "table data objects are absent or empty")


def run_corpus(directory):
    suites = [
        ("action-atomicity", ["-p", "ontoforge-storage", "--features", "test-support", "--test", "conformance_pg", "--locked", "action_atomic"], True),
        ("action-races", ["-p", "ontoforge-action", "--features", "test-support", "--test", "action_pg", "--locked"], True),
        ("action-idempotency", ["-p", "ontoforge-action", "--features", "test-support", "--test", "idempotency_pg", "--locked"], True),
        ("action-webhook-pg", ["-p", "ontoforge-action", "--test", "outbox_pg", "--locked"], True),
        ("action-source", ["-p", "ontoforge-action", "--test", "trino_action_it", "--locked"], True),
        ("action-outbox", ["-p", "ontoforge-action", "--test", "outbox", "--locked"], False),
        ("action-entrypoints", ["-p", "ontoforge-action", "--test", "entrypoints", "--locked"], False),
        ("engine", ["-p", "ontoforge-engine", "--test", "trino_it", "--locked"], True),
        ("cli", ["-p", "ontoforge", "--test", "cli_trino_it", "--locked"], True),
    ]
    records = []
    for name, options, ignored in suites:
        args = ["cargo", "test", *options, "--", *(["--ignored"] if ignored else [])]
        code, found, _ = call(args + ["--list"], limit=900, log=directory / (name + "-discovery.log"), check=False)
        targets = [options[i + 1] for i, option in enumerate(options) if option == "--test"]
        record = {"name": name, "targets": targets, "discovered": len(re.findall(r"^.+: test\r?$", found, re.M)),
                  "passed": 0, "failed": 0, "ignored": 0, "exit_code": code, "stage": "discovery"}
        records.append(record)
        write_json(directory / "corpus.json", records)
        if code:
            raise Failure("corpus discovery failed: " + name, code)
        require(record["discovered"] > 0, "corpus discovery was empty: " + name)
        code, output, _ = call(args + ["--nocapture"], limit=900, log=directory / (name + ".log"), check=False)
        summaries = re.findall(r"^test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;", output, re.M)
        record.update(stage="execution", exit_code=code, passed=sum(int(s[0]) for s in summaries),
                      failed=sum(int(s[1]) for s in summaries), ignored=sum(int(s[2]) for s in summaries),
                      summaries=len(summaries))
        write_json(directory / "corpus.json", records)
        if code:
            raise Failure("corpus execution failed: " + name, code)
        require(len(summaries) == len(targets) and record["passed"] == record["discovered"]
                and record["passed"] > 0 and record["failed"] == record["ignored"] == 0,
                "corpus execution count mismatch: " + name)


def scan(raw, credentials):
    require(len(credentials) == 3 and all(credentials), "empty scan credentials")
    needles = [value.encode() for value in credentials]
    files = list(raw.rglob("*"))
    require(bool(files), "empty evidence")
    for path in files:
        require(not path.is_symlink(), "evidence symlink refused")
        if path.is_file():
            content = path.read_bytes()
            require(not any(value in content for value in needles), "secret scan rejected evidence")


def snapshot():
    _, output, _ = docker("ps", "-aq")
    ids = output.split()
    if not ids:
        return {}
    _, output, _ = docker("inspect", *ids)
    return {o["Id"]: {"name": o["Name"], "status": o["State"]["Status"],
                       "started": o["State"]["StartedAt"]} for o in json.loads(output)}


def catalog_hashes():
    return {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in (ROOT / "ci/trino/catalog").glob("*") if p.is_file()}


def discover(owner, kind):
    args = ["ps", "-aq"] if kind == "container" else [kind, "ls", "-q"]
    return docker(*args, "--filter", "label=" + LABEL + "=" + owner)[1].split()


def cleanup(state):
    global END
    END = time.monotonic() + 60
    owner = state["owner"]
    require(re.fullmatch(r"iceberg-[0-9a-f]{24}", owner), "invalid owner")
    runtime = Path(state["runtime"])
    require(runtime.resolve().parent == Path(tempfile.gettempdir()).resolve(), "runtime parent refused")
    require(runtime.name == owner, "runtime name refused")
    require((runtime / "owner").read_text() == owner, "runtime owner refused")
    lock = Path(tempfile.gettempdir()) / "ontoforge-iceberg.lock"
    if lock.exists():
        require(bool(state.get("lock_token")) and (lock / "owner").read_text() == state["lock_token"],
                "lock owner changed")
    failed = False
    for kind in ("container", "network", "volume"):
        recorded = [r["id"] for r in state["resources"] if r["kind"] == kind]
        for resource in set(recorded + discover(owner, kind)):
            code, output, _ = docker(*([] if kind == "container" else [kind]), "inspect", resource, check=False)
            if code:
                if resource in discover(owner, kind):
                    failed = True
                continue
            info = json.loads(output)[0]
            actual = info["Id"] if kind != "volume" else info["Name"]
            labels = info["Config"].get("Labels", {}) if kind == "container" else info.get("Labels", {})
            if labels.get(LABEL) != owner or (kind != "volume" and not actual.startswith(resource)):
                failed = True
                continue
            args = ["rm", "-f", actual] if kind == "container" else [kind, "rm", actual]
            if docker(*args, check=False)[0]:
                failed = True
    remaining = sum(len(discover(owner, k)) for k in ("container", "network", "volume"))
    return {"ok": not failed and remaining == 0, "remaining": remaining}


def contract(version, nightly=False):
    require(version in TRINO, "unsupported Trino version")
    fault = os.environ.get("ONTOFORGE_ICEBERG_FAULT", "")
    require(fault in ("", "after-pg", "readiness-timeout"), "unsupported fault injection")
    require(not fault or os.environ.get("ONTOFORGE_ICEBERG_LIVE") == "1", "fault injection requires explicit live opt-in")
    owner = "iceberg-" + secrets.token_hex(12)
    runtime = Path(tempfile.gettempdir()) / owner
    runtime.mkdir(mode=0o700)
    private(runtime, directory=True)
    (runtime / "owner").write_text(owner)
    raw = runtime / "raw"
    raw.mkdir()
    evidence = Path(os.environ.get("ONTOFORGE_ICEBERG_EVIDENCE_DIR", str(ROOT / "target/phase072-minio" / owner))).resolve()
    require(not evidence.exists(), "evidence directory already exists")
    state_path = Path(os.environ.get("ONTOFORGE_ICEBERG_STATE", str(runtime / "state.json"))).resolve()
    state = {"owner": owner, "runtime": str(runtime), "resources": [], "evidence": str(evidence),
             "lock_token": os.environ.get("ICEBERG_LOCK_TOKEN")}
    write_json(state_path, state)
    private(state_path)
    credentials = [secrets.token_hex(24) for _ in range(3)]
    (runtime / "secrets.json").write_text(json.dumps(credentials))
    status, cleanup_result, scan_passed = 0, {"ok": False, "remaining": -1}, False
    baseline, hashes = None, catalog_hashes()
    statements = []

    def resource(kind, name, args):
        # The persisted owner permits recovery if creation succeeds but ledger writing fails.
        _, output, _ = docker(*args)
        identifier = output.strip()
        state["resources"].append({"kind": kind, "id": identifier, "name": name})
        write_json(state_path, state)
        return identifier

    def helper(operation, log=None, check=True):
        return docker("run", "--rm", "--network", owner, "--label", LABEL + "=" + owner,
                      "--env-file", runtime / "minio.env", "--entrypoint", "/bucket", IMAGE,
                      operation, "--bucket", "warehouse", limit=40, log=log, check=check)

    def sql(statement):
        statements.append(statement)
        write_json(raw / "sql-statements.json", statements)
        return docker("exec", owner + "-trino", "trino", "--output-format", "CSV_UNQUOTED",
                      "--execute", statement, limit=120)[1].strip()

    def wait_for(check):
        seconds = int(os.environ.get("ONTOFORGE_ICEBERG_READINESS_SECONDS", "180"))
        require(1 <= seconds <= 180, "invalid readiness deadline")
        deadline = time.monotonic() + seconds
        while True:
            require(time.monotonic() < deadline, "service readiness timeout")
            if check():
                return
            require(time.monotonic() < deadline, "service readiness timeout")
            time.sleep(2)

    def collect(phase):
        write_json(raw / ("counts-" + phase + ".json"), {"source": int(sql("SELECT count(*) FROM demo.aviation.flights")),
                                                       "copied": int(sql("SELECT count(*) FROM iceberg.aviation.flights"))})
        metadata = docker("exec", pgname, "psql", "-U", "postgres", "-d", "iceberg_catalog", "-At", "-c",
                          "SELECT metadata_location FROM iceberg_tables WHERE catalog_name='ontoforge_ci' "
                          "AND table_namespace='aviation' AND table_name='flights'")[1]
        name = "metadata-locations.txt" if phase == "before" else "metadata-locations-after.txt"
        (raw / name).write_text(metadata)
        helper("list", raw / ("objects-" + phase + ".json"))
        validate(raw, phase)
        return metadata

    def identity():
        _, containers, _ = docker("inspect", owner + "-pg", owner + "-minio", owner + "-trino")
        _, volumes, _ = docker("volume", "inspect", owner + "-pg", owner + "-minio")
        return {"containers": {o["Name"]: o["Id"] for o in json.loads(containers)},
                "volumes": {o["Name"]: {"created": o.get("CreatedAt"), "mountpoint": o.get("Mountpoint")}
                            for o in json.loads(volumes)}}

    try:
        baseline = snapshot()
        write_json(raw / "containers-before.json", baseline)
        if fault:
            write_json(raw / "fault.json", {"injected": fault, "natural_service_failure": False})
        call(["docker", "build", "--platform", "linux/amd64", "-t", IMAGE, "ci/minio"],
             limit=1800, log=raw / "build.log")
        _, image_info, _ = docker("image", "inspect", IMAGE)
        image_id = json.loads(image_info)[0]["Id"]
        versions = {}
        for binary in ("minio", "bucket"):
            versions[binary] = docker("run", "--rm", "--label", LABEL + "=" + owner, "--entrypoint", "/" + binary, IMAGE,
                                      "--version" if binary == "minio" else "version")[1].strip()
        extract = resource("container", owner + "-artifact", ["create", "--name", owner + "-artifact",
                           "--label", LABEL + "=" + owner, IMAGE])
        docker("cp", extract + ":/SHA256SUMS", runtime / "SHA256SUMS")
        write_json(raw / "provenance.json", {"source": SOURCE, "builder": BUILDER,
                   "image_id": image_id, "versions": versions, "sha256": (runtime / "SHA256SUMS").read_text(),
                   "build_exit": 0, "git_head": call(["git", "rev-parse", "HEAD"])[1].strip(),
                   "run_id": owner, "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
                   "implementation_sha256": IMPLEMENTATION_HASHES,
                   "artifact": "self-built DEVELOPMENT; no bit reproducibility claim"})
        resource("network", owner, ["network", "create", "--label", LABEL + "=" + owner, owner])
        for suffix in ("pg", "minio"):
            resource("volume", owner + "-" + suffix,
                     ["volume", "create", "--label", LABEL + "=" + owner, owner + "-" + suffix])
        (runtime / "pg.env").write_text("POSTGRES_PASSWORD=" + credentials[0] + "\n")
        (runtime / "minio.env").write_text("MINIO_ROOT_USER=" + credentials[1] + "\nMINIO_ROOT_PASSWORD=" + credentials[2]
            + "\nMINIO_BROWSER=off\nS3_ENDPOINT=http://minio:9000\nAWS_ACCESS_KEY_ID=" + credentials[1]
            + "\nAWS_SECRET_ACCESS_KEY=" + credentials[2] + "\nAWS_REGION=us-east-1\n")
        (runtime / "trino.env").write_text("AWS_ACCESS_KEY_ID=" + credentials[1]
            + "\nAWS_SECRET_ACCESS_KEY=" + credentials[2] + "\nAWS_REGION=us-east-1\n")
        catalog = runtime / "catalog"
        catalog.mkdir(mode=0o755)
        origins = sorted((ROOT / "ci/trino/catalog").glob("*.properties"))
        origins.append(ROOT / "ci/trino/iceberg/iceberg.properties")
        for origin in origins:
            content = origin.read_text().replace("localhost:5432", "postgres:5432")
            content = re.sub(r"(?m)^(.*connection-password)=.*$", lambda m: m[1] + "=" + credentials[0], content)
            (catalog / origin.name).write_text(content)
            (catalog / origin.name).chmod(0o644)
        for name in ("pg.env", "minio.env", "trino.env", "secrets.json"):
            private(runtime / name)
        common = ["--network", owner, "--label", LABEL + "=" + owner]
        pgname = owner + "-pg"
        resource("container", pgname, ["run", "-d", "--name", pgname, *common, "--network-alias", "postgres",
                 "--env-file", runtime / "pg.env", "-p", "127.0.0.1::5432", "-v", owner + "-pg:/var/lib/postgresql/data",
                 "-v", str(ROOT / "examples/ontologies/aviation/bindings/dev/fixtures") + ":/fixtures:ro", PG])
        if fault == "after-pg":
            raise Failure("injected failure after PostgreSQL creation", 42)
        wait_for(lambda: docker("exec", pgname, "pg_isready", "-U", "postgres", check=False)[0] == 0
                 and fault != "readiness-timeout")
        for database in ("demo", "editstore", "iceberg_catalog"):
            docker("exec", pgname, "createdb", "-U", "postgres", database)
        for database, filename, extra in [("demo", "seed.sql", ["-v", "fixtures=/fixtures"]),
                                          ("demo", "rowpred_corpus.sql", []),
                                          ("demo", "query_corpus.sql", []),
                                          ("iceberg_catalog", "iceberg_catalog.sql", [])]:
            docker("exec", "-i", pgname, "psql", "-U", "postgres", "-v", "ON_ERROR_STOP=1", "-d", database,
                   *extra, data=(ROOT / "ci/pg" / filename).read_bytes())
        resource("container", owner + "-minio", ["run", "-d", "--name", owner + "-minio", *common,
                 "--network-alias", "minio", "--env-file", runtime / "minio.env",
                 "-v", owner + "-minio:/data", IMAGE, "server", "/data", "--quiet"])
        # SDK calls have their own 30 second deadline; readiness remains bounded.
        wait_for(lambda: docker("run", "--rm", *common, "--env-file", runtime / "minio.env", "--entrypoint", "/bucket",
                                IMAGE, "make", "--bucket", "warehouse", limit=40, check=False)[0] == 0)
        resource("container", owner + "-trino", ["run", "-d", "--name", owner + "-trino", *common,
                 "--env-file", runtime / "trino.env",
                 "-p", "127.0.0.1::8080", "-v", str(catalog) + ":/etc/trino/catalog:ro", TRINO[version]])
        port = docker("port", owner + "-trino", "8080/tcp")[1].strip()
        require(re.fullmatch(r"127\.0\.0\.1:\d+", port), "non-loopback Trino port")
        pgport = docker("port", pgname, "5432/tcp")[1].strip()
        require(re.fullmatch(r"127\.0\.0\.1:\d+", pgport), "non-loopback PG port")
        os.environ.update(ONTOFORGE_TRINO_URL="http://" + port, ONTOFORGE_TRINO_ICEBERG="1",
                          ONTOFORGE_PG_URL="postgres://postgres:" + credentials[0] + "@" + pgport + "/editstore")

        def ready():
            try:
                with urllib.request.urlopen(os.environ["ONTOFORGE_TRINO_URL"] + "/v1/info", timeout=3) as response:
                    info = json.load(response)
                if info["starting"] is False and info["nodeVersion"]["version"] == version:
                    write_json(raw / "trino-info.json", info)
                    return True
            except (OSError, ValueError):
                pass
            return False

        wait_for(ready)
        docker("exec", owner + "-trino", "test", "-r", "/etc/trino/catalog/iceberg.properties")
        sql("CREATE SCHEMA iceberg.aviation")
        sql("CREATE TABLE iceberg.aviation.flights AS SELECT flight_id, flight_no, departure_airport, arrival_airport, "
            "tail_number, CAST(scheduled_departure AS timestamp(6)) AS scheduled_departure, status, delay_minutes, tags "
            "FROM demo.aviation.flights")
        exact(raw)
        metadata_before = collect("before")
        before = identity()
        boundary = len(statements)
        docker("restart", owner + "-minio", owner + "-trino", limit=120)
        port_after = docker("port", owner + "-trino", "8080/tcp")[1].strip()
        require(re.fullmatch(r"127\.0\.0\.1:\d+", port_after), "non-loopback restarted Trino port")
        os.environ["ONTOFORGE_TRINO_URL"] = "http://" + port_after
        wait_for(lambda: helper("list", check=False)[0] == 0)
        wait_for(ready)
        metadata_after = collect("after")
        exact(raw, "-after")
        after = identity()
        require(before == after, "restart changed container or volume identity")
        require(metadata_before == metadata_after, "restart changed table metadata location")
        require(not any(s.startswith("CREATE ") for s in statements[boundary:]), "restart recreated the table")
        write_json(raw / "restart.json", {"before": before, "after": after,
                   "metadata_unchanged": True, "statements_after_restart": statements[boundary:], "ctas_after_restart": 0,
                   "host_port_before": port, "host_port_after": port_after})
        if nightly:
            run_corpus(raw)
    except Failure as error:
        status = error.code
        print(str(error), file=sys.stderr)
    except Exception:
        status = 1
        print("fixture failed; private diagnostics retained until cleanup", file=sys.stderr)
    finally:
        # Capture diagnostics privately, never print unscanned service output.
        logs_complete = True
        for suffix in ("pg", "minio", "trino"):
            if not any(r["name"] == owner + "-" + suffix for r in state["resources"]):
                continue
            try:
                docker("logs", owner + "-" + suffix, limit=5, log=raw / (suffix + ".log"))
            except Failure as error:
                status = status or error.code
                logs_complete = False
            except Exception:
                status = status or 1
                logs_complete = False
        try:
            require(logs_complete, "service log collection failed")
            scan(raw, credentials)
            scan_passed = True
            evidence.mkdir(parents=True)
            for path in raw.iterdir():
                shutil.copy2(path, evidence / path.name)
        except Exception:
            status = status or 1
            print("evidence publication rejected", file=sys.stderr)
        try:
            cleanup_result = cleanup(state)
            if baseline is not None:
                after = snapshot()
                cleanup_result["existing_containers_unchanged"] = all(after.get(k) == v for k, v in baseline.items())
                cleanup_result["catalogs_unchanged"] = hashes == catalog_hashes()
                cleanup_result["ok"] &= cleanup_result["existing_containers_unchanged"] and cleanup_result["catalogs_unchanged"]
        except Failure as error:
            status = status or error.code
            cleanup_result = {"ok": False, "remaining": -1}
        except Exception:
            cleanup_result = {"ok": False, "remaining": -1}
        if not cleanup_result["ok"]:
            status = status or 1
        if cleanup_result["ok"]:
            try:
                if state_path.parent != runtime:
                    state_path.unlink(missing_ok=True)
                shutil.rmtree(runtime)
            except Exception:
                status = status or 1
                cleanup_result["ok"] = False
        try:
            evidence.mkdir(parents=True, exist_ok=True)
            write_json(evidence / "cleanup.json", cleanup_result)
            write_json(evidence / "result.json", {"status": "pass" if status == 0 else "fail", "exit_code": status,
                       "run_id": owner, "trino": version, "scan_passed": scan_passed,
                       "state": str(state_path) if not cleanup_result["ok"] else None})
            print("Iceberg result: " + str(evidence / "result.json"))
        except Exception:
            status = status or 1
            print("result publication failed", file=sys.stderr)
    return status


def helper_test():
    owner = "iceberg-" + secrets.token_hex(12)
    runtime = Path(tempfile.gettempdir()) / owner
    runtime.mkdir(mode=0o700)
    private(runtime, directory=True)
    (runtime / "owner").write_text(owner)
    state_path = Path(os.environ.get("ONTOFORGE_ICEBERG_STATE", str(runtime / "state.json"))).resolve()
    require(not state_path.exists(), "helper state already exists")
    state = {"owner": owner, "runtime": str(runtime), "resources": [],
             "lock_token": os.environ.get("ICEBERG_LOCK_TOKEN")}
    write_json(state_path, state)
    private(state_path)
    status = 0
    try:
        _, metadata, _ = docker("image", "inspect", IMAGE)
        require(json.loads(metadata)[0]["Config"]["Labels"]["org.opencontainers.image.revision"] == SOURCE,
                "helper image source does not match")
        _, identifier, _ = docker("create", "--name", owner, "--label", LABEL + "=" + owner, IMAGE)
        state["resources"].append({"kind": "container", "id": identifier.strip(), "name": owner})
        write_json(state_path, state)
        docker("cp", identifier.strip() + ":/bucket", runtime / "bucket")
        docker("cp", identifier.strip() + ":/SHA256SUMS", runtime / "SHA256SUMS")
        checksums = dict(line.split()[::-1] for line in (runtime / "SHA256SUMS").read_text().splitlines())
        actual = hashlib.sha256((runtime / "bucket").read_bytes()).hexdigest()
        require(actual == checksums["bucket"], "extracted helper checksum mismatch")
        print("verified bucket SHA256 " + actual, flush=True)
        if os.name == "nt":
            _, binary, _ = call(["wsl.exe", "-d", "Ubuntu-26.04", "--", "wslpath", "-u", (runtime / "bucket").as_posix()])
            binary = binary.strip()
            call(["wsl.exe", "-d", "Ubuntu-26.04", "--", "chmod", "700", binary])
            command = ["wsl.exe", "-d", "Ubuntu-26.04", "--cd", ROOT.as_posix(), "--", "env",
                       "ONTOFORGE_BUCKET_BINARY=" + binary, "PYTHONDONTWRITEBYTECODE=1",
                       "python3", "-m", "unittest", "-v", "ci.test_iceberg_services.BucketClientContract"]
        else:
            (runtime / "bucket").chmod(0o700)
            os.environ["ONTOFORGE_BUCKET_BINARY"] = str(runtime / "bucket")
            command = [sys.executable, "-m", "unittest", "-v", "ci.test_iceberg_services.BucketClientContract"]
        status, out, err = call(command, limit=60, check=False)
        print(out + err, end="", flush=True)
        if status == 0:
            require(re.search(r"Ran [1-9][0-9]* tests", out + err) and "skipped" not in out + err,
                    "helper test did not execute all cases")
    except Failure as error:
        status = error.code
        print(str(error), file=sys.stderr)
    except Exception:
        status = 1
        print("helper test failed", file=sys.stderr)
    finally:
        try:
            result = cleanup(state)
            if result["ok"]:
                shutil.rmtree(runtime)
                if state_path.parent != runtime:
                    state_path.unlink(missing_ok=True)
            else:
                status = status or 1
        except Exception:
            status = status or 1
    return status


def main():
    args = sys.argv[1:]
    if len(args) == 3 and args[0] in ("contract", "nightly") and args[1] == "--trino":
        with owner_lock():
            return contract(args[2], nightly=args[0] == "nightly")
    if args == ["helper-test"]:
        with owner_lock():
            return helper_test()
    if len(args) == 2 and args[0] == "exact":
        exact(Path(args[1]))
        return 0
    if len(args) == 2 and args[0] == "validate":
        validate(Path(args[1]))
        return 0
    if len(args) == 2 and args[0] == "permissions":
        private(Path(args[1]), directory=Path(args[1]).is_dir())
        return 0
    if len(args) == 3 and args[0] == "check-artifacts":
        scan(Path(args[1]), json.loads(Path(args[2]).read_text()))
        return 0
    if len(args) == 3 and args[:2] == ["cleanup", "--state"]:
        state_path = Path(args[2])
        if not state_path.exists():
            return 0
        state = json.loads(state_path.read_text())
        result = cleanup(state)
        if result["ok"]:
            runtime = Path(state["runtime"])
            shutil.rmtree(runtime)
            lock = Path(tempfile.gettempdir()) / "ontoforge-iceberg.lock"
            if state.get("lock_token") and lock.exists():
                require((lock / "owner").read_text() == state["lock_token"], "lock owner changed")
                (lock / "owner").unlink()
                lock.rmdir()
            if state_path.parent != runtime:
                state_path.unlink(missing_ok=True)
        print(json.dumps(result))
        return 0 if result["ok"] else 1
    raise Failure("usage: iceberg-services.sh {contract|nightly} --trino {483|484} | helper-test | cleanup --state PATH", 2)


try:
    sys.exit(main())
except Failure as error:
    print(str(error), file=sys.stderr)
    sys.exit(error.code)
except Exception:
    print("fixture command failed", file=sys.stderr)
    sys.exit(1)
PY
child=$!
wait "$child"
