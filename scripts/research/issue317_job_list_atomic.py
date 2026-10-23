"""#317 research: independently supervise a synthetic Win32 atomic Job launch.

Windows-hosted CI only. No Bridge production API, no operator PC, no secrets.
An outer Job owns the test host before the host is allowed to create any child.
The host creates a suspended synthetic child and is then killed by that outer
supervisor. The legacy case intentionally leaves an unassigned child behind;
the outer Job ALWAYS cleans it. Only fixed synthetic executables are used.
"""
import argparse
import ctypes
from ctypes import wintypes as w
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time

CREATE_SUSPENDED = 0x00000004
EXTENDED_STARTUPINFO_PRESENT = 0x00080000
PROC_THREAD_ATTRIBUTE_JOB_LIST = 0x0002000D


class StartupInfo(ctypes.Structure):
    _fields_ = [
        ("cb", w.DWORD),
        ("lpReserved", w.LPWSTR),
        ("lpDesktop", w.LPWSTR),
        ("lpTitle", w.LPWSTR),
        ("dwX", w.DWORD),
        ("dwY", w.DWORD),
        ("dwXSize", w.DWORD),
        ("dwYSize", w.DWORD),
        ("dwXCountChars", w.DWORD),
        ("dwYCountChars", w.DWORD),
        ("dwFillAttribute", w.DWORD),
        ("dwFlags", w.DWORD),
        ("wShowWindow", w.WORD),
        ("cbReserved2", w.WORD),
        ("lpReserved2", ctypes.c_void_p),
        ("hStdInput", w.HANDLE),
        ("hStdOutput", w.HANDLE),
        ("hStdError", w.HANDLE),
    ]


class StartupInfoEx(ctypes.Structure):
    _fields_ = [("StartupInfo", StartupInfo), ("lpAttributeList", ctypes.c_void_p)]


class ProcessInformation(ctypes.Structure):
    _fields_ = [
        ("hProcess", w.HANDLE),
        ("hThread", w.HANDLE),
        ("dwProcessId", w.DWORD),
        ("dwThreadId", w.DWORD),
    ]


def kernel():
    k = ctypes.WinDLL("kernel32", use_last_error=True)
    k.InitializeProcThreadAttributeList.argtypes = [
        ctypes.c_void_p, w.DWORD, w.DWORD, ctypes.POINTER(ctypes.c_size_t)
    ]
    k.InitializeProcThreadAttributeList.restype = w.BOOL
    k.UpdateProcThreadAttribute.argtypes = [
        ctypes.c_void_p, w.DWORD, ctypes.c_size_t,
        ctypes.c_void_p, ctypes.c_size_t, ctypes.c_void_p, ctypes.c_void_p
    ]
    k.UpdateProcThreadAttribute.restype = w.BOOL
    k.DeleteProcThreadAttributeList.argtypes = [ctypes.c_void_p]
    k.CreateProcessW.argtypes = [
        w.LPCWSTR, w.LPWSTR, ctypes.c_void_p, ctypes.c_void_p,
        w.BOOL, w.DWORD, ctypes.c_void_p, w.LPCWSTR,
        ctypes.c_void_p, ctypes.POINTER(ProcessInformation)
    ]
    k.CreateProcessW.restype = w.BOOL
    k.CloseHandle.argtypes = [w.HANDLE]
    k.CloseHandle.restype = w.BOOL
    return k


def checked(value):
    if not value:
        raise ctypes.WinError(ctypes.get_last_error())
    return value


def create_suspended(job_handle, atomic):
    """Use CreateProcessW without inherited handles or caller-supplied arguments.

    In the atomic path an extended attribute associates the Job at creation.
    The fallback case is purposefully unsafe and only used under outer Job.
    """
    k = kernel()
    buffer = None
    initialized = False
    si_ex = StartupInfoEx()
    si_ex.StartupInfo.cb = ctypes.sizeof(si_ex) if atomic else ctypes.sizeof(StartupInfo)
    flags = CREATE_SUSPENDED
    try:
        if atomic:
            size = ctypes.c_size_t()
            k.InitializeProcThreadAttributeList(None, 1, 0, ctypes.byref(size))
            if not size.value:
                raise ctypes.WinError(ctypes.get_last_error())
            buffer = ctypes.create_string_buffer(size.value)
            checked(k.InitializeProcThreadAttributeList(
                buffer, 1, 0, ctypes.byref(size)
            ))
            initialized = True
            hjob = w.HANDLE(job_handle)
            checked(k.UpdateProcThreadAttribute(
                buffer, 0, PROC_THREAD_ATTRIBUTE_JOB_LIST,
                ctypes.byref(hjob), ctypes.sizeof(hjob), None, None
            ))
            si_ex.lpAttributeList = ctypes.cast(buffer, ctypes.c_void_p)
            flags |= EXTENDED_STARTUPINFO_PRESENT

        # Synthetic and fixed: absolutely no user/model/repository command input.
        args = [sys.executable, "-c", "import time; time.sleep(90)"]
        cmdline = ctypes.create_unicode_buffer(subprocess.list2cmdline(args))
        pi = ProcessInformation()
        checked(k.CreateProcessW(
            sys.executable, cmdline, None, None, False, flags, None, None,
            ctypes.byref(si_ex), ctypes.byref(pi)
        ))
        try:
            # Thread is suspended. It is never resumed by this research proof.
            return True
        finally:
            checked(k.CloseHandle(pi.hThread))
            checked(k.CloseHandle(pi.hProcess))
    finally:
        if initialized:
            k.DeleteProcThreadAttributeList(buffer)


def outer_helper():
    p = Path(__file__).parents[1] / "tests" / "doctor_pipe_supervisor.py"
    spec = importlib.util.spec_from_file_location("optic317_outer", p)
    helper = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(helper)
    return helper


def host_worker(mode, root):
    helper = outer_helper()
    helper.wait_for(lambda: (root / "go").exists(), 8, "outer ownership gate")
    job = helper.Job()
    if mode == "atomic":
        create_suspended(job.handle, atomic=True)
        helper.wait_for(lambda: job.active() == 1, 2, "atomic inner Job membership")
        count = job.active()
    elif mode == "legacy":
        create_suspended(job.handle, atomic=False)
        # Deliberately leave process unassigned for the negative control.
        count = job.active()
        assert count == 0
    elif mode == "invalid":
        # A stale/invalid Job cannot silently fall back to an unowned spawn.
        try:
            create_suspended(0x7FFFFFFF, atomic=True)
        except OSError:
            pass
        else:
            raise AssertionError("invalid Job accepted")
        count = job.active()
        assert count == 0
    else:
        raise ValueError("unrecognized research fixture")

    # A separate process must never read a partly written readiness record.
    ready_temp = root / "ready.tmp"
    ready_temp.write_text(json.dumps({
        "inner_job_active": count,
        "mode": mode,
        "suspended": mode != "invalid",
        "rejected": mode == "invalid",
    }), encoding="ascii")
    os.replace(ready_temp, root / "ready")
    # The independent outer supervisor kills this host via its exact handle.
    while True:
        time.sleep(0.05)


def run_case(helper, mode, root):
    job = helper.Job()
    host = None
    start = time.monotonic()
    try:
        with (root / "host.log").open("wb") as log:
            host = subprocess.Popen(
                [sys.executable, str(Path(__file__).resolve()),
                 "--host", mode, "--root", str(root)],
                stdout=log, stderr=log,
            )
            job.assign(host)
            (root / "go").write_text("owned", encoding="ascii")
            helper.wait_for(lambda: (root / "ready").exists() or host.poll() is not None,
                            9, "host process started and created suspended child")
            assert host.poll() is None, "host exited before evidence gate"
            state = json.loads((root / "ready").read_text(encoding="ascii"))
            assert state == {
                "inner_job_active": 1 if mode == "atomic" else 0,
                "mode": mode,
                "suspended": mode != "invalid",
                "rejected": mode == "invalid",
            }
            before_target = 1 if mode == "invalid" else 2
            helper.wait_for(lambda: job.active() == before_target, 3,
                            "outer host/child counts before host exit")
            before = job.active()

            # Kills ONLY the synthetic host created by this test.
            host.kill()
            host.wait(timeout=3)
            target = 1 if mode == "legacy" else 0
            helper.wait_for(lambda: job.active() == target, 3,
                            "accounting after synthetic host death")
            after = job.active()
            return {
                "case": mode,
                "outer_before_host_death": before,
                "outer_after_host_death": after,
                "inner_job_count_before_host_death": state["inner_job_active"],
                "host_terminated_by_supervisor": True,
                "elapsed_ms": round((time.monotonic() - start) * 1000),
            }
    finally:
        # The external independent Job recovers both the host and any
        # deliberately unassigned suspended child, including on assertion error.
        try:
            job.close()
        finally:
            if host is not None:
                if host.poll() is None:
                    host.kill()
                host.wait(timeout=3)
                host._handle.Close()


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--host", choices=["atomic", "legacy", "invalid"])
    p.add_argument("--root")
    p.add_argument("--output")
    a = p.parse_args()
    if os.name != "nt":
        raise RuntimeError("only hosted Windows may execute this native research")
    if a.host:
        if not a.root or a.output:
            raise ValueError("invalid host mode")
        host_worker(a.host, Path(a.root))
        return
    if not a.output or a.root:
        raise ValueError("supervisor requires --output")
    output = Path(a.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    helper = outer_helper()
    proof = {
        "source_sha": os.environ.get("GITHUB_SHA"),
        "run_id": os.environ.get("GITHUB_RUN_ID"),
        "attempt": os.environ.get("GITHUB_RUN_ATTEMPT"),
        "research_only": True,
        "production_changed": False,
        "no_user_pc": True,
        "complete": False,
        "cases": [],
    }
    try:
        with tempfile.TemporaryDirectory(prefix="optic317-job-attribute-") as temp:
            for name in ("legacy", "atomic", "invalid"):
                root = Path(temp) / name
                root.mkdir()
                proof["cases"].append(run_case(helper, name, root))
        legacy, atomic, invalid = proof["cases"]
        assert legacy["outer_before_host_death"] == 2
        assert legacy["outer_after_host_death"] == 1
        assert atomic["outer_before_host_death"] == 2
        assert atomic["outer_after_host_death"] == 0
        assert invalid["outer_before_host_death"] == 1
        assert invalid["outer_after_host_death"] == 0
        proof["invalid_job_failed_closed"] = True
        proof["outer_job_cleaned_after_every_case"] = True
        proof["complete"] = True
    except BaseException as exc:
        proof["failure_class"] = type(exc).__name__
        raise
    finally:
        output.write_text(json.dumps(proof, indent=2), encoding="utf-8")
        print("OPTIC317_ATOMIC_JOB_RESEARCH " + json.dumps(proof, separators=(",", ":")))


if __name__ == "__main__":
    main()
