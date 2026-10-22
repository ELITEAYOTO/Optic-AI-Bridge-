"""Fixed-cardinality native observations of verified outer-Job processes only.

Never opens unrelated processes, logs PIDs/paths, changes protection or kills by
PID. A separate kill-on-close Job owns the gated Rust test before effects.
"""
import argparse
import ctypes
from ctypes import wintypes as w
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import time


class Observer:
    def __init__(self, job):
        self.job = job
        self.k = ctypes.WinDLL("kernel32", use_last_error=True)
        self.p = ctypes.WinDLL("psapi", use_last_error=True)
        self.k.OpenProcess.argtypes = [w.DWORD, w.BOOL, w.DWORD]
        self.k.OpenProcess.restype = w.HANDLE
        self.k.IsProcessInJob.argtypes = [w.HANDLE, w.HANDLE, ctypes.POINTER(w.BOOL)]
        self.k.QueryFullProcessImageNameW.argtypes = [w.HANDLE, w.DWORD, w.LPWSTR, ctypes.POINTER(w.DWORD)]
        self.k.GetProcessTimes.argtypes = [w.HANDLE] + [ctypes.POINTER(w.FILETIME)] * 4
        self.k.GetProcessHandleCount.argtypes = [w.HANDLE, ctypes.POINTER(w.DWORD)]
        self.k.GetProcessIoCounters.argtypes = [w.HANDLE, ctypes.c_void_p]
        self.p.GetProcessMemoryInfo.argtypes = [w.HANDLE, ctypes.c_void_p, w.DWORD]
        self.k.CloseHandle.argtypes = [w.HANDLE]

    def snapshot(self):
        class Ids(ctypes.Structure):
            _fields_ = [("assigned", w.DWORD), ("count", w.DWORD), ("ids", ctypes.c_size_t * 8)]
        class Memory(ctypes.Structure):
            _fields_ = [("cb", w.DWORD), ("faults", w.DWORD)] + [(name, ctypes.c_size_t) for name in
                ["peak_ws", "ws", "peak_pp", "pp", "peak_np", "np", "pagefile", "peak_pagefile"]]
        class Io(ctypes.Structure):
            _fields_ = [(name, ctypes.c_ulonglong) for name in ["reads", "writes", "other", "read_bytes", "write_bytes", "other_bytes"]]
        ids = Ids()
        if not self.job.k.QueryInformationJobObject(self.job.handle, 3, ctypes.byref(ids), ctypes.sizeof(ids), None):
            raise ctypes.WinError(ctypes.get_last_error())
        assert ids.count <= 8, "owned process observation ceiling"
        rows = []
        for pid in ids.ids[:ids.count]:
            # Query only a listed Job member, then revalidate the opened kernel
            # handle to reject a PID-reuse race. Never accept a PID as authority.
            handle = self.k.OpenProcess(0x400 | 0x10, False, pid)
            if not handle:
                rows.append({"role": "unavailable", "query_error": ctypes.get_last_error()})
                continue
            try:
                member = w.BOOL()
                if not self.k.IsProcessInJob(handle, self.job.handle, ctypes.byref(member)) or not member.value:
                    rows.append({"role": "membership-raced"})
                    continue
                name = ctypes.create_unicode_buffer(1024)
                size = w.DWORD(len(name))
                role = "owned-other"
                if self.k.QueryFullProcessImageNameW(handle, 0, name, ctypes.byref(size)):
                    base = Path(name.value).name.lower()
                    role = ("node" if base == "node.exe" else
                            "launcher" if base == "optic-bridge-isolation-launcher.exe" else
                            "test-host" if base.startswith("appcontainer_network_isolation_windows-") else "owned-other")
                created, exited, kernel, user = (w.FILETIME() for _ in range(4))
                memory, io, handles = Memory(), Io(), w.DWORD()
                memory.cb = ctypes.sizeof(memory)
                row = {"role": role}
                if self.k.GetProcessTimes(handle, ctypes.byref(created), ctypes.byref(exited), ctypes.byref(kernel), ctypes.byref(user)):
                    for label, value in [("kernel_ms", kernel), ("user_ms", user)]:
                        row[label] = ((value.dwHighDateTime << 32) | value.dwLowDateTime) // 10000
                if self.k.GetProcessHandleCount(handle, ctypes.byref(handles)):
                    row["handles"] = handles.value
                if self.k.GetProcessIoCounters(handle, ctypes.byref(io)):
                    row.update(read_bytes=io.read_bytes, write_bytes=io.write_bytes, reads=io.reads, writes=io.writes)
                if self.p.GetProcessMemoryInfo(handle, ctypes.byref(memory), ctypes.sizeof(memory)):
                    row.update(working_set_bytes=memory.ws, page_faults=memory.faults)
                rows.append(row)
            finally:
                self.k.CloseHandle(handle)
        return rows


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--test-exe", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--trials", type=int, choices=[1, 2, 3], default=3)
    args = parser.parse_args()
    assert os.name == "nt" and ctypes.sizeof(ctypes.c_void_p) == 8
    helper = Path(__file__).parents[1] / "tests" / "doctor_pipe_supervisor.py"
    spec = importlib.util.spec_from_file_location("outer_job", helper)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    result = {"source_sha": os.environ.get("OPTIC_SOURCE_SHA", os.environ.get("GITHUB_SHA")),
              "run": os.environ.get("GITHUB_RUN_ID"), "attempt": os.environ.get("GITHUB_RUN_ATTEMPT"),
              "complete": False, "trials": [], "original_gate_replaced": False}
    try:
        # Fresh-process observations; no claim about OS/file-cache coldness.
        # Stop at the first failure, no rerun-to-green or budget relaxation.
        for trial in range(1, args.trials + 1):
            temp = tempfile.TemporaryDirectory(prefix="optic185-owned-")
            root = Path(temp.name)
            job, process = module.Job(), None
            item = {"trial": trial, "snapshots": [], "outer_job_empty_confirmed": False}
            result["trials"].append(item)
            start = time.monotonic()
            try:
                env = os.environ.copy()
                env.update(OPTIC_NETWORK_SUPERVISOR_GATE=str(root / "go"), OPTIC_REQUIRE_HOSTED_TOOLCHAINS="1", OPTIC_LAUNCHER_PHASES="1")
                with (root / "log").open("wb") as log:
                    process = subprocess.Popen([args.test_exe, "--exact", "capability_free_appcontainer_denies_node_loopback_connection", "--nocapture", "--test-threads=1"], env=env, stdout=log, stderr=log)
                    job.assign(process)
                    observer = Observer(job)
                    (root / "go").write_text("owned", encoding="ascii")
                    launched = time.monotonic()
                    for scheduled in [0.1, 1.0, 3.0, 8.0]:
                        remaining = scheduled - (time.monotonic() - launched)
                        if remaining > 0:
                            try:
                                process.wait(timeout=remaining)
                                break
                            except subprocess.TimeoutExpired:
                                pass
                        if process.poll() is not None:
                            break
                        item["snapshots"].append({"elapsed_ms": round((time.monotonic() - launched) * 1000), "processes": observer.snapshot()})
                        output.write_text(json.dumps(result, indent=2), encoding="utf-8")
                    process.wait(timeout=max(0.1, 15 - (time.monotonic() - launched)))
                item["exit_code"] = process.returncode
                log = (root / "log").read_text(errors="replace")
                item["diagnostics"] = [re.sub(r'executable=".*?" manager_ready_ms=', 'executable="<node>" manager_ready_ms=', line)
                                       for line in log.splitlines() if line.startswith("OPTIC_NETWORK_ISOLATION ") or " ... OPTIC_NETWORK_ISOLATION " in line]
                # The libtest prefix is present when tests are serialized.
                if not item["diagnostics"]:
                    item["diagnostics"] = [re.sub(r'executable=".*?" manager_ready_ms=', 'executable="<node>" manager_ready_ms=', line)
                                           for line in log.splitlines() if "OPTIC_NETWORK_ISOLATION executable=" in line]
                assert len(item["diagnostics"]) == 1
            finally:
                try:
                    job.close()
                    item["outer_job_empty_confirmed"] = True
                finally:
                    if process:
                        process.wait(timeout=2)
                        process._handle.Close()
                    temp.cleanup()
                    item["elapsed_ms"] = round((time.monotonic() - start) * 1000)
                    output.write_text(json.dumps(result, indent=2), encoding="utf-8")
            if item["exit_code"] != 0:
                result["natural_failure_observed"] = True
                break
        result["complete"] = True
    finally:
        output.write_text(json.dumps(result, indent=2), encoding="utf-8")
        print(json.dumps(result))
    if result.get("natural_failure_observed"):
        raise SystemExit(1)


if __name__ == "__main__":
    main()
