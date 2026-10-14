# Prochain test FS : agenda et budget dans une fenêtre Electron

Le paquet conserve les outils d’édition et les profils Node check/test déjà
validés. Luna crée l’application et ses tests. Après l’arrêt confirmé d’Optic,
tu ouvres une fenêtre Electron avec un lanceur fixe fourni par le paquet.
Luna ne peut pas modifier ce lanceur ni exécuter un main/preload Electron avec
les droits de ton compte. Pas encore d’installateur, npm ou build via MCP.
Les données de test doivent être fictives. Utiliser Luna : le parcours Sonnet
observé perdait les headers personnalisés.

## 1. Préparation

Extraire le nouveau paquet dans E:\Optic-Test-FS\test12. Dans PowerShell :

```powershell
cd 'E:\Optic-Test-FS\test12'
powershell -NoProfile -ExecutionPolicy Bypass -File .\experiments\project-alpha\scripts\Setup-ProjectAlpha.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File .\experiments\project-alpha\scripts\Start-ProjectAlpha.ps1 -AllowRandomTunnelHost -NodeExecutable 'C:\Program Files\nodejs\node.exe'
```

Conserver le nom alpha-… affiché. Un nouveau lancement crée un autre projet.
Un tunnel existant vers127.0.0.1:8009 peut être conservé si toujours actif ; sinon :

```powershell
cloudflared tunnel --url http://127.0.0.1:8009
```

Dans FS, URL du tunnel suivie de /mcp. Autorisation :
`Bearer optic-probe-dummy-owner-a`. Headers sécurisés :

```json
{"X-Optic-Probe":"optic-probe-dummy-header","X-Optic-Probe-Secret":"optic-probe-dummy-secret"}
```

## 2. Prompts dans l’ordre

Envoyer PROMPT_PLANNER_LOGIC.md, puis son résultat à Codex. Après tests réussis,
envoyer PROMPT_PLANNER_UI.md. Durée25minutes : project_info donne le temps restant.
Ne pas redémarrer pour prolonger un projet en cours. Si la durée est insuffisante,
conserver les sources et transmettre les logs ; aucune reprise implicite.

## 3. Arrêt et fenêtre

Après réussite des tests et relectures, taper `stop` dans la console Optic.
Attendre `ok:true`, `session_revoked:true` et `viewer_ready:true`. L’arrêt ne coupe
pas le réseau du PC. Ne pas lancer un nouveau serveur. Dans une autre fenêtre :

```powershell
cd 'E:\Optic-Test-FS\test12'
powershell -NoProfile -ExecutionPolicy Bypass -File .\experiments\project-alpha\scripts\Open-ProjectAlpha.ps1 -RunName 'alpha-NOM_AFFICHE' -ElectronExecutable 'C:\Users\timot\node_modules\electron\dist\electron.exe'
```

Remplacer alpha-NOM_AFFICHE par le vrai nom, sans modifier les sources après stop.
La fenêtre peut être fermée et rouverte avec la même commande : ses données sont
conservées dans output/alpha-…/electron-data. Les exports JSON sont dans
output/alpha-…/exports (8 par ouverture, 64Ko chacun). L’IA n’y a pas accès.

## 4. Vérifications manuelles

- Créer deux événements avec jour/heure/durée ; changer de semaine et vérifier
  leur placement, puis modifier/supprimer un événement.
- Créer des dépenses sur deux mois ; vérifier le total et le filtre par catégorie.
- Créer un abonnement mensuel et un annuel ; vérifier échéances et coût annuel.
- Fermer/réouvrir : toutes les données doivent rester présentes.
- Exporter JSON, modifier les données, réimporter cet export : retour à la sauvegarde.
- Importer un fichier invalide : refus sans perte des données en cours.

Rapporter les résultats réels et les erreurs. Node check est une vérification de
syntaxe, test exécute tests.js ; ni l’un ni l’autre ne prouve l’interface graphique.
