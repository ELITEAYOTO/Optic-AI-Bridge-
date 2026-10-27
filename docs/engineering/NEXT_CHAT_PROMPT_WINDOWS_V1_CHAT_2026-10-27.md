# OPTIC AI BRIDGE — REPRISE EN CHAT CLASSIQUE — 27 OCTOBRE 2026

Tu reprends le projet comme responsable technique principal, architecte Rust/Windows, développeur principal, auditeur sécurité, reviewer et responsable des validations CI.

1. MODE DE TRAVAIL ET AUTORITÉ

Nous travaillons dans un CHAT CLASSIQUE. Je ne demande ni une tâche Codex, ni un espace Work, ni une exécution sur mon ordinateur. Ne crée pas de tâche/agent local, d'autre conversation ou d'automation et ne délègue pas à un autre agent sans ma demande explicite. Avance dans ce chat, avec les outils GitHub réellement disponibles.

Je te délègue les décisions techniques courantes dans le dépôt autorisé. Tu peux inspecter, développer sur une branche isolée, ouvrir une PR draft, corriger, valider et fusionner une petite correction prouvée selon les règles ci-dessous, sans redemander confirmation à chaque opération normale. Je veux du travail concret, maintenable et traçable, pas seulement un plan.

Si un outil manque ou si GitHub est accessible seulement en lecture, précise exactement ce qui est impossible. N'invente jamais lecture, commit, publication, test, fusion ou travail en arrière-plan. Prépare ce qui est réellement vérifiable et demande l'accès ou les contenus indispensables. Ne passe pas à Codex/Work ou à un accès machine pour contourner cette limite.

AUCUNE AUTORISATION PERMANENTE d'accéder à mon ordinateur, même en lecture seule, ni d'y exécuter une commande, un terminal, des tests, un serveur, un installateur ou un runner auto-hébergé. Tout accès machine nécessite un accord explicite, récent et limité. Les validations de cette reprise utilisent GitHub et la CI hébergée.

2. DÉPÔT UNIQUE ET OBJECTIF

https://github.com/ELITEAYOTO/Optic-AI-Bridge-

C'est le Bridge MCP local en Rust, priorité Windows 10/11 V1. Ne touche ni à Alpha-Optic-APP (application Desktop, autre IA), ni à REL, ni à leurs branches, PR, fichiers ou worktrees. Stabilisation Windows V1, pas de nouvelles fonctionnalités, de GUI ou de développement distant hors périmètre.

Windows V1 reste NO-GO. Les issues #317 et #323 restent OUVERTES tant que toutes leurs exigences ne sont pas démontrées. Aucun tag/release, assouplissement de sécurité ou fermeture anticipée. Les réglages GitHub, permissions et décisions de release nécessitent l'autorisation explicite du propriétaire lorsqu'ils sont concernés.

3. LECTURES OBLIGATOIRES DEPUIS MAIN ACTUEL

Lis directement dans le dépôt, avant toute modification :
- AGENTS.md
- AI_HANDOFF.md
- STATUS.md
- ROADMAP.md
- CHANGELOG.md
- PROJECT_CONTINUITY_PROMPT.md
- docs/engineering/HANDOFF_WINDOWS_V1_CHAT_2026-10-27.md
- docs/engineering/NEXT_CHAT_PROMPT_WINDOWS_V1_CHAT_2026-10-27.md
- docs/engineering/GITHUB_HANDOFF.md
- docs/engineering/NATIVE_GIT_READ_WITNESS_CONTRACT.md
- docs/security/SECURITY_INVARIANTS.md
- docs/engineering/REPOSITORY_WORKFLOW.md
- docs/engineering/CODEX_INTERRUPTION_RECOVERY.md

La passation CHAT du 27 octobre est le point d'entrée actuel. Celle du 26 octobre contient les prérequis et historiques antérieurs ; celles des 22–26 sont des instantanés pour leurs SHA/CI. Ne les traite pas comme l'état GitHub présent.

4. BASELINE À REVÉRIFIER, PAS À SUPPOSER

Dernier main vérifié AVANT la PR documentaire de cette nouvelle passation :
75947c9b4fb72fab2f3d45f34f5b750f09bc31b2

CI indépendante push/main :
https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/40477277322

Résultat vérifié : SUCCESS, run_attempt=1, quatre jobs sur quatre sur ce SHA exact : Rust Windows, Rust Ubuntu, Dependency policy, Release evidence guard.

La PR documentaire peut avoir fait avancer main. Commence donc par vérifier en direct :
- le SHA réel de branches/main ;
- la dernière CI event=push / branch=main sur CE SHA, sa tentative et chaque job ;
- les PR ouvertes, base, HEAD et draft ;
- les issues #317, #323, #185 et #319 ;
- les commits/travaux éventuellement publiés depuis le checkpoint par une autre IA.

Une CI verte d'une ancienne PR ou recherche ne prouve jamais le comportement du main actuel.

5. CE QUI A RÉELLEMENT ÉTÉ INTÉGRÉ

Les prérequis #369/#371/#373/#375/#377 sont présents : TaskLease enregistrée, scopes canoniques, politique, témoin de révocation de génération, admission comptée et permis de transport réels, transfert sans seconde admission.

Nouveaux changements du 27 octobre :
- #380 : run_owned_admitted_read possède une vraie OwnedSessionAdmissionPermit et le RequestPermit original dans le worker MCP. Les lectures Git utilisent ce worker et conservent les contrôles sensibles. L'interface empruntée délègue sans seconde admission.
- #381 : le préflight conserve l'échéance monotone absolue de l'ActionEnvelope. Une action de 100 ms n'est plus prolongée par un transport de 1 s. Stop ne libère pas ses réservations.
- #382 : GitReadService::prepare_native_status dérive son status fixe des cibles configurées du service, utilise le budget de sortie exact de l'action et consomme les permis originaux via le préflight enregistré. PreparedNativeGitStatus conserve commande, enveloppe immuable, admission, lease et première échéance capturée.
- #383 : NativeProcessReservation::finish ne synthétise plus la fermeture stdout/stderr sous TerminationUncertain, y compris Running normalisé en incertitude. Les observations par flux, capacités et refus de réapage sont conservés.

SHA intégrés / CI push/main indépendantes, toutes 4/4 au premier essai :
- #380 : ab45498c32b2cb9cdfddfdeaa32f0230447d9de0 / #40467892414
- #381 : 588f50f0a3fee4e2f6b1c08c21db4950888d6d77 / #40471474314
- #382 : d9d004b2bbd65c4f7cc520abfde3beb7963f0095 / #40474172602
- #383 : 75947c9b4fb72fab2f3d45f34f5b750f09bc31b2 / #40477277322

Le handoff contient les têtes de PR, CI candidates, préparations natives et liens exacts. Les premiers rouges sont préservés :
- #380 : format puis compilation E0599, pas une reproduction runtime ;
- #381 : #40470425246, vrai échec runtime du contrat inerte sur Windows/Ubuntu à l'échéance de l'action ;
- #382 : format et compilation d'interface absente, tests avec octets épinglés inertes, aucun Git exécuté ;
- #383 : #40476042507, échec runtime Windows dans le vrai registre avec état SIMULÉ (366 pass / 1 fail / 11 ignored), sans enfant ni faute Win32 injectée ; après correction 367 pass / 11 ignored. Le test cfg(windows) ne s'exécute pas sur Ubuntu.

Ne transforme pas ces preuves en preuve de terminaison native ou de vrais EOF.

6. LE RACCORD DE PRODUCTION N'EST TOUJOURS PAS FAIT

Dans crates/optic-bridge-mcp/src/git_tools.rs, authorize_git_read construit toujours task_lease: None pour le chemin legacy.

GitReadService utilise toujours HardenedCommandRunner dans crates/optic-bridge-runtime/src/git/read.rs. Les attentes de lecteurs stdout/stderr et d'écriture stdin peuvent toujours dépasser le délai lorsqu'un descendant garde les handles.

prepare_native_status et PreparedNativeGitStatus existent, MAIS la méthode n'est pas appelée par le handler MCP actif. Ils ne réservent pas de quotas natifs CPU/mémoire/processus, n'enregistrent aucun Job/root/pipes, ne créent aucun enfant et ne fournissent aucune preuve OS terminale.

#383 corrige un producteur de réservation privé existant ; il ne crée pas le propriétaire Git complet. Ne remplace pas brutalement les autorisations legacy par une TaskLease obligatoire sans migration complète ni conservation de la confidentialité.

7. RECHERCHES À CONSERVER SÉPARÉES

Au dernier contrôle, cinq PR étaient ouvertes :
- #327 : base bootstrap/issue-317-windows-rust-owner, HEAD ef3208d787baa0c6433abfa0b4aba687d8bc6bb7 ; recherche historique, non-draft.
- #352 : base main, HEAD 2fa45d6b2d1f3dd39309ca7d18a2209216b7f959 ; draft, acceptance de temps réel ROUGE.
- #353 : base main, HEAD b091dc280bfd186694e7c1d1bea743a20f6746c2 ; draft, propriétaire natif expérimental.
- #355 : base main, HEAD 5c104ee67f26b8d7b68132cf05a5f86eb53b580f ; draft, acceptance de révocation avant spawn ROUGE.
- #358 : HEAD f431c3efde2e5d9512eb1715ad3475196f18bf1d ; draft, EMPILÉE sur research/317-prepared-read-native-staging-2026-10-24 (#353), PAS sur main.

NE FUSIONNE JAMAIS #353/#358 en bloc. #366 n'a été intégré que dans cette recherche. Porter seulement des changements ciblés, revus et sans doublons.

8. PROCHAINE PRIORITÉ CONCRÈTE DE #317

Avancer vers un véritable propriétaire natif Windows ENREGISTRÉ DANS LE SERVICE, qui prend en charge le PreparedNativeGitStatus exact de #382 et les mêmes admission worker/session, RequestPermit et TaskLease Git canonique, sans double admission ni libération prématurée.

Inspecte les signatures réelles dans :
- crates/optic-bridge-mcp/src/server.rs et git_tools.rs ;
- crates/optic-bridge-runtime/src/git/read.rs et git/native_status.rs ;
- native_git_read_preflight.rs, session/registry.rs et task_lease_registry.rs ;
- process.rs, process/tests.rs et private_node_task.rs ;
- crates/optic-bridge-windows/src/job.rs et output_pipe.rs ;
- les propriétaires et tests expérimentaux #353/#358 ;
- les contrôles des chemins Git sensibles et de publication des résultats.

Réserver budgets globaux/par session CPU, mémoire, processus et sortie ; enregistrer propriétaire, Job/root/pipes AVANT toute création native. Fixer/vérifier Git par identité ouverte, épinglée et hashée, et la racine canonique. Réévaluer politique/epoch/session/lease/révocation/échéances à la dernière frontière de création SYNCHRONISÉE avec la révocation.

Un simple check suivi d'un spawn séparé, ou un témoin non révoqué, n'autorise jamais le lancement.

Sous TerminationUncertain, conserver owner, handles, permis, capacités, lease et récupération jusqu'aux preuves adaptées : racine terminée, Job vide, véritables EOF stdin/stdout/stderr selon le modèle de flux. Couvrir descendants, annulation/abandon, watchdog, crash hôte et récupération.

Pas de faux EOF, lecteurs détachés, PID arbitraire ou nettoyage présumé. Une étape non exécutante peut être nécessaire pour un obstacle précis, mais ne pas accumuler indéfiniment des helpers sans consommation réelle par le propriétaire.

9. ACCEPTANCES ET V1 TOUJOURS BLOQUANTES

#352 / CI rouge #40087176209 : timeout runner 200 ms, durée réelle environ 1,6 s alors que la borne est <900 ms. Renvoyer TimedOut après 1,6 s reste un échec. Ne change ni test ni seuil.

#355 / CI rouge #40158288203 : vrai Git lancé après publication de révocation pendant le drainage. Exiger propriétaire enregistré avant création et autorité de lancement synchronisée.

Puis résoudre séparément :
- #185 : intermittence Node/AppContainer Windows ;
- #319 : première réponse MCP potentiellement bloquée vers 30 s ;
- #176/#182 : confidentialité Git/contenus sensibles/provenance et approbations ;
- #323 : vrais essais Windows 10/11 autorisés, compatibilité, installation hors ligne, mise à jour, rollback, incidents, fiabilité, preuves et provenance release.

Une CI standard verte sur Windows Server ne satisfait pas tous ces critères. #317/#323 restent ouvertes, Windows V1 NO-GO.

10. MÉTHODE OBLIGATOIRE

- Partir du SHA actuel de main vérifié en direct ; branche isolée, une correction cohérente par PR. Ne jamais réécrire les travaux concurrents ou fusionner la recherche en bloc.
- Test-first : démontrer la propriété, constater le rouge avant correction quand possible. Qualifier séparément formatage, compilation, runtime, état simulé et preuve OS réelle. Une passation purement documentaire ne nécessite pas de rouge artificiel.
- Vérifier rustfmt, Clippy avec warnings refusés, Rust Windows/Ubuntu, dépendances, garde de release et validations natives applicables.
- Vérifier HEAD exact, SHA du run, tentative et résultat de chaque job. Corriger sur un NOUVEAU commit et conserver le premier échec ; pas de reruns pour masquer le rouge.
- PR en draft jusqu'à validation complète ; revue du diff, fichiers, base, HEAD, invariants et retours. Fusion squash avec expected_head_sha. Si ce contrôle est indisponible, le dire et ne pas prétendre l'avoir appliqué.
- Après fusion, récupérer le NOUVEAU SHA de main et vérifier sa CI indépendante event=push / branch=main. Un vert de PR n'est pas un vert du commit intégré.
- Actualiser issues, STATUS/ROADMAP/CHANGELOG et contrats/handoff quand nécessaire. Conserver liens CI exacts et limites. Aucun auto-close #317/#323.
- À chaque checkpoint : changements réels, PR/commits, tests et CI, premiers rouges, main vérifié, problèmes ouverts, NO-GO et prochaine priorité. Pas de promesse de travail invisible après la réponse.

11. SÉCURITÉ ET RÉCUPÉRATION

L'IA propose ; l'autorité applicative et la politique déterministe autorisent ; l'isolation contient.

Préserve identités serveur, scopes structurés, capacités étroites/révocables, délais monotones, chemins canoniques et contrôles symlink/reparse, quotas bornés, environnement allowlist sans secrets ambiants, et isolation du unsafe Win32 dans sa couche dédiée. Pas d'auto-autorisation IA, shell libre, arrêt arbitraire de PID, désactivation AppContainer/Defender ou contournement d'une protection.

Respecte CODEX_INTERRUPTION_RECOVERY.md sans le prendre comme autorisation d'accès à mon PC. Dans ce Chat GitHub-only, si le disque n'est pas autorisé/accessible, indique :
local recovery location inaccessible — not verified

Ne prétends pas avoir vérifié STATE/ACTIONS/TESTS/RECOVERY, SNAPSHOT/CURRENT.json, VERIFIED.json, checksums, HEAD/remote ou snapshots locaux. Aucun reset ou écrasement de travail non enregistré. Si je demande plus tard un test local, prépare un prompt exact dépôt/branche/SHA, scénarios, commandes, versions Rust/MSVC/SDK, journaux et empreintes ; exécution seulement après mon accord. Préserver un checkpoint vérifié avant épuisement de quota lorsqu'un environnement autorisé le permet.

Commence maintenant par vérifier GitHub en direct, lire la passation CHAT du 27 octobre et le code actuel, puis avancer concrètement sur la prochaine correction ciblée de #317. Reste en Chat classique et dans le dépôt unique. Windows V1 NO-GO ; #317 et #323 ouvertes.
