"""#317 research only: PR_SET_PDEATHSIG is not a descendant-tree containment rule.

Hosted Ubuntu, independent test-only outer subreaper. No Git effect, no
production process owner, no cgroup mutation, no user supplied PID.
Every created process self-expires, and the outer supervisor explicitly
stops/reaps its recorded descendants on success and failure.
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
import time

PR_SET_PDEATHSIG = 1
PR_SET_CHILD_SUBREAPER = 36
AUTONOMOUS_CEILING_SECONDS = 9
WAIT_SECONDS = 4

libc = ctypes.CDLL(None, use_errno=True)
libc.prctl.argtypes = [ctypes.c_int, ctypes.c_ulong, ctypes.c_ulong,
                       ctypes.c_ulong, ctypes.c_ulong]
libc.prctl.restype = ctypes.c_int


def prctl(setting, value):
    if libc.prctl(setting, value, 0, 0, 0) != 0:
        raise OSError(ctypes.get_errno(), "prctl failed")


def await_true(predicate, seconds, reason):
    until = time.monotonic() + seconds
    while time.monotonic() < until:
        if predicate():
            return
        time.sleep(0.005)
    raise RuntimeError("bounded fixture failure: " + reason)


def read_pid(path):
    # Used only for PIDs minted by our own disposable fixture and then pinned
    # with pidfd_open before signaling. Never use externally supplied PIDs.
    pid = int(path.read_text(encoding="ascii"))
    if pid <= 1:
        raise ValueError("invalid fixture PID")
    return pid


def running(pid):
    try:
        state = Path("/proc", str(pid), "stat").read_text().rsplit(") ", 1)[1][0]
        return state != "Z"
    except FileNotFoundError:
        return False


def join_known(pid, seconds):
    until = time.monotonic() + seconds
    while time.monotonic() < until:
        try:
            actual, status = os.waitpid(pid, os.WNOHANG)
        except ChildProcessError:
            return None
        if actual == pid:
            return status
        time.sleep(0.005)
    raise RuntimeError("fixture child not reaped within deadline")


def pidfd_signal(fd):
    if fd is None:
        return
    try:
        signal.pidfd_send_signal(fd, signal.SIGKILL)
    except ProcessLookupError:
        pass


def fixture_holder(root):
    (root / "holder.pid").write_text(str(os.getpid()), encoding="ascii")
    (root / "holder.ready").write_text("ready", encoding="ascii")
    time.sleep(AUTONOMOUS_CEILING_SECONDS)


def fixture_guarded(root, expected_parent):
    # Set signal AFTER creation; prevent race if the creating thread died
    # before prctl was installed.
    prctl(PR_SET_PDEATHSIG, signal.SIGKILL)
    if os.getppid() != expected_parent:
        raise RuntimeError("parent exited before PDEATHSIG armed")
    holder = subprocess.Popen(
        [sys.executable, "-I", str(Path(__file__).resolve()), "--role", "holder",
         "--root", str(root)],
        stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL, close_fds=True, start_new_session=True,
    )
    (root / "holder-child.pid").write_text(str(holder.pid), encoding="ascii")
    await_true(lambda: (root / "holder.ready").is_file(), WAIT_SECONDS,
               "holder initialization")
    (root / "guarded.armed").write_text("armed", encoding="ascii")
    time.sleep(AUTONOMOUS_CEILING_SECONDS)


def fixture_parent(root):
    parent = os.getpid()
    guarded = subprocess.Popen(
        [sys.executable, "-I", str(Path(__file__).resolve()), "--role", "guarded",
         "--root", str(root), "--expected-parent", str(parent)],
        stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL, close_fds=True, start_new_session=True,
    )
    (root / "guarded.pid").write_text(str(guarded.pid), encoding="ascii")
    await_true(lambda: (root / "guarded.armed").is_file(), WAIT_SECONDS,
               "direct child armed")
    await_true(lambda: (root / "holder.ready").is_file(), WAIT_SECONDS,
               "grandchild ready")
    (root / "parent.ready").write_text("ready", encoding="ascii")
    time.sleep(AUTONOMOUS_CEILING_SECONDS)


def experiment(root):
    # Only this disposable outer supervisor becomes a subreaper.
    # A Bridge-global subreaper is NOT proposed.
    prctl(PR_SET_CHILD_SUBREAPER, 1)
    parent = None
    guarded_pid = holder_pid = None
    guarded_fd = holder_fd = None
    guarded_reaped = holder_reaped = False
    record = {
        "parent_sigkill": False,
        "direct_child_pdeathsig_kill": False,
        "grandchild_survived_both": False,
        "grandchild_new_session": False,
        "external_supervisor_reaped": False,
        "production_owner_proven": False,
    }
    try:
        parent = subprocess.Popen(
            [sys.executable, "-I", str(Path(__file__).resolve()), "--role", "parent",
             "--root", str(root)],
            stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL, close_fds=True, start_new_session=True,
        )
        await_true(lambda: (root / "parent.ready").is_file(), WAIT_SECONDS,
                   "parent and descendants ready")
        guarded_pid = read_pid(root / "guarded.pid")
        holder_pid = read_pid(root / "holder.pid")
        if holder_pid != read_pid(root / "holder-child.pid"):
            raise AssertionError("holder PID disagrees across fixture markers")
        guarded_fd = os.pidfd_open(guarded_pid)
        holder_fd = os.pidfd_open(holder_pid)
        assert running(guarded_pid) and running(holder_pid)
        record["grandchild_new_session"] = os.getsid(holder_pid) == holder_pid
        assert record["grandchild_new_session"], "escape setup not established"

        parent.kill()  # only the known Popen handle, not a process group
        assert parent.wait(timeout=WAIT_SECONDS) == -signal.SIGKILL
        record["parent_sigkill"] = True
        direct_status = join_known(guarded_pid, WAIT_SECONDS)
        guarded_reaped = True
        record["direct_child_pdeathsig_kill"] = bool(
            direct_status is not None
            and os.WIFSIGNALED(direct_status)
            and os.WTERMSIG(direct_status) == signal.SIGKILL
        )
        assert record["direct_child_pdeathsig_kill"], "PDEATHSIG not proven"
        record["grandchild_survived_both"] = running(holder_pid)
        assert record["grandchild_survived_both"], "grandchild not alive after both exits"

        pidfd_signal(holder_fd)
        holder_status = join_known(holder_pid, WAIT_SECONDS)
        holder_reaped = True
        assert holder_status is not None and os.WIFSIGNALED(holder_status)
        record["external_supervisor_reaped"] = True
        return record
    finally:
        if parent is not None:
            if parent.poll() is None:
                parent.kill()
            parent.wait(timeout=WAIT_SECONDS)
        pidfd_signal(guarded_fd)
        pidfd_signal(holder_fd)
        # No arbitrary /proc walk, killpg, or unpinned machine-wide signals.
        if guarded_pid is not None and not guarded_reaped:
            join_known(guarded_pid, WAIT_SECONDS)
        if holder_pid is not None and not holder_reaped:
            join_known(holder_pid, WAIT_SECONDS)
        for fd in (guarded_fd, holder_fd):
            if fd is not None:
                os.close(fd)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--role", choices=["outer", "parent", "guarded", "holder"],
                        default="outer")
    parser.add_argument("--root")
    parser.add_argument("--expected-parent", type=int)
    parser.add_argument("--output")
    args = parser.parse_args()
    if args.role != "outer":
        root = Path(args.root)
        if args.role == "parent":
            fixture_parent(root)
        elif args.role == "guarded":
            fixture_guarded(root, args.expected_parent)
        else:
            fixture_holder(root)
        return
    if not args.output:
        parser.error("outer role requires --output")
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    evidence = {
        "sha": os.environ.get("GITHUB_SHA"),
        "run": os.environ.get("GITHUB_RUN_ID"),
        "attempt": os.environ.get("GITHUB_RUN_ATTEMPT"),
        "complete": False,
        "result": None,
    }
    try:
        with tempfile.TemporaryDirectory(prefix="optic-issue317-pdeathsig-") as tmp:
            evidence["result"] = experiment(Path(tmp))
        evidence["complete"] = True
    finally:
        output.write_text(json.dumps(evidence, indent=2), encoding="utf-8")
        print("OPTIC_317_PDEATHSIG " + json.dumps(evidence, separators=(",", ":")))


if __name__ == "__main__":
    main()
