//! Proof that unlinking a hidden popup pair removes only independently authorised annotations.
//!
//! ISO 19005-2 section 6.3.2 and ISO 19005-4 section 6.3.2 require every annotation but a `Popup`
//! to set the Print flag and clear Hidden, Invisible, `ToggleNoView` and `NoView`, so a hidden
//! markup annotation is discarded when its shape's remedy says so. Its popup is tied to it both
//! ways: ISO 32000-2 §12.5.6.2's Table 172 gives the markup annotation `/Popup`, "\[a]n indirect
//! reference to a popup annotation for entering or editing the text associated with this
//! annotation", and §12.5.6.14's Table 186 gives the popup `/Parent`. Removing one member leaves
//! the other naming an object no page holds, so the pair leaves together or not at all.

use std::collections::{BTreeMap, BTreeSet};

use pdf_syntax::{Dictionary, Document, Object, ObjectId};

use super::super::decision::Because;
use super::super::prepare::RemovedAnnotation;

const REFUSED: Because = Because::NotBuiltYet(
    "the linked annotation removal needs one closed reciprocal markup/Popup pair on one page, with both members independently hidden and discarded; malformed, shared, visible or interactive relationships are not inferred",
);

/// The page occurrence and every page annotation naming an object through Popup, Parent or IRT:
/// ISO 32000-2 Table 172's `/Popup` and `/IRT` (§12.5.6.2) and Table 186's `/Parent`
/// (§12.5.6.14), the three entries by which one annotation names another.
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
        // ISO 32000-2 §12.5.3's Table 167 bit positions: Invisible (bit 1, value 1) and
        // ToggleNoView (bit 9, value 256) clear, and Hidden (bit 2, value 2) or NoView (bit 6,
        // value 32) set, so the annotation is not drawn on screen whatever a pointer does.
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
    // ISO 32000-2 Table 171's markup subtypes with popup semantics; §12.5.6.2 excludes Sound.
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
        // Table 186's popup `/Open` and Table 175's text annotation `/Open` both default to
        // false (closed); a pair a reader would display open is not a hidden pair.
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
