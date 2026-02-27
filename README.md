# PDF Eater

Outil de manipulation de PDF en ligne de commande locale. Aucune donnée ne quitte votre machine — tout est traité côté serveur sur `localhost`.

## Fonctionnalités

| Onglet | Route | Description |
|---|---|---|
| Fusionner | `POST /merge` | Fusionne plusieurs PDF en un seul, dans l'ordre choisi |
| Extraire | `POST /extract` | Extrait un sous-ensemble de pages (ex : `1, 3, 5-8`) |
| Rotation | `POST /rotate` | Applique une rotation 90°/180°/270° à des pages spécifiques |
| Supprimer | `POST /delete` | Supprime des pages d'un PDF |
| Réorganiser | `POST /reorder` | Réordonne les pages par glisser-déposer |

Toutes les opérations affichent une barre de progression pendant l'upload.

## Stack

- **Backend** : Rust — [actix-web](https://actix.rs/) + [lopdf](https://github.com/J-F-Liu/lopdf)
- **Frontend** : HTML/CSS/JS vanilla, sans dépendance externe
- **Thèmes** : clair/sombre automatique (`prefers-color-scheme`) avec bascule manuelle
- **Mobile** : responsive, compatible iOS (safe-area-inset, tap targets 44 px)

## Structure du projet

```
pdf-eater/
├── src/
│   ├── main.rs                 # Serveur actix-web, routes, sécurité HTTP
│   ├── handlers/
│   │   ├── mod.rs
│   │   ├── merge.rs
│   │   ├── extract.rs
│   │   ├── rotate.rs
│   │   ├── delete.rs
│   │   └── reorder.rs
│   └── pdf/
│       ├── mod.rs
│       ├── utils.rs            # remap_object, find_pages_root, save_to_bytes
│       ├── merge.rs
│       ├── extract.rs          # parse_page_ranges
│       ├── rotate.rs
│       ├── delete.rs
│       └── reorder.rs
├── static/
│   ├── index.html
│   ├── style.css
│   └── app.js
└── Cargo.toml
```

## Installation

### Prérequis

- [Rust](https://rustup.rs/) 1.70+

### Dépendances `Cargo.toml`

```toml
[dependencies]
actix-web      = "4"
actix-files    = "0.6"
actix-multipart = "0.6"
tokio          = { version = "1", features = ["full"] }
lopdf          = "0.32"
futures-util   = "0.3"
log            = "0.4"
env_logger     = "0.11"
```

### Lancer le serveur

```bash
cargo run --release
```

Le serveur démarre sur `http://localhost:8080`. Pour activer les logs :

```bash
RUST_LOG=info cargo run --release
```

## Sécurité

Chaque requête est validée avant traitement :

- **Content-Type** : `application/pdf` obligatoire
- **Magic bytes** : vérification de `%PDF` en début de fichier
- **Taille** : max 1 Go par fichier, 20 fichiers par requête
- **Champs texte** : max 1 Ko (numéros de pages, angles, ordre)

En-têtes HTTP appliqués sur toutes les réponses :

```
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
X-XSS-Protection: 1; mode=block
Referrer-Policy: no-referrer
Content-Security-Policy: default-src 'self'; script-src 'self'; style-src 'self';
```

Le serveur écoute sur `127.0.0.1:8080` (loopback uniquement — pas exposé sur le réseau local).

## Syntaxe des plages de pages

Les onglets Extraire, Supprimer et Rotation acceptent la même syntaxe :

```
1          → page 1 uniquement
1, 3, 5    → pages 1, 3 et 5
2-5        → pages 2, 3, 4 et 5
1, 3-5, 8  → pages 1, 3, 4, 5 et 8
```

## Notes d'implémentation

**Manipulation PDF avec lopdf** — `Document::save()` ne peut écrire que sur disque. Chaque opération utilise donc un fichier temporaire dans `std::env::temp_dir()`, protégé par un guard `Drop` qui garantit sa suppression même en cas de panique.

**Reconstruction du document** — toutes les opérations (extraction, suppression, réorganisation) reconstruisent un document propre plutôt que de modifier le document source. Le mapping `old_id → new_id` est appliqué récursivement sur tous les objets (`Dictionary`, `Array`, `Stream`) via `remap_object()` pour que toutes les références internes (fonts, images, annotations) restent valides.

**Comptage des pages côté client** — l'onglet Réorganiser détecte le nombre de pages sans envoyer le fichier au serveur. Il lit deux tranches de 256 Ko (début et fin du fichier) et y cherche `/Count N` par expression régulière. Le plus grand `/Count` trouvé correspond au nœud `/Pages` racine.
