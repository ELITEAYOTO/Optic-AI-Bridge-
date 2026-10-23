"""#317 research: independently supervise real registry + native resume race.

Runs only on hosted Windows runners with fixed synthetic Rust test process.
An outer KILL_ON_JOB_CLOSE Job owns each test host *before* its gate opens.
"""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time


def job_helper():
    helper_path = Path(__file__).parents[1] / "tests" / "doctor_pipe_supervisor.py"
    spec = importlib.util.spec_from_file_location("optic317_job", helper_path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


EXPECTED = {
    "direct-revoke-race": "mode=direct-revoke-race;real-close-before-resume=1;marker=1;job-empty=1",
    "close-wins": "mode=close-wins;real-close=1;not-resumed=1;marker-absent=1;job-empty=1",
    "launch-wins": "mode=launch-wins;native-resume-before-real-close=1;marker=1;job-empty=1",
}


def run_mode(helper, executable, mode, directory):
    outer = helper.Job()
    host = None
    trace = directory / "trace.log"
    start = time.monotonic()
    env = os.environ.copy()
    env["OPTIC317_REGISTRY_MODE"] = mode
    env["OPTIC317_REGISTRY_GATE"] = str(directory / "gate")
    env["OPTIC317_REGISTRY_READY"] = str(directory / "ready")
    env["OPTIC317_REGISTRY_MARKER"] = str(directory / "marker")
    try:
        with trace.open("wb") as output:
            host = subprocess.Popen(
                [str(executable), "--exact", "registry_native_resume_race_host",
                 "--ignored", "--nocapture", "--test-threads=1"],
                stdout=output, stderr=output, env=env
            )
            outer.assign(host)
            (directory / "gate").write_text("outer-owned", encoding="ascii")
            helper.wait_for(
                lambda: (directory / "ready").exists() or host.poll() is not None,
                9, "real registry native fixture completion",
            )
            if host.poll() is not None:
                raise AssertionError("synthetic Rust host exited before proof")
            actual = (directory / "ready").read_text(encoding="ascii")
            assert actual == EXPECTED[mode], (mode, actual)
            assert (directory / "marker").exists() == (mode != "close-wins")
            helper.wait_for(lambda: outer.active() == 1, 2, "only owned host remains")
            before = outer.active()
            host.kill()
            host.wait(timeout=3)
            helper.wait_for(lambda: outer.active() == 0, 3, "outer Job drained")
            return {
                "mode": mode,
                "expected": actual,
                "outer_before_host_death": before,
                "outer_after_host_death": outer.active(),
                "marker_present": (directory / "marker").exists(),
                "elapsed_ms": round((time.monotonic() - start) * 1000),
            }
    except BaseException:
        if trace.exists():
            print("SYNTHETIC_RUST_TRACE " + trace.read_text(
                encoding="utf-8", errors="replace"
            )[-4000:])
        raise
    finally:
        try:
            outer.close()
        finally:
            if host is not None:
                if host.poll() is None:
                    host.kill()
                host.wait(timeout=3)
                host._handle.Close()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--test-exe", required=True)
    ap.add_argument("--output", required=True)
    args = ap.parse_args()
    if os.name != "nt":
        raise RuntimeError("hosted Windows-only synthetic native test")
    exe = Path(args.test_exe).resolve()
    assert exe.is_file()
    proof = {
        "source_sha": os.environ.get("GITHUB_SHA"),
        "run_id": os.environ.get("GITHUB_RUN_ID"),
        "attempt": os.environ.get("GITHUB_RUN_ATTEMPT"),
        "production_wired": False,
        "test_only": True,
        "real_session_registry": True,
        "real_native_resume": True,
        "outer_job_cleaned": False,
        "complete": False,
        "cases": [],
    }
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    try:
        with tempfile.TemporaryDirectory(prefix="optic317-real-registry-") as temp:
            for mode in EXPECTED:
                root = Path(temp) / mode
                root.mkdir()
                proof["cases"].append(run_mode(job_helper(), exe, mode, root))
        proof["outer_job_cleaned"] = True
        proof["complete"] = True
    except BaseException as exc:
        proof["failure_class"] = type(exc).__name__
        raise
    finally:
        output.write_text(json.dumps(proof, indent=2), encoding="utf-8")
        print("OPTIC317_REAL_REGISTRY_NATIVE " + json.dumps(proof, separators=(",", ":")))


if __name__ == "__main__":
    main()
