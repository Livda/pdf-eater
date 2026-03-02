mod handlers;
mod pdf;

use std::sync::OnceLock;

use actix_web::{middleware, web, App, HttpResponse, HttpServer};
use actix_files::Files;

// Noms des fichiers assets déterminés à la compilation par build.rs.
// En release : "style.a3f2c891.css" / "app.b7d41e02.js"
// En debug   : "style.css" / "app.js"
const STYLE_CSS_FILE: &str = env!("STYLE_CSS_FILE");
const APP_JS_FILE:    &str = env!("APP_JS_FILE");

// HTML avec les noms d'assets versionnés injectés, construit une seule fois.
static INDEX_HTML: OnceLock<String> = OnceLock::new();

fn versioned_html() -> &'static str {
    INDEX_HTML.get_or_init(|| {
        include_str!("../static/index.html")
            .replace(
                "/static/style.css\"",
                &format!("/static/{}\"", STYLE_CSS_FILE),
            )
            .replace(
                "/static/app.js\"",
                &format!("/static/{}\"", APP_JS_FILE),
            )
    })
}

// 1 Go en octets
pub const MAX_FILE_SIZE: usize = 1024 * 1024 * 1024;
// Nombre maximum de fichiers par requête
pub const MAX_FILES: usize = 20;
// Taille max d'un champ texte
pub const MAX_FIELD_SIZE: usize = 1024;

async fn index() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .insert_header(("X-Content-Type-Options", "nosniff"))
        .insert_header(("X-Frame-Options", "DENY"))
        .insert_header(("X-XSS-Protection", "1; mode=block"))
        .insert_header(("Referrer-Policy", "no-referrer"))
        .insert_header((
            "Content-Security-Policy",
            "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; frame-src blob:;",
        ))
        .body(versioned_html())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    log::info!("🚀 PDF Eater démarré → http://localhost:8080");
    log::info!("   Assets : {} | {}", STYLE_CSS_FILE, APP_JS_FILE);

    HttpServer::new(|| {
        let json_cfg = web::JsonConfig::default().limit(MAX_FILE_SIZE);

        App::new()
            .app_data(json_cfg)
            .wrap(middleware::Logger::default())
            .wrap(middleware::Compress::default())
            .route("/", web::get().to(index))
            .service(Files::new("/static", "./static").prefer_utf8(true))
            .route("/merge",   web::post().to(handlers::merge::merge_handler))
            .route("/extract", web::post().to(handlers::extract::extract_handler))
            .route("/rotate",  web::post().to(handlers::rotate::rotate_handler))
            .route("/delete",  web::post().to(handlers::delete::delete_handler))
            .route("/reorder", web::post().to(handlers::reorder::reorder_handler))
            .default_service(web::to(|| async {
                HttpResponse::NotFound().body("404 - Page introuvable")
            }))
    })
    .bind("0.0.0.0:8080")?
    .workers(4)
    .run()
    .await
}
