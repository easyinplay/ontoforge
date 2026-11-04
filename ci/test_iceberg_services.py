"""Black-box contracts for the isolated Iceberg fixture."""

import os
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

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

    def run_cli(self, *args):
        return subprocess.run([bash_path(), "--noprofile", "--norc", str(RUNNER), *map(str, args)],
                              cwd=ROOT, env=self.env, capture_output=True, text=True, timeout=15)

    def test_empty_url_cannot_pass(self):
        self.env.update(ONTOFORGE_TRINO_ICEBERG="1", ONTOFORGE_TRINO_URL="")
        self.assertNotEqual(self.run_cli("exact", self.directory).returncode, 0)

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


if __name__ == "__main__":
    unittest.main()
