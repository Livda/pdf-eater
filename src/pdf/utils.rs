use lopdf::{Dictionary, Document, Object, ObjectId};
use std::collections::HashMap;

pub fn remap_dict(dict: Dictionary, map: &HashMap<ObjectId, ObjectId>) -> Dictionary {
    let mut new_dict = Dictionary::new();
    for (k, v) in dict.iter() {
        new_dict.set(k.clone(), remap_object(v.clone(), map));
    }
    new_dict
}

pub fn remap_object(obj: Object, map: &HashMap<ObjectId, ObjectId>) -> Object {
    match obj {
        Object::Reference(id) => Object::Reference(*map.get(&id).unwrap_or(&id)),
        Object::Array(arr) => {
            Object::Array(arr.into_iter().map(|o| remap_object(o, map)).collect())
        }
        Object::Dictionary(dict) => Object::Dictionary(remap_dict(dict, map)),
        Object::Stream(mut stream) => {
            stream.dict = remap_dict(stream.dict, map);
            Object::Stream(stream)
        }
        other => other,
    }
}

pub fn find_by_type(doc: &Document, type_name: &[u8]) -> Option<ObjectId> {
    doc.objects
        .iter()
        .find(|(_, obj)| {
            obj.as_dict()
                .ok()
                .and_then(|d| d.get(b"Type").ok())
                .and_then(|t| t.as_name().ok())
                .map(|t| t == type_name)
                .unwrap_or(false)
        })
        .map(|(id, _)| *id)
}

pub fn find_pages_root(doc: &Document) -> Option<ObjectId> {
    doc.objects
        .iter()
        .find(|(_, obj)| {
            obj.as_dict()
                .ok()
                .map(|d| {
                    d.get(b"Type")
                        .ok()
                        .and_then(|t| t.as_name().ok())
                        .map(|t| t == b"Pages")
                        .unwrap_or(false)
                        && d.get(b"Parent").is_err()
                })
                .unwrap_or(false)
        })
        .map(|(id, _)| *id)
}

pub fn save_to_bytes(
    doc: &mut lopdf::Document,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    use std::time::{SystemTime, UNIX_EPOCH};

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    let tmp_path = std::env::temp_dir().join(format!("pdf_tool_{}.pdf", nanos));

    // Garantit la suppression même en cas d'erreur grâce au Drop
    struct TempFile(std::path::PathBuf);
    impl Drop for TempFile {
        fn drop(&mut self) {
            if self.0.exists() {
                if let Err(e) = std::fs::remove_file(&self.0) {
                    log::warn!("Impossible de supprimer le fichier temporaire {:?} : {}", self.0, e);
                }
            }
        }
    }
    let _guard = TempFile(tmp_path.clone());

    doc.save(&tmp_path)?;
    let bytes = std::fs::read(&tmp_path)?;
    // _guard est drop ici → suppression garantie même si read() a paniqué
    Ok(bytes)
}
