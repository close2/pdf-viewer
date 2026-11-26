//! What else in the file carries what the region held, and what the removal does with each
//! (ISO 32000-2 §12.5.6.23, ADR 3725).
//!
//! The clause binds two things beyond the content stream. "\[T]he content in the area specified
//! by the redact annotations" is removed — and an annotation whose rectangle meets the region
//! draws in that area, a widget's appearance shows its field's value there, and a structure
//! element's `/ActualText`, `/Alt` and `/E` stand in for the content it marks. And the processor
//! "shall remove all traces of the specified content" and "shall also be diligent in their
//! consideration of all content that can exist in a PDF document". So:
//!
//! - **An annotation whose `/Rect` meets the region** is removed with the `/Redact` annotations
//!   (§12.5.2's `/Rect` is where it draws), and so is a popup whose `/Parent` is one of them.
//!   Every removed annotation is written as the null object, so that no other reference to it — a
//!   popup's `/Parent`, a reply's `/IRT`, a field's `/Kids`, an `OBJR` — copies it back in.
//! - **A field whose widget was removed** loses `/V`, `/RV` and `/DV`: the value is what the
//!   widget drew in the region, and the default is the value a reset would draw there again.
//! - **A structure element** marking a sequence the removal reached loses `/ActualText`, `/Alt`,
//!   `/E` and `/T`, and every element above it loses the first three, which cover their whole
//!   subtree (§14.9.4: the actual text "shall be used as a replacement, not a description, for
//!   the content"). Whether all of the element's content was removed or part of it is reported,
//!   never decided silently: the choice the clause leaves — keep a part's description or drop it
//!   — is taken on the side that leaks nothing.
//!
//! **What the clause does not bind**: an outline item's title, a named destination, an embedded
//! file, an optional content group's name, the document's own XMP packet and information
//! dictionary, an annotation outside the region. None of them is content in the area, and
//! rewriting what a producer wrote elsewhere because it repeats the region's words would be this
//! program deciding what else the reader meant to remove. Diligence is taken as the duty to
//! *look*: every written object outside the region is read for the text the removal took, and
//! each place that still holds it is named in a [`crate::Warning`] — by its path from the
//! catalog, never by quoting the text, which would be one more copy of it — so that redacting it
//! there is the reader's decision rather than an accident.

use std::collections::{HashMap, HashSet, VecDeque};

use pdf_model::content::Interpretation;
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Object, ObjectId};

use super::marked::{DESCRIBING, Reached};
use super::{MAX_DEPTH, numbers, overlaps};

/// The entries of a field that hold or restore its value (Table 226, Table 228).
const VALUE_ENTRIES: [&[u8]; 3] = [b"V", b"RV", b"DV"];

/// The `/Redact` annotations of `page`, which §12.5.6.23 removes: "the redact annotations are
/// removed from the PDF document".
pub(super) fn redactions(document: &Document, page: &Dictionary) -> Vec<ObjectId> {
    let annots = document.get_key(page, "Annots");
    let Some(items) = annots.as_array() else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            let id = item.as_reference()?;
            let resolved = document.resolve(item);
            let subtype = document.get_key(resolved.as_dict()?, "Subtype");
            subtype
                .as_name()
                .is_some_and(|name| name.as_bytes() == b"Redact")
                .then_some(id)
        })
        .collect()
}

/// Whether an annotation's `/Rect` (§12.5.2, default user space) meets one of `boxes`.
pub(super) fn meets(document: &Document, annotation: &Dictionary, boxes: &[[f32; 4]]) -> bool {
    let Some(rect) = numbers(document, annotation, "Rect").filter(|rect| rect.len() == 4) else {
        return false;
    };
    let rect = [
        rect[0].min(rect[2]),
        rect[1].min(rect[3]),
        rect[0].max(rect[2]),
        rect[1].max(rect[3]),
    ];
    boxes.iter().any(|region| overlaps(*region, rect))
}

/// The annotations of `page` whose `/Rect` meets one of `boxes` (default user space).
pub(super) fn in_region(
    document: &Document,
    page: &Dictionary,
    boxes: &[[f32; 4]],
) -> Vec<ObjectId> {
    let annots = document.get_key(page, "Annots");
    let Some(items) = annots.as_array() else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            let id = item.as_reference()?;
            let resolved = document.resolve(item);
            meets(document, resolved.as_dict()?, boxes).then_some(id)
        })
        .collect()
}

/// The report's phrase for the annotations removed with the region beside the `/Redact` ones.
pub(super) fn removed_phrase(document: &Document, removed: &[ObjectId]) -> String {
    let numbers: Vec<String> = removed.iter().map(|id| id.number.to_string()).collect();
    let widgets: Vec<String> = removed
        .iter()
        .filter(|id| {
            document.get(**id).as_dict().is_some_and(|dict| {
                document
                    .get_key(dict, "Subtype")
                    .as_name()
                    .is_some_and(|name| name.as_bytes() == b"Widget")
            })
        })
        .map(|id| id.number.to_string())
        .collect();
    let fields = if widgets.is_empty() {
        String::new()
    } else {
        format!(
            "; {} of them widget(s), each field's value written nowhere with it",
            widgets.join(", ")
        )
    };
    format!(
        "annotation(s) {} drew in the region, or were the popup of one that did or of a /Redact \
         annotation, and were removed with it{fields}",
        numbers.join(", ")
    )
}

/// `removed` with the popups of its annotations added: each one's `/Popup`, and each annotation
/// of `page` whose `/Parent` is one of them.
pub(super) fn with_popups(document: &Document, page: &Dictionary, removed: &mut Vec<ObjectId>) {
    let mut popups = Vec::new();
    for id in removed.iter() {
        if let Some(dict) = document.get(*id).as_dict()
            && let Some(popup) = dict.get("Popup").and_then(Object::as_reference)
        {
            popups.push(popup);
        }
    }
    let annots = document.get_key(page, "Annots");
    for item in annots.as_array().map_or_else(Vec::new, <[Object]>::to_vec) {
        let Some(id) = item.as_reference() else {
            continue;
        };
        let parent = document
            .get(id)
            .as_dict()
            .and_then(|dict| dict.get("Parent").and_then(Object::as_reference));
        if parent.is_some_and(|parent| removed.contains(&parent)) {
            popups.push(id);
        }
    }
    for popup in popups {
        if !removed.contains(&popup) {
            removed.push(popup);
        }
    }
}

/// One object written with entries left out: its new dictionary, and what to say of it.
pub(super) struct Restated {
    pub(super) id: ObjectId,
    pub(super) dict: Dictionary,
    pub(super) detail: String,
}

/// `dict` without `keys`, and the keys it held among them, or `None` where it held none.
fn without(dict: &Dictionary, keys: &[&[u8]]) -> Option<(Dictionary, Vec<String>)> {
    let mut out = Dictionary::new();
    let mut dropped = Vec::new();
    for (key, value) in dict.iter() {
        if keys.contains(&key.as_bytes()) {
            if *value != Object::Null {
                dropped.push(format!("/{}", String::from_utf8_lossy(key.as_bytes())));
            }
        } else {
            out.insert(key.clone(), value.clone());
        }
    }
    (!dropped.is_empty()).then_some((out, dropped))
}

/// The fields whose value a removed widget showed: each field, from the widget up its `/Parent`
/// chain, that states a value, written without it (Table 226's inheritable `/V` and `/DV`).
pub(super) fn field_values(document: &Document, removed: &HashSet<ObjectId>) -> Vec<Restated> {
    let mut out: Vec<Restated> = Vec::new();
    let mut ordered: Vec<ObjectId> = removed.iter().copied().collect();
    ordered.sort_by_key(|id| id.number);
    for widget in ordered {
        let is_widget = document.get(widget).as_dict().is_some_and(|dict| {
            document
                .get_key(dict, "Subtype")
                .as_name()
                .is_some_and(|name| name.as_bytes() == b"Widget")
        });
        if !is_widget {
            continue;
        }
        let mut at = document
            .get(widget)
            .as_dict()
            .and_then(|dict| dict.get("Parent").and_then(Object::as_reference));
        let mut depth = 0;
        while let Some(field) = at
            && depth < MAX_DEPTH
        {
            depth = depth.saturating_add(1);
            let Some(dict) = document.get(field).as_dict().cloned() else {
                break;
            };
            if !removed.contains(&field)
                && !out.iter().any(|done| done.id == field)
                && let Some((dict, dropped)) = without(&dict, &VALUE_ENTRIES)
            {
                out.push(Restated {
                    id: field,
                    dict,
                    detail: format!(
                        "field {}'s {} not written; its widget {} met the region and was removed",
                        field.number,
                        dropped.join(" and "),
                        widget.number
                    ),
                });
            }
            at = dict.get("Parent").and_then(Object::as_reference);
        }
    }
    out
}

/// One content item the removal reached: the page and stream that hold it, and its identifier.
#[derive(Clone, Copy)]
pub(super) struct Item {
    pub(super) page: ObjectId,
    pub(super) reached: Reached,
}

/// What one structure element's content items amount to.
#[derive(Default)]
struct Marks {
    items: usize,
    whole: usize,
    reached: usize,
}

/// An element's `/K`, as a list (Table 355: one content item or kid, or an array of them).
fn kids(document: &Document, dict: &Dictionary) -> Vec<Object> {
    match document.get_key(dict, "K") {
        Object::Array(items) => items.clone(),
        Object::Null => Vec::new(),
        other => vec![other],
    }
}

/// What element `dict`'s content items amount to against what the removal reached (§14.7.5),
/// its child elements added to `children`. `page` is the page it states or inherits.
fn marks_of(
    document: &Document,
    dict: &Dictionary,
    page: Option<ObjectId>,
    items: &[Item],
    removed: &HashSet<ObjectId>,
    children: &mut Vec<ObjectId>,
) -> Marks {
    let reached = |page: Option<ObjectId>, stream: Option<ObjectId>, mcid: i64| {
        items.iter().find(|item| {
            Some(item.page) == page && item.reached.stream == stream && item.reached.mcid == mcid
        })
    };
    let mut marks = Marks::default();
    let mut count = |hit: Option<bool>| {
        marks.items = marks.items.saturating_add(1);
        if let Some(whole) = hit {
            marks.reached = marks.reached.saturating_add(1);
            marks.whole = marks.whole.saturating_add(usize::from(whole));
        }
    };
    for kid in kids(document, dict) {
        match document.resolve(&kid) {
            Object::Integer(mcid) => {
                count(reached(page, None, mcid).map(|item| item.reached.whole));
            }
            Object::Dictionary(item) if item.get("MCID").is_some() => {
                let at = item.get("Pg").and_then(Object::as_reference).or(page);
                let stream = item.get("Stm").and_then(Object::as_reference);
                let mcid = document.get_key(&item, "MCID").as_integer();
                count(
                    mcid.and_then(|mcid| reached(at, stream, mcid))
                        .map(|item| item.reached.whole),
                );
            }
            Object::Dictionary(item) if item.get("Obj").is_some() => {
                let object = item.get("Obj").and_then(Object::as_reference);
                count(
                    object
                        .is_some_and(|object| removed.contains(&object))
                        .then_some(true),
                );
            }
            Object::Dictionary(_) => children.extend(kid.as_reference()),
            _ => {}
        }
    }
    marks
}

/// The structure elements whose content the removal reached, and their ancestors, each written
/// without what describes that content (§14.7.2 Table 355, §14.9.3–§14.9.5).
pub(super) fn elements(
    document: &Document,
    catalog: &Dictionary,
    items: &[Item],
    removed: &HashSet<ObjectId>,
) -> Vec<Restated> {
    let Some(root) = catalog.get("StructTreeRoot") else {
        return Vec::new();
    };
    // Breadth first from the root, each element with the page it inherits and its parent.
    let mut queue: VecDeque<(ObjectId, Option<ObjectId>, Option<ObjectId>)> = VecDeque::new();
    let mut seen: HashSet<ObjectId> = HashSet::new();
    let mut parents: HashMap<ObjectId, ObjectId> = HashMap::new();
    let mut marked: Vec<(ObjectId, Marks)> = Vec::new();
    let tree = document.resolve(root);
    let enqueue = |list: &[Object], page, parent, queue: &mut VecDeque<_>| {
        for kid in list {
            if let Some(id) = kid.as_reference() {
                queue.push_back((id, page, parent));
            }
        }
    };
    if let Some(dict) = tree.as_dict() {
        enqueue(&kids(document, dict), None, None, &mut queue);
    }
    while let Some((id, inherited, parent)) = queue.pop_front() {
        if !seen.insert(id) {
            continue;
        }
        let object = document.get(id);
        let Some(dict) = object.as_dict() else {
            continue;
        };
        if let Some(parent) = parent {
            parents.insert(id, parent);
        }
        let page = dict.get("Pg").and_then(Object::as_reference).or(inherited);
        let mut children = Vec::new();
        let marks = marks_of(document, dict, page, items, removed, &mut children);
        queue.extend(children.into_iter().map(|child| (child, page, Some(id))));
        if marks.reached > 0 {
            marked.push((id, marks));
        }
    }

    let mut out: Vec<Restated> = Vec::new();
    let mut done: HashSet<ObjectId> = HashSet::new();
    for (id, marks) in marked {
        let whole = marks.whole == marks.items;
        let mut keys: Vec<&[u8]> = DESCRIBING.to_vec();
        keys.push(b"T");
        restate(document, id, &keys, whole, &mut done, &mut out);
        let mut at = parents.get(&id).copied();
        let mut depth = 0;
        while let Some(ancestor) = at
            && depth < MAX_DEPTH
        {
            depth = depth.saturating_add(1);
            restate(document, ancestor, &DESCRIBING, false, &mut done, &mut out);
            at = parents.get(&ancestor).copied();
        }
    }
    out
}

/// Adds element `id` written without `keys` to `out`, once.
fn restate(
    document: &Document,
    id: ObjectId,
    keys: &[&[u8]],
    whole: bool,
    done: &mut HashSet<ObjectId>,
    out: &mut Vec<Restated>,
) {
    if !done.insert(id) {
        return;
    }
    let Some(dict) = document.get(id).as_dict().cloned() else {
        return;
    };
    let Some((dict, dropped)) = without(&dict, keys) else {
        return;
    };
    let how = if whole {
        "every content item it marks was removed"
    } else {
        "part of the content it covers was removed, and the description covers that part too"
    };
    out.push(Restated {
        id,
        dict,
        detail: format!(
            "structure element {}'s {} not written; {how}",
            id.number,
            dropped.join(" and ")
        ),
    });
}

/// The text the removal took: for each stretch of consecutive codes it removed, the readback
/// from the first code's start to the last code's end, one run per line of it, each trimmed.
///
/// The range rather than the codes' own spans joined, because a sequence with §14.9.4's
/// `/ActualText` reads back its replacement once for all of its codes, and the separators the
/// interpreter inferred between words belong to the run.
pub(super) fn removed_runs(interpretation: &Interpretation, removed: &[usize]) -> Vec<String> {
    let mut indices: Vec<usize> = removed.to_vec();
    indices.sort_unstable();
    indices.dedup();
    let mut ranges: Vec<std::ops::Range<usize>> = Vec::new();
    let mut last: Option<usize> = None;
    for index in indices {
        let Some(span) = interpretation
            .text_layer
            .get(index)
            .map(|placed| placed.span.clone())
        else {
            continue;
        };
        match ranges.last_mut() {
            Some(range) if last.is_some_and(|last| last.saturating_add(1) == index) => {
                range.start = range.start.min(span.start);
                range.end = range.end.max(span.end);
            }
            _ => ranges.push(span),
        }
        last = Some(index);
    }
    let mut out: Vec<String> = ranges
        .into_iter()
        .filter_map(|range| interpretation.text.get(range))
        .flat_map(str::lines)
        .map(|run| run.trim().to_owned())
        .filter(|run| run.chars().any(char::is_alphanumeric))
        .collect();
    out.sort();
    out.dedup();
    out
}

/// Whether `text` holds `run` as words of its own: neither end continues a word of `text`.
fn holds(text: &str, run: &str) -> bool {
    text.match_indices(run).any(|(at, _)| {
        let before = text.get(..at).and_then(|head| head.chars().next_back());
        let after = text
            .get(at.saturating_add(run.len())..)
            .and_then(|tail| tail.chars().next());
        !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
    })
}

/// `bytes` read every way a PDF writes text: as §7.9.2.2 text, as Latin-1, as UTF-8, and as
/// UTF-16 of either byte order.
fn readings(bytes: &[u8]) -> Vec<String> {
    let units = |big: bool| -> String {
        let units: Vec<u16> = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| {
                if big {
                    u16::from_be_bytes(*pair)
                } else {
                    u16::from_le_bytes(*pair)
                }
            })
            .collect();
        String::from_utf16_lossy(&units)
    };
    vec![
        pdf_syntax::text_string(bytes),
        bytes.iter().copied().map(char::from).collect(),
        String::from_utf8_lossy(bytes).into_owned(),
        units(true),
        units(false),
    ]
}

/// Whether any string `object` holds, at any depth, holds a run.
fn strings_hold(object: &Object, runs: &[String], depth: usize) -> bool {
    if depth >= MAX_DEPTH {
        return false;
    }
    let next = depth.saturating_add(1);
    match object {
        Object::String(bytes) => readings(bytes)
            .iter()
            .any(|text| runs.iter().any(|run| holds(text, run))),
        Object::Array(items) => items.iter().any(|item| strings_hold(item, runs, next)),
        Object::Dictionary(dict) => dict
            .iter()
            .any(|(_, value)| strings_hold(value, runs, next)),
        Object::Stream(stream) => stream
            .dict
            .iter()
            .any(|(_, value)| strings_hold(value, runs, next)),
        _ => false,
    }
}

/// Whether a stream is one whose data is text a reader is given as such: a §14.3.2 metadata
/// packet or a §7.11.4 embedded file. A content stream's text is the page's, and the page outside
/// the region is the reader's to see; a font program or an image holds no words.
fn is_text_data(document: &Document, dict: &Dictionary) -> bool {
    document
        .get_key(dict, "Type")
        .as_name()
        .is_some_and(|name| matches!(name.as_bytes(), b"Metadata" | b"EmbeddedFile"))
}

/// Each place, outside the region, of the objects the output carries unchanged that still holds
/// one of `runs`: one phrase each, naming the object and its path from the catalog.
pub(super) fn survivors(
    document: &Document,
    root: ObjectId,
    written: &dyn Fn(ObjectId) -> bool,
    runs: &[String],
) -> Vec<String> {
    if runs.is_empty() {
        return Vec::new();
    }
    // Each object's first path from the catalog (or the trailer's `/Info`), breadth first.
    let mut paths: HashMap<ObjectId, String> = HashMap::new();
    let mut queue: VecDeque<(Object, String, usize)> = VecDeque::new();
    queue.push_back((Object::Reference(root), String::new(), 0));
    if let Some(info) = document.trailer().get("Info") {
        queue.push_back((info.clone(), "trailer /Info".to_owned(), 0));
    }
    let mut out = Vec::new();
    let mut seen: HashSet<ObjectId> = HashSet::new();
    while let Some((object, path, depth)) = queue.pop_front() {
        if depth >= MAX_DEPTH || path.len() > 512 {
            continue;
        }
        let next = depth.saturating_add(1);
        match object {
            Object::Reference(id) => {
                if !seen.insert(id) {
                    continue;
                }
                let resolved = document.get(id);
                if written(id) {
                    paths.insert(id, path.clone());
                    let place = if path.is_empty() {
                        "the catalog"
                    } else {
                        &path
                    };
                    if strings_hold(&resolved, runs, 0) {
                        out.push(format!("object {} at {place}", id.number));
                    } else if let Some(stream) = resolved.as_stream()
                        && is_text_data(document, &stream.dict)
                        && let Some(data) = document.decoded_stream_data(stream)
                        && readings(&data)
                            .iter()
                            .any(|text| runs.iter().any(|run| holds(text, run)))
                    {
                        out.push(format!("the stream object {} at {place}", id.number));
                    }
                }
                queue.push_back((resolved, path, next));
            }
            Object::Array(items) => {
                for item in &items {
                    queue.push_back((item.clone(), format!("{path}[]"), next));
                }
            }
            Object::Dictionary(dict) => {
                if depth == 0 && strings_hold(&Object::Dictionary(dict.clone()), runs, 0) {
                    out.push("the information dictionary at trailer /Info".to_owned());
                }
                for (key, value) in dict.iter() {
                    let key = String::from_utf8_lossy(key.as_bytes());
                    queue.push_back((value.clone(), format!("{path} /{key}"), next));
                }
            }
            Object::Stream(stream) => {
                for (key, value) in stream.dict.iter() {
                    let key = String::from_utf8_lossy(key.as_bytes());
                    queue.push_back((value.clone(), format!("{path} /{key}"), next));
                }
            }
            _ => {}
        }
    }
    out
}
