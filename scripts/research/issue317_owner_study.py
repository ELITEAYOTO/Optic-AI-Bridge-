"""Research-only ownership feasibility. Never imported by the Bridge.
All processes are known synthetic fixtures; outer watchdog runs independently.
"""
import argparse
import ctypes
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import threading
import time
sys.path.insert(0, str(Path(__file__).resolve().parent))
from issue317_supervisor import WindowsJob, pid_alive, reap, wait_for

def subreaper():
    if sys.platform.startswith("linux"):
        libc = ctypes.CDLL(None, use_errno=True)
        if libc.prctl(36, 1, 0, 0, 0):
            raise OSError(ctypes.get_errno(), "subreaper")

def resume_suspended(process):
    """Documented ToolHelp/ResumeThread; known suspended fixture only."""
    from ctypes import wintypes as w
    k = ctypes.WinDLL("kernel32", use_last_error=True)
    class Entry(ctypes.Structure):
        _fields_ = [("size", w.DWORD), ("usage", w.DWORD), ("tid", w.DWORD),
                    ("pid", w.DWORD), ("base", w.LONG), ("delta", w.LONG),
                    ("flags", w.DWORD)]
    k.CreateToolhelp32Snapshot.argtypes = [w.DWORD, w.DWORD]
    k.CreateToolhelp32Snapshot.restype = w.HANDLE
    k.Thread32First.argtypes = [w.HANDLE, ctypes.POINTER(Entry)]
    k.Thread32Next.argtypes = [w.HANDLE, ctypes.POINTER(Entry)]
    k.OpenThread.argtypes = [w.DWORD, w.BOOL, w.DWORD]
    k.OpenThread.restype = w.HANDLE
    k.ResumeThread.argtypes = [w.HANDLE]
    k.ResumeThread.restype = w.DWORD
    k.CloseHandle.argtypes = [w.HANDLE]
    snapshot = k.CreateToolhelp32Snapshot(4, 0)
    if snapshot == ctypes.c_void_p(-1).value:
        raise ctypes.WinError(ctypes.get_last_error())
    resumed = 0
    try:
        entry = Entry()
        entry.size = ctypes.sizeof(entry)
        more = k.Thread32First(snapshot, ctypes.byref(entry))
        while more:
            if entry.pid == process.pid:
                handle = k.OpenThread(2, False, entry.tid)
                if not handle:
                    raise ctypes.WinError(ctypes.get_last_error())
                try:
                    previous = k.ResumeThread(handle)
                    if previous == 0xFFFFFFFF:
                        raise ctypes.WinError(ctypes.get_last_error())
                    assert previous == 1, "unexpected suspension count"
                    resumed += 1
                finally:
                    k.CloseHandle(handle)
            more = k.Thread32Next(snapshot, ctypes.byref(entry))
    finally:
        k.CloseHandle(snapshot)
    assert resumed == 1, "fixture primary thread was not uniquely resumed"

class Owner:
    """One reserved slot, retained native owner, pipe objects and join handles.
    Returning Uncertain does NOT drop this object; caller must recover it.
    Research policy only: no session identity/CPU reservation integration.
    """
    def __init__(self, root):
        self.root = root
        self.process = None
        self.job = WindowsJob() if os.name == "nt" else None
        self.threads = []
        self.bytes = 0
        self.captured = bytearray()
        self.errors = []
        self.lock = threading.Lock()
        self.overflow = threading.Event()
        self.slot = True
        self.state = "Reserved"
        self.injected = []
        self.closed = False

    def start(self, exe, case, fault):
        assert self.slot and self.process is None
        environment = {k: os.environ[k] for k in ["SystemRoot", "SystemDrive"] if k in os.environ}
        environment["OPTIC_RESEARCH_PYTHON"] = sys.executable
        if fault == "spawn":
            exe = self.root / "nonexistent-fixture-executable"
        self.process = subprocess.Popen(
            [str(exe), "parent", str(self.root), case], env=environment,
            cwd=self.root, stdin=subprocess.PIPE if case == "stdin" else subprocess.DEVNULL,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, bufsize=0,
            creationflags=(0x4 | 0x08000000) if os.name == "nt" else 0,
            start_new_session=(os.name != "nt"))
        (self.root / "owned-root.pid").write_text(str(self.process.pid), encoding="ascii")
        if self.job:
            self.job.assign(self.process)
            # This test executes no fixture user code before assignment.
            assert not (self.root / "parent.pid").exists()
            resume_suspended(self.process)
        self.state = "Running"
        self.started = time.monotonic()
        for name, stream in [("stdout", self.process.stdout), ("stderr", self.process.stderr)]:
            if fault == "thread-start" and name == "stderr":
                self.injected.append("thread-start")
                raise OSError("injected thread creation failure")
            thread = threading.Thread(target=self.read, args=(stream, fault == "read" and name == "stdout"),
                                      daemon=False)
            thread.start()
            self.threads.append(thread)
        if case == "stdin":
            thread = threading.Thread(target=self.write, daemon=False)
            thread.start()
            self.threads.append(thread)

    def read(self, stream, fail):
        try:
            if fail:
                self.injected.append("read")
                raise OSError("injected read failure")
            while True:
                chunk = os.read(stream.fileno(), 8192)
                if not chunk:
                    break
                with self.lock:
                    room = max(0, 4096 - self.bytes)
                    self.captured.extend(chunk[:room])
                    self.bytes += len(chunk)
                    if self.bytes > 4096:
                        self.overflow.set()
        except OSError as error:
            with self.lock:
                self.errors.append("ReadError:" + type(error).__name__)

    def write(self):
        try:
            remaining = memoryview(bytes(256 * 1024))
            while remaining:
                size = os.write(self.process.stdin.fileno(), remaining)
                if not size:
                    raise OSError("zero write progress")
                remaining = remaining[size:]
        except OSError as error:
            with self.lock:
                self.errors.append("WriteError:" + type(error).__name__)
        finally:
            self.process.stdin.close()

    def reason(self, fault):
        while True:
            if fault == "wait" and (self.root / "holder.ready").exists():
                self.injected.append("wait")
                return "WaitErrorInjected"
            if self.errors:
                return "IoError"
            if self.overflow.is_set():
                return "OutputLimitExceeded"
            if self.process.poll() is not None:
                return "RootExited"
            if time.monotonic() - self.started >= 0.2:
                return "TimedOut"
            time.sleep(0.002)

    def empty(self):
        if self.process is None:
            return True
        if self.process.poll() is None:
            return False
        # This worker is a private subreaper, not the Bridge process.
        reap()
        if self.job:
            return self.job.active() == 0
        try:
            os.killpg(self.process.pid, 0)
            return False
        except ProcessLookupError:
            return True

    def finish(self, fault=None):
        if self.process:
            if fault == "terminate":
                # Real live holder remains; no permission/security manipulation.
                self.injected.append("terminate")
            elif self.job:
                self.job.kill()
            else:
                try:
                    os.killpg(self.process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
        if fault in ["terminate", "query"]:
            if fault == "query":
                self.injected.append("query")
            self.state = "Uncertain"
            return False
        until = time.monotonic() + 2
        while time.monotonic() < until:
            if self.empty() and all(not t.is_alive() for t in self.threads):
                for thread in self.threads:
                    thread.join()
                if self.process:
                    for stream in [self.process.stdin, self.process.stdout, self.process.stderr]:
                        if stream:
                            stream.close()
                if self.job:
                    self.job.close()
                if os.name == "nt" and self.process:
                    self.process._handle.Close()
                self.state = "Reclaimed"
                self.closed = True
                self.slot = False
                return True
            time.sleep(0.005)
        self.state = "Uncertain"
        return False

def worker(exe, root, case, fault):
    subreaper()
    owner = Owner(root)
    retained = None
    began = time.monotonic()
    try:
        try:
            owner.start(exe, case, fault)
            reason = owner.reason(fault)
        except OSError:
            reason = "SpawnError" if owner.process is None else "SetupError"
        if fault == "terminate":
            wait_for(lambda: (root / "holder.ready").exists(), 2, "retaining fixture ready")
        first = owner.finish(fault)
        if not first:
            retained = {
                "slot": owner.slot, "state": owner.state,
                "process_object": owner.process is not None,
                "join_handles": len(owner.threads), "closed": owner.closed,
                "job_handle": bool(owner.job.handle) if owner.job else None}
            assert owner.slot and not owner.closed and owner.process is not None
            assert owner.state == "Uncertain"
            # Admission of another command is refused while this one is retained.
            admission_allowed = not owner.slot
            assert not admission_allowed
            if fault == "terminate":
                assert pid_alive(int((root / "holder.pid").read_text()))
            assert owner.finish(), "retry could not reclaim retained owner"
        assert owner.closed and not owner.slot
        assert all(not t.is_alive() for t in owner.threads)
        assert len(owner.captured) <= 4096
        if fault == "spawn":
            assert reason == "SpawnError"
        elif fault == "thread-start":
            assert reason == "SetupError"
        elif fault == "wait":
            assert reason == "WaitErrorInjected"
        elif fault == "read":
            assert reason == "IoError"
        elif case == "timeout-both":
            assert reason == "TimedOut"
        elif case == "limit-both":
            # root-exit/overflow race; exact output ceiling must still be retained.
            assert owner.overflow.is_set()
        if case == "nonzero":
            assert owner.process.returncode == 7
        if case == "stdin":
            assert any(x.startswith("WriteError:") for x in owner.errors)
        result = dict(case=case, fault=fault, reason=reason,
                      elapsed_ms=round((time.monotonic()-began)*1000),
                      root_code=owner.process.returncode if owner.process else None,
                      captured_bytes=len(owner.captured), errors=owner.errors,
                      retained=retained, injected=owner.injected,
                      cleanup="tree-empty-and-threads-joined",
                      native_scope="job" if owner.job else "cooperative-process-group")
        (root / "result.json").write_text(json.dumps(result), encoding="utf-8")
    finally:
        # All known helpers have an autonomous eight-second lifetime too.
        if not owner.closed:
            owner.finish()

def adopted_children():
    """Only direct children of this private subreaper; never machine-wide PIDs."""
    return [int(x) for x in Path("/proc/self/task", str(os.getpid()), "children").read_text().split()]

def guarded(exe, case, fault, directory):
    root = directory / (case + "-" + fault)
    root.mkdir()
    outer_job = WindowsJob() if os.name == "nt" else None
    process = None
    try:
        with (root / "worker.log").open("wb") as log:
            process = subprocess.Popen(
                [sys.executable, "-I", str(Path(__file__).resolve()), "--worker",
                 "--fixture-exe", str(exe), "--root", str(root), "--case", case, "--fault", fault],
                stdout=log, stderr=log,
                creationflags=(0x4 | 0x08000000) if os.name == "nt" else 0,
                start_new_session=(os.name != "nt"))
            if outer_job:
                outer_job.assign(process)
                resume_suspended(process)
            process.wait(timeout=12)
        if process.returncode:
            # Controlled Python/Rust fixtures only; do not publish raw paths.
            detail = (root / "worker.log").read_text(errors="replace")[-8192:]
            detail = detail.replace(str(root), "<fixture>").replace(str(exe), "<fixture-exe>").replace(str(Path(__file__).resolve().parent), "<research>")
            raise RuntimeError("worker failed: " + case + "/" + fault + "\n" + detail)
        result = json.loads((root / "result.json").read_text())
        if outer_job:
            assert outer_job.active() == 0
        else:
            reap()
            assert adopted_children() == [], "residual adopted fixture"
        result["independent_guard"] = "quiescent"
        return result
    finally:
        if outer_job:
            outer_job.kill()
            outer_job.close()
        elif process and process.poll() is None:
            os.killpg(process.pid, signal.SIGKILL)
        if process:
            process.wait(timeout=3)
        if os.name != "nt":
            # After worker death, subreaper adopts only this experiment's orphans.
            # pidfds pin their identities before signalling; no PID reuse target.
            until = time.monotonic() + 3
            while time.monotonic() < until:
                reap()
                children = adopted_children()
                if not children:
                    break
                for pid in children:
                    try:
                        fd = os.pidfd_open(pid)
                        try:
                            signal.pidfd_send_signal(fd, signal.SIGKILL)
                        finally:
                            os.close(fd)
                    except ProcessLookupError:
                        pass
                time.sleep(0.01)
            assert adopted_children() == [], "outer recovery left owned descendants"

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--fixture-exe", required=True)
    parser.add_argument("--output")
    parser.add_argument("--worker", action="store_true")
    parser.add_argument("--root")
    parser.add_argument("--case")
    parser.add_argument("--fault")
    args = parser.parse_args()
    exe = Path(args.fixture_exe).resolve(strict=True)
    if args.worker:
        worker(exe, Path(args.root), args.case, args.fault)
        return
    subreaper()
    result = dict(sha=os.environ.get("GITHUB_SHA"), run=os.environ.get("GITHUB_RUN_ID"),
                  attempt=os.environ.get("GITHUB_RUN_ATTEMPT"), platform=sys.platform,
                  python=sys.version.split()[0], complete=False, cases=[],
                  production_fix=False)
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    try:
        with tempfile.TemporaryDirectory(prefix="optic-owner-study-") as directory:
            matrix = [(c, "none") for c in ["control", "nonzero", "null-stdio", "selective-null",
                      "stdout", "stderr", "both", "stdin", "timeout-both", "limit-both"]]
            matrix += [("both", f) for f in ["spawn", "thread-start", "read", "wait", "terminate", "query"]]
            for case, fault in matrix:
                result["cases"].append(guarded(exe, case, fault, Path(directory)))
                output.write_text(json.dumps(result, indent=2), encoding="utf-8")
        result["complete"] = True
    finally:
        output.write_text(json.dumps(result, indent=2), encoding="utf-8")
        print("OPTIC_OWNER_STUDY " + json.dumps(result, separators=(",", ":")))

if __name__ == "__main__":
    main()
