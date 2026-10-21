"""Hosted research only: independent ownership, deadline, release and cleanup."""
import argparse
import ctypes
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time

class WindowsJob:
    def __init__(self):
        from ctypes import wintypes as w
        self.k = ctypes.WinDLL("kernel32", use_last_error=True)
        self.k.CreateJobObjectW.argtypes = [ctypes.c_void_p, w.LPCWSTR]
        self.k.CreateJobObjectW.restype = w.HANDLE
        self.k.SetInformationJobObject.argtypes = [w.HANDLE, ctypes.c_int, ctypes.c_void_p, w.DWORD]
        self.k.AssignProcessToJobObject.argtypes = [w.HANDLE, w.HANDLE]
        self.k.QueryInformationJobObject.argtypes = [w.HANDLE, ctypes.c_int, ctypes.c_void_p, w.DWORD, ctypes.c_void_p]
        self.k.TerminateJobObject.argtypes = [w.HANDLE, w.UINT]
        self.k.CloseHandle.argtypes = [w.HANDLE]
        class Basic(ctypes.Structure):
            _fields_ = [("p", ctypes.c_longlong), ("j", ctypes.c_longlong), ("flags", w.DWORD),
                        ("min", ctypes.c_size_t), ("max", ctypes.c_size_t), ("count", w.DWORD),
                        ("affinity", ctypes.c_size_t), ("priority", w.DWORD), ("scheduling", w.DWORD)]
        class IO(ctypes.Structure):
            _fields_ = [(n, ctypes.c_ulonglong) for n in ["r", "w", "o", "rb", "wb", "ob"]]
        class Extended(ctypes.Structure):
            _fields_ = [("basic", Basic), ("io", IO), ("pm", ctypes.c_size_t),
                        ("jm", ctypes.c_size_t), ("pp", ctypes.c_size_t), ("pj", ctypes.c_size_t)]
        class Accounting(ctypes.Structure):
            _fields_ = [(n, ctypes.c_longlong) for n in ["u", "k", "pu", "pk"]] + [
                (n, w.DWORD) for n in ["faults", "total", "active", "terminated"]]
        self.accounting_type = Accounting
        self.handle = self.k.CreateJobObjectW(None, None)
        if not self.handle:
            raise ctypes.WinError(ctypes.get_last_error())
        limits = Extended()
        limits.basic.flags = 0x2000  # KILL_ON_JOB_CLOSE; fixture-owned, never breakaway.
        if not self.k.SetInformationJobObject(self.handle, 9, ctypes.byref(limits), ctypes.sizeof(limits)):
            self.close()
            raise ctypes.WinError(ctypes.get_last_error())

    def assign(self, process):
        if not self.k.AssignProcessToJobObject(self.handle, int(process._handle)):
            raise ctypes.WinError(ctypes.get_last_error())

    def active(self):
        data = self.accounting_type()
        if not self.k.QueryInformationJobObject(self.handle, 1, ctypes.byref(data), ctypes.sizeof(data), None):
            raise ctypes.WinError(ctypes.get_last_error())
        return data.active

    def kill(self):
        if not self.k.TerminateJobObject(self.handle, 87):
            raise ctypes.WinError(ctypes.get_last_error())

    def close(self):
        if self.handle:
            self.k.CloseHandle(self.handle)
            self.handle = None

def wait_for(test, seconds, label):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        if test():
            return
        time.sleep(0.005)
    raise RuntimeError("supervisor deadline: " + label)

def pid_alive(pid):
    if os.name == "nt":
        from ctypes import wintypes as w
        k = ctypes.WinDLL("kernel32", use_last_error=True)
        k.OpenProcess.argtypes = [w.DWORD, w.BOOL, w.DWORD]
        k.OpenProcess.restype = w.HANDLE
        k.WaitForSingleObject.argtypes = [w.HANDLE, w.DWORD]
        k.CloseHandle.argtypes = [w.HANDLE]
        handle = k.OpenProcess(0x100000, False, pid)
        if not handle:
            if ctypes.get_last_error() == 87:
                return False
            raise ctypes.WinError(ctypes.get_last_error())
        try:
            return k.WaitForSingleObject(handle, 0) == 258
        finally:
            k.CloseHandle(handle)
    try:
        # Subreaper can retain exited synthetic descendants as zombies.
        state = Path("/proc", str(pid), "stat").read_text().split(") ", 1)[1][0]
        return state != "Z"
    except FileNotFoundError:
        return False

def reap():
    if os.name != "nt":
        while True:
            try:
                pid, _ = os.waitpid(-1, os.WNOHANG)
                if pid == 0:
                    break
            except ChildProcessError:
                break

def exercise(executable, case, parent):
    root = parent / case
    root.mkdir()
    job = WindowsJob() if os.name == "nt" else None
    process = None
    success = False
    out = root / "outer.stdout"
    err = root / "outer.stderr"
    try:
        with out.open("wb") as stdout, err.open("wb") as stderr:
            process = subprocess.Popen([str(executable), "runner", str(root), case],
                                       stdout=stdout, stderr=stderr,
                                       start_new_session=(os.name != "nt"))
            if job:
                job.assign(process)
            (root / "go").write_text("synthetic", encoding="ascii")
            wait_for(lambda: (root / "parent.exiting").exists(), 4, "direct parent exiting")
            parent_pid = int((root / "parent.pid").read_text())
            holder_pid = int((root / "holder.pid").read_text()) if case != "control" else None
            wait_for(lambda: not pid_alive(parent_pid), 2, "direct parent actually exited")
            if holder_pid is not None:
                assert pid_alive(holder_pid), "retaining descendant absent"
            observed = time.monotonic()
            if case == "control":
                wait_for(lambda: process.poll() is not None, 1, "control returns")
                blocked = False
            else:
                # Direct child is proven gone, yet the configured 200ms deadline
                # cannot return while the synthetic descendant holds a pipe.
                time.sleep(1.0)
                blocked = process.poll() is None
                if case != "null-stdio":
                    assert blocked, "source-level liveness hypothesis not reproduced"
                    assert not (root / "result.json").exists(), "runner already returned"
            held_ms = round((time.monotonic() - observed) * 1000)
            (root / "release").write_text("synthetic", encoding="ascii")
            process.wait(timeout=4)
            assert process.returncode == 0, "fixture failed"
            if holder_pid is not None:
                wait_for(lambda: not pid_alive(holder_pid), 2, "descendant exit")
            if job:
                wait_for(lambda: job.active() == 0, 2, "job quiescence")
            reap()
            value = json.loads((root / "result.json").read_text(encoding="utf-8"))
            if case not in ["control", "null-stdio"]:
                assert value["runner_ms"] >= 1000, "deadline violation not measured"
            if case == "stdin":
                assert not value["ok"] and "Io" in value["error"], "writer failure not retained"
            else:
                assert value["ok"], "unrelated runner error"
            assert out.stat().st_size <= 4096 and err.stat().st_size <= 4096, "unexpected fixture output"
            success = True
            return dict(case=case,configured_deadline_ms=200,blocked_after_parent_exit=blocked,
                        externally_held_ms=held_ms,runner=value,cleanup="quiescent",
                        cause_scope="synthetic descendant retention; not natural MCP timeout")
    finally:
        (root / "release").write_text("synthetic", encoding="ascii")
        if process:
            if job:
                if not success:
                    job.kill()
            else:
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
            try:
                process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=3)
        if job:
            job.close()
        reap()

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--fixture-exe", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    executable = Path(args.fixture_exe).resolve(strict=True)
    if sys.platform.startswith("linux"):
        # This disposable supervisor adopts only descendants it creates.
        libc = ctypes.CDLL(None, use_errno=True)
        if libc.prctl(36, 1, 0, 0, 0):
            raise OSError(ctypes.get_errno(), "subreaper")
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    result = {"platform": sys.platform, "python": sys.version.split()[0],
              "claim": "supervised characterization of unchanged HardenedCommandRunner",
              "cases": [], "complete": False}
    started = time.monotonic()
    try:
        with tempfile.TemporaryDirectory(prefix="optic-issue317-") as temporary:
            for case in ["control", "null-stdio", "stdout", "stderr", "both", "stdin"]:
                result["cases"].append(exercise(executable, case, Path(temporary)))
                output.write_text(json.dumps(result, indent=2), encoding="utf-8")
        result["complete"] = True
    finally:
        result["elapsed_seconds"] = time.monotonic() - started
        output.write_text(json.dumps(result, indent=2), encoding="utf-8")
        print("OPTIC_317_SUPERVISED " + json.dumps(result, separators=(",", ":")))
    return 0

if __name__ == "__main__":
    sys.exit(main())
