use lopdf::{Dictionary, Document, Object, ObjectId};
use std::collections::HashMap;

use super::utils::{find_by_type, find_pages_root, remap_object, save_to_bytes};

pub fn merge_documents(
    documents: Vec<Document>,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let mut merged = Document::with_version("1.5");
    let mut root_kids: Vec<Object> = Vec::new();
    let mut total_pages: i64 = 0;

    for mut src in documents {
        src.decompress();

        let mut id_map: HashMap<ObjectId, ObjectId> = HashMap::new();
        for &old_id in src.objects.keys() {
            merged.max_id += 1;
            id_map.insert(old_id, (merged.max_id, 0));
        }

        let catalog_old    = find_by_type(&src, b"Catalog");
        let pages_root_old = find_pages_root(&src);

        for (old_id, object) in &src.objects {
            if Some(*old_id) == catalog_old {
                continue;
            }
            let new_id = id_map[old_id];
            merged
                .objects
                .insert(new_id, remap_object(object.clone(), &id_map));
        }

        if let Some(old_root) = pages_root_old {
            let new_root_id = id_map[&old_root];
            total_pages += merged
                .objects
                .get(&new_root_id)
                .and_then(|o| o.as_dict().ok())
                .and_then(|d| d.get(b"Count").ok())
                .and_then(|c| c.as_i64().ok())
                .unwrap_or(0);
            root_kids.push(Object::Reference(new_root_id));
        }
    }

    let pages_id: ObjectId = (merged.max_id + 1, 0);
    merged.max_id += 1;

    for kid in &root_kids {
        if let Object::Reference(kid_id) = kid {
            if let Some(obj) = merged.objects.get_mut(kid_id) {
                if let Object::Dictionary(dict) = obj {
                    dict.set("Parent", Object::Reference(pages_id));
                }
            }
        }
    }

    merged.objects.insert(
        pages_id,
        Object::Dictionary(Dictionary::from_iter(vec![
            ("Type",  Object::Name(b"Pages".to_vec())),
            ("Kids",  Object::Array(root_kids)),
            ("Count", Object::Integer(total_pages)),
        ])),
    );

    let catalog_id: ObjectId = (merged.max_id + 1, 0);
    merged.max_id += 1;

    merged.objects.insert(
        catalog_id,
        Object::Dictionary(Dictionary::from_iter(vec![
            ("Type",  Object::Name(b"Catalog".to_vec())),
            ("Pages", Object::Reference(pages_id)),
        ])),
    );

    merged.trailer.set("Root", Object::Reference(catalog_id));
    merged
        .trailer
        .set("Size", Object::Integer((merged.max_id + 1) as i64));

    save_to_bytes(&mut merged)
}
