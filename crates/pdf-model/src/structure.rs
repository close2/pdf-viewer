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
//! reason — "[b]ecause a stream cannot contain object references, there is no way for content
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

use pdf_syntax::{Dictionary, Document, Object, ObjectId, tree};

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
    /// Resolved here rather than when the tree was read; see [`Self::entries`].
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

/// One child of a structure element, in the four forms §14.7.5.1.1 defines.
///
/// > Content items are of two kinds:
///
/// marked-content sequences within content streams, and complete PDF objects such as
/// annotations and `XObject`s —
///
/// and the third possibility is not a content item at all but another element, which is what
/// makes the structure a tree. The clause's own restriction is what keeps this an enum rather
/// than a recursive type: "[c]ontent items shall be leaf nodes of the structure tree".
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
        mcid: i64,
        /// The page object it belongs to, if one was stated.
        page: Option<ObjectId>,
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
/// meaning in both — "[t]he K entry shall specify the immediate children of the structure
/// tree root, which shall be structure elements".
///
/// # What this is for
///
/// The parent tree above answers "which element does this marked-content sequence belong
/// to", which is what drawing a page needs. This answers the other direction — what the
/// document *says it is* — which is what a reading-order consumer, an accessibility tree or
/// a navigation panel needs. Like Table 99's `/Order`, the data is this crate's and the
/// consumer is not: nothing in this program yet hands a structure tree to anybody.
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
    /// which Table 355 states outright — "[i]f the value of K is a dictionary containing no
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
    fn child(document: &Document, entry: &Object, page: Option<ObjectId>) -> Option<Child> {
        if let Some(mcid) = document.resolve(entry).as_integer() {
            return Some(Child::MarkedContent { mcid, page });
        }
        let resolved = document.resolve(entry);
        let dict = resolved.as_dict()?;
        let kind = document.get_key(dict, "Type");
        let kind = kind.as_name().map(|name| name.as_bytes().to_vec());
        match kind.as_deref() {
            // Table 357: a marked-content reference names the sequence and may move both the
            // page and the stream it lives in.
            Some(b"MCR") => Some(Child::MarkedContent {
                mcid: document.get_key(dict, "MCID").as_integer()?,
                page: dict.get("Pg").and_then(Object::as_reference).or(page),
            }),
            // Table 358: an object reference. `/Obj` is required and is what identifies it.
            Some(b"OBJR") => Some(Child::Object {
                object: dict.get("Obj").and_then(Object::as_reference)?,
                page: dict.get("Pg").and_then(Object::as_reference).or(page),
            }),
            _ => Some(Child::Element(dict.clone())),
        }
    }

    /// The element's structure type, mapped through §14.7.3's `/RoleMap` where one applies.
    ///
    /// > Where names other than the standard ones are used, a role map should be provided in
    /// > the structure tree root using the RoleMap entry
    ///
    /// so a document's own `/Sect2` becomes the standard `/Sect` it maps to. The map is
    /// followed transitively — a role map may name a type that is itself mapped — with the
    /// same bound the tree walk uses, and a name that maps to itself or into a cycle answers
    /// the last name reached rather than looping.
    ///
    /// `None` for an element with no `/S`, which Table 355 makes required: a structure
    /// element that states no type says nothing about what it is, and inventing one would be
    /// the fallback-that-fills-the-page in another clause's clothing.
    #[must_use]
    pub fn role(&self, document: &Document, element: &Dictionary) -> Option<String> {
        let mut name = document.get_key(element, "S").as_name()?.clone();
        let map = document.get_key(&self.root, "RoleMap");
        let Some(map) = map.as_dict() else {
            return Some(String::from_utf8_lossy(name.as_bytes()).into_owned());
        };
        for _ in 0..MAX_DEPTH {
            let mapped = document.get_key(map, &String::from_utf8_lossy(name.as_bytes()));
            match mapped.as_name() {
                Some(next) if next != &name => name = next.clone(),
                _ => break,
            }
        }
        Some(String::from_utf8_lossy(name.as_bytes()).into_owned())
    }

    /// Every descendant of the root, depth first, with its depth.
    ///
    /// The order is §14.7.2's own: `/K` is a list, and the tree's order is what §14.8.2 calls
    /// the document's logical reading order. Bounded by [`MAX_DEPTH`] and by visiting each
    /// element once, because `/K` and `/P` are references a document controls.
    #[must_use]
    pub fn walk(&self, document: &Document) -> Vec<(usize, Child)> {
        let mut out = Vec::new();
        let mut seen: Vec<Dictionary> = Vec::new();
        self.descend(document, None, 0, &mut out, &mut seen);
        out
    }

    /// One level of [`Self::walk`].
    fn descend(
        &self,
        document: &Document,
        element: Option<&Dictionary>,
        depth: usize,
        out: &mut Vec<(usize, Child)>,
        seen: &mut Vec<Dictionary>,
    ) {
        if depth >= MAX_DEPTH || out.len() >= MAX_CHILDREN {
            return;
        }
        for child in self.children(document, element) {
            let descend_into = match &child {
                Child::Element(dict) if !seen.contains(dict) => Some(dict.clone()),
                _ => None,
            };
            out.push((depth, child));
            if let Some(dict) = descend_into {
                seen.push(dict.clone());
                self.descend(document, Some(&dict), depth.saturating_add(1), out, seen);
            }
        }
    }
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
/// So the walk goes up `/P` until an element states one, the chain ends, or [`MAX_ANCESTRY`] is
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
#[must_use]
pub fn document_language(document: &Document) -> Option<String> {
    let catalog = document.catalog().ok()?;
    text_entry(document, &catalog, "Lang")
}

#[cfg(test)]
mod tests {
    use super::{Child, ParentTree, Tree, actual_text};
    use pdf_syntax::Document;

    /// Builds a document from object bodies numbered from 1.
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

    /// A marked-content identifier finds its element through the number tree.
    ///
    /// Two lookups, both of which the clause states: the page's `/StructParents` is the key
    /// into `/ParentTree`, and the `/MCID` is "a zero-based index into the array" that comes
    /// back. The fixture puts two elements in the array so that indexing, rather than taking
    /// the first, is what passes.
    #[test]
    fn a_marked_content_identifier_finds_its_element() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /StructParents 7 >>",
            "<< /Type /StructTreeRoot /ParentTree 5 0 R >>",
            "<< /Nums [7 [6 0 R 8 0 R]] >>",
            "<< /Type /StructElem /S /P >>",
            "<< /Unused true >>",
            "<< /Type /StructElem /S /Span /ActualText (fi) >>",
        ]);
        let pages = crate::page::Pages::new(&doc);
        let page = pages.get(0).expect("page one");
        let parents = ParentTree::for_page(&doc, &page.dict);

        assert!(!parents.is_empty());
        let first = parents.element(&doc, 0).expect("the element for /MCID 0");
        assert_eq!(
            doc.get_key(&first, "S")
                .as_name()
                .map(|s| s.as_bytes().to_vec()),
            Some(b"P".to_vec())
        );
        assert_eq!(
            parents.element(&doc, 1).and_then(|e| actual_text(&doc, &e)),
            Some("fi".to_owned()),
            "the second entry, indexed rather than taken first"
        );
        assert!(parents.element(&doc, 2).is_none());
    }

    /// §14.7.2's tree, walked from the root, with §14.7.3's role map applied.
    ///
    /// The fixture is the shape §14.7.5.1.1 describes: an element whose children are another
    /// element and the three forms a content item takes — a bare integer, a marked-content
    /// reference and an object reference. Its own type is a name the document invented, which
    /// the role map takes to a standard one *transitively*, because a role map may name a
    /// type that is itself mapped.
    #[test]
    fn the_structure_tree_reads_its_children_and_maps_their_roles() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /StructParents 0 >>",
            "<< /Type /StructTreeRoot /K 5 0 R \
             /RoleMap << /Heading2 /MyHeading /MyHeading /H2 >> >>",
            "<< /Type /StructElem /S /Heading2 /P 4 0 R /Pg 3 0 R /K [0 6 0 R 7 0 R 8 0 R] >>",
            "<< /Type /StructElem /S /Span /P 5 0 R >>",
            "<< /Type /MCR /Pg 3 0 R /MCID 4 >>",
            "<< /Type /OBJR /Obj 9 0 R >>",
            "<< /Type /Annot /Subtype /Link /Rect [0 0 1 1] >>",
        ]);
        let tree = Tree::of(&doc).expect("a structure tree root");
        let page = pdf_syntax::ObjectId {
            number: 3,
            generation: 0,
        };

        let top = tree.children(&doc, None);
        assert_eq!(top.len(), 1, "one child of the root");
        let Some(Child::Element(heading)) = top.first() else {
            panic!("the root's child is an element: {top:?}");
        };
        assert_eq!(
            tree.role(&doc, heading).as_deref(),
            Some("H2"),
            "/Heading2 maps to /MyHeading maps to /H2"
        );

        let kids = tree.children(&doc, Some(heading));
        assert_eq!(
            kids.len(),
            4,
            "an element and three content items: {kids:?}"
        );
        assert_eq!(
            kids.first(),
            Some(&Child::MarkedContent {
                mcid: 0,
                page: Some(page)
            }),
            "an integer takes its page from the element's /Pg"
        );
        assert!(matches!(kids.get(1), Some(Child::Element(_))));
        assert_eq!(
            kids.get(2),
            Some(&Child::MarkedContent {
                mcid: 4,
                page: Some(page)
            })
        );
        assert_eq!(
            kids.get(3),
            Some(&Child::Object {
                object: pdf_syntax::ObjectId {
                    number: 9,
                    generation: 0
                },
                page: Some(page)
            }),
            "an object reference inherits the element's page where it states none"
        );

        // The walk is the same tree in the order `/K` states it, one level deeper for the
        // nested element's own children.
        let walked = tree.walk(&doc);
        assert_eq!(
            walked.len(),
            5,
            "the heading and its four children: {walked:?}"
        );
        assert!(matches!(walked.first(), Some((0, Child::Element(_)))));
        assert!(matches!(
            walked.get(1),
            Some((1, Child::MarkedContent { mcid: 0, .. }))
        ));
        assert!(
            matches!(walked.get(2), Some((1, Child::Element(_)))),
            "the nested element is a child at depth 1 and has none of its own"
        );
    }

    /// A `/K` cycle terminates, and an untagged document has no tree at all.
    #[test]
    fn a_structure_tree_that_points_at_itself_terminates() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            "<< /Type /StructTreeRoot /K 5 0 R >>",
            "<< /Type /StructElem /S /Sect /K [6 0 R] >>",
            "<< /Type /StructElem /S /Sect /K [5 0 R] >>",
        ]);
        let tree = Tree::of(&doc).expect("a structure tree root");
        assert_eq!(tree.walk(&doc).len(), 3, "each element is entered once");

        let untagged = document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        ]);
        assert!(Tree::of(&untagged).is_none());
    }

    /// A page with no `/StructParents` has no structure, and that is not a failure.
    #[test]
    fn a_page_outside_the_structure_tree_reads_as_empty() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        ]);
        let pages = crate::page::Pages::new(&doc);
        let page = pages.get(0).expect("page one");
        assert!(ParentTree::for_page(&doc, &page.dict).is_empty());
    }
}
