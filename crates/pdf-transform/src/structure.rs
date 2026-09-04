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
//!   two cannot be written; but its NOTE 1 says what the value *is*: "[t]he equivalence need not
//!   be exact; the role map merely indicates an approximate analogy between types, allowing PDF
//!   processors to share nonstandard structure elements in a reasonable way." An approximation is
//!   not an assertion two documents can contradict each other about, and a type left mapped to
//!   the first source's nearest equivalent is still shared in a reasonable way — where refusing
//!   would leave the reader with no structure at all.
//! - **`/ClassMap` — a colliding class is renamed, and every carried element's `/C` is
//!   rewritten.** An attribute class is not an approximation: §14.7.6.2 makes the attributes
//!   "attached to the given structure element", so an element given another source's `/Pa5` would
//!   be laid out by properties it never had. The rename is safe because the clause closes the set
//!   of things that name a class — "[s]tructure elements shall refer to the class by name", and
//!   "[t]he C entry in a structure element dictionary … shall contain a class name or an array of
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
//! §14.7.4's namespaces are the construction that would resolve the first two without a choice:
//! Table 356's `/RoleMapNS` maps one namespace's types to another's, so each source could keep
//! its own mapping under its own namespace. It is **not** taken, because a namespace name "should
//! take the form of a uniform resource identifier" and this program has no basis for inventing
//! one per source — the same reason ADR 0821 section 4 gives for not synthesising an outline item's
//! `/Title`.

use std::collections::{BTreeMap, BTreeSet};

use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};
use pdf_syntax::serialize::AssemblyError;
use pdf_syntax::{Document, tree};

use crate::{Refusal, Warning};

/// How deep the structure hierarchy is walked.
///
/// §14.7.2 puts no bound on the hierarchy's height, so this one is the parser's `max_depth`: a
/// `/K` chain deeper than that is a file whose own objects the reader would have refused to
/// nest. What stops the walk in practice is the visited set — a `/K` cycle is what a hostile
/// file writes, and an element reached twice is not descended twice.
const MAX_DEPTH: usize = 256;

/// Table 354's entries that are arrays and concatenate across sources.
///
/// `/Namespaces` is "[a]n array of namespaces used within the document", `/PronunciationLexicon`
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
    /// §14.7.5.2's second form: "[a]n integer that specifies the marked-content identifier".
    ///
    /// Nothing in it refers to anything, so there is nothing to carry.
    Item(Object),
    /// Table 357's marked-content reference or Table 358's object reference, held **as the
    /// source wrote it**, beside the output number of the page it was placed on.
    ///
    /// **Uncarried on purpose, and the ordering is the whole point.** [`Carry::plan`]'s own
    /// promise is that "[e]very kept element is given its slot here, before any page is built,
    /// so that a reference to one from anywhere in the closure maps to the rebuilt element
    /// rather than dragging the source's whole subtree in behind it" — and carrying a content
    /// item's values inside [`Carry::keep_child`] broke it, because `keep_child` runs while the
    /// hierarchy is still being decided and [`Carry::number`] has not handed out a slot yet. A
    /// document that writes a `/P` back-reference on its object references then copied the
    /// containing element through the closure walk, and the element's own `replace` a moment
    /// later was [`AssemblyError::AlreadyPlaced`] — which the whole of ISO 32000-2 itself does,
    /// on every one of its 1023 pages. Carried in [`Carry::element`] instead, after `number`.
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
        carry.check_ids(host, source_of)?;
        carry.read_mark_info(host, contributing);
        Ok(Some(carry))
    }

    /// Table 354's map- and list-valued entries for one source, reconciled into the merged root.
    ///
    /// Nothing here refuses: the two maps' collisions are answered by the module comment's first
    /// two rules, and only `/IDTree`'s is a refusal — which [`Carry::check_ids`] raises once the
    /// whole hierarchy is decided, so that it names both sources rather than whichever was second.
    fn read_root(
        &mut self,
        host: &mut dyn Host,
        at: usize,
        root: &Dictionary,
        warnings: &mut Vec<Warning>,
        source_of: &dyn Fn(usize) -> usize,
    ) {
        for entry in ["RoleMap", "ClassMap"] {
            let pairs = {
                let Some(document) = host.source(at) else {
                    return;
                };
                let Some(Object::Dictionary(dict)) = root.get(entry).map(|v| document.resolve(v))
                else {
                    continue;
                };
                dict.iter()
                    .map(|(key, value)| (key.as_bytes().to_vec(), document.resolve(value)))
                    .collect::<Vec<(Vec<u8>, Object)>>()
            };
            for (key, value) in pairs {
                if entry == "RoleMap" {
                    // §14.7.3's NOTE 1: "[t]he equivalence need not be exact; the role map merely
                    // indicates an approximate analogy between types". So a name two sources map
                    // differently keeps the first source's approximation, said out loud.
                    if let Some(held) = self.role_map.get(&key) {
                        if *held != value {
                            warnings.push(Warning {
                                source: source_of(at),
                                page: None,
                                detail: format!(
                                    "§14.7.3: /RoleMap already maps /{} and an earlier source's \
                                     mapping wins; the clause's NOTE 1 makes a role map \"an \
                                     approximate analogy between types\", so this source's \
                                     elements of that type keep the earlier equivalent",
                                    String::from_utf8_lossy(&key).escape_debug()
                                ),
                            });
                        }
                        continue;
                    }
                    let carried = host.carry_value(at, &value);
                    self.role_map.insert(key, carried);
                    continue;
                }
                // §14.7.6.2 attaches a class's attributes to the element that names it, so a
                // class two sources define differently is renamed rather than merged, and every
                // carried element's `/C` follows the rename.
                if self.class_map.get(&key).is_some_and(|held| *held == value) {
                    continue;
                }
                let carried = host.carry_value(at, &value);
                if self.class_map.contains_key(&key) {
                    let taken: BTreeSet<Vec<u8>> = self.class_map.keys().cloned().collect();
                    let renamed = crate::merge::free_key(&key, &taken);
                    warnings.push(Warning {
                        source: source_of(at),
                        page: None,
                        detail: format!(
                            "§14.7.6.2: the attribute class /{} is already this document's with \
                             other attributes, so this source's became /{}; every carried \
                             element's /C was rewritten to match",
                            String::from_utf8_lossy(&key).escape_debug(),
                            String::from_utf8_lossy(&renamed).escape_debug()
                        ),
                    });
                    self.class_renames
                        .entry(at)
                        .or_default()
                        .insert(key, renamed.clone());
                    self.class_map.insert(renamed, carried);
                } else {
                    self.class_map.insert(key, carried);
                }
            }
        }

        for entry in LIST_VALUED {
            let items = {
                let Some(document) = host.source(at) else {
                    return;
                };
                let Some(Object::Array(items)) = root.get(entry).map(|v| document.resolve(v))
                else {
                    continue;
                };
                items
            };
            let carried: Vec<Object> = items
                .iter()
                .map(|item| host.carry_value(at, item))
                .collect();
            self.lists.entry(entry).or_default().extend(carried);
        }

        // §14.7.5.4 makes `/ParentTreeNextKey` "an integer value greater than any that is
        // currently in use as a key in the structural parent tree" — a statement about the file
        // that holds the tree, so the output states its own rather than any source's.
        if root.get("ParentTreeNextKey").is_some() {
            warnings.push(Warning {
                source: source_of(at),
                page: None,
                detail: "§14.7.5.4: this source's /ParentTreeNextKey is not carried; the entry \
                         names a key in the parent tree of the file that states it, and the \
                         output states its own"
                    .to_owned(),
            });
        }
    }

    /// Walks one source's hierarchy from the structure tree root, keeping what reaches a page.
    ///
    /// # Errors
    ///
    /// [`Refusal::Assembly`] where the numbering is spent.
    fn decide(
        &mut self,
        host: &mut dyn Host,
        at: usize,
        root: &Dictionary,
        carried: &BTreeMap<ObjectId, ObjectId>,
    ) -> Result<(), Refusal> {
        let children = {
            let Some(document) = host.source(at) else {
                return Ok(());
            };
            children_of(document, root)
        };
        let mut seen = BTreeSet::new();
        for child in &children {
            let Some(id) = child.as_reference() else {
                // Table 355 makes `/P` "( Required; shall be an indirect reference )", so an
                // element written directly into its parent cannot be named by its own children,
                // and a hierarchy this module rebuilds by reference cannot hold it.
                self.dropped = self.dropped.saturating_add(1);
                continue;
            };
            if self.keep(host, at, id, None, carried, &mut seen, 0)? {
                self.tops.push((at, id));
            }
        }
        Ok(())
    }

    /// One element: kept where it reaches a carried page, dropped otherwise.
    ///
    /// Depth-first and bottom-up, because an element is kept for its descendants as much as for
    /// its own content: §14.7.2 makes `/K` "[t]he children of this structure element", and an
    /// ancestor of a kept child is what holds it in the hierarchy.
    ///
    /// # Errors
    ///
    /// [`Refusal::Assembly`] where the numbering is spent.
    #[expect(
        clippy::too_many_arguments,
        reason = "the walk's state, threaded rather than held in a struct that would outlive \
                  the one element it is about"
    )]
    fn keep(
        &mut self,
        host: &mut dyn Host,
        at: usize,
        id: ObjectId,
        parent: Option<ObjectId>,
        carried: &BTreeMap<ObjectId, ObjectId>,
        seen: &mut BTreeSet<ObjectId>,
        depth: usize,
    ) -> Result<bool, Refusal> {
        if depth >= MAX_DEPTH || !seen.insert(id) {
            return Ok(false);
        }
        let (children, own_page, stated_page, identifier) = {
            let Some(document) = host.source(at) else {
                return Ok(false);
            };
            let Object::Dictionary(element) = document.get(id) else {
                return Ok(false);
            };
            // Table 355's `/Pg`: "( Optional; required if K is an integer object or an array
            // containing integer objects; shall be an indirect reference ) A page object
            // representing a page on which some or all of the content items designated by the K
            // entry shall be rendered."
            let stated = element.get("Pg").and_then(Object::as_reference);
            let own = stated.filter(|page| carried.contains_key(page));
            let children = children_of(document, &element);
            let identifier = match element.get("ID") {
                Some(Object::String(bytes)) => Some(Object::String(bytes.clone())),
                _ => None,
            };
            (children, own, stated, identifier)
        };

        let mut kept: Vec<Child> = Vec::new();
        for child in &children {
            match self.child(
                host,
                at,
                child,
                id,
                own_page,
                stated_page,
                carried,
                seen,
                depth,
            )? {
                Outcome::Kept(child) => kept.push(child),
                Outcome::DroppedItem => {
                    self.dropped_items = self.dropped_items.saturating_add(1);
                }
                Outcome::Unplaceable => {
                    self.unplaceable = self.unplaceable.saturating_add(1);
                }
                Outcome::Dropped => {}
            }
        }

        if kept.is_empty() {
            // The element reaches no page the output holds. It is *blocked* rather than merely
            // skipped, so that a `/Ref` or an attribute naming it cannot drag its subtree in.
            host.block_object(at, id);
            self.dropped = self.dropped.saturating_add(1);
            return Ok(false);
        }
        // Table 355's `/ID`: "The string shall be unique among all elements in the document's
        // structure hierarchy." Recorded here, checked once the whole hierarchy is decided, so
        // that a collision names both sources rather than whichever was walked second.
        if let Some(Object::String(bytes)) = identifier {
            self.ids.entry(bytes.to_vec()).or_insert((at, id));
        }
        self.elements.push(Element {
            at,
            source: id,
            parent,
            children: kept,
            page: own_page.and_then(|page| carried.get(&page).copied()),
        });
        Ok(true)
    }

    /// What one child of an element turned out to be.
    #[expect(
        clippy::too_many_arguments,
        reason = "the walk's state, threaded rather than held in a struct that would outlive \
                  the one element it is about"
    )]
    fn child(
        &mut self,
        host: &mut dyn Host,
        at: usize,
        child: &Object,
        parent: ObjectId,
        own_page: Option<ObjectId>,
        stated_page: Option<ObjectId>,
        carried: &BTreeMap<ObjectId, ObjectId>,
        seen: &mut BTreeSet<ObjectId>,
        depth: usize,
    ) -> Result<Outcome, Refusal> {
        let resolved = {
            let Some(document) = host.source(at) else {
                return Ok(Outcome::Dropped);
            };
            document.resolve(child)
        };
        match &resolved {
            // §14.7.5.2's second form: "[a]n integer that specifies the marked-content
            // identifier", read against Table 355's `/Pg` on this element.
            Object::Integer(_) => Ok(match (own_page, stated_page) {
                (Some(_), _) => Outcome::Kept(Child::Item(resolved.clone())),
                (None, Some(_)) => Outcome::DroppedItem,
                (None, None) => Outcome::Unplaceable,
            }),
            Object::Dictionary(dict) => {
                let kind = dict
                    .get("Type")
                    .and_then(Object::as_name)
                    .map(|name| name.as_bytes().to_vec());
                // Table 357's marked-content reference and Table 358's object reference are the
                // two content items that are dictionaries. Each may state a `/Pg` of its own,
                // which "overrides any Pg entry in the structure element containing" it. Anything
                // else is Table 355's own default: "[i]f the value of K is a dictionary
                // containing no Type entry, it shall be assumed to be a structure element
                // dictionary."
                if let Some(b"MCR" | b"OBJR") = kind.as_deref() {
                    {
                        let here = dict.get("Pg").and_then(Object::as_reference);
                        let Some(page) = here.or(stated_page) else {
                            return Ok(Outcome::Unplaceable);
                        };
                        let Some(placed) = carried.get(&page).copied() else {
                            return Ok(Outcome::DroppedItem);
                        };
                        Ok(Outcome::Kept(Child::Reference {
                            dictionary: dict.clone(),
                            page: placed,
                        }))
                    }
                } else {
                    {
                        let Some(id) = child.as_reference() else {
                            self.dropped = self.dropped.saturating_add(1);
                            return Ok(Outcome::Dropped);
                        };
                        if self.keep(
                            host,
                            at,
                            id,
                            Some(parent),
                            carried,
                            seen,
                            depth.saturating_add(1),
                        )? {
                            Ok(Outcome::Kept(Child::Element(id)))
                        } else {
                            Ok(Outcome::Dropped)
                        }
                    }
                }
            }
            _ => Ok(Outcome::Dropped),
        }
    }

    /// Gives every kept element the slot that stands in for it.
    ///
    /// # Errors
    ///
    /// [`Refusal::Assembly`] where the numbering is spent.
    fn number(&mut self, host: &mut dyn Host) -> Result<(), Refusal> {
        for element in &self.elements {
            let placed = host
                .replace_object(element.at, element.source)
                .map_err(|why| {
                    Refusal::Assembly(format!(
                        "§14.7's structure element {}: {why}",
                        element.source.number
                    ))
                })?;
            self.placed.insert((element.at, element.source), placed);
        }
        Ok(())
    }

    /// Table 354's `/IDTree`, and the collision Table 355 forbids.
    ///
    /// # Errors
    ///
    /// [`Refusal::StructureConflict`] where two kept elements state one `/ID`.
    fn check_ids(
        &mut self,
        host: &mut dyn Host,
        source_of: &dyn Fn(usize) -> usize,
    ) -> Result<(), Refusal> {
        let mut collisions: Vec<String> = Vec::new();
        // One `/ID` may be stated by two elements of one source — the clause binds the document
        // that already held both, and carrying what the producer wrote is RFC 0002 section 11.1's
        // premise, which is ADR 0821 section 2's distinction. What cannot be written is one `/ID`
        // stated by two *different* sources, because the derived hierarchy holds both elements.
        for element in &self.elements {
            let Some(document) = host.source(element.at) else {
                continue;
            };
            let Some(Object::String(bytes)) = document.get_key_of(element.source, "ID") else {
                continue;
            };
            let Some((first, _)) = self.ids.get(bytes.as_ref()) else {
                continue;
            };
            if *first != element.at {
                collisions.push(format!(
                    "/ID {} (sources {} and {})",
                    String::from_utf8_lossy(&bytes).escape_debug(),
                    source_of(*first),
                    source_of(element.at)
                ));
            }
        }
        collisions.sort();
        collisions.dedup();
        if collisions.is_empty() {
            return Ok(());
        }
        Err(Refusal::StructureConflict {
            clause: "Table 355 makes a structure element's /ID \"unique among all elements in \
                     the document's structure hierarchy\", and the derived document is one \
                     hierarchy",
            keys: collisions.join("; "),
        })
    }

    /// Table 353's flags, read off every contributing document.
    ///
    /// §14.7.1's mark information dictionary, whose `/Marked` row reads:
    ///
    /// > A flag indicating whether the document conforms to tagged PDF conventions
    ///
    /// A document some of whose pages came out of a source that stated no structure does not
    /// conform to them, whatever its other sources said, so `/Marked` is a conjunction. The other
    /// two flags state that something is *present*, so they are unions.
    fn read_mark_info(&mut self, host: &mut dyn Host, contributing: &[usize]) {
        for at in contributing {
            let flags = host.source(*at).and_then(|document| {
                let catalog = document.catalog().ok()?;
                match catalog.get("MarkInfo").map(|v| document.resolve(v)) {
                    Some(Object::Dictionary(dict)) => Some(dict),
                    _ => None,
                }
            });
            let flag = |key: &str| {
                matches!(
                    flags.as_ref().and_then(|dict| dict.get(key)),
                    Some(&Object::Boolean(true))
                )
            };
            if !flag("Marked") {
                self.marked = false;
            }
            self.user_properties |= flag("UserProperties");
            self.suspects |= flag("Suspects");
        }
    }

    /// The `/StructParents` this output page states, where the carry gave it one.
    pub(crate) fn page_key(&self, placed: ObjectId) -> Option<i64> {
        self.page_keys.get(&placed).copied()
    }

    /// A fresh §14.7.5.4 key for an object that is a content item in its own right.
    ///
    /// Called as the closure walk reaches an annotation or an `XObject` stating `/StructParent`,
    /// so the key it is given is a function of the order the output's objects are numbered in —
    /// deterministic, which is RFC 0002 section 9's first layer.
    ///
    /// `None` where the source's parent tree has no entry for the object's key, or where the
    /// entry names an element this output does not hold: the caller then writes **no**
    /// `/StructParent` at all. A key into a tree the output does state, naming nothing, is the
    /// outcome ADR 0831 named as worse than an absent one — it would tell an assistive processor
    /// that the content has a parent element and then hand it nothing.
    pub(crate) fn object_key(&mut self, document: &Document, at: usize, old: i64) -> Option<i64> {
        let entry = Self::source_entry(document, old)?;
        let id = entry.as_reference()?;
        if !self.placed.contains_key(&(at, id)) {
            return None;
        }
        let key = self.next_key;
        self.next_key = self.next_key.saturating_add(1);
        self.object_keys.push((at, old, key));
        Some(key)
    }

    /// The output's `/StructTreeRoot`, for the catalog.
    pub(crate) fn root(&self) -> ObjectId {
        self.root
    }

    /// Table 353's mark information dictionary, where any flag is worth stating.
    ///
    /// Table 353 gives all three a default of `false`, so a dictionary of three false flags says
    /// exactly what saying nothing says; the entry is written only where one of them is true.
    pub(crate) fn mark_info(&self) -> Option<Object> {
        if !self.marked && !self.user_properties && !self.suspects {
            return None;
        }
        let mut dict = Dictionary::new();
        for (key, flag) in [
            ("Marked", self.marked),
            ("UserProperties", self.user_properties),
            ("Suspects", self.suspects),
        ] {
            if flag {
                dict.insert(Name::new(key.as_bytes()), Object::Boolean(true));
            }
        }
        Some(Object::Dictionary(dict))
    }

    /// Builds every kept element, §14.7.5.4's parent tree and the structure tree root.
    ///
    /// Run after the closure walk, because the object keys above are assigned by it.
    pub(crate) fn finish(mut self, host: &mut dyn Host, warnings: &mut Vec<Warning>) {
        let elements = std::mem::take(&mut self.elements);
        for element in &elements {
            let Some(placed) = self.placed.get(&(element.at, element.source)).copied() else {
                continue;
            };
            let built = self.element(host, element);
            host.place_object(placed, built);
        }

        let parent_tree = self.parent_tree(&*host);
        let mut root = Dictionary::new();
        root.insert(
            Name::new(&b"Type"[..]),
            Object::Name(Name::new(&b"StructTreeRoot"[..])),
        );
        let tops: Vec<Object> = self
            .tops
            .iter()
            .filter_map(|key| self.placed.get(key).copied())
            .map(Object::Reference)
            .collect();
        if !tops.is_empty() {
            root.insert(Name::new(&b"K"[..]), Object::Array(tops));
        }
        if let Some(tree) = parent_tree {
            root.insert(Name::new(&b"ParentTree"[..]), tree);
            // §14.7.5.4: "The ParentTreeNextKey entry in the structure tree root shall hold an
            // integer value greater than any that is currently in use as a key in the structural
            // parent tree."
            root.insert(
                Name::new(&b"ParentTreeNextKey"[..]),
                Object::Integer(self.next_key),
            );
        }
        for (entry, held) in [("RoleMap", &self.role_map), ("ClassMap", &self.class_map)] {
            if held.is_empty() {
                continue;
            }
            let mut dict = Dictionary::new();
            for (key, value) in held {
                dict.insert(Name::new(key.as_slice()), value.clone());
            }
            root.insert(Name::new(entry.as_bytes()), Object::Dictionary(dict));
        }
        let ids: Vec<Object> = self
            .ids
            .iter()
            .filter_map(|(bytes, key)| {
                self.placed
                    .get(key)
                    .map(|placed| (bytes.clone(), Object::Reference(*placed)))
            })
            .flat_map(|(bytes, value)| [Object::String(bytes.into()), value])
            .collect();
        if !ids.is_empty() {
            // §7.9.6 permits the whole tree in one node — "[i]f the root node has a Names entry,
            // it shall be the only node in the tree" — and the keys go in sorted, which is what
            // the clause requires of each node's `/Names` array.
            let mut node = Dictionary::new();
            node.insert(Name::new(&b"Names"[..]), Object::Array(ids));
            root.insert(Name::new(&b"IDTree"[..]), Object::Dictionary(node));
        }
        for (entry, items) in &self.lists {
            if items.is_empty() {
                continue;
            }
            root.insert(Name::new(entry.as_bytes()), Object::Array(items.clone()));
        }
        host.place_object(self.root, Object::Dictionary(root));
        self.report(warnings);
    }

    /// One rebuilt structure element: the source's entries, with the pruned hierarchy's own
    /// `/K`, `/P` and `/Pg`.
    fn element(&self, host: &mut dyn Host, element: &Element) -> Object {
        let source = {
            let Some(document) = host.source(element.at) else {
                return Object::Null;
            };
            match document.get(element.source) {
                Object::Dictionary(dict) => dict,
                _ => return Object::Null,
            }
        };
        let mut out = Dictionary::new();
        for (key, value) in source.iter() {
            match key.as_bytes() {
                // Rebuilt from the pruned hierarchy rather than carried.
                b"K" | b"P" | b"Pg" => {}
                // Table 355's `/Ref` is "zero, one or more indirect references to structure
                // elements", so an entry naming one the output does not hold is dropped rather
                // than written as §7.3.10's null: the array is a list of elements, and a null is
                // not one.
                // §14.7.6.2: "[t]he C entry in a structure element dictionary … shall contain
                // a class name or an array of class names (typically accompanied by revision
                // numbers as well)". The revision numbers are integers between the names and are
                // carried untouched; only a renamed class's name changes.
                b"C" => {
                    let renames = self.class_renames.get(&element.at);
                    let rename = |value: &Object| match (value.as_name(), renames) {
                        (Some(name), Some(renames)) => renames
                            .get(name.as_bytes())
                            .map_or_else(|| value.clone(), |to| Object::Name(Name::new(&to[..]))),
                        _ => value.clone(),
                    };
                    let mapped = match value {
                        Object::Array(items) => Object::Array(items.iter().map(rename).collect()),
                        other => rename(other),
                    };
                    out.insert(key.clone(), mapped);
                }
                b"Ref" => {
                    let items = host
                        .source(element.at)
                        .map(|document| document.resolve(value));
                    let kept: Vec<Object> = match items {
                        Some(Object::Array(items)) => items
                            .iter()
                            .filter_map(Object::as_reference)
                            .filter_map(|id| self.placed.get(&(element.at, id)).copied())
                            .map(Object::Reference)
                            .collect(),
                        _ => Vec::new(),
                    };
                    if !kept.is_empty() {
                        out.insert(key.clone(), Object::Array(kept));
                    }
                }
                _ => {
                    let carried = host.carry_value(element.at, value);
                    out.insert(key.clone(), carried);
                }
            }
        }
        let parent = element
            .parent
            .and_then(|id| self.placed.get(&(element.at, id)).copied())
            .unwrap_or(self.root);
        // Table 355: "( Required; shall be an indirect reference ) The structure element or the
        // structure tree root that is the immediate parent of this structure element."
        out.insert(Name::new(&b"P"[..]), Object::Reference(parent));
        if let Some(page) = element.page {
            out.insert(Name::new(&b"Pg"[..]), Object::Reference(page));
        }
        let mut children: Vec<Object> = Vec::with_capacity(element.children.len());
        for child in &element.children {
            match child {
                Child::Element(id) => {
                    if let Some(placed) = self.placed.get(&(element.at, *id)).copied() {
                        children.push(Object::Reference(placed));
                    }
                }
                Child::Item(object) => children.push(object.clone()),
                Child::Reference { dictionary, page } => {
                    let mut item = Dictionary::new();
                    for (key, value) in dictionary.iter() {
                        // Written explicitly below, so that a pruned hierarchy cannot leave the
                        // reference depending on an entry a dropped ancestor used to state.
                        if key.as_bytes() == b"Pg" {
                            continue;
                        }
                        let carried = host.carry_value(element.at, value);
                        item.insert(key.clone(), carried);
                    }
                    item.insert(Name::new(&b"Pg"[..]), Object::Reference(*page));
                    children.push(Object::Dictionary(item));
                }
            }
        }
        out.insert(Name::new(&b"K"[..]), Object::Array(children));
        Object::Dictionary(out)
    }

    /// §14.7.5.4's parent tree, as one number-tree node.
    ///
    /// > The tree shall contain an entry for each object that is a content item of at least one
    /// > structure element and for each content stream containing at least one marked-content
    /// > sequence that is a content item.
    ///
    /// A page's value is the source's array at its own length and in its own order, because the
    /// array is indexed by the marked-content identifiers in a content stream this suite carries
    /// byte for byte. An object's value is one reference, which is the clause's other bullet.
    ///
    /// The distinction the code below turns on: a page's *array* is resolved and its *members*
    /// are not. Resolving a member would throw away the identity this module maps; leaving the
    /// array unresolved wrote a one-long `[null]` for every source that states it out of line,
    /// which is most of them (ADR 0838).
    fn parent_tree(&mut self, host: &dyn Host) -> Option<Object> {
        let mut nums: Vec<(i64, Object)> = Vec::new();
        let mut orphaned = 0_u64;
        for (at, old, new) in &self.page_sources {
            let Some(value) = host
                .source(*at)
                .and_then(|document| Self::source_entry(document, *old))
            else {
                continue;
            };
            // The array itself may be an indirect object — §7.3.10 makes a reference equivalent
            // to what it names, and most producers write this one out of line — so the *array*
            // is resolved while its members are not. `resolve` follows one object rather than
            // the tree under it, so every member arrives as the reference this module maps.
            let array = host
                .source(*at)
                .map_or(Object::Null, |document| document.resolve(&value));
            let mapped = match array {
                Object::Array(items) => Object::Array(
                    items
                        .iter()
                        .map(|item| {
                            let mapped = self.element_reference(*at, item);
                            // A member the source filled and this carry could not: the element
                            // it named is not in the hierarchy the output holds, so the slot is
                            // §7.3.9's null and an assistive reader finds nothing at that
                            // marked-content identifier. Counted so that it is said out loud.
                            if item.as_reference().is_some() && mapped == Object::Null {
                                orphaned = orphaned.saturating_add(1);
                            }
                            mapped
                        })
                        .collect(),
                ),
                _ => Object::Array(vec![self.element_reference(*at, &value)]),
            };
            nums.push((*new, mapped));
        }
        for (at, old, new) in &self.object_keys {
            let Some(value) = host
                .source(*at)
                .and_then(|document| Self::source_entry(document, *old))
            else {
                continue;
            };
            nums.push((*new, self.element_reference(*at, &value)));
        }
        if nums.is_empty() {
            return None;
        }
        nums.sort_by_key(|(key, _)| *key);
        let mut array: Vec<Object> = Vec::new();
        for (key, value) in nums {
            array.push(Object::Integer(key));
            array.push(value);
        }
        let mut node = Dictionary::new();
        node.insert(Name::new(&b"Nums"[..]), Object::Array(array));
        self.orphaned_items = self.orphaned_items.saturating_add(orphaned);
        Some(Object::Dictionary(node))
    }

    /// One source's parent-tree value for a key, left as the tree states it.
    ///
    /// Unresolved, because §14.7.5.4's object-key value *is* a reference — "the value shall be an
    /// indirect reference to the parent structure element" — and resolving one throws away the
    /// identity this module maps. A page key's value is an array, and [`Self::parent_tree`]
    /// resolves that one level itself, for the reason stated there.
    fn source_entry(document: &Document, key: i64) -> Option<Object> {
        let root = structure_root(Some(document))?;
        let Object::Dictionary(tree) = root
            .get("ParentTree")
            .map(|value| document.resolve(value))?
        else {
            return None;
        };
        tree::lookup_unresolved(&tree, &tree::TreeKey::Number(key), &|value| {
            document.resolve(value)
        })
    }

    /// One parent-tree entry's element: the kept one it names, or §7.3.10's null.
    fn element_reference(&self, at: usize, value: &Object) -> Object {
        value
            .as_reference()
            .and_then(|id| self.placed.get(&(at, id)).copied())
            .map_or(Object::Null, Object::Reference)
    }

    /// What the carry wrote and what it lost, by reason, so that a tagged document's reader is
    /// told rather than left to find out.
    fn report(&self, warnings: &mut Vec<Warning>) {
        let mut said = |detail: String| {
            warnings.push(Warning {
                source: self.speaks_for,
                page: None,
                detail,
            });
        };
        said(format!(
            "§14.7: the structure tree is carried — {} element(s) written, {} dropped for \
             reaching no page the output holds",
            self.placed.len(),
            self.dropped
        ));
        if self.dropped_items > 0 {
            said(format!(
                "§14.7.5: {} content item(s) name a page the output does not hold and were \
                 dropped from their element's /K",
                self.dropped_items
            ));
        }
        if self.orphaned_items > 0 {
            // §14.7.2 makes the structure hierarchy what `/StructTreeRoot`'s `/K` reaches and
            // Table 355 makes `/P` required, so an element a source's parent tree names but its
            // own hierarchy does not reach is in the file and not in the tree. This carry keeps
            // the hierarchy, so the array position goes to null — which is a marked-content
            // sequence an assistive reader will now find nothing for, and two corpus documents
            // do it (ADR 0839).
            said(format!(
                "§14.7.5.4: {} parent-tree entr{} name a structure element the source's own \
                 hierarchy does not reach (§14.7.2, Table 355's required /P), so the output \
                 states §7.3.9's null there and that marked content has no structure",
                self.orphaned_items,
                if self.orphaned_items == 1 { "y" } else { "ies" }
            ));
        }
        if self.unplaceable > 0 {
            said(format!(
                "§14.7.5.2: {} marked-content item(s) could not be placed on any page — Table \
                 355 makes /Pg required where /K is an integer, and the element states none — \
                 and were dropped",
                self.unplaceable
            ));
        }
        if self.duplicated_pages > 0 {
            said(format!(
                "§14.7.5.4: {} page(s) are in the output twice and the second placement states \
                 no /StructParents — Table 355 gives a structure element one /Pg, so its content \
                 items can name only one of the two page objects",
                self.duplicated_pages
            ));
        }
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
