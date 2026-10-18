# Alpha édition de projet — test avec FranceStudent

Cette alpha conserve un projet public généré après arrêt et accepte sa reprise.
Tous les chats de la connexion partagent le projet. Un Node installé peut être
choisi localement pour les profils fixes check/test. Commandes arbitraires,
installation de dépendances et Electron contrôlé par le modèle restent exclus.
Le lecteur Electron est local et séparé. Les étapes d'édition restent un parcours
minimal. Mode ordinaire :50minutes maximum. `-Supervised` renouvelle des baux
bornés tant que le superviseur local reste actif, avec arrêt par l'opérateur.
La mémoire interne du chat FS n'est pas restaurée par la reprise des fichiers.

## Fichiers plus volumineux (paquet compatible requis)

Ajouter `-MaxFileKiB 32` à la commande autorise32768octets UTF-8 par fichier,
au lieu de8192par défaut. Conserver ce choix avec `-ResumeRun` si des fichiers
dépassent8Kio. Le manifeste doit porter `file_budget: operator-file-budget-v1`.
Les pages restent1024octets ou4096avec `-ReadPageBytes 4096`. Le corps HTTP reste
limité à65536octets : de nombreux caractères JSON échappés peuvent nécessiter
des petits patches versionnés. Le modèle consulte `optic_project_info.limits` ;
il ne peut élargir les permissions. Ne pas compacter artificiellement sous8Kio
si32Kio est autorisé. La recherche reste limitée au préfixe8Kio.

## Lectures paginées corrigées

Chaque appel respecte le plafond local1024ou4096octets, même si max_bytes=8192.
Lire à offset=0 puis suivre next_offset jusqu'à eof=true. Chaque bloc contient
content_version : exiger la même version sur tous les blocs avant de modifier.
Si la version change, reprendre la lecture complète sans écrire. Les champs de
suivi précèdent le texte ; aucun base64 dupliqué pour les blocs UTF-8 valides.
La version vient d'une lecture Rust complète bornée, pas d'un hash inventé.
Les logs HTTP/TOOL/STATUS/COMPLETE ont timestamp_utc à la milliseconde ; les
durées elapsed_ms/request_elapsed_ms restent monotones.

## 1. Préparer et lancer

Extraire Optic-Project-Alpha.zip dans un nouveau dossier, par exemple
E:\Optic-Test-FS\alpha1. Dans PowerShell, depuis ce dossier :

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\experiments\project-alpha\scripts\Setup-ProjectAlpha.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File .\experiments\project-alpha\scripts\Start-ProjectAlpha.ps1 -AllowRandomTunnelHost
```

La console doit afficher GENERATED PROJECT ALPHA et une adresse locale sur8009.
Garder cette fenêtre ouverte. Dans une deuxième fenêtre :

```powershell
cloudflared tunnel --url http://127.0.0.1:8009
```

## 2. Configurer la connexion FS

Utiliser l'adresse HTTPS annoncée par cloudflared, suivie de /mcp.
Token public de cette seule fixture : `optic-probe-dummy-owner-a`.
Conserver le mode qui a déjà fonctionné lors des tests précédents : les logs
doivent indiquer un seul header Authorization avec le schéma Bearer.
Headers JSON publics :

```json
{
  "X-Optic-Probe": "optic-probe-dummy-header",
  "X-Optic-Probe-Secret": "optic-probe-dummy-secret"
}
```

Activer la connexion. Ouvrir une nouvelle conversation Luna afin d'éviter une
ancienne liste d'outils. Six outils sont attendus : optic_project_info,
optic_fs_list, optic_fs_read, optic_fs_write, optic_fs_patch, optic_fs_search.
Ces valeurs d'authentification sont publiques et ne protègent aucun projet réel.
N'y mettre aucune donnée personnelle ; cette alpha sert au projet de test.

## 3. Premier prompt : confirmer le projet

> Appelle réellement optic_project_info puis optic_fs_list avec path="src".
> Affiche les deux résultats. N'écris aucun fichier pour l'instant. Si un appel
> échoue, arrête-toi et affiche le résultat réel.

Comparer les markers/request_sequence avec les lignes TOOL dans PowerShell.
Après confirmation, envoyer le prompt suivant.

## 4. Faire développer un mini-site

> Réalise réellement un petit gestionnaire de tâches en HTML/CSS/JavaScript,
> en effectuant les appels Optic l'un après l'autre, sans appels simultanés.
> sans bibliothèque, téléchargement, lien externe ni requête réseau. Crée
> src/index.html, src/style.css et src/app.js avec les outils Optic. Interface
> française, ajout/suppression/validation des tâches et filtre toutes/actives/
> terminées. Respecte file_bytes annoncé par optic_project_info et utilise des liens relatifs.
> Pour une création utilise expected_kind="absent". Pour modifier un fichier,
> lis-le entièrement et utilise son content_version comme expected_version
> avec expected_kind="content". Ne devine pas la version et ne réessaie pas
> aveuglément une écriture outcome_unknown. Les patches comptent des octets
> UTF-8. Termine par la liste et une lecture des trois fichiers pour vérifier
> leur contenu. Aucun outil d'exécution n'est disponible : ne prétends pas
> avoir lancé le site ou ses tests. Fournis les résultats réels et un résumé.

## 5. Vérifier une vraie modification

> Lis src/index.html et ajoute un compteur visible de tâches restantes.
> Modifie les fichiers nécessaires avec leurs versions exactes, puis relis-les.
> Utilise aussi optic_fs_search pour retrouver le texte du compteur dans le
> fichier correspondant. Affiche les résultats réels ; n'invente aucune exécution.

## 6. Arrêter et récupérer le projet

Dans la console du serveur, taper `stop`. Un résultat local avec ok=true,
drained=true et session_revoked=true est attendu. Les fichiers restent dans :
`output/alpha-.../project/src`. Le nom exact est annoncé au démarrage.
Inspecter les fichiers avant de les ouvrir dans un navigateur ; l'alpha n'offre
pas de confinement au code JavaScript exécuté par le navigateur.
Arrêter également cloudflared avec Ctrl+C dans sa propre fenêtre.

Pour le retour, envoyer le résumé de Luna et les lignes de preuve de la console
(sans contenu personnel). Indiquer si les trois fichiers existent et si leurs
modifications sont présentes. En cas d'erreur, conserver le projet et son journal
sans les modifier ; afficher l'erreur réelle avant tout nouveau lancement.

Les commandes locales rotate-a/revoke-a/proof-stats existent pour les vérifications
de cycle de vie. Leur test détaillé sera guidé après le premier mini-projet réussi.
Un nouveau lancement crée un nouveau projet ; il ne reprend pas une session passée.
# Optional generated Node check/test mode

Use only a package whose native runner proof passed. Default startup still
exposes six editing tools. Adding `-NodeExecutable 'C:\Program Files\nodejs\node.exe'`
explicitly selects that installed executable and enables ten tools, including
fixed `check` and `test` tasks. The package verifies both bridge and helper hashes.
No Node installation is bundled or performed.

Generated source slots model.js/app.js/tests.js are the only files Node can read.
Create browser index.html/style.css separately; place pure application logic in
model.js and assertions in tests.js. `check` validates source syntax; `test` runs
tests.js. Node cannot create build files, install dependencies or start a network
server in this mode. This is an execution milestone, not the complete app alpha.
All existing stop/revoke and shared-session rules apply. Full end-to-end user
instructions will be issued only after the matching package is validated.
