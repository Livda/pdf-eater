use std::{env, fs};

fn fnv1a_hex(data: &[u8]) -> String {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME:  u64 = 0x00000100000001b3;
    let hash = data.iter().fold(OFFSET, |h, &b| {
        (h ^ b as u64).wrapping_mul(PRIME)
    });
    format!("{:016x}", hash)[..8].to_owned()
}

/// Supprime les anciennes versions hachées d'un asset.
/// Conserve le fichier original (sans hash) et le fichier `keep`.
/// Un fichier haché a la forme : prefix + 8 caractères hex + suffix.
fn cleanup_old_hashed(dir: &str, prefix: &str, suffix: &str, keep: &str) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        // Vérifie que le nom a la forme "prefix<8-hex-chars>suffix"
        let hash_part = name
            .strip_prefix(prefix)
            .and_then(|s| s.strip_suffix(&suffix[1..])); // suffix commence par '.'
        let is_hashed = hash_part.map(|h| h.len() == 8 && h.chars().all(|c| c.is_ascii_hexdigit()))
            .unwrap_or(false);
        if is_hashed && name.as_ref() != keep {
            let _ = fs::remove_file(entry.path());
        }
    }
}

fn main() {
    println!("cargo:rerun-if-changed=static/style.css");
    println!("cargo:rerun-if-changed=static/app.js");

    let profile = env::var("PROFILE").unwrap_or_default();

    let css_data = fs::read("static/style.css").expect("static/style.css introuvable");
    let js_data  = fs::read("static/app.js").expect("static/app.js introuvable");

    let (css_file, js_file) = if profile == "release" {
        let css_hash = fnv1a_hex(&css_data);
        let js_hash  = fnv1a_hex(&js_data);

        let css_name = format!("style.{}.css", css_hash);
        let js_name  = format!("app.{}.js",    js_hash);

        // Supprime les anciennes versions hachées
        cleanup_old_hashed("static", "style.", ".css", &css_name);
        cleanup_old_hashed("static", "app.",   ".js",  &js_name);

        // Copie avec le nom haché (écrase si déjà correct)
        fs::copy("static/style.css", format!("static/{}", css_name))
            .expect("Impossible de copier style.css");
        fs::copy("static/app.js", format!("static/{}", js_name))
            .expect("Impossible de copier app.js");

        (css_name, js_name)
    } else {
        // En dev : les fichiers originaux, pas de renommage
        ("style.css".to_owned(), "app.js".to_owned())
    };

    println!("cargo:rustc-env=STYLE_CSS_FILE={}", css_file);
    println!("cargo:rustc-env=APP_JS_FILE={}",    js_file);
}
