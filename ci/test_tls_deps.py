"""Exercise the CI TLS guard with hermetic Cargo output on Windows and Linux."""

import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import textwrap
import unittest

ROOT = Path(__file__).resolve().parent.parent
TREE_ARGS = (
    "tree --workspace --locked --features ontoforge-storage/embedded-pg "
    "-e normal,build,dev --prefix none"
)


def bash_path():
    if os.name == "nt":
        git = shutil.which("git")
        candidates = [Path(git).parent.parent / "bin/bash.exe"] if git else []
        candidates.append(Path(os.environ.get("ProgramFiles", "C:/Program Files"))
                          / "Git/bin/bash.exe")
        for candidate in candidates:
            if candidate.is_file():
                return str(candidate)
        raise RuntimeError("Git Bash is required; Windows System32 bash is WSL")
    bash = shutil.which("bash")
    if not bash:
        raise RuntimeError("Bash is required")
    return bash


def workflow_command():
    workflow = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
    step = re.search(
        r"^      - name: No OpenSSL or native-tls in the dependency graph\n"
        r"(?P<body>(?:^        .*\n|^\n)+)", workflow, re.MULTILINE,
    )
    if step is None:
        raise AssertionError("CI TLS guard step is missing")
    run = re.search(r"^        run: (.*)\n", step["body"], re.MULTILINE)
    if run is None:
        raise AssertionError("CI TLS guard command is missing")
    if run[1] == "|":
        return textwrap.dedent(step["body"][run.end():])
    return run[1]


# The inverse-query branch emulates the old workflow for the RED reproduction.
# No real Cargo executable is called, even when the workflow launches a helper.
FAKE_CARGO = r"""
cargo() {
  printf '%s\n' "$*" >> "$TLS_TEST_CALLS"
  if [ "$TLS_TEST_FAILURE" = 1 ]; then
    echo 'cargo tree fixture failure' >&2
    return 101
  fi
  case "$*" in
    *'-i openssl') [ "$TLS_TEST_OPENSSL" = 1 ]; return ;;
    *'-i native-tls') [ "$TLS_TEST_NATIVE_TLS" = 1 ]; return ;;
  esac
  printf '%s\n' "$TLS_TEST_TREE"
}
export -f cargo
"""


class TlsDependencyTests(unittest.TestCase):
    def check_guard(self, tree, allowed, *, openssl=False, native_tls=False, failure=False):
        with tempfile.TemporaryDirectory(prefix="ontoforge-tls-") as directory:
            calls = Path(directory) / "calls"
            env = os.environ.copy()
            env.pop("BASH_ENV", None)
            env.pop("ENV", None)
            env.update(
                TLS_TEST_CALLS=calls.as_posix(), TLS_TEST_TREE=tree,
                TLS_TEST_FAILURE=str(int(failure)), TLS_TEST_OPENSSL=str(int(openssl)),
                TLS_TEST_NATIVE_TLS=str(int(native_tls)),
            )
            result = subprocess.run(
                [bash_path(), "--noprofile", "--norc", "-e", "-o", "pipefail", "-c",
                 FAKE_CARGO + workflow_command()],
                cwd=ROOT, env=env, capture_output=True, text=True, timeout=20,
            )
            self.assertEqual(result.returncode == 0, allowed, result.stdout + result.stderr)
            self.assertEqual(calls.read_text(encoding="utf-8").splitlines(), [TREE_ARGS])

    def test_neither_package_is_allowed(self):
        self.check_guard("ontoforge v0.1.0\nrustls v0.23.0", True)

    def test_openssl_only_is_rejected(self):
        self.check_guard("ontoforge v0.1.0\nopenssl v0.10.0\nrustls v0.23.0", False,
                         openssl=True)

    def test_native_tls_only_is_rejected(self):
        self.check_guard("ontoforge v0.1.0\nnative-tls v0.2.0 (*)", False, native_tls=True)

    def test_both_packages_are_rejected(self):
        self.check_guard("openssl v0.10.0\nnative-tls v0.2.0", False,
                         openssl=True, native_tls=True)

    def test_cargo_failure_is_rejected(self):
        self.check_guard("", False, failure=True)

    def test_similar_package_names_are_allowed(self):
        self.check_guard("openssl-probe v0.1.0\nopenssl-sys v0.9.0\n"
                         "native-tls-extra v0.1.0\nmy-openssl v0.1.0\n"
                         "ontoforge v0.1.0 (/workspace/openssl)", True)


if __name__ == "__main__":
    unittest.main()
