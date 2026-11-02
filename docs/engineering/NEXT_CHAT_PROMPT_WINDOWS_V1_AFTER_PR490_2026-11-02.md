# PROMPT DE REPRISE — Optic AI Bridge, Windows V1, apres PR #490 (2 novembre 2026)

Tu reprends **le developpement technique effectif** de https://github.com/ELITEAYOTO/Optic-AI-Bridge- apres la fusion de la PR #490. Le present prompt est une **photographie datee**, pas une commande a l'aveugle.

## A lire d'abord
- [Passation technique apres #490](HANDOFF_WINDOWS_V1_AFTER_PR490_2026-11-02.md).
- [Protocole natif/revocation #317](ISSUE317_NATIVE_LAUNCH_REVOKE_PROTOCOL_2026-11-02.md), encore **propose, non implemente**.
- [Issue gel V1 #323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) ; [PR negative #355](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/355).
- Dernieres preuves [PR #490](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/490#issuecomment-6587800904), [CI HEAD](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41518213511), [CI independante push/main](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/41519267054).

**Baseline avant la prochaine PR documentaire** : main `c69ef87155a1dc1a58520f8cfe11384db2c1ff5f`, tree `e9fcb448710e516f6048ea51be644af8531038ab`, independent CI #41519267054 SUCCESS 4/4 tentative 1. **Ce SHA aura change si la PR de passation a ete fusionnee : toujours reinterroger GitHub LIVE avant tout travail.**

## Objectif principal P0
L'ancien GitRead MCP utilise encore `HardenedCommandRunner`. Les PR #489 et #490 ont ajoute un controle stop de **generation originale** juste avant operation puis **chaque** `Command::spawn`, prouve RED/GREEN sur Git reel. Mais le controle n'est qu'une observation ; il n'est pas atomique avec `CreateProcessW` ou `ResumeThread`. **Windows V1 toujours NO-GO**, issue #323 ouverte.

Construire maintenant une veritable **autorite native de lancement proprietaire du service**, pour Git status uniquement au depart, avec session et TaskLease **originales**, policy epoch, host stop, repository/executable pin, deadline absolue, original Job et deux pipes. Revoquer sans bloquer un syscall Windows ; compter et reconcilier les appels deja engages, interdire toute nouvelle admission, ne jamais reprendre apres arret, prouver root exit / Job empty / stdout EOF / stderr EOF / no future call et traiter Unknown/crash. Respecter l'acceptation #355 sans weaken. Ne jamais appeler une custody non-executante « launcher de production ».

## Methode obligatoire
1. Lister les appareils du connecteur Remote Desktop Commander (ne pas supposer un ID) ; verifier qu'il est en ligne. Connecteur GitHub et/ou `gh` pour consulter **main**, SHA, tree, derniere CI de `push/main`, PRs/issus ouvertes et AGENTS/README. Continuer depuis l'etat live, pas depuis un commit en dur.
2. Une **PR terminee a la fois**, branche et repertoire LAB nouvellement isoles ; **aucun** changement au Bridge quotidien, Optic Desktop, REL, systeme, anciens LAB sales, secrets ou permissions.
3. Tests de comportement RED qui **s'executent** avant correction ; distinguer strictement compilateur, rustfmt, 0 tests, timeout/annulation de la vraie RED. Pas de rerun-to-green. Corriger minimalement, revoir entierement le diff, archiver les preuves.
4. Local : `cargo +1.99.0 fmt --all -- --check` ; `cargo +1.99.0 clippy --locked --offline --workspace --all-targets --all-features -- -D warnings` ; `cargo +1.99.0 test --locked --offline --workspace -- --test-threads=1` sur LAB isolE. Hosted exact-HEAD Windows/Ubuntu + dependency/release guard + workflows specialises. Inspecter les tests nommes dans Windows, source/checkout SHA, schema8/artefacts quand pertinents.
5. Publier commentaires de **self-review en COMMENT** (pas APPROVE) ; squash sur `expected_head_sha` et base exacte, comparer tree HEAD et merge ; verifier **nouvelle CI independante push/main** 4/4 premiere tentative sur le commit integre, et publier recu PR et #323. Ne commencer une autre PR qu'apres.
6. Respect absolu des gates : Windows V1 NO-GO, aucun tag/release/installateur, pas de runner auto-heberge, pas de degradation des protections, pas de merge aveugle des DRAFTs #471/#353/#358 ni de la recherche #355.
7. Stockage : creer une archive **verifiee SHA-256**, conserver sources+logs et ne supprimer que des `_target`/`_tmp` de LAB identifies. Ne toucher a aucun autre projet/dossier.

## Vision produit (pour compatibilite seulement)
A long terme Optic gerera des profils/permissions distinctes (lire, creer, modifier, restaurer, annuler), Git complet, compilation, nouveaux projets A-Z, tests isoles securises et VM. **Ne pas programmer ces fonctions maintenant** ; architecturer le noyau de securite pour qu'elles restent possibles.

## Premiere action
Verifier sur GitHub si la PR documentaire de passation est fusionnee, relever nouveau SHA+CI. Rechercher le prochain cas reproductible de vraie **revocation vs Create/Resume** de GitRead Windows avec original-generation issuer (et non un nouveau ticket de ledger inerte). Avancer autonomement selon la methode PR complete ; demander l'avis du proprietaire en cas de decision de produit, relachement de politique ou blocage inseparable de la securite.

**Important : aucune affirmation de GO ni d'absence absolue de processus lance apres revoke tant que les vrais effets Win32 concurrents ne sont pas prouves.**