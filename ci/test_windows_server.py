"""Real-process contracts for the Windows CI server owner; no Docker required."""

import ctypes
from ctypes import wintypes
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import unittest
import urllib.request

ROOT = Path(__file__).resolve().parent.parent
HELPER = ROOT / "ci/windows-server.py"


def eventually(predicate, timeout=10):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        value = predicate()
        if value:
            return value
        time.sleep(0.05)
    raise AssertionError("condition did not become true before deadline")


def read_state(path):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (FileNotFoundError, json.JSONDecodeError):
        return None


def process_handle(pid):
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
    kernel.OpenProcess.restype = wintypes.HANDLE
    kernel.WaitForSingleObject.argtypes = [wintypes.HANDLE, wintypes.DWORD]
    kernel.WaitForSingleObject.restype = wintypes.DWORD
    kernel.CloseHandle.argtypes = [wintypes.HANDLE]
    handle = kernel.OpenProcess(0x100000, False, pid)
    return kernel, handle


@unittest.skipUnless(os.name == "nt", "Windows private-console integration")
class WindowsServerTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="ontoforge-console-")
        self.path = Path(self.directory.name)
        self.state = self.path / "server.json"
        self.owners = []

    def tearDown(self):
        for owner in self.owners:
            if owner.poll() is None:
                owner.kill()
            owner.wait(timeout=5)
            owner.stdout.close()
            owner.stderr.close()
        self.directory.cleanup()

    def start(self, command):
        owner = subprocess.Popen(
            [sys.executable, str(HELPER), "run", "--state", str(self.state),
             "--log", str(self.path / "server.log"), "--", *command],
            stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            text=True, creationflags=subprocess.CREATE_NO_WINDOW,
        )
        self.owners.append(owner)
        eventually(lambda: read_state(self.state) or owner.poll() is not None)
        state = read_state(self.state)
        self.assertIsNotNone(state, "owner must publish process identity before accepting stop")
        return owner, state

    def fixture(self, mode):
        ready = self.path / "ready"
        owner, state = self.start([sys.executable, str(Path(__file__).resolve()),
                                   "--child", mode, str(ready)])
        eventually(ready.exists)
        return owner, state

    def stop(self, timeout=30):
        return subprocess.run(
            [sys.executable, str(HELPER), "stop", "--state", str(self.state),
             "--timeout", str(timeout)],
            stdin=subprocess.DEVNULL, capture_output=True, text=True,
            creationflags=subprocess.CREATE_NO_WINDOW, timeout=timeout + 5,
        )

    def test_real_binary_graceful_exit_zero_and_port_closed(self):
        binary = self.path / "ontoforge.exe"
        shutil.copy2(ROOT / "target/debug/ontoforge.exe", binary)
        with socket.socket() as listener:
            listener.bind(("127.0.0.1", 0))
            port = listener.getsockname()[1]
        owner, _ = self.start([str(binary), "serve", "--dev", "--data-dir",
                               str(self.path / "data"), "--bind", f"127.0.0.1:{port}"])

        def healthy():
            try:
                with urllib.request.urlopen(f"http://127.0.0.1:{port}/health", timeout=0.2) as response:
                    return response.status == 200
            except OSError:
                return False

        eventually(healthy)
        stopped = self.stop()
        self.assertEqual(stopped.returncode, 0, stopped.stderr)
        self.assertEqual(owner.wait(timeout=5), 0)
        with socket.socket() as check:
            self.assertNotEqual(check.connect_ex(("127.0.0.1", port)), 0)
        self.assertEqual(read_state(self.state)["exit_code"], 0)

    def test_nonzero_child_exit_is_preserved_for_owner_and_stop(self):
        for code in (17, 130, 137, 256):
            with self.subTest(code=code):
                self.state = self.path / f"server-{code}.json"
                owner, _ = self.start([sys.executable, "-c", f"import sys; sys.exit({code})"])
                self.assertEqual(owner.wait(timeout=5), code)
                stopped = self.stop()
                self.assertEqual(stopped.returncode, code, stopped.stderr)
                self.assertEqual(read_state(self.state)["exit_code"], code)

    def test_stale_creation_time_rejects_stop_without_killing_target(self):
        owner, state = self.fixture("graceful")
        stale = dict(state, creation_time=state["creation_time"] + 1)
        self.state.write_text(json.dumps(stale), encoding="utf-8")
        rejected = self.stop(timeout=1)
        self.assertNotEqual(rejected.returncode, 0)
        self.assertIsNone(owner.poll())
        self.state.write_text(json.dumps(state), encoding="utf-8")
        self.assertEqual(self.stop().returncode, 0)
        self.assertEqual(owner.wait(timeout=5), 0)

    def test_wrong_pid_does_not_signal_unrelated_process(self):
        owner, state = self.fixture("graceful")
        wrong = dict(state, pid=os.getpid())
        self.state.write_text(json.dumps(wrong), encoding="utf-8")
        rejected = self.stop(timeout=1)
        self.assertNotEqual(rejected.returncode, 0)
        self.assertIsNone(owner.poll())
        self.state.write_text(json.dumps(state), encoding="utf-8")
        self.assertEqual(self.stop().returncode, 0)
        self.assertEqual(owner.wait(timeout=5), 0)

    def test_timeout_is_failure_and_terminates_only_owned_target(self):
        owner, state = self.fixture("ignore")
        kernel, handle = process_handle(state["pid"])
        self.assertTrue(handle)
        try:
            started = time.monotonic()
            stopped = self.stop(timeout=0.3)
            self.assertNotEqual(stopped.returncode, 0)
            self.assertLess(time.monotonic() - started, 5)
            self.assertNotEqual(owner.wait(timeout=5), 0)
            self.assertEqual(kernel.WaitForSingleObject(handle, 1000), 0)
        finally:
            kernel.CloseHandle(handle)

    def test_owner_crash_reaps_child_via_job(self):
        owner, state = self.fixture("ignore")
        kernel, handle = process_handle(state["pid"])
        self.assertTrue(handle)
        try:
            owner.kill()
            self.assertNotEqual(owner.wait(timeout=5), 0)
            self.assertEqual(kernel.WaitForSingleObject(handle, 3000), 0)
            self.assertNotEqual(self.stop(timeout=1).returncode, 0)
        finally:
            kernel.CloseHandle(handle)

    def test_existing_run_state_is_not_overwritten(self):
        owner, state = self.fixture("graceful")
        duplicate = subprocess.run(
            [sys.executable, str(HELPER), "run", "--state", str(self.state),
             "--log", str(self.path / "duplicate.log"), "--", sys.executable, "-c", "pass"],
            capture_output=True, text=True, creationflags=subprocess.CREATE_NO_WINDOW, timeout=5,
        )
        self.assertNotEqual(duplicate.returncode, 0)
        self.assertEqual(read_state(self.state), state)
        self.assertIsNone(owner.poll())
        self.assertEqual(self.stop().returncode, 0)

    def test_extra_console_member_rejects_signal(self):
        owner, state = self.fixture("graceful")
        ready = self.path / "attached"
        attached = subprocess.Popen(
            [sys.executable, str(Path(__file__).resolve()), "--attach", str(state["pid"]), str(ready)],
            creationflags=subprocess.CREATE_NO_WINDOW,
            stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        )
        try:
            eventually(ready.exists)
            stopped = self.stop(timeout=1)
            self.assertNotEqual(stopped.returncode, 0)
            self.assertIn("console membership", stopped.stderr)
            self.assertIsNone(owner.poll())
            self.assertIsNone(attached.poll())
        finally:
            attached.kill()
            attached.wait(timeout=5)
        self.assertEqual(self.stop().returncode, 0)

    def test_bash_cleanup_uses_private_console_and_propagates_failure(self):
        bash = Path(os.environ.get("ProgramFiles", "C:/Program Files")) / "Git/bin/bash.exe"
        self.assertTrue(bash.is_file(), "Git Bash is required for the Windows CI cleanup contract")
        for mode, expected in (("graceful", 0), ("exit137", 1), ("exit256", 1)):
            with self.subTest(mode=mode):
                self.state = self.path / f"bash-{mode}.json"
                ready = self.path / f"ready-{mode}"
                owner, _ = self.start([sys.executable, str(Path(__file__).resolve()), "--child", mode, str(ready)])
                eventually(ready.exists)
                if mode.startswith("exit"):
                    self.assertEqual(owner.wait(timeout=5), int(mode[4:]))
                runtime = self.path / f"runtime-{mode}"
                runtime.mkdir()
                (runtime / "server.windows-state").write_text(str(self.state), encoding="utf-8")
                (runtime / "server.pid").write_text(str(owner.pid), encoding="ascii")
                environment = dict(os.environ, RUNNER_TEMP=str(runtime))
                environment.pop("OS", None)
                cleaned = subprocess.run([str(bash), str(ROOT / "ci/server-services.sh"), "cleanup"],
                                         env=environment, capture_output=True, text=True, timeout=10)
                self.assertEqual(cleaned.returncode, expected, cleaned.stdout + cleaned.stderr)
                self.assertEqual(owner.wait(timeout=5), 0 if mode == "graceful" else int(mode[4:]))

    def test_bash_without_os_preserves_nonzero_cleanup(self):
        bash = Path(os.environ.get("ProgramFiles", "C:/Program Files")) / "Git/bin/bash.exe"
        owner, _ = self.fixture("exit137")
        self.assertEqual(owner.wait(timeout=5), 137)
        runtime = self.path / "runtime"
        runtime.mkdir()
        (runtime / "server.windows-state").write_text(str(self.state), encoding="utf-8")
        (runtime / "server.pid").write_text(str(owner.pid), encoding="ascii")
        environment = dict(os.environ, RUNNER_TEMP=str(runtime))
        environment.pop("OS", None)
        cleaned = subprocess.run([str(bash), str(ROOT / "ci/server-services.sh"), "cleanup"],
                                 env=environment, capture_output=True, text=True, timeout=5)
        self.assertNotEqual(cleaned.returncode, 0, "missing OS must not bypass Windows exit verification")

    def test_bash_start_and_cleanup_without_os_use_real_windows_branch(self):
        bash = Path(os.environ.get("ProgramFiles", "C:/Program Files")) / "Git/bin/bash.exe"
        binary = self.path / "ontoforge.exe"
        shutil.copy2(ROOT / "target/debug/ontoforge.exe", binary)
        runtime = self.path / "runtime"
        runtime.mkdir()
        with socket.socket() as listener:
            listener.bind(("127.0.0.1", 0))
            port = listener.getsockname()[1]
        environment = dict(os.environ, RUNNER_TEMP=str(runtime))
        environment.pop("OS", None)
        script = r'''
unset OS
source "$1"
[[ "$BINARY" == *.exe ]] || exit 91
BINARY=$2
EVIDENCE_DIR=$3
trap cleanup EXIT
start_server serve --dev --data-dir "$4" --bind "$5"
[[ -s "$SERVER_STATE_FILE" ]] || exit 92
ready=0
for attempt in {1..100}; do
  if curl -fsS --max-time 1 "http://$5/health" >/dev/null 2>&1; then ready=1; break; fi
  sleep 0.1
done
[[ "$ready" -eq 1 ]] || exit 93
cleanup
trap - EXIT
[[ ! -f "$SERVER_PID_FILE" && ! -f "$SERVER_STATE_FILE" ]]
'''
        owner = subprocess.Popen(
            [str(bash), "-c", script, "probe", (ROOT / "ci/server-services.sh").as_posix(),
             binary.as_posix(), self.path.as_posix(), (self.path / "data").as_posix(), f"127.0.0.1:{port}"],
            env=environment, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
            creationflags=subprocess.CREATE_NO_WINDOW,
        )
        try:
            stdout, stderr = owner.communicate(timeout=20)
            self.assertEqual(owner.returncode, 0, stdout + stderr)
            states = list(runtime.rglob("state.json"))
            self.assertEqual(len(states), 1, "actual start branch must publish one Windows owner state")
            self.assertEqual(read_state(states[0])["exit_code"], 0)
            with socket.socket() as check:
                self.assertNotEqual(check.connect_ex(("127.0.0.1", port)), 0)
        finally:
            for state in runtime.rglob("state.json"):
                subprocess.run([sys.executable, str(HELPER), "stop", "--state", str(state), "--timeout", "2"],
                               capture_output=True, creationflags=subprocess.CREATE_NO_WINDOW, timeout=5)
            if owner.poll() is None:
                owner.kill()
            owner.communicate(timeout=5)


def child(mode, ready):
    import signal
    if mode == "ignore":
        ctypes.windll.kernel32.SetConsoleCtrlHandler(None, True)
    else:
        signal.signal(signal.SIGINT, lambda *_: sys.exit(0))
    Path(ready).write_text("ready", encoding="ascii")
    if mode.startswith("exit"):
        sys.exit(int(mode[4:]))
    while True:
        time.sleep(0.05)


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "--child":
        child(sys.argv[2], sys.argv[3])
    elif len(sys.argv) > 1 and sys.argv[1] == "--attach":
        kernel = ctypes.windll.kernel32
        kernel.FreeConsole()
        if not kernel.AttachConsole(int(sys.argv[2])):
            sys.exit(2)
        child("ignore", sys.argv[3])
    else:
        unittest.main()
