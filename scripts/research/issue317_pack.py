"""Package only bounded synthetic sources and completed-job result snapshots."""
import json
import os
from pathlib import Path
import urllib.request
import zipfile

repo = os.environ["GITHUB_REPOSITORY"]
sha = os.environ["GITHUB_SHA"]
normal = {"state": "unavailable"}
try:
    request = urllib.request.Request(
        "https://api.github.com/repos/" + repo + "/actions/runs?head_sha=" + sha + "&per_page=30",
        headers={"Authorization": "Bearer " + os.environ["GH_TOKEN"], "Accept": "application/vnd.github+json"})
    with urllib.request.urlopen(request, timeout=15) as response:
        runs = json.load(response)["workflow_runs"]
    ci = [r for r in runs if r["path"] == ".github/workflows/ci.yml"]
    normal = [{"id": r["id"], "sha": r["head_sha"], "status": r["status"],
               "conclusion": r["conclusion"], "attempt": r["run_attempt"]} for r in ci]
except Exception as error:
    normal = {"state": "unavailable", "error_type": type(error).__name__}
results = [json.loads(p.read_text()) for p in Path("evidence").rglob("owner-*.json")]
complete = (len(results) == 2 and all(r["complete"] and len(r["cases"]) == (17 if r["platform"] == "linux" else 16) and r["sha"] == sha for r in results))
provenance = {"sha": sha, "run": os.environ["GITHUB_RUN_ID"], "attempt": os.environ["GITHUB_RUN_ATTEMPT"],
              "primitive_experiments_complete": complete, "production_fix_validated": False,
              "independent_ci_at_archive_creation": normal, "results": results}
Path("package").mkdir(exist_ok=True)
Path("package/PROVENANCE.json").write_text(json.dumps(provenance, indent=2), encoding="utf-8")
report = ["# Issue 317 — restitution", "", "Production inchangée. Prototype de propriété uniquement.",
          "SHA : " + sha, "Expériences natives complètes : " + str(complete),
          "CI indépendante à la création : " + json.dumps(normal), "",
          "| OS | Cas | Injection | Raison | Durée ms | Récupération |",
          "|---|---|---|---|---:|---|"]
for r in results:
    for c in r["cases"]:
        report.append("| " + " | ".join(str(x) for x in [r["platform"], c["case"], c["fault"], c["reason"], c["elapsed_ms"], c["cleanup"]]) + " |")
report += ["", "Les injections ne prouvent pas une panne native spontanée. Les durées du prototype Python ne sont pas celles du runner Rust.",
           "Avis : correction expérimentale incomplète ; contrat de session/réservations et confinement Unix restent à prouver.",
           "La CI pending/unavailable n'est pas validée. Consulter Actions sur ce SHA après création.",
           "Le rouge historique 39613681993 (assemblage) est conservé. Aucune fusion ni nouvelle PR."]
Path("package/VALIDATION.md").write_text("\n".join(report) + "\n", encoding="utf-8")
with zipfile.ZipFile("package/optic-bridge-issue317-owned-cleanup-2026-10-21.zip", "w", zipfile.ZIP_DEFLATED) as archive:
    for source, name in [
        ("docs/research/ISSUE317_OWNED_CLEANUP_2026-10-21.md", "REPORT_AND_HANDOFF.md"),
        ("scripts/research/issue317_owner_study.py", "scripts/issue317_owner_study.py"),
        ("scripts/research/issue317_supervisor.py", "scripts/issue317_supervisor.py"),
        ("crates/optic-bridge-runtime/examples/issue317_pipe_fixture.rs", "scripts/issue317_pipe_fixture.rs"),
        ("package/PROVENANCE.json", "PROVENANCE.json"),
        ("package/VALIDATION.md", "VALIDATION.md")]:
        archive.write(source, name)
print(json.dumps({"complete": complete, "normal_ci": normal, "sha": sha}))
