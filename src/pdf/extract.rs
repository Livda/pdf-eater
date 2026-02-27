use lopdf::{Document, ObjectId};
use std::collections::HashSet;

use super::error::{PdfError, Result};
use super::utils::{
    copy_objects, find_catalog, find_pages_root, finalize,
    insert_catalog, insert_pages_node, set_parent,
};

pub fn extract_pages(data: &[u8], page_numbers: &[u32]) -> Result<Vec<u8>> {
    let mut src = Document::load_mem(data)?;
    src.decompress();

    let all_pages = src.get_pages();
    let total = all_pages.len();

    // Résolution des numéros de page en ObjectIds (dans l'ordre fourni)
    let page_ids: Vec<ObjectId> = page_numbers
        .iter()
        .map(|&n| {
            all_pages
                .get(&n)
                .copied()
                .ok_or(PdfError::PageOutOfRange { page: n, total: total as u32 })
        })
        .collect::<Result<_>>()?;

    let mut out = Document::with_version("1.5");

    let excluded: HashSet<ObjectId> = [find_catalog(&src), find_pages_root(&src)]
        .into_iter()
        .flatten()
        .collect();

    let id_map = copy_objects(&src, &mut out, &excluded);

    let new_page_ids: Vec<ObjectId> = page_ids.iter().map(|id| id_map[id]).collect();
    let pages_id = insert_pages_node(&mut out, &new_page_ids, new_page_ids.len() as i64);
    set_parent(&mut out, &new_page_ids, pages_id);

    let catalog_id = insert_catalog(&mut out, pages_id);
    finalize(&mut out, catalog_id)
}
