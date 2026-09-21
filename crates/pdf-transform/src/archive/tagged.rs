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
//! `Part` a grouping element that "[e]ncloses a grouping of structure elements without
//! consideration for their hierarchy", which is what a run of lines is; Table 366 makes `P` "[a]
//! low-level division of content" that "may enclose any low-level division of content", which is
//! what one of those lines is. A line the measure broke into two is **one** `P` with two
//! marked-content references, because the producer wrote one line: §14.7.5.2's Table 357 gives a
//! reference its own `/Pg`, so the two halves are described as one division of content even where
//! they fall on two pages.
//!
//! The grouping element is a child of the structure tree root rather than of anything the producer
//! wrote. Table 354 admits that in as many words — `/K` is "[t]he immediate child or children of
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
/// Table 355 makes a structure element's `/P` "[r]equired; shall be an indirect reference", and
/// the grouping element this writes has the structure tree root as its parent. A root the catalog
/// states as a direct dictionary is not an object there is a reference to.
pub(super) const TREE_ROOT_NOT_REFERENCED: &str = "this document's catalog states its \
     StructTreeRoot as a direct dictionary rather than an indirect reference, and ISO 32000-2 \
     Table 355 requires a structure element's P entry to be an indirect reference to its parent — \
     so the element describing an appended page has no way to name the root it hangs from";

/// Why a parent tree with intermediate nodes stops the remedy.
///
/// The same shape as [`super::preserve::LABELS_NOT_EXTENDABLE`] and for the same reason: §7.9.7's
/// number tree may spread its keys over `/Kids`, and putting a new key into the right leaf — with
/// every `/Limits` on the way down moved to match — is a rewrite of the producer's own tree rather
/// than an entry added to it.
pub(super) const PARENT_TREE_NOT_EXTENDABLE: &str = "this document states its structure tree's \
     ParentTree as a number tree with /Kids rather than one flat /Nums array, and putting a new \
     key into the right leaf — with every /Limits above it moved to match — is not built. \
     ISO 32000-2 \u{a7}14.7.5.4 requires an appended page's marked-content sequences to be findable \
     through that tree, so the page is not written rather than written unfindable";

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

/// Where the parent tree's `/Nums` array is written back.
#[derive(Debug, Clone)]
enum Nums {
    /// The structure tree root states `/ParentTree` as a direct dictionary, so the array goes
    /// back into the root's own dictionary.
    ///
    /// The dictionary carried here is the whole `/ParentTree` value, `/Nums` already replaced.
    InTheRoot(Dictionary),
    /// The parent tree is an object of its own, which is what §14.7.5.4's own example writes.
    ///
    /// Nothing of its dictionary is carried: the one entry that changes is `/Nums`, and the rest
    /// of what the producer wrote there is left where it is by the rewrite.
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
    /// The parent tree's `/Nums` pairs as the producer wrote them, and where they live.
    tree: Nums,
    /// The pairs the producer wrote, key and value alternating.
    pairs: Vec<Object>,
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
    /// [`TREE_ROOT_NOT_A_DICTIONARY`] or [`PARENT_TREE_NOT_EXTENDABLE`], each naming its shape.
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
        let (tree, pairs) = read_the_parent_tree(document, &root)?;
        // §14.7.5.4: "The ParentTreeNextKey entry in the structure tree root shall hold an integer
        // value greater than any that is currently in use as a key in the structural parent tree."
        // The producer's own value is honoured where it says that and corrected where it does not,
        // because a key already in use would put this page's array where a producer's page's is.
        let highest = pairs
            .chunks_exact(2)
            .filter_map(|pair| pair.first().and_then(Object::as_integer))
            .max();
        let stated_next = document.get_key(&root, "ParentTreeNextKey").as_integer();
        let next_key = stated_next
            .unwrap_or(0)
            .max(highest.map_or(0, |key| key.saturating_add(1)))
            .max(0);
        Ok(Some(Self {
            root: at,
            tree,
            pairs,
            next_key,
            pages: Vec::new(),
            groups: Vec::new(),
        }))
    }

    /// The `/StructParents` an appended page states, taken from the keys not yet in use.
    ///
    /// §14.7.5.4 makes the entry "[t]he integer key of this object's entry in the structural
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
    /// no free object number left for an element.
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
        let mut pairs = self.pairs;
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
            pairs.push(Object::Integer(*key));
            pairs.push(Object::Array(sequences));
        }
        Ok((
            Described {
                root: self.root,
                tops,
                next_key: self.next_key,
                tree: self.tree,
                pairs,
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

/// The parent tree's pairs and where they are written back, or why they cannot be extended.
fn read_the_parent_tree(
    document: &Document,
    root: &Dictionary,
) -> Result<(Nums, Vec<Object>), Because> {
    let Some(stated) = root.get("ParentTree") else {
        // §14.7.5.4 makes the entry "Required if any structure element contains content items",
        // and this conversion is about to add some; a tree with no entries of its own is created
        // as a flat one, which is the shape every entry below goes into.
        return Ok((Nums::InTheRoot(Dictionary::new()), Vec::new()));
    };
    let Some(tree) = document.resolve(stated).as_dict().cloned() else {
        return Err(Because::NotBuiltYet(PARENT_TREE_NOT_EXTENDABLE));
    };
    if tree.get("Kids").is_some() {
        return Err(Because::NotBuiltYet(PARENT_TREE_NOT_EXTENDABLE));
    }
    let pairs = match tree.get("Nums") {
        None => Vec::new(),
        Some(nums) => match document.resolve(nums) {
            Object::Array(items) => items.clone(),
            _ => return Err(Because::NotBuiltYet(PARENT_TREE_NOT_EXTENDABLE)),
        },
    };
    // A `/Nums` the root states through a reference of its own is a third object to rewrite, and
    // this one is not built: the array goes back where the dictionary holding it is.
    if tree
        .get("Nums")
        .is_some_and(|nums| nums.as_reference().is_some())
    {
        return Err(Because::NotBuiltYet(PARENT_TREE_NOT_EXTENDABLE));
    }
    Ok((
        match stated.as_reference() {
            Some(at) => Nums::InItsOwnObject(at),
            None => Nums::InTheRoot(tree),
        },
        pairs,
    ))
}

/// The entries a tagged document's structure tree gains for the pages this conversion appended.
///
/// Two of the producer's own dictionaries change and nothing else does: the structure tree root's
/// `/K` gains the grouping elements and its `/ParentTreeNextKey` moves on, and the parent tree's
/// `/Nums` gains one entry per appended page. `doc/adr/1163`.
#[derive(Debug)]
pub(super) struct Described {
    /// The structure tree root.
    root: ObjectId,
    /// The grouping elements the root's `/K` gains.
    tops: Vec<Object>,
    /// §14.7.5.4's `/ParentTreeNextKey`, past every key this conversion used.
    next_key: i64,
    /// Where the parent tree's `/Nums` is written back.
    tree: Nums,
    /// The `/Nums` pairs, the producer's own followed by this conversion's.
    pairs: Vec<Object>,
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
            if let Nums::InTheRoot(tree) = &self.tree {
                let mut tree = tree.clone();
                tree.insert(Name::new(&b"Nums"[..]), Object::Array(self.pairs.clone()));
                out.insert(Name::new(&b"ParentTree"[..]), Object::Dictionary(tree));
            }
            changed = true;
        }
        if let Nums::InItsOwnObject(at) = &self.tree
            && *at == id
        {
            out.insert(Name::new(&b"Nums"[..]), Object::Array(self.pairs.clone()));
            changed = true;
        }
        changed
    }
}
