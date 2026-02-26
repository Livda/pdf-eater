use lopdf::{Dictionary, Document, Object, ObjectId};
use std::collections::HashMap;

use super::utils::{find_by_type, find_pages_root, remap_object, save_to_bytes};

/// Réorganise les pages d'un PDF selon l'ordre donné.
/// `order` : numéros de page (1-indexés) dans le nouvel ordre souhaité.
/// Toutes les pages doivent être présentes exactement une fois.
pub fn reorder_pages(
    data: &[u8],
    order: &[u32],
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let mut src = Document::load_mem(data)?;
    src.decompress();

    let all_pages = src.get_pages();
    let total = all_pages.len() as u32;

    // Validation : bon nombre de pages
    if order.len() != total as usize {
        return Err(format!(
            "L'ordre doit contenir exactement {} page(s), {} fournie(s).",
            total,
            order.len()
        )
        .into());
    }

    // Validation : pas de doublon, pas de page hors limites
    let mut seen = std::collections::HashSet::new();
    for &n in order {
        if n < 1 || n > total {
            return Err(
                format!("Page {} invalide (le document a {} page(s)).", n, total).into(),
            );
        }
        if !seen.insert(n) {
            return Err(format!("Page {} apparaît plusieurs fois dans l'ordre.", n).into());
        }
    }

    // ObjectIds dans le nouvel ordre
    let reordered_ids: Vec<ObjectId> = order.iter().map(|n| all_pages[n]).collect();

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

    // Copie tous les objets sauf Catalog et Pages root (recréés ci-dessous)
    for (old_id, object) in &src.objects {
        if excluded.contains(old_id) {
            continue;
        }
        let new_id = id_map[old_id];
        out.objects
            .insert(new_id, remap_object(object.clone(), &id_map));
    }

    // Nouveau Pages root avec les pages dans le nouvel ordre
    let pages_id: ObjectId = (out.max_id + 1, 0);
    out.max_id += 1;

    let new_page_ids: Vec<ObjectId> = reordered_ids.iter().map(|id| id_map[id]).collect();

    // Met à jour le champ Parent de chaque page
    for &new_page_id in &new_page_ids {
        if let Some(Object::Dictionary(dict)) = out.objects.get_mut(&new_page_id) {
            dict.set("Parent", Object::Reference(pages_id));
        }
    }

    out.objects.insert(
        pages_id,
        Object::Dictionary(Dictionary::from_iter(vec![
            ("Type",  Object::Name(b"Pages".to_vec())),
            ("Kids",  Object::Array(
                new_page_ids.iter().map(|id| Object::Reference(*id)).collect(),
            )),
            ("Count", Object::Integer(new_page_ids.len() as i64)),
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
