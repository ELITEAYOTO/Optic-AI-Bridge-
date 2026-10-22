"""Independently own and supervise the real Git owner registry adoption test."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--test-exe", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    assert os.name == "nt", "native Windows required"
    source = Path(__file__).parents[1] / "tests" / "doctor_pipe_supervisor.py"
    spec = importlib.util.spec_from_file_location("outer_job", source)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    evidence = {"complete": False, "outer_job_empty_confirmed": False,
                "source_sha": os.environ.get("OPTIC_SOURCE_SHA", os.environ.get("GITHUB_SHA")),
                "production_git_wired": False}
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    started = time.monotonic()
    job = module.Job()
    process = None
    temporary = tempfile.TemporaryDirectory(prefix="optic317-registry-")
    try:
        root = Path(temporary.name)
        gate = root / "go"
        env = os.environ.copy()
        env["OPTIC317_REGISTRY_GATE"] = str(gate)
        with (root / "log").open("wb") as log:
            process = subprocess.Popen([
                args.test_exe, "--exact",
                "git_owner_registry::tests::native_rejected_adoption_retains_owner_until_tree_and_pipes_end",
                "--ignored", "--nocapture"], env=env, stdout=log, stderr=log)
            job.assign(process)
            gate.write_text("owned", encoding="ascii")
            process.wait(timeout=12)
        log = (root / "log").read_text(errors="replace")
        evidence["exit_code"] = process.returncode
        evidence["proof_markers"] = [line for line in log.splitlines()
                                     if line.startswith("OPTIC317_REGISTRY_NATIVE ")]
        assert process.returncode == 0, "native test failed (raw assertion log withheld)"
        assert len(evidence["proof_markers"]) == 1
        evidence["complete"] = True
    finally:
        try:
            job.close()
            evidence["outer_job_empty_confirmed"] = True
        finally:
            if process:
                process.wait(timeout=2)
                process._handle.Close()
            temporary.cleanup()
            evidence["elapsed_ms"] = round((time.monotonic() - started) * 1000)
            output.write_text(json.dumps(evidence, indent=2), encoding="utf-8")
            print(json.dumps(evidence))


if __name__ == "__main__":
    main()
