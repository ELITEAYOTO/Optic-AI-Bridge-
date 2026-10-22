"""Small public proof archive; never waits for or invents independent CI success."""
import hashlib
import json
import os
from pathlib import Path
import urllib.request
import zipfile

sha=os.environ.get("OPTIC_SOURCE_SHA") or os.environ["GITHUB_SHA"]
repo=os.environ["GITHUB_REPOSITORY"]
result_files=list(Path("evidence").rglob("doctor-pipes.json"))
results=[json.loads(p.read_text()) for p in result_files]
complete=len(results)==1 and results[0]["complete"] and len(results[0]["cases"])==12 and results[0]["sha"]==sha
ci={"state":"unavailable"}
try:
    request=urllib.request.Request(
        "https://api.github.com/repos/"+repo+"/actions/runs?head_sha="+sha+"&per_page=30",
        headers={"Authorization":"Bearer "+os.environ["GH_TOKEN"],"Accept":"application/vnd.github+json"})
    with urllib.request.urlopen(request,timeout=15) as response:
        runs=json.load(response)["workflow_runs"]
    ci=[{"id":r["id"],"sha":r["head_sha"],"status":r["status"],"conclusion":r["conclusion"],
         "attempt":r["run_attempt"],"event":r["event"]} for r in runs if r["path"]==".github/workflows/ci.yml"]
except Exception as error:
    ci={"state":"unavailable","error_type":type(error).__name__}
sources=[
    "scripts/Test-OpticAIBridge.ps1","scripts/tests/DoctorPipeWorker.ps1",
    "scripts/tests/doctor_pipe_supervisor.py","scripts/tests/doctor_pipe_fixture.py",
    "docs/research/WINDOWS_DOCTOR_PIPE_LIFETIME_2026-10-22.md"]
provenance=dict(sha=sha,checkout_sha=os.environ["GITHUB_SHA"],run=os.environ["GITHUB_RUN_ID"],attempt=os.environ["GITHUB_RUN_ATTEMPT"],
                native_doctor_proof_complete=complete,independent_ci_at_archive_creation=ci,
                natural_issue319_cause_proven=False,results=results,
                sources_sha256={p:hashlib.sha256(Path(p).read_bytes()).hexdigest() for p in sources})
out=Path("package");out.mkdir(exist_ok=True)
(out/"PROVENANCE.json").write_text(json.dumps(provenance,indent=2),encoding="utf-8")
lines=["# Validation Windows V1 — 22 octobre 2026","",
       "SHA : "+sha,"Preuve native complète : "+str(complete),
       "État de la CI indépendante à la création : "+json.dumps(ci),"",
       "La preuve concerne les chemins d'erreur du doctor, pas la cause naturelle du timeout MCP319.",
       "Une CI pending/queued/in_progress/unavailable n'est pas une validation réussie.",
       "Les premiers contrôles rouges et les premières observations incomplètes sont conservés dans REPORT_AND_HANDOFF.md.","",
       "| Mode | Source | Durée ms | Nettoyage |","|---|---|---:|---|"]
for result in results:
    for c in result["cases"]:
        lines.append("| "+" | ".join(str(x) for x in [c["mode"],c["source"],c.get("elapsed_ms",c.get("supervisor_ms")),c["cleanup"]])+" |")
(out/"VALIDATION.md").write_text("\n".join(lines)+"\n",encoding="utf-8")
handoff="""# Reprise — Windows V1
Lire REPORT_AND_HANDOFF.md, PROVENANCE.json et VALIDATION.md.
Recontrôler main, PR318, son SHA exact, ses reviews, CI/native et éventuelle CI post-squash.
Ne jamais confondre la CI observée pending à la création de l'archive avec son résultat final.
La cause naturelle319 reste ouverte ;185 est distinct et317 reste démontré sans correction Rust de production.
Priorités : trace naturelle319 avec phases bornées ; lancement AppContainer185 ; propriété/annulation/réservations Windows317.
La branche bootstrap/issue-319-doctor-bounded-pipes contient séparément une instrumentation Rust ciblée.
Ne pas fusionner les branches expérimentales en bloc. Pas d'expérience de confinement Linux.
Conserver Ubuntu comme contrôle existant ; aucun test/commande projet sur le PC utilisateur.
"""
(out/"REPRISE.md").write_text(handoff,encoding="utf-8")
with zipfile.ZipFile(out/"optic-bridge-windows-v1-stabilization-2026-10-22.zip","w",zipfile.ZIP_DEFLATED) as archive:
    for p in sources:
        archive.write(p,"REPORT_AND_HANDOFF.md" if p.startswith("docs/") else p)
    for name in ["PROVENANCE.json","VALIDATION.md","REPRISE.md"]:
        archive.write(out/name,name)
print(json.dumps({"sha":sha,"native_complete":complete,"independent_ci":ci}))
