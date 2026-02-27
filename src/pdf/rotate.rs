use lopdf::{Document, Object, ObjectId};
use std::collections::HashSet;

use super::error::{PdfError, Result};
use super::utils::{
    copy_objects, find_catalog, find_pages_root, finalize,
    insert_catalog, insert_pages_node, set_parent,
};

/// Applique des rotations à des pages spécifiques.
/// Chaque entrée de `rotations` est `(numéro_de_page, angle)` avec angle ∈ {90, 180, 270}.
/// L'angle est cumulé avec la rotation existante de la page.
pub fn rotate_pages(data: &[u8], rotations: &[(u32, i64)]) -> Result<Vec<u8>> {
    let mut src = Document::load_mem(data)?;
    src.decompress();

    let all_pages = src.get_pages();
    let total = all_pages.len();

    for &(page_number, angle) in rotations {
        let &page_id = all_pages
            .get(&page_number)
            .ok_or(PdfError::PageOutOfRange { page: page_number, total: total as u32 })?;

        let current = src
            .objects
            .get(&page_id)
            .and_then(|o| o.as_dict().ok())
            .and_then(|d| d.get(b"Rotate").ok())
            .and_then(|r| r.as_i64().ok())
            .unwrap_or(0);

        let new_rotation = (current + angle).rem_euclid(360);

        if let Some(Object::Dictionary(dict)) = src.objects.get_mut(&page_id) {
            dict.set("Rotate", Object::Integer(new_rotation));
        }
    }

    rebuild(src)
}

/// Reconstruit un document propre à partir d'un document modifié en place.
/// Nécessaire car lopdf ne garantit pas un PDF valide après modification directe.
fn rebuild(src: Document) -> Result<Vec<u8>> {
    let mut out = Document::with_version("1.5");

    let pages_root = find_pages_root(&src);

    let excluded: HashSet<ObjectId> = [find_catalog(&src), pages_root]
        .into_iter()
        .flatten()
        .collect();

    let id_map = copy_objects(&src, &mut out, &excluded);

    // Le nœud Pages source (avec ses Kids et sa hiérarchie intacte) devient
    // l'unique enfant du nouveau nœud Pages racine.
    if let Some(old_root) = pages_root {
        let new_root = id_map[&old_root];

        let count = src
            .objects
            .get(&old_root)
            .and_then(|o| o.as_dict().ok())
            .and_then(|d| d.get(b"Count").ok())
            .and_then(|c| c.as_i64().ok())
            .unwrap_or(0);

        let pages_id = insert_pages_node(&mut out, &[new_root], count);
        set_parent(&mut out, &[new_root], pages_id);

        let catalog_id = insert_catalog(&mut out, pages_id);
        return finalize(&mut out, catalog_id);
    }

    Err(PdfError::Lopdf(lopdf::Error::DictKey))
}
