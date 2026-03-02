# PDF Eater

Outil de manipulation de PDF dans le navigateur. Aucune donnée ne quitte votre machine — tout est traité côté serveur sur `localhost`.

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

- **Backend** : Rust — [actix-web](https://actix.rs/) + [lopdf](https://github.com/J-F-Liu/lopdf) 0.38
- **Frontend** : HTML/CSS/JS vanilla, sans dépendance externe
- **Thèmes** : clair/sombre automatique (`prefers-color-scheme`) avec bascule manuelle
- **Mobile** : responsive, compatible iOS (safe-area-inset, tap targets 44 px)

## Structure du projet

```
pdf-eater/
├── build.rs                    # Versionnage des assets (hash dans le nom en release)
├── Cargo.toml
├── Dockerfile
├── .gitignore
├── src/
│   ├── main.rs                 # Serveur actix-web, routes, sécurité HTTP
│   ├── handlers/               # Validation des requêtes multipart
│   │   ├── mod.rs
│   │   ├── merge.rs
│   │   ├── extract.rs
│   │   ├── rotate.rs
│   │   ├── delete.rs
│   │   └── reorder.rs
│   └── pdf/                    # Logique métier PDF
│       ├── mod.rs
│       ├── error.rs            # PdfError (thiserror)
│       ├── utils.rs            # Helpers partagés : load_document, copy_objects, …
│       ├── merge.rs
│       ├── extract.rs
│       ├── rotate.rs
│       ├── delete.rs
│       └── reorder.rs
└── static/
    ├── index.html
    ├── style.css
    └── app.js
```

## Installation

### Prérequis

- [Rust](https://rustup.rs/) 1.70+
- [qpdf](https://qpdf.sourceforge.io/) — normalisation des PDFs avec xref en stream

```bash
# macOS
brew install qpdf

# Debian / Ubuntu
sudo apt install qpdf
```

### Dépendances `Cargo.toml`

```toml
[dependencies]
actix-web       = "4"
actix-files     = "0.6"
actix-multipart = "0.6"
tokio           = { version = "1", features = ["full"] }
lopdf           = "0.38"
futures-util    = "0.3"
log             = "0.4"
env_logger      = "0.11"
thiserror       = "1"
tempfile        = "3"
```

### Lancer en développement

```bash
cargo run
```

### Lancer en release

```bash
cargo build --release
./target/release/pdf-eater
```

Le serveur démarre sur `http://localhost:8080`. Pour activer les logs de débogage :

```bash
RUST_LOG=pdf_eater=debug ./target/release/pdf-eater
```

### Docker

```bash
docker build -t pdf-eater .
docker run -p 8080:8080 pdf-eater
```

## Versionnage des assets

En build release, `build.rs` calcule un hash FNV-1a 8 caractères du contenu de chaque asset et génère des fichiers renommés dans `static/` :

```
static/style.css  →  static/style.a3f2c891.css
static/app.js     →  static/app.b7d41e02.js
```

Le HTML servi référence automatiquement les fichiers versionnés. Les anciens fichiers hachés sont supprimés à chaque build. En développement (`cargo run`), les noms originaux sont utilisés sans modification.

## Sécurité

Chaque requête est validée avant traitement :

- **Content-Type** : `application/pdf` obligatoire
- **Magic bytes** : vérification de `%PDF-` en début de fichier
- **Taille** : max 1 Go par fichier, 20 fichiers par requête
- **Champs texte** : max 1 Ko (numéros de pages, angles, ordre)

En-têtes HTTP appliqués sur toutes les réponses HTML :

```
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
X-XSS-Protection: 1; mode=block
Referrer-Policy: no-referrer
Content-Security-Policy: default-src 'self'; script-src 'self'; style-src 'self';
```

## Syntaxe des plages de pages

Les onglets Extraire, Supprimer et Rotation acceptent la même syntaxe :

```
1          → page 1 uniquement
1, 3, 5    → pages 1, 3 et 5
2-5        → pages 2, 3, 4 et 5
1, 3-5, 8  → pages 1, 3, 4, 5 et 8
```

## Notes d'implémentation

**Compatibilité PDF** — certains PDFs valides (scanners, iOS, exporteurs d'image) utilisent des xref en object streams (PDF 1.5+) que lopdf ne sait pas parser directement. `load_document()` dans `utils.rs` tente d'abord lopdf, et en cas d'échec appelle `qpdf --object-streams=disable` pour convertir les xref binaires en xref ASCII avant de réessayer. Si qpdf est absent, l'erreur lopdf originale est retournée.

**Reconstruction du document** — toutes les opérations reconstruisent un document propre plutôt que de modifier le document source en place. Le mapping `old_id → new_id` est appliqué récursivement sur tous les objets (`Dictionary`, `Array`, `Stream`) via `remap_object()` pour que les références internes (fontes, images, annotations) restent valides. Les helpers partagés (`copy_objects`, `insert_pages_node`, `insert_catalog`, `set_parent`, `finalize`) sont centralisés dans `pdf/utils.rs`.

**Fichiers temporaires** — `Document::save()` (lopdf) écrit obligatoirement sur disque. Chaque sérialisation utilise un `tempfile::NamedTempFile` dont la suppression est garantie par le `Drop` automatique, y compris en cas de panique.

**Comptage des pages côté client** — l'onglet Réorganiser détecte le nombre de pages sans envoyer le fichier au serveur. Il lit deux tranches de 256 Ko (début et fin du fichier) et y cherche `/Count N` par expression régulière. Le plus grand `/Count` trouvé correspond au nœud `/Pages` racine.
