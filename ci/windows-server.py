"""Own and stop a Windows CI server in a private, hidden console (stdlib only)."""

import argparse
import ctypes
from ctypes import wintypes
import json
import math
import os
from pathlib import Path
import subprocess
import sys
import time


class BasicJobLimits(ctypes.Structure):
    _fields_ = [("per_process_time", ctypes.c_int64), ("per_job_time", ctypes.c_int64),
                ("flags", wintypes.DWORD), ("min_working_set", ctypes.c_size_t),
                ("max_working_set", ctypes.c_size_t), ("active_processes", wintypes.DWORD),
                ("affinity", ctypes.c_size_t), ("priority", wintypes.DWORD),
                ("scheduling", wintypes.DWORD)]


class IoCounters(ctypes.Structure):
    _fields_ = [(name, ctypes.c_uint64) for name in
                ("reads", "writes", "other", "read_bytes", "write_bytes", "other_bytes")]


class ExtendedJobLimits(ctypes.Structure):
    _fields_ = [("basic", BasicJobLimits), ("io", IoCounters),
                ("process_memory", ctypes.c_size_t), ("job_memory", ctypes.c_size_t),
                ("peak_process_memory", ctypes.c_size_t), ("peak_job_memory", ctypes.c_size_t)]


class Windows:
    def __init__(self):
        self.kernel = ctypes.WinDLL("kernel32", use_last_error=True)
        signatures = {
            "OpenProcess": ([wintypes.DWORD, wintypes.BOOL, wintypes.DWORD], wintypes.HANDLE),
            "CloseHandle": ([wintypes.HANDLE], wintypes.BOOL),
            "GetProcessTimes": ([wintypes.HANDLE] + [ctypes.POINTER(wintypes.FILETIME)] * 4, wintypes.BOOL),
            "GetExitCodeProcess": ([wintypes.HANDLE, ctypes.POINTER(wintypes.DWORD)], wintypes.BOOL),
            "WaitForSingleObject": ([wintypes.HANDLE, wintypes.DWORD], wintypes.DWORD),
            "TerminateProcess": ([wintypes.HANDLE, wintypes.UINT], wintypes.BOOL),
            "CreateJobObjectW": ([ctypes.c_void_p, wintypes.LPCWSTR], wintypes.HANDLE),
            "SetInformationJobObject": ([wintypes.HANDLE, ctypes.c_int, ctypes.c_void_p, wintypes.DWORD], wintypes.BOOL),
            "AssignProcessToJobObject": ([wintypes.HANDLE, wintypes.HANDLE], wintypes.BOOL),
            "FreeConsole": ([], wintypes.BOOL),
            "AttachConsole": ([wintypes.DWORD], wintypes.BOOL),
            "GetConsoleProcessList": ([ctypes.POINTER(wintypes.DWORD), wintypes.DWORD], wintypes.DWORD),
            "SetConsoleCtrlHandler": ([ctypes.c_void_p, wintypes.BOOL], wintypes.BOOL),
            "GenerateConsoleCtrlEvent": ([wintypes.DWORD, wintypes.DWORD], wintypes.BOOL),
        }
        for name, (arguments, result) in signatures.items():
            function = getattr(self.kernel, name)
            function.argtypes = arguments
            function.restype = result

    @staticmethod
    def checked(value):
        if not value:
            raise ctypes.WinError(ctypes.get_last_error())
        return value

    def process(self, pid):
        # Keep this handle open throughout identity verification, signalling and waiting.
        return self.checked(self.kernel.OpenProcess(0x100000 | 0x1000 | 0x0001 | 0x0100, False, pid))

    def creation_time(self, handle):
        times = [wintypes.FILETIME() for _ in range(4)]
        self.checked(self.kernel.GetProcessTimes(handle, *(ctypes.byref(value) for value in times)))
        return (times[0].dwHighDateTime << 32) | times[0].dwLowDateTime

    def exited(self, handle, milliseconds=0):
        status = self.kernel.WaitForSingleObject(handle, milliseconds)
        if status == 0:
            return True
        if status == 258:
            return False
        raise ctypes.WinError(ctypes.get_last_error())

    def exit_code(self, handle):
        code = wintypes.DWORD()
        self.checked(self.kernel.GetExitCodeProcess(handle, ctypes.byref(code)))
        return code.value

    def job(self):
        job = self.checked(self.kernel.CreateJobObjectW(None, None))
        try:
            limits = ExtendedJobLimits()
            limits.basic.flags = 0x2000  # JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
            self.checked(self.kernel.SetInformationJobObject(job, 9, ctypes.byref(limits), ctypes.sizeof(limits)))
            return job
        except BaseException:
            self.kernel.CloseHandle(job)
            raise

    def interrupt_private_console(self, pid, process):
        self.kernel.FreeConsole()
        self.checked(self.kernel.AttachConsole(pid))
        try:
            members = (wintypes.DWORD * 16)()
            count = self.kernel.GetConsoleProcessList(members, len(members))
            if count != 2 or set(members[:count]) != {pid, os.getpid()}:
                raise RuntimeError("private console membership does not match target and helper")
            if self.exited(process):
                return
            self.checked(self.kernel.SetConsoleCtrlHandler(None, True))
            self.checked(self.kernel.GenerateConsoleCtrlEvent(0, 0))
        finally:
            self.kernel.FreeConsole()


def atomic_state(path, state):
    temporary = path.with_name(f"{path.name}.{os.getpid()}.tmp")
    try:
        with temporary.open("x", encoding="utf-8") as stream:
            json.dump(state, stream)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
    finally:
        temporary.unlink(missing_ok=True)


def run(args, windows):
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if not command:
        raise ValueError("run requires a command after --")
    args.state.parent.mkdir(parents=True, exist_ok=True)
    # A run owns a fresh state path. Never overwrite a previous process identity.
    with args.state.with_suffix(args.state.suffix + ".lock").open("x"):
        if args.state.exists():
            raise ValueError("state path already exists; use a new run directory")
        job = windows.job()
        child = None
        process = None
        try:
            startup = subprocess.STARTUPINFO()
            startup.dwFlags = subprocess.STARTF_USESHOWWINDOW
            startup.wShowWindow = subprocess.SW_HIDE
            with args.log.open("wb") as output:
                child = subprocess.Popen(command, stdin=subprocess.DEVNULL, stdout=output,
                                         stderr=output, creationflags=subprocess.CREATE_NEW_CONSOLE,
                                         startupinfo=startup)
                process = windows.process(child.pid)
                windows.checked(windows.kernel.AssignProcessToJobObject(job, process))
                state = {"pid": child.pid, "creation_time": windows.creation_time(process),
                         "phase": "running"}
                atomic_state(args.state, state)
                code = child.wait()
                atomic_state(args.state, dict(state, phase="exited", exit_code=code))
                return code
        finally:
            # Assignment failure never degrades to an unowned child.
            if child is not None and child.poll() is None:
                child.kill()
                child.wait(timeout=5)
            if process:
                windows.kernel.CloseHandle(process)
            windows.kernel.CloseHandle(job)


def stop(args, windows):
    state = json.loads(args.state.read_text(encoding="utf-8"))
    if state.get("phase") == "exited":
        return int(state["exit_code"])
    pid, expected_creation = int(state["pid"]), int(state["creation_time"])
    process = windows.process(pid)
    deadline = time.monotonic() + args.timeout

    def confirmed_exit():
        code = windows.exit_code(process)
        if code != 0:
            return code
        # Job termination after an owner crash may report zero. Only the owner's
        # durable completion record can confirm that zero was a normal exit.
        while time.monotonic() < deadline:
            latest = json.loads(args.state.read_text(encoding="utf-8"))
            if (latest.get("pid"), latest.get("creation_time")) != (pid, expected_creation):
                raise RuntimeError("state identity changed while waiting for owner")
            if latest.get("phase") == "exited":
                return int(latest["exit_code"])
            time.sleep(0.02)
        raise RuntimeError("owner did not confirm successful server exit")

    try:
        if windows.creation_time(process) != expected_creation:
            raise RuntimeError("process creation time does not match state; refusing to signal")
        if windows.exited(process):
            return confirmed_exit()
        windows.interrupt_private_console(pid, process)
        cleanup_budget = min(1.0, args.timeout / 2)
        remaining_ms = max(0, int((deadline - cleanup_budget - time.monotonic()) * 1000))
        if windows.exited(process, remaining_ms):
            return confirmed_exit()
        windows.checked(windows.kernel.TerminateProcess(process, 1))
        remaining_ms = max(0, int((deadline - time.monotonic()) * 1000))
        if not windows.exited(process, remaining_ms):
            raise RuntimeError("timed out waiting for forced process cleanup")
        print("Windows server shutdown timed out; forced cleanup is a failure", file=sys.stderr)
        return 1
    finally:
        windows.kernel.CloseHandle(process)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    modes = parser.add_subparsers(dest="mode", required=True)
    start = modes.add_parser("run")
    start.add_argument("--state", type=Path, required=True)
    start.add_argument("--log", type=Path, required=True)
    start.add_argument("command", nargs=argparse.REMAINDER)
    stopping = modes.add_parser("stop")
    stopping.add_argument("--state", type=Path, required=True)
    stopping.add_argument("--timeout", type=float, default=30)
    stopping.add_argument("--shell-status", action="store_true",
                          help="report any nonzero Windows exit as 1 for an 8-bit shell")
    args = parser.parse_args()
    if os.name != "nt":
        parser.error("this helper is Windows-only")
    if args.mode == "stop" and (not math.isfinite(args.timeout) or not 0 < args.timeout <= 30):
        parser.error("timeout must be greater than zero and at most 30 seconds")
    try:
        code = run(args, Windows()) if args.mode == "run" else stop(args, Windows())
        if args.mode == "stop" and args.shell_status and code != 0:
            print(f"Windows server exit status: {code}", file=sys.stderr)
            return 1
        return code
    except (OSError, ValueError, KeyError, RuntimeError, subprocess.SubprocessError) as error:
        print(f"Windows server helper failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
