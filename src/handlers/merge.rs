use actix_multipart::Multipart;
use actix_web::HttpResponse;
use futures_util::TryStreamExt;
use lopdf::Document;

use crate::pdf::merge::merge_documents;
use crate::{MAX_FILES, MAX_FILE_SIZE};

pub async fn merge_handler(mut payload: Multipart) -> HttpResponse {
    let mut pdf_data: Vec<Vec<u8>> = Vec::new();

    while let Ok(Some(mut field)) = payload.try_next().await {
        // Vérifie le Content-Type du champ
        let content_type = field
            .content_type()
            .map(|m| m.to_string())
            .unwrap_or_default();

        if content_type != "application/pdf" {
            return HttpResponse::BadRequest()
                .body("Seuls les fichiers PDF sont acceptés.");
        }

        // Limite le nombre de fichiers
        if pdf_data.len() >= MAX_FILES {
            return HttpResponse::BadRequest()
                .body(format!("Maximum {} fichiers par requête.", MAX_FILES));
        }

        let mut data: Vec<u8> = Vec::new();
        while let Ok(Some(chunk)) = field.try_next().await {
            data.extend_from_slice(&chunk);

            // Limite la taille de chaque fichier
            if data.len() > MAX_FILE_SIZE {
                return HttpResponse::PayloadTooLarge()
                    .body("Fichier trop volumineux (max 1 Go).");
            }
        }

        // Vérifie la signature PDF (%PDF-)
        if !is_valid_pdf(&data) {
            return HttpResponse::BadRequest()
                .body("Un fichier ne semble pas être un PDF valide.");
        }

        if !data.is_empty() {
            pdf_data.push(data);
        }
    }

    if pdf_data.len() < 2 {
        return HttpResponse::BadRequest()
            .body("Au moins 2 fichiers PDF sont requis.");
    }

    let mut documents = Vec::new();
    for (i, data) in pdf_data.iter().enumerate() {
        match Document::load_mem(data) {
            Ok(doc) => documents.push(doc),
            Err(e) => {
                log::warn!("Fichier PDF #{} invalide : {}", i + 1, e);
                return HttpResponse::BadRequest()
                    .body(format!("Fichier #{} invalide ou corrompu.", i + 1));
            }
        }
    }

    log::info!("Fusion de {} fichiers PDF", documents.len());

    match merge_documents(documents) {
        Ok(bytes) => {
            log::info!("Fusion réussie ({} octets)", bytes.len());
            HttpResponse::Ok()
                .content_type("application/pdf")
                .insert_header(("X-Content-Type-Options", "nosniff"))
                .append_header(("Content-Disposition", "attachment; filename=\"merged.pdf\""))
                .body(bytes)
        }
        Err(e) => {
            log::error!("Erreur de fusion : {}", e);
            HttpResponse::InternalServerError()
                .body("Erreur lors de la fusion des PDF.")
        }
    }
}

fn is_valid_pdf(data: &[u8]) -> bool {
    data.len() >= 5 && &data[..5] == b"%PDF-"
}
