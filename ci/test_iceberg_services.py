"""Black-box contracts for the isolated Iceberg fixture."""

import os
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
