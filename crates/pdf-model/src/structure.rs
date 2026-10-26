//! ISO 32000-2 §14.7's logical structure, as far as a *reader* of content needs it.
//!
//! The structure tree says what a page's marks mean: this run of glyphs is a heading, that one
//! is a paragraph, this figure has a description. None of it changes a mark, which is why
//! §14.1 opens by saying the clause's features "do not affect the final appearance of a
//! document" — and why what this module exists for is the half that is *not* appearance.
//!
//! # The one question a content stream can ask
//!
//! A content stream cannot refer to a structure element: §14.7.5.4 says so and gives the
//! reason — "\[b]ecause a stream cannot contain object references, there is no way for content
//! items that are marked-content sequences to refer directly back to their parent structure
//! elements". The standard's answer is the **structural parent tree**, a §7.9.7 number tree
//! keyed by the page's own `/StructParents`, whose value is an array indexed by
//! marked-content identifier.
//!
//! So a `BDC` carrying an `/MCID` can find its element in two lookups, and that is what this
//! module provides. It deliberately does not build the tree top-down: nothing here walks
//! `/K` from `/StructTreeRoot`, because no consumer in this program needs an *ordering* of
//! elements — §12.3.2.3's structure destinations read one element's own entries, and text
//! extraction asks about the element covering a sequence it is already inside.
//!
//! # Where the reading lives
//!
//! This file holds the state ([`Tree`]), the entry points that read `/K` (`of`, `children`,
//! `walk`, `identified_children`, `logical_range`) and the types and free functions the
//! concerns share. Each concern's `impl Tree` is its own module (ADR 2046):
//!
//! | module | concern |
//! |---|---|
//! | `roles` | a type through the role map, its namespace, the foreign and MathML counts |
//! | `attributes` | attribute objects, their owners and inheritance, the artifact and print field |
//! | `tables` | spans, header scope, cell headers, layout spacing, the writing mode, lists |
//! | `owners` | from an object, widget, appearance or identifier to its element |
//! | `logical` | §14.8.2.5's order and the text it gives |
//! | `inline` | Table 380's decoration, baseline shift and text position, as one element states them |

mod attributes;
mod inline;
mod logical;
mod owners;
mod roles;
mod tables;

use std::collections::{BTreeMap, BTreeSet};

use pdf_syntax::{Dictionary, Document, Name, Object, ObjectId, tree};

pub use inline::{InlineStatements, TextDecoration, TextPosition};

/// The `/StructParents`-keyed map from a marked-content identifier to its structure element.
///
/// Built for one page, once, and empty for the vast majority of documents — 89 of the corpus's
/// 974 have a `/StructTreeRoot` at all.
///
/// # What it costs, measured
///
/// Reading it for the specification's own first page is **96 M instructions, 4.8% of
/// interpreting that page** — measured with `callgrind_interpret` against the same page with
/// this struct stubbed out. Almost none of that is the descent: the parent tree's nodes carry
/// `/Limits`, so a lookup visits about one node per level. It is that the structure elements
/// and the tree's own nodes live in **object streams the drawing path never touches**, and
/// reaching them inflates those streams.
///
/// A page that states no `/StructParents` pays one dictionary lookup and nothing else, which is
/// 885 of the 974 corpus documents. The cost is therefore paid by tagged documents, for correct
/// text extraction, and it is written down here rather than hidden: whoever wants it back has
/// two routes — extract text on demand instead of during `interpret`, or read the structure only
/// when a caller asks for text. Both are API changes and neither should be made without a
/// second measurement.
#[derive(Debug, Clone, Default)]
pub struct ParentTree {
    /// The entries this page's marked-content identifiers name, **unresolved**, indexed by
    /// `/MCID`.
    ///
    /// A `Vec` rather than a map because the clause makes the identifier "a zero-based index
    /// into the array", and its NOTE asks producers to keep them small "to conserve space in
    /// the array" — so the array is dense by construction.
    ///
    /// Unresolved because resolving is not free and most entries are never asked about.
    /// Following all forty of the specification's own first page cost **96 M instructions**,
    /// 5% of interpreting the page, for an answer only sixteen marked-content sequences could
    /// have used — measured with `callgrind_interpret`, which is the only reason this is a
    /// `Vec<Object>` and not a `Vec<Dictionary>`.
    entries: Vec<Object>,
}

impl ParentTree {
    /// Reads the entry a page's `/StructParents` names, or an empty map.
    ///
    /// Three things have to be present and any of them may not be: the catalog's
    /// `/StructTreeRoot`, its `/ParentTree`, and the page's own `/StructParents`. A document
    /// missing any of them has no structure for this page, which is not an error.
    #[must_use]
    pub fn for_page(document: &Document, page: &Dictionary) -> Self {
        let Some(key) = document.get_key(page, "StructParents").as_integer() else {
            return Self::default();
        };
        let Ok(catalog) = document.catalog() else {
            return Self::default();
        };
        let root = document.get_key(&catalog, "StructTreeRoot");
        let Some(root) = root.as_dict() else {
            return Self::default();
        };
        let parent_tree = document.get_key(root, "ParentTree");
        let Some(parent_tree) = parent_tree.as_dict() else {
            return Self::default();
        };
        let Some(entry) = tree::lookup(parent_tree, &tree::TreeKey::Number(key), &|object| {
            document.resolve(object)
        }) else {
            return Self::default();
        };

        // "[T]he value shall be an array of indirect references to the sequences' parent
        // structure elements. The array element corresponding to each sequence shall be found
        // by using the sequence's marked-content identifier as a zero-based index into the
        // array." A file whose entry is a single element instead — which is the form
        // §14.7.5.4 gives an *object* content item — has one sequence, so it reads as an array
        // of one rather than as nothing.
        let entries: Vec<Object> = match entry {
            Object::Array(items) => items.clone(),
            element @ Object::Dictionary(_) => vec![element],
            _ => Vec::new(),
        };
        Self { entries }
    }

    /// The structure element a marked-content identifier belongs to.
    ///
    /// Resolved here rather than when the tree was read; see `Self::entries`.
    #[must_use]
    pub fn element(&self, document: &Document, mcid: i64) -> Option<Dictionary> {
        let index = usize::try_from(mcid).ok()?;
        document
            .resolve(self.entries.get(index)?)
            .as_dict()
            .cloned()
    }

    /// Whether this page names any structure at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Deepest nesting of `/K` walked when the structure tree is read.
///
/// §14.7.2 makes the hierarchy a tree, and `/K` is a reference a document controls, so a
/// file may state a cycle or a chain thousands deep. Real documents nest a handful of levels
/// — a part, a section, a paragraph, a span — and this is far past any of them.
const MAX_DEPTH: usize = 64;

/// Most children read from one `/K` array.
///
/// A page of a tagged document is one element with a child per marked-content sequence, so
/// this is a bound on a *document's* fan-out rather than on its depth. It is deliberately
/// large: a table of a thousand cells is one element with a thousand children and is not
/// malformed.
const MAX_CHILDREN: usize = 65_536;

/// Most items one [`Tree::walk`] returns, over the whole tree rather than one `/K`.
///
/// **Not [`MAX_CHILDREN`], which would cut the largest document this project owns to little over
/// half without saying so** — ISO 32000-2's structure tree is **129 389** items and a walk bounded
/// that way stops at 71 371 of them (ADR 0257). A bound that truncates a *valid* file is
/// measuring this project's patience rather than the file's malice, so it is set from the
/// largest real tree in reach with room above it, and [`Reading::truncated`] says when it is
/// reached at all.
///
/// One million items is eight times that tree and 40 MB of `(usize, Child)` at this crate's
/// sizes — measured, `size_of` is 40 — which is the shape of a bound that stops a malformed
/// file without stopping a large one. The whole 129 389 take **151 ms** to produce.
const MAX_ELEMENTS: usize = 1 << 20;

/// Most structure elements [`Tree::unkeyed_widget_owners`]'s search may enter.
///
/// That search runs §14.7.5.3's association backwards — from a widget annotation to the element
/// whose `/K` names it — and finding an element by its content is a walk of the tree, which is
/// exactly the cost ADR 0325 took off a page turn. So it is bounded by what one page's *answer*
/// may cost rather than by the tree's size: `viewer_core::accessibility::MAX_NODES` is 8192 and
/// documented as how many nodes one page's tree may hold, and a recovery free to enter more
/// elements than the answer may hold would spend a page's whole budget looking for one of its
/// content items.
///
/// The standard states no number here, so this is a decision rather than a derivation (trap 38),
/// and what it buys is measured: a page that omits Table 359's key costs a bounded search instead
/// of the 114 ms ISO 32000-2's 129 389-element tree takes to walk, on each of the 37 of its pages
/// that state a widget without that key. [`Reading::truncated`] says when it bit, and no corpus
/// document loses an element to it — the two whose trees name an object stating no
/// `/StructParent` hold 527 and 1922 elements (ADR 1151).
const MAX_OBJECT_SEARCH: usize = 8192;

/// One child of a structure element, in the four forms §14.7.5.1.1 defines.
///
/// > Content items are of two kinds:
///
/// marked-content sequences within content streams, and complete PDF objects such as
/// annotations and `XObject`s —
///
/// and the third possibility is not a content item at all but another element, which is what
/// makes the structure a tree. The clause's own restriction is what keeps this an enum rather
/// than a recursive type: "\[c]ontent items shall be leaf nodes of the structure tree".
#[derive(Debug, Clone, PartialEq)]
pub enum Child {
    /// Another structure element.
    Element(Dictionary),
    /// §14.7.5.2's marked-content sequence, by its identifier and the page it is on.
    ///
    /// The identifier arrives either as a bare integer — Table 355 makes `/Pg` required when
    /// it does — or through a marked-content reference dictionary (Table 357), which may name
    /// a different page and may name a stream other than the page's own content. The page is
    /// `None` where neither the reference nor the enclosing element states one.
    MarkedContent {
        /// The `/MCID` this sequence carries in the content stream.
        ///
        /// **Not a key on its own**: §14.7.5.2 makes it unique "within its content stream" and
        /// says nothing wider, so [`Self::MarkedContent::stream`] is the rest of it.
        mcid: i64,
        /// The page object it belongs to, if one was stated.
        page: Option<ObjectId>,
        /// Table 357's `/Stm`: the content stream the sequence is in, where it is not the page's.
        ///
        /// ISO 32000-2 §14.7.5.2, Table 357:
        ///
        /// > The content stream containing the marked-content sequence. This entry should be
        /// > present only if the marked-content sequence resides in a content stream other than
        /// > the content stream for the page (see 8.10, "Form XObjects" and 12.5.5, "Appearance
        /// > streams"). If this entry is absent, the marked-content sequence shall be contained
        /// > in the content stream of the page identified by Pg (either in the marked-content
        /// > reference dictionary or in the parent structure element).
        ///
        /// So `None` is not "unknown": it is the clause's own statement that the sequence is in
        /// the page's own `/Contents`. [`crate::content::ContentStream::named_by`] is the match.
        stream: Option<ObjectId>,
        /// Table 357's `/StmOwn`: the object that references the stream `/Stm` names.
        ///
        /// ISO 32000-2 §14.7.5.2, Table 357:
        ///
        /// > The indirect reference to the PDF object referencing the stream identified by the
        /// > Stm key. NOTE A common use for this would be to identify the annotation dictionary
        /// > owning the appearance stream.
        ///
        /// So where it names one of the page's annotations it is the same statement §14.7.5.3's
        /// object reference makes — this element's content belongs to that annotation — made
        /// from the marked-content side, and it is read into the same channel: the annotation's
        /// `/Rect` places the element and §12.7's control says what it is.
        owner: Option<ObjectId>,
    },
    /// §14.7.5.3's object reference (Table 358): a whole object, such as an annotation.
    Object {
        /// The object itself, from `/Obj`.
        object: ObjectId,
        /// The page it is on, from `/Pg`, if stated.
        page: Option<ObjectId>,
    },
}

/// §14.7.2's structure tree, read from the catalog's `/StructTreeRoot`.
///
/// The tree is walked on demand rather than built: a tagged document's structure has an
/// element per paragraph and a child per marked-content sequence, and nothing here needs all
/// of them at once. [`Tree::children`] is the whole traversal, and it is the same function
/// for the root and for an element because Table 354 and Table 355 give `/K` the same
/// meaning in both — "\[t]he K entry shall specify the immediate children of the structure
/// tree root, which shall be structure elements".
///
/// # What this is for
///
/// The parent tree above answers "which element does this marked-content sequence belong
/// to", which is what drawing a page needs. This answers the other direction — what the
/// document *says it is* — which is what a reading-order consumer, an accessibility tree or
/// a navigation panel needs. `viewer_core::Query::AccessibilityTree` answers with this
/// (ADR 0134), and `viewer-accessibility` puts the answer on AT-SPI
/// through AccessKit (ADR 0214) — so [`Tree::role`]'s §14.7.3 mapping and [`StandardType`] are
/// read by a program a person uses rather than by tests alone.
#[derive(Debug, Clone)]
pub struct Tree {
    /// The structure tree root dictionary itself.
    root: Dictionary,
}

impl Tree {
    /// Reads the catalog's `/StructTreeRoot`, if the document has one.
    ///
    /// `None` for an untagged document, which is 885 of the corpus's 974.
    #[must_use]
    pub fn of(document: &Document) -> Option<Self> {
        let catalog = document.catalog().ok()?;
        let root = document.get_key(&catalog, "StructTreeRoot");
        Some(Self {
            root: root.as_dict()?.clone(),
        })
    }

    /// The immediate children of an element, or of the root when `element` is `None`.
    ///
    /// `/K` is "a dictionary or array" at the root and one of four things at an element, and
    /// an array may mix all of them; a dictionary with no `/Type` is a structure element,
    /// which Table 355 states outright — "\[i]f the value of K is a dictionary containing no
    /// Type entry, it shall be assumed to be a structure element dictionary".
    ///
    /// `inherited_page` is the `/Pg` of the element being asked about, because Table 355
    /// makes that entry the page for the integer form of a content item.
    #[must_use]
    pub fn children(&self, document: &Document, element: Option<&Dictionary>) -> Vec<Child> {
        let node = element.unwrap_or(&self.root);
        // The *reference*, not the page it resolves to: `Document::get_key` resolves,
        // and what identifies a page here is its identity.
        let page = node.get("Pg").and_then(Object::as_reference);
        let kids = document.get_key(node, "K");
        let mut out = Vec::new();
        match &kids {
            Object::Array(items) => {
                for item in items.iter().take(MAX_CHILDREN) {
                    if let Some(child) = Self::child(document, item, page) {
                        out.push(child);
                    }
                }
            }
            _ => {
                if let Some(child) = Self::child(document, &kids, page) {
                    out.push(child);
                }
            }
        }
        out
    }

    /// One entry of a `/K`, in whichever of the four forms it takes.
    ///
    /// # One resolution, and the dictionary is moved rather than copied
    ///
    /// [`Document::resolve`] answers with an **owned** object, so every call is a deep copy of
    /// whatever the document's cache holds — and for a `/K` entry naming a structure element that
    /// is the element's whole dictionary, its own `/K` array included. This asked for three of
    /// them per child: one to test for §14.7.5.1.1's bare integer, a second for the dictionary,
    /// and a third where the entry turned out to be an element. It asks for one and moves it into
    /// [`Child::Element`].
    ///
    /// It is worth the paragraph because of where this function sits rather than because of the
    /// arithmetic. `viewer_core`'s accessibility walk resolves **every child of every ancestor**
    /// of a page's own elements, to find out which of them are elements at all — so on a document
    /// whose structure tree is the size of its page count this is where a screen reader's page
    /// turn goes: 70.8% of `Query::AccessibilityTree` on ISO 32000-2's page 700, measured under
    /// callgrind. ADR 0394 has the A/B and `viewer-core --example accessibility_cost` is the
    /// instrument.
    fn child(document: &Document, entry: &Object, page: Option<ObjectId>) -> Option<Child> {
        let resolved = document.resolve(entry);
        if let Some(mcid) = resolved.as_integer() {
            // §14.7.5.2 on the bare-integer form: it "may be done in the common case where the
            // marked-content sequence is contained in the content stream of the page that is
            // specified in the Pg entry of the structure element dictionary". So an integer names
            // the page's own stream, with no `/Stm` to say otherwise.
            return Some(Child::MarkedContent {
                mcid,
                page,
                stream: None,
                owner: None,
            });
        }
        let Object::Dictionary(dict) = resolved else {
            return None;
        };
        let kind = document.get_key(&dict, "Type");
        match kind.as_name().map(Name::as_bytes) {
            // Table 357: a marked-content reference names the sequence and may move both the
            // page and the stream it lives in.
            Some(b"MCR") => Some(Child::MarkedContent {
                mcid: document.get_key(&dict, "MCID").as_integer()?,
                page: dict.get("Pg").and_then(Object::as_reference).or(page),
                // The *reference*, like `/Pg` above and for the same reason: Table 357 makes
                // `/Stm` an indirect reference, and what identifies a content stream is which
                // object it is rather than what its bytes resolve to.
                stream: dict.get("Stm").and_then(Object::as_reference),
                // The same again for `/StmOwn` — "shall be an indirect reference", and the
                // object it names is the identity a consumer matches against the page's
                // annotations.
                owner: dict.get("StmOwn").and_then(Object::as_reference),
            }),
            // Table 358: an object reference. `/Obj` is required and is what identifies it.
            Some(b"OBJR") => Some(Child::Object {
                object: dict.get("Obj").and_then(Object::as_reference)?,
                page: dict.get("Pg").and_then(Object::as_reference).or(page),
            }),
            _ => Some(Child::Element(dict)),
        }
    }

    /// Every descendant of the root, depth first, with its depth.
    ///
    /// The order is §14.7.2's own: `/K` is a list, and the tree's order is what §14.8.2 calls
    /// the document's logical reading order. Bounded by `MAX_DEPTH`, by `MAX_ELEMENTS` and
    /// by visiting each element once, because `/K` and `/P` are references a document controls.
    ///
    /// The bound is *reported* rather than applied in silence — [`Reading::truncated`] — because a
    /// walk cut short is not the document's logical order and a consumer that cannot tell the
    /// two apart has been handed a partial reading as a complete one.
    #[must_use]
    pub fn walk(&self, document: &Document) -> Walk {
        let mut walk = Walk {
            items: Vec::new(),
            truncated: false,
        };
        let mut seen: BTreeSet<ObjectId> = BTreeSet::new();
        self.descend(document, None, 0, &mut walk, &mut seen);
        walk
    }

    /// A *range* of a page's readback, put into §14.8.2.5's logical content order, in the pieces
    /// the structure divides it into.
    ///
    /// [`Self::logical_text`] answers this question for the whole page; this answers it for the
    /// part of the page a person selected, which is the form a host needs when that person
    /// presses copy. A page whose producer wrote its columns out of order gives its text in that
    /// order, and §14.8.2.5.1 is where the other one is defined:
    ///
    /// > Logical content order -the ordering for semantic purposes -shall be defined by a
    /// > depth-first traversal of the document's logical structure hierarch y.
    ///
    /// Each piece is one content item's share of the range, in the traversal's order,
    /// with the [`Boundary`] the structure puts between it and the piece before — so a consumer
    /// joining them knows where a paragraph ended and where a table cell did, which the readback's
    /// own separators cannot say (they are inferred from where glyphs landed, in the order they
    /// were *drawn*). What character stands for each boundary is the consumer's choice.
    ///
    /// # What is real content, and what is left out
    ///
    /// §14.8.2.2.1: "\[w]here artifacts are to be included in the structure tree, they shall be
    /// included through the Artifact structure element type, and shall not be considered real
    /// content." So a sequence under an `Artifact` element gives no piece, and neither does a
    /// range of `artifacts` that §14.8.2.2.2's marked-content method declared — the running head
    /// and the folio a producer tagged `/Artifact`. Both are *accounted for*: they are what the
    /// producer said they are, not a part of the selection the tree failed to reach (ADR 1948).
    ///
    /// # Why it may answer `None`, which is the whole design
    ///
    /// A byte that is neither reached by the traversal nor declared an artifact is content the
    /// structure says nothing about. §14.8.2.2.2's other sentence calls it an artifact too — "\[a]ny
    /// content that is not included in the structure tree is an artifact" — but that is this
    /// reader's inference from an absence, and it cannot tell a decoration a producer left untagged
    /// from a paragraph its tree forgot. Leaving it out of what a person dragged over would make a
    /// copy that silently lost a paragraph, so this answers `Some` only where every byte is
    /// reached or declared, and the caller keeps the content order it already has otherwise —
    /// a worse reading and a complete one.
    ///
    /// `None` also for an empty range, for a page with no structure, for a truncated traversal
    /// (where "every byte reached" would mean "every byte the prefix reached"), and where the range
    /// is artifacts and nothing else: a person who dragged across a page number and pressed copy
    /// asked for the page number.
    ///
    /// # A `Private` element is copied, and no type inside it is read
    ///
    /// ISO 32000-1's Table 333 — the schema of the PDF 1.7 namespace, the only one that defines
    /// `Private` (§14.8.6.1, Annex M) — says neither the element nor any of its descendants shall
    /// be interpreted or exported to other document formats. A copy is not the second: §14.8.1's
    /// NOTE lists the operations tagged PDF serves, and pasting is one and conversion another,
    ///
    /// > Extraction of text and graphics for pasting into other applications
    ///
    /// > Conversion to other common file formats (such as HTML, XML, and RTF) with document
    /// > structure preserved
    ///
    /// The first half binds: the subtree's place in the traversal is §14.7's, but no element in it
    /// — the `Private` one included — is read as its type, so none ends a block or a cell and none
    /// is an `Artifact`, and the readback's own separators divide what it holds. Its content is
    /// then copied as the page order copies it (ADR 2042).
    ///
    /// # Annotations are placed where the tree names them
    ///
    /// An annotation's appearance reads back into the page's text after the page's own content,
    /// and it carries no `/MCID`, so the only statement of where it belongs is the tree's object
    /// reference. §14.8.2.5.2 makes that the rule:
    ///
    /// > The position of an annotation in the logical content order is determined from the
    /// > document's logical structure.
    ///
    /// So an `OBJR` naming one of `objects` gives that annotation's text a piece at its place in
    /// the traversal, once however often the tree names it — Table 358's NOTE 2: "if it is
    /// rendered multiple times on the same page, just a single object reference suffices". Bytes of
    /// it that a marked-content sequence already covers are that sequence's, because §14.7.5.3's
    /// NOTE 1 sends a part of an object's stream to §14.7.5.2 (ADR 1970).
    ///
    /// Takes the readback and its spans rather than a whole [`crate::Interpretation`], because
    /// the one caller that has a selection keeps those and not the interpretation — the same
    /// shape [`crate::accessibility::speech`] takes and for the same reason.
    #[must_use]
    pub fn logical_range(
        &self,
        document: &Document,
        page: ObjectId,
        spans: ContentSpans<'_>,
        range: std::ops::Range<usize>,
    ) -> Option<Vec<LogicalPiece>> {
        /// One element the traversal is inside.
        struct Open {
            depth: usize,
            ends_with: Boundary,
            category: Option<Category>,
            artifact: bool,
            private: bool,
        }
        if range.is_empty() {
            return None;
        }
        let ContentSpans {
            marked,
            artifacts,
            objects,
        } = spans;
        let walk = self.walk(document);
        if walk.truncated {
            return None;
        }
        let mut accounted = Accounted::new(range.clone(), artifacts);
        let mut pieces: Vec<LogicalPiece> = Vec::new();
        let mut open: Vec<Open> = Vec::new();
        let mut between = Boundary::Flow;
        let mut placed_objects: BTreeSet<ObjectId> = BTreeSet::new();
        for (depth, child) in walk.items {
            // The walk is depth-first and states depths, so an element has ended when an item no
            // deeper than it arrives; what it ends with is owed to the next piece.
            while let Some(top) = open.pop_if(|top| top.depth >= depth) {
                between = between.max(top.ends_with);
            }
            let item: Vec<std::ops::Range<usize>> = match child {
                Child::Element(element) => {
                    let parent = open.last();
                    let role = self.standard_role(document, &element);
                    // ISO 32000-1 Table 333, the PDF 1.7 namespace's `Private`: "Neither the
                    // Private element nor any of its descendants shall be interpreted or exported
                    // to other document formats." A copy is not an export (ADR 2042), so the
                    // subtree's content stays; its types are not read, which is what this walk
                    // does with a type it does not know.
                    let private = role == Some(StandardType::Private)
                        || parent.is_some_and(|parent| parent.private);
                    let kind = if private { None } else { role };
                    let (ends_with, category) =
                        Boundary::ending(kind.as_ref(), parent.and_then(|parent| parent.category));
                    open.push(Open {
                        depth,
                        ends_with,
                        category,
                        artifact: kind == Some(StandardType::Artifact)
                            || parent.is_some_and(|parent| parent.artifact),
                        private,
                    });
                    continue;
                }
                // §14.7.5.2's identifier and the stream it is unique within; see
                // [`Self::logical_text`].
                Child::MarkedContent {
                    mcid,
                    stream,
                    page: at,
                    ..
                } if at.is_none_or(|at| at == page) => {
                    crate::content::named_sequences(marked, mcid, stream)
                        .into_iter()
                        .map(|span| span.range.clone())
                        .collect()
                }
                // §14.8.2.5.2's annotation, at the place the tree names it, once.
                Child::Object { object, page: at }
                    if at.is_none_or(|at| at == page) && placed_objects.insert(object) =>
                {
                    objects
                        .iter()
                        .filter(|span| span.object == Some(object))
                        .flat_map(|span| outside_sequences(span.range.clone(), marked))
                        .collect()
                }
                Child::MarkedContent { .. } | Child::Object { .. } => continue,
            };
            let artifact = open.last().is_some_and(|parent| parent.artifact);
            for span in item {
                let Some(share) = accounted.cover(&span) else {
                    continue;
                };
                if artifact {
                    continue;
                }
                pieces.push(LogicalPiece {
                    range: share,
                    after: if pieces.is_empty() {
                        Boundary::Flow
                    } else {
                        between
                    },
                });
                between = Boundary::Flow;
            }
        }
        (accounted.whole() && !pieces.is_empty()).then_some(pieces)
    }

    /// [`Self::children`], with the object each child was reached through where it was reached
    /// through one.
    ///
    /// The identity is what `Self::descend` remembers, and it is deliberately not on
    /// [`Child`]: an element's *identity* is a fact about how the walk got there rather than
    /// about what the element is, and every other consumer of `children` wants the four forms
    /// §14.7.5.1.1 defines and nothing beside them.
    ///
    /// It is public because a second walk needs the same fact for a different reason: a caller
    /// that already knows *which* elements a page reaches — [`Self::elements_on_page`] — has to
    /// recognise them among their siblings, and a dictionary is not an identity.
    #[must_use]
    pub fn identified_children(
        &self,
        document: &Document,
        element: Option<&Dictionary>,
    ) -> Vec<(Child, Option<ObjectId>)> {
        let node = element.unwrap_or(&self.root);
        let page = node.get("Pg").and_then(Object::as_reference);
        let kids = document.get_key(node, "K");
        let mut out = Vec::new();
        match &kids {
            Object::Array(items) => {
                for item in items.iter().take(MAX_CHILDREN) {
                    if let Some(child) = Self::child(document, item, page) {
                        out.push((child, item.as_reference()));
                    }
                }
            }
            _ => {
                if let Some(child) = Self::child(document, &kids, page) {
                    out.push((child, node.get("K").and_then(Object::as_reference)));
                }
            }
        }
        out
    }
}

/// [`Tree::unkeyed_widget_owners`]'s state, in one place because it is one search.
///
/// Four things travel together down a descent that is looking for something: what it is looking
/// for, which elements it has entered, how many more it may enter, and what it has found. Passing
/// them separately is what the argument count above seven is warning about, and a struct that
/// exists once per search is what they are.
struct OwnerSearch<'a> {
    /// The objects whose element is wanted: this page's widgets that state no `/StructParent`.
    wanted: &'a BTreeSet<ObjectId>,
    /// Every element entered, by identity, so that a `/K` naming its own ancestor terminates.
    entered: BTreeSet<ObjectId>,
    /// How many more elements may be entered, from [`MAX_OBJECT_SEARCH`] downwards.
    budget: usize,
    /// The elements found, and whether the budget ran out before the tree did.
    found: Reading<ObjectId>,
}

/// A reading of the structure tree, and whether the bound cut it short.
///
/// A bare `Vec` cannot say "there was more", and a walk that stops at `MAX_ELEMENTS` has
/// produced a *prefix* of §14.8.2.5's logical order rather than the order itself. Both readings
/// this crate offers carry the flag, because both are §14.7.2's tree in a different shape and a
/// consumer of either has the same thing to lose by not knowing.
#[derive(Debug, Clone, PartialEq)]
pub struct Reading<T> {
    /// What was read, in the tree's own order.
    pub items: Vec<T>,
    /// Whether `MAX_ELEMENTS` stopped the read before the tree ran out.
    ///
    /// `false` for every document in reach of this project — ISO 32000-2's tree is the largest
    /// it owns and is an order of magnitude below the bound.
    pub truncated: bool,
}

/// [`Tree::walk`]'s answer: every element and content item, with the depth it was found at.
pub type Walk = Reading<(usize, Child)>;

/// The spans of one page's readback that [`Tree::logical_range`] orders: the three routes by which
/// a byte of [`crate::Interpretation::text`] is accounted for.
///
/// One struct rather than three slices because they are always asked together and are one
/// interpretation's: a caller holding a [`crate::Interpretation`] passes
/// [`ContentSpans::of`], and one that kept the spans and not the interpretation names them.
#[derive(Debug, Clone, Copy, Default)]
pub struct ContentSpans<'a> {
    /// §14.7.5.2's marked-content sequences: [`crate::Interpretation::marked`].
    pub marked: &'a [crate::content::MarkedSpan],
    /// §14.8.2.2's declared artifacts: [`crate::Interpretation::artifacts`]. Those found by
    /// absence are ignored, because they are this reader's inference (ADR 1948).
    pub artifacts: &'a [crate::content::ArtifactSpan],
    /// §14.7.5.3's annotations the tree claims: [`crate::Interpretation::objects`].
    pub objects: &'a [crate::content::ObjectSpan],
}

impl<'a> ContentSpans<'a> {
    /// All three of one interpretation's spans.
    #[must_use]
    pub fn of(interpretation: &'a crate::Interpretation) -> Self {
        Self {
            marked: &interpretation.marked,
            artifacts: &interpretation.artifacts,
            objects: &interpretation.objects,
        }
    }
}

/// Which bytes of a range of the readback [`Tree::logical_range`] has accounted for.
///
/// A set rather than a running total, because a `BDC` inside a `BDC` gives two spans over one
/// byte: counting would report full coverage for a range half of which was covered twice.
struct Accounted {
    /// The range asked about.
    range: std::ops::Range<usize>,
    /// One flag per byte of it.
    bytes: Vec<bool>,
}

impl Accounted {
    /// Nothing accounted for yet but §14.8.2.2's declared artifacts, which are the producer's
    /// statement of what they are (ADR 1948).
    fn new(range: std::ops::Range<usize>, artifacts: &[crate::content::ArtifactSpan]) -> Self {
        let mut accounted = Self {
            bytes: vec![false; range.len()],
            range,
        };
        for artifact in artifacts
            .iter()
            .filter(|artifact| artifact.found == crate::content::ArtifactSource::Declared)
        {
            accounted.cover(&artifact.range);
        }
        accounted
    }

    /// Marks the part of `span` inside the range, and answers that part — `None` where there is
    /// none.
    fn cover(&mut self, span: &std::ops::Range<usize>) -> Option<std::ops::Range<usize>> {
        let (from, to) = (
            span.start.max(self.range.start),
            span.end.min(self.range.end),
        );
        if from >= to {
            return None;
        }
        for byte in self
            .bytes
            .get_mut(from.saturating_sub(self.range.start)..to.saturating_sub(self.range.start))
            .unwrap_or_default()
        {
            *byte = true;
        }
        Some(from..to)
    }

    /// Whether every byte of the range is accounted for.
    fn whole(&self) -> bool {
        self.bytes.iter().all(|seen| *seen)
    }
}

/// The parts of `range` that no marked-content sequence covers, in order.
///
/// An annotation's range is its whole appearance's readback, and a sequence inside that appearance
/// is §14.7.5.2's content item rather than the object's: §14.7.5.3's NOTE 1 says that where "the
/// referenced content forms only part of the object's content stream, it is instead handled as a
/// marked -content sequence". Taking those bytes out is what keeps an annotation the tree names
/// both ways from being copied twice.
fn outside_sequences(
    range: std::ops::Range<usize>,
    marked: &[crate::content::MarkedSpan],
) -> Vec<std::ops::Range<usize>> {
    let mut inside: Vec<std::ops::Range<usize>> = marked
        .iter()
        .map(|span| span.range.start.max(range.start)..span.range.end.min(range.end))
        .filter(|part| part.start < part.end)
        .collect();
    inside.sort_by_key(|part| part.start);
    let mut out = Vec::new();
    let mut at = range.start;
    for part in inside {
        if part.start > at {
            out.push(at..part.start);
        }
        at = at.max(part.end);
    }
    if at < range.end {
        out.push(at..range.end);
    }
    out
}

/// One content item's share of a range of a page's readback, in §14.8.2.5.1's logical
/// content order: [`Tree::logical_range`]'s answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalPiece {
    /// The bytes of [`crate::Interpretation::text`] this piece is.
    pub range: std::ops::Range<usize>,
    /// What the structure puts between this piece and the one before it; [`Boundary::Flow`] for
    /// the first.
    pub after: Boundary,
}

/// What the structure tree says lies between two pieces of logical content.
///
/// Ordered by strength, so that where several elements end between two pieces — a cell, its row
/// and the table — the strongest is the one that stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Boundary {
    /// Nothing: the two are in one run of text, whatever inline elements open or close between
    /// them. §14.8.4.7.1 makes an inline element a part of its block's content — "\[i]nline
    /// structure elements and portions of actual content inside a parent structure element may
    /// occur in any combination and in any order" — so a `Span` or an `/ActualText` boundary
    /// divides no word.
    Flow,
    /// One table cell ended and the next one in its row begins: Table 371's `TH` or `TD`.
    Cell,
    /// A block ended. §14.8.3.3: "BLSEs shall be stacked within a reference area in
    /// block-progression order", so what follows one begins on a line of its own.
    Block,
}

impl Boundary {
    /// What an element ends with, and the category it is in, given its parent's category.
    ///
    /// §14.8.4.1's categories decide it, with [`Category::of`]'s rule for a type that may be
    /// either. Three exceptions, each a choice (ADR 1948):
    ///
    /// - **`Link`, `Annot` and `Form` end with nothing.** Table 368 makes each one about an
    ///   annotation — a `Link` is "\[a]n association between content enclosed by the Link structure
    ///   element and a corresponding link annotation" — so the content it encloses is the
    ///   surrounding text's, and a link inside a list item's body, which §14.8.4.1's rule makes
    ///   block level because `LBody` is not a block, would otherwise break its sentence where the
    ///   link ends.
    /// - **`Sub` ends a line**, though Table 367 calls it inline: it "\[e]ncloses content typically
    ///   perceived as a sub-division inside a block level structure element", and every example
    ///   the table gives is a line — of a verse, of source code, of a postal address.
    /// - **An element whose type is not standard ends with nothing** and takes its parent's
    ///   category: §14.8.4.1 requires a role map to a standard type, and a reader that invented a
    ///   boundary for a name it cannot read would be dividing text on a guess.
    fn ending(kind: Option<&StandardType>, parent: Option<Category>) -> (Self, Option<Category>) {
        let Some(kind) = kind else {
            return (Self::Flow, parent);
        };
        let category = kind.category().of(parent);
        let ends_with = match kind {
            StandardType::TableHeader | StandardType::TableData => Self::Cell,
            StandardType::Link | StandardType::Annotation | StandardType::Form => Self::Flow,
            StandardType::SubBlock => Self::Block,
            _ if category == Category::Inline => Self::Flow,
            _ => Self::Block,
        };
        (ends_with, Some(category))
    }
}

/// §14.8.5.5's statement that one `L` element carries on from another. Table 382.
///
/// The two shapes are the clause's own two cases, and they differ in *who* names the predecessor:
/// the document, or the clause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListContinuation {
    /// `/ContinuedFrom`: "\[t]he ID … of the list for which this list is a continuation".
    ///
    /// Table 355's `/ID` of another element, **unresolved** — [`list_predecessors`] is what turns
    /// it into a place in a walk, for the reason [`Tree::cell_headers`] returns identifiers too.
    From(Vec<u8>),
    /// The list states no `/ContinuedFrom`, so the clause says which list it means.
    ///
    /// ISO 32000-2 §14.8.5.5, Table 382:
    ///
    /// > If the ContinuedFrom attribute is not present, the continuation is from the preceding
    /// > list at the same level in the structure hierarchy.
    Preceding,
}

/// One `L` element of a walk, as [`list_predecessors`] needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListEntry {
    /// Where the list is in the caller's own walk, and what it gets back as an answer.
    ///
    /// Opaque here: this function compares positions for order and returns them unchanged, so a
    /// caller numbering its elements any way that increases along the walk gets its own numbers.
    pub position: usize,
    /// How deep the list is in §14.7.2's tree, which is what "the same level" is measured in.
    pub depth: usize,
    /// Table 355's `/ID`, where the list states one: "\[t]he element identifier, a byte string
    /// designating this structure element."
    pub id: Option<Vec<u8>>,
    /// [`Tree::list_continuation`]'s answer for this list.
    pub continuation: Option<ListContinuation>,
}

/// §14.8.5.5's continuations resolved over one walk: which list each continuing list continues.
///
/// `lists` is every `L` element the walk reached, **in walk order**, and the answer maps a
/// continuing list's `position` to its predecessor's. A continuing list that resolves to nothing is
/// absent from the map rather than pointed at something invented — WTPDF's own NOTE on this pair
/// says why that is still worth reporting: "\[t]here is value to the ContinuedList attribute even
/// when the previous list is not present, since it would help to explain the partial nature of the
/// content, for example, partial numbering."
///
/// # Only lists, and only earlier ones
///
/// `/ContinuedFrom` names "the ID … of the **list**", so an identifier belonging to a paragraph
/// names nothing here — which is why the population is the walk's `L` elements rather than all of
/// its elements.
///
/// And a predecessor is always **earlier in the walk**, on both routes. The clause's own fallback
/// says *preceding*, and a `/ContinuedFrom` naming a later list contradicts the entry's own words —
/// a continuation carries on from something that came before it. The rule also keeps the answer
/// usable by a caller that is building a parent-first list and can only point backwards, which is
/// the same constraint [`Tree::cell_headers`]'s identifiers are resolved under.
///
/// "The same level in the structure hierarchy" is read as the same depth, which is what the clause
/// says: a list at depth 3 under one section and a list at depth 3 under the next are at one level
/// whether or not they share a parent. In the shape the entry exists for — two lists with other
/// content between them — the two readings agree, because such lists are siblings.
#[must_use]
#[expect(
    clippy::doc_markdown,
    reason = "a verbatim quotation: WTPDF spells the attribute without backticks, and adding them \
              inside the quotation marks would make the conformance gate's quotation check fail"
)]
pub fn list_predecessors(lists: &[ListEntry]) -> BTreeMap<usize, usize> {
    let mut out = BTreeMap::new();
    // Both built as the walk is replayed rather than up front, which is what bounds a predecessor
    // to a list already passed without a second comparison.
    let mut by_id: BTreeMap<&[u8], usize> = BTreeMap::new();
    let mut preceding: BTreeMap<usize, usize> = BTreeMap::new();
    for list in lists {
        match &list.continuation {
            Some(ListContinuation::From(id)) => {
                if let Some(&found) = by_id.get(id.as_slice()) {
                    out.insert(list.position, found);
                }
            }
            Some(ListContinuation::Preceding) => {
                if let Some(&found) = preceding.get(&list.depth) {
                    out.insert(list.position, found);
                }
            }
            None => {}
        }
        if let Some(id) = list.id.as_deref() {
            // The first list to state an identifier keeps it: §14.7.2's `/IDTree` maps each
            // identifier to one element, so a repeated one is a document contradicting itself and
            // the earlier element is the one a `/ContinuedFrom` before this point already got.
            by_id.entry(id).or_insert(list.position);
        }
        preceding.insert(list.depth, list.position);
    }
    out
}

/// §14.8.5.6's `PrintField` attributes: what a non-interactive form field *was*. Table 383.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrintField {
    /// `/Role`, "\[t]he type of form field represented".
    ///
    /// `None` is Table 383's own default — "\[d]efault value: None specified" — which is an
    /// element carrying the other two entries and not saying what kind of field it was.
    pub role: Option<FieldRole>,
    /// `/Checked` (or the deprecated `/checked`), "\[t]he state of a radio button or check box
    /// field". Table 383's default is [`Checked::Off`].
    pub checked: Checked,
    /// `/Desc`, "\[t]he alternate name of the field", which NOTE 3 likens to an interactive
    /// field's `/TU`.
    pub description: Option<String>,
}

/// Table 383's five field roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldRole {
    /// `rb`: a radio button.
    RadioButton,
    /// `cb`: a check box.
    CheckBox,
    /// `pb`: a push button.
    PushButton,
    /// `tv`: a text-value field.
    ///
    /// The clause says where its value is, and it is not in the attribute: "\[t]he text that is
    /// the value of the field shall be the content of the Form structure element", so a
    /// consumer reads the element's own text — which §14.8.2.5's logical order already gives it.
    TextValue,
    /// `lb`: a listbox field.
    ListBox,
}

impl FieldRole {
    /// One of Table 383's five names, or `None` for anything else.
    #[must_use]
    pub fn read(name: &[u8]) -> Option<Self> {
        Some(match name {
            b"rb" => Self::RadioButton,
            b"cb" => Self::CheckBox,
            b"pb" => Self::PushButton,
            b"tv" => Self::TextValue,
            b"lb" => Self::ListBox,
            _ => return None,
        })
    }
}

/// Table 383's `/Checked`: the state of a box that cannot be clicked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Checked {
    /// `on`.
    On,
    /// `off`, the table's default.
    #[default]
    Off,
    /// `neutral` — neither on nor off, which a printed form leaves possible.
    Neutral,
}

impl Checked {
    /// One of Table 383's three names, or `None` for anything else.
    #[must_use]
    pub fn read(name: &[u8]) -> Option<Self> {
        Some(match name {
            b"on" => Self::On,
            b"off" => Self::Off,
            b"neutral" => Self::Neutral,
            _ => return None,
        })
    }
}

/// Table 384's `/Scope`: which of a table's axes one `TH` cell's content describes.
///
/// ISO 32000-2 §14.8.4.8.3 gives the header cell itself as
///
/// > A table header cell containing content describing one or more rows, columns or rows and
/// > columns of the table.
///
/// and this entry is what says which of the three it is. Nothing about the page depends on it —
/// a header cell was drawn by the marks its content stream made, whatever it describes — and
/// everything about *reading* the table does: a screen reader announces a cell's headers before
/// the cell, and a row's header announced as a column's puts the wrong word in front of every
/// cell in the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderScope {
    /// `Row`: the cell describes the row it is in.
    Row,
    /// `Column`: the cell describes the column it is in.
    Column,
    /// `Both`: the cell describes both.
    ///
    /// Table 384's assumption for a cell in the first row *and* the first column, and for one in
    /// neither — which is the corner cell of a table with headers on two sides, and every header
    /// cell buried inside one.
    Both,
}

impl HeaderScope {
    /// One of Table 384's three names, or `None` for anything else.
    ///
    /// ISO 32000-2 §14.8.5.7:
    ///
    /// > A name whose value shall be one of the following: Row, Column
    ///
    /// — and `Both`, which the Markdown conversion of the standard breaks across a space.
    #[must_use]
    pub fn read(name: &[u8]) -> Option<Self> {
        Some(match name {
            b"Row" => Self::Row,
            b"Column" => Self::Column,
            b"Both" => Self::Both,
            _ => return None,
        })
    }

    /// Table 384's assumed value, for a `TH` that states no `/Scope`.
    ///
    /// ISO 32000-2 §14.8.5.7:
    ///
    /// > If a Scope is not specified for a TH structure element, then the assumed value for the
    /// > Scope shall be determined as follows, taking into account the current value for
    /// > WritingMode
    ///
    /// > if it is in the first row and column, the Scope is assumed to be Both
    ///
    /// > otherwise, if it is in the first row, the Scope is assumed to be Column
    ///
    /// > otherwise, if it is in the first column, the Scope is assumed to be Row
    ///
    /// > otherwise, the Scope is assumed to be Both
    ///
    /// **The row and the column here are the *logical* ones — the table's own `TR` order and the
    /// cell's place in the grid — and that is the standard's reading rather than this crate's
    /// convenience.** §14.8.4.8.3's NOTE, on the header search these assumptions feed, says
    ///
    /// > This algorithm works for languages with different intrinsic directionality of the script
    /// > (such as right-to-left) because the structure always reflects the logical content order
    /// > of the table.
    ///
    /// so `WritingMode` decides where the first row and column are *drawn*, and the structure
    /// decides which they are. This reader has the structure, which is why it can answer without
    /// reading §14.8.5.4's layout attributes at all.
    #[must_use]
    pub fn assumed(row: usize, column: usize) -> Self {
        match (row, column) {
            (0, 0) => Self::Both,
            (0, _) => Self::Column,
            (_, 0) => Self::Row,
            _ => Self::Both,
        }
    }
}

/// How many columns of one table this reader places cells in.
///
/// A bound on a `/ColSpan` a document controls, and it cannot change an answer: [`HeaderScope`]'s
/// assumption turns on whether a cell is in the *first* column, and a cell placed at this bound is
/// not in the first column however far past it the document put it. Sixty-four times this many
/// `usize` is the worst a [`MAX_DEPTH`]-deep nest of tables can hold at once.
const MAX_TABLE_COLUMNS: usize = 4096;

/// Where one `TH` or `TD` sits in its table, and how far it reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellPlacement {
    /// The row, counting the table's `TR` elements from zero across `THead`, `TBody` and `TFoot`.
    pub row: usize,
    /// The first column the cell occupies, counting from zero.
    pub column: usize,
    /// Table 384's `/RowSpan`, at least 1.
    pub row_span: usize,
    /// Table 384's `/ColSpan`, at least 1.
    pub column_span: usize,
}

/// One table's grid, filled a row at a time as §14.7's tree is walked.
///
/// **A cell's column is not its position among its row's children**, which is the whole reason
/// this type exists. Table 384's `/RowSpan` lets a cell in an earlier row occupy a column of this
/// one — ISO 32000-2 §14.8.5.7 —
///
/// > The number of rows in the enclosing table that shall be spanned by the cell.
///
/// — so the second row of a table whose first cell spans two rows begins at column 1, and a
/// reader that counted children would call that cell the first column's and give it
/// [`HeaderScope::Row`] where §14.8.5.7 assumes [`HeaderScope::Both`].
///
/// The grid is filled in the order the tree is walked, which is the order the standard states the
/// table in: `begin_row` at each `TR`, `place` at each `TH` or `TD`. [`TableStack`] is what drives
/// it, and is what a consumer uses: a table may contain a table, so one grid is not enough.
#[derive(Debug, Clone, Default)]
struct TableGrid {
    /// The row being filled, or `None` before the first `TR`.
    row: Option<usize>,
    /// The column the next cell of this row is tried at, before spill is considered.
    next: usize,
    /// For each column, the first row in which it is free again.
    free_from: Vec<usize>,
}

impl TableGrid {
    /// Begins the next `TR` of this table.
    fn begin_row(&mut self) {
        self.row = Some(match self.row {
            None => 0,
            Some(row) => row.saturating_add(1),
        });
        self.next = 0;
    }

    /// Places the next cell of the row being filled, spanning as Table 384 says it does.
    ///
    /// `None` before any row has begun, which is a `TH` or `TD` outside a `TR`. Nothing is
    /// invented for it: the position is the one thing §14.8.5.7's assumption turns on, and a
    /// guess there would be this reader's opinion about which axis a header describes.
    fn place(&mut self, row_span: usize, column_span: usize) -> Option<CellPlacement> {
        let row = self.row?;
        let row_span = row_span.max(1);
        let column_span = column_span.max(1);
        let mut column = self.next;
        while column < MAX_TABLE_COLUMNS
            && self
                .free_from
                .get(column)
                .is_some_and(|free_from| *free_from > row)
        {
            column = column.saturating_add(1);
        }
        let end = column
            .saturating_add(column_span)
            .min(MAX_TABLE_COLUMNS)
            .max(column);
        if self.free_from.len() < end {
            self.free_from.resize(end, 0);
        }
        for slot in self.free_from.get_mut(column..end).into_iter().flatten() {
            *slot = row.saturating_add(row_span);
        }
        self.next = end;
        Some(CellPlacement {
            row,
            column,
            row_span,
            column_span,
        })
    }
}

/// Everything one `TH` or `TD` states about itself that its place in the grid cannot say.
///
/// [`Tree::cell_facts`] reads it; [`TableStack`] is what it is read for.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CellFacts {
    /// Table 384's `/RowSpan`, at least 1.
    pub row_span: usize,
    /// Table 384's `/ColSpan`, at least 1.
    pub column_span: usize,
    /// Table 384's `/Scope`, where the cell states one.
    ///
    /// `None` leaves §14.8.5.7's assumption to be made from the cell's place in the grid, which
    /// [`TableStack`] has and the caller does not.
    pub scope: Option<HeaderScope>,
    /// Table 355's `/ID`, where the element states one, which is what `/Headers` names cells by.
    pub id: Option<Vec<u8>>,
    /// Table 384's `/Headers`, unresolved, in the document's order.
    pub headers: Option<Vec<Vec<u8>>>,
}

/// How many grid entries one walk's tables hold between them before recording stops.
///
/// One entry per column a cell occupies, so a table of ordinary cells costs one apiece and a
/// `/ColSpan` of a thousand costs a thousand — which is what makes this a bound on a `/ColSpan` a
/// document controls rather than on a table anybody writes. `examples/cell_header_census` prints
/// [`TableStack::truncated`] over 1251 files and 21 883 cells, and has never seen it true. At this
/// crate's sizes the bound is about 6 MB of intervals.
///
/// **A cell past the bound is still placed**, so §14.8.5.7's assumed `/Scope` is unaffected; what
/// it loses is its part in §14.8.4.8.3's search, and [`TableStack::truncated`] says so rather than
/// leaving a shorter list of headers looking like a document that states fewer.
const MAX_TABLE_GRID: usize = 1 << 18;

/// One cell of one table, as the search reads it back.
#[derive(Debug, Clone)]
struct PlacedCell {
    /// What the caller called this cell — its own index, and what [`TableStack::headers`] answers.
    token: usize,
    /// Which of [`TableStack::tables`] the cell belongs to.
    table: usize,
    /// Where it sits.
    place: CellPlacement,
    /// Whether it is a `TH` rather than a `TD`.
    header: bool,
    /// Table 384's `/Scope`, stated or assumed — which is what the search filters on.
    scope: HeaderScope,
    /// Table 384's `/Headers`, unresolved.
    stated: Option<Vec<Vec<u8>>>,
}

/// The rows each column of one table has been occupied by, in increasing row order.
///
/// One list per column, each holding `[row, row + RowSpan)` and the cell that fills it. The
/// intervals of a column cannot overlap — [`TableGrid::place`] never puts a cell where one still
/// reaches — and they arrive in increasing row order because the walk fills the table row by row,
/// which is what makes [`TableStack::covering`] a binary search rather than a scan.
///
/// **A column's list, rather than a row's**, and the asymmetry is the memory: a cell is recorded
/// once per column it spans and its `/RowSpan` costs nothing, so a table of ordinary cells costs
/// one entry each.
#[derive(Debug, Clone, Default)]
struct TableColumns {
    /// The grid that places cells, which is the same one [`HeaderScope`]'s assumption needs.
    grid: TableGrid,
    /// Per column, the intervals of rows that are occupied and by which cell.
    occupied: Vec<Vec<(usize, usize, usize)>>,
}

/// The tables a walk of §14.7's tree is currently inside, and everything they placed.
///
/// §14.8.4.8.3 puts no bar on a `Table` inside a `TD`, so one grid is not enough and the enclosing
/// table's rows continue where the inner one's finish. The stack is keyed by the walk's own depth,
/// which is what both consumers of this type already have: a depth-first [`Tree::walk`] carries
/// one, and a recursive walk is at one.
///
/// It is deliberately *driven* rather than computed. Both callers walk the tree for their own
/// reasons — a census over a corpus, and `viewer_core`'s answer for one page — and neither should
/// walk it a second time to learn where its cells are.
///
/// # Why the headers are answered afterwards
///
/// [`Self::enter`] answers a cell's *place* immediately, because §14.8.5.7's assumption needs
/// nothing but the grid so far. [`Self::headers`] cannot: Table 384's `/Headers` names cells by
/// `/ID`, and nothing in the standard makes the cell it names one the walk has already reached.
/// So the identifiers are kept unresolved until the walk is over and the whole table is known —
/// which is also what lets the entry's own recursion terminate.
#[derive(Debug, Clone, Default)]
pub struct TableStack {
    /// Every table the walk has entered, in the order it entered them.
    tables: Vec<TableColumns>,
    /// The depth each open table was entered at, and which of [`Self::tables`] it is.
    ///
    /// One entry per depth, because [`Self::enter`] closes every table at or below the depth it
    /// is given before opening another, so this is bounded by whatever bounds the walk's depth.
    open: Vec<(usize, usize)>,
    /// Every cell the walk has placed, in walk order.
    cells: Vec<PlacedCell>,
    /// Which cell each stated `/ID` belongs to, for resolving `/Headers`.
    ///
    /// Cells only: Table 384 makes a `/Headers` entry "the element identifier … for a TH
    /// structure element", so an identifier naming anything else names nothing this answers with.
    ///
    /// **Not through Table 354's `/IDTree`**, which [`Tree::element_by_id`] reads and which would
    /// answer with a dictionary — and a dictionary is not something a caller can point at, because
    /// what this type answers in is the caller's own token for the element. It also means a
    /// document that states identifiers without the index Table 354 requires beside them is still
    /// read. All 475 of the corpus's `/Headers` entries resolve either way.
    by_id: BTreeMap<Vec<u8>, usize>,
    /// How much of [`MAX_TABLE_GRID`] has been spent.
    entries: usize,
    /// Whether that bound was reached, which is [`Self::truncated`].
    truncated: bool,
}

impl TableStack {
    /// A stack inside no table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether `MAX_TABLE_GRID` stopped this walk recording cells.
    ///
    /// `true` means [`Self::headers`] answers for a *part* of the document's tables, which is a
    /// different statement from a document whose cells state no headers. Reported rather than
    /// applied in silence, for [`Reading::truncated`]'s reason.
    #[must_use]
    pub fn truncated(&self) -> bool {
        self.truncated
    }

    /// Which table this walk is currently inside, counting the tables it has entered from zero.
    ///
    /// The identity [`CellPlacement`] deliberately lacks: a row and a column mean nothing without
    /// the grid they are in, and §14.8.4.8.3 puts no bar on a `Table` inside a `TD`, so two cells
    /// of one page may both be at row 0 column 0 and be in different tables. A caller grouping
    /// cells — §14.8.5.4.5's row and column adjustment is the one that needs to — asks this
    /// immediately after [`Self::enter`] answered with a placement.
    ///
    /// `None` outside every table, which is what a `TH` or `TD` a document wrote outside one gets.
    #[must_use]
    pub fn innermost_table(&self) -> Option<usize> {
        self.open.last().map(|(_, table)| *table)
    }

    /// Enters one structure element, and answers with its place in its table where it is a cell.
    ///
    /// `kind` is the element's §14.8.4 type **after §14.7.3's role mapping**, because a document's
    /// own name for a table cell is a table cell; [`Tree::standard_role`] is that reading. `None`
    /// — a type §14.8.4 does not define — is neither a table nor a cell and only closes what the
    /// walk has left.
    ///
    /// `token` is whatever the caller calls this element — its index in the caller's own list —
    /// and is what [`Self::headers`] answers in. It is read only for a cell.
    ///
    /// `facts` is asked for **only where the element turns out to be a cell**, which is why it is
    /// a closure rather than a value: [`Tree::cell_facts`] reads the element's attribute objects
    /// and its class map, and a tagged document's elements are overwhelmingly paragraphs and
    /// spans.
    pub fn enter(
        &mut self,
        depth: usize,
        kind: Option<&StandardType>,
        token: usize,
        facts: impl FnOnce() -> CellFacts,
    ) -> Option<CellPlacement> {
        while self.open.last().is_some_and(|(at, _)| *at >= depth) {
            self.open.pop();
        }
        match kind {
            Some(StandardType::Table) => {
                let table = self.tables.len();
                self.tables.push(TableColumns::default());
                self.open.push((depth, table));
                None
            }
            Some(StandardType::TableRow) => {
                if let Some(table) = self.open.last().map(|(_, table)| *table)
                    && let Some(columns) = self.tables.get_mut(table)
                {
                    columns.grid.begin_row();
                }
                None
            }
            Some(kind @ (StandardType::TableHeader | StandardType::TableData)) => {
                let facts = facts();
                let table = self.open.last().map(|(_, table)| *table)?;
                let columns = self.tables.get_mut(table)?;
                let place = columns.grid.place(facts.row_span, facts.column_span)?;
                self.record(
                    table,
                    token,
                    *kind == StandardType::TableHeader,
                    place,
                    facts,
                );
                Some(place)
            }
            _ => None,
        }
    }

    /// Keeps one placed cell, and the columns it occupies, for the search to read back.
    fn record(
        &mut self,
        table: usize,
        token: usize,
        header: bool,
        place: CellPlacement,
        facts: CellFacts,
    ) {
        // The same clamp [`TableGrid::place`] applied when it filled the grid: a `/ColSpan` past
        // [`MAX_TABLE_COLUMNS`] occupies the columns this reader has and no more, and recording
        // more of them than the grid knows about would put a cell where nothing can find it.
        let end = place
            .column
            .saturating_add(place.column_span)
            .min(MAX_TABLE_COLUMNS)
            .max(place.column);
        let width = end.saturating_sub(place.column);
        if self.entries.saturating_add(width) > MAX_TABLE_GRID {
            self.truncated = true;
            return;
        }
        self.entries = self.entries.saturating_add(width);
        let cell = self.cells.len();
        let rows = (place.row, place.row.saturating_add(place.row_span));
        if let Some(columns) = self.tables.get_mut(table) {
            if columns.occupied.len() < end {
                columns.occupied.resize(end, Vec::new());
            }
            for column in columns
                .occupied
                .get_mut(place.column..end)
                .into_iter()
                .flatten()
            {
                column.push((rows.0, rows.1, cell));
            }
        }
        if let Some(id) = facts.id {
            self.by_id.entry(id).or_insert(cell);
        }
        self.cells.push(PlacedCell {
            token,
            table,
            place,
            header,
            // §14.8.5.7's assumption is what a cell stating nothing gets, and it is about the
            // cell's place in the grid rather than its ordinal among its row's children.
            scope: facts
                .scope
                .unwrap_or_else(|| HeaderScope::assumed(place.row, place.column)),
            stated: facts.headers,
        });
    }

    /// §14.8.4.8.3's headers, for every cell this walk placed that has any.
    ///
    /// One entry per cell, in walk order, as `(the cell's token, the header cells' tokens)`; a
    /// cell whose headers come to nothing is not listed at all. Within an entry the order is
    /// §14.8.5.7's Table 384's: the entries are listed row IDs first and then column IDs, each
    /// group ordered from most specific to most general — which is the order the search itself
    /// produces, because it starts at the cell and walks outward.
    ///
    /// That is prose rather than a blockquote for the reason `doc/HANDOVER.md` gives about
    /// `doc/md/`: the conversion writes the sentence's "IDs" as "ID s", so no quotation of it can
    /// be verbatim against the file the checker reads. `pdftotext -layout` over
    /// `doc/ISO_32000-2_sponsored_EC3.pdf` says "row IDs", and the standard is what it says.
    ///
    /// Two routes, and §14.8.4.8.3 chooses between them rather than combining them:
    ///
    /// > If the Headers attribute (see 14.8.5, "Standard structure attributes") is not specified,
    /// > any cell in a table may have multiple headers associated with it. These headers are
    /// > defined either explicitly by the Headers attribute, or implicitly, by the following
    /// > algorithm
    ///
    /// A cell that states the attribute is answered from it, expanded by Table 384's own
    /// recursion — "the headers associated with any cell shall be those in its Headers array plus
    /// those in the Headers array of any TH cells in that array, and so on recursively" — and a
    /// cell that states none is answered by `Self::search`.
    ///
    /// **A header found twice is answered once.** Two searches build one list and a cell spanning
    /// both axes can be met by both of them, and the standard says nothing about the case; naming
    /// the same header twice would have a reader announce it twice, which is a choice this crate
    /// makes rather than a rule it read. It is also what makes the recursion above terminate.
    #[must_use]
    pub fn headers(&self) -> Vec<(usize, Vec<usize>)> {
        let mut out = Vec::new();
        for (index, cell) in self.cells.iter().enumerate() {
            let mut found = Vec::new();
            let mut seen = BTreeSet::new();
            seen.insert(index);
            if let Some(ids) = &cell.stated {
                self.resolve(ids, &mut seen, &mut found);
            } else {
                self.search(index, HeaderScope::Row, &mut seen, &mut found);
                self.search(index, HeaderScope::Column, &mut seen, &mut found);
            }
            if !found.is_empty() {
                out.push((
                    cell.token,
                    found
                        .into_iter()
                        .filter_map(|cell| self.cells.get(cell).map(|cell| cell.token))
                        .collect(),
                ));
            }
        }
        out
    }

    /// Table 384's `/Headers`, resolved and expanded by the entry's own recursion.
    ///
    /// Breadth-first, because the recursion appends the *named* cells' headers after the named
    /// cells themselves and the entry's order is "from most specific to most general". `seen`
    /// carries the cells already named, which is both the dedup rule [`Self::headers`] states and
    /// what terminates a document whose cells name each other.
    fn resolve(&self, ids: &[Vec<u8>], seen: &mut BTreeSet<usize>, out: &mut Vec<usize>) {
        let mut frontier: Vec<usize> = Vec::new();
        for id in ids {
            let Some(&cell) = self.by_id.get(id) else {
                continue;
            };
            if seen.insert(cell) {
                out.push(cell);
                frontier.push(cell);
            }
        }
        for _ in 0..MAX_DEPTH {
            if frontier.is_empty() {
                return;
            }
            let mut next = Vec::new();
            for named in frontier.drain(..) {
                let Some(ids) = self.cells.get(named).and_then(|cell| cell.stated.as_ref()) else {
                    continue;
                };
                for id in ids {
                    let Some(&cell) = self.by_id.get(id) else {
                        continue;
                    };
                    if seen.insert(cell) {
                        out.push(cell);
                        next.push(cell);
                    }
                }
            }
            frontier = next;
        }
    }

    /// §14.8.4.8.3's search along one axis, from a cell towards the first cell of its table.
    ///
    /// ISO 32000-2 §14.8.4.8.3:
    ///
    /// > To find headers for any data or header cell, begin from the current cell position and use
    /// > the current value of WritingMode to search towards the first cell in the appropriate
    /// > horizontal/vertical direction. The search terminates when any of these conditions is
    /// > reached:
    ///
    /// > the edge of the table is reached
    ///
    /// > a data cell is found after a header cell
    ///
    /// > a header cell has the Headers attribute set -the headers that are specified are appended
    /// > to the row/ column list that is being built
    ///
    /// > When a header cell is found in the search and the (implicit or explicit) Scope attribute
    /// > of the header cell is either Both or Row/Column , the header cell is appended to the end
    /// > of the list of row/column headers, resulting in a list of headers ordered from most
    /// > specific to most general.
    ///
    /// `axis` is which of the two lists is being built: [`HeaderScope::Row`] walks towards the
    /// first column of the cell's own row, [`HeaderScope::Column`] towards the first row of its
    /// own column. [`HeaderScope::Both`] is not an axis and searches nothing — it is what a
    /// *header* may be scoped to, never a direction.
    ///
    /// **The direction is the grid's rather than the page's**, which is the clause's own reading:
    /// its NOTE says the algorithm "works for languages with different intrinsic directionality of
    /// the script … because the structure always reflects the logical content order of the table",
    /// so `WritingMode` decides where the first cell is *drawn* and the structure decides which it
    /// is.
    fn search(
        &self,
        from: usize,
        axis: HeaderScope,
        seen: &mut BTreeSet<usize>,
        out: &mut Vec<usize>,
    ) {
        let Some(cell) = self.cells.get(from) else {
            return;
        };
        let (mut row, mut column) = (cell.place.row, cell.place.column);
        let mut after_a_header = false;
        loop {
            // "the edge of the table is reached" — which is column zero along a row and row zero
            // down a column, both of them exclusive because the cell itself is not its own header.
            match axis {
                HeaderScope::Row if column > 0 => column = column.saturating_sub(1),
                HeaderScope::Column if row > 0 => row = row.saturating_sub(1),
                _ => return,
            }
            let Some(found) = self.covering(cell.table, row, column) else {
                continue;
            };
            let Some(other) = self.cells.get(found) else {
                return;
            };
            // Step over the whole of the cell that was found rather than over one grid position,
            // so a cell spanning a hundred columns costs one step and is considered once.
            match axis {
                HeaderScope::Row => column = other.place.column,
                _ => row = other.place.row,
            }
            if !other.header {
                if after_a_header {
                    return;
                }
                continue;
            }
            after_a_header = true;
            // "either Both or Row/Column" — the axis being built, or a header that describes both.
            if (other.scope == axis || other.scope == HeaderScope::Both) && seen.insert(found) {
                out.push(found);
            }
            if let Some(ids) = &other.stated {
                self.resolve(ids, seen, out);
                return;
            }
        }
    }

    /// Which cell occupies one position of one table's grid.
    ///
    /// A binary search over the column's intervals, which are non-overlapping and in increasing
    /// row order; `None` for a position no cell reaches, which the search steps over because a gap
    /// in a table is neither a data cell nor a header cell.
    fn covering(&self, table: usize, row: usize, column: usize) -> Option<usize> {
        let intervals = self.tables.get(table)?.occupied.get(column)?;
        let at = intervals.partition_point(|(start, _, _)| *start <= row);
        let &(start, end, cell) = intervals.get(at.checked_sub(1)?)?;
        (start <= row && row < end).then_some(cell)
    }
}

/// §14.7.1's mark information dictionary. Table 353.
///
/// Three booleans in the catalog's `/MarkInfo`, and the first is what §14.8.1 turns on: "\[a]
/// tagged PDF document shall contain a mark information dictionary … with a value of true for
/// the Marked entry." So this is how a document *says* it is tagged, as against having a
/// structure tree — which a document may have without claiming to follow §14.8's conventions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MarkInfo {
    /// `/Marked`: "whether the document conforms to tagged PDF conventions". Default false.
    ///
    /// The entry's own sentence bounds what the claim is worth: "\[i]f `Suspects` is true, the
    /// document may not completely conform to tagged PDF conventions."
    pub marked: bool,
    /// `/UserProperties`: whether any structure element carries §14.7.6.4's user properties.
    ///
    /// A hint rather than a fact a reader needs: the clause says it "allow\[s] PDF processors to
    /// quickly determine whether it is necessary to search the structure tree for elements
    /// containing user properties", and `Tree::attributes` finds them either way.
    pub user_properties: bool,
    /// `/Suspects`, **deprecated in PDF 2.0**: whether the document holds tag suspects.
    pub suspects: bool,
}

impl MarkInfo {
    /// Reads the catalog's `/MarkInfo`, or Table 353's defaults where there is none.
    #[must_use]
    pub fn read(document: &Document) -> Self {
        let Ok(catalog) = document.catalog() else {
            return Self::default();
        };
        let info = document.get_key(&catalog, "MarkInfo");
        let Some(info) = info.as_dict() else {
            return Self::default();
        };
        let flag = |key: &str| matches!(document.get_key(info, key), Object::Boolean(true));
        Self {
            marked: flag("Marked"),
            user_properties: flag("UserProperties"),
            suspects: flag("Suspects"),
        }
    }
}

/// §14.8.4's standard structure types: the vocabulary a tagged document's `/S` names.
///
/// §14.8.4.1 makes the vocabulary closed for a tagged document — "\[a]ll structure elements
/// occurring within a tagged PDF document shall have a type matching one of those defined as a
/// Standard Structure Type, or a role map providing a mapping from the non-standard type to a
/// Standard Structure Type" — which is why [`Tree::role`] exists and why this enum is what its
/// answer *means*.
///
/// The four categories §14.8.4.1 divides them into are [`Category`], and three types are in more
/// than one: see [`Self::category`].
///
/// The last eleven are the PDF 1.7 namespace's alone, which Annex M lists and §14.8.6.1 makes the
/// default: which namespace defines which type is [`Self::defined_in`] (ADR 2014).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StandardType {
    /// `Document`: "\[e]ncloses a logical document".
    Document,
    /// `DocumentFragment` (PDF 2.0): "\[e]ncloses a logical document fragment".
    DocumentFragment,
    /// `Part`: "a grouping of structure elements without consideration for their hierarchy".
    Part,
    /// `Sect`: "a grouping of structure elements with consideration for their hierarchy".
    Section,
    /// `Div`: content "structured in fashion that is orthogonal to the semantic structure".
    Division,
    /// `Aside` (PDF 2.0): content "distinct from other content within its parent".
    Aside,
    /// `NonStruct`: "\[a] grouping element having no inherent structural significance".
    NonStructural,
    /// `P`: "\[a] low-level division of content", usually a paragraph.
    Paragraph,
    /// `Hn`: a heading at a stated level.
    ///
    /// The table writes it as `H n`, "with n being a sequence of digits representing an unsigned
    /// integer greater than or equal to 1" — so the level is part of the *name* rather than an
    /// attribute, and `H7` is as legal as `H1`.
    Heading(u32),
    /// `H`: a heading whose level "is not indicated" by the type, and is its nesting instead.
    UnnumberedHeading,
    /// `Title` (PDF 2.0): "the title of a document or high-level division of content".
    Title,
    /// `FENote` (PDF 2.0): a footnote or endnote.
    FootnoteOrEndnote,
    /// `Sub` (PDF 2.0): "a sub-division inside a block level" element.
    SubBlock,
    /// `Lbl`: a label, "content that distinguishes it from other content inside the same parent".
    Label,
    /// `Span`: "\[a] generic inline portion of content having no particular inherent
    /// characteristics".
    Span,
    /// `Em` (PDF 2.0): content emphasised.
    Emphasis,
    /// `Strong` (PDF 2.0): content of "strong importance".
    Strong,
    /// `Link`: "\[a]n association between content enclosed by the Link structure element and a
    /// corresponding link annotation" (§12.5.6.5), whose own subclause is §14.8.4.7.3.
    Link,
    /// `Annot`: "\[e]ncloses one or more PDF annotations and associated content, if any".
    ///
    /// Errata Collection 3 Issue #437 rewrites this row and `Form`'s away from *association*: the
    /// annotation is what the element is for and the content is the optional part, which is the
    /// reverse of what an association reads as (ADR 0273).
    Annotation,
    /// `Form`: "\[e]ncloses a PDF widget annotation and associated content, if any" (§12.7).
    Form,
    /// `Ruby`: the wrapper "around an entire ruby assembly".
    Ruby,
    /// `RB`: ruby base text, "\[t]he full-size text to which the ruby annotation is applied".
    RubyBase,
    /// `RT`: ruby annotation text, the smaller text placed beside the base.
    RubyText,
    /// `RP`: ruby punctuation, used only where a processor cannot place the annotation.
    RubyPunctuation,
    /// `Warichu`: the wrapper around a warichu assembly.
    Warichu,
    /// `WT`: warichu text, "formatted into two lines".
    WarichuText,
    /// `WP`: the punctuation surrounding it.
    WarichuPunctuation,
    /// `L`: a list.
    List,
    /// `LI`: one member of a list, "\[i]nternal to L (List) structure elements".
    ListItem,
    /// `LBody`: "\[t]he actual content of a list item", internal to `LI`.
    ListItemBody,
    /// `Table`: "\[a] two-dimensional logical structure of cells".
    Table,
    /// `TR`: a table row.
    TableRow,
    /// `TH`: a header cell.
    TableHeader,
    /// `TD`: a data cell.
    TableData,
    /// `THead`: the rows constituting the head of a table.
    TableHead,
    /// `TBody`: the rows constituting its body.
    TableBody,
    /// `TFoot`: the rows constituting its foot.
    TableFoot,
    /// `Caption`: a caption for a list, a table, a figure or a formula.
    Caption,
    /// `Figure`: graphical content.
    Figure,
    /// `Formula`: a formula.
    Formula,
    /// `Artifact` (PDF 2.0): §14.8.2.2's artifact, "for which semantics require a reference in
    /// the structure tree" — the *element* form, as against the marked-content one this crate
    /// reads in [`Artifact`].
    Artifact,
    /// `Art` (PDF 1.7 namespace only): an article, a self-contained body of text making one
    /// narrative or exposition (ISO 32000-1 Table 333).
    Article,
    /// `BlockQuote` (PDF 1.7 namespace only): one or more paragraphs attributed to someone other
    /// than the author of the text around them (ISO 32000-1 Table 333).
    BlockQuote,
    /// `TOC` (PDF 1.7 namespace only): a table of contents, a list of `TOCI` entries and of nested
    /// `TOC`s (ISO 32000-1 Table 333).
    TableOfContents,
    /// `TOCI` (PDF 1.7 namespace only): one member of a table of contents (ISO 32000-1 Table 333).
    TableOfContentsItem,
    /// `Index` (PDF 1.7 namespace only): entries of identifying text with `Reference` elements to
    /// where the text occurs in the body (ISO 32000-1 Table 333).
    Index,
    /// `Private` (PDF 1.7 namespace only): content private to the application that produced it,
    /// which ISO 32000-1 Table 333 says shall not be interpreted or exported, nor shall any of its
    /// descendants.
    Private,
    /// `Quote` (PDF 1.7 namespace only): an inline portion of text attributed to someone other
    /// than the author of the text around it (ISO 32000-1 Table 338).
    Quote,
    /// `Note` (PDF 1.7 namespace only): explanatory text such as a footnote or an endnote, referred
    /// to from the body (ISO 32000-1 Table 338) — PDF 2.0's `FENote` is its successor.
    Note,
    /// `Reference` (PDF 1.7 namespace only): a citation to content elsewhere in the document (ISO
    /// 32000-1 Table 338).
    Reference,
    /// `BibEntry` (PDF 1.7 namespace only): a reference identifying the external source of cited
    /// content (ISO 32000-1 Table 338).
    BibliographyEntry,
    /// `Code` (PDF 1.7 namespace only): a fragment of computer program text (ISO 32000-1 Table
    /// 338).
    Code,
}

/// §14.8.6.1's two standard structure namespaces, each of which defines a vocabulary of
/// [`StandardType`]s.
///
/// Annex M lists how the two differ: eleven types PDF 1.7's namespace defines and PDF 2.0's does
/// not, and nine the other way round, counting `Hn` above `H6` as one. The rest are in both, "but
/// are often defined differently", and this reader reads each by the subclause of ISO 32000-2
/// that defines it — the PDF 1.7 namespace is ISO 32000-1's clause 14.8, which §14.8.6.1 names as
/// that namespace's schema, and only the eleven of its own are read from there (ADR 2014).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandardNamespace {
    /// [`DEFAULT_STANDARD_NAMESPACE`], the standard structure namespace for PDF 1.7, which an
    /// element stating no namespace is in.
    Pdf1_7,
    /// [`STANDARD_NAMESPACE_2_0`], the standard structure namespace for PDF 2.0.
    Pdf2_0,
}

/// An element's structure type after §14.7.3's and §14.8.6.2's role mapping, and what §14.8.4's
/// vocabulary makes of it: [`Tree::mapped_role`]'s answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappedRole {
    /// The name the role map ended at — what the document wrote, or what its own map took it
    /// to. Empty for an element stating no `/S`.
    pub name: String,
    /// The standard type that name is in the namespace it ends in, or why it is none.
    pub standard: Result<StandardType, Nonstandard>,
}

/// Why an element's type, after its role map, is not one of §14.8.4's standard types.
///
/// Four answers rather than one because a person told "this is not a standard type" is owed the
/// reason, and the reasons are statements about different things: the element, the namespace's
/// vocabulary, the namespace, and the role map (ADR 2028).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nonstandard {
    /// The element states no `/S`, which Table 355 makes required.
    Unstated,
    /// The name ends in a standard structure namespace, or in none, and that vocabulary does not
    /// define it: §14.8.4.1's requirement unmet by the document, since "\[a]ll structure elements
    /// occurring within a tagged PDF document shall have a type matching one of those defined as
    /// a Standard Structure Type, or a role map providing a mapping from the non-standard type to
    /// a Standard Structure Type".
    Undefined,
    /// The name ends in a namespace that is not one of §14.8.6.1's two, so whatever it is called
    /// it is not §14.8.4's type — a foreign vocabulary's `Table` is a homonym. A namespace
    /// dictionary stating no `/NS` of its own is this too: Table 356 makes the entry required, and
    /// the alternative is to take a broken namespace for a standard one.
    Foreign,
    /// The role map returned to a type it had already reached before it reached one a standard
    /// structure namespace defines — §14.7.3 NOTE 2's second stop with nothing its first would
    /// have recognised.
    Circular,
}

/// Where [`Tree::walk_role_map`] ended.
struct RoleMapEnd {
    /// The name the walk answers with.
    name: String,
    /// The namespace that name is in, `None` for the default one.
    namespace: Option<Dictionary>,
    /// Whether the walk closed a circle in which no type was a standard one.
    circular: bool,
}

/// What a name means in the namespace it is in: §14.8.4's type, or `None`.
///
/// An explicit namespace is read in its own vocabulary and held to, and no namespace is read in
/// PDF 1.7's — §14.8.6.1's default — with a PDF 2.0 type stating none read as PDF 2.0's by choice
/// (ADR 2014 section 3). A namespace that is not standard defines no standard type.
fn standard_in(
    document: &Document,
    name: &str,
    namespace: Option<&Dictionary>,
) -> Option<StandardType> {
    match namespace {
        Some(space) => StandardType::read_in(name, Namespace::read(document, space)?.standard()?),
        None => StandardType::read_in(name, StandardNamespace::Pdf1_7)
            .or_else(|| StandardType::read_in(name, StandardNamespace::Pdf2_0)),
    }
}

/// §14.8.4.1's four categories of standard structure type.
///
/// > Some structure types -for example Table or Figure -may be used as block level types or as
/// > inline types, whereas others (e.g., H1 ) may only be used as block level types.
///
/// So a type does not always have one category, and [`Category::Contextual`] is that case with
/// the clause's own rule attached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    /// "\[I]dentify a whole document or a fragment of a document."
    Document,
    /// "\[M]ake it possible to organise the overall structure of content."
    Grouping,
    /// "\[S]tructure types that enclose actual content, like a heading or paragraph."
    Block,
    /// "\[E]nable structural organisation of content inside block level elements."
    Inline,
    /// A type whose category depends on where it is used, resolved by [`Category::of`].
    Contextual,
    /// A type that is only ever inside one other: a list's `LI`, a table's `TR`.
    ///
    /// The clause writes these as "Internal to L (List) structure elements" and "Internal to a
    /// Table structure" rather than as one of the four levels, which is a different kind of
    /// statement — about *where* the element may appear rather than about what it may contain.
    Internal,
}

impl Category {
    /// §14.8.4.1's rule for a type that may be block or inline.
    ///
    /// > If the structure element is used inside a block level element, it is an inline level
    /// > structure element … In all other cases it is a block level structure element.
    ///
    /// `parent` is the category of the element this one sits in, which a caller walking the tree
    /// has and this crate does not keep.
    #[must_use]
    pub fn of(self, parent: Option<Self>) -> Self {
        match self {
            Self::Contextual if parent == Some(Self::Block) => Self::Inline,
            Self::Contextual => Self::Block,
            other => other,
        }
    }
}

impl StandardType {
    /// Reads a name either standard structure namespace defines, or `None` for one neither does.
    ///
    /// The reading for a caller holding a name and not the namespace it ended in — an
    /// accessibility host is handed the mapped name alone (ADR 0214). [`Tree::standard_role`] is
    /// the reading that knows the namespace, and [`Self::read_in`] is the one it asks (ADR 2014).
    ///
    /// `None` is a finding rather than a default: §14.8.4.1 requires every element of a tagged
    /// document to be a standard type or role mapped to one, so a name that arrives here
    /// unmapped is a document that has not done what the clause asks — which a consumer may want
    /// to say, and cannot if the reader has invented a type for it.
    #[must_use]
    pub fn read(name: &str) -> Option<Self> {
        Self::named(name)
    }

    /// Reads a name in one standard structure namespace, or `None` for one it does not define.
    ///
    /// §14.8.6.1 makes the vocabulary a namespace's: "\[t]his schema is called the standard
    /// structure namespace for PDF 2.0", and the PDF 1.7 one is ISO 32000-1's clause 14.8. So a
    /// `Note` is a type in the default namespace and a name nobody defined in PDF 2.0's, and an
    /// `Em` the other way round — Annex M's two lists, which [`Self::defined_in`] holds.
    #[must_use]
    pub fn read_in(name: &str, space: StandardNamespace) -> Option<Self> {
        Self::named(name).filter(|kind| kind.defined_in(space))
    }

    /// Whether a standard structure namespace defines this type.
    ///
    /// Annex M: the PDF 1.7 namespace lacks the nine of [`Self::since_pdf_2_0`], and the PDF 2.0
    /// one lacks the eleven "defined in the standard structure namespace for PDF 1.7, but not in
    /// the standard structure namespace for PDF 2.0" — `Art`, `BlockQuote`, `TOC`, `TOCI`,
    /// `Index`, `Private`, `Quote`, `Note`, `Reference`, `BibEntry` and `Code`.
    #[must_use]
    pub fn defined_in(&self, space: StandardNamespace) -> bool {
        match space {
            StandardNamespace::Pdf1_7 => !self.since_pdf_2_0(),
            StandardNamespace::Pdf2_0 => !matches!(
                self,
                Self::Article
                    | Self::BlockQuote
                    | Self::TableOfContents
                    | Self::TableOfContentsItem
                    | Self::Index
                    | Self::Private
                    | Self::Quote
                    | Self::Note
                    | Self::Reference
                    | Self::BibliographyEntry
                    | Self::Code
            ),
        }
    }

    /// Every name of both vocabularies, before either namespace is asked about it.
    fn named(name: &str) -> Option<Self> {
        // `Hn` before the table, because the table cannot hold an unbounded family: the clause
        // makes `n` "a sequence of digits representing an unsigned integer greater than or equal
        // to 1", so `H2` and `H17` are both standard and neither is a name anybody enumerated.
        if let Some(level) = name.strip_prefix('H')
            && !level.is_empty()
            && level.bytes().all(|byte| byte.is_ascii_digit())
        {
            return level
                .parse()
                .ok()
                .filter(|level| *level >= 1)
                .map(Self::Heading);
        }
        Some(match name {
            "Document" => Self::Document,
            "DocumentFragment" => Self::DocumentFragment,
            "Part" => Self::Part,
            "Sect" => Self::Section,
            "Div" => Self::Division,
            "Aside" => Self::Aside,
            "NonStruct" => Self::NonStructural,
            "P" => Self::Paragraph,
            "H" => Self::UnnumberedHeading,
            "Title" => Self::Title,
            "FENote" => Self::FootnoteOrEndnote,
            "Sub" => Self::SubBlock,
            "Lbl" => Self::Label,
            "Span" => Self::Span,
            "Em" => Self::Emphasis,
            "Strong" => Self::Strong,
            "Link" => Self::Link,
            "Annot" => Self::Annotation,
            "Form" => Self::Form,
            "Ruby" => Self::Ruby,
            "RB" => Self::RubyBase,
            "RT" => Self::RubyText,
            "RP" => Self::RubyPunctuation,
            "Warichu" => Self::Warichu,
            "WT" => Self::WarichuText,
            "WP" => Self::WarichuPunctuation,
            "L" => Self::List,
            "LI" => Self::ListItem,
            "LBody" => Self::ListItemBody,
            "Table" => Self::Table,
            "TR" => Self::TableRow,
            "TH" => Self::TableHeader,
            "TD" => Self::TableData,
            "THead" => Self::TableHead,
            "TBody" => Self::TableBody,
            "TFoot" => Self::TableFoot,
            "Caption" => Self::Caption,
            "Figure" => Self::Figure,
            "Formula" => Self::Formula,
            "Artifact" => Self::Artifact,
            "Art" => Self::Article,
            "BlockQuote" => Self::BlockQuote,
            "TOC" => Self::TableOfContents,
            "TOCI" => Self::TableOfContentsItem,
            "Index" => Self::Index,
            "Private" => Self::Private,
            "Quote" => Self::Quote,
            "Note" => Self::Note,
            "Reference" => Self::Reference,
            "BibEntry" => Self::BibliographyEntry,
            "Code" => Self::Code,
            _ => return None,
        })
    }

    /// The category §14.8.4 puts this type in.
    ///
    /// [`Category::Contextual`] where the clause gives two or three — `Figure`, `Formula`,
    /// `Link`, `Annot`, `Form`, `Title`, `FENote`, `Caption`, `L` and `Artifact` — and
    /// [`Category::of`] is the rule that settles it.
    ///
    /// The eleven types of the PDF 1.7 namespace alone take the category ISO 32000-1's clause
    /// 14.8.4 gives them, which is that namespace's schema: grouping for the six of its Table 333
    /// and inline for the five of its Table 338.
    #[must_use]
    pub fn category(&self) -> Category {
        match self {
            Self::Document | Self::DocumentFragment => Category::Document,
            Self::Part | Self::Section | Self::Division | Self::Aside | Self::NonStructural => {
                Category::Grouping
            }
            Self::Article
            | Self::BlockQuote
            | Self::TableOfContents
            | Self::TableOfContentsItem
            | Self::Index
            | Self::Private => Category::Grouping,
            Self::Quote | Self::Note | Self::Reference | Self::BibliographyEntry | Self::Code => {
                Category::Inline
            }
            Self::Paragraph | Self::Heading(_) | Self::UnnumberedHeading | Self::Table => {
                Category::Block
            }
            Self::SubBlock
            | Self::Label
            | Self::Span
            | Self::Emphasis
            | Self::Strong
            | Self::Ruby
            | Self::RubyBase
            | Self::RubyText
            | Self::RubyPunctuation
            | Self::Warichu
            | Self::WarichuText
            | Self::WarichuPunctuation => Category::Inline,
            Self::ListItem
            | Self::ListItemBody
            | Self::TableRow
            | Self::TableHeader
            | Self::TableData
            | Self::TableHead
            | Self::TableBody
            | Self::TableFoot => Category::Internal,
            Self::Title
            | Self::FootnoteOrEndnote
            | Self::Link
            | Self::Annotation
            | Self::Form
            | Self::List
            | Self::Caption
            | Self::Figure
            | Self::Formula
            | Self::Artifact => Category::Contextual,
        }
    }

    /// Whether this type was introduced in PDF 2.0, which is Annex M's list of the types "only
    /// defined in the standard structure namespace for PDF 2.0".
    ///
    /// Which matters for §14.8.6.1's two standard namespaces: an element in the *default*
    /// namespace — `http://iso.org/pdf/ssn`, PDF 1.7's — cannot mean one of these by it, which is
    /// [`Self::defined_in`]'s answer and [`Tree::standard_role`]'s. A heading is one of them past
    /// level six, which the list writes as `H n` with `n` above 6: ISO 32000-1's Table 334 stops
    /// at `H6`.
    #[must_use]
    pub fn since_pdf_2_0(&self) -> bool {
        match self {
            Self::Heading(level) => *level > 6,
            other => matches!(
                other,
                Self::DocumentFragment
                    | Self::Aside
                    | Self::Title
                    | Self::FootnoteOrEndnote
                    | Self::SubBlock
                    | Self::Emphasis
                    | Self::Strong
                    | Self::Artifact
            ),
        }
    }
}

/// Table 378's `/WritingMode`: which way §14.8.3.3's two progressions run.
///
/// The eight names the cell defines, spelled as it spells them. What each decides is the
/// *meaning of an edge*, and §14.8.3.3 says why the standard needs a vocabulary for it at all:
///
/// > Because the progression directions can vary depending on the writing system, edges of areas
/// > and directions on the page are identified by terms that are neutral with respect to the
/// > progression order rather than by familiar terms such as up, down, left, and right. Block
/// > layout proceeds from before to after, inline from start to end.
///
/// Only the block half is read here, because only the block half has a consumer: §14.8.5.4.5
/// adjusts an allocation rectangle's before and after edges and leaves the start and end ones
/// alone. The inline half is carried in the variant names and nothing asks for it.
///
/// **Not §9.7's vertical writing**, and the two must not be confused: `/WM` in a `CIDFont` places
/// glyphs from §9.2.4's metrics, and this places blocks. A document may state either without the
/// other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WritingMode {
    /// `LrTb`: "\[i]nline progression from left to right; block progression from top to bottom."
    ///
    /// The default, and "the typical writing mode for Western writing systems".
    #[default]
    LrTb,
    /// `RlTb`: inline right to left, block top to bottom — "Arabic and Hebrew writing systems".
    RlTb,
    /// `TbRl`: inline top to bottom, block right to left — "Chinese and Japanese writing systems".
    TbRl,
    /// `TbLr`: inline top to bottom, block left to right — "writing systems like classical
    /// Mongolian".
    TbLr,
    /// `LrBt`: inline left to right, block bottom to top.
    ///
    /// "There is currently no known writing system to which this writing mode applies", which is
    /// the cell's own sentence and not a reason to leave the name unread: a document may state it.
    LrBt,
    /// `RlBt`: inline right to left, block bottom to top, and the cell says the same of it.
    RlBt,
    /// `BtRl`: inline bottom to top, block right to left — "the Ancient Berber writing system".
    BtRl,
    /// `BtLr`: inline bottom to top, block left to right — "the Batak writing system".
    BtLr,
}

impl WritingMode {
    /// Reads one of Table 378's eight names, or `None` for anything else.
    #[must_use]
    pub fn read(name: &[u8]) -> Option<Self> {
        match name {
            b"LrTb" => Some(Self::LrTb),
            b"RlTb" => Some(Self::RlTb),
            b"TbRl" => Some(Self::TbRl),
            b"TbLr" => Some(Self::TbLr),
            b"LrBt" => Some(Self::LrBt),
            b"RlBt" => Some(Self::RlBt),
            b"BtRl" => Some(Self::BtRl),
            b"BtLr" => Some(Self::BtLr),
            _ => None,
        }
    }

    /// Which way blocks stack under this mode — the second half of each name.
    #[must_use]
    pub fn block_progression(self) -> BlockProgression {
        match self {
            Self::LrTb | Self::RlTb => BlockProgression::TopToBottom,
            Self::LrBt | Self::RlBt => BlockProgression::BottomToTop,
            Self::TbRl | Self::BtRl => BlockProgression::RightToLeft,
            Self::TbLr | Self::BtLr => BlockProgression::LeftToRight,
        }
    }
}

/// The direction §14.8.3.3 stacks block-level elements in, and so where *before* and *after* are.
///
/// > BLSEs shall be stacked within a reference area in block-progression order. In general, the
/// > first BLSE shall be placed against the before edge of the reference area.
///
/// Named after the page rather than after the progression — "top to bottom" rather than
/// "vertical" — because the whole point of the term is that a consumer eventually has to turn it
/// back into an edge of a rectangle, and this is where that happens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockProgression {
    /// Before is the top edge, after the bottom: `LrTb` and `RlTb`.
    TopToBottom,
    /// Before is the bottom edge, after the top: `LrBt` and `RlBt`.
    BottomToTop,
    /// Before is the right edge, after the left: `TbRl` and `BtRl`.
    RightToLeft,
    /// Before is the left edge, after the right: `TbLr` and `BtLr`.
    LeftToRight,
}

impl BlockProgression {
    /// Whether blocks stack along the page's vertical axis, which is where a *height* is measured.
    ///
    /// Table 379 measures `/Height` "in the block-progression direction" and `/Width` "in the
    /// inline-progression direction", and those two directions are perpendicular in every one of
    /// Table 378's eight modes — so this one answer names both axes: a height is the page's y
    /// extent and a width its x extent where this is `true`, and the other way round where it is
    /// not. A `TbRl` table's rows run down the page and stack across it, so the same two words
    /// mean the other pair of edges.
    #[must_use]
    pub fn stacks_vertically(self) -> bool {
        matches!(self, Self::TopToBottom | Self::BottomToTop)
    }
}

/// Table 379's `/SpaceBefore` and `/SpaceAfter`, in default user space units.
///
/// Two numbers rather than one rectangle adjustment, because the clause keeps them apart: each is
/// "measured in default user space units in the block-progression direction" and each has its own
/// sentence about which neighbour cancels it. [`BlockProgression`] is what turns the pair into two
/// edges.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BlockSpacing {
    /// `/SpaceBefore`: "\[t]he amount of extra space preceding the before edge of the BLSE".
    pub before: f32,
    /// `/SpaceAfter`: "\[t]he amount of extra space following the after edge of the BLSE".
    pub after: f32,
}

/// §14.8.5.4.5's allocation rectangle, from a content rectangle and the element's spacing.
///
/// > For a BLSE, the allocation rectangle shall be equal to the content rectangle with its before
/// > and after edges adjusted by the element's SpaceBefore and SpaceAfter attributes, if any, but
/// > with no changes to the start and end edges.
///
/// `content` is `[llx, lly, urx, ury]` in **default user space**, where y increases upwards, and
/// so is the answer. Which pair of edges moves is `mode`'s: the before edge moves outward by
/// [`BlockSpacing::before`] and the after edge outward by [`BlockSpacing::after`], and the other
/// two are untouched, which is the clause's own last clause.
///
/// # Why the result is sorted rather than trusted
///
/// Neither cell forbids a negative number — Table 378's `/Padding`, the entry beside them,
/// documents its negative case outright — so a producer may state one and the adjusted edges may
/// cross. A rectangle whose lower-left is above its upper-right is not a rectangle, and every
/// consumer of this one intersects and unions; sorting keeps those meaning what they say. It is
/// the same reading `normalised_rectangle` applies to §7.9.5's arrays, one step further on.
#[must_use]
pub fn allocation_rectangle(
    content: [f32; 4],
    spacing: BlockSpacing,
    mode: WritingMode,
) -> [f32; 4] {
    let [llx, lly, urx, ury] = content;
    let adjusted = match mode.block_progression() {
        BlockProgression::TopToBottom => [llx, lly - spacing.after, urx, ury + spacing.before],
        BlockProgression::BottomToTop => [llx, lly - spacing.before, urx, ury + spacing.after],
        BlockProgression::LeftToRight => [llx - spacing.before, lly, urx + spacing.after, ury],
        BlockProgression::RightToLeft => [llx - spacing.after, lly, urx + spacing.before, ury],
    };
    [
        adjusted[0].min(adjusted[2]),
        adjusted[1].min(adjusted[3]),
        adjusted[0].max(adjusted[2]),
        adjusted[1].max(adjusted[3]),
    ]
}

/// §14.8.5.4.5's row and column adjustment for the cells of **one** table.
///
/// The clause derives a `TH` or `TD`'s content rectangle from its own marks and then makes it its
/// row's and its column's:
///
/// > The cell's height shall be adjusted to equal the maximum height of any cell in its row; its
/// > width shall be adjusted to the maximum width of any cell in its column.
///
/// `cells` is one table's cells, each with the place [`TableStack::enter`] gave it and the
/// rectangle a caller has already derived for it — `None` for a cell nothing placed. The answer is
/// in the same order and the same space; only the two *intervals* of each rectangle are read, so
/// the space may be the page's or a viewport's and its y axis may run either way.
/// `progression` decides which interval is the height, through
/// [`BlockProgression::stacks_vertically`].
///
/// # A row's band rather than a row's maximum, and the difference is measurable
///
/// The clause can say *maximum* because a layout process places a row's cells against one before
/// edge, so a cell given the largest height fills the row. This program does not lay the table out:
/// each rectangle is the bounding box of what that cell actually drew, and two cells of one row
/// rarely start at the same edge. Taking the maximum *extent* from each cell's own edge would push
/// a short cell's rectangle past the row it is in. So the operand is the row's **band** — the
/// interval its cells occupy between them — which equals the clause's maximum wherever the
/// producer's own layout did put the cells' before edges together, and is a bound rather than an
/// overhang where it did not. It is the direction the rest of this rectangle already errs in
/// (§14.8.3.3's row, ADR 0486), and ADR 1073 is the argument.
///
/// # A spanning cell contributes to no band and takes several
///
/// A cell with a `/RowSpan` of 2 is as tall as two rows, and letting it into either row's band
/// would make every cell beside it that tall. So a band is built from the cells that occupy the
/// row alone, and a spanning cell is then given the union of the bands of every row it covers —
/// Table 384's `/ColSpan` the same way across columns. A cell no band reaches keeps the rectangle
/// it came with, which is a table whose every cell spans.
///
/// # An empty cell gets a rectangle it did not have
///
/// A `TD` that drew nothing arrives here as `None` and leaves with the intersection of its row's
/// band and its column's — which is where that cell is on the page, derived entirely from its
/// neighbours' marks and the grid. That is the clause's own construction rather than an invention:
/// the cell's height *is* its row's and its width *is* its column's.
#[must_use]
pub fn table_cell_rectangles(
    cells: &[(CellPlacement, Option<[f32; 4]>)],
    progression: BlockProgression,
) -> Vec<Option<[f32; 4]>> {
    let vertical = progression.stacks_vertically();
    // A rectangle's two intervals, the block-progression one first: Table 379 measures a height
    // "in the block-progression direction" and a width "in the inline-progression direction".
    let split = |rect: &[f32; 4]| {
        let [x0, y0, x1, y1] = *rect;
        let across = (x0.min(x1), x0.max(x1));
        let down = (y0.min(y1), y0.max(y1));
        if vertical {
            (down, across)
        } else {
            (across, down)
        }
    };
    let join = |height: (f32, f32), width: (f32, f32)| {
        if vertical {
            [width.0, height.0, width.1, height.1]
        } else {
            [height.0, width.0, height.1, width.1]
        }
    };
    let mut rows: BTreeMap<usize, (f32, f32)> = BTreeMap::new();
    let mut columns: BTreeMap<usize, (f32, f32)> = BTreeMap::new();
    for (place, rect) in cells {
        let Some(rect) = rect else { continue };
        let (height, width) = split(rect);
        if place.row_span <= 1 {
            widen(rows.entry(place.row).or_insert(height), height);
        }
        if place.column_span <= 1 {
            widen(columns.entry(place.column).or_insert(width), width);
        }
    }
    // Every band the cell covers, unioned onto what the cell itself had. `range` rather than a
    // loop over the span, because Table 384's two entries are a document's own numbers and a
    // `/RowSpan` of a billion would otherwise be a billion iterations of nothing.
    let covered = |bands: &BTreeMap<usize, (f32, f32)>, from: usize, span: usize, own| {
        let last = from.saturating_add(span.max(1).saturating_sub(1));
        let mut out: Option<(f32, f32)> = own;
        for band in bands.range(from..=last).map(|(_, band)| *band) {
            match out.as_mut() {
                Some(so_far) => widen(so_far, band),
                None => out = Some(band),
            }
        }
        out
    };
    cells
        .iter()
        .map(|(place, rect)| {
            let own = rect.as_ref().map(&split);
            let height = covered(
                &rows,
                place.row,
                place.row_span,
                own.map(|(height, _)| height),
            );
            let width = covered(
                &columns,
                place.column,
                place.column_span,
                own.map(|(_, width)| width),
            );
            match (height, width) {
                (Some(height), Some(width)) => Some(join(height, width)),
                // A cell no band reaches, which is a table every one of whose cells spans.
                _ => *rect,
            }
        })
        .collect()
}

/// Unions one interval into another, which is the whole of the arithmetic above.
fn widen(band: &mut (f32, f32), span: (f32, f32)) {
    band.0 = band.0.min(span.0);
    band.1 = band.1.max(span.1);
}

/// ISO 32000-2 §14.8.2.2.2's artifact **by absence**: the runs of a page's readback that nothing
/// in the structure tree claims.
///
/// > Any content that is not included in the structure tree is an artifact
///
/// — and the sentence goes on to say it even of content no marked-content sequence tagged
/// `/Artifact` encloses. NOTE 2 widens the subject: "\[t]he phrase 'any content' above refers
/// to all page content as well as annotations."
///
/// `included` is every range of `text` some route into the structure tree covers — a sequence the
/// producer identified with an `/MCID` (§14.7.5.2), an annotation whose own `/StructParent` makes
/// it a content item (§14.7.5.3), and the artifacts the producer already declared. This is the
/// complement of their union, as the ranges a consumer may subtract.
///
/// # What decides inclusion, and what deliberately does not
///
/// §14.7.5.2 makes the `/MCID` the mechanism — a sequence "may be specified as a content item …
/// \[t]he marked-content sequence shall contain a property list … containing an MCID entry" — so a
/// sequence carrying one is a producer saying *this run is in the tree*. §14.7.5.4's parent tree is
/// the **index** back from the mark to the element, and a file may write one without the other:
/// two corpus documents state `/MCID`s their page's parent tree entry does not resolve while some
/// element's `/K` names them outright, 98 characters of body text between them. Deciding artifact
/// by absence on the index's silence would call a producer's broken index this reader's artifact,
/// so the caller's `included` is built from the `/MCID` and not from the resolution of it —
/// which also means this costs a tagged page no tree lookups at all. ADR 1100.
///
/// Empty runs are not returned: a gap of no characters between two included ranges is not content.
#[must_use]
pub fn artifacts_by_absence(
    text: &str,
    included: &[std::ops::Range<usize>],
) -> Vec<std::ops::Range<usize>> {
    let mut covered: Vec<std::ops::Range<usize>> = included
        .iter()
        .filter(|range| range.start < range.end && range.start < text.len())
        .map(|range| range.start..range.end.min(text.len()))
        .collect();
    covered.sort_unstable_by_key(|range| (range.start, range.end));

    let mut out = Vec::new();
    let mut at = 0_usize;
    for range in covered {
        if range.start > at {
            keep(text, at..range.start, &mut out);
        }
        at = at.max(range.end);
    }
    if at < text.len() {
        keep(text, at..text.len(), &mut out);
    }
    out
}

/// Keeps one run of [`artifacts_by_absence`]'s complement, unless it is white space alone.
///
/// A run that reads back as nothing but white space carries no content for the clause to
/// reclassify, and some of that white space is not the page's: §14.8.2.6.2 NOTE 1 is about a
/// document stating its word breaks, and `content.rs`'s `separate_text` reconstructs the ones a
/// document leaves to be inferred from where §9.4.4's text rendering matrix put the next glyph.
/// So a gap between two tagged paragraphs can be this reader's own inference, and naming it an
/// artifact would be this reader reporting on itself.
fn keep(text: &str, range: std::ops::Range<usize>, out: &mut Vec<std::ops::Range<usize>>) {
    if text
        .get(range.clone())
        .is_some_and(|run| run.chars().all(char::is_whitespace))
    {
        return;
    }
    out.push(range);
}

/// §14.8.2.2's artifact: content that is on the page and is not the document's content.
///
/// §14.8.2.2.1 divides a page in two. The real content is "material intentionally introduced
/// by the document's author and necessary to understand the content of the document"; "\[a]ll
/// other content is considered to be artifacts, whether generated by the PDF writer in the
/// course of pagination, layout, or other mechanical processes or introduced by the document
/// author for decoration".
///
/// Nothing here changes a mark. What it changes is what a *consumer* of the page's text may
/// do with it, and the clause is explicit that the choice is the consumer's: "\[a] text-to-speech
/// engine, for instance, may decide not to speak running heads or page numbers when the page is
/// turned", and NOTE 3 adds that "\[t]he purpose of tagged PDF is not to prescribe what the PDF
/// processor does, but to provide sufficient declarative and descriptive information to allow it
/// to make appropriate choices". So this crate reads which content is an artifact and of what
/// kind, and drops none of it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Artifact {
    /// `/Type` — Table 363's for the marked-content form, Table 385's for the element form — or
    /// `None` where neither states one.
    ///
    /// Absent for the `/Artifact BMC` form, which §14.8.2.2.2 calls "a generic artifact", and for
    /// an `Artifact` structure element with no attribute object.
    pub kind: Option<ArtifactKind>,
    /// `/Subtype`, which "should appear only when the `Type` entry has a value of `Pagination`"
    /// in Table 363 and "of Pagination or Inline" in Table 385 — carried as written rather than
    /// as an enum, because the entry is open in both: "\[a]dditional values may be specified for
    /// this entry, provided they comply with the naming conventions described in Annex E".
    pub subtype: Option<String>,
    /// `/BBox`: "the rectangle that completely encloses its visible extent", in default user
    /// space, as `[llx, lly, urx, ury]`. Both tables, in the same words.
    pub bbox: Option<[f32; 4]>,
    /// Table 363's `/Attached`: which page edges the artifact is logically attached to.
    ///
    /// "Page edges shall be defined by the page's crop box", and the order of the names "is
    /// immaterial" — so this is a set of four flags in the clause's own order, top, bottom,
    /// left, right.
    ///
    /// Four falses for the element form, because Table 385 has no such entry — the table's
    /// silence rather than an unread one.
    pub attached: [bool; 4],
}

/// The artifact types §14.8.2.2's two tables name, which are **four each and five in all**.
///
/// Table 363's property list and Table 385's attribute object describe the same artifact under
/// two forms, and three of the four names are shared — `Pagination`, `Layout` and `Page`. The
/// fourth is not: the property list's is `Background` and the attribute object's is PDF 2.0's
/// `Inline`. Each reader takes its own table's four, which is what keeps this one enum: a
/// consumer asks what kind of artifact it has, and the answer is never a name the table it came
/// from does not define.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactKind {
    /// `Pagination`: "ancillary page features such as running heads, folios (page numbers) or
    /// Bates Numbering".
    ///
    /// Both tables.
    Pagination,
    /// `Layout`: "purely cosmetic typographical or design elements such as footnote rules or
    /// decorative ornaments".
    ///
    /// Both tables.
    Layout,
    /// `Page`: "production aids extraneous to the document itself, such as cut marks and
    /// colour bars".
    ///
    /// Both tables. Table 385 says "cut marks and print control patches" for the same kind.
    Page,
    /// `Background`: content "from document templates that are often repeated unchanged across
    /// many pages".
    ///
    /// Table 363 alone, and the property list's `/Attached` has a sentence conditioned on it:
    /// "\[u]se of this entry for background artifacts shall be limited to full-page artifacts."
    Background,
    /// `Inline` (PDF 2.0): artifact content "that has context in the document's logical
    /// structure" — typically, the cell says, an artifact whose subtype is `LineNum` or
    /// `Redaction`.
    ///
    /// Table 385 alone, which is the table whose form *is* a structure element — §14.8.2.2.2's
    /// NOTE 1 says the element type exists "to accommodate artifact content in cases that have
    /// positional context relative to real content within the structure tree", and this name is
    /// that case saying so.
    Inline,
}

impl Artifact {
    /// Reads Table 363's property list. `None` is a perfectly good artifact: see [`Self::kind`].
    ///
    /// [`ArtifactKind::Inline`] is **not** one of the names this accepts: it is Table 385's
    /// fourth and a property list stating it has stated a name §14.8.2.2.2's table does not
    /// define. [`Tree::artifact`] is the reader that takes it, and it declines `Background` for
    /// the same reason in the other direction.
    #[must_use]
    pub fn read(document: &Document, list: &Dictionary) -> Self {
        let kind =
            document
                .get_key(list, "Type")
                .as_name()
                .and_then(|name| match name.as_bytes() {
                    b"Pagination" => Some(ArtifactKind::Pagination),
                    b"Layout" => Some(ArtifactKind::Layout),
                    b"Page" => Some(ArtifactKind::Page),
                    b"Background" => Some(ArtifactKind::Background),
                    _ => None,
                });
        let subtype = document
            .get_key(list, "Subtype")
            .as_name()
            .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned());

        let mut attached = [false; 4];
        if let Some(items) = document.get_key(list, "Attached").as_array() {
            for item in items {
                if let Some(name) = document.resolve(item).as_name() {
                    match name.as_bytes() {
                        b"Top" => attached[0] = true,
                        b"Bottom" => attached[1] = true,
                        b"Left" => attached[2] = true,
                        b"Right" => attached[3] = true,
                        _ => {}
                    }
                }
            }
        }

        Self {
            kind,
            subtype,
            bbox: rectangle(document, list),
            attached,
        }
    }
}

/// Table 363's `/BBox`, normalised the way a page's boxes are.
fn rectangle(document: &Document, list: &Dictionary) -> Option<[f32; 4]> {
    normalised_rectangle(document, &document.get_key(list, "BBox"))
}

/// Where §12.5.2 puts each annotation the page lists, in **default user space**.
///
/// # Why this is in §14.7's module
///
/// §14.7.5.3 is the clause that needs it. An object reference makes a structure element's content
/// "an entire PDF object, such as an `XObject` directly or indirectly referenced by a page
/// description or an annotation" — and for one of those two the standard states outright where the
/// object is. ISO 32000-2 §12.5.2, Table 166, makes `/Rect` "(Required)" and
///
/// > defining the location of the annotation on the page in default user space units
///
/// so an element whose content *is* an annotation has a place even when it marked no text, which
/// is exactly the population [`Tree::bounds`] could not reach: 333 of the corpus's 1675 placeless
/// elements that state no Table 379 `/BBox` are placed by this, and every `Form` element among
/// them (`pdf-model --example element_bounds_census`).
///
/// # The other half of that sentence has no answer, and it is not an oversight
///
/// An `XObject`'s place is the transformation matrix in force at the `Do` that painted it, which
/// lives in the content stream rather than in the object — and NOTE 2 says the same thing from the
/// producer's side: an object rendered "multiple times on the same page" needs only "a single
/// object reference", so the reference cannot be naming one of the places. Nothing is invented for
/// it here.
///
/// # Only the annotations the page itself lists
///
/// §12.5.2 makes `/Annots` "an array of annotation dictionaries", and membership in it is the one
/// available check that the referenced object *is* an annotation of *this* page. Table 166 makes
/// `/Type` optional, so a `/Rect` read off any dictionary that happens to carry one would place an
/// element from a `/Rect` the standard never promised — the shape ADR 0215 paid for when a
/// signature dictionary stating no `/Type` was read as not being one.
///
/// A map rather than a lookup per element, because a page is asked about once and its `/Annots`
/// array is walked once for however many object references its structure tree states.
#[must_use]
pub fn annotation_rectangles(
    document: &Document,
    page: &Dictionary,
) -> BTreeMap<ObjectId, [f32; 4]> {
    let mut out = BTreeMap::new();
    let entry = document.get_key(page, "Annots");
    let Some(array) = entry.as_array() else {
        return out;
    };
    for item in array {
        // The reference is the identity an object reference names; a `/Annots` entry written
        // inline has none, and §14.7.5.3's `/Obj` could not have named it.
        let Some(object) = item.as_reference() else {
            continue;
        };
        let Some(dict) = document.get(object).as_dict().cloned() else {
            continue;
        };
        if let Some(rect) = crate::annotation::rectangle(document, &dict, "Rect") {
            out.insert(object, rect);
        }
    }
    out
}

/// Table 166's `/Lang` for each annotation the page lists.
///
/// # Why this is in §14.9.2's module and not §12.5's
///
/// [`annotation_rectangles`]'s reason, one entry over. §14.7.5.3's object reference makes a
/// structure element's content an annotation, so the language §14.9.2.3's hierarchy gives that
/// element is decided by an entry on the *annotation*. ISO 32000-2 §12.5.2, Table 166, states what
/// the entry does:
///
/// > A language identifier overriding the document's language identifier to specify the natural
/// > language for all text in the annotation except where overridden by other explicit language
/// > specifications
///
/// §14.9.2.3 names three carriers of a language specification — the catalog, a structure element
/// and a marked-content sequence — and an annotation is none of them, which is why this entry is
/// read here rather than folded into [`language`]. What it settles is the one case that clause
/// leaves to Table 166: an element whose content *is* an annotation speaks the annotation's
/// language, because the annotation is the innermost thing that stated one and "all text in the
/// annotation" is what such an element carries.
///
/// # Empty, absent, and malformed are three answers and this returns two of them
///
/// An empty string is §14.9.2.2's "the language is unknown", which this turns into the
/// same `None` an absent entry gives — the choice `language` already made for a structure element,
/// made once and for the same reason. A tag that is not well-formed is carried as the file wrote
/// it: Errata Collection 3's Issue #105 states the "invalid means unknown" recovery for Table 29's
/// catalog entry alone, and inventing it here would cancel a language a reader could still use.
///
/// Only the annotations the page's own `/Annots` array lists, for [`annotation_rectangles`]'s
/// reason: an entry read off any dictionary that happens to carry a `/Lang` would be answering for
/// an object the standard never said was an annotation of this page.
#[must_use]
pub fn annotation_languages(document: &Document, page: &Dictionary) -> BTreeMap<ObjectId, String> {
    let mut out = BTreeMap::new();
    let entry = document.get_key(page, "Annots");
    let Some(array) = entry.as_array() else {
        return out;
    };
    for item in array {
        let Some(object) = item.as_reference() else {
            continue;
        };
        let Some(dict) = document.get(object).as_dict().cloned() else {
            continue;
        };
        if let Some(tag) = text_entry(document, &dict, "Lang") {
            out.insert(object, tag);
        }
    }
    out
}

/// A four-number array as a rectangle, normalised the way a page's boxes are.
///
/// Shared by Table 363's artifact `/BBox` and Table 379's layout one: both are "the rectangle
/// that completely encloses" something visible, both are stated in default user space, and
/// neither clause says which pair of corners comes first. §7.9.5 is why they are sorted rather
/// than trusted: a rectangle is "an array of four numbers giving the coordinates of a pair of
/// diagonally opposite corners", and the `[llx lly urx ury]` order is the one it says the array
/// "\[t]ypically" takes rather than the one it requires.
fn normalised_rectangle(document: &Document, array: &Object) -> Option<[f32; 4]> {
    let items = array.as_array()?;
    if items.len() < 4 {
        return None;
    }
    let mut values = [0f32; 4];
    for (slot, item) in values.iter_mut().zip(items) {
        let number = document.resolve(item).as_number()?;
        if !number.is_finite() {
            return None;
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a page coordinate is bounded by §14.11.2's 14 400 units"
        )]
        {
            *slot = number as f32;
        }
    }
    Some([
        values[0].min(values[2]),
        values[1].min(values[3]),
        values[0].max(values[2]),
        values[1].max(values[3]),
    ])
}

/// §14.8.6.1's default standard structure namespace, which is PDF 1.7's.
///
/// > To facilitate conversion of documents created against versions of the PDF standard
/// > earlier than PDF 2.0, the default standard structure namespace shall be
/// > "`http://iso.org/pdf/ssn`".
pub const DEFAULT_STANDARD_NAMESPACE: &str = "http://iso.org/pdf/ssn";

/// §14.8.6.1's standard structure namespace for PDF 2.0.
///
/// The two together are what the clause calls "the standard structure namespaces"; a tagged
/// document's elements are required to be in one of them, in §14.8.6.3's `MathML` namespace, or
/// role mapped into one.
pub const STANDARD_NAMESPACE_2_0: &str = "http://iso.org/pdf2/ssn";

/// §14.8.6.3's one domain-specific namespace: `MathML`.
///
/// "`MathML` is the only domain-specific namespace defined in PDF 2.0", and the clause exempts
/// it from role mapping — a namespace named here "\[does] not require a `RoleMapNS` entry".
///
/// **Without a version** (ADR 0254). `doc/md/` says
/// "`MathML` 3.0" throughout the clause and Errata Collection 3 takes the version out of every
/// one of them (Issue #72, `/State` `Review` `Completed`), because the normative reference it
/// pointed at is itself replaced — §2's "Mathematical Markup Language (`MathML`) Version 3.0"
/// becomes "`MathML` Core" (Issue #719). The namespace URI is unchanged, which is what this
/// constant is; what changed is that naming an edition here would now be naming the wrong one.
pub const MATHML_NAMESPACE: &str = "http://www.w3.org/1998/Math/MathML";

/// §14.8.6.3's `math` structure element type, spelled the way the subclause spells it.
///
/// > NOTE 2 The math structure element type is all lowercase to match the MathML 3.0
/// > specification.
///
/// A note rather than a `shall`, and it is still load-bearing: §7.3.5 makes a name's identity its
/// bytes, so `Math` and `math` are two different types and only one of them is this one.
pub const MATHML_MATH_TYPE: &str = "math";

/// §14.7.4.2's namespace dictionary. Table 356.
#[derive(Debug, Clone, PartialEq)]
pub struct Namespace {
    /// `/NS`: "\[t]he string defining the namespace name which this entry identifies
    /// (conventionally a uniform resource identifier, or URI)".
    ///
    /// A string rather than a URI: NOTE 1 says "\[i]t is not generally expected that a URI for
    /// a namespace name will resolve. It is instead used for uniqueness", so parsing it would
    /// answer a question nobody asks and could reject a name a document uses.
    pub name: String,
    /// `/Schema`: the file specification of the schema defining this namespace.
    ///
    /// Held as the raw object because §7.11's file specifications are refused by architecture
    /// — this reader has no filesystem — and because NOTE 2 says the schema has no required
    /// format. What a consumer can do with it is say that one exists.
    pub schema: Option<Object>,
    /// `/RoleMapNS`: this namespace's own role map, read by [`Tree::role`].
    pub role_map: Option<Dictionary>,
}

impl Namespace {
    /// Reads a namespace dictionary, or `None` where it states no name.
    ///
    /// Table 356 makes `/NS` required, and a namespace without one identifies nothing — the
    /// entry is the whole point of the dictionary.
    #[must_use]
    pub fn read(document: &Document, dict: &Dictionary) -> Option<Self> {
        let Object::String(bytes) = document.get_key(dict, "NS") else {
            return None;
        };
        Some(Self {
            name: pdf_syntax::text_string(&bytes),
            schema: dict.get("Schema").cloned(),
            role_map: document.get_key(dict, "RoleMapNS").as_dict().cloned(),
        })
    }

    /// Whether this is one of §14.8.6.1's two standard structure namespaces.
    ///
    /// It might look like a validator's question, on the reading that §14.8.6.2's "all structure
    /// elements shall be in at least one of the standard structure
    /// namespaces or in a namespace identified in 14.8.6.3" is addressed to a *document*, so
    /// asking it is validating a file rather than drawing one.
    ///
    /// That reading is half right and it hides a reader's question behind a writer's. Whether a
    /// document *conforms* is indeed not asked here; which vocabulary a type name belongs to is,
    /// every time [`Tree::standard_role`] is called, and §14.8.4's forty-one names mean what
    /// they mean **because of** the namespace they are defined in. So this is what tells a
    /// foreign namespace's `Table` from a table.
    #[must_use]
    pub fn is_standard(&self) -> bool {
        self.standard().is_some()
    }

    /// Which of §14.8.6.1's two standard structure namespaces this is, or `None` for any other.
    ///
    /// The name is compared whole, because a namespace name "is instead used for uniqueness" (NOTE
    /// 1 of §14.7.4.2) and two strings that differ name two namespaces.
    #[must_use]
    pub fn standard(&self) -> Option<StandardNamespace> {
        match self.name.as_str() {
            DEFAULT_STANDARD_NAMESPACE => Some(StandardNamespace::Pdf1_7),
            STANDARD_NAMESPACE_2_0 => Some(StandardNamespace::Pdf2_0),
            _ => None,
        }
    }
}

/// Whether §14.8.6.2 permits a tagged document's element to be in this namespace.
///
/// The clause's own disjunction: "all structure elements shall be in at least one of the
/// standard structure namespaces **or in a namespace identified in 14.8.6.3**". So the two of
/// §14.8.6.1 and the one of §14.8.6.3, and nothing else — "\[a]ny other namespaces can be
/// specified within a PDF document, but shall meet the requirements of role mapping described in
/// 14.8.6.2", which is what [`Tree::resolved`] has already followed by the time this is asked.
///
/// A dictionary stating no `/NS` is not permitted, for [`Tree::namespace`]'s reason: Table 356
/// makes the entry required, so the alternative is to treat a document's broken namespace as one
/// the clause names.
fn permitted_namespace(document: &Document, dict: &Dictionary) -> bool {
    Namespace::read(document, dict)
        .is_some_and(|space| space.is_standard() || space.name == MATHML_NAMESPACE)
}

/// What [`Tree::count_unenclosed_mathml`] carries down its descent.
///
/// A struct rather than three `&mut` parameters because the walk already takes the two the clause
/// needs — where it is and whether a `Formula` is above it — and a bound, an identity set and a
/// counter are the walk's own bookkeeping rather than its subject.
struct UnenclosedMathml {
    /// How many `math` elements in the `MathML` namespace have had no `Formula` above them.
    found: usize,
    /// Elements already counted, so that a tree which is its own descendant is counted once.
    seen: BTreeSet<ObjectId>,
    /// What is left of [`MAX_ELEMENTS`].
    budget: usize,
}

/// A namespace a tagged document's elements end in that §14.8.6.2 does not permit.
///
/// [`Tree::namespaces_outside_the_standard`]'s answer, one entry per namespace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForeignNamespace {
    /// The namespace name, or `None` where the namespace dictionary states no `/NS`.
    pub name: Option<String>,
    /// How many of the document's structure elements ended in it.
    pub elements: usize,
}

/// §14.7.6.1's attribute object. Table 360.
///
/// > An attribute object shall be a dictionary or stream that includes an O entry … identifying
/// > the conforming product that owns the attribute information. Other entries, except the NS
/// > entry, shall represent the attributes
///
/// which is why this holds the dictionary rather than a map: the attribute names are whatever
/// the owner defines, and the two entries that are *not* attributes are named by the clause.
///
/// **Errata Collection 3 replaces "conforming product that owns" with "owner of"**, so the clause
/// now reads "identifying the owner of the attribute information" (Issue #354, `/State`
/// `Completed`). Nothing here moves — the entry, its type and what it identifies are unchanged,
/// and the paragraph above already says *owner* — but the quotation is the 2020 text and says so
/// rather than pretending otherwise. The sponsored copy records EC3 as annotations and `doc/md/`
/// dropped every annotation in all fourteen documents; `tools/spec-errata` is what reads them
/// back, ADR 0252.
#[derive(Debug, Clone, PartialEq)]
pub struct AttributeObject {
    /// Table 360's `/O`: who owns these attributes.
    ///
    /// One of §14.8.5's standard owners, `UserProperties`, `NSO`, or a name registered under
    /// Annex E. `NSO` means the owner is the namespace in [`Self::namespace`], which the
    /// clause states in both directions: "\[i]f the value for the O entry is NSO then the NS
    /// entry shall be present", and §14.7.4.2 adds that a namespace name matching a standard
    /// owner's "shall be considered equivalent".
    pub owner: String,
    /// The same entry as §14.8.5.2's vocabulary, which is what decides §14.8.5.3's priority.
    pub kind: Owner,
    /// Table 360's `/NS`, present exactly when `/O` is `NSO`, as the namespace it names.
    pub namespace: Option<Namespace>,
    /// §14.7.6.3's revision number: the one stated beside this object in the element's `/A` or
    /// `/C` array, and **0 where none was**.
    ///
    /// "An attribute object or class name that is not followed by an integer array element
    /// shall have a revision number of 0". Deprecated with PDF 2.0, and read because it is
    /// what makes the arrays parseable at all.
    pub revision: i64,
    /// Which of §14.7.6's two routes attached this object: `true` for the element's `/C`,
    /// `false` for its `/A`.
    ///
    /// The route is part of §14.8.5.3's ranking rather than bookkeeping. Two of the clause's
    /// priorities name "the element's A entry" and the third names "a class map associated with
    /// the element's C entry", so an object's band cannot be read off its owner alone — see
    /// [`Self::priority`].
    pub from_class: bool,
    /// The dictionary itself, whose other entries are the attributes.
    pub dict: Dictionary,
}

impl AttributeObject {
    /// Reads one attribute object, or `None` for anything that is not one.
    ///
    /// Table 360 makes `/O` required, so a dictionary without one has not said whose
    /// attributes these are — and since an attribute's meaning is its owner's, a reader that
    /// took the entries anyway would be inventing a vocabulary.
    fn read(document: &Document, object: &Object, revision: i64, from_class: bool) -> Option<Self> {
        let resolved = document.resolve(object);
        let dict = resolved.as_dict()?;
        let owner = document.get_key(dict, "O").as_name()?.clone();
        let namespace = document
            .get_key(dict, "NS")
            .as_dict()
            .and_then(|space| Namespace::read(document, space));
        let owner = String::from_utf8_lossy(owner.as_bytes()).into_owned();
        Some(Self {
            kind: Owner::read(&owner),
            owner,
            namespace,
            revision,
            from_class,
            dict: dict.clone(),
        })
    }

    /// The owner this object's attributes are interpreted under, after §14.7.4.2's equivalence.
    ///
    /// §14.8.5.2 makes the owner the interpretation — "\[m]ultiple owners may define like-named
    /// attributes with different value types or interpretations" — and §14.7.4.2 says how an
    /// `NSO` object names one:
    ///
    /// > When the owner of an attribute object (see Table 360 -Entries common to all attribute
    /// > object dictionaries) is specified by an NS entry, the namespace name shall be considered
    /// > as identifying the owner.  For common namespace names which correspond to the values of
    /// > owner entries defined in Table 376 -Standard structure attribute owners, they shall be
    /// > considered equivalent.
    ///
    /// So the namespace name is put through [`Owner::read`], which is the whole of the
    /// correspondence this standard states: a name that *is* one of Table 376's values is that
    /// owner, and the clause lists no other pairing — it says "common namespace names which
    /// correspond" and never prints one, so a table of URIs mapped onto owner values would be
    /// this reader's invention wearing the clause's number.
    ///
    /// `None` is a **refusal, and it is what this program does not process**, in three shapes:
    ///
    /// - an export format's owner, `HTML-4.01` or `CSS-3` or an Annex E name — §14.8.5.2 says
    ///   such an object "shall be applied only when processing PDF content based on that format",
    ///   and nothing here translates to one;
    /// - `UserProperties`, whose attributes are §14.7.6.4's `/P` array rather than the entries
    ///   beside it;
    /// - a namespace owner outside §14.8.6.1's two standard structure namespaces whose name is
    ///   not a Table 376 value either. `MathML`'s namespace is the standard's own example of
    ///   one, and it is in the same position as `HTML-4.01`: a vocabulary this program does not
    ///   process.
    ///
    /// **A standard structure namespace is not refused**, and the reason is §14.8.6.1's: the two
    /// names it defines are the schema for §14.8.4 and §14.8.5 themselves, so the format such an
    /// owner indicates is PDF, which is the format being processed. ADR 1077.
    #[must_use]
    pub fn effective_owner(&self) -> Option<Owner> {
        match &self.kind {
            Owner::Namespace => {
                let space = self.namespace.as_ref()?;
                match Owner::read(&space.name) {
                    native if native.is_pdf_native() => Some(native),
                    // Not an equivalence the clause states: the namespace is PDF's own
                    // vocabulary, which is not one Table 376 owner but the table's whole
                    // PDF-native half, so it ranks where §14.8.5.3 puts an owner that is none
                    // of the five.
                    _ if space.is_standard() => Some(Owner::Namespace),
                    _ => None,
                }
            }
            Owner::UserProperties | Owner::Format(_) => None,
            owner => Some(owner.clone()),
        }
    }

    /// §14.8.5.3's band for this object, or `None` where the clause does not admit it.
    ///
    /// The clause's first three priorities, each in its own words, and the band is decided by
    /// [`Self::effective_owner`] and [`Self::from_class`] together:
    ///
    /// 1. [`Priority::Owner`] — "\[t]he value of the attribute specified in the element's A
    ///    entry, owned by an owner as specified by the O entry, or, if the value of the O entry
    ///    is NSO , the NS entry, excluding Layout, PrintField, Table , List and Artifact , if
    ///    present, and if processing based on the format indicated by the owner value";
    /// 2. [`Priority::Standard`] — "owned by Layout, PrintField, Table, List or Artifact";
    /// 3. [`Priority::Class`] — "specified in a class map associated with the element's C entry".
    ///
    /// The clause conditions band 1 and puts no condition on band 3, and both of those are the
    /// clause's own asymmetry rather than this reader's: an object the first two bands would
    /// refuse is refused in the class map too, because §14.8.5.2's sentence about an export
    /// format is about the *object* and names no route.
    #[must_use]
    #[expect(
        clippy::doc_markdown,
        reason = "a verbatim quotation: §14.8.5.3 spells the owner names without backticks, and \
                  adding them inside the quotation marks would make the conformance gate's \
                  quotation check fail"
    )]
    pub fn priority(&self) -> Option<Priority> {
        let owner = self.effective_owner()?;
        if self.from_class {
            return Some(Priority::Class);
        }
        Some(if owner.is_pdf_native() {
            Priority::Standard
        } else {
            Priority::Owner
        })
    }

    /// The owner as the file wrote it: `/O`, or the namespace name where `/O` is `NSO`.
    ///
    /// §14.7.4.2 is why the second case is the name rather than the string `NSO` — "the
    /// namespace name shall be considered as identifying the owner" — and this is what a refusal
    /// says out loud, so that an owner nobody here processes is named rather than dropped.
    #[must_use]
    pub fn owner_as_written(&self) -> String {
        match (&self.kind, self.namespace.as_ref()) {
            (Owner::Namespace, Some(space)) => space.name.clone(),
            _ => self.owner.clone(),
        }
    }

    /// One attribute's value, resolved.
    ///
    /// `/O` and `/NS` are not attributes and are not answered: the clause says the other
    /// entries "shall represent the attributes", and these two identify the owner.
    #[must_use]
    pub fn get(&self, document: &Document, name: &str) -> Option<Object> {
        if name == "O" || name == "NS" {
            return None;
        }
        match document.get_key(&self.dict, name) {
            Object::Null => None,
            value => Some(value),
        }
    }

    /// §14.7.6.4's user properties, where this object holds them.
    ///
    /// Empty unless `/O` is `UserProperties`, which Table 361 requires of an object carrying
    /// a `/P` array — the entry name is one a standard owner could also use, and reading it
    /// as user properties on any owner would be inventing the type of somebody else's
    /// attribute.
    #[must_use]
    pub fn user_properties(&self, document: &Document) -> Vec<UserProperty> {
        if self.owner != "UserProperties" {
            return Vec::new();
        }
        let properties = document.get_key(&self.dict, "P");
        let Some(items) = properties.as_array() else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for item in items.iter().take(MAX_CHILDREN) {
            let resolved = document.resolve(item);
            let Some(dict) = resolved.as_dict() else {
                continue;
            };
            let Object::String(name) = document.get_key(dict, "N") else {
                continue;
            };
            let value = document.get_key(dict, "V");
            if matches!(value, Object::Null) {
                // Table 362 makes both `/N` and `/V` required; a property with no value
                // states nothing about the object it is attached to.
                continue;
            }
            out.push(UserProperty {
                name: pdf_syntax::text_string(&name),
                value,
                formatted: match document.get_key(dict, "F") {
                    Object::String(bytes) => Some(pdf_syntax::text_string(&bytes)),
                    _ => None,
                },
                hidden: matches!(document.get_key(dict, "H"), Object::Boolean(true)),
            });
        }
        out
    }
}

/// §14.8.5.3's priority, as far as it is about *this* element.
///
/// > The following list shows the priority for determining attribute values. A PDF processor
/// > determines an attribute's value to be the first item in the following list that applies
///
/// The list has five items and this type is its first three; the fourth is the parent's resolved
/// value and is [`Tree::inherited_attribute`], and the fifth is the attribute's own default,
/// which §14.8.5.4's tables state per attribute and this reader answers wherever it knows the
/// attribute. The derived order **is** the clause's: `Owner` beats `Standard` beats `Class`, so
/// the winner among a set of candidates is the smallest.
///
/// Each band's members are [`AttributeObject::priority`]'s subject, and the one that needs saying
/// twice is band 1's condition: it applies "if processing based on the format indicated by the
/// owner value", which for `HTML-4.01` this program never is and for §14.8.6.1's standard
/// structure namespaces it always is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    /// Priority 1: an `/A` object whose owner is none of the five PDF-native ones, and whose
    /// format this program is processing.
    Owner,
    /// Priority 2: an `/A` object owned by `Layout`, `PrintField`, `Table`, `List` or `Artifact`.
    Standard,
    /// Priority 3: an object reached through the element's `/C` and the structure tree root's
    /// class map.
    Class,
}

/// The value `name` takes among `attached`, by [`Priority`], where `admits` allows the owner.
///
/// Two orderings at once, and keeping them apart is the whole of the function: §14.8.5.3 ranks
/// the *bands* and §14.7.6 ranks the objects *inside* one. So the walk runs backwards — which is
/// the order [`Tree::attributes`] made the last stated value the winner in — and takes the first
/// value it meets at each band, keeping the best band it has seen. An object stating nothing
/// under `name` does not claim its band, because the clause ranks the *value* of an attribute
/// and an object that states none has stated no value.
///
/// `admits` is §14.8.5.8's narrowing and nothing else: [`Tree::attribute`] admits every owner the
/// clause ranks, and [`Tree::artifact_attribute`] takes four of the five PDF-native ones out.
fn ranked(
    document: &Document,
    attached: &[AttributeObject],
    name: &str,
    admits: impl Fn(&Owner) -> bool,
) -> Option<Object> {
    let mut best: Option<(Priority, Object)> = None;
    for object in attached.iter().rev() {
        let Some(owner) = object.effective_owner() else {
            continue;
        };
        if !admits(&owner) {
            continue;
        }
        let Some(priority) = object.priority() else {
            continue;
        };
        if best.as_ref().is_some_and(|(held, _)| *held <= priority) {
            continue;
        }
        if let Some(value) = object.get(document, name) {
            best = Some((priority, value));
        }
    }
    best.map(|(_, value)| value)
}

/// §14.8.5.2's standard attribute owners. Table 376.
///
/// The owner "determines the interpretation of the attributes defined in the object", so it is
/// not a label: `/BBox` under `Layout` and `/BBox` under `HTML-4.01` are different attributes
/// that happen to share a name. It also decides §14.8.5.3's priority, where the five *PDF-native*
/// owners rank below the format-specific ones.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Owner {
    /// `Layout`: "\[a]ttributes governing the layout of content".
    Layout,
    /// `List`: "governing the numbering of lists".
    List,
    /// `PrintField`: "governing `Form` structure elements for non-interactive form fields".
    PrintField,
    /// `Table`: "governing the organisation of cells in tables".
    Table,
    /// `Artifact`: "governing `Artifact` structure elements".
    Artifact,
    /// `UserProperties`: §14.7.6.4's, which Table 360 names beside the standard owners.
    UserProperties,
    /// `NSO`: the owner is the namespace the object's `/NS` names, which Table 360 requires to
    /// be present in exactly this case.
    ///
    /// This is `/O` as written and not the owner: §14.7.4.2 says "the namespace name shall be
    /// considered as identifying the owner", so which owner it is takes the namespace as well
    /// and is [`AttributeObject::effective_owner`].
    Namespace,
    /// One of Table 376's format-specific owners — `XML-1.00`, `HTML-4.01`, `CSS-3`, `ARIA-1.1`
    /// and the rest — or a name registered under Annex E, kept as written.
    ///
    /// One arm rather than thirteen: this reader translates to none of those formats, so what it
    /// needs from the name is that it is *not* one of the five PDF-native owners, which is what
    /// §14.8.5.3's first priority turns on.
    Format(String),
}

impl Owner {
    /// Reads Table 360's `/O`.
    #[must_use]
    pub fn read(name: &str) -> Self {
        match name {
            "Layout" => Self::Layout,
            "List" => Self::List,
            "PrintField" => Self::PrintField,
            "Table" => Self::Table,
            "Artifact" => Self::Artifact,
            "UserProperties" => Self::UserProperties,
            "NSO" => Self::Namespace,
            other => Self::Format(other.to_owned()),
        }
    }

    /// Whether this owner is one §14.8.5.3 ranks *below* a format-specific one.
    ///
    /// The clause's first priority is an attribute "owned by an owner as specified by the O
    /// entry … excluding `Layout`, `PrintField`, `Table`, `List` and `Artifact`, if present, and if
    /// processing based on the format indicated by the owner value"; its second is one owned by
    /// those five. So the five are the PDF-native vocabulary and everything else is somebody
    /// else's format — which outranks it *for a processor translating to that format*, and this
    /// one translates to none.
    ///
    /// Asked of an *effective* owner ([`AttributeObject::effective_owner`]) rather than of `/O` as
    /// written, because §14.7.4.2 lets a namespace name be one of these five.
    #[must_use]
    pub fn is_pdf_native(&self) -> bool {
        matches!(
            self,
            Self::Layout | Self::List | Self::PrintField | Self::Table | Self::Artifact
        )
    }
}

/// §14.7.6.4's user property. Table 362.
///
/// The clause's own example is a CAD part: several transistors "might have the same appearance
/// but different attributes such as type and part number". None of it is graphical, which is
/// why this crate reads it and draws nothing with it.
#[derive(Debug, Clone, PartialEq)]
pub struct UserProperty {
    /// `/N`: the property's name.
    pub name: String,
    /// `/V`: its value, as the object the file states.
    ///
    /// Any PDF object: "PDF writers should use only text string, number, and boolean values.
    /// PDF processors should display text, number and boolean values to users but need not
    /// display values of other types; however, they should not treat other values as errors."
    /// So the value arrives as it was written and the decision about what to show with it is
    /// the consumer's.
    pub value: Object,
    /// `/F`: a formatted representation of the value, "for example \"($123.45)\" for the
    /// number -123.45".
    pub formatted: Option<String>,
    /// `/H`: whether the property "shall not be shown in any user interface element that
    /// presents the attributes of an object". Default false.
    pub hidden: bool,
}

/// §14.7.6.3's pairing: each entry of an `/A` or `/C` array, with the revision beside it.
///
/// The array holds "a single or a pair of array elements, the first or only element shall
/// contain the attribute object itself and the second (when present) shall contain the integer
/// revision number" — so an integer *following* a value belongs to it, and an integer in any
/// other position is not an entry. A single object rather than an array is one entry with
/// revision 0, which is the form Table 355 allows for `/A` and `/C` alike.
fn paired(document: &Document, entry: &Object) -> Vec<(Object, i64)> {
    let Some(items) = entry.as_array() else {
        return match entry {
            Object::Null => Vec::new(),
            single => vec![(single.clone(), 0)],
        };
    };
    let mut out: Vec<(Object, i64)> = Vec::new();
    for item in items.iter().take(MAX_CHILDREN) {
        // The integer test is on the *unresolved* item: an indirect reference is an attribute
        // object, and resolving first would turn a reference to an integer — which no valid
        // file writes here — into a revision for the entry before it.
        if let Object::Integer(revision) = item {
            if let Some(last) = out.last_mut() {
                last.1 = *revision;
            }
            continue;
        }
        let resolved = match item {
            Object::Reference(_) => item.clone(),
            other => document.resolve(other),
        };
        out.push((resolved, 0));
    }
    out
}

/// Deepest chain of `/P` links followed when a structure element inherits its language.
///
/// §14.9.2.3 makes the inheritance unbounded — an element "shall inherit its language from any
/// parent element that has one" — and `/P` is a reference a document controls, so a file may
/// state a cycle or a chain thousands deep. Real hierarchies are a handful of levels; this is
/// far past any of them and is what makes the walk terminate. Reaching it answers "no language
/// stated", which is the same answer an untagged document gives, because a language is not a
/// mark on the page and refusing to speak would be worse than speaking in the default.
const MAX_ANCESTRY: usize = 64;

/// Table 355's `/ActualText` on a structure element, decoded.
///
/// §14.9.4 puts replacement text in two places — a `Span` property list and a structure element
/// — and says the same thing about both: it "shall be used as a replacement, not a description,
/// for the content". The property-list form is read where the property list is; this is the
/// other, and it needs the parent tree to be reachable at all.
#[must_use]
pub fn actual_text(document: &Document, element: &Dictionary) -> Option<String> {
    text_entry(document, element, "ActualText")
}

/// Table 355's `/Alt`, §14.9.3's alternate description of what the element contains.
#[must_use]
pub fn alternate_description(document: &Document, element: &Dictionary) -> Option<String> {
    text_entry(document, element, "Alt")
}

/// Table 355's `/E`, §14.9.5's expansion of the abbreviation the element tags.
#[must_use]
pub fn expansion(document: &Document, element: &Dictionary) -> Option<String> {
    text_entry(document, element, "E")
}

/// Table 355's `/Lang` on an element, or the nearest ancestor's.
///
/// §14.9.2.3 states both halves of this in one sentence:
///
/// > A structure element's language specification. If a structure element does not have a Lang
/// > entry, the element shall inherit its language from any parent element that has one.
///
/// So the walk goes up `/P` until an element states one, the chain ends, or `MAX_ANCESTRY` is
/// reached. `None` means no element in the chain said anything, which leaves the document
/// catalog's default in force.
#[must_use]
pub fn language(document: &Document, element: &Dictionary) -> Option<String> {
    let mut current = element.clone();
    for _ in 0..MAX_ANCESTRY {
        if let Some(tag) = text_entry(document, &current, "Lang") {
            return Some(tag);
        }
        // The structure tree root is the one parent that is not an element, and it states no
        // language of its own — §14.9.2.3 puts the document's default in the catalog instead.
        current = document.get_key(&current, "P").as_dict()?.clone();
    }
    None
}

/// A text-string entry on a structure element, decoded and with an empty value discarded.
///
/// §14.9.2.2 gives the empty string a meaning for `/Lang` — it is how a file says "the language
/// is unknown" — and that is the same answer as stating nothing, so both arrive here as `None`.
/// For `/Alt`, `/E` and `/ActualText` an empty string states no substitution, and treating it
/// as one would delete the text the element encloses.
fn text_entry(document: &Document, element: &Dictionary, key: &str) -> Option<String> {
    match document.get_key(element, key) {
        Object::String(bytes) => {
            let text = pdf_syntax::text_string(&bytes);
            (!text.is_empty()).then_some(text)
        }
        _ => None,
    }
}

/// The document catalog's `/Lang`, §14.9.2.3's default for everything in the file.
///
/// > The Lang entry in the document catalog dictionary shall specify the default natural
/// > language for all text in the document.
///
/// It is read here rather than in `page.rs` because it is the top of §14.9.2's hierarchy and
/// the rest of that hierarchy is this module's; 95 of the corpus's 974 documents state one.
///
/// A tag that is not well-formed answers `None`, because Errata Collection 3 (Issue #105)
/// inserts *or invalid (see 14.9.2, "Natural language specification")* into Table 29's `/Lang`
/// entry, so its last sentence reads: if this entry is absent or invalid, the language shall
/// be considered unknown — the same answer as an absent one, which is what `None` already
/// means here. The published entry stated that recovery for an absent tag only, and this
/// entry is the one place the standard states it at all: an element's or a `Span`'s invalid
/// tag is still carried as the file writes it, since no clause says what a reader does there
/// and inventing "unknown" would also cancel §14.9.2.3's inheritance for that element.
#[must_use]
pub fn document_language(document: &Document) -> Option<String> {
    let catalog = document.catalog().ok()?;
    text_entry(document, &catalog, "Lang").filter(|tag| well_formed_language_tag(tag))
}

/// Whether `tag` is BCP 47's `Language-Tag` — the well-formedness half of "valid".
///
/// §14.9.2.2 defines what a language identifier is:
///
/// > A language identifier shall either be the empty text string, to indicate that the
/// > language is unknown, or a Language-Tag as defined in BCP 47.
///
/// and Table 29's amended `/Lang` entry (Issue #105, above) is what makes the question a
/// reader's: an invalid identifier on the catalog is unknown rather than a language. BCP 47
/// (RFC 5646 section 2.1) defines `Language-Tag` twice over — a grammar, and a *validity* judgement
/// against the IANA subtag registry. This answers the grammar alone, as a deliberate choice:
/// the registry is a moving list this program does not hold, and a tag it would reject —
/// `zz-QQ`, well-formed and unassigned — still names a language to the file's own reader in a
/// way `(Deutsch, Français)`, which real producers write, does not. §14.9.2.2's own
/// case-insensitivity sentence is why every comparison here ignores case.
///
/// The grammar, from RFC 5646 section 2.1's ABNF: `langtag` is a language subtag (2–3 letters, with
/// up to three 3-letter extended-language subtags, or one of 4–8 letters), then optionally a
/// 4-letter script, a region (2 letters or 3 digits), any number of variants (5–8 characters,
/// or 4 beginning with a digit), any number of singleton-introduced extensions, and a private
/// `x-` tail; a private tail may also stand alone, and a closed list of grandfathered tags
/// predates the grammar.
///
/// **Public because a second reader needs the same test and may not have a second grammar.**
/// ISO 19005-2 section 6.7.4's only `shall` is that a `/Lang` which is present be a language
/// identifier as §14.9.2 defines, and `crates/pdf-archive` judges that for a *structure element*
/// and a marked-content property list where [`document_language`] answers only for the
/// catalog. Duplicating ninety lines of RFC 5646 ABNF and the grandfathered list into that crate
/// would give this project two grammars to keep in step, and would contradict `pdf-archive`'s own
/// design, which is to add no reader of its own.
///
/// Widening it costs the viewer nothing: the function is pure, allocates one `Vec` of borrowed
/// subtags, and is on no path the viewer walks more than once per document.
#[must_use]
pub fn well_formed_language_tag(tag: &str) -> bool {
    if GRANDFATHERED
        .iter()
        .any(|kept| kept.eq_ignore_ascii_case(tag))
    {
        return true;
    }
    let subtags: Vec<&str> = tag.split('-').collect();
    // Every subtag, whatever its role, is 1–8 ASCII alphanumerics; checking that once is what
    // lets the role checks below ask only about length and letter-versus-digit.
    if subtags.iter().any(|subtag| {
        subtag.is_empty()
            || subtag.len() > 8
            || !subtag.bytes().all(|byte| byte.is_ascii_alphanumeric())
    }) {
        return false;
    }
    // `privateuse` standing alone: `x` then at least one subtag.
    if subtags[0].eq_ignore_ascii_case("x") {
        return subtags.len() > 1;
    }
    language_tag_roles(&subtags)
}

/// RFC 5646 section 2.1's `langtag` production, one role at a time over the split subtags.
///
/// Each block peels what its role admits off the front of the slice, which is the ABNF's own
/// order: language with its extended-language subtags, script, region, variants, extensions,
/// and the private tail. What is left must be nothing.
fn language_tag_roles(subtags: &[&str]) -> bool {
    let letters = |subtag: &str| subtag.bytes().all(|byte| byte.is_ascii_alphabetic());
    let digits = |subtag: &str| subtag.bytes().all(|byte| byte.is_ascii_digit());

    // `language`: 2–3 letters — then up to three 3-letter `extlang`s — or 4–8 letters alone.
    let Some((language, mut rest)) = subtags.split_first() else {
        return false;
    };
    if !letters(language) {
        return false;
    }
    match language.len() {
        2 | 3 => {
            for _ in 0..3 {
                match rest {
                    [extended, tail @ ..] if extended.len() == 3 && letters(extended) => {
                        rest = tail;
                    }
                    _ => break,
                }
            }
        }
        4..=8 => {}
        _ => return false,
    }
    // `script`, then `region`, at most one each.
    if let [script, tail @ ..] = rest
        && script.len() == 4
        && letters(script)
    {
        rest = tail;
    }
    if let [region, tail @ ..] = rest
        && ((region.len() == 2 && letters(region)) || (region.len() == 3 && digits(region)))
    {
        rest = tail;
    }
    // `variant`s: 5–8 characters, or 4 beginning with a digit.
    loop {
        match rest {
            [variant, tail @ ..]
                if variant.len() >= 5
                    || (variant.len() == 4 && variant.as_bytes()[0].is_ascii_digit()) =>
            {
                rest = tail;
            }
            _ => break,
        }
    }
    // `extension`s: a singleton that is not `x`, then at least one subtag of 2–8 characters.
    while let [singleton, tail @ ..] = rest {
        if singleton.len() != 1 || singleton.eq_ignore_ascii_case("x") {
            break;
        }
        rest = tail;
        let mut carried = false;
        loop {
            match rest {
                [subtag, tail @ ..] if subtag.len() >= 2 => {
                    rest = tail;
                    carried = true;
                }
                _ => break,
            }
        }
        if !carried {
            return false;
        }
    }
    // `privateuse` as a tail: everything after the `x` is its own, and one subtag must follow.
    if let [private, tail @ ..] = rest
        && private.eq_ignore_ascii_case("x")
    {
        return !tail.is_empty();
    }
    rest.is_empty()
}

/// RFC 5646 section 2.1's `grandfathered` production: a closed list, part of `Language-Tag` by
/// definition, so refusing them would refuse tags the grammar names.
const GRANDFATHERED: [&str; 26] = [
    "en-GB-oed",
    "i-ami",
    "i-bnn",
    "i-default",
    "i-enochian",
    "i-hak",
    "i-klingon",
    "i-lux",
    "i-mingo",
    "i-navajo",
    "i-pwn",
    "i-tao",
    "i-tay",
    "i-tsu",
    "sgn-BE-FR",
    "sgn-BE-NL",
    "sgn-CH-DE",
    "art-lojban",
    "cel-gaulish",
    "no-bok",
    "no-nyn",
    "zh-guoyu",
    "zh-hakka",
    "zh-min",
    "zh-min-nan",
    "zh-xiang",
];

#[cfg(test)]
mod tests;
