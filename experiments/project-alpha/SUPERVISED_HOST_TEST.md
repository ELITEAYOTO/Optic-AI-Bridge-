# Test FS — connexion supervisée et projet conservé

Cette alpha travaille uniquement dans son projet généré ou un run alpha conservé.
Node garde les profils check/test et ses trois fichiers autorisés. Pas encore de
npm install, serveur web, compilation Electron/Rust ou projet privé existant.

## 1. Mettre le paquet à jour

Extraire le paquet vérifié dans le même dossier que l'alpha à reprendre, en
remplaçant experiments/ et packaging/ ainsi que le manifeste fourni. Le ZIP ne
contient pas output/ : conserver ce dossier et ne pas déplacer le run existant,
car son journal est lié au chemin du projet. Pour un essai neuf, utiliser un
dossier séparé et omettre ResumeRun. COMMENCER_ICI.md est cette notice.

Dans PowerShell, depuis le dossier du paquet, exécuter au besoin :

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\experiments\project-alpha\scripts\Setup-ProjectAlpha.ps1
```

## 2. Lancer la connexion supervisée

Exemple pour le run déjà conservé dans test12 :

```powershell
cd 'E:\Optic-Test-FS\test12'
powershell -NoProfile -ExecutionPolicy Bypass -File .\experiments\project-alpha\scripts\Start-ProjectAlpha.ps1 -AllowRandomTunnelHost -NodeExecutable 'C:\Program Files\nodejs\node.exe' -ReadPageBytes 4096 -Supervised -ResumeRun 'alpha-zpre99l7'
```

Ne pas ajouter SessionMinutes à Supervised. La console doit annoncer
Operator-supervised until local stop. La durée de chaque autorisation interne
est courte et renouvelée localement, sans recréer les droits expirés/révoqués.
La connexion ne s'arrête plus au bout de50minutes dans ce mode.

Un essai neuf utilise la même commande sans ResumeRun. La console indique alors
le nouveau nom alpha-... à conserver pour toute reprise.

## 3. Tunnel et configuration FS

Dans un autre PowerShell :

```powershell
cloudflared tunnel --url http://127.0.0.1:8009
```

Configurer l'URL HTTPS affichée, suivie de /mcp. Renseigner à nouveau les champs
si l'éditeur FS les efface :

Autorisation : `Bearer optic-probe-dummy-owner-a`

Headers sécurisés JSON :

```json
{
  "X-Optic-Probe": "optic-probe-dummy-header",
  "X-Optic-Probe-Secret": "optic-probe-dummy-secret"
}
```

Ce sont les valeurs publiques de cette alpha. Tous les chats utilisant cette
connexion partagent son projet et ses permissions. Aucun identifiant de chat
fiable n'est fourni par FS ; ne pas présenter ce mode comme une isolation par chat.

## 4. Premier prompt, sans modification

> Utilise réellement Optic : appelle optic_project_info, puis optic_fs_list avec
> path="src", puis optic_task_list. Affiche le mode de connexion, remaining_seconds,
> les capacités, les limites et les résultats des listes. Ne modifie aucun fichier
> et ne lance aucune tâche pour cette étape. En cas d'erreur, affiche le résultat réel.

Attendu : mode=supervised, remaining_seconds=null, read_page_bytes=4096,
13outils dans le mode Node supervisé (14avec l'option locale AllowDirectories), fichiers du run conservé et liste de tâches initiale
vide. Null signifie que la connexion n'a pas d'échéance fixe ; ce n'est pas une
autorisation illimitée ni une indication que le noyau ne contrôle plus les délais.

## 5. Reconnexion et travail

Après validation de l'étape4, un nouveau chat sur la même connexion peut inspecter
le même projet puis poursuivre son objectif. Les fichiers sont la référence,
pas la mémoire d'un ancien chat. Les lectures gardent version/offset/eof ; si le
résultat côté hôte est tronqué, demander une page plus petite, par exemple1024.
Ne jamais relancer aveuglément une écriture ou une tâche au résultat incertain.

optic_task_list récupère les identifiants déjà reçus par le bridge, y compris
après une réponse perdue côté chat. Une liste vide ne prouve pas qu'un démarrage
incertain côté noyau n'a pas eu lieu. Lire résultat/sorties puis utiliser
optic_task_forget pour libérer un résultat terminé. Les tâches actives ou dont
la terminaison est incertaine ne peuvent pas être oubliées.

Les tâches restent limitées en durée, mémoire, sortie et nombre de processus.
512requêtes HTTP par fenêtre fixe60s ;16démarrages par fenêtre fixe60s ;16tâches
retenues. Les fenêtres fixes peuvent autoriser un pic autour d'une frontière ;
les limites de concurrence restent également actives. Les refus de débit connus
ne sont pas des démarrages incertains. retry_after_seconds indique le délai connu.
La création source est bornée à128emplacements de fichiers de8192octets chacun.
Les réservations échouées restent comptées prudemment jusqu'à une reprise locale.
Les journaux de récupération et sorties gardent leurs limites natives. Les preuves
console restent best-effort et bornées, pas un historique infini des requêtes.

## 6. Arrêt et reprise

L'outil optic_diagnostics affiche les8derniers résultats sûrs, leur réception et
fin UTC en millisecondes, leur durée, les issues incertaines et le travail retenu.
L'anneau est limité à32événements en mémoire et réinitialisé au redémarrage ou au
changement de credential. Il ne contient ni arguments, ni sources, ni jetons, ni
exceptions brutes. Les sorties d'un programme se lisent avec optic_task_read.
À la reprise, un résumé filtré du précédent rapport d'arrêt est disponible ;
previous_shutdown_unconfirmed signifie absence de preuve d'arrêt propre, pas
preuve d'un crash. Aucun accès aux journaux internes FS n'est revendiqué.

Dans la console du serveur : `stop`. L'arrêt demeure disponible après épuisement
du budget des commandes de contrôle. EOF ferme aussi la connexion ; revoke-a
arrête le mode supervisé et révoque le noyau. Rotate-a conserve le contrôle local
mais invalide l'accès aux anciennes tâches par l'ancien epoch de credential.
Une reprise explicite renouvelle l'historique local si des anciens emplacements
ne sont plus accessibles après rotation.

Le rapport output/alpha-.../last-stop.json distingue la révocation/drain confirmés
et la livraison des diagnostics. Un crash ne doit pas laisser un ancien rapport
comme preuve de succès. Pour reprendre après arrêt, relancer avec le même ResumeRun.
Une vraie panne/expiration du noyau ferme l'adaptateur ; il ne recrée pas les droits.
Les coupures de génération signalées à FS sont un incident distinct ; ce mode
n'affirme pas corriger le flux FS-vers-modèle.
