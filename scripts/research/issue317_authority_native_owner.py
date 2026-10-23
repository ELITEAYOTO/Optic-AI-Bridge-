"""Research only: pre-own every fixed Rust host and descendant in an outer Job."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

MODES = ["terminal-code-error", "pending-root-retention", "close-before-create", "close-before-resume", "launch-wins", "close-during-native",
         "create-error", "resume-error", "stop-error", "query-error", "observation-uncertain", "client-drop",
         "descendant-pipes", "output-limit", "panic-under-gate", "handle-recovery",
         "ticket-close-wins", "ticket-cancelled-before-create", "ticket-launch-wins",
         "ticket-direct-close", "ticket-deadline", "ticket-client-drop",
         "ticket-host-exit-stop-error", "ticket-host-exit-observation"]

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
                if mode.startswith('ticket-host-exit-') and (root / "pending-ready").exists():
                    proof["host_exit_with_pending_owner"] = True
                    break
                if trace.stat().st_size > 128 * 1024:
                    raise RuntimeError("synthetic trace limit")
                if host.poll() is not None:
                    raise RuntimeError("native host failed before proof")
                if time.monotonic() >= deadline:
                    raise TimeoutError("native owner external deadline")
                for phase, expected in [("uncertain", 1), ("retained", 2), ("ticket", 1)]:
                    ready, confirmed = root / (phase + "-ready"), root / (phase + "-confirmed")
                    if ready.exists() and not confirmed.exists():
                        outer_api.wait_for(lambda: job.active() == expected, 2, phase + " witness")
                        proof["outer_active_during_" + phase] = job.active()
                        confirmed.write_text("external-accounting-confirmed", encoding="ascii")
                time.sleep(.002)
            pending_exit = proof.get("host_exit_with_pending_owner", False)
            state = json.loads((root / ("pending-ready" if pending_exit else "ready")).read_text(encoding="utf-8"))
            assert state["mode"] == mode and state["production_wired"] is False
            assert state["atomic_job_association"] == state["native_created"]
            assert state["native_created"] == (mode not in ("close-before-create", "create-error",
                "ticket-close-wins", "ticket-cancelled-before-create", "ticket-client-drop"))
            for key in ["instrumented_registry", "real_win32"]:
                assert state[key] is True, key
            if pending_exit:
                assert state["pending_keeper_present"] and state["legacy_revoke_pending"]
                assert state["active_admissions"] == 1 and state["active_jobs"] == 1
                for key in ["native_terminal_and_both_eof", "stop_worker_joined", "shared_accounting_released_after_proof"]:
                    assert state[key] is False
            else:
                for key in ["native_terminal_and_both_eof", "stop_worker_joined",
                            "shared_accounting_released_after_proof", "other_session_active"]:
                    assert state[key] is True, key
                assert state["combined_output_bytes"] <= 16 * 1024
            if mode == "handle-recovery":
                assert state["handle_count_stable_after_warmup"] is True
            if mode == "terminal-code-error":
                assert state["terminal_observation_error_retries"] == 3
                assert state["terminal_no_partial_publication"] is True
                assert state["status"] == "Failed" and state["native_error_code"] == 6
            if mode == "pending-root-retention":
                assert state["bounded_root_capacity"] == 1
                assert state["repeated_observation_errors"] == 4
                for key in ["preparation_rejected_before_effects", "same_keeper_retained_during_shutdown", "shutdown_complete_only_after_proof"]:
                    assert state[key] is True, key
            if mode in ("close-before-create", "close-before-resume", "create-error", "resume-error",
                        "ticket-close-wins", "ticket-cancelled-before-create", "ticket-direct-close", "ticket-deadline", "ticket-client-drop"):
                assert not (root / "executed").exists()
            expected = 2 if mode == "ticket-host-exit-stop-error" else 1
            outer_api.wait_for(lambda: job.active() == expected, 2, "owned accounting before host exit")
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
