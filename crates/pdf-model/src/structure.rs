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
    /// **Which map applies is §14.8.6.2's question, not §14.7.3's.** An element that names its
    /// namespace is mapped by *that namespace's* `/RoleMapNS`: "if the structure element does
    /// not explicitly identify its namespace using an `NS` entry, it should use the `RoleMap`
    /// entry in the Structure Tree Root dictionary … If the structure element is in an
    /// explicit namespace, then … the `RoleMapNS` entry within that namespace dictionary shall
    /// provide the role mapping". A `/RoleMapNS` value may also name a *target namespace*
    /// alongside the type, which is how one document's vocabulary maps into another's, and
    /// this follows that too — the loop's state is a name **and** a namespace.
    ///
    /// `None` for an element with no `/S`, which Table 355 makes required: a structure
    /// element that states no type says nothing about what it is, and inventing one would be
    /// the fallback-that-fills-the-page in another clause's clothing.
    #[must_use]
    pub fn role(&self, document: &Document, element: &Dictionary) -> Option<String> {
        let mut name = document.get_key(element, "S").as_name()?.clone();
        // The element's own namespace, if it states one. `None` means §14.8.6.1's default
        // standard structure namespace, which is where the root's `/RoleMap` applies.
        let mut namespace = document.get_key(element, "NS").as_dict().cloned();

        for _ in 0..MAX_DEPTH {
            let key = String::from_utf8_lossy(name.as_bytes()).into_owned();
            // §14.8.6.2 decides which of the two maps is consulted, and nothing else does:
            // an element in an explicit namespace is mapped by that namespace's own.
            let map = match &namespace {
                Some(space) => document.get_key(space, "RoleMapNS"),
                None => document.get_key(&self.root, "RoleMap"),
            };
            let mapped = map
                .as_dict()
                .map_or(Object::Null, |map| document.get_key(map, &key));
            match mapped {
                // "a single name identifying a structure element type in the default standard
                // structure namespace" — so the mapping leaves whatever namespace it was in.
                Object::Name(next) => {
                    if next == name && namespace.is_none() {
                        break;
                    }
                    name = next;
                    namespace = None;
                }
                // "an array where the first value shall be a structure element type name in a
                // target namespace with the second value being an indirect reference to the
                // target namespace dictionary".
                Object::Array(items) => {
                    let Some(next) = items
                        .first()
                        .and_then(|item| document.resolve(item).as_name().cloned())
                    else {
                        break;
                    };
                    let target = items
                        .get(1)
                        .map(|item| document.resolve(item))
                        .and_then(|object| object.as_dict().cloned());
                    if next == name && target == namespace {
                        break;
                    }
                    name = next;
                    namespace = target;
                }
                _ => break,
            }
        }
        Some(String::from_utf8_lossy(name.as_bytes()).into_owned())
    }

    /// The element's §14.8.4 standard type, after §14.7.3's and §14.8.6.2's role mapping.
    ///
    /// [`Self::role`] answers the *name*, which is what a document wrote or what its role map
    /// took it to; this answers what that name means in the standard's own vocabulary. `None`
    /// where the mapped name is not a standard type, which §14.8.4.1 makes a defect in the
    /// document rather than a gap here — "[a]ll structure elements occurring within a tagged PDF
    /// document shall have a type matching one of those defined as a Standard Structure Type, or
    /// a role map providing a mapping from the non-standard type to a Standard Structure Type".
    #[must_use]
    pub fn standard_role(&self, document: &Document, element: &Dictionary) -> Option<StandardType> {
        StandardType::read(&self.role(document, element)?)
    }

    /// The namespace name an element is in, §14.8.6.1's default where it states none.
    ///
    /// > When a namespace is not explicitly specified for a given structure element or
    /// > attribute, it shall be assumed to be within this default standard structure namespace.
    ///
    /// which is [`DEFAULT_STANDARD_NAMESPACE`]. An element that states a `/NS` whose dictionary
    /// has no `/NS` string of its own has named a namespace this reader cannot identify, and
    /// answers `None` rather than the default — Table 356 makes the entry required, so the
    /// alternative would be to report a document's broken namespace as the standard one.
    #[must_use]
    pub fn namespace(&self, document: &Document, element: &Dictionary) -> Option<String> {
        let stated = document.get_key(element, "NS");
        let Some(space) = stated.as_dict() else {
            return Some(DEFAULT_STANDARD_NAMESPACE.to_owned());
        };
        Namespace::read(document, space).map(|space| space.name)
    }

    /// Every attribute object attached to an element, in **increasing precedence order**.
    ///
    /// §14.7.6 attaches attributes by two routes and states which wins in each. Within an
    /// element's `/A`, "[i]f a given attribute is specified more than once, the later (in array
    /// order) entry shall take precedence"; between the two routes, "[i]f both the A and C
    /// entries are present and a given attribute is specified by both, the one specified by
    /// the A entry shall take precedence". So the list is the `/C` classes first and the `/A`
    /// objects after them, each in array order, and the *last* object stating an attribute is
    /// the one that holds — which is what [`Self::attribute`] does with it.
    ///
    /// Both entries carry §14.7.6.3's revision numbers, and reading them is not optional even
    /// though the mechanism is deprecated in PDF 2.0 and useless to a reader that does not
    /// edit: the revision is "the second (when present)" element of a *pair* inside the same
    /// array, so a reader that did not know about it would take an integer for an attribute
    /// object. The clause's own reason is in its NOTE 3 — "since an attribute object reference
    /// is distinct from an integer, that distinction is used to determine whether the
    /// attribute object is represented in the array by a single or a pair of entries".
    #[must_use]
    pub fn attributes(&self, document: &Document, element: &Dictionary) -> Vec<AttributeObject> {
        let mut out = Vec::new();
        for (name, revision) in paired(document, &document.get_key(element, "C")) {
            let Object::Name(class) = name else { continue };
            let map = document.get_key(&self.root, "ClassMap");
            let Some(map) = map.as_dict() else { continue };
            let named = document.get_key(map, &String::from_utf8_lossy(class.as_bytes()));
            // "The corresponding value shall be an attribute object or an array of such
            // objects" — and this array is a plain list, since §14.7.6.3 puts the revision
            // beside the class *name* rather than beside the object it names.
            match named {
                Object::Array(items) => {
                    for item in items.iter().take(MAX_CHILDREN) {
                        if let Some(object) =
                            AttributeObject::read(document, &document.resolve(item), revision)
                        {
                            out.push(object);
                        }
                    }
                }
                other => {
                    if let Some(object) = AttributeObject::read(document, &other, revision) {
                        out.push(object);
                    }
                }
            }
        }
        for (object, revision) in paired(document, &document.get_key(element, "A")) {
            if let Some(object) = AttributeObject::read(document, &object, revision) {
                out.push(object);
            }
        }
        out
    }

    /// One attribute's value, by §14.8.5.3's priority, **without** the inherited step.
    ///
    /// The clause states five priorities and this applies the first three, which are the ones
    /// about *this* element:
    ///
    /// 1. an `/A` attribute owned by a format-specific owner — anything but Table 376's five
    ///    PDF-native ones — "if processing based on the format indicated by the owner value",
    ///    which this program is not, so it is skipped rather than preferred;
    /// 2. an `/A` attribute owned by `Layout`, `PrintField`, `Table`, `List` or `Artifact`;
    /// 3. an attribute from a class the element's `/C` names.
    ///
    /// §14.7.6's two rules decide within each of those, and [`Self::attributes`] has already put
    /// the objects in the order that makes the last match the winner.
    ///
    /// The fourth priority — "[t]he resolved value of the parent structure element, if the
    /// attribute is inheritable" — is [`Self::inherited_attribute`], because whether an
    /// attribute *is* inheritable is stated per attribute in §14.8.5.4's tables and is a
    /// property of the attribute rather than of the element. The fifth, a default, is the same
    /// kind of statement and belongs to whoever knows the attribute.
    #[must_use]
    pub fn attribute(
        &self,
        document: &Document,
        element: &Dictionary,
        name: &str,
    ) -> Option<Object> {
        let attached = self.attributes(document, element);
        // Priority 2 and 3 together, in the order `attributes` returns them: classes first,
        // then `/A`, so the last match wins. A format-specific owner is not consulted at all —
        // the clause conditions priority 1 on "processing based on the format indicated by the
        // owner value", and nothing here translates to XML, HTML or CSS.
        attached
            .iter()
            .rev()
            .filter(|object| object.kind.is_pdf_native() || object.kind == Owner::Namespace)
            .find_map(|object| object.get(document, name))
    }

    /// §14.8.5.3's fourth priority: the value this element or its nearest ancestor states.
    ///
    /// > Inheritable attributes propagate down the structure tree; that is, an attribute that is
    /// > specified for an element shall apply to all the descendants of the element in the
    /// > structure tree unless a descendent element specifies an explicit value for the attribute.
    ///
    /// The walk goes up `/P`, which is the same chain §14.9.2.3's language inheritance uses and
    /// is bounded the same way — a `/P` is a reference a document controls. **Whether to call
    /// this or [`Self::attribute`] is the caller's decision**, because the clause makes
    /// inheritability a property of each attribute: §14.8.5.4's tables say which are inheritable,
    /// and a reader that inherited every attribute would give a paragraph its table's
    /// `/ColSpan`.
    ///
    /// The clause's own closing sentence is why this returns the same kind of answer as the
    /// direct one: "[t]here is no semantic distinction between attributes that are specified
    /// explicitly and ones that are inherited."
    #[must_use]
    pub fn inherited_attribute(
        &self,
        document: &Document,
        element: &Dictionary,
        name: &str,
    ) -> Option<Object> {
        let mut current = element.clone();
        for _ in 0..MAX_ANCESTRY {
            if let Some(value) = self.attribute(document, &current, name) {
                return Some(value);
            }
            current = document.get_key(&current, "P").as_dict()?.clone();
        }
        None
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

    /// §14.8.5.6's `PrintField` attributes, for a `Form` element of a non-interactive form.
    ///
    /// The clause explains what the attribute is *for*, and it is the only place in §14.8.5 that
    /// describes something a reader could act on: a form
    ///
    /// > may have originally contained interactive fields such as text fields and radio buttons
    /// > but were then converted into non-interactive PDF files, or they may have been designed
    /// > to be printed out and filled in manually.
    ///
    /// So the widget is gone and the marks are all that is left, and this attribute is what says
    /// the marks were a check box, whether it was ticked, and what the field was called. A
    /// screen reader that skipped it would read a form as a page of unlabelled boxes.
    ///
    /// `None` where the element states no `PrintField` attributes, which is every element of
    /// every corpus document — measured.
    ///
    /// Which attribute objects may carry it is stated by the clause and applied by
    /// [`Self::attribute`] already: it "may only be defined in attribute objects whose O (owner)
    /// entry has the value `PrintField` or whose owner is any other owner excluding `Layout`,
    /// `List`, `Table` and `Artifact`" — a paraphrase in this crate's own spelling of the names.
    #[must_use]
    pub fn print_field(&self, document: &Document, element: &Dictionary) -> Option<PrintField> {
        let role = self
            .attribute(document, element, "Role")
            .and_then(|value| value.as_name().map(|name| name.as_bytes().to_vec()));
        // Table 383 gives this entry two spellings and deprecates one of them: "Checked,
        // checked … lower case form is deprecated in PDF 2.0", with NOTE 2 explaining that the
        // old case "did not conform to the same conventions used elsewhere in this standard".
        // Both are read, the current spelling first, because deprecation tells a *writer* what
        // to stop doing — the same reading §12.2's `/ViewArea` gets.
        let checked = self
            .attribute(document, element, "Checked")
            .or_else(|| self.attribute(document, element, "checked"))
            .and_then(|value| value.as_name().map(|name| name.as_bytes().to_vec()));
        let description = match self.attribute(document, element, "Desc") {
            Some(Object::String(bytes)) => Some(pdf_syntax::text_string(&bytes)),
            _ => None,
        };
        if role.is_none() && checked.is_none() && description.is_none() {
            return None;
        }
        Some(PrintField {
            role: role.as_deref().and_then(FieldRole::read),
            // "Default value: off ." — applied here rather than left to a caller, because the
            // clause states it and an unticked box is what a form full of them looks like.
            checked: checked
                .as_deref()
                .and_then(Checked::read)
                .unwrap_or(Checked::Off),
            description,
        })
    }

    /// The structure element a `/ID` names, through §14.7.2's Table 354 `/IDTree`.
    ///
    /// > A name tree (see 7.9.6, "Name trees") that maps element identifiers (see "Table 355 -
    /// > Entries in a structure element dictionary") to the structure elements they denote.
    ///
    /// The identifiers are the `/ID` entries §14.7.2 puts on elements, and the tree is
    /// `pdf-syntax`'s: §7.9.6's keys are byte strings compared by code, which is what
    /// [`pdf_syntax::tree::lookup`] does. §14.7.7's worked example carries one, mapping `Chap1`,
    /// `Sec1.1`, `Sec1.2` and `Sec1.3` to four elements.
    ///
    /// `None` for a document with no `/IDTree` — 89 of the corpus's 89 tagged ones — or an
    /// identifier it does not hold.
    #[must_use]
    pub fn element_by_id(&self, document: &Document, id: &[u8]) -> Option<Dictionary> {
        let tree = document.get_key(&self.root, "IDTree");
        let tree = tree.as_dict()?;
        let found = tree::lookup(tree, &tree::TreeKey::Name(id), &|object| {
            document.resolve(object)
        })?;
        document.resolve(&found).as_dict().cloned()
    }

    /// §14.8.2.5.1's logical content order, for one page.
    ///
    /// > Logical content order -the ordering for semantic purposes -shall be defined by a
    /// > depth-first traversal of the document's logical structure hierarch y.
    ///
    /// against which the other order is the stream's: "[p]age content order shall be defined by
    /// the sequencing of graphics objects within a page's content stream". The clause says the
    /// two "should coincide" and then spends a NOTE explaining when they cannot — overlapping
    /// objects, a headline spanning two pages, two articles beginning on one page — so a
    /// consumer that wants meaning has to walk the tree and a consumer that wants pixels has to
    /// walk the stream.
    ///
    /// The items are this page's only, in the tree's order, and **annotations are among them**:
    /// §14.8.2.5.2 says an annotation "is not interleaved within the page's content stream" and
    /// that "[t]he position of an annotation in the logical content order is determined from the
    /// document's logical structure", which is exactly the object references this walk already
    /// returns.
    ///
    /// A content item whose `/Pg` is absent is included: Table 355 makes the entry required only
    /// where an element has content items of the integer form, and an element that states none
    /// anywhere in its ancestry has left the page unstated rather than said "not this page".
    #[must_use]
    pub fn logical_order(&self, document: &Document, page: ObjectId) -> Vec<Child> {
        self.walk(document)
            .into_iter()
            .filter_map(|(_, child)| match child {
                // An element is not a content item; the walk returns both and only the leaves
                // are ordered content.
                Child::MarkedContent { page: at, .. } | Child::Object { page: at, .. } => {
                    at.is_none_or(|at| at == page).then_some(child)
                }
                Child::Element(_) => None,
            })
            .collect()
    }

    /// A page's readback, rearranged into §14.8.2.5.1's logical content order.
    ///
    /// Every marked-content sequence the structure tree reaches, in the tree's order, with the
    /// text each one covered in [`crate::Interpretation::text`]. Sequences the tree does not
    /// reach are **left out**, which is the clause's own position twice over: only structure
    /// elements are part of the logical content order, and §14.8.2.5.1 NOTE 3 says of the one
    /// case a reader might expect otherwise that "[a]rtifacts not contained within an Artifact
    /// structure element are not considered part of the logical content order".
    ///
    /// Annotations are in [`Self::logical_order`] and not here: an annotation's text is its own
    /// (§12.5.6's `/Contents`, a field's value) rather than a range of the page's readback, and
    /// splicing it into this string would make the result neither the page's text nor the
    /// document's.
    ///
    /// Equal to `interpretation.text` on a page whose two orders coincide, which the clause
    /// says they *should* — and `tests/logical_order.rs` measures how often they do.
    #[must_use]
    pub fn logical_text(
        &self,
        document: &Document,
        page: ObjectId,
        interpretation: &crate::Interpretation,
    ) -> String {
        let mut out = String::new();
        for item in self.logical_order(document, page) {
            let Child::MarkedContent { mcid, .. } = item else {
                continue;
            };
            for span in &interpretation.marked {
                if span.mcid == mcid
                    && let Some(text) = interpretation.text.get(span.range.clone())
                {
                    out.push_str(text);
                }
            }
        }
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

/// §14.8.5.6's `PrintField` attributes: what a non-interactive form field *was*. Table 383.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrintField {
    /// `/Role`, "[t]he type of form field represented".
    ///
    /// `None` is Table 383's own default — "[d]efault value: None specified" — which is an
    /// element carrying the other two entries and not saying what kind of field it was.
    pub role: Option<FieldRole>,
    /// `/Checked` (or the deprecated `/checked`), "[t]he state of a radio button or check box
    /// field". Table 383's default is [`Checked::Off`].
    pub checked: Checked,
    /// `/Desc`, "[t]he alternate name of the field", which NOTE 3 likens to an interactive
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
    /// The clause says where its value is, and it is not in the attribute: "[t]he text that is
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

/// §14.7.1's mark information dictionary. Table 353.
///
/// Three booleans in the catalog's `/MarkInfo`, and the first is what §14.8.1 turns on: "[a]
/// tagged PDF document shall contain a mark information dictionary … with a value of true for
/// the Marked entry." So this is how a document *says* it is tagged, as against having a
/// structure tree — which a document may have without claiming to follow §14.8's conventions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MarkInfo {
    /// `/Marked`: "whether the document conforms to tagged PDF conventions". Default false.
    ///
    /// The entry's own sentence bounds what the claim is worth: "[i]f `Suspects` is true, the
    /// document may not completely conform to tagged PDF conventions."
    pub marked: bool,
    /// `/UserProperties`: whether any structure element carries §14.7.6.4's user properties.
    ///
    /// A hint rather than a fact a reader needs: the clause says it "allow[s] PDF processors to
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
/// §14.8.4.1 makes the vocabulary closed for a tagged document — "[a]ll structure elements
/// occurring within a tagged PDF document shall have a type matching one of those defined as a
/// Standard Structure Type, or a role map providing a mapping from the non-standard type to a
/// Standard Structure Type" — which is why [`Tree::role`] exists and why this enum is what its
/// answer *means*.
///
/// The four categories §14.8.4.1 divides them into are [`Category`], and three types are in more
/// than one: see [`Self::category`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StandardType {
    /// `Document`: "[e]ncloses a logical document".
    Document,
    /// `DocumentFragment` (PDF 2.0): "[e]ncloses a logical document fragment".
    DocumentFragment,
    /// `Part`: "a grouping of structure elements without consideration for their hierarchy".
    Part,
    /// `Sect`: "a grouping of structure elements with consideration for their hierarchy".
    Section,
    /// `Div`: content "structured in fashion that is orthogonal to the semantic structure".
    Division,
    /// `Aside` (PDF 2.0): content "distinct from other content within its parent".
    Aside,
    /// `NonStruct`: "[a] grouping element having no inherent structural significance".
    NonStructural,
    /// `P`: "[a] low-level division of content", usually a paragraph.
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
    /// `Span`: "[a] generic inline portion of content having no particular inherent
    /// characteristics".
    Span,
    /// `Em` (PDF 2.0): content emphasised.
    Emphasis,
    /// `Strong` (PDF 2.0): content of "strong importance".
    Strong,
    /// `Link`: an association between content and a §12.5.6.5 link annotation.
    Link,
    /// `Annot`: an association between content and an annotation of another kind.
    Annotation,
    /// `Form`: an association between content and a §12.7 form field.
    Form,
    /// `Ruby`: the wrapper "around an entire ruby assembly".
    Ruby,
    /// `RB`: ruby base text, "[t]he full-size text to which the ruby annotation is applied".
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
    /// `LI`: one member of a list, "[i]nternal to L (List) structure elements".
    ListItem,
    /// `LBody`: "[t]he actual content of a list item", internal to `LI`.
    ListItemBody,
    /// `Table`: "[a] two-dimensional logical structure of cells".
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
    /// "[I]dentify a whole document or a fragment of a document."
    Document,
    /// "[M]ake it possible to organise the overall structure of content."
    Grouping,
    /// "[S]tructure types that enclose actual content, like a heading or paragraph."
    Block,
    /// "[E]nable structural organisation of content inside block level elements."
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
    /// Reads one of §14.8.4's names, or `None` for anything the clause does not define.
    ///
    /// `None` is a finding rather than a default: §14.8.4.1 requires every element of a tagged
    /// document to be a standard type or role mapped to one, so a name that arrives here
    /// unmapped is a document that has not done what the clause asks — which a consumer may want
    /// to say, and cannot if the reader has invented a type for it.
    #[must_use]
    pub fn read(name: &str) -> Option<Self> {
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
            _ => return None,
        })
    }

    /// The category §14.8.4 puts this type in.
    ///
    /// [`Category::Contextual`] where the clause gives two or three — `Figure`, `Formula`,
    /// `Link`, `Annot`, `Form`, `Title`, `FENote`, `Caption`, `L` and `Artifact` — and
    /// [`Category::of`] is the rule that settles it.
    #[must_use]
    pub fn category(&self) -> Category {
        match self {
            Self::Document | Self::DocumentFragment => Category::Document,
            Self::Part | Self::Section | Self::Division | Self::Aside | Self::NonStructural => {
                Category::Grouping
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

    /// Whether this type was introduced in PDF 2.0.
    ///
    /// Which matters for §14.8.6.1's two standard namespaces: a document in the *default*
    /// namespace — `http://iso.org/pdf/ssn`, PDF 1.7's — cannot mean one of these by it, and
    /// Annex M is where the standard lists the difference. A consumer that cares which
    /// vocabulary a name came from needs both this and [`Tree::namespace`].
    #[must_use]
    pub fn since_pdf_2_0(&self) -> bool {
        matches!(
            self,
            Self::DocumentFragment
                | Self::Aside
                | Self::Title
                | Self::FootnoteOrEndnote
                | Self::SubBlock
                | Self::Emphasis
                | Self::Strong
                | Self::Artifact
        )
    }
}

/// §14.8.2.2's artifact: content that is on the page and is not the document's content.
///
/// §14.8.2.2.1 divides a page in two. The real content is "material intentionally introduced
/// by the document's author and necessary to understand the content of the document"; "[a]ll
/// other content is considered to be artifacts, whether generated by the PDF writer in the
/// course of pagination, layout, or other mechanical processes or introduced by the document
/// author for decoration".
///
/// Nothing here changes a mark. What it changes is what a *consumer* of the page's text may
/// do with it, and the clause is explicit that the choice is the consumer's: "[a] text-to-speech
/// engine, for instance, may decide not to speak running heads or page numbers when the page is
/// turned", and NOTE 3 adds that "[t]he purpose of tagged PDF is not to prescribe what the PDF
/// processor does, but to provide sufficient declarative and descriptive information to allow it
/// to make appropriate choices". So this crate reads which content is an artifact and of what
/// kind, and drops none of it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Artifact {
    /// Table 363's `/Type`, or `None` where the property list states none or is absent.
    ///
    /// Absent for the `/Artifact BMC` form, which §14.8.2.2.2 calls "a generic artifact".
    pub kind: Option<ArtifactKind>,
    /// Table 363's `/Subtype`, which "should appear only when the `Type` entry has a value of
    /// `Pagination`" — carried as written rather than as an enum, because the entry is open:
    /// "[a]dditional values may be specified for this entry, provided they comply with the
    /// naming conventions described in Annex E".
    pub subtype: Option<String>,
    /// Table 363's `/BBox`: "the rectangle that completely encloses its visible extent", in
    /// default user space, as `[llx, lly, urx, ury]`.
    pub bbox: Option<[f32; 4]>,
    /// Table 363's `/Attached`: which page edges the artifact is logically attached to.
    ///
    /// "Page edges shall be defined by the page's crop box", and the order of the names "is
    /// immaterial" — so this is a set of four flags in the clause's own order, top, bottom,
    /// left, right.
    pub attached: [bool; 4],
}

/// Table 363's four artifact types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactKind {
    /// `Pagination`: "ancillary page features such as running heads, folios (page numbers) or
    /// Bates Numbering".
    Pagination,
    /// `Layout`: "purely cosmetic typographical or design elements such as footnote rules or
    /// decorative ornaments".
    Layout,
    /// `Page`: "production aids extraneous to the document itself, such as cut marks and
    /// colour bars".
    Page,
    /// `Background`: content "from document templates that are often repeated unchanged across
    /// many pages".
    Background,
}

impl Artifact {
    /// Reads Table 363's property list. `None` is a perfectly good artifact: see [`Self::kind`].
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
    let array = document.get_key(list, "BBox");
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
/// > "http://iso.org/pdf/ssn".
#[expect(
    clippy::doc_markdown,
    reason = "a verbatim quotation: the bare URL is the standard's own text, and rustdoc's \
              preferred `<…>` form would make the conformance gate's quotation check fail"
)]
pub const DEFAULT_STANDARD_NAMESPACE: &str = "http://iso.org/pdf/ssn";

/// §14.8.6.1's standard structure namespace for PDF 2.0.
///
/// The two together are what the clause calls "the standard structure namespaces"; a tagged
/// document's elements are required to be in one of them, in §14.8.6.3's `MathML` namespace, or
/// role mapped into one.
pub const STANDARD_NAMESPACE_2_0: &str = "http://iso.org/pdf2/ssn";

/// §14.8.6.3's one domain-specific namespace: `MathML` 3.0.
///
/// "`MathML` is the only domain-specific namespace defined in PDF 2.0", and the clause exempts
/// it from role mapping — a namespace named here "[does] not require a `RoleMapNS` entry".
pub const MATHML_NAMESPACE: &str = "http://www.w3.org/1998/Math/MathML";

/// §14.7.4.2's namespace dictionary. Table 356.
#[derive(Debug, Clone, PartialEq)]
pub struct Namespace {
    /// `/NS`: "[t]he string defining the namespace name which this entry identifies
    /// (conventionally a uniform resource identifier, or URI)".
    ///
    /// A string rather than a URI: NOTE 1 says "[i]t is not generally expected that a URI for
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
    #[must_use]
    pub fn is_standard(&self) -> bool {
        self.name == DEFAULT_STANDARD_NAMESPACE || self.name == STANDARD_NAMESPACE_2_0
    }
}

/// §14.7.6.1's attribute object. Table 360.
///
/// > An attribute object shall be a dictionary or stream that includes an O entry … identifying
/// > the conforming product that owns the attribute information. Other entries, except the NS
/// > entry, shall represent the attributes
///
/// which is why this holds the dictionary rather than a map: the attribute names are whatever
/// the owner defines, and the two entries that are *not* attributes are named by the clause.
#[derive(Debug, Clone, PartialEq)]
pub struct AttributeObject {
    /// Table 360's `/O`: who owns these attributes.
    ///
    /// One of §14.8.5's standard owners, `UserProperties`, `NSO`, or a name registered under
    /// Annex E. `NSO` means the owner is the namespace in [`Self::namespace`], which the
    /// clause states in both directions: "[i]f the value for the O entry is NSO then the NS
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
    /// The dictionary itself, whose other entries are the attributes.
    pub dict: Dictionary,
}

impl AttributeObject {
    /// Reads one attribute object, or `None` for anything that is not one.
    ///
    /// Table 360 makes `/O` required, so a dictionary without one has not said whose
    /// attributes these are — and since an attribute's meaning is its owner's, a reader that
    /// took the entries anyway would be inventing a vocabulary.
    fn read(document: &Document, object: &Object, revision: i64) -> Option<Self> {
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
            dict: dict.clone(),
        })
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

/// §14.8.5.2's standard attribute owners. Table 376.
///
/// The owner "determines the interpretation of the attributes defined in the object", so it is
/// not a label: `/BBox` under `Layout` and `/BBox` under `HTML-4.01` are different attributes
/// that happen to share a name. It also decides §14.8.5.3's priority, where the five *PDF-native*
/// owners rank below the format-specific ones.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Owner {
    /// `Layout`: "[a]ttributes governing the layout of content".
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
    use super::{Checked, Child, FieldRole, ParentTree, Tree, actual_text};
    use pdf_syntax::{Document, Object};

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

    /// §14.7.6's attributes, through both routes, with both precedence rules.
    ///
    /// The fixture attaches three attribute objects to one element: a class from the
    /// `/ClassMap` and two in the element's own `/A`, all three naming `/BackgroundColor`. The
    /// clause decides the winner twice over — "[i]f a given attribute is specified more than
    /// once, the later (in array order) entry shall take precedence" within `/A`, and "[i]f
    /// both the A and C entries are present and a given attribute is specified by both, the
    /// one specified by the A entry shall take precedence" between the two. So the answer is
    /// the *last* object of `/A`, and an attribute only the class states is still found.
    #[test]
    fn an_attribute_is_found_through_a_class_and_overridden_by_the_elements_own() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            "<< /Type /StructTreeRoot /K 5 0 R /ClassMap << /Warm 8 0 R >> >>",
            "<< /Type /StructElem /S /P /C [/Warm 3] /A [6 0 R 7 0 R 4] >>",
            "<< /O /Layout /BackgroundColor [0 0 1] >>",
            "<< /O /Layout /BackgroundColor [0 1 0] /TextAlign /Center >>",
            "<< /O /Layout /BackgroundColor [1 0 0] /Placement /Block >>",
        ]);
        let tree = Tree::of(&doc).expect("a structure tree root");
        let Some(Child::Element(element)) = tree.children(&doc, None).first().cloned() else {
            panic!("one element under the root");
        };

        let attached = tree.attributes(&doc, &element);
        assert_eq!(
            attached.len(),
            3,
            "one class and two of its own: {attached:?}"
        );
        assert_eq!(
            attached.iter().map(|a| a.revision).collect::<Vec<_>>(),
            vec![3, 0, 4],
            "§14.7.6.3: the integer after an entry is its revision, and an entry without \
             one is 0"
        );

        let colour = tree.attribute(&doc, &element, "BackgroundColor");
        assert_eq!(
            colour
                .as_ref()
                .and_then(Object::as_array)
                .map(<[Object]>::to_vec),
            Some(vec![
                Object::Integer(0),
                Object::Integer(1),
                Object::Integer(0)
            ]),
            "the later of the two /A objects wins, and /A beats /C"
        );
        assert!(
            tree.attribute(&doc, &element, "Placement").is_some(),
            "an attribute only the class states is still attached"
        );
        assert!(
            tree.attribute(&doc, &element, "O").is_none(),
            "/O identifies the owner and is not an attribute"
        );
    }

    /// §14.7.6.4's user properties, read from the clause's own example.
    ///
    /// The EXAMPLE at the end of §14.7.6.4 attaches four properties to a `Figure` — a part
    /// name, a part number, a supplier marked `/H true`, and a price of -37.99 formatted as
    /// `($37.99)`. Three of them are here, which are the three that state something a reader
    /// has to decide about: a plain value, a hidden one, and one whose formatted form differs
    /// from its value. That last is why the raw `/V` is kept beside `/F`.
    #[test]
    fn user_properties_are_read_with_their_formatting_and_their_hidden_flag() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            "<< /Type /StructTreeRoot /K 5 0 R >>",
            "<< /Type /StructElem /S /Figure /A 6 0 R >>",
            "<< /O /UserProperties /P [ << /N (Part Name) /V (Framostat) >> \
             << /N (Supplier) /V (Just Framostats) /H true >> \
             << /N (Price) /V -37.99 /F ($37.99) >> ] >>",
        ]);
        let tree = Tree::of(&doc).expect("a structure tree root");
        let Some(Child::Element(element)) = tree.children(&doc, None).first().cloned() else {
            panic!("one element under the root");
        };
        let attached = tree.attributes(&doc, &element);
        let [object] = attached.as_slice() else {
            panic!("one attribute object: {attached:?}");
        };
        assert_eq!(object.owner, "UserProperties");

        let properties = object.user_properties(&doc);
        assert_eq!(properties.len(), 3);
        assert_eq!(
            properties.first().map(|p| p.name.as_str()),
            Some("Part Name")
        );
        assert_eq!(
            properties.get(1).map(|p| p.hidden),
            Some(true),
            "/H true hides the property from a panel"
        );
        assert_eq!(
            properties.get(2).and_then(|p| p.formatted.clone()),
            Some("$37.99".to_owned())
        );
        assert!(
            tree.attributes(&doc, &element)
                .iter()
                .all(|object| object.get(&doc, "P").is_some()),
            "the /P array is reachable as the attribute it is"
        );
    }

    /// §14.8.6.2: an element in an explicit namespace is mapped by *that* namespace.
    ///
    /// > if the structure element does not explicitly identify its namespace using an NS
    /// > entry, it should use the RoleMap entry in the Structure Tree Root dictionary … If the
    /// > structure element is in an explicit namespace, then that namespace shall be identified
    /// > in the structure tree root dictionary's Namespaces array entry and the RoleMapNS entry
    /// > within that namespace dictionary shall provide the role mapping, if any.
    ///
    /// The fixture states both maps and they disagree on purpose: the root's `/RoleMap` would
    /// take `/Recipe` to `/Div`, and the element's own namespace takes it to `/Sect`. A reader
    /// that consulted the root would pass a test where only one map existed.
    #[test]
    fn a_namespaces_own_role_map_is_the_one_that_applies() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            "<< /Type /StructTreeRoot /K [5 0 R 7 0 R] /Namespaces [6 0 R] \
             /RoleMap << /Recipe /Div >> >>",
            "<< /Type /StructElem /S /Recipe /NS 6 0 R >>",
            "<< /Type /Namespace /NS (http://example.invalid/cookbook) \
             /RoleMapNS << /Recipe /Sect >> >>",
            "<< /Type /StructElem /S /Recipe >>",
        ]);
        let tree = Tree::of(&doc).expect("a structure tree root");
        let children = tree.children(&doc, None);
        let [Child::Element(in_namespace), Child::Element(plain)] = children.as_slice() else {
            panic!("two elements: {children:?}");
        };

        assert_eq!(tree.role(&doc, in_namespace).as_deref(), Some("Sect"));
        assert_eq!(
            tree.namespace(&doc, in_namespace).as_deref(),
            Some("http://example.invalid/cookbook")
        );
        assert_eq!(
            tree.role(&doc, plain).as_deref(),
            Some("Div"),
            "an element with no /NS is mapped by the root's /RoleMap"
        );
        assert_eq!(
            tree.namespace(&doc, plain).as_deref(),
            Some(super::DEFAULT_STANDARD_NAMESPACE),
            "§14.8.6.1: an element that names no namespace is in the default standard one"
        );
    }

    /// A `/RoleMapNS` may map *into another namespace*, and the walk follows it.
    ///
    /// Table 356's second form: "an array where the first value shall be a structure element
    /// type name in a target namespace with the second value being an indirect reference to
    /// the target namespace dictionary". So one document's `/Ingredient` becomes another
    /// namespace's `/ListItem`, which that namespace maps to the standard `/LI`.
    #[test]
    fn a_role_map_may_lead_into_another_namespace() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            "<< /Type /StructTreeRoot /K 5 0 R /Namespaces [6 0 R 7 0 R] >>",
            "<< /Type /StructElem /S /Ingredient /NS 6 0 R >>",
            "<< /Type /Namespace /NS (http://example.invalid/cookbook) \
             /RoleMapNS << /Ingredient [/ListItem 7 0 R] >> >>",
            "<< /Type /Namespace /NS (http://example.invalid/lists) \
             /RoleMapNS << /ListItem /LI >> >>",
        ]);
        let tree = Tree::of(&doc).expect("a structure tree root");
        let Some(Child::Element(element)) = tree.children(&doc, None).first().cloned() else {
            panic!("one element");
        };
        assert_eq!(tree.role(&doc, &element).as_deref(), Some("LI"));
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

    /// §14.8.4's vocabulary, including the family the table cannot enumerate.
    ///
    /// `Hn` is written in Table 372 as `H n`, "with n being a sequence of digits representing an
    /// unsigned integer greater than or equal to 1", so it is a *family* of names and not a name
    /// — which is why it is read before the table and why `H17` is as standard as `H1`. `H0` is
    /// not: the clause's own bound is 1.
    #[test]
    fn a_heading_carries_its_level_and_the_table_carries_the_rest() {
        use super::{Category, StandardType};

        assert_eq!(StandardType::read("H1"), Some(StandardType::Heading(1)));
        assert_eq!(StandardType::read("H17"), Some(StandardType::Heading(17)));
        assert_eq!(
            StandardType::read("H"),
            Some(StandardType::UnnumberedHeading)
        );
        assert_eq!(
            StandardType::read("H0"),
            None,
            "the clause's own bound is 1"
        );
        assert_eq!(StandardType::read("Hx"), None);
        assert_eq!(StandardType::read("Paragraph"), None, "not a standard name");

        assert_eq!(
            StandardType::read("TD").map(|kind| kind.category()),
            Some(Category::Internal),
            "a table cell is stated as internal to a table rather than as a level"
        );
        assert!(
            StandardType::read("Em").is_some_and(|kind| kind.since_pdf_2_0()),
            "emphasis is one of the eight types PDF 2.0 added"
        );
        assert!(
            StandardType::read("P").is_some_and(|kind| !kind.since_pdf_2_0()),
            "and a paragraph is not"
        );
    }

    /// §14.8.4.1's rule for a type that is block *or* inline, applied where it is used.
    ///
    /// > If the structure element is used inside a block level element, it is an inline level
    /// > structure element … In all other cases it is a block level structure element.
    #[test]
    fn a_contextual_type_takes_its_category_from_its_parent() {
        use super::{Category, StandardType};

        let figure = StandardType::read("Figure").expect("a standard type");
        assert_eq!(figure.category(), Category::Contextual);
        assert_eq!(
            figure.category().of(Some(Category::Block)),
            Category::Inline,
            "inside a paragraph, a figure is inline"
        );
        assert_eq!(
            figure.category().of(Some(Category::Grouping)),
            Category::Block,
            "and anywhere else it is block level"
        );
        assert_eq!(figure.category().of(None), Category::Block);
        assert_eq!(
            StandardType::read("P")
                .expect("a paragraph")
                .category()
                .of(Some(Category::Block)),
            Category::Block,
            "a type with one category keeps it wherever it is used"
        );
    }

    /// The role map's answer is read as the standard type it names.
    ///
    /// The point of the pair: a document's own `/Chapter` is not a standard type, and after
    /// §14.7.3's mapping it *is* one. A consumer asking "is this a heading" gets an answer for
    /// the mapped name and nothing for a name nobody mapped, which §14.8.4.1 makes a defect in
    /// the document rather than a gap in the reader.
    #[test]
    fn a_mapped_role_reads_as_its_standard_type() {
        use super::StandardType;

        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            "<< /Type /StructTreeRoot /K [5 0 R 6 0 R] /RoleMap << /Chapter /H1 >> >>",
            "<< /Type /StructElem /S /Chapter >>",
            "<< /Type /StructElem /S /Sidebar >>",
        ]);
        let tree = Tree::of(&doc).expect("a structure tree root");
        let children = tree.children(&doc, None);
        let [Child::Element(mapped), Child::Element(unmapped)] = children.as_slice() else {
            panic!("two elements: {children:?}");
        };
        assert_eq!(
            tree.standard_role(&doc, mapped),
            Some(StandardType::Heading(1))
        );
        assert_eq!(
            tree.standard_role(&doc, unmapped),
            None,
            "a name no role map takes to a standard type is not one"
        );
        assert_eq!(
            tree.role(&doc, unmapped).as_deref(),
            Some("Sidebar"),
            "and the name the document wrote is still available"
        );
    }

    /// §14.8.1's mark information dictionary, and the difference between having a tree and
    /// claiming to be tagged.
    ///
    /// "A tagged PDF document shall contain a mark information dictionary … with a value of true
    /// for the Marked entry", so a document with a structure tree and no `/MarkInfo` has a tree
    /// and has not made the claim — which is a real distinction and not a technicality: §14.8's
    /// rules are what `Marked` asserts conformance to.
    #[test]
    fn a_document_says_it_is_tagged_in_its_mark_information_dictionary() {
        use super::MarkInfo;

        let tagged = document(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R \
             /MarkInfo << /Marked true /UserProperties true >> >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            "<< /Type /StructTreeRoot >>",
        ]);
        let info = MarkInfo::read(&tagged);
        assert!(info.marked);
        assert!(info.user_properties);
        assert!(!info.suspects, "Table 353's default");

        let untagged = document(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            "<< /Type /StructTreeRoot >>",
        ]);
        assert!(
            !MarkInfo::read(&untagged).marked,
            "a structure tree is not the claim; the entry is"
        );
        assert!(Tree::of(&untagged).is_some(), "and the tree is still there");
    }

    /// §14.8.5.3's priority: a format-specific owner is skipped and an inheritable value walks
    /// up `/P`.
    ///
    /// The fixture states `/TextAlign` three times over two elements: an `HTML-4.01` object on
    /// the child, a `Layout` object on the *parent*, and a `Layout` object on the child. The
    /// clause's first priority applies "if processing based on the format indicated by the owner
    /// value" — this program processes to no format, so the `HTML-4.01` value is not the answer
    /// even though it outranks `Layout` for a processor translating to HTML. The parent's value
    /// is the fourth priority and only reachable through `inherited_attribute`.
    #[test]
    fn an_attributes_owner_and_its_ancestry_decide_which_value_applies() {
        use super::Owner;

        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            "<< /Type /StructTreeRoot /K 5 0 R >>",
            "<< /Type /StructElem /S /Sect /A 8 0 R /K [6 0 R 7 0 R] >>",
            "<< /Type /StructElem /S /P /P 5 0 R /A [9 0 R 10 0 R] >>",
            "<< /Type /StructElem /S /P /P 5 0 R >>",
            "<< /O /Layout /TextAlign /Justify >>",
            "<< /O /HTML-4.01 /TextAlign /Center >>",
            "<< /O /Layout /TextAlign /Start >>",
        ]);
        let tree = Tree::of(&doc).expect("a structure tree root");
        let Some(Child::Element(section)) = tree.children(&doc, None).first().cloned() else {
            panic!("one element under the root");
        };
        let kids = tree.children(&doc, Some(&section));
        let [Child::Element(stated), Child::Element(bare)] = kids.as_slice() else {
            panic!("two paragraphs: {kids:?}");
        };

        assert_eq!(
            tree.attributes(&doc, stated)
                .iter()
                .map(|object| object.kind.clone())
                .collect::<Vec<_>>(),
            vec![Owner::Format("HTML-4.01".to_owned()), Owner::Layout]
        );
        assert_eq!(
            tree.attribute(&doc, stated, "TextAlign")
                .and_then(|value| value.as_name().map(|n| n.as_bytes().to_vec())),
            Some(b"Start".to_vec()),
            "the Layout value, because nothing here processes to HTML"
        );
        assert_eq!(
            tree.attribute(&doc, bare, "TextAlign"),
            None,
            "the parent's value is not this element's own"
        );
        assert_eq!(
            tree.inherited_attribute(&doc, bare, "TextAlign")
                .and_then(|value| value.as_name().map(|n| n.as_bytes().to_vec())),
            Some(b"Justify".to_vec()),
            "§14.8.5.3's fourth priority, up /P"
        );
        assert_eq!(
            tree.inherited_attribute(&doc, stated, "TextAlign")
                .and_then(|value| value.as_name().map(|n| n.as_bytes().to_vec())),
            Some(b"Start".to_vec()),
            "an element that states its own is not overridden by its parent's"
        );
    }

    /// §14.8.5.6's `PrintField`: what a form field was, after the field itself was flattened.
    ///
    /// Three elements, one per way Table 383 can be written: a check box that is ticked, a
    /// radio button using the **deprecated lower-case** `/checked`, and a text-value field with
    /// only a `/Desc`. The last one also pins the table's default — an element that states no
    /// state is `off`, which is what an unticked box on a printed form looks like.
    #[test]
    fn a_flattened_form_field_says_what_it_was() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /StructParents 0 >>",
            "<< /Type /StructTreeRoot /K [5 0 R 6 0 R 7 0 R 8 0 R] >>",
            "<< /Type /StructElem /S /Form /Pg 3 0 R              /A << /O /PrintField /Role /cb /Checked /on /Desc (Agree to the terms) >> >>",
            "<< /Type /StructElem /S /Form /Pg 3 0 R              /A << /O /PrintField /Role /rb /checked /neutral >> >>",
            "<< /Type /StructElem /S /Form /Pg 3 0 R              /A << /O /PrintField /Role /tv /Desc (Surname) >> >>",
            "<< /Type /StructElem /S /P /Pg 3 0 R >>",
        ]);
        let tree = Tree::of(&doc).expect("a structure tree");
        let elements: Vec<_> = tree
            .children(&doc, None)
            .into_iter()
            .filter_map(|child| match child {
                Child::Element(dict) => Some(dict),
                _ => None,
            })
            .collect();

        let box_ticked = tree
            .print_field(&doc, &elements[0])
            .expect("a check box's attributes");
        assert_eq!(box_ticked.role, Some(FieldRole::CheckBox));
        assert_eq!(box_ticked.checked, Checked::On);
        assert_eq!(
            box_ticked.description.as_deref(),
            Some("Agree to the terms")
        );

        let button = tree
            .print_field(&doc, &elements[1])
            .expect("a radio button's attributes");
        assert_eq!(button.role, Some(FieldRole::RadioButton));
        assert_eq!(
            button.checked,
            Checked::Neutral,
            "the lower-case /checked is deprecated in PDF 2.0 and still read"
        );

        let text = tree
            .print_field(&doc, &elements[2])
            .expect("a text-value field's attributes");
        assert_eq!(text.role, Some(FieldRole::TextValue));
        assert_eq!(
            text.checked,
            Checked::Off,
            "Table 383's default for a field that states no state"
        );

        assert!(
            tree.print_field(&doc, &elements[3]).is_none(),
            "an element with no PrintField attributes has none"
        );
    }

    /// §14.7.2's `/IDTree`: an element found by the identifier it states.
    #[test]
    fn an_element_is_found_by_its_identifier() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            "<< /Type /StructTreeRoot /K [5 0 R] /IDTree 6 0 R >>",
            "<< /Type /StructElem /S /Sect /ID (Chap1) >>",
            "<< /Names [(Chap1) 5 0 R] >>",
        ]);
        let tree = Tree::of(&doc).expect("a structure tree");
        let found = tree.element_by_id(&doc, b"Chap1").expect("the element");
        assert_eq!(tree.role(&doc, &found).as_deref(), Some("Sect"));
        assert!(tree.element_by_id(&doc, b"Chap2").is_none());
    }
}
