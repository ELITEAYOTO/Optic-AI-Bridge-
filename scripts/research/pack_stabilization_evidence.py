"""Create the sanitized campaign archive on a disposable GitHub-hosted runner."""
import json
import os
from pathlib import Path
import zipfile

root = Path.cwd()
collected = root / "target" / "collected"
files = sorted(collected.rglob("*.json")) if collected.exists() else []
manifest = {
    "source_sha": os.environ["GITHUB_SHA"],
    "workflow_run": os.environ["GITHUB_RUN_ID"],
    "attempt": os.environ["GITHUB_RUN_ATTEMPT"],
    "research_job_result": os.environ.get("RESEARCH_JOB_RESULT", "unknown"),
    "main_at_start": "80fe108b288d2b0b6676d6128d9e9beb922eda3f",
    "pr318_head": "49fe3efd34c9ffe9c10c1c5945b37da2c6a74028",
    "results": [json.loads(p.read_text(encoding="utf-8")) for p in files],
}
if len(files) != 2 or not all(x.get("complete") for x in manifest["results"]):
    manifest["evidence_complete"] = False
else:
    manifest["evidence_complete"] = True
sources = {
    "REPORT.md": "docs/research/CODEX_STABILIZATION_2026-10-21.md",
    "scripts/issue317_supervisor.py": "scripts/research/issue317_supervisor.py",
    "scripts/issue317_pipe_fixture.rs": "crates/optic-bridge-runtime/examples/issue317_pipe_fixture.rs",
    "scripts/Test-DoctorTimeoutDiagnostics.ps1": "scripts/tests/Test-DoctorTimeoutDiagnostics.ps1",
}
out = root / "target" / "optic-bridge-codex-stabilization-2026-10-21.zip"
out.parent.mkdir(exist_ok=True)
validation = "# Actual hosted evidence\n\n"
validation += "Source: " + manifest["source_sha"] + "\n\n"
validation += "Workflow: https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/" + manifest["workflow_run"] + "\n\n"
validation += "Attempt: " + manifest["attempt"] + "; research matrix: " + manifest["research_job_result"] + ".\n\n"
validation += "Complete two-platform result collection: " + str(manifest["evidence_complete"]) + ".\n\n"
validation += "Expected deadline violations characterize the unchanged runner; they are not passing production deadlines. No natural #319 or #185 reproduction is implied.\n\n"
for result in manifest["results"]:
    validation += "## " + result["platform"] + "\n\n"
    for case in result["cases"]:
        validation += "- " + case["case"] + ": runner " + str(case["runner"]["runner_ms"]) + " ms; configured 200 ms; blocked after direct exit = " + str(case["blocked_after_parent_exit"]) + "; cleanup " + case["cleanup"] + ".\n"
with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as archive:
    for label, source in sources.items():
        archive.writestr(label, (root / source).read_text(encoding="utf-8"))
    archive.writestr("PROVENANCE.json", json.dumps(manifest, indent=2))
    archive.writestr("VALIDATION.md", validation)
print("OPTIC_RESEARCH_ARCHIVE entries=6 complete=" + str(manifest["evidence_complete"]))
if not manifest["evidence_complete"]:
    raise SystemExit("Incomplete research evidence; archive retained without success claim")
