//! Proof that unlinking a hidden popup pair removes only independently authorised annotations.

use std::collections::{BTreeMap, BTreeSet};

use pdf_syntax::{Dictionary, Document, Object, ObjectId};

use super::super::decision::Because;
use super::super::prepare::RemovedAnnotation;

const REFUSED: Because = Because::NotBuiltYet(
    "the linked annotation removal needs one closed reciprocal markup/Popup pair on one page, with both members independently hidden and discarded; malformed, shared, visible or interactive relationships are not inferred",
);

/// The page occurrence and every page annotation naming an object through Popup, Parent or IRT.
#[derive(Default)]
struct Graph {
    pages: BTreeMap<ObjectId, Vec<usize>>,
    incoming: BTreeMap<ObjectId, Vec<(Option<ObjectId>, &'static str)>>,
}

impl Graph {
    fn read(document: &Document) -> Result<Self, Because> {
        let mut graph = Self::default();
        for (page, value) in pdf_model::Pages::new(document)
            .every()
            .into_iter()
            .enumerate()
        {
            let value = value.ok_or(REFUSED)?;
            let annotations = document.get_key(&value.dict, "Annots");
            if annotations == Object::Null {
                continue;
            }
            for entry in annotations.as_array().ok_or(REFUSED)? {
                let id = entry.as_reference();
                if let Some(id) = id {
                    graph.pages.entry(id).or_default().push(page);
                }
                let value = document.resolve(entry);
                let dict = value.as_dict().ok_or(REFUSED)?;
                for key in ["Popup", "Parent", "IRT"] {
                    if let Some(target) = dict.get(key).and_then(Object::as_reference) {
                        graph.incoming.entry(target).or_default().push((id, key));
                    }
                }
            }
        }
        Ok(graph)
    }

    fn sole_link(&self, to: ObjectId, from: ObjectId, key: &'static str) -> bool {
        self.incoming
            .get(&to)
            .is_some_and(|links| links.as_slice() == [(Some(from), key)])
    }
}

/// Checks only where a discarded annotation has a relationship; unlinked policy is unchanged.
pub(super) fn prove(document: &Document, removed: &[RemovedAnnotation]) -> Result<(), Because> {
    let linked: Vec<_> = removed
        .iter()
        .filter(|row| {
            document
                .get(row.at)
                .as_dict()
                .is_some_and(|dict| dict.get("Popup").is_some() || dict.get("Parent").is_some())
        })
        .collect();
    if linked.is_empty() {
        return Ok(());
    }
    let graph = Graph::read(document)?;
    let hidden: BTreeSet<_> = removed
        .iter()
        .filter(|row| {
            row.flags
                .is_some_and(|flags| flags >= 0 && flags & (1 | 256) == 0 && flags & (2 | 32) != 0)
        })
        .map(|row| row.at)
        .collect();
    for row in linked {
        let object = document.get(row.at);
        let dict = object.as_dict().ok_or(REFUSED)?;
        let (parent, popup) = if named(document, dict, "Popup") {
            (reference(dict, "Parent")?, row.at)
        } else {
            (row.at, reference(dict, "Popup")?)
        };
        pair(document, &graph, &hidden, parent, popup)?;
    }
    Ok(())
}

fn reference(dict: &Dictionary, key: &str) -> Result<ObjectId, Because> {
    dict.get(key).and_then(Object::as_reference).ok_or(REFUSED)
}

fn named(document: &Document, dict: &Dictionary, name: &str) -> bool {
    document
        .get_key(dict, "Subtype")
        .as_name()
        .is_some_and(|value| value.as_bytes() == name.as_bytes())
}

fn pair(
    document: &Document,
    graph: &Graph,
    hidden: &BTreeSet<ObjectId>,
    parent: ObjectId,
    popup: ObjectId,
) -> Result<(), Because> {
    let parent_object = document.get(parent);
    let popup_object = document.get(popup);
    let parent_dict = parent_object.as_dict().ok_or(REFUSED)?;
    let popup_dict = popup_object.as_dict().ok_or(REFUSED)?;
    let parent_pages = graph.pages.get(&parent).ok_or(REFUSED)?;
    let popup_pages = graph.pages.get(&popup).ok_or(REFUSED)?;
    // Table 171's markup subtypes with popup semantics; §12.5.6.2 excludes Sound.
    let subtype = document.get_key(parent_dict, "Subtype");
    let markup = subtype.as_name().is_some_and(|name| {
        matches!(
            name.as_bytes(),
            b"Text"
                | b"FreeText"
                | b"Line"
                | b"Square"
                | b"Circle"
                | b"Polygon"
                | b"PolyLine"
                | b"Highlight"
                | b"Underline"
                | b"Squiggly"
                | b"StrikeOut"
                | b"Stamp"
                | b"Caret"
                | b"Ink"
                | b"FileAttachment"
                | b"Redact"
        )
    });
    if parent == popup
        || !markup
        || !named(document, popup_dict, "Popup")
        || !hidden.contains(&parent)
        || !hidden.contains(&popup)
        || parent_pages.len() != 1
        || parent_pages != popup_pages
        || reference(parent_dict, "Popup")? != popup
        || reference(popup_dict, "Parent")? != parent
        || !graph.sole_link(popup, parent, "Popup")
        || !graph.sole_link(parent, popup, "Parent")
        || parent_dict.get("Parent").is_some()
        || popup_dict.get("Popup").is_some()
        || ["A", "AA", "IRT", "StructParent"]
            .iter()
            .any(|key| parent_dict.get(key).is_some() || popup_dict.get(key).is_some())
        || popup_dict.get("AP").is_some()
        || [parent_dict, popup_dict].iter().any(|dict| {
            !matches!(
                document.get_key(dict, "Open"),
                Object::Null | Object::Boolean(false)
            )
        })
    {
        return Err(REFUSED);
    }
    Ok(())
}
