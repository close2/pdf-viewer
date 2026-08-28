//! ISO 32000-2 §12.3.2's destinations.
//!
//! A destination is what a link, an outline item, a go-to action or the catalog's
//! `/OpenAction` points at. §12.3.2.1 says what one is, and the three items are worth keeping
//! apart because this crate can answer only the first:
//!
//! > A destination defines a particular view of a document, consisting of the following items:
//! >
//! > - The page of the document that shall be displayed
//! > - The location of the document window on that page
//! > - The magnification (zoom) factor
//!
//! The **page** is a property of the document and is computed here. The **location** and the
//! **magnification** are properties of a *window*, so [`View`] carries Table 149's parameters
//! exactly as the document states them and this crate stops there — a viewport is not something
//! a model of a file can invent. `viewer_core::Open::apply_view` is where they are applied, and
//! it is the one place that has both a window and a display list to measure a `/FitB` against.
//!
//! This module said for sixty-nine sessions that they were "properties of a window this program
//! does not have"; the program acquired scrolling and zoom in the hundred-and-thirty-second
//! session and the sentence stayed. ADR 0162.
//!
//! # Three spellings of one thing
//!
//! §12.3.2.2 gives the explicit array. §12.3.2.3 replaces its first entry with a structure
//! element and states an algorithm for getting back to a page. §12.3.2.4 replaces the whole
//! array with a name or a string looked up in one of two places, both of which this module
//! reads. [`Destination::read`] accepts all three, because every caller in the standard —
//! `/Dest`, `/D`, `/OpenAction` — may be given any of them.

use std::collections::BTreeMap;

use pdf_syntax::{Dictionary, Document, Object, ObjectId, tree};

use crate::page::Pages;

/// How far a chain of named destinations will be followed.
///
/// A named destination's value is an array or a dictionary, so one hop is all the clause
/// describes; the bound exists because a file may name a destination whose value is another
/// name, and a reader that followed that without counting would not return.
const MAX_INDIRECTION: usize = 8;

/// Table 149's destination syntax: where on the page, and how large.
///
/// A `None` is the clause's own null: "[a] null value for any of the parameters left, top, or
/// zoom specifies that the current value of that parameter shall be retained unchanged" — so
/// the absence of a number is a *distinct* instruction rather than a missing one, and it
/// cannot be collapsed into a default without changing what the document said.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum View {
    /// `[page /XYZ left top zoom]` — a corner and a magnification.
    Xyz {
        /// The left edge, in default user space.
        left: Option<f32>,
        /// The top edge, in default user space.
        top: Option<f32>,
        /// The magnification. "A zoom value of 0 has the same meaning as a null value", so a
        /// zero read from the file arrives here as `None`.
        zoom: Option<f32>,
    },
    /// `[page /Fit]` — the whole page, both directions.
    Fit,
    /// `[page /FitH top]` — the full width, with `top` at the top of the window.
    FitH {
        /// The vertical coordinate placed at the top edge of the window.
        top: Option<f32>,
    },
    /// `[page /FitV left]` — the full height, with `left` at the left of the window.
    FitV {
        /// The horizontal coordinate placed at the left edge of the window.
        left: Option<f32>,
    },
    /// `[page /FitR left bottom right top]` — a rectangle, both directions.
    FitR {
        /// The rectangle, as the array states it: left, bottom, right, top.
        rect: [f32; 4],
    },
    /// `[page /FitB]` — the page's bounding box, both directions.
    FitB,
    /// `[page /FitBH top]` — the bounding box's width.
    FitBH {
        /// The vertical coordinate placed at the top edge of the window.
        top: Option<f32>,
    },
    /// `[page /FitBV left]` — the bounding box's height.
    FitBV {
        /// The horizontal coordinate placed at the left edge of the window.
        left: Option<f32>,
    },
}

impl View {
    /// Reads Table 149's name and its parameters from the rest of the destination array.
    ///
    /// A form whose name the table does not list is not a view, and neither is one whose
    /// required parameters are absent — `/FitR` is the only form with any, and a `/FitR`
    /// short of its four numbers states no rectangle at all.
    fn read(document: &Document, name: &[u8], rest: &[Object]) -> Option<Self> {
        let numbers: Vec<Option<f32>> = rest
            .iter()
            .map(|object| {
                document
                    .resolve(object)
                    .as_number()
                    .map(narrow)
                    .filter(|value| value.is_finite())
            })
            .collect();
        Self::from_keyword(name, &numbers)
    }

    /// Table 149's eight forms, from the keyword and the numbers that follow it.
    ///
    /// The one place Table 149 is spelled, and it has two callers because the standard gives it
    /// two spellings: §12.3.2.2's array, where the numbers are objects, and Annex O's `view`
    /// parameter, where they are text in a URI and the table is named outright — "[t]he keyword
    /// shall correspond to one of the keywords defined in "Table 149 - Destination syntax" with
    /// appropriate position values".
    ///
    /// A `None` in `numbers` is the clause's own null and a number the caller did not supply is
    /// the same thing, which is what lets a short list stand for a destination that states
    /// nothing after its keyword.
    #[must_use]
    pub fn from_keyword(keyword: &[u8], numbers: &[Option<f32>]) -> Option<Self> {
        let number = |index: usize| numbers.get(index).copied().flatten();
        Some(match keyword {
            b"XYZ" => Self::Xyz {
                left: number(0),
                top: number(1),
                // "A zoom value of 0 has the same meaning as a null value."
                zoom: number(2).filter(|zoom| *zoom != 0.0),
            },
            b"Fit" => Self::Fit,
            b"FitH" => Self::FitH { top: number(0) },
            b"FitV" => Self::FitV { left: number(0) },
            b"FitR" => Self::FitR {
                rect: [number(0)?, number(1)?, number(2)?, number(3)?],
            },
            b"FitB" => Self::FitB,
            b"FitBH" => Self::FitBH { top: number(0) },
            b"FitBV" => Self::FitBV { left: number(0) },
            _ => return None,
        })
    }
}

/// Narrows a coordinate to the precision the rest of this crate works in.
#[expect(
    clippy::cast_possible_truncation,
    reason = "destination coordinates are default user space, bounded by the format at \
              14 400 units, far inside f32's exact integer range"
)]
fn narrow(value: f64) -> f32 {
    value as f32
}

/// What a destination's first entry names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// An indirect reference, to a page object (§12.3.2.2) or a structure element (§12.3.2.3).
    ///
    /// Which of the two it is, is a property of the object rather than of the destination, and
    /// [`Destination::page_index`] is where it is decided — by asking the page tree first,
    /// because a reference the page tree holds *is* a page whatever its `/Type` says.
    Object(ObjectId),
    /// A page number in another document.
    ///
    /// §12.3.2.2's NOTE: "No page object can be specified for a destination associated with a
    /// remote go-to action … In this case, the page parameter specifies an integer page number
    /// within the remote document instead of a page object in the current document." So this
    /// form names nothing in *this* file, which is why [`Destination::page_index`] answers
    /// `None` for it rather than treating the number as an index here.
    Number(i64),
}

/// A destination: which page, and which view of it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Destination {
    /// What the array's first entry names.
    pub target: Target,
    /// Table 149's form and its parameters.
    pub view: View,
}

impl Destination {
    /// Reads a destination from any of the three forms the standard allows a caller to hold.
    ///
    /// - an explicit array (§12.3.2.2, §12.3.2.3);
    /// - a dictionary with a `/D` entry, which §12.3.2.4 describes and which "allows additional
    ///   attributes to be associated with the destination";
    /// - a name or a string, resolved against the two tables §12.3.2.4 defines.
    ///
    /// Returns `None` for an object that is none of those, which includes an array whose form
    /// name Table 149 does not list.
    #[must_use]
    pub fn read(document: &Document, object: &Object) -> Option<Self> {
        Self::read_within(document, object, 0)
    }

    /// [`Self::read`], counting the hops a named destination costs.
    fn read_within(document: &Document, object: &Object, depth: usize) -> Option<Self> {
        if depth > MAX_INDIRECTION {
            return None;
        }
        match document.resolve(object) {
            Object::Array(items) => Self::from_array(document, &items),
            // §12.3.2.4's dictionary form, which is the *destination's* own dictionary and not
            // an action's: "the corresponding value is either an array defining the
            // destination … or a dictionary with a D entry whose value is such an array and may
            // optionally contain an SD entry as defined in "Table 201 -Action types"".
            //
            // **The `/SD` half of that sentence became reachable through an erratum**, which is
            // why it went unread until the five-hundred-and-ninety-fifth session: Errata
            // Collection 3 repointed the reference from Table 201 — a list of action *types*,
            // where nothing is defined at all — to Tables 202–204, where §12.6.4.2's `/SD` is.
            // With the reference broken there was nothing for this arm to read; with it
            // repaired the entry is the same entry a go-to action states, so it is read by the
            // same rule and in the same order.
            Object::Dictionary(dict) => Self::preferring_structure(document, &dict, depth),
            Object::Name(name) => Self::named(document, &Key::Name(name.as_bytes()), depth),
            Object::String(bytes) => Self::named(document, &Key::String(&bytes), depth),
            _ => None,
        }
    }

    /// Reads Table 149's array, whichever kind of first entry it has.
    fn from_array(document: &Document, items: &[Object]) -> Option<Self> {
        let target = match items.first()? {
            Object::Reference(id) => Target::Object(*id),
            // An integer first entry is the remote form; anything else states no page.
            other => Target::Number(document.resolve(other).as_integer()?),
        };
        let name = document.resolve(items.get(1)?);
        let name = name.as_name()?;
        let rest = items.get(2..).unwrap_or_default();
        Some(Self {
            target,
            view: View::read(document, name.as_bytes(), rest)?,
        })
    }

    /// §12.3.2.4's lookup, in the two places the clause defines and in that order.
    ///
    /// > In PDF 1.1, the correspondence between name objects and destinations shall be defined
    /// > by the Dests entry in the document catalog dictionary
    ///
    /// > In PDF 1.2 and later, the correspondence between strings and destinations may
    /// > alternatively be defined by the Dests entry in the document's name dictionary
    ///
    /// The clause pairs a *name* with the catalog's dictionary and a *string* with the name
    /// tree, and NOTE 4 says a document "can contain both types". Both tables are asked here,
    /// in the order the clause introduces them, because "alternatively" is a sentence about
    /// where a document keeps its table rather than about which kind of object may be looked
    /// up in it — and because the key is the same bytes either way.
    ///
    /// **The corpus says the clause's own pairing holds without exception**, which is worth
    /// recording as a measurement rather than an assumption: of the 106 named destinations
    /// reachable from link annotations in the 974 documents, the 22 that resolve are 2 name
    /// objects found in a catalog `/Dests` and 20 strings found in a name tree, and not one
    /// crosses. So the second lookup changes no answer in any file we have.
    ///
    /// Annex J.3.3 and J.3.4 are what "the same key" means, and they cost nothing here: both
    /// reduce to a binary comparison of the *decoded* bytes — a literal string's escapes
    /// expanded, a hexadecimal string converted, a name's `#` escapes resolved — which is
    /// exactly what the lexer has already produced by the time an [`Object`] exists.
    fn named(document: &Document, key: &Key<'_>, depth: usize) -> Option<Self> {
        let catalog = document.catalog().ok()?;

        if let Some(dests) = document.get_key(&catalog, "Dests").as_dict()
            && let Some(value) = dests.get_by_name(&pdf_syntax::Name::new(key.bytes().to_vec()))
        {
            return Self::read_within(document, value, depth.saturating_add(1));
        }

        let names = document.get_key(&catalog, "Names");
        let names = names.as_dict()?;
        let root = document.get_key(names, "Dests");
        let root = root.as_dict()?;
        let value = tree::lookup(root, &tree::TreeKey::Name(key.bytes()), &|object| {
            document.resolve(object)
        })?;
        Self::read_within(document, &value, depth.saturating_add(1))
    }

    /// The catalog's `/OpenAction`, where it names a destination.
    ///
    /// Table 29: "A value specifying a destination that shall be displayed or an action that
    /// shall be performed when the document is opened. The value shall be either an array
    /// defining a destination … or an action dictionary". So there are two shapes, and only
    /// some actions carry a destination: §12.6.4.2's go-to action has `/D`, and the other
    /// eighteen action types either go somewhere this reader will not follow or do something
    /// that is not a view. An `/OpenAction` naming one of those is not a defect and is not a
    /// destination, so it answers `None`.
    ///
    /// Table 29 also states what an absent entry means, which is why the caller may treat
    /// `None` as an ordinary document rather than as a failure: "If this entry is absent, the
    /// document shall be opened to the top of the first page at the default magnification
    /// factor."
    #[must_use]
    pub fn open_action(document: &Document) -> Option<Self> {
        let catalog = document.catalog().ok()?;
        let entry = document.get_key(&catalog, "OpenAction");
        match &entry {
            Object::Array(_) => Self::read(document, &entry),
            Object::Dictionary(action) => {
                // §12.6.2's `/S` names the action type; only a go-to action states a view of
                // this document. A `/GoToR` or `/GoToE` names a page in another file, which
                // this reader has no way to open, and its `/D` is that file's destination.
                let kind = document.get_key(action, "S");
                let kind = kind.as_name()?;
                if kind.as_bytes() != b"GoTo" {
                    return None;
                }
                Self::of_go_to(document, action)
            }
            _ => None,
        }
    }

    /// The destination a §12.6.4.2 go-to action states, from Table 202's two entries in the
    /// clause's own order.
    ///
    /// `/D` is required and `/SD` is optional, and the table states which wins:
    ///
    /// > (Optional; PDF 2.0) The structure destination to jump to (see 12.3.2.3, "Structure
    /// > destinations"). If present, the structure destination should take precedence over
    /// > destination in the D entry.
    ///
    /// **This is the entry `doc/todo/01`'s fifteenth sweep found nothing reading**, while
    /// §12.3.2.3's own row has been `implemented` since the algorithm that resolves a structure
    /// element to a page landed: the capability arrived and the entry that turns it on was never
    /// wired to it, which is `doc/habits.md`'s sixth refusal shape and the third finding of that
    /// shape (ADRs 0295, 0315, 0319). Nothing else in the standard states an `/SD`: Table 203's
    /// is a remote go-to's, which `CLAUDE.md` excludes.
    ///
    /// The precedence is a `should` rather than a `shall`, and that is exactly why an `/SD` this
    /// reader cannot read as a destination falls back to `/D` instead of refusing: `/D` is the
    /// required entry and jumping to it is conforming. An `/SD` that *is* a destination always
    /// resolves, because §12.3.2.3 ends its own algorithm with "the page reference shall be
    /// assumed to be the first page in the document".
    #[must_use]
    pub fn of_go_to(document: &Document, action: &Dictionary) -> Option<Self> {
        Self::preferring_structure(document, action, 0)
    }

    /// `/SD` where it names a destination and `/D` otherwise, counting the hops either costs.
    ///
    /// One function for §12.6.4.2's action dictionary and §12.3.2.4's destination dictionary,
    /// because the clause makes them one entry: a named destination's dictionary "may
    /// optionally contain an SD entry **as defined in**" the go-to action's own table. Two
    /// readings of one sentence is how the two would come to disagree about which entry wins.
    fn preferring_structure(document: &Document, dict: &Dictionary, depth: usize) -> Option<Self> {
        let deeper = depth.saturating_add(1);
        dict.get("SD")
            .and_then(|entry| Self::read_within(document, entry, deeper))
            .or_else(|| Self::read_within(document, dict.get("D")?, deeper))
    }

    /// The zero-based index of the page this destination displays, or `None`.
    ///
    /// Three answers are `None`, and they are different things: a [`Target::Number`], which
    /// names a page in another document; a reference to an object that is neither in the page
    /// tree nor a structure element with content; and a structure destination in a document
    /// with no pages.
    ///
    /// The page tree is asked *first*, before any `/Type` is read, because a reference the page
    /// tree holds is a page whatever the object says about itself — and only a reference the
    /// tree does not hold can be §12.3.2.3's structure element.
    #[must_use]
    pub fn page_index(&self, document: &Document, pages: &Pages<'_>) -> Option<usize> {
        self.resolve(document, pages, None)
    }

    /// As [`Self::page_index`], against a page index prepared once by [`Pages::indices`].
    ///
    /// The same answer, and the reason to want it is arithmetic: `index_of` cannot skip a
    /// subtree, so resolving *many* destinations one at a time is a tree walk apiece. §12.3.3's
    /// outline is the caller that showed it — 988 items over 1023 pages, 344 ms of every page
    /// turn until the hundred-and-forty-first session.
    #[must_use]
    pub fn page_index_with(
        &self,
        document: &Document,
        pages: &Pages<'_>,
        indices: &BTreeMap<ObjectId, usize>,
    ) -> Option<usize> {
        self.resolve(document, pages, Some(indices))
    }

    /// The page this destination names, with or without a prepared index.
    fn resolve(
        &self,
        document: &Document,
        pages: &Pages<'_>,
        indices: Option<&BTreeMap<ObjectId, usize>>,
    ) -> Option<usize> {
        let locate = |id| match indices {
            Some(indices) => indices.get(&id).copied(),
            None => pages.index_of(id),
        };
        let Target::Object(id) = self.target else {
            return None;
        };
        if let Some(index) = locate(id) {
            return Some(index);
        }
        let element = document.get(id);
        let element = element.as_dict()?;
        structure_page(document, element, 0).and_then(locate).or({
            // §12.3.2.3: "In the case where no page content is identified, then the page
            // reference shall be assumed to be the first page in the document." Only reached
            // once the object has been established to be a structure element, so a dangling
            // reference to nothing still answers `None` rather than page one.
            if is_structure_element(document, element) {
                Some(0)
            } else {
                None
            }
        })
    }

    /// The page this destination names, read in the document it was *written for*.
    ///
    /// The difference from [`Self::page_index`] is one case, and it is the case §12.6.4.3's and
    /// §12.6.4.4's destinations are made of. §12.3.2.2:
    ///
    /// > No page object can be specified for a destination associated with a remote go-to action
    /// > … because the destination page is in a different PDF document. In this case, the page
    /// > parameter specifies an integer page number within the remote document instead of a page
    /// > object in the current document.
    ///
    /// and its NOTE says "[t]he above paragraph was corrected to also include embedded go-to
    /// actions (2020)". Table 203 gives the numbering: "[t]he first page shall be numbered 0."
    ///
    /// So [`Target::Number`] *is* an index here, where in [`Self::page_index`] it is a number
    /// about a file this reader does not have. Both are answered because a destination reached
    /// through §12.6.4.4 may equally be an indirect reference: the target is a whole document,
    /// and a reference in it is resolved against it.
    #[must_use]
    pub fn page_index_in_target(&self, target: &Document, pages: &Pages<'_>) -> Option<usize> {
        match self.target {
            Target::Number(number) => usize::try_from(number).ok(),
            Target::Object(_) => self.page_index(target, pages),
        }
    }
}

/// Whether a dictionary is a structure element, for §12.3.2.3's fallback.
///
/// Table 355 makes `/Type` optional — "if present, shall be `StructElem`" — and `/S`, the
/// structure type, required. So the pair of them is the test, and a dictionary with neither is
/// not a structure element and does not get the fallback to page one.
fn is_structure_element(document: &Document, element: &Dictionary) -> bool {
    let has_type = document
        .get_key(element, "Type")
        .as_name()
        .is_some_and(|name| name.as_bytes() == b"StructElem");
    has_type || document.get_key(element, "S").as_name().is_some()
}

/// The page a structure element's content sits on, for a caller holding the element itself.
///
/// §12.3.2.3's algorithm, reached without a [`Destination`]. Annex O's `structelem` parameter is
/// the caller that needs it: Table Annex O.3 names a structure element by the `/ID` §14.7.2's
/// `/IDTree` files it under rather than by a reference, so there is no destination array to read
/// and the same question — which page is this element's content on — still has to be answered.
///
/// `None` where the element identifies no page. Annex O states what to do then and so does
/// §12.3.2.3, in the same words, and both are the caller's to apply: "the first page in the
/// document shall be identified".
#[must_use]
pub fn structure_element_page(
    document: &Document,
    element: &Dictionary,
    pages: &Pages<'_>,
) -> Option<usize> {
    structure_page(document, element, 0).and_then(|page| pages.index_of(page))
}

/// §12.3.2.3's algorithm for the page a structure element's content sits on.
///
/// > The kids of the structure element shall be processed in linear array order. If the first
/// > kid is a marked-content reference or an object reference (see 14.6, "Marked content"),
/// > then the page to which that reference belongs shall be used as the page. If the first kid
/// > is a structure element, then processing shall continue down to that element using the same
/// > algorithm recursively. If no content or object reference is found under the first entry,
/// > processing should proceed to next entry, repeating the process. This shall continue until
/// > all entries have been processed or until the first page is identified.
///
/// The page a reference "belongs to" is Table 357's and Table 358's `/Pg`, and where the
/// reference states none it is the containing element's — §14.7.5.3 says of an object
/// reference's `/Pg` that it takes precedence over any `/Pg` in the structure element containing
/// the object reference, and is required where that element has none. (This sentence read "[t]his
/// entry overrides any Pg entry in the structure element containing the object reference; it
/// shall be used if the structure element has no such entry" until Errata Collection 3 — Issue
/// #431, `/State` `Review` `Accepted` — which also reclassifies the entry from "Optional" to
/// "Sometimes required". **Two accepted errata amend that one sentence and this comment credited
/// one of them with both**: #431 replaces everything after *overrides* — which it makes *takes
/// precedence over* — with a second sentence, *This entry is required if the structure element
/// containing the object reference has no Pg entry.*, while Issue #339, `/State` `Review`
/// `Completed`, strikes the *used* inside the run #431 replaces and writes *required* there. They
/// overlap in text and agree in meaning, which is the opposite of Table 161's pair. Neither
/// changes what this function does; ADR 0253 and ADR 0728.) §14.7.5.2's integer
/// form of a kid means a marked-content sequence "contained in
/// the content stream of the page that is specified in the Pg entry of the structure element".
fn structure_page(document: &Document, element: &Dictionary, depth: usize) -> Option<ObjectId> {
    if depth > MAX_INDIRECTION {
        return None;
    }
    // The *reference* rather than the object: a page is identified by its object number here,
    // exactly as a destination's own first entry is, and resolving it would throw that away.
    let own_page = element.get("Pg").and_then(Object::as_reference);

    let kids = document.get_key(element, "K");
    // "The kids … shall be processed in linear array order", and a single kid may be written
    // without its array, which Table 355 allows by typing `/K` as "(various)".
    let kids = match &kids {
        Object::Array(items) => items.clone(),
        Object::Null => Vec::new(),
        single => vec![single.clone()],
    };

    for kid in &kids {
        let kid = document.resolve(kid);
        match &kid {
            // An integer is a marked-content identifier in this element's own page.
            Object::Integer(_) => {
                if let Some(page) = own_page {
                    return Some(page);
                }
            }
            Object::Dictionary(dict) => {
                let stated = document.get_key(dict, "Type");
                let stated = stated.as_name().map(|name| name.as_bytes().to_vec());
                match stated.as_deref() {
                    // Table 357's marked-content reference and Table 358's object reference
                    // both name their page with `/Pg`, and fall back to this element's.
                    Some(b"MCR" | b"OBJR") => {
                        if let Some(page) =
                            dict.get("Pg").and_then(Object::as_reference).or(own_page)
                        {
                            return Some(page);
                        }
                    }
                    // Anything else under `/K` is a child structure element.
                    _ => {
                        if let Some(page) = structure_page(document, dict, depth.saturating_add(1))
                        {
                            return Some(page);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    // No kid identified a page. The element's own `/Pg` is still a statement about where its
    // content is, and the clause's algorithm reaches it through any integer kid; an element
    // with `/Pg` and no kids at all says the same thing with less.
    own_page
}

/// A named destination's key, which the clause spells two ways.
enum Key<'a> {
    /// §12.3.2.4's PDF 1.1 form: a name object.
    Name(&'a [u8]),
    /// Its PDF 1.2 form: a byte string.
    String(&'a [u8]),
}

impl Key<'_> {
    /// The bytes Annex J compares. Both forms reduce to the same thing.
    fn bytes(&self) -> &[u8] {
        match self {
            Self::Name(bytes) | Self::String(bytes) => bytes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Destination, Target, View, structure_element_page};
    use pdf_syntax::{Document, Object};

    /// Builds a one-page document from object bodies numbered from 1.
    fn document(objects: &[&str]) -> Document {
        use std::fmt::Write as _;
        let mut out = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
        }
        let xref_at = out.len();
        let _ = write!(
            out,
            "xref\n0 {}\n0000000000 65535 f \n",
            objects.len().saturating_add(1)
        );
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
            objects.len().saturating_add(1)
        );
        Document::open(out.into_bytes()).expect("a valid file")
    }

    /// Table 149's eight forms, each with the parameters the table gives it.
    ///
    /// One array per row of the table, so a form dropped from [`View::read`] fails here rather
    /// than falling through to `None` in a document nobody reads.
    #[test]
    fn every_form_table_149_lists_is_read() {
        let doc = document(&["<< /Type /Catalog /Pages 2 0 R >>"]);
        let form = |text: &str| {
            let object = pdf_syntax::Parser::new(text.as_bytes())
                .parse_object()
                .expect("an array");
            Destination::read(&doc, &object).map(|destination| destination.view)
        };

        assert_eq!(
            form("[2 0 R /XYZ 10 20 2]"),
            Some(View::Xyz {
                left: Some(10.0),
                top: Some(20.0),
                zoom: Some(2.0)
            })
        );
        assert_eq!(form("[2 0 R /Fit]"), Some(View::Fit));
        assert_eq!(
            form("[2 0 R /FitH 700]"),
            Some(View::FitH { top: Some(700.0) })
        );
        assert_eq!(
            form("[2 0 R /FitV 5]"),
            Some(View::FitV { left: Some(5.0) })
        );
        assert_eq!(
            form("[2 0 R /FitR 1 2 3 4]"),
            Some(View::FitR {
                rect: [1.0, 2.0, 3.0, 4.0]
            })
        );
        assert_eq!(form("[2 0 R /FitB]"), Some(View::FitB));
        assert_eq!(
            form("[2 0 R /FitBH 9]"),
            Some(View::FitBH { top: Some(9.0) })
        );
        assert_eq!(
            form("[2 0 R /FitBV 9]"),
            Some(View::FitBV { left: Some(9.0) })
        );
        assert_eq!(
            form("[2 0 R /Fits]"),
            None,
            "a form the table does not list"
        );
    }

    /// A null parameter and a zero zoom mean "leave it as it is", and they are not zero.
    ///
    /// Two sentences in one test because they say the same thing twice: "[a] null value for any
    /// of the parameters left, top, or zoom specifies that the current value of that parameter
    /// shall be retained unchanged. A zoom value of 0 has the same meaning as a null value."
    /// A reader that read either as a number would jump to the page's origin at no
    /// magnification.
    #[test]
    fn a_null_parameter_is_not_a_zero() {
        let doc = document(&["<< /Type /Catalog /Pages 2 0 R >>"]);
        let form = |text: &str| {
            let object = pdf_syntax::Parser::new(text.as_bytes())
                .parse_object()
                .expect("an array");
            Destination::read(&doc, &object).map(|destination| destination.view)
        };

        assert_eq!(
            form("[2 0 R /XYZ null null 0]"),
            Some(View::Xyz {
                left: None,
                top: None,
                zoom: None
            })
        );
        assert_eq!(
            form("[2 0 R /XYZ 0 0 1]"),
            Some(View::Xyz {
                left: Some(0.0),
                top: Some(0.0),
                zoom: Some(1.0)
            }),
            "a stated zero coordinate is a coordinate"
        );
    }

    /// §12.3.2.4's two tables, and one document containing both.
    ///
    /// NOTE 4 says "[a] document that supports PDF 1.2 or later can contain both types", which
    /// is what this file is: `/First` in the catalog's PDF 1.1 dictionary, `Second` in the name
    /// dictionary's tree. Both resolve, and a key in neither resolves to nothing.
    #[test]
    fn a_named_destination_is_found_in_either_table() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /Dests 4 0 R /Names 5 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            "<< /First [3 0 R /Fit] >>",
            "<< /Dests << /Names [(Second) << /D [3 0 R /FitB] >>] >> >>",
        ]);

        let read = |object: Object| Destination::read(&doc, &object).map(|d| d.view);
        assert_eq!(
            read(Object::Name(pdf_syntax::Name::new(b"First".to_vec()))),
            Some(View::Fit),
            "the catalog's /Dests dictionary, which is the PDF 1.1 form"
        );
        assert_eq!(
            read(Object::String(b"Second".as_slice().into())),
            Some(View::FitB),
            "the name dictionary's tree, through a /D dictionary"
        );
        assert_eq!(read(Object::String(b"Third".as_slice().into())), None);
    }

    /// A named destination whose value names itself terminates.
    ///
    /// Nothing in the clause describes a name whose value is a name, so a file that writes one
    /// is malformed — and a reader that followed it without counting would not return.
    #[test]
    fn a_cycle_between_names_terminates() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /Dests 3 0 R >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /Loop /Loop >>",
        ]);
        assert_eq!(
            Destination::read(&doc, &Object::Name(pdf_syntax::Name::new(b"Loop".to_vec()))),
            None
        );
    }

    /// The `/OpenAction` in both of Table 29's shapes, and one that is not a destination.
    #[test]
    fn an_open_action_may_be_an_array_or_a_go_to_action() {
        let array = document(&[
            "<< /Type /Catalog /Pages 2 0 R /OpenAction [3 0 R /XYZ null 792 null] >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        ]);
        assert_eq!(
            Destination::open_action(&array).map(|d| d.target),
            Some(Target::Object(pdf_syntax::ObjectId::new(3, 0)))
        );

        let action = document(&[
            "<< /Type /Catalog /Pages 2 0 R /OpenAction << /S /GoTo /D [3 0 R /Fit] >> >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        ]);
        assert_eq!(
            Destination::open_action(&action).map(|d| d.view),
            Some(View::Fit)
        );

        let script = document(&[
            "<< /Type /Catalog /Pages 2 0 R /OpenAction << /S /JavaScript /JS (app.alert\\(1\\)) >> >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        ]);
        assert_eq!(
            Destination::open_action(&script),
            None,
            "an action that is not a go-to states no view of this document"
        );
    }

    /// §12.3.2.3's algorithm, on the example the clause describes in words.
    ///
    /// The destination names a structure element rather than a page, its first kid is another
    /// structure element with no page of its own, and the page falls out of *that* element's
    /// marked-content reference. Every step of the recursion is exercised, and the answer is
    /// the second page rather than the first so that the clause's own fallback cannot pass it.
    #[test]
    fn a_structure_destination_finds_its_page_through_the_kids() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 5 0 R >>",
            "<< /Type /Pages /Count 2 /Kids [3 0 R 4 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            "<< /Type /StructElem /S /Sect /K [6 0 R] >>",
            "<< /Type /StructElem /S /P /K [<< /Type /MCR /Pg 4 0 R /MCID 0 >>] >>",
        ]);
        let pages = crate::page::Pages::new(&doc);
        let destination = Destination::read(
            &doc,
            &pdf_syntax::Parser::new(b"[5 0 R /Fit]")
                .parse_object()
                .expect("an array"),
        )
        .expect("a destination");
        assert_eq!(destination.page_index(&doc, &pages), Some(1));
    }

    /// A structure element whose kids identify no page falls back to the first page, and an
    /// ordinary dangling reference does not.
    ///
    /// The two halves are the difference between the clause's fallback and a guess: "[i]n the
    /// case where no page content is identified, then the page reference shall be assumed to be
    /// the first page in the document" is a sentence about *structure destinations*, and
    /// applying it to any unresolvable reference would send a broken link to page one and call
    /// it correct.
    #[test]
    fn the_fallback_to_page_one_belongs_to_structure_destinations_only() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            "<< /Type /StructElem /S /P >>",
            "<< /Length 0 >>\nstream\n\nendstream",
        ]);
        let pages = crate::page::Pages::new(&doc);
        let index = |text: &str| {
            Destination::read(
                &doc,
                &pdf_syntax::Parser::new(text.as_bytes())
                    .parse_object()
                    .expect("an array"),
            )
            .expect("a destination")
            .page_index(&doc, &pages)
        };
        assert_eq!(
            index("[4 0 R /Fit]"),
            Some(0),
            "a structure element with no content"
        );
        assert_eq!(
            index("[5 0 R /Fit]"),
            None,
            "an object that is not a page or an element"
        );
    }

    /// Table 202's precedence, on a pair of actions differing only in `/SD`.
    ///
    /// Trap 8's shape, said out loud: 1187 documents on this disk state ten `/SD` between two
    /// files and **not one of the ten names a page or a view its own `/D` does not**
    /// (`examples/structure_destination_census`), so the corpus cannot rank this sentence and
    /// the witness has to be built. The pair differs in the single entry, which is what makes
    /// the test about the entry rather than about a jump.
    #[test]
    fn a_go_to_actions_structure_destination_takes_precedence_over_its_page_destination() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 5 0 R >>",
            "<< /Type /Pages /Count 2 /Kids [3 0 R 4 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            "<< /Type /StructElem /S /P /K [<< /Type /MCR /Pg 4 0 R /MCID 0 >>] >>",
            "<< /Type /Action /S /GoTo /D [3 0 R /Fit] >>",
            "<< /Type /Action /S /GoTo /D [3 0 R /Fit] /SD [5 0 R /FitH 500] >>",
            "<< /Type /Action /S /GoTo /D [3 0 R /Fit] /SD 99 0 R >>",
        ]);
        let pages = crate::page::Pages::new(&doc);
        let jump = |number: u32| {
            let object = doc.get(pdf_syntax::ObjectId::new(number, 0));
            let action = object.as_dict().expect("an action dictionary").clone();
            let destination = Destination::of_go_to(&doc, &action).expect("a destination");
            (destination.page_index(&doc, &pages), destination.view)
        };

        assert_eq!(jump(6), (Some(0), View::Fit), "the /D alone");
        assert_eq!(
            jump(7),
            (Some(1), View::FitH { top: Some(500.0) }),
            "the /SD takes precedence, page and view together"
        );
        assert_eq!(
            jump(8),
            (Some(0), View::Fit),
            "an /SD that is no destination leaves the required /D, which is what a `should` \
             permits"
        );
    }

    /// The same entry in §12.3.2.4's own dictionary, reached through a name and through a string.
    ///
    /// The clause makes a named destination's dictionary carry the go-to action's `/SD` — "a
    /// dictionary with a D entry whose value is such an array and may optionally contain an SD
    /// entry as defined in" that action's table — and the erratum that repointed the reference
    /// is what made the sentence readable at all. Both of §12.3.2.4's tables are asked, because
    /// the entry is the destination's rather than the table's, and a name and a string reach it
    /// by different routes here.
    #[test]
    fn a_named_destinations_structure_destination_takes_precedence_over_its_page_destination() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 5 0 R /Dests 6 0 R \
              /Names << /Dests 7 0 R >> >>",
            "<< /Type /Pages /Count 2 /Kids [3 0 R 4 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            "<< /Type /StructElem /S /P /K [<< /Type /MCR /Pg 4 0 R /MCID 0 >>] >>",
            "<< /Plain << /D [3 0 R /Fit] >> \
               /Structured << /D [3 0 R /Fit] /SD [5 0 R /FitH 500] >> \
               /Unreadable << /D [3 0 R /Fit] /SD 99 0 R >> >>",
            "<< /Names [(Structured) << /D [3 0 R /Fit] /SD [5 0 R /FitH 500] >>] >>",
        ]);
        let pages = crate::page::Pages::new(&doc);
        let jump = |text: &str| {
            let object = pdf_syntax::Parser::new(text.as_bytes())
                .parse_object()
                .expect("a name or a string");
            let destination = Destination::read(&doc, &object).expect("a destination");
            (destination.page_index(&doc, &pages), destination.view)
        };

        assert_eq!(jump("/Plain"), (Some(0), View::Fit), "the /D alone");
        assert_eq!(
            jump("/Structured"),
            (Some(1), View::FitH { top: Some(500.0) }),
            "the /SD takes precedence in the catalog's own /Dests dictionary"
        );
        assert_eq!(
            jump("(Structured)"),
            (Some(1), View::FitH { top: Some(500.0) }),
            "and in the name tree, which is where PDF 1.2 and later put the table"
        );
        assert_eq!(
            jump("/Unreadable"),
            (Some(0), View::Fit),
            "an /SD that is no destination leaves the required /D, exactly as a go-to action's \
             does"
        );
    }

    /// A page *number* names a page in another document, and not one in this one.
    ///
    /// §12.3.2.2's NOTE is explicit that the integer form belongs to remote and embedded go-to
    /// actions. Reading it as an index here would send `[2 /Fit]` to this file's third page,
    /// which is a page the document never mentioned.
    #[test]
    fn a_page_number_names_nothing_in_this_document() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        ]);
        let pages = crate::page::Pages::new(&doc);
        let destination = Destination::read(
            &doc,
            &pdf_syntax::Parser::new(b"[2 /Fit]")
                .parse_object()
                .expect("an array"),
        )
        .expect("a destination");
        assert_eq!(destination.target, Target::Number(2));
        assert_eq!(destination.page_index(&doc, &pages), None);
    }

    /// Annex O's `structelem`, end to end at the model's level: §14.7.2's `/IDTree` for the
    /// element and §12.3.2.3's algorithm for its page.
    ///
    /// Written by hand, and **the reason written here was false from the round that wrote it**:
    /// it said "no corpus document has an `/IDTree` at all — the 89 tagged ones state none", and
    /// 12 of those 89 state one, holding between 1 and 285 identifiers. [`Tree::element_by_id`]
    /// carries the correction and the count; this comment repeated the retired claim for the
    /// whole time, which is `doc/habits.md`'s "a retired claim is a string, and strings are
    /// greppable" (ADR 0405).
    ///
    /// [`Tree::element_by_id`]: crate::structure::Tree::element_by_id
    ///
    /// What the fixture is still for is the *other* half: a document whose `/IDTree` names an
    /// element whose own `/Pg` is absent, so §12.3.2.3's algorithm has to find the page from the
    /// element's content items. §14.7.7's own worked example is the shape being imitated, down to
    /// the `Chap1` key.
    #[test]
    fn a_structure_element_named_by_id_identifies_its_page() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 5 0 R >>",
            "<< /Type /Pages /Count 2 /Kids [3 0 R 4 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            "<< /Type /StructTreeRoot /K [6 0 R] /IDTree 8 0 R >>",
            "<< /Type /StructElem /S /Sect /ID (Chap2) /K [7 0 R] >>",
            "<< /Type /MCR /Pg 4 0 R /MCID 0 >>",
            "<< /Names [(Chap2) 6 0 R] >>",
        ]);
        let pages = crate::page::Pages::new(&doc);
        let tree = crate::structure::Tree::of(&doc).expect("a structure tree");
        let element = tree
            .element_by_id(&doc, b"Chap2")
            .expect("the element the /IDTree files under Chap2");
        assert_eq!(
            structure_element_page(&doc, &element, &pages),
            Some(1),
            "its only content item is a marked-content reference to the second page"
        );

        // "If no content is contained within the hierarchy of the structure element … the first
        // page in the document shall be identified" — stated by Annex O and by §12.3.2.3, and
        // applied by the caller, so what this answers is `None` rather than page one.
        let empty = document(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            "<< /Type /StructTreeRoot /K [5 0 R] /IDTree 6 0 R >>",
            "<< /Type /StructElem /S /Sect /ID (Chap2) >>",
            "<< /Names [(Chap2) 5 0 R] >>",
        ]);
        let pages = crate::page::Pages::new(&empty);
        let tree = crate::structure::Tree::of(&empty).expect("a structure tree");
        let element = tree.element_by_id(&empty, b"Chap2").expect("the element");
        assert_eq!(structure_element_page(&empty, &element, &pages), None);
    }
}
