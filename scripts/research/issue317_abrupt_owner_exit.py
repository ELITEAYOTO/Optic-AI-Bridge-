"""Research-only #317: crash the synthetic native owner under an independent guard."""
import argparse
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time

sys.path.insert(0, str(Path(__file__).resolve().parent))
from issue317_owner_study import Owner, adopted_children, resume_suspended, subreaper
from issue317_supervisor import WindowsJob, pid_alive, reap, wait_for

CRASH_EXIT = 79
CONFIRM_SECONDS = 2


def worker(executable, root):
    subreaper()
    owner = Owner(root)
    owner.start(executable, "both", "none")
    wait_for(lambda: (root / "holder.ready").exists(), 2, "holder ready")
    (root / "crash.ready").write_text("synthetic", encoding="ascii")
    # No finally, Python atexit, owner destructor or thread join may execute.
    os._exit(CRASH_EXIT)


def recover_adopted():
    # Private, single-fixture Linux supervisor only. Pidfds pin known adopted
    # child identities; there is no machine-wide PID search or signal.
    deadline = time.monotonic() + 3
    while time.monotonic() < deadline:
        reap()
        children = adopted_children()
        if not children:
            return
        for pid in children:
            try:
                fd = os.pidfd_open(pid)
            except ProcessLookupError:
                continue
            try:
                try:
                    signal.pidfd_send_signal(fd, signal.SIGKILL)
                except ProcessLookupError:
                    pass
            finally:
                os.close(fd)
        time.sleep(0.01)
    reap()
    assert not adopted_children(), "outer guard failed to reclaim synthetic children"


def supervise(executable, root):
    job = WindowsJob() if os.name == "nt" else None
    proc = None
    confirmed = False
    report = {
        "platform": sys.platform, "case": "abrupt-owner-exit",
        "inner_scope": "kill-on-close-job" if job else "unix-process-group",
        "worker_exit": None, "holder_survived_owner": None,
        "outer_cleanup": "unconfirmed", "production_fix": False,
    }
    try:
        proc = subprocess.Popen(
            [sys.executable, "-I", str(Path(__file__).resolve()),
             "--worker", "--fixture-exe", str(executable), "--root", str(root)],
            cwd=root, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            creationflags=(0x4 | 0x08000000) if job else 0,
            start_new_session=job is None,
        )
        if job:
            job.assign(proc)
            resume_suspended(proc)
        wait_for(lambda: (root / "crash.ready").exists(), 5, "worker prepared native holder")
        proc.wait(timeout=12)
        assert proc.returncode == CRASH_EXIT, "worker did not crash at intended boundary"
        report["worker_exit"] = proc.returncode
        holder = int((root / "holder.pid").read_text(encoding="ascii"))
        if job:
            # The independently retained outer Job is NOT terminated before
            # this check. The worker-owned inner Job should close on OS exit.
            wait_for(lambda: job.active() == 0, CONFIRM_SECONDS,
                     "inner kill-on-close Job quiescence")
            assert not pid_alive(holder), "native holder survived inner Job closure"
            report["holder_survived_owner"] = False
        else:
            wait_for(lambda: holder in adopted_children(), CONFIRM_SECONDS,
                     "known orphan adopted by independent subreaper")
            assert pid_alive(holder), "synthetic Unix holder exited before observation"
            report["holder_survived_owner"] = True
            fd = os.pidfd_open(holder)
            try:
                signal.pidfd_send_signal(fd, signal.SIGKILL)
                deadline = time.monotonic() + CONFIRM_SECONDS
                while time.monotonic() < deadline:
                    try:
                        pid, _ = os.waitpid(holder, os.WNOHANG)
                    except ChildProcessError:
                        pid = holder
                    if pid == holder:
                        break
                    time.sleep(0.005)
                else:
                    raise RuntimeError("known holder not explicitly reaped")
            finally:
                os.close(fd)
            recover_adopted()
        report["outer_cleanup"] = "confirmed"
        confirmed = True
        return report
    finally:
        if job:
            try:
                job.kill()
                wait_for(lambda: job.active() == 0, CONFIRM_SECONDS,
                         "outer fallback Job quiescence")
            finally:
                job.close()
        if proc:
            if proc.poll() is None:
                proc.kill()
            proc.wait(timeout=3)
        if os.name != "nt":
            recover_adopted()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--fixture-exe", required=True)
    parser.add_argument("--output")
    parser.add_argument("--worker", action="store_true")
    parser.add_argument("--root")
    args = parser.parse_args()
    exe = Path(args.fixture_exe).resolve(strict=True)
    if args.worker:
        worker(exe, Path(args.root))
        return
    if not args.output:
        parser.error("--output is required for the supervisor")
    subreaper()
    path = Path(args.output)
    path.parent.mkdir(parents=True, exist_ok=True)
    proof = {
        "sha": os.environ.get("GITHUB_SHA"),
        "run": os.environ.get("GITHUB_RUN_ID"),
        "attempt": os.environ.get("GITHUB_RUN_ATTEMPT"),
        "complete": False, "result": None,
    }
    try:
        with tempfile.TemporaryDirectory(prefix="optic-issue317-crash-") as tmp:
            proof["result"] = supervise(exe, Path(tmp))
        proof["complete"] = True
    finally:
        path.write_text(json.dumps(proof, indent=2), encoding="utf-8")
        print("OPTIC_317_ABRUPT_EXIT " + json.dumps(proof, separators=(",", ":")))


if __name__ == "__main__":
    main()
