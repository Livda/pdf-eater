use lopdf::{Document, ObjectId};
use std::collections::HashSet;

use super::error::{PdfError, Result};
use super::utils::{
    copy_objects, find_catalog, find_pages_root,
    finalize, insert_catalog, insert_pages_node,
    load_document, set_parent,
};

/// Fusionne plusieurs PDFs (fournis en bytes bruts) en un seul document.
/// Le parsing est effectué ici — une seule fois par fichier — pour éviter
/// les échecs sur les PDFs avec xref en stream (PDF ≥ 1.5, scanners, etc.)
pub fn merge_documents(files: Vec<Vec<u8>>) -> Result<Vec<u8>> {
    let mut merged = Document::with_version("1.5");
    let mut sub_roots: Vec<ObjectId> = Vec::new();
    let mut total_pages: i64 = 0;

    for (i, data) in files.iter().enumerate() {
        let mut src = load_document(data)
            .map_err(|e| PdfError::InvalidFile(
                format!("Fichier #{} invalide ou corrompu : {}", i + 1, e)))?;
        src.decompress();

        // N'exclure que le Catalog — le nœud Pages racine doit être copié
        // pour devenir enfant du nouveau Pages racine fusionné.
        let excluded: HashSet<ObjectId> = find_catalog(&src).into_iter().collect();

        let id_map = copy_objects(&src, &mut merged, &excluded);

        if let Some(old_root) = find_pages_root(&src) {
            let new_root = id_map[&old_root];

            total_pages += merged
                .objects
                .get(&new_root)
                .and_then(|o| o.as_dict().ok())
                .and_then(|d| d.get(b"Count").ok())
                .and_then(|c| c.as_i64().ok())
                .unwrap_or(0);

            sub_roots.push(new_root);
        }
    }

    let pages_id = insert_pages_node(&mut merged, &sub_roots, total_pages);
    set_parent(&mut merged, &sub_roots, pages_id);

    let catalog_id = insert_catalog(&mut merged, pages_id);
    finalize(&mut merged, catalog_id)
}
