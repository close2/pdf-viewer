//! The structural parent tree rebuilt from the structure tree, where a source's loses a content item.
//!
//! The structure tree is written downward — an element's `/K` names its content items — and the
//! parent tree is the same association written the other way, because a content stream cannot
//! name the element it belongs to. ISO 32000-2 §14.7.5.4:
//!
//! > Because a stream cannot contain object references, there is no way for content items that
//! > are markedcontent sequences to refer directly back to their parent structure elements (the
//! > ones to which they belong as content items). Instead, a different mechanism, the structural
//! > parent tree , shall be provided for this purpose.
//!
//! So every value this module writes is read off an element that already claims the item, and
//! nothing is invented: no element, no content stream, no mark. `pdf_archive`'s row
//! `logical-structure/content-items-find-their-elements-through-the-parent-tree` finds the holders
//! whose route back is broken, and this module repairs exactly those (`doc/adr/2674`).
//!
//! # The construction
//!
//! | a holder whose | takes |
//! |---|---|
//! | key is right | nothing — a producer key that leads back is never changed |
//! | key is missing | a new key from `/ParentTreeNextKey`, and an entry under it |
//! | key leads to no entry, to a value of the wrong shape, or to one naming no claimant | a new key and a new entry; the producer's entry stays, leading nowhere |
//! | key is the other one of Table 359's two | the right one, and the wrong one taken out |
//!
//! **The structure tree is the authority where the two disagree**, because the clause makes the
//! parent tree the means of finding "their parent structure elements": it is defined *from* the
//! elements' claims, and a value naming an element that claims nothing is one the clause's own
//! derivation cannot produce. A sequences entry keeps every slot of the producer's array that
//! already names a claimant, and every slot no element claims, so only the slots the structure
//! tree contradicts change. A new key rather than an edit of the producer's entry, because the new
//! entries then extend the tree after its greatest key through [`NumberTree::extended`], whatever
//! its depth, and no producer node or `/Limits` is rewritten (`doc/adr/2672`); an entry two holders
//! share stays right for the one it was right for.
//!
//! # What is still refused, by name
//!
//! What the walk cannot read or the clause cannot satisfy: a structure tree past
//! [`MOST_STRUCTURE_DEPTH`] levels or [`MOST_STRUCTURE_VISITS`] elements and items, an object
//! claimed both as a content item and as the holder of claimed sequences (Table 359: "not both"),
//! an item only elements written inside their parent's `/K` claim, which have no reference an
//! entry could hold, and a document past [`MOST_KEYED_OBJECTS`] pages, annotations and `XObject`s,
//! whose stated keys the walk could not all read — a new key has to be past every key any object
//! states, claimed or not, or that object would find an element that never claimed it. The parent tree's own unreadable shapes are [`super::tagged`]'s refusals.
//!
//! [`NumberTree::extended`]: super::preserve::NumberTree::extended

use std::collections::{BTreeMap, BTreeSet};

use pdf_model::structure::{Child, Tree};
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Object, ObjectId};

use super::decision::Because;
use super::prepare::Spare;
use super::tagged::{Described, Planned};

/// How deep the structure walk descends: `pdf_model`'s own bound, and the checker's.
const MOST_STRUCTURE_DEPTH: usize = 64;

/// How many elements and content items the structure walk visits: the checker's bound.
const MOST_STRUCTURE_VISITS: usize = 1 << 20;

/// Why a structure tree too large to read gets no rebuilt parent tree.
pub(super) const STRUCTURE_TOO_LARGE: &str = "this document's structure tree is deeper than 64 \
     levels or holds more than 1048576 elements and content items, which is past what this \
     conversion reads, so the parent tree ISO 32000-2 section 14.7.5.4 requires cannot be rebuilt \
     from it";

/// Why an object that is both a content item and a holder of claimed sequences is refused.
pub(super) const ITEM_AND_CONTAINER: &str = "a structure element in this document claims an \
     object as a content item in its entirety while another claims marked-content sequences inside \
     it, and ISO 32000-2 Table 359 lets an object be one or the other but not both, so no \
     StructParent or StructParents key this conversion could write would be true of it";

/// Why an item only inline elements claim is refused.
pub(super) const CLAIMED_INLINE_ONLY: &str = "a content item in this document is claimed only by \
     structure elements written directly inside their parent's K rather than as objects of their \
     own, and ISO 32000-2 section 14.7.5.4 makes a parent tree entry an indirect reference to the \
     element, which such an element does not have; giving it one would rewrite the producer's \
     structure tree";

/// How many pages, annotations and `XObject`s [`greatest_stated_key`] reads before it stops.
const MOST_KEYED_OBJECTS: usize = 1 << 20;

/// Why a document too large to read every stated key of gets no rebuilt parent tree.
pub(super) const KEYS_TOO_MANY: &str = "this document holds more than 1048576 pages, annotations \
     and XObjects, which is past what this conversion reads, so it cannot know that a key it would \
     give a content item is one no other object already states, and a new parent tree entry under \
     a key another object states would lead that object to an element that never claimed it";

/// Which of Table 359's two entries a holder states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Holds {
    /// `/StructParents`: a page or stream holding claimed marked-content sequences.
    Sequences,
    /// `/StructParent`: an object claimed as a content item in its entirety.
    Object,
}

impl Holds {
    /// The entry this holder states, and the one it may not state beside it.
    #[must_use]
    pub(super) const fn keys(self) -> (&'static str, &'static str) {
        match self {
            Self::Sequences => ("StructParents", "StructParent"),
            Self::Object => ("StructParent", "StructParents"),
        }
    }
}

/// The elements claiming one content item, in the order the walk met them.
#[derive(Debug, Default)]
struct Claimants {
    /// The first claiming element written as an object of its own: the one a new entry names.
    first: Option<ObjectId>,
    /// Every claiming element written as an object of its own.
    indirect: BTreeSet<ObjectId>,
    /// Whether an element written inside its parent's `/K` claims it too.
    inline: bool,
}

impl Claimants {
    fn add(&mut self, element: Option<ObjectId>) {
        match element {
            Some(id) => {
                self.first.get_or_insert(id);
                self.indirect.insert(id);
            }
            None => self.inline = true,
        }
    }

    /// Whether a value the producer wrote names one of these elements: the checker's reading,
    /// so that what this module leaves alone is what the row passes.
    fn named_by(&self, document: &Document, value: Option<&Object>) -> bool {
        match value {
            Some(Object::Reference(id)) => {
                self.indirect.contains(id) || (self.inline && document.get(*id).as_dict().is_some())
            }
            Some(Object::Dictionary(_)) => self.inline,
            _ => false,
        }
    }

    /// The element a new entry names, or the refusal for an item no indirect element claims.
    fn named(&self) -> Result<Object, Because> {
        self.first
            .map(Object::Reference)
            .ok_or(Because::NotBuiltYet(CLAIMED_INLINE_ONLY))
    }
}

/// Every content item the structure tree claims, by the object that holds its key.
#[derive(Debug, Default)]
struct Claims {
    /// §14.7.5.2's sequences: the page or stream they are in, then the `/MCID`.
    sequences: BTreeMap<ObjectId, BTreeMap<i64, Claimants>>,
    /// §14.7.5.3's objects, by the object.
    objects: BTreeMap<ObjectId, Claimants>,
}

impl Claims {
    /// Walks the structure tree depth first, each indirect element once, in `/K` order.
    fn gather(document: &Document, tree: &Tree) -> Result<Self, Because> {
        let too_large = || Because::NotBuiltYet(STRUCTURE_TOO_LARGE);
        let mut claims = Self::default();
        let mut seen: BTreeSet<ObjectId> = BTreeSet::new();
        let mut visits = 0_usize;
        let mut stack: Vec<(Option<ObjectId>, Dictionary, usize)> = Vec::new();
        // The root's own `/K` names elements only (Table 354), so an item there has no parent
        // element to be found; reversed so that the first child is the first popped.
        for (child, id) in tree.identified_children(document, None).into_iter().rev() {
            if let Child::Element(dict) = child
                && id.is_none_or(|id| seen.insert(id))
            {
                stack.push((id, dict, 0));
            }
        }
        while let Some((element, dict, depth)) = stack.pop() {
            if depth >= MOST_STRUCTURE_DEPTH {
                return Err(too_large());
            }
            let mut below = Vec::new();
            for (child, id) in tree.identified_children(document, Some(&dict)) {
                visits = visits.saturating_add(1);
                if visits > MOST_STRUCTURE_VISITS {
                    return Err(too_large());
                }
                match child {
                    Child::Element(child) => {
                        if id.is_none_or(|id| seen.insert(id)) {
                            below.push((id, child, depth.saturating_add(1)));
                        }
                    }
                    Child::MarkedContent {
                        mcid, page, stream, ..
                    } => {
                        // An item with neither `/Stm` nor `/Pg` has no object to hold its key,
                        // and the checker passes it over for the same reason.
                        if let Some(holder) = stream.or(page) {
                            claims
                                .sequences
                                .entry(holder)
                                .or_default()
                                .entry(mcid)
                                .or_default()
                                .add(element);
                        }
                    }
                    Child::Object { object, .. } => {
                        claims.objects.entry(object).or_default().add(element);
                    }
                }
            }
            stack.extend(below.into_iter().rev());
        }
        Ok(claims)
    }
}

/// The greatest `/StructParents` or `/StructParent` any page, annotation or `XObject` states.
///
/// §14.7.5.4 places the entry in a page object, in the stream dictionary of a form or image
/// `XObject`, or in an annotation dictionary, and nowhere else. The walk reads every page with its inherited resources, each
/// page's annotations and their appearance streams, and every `XObject` those resources and
/// appearances name, each once.
///
/// # Errors
///
/// [`Because::NotBuiltYet`] with [`KEYS_TOO_MANY`] past [`MOST_KEYED_OBJECTS`].
fn greatest_stated_key(document: &Document) -> Result<Option<i64>, Because> {
    let mut greatest: Option<i64> = None;
    let mut read = 0_usize;
    let mut note = |dict: &Dictionary| -> Result<(), Because> {
        read = read.saturating_add(1);
        if read > MOST_KEYED_OBJECTS {
            return Err(Because::NotBuiltYet(KEYS_TOO_MANY));
        }
        for key in ["StructParents", "StructParent"] {
            if let Some(value) = document.get_key(dict, key).as_integer() {
                greatest = Some(greatest.map_or(value, |held| held.max(value)));
            }
        }
        Ok(())
    };
    let mut seen: BTreeSet<ObjectId> = BTreeSet::new();
    let mut streams: Vec<Object> = Vec::new();
    // Every `XObject` a resource dictionary names, and every stream an appearance dictionary
    // reaches (§12.5.5: a stream, or a dictionary of streams by appearance state).
    let named_xobjects = |resources: &Dictionary, streams: &mut Vec<Object>| {
        if let Some(xobjects) = document.get_key(resources, "XObject").as_dict() {
            streams.extend(xobjects.iter().map(|(_, value)| value.clone()));
        }
    };
    let pages = pdf_model::Pages::new(document);
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        note(&page.dict)?;
        named_xobjects(&page.resources, &mut streams);
        let annotations = document.get_key(&page.dict, "Annots");
        for annotation in annotations.as_array().into_iter().flatten() {
            if let Some(id) = annotation.as_reference()
                && !seen.insert(id)
            {
                continue;
            }
            let annotation = document.resolve(annotation);
            let Some(annotation) = annotation.as_dict() else {
                continue;
            };
            note(annotation)?;
            let appearances = document.get_key(annotation, "AP");
            for (_, appearance) in appearances.as_dict().into_iter().flat_map(Dictionary::iter) {
                match appearance {
                    Object::Reference(_) | Object::Stream(_) => streams.push(appearance.clone()),
                    Object::Dictionary(states) => {
                        streams.extend(states.iter().map(|(_, value)| value.clone()));
                    }
                    _ => {}
                }
            }
        }
    }
    let mut popped = 0_usize;
    while let Some(stream) = streams.pop() {
        popped = popped.saturating_add(1);
        if popped > MOST_KEYED_OBJECTS {
            return Err(Because::NotBuiltYet(KEYS_TOO_MANY));
        }
        if let Some(id) = stream.as_reference()
            && !seen.insert(id)
        {
            continue;
        }
        let stream = document.resolve(&stream);
        let Some(dict) = stream.as_dict() else {
            continue;
        };
        // An appearance-state dictionary reached through a reference holds streams, not keys.
        if !matches!(stream, Object::Stream(_)) {
            streams.extend(dict.iter().map(|(_, value)| value.clone()));
            continue;
        }
        note(dict)?;
        if let Some(resources) = document.get_key(dict, "Resources").as_dict() {
            named_xobjects(resources, &mut streams);
        }
    }
    Ok(greatest)
}

/// The keys and entries a source's parent tree gains so that every content item finds its element.
///
/// Built before any page is appended, because an appended page's key comes after these
/// ([`Planned::rebuilding`]); its edits to the structure tree root and the parent tree's root
/// ride in the appended pages' [`Described`] where pages are appended, and in [`Self::alone`]'s
/// where none are, so the two never write the same dictionary twice.
#[derive(Debug, Default)]
pub(super) struct Rebuilt {
    /// Each producer object that takes a key, which entry, and the key.
    pub(super) keys: BTreeMap<ObjectId, (Holds, i64)>,
    /// The entries under those keys, ascending from the producer's `/ParentTreeNextKey`.
    pub(super) pairs: Vec<(i64, Object)>,
    /// The `/ParentTreeNextKey` past every key this rebuild used.
    pub(super) next_key: i64,
    /// The root edits, where no appended page carries them.
    pub(super) described: Option<Described>,
    /// The objects those edits add — a new parent-tree leaf where the root holds `/Kids`.
    pub(super) written: Vec<(ObjectId, Object)>,
}

impl Rebuilt {
    /// The keys and entries every holder the parent tree loses takes, read from the structure tree.
    ///
    /// Empty where the document has no structure tree or no holder is lost.
    ///
    /// # Errors
    ///
    /// [`Because::NotBuiltYet`] with [`STRUCTURE_TOO_LARGE`], [`ITEM_AND_CONTAINER`],
    /// [`CLAIMED_INLINE_ONLY`] or [`KEYS_TOO_MANY`], or with one of [`Planned::of`]'s refusals of a parent tree it
    /// cannot read.
    pub(super) fn of(document: &Document) -> Result<Self, Because> {
        let Some(planned) = Planned::of(document)? else {
            return Ok(Self::default());
        };
        let Some(tree) = Tree::of(document) else {
            return Ok(Self::default());
        };
        let claims = Claims::gather(document, &tree)?;
        let entries = planned.entries();
        // §14.7.5.4's `/ParentTreeNextKey` is past every key the tree holds; any page, annotation
        // or `XObject` may state a key the tree does not hold, claimed or not, and a new entry
        // under it would be found from that object too — a link to an element that never claimed
        // it — so the new keys start past every key the document states.
        let stated = greatest_stated_key(document)?;
        let mut rebuilt = Self {
            next_key: planned
                .next_key()
                .max(stated.map_or(0, |key| key.saturating_add(1))),
            ..Self::default()
        };
        for (holder, sequences) in &claims.sequences {
            if claims.objects.contains_key(holder) {
                return Err(Because::NotBuiltYet(ITEM_AND_CONTAINER));
            }
            let value = document.get(*holder);
            let Some(dict) = value.as_dict() else {
                continue;
            };
            let stated = document.get_key(dict, "StructParents").as_integer();
            let array: Vec<Object> = stated
                .and_then(|key| entries.get(&key))
                .and_then(|entry| document.resolve(entry).as_array().map(<[Object]>::to_vec))
                .unwrap_or_default();
            let found = |mcid: i64| usize::try_from(mcid).ok().and_then(|at| array.get(at));
            let lost: Vec<(i64, &Claimants)> = sequences
                .iter()
                .filter(|(mcid, claimants)| !claimants.named_by(document, found(**mcid)))
                .map(|(mcid, claimants)| (*mcid, claimants))
                .collect();
            if lost.is_empty() && stated.is_some() {
                continue;
            }
            let mut array = array;
            for (mcid, claimants) in lost {
                let Ok(index) = usize::try_from(mcid) else {
                    continue;
                };
                if array.len() <= index {
                    array.resize(index.saturating_add(1), Object::Null);
                }
                if let Some(slot) = array.get_mut(index) {
                    *slot = claimants.named()?;
                }
            }
            rebuilt.take(*holder, Holds::Sequences, Object::Array(array));
        }
        for (object, claimants) in &claims.objects {
            let value = document.get(*object);
            let Some(dict) = value.as_dict() else {
                continue;
            };
            let stated = document.get_key(dict, "StructParent").as_integer();
            if claimants.named_by(document, stated.and_then(|key| entries.get(&key))) {
                continue;
            }
            rebuilt.take(*object, Holds::Object, claimants.named()?);
        }
        Ok(rebuilt)
    }

    /// Gives one holder the next key and its entry.
    fn take(&mut self, holder: ObjectId, holds: Holds, entry: Object) {
        let key = self.next_key;
        self.next_key = self.next_key.saturating_add(1);
        self.keys.insert(holder, (holds, key));
        self.pairs.push((key, entry));
    }

    /// How many entries the parent tree gains: what the report counts.
    #[must_use]
    pub(super) fn len(&self) -> usize {
        self.pairs.len()
    }

    /// Whether the parent tree loses nothing.
    #[must_use]
    pub(super) fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    /// Builds the root edits on their own, for a conversion that appends no page to carry them.
    ///
    /// # Errors
    ///
    /// [`Because::NotBuiltYet`] with [`super::preserve::NO_SPARE_OBJECT`] where no object number
    /// is left for a new leaf, or with [`Planned::of`]'s refusals.
    pub(super) fn alone(&mut self, document: &Document, spare: &mut Spare) -> Result<(), Because> {
        if self.is_empty() {
            return Ok(());
        }
        let Some(mut planned) = Planned::of(document)? else {
            return Ok(());
        };
        planned.rebuilding(self);
        let (described, written) = planned.finish(document, spare, true)?;
        self.described = Some(described);
        self.written = written;
        Ok(())
    }

    /// What `xmpMM:History` records, where anything was rebuilt.
    #[must_use]
    pub(super) fn history(&self) -> Option<String> {
        (!self.is_empty()).then(|| {
            format!(
                "ISO 32000-2 14.7.5.4: the structural parent tree was rebuilt from the structure \
                 tree, {} entr{} written so that every content item finds its element",
                self.len(),
                if self.len() == 1 { "y" } else { "ies" }
            )
        })
    }
}
