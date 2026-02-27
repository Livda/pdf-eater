use lopdf::{Document, ObjectId};
use std::collections::HashSet;

use super::error::{PdfError, Result};
use super::utils::{
    copy_objects, find_catalog, find_pages_root, finalize,
    insert_catalog, insert_pages_node, set_parent,
};

pub fn delete_pages(data: &[u8], page_numbers: &[u32]) -> Result<Vec<u8>> {
    let mut src = Document::load_mem(data)?;
    src.decompress();

    let all_pages = src.get_pages();
    let total = all_pages.len() as u32;

    let to_delete: HashSet<u32> = page_numbers.iter().copied().collect();

    // Validation : pages dans les bornes
    for &n in &to_delete {
        if n < 1 || n > total {
            return Err(PdfError::PageOutOfRange { page: n, total });
        }
    }

    // Validation : au moins une page conservée
    if to_delete.len() == total as usize {
        return Err(PdfError::WouldDeleteAll);
    }

    // Pages conservées, triées pour respecter l'ordre du document
    let mut kept: Vec<(u32, ObjectId)> = all_pages
        .into_iter()
        .filter(|(n, _)| !to_delete.contains(n))
        .collect();
    kept.sort_by_key(|(n, _)| *n);
    let kept_ids: Vec<ObjectId> = kept.into_iter().map(|(_, id)| id).collect();

    let mut out = Document::with_version("1.5");

    // Exclut Catalog, Pages root, et les pages supprimées
    let deleted_ids: HashSet<ObjectId> = src
        .get_pages()
        .into_iter()
        .filter(|(n, _)| to_delete.contains(n))
        .map(|(_, id)| id)
        .collect();

    let excluded: HashSet<ObjectId> = [find_catalog(&src), find_pages_root(&src)]
        .into_iter()
        .flatten()
        .chain(deleted_ids)
        .collect();

    let id_map = copy_objects(&src, &mut out, &excluded);

    let new_kept_ids: Vec<ObjectId> = kept_ids.iter().map(|id| id_map[id]).collect();
    let pages_id = insert_pages_node(&mut out, &new_kept_ids, new_kept_ids.len() as i64);
    set_parent(&mut out, &new_kept_ids, pages_id);

    let catalog_id = insert_catalog(&mut out, pages_id);
    finalize(&mut out, catalog_id)
}
