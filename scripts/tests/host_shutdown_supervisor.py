"""Hosted-only physical Bridge signal proof; cleanup is not bounded-exit acceptance."""
import argparse
import ctypes
import importlib.util
import json
import os
from pathlib import Path
import signal
import shutil
import subprocess
import sys
import tempfile
import time

LIMIT = 65536


def active_proof():
    spec = importlib.util.spec_from_file_location("active_job_shutdown", Path(__file__).with_name("active_job_shutdown.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def wait_for(predicate, seconds, label):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        if predicate():
            return
        time.sleep(0.01)
    raise RuntimeError("physical host-stop deadline: " + label)


def bounded_read(path):
    if not path.exists():
        return b""
    with path.open("rb") as stream:
        value = stream.read(LIMIT + 1)
    assert len(value) <= LIMIT, "fixture output exceeded bound"
    return value


def phases(path):
    observed = []
    for line in bounded_read(path).decode("utf-8", errors="replace").splitlines(keepends=True):
        # stderr formatting may use several writes. A prefix is not a complete
        # timing record, and must neither crash this observer nor satisfy a gate.
        if line.endswith("\n") and line.startswith("OPTIC_STARTUP phase="):
            phase, separator, elapsed = line[len("OPTIC_STARTUP phase="):].partition(" elapsed_ms=")
            assert separator and phase and elapsed.strip().isdigit(), "malformed complete timing record"
            observed.append(phase)
    return observed


def receive(path, request_id, process):
    found = []

    def ready():
        for line in bounded_read(path).splitlines(keepends=True):
            if not line.endswith(b"\n"):
                continue
            message = json.loads(line.decode("utf-8"))
            if message.get("id") == request_id:
                found.append(message)
                return True
        assert process.poll() is None, "Bridge exited before MCP response"
        return False

    wait_for(ready, 10, "MCP response")
    assert "result" in found[0] and "error" not in found[0], "MCP request failed"
    return found[0]["result"]


def send(process, message):
    process.stdin.write((json.dumps(message) + "\n").encode("utf-8"))
    process.stdin.flush()


def windows_console():
    from ctypes import wintypes as w
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    callback_type = ctypes.WINFUNCTYPE(w.BOOL, w.DWORD)
    callback = callback_type(lambda event: True)
    kernel.SetConsoleCtrlHandler.argtypes = [ctypes.c_void_p, w.BOOL]
    kernel.GetConsoleProcessList.argtypes = [ctypes.POINTER(w.DWORD), w.DWORD]
    kernel.GenerateConsoleCtrlEvent.argtypes = [w.DWORD, w.DWORD]
    # Only this isolated worker's handler is changed. Do not inherit Ctrl+C ignore.
    if not kernel.SetConsoleCtrlHandler(None, False):
        raise ctypes.WinError(ctypes.get_last_error())
    if not kernel.SetConsoleCtrlHandler(ctypes.cast(callback, ctypes.c_void_p), True):
        raise ctypes.WinError(ctypes.get_last_error())

    def deliver(process, event):
        clients = (w.DWORD * 8)()
        count = kernel.GetConsoleProcessList(clients, 8)
        assert 0 < count <= 8, "missing/private console not verified"
        assert set(clients[:count]) == {os.getpid(), process.pid}, "foreign console client; observed=" + str(sorted(clients[:count]))
        # CTRL+C cannot target a nonzero process group. Broadcast only after the
        # private console inventory proves that no runner/other process is present.
        if not kernel.GenerateConsoleCtrlEvent(event, 0):
            raise ctypes.WinError(ctypes.get_last_error())

    return deliver, callback  # Keep the native callback alive until worker exit.


def stdout_pipe_state(process):
    """Borrow this worker's sole stdout reader; observe without consuming bytes."""
    import msvcrt
    from ctypes import wintypes as w
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel.PeekNamedPipe.argtypes = [w.HANDLE, ctypes.c_void_p, w.DWORD,
                                    ctypes.c_void_p, ctypes.POINTER(w.DWORD), ctypes.c_void_p]
    kernel.GetNamedPipeInfo.argtypes = [w.HANDLE, ctypes.c_void_p,
                                      ctypes.POINTER(w.DWORD), ctypes.POINTER(w.DWORD), ctypes.c_void_p]
    handle = w.HANDLE(msvcrt.get_osfhandle(process.stdout.fileno()))
    available, outgoing, incoming = w.DWORD(), w.DWORD(), w.DWORD()
    if not kernel.PeekNamedPipe(handle, None, 0, None, ctypes.byref(available), None):
        raise ctypes.WinError(ctypes.get_last_error())
    if not kernel.GetNamedPipeInfo(handle, None, ctypes.byref(outgoing), ctypes.byref(incoming), None):
        raise ctypes.WinError(ctypes.get_last_error())
    assert available.value <= LIMIT, "stdout queue exceeded fixture bound"
    return dict(queued_bytes=available.value, capacity_bytes=max(outgoing.value, incoming.value))


def receive_pipe(process, request_id):
    data = bytearray()
    found = []

    def ready():
        available = stdout_pipe_state(process)["queued_bytes"]
        if available:
            assert len(data) < LIMIT, "MCP response exceeded fixture bound"
            data.extend(process.stdout.read(min(available, LIMIT - len(data))))
            for line in data.splitlines(keepends=True):
                if line.endswith(b"\n"):
                    message = json.loads(line.decode("utf-8"))
                    if message.get("id") == request_id:
                        found.append(message)
                        return True
        assert process.poll() is None, "Bridge exited before pipe response"
        return False

    wait_for(ready, 10, "pipe MCP response")
    assert "result" in found[0] and "error" not in found[0]
    return found[0]["result"]


def stalled_stdout_case(command, root, deliver):
    """Require joined stdout and root exit while both unread pipe ends stay open."""
    process = None
    log = root / "stderr.log"
    with log.open("wb") as error:
        try:
            process = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                       stderr=error, bufsize=0,
                                       env=dict(os.environ, OPTIC_STARTUP_TIMING="1"))

            def phase(name):
                if name in phases(log):
                    return True
                if process.poll() is not None:
                    assert name in phases(log), "Bridge exited before phase: " + name
                    return True
                return False

            wait_for(lambda: phase("host-shutdown-listening"), 10, "stdout-case listener")
            send(process, {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
                "protocolVersion": "2025-03-26", "capabilities": {},
                "clientInfo": {"name": "stalled-stdout-proof", "version": "0"}}})
            receive_pipe(process, 1)
            send(process, {"jsonrpc": "2.0", "method": "notifications/initialized"})
            wait_for(lambda: phase("mcp-service-started"), 10, "stdout-case initialized")
            send(process, {"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
                "name": "session_info", "arguments": {}}})
            assert receive_pipe(process, 2).get("isError") is not True
            # Stop consuming stdout. Fixed metadata requests only; no admitted
            # file/process/Git work and no production diagnostic hooks added.
            batch = b"".join((json.dumps({"jsonrpc": "2.0", "id": index,
                "method": "tools/call", "params": {"name": "session_info", "arguments": {}}}) + "\n").encode("utf-8")
                for index in range(10, 42))
            assert len(batch) <= 4096
            assert process.stdin.write(batch) == len(batch), "partial fixture request batch"
            process.stdin.flush()
            wait_for(lambda: stdout_pipe_state(process)["queued_bytes"] > 0, 10, "unread output")
            time.sleep(0.25)  # Let fixed responses queue; no stdout reader is running.
            before = stdout_pipe_state(process)
            assert before["queued_bytes"] > 0
            deliver(process, 1)  # Ctrl+Break, only the verified private console.
            wait_for(lambda: phase("host-stop-observed"), 10, "stdout-case stop")
            wait_for(lambda: phase("original-session-cleanup-complete"), 10, "stdout-case cleanup")
            wait_for(lambda: phase("stdin-reader-joined"), 10, "stdout-case input join")
            assert not process.stdin.closed and not process.stdout.closed
            process.wait(timeout=5)
            assert process.returncode == 1 and not process.stdin.closed and not process.stdout.closed
            after = stdout_pipe_state(process)
            assert after["queued_bytes"] > 0, "parent must not drain output to permit exit"
            ordered = phases(log)
            assert ordered.index("host-stop-observed") < ordered.index("original-session-cleanup-complete")
            assert ordered.index("original-session-cleanup-complete") < ordered.index("stdin-reader-joined")
            assert ordered.index("stdin-reader-joined") < ordered.index("stdout-writer-joined")
            result = dict(event="stalled-stdout", initialized=True, characterization_only=False,
                          physical_signal=True, original_cleanup_observed=True, stdin_reader_joined=True,
                          stdout_writer_joined=True, stdin_held_open_through_exit=True,
                          stdout_reader_held_open_through_exit=True, exit_with_unread_stdout=True,
                          stdout_before_stop=before, stdout_after_cleanup=after,
                          released_by_stdout_reader_close=False, final_exit_code=process.returncode)
            (root / "result.json").write_text(json.dumps(result), encoding="utf-8")
        except BaseException:
            detail = bounded_read(log).decode("utf-8", errors="replace")
            print("BRIDGE_STDERR " + detail.replace(str(root), "<fixture>")[-2000:], file=sys.stderr)
            raise
        finally:
            if process is not None:
                if process.poll() is None:
                    process.kill()  # Recovery only; never a successful result.
                process.wait(timeout=2)
                for stream in [process.stdin, process.stdout]:
                    if stream and not stream.closed:
                        stream.close()
                process._handle.Close()


def worker(args):
    root = Path(args.root)
    wait_for(lambda: (root / "go").exists(), 10, "external ownership gate")
    deliver = callback = None
    if os.name == "nt" and args.event not in ["headless", "active-job"]:
        deliver, callback = windows_console()
    command = [str(Path(args.bridge).resolve()), str(root)]
    if args.event in ["active-job", "active-job-signal"]:
        node = Path(args.node).resolve(strict=True)
        assert node.name.lower() == "node.exe"
        command.extend(["--allow-executable=interpreter:" + str(node), "--allow-isolated-node=" + str(node)])
    if args.event == "stalled-stdout":
        return stalled_stdout_case(command, root, deliver)
    environment = dict(os.environ, OPTIC_STARTUP_TIMING="1")
    if args.event in ["active-job", "active-job-signal"]:
        environment["OPTIC_LAUNCHER_PHASES"] = "1"
    process = None
    with (root / "stderr.log").open("wb") as error, (root / "stdout.log").open("wb") as output:
        try:
            process = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=output,
                                       stderr=error, env=environment)
            log = root / "stderr.log"

            def phase(name):
                current = phases(log)
                if name in current:
                    return True
                if process.poll() is not None:
                    assert name in phases(log), "Bridge exited before phase: " + name
                    return True
                return False

            wait_for(lambda: phase("host-shutdown-listening"), 10, "OS listener registration")
            if args.initialized or args.event == "headless":
                send(process, {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
                    "protocolVersion": "2025-03-26", "capabilities": {},
                    "clientInfo": {"name": "physical-host-stop-proof", "version": "0"}}})
                receive(root / "stdout.log", 1, process)
                send(process, {"jsonrpc": "2.0", "method": "notifications/initialized"})
                wait_for(lambda: phase("mcp-service-started"), 10, "initialized service")
                send(process, {"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
                    "name": "session_info", "arguments": {}}})
                assert receive(root / "stdout.log", 2, process).get("isError") is not True
            if args.event in ["active-job", "active-job-signal"]:
                result = active_proof().run_case(process, log, root, node, send, receive, phases, wait_for,
                                               deliver if args.event == "active-job-signal" else None)
            elif args.event == "headless":
                process.stdin.close()
                process.wait(timeout=5)
                assert process.returncode == 0, "headless EOF failed"
                assert "original-session-cleanup-complete" in phases(log)
                assert "stdin-reader-joined" in phases(log)
                assert "stdout-writer-joined" in phases(log)
                result = dict(event="headless", initialized=True, eof_exit_code=0,
                              cleanup_marker=True, stdin_reader_joined=True,
                              stdout_writer_joined=True, physical_signal=False)
            else:
                if os.name == "nt":
                    deliver(process, {"ctrl-c": 0, "ctrl-break": 1}[args.event])
                else:
                    os.kill(process.pid, {"sigint": signal.SIGINT, "sigterm": signal.SIGTERM}[args.event])
                wait_for(lambda: phase("host-stop-observed"), 10, "physical signal receipt")
                wait_for(lambda: phase("original-session-cleanup-complete"), 10, "original cleanup")
                assert not process.stdin.closed, "input was closed before cleanup proof"
                if os.name == "nt":
                    # Windows must leave the parent input open through root exit.
                    # Closing it to unblock a reader is no longer an acceptance path.
                    process.wait(timeout=5)
                    assert not process.stdin.closed
                    assert "stdin-reader-joined" in phases(log)
                    assert "stdout-writer-joined" in phases(log)
                    open_stdin_exit = True
                else:
                    # Unix remains characterization, separate from Windows acceptance.
                    time.sleep(0.5)
                    retained = process.poll() is None
                    open_stdin_exit = False
                process.stdin.close()
                process.wait(timeout=5)
                assert process.returncode == 1, "expected preserved Interrupted failure"
                ordered = phases(log)
                assert ordered.index("host-shutdown-listening") < ordered.index("host-stop-observed")
                assert ordered.index("host-stop-observed") < ordered.index("original-session-cleanup-complete")
                result = dict(event=args.event, initialized=args.initialized,
                              physical_signal=True, cleanup_with_stdin_open=True,
                              exit_with_stdin_open=open_stdin_exit,
                              final_exit_code=process.returncode)
                if os.name == "nt":
                    assert ordered.index("original-session-cleanup-complete") < ordered.index("stdin-reader-joined")
                    result["stdin_reader_joined"] = True
                    assert ordered.index("stdin-reader-joined") < ordered.index("stdout-writer-joined")
                    result["stdout_writer_joined"] = True
                else:
                    result.update(alive_500ms_after_cleanup=retained,
                                  exit_after_input_close=process.returncode)
            (root / "result.json").write_text(json.dumps(result), encoding="utf-8")
        except BaseException:
            detail = bounded_read(root / "stderr.log").decode("utf-8", errors="replace")
            print("BRIDGE_STDERR " + detail.replace(str(root), "<fixture>")[-2000:], file=sys.stderr)
            raise
        finally:
            if process is not None:
                if process.poll() is None:
                    process.kill()  # Failure recovery only; never a successful receipt.
                process.wait(timeout=2)
                if process.stdin and not process.stdin.closed:
                    process.stdin.close()
                if os.name == "nt":
                    process._handle.Close()


def supervise(args):
    bridge = Path(args.bridge).resolve()
    assert bridge.is_file(), "built Bridge binary missing"
    guard = None
    if os.name == "nt":
        spec = importlib.util.spec_from_file_location("guard", Path(__file__).with_name("doctor_pipe_supervisor.py"))
        guard = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(guard)
    events = ["ctrl-c", "ctrl-break"] if os.name == "nt" else ["sigint", "sigterm"]
    cases = [(event, initialized) for event in events for initialized in [False, True]]
    if os.name == "nt":
        cases.append(("headless", True))
        cases.append(("stalled-stdout", True))
        cases.append(("active-job", True))
        cases.append(("active-job-signal", True))
    result = dict(schema_version=6, sha=os.environ.get("OPTIC_SOURCE_SHA") or os.environ["GITHUB_SHA"],
                  checkout_sha=os.environ["GITHUB_SHA"], run=os.environ["GITHUB_RUN_ID"],
                  attempt=os.environ["GITHUB_RUN_ATTEMPT"], complete=False, cases=[],
                  claim="Windows ordinary stdio and active isolated Node EOF/physical-stop exit; not held-admission/native Git acceptance; Unix characterization only")
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    try:
        with tempfile.TemporaryDirectory(prefix="optic-physical-stop-") as temporary:
            for index, (event, initialized) in enumerate(cases):
                root = Path(temporary) / str(index)
                root.mkdir()
                command = [sys.executable, "-I", str(Path(__file__).resolve()), "--worker",
                           "--bridge", str(bridge), "--root", str(root), "--event", event]
                if initialized:
                    command.append("--initialized")
                if event in ["active-job", "active-job-signal"]:
                    node = shutil.which("node.exe")
                    assert node, "hosted operator Node missing"
                    command.extend(["--node", str(Path(node).resolve(strict=True))])
                options = dict(start_new_session=True) if os.name != "nt" else {}
                if os.name == "nt":
                    startup = subprocess.STARTUPINFO()
                    startup.dwFlags |= subprocess.STARTF_USESHOWWINDOW
                    startup.wShowWindow = 0
                    options.update(startupinfo=startup, creationflags=(
                        subprocess.CREATE_NO_WINDOW if event in ["headless", "active-job"] else subprocess.CREATE_NEW_CONSOLE))
                job = guard.Job() if guard else None
                process = None
                completed = False
                try:
                    with (root / "worker.log").open("wb") as log:
                        process = subprocess.Popen(command, stdin=subprocess.DEVNULL,
                                                   stdout=log, stderr=log, **options)
                        if job:
                            job.assign(process)
                        (root / "go").write_text("owned", encoding="ascii")
                        if job:
                            worker_deadline = time.monotonic() + 45
                            if event in ["active-job", "active-job-signal"]:
                                wait_for(lambda: (root / "active-ready.json").exists() or process.poll() is not None,
                                         25, "managed process readiness handoff")
                                assert process.poll() is None, "active-job worker exited before ownership check"
                                ready = json.loads(bounded_read(root / "active-ready.json").decode("utf-8"))
                                assert ready["pid"] in active_proof().job_members(job), "managed PID absent from original outer Job"
                                active_proof().publish_json(root / "active-owned.json", ready)
                            process.wait(timeout=max(0.001, worker_deadline - time.monotonic()))
                            worker_exit = process.returncode
                        else:
                            # Keep the worker unreaped until recovery is decided,
                            # so its private process-group ID cannot be reused.
                            exited = []

                            def worker_exited():
                                status = os.waitid(os.P_PID, process.pid,
                                                   os.WEXITED | os.WNOHANG | os.WNOWAIT)
                                if status is not None:
                                    exited.append(status)
                                    return True
                                return False

                            wait_for(worker_exited, 45, "worker exit")
                            status = exited[0]
                            worker_exit = status.si_status if status.si_code == os.CLD_EXITED else -status.si_status
                    assert worker_exit == 0, "physical fixture failed: " + event + ": " + bounded_read(root / "worker.log").decode("utf-8", errors="replace")[-2000:]
                    case = json.loads((root / "result.json").read_text(encoding="utf-8"))
                    if job:
                        wait_for(lambda: job.active() == 0, 2, "Job empty after normal fixture exit")
                    case["supervisor_cleanup"] = "normal-exit-job-empty" if job else "normal-exit-worker-reaped"
                    result["cases"].append(case)
                    completed = True
                finally:
                    try:
                        if job:
                            job.close()
                        elif process is not None and not completed:
                            try:
                                os.killpg(process.pid, signal.SIGKILL)
                            except ProcessLookupError:
                                pass
                    finally:
                        if process is not None:
                            if process.poll() is None:
                                process.kill()
                            process.wait(timeout=2)
                            if os.name == "nt":
                                process._handle.Close()
                output.write_text(json.dumps(result, indent=2), encoding="utf-8")
        result["complete"] = True
    finally:
        output.write_text(json.dumps(result, indent=2), encoding="utf-8")
        print("OPTIC_PHYSICAL_HOST_STOP " + json.dumps(result, separators=(",", ":")))


if __name__ == "__main__":
    assert os.environ.get("GITHUB_ACTIONS") == "true" and os.environ.get("RUNNER_ENVIRONMENT") == "github-hosted", "GitHub-hosted fixture only"
    parser = argparse.ArgumentParser()
    parser.add_argument("--bridge", required=True)
    parser.add_argument("--output")
    parser.add_argument("--worker", action="store_true")
    parser.add_argument("--root")
    parser.add_argument("--event", choices=["ctrl-c", "ctrl-break", "sigint", "sigterm", "headless", "stalled-stdout", "active-job", "active-job-signal"])
    parser.add_argument("--node")
    parser.add_argument("--initialized", action="store_true")
    options = parser.parse_args()
    worker(options) if options.worker else supervise(options)
