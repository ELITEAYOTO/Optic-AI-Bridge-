# Test FranceStudent — lecture réelle, fichiers générés seulement

Ce paquet expérimental ne permet pas de choisir un projet personnel. Tous les
chats utilisant cette connexion partagent la même session Optic de test. Aucun
jeton réel n'est demandé. Le binaire et les scripts proviennent du commit indiqué
dans SOURCE_COMMIT.txt ; le manifeste détecte les mélanges de binaires, sans
constituer une signature ni une protection contre une modification malveillante.

Procéder une étape à la fois avec Codex ; ne pas démarrer le tunnel avant la
validation du paquet et du démarrage local.

1. Extraire le ZIP dans un nouveau dossier et ouvrir PowerShell à sa racine.
2. Installer l'environnement isolé :
   `powershell -NoProfile -ExecutionPolicy Bypass -File .\experiments\real-link-admission\scripts\Setup-FixedReadHost.ps1`
3. Démarrer le serveur de test :
   `powershell -NoProfile -ExecutionPolicy Bypass -File .\experiments\real-link-admission\scripts\Start-FixedReadHost.ps1 -AllowRandomTunnelHost`
4. Dans un autre PowerShell : `cloudflared tunnel --url http://127.0.0.1:8008`.
   Utiliser l'URL obtenue avec `/mcp`. Garder les deux fenêtres ouvertes.
5. Configurer le MCP FranceStudent avec ces valeurs publiques de test seulement :
   Authorization `Bearer optic-probe-dummy-owner-a` ; headers JSON
   `{"X-Optic-Probe":"optic-probe-dummy-header","X-Optic-Probe-Secret":"optic-probe-dummy-secret"}`.
   Si FS ajoute déjà Bearer au champ, suivre le format vérifié lors des précédents
   essais et confirmer `scheme: Bearer` dans le retour, sans doubler le préfixe.
6. Premier prompt Luna :
   « Appelle réellement optic_session_info, puis optic_fixture_list, puis
   optic_fixture_read, une fois chacun, sans argument. Affiche les trois résultats
   complets. Si un appel échoue, arrête-toi sans inventer de résultat. »
   Comparer les markers/server_instance/request_sequence avec la console.
7. Dans le serveur : `rotate-a`. Sans modifier FS, demander un seul nouvel appel
   `optic_session_info`. Le refus401 est attendu ; ne pas créer une session.
8. Mettre FS à jour avec le dummy `optic-probe-dummy-owner-a-v2`, puis demander
   `optic_session_info` et `optic_fixture_read`. Les métadonnées de la session
   réelle doivent rester identiques ; aucun délai n'est renouvelé implicitement.
9. Dans le serveur : `revoke-a`. Un nouvel appel doit être refusé401.
10. Dans le serveur : `stop`. Vérifier l'acknowledgement local
    `ok:true`, `drained:true`, `session_revoked:true`, puis arrêter cloudflared.
    Un code d'échec ou des logs absents ne constituent pas une preuve de succès.

Durée maximale25minutes, huit demandes amont retenues maximum, trois outils fixes.
Pour les essais suivants, redémarrer crée une nouvelle fixture et session de test.
Pas de persistance ni reprise d'un projet réel. Les appels simultanés du host,
son annulation de génération et tous les outils d'écriture/processus/Git ne sont
pas validés par ce scénario.
