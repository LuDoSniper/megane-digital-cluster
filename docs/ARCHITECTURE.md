# Architecture : trois responsabilités distinctes

```text
  [SIMULATEUR]                                [OBD-II futur · lecture seule]
  conduite / règles / moteur / pentes          lecture / décodage / fraîcheur
           |                                         |
           +------------- TelemetrySource -----------+
                                |
                        VehicleSnapshot
          (valeurs + qualité + états déjà déterminés)
                                |
                        [DISPLAY DIOXUS]
       composition / SVG / étiquettes / couleurs uniquement
```

## 1. Générateur de données : `cluster-simulator`

- La **seule** unité autorisée à transformer l'entrée de l'utilisateur en physique automobile virtuelle.
- Contrôle la pédale virtuelle, l'embrayage, la boîte, les pentes, les feux, le régulateur, le limiteur, les recommandations de rapport et le compteur kilométrique fictif.
- Applique le régulateur/limiteur à la **demande moteur simulée** : jamais à un véhicule.
- Produit un `VehicleSnapshot` avec des `Reading::simulated(...)`, et rend les champs explicitement inconnus / périmés lors des scénarios de panne.

## 2. Contrat de données brutes normalisées : `cluster-core`

- `VehicleSnapshot` et `Reading<T>` sont des DTO sans simulateur, moteur physique, ni code UI.
- Chaque signal est accompagné de sa qualité (`Simulated`, `Measured`, `Unknown`, `Stale`). Une mesure absente n'est pas déduite ou inventée.
- Boîte, état moteur, assistance et conseil de changement de rapport peuvent eux aussi être inconnus.
- `TelemetrySource` expose `snapshot()` uniquement ; pas de méthode d'écriture sur les calculateurs.
- Un adaptateur OBD-II futur implémentera ce trait **sans jamais utiliser les règles du simulateur**. Il renseignera `Unknown` quand un signal (rapport, témoin, kilométrage...) n'est pas disponible.
- Le nom « données brutes » désigne ici les données *normalisées par le fournisseur*, pas les trames CAN sans décodage.

## 3. Affichage : `cluster-app/src/display.rs`

- Ne connaît ni `Simulator`, ni `Command`, ni la dynamique moteur.
- Reçoit un `VehicleSnapshot`, puis se limite au formatage visuel et au placement des états fournis.
- N'active pas de voyant, n'applique aucun frein, n'infère pas un rapport, ne fabrique pas une recommandation.
- Les clignotants sont générés par la source avec leur phase d'allumage : l'affichage se contente de dessiner `LampState::On` ou `Off`.
- Le kilométrage simulé est indiqué **fictif**. L'OBD générique ne fournit pas de garantie de reprise du kilométrage légal.

## Modèle de régulateur / limiteur en simulation

Approche d'après le MR Renault Mégane X84 (versions équipées ; comportement approximé, pas un firmware Renault ni une règle légale universelle) :

- `OFF` / `RÉGULATEUR` / `LIMITEUR` : l'interrupteur arme un mode, sans activation immédiate ; `SET` engage une consigne (30–200 km/h, moteur en marche, boîte en marche avant, frein et embrayage relâchés).
- Régulateur : commande une demande moteur calculée pour maintenir la vitesse sur plat et en côte, sans déplacer la pédale de l'utilisateur. Frein, embrayage, changement de rapport, moteur arrêté ou défaut le mettent en veille ; `RESUME` restaure la consigne après retrait de la cause.
- Limiteur : l'utilisateur reste maître de la pédale, dont la **demande effective** est plafonnée. Freinage manuel n'annule pas le limiteur. Un appui ≥ 98 % sert de démonstration du point dur, et laisse temporairement dépasser la consigne.
- Aucun système ne freine automatiquement dans cette simulation d'une génération de Mégane II. Une descente suffit donc à dépasser la vitesse programmée ; `unable_to_hold` est signalé par la source.
- Clutch, frein, pente et demande moteur effective visibles dans le laboratoire. Les tests automatisés vérifient maintien de vitesse, pause frein/embrayage, maintien du limiteur, dépassement volontaire et pente.

Sources :
- Renault Mégane X84, documentation technique MR (injection essence), chapitre 17B « Régulateur–limiteur de vitesse » : https://www.manualslib.fr/manual/421795/Renault-Megane-X84-2004.html?page=730
- Conditions d'activation, caractéristiques du point dur : https://www.technirevue.com/revue_technique/revue-technique-renault-megane-2.pdf

## Étapes suivantes

Phase 2 : cartographie réelle et **lecture seule** via un adaptateur fiable ; aucune commande CAN, aucun contrôle de papillon ou action moteur. La vitesse et le régime peuvent être disponibles via OBD-II, mais témoins, état régulateur/limiteur, rapport et kilométrage peuvent être absents. Les capteurs critiques ne doivent jamais être supposés normaux.

Phase 3 : écran provisoire qui ne cache pas le combiné Renault d'origine. Tester alimentation, visibilité et pertes de télémétrie.

Phase 4 : étude du kilométrage, des témoins de sécurité, de l'électronique et de l'homologation avant tout projet de remplacement définitif.
