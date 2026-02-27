use lopdf::{Dictionary, Document, Object, ObjectId};
use std::collections::{HashMap, HashSet};

use super::error::{PdfError, Result};

// ── Remapping d'objets ────────────────────────────────────────────────────────

/// Réécrit récursivement toutes les références d'un objet selon `map`.
/// Les références absentes de `map` sont conservées telles quelles.
pub fn remap_object(obj: Object, map: &HashMap<ObjectId, ObjectId>) -> Object {
    match obj {
        Object::Reference(id) => Object::Reference(*map.get(&id).unwrap_or(&id)),
        Object::Array(arr) => {
            Object::Array(arr.into_iter().map(|o| remap_object(o, map)).collect())
        }
        Object::Dictionary(dict) => Object::Dictionary(remap_dict(dict, map)),
        Object::Stream(mut s) => {
            s.dict = remap_dict(s.dict, map);
            Object::Stream(s)
        }
        other => other,
    }
}

fn remap_dict(dict: Dictionary, map: &HashMap<ObjectId, ObjectId>) -> Dictionary {
    let mut out = Dictionary::new();
    for (k, v) in dict.iter() {
        out.set(k.clone(), remap_object(v.clone(), map));
    }
    out
}

// ── Recherche d'objets ────────────────────────────────────────────────────────

fn dict_type(dict: &Dictionary) -> Option<&[u8]> {
    dict.get(b"Type").ok()?.as_name().ok()
}

/// Retourne l'ID du Catalog du document.
pub fn find_catalog(doc: &Document) -> Option<ObjectId> {
    doc.objects.iter().find_map(|(&id, obj)| {
        obj.as_dict()
            .ok()
            .filter(|d| dict_type(d) == Some(b"Catalog"))
            .map(|_| id)
    })
}

/// Retourne l'ID du nœud Pages racine (celui qui n'a pas de Parent).
pub fn find_pages_root(doc: &Document) -> Option<ObjectId> {
    doc.objects.iter().find_map(|(&id, obj)| {
        obj.as_dict()
            .ok()
            .filter(|d| dict_type(d) == Some(b"Pages") && d.get(b"Parent").is_err())
            .map(|_| id)
    })
}

// ── Construction de document ──────────────────────────────────────────────────

/// Copie tous les objets de `src` vers `dst` en excluant ceux dans `exclude`.
/// Alloue de nouveaux IDs dans `dst` et retourne le mapping old_id → new_id.
pub fn copy_objects(
    src: &Document,
    dst: &mut Document,
    exclude: &HashSet<ObjectId>,
) -> HashMap<ObjectId, ObjectId> {
    // Première passe : allouer les nouveaux IDs
    let mut id_map = HashMap::with_capacity(src.objects.len());
    for &old_id in src.objects.keys() {
        dst.max_id += 1;
        id_map.insert(old_id, (dst.max_id, 0));
    }
    // Deuxième passe : copier les objets remappés
    for (old_id, object) in &src.objects {
        if exclude.contains(old_id) {
            continue;
        }
        dst.objects
            .insert(id_map[old_id], remap_object(object.clone(), &id_map));
    }
    id_map
}

/// Met à jour le champ `Parent` de chaque page listée dans `page_ids`.
pub fn set_parent(doc: &mut Document, page_ids: &[ObjectId], parent_id: ObjectId) {
    for &id in page_ids {
        if let Some(Object::Dictionary(dict)) = doc.objects.get_mut(&id) {
            dict.set("Parent", Object::Reference(parent_id));
        }
    }
}

/// Insère un nœud Pages dans `doc` et retourne son ID.
/// `count` est le nombre total de pages feuilles (peut différer de `kids.len()`
/// si les kids sont eux-mêmes des nœuds Pages intermédiaires).
pub fn insert_pages_node(doc: &mut Document, kids: &[ObjectId], count: i64) -> ObjectId {
    let pages_id = alloc_id(doc);
    doc.objects.insert(
        pages_id,
        Object::Dictionary(Dictionary::from_iter(vec![
            ("Type",  Object::Name(b"Pages".to_vec())),
            ("Kids",  Object::Array(kids.iter().map(|&id| Object::Reference(id)).collect())),
            ("Count", Object::Integer(count)),
        ])),
    );
    pages_id
}

/// Insère un Catalog dans `doc` pointant vers `pages_id` et retourne son ID.
pub fn insert_catalog(doc: &mut Document, pages_id: ObjectId) -> ObjectId {
    let catalog_id = alloc_id(doc);
    doc.objects.insert(
        catalog_id,
        Object::Dictionary(Dictionary::from_iter(vec![
            ("Type",  Object::Name(b"Catalog".to_vec())),
            ("Pages", Object::Reference(pages_id)),
        ])),
    );
    catalog_id
}

/// Finalise le trailer et sérialise le document.
pub fn finalize(doc: &mut Document, catalog_id: ObjectId) -> Result<Vec<u8>> {
    doc.trailer.set("Root", Object::Reference(catalog_id));
    doc.trailer.set("Size", Object::Integer((doc.max_id + 1) as i64));
    save_to_bytes(doc)
}

fn alloc_id(doc: &mut Document) -> ObjectId {
    doc.max_id += 1;
    (doc.max_id, 0)
}

fn save_to_bytes(doc: &mut Document) -> Result<Vec<u8>> {
    let tmp = tempfile::NamedTempFile::new()?;
    doc.save(tmp.path())?;
    Ok(std::fs::read(tmp.path())?)
}

// ── Parsing des plages de pages ───────────────────────────────────────────────

/// Parse une chaîne de plages de pages en liste de numéros (1-indexés, ordonnés, sans doublons).
///
/// Exemples : `"1, 3, 5-7, 10"` → `[1, 3, 5, 6, 7, 10]`
pub fn parse_page_ranges(input: &str) -> Result<Vec<u32>> {
    let mut pages: Vec<u32> = Vec::new();

    for part in input.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((a_str, b_str)) = part.split_once('-') {
            let a = parse_page_number(a_str, part)?;
            let b = parse_page_number(b_str, part)?;
            if a > b {
                return Err(PdfError::InvalidRange(part.to_owned()));
            }
            pages.extend(a..=b);
        } else {
            pages.push(parse_page_number(part, part)?);
        }
    }

    if pages.is_empty() {
        return Err(PdfError::NoPages);
    }

    // Déduplique en conservant l'ordre de première apparition
    let mut seen = HashSet::new();
    pages.retain(|&p| seen.insert(p));

    Ok(pages)
}

fn parse_page_number(s: &str, context: &str) -> Result<u32> {
    s.trim()
        .parse()
        .map_err(|_| PdfError::InvalidPageNumber(context.to_owned()))
}
