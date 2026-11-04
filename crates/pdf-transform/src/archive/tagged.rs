//! The §14.7 entries a page appended to a *tagged* document owes its structure tree.
//!
//! # What this is for
//!
//! [`super::preserve`] composes a page out of content the document already holds, under the
//! permission `doc/adr/1014` records. A document that describes its content in a structure tree
//! owes that page a description too. §14.8.2.2.1 divides a document's content into real content
//! and artifacts, and its last sentence closes the division rather than leaving a gap: content no
//! structure tree reaches **is** an artifact, whether or not anything marked it as one. So leaving
//! a page out of the tree does not leave it undescribed — it classifies it.
//!
//! The preserved packet is the producer's own metadata, kept as content because the archive may
//! not keep it as metadata. It is not pagination, not a layout ornament and not a production aid,
//! which is what §14.8.2.2.1's other class is — so it is real content, and real content the
//! structure tree does not reach is content this conversion would have silently reclassified.
//! `doc/adr/1163` is the argument; this module is what it builds.
//!
//! # The three things §14.7.5.4 asks for, and where each is written
//!
//! | owed | written |
//! |---|---|
//! | a marked-content sequence per line, each with its own `/MCID` | [`super::preserve`]'s content stream |
//! | `/StructParents` on every appended page | [`Planned::key_for_a_page`], onto the page dictionary |
//! | an entry per page in the `/ParentTree`, and `/ParentTreeNextKey` moved on | [`Described::apply`] |
//!
//! and §14.7.2's hierarchy — a `/StructElem` per line under one grouping element — comes back
//! from [`Planned::finish`] beside those edits, as objects for the walk to add.
//!
//! # The shape of the hierarchy, and why it is that shape
//!
//! One `Part` per preserved object, holding one `P` per line the producer wrote. Table 365 makes
//! `Part` a grouping element that "\[e]ncloses a grouping of structure elements without
//! consideration for their hierarchy", which is what a run of lines is; Table 366 makes `P` "\[a]
//! low-level division of content" that "may enclose any low-level division of content", which is
//! what one of those lines is. A line the measure broke into two is **one** `P` with two
//! marked-content references, because the producer wrote one line: §14.7.5.2's Table 357 gives a
//! reference its own `/Pg`, so the two halves are described as one division of content even where
//! they fall on two pages.
//!
//! The grouping element is a child of the structure tree root rather than of anything the producer
//! wrote. Table 354 admits that in as many words — `/K` is "\[t]he immediate child or children of
//! the structure tree root", "either a dictionary representing a single structure element or an
//! array of such dictionaries" — and it is the smallest edit there is: one entry of one dictionary
//! the producer wrote, and not one of the producer's own elements touched.
//!
//! Annex L's Table L.2 is not the nesting rule these elements answer to, and the reason is
//! §14.8.6.1's: the annex governs "the standard structure namespace for PDF 2.0", an element with
//! no `/NS` entry is in the *default* standard structure namespace, and that clause makes the
//! default namespace the PDF 1.7 one. These elements state no `/NS`, so they are read under
//! ISO 32000-1:2008 clause 14.8 — which is the edition ISO 19005-2 section 6.7.2.1 requires a
//! Level A file to meet anyway.

use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};

use super::decision::Because;
use super::prepare::Spare;
use super::preserve::{NumberTree, TreeRefusals};

/// Why a document that claims tagged conventions with no structure tree gets no appended page.
///
/// §14.8.2.2.1 makes content the structure tree does not reach an artifact, and the preserved
/// packet is real content. Where the catalog claims `/MarkInfo` with `/Marked true` there is a
/// claim to keep true and — with no `/StructTreeRoot` — nowhere to keep it: writing one would be
/// this conversion asserting a logical structure the producer never wrote, which ISO 19005-2
/// section 6.7.1 tells a writer not to do.
pub(super) const CLAIMS_TAGGED_WITH_NO_TREE: &str = "this document's catalog claims MarkInfo with \
     Marked true and states no StructTreeRoot, so there is no structure tree for an appended page \
     to be described in — and ISO 32000-2 \u{a7}14.8.2.2.1 makes content no structure tree reaches an \
     artifact, which the content this remedy preserves is not. Writing a structure tree this \
     document never had would be this conversion asserting a logical structure its producer did \
     not, which ISO 19005-2 clause 6.7.1 advises a writer against";

/// Why a structure tree stated inline in the catalog stops the remedy.
///
/// Table 355 makes a structure element's `/P` "\[r]equired; shall be an indirect reference", and
/// the grouping element this writes has the structure tree root as its parent. A root the catalog
/// states as a direct dictionary is not an object there is a reference to.
pub(super) const TREE_ROOT_NOT_REFERENCED: &str = "this document's catalog states its \
     StructTreeRoot as a direct dictionary rather than an indirect reference, and ISO 32000-2 \
     Table 355 requires a structure element's P entry to be an indirect reference to its parent — \
     so the element describing an appended page has no way to name the root it hangs from";

/// Why a structural parent tree this converter cannot read stops the remedy.
///
/// §14.7.5.4 requires every appended page's marked-content sequences to be findable through the
/// parent tree, and a tree whose shape is not §7.9.7's leaves no place a key could be put that a
/// reader would find. The tree is extended by [`super::preserve::NumberTree`], whatever its depth
/// (`doc/adr/2672`); this and the three after it are the only shapes refused.
pub(super) const PARENT_TREE_NOT_A_TREE: &str = "this document's structure tree states a \
     ParentTree that is not a number tree this converter can read: a node is not a dictionary, a \
     /Kids or /Nums is not an array, a /Nums holds a key without its value or a key that is not an \
     integer, or the root holds both /Kids and /Nums (ISO 32000-2 \u{a7}7.9.6 and \u{a7}7.9.7, \
     Table 37). \u{a7}14.7.5.4 requires an appended page's marked-content sequences to be findable \
     through that tree, so the page is not written rather than written unfindable";

/// Why a structural parent tree that reaches a node twice stops the remedy.
pub(super) const PARENT_TREE_CYCLE: &str = "this document's structure tree states a ParentTree \
     that names one of its own nodes twice through /Kids, so it is not the tree ISO 32000-2 \
     \u{a7}7.9.7 describes and the keys it already uses cannot be known; an appended page's key \
     could collide with one of them, and \u{a7}14.7.5.4 makes the key the only way to its entry";

/// Why a structural parent tree whose keys are out of order stops the remedy.
pub(super) const PARENT_TREE_OUT_OF_ORDER: &str = "this document's structure tree states a \
     ParentTree that does not hold its keys in ascending order, which ISO 32000-2 \u{a7}7.9.7 \
     requires (\"shall be sorted in ascending numerical order\"), so which entry a key finds is \
     not something the tree states and an appended page's entry would not be either";

/// Why a structural parent tree deeper than [`super::preserve::DEEPEST_NUMBER_TREE`] stops the
/// remedy.
pub(super) const PARENT_TREE_TOO_DEEP: &str = "this document's structure tree states a \
     ParentTree nested more than 64 nodes deep through /Kids. A balanced tree over every page and \
     annotation a file can hold is a few levels deep, so this converter reads no further rather \
     than walk a structure built to make a reader walk";

/// The parent tree's refusals, which [`super::preserve::NumberTree`] reads it under.
const PARENT_TREE: TreeRefusals = TreeRefusals {
    not_a_tree: PARENT_TREE_NOT_A_TREE,
    cycle: PARENT_TREE_CYCLE,
    out_of_order: PARENT_TREE_OUT_OF_ORDER,
    too_deep: PARENT_TREE_TOO_DEEP,
};

/// Why a structure tree root of an unreadable shape stops the remedy.
pub(super) const TREE_ROOT_NOT_A_DICTIONARY: &str = "this document's catalog names a \
     StructTreeRoot that does not resolve to a dictionary, so ISO 32000-2 Table 354's entries are \
     not there to extend";

/// Why marks preserved on a page are refused in a tagged document.
///
/// **The one shape this module does not build, and it is a shape rather than a shortfall.** A
/// relocated appearance is the producer's own real content, so §14.8.2.2.1 puts it in the
/// structure tree — but Table 355 makes `/S` required, and what an annotation's appearance *is*
/// semantically is a fact only its producer held. Choosing one would be inventing the structural
/// information ISO 19005-2 section 6.7.1 tells a writer not to invent, and `Annot` is not the
/// answer either: Table 368 makes that type "an association between the content enclosed by the
/// Annot structure element and one or more corresponding PDF annotations", and the annotation is
/// exactly what this remedy has had to remove. An operator who knows what the artwork is could say
/// so, which is the `supply` shape `doc/rfc/0007` section 0.2 already has a word for.
pub(super) const MARKS_IN_A_TAGGED_DOCUMENT: &str = "these marks are an annotation's appearance \
     stream, and putting them on a page of a document that describes its content in a structure \
     tree would need a structure type for them — which ISO 32000-2 Table 355 makes required and \
     which only the producer knew. This conversion will not choose one: ISO 19005-2 clause 6.7.1 \
     advises a writer against adding structural information the source does not state, and the \
     Annot type is an association with an annotation this remedy has had to remove. Authorising \
     the loss with --authorise forbidden-annotation is the answer today";

/// Whether the document says its content is described — a structure tree, or a claim of one.
///
/// §14.8.2.2 divides a document's content into real content and artifacts for a *tagged* file, and
/// Table 353 makes `/Marked` the claim that a file "conforms to the Tagged PDF conventions". A
/// document making neither statement has said nothing an appended page could make false.
#[must_use]
pub(super) fn describes_its_content(document: &Document) -> bool {
    let Ok(catalog) = document.catalog() else {
        return false;
    };
    !document.get_key(&catalog, "StructTreeRoot").is_null()
        || document
            .get_key(&catalog, "MarkInfo")
            .as_dict()
            .is_some_and(|info| document.get_key(info, "Marked") == Object::Boolean(true))
}

/// Where the parent tree's root is written back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Site {
    /// The structure tree root states `/ParentTree` as a direct dictionary, or states none, so
    /// the extended root goes back into the structure tree root's own dictionary.
    InTheRoot,
    /// The parent tree's root is an object of its own, which is what §14.7.5.4's own example
    /// writes.
    InItsOwnObject(ObjectId),
}

/// The structure tree's shape, read before any page is laid out.
///
/// Read first because §14.7.5.4's keys have to be known before a page dictionary can state
/// `/StructParents`, and the page dictionary is built as the text is set. Every refusal this
/// module makes is made here, so that a document it cannot describe costs no layout.
#[derive(Debug)]
pub(super) struct Planned {
    /// The structure tree root, which every top-level element states as its `/P`.
    root: ObjectId,
    /// The parent tree as the producer wrote it, every node read.
    tree: NumberTree,
    /// Where the parent tree's root lives.
    site: Site,
    /// The next key §14.7.5.4 admits: greater than any the parent tree already uses.
    next_key: i64,
    /// Each appended page's key, in the order the pages were composed.
    pages: Vec<(ObjectId, i64)>,
    /// One entry per preserved object: the grouping element's lines, each line's references.
    groups: Vec<Vec<Vec<Reference>>>,
}

/// One marked-content sequence a line of preserved text was set as.
#[derive(Debug, Clone, Copy)]
pub(super) struct Reference {
    /// The page the sequence is on, in the source's numbering.
    pub(super) page: ObjectId,
    /// §14.7.5.2's identifier, unique within that page's content stream.
    pub(super) mcid: i64,
}

impl Planned {
    /// Reads the tree a document states, or the reason its page cannot be described in it.
    ///
    /// `Ok(None)` where the document is not tagged at all, which needs no structure entries and
    /// is the common case: §14.8.2.2 divides content into real content and artifacts only for a
    /// tagged file, and a document with neither a `/StructTreeRoot` nor a `/MarkInfo` claim has
    /// said nothing this page could make false.
    ///
    /// # Errors
    ///
    /// [`Because::NotBuiltYet`] with [`CLAIMS_TAGGED_WITH_NO_TREE`], [`TREE_ROOT_NOT_REFERENCED`],
    /// [`TREE_ROOT_NOT_A_DICTIONARY`], [`PARENT_TREE_NOT_A_TREE`], [`PARENT_TREE_CYCLE`],
    /// [`PARENT_TREE_OUT_OF_ORDER`] or [`PARENT_TREE_TOO_DEEP`], each naming its shape.
    pub(super) fn of(document: &Document) -> Result<Option<Self>, Because> {
        let Ok(catalog) = document.catalog() else {
            return Ok(None);
        };
        let claims_tagged = document
            .get_key(&catalog, "MarkInfo")
            .as_dict()
            .is_some_and(|info| document.get_key(info, "Marked") == Object::Boolean(true));
        let Some(stated) = catalog.get("StructTreeRoot") else {
            if claims_tagged {
                return Err(Because::NotBuiltYet(CLAIMS_TAGGED_WITH_NO_TREE));
            }
            return Ok(None);
        };
        let Some(at) = stated.as_reference() else {
            return Err(Because::NotBuiltYet(TREE_ROOT_NOT_REFERENCED));
        };
        let Some(root) = document.resolve(stated).as_dict().cloned() else {
            return Err(Because::NotBuiltYet(TREE_ROOT_NOT_A_DICTIONARY));
        };
        let (tree, site) = read_the_parent_tree(document, &root)?;
        // §14.7.5.4: "The ParentTreeNextKey entry in the structure tree root shall hold an integer
        // value greater than any that is currently in use as a key in the structural parent tree."
        // The producer's own value is honoured where it says that and corrected where it does not,
        // because a key already in use would put this page's array where a producer's page's is —
        // so no appended page's key can collide with one the tree holds, at any depth.
        let highest = tree.last;
        let stated_next = document.get_key(&root, "ParentTreeNextKey").as_integer();
        let next_key = stated_next
            .unwrap_or(0)
            .max(highest.map_or(0, |key| key.saturating_add(1)))
            .max(0);
        Ok(Some(Self {
            root: at,
            tree,
            site,
            next_key,
            pages: Vec::new(),
            groups: Vec::new(),
        }))
    }

    /// The `/StructParents` an appended page states, taken from the keys not yet in use.
    ///
    /// §14.7.5.4 makes the entry "\[t]he integer key of this object's entry in the structural
    /// parent tree", and every appended page holds marked-content sequences, so every one needs
    /// its own.
    pub(super) fn key_for_a_page(&mut self, page: ObjectId) -> i64 {
        let key = self.next_key;
        self.next_key = self.next_key.saturating_add(1);
        self.pages.push((page, key));
        key
    }

    /// Records one preserved object's lines, each as the sequences it was set as.
    pub(super) fn set(&mut self, lines: Vec<Vec<Reference>>) {
        self.groups.push(lines);
    }

    /// Builds §14.7.2's elements and the entries the tree gains, or why a number ran out.
    ///
    /// The elements come back beside the edits rather than inside them: they are objects the walk
    /// adds, exactly as an appended page and its content stream are, and the edits are two of the
    /// producer's own dictionaries rewritten in place.
    ///
    /// # Errors
    ///
    /// [`Because::NotBuiltYet`] with [`super::preserve::NO_SPARE_OBJECT`] where the document has
    /// no free object number left for an element or for the parent tree's new leaf.
    pub(super) fn finish(
        self,
        document: &Document,
        spare: &mut Spare,
    ) -> Result<(Described, Vec<(ObjectId, Object)>), Because> {
        let mut written: Vec<(ObjectId, Object)> = Vec::new();
        let mut tops: Vec<Object> = Vec::new();
        // The element each line was written as, in the layout's own shape, so that the parent-tree
        // arrays below can be indexed by the same two positions the layout used. A line that set
        // no marks — a blank one the producer wrote — has no element, because there is no content
        // of it for the tree to reach.
        let mut elements: Vec<Vec<Option<ObjectId>>> = Vec::new();
        for lines in &self.groups {
            let group_at = take(document, spare)?;
            let mut kids = Vec::new();
            let mut of_this_group = Vec::new();
            for line in lines {
                if line.is_empty() {
                    of_this_group.push(None);
                    continue;
                }
                let line_at = take(document, spare)?;
                written.push((line_at, paragraph(group_at, line)));
                kids.push(Object::Reference(line_at));
                of_this_group.push(Some(line_at));
            }
            elements.push(of_this_group);
            if kids.is_empty() {
                continue;
            }
            written.push((group_at, grouping(self.root, kids)));
            tops.push(Object::Reference(group_at));
        }
        let mut pairs = Vec::new();
        for (page, key) in &self.pages {
            let mut sequences: Vec<Object> = Vec::new();
            for (group, lines) in self.groups.iter().enumerate() {
                for (line, references) in lines.iter().enumerate() {
                    let element = elements.get(group).and_then(|ids| ids.get(line)).copied();
                    for reference in references.iter().filter(|held| held.page == *page) {
                        put(&mut sequences, reference.mcid, element.flatten());
                    }
                }
            }
            pairs.push((*key, Object::Array(sequences)));
        }
        // The keys were handed out from past the tree's greatest, in order, so they extend it
        // wherever its keys are held (`doc/adr/2672`).
        let tree = self.tree.extended(document, pairs, spare, &mut written)?;
        Ok((
            Described {
                root: self.root,
                tops,
                next_key: self.next_key,
                site: self.site,
                tree,
            },
            written,
        ))
    }
}

/// Puts one sequence's parent at its own index in §14.7.5.4's array, filling the gaps with null.
///
/// > The array element corresponding to each sequence shall be found by using the sequence's
/// > marked -content identifier as a zero-based index into the array.
///
/// The identifiers this module hands out run from zero without a gap, so the fill never happens on
/// a page this program composed; it is here because the clause makes the index, not the order, the
/// thing that decides, and §7.3.9's null is what an array member with no element is.
fn put(sequences: &mut Vec<Object>, mcid: i64, element: Option<ObjectId>) {
    let Ok(index) = usize::try_from(mcid) else {
        return;
    };
    while sequences.len() <= index {
        sequences.push(Object::Null);
    }
    if let Some(element) = element
        && let Some(slot) = sequences.get_mut(index)
    {
        *slot = Object::Reference(element);
    }
}

/// A free object number, or the refusal [`super::preserve`] already has a sentence for.
fn take(document: &Document, spare: &mut Spare) -> Result<ObjectId, Because> {
    spare
        .take(document)
        .ok_or(Because::NotBuiltYet(super::preserve::NO_SPARE_OBJECT))
}

/// Table 365's `Part`: the grouping element one preserved object's lines hang from.
fn grouping(root: ObjectId, kids: Vec<Object>) -> Object {
    let mut dict = Dictionary::new();
    dict.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"StructElem"[..])),
    );
    dict.insert(Name::new(&b"S"[..]), Object::Name(Name::new(&b"Part"[..])));
    dict.insert(Name::new(&b"P"[..]), Object::Reference(root));
    dict.insert(Name::new(&b"K"[..]), Object::Array(kids));
    Object::Dictionary(dict)
}

/// Table 366's `P`: one line of the preserved content, as the producer broke it.
///
/// Its children are Table 357's marked-content references rather than bare identifiers, and that
/// is what lets one line be one element: a line the measure broke across a page boundary has two
/// sequences on two pages, and only the reference form states a page of its own.
fn paragraph(parent: ObjectId, line: &[Reference]) -> Object {
    let mut kids = Vec::new();
    for reference in line {
        let mut child = Dictionary::new();
        child.insert(
            Name::new(&b"Type"[..]),
            Object::Name(Name::new(&b"MCR"[..])),
        );
        child.insert(Name::new(&b"Pg"[..]), Object::Reference(reference.page));
        child.insert(Name::new(&b"MCID"[..]), Object::Integer(reference.mcid));
        kids.push(Object::Dictionary(child));
    }
    let mut dict = Dictionary::new();
    dict.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"StructElem"[..])),
    );
    dict.insert(Name::new(&b"S"[..]), Object::Name(Name::new(&b"P"[..])));
    dict.insert(Name::new(&b"P"[..]), Object::Reference(parent));
    dict.insert(Name::new(&b"K"[..]), Object::Array(kids));
    Object::Dictionary(dict)
}

/// The parent tree, every node read, and where its root is written back — or why it cannot be.
fn read_the_parent_tree(
    document: &Document,
    root: &Dictionary,
) -> Result<(NumberTree, Site), Because> {
    let Some(stated) = root.get("ParentTree") else {
        // §14.7.5.4 makes the entry "Required if any structure element contains content items",
        // and this conversion is about to add some; a tree with no entries of its own is created
        // in the structure tree root, as a root holding `/Nums`.
        return Ok((NumberTree::empty(PARENT_TREE), Site::InTheRoot));
    };
    let Some(tree) = document.resolve(stated).as_dict().cloned() else {
        return Err(Because::NotBuiltYet(PARENT_TREE_NOT_A_TREE));
    };
    let at = stated.as_reference();
    let tree = NumberTree::read(document, tree, at, PARENT_TREE)?;
    Ok((tree, at.map_or(Site::InTheRoot, Site::InItsOwnObject)))
}

/// The entries a tagged document's structure tree gains for the pages this conversion appended.
///
/// Two of the producer's own dictionaries change and nothing else does: the structure tree root's
/// `/K` gains the grouping elements and its `/ParentTreeNextKey` moves on, and the parent tree's
/// root gains one entry per appended page — in its own `/Nums`, or in a new leaf its `/Kids`
/// gains. `doc/adr/1163`, `doc/adr/2672`.
#[derive(Debug)]
pub(super) struct Described {
    /// The structure tree root.
    root: ObjectId,
    /// The grouping elements the root's `/K` gains.
    tops: Vec<Object>,
    /// §14.7.5.4's `/ParentTreeNextKey`, past every key this conversion used.
    next_key: i64,
    /// Where the parent tree's root is written back.
    site: Site,
    /// The parent tree's root as it becomes, holding or reaching every appended page's entry.
    tree: Dictionary,
}

impl Described {
    /// Applies whichever of the two edits this object is the site of.
    ///
    /// Answers whether the dictionary changed, which is what the rewrite counts a change by.
    pub(super) fn apply(&self, document: &Document, id: ObjectId, out: &mut Dictionary) -> bool {
        let mut changed = false;
        if id == self.root {
            // Table 354: `/K` is "[t]he immediate child or children of the structure tree root in
            // the structure hierarchy. The value may be either a dictionary representing a single
            // structure element or an array of such dictionaries." A root stating one dictionary
            // becomes a root stating an array of two; a root stating an array gains a member; a
            // root stating nothing states this conversion's alone.
            let mut kids = match document.get_key(out, "K") {
                Object::Array(items) => items.clone(),
                Object::Null => Vec::new(),
                other => vec![other],
            };
            kids.extend(self.tops.iter().cloned());
            out.insert(Name::new(&b"K"[..]), Object::Array(kids));
            out.insert(
                Name::new(&b"ParentTreeNextKey"[..]),
                Object::Integer(self.next_key),
            );
            if self.site == Site::InTheRoot {
                out.insert(
                    Name::new(&b"ParentTree"[..]),
                    Object::Dictionary(self.tree.clone()),
                );
            }
            changed = true;
        }
        if self.site == Site::InItsOwnObject(id) {
            // Only the entries the extension writes are put back, so whatever else the producer
            // wrote in the node — or another edit of this rewrite — stays as it is.
            for (key, value) in self.tree.iter() {
                if matches!(key.as_bytes(), b"Kids" | b"Nums" | b"Limits") {
                    out.insert(key.clone(), value.clone());
                }
            }
            changed = true;
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A document whose structure tree root, object 2, states `/ParentTree` as `tree`, with
    /// `nodes` as objects 3 on and `/ParentTreeNextKey` as `next`.
    ///
    /// Nothing but the catalog and the root is read by [`Planned::of`], so the fixture holds no
    /// page; the appended pages' identifiers are numbers past every object it writes.
    fn tagged(tree: &str, next: i64, nodes: &[&str]) -> Document {
        use std::fmt::Write as _;
        let catalog = "<< /Type /Catalog /StructTreeRoot 2 0 R >>".to_owned();
        let root = format!(
            "<< /Type /StructTreeRoot /K [] /ParentTree {tree} /ParentTreeNextKey {next} >>"
        );
        let objects: Vec<&str> = [catalog.as_str(), root.as_str()]
            .into_iter()
            .chain(nodes.iter().copied())
            .collect();
        let mut out = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
        }
        let size = objects.len().saturating_add(1);
        let at = out.len();
        let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
        for offset in offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{at}\n%%EOF\n"
        );
        Document::open(out.into_bytes()).expect("the fixture opens")
    }

    /// The structure entries two pages appended to `document` take, one line on each.
    fn described(document: &Document) -> Result<(Described, Vec<(ObjectId, Object)>), Because> {
        let mut planned = Planned::of(document)?.expect("the document is tagged");
        let pages = [ObjectId::new(90, 0), ObjectId::new(91, 0)];
        for page in pages {
            let _ = planned.key_for_a_page(page);
        }
        planned.set(
            pages
                .iter()
                .map(|&page| vec![Reference { page, mcid: 0 }])
                .collect(),
        );
        planned.finish(document, &mut Spare { next: 100 })
    }

    fn integers(object: &Object) -> Vec<i64> {
        object
            .as_array()
            .expect("an array")
            .iter()
            .filter_map(Object::as_integer)
            .collect()
    }

    #[test]
    fn a_parent_tree_with_kids_takes_the_pages_entries_in_one_new_leaf() {
        // Two levels, and a `/ParentTreeNextKey` that understates the greatest key: §14.7.5.4
        // makes it "greater than any that is currently in use", so the pages take 8 and 9.
        let document = tagged(
            "3 0 R",
            2,
            &[
                "<< /Kids [4 0 R] >>",
                "<< /Limits [0 7] /Kids [5 0 R 6 0 R] >>",
                "<< /Limits [0 0] /Nums [0 []] >>",
                "<< /Limits [7 7] /Nums [7 []] >>",
            ],
        );
        let (described, written) = described(&document).expect("it extends");
        assert_eq!(described.next_key, 10);
        assert_eq!(described.site, Site::InItsOwnObject(ObjectId::new(3, 0)));
        let kids = described.tree.get("Kids").expect("kids");
        let Some([Object::Reference(producers), Object::Reference(leaf)]) = kids.as_array() else {
            panic!("the root gains one kid: {kids:?}");
        };
        assert_eq!(*producers, ObjectId::new(4, 0));
        let node = written
            .iter()
            .find(|(id, _)| id == leaf)
            .and_then(|(_, node)| node.as_dict())
            .expect("the leaf is written");
        assert_eq!(integers(node.get("Limits").expect("limits")), [8, 9]);
        assert_eq!(integers(node.get("Nums").expect("nums")), [8, 9]);
        // The rewrite puts back the root's own entries, and only those, into the producer's node.
        let mut out = Dictionary::new();
        out.insert(Name::new(&b"Producer"[..]), Object::Boolean(true));
        assert!(described.apply(&document, ObjectId::new(3, 0), &mut out));
        assert_eq!(out.get("Kids"), Some(kids));
        assert_eq!(out.get("Producer"), Some(&Object::Boolean(true)));
    }

    #[test]
    fn a_root_nums_stated_through_a_reference_is_written_into_the_root() {
        let document = tagged("<< /Nums 3 0 R >>", 1, &["[0 []]"]);
        let (described, written) = described(&document).expect("it extends");
        assert_eq!(described.site, Site::InTheRoot);
        assert_eq!(
            // The values are arrays, so the integers are the keys.
            integers(described.tree.get("Nums").expect("nums")),
            [0, 1, 2]
        );
        assert!(
            written.iter().all(|(_, object)| object
                .as_dict()
                .is_some_and(|node| node.get("Limits").is_none())),
            "no leaf is added to a flat tree"
        );
    }

    #[test]
    fn a_parent_tree_that_cannot_be_read_is_refused_by_name() {
        let refused = |tree: &str, nodes: &[&str]| match described(&tagged(tree, 9, nodes)) {
            Err(Because::NotBuiltYet(reason)) => reason,
            other => panic!("{tree} {nodes:?}: {other:?}"),
        };
        for (tree, nodes) in [
            ("7", &[][..]),
            ("<< /Kids [3 0 R] >>", &["7"][..]),
            ("<< /Nums [0 [] 4] >>", &[][..]),
            (
                "<< /Kids [3 0 R] >>",
                &["<< /Limits [0 0] /Nums [(0) []] >>"][..],
            ),
            (
                "<< /Kids [3 0 R] /Nums [0 []] >>",
                &["<< /Limits [1 1] /Nums [1 []] >>"][..],
            ),
        ] {
            assert_eq!(
                refused(tree, nodes),
                PARENT_TREE_NOT_A_TREE,
                "{tree} {nodes:?}"
            );
        }
        assert_eq!(
            refused(
                "<< /Kids [3 0 R] >>",
                &["<< /Limits [0 0] /Kids [3 0 R] >>"]
            ),
            PARENT_TREE_CYCLE
        );
        assert_eq!(
            refused(
                "<< /Kids [3 0 R 4 0 R] >>",
                &[
                    "<< /Limits [0 5] /Nums [0 [] 5 []] >>",
                    "<< /Limits [3 3] /Nums [3 []] >>"
                ]
            ),
            PARENT_TREE_OUT_OF_ORDER
        );
        assert_eq!(
            refused("<< /Nums [4 [] 2 []] >>", &[]),
            PARENT_TREE_OUT_OF_ORDER
        );
        // A chain one node deeper than the bound, and the same chain one node shallower read.
        let deepest = super::super::preserve::DEEPEST_NUMBER_TREE.saturating_add(1);
        let chain = |length: usize| -> Vec<String> {
            let mut chain: Vec<String> = (0..length)
                .map(|at| format!("<< /Limits [0 0] /Kids [{} 0 R] >>", at.saturating_add(4)))
                .collect();
            chain.push("<< /Limits [0 0] /Nums [0 []] >>".to_owned());
            chain
        };
        let too_deep = chain(deepest);
        let nodes: Vec<&str> = too_deep.iter().map(String::as_str).collect();
        assert_eq!(refused("3 0 R", &nodes), PARENT_TREE_TOO_DEEP);
        let read = chain(deepest.saturating_sub(1));
        let nodes: Vec<&str> = read.iter().map(String::as_str).collect();
        assert!(described(&tagged("3 0 R", 9, &nodes)).is_ok());
    }
}
