//! The structure tree's leaves that name a photographed page, taken out of the tree (owner answer
//! A707, ADR 2516).
//!
//! # What a leaf is, and why it goes
//!
//! ISO 32000-2 §14.7.5 gives a structure element three kinds of content item in its `/K` entry:
//! an integer marked-content identifier, for a marked-content sequence on the page the element's
//! `/Pg` names (§14.7.5.2); a marked-content reference dictionary, `/Type /MCR`, which may name a
//! page of its own; and an object reference dictionary, `/Type /OBJR`, for an entire object such
//! as an `XObject` (§14.7.5.3). Of the object reference's `/Pg`, Table 358:
//!
//! > This entry overrides any Pg entry in the structure element containing the object reference;
//! > it shall be used if the structure element has no such entry.
//!
//! and Table 357 states the same of the marked-content reference. A photographed page's content
//! stream is replaced by one image (`super::photographed`), so a marked-content sequence a leaf
//! names on that page is no longer anywhere, and an `XObject` only that content drew is drawn by
//! nothing. Those leaves are the ones taken out; every other leaf, every element (an element that
//! loses all its leaves is kept, with no `/K`) and the parent tree stay as they are. The loss is
//! [`crate::archive::Loss::TaggingOfPhotographedPages`], counted per page.
//!
//! # What stays, and why
//!
//! - **A leaf whose element names no page.** An integer's page is its element's own `/Pg`; the
//!   standard does not hand an element's page down to its children, so an integer whose element
//!   names none names no page this converter can find, and it is left as it is.
//! - **An object reference to anything but a stream, or a leaf in an annotation's appearance.**
//!   An annotation stays on the photographed page with its appearance (§12.5.5), so the object it
//!   is, the appearance streams its `/AP` names and a marked-content reference whose `/Stm` is
//!   one of them are still drawn there. A form `XObject` an appearance draws in turn is not
//!   followed: a leaf that names it as its `/Stm` goes with the page, a loss counted like the
//!   others rather than one left unstated.
//! - **The parent tree** (§14.7.5.4). Its entry for the photographed page's `/StructParents`
//!   names elements that are kept; the page loses its `/StructParents` (`super::super::rewrite`),
//!   so the entry is reached from nothing that is drawn, and a reader looking up content of the
//!   page finds none to look up.
//!
//! # The walk
//!
//! Over the tree's indirect elements with an explicit stack and a set of those visited, because a
//! document's graph is the file's to shape and a cycle in it is a malformed file rather than an
//! infinite one. Direct elements nest inside one object, so their depth is bounded, and the walk
//! counts every element it reads against [`MAX_ELEMENTS`].

use std::collections::{BTreeMap, BTreeSet};

use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};

use super::super::decision::Because;

/// How many structure elements one conversion reads: well above the tree of a tagged book of a
/// few thousand pages, and a bound on what a hostile tree costs.
const MAX_ELEMENTS: usize = 1 << 20;

/// How deep direct elements nest inside one object before the walk stops.
const MAX_DEPTH: usize = 64;

/// What the tree lost.
#[derive(Debug, Default)]
pub(super) struct Untagged {
    /// Every element that lost a leaf, under its own object number, as it is now written.
    pub(super) written: Vec<(ObjectId, Object)>,
    /// How many leaves each photographed page lost, by the page's object.
    pub(super) leaves: BTreeMap<ObjectId, usize>,
}

/// Takes every leaf that names one of `pages` out of the document's structure tree.
///
/// # Errors
///
/// A sentence naming the bound, where the tree holds more elements than [`MAX_ELEMENTS`] or
/// nests direct elements deeper than [`MAX_DEPTH`]; and one where the tree's root is a direct
/// dictionary of the catalog's whose own leaves would change, which is not written here.
pub(super) fn untag(document: &Document, pages: &BTreeSet<ObjectId>) -> Result<Untagged, Because> {
    let catalog = document.catalog().unwrap_or_default();
    let mut walk = Walk {
        document,
        pages,
        appearances: BTreeMap::new(),
        pending: Vec::new(),
        seen: BTreeSet::new(),
        leaves: BTreeMap::new(),
        read: 0,
    };
    match catalog.get("StructTreeRoot") {
        Some(Object::Reference(root)) => {
            walk.seen.insert(*root);
            walk.pending.push(*root);
        }
        Some(Object::Dictionary(root)) => {
            if walk.element(root, 0)?.is_some() {
                return Err(Because::NotBuiltYet(DIRECT_ROOT));
            }
        }
        _ => return Ok(Untagged::default()),
    }
    let mut written = Vec::new();
    while let Some(id) = walk.pending.pop() {
        let Object::Dictionary(element) = document.get(id) else {
            continue;
        };
        if let Some(pruned) = walk.element(&element, 0)? {
            written.push((id, Object::Dictionary(pruned)));
        }
    }
    Ok(Untagged {
        written,
        leaves: walk.leaves,
    })
}

/// What one item of a `/K` entry becomes.
enum Kid {
    /// It stays as it is.
    Keep,
    /// It names a photographed page and goes.
    Remove,
    /// It is a direct element that lost a leaf of its own.
    Replace(Object),
}

/// The three kinds of item §14.7.5 puts in a `/K` entry, besides an integer.
enum Item {
    /// A structure element, whose own `/K` is walked.
    Element,
    /// A marked-content reference (Table 357).
    MarkedContent,
    /// An object reference (Table 358).
    WholeObject,
}

struct Walk<'a> {
    document: &'a Document,
    pages: &'a BTreeSet<ObjectId>,
    /// The streams each photographed page's annotations name as appearances, read once a page.
    appearances: BTreeMap<ObjectId, BTreeSet<ObjectId>>,
    pending: Vec<ObjectId>,
    seen: BTreeSet<ObjectId>,
    leaves: BTreeMap<ObjectId, usize>,
    read: usize,
}

impl Walk<'_> {
    /// The element with the leaves that name a photographed page removed, or `None` where it
    /// keeps every leaf.
    fn element(
        &mut self,
        element: &Dictionary,
        depth: usize,
    ) -> Result<Option<Dictionary>, Because> {
        self.read = self.read.saturating_add(1);
        if self.read > MAX_ELEMENTS {
            return Err(Because::NotBuiltYet(TOO_MANY_ELEMENTS));
        }
        let page = page_of(element);
        let Some(kids) = element.get("K") else {
            return Ok(None);
        };
        let items = match kids {
            Object::Array(items) => items.as_slice(),
            single => std::slice::from_ref(single),
        };
        let mut kept = Vec::with_capacity(items.len());
        let mut changed = false;
        for item in items {
            match self.kid(item, page, depth)? {
                Kid::Keep => kept.push(item.clone()),
                Kid::Remove => changed = true,
                Kid::Replace(replaced) => {
                    kept.push(replaced);
                    changed = true;
                }
            }
        }
        if !changed {
            return Ok(None);
        }
        let mut pruned = element.clone();
        if kept.is_empty() {
            pruned.remove("K");
        } else {
            pruned.insert(Name::new(&b"K"[..]), Object::Array(kept));
        }
        Ok(Some(pruned))
    }

    fn kid(&mut self, kid: &Object, page: Option<ObjectId>, depth: usize) -> Result<Kid, Because> {
        match kid {
            Object::Integer(_) => Ok(self.leaf(page)),
            Object::Reference(id) => {
                let Object::Dictionary(dict) = self.document.get(*id) else {
                    return Ok(Kid::Keep);
                };
                Ok(match item(&dict) {
                    Item::Element => {
                        if self.seen.insert(*id) {
                            self.pending.push(*id);
                        }
                        Kid::Keep
                    }
                    Item::MarkedContent => self.marked_content(&dict, page),
                    Item::WholeObject => self.whole_object(&dict, page),
                })
            }
            Object::Dictionary(dict) => match item(dict) {
                Item::Element => {
                    if depth >= MAX_DEPTH {
                        return Err(Because::NotBuiltYet(TOO_DEEP));
                    }
                    Ok(match self.element(dict, depth.saturating_add(1))? {
                        Some(pruned) => Kid::Replace(Object::Dictionary(pruned)),
                        None => Kid::Keep,
                    })
                }
                Item::MarkedContent => Ok(self.marked_content(dict, page)),
                Item::WholeObject => Ok(self.whole_object(dict, page)),
            },
            _ => Ok(Kid::Keep),
        }
    }

    /// A marked-content leaf on `page`: removed and counted where the page was photographed.
    fn leaf(&mut self, page: Option<ObjectId>) -> Kid {
        match page {
            Some(page) if self.pages.contains(&page) => {
                let count = self.leaves.entry(page).or_insert(0);
                *count = count.saturating_add(1);
                Kid::Remove
            }
            _ => Kid::Keep,
        }
    }

    /// A marked-content reference: removed where its page was photographed, unless its `/Stm`
    /// names an annotation's appearance, which stays on the page (Table 357's `/Stm`).
    fn marked_content(&mut self, reference: &Dictionary, page: Option<ObjectId>) -> Kid {
        let page = page_of(reference).or(page);
        if let (Some(page), Some(Object::Reference(stream))) = (page, reference.get("Stm"))
            && self.pages.contains(&page)
            && self.is_appearance(page, *stream)
        {
            return Kid::Keep;
        }
        self.leaf(page)
    }

    /// An object reference: removed where its page was photographed and the object is a stream
    /// no annotation of the page names as an appearance, so that only the old content drew it.
    fn whole_object(&mut self, reference: &Dictionary, page: Option<ObjectId>) -> Kid {
        let Some(page) = page_of(reference).or(page) else {
            return Kid::Keep;
        };
        if !self.pages.contains(&page) {
            return Kid::Keep;
        }
        let Some(Object::Reference(object)) = reference.get("Obj") else {
            return Kid::Keep;
        };
        if !matches!(self.document.get(*object), Object::Stream(_))
            || self.is_appearance(page, *object)
        {
            return Kid::Keep;
        }
        self.leaf(Some(page))
    }

    /// Whether an annotation of `page` names `stream` as an appearance.
    fn is_appearance(&mut self, page: ObjectId, stream: ObjectId) -> bool {
        let document = self.document;
        self.appearances
            .entry(page)
            .or_insert_with(|| appearances_of(document, page))
            .contains(&stream)
    }
}

/// Which of the three dictionaries a `/K` entry may hold this is, by its `/Type`. ISO 32000-2
/// §14.7.2, Table 355's `/K`:
///
/// > If the value of K is a dictionary containing no Type entry, it shall be assumed to be a
/// > structure element dictionary.
fn item(dict: &Dictionary) -> Item {
    match dict
        .get("Type")
        .and_then(Object::as_name)
        .map(Name::as_bytes)
    {
        Some(b"MCR") => Item::MarkedContent,
        Some(b"OBJR") => Item::WholeObject,
        _ => Item::Element,
    }
}

/// The page a dictionary's own `/Pg` names, which Tables 355 (§14.7.2), 357 and 358 (§14.7.5)
/// require to be an indirect reference.
fn page_of(dict: &Dictionary) -> Option<ObjectId> {
    match dict.get("Pg") {
        Some(Object::Reference(page)) => Some(*page),
        _ => None,
    }
}

/// The streams the annotations of `page` name as their normal, rollover and down appearances,
/// directly or under an appearance state (ISO 32000-2 §12.5.5).
fn appearances_of(document: &Document, page: ObjectId) -> BTreeSet<ObjectId> {
    let mut named = BTreeSet::new();
    let Object::Dictionary(page) = document.get(page) else {
        return named;
    };
    let Object::Array(annotations) = document.resolve(&document.get_key(&page, "Annots")) else {
        return named;
    };
    for annotation in &annotations {
        let Object::Dictionary(annotation) = document.resolve(annotation) else {
            continue;
        };
        let Object::Dictionary(appearance) = document.resolve(&document.get_key(&annotation, "AP"))
        else {
            continue;
        };
        for key in ["N", "R", "D"] {
            match appearance.get(key) {
                Some(Object::Reference(stream)) => {
                    if let Object::Dictionary(states) = document.get(*stream) {
                        named.extend(states.iter().filter_map(|(_, state)| match state {
                            Object::Reference(state) => Some(*state),
                            _ => None,
                        }));
                    } else {
                        named.insert(*stream);
                    }
                }
                Some(Object::Dictionary(states)) => {
                    named.extend(states.iter().filter_map(|(_, state)| match state {
                        Object::Reference(state) => Some(*state),
                        _ => None,
                    }));
                }
                _ => {}
            }
        }
    }
    named
}

/// The tree holds more elements than one conversion reads.
const TOO_MANY_ELEMENTS: &str = "this document's structure tree holds more elements than one \
     conversion reads to take a photographed page's leaves out of it";

/// Direct elements nest past the bound.
const TOO_DEEP: &str = "this document's structure tree nests elements written inside one object \
     deeper than one conversion reads to take a photographed page's leaves out of it";

/// The root is a direct dictionary whose own leaves would change.
const DIRECT_ROOT: &str = "this document's structure tree root is written inside the catalog and \
     holds leaves that name a photographed page directly, which this conversion does not rewrite";
