# Reprise nouveau chat — Optic AI Bridge apres PR #503, 4 novembre 2026

Reprends le travail **concret** sur Optic AI Bridge, depot https://github.com/ELITEAYOTO/Optic-AI-Bridge- et machine Windows 10 RDC DESKTOP-IOSTF61 (device 28a27731-11a0-4f26-bce2-0d238709b315).

Lis d'abord docs/engineering/HANDOFF_WINDOWS_V1_AFTER_PR503_2026-11-04.md et le ZIP de passation de ce chat, tout particulierement 00_LIRE_EN_PREMIER.md, 01_PROMPT_DE_REPRISE.md, 02_ETAT_GITHUB_ET_PREUVES.md, 03_ARCHITECTURE_P0.md et 04_TESTS_METHODE_ET_CI.md.

**Checkpoint historique a re-verifier EN DIRECT** : main=150cd5e4f7f4220bcef7406b1dc4466bde2acee5 ; tree=9a294586129ec1b2ac0ffa605e3e252c1430b8fe ; PR #503 MERGED, CI PR #41820133578 + 3 workflows natifs tous verts au premier essai, CI push/main #41820929161 4/4 SUCCESS attempt1. Les PR #501 et #502 ont aussi des RED/GREEN kernel sur vrais threads Win32 et des CI post-main vertes. Une PR documentaire apres ce checkpoint pourra avoir change main ; verifier sa CI avant de travailler.

**Aucune V1 publiable : #323 OPEN, Windows V1 NO-GO.** PR #499 toujours OPEN/bloquee apres premiere CI Windows rouge #185, interdiction de rerun pour obtenir du vert. #185 Node et #319 initialize restent ouverts. Anciennes recherches #327/#353/#355/#358 non fusionnees. GitRead MCP reste legacy HardenedCommandRunner.

**Priorite P0** : securiser une autorisation service-owned, generation-bound et revocation-linearizable pour session originale, TaskLease, policy/epoch, host, pin Git/repo, Job et horizon, en face des vrais appels CreateProcessW et ResumeThread. Ne pas prendre les observations disjointes pour un issuer. Ne jamais garder un verrou de registre pendant un syscall bloquant. Comptabiliser/gerer tout effet Win32 en vol qui revient apres revoke, sans laisser son root reprendre, avec gardien durable et preuves root exit + Job-empty + deux EOF physiques + no-future-launch + impossibilite de faux succès sous crash/unknown. Conserver le RED de recherche #355 comme critere d'acceptation.

**Methode** : nouveau LAB Windows isole sous E:\Optic-IA-Bridge, jamais toucher Bridge quotidien/Desktop/REL/plugin. Prouver test OS RED reel (verifier running 1 test), correction minimale GREEN, repetitions, crate Windows, workspace standard et all-features first complete attempt, fmt, Clippy strict -D warnings, diff. Une PR a la fois ; exact SHA ; CI hebergee Windows/Ubuntu/native/deps/release attempt1 ; revue COMMENT, pas APPROVE artificiel ; squash garde par --match-head-commit ; arbres identiques ; CI independante push/main 4/4 attempt1 ; commenter #323 et conserver journaux SHA256. Si une CI est rouge au premier essai, documenter et STOP (ne pas rerun-to-green).

Vision future, NON PRIORITAIRE : profils et permissions fines, Git complet, compiler, creer/restaurer/annuler projets, tests isoles, MCP Library (Figma/Blender), IA locale isolee. Windows V1 securisee d'abord.

Travaille en autonomie dans cette methode, donne des retours sur les vraies preuves et les limites, et renouvelle la documentation/reprise avant une future limite de chat.
