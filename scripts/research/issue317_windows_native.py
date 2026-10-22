"""Hosted Windows, independent outer Job for a native Rust owner slice."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
import zipfile

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--test-exe", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    assert os.name == "nt"
    helper = Path(__file__).parents[1] / "tests" / "doctor_pipe_supervisor.py"
    spec = importlib.util.spec_from_file_location("optic_outer_job", helper)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    result = dict(
        sha=os.environ["GITHUB_SHA"], run=os.environ["GITHUB_RUN_ID"],
        attempt=os.environ["GITHUB_RUN_ATTEMPT"], complete=False, cases=[],
        production_runner_wired=False, stdin_writer_proven=False,
        session_lifecycle_proven=False, user_pc_execution=False,
    )
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    job = module.Job()
    process = None
    started = time.monotonic()
    try:
        with tempfile.TemporaryDirectory(prefix="optic317-native-") as temporary:
            root = Path(temporary)
            gate = root / "go"
            env = os.environ.copy()
            env["OPTIC317_OUTER_GATE"] = str(gate)
            with (root / "worker.log").open("wb") as log:
                process = subprocess.Popen(
                    [str(Path(args.test_exe).resolve()), "--exact",
                     "job::command_owner_probe::native_windows_owner_matrix", "--nocapture"],
                    env=env, stdout=log, stderr=log,
                )
                job.assign(process)
                gate.write_text("synthetic", encoding="ascii")
                process.wait(timeout=40)
            text = (root / "worker.log").read_text(errors="replace")
            # Fixed, numeric/boolean-only result lines. Never archive raw logs,
            # environment, PIDs, machine paths or arbitrary assertion text.
            for line in text.splitlines():
                if line.startswith("OPTIC317_CASE "):
                    result["cases"].append(json.loads(line.removeprefix("OPTIC317_CASE ")))
                elif line.startswith("OPTIC317_NATIVE_COMPLETE "):
                    result["native"] = json.loads(line.removeprefix("OPTIC317_NATIVE_COMPLETE "))
            assert process.returncode == 0, "native fixture failed; raw log withheld"
            assert len(result["cases"]) == 8
            assert result["native"]["cases"] == 8
            result["complete"] = True
    finally:
        try:
            job.close()
            result["outer_job_empty_confirmed"] = True
        finally:
            if process:
                if process.poll() is None:
                    process.kill()
                process.wait(timeout=2)
                process._handle.Close()
            result["elapsed_ms"] = round((time.monotonic() - started) * 1000)
            output.write_text(json.dumps(result, indent=2), encoding="utf-8")
            print("OPTIC317_RUST_OWNER_PROOF " + json.dumps(result, separators=(",", ":")))
            archive = output.parent / "optic-windows-317-native-slice-2026-10-22.zip"
            with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as z:
                z.write(output, "VALIDATION.json")
                for path in [
                    "docs/decisions/ADR-0035-WINDOWS-GIT-OWNER-STAGING.md",
                    "docs/research/WINDOWS_V1_OWNER_AUDIT_2026-10-22.md",
                    "crates/optic-bridge-windows/src/command_owner_probe.rs",
                    "scripts/research/issue317_windows_native.py",
                ]:
                    z.write(path, path)

if __name__ == "__main__":
    main()
