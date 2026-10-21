"""Create the sanitized campaign archive on a disposable GitHub-hosted runner."""
import json
import os
from pathlib import Path
import zipfile
import time
import urllib.request

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
# Correlate the independently running normal CI with THIS exact source head.
# Polling is read-only; no CI rerun or security-test timeout change.
base = "https://api.github.com/repos/ELITEAYOTO/Optic-AI-Bridge-"
def api(path):
    request = urllib.request.Request(base + path, headers={"User-Agent": "optic-hosted-evidence"})
    with urllib.request.urlopen(request, timeout=20) as response:
        return json.load(response)
ci = None
deadline = time.monotonic() + 480
while time.monotonic() < deadline:
    runs = api("/actions/runs?head_sha=" + manifest["source_sha"] + "&per_page=30")
    matching = [r for r in runs["workflow_runs"] if r["name"] == "CI" and r["head_sha"] == manifest["source_sha"]]
    if matching:
        found = matching[0]
        ci = {k: found.get(k) for k in ["id", "html_url", "head_sha", "status", "conclusion", "run_attempt"]}
        if found["status"] == "completed":
            jobs = api("/actions/runs/" + str(found["id"]) + "/attempts/" + str(found["run_attempt"]) + "/jobs?per_page=100")
            ci["jobs"] = [{k: j.get(k) for k in ["id", "name", "conclusion"]} for j in jobs["jobs"]]
            break
    time.sleep(20)
manifest["normal_ci"] = ci
live_main = api("/branches/main")
live_prs = api("/pulls?state=open&per_page=100")
manifest["repository_checkpoint"] = {
    "main_sha": live_main["commit"]["sha"],
    "main_protected_flag": live_main["protected"],
    "open_prs": [{"number": p["number"], "head": p["head"]["sha"], "state": p["state"]} for p in live_prs],
    "protection_details": "Initial integration read returned 403; no protection changes",
}

manifest["retained_initial_ci_failures"] = [
    {"source": "5f00209e181629ce9536a075f274648a38af9831", "run": 39608173136, "attempt": 1, "reason": "research formatting and intentional orphan fixture lint"},
    {"source": "2f2f608d6768b073bbc3992d1177598e2f803beb", "run": 39609013696, "attempt": 1, "reason": "test-only explicit drop Clippy error"},
]
manifest["previous_validated_head"] = {"source": "dfc8ba9f8013de97794e80b276a2ebebc098b497", "ci": 39610113717, "research": 39610114084, "attempt": 1}

if len(files) != 2 or not all(x.get("complete") for x in manifest["results"]):
    manifest["evidence_complete"] = False
else:
    manifest["evidence_complete"] = True
sources = {
    "REPORT.md": "docs/research/CODEX_STABILIZATION_2026-10-21.md",
    "scripts/issue317_supervisor.py": "scripts/research/issue317_supervisor.py",
    "scripts/issue317_pipe_fixture.rs": "crates/optic-bridge-runtime/examples/issue317_pipe_fixture.rs",
    "scripts/Test-DoctorTimeoutDiagnostics.ps1": "scripts/tests/Test-DoctorTimeoutDiagnostics.ps1",
    "scripts/inspect_rmcp_handshake.py": "scripts/research/inspect_rmcp_handshake.py",
}
out = root / "target" / "optic-bridge-codex-stabilization-2026-10-21.zip"
out.parent.mkdir(exist_ok=True)
validation = "# Actual hosted evidence\n\n"
validation += "Source: " + manifest["source_sha"] + "\n\n"
validation += "Workflow: https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/" + manifest["workflow_run"] + "\n\n"
validation += "Attempt: " + manifest["attempt"] + "; research matrix: " + manifest["research_job_result"] + ".\n\n"
validation += "Complete two-platform result collection: " + str(manifest["evidence_complete"]) + ".\n\n"
validation += "Independent normal CI on exact source: " + json.dumps(ci) + ".\n\n"
validation += "Expected deadline violations characterize the unchanged runner; they are not passing production deadlines. No natural #319 or #185 reproduction is implied.\n\n"
for result in manifest["results"]:
    validation += "## " + result["platform"] + "\n\n"
    for case in result["cases"]:
        validation += "- " + case["case"] + ": runner " + str(case["runner"]["runner_ms"]) + " ms; configured 200 ms; blocked after direct exit = " + str(case["blocked_after_parent_exit"]) + "; cleanup " + case["cleanup"] + ".\n"
fr = "# Rapport exécutif — campagne hébergée du 21 octobre 2026\n\n"
fr += "Source expérimentale exacte : " + manifest["source_sha"] + ". Aucune fusion ni nouvelle PR. Le seul emplacement de PR reste occupé par #318.\n\n"
fr += "## Faits démontrés\n\n"
fr += "#317 : dépassement du délai du runner après disparition du parent direct, causé dans les fixtures par la durée de vie des pipes hérités. Libération extérieure contrôlée, récupération des processus et contrôles de fin effectués pour chaque cas. Les valeurs réelles figurent ci-dessous ; la réussite du reproducteur ne signifie pas que le runner respecte son délai.\n\n"
for result in manifest["results"]:
    fr += "### " + result["platform"] + "\n\n"
    fr += "| Cas | Délai configuré | Durée du runner | Encore bloqué après sortie parent | Nettoyage |\n|---|---:|---:|---|---|\n"
    for case in result["cases"]:
        fr += "| " + case["case"] + " | " + str(case["configured_deadline_ms"]) + " ms | " + str(case["runner"]["runner_ms"]) + " ms | " + str(case["blocked_after_parent_exit"]) + " | " + case["cleanup"] + " |\n"
    fr += "\n"
fr += "## Limites et classement\n\n"
fr += "- Défaut du produit reproduit : #317, délai de retour non borné par le délai annoncé lorsque les lecteurs/writer attendent des handles conservés. Les tests sont volontairement libérés et supervisés ; ils ne mesurent pas une attente infinie.\n"
fr += "- #319 : diagnostic amélioré et régressions synthétiques, mais aucune cause démontrée du timeout naturel de 30 secondes. Lecture d'identité synchrone avant serveur/transport ; plafond en octets distinct d'une borne temporelle. Voir REPORT.md pour la cartographie complète.\n"
fr += "- #185 : analyse des traces historiques, aucune nouvelle cause ni reproduction. Le budget AppContainer de 10 secondes et la preuve de tentative/refus réseau restent intacts.\n"
fr += "- Défauts du harness : premiers échecs rustfmt/Clippy conservés et corrigés. Le contrôle nul-stdio Windows a exposé une différence réelle, conservée puis comparée à un héritage sélectif. Aucun échec n'a été requalifié en confinement réussi.\n"
fr += "- Aucun correctif de production du runner, aucune optimisation de tampon/cache, aucune suppression de hash/pinning, aucun élargissement d'autorité. Les marqueurs de progression sont expérimentaux, optionnels, fixes et bornés.\n\n"
fr += "## Validation et état\n\n"
fr += "CI normale exacte : " + json.dumps(ci) + ".\n\n"
fr += "État GitHub observé pendant la restitution : " + json.dumps(manifest["repository_checkpoint"]) + ".\n\n"
fr += "Les scripts n'ont été exécutés que sur les runners GitHub hébergés. Aucun accès ni aucune commande sur le PC utilisateur. Aucun exécutable n'est inclus. Les sources et données synthétiques sont récupérables au SHA exact indiqué dans PROVENANCE.json.\n\n"
fr += "Les lectures natives lentes observées historiquement peuvent expliquer des latences de réussite ; elles ne prouvent pas la cause de la tentative rouge #318. Aucun gain de production n'est revendiqué. Les fichiers canoniques de main contiennent des checkpoints historiques ; les références live et les tentatives de CI doivent primer pour les faits.\n"
handoff = "# Reprise GitHub — stabilisation\n\n"
handoff += "Lire RAPPORT_TECHNIQUE.md, REPORT.md, VALIDATION.md et PROVENANCE.json. Dépôt unique : https://github.com/ELITEAYOTO/Optic-AI-Bridge-. Aucun test ni commande sur le PC opérateur.\n\n"
handoff += "Dernier état enregistré : " + json.dumps(manifest["repository_checkpoint"]) + ". Branche de recherche bootstrap/issue-319-process-lease-phases, source validée/mesurée " + manifest["source_sha"] + ". Ne pas fusionner cette branche en bloc. #318 reste un sujet distinct : ne pas substituer sa CI rouge par celle de la recherche.\n\n"
handoff += "1. Préparer une correction isolée de #317 avec propriété de l'arbre établie avant exécution et interruption réelle des pipes ; conserver les workers joignables. Terminé seulement après régressions Windows/Linux de tous les cas présents, erreurs et fermeture prouvée, puis CI sur SHA exact. Aucune seconde PR tant que #318 est active.\n"
handoff += "2. Exploiter les marqueurs fixes #319 lors d'une prochaine défaillance naturelle. Terminé seulement avec une signature non tronquée permettant de situer la phase et un reproducteur causal ; un succès ou un benchmark de cache ne résout pas le timeout historique.\n"
handoff += "3. Reprendre #185 uniquement avec une nouvelle hypothèse falsifiable sur livraison de requête/lanceur/loader/première sortie. Préserver le budget de 10 s et l'attestation de tentative/refus. Avant tout choix sur #318, revérifier son SHA, ses reviews et sa CI propre ; aucune fusion autorisée par cette campagne.\n\n"
handoff += "Les fixtures ont été arrêtées/récupérées dans les jobs terminés ; aucun processus de campagne n'a été lancé sur le PC. Les modifications sont toutes publiées sur la branche expérimentale ; aucun état local de code n'a été créé. Les tentatives rouges et le protocole sont conservés dans REPORT.md et Actions. Préserver #176/#182 et les invariants du dépôt.\n\n"
handoff += "Ne pas exécuter le helper Rust seul : sa sûreté de nettoyage dépend du superviseur. Sur un prochain runner, relire le code et les limites avant reprise. Vérifier le quota réel, consolider à 20 %, arrêter les nouveaux chantiers à 10 % et conserver la réserve finale.\n"

with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as archive:
    for label, source in sources.items():
        archive.writestr(label, (root / source).read_text(encoding="utf-8"))
    archive.writestr("PROVENANCE.json", json.dumps(manifest, indent=2))
    archive.writestr("VALIDATION.md", validation)
    archive.writestr("RAPPORT_TECHNIQUE.md", fr)
    archive.writestr("REPRISE_GITHUB.md", handoff)
print("OPTIC_RESEARCH_ARCHIVE entries=" + str(len(sources) + 4) + " complete=" + str(manifest["evidence_complete"]))
if not manifest["evidence_complete"] or not ci or ci.get("conclusion") != "success":
    raise SystemExit("Incomplete or failed exact-head evidence; archive retained without success claim")
