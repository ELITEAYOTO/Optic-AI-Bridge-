import argparse
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--shell", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    spec = importlib.util.spec_from_file_location("job_guard", Path("scripts/tests/doctor_pipe_supervisor.py"))
    guard = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(guard)
    result = dict(sha=os.environ.get("OPTIC_SOURCE_SHA") or os.environ["GITHUB_SHA"], checkout_sha=os.environ["GITHUB_SHA"], run=os.environ["GITHUB_RUN_ID"], attempt=os.environ["GITHUB_RUN_ATTEMPT"], complete=False)
    with tempfile.TemporaryDirectory(prefix="optic-wire-") as temporary:
        root = Path(temporary)
        job = guard.Job()
        p = None
        try:
            with (root/"raw.log").open("wb") as log:
                p = subprocess.Popen([args.shell, "-NoProfile", "-NonInteractive", "-File",
                    "scripts/tests/Probe-McpWire.ps1", "-Python", sys.executable,
                    "-Fixture", str(Path("scripts/tests/mcp_wire_fixture.py").resolve()),
                    "-Root", str(root)], stdout=log, stderr=log)
                job.assign(p)
                guard.wait_for(lambda:(root/"ready").exists() or p.poll() is not None,8,"startup")
                assert p.poll() is None
                (root/"go").write_text("synthetic")
                p.wait(timeout=10)
            assert p.returncode == 0, "wire probe failed; raw log withheld"
            result["cases"] = json.loads((root/"result.json").read_text(encoding="utf-8-sig"))
            assert len(result["cases"]) == 3
            legacy, fixed, failed = result["cases"]
            if legacy["edition"] == "Desktop":
                assert legacy["prefix_hex"] == "efbbbf" and not legacy["valid_json"]
            assert fixed["stdin_preamble_bytes"] == 0 and fixed["valid_json"]
            assert fixed["unicode_roundtrip"] and fixed["console_restored"]
            assert failed["failed_start"] and failed["console_restored"]
        finally:
            try:
                job.close()
                result["cleanup"] = "job-empty-confirmed"
            finally:
                if p:
                    if p.poll() is None: p.kill()
                    p.wait(timeout=2)
                    p._handle.Close()
    result["complete"] = True
    Path(args.output).write_text(json.dumps(result,indent=2),encoding="utf-8")
    print("OPTIC_WIRE_PROOF "+json.dumps(result,separators=(",",":")))

if __name__ == "__main__": main()
