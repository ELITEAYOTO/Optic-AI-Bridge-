"""Independently supervise Rust Win32 atomic Job-list tests on hosted Windows.

No production execution, no user PC and no arbitrary PID termination.
One pre-armed external Job owns each test host before the Rust gate opens.
"""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time


def load_supervisor():
    path = Path(__file__).parents[1] / "tests" / "doctor_pipe_supervisor.py"
    spec = importlib.util.spec_from_file_location("optic317_outer_job", path)
    helper = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(helper)
    return helper


def run_case(helper, exe, mode, root):
    job = helper.Job()
    host = None
    start = time.monotonic()
    gate = root / "gate"
    ready = root / "ready"
    try:
        env = os.environ.copy()
        env["OPTIC317_RUST_MODE"] = mode
        env["OPTIC317_RUST_GATE"] = str(gate)
        env["OPTIC317_RUST_READY"] = str(ready)
        with (root / "host.log").open("wb") as logfile:
            host = subprocess.Popen(
                [str(exe), "--exact",
                 "job::atomic_job_list_probe::native_atomic_job_list_host_fixture",
                 "--ignored", "--nocapture"],
                stdout=logfile, stderr=logfile, env=env,
            )
            job.assign(host)
            gate.write_text("independent Job armed", encoding="ascii")
            helper.wait_for(
                lambda: ready.exists() or host.poll() is not None,
                9, "Rust host readiness",
            )
            if host.poll() is not None:
                raise RuntimeError("Rust host failed before ready")
            state = ready.read_text(encoding="ascii")
            expected_state = (
                "mode=atomic;inner_active=1;suspended=1"
                if mode == "atomic" else "mode=invalid;inner_active=0;no_spawn=1"
            )
            if state != expected_state:
                raise AssertionError("Rust native ready data did not match")
            before = 2 if mode == "atomic" else 1
            helper.wait_for(lambda: job.active() == before, 3, "outer owned count")
            # Terminate ONLY the exact Rust host handle created by this test.
            host.kill()
            host.wait(timeout=3)
            helper.wait_for(lambda: job.active() == 0, 3, "host crash recovery")
            return {
                "mode": mode, "outer_before": before, "outer_after": job.active(),
                "rust_native_attribute_proven": mode == "atomic",
                "invalid_job_failed_closed": mode == "invalid",
                "elapsed_ms": round((time.monotonic() - start) * 1000),
            }
    finally:
        # Recover even on assertion/panic/timeout.
        try:
            job.close()
        finally:
            if host is not None:
                if host.poll() is None:
                    host.kill()
                host.wait(timeout=3)
                host._handle.Close()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--test-exe", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    if os.name != "nt":
        raise RuntimeError("Windows hosted native evidence only")
    result = {
        "source_sha": os.environ.get("GITHUB_SHA"),
        "run_id": os.environ.get("GITHUB_RUN_ID"),
        "attempt": os.environ.get("GITHUB_RUN_ATTEMPT"),
        "research_only": True,
        "rust_platform_crate_test_only": True,
        "production_wired": False,
        "user_pc_tested": False,
        "complete": False,
        "cases": [],
    }
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    exe = Path(args.test_exe).resolve()
    helper = load_supervisor()
    try:
        with tempfile.TemporaryDirectory(prefix="optic317-rust-atomic-") as temp:
            for mode in ("atomic", "invalid"):
                root = Path(temp) / mode
                root.mkdir()
                result["cases"].append(run_case(helper, exe, mode, root))
        result["external_jobs_cleaned"] = True
        result["complete"] = True
    except BaseException as exc:
        result["failure_class"] = type(exc).__name__
        raise
    finally:
        output.write_text(json.dumps(result, indent=2), encoding="utf-8")
        print("OPTIC317_RUST_ATOMIC " + json.dumps(result, separators=(",", ":")))


if __name__ == "__main__":
    main()
