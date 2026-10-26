//! ISO 32000-2 §14.7's logical structure, carried into a document derived from others.
//!
//! §14.7.1 states the shape of the thing in one sentence, and both of its halves are this
//! module's subject:
//!
//! > A PDF document's logical structure shall be stored separately from its visible content,
//! > with pointers from each to the other.
//!
//! The structure is the catalog's `/StructTreeRoot` and the hierarchy of elements under it
//! (§14.7.2); the pointers from the content back to it are §14.7.5.4's *parent tree*, keyed by
//! the `/StructParents` a page states and by the `/StructParent` an annotation or an `XObject`
//! states. A verb that carries pages out of one document into another has to carry both, and the
//! pointers are what make it more than a copy: the keys are the source document's numbering and
//! the output has its own.
//!
//! # What crosses, and what does not
//!
//! **The marked-content identifiers inside the carried content streams are not rewritten**, and
//! that is a claim to check rather than to assume. §14.7.5.2 makes an `/MCID` "an integer
//! marked-content identifier that uniquely identifies the marked-content sequence within its
//! content stream", and §14.7.5.4 says what the number is then used for:
//!
//! > The array element corresponding to each sequence shall be found by using the sequence's
//! > marked -content identifier as a zero-based index into the array.
//!
//! So an identifier means nothing outside the stream that holds it and the one array that
//! stream's key names. RFC 0002 section 11.1 makes every content stream cross byte for byte, and this
//! module carries each page's parent-tree array **at its own length and in its own order** —
//! only the *key* is renumbered. Both ends of the index therefore move together, and no
//! identifier in any content stream has to change. The corpus walks assert the consequence:
//! every carried page's key resolves in the output's parent tree, and every carried element's
//! `/Pg` names a page the output holds.
//!
//! # Three namespaces two sources can collide in, and three different answers
//!
//! Table 354's `/RoleMap`, `/ClassMap` and `/IDTree` are each a map whose keys two documents can
//! both use. They are **not** one problem, and the clauses say why they are not:
//!
//! - **`/RoleMap` — the first source's mapping wins, and the collision is warned about by name.**
//!   §14.7.3 makes the value "a single name identifying the target structure element type", so
//!   two cannot be written; but its NOTE 1 says what the value *is*: "\[t]he equivalence need not
//!   be exact; the role map merely indicates an approximate analogy between types, allowing PDF
//!   processors to share nonstandard structure elements in a reasonable way." An approximation is
//!   not an assertion two documents can contradict each other about, and a type left mapped to
//!   the first source's nearest equivalent is still shared in a reasonable way — where refusing
//!   would leave the reader with no structure at all.
//! - **`/ClassMap` — a colliding class is renamed, and every carried element's `/C` is
//!   rewritten.** An attribute class is not an approximation: §14.7.6.2 makes the attributes
//!   "attached to the given structure element", so an element given another source's `/Pa5` would
//!   be laid out by properties it never had. The rename is safe because the clause closes the set
//!   of things that name a class — "\[s]tructure elements shall refer to the class by name", and
//!   "\[t]he C entry in a structure element dictionary … shall contain a class name or an array of
//!   class names" — which is ADR 0821 section 3's rule for `/Dests`: chase the references where the
//!   standard says what states them.
//! - **`/IDTree` — refused by name.** Table 355 makes an element's `/ID` "unique among all
//!   elements in the document's structure hierarchy" and the derived document is one hierarchy,
//!   so two elements cannot both keep it. A rename is what the class map got, and here the same
//!   rule forbids it: §14.8.5's `/Headers` attribute is "an array of byte strings, where each
//!   byte string shall be the element identifier", Annex E permits further attributes, and this
//!   program does not know what else names an identifier. So it is
//!   [`Refusal::StructureConflict`], naming the clause and every colliding identifier, at RFC
//!   0002 section 4.4's exit status 4 — §12.7.4.2's precedent (ADR 0821 section 2): the request is
//!   well formed and this program declines to write a document a clause forbids.
//!
//! Where two sources agree on a key, the entry is carried once and nothing is reported.
//!
//! # One `Document` under the root, which Annex L requires
//!
//! Table L.2's first row gives `StructTreeRoot` one child type, `Document`, at an occurrence of
//! `1`; the legend defines `1..n` and `0..1` beside it, so the bare `1` reads as exactly one. Two
//! sources whose roots each hold a PDF 2.0 `Document` would therefore give the merged root two,
//! and the annex binds them: "\[e]lements in the standard structure namespace for PDF 2.0 shall
//! not have child or parent elements in the standard structure namespace for PDF 2.0 that are not
//! explicitly listed in Table L.2". So where the root's top-level elements include PDF 2.0
//! `Document`s from more than one source, they are written inside **one** `Document` of that
//! namespace, each one's `/P` naming it — Table L.2 lists `Document` among a `Document`'s
//! children at `0..n`, and Table 364's EXAMPLE 2 is this shape exactly: "the PDF at the top level
//! is one document containing several documents". Every source element crosses as the producer
//! typed it, so a carried `Document` is not re-typed `DocumentFragment` (ADR 1474 says why), and a
//! source whose own root already held two crosses as written — the annex binds the file that held
//! them, which is ADR 0821 section 2's distinction again.
//!
//! §14.7.4's namespaces are the construction that would resolve the first two without a choice:
//! Table 356's `/RoleMapNS` maps one namespace's types to another's, so each source could keep
//! its own mapping under its own namespace. It is **not** taken, because a namespace name "should
//! take the form of a uniform resource identifier" and this program has no basis for inventing
//! one per source — the same reason ADR 0821 section 4 gives for not synthesising an outline item's
//! `/Title`.
//!
//! # Where the code is
//!
//! This file holds the types, the [`Host`] seam and [`Carry::plan`]; `impl Carry` continues in
//! `reconcile` (the root's entries), `hierarchy` (which elements are kept), `wrapper` (Annex L's
//! `Document`), `keys` (the parent-tree keys), `write` (the rebuilt tree) and `report` (the
//! warnings).

use std::collections::{BTreeMap, BTreeSet};

use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Object, ObjectId};
use pdf_syntax::serialize::AssemblyError;

use crate::{Refusal, Warning};

mod hierarchy;
mod keys;
mod reconcile;
mod report;
mod wrapper;
mod write;

/// How deep the structure hierarchy is walked.
///
/// §14.7.2 puts no bound on the hierarchy's height, so this one is the parser's `max_depth`: a
/// `/K` chain deeper than that is a file whose own objects the reader would have refused to
/// nest. What stops the walk in practice is the visited set — a `/K` cycle is what a hostile
/// file writes, and an element reached twice is not descended twice.
const MAX_DEPTH: usize = 256;

/// Table 354's entries that are arrays and concatenate across sources.
///
/// `/Namespaces` is "\[a]n array of namespaces used within the document", `/PronunciationLexicon`
/// "one or more indirect references to file specification dictionaries", `/AF` "one or more file
/// specification dictionaries" — three lists, none of them a namespace two sources can collide
/// in, so the union is the whole reconciliation.
const LIST_VALUED: [&str; 3] = ["Namespaces", "PronunciationLexicon", "AF"];

/// One page of the output, as the structure carry needs to see it.
pub(crate) struct CarriedPage {
    /// The document's position among the opened ones.
    pub(crate) at: usize,
    /// The source page object.
    pub(crate) source: ObjectId,
    /// Its number in the output.
    pub(crate) placed: ObjectId,
    /// Whether an earlier placement already stands in for this source page.
    pub(crate) duplicate: bool,
}

/// What a verb's own object table lets this module do.
///
/// `split` builds one piece out of one document and `merge` builds one file out of several; they
/// hold different walk state and neither's is this module's business. What both can answer is
/// the questions below, so §14.7 is read and written **once** rather than once per verb.
pub(crate) trait Host {
    /// The document at this position among the opened ones.
    fn source(&self, at: usize) -> Option<&Document>;
    /// One value with every reference mapped into the output's numbering.
    fn carry_value(&mut self, at: usize, value: &Object) -> Object;
    /// A number for an object this module will build.
    ///
    /// # Errors
    ///
    /// The assembly's own, unaltered — see [`Host::replace_object`] for why that matters.
    fn reserve_slot(&mut self) -> Result<ObjectId, AssemblyError>;
    /// A number that **stands in for** a source object, so that references to it map here.
    ///
    /// # Errors
    ///
    /// The assembly's own, unaltered. **This used to be an `Option`, and the `None` was reported
    /// as "the derived document needs more objects than one file can number"** — a sentence about
    /// `u32::MAX` that a five-page document could produce, because
    /// [`AssemblyError::AlreadyPlaced`] arrives here too and was flattened into it. Principle 1:
    /// an error is propagated, not renamed.
    fn replace_object(&mut self, at: usize, id: ObjectId) -> Result<ObjectId, AssemblyError>;
    /// Fills a slot this module reserved.
    fn place_object(&mut self, id: ObjectId, object: Object);
    /// Refuses a source object to the closure walk, so that nothing drags it in by reference.
    fn block_object(&mut self, at: usize, id: ObjectId);
}

/// One kept child of a structure element, before the output's numbering is known.
enum Child {
    /// Another structure element, named by its object in the source.
    Element(ObjectId),
    /// §14.7.5.2's second form: "\[a]n integer that specifies the marked-content identifier".
    ///
    /// Nothing in it refers to anything, so there is nothing to carry.
    Item(Object),
    /// Table 357's marked-content reference or Table 358's object reference, held **as the
    /// source wrote it**, beside the output number of the page it was placed on.
    ///
    /// **Uncarried on purpose, and the ordering is the whole point.** [`Carry::plan`]'s own
    /// promise is that "\[e]very kept element is given its slot here, before any page is built,
    /// so that a reference to one from anywhere in the closure maps to the rebuilt element
    /// rather than dragging the source's whole subtree in behind it" — and carrying a content
    /// item's values inside [`Carry::child`] would break it, because `child` runs while the
    /// hierarchy is still being decided and [`Carry::number`] has not handed out a slot yet. A
    /// document that writes a `/P` back-reference on its object references would then copy the
    /// containing element through the closure walk, and the element's own `replace` a moment
    /// later would be [`AssemblyError::AlreadyPlaced`] — and ISO 32000-2 itself is such a
    /// document, on every one of its 1023 pages. Carried in [`Carry::element`] instead, after
    /// `number`.
    Reference {
        /// The source's dictionary, every value still in the source's numbering.
        dictionary: Dictionary,
        /// The output number of the page it belongs to, which overrides any `/Pg` it states.
        page: ObjectId,
    },
}

/// One structure element that reaches a page the output holds.
struct Element {
    /// The document's position among the opened ones.
    at: usize,
    /// The element in the source.
    source: ObjectId,
    /// Its parent in the source, or `None` for a child of the structure tree root.
    parent: Option<ObjectId>,
    /// Its children, in the source's order, content items included.
    children: Vec<Child>,
    /// The output's page for its `/Pg`, where the source stated one the output holds.
    page: Option<ObjectId>,
}

/// What one child of a structure element turned out to be.
enum Outcome {
    /// A kept child.
    Kept(Child),
    /// A content item on a page the output does not hold.
    DroppedItem,
    /// A marked-content item with no `/Pg` anywhere to place it on.
    Unplaceable,
    /// An element that reaches nothing, or a child of no kind Table 355 names.
    Dropped,
}

/// §14.7's structure, planned over what the output's pages reach.
pub(crate) struct Carry {
    /// The output's `/StructTreeRoot`.
    root: ObjectId,
    /// Every element that reaches a carried page, in the order it was decided.
    elements: Vec<Element>,
    /// The output slot each kept source element was given.
    placed: BTreeMap<(usize, ObjectId), ObjectId>,
    /// The kept top-level elements, in the order their sources contribute.
    tops: Vec<(usize, ObjectId)>,
    /// Each carried page's new `/StructParents`, by the page's number in the output.
    page_keys: BTreeMap<ObjectId, i64>,
    /// Where each page key's parent-tree value comes from: the source, and the key it had.
    page_sources: Vec<(usize, i64, i64)>,
    /// The same for §14.7.5.4's objects, filled as the closure walk reaches them.
    object_keys: Vec<(usize, i64, i64)>,
    /// The next free key, which §14.7.5.4 also makes `/ParentTreeNextKey`'s value.
    next_key: i64,
    /// Table 354's `/RoleMap`, merged.
    role_map: BTreeMap<Vec<u8>, Object>,
    /// Table 354's `/ClassMap`, merged.
    class_map: BTreeMap<Vec<u8>, Object>,
    /// Per source, the class names §14.7.6.2 made this carry rename: the old bytes to the new.
    class_renames: BTreeMap<usize, BTreeMap<Vec<u8>, Vec<u8>>>,
    /// Table 355's `/ID` for each kept element, for Table 354's `/IDTree`.
    ids: BTreeMap<Vec<u8>, (usize, ObjectId)>,
    /// Table 354's list-valued entries, concatenated across the sources that state them.
    lists: BTreeMap<&'static str, Vec<Object>>,
    /// Table 353's `/Marked`, true only where every contributing document claimed it.
    marked: bool,
    /// Table 353's `/UserProperties`, true where any contributing document claimed it.
    user_properties: bool,
    /// Table 353's `/Suspects`, deprecated in PDF 2.0 and carried for the same reason.
    suspects: bool,
    /// The plan's source index to name in a warning.
    speaks_for: usize,
    /// Elements that reach no carried page, so are not written.
    dropped: u64,
    /// §14.7.5.4 array members the source filled with an element and the output writes as
    /// §7.3.9's null, because the element is not in the hierarchy the output carries.
    orphaned_items: u64,
    /// Elements with a marked-content child and no `/Pg` to place it on.
    unplaceable: u64,
    /// Content items dropped because the page they name is not in the output.
    dropped_items: u64,
    /// Pages the output holds a second time, whose `/StructParents` is therefore not written.
    duplicated_pages: u64,
    /// The one `Document` Annex L puts the sources' PDF 2.0 `Document`s under, where it is needed.
    wrapper: Option<Wrapper>,
}

/// The `Document` the module comment's Annex L section writes under the structure tree root.
struct Wrapper {
    /// Its slot in the output.
    id: ObjectId,
    /// Table 355's `/NS`: an indirect reference to the PDF 2.0 namespace dictionary.
    namespace: Object,
    /// The top-level elements it holds, by source and source object, in the order they came.
    children: Vec<(usize, ObjectId)>,
}

impl Carry {
    /// Reads every contributing document's structure tree and plans what the output carries.
    ///
    /// `Ok(None)` where no contributing document states a `/StructTreeRoot`, which is the common
    /// case and costs nothing at all.
    ///
    /// Every kept element is given its slot here, before any page is built, so that a reference
    /// to one from anywhere in the closure maps to the rebuilt element rather than dragging the
    /// source's whole subtree in behind it.
    ///
    /// # Errors
    ///
    /// [`Refusal::StructureConflict`] where two sources state a `/RoleMap`, `/ClassMap` or
    /// `/IDTree` entry the derived document cannot hold both of, and [`Refusal::Assembly`] where
    /// the numbering is spent.
    pub(crate) fn plan(
        host: &mut dyn Host,
        contributing: &[usize],
        pages: &[CarriedPage],
        warnings: &mut Vec<Warning>,
        source_of: &dyn Fn(usize) -> usize,
    ) -> Result<Option<Self>, Refusal> {
        let mut roots: Vec<(usize, Dictionary)> = Vec::new();
        for at in contributing {
            if let Some(root) = structure_root(host.source(*at)) {
                roots.push((*at, root));
            }
        }
        if roots.is_empty() {
            return Ok(None);
        }

        let root = host
            .reserve_slot()
            .map_err(|why| Refusal::Assembly(why.to_string()))?;
        let mut carry = Self {
            root,
            elements: Vec::new(),
            placed: BTreeMap::new(),
            tops: Vec::new(),
            page_keys: BTreeMap::new(),
            page_sources: Vec::new(),
            object_keys: Vec::new(),
            next_key: 0,
            role_map: BTreeMap::new(),
            class_map: BTreeMap::new(),
            class_renames: BTreeMap::new(),
            ids: BTreeMap::new(),
            lists: BTreeMap::new(),
            marked: true,
            user_properties: false,
            suspects: false,
            speaks_for: contributing.first().map_or(0, |at| source_of(*at)),
            dropped: 0,
            orphaned_items: 0,
            unplaceable: 0,
            dropped_items: 0,
            duplicated_pages: 0,
            wrapper: None,
        };

        // §14.7.5.4's page keys are assigned in output order, so that a reader of the derived
        // file meets them the way it meets the pages. A page the output holds twice keeps one
        // key: Table 355's `/Pg` is one page object, so a marked-content sequence's parent
        // element can name only one of the two placements, and a second key naming the same
        // elements would tell a reader the content is on both.
        let tagged: BTreeSet<usize> = roots.iter().map(|(at, _)| *at).collect();
        let mut carried: BTreeMap<usize, BTreeMap<ObjectId, ObjectId>> = BTreeMap::new();
        for page in pages {
            if !tagged.contains(&page.at) {
                continue;
            }
            if page.duplicate {
                carry.duplicated_pages = carry.duplicated_pages.saturating_add(1);
                continue;
            }
            carried
                .entry(page.at)
                .or_default()
                .insert(page.source, page.placed);
            let old = host.source(page.at).and_then(|document| {
                document
                    .get_key_of(page.source, "StructParents")
                    .as_ref()
                    .and_then(Object::as_integer)
            });
            if let Some(old) = old {
                let key = carry.next_key;
                carry.next_key = carry.next_key.saturating_add(1);
                carry.page_keys.insert(page.placed, key);
                carry.page_sources.push((page.at, old, key));
            }
        }

        for (at, root_dict) in &roots {
            carry.read_root(host, *at, root_dict, warnings, source_of);
            let pages_here = carried.get(at).cloned().unwrap_or_default();
            carry.decide(host, *at, root_dict, &pages_here)?;
        }
        carry.number(host)?;
        carry.wrap_documents(host)?;
        carry.check_ids(host, source_of)?;
        carry.read_mark_info(host, contributing);
        Ok(Some(carry))
    }
}

/// A dictionary's `/K` children, **unresolved**.
///
/// Table 354 and Table 355 both make `/K` "either a dictionary … or an array of such
/// dictionaries", and either the entry or its items may be indirect. The items come back as the
/// file states them, because §14.7.2 makes a child that is a structure element an *object* — its
/// `/P` "shall be an indirect reference" and its parent tree entry is one too — so resolving a
/// child before asking which element it is throws away the only thing that identifies it.
fn children_of(document: &Document, dict: &Dictionary) -> Vec<Object> {
    let Some(value) = dict.get("K") else {
        return Vec::new();
    };
    match value {
        Object::Array(items) => items.clone(),
        Object::Reference(_) => match document.resolve(value) {
            Object::Array(items) => items,
            _ => vec![value.clone()],
        },
        other => vec![other.clone()],
    }
}

/// A document's `/StructTreeRoot`, where its catalog states one that is a dictionary.
fn structure_root(document: Option<&Document>) -> Option<Dictionary> {
    let document = document?;
    let catalog = document.catalog().ok()?;
    match catalog
        .get("StructTreeRoot")
        .map(|value| document.resolve(value))
    {
        Some(Object::Dictionary(root)) => Some(root),
        _ => None,
    }
}

/// Whether this document states a structure tree at all.
///
/// Asked before [`Carry::plan`] runs, because the closure walk has to know — from the first
/// object it reaches — that an annotation's or an `XObject`'s `/StructParent` is a key it will
/// renumber rather than a value that crosses.
pub(crate) fn states_a_tree(document: Option<&Document>) -> bool {
    structure_root(document).is_some()
}

/// Table 359's key for an object that is a content item in its own right, where it states one.
///
/// §14.7.5.4's Table 359 gives the entry in one sentence:
///
/// > The integer key of this object's entry in the structural parent tree.
pub(crate) fn struct_parent(value: &Object) -> Option<i64> {
    let dict = match value {
        Object::Dictionary(dict) => dict,
        Object::Stream(stream) => &stream.dict,
        _ => return None,
    };
    dict.get("StructParent").and_then(Object::as_integer)
}
