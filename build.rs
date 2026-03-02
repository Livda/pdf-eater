// build.rs — Génère les noms de fichiers assets versionnés pour le cache-busting.
//
// En release : copie style.css → static/style.{hash}.css
//              copie app.js   → static/app.{hash}.js
//              exporte STYLE_CSS_FILE et APP_JS_FILE via cargo:rustc-env
//
// En debug   : utilise les noms d'origine (style.css / app.js) pour un
//              rechargement rapide sans copie inutile.

use std::fs;

/// Hash FNV-1a 64-bit, déterministe, sans dépendance externe.
/// Retourne les 8 premiers caractères hex (32 bits d'entropie — largement
/// suffisant pour du cache-busting).
fn fnv1a_short(data: &[u8]) -> String {
    let mut hash: u64 = 14_695_981_039_346_656_037;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    format!("{:016x}", hash)[..8].to_string()
}

fn main() {
    // Déclenche la recompilation si ces fichiers changent.
    println!("cargo:rerun-if-changed=static/style.css");
    println!("cargo:rerun-if-changed=static/app.js");

    let profile = std::env::var("PROFILE").unwrap_or_default();
    let is_release = profile == "release";

    if is_release {
        // ── Mode release : hash + copie des fichiers versionnés ──────────
        let css_data = fs::read("static/style.css")
            .expect("build.rs : impossible de lire static/style.css");
        let js_data = fs::read("static/app.js")
            .expect("build.rs : impossible de lire static/app.js");

        let css_hash = fnv1a_short(&css_data);
        let js_hash  = fnv1a_short(&js_data);

        let css_filename = format!("style.{}.css", css_hash);
        let js_filename  = format!("app.{}.js",    js_hash);

        // Copie les fichiers hachés dans static/ pour qu'actix-files
        // puisse les servir à l'exécution.
        fs::write(format!("static/{}", css_filename), &css_data)
            .unwrap_or_else(|e| panic!("build.rs : écriture {} échouée : {}", css_filename, e));
        fs::write(format!("static/{}", js_filename), &js_data)
            .unwrap_or_else(|e| panic!("build.rs : écriture {} échouée : {}", js_filename, e));

        println!("cargo:rustc-env=STYLE_CSS_FILE={}", css_filename);
        println!("cargo:rustc-env=APP_JS_FILE={}", js_filename);

        eprintln!("build.rs [release] assets versionnés :");
        eprintln!("  style.css → {}", css_filename);
        eprintln!("  app.js    → {}", js_filename);
    } else {
        // ── Mode debug : noms d'origine, aucune copie ────────────────────
        println!("cargo:rustc-env=STYLE_CSS_FILE=style.css");
        println!("cargo:rustc-env=APP_JS_FILE=app.js");

        eprintln!("build.rs [debug] assets non versionnés (style.css / app.js)");
    }
}
