# Optic — garde-fous d'évolutivité à préserver (proposition)

**Statut : DOCUMENT DE CONCEPTION / PAS UNE ADR ACCEPTÉE.** 2 novembre 2026. Ce texte n'altère aucun contrat exécutable ni les gates [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323).

## Décision de méthode

**Préserver des frontières de confiance maintenant, sans construire prématurément une architecture de plugins ou d'IA locale.** Toute modification V1 du cœur motivée par cette vision doit être justifiée par un défaut concret, un test ou une incompatibilité démontrée ; sinon documenter et différer.

## Fondations existantes à conserver

1. **MCP reste un adaptateur** (ADR-0002) : pas de logique de sécurité dépendante du transport. Les services typés doivent rester testables indépendamment d'un client MCP.
2. **Effets normalisés et typés** : les arguments bruts de l'IA ne deviennent pas des commandes système. La politique doit être évaluée sur la cible canonique et l'état réel.
3. **Identité et autorité d'origine** : session applicative, TaskLease et génération, scope, policy epoch, contexte de projet, délais monotones et budgets sont contrôlés côté Optic ; aucun agent ne peut les forger ou les élargir.
4. **La découverte d'outil n'est pas une permission** : un outil visible, installé ou déclaré par un manifest ne reçoit pas automatiquement son autorité.
5. **Services OS possédés et bornés** : handles, processus descendants, pipes, quotas et arrêt restent sous responsabilité du Bridge jusqu'à preuve terminale ou classification explicite « incertain ».
6. **Mutations à préconditions** : versions attendues, identité canonique, journal/récupération quand l'action est réversible, et catégories distinctes pour les effets externes irréversibles.
7. **Données non fiables** : fichiers, sorties d'outils, réponses d'extensions et instructions d'un modèle ne peuvent ni changer la politique ni approuver des secrets ou un accès réseau.

## Profils : ne pas confondre les concepts

Le ToolProfile actuel (dans optic-bridge-core et la configuration app) sert à décrire **une invocation d'exécutable** avec arguments, cwd, environnement, limites et approbation contrôlés.

Un futur **AgentProfile / PermissionProfile** représenterait l'ensemble des capacités octroyées à un agent, avec ses scopes, règles de confirmation, quotas, autorité de projet et durée. C'est une future couche d'orchestration à concevoir sur les primitives déjà présentes, **pas un remplacement immédiat de ToolProfile**.

Questions de compatibilité à poser à chaque PR de sécurité :
- La permission vient-elle bien de l'opérateur ou d'un registre serveur, jamais d'un champ fourni par l'IA ?
- La cible est-elle résolue avant décision et contrôlée à l'engagement ?
- La révocation d'une session/lease/policy/host peut-elle encore permettre un nouvel effet ?
- Les limites CPU, mémoire, disque, sortie, durée et réseau sont-elles réellement imposées ?
- L'échec d'une preuve de terminaison laisse-t-il l'état explicitement incertain plutôt qu'un faux succès ?

## Extensions Library-MCP : future frontière de confiance

**Déclaration future d'un adaptateur (indicative, non exécutable) :**
- identifiant, namespace, version et provenance du composant ;
- opérations et effets typés, classes de cibles, permissions/scopes requis ;
- contraintes de ressources, accès réseau, gestion des credentials, sortie maximale ;
- approbation et classification read-only / transactional / compensable / irreversible ;
- supervision, arrêt, preuve de fin, comportement de récupération et retrait d'autorisation.

**Un manifest de plugin décrit, mais n'accorde jamais l'autorité.** Le point d'effet doit encore être contrôlé.

Une crate Rust exécutée dans le processus du Bridge partage sa zone de confiance ; l'appeler « plugin » ne l'isole pas. Les adaptateurs externes ou serveurs MCP tiers ne sont pas automatiquement sûrs non plus : leurs privilèges OS, réseau, secrets et processus doivent être traités comme un modèle de menace spécifique. Prévoir à terme un superviseur hors processus pour les extensions non fiables.

Pour Figma, séparer lecture et modification, documents et droits d'accès, jetons et appels réseau. Pour Blender, distinguer inspection/modification de scène, rendu et exécution de scripts Python, dont les risques diffèrent.

## Build, Git et annulation : besoins à différencier

- GitRead, GitIntegrate, commit local, reset, push/force-push et gestion des remotes ne devraient pas être une seule permission « Git tout ».
- Les builds/tests de code de dépôt sont des exécutions potentiellement hostiles. Un Windows Job Object gère des arbres et ressources mais **n'est pas à lui seul une sandbox de fichiers et réseau** ; utiliser une isolation validée ou une VM jetable selon le risque.
- Le retour arrière exige un checkpoint ou une transaction réellement prouvée. Les effets sur des services extérieurs peuvent être impossibles à annuler ; demander alors un accord distinct.
- L'IA locale ne sera confinée que si ses processus, accès et chemins de sortie sont contrôlés hors du texte du prompt.

## Préoccupation P0 actuelle : révocation et engagements réels

Le renforcement de #317 est aussi la bonne base pour des extensions futures, mais ce n'est pas un motif pour créer un framework universel aujourd'hui.

Au 2 novembre, après #482 : l'observation simultanée des registres session et TaskLease ne constitue pas encore un engagement Win32 linéarisé avec policy epoch, host-stop, réels CreateProcessW/ResumeThread et custodies physiques. GitRead de production utilise toujours HardenedCommandRunner. Le test de révocation réelle #355 demeure négatif ; #323 reste **OPEN / V1 NO-GO**. La sécurisation de ces chemins existants est prioritaire.

## Règle de décision pour les prochains développements

Classer toute demande future en :
- **Déjà préservée** : aucune modification de code ; référence documentaire.
- **Défaut démontré de sécurité/architecture V1** : scénario reproductible, test RED, correctif minimal, PR complète, CI Windows/Ubuntu, comparaison de trees, CI push/main indépendante.
- **Évolution future spéculative** : noter dans la roadmap, ne pas ajouter de crates ni de privilèges maintenant.

Le propriétaire peut faire évoluer cette vision sans qu'elle contourne le processus de revue ou les invariants actuels.

## Références

- [Vision produit](../product/OPTIC_VISION_LONG_TERME_2026-11-02.md)
- [ADR-0002](../decisions/ADR-0002-MCP_AS_ADAPTER.md)
- [Invariants](../security/SECURITY_INVARIANTS.md)
- [Politique](../security/POLICY_CAPABILITIES.md)
- [Architecture IA](AI_NATIVE_SAFETY_ARCHITECTURE.md)
