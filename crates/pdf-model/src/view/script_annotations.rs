//! The annotations a document's realm is told of, what a script's `Annotation` object changes, and
//! how a save writes the change (ADR 1700).
//!
//! Adobe's *JavaScript for Acrobat API Reference*, "Annotation", "Annotation properties" and "Doc
//! methods" (`getAnnot`, `getAnnots`, `syncAnnotScan`), cited and never quoted, is what a script
//! asks; every answer is read from the entry ISO 32000-2 states for it — Table 166's `/Subtype`,
//! `/Rect`, `/NM`, `/Contents`, `/M` and `/F`, Table 167's bits, Table 172's `/T` and `/Popup`,
//! Table 186's and Table 175's `/Open`. Which annotations are a script's, which writes it may make
//! and how they are saved are documented choices under principle 5, each named below.
//!
//! **A script reaches what a person's edit already reaches, and no further** (RFC 0008 section
//! 4.2): `hidden` is §12.6.4.11's hide, `contents` is a person's retyping of §12.5.6.6's free text
//! annotation, and `popupOpen` is what a click on a note does to its window. Each is a log beside
//! the document, and §7.5.6's update is how a save writes it: the annotation's dictionary replaced
//! with the entry changed, the producer's bytes left beneath it.
//!
//! **The file's statement is read once per realm**, the first time a realm is told of the document,
//! and every later telling lays the view state's changes over that reading: a page's `/Annots` is
//! the file's and does not change while it is open. The walk is every page's, because `getAnnots()`
//! with no page answers every page; it is made only where a runner is supplied and a script runs,
//! never on the launch path (ADR 1700 section 3 has its cost).

use std::collections::BTreeMap;

use pdf_syntax::{Dictionary, Document, Name, Object, ObjectId};

use super::script_model::{AnnotationChange, AnnotationState};
use super::{Update, ViewState};

/// The subtypes a script's `getAnnots` answers: the seventeen Adobe's "Annotation types" page
/// lists, which are §12.5.6's markup annotations and the two this program's reader edits beside
/// them.
///
/// A widget is a `Field` to a script and a link is reached through other members, and neither is
/// on the list; a popup is its parent's window, reached through `popupOpen` (ADR 1700).
const SCRIPTED_KINDS: [&str; 17] = [
    "Caret",
    "Circle",
    "FileAttachment",
    "FreeText",
    "Highlight",
    "Ink",
    "Line",
    "Polygon",
    "PolyLine",
    "Redact",
    "Sound",
    "Square",
    "Squiggly",
    "Stamp",
    "StrikeOut",
    "Text",
    "Underline",
];

/// The three subtypes Adobe's `popupOpen` is not a property of.
const WINDOWLESS_KINDS: [&str; 3] = ["FreeText", "Sound", "FileAttachment"];

/// Most annotations a realm is told of.
///
/// A page's `/Annots` is the document's, so its length is too, and what crosses to a confined
/// worker is bounded by `pdf_script::wire`'s count of items; half of that is more markup than any
/// document of the census population carries, and an annotation past it is reported, never
/// silently left out.
pub(super) const MAX_ANNOTATIONS: usize = 1 << 15;

/// Table 167 bit 2, `Hidden`.
const HIDDEN: i64 = 1 << 1;
/// Table 167 bit 7, `ReadOnly`.
const READ_ONLY: i64 = 1 << 6;

/// One annotation as the file states it, and the dictionary whose `/Open` opens its window.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Read {
    /// What the file says, before anything a reader did.
    state: AnnotationState,
    /// The annotation whose `/Open` a `popupOpen` write sets: the popup, for Table 186's entry, or
    /// a text annotation with no popup itself, for Table 175's.
    window: Option<ObjectId>,
}

/// What scripts keep of the document's annotations beside the edit log.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct Annotations {
    /// The file's statement of every scripted annotation, read the first time a realm is told.
    read: Option<Vec<Read>>,
    /// How many annotations past [`MAX_ANNOTATIONS`] the reading left out.
    left_out: usize,
    /// Each annotation a script set `hidden` on, and the value it set — what a save writes.
    pub(super) hidden: BTreeMap<ObjectId, bool>,
    /// Each popup — or text annotation with no popup — a script opened or closed, and whether it
    /// is open: what the window shows and a save writes as its `/Open`.
    pub(super) opened: BTreeMap<ObjectId, bool>,
}

impl ViewState {
    /// Every annotation a script's `getAnnots` answers, as the reader is shown it now: the file's
    /// statement, then each person's addition, in page order (ADR 1700).
    pub(super) fn annotation_states(&mut self, document: &Document) -> Vec<AnnotationState> {
        let pages = self.page_indices(document).clone();
        if self.scripting.annotations.read.is_none() {
            let (read, left_out) = read_annotations(document, &pages);
            self.scripting.annotations.read = Some(read);
            self.scripting.annotations.left_out = left_out;
            if left_out > 0 {
                self.report(format!(
                    "{left_out} annotations past the {MAX_ANNOTATIONS}th are not told to the \
                     document's scripts, and getAnnots does not answer them (ADR 1700)"
                ));
            }
        }
        let read = self
            .scripting
            .annotations
            .read
            .as_deref()
            .unwrap_or_default();
        let mut states: Vec<AnnotationState> = read
            .iter()
            .map(|read| {
                let id = ObjectId {
                    number: read.state.number,
                    generation: read.state.generation,
                };
                let mut state = read.state.clone();
                if let Some(hidden) = self.annotation_hidden(id) {
                    state.hidden = hidden;
                }
                if let Some(text) = self.retyped.get(&id) {
                    state.contents.clone_from(text);
                }
                if let Some(open) = read.window.and_then(|window| self.popup_opened(window)) {
                    state.popup_open = Some(open);
                }
                state
            })
            .collect();
        for added in &self.added {
            let Some(page) = pages.get(&added.page) else {
                continue;
            };
            if let Some(mut read) = read_one(document, added.id, &added.dict, *page) {
                if let Some(hidden) = self.annotation_hidden(added.id) {
                    read.state.hidden = hidden;
                }
                states.push(read.state);
            }
        }
        // Stable, so each page keeps the file's `/Annots` order with its additions after it —
        // the order §12.5.2 draws them in.
        states.sort_by_key(|state| state.page);
        states
    }

    /// Whether a script opened or closed this popup's window, or a text annotation's with no
    /// popup, where one did.
    #[must_use]
    pub(crate) fn popup_opened(&self, popup: ObjectId) -> Option<bool> {
        self.scripting.annotations.opened.get(&popup).copied()
    }

    /// A script's change to one annotation, made as a person's edit of it would be made, or
    /// reported where no person's edit reaches it (ADR 1700).
    pub(super) fn change_annotation(
        &mut self,
        document: &Document,
        annotation: ObjectId,
        change: &AnnotationChange,
    ) {
        let window = self
            .scripting
            .annotations
            .read
            .as_deref()
            .unwrap_or_default()
            .iter()
            .find(|read| {
                read.state.number == annotation.number
                    && read.state.generation == annotation.generation
            })
            .map(|read| read.window);
        let added = self.added.iter().any(|added| added.id == annotation);
        if window.is_none() && !added {
            self.report(format!(
                "a script changed annotation {} {}, which is not one of the document's scripted \
                 annotations, so nothing changes (ADR 1700)",
                annotation.number, annotation.generation
            ));
            return;
        }
        match change {
            AnnotationChange::Hidden(hidden) => {
                self.set_hidden(annotation, *hidden);
                self.scripting
                    .annotations
                    .hidden
                    .insert(annotation, *hidden);
            }
            AnnotationChange::Contents(text) => {
                if !self.set_free_text(document, annotation, text) {
                    self.report(format!(
                        "a script set the contents of annotation {} {}, and a reader's edit \
                         retypes only a free text annotation's, so nothing changes (ADR 1700)",
                        annotation.number, annotation.generation
                    ));
                }
            }
            AnnotationChange::PopupOpen(open) => match window.flatten() {
                Some(window) => {
                    self.scripting.annotations.opened.insert(window, *open);
                }
                None => self.report(format!(
                    "a script opened or closed the window of annotation {} {}, which has none, \
                     so nothing changes (ADR 1700)",
                    annotation.number, annotation.generation
                )),
            },
        }
    }

    /// Writes what scripts set on annotations: Table 167's `Hidden` bit into `/F`, and Table 186's
    /// `/Open` on each popup a script opened or closed (ADR 1700).
    ///
    /// **The `Hidden` bit written is the one the reader sees**, which a later hide action may have
    /// changed since the script set it: a save writes the page as it is shown. A popup closed by a
    /// script whose text annotation also states Table 175's `/Open` has that entry cleared too,
    /// since `crate::popup` opens the window where either says so and the save would otherwise
    /// write a window that opens.
    pub(super) fn write_scripted_annotations(&self, document: &Document, update: &mut Update) {
        for (annotation, hidden) in &self.scripting.annotations.hidden {
            let Some(mut dict) = update.current(document, *annotation) else {
                continue;
            };
            let hidden = self.annotation_hidden(*annotation).unwrap_or(*hidden);
            let flags = document
                .get_key(&dict, "F")
                .as_integer()
                .unwrap_or_default();
            let flags = if hidden {
                flags | HIDDEN
            } else {
                flags & !HIDDEN
            };
            dict.insert(Name::new(&b"F"[..]), Object::Integer(flags));
            update.stamp(&mut dict);
            update.put(*annotation, Object::Dictionary(dict));
        }
        for (popup, open) in &self.scripting.annotations.opened {
            let Some(mut dict) = update.current(document, *popup) else {
                continue;
            };
            dict.insert(Name::new(&b"Open"[..]), Object::Boolean(*open));
            let parent = dict.get("Parent").and_then(Object::as_reference);
            update.put(*popup, Object::Dictionary(dict));
            if *open {
                continue;
            }
            let Some(parent) = parent else {
                continue;
            };
            let Some(mut owner) = update.current(document, parent) else {
                continue;
            };
            let is_text = document
                .get_key(&owner, "Subtype")
                .as_name()
                .is_some_and(|subtype| subtype.as_bytes() == b"Text");
            if is_text && document.get_key(&owner, "Open") == Object::Boolean(true) {
                owner.insert(Name::new(&b"Open"[..]), Object::Boolean(false));
                update.stamp(&mut owner);
                update.put(parent, Object::Dictionary(owner));
            }
        }
    }
}

/// Every scripted annotation of every page, in page order, and how many past
/// [`MAX_ANNOTATIONS`] were left out.
fn read_annotations(document: &Document, pages: &BTreeMap<ObjectId, usize>) -> (Vec<Read>, usize) {
    let mut ordered: Vec<(usize, ObjectId)> = pages.iter().map(|(id, page)| (*page, *id)).collect();
    ordered.sort_unstable();
    let mut read = Vec::new();
    let mut left_out = 0_usize;
    for (index, page) in ordered {
        let page_dict = document.get(page);
        let Some(page_dict) = page_dict.as_dict() else {
            continue;
        };
        let annotations = document.get_key(page_dict, "Annots");
        for entry in annotations.as_array().unwrap_or_default() {
            let Some(id) = entry.as_reference() else {
                continue;
            };
            let resolved = document.resolve(entry);
            let Some(dict) = resolved.as_dict() else {
                continue;
            };
            let Some(one) = read_one(document, id, dict, index) else {
                continue;
            };
            if read.len() < MAX_ANNOTATIONS {
                read.push(one);
            } else {
                left_out = left_out.saturating_add(1);
            }
        }
    }
    (read, left_out)
}

/// One annotation as the file states it, or `None` where its subtype is not a script's.
fn read_one(document: &Document, id: ObjectId, dict: &Dictionary, page: usize) -> Option<Read> {
    let kind = document.get_key(dict, "Subtype");
    let kind = std::str::from_utf8(kind.as_name()?.as_bytes()).ok()?;
    if !SCRIPTED_KINDS.contains(&kind) {
        return None;
    }
    let flags = document.get_key(dict, "F").as_integer().unwrap_or_default();
    let popup = dict
        .get("Popup")
        .and_then(Object::as_reference)
        .filter(|popup| document.get(*popup).as_dict().is_some());
    // Table 186's `/Open` where the annotation has a popup; Table 175's own `/Open` on a text
    // annotation that has none, which §12.5.6.4 makes the state its window is shown in.
    let (window, popup_open) = match popup {
        _ if WINDOWLESS_KINDS.contains(&kind) => (None, None),
        Some(window) => {
            let popup = document.get(window);
            let open = popup.as_dict().is_some_and(|popup| {
                crate::popup::opens_with_the_page(document, popup, Some(dict))
            });
            (Some(window), Some(open))
        }
        None if kind == "Text" => (
            Some(id),
            Some(document.get_key(dict, "Open") == Object::Boolean(true)),
        ),
        None => (None, None),
    };
    Some(Read {
        state: AnnotationState {
            number: id.number,
            generation: id.generation,
            page: u32::try_from(page).unwrap_or(u32::MAX),
            kind: kind.to_owned(),
            rect: rectangle(document, dict),
            name: text(document, dict, "NM"),
            contents: text(document, dict, "Contents").unwrap_or_default(),
            author: text(document, dict, "T"),
            modified: text(document, dict, "M").and_then(|text| moment(&text)),
            hidden: flags & HIDDEN != 0,
            read_only: flags & READ_ONLY != 0,
            popup_open,
        },
        window,
    })
}

/// Table 166's `/Rect`, normalised so that the first corner is the lower left, as §7.9.5 reads any
/// rectangle; zeros where the entry is not four numbers.
fn rectangle(document: &Document, dict: &Dictionary) -> [f64; 4] {
    let rect = document.get_key(dict, "Rect");
    let corners: Vec<f64> = rect
        .as_array()
        .unwrap_or_default()
        .iter()
        .filter_map(|value| document.resolve(value).as_number())
        .collect();
    let [x0, y0, x1, y1] = corners.as_slice() else {
        return [0.0; 4];
    };
    [x0.min(*x1), y0.min(*y1), x0.max(*x1), y0.max(*y1)]
}

/// A text string entry, decoded as §7.9.2.2 reads one.
fn text(document: &Document, dict: &Dictionary, key: &str) -> Option<String> {
    match document.get_key(dict, key) {
        Object::String(bytes) => Some(pdf_syntax::text_string(&bytes)),
        _ => None,
    }
}

/// §7.9.4's date as the milliseconds since 1970-01-01T00:00:00Z a script's `Date` is made from,
/// where the text parses as one.
pub(super) fn moment(text: &str) -> Option<i64> {
    pdf_syntax::Date::parse(text).map(|date| {
        date.instant()
            .saturating_mul(60)
            .saturating_add(i64::from(date.second))
            .saturating_mul(1000)
    })
}
