# Diagnostic de connexion indépendant du projet

Ce test ne donne accès à aucun projet, fichier personnel ou processus. Son unique
outil renvoie des données fictives déterministes. Utiliser un serveur distinct
sur8010 ; ne pas remplacer le serveur de travail8009 par ce test.

Après Setup-ProjectAlpha, dans la racine du paquet :

```powershell
.\experiments\francestudent-probe\.venv\Scripts\python.exe .\experiments\project-alpha\diagnostic_host.py --port 8010 --allow-random-tunnel-host
```

Le serveur de diagnostic s'arrête après20minutes, ou Ctrl+C local. Cette limite
concerne ce banc de test isolé, pas le futur mode supervisé du projet. Le test
utilise uniquement les identifiants publics dummy déjà utilisés pour l'alpha.

Dans une autre fenêtre, lancer le contrôle rapide local :

```powershell
.\experiments\francestudent-probe\.venv\Scripts\python.exe .\experiments\project-alpha\diagnostic_client.py --url http://127.0.0.1:8010/mcp --quick --report .\diagnostic-local-quick.json
```

Sans --quick, la matrice réalise48échantillons (3répétitions, tailles128/1024/4096/
8192octets et délais0/1/3/10secondes). Compter environ3minutes. L'option --idle
ajoute des silences de15/45/75/100secondes dans la même session, environ4minutes.
Chaque rapport est créé sans écraser un fichier existant, limité à128KiB et64
échantillons. Choisir un autre nom pour une nouvelle mesure.

Pour Cloudflare ou ngrok, pointer un tunnel de diagnostic sur8010, puis reprendre
exactement le même client avec l'URL publique suivie de/mcp et un nom de rapport
différent. Ne pas publier d'URL avec jeton dans la query. Le client utilise déjà
Bearer et les deux headers dummy, refuse les URLs contenant des identifiants,
ne suit pas les redirections et n'écrit pas les contenus/URLs/auth dans le rapport.
Il ne prend pas de credential personnel. Ne tester que le serveur synthétique.

Chaque résultat vérifie texte complet, nombre d'octets, SHA256, empreinte du code
serveur, identité éphémère du run et marqueur/numéro de requête corrélables avec
les logs locaux. Les heures UTC et la durée monotone sont celles du client.
http_statuses recense les réponses HTTP observées pendant l'appel, y compris
d'éventuelles notifications/reprises SDK ; ce n'est pas une preuve1:1 de la seule
requête de l'outil. Les exceptions sont classées sans copier leur texte sensible.
Le rapport est une observation, pas une attribution automatique de responsabilité.

Seulement après les trois mesures comparables, configurer un connecteur FS
séparé vers ce serveur de diagnostic, avec les mêmes headers dummy. Premier
prompt : « Appelle réellement diagnostic_sample avec size_bytes=128 et
delay_seconds=0, puis résume le résultat en une phrase. » Puis tester un appel
8192/0 et128/10 séparément. Conserver heure, modèle, message d'erreur éventuel et
logs corrélés ; ne pas copier les paramètres personnels du compte ou un HAR brut.

Un HTTP200 local ne prouve pas que FS a reçu le corps. Le client de test prouve
sa propre réception, pas celle de FS. Le flux FS↔modèle reste une autre liaison ;
ses événements de fin/échec doivent être corrélés côté FS si nos mesures passent.

## Arrêt du lanceur alpha

Après stop ou expiration, output/alpha-…/last-stop.json conserve un compte rendu
opérateur d'au plus1KiB. revocation_confirmed et work_drained décrivent l'arrêt
du cœur ; diagnostics_drained/failed décrivent la sortie console, séparément.
Un diagnostic bloqué peut rester non livré sans bloquer les opérations ou
provoquer une erreur fatale du BufferedWriter à la fermeture. Ce fichier n'est
ni une signature, ni une preuve de résistance à une panne électrique, ni une
autorisation. Sa création peut échouer sur erreur disque : absence de rapport
ne signifie pas succès. Les callbacks sink personnalisés restent à la charge
de leurs tests ; le correctif vise le sink natif par défaut du lanceur.
