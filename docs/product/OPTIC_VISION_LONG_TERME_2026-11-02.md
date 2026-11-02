# Optic — vision produit à long terme (proposition)

**Statut : VISION / HORS PÉRIMÈTRE WINDOWS V1.** Rédigé le 2 novembre 2026 à partir de la discussion avec le propriétaire du projet.

Ce document exprime une direction, pas une promesse de disponibilité. Il ne change ni les permissions actuelles ni les conditions du gate [Windows V1 #323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323).

## Ambition

Optic doit devenir un courtier local **d'autorisation, d'exécution et de supervision** pour des agents IA. L'IA propose une action ; Optic vérifie l'identité, le projet, la permission, la cible, les ressources, l'approbation et la révocation, puis réalise — ou refuse — l'effet par un service typé. Le modèle ne devient jamais l'autorité de sécurité.

## Capacités recherchées à terme

### 1. Profils d'agents et permissions fines

- Plusieurs profils indépendants, attachés à des identités, sessions, projets et politiques distincts.
- Lecture, création, modification, suppression, restauration et annulation **seulement dans le périmètre accordé**.
- Droits par famille d'action, chemin canonique, répertoire, exécutable, dépôt Git, destination réseau, secret, durée et budget.
- Actions courantes préautorisées à l'intérieur d'une enveloppe limitée ; confirmation humaine pour les limites sensibles et effets irréversibles.
- Révocation effective, journalisation limitée et protection des données sensibles.
- Une configuration d'interface n'est pas une permission : le moteur Rust reste la seule autorité finale.

**Attention au vocabulaire :** les ToolProfiles existants définissent des invocations de programmes contrôlées, pas encore un système complet de profils d'agents. Cette vision ne demande pas de les renommer ou refactorer maintenant.

### 2. Développement complet mais contrôlé

- Lire, analyser, créer et modifier des projets de A à Z.
- Git par opérations différenciées : status/diff/log, branches, commits, intégration, reset, synchronisation et publication selon des politiques séparées.
- Dépendances, build, compilation et exécution de tests dans des environnements bornés, isolés et jetables.
- Checkpoints de fichiers et de worktrees, journal et récupération après incident.
- Refus d'annoncer un « undo » universel : push, publication, envoi de messages et opérations externes peuvent être seulement compensables ou irréversibles.

### 3. Bibliothèque future d'adaptateurs : Library-MCP

À une phase ultérieure, Optic pourra accueillir une bibliothèque optionnelle d'outils (par exemple Figma, Blender ou d'autres serveurs MCP).

Chaque adaptateur devra déclarer ses effets, cibles, version, provenance, permissions, ressources, besoin réseau, secrets, comportement d'arrêt et réversibilité. Installer ou découvrir un outil **n'accorde aucun droit**. Un adaptateur externe doit être supervisé et, s'il n'est pas pleinement de confiance, isolé hors du processus central.

**Library-MCP est pour l'instant un concept de produit.** Aucune nouvelle crate, API de plugin, connexion distante ni accès Figma/Blender n'est autorisé par ce document.

### 4. IA locale, seulement en dernier lieu

Une IA locale pourrait utiliser Optic comme seule voie d'effets, si son environnement est réellement confiné : pas d'accès ambiant aux fichiers, commandes, secrets, réseau ou applications en dehors du contrôleur. Les prompts ne suffisent pas à imposer ce comportement. Ce chantier doit avoir un modèle de menace et des tests adversariaux propres, après validation du cœur et de l'isolation.

## Séquencement souhaité

1. **Windows V1 :** achever les gates en cours (#323, arrêt/révocation GitRead réel, lectures sensibles, intermittences, installation et récupération) sans réduire les garanties.
2. **Profils et ergonomie :** concevoir progressivement les profils d'agents/permissions et leur gestion depuis Optic Desktop, en conservant le cœur indépendant de l'UI.
3. **Développement de bout en bout :** outils Git supplémentaires et build/test réellement isolés, stratégies de restauration et de reprise.
4. **Library-MCP :** adaptateurs vérifiés et permissions explicites par intégration.
5. **IA locale isolée :** uniquement après preuve de l'absence de chemins d'action hors Optic.

Il ne s'agit pas d'un engagement de version ou de date. Aucune phase future n'autorise à franchir les portes V1 prématurément.

## Non-objectifs immédiats

Pas de nouvelle crate « bibliothèque », de shell universel, de modèle local, de commande Git libre, d'accès distant, de plugin Figma/Blender ni de refactoring transversal préventif pendant la solidification Windows V1.

## Documents de référence

- [Architecture et garde-fous d'évolutivité](../architecture/OPTIC_GARDE_FOUS_EVOLUTIVITE_2026-11-02.md)
- [ADR-0002 : MCP is an adapter](../decisions/ADR-0002-MCP_AS_ADAPTER.md)
- [Architecture système](../architecture/SYSTEM_ARCHITECTURE.md)
- [Invariants de sécurité](../security/SECURITY_INVARIANTS.md)
- [Modèle de politique](../security/POLICY_CAPABILITIES.md)
- [Roadmap actuelle](../../ROADMAP.md) et [statut](../../STATUS.md)

Ce document reste descriptif ; seuls les contrats et tests réellement acceptés dans le code constituent une preuve fonctionnelle.
