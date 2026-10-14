Utilise réellement Optic pour préparer la logique d’une application française
« Agenda & Budget ». Projet généré seulement, aucune dépendance externe.

Commence par optic_project_info et optic_fs_list(path="src"). S’il reste moins
de600secondes, arrête-toi avant toute écriture. Lis intégralement README.txt,
model.js et tests.js, par blocs1024octets en suivant next_offset jusqu’à eof.
Exige une version stable par fichier. Remplacements avec expected_kind=content
et version observée ; créations seulement expected_kind=absent.

Modifie uniquement src/model.js et src/tests.js. model.js est une bibliothèque
pure utilisable par require('./model.js') dans Node et par globalThis.Planner
dans le navigateur. Aucun DOM, fichier, réseau ou localStorage dans model.js.
Chaque fichier doit rester sous7500octets UTF8.

Prépare ces fonctions, avec une API claire documentée dans model.js :
1. Événements : créer/modifier/supprimer ; titre, date YYYY-MM-DD, heure HH:mm,
   durée1–1440minutes. Filtrer une semaine du lundi au dimanche. Dates réelles,
   titres non vides ; les dates/heures représentent l’heure locale choisie,
   sans conversion implicite de fuseau dans la logique.
2. Dépenses : libellé, date, catégorie, montant positif en centimes entiers ;
   créer/modifier/supprimer, filtrer mois/catégorie et total exact en centimes.
3. Abonnements : libellé, montant en centimes, date de début, fréquence mensuelle
   ou annuelle. Calculer coût annuel, équivalent mensuel et prochaine échéance
   à partir d’une date donnée. Si le jour n’existe pas dans un mois, ramener au
   dernier jour en conservant l’ancre initiale pour les mois suivants.
4. Sauvegarde : état JSON version1 contenant events, expenses et subscriptions.
   Export/import validés, maximum64Ko et100éléments par liste. IDs uniques,
   champs/types/dates/montants vérifiés. L’import invalide ne modifie pas l’état
   fourni. Aucune clé supplémentaire de privilège ni valeur non finie.

tests.js utilise node:assert/strict et de vraies assertions pour événements,
semaines aux limites de mois, totaux par mois, filtres et suppressions,
abonnements mensuels/annuels, fin de mois et année bissextile, import/export,
doublons/JSON invalide/types erronés/montants négatifs. Au succès, afficher
PLANNER_TESTS_PASS. Ne te contente pas de tests qui n’appellent pas la logique.

Relis les deux fichiers intégralement avec version stable. Lance le profil
check puis test, chacun une seule fois ; conserve chaque job_id, consulte
task_result jusqu’à état terminal et lis stdout/stderr par pages si nécessaire.
Si une écriture/un lancement expire sans confirmation, arrête-toi sans retenter.
Si un test échoue, affiche l’erreur réelle et arrête-toi pour inspection.
Résumé : API choisie, versions, tailles, job_id, états/codes et sorties.
Ne crée pas encore l’interface et ne prétends pas avoir exécuté Electron.
