use actix_multipart::Multipart;
use actix_web::HttpResponse;
use futures_util::TryStreamExt;

use crate::pdf::delete::delete_pages;
use crate::pdf::utils::parse_page_ranges;
use crate::{MAX_FILE_SIZE, MAX_FIELD_SIZE};

pub async fn delete_handler(mut payload: Multipart) -> HttpResponse {
    let mut pdf_data: Option<Vec<u8>> = None;
    let mut pages_input = String::new();

    while let Ok(Some(mut field)) = payload.try_next().await {
        let name = field.name().to_string();
        let mut data: Vec<u8> = Vec::new();

        while let Ok(Some(chunk)) = field.try_next().await {
            data.extend_from_slice(&chunk);
            if data.len() > MAX_FILE_SIZE {
                return HttpResponse::PayloadTooLarge()
                    .body("Fichier trop volumineux (max 1 Go).");
            }
        }

        match name.as_str() {
            "file" => {
                if data.len() < 4 || &data[..4] != b"%PDF" {
                    return HttpResponse::BadRequest()
                        .body("Le fichier n'est pas un PDF valide.");
                }
                pdf_data = Some(data);
            }
            "pages" => {
                if data.len() > MAX_FIELD_SIZE {
                    return HttpResponse::BadRequest()
                        .body("Paramètre 'pages' trop long.");
                }
                pages_input = String::from_utf8_lossy(&data).trim().to_string();
            }
            _ => {}
        }
    }

    let pdf_data = match pdf_data {
        Some(d) => d,
        None => return HttpResponse::BadRequest().body("Aucun fichier PDF reçu."),
    };

    if pages_input.is_empty() {
        return HttpResponse::BadRequest().body("Aucune page spécifiée.");
    }

    let page_numbers = match parse_page_ranges(&pages_input) {
        Ok(p) => p,
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
    };

    match delete_pages(&pdf_data, &page_numbers) {
        Ok(bytes) => HttpResponse::Ok()
            .content_type("application/pdf")
            .append_header(("Content-Disposition", "attachment; filename=\"deleted.pdf\""))
            .body(bytes),
        Err(e) => HttpResponse::InternalServerError()
            .body(format!("Erreur de suppression : {}", e)),
    }
}

