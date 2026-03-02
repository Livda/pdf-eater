FROM rust:1-alpine AS builder

WORKDIR /app

# ── 1. Cache des dépendances ──────────────────────────────────────────────────
# On compile les deps en isolation avec des stubs minimalistes.
# Le build.rs stub exporte les vars sans lire static/ (qui n'existe pas encore).
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src static && \
    printf 'fn main() {\n  println!("cargo:rustc-env=STYLE_CSS_FILE=style.css");\n  println!("cargo:rustc-env=APP_JS_FILE=app.js");\n}\n' > build.rs && \
    echo 'fn main() {}' > src/main.rs && \
    cargo build --release 2>/dev/null || true && \
    rm -f build.rs src/main.rs

# ── 2. Build réel ─────────────────────────────────────────────────────────────
COPY build.rs ./
COPY static ./static
COPY src ./src

# build.rs génère static/style.<hash>.css et static/app.<hash>.js,
# puis exporte STYLE_CSS_FILE et APP_JS_FILE via cargo:rustc-env.
RUN cargo build --release

# ── Image finale ──────────────────────────────────────────────────────────────
FROM alpine:3.23 AS prod

RUN apk add --no-cache qpdf
WORKDIR /app

# IMPORTANT : static/ vient du BUILDER, pas de l'hôte.
# Le builder contient les fichiers hachés générés par build.rs.
COPY --from=builder /app/static ./static
COPY --from=builder /app/target/release/pdf-eater .

EXPOSE 8080
CMD ["./pdf-eater"]
