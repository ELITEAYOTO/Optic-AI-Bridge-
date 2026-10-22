"""Independently own and supervise the real Git owner registry adoption test."""
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
    parser.add_argument("--test-exe", required=True)
    parser.add_argument("--output", required=True)
    selection = parser.add_mutually_exclusive_group()
    selection.add_argument("--prepared", action="store_true")
    selection.add_argument("--matrix", action="store_true")
    selection.add_argument("--native-errors", action="store_true")
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
        env["OPTIC317_PYTHON"] = sys.executable
        env["OPTIC317_FIXTURE"] = str(Path(__file__).with_name("issue317_baseline_fixture.py").resolve())
        with (root / "log").open("wb") as log:
            process = subprocess.Popen([
                args.test_exe, "--exact",
                ("job::prepared_read_command::tests::native_partial_failure_matrix" if args.native_errors else
                 "git_owner_registry::native_tests::prepared_owner_native_matrix" if args.matrix else
                 "git_owner_registry::native_tests::prepared_owner_stops_while_native_observer_is_blocked"
                 if args.prepared else "git_owner_registry::tests::native_rejected_adoption_retains_owner_until_tree_and_pipes_end"),
                "--ignored", "--nocapture"], env=env, stdout=log, stderr=log)
            job.assign(process)
            gate.write_text("owned", encoding="ascii")
            if args.prepared:
                module.wait_for(lambda: (root / "ready").exists() or process.poll() is not None, 4, "native ready")
                assert process.poll() is None
                evidence["active_before_stop"] = job.active()
                assert evidence["active_before_stop"] == 2
                (root / "outer-ready").write_text("two native processes observed", encoding="ascii")
                module.wait_for(lambda: (root / "revoked").exists(), 2, "real revoke flag")
                before = time.monotonic()
                module.wait_for(lambda: job.active() == 1, 2, "watchdog tree stop while observer locked")
                evidence["post_revoke_marker_wait_ms"] = round((time.monotonic() - before) * 1000)
                evidence["active_while_observer_held"] = job.active()
                (root / "outer-confirmed").write_text("native outer Job accounting", encoding="ascii")
            process.wait(timeout=12)
        log = (root / "log").read_text(errors="replace")
        evidence["exit_code"] = process.returncode
        evidence["proof_markers"] = [line for line in log.splitlines()
                                     if line.startswith("OPTIC317_REGISTRY_NATIVE ")]
        evidence["cases"] = [json.loads(line.removeprefix("OPTIC317_PREPARED_CASE ")) for line in log.splitlines()
                             if line.startswith("OPTIC317_PREPARED_CASE ")]
        if process.returncode:
            # These selected synthetic tests emit assertions, not user data.
            # Preserve a bounded diagnostic instead of deleting the only
            # first-failure detail with the temporary directory.
            diagnostic = log[-6000:]
            for private, replacement in [(str(root), "<fixture>"),
                                         (args.test_exe, "<test-exe>"),
                                         (sys.executable, "<python>"),
                                         (str(Path.home()), "<home>")]:
                diagnostic = diagnostic.replace(private, replacement)
            evidence["failure_diagnostic"] = diagnostic
        assert process.returncode == 0, "native test failed (raw assertion log withheld)"
        assert len(evidence["proof_markers"]) == 1
        if args.matrix:
            assert len(evidence["cases"]) == 10
        if args.native_errors:
            assert len(evidence["cases"]) == 3
        evidence["complete"] = True
    except BaseException as error:
        evidence["supervisor_error_type"] = type(error).__name__
        evidence["active_before_cleanup"] = job.active()
        if (root / "log").exists() and "failure_diagnostic" not in evidence:
            diagnostic = (root / "log").read_text(errors="replace")[-6000:]
            for private, replacement in [(str(root), "<fixture>"),
                                         (args.test_exe, "<test-exe>"),
                                         (sys.executable, "<python>"),
                                         (str(Path.home()), "<home>")]:
                diagnostic = diagnostic.replace(private, replacement)
            evidence["failure_diagnostic"] = diagnostic
        raise
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
