"""Fixed synthetic-host death under an INDEPENDENT kill-on-close outer Job.

Windows GitHub-hosted research only; never touches a user's computer/PID.
The outer Job remains OPEN when the exact Popen host is killed. The suspended
descendant must disappear because the dead host loses its inner Job handle.
"""
import argparse
import ctypes
from ctypes import wintypes as w
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time


def wait_for(predicate, seconds, label):
    until = time.monotonic() + seconds
    while time.monotonic() < until:
        if predicate():
            return
        time.sleep(0.005)
    raise RuntimeError("independent supervisor deadline: " + label)


class OuterJob:
    def __init__(self):
        self.k = ctypes.WinDLL("kernel32", use_last_error=True)
        self.k.CreateJobObjectW.argtypes = [ctypes.c_void_p, w.LPCWSTR]
        self.k.CreateJobObjectW.restype = w.HANDLE
        self.k.SetInformationJobObject.argtypes = [w.HANDLE, ctypes.c_int, ctypes.c_void_p, w.DWORD]
        self.k.AssignProcessToJobObject.argtypes = [w.HANDLE, w.HANDLE]
        self.k.QueryInformationJobObject.argtypes = [
            w.HANDLE, ctypes.c_int, ctypes.c_void_p, w.DWORD, ctypes.c_void_p
        ]
        self.k.TerminateJobObject.argtypes = [w.HANDLE, w.UINT]
        self.k.CloseHandle.argtypes = [w.HANDLE]

        class Basic(ctypes.Structure):
            _fields_ = [
                ("per_process_user", ctypes.c_longlong),
                ("per_job_user", ctypes.c_longlong),
                ("limits", w.DWORD),
                ("min_working_set", ctypes.c_size_t),
                ("max_working_set", ctypes.c_size_t),
                ("max_processes", w.DWORD),
                ("affinity", ctypes.c_size_t),
                ("priority", w.DWORD),
                ("scheduling", w.DWORD),
            ]

        class Io(ctypes.Structure):
            _fields_ = [(field, ctypes.c_ulonglong) for field in ["r", "w", "o", "rb", "wb", "ob"]]

        class Extended(ctypes.Structure):
            _fields_ = [
                ("basic", Basic), ("io", Io),
                ("process_memory", ctypes.c_size_t), ("job_memory", ctypes.c_size_t),
                ("peak_process_memory", ctypes.c_size_t), ("peak_job_memory", ctypes.c_size_t),
            ]

        class Accounting(ctypes.Structure):
            _fields_ = [(name, ctypes.c_longlong) for name in ["u", "k", "pu", "pk"]]
            _fields_ += [(name, w.DWORD) for name in ["faults", "total", "active", "terminated"]]

        self.accounting = Accounting
        self.handle = self.k.CreateJobObjectW(None, None)
        if not self.handle:
            raise ctypes.WinError(ctypes.get_last_error())
        try:
            limits = Extended()
            limits.basic.limits = 0x2000  # JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
            if not self.k.SetInformationJobObject(
                self.handle, 9, ctypes.byref(limits), ctypes.sizeof(limits)
            ):
                raise ctypes.WinError(ctypes.get_last_error())
        except BaseException:
            self.k.CloseHandle(self.handle)
            self.handle = None
            raise

    def assign(self, process):
        # Only the exact Popen handle returned for this fixture is accepted.
        if not self.k.AssignProcessToJobObject(self.handle, int(process._handle)):
            raise ctypes.WinError(ctypes.get_last_error())

    def active(self):
        accounting = self.accounting()
        if not self.k.QueryInformationJobObject(
            self.handle, 1, ctypes.byref(accounting), ctypes.sizeof(accounting), None
        ):
            raise ctypes.WinError(ctypes.get_last_error())
        return accounting.active

    def close(self):
        if self.handle:
            try:
                if not self.k.TerminateJobObject(self.handle, 87):
                    raise ctypes.WinError(ctypes.get_last_error())
                wait_for(lambda: self.active() == 0, 3, "outer Job cleanup")
            finally:
                self.k.CloseHandle(self.handle)
                self.handle = None


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--test-exe", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    if os.name != "nt":
        raise RuntimeError("Windows hosted native proof only")
    executable = Path(args.test_exe).resolve(strict=True)
    output = Path(args.output).resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    evidence = {
        "source_sha": os.environ.get("GITHUB_SHA"),
        "workflow_attempt": os.environ.get("GITHUB_RUN_ATTEMPT"),
        "precreated_kernel_job": True,
        "production_wired": False,
        "external_supervisor": True,
        "complete": False,
    }

    with tempfile.TemporaryDirectory(prefix="optic317-joblist-outer-", dir=output.parent) as tmp:
        root = Path(tmp)
        job = OuterJob()
        host = None
        trace = root / "host.log"
        try:
            environment = {
                key: os.environ[key] for key in ["SystemRoot", "WINDIR"] if key in os.environ
            }
            environment.update({
                "TEMP": str(root),
                "TMP": str(root),
                "OPTIC_317_HOST_ROOT": str(root),
            })
            with trace.open("wb") as log:
                host = subprocess.Popen(
                    [
                        str(executable),
                        "--exact",
                        "job::create_time_research::synthetic_creator_host_for_supervisor",
                        "--ignored", "--nocapture", "--test-threads=1",
                    ],
                    cwd=root, env=environment, stdout=log, stderr=log,
                )
                job.assign(host)
                wait_for(lambda: job.active() == 1, 3, "outer Job owns fixed host before gate")
                evidence["outer_before_native_creation"] = 1
                (root / "outer-owned-gate").write_text("owned", encoding="ascii")
                ready = root / "inner-suspended-ready"
                wait_for(lambda: ready.exists() or host.poll() is not None, 8, "host creates child")
                if host.poll() is not None:
                    raise RuntimeError("creator exited unexpectedly before synthetic death")
                assert ready.read_bytes() == b"kernel-job-associated"
                wait_for(lambda: job.active() == 2, 3, "outer sees suspended root and host")
                evidence["outer_while_both_alive"] = job.active()
                assert not (root / "must-never-run").exists()
                host.kill()  # known exact host handle, NOT a PID search
                host.wait(timeout=3)
                evidence["synthetic_host_exit_confirmed"] = True

                # IMPORTANT: leave outer Job handle OPEN during this check.
                # If the inner Job handle leaks or lacks kill-on-close, the
                # suspended child persists here (test fails, outer fallback cleans).
                wait_for(lambda: job.active() == 0, 3, "inner Job killed orphan on host exit")
                evidence["outer_after_host_exit_while_still_open"] = job.active()
                assert not (root / "must-never-run").exists()
                evidence["suspended_child_never_resumed"] = True
                evidence["complete"] = True
        except BaseException as error:
            evidence["failure_class"] = type(error).__name__
            if trace.exists():
                tail = trace.read_text(encoding="utf-8", errors="replace")[-5000:]
                evidence["sanitized_log_tail"] = tail.replace(str(root), "<fixture>").replace(
                    str(Path.home()), "<home>"
                )
            raise
        finally:
            try:
                job.close()  # exact outer containment even after proof failure
                evidence["outer_job_cleanup_verified"] = True
            finally:
                if host is not None:
                    if host.poll() is None:
                        host.kill()
                    host.wait(timeout=3)
                    host._handle.Close()
                output.write_text(json.dumps(evidence, indent=2), encoding="utf-8")
                print("OPTIC317_KERNEL_JOB_HOST_DEATH " + json.dumps(evidence, separators=(",", ":")))


if __name__ == "__main__":
    main()
