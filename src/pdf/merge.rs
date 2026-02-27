use lopdf::{Document, ObjectId};
use std::collections::HashSet;

use super::error::Result;
use super::utils::{copy_objects, find_catalog, find_pages_root, insert_catalog, insert_pages_node, finalize, set_parent};

pub fn merge_documents(documents: Vec<Document>) -> Result<Vec<u8>> {
    let mut merged = Document::with_version("1.5");
    let mut sub_roots: Vec<ObjectId> = Vec::new();
    let mut total_pages: i64 = 0;

    for mut src in documents {
        src.decompress();

        // Le Catalog n'est pas utile dans le document fusionné : on l'exclut.
        // Le nœud Pages racine de chaque source devient un enfant du nouveau
        // nœud Pages racine commun — ses Kids et Count internes sont conservés.
        let catalog = find_catalog(&src).into_iter().collect::<HashSet<_>>();
        let pages_root = find_pages_root(&src);

        let id_map = copy_objects(&src, &mut merged, &catalog);

        if let Some(old_root) = pages_root {
            let new_root = id_map[&old_root];

            // Accumule le nombre total de pages feuilles depuis le /Count du sous-arbre
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

    // Le nouveau Pages root agrège tous les sous-arbres.
    // On met à jour leur Parent après avoir alloué l'ID du nouveau nœud racine.
    let pages_id = insert_pages_node(&mut merged, &sub_roots, total_pages);
    set_parent(&mut merged, &sub_roots, pages_id);

    let catalog_id = insert_catalog(&mut merged, pages_id);
    finalize(&mut merged, catalog_id)
}
