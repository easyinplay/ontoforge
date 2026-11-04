"""Black-box contracts for the isolated Iceberg fixture."""

import os
import json
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
        class Trino(BaseHTTPRequestHandler):
            def log_message(self, *args):
                pass

            def do_GET(self):
                self.send_response(200)
                self.end_headers()
                self.wfile.write(b'{"starting":false,"nodeVersion":{"version":"483"}}')

        server = ThreadingHTTPServer(("127.0.0.1", 0), Trino)
        threading.Thread(target=server.serve_forever, daemon=True).start()
        self.addCleanup(server.server_close)
        self.addCleanup(server.shutdown)
        self.env["TEST_PORT"] = str(server.server_port)

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
        self.full_fixture("term")
        self.env["TEST_READY"] = str(self.directory / "ready")
        command = 'bash ci/iceberg-services.sh contract --trino 483 & child=$!; '
        command += 'for i in {1..100}; do [[ -f "$TEST_READY" ]] && break; sleep 0.1; done; '
        command += 'kill -TERM "$child"; wait "$child"'
        result = subprocess.run([bash_path(), "--noprofile", "--norc", "-c", command],
                                cwd=ROOT, env=self.env, capture_output=True, text=True, timeout=45)
        self.assertEqual(result.returncode, 143, result.stderr)
        db = json.loads(Path(self.env["TEST_DB"]).read_text())
        self.assertEqual(db["resources"], {}, "TERM must clean resources after stopping the active child")

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
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
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
            output = "29"
        elif "-At" in args:
            output = "s3://warehouse/aviation/flights-id/metadata/a.json"
    elif args[0] == "port":
        output = "127.0.0.1:" + (os.environ["TEST_PORT"] if args[-1] == "8080/tcp" else "15432")
    else:
        code = 88
    path.write_text(json.dumps(db))
    print(output)
    return code


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1].startswith("--fake-"):
        sys.stdout.reconfigure(newline="\n")
    if len(sys.argv) > 1 and sys.argv[1] == "--fake-cleanup":
        sys.exit(fake_cleanup(sys.argv[2:]))
    if len(sys.argv) > 1 and sys.argv[1] == "--fake-fixture":
        sys.exit(fake_fixture(sys.argv[2:]))
    unittest.main()
