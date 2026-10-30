# Mégane Digital Cluster

Prototype expérimental de combiné numérique pour Mégane II Sedan (2004).

- Simulateur logiciel uniquement ; aucune connexion à la voiture en phase 1.
- **Fenêtré par défaut** ; le plein écran ne s'active au démarrage que si `config/cluster.toml` contient `fullscreen = true`.
- Rust + Dioxus desktop, source de télémétrie séparée de l'affichage.
- Ne pas masquer ou supprimer les instruments d'origine.
