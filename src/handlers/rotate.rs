use actix_multipart::Multipart;
use actix_web::HttpResponse;
use futures_util::TryStreamExt;

use crate::pdf::rotate::rotate_pages;
use crate::{MAX_FILE_SIZE, MAX_FIELD_SIZE};

pub async fn rotate_handler(mut payload: Multipart) -> HttpResponse {
    let mut pdf_data: Option<Vec<u8>> = None;
    let mut rotations_input = String::new();

    while let Ok(Some(mut field)) = payload.try_next().await {
        let name = field.name().to_string();
        let mut data: Vec<u8> = Vec::new();

        while let Ok(Some(chunk)) = field.try_next().await {
            data.extend_from_slice(&chunk);
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
                let ct = field.content_type().map(|m| m.to_string()).unwrap_or_default();
                if ct != "application/pdf" {
                    return HttpResponse::BadRequest().body("Seuls les fichiers PDF sont acceptés.");
                }
                if data.len() < 5 || &data[..5] != b"%PDF-" {
                    return HttpResponse::BadRequest().body("Le fichier ne semble pas être un PDF valide.");
                }
                pdf_data = Some(data);
            }
            "rotations" => {
                match std::str::from_utf8(&data) {
                    Ok(s) => rotations_input = s.trim().to_string(),
                    Err(_) => return HttpResponse::BadRequest().body("Encodage invalide."),
                }
            }
            _ => {}
        }
    }

    let Some(data) = pdf_data else {
        return HttpResponse::BadRequest().body("Fichier PDF requis.");
    };

    // Format attendu : "1:90,2:180,3:270"
    let rotations = match parse_rotations(&rotations_input) {
        Ok(r) => r,
        Err(e) => return HttpResponse::BadRequest().body(e),
    };

    if rotations.is_empty() {
        return HttpResponse::BadRequest().body("Aucune rotation spécifiée.");
    }

    log::info!("Rotation de {} page(s)", rotations.len());

    match rotate_pages(&data, &rotations) {
        Ok(bytes) => {
            log::info!("Rotation réussie ({} octets)", bytes.len());
            HttpResponse::Ok()
                .content_type("application/pdf")
                .insert_header(("X-Content-Type-Options", "nosniff"))
                .append_header(("Content-Disposition", "attachment; filename=\"rotated.pdf\""))
                .body(bytes)
        }
        Err(e) => {
            log::error!("Erreur de rotation : {}", e);
            HttpResponse::InternalServerError().body("Erreur lors de la rotation.")
        }
    }
}

/// Parse "1:90,3:180,5:270" → [(1, 90), (3, 180), (5, 270)]
fn parse_rotations(input: &str) -> Result<Vec<(u32, i64)>, String> {
    let valid_angles = [90i64, 180, 270];
    let mut result = Vec::new();

    for part in input.split(',') {
        let part = part.trim();
        if part.is_empty() { continue; }
        let mut iter = part.splitn(2, ':');
        let page: u32 = iter.next().unwrap_or("").trim().parse()
            .map_err(|_| format!("Numéro de page invalide : '{}'", part))?;
        let angle: i64 = iter.next().unwrap_or("").trim().parse()
            .map_err(|_| format!("Angle invalide : '{}'", part))?;
        if !valid_angles.contains(&angle) {
            return Err(format!("Angle invalide : {} (valeurs autorisées : 90, 180, 270)", angle));
        }
        result.push((page, angle));
    }

    Ok(result)
}
