use actix_multipart::Multipart;
use actix_web::HttpResponse;
use futures_util::TryStreamExt;

use crate::pdf::extract::extract_pages;
use crate::pdf::utils::parse_page_ranges;
use crate::{MAX_FIELD_SIZE, MAX_FILE_SIZE};

pub async fn extract_handler(mut payload: Multipart) -> HttpResponse {
    let mut pdf_data: Option<Vec<u8>> = None;
    let mut pages_input = String::new();

    while let Ok(Some(mut field)) = payload.try_next().await {
        let name = field.name().to_string();
        let mut data: Vec<u8> = Vec::new();

        while let Ok(Some(chunk)) = field.try_next().await {
            data.extend_from_slice(&chunk);

            // Limite selon le type de champ
            let limit = if name == "file" { MAX_FILE_SIZE } else { MAX_FIELD_SIZE };
            if data.len() > limit {
                return HttpResponse::PayloadTooLarge()
                    .body(if name == "file" {
                        "Fichier trop volumineux (max 1 Go).".to_string()
                    } else {
                        format!("Champ '{}' trop long.", name)
                    });
            }
        }

        match name.as_str() {
            "file" => {
                // Vérifie le Content-Type
                let content_type = field
                    .content_type()
                    .map(|m| m.to_string())
                    .unwrap_or_default();

                if content_type != "application/pdf" {
                    return HttpResponse::BadRequest()
                        .body("Seuls les fichiers PDF sont acceptés.");
                }

                // Vérifie la signature PDF
                if !is_valid_pdf(&data) {
                    return HttpResponse::BadRequest()
                        .body("Le fichier ne semble pas être un PDF valide.");
                }

                pdf_data = Some(data);
            }
            "pages" => {
                match std::str::from_utf8(&data) {
                    Ok(s) => {
                        // N'autorise que les caractères valides pour une plage de pages
                        let clean = s.trim();
                        if !clean.chars().all(|c| c.is_ascii_digit() || c == ',' || c == '-' || c == ' ') {
                            return HttpResponse::BadRequest()
                                .body("Format de pages invalide.");
                        }
                        pages_input = clean.to_string();
                    }
                    Err(_) => {
                        return HttpResponse::BadRequest().body("Encodage invalide.");
                    }
                }
            }
            // Ignore tout champ inconnu
            _ => {
                log::debug!("Champ multipart inconnu ignoré : '{}'", name);
            }
        }
    }

    let Some(data) = pdf_data else {
        return HttpResponse::BadRequest().body("Fichier PDF requis.".to_string());
    };

    let page_numbers: Vec<u32> = match parse_page_ranges(&pages_input) {
        Ok(p) => p,
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
    };

    // Limite le nombre de pages à extraire
    if page_numbers.len() > 500 {
        return HttpResponse::BadRequest()
            .body("Maximum 500 pages par extraction.".to_string());
    }

    let filename = if page_numbers.len() == 1 {
        format!("page_{}.pdf", page_numbers[0])
    } else {
        "pages_extraites.pdf".to_string()
    };

    log::info!("Extraction de {} page(s)", page_numbers.len());

    match extract_pages(&data, &page_numbers) {
        Ok(bytes) => {
            log::info!("Extraction réussie ({} octets)", bytes.len());
            HttpResponse::Ok()
                .content_type("application/pdf")
                .insert_header(("X-Content-Type-Options", "nosniff"))
                .append_header(("Content-Disposition",
                    format!("attachment; filename=\"{}\"", filename)))
                .body(bytes)
        }
        Err(e) => {
            log::error!("Erreur d'extraction : {}", e);
            HttpResponse::InternalServerError()
                .body("Erreur lors de l'extraction.")
        }
    }
}

fn is_valid_pdf(data: &[u8]) -> bool {
    data.len() >= 5 && &data[..5] == b"%PDF-"
}
