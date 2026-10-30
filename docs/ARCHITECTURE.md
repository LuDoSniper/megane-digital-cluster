# Architecture technique — Phase 1

## Séparation en trois crates

```text
cluster-simulator -> cluster-core <- cluster-app
                  (contrats de télémétrie)
```

- `cluster-core` : données typées, `Reading<T>`, qualité, voyants et trait **lecture seule** `TelemetrySource`.
- `cluster-simulator` : moteur déterministe, commandes de simulation, cas de panne, cohérence approximative vitesse/régime/rapport. Aucun accès véhicule.
- `cluster-app` : application Windows/Linux Dioxus desktop, CSS intégré à l'exécutable, lecteur TOML et contrôles d'essai.

## Qualité des signaux

- `Simulated`: seule qualité autorisée à être affichée comme une valeur valide dans cette phase.
- `Unknown`: valeur absente, rendue `—` sur les instruments ; témoins rendus `?`.
- `Stale`: dernière valeur connue périmée, interdite d'affichage comme donnée valide.

Le futur adaptateur OBD-II devra introduire une qualité `Live` mesurée, une horloge de fraîcheur et des tests de déconnexion. La simulation et la qualité `Simulated` ne seront jamais présentées comme des valeurs mesurées dans une voiture.

## Démarrage fenêtré / plein écran

`AppConfig::load()` cherche successivement :

1. Variable d'environnement `CLUSTER_CONFIG` si elle est définie ;
2. `./config/cluster.toml` ;
3. `config/cluster.toml` à côté de l'exécutable ;
4. `cluster.toml` à côté de l'exécutable.

Si aucun fichier n'existe, les paramètres intégrés démarrent en fenêtre 1360×840. En cas d'erreur TOML, une erreur est émise dans la console et on revient aux valeurs **fenêtrées**. Le plein écran sans bordure n'est utilisé que si `fullscreen = true`.

## Rendement

Cadence 50 ms configurable. Un tick récupère une seule variation de temps monotone et met à jour le simulateur, puis la vue. L'intégration limite les écarts de durée à 200 ms pour éviter les sauts de la simulation après sommeil du PC.

## Feuille de route après le prototype

- Phase 2 : adaptateur OBD-II lecture seule, identification précise des PID disponibles sur Mégane II Sedan 2004. Réseau CAN : diagnostic passif seulement.
- Phase 3 : écran auxiliaire, tests d'alimentation, redémarrage, perte de signal, lisibilité, température.
- Phase 4 : témoins Renault, odomètre certifiable, interopérabilité électronique, validation de conformité et assurances avant toute suppression du combiné d'origine.
