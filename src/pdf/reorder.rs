use lopdf::{Document, ObjectId};
use std::collections::HashSet;

use super::error::{PdfError, Result};
use super::utils::{
    copy_objects, find_catalog, find_pages_root, finalize,
    insert_catalog, insert_pages_node, set_parent,
};

/// Réorganise les pages selon `order` (numéros 1-indexés dans le nouvel ordre).
/// Toutes les pages du document doivent figurer exactement une fois dans `order`.
pub fn reorder_pages(data: &[u8], order: &[u32]) -> Result<Vec<u8>> {
    let mut src = Document::load_mem(data)?;
    src.decompress();

    let all_pages = src.get_pages();
    let total = all_pages.len();

    if order.len() != total {
        return Err(PdfError::WrongPageCount { expected: total, got: order.len() });
    }

    // Validation : pas de doublon, pas de page hors limites
    let mut seen = HashSet::new();
    for &n in order {
        if n < 1 || n > total as u32 {
            return Err(PdfError::PageOutOfRange { page: n, total: total as u32 });
        }
        if !seen.insert(n) {
            return Err(PdfError::DuplicatePage(n));
        }
    }

    let reordered_ids: Vec<ObjectId> = order.iter().map(|n| all_pages[n]).collect();

    let mut out = Document::with_version("1.5");

    let excluded: HashSet<ObjectId> = [find_catalog(&src), find_pages_root(&src)]
        .into_iter()
        .flatten()
        .collect();

    let id_map = copy_objects(&src, &mut out, &excluded);

    let new_page_ids: Vec<ObjectId> = reordered_ids.iter().map(|id| id_map[id]).collect();
    let pages_id = insert_pages_node(&mut out, &new_page_ids, new_page_ids.len() as i64);
    set_parent(&mut out, &new_page_ids, pages_id);

    let catalog_id = insert_catalog(&mut out, pages_id);
    finalize(&mut out, catalog_id)
}
