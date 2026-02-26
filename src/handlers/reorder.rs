use actix_multipart::Multipart;
use actix_web::HttpResponse;
use futures_util::StreamExt;

use crate::{MAX_FIELD_SIZE, MAX_FILE_SIZE};
use crate::pdf::reorder::reorder_pages;

pub async fn reorder_handler(mut payload: Multipart) -> HttpResponse {
    let mut file_data: Option<Vec<u8>> = None;
    let mut order_str = String::new();

    while let Some(item) = payload.next().await {
        let mut field = match item {
            Ok(f) => f,
            Err(e) => return HttpResponse::BadRequest().body(format!("Erreur multipart : {}", e)),
        };

        let name = field
            .content_disposition()
            .get_name()
            .unwrap_or("")
            .to_string();

        match name.as_str() {
            "file" => {
                let content_type = field
                    .content_type()
                    .map(|ct| ct.to_string())
                    .unwrap_or_default();
                if content_type != "application/pdf" {
                    return HttpResponse::BadRequest()
                        .body("Le fichier doit être un PDF (application/pdf).");
                }

                let mut buf = Vec::new();
                while let Some(chunk) = field.next().await {
                    match chunk {
                        Ok(data) => {
                            buf.extend_from_slice(&data);
                            if buf.len() > MAX_FILE_SIZE {
                                return HttpResponse::PayloadTooLarge()
                                    .body("Fichier trop volumineux (max 1 Go).");
                            }
                        }
                        Err(e) => {
                            return HttpResponse::BadRequest()
                                .body(format!("Erreur lecture fichier : {}", e))
                        }
                    }
                }

                if !buf.starts_with(b"%PDF") {
                    return HttpResponse::BadRequest().body("Fichier non reconnu comme PDF.");
                }

                file_data = Some(buf);
            }

            "order" => {
                let mut buf = Vec::new();
                while let Some(chunk) = field.next().await {
                    match chunk {
                        Ok(data) => {
                            buf.extend_from_slice(&data);
                            if buf.len() > MAX_FIELD_SIZE {
                                return HttpResponse::BadRequest()
                                    .body("Champ 'order' trop long.");
                            }
                        }
                        Err(e) => {
                            return HttpResponse::BadRequest()
                                .body(format!("Erreur lecture champ : {}", e))
                        }
                    }
                }
                order_str = String::from_utf8_lossy(&buf).to_string();
            }

            _ => {
                while let Some(chunk) = field.next().await {
                    if chunk.is_err() { break; }
                }
            }
        }
    }

    let data = match file_data {
        Some(d) => d,
        None => return HttpResponse::BadRequest().body("Aucun fichier fourni."),
    };

    if order_str.is_empty() {
        return HttpResponse::BadRequest().body("Paramètre 'order' manquant.");
    }

    let order: Vec<u32> = match order_str
        .split(',')
        .map(|s| s.trim().parse::<u32>())
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(v) => v,
        Err(_) => return HttpResponse::BadRequest()
            .body("Paramètre 'order' invalide (attendu : \"1,3,2,...\")."),
    };

    match reorder_pages(&data, &order) {
        Ok(result) => HttpResponse::Ok()
            .content_type("application/pdf")
            .insert_header(("Content-Disposition", "attachment; filename=\"reordered.pdf\""))
            .body(result),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}
