"""Research only: pre-own every fixed Rust host and descendant in an outer Job."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

MODES = ["close-before-create", "close-before-resume", "launch-wins", "close-during-native",
         "create-error", "resume-error", "stop-error", "query-error", "observation-uncertain", "client-drop",
         "descendant-pipes", "output-limit", "panic-under-gate", "handle-recovery"]

def helper():
    path = Path(__file__).parents[1] / "tests" / "doctor_pipe_supervisor.py"
    spec = importlib.util.spec_from_file_location("owned_outer_job", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module

def run_case(outer_api, executable, mode, root):
    job = outer_api.Job()  # configured BEFORE host starts; gate before child effects
    host = None
    proof = {"mode": mode, "complete": False, "outer_job_cleaned": False}
    started = time.monotonic()
    env = {key: os.environ[key] for key in ("SystemRoot", "WINDIR") if key in os.environ}
    env.update(TEMP=str(root), TMP=str(root), OPTIC317_OWNER_GATE=str(root / "gate"),
               OPTIC317_OWNER_MODE=mode)
    trace = root / "trace"
    try:
        with trace.open("wb") as log:
            host = subprocess.Popen([str(executable), "--exact",
                "process::authority_native_owner_tests::authority_native_owner_host", "--ignored",
                "--nocapture", "--test-threads=1"], cwd=root, env=env, stdout=log, stderr=log)
            job.assign(host)
            (root / "gate").write_text("outer-owned", encoding="ascii")
            deadline = time.monotonic() + 12
            while not (root / "ready").exists():
                if trace.stat().st_size > 128 * 1024:
                    raise RuntimeError("synthetic trace limit")
                if host.poll() is not None:
                    raise RuntimeError("native host failed before proof")
                if time.monotonic() >= deadline:
                    raise TimeoutError("native owner external deadline")
                for phase, expected in [("uncertain", 1), ("retained", 2)]:
                    ready, confirmed = root / (phase + "-ready"), root / (phase + "-confirmed")
                    if ready.exists() and not confirmed.exists():
                        outer_api.wait_for(lambda: job.active() == expected, 2, phase + " witness")
                        proof["outer_active_during_" + phase] = job.active()
                        confirmed.write_text("external-accounting-confirmed", encoding="ascii")
                time.sleep(.002)
            state = json.loads((root / "ready").read_text(encoding="utf-8"))
            assert state["mode"] == mode and state["production_wired"] is False
            assert state["atomic_job_association"] == state["native_created"]
            assert state["native_created"] == (mode not in ("close-before-create", "create-error"))
            for key in ["instrumented_registry", "real_win32",
                        "native_terminal_and_both_eof", "stop_worker_joined",
                        "shared_accounting_released_after_proof", "other_session_active"]:
                assert state[key] is True, key
            assert state["combined_output_bytes"] <= 16 * 1024
            if mode == "handle-recovery":
                assert state["handle_count_stable_after_warmup"] is True
            if mode in ("close-before-create", "close-before-resume", "create-error", "resume-error"):
                assert not (root / "executed").exists()
            outer_api.wait_for(lambda: job.active() == 1, 2, "host only after owned proof")
            proof["outer_before_host_exit"] = job.active()
            host.kill()  # exact synthetic handle; never a guessed PID
            host.wait(timeout=3)
            outer_api.wait_for(lambda: job.active() == 0, 2, "outer tree recovered")
            proof.update(native=state, outer_after_host_exit=job.active(), complete=True)
    except BaseException as error:
        proof["failure_class"] = type(error).__name__
        if trace.exists():
            text = trace.read_text(encoding="utf-8", errors="replace")[-10000:]
            text = text.replace(str(root), "<fixture>").replace(str(Path.home()), "<operator-home>")
            proof["failure_diagnostic"] = text
        raise
    finally:
        try:
            proof["outer_before_cleanup"] = job.active()
            job.close()
            proof["outer_job_cleaned"] = True
        finally:
            if host is not None:
                if host.poll() is None:
                    host.kill()
                host.wait(timeout=3)
                host._handle.Close()
            proof["elapsed_ms"] = round((time.monotonic() - started) * 1000)
            # Emit even on a failing case; caller persists this independently.
            (root.parent / (mode + ".json")).write_text(json.dumps(proof, indent=2), encoding="utf-8")
    return proof

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--test-exe", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--mode", choices=MODES)
    args = parser.parse_args()
    assert os.name == "nt"
    output = Path(args.output).resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    result = {"source_sha": os.environ.get("OPTIC_SOURCE_SHA", os.environ.get("GITHUB_SHA")),
              "run_id": os.environ.get("GITHUB_RUN_ID"), "attempt": os.environ.get("GITHUB_RUN_ATTEMPT"),
              "production_wired": False, "instrumented_registry": True,
              "complete": False, "cases": []}
    modes = [args.mode] if args.mode else MODES
    try:
        with tempfile.TemporaryDirectory(prefix="optic317-authority-owner-", dir=output.parent) as temp:
            base = Path(temp)
            for mode in modes:
                root = base / mode
                root.mkdir()
                try:
                    result["cases"].append(run_case(helper(), Path(args.test_exe).resolve(), mode, root))
                finally:
                    partial = base / (mode + ".json")
                    if partial.exists() and (not result["cases"] or result["cases"][-1]["mode"] != mode):
                        result["cases"].append(json.loads(partial.read_text(encoding="utf-8")))
        result["complete"] = True
    except BaseException as error:
        result["failure_class"] = type(error).__name__
        raise
    finally:
        output.write_text(json.dumps(result, indent=2), encoding="utf-8")
        print("OPTIC317_AUTHORITY_NATIVE_OWNER " + json.dumps(result, separators=(",", ":")))

if __name__ == "__main__":
    main()
