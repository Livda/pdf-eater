use lopdf::{Dictionary, Document, Object, ObjectId};
use std::collections::HashMap;

use super::utils::{find_by_type, find_pages_root, remap_object, save_to_bytes};

pub fn delete_pages(
    data: &[u8],
    page_numbers: &[u32],
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let mut src = Document::load_mem(data)?;
    src.decompress();

    let all_pages = src.get_pages();
    let total = all_pages.len() as u32;

    // Validation
    for &n in page_numbers {
        if n < 1 || n > total {
            return Err(format!(
                "Page {} invalide (le document a {} page(s))",
                n, total
            )
            .into());
        }
    }

    let pages_to_delete: std::collections::HashSet<u32> =
        page_numbers.iter().cloned().collect();

    if pages_to_delete.len() == total as usize {
        return Err("Impossible de supprimer toutes les pages du document.".into());
    }

    // Pages à conserver, dans l'ordre
    let mut kept_pages: Vec<(u32, ObjectId)> = all_pages
        .into_iter()
        .filter(|(n, _)| !pages_to_delete.contains(n))
        .collect();
    kept_pages.sort_by_key(|(n, _)| *n);
    let kept_ids: Vec<ObjectId> = kept_pages.into_iter().map(|(_, id)| id).collect();

    // Construit le mapping old_id → new_id
    let mut out = Document::with_version("1.5");
    let mut id_map: HashMap<ObjectId, ObjectId> = HashMap::new();
    for &old_id in src.objects.keys() {
        out.max_id += 1;
        id_map.insert(old_id, (out.max_id, 0));
    }

    let catalog_old    = find_by_type(&src, b"Catalog");
    let pages_root_old = find_pages_root(&src);

    let excluded: std::collections::HashSet<ObjectId> =
        [catalog_old, pages_root_old].iter().filter_map(|x| *x).collect();

    // Copie tous les objets sauf Catalog, Pages root et pages supprimées
    let deleted_ids: std::collections::HashSet<ObjectId> = all_pages_ids_to_delete(
        &src,
        page_numbers,
        &src.get_pages()
            .into_iter()
            .filter(|(n, _)| pages_to_delete.contains(n))
            .map(|(_, id)| id)
            .collect::<Vec<_>>(),
    );

    for (old_id, object) in &src.objects {
        if excluded.contains(old_id) || deleted_ids.contains(old_id) {
            continue;
        }
        let new_id = id_map[old_id];
        out.objects.insert(new_id, remap_object(object.clone(), &id_map));
    }

    // Nouveau Pages root
    let pages_id: ObjectId = (out.max_id + 1, 0);
    out.max_id += 1;

    let new_kept_ids: Vec<ObjectId> = kept_ids.iter().map(|id| id_map[id]).collect();

    // Met à jour le Parent de chaque page conservée
    for &new_page_id in &new_kept_ids {
        if let Some(obj) = out.objects.get_mut(&new_page_id) {
            if let Object::Dictionary(dict) = obj {
                dict.set("Parent", Object::Reference(pages_id));
            }
        }
    }

    out.objects.insert(
        pages_id,
        Object::Dictionary(Dictionary::from_iter(vec![
            ("Type",  Object::Name(b"Pages".to_vec())),
            ("Kids",  Object::Array(new_kept_ids.iter().map(|id| Object::Reference(*id)).collect())),
            ("Count", Object::Integer(new_kept_ids.len() as i64)),
        ])),
    );

    let catalog_id: ObjectId = (out.max_id + 1, 0);
    out.max_id += 1;

    out.objects.insert(
        catalog_id,
        Object::Dictionary(Dictionary::from_iter(vec![
            ("Type",  Object::Name(b"Catalog".to_vec())),
            ("Pages", Object::Reference(pages_id)),
        ])),
    );

    out.trailer.set("Root", Object::Reference(catalog_id));
    out.trailer.set("Size", Object::Integer((out.max_id + 1) as i64));

    save_to_bytes(&mut out)
}

/// Retourne les ObjectIds des pages à supprimer (juste les pages, pas leurs ressources)
fn all_pages_ids_to_delete(
    _doc: &Document,
    _page_numbers: &[u32],
    page_ids: &[ObjectId],
) -> std::collections::HashSet<ObjectId> {
    page_ids.iter().cloned().collect()
}
