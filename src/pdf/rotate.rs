use lopdf::{Document, Object};

use super::utils::{find_by_type, find_pages_root, save_to_bytes};

pub fn rotate_pages(
    data: &[u8],
    rotations: &[(u32, i64)], // (numéro de page, angle : 90 | 180 | 270)
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let mut doc = Document::load_mem(data)?;
    doc.decompress();

    // Construit un mapping numéro → ObjectId
    let pages = doc.get_pages();

    for (page_number, angle) in rotations {
        // Normalise l'angle : cumule avec la rotation existante
        let page_id = pages.get(page_number).ok_or_else(|| {
            format!(
                "Page {} introuvable (le document a {} page(s))",
                page_number,
                pages.len()
            )
        })?;

        let current_rotation = doc
            .objects
            .get(page_id)
            .and_then(|o| o.as_dict().ok())
            .and_then(|d| {
                // La rotation peut être héritée du nœud Pages parent
                d.get(b"Rotate").ok().and_then(|r| r.as_i64().ok())
            })
            .unwrap_or(0);

        let new_rotation = (current_rotation + angle).rem_euclid(360);

        if let Some(obj) = doc.objects.get_mut(page_id) {
            if let Object::Dictionary(dict) = obj {
                dict.set("Rotate", Object::Integer(new_rotation));
            }
        }
    }

    // Reconstruit proprement le document (nouveau Catalog + Pages root)
    rebuild_doc(doc)
}

/// Reconstruit un Document propre sans modifier la structure des pages
fn rebuild_doc(
    src: Document,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    use lopdf::{Dictionary, Document as LDoc, Object, ObjectId};
    use std::collections::HashMap;

    let mut out = LDoc::with_version("1.5");
    let mut id_map: HashMap<ObjectId, ObjectId> = HashMap::new();

    for &old_id in src.objects.keys() {
        out.max_id += 1;
        id_map.insert(old_id, (out.max_id, 0));
    }

    let catalog_old    = find_by_type(&src, b"Catalog");
    let pages_root_old = find_pages_root(&src);

    for (old_id, object) in &src.objects {
        if Some(*old_id) == catalog_old { continue; }
        let new_id = id_map[old_id];
        out.objects.insert(new_id, super::utils::remap_object(object.clone(), &id_map));
    }

    // Nouveau Pages root
    let pages_id: ObjectId = (out.max_id + 1, 0);
    out.max_id += 1;

    // Trouve l'ancien Pages root remappé
    if let Some(old_root) = pages_root_old {
        let new_root_id = id_map[&old_root];
        if let Some(obj) = out.objects.get_mut(&new_root_id) {
            if let Object::Dictionary(dict) = obj {
                dict.set("Parent", Object::Reference(pages_id));
            }
        }

        let count = out.objects.get(&new_root_id)
            .and_then(|o| o.as_dict().ok())
            .and_then(|d| d.get(b"Count").ok())
            .and_then(|c| c.as_i64().ok())
            .unwrap_or(0);

        out.objects.insert(
            pages_id,
            Object::Dictionary(Dictionary::from_iter(vec![
                ("Type",  Object::Name(b"Pages".to_vec())),
                ("Kids",  Object::Array(vec![Object::Reference(new_root_id)])),
                ("Count", Object::Integer(count)),
            ])),
        );
    }

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
