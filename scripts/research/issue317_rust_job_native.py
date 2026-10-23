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
        env["OPTIC317_RETAINER_READY"] = str(root / "descendant")
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
                13 if mode == "retained-stdio" else 9,
                "Rust host readiness",
            )
            if host.poll() is not None:
                raise RuntimeError("Rust host failed before ready")
            state = ready.read_text(encoding="ascii")
            expected_states = {
                "retained-stdio": "mode=retained-stdio;root_exited=1;descendant_held_pipes=1;stop=1;both_eof=1;job_empty=1",
                "stdio-bad-exe": "mode=stdio-bad-exe;both-attributes=1;no-spawn=1;both-eof=1;handles-stable=1",
                "atomic": "mode=atomic;inner_active=1;suspended=1",
                "no-inherit": "mode=no-inherit;inner_active=1;pipe_eof=1;suspended=1",
                "invalid": "mode=invalid;inner_active=0;no_spawn=1",
                "missing": "mode=missing;inner_active=0;no_spawn=1;handles_stable=1",
                "stdio": "mode=stdio;stdout=1;stderr=1;extra_eof=1;bounded=1;job_empty=1",
            }
            expected_state = expected_states[mode]
            if state != expected_state:
                raise AssertionError("Rust native ready data did not match")
            before = 2 if mode in ("atomic", "no-inherit") else 1
            helper.wait_for(lambda: job.active() == before, 3, "outer owned count")
            # Terminate ONLY the exact Rust host handle created by this test.
            host.kill()
            host.wait(timeout=3)
            helper.wait_for(lambda: job.active() == 0, 3, "host crash recovery")
            return {
                "mode": mode, "outer_before": before, "outer_after": job.active(),
                "rust_native_attribute_proven": mode in ("atomic", "no-inherit"),
                "spare_inheritable_writer_not_leaked": mode == "no-inherit",
                "invalid_job_failed_closed": mode == "invalid",
                "missing_executable_handle_count_stable": mode == "missing",
                "selected_stdout_stderr_and_spare_denial": mode == "stdio",
                "retained_descendant_stopped_and_both_pipes_eof": mode == "retained-stdio",
                "dual_attribute_failed_spawn_clean": mode == "stdio-bad-exe",
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
            for mode in ("atomic", "no-inherit", "invalid", "missing", "stdio",
                         "retained-stdio", "stdio-bad-exe"):
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
