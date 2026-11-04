"""Black-box contracts for the isolated Iceberg fixture."""

import os
import ast
import json
import secrets
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from urllib.parse import parse_qs, urlparse

ROOT = Path(__file__).resolve().parent.parent
RUNNER = ROOT / "ci/iceberg-services.sh"


def bash_path():
    if os.name == "nt":
        return str(Path(os.environ.get("ProgramFiles", "C:/Program Files")) / "Git/bin/bash.exe")
    return "bash"


class RunnerContract(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="iceberg-test-")
        self.addCleanup(self.temp.cleanup)
        self.directory = Path(self.temp.name)
        self.env = dict(os.environ)
        self.env.pop("BASH_ENV", None)
        self.env.pop("ENV", None)
        self.env["ONTOFORGE_ICEBERG_EVIDENCE_DIR"] = str(self.directory / "evidence")

    def run_cli(self, *args):
        return subprocess.run([bash_path(), "--noprofile", "--norc", str(RUNNER), *map(str, args)],
                              cwd=ROOT, env=self.env, capture_output=True, text=True, timeout=30)

    def test_empty_url_cannot_pass(self):
        self.env.update(ONTOFORGE_TRINO_ICEBERG="1", ONTOFORGE_TRINO_URL="")
        self.assertNotEqual(self.run_cli("exact", self.directory).returncode, 0)

    def fake_tool(self, name, script):
        directory = self.directory / "bin"
        directory.mkdir(exist_ok=True)
        path = directory / name
        path.write_text("#!/usr/bin/env bash\n" + script, encoding="utf-8")
        path.chmod(0o755)
        self.env["PATH"] = str(directory) + os.pathsep + self.env["PATH"]

    def test_existing_owner_lock_prevents_any_docker_calls(self):
        lock = self.directory / "ontoforge-iceberg.lock"
        lock.mkdir()
        (lock / "owner").write_text("another-run")
        calls = self.directory / "docker-called"
        self.env.update(TMPDIR=str(self.directory), TMP=str(self.directory), TEMP=str(self.directory),
                        TEST_CALLS=str(calls))
        self.fake_tool("docker", 'printf invoked >> "$TEST_CALLS"\nexit 77\n')
        result = self.run_cli("contract", "--trino", "483")
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(calls.exists(), "existing owner must be rejected before touching Docker")
        self.assertEqual((lock / "owner").read_text(), "another-run")

    def test_cleanup_checks_owner_and_preserves_unrelated_resources(self):
        owner = "iceberg-" + "a" * 24
        runtime = self.directory / owner
        runtime.mkdir()
        (runtime / "owner").write_text(owner)
        state = {"owner": owner, "runtime": str(runtime), "resources": [
            {"kind": "container", "id": "owned", "name": "owned"},
            {"kind": "container", "id": "foreign", "name": "foreign"}]}
        path = runtime / "state.json"
        path.write_text(json.dumps(state))
        self.env.update(TMPDIR=str(self.directory), TMP=str(self.directory), TEMP=str(self.directory),
                        TEST_OWNER=owner, TEST_DB=str(self.directory / "resources.json"))
        database = Path(self.env["TEST_DB"])
        database.write_text(json.dumps({"owned": owner, "foreign": "another-run"}))
        self.fake_tool("docker", 'exec "' + sys.executable.replace("\\", "/") + '" "'
                       + Path(__file__).as_posix() + '" --fake-cleanup "$@"\n')
        result = self.run_cli("cleanup", "--state", path)
        self.assertNotEqual(result.returncode, 0, "wrong-owner ledger entry must fail cleanup")
        self.assertEqual(json.loads(database.read_text()), {"foreign": "another-run"})
        state["resources"] = []
        path.write_text(json.dumps(state))
        self.assertEqual(self.run_cli("cleanup", "--state", path).returncode, 0)
        self.assertFalse(runtime.exists(), "independent cleanup must remove owned private files")

    def test_cleanup_failure_is_nonzero_and_resources_remain(self):
        owner = "iceberg-" + "b" * 24
        runtime = self.directory / owner
        runtime.mkdir()
        (runtime / "owner").write_text(owner)
        state = runtime / "state.json"
        state.write_text(json.dumps({"owner": owner, "runtime": str(runtime), "resources": []}))
        self.env.update(TMPDIR=str(self.directory), TMP=str(self.directory), TEMP=str(self.directory),
                        TEST_OWNER=owner, TEST_DB=str(self.directory / "resources.json"), TEST_REMOVE_FAILURE="1")
        database = Path(self.env["TEST_DB"])
        database.write_text(json.dumps({"owned": owner}))
        self.fake_tool("docker", 'exec "' + sys.executable.replace("\\", "/") + '" "'
                       + Path(__file__).as_posix() + '" --fake-cleanup "$@"\n')
        result = self.run_cli("cleanup", "--state", state)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(json.loads(database.read_text()), {"owned": owner})

    def full_fixture(self, mode):
        self.env.update(TMPDIR=str(self.directory), TMP=str(self.directory), TEMP=str(self.directory),
                        TEST_DB=str(self.directory / "resources.json"), TEST_MODE=mode,
                        ONTOFORGE_ICEBERG_READINESS_SECONDS="1")
        Path(self.env["TEST_DB"]).write_text(json.dumps({"resources": {}, "calls": []}))
        self.fake_tool("docker", 'exec "' + sys.executable.replace("\\", "/") + '" "'
                       + Path(__file__).as_posix() + '" --fake-fixture "$@"\n')
        self.fake_tool("cargo", 'case "$*" in *--list*) echo "iceberg_table_is_readable: test";; '
                       '*) echo "test result: ok. 1 passed; 0 failed; 0 ignored;";; esac\n')
        version = self.env.get("TEST_TRINO_VERSION", "483")

        class Trino(BaseHTTPRequestHandler):
            def log_message(self, *args):
                pass

            def do_GET(self):
                if mode == "port-change" and self.server.server_port == server.server_port:
                    if json.loads(Path(self_outer.env["TEST_DB"]).read_text()).get("restarted"):
                        self.send_response(503)
                        self.end_headers()
                        return
                self.send_response(200)
                self.end_headers()
                self.wfile.write(json.dumps({"starting": False, "nodeVersion": {"version": version}}).encode())

        self_outer = self
        server = ThreadingHTTPServer(("127.0.0.1", 0), Trino)
        threading.Thread(target=lambda: server.serve_forever(poll_interval=0.02), daemon=True).start()
        self.addCleanup(server.server_close)
        self.addCleanup(server.shutdown)
        self.env["TEST_PORT"] = str(server.server_port)
        if mode == "port-change":
            second = ThreadingHTTPServer(("127.0.0.1", 0), Trino)
            threading.Thread(target=lambda: second.serve_forever(poll_interval=0.02), daemon=True).start()
            self.addCleanup(second.server_close)
            self.addCleanup(second.shutdown)
            self.env["TEST_PORT_AFTER"] = str(second.server_port)

    def test_restart_refreshes_dynamic_host_port(self):
        self.full_fixture("port-change")
        result = self.run_cli("contract", "--trino", "483")
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_both_versions_restart_without_recreating_the_table(self):
        for version in ("483", "484"):
            with self.subTest(version=version):
                self.env["TEST_TRINO_VERSION"] = version
                self.env["ONTOFORGE_ICEBERG_EVIDENCE_DIR"] = str(self.directory / ("proof-" + version))
                self.full_fixture("success")
                result = self.run_cli("contract", "--trino", version)
                self.assertEqual(result.returncode, 0, result.stderr)
                calls = json.loads(Path(self.env["TEST_DB"]).read_text())["calls"]
                restarts = [i for i, args in enumerate(calls) if args[0] == "restart"]
                self.assertEqual(len(restarts), 1, "contract must restart MinIO and Trino exactly once")
                self.assertEqual(len(calls[restarts[0]][1:]), 2)
                after = calls[restarts[0] + 1:]
                self.assertFalse(any("--execute" in args and args[-1].startswith("CREATE ") for args in after))
                proof = self.directory / ("proof-" + version)
                self.assertEqual(json.loads((proof / "counts-after.json").read_text()), {"source": 29, "copied": 29})
                self.assertEqual(json.loads((proof / "restart.json").read_text())["before"],
                                 json.loads((proof / "restart.json").read_text())["after"])

    def test_restart_cannot_hide_lost_rows(self):
        self.full_fixture("restart-empty")
        result = self.run_cli("contract", "--trino", "483")
        self.assertNotEqual(result.returncode, 0, "empty table after restart must fail")

    def test_unknown_fault_is_rejected_before_creating_resources(self):
        self.full_fixture("success")
        self.env["ONTOFORGE_ICEBERG_FAULT"] = "unknown"
        result = self.run_cli("contract", "--trino", "483")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(json.loads(Path(self.env["TEST_DB"]).read_text())["calls"], [])

    def test_success_requires_complete_logs_and_successful_cleanup(self):
        self.full_fixture("success-logs-fail")
        result = self.run_cli("contract", "--trino", "483")
        self.assertNotEqual(result.returncode, 0, "unreadable service log must fail the whole contract")
        self.assertEqual(json.loads(Path(self.env["TEST_DB"]).read_text())["resources"], {})
        self.assertEqual(json.loads((self.directory / "evidence/result.json").read_text())["status"], "fail")

    def test_result_write_error_preserves_original_exit_and_removes_private_files(self):
        self.full_fixture("partial42-result-fail")
        result = self.run_cli("contract", "--trino", "483")
        self.assertEqual(result.returncode, 42, result.stderr)
        self.assertEqual(json.loads(Path(self.env["TEST_DB"]).read_text())["resources"], {})
        self.assertEqual(list(self.directory.glob("iceberg-*")), [])

    def test_clean_success_publishes_proof_and_labels_every_container(self):
        self.full_fixture("success")
        result = self.run_cli("contract", "--trino", "483")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads((self.directory / "evidence/result.json").read_text())["status"], "pass")
        calls = json.loads(Path(self.env["TEST_DB"]).read_text())["calls"]
        for args in calls:
            if args[0] in ("run", "create"):
                self.assertIn("--label", args, "every container must be recoverable by owner")
            if args[:2] == ["run", "-d"]:
                if "--network-alias" in args and args[args.index("--network-alias") + 1] == "minio":
                    self.assertNotIn("-p", args)
                elif "-p" in args:
                    self.assertTrue(args[args.index("-p") + 1].startswith("127.0.0.1::"))

    def test_successful_operation_with_cleanup_failure_is_failure(self):
        self.full_fixture("success-cleanup-fail")
        result = self.run_cli("contract", "--trino", "483")
        self.assertNotEqual(result.returncode, 0)
        proof = json.loads((self.directory / "evidence/result.json").read_text())
        self.assertEqual(proof["status"], "fail")
        self.assertFalse(json.loads((self.directory / "evidence/cleanup.json").read_text())["ok"])

    def test_secret_in_service_log_blocks_publication(self):
        self.full_fixture("success-secret-log")
        result = self.run_cli("contract", "--trino", "483")
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((self.directory / "evidence/trino.log").exists())
        self.assertFalse((self.directory / "evidence/counts-before.json").exists())
        self.assertEqual(json.loads(Path(self.env["TEST_DB"]).read_text())["resources"], {})

    def test_partial_create_and_cleanup_failure_preserve_original_exit(self):
        self.full_fixture("partial42-cleanup-fail")
        result = self.run_cli("contract", "--trino", "483")
        self.assertEqual(result.returncode, 42, result.stderr)
        public = json.loads((self.directory / "evidence/result.json").read_text())
        self.assertEqual(public["exit_code"], 42)
        self.assertTrue(json.loads(Path(self.env["TEST_DB"]).read_text())["resources"])
        self.assertFalse((self.directory / "ontoforge-iceberg.lock").exists())

    def test_partial_create_without_ledger_is_recovered_by_label(self):
        self.full_fixture("partial42")
        result = self.run_cli("contract", "--trino", "483")
        self.assertEqual(result.returncode, 42, result.stderr)
        self.assertEqual(json.loads(Path(self.env["TEST_DB"]).read_text())["resources"], {})
        self.assertEqual(json.loads((self.directory / "evidence/cleanup.json").read_text())["remaining"], 0)

    def test_readiness_timeout_cleans_only_owned_resources(self):
        self.full_fixture("readiness-timeout")
        started = time.monotonic()
        result = self.run_cli("contract", "--trino", "483")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("service readiness timeout", result.stderr)
        self.assertLess(time.monotonic() - started, 25)
        self.assertEqual(json.loads(Path(self.env["TEST_DB"]).read_text())["resources"], {})

    def test_term_reaps_child_and_cleans_resources(self):
        self.check_term("term")

    def test_term_during_log_collection_cannot_publish_pass(self):
        self.check_term("term-logs")
        self.assertEqual(json.loads((self.directory / "evidence/result.json").read_text())["exit_code"], 143)

    def check_term(self, mode):
        self.full_fixture(mode)
        self.env["TEST_READY"] = str(self.directory / "ready")
        command = 'bash ci/iceberg-services.sh contract --trino 483 & child=$!; '
        command += 'for i in {1..300}; do [[ -f "$TEST_READY" ]] && break; sleep 0.1; done; '
        command += 'kill -TERM "$child"; wait "$child"'
        result = subprocess.run([bash_path(), "--noprofile", "--norc", "-c", command],
                                cwd=ROOT, env=self.env, capture_output=True, text=True, timeout=45)
        self.assertEqual(result.returncode, 143, result.stderr)
        self.assertTrue(Path(self.env["TEST_READY"]).exists(), "signal must reach the named operation phase")
        db = json.loads(Path(self.env["TEST_DB"]).read_text())
        self.assertEqual(db["resources"], {}, "TERM must clean resources after stopping the active child")

    def test_name_conflict_never_deletes_the_other_owner(self):
        self.full_fixture("name-conflict")
        result = self.run_cli("contract", "--trino", "483")
        self.assertNotEqual(result.returncode, 0)
        resources = json.loads(Path(self.env["TEST_DB"]).read_text())["resources"]
        self.assertEqual(len(resources), 1)
        self.assertEqual(next(iter(resources.values()))["owner"], "foreign-owner")

    def test_scan_rejects_secret_empty_credentials_and_missing_input(self):
        raw = self.directory / "raw"
        raw.mkdir()
        log = raw / "operation.log"
        log.write_text("safe operation")
        credentials = self.directory / "secrets.json"
        credentials.write_text(json.dumps(["test-pg", "test-access", "test-secret"]))
        self.assertEqual(self.run_cli("check-artifacts", raw, credentials).returncode, 0)
        log.write_text("test-secret")
        rejected = self.run_cli("check-artifacts", raw, credentials)
        self.assertNotEqual(rejected.returncode, 0)
        self.assertNotIn("test-secret", rejected.stdout + rejected.stderr)
        log.write_text("safe operation")
        credentials.write_text(json.dumps(["", "test-access", "test-secret"]))
        self.assertNotEqual(self.run_cli("check-artifacts", raw, credentials).returncode, 0)
        credentials.unlink()
        self.assertNotEqual(self.run_cli("check-artifacts", raw, credentials).returncode, 0)

    def test_private_acl_trusted_owner_and_child_are_accepted(self):
        result = self.run_cli("permissions", self.directory)
        self.assertEqual(result.returncode, 0, result.stderr)
        child = self.directory / "private.txt"
        child.write_text("nonsecret test data")
        result = self.run_cli("permissions", child)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_private_acl_rejects_untrusted_explicit_access(self):
        if os.name != "nt":
            self.directory.chmod(0o777)
            self.assertEqual(self.run_cli("permissions", self.directory).returncode, 0)
            self.assertEqual(self.directory.stat().st_mode & 0o077, 0)
            return
        for sid in ("S-1-1-0", "S-1-5-32-545", "S-1-5-11"):
            with self.subTest(sid=sid):
                directory = self.directory / sid
                directory.mkdir()
                grant = subprocess.run(["icacls", str(directory), "/grant", "*" + sid + ":(OI)(CI)R"],
                                       capture_output=True, text=True)
                self.assertEqual(grant.returncode, 0, grant.stderr)
                self.assertNotEqual(self.run_cli("permissions", directory).returncode, 0)

    def test_exact_discovery_and_execution_are_both_required(self):
        fake = self.directory / "bin"
        fake.mkdir()
        cargo = fake / "cargo"
        cargo.write_text('#!/usr/bin/env bash\nprintf "%s\\n" "$*" >> "$TEST_CALLS"\n'
                         'case "$*" in *--list*) printf "%s\\n" "$TEST_LIST";; '
                         '*) printf "%s\\n" "$TEST_RESULT";; esac\n', encoding="utf-8")
        cargo.chmod(0o755)
        self.env.update(PATH=str(fake) + os.pathsep + self.env["PATH"],
                        ONTOFORGE_TRINO_ICEBERG="1", ONTOFORGE_TRINO_URL="http://127.0.0.1:1234",
                        TEST_CALLS=str(self.directory / "calls"))
        for listing, summary, allowed, executions in [
            ("iceberg_table_is_readable: test\n\n1 test, 0 benchmarks", "test result: ok. 1 passed; 0 failed; 0 ignored;", True, 2),
            ("0 tests, 0 benchmarks", "test result: ok. 1 passed; 0 failed; 0 ignored;", False, 1),
            ("iceberg_table_is_readable: test\nother: test\n2 tests, 0 benchmarks", "", False, 1),
            ("iceberg_table_is_readable: test\n1 test, 0 benchmarks", "test result: ok. 0 passed; 0 failed; 1 ignored;", False, 2),
            ("iceberg_table_is_readable: test\n1 test, 0 benchmarks", "test result: ok. 0 passed; 0 failed; 0 ignored;", False, 2),
        ]:
            with self.subTest(listing=listing, summary=summary):
                calls = self.directory / "calls"
                calls.unlink(missing_ok=True)
                self.env.update(TEST_LIST=listing, TEST_RESULT=summary)
                result = self.run_cli("exact", self.directory)
                self.assertEqual(result.returncode == 0, allowed, result.stdout + result.stderr)
                self.assertTrue(calls.exists(), "runner must actually invoke Cargo discovery")
                self.assertEqual(len(calls.read_text().splitlines()), executions)

    def test_contract_evidence_requires_counts_s3_and_nonempty_objects(self):
        fixtures = {
            "counts-before.json": {"source": 29, "copied": 29},
            "objects-before.json": [{"key": "aviation/flights-id/metadata/00001.json", "size": 43},
                                    {"key": "aviation/flights-id/data/00001.parquet", "size": 80}],
        }
        for name, content in fixtures.items():
            (self.directory / name).write_text(json.dumps(content))
        location = self.directory / "metadata-locations.txt"
        location.write_text("s3://warehouse/aviation/flights-id/metadata/00001.json\n")
        self.assertEqual(self.run_cli("validate", self.directory).returncode, 0)
        for filename, bad in [("counts-before.json", {"source": 28, "copied": 29}),
                              ("counts-before.json", {"source": 29, "copied": 0}),
                              ("objects-before.json", []),
                              ("objects-before.json", [{"key": "unrelated/data/file", "size": 3}])]:
            with self.subTest(filename=filename, bad=bad):
                (self.directory / filename).write_text(json.dumps(bad))
                self.assertNotEqual(self.run_cli("validate", self.directory).returncode, 0)
                (self.directory / filename).write_text(json.dumps(fixtures[filename]))
        location.write_text("local:///warehouse/metadata.json\n")
        self.assertNotEqual(self.run_cli("validate", self.directory).returncode, 0)

    def test_missing_iceberg_environment_cannot_pass(self):
        env = dict(os.environ)
        env.pop("ONTOFORGE_TRINO_ICEBERG", None)
        env.pop("ONTOFORGE_TRINO_URL", None)
        with tempfile.TemporaryDirectory() as directory:
            result = subprocess.run(
                [bash_path(), "--noprofile", "--norc", str(RUNNER), "exact", directory],
                cwd=ROOT, env=env, capture_output=True, text=True, timeout=10,
            )
        self.assertNotEqual(result.returncode, 0, "missing Iceberg environment must fail before Cargo")


class LiveFaultContract(unittest.TestCase):
    def setUp(self):
        self.assertEqual(os.environ.get("ONTOFORGE_ICEBERG_LIVE"), "1", "live Docker tests require explicit opt-in")

    def snapshot(self):
        ids = subprocess.check_output(["docker", "ps", "-aq"], text=True).split()
        if not ids:
            return {}
        data = json.loads(subprocess.check_output(["docker", "inspect", *ids], text=True))
        return {o["Id"]: {"name": o["Name"], "status": o["State"]["Status"], "started": o["State"]["StartedAt"]}
                for o in data}

    def check_fault(self, fault, expected):
        baseline = self.snapshot()
        evidence = ROOT / "target/phase072-minio/faults" / (fault + "-" + secrets.token_hex(6))
        with tempfile.TemporaryDirectory(prefix="iceberg-live-") as directory:
            state_path = Path(directory) / "state.json"
            pid_path = Path(directory) / "pid"
            env = dict(os.environ, ONTOFORGE_ICEBERG_STATE=str(state_path),
                       ONTOFORGE_ICEBERG_EVIDENCE_DIR=str(evidence), ONTOFORGE_ICEBERG_READINESS_SECONDS="2",
                       ICEBERG_TEST_PID=str(pid_path))
            env.pop("ONTOFORGE_ICEBERG_FAULT", None)
            if fault != "term":
                env["ONTOFORGE_ICEBERG_FAULT"] = fault
            command = 'bash ci/iceberg-services.sh contract --trino 483 & child=$!; '
            command += 'printf "%s" "$child" > "$ICEBERG_TEST_PID"; wait "$child"'
            process = subprocess.Popen([bash_path(), "--noprofile", "--norc", "-c", command],
                                       cwd=ROOT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            state, signalled = None, False
            deadline = time.monotonic() + 180
            try:
                while process.poll() is None:
                    self.assertLess(time.monotonic(), deadline, "live fault exceeded deadline")
                    try:
                        state = json.loads(state_path.read_text())
                    except (FileNotFoundError, json.JSONDecodeError):
                        pass
                    if fault == "term" and not signalled and state and any(
                            r["kind"] == "container" and r["name"].endswith("-pg") for r in state["resources"]):
                        subprocess.run([bash_path(), "--noprofile", "--norc", "-c",
                                        'kill -TERM "$(cat "$ICEBERG_TEST_PID")"'], env=env, check=True)
                        signalled = True
                    time.sleep(0.1)
                out, err = process.communicate(timeout=10)
                self.assertEqual(process.returncode, expected, out + err)
                self.assertIsNotNone(state, "run must publish owner before starting resources")
                if fault == "term":
                    self.assertTrue(signalled, "TERM must be sent after actual PG creation")
                proof = json.loads((evidence / "result.json").read_text())
                clean = json.loads((evidence / "cleanup.json").read_text())
                self.assertEqual(proof["exit_code"], expected)
                self.assertTrue(clean["ok"])
                self.assertEqual(clean["remaining"], 0)
                self.assertFalse(Path(state["runtime"]).exists())
                self.assertFalse(state_path.exists())
                for args in (["ps", "-aq"], ["network", "ls", "-q"], ["volume", "ls", "-q"]):
                    remaining = subprocess.check_output(["docker", *args, "--filter",
                                "label=ontoforge.iceberg.owner=" + state["owner"]], text=True)
                    self.assertEqual(remaining.strip(), "")
                self.assertEqual(self.snapshot(), baseline)
                (evidence / "fault-verification.json").write_text(json.dumps({"fault": fault,
                    "exit_code": process.returncode, "resources_before": baseline, "resources_after": self.snapshot(),
                    "private_runtime_removed": True, "owner": state["owner"], "injected": True}, indent=2))
            finally:
                if process.poll() is None:
                    subprocess.run([bash_path(), "--noprofile", "--norc", "-c",
                                    'kill -TERM "$(cat "$ICEBERG_TEST_PID")"'], env=env, check=False)
                    process.communicate(timeout=70)
                if state_path.exists():
                    subprocess.run([bash_path(), str(RUNNER), "cleanup", "--state", str(state_path)],
                                   capture_output=True, timeout=70)

    def test_after_pg_failure_leaves_no_owner_resources(self):
        self.check_fault("after-pg", 42)

    def test_readiness_timeout_leaves_no_owner_resources(self):
        self.check_fault("readiness-timeout", 1)

    def test_term_after_creation_leaves_no_owner_resources(self):
        self.check_fault("term", 143)


class NightlyContract(unittest.TestCase):
    setUp = RunnerContract.setUp
    run_cli = RunnerContract.run_cli
    fake_tool = RunnerContract.fake_tool
    full_fixture = RunnerContract.full_fixture

    def nightly_fixture(self, mode="success"):
        self.full_fixture("success")
        self.env["TEST_CARGO_MODE"] = mode
        self.fake_tool("cargo", 'exec "' + sys.executable.replace("\\", "/") + '" "'
                       + Path(__file__).as_posix() + '" --fake-cargo "$@"\n')

    def test_nightly_keeps_every_existing_target_filter_and_feature(self):
        self.nightly_fixture()
        result = self.run_cli("nightly", "--trino", "483")
        self.assertEqual(result.returncode, 0, result.stderr)
        calls = json.loads(Path(self.env["TEST_DB"]).read_text())["cargo_calls"]
        expected = [
            ["-p", "ontoforge-storage", "--features", "test-support", "--test", "conformance_pg", "--locked", "action_atomic"],
            ["-p", "ontoforge-action", "--features", "test-support", "--test", "action_pg", "--locked"],
            ["-p", "ontoforge-action", "--features", "test-support", "--test", "idempotency_pg", "--locked"],
            ["-p", "ontoforge-action", "--test", "outbox_pg", "--locked"],
            ["-p", "ontoforge-action", "--test", "trino_action_it", "--locked"],
            ["-p", "ontoforge-action", "--test", "outbox", "--test", "entrypoints", "--locked"],
            ["-p", "ontoforge-engine", "--test", "trino_it", "--locked"],
            ["-p", "ontoforge", "--test", "cli_trino_it", "--locked"],
        ]
        for args in expected:
            selected = [call for call in calls if call[1:len(args)+1] == args and call[len(args)+1] == "--"]
            self.assertEqual(len(selected), 2, args)
            self.assertIn("--list", selected[0])
            self.assertNotIn("--list", selected[1])
        corpus = json.loads((self.directory / "evidence/corpus.json").read_text())
        self.assertEqual(len(corpus), 8)
        self.assertTrue(all(row["passed"] > 0 and row["failed"] == 0 and row["ignored"] == 0 for row in corpus))

    def test_corpus_zero_discovery_zero_execution_and_failure_cannot_pass(self):
        for mode in ("zero-discovery", "zero-execution", "failure"):
            with self.subTest(mode=mode):
                self.env["ONTOFORGE_ICEBERG_EVIDENCE_DIR"] = str(self.directory / mode)
                self.nightly_fixture(mode)
                result = self.run_cli("nightly", "--trino", "483")
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(json.loads(Path(self.env["TEST_DB"]).read_text())["resources"], {})

    def test_manual_failure_never_enables_report_job(self):
        workflow = (ROOT / ".github/workflows/nightly.yml").read_text()
        report = workflow.split("  report-failure:", 1)[1]
        expression = next(line.split("if:", 1)[1].strip() for line in report.splitlines() if line.strip().startswith("if:"))
        for event, failed, expected in [("workflow_dispatch", True, False), ("workflow_dispatch", False, False),
                                        ("schedule", False, False), ("schedule", True, True)]:
            with self.subTest(event=event, failed=failed):
                code = expression.removeprefix("${{").removesuffix("}}").strip()
                code = code.replace("github.event_name", repr(event)).replace("failure()", repr(failed)).replace("&&", "and")
                parsed = ast.parse(code, mode="eval")
                self.assertTrue(all(isinstance(n, (ast.Expression, ast.BoolOp, ast.And, ast.Compare, ast.Eq, ast.Constant))
                                    for n in ast.walk(parsed)))
                self.assertEqual(eval(compile(parsed, "workflow-condition", "eval"), {"__builtins__": {}}), expected)

    def test_missing_cleanup_state_is_safe_noop(self):
        result = self.run_cli("cleanup", "--state", self.directory / "missing.json")
        self.assertEqual(result.returncode, 0, result.stderr)


class BucketClientContract(unittest.TestCase):
    def setUp(self):
        self.binary = os.environ["ONTOFORGE_BUCKET_BINARY"]
        self.mode = "empty"
        self.requests = []
        parent = self

        class S3(BaseHTTPRequestHandler):
            def log_message(self, *args):
                pass

            def respond(self, code, body):
                self.send_response(code)
                self.send_header("Content-Type", "application/xml")
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                try:
                    self.wfile.write(body)
                except (BrokenPipeError, ConnectionResetError):
                    pass

            def do_PUT(self):
                parent.requests.append(self.path)
                self.respond(200, b"")

            def do_GET(self):
                parent.requests.append(self.path)
                query = parse_qs(urlparse(self.path).query)
                if parent.mode == "slow":
                    time.sleep(3)
                if parent.mode == "error" or (parent.mode == "stream-error" and "continuation-token" in query):
                    body = ("<Error><Code>AccessDenied</Code><Message>" + parent.env["AWS_ACCESS_KEY_ID"]
                            + " " + parent.env["AWS_SECRET_ACCESS_KEY"] + "</Message></Error>").encode()
                    self.respond(500 if parent.mode == "error" else 403, body)
                    return
                keys = []
                more = "false"
                token = ""
                if parent.mode in ("pages", "stream-error"):
                    if "continuation-token" in query:
                        keys = [("flights/data/a.parquet", 80)]
                    else:
                        keys = [("flights/metadata/a.json", 43)]
                        more, token = "true", "<NextContinuationToken>next</NextContinuationToken>"
                body = '<ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/"><Name>warehouse</Name>'
                body += "<IsTruncated>" + more + "</IsTruncated>" + token
                for key, size in keys:
                    body += "<Contents><Key>" + key + "</Key><Size>" + str(size) + "</Size></Contents>"
                self.respond(200, (body + "</ListBucketResult>").encode())

        self.server = ThreadingHTTPServer(("127.0.0.1", 0), S3)
        self.thread = threading.Thread(target=lambda: self.server.serve_forever(poll_interval=0.02), daemon=True)
        self.thread.start()
        self.env = dict(os.environ, S3_ENDPOINT="http://127.0.0.1:" + str(self.server.server_port),
                        AWS_ACCESS_KEY_ID="fixture-access-7bca", AWS_SECRET_ACCESS_KEY="fixture-secret-eaa7",
                        AWS_REGION="us-east-1", BUCKET_DEADLINE_SECONDS="1")

    def tearDown(self):
        self.server.shutdown()
        self.server.server_close()
        self.thread.join(timeout=2)

    def invoke(self, *args):
        result = subprocess.run([self.binary, *args], env=self.env, capture_output=True, text=True, timeout=5)
        for name in ("AWS_ACCESS_KEY_ID", "AWS_SECRET_ACCESS_KEY"):
            if self.env[name]:
                self.assertNotIn(self.env[name], result.stdout + result.stderr)
        return result

    def test_make_bucket_uses_real_sdk_request(self):
        result = self.invoke("make", "--bucket", "warehouse")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.requests, ["/warehouse/"])

    def test_empty_list_is_json_array(self):
        result = self.invoke("list", "--bucket", "warehouse")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout), [])

    def test_list_consumes_both_pages(self):
        self.mode = "pages"
        result = self.invoke("list", "--bucket", "warehouse")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout), [{"key": "flights/metadata/a.json", "size": 43},
                                                   {"key": "flights/data/a.parquet", "size": 80}])
        self.assertEqual(len(self.requests), 2)

    def test_sdk_error_never_prints_response_credentials(self):
        self.mode = "error"
        self.assertNotEqual(self.invoke("list", "--bucket", "warehouse").returncode, 0)

    def test_midstream_error_does_not_publish_partial_objects(self):
        self.mode = "stream-error"
        result = self.invoke("list", "--bucket", "warehouse")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stdout, "")
        self.assertGreaterEqual(len(self.requests), 2)

    def test_deadline_bounds_a_stalled_request(self):
        self.mode = "slow"
        started = time.monotonic()
        self.assertNotEqual(self.invoke("list", "--bucket", "warehouse").returncode, 0)
        self.assertLess(time.monotonic() - started, 2.5)

    def test_empty_credentials_and_userinfo_are_rejected_before_network(self):
        self.env["AWS_ACCESS_KEY_ID"] = ""
        self.assertNotEqual(self.invoke("list", "--bucket", "warehouse").returncode, 0)
        self.env["AWS_ACCESS_KEY_ID"] = "fixture-access-7bca"
        self.env["S3_ENDPOINT"] = self.env["S3_ENDPOINT"].replace("http://", "http://user:password@")
        self.assertNotEqual(self.invoke("list", "--bucket", "warehouse").returncode, 0)
        self.assertEqual(self.requests, [])

    def test_argument_error_never_prints_credentials(self):
        self.assertNotEqual(self.invoke("list", "--" + self.env["AWS_SECRET_ACCESS_KEY"]).returncode, 0)


def fake_cleanup(args):
    path = Path(os.environ["TEST_DB"])
    resources = json.loads(path.read_text())
    if args[:1] == ["ps"]:
        assert "label=ontoforge.iceberg.owner=" + os.environ["TEST_OWNER"] in args
        print("\n".join(k for k, v in resources.items() if v == os.environ["TEST_OWNER"]))
    elif args[:1] == ["inspect"]:
        if args[1] not in resources:
            return 1
        print(json.dumps([{"Id": args[1], "Config": {"Labels": {"ontoforge.iceberg.owner": resources[args[1]]}}}]))
    elif args[:1] == ["rm"]:
        if os.environ.get("TEST_REMOVE_FAILURE"):
            return 42
        del resources[args[-1]]
        path.write_text(json.dumps(resources))
    elif args[0] in ("network", "volume") and args[1] == "ls":
        pass
    else:
        return 88
    return 0


def fake_fixture(args):
    path = Path(os.environ["TEST_DB"])
    db = json.loads(path.read_text())
    db["calls"].append(args)
    resources = db["resources"]
    mode = os.environ["TEST_MODE"]
    code, output = 0, ""
    if args[0] == "build":
        pass
    elif args[:2] == ["image", "inspect"]:
        output = json.dumps([{"Id": "sha256:fixture", "Config": {"Labels": {
            "org.opencontainers.image.revision": "9e49d5e7a648f00e26f2246f4dc28e6b07f8c84a"}}}])
    elif args[0] == "ps" or args[:2] in (["network", "ls"], ["volume", "ls"]):
        kind = "container" if args[0] == "ps" else args[0]
        label = args[args.index("--filter") + 1] if "--filter" in args else None
        if label:
            assert label.startswith("label=ontoforge.iceberg.owner=")
        output = "\n".join(k for k, v in resources.items() if v["kind"] == kind
                           and (label is None or label.endswith("=" + v["owner"])))
    elif args[0] == "inspect" or (args[0] in ("network", "volume") and args[1] == "inspect"):
        ids = args[1:] if args[0] == "inspect" else args[2:]
        result = []
        for identifier in ids:
            if identifier not in resources:
                code = 1
                break
            r = resources[identifier]
            labels = {"ontoforge.iceberg.owner": r["owner"]}
            result.append({"Id": identifier, "Name": identifier, "Config": {"Labels": labels}, "Labels": labels,
                           "State": {"Status": "running", "StartedAt": "fixture-start"}})
        output = json.dumps(result)
    elif args[0] == "run" and "--rm" in args:
        output = "DEVELOPMENT fixture version"
        if "list" in args:
            output = json.dumps([{"key": "aviation/flights-id/metadata/a.json", "size": 30},
                                 {"key": "aviation/flights-id/data/a.parquet", "size": 90}])
    elif args[0] in ("run", "create") or args[:2] in (["network", "create"], ["volume", "create"]):
        if mode == "term" and args[:2] == ["network", "create"]:
            Path(os.environ["TEST_READY"]).write_text("ready")
            time.sleep(30)
        kind = "container" if args[0] in ("run", "create") else args[0]
        name = args[args.index("--name") + 1] if "--name" in args else args[-1]
        owner = args[args.index("--label") + 1].split("=", 1)[1]
        resources[name] = {"kind": kind, "owner": owner}
        output = name
        if mode == "name-conflict":
            resources[name]["owner"] = "foreign-owner"
            code = 9
        if mode.startswith("partial42") and args[0] == "create":
            code = 42
        if mode == "partial42-result-fail":
            (Path(os.environ["ONTOFORGE_ICEBERG_EVIDENCE_DIR"]) / "result.json").mkdir(parents=True, exist_ok=True)
    elif args[0] == "rm" or args[:2] in (["network", "rm"], ["volume", "rm"]):
        if mode.endswith("cleanup-fail"):
            code = 9
        else:
            resources.pop(args[-1], None)
    elif args[0] == "cp":
        Path(args[-1]).write_text("fixture-minio  minio\nfixture-bucket  bucket\n")
    elif args[0] == "logs":
        if mode == "term-logs" and args[1].endswith("-trino"):
            Path(os.environ["TEST_READY"]).write_text("ready")
            time.sleep(30)
        output = "fixture service log"
        if mode == "success-logs-fail":
            code = 7
        if mode == "success-secret-log":
            owner = resources[args[1]]["owner"]
            output = json.loads((Path(tempfile.gettempdir()) / owner / "secrets.json").read_text())[1]
    elif args[0] == "exec" and "pg_isready" in args:
        db["readiness_calls"] = db.get("readiness_calls", 0) + 1
        code = 1 if mode == "readiness-timeout" and db["readiness_calls"] < 3 else 0
    elif args[0] == "exec":
        if "--execute" in args and "count(*)" in args[-1]:
            output = "0" if mode == "restart-empty" and db.get("restarted") else "29"
        elif "-At" in args:
            output = "s3://warehouse/aviation/flights-id/metadata/a.json"
    elif args[0] == "port":
        port = os.environ.get("TEST_PORT_AFTER", os.environ["TEST_PORT"]) if db.get("restarted") else os.environ["TEST_PORT"]
        output = "127.0.0.1:" + (port if args[-1] == "8080/tcp" else "15432")
    elif args[0] == "restart":
        db["restarted"] = True
    else:
        code = 88
    path.write_text(json.dumps(db))
    print(output)
    return code


def fake_cargo(args):
    path = Path(os.environ["TEST_DB"])
    db = json.loads(path.read_text())
    db.setdefault("cargo_calls", []).append(args)
    path.write_text(json.dumps(db))
    exact = "iceberg_table_is_readable" in args
    mode = os.environ.get("TEST_CARGO_MODE", "success")
    if "--list" in args:
        if exact:
            print("iceberg_table_is_readable: test")
        elif mode != "zero-discovery":
            print("first_case: test\nsecond_case: test")
        return 0
    if not exact and mode == "failure":
        print("test result: FAILED. 0 passed; 1 failed; 0 ignored;")
        return 42
    passed = 1 if exact else (0 if mode == "zero-execution" else 2)
    print(f"test result: ok. {passed} passed; 0 failed; 0 ignored;")
    return 0


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1].startswith("--fake-"):
        sys.stdout.reconfigure(newline="\n")
    if len(sys.argv) > 1 and sys.argv[1] == "--fake-cleanup":
        sys.exit(fake_cleanup(sys.argv[2:]))
    if len(sys.argv) > 1 and sys.argv[1] == "--fake-fixture":
        sys.exit(fake_fixture(sys.argv[2:]))
    if len(sys.argv) > 1 and sys.argv[1] == "--fake-cargo":
        sys.exit(fake_cargo(sys.argv[2:]))
    unittest.main()
