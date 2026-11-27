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
//!   — is taken on the side that leaks nothing. An element written directly inside its parent's
//!   `/K` is restated inside the object that holds it (ADR 3809), or inside the structure tree
//!   root where the catalog states the root directly (ADR 3845), and an element whose form
//!   sequence is in a placement copy of the form gains a reference naming the copy by its own
//!   number wherever the copy still holds the sequence (ADRs 3811 and 3846).
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
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};

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

/// One object written with entries left out: its new dictionary, and what to say of it. Where
/// `id` is the catalog, `dict` is the structure tree root the catalog states directly ([`Place`]).
pub(super) struct Restated {
    pub(super) id: ObjectId,
    pub(super) dict: Dictionary,
    pub(super) detail: String,
    /// Content items to add to elements of `dict`, each at the place of its element below the
    /// object (see [`Place`]), naming a copy only the writer has a number for (ADR 3811).
    pub(super) added: Vec<(Vec<usize>, Copied)>,
    /// Objects of the output, by their numbers there, to add at the end of `dict`'s `/Kids`: a
    /// parent tree's new leaf (ADR 3846).
    pub(super) kids_added: Vec<ObjectId>,
}

/// Adds to the element at `path` below `dict` one content item per `items` entry: a §14.7.5.2
/// marked-content reference with `/Stm` the given copy's number in the output, `/Pg` its page's.
pub(super) fn add_references(
    dict: &mut Dictionary,
    path: &[usize],
    references: &[(ObjectId, ObjectId, i64)],
) {
    at_path(dict, path, &mut |element| {
        let mut list = match element.remove("K") {
            Some(Object::Array(items)) => items,
            Some(Object::Null) | None => Vec::new(),
            Some(single) => vec![single],
        };
        for (page, stream, mcid) in references {
            let mut reference = Dictionary::new();
            reference.insert(
                Name::new(&b"Type"[..]),
                Object::Name(Name::new(&b"MCR"[..])),
            );
            reference.insert(Name::new(&b"Pg"[..]), Object::Reference(*page));
            reference.insert(Name::new(&b"Stm"[..]), Object::Reference(*stream));
            reference.insert(Name::new(&b"MCID"[..]), Object::Integer(*mcid));
            list.push(Object::Dictionary(reference));
        }
        element.insert(Name::new(&b"K"[..]), Object::Array(list));
        Some(())
    });
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
                    added: Vec::new(),
                    kids_added: Vec::new(),
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
    /// The placement copies still holding one of its sequences, whole or in part (ADRs 3811
    /// and 3846).
    copies: Vec<Copied>,
}

/// A marked-content sequence of a form written in a placement copy of its own (ADR 1363) that
/// the copy still holds, whole or in part: the structure element that marks the form's sequence
/// is given a reference to the copy's (§14.7.5.2's `/Stm`), since no reference names the copy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Copied {
    /// The page the copy is drawn on.
    pub(super) page: ObjectId,
    /// The page's form edit the copy is.
    pub(super) copy: usize,
    /// The sequence's `/MCID` in the copy's stream, which is the form's.
    pub(super) mcid: i64,
}

/// An element's `/K`, as a list (Table 355: one content item or kid, or an array of them).
fn kids(document: &Document, dict: &Dictionary) -> Vec<Object> {
    match document.get_key(dict, "K") {
        Object::Array(items) => items.clone(),
        Object::Null => Vec::new(),
        other => vec![other],
    }
}

/// Where one structure element is written: the indirect object that holds it, and the positions
/// in `/K` that lead from that object's dictionary down to it through elements written directly
/// (§14.7.2 Table 355 lets `/K` hold a kid as a dictionary as well as a reference).
///
/// An element written directly has no object of its own to be restated in, so it is restated
/// inside its holder: the holder is written again with the element's entries removed at its
/// place.
///
/// A structure tree root written directly in the catalog (Table 29 does not require the root to
/// be indirect) holds its direct kids the same way: their holder is the catalog, `in_catalog` says
/// that the path starts at the catalog's `/StructTreeRoot` rather than at the holder's own
/// dictionary, and the restated root is written into the catalog (ADR 3845).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
struct Place {
    holder: ObjectId,
    path: Vec<usize>,
    in_catalog: bool,
}

impl Place {
    /// The place of the kid at `position` of this element's `/K`, `kid` as written there.
    fn kid(&self, kid: &Object, position: usize) -> Option<Self> {
        match kid {
            Object::Reference(id) => Some(Self {
                holder: *id,
                path: Vec::new(),
                in_catalog: false,
            }),
            Object::Dictionary(_) => {
                let mut path = self.path.clone();
                path.push(position);
                Some(Self {
                    holder: self.holder,
                    path,
                    in_catalog: self.in_catalog,
                })
            }
            _ => None,
        }
    }

    /// The report's name for the element.
    fn named(&self) -> String {
        let at = self
            .path
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        if self.in_catalog {
            format!(
                "the structure element written directly in the /K of the structure tree root \
                 the catalog (object {}) states directly (at {at})",
                self.holder.number
            )
        } else if self.path.is_empty() {
            format!("structure element {}", self.holder.number)
        } else {
            format!(
                "the structure element written directly in object {}'s /K (at {at})",
                self.holder.number
            )
        }
    }
}

/// Runs `edit` on the element at `path` below `dict`, followed through `/K` (see [`Place`]), and
/// answers what it answered, or `None` where `path` leads to no element.
fn at_path<T>(
    dict: &mut Dictionary,
    path: &[usize],
    edit: &mut dyn FnMut(&mut Dictionary) -> Option<T>,
) -> Option<T> {
    let Some((first, rest)) = path.split_first() else {
        return edit(dict);
    };
    let mut k = dict.remove("K")?;
    let kid = match &mut k {
        Object::Array(items) => items.get_mut(*first),
        single @ Object::Dictionary(_) if *first == 0 => Some(single),
        _ => None,
    };
    let answer = match kid {
        Some(Object::Dictionary(kid)) => at_path(kid, rest, edit),
        _ => None,
    };
    dict.insert(Name::new(&b"K"[..]), k);
    answer
}

/// What element `dict`'s content items amount to against what the removal reached (§14.7.5),
/// its child elements, with their places below `place`, added to `children`. `page` is the page
/// it states or inherits.
fn marks_of(
    document: &Document,
    dict: &Dictionary,
    (place, page): (&Place, Option<ObjectId>),
    (items, placements): (&[Item], &[(ObjectId, usize, ObjectId)]),
    removed: &HashSet<ObjectId>,
    children: &mut Vec<(Place, Dictionary)>,
) -> Marks {
    // The form's own stream as written: a sequence reached in a placement copy is not in it.
    let reached = |page: Option<ObjectId>, stream: Option<ObjectId>, mcid: i64| {
        items.iter().find(|item| {
            Some(item.page) == page
                && item.reached.stream == stream
                && item.reached.mcid == mcid
                && item.reached.copy.is_none()
        })
    };
    let mut copies: Vec<Copied> = Vec::new();
    let mut marks = Marks::default();
    let mut count = |hit: Option<bool>| {
        marks.items = marks.items.saturating_add(1);
        if let Some(whole) = hit {
            marks.reached = marks.reached.saturating_add(1);
            marks.whole = marks.whole.saturating_add(usize::from(whole));
        }
    };
    for (position, kid) in kids(document, dict).iter().enumerate() {
        match document.resolve(kid) {
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
                // A form drawn more than once and written as copies (ADR 1363): the sequence is
                // in each copy too, one content item more for each, and a copy that still shows
                // part of it is named to the element by the copy's own number (ADR 3811).
                let mut seen: Vec<usize> = Vec::new();
                for copied in items.iter().filter(|item| {
                    Some(item.page) == at
                        && item.reached.stream == stream
                        && Some(item.reached.mcid) == mcid
                }) {
                    let Some(copy) = copied.reached.copy else {
                        continue;
                    };
                    if seen.contains(&copy) {
                        continue;
                    }
                    seen.push(copy);
                    count(Some(copied.reached.whole));
                    if !copied.reached.whole {
                        copies.push(Copied {
                            page: copied.page,
                            copy,
                            mcid: copied.reached.mcid,
                        });
                    }
                }
                // A copy the removal reached no sequence of this element in holds the sequence
                // as the form does: one content item more, not reached, named the same way.
                for (copy_page, copy, _) in placements.iter().filter(|(copy_page, copy, form)| {
                    Some(*copy_page) == at && Some(*form) == stream && !seen.contains(copy)
                }) {
                    let Some(mcid) = mcid else {
                        break;
                    };
                    count(None);
                    copies.push(Copied {
                        page: *copy_page,
                        copy: *copy,
                        mcid,
                    });
                }
            }
            Object::Dictionary(item) if item.get("Obj").is_some() => {
                let object = item.get("Obj").and_then(Object::as_reference);
                count(
                    object
                        .is_some_and(|object| removed.contains(&object))
                        .then_some(true),
                );
            }
            Object::Dictionary(child) => {
                children.extend(place.kid(kid, position).map(|place| (place, child)));
            }
            _ => {}
        }
    }
    marks.copies = copies;
    marks
}

/// One element to be written without entries: which, and whether all of its content went.
struct Restatement {
    place: Place,
    keys: Vec<&'static [u8]>,
    whole: bool,
    copies: Vec<Copied>,
}

/// The structure elements whose content the removal reached, and their ancestors, each written
/// without what describes that content (§14.7.2 Table 355, §14.9.3–§14.9.5) — an element written
/// directly in its parent's `/K` restated inside the object that holds it.
///
/// Where the catalog (object `catalog_id`) states the structure tree root directly, the elements
/// written directly in the root's `/K` are restated inside it, and the answer for them carries
/// `catalog_id` with the restated *root* as its dictionary: the caller writes it into the catalog.
///
/// `placements` are the placement copies of forms (ADR 1363), each as its page, the page's form
/// edit and the form: an element marking a sequence of the form is given a reference to each
/// copy that still holds the sequence, reached or not, so that the structure tree names every
/// content item the copy's own parent-tree entry names (§14.7.5.4, ADR 3846).
pub(super) fn elements(
    document: &Document,
    (catalog_id, catalog): (ObjectId, &Dictionary),
    (items, placements): (&[Item], &[(ObjectId, usize, ObjectId)]),
    removed: &HashSet<ObjectId>,
) -> Vec<Restated> {
    let Some(root) = catalog.get("StructTreeRoot") else {
        return Vec::new();
    };
    let tree = document.resolve(root);
    let Some(tree) = tree.as_dict() else {
        return Vec::new();
    };
    // Breadth first from the root, each element with its place, the page it inherits and its
    // parent's place.
    let mut queue = root_kids(document, tree, root.as_reference().ok_or(catalog_id));
    let mut seen: HashSet<Place> = HashSet::new();
    let mut parents: HashMap<Place, Place> = HashMap::new();
    let mut marked: Vec<(Place, Marks)> = Vec::new();
    // Elements the removal did not reach that only gain references to placement copies.
    let mut named: Vec<(Place, Marks)> = Vec::new();
    while let Some((place, dict, inherited, parent)) = queue.pop_front() {
        if place.path.len() > MAX_DEPTH || !seen.insert(place.clone()) {
            continue;
        }
        if let Some(parent) = parent {
            parents.insert(place.clone(), parent);
        }
        let page = dict.get("Pg").and_then(Object::as_reference).or(inherited);
        let mut children = Vec::new();
        let marks = marks_of(
            document,
            &dict,
            (&place, page),
            (items, placements),
            removed,
            &mut children,
        );
        queue.extend(
            children
                .into_iter()
                .map(|(child, dict)| (child, dict, page, Some(place.clone()))),
        );
        if marks.reached > 0 {
            marked.push((place, marks));
        } else if !marks.copies.is_empty() {
            named.push((place, marks));
        }
    }

    let mut wanted: Vec<Restatement> = Vec::new();
    let mut want = |place: &Place, keys: &[&'static [u8]], marks: Option<&Marks>| {
        let at = if let Some(at) = wanted.iter().position(|item| item.place == *place) {
            at
        } else {
            wanted.push(Restatement {
                place: place.clone(),
                keys: Vec::new(),
                whole: false,
                copies: Vec::new(),
            });
            wanted.len().saturating_sub(1)
        };
        if let Some(item) = wanted.get_mut(at) {
            for key in keys {
                if !item.keys.contains(key) {
                    item.keys.push(key);
                }
            }
            if let Some(marks) = marks {
                item.whole |= marks.whole == marks.items;
                item.copies.extend(marks.copies.iter().copied());
            }
        }
    };
    for (place, marks) in &marked {
        let mut keys: Vec<&'static [u8]> = DESCRIBING.to_vec();
        keys.push(b"T");
        want(place, &keys, Some(marks));
        let mut at = parents.get(place);
        let mut depth = 0;
        while let Some(ancestor) = at
            && depth < MAX_DEPTH
        {
            depth = depth.saturating_add(1);
            want(ancestor, &DESCRIBING, None);
            at = parents.get(ancestor);
        }
    }
    for (place, marks) in &named {
        want(place, &[], Some(marks));
    }
    restate(document, &wanted, tree)
}

/// The structure tree root `tree`'s kids, each with its place: the root is the object `Ok`
/// names, or is written directly in the catalog `Err` names (see [`Place`]).
fn root_kids(
    document: &Document,
    tree: &Dictionary,
    root: Result<ObjectId, ObjectId>,
) -> VecDeque<(Place, Dictionary, Option<ObjectId>, Option<Place>)> {
    let root_place = Place {
        holder: root.unwrap_or_else(|catalog| catalog),
        path: Vec::new(),
        in_catalog: root.is_err(),
    };
    let mut queue = VecDeque::new();
    for (position, kid) in kids(document, tree).iter().enumerate() {
        if let (Some(place), Some(dict)) = (
            root_place.kid(kid, position),
            document.resolve(kid).as_dict(),
        ) {
            queue.push_back((place, dict.clone(), None, None));
        }
    }
    queue
}

/// Each holder of the `wanted` elements written again without their keys, with what to say. A
/// holder whose elements stand in a root the catalog states directly is that root, `tree`.
fn restate(document: &Document, wanted: &[Restatement], tree: &Dictionary) -> Vec<Restated> {
    let mut holders: Vec<ObjectId> = wanted.iter().map(|item| item.place.holder).collect();
    holders.sort_by_key(|id| (id.number, id.generation));
    holders.dedup();
    let mut out = Vec::new();
    for holder in holders {
        let in_catalog = wanted
            .iter()
            .any(|item| item.place.holder == holder && item.place.in_catalog);
        let dict = if in_catalog {
            Some(tree.clone())
        } else {
            document.get(holder).as_dict().cloned()
        };
        let Some(mut dict) = dict else {
            continue;
        };
        let mut details = Vec::new();
        let mut added = Vec::new();
        for item in wanted.iter().filter(|item| item.place.holder == holder) {
            added.extend(
                item.copies
                    .iter()
                    .map(|copied| (item.place.path.clone(), *copied)),
            );
            if !item.copies.is_empty() {
                details.push(format!(
                    "{} given a reference to each of {} placement copies of a form that still \
                     hold its content, whole or in part",
                    item.place.named(),
                    item.copies.len()
                ));
            }
            let dropped = at_path(&mut dict, &item.place.path, &mut |element| {
                let (written, dropped) = without(element, &item.keys)?;
                *element = written;
                Some(dropped)
            });
            let Some(dropped) = dropped else {
                continue;
            };
            let how = if item.whole {
                "every content item it marks was removed"
            } else {
                "part of the content it covers was removed, and the description covers that part \
                 too"
            };
            details.push(format!(
                "{}'s {} not written; {how}",
                item.place.named(),
                dropped.join(" and ")
            ));
        }
        if !details.is_empty() {
            out.push(Restated {
                id: holder,
                dict,
                detail: details.join("; "),
                added,
                kids_added: Vec::new(),
            });
        }
    }
    out
}

/// The text the removal took: for each stretch of consecutive codes it removed, the readback
/// from the first code's start to the last code's end, one run per line of it, each trimmed.
///
/// The range rather than the codes' own spans joined, because a sequence with §14.9.4's
/// `/ActualText` reads back its replacement once for all of its codes, and the separators the
/// interpreter inferred between words belong to the run. Under a replacement the run is the whole
/// replacement even where the region took part of its codes: §14.9.4 makes the value a character
/// substitution for the whole content the sequence encloses, and nothing maps a part of the value
/// to a part of the codes (ADR 3810).
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
