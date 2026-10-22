"""Measure the unchanged production runner under an independent Windows Job."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--example-exe", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    assert os.name == "nt"
    source = Path(__file__).parents[1] / "tests" / "doctor_pipe_supervisor.py"
    spec = importlib.util.spec_from_file_location("outer_job", source)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    fixture = Path(__file__).with_name("issue317_baseline_fixture.py").resolve()
    result = {"complete": False, "runner_deadline_ms": 200, "cases": [],
              "source_sha": os.environ.get("OPTIC_SOURCE_SHA", os.environ.get("GITHUB_SHA"))}
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    try:
        for mode in ["normal", "nonzero", "output", "stdout", "stderr", "stdin", "timeout"]:
            job, process = module.Job(), None
            case = {"mode": mode, "outer_job_empty_confirmed": False}
            started = time.monotonic()
            temporary = tempfile.TemporaryDirectory(prefix="optic317-baseline-")
            try:
                root = Path(temporary.name)
                env = os.environ.copy()
                env["OPTIC317_BASELINE_GATE"] = str(root / "go")
                with (root / "log").open("wb") as log:
                    process = subprocess.Popen([args.example_exe, sys.executable, str(fixture), mode, str(root)],
                                               env=env, stdout=log, stderr=log)
                    job.assign(process)
                    (root / "go").write_text("owned", encoding="ascii")
                    try:
                        process.wait(timeout=2)
                        blocked = False
                    except subprocess.TimeoutExpired:
                        blocked = True
                case["blocked_after_2s"] = blocked
                case["observed_ms"] = round((time.monotonic() - started) * 1000)
                case["active_processes_before_cleanup"] = job.active()
                log = (root / "log").read_text(errors="replace")
                case["result"] = [line for line in log.splitlines() if line.startswith("OPTIC317_BASELINE ")]
                assert blocked == (mode in ["stdout", "stderr", "stdin", "timeout"]), mode
                if not blocked:
                    assert process.returncode == 0 and len(case["result"]) == 1
                    expected = {"normal": "code=0", "nonzero": "code=7", "output": "output exceeded"}[mode]
                    assert expected in case["result"][0]
            finally:
                try:
                    job.close()
                    case["outer_job_empty_confirmed"] = True
                finally:
                    if process:
                        process.wait(timeout=2)
                        process._handle.Close()
                    temporary.cleanup()
                    result["cases"].append(case)
                    output.write_text(json.dumps(result, indent=2), encoding="utf-8")
        result["complete"] = True
    finally:
        output.write_text(json.dumps(result, indent=2), encoding="utf-8")
        print(json.dumps(result))


if __name__ == "__main__":
    main()
