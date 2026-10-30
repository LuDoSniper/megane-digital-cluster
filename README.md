# Mégane Digital Cluster

Combiné numérique **expérimental** pour une Renault Mégane II Sedan / Classic 4 portes, 1.6 16V essence (2004).

**Phase 1 : application PC Rust + Dioxus avec télémétrie totalement simulée.** Le programme ne se connecte pas à la voiture et n'en remplace aucun instrument. [Lire l'avis de sécurité](docs/NOTICE_SECURITE.md).

## V0.2 — Combiné routier et laboratoire séparés

L'application démarre sur **le véritable aperçu de combiné** : cadrans, vitesse centrale,
rapport, suggestion de changement de rapport, témoins dessinés en SVG, jauges et assistance.
Le **laboratoire** (commandes de simulation) est masqué par défaut et s'ouvre avec **F2** ou
« Ouvrir le laboratoire » : la vue du combiné n'embarque aucune commande du simulateur.
Les deux vues partagent un modèle de télémétrie indépendante des futurs lecteurs OBD.

- Simulation 5 vitesses + neutre + marche arrière **R** (enclenchable sous 2 km/h).
- Clignotants animés (~90/min) et feux de détresse, croisement vert et plein phare bleu.
- Témoin moteur, huile, batterie, ABS, airbag, frein, ceinture ; valeurs inconnues explicites.
- Suggestion de rapport ↑/↓ selon régime, purement indicative : **ne pas l'utiliser pour conduire**.
- Régulateur et limiteur fictifs avec sélection, SET, +/- 1 km/h, annulation et reprise.
  Régulateur coupé par frein/embrayage simulé lors du changement de rapport ; limiteur
  avec « kickdown » simulé au-delà de 95 % de pédale. Aucun ordre envoyé au véhicule.
- Trois thèmes (RS, Classique, Glacier), panne capteurs, perte de données, démarrage à froid.

### Clavier (actif partout sauf dans un champ de saisie)

| Touche | Fonction |
|---|---|
| ↑ / ↓ | Accélérateur +5 / −5 % (maintien possible) |
| → / ← | Frein +5 / −5 % (maintien possible) |
| Pavé num. 1 à 5 (ou ligne des chiffres) | Sélection du rapport |
| 0 | Point mort N |
| R ou « . » du pavé | Marche arrière (à l'arrêt) |
| Q / E / H | Clignotant gauche / droit / feux de détresse |
| B / F | Feux de croisement / pleins phares |
| C / V | Mode régulateur / limiteur (réappuyer : OFF) |
| Entrée | SET (au moins 30 km/h) |
| + / − | Ajuster la consigne d'assistance |
| X / Z | Annuler / reprendre l'assistance |
| Espace | Pause simulation |
| F2 / Échap | Ouvrir/fermer le laboratoire |

La simulation volontairement approximative **ne représente pas le comportement réel** de
la Mégane II (lois de pédale, 5 rapports, couple, freinage, etc.). La marche arrière
n'est pas une commande électrique à envoyer au véhicule.

## Démarrage sur PC

**Pré-requis :** [Rust](https://rustup.rs/) (toolchain stable), Windows 10/11 avec WebView2 (généralement présent) ou Linux avec les bibliothèques WebKitGTK requises par Dioxus.

Depuis la racine du dépôt :

```powershell
cargo run -p cluster-app
```

L'application démarre **en mode fenêtré** avec une taille de 1360 × 840. Pour un build optimisé :

```powershell
cargo run --release -p cluster-app
```

### Configuration de la fenêtre

Fichier `config/cluster.toml` :

```toml
[window]
fullscreen = false  # défaut : fenêtré
width = 1360
height = 840

[interface]
theme = "rs"       # rs, classic ou ice
tick_ms = 50
show_lab_on_start = false # option laboratoire, distincte du mode plein écran
```

Pour lancer directement en plein écran (sans bordure), remplacer **uniquement** `fullscreen = false` par `fullscreen = true`, enregistrer et **redémarrer l'application**. À `false`, la fenêtre reste redimensionnable et comporte les contrôles natifs de fermeture/minimisation.

Pour un fichier alternatif, définir `CLUSTER_CONFIG` sur son chemin absolu. Sur une version Windows portable, conserver `config/cluster.toml` à côté de `megane-digital-cluster.exe` dans le sous-dossier `config`.

### Tester rapidement les scénarios

1. Ouvrir le laboratoire (`F2`) ou utiliser le clavier pour piloter le simulateur.
2. Activer un témoin pour vérifier sa couleur ; il s'agit de faux voyants.
3. Choisir « Capteurs indisponibles » : vitesse, régime et température affichent `—`.
4. Choisir « Connexion perdue » : valeurs périmées masquées et témoins marqués `?`.
5. Tester les trois thèmes et la remise à zéro du compteur de trajet.

## Tests / compilation continue

```powershell
cargo fmt --all
cargo test --workspace
cargo build --release -p cluster-app
```

À chaque push sur `main` et à chaque PR, GitHub Actions utilise **Swatinem/rust-cache** pour garder le registre Cargo et les dépendances compilées, fait les tests **en release** pour partager les builds avec l’exécutable, annule les runs obsolètes sur la même branche et compile en release et fournit un artefact **`megane-digital-cluster-windows-<sha>`** (exécutable + configuration). Téléchargement depuis l'onglet **Actions** du dépôt.

## Structure

```text
crates/cluster-core/       modèle des signaux et contrat télémétrie
crates/cluster-simulator/  simulation déterministe + tests
crates/cluster-app/        Dioxus desktop et CSS
config/cluster.toml        options démarrage fenêtre + interface
docs/                      architecture et consignes de sécurité
.github/workflows/ci.yml  format, tests et EXE Windows portable
```

[Architecture et prochaines phases](docs/ARCHITECTURE.md).

> **Attention :** la vitesse, les voyants et le kilométrage ne sont pas des informations réelles. Ne pas utiliser cette application pour conduire ou en remplacement du compteur d'origine.

### Références pour l'iconographie (prototype, sans homologation)

Les couleurs fonctionnelles du prototype s'appuient sur les conventions de
[ISO 2575:2021](https://www.iso.org/standard/68409.html) (vert clignotants/feux de croisement,
bleu feux de route, ambre défaillances, rouge danger). Les **formes SVG sont des
approximations originales**, pas une reproduction officielle du catalogue ISO ni
une validation d'homologation. Certaines fonctions dépendent de l'équipement réel
et doivent être vérifiées sur la voiture. La logique du régulateur/limiteur simulé
est guidée par la [documentation technique de la Mégane II](https://manualzz.com/doc/53328114/renault-megane-ii-manuel-utilisateur),
**sans aucune prétention de représenter fidèlement les calculateurs Renault**.

## Architecture des données et conduite assistée (v0.3)

L'**affichage** reçoit un `VehicleSnapshot` (vitesse, régime, état des voyants, consignes et qualité des mesures). Il n'effectue aucune action sur la voiture. Le **simulateur** génère ces données et y applique toute la logique de régulation, limitation, inertie, pente et changement de rapport. Plus tard, un **fournisseur OBD-II en lecture seule** pourra remplacer la source du simulateur, mais ne devra pas appeler ses algorithmes. Voir [l'architecture détaillée](docs/ARCHITECTURE.md).

- Régulateur simulé : `SET` à partir de 30 km/h, maintien de la vitesse sans accélérateur, veille au frein, à l'embrayage ou au changement de rapport, puis `RESUME`.
- Limiteur simulé : plafonnement **effectif** de la demande moteur, sans freinage automatique ; dépassement volontaire (pédale ≥ 98 %) et avertissement en descente.
- Le laboratoire (`F2`) montre la différence entre la **pédale demandée** et le **gaz réellement commandé** par les règles simulées. On peut également modifier l'embrayage (`J`) et la pente.
- Ces règles s'inspirent de la documentation Renault X84 et **ne prétendent pas être une implémentation légale/certifiée ni identique à tous les niveaux d'équipement de 2004**. Aucun raccordement à un calculateur réel.


## V0.4 — Commandes physiques et interfaces

- Maintenir **↑** pour appliquer progressivement l'accélérateur (0 → 100 % en ~0,3 s), puis relâcher : retour automatique à zéro (~0,25 s). Maintenir **↓** pour le frein, avec le même retour automatique. Les curseurs du laboratoire restent des commandes manuelles indépendantes.
- **←/→** ajustent la consigne du régulateur/limiteur ; pavé numérique 1–5, 0, R inchangés ; libération automatique des pédales lors de la perte du focus de la fenêtre.
- Choisir en laboratoire l'une des dispositions **RS Digital (original, par défaut)**, **Grand Touring** ou **Essentiel**. Les coloris RS/Classique/Glacier restent indépendants de la disposition. Config : `layout = "original"`.
- Géométrie des arcs de compteurs fixe ; seule la longueur visible est animée via le `stroke-dashoffset` SVG, corrigeant les déformations observées lors des variations rapides.
- Ouvrants d'une **Sedan 4 portes** (4 portes, coffre, capot) et 4 capteurs TPMS virtuels indépendants, avec scénarios de dégonflage et panne. Les alertes ne sont pas dérivées côté affichage.
- Dynamique longitudinale du profil K4M + **boîte JH3 142 provisoire**, couple moteur interpolé, vrais rapports/démultiplication, dimensions pneus, résistances aérodynamiques/de roulement, pente et inertie. **Il reste des hypothèses importantes** : [références et limites](docs/VEHICLE_SOURCES.md). Ne pas traiter les valeurs comme exactes pour le véhicule réel.
