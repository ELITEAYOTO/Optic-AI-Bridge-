# Optic AI Bridge — passation technique au nouveau chat
_Date de référence : 22 octobre 2026 (Europe/Paris). Document de reprise préparé sur la branche expérimentale #317. **Ne fait pas foi comme état GitHub live.**_

> Ce fichier est une passation **en attente d'intégration** sur `bootstrap/issue-317-owned-cleanup-study`, et non un document nouveau déjà fusionné dans `main`. Le prochain chat doit relire `AGENTS.md` et vérifier les SHA et CI en direct. `main` conserve à cette date d'anciens checkpoints documentaires ; ne pas prétendre qu'ils ont été mis à jour dans la branche canonique.

## 1. Dépôt et orientation

- **Dépôt unique :** https://github.com/ELITEAYOTO/Optic-AI-Bridge-
- **Mission :** direction technique et **stabilisation de l'existant**, pré-alpha. Le propriétaire délègue les décisions techniques courantes, mais **pas** les permissions, fusions risquées, changements de politique ou exécutions sur son ordinateur.
- **Gel des fonctionnalités :** aucun nouveau transport distant, Electron/GUI, Cargo général, élargissement d'accès, release, merge global `alpha/generated-project-editing`.
- **Exécution autorisée :** lecture/écriture GitHub sur ce dépôt, environnement isolé appartenant à l'agent si autorisé, CI **GitHub-hosted native Windows/Ubuntu**. Ne jamais lancer build, tests, scripts, installers, tunnel, commande, configuration ou service sur le **PC utilisateur**, même via un connecteur. Pas de runner self-hosted.
- **Source de vérité :** SHA et statuts GitHub live, pas les notes datées.

## 2. Inventaire vérifié le 22 octobre 2026

| Sujet | Dernier checkpoint réellement vérifié |
| --- | --- |
| `main` | `80fe108b288d2b0b6676d6128d9e9beb922eda3f` |
| CI indépendante `main` | [#39583041075](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39583041075), **success**, tentative 1, SHA exact de main ci-dessus |
| PR ouvertes | **une seule : [#318](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/318)** ; tête `49fe3efd34c9ffe9c10c1c5945b37da2c6a74028` |
| Gate #318 | [#39585600089](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39585600089), **failure**, tentative 1, Windows first `initialize` MCP timeout 30 s. Ne pas fusionner ni ignorer ; son correctif traite les modes Git non octaux, distinct du timeout. |
| Recherche #317 | `bootstrap/issue-317-owned-cleanup-study` à `7b36d971022f6cd732a01b7440b09022e17e277d` avant cette **mise à jour documentaire** |
| Dernier code #317 évalué | [CI #39696149147](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39696149147), **success** sur `7b36d971022f6cd732a01b7440b09022e17e277d`, Windows/Ubuntu/dépendances, tentative 1 |
| Diagnostic #319 | `bootstrap/issue-319-process-lease-phases` à `49b64e213e144be4ba5a608616183add8e9d47a1`, voir [rapport Codex](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/blob/49b64e213e144be4ba5a608616183add8e9d47a1/docs/research/CODEX_STABILIZATION_2026-10-21.md) |
| État produit | Pré-alpha, #317/#319/#185 ouvertes. **#176 et #182 bloquent l'utilisation distante sur de vrais projets.** |

Après le commit de cette passation, la **tête de recherche changera** sans changer les preuves de code aux SHAs ci-dessus. Vérifier la nouvelle CI de documentation et distinguer CI de documentation vs preuves précédentes sur les SHAs de code. Les SHA et résultats ci-dessus **ne s'appliquent pas automatiquement** à un commit ultérieur.

## 3. Documents à lire dans l'ordre

1. `AGENTS.md`, `AI_HANDOFF.md`, `README.md`, `STATUS.md`, `ROADMAP.md`.
2. `docs/engineering/GITHUB_HANDOFF.md`, `docs/engineering/REPOSITORY_WORKFLOW.md`, `docs/engineering/RELIABILITY_DEBT.md`, `PROJECT_CONTINUITY_PROMPT.md`.
3. `docs/security/SECURITY_INVARIANTS.md`, `docs/decisions/ADR-0034-SENSITIVE-READ-CLASSIFICATION.md`, autres ADR liés à la modification envisagée.
4. **Ce document** et [ASTRA_CODEX_ISSUE317_EVIDENCE_2026-10-22.md](../research/ASTRA_CODEX_ISSUE317_EVIDENCE_2026-10-22.md), puis [ISSUE317_OWNED_CLEANUP_2026-10-21.md](../research/ISSUE317_OWNED_CLEANUP_2026-10-21.md). Ce dernier est un **checkpoint antérieur à plusieurs commits de recherche** : il disait alors « no production code changed », ce qui n'est plus vrai de la **branche expérimentale** après la correction ciblée GitIntegration.
5. Issues et preuves : [#317](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317), [#319](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/319), [#185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185), [#176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/176), [#182](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/182) et commentaires récents; états CI et commit exact.

Les énormes historiques de `STATUS`/`GITHUB_HANDOFF` ne doivent pas être réinterprétés comme état live. Ce checkpoint est placé en tête sur la branche de recherche.

## 4. Ce qui a été vraiment démontré — ne pas refaire

**#317 : blocage pipe natif démontré.** `HardenedCommandRunner` applique son délai à l'attente du processus direct puis effectue des `join()` potentiellement non bornés si un descendant conserve stdout/stderr/stdin. Reproduction sous **Windows natif et Ubuntu** en environnement GitHub hébergé, par superviseur indépendant, avec délai runner 200 ms et retours de l'ordre de 1 s après libération du descendant. Le runner de production de `main` reste **inchangé**.

**Astra/Codex : propriété de nettoyage expérimentale.** Prototypes supervisés conservant processus, pipes, threads et réservation lors de nettoyage incertain ; Linux 17 cas, Windows 16 cas validés sur [#39688523435](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39688523435), puis [CI normale #39688523364](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39688523364) verte sur `06d688b6b05642fc749ab6137188fac9466fa640`. Contre-exemple Linux : descendant échappant au groupe Unix et gardant les pipes ; `waitpid` explicite nécessaire dans la fixture à subreaper **privé au test**, non transposable tel quel dans Bridge. Ce n'est **pas** un correctif production.

**Comptabilisation GitIntegration : défaut et correctif local sur la branche de recherche.** Dans `crates/optic-bridge-mcp/src/git_integrate_tools.rs`, le `RequestPermit` était abandonné lors de l'annulation du waiter alors qu'un `spawn_blocking` travaillait encore. Une caractérisation Ubuntu l'a observé. Correction étroite : déplacer le permis **dans le worker bloquant**, avec tests succès, erreur, annulation et refus de capacité anticipée ; [CI #39690755337](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39690755337), **success Windows/Ubuntu/dépendances** au SHA `3a5bb8cf2e07528e774eca74479c18020e96bf7f`. **Non intégrée dans main et ne corrige pas les pipes.**

**Admission et révocation :** test borné confirmé [#39691887518](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39691887518) au SHA `f2f4422156c96e52f55026b8631ccd5a75b6bbfd`. Admission Git synthétique reste détenue après annulation du waiter ; révocation interdit les nouvelles admissions et attend que le worker se termine. **Saturation de `session_cancel`** : [#39693485443](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39693485443), success Windows/Ubuntu sur `afcee4e479cc71566187c9be42b786280cdf057a` : capacité transport=1 occupée => `session_cancel` renvoie `optic.server_busy` **sans révoquer** ; après libération worker, révocation réussit. Ne pas transformer une absence de révocation en preuve d'arrêt.

**Ambiguïté d'effet Git CAS :** [#39696149147](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39696149147), success Windows/Ubuntu/dépendances au SHA `7b36d971022f6cd732a01b7440b09022e17e277d`. Fixture Rust `issue317_git_ref_timeout.rs` exécute réellement `git update-ref --no-deref` dans un repo **temporaire**, puis tient le processus direct : `HardenedCommandRunner::TimedOut` avec **ref déjà avancée** ; cas témoin même `TimedOut` mais **ref inchangée**. Cela établit l'ambiguïté du résultat de mutation, **pas** un incident naturel de `GitIntegrationService` ou la résolution de #317.

**#319 / #185 restent distinctes :** #319 30 s premier `initialize`, démarrage / empreinte identité Node étudiés, instrumentations et smokes réussis mais cause de l'échec naturel **inconnue**. #185 AppContainer Node ~10 s, échec intermittent sans preuve suffisante de refus réseau lors des rouges; ne jamais déclarer « isolation réussie » à partir d'une absence de sortie.

## 5. Ce qui manque avant un patch natif #317

Une solution crédible exige **un propriétaire et une autorité de nettoyage** avant la création native du processus : `SessionGitRead`, `SessionGitIntegrate`, `SessionWorktree`, **`HostStartupRecovery`** sans faux `SessionHandle`, avec capacité CPU/mémoire/slots, handles, fils et threads I/O possédés et comptés.

- Une demande d'arrêt **explicitement authentifiée et de portée bornée** doit être possible avant le drain des admissions, même lorsque les slots de transport ordinaires sont saturés. Il ne s'agit PAS d'autoriser un bypass MCP ou un kill par PID arbitraire.
- `SessionRegistry::revoke` marque puis attend `active_admissions=0`. `SessionLifecycleManager::revoke` n'appelle `ProcessManager::cancel_session` qu'après ce drain ; `HardenedCommandRunner` Git ne figure pas dans `ProcessManager`. Le cleanup **post-drain** ne peut pas être l'unique moyen de débloquer l'admission Git retenue.
- Préserver `RunningOwned -> StopRequested -> NativeTreeProvenTerminal -> PipeIoCancelledAndJoined -> ResourceRelease`. Tout échec partiel = owner `RetainedUncertain` **non évictable**, ressources/capacité/admission conservées, nouvelle admission refusée. Le shutdown doit également le traiter.
- Windows : assignation à Job Object avant resume, confirmation native de terminaison **et** des pipes/threads. Unix : groupe échappable; un sous-reaper global Python n'est pas un contrat acceptable dans le Bridge Rust. Prouver reaping des enfants légitimement possédés, sans PID arbitraire.
- Ne jamais confondre **`TerminationUncertain`** (ressources/processus) et **`GitRefOutcomeUncertain`** (effet potentiellement déjà commité). Timeout après CAS ne signifie pas « zéro effet »; pas de retry automatique, pas de relâchement CAS.
- `seal_quiescent`/`try_reap` doivent observer le futur registre Git, et non seulement les jobs `ProcessManager`.
- Pas de simple `join` détaché, augmentation de timeout, élargissement de permissions ni réduction de couverture Windows.

Registre et formulation de tests proposés : [issue #317, contrat](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317#issuecomment-6329018999) et [preuve CAS](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317#issuecomment-6329260125). (Espaces autour des URL à ignorer lors de la copie Markdown.)

## 6. Plan d'action pour le prochain chat

1. **Contrôle live obligatoire.** Inspecter `main`, toutes PR ouvertes, issues et tentatives CI; vérifier la tête de recherche après la présente passation. Ne pas interpréter comme actuel un SHA historique. Lire les règles et le registre de preuves.
2. **Choisir un incrément scientifique utile.** Le plus pertinent : conception / tests isolés Rust d'un owner conservant l'admission et la réservation sous arrêt incertain **sans toucher aux services de production** tant que l'architecture n'est pas prouvée. Alternativement étudier la sémantique d'erreur Git CAS, mais uniquement après formuler un test bout-en-bout falsifiable de `GitIntegrationService`. Ne pas dupliquer les smokes déjà verts.
3. **Si modification justifiée :** une branche propre par sujet, fixture synthétique strictement bornée et watchdog indépendant, Windows natif + Linux sur SHA exact, preuves de négatifs et nettoyages, aucune CI contournée. **Une seule PR ouverte : #318**, donc pas de nouvelle PR tant qu'elle bloque. Éviter d'empiler arbitrairement des patches hétérogènes sur la branche #317.
4. **Avant toute intégration :** isoler le correctif minimal depuis `main`, relire ADR/invariants, faire une revue native, ne pas fusionner si #318 est encore rouge. Des CI vertes sur #317/#319 ne réparent pas #318.
5. **Documentation à jour après chaque preuve :** issue, SHA, tentative CI, système, hypothèse, résultat, négatifs, limites. Ne jamais déclarer le produit « stable à 100 % ».

## 7. Astra / Codex : contexte de collaboration

Le propriétaire utilise aussi un agent nommé **Astra / Codex** pour des campagnes approfondies. Lors du dernier échange, Astra a analysé #317 sur une branche autonome et produit 16 cas Windows / 17 cas Linux, sans changement de production. Astra a commencé une approche d'owner avec retention `Uncertain`, puis son quota a chuté (mesures rapportées **98 %, puis 35 %, 18 %, 1 %**) et a été épuisé. La prochaine possibilité annoncée par l'interface à ce moment était **04 h 37** heure locale, mais **ne pas considérer cette heure ou le pourcentage comme un quota live**. Les crédits sont une contrainte de campagne, pas une raison de précipiter un correctif.

Depuis, le chat a corrigé le dernier cas fixture Linux d'Astra, obtenu les validations natives et effectué des expériences supplémentaires GitIntegration, admission/révocation, saturation et CAS. Astra doit donc **repartir du SHA GitHub récent**, pas de son ancien texte compacté ni de ses anciens fichiers de preuve. Quand il est disponible, lui confier une mission bornée sur les preuves manquantes, garder **10–15 % pour clôture**, arrêter les nouveaux chantiers vers 20 %, conserver les premières tentatives rouges. L'utilisateur transmet ses rapports et ZIP, à auditer avant toute intégration. **Aucun travail en arrière-plan n'est supposé par cette note.**

## 8. Blocages sûreté non résolus

- [#176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/176) : secrets dans fichiers à noms ordinaires, identités/alias et courses.
- [#182](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/182) : provenance Git index/commit, métadonnées refs/branch et classification incomplète. Les corrections précédemment fusionnées fournissent un plancher, pas une confidentialité exhaustive.
- [#185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185) et [#319](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/319) : timeouts distincts sans cause naturelle déterministe établie.
- Pas d'usage distant réel sur workspace client et aucune affirmation que les protections de session/procès sont achevées.

**Règle finale :** le prochain chat doit privilégier **une preuve reproductible supplémentaire ou une absence de modification motivée** à un patch spéculatif.