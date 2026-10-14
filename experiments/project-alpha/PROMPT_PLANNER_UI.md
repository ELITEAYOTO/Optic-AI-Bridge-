Crée l’interface française « Agenda & Budget » à partir de la logique model.js
et des tests.js existants, en utilisant réellement les outils Optic.

Commence par project_info. S’il reste moins de480secondes, arrête-toi avant
toute modification. Lis intégralement model.js, tests.js et app.js en suivant
next_offset jusqu’à eof et exige une version stable. Vérifie index.html et
style.css avec fs_list : lire intégralement s’ils existent. Préconditions exactes
pour remplacer, expected_kind=absent pour créer. Aucun nouveau ticket/projet.

Modifie uniquement src/index.html, src/style.css et src/app.js ; ne change pas
la logique ni ses tests. Chaque fichier sous7500octets UTF8. HTML charge dans
l’ordre model.js et app.js via des scripts externes locaux. Aucun script/style
inline, onclick, import CSS, réseau, bibliothèque, fichier additionnel ou API
Node/Electron. Utilise Planner pour la logique ; pas de require dans app.js.

Interface responsive, soignée, claire et utilisable au clavier :
- Agenda hebdomadaire avec semaine précédente/suivante, événements jour/heure/
  durée, formulaire d’ajout, modification et suppression.
- Dépenses : sélection du mois, catégories, ajout/modification/suppression et total.
- Abonnements mensuels/annuels : gestion, prochaine échéance et coût annuel.
- Sauvegarde localeStorage après chaque changement et restauration au démarrage.
  Une sauvegarde invalide doit afficher une erreur sans être silencieusement
  écrasée. Import JSON via input type=file puis FileReader (taille<=64Ko), avec
  validation Planner et confirmation avant remplacement. Échec sans perte.
  Export avec Blob de type application/json et lien download (pas d’IPC ni fs).
- États vides, validations explicites, montants en euros, dates lisibles.
  Affiche les libellés saisis avec textContent, pas du HTML interprété.
  Aucun exemple personnel : uniquement données fictives si tu en ajoutes.

Relis les cinq fichiers intégralement, versions stables, EOF atteint pour chacun.
Lance check puis test une fois chacun, suis les job_id jusqu’à état terminal,
lis les sorties. Arrête-toi sur erreur ou résultat inconnu sans relancer une
écriture/un processus. Résume versions/tailles, états/codes/sorties et ce qui
reste à tester. La fenêtre Electron sera ouverte localement par l’utilisateur
après stop : ne prétends pas l’avoir lancée ni testée.
