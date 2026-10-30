# Mégane Digital Cluster

Combiné numérique **expérimental** pour une Renault Mégane II Sedan / Classic 4 portes, 1.6 16V essence (2004).

**Phase 1 : application PC Rust + Dioxus avec télémétrie totalement simulée.** Le programme ne se connecte pas à la voiture et n'en remplace aucun instrument. [Lire l'avis de sécurité](docs/NOTICE_SECURITE.md).

## Fonctionnalités disponibles

- Compteur numérique de vitesse, compte-tours animé et affichage du rapport de boîte.
- Carburant, température, tension, trajet et kilométrage **fictif**.
- 10 témoins (moteur, huile, batterie, ABS, airbag, frein, ceinture, clignotants, phares).
- Variantes visuelles RS, Classique et Glacier.
- Simulateur interactif : accélérateur, frein, boîte 5, arrêt/démarrage moteur, pause, remise à zéro.
- Scénarios : conduite normale, démarrage à froid, défaut moteur, capteurs absents, perte de liaison.
- Architecture `TelemetrySource` lecture seule pour préparer une future source OBD-II sans modifier les composants UI.
- Dégradation contrôlée : `—` pour donnée absente/périmée, `?` pour témoin dont l'état est inconnu.

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
```

Pour lancer directement en plein écran (sans bordure), remplacer **uniquement** `fullscreen = false` par `fullscreen = true`, enregistrer et **redémarrer l'application**. À `false`, la fenêtre reste redimensionnable et comporte les contrôles natifs de fermeture/minimisation.

Pour un fichier alternatif, définir `CLUSTER_CONFIG` sur son chemin absolu. Sur une version Windows portable, conserver `config/cluster.toml` à côté de `megane-digital-cluster.exe` dans le sous-dossier `config`.

### Tester rapidement les scénarios

1. Modifier la valeur d'accélérateur ou de frein, et passer les rapports `−` / `+`.
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

À chaque push sur `main` et à chaque PR, GitHub Actions effectue les tests sous Windows, compile en release et fournit un artefact **`megane-digital-cluster-windows-<sha>`** (exécutable + configuration). Téléchargement depuis l'onglet **Actions** du dépôt.

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
