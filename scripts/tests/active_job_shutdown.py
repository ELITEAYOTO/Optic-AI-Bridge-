"""Hosted physical supervisor helper; no standalone execution or product hooks."""
import base64
import ctypes
from pathlib import Path
import json
import os
import re
import secrets
import sys
import time


def publish_json(path, value):
    text = json.dumps(value)
    assert len(text.encode("utf-8")) <= 256, "ownership record exceeded fixture bound"
    pending = path.with_name(path.name + ".pending")
    with pending.open("x", encoding="utf-8") as stream:
        stream.write(text)
    pending.replace(path)  # Reader never sees a partial ownership record.


def node_program(nonce):
    assert re.fullmatch(r"[a-f0-9]{32}", nonce), "invalid fixture nonce"
    return "process.stdout.write('OPTIC_ACTIVE " + nonce + " '+process.pid+'\\n');setInterval(()=>{},1000);"


def ready_pid(snapshot, nonce):
    """Parse only a bounded complete record from the actual managed stdout."""
    assert len(snapshot) <= 256, "unexpected active-job output size"
    if not snapshot.endswith(b"\n"):
        return None
    match = re.fullmatch(rb"OPTIC_ACTIVE ([a-f0-9]{32}) ([1-9][0-9]*)\n", snapshot)
    assert match and match[1].decode("ascii") == nonce, "wrong active-job readiness record"
    pid = int(match[2])
    assert pid <= 0xFFFFFFFF, "PID exceeds Windows DWORD"
    return pid


def job_members(job):
    """Query the original external Job, including nested Jobs; never kill by PID."""
    from ctypes import wintypes as w
    class Members(ctypes.Structure):
        _fields_ = [("assigned", w.DWORD), ("count", w.DWORD),
                    ("pids", ctypes.c_size_t * 16)]
    value = Members()
    if not job.k.QueryInformationJobObject(job.handle, 3, ctypes.byref(value),
                                          ctypes.sizeof(value), None):
        raise ctypes.WinError(ctypes.get_last_error())
    assert value.assigned == value.count <= 16, "incomplete external Job inventory"
    return set(value.pids[:value.count])


class ObservedProcess:
    """Read-only, non-inherited handle retained across EOF and process exit."""
    def __init__(self, pid, executable):
        from ctypes import wintypes as w
        self.k = ctypes.WinDLL("kernel32", use_last_error=True)
        self.k.OpenProcess.argtypes = [w.DWORD, w.BOOL, w.DWORD]
        self.k.OpenProcess.restype = w.HANDLE
        self.k.WaitForSingleObject.argtypes = [w.HANDLE, w.DWORD]
        self.k.WaitForSingleObject.restype = w.DWORD
        self.k.QueryFullProcessImageNameW.argtypes = [w.HANDLE, w.DWORD, w.LPWSTR,
                                                     ctypes.POINTER(w.DWORD)]
        self.k.CloseHandle.argtypes = [w.HANDLE]
        self.handle = self.k.OpenProcess(0x100000 | 0x1000, False, pid)
        if not self.handle:
            raise ctypes.WinError(ctypes.get_last_error())
        try:
            name = ctypes.create_unicode_buffer(32768)
            length = w.DWORD(len(name))
            if not self.k.QueryFullProcessImageNameW(self.handle, 0, name, ctypes.byref(length)):
                raise ctypes.WinError(ctypes.get_last_error())
            assert Path(name.value).samefile(executable), "managed PID image differs from operator Node"
            security = ctypes.WinDLL("advapi32", use_last_error=True)
            security.OpenProcessToken.argtypes = [w.HANDLE, w.DWORD, ctypes.POINTER(w.HANDLE)]
            security.GetTokenInformation.argtypes = [w.HANDLE, ctypes.c_int, ctypes.c_void_p,
                                                     w.DWORD, ctypes.POINTER(w.DWORD)]
            token = w.HANDLE()
            if not security.OpenProcessToken(self.handle, 0x0008, ctypes.byref(token)):
                raise ctypes.WinError(ctypes.get_last_error())
            try:
                is_appcontainer, returned = w.DWORD(), w.DWORD()
                if not security.GetTokenInformation(token, 29, ctypes.byref(is_appcontainer),
                                                     ctypes.sizeof(is_appcontainer), ctypes.byref(returned)):
                    raise ctypes.WinError(ctypes.get_last_error())
                assert returned.value == 4 and is_appcontainer.value == 1, "target token is not AppContainer"
            finally:
                self.k.CloseHandle(token)
            self.require_running()
        except BaseException:
            self.close()
            raise

    def require_running(self):
        assert self.k.WaitForSingleObject(self.handle, 0) == 258, "managed Node is not alive"

    def require_exited(self, remaining_seconds):
        assert remaining_seconds > 0, "shared exit deadline exhausted"
        result = self.k.WaitForSingleObject(self.handle, max(1, int(remaining_seconds * 1000)))
        assert result == 0, "managed Node did not reach kernel-signaled exit before deadline"

    def close(self):
        if self.handle:
            self.k.CloseHandle(self.handle)
            self.handle = None


def run_case(process, log, root, executable, send, receive, phases, wait_for, signal_stop=None):
    assert os.name == "nt", "Windows managed-job proof only"
    nonce = secrets.token_hex(16)
    next_id = 10

    def tool(name, arguments):
        nonlocal next_id
        request_id = next_id
        next_id += 1
        send(process, {"jsonrpc": "2.0", "id": request_id, "method": "tools/call",
                       "params": {"name": name, "arguments": arguments}})
        result = receive(root / "stdout.log", request_id, process)
        assert result.get("isError") is not True, "managed tool error: " + json.dumps(result)
        payload = result.get("structuredContent")
        assert isinstance(payload, dict), "managed tool returned no structured payload"
        return payload

    launched_at = time.monotonic()
    script = node_program(nonce)
    job_id = tool("process_start", dict(executable=str(executable), args=["-e", script],
                  cwd=None, env_allowlist=[], network=False, timeout_ms=30000,
                  output_budget=4096, memory_bytes=268435456, process_count=1))["job_id"]
    found = []

    def ready():
        payload = tool("process_read", dict(job_id=job_id, stream="stdout", cursor=0, max_bytes=256))
        assert payload["encoding"] == "base64" and not payload["truncated"]
        pid = ready_pid(base64.b64decode(payload["data"], validate=True), nonce)
        if pid is not None:
            found.append(pid)
            return True
        assert not payload["eof"], "managed Node exited before readiness"
        time.sleep(0.15)  # Bound RPC/log volume inside the unchanged 10s horizon.
        return False

    try:
        wait_for(ready, 10, "actual managed Node readiness")
    except Exception:
        # Fixed phase evidence for the FIRST failure, not a retry or fallback.
        try:
            status = tool("process_result", dict(job_id=job_id))
            error = tool("process_read", dict(job_id=job_id, stream="stderr", cursor=0, max_bytes=4096))
            detail = base64.b64decode(error["data"], validate=True).decode("utf-8", errors="replace")
            print("ACTIVE_JOB_FAILURE " + json.dumps(dict(status=status, stderr=detail)), file=sys.stderr)
        except Exception as diagnostic:
            print("ACTIVE_JOB_DIAGNOSTIC_FAILED " + type(diagnostic).__name__, file=sys.stderr)
        raise
    pid = found[0]
    observed = ObservedProcess(pid, executable)
    try:
        # Parent must attest membership in its exact retained external Job,
        # not just any Job or a later lookup of this numeric PID.
        publish_json(root / "active-ready.json", dict(pid=pid, nonce=nonce))
        wait_for(lambda: (root / "active-owned.json").exists(), 5, "original external Job ownership")
        assert json.loads((root / "active-owned.json").read_text(encoding="utf-8")) == dict(pid=pid, nonce=nonce)
        assert tool("process_result", dict(job_id=job_id))["status"] == "running"
        observed.require_running()
        before_eof_ms = round((time.monotonic() - launched_at) * 1000)
        assert before_eof_ms < 20000, "insufficient distance from the 30s child budget"
        stopped_at = time.monotonic()
        deadline = stopped_at + 5
        if signal_stop is None:
            process.stdin.close()  # Actual client transport EOF; no process_stop RPC.
        else:
            signal_stop(process, 1)  # Existing strict private-console guard remains.
        process.wait(timeout=max(0.001, deadline - time.monotonic()))
        expected_exit = 0 if signal_stop is None else 1
        assert process.returncode == expected_exit, "Bridge lost expected shutdown result"
        observed.require_exited(deadline - time.monotonic())
        completed_at = time.monotonic()
        assert completed_at <= deadline, "combined Bridge/Node observation exceeded 5s"
        assert (time.monotonic() - launched_at) * 1000 < 30000, "natural job timeout could explain exit"
        ordered = phases(log)
        if signal_stop is not None:
            assert not process.stdin.closed, "signal case must not close input to force exit"
            assert ordered.index("host-stop-observed") < ordered.index("original-session-cleanup-complete")
        assert ordered.index("original-session-cleanup-complete") < ordered.index("stdin-reader-joined")
        assert ordered.index("stdin-reader-joined") < ordered.index("stdout-writer-joined")
        return dict(event="active-job" if signal_stop is None else "active-job-signal",
                    initialized=True, physical_signal=signal_stop is not None,
                    managed_job_running_before_shutdown=True, readiness_nonce_matched=True,
                    native_image_matches_operator=True, original_outer_job_member=True,
                    native_appcontainer_token_verified=True,
                    retained_process_handle_signaled=True, before_shutdown_ms=before_eof_ms,
                    shutdown_to_kernel_exit_ms=round((completed_at - stopped_at) * 1000),
                    natural_timeout_reached=False, process_stop_rpc=False, bridge_exit_code=expected_exit,
                    stdin_held_open_through_exit=signal_stop is not None,
                    stdin_reader_joined=True, stdout_writer_joined=True,
                    held_admission_proven=False, native_git_proven=False)
    finally:
        observed.close()  # No termination right requested or process kill issued.
