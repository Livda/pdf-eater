use lopdf::{Dictionary, Document, Object, ObjectId};
use std::collections::HashMap;

use super::utils::{find_by_type, find_pages_root, remap_object, save_to_bytes};

pub fn extract_pages(
    data: &[u8],
    page_numbers: &[u32],
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let mut src = Document::load_mem(data)?;
    src.decompress();

    let all_pages = src.get_pages();
    let total = all_pages.len();

    // Valide et déduplique les numéros de page
    let mut page_ids: Vec<ObjectId> = Vec::new();
    for &n in page_numbers {
        let id = all_pages.get(&n).ok_or_else(|| {
            format!("Page {} introuvable (le document a {} page(s))", n, total)
        })?;
        page_ids.push(*id);
    }

    let mut extracted = Document::with_version("1.5");
    let mut id_map: HashMap<ObjectId, ObjectId> = HashMap::new();
    for &old_id in src.objects.keys() {
        extracted.max_id += 1;
        id_map.insert(old_id, (extracted.max_id, 0));
    }

    let catalog_old    = find_by_type(&src, b"Catalog");
    let pages_root_old = find_pages_root(&src);
    let excluded: std::collections::HashSet<ObjectId> =
        [catalog_old, pages_root_old].iter().filter_map(|x| *x).collect();

    for (old_id, object) in &src.objects {
        if excluded.contains(old_id) { continue; }
        let new_id = id_map[old_id];
        extracted.objects.insert(new_id, remap_object(object.clone(), &id_map));
    }

    let pages_id: ObjectId = (extracted.max_id + 1, 0);
    extracted.max_id += 1;

    let new_page_ids: Vec<ObjectId> = page_ids.iter().map(|id| id_map[id]).collect();

    for &new_page_id in &new_page_ids {
        if let Some(obj) = extracted.objects.get_mut(&new_page_id) {
            if let Object::Dictionary(dict) = obj {
                dict.set("Parent", Object::Reference(pages_id));
            }
        }
    }

    extracted.objects.insert(
        pages_id,
        Object::Dictionary(Dictionary::from_iter(vec![
            ("Type",  Object::Name(b"Pages".to_vec())),
            ("Kids",  Object::Array(new_page_ids.iter().map(|id| Object::Reference(*id)).collect())),
            ("Count", Object::Integer(new_page_ids.len() as i64)),
        ])),
    );

    let catalog_id: ObjectId = (extracted.max_id + 1, 0);
    extracted.max_id += 1;

    extracted.objects.insert(
        catalog_id,
        Object::Dictionary(Dictionary::from_iter(vec![
            ("Type",  Object::Name(b"Catalog".to_vec())),
            ("Pages", Object::Reference(pages_id)),
        ])),
    );

    extracted.trailer.set("Root", Object::Reference(catalog_id));
    extracted.trailer.set("Size", Object::Integer((extracted.max_id + 1) as i64));

    save_to_bytes(&mut extracted)
}

/// Parse "1, 3, 5-7, 10" → [1, 3, 5, 6, 7, 10]
pub fn parse_page_ranges(input: &str) -> Result<Vec<u32>, String> {
    let mut pages = Vec::new();
    for part in input.split(',') {
        let part = part.trim();
        if part.contains('-') {
            let mut bounds = part.splitn(2, '-');
            let a: u32 = bounds.next().unwrap_or("").trim().parse()
                .map_err(|_| format!("Plage invalide : '{}'", part))?;
            let b: u32 = bounds.next().unwrap_or("").trim().parse()
                .map_err(|_| format!("Plage invalide : '{}'", part))?;
            if a > b { return Err(format!("Plage invalide : {} > {}", a, b)); }
            pages.extend(a..=b);
        } else if !part.is_empty() {
            let n: u32 = part.parse()
                .map_err(|_| format!("Numéro invalide : '{}'", part))?;
            pages.push(n);
        }
    }
    if pages.is_empty() { return Err("Aucune page spécifiée.".into()); }
    // Déduplique en gardant l'ordre
    let mut seen = std::collections::HashSet::new();
    pages.retain(|p| seen.insert(*p));
    Ok(pages)
}
